//! The two placeholder champions of the M1 Duel Sandbox (08 roadmap, slice 4): a skillshot
//! mage and a marksman. Original working names and kits (09 §1). Each kit has a linear
//! skillshot, a delayed ground AoE, a dash or blink, and a hard-CC skillshot, plus a ranged
//! basic attack. All numbers are *(start)* values.
//!
//! The sandbox has no levels, items or resources yet: stats are fixed "mid-game-ish" values.

use crate::ability::{
    Ability, BARRIER, BLINK, Blink, Cc, Damage, DamageKind, Dash, DelayedArea, Effect, LineSkillshot, ReactionClass,
};
use crate::time::SimDuration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChampionId {
    /// Skillshot mage.
    Ember = 0,
    /// Marksman.
    Vesper = 1,
}

impl ChampionId {
    pub const ALL: [ChampionId; 2] = [ChampionId::Ember, ChampionId::Vesper];

    pub fn from_u8(v: u8) -> Option<Self> {
        Self::ALL.get(v as usize).copied()
    }

    pub fn by_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.def().name.eq_ignore_ascii_case(name))
    }

    pub fn def(self) -> &'static ChampionDef {
        match self {
            ChampionId::Ember => &EMBER,
            ChampionId::Vesper => &VESPER,
        }
    }

    /// Ability in `slot` (0–3 = Q W E R, 4 = D Blink, 5 = F Barrier).
    pub fn ability(self, slot: u8) -> Option<Ability> {
        match slot {
            0..=3 => Some(self.def().abilities[slot as usize]),
            4 => Some(BLINK),
            5 => Some(BARRIER),
            _ => None,
        }
    }
}

/// Combat stats (02 §1), fixed per unit for now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub max_health: f32,
    /// Health per second.
    pub health_regen: f32,
    pub armor: f32,
    pub magic_resist: f32,
    pub attack_damage: f32,
    pub ability_power: f32,
    pub move_speed: f32,
}

impl Stats {
    /// Proxies and other units without combat stats.
    pub const NONE: Stats = Stats {
        max_health: 0.0,
        health_regen: 0.0,
        armor: 0.0,
        magic_resist: 0.0,
        attack_damage: 0.0,
        ability_power: 0.0,
        move_speed: 0.0,
    };
}

/// A ranged basic attack (02 §7): windup, then a homing bolt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackSpec {
    /// Center to the target's edge: in range when `distance ≤ range + target gameplay radius`.
    pub range: f32,
    /// Attacks per second.
    pub attack_speed: f32,
    /// Part of the attack timer before the bolt launches.
    pub windup_fraction: f32,
    pub bolt_speed: f32,
}

impl AttackSpec {
    /// Attack timer `1 / AS`, exact on the sub-tick timeline.
    pub fn period(&self) -> SimDuration {
        SimDuration((1920.0 / self.attack_speed).round() as u64)
    }

    pub fn windup(&self) -> SimDuration {
        SimDuration((1920.0 * self.windup_fraction / self.attack_speed).round() as u64)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChampionDef {
    pub name: &'static str,
    pub stats: Stats,
    pub attack: AttackSpec,
    pub abilities: [Ability; 4],
}

const fn ms(v: u64) -> SimDuration {
    SimDuration::from_millis(v)
}

pub const EMBER: ChampionDef = ChampionDef {
    name: "Ember",
    stats: Stats {
        max_health: 600.0,
        health_regen: 1.5,
        armor: 22.0,
        magic_resist: 30.0,
        attack_damage: 50.0,
        ability_power: 80.0,
        move_speed: 325.0,
    },
    attack: AttackSpec { range: 525.0, attack_speed: 0.65, windup_fraction: 0.2, bolt_speed: 1600.0 },
    abilities: [
        Ability {
            name: "Ember Lance",
            cooldown: ms(4000),
            effect: Effect::Line(LineSkillshot {
                windup: ms(250),
                speed: 1600.0,
                radius: 35.0,
                range: 1100.0,
                damage: Damage { kind: DamageKind::Magic, base: 70.0, ad_ratio: 0.0, ap_ratio: 0.6 },
                cc: Cc::None,
            }),
            reaction: ReactionClass::Burst,
        },
        Ability {
            name: "Cinder Bloom",
            cooldown: ms(9000),
            effect: Effect::Area(DelayedArea {
                windup: ms(250),
                range: 900.0,
                radius: 160.0,
                delay: ms(850),
                damage: Damage { kind: DamageKind::Magic, base: 90.0, ad_ratio: 0.0, ap_ratio: 0.7 },
            }),
            reaction: ReactionClass::Burst,
        },
        Ability {
            name: "Flicker",
            cooldown: ms(14_000),
            effect: Effect::Blink(Blink { range: 350.0 }),
            reaction: ReactionClass::None,
        },
        Ability {
            name: "Binding Sigil",
            cooldown: ms(20_000),
            effect: Effect::Line(LineSkillshot {
                windup: ms(300),
                speed: 1300.0,
                radius: 60.0,
                range: 1150.0,
                damage: Damage { kind: DamageKind::Magic, base: 100.0, ad_ratio: 0.0, ap_ratio: 0.5 },
                cc: Cc::Stun(ms(1250)),
            }),
            reaction: ReactionClass::HardCc,
        },
    ],
};

pub const VESPER: ChampionDef = ChampionDef {
    name: "Vesper",
    stats: Stats {
        max_health: 620.0,
        health_regen: 1.4,
        armor: 28.0,
        magic_resist: 30.0,
        attack_damage: 66.0,
        ability_power: 0.0,
        move_speed: 325.0,
    },
    attack: AttackSpec { range: 575.0, attack_speed: 0.8, windup_fraction: 0.18, bolt_speed: 2200.0 },
    abilities: [
        Ability {
            name: "Longshot",
            cooldown: ms(5000),
            effect: Effect::Line(LineSkillshot {
                windup: ms(250),
                speed: 2000.0,
                radius: 30.0,
                range: 1150.0,
                damage: Damage { kind: DamageKind::Physical, base: 40.0, ad_ratio: 1.1, ap_ratio: 0.0 },
                cc: Cc::None,
            }),
            reaction: ReactionClass::Poke,
        },
        Ability {
            name: "Shrapnel Charge",
            cooldown: ms(10_000),
            effect: Effect::Area(DelayedArea {
                windup: ms(250),
                range: 850.0,
                radius: 200.0,
                delay: ms(1000),
                damage: Damage { kind: DamageKind::Physical, base: 50.0, ad_ratio: 0.8, ap_ratio: 0.0 },
            }),
            reaction: ReactionClass::Burst,
        },
        Ability {
            name: "Tumble",
            cooldown: ms(7000),
            effect: Effect::Dash(Dash { range: 325.0, speed: 1000.0 }),
            reaction: ReactionClass::None,
        },
        Ability {
            name: "Snare Net",
            cooldown: ms(20_000),
            effect: Effect::Line(LineSkillshot {
                windup: ms(250),
                speed: 1200.0,
                radius: 70.0,
                range: 1000.0,
                damage: Damage { kind: DamageKind::Physical, base: 60.0, ad_ratio: 0.5, ap_ratio: 0.0 },
                cc: Cc::Root(ms(1500)),
            }),
            reaction: ReactionClass::HardCc,
        },
    ],
};

#[cfg(test)]
mod tests {
    use super::*;

    /// D10 (03a §8): every dodge-intended ability gives a centered target enough time to react
    /// and walk out at its class distance (linear) or before detonation (areas).
    #[test]
    fn every_kit_passes_the_reaction_budget() {
        for c in ChampionId::ALL {
            for a in c.def().abilities {
                let Some(frac) = a.reaction.range_fraction() else { continue };
                let (have, need) = match a.effect {
                    Effect::Line(s) => (s.reaction_time(frac * s.range), s.reaction_needed()),
                    Effect::Area(s) => (s.reaction_time(), s.reaction_needed()),
                    _ => panic!("{}: only skillshots and areas are dodge-intended", a.name),
                };
                assert!(have >= need, "{} {}: {have:.3} s < {need:.3} s", c.def().name, a.name);
            }
        }
    }

    #[test]
    fn hard_cc_abilities_are_classed_hard_cc() {
        for c in ChampionId::ALL {
            for a in c.def().abilities {
                if let Effect::Line(s) = a.effect {
                    assert_eq!(s.cc.is_hard(), a.reaction == ReactionClass::HardCc, "{}", a.name);
                }
            }
        }
    }

    #[test]
    fn attack_timers_are_exact_subticks() {
        let a = VESPER.attack;
        assert_eq!(a.period().0, 2400); // 1 / 0.8 s
        assert_eq!(a.windup().0, 432); // 18% of it
        assert_eq!(ChampionId::by_name("ember"), Some(ChampionId::Ember));
        assert_eq!(ChampionId::from_u8(1), Some(ChampionId::Vesper));
    }
}
