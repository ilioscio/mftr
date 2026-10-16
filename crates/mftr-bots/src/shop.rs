//! Shopping and drafting (17 §7.3): the build path with its defensive item chosen against the
//! enemy's damage, starting items and potions, control wards, ability ranks, augments and Stat
//! Anvils.

use crate::kit::{self, Archetype, use_of};
use mftr_sim::ability::Effect as Shape;
use mftr_sim::augments::{self, Effect};
use mftr_sim::items::{self, *};
use mftr_sim::world::Progress;
use mftr_sim::{ChampionId, CommandKind, Unit};

/// Gold a stat point is worth (what a component charges for it), for comparing augments and
/// anvils.
mod worth {
    pub const AD: f32 = 35.0;
    pub const AP: f32 = 21.0;
    pub const AS: f32 = 2500.0;
    pub const HEALTH: f32 = 2.7;
    pub const ARMOR: f32 = 20.0;
    pub const MR: f32 = 18.0;
    pub const HASTE: f32 = 40.0;
    pub const MS: f32 = 12.0;
    pub const MS_PCT: f32 = 4000.0;
    pub const LIFE_STEAL: f32 = 3700.0;
    pub const REGEN: f32 = 30.0;
}

/// How much a champion gets from each stat (0–1.2).
#[derive(Clone, Copy, Debug)]
struct Profile {
    ad: f32,
    ap: f32,
    attack_speed: f32,
    health: f32,
    resist: f32,
    haste: f32,
    speed: f32,
    life_steal: f32,
}

fn profile(c: ChampionId) -> Profile {
    use Archetype as A;
    let p = |ad, ap, attack_speed, health, resist, haste, speed, life_steal| Profile {
        ad,
        ap,
        attack_speed,
        health,
        resist,
        haste,
        speed,
        life_steal,
    };
    match Archetype::of(c) {
        A::Marksman => p(1.0, 0.0, 1.1, 0.4, 0.35, 0.5, 0.7, 0.9),
        A::Skirmisher => p(1.0, 0.0, 0.9, 0.5, 0.4, 0.7, 0.8, 0.8),
        A::Assassin => p(1.1, 0.0, 0.5, 0.5, 0.4, 0.8, 0.9, 0.7),
        A::Bruiser => p(0.9, 0.0, 0.5, 0.9, 0.8, 0.8, 0.7, 0.7),
        A::Tank | A::Warden => p(0.2, 0.15, 0.2, 1.1, 1.1, 0.9, 0.6, 0.2),
        A::Mage | A::Artillery => p(0.0, 1.1, 0.05, 0.4, 0.35, 1.0, 0.6, 0.0),
        A::Battlemage => p(0.0, 1.0, 0.1, 0.7, 0.6, 0.9, 0.6, 0.0),
        A::Enchanter => p(0.0, 0.9, 0.05, 0.5, 0.45, 1.1, 0.7, 0.0),
    }
}

/// What a stat bonus is worth to `c`, in gold.
fn bonus_worth(c: ChampionId, b: &Bonus) -> f32 {
    let p = profile(c);
    // Percent bonuses act on what a champion has mid-match.
    let (ad_base, ap_base, health_base) = (120.0, 180.0, 1800.0);
    b.attack_damage * worth::AD * p.ad
        + b.attack_damage_pct * ad_base * worth::AD * p.ad
        + b.ability_power * worth::AP * p.ap
        + b.ability_power_pct * ap_base * worth::AP * p.ap
        + b.attack_speed * worth::AS * p.attack_speed
        + (b.health + b.health_pct * health_base) * worth::HEALTH * p.health
        + b.health_regen * worth::REGEN * p.health
        + (b.armor * worth::ARMOR + b.magic_resist * worth::MR) * p.resist
        + b.resist_pct * 60.0 * (worth::ARMOR + worth::MR) * p.resist
        + b.ability_haste * worth::HASTE * p.haste
        + (b.move_speed * worth::MS + b.move_speed_pct * worth::MS_PCT) * p.speed
        + b.life_steal * worth::LIFE_STEAL * p.life_steal
}

/// How many of `c`'s abilities are skillshots of each kind: (lines, areas).
fn skillshots(c: ChampionId) -> (f32, f32) {
    let (mut lines, mut areas) = (0.0, 0.0);
    for slot in 0..4u8 {
        match c.ability(slot).map(|a| a.effect) {
            Some(Shape::Line(_)) => lines += 1.0,
            Some(Shape::Area(_)) => areas += 1.0,
            _ => {}
        }
    }
    (lines, areas)
}

/// What an augment's effect is worth to `c`, in gold (17 §7.3).
fn effect_worth(c: ChampionId, e: Effect) -> f32 {
    use Archetype as A;
    let a = Archetype::of(c);
    let ranged = c.def().attack.range > 300.0;
    let (lines, areas) = skillshots(c);
    let casts_a_lot = matches!(a, A::Mage | A::Artillery | A::Battlemage | A::Enchanter);
    match e {
        Effect::None => 0.0,
        Effect::AdToAp(_) | Effect::ApToAd(_) => -200.0,
        Effect::Multishot => lines * 650.0,
        Effect::Echo => (lines + areas) * 520.0,
        Effect::Wide => (lines + areas) * 360.0,
        Effect::Titan if a.front() => 2100.0,
        Effect::Titan => 700.0,
        Effect::Pebble if a.back() || a == A::Assassin => 1500.0,
        Effect::Pebble => 900.0,
        Effect::Unstable => 1000.0,
        Effect::Executioner if a == A::Assassin => 1300.0,
        Effect::Executioner => 650.0,
        Effect::FirstStrike if matches!(a, A::Mage | A::Assassin | A::Artillery) => 900.0,
        Effect::FirstStrike => 450.0,
        Effect::LastStand if matches!(a, A::Bruiser | A::Battlemage) => 1000.0,
        Effect::LastStand => 350.0,
        Effect::Spellcrit if casts_a_lot => 1600.0,
        Effect::Spellcrit => 700.0,
        Effect::Spellhunger if casts_a_lot => 1300.0,
        Effect::Spellhunger => 500.0,
        Effect::SpellVamp if matches!(a, A::Battlemage | A::Bruiser) => 1100.0,
        Effect::SpellVamp => 600.0,
        Effect::Thorns if a.front() => 1300.0,
        Effect::Thorns => 300.0,
        Effect::Spellblade if !casts_a_lot => 1100.0,
        Effect::Spellblade => 450.0,
        Effect::Reset if a == A::Assassin => 1400.0,
        Effect::Reset => 600.0,
        Effect::AnvilLuck => 500.0,
        Effect::ChampionOfChaos => 1500.0,
        Effect::Fundamentals if casts_a_lot => 1100.0,
        Effect::Fundamentals => 700.0,
        Effect::CloseQuarters if !ranged => 0.0,
        Effect::CloseQuarters if a == A::Battlemage => 400.0,
        Effect::CloseQuarters => -1800.0,
        Effect::Sharpshooter if !ranged => -300.0,
        Effect::Sharpshooter if matches!(a, A::Marksman | A::Skirmisher) => 1300.0,
        Effect::Sharpshooter => 400.0,
        Effect::Spell(ability) => match use_of(&ability) {
            u if u.moves && a.back() => 1100.0,
            u if u.moves => 650.0,
            u if u.sustain && a.support() => 1300.0,
            u if u.sustain => 850.0,
            u if u.hard_cc && a.front() => 1500.0,
            _ => 900.0,
        },
    }
}

/// What augment `id` is worth to `c`.
pub fn augment_worth(c: ChampionId, id: u8) -> f32 {
    augments::augment(id).map_or(f32::NEG_INFINITY, |a| bonus_worth(c, &a.bonus) + effect_worth(c, a.effect))
}

/// The draft choice to keep, or a choice to reroll first (`Err`): rerolls go to the worst offer
/// when the best is poor, while it can still be rerolled.
pub fn draft(c: ChampionId, p: &Progress) -> Result<u8, u8> {
    let scored: Vec<(usize, f32)> =
        p.offer.iter().enumerate().filter(|(_, id)| **id != 0).map(|(i, id)| (i, augment_worth(c, *id))).collect();
    let best = scored.iter().copied().max_by(|a, b| a.1.total_cmp(&b.1)).map_or((0, 0.0), |b| b);
    let worst = scored.iter().copied().min_by(|a, b| a.1.total_cmp(&b.1));
    if let Some((w, value)) = worst
        && best.1 < 1200.0
        && value < 700.0
        && p.rerolled & (1 << w) == 0
    {
        return Err(w as u8);
    }
    Ok(best.0 as u8)
}

/// The anvil choice to keep.
pub fn anvil_pick(c: ChampionId, p: &Progress) -> u8 {
    let mut best = (0u8, f32::NEG_INFINITY);
    for (i, &choice) in p.anvil_offer.iter().enumerate() {
        if let Some((tier, stat)) = mftr_sim::anvils::unpack(choice) {
            let units = mftr_sim::anvils::TIER_UNITS[tier.min(2) as usize];
            let v = bonus_worth(c, &stat.bonus(units));
            if v > best.1 {
                best = (i as u8, v);
            }
        }
    }
    best.0
}

/// Which basic ability to rank first: damage over cooldown for damage, healing and shielding
/// for supports; movement last.
fn rank_priority(c: ChampionId, slot: u8) -> f32 {
    let Some(a) = c.ability(slot) else { return 0.0 };
    let u = use_of(&a);
    let cd = (a.cooldown.0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32).max(1.0);
    let support = Archetype::of(c).support();
    let mut v = if u.damage { a.per_rank.damage.max(10.0) / cd * 10.0 } else { 0.0 };
    if u.sustain {
        v += if support { 40.0 } else { 12.0 };
    }
    if u.hard_cc {
        v += 6.0;
    }
    if u.moves && !u.damage {
        v = 1.0;
    }
    v
}

/// The slot to put an ability point into: the ultimate whenever it can rank, then each basic
/// once, then the basics by priority.
pub fn level_up(c: ChampionId, p: &Progress) -> Option<u8> {
    use mftr_sim::champion::max_rank;
    if p.points == 0 {
        return None;
    }
    let can = |s: u8| p.ranks[s as usize] < max_rank(s, p.level);
    if can(3) {
        return Some(3);
    }
    let mut basics: Vec<u8> = (0..3).filter(|s| can(*s)).collect();
    basics.sort_by(|a, b| rank_priority(c, *b).total_cmp(&rank_priority(c, *a)));
    basics.iter().copied().find(|s| p.ranks[*s as usize] == 0).or(basics.first().copied())
}

/// The champion's build path, its defensive item swapped for the one that answers the enemy's
/// damage (`physical`: the share of it that's physical).
pub fn path(c: ChampionId, physical: f32) -> Vec<u8> {
    let base = items::build_path(c);
    let mut p = base.to_vec();
    let has = |id: u8| base.contains(&id);
    let mr = [WARDING_CLOAK, VITAL_CRYSTAL, FOCUS_CHARM, WARDSTONE_MANTLE];
    let armor = [PADDED_VEST, CHAIN_COAT, VITAL_CRYSTAL, BRAMBLE_PLATE];
    let swap = |p: &mut Vec<u8>, from: &[u8; 4], to: &[u8; 4]| {
        if let Some(i) = p.windows(4).position(|w| w == from) {
            p.splice(i..i + 4, to.iter().copied());
        }
    };
    if has(WARDSTONE_MANTLE) && !has(BRAMBLE_PLATE) && physical > 0.62 {
        swap(&mut p, &mr, &armor);
    } else if has(BRAMBLE_PLATE) && !has(WARDSTONE_MANTLE) && physical < 0.38 {
        swap(&mut p, &armor, &mr);
    }
    p
}

/// The inventory after buying `ids` in order from nothing (recipes consume their components).
fn simulate(ids: &[u8]) -> Vec<u8> {
    let mut inv: Vec<u8> = Vec::new();
    for &id in ids {
        if let Some(it) = items::item(id) {
            for part in it.recipe {
                if let Some(i) = inv.iter().position(|x| x == part) {
                    inv.remove(i);
                }
            }
        }
        inv.push(id);
    }
    inv
}

/// The next item of `path` given what the champion holds: the step after the longest prefix
/// of the path whose result it has (so a bot that takes over a champion mid-match carries on
/// from its items).
pub fn next_item(path: &[u8], held: &[u8; INVENTORY]) -> Option<u8> {
    let mut have: Vec<u8> = held.iter().copied().filter(|id| *id != 0 && items::stackable(*id).is_none()).collect();
    have.sort_unstable();
    let mut done = 0;
    for k in 1..=path.len() {
        let mut want = simulate(&path[..k]);
        want.sort_unstable();
        // `want` ⊆ `have` (as multisets).
        let mut h = have.iter().peekable();
        let fits = want.iter().all(|w| {
            while let Some(&&x) = h.peek() {
                h.next();
                if x == *w {
                    return true;
                }
            }
            false
        });
        if fits {
            done = k;
        }
    }
    path.get(done).copied()
}

/// What to buy now, if anything (one purchase per call). `physical`: the enemy's physical share;
/// `potions`, `wards`: how many the role wants on hand.
pub fn buy(u: &Unit, physical: f32, potions: u8, wards: u8) -> Option<CommandKind> {
    let c = u.champion?;
    let p = &u.state.progress;
    let gold = p.gold;
    let free = p.items.iter().filter(|i| **i == 0).count();
    let count =
        |id: u8| -> u8 { p.items.iter().zip(p.charges).filter(|(i, _)| **i == id).map(|(_, n)| n.max(1)).sum() };
    let path = path(c, physical);
    if let Some(next) = next_item(&path, &p.items)
        && let Some((cost, used)) = items::price(next, &p.items)
    {
        if cost <= gold {
            if free > 0 || !used.is_empty() {
                return Some(CommandKind::Buy(next));
            }
            // Full: make room by selling a consumable.
            if let Some(slot) = p.items.iter().position(|i| items::stackable(*i).is_some()) {
                return Some(CommandKind::Sell(slot as u8));
            }
        }
    }
    if count(HEALTH_POTION) < potions && gold >= 50.0 && (free > 1 || count(HEALTH_POTION) > 0) {
        return Some(CommandKind::Buy(HEALTH_POTION));
    }
    if count(CONTROL_WARD) < wards && gold >= 75.0 && (free > 1 || count(CONTROL_WARD) > 0) {
        return Some(CommandKind::Buy(CONTROL_WARD));
    }
    None
}

/// The enemy's share of physical damage, from the champions it fields.
pub fn physical_share(enemies: impl Iterator<Item = ChampionId>) -> f32 {
    let (mut phys, mut all) = (0.0, 0.0);
    for c in enemies {
        let a = Archetype::of(c);
        let (p, m) = match a {
            Archetype::Marksman | Archetype::Skirmisher | Archetype::Assassin | Archetype::Bruiser => (1.0, 0.1),
            Archetype::Tank | Archetype::Warden => (0.4, 0.4),
            _ => (0.1, 1.0),
        };
        phys += p;
        all += p + m;
    }
    if all > 0.0 { phys / all } else { 0.5 }
}

/// Damage-ability count, for tests and the coach.
pub fn damage_abilities(c: ChampionId) -> usize {
    (0..4u8).filter(|s| c.ability(*s).is_some_and(|a| kit::use_of(&a).damage)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_next_item_follows_the_path_from_any_inventory() {
        let path = items::build_path(ChampionId::Ember);
        assert_eq!(next_item(path, &[0; 6]), Some(BOOTS));
        assert_eq!(next_item(path, &[BOOTS, CHARGED_WAND, 0, 0, 0, 0]), Some(VITAL_CRYSTAL));
        // Boots consumed into Sage Boots: not bought again.
        assert_eq!(next_item(path, &[SAGE_BOOTS, INFERNO_DIADEM, 0, 0, 0, 0]), Some(CHARGED_WAND));
        // Potions don't count.
        assert_eq!(next_item(path, &[HEALTH_POTION, BOOTS, 0, 0, 0, 0]), Some(CHARGED_WAND));
    }

    #[test]
    fn defenses_answer_the_enemys_damage() {
        let p = path(ChampionId::Ember, 0.9);
        assert!(p.contains(&BRAMBLE_PLATE) && !p.contains(&WARDSTONE_MANTLE));
        let p = path(ChampionId::Rook, 0.1);
        assert!(p.contains(&WARDSTONE_MANTLE) && !p.contains(&BRAMBLE_PLATE));
        assert_eq!(path(ChampionId::Ember, 0.5), items::build_path(ChampionId::Ember));
    }

    #[test]
    fn augments_fit_the_champion() {
        // A marksman wants attack stats, a mage ability power; nobody wants Close Quarters on a
        // marksman.
        assert!(augment_worth(ChampionId::Vesper, 22) > augment_worth(ChampionId::Vesper, 21));
        assert!(augment_worth(ChampionId::Ember, 21) > augment_worth(ChampionId::Ember, 22));
        assert!(augment_worth(ChampionId::Vesper, 53) < 0.0);
        assert!(augment_worth(ChampionId::Bastion, 27) > augment_worth(ChampionId::Bastion, 28));
    }

    #[test]
    fn ultimates_rank_first_and_every_basic_once_early() {
        let mut p = mftr_sim::world::Rules::CLASSIC.progress();
        p.level = 6;
        p.points = 1;
        p.ranks = [2, 1, 1, 0];
        assert_eq!(level_up(ChampionId::Ember, &p), Some(3));
        p.ranks = [1, 0, 0, 0];
        p.level = 2;
        let s = level_up(ChampionId::Ember, &p).unwrap();
        assert!(s == 1 || s == 2);
    }
}
