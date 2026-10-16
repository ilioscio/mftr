//! Combat micro (17 §7.2–§7.3): picking a target, choosing and aiming an ability (combos come
//! from the hit model), attacking and kiting, healing and shielding allies, escaping.

use crate::agent::{Agent, Ctx};
use crate::aim;
use crate::fight;
use crate::kit::{self, Archetype, use_of};
use mftr_sim::ability::Effect;
use mftr_sim::{CommandKind, QPoint, Unit, UnitId, UnitKind, Vec2};

pub fn cast(slot: u8, at: Vec2) -> CommandKind {
    CommandKind::Cast { slot, target: QPoint::from_vec2(at) }
}

/// The enemies a bot may fight: champions in sight, and in Horde the horde itself.
pub fn foes<'a, 'w>(c: &'a Ctx<'a, 'w>) -> impl Iterator<Item = &'w Unit> + 'a {
    let horde = c.b.world.rules().horde;
    c.b.enemies
        .iter()
        .copied()
        .chain(c.b.enemy_minions.iter().copied().filter(move |_| horde))
        .filter(|u| u.targetable())
}

/// Damage a second our champions near `at` deal (for valuing crowd control).
pub fn team_dps(c: &Ctx, at: Vec2) -> f32 {
    c.b.allies
        .iter()
        .filter(|a| a.state.alive() && a.state.pos.distance(at) < 1200.0)
        .map(|a| a.stats.attack_damage * a.attack.map_or(0.7, |s| s.attack_speed) + a.stats.ability_power * 0.4)
        .sum()
}

/// A turret of theirs covers `p` and standing there isn't worth its shots. It's fine while two
/// or more of our minions are in its range to take them (and it isn't already shooting us);
/// otherwise it's a dive, worth it only to kill a champion with the fight clearly ours (and
/// only for the tiers that dive).
pub fn turret_unsafe(c: &Ctx, p: Vec2) -> bool {
    let me = c.me;
    c.b.enemy_turrets_covering(p, 120.0).any(|t| {
        let range = t.attack.map_or(775.0, |a| a.range);
        if t.state.order == mftr_sim::Order::Attack(me.id) {
            return true;
        }
        let tanks = c.b.ally_minions.iter().filter(|m| m.state.pos.distance(t.state.pos) <= range).count();
        if tanks >= 2 {
            return false;
        }
        if c.strategy < 2 || !c.b.enemies.iter().any(|e| e.state.pos.distance(t.state.pos) < range + 200.0) {
            return true;
        }
        let (us, them) = c.b.engagement(t.state.pos, 1100.0, Some(c.memory));
        fight::outcome(&us, &them) < 0.45
    })
}

/// A structure we can take now without minions to tank it: no enemy champion there to defend
/// it (seen or likely), and our champions near it kill it before its shots would kill the
/// sturdiest of them (17 §6's numbers windows; Veteran and up).
pub fn siege_window(c: &Ctx, t: &Unit) -> bool {
    if c.strategy < 1 || !t.targetable() {
        return false;
    }
    let at = t.state.pos;
    if c.b.enemies.iter().any(|e| e.state.pos.distance(at) < 1400.0) || c.memory.danger(at, 1400.0, c.now) > 0.4 {
        return false;
    }
    let ours: Vec<&Unit> =
        c.b.allies.iter().copied().filter(|a| a.state.alive() && a.state.pos.distance(at) < 1300.0).collect();
    if ours.is_empty() {
        return false;
    }
    let dps: f32 = ours.iter().map(|a| a.stats.attack_damage * a.attack.map_or(0.7, |s| s.attack_speed)).sum::<f32>()
        * t.champion_damage_taken
        * mftr_sim::combat::resist_multiplier(t.stats.armor);
    let to_kill = t.state.health / dps.max(1.0);
    if t.kind != UnitKind::Turret {
        return true;
    }
    // Its shots ramp on one champion; the sturdiest takes them.
    let sturdiest = ours.iter().map(|a| a.state.health * (1.0 + a.stats.armor / 100.0)).fold(0.0, f32::max);
    let shot = t.stats.attack_damage * 2.0;
    let survives = sturdiest / (shot * t.attack.map_or(0.83, |s| s.attack_speed)).max(1.0);
    to_kill < survives * (0.7 + 0.25 * (ours.len() as f32 - 1.0)).min(1.5)
}

/// Our team's burst on `t` now: every ally within `reach` of it.
pub fn team_burst(c: &Ctx, t: &Unit, reach: f32) -> f32 {
    c.b.allies
        .iter()
        .filter(|a| a.state.alive() && a.state.pos.distance(t.state.pos) <= reach)
        .map(|a| burst_on(a, t, c))
        .sum()
}

/// `me`'s burst on `t` now: ready abilities (with their typical hit) and two attacks.
pub fn burst_on(me: &Unit, t: &Unit, c: &Ctx) -> f32 {
    let mut dmg = 2.0 * kit::attack_damage(me, t);
    for slot in 0..4u8 {
        if !me.state.can_cast(c.now, slot) {
            continue;
        }
        let Some(a) = me.ability(slot) else { continue };
        if !use_of(&a).damage {
            continue;
        }
        let (kind, raw) = kit::raw_damage(&a, me, kit::rank(me, slot, true));
        dmg += kit::mitigated(kind, raw, t) * 0.8;
    }
    dmg
}

/// Pick the target to fight: how quickly we kill it, how much it hurts us, who allies (a human
/// above all) already hit, and the cost of reaching it.
pub fn pick_target<'w>(a: &Agent, c: &Ctx<'_, 'w>, chase: bool) -> Option<&'w Unit> {
    let me = c.me;
    let horde = c.b.world.rules().horde;
    let my_range = me.attack.map_or(150.0, |s| s.range);
    let back = me.champion.is_some_and(|ch| Archetype::of(ch).back());
    let mut best: Option<(f32, &Unit)> = None;
    for t in foes(c) {
        let d = me.state.pos.distance(t.state.pos);
        let reach = my_range + t.gameplay_radius;
        let limit = if chase { 1300.0 } else { reach + 280.0 };
        if d > limit {
            continue;
        }
        if t.kind == UnitKind::Champion && turret_unsafe(c, t.state.pos) {
            continue;
        }
        let share = t.state.health / t.stats.max_health.max(1.0);
        let threat = c.b.enemy_power(t.id).map_or(60.0, |p| p.dps.total());
        let mut score = (1.0 + threat / 150.0) * (1.0 + 2.0 * (1.0 - share));
        if burst_on(me, t, c) >= t.state.health {
            score *= 2.5; // a kill now
        }
        let focus = c.memory.damage_on(t.id, 2.0, c.now);
        score *= 1.0 + (focus / t.stats.max_health.max(1.0)).min(1.0);
        // Follow a human's lead.
        let human_on_it = c.b.allies.iter().any(|ally| {
            ally.id != me.id && !c.is_bot(ally) && matches!(ally.state.order, mftr_sim::Order::Attack(id) if id == t.id)
        });
        if human_on_it {
            score *= 1.6;
        }
        score /= 1.0 + (d - reach).max(0.0) / 500.0;
        // Back-liners don't walk past the front line for a target behind it.
        if back && t.kind == UnitKind::Champion {
            let shielded = c.b.enemies.iter().any(|f| {
                f.id != t.id
                    && f.state.pos.distance(me.state.pos) + 150.0 < d
                    && f.state.pos.distance(t.state.pos) < 600.0
            });
            if shielded && d > reach {
                score *= 0.5;
            }
        }
        if horde && t.kind != UnitKind::Champion {
            // The horde: siege minions outrange turrets, casters next; bosses take a while.
            let r = t.attack.map_or(0.0, |s| s.range);
            score *= match t.kind {
                UnitKind::Monster => 0.9,
                _ if (200.0..500.0).contains(&r) => 1.5,
                _ if r >= 500.0 => 1.2,
                _ => 1.0,
            };
            score *= 0.6;
        }
        if a.focus().is_some_and(|f| f == t.id) {
            score *= 1.35;
        }
        if best.is_none_or(|(s, _)| score > s) {
            best = Some((score, t));
        }
    }
    best.map(|(_, t)| t)
}

/// The best ability to cast at `target` (or into a group), if any is worth it now.
pub fn cast_best(a: &mut Agent, c: &Ctx, target: &Unit, aggressive: bool) -> Option<CommandKind> {
    best_cast(a, c, target, aggressive).map(|(_, cmd)| cmd)
}

/// The best ability to cast at `target` (or into a group) and what landing it is worth (damage
/// after resistances, crowd control valued as the damage that lands meanwhile).
pub fn best_cast(a: &mut Agent, c: &Ctx, target: &Unit, aggressive: bool) -> Option<(f32, CommandKind)> {
    let me = c.me;
    let now = c.now;
    let horde = c.b.world.rules().horde;
    let dps = team_dps(c, me.state.pos);
    let mut best: Option<(f32, CommandKind)> = None;
    let blockers: Vec<(Vec2, f32)> =
        c.b.enemy_minions
            .iter()
            .chain(c.b.monsters.iter())
            .filter(|m| m.state.pos.distance(me.state.pos) < 1300.0)
            .map(|m| (m.state.pos, m.gameplay_radius))
            .collect();
    for slot in 0..6u8 {
        if !me.state.can_cast(now, slot) {
            continue;
        }
        let Some(ab) = me.ability(slot) else { continue };
        let u = use_of(&ab);
        if !u.damage || matches!(ab.effect, Effect::Claim(_)) {
            continue;
        }
        if a.rng_f32() > a.p.ability_use {
            continue;
        }
        let rank = kit::rank(me, slot, true);
        let ult = slot == 3;
        let d = me.state.pos.distance(target.state.pos);
        let reach = kit::reach(&ab);
        // Lunges: to close the gap (when committed) or to finish.
        if u.onto_enemy {
            if d > reach + target.gameplay_radius {
                continue;
            }
            let (kind, raw) = kit::raw_damage(&ab, me, rank);
            let dmg = kit::mitigated(kind, raw, target);
            let gap = d > me.attack.map_or(150.0, |s| s.range) + target.gameplay_radius + 40.0;
            let kills = dmg + kit::attack_damage(me, target) >= target.state.health;
            if !(kills || (aggressive && gap) || (!ult && aggressive && d < 400.0)) {
                continue;
            }
            if turret_unsafe(c, target.state.pos) && !kills {
                continue;
            }
            let v = dmg * if kills { 3.0 } else { 1.0 } + kit::cc_seconds(&ab) * dps;
            if ult && !kills && dmg < target.state.health * 0.35 {
                continue;
            }
            if best.is_none_or(|(b, _)| v > b) {
                best = Some((v, cast(slot, target.state.pos)));
            }
            continue;
        }
        // Areas: wherever they catch the most.
        let value_of = |t: &Unit| -> f32 {
            let (kind, raw) = kit::raw_damage(&ab, me, rank);
            let w = if t.kind == UnitKind::Champion {
                1.0
            } else if horde {
                0.45
            } else {
                0.08
            };
            (kit::mitigated(kind, raw, t) + kit::cc_seconds(&ab) * dps * (t.kind == UnitKind::Champion) as u8 as f32)
                * w
        };
        let (point, value, chance) = match ab.effect {
            Effect::Area(ar) if ar.range == 0.0 => {
                // A nova around us.
                let mut v = 0.0;
                let mut hits_champion = false;
                for t in foes(c) {
                    let p = kit::hit_chance(&ab, me, t, now, false);
                    if p > 0.3 {
                        v += value_of(t) * p;
                        hits_champion |= t.kind == UnitKind::Champion;
                    }
                }
                if !hits_champion && !horde {
                    continue;
                }
                (me.state.pos, v, 1.0)
            }
            Effect::Area(ar) => {
                let t_impact = kit::impact_time(&ab, d);
                let pts: Vec<(Vec2, f32, f32)> = foes(c)
                    .filter(|t| t.state.pos.distance(me.state.pos) < ar.range + ar.radius + 300.0)
                    .map(|t| {
                        let p = aim::predict(t, now, t_impact, a.p.lead.max(0.5));
                        let chance = kit::hit_chance(&ab, me, t, now, false);
                        (p, t.gameplay_radius, value_of(t) * chance)
                    })
                    .collect();
                let Some((center, v)) = aim::best_area(me.state.pos, ar.range, ar.radius, &pts) else { continue };
                let hits_target = center.distance(aim::predict(target, now, t_impact, 1.0)) <= ar.radius + 20.0;
                let center = if hits_target {
                    // Aim error applies to the chosen center too.
                    let e = aim::aim(&ab, me.state.pos, target, now, a.p.lead, a.p.aim_error, a.rng());
                    center.lerp(e, 0.3)
                } else {
                    center
                };
                (center, v, 1.0)
            }
            _ => {
                if d > reach + target.gameplay_radius * 0.5 {
                    continue;
                }
                let aim_at = aim::aim(&ab, me.state.pos, target, now, a.p.lead, a.p.aim_error, a.rng());
                let blocked = u.blocked_by_units
                    && kit::line_blocked(me.state.pos, target.state.pos, kit::width(&ab), blockers.iter().copied());
                let chance = kit::hit_chance(&ab, me, target, now, blocked);
                // A channel locks us: only from safety, or for a kill.
                if matches!(ab.effect, Effect::Beam(_))
                    && c.b.enemies.iter().any(|e| e.state.pos.distance(me.state.pos) < 550.0)
                {
                    continue;
                }
                (aim_at, value_of(target) * chance, chance)
            }
        };
        // No mana: a basic ability is worth casting at any fair chance (it comes back); hard
        // crowd control and long cooldowns wait for a likelier hit; ultimates for a good one.
        let cd = ab.cooldown_at(rank).0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32;
        let min_chance = if ult {
            0.5
        } else if kit::cc_seconds(&ab) > 0.0 {
            if aggressive { 0.3 } else { 0.45 }
        } else if cd <= 9.0 {
            0.12
        } else {
            0.28
        };
        if chance < min_chance || value <= 0.0 {
            continue;
        }
        // Ultimates: for a kill, a big chunk, or more than one champion.
        let (kind, raw) = kit::raw_damage(&ab, me, rank);
        let on_target = kit::mitigated(kind, raw, target);
        let kills = on_target * chance.max(0.6) + kit::attack_damage(me, target) >= target.state.health;
        if ult && !horde && !kills && value < target.stats.max_health * 0.3 && value < on_target * 1.8 {
            continue;
        }
        if !aggressive && kit::cc_seconds(&ab) > 0.0 && !kills && value < on_target * 1.5 {
            // Hold crowd control for a fight, not a poke.
            continue;
        }
        let v = value * if kills { 2.5 } else { 1.0 } * if ult { 0.8 } else { 1.0 };
        if best.is_none_or(|(b, _)| v > b) {
            best = Some((v, cast(slot, point)));
        }
    }
    best
}

/// Attack `t`, kiting when the tier does and it pays (a ranged champion against a shorter
/// reach); `chase`: walk after it when out of reach.
pub fn attack(a: &mut Agent, c: &Ctx, t: &Unit, chase: bool) -> Option<CommandKind> {
    let me = c.me;
    let st = &me.state;
    let d = st.pos.distance(t.state.pos);
    let my_range = me.attack.map_or(150.0, |s| s.range);
    let reach = my_range + t.gameplay_radius;
    if d > reach + 30.0 && !chase {
        return None;
    }
    let their_range = t.attack.map_or(150.0, |s| s.range) + me.gameplay_radius;
    let ready_in = if st.attack_ready_at > c.now { st.attack_ready_at.secs_since(c.now) } else { 0.0 };
    let back = me.champion.is_some_and(|ch| Archetype::of(ch).back());
    if back && d > reach + 30.0 {
        // A back-liner doesn't walk past the enemy's front line for a target: where it would
        // have to stand must be out of every other enemy champion's reach.
        let stand = t.state.pos + (st.pos - t.state.pos).normalize_or_zero() * (reach - 30.0);
        let exposed = c.b.enemies.iter().any(|e| {
            e.id != t.id
                && e.state.pos.distance(stand) < e.attack.map_or(150.0, |s| s.range) + me.gameplay_radius + 120.0
        });
        if exposed {
            return None;
        }
    }
    // Spacing: a ranged champion steps back from a shorter-reach enemy closing in while its own
    // attack recharges (every tier but the lowest; the top tiers kite every attack).
    if a.p.strategy >= 1
        && my_range > 300.0
        && st.attack.is_none()
        && ready_in > 0.2
        && let Some(threat) =
            c.b.enemies.iter().filter(|e| e.attack.map_or(150.0, |s| s.range) < my_range - 100.0).find(|e| {
                e.state.pos.distance(st.pos) < e.attack.map_or(150.0, |s| s.range) + me.gameplay_radius + 90.0
            })
    {
        let away = (st.pos - threat.state.pos).normalize_or_zero();
        let to = st.pos + away * 220.0;
        if c.b.world.map().walkable(to, 40.0) && !turret_unsafe(c, to) {
            a.wake_at(c.now.plus(mftr_sim::SimDuration::from_millis((ready_in * 1000.0) as u64)));
            return Some(CommandKind::MoveTo(QPoint::from_vec2(to)));
        }
    }
    if a.p.kites
        && my_range > 300.0
        && their_range < reach - 100.0
        && st.attack.is_none()
        && ready_in > 0.15
        && d < reach
    {
        // Step back between attacks, keeping it in range.
        let away = (st.pos - t.state.pos).normalize_or_zero();
        let step = (reach - d - 20.0).clamp(80.0, 220.0);
        let to = st.pos + away * step;
        if c.b.world.map().walkable(to, 40.0) && !turret_unsafe(c, to) {
            a.wake_at(c.now.plus(mftr_sim::SimDuration::from_millis((ready_in * 1000.0) as u64)));
            return Some(CommandKind::MoveTo(QPoint::from_vec2(to)));
        }
    }
    Some(CommandKind::Attack(t.id))
}

/// Fight: pick a target, cast what's worth it, else attack. `chase`: follow targets out of
/// reach. `prefer`: a target to favor (a gank's, a ping's).
pub fn fight(a: &mut Agent, c: &Ctx, chase: bool, prefer: Option<UnitId>) -> Option<CommandKind> {
    let target = prefer
        .and_then(|id| c.b.unit(id))
        .filter(|t| t.targetable() && foes(c).any(|f| f.id == t.id))
        .filter(|t| t.kind != UnitKind::Champion || !turret_unsafe(c, t.state.pos))
        .or_else(|| pick_target(a, c, chase))?;
    a.set_focus(target.id, c.now);
    if let Some(cmd) = cast_best(a, c, target, true) {
        return Some(cmd);
    }
    attack(a, c, target, chase)
}

/// Heal or shield whoever needs it most in reach (17 §7.2, peel), ourselves included.
pub fn support(a: &mut Agent, c: &Ctx) -> Option<CommandKind> {
    let me = c.me;
    let now = c.now;
    for slot in 0..6u8 {
        if !me.state.can_cast(now, slot) {
            continue;
        }
        let Some(ab) = me.ability(slot) else { continue };
        let (range, heals, shields) = match ab.effect {
            Effect::Support(s) => (s.range, s.heal > 0.0 || s.heal_missing > 0.0, s.shield > 0.0),
            Effect::Shield(_) => (0.0, false, true),
            _ => continue,
        };
        let mut best: Option<(f32, Vec2)> = None;
        for ally in c.b.allies.iter().filter(|x| x.state.alive()) {
            let d = ally.state.pos.distance(me.state.pos);
            if ally.id != me.id && (range <= 0.0 || d > range) {
                continue;
            }
            let share = ally.state.health / ally.stats.max_health.max(1.0);
            let hit = c.memory.damage_on(ally.id, 2.0, now) / ally.stats.max_health.max(1.0);
            let mut urgency = 0.0;
            if heals {
                urgency += (1.0 - share) * 1.2;
            }
            if shields {
                urgency += hit * 3.0 + if share < 0.5 && hit > 0.0 { 0.4 } else { 0.0 };
            }
            // Barrier-like self shields: only to live through a burst.
            if matches!(ab.effect, Effect::Shield(_)) && !(share < 0.35 && hit > 0.12) {
                continue;
            }
            if ally.id != me.id && !c.is_bot(ally) {
                urgency *= 1.2; // humans first
            }
            if urgency > 0.45 && best.is_none_or(|(u, _)| urgency > u) {
                best = Some((urgency, ally.state.pos));
            }
        }
        if let Some((_, at)) = best {
            let _ = a;
            return Some(cast(slot, at));
        }
    }
    None
}

/// Get away toward `safe`: crowd-control the nearest chaser, else dash or blink that way.
pub fn escape(a: &mut Agent, c: &Ctx, safe: Vec2) -> Option<CommandKind> {
    let me = c.me;
    let now = c.now;
    let chaser =
        c.b.enemies
            .iter()
            .filter(|e| e.state.pos.distance(me.state.pos) < 650.0)
            .min_by(|x, y| x.state.pos.distance(me.state.pos).total_cmp(&y.state.pos.distance(me.state.pos)))
            .copied()?;
    // Slow or stop it.
    for slot in 0..6u8 {
        if !me.state.can_cast(now, slot) {
            continue;
        }
        let Some(ab) = me.ability(slot) else { continue };
        let u = use_of(&ab);
        if !(u.hard_cc || u.slow) || u.onto_enemy {
            continue;
        }
        if kit::hit_chance(&ab, me, chaser, now, false) < 0.45 {
            continue;
        }
        let at = if u.nova {
            me.state.pos
        } else {
            aim::aim(&ab, me.state.pos, chaser, now, a.p.lead, a.p.aim_error, a.rng())
        };
        return Some(cast(slot, at));
    }
    // Dash or blink away (the utility Blink only when it's this or dying).
    let dir = (safe - me.state.pos).normalize_or_zero();
    let desperate = me.state.health < me.stats.max_health * 0.25;
    for slot in [2u8, 0, 1, 3, 5, 4] {
        if !me.state.can_cast(now, slot) {
            continue;
        }
        let Some(ab) = me.ability(slot) else { continue };
        let range = match ab.effect {
            Effect::Dash(d) => d.range,
            Effect::Blink(b) if slot != 4 || desperate => b.range,
            _ => continue,
        };
        let to = me.state.pos + dir * range;
        if c.b.world.map().walkable(to, 40.0) {
            return Some(cast(slot, to));
        }
    }
    None
}
