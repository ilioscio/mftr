//! The simulation world: units, commands and the fixed-tick step.
//!
//! Tick phases (04 §3):
//! 0. Respawns that are due.
//! 1. AI decisions for server-driven units (patrols, turrets).
//! 2. A per-unit timeline on the exact [`SimTime`] axis: movement with unit collision against
//!    start-of-tick positions (`collision.rs`), sub-tick commands, cast and attack windups
//!    (rooted), dashes, blinks, stuns and roots, each taking effect at its exact instant.
//!    Attack targets are judged against start-of-tick positions too, so the outcome is
//!    order-independent and client prediction reproduces it from proxies.
//! 3. Effects: missiles spawn at their fire instant and resolve with exact swept hits
//!    (`projectile::first_contact`), delayed areas detonate, homing attack bolts fly. Damage goes
//!    through the 02 §5 pipeline (resistances, shields, health, death).
//! 4. Regeneration and shield expiry.
//!
//! In **prediction mode** (the client's world: own unit plus proxies) phase 3 doesn't run:
//! hits on others are never predicted (03a §7). Own missiles, areas and bolts are still
//! announced (id 0) so the client can draw them at once.

use crate::ability::{Ability, Cc, DamageKind, Effect, LineSkillshot, SLOTS, TURRET_SHOT};
use crate::champion::{AttackSpec, ChampionId, Stats};
use crate::collision::{Obstacle, choose_detour, constrained_move};
use crate::combat::resist_multiplier;
use crate::hash::{StateHasher, StateSink};
use crate::map::{Map, MapId};
use crate::math::{QPoint, Vec2};
use crate::projectile::first_contact;
use crate::rng::Pcg32;
use crate::time::{SUBTICKS, SUBTICKS_PER_SECOND, SimDuration, SimTime, SubTick, TICK_DT, Tick};
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
    /// Static structure (the dodge rig's shooters for now). Immune to skillshots and attacks.
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
    /// Chase a unit until in attack range, then attack it on every attack timer.
    Attack(UnitId),
    /// Walk toward a point, attacking the nearest enemy that comes into range on the way.
    AttackMove(QPoint),
}

/// Ticks of poor progress before a blocked unit looks for a detour (03a §5).
const STUCK_TICKS: u8 = 3;
const STUCK_PROGRESS: f32 = 0.2;
/// Only units this close can matter for one tick of movement or a detour probe.
const BROADPHASE: f32 = 300.0;
/// A blocked unit this close to its goal counts as arrived.
const GIVE_UP_DISTANCE: f32 = 80.0;
/// Float slack for "in attack range" after halting at the range circle.
const RANGE_SLACK: f32 = 0.5;
/// An attack target that moved this far from the chase path's end gets a new path.
const REPATH_DISTANCE: f32 = 25.0;

/// A cast in progress: the caster is rooted until it fires.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cast {
    pub slot: u8,
    pub dir: Vec2,
    /// Target point (areas: already clamped to the ability's range).
    pub point: Vec2,
    pub fire_at: SimTime,
    /// Sequence number of the command that started it (0 for AI casts).
    pub seq: u32,
}

/// A basic attack winding up: the attacker is rooted; the bolt launches at `fire_at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackWindup {
    pub target: UnitId,
    pub fire_at: SimTime,
}

/// A dash in progress: constant speed toward `to`, ignoring units, sliding on walls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DashMove {
    pub dir: Vec2,
    pub to: Vec2,
    pub speed: f32,
    pub end_at: SimTime,
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

    fn goal(&self) -> Option<Vec2> {
        (self.len > 0).then(|| self.points[self.len as usize - 1])
    }
}

/// Everything client prediction needs to reproduce a unit bit-exactly (sent lossless for the
/// own champion, 03b §6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitState {
    pub pos: Vec2,
    pub order: Order,
    pub move_speed: f32,
    /// Waypoints toward the order's goal, around walls.
    pub path: Path,
    /// Temporary waypoint around blocking units, taken before resuming the path.
    pub detour: Option<Vec2>,
    /// Consecutive ticks of poor progress while moving.
    pub stuck: u8,
    pub cast: Option<Cast>,
    pub attack: Option<AttackWindup>,
    /// When the next basic attack may start (the attack timer, 02 §7).
    pub attack_ready_at: SimTime,
    pub dash: Option<DashMove>,
    pub stunned_until: SimTime,
    pub rooted_until: SimTime,
    /// When each slot (Q W E R D F) is off cooldown.
    pub cooldowns: [SimTime; SLOTS],
    pub health: f32,
    /// Shield points (Barrier), valid until `shield_until`.
    pub shield: f32,
    pub shield_until: SimTime,
    /// Dead until this instant (then respawns at home).
    pub respawn_at: Option<SimTime>,
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
            attack: None,
            attack_ready_at: SimTime(0),
            dash: None,
            stunned_until: SimTime(0),
            rooted_until: SimTime(0),
            cooldowns: [SimTime(0); SLOTS],
            health: 0.0,
            shield: 0.0,
            shield_until: SimTime(0),
            respawn_at: None,
        }
    }

    pub fn alive(&self) -> bool {
        self.respawn_at.is_none()
    }

    /// Where the unit is currently heading (detour first, then the path), if anywhere.
    pub fn heading(&self) -> Option<Vec2> {
        if let Some(d) = self.detour {
            return Some(d);
        }
        match self.order {
            Order::Idle => None,
            _ => self.path.waypoints().first().copied(),
        }
    }

    /// Replace the order (a command or AI decision) and plan the route on `map`.
    pub fn set_order(&mut self, order: Order, map: &Map) {
        self.order = order;
        self.detour = None;
        self.stuck = 0;
        self.route(map);
    }

    /// (Re)plan the path from the current position toward the order's goal. Attack orders
    /// plan toward their target's position when the timeline next looks at it.
    fn route(&mut self, map: &Map) {
        self.path = match self.order {
            Order::MoveTo(q) | Order::AttackMove(q) => Path::from_points(&map.find_path(self.pos, q.to_vec2())),
            Order::Idle | Order::Attack(_) => Path::EMPTY,
        };
        if matches!(self.order, Order::MoveTo(_) | Order::AttackMove(_)) && self.path.len == 0 {
            self.order = Order::Idle; // unreachable
        }
    }

    pub fn can_move(&self, at: SimTime) -> bool {
        self.alive()
            && self.cast.is_none()
            && self.attack.is_none()
            && self.dash.is_none()
            && self.stunned_until <= at
            && self.rooted_until <= at
    }

    /// Whether `slot` could be cast at `at` (alive, not stunned, not busy, off cooldown).
    pub fn can_cast(&self, at: SimTime, slot: u8) -> bool {
        self.alive()
            && self.cast.is_none()
            && self.dash.is_none()
            && self.stunned_until <= at
            && self.cooldowns.get(slot as usize).is_some_and(|c| *c <= at)
    }

    /// An attack windup cancelled by a new order resets the attack timer, with no damage (02 §7).
    fn cancel_attack(&mut self, at: SimTime) {
        if self.attack.take().is_some() {
            self.attack_ready_at = at;
        }
    }

    /// Advance by `dt` seconds toward the current heading at constant speed (instant turns,
    /// R01 §2), blocked by `obstacles` and the map's walls. A `halt` circle (attack range
    /// around a target) stops the move where it enters the circle. Returns
    /// `(desired, achieved, fraction of dt used)`.
    pub fn advance(
        &mut self,
        dt: f32,
        radius: f32,
        obstacles: &[Obstacle],
        map: &Map,
        halt: Option<(Vec2, f32)>,
    ) -> (f32, f32, f32) {
        if dt <= 0.0 {
            return (0.0, 0.0, 1.0);
        }
        let Some(target) = self.heading() else { return (0.0, 0.0, 1.0) };
        if let Some((c, reach)) = halt
            && (self.pos - c).length_sq() <= reach * reach
        {
            return (0.0, 0.0, 1.0);
        }
        let to = target - self.pos;
        let dist = to.length();
        let step = self.move_speed * dt;
        let (mut delta, mut arrives) = if step >= dist { (to, true) } else { (to * (step / dist), false) };
        let mut used = 1.0;
        if let Some((c, reach)) = halt
            && let Some(tau) = first_contact(self.pos, self.pos + delta, c, c, reach)
            && tau < 1.0
        {
            delta = delta * tau;
            arrives = false;
            used = tau;
        }
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
                    if matches!(self.order, Order::Attack(_)) {
                        self.path = Path::EMPTY; // the target moved on: re-plan toward it
                    } else if self.path.complete {
                        self.order = Order::Idle;
                        self.path = Path::EMPTY;
                    } else {
                        self.route(map);
                    }
                }
            }
        }
        (delta.length(), achieved, used)
    }

    /// One dash segment of `dt` seconds.
    fn dash_advance(&mut self, dt: f32, radius: f32, map: &Map) {
        let Some(d) = self.dash else { return };
        let rest = d.to - self.pos;
        let len = rest.length();
        let step = d.speed * dt;
        let delta = if step >= len { rest } else { d.dir * step };
        self.pos = constrained_move(self.pos, delta, radius, &[], map.edges());
    }

    /// Bit-exact equality: what prediction reconciliation compares.
    pub fn bits_eq(&self, other: &Self) -> bool {
        let (mut a, mut b) = (Vec::with_capacity(256), Vec::with_capacity(256));
        self.hash_into(&mut a);
        other.hash_into(&mut b);
        a == b
    }

    /// Serialize every field (state hash and [`Self::bits_eq`]).
    pub fn hash_into(&self, h: &mut impl StateSink) {
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
            Order::Attack(id) => {
                h.write_u8(2);
                h.write_u32(id.0);
            }
            Order::AttackMove(q) => {
                h.write_u8(3);
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
                h.write_u8(c.slot);
                h.write_f32(c.dir.x);
                h.write_f32(c.dir.y);
                h.write_f32(c.point.x);
                h.write_f32(c.point.y);
                h.write_u64(c.fire_at.0);
                h.write_u32(c.seq);
            }
        }
        match self.attack {
            None => h.write_u8(0),
            Some(a) => {
                h.write_u8(1);
                h.write_u32(a.target.0);
                h.write_u64(a.fire_at.0);
            }
        }
        h.write_u64(self.attack_ready_at.0);
        match self.dash {
            None => h.write_u8(0),
            Some(d) => {
                h.write_u8(1);
                h.write_f32(d.dir.x);
                h.write_f32(d.dir.y);
                h.write_f32(d.to.x);
                h.write_f32(d.to.y);
                h.write_f32(d.speed);
                h.write_u64(d.end_at.0);
            }
        }
        h.write_u64(self.stunned_until.0);
        h.write_u64(self.rooted_until.0);
        for c in self.cooldowns {
            h.write_u64(c.0);
        }
        h.write_f32(self.health);
        h.write_f32(self.shield);
        h.write_u64(self.shield_until.0);
        match self.respawn_at {
            None => h.write_u8(0),
            Some(t) => {
                h.write_u8(1);
                h.write_u64(t.0);
            }
        }
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
    pub champion: Option<ChampionId>,
    pub stats: Stats,
    /// Respawn point.
    pub home: Vec2,
}

impl Unit {
    /// The ability in `slot`: a champion's kit and utility spells, or the turret's shot.
    pub fn ability(&self, slot: u8) -> Option<Ability> {
        match (self.champion, self.kind) {
            (Some(c), _) => c.ability(slot),
            (None, UnitKind::Turret) if slot == 0 => Some(TURRET_SHOT),
            _ => None,
        }
    }

    pub fn attack_spec(&self) -> Option<AttackSpec> {
        self.champion.map(|c| c.def().attack)
    }

    /// Can be hit by skillshots, areas and attacks (turrets are immune for now).
    pub fn targetable(&self) -> bool {
        self.kind != UnitKind::Turret && self.state.alive()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandKind {
    MoveTo(QPoint),
    Stop,
    /// Cast the ability in `slot` (0–5 = Q W E R D F) toward a ground point.
    Cast {
        slot: u8,
        target: QPoint,
    },
    /// Basic-attack a unit (chasing it into range).
    Attack(UnitId),
    AttackMove(QPoint),
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
    /// Raw damage, from the caster's stats at fire time.
    pub power: f32,
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

/// A delayed ground AoE on its way to detonation (telegraphed from `spawn_at`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub id: u32,
    pub owner: UnitId,
    pub team: Team,
    pub center: Vec2,
    pub radius: f32,
    pub spawn_at: SimTime,
    pub detonate_at: SimTime,
    pub kind: DamageKind,
    pub power: f32,
    pub cast_seq: u32,
}

/// A homing basic-attack bolt: not dodgeable, flies at the target until it lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bolt {
    pub id: u32,
    pub owner: UnitId,
    pub team: Team,
    pub target: UnitId,
    pub origin: Vec2,
    pub pos: Vec2,
    pub speed: f32,
    pub launched_at: SimTime,
    pub power: f32,
}

/// Things that happened during a step, for the network layer and the client display.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimEvent {
    CastStarted {
        unit: UnitId,
        slot: u8,
        at: SimTime,
        dir: Vec2,
        point: Vec2,
        fire_at: SimTime,
        seq: u32,
    },
    /// In a world in prediction mode the id is 0.
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
    AreaSpawned(Area),
    AreaDetonated {
        id: u32,
        at: SimTime,
    },
    AttackLaunched(Bolt),
    /// `hit` is false when the target died or vanished first.
    AttackLanded {
        id: u32,
        target: UnitId,
        at: SimTime,
        hit: bool,
    },
    /// Post-mitigation damage: `amount` reached health, `absorbed` was taken by shields.
    Damage {
        source: UnitId,
        target: UnitId,
        kind: DamageKind,
        amount: f32,
        absorbed: f32,
        at: SimTime,
    },
    Died {
        unit: UnitId,
        killer: UnitId,
        at: SimTime,
        respawn_at: SimTime,
    },
    Respawned {
        unit: UnitId,
        pos: Vec2,
        at: SimTime,
    },
    Blinked {
        unit: UnitId,
        from: Vec2,
        to: Vec2,
        at: SimTime,
    },
    Dashed {
        unit: UnitId,
        from: Vec2,
        to: Vec2,
        at: SimTime,
        end_at: SimTime,
    },
    Shielded {
        unit: UnitId,
        amount: f32,
        at: SimTime,
        until: SimTime,
    },
}

pub const CHAMPION_MOVE_SPEED: f32 = 325.0;
pub const CHAMPION_COLLISION_RADIUS: f32 = 35.0;
pub const CHAMPION_GAMEPLAY_RADIUS: f32 = 65.0;
pub const MINION_MOVE_SPEED: f32 = 325.0;
pub const TURRET_COLLISION_RADIUS: f32 = 60.0;
pub const TURRET_GAMEPLAY_RADIUS: f32 = 80.0;
/// Sandbox respawn timers (01 §12 scales these with level later).
pub const CHAMPION_RESPAWN: SimDuration = SimDuration::from_millis(6000);
pub const MINION_RESPAWN: SimDuration = SimDuration::from_millis(12_000);

impl MinionKind {
    /// (collision radius, gameplay radius), 01 §4 *(start)* values.
    pub fn radii(self) -> (f32, f32) {
        match self {
            MinionKind::Melee | MinionKind::Caster => (25.0, 48.0),
            MinionKind::Siege => (35.0, 65.0),
        }
    }

    pub fn stats(self) -> Stats {
        let (max_health, armor) = match self {
            MinionKind::Melee => (480.0, 0.0),
            MinionKind::Caster => (300.0, 0.0),
            MinionKind::Siege => (900.0, 15.0),
        };
        Stats { max_health, armor, move_speed: MINION_MOVE_SPEED, ..Stats::NONE }
    }
}

/// A unit as seen by attackers and acquisition during phase 2: its start-of-tick state.
#[derive(Clone, Copy, Debug)]
struct Target {
    id: UnitId,
    team: Team,
    pos: Vec2,
    radius: f32,
}

/// Something that fired during phase 2, created in phase 3.
enum Fired {
    Missile(Missile),
    Area(Area),
    Bolt(Bolt),
}

#[derive(Clone, Debug)]
pub struct World {
    tick: Tick,
    units: Vec<Unit>,
    next_unit: u32,
    rng: Pcg32,
    missiles: Vec<Missile>,
    areas: Vec<Area>,
    bolts: Vec<Bolt>,
    map: Arc<Map>,
    /// Id counter shared by missiles, areas and bolts.
    next_missile: u32,
    prediction: bool,
    events: Vec<SimEvent>,
    /// Per team (blue, red): enemy units it can't see, which its units can't target with
    /// attack orders or attack-move. Set by the server from its vision every tick; client
    /// prediction only knows visible units anyway.
    hidden: [Vec<UnitId>; 2],
}

impl World {
    pub fn new(seed: u64) -> Self {
        Self {
            tick: Tick(0),
            units: Vec::new(),
            next_unit: 1,
            rng: Pcg32::new(seed, 0x4d46_5452),
            missiles: Vec::new(),
            areas: Vec::new(),
            bolts: Vec::new(),
            next_missile: 1,
            prediction: false,
            events: Vec::new(),
            hidden: [Vec::new(), Vec::new()],
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

    pub fn areas(&self) -> &[Area] {
        &self.areas
    }

    pub fn bolts(&self) -> &[Bolt] {
        &self.bolts
    }

    /// Client prediction mode: no effects on others are simulated (03a §7), but own casts
    /// still emit `MissileSpawned` / `AreaSpawned` / `AttackLaunched` (id 0) for drawing.
    pub fn set_prediction_mode(&mut self, prediction: bool) {
        self.prediction = prediction;
        if prediction {
            self.missiles.clear();
            self.areas.clear();
            self.bolts.clear();
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

    pub fn spawn_champion(&mut self, owner: PlayerId, team: Team, champion: ChampionId, pos: Vec2) -> UnitId {
        let id = self.next_id();
        let stats = champion.def().stats;
        self.units.push(Unit::champion(id, owner, team, champion, pos, pos, stats));
        id
    }

    pub fn spawn_minion(&mut self, kind: MinionKind, team: Team, pos: Vec2, brain: Option<Brain>) -> UnitId {
        let id = self.next_id();
        let (collision_radius, gameplay_radius) = kind.radii();
        let stats = kind.stats();
        let mut state = UnitState::new(pos, stats.move_speed);
        state.health = stats.max_health;
        self.units.push(Unit {
            id,
            kind: UnitKind::Minion,
            owner: None,
            team,
            state,
            collision_radius,
            gameplay_radius,
            brain,
            champion: None,
            stats,
            home: pos,
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
            champion: None,
            stats: Stats::NONE,
            home: pos,
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

    pub fn map(&self) -> &Arc<Map> {
        &self.map
    }

    /// Use a map (server: at match start; client: from the welcome). Walls and paths then
    /// apply to every unit.
    pub fn set_map(&mut self, map: Arc<Map>) {
        self.map = map;
    }

    /// Units `team` can't see (and so can't attack) from the next tick on.
    pub fn set_hidden(&mut self, team: Team, mut ids: Vec<UnitId>) {
        ids.sort();
        self.hidden[team as usize] = ids;
    }

    /// Rewind or fast-forward the tick counter (prediction reconciliation only).
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
        let prediction = self.prediction;
        let World { units, rng, missiles, areas, bolts, next_missile, events, map, hidden, .. } = self;
        let map: &Map = map;

        // Phase 0: respawns.
        for u in units.iter_mut() {
            if u.state.respawn_at.is_some_and(|r| r <= s0) {
                u.state = UnitState::new(u.home, u.stats.move_speed);
                u.state.health = u.stats.max_health;
                if let Some(Brain::Patrol { a, b, .. }) = u.brain {
                    u.brain = Some(Brain::Patrol { a, b, toward_b: true });
                }
                events.push(SimEvent::Respawned { unit: u.id, pos: u.home, at: s0 });
            }
        }

        // Phase 1: AI.
        let champions: Vec<(Team, Vec2, Option<Vec2>, f32)> = units
            .iter()
            .filter(|u| u.kind == UnitKind::Champion && u.state.alive())
            .map(|u| (u.team, u.state.pos, u.state.heading(), u.state.move_speed))
            .collect();
        for unit in units.iter_mut() {
            match unit.brain {
                Some(Brain::Patrol { a, b, toward_b }) if unit.state.order == Order::Idle && unit.state.alive() => {
                    unit.state.set_order(Order::MoveTo(if toward_b { b } else { a }), map);
                    unit.brain = Some(Brain::Patrol { a, b, toward_b: !toward_b });
                }
                Some(Brain::Turret { range }) if unit.state.can_cast(s0, 0) => {
                    let me = unit.state.pos;
                    let mut best: Option<(f32, Vec2, Option<Vec2>, f32)> = None;
                    for &(team, pos, heading, speed) in &champions {
                        let d = pos.distance(me);
                        if team != unit.team && d <= range as f32 && best.is_none_or(|(bd, ..)| d < bd) {
                            best = Some((d, pos, heading, speed));
                        }
                    }
                    let Some((d, pos, heading, speed)) = best else { continue };
                    let Effect::Line(spec) = TURRET_SHOT.effect else { continue };
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
                    try_cast(unit, 0, aim, s0, 0, map, events);
                }
                _ => {}
            }
        }

        // Phase 2: per-unit timelines against start-of-tick positions (order-independent).
        let starts: Vec<(UnitId, UnitKind, Team, Obstacle)> = units
            .iter()
            .filter(|u| u.collision_radius > 0.0 && u.state.alive() && u.state.dash.is_none())
            .map(|u| (u.id, u.kind, u.team, Obstacle { pos: u.state.pos, radius: u.collision_radius }))
            .collect();
        let roster: Vec<Target> = units
            .iter()
            .filter(|u| u.targetable())
            .map(|u| Target { id: u.id, team: u.team, pos: u.state.pos, radius: u.gameplay_radius })
            .collect();
        let start_pos: Vec<(UnitId, Vec2)> = units.iter().map(|u| (u.id, u.state.pos)).collect();
        let mut fired: Vec<Fired> = Vec::new();
        for unit in units.iter_mut() {
            let mine: Vec<&Command> = match unit.owner {
                Some(owner) => cmds.iter().copied().filter(|c| c.player == owner).collect(),
                None => Vec::new(),
            };
            if !unit.state.alive() {
                continue; // commands while dead are dropped
            }
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
            let (mut desired, mut achieved) = (0.0f32, 0.0f32);
            let mut t = s0;
            let mut next_cmd = 0;
            loop {
                // Instants at `t`: windups and dashes ending, then commands.
                if t > s0 {
                    fire_due(unit, t, &roster, &mut fired, map);
                }
                while let Some(c) = mine.get(next_cmd)
                    && SimTime::at(k, c.sub) == t
                {
                    apply_command(unit, c, t, map, events);
                    next_cmd += 1;
                }
                if t >= s1 {
                    break;
                }
                let halt = think(unit, &roster, &hidden[unit.team as usize], t, map);
                let cmd_at = mine.get(next_cmd).map(|c| SimTime::at(k, c.sub));
                let st = &unit.state;
                let waits = [
                    cmd_at,
                    st.cast.map(|c| c.fire_at),
                    st.attack.map(|a| a.fire_at),
                    st.dash.map(|d| d.end_at),
                    Some(st.stunned_until),
                    Some(st.rooted_until),
                    halt.map(|_| st.attack_ready_at),
                ];
                let mut next = waits.into_iter().flatten().filter(|w| *w > t && *w <= s1).min().unwrap_or(s1);
                let span = next.0 - t.0;
                let dt = span as f32 / SUBTICKS_PER_SECOND as f32;
                if unit.state.dash.is_some() {
                    unit.state.dash_advance(dt, radius, map);
                } else if unit.state.can_move(t) {
                    let (d, a, used) = unit.state.advance(dt, radius, &obstacles, map, halt);
                    desired += d;
                    achieved += a;
                    if used < 1.0 {
                        // Reached attack range part-way: look again from there.
                        let part = ((used * span as f32).ceil() as u64).clamp(1, span);
                        next = SimTime(t.0 + part);
                    }
                }
                t = next;
            }
            update_stuck(&mut unit.state, desired, achieved, radius, &obstacles, map);
        }

        // Phase 3: effects.
        for f in fired {
            let id = if prediction {
                0
            } else {
                *next_missile += 1;
                *next_missile - 1
            };
            match f {
                Fired::Missile(m) => {
                    let m = Missile { id, ..m };
                    events.push(SimEvent::MissileSpawned(m));
                    if !prediction {
                        missiles.push(m);
                    }
                }
                Fired::Area(a) => {
                    let a = Area { id, ..a };
                    events.push(SimEvent::AreaSpawned(a));
                    if !prediction {
                        areas.push(a);
                    }
                }
                Fired::Bolt(b) => {
                    let b = Bolt { id, ..b };
                    events.push(SimEvent::AttackLaunched(b));
                    if !prediction {
                        bolts.push(b);
                    }
                }
            }
        }
        if !prediction {
            resolve_effects(units, missiles, areas, bolts, &start_pos, s0, s1, events);
        }

        // Phase 4: regeneration and shield expiry.
        for u in units.iter_mut().filter(|u| u.state.alive()) {
            let max = u.stats.max_health;
            if u.state.health < max && u.stats.health_regen > 0.0 {
                u.state.health = (u.state.health + u.stats.health_regen * TICK_DT).min(max);
            }
            if u.state.shield > 0.0 && u.state.shield_until <= s1 {
                u.state.shield = 0.0;
            }
        }
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
        for ids in &self.hidden {
            h.write_u32(ids.len() as u32);
            for id in ids {
                h.write_u32(id.0);
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
            h.write_f32(m.power);
        }
        for a in &self.areas {
            h.write_u32(a.id);
            h.write_f32(a.center.x);
            h.write_f32(a.center.y);
            h.write_u64(a.detonate_at.0);
            h.write_f32(a.power);
        }
        for b in &self.bolts {
            h.write_u32(b.id);
            h.write_u32(b.target.0);
            h.write_f32(b.pos.x);
            h.write_f32(b.pos.y);
            h.write_f32(b.power);
        }
        h.finish()
    }
}

impl Unit {
    /// A champion at full health (also used by client prediction for its own unit).
    pub fn champion(
        id: UnitId,
        owner: PlayerId,
        team: Team,
        champion: ChampionId,
        pos: Vec2,
        home: Vec2,
        stats: Stats,
    ) -> Unit {
        let mut state = UnitState::new(pos, stats.move_speed);
        state.health = stats.max_health;
        Unit {
            id,
            kind: UnitKind::Champion,
            owner: Some(owner),
            team,
            state,
            collision_radius: CHAMPION_COLLISION_RADIUS,
            gameplay_radius: CHAMPION_GAMEPLAY_RADIUS,
            brain: None,
            champion: Some(champion),
            stats,
            home,
        }
    }
}

/// Attack logic at instant `t` (Attack and AttackMove orders): start a windup when a valid
/// target is in range and the attack timer allows, chase otherwise. Returns the range circle
/// the unit's movement must halt at, if it has a target.
fn think(unit: &mut Unit, roster: &[Target], hidden: &[UnitId], t: SimTime, map: &Map) -> Option<(Vec2, f32)> {
    let atk = unit.attack_spec()?;
    let (me, my_team) = (unit.id, unit.team);
    let st = &mut unit.state;
    if let Some(w) = st.attack {
        // Winding up: rooted; keep facing the committed target.
        return roster.iter().find(|r| r.id == w.target).map(|r| (r.pos, atk.range + r.radius));
    }
    let enemy = |r: &&Target| r.team != my_team && r.id != me && hidden.binary_search(&r.id).is_err();
    let target = match st.order {
        Order::Attack(id) => match roster.iter().filter(enemy).find(|r| r.id == id) {
            Some(r) => *r,
            None => {
                st.set_order(Order::Idle, map); // dead or gone
                return None;
            }
        },
        Order::AttackMove(_) => {
            let mut best: Option<(f32, Target)> = None;
            for r in roster.iter().filter(enemy) {
                let gap = r.pos.distance(st.pos) - (atk.range + r.radius);
                if gap <= RANGE_SLACK && best.is_none_or(|(g, b)| (gap, r.id) < (g, b.id)) {
                    best = Some((gap, *r));
                }
            }
            best?.1
        }
        Order::Idle | Order::MoveTo(_) => return None,
    };
    let reach = atk.range + target.radius;
    if target.pos.distance(st.pos) <= reach + RANGE_SLACK {
        if st.cast.is_none() && st.dash.is_none() && st.stunned_until <= t && st.attack_ready_at <= t {
            st.attack = Some(AttackWindup { target: target.id, fire_at: t.plus(atk.windup()) });
            st.attack_ready_at = t.plus(atk.period());
        }
    } else if matches!(st.order, Order::Attack(_))
        && st.detour.is_none()
        && st.path.goal().is_none_or(|g| g.distance(target.pos) > REPATH_DISTANCE)
    {
        st.path = Path::from_points(&map.find_path(st.pos, target.pos));
    }
    Some((target.pos, reach))
}

/// Windups, casts and dashes whose instant is `t`.
fn fire_due(unit: &mut Unit, t: SimTime, roster: &[Target], fired: &mut Vec<Fired>, map: &Map) {
    let (id, team, stats) = (unit.id, unit.team, unit.stats);
    if let Some(c) = unit.state.cast
        && c.fire_at == t
    {
        unit.state.cast = None;
        match unit.ability(c.slot).map(|a| a.effect) {
            Some(Effect::Line(spec)) => fired.push(Fired::Missile(Missile {
                id: 0,
                owner: id,
                team,
                origin: unit.state.pos,
                dir: c.dir,
                spec,
                spawn_at: t,
                cast_seq: c.seq,
                power: spec.damage.raw(stats.attack_damage, stats.ability_power),
            })),
            Some(Effect::Area(a)) => fired.push(Fired::Area(Area {
                id: 0,
                owner: id,
                team,
                center: c.point,
                radius: a.radius,
                spawn_at: t,
                detonate_at: t.plus(a.delay),
                kind: a.damage.kind,
                power: a.damage.raw(stats.attack_damage, stats.ability_power),
                cast_seq: c.seq,
            })),
            _ => {}
        }
    }
    if let Some(w) = unit.state.attack
        && w.fire_at == t
    {
        unit.state.attack = None;
        if let Some(atk) = unit.attack_spec()
            && roster.iter().any(|r| r.id == w.target)
        {
            fired.push(Fired::Bolt(Bolt {
                id: 0,
                owner: id,
                team,
                target: w.target,
                origin: unit.state.pos,
                pos: unit.state.pos,
                speed: atk.bolt_speed,
                launched_at: t,
                power: stats.attack_damage,
            }));
        }
    }
    if let Some(d) = unit.state.dash
        && d.end_at == t
    {
        unit.state.dash = None;
        unit.state.detour = None;
        unit.state.route(map);
    }
}

fn apply_command(unit: &mut Unit, c: &Command, t: SimTime, map: &Map, events: &mut Vec<SimEvent>) {
    let st = &mut unit.state;
    match c.kind {
        CommandKind::MoveTo(q) => {
            st.cancel_attack(t);
            st.set_order(Order::MoveTo(q), map);
        }
        CommandKind::AttackMove(q) => {
            st.cancel_attack(t);
            st.set_order(Order::AttackMove(q), map);
        }
        CommandKind::Attack(target) => {
            if target == unit.id {
                return;
            }
            if st.attack.is_some_and(|w| w.target != target) {
                st.cancel_attack(t);
            }
            if st.order != Order::Attack(target) {
                st.set_order(Order::Attack(target), map);
            }
        }
        CommandKind::Stop => {
            st.cancel_attack(t);
            st.set_order(Order::Idle, map);
        }
        CommandKind::Cast { slot, target } => try_cast(unit, slot, target.to_vec2(), t, c.seq, map, events),
    }
}

/// Validate and start a cast at `t` (03 §5: the sim validates everything). Skillshots and
/// areas wind up (rooted); dashes, blinks and shields take effect at once.
fn try_cast(unit: &mut Unit, slot: u8, target: Vec2, t: SimTime, seq: u32, map: &Map, events: &mut Vec<SimEvent>) {
    let Some(ability) = unit.ability(slot) else { return };
    let (id, radius) = (unit.id, unit.collision_radius);
    let st = &mut unit.state;
    if !st.can_cast(t, slot) {
        return;
    }
    let to = target - st.pos;
    let dir = to.normalize_or_zero();
    let len = to.length();
    match ability.effect {
        Effect::Line(spec) => {
            if dir == Vec2::ZERO {
                return;
            }
            st.cancel_attack(t);
            let fire_at = t.plus(spec.windup);
            st.cast = Some(Cast { slot, dir, point: target, fire_at, seq });
            events.push(SimEvent::CastStarted { unit: id, slot, at: t, dir, point: target, fire_at, seq });
        }
        Effect::Area(a) => {
            st.cancel_attack(t);
            let point = if len > a.range { st.pos + dir * a.range } else { target };
            let fire_at = t.plus(a.windup);
            st.cast = Some(Cast { slot, dir, point, fire_at, seq });
            events.push(SimEvent::CastStarted { unit: id, slot, at: t, dir, point, fire_at, seq });
        }
        Effect::Dash(d) => {
            if dir == Vec2::ZERO || st.rooted_until > t {
                return;
            }
            st.cancel_attack(t);
            let dist = len.min(d.range);
            let to = st.pos + dir * dist;
            let end_at = SimTime(t.0 + ((dist / d.speed * SUBTICKS_PER_SECOND as f32).ceil() as u64).max(1));
            st.dash = Some(DashMove { dir, to, speed: d.speed, end_at });
            st.detour = None;
            events.push(SimEvent::Dashed { unit: id, from: st.pos, to, at: t, end_at });
        }
        Effect::Blink(b) => {
            if dir == Vec2::ZERO {
                return;
            }
            st.cancel_attack(t);
            let from = st.pos;
            st.pos = blink_landing(map, from, dir, len.min(b.range), radius);
            st.detour = None;
            st.stuck = 0;
            st.route(map);
            events.push(SimEvent::Blinked { unit: id, from, to: st.pos, at: t });
        }
        Effect::Shield(s) => {
            st.shield = s.amount;
            st.shield_until = t.plus(s.duration);
            events.push(SimEvent::Shielded { unit: id, amount: s.amount, at: t, until: st.shield_until });
        }
    }
    st.cooldowns[slot as usize] = t.plus(ability.cooldown);
}

/// Where a blink toward `dir` lands: the farthest walkable, in-bounds point on the line, so a
/// blink can cross a thin wall but never ends inside one.
fn blink_landing(map: &Map, from: Vec2, dir: Vec2, dist: f32, radius: f32) -> Vec2 {
    let mut d = dist;
    loop {
        let p = from + dir * d;
        if map.walkable(p, radius) && map.in_bounds(p, radius) {
            return p;
        }
        if d <= 0.0 {
            return from;
        }
        d = (d - 10.0).max(0.0);
    }
}

/// Phase 3 on the server: missiles, areas and bolts against the units' motion this tick.
#[allow(clippy::too_many_arguments)]
fn resolve_effects(
    units: &mut [Unit],
    missiles: &mut Vec<Missile>,
    areas: &mut Vec<Area>,
    bolts: &mut Vec<Bolt>,
    start_pos: &[(UnitId, Vec2)],
    s0: SimTime,
    s1: SimTime,
    events: &mut Vec<SimEvent>,
) {
    let motion: Vec<(UnitId, Team, f32, Vec2, Vec2)> = units
        .iter()
        .filter(|u| u.targetable())
        .map(|u| {
            let start = start_pos.iter().find(|(id, _)| *id == u.id).map_or(u.state.pos, |(_, p)| *p);
            (u.id, u.team, u.gameplay_radius, start, u.state.pos)
        })
        .collect();
    missiles.retain(|m| {
        let a = m.spawn_at.max(s0);
        let b = m.end_at().min(s1);
        let mut hits: Vec<(SimTime, UnitId)> = motion
            .iter()
            .filter(|&&(id, team, ..)| team != m.team && id != m.owner)
            .filter_map(|&(id, _, r, q0, q1)| m.first_hit(a, b, s0, q0, q1, r).map(|at| (at, id)))
            .collect();
        hits.sort();
        // The earliest contact with a unit that is still alive (an earlier effect this tick may
        // have killed the first candidate).
        let hit = hits.into_iter().find(|(_, id)| units.iter().any(|u| u.id == *id && u.targetable()));
        if let Some((at, target)) = hit {
            events.push(SimEvent::MissileHit { id: m.id, target, at });
            if let Some(u) = units.iter_mut().find(|u| u.id == target) {
                match m.spec.cc {
                    Cc::None => {}
                    Cc::Stun(d) => {
                        u.state.stunned_until = u.state.stunned_until.max(at.plus(d));
                        u.state.cast = None; // hard CC interrupts casts and attacks
                        u.state.attack = None;
                    }
                    Cc::Root(d) => u.state.rooted_until = u.state.rooted_until.max(at.plus(d)),
                }
                deal_damage(u, m.owner, m.power, m.spec.damage.kind, at, events);
            }
            return false;
        }
        if m.end_at() <= s1 {
            events.push(SimEvent::MissileExpired { id: m.id, at: m.end_at() });
            return false;
        }
        true
    });
    areas.retain(|a| {
        if a.detonate_at > s1 {
            return true;
        }
        let frac = (a.detonate_at.0.saturating_sub(s0.0)) as f32 / SUBTICKS as f32;
        events.push(SimEvent::AreaDetonated { id: a.id, at: a.detonate_at });
        for &(id, team, r, q0, q1) in &motion {
            let reach = a.radius + r;
            if team != a.team && (q0.lerp(q1, frac) - a.center).length_sq() <= reach * reach {
                if let Some(u) = units.iter_mut().find(|u| u.id == id) {
                    deal_damage(u, a.owner, a.power, a.kind, a.detonate_at, events);
                }
            }
        }
        false
    });
    bolts.retain_mut(|b| {
        let Some(target) = units.iter_mut().find(|u| u.id == b.target && u.targetable()) else {
            events.push(SimEvent::AttackLanded { id: b.id, target: b.target, at: s1, hit: false });
            return false;
        };
        let from = b.launched_at.max(s0);
        let dt = (s1.0 - from.0) as f32 / SUBTICKS_PER_SECOND as f32;
        let to = target.state.pos - b.pos;
        let gap = (to.length() - target.gameplay_radius).max(0.0);
        let step = b.speed * dt;
        if gap <= step {
            let at = SimTime(from.0 + (gap / b.speed * SUBTICKS_PER_SECOND as f32) as u64).min(s1);
            events.push(SimEvent::AttackLanded { id: b.id, target: b.target, at, hit: true });
            deal_damage(target, b.owner, b.power, DamageKind::Physical, at, events);
            return false;
        }
        b.pos += to.normalize_or_zero() * step;
        true
    });
}

/// The damage pipeline (02 §5), M1 subset: resistance mitigation, shields, health, death.
fn deal_damage(u: &mut Unit, source: UnitId, raw: f32, kind: DamageKind, at: SimTime, events: &mut Vec<SimEvent>) {
    if raw <= 0.0 || !u.targetable() {
        return;
    }
    let resist = match kind {
        DamageKind::Physical => u.stats.armor,
        DamageKind::Magic => u.stats.magic_resist,
        DamageKind::True => 0.0,
    };
    let mut amount = if kind == DamageKind::True { raw } else { raw * resist_multiplier(resist) };
    let st = &mut u.state;
    let mut absorbed = 0.0;
    if st.shield > 0.0 && st.shield_until > at {
        absorbed = st.shield.min(amount);
        st.shield -= absorbed;
        amount -= absorbed;
    }
    st.health -= amount;
    events.push(SimEvent::Damage { source, target: u.id, kind, amount, absorbed, at });
    if st.health <= 0.0 {
        let respawn_at = at.plus(if u.kind == UnitKind::Champion { CHAMPION_RESPAWN } else { MINION_RESPAWN });
        *st = UnitState { respawn_at: Some(respawn_at), ..UnitState::new(st.pos, st.move_speed) };
        events.push(SimEvent::Died { unit: u.id, killer: source, at, respawn_at });
    }
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
/// within reach, stop where we are, like the reference game does. A chase never gives up
/// this way: its goal is the (occupied) target itself.
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
    let goal = match state.order {
        Order::MoveTo(q) | Order::AttackMove(q) => q.to_vec2(),
        Order::Attack(_) => match state.path.goal() {
            Some(g) => g,
            None => return,
        },
        Order::Idle => return,
    };
    let chasing = matches!(state.order, Order::Attack(_));
    let occupied = obstacles.iter().any(|o| (goal - o.pos).length_sq() < (radius + o.radius) * (radius + o.radius));
    if !chasing && (occupied || (goal - state.pos).length() < GIVE_UP_DISTANCE) {
        state.set_order(Order::Idle, map);
        return;
    }
    let reach = 2.0 * (radius + 35.0);
    state.detour = choose_detour(state.pos, goal, radius, reach, obstacles, map.edges());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Cc;
    use crate::champion::EMBER;
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
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
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
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, SUBTICKS / 2, (2000.0, 1000.0))]);
        let x = w.unit(id).unwrap().state.pos.x;
        assert!((x - (1000.0 + 325.0 / 60.0)).abs() < 1e-3, "{x}");
    }

    #[test]
    fn commands_for_other_ticks_are_ignored() {
        let mut w = World::new(1);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 5, 0, (2000.0, 1000.0))]);
        assert_eq!(w.unit(id).unwrap().state.order, Order::Idle);
    }

    #[test]
    fn champion_walks_around_a_minion_clump() {
        let mut w = World::new(1);
        clump(&mut w, Vec2::new(1500.0, 1000.0), 2);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
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
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
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
            let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
            let other = w.spawn_champion(PlayerId(1), other_team, ChampionId::Ember, Vec2::new(1200.0, 1000.0)); // in the way
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
        let a = full.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(500.0, 500.0));
        full.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(7000.0, 7000.0));
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
        let me = full.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
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
        cast_slot(player, seq, tick, sub, 0, target)
    }

    fn cast_slot(player: u8, seq: u32, tick: u32, sub: u8, slot: u8, target: (f32, f32)) -> Command {
        Command {
            player: PlayerId(player),
            seq,
            tick: Tick(tick),
            sub: SubTick::new(sub),
            kind: CommandKind::Cast { slot, target: QPoint::from_vec2(Vec2::new(target.0, target.1)) },
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
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
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
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1800.0, 1000.0));
        w.step(&[cast_slot(0, 1, 1, 0, 3, (1800.0, 1000.0))]); // Binding Sigil
        let ev = run_until_quiet(&mut w, 40);
        let hit = ev.iter().find_map(|e| match e {
            SimEvent::MissileHit { target, at, .. } => Some((*target, *at)),
            _ => None,
        });
        let (target, at) = hit.expect("should hit");
        assert_eq!(target, enemy);
        // Contact when the gap is 60 + 65 = 125 u: 675 u at 1300 u/s after the 0.3 s windup.
        let expected = 576.0 + 675.0 / 1300.0 * 1920.0;
        assert!((at.0 as f32 - expected).abs() <= 1.0, "{} vs {expected}", at.0);
        let Effect::Line(sigil) = EMBER.abilities[3].effect else { panic!() };
        let Cc::Stun(stun) = sigil.cc else { panic!() };
        let e = w.unit(enemy).unwrap().state;
        assert_eq!(e.stunned_until, at.plus(stun));
        // 100 + 0.5 × 80 AP = 140 magic damage into 30 MR.
        let dealt = ev.iter().find_map(|e| match e {
            SimEvent::Damage { target, amount, .. } if *target == enemy => Some(*amount),
            _ => None,
        });
        assert!((dealt.unwrap() - 140.0 * 100.0 / 130.0).abs() < 1e-3, "{dealt:?}");
        assert!((e.health - (600.0 - 140.0 * 100.0 / 130.0)).abs() < 1.5, "{} (plus regen)", e.health);
    }

    #[test]
    fn walking_out_in_time_dodges_and_too_late_is_hit() {
        for (react_tick, expect_hit) in [(6u32, false), (20u32, true)] {
            let mut w = World::new(1);
            w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
            w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1800.0, 1000.0));
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
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        let minion = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1400.0, 1010.0), None);
        w.spawn_minion(MinionKind::Caster, Team::Blue, Vec2::new(1200.0, 1000.0), None); // allied: ignored
        w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1800.0, 1000.0));
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
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
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
        let target = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1600.0, 1000.0));
        w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1300.0, 1000.0)); // ally of the turret
        let ev = run_until_quiet(&mut w, 40);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::MissileHit { target: t, .. } if *t == target)));
    }

    /// Client-style prediction (missiles disabled) must still reproduce the caster exactly.
    #[test]
    fn prediction_without_missiles_matches_the_caster() {
        let mut full = World::new(9);
        let me = full.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        full.spawn_turret(Team::Red, Vec2::new(5000.0, 5000.0), 1100); // out of range
        let mut predicted = World::from_units(full.tick(), vec![full.unit(me).unwrap().clone()]);
        predicted.set_prediction_mode(true);
        let mut rng = Pcg32::new(3, 3);
        for k in 1..=900u32 {
            let mut c = Vec::new();
            if rng.next_u32() % 7 == 0 {
                let t = (rng.range_f32(500.0, 2500.0), rng.range_f32(500.0, 2500.0));
                let sub = (rng.next_u32() % 64) as u8;
                c.push(if rng.next_u32() % 3 == 0 {
                    cast_slot(0, k, k, sub, (rng.next_u32() % 6) as u8, t)
                } else {
                    cmd(0, k, k, sub, t)
                });
            }
            full.step(&c);
            predicted.step(&c);
            assert!(predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state), "tick {k}");
        }
    }

    fn attack(player: u8, seq: u32, tick: u32, target: UnitId) -> Command {
        Command {
            player: PlayerId(player),
            seq,
            tick: Tick(tick),
            sub: SubTick::START,
            kind: CommandKind::Attack(target),
        }
    }

    fn damage_to(ev: &[SimEvent], who: UnitId) -> Vec<(SimTime, f32)> {
        ev.iter()
            .filter_map(|e| match e {
                SimEvent::Damage { target, amount, absorbed, at, .. } if *target == who => {
                    Some((*at, amount + absorbed))
                }
                _ => None,
            })
            .collect()
    }

    /// 02 §7: chase into range, wind up (rooted), launch a homing bolt, repeat on the timer.
    #[test]
    fn basic_attacks_chase_wind_up_and_hit_on_the_timer() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1900.0, 1000.0));
        w.step(&[attack(0, 1, 1, enemy)]);
        let mut ev = w.take_events();
        ev.extend(run_until_quiet(&mut w, 150)); // 5 s
        let reach = 575.0 + 65.0;
        let p = w.unit(me).unwrap().state.pos;
        assert!((p.distance(Vec2::new(1900.0, 1000.0)) - reach).abs() < 1.0, "stops at range: {p:?}");
        let hits = damage_to(&ev, enemy);
        // Walk 260 u (0.8 s), wind up 0.225 s, fly 575 u at 2200 u/s: first hit at ~1.29 s,
        // then one every 1.25 s.
        assert_eq!(hits.len(), 3, "{hits:?}");
        assert!((hits[0].0.0 as f32 - (1536.0 + 432.0 + 575.0 / 2200.0 * 1920.0)).abs() < 2.0, "{hits:?}");
        let per_hit = 66.0 * 100.0 / 122.0;
        assert!(hits.iter().all(|(_, a)| (a - per_hit).abs() < 1e-3), "{hits:?}");
        let gaps: Vec<u64> = hits.windows(2).map(|h| h[1].0.0 - h[0].0.0).collect();
        assert!(gaps.iter().all(|g| *g == 2400), "attack period 1.25 s: {gaps:?}");
    }

    /// Orb-walk cancel: a move order during the windup cancels the attack, with no damage.
    #[test]
    fn moving_during_the_windup_cancels_the_attack() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1500.0, 1000.0));
        w.step(&[attack(0, 1, 1, enemy)]);
        assert!(w.unit(UnitId(1)).unwrap().state.attack.is_some(), "in range: winds up at once");
        w.step(&[cmd(0, 2, 2, 0, (900.0, 1000.0))]); // windup is 0.225 s: still winding up
        let ev = run_until_quiet(&mut w, 60);
        assert!(damage_to(&ev, enemy).is_empty());
        assert!(!ev.iter().any(|e| matches!(e, SimEvent::AttackLaunched(_))));
    }

    #[test]
    fn attack_move_attacks_the_nearest_enemy_in_range() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let near = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1700.0, 1100.0), None);
        let far = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1900.0, 900.0), None);
        w.spawn_minion(MinionKind::Caster, Team::Blue, Vec2::new(1400.0, 1000.0), None); // ally: ignored
        let q = QPoint::from_vec2(Vec2::new(3000.0, 1000.0));
        w.step(&[Command {
            player: PlayerId(0),
            seq: 1,
            tick: Tick(1),
            sub: SubTick::START,
            kind: CommandKind::AttackMove(q),
        }]);
        let ev = run_until_quiet(&mut w, 90);
        assert!(!damage_to(&ev, near).is_empty());
        assert!(damage_to(&ev, far).is_empty(), "the nearer one first");
        assert!(w.unit(UnitId(1)).unwrap().state.pos.x < 1500.0, "stopped to attack");
    }

    #[test]
    fn delayed_area_hits_who_stays_and_misses_who_walks_out() {
        for (walk, expect_hit) in [(false, true), (true, false)] {
            let mut w = World::new(1);
            w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
            let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1700.0, 1000.0));
            w.step(&[cast_slot(0, 1, 1, 0, 1, (1700.0, 1000.0))]); // Cinder Bloom on the target
            let mut ev = w.take_events();
            for k in 2..=60 {
                // React after 0.4 s and walk straight out (needs (160 + 65) / 325 ≈ 0.69 s).
                let c = if walk && k == 13 { vec![cmd(1, 2, k, 0, (1700.0, 1400.0))] } else { vec![] };
                w.step(&c);
                ev.extend(w.take_events());
            }
            let det = ev.iter().find_map(|e| match e {
                SimEvent::AreaDetonated { at, .. } => Some(*at),
                _ => None,
            });
            assert_eq!(det, Some(SimTime(480 + 1632)), "0.25 s windup + 0.85 s delay");
            assert_eq!(!damage_to(&ev, enemy).is_empty(), expect_hit, "walk={walk}");
        }
    }

    #[test]
    fn dash_ignores_units_and_blink_crosses_a_thin_wall() {
        let mut w = World::new(1);
        let v = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        clump(&mut w, Vec2::new(1160.0, 1000.0), 1); // right in the dash path
        w.step(&[cast_slot(0, 1, 1, 0, 2, (2000.0, 1000.0))]); // Tumble: 325 u at 1000 u/s
        for _ in 0..12 {
            w.step(&[]);
        }
        let s = w.unit(v).unwrap().state;
        assert!(s.dash.is_none() && (s.pos.x - 1325.0).abs() < 0.01 && s.pos.y == 1000.0, "{:?}", s.pos);

        let mut w = arena_world(1);
        let e = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(2300.0, 1200.0));
        w.step(&[cast_slot(0, 1, 1, 0, 4, (2900.0, 1200.0))]); // Blink 400 u over the 80 u wall
        let s = w.unit(e).unwrap().state;
        assert_eq!(s.pos, Vec2::new(2700.0, 1200.0));
        assert!(s.cooldowns[4] > SimTime(0));
        // Into the wall: lands short of it, never inside.
        let e2 = w.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Ember, Vec2::new(2100.0, 1000.0));
        w.step(&[cast_slot(1, 2, 2, 0, 4, (2420.0, 1000.0))]);
        let p = w.unit(e2).unwrap().state.pos;
        assert!(w.map().walkable(p, CHAMPION_COLLISION_RADIUS) && p.x < 2380.0 && p.x > 2300.0, "{p:?}");
    }

    #[test]
    fn barrier_absorbs_damage_then_expires() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1800.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (1800.0, 1000.0)), cast_slot(1, 2, 1, 0, 5, (1800.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 30);
        let lance = 118.0 * 100.0 / 130.0;
        let hit = ev.iter().find_map(|e| match e {
            SimEvent::Damage { amount, absorbed, .. } => Some((*amount, *absorbed)),
            _ => None,
        });
        assert_eq!(hit.map(|h| h.0), Some(0.0));
        assert!((hit.unwrap().1 - lance).abs() < 1e-3);
        let s = w.unit(enemy).unwrap().state;
        assert!((s.shield - (150.0 - lance)).abs() < 1e-3 && s.health == 600.0, "{s:?}");
        run_until_quiet(&mut w, 60);
        assert_eq!(w.unit(enemy).unwrap().state.shield, 0.0, "expired after 2.5 s");
    }

    #[test]
    fn root_stops_movement_but_not_attacks() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Vesper, Vec2::new(1600.0, 1000.0));
        let me = UnitId(1);
        w.step(&[cast_slot(0, 1, 1, 0, 3, (1600.0, 1000.0))]); // Snare Net
        let ev = run_until_quiet(&mut w, 30);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::MissileHit { target, .. } if *target == enemy)));
        let rooted = w.unit(enemy).unwrap().state;
        assert!(rooted.rooted_until > SimTime::end_of(w.tick()));
        let k = w.tick().0 + 1;
        w.step(&[cmd(1, 2, k, 0, (1700.0, 2000.0)), attack(1, 3, k, me)]);
        let ev = run_until_quiet(&mut w, 15);
        assert_eq!(w.unit(enemy).unwrap().state.pos, rooted.pos, "rooted: can't walk");
        assert!(!damage_to(&ev, me).is_empty(), "rooted: can still attack");
    }

    #[test]
    fn units_die_and_respawn_at_home_with_full_health() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let minion = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1500.0, 1000.0), None);
        w.step(&[attack(0, 1, 1, minion)]);
        let ev = run_until_quiet(&mut w, 200);
        let died = ev.iter().find_map(|e| match e {
            SimEvent::Died { unit, respawn_at, .. } if *unit == minion => Some(*respawn_at),
            _ => None,
        });
        // 300 HP, 66 damage per attack: the 5th attack kills it.
        assert_eq!(damage_to(&ev, minion).len(), 5);
        let respawn_at = died.expect("minion should die");
        assert_eq!(w.unit(UnitId(1)).unwrap().state.order, Order::Idle, "target gone: stop attacking");
        assert!(!w.unit(minion).unwrap().state.alive());
        let ev = run_until_quiet(&mut w, 400);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::Respawned { unit, .. } if *unit == minion)));
        let m = w.unit(minion).unwrap();
        assert!(m.state.alive() && m.state.health == 300.0 && m.state.pos == m.home);
        assert!(SimTime::end_of(w.tick()) >= respawn_at);
    }

    /// Prediction stays bit-exact for a champion using its whole kit (skillshots, areas, dash,
    /// blink, shield) and attacking units it only knows as proxies.
    #[test]
    fn prediction_matches_through_the_whole_kit_and_attacks() {
        for champ in ChampionId::ALL {
            let mut full = arena_world(12);
            let me = full.spawn_champion(PlayerId(0), Team::Blue, champ, Vec2::new(1000.0, 3300.0));
            for i in 0..4 {
                full.spawn_minion(MinionKind::Siege, Team::Red, Vec2::new(1300.0 + 350.0 * i as f32, 3100.0), None);
            }
            let mut rng = Pcg32::new(4, 4);
            for k in 1..=2400u32 {
                let mut c = Vec::new();
                if rng.next_u32() % 9 == 0 {
                    let t = (rng.range_f32(600.0, 2600.0), rng.range_f32(2800.0, 3700.0));
                    let sub = (rng.next_u32() % 64) as u8;
                    let r = rng.next_u32() % 10;
                    c.push(match r {
                        0..=3 => cmd(0, k, k, sub, t),
                        4 | 5 => {
                            let targets: Vec<UnitId> =
                                full.units().iter().filter(|u| u.team == Team::Red).map(|u| u.id).collect();
                            let mut a = attack(0, k, k, targets[(rng.next_u32() as usize) % targets.len()]);
                            a.sub = SubTick::new(sub);
                            a
                        }
                        6 => Command {
                            kind: CommandKind::AttackMove(QPoint::from_vec2(Vec2::new(t.0, t.1))),
                            ..cmd(0, k, k, sub, t)
                        },
                        _ => cast_slot(0, k, k, sub, (rng.next_u32() % 6) as u8, t),
                    });
                }
                let own = full.unit(me).unwrap().clone();
                let proxies = full.units().iter().filter(|u| u.id != me && u.state.alive()).map(|u| Unit {
                    state: UnitState::new(u.state.pos, 0.0),
                    brain: None,
                    ..u.clone()
                });
                let mut predicted = World::from_units(full.tick(), vec![own]);
                predicted.set_map(MapId::Arena.shared());
                predicted.set_prediction_mode(true);
                predicted.replace_others(me, proxies);
                predicted.step(&c);
                full.step(&c);
                full.take_events();
                assert!(
                    predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state),
                    "{champ:?} tick {k}\n{:?}\n{:?}",
                    predicted.unit(me).unwrap().state,
                    full.unit(me).unwrap().state
                );
            }
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
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(2200.0, 1200.0));
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
        let me = full.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(2000.0, 2000.0));
        full.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(2200.0, 2100.0));
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
            w.spawn_champion(PlayerId(p), team, ChampionId::Ember, Vec2::new(600.0 + p as f32 * 500.0, 3300.0));
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

    const GOLDEN_HASH_ARENA: u64 = 0x3c56_d326_2231_2020;

    /// Cross-platform determinism canary: a scripted match must hash to the same value on
    /// every OS and CPU. If this fails on one platform, the sim used non-deterministic math.
    #[test]
    fn golden_state_hash() {
        let mut w = World::new(0xC0FFEE);
        for p in 0..10u8 {
            let team = if p < 5 { Team::Blue } else { Team::Red };
            let x = 500.0 + p as f32 * 1300.0;
            w.spawn_champion(PlayerId(p), team, ChampionId::ALL[p as usize % 2], Vec2::new(x, 7000.0));
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
        // A duelling pair that only attacks each other: deaths and respawns in the hash.
        let duel_a = w.spawn_champion(PlayerId(10), Team::Blue, ChampionId::Vesper, Vec2::new(7000.0, 12_000.0));
        let duel_b = w.spawn_champion(PlayerId(11), Team::Red, ChampionId::Ember, Vec2::new(7400.0, 12_000.0));
        let mut rng = Pcg32::new(2024, 7);
        let mut seen = [0u32; 6];
        let mut seq = 0;
        for k in 1..=9000u32 {
            let mut cmds = Vec::new();
            if k % 150 == 1 {
                seq += 2;
                cmds.push(attack(10, seq - 1, k, duel_b));
                cmds.push(attack(11, seq, k, duel_a));
            }
            for p in 0..10u8 {
                if rng.next_u32() % 17 == 0 {
                    seq += 1;
                    let sub = (rng.next_u32() % SUBTICKS as u32) as u8;
                    let t = (rng.range_f32(0.0, 14_800.0), rng.range_f32(0.0, 14_800.0));
                    // Mostly moves, some casts of every slot and some attacks (missiles, areas,
                    // bolts, dashes, blinks, shields, damage, deaths and respawns in the hash).
                    let r = rng.next_u32() % 8;
                    cmds.push(match r {
                        0 | 1 => cast_slot(p, seq, k, sub, (rng.next_u32() % 6) as u8, t),
                        2 => attack(p, seq, k, UnitId(1 + rng.next_u32() % 70)),
                        _ => cmd(p, seq, k, sub, t),
                    });
                }
            }
            w.step(&cmds);
            for e in w.take_events() {
                let i = match e {
                    SimEvent::Damage { .. } => 0,
                    SimEvent::Died { .. } => 1,
                    SimEvent::AreaDetonated { .. } => 2,
                    SimEvent::AttackLanded { hit: true, .. } => 3,
                    SimEvent::Dashed { .. } | SimEvent::Blinked { .. } => 4,
                    SimEvent::Respawned { .. } => 5,
                    _ => continue,
                };
                seen[i] += 1;
            }
        }
        assert!(seen.iter().all(|n| *n > 0), "damage, deaths, areas, bolts, dashes, respawns: {seen:?}");
        assert_eq!(w.tick(), Tick(9000));
        assert_eq!(w.state_hash(), GOLDEN_HASH, "hash = {:#018x}", w.state_hash());
    }

    /// Recorded on x86_64-unknown-linux-gnu (debug and release agree). CI checks Linux, macOS
    /// (aarch64) and Windows.
    const GOLDEN_HASH: u64 = 0xa34f_9cfd_b71b_2c23;
}
