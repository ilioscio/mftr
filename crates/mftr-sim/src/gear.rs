//! What items do in a match (18 §4): the item state a champion keeps through death
//! ([`Gear`]: cooldowns, stacks, the quest), the timed effects on any unit ([`Fx`]: Wounds,
//! burns, Sunder, Expose, speed buffs, stasis), the attacker's side of the damage pipeline
//! ([`Hitter`]), heals and shields, the effects that follow a tick's damage, and the per-tick
//! item effects (burn and Smolder pulses, Warmth, Light feet, charges, the quest's stages).

use crate::ability::DamageKind;
use crate::champion::Stats;
use crate::combat::{Penetration, effective_resist, lethality_to_flat_pen, resist_multiplier};
use crate::hash::StateSink;
use crate::items::{self, INVENTORY, Passive, Passives};
use crate::time::{SUBTICKS_PER_SECOND, SimDuration, SimTime};
use crate::world::{DamageOrigin, SimEvent, Unit, UnitId, UnitKind, UnitState};

/// Passive cooldowns in `Gear::passive_ready`.
pub const SPELLBLADE: usize = 0;
pub const AMBUSH: usize = 1;
pub const RALLY: usize = 2;
pub const BULWARK: usize = 3;
pub const PASSIVE_COOLDOWNS: usize = 4;

/// Spellblade: an ability arms it this long; then its cooldown.
pub const SPELLBLADE_WINDOW: SimDuration = SimDuration::from_millis(10_000);
pub const SPELLBLADE_COOLDOWN: SimDuration = SimDuration::from_millis(1500);
/// Ambush: the second hit must land within this of the first.
pub const AMBUSH_WINDOW: SimDuration = SimDuration::from_millis(2000);
pub const AMBUSH_SHIELD: SimDuration = SimDuration::from_millis(2000);
/// Wounds last this long (refreshed by each hit).
pub const WOUNDS_FOR: SimDuration = SimDuration::from_millis(3000);
/// Burns (Searing) and Smolder strike this often.
pub const BURN_EVERY: SimDuration = SimDuration::from_millis(500);
pub const SMOLDER_EVERY: SimDuration = SimDuration::from_millis(1000);
/// Smolder burns while its holder dealt or took damage this recently.
pub const IN_COMBAT: SimDuration = SimDuration::from_millis(3000);

/// Item state that survives death (`Progress::gear`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gear {
    /// When each inventory slot's active is ready again (it moves with its item).
    pub ready: [SimTime; INVENTORY],
    /// When each passive with a cooldown is ready again (`SPELLBLADE`, `AMBUSH`, `RALLY`,
    /// `BULWARK`).
    pub passive_ready: [SimTime; PASSIVE_COOLDOWNS],
    /// Tally stacks.
    pub tally: u8,
    /// Gold earned for the Pilgrim's quest.
    pub quest: u16,
    /// Tithe and Spoils charges spent, and when the next one comes back.
    pub tithe_spent: u8,
    pub tithe_at: SimTime,
    pub spoils_spent: u8,
    pub spoils_at: SimTime,
    /// Monsters fed to the hatchling (1 a small one, 2 a big one).
    pub food: u8,
}

impl Gear {
    pub const NONE: Gear = Gear {
        ready: [SimTime(0); INVENTORY],
        passive_ready: [SimTime(0); PASSIVE_COOLDOWNS],
        tally: 0,
        quest: 0,
        tithe_spent: 0,
        tithe_at: SimTime(0),
        spoils_spent: 0,
        spoils_at: SimTime(0),
        food: 0,
    };

    pub fn is_default(&self) -> bool {
        *self == Gear::NONE
    }

    pub fn hash_into(&self, h: &mut impl StateSink) {
        // Without item state, the hash is what it was before items had any.
        if self.is_default() {
            return;
        }
        for t in self.ready.iter().chain(&self.passive_ready) {
            h.write_u64(t.0);
        }
        h.write_u8(self.tally);
        h.write_u16(self.quest);
        h.write_u8(self.tithe_spent);
        h.write_u64(self.tithe_at.0);
        h.write_u8(self.spoils_spent);
        h.write_u64(self.spoils_at.0);
        h.write_u8(self.food);
    }

    /// The hatchling is grown up.
    pub fn grown(&self) -> bool {
        self.food >= items::hatchling::GROWN
    }
}

/// Timed item effects on a unit (`UnitState::fx`); all of it ends with death.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fx {
    /// Wounds: healing × (1 − `wounds` %) until then.
    pub wounds: u8,
    pub wounds_until: SimTime,
    /// A burn (Searing): `burn` magic damage a second from `burn_by`'s item `burn_item`, struck
    /// every `BURN_EVERY` from `burn_next` until `burn_until`.
    pub burn: f32,
    pub burn_by: UnitId,
    pub burn_item: u8,
    pub burn_next: SimTime,
    pub burn_until: SimTime,
    /// Sunder: stacks of `sunder_pct` % armor reduction.
    pub sunder: u8,
    pub sunder_pct: u8,
    pub sunder_until: SimTime,
    /// Expose: armor and magic resist reduced by this %.
    pub expose: u8,
    pub expose_until: SimTime,
    /// A movement speed buff in %: flat until `haste_until`, or fading to 0 over the last
    /// `haste_fade` ms when that's not 0.
    pub haste: u8,
    pub haste_until: SimTime,
    pub haste_fade: u16,
    /// Quicken's flat movement speed, and Light feet's (set each tick).
    pub quick: u8,
    pub quick_until: SimTime,
    pub light: u8,
    /// Stasis (Encase): untargetable and unable to act.
    pub stasis_until: SimTime,
    /// Petrify: the unit deals `petrify_cut` % less damage.
    pub petrify_cut: u8,
    pub petrify_until: SimTime,
    /// Knocked up or pulled until then (cleanses don't end it).
    pub airborne_until: SimTime,
    /// Last damage dealt to or taken from a champion; last damage taken; last damage dealt.
    pub combat_at: SimTime,
    pub hurt_at: SimTime,
    pub dealt_at: SimTime,
    /// Ambush: the champion first hit, and when.
    pub combo: UnitId,
    pub combo_at: SimTime,
    /// Plunge's mark: `mark_by` deals `mark_amp` % more to this unit until then.
    pub mark_by: UnitId,
    pub mark_amp: u8,
    pub mark_until: SimTime,
    /// Kindle: this unit's attacks deal this much extra magic damage until then.
    pub kindle: u8,
    pub kindle_until: SimTime,
    /// Smolder's next pulse.
    pub smolder_at: SimTime,
}

impl Fx {
    pub const NONE: Fx = Fx {
        wounds: 0,
        wounds_until: SimTime(0),
        burn: 0.0,
        burn_by: UnitId(0),
        burn_item: 0,
        burn_next: SimTime(0),
        burn_until: SimTime(0),
        sunder: 0,
        sunder_pct: 0,
        sunder_until: SimTime(0),
        expose: 0,
        expose_until: SimTime(0),
        haste: 0,
        haste_until: SimTime(0),
        haste_fade: 0,
        quick: 0,
        quick_until: SimTime(0),
        light: 0,
        stasis_until: SimTime(0),
        petrify_cut: 0,
        petrify_until: SimTime(0),
        airborne_until: SimTime(0),
        combat_at: SimTime(0),
        hurt_at: SimTime(0),
        dealt_at: SimTime(0),
        combo: UnitId(0),
        combo_at: SimTime(0),
        mark_by: UnitId(0),
        mark_amp: 0,
        mark_until: SimTime(0),
        kindle: 0,
        kindle_until: SimTime(0),
        smolder_at: SimTime(0),
    };

    pub fn is_default(&self) -> bool {
        *self == Fx::NONE
    }

    pub fn hash_into(&self, h: &mut impl StateSink) {
        if self.is_default() {
            return;
        }
        h.write_u8(self.wounds);
        h.write_u64(self.wounds_until.0);
        h.write_f32(self.burn);
        h.write_u32(self.burn_by.0);
        h.write_u8(self.burn_item);
        h.write_u64(self.burn_next.0);
        h.write_u64(self.burn_until.0);
        h.write_u8(self.sunder);
        h.write_u8(self.sunder_pct);
        h.write_u64(self.sunder_until.0);
        h.write_u8(self.expose);
        h.write_u64(self.expose_until.0);
        h.write_u8(self.haste);
        h.write_u64(self.haste_until.0);
        h.write_u16(self.haste_fade);
        h.write_u8(self.quick);
        h.write_u64(self.quick_until.0);
        h.write_u8(self.light);
        h.write_u64(self.stasis_until.0);
        h.write_u8(self.petrify_cut);
        h.write_u64(self.petrify_until.0);
        h.write_u64(self.airborne_until.0);
        h.write_u64(self.combat_at.0);
        h.write_u64(self.hurt_at.0);
        h.write_u64(self.dealt_at.0);
        h.write_u32(self.combo.0);
        h.write_u64(self.combo_at.0);
        h.write_u32(self.mark_by.0);
        h.write_u8(self.mark_amp);
        h.write_u64(self.mark_until.0);
        h.write_u8(self.kindle);
        h.write_u64(self.kindle_until.0);
        h.write_u64(self.smolder_at.0);
    }

    /// Movement speed multiplier and flat bonus from item buffs at `at`.
    pub fn speed(&self, at: SimTime) -> (f32, f32) {
        let mut pct = 0.0;
        if self.haste > 0 && self.haste_until > at {
            pct = self.haste as f32 / 100.0;
            if self.haste_fade > 0 {
                let left = (self.haste_until.0 - at.0) as f32 * 1000.0 / SUBTICKS_PER_SECOND as f32;
                pct *= (left / self.haste_fade as f32).min(1.0);
            }
        }
        let quick = if self.quick_until > at { self.quick as f32 } else { 0.0 };
        (1.0 + pct, quick + self.light as f32)
    }

    /// The unit is in stasis at `at`.
    pub fn encased(&self, at: SimTime) -> bool {
        self.stasis_until > at
    }

    /// Wounds' cut at `at` (0–1).
    pub fn wounds_at(&self, at: SimTime) -> f32 {
        if self.wounds_until > at { self.wounds as f32 / 100.0 } else { 0.0 }
    }

    /// Give a speed buff: the stronger applies.
    pub fn hasten(&mut self, pct: u8, until: SimTime, fade_ms: u16, at: SimTime) {
        if self.haste_until <= at || pct >= self.haste {
            (self.haste, self.haste_until, self.haste_fade) = (pct, until, fade_ms);
        }
    }

    /// Apply Wounds: the strongest applies, each hit refreshes.
    pub fn wound(&mut self, pct: f32, at: SimTime) {
        let p = (pct * 100.0).round() as u8;
        if self.wounds_until <= at || p >= self.wounds {
            self.wounds = p;
        }
        self.wounds_until = at.plus(WOUNDS_FOR);
    }
}

/// The attacker's side of a hit (02 §4–§5): who, its penetration and its outgoing modifiers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hitter {
    pub id: UnitId,
    pub kind: UnitKind,
    /// Armor and magic penetration (lethality already converted by level).
    pub armor_pen: Penetration,
    pub magic_pen: Penetration,
    /// Outgoing damage × this (Petrify).
    pub out: f32,
    /// Topple: up to `amp` more against more max health, full at `full_at` more; and the
    /// attacker's max health.
    pub topple: Option<(f32, f32)>,
    pub max_health: f32,
    /// A hatchling's holder: more damage to monsters.
    pub hatchling: bool,
}

impl Hitter {
    /// A source without a unit (a fountain) or without modifiers.
    pub fn plain(id: UnitId, kind: UnitKind) -> Hitter {
        Hitter {
            id,
            kind,
            armor_pen: Penetration::default(),
            magic_pen: Penetration::default(),
            out: 1.0,
            topple: None,
            max_health: 0.0,
            hatchling: false,
        }
    }

    pub fn of(u: &Unit, at: SimTime) -> Hitter {
        if u.kind != UnitKind::Champion {
            return Hitter::plain(u.id, u.kind);
        }
        let s = &u.stats;
        let p = items::passives(&u.state.progress.items);
        let fx = &u.state.fx;
        Hitter {
            id: u.id,
            kind: u.kind,
            armor_pen: Penetration {
                percent_pen: s.armor_pen,
                flat_pen: lethality_to_flat_pen(s.lethality, u.state.progress.level),
                ..Default::default()
            },
            magic_pen: Penetration { percent_pen: s.magic_pen_pct, flat_pen: s.magic_pen, ..Default::default() },
            out: if fx.petrify_until > at { 1.0 - fx.petrify_cut as f32 / 100.0 } else { 1.0 },
            topple: match p.find(|p| matches!(p, Passive::Topple { .. })) {
                Some((Passive::Topple { amp, full_at }, _)) => Some((amp, full_at)),
                _ => None,
            },
            max_health: s.max_health,
            hatchling: p.find(|p| matches!(p, Passive::Hatchling(_))).is_some(),
        }
    }

    /// The hitter for unit `id` among `units` (a plain one if it's gone).
    pub fn find(units: &[Unit], id: UnitId, at: SimTime) -> Hitter {
        units.iter().find(|u| u.id == id).map_or(Hitter::plain(id, UnitKind::Minion), |u| Hitter::of(u, at))
    }
}

/// What `raw` damage of `kind` from `by` comes to on `u` after its resistances (with the
/// target's reductions and the attacker's penetration, 02 §4), the attacker's outgoing
/// modifiers and the target's damage cuts (02 §5 steps 3–6). `attack`: a basic attack.
pub fn mitigate(u: &Unit, by: &Hitter, raw: f32, kind: DamageKind, attack: bool, at: SimTime) -> f32 {
    let fx = &u.state.fx;
    let mut amount = raw * by.out;
    // Topple, Plunge's mark, a hatchling against monsters.
    if let Some((amp, full_at)) = by.topple
        && u.kind == UnitKind::Champion
    {
        amount *= 1.0 + amp * ((u.stats.max_health - by.max_health) / full_at).clamp(0.0, 1.0);
    }
    if fx.mark_by == by.id && fx.mark_until > at {
        amount *= 1.0 + fx.mark_amp as f32 / 100.0;
    }
    if by.hatchling && u.kind == UnitKind::Monster {
        amount *= items::hatchling::MONSTER_AMP;
    }
    let reduce = |base: f32| {
        let sunder = if fx.sunder_until > at { fx.sunder as f32 * fx.sunder_pct as f32 / 100.0 } else { 0.0 };
        let expose = if fx.expose_until > at { fx.expose as f32 / 100.0 } else { 0.0 };
        (base, (1.0 - (1.0 - sunder) * (1.0 - expose)).clamp(0.0, 1.0), expose)
    };
    amount = match kind {
        DamageKind::Physical => {
            let (armor, red, _) = reduce(u.stats.armor);
            let pen = Penetration { percent_reduction: red, ..by.armor_pen };
            amount * resist_multiplier(effective_resist(armor, &pen))
        }
        DamageKind::Magic => {
            let (mr, _, expose) = reduce(u.stats.magic_resist);
            let pen = Penetration { percent_reduction: expose, ..by.magic_pen };
            amount * resist_multiplier(effective_resist(mr, &pen))
        }
        DamageKind::True => amount,
    };
    if u.kind == UnitKind::Champion {
        let p = items::passives(&u.state.progress.items);
        if attack && let Some((Passive::AttackCut(c), _)) = p.find(|p| matches!(p, Passive::AttackCut(_))) {
            amount *= 1.0 - c;
        }
        if by.kind == UnitKind::Monster && p.find(|p| matches!(p, Passive::Hatchling(_))).is_some() {
            amount *= items::hatchling::MONSTER_CUT;
        }
    }
    amount
}

/// A crit's damage multiplier from `crit_damage` (bonus) against `target` (CritCut).
pub fn crit_multiplier(crit_damage: f32, target: &Unit) -> f32 {
    let cut = match items::passives(&target.state.progress.items).find(|p| matches!(p, Passive::CritCut(_))) {
        Some((Passive::CritCut(c), _)) if target.kind == UnitKind::Champion => c,
        _ => 0.0,
    };
    1.0 + (items::CRIT_DAMAGE + crit_damage - 1.0) * (1.0 - cut)
}

/// Heal `u` by `amount` at `at`, after Wounds and healing received (not the fountain's).
/// Returns the health restored.
pub fn heal(u: &mut Unit, amount: f32, at: SimTime, events: &mut Vec<SimEvent>) -> f32 {
    let (id, stats) = (u.id, u.stats);
    heal_state(&mut u.state, &stats, id, amount, at, events)
}

/// [`heal`] on a unit's state and stats.
pub fn heal_state(
    st: &mut UnitState,
    stats: &Stats,
    id: UnitId,
    amount: f32,
    at: SimTime,
    events: &mut Vec<SimEvent>,
) -> f32 {
    if !st.alive() || amount <= 0.0 {
        return 0.0;
    }
    let factor = (1.0 + stats.heal_received) * (1.0 - st.fx.wounds_at(at));
    let gained = (amount * factor).min(stats.max_health - st.health).max(0.0);
    if gained > 0.0 {
        st.health += gained;
        events.push(SimEvent::Healed { unit: id, amount: gained, at });
    }
    gained
}

/// How much a heal over time (potions, regeneration) is worth on `u` at `at`.
pub fn heal_factor(u: &Unit, at: SimTime) -> f32 {
    (1.0 + u.stats.heal_received) * (1.0 - u.state.fx.wounds_at(at))
}

/// Shield `u` with `amount` (after healing received) until `until`: added to a shield still
/// up, which lasts as long as the longer of the two.
pub fn shield(u: &mut Unit, amount: f32, until: SimTime, at: SimTime, events: &mut Vec<SimEvent>) {
    let (id, stats) = (u.id, u.stats);
    shield_state(&mut u.state, &stats, id, amount, until, at, events);
}

/// [`shield`] on a unit's state and stats.
pub fn shield_state(
    st: &mut UnitState,
    stats: &Stats,
    id: UnitId,
    amount: f32,
    until: SimTime,
    at: SimTime,
    events: &mut Vec<SimEvent>,
) {
    if !st.alive() || amount <= 0.0 {
        return;
    }
    let amount = amount * (1.0 + stats.heal_received);
    st.shield = if st.shield_until > at { st.shield + amount } else { amount };
    st.shield_until = st.shield_until.max(until);
    events.push(SimEvent::Shielded { unit: id, amount, at, until: st.shield_until });
}

/// A champion's bonus health (over its level's base), for "% bonus health" effects.
pub fn bonus_health(u: &Unit) -> f32 {
    let base = u.champion.map_or(u.stats.max_health, |c| c.def().stats_at(u.state.progress.level).max_health);
    (u.stats.max_health - base).max(0.0)
}

/// Bonus attack damage (over the level's base).
pub fn bonus_ad(u: &Unit) -> f32 {
    let base = u.champion.map_or(u.stats.attack_damage, |c| c.def().stats_at(u.state.progress.level).attack_damage);
    (u.stats.attack_damage - base).max(0.0)
}

/// Bonus armor.
pub fn bonus_armor(u: &Unit) -> f32 {
    let base = u.champion.map_or(u.stats.armor, |c| c.def().stats_at(u.state.progress.level).armor);
    (u.stats.armor - base).max(0.0)
}

/// The counted passives of a unit's inventory (none for non-champions).
pub fn of(u: &Unit) -> Passives {
    if u.kind == UnitKind::Champion { items::passives(&u.state.progress.items) } else { Passives::NONE }
}

fn ms(v: u64) -> SimDuration {
    SimDuration::from_millis(v)
}

/// Which combat clocks a unit's passives read (only those are kept, so units without such
/// items keep the state, and the hash, they had before items).
#[derive(Clone, Copy)]
struct Clocks {
    /// Last damage taken (Warmth, Smolder), dealt (Smolder), champion combat (Light feet, a
    /// Barkling).
    hurt: bool,
    dealt: bool,
    combat: bool,
}

impl Clocks {
    fn note(self, fx: &mut Fx, at: SimTime, dealing: bool, champions: bool) {
        if !dealing && self.hurt {
            fx.hurt_at = at;
        }
        if dealing && self.dealt {
            fx.dealt_at = at;
        }
        if champions && self.combat {
            fx.combat_at = at;
        }
    }
}

fn clocks(p: &Passives) -> Clocks {
    let has = |f: &dyn Fn(&Passive) -> bool| p.find(f).is_some();
    let smolder = has(&|p| matches!(p, Passive::Smolder { .. }));
    Clocks {
        hurt: smolder || has(&|p| matches!(p, Passive::Warmth { .. })),
        dealt: smolder,
        combat: has(&|p| matches!(p, Passive::LightFeet { .. } | Passive::Hatchling(items::Hatchling::Barkling))),
    }
}

/// Item effects of this tick's damage (server only), once per damage event in `events[from..]`:
/// combat timers, omnivamp, Wounds, Searing, Sunder, Expose, Tithe and Ambush. Damage these
/// cause (Ambush) doesn't trigger them again.
pub fn after_damage(units: &mut [Unit], events: &mut Vec<SimEvent>, from: usize) {
    let hits: Vec<(UnitId, UnitId, DamageOrigin, DamageKind, f32, SimTime)> = events[from..]
        .iter()
        .filter_map(|e| match *e {
            SimEvent::Damage { source, target, origin, kind, amount, absorbed, at, .. } => {
                Some((source, target, origin, kind, amount + absorbed, at))
            }
            _ => None,
        })
        .collect();
    for (source, target, origin, kind, dealt, at) in hits {
        if dealt <= 0.0 {
            continue;
        }
        let Some(t) = units.iter().find(|u| u.id == target) else { continue };
        let (target_kind, target_champion, target_max) = (t.kind, t.kind == UnitKind::Champion, t.stats.max_health);
        let target_clocks = clocks(&of(t));
        let Some(s) = units.iter().find(|u| u.id == source) else { continue };
        let source_champion = s.kind == UnitKind::Champion;
        if let Some(t) = units.iter_mut().find(|u| u.id == target) {
            target_clocks.note(&mut t.state.fx, at, false, source_champion);
        }
        if !source_champion {
            continue;
        }
        let passives = of(units.iter().find(|u| u.id == source).expect("source"));
        let item_effect = matches!(origin, DamageOrigin::Item(_));
        let Some(o) = units.iter_mut().find(|u| u.id == source) else { continue };
        clocks(&passives).note(&mut o.state.fx, at, true, target_champion);
        // Omnivamp: a share of all damage dealt.
        if o.stats.omnivamp > 0.0 && target_kind != UnitKind::Ward {
            heal(o, o.stats.omnivamp * dealt, at, events);
        }
        // Tithe (the support quest): attacks and abilities on champions and structures.
        if !item_effect && (target_champion || target_kind.is_structure()) {
            tithe(o, at, events);
        }
        if !target_champion {
            continue;
        }
        let ambush_ready = o.state.progress.gear.passive_ready[AMBUSH] <= at;
        let (combo, combo_at) = (o.state.fx.combo, o.state.fx.combo_at);
        let mut ambush: Option<(f32, f32, f32, u8)> = None;
        if !item_effect
            && let Some((Passive::Ambush { max_hp, shield: s, bonus_ad: r, cooldown_ms }, item)) =
                passives.find(|p| matches!(p, Passive::Ambush { .. }))
            && ambush_ready
        {
            if combo == target && combo_at < at && at.0 - combo_at.0 <= AMBUSH_WINDOW.0 {
                o.state.progress.gear.passive_ready[AMBUSH] = at.plus(ms(cooldown_ms));
                o.state.fx.combo = UnitId(0);
                ambush = Some((max_hp * target_max, s + r * bonus_ad(o), 0.0, item));
            } else if combo != target || at.0 - combo_at.0 > AMBUSH_WINDOW.0 {
                (o.state.fx.combo, o.state.fx.combo_at) = (target, at);
            }
        }
        if let Some((_, amount, _, _)) = ambush {
            shield(o, amount, at.plus(AMBUSH_SHIELD), at, events);
        }
        let by = Hitter::of(units.iter().find(|u| u.id == source).expect("source"), at);
        let Some(t) = units.iter_mut().find(|u| u.id == target) else { continue };
        for (p, item) in passives.iter() {
            match p {
                Passive::Wounds { pct, physical, magic }
                    if (physical && kind == DamageKind::Physical) || (magic && kind == DamageKind::Magic) =>
                {
                    t.state.fx.wound(pct, at)
                }
                Passive::Searing { max_hp, ms: d } if matches!(origin, DamageOrigin::Ability(_)) => {
                    let fx = &mut t.state.fx;
                    if fx.burn_until <= at {
                        fx.burn_next = at.plus(BURN_EVERY);
                    }
                    (fx.burn, fx.burn_by, fx.burn_item, fx.burn_until) =
                        (max_hp * target_max, source, item, at.plus(ms(d)));
                }
                Passive::Sunder { per_stack, stacks, ms: d } if kind == DamageKind::Physical && !item_effect => {
                    let fx = &mut t.state.fx;
                    let n = if fx.sunder_until > at { fx.sunder } else { 0 };
                    fx.sunder = (n + 1).min(stacks);
                    fx.sunder_pct = (per_stack * 100.0).round() as u8;
                    fx.sunder_until = at.plus(ms(d));
                }
                Passive::Expose { pct, ms: d } => {
                    t.state.fx.expose = (pct * 100.0).round() as u8;
                    t.state.fx.expose_until = at.plus(ms(d));
                }
                _ => {}
            }
        }
        if let Some((raw, _, _, item)) = ambush {
            let amount = mitigate(t, &by, raw, DamageKind::Physical, false, at);
            crate::world::damage(t, source, DamageOrigin::Item(item), amount, DamageKind::Physical, at, events);
        }
    }
}

/// Tithe (the support quest, 18 §6): a charge's gold to the holder of the quest's item, if it
/// has a charge left. Paid for hitting enemy champions and structures, and for healing and
/// shielding allied champions.
pub fn tithe(o: &mut Unit, at: SimTime, events: &mut Vec<SimEvent>) {
    use items::quest::*;
    if o.kind != UnitKind::Champion || of(o).find(|p| matches!(p, Passive::Quest)).is_none() {
        return;
    }
    let g = &mut o.state.progress.gear;
    if g.tithe_spent >= TITHE_CHARGES {
        return;
    }
    g.tithe_spent += 1;
    if g.tithe_at.0 == 0 {
        g.tithe_at = at.plus(TITHE_EVERY);
    }
    g.quest = g.quest.saturating_add(TITHE_GOLD as u16);
    o.state.progress.gold += TITHE_GOLD;
    events.push(SimEvent::Reward { unit: o.id, gold: TITHE_GOLD, xp: 0, at });
}

/// Item effects over time, every tick, for champions (and burns on anything): charges coming
/// back, the quest's stage, Light feet, Warmth, a grown Barkling's shield; and on the server
/// (`!prediction`) burn and Smolder pulses.
pub fn tick(units: &mut [Unit], prediction: bool, s0: SimTime, s1: SimTime, events: &mut Vec<SimEvent>) {
    let dt = (s1.0 - s0.0) as f32 / SUBTICKS_PER_SECOND as f32;
    for u in units.iter_mut().filter(|u| u.kind == UnitKind::Champion) {
        let passives = of(u);
        let alive = u.state.alive();
        let p = &mut u.state.progress;
        let g = &mut p.gear;
        // Tithe and Spoils charges come back one at a time.
        for (spent, at, every) in [
            (&mut g.tithe_spent, &mut g.tithe_at, items::quest::TITHE_EVERY),
            (&mut g.spoils_spent, &mut g.spoils_at, items::quest::SPOILS_EVERY),
        ] {
            if at.0 != 0 && *at <= s1 {
                *spent = spent.saturating_sub(1);
                *at = if *spent > 0 { at.plus(every) } else { SimTime(0) };
            }
        }
        // The quest's first stage: the Ledger becomes the Astrolabe, its wards full.
        if g.quest >= items::quest::STAGE
            && let Some(s) = p.items.iter().position(|i| *i == items::PILGRIMS_LEDGER)
        {
            p.items[s] = items::PILGRIMS_ASTROLABE;
            p.charges[s] = items::stackable(items::PILGRIMS_ASTROLABE).map_or(0, |(n, _)| n);
        }
        if !alive {
            continue;
        }
        let fx = &mut u.state.fx;
        // Light feet: speed out of champion combat.
        fx.light = match passives.find(|p| matches!(p, Passive::LightFeet { .. })) {
            Some((Passive::LightFeet { speed, after_ms }, _)) if fx.combat_at.plus(ms(after_ms)) <= s1 => speed as u8,
            _ => 0,
        };
        // Warmth: regenerate after a while without taking damage.
        if let Some((Passive::Warmth { max_hp, after_ms }, _)) = passives.find(|p| matches!(p, Passive::Warmth { .. }))
            && fx.hurt_at.plus(ms(after_ms)) <= s1
            && u.state.health < u.stats.max_health
        {
            let gain = max_hp * u.stats.max_health * dt * heal_factor(u, s1);
            u.state.health = (u.state.health + gain).min(u.stats.max_health);
        }
        // A grown Barkling keeps a shield up out of champion combat.
        if matches!(
            passives.find(|p| matches!(p, Passive::Hatchling(_))),
            Some((Passive::Hatchling(items::Hatchling::Barkling), _))
        ) && u.state.progress.gear.grown()
            && u.state.fx.combat_at.plus(ms(items::hatchling::BARK_AFTER_MS)) <= s1
            && u.state.shield_until <= s1.plus(ms(1000))
        {
            let amount = items::hatchling::BARK_SHIELD + items::hatchling::BARK_BONUS_HP * bonus_health(u);
            let st = &mut u.state;
            st.shield = st.shield.max(amount);
            st.shield_until = s1.plus(ms(items::hatchling::BARK_MS));
        }
    }
    if prediction {
        return;
    }
    // Burns (Searing): pulses until they end.
    let mut pulses: Vec<(UnitId, UnitId, u8, f32, SimTime)> = Vec::new();
    for u in units.iter_mut().filter(|u| u.state.alive()) {
        let fx = &mut u.state.fx;
        while fx.burn_until.0 != 0 && fx.burn_next <= s1 && fx.burn_next <= fx.burn_until {
            pulses.push((
                fx.burn_by,
                u.id,
                fx.burn_item,
                fx.burn * BURN_EVERY.0 as f32 / SUBTICKS_PER_SECOND as f32,
                fx.burn_next,
            ));
            fx.burn_next = fx.burn_next.plus(BURN_EVERY);
        }
        if fx.burn_until <= s1 {
            (fx.burn, fx.burn_until, fx.burn_next) = (0.0, SimTime(0), SimTime(0));
        }
    }
    // Smolder: holders in combat burn enemies around them every second.
    let smolderers: Vec<(UnitId, f32, f32, u8)> = units
        .iter()
        .filter(|u| u.kind == UnitKind::Champion && u.state.alive())
        .filter_map(|u| match of(u).find(|p| matches!(p, Passive::Smolder { .. })) {
            Some((Passive::Smolder { base, bonus_hp, radius }, item)) => {
                let fx = &u.state.fx;
                let fighting = fx.hurt_at.plus(IN_COMBAT) > s1 || fx.dealt_at.plus(IN_COMBAT) > s1;
                (fighting && (fx.hurt_at.0 != 0 || fx.dealt_at.0 != 0)).then_some((
                    u.id,
                    base + bonus_hp * bonus_health(u),
                    radius,
                    item,
                ))
            }
            _ => None,
        })
        .collect();
    for (id, dps, radius, item) in smolderers {
        let Some(u) = units.iter_mut().find(|u| u.id == id) else { continue };
        if u.state.fx.smolder_at > s1 {
            continue;
        }
        u.state.fx.smolder_at = s1.plus(SMOLDER_EVERY);
        let (team, pos) = (u.team, u.state.pos);
        let near: Vec<UnitId> = units
            .iter()
            .filter(|v| v.team != team && v.targetable() && !v.kind.is_structure() && v.kind != UnitKind::Ward)
            .filter(|v| (v.state.pos - pos).length() <= radius + v.gameplay_radius)
            .map(|v| v.id)
            .collect();
        for v in near {
            pulses.push((id, v, item, dps * SMOLDER_EVERY.0 as f32 / SUBTICKS_PER_SECOND as f32, s1));
        }
    }
    for (by, target, item, raw, at) in pulses {
        let hitter = Hitter::find(units, by, at);
        let Some(t) = units.iter_mut().find(|u| u.id == target) else { continue };
        let amount = mitigate(t, &hitter, raw, DamageKind::Magic, false, at);
        crate::world::damage(t, by, DamageOrigin::Item(item), amount, DamageKind::Magic, at, events);
    }
}
