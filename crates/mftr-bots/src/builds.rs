//! Builds by persona (17 §14): the same champion builds the way its player would. The steady
//! persona keeps the champion's balanced path (`items::build_path`, 18 §9); the others take
//! their class's legendaries in their own order, their own boots, starter, lantern and
//! hatchling. A path is assembled the way the hand-written ones are: the starter, Boots, the
//! first legendary from its components up, the boots upgrade, then the rest.
//!
//! What each persona wants from an item, class by class:
//! - **Daredevil:** to get in and stay in: dashes and engage actives (Windstride's Surge,
//!   Shrikeblade's Plunge, Stonewall's Quake), stickiness (Threefold Brand), sustain in the
//!   middle of a fight (Bloodletter's Maul, Crimson Fang). Swift Boots; a mage snowballs on
//!   the Tally Charm.
//! - **Carry:** the most damage the class can reach: amplifiers first (Starsteel, Grand
//!   Grimoire), penetration and multi-target after. A mage snowballs on the Tally Charm.
//! - **Guardian:** what helps the team: Kinship Locket and Purifying Chalice where the class
//!   can carry them, anti-heal (Blightcaller Tome), tank-shredding (Titanbane Bow), peel
//!   (Stonewall's Quake), Lionheart to survive for its carry. Sage Boots.
//! - **Cautious:** to live: Amber Reliquary's Encase, Lionheart, Crimson Fang's shield,
//!   Veilrunner's Fleet, Heartstone. Boots against the enemy's damage.
//! - **Trickster:** burst and mobility from odd angles: Windstride, Veilrunner, Gale Saber,
//!   Threefold Brand, Quake out of brush. Swift Boots.

use crate::kit::Archetype;
use crate::persona::Kind;
use mftr_sim::ChampionId;
use mftr_sim::items::{self, *};

/// Boots a persona wants: a pair, or the pair that answers the enemy's damage.
#[derive(Clone, Copy)]
enum Boots {
    Pair(u8),
    Defensive,
    /// The champion's own.
    Keep,
}

/// What a persona builds on a champion of class `a`: legendaries in order (the path's others
/// fill in after), boots, and a starter if it changes it.
fn wants(kind: Kind, a: Archetype) -> (&'static [u8], Boots, Option<u8>) {
    use Archetype as A;
    use Kind as K;
    let caster = matches!(a, A::Mage | A::Artillery | A::Battlemage);
    match (kind, a) {
        (K::Steady, _) => (&[], Boots::Keep, None),
        // Mages.
        (K::Daredevil, _) if caster => {
            (&[WINDSTRIDE_BELT, INFERNO_DIADEM, GRAND_GRIMOIRE], Boots::Pair(SWIFT_BOOTS), Some(TALLY_CHARM))
        }
        (K::Carry, _) if caster => {
            (&[GRAND_GRIMOIRE, HOLLOW_SCEPTER, INFERNO_DIADEM], Boots::Pair(SPELLSTRIDERS), Some(TALLY_CHARM))
        }
        (K::Guardian, _) if caster => {
            (&[BLIGHTCALLER_TOME, AMBER_RELIQUARY, INFERNO_DIADEM], Boots::Pair(SAGE_BOOTS), None)
        }
        (K::Cautious, _) if caster => (&[AMBER_RELIQUARY, INFERNO_DIADEM, WINDSTRIDE_BELT], Boots::Defensive, None),
        (K::Trickster, _) if caster => {
            (&[WINDSTRIDE_BELT, HOLLOW_SCEPTER, GRAND_GRIMOIRE], Boots::Pair(SWIFT_BOOTS), None)
        }
        // Marksmen and skirmishers.
        (K::Daredevil, A::Marksman | A::Skirmisher) => {
            (&[GALE_SABER, CRIMSON_FANG, STARSTEEL_BLADE], Boots::Keep, None)
        }
        (K::Carry, A::Marksman | A::Skirmisher) => {
            (&[STARSTEEL_BLADE, FANFIRE_QUIVER, TITANBANE_BOW], Boots::Keep, None)
        }
        (K::Guardian, A::Marksman | A::Skirmisher) => (&[TITANBANE_BOW, CRIMSON_FANG], Boots::Keep, None),
        (K::Cautious, A::Marksman | A::Skirmisher) => {
            (&[CRIMSON_FANG, GALE_SABER, STARSTEEL_BLADE], Boots::Defensive, None)
        }
        (K::Trickster, A::Marksman | A::Skirmisher) => {
            (&[GALE_SABER, FANFIRE_QUIVER, STARSTEEL_BLADE], Boots::Pair(SWIFT_BOOTS), None)
        }
        // Assassins.
        (K::Daredevil, A::Assassin) => (&[SHRIKEBLADE, DUSKFANG, VEILRUNNER_DAGGER], Boots::Pair(SWIFT_BOOTS), None),
        (K::Carry, A::Assassin) => (&[DUSKFANG, SHRIKEBLADE, VEILRUNNER_DAGGER], Boots::Keep, None),
        (K::Guardian, A::Assassin) => (&[SHRIKEBLADE, LIONHEART_TALISMAN, DUSKFANG], Boots::Defensive, None),
        (K::Cautious, A::Assassin) => (&[VEILRUNNER_DAGGER, DUSKFANG, LIONHEART_TALISMAN], Boots::Defensive, None),
        (K::Trickster, A::Assassin) => (&[VEILRUNNER_DAGGER, DUSKFANG, SHRIKEBLADE], Boots::Pair(SWIFT_BOOTS), None),
        // Fighters.
        (K::Daredevil, A::Bruiser) => {
            (&[THREEFOLD_BRAND, BLOODLETTERS_MAUL, LIONHEART_TALISMAN], Boots::Pair(SWIFT_BOOTS), None)
        }
        (K::Carry, A::Bruiser) => (&[REAPING_CLEAVER, SUNDERING_AXE, THREEFOLD_BRAND], Boots::Keep, None),
        (K::Guardian, A::Bruiser) => (&[LIONHEART_TALISMAN, KINSHIP_LOCKET, BLOODLETTERS_MAUL], Boots::Defensive, None),
        (K::Cautious, A::Bruiser) => (&[LIONHEART_TALISMAN, BLOODLETTERS_MAUL, SUNDERING_AXE], Boots::Defensive, None),
        (K::Trickster, A::Bruiser) => {
            (&[THREEFOLD_BRAND, REAPING_CLEAVER, SUNDERING_AXE], Boots::Pair(SWIFT_BOOTS), None)
        }
        // Tanks and wardens.
        (K::Daredevil, A::Tank | A::Warden) => {
            (&[STONEWALL_AEGIS, FURNACE_PLATE, STONESKIN_WARD], Boots::Defensive, None)
        }
        (K::Carry, A::Tank | A::Warden) => (&[FURNACE_PLATE, BRIARHIDE_PLATE, HEARTSTONE], Boots::Defensive, None),
        (K::Guardian, A::Tank | A::Warden) => {
            (&[KINSHIP_LOCKET, STONEWALL_AEGIS, WARDSTONE_MANTLE], Boots::Pair(SAGE_BOOTS), None)
        }
        (K::Cautious, A::Tank | A::Warden) => (&[HEARTSTONE, STONESKIN_WARD, WARDSTONE_MANTLE], Boots::Defensive, None),
        (K::Trickster, A::Tank | A::Warden) => {
            (&[STONEWALL_AEGIS, FURNACE_PLATE, BRIARHIDE_PLATE], Boots::Pair(SWIFT_BOOTS), None)
        }
        // Enchanters.
        (K::Daredevil, A::Enchanter) => {
            (&[KINSHIP_LOCKET, WINDSTRIDE_BELT, INFERNO_DIADEM], Boots::Pair(SWIFT_BOOTS), None)
        }
        (K::Carry, A::Enchanter) => (&[INFERNO_DIADEM, GRAND_GRIMOIRE, PURIFYING_CHALICE], Boots::Keep, None),
        (K::Guardian, A::Enchanter) => (&[PURIFYING_CHALICE, KINSHIP_LOCKET, WARDSTONE_MANTLE], Boots::Keep, None),
        (K::Cautious, A::Enchanter) => (&[KINSHIP_LOCKET, PURIFYING_CHALICE, AMBER_RELIQUARY], Boots::Defensive, None),
        (K::Trickster, A::Enchanter) => {
            (&[WINDSTRIDE_BELT, PURIFYING_CHALICE, INFERNO_DIADEM], Boots::Pair(SWIFT_BOOTS), None)
        }
        _ => (&[], Boots::Keep, None),
    }
}

fn tier(id: u8) -> Option<Tier> {
    items::item(id).map(|i| i.tier)
}

/// `c`'s build as `kind` plays it, from its balanced path `base`; `physical` is the enemy's
/// physical share (for the boots that answer it, and the armor or magic-resist legendary).
pub fn path(c: ChampionId, kind: Kind, base: &[u8], physical: f32) -> Vec<u8> {
    let (prefer, boots, starter) = wants(kind, Archetype::of(c));
    if prefer.is_empty() && matches!(boots, Boots::Keep) && starter.is_none() {
        return base.to_vec();
    }
    let base_legendaries: Vec<u8> = base.iter().copied().filter(|id| tier(*id) == Some(Tier::Legendary)).collect();
    // The resist legendary answers the enemy's damage.
    let answer = |id: u8| match id {
        WARDSTONE_MANTLE if physical > 0.62 => BRIARHIDE_PLATE,
        BRIARHIDE_PLATE if physical < 0.38 => WARDSTONE_MANTLE,
        _ => id,
    };
    let mut legendaries: Vec<u8> = Vec::new();
    for id in prefer.iter().chain(&base_legendaries).map(|id| answer(*id)) {
        if !legendaries.contains(&id) && legendaries.len() < base_legendaries.len().max(3) {
            legendaries.push(id);
        }
    }
    let base_boots =
        base.iter().copied().find(|id| tier(*id) == Some(Tier::Boots) && *id != BOOTS).unwrap_or(SWIFT_BOOTS);
    let boots = match boots {
        Boots::Pair(id) => id,
        Boots::Keep => base_boots,
        Boots::Defensive if physical >= 0.5 => IRONSHOD_BOOTS,
        Boots::Defensive => STALWART_SABATONS,
    };
    let starter =
        starter.or_else(|| base.iter().copied().find(|id| tier(*id) == Some(Tier::Starter))).unwrap_or(SQUIRES_BLADE);
    let mut out = vec![starter, BOOTS];
    for (i, &l) in legendaries.iter().enumerate() {
        out.extend(items::buy_order(l));
        if i == 0 {
            // The boots upgrade: its parts but the Boots already bought.
            let order = items::buy_order(boots);
            let mut skipped = false;
            for id in order {
                if id == BOOTS && !skipped {
                    skipped = true;
                    continue;
                }
                out.push(id);
            }
        }
    }
    out
}

/// The lantern a support's quest ends in (18 §6): the class's, unless the persona would rather
/// start fights (the Hunter's Expose) or protect (the Bulwark on a front-liner).
pub fn lantern(c: ChampionId, kind: Kind) -> u8 {
    use Archetype as A;
    let a = Archetype::of(c);
    match kind {
        Kind::Daredevil | Kind::Trickster if !a.support() || a.front() => HUNTERS_LANTERN,
        Kind::Guardian | Kind::Cautious if a.front() => BULWARK_LANTERN,
        Kind::Carry if !a.front() && !matches!(a, A::Enchanter | A::Mage | A::Artillery | A::Battlemage) => {
            HUNTERS_LANTERN
        }
        _ => match a {
            A::Enchanter | A::Mage | A::Artillery | A::Battlemage => SUNWARD_LANTERN,
            A::Tank | A::Warden => BULWARK_LANTERN,
            _ => HUNTERS_LANTERN,
        },
    }
}

/// The hatchling a jungler raises (18 §7): Flintling's Claim on champions for a daredevil,
/// Windling's speed to roam for a trickster, Barkling's shield to stay safe; else the class's.
pub fn hatchling(c: ChampionId, kind: Kind) -> u8 {
    use Archetype as A;
    match kind {
        Kind::Daredevil => FLINTLING,
        Kind::Trickster => WINDLING,
        Kind::Cautious | Kind::Guardian => BARKLING,
        _ => match Archetype::of(c) {
            A::Tank | A::Warden | A::Bruiser => BARKLING,
            A::Assassin | A::Skirmisher => FLINTLING,
            _ => WINDLING,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every persona's build for every champion is a real build: buyable in order (each
    /// recipe's parts bought before it), the starter first, one pair of boots upgraded, as
    /// many legendaries as the balanced path (at least three), none twice.
    #[test]
    fn every_persona_build_is_whole() {
        for c in ChampionId::ALL {
            let base = items::build_path(c);
            for kind in Kind::ALL {
                for physical in [0.2, 0.5, 0.8] {
                    let p = path(c, kind, base, physical);
                    assert_eq!(tier(p[0]), Some(Tier::Starter), "{c:?} {kind:?}");
                    let legendaries: Vec<u8> =
                        p.iter().copied().filter(|id| tier(*id) == Some(Tier::Legendary)).collect();
                    let base_n = base.iter().filter(|id| tier(**id) == Some(Tier::Legendary)).count();
                    assert_eq!(legendaries.len(), base_n.max(3), "{c:?} {kind:?}");
                    let mut seen = legendaries.clone();
                    seen.sort_unstable();
                    seen.dedup();
                    assert_eq!(seen.len(), legendaries.len(), "{c:?} {kind:?}: a legendary twice");
                    let upgraded = p.iter().filter(|id| tier(**id) == Some(Tier::Boots) && **id != BOOTS).count();
                    assert_eq!(upgraded, 1, "{c:?} {kind:?}: one pair of boots upgraded");
                    assert_eq!(p.iter().filter(|id| **id == BOOTS).count(), 1, "{c:?} {kind:?}: Boots bought once");
                    // Recipes find their parts in what's been bought (the hand-written balanced
                    // paths buy some epics outright).
                    if kind == Kind::Steady {
                        continue;
                    }
                    let mut inv: Vec<u8> = Vec::new();
                    for &id in &p {
                        let it = items::item(id).expect("an item");
                        for part in it.recipe {
                            let i = inv.iter().position(|x| x == part);
                            assert!(i.is_some(), "{c:?} {kind:?}: {} before its {}", it.name, part);
                            inv.remove(i.unwrap());
                        }
                        inv.push(id);
                    }
                }
            }
        }
    }

    #[test]
    fn personas_build_differently_and_steady_keeps_the_balanced_path() {
        for c in ChampionId::ALL {
            let base = items::build_path(c);
            assert_eq!(path(c, Kind::Steady, base, 0.5), base.to_vec());
        }
        let first = |c, k| {
            path(c, k, items::build_path(c), 0.5).into_iter().find(|id| tier(*id) == Some(Tier::Legendary)).unwrap()
        };
        assert_eq!(first(ChampionId::Ember, Kind::Carry), GRAND_GRIMOIRE);
        assert_eq!(first(ChampionId::Ember, Kind::Cautious), AMBER_RELIQUARY);
        assert_eq!(first(ChampionId::Bastion, Kind::Guardian), KINSHIP_LOCKET);
        assert_eq!(first(ChampionId::Shade, Kind::Trickster), VEILRUNNER_DAGGER);
        assert_eq!(path(ChampionId::Ember, Kind::Carry, items::build_path(ChampionId::Ember), 0.5)[0], TALLY_CHARM);
        assert_eq!(lantern(ChampionId::Bastion, Kind::Daredevil), HUNTERS_LANTERN);
        assert_eq!(lantern(ChampionId::Lumen, Kind::Guardian), SUNWARD_LANTERN);
        assert_eq!(hatchling(ChampionId::Rook, Kind::Daredevil), FLINTLING);
    }
}
