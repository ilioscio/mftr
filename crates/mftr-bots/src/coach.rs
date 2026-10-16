//! The team's coach (17 §6): once a second it reads the map (the phase, each lane, who's where,
//! who's dead, what humans asked for) and gives each of its bots a task. The bots then weigh
//! that task against what's in front of them.

use crate::board::Board;
use crate::fight;
use crate::kit::{Archetype, Role, assign_roles};
use crate::knowledge::Knowledge;
use crate::memory::{CampState, Memory};
use mftr_sim::jungle::CampKind;
use mftr_sim::ping::{Ping, PingKind};
use mftr_sim::rng::Pcg32;
use mftr_sim::{ChampionId, PlayerId, SimTime, Team, Tick, Unit, UnitId, UnitKind, Vec2};

/// What the coach wants a bot to do.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Task {
    /// Farm and hold a lane.
    Lane(usize),
    /// Clear our camps.
    Jungle,
    /// Catch an enemy laner with this lane's ally.
    Gank { lane: usize, target: UnitId },
    /// Push this lane's next structure with the team.
    Siege { lane: usize },
    /// Protect a structure under threat.
    Defend { lane: usize, at: Vec2 },
    /// Gather and fight around a point.
    Group { at: Vec2 },
    /// Take a camp (buffs, contested camps).
    Objective { camp: usize },
    /// Stand at a point until something comes (Horde's line, a regroup).
    Hold { at: Vec2 },
    /// A human's call, until it expires (17 §8).
    Answer { kind: PingKind, at: Vec2, target: Option<UnitId>, until: SimTime },
}

/// What kind of match the coach is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// Several lanes and a jungle (Crossroads).
    Classic,
    /// One lane, no jungle (The Bridge).
    Aram,
    /// Defend against the horde.
    Horde,
    /// No lanes (sandboxes): fight what's near.
    Open,
}

/// A team member as the coach sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Member {
    pub player: PlayerId,
    pub unit: UnitId,
    pub champion: ChampionId,
    /// Played by a bot (else a human).
    pub bot: bool,
    /// The bot's strategic reach (`Params::strategy`) and ping habit.
    pub strategy: u8,
    pub pings: f32,
}

/// A human ping the coach is acting on.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Heard {
    ping: Ping,
    at: SimTime,
    /// Bots that already answered it (bit per member index).
    answered: u16,
}

pub struct Coach {
    pub team: Team,
    pub mode: Mode,
    roles: Vec<(PlayerId, Role)>,
    human_roles: Vec<(PlayerId, Role)>,
    tasks: Vec<(PlayerId, Task)>,
    next_at: u32,
    /// Ward spots that matter now.
    pub wards: Vec<Vec2>,
    /// Places to play safe around (an "enemy missing" call), until when.
    pub cautions: Vec<(Vec2, SimTime)>,
    heard: Vec<Heard>,
    /// Pings the team's bots send (taken by the server each tick).
    pub out: Vec<Ping>,
    last_ping: SimTime,
    bot_pinged: Vec<(PlayerId, SimTime)>,
    announced: Vec<(PlayerId, Task)>,
    rng: Pcg32,
    /// The lane the team pushes together in the mid game, and since when.
    focus: Option<(usize, SimTime)>,
}

/// How often the coach thinks (ticks).
const EVERY: u32 = 30;
/// How long a human's ping is acted on (s).
const HEARD_FOR: f32 = 12.0;

impl Coach {
    pub fn new(team: Team, mode: Mode, seed: u64) -> Self {
        Coach {
            team,
            mode,
            roles: Vec::new(),
            human_roles: Vec::new(),
            tasks: Vec::new(),
            next_at: 0,
            wards: Vec::new(),
            cautions: Vec::new(),
            heard: Vec::new(),
            out: Vec::new(),
            last_ping: SimTime(0),
            bot_pinged: Vec::new(),
            announced: Vec::new(),
            rng: Pcg32::new(seed ^ 0x636f_6163_68, team as u64),
            focus: None,
        }
    }

    /// The task for `player` (Hold at home before the coach has thought).
    pub fn task(&self, player: PlayerId) -> Option<Task> {
        self.tasks.iter().find(|(p, _)| *p == player).map(|(_, t)| *t)
    }

    pub fn role(&self, player: PlayerId) -> Option<Role> {
        self.roles.iter().find(|(p, _)| *p == player).map(|(_, r)| *r)
    }

    /// A human teammate pinged.
    pub fn hear(&mut self, ping: Ping, now: SimTime) {
        self.heard.retain(|h| now.secs_since(h.at) < HEARD_FOR);
        self.heard.push(Heard { ping, at: now, answered: 0 });
        if ping.kind == PingKind::Missing || ping.kind == PingKind::Danger {
            self.cautions.push((ping.at, now.plus(mftr_sim::SimDuration::from_millis(10_000))));
        }
    }

    /// Whether it's time to think (staggered between teams).
    pub fn due(&self, k: Tick) -> bool {
        k.0 >= self.next_at
    }

    /// Think: reassign every bot's task.
    pub fn update(&mut self, b: &Board, memory: &Memory, knowledge: &Knowledge, members: &[Member], k: Tick) {
        self.next_at = k.0 + EVERY;
        let now = b.now;
        self.cautions.retain(|(_, until)| *until > now);
        self.heard.retain(|h| now.secs_since(h.at) < HEARD_FOR);
        let strategy = members.iter().filter(|m| m.bot).map(|m| m.strategy).max().unwrap_or(0);
        let mut tasks: Vec<(PlayerId, Task)> = match self.mode {
            Mode::Classic => self.classic(b, memory, knowledge, members, strategy),
            Mode::Aram => self.aram(b, memory, knowledge, members, strategy),
            Mode::Horde => self.horde(b, knowledge, members),
            Mode::Open => members.iter().map(|m| (m.player, Task::Group { at: b.world.map().size * 0.5 })).collect(),
        };
        self.answer_pings(b, knowledge, members, &mut tasks);
        self.announce(b, memory, knowledge, members, &tasks);
        self.wards = self.ward_spots(b, knowledge, members);
        self.tasks = tasks;
    }

    fn unit<'w>(b: &Board<'w>, m: &Member) -> Option<&'w Unit> {
        b.unit(m.unit)
    }

    /// Roles (Classic): humans keep the lane they went to; bots take the best fit of the rest.
    fn assign(&mut self, b: &Board, knowledge: &Knowledge, members: &[Member]) {
        // Humans' roles, from where they play once the lanes have settled.
        if b.secs > 75.0 {
            for m in members.iter().filter(|m| !m.bot) {
                let Some(u) = Self::unit(b, m).filter(|u| u.state.alive()) else { continue };
                let role = match knowledge.nearest_lane(u.state.pos) {
                    Some((lane, d)) if d < 1400.0 => match lane {
                        0 => Role::Top,
                        1 => Role::Mid,
                        _ => {
                            let carry = !Archetype::of(m.champion).support();
                            if carry { Role::Bottom } else { Role::Support }
                        }
                    },
                    _ => Role::Jungle,
                };
                match self.human_roles.iter_mut().find(|(p, _)| *p == m.player) {
                    Some(r) => r.1 = role,
                    None => self.human_roles.push((m.player, role)),
                }
            }
        }
        let champions: Vec<ChampionId> = members.iter().map(|m| m.champion).collect();
        let mut fixed: Vec<Option<Role>> =
            members.iter().map(|m| self.human_roles.iter().find(|(p, _)| *p == m.player).map(|(_, r)| *r)).collect();
        // Two humans claiming one role: the second floats.
        for i in 0..fixed.len() {
            if fixed[i].is_some() && fixed[..i].contains(&fixed[i]) {
                fixed[i] = None;
            }
        }
        // Keep bots' roles stable once given, unless a human took one.
        for (i, m) in members.iter().enumerate() {
            if m.bot
                && let Some(r) = self.role(m.player)
                && !fixed.contains(&Some(r))
            {
                fixed[i] = Some(r);
            }
        }
        let roles = assign_roles(&champions, &fixed);
        self.roles = members.iter().zip(roles).map(|(m, r)| (m.player, r)).collect();
    }

    fn classic(
        &mut self,
        b: &Board,
        memory: &Memory,
        knowledge: &Knowledge,
        members: &[Member],
        strategy: u8,
    ) -> Vec<(PlayerId, Task)> {
        self.assign(b, knowledge, members);
        let now = b.now;
        let lanes = knowledge.lanes.len();
        let lane_of = |r: Role| r.lane().map(|l| l.min(lanes.saturating_sub(1)));
        let mut tasks: Vec<(PlayerId, Task)> = members
            .iter()
            .filter(|m| m.bot)
            .map(|m| {
                let role = self.role(m.player).unwrap_or(Role::Mid);
                (m.player, lane_of(role).map_or(Task::Jungle, Task::Lane))
            })
            .collect();
        let alive = |m: &Member| Self::unit(b, m).is_some_and(|u| u.state.alive());
        let home = knowledge.homes[self.team as usize];
        let pos = |m: &Member| Self::unit(b, m).map_or(home, |u| u.state.pos);
        let hp = |m: &Member| Self::unit(b, m).map_or(0.0, |u| u.state.health / u.stats.max_health.max(1.0));
        let set = |tasks: &mut Vec<(PlayerId, Task)>, p: PlayerId, t: Task| {
            if let Some(e) = tasks.iter_mut().find(|(q, _)| *q == p) {
                e.1 = t;
            }
        };
        let roles = self.roles.clone();
        let role_of = |p: PlayerId| roles.iter().find(|(q, _)| *q == p).map(|(_, r)| *r);
        let is_jungler = |p: PlayerId| role_of(p) == Some(Role::Jungle);
        let outer_down =
            b.world.units().iter().filter(|u| u.kind == UnitKind::Turret && u.tier == 1 && !u.state.alive()).count();
        let mid_game = b.secs > 14.0 * 60.0 || outer_down >= 2;
        let late = b.secs > 26.0 * 60.0;

        // Mid game: the team groups in one lane and pushes with its wave; a split pusher may
        // take another.
        if mid_game && strategy >= 1 {
            let focus = self.focus_lane(b, knowledge, now);
            let mut splitter = None;
            if strategy >= 2 && !late {
                splitter = members
                    .iter()
                    .filter(|m| m.bot && alive(m) && hp(m) > 0.6 && !is_jungler(m.player))
                    .filter(|m| matches!(Archetype::of(m.champion), Archetype::Bruiser | Archetype::Assassin))
                    .map(|m| m.player)
                    .next();
            }
            let at = self.lane_front(b, knowledge, focus);
            for m in members.iter().filter(|m| m.bot) {
                if Some(m.player) == splitter {
                    let side = (0..lanes).filter(|l| *l != focus).max_by(|a, c| {
                        let w = |l: usize| b.lanes[l].our_count as f32 - b.lanes[l].their_count as f32;
                        w(*a).total_cmp(&w(*c))
                    });
                    if let Some(l) = side {
                        set(&mut tasks, m.player, Task::Siege { lane: l });
                    }
                    continue;
                }
                if is_jungler(m.player) && !late {
                    continue;
                }
                let near_structure = b.lanes[focus]
                    .their_target
                    .and_then(|id| b.unit(id))
                    .is_some_and(|t| b.lanes[focus].ours.is_some_and(|(_, p)| p.distance(t.state.pos) < 1500.0));
                let t = if near_structure { Task::Siege { lane: focus } } else { Task::Group { at } };
                set(&mut tasks, m.player, t);
            }
        }

        // A numbers advantage: enemies dead long enough to push a structure.
        let dead: Vec<f32> = memory
            .enemies
            .iter()
            .filter(|s| !s.alive)
            .filter_map(|s| s.respawn_at.map(|t| t.secs_since(now)))
            .filter(|left| *left > 12.0)
            .collect();
        if strategy >= 1 && (dead.len() >= 2 || (late && !dead.is_empty())) {
            let window = dead.iter().copied().fold(f32::MAX, f32::min);
            // The lane whose next structure our living bots reach soonest.
            let best = (0..lanes)
                .filter_map(|l| {
                    let t = b.lanes[l].their_target.and_then(|id| b.unit(id))?;
                    let reach: f32 = members
                        .iter()
                        .filter(|m| m.bot && alive(m))
                        .map(|m| knowledge.travel(pos(m), t.state.pos))
                        .fold(0.0, f32::max);
                    Some((l, t.id, reach))
                })
                .min_by(|a, c| a.2.total_cmp(&c.2));
            if let Some((lane, _, _)) = best {
                for m in members.iter().filter(|m| m.bot && alive(m)) {
                    let target = b.lanes[lane].their_target.and_then(|id| b.unit(id));
                    let far = target.is_some_and(|t| knowledge.travel(pos(m), t.state.pos) / 340.0 > window - 4.0);
                    if !far {
                        set(&mut tasks, m.player, Task::Siege { lane });
                    }
                }
            }
        }

        // Defend structures under threat.
        if strategy >= 1 {
            for lane in 0..lanes {
                let Some(structure) = b
                    .ally_structures
                    .iter()
                    .filter(|s| s.lane as usize == lane && s.kind != UnitKind::Relic && s.targetable())
                    .min_by(|a, c| {
                        knowledge
                            .along(lane, self.team, a.state.pos)
                            .0
                            .total_cmp(&knowledge.along(lane, self.team, c.state.pos).0)
                    })
                    .copied()
                    .filter(|s| s.tier >= 1)
                else {
                    continue;
                };
                let at = structure.state.pos;
                let threat = b.enemies.iter().filter(|e| e.state.pos.distance(at) < 1600.0).count();
                let hurt = structure.state.health < structure.stats.max_health * 0.85;
                if threat == 0 || (threat < 2 && !hurt) {
                    continue;
                }
                let mut sent = members
                    .iter()
                    .filter(|m| Self::unit(b, m).is_some_and(|u| u.state.alive() && u.state.pos.distance(at) < 1800.0))
                    .count();
                let mut helpers: Vec<&Member> = members
                    .iter()
                    .filter(|m| m.bot && alive(m) && pos(m).distance(at) >= 1800.0)
                    .filter(|m| knowledge.travel(pos(m), at) < 340.0 * 22.0)
                    .collect();
                helpers.sort_by(|a, c| knowledge.travel(pos(a), at).total_cmp(&knowledge.travel(pos(c), at)));
                for m in helpers {
                    if sent > threat + (strategy >= 2) as usize {
                        break;
                    }
                    set(&mut tasks, m.player, Task::Defend { lane, at });
                    sent += 1;
                }
            }
        }

        // Ganks: the jungler (and a mid laner whose wave is shoved, at higher strategy).
        if strategy >= 1 && !mid_game {
            for m in members.iter().filter(|m| m.bot && alive(m)) {
                let jungler = is_jungler(m.player);
                let roamer = strategy >= 2 && self.role(m.player) == Some(Role::Mid) && {
                    let v = b.lanes.get(1).copied().unwrap_or_default();
                    v.their_count == 0 && v.our_count >= 3
                };
                if !jungler && !roamer {
                    continue;
                }
                if hp(m) < 0.55 {
                    continue;
                }
                if let Some((lane, target)) = self.gank_target(b, memory, knowledge, members, m) {
                    set(&mut tasks, m.player, Task::Gank { lane, target });
                    // The lane's ally commits with it.
                    for ally in members.iter().filter(|a| a.bot && a.player != m.player) {
                        if self.role(ally.player).and_then(|r| r.lane()).map(|l| l.min(lanes - 1)) == Some(lane)
                            && hp(ally) > 0.4
                        {
                            set(&mut tasks, ally.player, Task::Gank { lane, target });
                        }
                    }
                    break;
                }
            }
        }

        // Buffs: the jungler takes its own; when the enemy jungler is seen at ours, the
        // nearest help.
        for (i, camp) in knowledge.camps.iter().enumerate() {
            if !matches!(camp.kind, CampKind::Warden | CampKind::Brute)
                || knowledge.side_of(camp.pos) != Some(self.team)
            {
                continue;
            }
            let contested = b.enemies.iter().any(|e| e.state.pos.distance(camp.pos) < 1200.0)
                && matches!(memory.camps.get(i).map(|c| c.0), Some(CampState::Up));
            if contested && strategy >= 1 {
                let mut helpers: Vec<&Member> = members.iter().filter(|m| m.bot && alive(m)).collect();
                helpers
                    .sort_by(|a, c| knowledge.travel(pos(a), camp.pos).total_cmp(&knowledge.travel(pos(c), camp.pos)));
                for m in helpers.into_iter().take(2) {
                    if knowledge.travel(pos(m), camp.pos) < 340.0 * 12.0 {
                        set(&mut tasks, m.player, Task::Objective { camp: i });
                    }
                }
            }
        }
        tasks
    }

    /// The lane the team pushes together: where our wave is farthest along and their next
    /// structure is weakest; kept for a while once chosen.
    fn focus_lane(&mut self, b: &Board, knowledge: &Knowledge, now: SimTime) -> usize {
        if let Some((l, since)) = self.focus
            && now.secs_since(since) < 45.0
            && b.lanes.get(l).is_some_and(|v| v.their_target.is_some())
        {
            return l;
        }
        let score = |l: usize| -> f32 {
            let v = &b.lanes[l];
            let wave = v.ours.map_or(0.0, |(d, _)| d / knowledge.lane_length(l, self.team).max(1.0));
            let structure = v
                .their_target
                .and_then(|id| b.unit(id))
                .map_or(0.0, |t| 1.0 - t.state.health / t.stats.max_health.max(1.0));
            // Mid is shortest to rotate from.
            wave + structure * 0.6 + if l == 1 { 0.15 } else { 0.0 } + if v.their_target.is_some() { 0.0 } else { -2.0 }
        };
        let best = (0..b.lanes.len()).max_by(|a, c| score(*a).total_cmp(&score(*c))).unwrap_or(0);
        self.focus = Some((best, now));
        best
    }

    /// Where our wave is in `lane`, or our frontmost turret there.
    fn lane_front(&self, b: &Board, knowledge: &Knowledge, lane: usize) -> Vec2 {
        let v = &b.lanes[lane];
        let d = match (v.ours, v.our_turret) {
            (Some((d, _)), _) => d - 300.0,
            (None, Some((d, _))) => d + 400.0,
            _ => knowledge.lane_length(lane, self.team) * 0.4,
        };
        knowledge.lane_point(lane, self.team, d.max(0.0))
    }

    /// The best gank for `m`: an enemy laner overextended or hurt, our laner there and healthy,
    /// the fight favorable, and `m` close enough to get there in time.
    fn gank_target(
        &self,
        b: &Board,
        memory: &Memory,
        knowledge: &Knowledge,
        members: &[Member],
        m: &Member,
    ) -> Option<(usize, UnitId)> {
        let me = b.unit(m.unit)?;
        let mut best: Option<(f32, usize, UnitId)> = None;
        for e in &b.enemies {
            let Some((lane, off)) = knowledge.nearest_lane(e.state.pos) else { continue };
            if off > 1300.0 {
                continue;
            }
            let travel = knowledge.travel(me.state.pos, e.state.pos);
            if travel > 340.0 * 14.0 || travel < 300.0 {
                continue;
            }
            // How far it stands from its own nearest turret.
            let safety = b
                .enemy_structures
                .iter()
                .filter(|t| t.kind == UnitKind::Turret)
                .map(|t| t.state.pos.distance(e.state.pos))
                .fold(f32::MAX, f32::min);
            let hurt = 1.0 - e.state.health / e.stats.max_health.max(1.0);
            let exposed = ((safety - 1200.0) / 2500.0).clamp(0.0, 1.0) + hurt * 0.8;
            if exposed < 0.35 {
                continue;
            }
            // Our side: the ganker plus allies in the lane; theirs: who's seen near, plus missing.
            let (mut us, mut them) = b.engagement(e.state.pos, 1300.0, Some(memory));
            us.add(&fight::power(me, b.now, true, |s| me.state.can_cast(b.now, s)), 1.0);
            let ally_there = members.iter().any(|a| {
                a.player != m.player
                    && b.unit(a.unit).is_some_and(|u| u.state.alive() && u.state.pos.distance(e.state.pos) < 1500.0)
            });
            if !ally_there {
                // A lone gank needs a much better fight.
                them.hp *= 1.3;
            }
            let o = fight::outcome(&us, &them);
            if o < 0.2 {
                continue;
            }
            let score = exposed + o - travel / 6000.0;
            if best.is_none_or(|(s, ..)| score > s) {
                best = Some((score, lane, e.id));
            }
        }
        best.map(|(_, l, id)| (l, id))
    }

    fn aram(
        &mut self,
        b: &Board,
        memory: &Memory,
        knowledge: &Knowledge,
        members: &[Member],
        strategy: u8,
    ) -> Vec<(PlayerId, Task)> {
        // The team moves as one around its wave; pushes the next structure with numbers up.
        let front = self.lane_front(b, knowledge, 0);
        let dead = memory
            .enemies
            .iter()
            .filter(|s| !s.alive && s.respawn_at.is_some_and(|t| t.secs_since(b.now) > 8.0))
            .count();
        let push = strategy >= 1 && (dead >= 2 || (dead >= 1 && b.lanes[0].our_count >= 3));
        members
            .iter()
            .filter(|m| m.bot)
            .map(|m| (m.player, if push { Task::Siege { lane: 0 } } else { Task::Group { at: front } }))
            .collect()
    }

    fn horde(&mut self, b: &Board, knowledge: &Knowledge, members: &[Member]) -> Vec<(PlayerId, Task)> {
        let team = self.team;
        let home = knowledge.homes[team as usize];
        // Our frontmost standing turret, and where the horde comes from.
        let turret = b
            .ally_structures
            .iter()
            .filter(|s| s.kind == UnitKind::Turret)
            .max_by(|a, c| a.state.pos.distance(home).total_cmp(&c.state.pos.distance(home)))
            .map(|s| s.state.pos);
        let spawn = b.world.map().layout.wave_spawn.first().map(|s| s[1 - team as usize]);
        let anchor = turret.unwrap_or(home);
        let toward = spawn.map_or(Vec2::new(1.0, 0.0), |s| (s - anchor).normalize_or_zero());
        let between = b.world.game().next_wave_at.is_some();
        // The horde's front: its unit nearest home.
        let front = b
            .enemy_minions
            .iter()
            .min_by(|a, c| a.state.pos.distance(home).total_cmp(&c.state.pos.distance(home)))
            .map(|u| u.state.pos);
        let at = match (between, front) {
            (true, _) | (false, None) => {
                // Before a wave, wait a little in front of the turret; when it's out but unseen,
                // go halfway to meet it.
                let out = if between { 300.0 } else { spawn.map_or(300.0, |s| s.distance(anchor) * 0.5) };
                anchor + toward * out
            }
            (false, Some(f)) => {
                // Meet it where the fight's ours; fall back toward the turret where it isn't.
                let (us, them) = b.engagement(f, 1200.0, None);
                let o = fight::outcome(&us, &them);
                if o > -0.1 || f.distance(home) > anchor.distance(home) + 2500.0 {
                    f - toward * 250.0
                } else {
                    anchor + toward * 150.0
                }
            }
        };
        members.iter().filter(|m| m.bot).map(|m| (m.player, Task::Group { at })).collect()
    }

    /// Humans' pings become tasks for the bots they reach (17 §8).
    fn answer_pings(&mut self, b: &Board, knowledge: &Knowledge, members: &[Member], tasks: &mut [(PlayerId, Task)]) {
        let now = b.now;
        let until = |s: f32| now.plus(mftr_sim::SimDuration::from_millis((s * 1000.0) as u64));
        let mut acks: Vec<(PlayerId, Vec2)> = Vec::new();
        for h in self.heard.iter_mut() {
            let p = h.ping;
            // Bots by how soon they'd get there.
            let mut near: Vec<(usize, f32)> = members
                .iter()
                .enumerate()
                .filter(|(_, m)| m.bot && m.strategy >= 1 && Some(m.unit) != p.from)
                .filter_map(|(i, m)| {
                    let u = b.unit(m.unit).filter(|u| u.state.alive())?;
                    Some((i, knowledge.travel(u.state.pos, p.at)))
                })
                .collect();
            near.sort_by(|a, c| a.1.total_cmp(&c.1));
            let (reach, count, secs) = match p.kind {
                PingKind::Attack => (6500.0, 5, 10.0),
                PingKind::Group | PingKind::OnMyWay => (7000.0, 5, 12.0),
                PingKind::Defend => (8000.0, 5, 15.0),
                PingKind::Assist => (5000.0, 2, 10.0),
                PingKind::Danger => (1800.0, 5, 4.0),
                PingKind::Missing | PingKind::Alert => continue,
            };
            // A hopeless call isn't followed (17 §8): judge the fight where it points.
            if matches!(p.kind, PingKind::Attack | PingKind::Assist) {
                let (us, them) = b.engagement(p.at, 1500.0, None);
                let mut us = us;
                for (i, _) in near.iter().take(count) {
                    if let Some(u) = b.unit(members[*i].unit) {
                        us.add(&fight::power(u, now, true, |s| u.state.can_cast(now, s)), 1.0);
                    }
                }
                if fight::outcome(&us, &them) < -0.45 {
                    continue;
                }
            }
            let lane = knowledge.nearest_lane(p.at).map_or(0, |(l, _)| l);
            for (i, d) in near.into_iter().take(count) {
                if d > reach {
                    continue;
                }
                let m = &members[i];
                let task = match p.kind {
                    PingKind::Defend => Task::Defend { lane, at: p.at },
                    kind => Task::Answer { kind, at: p.at, target: p.target, until: until(secs) },
                };
                if let Some(e) = tasks.iter_mut().find(|(q, _)| *q == m.player) {
                    e.1 = task;
                }
                if h.answered & (1 << i) == 0 && p.kind != PingKind::Danger {
                    h.answered |= 1 << i;
                    acks.push((m.player, p.at));
                }
            }
        }
        for (player, at) in acks {
            if let Some(m) = members.iter().find(|m| m.player == player) {
                self.send(m, Ping { kind: PingKind::OnMyWay, at, target: None, from: Some(m.unit) }, now, true);
            }
        }
    }

    /// Bots announce what the coach decided (17 §8): once per new decision, rate-limited.
    fn announce(
        &mut self,
        b: &Board,
        memory: &Memory,
        knowledge: &Knowledge,
        members: &[Member],
        tasks: &[(PlayerId, Task)],
    ) {
        let now = b.now;
        for (player, task) in tasks {
            let Some(m) = members.iter().find(|m| m.player == *player) else { continue };
            let before = self.announced.iter().find(|(p, _)| p == player).map(|(_, t)| *t);
            let same_kind = |a: Option<Task>, t: &Task| match (a, t) {
                (Some(Task::Gank { target: x, .. }), Task::Gank { target: y, .. }) => x == *y,
                (Some(Task::Siege { lane: x }), Task::Siege { lane: y }) => x == *y,
                (Some(Task::Defend { lane: x, .. }), Task::Defend { lane: y, .. }) => x == *y,
                (Some(Task::Objective { camp: x }), Task::Objective { camp: y }) => x == *y,
                (Some(a), t) => std::mem::discriminant(&a) == std::mem::discriminant(t),
                (None, _) => false,
            };
            if same_kind(before, task) {
                continue;
            }
            match self.announced.iter_mut().find(|(p, _)| p == player) {
                Some(e) => e.1 = *task,
                None => self.announced.push((*player, *task)),
            }
            let ping = match *task {
                Task::Gank { target, .. } if self.role(*player) == Some(Role::Jungle) => b.unit(target).map(|t| Ping {
                    kind: PingKind::OnMyWay,
                    at: t.state.pos,
                    target: Some(target),
                    from: Some(m.unit),
                }),
                Task::Siege { lane } => b.lanes[lane].their_target.and_then(|id| b.unit(id)).map(|t| Ping {
                    kind: PingKind::Attack,
                    at: t.state.pos,
                    target: Some(t.id),
                    from: Some(m.unit),
                }),
                Task::Defend { at, .. } => Some(Ping { kind: PingKind::Defend, at, target: None, from: Some(m.unit) }),
                Task::Objective { camp } => knowledge.camps.get(camp).map(|c| Ping {
                    kind: PingKind::Group,
                    at: c.pos,
                    target: None,
                    from: Some(m.unit),
                }),
                _ => None,
            };
            if let Some(p) = ping {
                self.send(m, p, now, false);
            }
        }
        // A laner whose opponent just vanished calls it.
        for m in members.iter().filter(|m| m.bot && m.strategy >= 1) {
            let Some(lane) = self.role(m.player).and_then(|r| r.lane()) else { continue };
            let Some(u) = b.unit(m.unit).filter(|u| u.state.alive()) else { continue };
            for (id, l, at) in memory.vanished.iter().copied() {
                if l == Some(lane.min(knowledge.lanes.len().saturating_sub(1)))
                    && now.secs_since(at) < 1.2
                    && let Some(s) = memory.enemy(id)
                    && s.pos.distance(u.state.pos) < 2200.0
                {
                    let p = Ping { kind: PingKind::Missing, at: s.pos, target: None, from: Some(m.unit) };
                    self.send(m, p, now, false);
                }
            }
        }
    }

    /// Queue a bot's ping, within the team's and the bot's limits (`ack`: an answer to a human,
    /// which always goes out if the bot may ping at all).
    fn send(&mut self, m: &Member, p: Ping, now: SimTime, ack: bool) {
        if !ack && now.secs_since(self.last_ping) < 3.0 && self.last_ping.0 != 0 {
            return;
        }
        if let Some((_, at)) = self.bot_pinged.iter().find(|(q, _)| *q == m.player)
            && now.secs_since(*at) < if ack { 2.0 } else { 8.0 }
        {
            return;
        }
        if self.rng.next_f32() >= m.pings {
            return;
        }
        self.last_ping = now;
        match self.bot_pinged.iter_mut().find(|(q, _)| *q == m.player) {
            Some(e) => e.1 = now,
            None => self.bot_pinged.push((m.player, now)),
        }
        self.out.push(p);
    }

    /// Ward spots that matter now: by each lane our bots play, the river spots and brush
    /// nearest it on our half or the middle; later, around the focus lane.
    fn ward_spots(&self, b: &Board, knowledge: &Knowledge, members: &[Member]) -> Vec<Vec2> {
        if knowledge.ward_spots.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        let lanes: Vec<usize> = members
            .iter()
            .filter_map(|m| self.role(m.player).and_then(|r| r.lane()))
            .map(|l| l.min(knowledge.lanes.len().saturating_sub(1)))
            .collect();
        for lane in lanes {
            let front = self.lane_front(b, knowledge, lane);
            let mut spots: Vec<Vec2> = knowledge
                .ward_spots
                .iter()
                .filter(|s| s.lane == Some(lane) && s.side != Some(crate::other(self.team)))
                .map(|s| s.pos)
                .collect();
            spots.sort_by(|a, c| a.distance(front).total_cmp(&c.distance(front)));
            out.extend(spots.into_iter().take(2));
        }
        out.sort_by(|a, c| a.x.total_cmp(&c.x).then(a.y.total_cmp(&c.y)));
        out.dedup_by(|a, c| a.distance(*c) < 300.0);
        out
    }
}
