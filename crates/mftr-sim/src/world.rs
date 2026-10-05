//! The simulation world: units, commands and the fixed-tick step.
//!
//! Tick phases: (1) AI decisions for server-driven units, (2) movement with unit collision
//! against start-of-tick positions (`collision.rs`), including sub-tick command application.
//! Abilities, projectiles and vision land in later M1 slices on top of this structure.

use crate::collision::{Obstacle, choose_detour, constrained_move};
use crate::hash::StateHasher;
use crate::math::{QPoint, Vec2};
use crate::rng::Pcg32;
use crate::time::{SubTick, TICK_DT, Tick};

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

/// Everything client prediction needs to reproduce a unit's movement bit-exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitState {
    pub pos: Vec2,
    pub order: Order,
    pub move_speed: f32,
    /// Temporary waypoint around blocking units, taken before resuming `order`.
    pub detour: Option<Vec2>,
    /// Consecutive ticks of poor progress while moving.
    pub stuck: u8,
}

impl UnitState {
    pub fn new(pos: Vec2, move_speed: f32) -> Self {
        Self { pos, order: Order::Idle, move_speed, detour: None, stuck: 0 }
    }

    /// Where the unit is currently heading (detour first), if anywhere.
    pub fn heading(&self) -> Option<Vec2> {
        match (self.detour, self.order) {
            (Some(d), _) => Some(d),
            (None, Order::MoveTo(q)) => Some(q.to_vec2()),
            (None, Order::Idle) => None,
        }
    }

    /// Replace the order (a player command): detours and stuck state reset.
    pub fn set_order(&mut self, order: Order) {
        self.order = order;
        self.detour = None;
        self.stuck = 0;
    }

    /// Advance by `dt` seconds toward the current heading at constant speed (instant turns,
    /// R01 §2), blocked by `obstacles`. Returns `(desired, achieved)` distance.
    pub fn advance(&mut self, dt: f32, radius: f32, obstacles: &[Obstacle]) -> (f32, f32) {
        if dt <= 0.0 {
            return (0.0, 0.0);
        }
        let Some(target) = self.heading() else { return (0.0, 0.0) };
        let to = target - self.pos;
        let dist = to.length();
        let step = self.move_speed * dt;
        let (delta, arrives) = if step >= dist { (to, true) } else { (to * (step / dist), false) };
        let new_pos = constrained_move(self.pos, delta, radius, obstacles);
        let achieved = (new_pos - self.pos).length();
        self.pos = new_pos;
        if arrives && new_pos == target {
            if self.detour.is_some() {
                self.detour = None;
            } else {
                self.order = Order::Idle;
            }
            self.stuck = 0;
        }
        (delta.length(), achieved)
    }

    /// Bit-exact equality: what prediction reconciliation compares.
    pub fn bits_eq(&self, other: &Self) -> bool {
        self.pos.to_bits() == other.pos.to_bits()
            && self.order == other.order
            && self.move_speed.to_bits() == other.move_speed.to_bits()
            && self.detour.map(Vec2::to_bits) == other.detour.map(Vec2::to_bits)
            && self.stuck == other.stuck
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
        match self.detour {
            None => h.write_u8(0),
            Some(d) => {
                h.write_u8(1);
                h.write_f32(d.x);
                h.write_f32(d.y);
            }
        }
        h.write_u8(self.stuck);
    }
}

/// Server-side decision making for units without a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Brain {
    /// Walk back and forth between two points (minion-dummy waves, M1 sandbox).
    Patrol { a: QPoint, b: QPoint, toward_b: bool },
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandKind {
    MoveTo(QPoint),
    Stop,
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

pub const CHAMPION_MOVE_SPEED: f32 = 325.0;
pub const CHAMPION_COLLISION_RADIUS: f32 = 35.0;
pub const CHAMPION_GAMEPLAY_RADIUS: f32 = 65.0;
pub const MINION_MOVE_SPEED: f32 = 325.0;

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
}

impl World {
    pub fn new(seed: u64) -> Self {
        Self { tick: Tick(0), units: Vec::new(), next_unit: 1, rng: Pcg32::new(seed, 0x4d46_5452) }
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

    pub fn despawn(&mut self, id: UnitId) {
        self.units.retain(|u| u.id != id);
    }

    /// Build a partial world, as client prediction does (own unit plus collision proxies).
    pub fn from_units(tick: Tick, units: Vec<Unit>) -> Self {
        let next_unit = units.iter().map(|u| u.id.0 + 1).max().unwrap_or(1);
        Self { tick, units, next_unit, rng: Pcg32::new(0, 0) }
    }

    /// Replace every unit except `keep` (client prediction refreshes its collision proxies).
    pub fn replace_others(&mut self, keep: UnitId, others: impl IntoIterator<Item = Unit>) {
        self.units.retain(|u| u.id == keep);
        self.units.extend(others.into_iter().filter(|u| u.id != keep));
        self.units.sort_by_key(|u| u.id);
    }

    /// Rewind or fast-forward the tick counter (prediction reconciliation only).
    pub fn set_tick(&mut self, tick: Tick) {
        self.tick = tick;
    }

    /// Simulate the next tick. Only commands whose `tick` equals the new tick are applied, in
    /// `(sub, player, seq)` order. Each unit integrates piecewise between its commands' sub-tick
    /// times, so a 30 Hz tick adds no input quantization.
    pub fn step(&mut self, commands: &[Command]) {
        let k = self.tick.next();
        let mut cmds: Vec<&Command> = commands.iter().filter(|c| c.tick == k).collect();
        cmds.sort_by_key(|c| (c.sub, c.player, c.seq));

        // Phase 1: AI.
        for unit in &mut self.units {
            if let Some(Brain::Patrol { a, b, toward_b }) = unit.brain
                && unit.state.order == Order::Idle
            {
                unit.state.set_order(Order::MoveTo(if toward_b { b } else { a }));
                unit.brain = Some(Brain::Patrol { a, b, toward_b: !toward_b });
            }
        }

        // Phase 2: movement against start-of-tick positions (order-independent).
        let starts: Vec<(UnitId, Obstacle)> = self
            .units
            .iter()
            .filter(|u| u.collision_radius > 0.0)
            .map(|u| (u.id, Obstacle { pos: u.state.pos, radius: u.collision_radius }))
            .collect();
        for unit in &mut self.units {
            let here = unit.state.pos;
            let obstacles: Vec<Obstacle> = starts
                .iter()
                .filter(|(id, o)| *id != unit.id && (o.pos - here).length_sq() < BROADPHASE * BROADPHASE)
                .map(|(_, o)| *o)
                .collect();
            let radius = unit.collision_radius;
            let (mut desired, mut achieved) = (0.0f32, 0.0f32);
            let mut elapsed = 0.0f32;
            if let Some(owner) = unit.owner {
                for c in cmds.iter().filter(|c| c.player == owner) {
                    let at = c.sub.fraction();
                    let (d, a) = unit.state.advance((at - elapsed) * TICK_DT, radius, &obstacles);
                    desired += d;
                    achieved += a;
                    elapsed = at;
                    unit.state.set_order(match c.kind {
                        CommandKind::MoveTo(q) => Order::MoveTo(q),
                        CommandKind::Stop => Order::Idle,
                    });
                }
            }
            let (d, a) = unit.state.advance((1.0 - elapsed) * TICK_DT, radius, &obstacles);
            desired += d;
            achieved += a;
            update_stuck(&mut unit.state, desired, achieved, radius, &obstacles);
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
            }
        }
        h.finish()
    }
}

/// Stuck detection and detours (03a §5): poor progress for a few ticks → take a short detour
/// around the blockers. If the goal itself is occupied (clicked into a clump) or already
/// within reach, stop where we are, like the reference game does.
fn update_stuck(state: &mut UnitState, desired: f32, achieved: f32, radius: f32, obstacles: &[Obstacle]) {
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
        state.set_order(Order::Idle);
        return;
    }
    let reach = 2.0 * (radius + 35.0);
    state.detour = choose_detour(state.pos, goal, radius, reach, obstacles);
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
                    cmds.push(cmd(p, seq, k, sub, t));
                }
            }
            w.step(&cmds);
        }
        assert_eq!(w.tick(), Tick(9000));
        assert_eq!(w.state_hash(), GOLDEN_HASH, "hash = {:#018x}", w.state_hash());
    }

    /// Recorded on x86_64-pc-windows-msvc. CI checks Linux, macOS (aarch64) and Windows.
    const GOLDEN_HASH: u64 = 0x6b1b_a529_2846_cc42;
}
