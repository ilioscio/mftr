//! The simulation world: units, commands and the fixed-tick step.
//!
//! Tick phases:
//! 1. AI decisions for server-driven units (patrols, turrets).
//! 2. A per-unit timeline on the exact [`SimTime`] axis: movement with unit collision against
//!    start-of-tick positions (`collision.rs`), sub-tick commands, cast windups (rooted) and
//!    stuns, each taking effect at its exact instant.
//! 3. Missiles: spawn at their fire instant, exact swept hits against units' motion this tick
//!    (`projectile::first_contact`), expiry.
//!
//! Vision lands in slice 3 on top of this structure.

use crate::ability::{LineSkillshot, SANDBOX_LANCE, TURRET_SHOT};
use crate::collision::{Obstacle, choose_detour, constrained_move};
use crate::hash::StateHasher;
use crate::map::{Map, MapId};
use crate::math::{QPoint, Vec2};
use crate::projectile::first_contact;
use crate::rng::Pcg32;
use crate::time::{SUBTICKS, SUBTICKS_PER_SECOND, SimTime, SubTick, Tick};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Team {
    Blue,
    Red,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitKind {
    Champion,
    Minion,
    /// Static structure (the dodge rig's shooters for now). Immune to skillshots.
    Turret,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MinionKind {
    Melee,
    Caster,
    Siege,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Order {
    Idle,
    MoveTo(QPoint),
}

/// Ticks of poor progress before a blocked unit looks for a detour (03a §5).
const STUCK_TICKS: u8 = 3;
const STUCK_PROGRESS: f32 = 0.2;
/// Only units this close can matter for one tick of movement or a detour probe.
const BROADPHASE: f32 = 300.0;
/// A blocked unit this close to its goal counts as arrived.
const GIVE_UP_DISTANCE: f32 = 80.0;

/// A cast in progress: the caster is rooted until the missile fires.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cast {
    pub dir: Vec2,
    pub fire_at: SimTime,
    /// Sequence number of the command that started it (0 for AI casts).
    pub seq: u32,
}

/// Most waypoints a path keeps; longer paths are re-planned when they run out.
pub const MAX_PATH: usize = 12;

/// An any-angle path toward the current move goal (from [`Map::find_path`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Path {
    pub points: [Vec2; MAX_PATH],
    pub len: u8,
    pub next: u8,
    /// The last point is the final destination (else the path was truncated).
    pub complete: bool,
}

impl Path {
    pub const EMPTY: Path = Path { points: [Vec2::ZERO; MAX_PATH], len: 0, next: 0, complete: true };

    fn from_points(points: &[Vec2]) -> Path {
        let mut p = Path::EMPTY;
        let n = points.len().min(MAX_PATH);
        p.points[..n].copy_from_slice(&points[..n]);
        p.len = n as u8;
        p.complete = points.len() <= MAX_PATH;
        p
    }

    pub fn waypoints(&self) -> &[Vec2] {
        &self.points[self.next as usize..self.len as usize]
    }

    fn bits(&self) -> ([[u32; 2]; MAX_PATH], u8, u8, bool) {
        let mut b = [[0u32; 2]; MAX_PATH];
        for (i, p) in self.points[..self.len as usize].iter().enumerate() {
            b[i] = p.to_bits();
        }
        (b, self.len, self.next, self.complete)
    }
}

/// Everything client prediction needs to reproduce a unit bit-exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitState {
    pub pos: Vec2,
    pub order: Order,
    pub move_speed: f32,
    /// Waypoints toward the `MoveTo` goal, around walls.
    pub path: Path,
    /// Temporary waypoint around blocking units, taken before resuming the path.
    pub detour: Option<Vec2>,
    /// Consecutive ticks of poor progress while moving.
    pub stuck: u8,
    pub cast: Option<Cast>,
    pub stunned_until: SimTime,
    pub q_ready_at: SimTime,
}

impl UnitState {
    pub fn new(pos: Vec2, move_speed: f32) -> Self {
        Self {
            pos,
            order: Order::Idle,
            move_speed,
            path: Path::EMPTY,
            detour: None,
            stuck: 0,
            cast: None,
            stunned_until: SimTime(0),
            q_ready_at: SimTime(0),
        }
    }

    /// Where the unit is currently heading (detour first, then the path), if anywhere.
    pub fn heading(&self) -> Option<Vec2> {
        if let Some(d) = self.detour {
            return Some(d);
        }
        match self.order {
            Order::MoveTo(_) => self.path.waypoints().first().copied(),
            Order::Idle => None,
        }
    }

    /// Replace the order (a command or AI decision) and plan the route on `map`.
    pub fn set_order(&mut self, order: Order, map: &Map) {
        self.order = order;
        self.detour = None;
        self.stuck = 0;
        self.route(map);
    }

    /// (Re)plan the path from the current position toward the order's goal.
    fn route(&mut self, map: &Map) {
        self.path = match self.order {
            Order::MoveTo(q) => Path::from_points(&map.find_path(self.pos, q.to_vec2())),
            Order::Idle => Path::EMPTY,
        };
        if self.order != Order::Idle && self.path.len == 0 {
            self.order = Order::Idle; // unreachable
        }
    }

    pub fn can_move(&self, at: SimTime) -> bool {
        self.cast.is_none() && self.stunned_until <= at
    }

    pub fn can_cast(&self, at: SimTime) -> bool {
        self.cast.is_none() && self.stunned_until <= at && self.q_ready_at <= at
    }

    /// Advance by `dt` seconds toward the current heading at constant speed (instant turns,
    /// R01 §2), blocked by `obstacles` and the map's walls. Returns `(desired, achieved)`.
    pub fn advance(&mut self, dt: f32, radius: f32, obstacles: &[Obstacle], map: &Map) -> (f32, f32) {
        if dt <= 0.0 {
            return (0.0, 0.0);
        }
        let Some(target) = self.heading() else { return (0.0, 0.0) };
        let to = target - self.pos;
        let dist = to.length();
        let step = self.move_speed * dt;
        let (delta, arrives) = if step >= dist { (to, true) } else { (to * (step / dist), false) };
        let new_pos = constrained_move(self.pos, delta, radius, obstacles, map.edges());
        let achieved = (new_pos - self.pos).length();
        self.pos = new_pos;
        if arrives && new_pos == target {
            self.stuck = 0;
            if self.detour.is_some() {
                // Off the planned path now: re-plan from here.
                self.detour = None;
                self.route(map);
            } else {
                self.path.next += 1;
                if self.path.next >= self.path.len {
                    if self.path.complete {
                        self.order = Order::Idle;
                        self.path = Path::EMPTY;
                    } else {
                        self.route(map);
                    }
                }
            }
        }
        (delta.length(), achieved)
    }

    /// Bit-exact equality: what prediction reconciliation compares.
    pub fn bits_eq(&self, other: &Self) -> bool {
        let cast_bits = |c: &Option<Cast>| c.map(|c| (c.dir.to_bits(), c.fire_at, c.seq));
        self.pos.to_bits() == other.pos.to_bits()
            && self.order == other.order
            && self.move_speed.to_bits() == other.move_speed.to_bits()
            && self.path.bits() == other.path.bits()
            && self.detour.map(Vec2::to_bits) == other.detour.map(Vec2::to_bits)
            && self.stuck == other.stuck
            && cast_bits(&self.cast) == cast_bits(&other.cast)
            && self.stunned_until == other.stunned_until
            && self.q_ready_at == other.q_ready_at
    }

    pub fn hash_into(&self, h: &mut StateHasher) {
        h.write_f32(self.pos.x);
        h.write_f32(self.pos.y);
        h.write_f32(self.move_speed);
        match self.order {
            Order::Idle => h.write_u8(0),
            Order::MoveTo(q) => {
                h.write_u8(1);
                h.write_u16(q.x);
                h.write_u16(q.y);
            }
        }
        h.write_u8(self.path.len);
        h.write_u8(self.path.next);
        h.write_u8(self.path.complete as u8);
        for p in &self.path.points[..self.path.len as usize] {
            h.write_f32(p.x);
            h.write_f32(p.y);
        }
        match self.detour {
            None => h.write_u8(0),
            Some(d) => {
                h.write_u8(1);
                h.write_f32(d.x);
                h.write_f32(d.y);
            }
        }
        h.write_u8(self.stuck);
        match self.cast {
            None => h.write_u8(0),
            Some(c) => {
                h.write_u8(1);
                h.write_f32(c.dir.x);
                h.write_f32(c.dir.y);
                h.write_u64(c.fire_at.0);
                h.write_u32(c.seq);
            }
        }
        h.write_u64(self.stunned_until.0);
        h.write_u64(self.q_ready_at.0);
    }
}

/// Server-side decision making for units without a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Brain {
    /// Walk back and forth between two points (minion-dummy waves, M1 sandbox).
    Patrol { a: QPoint, b: QPoint, toward_b: bool },
    /// Fire the turret shot at the nearest enemy champion in range (the dodge rig, 03 §14).
    /// Half the shots aim at the target's position, half lead its movement.
    Turret { range: u16 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub owner: Option<PlayerId>,
    pub team: Team,
    pub state: UnitState,
    /// Unit-vs-unit movement blocking (D11). Zero = doesn't collide.
    pub collision_radius: f32,
    /// Hitbox for abilities and targeting.
    pub gameplay_radius: f32,
    pub brain: Option<Brain>,
}

impl Unit {
    /// The unit's Q (the only ability in the sandbox).
    pub fn skillshot(&self) -> LineSkillshot {
        match self.kind {
            UnitKind::Turret => TURRET_SHOT,
            _ => SANDBOX_LANCE,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandKind {
    MoveTo(QPoint),
    Stop,
    /// Cast the line skillshot toward a ground point.
    CastQ(QPoint),
}

/// A player command, applied at `tick` at sub-tick position `sub` (03a §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Command {
    pub player: PlayerId,
    pub seq: u32,
    pub tick: Tick,
    pub sub: SubTick,
    pub kind: CommandKind,
}

/// An analytic line missile (03a §6): position is a closed-form function of these fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Missile {
    pub id: u32,
    pub owner: UnitId,
    pub team: Team,
    pub origin: Vec2,
    pub dir: Vec2,
    pub spec: LineSkillshot,
    pub spawn_at: SimTime,
    /// Sequence number of the cast command (lets the caster's client match its prediction).
    pub cast_seq: u32,
}

impl Missile {
    pub fn end_at(&self) -> SimTime {
        let life = (self.spec.range / self.spec.speed * SUBTICKS_PER_SECOND as f32) as u64;
        SimTime(self.spawn_at.0 + life)
    }

    /// Center position at `t`, clamped to the missile's lifetime.
    pub fn position_at(&self, t: SimTime) -> Vec2 {
        let s = t.secs_since(self.spawn_at).min(self.spec.range / self.spec.speed);
        self.origin + self.dir * (self.spec.speed * s)
    }

    /// Earliest contact in `[a, b]` with a target moving linearly from `q0` (at `s0`) to `q1`
    /// (at `s0 + 1 tick`). The exact rule the server uses, shared with client display code.
    pub fn first_hit(&self, a: SimTime, b: SimTime, s0: SimTime, q0: Vec2, q1: Vec2, radius: f32) -> Option<SimTime> {
        if a > b {
            return None;
        }
        let at = |t: SimTime| q0.lerp(q1, (t.0 - s0.0) as f32 / SUBTICKS as f32);
        let tau = first_contact(self.position_at(a), self.position_at(b), at(a), at(b), self.spec.radius + radius)?;
        Some(SimTime(a.0 + (tau * (b.0 - a.0) as f32) as u64))
    }
}

/// Things that happened during a step, for the network layer and the client display.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimEvent {
    CastStarted {
        unit: UnitId,
        at: SimTime,
        dir: Vec2,
        fire_at: SimTime,
        seq: u32,
    },
    /// In a world with missiles disabled (client prediction) the id is 0.
    MissileSpawned(Missile),
    MissileHit {
        id: u32,
        target: UnitId,
        at: SimTime,
    },
    MissileExpired {
        id: u32,
        at: SimTime,
    },
}

pub const CHAMPION_MOVE_SPEED: f32 = 325.0;
pub const CHAMPION_COLLISION_RADIUS: f32 = 35.0;
pub const CHAMPION_GAMEPLAY_RADIUS: f32 = 65.0;
pub const MINION_MOVE_SPEED: f32 = 325.0;
pub const TURRET_COLLISION_RADIUS: f32 = 60.0;
pub const TURRET_GAMEPLAY_RADIUS: f32 = 80.0;

impl MinionKind {
    /// (collision radius, gameplay radius), 01 §4 *(start)* values.
    pub fn radii(self) -> (f32, f32) {
        match self {
            MinionKind::Melee | MinionKind::Caster => (25.0, 48.0),
            MinionKind::Siege => (35.0, 65.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct World {
    tick: Tick,
    units: Vec<Unit>,
    next_unit: u32,
    rng: Pcg32,
    missiles: Vec<Missile>,
    map: Arc<Map>,
    next_missile: u32,
    missiles_enabled: bool,
    events: Vec<SimEvent>,
}

impl World {
    pub fn new(seed: u64) -> Self {
        Self {
            tick: Tick(0),
            units: Vec::new(),
            next_unit: 1,
            rng: Pcg32::new(seed, 0x4d46_5452),
            missiles: Vec::new(),
            next_missile: 1,
            missiles_enabled: true,
            events: Vec::new(),
            map: MapId::Open.shared(),
        }
    }

    /// The last simulated tick.
    pub fn tick(&self) -> Tick {
        self.tick
    }

    pub fn rng(&mut self) -> &mut Pcg32 {
        &mut self.rng
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    pub fn unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.iter_mut().find(|u| u.id == id)
    }

    pub fn missiles(&self) -> &[Missile] {
        &self.missiles
    }

    /// Client prediction runs without missiles: hits on others are never predicted (03a §7),
    /// but own casts still emit `MissileSpawned` (id 0) so the client can draw them at once.
    pub fn set_missiles_enabled(&mut self, enabled: bool) {
        self.missiles_enabled = enabled;
        if !enabled {
            self.missiles.clear();
        }
    }

    /// Events produced since the last call.
    pub fn take_events(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }

    fn next_id(&mut self) -> UnitId {
        let id = UnitId(self.next_unit);
        self.next_unit += 1;
        id
    }

    pub fn spawn_champion(&mut self, owner: PlayerId, team: Team, pos: Vec2) -> UnitId {
        let id = self.next_id();
        self.units.push(Unit {
            id,
            kind: UnitKind::Champion,
            owner: Some(owner),
            team,
            state: UnitState::new(pos, CHAMPION_MOVE_SPEED),
            collision_radius: CHAMPION_COLLISION_RADIUS,
            gameplay_radius: CHAMPION_GAMEPLAY_RADIUS,
            brain: None,
        });
        id
    }

    pub fn spawn_minion(&mut self, kind: MinionKind, team: Team, pos: Vec2, brain: Option<Brain>) -> UnitId {
        let id = self.next_id();
        let (collision_radius, gameplay_radius) = kind.radii();
        self.units.push(Unit {
            id,
            kind: UnitKind::Minion,
            owner: None,
            team,
            state: UnitState::new(pos, MINION_MOVE_SPEED),
            collision_radius,
            gameplay_radius,
            brain,
        });
        id
    }

    pub fn spawn_turret(&mut self, team: Team, pos: Vec2, range: u16) -> UnitId {
        let id = self.next_id();
        self.units.push(Unit {
            id,
            kind: UnitKind::Turret,
            owner: None,
            team,
            state: UnitState::new(pos, 0.0),
            collision_radius: TURRET_COLLISION_RADIUS,
            gameplay_radius: TURRET_GAMEPLAY_RADIUS,
            brain: Some(Brain::Turret { range }),
        });
        id
    }

    pub fn despawn(&mut self, id: UnitId) {
        self.units.retain(|u| u.id != id);
    }

    /// Build a partial world, as client prediction does (own unit plus collision proxies).
    pub fn from_units(tick: Tick, units: Vec<Unit>) -> Self {
        let next_unit = units.iter().map(|u| u.id.0 + 1).max().unwrap_or(1);
        Self { tick, units, next_unit, ..World::new(0) }
    }

    /// Replace every unit except `keep` (client prediction refreshes its collision proxies).
    pub fn replace_others(&mut self, keep: UnitId, others: impl IntoIterator<Item = Unit>) {
        self.units.retain(|u| u.id == keep);
        self.units.extend(others.into_iter().filter(|u| u.id != keep));
        self.units.sort_by_key(|u| u.id);
    }

    /// Rewind or fast-forward the tick counter (prediction reconciliation only).
    pub fn map(&self) -> &Arc<Map> {
        &self.map
    }

    /// Use a map (server: at match start; client: from the welcome). Walls and paths then
    /// apply to every unit.
    pub fn set_map(&mut self, map: Arc<Map>) {
        self.map = map;
    }

    pub fn set_tick(&mut self, tick: Tick) {
        self.tick = tick;
    }

    /// Simulate the next tick. Only commands whose `tick` equals the new tick are applied, in
    /// `(sub, player, seq)` order, each at its exact sub-tick instant.
    pub fn step(&mut self, commands: &[Command]) {
        let k = self.tick.next();
        let s0 = SimTime::end_of(self.tick);
        let s1 = SimTime::end_of(k);
        let mut cmds: Vec<&Command> = commands.iter().filter(|c| c.tick == k).collect();
        cmds.sort_by_key(|c| (c.sub, c.player, c.seq));
        let World { units, rng, missiles, next_missile, missiles_enabled, events, map, .. } = self;
        let map: &Map = map;

        // Phase 1: AI.
        let champions: Vec<(Team, Vec2, Option<Vec2>, f32)> = units
            .iter()
            .filter(|u| u.kind == UnitKind::Champion)
            .map(|u| (u.team, u.state.pos, u.state.heading(), u.state.move_speed))
            .collect();
        for unit in units.iter_mut() {
            match unit.brain {
                Some(Brain::Patrol { a, b, toward_b }) if unit.state.order == Order::Idle => {
                    unit.state.set_order(Order::MoveTo(if toward_b { b } else { a }), map);
                    unit.brain = Some(Brain::Patrol { a, b, toward_b: !toward_b });
                }
                Some(Brain::Turret { range }) if unit.state.can_cast(s0) => {
                    let me = unit.state.pos;
                    let mut best: Option<(f32, Vec2, Option<Vec2>, f32)> = None;
                    for &(team, pos, heading, speed) in &champions {
                        let d = pos.distance(me);
                        if team != unit.team && d <= range as f32 && best.is_none_or(|(bd, ..)| d < bd) {
                            best = Some((d, pos, heading, speed));
                        }
                    }
                    let Some((d, pos, heading, speed)) = best else { continue };
                    let spec = unit.skillshot();
                    let mut aim = pos;
                    if rng.next_u32() % 2 == 1
                        && let Some(h) = heading
                    {
                        // Lead the target: where it will be when the missile arrives.
                        let t = spec.windup.0 as f32 / SUBTICKS_PER_SECOND as f32 + d / spec.speed;
                        let to = h - pos;
                        let len = to.length();
                        let travel = (speed * t).min(len);
                        if len > 0.0 {
                            aim = pos + to * (travel / len);
                        }
                    }
                    start_cast(&mut unit.state, unit.id, spec, aim, s0, 0, events);
                }
                _ => {}
            }
        }

        // Phase 2: per-unit timelines against start-of-tick positions (order-independent).
        let starts: Vec<(UnitId, UnitKind, Team, Obstacle)> = units
            .iter()
            .filter(|u| u.collision_radius > 0.0)
            .map(|u| (u.id, u.kind, u.team, Obstacle { pos: u.state.pos, radius: u.collision_radius }))
            .collect();
        let mut fired: Vec<(UnitId, Team, Vec2, Cast, LineSkillshot)> = Vec::new();
        for unit in units.iter_mut() {
            let here = unit.state.pos;
            let (kind, team) = (unit.kind, unit.team);
            let obstacles: Vec<Obstacle> = starts
                .iter()
                .filter(|(id, k, t, o)| {
                    *id != unit.id && blocks(kind, team, *k, *t) && (o.pos - here).length_sq() < BROADPHASE * BROADPHASE
                })
                .map(|(_, _, _, o)| *o)
                .collect();
            let radius = unit.collision_radius;
            let spec = unit.skillshot();
            let mine: Vec<&Command> = match unit.owner {
                Some(owner) => cmds.iter().copied().filter(|c| c.player == owner).collect(),
                None => Vec::new(),
            };
            let (mut desired, mut achieved) = (0.0f32, 0.0f32);
            let mut t = s0;
            let mut next_cmd = 0;
            loop {
                let cmd_at = mine.get(next_cmd).map(|c| SimTime::at(k, c.sub));
                let fire_at = unit.state.cast.map(|c| c.fire_at).filter(|f| *f > t && *f <= s1);
                let unstun = Some(unit.state.stunned_until).filter(|u| *u > t && *u <= s1);
                let next = [cmd_at, fire_at, unstun, Some(s1)].into_iter().flatten().min().unwrap_or(s1);
                if next > t && unit.state.can_move(t) {
                    let dt = (next.0 - t.0) as f32 / SUBTICKS_PER_SECOND as f32;
                    let (d, a) = unit.state.advance(dt, radius, &obstacles, map);
                    desired += d;
                    achieved += a;
                }
                t = next;
                if let Some(c) = unit.state.cast
                    && c.fire_at == t
                {
                    fired.push((unit.id, unit.team, unit.state.pos, c, spec));
                    unit.state.cast = None;
                }
                while let Some(c) = mine.get(next_cmd)
                    && SimTime::at(k, c.sub) == t
                {
                    match c.kind {
                        CommandKind::MoveTo(q) => unit.state.set_order(Order::MoveTo(q), map),
                        CommandKind::Stop => unit.state.set_order(Order::Idle, map),
                        CommandKind::CastQ(q) => {
                            if unit.state.can_cast(t) {
                                start_cast(&mut unit.state, unit.id, spec, q.to_vec2(), t, c.seq, events);
                            }
                        }
                    }
                    next_cmd += 1;
                }
                if t >= s1 {
                    break;
                }
            }
            update_stuck(&mut unit.state, desired, achieved, radius, &obstacles, map);
        }

        // Phase 3: missiles.
        for (owner, team, origin, cast, spec) in fired {
            let id = if *missiles_enabled {
                *next_missile += 1;
                *next_missile - 1
            } else {
                0
            };
            let m =
                Missile { id, owner, team, origin, dir: cast.dir, spec, spawn_at: cast.fire_at, cast_seq: cast.seq };
            events.push(SimEvent::MissileSpawned(m));
            if *missiles_enabled {
                missiles.push(m);
            }
        }
        let motion: Vec<(UnitId, Team, UnitKind, f32, Vec2, Vec2)> = units
            .iter()
            .map(|u| {
                let start = starts.iter().find(|(id, ..)| *id == u.id).map_or(u.state.pos, |(.., o)| o.pos);
                (u.id, u.team, u.kind, u.gameplay_radius, start, u.state.pos)
            })
            .collect();
        missiles.retain(|m| {
            let a = m.spawn_at.max(s0);
            let b = m.end_at().min(s1);
            let mut hit: Option<(SimTime, UnitId)> = None;
            for &(id, team, kind, r, q0, q1) in &motion {
                if team == m.team || kind == UnitKind::Turret || id == m.owner {
                    continue;
                }
                if let Some(at) = m.first_hit(a, b, s0, q0, q1, r)
                    && hit.is_none_or(|(best, bid)| (at, id) < (best, bid))
                {
                    hit = Some((at, id));
                }
            }
            if let Some((at, target)) = hit {
                events.push(SimEvent::MissileHit { id: m.id, target, at });
                if let Some(u) = units.iter_mut().find(|u| u.id == target) {
                    u.state.stunned_until = u.state.stunned_until.max(at.plus(m.spec.stun));
                    u.state.cast = None; // hard CC interrupts casts
                }
                return false;
            }
            if m.end_at() <= s1 {
                events.push(SimEvent::MissileExpired { id: m.id, at: m.end_at() });
                return false;
            }
            true
        });
        self.tick = k;
    }

    pub fn state_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        h.write_u32(self.tick.0);
        let (s, i) = self.rng.state_parts();
        h.write_u64(s);
        h.write_u64(i);
        for u in &self.units {
            h.write_u32(u.id.0);
            h.write_u8(u.owner.map_or(0xff, |p| p.0));
            h.write_u8(u.team as u8);
            u.state.hash_into(&mut h);
            match u.brain {
                None => h.write_u8(0),
                Some(Brain::Patrol { toward_b, .. }) => h.write_u8(1 + toward_b as u8),
                Some(Brain::Turret { range }) => {
                    h.write_u8(3);
                    h.write_u16(range);
                }
            }
        }
        h.write_u32(self.next_missile);
        for m in &self.missiles {
            h.write_u32(m.id);
            h.write_f32(m.origin.x);
            h.write_f32(m.origin.y);
            h.write_f32(m.dir.x);
            h.write_f32(m.dir.y);
            h.write_u64(m.spawn_at.0);
        }
        h.finish()
    }
}

fn start_cast(
    state: &mut UnitState,
    unit: UnitId,
    spec: LineSkillshot,
    target: Vec2,
    at: SimTime,
    seq: u32,
    events: &mut Vec<SimEvent>,
) {
    let dir = (target - state.pos).normalize_or_zero();
    if dir == Vec2::ZERO {
        return;
    }
    let fire_at = at.plus(spec.windup);
    state.cast = Some(Cast { dir, fire_at, seq });
    state.q_ready_at = at.plus(spec.cooldown);
    events.push(SimEvent::CastStarted { unit, at, dir, fire_at, seq });
}

/// Whether a unit of `other_kind`/`other_team` blocks a mover of `kind`/`team` (D11, D23).
/// Every unit with a collision radius blocks every other: allied and enemy champions, all
/// minions, turrets. Measured in the reference game (R03: a champion stops at contact with a
/// standing ally and paths around it). Kept as a function so ghosting effects can opt out.
pub fn blocks(_kind: UnitKind, _team: Team, _other_kind: UnitKind, _other_team: Team) -> bool {
    true
}

/// Stuck detection and detours (03a §5): poor progress for a few ticks → take a short detour
/// around the blockers. If the goal itself is occupied (clicked into a clump) or already
/// within reach, stop where we are, like the reference game does.
fn update_stuck(state: &mut UnitState, desired: f32, achieved: f32, radius: f32, obstacles: &[Obstacle], map: &Map) {
    if desired <= 0.5 || achieved >= desired * STUCK_PROGRESS {
        state.stuck = 0;
        return;
    }
    state.stuck = state.stuck.saturating_add(1);
    if state.stuck < STUCK_TICKS {
        return;
    }
    state.stuck = 0;
    let Order::MoveTo(goal) = state.order else { return };
    let goal = goal.to_vec2();
    let occupied = obstacles.iter().any(|o| (goal - o.pos).length_sq() < (radius + o.radius) * (radius + o.radius));
    if occupied || (goal - state.pos).length() < GIVE_UP_DISTANCE {
        state.set_order(Order::Idle, map);
        return;
    }
    let reach = 2.0 * (radius + 35.0);
    state.detour = choose_detour(state.pos, goal, radius, reach, obstacles, map.edges());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::SUBTICKS;

    fn cmd(player: u8, seq: u32, tick: u32, sub: u8, target: (f32, f32)) -> Command {
        Command {
            player: PlayerId(player),
            seq,
            tick: Tick(tick),
            sub: SubTick::new(sub),
            kind: CommandKind::MoveTo(QPoint::from_vec2(Vec2::new(target.0, target.1))),
        }
    }

    /// A tight hex clump of minions around `center` (adjacent minions touch).
    fn clump(w: &mut World, center: Vec2, rings: i32) {
        let s = 50.0;
        for q in -rings..=rings {
            for r in -rings..=rings {
                if (q + r).abs() > rings {
                    continue;
                }
                let x = center.x + s * (q as f32 + r as f32 * 0.5);
                let y = center.y + s * 0.866_025_4 * r as f32;
                w.spawn_minion(MinionKind::Melee, Team::Red, Vec2::new(x, y), None);
            }
        }
    }

    #[test]
    fn moves_at_constant_speed_and_stops_at_target() {
        let mut w = World::new(1);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (2000.0, 1000.0))]);
        let x = w.unit(id).unwrap().state.pos.x;
        assert!((x - (1000.0 + 325.0 / 30.0)).abs() < 1e-3, "{x}");
        for _ in 0..200 {
            w.step(&[]);
        }
        let s = w.unit(id).unwrap().state;
        assert_eq!(s.pos, Vec2::new(2000.0, 1000.0));
        assert_eq!(s.order, Order::Idle);
    }

    #[test]
    fn subtick_command_moves_only_for_the_rest_of_the_tick() {
        let mut w = World::new(1);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, SUBTICKS / 2, (2000.0, 1000.0))]);
        let x = w.unit(id).unwrap().state.pos.x;
        assert!((x - (1000.0 + 325.0 / 60.0)).abs() < 1e-3, "{x}");
    }

    #[test]
    fn commands_for_other_ticks_are_ignored() {
        let mut w = World::new(1);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 5, 0, (2000.0, 1000.0))]);
        assert_eq!(w.unit(id).unwrap().state.order, Order::Idle);
    }

    #[test]
    fn champion_walks_around_a_minion_clump() {
        let mut w = World::new(1);
        clump(&mut w, Vec2::new(1500.0, 1000.0), 2);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (2000.0, 1000.0))]);
        let mut min_gap = f32::INFINITY;
        for _ in 0..300 {
            w.step(&[]);
            let me = w.unit(id).unwrap().state.pos;
            for u in w.units().iter().filter(|u| u.id != id) {
                min_gap = min_gap.min(me.distance(u.state.pos) - 60.0);
            }
        }
        let s = w.unit(id).unwrap().state;
        assert!(min_gap > -0.05, "penetrated a minion by {}", -min_gap);
        assert_eq!(s.order, Order::Idle, "never arrived: {s:?}");
        assert!(s.pos.distance(Vec2::new(2000.0, 1000.0)) < 1.0, "{s:?}");
    }

    #[test]
    fn clicking_into_a_clump_stops_at_its_edge() {
        let mut w = World::new(1);
        clump(&mut w, Vec2::new(1500.0, 1000.0), 2);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (1500.0, 1000.0))]); // the clump's center
        for _ in 0..200 {
            w.step(&[]);
        }
        let s = w.unit(id).unwrap().state;
        assert_eq!(s.order, Order::Idle, "{s:?}");
        assert!(s.pos.x < 1400.0, "{s:?}");
    }

    #[test]
    fn patrol_minions_walk_back_and_forth() {
        let mut w = World::new(1);
        let a = QPoint::from_vec2(Vec2::new(1000.0, 1000.0));
        let b = QPoint::from_vec2(Vec2::new(1300.0, 1000.0));
        let id =
            w.spawn_minion(MinionKind::Caster, Team::Blue, a.to_vec2(), Some(Brain::Patrol { a, b, toward_b: true }));
        let mut max_x: f32 = 0.0;
        for _ in 0..90 {
            w.step(&[]);
            max_x = max_x.max(w.unit(id).unwrap().state.pos.x);
        }
        assert_eq!(max_x, 1300.0);
        assert!(w.unit(id).unwrap().state.pos.x < 1300.0, "should be heading back");
    }

    /// R03: walking at a standing champion, ally or enemy, stops at contact and then paths
    /// around it to the far side.
    #[test]
    fn champions_path_around_allied_and_enemy_champions() {
        for other_team in [Team::Blue, Team::Red] {
            let mut w = World::new(1);
            let me = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
            let other = w.spawn_champion(PlayerId(1), other_team, Vec2::new(1200.0, 1000.0)); // in the way
            w.step(&[cmd(0, 1, 1, 0, (1400.0, 1000.0))]);
            let (mut max_dev, mut min_gap) = (0.0f32, f32::INFINITY);
            for _ in 0..90 {
                w.step(&[]);
                let p = w.unit(me).unwrap().state.pos;
                max_dev = max_dev.max((p.y - 1000.0).abs());
                min_gap = min_gap.min(p.distance(w.unit(other).unwrap().state.pos));
            }
            let end = w.unit(me).unwrap().state.pos;
            assert!(min_gap >= 70.0 - 0.05, "{other_team:?}: walked through (gap {min_gap})");
            assert!(max_dev > 30.0, "{other_team:?}: should path around: {end:?}");
            assert!(end.distance(Vec2::new(1400.0, 1000.0)) < 1.0, "{other_team:?}: should arrive: {end:?}");
        }
    }

    /// Re-simulating the same commands from a snapshot must be bit-identical:
    /// this is what client prediction relies on. Players stay in separate halves of the map so
    /// the partial world (own unit only) sees the same obstacles as the full one.
    #[test]
    fn partial_world_replay_is_bit_exact() {
        let mut full = World::new(7);
        let a = full.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(500.0, 500.0));
        full.spawn_champion(PlayerId(1), Team::Red, Vec2::new(7000.0, 7000.0));
        let mut rng = Pcg32::new(99, 1);
        let mut cmds = Vec::new();
        for i in 0..300u32 {
            if rng.next_u32() % 5 == 0 {
                let p = (rng.next_u32() % 2) as u8;
                let sub = (rng.next_u32() % SUBTICKS as u32) as u8;
                let base = if p == 0 { 0.0 } else { 6000.0 };
                let t = (base + rng.range_f32(0.0, 3000.0), base + rng.range_f32(0.0, 3000.0));
                cmds.push(cmd(p, i, i + 1, sub, t));
            }
        }
        for k in 1..=100 {
            full.step(&cmds.iter().copied().filter(|c| c.tick == Tick(k)).collect::<Vec<_>>());
        }
        let mut partial = World::from_units(full.tick(), vec![full.unit(a).unwrap().clone()]);
        for k in 101..=300 {
            let tick_cmds: Vec<Command> = cmds.iter().copied().filter(|c| c.tick == Tick(k)).collect();
            full.step(&tick_cmds);
            partial.step(&tick_cmds);
            assert!(partial.unit(a).unwrap().state.bits_eq(&full.unit(a).unwrap().state), "diverged at {k}");
        }
    }

    /// Prediction with *exact* proxies (the other units' true start-of-tick positions) must
    /// reproduce the server bit-exactly, even while colliding: the property 03a §5 builds on.
    #[test]
    fn prediction_with_exact_proxies_matches_through_collisions() {
        let mut full = World::new(3);
        clump(&mut full, Vec2::new(1500.0, 1000.0), 2);
        let a = QPoint::from_vec2(Vec2::new(1200.0, 700.0));
        let b = QPoint::from_vec2(Vec2::new(1200.0, 1300.0));
        full.spawn_minion(MinionKind::Siege, Team::Blue, a.to_vec2(), Some(Brain::Patrol { a, b, toward_b: true }));
        let me = full.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        let mut rng = Pcg32::new(5, 5);
        for k in 1..=600u32 {
            let tick_cmds: Vec<Command> = if rng.next_u32() % 9 == 0 {
                let t = (rng.range_f32(900.0, 2100.0), rng.range_f32(600.0, 1400.0));
                vec![cmd(0, k, k, (rng.next_u32() % 64) as u8, t)]
            } else {
                Vec::new()
            };
            // Client-style prediction: own unit + every other unit as a static proxy.
            let own = full.unit(me).unwrap().clone();
            let proxies = full.units().iter().filter(|u| u.id != me).map(|u| Unit {
                state: UnitState::new(u.state.pos, 0.0),
                brain: None,
                ..u.clone()
            });
            let mut predicted = World::from_units(full.tick(), vec![own]);
            predicted.replace_others(me, proxies);
            predicted.step(&tick_cmds);
            full.step(&tick_cmds);
            assert!(predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state), "diverged at tick {k}");
        }
    }

    fn cast(player: u8, seq: u32, tick: u32, sub: u8, target: (f32, f32)) -> Command {
        Command {
            player: PlayerId(player),
            seq,
            tick: Tick(tick),
            sub: SubTick::new(sub),
            kind: CommandKind::CastQ(QPoint::from_vec2(Vec2::new(target.0, target.1))),
        }
    }

    fn run_until_quiet(w: &mut World, ticks: u32) -> Vec<SimEvent> {
        let mut ev = Vec::new();
        for _ in 0..ticks {
            w.step(&[]);
            ev.extend(w.take_events());
        }
        ev
    }

    #[test]
    fn cast_roots_for_the_windup_then_resumes_the_move() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (3000.0, 1000.0)), cast(0, 2, 1, 32, (1000.0, 3000.0))]);
        let ev = w.take_events();
        let fire_at = match ev[0] {
            SimEvent::CastStarted { at, fire_at, .. } => {
                assert_eq!(at, SimTime(32));
                fire_at
            }
            e => panic!("{e:?}"),
        };
        assert_eq!(fire_at, SimTime(32 + 480), "0.25 s windup = 480 sub-ticks");
        // Moved for half a tick, then rooted.
        let x_after_first = w.unit(me).unwrap().state.pos.x;
        assert!((x_after_first - (1000.0 + 325.0 / 60.0)).abs() < 1e-3);
        for _ in 0..6 {
            w.step(&[]);
        }
        assert_eq!(w.unit(me).unwrap().state.pos.x, x_after_first, "rooted during windup");
        let ev = run_until_quiet(&mut w, 3);
        assert!(
            ev.iter().any(|e| matches!(e, SimEvent::MissileSpawned(m) if m.spawn_at == fire_at && m.cast_seq == 2))
        );
        assert!(w.unit(me).unwrap().state.pos.x > x_after_first, "resumes the queued move after firing");
    }

    #[test]
    fn missile_hits_and_stuns_a_stationary_enemy() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, Vec2::new(1800.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (1800.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 40);
        let hit = ev.iter().find_map(|e| match e {
            SimEvent::MissileHit { target, at, .. } => Some((*target, *at)),
            _ => None,
        });
        let (target, at) = hit.expect("should hit");
        assert_eq!(target, enemy);
        // Contact when the gap is 35 + 65 = 100 u: 700 u at 1600 u/s after the 0.25 s windup.
        let expected = 480.0 + 700.0 / 1600.0 * 1920.0;
        assert!((at.0 as f32 - expected).abs() <= 1.0, "{} vs {expected}", at.0);
        let e = w.unit(enemy).unwrap().state;
        assert_eq!(e.stunned_until, at.plus(SANDBOX_LANCE.stun));
    }

    #[test]
    fn walking_out_in_time_dodges_and_too_late_is_hit() {
        for (react_tick, expect_hit) in [(6u32, false), (20u32, true)] {
            let mut w = World::new(1);
            w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
            w.spawn_champion(PlayerId(1), Team::Red, Vec2::new(1800.0, 1000.0));
            let mut ev = Vec::new();
            for k in 1..=45u32 {
                let mut c = Vec::new();
                if k == 1 {
                    c.push(cast(0, 1, 1, 0, (1800.0, 1000.0)));
                }
                if k == react_tick {
                    c.push(cmd(1, 2, k, 0, (1800.0, 1400.0)));
                }
                w.step(&c);
                ev.extend(w.take_events());
            }
            let hit = ev.iter().any(|e| matches!(e, SimEvent::MissileHit { .. }));
            assert_eq!(hit, expect_hit, "reacting at tick {react_tick}");
        }
    }

    #[test]
    fn an_enemy_minion_in_the_way_takes_the_skillshot() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        let minion = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1400.0, 1010.0), None);
        w.spawn_minion(MinionKind::Caster, Team::Blue, Vec2::new(1200.0, 1000.0), None); // allied: ignored
        w.spawn_champion(PlayerId(1), Team::Red, Vec2::new(1800.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (1800.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 40);
        let targets: Vec<UnitId> = ev
            .iter()
            .filter_map(|e| match e {
                SimEvent::MissileHit { target, .. } => Some(*target),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec![minion]);
    }

    #[test]
    fn cooldown_blocks_recast_and_expired_missiles_report() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (2000.0, 1000.0))]);
        w.step(&[cast(0, 2, 2, 0, (2000.0, 1000.0))]); // on cooldown (and still casting)
        let ev = run_until_quiet(&mut w, 60);
        let mut all = w.take_events();
        all.extend(ev);
        let spawned = all.iter().filter(|e| matches!(e, SimEvent::MissileSpawned(_))).count();
        assert_eq!(spawned, 1);
        assert!(all.iter().any(|e| matches!(e, SimEvent::MissileExpired { .. })));
    }

    #[test]
    fn turret_shoots_the_nearest_enemy_champion() {
        let mut w = World::new(1);
        w.spawn_turret(Team::Red, Vec2::new(1000.0, 1000.0), 1100);
        let target = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1600.0, 1000.0));
        w.spawn_champion(PlayerId(1), Team::Red, Vec2::new(1300.0, 1000.0)); // ally of the turret
        let ev = run_until_quiet(&mut w, 40);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::MissileHit { target: t, .. } if *t == target)));
    }

    /// Client-style prediction (missiles disabled) must still reproduce the caster exactly.
    #[test]
    fn prediction_without_missiles_matches_the_caster() {
        let mut full = World::new(9);
        let me = full.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(1000.0, 1000.0));
        full.spawn_turret(Team::Red, Vec2::new(5000.0, 5000.0), 1100); // out of range
        let mut predicted = World::from_units(full.tick(), vec![full.unit(me).unwrap().clone()]);
        predicted.set_missiles_enabled(false);
        let mut rng = Pcg32::new(3, 3);
        for k in 1..=900u32 {
            let mut c = Vec::new();
            if rng.next_u32() % 7 == 0 {
                let t = (rng.range_f32(500.0, 2500.0), rng.range_f32(500.0, 2500.0));
                let sub = (rng.next_u32() % 64) as u8;
                c.push(if rng.next_u32() % 3 == 0 { cast(0, k, k, sub, t) } else { cmd(0, k, k, sub, t) });
            }
            full.step(&c);
            predicted.step(&c);
            assert!(predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state), "tick {k}");
        }
    }

    fn arena_world(seed: u64) -> World {
        let mut w = World::new(seed);
        w.set_map(MapId::Arena.shared());
        w
    }

    #[test]
    fn champion_paths_around_a_wall_and_never_enters_one() {
        let mut w = arena_world(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(2200.0, 1200.0));
        w.step(&[cmd(0, 1, 1, 0, (2700.0, 1200.0))]); // straight line crosses the long wall
        let map = w.map().clone();
        for _ in 0..600 {
            w.step(&[]);
            let p = w.unit(me).unwrap().state.pos;
            assert!(map.walkable(p, CHAMPION_COLLISION_RADIUS - 0.05), "inside a wall at {p:?}");
        }
        let s = w.unit(me).unwrap().state;
        assert_eq!(s.order, Order::Idle);
        assert!(s.pos.distance(Vec2::new(2700.0, 1200.0)) < 1.0, "{s:?}");
    }

    /// Prediction with the shared map and exact proxies stays bit-exact through walls, paths,
    /// re-plans and unit collision.
    #[test]
    fn prediction_with_map_matches_the_server() {
        let mut full = arena_world(4);
        clump(&mut full, Vec2::new(1200.0, 1200.0), 1);
        let me = full.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(2000.0, 2000.0));
        full.spawn_champion(PlayerId(1), Team::Red, Vec2::new(2200.0, 2100.0));
        let mut rng = Pcg32::new(8, 8);
        for k in 1..=1500u32 {
            let mut c = Vec::new();
            if rng.next_u32() % 11 == 0 {
                let t = (rng.range_f32(300.0, 3700.0), rng.range_f32(300.0, 3700.0));
                c.push(cmd(0, k, k, (rng.next_u32() % 64) as u8, t));
            }
            if rng.next_u32() % 13 == 0 {
                let t = (rng.range_f32(300.0, 3700.0), rng.range_f32(300.0, 3700.0));
                c.push(cmd(1, k, k, 0, t));
            }
            let own = full.unit(me).unwrap().clone();
            let proxies = full.units().iter().filter(|u| u.id != me).map(|u| Unit {
                state: UnitState::new(u.state.pos, 0.0),
                brain: None,
                ..u.clone()
            });
            let mut predicted = World::from_units(full.tick(), vec![own]);
            predicted.set_map(MapId::Arena.shared());
            predicted.replace_others(me, proxies);
            predicted.step(&c);
            full.step(&c);
            assert!(predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state), "tick {k}");
        }
    }

    /// Determinism canary for pathfinding and wall collision (arena map).
    #[test]
    fn golden_state_hash_arena() {
        let mut w = arena_world(0xA4E7A);
        for p in 0..6u8 {
            let team = if p % 2 == 0 { Team::Blue } else { Team::Red };
            w.spawn_champion(PlayerId(p), team, Vec2::new(600.0 + p as f32 * 500.0, 3300.0));
        }
        clump(&mut w, Vec2::new(1200.0, 1200.0), 1);
        let mut rng = Pcg32::new(77, 7);
        let mut seq = 0;
        for k in 1..=6000u32 {
            let mut cmds = Vec::new();
            for p in 0..6u8 {
                if rng.next_u32() % 23 == 0 {
                    seq += 1;
                    let t = (rng.range_f32(0.0, 4000.0), rng.range_f32(0.0, 4000.0));
                    cmds.push(cmd(p, seq, k, (rng.next_u32() % SUBTICKS as u32) as u8, t));
                }
            }
            w.step(&cmds);
            w.take_events();
        }
        assert_eq!(w.state_hash(), GOLDEN_HASH_ARENA, "hash = {:#018x}", w.state_hash());
    }

    const GOLDEN_HASH_ARENA: u64 = 0xe294_54c2_61b9_7f5d;

    /// Cross-platform determinism canary: a scripted match must hash to the same value on
    /// every OS and CPU. If this fails on one platform, the sim used non-deterministic math.
    #[test]
    fn golden_state_hash() {
        let mut w = World::new(0xC0FFEE);
        for p in 0..10u8 {
            let team = if p < 5 { Team::Blue } else { Team::Red };
            let x = 500.0 + p as f32 * 1300.0;
            w.spawn_champion(PlayerId(p), team, Vec2::new(x, 7000.0));
        }
        for i in 0..4 {
            clump(&mut w, Vec2::new(2000.0 + 3000.0 * i as f32, 5000.0), 2);
            let a = QPoint::from_vec2(Vec2::new(1000.0 + 3000.0 * i as f32, 9000.0));
            let b = QPoint::from_vec2(Vec2::new(3000.0 + 3000.0 * i as f32, 9000.0));
            for j in 0..6 {
                let start = a.to_vec2() + Vec2::new(0.0, 55.0 * j as f32);
                w.spawn_minion(MinionKind::Caster, Team::Blue, start, Some(Brain::Patrol { a, b, toward_b: true }));
            }
            w.spawn_turret(Team::Red, Vec2::new(1500.0 + 3500.0 * i as f32, 7600.0), 1100);
        }
        let mut rng = Pcg32::new(2024, 7);
        let mut seq = 0;
        for k in 1..=9000u32 {
            let mut cmds = Vec::new();
            for p in 0..10u8 {
                if rng.next_u32() % 17 == 0 {
                    seq += 1;
                    let sub = (rng.next_u32() % SUBTICKS as u32) as u8;
                    let t = (rng.range_f32(0.0, 14_800.0), rng.range_f32(0.0, 14_800.0));
                    // Mostly moves, some skillshots (casts, missiles, hits and stuns in the hash).
                    cmds.push(if rng.next_u32() % 4 == 0 { cast(p, seq, k, sub, t) } else { cmd(p, seq, k, sub, t) });
                }
            }
            w.step(&cmds);
            w.take_events();
        }
        assert_eq!(w.tick(), Tick(9000));
        assert_eq!(w.state_hash(), GOLDEN_HASH, "hash = {:#018x}", w.state_hash());
    }

    /// Recorded on x86_64-pc-windows-msvc. CI checks Linux, macOS (aarch64) and Windows.
    const GOLDEN_HASH: u64 = 0x194c_0375_f359_2afd;
}
