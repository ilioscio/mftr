//! Placeholder champions (original working names and kits, 09 §1): the M1 Duel Sandbox's
//! skillshot mage and marksman, M2 slice 4's tank, bruiser, enchanter and assassin, which
//! bring melee attacks, slows, knock-ups, a pull, ally heals and shields, and targeted dashes,
//! and M3 slice 5's artillery mage, warden, battlemage and skirmisher.
//! All numbers are *(start)* values.
//!
//! Stats are level-1 values that grow with level (02 §1); ability numbers are rank-1 values that
//! grow with rank (M2 slice 2). Sandboxes play everything at level 1, rank 1.

use crate::ability::{
    Ability, BARRIER, BLINK, Blink, Cc, Damage, DamageKind, Dash, DelayedArea, Effect, LineSkillshot, Lunge,
    RankScaling, ReactionClass, Support, Transforms,
};
use crate::combat::stat_at_level;
use crate::time::SimDuration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChampionId {
    /// Skillshot mage.
    Ember = 0,
    /// Marksman.
    Vesper = 1,
    /// Tank / engage (melee).
    Bastion = 2,
    /// Bruiser (melee).
    Rook = 3,
    /// Enchanter.
    Lumen = 4,
    /// Assassin (melee).
    Shade = 5,
    /// Artillery mage.
    Quill = 6,
    /// Warden: tank and protector (melee).
    Cairn = 7,
    /// Battlemage: short-range sustained magic.
    Marrow = 8,
    /// Skirmisher: mobile ranged physical damage.
    Wren = 9,
}

impl ChampionId {
    pub const ALL: [ChampionId; 10] = [
        ChampionId::Ember,
        ChampionId::Vesper,
        ChampionId::Bastion,
        ChampionId::Rook,
        ChampionId::Lumen,
        ChampionId::Shade,
        ChampionId::Quill,
        ChampionId::Cairn,
        ChampionId::Marrow,
        ChampionId::Wren,
    ];

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
            ChampionId::Bastion => &BASTION,
            ChampionId::Rook => &ROOK,
            ChampionId::Lumen => &LUMEN,
            ChampionId::Shade => &SHADE,
            ChampionId::Quill => &QUILL,
            ChampionId::Cairn => &CAIRN,
            ChampionId::Marrow => &MARROW,
            ChampionId::Wren => &WREN,
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
    /// Ability haste (items): cooldowns × 100 / (100 + haste) (02 §8).
    pub ability_haste: f32,
    /// Share of basic-attack damage dealt returned as health (items).
    pub life_steal: f32,
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
        ability_haste: 0.0,
        life_steal: 0.0,
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
            ability_haste: 0.0,
            life_steal: 0.0,
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
        ability_haste: 0.0,
        life_steal: 0.0,
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
        ability_haste: 0.0,
        life_steal: 0.0,
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
            transforms: Transforms::LINE,
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
                cc: Cc::None,
            }),
            reaction: ReactionClass::Burst,
            per_rank: ranks(40.0, 500),
            transforms: Transforms::AREA,
        },
        Ability {
            name: "Flicker",
            cooldown: ms(14_000),
            effect: Effect::Blink(Blink { range: 350.0 }),
            reaction: ReactionClass::None,
            per_rank: ranks(0.0, 1000),
            transforms: Transforms::NONE,
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
            transforms: Transforms::LINE,
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
        ability_haste: 0.0,
        life_steal: 0.0,
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
            transforms: Transforms::LINE,
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
                cc: Cc::None,
            }),
            reaction: ReactionClass::Burst,
            per_rank: ranks(30.0, 600),
            transforms: Transforms::AREA,
        },
        Ability {
            name: "Tumble",
            cooldown: ms(7000),
            effect: Effect::Dash(Dash { range: 325.0, speed: 1000.0 }),
            reaction: ReactionClass::None,
            per_rank: ranks(0.0, 600),
            transforms: Transforms::NONE,
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
            transforms: Transforms::LINE,
        },
    ],
};

const fn magic(base: f32, ap_ratio: f32) -> Damage {
    Damage { kind: DamageKind::Magic, base, ad_ratio: 0.0, ap_ratio }
}

const fn physical(base: f32, ad_ratio: f32) -> Damage {
    Damage { kind: DamageKind::Physical, base, ad_ratio, ap_ratio: 0.0 }
}

const fn slow(pct: u8, duration_ms: u64) -> Cc {
    Cc::Slow { pct, duration: ms(duration_ms) }
}

const fn stats(health: f32, regen: f32, armor: f32, magic_resist: f32, ad: f32, ap: f32, speed: f32) -> Stats {
    Stats {
        max_health: health,
        health_regen: regen,
        armor,
        magic_resist,
        attack_damage: ad,
        ability_power: ap,
        move_speed: speed,
        ability_haste: 0.0,
        life_steal: 0.0,
    }
}

/// A melee basic attack: lands at the end of the windup.
const fn melee(range: f32, attack_speed: f32, windup_fraction: f32) -> AttackSpec {
    AttackSpec { range, attack_speed, windup_fraction, bolt_speed: 0.0 }
}

/// A self-centered area: `range` 0, no delay after the windup.
const fn nova(windup_ms: u64, radius: f32, damage: Damage, cc: Cc) -> Effect {
    Effect::Area(DelayedArea { windup: ms(windup_ms), range: 0.0, radius, delay: ms(0), damage, cc })
}

const NO_SUPPORT: Support =
    Support { range: 0.0, heal: 0.0, heal_ap: 0.0, heal_missing: 0.0, shield: 0.0, shield_ap: 0.0, duration: ms(0) };

/// Tank / engage: a pull to start fights, a slowing stomp, a self shield, and a big delayed
/// knock-up to lock a group down.
pub const BASTION: ChampionDef = ChampionDef {
    name: "Bastion",
    stats: stats(680.0, 1.8, 36.0, 32.0, 62.0, 0.0, 335.0),
    growth: growth(110.0, 4.8, 1.8, 3.5),
    attack: melee(150.0, 0.65, 0.3),
    abilities: [
        Ability {
            name: "Grapple",
            cooldown: ms(14_000),
            effect: Effect::Line(LineSkillshot {
                windup: ms(350),
                speed: 1500.0,
                radius: 60.0,
                range: 900.0,
                damage: magic(80.0, 0.5),
                cc: Cc::Pull(150),
            }),
            reaction: ReactionClass::HardCc,
            per_rank: ranks(40.0, 1000),
            transforms: Transforms::WIDE,
        },
        Ability {
            name: "Bulwark",
            cooldown: ms(12_000),
            effect: Effect::Support(Support { shield: 140.0, duration: ms(3000), ..NO_SUPPORT }),
            reaction: ReactionClass::None,
            per_rank: ranks(30.0, 1000),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Tremor",
            cooldown: ms(8000),
            effect: nova(250, 300.0, magic(60.0, 0.4), slow(40, 1500)),
            reaction: ReactionClass::None,
            per_rank: ranks(30.0, 500),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Upheaval",
            cooldown: ms(80_000),
            effect: Effect::Area(DelayedArea {
                windup: ms(250),
                range: 650.0,
                radius: 250.0,
                delay: ms(1150),
                damage: magic(150.0, 0.6),
                cc: Cc::Knockup(ms(1000)),
            }),
            reaction: ReactionClass::HardCc,
            per_rank: ranks(100.0, 15_000),
            transforms: Transforms::AREA,
        },
    ],
};

/// Bruiser: a cleave, a missing-health heal, a slowing lunge, and a slowing shockwave.
pub const ROOK: ChampionDef = ChampionDef {
    name: "Rook",
    stats: stats(650.0, 1.7, 33.0, 32.0, 68.0, 0.0, 340.0),
    growth: growth(102.0, 4.5, 1.6, 3.8),
    attack: melee(175.0, 0.7, 0.28),
    abilities: [
        Ability {
            name: "Cleave",
            cooldown: ms(6000),
            effect: nova(200, 275.0, physical(40.0, 1.0), Cc::None),
            reaction: ReactionClass::None,
            per_rank: ranks(25.0, 500),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Second Wind",
            cooldown: ms(14_000),
            effect: Effect::Support(Support { heal: 40.0, heal_missing: 0.12, ..NO_SUPPORT }),
            reaction: ReactionClass::None,
            per_rank: ranks(20.0, 1000),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Lunge",
            cooldown: ms(10_000),
            effect: Effect::Lunge(Lunge {
                range: 550.0,
                speed: 1500.0,
                damage: physical(50.0, 0.6),
                cc: slow(30, 1000),
            }),
            reaction: ReactionClass::None,
            per_rank: ranks(30.0, 800),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Shockwave",
            cooldown: ms(60_000),
            effect: Effect::Line(LineSkillshot {
                windup: ms(450),
                speed: 1400.0,
                radius: 90.0,
                range: 700.0,
                damage: physical(150.0, 0.8),
                cc: slow(50, 1500),
            }),
            reaction: ReactionClass::Burst,
            per_rank: ranks(100.0, 10_000),
            transforms: Transforms::LINE,
        },
    ],
};

/// Enchanter: heals and shields allies, a slowing poke, and a delayed rooting halo.
pub const LUMEN: ChampionDef = ChampionDef {
    name: "Lumen",
    stats: stats(560.0, 1.3, 22.0, 30.0, 48.0, 60.0, 330.0),
    growth: growth(88.0, 4.0, 1.3, 2.6),
    attack: AttackSpec { range: 550.0, attack_speed: 0.65, windup_fraction: 0.2, bolt_speed: 1500.0 },
    abilities: [
        Ability {
            name: "Mending Light",
            cooldown: ms(10_000),
            effect: Effect::Support(Support { range: 700.0, heal: 70.0, heal_ap: 0.35, ..NO_SUPPORT }),
            reaction: ReactionClass::None,
            per_rank: ranks(25.0, 800),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Aegis",
            cooldown: ms(12_000),
            effect: Effect::Support(Support {
                range: 700.0,
                shield: 80.0,
                shield_ap: 0.4,
                duration: ms(2500),
                ..NO_SUPPORT
            }),
            reaction: ReactionClass::None,
            per_rank: ranks(30.0, 800),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Lull",
            cooldown: ms(8000),
            effect: Effect::Line(LineSkillshot {
                windup: ms(250),
                speed: 1600.0,
                radius: 55.0,
                range: 900.0,
                damage: magic(50.0, 0.4),
                cc: slow(40, 2000),
            }),
            reaction: ReactionClass::Poke,
            per_rank: ranks(30.0, 500),
            transforms: Transforms::LINE,
        },
        Ability {
            name: "Binding Halo",
            cooldown: ms(70_000),
            effect: Effect::Area(DelayedArea {
                windup: ms(250),
                range: 800.0,
                radius: 200.0,
                delay: ms(1000),
                damage: magic(120.0, 0.5),
                cc: Cc::Root(ms(1250)),
            }),
            reaction: ReactionClass::HardCc,
            per_rank: ranks(80.0, 10_000),
            transforms: Transforms::AREA,
        },
    ],
};

/// Assassin: a lunge in, a slowing fan of blades, a quick dash out, and a lunging execution.
pub const SHADE: ChampionDef = ChampionDef {
    name: "Shade",
    stats: stats(590.0, 1.5, 28.0, 32.0, 70.0, 0.0, 345.0),
    growth: growth(95.0, 4.3, 1.5, 3.9),
    attack: melee(125.0, 0.72, 0.25),
    abilities: [
        Ability {
            name: "Shadow Step",
            cooldown: ms(9000),
            effect: Effect::Lunge(Lunge { range: 600.0, speed: 1800.0, damage: physical(60.0, 0.8), cc: Cc::None }),
            reaction: ReactionClass::None,
            per_rank: ranks(30.0, 800),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Fan of Blades",
            cooldown: ms(7000),
            effect: nova(150, 275.0, physical(50.0, 0.6), slow(25, 1000)),
            reaction: ReactionClass::None,
            per_rank: ranks(25.0, 500),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Veil Step",
            cooldown: ms(12_000),
            effect: Effect::Dash(Dash { range: 400.0, speed: 1400.0 }),
            reaction: ReactionClass::None,
            per_rank: ranks(0.0, 1000),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Execution",
            cooldown: ms(60_000),
            effect: Effect::Lunge(Lunge { range: 500.0, speed: 2000.0, damage: physical(150.0, 1.2), cc: Cc::None }),
            reaction: ReactionClass::None,
            per_rank: ranks(100.0, 10_000),
            transforms: Transforms::NONE,
        },
    ],
};

/// A delayed ground area at `range` (0: centered on the caster).
const fn area(windup_ms: u64, range: f32, radius: f32, delay_ms: u64, damage: Damage, cc: Cc) -> Effect {
    Effect::Area(DelayedArea { windup: ms(windup_ms), range, radius, delay: ms(delay_ms), damage, cc })
}

const fn line(windup_ms: u64, speed: f32, radius: f32, range: f32, damage: Damage, cc: Cc) -> Effect {
    Effect::Line(LineSkillshot { windup: ms(windup_ms), speed, radius, range, damage, cc })
}

/// Artillery mage: long-range poke from behind the line, a slowing field, a hop back, and a
/// map-length lance.
pub const QUILL: ChampionDef = ChampionDef {
    name: "Quill",
    stats: stats(540.0, 1.3, 20.0, 30.0, 48.0, 70.0, 325.0),
    growth: growth(86.0, 3.9, 1.3, 2.6),
    attack: AttackSpec { range: 550.0, attack_speed: 0.62, windup_fraction: 0.22, bolt_speed: 1500.0 },
    abilities: [
        Ability {
            name: "Arc Shot",
            cooldown: ms(5000),
            effect: area(250, 1300.0, 120.0, 800, magic(60.0, 0.5), Cc::None),
            reaction: ReactionClass::Poke,
            per_rank: ranks(30.0, 400),
            transforms: Transforms::AREA,
        },
        Ability {
            name: "Static Field",
            cooldown: ms(12_000),
            effect: area(250, 900.0, 220.0, 600, magic(40.0, 0.3), slow(35, 1500)),
            reaction: ReactionClass::None,
            per_rank: ranks(20.0, 800),
            transforms: Transforms::AREA,
        },
        Ability {
            name: "Recoil",
            cooldown: ms(12_000),
            effect: Effect::Dash(Dash { range: 300.0, speed: 1200.0 }),
            reaction: ReactionClass::None,
            per_rank: ranks(0.0, 1000),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Starfall Lance",
            cooldown: ms(80_000),
            effect: line(600, 1600.0, 70.0, 2500.0, magic(180.0, 0.8), Cc::None),
            reaction: ReactionClass::Burst,
            per_rank: ranks(100.0, 15_000),
            transforms: Transforms::LINE,
        },
    ],
};

/// Warden: slows to peel, a shield for an ally, a delayed root, and a delayed knock-up around
/// itself to hold a choke.
pub const CAIRN: ChampionDef = ChampionDef {
    name: "Cairn",
    stats: stats(660.0, 1.8, 34.0, 34.0, 58.0, 20.0, 335.0),
    growth: growth(106.0, 4.7, 1.9, 3.2),
    attack: melee(150.0, 0.62, 0.3),
    abilities: [
        Ability {
            name: "Stone Lash",
            cooldown: ms(7000),
            effect: line(250, 1500.0, 50.0, 800.0, magic(60.0, 0.4), slow(40, 1500)),
            reaction: ReactionClass::Poke,
            per_rank: ranks(30.0, 500),
            transforms: Transforms::LINE,
        },
        Ability {
            name: "Shelter",
            cooldown: ms(12_000),
            effect: Effect::Support(Support {
                range: 700.0,
                shield: 100.0,
                shield_ap: 0.3,
                duration: ms(2500),
                ..NO_SUPPORT
            }),
            reaction: ReactionClass::None,
            per_rank: ranks(30.0, 800),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Rockfall",
            cooldown: ms(14_000),
            effect: area(250, 750.0, 180.0, 1000, magic(50.0, 0.4), Cc::Root(ms(1000))),
            reaction: ReactionClass::HardCc,
            per_rank: ranks(30.0, 1000),
            transforms: Transforms::AREA,
        },
        Ability {
            name: "Monolith",
            cooldown: ms(80_000),
            effect: area(250, 0.0, 325.0, 1350, magic(140.0, 0.5), Cc::Knockup(ms(1000))),
            reaction: ReactionClass::HardCc,
            per_rank: ranks(80.0, 15_000),
            transforms: Transforms::AREA,
        },
    ],
};

/// Battlemage: a draining nova to fight in close, a rooting skillshot, a self heal, and a large
/// slowing area to win extended fights.
pub const MARROW: ChampionDef = ChampionDef {
    name: "Marrow",
    stats: stats(610.0, 1.6, 26.0, 32.0, 52.0, 60.0, 335.0),
    growth: growth(98.0, 4.3, 1.5, 2.8),
    attack: AttackSpec { range: 475.0, attack_speed: 0.65, windup_fraction: 0.22, bolt_speed: 1500.0 },
    abilities: [
        Ability {
            name: "Siphon",
            cooldown: ms(5000),
            effect: nova(200, 300.0, magic(50.0, 0.45), Cc::None),
            reaction: ReactionClass::None,
            per_rank: ranks(25.0, 400),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Grasping Bones",
            cooldown: ms(11_000),
            effect: line(300, 1300.0, 60.0, 850.0, magic(70.0, 0.5), Cc::Root(ms(1000))),
            reaction: ReactionClass::HardCc,
            per_rank: ranks(35.0, 800),
            transforms: Transforms::LINE,
        },
        Ability {
            name: "Grave Pact",
            cooldown: ms(14_000),
            effect: Effect::Support(Support { heal: 50.0, heal_ap: 0.3, ..NO_SUPPORT }),
            reaction: ReactionClass::None,
            per_rank: ranks(25.0, 1000),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Ossuary",
            cooldown: ms(70_000),
            effect: area(250, 600.0, 300.0, 1300, magic(180.0, 0.7), slow(50, 2000)),
            reaction: ReactionClass::Burst,
            per_rank: ranks(100.0, 10_000),
            transforms: Transforms::AREA,
        },
    ],
};

/// Skirmisher: a quick poke, slowing caltrops, a pounce onto a target, and a wide volley of
/// arrows.
pub const WREN: ChampionDef = ChampionDef {
    name: "Wren",
    stats: stats(600.0, 1.4, 27.0, 30.0, 62.0, 0.0, 335.0),
    growth: growth(94.0, 4.3, 1.3, 3.3),
    attack: AttackSpec { range: 500.0, attack_speed: 0.75, windup_fraction: 0.2, bolt_speed: 2000.0 },
    abilities: [
        Ability {
            name: "Ricochet",
            cooldown: ms(5000),
            effect: line(250, 1800.0, 40.0, 950.0, physical(30.0, 0.9), Cc::None),
            reaction: ReactionClass::Poke,
            per_rank: ranks(30.0, 400),
            transforms: Transforms::LINE,
        },
        Ability {
            name: "Caltrops",
            cooldown: ms(12_000),
            effect: area(200, 700.0, 180.0, 400, physical(30.0, 0.4), slow(45, 2000)),
            reaction: ReactionClass::None,
            per_rank: ranks(20.0, 800),
            transforms: Transforms::AREA,
        },
        Ability {
            name: "Pounce",
            cooldown: ms(10_000),
            effect: Effect::Lunge(Lunge { range: 450.0, speed: 1600.0, damage: physical(40.0, 0.5), cc: Cc::None }),
            reaction: ReactionClass::None,
            per_rank: ranks(25.0, 800),
            transforms: Transforms::NONE,
        },
        Ability {
            name: "Hail of Arrows",
            cooldown: ms(60_000),
            effect: area(250, 1000.0, 280.0, 1250, physical(120.0, 0.9), slow(30, 1000)),
            reaction: ReactionClass::Burst,
            per_rank: ranks(80.0, 10_000),
            transforms: Transforms::AREA,
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
                let cc = match a.effect {
                    Effect::Line(s) => s.cc,
                    // Self-centered novas are melee-range tools, not skillshots.
                    Effect::Area(s) if s.range > 0.0 => s.cc,
                    Effect::Lunge(l) => {
                        assert!(!l.cc.is_hard(), "{}: point-and-click hard CC isn't dodgeable", a.name);
                        continue;
                    }
                    _ => continue,
                };
                assert_eq!(cc.is_hard(), a.reaction == ReactionClass::HardCc, "{}", a.name);
            }
        }
    }

    /// Ten archetypes (M2's six, M3's four), each with a distinct kit.
    #[test]
    fn ten_champions_with_unique_names() {
        let names: std::collections::BTreeSet<&str> =
            ChampionId::ALL.iter().flat_map(|c| c.def().abilities.map(|a| a.name)).collect();
        assert_eq!(names.len(), 40);
        for (i, c) in ChampionId::ALL.into_iter().enumerate() {
            assert_eq!(ChampionId::from_u8(i as u8), Some(c));
            assert_eq!(ChampionId::by_name(c.def().name), Some(c));
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
