//! Snapshot deltas and path coasting (03b §6, DECISIONS Q13).
//!
//! Other units are sent relative to a **baseline**: the last snapshot the client reports having
//! reconstructed. A unit whose fields didn't change since the baseline isn't sent at all; a
//! changed one carries only the field groups that changed.
//!
//! **Path coasting:** a moving unit's position at the new tick is predicted from the baseline
//! by advancing it along its replicated heading at its speed (the same math on both sides, on
//! quantized inputs, so the result is identical). If the true position is within
//! [`COAST_TOLERANCE`] of that, the position is omitted. Minions walking a lane, or anyone
//! walking a straight path, then cost nothing until they turn or stop.
//!
//! The server keeps what the client *reconstructs* (coasted positions included), never the
//! truth, as the next baseline, so coasting errors never accumulate beyond the tolerance.

use crate::msg::RemoteUnit;
use mftr_sim::{QPoint, TICK_DT, UnitId};

/// Coasted positions this close to the truth (in game units) are good enough: not sent. Two
/// quantization steps: larger values (2 u was tried) cost nothing in bandwidth but nudge
/// predicted interceptions (phantom hits in the lab duel).
pub const COAST_TOLERANCE: f32 = 0.5;

/// Field groups of a unit update.
pub const STATIC: u8 = 1; // kind, team, radii, champion: only when a unit is new to the client
pub const POS: u8 = 2;
pub const MOTION: u8 = 4; // heading target and speed
pub const VITALS: u8 = 8; // health, max health, shield, level
pub const FLAGS: u8 = 16; // casting, attacking, stunned, rooted, dashing, protected, slowed
pub const ALL: u8 = STATIC | POS | MOTION | VITALS | FLAGS;

/// One unit in a delta snapshot: `mask` says which groups of `unit` are meaningful.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitUpdate {
    pub mask: u8,
    pub unit: RemoteUnit,
}

/// Where `base` is expected `ticks` later: along its heading at its speed, stopping at the
/// heading point. Deterministic on every platform (IEEE `+ - * / sqrt` on quantized inputs).
pub fn coast(base: &RemoteUnit, ticks: u32) -> QPoint {
    let Some(target) = base.target else { return base.pos };
    if base.speed == 0 || ticks == 0 {
        return base.pos;
    }
    let p = base.pos.to_vec2();
    let to = target.to_vec2() - p;
    let len = to.length();
    let step = base.speed as f32 * TICK_DT * ticks as f32;
    if len <= step {
        return target;
    }
    QPoint::from_vec2(p + to * (step / len))
}

fn flags(u: &RemoteUnit) -> [bool; 7] {
    [u.casting, u.attacking, u.stunned, u.rooted, u.dashing, u.protected, u.slowed]
}

/// The update to send for `current`, given the client's `base` record `ticks` ago, and the
/// record the client will reconstruct from it. `None` = nothing to send (the unit coasts).
pub fn diff(base: Option<&RemoteUnit>, current: &RemoteUnit, ticks: u32) -> (Option<UnitUpdate>, RemoteUnit) {
    let Some(b) = base else {
        return (Some(UnitUpdate { mask: ALL, unit: *current }), *current);
    };
    let mut mask = 0;
    let coasted = coast(b, ticks);
    let mut record = *current;
    if coasted.to_vec2().distance(current.pos.to_vec2()) <= COAST_TOLERANCE {
        record.pos = coasted;
    } else {
        mask |= POS;
    }
    if (b.target, b.speed) != (current.target, current.speed) {
        mask |= MOTION;
    }
    if (b.health, b.max_health, b.shield, b.level)
        != (current.health, current.max_health, current.shield, current.level)
    {
        mask |= VITALS;
    }
    if flags(b) != flags(current) {
        mask |= FLAGS;
    }
    if (b.kind, b.team, b.collision_radius, b.gameplay_radius, b.champion, b.augments)
        != (
            current.kind,
            current.team,
            current.collision_radius,
            current.gameplay_radius,
            current.champion,
            current.augments,
        )
    {
        mask |= STATIC;
    }
    ((mask != 0).then_some(UnitUpdate { mask, unit: record }), record)
}

/// What the client reconstructs: the baseline coasted `ticks` forward, with the update's
/// groups applied on top (a unit without a baseline must carry every group).
pub fn apply(base: Option<&RemoteUnit>, update: Option<&UnitUpdate>, ticks: u32) -> Option<RemoteUnit> {
    let mut r = match (base, update) {
        (Some(b), _) => RemoteUnit { pos: coast(b, ticks), ..*b },
        (None, Some(u)) if u.mask & STATIC != 0 => u.unit,
        _ => return None,
    };
    let Some(u) = update else { return Some(r) };
    let n = &u.unit;
    if u.mask & STATIC != 0 {
        (r.kind, r.team, r.collision_radius, r.gameplay_radius, r.champion, r.augments) =
            (n.kind, n.team, n.collision_radius, n.gameplay_radius, n.champion, n.augments);
    }
    if u.mask & POS != 0 {
        r.pos = n.pos;
    }
    if u.mask & MOTION != 0 {
        (r.target, r.speed) = (n.target, n.speed);
    }
    if u.mask & VITALS != 0 {
        (r.health, r.max_health, r.shield, r.level) = (n.health, n.max_health, n.shield, n.level);
    }
    if u.mask & FLAGS != 0 {
        [r.casting, r.attacking, r.stunned, r.rooted, r.dashing, r.protected, r.slowed] = flags(n);
    }
    r.id = n.id;
    Some(r)
}

/// The full set of units a client reconstructs from a baseline set and a delta snapshot.
pub fn reconstruct(
    base: &std::collections::BTreeMap<UnitId, RemoteUnit>,
    updates: &[UnitUpdate],
    removed: &[UnitId],
    ticks: u32,
) -> std::collections::BTreeMap<UnitId, RemoteUnit> {
    let mut out = std::collections::BTreeMap::new();
    for (id, b) in base {
        if removed.contains(id) {
            continue;
        }
        let u = updates.iter().find(|u| u.unit.id == *id);
        if let Some(r) = apply(Some(b), u, ticks) {
            out.insert(*id, r);
        }
    }
    for u in updates.iter().filter(|u| !base.contains_key(&u.unit.id)) {
        if let Some(r) = apply(None, Some(u), ticks) {
            out.insert(u.unit.id, r);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::{Team, UnitKind, Vec2};

    fn unit(x: f32, target: Option<(f32, f32)>) -> RemoteUnit {
        RemoteUnit {
            id: UnitId(7),
            kind: UnitKind::Minion,
            team: Team::Red,
            pos: QPoint::from_vec2(Vec2::new(x, 1500.0)),
            target: target.map(|(x, y)| QPoint::from_vec2(Vec2::new(x, y))),
            speed: 325,
            collision_radius: 25,
            gameplay_radius: 48,
            protected: false,
            champion: None,
            augments: [0; 4],
            health: 300,
            max_health: 300,
            shield: 0,
            level: 0,
            casting: false,
            attacking: false,
            stunned: false,
            rooted: false,
            dashing: false,
            slowed: false,
        }
    }

    #[test]
    fn a_unit_walking_its_path_coasts_for_free() {
        let base = unit(1000.0, Some((9000.0, 1500.0)));
        // 10 ticks later it is exactly where coasting puts it: nothing to send.
        let now = RemoteUnit { pos: coast(&base, 10), ..base };
        let (upd, record) = diff(Some(&base), &now, 10);
        assert!(upd.is_none());
        assert_eq!(apply(Some(&base), None, 10), Some(record));
    }

    #[test]
    fn turning_stopping_and_damage_send_only_their_groups() {
        let base = unit(1000.0, Some((9000.0, 1500.0)));
        let mut now = RemoteUnit { pos: QPoint::from_vec2(Vec2::new(1060.0, 1520.0)), target: None, ..base };
        now.health = 250;
        let (upd, record) = diff(Some(&base), &now, 3);
        let upd = upd.unwrap();
        assert_eq!(upd.mask, POS | MOTION | VITALS);
        assert_eq!(apply(Some(&base), Some(&upd), 3), Some(record));
        assert_eq!(record, now);
    }

    #[test]
    fn reconstruction_matches_the_server_records_exactly() {
        let mut base = std::collections::BTreeMap::new();
        for i in 0..20u32 {
            let mut u = unit(500.0 + 100.0 * i as f32, if i % 3 == 0 { None } else { Some((9000.0, 1500.0)) });
            u.id = UnitId(i);
            base.insert(u.id, u);
        }
        // New positions: some coast, some deviate, one leaves, one arrives.
        let mut current: Vec<RemoteUnit> = base
            .values()
            .filter(|u| u.id != UnitId(4))
            .map(|u| {
                let mut n = RemoteUnit { pos: coast(u, 5), ..*u };
                if u.id.0 % 4 == 1 {
                    n.pos = QPoint { x: n.pos.x + 40, y: n.pos.y };
                }
                n
            })
            .collect();
        current.push(RemoteUnit { id: UnitId(99), ..unit(7000.0, None) });
        let mut records = std::collections::BTreeMap::new();
        let mut updates = Vec::new();
        for c in &current {
            let (u, r) = diff(base.get(&c.id), c, 5);
            updates.extend(u);
            records.insert(c.id, r);
        }
        let removed = vec![UnitId(4)];
        assert!(updates.len() < current.len() / 2, "most units coast: {} updates", updates.len());
        assert_eq!(reconstruct(&base, &updates, &removed, 5), records);
    }
}
