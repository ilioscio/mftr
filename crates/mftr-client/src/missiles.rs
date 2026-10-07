//! Missile tracking, display timelines and the ghost-hit measurement (03a §7, 03 §1, §14).
//!
//! Display policy:
//! - **Enemy missiles** are drawn on `T_input`, where the server will judge the collision with
//!   our own (predicted) champion. A predicted hit on us shows the impact immediately; damage
//!   and CC wait for the server.
//! - **Own missiles** spawn from prediction and use Option B (D12): `T_input` at the hand,
//!   blending toward `T_interp` over the flight, so impacts line up with enemies as drawn.
//!   Hits are never predicted, only shown when confirmed.
//! - **Allied missiles** are drawn on `T_interp`, like their casters and targets.
//!
//! Ghost hits: for each enemy missile the client freezes what it *showed* (hit or dodge) when
//! the input timeline passes the resolution point, then compares with the server's verdict.

use mftr_sim::{Missile, SUBTICKS, SimTime, Tick, UnitId, UnitState, Vec2};
use std::collections::BTreeMap;

/// How far past the resolution an impact flash stays visible (seconds).
const IMPACT_SECONDS: f64 = 0.15;
/// Option B: own missiles stay on `T_input` for this distance, then blend toward `T_interp`.
const OWN_BLEND_START: f32 = 300.0;
/// Paths passing within this extra distance of a hit count as near-misses (03 §1 denominator).
const NEAR_MISS_MARGIN: f32 = 60.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Own,
    Ally,
    Enemy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Hit,
    Miss,
    /// Shown as "unconfirmed": an interception by another unit was predicted first (03a §7).
    Uncertain,
}

#[derive(Clone, Copy, Debug)]
pub struct Tracked {
    pub m: Missile,
    pub side: Side,
    /// Server end: when, and which unit it hit (None = expired).
    pub end: Option<(SimTime, Option<UnitId>)>,
    /// Local clock time when we first learned of it.
    pub seen_local: f64,
    /// Enemy missiles only: what the client displayed, frozen at resolution.
    pub shown: Option<Outcome>,
    pub near_miss: bool,
    pub counted: bool,
    pub dodged_by_bot: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MissileRender {
    /// Server id, or `u32::MAX - (cast_seq << 4 | shot)` for an own missile not yet confirmed.
    pub key: u32,
    pub side: Side,
    pub pos: Vec2,
    pub dir: Vec2,
    pub radius: f32,
    /// Showing the impact flash instead of the missile.
    pub impact: bool,
    /// Predicted to be intercepted by another unit: draw dimmed until the server confirms.
    pub unconfirmed: bool,
    /// Stuns or roots on hit: drawn with the shared hard-CC accent (05 §1).
    pub hard_cc: bool,
    /// The caster (for the spawn streak from its drawn hand, 03a §7).
    pub owner: UnitId,
}

/// An enemy missile as the player sees it, for scripted dodgers (dodge rig bot).
#[derive(Clone, Copy, Debug)]
pub struct Threat {
    pub id: u32,
    pub pos: Vec2,
    pub dir: Vec2,
    pub radius: f32,
    pub predicted_hit: Option<SimTime>,
    pub visible_for: f64,
}

#[derive(Clone, Debug, Default)]
pub struct DodgeStats {
    pub enemy_missiles: u64,
    pub near_misses: u64,
    pub server_hits: u64,
    pub shown_hits: u64,
    /// Shown as a dodge, but the server hit us (the number Pillar 1 lives by).
    pub ghost_hits: u64,
    /// Shown as a hit, but the server says miss (favorable, still a mismatch).
    pub phantom_hits: u64,
    /// Resolved while shown as unconfirmed (predicted interception); not judged either way.
    pub uncertain: u64,
    /// Resolved after the server had already killed us with something else (the missile
    /// passed a corpse); not judged either way.
    pub died_first: u64,
}

/// How own missiles are drawn (03a §7, D12; blind A/B test in M1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum OwnMissileDisplay {
    /// Option A: always on `T_input`, honest about where the missile really is.
    InputTimeline,
    /// Option B (default): leaves the hand on `T_input`, blends toward `T_interp` over the
    /// flight so impacts line up with enemies as drawn.
    #[default]
    Blend,
}

#[derive(Default)]
pub struct MissileBook {
    pub tracked: BTreeMap<u32, Tracked>,
    /// Own casts predicted locally, keyed by cast command sequence.
    /// By (cast sequence, shot): Multishot and Echo fire several missiles per cast.
    pub predicted_own: BTreeMap<(u32, u8), Missile>,
    pub stats: DodgeStats,
    pub own_display: OwnMissileDisplay,
}

/// Fractional sub-tick instant on a display timeline given in fractional ticks.
fn st(ticks: f64) -> f64 {
    ticks * SUBTICKS as f64
}

fn pos_at(m: &Missile, at_st: f64) -> Vec2 {
    let secs = ((at_st - m.spawn_at.0 as f64) / (SUBTICKS as f64 * 30.0)).max(0.0) as f32;
    m.origin + m.dir * (m.spec.speed * secs.min(m.spec.range / m.spec.speed))
}

impl MissileBook {
    pub fn on_spawn(&mut self, m: Missile, own_unit: UnitId, own_team: mftr_sim::Team, now: f64) {
        let side = if m.owner == own_unit {
            self.predicted_own.remove(&(m.cast_seq, m.shot));
            Side::Own
        } else if m.team == own_team {
            Side::Ally
        } else {
            self.stats.enemy_missiles += 1;
            Side::Enemy
        };
        self.tracked.insert(
            m.id,
            Tracked {
                m,
                side,
                end: None,
                seen_local: now,
                shown: None,
                near_miss: false,
                counted: false,
                dodged_by_bot: false,
            },
        );
    }

    pub fn on_end(&mut self, id: u32, at: SimTime, target: Option<UnitId>) {
        if let Some(t) = self.tracked.get_mut(&id) {
            t.end = Some((at, target));
        }
    }

    /// Earliest predicted contact between an enemy missile and our predicted path, using the
    /// server's exact rule (`Missile::first_hit`) over the ticks we have predicted.
    pub fn predicted_self_hit(
        m: &Missile,
        own_radius: f32,
        history: &dyn Fn(Tick) -> Option<UnitState>,
        last: Tick,
        margin: f32,
    ) -> Option<SimTime> {
        Self::predicted_self_hit_in(m, own_radius, history, last, margin).map(|(_, at)| at)
    }

    /// `predicted_self_hit`, with the tick whose step judges it.
    fn predicted_self_hit_in(
        m: &Missile,
        own_radius: f32,
        history: &dyn Fn(Tick) -> Option<UnitState>,
        last: Tick,
        margin: f32,
    ) -> Option<(Tick, SimTime)> {
        let first = Tick((m.spawn_at.0 / SUBTICKS as u64) as u32 + 1);
        let end = m.end_at();
        let mut k = first;
        while k <= last {
            let s0 = SimTime::end_of(Tick(k.0 - 1));
            let s1 = SimTime::end_of(k);
            if s0 >= end {
                break;
            }
            if let (Some(a), Some(b)) = (history(Tick(k.0 - 1)), history(k)) {
                let lo = m.spawn_at.max(s0);
                let hi = end.min(s1);
                if let Some(at) = m.first_hit(lo, hi, s0, a.pos, b.pos, own_radius + margin) {
                    return Some((k, at));
                }
            }
            k = k.next();
        }
        None
    }

    /// Predicted hits on us, one per enemy volley (Multishot): the server lets a volley hit
    /// each unit once, so the missile it steps first into us takes the hit (the earliest tick,
    /// then spawn order) and the rest of the volley passes through. A missile predicted to be
    /// intercepted by another unit first doesn't take it.
    fn self_hits(
        &self,
        own_radius: f32,
        history: &dyn Fn(Tick) -> Option<UnitState>,
        last: Tick,
        interceptions: &BTreeMap<u32, SimTime>,
    ) -> BTreeMap<u32, SimTime> {
        let mut first: BTreeMap<(UnitId, u32, bool), (Tick, u32, SimTime)> = BTreeMap::new();
        for (id, t) in self.tracked.iter().filter(|(_, t)| t.side == Side::Enemy) {
            if let Some((k, at)) = Self::predicted_self_hit_in(&t.m, own_radius, history, last, 0.0)
                && interceptions.get(id).is_none_or(|i| *i >= at)
            {
                let e = first.entry(t.m.volley()).or_insert((k, *id, at));
                if (k, *id) < (e.0, e.1) {
                    *e = (k, *id, at);
                }
            }
        }
        first.into_values().map(|(_, id, at)| (id, at)).collect()
    }

    /// Freeze displayed outcomes and classify against the server (ghost-hit metric).
    /// `interceptions`: per enemy missile, the earliest predicted contact with another unit of
    /// our team (from collision proxies). If it comes before our own predicted hit, the missile
    /// is shown as unconfirmed instead of hitting us.
    pub fn update_outcomes(
        &mut self,
        t_input: f64,
        own: UnitId,
        own_radius: f32,
        history: &dyn Fn(Tick) -> Option<UnitState>,
        last: Tick,
        interceptions: &BTreeMap<u32, SimTime>,
    ) {
        let now_st = st(t_input);
        let hits = self.self_hits(own_radius, history, last, interceptions);
        for (id, t) in self.tracked.iter_mut().filter(|(_, t)| t.side == Side::Enemy) {
            if t.shown.is_none() {
                let hit = hits.get(id).copied();
                // The rest of a volley that already hit us passes through: not a near-miss.
                let passes = hit.is_none() && Self::predicted_self_hit(&t.m, own_radius, history, last, 0.0).is_some();
                let icpt = interceptions.get(id).copied().filter(|i| hit.is_none_or(|h| *i < h));
                let resolve_at = [hit, icpt, t.end.map(|(e, _)| e)].into_iter().flatten().min().unwrap_or(t.m.end_at());
                if now_st >= resolve_at.0 as f64 {
                    t.shown = Some(match (icpt, hit) {
                        (Some(_), _) => Outcome::Uncertain,
                        (None, Some(_)) => Outcome::Hit,
                        (None, None) => Outcome::Miss,
                    });
                    t.near_miss = !passes
                        && Self::predicted_self_hit(&t.m, own_radius, history, last, NEAR_MISS_MARGIN).is_some();
                }
            }
            if let (Some(shown), Some((_, target)), false) = (t.shown, t.end, t.counted) {
                t.counted = true;
                let server_hit = target == Some(own);
                let s = &mut self.stats;
                s.server_hits += server_hit as u64;
                if shown == Outcome::Uncertain {
                    s.uncertain += 1;
                    continue;
                }
                // Our history is authoritative up to the latest snapshot: were we already dead?
                let end_tick = Tick((t.end.map_or(t.m.end_at(), |(e, _)| e).0 / SUBTICKS as u64) as u32 + 1);
                if !server_hit && history(end_tick).is_some_and(|h| !h.alive()) {
                    s.died_first += 1;
                    continue;
                }
                s.near_misses += t.near_miss as u64;
                s.shown_hits += (shown == Outcome::Hit) as u64;
                match (shown, server_hit) {
                    (Outcome::Miss, true) => s.ghost_hits += 1,
                    (Outcome::Hit, false) => s.phantom_hits += 1,
                    _ => {}
                }
            }
        }
        // Forget missiles a second after they're gone from every timeline (enemy ones only
        // once their outcome is counted).
        self.tracked.retain(|_, t| {
            let end = t.end.map(|(e, _)| e).unwrap_or(t.m.end_at());
            now_st < end.0 as f64 + st(30.0) || (t.side == Side::Enemy && !t.counted)
        });
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &self,
        t_input: f64,
        t_interp: f64,
        own_radius: f32,
        history: &dyn Fn(Tick) -> Option<UnitState>,
        last: Tick,
        interceptions: &BTreeMap<u32, SimTime>,
    ) -> Vec<MissileRender> {
        let mut out = Vec::new();
        let impact_st = st(IMPACT_SECONDS * 30.0);
        let mut push = |key: u32, m: &Missile, side: Side, at: f64, end: Option<(SimTime, bool)>, unconfirmed: bool| {
            if at < m.spawn_at.0 as f64 {
                return;
            }
            let (end_at, is_hit) = end.unwrap_or((m.end_at(), false));
            if at >= end_at.0 as f64 {
                if is_hit && at < end_at.0 as f64 + impact_st {
                    out.push(MissileRender {
                        key,
                        side,
                        pos: pos_at(m, end_at.0 as f64),
                        dir: m.dir,
                        radius: m.spec.radius,
                        impact: true,
                        unconfirmed: false,
                        hard_cc: m.spec.cc.is_hard(),
                        owner: m.owner,
                    });
                }
                return;
            }
            let (pos, dir, radius) = (pos_at(m, at), m.dir, m.spec.radius);
            let hard_cc = m.spec.cc.is_hard();
            let owner = m.owner;
            out.push(MissileRender { key, side, pos, dir, radius, impact: false, unconfirmed, hard_cc, owner });
        };
        let hits = self.self_hits(own_radius, history, last, interceptions);
        let option = self.own_display;
        let own_display = |m: &Missile| {
            if option == OwnMissileDisplay::InputTimeline {
                return st(t_input);
            }
            let traveled =
                ((st(t_input) - m.spawn_at.0 as f64) / (SUBTICKS as f64 * 30.0)).max(0.0) as f32 * m.spec.speed;
            let w = ((traveled - OWN_BLEND_START) / (m.spec.range - OWN_BLEND_START).max(1.0)).clamp(0.0, 1.0) as f64;
            st(t_input - w * (t_input - t_interp))
        };
        for (&(cast_seq, shot), m) in &self.predicted_own {
            push(u32::MAX - (cast_seq.wrapping_shl(4) | shot as u32), m, Side::Own, own_display(m), None, false);
        }
        for (id, t) in &self.tracked {
            let server_end = t.end.map(|(e, tg)| (e, tg.is_some()));
            match t.side {
                Side::Own => push(*id, &t.m, Side::Own, own_display(&t.m), server_end, false),
                Side::Ally => push(*id, &t.m, Side::Ally, st(t_interp), server_end, false),
                Side::Enemy => {
                    let at = st(t_input);
                    let predicted = hits.get(id).copied();
                    let icpt = interceptions.get(id).copied().filter(|i| predicted.is_none_or(|h| *i < h));
                    match icpt {
                        // Predicted to hit someone else first: never hide the threat on a guess.
                        // Keep it moving, dimmed, until the server says what happened.
                        Some(i) => push(*id, &t.m, Side::Enemy, at, server_end, at >= i.0 as f64),
                        // Predicted hit on us ends it on our timeline; otherwise the server's end.
                        None => {
                            let end = predicted.map(|p| (p, true)).or(server_end);
                            push(*id, &t.m, Side::Enemy, at, end, false)
                        }
                    }
                }
            }
        }
        out
    }

    pub fn threats(
        &self,
        t_input: f64,
        now_local: f64,
        own_radius: f32,
        history: &dyn Fn(Tick) -> Option<UnitState>,
        last: Tick,
    ) -> Vec<Threat> {
        let hits = self.self_hits(own_radius, history, last, &BTreeMap::new());
        self.tracked
            .iter()
            .filter(|(_, t)| t.side == Side::Enemy && t.end.is_none() && st(t_input) < t.m.end_at().0 as f64)
            .map(|(id, t)| Threat {
                id: *id,
                pos: pos_at(&t.m, st(t_input)),
                dir: t.m.dir,
                radius: t.m.spec.radius,
                predicted_hit: hits.get(id).copied(),
                visible_for: now_local - t.seen_local,
            })
            .collect()
    }

    /// Keep own-cast predictions spawned up to `keep_through` (later ones are re-created by
    /// re-simulation) and not older than `stale_before` (the server never confirmed them:
    /// a rejected cast).
    pub fn prune_predicted(&mut self, keep_through: SimTime, stale_before: SimTime) {
        self.predicted_own.retain(|_, m| m.spawn_at <= keep_through && m.spawn_at >= stale_before);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::ability::{Cc, Damage, DamageKind, LineSkillshot};
    use mftr_sim::{Team, UnitState};

    /// Two missiles of one Multishot volley cross our hitbox one after the other (the second
    /// spawned a tick later, so both pass through us): the server lets the volley hit us once,
    /// and so does the display. An echo is a volley of its own.
    #[test]
    fn a_volley_hits_us_once() {
        let me = UnitId(1);
        let spec = LineSkillshot {
            windup: mftr_sim::SimDuration(0),
            speed: 1500.0,
            radius: 40.0,
            range: 1500.0,
            damage: Damage { kind: DamageKind::Magic, base: 50.0, ad_ratio: 0.0, ap_ratio: 0.0 },
            cc: Cc::None,
        };
        let missile = |id: u32, shot: u8, tick: u64| Missile {
            id,
            owner: UnitId(9),
            team: Team::Red,
            origin: Vec2::new(0.0, 0.0),
            dir: Vec2::new(1.0, 0.0),
            spec,
            spawn_at: SimTime(tick * SUBTICKS as u64),
            cast_seq: 7,
            power: 50.0,
            shot,
        };
        let mut book = MissileBook::default();
        for (id, shot, tick) in [(10, 0, 1), (11, 1, 2), (12, mftr_sim::augments::ECHO_SHOT, 3)] {
            book.on_spawn(missile(id, shot, tick), me, Team::Blue, 0.0);
        }
        let standing = UnitState::new(Vec2::new(600.0, 0.0), 325.0);
        let history = |_: Tick| Some(standing);
        let hits = book.self_hits(65.0, &history, Tick(60), &BTreeMap::new());
        assert_eq!(hits.keys().copied().collect::<Vec<_>>(), vec![10, 12], "the first missile and the echo");
        // Predicted to hit an ally first, the first missile leaves the hit to the next one.
        let icpt = BTreeMap::from([(10, SimTime(SUBTICKS as u64))]);
        let hits = book.self_hits(65.0, &history, Tick(60), &icpt);
        assert_eq!(hits.keys().copied().collect::<Vec<_>>(), vec![11, 12]);
    }
}
