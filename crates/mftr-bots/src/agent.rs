//! One bot (17 §7): menus (ranks, shop, drafts), dodging, emergencies, then a behavior chosen
//! between the coach's task and what's in front of it, with commitment so it doesn't dither.
//! Everything goes out as ordinary commands, paced by the tier.

use crate::board::Board;
use crate::coach::{Coach, Task};
use crate::dodge;
use crate::fight;
use crate::jungle;
use crate::kit::{Archetype, Role};
use crate::knowledge::Knowledge;
use crate::laning;
use crate::memory::{Memory, ThreatKey};
use crate::micro;
use crate::shop;
use crate::tier::{Params, Tier};
use mftr_sim::ping::PingKind;
use mftr_sim::rng::Pcg32;
use mftr_sim::world::can_shop;
use mftr_sim::{CommandKind, PlayerId, QPoint, SimDuration, SimTime, Tick, Unit, UnitId, UnitKind, Vec2};

/// What a bot is doing (for commitment, and for the measurements).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Retreat,
    Recall,
    Fight,
    Farm,
    Jungle,
    Travel,
    Siege,
    Ward,
}

/// What a bot thinks with: its team's board, memory and coach, the map, and its champion.
pub struct Ctx<'a, 'w> {
    pub b: &'a Board<'w>,
    pub memory: &'a Memory,
    pub k: &'a Knowledge,
    pub coach: &'a Coach,
    pub me: &'w Unit,
    pub now: SimTime,
    pub strategy: u8,
    /// Champions played by bots (everyone else is human).
    pub bots: &'a [UnitId],
    pub role: Option<Role>,
}

impl Ctx<'_, '_> {
    pub fn is_bot(&self, u: &Unit) -> bool {
        self.bots.contains(&u.id)
    }

    pub fn role_support(&self) -> bool {
        self.role == Some(Role::Support)
    }
}

/// Counters for the measurements (17 §11).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AgentStats {
    pub orders: u32,
    pub casts: u32,
    pub dodges: u32,
    pub last_hit_tries: u32,
    pub thinks: u32,
    /// Ticks spent in each mode (`Mode as usize`), dead included as `Idle`.
    pub mode_ticks: [u32; 9],
}

pub struct Agent {
    pub player: PlayerId,
    pub tier: Tier,
    pub p: Params,
    rng: Pcg32,
    pub next_think: u32,
    last_order: Option<(CommandKind, u32)>,
    tokens: f32,
    tokens_at: u32,
    pub mode: Mode,
    mode_since: SimTime,
    dodging_until: SimTime,
    decided: Vec<(ThreatKey, SimTime, bool)>,
    focus: Option<(UnitId, SimTime)>,
    /// This think's read of the local fight (17 §5), for laning's spacing.
    pub local: f32,
    skew: f32,
    wake: Option<SimTime>,
    pub stats: AgentStats,
}

impl Agent {
    pub fn new(player: PlayerId, tier: Tier, seed: u64) -> Self {
        let p = tier.params();
        Agent {
            player,
            tier,
            p,
            rng: Pcg32::new(seed ^ 0x0061_6765_6e74, player.0 as u64 + 101),
            next_think: player.0 as u32 % p.think_calm.max(1),
            last_order: None,
            tokens: 2.0,
            tokens_at: 0,
            mode: Mode::Idle,
            mode_since: SimTime(0),
            dodging_until: SimTime(0),
            decided: Vec::new(),
            focus: None,
            local: 0.0,
            skew: 0.0,
            wake: None,
            stats: AgentStats::default(),
        }
    }

    pub fn rng(&mut self) -> &mut Pcg32 {
        &mut self.rng
    }

    pub fn rng_f32(&mut self) -> f32 {
        self.rng.next_f32()
    }

    pub fn focus(&self) -> Option<UnitId> {
        self.focus.map(|(id, _)| id)
    }

    pub fn set_focus(&mut self, id: UnitId, now: SimTime) {
        if self.focus.is_none_or(|(f, _)| f != id) {
            self.focus = Some((id, now));
        }
    }

    /// Think sooner than scheduled (an attack ready, a threat's reaction time up).
    pub fn wake_at(&mut self, t: SimTime) {
        self.wake = Some(self.wake.map_or(t, |w| w.min(t)));
    }

    /// The tier's last-hit timing error for the next decision (s).
    pub fn last_hit_skew(&self) -> f32 {
        self.skew
    }

    pub fn count_last_hit_try(&mut self) {
        self.stats.last_hit_tries += 1;
    }

    /// The tier's noise on a fight read.
    pub fn read_noise(&mut self) -> f32 {
        (self.rng.next_f32() * 2.0 - 1.0) * self.p.read_noise
    }

    /// Whether this bot thinks on tick `k`.
    pub fn due(&self, k: Tick) -> bool {
        k.0 >= self.next_think || self.wake.is_some_and(|w| SimTime::end_of(k) >= w)
    }

    /// A movement order toward `to`, unless it repeats the last one (re-planning costs the
    /// simulation a path, 17 §10).
    pub fn go(&mut self, c: &Ctx, to: Vec2) -> Option<CommandKind> {
        if c.me.state.pos.distance(to) < 60.0 {
            return None;
        }
        Some(CommandKind::MoveTo(QPoint::from_vec2(to)))
    }

    /// Commands for this think: a menu click (ranks, shop, drafts; free) and an order (paced by
    /// the tier's orders a second).
    pub fn think(&mut self, c: &Ctx, k: Tick) -> (Option<CommandKind>, Option<CommandKind>) {
        self.stats.thinks += 1;
        let since = k.0.saturating_sub(self.tokens_at);
        self.stats.mode_ticks[self.mode as usize] += since;
        self.wake = None;
        let me = c.me;
        let fighting = c.b.enemies.iter().any(|e| e.state.pos.distance(me.state.pos) < 1500.0)
            || (c.b.world.rules().horde
                && c.b.enemy_minions.iter().any(|e| e.state.pos.distance(me.state.pos) < 1200.0));
        let every = if fighting { self.p.think_fight } else { self.p.think_calm };
        self.next_think = k.0 + every;
        // Refill the order budget.
        let dt = (k.0.saturating_sub(self.tokens_at)) as f32 / mftr_sim::TICK_HZ as f32;
        self.tokens = (self.tokens + dt * self.p.orders_per_second).min(2.0);
        self.tokens_at = k.0;
        // A fresh last-hit timing error each think.
        self.skew = (self.rng.next_f32() * 2.0 - 1.0) * self.p.last_hit_error_ms / 1000.0;

        let menu = self.menu(c);
        if !me.state.alive() {
            self.mode = Mode::Idle;
            self.decided.clear();
            return (menu, None);
        }
        let action = self.act(c);
        let action = action.and_then(|cmd| self.pace(c, cmd, k));
        (menu, action)
    }

    /// Spend a point, pick a draft or an anvil, choose the jungler's spell, shop.
    fn menu(&mut self, c: &Ctx) -> Option<CommandKind> {
        let me = c.me;
        let p = &me.state.progress;
        let champ = me.champion?;
        if p.offer[0] != 0 {
            if self.tier == Tier::Recruit && self.rng.next_f32() < 0.5 {
                return Some(CommandKind::PickAugment((self.rng.next_u32() % 3) as u8));
            }
            return Some(match shop::draft(champ, p) {
                Ok(i) => CommandKind::PickAugment(i),
                Err(i) => CommandKind::RerollAugment(i),
            });
        }
        if p.anvil_offer[0] != 0 {
            return Some(CommandKind::PickAnvil(shop::anvil_pick(champ, p)));
        }
        if let Some(slot) = shop::level_up(champ, p) {
            return Some(CommandKind::LevelUp(slot));
        }
        let rules = c.b.world.rules();
        if !can_shop(me, c.b.world.map(), &rules) {
            return None;
        }
        let jungler = c.role == Some(Role::Jungle) && !c.k.camps.is_empty();
        if jungler && p.spell_f != mftr_sim::ability::SPELL_CLAIM {
            return Some(CommandKind::ChooseSpell(mftr_sim::ability::SPELL_CLAIM));
        }
        let physical = shop::physical_share(c.memory.enemies.iter().map(|s| s.champion));
        let classic = !c.k.camps.is_empty() && rules.passive_gold < 3.0;
        let potions = if classic && c.b.secs < 18.0 * 60.0 { 2 } else { 0 };
        let wards = if classic && c.b.world.map().layout.wards && self.p.strategy >= 2 && c.b.secs > 240.0 {
            matches!(c.role, Some(Role::Support | Role::Jungle)) as u8
        } else {
            0
        };
        if let Some(cmd) = shop::buy(me, physical, potions, wards) {
            return Some(cmd);
        }
        if rules.augments
            && p.level >= mftr_sim::anvils::MIN_LEVEL
            && p.gold >= mftr_sim::anvils::COST + 200.0
            && shop::next_item(&shop::path(champ, physical), &p.items).is_none()
        {
            return Some(CommandKind::BuyAnvil);
        }
        None
    }

    /// Respect the tier's pace: orders spend a budget refilled at its orders a second, and an
    /// order that repeats what the champion is already doing isn't sent.
    fn pace(&mut self, c: &Ctx, cmd: CommandKind, k: Tick) -> Option<CommandKind> {
        let st = &c.me.state;
        let repeat = match (cmd, self.last_order) {
            (CommandKind::Attack(id), _) => st.order == mftr_sim::Order::Attack(id),
            (CommandKind::MoveTo(q), Some((CommandKind::MoveTo(r), at))) => {
                q.to_vec2().distance(r.to_vec2()) < 120.0 && k.0 < at + 45 && st.order != mftr_sim::Order::Idle
            }
            (CommandKind::AttackMove(q), Some((CommandKind::AttackMove(r), at))) => {
                q.to_vec2().distance(r.to_vec2()) < 150.0 && k.0 < at + 60 && st.order != mftr_sim::Order::Idle
            }
            (CommandKind::Stop, _) => st.order == mftr_sim::Order::Idle,
            _ => false,
        };
        if repeat {
            return None;
        }
        if self.tokens < 1.0 {
            // Out of orders for now: think again as soon as there's one (a dodge can't wait for
            // the next regular think).
            let ticks = ((1.0 - self.tokens) / self.p.orders_per_second * mftr_sim::TICK_HZ as f32).ceil() as u32;
            self.wake_at(SimTime::end_of(Tick(k.0 + ticks.max(1))));
            return None;
        }
        self.tokens -= 1.0;
        self.last_order = Some((cmd, k.0));
        self.stats.orders += 1;
        if matches!(cmd, CommandKind::Cast { .. }) {
            self.stats.casts += 1;
        }
        Some(cmd)
    }

    fn set_mode(&mut self, m: Mode, now: SimTime) {
        if self.mode != m {
            self.mode = m;
            self.mode_since = now;
        }
    }

    /// The order to give now.
    fn act(&mut self, c: &Ctx) -> Option<CommandKind> {
        let me = c.me;
        let now = c.now;
        let st = &me.state;
        // Recalling: let it finish unless it's interrupted by danger.
        if st.recalling() {
            let threatened = c.b.enemies.iter().any(|e| e.state.pos.distance(st.pos) < 1100.0)
                || c.memory.damage_on(me.id, 1.0, now) > 0.0;
            if !threatened {
                return None;
            }
        }
        if let Some(cmd) = self.dodge(c) {
            self.stats.dodges += 1;
            return Some(cmd);
        }
        if now < self.dodging_until {
            return None;
        }
        if let Some(cmd) = self.emergency(c) {
            return Some(cmd);
        }
        self.decide(c)
    }

    /// Step out of what's about to hit us, after the tier's reaction time, if it chose to try.
    fn dodge(&mut self, c: &Ctx) -> Option<CommandKind> {
        let me = c.me;
        let now = c.now;
        let prefer = match c.coach.task(self.player) {
            Some(Task::Lane(l)) => c.k.along(l, c.b.team, me.state.pos).1 * -1.0,
            _ => (c.k.homes[c.b.team as usize] - me.state.pos).normalize_or_zero(),
        };
        let threats = dodge::threats(c.b.world, me, c.memory, now, prefer);
        self.decided.retain(|(k, ..)| threats.iter().any(|t| t.root == *k));
        for t in threats {
            let (react_at, will) = match self.decided.iter().find(|(k, ..)| *k == t.root) {
                Some(&(_, at, will)) => (at, will),
                None => {
                    let jitter = (self.rng.next_f32() * 2.0 - 1.0) * self.p.reaction_spread_ms;
                    let ms = (self.p.reaction_ms + jitter).max(120.0);
                    let at = t.seen_at.plus(SimDuration::from_millis(ms as u64));
                    // Hard crowd control is worth more care.
                    let chance = if t.hard { (self.p.dodge + 0.1).min(1.0) } else { self.p.dodge };
                    let will = self.rng.next_f32() < chance;
                    self.decided.push((t.root, at, will));
                    (at, will)
                }
            };
            if !will {
                continue;
            }
            if now < react_at {
                self.wake_at(react_at);
                continue;
            }
            if t.step < 5.0 {
                continue;
            }
            if !dodge::escapable(me, &t, now) {
                // Hard crowd control we can't walk out of: the top tiers dash or blink out
                // (17 §7.3).
                if t.hard
                    && self.p.strategy >= 2
                    && t.hits_in > 0.08
                    && let Some(cmd) = self.dash_out(c, t.to)
                {
                    self.dodging_until = now.plus(SimDuration::from_millis(((t.hits_in + 0.1) * 1000.0) as u64));
                    return Some(cmd);
                }
                continue;
            }
            self.dodging_until = now.plus(SimDuration::from_millis(((t.hits_in + 0.1) * 1000.0) as u64));
            return Some(CommandKind::MoveTo(QPoint::from_vec2(t.to)));
        }
        None
    }

    /// A dash or blink toward `to` (a dodge's way out), as far as it goes, if one is ready and
    /// lands on open ground: an ability's dash first, the Blink spell last.
    fn dash_out(&mut self, c: &Ctx, to: Vec2) -> Option<CommandKind> {
        let me = c.me;
        let dir = (to - me.state.pos).normalize_or_zero();
        if dir == Vec2::ZERO {
            return None;
        }
        for slot in [0u8, 1, 2, 3, 5, 4] {
            if !me.state.can_cast(c.now, slot) {
                continue;
            }
            let range = match me.ability(slot).map(|a| a.effect) {
                Some(mftr_sim::ability::Effect::Dash(d)) => d.range,
                Some(mftr_sim::ability::Effect::Blink(b)) => b.range,
                _ => continue,
            };
            let at = me.state.pos + dir * range;
            if c.b.world.map().walkable(at, 40.0) {
                return Some(micro::cast(slot, at));
            }
        }
        None
    }

    /// A potion when hurt, a shield to live through a burst.
    fn emergency(&mut self, c: &Ctx) -> Option<CommandKind> {
        let me = c.me;
        let st = &me.state;
        let share = st.health / me.stats.max_health.max(1.0);
        let home = c.k.fountains[c.b.team as usize].is_some_and(|f| f.distance(st.pos) < 1200.0);
        if share < 0.6 && st.potion_until <= c.now && !home {
            let slot = st.progress.items.iter().position(|id| mftr_sim::items::consumable(*id).is_some());
            if let Some(slot) = slot {
                return Some(CommandKind::UseItem(slot as u8));
            }
        }
        micro::support(self, c)
    }

    /// How the local fight reads (with the tier's noise).
    fn local(&mut self, c: &Ctx) -> f32 {
        let (us, them) = c.b.engagement(c.me.state.pos, 1250.0, Some(c.memory));
        fight::outcome(&us, &them) + self.read_noise()
    }

    fn decide(&mut self, c: &Ctx) -> Option<CommandKind> {
        let me = c.me;
        let now = c.now;
        let st = &me.state;
        let share = st.health / me.stats.max_health.max(1.0);
        let horde = c.b.world.rules().horde;
        let task = c.coach.task(self.player).unwrap_or(Task::Hold { at: st.pos });
        let near: Vec<&Unit> = c.b.enemies.iter().copied().filter(|e| e.state.pos.distance(st.pos) < 1300.0).collect();
        let local = if near.is_empty() && !horde { 1.0 } else { self.local(c) };
        self.local = local;
        let heals_home = c.k.fountains[c.b.team as usize].is_some() && c.b.world.map().layout.fountain_heals;

        // Horde, between waves: home to heal or shop, then the coach's spot.
        if horde && c.b.world.game().next_wave_at.is_some() {
            let gold_ready = shop::next_item(&shop::path(me.champion?, 0.5), &st.progress.items)
                .and_then(|n| mftr_sim::items::price(n, &st.progress.items))
                .is_some_and(|(cost, _)| cost <= st.progress.gold);
            let far = c.k.fountains[c.b.team as usize].is_some_and(|f| f.distance(st.pos) > 600.0);
            if (share < 0.9 || gold_ready) && far {
                self.set_mode(Mode::Recall, now);
                return Some(CommandKind::Recall);
            }
            if share < 0.9 && !far {
                return None;
            }
        }

        // Flee: too hurt with enemies around, or losing a fight that's on us (we're being hit,
        // or they're in reach and we're half down). A laner merely outmatched keeps its distance
        // instead (laning's spacing), without leaving the lane.
        let danger = c.memory.danger(st.pos, 1500.0, now);
        // Without a fountain to heal at (ARAM), a champion kept alive at low health is a
        // champion short in every fight: it stays in, behind its team, and flees only when
        // nearly dead (dying sends it home whole, and to the shop).
        let low = heals_home && (share < self.p.retreat_health || share < 0.1);
        let hit = c.memory.damage_on(me.id, 1.5, now) > me.stats.max_health * 0.04;
        let in_reach = near.iter().any(|e| {
            let reach = c.b.enemy_power(e.id).map_or(500.0, |p| p.range) + 150.0;
            e.state.pos.distance(st.pos) < reach
        });
        let losing = local < -0.35 && in_reach && (hit || share < 0.5) && (heals_home || local < -0.55 || share < 0.3);
        // The team's call to go in holds unless we're about to die.
        let committed_fight = (self.mode == Mode::Fight && now.secs_since(self.mode_since) < self.p.overchase)
            || (matches!(task, Task::Engage { .. }) && share > self.p.retreat_health);
        if ((low && (!near.is_empty() || danger > 0.5 || share < 0.12)) || (losing && !committed_fight))
            && !(horde && c.b.enemy_minions.is_empty() && near.is_empty())
        {
            return self.retreat(c, local);
        }
        // Keep fleeing a little while it's still dangerous (no dithering at the edge).
        if self.mode == Mode::Retreat
            && now.secs_since(self.mode_since) < 2.0
            && (in_reach || share < if heals_home { 0.45 } else { 0.3 })
        {
            return self.retreat(c, local);
        }

        // Home to heal or spend when it's safe and worth it.
        if let Some(cmd) = self.maybe_recall(c, share, danger) {
            return Some(cmd);
        }

        // Fight what's in front of us when the fight is ours, a kill is on, or a teammate is
        // fighting next to us.
        if !near.is_empty() || horde {
            let threshold = match task {
                Task::Lane(_) => 0.18,
                Task::Jungle => 0.1,
                Task::Gank { .. } => -0.15,
                Task::Engage { .. } => -0.4,
                Task::Hold { .. } => -0.05,
                // On the way to a human's call: fight what's on us, not what's on the way.
                Task::Answer { kind: PingKind::Attack | PingKind::Assist, .. } => -0.2,
                Task::Answer { .. } if c.memory.damage_on(me.id, 1.5, now) == 0.0 => 0.6,
                _ => -0.1,
            } - if self.mode == Mode::Fight { 0.15 } else { 0.0 };
            let ally_fighting = c.b.allies.iter().any(|a| {
                a.id != me.id
                    && a.state.alive()
                    && a.state.pos.distance(st.pos) < 1000.0
                    && matches!(a.state.order, mftr_sim::Order::Attack(id) if c.b.enemies.iter().any(|e| e.id == id))
            });
            let kill = near.iter().any(|e| {
                e.state.health * 1.05 < micro::team_burst(c, e, 900.0) && !micro::turret_unsafe(c, e.state.pos)
            });
            let overchase = self.mode == Mode::Fight && now.secs_since(self.mode_since) < self.p.overchase;
            let fight = local >= threshold || kill || (ally_fighting && local > -0.3) || overchase;
            let in_lane = matches!(task, Task::Lane(_));
            if fight && (!in_lane || (local > 0.35 && share > 0.5) || kill || ally_fighting) {
                let chase = !matches!(task, Task::Lane(_) | Task::Hold { .. }) || kill;
                if let Some(cmd) = micro::fight(self, c, chase, None) {
                    self.set_mode(Mode::Fight, now);
                    return Some(cmd);
                }
            }
        }

        // Health relics (maps that have them), when hurt and it's safe to take one.
        if share < 0.65
            && let Some(r) =
                c.b.relics
                    .iter()
                    .filter(|r| r.state.alive())
                    .filter(|r| r.state.pos.distance(st.pos) < 2200.0)
                    .filter(|r| !micro::turret_unsafe(c, r.state.pos))
                    .filter(|r| {
                        !c.b.enemies.iter().any(|e| e.state.pos.distance(r.state.pos) < r.state.pos.distance(st.pos))
                    })
                    .min_by(|a, b| a.state.pos.distance(st.pos).total_cmp(&b.state.pos.distance(st.pos)))
        {
            self.set_mode(Mode::Travel, now);
            return self.go(c, r.state.pos);
        }

        // Wards on the way.
        if near.is_empty()
            && let Some(cmd) = self.ward(c)
        {
            self.set_mode(Mode::Ward, now);
            return Some(cmd);
        }

        self.follow(c, task)
    }

    /// Carry out the coach's task.
    fn follow(&mut self, c: &Ctx, task: Task) -> Option<CommandKind> {
        let me = c.me;
        let now = c.now;
        let st = &me.state;
        match task {
            Task::Lane(l) => {
                self.set_mode(Mode::Farm, now);
                // Shove before a recall: push when we'd like to go home.
                let push = st.progress.gold > 1300.0 || c.b.lanes.get(l).is_some_and(|v| v.their_count == 0);
                laning::farm(self, c, l, push)
            }
            Task::Jungle => {
                if let Some(i) = jungle::next_camp(self, c) {
                    self.set_mode(Mode::Jungle, now);
                    return jungle::clear(self, c, i);
                }
                // Nothing up: help the nearest lane.
                let lane = c.k.nearest_lane(st.pos).map_or(0, |(l, _)| l);
                self.set_mode(Mode::Farm, now);
                laning::farm(self, c, lane, false)
            }
            Task::Objective { camp } => {
                self.set_mode(Mode::Jungle, now);
                jungle::clear(self, c, camp)
            }
            Task::Gank { lane, target } => {
                // The lane's own laner commits when the ganker arrives, not before.
                let ganker =
                    c.role != Some(Role::Jungle) && c.role.and_then(|r| r.lane()).is_some_and(|l| l == lane.min(2));
                if ganker
                    && let Some(t) = c.b.enemies.iter().find(|e| e.id == target)
                    && !c
                        .b
                        .allies
                        .iter()
                        .any(|a| a.id != me.id && a.state.alive() && a.state.pos.distance(t.state.pos) < 900.0)
                {
                    self.set_mode(Mode::Farm, now);
                    return laning::farm(self, c, lane, false);
                }
                let Some(t) = c.b.enemies.iter().find(|e| e.id == target) else {
                    // Lost sight: go where it was, unless that's been a while.
                    let s = c.memory.enemy(target)?;
                    if now.secs_since(s.seen_at) > 4.0 {
                        return laning::farm(self, c, lane, false);
                    }
                    self.set_mode(Mode::Travel, now);
                    return self.go(c, s.pos);
                };
                let d = t.state.pos.distance(st.pos);
                if d < 900.0 {
                    self.set_mode(Mode::Fight, now);
                    return micro::fight(self, c, true, Some(target));
                }
                self.set_mode(Mode::Travel, now);
                self.go(c, crate::aim::predict(t, now, 1.0, 1.0))
            }
            Task::Siege { lane } => {
                self.set_mode(Mode::Siege, now);
                // A window to take the structure outright: go hit it.
                if let Some(t) = c.b.lanes.get(lane).and_then(|v| v.their_target).and_then(|id| c.b.unit(id))
                    && micro::siege_window(c, t)
                {
                    if st.pos.distance(t.state.pos) > 1300.0 {
                        return self.go(c, t.state.pos);
                    }
                    return Some(CommandKind::Attack(t.id));
                }
                laning::farm(self, c, lane, true)
            }
            Task::Defend { lane, at } => {
                if st.pos.distance(at) > 1200.0 {
                    self.set_mode(Mode::Travel, now);
                    return self.go(c, at);
                }
                self.set_mode(Mode::Farm, now);
                if let Some(cmd) = micro::fight(self, c, false, None) {
                    return Some(cmd);
                }
                laning::farm(self, c, lane, true)
            }
            Task::Engage { at } => {
                // The front line goes in; the back line fights from behind it.
                self.set_mode(Mode::Fight, now);
                let back = me.champion.is_some_and(|ch| Archetype::of(ch).back());
                if let Some(cmd) = micro::fight(self, c, !back, None) {
                    return Some(cmd);
                }
                if back {
                    let home = c.k.homes[c.b.team as usize];
                    let range = me.attack.map_or(500.0, |s| s.range);
                    return self.go(c, at + (home - at).normalize_or_zero() * range);
                }
                self.go(c, at)
            }
            Task::Group { at } | Task::Hold { at } => {
                let horde = c.b.world.rules().horde;
                if horde && let Some(cmd) = micro::fight(self, c, true, None) {
                    self.set_mode(Mode::Fight, now);
                    return Some(cmd);
                }
                if st.pos.distance(at) > 700.0 {
                    self.set_mode(Mode::Travel, now);
                    let range_back = me.champion.is_some_and(|ch| Archetype::of(ch).back()) as u8 as f32 * 200.0;
                    let toward_home = (c.k.homes[c.b.team as usize] - at).normalize_or_zero();
                    return self.go(c, at + toward_home * range_back);
                }
                let lane = c.k.nearest_lane(st.pos).map_or(0, |(l, _)| l);
                self.set_mode(Mode::Farm, now);
                laning::farm(self, c, lane, matches!(task, Task::Group { .. }))
            }
            Task::Answer { kind, at, target, until } => {
                if now > until {
                    return None;
                }
                match kind {
                    PingKind::Danger => {
                        let away = (st.pos - at).normalize_or_zero();
                        let home = (c.k.homes[c.b.team as usize] - st.pos).normalize_or_zero();
                        self.set_mode(Mode::Retreat, now);
                        self.go(c, st.pos + (away + home).normalize_or_zero() * 600.0)
                    }
                    _ => {
                        if let Some(t) = target.and_then(|id| c.b.unit(id)).filter(|t| t.targetable()) {
                            if t.kind.is_structure() {
                                if st.pos.distance(t.state.pos) < 1000.0 && !micro::turret_unsafe(c, t.state.pos) {
                                    return Some(CommandKind::Attack(t.id));
                                }
                            } else if t.state.pos.distance(st.pos) < 1100.0 {
                                self.set_mode(Mode::Fight, now);
                                return micro::fight(self, c, true, Some(t.id));
                            }
                        }
                        if st.pos.distance(at) > 500.0 {
                            self.set_mode(Mode::Travel, now);
                            return self.go(c, at);
                        }
                        // There: hold the spot, fighting only what comes into reach (an attack
                        // or assist call chases).
                        self.set_mode(Mode::Fight, now);
                        micro::fight(self, c, matches!(kind, PingKind::Attack | PingKind::Assist), None)
                    }
                }
            }
        }
    }

    /// Back off toward safety: our nearest turret behind us, else home; slowing chasers and
    /// using dashes on the way. With nothing near, home where the fountain heals.
    fn retreat(&mut self, c: &Ctx, local: f32) -> Option<CommandKind> {
        let me = c.me;
        let now = c.now;
        let st = &me.state;
        self.set_mode(Mode::Retreat, now);
        let home = c.k.homes[c.b.team as usize];
        let safe =
            c.b.ally_structures
                .iter()
                .filter(|s| s.kind == UnitKind::Turret)
                .filter(|s| s.state.pos.distance(home) < st.pos.distance(home) + 200.0)
                .min_by(|a, b| a.state.pos.distance(st.pos).total_cmp(&b.state.pos.distance(st.pos)))
                .map_or(home, |s| s.state.pos + (home - s.state.pos).normalize_or_zero() * 250.0);
        if let Some(cmd) = micro::escape(self, c, safe) {
            return Some(cmd);
        }
        let near = c.b.enemies.iter().any(|e| e.state.pos.distance(st.pos) < 1500.0);
        let heals_home = c.k.fountains[c.b.team as usize].is_some() && c.b.world.map().layout.fountain_heals;
        let share = st.health / me.stats.max_health.max(1.0);
        if !near
            && heals_home
            && share < 0.45
            && st.pos.distance(home) > 2500.0
            && c.memory.damage_on(me.id, 2.0, now) == 0.0
        {
            self.set_mode(Mode::Recall, now);
            return Some(CommandKind::Recall);
        }
        // Cornered and slower than them: fight back rather than die walking.
        if local > -0.6
            && let Some(e) = c.b.enemies.iter().find(|e| e.state.pos.distance(st.pos) < 260.0)
            && e.state.speed_at(now) >= st.speed_at(now)
            && let Some(cmd) = micro::cast_best(self, c, e, true)
        {
            return Some(cmd);
        }
        // Safe under our turret: defend from there (its shots and ours on the minions it
        // covers), rather than stand by while the wave takes it.
        if st.pos.distance(safe) < 500.0
            && !c.b.enemies.iter().any(|e| e.state.pos.distance(st.pos) < e.attack.map_or(500.0, |s| s.range) + 250.0)
            && let Some(t) = c.b.ally_turrets_covering(st.pos, 0.0).next()
        {
            let range = t.attack.map_or(775.0, |s| s.range);
            if let laning::LastHit::Take(m) = laning::last_hit(self, c, 900.0)
                && m.state.pos.distance(t.state.pos) <= range
            {
                return Some(CommandKind::Attack(m.id));
            }
            let covered =
                c.b.enemy_minions
                    .iter()
                    .filter(|m| m.state.pos.distance(t.state.pos) <= range && m.state.pos.distance(st.pos) < 700.0)
                    .min_by(|a, b| a.state.health.total_cmp(&b.state.health));
            if let Some(m) = covered {
                return Some(CommandKind::Attack(m.id));
            }
        }
        self.go(c, if heals_home && !near { home } else { safe })
    }

    /// Recall when it's safe and worth it: hurt, or rich enough for a real item, with the wave
    /// handled.
    fn maybe_recall(&mut self, c: &Ctx, share: f32, danger: f32) -> Option<CommandKind> {
        let me = c.me;
        let st = &me.state;
        let has_fountain = c.k.fountains[c.b.team as usize].is_some();
        let heals = has_fountain && c.b.world.map().layout.fountain_heals;
        if !has_fountain || c.b.world.rules().horde {
            return None;
        }
        let home = c.k.homes[c.b.team as usize];
        if st.pos.distance(home) < 3000.0 {
            return None;
        }
        let champ = me.champion?;
        let next_cost = shop::next_item(&shop::path(champ, 0.5), &st.progress.items)
            .and_then(|n| mftr_sim::items::price(n, &st.progress.items))
            .map_or(f32::MAX, |(cost, _)| cost);
        if !heals && crate::tier::off("aram-recall") {
            return None;
        }
        // Without healing at home (ARAM), only a real purchase is worth the walk back.
        let rich = if heals {
            st.progress.gold >= next_cost.max(1100.0) + 150.0 * (self.p.strategy as f32)
        } else {
            self.p.strategy >= 1 && st.progress.gold >= next_cost.max(1800.0) + 300.0
        };
        let hurt = heals && share < 0.38;
        if !(hurt || rich) {
            return None;
        }
        let safe = !c.b.enemies.iter().any(|e| e.state.pos.distance(st.pos) < 1500.0)
            && danger < 0.5
            && c.memory.damage_on(me.id, 3.0, c.now) == 0.0;
        if !safe {
            if hurt {
                return self.retreat(c, 0.0);
            }
            return None;
        }
        // A rich bot with a wave to clear clears it first (it'd lose it otherwise).
        if !hurt && c.b.enemy_minions.iter().any(|m| m.state.pos.distance(st.pos) < 900.0) && self.p.strategy >= 1 {
            return None;
        }
        self.set_mode(Mode::Recall, c.now);
        Some(CommandKind::Recall)
    }

    /// Place a ward at a spot the coach wants watched, when one is in reach and unwatched.
    fn ward(&mut self, c: &Ctx) -> Option<CommandKind> {
        use mftr_sim::wards::{PLACE_RANGE, TRINKET_TOTEM, WARD_VISION};
        let me = c.me;
        let p = &me.state.progress;
        if !c.b.world.map().layout.wards || self.p.strategy == 0 {
            return None;
        }
        let watched = |s: Vec2| {
            c.b.world.units().iter().any(|u| {
                u.kind == UnitKind::Ward
                    && u.team == c.b.team
                    && u.state.alive()
                    && u.state.pos.distance(s) < WARD_VISION * 0.8
            })
        };
        let spot = c
            .coach
            .wards
            .iter()
            .copied()
            .filter(|s| s.distance(me.state.pos) <= PLACE_RANGE + 250.0 && !watched(*s))
            .min_by(|a, b| a.distance(me.state.pos).total_cmp(&b.distance(me.state.pos)))?;
        if p.trinket == TRINKET_TOTEM && p.trinket_charges > 0 {
            return Some(CommandKind::UseTrinket(QPoint::from_vec2(spot)));
        }
        let slot = p.items.iter().position(|id| mftr_sim::items::placeable(*id))?;
        Some(CommandKind::UseItemAt { slot: slot as u8, target: QPoint::from_vec2(spot) })
    }
}
