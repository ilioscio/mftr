//! Mayhem: Horde. Five champions defend the blue Base on the Mayhem map. Waves advance
//! only when every scheduled enemy is dead; clearing 20 wins the campaign and starts an
//! endless run. Uses the existing minions, buff monsters, items and four augment slots.

use crate::champion::Stats;
use crate::hash::StateSink;
use crate::jungle::MonsterKind;
use crate::{MinionKind, SimDuration, SimTime, UnitId};

pub const GOAL: u32 = 20;
pub const PLAYERS: u8 = 5;
pub const PREPARE: SimDuration = SimDuration::from_millis(30_000);
pub const PACK_INTERVAL: SimDuration = SimDuration::from_millis(1500);
pub const PACK_SIZE: u16 = 6;
/// Four existing drafts, before these waves. Unpicked drafts queue, never overwrite one.
pub const DRAFT_WAVES: [u32; 4] = [1, 6, 11, 16];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HordeState {
    pub wave: u32,
    pub cleared: u32,
    pub spawned: u16,
    pub next_pack_at: SimTime,
    pub boss: Option<UnitId>,
    /// Campaign victory is a milestone, not a frozen match: wave 21 follows it.
    pub won_at: Option<SimTime>,
}

impl HordeState {
    pub fn hash_into(&self, h: &mut impl StateSink) {
        h.write_u32(self.wave);
        h.write_u32(self.cleared);
        h.write_u16(self.spawned);
        h.write_u64(self.next_pack_at.0);
        h.write_u32(self.boss.map_or(0, |id| id.0));
        h.write_u64(self.won_at.map_or(u64::MAX, |at| at.0));
    }

    pub fn drafts(&self) -> u8 {
        // During the break, the next wave's draft is available already.
        DRAFT_WAVES.iter().filter(|&&w| w <= self.cleared.saturating_add(1)).count() as u8
    }
}

/// Spawn six at a time instead of stacking an entire horde on one collision point. The
/// population is capped at 116 + one boss even in endless; strength continues to grow.
#[derive(Clone, Copy, Debug)]
pub struct Wave {
    pub number: u32,
    pub melee: u16,
    pub casters: u16,
    pub siege: u16,
    pub supers: u16,
}

impl Wave {
    pub fn new(number: u32) -> Self {
        let n = number.saturating_sub(1);
        Self {
            number,
            melee: (6 + 2 * n.min(29)) as u16,
            casters: (4 + n.min(28)) as u16,
            siege: (number / 3).min(16) as u16,
            supers: (number.saturating_sub(7) / 4).min(4) as u16,
        }
    }

    pub fn count(self) -> u16 {
        self.melee + self.casters + self.siege + self.supers
    }

    pub fn kind(self, index: u16) -> MinionKind {
        // Mix melee and casters through the packs, with siege and supers bringing up the rear.
        let bodies = self.melee + self.casters;
        if index < bodies {
            let before = index as u32 * self.casters as u32 / bodies as u32;
            let after = (index as u32 + 1) * self.casters as u32 / bodies as u32;
            if after > before { MinionKind::Caster } else { MinionKind::Melee }
        } else if index < bodies + self.siege {
            MinionKind::Siege
        } else {
            MinionKind::Super
        }
    }

    pub fn stats(self, kind: MinionKind) -> Stats {
        let n = self.number.saturating_sub(1).min(1000) as f32;
        let mut stats = kind.stats();
        stats.max_health = (stats.max_health * (0.75 + 0.07 * n)).min(60_000.0);
        stats.attack_damage *= 0.65 + 0.075 * n;
        // Resistances grow gently: AoE mages and physical carries both remain useful.
        stats.armor += (1.5 * n).min(100.0);
        stats.magic_resist += n.min(80.0);
        stats.move_speed = (325.0 + n * 2.0).min(425.0);
        stats
    }

    pub fn boss(self) -> Option<MonsterKind> {
        if self.number > 0 && self.number.is_multiple_of(20) {
            Some(MonsterKind::Brute)
        } else if self.number > 0 && self.number.is_multiple_of(10) {
            Some(MonsterKind::Warden)
        } else {
            None
        }
    }

    pub fn boss_stats(self) -> Stats {
        let big = self.boss() == Some(MonsterKind::Brute);
        let endless = self.number.saturating_sub(if big { 20 } else { 10 }).min(1000) as f32;
        Stats {
            max_health: ((if big { 32_000.0 } else { 10_000.0 }) * (1.0 + 0.07 * endless)).min(60_000.0),
            armor: if big { 60.0 } else { 35.0 },
            magic_resist: if big { 60.0 } else { 35.0 },
            attack_damage: (if big { 220.0 } else { 95.0 }) * (1.0 + 0.04 * endless),
            move_speed: 300.0,
            ..Stats::NONE
        }
    }

    /// A shared completion purse supplements shared last-hit gold and Mayhem's 4 gold/s.
    /// Four to five completed items by wave 20; endless gold can buy existing Stat Anvils.
    pub fn clear_gold(self) -> f32 {
        140.0
            + 18.0 * self.number.min(20) as f32
            + match self.boss() {
                Some(MonsterKind::Brute) => 750.0,
                Some(_) => 250.0,
                None => 0.0,
            }
    }

    /// Each champion gets this on clear, plus shared kill XP. Level 6 in the early waves,
    /// around 11 at the miniboss, and 18 before the final boss, independent of last hits.
    pub fn clear_xp(self) -> u32 {
        200 + 55 * self.number.min(20)
    }

    pub fn break_after(self) -> SimDuration {
        let next = self.number.saturating_add(1);
        let ms = if self.number == GOAL {
            40_000
        } else if DRAFT_WAVES.contains(&next) || next.is_multiple_of(10) {
            30_000
        } else {
            20_000
        };
        SimDuration::from_millis(ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hordes_grow_and_endless_stays_bounded() {
        let mut previous = (0, 0.0, 0.0);
        for number in 1..=80 {
            let w = Wave::new(number);
            let s = w.stats(MinionKind::Melee);
            assert!(w.count() >= previous.0 && s.max_health > previous.1 && s.attack_damage > previous.2);
            assert!(w.count() <= 116);
            let kinds: Vec<_> = (0..w.count()).map(|i| w.kind(i)).collect();
            for (kind, count) in [
                (MinionKind::Melee, w.melee),
                (MinionKind::Caster, w.casters),
                (MinionKind::Siege, w.siege),
                (MinionKind::Super, w.supers),
            ] {
                assert_eq!(kinds.iter().filter(|&&k| k == kind).count(), count as usize);
            }
            previous = (w.count(), s.max_health, s.attack_damage);
        }
        assert_eq!(Wave::new(10).boss(), Some(MonsterKind::Warden));
        assert_eq!(Wave::new(20).boss(), Some(MonsterKind::Brute));
        assert!(Wave::new(19).boss().is_none());
        assert_eq!(Wave::new(u32::MAX).count(), 116);
        assert!(Wave::new(1000).boss_stats().max_health <= 60_000.0);
    }
}
