//! Analytic projectiles and exact swept hit tests (`docs/design/03a-netcode-time-and-prediction.md` §6).
//!
//! A linear projectile's position is a closed-form function of its spawn parameters, so the
//! client computes exactly the same positions the server uses. Hits are decided by the closest
//! approach of two linearly moving circles over a time interval, so fast projectiles can't
//! tunnel past a target and update order doesn't matter.

use crate::math::Vec2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearProjectile {
    pub origin: Vec2,
    /// Unit direction.
    pub dir: Vec2,
    /// Units per second.
    pub speed: f32,
    /// Half the ability's `width`.
    pub radius: f32,
    pub range: f32,
    /// Simulation time (seconds) at which the projectile left the caster.
    pub spawn_time: f32,
}

impl LinearProjectile {
    pub fn end_time(&self) -> f32 {
        self.spawn_time + self.range / self.speed
    }

    pub fn is_alive_at(&self, t: f32) -> bool {
        t >= self.spawn_time && t <= self.end_time()
    }

    /// Center position at simulation time `t`, clamped to the projectile's lifetime.
    pub fn position_at(&self, t: f32) -> Vec2 {
        let travel = (t - self.spawn_time).clamp(0.0, self.range / self.speed);
        self.origin + self.dir * (self.speed * travel)
    }

    /// Earliest contact time in `[t0, t1]` with a target moving linearly from `q0` (at `t0`)
    /// to `q1` (at `t1`), with gameplay radius `target_radius`. The projectile only counts
    /// while alive.
    pub fn sweep(&self, t0: f32, t1: f32, q0: Vec2, q1: Vec2, target_radius: f32) -> Option<f32> {
        let a = t0.max(self.spawn_time);
        let b = t1.min(self.end_time());
        if a > b || t1 <= t0 {
            return None;
        }
        // Target positions at the clipped interval ends.
        let span = t1 - t0;
        let qa = q0.lerp(q1, (a - t0) / span);
        let qb = q0.lerp(q1, (b - t0) / span);
        let tau = first_contact(self.position_at(a), self.position_at(b), qa, qb, self.radius + target_radius)?;
        Some(a + (b - a) * tau)
    }
}

/// Earliest normalized time `tau ∈ [0, 1]` at which two points moving linearly
/// (`p0→p1` and `q0→q1` over the same interval) come within distance `r`.
pub fn first_contact(p0: Vec2, p1: Vec2, q0: Vec2, q1: Vec2, r: f32) -> Option<f32> {
    // Relative position d(tau) = a + b * tau.
    let a = p0 - q0;
    let b = (p1 - p0) - (q1 - q0);
    let c = a.length_sq() - r * r;
    if c <= 0.0 {
        return Some(0.0);
    }
    let bb = b.length_sq();
    if bb == 0.0 {
        return None;
    }
    let ab = a.dot(b);
    if ab >= 0.0 {
        return None; // separating or parallel
    }
    let disc = ab * ab - bb * c;
    if disc < 0.0 {
        return None;
    }
    let tau = (-ab - disc.sqrt()) / bb;
    if tau <= 1.0 { Some(tau) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(speed: f32) -> LinearProjectile {
        LinearProjectile {
            origin: Vec2::new(0.0, 0.0),
            dir: Vec2::new(1.0, 0.0),
            speed,
            radius: 35.0,
            range: 1100.0,
            spawn_time: 0.0,
        }
    }

    #[test]
    fn head_on_hit_time_is_exact() {
        // Stationary target 1000 u away, radius 65: contact when the gap is 100 u.
        let p = shot(1600.0);
        let t = p.sweep(0.0, 1.0, Vec2::new(1000.0, 0.0), Vec2::new(1000.0, 0.0), 65.0).unwrap();
        assert!((t - 900.0 / 1600.0).abs() < 1e-5, "{t}");
    }

    #[test]
    fn no_tunneling_through_thin_target_in_one_big_step() {
        // 3000 u/s across a whole 33 ms tick (100 u per tick) past a 10 u target.
        let p = LinearProjectile { radius: 1.0, ..shot(3000.0) };
        let q = Vec2::new(550.0, 0.0);
        let hit = p.sweep(0.166, 0.2, q, q, 10.0);
        assert!(hit.is_some());
    }

    #[test]
    fn walking_out_of_the_corridor_dodges() {
        // Target 800 u down range, starts dead-center, walks perpendicular at 335 u/s.
        // The corridor half-width is 35 + 65 = 100 u, so leaving takes ~0.299 s, and
        // the projectile arrives at ~0.4375 s. Starting the walk at 0.10 s dodges; 0.20 s doesn't.
        let p = shot(1600.0);
        let dodge = |start: f32| {
            let dt = 1.0 / 30.0;
            let mut t = 0.0f32;
            while t < 1.0 {
                let y = |tt: f32| ((tt - start).max(0.0)) * 335.0;
                if let Some(h) = p.sweep(t, t + dt, Vec2::new(800.0, y(t)), Vec2::new(800.0, y(t + dt)), 65.0) {
                    return Some(h);
                }
                t += dt;
            }
            None
        };
        assert!(dodge(0.10).is_none(), "should dodge");
        assert!(dodge(0.20).is_some(), "should be hit");
    }

    #[test]
    fn expired_projectile_cannot_hit() {
        let p = shot(1600.0); // lives 0.6875 s
        let q = Vec2::new(1200.0, 0.0);
        assert!(p.sweep(0.6, 2.0, q, q, 65.0).is_none());
    }

    #[test]
    fn already_overlapping_is_contact_at_start() {
        assert_eq!(first_contact(Vec2::ZERO, Vec2::ZERO, Vec2::new(5.0, 0.0), Vec2::new(5.0, 0.0), 10.0), Some(0.0));
    }
}
