//! Laning (17 §7.1): last hits timed against every attack already coming at a minion, where to
//! stand by the wave, trades when they pay, pushing when the coach or the bot wants the wave
//! gone, and respect for the enemy turret.

use crate::agent::{Agent, Ctx};
use crate::fight;
use crate::kit::{self, use_of};
use crate::micro;
use mftr_sim::ability::Effect;
use mftr_sim::{CommandKind, SimTime, Unit, UnitKind, Vec2};

/// The hits coming at `m` within `horizon` seconds, as (seconds from now, damage), soonest
/// first: bolts in flight, wind-ups, and the next attacks of everyone of ours attacking it.
pub fn incoming(c: &Ctx, m: &Unit, horizon: f32, skip: Option<mftr_sim::UnitId>) -> Vec<(f32, f32)> {
    let now = c.now;
    let world = c.b.world;
    let mut hits = Vec::new();
    for b in world.bolts().iter().filter(|b| b.target == m.id) {
        if Some(b.owner) == skip {
            continue;
        }
        let t = b.pos.distance(m.state.pos) / b.speed.max(1.0);
        let dmg = match b.kind {
            mftr_sim::ability::DamageKind::True => b.power,
            _ => b.power * mftr_sim::combat::resist_multiplier(m.stats.armor),
        };
        hits.push((t, dmg));
    }
    let attackers = c.b.ally_minions.iter().chain(c.b.ally_structures.iter()).chain(c.b.allies.iter());
    for u in attackers {
        if Some(u.id) == skip || !u.state.alive() {
            continue;
        }
        let Some(spec) = u.attack else { continue };
        let on_it = matches!(u.state.order, mftr_sim::Order::Attack(id) if id == m.id)
            || u.state.attack.is_some_and(|w| w.target == m.id);
        if !on_it {
            continue;
        }
        let dist = u.state.pos.distance(m.state.pos);
        if dist > spec.range + m.gameplay_radius + 60.0 {
            continue;
        }
        let travel = if spec.bolt_speed > 0.0 { dist / spec.bolt_speed } else { 0.0 };
        let dmg = if u.kind == UnitKind::Turret {
            mftr_sim::lane::turret_minion_share(m.attack.map_or(0.0, |a| a.range)) * m.stats.max_health
        } else {
            kit::attack_damage(u, m)
        };
        let secs = |t: SimTime| if t > now { t.secs_since(now) } else { 0.0 };
        let windup = spec.windup().0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32;
        let period = spec.period().0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32;
        // The swing under way, then the next ones on its timer.
        let mut next_start = secs(u.state.attack_ready_at);
        if let Some(w) = u.state.attack.filter(|w| w.target == m.id) {
            hits.push((secs(w.fire_at) + travel, dmg));
            next_start = next_start.max(secs(w.fire_at) + period - windup);
        }
        while next_start + windup + travel < horizon {
            hits.push((next_start + windup + travel, dmg));
            next_start += period;
        }
    }
    hits.sort_by(|a, b| a.0.total_cmp(&b.0));
    hits
}

/// `m`'s health `t` seconds from now (before our own hit), and when `hits` kill it, if they do.
pub fn health_at(m: &Unit, hits: &[(f32, f32)], t: f32) -> (f32, Option<f32>) {
    let (mut at_t, mut left, mut death) = (m.state.health, m.state.health, None);
    for &(at, dmg) in hits {
        if at < t {
            at_t -= dmg;
        }
        if death.is_none() {
            left -= dmg;
            if left <= 0.0 {
                death = Some(at);
            }
        }
    }
    (at_t, death)
}

/// Seconds until our attack would land on `m` from here: walking into range, the attack timer,
/// the wind-up and the bolt's flight.
pub fn our_hit_in(me: &Unit, m: &Unit, now: SimTime) -> f32 {
    let Some(spec) = me.attack else { return f32::INFINITY };
    let d = me.state.pos.distance(m.state.pos);
    let reach = spec.range + m.gameplay_radius;
    let walk = (d - reach).max(0.0) / me.state.speed_at(now).max(1.0);
    let ready = if me.state.attack_ready_at > now { me.state.attack_ready_at.secs_since(now) } else { 0.0 };
    let windup = spec.windup().0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32;
    let travel = if spec.bolt_speed > 0.0 { d.min(reach) / spec.bolt_speed } else { 0.0 };
    walk.max(ready) + windup + travel
}

/// The last hit to take now, or a minion to wait by.
pub enum LastHit<'w> {
    /// Attack it now: our hit lands just as it can die to it.
    Take(&'w Unit),
    /// It'll be ours soon (swing in this many seconds): stand near it, don't touch it yet.
    Wait(&'w Unit, f32),
    None,
}

/// When `m` can be last-hit with `dmg`: the window from the hit that brings it within `dmg`
/// (now, if it's already there) to the hit that kills it (`None`: none in the horizon).
pub fn window(m: &Unit, hits: &[(f32, f32)], dmg: f32) -> Option<(f32, Option<f32>)> {
    let mut hp = m.state.health;
    let mut open = if hp <= dmg { Some(0.0) } else { None };
    for &(at, d) in hits {
        hp -= d;
        if hp <= 0.0 {
            return open.map(|o| (o, Some(at)));
        }
        if open.is_none() && hp <= dmg {
            open = Some(at);
        }
    }
    open.map(|o| (o, None))
}

/// The enemy champions to keep away from while farming (when we'd lose a fight to them), with
/// how far: their attack's reach, or a lunge's when it's up (what can't be dodged; skillshots
/// are dodged instead), and a step more.
pub fn keep_away<'w>(a: &Agent, c: &Ctx<'_, 'w>) -> Vec<(&'w Unit, f32)> {
    if a.local >= -0.1 {
        return Vec::new();
    }
    let tracking = a.p.tracking;
    c.b.enemies
        .iter()
        .copied()
        .filter(|e| e.state.pos.distance(c.me.state.pos) < 1500.0)
        .map(|e| {
            let mut reach = e.attack.map_or(150.0, |s| s.range) + c.me.gameplay_radius;
            for slot in 0..4u8 {
                if let Some(ab) = e.ability(slot)
                    && kit::use_of(&ab).onto_enemy
                    && c.memory.ready(e, slot, c.now, tracking)
                {
                    reach = reach.max(kit::reach(&ab) + c.me.gameplay_radius);
                }
            }
            (e, reach + 100.0)
        })
        .collect()
}

/// Whether we can stand within reach of `m` (to hit it) and still be outside every `away`.
fn reachable_safely(me: &Unit, m: &Unit, away: &[(&Unit, f32)]) -> bool {
    let range = me.attack.map_or(150.0, |s| s.range) + m.gameplay_radius;
    away.iter().all(|(e, r)| e.state.pos.distance(m.state.pos) + range > *r + 40.0)
}

pub fn last_hit<'w>(a: &mut Agent, c: &Ctx<'_, 'w>, within: f32) -> LastHit<'w> {
    let me = c.me;
    let now = c.now;
    let mut take: Option<(f32, &Unit)> = None;
    let mut wait: Option<(f32, f32, &Unit)> = None;
    let range = me.attack.map_or(0.0, |s| s.range);
    let away = keep_away(a, c);
    // Our own bolt already flying at a minion: that one's handled.
    let shot: Vec<mftr_sim::UnitId> = c.b.world.bolts().iter().filter(|b| b.owner == me.id).map(|b| b.target).collect();
    let winding = me.state.attack.map(|w| w.target);
    for m in c.b.enemy_minions.iter().copied() {
        if shot.contains(&m.id) {
            continue;
        }
        let d = me.state.pos.distance(m.state.pos);
        if d > within || !m.targetable() || micro::turret_unsafe(c, m.state.pos) && d > range {
            continue;
        }
        if !reachable_safely(me, m, &away) {
            continue;
        }
        let hits = incoming(c, m, 4.0, Some(me.id));
        let ours = our_hit_in(me, m, now) + a.last_hit_skew();
        let dmg = kit::attack_damage(me, m) * 0.98;
        let Some((open, close)) = window(m, &hits, dmg) else { continue };
        // The swing under way stays on its minion while that's still a last hit.
        let gold = mftr_sim::lane::minion_reward(m.attack.map_or(0.0, |s| s.range)).0
            + if winding == Some(m.id) { 1000.0 } else { 0.0 };
        if ours >= open && close.is_none_or(|c| ours < c - 0.02) {
            if take.is_none_or(|(g, _)| gold > g) {
                take = Some((gold, m));
            }
        } else if ours < open && open - ours < 2.5 {
            // Too early: swing when our hit would land just after the window opens.
            let fire_in = open - ours + 0.04;
            if wait.is_none_or(|(f, g, _)| fire_in < f - 0.3 || (fire_in < f + 0.3 && gold > g)) {
                wait = Some((fire_in, gold, m));
            }
        }
    }
    match (take, wait) {
        (Some((_, m)), _) => LastHit::Take(m),
        (None, Some((fire_in, _, m))) => {
            a.wake_at(now.plus(mftr_sim::SimDuration::from_millis((fire_in * 1000.0) as u64)));
            LastHit::Wait(m, fire_in)
        }
        _ => LastHit::None,
    }
}

/// A last hit our attack can't make in time (the window closes before it would land), taken
/// with a basic ability that kills in the window: a line with the minion first in its path, an
/// area or a nova. Veteran and up; abilities are kept for trades while an enemy champion is
/// close, unless they come back quickly.
pub fn ability_last_hit(a: &mut Agent, c: &Ctx) -> Option<CommandKind> {
    let me = c.me;
    let now = c.now;
    if a.p.strategy < 1 || me.champion.is_none() {
        return None;
    }
    let enemy_close = c.b.enemies.iter().any(|e| e.state.pos.distance(me.state.pos) < 1000.0);
    for slot in 0..3u8 {
        if !me.state.can_cast(now, slot) {
            continue;
        }
        let Some(ab) = me.ability(slot) else { continue };
        let u = kit::use_of(&ab);
        if !u.damage || u.onto_enemy || u.moves {
            continue;
        }
        let rank = kit::rank(me, slot, true);
        let cd = ab.cooldown_at(rank).0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32;
        if cd > 10.0 || (enemy_close && cd > 5.0) {
            continue;
        }
        let (kind, raw) = kit::raw_damage(&ab, me, rank);
        for m in c.b.enemy_minions.iter().copied() {
            let d = me.state.pos.distance(m.state.pos);
            if d > kit::reach(&ab) + m.gameplay_radius * 0.5 || micro::turret_unsafe(c, m.state.pos) {
                continue;
            }
            let hits = incoming(c, m, 3.0, Some(me.id));
            let auto = our_hit_in(me, m, now);
            let dmg = kit::mitigated(kind, raw, m) * 0.97;
            let Some((open, close)) = window(m, &hits, kit::attack_damage(me, m)) else { continue };
            // Only what our attack would miss.
            if close.is_none_or(|t| auto < t) && auto >= open {
                continue;
            }
            let t = kit::impact_time(&ab, d);
            let Some((open_ab, close_ab)) = window(m, &hits, dmg) else { continue };
            if t < open_ab || close_ab.is_some_and(|x| t >= x - 0.03) || close.is_none() {
                continue;
            }
            let at = match ab.effect {
                mftr_sim::ability::Effect::Area(ar) if ar.range == 0.0 => me.state.pos,
                _ => crate::aim::predict(m, now, t, 1.0),
            };
            if u.blocked_by_units {
                let others =
                    c.b.enemy_minions
                        .iter()
                        .chain(c.b.enemies.iter())
                        .filter(|o| o.id != m.id)
                        .map(|o| (o.state.pos, o.gameplay_radius));
                if kit::line_blocked(me.state.pos, at, kit::width(&ab), others) {
                    continue;
                }
            }
            return Some(micro::cast(slot, at));
        }
    }
    None
}

/// Where to stand in `lane`: just behind our wave's front, within reach of theirs (ranged a
/// little farther back), out of their turret's range unless our minions take its shots, and
/// out of the enemy laner's reach when we're not trading.
fn spot(c: &Ctx, lane: usize, cautious: bool) -> Vec2 {
    let me = c.me;
    let team = c.b.team;
    let v = &c.b.lanes[lane];
    let range = me.attack.map_or(150.0, |s| s.range);
    let ranged = range > 300.0;
    let back = if cautious { 450.0 } else { 0.0 };
    let d = match (v.ours, v.theirs) {
        (Some((o, _)), Some((t, _))) if t - o < 700.0 => {
            // Waves meeting: behind the clash.
            let clash = (o + t) * 0.5;
            // Melee waits a step back and walks in for its last hits.
            if ranged { clash - range * 0.75 } else { clash - 300.0 }
        }
        (Some((o, _)), _) => o - if ranged { 250.0 } else { 120.0 },
        (None, Some((t, _))) => t - range - 250.0,
        (None, None) => v.our_turret.map_or(c.k.lane_length(lane, team) * 0.3, |(d, _)| d + 250.0),
    } - back;
    let mut d = d.max(0.0);
    // Back off along the lane while their turret would shoot us there.
    for _ in 0..25 {
        let p = c.k.lane_point(lane, team, d);
        if !micro::turret_unsafe(c, p) || d <= 0.0 {
            break;
        }
        d -= 120.0;
    }
    let p = c.k.lane_point(lane, team, d.max(0.0));
    let (_, dir, _) = c.k.along(lane, team, p);
    // A little to the side, so laners don't stack.
    p + Vec2::new(-dir.y, dir.x) * (((me.id.0 % 5) as f32 - 2.0) * 50.0)
}

/// Farm and hold `lane`: take last hits, trade when it pays, push when asked, keep a good spot.
pub fn farm(a: &mut Agent, c: &Ctx, lane: usize, push: bool) -> Option<CommandKind> {
    let me = c.me;
    let now = c.now;
    if c.b.lanes.get(lane).is_none() {
        // No lanes (a sandbox): fight whatever's there.
        return micro::fight(a, c, true, None);
    }
    let cautious = c.coach.cautions.iter().any(|(p, _)| p.distance(me.state.pos) < 3000.0)
        || c.memory.danger(me.state.pos, 1500.0, now) > 1.3;
    // A support with its carry nearby (a bot or a human) leaves the last hits to it.
    let leave_cs = c.role_support()
        && c.b.allies.iter().any(|x| x.id != me.id && x.state.alive() && x.state.pos.distance(me.state.pos) < 1000.0);
    let lh = if leave_cs { LastHit::None } else { last_hit(a, c, 1100.0) };
    // A kill in reach comes first; then the last hit due now; trades only when no last hit is
    // about to come up.
    if let Some(cmd) = kill(a, c) {
        return Some(cmd);
    }
    if let LastHit::Take(m) = lh {
        a.count_last_hit_try();
        return Some(CommandKind::Attack(m.id));
    }
    if !leave_cs && let Some(cmd) = ability_last_hit(a, c) {
        return Some(cmd);
    }
    let soon = matches!(lh, LastHit::Wait(_, t) if t < 0.8);
    if !soon && let Some(cmd) = trade(a, c) {
        return Some(cmd);
    }
    {
        match lh {
            LastHit::Wait(m, _) if !push => {
                // Stand within reach of it, without attacking it.
                let range = me.attack.map_or(150.0, |s| s.range);
                let d = me.state.pos.distance(m.state.pos);
                if d > range + 50.0 && !micro::turret_unsafe(c, m.state.pos) {
                    let to = m.state.pos + (me.state.pos - m.state.pos).normalize_or_zero() * (range - 30.0);
                    return a.go(c, to);
                }
                // Already in reach: hold still (Stop only if we're attacking something else).
                if matches!(me.state.order, mftr_sim::Order::Attack(_)) {
                    return Some(CommandKind::Stop);
                }
                return None;
            }
            _ => {}
        }
    }
    // Pushing: hit the wave (lowest health first), and use spare area abilities on it.
    // Push only on purpose (a recall, a siege, a roam), or with no one there to contest it: a
    // wave shoved into their turret is gold the turret takes.
    let alone = !c.b.enemies.iter().any(|e| e.state.pos.distance(me.state.pos) < 2500.0)
        && c.memory.danger(me.state.pos, 2500.0, now) < 0.5;
    let pushing = push || (alone && a.p.strategy >= 1);
    if pushing && !leave_cs {
        if let Some(cmd) = wave_spell(a, c) {
            return Some(cmd);
        }
        let range = me.attack.map_or(150.0, |s| s.range);
        let target =
            c.b.enemy_minions
                .iter()
                .filter(|m| m.targetable() && m.state.pos.distance(me.state.pos) < range + 300.0)
                .filter(|m| !micro::turret_unsafe(c, m.state.pos))
                .min_by(|x, y| x.state.health.total_cmp(&y.state.health));
        if let Some(m) = target {
            return Some(CommandKind::Attack(m.id));
        }
        // Their wave is gone: hit the structure if it's safe.
        if let Some(cmd) = siege_structure(c, lane) {
            return Some(cmd);
        }
    }
    let mut to = spot(c, lane, cautious);
    // Outmatched: hold back out of their reach.
    for (e, r) in keep_away(a, c) {
        let d = to.distance(e.state.pos);
        if d < r {
            let home = c.k.homes[c.b.team as usize];
            let back = (home - to).normalize_or_zero();
            to += back * (r - d + 60.0);
        }
    }
    match a.go(c, to) {
        Some(cmd) => Some(cmd),
        // Settled: drop an attack that's no longer wanted (a harass, a minion we'd push).
        None if matches!(me.state.order, mftr_sim::Order::Attack(_)) && me.state.attack.is_none() => {
            Some(CommandKind::Stop)
        }
        None => None,
    }
}

/// An area ability on a wave worth clearing (three or more minions caught), cooldowns allowing.
pub fn wave_spell(a: &mut Agent, c: &Ctx) -> Option<CommandKind> {
    let me = c.me;
    if me.champion.is_none() || c.b.enemies.iter().any(|e| e.state.pos.distance(me.state.pos) < 900.0) {
        return None;
    }
    for slot in 0..3u8 {
        if !me.state.can_cast(c.now, slot) {
            continue;
        }
        let Some(ab) = me.ability(slot) else { continue };
        let Effect::Area(ar) = ab.effect else { continue };
        if !use_of(&ab).damage || a.rng_f32() > 0.5 {
            continue;
        }
        let pts: Vec<(Vec2, f32, f32)> =
            c.b.enemy_minions
                .iter()
                .filter(|m| m.state.pos.distance(me.state.pos) < ar.range + ar.radius + 100.0)
                .map(|m| (m.state.pos, m.gameplay_radius, 1.0))
                .collect();
        if let Some((at, n)) = crate::aim::best_area(me.state.pos, ar.range, ar.radius, &pts)
            && n >= 3.0
        {
            return Some(micro::cast(slot, if ar.range == 0.0 { me.state.pos } else { at }));
        }
    }
    None
}

/// Hit the lane's next enemy structure if it's safe to (our minions take its shots, or it can't
/// shoot).
pub fn siege_structure(c: &Ctx, lane: usize) -> Option<CommandKind> {
    let me = c.me;
    let t = c.b.lanes.get(lane)?.their_target.and_then(|id| c.b.unit(id))?;
    if me.state.pos.distance(t.state.pos) > 1300.0 {
        return None;
    }
    if t.kind == UnitKind::Turret && micro::turret_unsafe(c, t.state.pos) && !micro::siege_window(c, t) {
        return None;
    }
    Some(CommandKind::Attack(t.id))
}

/// An enemy champion we can kill now: our team's burst on it beats its health, it's in our
/// reach, and not under its turret (unless a dive pays, `micro::turret_unsafe`).
pub fn kill(a: &mut Agent, c: &Ctx) -> Option<CommandKind> {
    let me = c.me;
    let reach = me.attack.map_or(150.0, |s| s.range) + 250.0;
    let e =
        c.b.enemies
            .iter()
            .copied()
            .filter(|e| e.state.pos.distance(me.state.pos) <= reach)
            .filter(|e| !micro::turret_unsafe(c, e.state.pos))
            .find(|e| micro::team_burst(c, e, 900.0) >= e.state.health * 1.1)?;
    micro::fight(a, c, true, Some(e.id))
}

/// A trade with the enemy laner (17 §7.1): all in when the fight is clearly ours; else a poke
/// when what it lands is worth more than what comes back (their burst if we stand in their
/// reach, and their minions turning on us for hitting their champion), and we're not the more
/// hurt one.
fn trade(a: &mut Agent, c: &Ctx) -> Option<CommandKind> {
    let me = c.me;
    let now = c.now;
    let e =
        c.b.enemies
            .iter()
            .filter(|e| e.state.pos.distance(me.state.pos) < 1100.0)
            .min_by(|x, y| x.state.pos.distance(me.state.pos).total_cmp(&y.state.pos.distance(me.state.pos)))
            .copied()?;
    if micro::turret_unsafe(c, e.state.pos) {
        return None;
    }
    let share = |u: &Unit| u.state.health / u.stats.max_health.max(1.0);
    let (us, them) = c.b.engagement(me.state.pos.lerp(e.state.pos, 0.5), 1100.0, Some(c.memory));
    let o = fight::outcome(&us, &them) + a.read_noise();
    if o > 0.35 && share(me) > 0.5 && a.p.strategy >= 1 {
        return micro::fight(a, c, true, Some(e.id));
    }
    if share(me) + 0.12 < share(e) {
        return None;
    }
    // What hitting it costs: their minions near us turn on us for a couple of seconds, and
    // their burst lands if we're in their reach.
    let minion_dps: f32 =
        c.b.enemy_minions
            .iter()
            .filter(|m| m.state.pos.distance(me.state.pos) < 750.0)
            .map(|m| kit::attack_damage(m, me) * m.attack.map_or(0.0, |s| s.attack_speed))
            .sum();
    let their = c.b.enemy_power(e.id).copied().unwrap_or_default();
    let in_their_reach = me.state.pos.distance(e.state.pos) <= their.range + 100.0;
    let retaliation = minion_dps * 1.8
        + if in_their_reach { their.burst.against(me.stats.armor, me.stats.magic_resist) * 0.5 } else { 0.0 };
    if let Some((value, cmd)) = micro::best_cast(a, c, e, false)
        && value > retaliation * 0.8
    {
        return Some(cmd);
    }
    // A single ranged attack when it's free: no minions of theirs near us, or it's busy
    // last-hitting, and the attack is ready (the next think goes back to farming).
    let range = me.attack.map_or(150.0, |s| s.range);
    let d = me.state.pos.distance(e.state.pos);
    let busy = matches!(e.state.order, mftr_sim::Order::Attack(id) if c.b.unit(id).is_some_and(|u| u.kind == UnitKind::Minion));
    let ready = me.state.attack_ready_at <= now && me.state.attack.is_none();
    if range > 300.0 && ready && d <= range + e.gameplay_radius && (minion_dps == 0.0 || (busy && minion_dps < 40.0)) {
        return Some(CommandKind::Attack(e.id));
    }
    None
}
