//! Movement blocking (D11/D23, `docs/design/03a-netcode-time-and-prediction.md` §5): units
//! against other units' collision circles, and against wall segments (slice 3).
//!
//! Rules:
//! - A moving unit is blocked by other units' collision circles at their **start-of-tick**
//!   positions, and by wall edges. Each unit's result depends only on its own state and those
//!   positions, so the outcome is independent of update order and client prediction can
//!   reproduce it from collision proxies and the shared map.
//! - No shoving: a mover never displaces anything. It stops at contact and slides along the
//!   remaining tangent motion.
//! - An overlap that already exists (spawn, two movers meeting head-on) only forbids moving
//!   deeper, so units separate naturally on the following ticks.

use crate::map::closest_point_on_segment;
use crate::math::Vec2;
use crate::projectile::first_contact;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Obstacle {
    pub pos: Vec2,
    pub radius: f32,
}

/// A wall edge.
pub type Wall = (Vec2, Vec2);

const PASSES: usize = 4;
/// Penetration below this is treated as touching (float noise at contact points).
const SKIN: f32 = 0.01;

/// Earliest normalized time a point moving `p → p + m` comes within `r` of segment `a-b`.
fn sweep_capsule(p: Vec2, m: Vec2, a: Vec2, b: Vec2, r: f32) -> Option<f32> {
    let mut best = first_contact(p, p + m, a, a, r);
    if let Some(t) = first_contact(p, p + m, b, b, r) {
        best = Some(best.map_or(t, |x: f32| x.min(t)));
    }
    let ab = b - a;
    let len = ab.length();
    if len > 0.0 {
        let d = ab * (1.0 / len);
        let n = Vec2::new(-d.y, d.x);
        let s0 = (p - a).dot(n);
        let sn = m.dot(n);
        if s0.abs() >= r && sn != 0.0 {
            let target = if s0 > 0.0 { r } else { -r };
            let t = (target - s0) / sn;
            if (0.0..=1.0).contains(&t) {
                let u = (p + m * t - a).dot(d);
                if (0.0..=len).contains(&u) {
                    best = Some(best.map_or(t, |x: f32| x.min(t)));
                }
            }
        }
    }
    best
}

/// Slide rule shared by circles and walls: advance to contact at `tau`, keep only the part of
/// the remaining motion that doesn't push into the contact normal `n`.
fn slide(m: Vec2, tau: f32, n: Vec2) -> Vec2 {
    let reach = m * tau;
    let rest = m * (1.0 - tau);
    let rn = rest.dot(n);
    reach + if rn < 0.0 { rest - n * rn } else { rest }
}

/// New position after trying to move `pos` by `delta`, blocked by `obstacles` and `walls`
/// (in a deterministic order). Never returns a position deeper inside anything than the start.
pub fn constrained_move(pos: Vec2, delta: Vec2, radius: f32, obstacles: &[Obstacle], walls: &[Wall]) -> Vec2 {
    let mut m = delta;
    let reach = delta.length() + radius + 1.0;
    let near_walls: Vec<Wall> =
        walls.iter().copied().filter(|&(a, b)| (pos - closest_point_on_segment(pos, a, b)).length() <= reach).collect();
    for _ in 0..PASSES {
        let mut changed = false;
        for o in obstacles {
            let r = radius + o.radius;
            let rel = pos - o.pos;
            let end = rel + m;
            if end.length_sq() >= r * r {
                continue;
            }
            let start_sq = rel.length_sq();
            if start_sq < r * r {
                // Already overlapping: only forbid moving closer.
                let len = start_sq.sqrt();
                if len <= 1e-4 {
                    continue;
                }
                let n = rel * (1.0 / len);
                let mn = m.dot(n);
                if mn < 0.0 {
                    m -= n * mn;
                    changed = true;
                }
                continue;
            }
            // Approaching: advance to contact, slide the remainder along the tangent.
            let Some(tau) = first_contact(pos, pos + m, o.pos, o.pos, r) else { continue };
            m = slide(m, tau, (rel + m * tau).normalize_or_zero());
            changed = true;
        }
        for &(a, b) in &near_walls {
            let end = pos + m;
            if (end - closest_point_on_segment(end, a, b)).length_sq() >= radius * radius {
                continue;
            }
            let rel = pos - closest_point_on_segment(pos, a, b);
            if rel.length_sq() < radius * radius {
                let len = rel.length();
                if len <= 1e-4 {
                    continue;
                }
                let n = rel * (1.0 / len);
                let mn = m.dot(n);
                if mn < 0.0 {
                    m -= n * mn;
                    changed = true;
                }
                continue;
            }
            let Some(tau) = sweep_capsule(pos, m, a, b, radius) else { continue };
            let contact = pos + m * tau;
            m = slide(m, tau, (contact - closest_point_on_segment(contact, a, b)).normalize_or_zero());
            changed = true;
        }
        if !changed {
            break;
        }
    }
    // Anything still deeper than at the start (pinned) cancels the move.
    for o in obstacles {
        let r = radius + o.radius - SKIN;
        let start = pos - o.pos;
        let end = start + m;
        if end.length_sq() < r * r && end.length_sq() < start.length_sq() {
            return pos;
        }
    }
    for &(a, b) in &near_walls {
        let end = pos + m;
        let de = (end - closest_point_on_segment(end, a, b)).length();
        let ds = (pos - closest_point_on_segment(pos, a, b)).length();
        if de < radius - SKIN && de < ds {
            return pos;
        }
    }
    pos + m
}

/// Sixteen unit directions (22.5° apart) as literals: no trigonometry in simulation code.
pub const DIRECTIONS_16: [Vec2; 16] = [
    Vec2::new(1.0, 0.0),
    Vec2::new(0.923_879_5, 0.382_683_43),
    Vec2::new(0.707_106_77, 0.707_106_77),
    Vec2::new(0.382_683_43, 0.923_879_5),
    Vec2::new(0.0, 1.0),
    Vec2::new(-0.382_683_43, 0.923_879_5),
    Vec2::new(-0.707_106_77, 0.707_106_77),
    Vec2::new(-0.923_879_5, 0.382_683_43),
    Vec2::new(-1.0, 0.0),
    Vec2::new(-0.923_879_5, -0.382_683_43),
    Vec2::new(-0.707_106_77, -0.707_106_77),
    Vec2::new(-0.382_683_43, -0.923_879_5),
    Vec2::new(0.0, -1.0),
    Vec2::new(0.382_683_43, -0.923_879_5),
    Vec2::new(0.707_106_77, -0.707_106_77),
    Vec2::new(0.923_879_5, -0.382_683_43),
];

/// Pick a short detour waypoint around blocking units: among 16 directions, the free one
/// (clear straight path and clear end point) that ends closest to `goal`. Deterministic.
pub fn choose_detour(
    pos: Vec2,
    goal: Vec2,
    radius: f32,
    reach: f32,
    obstacles: &[Obstacle],
    walls: &[Wall],
) -> Option<Vec2> {
    let mut best: Option<(f32, Vec2)> = None;
    for d in DIRECTIONS_16 {
        let cand = pos + d * reach;
        let blocked = obstacles.iter().any(|o| {
            let r = radius + o.radius;
            let rel = pos - o.pos;
            if (cand - o.pos).length_sq() < r * r {
                return true;
            }
            if rel.length_sq() <= (r + SKIN) * (r + SKIN) {
                // Touching or overlapping now: only directions into the obstacle are blocked.
                d.dot(rel) < 0.0
            } else {
                first_contact(pos, cand, o.pos, o.pos, r).is_some()
            }
        });
        let mid = pos + d * (reach * 0.5);
        let walled = walls.iter().any(|&(a, b)| {
            crate::map::segments_intersect(pos, cand, a, b)
                || crate::map::dist_point_segment(cand, a, b) < radius
                || crate::map::dist_point_segment(mid, a, b) < radius
        });
        if blocked || walled {
            continue;
        }
        let score = (goal - cand).length_sq();
        if best.is_none_or(|(s, _)| score < s) {
            best = Some((score, cand));
        }
    }
    best.map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ob(x: f32, y: f32, r: f32) -> Obstacle {
        Obstacle { pos: Vec2::new(x, y), radius: r }
    }

    #[test]
    fn free_motion_is_unchanged() {
        let p = constrained_move(Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0), 35.0, &[ob(500.0, 0.0, 25.0)], &[]);
        assert_eq!(p, Vec2::new(10.0, 0.0));
    }

    #[test]
    fn head_on_stops_at_contact() {
        // Contact distance 60; start 70 away, try to move 20.
        let p = constrained_move(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0), 35.0, &[ob(70.0, 0.0, 25.0)], &[]);
        assert!((p.x - 10.0).abs() < 1e-3 && p.y.abs() < 1e-3, "{p:?}");
    }

    #[test]
    fn glancing_contact_slides() {
        // Obstacle slightly off the path: the mover slides around it, losing some progress.
        let p = constrained_move(Vec2::new(0.0, 0.0), Vec2::new(20.0, 0.0), 35.0, &[ob(70.0, 30.0, 25.0)], &[]);
        let o = Vec2::new(70.0, 30.0);
        assert!(p.distance(o) >= 60.0 - 0.02, "penetrated: {}", p.distance(o));
        assert!(p.x > 10.0 && p.y < 0.0, "should slide down and keep going: {p:?}");
    }

    #[test]
    fn existing_overlap_allows_moving_apart_only() {
        let start = Vec2::new(0.0, 0.0);
        let o = [ob(40.0, 0.0, 25.0)]; // 40 < 60: overlapping
        let away = constrained_move(start, Vec2::new(-10.0, 0.0), 35.0, &o, &[]);
        assert_eq!(away, Vec2::new(-10.0, 0.0));
        let into = constrained_move(start, Vec2::new(10.0, 0.0), 35.0, &o, &[]);
        assert!(into.x <= 1e-4, "{into:?}");
    }

    #[test]
    fn pinned_between_two_does_not_squeeze_through() {
        // Gap between two minions is 40 u, the champion needs 70.
        let o = [ob(60.0, 45.0, 25.0), ob(60.0, -45.0, 25.0)];
        let mut p = Vec2::new(0.0, 0.0);
        for _ in 0..20 {
            p = constrained_move(p, Vec2::new(10.0, 0.0), 35.0, &o, &[]);
        }
        for x in &o {
            assert!(p.distance(x.pos) >= 60.0 - 0.05, "{p:?}");
        }
        assert!(p.x < 60.0, "squeezed through: {p:?}");
    }

    #[test]
    fn detour_goes_around_a_wall_of_units() {
        let wall: Vec<Obstacle> = (-3..=3).map(|i| ob(100.0, i as f32 * 50.0, 25.0)).collect();
        let d = choose_detour(Vec2::new(40.0, 0.0), Vec2::new(400.0, 0.0), 35.0, 120.0, &wall, &[]).unwrap();
        for o in &wall {
            assert!(d.distance(o.pos) >= 60.0);
        }
    }

    #[test]
    fn wall_stops_head_on_and_slides_at_an_angle() {
        let wall = [(Vec2::new(100.0, -500.0), Vec2::new(100.0, 500.0))];
        // Head-on: stops with the circle touching the wall (x = 100 - 35).
        let mut p = Vec2::new(0.0, 0.0);
        for _ in 0..20 {
            p = constrained_move(p, Vec2::new(10.0, 0.0), 35.0, &[], &wall);
        }
        assert!((p.x - 65.0).abs() < 0.05 && p.y.abs() < 1e-3, "{p:?}");
        // At 45°: keeps sliding along the wall.
        let mut q = Vec2::new(0.0, 0.0);
        for _ in 0..20 {
            q = constrained_move(q, Vec2::new(7.0, 7.0), 35.0, &[], &wall);
        }
        assert!(q.x <= 65.0 + 0.05 && q.y > 100.0, "{q:?}");
    }

    #[test]
    fn wall_corner_is_rounded_and_never_penetrated() {
        // Passing beside the end of a wall (20 u off its endpoint): rounds the corner, never clips it.
        let wall = [(Vec2::new(100.0, -500.0), Vec2::new(100.0, 0.0))];
        let mut p = Vec2::new(0.0, 20.0);
        for _ in 0..40 {
            p = constrained_move(p, Vec2::new(8.0, 0.0), 35.0, &[], &wall);
            assert!(crate::map::dist_point_segment(p, wall[0].0, wall[0].1) >= 35.0 - 0.05, "{p:?}");
        }
        assert!(p.x > 100.0, "slid around the corner: {p:?}");
    }

    #[test]
    fn detours_never_go_through_walls() {
        let wall = [(Vec2::new(100.0, -500.0), Vec2::new(100.0, 500.0))];
        if let Some(d) = choose_detour(Vec2::new(60.0, 0.0), Vec2::new(400.0, 0.0), 35.0, 140.0, &[], &wall) {
            assert!(d.x <= 65.0, "{d:?}");
        }
    }

    #[test]
    fn directions_are_unit_length() {
        for d in DIRECTIONS_16 {
            assert!((d.length() - 1.0).abs() < 1e-6);
        }
    }
}
