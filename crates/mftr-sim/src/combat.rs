//! Combat formulas from `docs/design/02-combat-math.md`. Every formula has golden tests.

/// Damage multiplier for a (post-penetration) resistance value. Negative resistance amplifies.
pub fn resist_multiplier(r: f32) -> f32 {
    if r >= 0.0 { 100.0 / (100.0 + r) } else { 2.0 - 100.0 / (100.0 - r) }
}

/// Effective health against one damage type (for non-negative resistance).
pub fn effective_health(hp: f32, r: f32) -> f32 {
    hp * (1.0 + r.max(0.0) / 100.0)
}

/// Resistance modifiers of one attacker, applied in the order defined in 02 §4.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Penetration {
    /// Reduces the target's actual stat; may push it below zero.
    pub flat_reduction: f32,
    /// Fraction in `[0, 1]`, applies only to positive resistance.
    pub percent_reduction: f32,
    /// Fraction in `[0, 1]`, attacker-only, never below zero.
    pub percent_pen: f32,
    /// Attacker-only, never below zero. Lethality must already be converted.
    pub flat_pen: f32,
}

/// The target's resistance as seen by this attacker:
/// flat reduction → % reduction → % penetration → flat penetration.
pub fn effective_resist(r: f32, p: &Penetration) -> f32 {
    let mut r = r - p.flat_reduction;
    if r > 0.0 {
        r *= 1.0 - p.percent_reduction;
    }
    if r > 0.0 {
        r *= 1.0 - p.percent_pen;
    }
    if r > 0.0 {
        r = (r - p.flat_pen).max(0.0);
    }
    r
}

/// Lethality converts to flat armor penetration scaling with the attacker's level (1–18).
pub fn lethality_to_flat_pen(lethality: f32, level: u8) -> f32 {
    lethality * (0.6 + 0.4 * level as f32 / 18.0)
}

/// Base stat at `level` (1–18) using the slightly accelerating growth curve.
pub fn stat_at_level(base: f32, growth: f32, level: u8) -> f32 {
    let n = level.saturating_sub(1) as f32;
    base + growth * n * (0.7025 + 0.0175 * n)
}

/// Cooldown after ability haste. 100 haste = twice as many casts.
pub fn cooldown_with_haste(base: f32, haste: f32) -> f32 {
    base * 100.0 / (100.0 + haste)
}

/// CC duration after multiplicatively stacking tenacity sources.
pub fn duration_after_tenacity(duration: f32, tenacities: &[f32]) -> f32 {
    tenacities.iter().fold(duration, |d, t| d * (1.0 - t.clamp(0.0, 1.0)))
}

/// Movement speed soft caps and floor (02 §9).
pub fn soft_capped_move_speed(raw: f32) -> f32 {
    if raw > 490.0 {
        raw * 0.5 + 230.0
    } else if raw > 415.0 {
        raw * 0.8 + 83.0
    } else if raw < 220.0 {
        raw * 0.5 + 110.0
    } else {
        raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-3, "{a} != {b}");
    }

    #[test]
    fn resistance_table() {
        close(resist_multiplier(-20.0), 1.1667);
        close(resist_multiplier(0.0), 1.0);
        close(resist_multiplier(50.0), 0.6667);
        close(resist_multiplier(100.0), 0.5);
        close(resist_multiplier(200.0), 0.3333);
        close(resist_multiplier(300.0), 0.25);
    }

    #[test]
    fn effective_health_is_linear_in_resistance() {
        let gain_0_100 = effective_health(1000.0, 100.0) - effective_health(1000.0, 0.0);
        let gain_200_300 = effective_health(1000.0, 300.0) - effective_health(1000.0, 200.0);
        close(gain_0_100, gain_200_300);
    }

    /// The worked example from 02 §4: 120 armor → 46.6 effective → takes 68.2%.
    #[test]
    fn penetration_worked_example() {
        let p = Penetration { flat_reduction: 10.0, percent_reduction: 0.20, percent_pen: 0.30, flat_pen: 15.0 };
        let r = effective_resist(120.0, &p);
        close(r, 46.6);
        close(resist_multiplier(r), 0.6821);
    }

    #[test]
    fn penetration_never_goes_below_zero() {
        let p = Penetration { flat_pen: 20.0, ..Default::default() };
        close(effective_resist(10.0, &p), 0.0);
    }

    #[test]
    fn reduction_can_go_negative_and_skips_percent_steps() {
        let p = Penetration { flat_reduction: 30.0, percent_reduction: 0.5, percent_pen: 0.5, flat_pen: 10.0 };
        let r = effective_resist(10.0, &p);
        close(r, -20.0);
        close(resist_multiplier(r), 1.1667);
    }

    #[test]
    fn lethality_scales_with_level() {
        close(lethality_to_flat_pen(18.0, 1), 18.0 * (0.6 + 0.4 / 18.0));
        close(lethality_to_flat_pen(18.0, 18), 18.0);
    }

    #[test]
    fn stat_growth_curve() {
        close(stat_at_level(600.0, 100.0, 1), 600.0);
        close(stat_at_level(600.0, 100.0, 2), 600.0 + 100.0 * 0.72);
        close(stat_at_level(600.0, 100.0, 18), 600.0 + 100.0 * 17.0);
    }

    #[test]
    fn haste_and_tenacity() {
        close(cooldown_with_haste(10.0, 100.0), 5.0);
        close(cooldown_with_haste(10.0, 0.0), 10.0);
        close(duration_after_tenacity(2.0, &[0.3, 0.3]), 0.98);
    }

    #[test]
    fn move_speed_caps_are_continuous() {
        close(soft_capped_move_speed(415.0), 415.0);
        close(soft_capped_move_speed(490.0), 475.0);
        close(soft_capped_move_speed(220.0), 220.0);
        close(soft_capped_move_speed(330.0), 330.0);
        close(soft_capped_move_speed(600.0), 530.0);
    }
}
