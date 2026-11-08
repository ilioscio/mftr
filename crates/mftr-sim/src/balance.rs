//! The balance budget (CR7, 20 §9, D71): every champion spends the same budget of power points,
//! and every number that changes a fight has a price.
//!
//! A kit is priced by **what it's expected to do**, not by its raw numbers: each ability's
//! damage, crowd control, mobility and protection per second of cooldown, times the chance it
//! lands (a deterministic version of the bots' hit-probability model, D62: a skillshot fired at
//! 70% of its range at an average champion who reacts in 0.25 s), at a mid-game reference (level
//! 9, basic abilities rank 3, ultimates rank 2, typical item stats). Base stats are priced as
//! durability (effective health) and sustained damage (the basic attack), plus move speed.
//!
//! The weights are **anchored** on the official ten (D71): their prices average [`BUDGET`]. They
//! are a first fit; the random league (`mftr-tools league`) measures how well prices predict
//! bot-match win rates, and refits them.

use crate::ability::{Ability, Cc, Effect};
use crate::champion::{ChampionDef, ChampionId, max_rank};

/// Points every champion may spend *(start)*.
pub const BUDGET: f32 = 100.0;
/// How far over the budget a champion may go before it's refused (the official ten's spread).
pub const TOLERANCE: f32 = 0.10;

/// The mid-game reference the prices are taken at.
const LEVEL: u8 = 9;
const BONUS_AD: f32 = 35.0;
const BONUS_AP: f32 = 70.0;
const REACTION: f32 = 0.25;
const TARGET_SPEED: f32 = 335.0;
const TARGET_RADIUS: f32 = 65.0;

/// The price's parts, each in points.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Price {
    pub durability: f32,
    pub attack: f32,
    pub speed: f32,
    pub damage: f32,
    pub control: f32,
    pub mobility: f32,
    pub protection: f32,
}

impl Price {
    pub fn total(&self) -> f32 {
        self.durability + self.attack + self.speed + self.damage + self.control + self.mobility + self.protection
    }

    pub fn parts(&self) -> [(&'static str, f32); 7] {
        [
            ("durability", self.durability),
            ("attack", self.attack),
            ("speed", self.speed),
            ("damage", self.damage),
            ("control", self.control),
            ("mobility", self.mobility),
            ("protection", self.protection),
        ]
    }
}

/// The raw features before weighting (per second, or plain stats).
#[derive(Clone, Copy, Debug, Default)]
pub struct Features {
    pub ehp: f32,
    pub dps: f32,
    pub speed: f32,
    pub damage: f32,
    pub control: f32,
    pub mobility: f32,
    pub protection: f32,
}

/// Points per unit of each feature *(start)*: a least-squares fit pulling the official ten toward
/// [`BUDGET`], regularized toward hand-set weights (they land at 80–108; Quill, an artillery mage
/// whose beam is hard to price, lowest). Mobility then down and protection up by about a sixth:
/// the first random leagues (08 CR7) found both signs in each run.
const W_EHP: f32 = 0.0235;
const W_DPS: f32 = 0.19;
const W_SPEED: f32 = 0.22;
const W_DAMAGE: f32 = 0.22;
const W_CONTROL: f32 = 100.0;
const W_MOBILITY: f32 = 0.052;
const W_PROTECTION: f32 = 0.38;

/// The chance an ability lands (see the module notes).
pub fn hit_chance(a: &Ability) -> f32 {
    let skillshot = |t: f32, half_width: f32| {
        let free = (t - REACTION).max(0.0);
        let cover = TARGET_SPEED * free;
        let need = half_width + TARGET_RADIUS;
        if cover < need * 0.9 { 0.95 } else { (need / cover).clamp(0.08, 0.85) }
    };
    let secs = |d: crate::time::SimDuration| d.0 as f32 / crate::time::SUBTICKS_PER_SECOND as f32;
    match a.effect {
        Effect::Line(s) => skillshot(secs(s.windup) + 0.7 * s.range / s.speed.max(1.0), s.radius),
        Effect::Area(s) if s.range == 0.0 => {
            // A nova: the target must be close, and a delayed one can be walked out of.
            if s.delay.0 == 0 { 0.6 } else { skillshot(secs(s.windup) + secs(s.delay), s.radius) * 0.7 }
        }
        Effect::Area(s) => skillshot(secs(s.windup) + secs(s.delay), s.radius),
        Effect::Beam(s) => skillshot(secs(s.windup), s.radius),
        // Point and click.
        Effect::Lunge(_) | Effect::Support(_) | Effect::Shield(_) | Effect::Claim(_) => 1.0,
        Effect::Dash(_) | Effect::Blink(_) => 1.0,
    }
}

fn cc_value(c: Cc) -> f32 {
    let s = |d: crate::time::SimDuration| d.0 as f32 / crate::time::SUBTICKS_PER_SECOND as f32;
    match c {
        Cc::None => 0.0,
        Cc::Stun(d) | Cc::Knockup(d) => s(d) * 1.0,
        Cc::Root(d) => s(d) * 0.7,
        Cc::Pull(_) => 0.9,
        Cc::Slow { pct, duration } => s(duration) * pct as f32 / 100.0 * 0.5,
    }
}

/// The raw features of `def` at the reference.
pub fn features(def: &ChampionDef) -> Features {
    let st = def.stats_at(LEVEL);
    let ad = st.attack_damage + BONUS_AD;
    let ap = st.ability_power + BONUS_AP;
    let mut f = Features {
        ehp: st.max_health * (1.0 + (st.armor + st.magic_resist) / 200.0),
        dps: ad
            * def.attack.attack_speed
            * if def.attack.bolt_speed > 0.0 { 1.0 + def.attack.range / 1500.0 } else { 1.0 },
        speed: (st.move_speed - 300.0).max(0.0),
        ..Features::default()
    };
    for (slot, a) in def.abilities.iter().enumerate() {
        let rank = max_rank(slot as u8, LEVEL).max(1);
        let cd = a.cooldown_at(rank).0 as f32 / crate::time::SUBTICKS_PER_SECOND as f32;
        let cd = cd.max(0.5);
        let p = hit_chance(a);
        let bonus = a.bonus_damage_at(rank);
        let dmg = |d: crate::ability::Damage| d.base + bonus + d.ad_ratio * ad + d.ap_ratio * ap;
        match a.effect {
            Effect::Line(s) => {
                f.damage += dmg(s.damage) * p / cd;
                f.control += cc_value(s.cc) * p / cd;
            }
            Effect::Area(s) => {
                let pulses = 1.0 + s.pulses.share * s.pulses.count as f32;
                f.damage += dmg(s.damage) * pulses * p / cd;
                f.control += (cc_value(s.cc) + cc_value(s.pulses.cc) * s.pulses.count as f32 * 0.5) * p / cd;
            }
            Effect::Beam(s) => {
                f.damage += dmg(s.damage) * p / cd;
                f.control += cc_value(s.cc) * p / cd;
            }
            Effect::Lunge(l) => {
                f.damage += dmg(l.damage) / cd;
                f.control += cc_value(l.cc) / cd;
                f.mobility += l.range / cd;
            }
            Effect::Dash(d) => {
                let veil = if d.veil.some() { 1.6 } else { 1.0 };
                f.mobility += d.range * veil / cd;
            }
            Effect::Blink(b) => f.mobility += b.range * 1.3 / cd,
            Effect::Shield(s) => f.protection += s.amount / cd,
            Effect::Support(s) => {
                let ally = if s.range > 0.0 { 1.4 } else { 1.0 };
                let amount =
                    s.heal + bonus * 0.5 + s.heal_ap * ap + s.heal_missing * 300.0 + s.shield + s.shield_ap * ap;
                f.protection += amount * ally / cd;
            }
            Effect::Claim(_) => {}
        }
    }
    passive(def, &mut f, ad, ap);
    f
}

/// A champion's own passive (20 §8.1), priced like the rest: what it adds a second at the
/// reference, in the part it adds to. *(start)* shares: a cleave finds a second target half the
/// time, a speed burst is in a fight half the time, a low-health shield is worth three times its
/// average rate (it comes when it matters).
fn passive(def: &ChampionDef, f: &mut Features, ad: f32, ap: f32) {
    use crate::items::Passive;
    let st = def.stats_at(LEVEL);
    let rate = def.attack.attack_speed;
    let ranged = def.attack.bolt_speed > 0.0;
    let attack_dps = f.dps;
    // Abilities cast a second (Q W E), for what follows them.
    let casts: f32 = def.abilities[..3]
        .iter()
        .enumerate()
        .map(|(slot, a)| {
            let rank = max_rank(slot as u8, LEVEL).max(1);
            1.0 / (a.cooldown_at(rank).0 as f32 / crate::time::SUBTICKS_PER_SECOND as f32).max(0.5)
        })
        .sum();
    match def.passive {
        Passive::OnHitMagic { base, ap_ratio } => f.dps += (base + ap_ratio * ap) * rate,
        Passive::ThirdHit { base, bonus_ad } => f.dps += (base + bonus_ad * (ad - st.attack_damage)) * rate / 3.0,
        // Half the casts are followed by an attack in time.
        Passive::Spellblade { base_ad } => f.damage += base_ad * st.attack_damage * casts.min(rate) * 0.5,
        Passive::Cleave { melee, ranged: far, .. } => f.dps += attack_dps * if ranged { far } else { melee } * 0.5,
        Passive::Quicken { speed, ms } => f.speed += speed * (rate * ms as f32 / 1000.0).min(1.0) * 0.5,
        Passive::Sunder { per_stack, stacks, .. } => f.dps += attack_dps * per_stack * stacks as f32 * 0.4,
        Passive::Thorns { base, wounds, .. } => {
            f.damage += base * 0.5;
            f.protection += wounds * 10.0;
        }
        Passive::Rally { shield, cooldown_ms, .. } => {
            f.protection += shield * 3.0 / (cooldown_ms as f32 / 1000.0).max(1.0)
        }
        Passive::Warmth { max_hp, .. } => f.protection += max_hp * st.max_health * 0.3,
        _ => {}
    }
}

/// What `def` costs.
pub fn price(def: &ChampionDef) -> Price {
    let f = features(def);
    Price {
        durability: f.ehp * W_EHP,
        attack: f.dps * W_DPS,
        speed: f.speed * W_SPEED,
        damage: f.damage * W_DAMAGE,
        control: f.control * W_CONTROL,
        mobility: f.mobility * W_MOBILITY,
        protection: f.protection * W_PROTECTION,
    }
}

/// The most a champion may cost.
pub fn cap() -> f32 {
    BUDGET * (1.0 + TOLERANCE)
}

/// Over the budget: the points over, else `None`.
pub fn over(def: &ChampionDef) -> Option<f32> {
    let t = price(def).total();
    (t > cap()).then_some(t - BUDGET)
}

/// The official ten's prices (the anchors).
pub fn anchors() -> Vec<(ChampionId, f32)> {
    ChampionId::ALL.iter().map(|c| (*c, price(c.def()).total())).collect()
}

/// Scale the kit's damage, healing and shielding (bases, ratios, rank steps) down by one factor,
/// its cooldowns up by that factor's square root and its health and resistances down a little,
/// until it fits the budget (20 §9.3's Auto-fit): the gentlest step that does, to 1%.
pub fn auto_fit(def: &ChampionDef) -> ChampionDef {
    let mut out = *def;
    if over(def).is_none() {
        return out;
    }
    let mut k = 1.0;
    while k > 0.3 {
        k -= 0.01;
        out = scaled(def, k);
        if over(&out).is_none() {
            break;
        }
    }
    out
}

fn scaled(def: &ChampionDef, k: f32) -> ChampionDef {
    let mut out = *def;
    let dmg = |d: &mut crate::ability::Damage| {
        d.base *= k;
        d.ad_ratio *= k;
        d.ap_ratio *= k;
    };
    for a in &mut out.abilities {
        a.per_rank.damage *= k;
        match &mut a.effect {
            Effect::Line(s) => dmg(&mut s.damage),
            Effect::Area(s) => dmg(&mut s.damage),
            Effect::Beam(s) => dmg(&mut s.damage),
            Effect::Lunge(l) => dmg(&mut l.damage),
            Effect::Shield(s) => s.amount *= k,
            Effect::Support(s) => {
                s.heal *= k;
                s.heal_ap *= k;
                s.heal_missing *= k;
                s.shield *= k;
                s.shield_ap *= k;
            }
            _ => {}
        }
    }
    // Cooldowns lengthen and the body eases a little as power comes down.
    let slower = 1.0 / k.sqrt();
    for a in &mut out.abilities {
        a.cooldown = a.cooldown.scaled_ms(slower);
        a.per_rank.cooldown = a.per_rank.cooldown.scaled_ms(slower);
    }
    let ease = 0.7 + 0.3 * k; // no powf in the sim (03 §17: platform math differs)
    out.stats.attack_damage *= k.sqrt();
    out.stats.max_health *= ease;
    out.stats.armor *= ease;
    out.stats.magic_resist *= ease;
    out.growth.max_health *= ease;
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_passive_costs_a_few_points() {
        use crate::items::Passive;
        let base = *ChampionId::Rook.def();
        let plain = price(&base).total();
        for p in [
            Passive::OnHitMagic { base: 18.0, ap_ratio: 0.2 },
            Passive::ThirdHit { base: 45.0, bonus_ad: 0.4 },
            Passive::Spellblade { base_ad: 0.8 },
            Passive::Cleave { melee: 0.35, ranged: 0.15, radius: 300.0 },
            Passive::Quicken { speed: 25.0, ms: 2000 },
            Passive::Sunder { per_stack: 0.04, stacks: 5, ms: 5000 },
            Passive::Thorns { base: 12.0, bonus_armor: 0.15, wounds: 0.3 },
            Passive::Rally { shield: 120.0, bonus_hp: 0.3, threshold: 0.3, duration_ms: 3000, cooldown_ms: 60_000 },
            Passive::Warmth { max_hp: 0.02, after_ms: 6000 },
        ] {
            let with = price(&ChampionDef { passive: p, ..base }).total();
            assert!((1.0..8.0).contains(&(with - plain)), "{p:?}: {:.1} points", with - plain);
        }
    }

    #[test]
    fn the_official_ten_fit_the_budget() {
        let prices = anchors();
        let mean = prices.iter().map(|p| p.1).sum::<f32>() / prices.len() as f32;
        assert!((mean - BUDGET).abs() < 5.0, "{prices:?}");
        for (c, p) in &prices {
            assert!(*p <= cap(), "{} costs {p:.1}", c.def().name);
            assert!(over(c.def()).is_none());
        }
    }

    #[test]
    fn more_power_costs_more_and_auto_fit_brings_it_back() {
        let mut d = *ChampionId::Ember.def();
        let base = price(&d).total();
        if let Effect::Line(s) = &mut d.abilities[0].effect {
            s.damage.base *= 3.0;
            s.radius = 120.0;
        }
        d.abilities[3].cooldown = crate::time::SimDuration(d.abilities[3].cooldown.0 / 2);
        assert!(price(&d).total() > base + 10.0);
        assert!(over(&d).is_some(), "{}", price(&d).total());
        let fitted = auto_fit(&d);
        assert!(over(&fitted).is_none());
        // A wider, slower skillshot lands more often.
        let mut wide = *ChampionId::Ember.def();
        if let Effect::Line(s) = &mut wide.abilities[0].effect {
            s.radius = 100.0;
        }
        assert!(hit_chance(&wide.abilities[0]) > hit_chance(&ChampionId::Ember.def().abilities[0]));
    }

    #[test]
    #[ignore]
    fn print_the_anchors() {
        for c in ChampionId::ALL {
            let f = features(c.def());
            let p = price(c.def());
            println!(
                "{:8} ehp {:6.0} dps {:5.0} spd {:3.0} dmg {:5.1} cc {:.3} mob {:5.1} prot {:5.1} | dur {:5.1} att {:5.1} spd {:4.1} dmg {:5.1} cc {:5.1} mob {:5.1} prot {:5.1} = {:6.1}",
                c.def().name,
                f.ehp,
                f.dps,
                f.speed,
                f.damage,
                f.control,
                f.mobility,
                f.protection,
                p.durability,
                p.attack,
                p.speed,
                p.damage,
                p.control,
                p.mobility,
                p.protection,
                p.total()
            );
        }
    }
}
