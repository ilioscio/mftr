//! The two placeholder champions of the M1 Duel Sandbox (08 roadmap, slice 4): a skillshot
//! mage and a marksman. Original working names and kits (09 §1). Each kit has a linear
//! skillshot, a delayed ground AoE, a dash or blink, and a hard-CC skillshot, plus a ranged
//! basic attack. All numbers are *(start)* values.
//!
//! Stats are level-1 values that grow with level (02 §1); ability numbers are rank-1 values that
//! grow with rank (M2 slice 2). Sandboxes play everything at level 1, rank 1.

use crate::ability::{
    Ability, BARRIER, BLINK, Blink, Cc, Damage, DamageKind, Dash, DelayedArea, Effect, LineSkillshot, RankScaling,
    ReactionClass,
};
use crate::combat::stat_at_level;
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
    /// Level 1.
    pub stats: Stats,
    /// Added per level on the 02 §1 curve (move speed doesn't grow).
    pub growth: Stats,
    pub attack: AttackSpec,
    pub abilities: [Ability; 4],
}

impl ChampionDef {
    /// Stats at `level` (1–18).
    pub fn stats_at(&self, level: u8) -> Stats {
        let (b, g) = (&self.stats, &self.growth);
        let at = |base: f32, growth: f32| stat_at_level(base, growth, level.max(1));
        Stats {
            max_health: at(b.max_health, g.max_health),
            health_regen: at(b.health_regen, g.health_regen),
            armor: at(b.armor, g.armor),
            magic_resist: at(b.magic_resist, g.magic_resist),
            attack_damage: at(b.attack_damage, g.attack_damage),
            ability_power: at(b.ability_power, g.ability_power),
            move_speed: b.move_speed,
        }
    }
}

/// Highest rank of slot `slot` (0–3) at `level`: basic abilities ⌈level / 2⌉ up to 5, the
/// ultimate 1 / 2 / 3 at levels 6 / 11 / 16.
pub fn max_rank(slot: u8, level: u8) -> u8 {
    if slot == 3 { (level.saturating_sub(1) / 5).min(3) } else { level.div_ceil(2).min(5) }
}

const fn growth(health: f32, armor: f32, magic_resist: f32, attack_damage: f32) -> Stats {
    Stats {
        max_health: health,
        health_regen: 0.08,
        armor,
        magic_resist,
        attack_damage,
        ability_power: 0.0,
        move_speed: 0.0,
    }
}

const fn ranks(damage: f32, cooldown_ms: u64) -> RankScaling {
    RankScaling { damage, cooldown: SimDuration::from_millis(cooldown_ms) }
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
    growth: growth(90.0, 4.2, 1.3, 3.0),
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
            per_rank: ranks(40.0, 500),
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
            per_rank: ranks(40.0, 500),
        },
        Ability {
            name: "Flicker",
            cooldown: ms(14_000),
            effect: Effect::Blink(Blink { range: 350.0 }),
            reaction: ReactionClass::None,
            per_rank: ranks(0.0, 1000),
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
            per_rank: ranks(80.0, 3000),
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
    growth: growth(96.0, 4.4, 1.3, 3.2),
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
            per_rank: ranks(35.0, 400),
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
            per_rank: ranks(30.0, 600),
        },
        Ability {
            name: "Tumble",
            cooldown: ms(7000),
            effect: Effect::Dash(Dash { range: 325.0, speed: 1000.0 }),
            reaction: ReactionClass::None,
            per_rank: ranks(0.0, 600),
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
            per_rank: ranks(60.0, 3000),
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

#[cfg(test)]
mod rank_tests {
    use super::*;

    #[test]
    fn rank_gates_and_growth() {
        assert_eq!((max_rank(0, 1), max_rank(0, 2), max_rank(0, 3), max_rank(0, 9), max_rank(0, 18)), (1, 1, 2, 5, 5));
        assert_eq!(
            (max_rank(3, 5), max_rank(3, 6), max_rank(3, 11), max_rank(3, 16), max_rank(3, 18)),
            (0, 1, 2, 3, 3)
        );
        assert_eq!(EMBER.stats_at(1), EMBER.stats);
        let l18 = VESPER.stats_at(18);
        assert!((l18.max_health - (620.0 + 96.0 * 17.0)).abs() < 1e-3);
        let q = EMBER.abilities[0];
        assert_eq!(q.cooldown_at(5).0, q.cooldown.0 - 4 * SimDuration::from_millis(500).0);
        assert_eq!(q.bonus_damage_at(3), 80.0);
    }
}
