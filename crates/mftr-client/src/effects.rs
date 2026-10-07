//! Delayed ground areas and basic-attack bolts: tracking and display timelines (03a §7).
//!
//! - **Areas:** enemy telegraphs are drawn on `T_input` from the first event, so the
//!   detonation you see is when the server judges it against your own (predicted) position.
//!   Own areas appear at once from prediction (also on `T_input`); allied ones on `T_interp`.
//! - **Bolts** are homing and not dodgeable, so they only need to look right: own bolts leave
//!   the hand on `T_input` (predicted), everyone else's fly on `T_interp`. All of them aim at
//!   wherever the target is drawn.
//!
//! Damage is never predicted: health changes and numbers come from the server.

use crate::missiles::Side;
use mftr_sim::{Area, Bolt, SUBTICKS, SimTime, Team, UnitId, Vec2};
use std::collections::BTreeMap;

/// How long a detonation flash stays visible (seconds).
const FLASH_SECONDS: f64 = 0.2;
/// Own predictions the server never confirms (a rejected cast) are dropped after this.
const STALE_SECONDS: f64 = 1.0;

fn st(ticks: f64) -> f64 {
    ticks * SUBTICKS as f64
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AreaRender {
    /// Server id, or `u32::MAX - (cast_seq << 4 | shot)` for an own area not yet confirmed.
    pub key: u32,
    pub side: Side,
    pub center: Vec2,
    pub radius: f32,
    /// 0 at the telegraph's start, 1 at detonation (the inside fills toward it, 05 §1).
    pub progress: f32,
    /// Showing the detonation flash.
    pub detonated: bool,
    /// Stuns, roots, knocks up or pulls (drawn with the hard-CC accent).
    pub hard_cc: bool,
    /// The caster (its champion's VFX, A4b).
    pub owner: UnitId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoltRender {
    pub key: u32,
    pub side: Side,
    pub pos: Vec2,
    pub dir: Vec2,
    /// The attacker (its champion's VFX, A4b).
    pub owner: UnitId,
}

#[derive(Clone, Copy, Debug)]
struct TrackedBolt {
    b: Bolt,
    side: Side,
    landed: Option<SimTime>,
}

#[derive(Default)]
pub struct EffectBook {
    areas: BTreeMap<u32, (Area, Side)>,
    /// Own areas predicted locally, by cast sequence.
    predicted_areas: BTreeMap<(u32, u8), Area>,
    bolts: BTreeMap<u32, TrackedBolt>,
    /// Own bolts predicted locally, by launch instant.
    predicted_bolts: BTreeMap<SimTime, Bolt>,
}

fn side_of(owner: UnitId, team: Team, own_unit: UnitId, own_team: Team) -> Side {
    if owner == own_unit {
        Side::Own
    } else if team == own_team {
        Side::Ally
    } else {
        Side::Enemy
    }
}

impl EffectBook {
    pub fn predict_area(&mut self, a: Area) {
        self.predicted_areas.insert((a.cast_seq, a.shot), a);
    }

    pub fn predicted_area_count(&self) -> usize {
        self.predicted_areas.len()
    }

    pub fn predict_bolt(&mut self, b: Bolt) {
        self.predicted_bolts.insert(b.launched_at, b);
    }

    pub fn on_area(&mut self, a: Area, own_unit: UnitId, own_team: Team) {
        let side = side_of(a.owner, a.team, own_unit, own_team);
        if side == Side::Own {
            self.predicted_areas.remove(&(a.cast_seq, a.shot));
        }
        self.areas.insert(a.id, (a, side));
    }

    pub fn on_bolt(&mut self, b: Bolt, own_unit: UnitId, own_team: Team) {
        let side = side_of(b.owner, b.team, own_unit, own_team);
        if side == Side::Own {
            self.predicted_bolts.remove(&b.launched_at);
        }
        self.bolts.insert(b.id, TrackedBolt { b, side, landed: None });
    }

    pub fn on_bolt_landed(&mut self, id: u32, at: SimTime) {
        if let Some(t) = self.bolts.get_mut(&id) {
            t.landed = Some(at);
        }
    }

    /// Keep own predictions made up to `keep_through` (later ones are re-created by
    /// re-simulation) and not older than `stale_before`.
    pub fn prune_predicted(&mut self, keep_through: SimTime, stale_before: SimTime) {
        self.predicted_areas.retain(|_, a| a.spawn_at <= keep_through && a.spawn_at >= stale_before);
        self.predicted_bolts.retain(|_, b| b.launched_at <= keep_through && b.launched_at >= stale_before);
    }

    /// Forget what has ended on every timeline (`t_interp`, in fractional ticks, is the latest).
    pub fn forget_before(&mut self, t_interp: f64) {
        let now = st(t_interp);
        let flash = st(FLASH_SECONDS * 30.0);
        self.areas.retain(|_, (a, _)| now < a.detonate_at.0 as f64 + flash);
        self.bolts.retain(|_, t| t.landed.is_none_or(|l| now < l.0 as f64 + st(1.0)));
        let stale = SimTime(st(t_interp - STALE_SECONDS * 30.0).max(0.0) as u64);
        self.prune_predicted(SimTime(u64::MAX), stale);
    }

    pub fn areas(&self, t_input: f64, t_interp: f64) -> Vec<AreaRender> {
        let mut out = Vec::new();
        let mut push = |key: u32, a: &Area, side: Side| {
            let at = st(if side == Side::Ally { t_interp } else { t_input });
            let (s, d) = (a.spawn_at.0 as f64, a.detonate_at.0 as f64);
            if at < s || at >= d + st(FLASH_SECONDS * 30.0) {
                return;
            }
            let progress = ((at - s) / (d - s).max(1.0)).min(1.0) as f32;
            out.push(AreaRender {
                key,
                side,
                center: a.center,
                radius: a.radius,
                progress,
                detonated: at >= d,
                hard_cc: a.cc.is_hard(),
                owner: a.owner,
            });
        };
        for (&(seq, shot), a) in &self.predicted_areas {
            push(u32::MAX - (seq.wrapping_shl(4) | shot as u32), a, Side::Own);
        }
        for (id, (a, side)) in &self.areas {
            push(*id, a, *side);
        }
        out
    }

    /// Bolts in flight. `drawn` gives where a unit is drawn this frame (the bolt's target).
    pub fn bolts(&self, t_input: f64, t_interp: f64, drawn: &dyn Fn(UnitId) -> Option<Vec2>) -> Vec<BoltRender> {
        let mut out = Vec::new();
        let mut push = |key: u32, b: &Bolt, side: Side, landed: Option<SimTime>| {
            let at = st(if side == Side::Own { t_input } else { t_interp });
            if at < b.launched_at.0 as f64 || landed.is_some_and(|l| at >= l.0 as f64) {
                return;
            }
            let Some(target) = drawn(b.target) else { return };
            let to = target - b.origin;
            let flown = (at - b.launched_at.0 as f64) as f32 / (SUBTICKS as f32 * 30.0) * b.speed;
            let dist = to.length();
            if flown >= dist {
                return; // arrived where the target is drawn; damage shows when confirmed
            }
            let dir = to.normalize_or_zero();
            out.push(BoltRender { key, side, pos: b.origin + dir * flown, dir, owner: b.owner });
        };
        for (at, b) in &self.predicted_bolts {
            push(u32::MAX - (at.0 as u32), b, Side::Own, None);
        }
        for (id, t) in &self.bolts {
            push(*id, &t.b, t.side, t.landed);
        }
        out
    }
}
