//! Items (M2 slice 3, 01 §11, 02 §10): a small catalog of components, boots and legendary
//! items with original names, recipes, and the ordered stat stack that turns a champion's level
//! and inventory into its combat stats.
//!
//! Stat stack (02 §10): level stats → flat item bonuses → percent bonuses → caps. Every number is
//! a *(start)* value.

use crate::champion::{AttackSpec, ChampionDef, Stats};

/// Inventory slots (01 §11; the trinket slot comes with vision items in M4).
pub const INVENTORY: usize = 6;
/// Selling refunds this share of the item's total cost.
pub const SELL_REFUND: f32 = 0.7;
/// Attack speed cap (02 §7).
pub const ATTACK_SPEED_CAP: f32 = 2.5;

/// Stat bonuses an item grants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bonus {
    pub health: f32,
    pub health_regen: f32,
    pub armor: f32,
    pub magic_resist: f32,
    pub attack_damage: f32,
    pub ability_power: f32,
    /// Fraction of base attack speed (0.25 = +25%).
    pub attack_speed: f32,
    pub ability_haste: f32,
    pub life_steal: f32,
    /// Flat movement speed.
    pub move_speed: f32,
    /// Fraction of movement speed.
    pub move_speed_pct: f32,
    /// Fraction of total ability power (applied after flat bonuses).
    pub ability_power_pct: f32,
    /// Fractions of total health, attack damage and armor plus magic resist (augments).
    pub health_pct: f32,
    pub attack_damage_pct: f32,
    pub resist_pct: f32,
}

impl Bonus {
    pub const NONE: Bonus = Bonus {
        health: 0.0,
        health_regen: 0.0,
        armor: 0.0,
        magic_resist: 0.0,
        attack_damage: 0.0,
        ability_power: 0.0,
        attack_speed: 0.0,
        ability_haste: 0.0,
        life_steal: 0.0,
        move_speed: 0.0,
        move_speed_pct: 0.0,
        ability_power_pct: 0.0,
        health_pct: 0.0,
        attack_damage_pct: 0.0,
        resist_pct: 0.0,
    };
}

/// Unique passives: mechanics beyond stats (01 §11: unique, they don't stack with themselves).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Passive {
    None,
    /// Basic attacks deal extra magic damage: `base + ap_ratio × AP`.
    OnHitMagic {
        base: f32,
        ap_ratio: f32,
    },
    /// Dropping below `threshold` of max health grants a shield (cooldown in seconds).
    Lifeline {
        shield: f32,
        threshold: f32,
        duration_ms: u64,
        cooldown_ms: u64,
    },
}

/// An item's active (01 §11: items 1–6): drunk with its slot's key.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Active {
    None,
    /// Heals `heal` over `duration_ms`; holds up to `charges` (stacked potions or a flask's
    /// charges). Refillable ones refill whenever their holder can shop; the others are used up.
    Consumable {
        heal: f32,
        duration_ms: u64,
        charges: u8,
        refills: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    pub id: u8,
    pub name: &'static str,
    /// Total cost (components included).
    pub cost: f32,
    pub bonus: Bonus,
    pub passive: Passive,
    /// Items consumed when this one is bought (each found in the inventory saves its cost).
    pub recipe: &'static [u8],
    /// Boots: only one pair counts.
    pub boots: bool,
    pub active: Active,
}

const fn b() -> Bonus {
    Bonus::NONE
}

macro_rules! item {
    ($id:expr, $name:expr, $cost:expr, $bonus:expr) => {
        Item {
            id: $id,
            name: $name,
            cost: $cost,
            bonus: $bonus,
            passive: Passive::None,
            recipe: &[],
            boots: false,
            active: Active::None,
        }
    };
    ($id:expr, $name:expr, $cost:expr, $bonus:expr, $recipe:expr) => {
        Item {
            id: $id,
            name: $name,
            cost: $cost,
            bonus: $bonus,
            passive: Passive::None,
            recipe: $recipe,
            boots: false,
            active: Active::None,
        }
    };
    ($id:expr, $name:expr, $cost:expr, $bonus:expr, $recipe:expr, $passive:expr) => {
        Item {
            id: $id,
            name: $name,
            cost: $cost,
            bonus: $bonus,
            passive: $passive,
            recipe: $recipe,
            boots: false,
            active: Active::None,
        }
    };
}

// Item ids (stable on the wire; 0 = empty slot).
pub const LONG_KNIFE: u8 = 1;
pub const SPARK_SHARD: u8 = 2;
pub const VITAL_CRYSTAL: u8 = 3;
pub const PADDED_VEST: u8 = 4;
pub const WARDING_CLOAK: u8 = 5;
pub const QUICK_DAGGER: u8 = 6;
pub const BOOTS: u8 = 7;
pub const FOCUS_CHARM: u8 = 8;
pub const HEAVY_PICK: u8 = 9;
pub const CHARGED_WAND: u8 = 10;
pub const TITAN_BELT: u8 = 11;
pub const LEECH_FANG: u8 = 12;
pub const ARC_BOW: u8 = 13;
pub const CHAIN_COAT: u8 = 14;
pub const SWIFT_BOOTS: u8 = 15;
pub const BATTLE_BOOTS: u8 = 16;
pub const SAGE_BOOTS: u8 = 17;
pub const INFERNO_DIADEM: u8 = 18;
pub const GRAND_GRIMOIRE: u8 = 19;
pub const CRIMSON_FANG: u8 = 20;
pub const HEARTSTONE: u8 = 21;
pub const BRAMBLE_PLATE: u8 = 22;
pub const WARDSTONE_MANTLE: u8 = 23;
pub const ARC_TEMPEST: u8 = 24;
pub const GALE_SABER: u8 = 25;
pub const LIFELINE_TALISMAN: u8 = 26;
pub const HEALTH_POTION: u8 = 27;
pub const REFILLABLE_FLASK: u8 = 28;

pub const CATALOG: [Item; 28] = [
    // Consumables (the reference game's values): a potion heals 120 over 15 s and up to 5
    // stack in a slot; the flask heals 100 over 12 s, twice, and refills at the fountain.
    Item {
        active: Active::Consumable { heal: 120.0, duration_ms: 15_000, charges: 5, refills: false },
        ..item!(HEALTH_POTION, "Health Potion", 50.0, b())
    },
    Item {
        active: Active::Consumable { heal: 100.0, duration_ms: 12_000, charges: 2, refills: true },
        ..item!(REFILLABLE_FLASK, "Refillable Flask", 150.0, b())
    },
    // Components.
    item!(LONG_KNIFE, "Long Knife", 350.0, Bonus { attack_damage: 10.0, ..b() }),
    item!(SPARK_SHARD, "Spark Shard", 435.0, Bonus { ability_power: 20.0, ..b() }),
    item!(VITAL_CRYSTAL, "Vital Crystal", 400.0, Bonus { health: 150.0, ..b() }),
    item!(PADDED_VEST, "Padded Vest", 300.0, Bonus { armor: 15.0, ..b() }),
    item!(WARDING_CLOAK, "Warding Cloak", 450.0, Bonus { magic_resist: 25.0, ..b() }),
    item!(QUICK_DAGGER, "Quick Dagger", 300.0, Bonus { attack_speed: 0.12, ..b() }),
    Item { boots: true, ..item!(BOOTS, "Boots", 300.0, Bonus { move_speed: 25.0, ..b() }) },
    item!(FOCUS_CHARM, "Focus Charm", 400.0, Bonus { ability_haste: 10.0, ..b() }),
    item!(HEAVY_PICK, "Heavy Pick", 875.0, Bonus { attack_damage: 25.0, ..b() }),
    item!(CHARGED_WAND, "Charged Wand", 850.0, Bonus { ability_power: 40.0, ..b() }),
    item!(TITAN_BELT, "Titan Belt", 900.0, Bonus { health: 350.0, ..b() }, &[VITAL_CRYSTAL]),
    item!(LEECH_FANG, "Leech Fang", 900.0, Bonus { attack_damage: 15.0, life_steal: 0.07, ..b() }, &[LONG_KNIFE]),
    item!(ARC_BOW, "Arc Bow", 700.0, Bonus { attack_speed: 0.25, ..b() }, &[QUICK_DAGGER]),
    item!(CHAIN_COAT, "Chain Coat", 800.0, Bonus { armor: 40.0, ..b() }, &[PADDED_VEST]),
    // Boots.
    Item { boots: true, ..item!(SWIFT_BOOTS, "Swift Boots", 900.0, Bonus { move_speed: 60.0, ..b() }, &[BOOTS]) },
    Item {
        boots: true,
        ..item!(
            BATTLE_BOOTS,
            "Battle Boots",
            1100.0,
            Bonus { move_speed: 45.0, attack_speed: 0.25, ..b() },
            &[BOOTS, QUICK_DAGGER]
        )
    },
    Item {
        boots: true,
        ..item!(SAGE_BOOTS, "Sage Boots", 950.0, Bonus { move_speed: 45.0, ability_haste: 15.0, ..b() }, &[BOOTS])
    },
    // Legendary.
    item!(
        INFERNO_DIADEM,
        "Inferno Diadem",
        2900.0,
        Bonus { ability_power: 80.0, health: 200.0, ability_haste: 15.0, ..b() },
        &[CHARGED_WAND, VITAL_CRYSTAL, FOCUS_CHARM]
    ),
    item!(
        GRAND_GRIMOIRE,
        "Grand Grimoire",
        3600.0,
        Bonus { ability_power: 120.0, ability_power_pct: 0.35, ..b() },
        &[CHARGED_WAND, CHARGED_WAND]
    ),
    item!(
        CRIMSON_FANG,
        "Crimson Fang",
        3000.0,
        Bonus { attack_damage: 55.0, life_steal: 0.18, ..b() },
        &[LEECH_FANG, HEAVY_PICK]
    ),
    item!(
        HEARTSTONE,
        "Heartstone",
        3000.0,
        Bonus { health: 800.0, health_regen: 2.0, ..b() },
        &[TITAN_BELT, VITAL_CRYSTAL]
    ),
    item!(
        BRAMBLE_PLATE,
        "Bramble Plate",
        2700.0,
        Bonus { armor: 70.0, health: 350.0, ..b() },
        &[CHAIN_COAT, VITAL_CRYSTAL]
    ),
    item!(
        WARDSTONE_MANTLE,
        "Wardstone Mantle",
        2800.0,
        Bonus { magic_resist: 60.0, health: 400.0, ability_haste: 10.0, ..b() },
        &[WARDING_CLOAK, VITAL_CRYSTAL, FOCUS_CHARM]
    ),
    item!(
        ARC_TEMPEST,
        "Arc Tempest",
        3000.0,
        Bonus { ability_power: 50.0, attack_speed: 0.40, ..b() },
        &[ARC_BOW, CHARGED_WAND],
        Passive::OnHitMagic { base: 15.0, ap_ratio: 0.15 }
    ),
    item!(
        GALE_SABER,
        "Gale Saber",
        3000.0,
        Bonus { attack_damage: 45.0, attack_speed: 0.25, move_speed_pct: 0.07, ..b() },
        &[HEAVY_PICK, ARC_BOW]
    ),
    item!(
        LIFELINE_TALISMAN,
        "Lifeline Talisman",
        3100.0,
        Bonus { health: 400.0, attack_damage: 40.0, ..b() },
        &[TITAN_BELT, HEAVY_PICK],
        Passive::Lifeline { shield: 250.0, threshold: 0.3, duration_ms: 3000, cooldown_ms: 60_000 }
    ),
];

pub fn item(id: u8) -> Option<&'static Item> {
    CATALOG.iter().find(|i| i.id == id)
}

/// Gold needed to buy `id` with this inventory, and the slots its recipe consumes.
/// What a consumable does, if `id` is one: (heal, duration, most charges, refills).
pub fn consumable(id: u8) -> Option<(f32, u64, u8, bool)> {
    match item(id)?.active {
        Active::Consumable { heal, duration_ms, charges, refills } => Some((heal, duration_ms, charges, refills)),
        Active::None => None,
    }
}

pub fn price(id: u8, inventory: &[u8; INVENTORY]) -> Option<(f32, Vec<usize>)> {
    let it = item(id)?;
    let mut used: Vec<usize> = Vec::new();
    let mut cost = it.cost;
    for &part in it.recipe {
        if let Some(slot) = (0..INVENTORY).find(|s| inventory[*s] == part && !used.contains(s)) {
            used.push(slot);
            cost -= item(part).map_or(0.0, |p| p.cost);
        }
    }
    Some((cost.max(0.0), used))
}

/// Unique passives from an inventory (each counted once).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Passives {
    /// Base, AP ratio, and the item it comes from (death recaps name it).
    pub on_hit_magic: Option<(f32, f32, u8)>,
    pub lifeline: Option<(f32, f32, u64, u64)>,
}

pub fn passives(inventory: &[u8; INVENTORY]) -> Passives {
    let mut p = Passives::default();
    for it in inventory.iter().filter_map(|id| item(*id)) {
        match it.passive {
            Passive::OnHitMagic { base, ap_ratio } => p.on_hit_magic = Some((base, ap_ratio, it.id)),
            Passive::Lifeline { shield, threshold, duration_ms, cooldown_ms } => {
                p.lifeline = Some((shield, threshold, duration_ms, cooldown_ms))
            }
            Passive::None => {}
        }
    }
    p
}

/// Percent bonuses collected on the way through the flat stage.
#[derive(Default)]
struct Percents {
    attack_speed: f32,
    ability_power: f32,
    move_speed_flat: f32,
    move_speed: f32,
    health: f32,
    attack_damage: f32,
    resist: f32,
}

fn add_flat(s: &mut Stats, x: &Bonus) {
    s.max_health += x.health;
    s.health_regen += x.health_regen;
    s.armor += x.armor;
    s.magic_resist += x.magic_resist;
    s.attack_damage += x.attack_damage;
    s.ability_power += x.ability_power;
    s.ability_haste += x.ability_haste;
    s.life_steal += x.life_steal;
}

fn add_percents(p: &mut Percents, x: &Bonus) {
    p.ability_power += x.ability_power_pct;
    p.move_speed += x.move_speed_pct;
    p.health += x.health_pct;
    p.attack_damage += x.attack_damage_pct;
    p.resist += x.resist_pct;
}

/// The stat stack (02 §10): level stats, then flat item and augment bonuses, then augment
/// conversions, then percent bonuses, then caps. Returns the stats and the attack speed
/// (attacks per second) for `base_attack_speed`.
pub fn apply_items(
    level_stats: Stats,
    base_attack_speed: f32,
    inventory: &[u8; INVENTORY],
    augments: &[u8; crate::augments::SLOTS],
    growth: crate::augments::Growth,
) -> (Stats, f32) {
    let form = crate::augments::form(augments, growth.unstable_tiny);
    let mut s = level_stats;
    let mut pct = Percents::default();
    let mut boots_counted = false;
    let mut seen: Vec<u8> = Vec::new();
    for it in inventory.iter().filter_map(|id| item(*id)) {
        let x = &it.bonus;
        add_flat(&mut s, x);
        pct.attack_speed += x.attack_speed;
        // Only one pair of boots, and a legendary's percent bonus once (unique).
        if !(it.boots && boots_counted) {
            pct.move_speed_flat += x.move_speed;
        }
        boots_counted |= it.boots;
        if !seen.contains(&it.id) {
            add_percents(&mut pct, x);
        }
        seen.push(it.id);
    }
    // Stat Anvils (Mayhem): flat stats, attack speed and move speed like an item's.
    for (stat, n) in crate::anvils::STATS.iter().zip(growth.anvil) {
        if n > 0 {
            let x = stat.bonus(n);
            add_flat(&mut s, &x);
            pct.attack_speed += x.attack_speed;
            add_percents(&mut pct, &x);
        }
    }
    for a in crate::augments::held(augments) {
        add_flat(&mut s, &a.bonus);
        pct.attack_speed += a.bonus.attack_speed;
        pct.move_speed_flat += a.bonus.move_speed;
        add_percents(&mut pct, &a.bonus);
        match a.effect {
            crate::augments::Effect::Spellhunger => s.ability_power += growth.stacks as f32,
            crate::augments::Effect::ChampionOfChaos if growth.chaos_done => {
                add_flat(&mut s, &crate::augments::CHAOS_REWARD)
            }
            _ => {}
        }
    }
    for a in crate::augments::held(augments) {
        match a.effect {
            crate::augments::Effect::AdToAp(rate) => {
                let bonus = (s.attack_damage - level_stats.attack_damage).max(0.0);
                s.attack_damage -= bonus;
                s.ability_power += bonus * rate;
            }
            crate::augments::Effect::ApToAd(rate) => {
                let bonus = (s.ability_power - level_stats.ability_power).max(0.0);
                s.ability_power -= bonus;
                s.attack_damage += bonus * rate;
            }
            _ => {}
        }
    }
    if growth.hyper {
        pct.attack_speed += crate::world::HYPER_ATTACK_SPEED;
    }
    // Size forms (Titan, Pebble, Unstable Experiment).
    match form {
        crate::augments::Form::Huge => {
            pct.health += crate::augments::TITAN_HEALTH;
            // Adaptive force: ability power for champions building it, else attack damage.
            let force = crate::augments::TITAN_FORCE;
            if s.ability_power - level_stats.ability_power > s.attack_damage - level_stats.attack_damage {
                s.ability_power += force;
            } else {
                s.attack_damage += force * 0.6;
            }
        }
        crate::augments::Form::Tiny => pct.move_speed += crate::augments::PEBBLE_SPEED,
        crate::augments::Form::Normal => {}
    }
    s.ability_power *= 1.0 + pct.ability_power;
    s.attack_damage *= 1.0 + pct.attack_damage;
    s.max_health *= (1.0 + pct.health).max(0.1);
    s.armor *= 1.0 + pct.resist;
    s.magic_resist *= 1.0 + pct.resist;
    s.move_speed = crate::combat::soft_capped_move_speed((s.move_speed + pct.move_speed_flat) * (1.0 + pct.move_speed));
    let attack_speed = (base_attack_speed * (1.0 + pct.attack_speed)).min(ATTACK_SPEED_CAP);
    (s, attack_speed)
}

/// A champion's stats and basic attack for its level, items and augments.
pub fn champion_stats(def: &ChampionDef, key: &crate::world::StatsKey) -> (Stats, AttackSpec) {
    let (level, inventory, augments, growth) = key;
    let (stats, attack_speed) =
        apply_items(def.stats_at(*level), def.attack.attack_speed, inventory, augments, *growth);
    let mut attack = AttackSpec { attack_speed, ..def.attack };
    let ranged = attack.bolt_speed > 0.0;
    for a in crate::augments::held(augments) {
        match a.effect {
            // Close Quarters: the ranged attack becomes a melee strike.
            crate::augments::Effect::CloseQuarters if ranged => {
                attack.range = crate::augments::CLOSE_QUARTERS_RANGE;
                attack.bolt_speed = 0.0;
            }
            crate::augments::Effect::Sharpshooter if ranged => attack.range += crate::augments::SHARPSHOOTER_RANGE,
            _ => {}
        }
    }
    (stats, attack)
}

/// Bot build paths (components first, so recipes discount them).
pub fn build_path(champion: crate::champion::ChampionId) -> &'static [u8] {
    use crate::champion::ChampionId;
    match champion {
        ChampionId::Ember => &[
            BOOTS,
            CHARGED_WAND,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            INFERNO_DIADEM,
            SAGE_BOOTS,
            CHARGED_WAND,
            CHARGED_WAND,
            GRAND_GRIMOIRE,
            WARDING_CLOAK,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            WARDSTONE_MANTLE,
        ],
        ChampionId::Vesper => &[
            BOOTS,
            LONG_KNIFE,
            LEECH_FANG,
            HEAVY_PICK,
            CRIMSON_FANG,
            QUICK_DAGGER,
            BATTLE_BOOTS,
            HEAVY_PICK,
            QUICK_DAGGER,
            ARC_BOW,
            GALE_SABER,
            VITAL_CRYSTAL,
            TITAN_BELT,
            HEAVY_PICK,
            LIFELINE_TALISMAN,
        ],
        ChampionId::Bastion => &[
            BOOTS,
            VITAL_CRYSTAL,
            PADDED_VEST,
            CHAIN_COAT,
            BRAMBLE_PLATE,
            SWIFT_BOOTS,
            VITAL_CRYSTAL,
            TITAN_BELT,
            VITAL_CRYSTAL,
            HEARTSTONE,
            WARDING_CLOAK,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            WARDSTONE_MANTLE,
        ],
        ChampionId::Rook => &[
            BOOTS,
            LONG_KNIFE,
            LEECH_FANG,
            VITAL_CRYSTAL,
            TITAN_BELT,
            HEAVY_PICK,
            LIFELINE_TALISMAN,
            QUICK_DAGGER,
            BATTLE_BOOTS,
            PADDED_VEST,
            CHAIN_COAT,
            VITAL_CRYSTAL,
            BRAMBLE_PLATE,
        ],
        ChampionId::Lumen => &[
            BOOTS,
            CHARGED_WAND,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            INFERNO_DIADEM,
            SAGE_BOOTS,
            WARDING_CLOAK,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            WARDSTONE_MANTLE,
        ],
        ChampionId::Shade => &[
            BOOTS,
            LONG_KNIFE,
            LEECH_FANG,
            HEAVY_PICK,
            CRIMSON_FANG,
            SWIFT_BOOTS,
            HEAVY_PICK,
            QUICK_DAGGER,
            ARC_BOW,
            GALE_SABER,
        ],
        ChampionId::Quill | ChampionId::Marrow => &[
            BOOTS,
            CHARGED_WAND,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            INFERNO_DIADEM,
            SAGE_BOOTS,
            CHARGED_WAND,
            CHARGED_WAND,
            GRAND_GRIMOIRE,
            WARDING_CLOAK,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            WARDSTONE_MANTLE,
        ],
        ChampionId::Cairn => &[
            BOOTS,
            VITAL_CRYSTAL,
            PADDED_VEST,
            CHAIN_COAT,
            BRAMBLE_PLATE,
            SWIFT_BOOTS,
            WARDING_CLOAK,
            VITAL_CRYSTAL,
            FOCUS_CHARM,
            WARDSTONE_MANTLE,
            VITAL_CRYSTAL,
            TITAN_BELT,
        ],
        ChampionId::Wren => &[
            BOOTS,
            LONG_KNIFE,
            LEECH_FANG,
            HEAVY_PICK,
            CRIMSON_FANG,
            QUICK_DAGGER,
            BATTLE_BOOTS,
            HEAVY_PICK,
            QUICK_DAGGER,
            ARC_BOW,
            GALE_SABER,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::champion::{EMBER, VESPER};

    #[test]
    fn catalog_is_consistent() {
        for (i, it) in CATALOG.iter().enumerate() {
            assert_eq!(item(it.id), Some(it), "id {} at {i}", it.id);
            let parts: f32 = it.recipe.iter().map(|p| item(*p).unwrap().cost).sum();
            assert!(parts < it.cost, "{}: components cost more than the item", it.name);
        }
        assert!(CATALOG.len() >= 25);
    }

    /// 02 §10 golden case: flat AP first, then the Grand Grimoire's +35%, so 80 + 20 + 120
    /// = 220 AP becomes 297.
    #[test]
    fn stat_stack_applies_flat_then_percent() {
        let inv = [SPARK_SHARD, GRAND_GRIMOIRE, 0, 0, 0, 0];
        let (s, _) = apply_items(EMBER.stats, EMBER.attack.attack_speed, &inv, &[0; 4], crate::augments::Growth::NONE);
        assert!((s.ability_power - 220.0 * 1.35).abs() < 1e-3, "{}", s.ability_power);
    }

    #[test]
    fn attack_speed_caps_and_boots_dont_stack() {
        let inv = [ARC_TEMPEST, ARC_TEMPEST, GALE_SABER, BATTLE_BOOTS, ARC_BOW, ARC_BOW];
        // +180% attack speed: 0.8 → 2.24; from a 1.0 base it would be 2.8, capped at 2.5.
        let (s, attack_speed) =
            apply_items(VESPER.stats, VESPER.attack.attack_speed, &inv, &[0; 4], crate::augments::Growth::NONE);
        assert!((attack_speed - 2.24).abs() < 1e-4, "{attack_speed}");
        assert_eq!(apply_items(VESPER.stats, 1.0, &inv, &[0; 4], crate::augments::Growth::NONE).1, ATTACK_SPEED_CAP);
        // One pair of boots (+45) and the Gale Saber's 7%: (325 + 45) × 1.07 = 395.9.
        assert!((s.move_speed - 395.9).abs() < 1e-3, "{}", s.move_speed);
        let two_boots = [SWIFT_BOOTS, BOOTS, 0, 0, 0, 0];
        let (s, _) = apply_items(VESPER.stats, 0.8, &two_boots, &[0; 4], crate::augments::Growth::NONE);
        assert_eq!(s.move_speed, 385.0, "only the first pair counts");
    }

    /// Augments join the stack: flat with the items, then conversions, then percents.
    #[test]
    fn augments_add_flat_then_convert_then_scale() {
        // Whetstone (+20 AD) and a Long Knife (+10 AD), then Conversion: the 30 bonus AD
        // becomes 33 AP.
        let (s, _) =
            apply_items(VESPER.stats, 0.8, &[LONG_KNIFE, 0, 0, 0, 0, 0], &[2, 9, 0, 0], crate::augments::Growth::NONE);
        assert!((s.attack_damage - VESPER.stats.attack_damage).abs() < 1e-4);
        assert!((s.ability_power - (VESPER.stats.ability_power + 33.0)).abs() < 1e-3, "{}", s.ability_power);
        // Apex Form: +25% health, AD, AP, armor and magic resist, after flat bonuses.
        let (s, _) = apply_items(
            EMBER.stats,
            0.8,
            &[VITAL_CRYSTAL, 0, 0, 0, 0, 0],
            &[17, 0, 0, 0],
            crate::augments::Growth::NONE,
        );
        assert!((s.max_health - (EMBER.stats.max_health + 150.0) * 1.25).abs() < 1e-2);
        assert!((s.armor - EMBER.stats.armor * 1.25).abs() < 1e-4);
        // Swift Hands adds to the item attack speed bonus: 0.8 × (1 + 0.12 + 0.30).
        let (_, attack_speed) = apply_items(
            VESPER.stats,
            0.8,
            &[QUICK_DAGGER, 0, 0, 0, 0, 0],
            &[6, 0, 0, 0],
            crate::augments::Growth::NONE,
        );
        assert!((attack_speed - 0.8 * 1.42).abs() < 1e-5, "{attack_speed}");
    }

    #[test]
    fn recipes_discount_components_in_the_inventory() {
        let inv = [CHARGED_WAND, VITAL_CRYSTAL, 0, 0, 0, 0];
        let (cost, used) = price(INFERNO_DIADEM, &inv).unwrap();
        assert_eq!(cost, 2900.0 - 850.0 - 400.0);
        assert_eq!(used, vec![0, 1]);
        let (cost, _) = price(GRAND_GRIMOIRE, &[CHARGED_WAND, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(cost, 3600.0 - 850.0, "one of two wands");
    }
}
