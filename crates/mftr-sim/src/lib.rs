//! `mftr-sim`: the deterministic MFTR simulation core.
//!
//! Each tick is a pure function of `(state, commands)`. There is no I/O, no wall clock,
//! no threads and no platform-dependent math. Server and client run this exact code;
//! see `docs/design/03-netcode.md` §17 for the determinism policy.

pub mod combat;
pub mod hash;
pub mod math;
pub mod projectile;
pub mod rng;
pub mod time;
pub mod world;

pub use math::{QPoint, Vec2};
pub use time::{SUBTICKS, SubTick, TICK_DT, TICK_DT_F64, TICK_HZ, Tick};
pub use world::{Command, CommandKind, Order, PlayerId, Team, Unit, UnitId, UnitState, World};
