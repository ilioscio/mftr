//! Item tooltips (18): what an item's stats, passives and active say, generated from the sim's
//! data so the text can't drift from the numbers.

use mftr_sim::items::{self, Active, Hatchling, Item, Passive};

/// One tooltip line and what it is (a stat's key for its color, "passive" or "active").
pub struct Line {
    pub text: String,
    pub kind: &'static str,
}

fn pct(v: f32) -> String {
    format!("{}%", (v * 100.0).round())
}

fn secs(ms: u64) -> String {
    let s = ms as f32 / 1000.0;
    if s.fract() == 0.0 { format!("{s:.0} s") } else { format!("{s:.1} s") }
}

/// The stat lines: flat bonuses first, then percents.
pub fn stats(it: &Item) -> Vec<Line> {
    let b = &it.bonus;
    let mut out = Vec::new();
    let flat = [
        (b.health, "health", "hp"),
        (b.health_regen, "health per second", "hp"),
        (b.armor, "armor", "armor"),
        (b.magic_resist, "magic resist", "mr"),
        (b.attack_damage, "attack damage", "ad"),
        (b.ability_power, "ability power", "ap"),
        (b.ability_haste, "ability haste", "haste"),
        (b.lethality, "lethality", "ad"),
        (b.magic_pen, "magic penetration", "ap"),
        (b.move_speed, "move speed", "ms"),
    ];
    for (v, name, kind) in flat {
        if v != 0.0 {
            out.push(Line { text: format!("+{v} {name}"), kind });
        }
    }
    let percents = [
        (b.attack_speed, "attack speed", "as"),
        (b.crit_chance, "critical strike chance", "as"),
        (b.crit_damage, "critical strike damage", "ad"),
        (b.armor_pen, "armor penetration", "ad"),
        (b.magic_pen_pct, "magic penetration", "ap"),
        (b.life_steal, "life steal", "lifesteal"),
        (b.omnivamp, "omnivamp", "lifesteal"),
        (b.tenacity, "tenacity", "mr"),
        (b.slow_resist, "slow resistance", "ms"),
        (b.heal_power, "heal and shield power", "hp"),
        (b.heal_received, "healing and shields received", "hp"),
        (b.move_speed_pct, "move speed", "ms"),
        (b.ability_power_pct, "ability power", "ap"),
    ];
    for (v, name, kind) in percents {
        if v != 0.0 {
            out.push(Line { text: format!("+{} {name}", pct(v)), kind });
        }
    }
    out
}

/// A passive's name and what it does.
pub fn passive(p: &Passive) -> Option<(&'static str, String)> {
    Some(match *p {
        Passive::None => return None,
        Passive::OnHitMagic { base, ap_ratio } if ap_ratio > 0.0 => {
            ("Arc", format!("Basic attacks deal {base} (+{} AP) magic damage.", pct(ap_ratio)))
        }
        Passive::OnHitMagic { base, .. } => ("Arc", format!("Basic attacks deal {base} magic damage.")),
        Passive::ThirdHit { base, bonus_ad } => {
            ("Gale", format!("Every third basic attack deals {base} (+{} bonus AD) physical damage.", pct(bonus_ad)))
        }
        Passive::Spellblade { base_ad } => (
            "Spellblade",
            format!(
                "After using an ability, your next basic attack within 10 s deals {} of base AD as bonus physical damage (1.5 s cooldown).",
                pct(base_ad)
            ),
        ),
        Passive::Cleave { melee, ranged, radius } => (
            "Cleave",
            format!(
                "Basic attacks deal {} of AD to other enemies within {radius} of the target ({} for ranged champions).",
                pct(melee),
                pct(ranged)
            ),
        ),
        Passive::Fanfire { targets, share, radius } => (
            "Fanfire",
            format!(
                "Ranged basic attacks also fire bolts at up to {targets} enemies within {radius} of the target, for {} damage. They apply on-hit effects.",
                pct(share)
            ),
        ),
        Passive::Quicken { speed, ms } => {
            ("Quicken", format!("Basic attacks grant {speed} move speed for {}.", secs(ms)))
        }
        Passive::Sunder { per_stack, stacks, ms } => (
            "Sunder",
            format!(
                "Physical damage to a champion reduces its armor by {} for {}, stacking {stacks} times (up to {}).",
                pct(per_stack),
                secs(ms),
                pct(per_stack * stacks as f32)
            ),
        ),
        Passive::Expose { pct: p, ms } => (
            "Expose",
            format!("Your hits on champions reduce their armor and magic resist by {} for {}.", pct(p), secs(ms)),
        ),
        Passive::Wounds { pct: p, physical, magic } => {
            let kind = match (physical, magic) {
                (true, false) => "Physical damage",
                (false, true) => "Magic damage",
                _ => "Damage",
            };
            ("Wounds", format!("{kind} to champions applies {} Wounds (less healing) for 3 s.", pct(p)))
        }
        Passive::Thorns { base, bonus_armor, wounds } => (
            "Thorns",
            format!(
                "When struck by a basic attack, the attacker takes {base} (+{} bonus armor) magic damage and {} Wounds for 3 s.",
                pct(bonus_armor),
                pct(wounds)
            ),
        ),
        Passive::Searing { max_hp, ms } => (
            "Searing",
            format!(
                "Ability damage to champions burns them for {} of their max health as magic damage each second for {}.",
                pct(max_hp),
                secs(ms)
            ),
        ),
        Passive::Smolder { base, bonus_hp, radius } => (
            "Smolder",
            format!(
                "While in combat, deal {base} (+{} bonus health) magic damage each second to enemies within {radius}.",
                pct(bonus_hp)
            ),
        ),
        Passive::Ambush { max_hp, shield, bonus_ad, cooldown_ms } => (
            "Ambush",
            format!(
                "Two separate hits on a champion within 2 s deal {} of its max health as physical damage and shield you for {shield} (+{} bonus AD) for 2 s ({} cooldown).",
                pct(max_hp),
                pct(bonus_ad),
                secs(cooldown_ms)
            ),
        ),
        Passive::Rally { shield, bonus_hp, threshold, duration_ms, cooldown_ms } => (
            "Rally",
            format!(
                "Falling below {} health grants a shield of {shield} (+{} bonus health) for {} ({} cooldown).",
                pct(threshold),
                pct(bonus_hp),
                secs(duration_ms),
                secs(cooldown_ms)
            ),
        ),
        Passive::Overflow { base, per_level, ms } => (
            "Overflow",
            format!(
                "Life steal past full health becomes a shield of up to {base} (+{per_level} per level) for {}.",
                secs(ms)
            ),
        ),
        Passive::Warmth { max_hp, after_ms } => (
            "Warmth",
            format!(
                "After {} without taking damage, regenerate {} of max health each second.",
                secs(after_ms),
                pct(max_hp)
            ),
        ),
        Passive::Topple { amp, full_at } => (
            "Topple",
            format!(
                "Deal up to {} more damage to champions with more max health than you (full at {full_at} more).",
                pct(amp)
            ),
        ),
        Passive::LightFeet { speed, after_ms } => {
            ("Light feet", format!("+{speed} move speed after {} out of champion combat.", secs(after_ms)))
        }
        Passive::CritCut(c) => ("Unshaken", format!("Critical strikes deal {} less damage to you.", pct(c))),
        Passive::AttackCut(c) => ("Plated", format!("Basic attacks deal {} less damage to you.", pct(c))),
        Passive::Kindle { magic, ms } => (
            "Kindle",
            format!(
                "Your heals and shields on allies make their basic attacks deal {magic} bonus magic damage for {}.",
                secs(ms)
            ),
        ),
        Passive::Bulwark { shield, bonus_hp, radius, cooldown_ms } => (
            "Bulwark",
            format!(
                "Immobilizing an enemy champion shields allied champions within {radius} for {shield} (+{} of your bonus health) ({} cooldown).",
                pct(bonus_hp),
                secs(cooldown_ms)
            ),
        ),
        Passive::Tally { ap, most, kill, assist, death } => (
            "Tally",
            format!(
                "+{ap} ability power per stack: {kill} stacks a champion kill, {assist} an assist, up to {most}. Dying loses {death}."
            ),
        ),
        Passive::Quest => {
            use items::quest::*;
            (
                "Pilgrim's quest",
                format!(
                    "Tithe: hitting enemy champions and structures, and healing or shielding allied champions, earns {TITHE_GOLD} gold ({TITHE_CHARGES} charges). Spoils: attacking an enemy minion low on health with an ally near kills it, and the ally earns its gold too ({SPOILS_CHARGES} charges). Your own minion kills pay half. At {STAGE} quest gold this becomes the Pilgrim's Astrolabe; at {DONE}, upgrade it to a lantern for free."
                ),
            )
        }
        Passive::Hatchling(h) => {
            use items::hatchling::*;
            let grown = match h {
                Hatchling::Windling => {
                    format!("Claiming grants {WIND_SPEED}% move speed, fading over {}.", secs(WIND_MS))
                }
                Hatchling::Barkling => format!(
                    "Claiming, or {} out of champion combat, shields you for {BARK_SHIELD} (+{} bonus health).",
                    secs(BARK_AFTER_MS),
                    pct(BARK_BONUS_HP)
                ),
                Hatchling::Flintling => format!(
                    "Claim can strike enemy champions: {FLINT_BASE} (+{} of their max health) true damage and a {FLINT_SLOW}% slow.",
                    pct(FLINT_MAX_HP)
                ),
            };
            (
                "Hatchling",
                format!(
                    "{} more damage to monsters and {} less from them. Grows up after {GROWN} monsters (big ones count twice): {grown} Needs Claim.",
                    pct(MONSTER_AMP - 1.0),
                    pct(1.0 - MONSTER_CUT)
                ),
            )
        }
    })
}

/// An active's name and what it does.
pub fn active(a: &Active) -> Option<(&'static str, String)> {
    Some(match *a {
        Active::None => return None,
        Active::Consumable { heal, duration_ms, charges, refills: true } => {
            ("Drink", format!("Heals {heal} over {}. {charges} charges, refilled at the fountain.", secs(duration_ms)))
        }
        Active::Consumable { heal, duration_ms, charges, .. } => {
            ("Drink", format!("Heals {heal} over {}. Up to {charges} in a slot.", secs(duration_ms)))
        }
        Active::ControlWard { charges } => (
            "Control ward",
            format!(
                "Place a control ward at the cursor. It watches until destroyed, and reveals and disables enemy stealth wards it sees. One out at a time; up to {charges} in a slot."
            ),
        ),
        Active::StealthWard { charges } => {
            ("Ward", format!("Place a stealth ward at the cursor. {charges} charges, refilled at the fountain."))
        }
        Active::ReapingArc { ad, radius, cooldown_ms } => (
            "Reaping Arc",
            format!(
                "Deal {} of AD as physical damage to enemies within {radius} and reset your attack timer ({} cooldown).",
                pct(ad),
                secs(cooldown_ms)
            ),
        ),
        Active::Bloodletting { ad, radius, heal_missing, cooldown_ms } => (
            "Bloodletting",
            format!(
                "Deal {} of AD as physical damage to enemies within {radius}; heal {} of your missing health per champion hit ({} cooldown).",
                pct(ad),
                pct(heal_missing),
                secs(cooldown_ms)
            ),
        ),
        Active::Fleet { pct: p, ms, cooldown_ms } => {
            ("Fleet", format!("+{p}% move speed for {} ({} cooldown).", secs(ms), secs(cooldown_ms)))
        }
        Active::Plunge { range, base, bonus_ad, amp, amp_ms, cooldown_ms } => (
            "Plunge",
            format!(
                "Dash to the enemy champion nearest the cursor (within {range}) and deal {base} (+{} bonus AD) physical damage; you deal {} more damage to it for {} ({} cooldown).",
                pct(bonus_ad),
                pct(amp),
                secs(amp_ms),
                secs(cooldown_ms)
            ),
        ),
        Active::Encase { ms, cooldown_ms } => (
            "Encase",
            format!(
                "Become invulnerable and untargetable for {}, unable to move, attack or cast ({} cooldown).",
                secs(ms),
                secs(cooldown_ms)
            ),
        ),
        Active::Surge { dash, bolts, base, ap_ratio, cooldown_ms } => (
            "Surge",
            format!(
                "Dash {dash} toward the cursor and fire {bolts} bolts in a fan, each dealing {base} (+{} AP) magic damage; an enemy is hit once ({} cooldown).",
                pct(ap_ratio),
                secs(cooldown_ms)
            ),
        ),
        Active::Quake { radius, slow, ms, cooldown_ms } => (
            "Quake",
            format!("Slow enemies within {radius} by {slow}% for {} ({} cooldown).", secs(ms), secs(cooldown_ms)),
        ),
        Active::Petrify { base, bonus_hp, ms, cut, cooldown_ms } => (
            "Petrify",
            format!(
                "Gain a shield of {base} (+{} bonus health) for {}, but deal {} less damage meanwhile ({} cooldown).",
                pct(bonus_hp),
                secs(ms),
                pct(cut),
                secs(cooldown_ms)
            ),
        ),
        Active::Embrace { base, per_level, radius, ms, cooldown_ms } => (
            "Embrace",
            format!(
                "Shield yourself and allied champions within {radius} for {base} (+{per_level} per level) for {} ({} cooldown).",
                secs(ms),
                secs(cooldown_ms)
            ),
        ),
        Active::Absolve { range, heal, heal_missing, speed, ms, cooldown_ms } => (
            "Absolve",
            format!(
                "On the allied champion nearest the cursor (within {range}), else yourself: remove stuns, roots and slows (not knock-ups or pulls), heal {heal} (+{} missing health) and grant {speed}% move speed fading over {} ({} cooldown).",
                pct(heal_missing),
                secs(ms),
                secs(cooldown_ms)
            ),
        ),
    })
}

/// Every tooltip line: stats, passives, the active.
pub fn lines(it: &Item) -> Vec<Line> {
    let mut out = stats(it);
    for p in &it.passives {
        if let Some((name, text)) = passive(p) {
            out.push(Line { text: format!("{name}: {text}"), kind: "passive" });
        }
    }
    if let Some((name, text)) = active(&it.active) {
        out.push(Line { text: format!("Active ({name}): {text}"), kind: "active" });
    }
    out
}
