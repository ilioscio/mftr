//! Basic server-side bots (M2 slice 5): fill empty slots so a match can be played (and tested)
//! with fewer than ten humans. A bot plays through ordinary commands, exactly like a client,
//! so replays record it like anyone else. It only uses what its team can see.
//!
//! Behavior: follow its minion wave down the lane, staying out of enemy turret range unless
//! allied minions are tanking; fight enemy champions in reach with its whole kit; heal and
//! shield hurt allies; when low, take a safe health relic, or go home where the fountain heals
//! (not in ARAM: there it keeps fighting from behind its wave); shop its build path in the
//! fountain or while dead; spend ability points (ultimate first).

use mftr_sim::ability::{Effect, SLOTS};
use mftr_sim::champion::max_rank;
use mftr_sim::lane::TURRET_ATTACK;
use mftr_sim::rng::Pcg32;
use mftr_sim::world::can_shop;
use mftr_sim::{Command, CommandKind, PlayerId, QPoint, SimTime, SubTick, Team, Tick, Unit, UnitKind, Vec2, World};

/// How often a bot decides (ticks): ~5 times a second, staggered by player.
const THINK_EVERY: u32 = 6;
/// Engage enemy champions this close.
const ENGAGE: f32 = 900.0;
/// Retreat below this share of max health; stop retreating above `HEALED` (a fountain that
/// heals) or `RELIEVED` (relics only).
const LOW: f32 = 0.3;
const HEALED: f32 = 0.9;
const RELIEVED: f32 = 0.5;
/// How far a low bot walks for a health relic.
const RELIC_REACH: f32 = 2500.0;
/// Turret danger zone: its range plus a margin.
const TURRET_DANGER: f32 = TURRET_ATTACK.range + 150.0;

pub struct Bot {
    pub player: PlayerId,
    rng: Pcg32,
    seq: u32,
    build: usize,
    retreating: bool,
    /// The last movement-type order and when it was issued.
    last_goal: Option<(CommandKind, Tick)>,
    now: Tick,
}

impl Bot {
    pub fn new(player: PlayerId, seed: u64) -> Self {
        Self {
            player,
            rng: Pcg32::new(seed ^ 0x626f_7473, player.0 as u64),
            seq: 1,
            build: 0,
            retreating: false,
            last_goal: None,
            now: Tick(0),
        }
    }

    /// Commands for tick `k` (the tick about to be simulated).
    pub fn think(&mut self, world: &World, k: Tick) -> Vec<Command> {
        if !(k.0 + self.player.0 as u32).is_multiple_of(THINK_EVERY) {
            return Vec::new();
        }
        let Some(me) = world.units().iter().find(|u| u.owner == Some(self.player) && u.kind == UnitKind::Champion)
        else {
            return Vec::new();
        };
        self.now = k;
        let kind = self.decide(world, me, SimTime::end_of(Tick(k.0 - 1)));
        kind.map(|kind| {
            self.seq += 1;
            vec![Command { player: self.player, seq: self.seq, tick: k, sub: SubTick::START, kind }]
        })
        .unwrap_or_default()
    }

    fn decide(&mut self, world: &World, me: &Unit, t: SimTime) -> Option<CommandKind> {
        let st = &me.state;
        let p = &st.progress;
        // An open augment draft (ARAM: Mayhem): take one of the choices.
        if p.offer[0] != 0 {
            let choice = self.rng.next_u32() as usize % mftr_sim::augments::CHOICES;
            return Some(CommandKind::PickAugment(choice as u8));
        }
        // Shop (dead or in the fountain): the next affordable step of the build path.
        if can_shop(me, world.map(), &world.rules()) {
            let path = mftr_sim::items::build_path(me.champion?);
            while self.build < path.len() && p.items.contains(&path[self.build]) {
                self.build += 1;
            }
            if let Some(&next) = path.get(self.build)
                && mftr_sim::items::price(next, &p.items).is_some_and(|(cost, _)| cost <= p.gold)
                && p.items.contains(&0)
            {
                return Some(CommandKind::Buy(next));
            }
        }
        if !st.alive() {
            self.retreating = false;
            return None;
        }
        if p.points > 0 {
            let slot = if p.ranks[3] < max_rank(3, p.level) {
                Some(3)
            } else {
                (0..3u8).filter(|s| p.ranks[*s as usize] < max_rank(*s, p.level)).min_by_key(|s| p.ranks[*s as usize])
            };
            if let Some(slot) = slot {
                return Some(CommandKind::LevelUp(slot));
            }
        }
        let champ = me.champion?;
        let team = me.team;
        let hp = st.health / me.stats.max_health.max(1.0);
        let ready = |slot: u8| st.can_cast(t, slot);
        let hidden = world.hidden(team);
        let visible = |u: &&Unit| u.state.alive() && !hidden.contains(&u.id);
        let fountain = world.map().layout.fountains[team as usize].map(|(c, _)| c);
        let heals_home = fountain.is_some() && world.map().layout.fountain_heals;

        // Low: take a safe health relic, else go home where the fountain heals. ARAM's doesn't
        // (no fountain healing after leaving base): waiting there or behind a turret for health
        // that never comes leaves the team a player short, so a bot keeps fighting from behind
        // its wave instead (`cautious`) until a relic is up.
        if hp < LOW {
            self.retreating = true;
        }
        if self.retreating && hp > if heals_home { HEALED } else { RELIEVED } {
            self.retreating = false;
        }
        let mut cautious = false;
        if self.retreating {
            if ready(5) && hp < LOW {
                return Some(cast(5, st.pos));
            }
            let enemies: Vec<Vec2> = world
                .units()
                .iter()
                .filter(visible)
                .filter(|u| u.team != team && u.kind == UnitKind::Champion)
                .map(|u| u.state.pos)
                .collect();
            // A relic is safe outside enemy turret cover and with no enemy champion nearer to it.
            let relic = world
                .units()
                .iter()
                .filter(|u| u.kind == UnitKind::Relic && u.state.alive())
                .map(|u| (u.state.pos, u.state.pos.distance(st.pos)))
                .filter(|&(pos, d)| {
                    d < RELIC_REACH && !self.unsafe_at(world, team, pos) && enemies.iter().all(|e| e.distance(pos) > d)
                })
                .min_by(|a, b| a.1.total_cmp(&b.1));
            match (relic, fountain) {
                (Some((pos, _)), _) => return self.go_to(CommandKind::MoveTo(QPoint::from_vec2(pos))),
                (None, Some(home)) if heals_home => return self.go_to(CommandKind::MoveTo(QPoint::from_vec2(home))),
                _ => cautious = true,
            }
        }

        // Heal or shield the most hurt ally champion in range, else itself when hurt.
        for slot in (0..4u8).filter(|s| ready(*s)) {
            let Some(Effect::Support(sup)) = champ.ability(slot).map(|a| a.effect) else { continue };
            let ally = world
                .units()
                .iter()
                .filter(|u| u.team == team && u.id != me.id && u.kind == UnitKind::Champion && u.state.alive())
                .filter(|u| sup.range > 0.0 && u.state.pos.distance(st.pos) <= sup.range)
                .filter(|u| u.state.health < 0.7 * u.stats.max_health)
                .min_by(|a, b| (a.state.health / a.stats.max_health).total_cmp(&(b.state.health / b.stats.max_health)));
            if let Some(a) = ally {
                return Some(cast(slot, a.state.pos));
            }
            if hp < 0.6 {
                return Some(cast(slot, st.pos));
            }
        }

        // Fight the nearest visible enemy champion in reach, unless that means diving a turret.
        let enemy = world
            .units()
            .iter()
            .filter(visible)
            .filter(|u| u.team != team && u.kind == UnitKind::Champion && u.targetable())
            .filter(|u| u.state.pos.distance(st.pos) <= ENGAGE)
            .min_by(|a, b| a.state.pos.distance(st.pos).total_cmp(&b.state.pos.distance(st.pos)));
        if let Some(e) = enemy
            && !self.unsafe_at(world, team, e.state.pos)
        {
            let d = e.state.pos.distance(st.pos);
            // A cautious bot fights what it can reach from where it stands, but doesn't chase.
            let reach = me.attack.map_or(0.0, |a| a.range) + e.gameplay_radius;
            let in_range = |slot: u8| match champ.ability(slot).map(|a| a.effect) {
                Some(Effect::Line(s)) => d <= s.range * 0.9,
                Some(Effect::Area(a)) if a.range == 0.0 => d <= a.radius * 0.8,
                Some(Effect::Area(a)) => d <= a.range,
                Some(Effect::Lunge(l)) => d <= l.range && !cautious,
                _ => false,
            };
            let shots: Vec<u8> = (0..4u8).filter(|s| ready(*s) && in_range(*s)).collect();
            if !shots.is_empty() && self.rng.next_f32() < 0.6 {
                let slot = shots[self.rng.next_u32() as usize % shots.len()];
                // Lead a moving target a little.
                let aim = e.state.heading().map_or(e.state.pos, |h| {
                    e.state.pos + (h - e.state.pos).normalize_or_zero() * self.rng.range_f32(0.0, 120.0)
                });
                return Some(cast(slot, aim));
            }
            if hp < 0.4 && ready(5) {
                return Some(cast(5, st.pos));
            }
            if !cautious || d <= reach {
                return self.go_to(CommandKind::Attack(e.id));
            }
        }

        // Siege: hit an exposed enemy structure in reach when it's safe (its turret shoots the
        // minions tanking it, or it can't shoot at all).
        let siege = world
            .units()
            .iter()
            .filter(|u| u.team != team && u.kind.is_structure() && u.targetable())
            .filter(|u| u.state.pos.distance(st.pos) <= ENGAGE)
            .filter(|u| !self.unsafe_at(world, team, u.state.pos))
            .min_by(|a, b| a.state.pos.distance(st.pos).total_cmp(&b.state.pos.distance(st.pos)));
        if let Some(target) = siege
            && !cautious
        {
            return self.go_to(CommandKind::Attack(target.id));
        }

        // Push with the wave: just behind the frontmost allied minion, out of unsafe turret range.
        let lane = &world.map().layout.lanes[team as usize];
        let (Some(&start), Some(&end)) = (lane.first(), lane.last()) else {
            return self.go_to(CommandKind::AttackMove(QPoint::from_vec2(world.map().size * 0.5)));
        };
        let dir = (end - start).normalize_or_zero();
        let progress = |p: Vec2| (p - start).dot(dir);
        let front = world
            .units()
            .iter()
            .filter(|u| u.team == team && u.kind == UnitKind::Minion && u.state.alive())
            .max_by(|a, b| progress(a.state.pos).total_cmp(&progress(b.state.pos)));
        let behind = if cautious { 550.0 } else { 150.0 };
        let mut goal = match front {
            Some(m) => m.state.pos - dir * behind,
            // No wave: wait by the frontmost allied turret.
            None => world
                .units()
                .iter()
                .filter(|u| u.team == team && u.kind == UnitKind::Turret && u.state.alive())
                .max_by(|a, b| progress(a.state.pos).total_cmp(&progress(b.state.pos)))
                .map_or(start, |u| u.state.pos),
        };
        // Back off along the lane while the spot is in an unsafe turret's range.
        for _ in 0..20 {
            if !self.unsafe_at(world, team, goal) {
                break;
            }
            goal -= dir * 100.0;
        }
        let goal = goal + Vec2::new(0.0, self.rng.range_f32(-120.0, 120.0));
        self.go_to(CommandKind::AttackMove(QPoint::from_vec2(goal)))
    }

    /// An enemy turret covers `p`, fewer than two allied minions are there to take its shots,
    /// and the team doesn't outnumber the defenders nearby by two (a dive).
    fn unsafe_at(&self, world: &World, team: Team, p: Vec2) -> bool {
        let champions_near = |side: Team, at: Vec2| {
            world
                .units()
                .iter()
                .filter(|u| u.team == side && u.kind == UnitKind::Champion && u.state.alive())
                .filter(|u| u.state.pos.distance(at) <= 1200.0)
                .count()
        };
        world.units().iter().filter(|u| u.team != team && u.kind == UnitKind::Turret && u.state.alive()).any(|t| {
            let pos = t.state.pos;
            pos.distance(p) <= TURRET_DANGER && {
                let tanks = world
                    .units()
                    .iter()
                    .filter(|m| m.team == team && m.kind == UnitKind::Minion && m.state.alive())
                    .filter(|m| m.state.pos.distance(pos) <= TURRET_ATTACK.range)
                    .count();
                let enemy = if team == Team::Blue { Team::Red } else { Team::Blue };
                tanks < 2 && champions_near(team, pos) < champions_near(enemy, pos) + 2
            }
        })
    }

    /// Issue a movement-type order unless it repeats the last one (re-planning every think
    /// would cost commands and bandwidth for nothing): the same target, or a point within
    /// 200 u, within 2 s.
    fn go_to(&mut self, kind: CommandKind) -> Option<CommandKind> {
        let same = |a: CommandKind, b: CommandKind| match (a, b) {
            (CommandKind::Attack(x), CommandKind::Attack(y)) => x == y,
            (CommandKind::MoveTo(p), CommandKind::MoveTo(q))
            | (CommandKind::AttackMove(p), CommandKind::AttackMove(q)) => (p.to_vec2() - q.to_vec2()).length() < 200.0,
            _ => false,
        };
        if let Some((last, at)) = self.last_goal
            && at.0 + 60 > self.now.0
            && same(last, kind)
        {
            return None;
        }
        self.last_goal = Some((kind, self.now));
        Some(kind)
    }
}

fn cast(slot: u8, at: Vec2) -> CommandKind {
    debug_assert!((slot as usize) < SLOTS);
    CommandKind::Cast { slot, target: QPoint::from_vec2(at) }
}
