//! The match loop of a lane map (M2 slice 1, 01 §1–§4): structures and their destruction
//! order, minion waves walking a lane, minion and turret target selection, the fountain, health
//! relics and the win condition.
//!
//! Everything here is deterministic simulation (server and replays); clients don't predict
//! other units, but they do see `protected` flags so their own attack orders are predicted
//! against the same rules.

use crate::champion::{AttackSpec, Stats};
use crate::hash::StateSink;
use crate::map::Map;
use crate::math::{QPoint, Vec2};
use crate::time::{SimDuration, SimTime};
use crate::world::{Brain, MinionKind, Order, Team, Unit, UnitId, UnitKind};

/// How far lane minions look for something to fight.
pub const MINION_ACQUIRE: f32 = 700.0;
/// A minion drops a target that drags it this far from where it is.
pub const MINION_LEASH: f32 = 1000.0;
/// How long an attack on a champion keeps calling for help (minions and turrets, 01 §3–§4).
pub const AGGRESSION_MEMORY: SimDuration = SimDuration::from_millis(2000);
/// Champions who damaged a champion this recently share its kill as assists (01 §5).
pub const ASSIST_MEMORY: SimDuration = SimDuration::from_millis(10_000);
/// Champions this close to a dying enemy share its experience (01 §6).
pub const XP_RANGE: f32 = 1400.0;
/// Every champion of the team that destroys a turret gets this much gold.
pub const TURRET_GOLD: f32 = 150.0;
pub const KILL_GOLD: f32 = 300.0;

/// (gold for the last hit, experience) of a lane minion, by its attack range (01 §5–§6).
pub fn minion_reward(attack_range: f32) -> (f32, u32) {
    match crate::world::MinionKind::from_attack_range(attack_range) {
        crate::world::MinionKind::Melee => (21.0, 60),
        crate::world::MinionKind::Caster => (14.0, 30),
        crate::world::MinionKind::Siege => (60.0, 90),
        crate::world::MinionKind::Super => (60.0, 97),
    }
}

/// Kill gold for a champion on `streak` (01 §5): kill streaks raise the bounty, death streaks
/// lower it.
pub fn bounty(streak: i8) -> f32 {
    if streak >= 2 {
        KILL_GOLD + (50.0 * (streak - 1) as f32).min(500.0)
    } else if streak <= -2 {
        (KILL_GOLD - 40.0 * ((-streak - 1).min(4)) as f32).max(140.0)
    } else {
        KILL_GOLD
    }
}

/// Each of `sharers` gets `total / n`, plus 15% per extra sharer (01 §6: fewer sharers level
/// faster, but sharing isn't a pure split).
pub fn shared_xp(total: u32, sharers: usize) -> u32 {
    if sharers == 0 {
        return 0;
    }
    let n = sharers as f32;
    (total as f32 / n * (1.0 + 0.15 * (n - 1.0))).round() as u32
}
/// Consecutive turret shots on the same champion hit harder: +35% each, up to 5 steps.
pub const TURRET_HEAT_STEP: f32 = 0.35;
pub const TURRET_HEAT_MAX: u8 = 5;
pub const WAVE_INTERVAL: SimDuration = SimDuration::from_millis(30_000);
pub const FIRST_WAVE: SimDuration = SimDuration::from_millis(15_000);
pub const GATEHOUSE_RESPAWN: SimDuration = SimDuration::from_millis(300_000);
pub const RELIC_RESPAWN: SimDuration = SimDuration::from_millis(40_000);
/// A relic heals this share of max health.
pub const RELIC_HEAL: f32 = 0.25;
/// Fountain: allies heal this share of max health per second; enemies take true damage.
pub const FOUNTAIN_HEAL: f32 = 0.15;
pub const FOUNTAIN_DPS: f32 = 1000.0;

/// Structure stats (start values). Turrets attack with homing bolts.
pub const TURRET_STATS: Stats = Stats {
    max_health: 2500.0,
    health_regen: 0.0,
    armor: 50.0,
    magic_resist: 50.0,
    attack_damage: 160.0,
    ..Stats::NONE
};
pub const TURRET_ATTACK: AttackSpec =
    AttackSpec { range: 775.0, attack_speed: 0.83, windup_fraction: 0.15, bolt_speed: 1200.0 };
pub const GATEHOUSE_STATS: Stats = Stats { max_health: 3000.0, armor: 20.0, magic_resist: 20.0, ..Stats::NONE };
pub const BASE_STATS: Stats = Stats { max_health: 4000.0, ..Stats::NONE };

/// Share of a minion's max health one turret shot takes (true damage).
pub fn turret_minion_share(minion_attack_range: f32) -> f32 {
    if minion_attack_range >= 500.0 {
        0.70 // caster
    } else if minion_attack_range >= 200.0 {
        0.14 // siege
    } else if minion_attack_range >= 150.0 {
        0.07 // super
    } else {
        0.45 // melee
    }
}

/// Lane minion attacks (01 §4 *(start)*): melee hits at the end of its windup.
pub fn minion_attack(kind: MinionKind) -> AttackSpec {
    match kind {
        MinionKind::Melee => AttackSpec { range: 110.0, attack_speed: 1.25, windup_fraction: 0.3, bolt_speed: 0.0 },
        MinionKind::Caster => AttackSpec { range: 550.0, attack_speed: 0.67, windup_fraction: 0.3, bolt_speed: 650.0 },
        MinionKind::Siege => AttackSpec { range: 300.0, attack_speed: 1.0, windup_fraction: 0.3, bolt_speed: 1200.0 },
        MinionKind::Super => AttackSpec { range: 170.0, attack_speed: 0.85, windup_fraction: 0.3, bolt_speed: 0.0 },
    }
}

pub fn minion_damage(kind: MinionKind) -> f32 {
    match kind {
        MinionKind::Melee => 12.0,
        MinionKind::Caster => 23.0,
        MinionKind::Siege => 40.0,
        MinionKind::Super => 190.0,
    }
}

/// The dynamic state of a match on a lane map (the static part is the map's `Layout`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MatchState {
    pub next_wave_at: Option<SimTime>,
    pub waves_spawned: u32,
    /// Recent champion-on-champion attacks: (attacker, victim, at).
    pub aggression: Vec<(UnitId, UnitId, SimTime)>,
    /// Set once a Base falls.
    pub winner: Option<(Team, SimTime)>,
}

impl MatchState {
    pub fn hash_into(&self, h: &mut impl StateSink) {
        h.write_u64(self.next_wave_at.map_or(u64::MAX, |t| t.0));
        h.write_u32(self.waves_spawned);
        h.write_u32(self.aggression.len() as u32);
        for (a, v, t) in &self.aggression {
            h.write_u32(a.0);
            h.write_u32(v.0);
            h.write_u64(t.0);
        }
        match self.winner {
            None => h.write_u8(0),
            Some((team, at)) => {
                h.write_u8(1 + team as u8);
                h.write_u64(at.0);
            }
        }
    }

    /// Minions of wave `n` (0-based): 3 melee, 3 casters, a siege minion every third wave, and
    /// a super minion in front while the team has an enemy Gatehouse down (`empowered`).
    pub fn wave(n: u32, empowered: bool) -> Vec<MinionKind> {
        let mut w = if empowered { vec![MinionKind::Super] } else { Vec::new() };
        w.extend([MinionKind::Melee; 3]);
        if n % 3 == 2 {
            w.push(MinionKind::Siege);
        }
        w.extend([MinionKind::Caster; 3]);
        w
    }
}

/// A unit as AI decisions see it at the start of the tick.
#[derive(Clone, Copy, Debug)]
pub struct Seen {
    pub id: UnitId,
    pub team: Team,
    pub kind: UnitKind,
    pub pos: Vec2,
    pub radius: f32,
    /// Attack range (minion type for turret priorities), 0 if none.
    pub range: f32,
    pub targetable: bool,
}

impl Seen {
    pub fn of(u: &Unit) -> Seen {
        Seen {
            id: u.id,
            team: u.team,
            kind: u.kind,
            pos: u.state.pos,
            radius: u.gameplay_radius,
            range: u.attack.map_or(0.0, |a| a.range),
            targetable: u.targetable(),
        }
    }
}

/// Structures still standing protect the ones behind them (01 §3): a structure can be hurt only
/// when every structure of its team with a lower tier is down. Recomputed every tick.
pub fn update_protection(units: &mut [Unit]) {
    let lowest = |team: Team, units: &[Unit]| {
        units.iter().filter(|u| u.team == team && u.tier > 0 && u.state.alive()).map(|u| u.tier).min()
    };
    let lows = [lowest(Team::Blue, units), lowest(Team::Red, units)];
    for u in units.iter_mut().filter(|u| u.tier > 0) {
        u.protected = lows[u.team as usize].is_some_and(|low| u.tier > low);
    }
}

fn enemy_valid(me: &Unit, s: &Seen, hidden: &[UnitId]) -> bool {
    s.team != me.team && s.targetable && hidden.binary_search(&s.id).is_err()
}

/// The champion that recently attacked one of `team`'s champions inside `area` (center, radius),
/// if it's in there too: what turrets and minions answer first (01 §3–§4).
fn aggressor(
    team: Team,
    seen: &[Seen],
    aggression: &[(UnitId, UnitId, SimTime)],
    area: (Vec2, f32),
    hidden: &[UnitId],
) -> Option<UnitId> {
    let find = |id: UnitId| seen.iter().find(|s| s.id == id);
    let inside = |s: &Seen| (s.pos - area.0).length_sq() <= area.1 * area.1;
    aggression
        .iter()
        .rev()
        .filter_map(|&(a, v, _)| Some((find(a)?, find(v)?)))
        .find(|(a, v)| {
            v.team == team
                && a.team != team
                && a.targetable
                && hidden.binary_search(&a.id).is_err()
                && inside(a)
                && inside(v)
        })
        .map(|(a, _)| a.id)
}

/// Lane minion decisions (01 §4): answer calls for help, else keep a valid target, else take
/// the closest enemy minion, then structure, then champion; with nothing to fight, walk the lane.
pub fn laner_think(
    unit: &mut Unit,
    seen: &[Seen],
    lane: &[Vec2],
    aggression: &[(UnitId, UnitId, SimTime)],
    hidden: &[UnitId],
    map: &Map,
) {
    let Some(Brain::Laner { mut next }) = unit.brain else { return };
    if !unit.state.alive() {
        return;
    }
    let me = unit.state.pos;
    let valid = |s: &&Seen| enemy_valid(unit, s, hidden);
    let near = |s: &&Seen, r: f32| (s.pos - me).length_sq() <= r * r;
    let help = aggressor(unit.team, seen, aggression, (me, MINION_ACQUIRE), hidden);
    let current = match unit.state.order {
        Order::Attack(id) => seen.iter().filter(valid).find(|s| s.id == id && near(s, MINION_LEASH)).map(|s| s.id),
        _ => None,
    };
    let rank = |s: &Seen| match s.kind {
        UnitKind::Minion => 0,
        UnitKind::Turret | UnitKind::Gatehouse | UnitKind::Base => 1,
        _ => 2,
    };
    let pick = help.or(current).or_else(|| {
        seen.iter()
            .filter(valid)
            .filter(|s| near(s, MINION_ACQUIRE))
            .min_by(|a, b| {
                (rank(a), (a.pos - me).length_sq(), a.id)
                    .partial_cmp(&(rank(b), (b.pos - me).length_sq(), b.id))
                    .unwrap()
            })
            .map(|s| s.id)
    });
    if let Some(id) = pick {
        if unit.state.order != Order::Attack(id) {
            unit.state.set_order(Order::Attack(id), map);
        }
        return;
    }
    // Walk the lane: on to the next waypoint once close to this one.
    while (next as usize) + 1 < lane.len() && (lane[next as usize] - me).length() < 150.0 {
        next += 1;
    }
    unit.brain = Some(Brain::Laner { next });
    if let Some(&wp) = lane.get(next as usize) {
        let goal = Order::MoveTo(QPoint::from_vec2(wp));
        if unit.state.order != goal {
            unit.state.set_order(goal, map);
        }
    }
}

/// Lane turret decisions (01 §3): a champion attacking an allied champion in range first, then
/// keep the current target while it's valid, else siege > caster > melee minions (closest
/// first), else the closest champion.
pub fn tower_think(
    unit: &mut Unit,
    seen: &[Seen],
    aggression: &[(UnitId, UnitId, SimTime)],
    hidden: &[UnitId],
    map: &Map,
) {
    let Some(Brain::Tower { .. }) = unit.brain else { return };
    let Some(atk) = unit.attack else { return };
    if !unit.state.alive() {
        return;
    }
    let me = unit.state.pos;
    let in_range = |s: &Seen| (s.pos - me).length() <= atk.range + s.radius;
    let valid = |s: &&Seen| enemy_valid(unit, s, hidden) && in_range(s);
    let help = aggressor(unit.team, seen, aggression, (me, atk.range + 65.0), hidden)
        .filter(|id| seen.iter().filter(valid).any(|s| s.id == *id));
    let current = match unit.state.order {
        Order::Attack(id) => seen.iter().filter(valid).find(|s| s.id == id).map(|s| s.id),
        _ => None,
    };
    let rank = |s: &Seen| match s.kind {
        UnitKind::Minion if s.range >= 200.0 && s.range < 500.0 => 0, // siege
        UnitKind::Minion if s.range >= 500.0 => 1,                    // caster
        UnitKind::Minion => 2,                                        // melee
        _ => 3,
    };
    let pick = help.or(current).or_else(|| {
        seen.iter()
            .filter(valid)
            .min_by(|a, b| {
                (rank(a), (a.pos - me).length_sq(), a.id)
                    .partial_cmp(&(rank(b), (b.pos - me).length_sq(), b.id))
                    .unwrap()
            })
            .map(|s| s.id)
    });
    match pick {
        Some(id) if unit.state.order != Order::Attack(id) => {
            unit.state.attack = None; // a turret re-targets at once
            unit.state.set_order(Order::Attack(id), map);
        }
        None if unit.state.order != Order::Idle => {
            unit.state.attack = None;
            unit.state.set_order(Order::Idle, map);
        }
        _ => {}
    }
}

/// Damage of a turret shot at a target (01 §3): ramping on champions, a share of max health on
/// minions (true damage). Updates the turret's heat.
pub fn turret_shot(
    brain: &mut Option<Brain>,
    base: f32,
    target: UnitId,
    target_kind: UnitKind,
    target_range: f32,
    target_max_health: f32,
) -> (f32, bool) {
    let Some(Brain::Tower { heat, last }) = brain.as_mut() else { return (base, false) };
    if target_kind == UnitKind::Minion {
        *heat = 0;
        *last = target;
        return (turret_minion_share(target_range) * target_max_health, true);
    }
    *heat = if *last == target { (*heat + 1).min(TURRET_HEAT_MAX) } else { 0 };
    *last = target;
    (base * (1.0 + TURRET_HEAT_STEP * *heat as f32), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waves_have_a_siege_minion_every_third_and_a_super_when_empowered() {
        assert_eq!(MatchState::wave(0, false).len(), 6);
        assert_eq!(MatchState::wave(2, false).len(), 7);
        assert!(MatchState::wave(5, false).contains(&MinionKind::Siege));
        assert!(!MatchState::wave(4, false).contains(&MinionKind::Super));
        assert_eq!(MatchState::wave(4, true)[0], MinionKind::Super);
        assert_eq!(MatchState::wave(5, true).len(), 8);
        for kind in [MinionKind::Melee, MinionKind::Caster, MinionKind::Siege, MinionKind::Super] {
            assert_eq!(MinionKind::from_attack_range(minion_attack(kind).range), kind);
        }
        assert_eq!(turret_minion_share(minion_attack(MinionKind::Super).range), 0.07);
        assert_eq!(turret_minion_share(minion_attack(MinionKind::Caster).range), 0.70);
        assert_eq!(turret_minion_share(minion_attack(MinionKind::Siege).range), 0.14);
        assert_eq!(turret_minion_share(minion_attack(MinionKind::Melee).range), 0.45);
    }
}
