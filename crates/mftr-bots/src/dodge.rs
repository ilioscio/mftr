//! Dodging (17 §7.3): enemy areas (seen from their telegraph), beams (their aim is shown, then
//! the channel), missiles in sight and line wind-ups. For each, how soon it hits this bot and
//! where to step to be out of it.

use crate::kit;
use crate::memory::{Memory, ThreatKey};
use mftr_sim::ability::{Cc, Effect};
use mftr_sim::map::Map;
use mftr_sim::{SimTime, Unit, Vec2, World};

/// Room left around a dodge (u).
const MARGIN: f32 = 45.0;
/// Clearance past the edge that's enough to be missed.
const TIGHT: f32 = 10.0;

/// Something about to hit a bot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Threat {
    pub key: ThreatKey,
    /// The cast it came from (its wind-up's key), so a dodge is decided once per cast.
    pub root: ThreatKey,
    /// When the team first saw it.
    pub seen_at: SimTime,
    /// Seconds until it hits.
    pub hits_in: f32,
    /// Where to step to be clear of it.
    pub to: Vec2,
    /// How far that is.
    pub step: f32,
    /// It stuns, roots, knocks up or pulls.
    pub hard: bool,
}

fn secs(d: mftr_sim::SimDuration) -> f32 {
    d.0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32
}

/// Out of a line through `origin` along `dir` of half-width `half`: sideways, to the side `me`
/// is already on (or `prefer`'s side when on the line), or the other when that's a wall.
fn off_line(map: &Map, me: &Unit, origin: Vec2, dir: Vec2, half: f32, prefer: Vec2) -> (Vec2, f32) {
    let p = me.state.pos;
    let rel = p - origin;
    let side = rel - dir * rel.dot(dir);
    let perp = Vec2::new(-dir.y, dir.x);
    let lateral = side.length();
    let need_same = half + me.gameplay_radius + MARGIN - lateral;
    let need_other = half + me.gameplay_radius + MARGIN + lateral;
    let same = if lateral > 1.0 {
        side * (1.0 / lateral)
    } else if perp.dot(prefer) >= 0.0 {
        perp
    } else {
        -perp
    };
    let a = p + same * need_same.max(0.0);
    if map.walkable(a, 30.0) {
        return (a, need_same.max(0.0));
    }
    let b = p - same * need_other;
    (b, need_other)
}

/// Out of a circle at `center`: straight out, or along `prefer` when that also leaves it sooner
/// than walking into a wall.
fn out_of_circle(map: &Map, me: &Unit, center: Vec2, radius: f32, prefer: Vec2) -> (Vec2, f32) {
    let p = me.state.pos;
    let rel = p - center;
    let need = radius + me.gameplay_radius + MARGIN;
    let out = if rel.length() > 1.0 { rel.normalize_or_zero() } else { prefer.normalize_or_zero() };
    let mut best: Option<(Vec2, f32)> = None;
    // Straight out, and a few turns of it toward `prefer`.
    for turn in [0.0f32, 0.6, -0.6, 1.2, -1.2] {
        let (s, c) = turn.sin_cos();
        let dir = Vec2::new(out.x * c - out.y * s, out.x * s + out.y * c);
        // Distance along `dir` from p to the circle's edge (plus margin).
        let b = rel.dot(dir);
        let disc = b * b - (rel.length_sq() - need * need);
        if disc < 0.0 {
            continue;
        }
        let step = -b + disc.sqrt();
        let to = p + dir * step;
        if !map.walkable(to, 30.0) {
            continue;
        }
        let score = step - 40.0 * dir.dot(prefer.normalize_or_zero());
        if best.is_none_or(|(_, s)| score < s) {
            best = Some((to, step));
        }
    }
    best.unwrap_or((p + out * (need - rel.length()).max(0.0), (need - rel.length()).max(0.0)))
}

/// Everything about to hit `me` that its team has seen, soonest first. `prefer`: the direction
/// it would rather step (its goal), for choosing sides.
pub fn threats(world: &World, me: &Unit, memory: &Memory, now: SimTime, prefer: Vec2) -> Vec<Threat> {
    let map = world.map();
    let p = me.state.pos;
    let r = me.gameplay_radius;
    let mut out = Vec::new();
    let mut push = |key: ThreatKey, hits_in: f32, (to, step): (Vec2, f32), hard: bool| {
        if let Some(seen_at) = memory.first_seen(key) {
            out.push(Threat { key, root: memory.root(key), seen_at, hits_in, to, step, hard });
        }
    };
    for a in world.areas().iter().filter(|a| a.team != me.team) {
        if p.distance(a.center) > a.radius + r {
            continue;
        }
        let hits_in = if a.lingering { 0.0 } else { a.detonate_at.secs_since(now).max(0.0) };
        let hard = a.cc.is_hard() || a.pulses.cc.is_hard();
        push(ThreatKey::Area(a.id), hits_in, out_of_circle(map, me, a.center, a.radius, prefer), hard);
    }
    for m in world.missiles().iter().filter(|m| m.team != me.team) {
        let at = m.position_at(now);
        let rel = p - at;
        let along = rel.dot(m.dir);
        let left = m.spec.range - m.origin.distance(at);
        if along < -r || along > left + r {
            continue;
        }
        let lateral = (rel - m.dir * along).length();
        if lateral > m.spec.radius + r {
            continue;
        }
        let hits_in = (along - r).max(0.0) / m.spec.speed.max(1.0);
        let hard = m.spec.cc.is_hard();
        push(ThreatKey::Missile(m.id), hits_in, off_line(map, me, at, m.dir, m.spec.radius, prefer), hard);
    }
    for c in world.channels().iter().filter(|c| c.team != me.team) {
        let rel = p - c.origin;
        let along = rel.dot(c.dir);
        if along < -r || along > c.spec.range + r || (rel - c.dir * along).length() > c.spec.radius + r {
            continue;
        }
        push(ThreatKey::Beam(c.id), 0.0, off_line(map, me, c.origin, c.dir, c.spec.radius, prefer), false);
    }
    // Wind-ups: a beam's aim is shown; a line's direction shows in the caster's stance.
    for u in world.units().iter().filter(|u| u.team != me.team && u.state.alive()) {
        let Some(cast) = u.state.cast else { continue };
        let Some(ability) = u.ability(cast.slot) else { continue };
        let (half, range, speed, hard) = match ability.effect {
            Effect::Beam(b) => (b.radius, b.range, f32::INFINITY, b.cc.is_hard()),
            Effect::Line(l) => (l.radius, l.range, l.speed, l.cc.is_hard()),
            Effect::Area(a) => {
                // An area being cast lands where it's aimed: a threat once it's telegraphed.
                if p.distance(cast.point) <= a.radius + r {
                    let hits_in = cast.fire_at.secs_since(now).max(0.0) + secs(a.delay);
                    let hard = !matches!(a.cc, Cc::None | Cc::Slow { .. });
                    push(
                        ThreatKey::Windup(u.id, cast.fire_at),
                        hits_in,
                        out_of_circle(map, me, cast.point, a.radius, prefer),
                        hard,
                    );
                }
                continue;
            }
            _ => continue,
        };
        let rel = p - u.state.pos;
        let along = rel.dot(cast.dir);
        if along < -r || along > range + r || (rel - cast.dir * along).length() > half + r {
            continue;
        }
        let hits_in = cast.fire_at.secs_since(now).max(0.0) + along.max(0.0) / speed;
        push(
            ThreatKey::Windup(u.id, cast.fire_at),
            hits_in,
            off_line(map, me, u.state.pos, cast.dir, half, prefer),
            hard,
        );
    }
    out.sort_by(|a, b| a.hits_in.total_cmp(&b.hits_in));
    out
}

/// Whether `me` can be clear of `t` before it hits, walking at its speed.
pub fn escapable(me: &Unit, t: &Threat, now: SimTime) -> bool {
    let speed = me.state.speed_at(now).max(1.0);
    let free = t.hits_in - kit::locked_for(me, now);
    // The step aims past the edge by MARGIN; only a sliver of it is needed to be missed.
    let need = (t.step - MARGIN + TIGHT).max(0.0);
    need / speed <= free + 0.05
}
