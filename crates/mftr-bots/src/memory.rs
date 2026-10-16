//! A team's memory (17 §2): what its bots have seen, fed after every tick with the team's own
//! fog and the events it could have witnessed. Everything a bot knows of the enemy comes from
//! here or from units its team sees right now; nothing reads a hidden unit.

use crate::fight::Power;
use crate::knowledge::Knowledge;
use crate::tier::Tracking;
use mftr_sim::ability::SLOTS;
use mftr_sim::items::INVENTORY;
use mftr_sim::vision::Vision;
use mftr_sim::world::{Brain, World};
use mftr_sim::{ChampionId, SimEvent, SimTime, Team, Unit, UnitId, UnitKind, Vec2};

/// How long damage is remembered (who's hitting whom: focus, incoming damage).
const DAMAGE_MEMORY_S: f32 = 3.0;

/// What a team knows of one enemy champion.
#[derive(Clone, Debug, PartialEq)]
pub struct Sighting {
    pub id: UnitId,
    pub champion: ChampionId,
    /// Where it was last seen, and how it was moving then.
    pub pos: Vec2,
    pub vel: Vec2,
    pub seen_at: SimTime,
    /// Seen this tick.
    pub visible: bool,
    pub health: f32,
    pub max_health: f32,
    pub level: u8,
    pub items: [u8; INVENTORY],
    pub alive: bool,
    /// When it's back (deaths are announced to everyone: the kill feed and scoreboard).
    pub respawn_at: Option<SimTime>,
    /// When each slot was last seen cast (0: never).
    pub cast_at: [SimTime; SLOTS],
    /// Its ability haste when last seen (for cooldowns).
    pub haste: f32,
    /// Its strength when last seen (everything assumed ready), for fights it may join unseen.
    pub power: Power,
}

/// A threat a bot may dodge, by what it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThreatKey {
    Missile(u32),
    Area(u32),
    Beam(u32),
    /// A unit winding up a cast that fires at this instant.
    Windup(UnitId, SimTime),
}

/// A camp as the team last knew it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CampState {
    /// Not seen yet (before the first spawn, everyone knows when it comes).
    Unknown,
    /// Standing when last seen.
    Up,
    /// Down, back at this instant (its death was seen).
    DownUntil(SimTime),
    /// Down when seen, death unseen: back no later than this.
    DownBy(SimTime),
}

#[derive(Clone, Debug)]
pub struct Memory {
    pub team: Team,
    pub enemies: Vec<Sighting>,
    /// Threats seen: key, when first seen, and the wind-up it came from (itself if none), so
    /// a missile, area or beam inherits its cast's sighting and the dodge decided on it.
    threats: Vec<(ThreatKey, SimTime, ThreatKey)>,
    pub camps: Vec<(CampState, SimTime)>,
    /// Enemy wards seen standing: id, where, until when (0: until destroyed).
    pub wards: Vec<(UnitId, Vec2, SimTime)>,
    /// Recent damage the team saw: (source, target, amount, when).
    pub damage: Vec<(UnitId, UnitId, f32, SimTime)>,
    /// Where each enemy champion was when the team last lost sight of it near a lane (for
    /// "enemy missing" calls): id, lane, when.
    pub vanished: Vec<(UnitId, Option<usize>, SimTime)>,
}

impl Memory {
    pub fn new(team: Team) -> Self {
        Memory {
            team,
            enemies: Vec::new(),
            threats: Vec::new(),
            camps: Vec::new(),
            wards: Vec::new(),
            damage: Vec::new(),
            vanished: Vec::new(),
        }
    }

    /// Learn from the tick just simulated: `events` it produced, and `vision`, the team's sight
    /// after it (the world's hidden lists are already updated).
    pub fn observe(&mut self, world: &World, events: &[SimEvent], vision: &Vision, knowledge: &Knowledge) {
        let team = self.team;
        let now = SimTime::end_of(world.tick());
        let hidden = world.hidden(team);
        let visible = |id: UnitId| hidden.binary_search(&id).is_err();
        let map = world.map();
        // Enemy champions.
        for u in world.units().iter().filter(|u| u.kind == UnitKind::Champion && u.team != team) {
            let Some(champion) = u.champion else { continue };
            let i = match self.enemies.iter().position(|s| s.id == u.id) {
                Some(i) => i,
                None => {
                    self.enemies.push(Sighting {
                        id: u.id,
                        champion,
                        pos: u.home,
                        vel: Vec2::ZERO,
                        seen_at: SimTime(0),
                        visible: false,
                        health: u.stats.max_health,
                        max_health: u.stats.max_health,
                        level: u.state.progress.level,
                        items: [0; INVENTORY],
                        alive: true,
                        respawn_at: None,
                        cast_at: [SimTime(0); SLOTS],
                        haste: 0.0,
                        power: Power::default(),
                    });
                    self.enemies.len() - 1
                }
            };
            let s = &mut self.enemies[i];
            let was_visible = s.visible;
            s.visible = u.state.alive() && visible(u.id);
            if s.visible {
                s.pos = u.state.pos;
                s.vel = velocity(u, now);
                s.seen_at = now;
                s.health = u.state.health;
                s.max_health = u.stats.max_health;
                s.level = u.state.progress.level;
                s.items = u.state.progress.items;
                s.haste = u.stats.ability_haste;
                s.power = crate::fight::power(u, now, false, |_| true);
                s.alive = true;
            } else if was_visible && s.alive {
                let lane = knowledge.nearest_lane(s.pos).filter(|(_, d)| *d < 1500.0).map(|(l, _)| l);
                self.vanished.retain(|(id, ..)| *id != u.id);
                self.vanished.push((u.id, lane, now));
            }
        }
        for e in events {
            match *e {
                SimEvent::Died { unit, respawn_at, .. } => {
                    if let Some(s) = self.enemies.iter_mut().find(|s| s.id == unit) {
                        s.alive = false;
                        s.visible = false;
                        s.respawn_at = Some(respawn_at);
                    }
                    self.vanished.retain(|(id, ..)| *id != unit);
                }
                SimEvent::Respawned { unit, pos, at } => {
                    if let Some(s) = self.enemies.iter_mut().find(|s| s.id == unit) {
                        s.alive = true;
                        s.respawn_at = None;
                        s.pos = pos;
                        s.vel = Vec2::ZERO;
                        s.seen_at = at;
                        s.health = s.max_health;
                    }
                }
                SimEvent::CastStarted { unit, slot, at, .. } if visible(unit) => {
                    if let Some(s) = self.enemies.iter_mut().find(|s| s.id == unit) {
                        s.cast_at[slot as usize % SLOTS] = at;
                    }
                }
                SimEvent::Damage { source, target, amount, absorbed, at, .. } => {
                    let ours = |id: UnitId| world.unit(id).is_some_and(|u| u.team == team);
                    if ours(source) || ours(target) || (visible(source) && visible(target)) {
                        self.damage.push((source, target, amount + absorbed, at));
                    }
                }
                _ => {}
            }
        }
        let horizon =
            SimTime(now.0.saturating_sub((DAMAGE_MEMORY_S * mftr_sim::time::SUBTICKS_PER_SECOND as f32) as u64));
        self.damage.retain(|d| d.3 >= horizon);
        self.vanished.retain(|v| now.secs_since(v.2) < 20.0);

        // Threats: when each was first seen (17 §3: a response comes no sooner than this plus
        // the reaction time).
        let enemy = |t: Team| t != team;
        // Each with its caster and when it was fired, to find the wind-up it came from.
        let mut live: Vec<(ThreatKey, Option<(UnitId, SimTime)>)> = Vec::new();
        for m in world.missiles().iter().filter(|m| enemy(m.team)) {
            let k = ThreatKey::Missile(m.id);
            if self.first_seen(k).is_some() || vision.sees(map, m.position_at(now)) {
                live.push((k, Some((m.owner, m.spawn_at))));
            }
        }
        for a in world.areas().iter().filter(|a| enemy(a.team)) {
            let k = ThreatKey::Area(a.id);
            if self.first_seen(k).is_some() || vision.sees(map, a.center) {
                live.push((k, Some((a.owner, a.spawn_at))));
            }
        }
        for c in world.channels().iter().filter(|c| enemy(c.team)) {
            let k = ThreatKey::Beam(c.id);
            if self.first_seen(k).is_some() || visible(c.owner) {
                live.push((k, Some((c.owner, c.spawn_at))));
            }
        }
        for u in world.units().iter().filter(|u| enemy(u.team) && u.kind == UnitKind::Champion && visible(u.id)) {
            if let Some(c) = u.state.cast {
                live.push((ThreatKey::Windup(u.id, c.fire_at), None));
            }
        }
        let tick = mftr_sim::SUBTICKS as u64;
        for (k, origin) in &live {
            if self.first_seen(*k).is_some() {
                continue;
            }
            // Fired from a wind-up we saw: the same threat, seen since then.
            let from = origin.and_then(|(owner, fired)| {
                self.threats.iter().find(
                    |(w, ..)| matches!(*w, ThreatKey::Windup(u, at) if u == owner && at.0.abs_diff(fired.0) <= tick),
                )
            });
            let (seen, root) = from.map_or((now, *k), |(_, seen, root)| (*seen, *root));
            self.threats.push((*k, seen, root));
        }
        self.threats.retain(|(k, ..)| live.iter().any(|(l, _)| l == k));

        // Camps: seen standing or empty; a death seen gives the exact respawn.
        let camps = &map.layout.camps;
        if self.camps.len() != camps.len() {
            self.camps = vec![(CampState::Unknown, SimTime(0)); camps.len()];
        }
        // Before the first spawn, everyone knows when it comes.
        let first = world.game().started_at.plus(mftr_sim::jungle::FIRST_SPAWN) > now;
        for (i, camp) in camps.iter().enumerate() {
            if !vision.sees(map, camp.pos) {
                continue;
            }
            let standing = world
                .units()
                .iter()
                .any(|u| u.state.alive() && matches!(u.brain, Some(Brain::Monster { camp, .. }) if camp as usize == i));
            let next = world.game().camps.get(i).copied().unwrap_or(SimTime(0));
            let state = if standing {
                CampState::Up
            } else {
                let died_now = events.iter().any(|e| {
                    matches!(e, SimEvent::Died { unit, .. }
                        if world.unit(*unit).is_some_and(|u| matches!(u.brain, Some(Brain::Monster { camp, .. }) if camp as usize == i)))
                });
                match self.camps[i].0 {
                    CampState::DownUntil(t) if t > now => CampState::DownUntil(t),
                    _ if died_now || first => CampState::DownUntil(next),
                    CampState::DownBy(t) if t > now => CampState::DownBy(t),
                    _ => CampState::DownBy(now.plus(camp.kind.respawn())),
                }
            };
            self.camps[i] = (state, now);
        }

        // Enemy wards.
        for u in world.units().iter().filter(|u| u.kind == UnitKind::Ward && enemy(u.team) && visible(u.id)) {
            let until = match u.brain {
                Some(Brain::Ward { expires }) => expires,
                _ => SimTime(0),
            };
            if !self.wards.iter().any(|w| w.0 == u.id) {
                self.wards.push((u.id, u.state.pos, until));
            }
        }
        self.wards.retain(|(id, pos, until)| {
            let gone = world.unit(*id).is_none_or(|u| !u.state.alive());
            let seen_gone = gone && vision.sees(map, *pos);
            !seen_gone && (until.0 == 0 || *until > now)
        });
    }

    /// When a threat was first seen (None: not seen, or gone).
    pub fn first_seen(&self, key: ThreatKey) -> Option<SimTime> {
        self.threats.iter().find(|(k, ..)| *k == key).map(|(_, t, _)| *t)
    }

    /// The wind-up a threat came from (itself if none): dodges are decided once per cast.
    pub fn root(&self, key: ThreatKey) -> ThreatKey {
        self.threats.iter().find(|(k, ..)| *k == key).map_or(key, |(_, _, r)| *r)
    }

    pub fn enemy(&self, id: UnitId) -> Option<&Sighting> {
        self.enemies.iter().find(|s| s.id == id)
    }

    /// Whether enemy `id`'s `slot` is up as far as the team can tell: casts it saw, the
    /// cooldown its level and haste allow, and what the tier tracks.
    pub fn ready(&self, u: &Unit, slot: u8, now: SimTime, tracking: Tracking) -> bool {
        let tracked = match tracking {
            Tracking::None => false,
            Tracking::Ultimates => slot == 3,
            Tracking::Abilities => slot < 4,
            Tracking::Everything => true,
        };
        if !tracked {
            return true;
        }
        let Some(s) = self.enemy(u.id) else { return true };
        let at = s.cast_at[slot as usize % SLOTS];
        if at.0 == 0 {
            return true;
        }
        let Some(a) = u.ability(slot) else { return false };
        let base = a.cooldown_at(crate::kit::rank(u, slot, false)).0 as f32;
        let cd = base * 100.0 / (100.0 + s.haste);
        now.0 as f32 >= at.0 as f32 + cd
    }

    /// Enemy champions alive and not in sight, with how far they could be from where they were
    /// last seen by now.
    pub fn missing(&self, now: SimTime) -> impl Iterator<Item = (&Sighting, f32)> {
        self.enemies.iter().filter(|s| s.alive && !s.visible).map(move |s| {
            let since = if s.seen_at.0 == 0 { 600.0 } else { now.secs_since(s.seen_at) };
            (s, (since * 345.0).min(20_000.0))
        })
    }

    /// How many enemy champions could be within `r` of `p` now: those seen there, plus a share
    /// of each missing one by how much of the area it could have reached overlaps (17 §6).
    pub fn danger(&self, p: Vec2, r: f32, now: SimTime) -> f32 {
        let mut n = 0.0;
        for s in self.enemies.iter().filter(|s| s.alive) {
            if s.visible {
                if s.pos.distance(p) <= r {
                    n += 1.0;
                }
                continue;
            }
            let since = if s.seen_at.0 == 0 { 600.0 } else { now.secs_since(s.seen_at) };
            let reach = (since * 345.0).max(1.0);
            let d = s.pos.distance(p);
            if d > reach + r {
                continue;
            }
            // Spread evenly over the area it could have reached: the share inside our circle.
            let share = ((r * r) / (reach * reach)).min(1.0);
            // Fresh sightings near p count fully: it's probably still there.
            n += if d < r && since < 3.0 { 1.0 } else { share.max(0.15) };
        }
        n
    }

    /// Damage `target` took in the last `secs` (how hard it's being focused).
    pub fn damage_on(&self, target: UnitId, secs: f32, now: SimTime) -> f32 {
        self.damage.iter().filter(|d| d.1 == target && now.secs_since(d.3) <= secs).map(|d| d.2).sum()
    }

    /// Who's been hitting `target` lately.
    pub fn attackers_of(&self, target: UnitId, secs: f32, now: SimTime) -> impl Iterator<Item = UnitId> + '_ {
        self.damage.iter().filter(move |d| d.1 == target && now.secs_since(d.3) <= secs).map(|d| d.0)
    }
}

/// How a unit is moving: toward its next waypoint at its current speed.
pub fn velocity(u: &Unit, now: SimTime) -> Vec2 {
    if let Some(d) = u.state.dash {
        return d.dir * d.speed;
    }
    if !u.state.can_move(now) {
        return Vec2::ZERO;
    }
    match u.state.heading() {
        Some(h) => (h - u.state.pos).normalize_or_zero() * u.state.speed_at(now),
        None => Vec2::ZERO,
    }
}
