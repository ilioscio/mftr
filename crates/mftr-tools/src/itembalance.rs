//! `mftr-tools items`: the item set on paper (18's balance pass). Two tables:
//!
//! - **Gold efficiency:** each item's stats priced at what the components charge for them (AD
//!   from the Long Knife, AP from the Spark Shard, and so on; stats no component sells are
//!   priced by convention, `PRICES`), against its cost. Passives and actives aren't counted:
//!   a legendary's shortfall is what its mechanic has to be worth.
//! - **In a fight:** each legendary added to a level-13 champion of its class with a fixed core
//!   (`CORES`), against a reference bruiser target (2,400 health, 110 armor, 70 magic resist):
//!   attack damage a second (crit by expectation, on-hit, every third hit, penetration,
//!   Topple), an ability combo's damage (with Searing's burn), and effective health. Each is
//!   shown as gained over the core alone, per 1,000 gold.

use mftr_sim::ability::{DamageKind, Effect};
use mftr_sim::champion::ChampionId;
use mftr_sim::combat::{Penetration, effective_resist, lethality_to_flat_pen, resist_multiplier};
use mftr_sim::items::{self, Bonus, CATALOG, Item, Passive, Tier};

/// What a point of each stat is worth in gold. The first nine come from the components'
/// prices; the rest by convention (the reference game's, scaled to our components).
pub struct Prices {
    pub ad: f32,
    pub ap: f32,
    pub hp: f32,
    pub armor: f32,
    pub mr: f32,
    pub attack_speed: f32,
    pub haste: f32,
    pub crit: f32,
    pub ms: f32,
}

pub fn prices() -> Prices {
    let per = |id: u8, v: fn(&Bonus) -> f32| {
        let it = items::item(id).expect("component");
        it.cost / v(&it.bonus)
    };
    Prices {
        ad: per(items::LONG_KNIFE, |b| b.attack_damage),
        ap: per(items::SPARK_SHARD, |b| b.ability_power),
        hp: per(items::VITAL_CRYSTAL, |b| b.health),
        armor: per(items::PADDED_VEST, |b| b.armor),
        mr: per(items::WARDING_CLOAK, |b| b.magic_resist),
        attack_speed: per(items::QUICK_DAGGER, |b| b.attack_speed),
        haste: per(items::FOCUS_CHARM, |b| b.ability_haste),
        crit: per(items::FORTUNES_DIE, |b| b.crit_chance),
        ms: per(items::BOOTS, |b| b.move_speed),
    }
}

/// Gold per point of the stats no component sells *(by convention)*.
pub const LETHALITY: f32 = 30.0;
pub const MAGIC_PEN: f32 = 31.0;
/// Per 1.0 (100%) of a percent stat.
pub const ARMOR_PEN: f32 = 4000.0;
pub const MAGIC_PEN_PCT: f32 = 4000.0;
pub const LIFE_STEAL: f32 = 4000.0;
pub const OMNIVAMP: f32 = 4000.0;
pub const TENACITY: f32 = 1500.0;
pub const SLOW_RESIST: f32 = 800.0;
pub const HEAL_POWER: f32 = 3000.0;
pub const HEAL_RECEIVED: f32 = 2000.0;
pub const MS_PCT: f32 = 4000.0;
/// Health per second.
pub const REGEN: f32 = 90.0;
/// Percent ability power: worth it on ~250 AP.
pub const AP_PCT_BASE: f32 = 250.0;

/// An item's stats in gold.
pub fn stat_gold(b: &Bonus, p: &Prices) -> f32 {
    b.attack_damage * p.ad
        + b.ability_power * p.ap
        + b.health * p.hp
        + b.armor * p.armor
        + b.magic_resist * p.mr
        + b.attack_speed * p.attack_speed
        + b.ability_haste * p.haste
        + b.crit_chance * p.crit
        + b.move_speed * p.ms
        + b.lethality * LETHALITY
        + b.magic_pen * MAGIC_PEN
        + b.armor_pen * ARMOR_PEN
        + b.magic_pen_pct * MAGIC_PEN_PCT
        + b.life_steal * LIFE_STEAL
        + b.omnivamp * OMNIVAMP
        + b.tenacity * TENACITY
        + b.slow_resist * SLOW_RESIST
        + b.heal_power * HEAL_POWER
        + b.heal_received * HEAL_RECEIVED
        + b.move_speed_pct * MS_PCT
        + b.health_regen * REGEN
        + b.ability_power_pct * AP_PCT_BASE * p.ap
}

/// The reference target: a bruiser at level 13.
pub const TARGET_HP: f32 = 2400.0;
pub const TARGET_ARMOR: f32 = 110.0;
pub const TARGET_MR: f32 = 70.0;
pub const LEVEL: u8 = 13;

/// A class's reference champion and the core it already has when the legendary is added.
pub struct Core {
    pub class: &'static str,
    pub champion: ChampionId,
    pub items: &'static [u8],
}

pub const CORES: [Core; 6] = [
    Core { class: "Fighter", champion: ChampionId::Rook, items: &[items::IRONSHOD_BOOTS, items::RAGE_KNOT] },
    Core { class: "Assassin", champion: ChampionId::Shade, items: &[items::SWIFT_BOOTS, items::WHETTED_KRIS] },
    Core { class: "Marksman", champion: ChampionId::Vesper, items: &[items::BATTLE_BOOTS, items::GALE_CLOAK] },
    Core { class: "Mage", champion: ChampionId::Ember, items: &[items::SPELLSTRIDERS, items::CHARGED_WAND] },
    Core { class: "Tank", champion: ChampionId::Bastion, items: &[items::IRONSHOD_BOOTS, items::TITAN_BELT] },
    Core { class: "Support", champion: ChampionId::Lumen, items: &[items::SAGE_BOOTS, items::GLOWING_GARNET] },
];

/// What a champion with this inventory does in the reference fight.
#[derive(Clone, Copy, Debug, Default)]
pub struct Fight {
    /// Basic-attack damage a second on the target.
    pub dps: f32,
    /// One cast of each of Q W E R on the target (and Searing's burn).
    pub combo: f32,
    /// Effective health against physical and magic damage, half each.
    pub ehp: f32,
}

fn inventory(ids: &[u8]) -> [u8; items::INVENTORY] {
    let mut inv = [0u8; items::INVENTORY];
    for (i, id) in ids.iter().enumerate().take(items::INVENTORY) {
        inv[i] = *id;
    }
    inv
}

/// The fight numbers for `champion` holding `ids` at level 13.
pub fn fight(champion: ChampionId, ids: &[u8]) -> Fight {
    let def = champion.def();
    let inv = inventory(ids);
    let mut growth = mftr_sim::augments::Growth::NONE;
    // Tally at half its stacks.
    growth.tally = 5;
    let key = (LEVEL, inv, [0u8; 4], growth);
    let (s, attack) = items::champion_stats(def, &key);
    let base = def.stats_at(LEVEL);
    let passives = items::passives(&inv);
    let bonus_ad = (s.attack_damage - base.attack_damage).max(0.0);
    let armor_pen = Penetration {
        percent_pen: s.armor_pen,
        flat_pen: lethality_to_flat_pen(s.lethality, LEVEL),
        ..Default::default()
    };
    let magic_pen = Penetration { percent_pen: s.magic_pen_pct, flat_pen: s.magic_pen, ..Default::default() };
    let phys = resist_multiplier(effective_resist(TARGET_ARMOR, &armor_pen));
    let magic = resist_multiplier(effective_resist(TARGET_MR, &magic_pen));
    let topple = match passives.find(|p| matches!(p, Passive::Topple { .. })) {
        Some((Passive::Topple { amp, full_at }, _)) => {
            1.0 + amp * ((TARGET_HP - s.max_health) / full_at).clamp(0.0, 1.0)
        }
        _ => 1.0,
    };
    // Basic attacks: crit by expectation, every third hit, on-hit magic.
    let crit = 1.0 + s.crit_chance * (items::CRIT_DAMAGE + s.crit_damage - 1.0);
    let mut hit = s.attack_damage * crit * phys;
    if let Some((Passive::ThirdHit { base: b, bonus_ad: r }, _)) =
        passives.find(|p| matches!(p, Passive::ThirdHit { .. }))
    {
        hit += (b + r * bonus_ad) / 3.0 * phys;
    }
    if let Some((Passive::OnHitMagic { base: b, ap_ratio }, _)) =
        passives.find(|p| matches!(p, Passive::OnHitMagic { .. }))
    {
        hit += (b + ap_ratio * s.ability_power) * magic;
    }
    // Sunder at full stacks: the armor the attacks meet.
    if let Some((Passive::Sunder { per_stack, stacks, .. }, _)) = passives.find(|p| matches!(p, Passive::Sunder { .. }))
    {
        let pen = Penetration { percent_reduction: per_stack * stacks as f32, ..armor_pen };
        hit *= resist_multiplier(effective_resist(TARGET_ARMOR, &pen)) / phys;
    }
    let dps = hit * attack.attack_speed * topple;
    // An ability combo: each of Q W E R once at level 13's ranks (5, 4, 2, 2).
    let ranks = [5u8, 4, 2, 2];
    let mut combo = 0.0;
    let mut hits = 0;
    for (slot, rank) in ranks.iter().enumerate() {
        let a = def.abilities[slot];
        let dmg = match a.effect {
            Effect::Line(l) => Some(l.damage),
            Effect::Area(r) => Some(r.damage),
            Effect::Beam(b) => Some(b.damage),
            Effect::Lunge(l) => Some(l.damage),
            _ => None,
        };
        let Some(d) = dmg else { continue };
        let raw = d.raw(s.attack_damage, s.ability_power) + a.bonus_damage_at(*rank);
        if raw <= 0.0 {
            continue;
        }
        hits += 1;
        combo += raw
            * topple
            * match d.kind {
                DamageKind::Physical => phys,
                DamageKind::Magic => magic,
                DamageKind::True => 1.0,
            };
    }
    if hits > 0
        && let Some((Passive::Searing { max_hp, ms }, _)) = passives.find(|p| matches!(p, Passive::Searing { .. }))
    {
        // The burn runs through the combo (~1.5 s) and its full length after.
        combo += max_hp * TARGET_HP * (1.5 + ms as f32 / 1000.0) * magic;
    }
    let ehp = s.max_health * 0.5 * ((1.0 + s.armor / 100.0) + (1.0 + s.magic_resist / 100.0));
    Fight { dps, combo, ehp }
}

fn row_stats(it: &Item, p: &Prices) -> (f32, f32) {
    let g = stat_gold(&it.bonus, p);
    (g, g / it.cost.max(1.0))
}

pub fn print() {
    let p = prices();
    println!(
        "Stat prices (gold a point): AD {:.1}  AP {:.2}  HP {:.2}  armor {:.1}  MR {:.1}  AS {:.0} per 100%  haste {:.0}  crit {:.0} per 100%  MS {:.0}",
        p.ad, p.ap, p.hp, p.armor, p.mr, p.attack_speed, p.haste, p.crit, p.ms
    );
    println!("\nGold efficiency (stats only; passives and actives not counted)");
    for tier in [Tier::Starter, Tier::Component, Tier::Epic, Tier::Boots, Tier::Legendary, Tier::Support] {
        println!("  {}", tier.name());
        for it in CATALOG.iter().filter(|i| i.tier == tier) {
            let (g, eff) = row_stats(it, &p);
            let mut mech: Vec<String> = it
                .passives
                .iter()
                .filter(|p| p.kind().is_some())
                .map(|p| format!("{p:?}").split([' ', '(']).next().unwrap_or("").to_string())
                .collect();
            if it.active.cooldown().is_some() {
                mech.push("active".into());
            }
            println!(
                "    {:<22} {:>5.0}  stats {:>5.0}  {:>5.1}%  {}",
                it.name,
                it.cost,
                g,
                eff * 100.0,
                mech.join(", ")
            );
        }
    }
    println!(
        "\nIn a fight (level {LEVEL}, against {TARGET_HP} health, {TARGET_ARMOR} armor, {TARGET_MR} MR): what each legendary adds to its class's core, and per 1,000 gold"
    );
    for core in &CORES {
        let before = fight(core.champion, core.items);
        println!(
            "  {} ({:?} with {}): attacks {:.0}/s, combo {:.0}, effective health {:.0}",
            core.class,
            core.champion,
            core.items.iter().filter_map(|i| items::item(*i)).map(|i| i.name).collect::<Vec<_>>().join(" + "),
            before.dps,
            before.combo,
            before.ehp
        );
        for it in CATALOG.iter().filter(|i| i.tier == Tier::Legendary && items::class_of(i.id) == Some(core.class)) {
            let mut ids = core.items.to_vec();
            ids.push(it.id);
            let after = fight(core.champion, &ids);
            let k = 1000.0 / it.cost;
            println!(
                "    {:<22} attacks {:>+6.0} ({:>+5.0}/1k)  combo {:>+6.0} ({:>+5.0}/1k)  EHP {:>+6.0} ({:>+5.0}/1k)",
                it.name,
                after.dps - before.dps,
                (after.dps - before.dps) * k,
                after.combo - before.combo,
                (after.combo - before.combo) * k,
                after.ehp - before.ehp,
                (after.ehp - before.ehp) * k
            );
        }
    }
}
