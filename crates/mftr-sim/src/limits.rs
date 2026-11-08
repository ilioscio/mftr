//! What any champion's kit must respect, whoever made it (CR1b, 20 §9.4): the rules no balance
//! budget buys out, and sane bounds on every number. The official ten pass them (a test), and
//! `mftr-pack` refuses a champion file that doesn't.
//!
//! - **D10:** every dodge-intended ability gives a centered target time to react and walk out.
//! - **D36:** hard CC only on skillshots and telegraphed areas, never point-and-click; a hard-CC
//!   skillshot is classed `HardCc` (its reaction distance is 80% of its range).
//! - **D44:** delivery transformers only where they make sense (lines: all three; areas: echo
//!   and wide; nothing else).
//! - **Bounds** *(start)*: wide enough for any kit the genre knows, narrow enough that nothing
//!   breaks the sim or the screen. They are not balance: the balance budget (CR7) prices
//!   everything inside them.

use crate::ability::{Ability, Cc, Damage, Effect, ReactionClass, Transforms};
use crate::champion::{AttackSpec, ChampionDef, Stats};
use crate::time::{SUBTICKS_PER_SECOND, SimDuration};

fn ms(d: SimDuration) -> f32 {
    d.0 as f32 * 1000.0 / SUBTICKS_PER_SECOND as f32
}

struct Check<'a> {
    out: &'a mut Vec<String>,
    at: String,
}

impl Check<'_> {
    fn range(&mut self, what: &str, v: f32, lo: f32, hi: f32) {
        if !(lo..=hi).contains(&v) {
            self.out.push(format!("{}{what}: {v} is outside {lo}–{hi}", self.at));
        }
    }

    fn time(&mut self, what: &str, d: SimDuration, lo_ms: f32, hi_ms: f32) {
        self.range(&format!("{what} (ms)"), ms(d), lo_ms, hi_ms);
    }

    fn fail(&mut self, msg: impl std::fmt::Display) {
        self.out.push(format!("{}{msg}", self.at));
    }

    fn damage(&mut self, d: &Damage) {
        self.range("damage.base", d.base, 0.0, 400.0);
        self.range("damage.ad_ratio", d.ad_ratio, 0.0, 2.0);
        self.range("damage.ap_ratio", d.ap_ratio, 0.0, 2.0);
    }

    fn cc(&mut self, c: Cc) {
        match c {
            Cc::None => {}
            Cc::Stun(d) | Cc::Root(d) | Cc::Knockup(d) => self.time("cc", d, 250.0, 2500.0),
            Cc::Pull(to) => self.range("cc pull distance", to as f32, 100.0, 400.0),
            Cc::Slow { pct, duration } => {
                self.range("cc slow %", pct as f32, 5.0, 80.0);
                self.time("cc slow", duration, 250.0, 4000.0);
            }
        }
    }
}

/// A value's `(lowest, highest)`.
type Bounds = (f32, f32);

fn stats(c: &mut Check, s: &Stats, growth: bool) {
    // Level 1 values, then per-level growth.
    let bounds: [(&str, f32, Bounds, Bounds); 7] = [
        ("max_health", s.max_health, (400.0, 900.0), (0.0, 130.0)),
        ("health_regen", s.health_regen, (0.0, 4.0), (0.0, 0.3)),
        ("armor", s.armor, (0.0, 60.0), (0.0, 6.0)),
        ("magic_resist", s.magic_resist, (0.0, 60.0), (0.0, 3.0)),
        ("attack_damage", s.attack_damage, (0.0, 90.0), (0.0, 5.0)),
        ("ability_power", s.ability_power, (0.0, 120.0), (0.0, 0.0)),
        ("move_speed", s.move_speed, (300.0, 360.0), (0.0, 0.0)),
    ];
    for (name, v, base, grow) in bounds {
        let (lo, hi) = if growth { grow } else { base };
        c.range(name, v, lo, hi);
    }
}

fn attack(c: &mut Check, a: &AttackSpec) {
    c.range("attack_speed", a.attack_speed, 0.5, 1.0);
    c.range("windup_fraction", a.windup_fraction, 0.1, 0.4);
    if a.bolt_speed == 0.0 {
        c.range("melee range", a.range, 100.0, 200.0);
    } else {
        c.range("range", a.range, 300.0, 650.0);
        c.range("bolt_speed", a.bolt_speed, 1000.0, 3000.0);
    }
}

fn ability(c: &mut Check, a: &Ability, slot: usize) {
    let ult = slot == 3;
    c.time("cooldown", a.cooldown, 1000.0, if ult { 180_000.0 } else { 60_000.0 });
    c.range("per_rank.damage", a.per_rank.damage, 0.0, 150.0);
    let ranks = if ult { 2 } else { 4 };
    if a.per_rank.cooldown.0 * ranks >= a.cooldown.0 {
        c.fail("per_rank.cooldown: the top rank's cooldown must stay above zero");
    }
    // D44: transformers where they make sense.
    let allowed = match a.effect {
        Effect::Line(_) => Transforms::LINE,
        Effect::Area(_) => Transforms::AREA,
        _ => Transforms::NONE,
    };
    if a.transforms.0 & !allowed.0 != 0 {
        c.fail("transforms: this effect doesn't take them (D44)");
    }
    match a.effect {
        Effect::Line(s) => {
            c.time("windup", s.windup, 0.0, 1500.0);
            c.range("speed", s.speed, 400.0, 3000.0);
            c.range("radius", s.radius, 20.0, 150.0);
            c.range("range", s.range, 300.0, 2000.0);
            c.damage(&s.damage);
            c.cc(s.cc);
        }
        Effect::Area(s) => {
            c.time("windup", s.windup, 0.0, 1500.0);
            c.range("range", s.range, 0.0, 2000.0);
            c.range("radius", s.radius, 50.0, 500.0);
            c.time("delay", s.delay, 0.0, 3000.0);
            c.damage(&s.damage);
            c.cc(s.cc);
            if s.pulses.count > 0 {
                c.range("pulses.count", s.pulses.count as f32, 1.0, 10.0);
                c.time("pulses.every", s.pulses.every, 250.0, 2000.0);
                c.range("pulses.share", s.pulses.share, 0.0, 1.0);
                if s.pulses.cc.is_hard() {
                    c.fail("pulses.cc: lingering areas can't hard-CC (D36)");
                }
                c.cc(s.pulses.cc);
            }
        }
        Effect::Dash(d) => {
            c.range("range", d.range, 100.0, 800.0);
            c.range("speed", d.speed, 600.0, 3000.0);
            if d.veil.some() {
                c.range("veil.radius", d.veil.radius, 100.0, 400.0);
                c.time("veil", d.veil.duration, 500.0, 6000.0);
            }
        }
        Effect::Blink(b) => c.range("range", b.range, 100.0, 600.0),
        Effect::Shield(s) => {
            c.range("amount", s.amount, 0.0, 400.0);
            c.time("duration", s.duration, 500.0, 6000.0);
        }
        Effect::Support(s) => {
            c.range("range", s.range, 0.0, 1000.0);
            c.range("heal", s.heal, 0.0, 300.0);
            c.range("heal_ap", s.heal_ap, 0.0, 1.5);
            c.range("heal_missing", s.heal_missing, 0.0, 0.3);
            c.range("shield", s.shield, 0.0, 400.0);
            c.range("shield_ap", s.shield_ap, 0.0, 1.5);
            if s.shield > 0.0 || s.shield_ap > 0.0 {
                c.time("duration", s.duration, 500.0, 6000.0);
            }
            if s.heal + s.heal_ap + s.heal_missing + s.shield + s.shield_ap == 0.0 {
                c.fail("support: heals and shields nothing");
            }
        }
        Effect::Lunge(l) => {
            c.range("range", l.range, 100.0, 800.0);
            c.range("speed", l.speed, 600.0, 3000.0);
            c.damage(&l.damage);
            c.cc(l.cc);
        }
        Effect::Beam(b) => {
            c.time("windup", b.windup, 0.0, 1500.0);
            c.range("range", b.range, 300.0, 2000.0);
            c.range("radius", b.radius, 20.0, 150.0);
            c.time("duration", b.duration, 250.0, 3000.0);
            c.time("every", b.every, 100.0, 1000.0);
            c.damage(&b.damage);
            c.cc(b.cc);
        }
        Effect::Claim(_) => c.fail("Claim is a utility spell, not a champion ability"),
    }
    // D36 and D10: hard CC must be dodgeable, and dodge-intended shapes must be.
    let (cc, dodgeable) = match a.effect {
        Effect::Line(s) => (s.cc, true),
        Effect::Area(s) => (s.cc, s.range > 0.0 || s.delay.0 > 0),
        Effect::Beam(s) => (s.cc, true),
        Effect::Lunge(l) => (l.cc, false),
        _ => (Cc::None, false),
    };
    if cc.is_hard() && !dodgeable {
        c.fail("hard CC on an effect you can't dodge (D36)");
    }
    let skillshot =
        matches!(a.effect, Effect::Line(_) | Effect::Beam(_)) || matches!(a.effect, Effect::Area(s) if s.range > 0.0);
    if skillshot && cc.is_hard() != (a.reaction == ReactionClass::HardCc) {
        c.fail("reaction: hard-CC skillshots are classed HardCc, and only they are");
    }
    if let Some(frac) = a.reaction.range_fraction() {
        let (have, need) = match a.effect {
            Effect::Line(s) => (s.reaction_time(frac * s.range), s.reaction_needed()),
            Effect::Area(s) => (s.reaction_time(), s.reaction_needed()),
            Effect::Beam(s) => (s.reaction_time(), s.reaction_needed()),
            _ => {
                c.fail("reaction: only skillshots and areas are dodge-intended");
                return;
            }
        };
        if have < need {
            c.fail(format!("too fast to dodge (D10): {have:.3} s to react, {need:.3} s needed"));
        }
    }
}

/// Every way `def` breaks the limits, as readable lines; empty if it passes.
pub fn problems(def: &ChampionDef) -> Vec<String> {
    let mut out = Vec::new();
    stats(&mut Check { out: &mut out, at: "stats.".into() }, &def.stats, false);
    stats(&mut Check { out: &mut out, at: "growth.".into() }, &def.growth, true);
    attack(&mut Check { out: &mut out, at: "attack.".into() }, &def.attack);
    for (i, a) in def.abilities.iter().enumerate() {
        let slot = ["Q", "W", "E", "R"][i];
        ability(&mut Check { out: &mut out, at: format!("{slot} {}: ", a.name) }, a, i);
    }
    passive(&mut Check { out: &mut out, at: "passive: ".into() }, &def.passive);
    resource(&mut Check { out: &mut out, at: "resource: ".into() }, &def.resource);
    let mut names: Vec<&str> = def.abilities.iter().map(|a| a.name).collect();
    names.sort_unstable();
    names.dedup();
    if names.len() < 4 {
        out.push("abilities: each needs its own name".into());
    }
    out
}

/// A kit's resource (01 §9): pools and costs within reason, and every cost payable from a full
/// pool.
fn resource(c: &mut Check, r: &crate::champion::Resource) {
    use crate::champion::ResourceKind;
    match r.kind {
        ResourceKind::None => {}
        ResourceKind::Mana | ResourceKind::Energy | ResourceKind::Rage => {
            c.range("max", r.max, 50.0, 1000.0);
            let hi = match r.kind {
                ResourceKind::Mana => 30.0,
                ResourceKind::Energy => 80.0,
                _ => 40.0,
            };
            c.range("regen", r.regen, 0.5, hi);
            for (i, cost) in r.costs.iter().enumerate() {
                c.range(&format!("costs[{i}]"), *cost, 0.0, r.max);
            }
        }
        ResourceKind::Health => {
            for (i, cost) in r.costs.iter().enumerate() {
                c.range(&format!("costs[{i}]"), *cost, 0.0, 300.0);
            }
        }
    }
}

/// A champion's own passive (20 §8.1): about an item's strength at most, so no passive
/// outweighs a kit.
fn passive(c: &mut Check, p: &crate::items::Passive) {
    use crate::items::Passive;
    let ms = |v: u64| v as f32;
    match *p {
        Passive::None => {}
        Passive::OnHitMagic { base, ap_ratio } => {
            c.range("base", base, 0.0, 60.0);
            c.range("ap_ratio", ap_ratio, 0.0, 0.6);
        }
        Passive::ThirdHit { base, bonus_ad } => {
            c.range("base", base, 0.0, 150.0);
            c.range("bonus_ad", bonus_ad, 0.0, 1.0);
        }
        Passive::Spellblade { base_ad } => c.range("base_ad", base_ad, 0.0, 2.0),
        Passive::Cleave { melee, ranged, radius } => {
            c.range("melee", melee, 0.0, 0.6);
            c.range("ranged", ranged, 0.0, 0.3);
            c.range("radius", radius, 100.0, 400.0);
        }
        Passive::Quicken { speed, ms: d } => {
            c.range("speed", speed, 0.0, 60.0);
            c.range("ms", ms(d), 500.0, 4000.0);
        }
        Passive::Sunder { per_stack, stacks, ms: d } => {
            c.range("per_stack × stacks", per_stack * stacks as f32, 0.0, 0.3);
            c.range("stacks", stacks as f32, 1.0, 10.0);
            c.range("ms", ms(d), 1000.0, 8000.0);
        }
        Passive::Thorns { base, bonus_armor, wounds } => {
            c.range("base", base, 0.0, 40.0);
            c.range("bonus_armor", bonus_armor, 0.0, 0.5);
            c.range("wounds", wounds, 0.0, 0.5);
        }
        Passive::Rally { shield, bonus_hp, threshold, duration_ms, cooldown_ms } => {
            c.range("shield", shield, 0.0, 400.0);
            c.range("bonus_hp", bonus_hp, 0.0, 1.0);
            c.range("threshold", threshold, 0.1, 0.5);
            c.range("duration_ms", ms(duration_ms), 1000.0, 6000.0);
            c.range("cooldown_ms", ms(cooldown_ms), 20_000.0, 180_000.0);
        }
        Passive::Warmth { max_hp, after_ms } => {
            c.range("max_hp", max_hp, 0.0, 0.05);
            c.range("after_ms", ms(after_ms), 3000.0, 15_000.0);
        }
        other => c.fail(format!("{other:?} isn't a champion passive")),
    }
}

/// Read a champion file and check it, its price included (D71): the definition, or every
/// problem on one line.
pub fn read_champion_file(text: &str) -> Result<ChampionDef, String> {
    let def = crate::data::read_champion(text).map_err(|e| e.to_string())?;
    let mut problems = problems(&def);
    // The balance budget (D71): a champion that costs more than it may isn't played.
    if let Some(over) = crate::balance::over(&def) {
        problems.push(format!(
            "costs {:.0} points, over the {:.0}-point budget by {over:.0} (20 §9)",
            crate::balance::price(&def).total(),
            crate::balance::BUDGET
        ));
    }
    if problems.is_empty() { Ok(def) } else { Err(format!("`{}`: {}", def.name, problems.join("; "))) }
}

/// [`read_champion_file`], then register it: its id (the same one if it's registered already).
pub fn load_champion_file(text: &str) -> Result<crate::champion::ChampionId, String> {
    let def = read_champion_file(text)?;
    crate::champion::ChampionId::register(def).map_err(|_| "too many champions registered".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::champion::ChampionId;
    use crate::data::{read_champion, write_champion};

    #[test]
    fn the_official_ten_pass() {
        for c in ChampionId::ALL {
            assert_eq!(problems(c.def()), Vec::<String>::new(), "{}", c.def().name);
        }
    }

    fn with(edit: &str, to: &str) -> Vec<String> {
        let text = write_champion(ChampionId::Ember.def(), "").replace(edit, to);
        assert!(text.contains(to), "{edit} not found");
        problems(&read_champion(&text).unwrap())
    }

    fn has(problems: &[String], what: &str) {
        assert!(problems.iter().any(|p| p.contains(what)), "{what} not in {problems:?}");
    }

    #[test]
    fn breaking_a_rule_is_reported() {
        // Bounds.
        has(&with("move_speed: 325", "move_speed: 500"), "stats.move_speed: 500 is outside");
        has(&with("radius: 35, range: 1100", "radius: 35, range: 9000"), "Q Ember Lance: range");
        // D10: a stun that lands before anyone can react.
        has(&with("windup_ms: 300, speed: 1300", "windup_ms: 0, speed: 3000"), "too fast to dodge (D10)");
        // D36: a hard-CC skillshot not classed as one, and hard CC on a point-and-click lunge.
        has(&with("reaction: HardCc", "reaction: Burst"), "classed HardCc");
        let lunge = "effect: Lunge(range: 500, speed: 1500, damage: (kind: Magic, base: 50), cc: Stun(ms: 1000))";
        has(&with("effect: Blink(range: 350)", lunge), "can't dodge (D36)");
        // D44: a blink that multishots.
        has(
            &with(
                "effect: Blink(range: 350),\n            reaction: None,",
                "effect: Blink(range: 350),\n            reaction: None,\n            transforms: [Multishot],",
            ),
            "D44",
        );
        has(&with("\"Flicker\"", "\"Ember Lance\""), "its own name");
    }
}
