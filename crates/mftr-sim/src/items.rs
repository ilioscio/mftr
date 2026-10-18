//! Items (18, 01 §11, 02 §10): the catalog in tiers (starters, components, epics,
//! legendaries, boots, consumables, the support quest and the jungle hatchlings), with
//! original names, recipes, unique passives and actives, and the ordered stat stack that turns
//! a champion's level and inventory into its combat stats.
//!
//! Stat stack (02 §10): level stats → flat item bonuses → percent bonuses → caps. Every number is
//! a *(start)* value. What the passives and actives do in a match is in `gear.rs`.

use crate::champion::{AttackSpec, ChampionDef, Stats};
use crate::time::SimDuration;

/// Inventory slots (01 §11; the trinket has its own).
pub const INVENTORY: usize = 6;
/// Selling refunds this share of the item's total cost.
pub const SELL_REFUND: f32 = 0.7;
/// Attack speed cap (02 §7).
pub const ATTACK_SPEED_CAP: f32 = 2.5;
/// A critical strike's damage before bonus crit damage (02 §1).
pub const CRIT_DAMAGE: f32 = 1.75;

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
    /// 18 §3: crit chance and bonus crit damage, lethality, percent armor penetration, flat and
    /// percent magic penetration, omnivamp, tenacity, slow resistance, heal and shield power,
    /// healing received.
    pub crit_chance: f32,
    pub crit_damage: f32,
    pub lethality: f32,
    pub armor_pen: f32,
    pub magic_pen: f32,
    pub magic_pen_pct: f32,
    pub omnivamp: f32,
    pub tenacity: f32,
    pub slow_resist: f32,
    pub heal_power: f32,
    pub heal_received: f32,
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
        crit_chance: 0.0,
        crit_damage: 0.0,
        lethality: 0.0,
        armor_pen: 0.0,
        magic_pen: 0.0,
        magic_pen_pct: 0.0,
        omnivamp: 0.0,
        tenacity: 0.0,
        slow_resist: 0.0,
        heal_power: 0.0,
        heal_received: 0.0,
    };
}

/// Where an item sits in the shop (18 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Starter,
    Component,
    Epic,
    Legendary,
    Boots,
    Consumable,
    /// The Pilgrim's quest and its lanterns (18 §6).
    Support,
    /// The hatchlings (18 §7).
    Jungle,
}

impl Tier {
    pub fn name(self) -> &'static str {
        match self {
            Tier::Starter => "Starter",
            Tier::Component => "Component",
            Tier::Epic => "Epic",
            Tier::Legendary => "Legendary",
            Tier::Boots => "Boots",
            Tier::Consumable => "Consumable",
            Tier::Support => "Support",
            Tier::Jungle => "Jungle",
        }
    }

    /// Wire and UI index (the shop's columns).
    pub fn index(self) -> u8 {
        self as u8
    }
}

/// A jungle hatchling's kind (18 §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hatchling {
    /// Grown: Claiming grants a burst of speed.
    Windling,
    /// Grown: Claiming, or a while out of champion combat, shields.
    Barkling,
    /// Grown: Claim can strike enemy champions.
    Flintling,
}

/// Unique passives (18 §4): mechanics beyond stats. Unique: two items with the same kind give
/// it once, the stronger.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Passive {
    None,
    /// Basic attacks deal extra magic damage: `base + ap_ratio × AP`.
    OnHitMagic {
        base: f32,
        ap_ratio: f32,
    },
    /// Every third basic attack deals `base + bonus_ad × bonus AD` extra physical damage.
    ThirdHit {
        base: f32,
        bonus_ad: f32,
    },
    /// After an ability, the next basic attack deals `base_ad × base AD` extra physical damage.
    Spellblade {
        base_ad: f32,
    },
    /// Basic attacks also hit enemies within `radius` of the target for a share of AD.
    Cleave {
        melee: f32,
        ranged: f32,
        radius: f32,
    },
    /// Ranged basic attacks also fire at up to `targets` enemies near the target, for `share`.
    Fanfire {
        targets: u8,
        share: f32,
        radius: f32,
    },
    /// Basic attacks grant `speed` movement speed for a while.
    Quicken {
        speed: f32,
        ms: u64,
    },
    /// Physical damage to a champion reduces its armor by `per_stack` a stack.
    Sunder {
        per_stack: f32,
        stacks: u8,
        ms: u64,
    },
    /// Hits on champions reduce their armor and magic resist.
    Expose {
        pct: f32,
        ms: u64,
    },
    /// Damage to champions of these kinds applies Wounds.
    Wounds {
        pct: f32,
        physical: bool,
        magic: bool,
    },
    /// Basic attacks on the holder hurt the attacker and apply Wounds.
    Thorns {
        base: f32,
        bonus_armor: f32,
        wounds: f32,
    },
    /// Ability damage to champions burns `max_hp` of their max health a second.
    Searing {
        max_hp: f32,
        ms: u64,
    },
    /// In combat, burn nearby enemies every second.
    Smolder {
        base: f32,
        bonus_hp: f32,
        radius: f32,
    },
    /// Two separate hits on a champion within `ambush::WINDOW`: % max health and a shield.
    Ambush {
        max_hp: f32,
        shield: f32,
        bonus_ad: f32,
        cooldown_ms: u64,
    },
    /// Dropping below `threshold` health grants a shield.
    Rally {
        shield: f32,
        bonus_hp: f32,
        threshold: f32,
        duration_ms: u64,
        cooldown_ms: u64,
    },
    /// Life steal past full health becomes a shield.
    Overflow {
        base: f32,
        per_level: f32,
        ms: u64,
    },
    /// After `after_ms` without taking damage, regenerate `max_hp` of max health a second.
    Warmth {
        max_hp: f32,
        after_ms: u64,
    },
    /// Up to `amp` more damage to targets with more max health (full at `full_at` more).
    Topple {
        amp: f32,
        full_at: f32,
    },
    /// Movement speed after `after_ms` out of champion combat.
    LightFeet {
        speed: f32,
        after_ms: u64,
    },
    /// Critical strikes deal this much less to the holder.
    CritCut(f32),
    /// Basic attacks deal this much less to the holder.
    AttackCut(f32),
    /// The holder's heals and shields on allies make their attacks deal extra magic damage.
    Kindle {
        magic: f32,
        ms: u64,
    },
    /// Immobilizing an enemy champion shields allies nearby.
    Bulwark {
        shield: f32,
        bonus_hp: f32,
        radius: f32,
        cooldown_ms: u64,
    },
    /// Takedowns stack ability power.
    Tally {
        ap: f32,
        most: u8,
        kill: u8,
        assist: u8,
        death: u8,
    },
    /// The support quest: Tithe and Spoils (18 §6).
    Quest,
    /// A jungle hatchling (18 §7).
    Hatchling(Hatchling),
}

/// Kinds of unique passives (one of each counts).
pub const PASSIVE_KINDS: usize = 26;

impl Passive {
    /// Its kind (an index below `PASSIVE_KINDS`), `None` for no passive.
    pub fn kind(&self) -> Option<usize> {
        Some(match self {
            Passive::None => return None,
            Passive::OnHitMagic { .. } => 0,
            Passive::ThirdHit { .. } => 1,
            Passive::Spellblade { .. } => 2,
            Passive::Cleave { .. } => 3,
            Passive::Fanfire { .. } => 4,
            Passive::Quicken { .. } => 5,
            Passive::Sunder { .. } => 6,
            Passive::Expose { .. } => 7,
            Passive::Wounds { physical: true, magic: false, .. } => 8,
            Passive::Wounds { .. } => 9,
            Passive::Thorns { .. } => 10,
            Passive::Searing { .. } => 11,
            Passive::Smolder { .. } => 12,
            Passive::Ambush { .. } => 13,
            Passive::Rally { .. } => 14,
            Passive::Overflow { .. } => 15,
            Passive::Warmth { .. } => 16,
            Passive::Topple { .. } => 17,
            Passive::LightFeet { .. } => 18,
            Passive::CritCut(_) => 19,
            Passive::AttackCut(_) => 20,
            Passive::Kindle { .. } => 21,
            Passive::Bulwark { .. } => 22,
            Passive::Tally { .. } => 23,
            Passive::Quest => 24,
            Passive::Hatchling(_) => 25,
        })
    }

    /// How strong it is against another of its kind (the stronger one counts).
    fn strength(&self) -> f32 {
        match *self {
            Passive::OnHitMagic { base, ap_ratio } => base + 100.0 * ap_ratio,
            Passive::ThirdHit { base, bonus_ad } => base + 100.0 * bonus_ad,
            Passive::Spellblade { base_ad } => base_ad,
            Passive::Cleave { melee, .. } => melee,
            Passive::Fanfire { share, targets, .. } => share * targets as f32,
            Passive::Quicken { speed, .. } => speed,
            Passive::Sunder { per_stack, stacks, .. } => per_stack * stacks as f32,
            Passive::Expose { pct, .. } => pct,
            Passive::Wounds { pct, .. } => pct,
            Passive::Thorns { base, .. } => base,
            Passive::Searing { max_hp, .. } => max_hp,
            Passive::Smolder { base, .. } => base,
            Passive::Ambush { max_hp, .. } => max_hp,
            Passive::Rally { shield, .. } => shield,
            Passive::Overflow { base, .. } => base,
            Passive::Warmth { max_hp, .. } => max_hp,
            Passive::Topple { amp, .. } => amp,
            Passive::LightFeet { speed, .. } => speed,
            Passive::CritCut(c) | Passive::AttackCut(c) => c,
            Passive::Kindle { magic, .. } => magic,
            Passive::Bulwark { shield, .. } => shield,
            Passive::Tally { ap, .. } => ap,
            Passive::Quest | Passive::Hatchling(_) | Passive::None => 0.0,
        }
    }
}

/// An item's active (18 §4): used with its slot's key (1–6), quick-cast toward the cursor.
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
    /// Placed at the cursor as a control ward (01 §8); up to `charges` stack in a slot.
    ControlWard {
        charges: u8,
    },
    /// Places a stealth ward at the cursor; `charges` refill at the fountain.
    StealthWard {
        charges: u8,
    },
    /// Physical damage around the holder, and the attack timer resets.
    ReapingArc {
        ad: f32,
        radius: f32,
        cooldown_ms: u64,
    },
    /// Physical damage around the holder; heals a share of missing health per champion hit.
    Bloodletting {
        ad: f32,
        radius: f32,
        heal_missing: f32,
        cooldown_ms: u64,
    },
    /// Movement speed for a while.
    Fleet {
        pct: u8,
        ms: u64,
        cooldown_ms: u64,
    },
    /// Dash to the enemy champion nearest the cursor; damage, then more of the holder's damage
    /// on it for a while.
    Plunge {
        range: f32,
        base: f32,
        bonus_ad: f32,
        amp: f32,
        amp_ms: u64,
        cooldown_ms: u64,
    },
    /// Stasis: invulnerable, untargetable and unable to act.
    Encase {
        ms: u64,
        cooldown_ms: u64,
    },
    /// Dash toward the cursor and fire a fan of bolts, 10° apart (up to 9).
    Surge {
        dash: f32,
        bolts: u8,
        base: f32,
        ap_ratio: f32,
        cooldown_ms: u64,
    },
    /// Slow enemies around the holder.
    Quake {
        radius: f32,
        slow: u8,
        ms: u64,
        cooldown_ms: u64,
    },
    /// A shield of `base + bonus_hp × bonus health`; the holder deals `cut` less meanwhile.
    Petrify {
        base: f32,
        bonus_hp: f32,
        ms: u64,
        cut: f32,
        cooldown_ms: u64,
    },
    /// Shield the holder and allied champions around it.
    Embrace {
        base: f32,
        per_level: f32,
        radius: f32,
        ms: u64,
        cooldown_ms: u64,
    },
    /// Cleanse, heal and speed up the ally nearest the cursor (or the holder).
    Absolve {
        range: f32,
        heal: f32,
        heal_missing: f32,
        speed: u8,
        ms: u64,
        cooldown_ms: u64,
    },
}

impl Active {
    /// The active's cooldown (none for consumables and wards: they use charges).
    pub fn cooldown(&self) -> Option<SimDuration> {
        let ms = match *self {
            Active::ReapingArc { cooldown_ms, .. }
            | Active::Bloodletting { cooldown_ms, .. }
            | Active::Fleet { cooldown_ms, .. }
            | Active::Plunge { cooldown_ms, .. }
            | Active::Encase { cooldown_ms, .. }
            | Active::Surge { cooldown_ms, .. }
            | Active::Quake { cooldown_ms, .. }
            | Active::Petrify { cooldown_ms, .. }
            | Active::Embrace { cooldown_ms, .. }
            | Active::Absolve { cooldown_ms, .. } => cooldown_ms,
            Active::None | Active::Consumable { .. } | Active::ControlWard { .. } | Active::StealthWard { .. } => {
                return None;
            }
        };
        Some(SimDuration::from_millis(ms))
    }

    /// Used at a point (the cursor): wards, a dash, a targeted ally or enemy.
    pub fn aimed(&self) -> bool {
        matches!(
            self,
            Active::ControlWard { .. }
                | Active::StealthWard { .. }
                | Active::Plunge { .. }
                | Active::Surge { .. }
                | Active::Absolve { .. }
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Item {
    pub id: u8,
    pub name: &'static str,
    /// Total cost (components included).
    pub cost: f32,
    pub tier: Tier,
    pub bonus: Bonus,
    /// Up to two unique passives.
    pub passives: [Passive; 2],
    /// Items consumed when this one is bought (each found in the inventory saves its cost).
    pub recipe: &'static [u8],
    pub active: Active,
}

impl Item {
    pub fn boots(&self) -> bool {
        self.tier == Tier::Boots
    }
}

const fn b() -> Bonus {
    Bonus::NONE
}

const NO: [Passive; 2] = [Passive::None, Passive::None];

const fn it(id: u8, name: &'static str, cost: f32, tier: Tier, bonus: Bonus, recipe: &'static [u8]) -> Item {
    Item { id, name, cost, tier, bonus, passives: NO, recipe, active: Active::None }
}

const fn one(p: Passive) -> [Passive; 2] {
    [p, Passive::None]
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
pub const BRIARHIDE_PLATE: u8 = 22;
pub const WARDSTONE_MANTLE: u8 = 23;
pub const ARC_TEMPEST: u8 = 24;
pub const GALE_SABER: u8 = 25;
pub const LIONHEART_TALISMAN: u8 = 26;
pub const HEALTH_POTION: u8 = 27;
pub const REFILLABLE_FLASK: u8 = 28;
pub const CONTROL_WARD: u8 = 29;
pub const SQUIRES_BLADE: u8 = 30;
pub const SQUIRES_RING: u8 = 31;
pub const SQUIRES_BUCKLER: u8 = 32;
pub const TALLY_CHARM: u8 = 33;
pub const PILGRIMS_LEDGER: u8 = 34;
pub const PILGRIMS_ASTROLABE: u8 = 35;
pub const SUNWARD_LANTERN: u8 = 36;
pub const BULWARK_LANTERN: u8 = 37;
pub const HUNTERS_LANTERN: u8 = 38;
pub const WINDLING: u8 = 39;
pub const IRONSHOD_BOOTS: u8 = 40;
pub const STALWART_SABATONS: u8 = 41;
pub const SPELLSTRIDERS: u8 = 42;
pub const FORTUNES_DIE: u8 = 43;
pub const GREATSWORD: u8 = 44;
pub const ELDER_STAFF: u8 = 45;
pub const SPELLWARD_MANTLE: u8 = 46;
pub const RUNED_HILT: u8 = 47;
pub const SPLITTING_AXE: u8 = 48;
pub const WHETTED_KRIS: u8 = 49;
pub const GALE_CLOAK: u8 = 50;
pub const GLOWING_GARNET: u8 = 51;
pub const BARBED_HOOK: u8 = 52;
pub const BLIGHT_ORB: u8 = 53;
pub const RAGE_KNOT: u8 = 54;
pub const THREEFOLD_BRAND: u8 = 55;
pub const SUNDERING_AXE: u8 = 56;
pub const REAPING_CLEAVER: u8 = 57;
pub const BLOODLETTERS_MAUL: u8 = 58;
pub const VEILRUNNER_DAGGER: u8 = 59;
pub const DUSKFANG: u8 = 60;
pub const SHRIKEBLADE: u8 = 61;
pub const STARSTEEL_BLADE: u8 = 62;
pub const FANFIRE_QUIVER: u8 = 63;
pub const TITANBANE_BOW: u8 = 64;
pub const AMBER_RELIQUARY: u8 = 65;
pub const HOLLOW_SCEPTER: u8 = 66;
pub const WINDSTRIDE_BELT: u8 = 67;
pub const BLIGHTCALLER_TOME: u8 = 68;
pub const FURNACE_PLATE: u8 = 69;
pub const STONEWALL_AEGIS: u8 = 70;
pub const STONESKIN_WARD: u8 = 71;
pub const KINSHIP_LOCKET: u8 = 72;
pub const PURIFYING_CHALICE: u8 = 73;
pub const BARKLING: u8 = 74;
pub const FLINTLING: u8 = 75;

use Tier::*;

/// The Spellblade passive's numbers.
const SPELLBLADE: f32 = 1.0;
const QUICKEN: Passive = Passive::Quicken { speed: 20.0, ms: 2000 };
const MONSTER_FEED: Passive = Passive::None;

pub static CATALOG: [Item; 75] = [
    // ---- Starters (one at a time).
    it(
        SQUIRES_BLADE,
        "Squire's Blade",
        450.0,
        Starter,
        Bonus { attack_damage: 8.0, health: 80.0, omnivamp: 0.03, ..b() },
        &[],
    ),
    it(
        SQUIRES_RING,
        "Squire's Ring",
        400.0,
        Starter,
        Bonus { ability_power: 16.0, health: 60.0, health_regen: 0.5, ..b() },
        &[],
    ),
    it(
        SQUIRES_BUCKLER,
        "Squire's Buckler",
        450.0,
        Starter,
        Bonus { health: 130.0, armor: 6.0, health_regen: 1.5, ..b() },
        &[],
    ),
    Item {
        passives: one(Passive::Tally { ap: 3.0, most: 10, kill: 2, assist: 1, death: 4 }),
        ..it(TALLY_CHARM, "Tally Charm", 350.0, Starter, Bonus { ability_power: 15.0, health: 40.0, ..b() }, &[])
    },
    // ---- Consumables: a potion heals 120 over 15 s, up to 5 in a slot; the flask heals 100
    // over 12 s, twice, refilled at the fountain; control wards, two to a slot.
    Item {
        active: Active::Consumable { heal: 120.0, duration_ms: 15_000, charges: 5, refills: false },
        ..it(HEALTH_POTION, "Health Potion", 50.0, Consumable, b(), &[])
    },
    Item {
        active: Active::Consumable { heal: 100.0, duration_ms: 12_000, charges: 2, refills: true },
        ..it(REFILLABLE_FLASK, "Refillable Flask", 150.0, Consumable, b(), &[])
    },
    Item { active: Active::ControlWard { charges: 2 }, ..it(CONTROL_WARD, "Control Ward", 75.0, Consumable, b(), &[]) },
    // ---- Components.
    it(LONG_KNIFE, "Long Knife", 350.0, Component, Bonus { attack_damage: 10.0, ..b() }, &[]),
    it(SPARK_SHARD, "Spark Shard", 435.0, Component, Bonus { ability_power: 20.0, ..b() }, &[]),
    it(VITAL_CRYSTAL, "Vital Crystal", 400.0, Component, Bonus { health: 150.0, ..b() }, &[]),
    it(PADDED_VEST, "Padded Vest", 300.0, Component, Bonus { armor: 15.0, ..b() }, &[]),
    it(WARDING_CLOAK, "Warding Cloak", 450.0, Component, Bonus { magic_resist: 25.0, ..b() }, &[]),
    it(QUICK_DAGGER, "Quick Dagger", 300.0, Component, Bonus { attack_speed: 0.12, ..b() }, &[]),
    it(FOCUS_CHARM, "Focus Charm", 400.0, Component, Bonus { ability_haste: 10.0, ..b() }, &[]),
    it(FORTUNES_DIE, "Fortune's Die", 600.0, Component, Bonus { crit_chance: 0.15, ..b() }, &[]),
    it(HEAVY_PICK, "Heavy Pick", 875.0, Component, Bonus { attack_damage: 25.0, ..b() }, &[]),
    it(CHARGED_WAND, "Charged Wand", 850.0, Component, Bonus { ability_power: 40.0, ..b() }, &[]),
    it(GREATSWORD, "Greatsword", 1300.0, Component, Bonus { attack_damage: 40.0, ..b() }, &[]),
    it(ELDER_STAFF, "Elder Staff", 1250.0, Component, Bonus { ability_power: 60.0, ..b() }, &[]),
    // ---- Epics.
    it(TITAN_BELT, "Titan Belt", 900.0, Epic, Bonus { health: 350.0, ..b() }, &[VITAL_CRYSTAL]),
    it(LEECH_FANG, "Leech Fang", 900.0, Epic, Bonus { attack_damage: 15.0, life_steal: 0.07, ..b() }, &[LONG_KNIFE]),
    Item {
        passives: one(Passive::OnHitMagic { base: 15.0, ap_ratio: 0.0 }),
        ..it(ARC_BOW, "Arc Bow", 700.0, Epic, Bonus { attack_speed: 0.25, ..b() }, &[QUICK_DAGGER])
    },
    it(CHAIN_COAT, "Chain Coat", 800.0, Epic, Bonus { armor: 40.0, ..b() }, &[PADDED_VEST]),
    it(SPELLWARD_MANTLE, "Spellward Mantle", 900.0, Epic, Bonus { magic_resist: 45.0, ..b() }, &[WARDING_CLOAK]),
    Item {
        passives: one(Passive::Spellblade { base_ad: SPELLBLADE }),
        ..it(RUNED_HILT, "Runed Hilt", 700.0, Epic, b(), &[])
    },
    Item {
        passives: one(Passive::Cleave { melee: 0.4, ranged: 0.2, radius: 300.0 }),
        ..it(
            SPLITTING_AXE,
            "Splitting Axe",
            1200.0,
            Epic,
            Bonus { attack_damage: 20.0, ..b() },
            &[LONG_KNIFE, LONG_KNIFE],
        )
    },
    it(
        WHETTED_KRIS,
        "Whetted Kris",
        1000.0,
        Epic,
        Bonus { attack_damage: 20.0, lethality: 10.0, ..b() },
        &[LONG_KNIFE, LONG_KNIFE],
    ),
    it(
        GALE_CLOAK,
        "Gale Cloak",
        1050.0,
        Epic,
        Bonus { attack_speed: 0.18, crit_chance: 0.15, move_speed_pct: 0.05, ..b() },
        &[QUICK_DAGGER, FORTUNES_DIE],
    ),
    it(
        GLOWING_GARNET,
        "Glowing Garnet",
        800.0,
        Epic,
        Bonus { health: 200.0, ability_haste: 10.0, ..b() },
        &[VITAL_CRYSTAL],
    ),
    Item {
        passives: one(Passive::Wounds { pct: 0.25, physical: true, magic: false }),
        ..it(BARBED_HOOK, "Barbed Hook", 800.0, Epic, Bonus { attack_damage: 20.0, ..b() }, &[LONG_KNIFE])
    },
    Item {
        passives: one(Passive::Wounds { pct: 0.25, physical: false, magic: true }),
        ..it(BLIGHT_ORB, "Blight Orb", 800.0, Epic, Bonus { ability_power: 30.0, ..b() }, &[SPARK_SHARD])
    },
    Item {
        passives: one(QUICKEN),
        ..it(
            RAGE_KNOT,
            "Rage Knot",
            1100.0,
            Epic,
            Bonus { health: 200.0, attack_damage: 15.0, ..b() },
            &[VITAL_CRYSTAL, LONG_KNIFE],
        )
    },
    // ---- Boots (one pair).
    it(BOOTS, "Boots", 300.0, Boots, Bonus { move_speed: 25.0, ..b() }, &[]),
    it(SWIFT_BOOTS, "Swift Boots", 900.0, Boots, Bonus { move_speed: 60.0, slow_resist: 0.25, ..b() }, &[BOOTS]),
    it(
        BATTLE_BOOTS,
        "Battle Boots",
        1100.0,
        Boots,
        Bonus { move_speed: 45.0, attack_speed: 0.25, ..b() },
        &[BOOTS, QUICK_DAGGER],
    ),
    it(SAGE_BOOTS, "Sage Boots", 950.0, Boots, Bonus { move_speed: 45.0, ability_haste: 15.0, ..b() }, &[BOOTS]),
    Item {
        passives: one(Passive::AttackCut(0.12)),
        ..it(
            IRONSHOD_BOOTS,
            "Ironshod Boots",
            1100.0,
            Boots,
            Bonus { move_speed: 45.0, armor: 20.0, ..b() },
            &[BOOTS, PADDED_VEST],
        )
    },
    it(
        STALWART_SABATONS,
        "Stalwart Sabatons",
        1100.0,
        Boots,
        Bonus { move_speed: 45.0, magic_resist: 25.0, tenacity: 0.3, ..b() },
        &[BOOTS, WARDING_CLOAK],
    ),
    it(SPELLSTRIDERS, "Spellstriders", 1100.0, Boots, Bonus { move_speed: 45.0, magic_pen: 15.0, ..b() }, &[BOOTS]),
    // ---- Legendary: fighters.
    Item {
        passives: [Passive::Spellblade { base_ad: 2.0 }, QUICKEN],
        ..it(
            THREEFOLD_BRAND,
            "Threefold Brand",
            3300.0,
            Legendary,
            Bonus { attack_damage: 35.0, attack_speed: 0.3, health: 300.0, ability_haste: 15.0, ..b() },
            &[RUNED_HILT, RAGE_KNOT, QUICK_DAGGER],
        )
    },
    Item {
        passives: one(Passive::Sunder { per_stack: 0.05, stacks: 6, ms: 6000 }),
        ..it(
            SUNDERING_AXE,
            "Sundering Axe",
            3000.0,
            Legendary,
            Bonus { attack_damage: 40.0, health: 400.0, ability_haste: 20.0, ..b() },
            &[RAGE_KNOT, FOCUS_CHARM, LONG_KNIFE],
        )
    },
    Item {
        passives: one(Passive::Cleave { melee: 0.6, ranged: 0.3, radius: 350.0 }),
        active: Active::ReapingArc { ad: 1.0, radius: 350.0, cooldown_ms: 10_000 },
        ..it(
            REAPING_CLEAVER,
            "Reaping Cleaver",
            3200.0,
            Legendary,
            Bonus { attack_damage: 60.0, ability_haste: 15.0, life_steal: 0.1, ..b() },
            &[SPLITTING_AXE, LEECH_FANG, FOCUS_CHARM],
        )
    },
    Item {
        passives: one(Passive::Rally {
            shield: 150.0,
            bonus_hp: 0.6,
            threshold: 0.3,
            duration_ms: 4000,
            cooldown_ms: 60_000,
        }),
        ..it(
            LIONHEART_TALISMAN,
            "Lionheart Talisman",
            3100.0,
            Legendary,
            Bonus { attack_damage: 45.0, health: 400.0, tenacity: 0.2, ..b() },
            &[TITAN_BELT, HEAVY_PICK],
        )
    },
    Item {
        active: Active::Bloodletting { ad: 1.0, radius: 450.0, heal_missing: 0.1, cooldown_ms: 15_000 },
        ..it(
            BLOODLETTERS_MAUL,
            "Bloodletter's Maul",
            3100.0,
            Legendary,
            Bonus { attack_damage: 45.0, health: 400.0, ability_haste: 15.0, ..b() },
            &[RAGE_KNOT, HEAVY_PICK],
        )
    },
    // ---- Legendary: assassins.
    Item {
        passives: one(Passive::LightFeet { speed: 20.0, after_ms: 5000 }),
        active: Active::Fleet { pct: 20, ms: 6000, cooldown_ms: 45_000 },
        ..it(
            VEILRUNNER_DAGGER,
            "Veilrunner Dagger",
            2900.0,
            Legendary,
            Bonus { attack_damage: 55.0, lethality: 18.0, ability_haste: 10.0, ..b() },
            &[WHETTED_KRIS, FOCUS_CHARM, LONG_KNIFE],
        )
    },
    Item {
        passives: one(Passive::Ambush { max_hp: 0.06, shield: 80.0, bonus_ad: 0.25, cooldown_ms: 8000 }),
        ..it(
            DUSKFANG,
            "Duskfang",
            3000.0,
            Legendary,
            Bonus { attack_damage: 60.0, lethality: 15.0, ability_haste: 15.0, ..b() },
            &[WHETTED_KRIS, HEAVY_PICK, FOCUS_CHARM],
        )
    },
    Item {
        active: Active::Plunge {
            range: 500.0,
            base: 75.0,
            bonus_ad: 0.3,
            amp: 0.15,
            amp_ms: 3000,
            cooldown_ms: 60_000,
        },
        ..it(
            SHRIKEBLADE,
            "Shrikeblade",
            3100.0,
            Legendary,
            Bonus { attack_damage: 60.0, lethality: 18.0, ..b() },
            &[WHETTED_KRIS, HEAVY_PICK],
        )
    },
    // ---- Legendary: marksmen.
    it(
        STARSTEEL_BLADE,
        "Starsteel Blade",
        3400.0,
        Legendary,
        Bonus { attack_damage: 65.0, crit_chance: 0.25, crit_damage: 0.4, ..b() },
        &[GREATSWORD, HEAVY_PICK, FORTUNES_DIE],
    ),
    Item {
        passives: one(Passive::Fanfire { targets: 2, share: 0.45, radius: 350.0 }),
        ..it(
            FANFIRE_QUIVER,
            "Fanfire Quiver",
            2800.0,
            Legendary,
            Bonus { attack_speed: 0.4, crit_chance: 0.25, move_speed_pct: 0.05, ..b() },
            &[GALE_CLOAK, ARC_BOW],
        )
    },
    Item {
        passives: one(Passive::Topple { amp: 0.15, full_at: 1500.0 }),
        ..it(
            TITANBANE_BOW,
            "Titanbane Bow",
            3000.0,
            Legendary,
            Bonus { attack_damage: 35.0, crit_chance: 0.25, armor_pen: 0.3, ..b() },
            &[GREATSWORD, FORTUNES_DIE],
        )
    },
    Item {
        passives: one(Passive::Overflow { base: 50.0, per_level: 10.0, ms: 10_000 }),
        ..it(
            CRIMSON_FANG,
            "Crimson Fang",
            3100.0,
            Legendary,
            Bonus { attack_damage: 60.0, life_steal: 0.15, ..b() },
            &[LEECH_FANG, GREATSWORD],
        )
    },
    Item {
        passives: one(Passive::ThirdHit { base: 60.0, bonus_ad: 0.5 }),
        ..it(
            GALE_SABER,
            "Gale Saber",
            2900.0,
            Legendary,
            Bonus { attack_damage: 40.0, attack_speed: 0.3, move_speed_pct: 0.07, ..b() },
            &[HEAVY_PICK, ARC_BOW],
        )
    },
    // ---- Legendary: mages.
    it(
        GRAND_GRIMOIRE,
        "Grand Grimoire",
        3600.0,
        Legendary,
        Bonus { ability_power: 120.0, ability_power_pct: 0.35, ..b() },
        &[ELDER_STAFF, CHARGED_WAND],
    ),
    Item {
        active: Active::Encase { ms: 2500, cooldown_ms: 120_000 },
        ..it(
            AMBER_RELIQUARY,
            "Amber Reliquary",
            3000.0,
            Legendary,
            Bonus { ability_power: 100.0, armor: 45.0, ability_haste: 10.0, ..b() },
            &[CHARGED_WAND, CHAIN_COAT],
        )
    },
    it(
        HOLLOW_SCEPTER,
        "Hollow Scepter",
        3000.0,
        Legendary,
        Bonus { ability_power: 80.0, magic_pen_pct: 0.4, ..b() },
        &[ELDER_STAFF, SPARK_SHARD],
    ),
    Item {
        passives: one(Passive::Searing { max_hp: 0.015, ms: 3000 }),
        ..it(
            INFERNO_DIADEM,
            "Inferno Diadem",
            3000.0,
            Legendary,
            Bonus { ability_power: 80.0, health: 300.0, ability_haste: 15.0, ..b() },
            &[CHARGED_WAND, GLOWING_GARNET],
        )
    },
    Item {
        passives: one(Passive::OnHitMagic { base: 20.0, ap_ratio: 0.25 }),
        ..it(
            ARC_TEMPEST,
            "Arc Tempest",
            3000.0,
            Legendary,
            Bonus { ability_power: 70.0, attack_speed: 0.4, ..b() },
            &[ARC_BOW, CHARGED_WAND],
        )
    },
    Item {
        active: Active::Surge { dash: 275.0, bolts: 5, base: 110.0, ap_ratio: 0.15, cooldown_ms: 40_000 },
        ..it(
            WINDSTRIDE_BELT,
            "Windstride Belt",
            2800.0,
            Legendary,
            Bonus { ability_power: 75.0, health: 300.0, ability_haste: 10.0, ..b() },
            &[CHARGED_WAND, GLOWING_GARNET],
        )
    },
    Item {
        passives: one(Passive::Wounds { pct: 0.4, physical: false, magic: true }),
        ..it(
            BLIGHTCALLER_TOME,
            "Blightcaller Tome",
            2800.0,
            Legendary,
            Bonus { ability_power: 75.0, ability_haste: 20.0, ..b() },
            &[BLIGHT_ORB, SPARK_SHARD, FOCUS_CHARM],
        )
    },
    // ---- Legendary: tanks.
    Item {
        passives: one(Passive::Warmth { max_hp: 0.03, after_ms: 6000 }),
        ..it(
            HEARTSTONE,
            "Heartstone",
            3000.0,
            Legendary,
            Bonus { health: 800.0, health_regen: 1.0, ..b() },
            &[TITAN_BELT, VITAL_CRYSTAL],
        )
    },
    Item {
        passives: one(Passive::Thorns { base: 10.0, bonus_armor: 0.15, wounds: 0.4 }),
        ..it(
            BRIARHIDE_PLATE,
            "Briarhide Plate",
            2700.0,
            Legendary,
            Bonus { armor: 70.0, health: 350.0, ..b() },
            &[CHAIN_COAT, VITAL_CRYSTAL],
        )
    },
    Item {
        passives: one(Passive::Smolder { base: 15.0, bonus_hp: 0.015, radius: 325.0 }),
        ..it(
            FURNACE_PLATE,
            "Furnace Plate",
            2800.0,
            Legendary,
            Bonus { health: 450.0, armor: 45.0, ..b() },
            &[TITAN_BELT, CHAIN_COAT],
        )
    },
    Item {
        passives: one(Passive::CritCut(0.3)),
        active: Active::Quake { radius: 500.0, slow: 55, ms: 2000, cooldown_ms: 90_000 },
        ..it(
            STONEWALL_AEGIS,
            "Stonewall Aegis",
            2800.0,
            Legendary,
            Bonus { health: 450.0, armor: 60.0, ..b() },
            &[CHAIN_COAT, TITAN_BELT],
        )
    },
    it(
        WARDSTONE_MANTLE,
        "Wardstone Mantle",
        2800.0,
        Legendary,
        Bonus { magic_resist: 60.0, health: 400.0, ability_haste: 10.0, health_regen: 2.0, heal_received: 0.25, ..b() },
        &[SPELLWARD_MANTLE, GLOWING_GARNET],
    ),
    Item {
        active: Active::Petrify { base: 100.0, bonus_hp: 1.0, ms: 2500, cut: 0.6, cooldown_ms: 90_000 },
        ..it(
            STONESKIN_WARD,
            "Stoneskin Ward",
            3000.0,
            Legendary,
            Bonus { armor: 60.0, magic_resist: 60.0, ability_haste: 15.0, ..b() },
            &[CHAIN_COAT, SPELLWARD_MANTLE, FOCUS_CHARM],
        )
    },
    // ---- Legendary: supports.
    Item {
        active: Active::Embrace { base: 120.0, per_level: 10.0, radius: 800.0, ms: 2500, cooldown_ms: 90_000 },
        ..it(
            KINSHIP_LOCKET,
            "Kinship Locket",
            2500.0,
            Legendary,
            Bonus { health: 200.0, armor: 30.0, magic_resist: 30.0, ability_haste: 10.0, ..b() },
            &[GLOWING_GARNET, PADDED_VEST, WARDING_CLOAK],
        )
    },
    Item {
        active: Active::Absolve {
            range: 650.0,
            heal: 100.0,
            heal_missing: 0.1,
            speed: 40,
            ms: 2000,
            cooldown_ms: 120_000,
        },
        ..it(
            PURIFYING_CHALICE,
            "Purifying Chalice",
            2300.0,
            Legendary,
            Bonus { magic_resist: 50.0, ability_haste: 15.0, heal_power: 0.15, health_regen: 1.0, ..b() },
            &[SPELLWARD_MANTLE, FOCUS_CHARM],
        )
    },
    // ---- The Pilgrim's quest (18 §6): the Ledger becomes the Astrolabe at `QUEST_STAGE`
    // quest gold, which upgrades for free into a lantern at `QUEST_DONE`.
    Item {
        passives: one(Passive::Quest),
        ..it(
            PILGRIMS_LEDGER,
            "Pilgrim's Ledger",
            400.0,
            Support,
            Bonus { health: 30.0, health_regen: 1.0, ability_haste: 5.0, ..b() },
            &[],
        )
    },
    Item {
        passives: one(Passive::Quest),
        active: Active::StealthWard { charges: 3 },
        ..it(
            PILGRIMS_ASTROLABE,
            "Pilgrim's Astrolabe",
            400.0,
            Support,
            Bonus { health: 130.0, health_regen: 1.0, ability_haste: 15.0, ..b() },
            &[],
        )
    },
    Item {
        passives: one(Passive::Kindle { magic: 20.0, ms: 4000 }),
        active: Active::StealthWard { charges: 4 },
        ..it(
            SUNWARD_LANTERN,
            "Sunward Lantern",
            400.0,
            Support,
            Bonus { ability_power: 35.0, health: 150.0, ability_haste: 10.0, heal_power: 0.1, ..b() },
            &[PILGRIMS_ASTROLABE],
        )
    },
    Item {
        passives: one(Passive::Bulwark { shield: 60.0, bonus_hp: 0.05, radius: 600.0, cooldown_ms: 8000 }),
        active: Active::StealthWard { charges: 4 },
        ..it(
            BULWARK_LANTERN,
            "Bulwark Lantern",
            400.0,
            Support,
            Bonus { health: 250.0, armor: 15.0, magic_resist: 15.0, ability_haste: 10.0, ..b() },
            &[PILGRIMS_ASTROLABE],
        )
    },
    Item {
        passives: one(Passive::Expose { pct: 0.12, ms: 4000 }),
        active: Active::StealthWard { charges: 4 },
        ..it(
            HUNTERS_LANTERN,
            "Hunter's Lantern",
            400.0,
            Support,
            Bonus { attack_damage: 25.0, health: 200.0, ability_haste: 10.0, ..b() },
            &[PILGRIMS_ASTROLABE],
        )
    },
    // ---- Hatchlings (18 §7): only with Claim.
    Item {
        passives: [Passive::Hatchling(Hatchling::Windling), MONSTER_FEED],
        ..it(WINDLING, "Windling", 450.0, Jungle, b(), &[])
    },
    Item {
        passives: [Passive::Hatchling(Hatchling::Barkling), MONSTER_FEED],
        ..it(BARKLING, "Barkling", 450.0, Jungle, b(), &[])
    },
    Item {
        passives: [Passive::Hatchling(Hatchling::Flintling), MONSTER_FEED],
        ..it(FLINTLING, "Flintling", 450.0, Jungle, b(), &[])
    },
];

/// The support quest (18 §6) *(start values)*.
pub mod quest {
    use crate::time::SimDuration;
    /// Tithe: gold per charge, charges, one charge back every this long.
    pub const TITHE_GOLD: f32 = 18.0;
    pub const TITHE_CHARGES: u8 = 3;
    pub const TITHE_EVERY: SimDuration = SimDuration::from_millis(8_000);
    /// Spoils: executes an enemy minion below this share of its health (melee holders: the
    /// second) with an allied champion within `SPOILS_RANGE`; charges, one back every this long.
    pub const SPOILS_BELOW: f32 = 0.4;
    pub const SPOILS_BELOW_MELEE: f32 = 0.6;
    pub const SPOILS_RANGE: f32 = 1000.0;
    pub const SPOILS_CHARGES: u8 = 3;
    pub const SPOILS_EVERY: SimDuration = SimDuration::from_millis(30_000);
    /// The holder's own minion kills pay this share.
    pub const OWN_MINION_SHARE: f32 = 0.5;
    /// Quest gold at which the Ledger becomes the Astrolabe, and at which it can become a lantern.
    pub const STAGE: u16 = 400;
    pub const DONE: u16 = 1000;
}

/// The hatchlings (18 §7) *(start values)*.
pub mod hatchling {
    /// Damage to monsters ×, damage from monsters ×.
    pub const MONSTER_AMP: f32 = 1.25;
    pub const MONSTER_CUT: f32 = 0.8;
    /// Food (monsters killed: 1 a small one, 2 a big one) at which a hatchling grows up.
    pub const GROWN: u8 = 32;
    /// Windling: a burst of speed after Claiming, fading.
    pub const WIND_SPEED: u8 = 40;
    pub const WIND_MS: u64 = 2000;
    /// Barkling: a shield after Claiming or this long out of champion combat.
    pub const BARK_SHIELD: f32 = 150.0;
    pub const BARK_BONUS_HP: f32 = 0.1;
    pub const BARK_AFTER_MS: u64 = 10_000;
    pub const BARK_MS: u64 = 4000;
    /// Flintling: Claim on a champion: true damage `FLINT_BASE + FLINT_MAX_HP × max health`
    /// and a slow.
    pub const FLINT_BASE: f32 = 60.0;
    pub const FLINT_MAX_HP: f32 = 0.04;
    pub const FLINT_SLOW: u8 = 20;
    pub const FLINT_SLOW_MS: u64 = 2000;
}

pub fn item(id: u8) -> Option<&'static Item> {
    CATALOG.iter().find(|i| i.id == id)
}

/// What a consumable does, if `id` is one: (heal, duration, most charges, refills).
pub fn consumable(id: u8) -> Option<(f32, u64, u8, bool)> {
    match item(id)?.active {
        Active::Consumable { heal, duration_ms, charges, refills } => Some((heal, duration_ms, charges, refills)),
        _ => None,
    }
}

/// Items that stack in a slot or carry charges: how many, and whether they refill (potions, a
/// flask, control wards, a support item's stealth wards).
pub fn stackable(id: u8) -> Option<(u8, bool)> {
    match item(id)?.active {
        Active::Consumable { charges, refills, .. } => Some((charges, refills)),
        Active::ControlWard { charges } => Some((charges, false)),
        Active::StealthWard { charges } => Some((charges, true)),
        _ => None,
    }
}

/// Bought by the piece (potions and control wards stack in one slot; a flask is one per
/// champion).
pub fn sold_by_charge(id: u8) -> bool {
    item(id).is_some_and(|i| i.tier == Tier::Consumable)
}

/// Placed at the cursor as a control ward.
pub fn placeable(id: u8) -> bool {
    item(id).is_some_and(|i| matches!(i.active, Active::ControlWard { .. }))
}

/// Its active is used at a point (the cursor).
pub fn aimed(id: u8) -> bool {
    item(id).is_some_and(|i| i.active.aimed())
}

/// What else decides whether an item can be bought (18 §2).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shopper {
    /// The map has wards (control wards and the support quest).
    pub wards: bool,
    /// The F spell is Claim (hatchlings).
    pub claim: bool,
    /// Quest gold earned (the lanterns).
    pub quest: u16,
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

/// Whether `id` may be bought into an inventory that will hold `after` (the components it
/// uses gone): one starter, one pair of boots, one support item, one hatchling (with Claim),
/// one of each legendary; the quest's items on maps with wards, a lantern once the quest is
/// done; the Astrolabe only from the quest.
pub fn may_buy(id: u8, after: &[u8; INVENTORY], shopper: Shopper) -> bool {
    let Some(it) = item(id) else { return false };
    let held = |pred: &dyn Fn(&Item) -> bool| after.iter().filter_map(|i| item(*i)).any(pred);
    match it.tier {
        Tier::Starter | Tier::Boots | Tier::Support | Tier::Jungle if held(&|o| o.tier == it.tier) => false,
        Tier::Legendary if held(&|o| o.id == id) => false,
        Tier::Jungle if !shopper.claim => false,
        Tier::Support if !shopper.wards => false,
        _ if id == PILGRIMS_ASTROLABE => false,
        _ if it.recipe.contains(&PILGRIMS_ASTROLABE) => shopper.quest >= quest::DONE,
        _ => true,
    }
}

/// Unique passives from an inventory: the strongest of each kind, and the item it comes from
/// (death recaps name it).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Passives([Option<(Passive, u8)>; PASSIVE_KINDS]);

impl Passives {
    pub const NONE: Passives = Passives([None; PASSIVE_KINDS]);

    /// The counted passive of `p`'s kind (pass any value of the kind, e.g.
    /// `Passive::Cleave { melee: 0.0, ranged: 0.0, radius: 0.0 }`), and its item.
    pub fn get(&self, kind: usize) -> Option<(Passive, u8)> {
        self.0.get(kind).copied().flatten()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Passive, u8)> + '_ {
        self.0.iter().flatten().copied()
    }

    /// Any passive of this shape (`|p| matches!(p, Passive::Cleave { .. })`).
    pub fn find(&self, pred: impl Fn(&Passive) -> bool) -> Option<(Passive, u8)> {
        self.iter().find(|(p, _)| pred(p))
    }
}

pub fn passives(inventory: &[u8; INVENTORY]) -> Passives {
    let mut out = Passives::NONE;
    for it in inventory.iter().filter_map(|id| item(*id)) {
        for p in it.passives {
            let Some(k) = p.kind() else { continue };
            if out.0[k].is_none_or(|(q, _)| p.strength() > q.strength()) {
                out.0[k] = Some((p, it.id));
            }
        }
    }
    out
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
    /// What survives each multiplicative stat: Π(1 − x).
    tenacity_left: f32,
    armor_pen_left: f32,
    magic_pen_left: f32,
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
    s.crit_chance += x.crit_chance;
    s.crit_damage += x.crit_damage;
    s.lethality += x.lethality;
    s.magic_pen += x.magic_pen;
    s.omnivamp += x.omnivamp;
    s.slow_resist += x.slow_resist;
    s.heal_power += x.heal_power;
    s.heal_received += x.heal_received;
}

fn add_percents(p: &mut Percents, x: &Bonus) {
    p.ability_power += x.ability_power_pct;
    p.move_speed += x.move_speed_pct;
    p.health += x.health_pct;
    p.attack_damage += x.attack_damage_pct;
    p.resist += x.resist_pct;
    p.tenacity_left *= 1.0 - x.tenacity;
    p.armor_pen_left *= 1.0 - x.armor_pen;
    p.magic_pen_left *= 1.0 - x.magic_pen_pct;
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
    let mut pct = Percents { tenacity_left: 1.0, armor_pen_left: 1.0, magic_pen_left: 1.0, ..Default::default() };
    let mut boots_counted = false;
    let mut seen: Vec<u8> = Vec::new();
    for it in inventory.iter().filter_map(|id| item(*id)) {
        let x = &it.bonus;
        add_flat(&mut s, x);
        pct.attack_speed += x.attack_speed;
        // Only one pair of boots, and an item's percent bonuses once (unique).
        if !(it.boots() && boots_counted) {
            pct.move_speed_flat += x.move_speed;
        }
        boots_counted |= it.boots();
        if !seen.contains(&it.id) {
            add_percents(&mut pct, x);
        }
        seen.push(it.id);
    }
    // Tally: ability power for its stacks.
    if let Some((Passive::Tally { ap, .. }, _)) = passives(inventory).find(|p| matches!(p, Passive::Tally { .. })) {
        s.ability_power += ap * growth.tally as f32;
    }
    if growth.insight {
        s.ability_haste += crate::jungle::INSIGHT_HASTE;
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
    s.tenacity = 1.0 - pct.tenacity_left;
    s.armor_pen = 1.0 - pct.armor_pen_left;
    s.magic_pen_pct = 1.0 - pct.magic_pen_left;
    s.crit_chance = s.crit_chance.clamp(0.0, 1.0);
    s.slow_resist = s.slow_resist.clamp(0.0, 0.9);
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

/// A legendary's class (18 §5): who it's for. `None` for other tiers.
pub fn class_of(id: u8) -> Option<&'static str> {
    if item(id)?.tier != Tier::Legendary {
        return None;
    }
    Some(match id {
        THREEFOLD_BRAND | SUNDERING_AXE | REAPING_CLEAVER | LIONHEART_TALISMAN | BLOODLETTERS_MAUL => "Fighter",
        VEILRUNNER_DAGGER | DUSKFANG | SHRIKEBLADE => "Assassin",
        STARSTEEL_BLADE | FANFIRE_QUIVER | TITANBANE_BOW | CRIMSON_FANG | GALE_SABER => "Marksman",
        GRAND_GRIMOIRE | AMBER_RELIQUARY | HOLLOW_SCEPTER | INFERNO_DIADEM | ARC_TEMPEST | WINDSTRIDE_BELT
        | BLIGHTCALLER_TOME => "Mage",
        HEARTSTONE | BRIARHIDE_PLATE | FURNACE_PLATE | STONEWALL_AEGIS | WARDSTONE_MANTLE | STONESKIN_WARD => "Tank",
        _ => "Support",
    })
}

/// `id` with its whole recipe tree before it, components first: the order to buy it in.
pub fn buy_order(id: u8) -> Vec<u8> {
    let mut out = Vec::new();
    if let Some(it) = item(id) {
        for part in it.recipe {
            out.extend(buy_order(*part));
        }
    }
    out.push(id);
    out
}

/// Bot build paths (components first, so recipes discount them), after a starter.
pub fn build_path(champion: crate::champion::ChampionId) -> &'static [u8] {
    use crate::champion::ChampionId;
    match champion {
        ChampionId::Ember | ChampionId::Quill => &[
            SQUIRES_RING,
            BOOTS,
            CHARGED_WAND,
            GLOWING_GARNET,
            INFERNO_DIADEM,
            SPELLSTRIDERS,
            ELDER_STAFF,
            CHARGED_WAND,
            GRAND_GRIMOIRE,
            ELDER_STAFF,
            SPARK_SHARD,
            HOLLOW_SCEPTER,
            CHARGED_WAND,
            CHAIN_COAT,
            AMBER_RELIQUARY,
        ],
        ChampionId::Marrow => &[
            SQUIRES_RING,
            BOOTS,
            CHARGED_WAND,
            GLOWING_GARNET,
            INFERNO_DIADEM,
            SAGE_BOOTS,
            CHARGED_WAND,
            GLOWING_GARNET,
            WINDSTRIDE_BELT,
            CHARGED_WAND,
            CHAIN_COAT,
            AMBER_RELIQUARY,
            ELDER_STAFF,
            CHARGED_WAND,
            GRAND_GRIMOIRE,
        ],
        ChampionId::Vesper => &[
            SQUIRES_BLADE,
            BOOTS,
            GREATSWORD,
            HEAVY_PICK,
            FORTUNES_DIE,
            STARSTEEL_BLADE,
            QUICK_DAGGER,
            BATTLE_BOOTS,
            QUICK_DAGGER,
            FORTUNES_DIE,
            GALE_CLOAK,
            ARC_BOW,
            FANFIRE_QUIVER,
            GREATSWORD,
            FORTUNES_DIE,
            TITANBANE_BOW,
            LEECH_FANG,
            GREATSWORD,
            CRIMSON_FANG,
        ],
        ChampionId::Wren => &[
            SQUIRES_BLADE,
            BOOTS,
            HEAVY_PICK,
            QUICK_DAGGER,
            ARC_BOW,
            GALE_SABER,
            QUICK_DAGGER,
            BATTLE_BOOTS,
            GREATSWORD,
            HEAVY_PICK,
            FORTUNES_DIE,
            STARSTEEL_BLADE,
            GREATSWORD,
            FORTUNES_DIE,
            TITANBANE_BOW,
            LEECH_FANG,
            GREATSWORD,
            CRIMSON_FANG,
        ],
        ChampionId::Shade => &[
            SQUIRES_BLADE,
            BOOTS,
            LONG_KNIFE,
            LONG_KNIFE,
            WHETTED_KRIS,
            HEAVY_PICK,
            SHRIKEBLADE,
            SWIFT_BOOTS,
            LONG_KNIFE,
            LONG_KNIFE,
            WHETTED_KRIS,
            HEAVY_PICK,
            FOCUS_CHARM,
            DUSKFANG,
            LONG_KNIFE,
            LONG_KNIFE,
            WHETTED_KRIS,
            FOCUS_CHARM,
            LONG_KNIFE,
            VEILRUNNER_DAGGER,
        ],
        ChampionId::Rook => &[
            SQUIRES_BLADE,
            BOOTS,
            VITAL_CRYSTAL,
            LONG_KNIFE,
            RAGE_KNOT,
            HEAVY_PICK,
            BLOODLETTERS_MAUL,
            IRONSHOD_BOOTS,
            VITAL_CRYSTAL,
            LONG_KNIFE,
            RAGE_KNOT,
            FOCUS_CHARM,
            LONG_KNIFE,
            SUNDERING_AXE,
            VITAL_CRYSTAL,
            TITAN_BELT,
            HEAVY_PICK,
            LIONHEART_TALISMAN,
        ],
        ChampionId::Bastion | ChampionId::Cairn => &[
            SQUIRES_BUCKLER,
            BOOTS,
            VITAL_CRYSTAL,
            TITAN_BELT,
            PADDED_VEST,
            CHAIN_COAT,
            FURNACE_PLATE,
            IRONSHOD_BOOTS,
            VITAL_CRYSTAL,
            GLOWING_GARNET,
            PADDED_VEST,
            WARDING_CLOAK,
            KINSHIP_LOCKET,
            WARDING_CLOAK,
            SPELLWARD_MANTLE,
            VITAL_CRYSTAL,
            GLOWING_GARNET,
            WARDSTONE_MANTLE,
            PADDED_VEST,
            CHAIN_COAT,
            VITAL_CRYSTAL,
            TITAN_BELT,
            STONEWALL_AEGIS,
        ],
        ChampionId::Lumen => &[
            SQUIRES_RING,
            BOOTS,
            WARDING_CLOAK,
            SPELLWARD_MANTLE,
            FOCUS_CHARM,
            PURIFYING_CHALICE,
            SAGE_BOOTS,
            VITAL_CRYSTAL,
            GLOWING_GARNET,
            PADDED_VEST,
            WARDING_CLOAK,
            KINSHIP_LOCKET,
            CHARGED_WAND,
            GLOWING_GARNET,
            INFERNO_DIADEM,
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::champion::{EMBER, VESPER};

    #[test]
    fn catalog_is_consistent() {
        let mut ids: Vec<u8> = CATALOG.iter().map(|i| i.id).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len(), "ids are unique");
        for (i, it) in CATALOG.iter().enumerate() {
            assert_eq!(item(it.id), Some(it), "id {} at {i}", it.id);
            let parts: f32 = it.recipe.iter().map(|p| item(*p).unwrap().cost).sum();
            // Lanterns upgrade from the Astrolabe for free.
            assert!(parts < it.cost || it.tier == Tier::Support, "{}: components cost more than the item", it.name);
            for p in it.recipe {
                assert!(item(*p).is_some_and(|p| p.tier != Tier::Legendary), "{}: builds from a legendary", it.name);
            }
            if it.tier == Tier::Legendary {
                assert!(!it.recipe.is_empty() && it.cost >= 2300.0, "{}", it.name);
            }
        }
        assert!(CATALOG.len() >= 70);
    }

    #[test]
    fn every_build_path_is_buyable_in_order() {
        use crate::champion::ChampionId;
        for c in ChampionId::ALL {
            for id in build_path(c) {
                assert!(item(*id).is_some(), "{c:?}: {id}");
            }
        }
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
        // +185% attack speed: 0.8 → 2.28; from a 1.0 base it would be 2.85, capped at 2.5.
        let (s, attack_speed) =
            apply_items(VESPER.stats, VESPER.attack.attack_speed, &inv, &[0; 4], crate::augments::Growth::NONE);
        assert!((attack_speed - 2.28).abs() < 1e-4, "{attack_speed}");
        assert_eq!(apply_items(VESPER.stats, 1.0, &inv, &[0; 4], crate::augments::Growth::NONE).1, ATTACK_SPEED_CAP);
        // One pair of boots (+45) and the Gale Saber's 7%: (325 + 45) × 1.07 = 395.9.
        assert!((s.move_speed - 395.9).abs() < 1e-3, "{}", s.move_speed);
        let two_boots = [SWIFT_BOOTS, BOOTS, 0, 0, 0, 0];
        let (s, _) = apply_items(VESPER.stats, 0.8, &two_boots, &[0; 4], crate::augments::Growth::NONE);
        assert_eq!(s.move_speed, 385.0, "only the first pair counts");
    }

    /// Tenacity and percent penetration stack multiplicatively; crit chance adds.
    #[test]
    fn new_stats_stack() {
        let inv = [STALWART_SABATONS, LIONHEART_TALISMAN, TITANBANE_BOW, STARSTEEL_BLADE, HOLLOW_SCEPTER, 0];
        let (s, _) = apply_items(VESPER.stats, 0.8, &inv, &[0; 4], crate::augments::Growth::NONE);
        assert!((s.tenacity - (1.0 - 0.7 * 0.8)).abs() < 1e-5, "{}", s.tenacity);
        assert!((s.crit_chance - 0.5).abs() < 1e-5);
        assert!((s.crit_damage - 0.4).abs() < 1e-5);
        assert!((s.armor_pen - 0.3).abs() < 1e-5);
        assert!((s.magic_pen_pct - 0.4).abs() < 1e-5);
        let (s, _) = apply_items(VESPER.stats, 0.8, &[FORTUNES_DIE; 6], &[0; 4], crate::augments::Growth::NONE);
        assert_eq!(s.crit_chance, 0.9);
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
        let inv = [CHARGED_WAND, GLOWING_GARNET, 0, 0, 0, 0];
        let (cost, used) = price(INFERNO_DIADEM, &inv).unwrap();
        assert_eq!(cost, 3000.0 - 850.0 - 800.0);
        assert_eq!(used, vec![0, 1]);
        let (cost, _) = price(SPLITTING_AXE, &[LONG_KNIFE, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(cost, 1200.0 - 350.0, "one of two knives");
    }

    #[test]
    fn buying_rules() {
        // `may_buy` sees the inventory as it will be, components gone, without the new item.
        let s = Shopper { wards: true, claim: false, quest: 0 };
        assert!(!may_buy(SQUIRES_RING, &[SQUIRES_BLADE, 0, 0, 0, 0, 0], s), "one starter");
        assert!(may_buy(SQUIRES_RING, &[BOOTS, 0, 0, 0, 0, 0], s));
        assert!(!may_buy(SWIFT_BOOTS, &[SAGE_BOOTS, 0, 0, 0, 0, 0], s), "one pair of boots");
        assert!(!may_buy(GRAND_GRIMOIRE, &[GRAND_GRIMOIRE, 0, 0, 0, 0, 0], s), "one of each legendary");
        assert!(may_buy(LONG_KNIFE, &[LONG_KNIFE, 0, 0, 0, 0, 0], s), "components repeat");
        assert!(may_buy(WINDLING, &[0; 6], Shopper { claim: true, ..s }));
        assert!(!may_buy(WINDLING, &[0; 6], s), "hatchlings need Claim");
        assert!(!may_buy(BARKLING, &[WINDLING, 0, 0, 0, 0, 0], Shopper { claim: true, ..s }), "one hatchling");
        assert!(!may_buy(PILGRIMS_LEDGER, &[0; 6], Shopper { wards: false, ..s }), "the quest needs wards");
        assert!(!may_buy(PILGRIMS_ASTROLABE, &[0; 6], s), "only from the quest");
        assert!(!may_buy(SUNWARD_LANTERN, &[0; 6], Shopper { quest: quest::DONE - 1, ..s }));
        assert!(may_buy(SUNWARD_LANTERN, &[0; 6], Shopper { quest: quest::DONE, ..s }));
        assert_eq!(price(SUNWARD_LANTERN, &[PILGRIMS_ASTROLABE, 0, 0, 0, 0, 0]).unwrap().0, 0.0, "free upgrade");
    }

    #[test]
    fn unique_passives_count_the_strongest_once() {
        let p = passives(&[ARC_BOW, ARC_TEMPEST, RUNED_HILT, THREEFOLD_BRAND, 0, 0]);
        let on_hit = p.find(|p| matches!(p, Passive::OnHitMagic { .. })).unwrap();
        assert_eq!(on_hit, (Passive::OnHitMagic { base: 20.0, ap_ratio: 0.25 }, ARC_TEMPEST));
        let blade = p.find(|p| matches!(p, Passive::Spellblade { .. })).unwrap();
        assert_eq!(blade.1, THREEFOLD_BRAND);
        assert_eq!(p.iter().count(), 3, "on-hit, spellblade, quicken");
    }
}
