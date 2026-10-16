//! What a champion's kit is for, read from the ability data (17 §2, §4): so augments, items and
//! new champions need no bot code. Expected damage, reach, time to impact and the chance an
//! ability lands on a given target.

use mftr_sim::ability::{Ability, Cc, DamageKind, Effect};
use mftr_sim::champion::max_rank;
use mftr_sim::combat::resist_multiplier;
use mftr_sim::{ChampionId, SimTime, Unit, Vec2};

/// What a champion is, in a fight (01 §9's classes).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Archetype {
    Mage,
    Marksman,
    Tank,
    Bruiser,
    Enchanter,
    Assassin,
    Artillery,
    Warden,
    Battlemage,
    Skirmisher,
}

impl Archetype {
    pub fn of(c: ChampionId) -> Archetype {
        match c {
            ChampionId::Ember => Archetype::Mage,
            ChampionId::Vesper => Archetype::Marksman,
            ChampionId::Bastion => Archetype::Tank,
            ChampionId::Rook => Archetype::Bruiser,
            ChampionId::Lumen => Archetype::Enchanter,
            ChampionId::Shade => Archetype::Assassin,
            ChampionId::Quill => Archetype::Artillery,
            ChampionId::Cairn => Archetype::Warden,
            ChampionId::Marrow => Archetype::Battlemage,
            ChampionId::Wren => Archetype::Skirmisher,
        }
    }

    /// Stands in front: takes the hits and starts or holds fights.
    pub fn front(self) -> bool {
        matches!(self, Archetype::Tank | Archetype::Bruiser | Archetype::Warden)
    }

    /// Protects others rather than carrying.
    pub fn support(self) -> bool {
        matches!(self, Archetype::Enchanter | Archetype::Warden)
    }

    /// Deals its damage from range, behind the front line.
    pub fn back(self) -> bool {
        matches!(self, Archetype::Mage | Archetype::Marksman | Archetype::Artillery | Archetype::Enchanter)
    }
}

/// Classic's roles (01 §9).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Role {
    Top,
    Jungle,
    Mid,
    Bottom,
    Support,
}

impl Role {
    pub const ALL: [Role; 5] = [Role::Top, Role::Jungle, Role::Mid, Role::Bottom, Role::Support];

    /// The lane a role plays on a three-lane map (top 0, mid 1, bottom 2), or `None` (jungle).
    pub fn lane(self) -> Option<usize> {
        match self {
            Role::Top => Some(0),
            Role::Mid => Some(1),
            Role::Bottom | Role::Support => Some(2),
            Role::Jungle => None,
        }
    }

    /// How well `c` plays this role, 0–1.
    pub fn fit(self, c: ChampionId) -> f32 {
        use Archetype as A;
        match (Archetype::of(c), self) {
            (A::Mage, Role::Mid) => 1.0,
            (A::Mage, Role::Support) => 0.5,
            (A::Marksman, Role::Bottom) => 1.0,
            (A::Tank, Role::Support) => 0.9,
            (A::Tank, Role::Top) => 0.8,
            (A::Tank, Role::Jungle) => 0.6,
            (A::Bruiser, Role::Top) => 1.0,
            (A::Bruiser, Role::Jungle) => 0.8,
            (A::Enchanter, Role::Support) => 1.0,
            (A::Assassin, Role::Jungle) => 1.0,
            (A::Assassin, Role::Mid) => 0.8,
            (A::Artillery, Role::Mid) => 0.9,
            (A::Artillery, Role::Support) => 0.6,
            (A::Warden, Role::Support) => 1.0,
            (A::Warden, Role::Top) => 0.6,
            (A::Battlemage, Role::Top) => 0.8,
            (A::Battlemage, Role::Mid) => 0.8,
            (A::Battlemage, Role::Jungle) => 0.6,
            (A::Skirmisher, Role::Bottom) => 0.9,
            (A::Skirmisher, Role::Jungle) => 0.7,
            _ => 0.2,
        }
    }
}

/// The best assignment of `roles` to `champions` (each role at most once): the one with the
/// highest total fit. `fixed` entries (Some) keep their role (humans' choices).
pub fn assign_roles(champions: &[ChampionId], fixed: &[Option<Role>]) -> Vec<Role> {
    let n = champions.len().min(5);
    // Every ordering of the five roles (120), the first `n` of each given out in order.
    let mut best: (f32, Vec<Role>) = (f32::NEG_INFINITY, Vec::new());
    let mut order: Vec<usize> = (0..5).collect();
    let mut c = [0usize; 5];
    let mut consider = |order: &[usize]| {
        let roles: Vec<Role> = order[..n].iter().map(|&i| Role::ALL[i]).collect();
        if roles.iter().zip(fixed).any(|(r, f)| f.is_some_and(|f| f != *r)) {
            return;
        }
        let score: f32 = roles.iter().zip(champions).map(|(r, ch)| r.fit(*ch)).sum();
        if score > best.0 {
            best = (score, roles);
        }
    };
    // Heap's algorithm.
    consider(&order);
    let mut i = 0;
    while i < 5 {
        if c[i] < i {
            if i % 2 == 0 {
                order.swap(0, i)
            } else {
                order.swap(c[i], i)
            }
            consider(&order);
            c[i] += 1;
            i = 0;
        } else {
            c[i] = 0;
            i += 1;
        }
    }
    // More than five (custom games): the rest double up on lanes.
    let mut roles = best.1;
    while roles.len() < champions.len() {
        roles.push(Role::ALL[roles.len() % 5]);
    }
    roles
}

/// What an ability does for a bot.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Use {
    pub damage: bool,
    /// Stuns, roots, knock-ups and pulls.
    pub hard_cc: bool,
    pub slow: bool,
    /// Moves the caster (dashes, blinks; lunges move onto a target).
    pub moves: bool,
    /// Moves onto an enemy (lunges).
    pub onto_enemy: bool,
    /// Heals or shields.
    pub sustain: bool,
    /// Heals or shields an ally (else only the caster).
    pub ally: bool,
    /// Centered on the caster.
    pub nova: bool,
    /// Lands only where it's aimed after a delay or a flight (a skillshot or a delayed area).
    pub skillshot: bool,
    /// A line missile that stops at the first enemy it touches.
    pub blocked_by_units: bool,
}

pub fn use_of(a: &Ability) -> Use {
    let hard = |cc: Cc| cc.is_hard();
    let slow = |cc: Cc| matches!(cc, Cc::Slow { .. });
    match a.effect {
        Effect::Line(l) => Use {
            damage: true,
            hard_cc: hard(l.cc),
            slow: slow(l.cc),
            skillshot: true,
            blocked_by_units: true,
            ..Use::default()
        },
        Effect::Area(ar) => Use {
            damage: true,
            hard_cc: hard(ar.cc),
            slow: slow(ar.cc) || slow(ar.pulses.cc),
            nova: ar.range == 0.0,
            skillshot: ar.range > 0.0 || ar.delay.0 > 0,
            ..Use::default()
        },
        Effect::Beam(b) => Use { damage: true, hard_cc: hard(b.cc), skillshot: true, ..Use::default() },
        Effect::Lunge(l) => {
            Use { damage: true, hard_cc: hard(l.cc), slow: slow(l.cc), moves: true, onto_enemy: true, ..Use::default() }
        }
        Effect::Dash(_) | Effect::Blink(_) => Use { moves: true, ..Use::default() },
        Effect::Shield(_) => Use { sustain: true, ..Use::default() },
        Effect::Support(s) => Use { sustain: true, ally: s.range > 0.0, ..Use::default() },
        Effect::Claim(_) => Use { damage: true, ..Use::default() },
    }
}

/// How far from the caster an ability reaches a target's center (plus the target's radius for
/// areas and beams).
pub fn reach(a: &Ability) -> f32 {
    match a.effect {
        Effect::Line(l) => l.range,
        Effect::Area(ar) if ar.range == 0.0 => ar.radius,
        Effect::Area(ar) => ar.range + ar.radius * 0.5,
        Effect::Beam(b) => b.range,
        Effect::Lunge(l) => l.range,
        Effect::Dash(d) => d.range,
        Effect::Blink(b) => b.range,
        Effect::Support(s) => s.range,
        Effect::Shield(_) => 0.0,
        Effect::Claim(c) => c.range,
    }
}

/// Seconds from the cast until it lands on something `dist` away.
pub fn impact_time(a: &Ability, dist: f32) -> f32 {
    let s = |d: mftr_sim::SimDuration| d.0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32;
    match a.effect {
        Effect::Line(l) => s(l.windup) + dist / l.speed.max(1.0),
        Effect::Area(ar) => s(ar.windup) + s(ar.delay),
        Effect::Beam(b) => s(b.windup),
        Effect::Lunge(l) => dist / l.speed.max(1.0),
        _ => 0.0,
    }
}

/// An ability's half-width for dodging (lines, beams) or radius (areas).
pub fn width(a: &Ability) -> f32 {
    match a.effect {
        Effect::Line(l) => l.radius,
        Effect::Area(ar) => ar.radius,
        Effect::Beam(b) => b.radius,
        _ => 0.0,
    }
}

/// The ability's hard crowd control, in seconds (pulls count as half a second).
pub fn cc_seconds(a: &Ability) -> f32 {
    let cc = match a.effect {
        Effect::Line(l) => l.cc,
        Effect::Area(ar) => ar.cc,
        Effect::Beam(b) => b.cc,
        Effect::Lunge(l) => l.cc,
        _ => Cc::None,
    };
    let s = |d: mftr_sim::SimDuration| d.0 as f32 / mftr_sim::time::SUBTICKS_PER_SECOND as f32;
    match cc {
        Cc::Stun(d) | Cc::Root(d) | Cc::Knockup(d) => s(d),
        Cc::Pull(_) => 0.5,
        _ => 0.0,
    }
}

/// An ability's rank: the caster's own for itself and its allies; for an enemy, what its level
/// usually allows (the usual order: the ultimate whenever it can, then the rest evenly).
pub fn rank(u: &Unit, slot: u8, known: bool) -> u8 {
    if slot >= 4 {
        return 1;
    }
    if known {
        return u.state.progress.ranks[slot as usize];
    }
    let level = u.state.progress.level.max(1);
    if slot == 3 {
        max_rank(3, level)
    } else {
        let basics = level.saturating_sub(max_rank(3, level));
        ((basics + 2 - slot.min(2)) / 3).clamp(1, max_rank(slot, level))
    }
}

/// Raw damage (before resistances) the ability deals one target, with the caster's stats, at
/// `rank`: its strike, plus half of a lingering area's or beam's further strikes (targets walk
/// out of them).
pub fn raw_damage(a: &Ability, caster: &Unit, rank: u8) -> (DamageKind, f32) {
    let (ad, ap) = (caster.stats.attack_damage, caster.stats.ability_power);
    let bonus = a.bonus_damage_at(rank);
    match a.effect {
        Effect::Line(l) => (l.damage.kind, l.damage.raw(ad, ap) + bonus),
        Effect::Area(ar) => {
            let strike = ar.damage.raw(ad, ap) + bonus;
            (ar.damage.kind, strike * (1.0 + 0.5 * ar.pulses.count as f32 * ar.pulses.share))
        }
        Effect::Beam(b) => (b.damage.kind, (b.damage.raw(ad, ap) + bonus) * 0.75),
        Effect::Lunge(l) => (l.damage.kind, l.damage.raw(ad, ap) + bonus),
        Effect::Claim(c) => {
            let upgraded = caster.state.progress.camps >= mftr_sim::jungle::CLAIM_UPGRADE_CAMPS;
            (DamageKind::True, if upgraded { c.upgraded } else { c.damage })
        }
        _ => (DamageKind::True, 0.0),
    }
}

/// `raw` damage of `kind` after `target`'s resistances.
pub fn mitigated(kind: DamageKind, raw: f32, target: &Unit) -> f32 {
    match kind {
        DamageKind::Physical => raw * resist_multiplier(target.stats.armor),
        DamageKind::Magic => raw * resist_multiplier(target.stats.magic_resist),
        DamageKind::True => raw,
    }
}

/// A basic attack's damage on `target` (physical; on-hit passives aside).
pub fn attack_damage(attacker: &Unit, target: &Unit) -> f32 {
    attacker.stats.attack_damage * resist_multiplier(target.stats.armor)
}

/// How long `u` can't move from `now` (hard crowd control, a wind-up, a channel, a recall).
pub fn locked_for(u: &Unit, now: SimTime) -> f32 {
    let st = &u.state;
    let mut until = st.stunned_until.max(st.rooted_until);
    if let Some(c) = st.cast.filter(|c| !c.mobile) {
        until = until.max(c.fire_at);
    }
    if let Some(w) = st.attack {
        until = until.max(w.fire_at);
    }
    if let Some(r) = st.recovery {
        until = until.max(r.hard_until);
    }
    if st.recalling() {
        until = until.max(now.plus(mftr_sim::SimDuration::from_millis(1000)));
    }
    if until > now { until.secs_since(now) } else { 0.0 }
}

/// How quickly a human (or a bot) answers a skillshot it sees coming: the allowance in the hit
/// model.
pub const DODGE_REACTION: f32 = 0.3;

/// The chance `a`, cast now by `caster` at `target`'s predicted spot, lands (17 §4). `blocked`:
/// something else stands in a line missile's path first.
pub fn hit_chance(a: &Ability, caster: &Unit, target: &Unit, now: SimTime, blocked: bool) -> f32 {
    let d = caster.state.pos.distance(target.state.pos);
    let u = use_of(a);
    if u.onto_enemy || matches!(a.effect, Effect::Claim(_)) {
        return if d <= reach(a) + target.gameplay_radius { 1.0 } else { 0.0 };
    }
    if u.blocked_by_units && blocked {
        return 0.0;
    }
    if !u.damage {
        return 0.0;
    }
    if u.nova && !u.skillshot {
        // An instant nova: it lands if the target is inside at the wind-up's end.
        let room = width(a) + target.gameplay_radius - d;
        let t = impact_time(a, d);
        let escape = target.state.speed_at(now) * (t - locked_for(target, now)).max(0.0);
        return if room > escape {
            0.95
        } else if room > 0.0 {
            0.6
        } else {
            0.0
        };
    }
    let t = impact_time(a, d);
    let free = (t - locked_for(target, now) - DODGE_REACTION).max(0.0);
    let can_cover = target.state.speed_at(now) * free;
    let need = width(a) + target.gameplay_radius;
    if can_cover < need * 0.9 { 0.95 } else { (need / can_cover).clamp(0.08, 0.85) }
}

/// The value of landing `a` on `target`: its damage after resistances, plus its crowd control
/// valued as damage the caster's side deals meanwhile (`team_dps`).
pub fn value_on(a: &Ability, caster: &Unit, target: &Unit, rank: u8, team_dps: f32) -> f32 {
    let (kind, raw) = raw_damage(a, caster, rank);
    mitigated(kind, raw, target) + cc_seconds(a) * team_dps
}

/// A line from `from` toward `to` passes within reach of an `other` unit before reaching `to`
/// (whose radius the caller gives): the first one hit would be it.
pub fn line_blocked(from: Vec2, to: Vec2, half_width: f32, others: impl Iterator<Item = (Vec2, f32)>) -> bool {
    let seg = to - from;
    let len = seg.length();
    if len < 1.0 {
        return false;
    }
    let dir = seg * (1.0 / len);
    others.into_iter().any(|(p, r)| {
        let rel = p - from;
        let along = rel.dot(dir);
        along > 0.0 && along < len - r && (rel - dir * along).length() < half_width + r
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roles_go_to_the_champions_that_fit_them() {
        use ChampionId::*;
        let roles = assign_roles(&[Vesper, Lumen, Rook, Shade, Ember], &[None; 5]);
        assert_eq!(roles, [Role::Bottom, Role::Support, Role::Top, Role::Jungle, Role::Mid]);
        // A human's role is kept, the rest rearrange around it.
        let roles = assign_roles(&[Vesper, Lumen, Rook, Shade, Ember], &[None, None, None, Some(Role::Mid), None]);
        assert_eq!(roles[3], Role::Mid);
        assert!(!roles.iter().enumerate().any(|(i, r)| i != 3 && *r == Role::Mid));
    }

    #[test]
    fn an_enemys_ranks_follow_its_level() {
        let mut u = mftr_sim::world::Unit::new(
            mftr_sim::UnitId(1),
            mftr_sim::UnitKind::Champion,
            mftr_sim::Team::Red,
            Vec2::ZERO,
            (65.0, 65.0),
            mftr_sim::champion::Stats::NONE,
        );
        for level in 1..=18u8 {
            u.state.progress.level = level;
            let total: u8 = (0..4).map(|s| rank(&u, s, false)).sum();
            assert!(total <= level.max(3) + 1, "level {level}: {total}");
            assert_eq!(rank(&u, 3, false), max_rank(3, level));
        }
    }
}
