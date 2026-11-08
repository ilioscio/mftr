//! The passive catalog (20 §8.1): what a champion made in the creator can take as its innate
//! passive. Each is one of the sim's unique passives (the items' mechanics, 18 §4) with
//! numbers of its own, chosen in the Moveset step and tuned by a power knob in Balance like
//! the abilities. The kit carries the passive itself (`passive:` in the champion file), so the
//! sim and every client run the same one.

use mftr_sim::items::Passive;

/// (id, name, what it does, the passive at power 1).
pub const PASSIVES: [(&str, &str, &str, Passive); 9] = [
    (
        "arcane_strikes",
        "Arcane strikes",
        "Basic attacks deal extra magic damage.",
        Passive::OnHitMagic { base: 18.0, ap_ratio: 0.2 },
    ),
    (
        "third_blow",
        "Third blow",
        "Every third basic attack deals extra physical damage.",
        Passive::ThirdHit { base: 45.0, bonus_ad: 0.4 },
    ),
    (
        "spellblade",
        "Spellblade",
        "After an ability, the next basic attack deals extra damage.",
        Passive::Spellblade { base_ad: 0.8 },
    ),
    (
        "cleave",
        "Cleave",
        "Basic attacks also hit enemies around the target.",
        Passive::Cleave { melee: 0.35, ranged: 0.15, radius: 300.0 },
    ),
    ("fleet", "Fleet", "Basic attacks grant a burst of move speed.", Passive::Quicken { speed: 25.0, ms: 2000 }),
    (
        "sunder",
        "Sunder",
        "Physical damage to champions strips their armor, stacking.",
        Passive::Sunder { per_stack: 0.04, stacks: 5, ms: 5000 },
    ),
    (
        "thorns",
        "Thorns",
        "Basic attacks on you hurt the attacker and apply Wounds.",
        Passive::Thorns { base: 12.0, bonus_armor: 0.15, wounds: 0.3 },
    ),
    (
        "last_stand",
        "Last stand",
        "Dropping low on health grants a shield.",
        Passive::Rally { shield: 120.0, bonus_hp: 0.08, threshold: 0.3, duration_ms: 3000, cooldown_ms: 60000 },
    ),
    (
        "second_wind",
        "Second wind",
        "Out of harm a while, regenerate health quickly.",
        Passive::Warmth { max_hp: 0.02, after_ms: 6000 },
    ),
];

/// The catalog entry the kit's passive is (by its kind: tuned numbers are still that passive),
/// if any.
pub fn of(p: Passive) -> Option<&'static (&'static str, &'static str, &'static str, Passive)> {
    let kind = p.kind()?;
    PASSIVES.iter().find(|e| e.3.kind() == Some(kind))
}

/// The passive `id` names, at power 1.
pub fn get(id: &str) -> Option<Passive> {
    PASSIVES.iter().find(|e| e.0 == id).map(|e| e.3)
}

/// The resources a kit can run on (20 §8.1, 01 §9): (id, name, what it does, the resource). The
/// costs are a start the Balance step's price reflects (a thirsty kit is cheaper).
pub const RESOURCES: [(&str, &str, &str, mftr_sim::champion::Resource); 4] = {
    use mftr_sim::champion::{Resource, ResourceKind};
    [
        (
            "mana",
            "Mana",
            "A pool that grows with level and refills slowly, and at the fountain.",
            Resource { kind: ResourceKind::Mana, max: 300.0, regen: 6.0, costs: [40.0, 50.0, 60.0, 100.0] },
        ),
        (
            "energy",
            "Energy",
            "A small pool that refills fast: bursts, then a breath.",
            Resource { kind: ResourceKind::Energy, max: 200.0, regen: 40.0, costs: [50.0, 40.0, 60.0, 0.0] },
        ),
        (
            "rage",
            "Rage",
            "Starts empty, builds on basic attacks that land, drains out of use.",
            Resource { kind: ResourceKind::Rage, max: 100.0, regen: 10.0, costs: [20.0, 30.0, 30.0, 60.0] },
        ),
        (
            "health",
            "Health",
            "Abilities cost health.",
            Resource { kind: ResourceKind::Health, max: 0.0, regen: 0.0, costs: [30.0, 40.0, 50.0, 80.0] },
        ),
    ]
};

/// The id of the resource kind `r` is ("" costless).
pub fn resource_id(r: &mftr_sim::champion::Resource) -> &'static str {
    RESOURCES.iter().find(|e| e.3.kind == r.kind).map_or("", |e| e.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_resource_is_legal() {
        let rook = *mftr_sim::ChampionId::Rook.def();
        for (id, _, _, r) in RESOURCES {
            let def = mftr_sim::champion::ChampionDef { resource: r, ..rook };
            assert!(mftr_sim::limits::problems(&def).is_empty(), "{id}: {:?}", mftr_sim::limits::problems(&def));
            assert_eq!(resource_id(&r), id);
        }
    }

    #[test]
    fn every_passive_is_legal_and_written() {
        let rook = *mftr_sim::ChampionId::Rook.def();
        for (id, _, _, p) in PASSIVES {
            let def = mftr_sim::champion::ChampionDef { passive: p, ..rook };
            assert!(mftr_sim::limits::problems(&def).is_empty(), "{id}: {:?}", mftr_sim::limits::problems(&def));
            let text = mftr_sim::data::write_champion(&def, "");
            assert_eq!(mftr_sim::data::read_champion(&text), Ok(def), "{id}");
            assert_eq!(of(p).map(|e| e.0), Some(id));
        }
    }
}
