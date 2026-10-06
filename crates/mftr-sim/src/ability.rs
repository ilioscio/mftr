//! Abilities (M1 slice 2): a single data-described line skillshot. The full effect system of
//! 04 §4 grows from this once more ability shapes exist.

use crate::time::SimDuration;

/// A linear skillshot: windup (caster rooted), then a missile that stops at the first enemy
/// unit it touches and stuns it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineSkillshot {
    pub windup: SimDuration,
    pub cooldown: SimDuration,
    /// Units per second.
    pub speed: f32,
    /// Half the ability's `width`.
    pub radius: f32,
    pub range: f32,
    pub stun: SimDuration,
}

impl LineSkillshot {
    /// The D10 reaction check (03a §8): time a centered target needs to react and walk out,
    /// assuming 0.25 s human reaction, 0.17 s network allowance, and a 65 u / 335 u/s champion.
    pub fn reaction_needed(&self) -> f32 {
        0.25 + 0.17 + (self.radius + 65.0) / 335.0
    }

    /// Time from cast start until the missile reaches distance `d`.
    pub fn reaction_time(&self, d: f32) -> f32 {
        self.windup.0 as f32 / 1920.0 + d / self.speed
    }
}

/// Sandbox champion Q: a hard-CC line skillshot. Checked against D10 at 80% range in tests.
pub const SANDBOX_LANCE: LineSkillshot = LineSkillshot {
    windup: SimDuration::from_millis(250),
    cooldown: SimDuration::from_millis(1500),
    speed: 1600.0,
    radius: 35.0,
    range: 1100.0,
    stun: SimDuration::from_millis(750),
};

/// The dodge-rig turret's shot: the same missile on a steady cadence.
pub const TURRET_SHOT: LineSkillshot = LineSkillshot { cooldown: SimDuration::from_millis(1200), ..SANDBOX_LANCE };

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lance_passes_the_d10_rule_for_hard_cc() {
        let s = SANDBOX_LANCE;
        assert_eq!(s.windup.0, 480);
        assert!(
            s.reaction_time(0.8 * s.range) >= s.reaction_needed(),
            "{} < {}",
            s.reaction_time(880.0),
            s.reaction_needed()
        );
    }
}
