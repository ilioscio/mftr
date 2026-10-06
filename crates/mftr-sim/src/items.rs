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
}

const fn b() -> Bonus {
    Bonus::NONE
}

macro_rules! item {
    ($id:expr, $name:expr, $cost:expr, $bonus:expr) => {
        Item { id: $id, name: $name, cost: $cost, bonus: $bonus, passive: Passive::None, recipe: &[], boots: false }
    };
    ($id:expr, $name:expr, $cost:expr, $bonus:expr, $recipe:expr) => {
        Item { id: $id, name: $name, cost: $cost, bonus: $bonus, passive: Passive::None, recipe: $recipe, boots: false }
    };
    ($id:expr, $name:expr, $cost:expr, $bonus:expr, $recipe:expr, $passive:expr) => {
        Item { id: $id, name: $name, cost: $cost, bonus: $bonus, passive: $passive, recipe: $recipe, boots: false }
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

pub const CATALOG: [Item; 26] = [
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
    pub on_hit_magic: Option<(f32, f32)>,
    pub lifeline: Option<(f32, f32, u64, u64)>,
}

pub fn passives(inventory: &[u8; INVENTORY]) -> Passives {
    let mut p = Passives::default();
    for it in inventory.iter().filter_map(|id| item(*id)) {
        match it.passive {
            Passive::OnHitMagic { base, ap_ratio } => p.on_hit_magic = Some((base, ap_ratio)),
            Passive::Lifeline { shield, threshold, duration_ms, cooldown_ms } => {
                p.lifeline = Some((shield, threshold, duration_ms, cooldown_ms))
            }
            Passive::None => {}
        }
    }
    p
}

/// The stat stack (02 §10): level stats, then flat item bonuses, then percent bonuses, then
/// caps. Returns the stats and the attack speed (attacks per second) for `base_attack_speed`.
pub fn apply_items(level_stats: Stats, base_attack_speed: f32, inventory: &[u8; INVENTORY]) -> (Stats, f32) {
    let mut s = level_stats;
    let (mut as_bonus, mut ap_pct, mut ms_flat, mut ms_pct) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut boots_counted = false;
    let mut seen: Vec<u8> = Vec::new();
    for it in inventory.iter().filter_map(|id| item(*id)) {
        let x = &it.bonus;
        s.max_health += x.health;
        s.health_regen += x.health_regen;
        s.armor += x.armor;
        s.magic_resist += x.magic_resist;
        s.attack_damage += x.attack_damage;
        s.ability_power += x.ability_power;
        s.ability_haste += x.ability_haste;
        s.life_steal += x.life_steal;
        as_bonus += x.attack_speed;
        // Only one pair of boots, and a legendary's percent bonus once (unique).
        if !(it.boots && boots_counted) {
            ms_flat += x.move_speed;
        }
        boots_counted |= it.boots;
        if !seen.contains(&it.id) {
            ap_pct += x.ability_power_pct;
            ms_pct += x.move_speed_pct;
        }
        seen.push(it.id);
    }
    s.ability_power *= 1.0 + ap_pct;
    s.move_speed = crate::combat::soft_capped_move_speed((s.move_speed + ms_flat) * (1.0 + ms_pct));
    let attack_speed = (base_attack_speed * (1.0 + as_bonus)).min(ATTACK_SPEED_CAP);
    (s, attack_speed)
}

/// A champion's stats and basic attack at `level` with `inventory`.
pub fn champion_stats(def: &ChampionDef, level: u8, inventory: &[u8; INVENTORY]) -> (Stats, AttackSpec) {
    let (stats, attack_speed) = apply_items(def.stats_at(level), def.attack.attack_speed, inventory);
    (stats, AttackSpec { attack_speed, ..def.attack })
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
        let (s, _) = apply_items(EMBER.stats, EMBER.attack.attack_speed, &inv);
        assert!((s.ability_power - 220.0 * 1.35).abs() < 1e-3, "{}", s.ability_power);
    }

    #[test]
    fn attack_speed_caps_and_boots_dont_stack() {
        let inv = [ARC_TEMPEST, ARC_TEMPEST, GALE_SABER, BATTLE_BOOTS, ARC_BOW, ARC_BOW];
        // +180% attack speed: 0.8 → 2.24; from a 1.0 base it would be 2.8, capped at 2.5.
        let (s, attack_speed) = apply_items(VESPER.stats, VESPER.attack.attack_speed, &inv);
        assert!((attack_speed - 2.24).abs() < 1e-4, "{attack_speed}");
        assert_eq!(apply_items(VESPER.stats, 1.0, &inv).1, ATTACK_SPEED_CAP);
        // One pair of boots (+45) and the Gale Saber's 7%: (325 + 45) × 1.07 = 395.9.
        assert!((s.move_speed - 395.9).abs() < 1e-3, "{}", s.move_speed);
        let two_boots = [SWIFT_BOOTS, BOOTS, 0, 0, 0, 0];
        let (s, _) = apply_items(VESPER.stats, 0.8, &two_boots);
        assert_eq!(s.move_speed, 385.0, "only the first pair counts");
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
