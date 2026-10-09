//! Vision tools (01 §8, M4 slice 4): wards and the trinkets that place or hunt them.
//!
//! - A **stealth ward** (the Warding Totem trinket, T) watches an area for a while. Enemies
//!   can't see it unless something reveals it.
//! - A **control ward** (a shop item) watches until it's destroyed, can be seen, and reveals and
//!   disables enemy stealth wards in its sight: they give their team no vision.
//! - The **Sweeping Lens** trinket does the same around its champion for a few seconds.
//!
//! Wards take one point of damage from each basic attack (abilities, minions and turrets leave
//! them alone) and pay a little gold to whoever destroys one. Maps opt in (`Layout::wards`).

use crate::champion::Stats;
use crate::math::Vec2;
use crate::time::{SimDuration, SimTime};
use crate::world::{Brain, Team, Unit, UnitId, UnitKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WardKind {
    Stealth = 0,
    Control = 1,
}

impl WardKind {
    pub fn from_wire(v: u8) -> Option<Self> {
        match v {
            0 => Some(WardKind::Stealth),
            1 => Some(WardKind::Control),
            _ => None,
        }
    }

    /// Basic attacks it takes to destroy.
    pub fn hits(self) -> f32 {
        match self {
            WardKind::Stealth => 3.0,
            WardKind::Control => 4.0,
        }
    }

    /// Gold for destroying an enemy one.
    pub fn gold(self) -> f32 {
        match self {
            WardKind::Stealth => 15.0,
            WardKind::Control => 30.0,
        }
    }
}

/// How far a ward sees (and a control ward reveals).
pub const WARD_VISION: f32 = 900.0;
/// How far from its champion a ward can be placed (farther clicks place it at this range).
pub const PLACE_RANGE: f32 = 600.0;
/// Stealth wards a champion can have out (placing another removes the oldest), control wards.
pub const MAX_STEALTH: usize = 3;
pub const MAX_CONTROL: usize = 1;
/// A stealth ward lasts this long, plus `STEALTH_LIFE_PER_LEVEL` a level of its placer.
pub const STEALTH_LIFE: SimDuration = SimDuration::from_millis(90_000);
pub const STEALTH_LIFE_PER_LEVEL: SimDuration = SimDuration::from_millis(3000);

/// The trinkets (`Progress::trinket`).
pub const TRINKET_TOTEM: u8 = 0;
pub const TRINKET_LENS: u8 = 1;
/// The Warding Totem holds up to 2 charges and regains one every 2 minutes.
pub const TOTEM_CHARGES: u8 = 2;
pub const TOTEM_RECHARGE: SimDuration = SimDuration::from_millis(120_000);
/// The Sweeping Lens reveals and disables enemy stealth wards within this of its champion,
/// for this long, then cools down.
pub const SWEEP_RADIUS: f32 = 450.0;
pub const SWEEP_FOR: SimDuration = SimDuration::from_millis(6000);
pub const LENS_COOLDOWN: SimDuration = SimDuration::from_millis(90_000);
/// Swapping trinkets (in the shop) leaves the new one unready this long.
pub const TRINKET_SWAP: SimDuration = SimDuration::from_millis(30_000);

/// How long a stealth ward placed by a champion of `level` lasts.
pub fn stealth_life(level: u8) -> SimDuration {
    SimDuration(STEALTH_LIFE.0 + STEALTH_LIFE_PER_LEVEL.0 * level as u64)
}

/// Where a ward ordered at `target` lands for a champion at `from`: no farther than
/// `PLACE_RANGE`.
pub fn placement(from: Vec2, target: Vec2) -> Vec2 {
    let to = target - from;
    let len = to.length();
    if len <= PLACE_RANGE { target } else { from + to * (PLACE_RANGE / len) }
}

/// A ward unit: no collision, hit points counting attacks, gone at `expires` (0: never).
pub fn ward_unit(id: UnitId, kind: WardKind, team: Team, pos: Vec2, placed_by: UnitId, expires: SimTime) -> Unit {
    let stats = Stats { max_health: kind.hits(), ..Stats::NONE };
    let mut u = Unit::new(id, UnitKind::Ward, team, pos, (0.0, 35.0), stats);
    u.ward = Some(kind);
    u.placed_by = Some(placed_by);
    u.brain = Some(Brain::Ward { expires });
    u
}

/// The places `team` reveals stealth wards in now: its control wards' sight and its active
/// sweeps (center, radius).
pub fn revealers(units: &[Unit], team: Team, now: SimTime) -> Vec<(Vec2, f32)> {
    units
        .iter()
        .filter(|u| u.team == team && u.state.alive())
        .filter_map(|u| match (u.kind, u.ward) {
            (UnitKind::Ward, Some(WardKind::Control)) => Some((u.state.pos, WARD_VISION)),
            (UnitKind::Champion, _) if u.state.progress.sweep_until > now => Some((u.state.pos, SWEEP_RADIUS)),
            _ => None,
        })
        .collect()
}

/// The other side.
pub fn enemy_of(team: Team) -> Team {
    match team {
        Team::Blue => Team::Red,
        Team::Red => Team::Blue,
        Team::Neutral => Team::Neutral,
    }
}
