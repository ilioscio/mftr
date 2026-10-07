//! Augments (M3, 06 §3): permanent modifiers drafted during an ARAM: Mayhem match.
//!
//! At scheduled levels a champion is offered three augments of one tier and keeps one; each
//! offer can be rerolled once. Offers come from the champion's own augment seed and the draft
//! and reroll counts, never from the world RNG, so picking and rerolling are predicted on the
//! client exactly like shopping. Stat augments join the item stat stack (02 §10): flat bonuses
//! with the items, then conversions, then percent bonuses, then caps.

use crate::items::Bonus;
use crate::rng::Pcg32;
use crate::world::Progress;

/// Augments a champion can hold.
pub const SLOTS: usize = 4;
/// Choices in each offer.
pub const CHOICES: usize = 3;
/// A draft opens when the champion reaches each of these levels (ARAM starts at level 3, so
/// the first opens at once) *(start values)*.
pub const DRAFT_LEVELS: [u8; SLOTS] = [1, 7, 11, 15];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Simple stat conversions and consistent passives.
    Silver,
    /// Playstyle adjustments and strong conditionals.
    Gold,
    /// Identity-rewriting game-breakers.
    Prismatic,
}

impl Tier {
    pub fn name(self) -> &'static str {
        match self {
            Tier::Silver => "Silver",
            Tier::Gold => "Gold",
            Tier::Prismatic => "Prismatic",
        }
    }
}

/// What an augment does beyond its stat bonus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    None,
    /// Bonus attack damage (from items and augments) becomes ability power at this rate.
    AdToAp(f32),
    /// Bonus ability power becomes attack damage at this rate.
    ApToAd(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Augment {
    /// Stable on the wire; 0 = none.
    pub id: u8,
    pub name: &'static str,
    pub tier: Tier,
    /// For the draft cards.
    pub text: &'static str,
    pub bonus: Bonus,
    pub effect: Effect,
}

const fn b() -> Bonus {
    Bonus::NONE
}

macro_rules! aug {
    ($id:expr, $tier:ident, $name:expr, $text:expr, $bonus:expr) => {
        Augment { id: $id, name: $name, tier: Tier::$tier, text: $text, bonus: $bonus, effect: Effect::None }
    };
    ($id:expr, $tier:ident, $name:expr, $text:expr, $bonus:expr, $effect:expr) => {
        Augment { id: $id, name: $name, tier: Tier::$tier, text: $text, bonus: $bonus, effect: $effect }
    };
}

/// Every augment (original names; *(start values)*).
pub const CATALOG: &[Augment] = &[
    // Silver: stats and conversions.
    aug!(
        1,
        Silver,
        "Thick Skin",
        "+350 health and +3 health regeneration per second.",
        Bonus { health: 350.0, health_regen: 3.0, ..b() }
    ),
    aug!(2, Silver, "Whetstone", "+20 attack damage.", Bonus { attack_damage: 20.0, ..b() }),
    aug!(3, Silver, "Spark", "+35 ability power.", Bonus { ability_power: 35.0, ..b() }),
    aug!(4, Silver, "Quickstep", "+8% movement speed.", Bonus { move_speed_pct: 0.08, ..b() }),
    aug!(5, Silver, "Plating", "+25 armor and +25 magic resist.", Bonus { armor: 25.0, magic_resist: 25.0, ..b() }),
    aug!(6, Silver, "Swift Hands", "+30% attack speed.", Bonus { attack_speed: 0.30, ..b() }),
    aug!(7, Silver, "Clear Mind", "+20 ability haste.", Bonus { ability_haste: 20.0, ..b() }),
    aug!(8, Silver, "Bloodthirst", "+10% life steal.", Bonus { life_steal: 0.10, ..b() }),
    aug!(9, Silver, "Conversion", "Your bonus attack damage becomes ability power at 110%.", b(), Effect::AdToAp(1.1)),
    aug!(10, Silver, "Reversal", "Your bonus ability power becomes attack damage at 60%.", b(), Effect::ApToAd(0.6)),
    // Gold: bigger packages.
    aug!(
        11,
        Gold,
        "Brute Force",
        "+30 attack damage and +20 ability haste.",
        Bonus { attack_damage: 30.0, ability_haste: 20.0, ..b() }
    ),
    aug!(
        12,
        Gold,
        "Arcane Surge",
        "+60 ability power, then +10% ability power.",
        Bonus { ability_power: 60.0, ability_power_pct: 0.10, ..b() }
    ),
    aug!(13, Gold, "Juggernaut", "+15% health and +40 armor.", Bonus { health_pct: 0.15, armor: 40.0, ..b() }),
    aug!(
        14,
        Gold,
        "Fleet",
        "+12% movement speed and +30% attack speed.",
        Bonus { move_speed_pct: 0.12, attack_speed: 0.30, ..b() }
    ),
    aug!(
        15,
        Gold,
        "Vampiric Pact",
        "+15% life steal and +20 attack damage.",
        Bonus { life_steal: 0.15, attack_damage: 20.0, ..b() }
    ),
    aug!(
        16,
        Gold,
        "Glass Cannon",
        "+25% attack damage and ability power, but 20% less health.",
        Bonus { attack_damage_pct: 0.25, ability_power_pct: 0.25, health_pct: -0.20, ..b() }
    ),
    aug!(
        20,
        Gold,
        "Sorcery Engine",
        "+40 ability power and +25 ability haste.",
        Bonus { ability_power: 40.0, ability_haste: 25.0, ..b() }
    ),
    // Prismatic.
    aug!(
        17,
        Prismatic,
        "Apex Form",
        "+25% health, attack damage, ability power, armor and magic resist.",
        Bonus { health_pct: 0.25, attack_damage_pct: 0.25, ability_power_pct: 0.25, resist_pct: 0.25, ..b() }
    ),
    aug!(
        18,
        Prismatic,
        "Celerity",
        "+60 ability haste and +15% movement speed.",
        Bonus { ability_haste: 60.0, move_speed_pct: 0.15, ..b() }
    ),
    aug!(
        19,
        Prismatic,
        "Bulwark",
        "+1000 health, +60 armor and +60 magic resist.",
        Bonus { health: 1000.0, armor: 60.0, magic_resist: 60.0, ..b() }
    ),
    aug!(
        21,
        Prismatic,
        "Overload",
        "+100 ability power, then +20% ability power.",
        Bonus { ability_power: 100.0, ability_power_pct: 0.20, ..b() }
    ),
    aug!(
        22,
        Prismatic,
        "Warlord",
        "+60 attack damage and +40% attack speed.",
        Bonus { attack_damage: 60.0, attack_speed: 0.40, ..b() }
    ),
    aug!(
        23,
        Prismatic,
        "Undying",
        "+500 health and +25% life steal.",
        Bonus { health: 500.0, life_steal: 0.25, ..b() }
    ),
];

pub fn augment(id: u8) -> Option<&'static Augment> {
    CATALOG.iter().find(|a| a.id == id)
}

/// The augments in a holder's slots.
pub fn held(slots: &[u8; SLOTS]) -> impl Iterator<Item = &'static Augment> + '_ {
    slots.iter().filter_map(|id| augment(*id))
}

/// The tier of draft `draft` (0-based) for a champion with this seed: Silver first, one
/// guaranteed Prismatic at the second or third draft, Gold otherwise.
pub fn tier_of(seed: u32, draft: u8) -> Tier {
    let prismatic = 1 + (seed & 1) as u8;
    match draft {
        0 => Tier::Silver,
        d if d == prismatic => Tier::Prismatic,
        _ => Tier::Gold,
    }
}

/// Three distinct augments of the draft's tier that the champion doesn't hold (and, for a
/// reroll, that weren't just offered when the tier has enough of them).
pub fn make_offer(seed: u32, draft: u8, roll: u8, held: &[u8; SLOTS], previous: &[u8; CHOICES]) -> [u8; CHOICES] {
    let tier = tier_of(seed, draft);
    let mut pool: Vec<u8> = CATALOG.iter().filter(|a| a.tier == tier && !held.contains(&a.id)).map(|a| a.id).collect();
    let fresh: Vec<u8> = pool.iter().copied().filter(|id| !previous.contains(id)).collect();
    if fresh.len() >= CHOICES {
        pool = fresh;
    }
    let mut rng = Pcg32::new(seed as u64, 1 + draft as u64 * 2 + roll as u64);
    let mut offer = [0u8; CHOICES];
    for slot in offer.iter_mut() {
        if pool.is_empty() {
            break;
        }
        *slot = pool.swap_remove(rng.next_u32() as usize % pool.len());
    }
    offer
}

/// Open the next draft once the champion's level reaches it (one at a time).
pub fn update_draft(p: &mut Progress) {
    let d = p.drafted as usize;
    if p.offer[0] != 0 || d >= SLOTS || p.level < DRAFT_LEVELS[d] {
        return;
    }
    p.offer = make_offer(p.augment_seed, p.drafted, 0, &p.augments, &[0; CHOICES]);
    p.drafted += 1;
    p.rerolled = false;
}

/// Keep choice `choice` of the open offer.
pub fn pick(p: &mut Progress, choice: u8) {
    let Some(&id) = p.offer.get(choice as usize) else { return };
    let Some(slot) = p.augments.iter().position(|a| *a == 0) else { return };
    if id == 0 {
        return;
    }
    p.augments[slot] = id;
    p.offer = [0; CHOICES];
}

/// Replace the open offer with three new choices (once per draft).
pub fn reroll(p: &mut Progress) {
    if p.offer[0] == 0 || p.rerolled || p.drafted == 0 {
        return;
    }
    p.offer = make_offer(p.augment_seed, p.drafted - 1, 1, &p.augments, &p.offer);
    p.rerolled = true;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        for (i, a) in CATALOG.iter().enumerate() {
            assert!(a.id != 0);
            assert_eq!(augment(a.id), Some(a), "id {} at {i}", a.id);
            assert!(!a.text.is_empty());
        }
        // An offer and its reroll show six different augments, even in the second Gold draft
        // (one Gold augment already held).
        for (tier, need) in [(Tier::Silver, 6), (Tier::Gold, 7), (Tier::Prismatic, 6)] {
            let n = CATALOG.iter().filter(|a| a.tier == tier).count();
            assert!(n >= need, "{tier:?} has {n}");
        }
    }

    #[test]
    fn drafts_open_by_level_and_picks_fill_slots() {
        let mut p = Progress { level: 3, augment_seed: 77, ..Progress::SANDBOX };
        update_draft(&mut p);
        assert_eq!(p.drafted, 1);
        let first = p.offer;
        assert!(first.iter().all(|id| augment(*id).is_some_and(|a| a.tier == Tier::Silver)));
        update_draft(&mut p);
        assert_eq!(p.offer, first, "one draft at a time");
        reroll(&mut p);
        assert!(p.rerolled);
        assert!(p.offer.iter().all(|id| !first.contains(id)), "a reroll shows new choices");
        let again = p.offer;
        reroll(&mut p);
        assert_eq!(p.offer, again, "one reroll per draft");
        pick(&mut p, 1);
        assert_eq!(p.augments, [again[1], 0, 0, 0]);
        assert_eq!(p.offer, [0; CHOICES]);
        update_draft(&mut p);
        assert_eq!(p.offer, [0; CHOICES], "the next draft waits for level 7");
        p.level = 11;
        update_draft(&mut p);
        assert_eq!(p.drafted, 2);
        pick(&mut p, 0);
        update_draft(&mut p);
        assert_eq!(p.drafted, 3, "a draft skipped by fast leveling opens right after");
        // The same seed always gives the same offers (prediction depends on it).
        let mut q = Progress { level: 3, augment_seed: 77, ..Progress::SANDBOX };
        update_draft(&mut q);
        assert_eq!(q.offer, first);
    }

    #[test]
    fn every_seed_gets_one_prismatic_draft() {
        for seed in 0..64 {
            let tiers: Vec<Tier> = (0..SLOTS as u8).map(|d| tier_of(seed, d)).collect();
            assert_eq!(tiers.iter().filter(|t| **t == Tier::Prismatic).count(), 1, "{tiers:?}");
            assert_eq!(tiers[0], Tier::Silver);
        }
    }
}
