//! Veils (Shade's shroud, 01 §9): a dash that leaves a cloud of smoke where it lands. While its
//! caster stands inside, the enemy can't see it, so attack orders on it end and nothing new
//! can target it; skillshots and areas still hit what they touch. Attacking, casting or
//! dealing damage from inside shows it for a moment. Turrets see into it, and so does a
//! control ward's reveal. The cloud itself is seen by everyone.
//!
//! The simulation keeps the veils (hashed, so replays and prediction agree); vision (and from
//! it the server's per-team hidden lists, which end attack orders) asks [`veiled`].

use crate::hash::StateSink;
use crate::math::Vec2;
use crate::time::{SimDuration, SimTime};
use crate::world::{SimEvent, Team, Unit, UnitId, UnitKind};

/// How long acting from inside a veil shows its caster.
pub const REVEAL: SimDuration = SimDuration::from_millis(750);

/// A dash's veil: the cloud's radius and how long it lasts once the dash lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VeilSpec {
    pub radius: f32,
    pub duration: SimDuration,
}

impl VeilSpec {
    /// No veil (an ordinary dash).
    pub const NONE: VeilSpec = VeilSpec { radius: 0.0, duration: SimDuration(0) };

    pub fn some(self) -> bool {
        self.radius > 0.0 && self.duration.0 > 0
    }
}

/// A cloud on the ground.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Veil {
    pub owner: UnitId,
    pub team: Team,
    pub center: Vec2,
    pub radius: f32,
    /// From when the dash lands until it fades.
    pub from: SimTime,
    pub until: SimTime,
    /// The owner shows until then (it acted from inside).
    pub revealed_until: SimTime,
}

impl Veil {
    pub fn active(&self, t: SimTime) -> bool {
        t >= self.from && t < self.until
    }

    fn hash_into(&self, h: &mut impl StateSink) {
        h.write_u32(self.owner.0);
        h.write_u8(self.team as u8);
        h.write_f32(self.center.x);
        h.write_f32(self.center.y);
        h.write_f32(self.radius);
        h.write_u64(self.from.0);
        h.write_u64(self.until.0);
        h.write_u64(self.revealed_until.0);
    }
}

/// Bring the veils up to the end of a tick (`s1`), from that tick's `events`: new clouds,
/// casters showing themselves by acting, clouds that faded.
pub fn update(veils: &mut Vec<Veil>, events: &[SimEvent], s1: SimTime) {
    for e in events {
        match *e {
            SimEvent::Veiled { unit, team, center, radius, from, until } => {
                // One cloud per caster: a new one replaces the old.
                veils.retain(|v| v.owner != unit);
                veils.push(Veil { owner: unit, team, center, radius, from, until, revealed_until: SimTime(0) });
            }
            SimEvent::CastStarted { unit, at, .. } | SimEvent::ItemUsed { unit, at, .. } => show(veils, unit, at),
            SimEvent::Damage { source, at, .. } => show(veils, source, at),
            _ => {}
        }
    }
    veils.retain(|v| v.until > s1);
}

fn show(veils: &mut [Veil], unit: UnitId, at: SimTime) {
    for v in veils.iter_mut().filter(|v| v.owner == unit && v.active(at)) {
        v.revealed_until = v.revealed_until.max(at.plus(REVEAL));
    }
}

/// Whether `unit` is hidden in its own veil at `t` (inside, the cloud up, not showing).
pub fn veiled(veils: &[Veil], unit: &Unit, t: SimTime) -> bool {
    unit.kind == UnitKind::Champion
        && unit.state.alive()
        && veils.iter().any(|v| {
            v.owner == unit.id && v.active(t) && t >= v.revealed_until && unit.state.pos.distance(v.center) <= v.radius
        })
}

pub fn hash_into(veils: &[Veil], h: &mut impl StateSink) {
    // No veils: the hash is what it was before there were any.
    if veils.is_empty() {
        return;
    }
    h.write_u32(0x5645_494c);
    h.write_u32(veils.len() as u32);
    for v in veils {
        v.hash_into(h);
    }
}
