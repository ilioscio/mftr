//! Jungling (17 §6–§7): which camp next (standing or due by the time it gets there, by the
//! timers the team saw), clearing it with area abilities and Claim, securing big monsters, and
//! taking the enemy's camps when its jungler is seen far away (the top tier).

use crate::agent::{Agent, Ctx};
use crate::kit;
use crate::memory::CampState;
use crate::micro;
use mftr_sim::ability::Effect;
use mftr_sim::jungle::CampKind;
use mftr_sim::world::Brain;
use mftr_sim::{CommandKind, SimTime, Unit};

/// When camp `i` is (or will be) up, as far as the team knows; `None`: unknown.
fn up_at(c: &Ctx, i: usize) -> Option<SimTime> {
    let first = c.b.world.game().started_at.plus(mftr_sim::jungle::FIRST_SPAWN);
    if c.now < first {
        return Some(first);
    }
    match c.memory.camps.get(i).map(|x| x.0) {
        Some(CampState::Up) => Some(c.now),
        Some(CampState::DownUntil(t)) => Some(t.max(c.now)),
        Some(CampState::DownBy(t)) => Some(t.max(c.now)),
        // Never seen since the first spawn: probably up.
        Some(CampState::Unknown) | None => Some(c.now),
    }
}

/// The next camp to take: ours (or theirs, for the top tier when it's safe) that will be up by
/// the time we get there, nearest first, buffs preferred.
pub fn next_camp(a: &Agent, c: &Ctx) -> Option<usize> {
    let me = c.me;
    let team = c.b.team;
    let speed = me.state.speed_at(c.now).max(300.0);
    let mut best: Option<(f32, usize)> = None;
    for (i, camp) in c.k.camps.iter().enumerate() {
        let ours = c.k.side_of(camp.pos) == Some(team);
        if !ours {
            // Counter-jungling: their jungler seen far from here lately, and nobody of theirs near.
            if a.p.strategy < 3 {
                continue;
            }
            let safe = c.memory.danger(camp.pos, 2200.0, c.now) < 0.35;
            if !safe {
                continue;
            }
        }
        let Some(at) = up_at(c, i) else { continue };
        let travel = c.k.travel(me.state.pos, camp.pos) / speed;
        let wait = (at.secs_since(c.now) - travel).max(0.0);
        if wait > 25.0 {
            continue;
        }
        let buff = matches!(camp.kind, CampKind::Warden | CampKind::Brute);
        let score = travel + wait * 1.5 - if buff { 6.0 } else { 0.0 } + if ours { 0.0 } else { 4.0 };
        if best.is_none_or(|(s, _)| score < s) {
            best = Some((score, i));
        }
    }
    best.map(|(_, i)| i)
}

/// The camp's monsters standing (that we can see).
fn monsters<'a, 'w>(c: &'a Ctx<'a, 'w>, camp: usize) -> impl Iterator<Item = &'w Unit> + 'a {
    c.b.monsters
        .iter()
        .copied()
        .filter(move |u| matches!(u.brain, Some(Brain::Monster { camp: x, .. }) if x as usize == camp))
}

/// Take camp `i`: walk there, hit its weakest monster (area abilities when they catch two or a
/// big one), finish a big one with Claim, and Claim earlier when an enemy could steal it.
pub fn clear(a: &mut Agent, c: &Ctx, i: usize) -> Option<CommandKind> {
    let me = c.me;
    let camp = c.k.camps.get(i)?;
    let mut list: Vec<&Unit> = monsters(c, i).collect();
    if list.is_empty() {
        if me.state.pos.distance(camp.pos) > 250.0 {
            return a.go(c, camp.pos);
        }
        return None;
    }
    list.sort_by(|x, y| x.state.health.total_cmp(&y.state.health));
    let target = list[0];
    // Claim a big monster when Claim would finish it (sooner when an enemy is close).
    if let Some(Effect::Claim(cl)) = me.ability(5).map(|ab| ab.effect)
        && me.state.can_cast(c.now, 5)
    {
        let power =
            if me.state.progress.camps >= mftr_sim::jungle::CLAIM_UPGRADE_CAMPS { cl.upgraded } else { cl.damage };
        let contested = c.b.enemies.iter().any(|e| e.state.pos.distance(camp.pos) < 1300.0);
        for m in &list {
            let big = m.monster.is_some_and(|k| k.def().big);
            let margin = if contested { 1.35 } else { 1.0 };
            if big
                && m.state.health <= power * margin
                && m.state.pos.distance(me.state.pos) <= cl.range + m.gameplay_radius
            {
                return Some(micro::cast(5, m.state.pos));
            }
        }
    }
    // Area abilities on the pack.
    for slot in 0..3u8 {
        if !me.state.can_cast(c.now, slot) {
            continue;
        }
        let Some(ab) = me.ability(slot) else { continue };
        if !kit::use_of(&ab).damage || kit::use_of(&ab).onto_enemy && list.len() > 1 {
            continue;
        }
        let d = me.state.pos.distance(target.state.pos);
        if d > kit::reach(&ab) + target.gameplay_radius {
            continue;
        }
        let worth = list.len() >= 2 || target.monster.is_some_and(|k| k.def().big);
        if worth && a.rng_f32() < a.p.ability_use * 0.8 {
            let at = match ab.effect {
                Effect::Area(ar) if ar.range == 0.0 => me.state.pos,
                _ => target.state.pos,
            };
            return Some(micro::cast(slot, at));
        }
    }
    Some(CommandKind::Attack(target.id))
}
