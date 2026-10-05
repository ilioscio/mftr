//! The simulation world: units, commands and the fixed-tick step.
//!
//! M0 scope: champions moving on a flat plane with sub-tick commands. Unit collision, abilities
//! and vision land in M1 on top of this structure.

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
pub enum Order {
    Idle,
    MoveTo(QPoint),
}

/// Everything client prediction needs to reproduce a unit's movement bit-exactly.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitState {
    pub pos: Vec2,
    pub order: Order,
    pub move_speed: f32,
}

impl UnitState {
    /// Advance by `dt` seconds under the current order. Constant speed, instant turns
    /// (matches the reference game's feel, see R01 §2).
    pub fn advance(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        if let Order::MoveTo(q) = self.order {
            let target = q.to_vec2();
            let to = target - self.pos;
            let dist = to.length();
            let step = self.move_speed * dt;
            if step >= dist {
                self.pos = target;
                self.order = Order::Idle;
            } else {
                self.pos += to * (step / dist);
            }
        }
    }

    /// Bit-exact equality: what prediction reconciliation compares.
    pub fn bits_eq(&self, other: &Self) -> bool {
        self.pos.to_bits() == other.pos.to_bits()
            && self.order == other.order
            && self.move_speed.to_bits() == other.move_speed.to_bits()
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
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub id: UnitId,
    pub owner: Option<PlayerId>,
    pub team: Team,
    pub state: UnitState,
    /// Unit-vs-unit movement blocking (D11).
    pub collision_radius: f32,
    /// Hitbox for abilities and targeting.
    pub gameplay_radius: f32,
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

    pub fn spawn_champion(&mut self, owner: PlayerId, team: Team, pos: Vec2) -> UnitId {
        let id = UnitId(self.next_unit);
        self.next_unit += 1;
        self.units.push(Unit {
            id,
            owner: Some(owner),
            team,
            state: UnitState { pos, order: Order::Idle, move_speed: CHAMPION_MOVE_SPEED },
            collision_radius: CHAMPION_COLLISION_RADIUS,
            gameplay_radius: CHAMPION_GAMEPLAY_RADIUS,
        });
        id
    }

    pub fn despawn(&mut self, id: UnitId) {
        self.units.retain(|u| u.id != id);
    }

    /// Build a partial world, as client prediction does (own unit only, at a known tick).
    pub fn from_units(tick: Tick, units: Vec<Unit>) -> Self {
        let next_unit = units.iter().map(|u| u.id.0 + 1).max().unwrap_or(1);
        Self { tick, units, next_unit, rng: Pcg32::new(0, 0) }
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
        for unit in &mut self.units {
            let mut elapsed = 0.0f32;
            if let Some(owner) = unit.owner {
                for c in cmds.iter().filter(|c| c.player == owner) {
                    let at = c.sub.fraction();
                    unit.state.advance((at - elapsed) * TICK_DT);
                    elapsed = at;
                    unit.state.order = match c.kind {
                        CommandKind::MoveTo(q) => Order::MoveTo(q),
                        CommandKind::Stop => Order::Idle,
                    };
                }
            }
            unit.state.advance((1.0 - elapsed) * TICK_DT);
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
        }
        h.finish()
    }
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

    /// Re-simulating the same commands from a snapshot must be bit-identical:
    /// this is what client prediction relies on.
    #[test]
    fn partial_world_replay_is_bit_exact() {
        let mut full = World::new(7);
        let a = full.spawn_champion(PlayerId(0), Team::Blue, Vec2::new(500.0, 500.0));
        full.spawn_champion(PlayerId(1), Team::Red, Vec2::new(3000.0, 3000.0));
        let mut rng = Pcg32::new(99, 1);
        let mut cmds = Vec::new();
        for i in 0..300u32 {
            if rng.next_u32() % 5 == 0 {
                let p = (rng.next_u32() % 2) as u8;
                let sub = (rng.next_u32() % SUBTICKS as u32) as u8;
                let t = (rng.range_f32(0.0, 4000.0), rng.range_f32(0.0, 4000.0));
                cmds.push(cmd(p, i, i + 1, sub, t));
            }
        }
        // Snapshot player 0's unit at tick 100, replay only its own commands.
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
    const GOLDEN_HASH: u64 = 0xc301_f4d6_0984_add6;
}
