//! Aiming (17 §3): where a target will be, led as much as the tier leads, with the tier's error
//! across the shot.

use crate::kit;
use crate::memory::velocity;
use mftr_sim::ability::{Ability, Effect};
use mftr_sim::rng::Pcg32;
use mftr_sim::{SimTime, Unit, Vec2};

/// Where `u` will be `t` seconds from `now`: still while it can't move, then along its path
/// at its speed (stopping at the path's end), scaled by `lead` (0: where it stands).
pub fn predict(u: &Unit, now: SimTime, t: f32, lead: f32) -> Vec2 {
    let st = &u.state;
    let moving = (t - kit::locked_for(u, now)).max(0.0) * lead;
    if moving <= 0.0 {
        return st.pos;
    }
    if let Some(d) = st.dash {
        let left = d.to - st.pos;
        return st.pos + left.normalize_or_zero() * (d.speed * moving).min(left.length());
    }
    let v = velocity(u, now);
    let speed = v.length();
    if speed < 1.0 {
        return st.pos;
    }
    // Walk the remaining path.
    let mut left = speed * moving;
    let mut at = st.pos;
    let detour = st.detour.into_iter();
    for wp in detour.chain(st.path.waypoints().iter().copied()) {
        let d = at.distance(wp);
        if d >= left {
            return at + (wp - at).normalize_or_zero() * left;
        }
        left -= d;
        at = wp;
    }
    at
}

/// A normal sample (Box–Muller), mean 0, deviation 1.
fn gauss(rng: &mut Pcg32) -> f32 {
    let u1 = rng.next_f32().max(1e-6);
    let u2 = rng.next_f32();
    (-2.0 * u1.ln()).sqrt() * (std::f32::consts::TAU * u2).cos()
}

/// The point to cast `a` at to hit `target` from `from`: its predicted spot when the ability
/// lands (solved over a few rounds for missiles, whose flight depends on the distance), plus
/// the tier's error across the shot (`error` u at 1,000 u).
pub fn aim(a: &Ability, from: Vec2, target: &Unit, now: SimTime, lead: f32, error: f32, rng: &mut Pcg32) -> Vec2 {
    let mut p = target.state.pos;
    for _ in 0..3 {
        let t = kit::impact_time(a, from.distance(p));
        p = predict(target, now, t, lead);
    }
    let to = p - from;
    let dist = to.length();
    if dist < 1.0 || error <= 0.0 {
        return p;
    }
    let dir = to * (1.0 / dist);
    let across = Vec2::new(-dir.y, dir.x);
    let scale = (dist / 1000.0).clamp(0.3, 1.3);
    let mut out = p + across * (gauss(rng) * error * scale);
    // Areas also miss short or long a little.
    if matches!(a.effect, Effect::Area(_)) {
        out += dir * (gauss(rng) * error * 0.5 * scale);
    }
    out
}

/// The center for an area of `radius` (cast from `from`, reaching `range`) that catches the most
/// value: each candidate (a target's predicted spot, or between two) scored by the summed value
/// of the units inside. Returns the center and its value.
pub fn best_area(from: Vec2, range: f32, radius: f32, targets: &[(Vec2, f32, f32)]) -> Option<(Vec2, f32)> {
    let score = |c: Vec2| -> f32 {
        targets.iter().filter(|(p, r, _)| p.distance(c) <= radius + r * 0.5).map(|(_, _, v)| *v).sum()
    };
    let mut best: Option<(Vec2, f32)> = None;
    let mut consider = |c: Vec2| {
        let c = if range > 0.0 && c.distance(from) > range { from + (c - from).normalize_or_zero() * range } else { c };
        let s = score(c);
        if best.is_none_or(|(_, b)| s > b) {
            best = Some((c, s));
        }
    };
    for (i, a) in targets.iter().enumerate() {
        consider(a.0);
        for b in targets.iter().skip(i + 1).take(6) {
            if a.0.distance(b.0) <= radius * 2.0 {
                consider(a.0.lerp(b.0, 0.5));
            }
        }
    }
    best.filter(|(_, s)| *s > 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn areas_go_where_they_catch_the_most() {
        let targets = [
            (Vec2::new(0.0, 500.0), 65.0, 1.0),
            (Vec2::new(100.0, 520.0), 65.0, 1.0),
            (Vec2::new(-800.0, 0.0), 65.0, 1.5),
        ];
        let (c, v) = best_area(Vec2::ZERO, 900.0, 200.0, &targets).unwrap();
        assert!(v >= 2.0 && c.distance(Vec2::new(50.0, 510.0)) < 80.0, "{c:?} {v}");
        // A little out of range: clamped toward it, still catching its edge; far out: nothing.
        let near = [(Vec2::new(-380.0, 0.0), 65.0, 1.0)];
        let (c, _) = best_area(Vec2::ZERO, 300.0, 100.0, &near).unwrap();
        assert!((c.length() - 300.0).abs() < 1.0);
        assert!(best_area(Vec2::ZERO, 300.0, 100.0, &targets[2..]).is_none());
    }
}
