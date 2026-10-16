//! A team's view of the world this tick (17 §2): every unit it can see, sorted by what it is,
//! and each lane's wave state. Built once per team per tick, only when a bot of that team
//! thinks.

use crate::fight::{self, Power, Side};
use crate::knowledge::Knowledge;
use crate::memory::Memory;
use crate::tier::Tracking;
use mftr_sim::world::{Brain, World};
use mftr_sim::{SimTime, Team, Unit, UnitId, UnitKind, Vec2};

/// One lane as a team sees it, measured along its own path (from its base toward the enemy's).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LaneView {
    /// Our frontmost minion (farthest along), and theirs (the one deepest toward us).
    pub ours: Option<(f32, Vec2)>,
    pub theirs: Option<(f32, Vec2)>,
    pub our_count: u8,
    pub their_count: u8,
    pub our_health: f32,
    pub their_health: f32,
    /// Our frontmost standing turret and their frontmost (nearest us), with how far along.
    pub our_turret: Option<(f32, UnitId)>,
    pub their_turret: Option<(f32, UnitId)>,
    /// Their next structure that can be hurt (unprotected), if any.
    pub their_target: Option<UnitId>,
}

pub struct Board<'w> {
    pub world: &'w World,
    pub team: Team,
    pub now: SimTime,
    /// Seconds into the match.
    pub secs: f32,
    /// Our champions (alive or not).
    pub allies: Vec<&'w Unit>,
    /// Enemy champions in sight.
    pub enemies: Vec<&'w Unit>,
    pub ally_minions: Vec<&'w Unit>,
    /// Enemy lane minions in sight (and Horde's bosses).
    pub enemy_minions: Vec<&'w Unit>,
    /// Jungle monsters in sight.
    pub monsters: Vec<&'w Unit>,
    pub ally_structures: Vec<&'w Unit>,
    pub enemy_structures: Vec<&'w Unit>,
    pub relics: Vec<&'w Unit>,
    /// Enemy wards in sight.
    pub enemy_wards: Vec<&'w Unit>,
    pub lanes: Vec<LaneView>,
    /// Each enemy in sight's power, with its cooldowns as the team tracked them.
    pub enemy_power: Vec<(UnitId, Power)>,
}

impl<'w> Board<'w> {
    pub fn build(world: &'w World, team: Team, memory: &Memory, knowledge: &Knowledge, tracking: Tracking) -> Self {
        let now = SimTime::end_of(world.tick());
        let hidden = world.hidden(team);
        let seen = |u: &Unit| u.team == team || hidden.binary_search(&u.id).is_err();
        let mut b = Board {
            world,
            team,
            now,
            secs: world.match_secs(now),
            allies: Vec::with_capacity(5),
            enemies: Vec::with_capacity(5),
            ally_minions: Vec::new(),
            enemy_minions: Vec::new(),
            monsters: Vec::new(),
            ally_structures: Vec::new(),
            enemy_structures: Vec::new(),
            relics: Vec::new(),
            enemy_wards: Vec::new(),
            lanes: vec![LaneView::default(); knowledge.lanes.len()],
            enemy_power: Vec::with_capacity(5),
        };
        for u in world.units() {
            let ours = u.team == team;
            match u.kind {
                UnitKind::Champion if ours => b.allies.push(u),
                _ if !u.state.alive() || !seen(u) => {}
                UnitKind::Champion => b.enemies.push(u),
                UnitKind::Minion if ours => b.ally_minions.push(u),
                UnitKind::Minion => b.enemy_minions.push(u),
                UnitKind::Monster if u.team == Team::Neutral => b.monsters.push(u),
                UnitKind::Monster if !ours => b.enemy_minions.push(u),
                UnitKind::Relic => b.relics.push(u),
                UnitKind::Ward if !ours => b.enemy_wards.push(u),
                k if k.is_structure() && ours => b.ally_structures.push(u),
                k if k.is_structure() => b.enemy_structures.push(u),
                _ => {}
            }
        }
        for e in &b.enemies {
            let p = fight::power(e, now, false, |slot| memory.ready(e, slot, now, tracking));
            b.enemy_power.push((e.id, p));
        }
        // Lanes.
        for (li, view) in b.lanes.iter_mut().enumerate() {
            let along = |p: Vec2| knowledge.along(li, team, p).0;
            let lane_of = |u: &Unit| matches!(u.brain, Some(Brain::Laner { lane, .. }) if lane as usize == li);
            for m in b.ally_minions.iter().filter(|m| lane_of(m)) {
                let d = along(m.state.pos);
                view.our_count += 1;
                view.our_health += m.state.health;
                if view.ours.is_none_or(|(f, _)| d > f) {
                    view.ours = Some((d, m.state.pos));
                }
            }
            for m in b.enemy_minions.iter().filter(|m| lane_of(m)) {
                let d = along(m.state.pos);
                view.their_count += 1;
                view.their_health += m.state.health;
                if view.theirs.is_none_or(|(f, _)| d < f) {
                    view.theirs = Some((d, m.state.pos));
                }
            }
            for t in b.ally_structures.iter().filter(|t| t.kind == UnitKind::Turret && t.lane as usize == li) {
                let d = along(t.state.pos);
                if view.our_turret.is_none_or(|(f, _)| d > f) {
                    view.our_turret = Some((d, t.id));
                }
            }
            for t in b.enemy_structures.iter().filter(|t| t.lane as usize == li && t.kind != UnitKind::Relic) {
                let d = along(t.state.pos);
                if t.kind == UnitKind::Turret && view.their_turret.is_none_or(|(f, _)| d < f) {
                    view.their_turret = Some((d, t.id));
                }
            }
            view.their_target = b
                .enemy_structures
                .iter()
                .filter(|t| t.targetable() && t.kind != UnitKind::Relic && (t.lane as usize == li || t.tier >= 5))
                .min_by(|a, c| along(a.state.pos).total_cmp(&along(c.state.pos)))
                .map(|t| t.id);
        }
        b
    }

    pub fn unit(&self, id: UnitId) -> Option<&'w Unit> {
        self.world.unit(id)
    }

    /// An enemy in sight's power (cooldowns as tracked).
    pub fn enemy_power(&self, id: UnitId) -> Option<&Power> {
        self.enemy_power.iter().find(|(i, _)| *i == id).map(|(_, p)| p)
    }

    /// Enemy structures that shoot (turrets) covering `p`, with their range.
    pub fn enemy_turrets_covering(&self, p: Vec2, margin: f32) -> impl Iterator<Item = &'w Unit> + '_ {
        self.enemy_structures.iter().copied().filter(move |t| {
            t.kind == UnitKind::Turret && t.attack.is_some_and(|a| t.state.pos.distance(p) <= a.range + margin)
        })
    }

    pub fn ally_turrets_covering(&self, p: Vec2, margin: f32) -> impl Iterator<Item = &'w Unit> + '_ {
        self.ally_structures.iter().copied().filter(move |t| {
            t.kind == UnitKind::Turret && t.attack.is_some_and(|a| t.state.pos.distance(p) <= a.range + margin)
        })
    }

    /// Sides of an engagement at `at`: our champions and the enemies we see within `radius`,
    /// the turrets that would shoot, a share of the minions there turning on champions, and
    /// (with `memory`) the missing enemies that could join, by how likely.
    pub fn engagement(&self, at: Vec2, radius: f32, memory: Option<&Memory>) -> (Side, Side) {
        let (mut us, mut them) = (Side::default(), Side::default());
        for a in self.allies.iter().filter(|a| a.state.alive() && a.state.pos.distance(at) <= radius) {
            us.add(&fight::power(a, self.now, true, |slot| a.state.can_cast(self.now, slot)), 1.0);
        }
        for (id, p) in &self.enemy_power {
            if self.unit(*id).is_some_and(|e| e.state.pos.distance(at) <= radius) {
                them.add(p, 1.0);
            }
        }
        if let Some(memory) = memory {
            for (s, reach) in memory.missing(self.now) {
                let d = s.pos.distance(at);
                if d > reach + radius {
                    continue;
                }
                let share =
                    if d < radius && reach < 600.0 { 0.9 } else { ((radius / reach.max(1.0)).powi(2)).clamp(0.1, 0.5) };
                them.add(&s.power, share);
            }
        }
        for t in self.enemy_turrets_covering(at, 200.0) {
            them.add_support(fight::turret_power(t).dps);
        }
        for t in self.ally_turrets_covering(at, 200.0) {
            us.add_support(fight::turret_power(t).dps);
        }
        let minion_dps = |ms: &[&Unit]| {
            ms.iter()
                .filter(|m| m.state.pos.distance(at) <= radius.min(900.0))
                .map(|m| m.stats.attack_damage * m.attack.map_or(0.0, |a| a.attack_speed))
                .sum::<f32>()
        };
        // Lane minions mostly hit minions; they turn on a champion that hits theirs.
        let share = if self.world.rules().horde { 0.8 } else { 0.25 };
        us.add_support(fight::Dmg { phys: minion_dps(&self.ally_minions) * share, ..Default::default() });
        them.add_support(fight::Dmg { phys: minion_dps(&self.enemy_minions) * share, ..Default::default() });
        if self.world.rules().horde {
            // The horde's minions are the enemy: their health counts like champions'.
            for m in self.enemy_minions.iter().filter(|m| m.state.pos.distance(at) <= radius) {
                them.hp += m.state.health;
                them.champions += 0.15;
            }
        }
        (us, them)
    }
}
