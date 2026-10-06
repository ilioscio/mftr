//! `mftr-sim`: the deterministic MFTR simulation core.
//!
//! Each tick is a pure function of `(state, commands)`. There is no I/O, no wall clock,
//! no threads and no platform-dependent math. Server and client run this exact code;
//! see `docs/design/03-netcode.md` §17 for the determinism policy.

pub mod ability;
pub mod champion;
pub mod collision;
pub mod combat;
pub mod hash;
pub mod lane;
pub mod map;
pub mod math;
pub mod projectile;
pub mod rng;
pub mod time;
pub mod vision;
pub mod world;

pub use champion::ChampionId;
pub use math::{QPoint, Vec2};
pub use time::{SUBTICKS, SimDuration, SimTime, SubTick, TICK_DT, TICK_DT_F64, TICK_HZ, Tick};
pub use world::{
    Area, AttackWindup, Bolt, Brain, Cast, Command, CommandKind, DashMove, MinionKind, Missile, Order, PlayerId,
    SimEvent, Team, Unit, UnitId, UnitKind, UnitState, World,
};
