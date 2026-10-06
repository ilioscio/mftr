//! Abilities (M1 slice 4): the effect shapes the two sandbox kits need, as plain data. The full
//! data-driven effect system of 04 §4 (RON, D8) grows from these once more shapes exist.
//!
//! Every dodge-intended ability declares a [`ReactionClass`] and is checked against the D10
//! reaction budget (03a §8) in tests.

use crate::time::{SUBTICKS_PER_SECOND, SimDuration};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageKind {
    Physical,
    Magic,
    True,
}

/// Raw damage before mitigation: `base + ad_ratio × AD + ap_ratio × AP` (02 §5 step 1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Damage {
    pub kind: DamageKind,
    pub base: f32,
    pub ad_ratio: f32,
    pub ap_ratio: f32,
}

impl Damage {
    pub const NONE: Damage = Damage { kind: DamageKind::Magic, base: 0.0, ad_ratio: 0.0, ap_ratio: 0.0 };

    pub fn raw(&self, attack_damage: f32, ability_power: f32) -> f32 {
        self.base + self.ad_ratio * attack_damage + self.ap_ratio * ability_power
    }
}

/// Crowd control applied on hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Cc {
    None,
    /// No moving, casting or attacking; interrupts casts and attack windups.
    Stun(SimDuration),
    /// No moving or dashing; casting, attacking and blinking still work.
    Root(SimDuration),
}

impl Cc {
    pub fn is_hard(self) -> bool {
        self != Cc::None
    }
}

/// D10 classes: the fraction of max range at which the reaction rule must hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReactionClass {
    HardCc,
    Burst,
    Poke,
    /// Not dodge-intended (self-casts, movement).
    None,
}

impl ReactionClass {
    pub fn range_fraction(self) -> Option<f32> {
        match self {
            ReactionClass::HardCc => Some(0.8),
            ReactionClass::Burst => Some(0.9),
            ReactionClass::Poke => Some(1.0),
            ReactionClass::None => None,
        }
    }
}

/// The D10 reaction check (03a §8): time a centered target needs to react and walk out of a
/// shape of half-width `half_width`, assuming 0.25 s human reaction, 0.17 s network allowance,
/// and a 65 u / 335 u/s champion.
pub fn reaction_needed(half_width: f32) -> f32 {
    0.25 + 0.17 + (half_width + 65.0) / 335.0
}

fn secs(d: SimDuration) -> f32 {
    d.0 as f32 / SUBTICKS_PER_SECOND as f32
}

/// A linear skillshot: windup (caster rooted), then a missile that stops at the first enemy
/// unit it touches, damages it and applies its CC.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineSkillshot {
    pub windup: SimDuration,
    /// Units per second.
    pub speed: f32,
    /// Half the ability's `width`.
    pub radius: f32,
    pub range: f32,
    pub damage: Damage,
    pub cc: Cc,
}

impl LineSkillshot {
    pub fn reaction_needed(&self) -> f32 {
        reaction_needed(self.radius)
    }

    /// Time from cast start until the missile reaches distance `d`.
    pub fn reaction_time(&self, d: f32) -> f32 {
        secs(self.windup) + d / self.speed
    }
}

/// A delayed ground AoE: windup (rooted), then a circle that detonates `delay` later and hits
/// every enemy unit overlapping it at that instant. The telegraph is visible the whole time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DelayedArea {
    pub windup: SimDuration,
    /// Max distance from the caster to the circle's center (farther targets are clamped).
    pub range: f32,
    pub radius: f32,
    pub delay: SimDuration,
    pub damage: Damage,
}

impl DelayedArea {
    /// Walking out of a centered circle takes `(radius + 65) / 335` (03a §8).
    pub fn reaction_needed(&self) -> f32 {
        reaction_needed(self.radius)
    }

    pub fn reaction_time(&self) -> f32 {
        secs(self.windup) + secs(self.delay)
    }
}

/// A dash toward a point at fixed speed: ignores units, slides along walls (D11).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dash {
    pub range: f32,
    pub speed: f32,
}

/// An instant teleport toward a point, landing short of walls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blink {
    pub range: f32,
}

/// A self shield that absorbs damage of any kind until it expires.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shield {
    pub amount: f32,
    pub duration: SimDuration,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    Line(LineSkillshot),
    Area(DelayedArea),
    Dash(Dash),
    Blink(Blink),
    Shield(Shield),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ability {
    pub name: &'static str,
    pub cooldown: SimDuration,
    pub effect: Effect,
    pub reaction: ReactionClass,
}

/// Ability slots on the wire and in the cooldown array: Q W E R, then utility spells D F.
pub const SLOTS: usize = 6;
pub const SLOT_NAMES: [&str; SLOTS] = ["Q", "W", "E", "R", "D", "F"];

/// Utility spell: Blink (01 §10).
pub const BLINK: Ability = Ability {
    name: "Blink",
    cooldown: SimDuration::from_millis(300_000),
    effect: Effect::Blink(Blink { range: 400.0 }),
    reaction: ReactionClass::None,
};

/// Utility spell: Barrier (01 §10; R01 §5 shows it lasting ~2.2 s).
pub const BARRIER: Ability = Ability {
    name: "Barrier",
    cooldown: SimDuration::from_millis(180_000),
    effect: Effect::Shield(Shield { amount: 150.0, duration: SimDuration::from_millis(2500) }),
    reaction: ReactionClass::None,
};

/// The dodge-rig turret's shot (03 §14): a hard-CC line missile with no damage, on a steady
/// cadence, so the rig measures dodging without anyone dying.
pub const TURRET_SHOT: Ability = Ability {
    name: "Turret shot",
    cooldown: SimDuration::from_millis(1200),
    effect: Effect::Line(LineSkillshot {
        windup: SimDuration::from_millis(250),
        speed: 1600.0,
        radius: 35.0,
        range: 1100.0,
        damage: Damage::NONE,
        cc: Cc::Stun(SimDuration::from_millis(750)),
    }),
    reaction: ReactionClass::HardCc,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turret_shot_passes_the_d10_rule_for_hard_cc() {
        let Effect::Line(s) = TURRET_SHOT.effect else { panic!() };
        assert_eq!(s.windup.0, 480);
        assert!(s.reaction_time(0.8 * s.range) >= s.reaction_needed());
    }

    #[test]
    fn damage_ratios() {
        let d = Damage { kind: DamageKind::Physical, base: 40.0, ad_ratio: 1.1, ap_ratio: 0.5 };
        assert!((d.raw(100.0, 20.0) - 160.0).abs() < 1e-4);
    }
}
