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
/// Turret shot damage (D53, the reference ARAM's outer turrets): 185 at the start, growing with
/// every wave (~9 a minute) to 293 after 12 minutes.
pub const TURRET_DAMAGE: f32 = 185.0;
pub const TURRET_DAMAGE_PER_WAVE: f32 = 4.5;
pub const TURRET_DAMAGE_MAX: f32 = 293.0;
/// Consecutive turret shots on champions hit harder: +50% each, up to +150%. The heat cools
/// 5 s after the last shot at a champion; switching champions keeps it, minions don't touch it.
pub const TURRET_HEAT_STEP: f32 = 0.5;
pub const TURRET_HEAT_MAX: u8 = 3;
pub const TURRET_HEAT_COOL: SimDuration = SimDuration::from_millis(5000);
/// Waves (D55, the reference ARAM's pacing): the first at 0:50, then every 25 s, the interval
/// shrinking from 15:00 to 13 s at 25:00, so a lead can close a match out.
pub const FIRST_WAVE: SimDuration = SimDuration::from_millis(50_000);
pub const WAVE_INTERVAL: SimDuration = SimDuration::from_millis(25_000);
pub const WAVE_INTERVAL_LATE: SimDuration = SimDuration::from_millis(13_000);
pub const WAVES_FASTER_FROM_S: f32 = 15.0 * 60.0;
pub const WAVES_FASTEST_AT_S: f32 = 25.0 * 60.0;

/// The time to the next wave, `secs` into the match.
pub fn wave_interval(secs: f32) -> SimDuration {
    let f = ((secs - WAVES_FASTER_FROM_S) / (WAVES_FASTEST_AT_S - WAVES_FASTER_FROM_S)).clamp(0.0, 1.0);
    let ms = WAVE_INTERVAL.0 as f32 + (WAVE_INTERVAL_LATE.0 as f32 - WAVE_INTERVAL.0 as f32) * f;
    SimDuration(ms.round() as u64)
}

/// How a lane map paces its waves: The Bridge like the reference ARAM (D55), Crossroads like
/// the reference 5v5 (the first wave at 1:05, then every 30 s; a siege minion every third wave,
/// every second from 15:00 and in every wave from 25:00; minion upgrades every 90 s).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pacing {
    #[default]
    Aram,
    Classic,
}

pub const CLASSIC_FIRST_WAVE: SimDuration = SimDuration::from_millis(65_000);
pub const CLASSIC_WAVE_INTERVAL: SimDuration = SimDuration::from_millis(30_000);
pub const CLASSIC_UPGRADE_EVERY_S: f32 = 90.0;

impl Pacing {
    /// The first wave, after the match starts.
    pub fn first_wave(self) -> SimDuration {
        match self {
            Pacing::Aram => FIRST_WAVE,
            Pacing::Classic => CLASSIC_FIRST_WAVE,
        }
    }

    /// The time to the next wave, `secs` into the match.
    pub fn interval(self, secs: f32) -> SimDuration {
        match self {
            Pacing::Aram => wave_interval(secs),
            Pacing::Classic => CLASSIC_WAVE_INTERVAL,
        }
    }

    /// Upgrades a minion spawned `secs` into the match has.
    pub fn upgrades(self, secs: f32) -> u32 {
        match self {
            Pacing::Aram => minion_upgrades(secs),
            Pacing::Classic => ((secs.max(0.0) / CLASSIC_UPGRADE_EVERY_S) as u32).min(MINION_UPGRADES_MAX),
        }
    }

    /// Whether wave `n` (0-based), spawning `secs` into the match, brings a siege minion.
    pub fn siege(self, n: u32, secs: f32) -> bool {
        match self {
            Pacing::Aram => n >= 2 && n.is_multiple_of(2),
            Pacing::Classic if secs >= WAVES_FASTEST_AT_S => true,
            Pacing::Classic if secs >= WAVES_FASTER_FROM_S => n.is_multiple_of(2),
            Pacing::Classic => n % 3 == 2,
        }
    }
}

/// Minions grow stronger every 50 s of the match (up to 30 upgrades): health and damage per
/// upgrade, by kind.
pub const MINION_UPGRADE_EVERY_S: f32 = 50.0;
pub const MINION_UPGRADES_MAX: u32 = 30;

pub fn minion_upgrade(kind: MinionKind) -> (f32, f32) {
    match kind {
        MinionKind::Melee => (22.0, 0.6),
        MinionKind::Caster => (9.0, 1.5),
        MinionKind::Siege => (25.0, 1.5),
        MinionKind::Super => (100.0, 5.0),
    }
}

/// Upgrades a minion spawned `secs` into the match has.
pub fn minion_upgrades(secs: f32) -> u32 {
    ((secs.max(0.0) / MINION_UPGRADE_EVERY_S) as u32).min(MINION_UPGRADES_MAX)
}

/// Minions walk faster as the match goes on: +25 at 10, 15, 20 and 25 minutes (325 → 425).
pub fn minion_speed(secs: f32) -> f32 {
    let steps = ((secs / 60.0 - 5.0) / 5.0).floor().clamp(0.0, 4.0);
    crate::world::MINION_MOVE_SPEED + 25.0 * steps
}

/// Champions hit structures harder as the match goes on: +0% at 5:00 to +25% at 20:00.
pub const STRUCTURE_AMP_MAX: f32 = 0.25;

pub fn structure_amp(secs: f32) -> f32 {
    1.0 + STRUCTURE_AMP_MAX * ((secs / 60.0 - 5.0) / 15.0).clamp(0.0, 1.0)
}
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
    attack_damage: TURRET_DAMAGE,
    ..Stats::NONE
};

/// A turret's shot damage once `waves` waves have spawned.
pub fn turret_damage(waves: u32) -> f32 {
    (TURRET_DAMAGE + TURRET_DAMAGE_PER_WAVE * waves as f32).min(TURRET_DAMAGE_MAX)
}
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
    /// When the match began (the clock, waves and scaling count from it).
    pub started_at: SimTime,
    /// Recent champion-on-champion attacks: (attacker, victim, at).
    pub aggression: Vec<(UnitId, UnitId, SimTime)>,
    /// Set once a Base falls.
    pub winner: Option<(Team, SimTime)>,
    /// Per jungle camp (`Layout::camps`): when it spawns next, or 0 while it stands.
    pub camps: Vec<SimTime>,
}

impl MatchState {
    pub fn hash_into(&self, h: &mut impl StateSink) {
        h.write_u64(self.next_wave_at.map_or(u64::MAX, |t| t.0));
        h.write_u32(self.waves_spawned);
        h.write_u64(self.started_at.0);
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
        // Maps without a jungle hash as before.
        for t in &self.camps {
            h.write_u64(t.0);
        }
    }

    /// Minions of an ARAM wave `n` (0-based): 3 melee, 3 casters, a siege minion every second
    /// wave from the third, and a super minion in front while the team has an enemy Gatehouse
    /// down (`empowered`).
    pub fn wave(n: u32, empowered: bool) -> Vec<MinionKind> {
        Self::wave_of(Pacing::Aram.siege(n, 0.0), empowered)
    }

    /// A wave: 3 melee, a siege minion if `siege`, 3 casters, and a super minion in front if
    /// `empowered`.
    pub fn wave_of(siege: bool, empowered: bool) -> Vec<MinionKind> {
        let mut w = if empowered { vec![MinionKind::Super] } else { Vec::new() };
        w.extend([MinionKind::Melee; 3]);
        if siege {
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

/// Structures still standing protect the ones behind them (01 §3), lane by lane: tiers 1–4
/// (outer, inner and gatehouse turrets, the Gatehouse) fall in order down their own lane; the
/// base turrets (5) can be hurt once one of the team's Gatehouses is down; the Base (6) once
/// both base turrets are down too. Recomputed every tick.
pub fn update_protection(units: &mut [Unit]) {
    let standing: Vec<(Team, u8, u8)> =
        units.iter().filter(|u| u.tier > 0 && u.state.alive()).map(|u| (u.team, u.lane, u.tier)).collect();
    // A Gatehouse is down: one has fallen (and not yet respawned), or none stands at all.
    let gate_down = |team: Team| {
        let gates = units.iter().filter(|u| u.team == team && u.tier == GATEHOUSE_TIER);
        gates.clone().any(|u| !u.state.alive()) || !gates.clone().any(|u| u.state.alive())
    };
    let down = [gate_down(Team::Blue), gate_down(Team::Red)];
    for u in units.iter_mut().filter(|u| u.tier > 0) {
        let (team, lane, tier) = (u.team, u.lane, u.tier);
        u.protected = match tier {
            1..=GATEHOUSE_TIER => standing.iter().any(|&(t, l, x)| t == team && l == lane && x < tier),
            BASE_TURRET_TIER => !down[team as usize],
            _ => !down[team as usize] || standing.iter().any(|&(t, _, x)| t == team && x == BASE_TURRET_TIER),
        };
    }
}

/// Structure tiers (`Placement::tier`).
pub const GATEHOUSE_TIER: u8 = 4;
pub const BASE_TURRET_TIER: u8 = 5;

/// Turret plating (01 §3, maps with `Layout::plating`): outer turrets carry 5 plates until
/// 14:00. Each 20% of health lost breaks one (the last falls with the turret), and each pays
/// 125 gold, split among the enemy champions near the turret.
pub const PLATES: u8 = 5;
pub const PLATE_GOLD: f32 = 125.0;
pub const PLATING_FALLS_S: f32 = 14.0 * 60.0;

/// Plates fall off every turret at 14:00 (their health stays). Every tick.
pub fn update_plating(units: &mut [Unit], secs: f32) {
    if secs >= PLATING_FALLS_S {
        for u in units.iter_mut().filter(|u| u.plates > 0) {
            u.plates = 0;
        }
    }
}

/// The plates a turret with `health` of `max` has left.
pub fn plates_left(health: f32, max: f32) -> u8 {
    ((health.max(0.0) / max.max(1.0)) * PLATES as f32).ceil().min(PLATES as f32) as u8
}

/// Backdoor protection (01 §3, maps with `Layout::backdoor`): a structure with none of the
/// attacker's minions within this range takes a third of champions' damage.
pub const BACKDOOR_RANGE: f32 = 1000.0;
pub const BACKDOOR_DAMAGE: f32 = 1.0 / 3.0;

/// How much champions' damage hurts each structure now: more late in a match
/// (`structure_amp`), and a third with backdoor protection when `backdoor` and none of the
/// enemy's minions are near it. Every tick.
pub fn update_structure_amp(units: &mut [Unit], secs: f32, backdoor: bool) {
    let amp = structure_amp(secs);
    let minions: Vec<(Team, Vec2)> = if backdoor {
        units.iter().filter(|u| u.kind == UnitKind::Minion && u.state.alive()).map(|u| (u.team, u.state.pos)).collect()
    } else {
        Vec::new()
    };
    for u in units.iter_mut().filter(|u| u.tier > 0) {
        let covered = !backdoor
            || minions
                .iter()
                .any(|&(t, p)| t != u.team && (p - u.state.pos).length_sq() <= BACKDOOR_RANGE * BACKDOOR_RANGE);
        u.champion_damage_taken = if covered { amp } else { amp * BACKDOOR_DAMAGE };
    }
}

/// Lane minions and turrets fight the other team, never jungle monsters or wards.
fn enemy_valid(me: &Unit, s: &Seen, hidden: &[UnitId]) -> bool {
    s.team != me.team
        && !matches!(s.kind, UnitKind::Monster | UnitKind::Ward)
        && s.targetable
        && hidden.binary_search(&s.id).is_err()
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
    let Some(Brain::Laner { lane: which, mut next }) = unit.brain else { return };
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
    unit.brain = Some(Brain::Laner { lane: which, next });
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

/// Damage of a turret shot fired at `t` (01 §3): ramping on champions, a share of max health
/// on minions (true damage). Updates the turret's heat.
pub fn turret_shot(
    brain: &mut Option<Brain>,
    base: f32,
    t: SimTime,
    target_kind: UnitKind,
    target_range: f32,
    target_max_health: f32,
) -> (f32, bool) {
    let Some(Brain::Tower { heat, cools_at }) = brain.as_mut() else { return (base, false) };
    if target_kind == UnitKind::Minion {
        return (turret_minion_share(target_range) * target_max_health, true);
    }
    *heat = if t < *cools_at { (*heat + 1).min(TURRET_HEAT_MAX) } else { 0 };
    *cools_at = t.plus(TURRET_HEAT_COOL);
    (base * (1.0 + TURRET_HEAT_STEP * *heat as f32), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waves_have_a_siege_minion_every_second_from_the_third_and_a_super_when_empowered() {
        assert_eq!(MatchState::wave(0, false).len(), 6);
        assert_eq!(MatchState::wave(1, false).len(), 6);
        assert_eq!(MatchState::wave(2, false).len(), 7);
        assert_eq!(MatchState::wave(3, false).len(), 6);
        assert!(MatchState::wave(4, false).contains(&MinionKind::Siege));
        assert!(!MatchState::wave(4, false).contains(&MinionKind::Super));
        assert_eq!(MatchState::wave(4, true)[0], MinionKind::Super);
        assert_eq!(MatchState::wave(6, true).len(), 8);
        for kind in [MinionKind::Melee, MinionKind::Caster, MinionKind::Siege, MinionKind::Super] {
            assert_eq!(MinionKind::from_attack_range(minion_attack(kind).range), kind);
        }
        assert_eq!(turret_minion_share(minion_attack(MinionKind::Super).range), 0.07);
        assert_eq!(turret_minion_share(minion_attack(MinionKind::Caster).range), 0.70);
    }

    /// D55: the reference ARAM's pacing. Waves every 25 s, 13 s from 25:00; minions upgrade
    /// every 50 s and speed up from 10:00 to 425 at 25:00; structures take up to +25% from
    /// champions by 20:00.
    #[test]
    fn the_pacing_speeds_up_late_in_a_match() {
        let ms = |s: f32| wave_interval(s).0 / crate::time::SUBTICKS_PER_SECOND;
        assert_eq!(ms(0.0), 25);
        assert_eq!(ms(14.0 * 60.0), 25);
        assert_eq!(ms(20.0 * 60.0), 19);
        assert_eq!(ms(30.0 * 60.0), 13);
        assert_eq!(minion_upgrades(49.0), 0);
        assert_eq!(minion_upgrades(500.0), 10);
        assert_eq!(minion_upgrades(99_999.0), MINION_UPGRADES_MAX);
        // Crossroads: the reference 5v5's cadence.
        let c = Pacing::Classic;
        assert_eq!(
            (c.first_wave(), c.interval(0.0), c.interval(2000.0)),
            (CLASSIC_FIRST_WAVE, CLASSIC_WAVE_INTERVAL, CLASSIC_WAVE_INTERVAL)
        );
        assert_eq!((0..6).map(|n| c.siege(n, 300.0)).collect::<Vec<_>>(), [false, false, true, false, false, true]);
        assert!(c.siege(30, 16.0 * 60.0) && !c.siege(31, 16.0 * 60.0) && c.siege(51, 26.0 * 60.0));
        assert_eq!((c.upgrades(89.0), c.upgrades(900.0)), (0, 10));
        assert_eq!(minion_speed(9.0 * 60.0), 325.0);
        assert_eq!(minion_speed(10.0 * 60.0), 350.0);
        assert_eq!(minion_speed(26.0 * 60.0), 425.0);
        assert_eq!(structure_amp(0.0), 1.0);
        assert!((structure_amp(12.5 * 60.0) - 1.125).abs() < 1e-4);
        assert_eq!(structure_amp(40.0 * 60.0), 1.25);
        assert_eq!(turret_minion_share(minion_attack(MinionKind::Siege).range), 0.14);
        assert_eq!(turret_minion_share(minion_attack(MinionKind::Melee).range), 0.45);
    }
}
