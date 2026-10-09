//! Stat Anvils (ARAM: Mayhem, 06 §3; like the reference game's): late gold buys stats. From
//! level 9 a champion who can shop buys an anvil for 750 gold; its tier is rolled (Silver,
//! Gold or Prismatic) and it offers three stat bonuses of that tier, one of which the champion
//! keeps for the match. Anvils add up. Deterministic from the champion's seed, so predicted.

use crate::items::Bonus;
use crate::rng::Pcg32;

pub const COST: f32 = 750.0;
pub const MIN_LEVEL: u8 = 9;
/// Choices an anvil offers.
pub const CHOICES: usize = 3;

/// The stats an anvil can grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stat {
    AttackDamage,
    AbilityPower,
    Health,
    Armor,
    MagicResist,
    AttackSpeed,
    AbilityHaste,
    MoveSpeed,
}

pub const STATS: [Stat; 8] = [
    Stat::AttackDamage,
    Stat::AbilityPower,
    Stat::Health,
    Stat::Armor,
    Stat::MagicResist,
    Stat::AttackSpeed,
    Stat::AbilityHaste,
    Stat::MoveSpeed,
];

/// Tier odds, in percent: Silver, Gold, Prismatic.
pub const ODDS: [u32; 3] = [60, 30, 10];
/// What one tier is worth, in units of a stat's step: Silver 5, Gold 9, Prismatic 15.
pub const TIER_UNITS: [u16; 3] = [5, 9, 15];

impl Stat {
    pub fn name(self) -> &'static str {
        match self {
            Stat::AttackDamage => "attack damage",
            Stat::AbilityPower => "ability power",
            Stat::Health => "health",
            Stat::Armor => "armor",
            Stat::MagicResist => "magic resist",
            Stat::AttackSpeed => "attack speed",
            Stat::AbilityHaste => "ability haste",
            Stat::MoveSpeed => "move speed",
        }
    }

    /// The bonus of `units` steps of this stat (a Silver anvil is 5 steps: +10 attack damage,
    /// +16 ability power, +150 health…).
    pub fn bonus(self, units: u16) -> Bonus {
        let n = units as f32;
        let b = Bonus::NONE;
        match self {
            Stat::AttackDamage => Bonus { attack_damage: 2.0 * n, ..b },
            Stat::AbilityPower => Bonus { ability_power: 3.2 * n, ..b },
            Stat::Health => Bonus { health: 30.0 * n, ..b },
            Stat::Armor => Bonus { armor: 2.4 * n, ..b },
            Stat::MagicResist => Bonus { magic_resist: 2.4 * n, ..b },
            Stat::AttackSpeed => Bonus { attack_speed: 0.02 * n, ..b },
            Stat::AbilityHaste => Bonus { ability_haste: 1.6 * n, ..b },
            Stat::MoveSpeed => Bonus { move_speed_pct: 0.006 * n, ..b },
        }
    }
}

/// An offer's choice on the wire and in `Progress`: the tier (0–2) times 16 plus the stat's
/// index plus 1; 0 is no choice.
pub fn pack(tier: u8, stat: usize) -> u8 {
    tier * 16 + stat as u8 + 1
}

pub fn unpack(c: u8) -> Option<(u8, Stat)> {
    let stat = *STATS.get(((c & 15) as usize).checked_sub(1)?)?;
    Some((c >> 4, stat))
}

/// The `n`th anvil a champion with this seed buys: a tier, and three different stats of it.
pub fn roll(seed: u32, n: u8) -> [u8; CHOICES] {
    let mut rng = Pcg32::new(seed as u64, 5000 + n as u64);
    let r = rng.next_u32() % 100;
    let tier = if r < ODDS[0] {
        0
    } else if r < ODDS[0] + ODDS[1] {
        1
    } else {
        2
    };
    let mut pool: Vec<usize> = (0..STATS.len()).collect();
    let mut out = [0u8; CHOICES];
    for c in out.iter_mut() {
        let i = pool.swap_remove(rng.next_u32() as usize % pool.len());
        *c = pack(tier, i);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anvils_offer_three_different_stats_of_one_tier() {
        let mut tiers = [0; 3];
        for seed in 0..300 {
            let offer = roll(seed, 0);
            let parsed: Vec<(u8, Stat)> = offer.iter().map(|c| unpack(*c).unwrap()).collect();
            assert!(parsed.iter().all(|(t, _)| *t == parsed[0].0), "one tier");
            assert!(parsed[0].1 != parsed[1].1 && parsed[1].1 != parsed[2].1 && parsed[0].1 != parsed[2].1);
            tiers[parsed[0].0 as usize] += 1;
        }
        assert!(tiers[0] > tiers[1] && tiers[1] > tiers[2] && tiers[2] > 0, "{tiers:?}");
        assert_eq!(Stat::AttackDamage.bonus(TIER_UNITS[0]).attack_damage, 10.0);
        assert_eq!(Stat::Health.bonus(TIER_UNITS[2]).health, 450.0);
    }
}
