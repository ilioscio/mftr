//! The fight model (17 §5): "if we fight here now, who wins?" Each side's sustained damage,
//! ready burst, effective health and crowd control, turned into the time each needs to kill the
//! other. The coach asks it about teamfights, ganks, dives and objectives; agents about
//! trades, all-ins, chases and retreats.

use crate::kit::{self, use_of};
use mftr_sim::ability::DamageKind;
use mftr_sim::combat::resist_multiplier;
use mftr_sim::{SimTime, Unit};

/// Damage by type.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Dmg {
    pub phys: f32,
    pub magic: f32,
    pub tru: f32,
}

impl Dmg {
    fn add(&mut self, kind: DamageKind, v: f32) {
        match kind {
            DamageKind::Physical => self.phys += v,
            DamageKind::Magic => self.magic += v,
            DamageKind::True => self.tru += v,
        }
    }

    fn plus(self, o: Dmg, k: f32) -> Dmg {
        Dmg { phys: self.phys + o.phys * k, magic: self.magic + o.magic * k, tru: self.tru + o.tru * k }
    }

    /// After resistances.
    pub fn against(self, armor: f32, mr: f32) -> f32 {
        self.phys * resist_multiplier(armor) + self.magic * resist_multiplier(mr) + self.tru
    }

    pub fn total(self) -> f32 {
        self.phys + self.magic + self.tru
    }
}

/// One fighter's strength.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Power {
    /// Damage a second over a fight.
    pub dps: Dmg,
    /// Damage of what's ready now.
    pub burst: Dmg,
    /// Health and shields.
    pub hp: f32,
    pub max_hp: f32,
    pub armor: f32,
    pub mr: f32,
    /// Seconds of hard crowd control ready.
    pub cc: f32,
    /// How far it hits from.
    pub range: f32,
}

/// How often an ability of this kind lands in a fight (the hit model's typical outcome).
fn typical_hit(a: &mftr_sim::ability::Ability) -> f32 {
    let u = use_of(a);
    if u.onto_enemy || (u.nova && !u.skillshot) {
        0.9
    } else if u.skillshot {
        0.6
    } else {
        0.8
    }
}

/// `u`'s power now. `ready(slot)`: whether the slot is up, as far as the asker knows (its own
/// team's cooldowns, or what it tracked of an enemy's). `known`: the asker knows its ranks.
pub fn power(u: &Unit, now: SimTime, known: bool, ready: impl Fn(u8) -> bool) -> Power {
    let st = &u.state;
    let mut p = Power {
        hp: st.health + if st.shield_until > now { st.shield } else { 0.0 },
        max_hp: u.stats.max_health,
        armor: u.stats.armor,
        mr: u.stats.magic_resist,
        ..Power::default()
    };
    if let Some(a) = u.attack {
        p.dps.phys += u.stats.attack_damage * a.attack_speed;
        p.range = a.range;
    }
    if u.champion.is_none() {
        return p;
    }
    let haste = 100.0 / (100.0 + u.stats.ability_haste);
    for slot in 0..4u8 {
        let Some(a) = u.ability(slot) else { continue };
        let rank = kit::rank(u, slot, known);
        if rank == 0 {
            continue;
        }
        let (kind, raw) = kit::raw_damage(&a, u, rank);
        let hit = typical_hit(&a);
        let cd = (a.cooldown_at(rank).0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32 * haste).max(1.0);
        p.dps.add(kind, raw * hit / cd);
        if ready(slot) {
            p.burst.add(kind, raw * hit);
            p.cc += kit::cc_seconds(&a) * hit;
            if use_of(&a).damage {
                p.range = p.range.max(kit::reach(&a).min(1200.0));
            }
        }
    }
    p
}

/// A turret's power against champions (its shot every attack timer, warmed up a little).
pub fn turret_power(t: &Unit) -> Power {
    let shots = t.attack.map_or(0.83, |a| a.attack_speed);
    Power {
        dps: Dmg { phys: t.stats.attack_damage * 1.6 * shots, ..Dmg::default() },
        hp: t.state.health,
        max_hp: t.stats.max_health,
        range: t.attack.map_or(750.0, |a| a.range),
        ..Power::default()
    }
}

/// A side of an engagement: its fighters added up.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Side {
    pub dps: Dmg,
    pub burst: Dmg,
    /// Health of the champions (what the other side must chew through).
    pub hp: f32,
    /// Health-weighted resistances.
    armor_hp: f32,
    mr_hp: f32,
    pub cc: f32,
    pub champions: f32,
}

impl Side {
    /// Add a champion's power, `share` of it (a missing enemy that may or may not come).
    pub fn add(&mut self, p: &Power, share: f32) {
        self.dps = self.dps.plus(p.dps, share);
        self.burst = self.burst.plus(p.burst, share);
        self.hp += p.hp * share;
        self.armor_hp += p.armor * p.hp * share;
        self.mr_hp += p.mr * p.hp * share;
        self.cc += p.cc * share;
        self.champions += share;
    }

    /// Add damage that isn't a champion's (a turret, minions turning on champions).
    pub fn add_support(&mut self, dps: Dmg) {
        self.dps = self.dps.plus(dps, 1.0);
    }

    pub fn armor(&self) -> f32 {
        if self.hp > 0.0 { self.armor_hp / self.hp } else { 30.0 }
    }

    pub fn mr(&self) -> f32 {
        if self.hp > 0.0 { self.mr_hp / self.hp } else { 30.0 }
    }
}

/// Seconds `a` needs to kill all of `b`: `b`'s health, less `a`'s ready burst, over `a`'s damage
/// a second, plus the time `b`'s crowd control takes away.
fn time_to_kill(a: &Side, b: &Side) -> f32 {
    let (armor, mr) = (b.armor(), b.mr());
    let dps = a.dps.against(armor, mr).max(1.0);
    let burst = a.burst.against(armor, mr) * 0.8;
    (b.hp - burst).max(b.hp * 0.1) / dps + b.cc * 0.5
}

/// Who wins if `us` and `them` fight now: −1 (we lose badly) to +1 (we win easily); 0 is even.
/// Sides with no champions: whoever has one wins outright.
pub fn outcome(us: &Side, them: &Side) -> f32 {
    match (us.champions > 0.05, them.champions > 0.05) {
        (false, false) => return 0.0,
        (true, false) => return 1.0,
        (false, true) => return -1.0,
        _ => {}
    }
    let (t_us, t_them) = (time_to_kill(us, them), time_to_kill(them, us));
    (t_them - t_us) / (t_us + t_them).max(0.01)
}

/// Seconds for `side` to kill one unit with `hp` health and these resistances (burst first).
pub fn time_to_kill_one(side: &Side, hp: f32, armor: f32, mr: f32) -> f32 {
    let burst = side.burst.against(armor, mr) * 0.8;
    (hp - burst).max(0.0) / side.dps.against(armor, mr).max(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::map::MapId;
    use mftr_sim::world::{PlayerId, Rules, World};
    use mftr_sim::{ChampionId, Team, Vec2};

    fn world() -> (World, Vec<mftr_sim::UnitId>) {
        let mut w = World::new(1);
        w.set_map(MapId::Crossroads.shared());
        w.set_rules(Rules::CLASSIC);
        w.start_match();
        let ids = (0..4u8)
            .map(|i| {
                let team = if i < 2 { Team::Blue } else { Team::Red };
                let c = [ChampionId::Rook, ChampionId::Vesper, ChampionId::Rook, ChampionId::Vesper][i as usize];
                w.spawn_champion(PlayerId(i), team, c, Vec2::new(7000.0 + i as f32 * 50.0, 7000.0))
            })
            .collect();
        w.step(&[]);
        (w, ids)
    }

    #[test]
    fn even_fights_are_even_and_numbers_win() {
        let (w, ids) = world();
        let now = SimTime::end_of(w.tick());
        let p = |i: usize| power(w.unit(ids[i]).unwrap(), now, true, |_| true);
        let (mut a, mut b) = (Side::default(), Side::default());
        a.add(&p(0), 1.0);
        a.add(&p(1), 1.0);
        b.add(&p(2), 1.0);
        b.add(&p(3), 1.0);
        assert!(outcome(&a, &b).abs() < 0.05, "mirror: {}", outcome(&a, &b));
        let mut solo = Side::default();
        solo.add(&p(2), 1.0);
        assert!(outcome(&a, &solo) > 0.3, "two on one: {}", outcome(&a, &solo));
        assert!(outcome(&solo, &a) < -0.3);
        // A hurt side does worse.
        let mut hurt = solo;
        hurt.hp *= 0.3;
        let mut one = Side::default();
        one.add(&p(0), 1.0);
        assert!(outcome(&one, &hurt) > 0.3);
    }
}
