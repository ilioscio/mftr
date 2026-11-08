//! A champion being made in the creator (CR3, 20 §2, §11.1): its name, its kit and its looks,
//! saved as a champion file and read back.
//!
//! Until the moveset step (CR6), a recipe's kit is borrowed whole from an official champion
//! (`kit_from`); the file carries the kit's numbers, so it plays exactly as written.

use mftr_sim::ChampionId;
use mftr_sim::champion::ChampionDef;
use mftr_sim::data;

use crate::looks::{self, Looks};

#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    pub name: String,
    pub kit: ChampionDef,
    pub looks: Looks,
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe { name: "New Champion".into(), kit: *ChampionId::Rook.def(), looks: Looks::default() }
    }
}

impl Recipe {
    /// Problems that stop it from being saved or played: the name, and the kit's limits
    /// (`mftr_sim::limits`). Empty when it's good.
    pub fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();
        let n = self.name.trim();
        if n.is_empty() || n.len() > data::MAX_NAME || n.chars().any(char::is_control) || n.contains('"') {
            out.push(format!("name: 1–{} characters, no quotes", data::MAX_NAME));
        }
        if ChampionId::ALL.iter().any(|c| c.def().name.eq_ignore_ascii_case(n)) {
            out.push(format!("name: `{n}` is an official champion's"));
        }
        out.extend(mftr_sim::limits::problems(&self.kit));
        if let Some(over) = mftr_sim::balance::over(&self.kit) {
            out.push(format!("{over:.0} points over the budget (20 §9): tune it down or Auto-fit"));
        }
        out
    }

    /// The champion file.
    pub fn to_text(&self) -> String {
        let comment =
            format!("{}: made in the champion creator (20). Durations are in milliseconds.", self.name.trim());
        data::write_champion_as(&self.kit, self.name.trim(), &comment, &looks::write(&self.looks))
    }

    /// Read a champion file back: its kit, and its looks (the default ones without).
    pub fn from_text(text: &str) -> Result<Recipe, String> {
        let (kit, raw) = data::read_champion_and_looks(text).map_err(|e| e.to_string())?;
        let looks = match raw {
            Some((v, at)) => looks::read(v, &at).map_err(|e| e.to_string())?,
            None => Looks::default(),
        };
        Ok(Recipe { name: kit.name.to_string(), kit, looks })
    }

    /// Take an official champion's kit (stats, attack and abilities) and its moves, keeping
    /// the name.
    pub fn borrow_kit(&mut self, from: ChampionId) {
        self.kit = *from.def();
        self.looks.moves = Some(crate::moves::Moves::of(from));
    }

    /// The moves now (a recipe without them animates as its kit's champion would, else Rook).
    pub fn moves(&self) -> crate::moves::Moves {
        self.looks.moves.clone().unwrap_or_else(|| crate::moves::Moves::of(self.kit_from().unwrap_or(ChampionId::Rook)))
    }

    /// Put move `id` on `slot` (CR6): its animation, effects and sounds, and its numbers in the
    /// kit (the attack's spec, or the ability), which the balance step then tunes (CR7). The
    /// class the bots play it as follows the whole kit (`balance::infer_class`). False if it doesn't fit.
    pub fn set_move(&mut self, slot: &str, id: &str) -> bool {
        let Some((c, from)) = crate::moves::parse(id) else { return false };
        if !crate::moves::fits(id, slot) {
            return false;
        }
        let mut moves = self.moves();
        moves.set(slot, id);
        self.looks.moves = Some(moves);
        let src = c.def();
        match slot {
            "attack" => {
                // The attack's champion is the stance: its body's stats come along (20 §9).
                self.kit.attack = src.attack;
                self.kit.class = src.class;
                self.kit.stats = src.stats;
                self.kit.growth = src.growth;
            }
            _ => {
                let i = ["q", "w", "e", "r"].iter().position(|s| *s == slot).unwrap_or(0);
                let j = ["q", "w", "e", "r"].iter().position(|s| *s == from).unwrap_or(0);
                self.kit.abilities[i] = src.abilities[j];
            }
        }
        // The class the bots play it as follows the whole kit, not the attack alone (08 CR7).
        self.kit.class = mftr_sim::balance::infer_class(&self.kit);
        true
    }

    /// The passive from the catalog (20 §8.1), by id; "" for none. False if there's no such one.
    pub fn set_passive(&mut self, id: &str) -> bool {
        if id.is_empty() {
            self.kit.passive = mftr_sim::items::Passive::None;
            return true;
        }
        match crate::passives::get(id) {
            Some(p) => {
                self.kit.passive = p;
                true
            }
            None => false,
        }
    }

    /// The resource by id (`passives::RESOURCES`), "" for costless. False if there's no such one.
    pub fn set_resource(&mut self, id: &str) -> bool {
        if id.is_empty() {
            self.kit.resource = mftr_sim::champion::Resource::NONE;
            return true;
        }
        match crate::passives::RESOURCES.iter().find(|e| e.0 == id) {
            Some(e) => {
                self.kit.resource = e.3;
                true
            }
            None => false,
        }
    }

    /// The catalog id of the kit's passive ("" for none).
    pub fn passive(&self) -> &'static str {
        crate::passives::of(self.kit.passive).map_or("", |e| e.0)
    }

    /// The official champion whose kit this is, if it's one of theirs unchanged.
    pub fn kit_from(&self) -> Option<ChampionId> {
        ChampionId::ALL.into_iter().find(|c| {
            let d = c.def();
            d.stats == self.kit.stats && d.attack == self.kit.attack && d.abilities == self.kit.abilities
        })
    }
}

/// The knobs the balance step turns (CR7, 20 §9.5): each relative to the move's own numbers.
impl Recipe {
    /// The official ability (or attack) a slot's move came from.
    fn source(&self, slot: &str) -> Option<(ChampionId, &'static str)> {
        crate::moves::parse(self.moves().get(slot))
    }

    fn source_ability(&self, slot: &str) -> Option<mftr_sim::ability::Ability> {
        let (c, from) = self.source(slot)?;
        let j = ["q", "w", "e", "r"].iter().position(|s| *s == from)?;
        Some(c.def().abilities[j])
    }

    /// A slot's power (its damage, healing and shields, × its move's) and cooldown (× its move's).
    pub fn tuning(&self, slot: &str) -> (f32, f32) {
        let i = ["q", "w", "e", "r"].iter().position(|s| *s == slot);
        let (Some(i), Some(src)) = (i, self.source_ability(slot)) else {
            let (c, _) = self.source("attack").unwrap_or((ChampionId::Rook, "attack"));
            let a = c.def();
            return (
                self.kit.stats.attack_damage / a.stats.attack_damage.max(1.0),
                a.attack.attack_speed / self.kit.attack.attack_speed.max(0.01),
            );
        };
        let now = &self.kit.abilities[i];
        (power_of(now) / power_of(&src).max(1e-3), now.cooldown.0 as f32 / src.cooldown.0.max(1) as f32)
    }

    /// Set a slot's power and cooldown (factors of its move's own numbers, clamped to 0.5–1.6 and
    /// 0.6–2). "attack" scales attack damage and the attack's period.
    pub fn tune(&mut self, slot: &str, power: f32, cooldown: f32) {
        let (power, cooldown) = (power.clamp(0.5, 1.6), cooldown.clamp(0.6, 2.0));
        if slot == "attack" {
            let (c, _) = self.source("attack").unwrap_or((ChampionId::Rook, "attack"));
            let a = c.def();
            self.kit.stats.attack_damage = a.stats.attack_damage * power;
            self.kit.attack.attack_speed = a.attack.attack_speed / cooldown;
            return;
        }
        let (Some(i), Some(src)) = (["q", "w", "e", "r"].iter().position(|s| *s == slot), self.source_ability(slot))
        else {
            return;
        };
        let mut a = scale_power(&src, power);
        a.cooldown = src.cooldown.scaled_ms(cooldown);
        a.per_rank.cooldown = src.per_rank.cooldown.scaled_ms(cooldown);
        self.kit.abilities[i] = a;
    }

    /// The body's stats against its stance champion's: health, armor, magic resist, move speed.
    pub fn stat_tuning(&self) -> [f32; 4] {
        let (c, _) = self.source("attack").unwrap_or((ChampionId::Rook, "attack"));
        let (a, b) = (&c.def().stats, &self.kit.stats);
        [
            b.max_health / a.max_health,
            b.armor / a.armor.max(1.0),
            b.magic_resist / a.magic_resist.max(1.0),
            b.move_speed / a.move_speed,
        ]
    }

    /// Set them (each 0.8–1.2 of the stance champion's; move speed 0.94–1.06).
    pub fn tune_stats(&mut self, k: [f32; 4]) {
        let (c, _) = self.source("attack").unwrap_or((ChampionId::Rook, "attack"));
        let (a, ga) = (c.def().stats, c.def().growth);
        let f = |v: f32| v.clamp(0.8, 1.2);
        self.kit.stats.max_health = a.max_health * f(k[0]);
        self.kit.growth.max_health = ga.max_health * f(k[0]);
        self.kit.stats.armor = a.armor * f(k[1]);
        self.kit.growth.armor = ga.armor * f(k[1]);
        self.kit.stats.magic_resist = a.magic_resist * f(k[2]);
        self.kit.growth.magic_resist = ga.magic_resist * f(k[2]);
        self.kit.stats.move_speed = (a.move_speed * k[3].clamp(0.94, 1.06)).clamp(300.0, 360.0);
    }

    /// Scale everything down until it fits the budget (20 §9.3).
    pub fn auto_fit(&mut self) {
        self.kit = mftr_sim::balance::auto_fit(&self.kit);
    }
}

/// An ability's damage, healing and shield amounts, summed (what `scale_power` scales).
fn power_of(a: &mftr_sim::ability::Ability) -> f32 {
    use mftr_sim::ability::Effect;
    let d = |d: &mftr_sim::ability::Damage| d.base + 100.0 * (d.ad_ratio + d.ap_ratio);
    match &a.effect {
        Effect::Line(s) => d(&s.damage),
        Effect::Area(s) => d(&s.damage),
        Effect::Beam(s) => d(&s.damage),
        Effect::Lunge(l) => d(&l.damage),
        Effect::Shield(s) => s.amount,
        Effect::Support(s) => s.heal + s.shield + 100.0 * (s.heal_ap + s.shield_ap + s.heal_missing),
        Effect::Dash(d) => d.range,
        Effect::Blink(b) => b.range,
        Effect::Claim(_) => 0.0,
    }
}

fn scale_power(src: &mftr_sim::ability::Ability, k: f32) -> mftr_sim::ability::Ability {
    use mftr_sim::ability::Effect;
    let mut a = *src;
    let d = |d: &mut mftr_sim::ability::Damage| {
        d.base *= k;
        d.ad_ratio *= k;
        d.ap_ratio *= k;
    };
    a.per_rank.damage *= k;
    match &mut a.effect {
        Effect::Line(s) => d(&mut s.damage),
        Effect::Area(s) => d(&mut s.damage),
        Effect::Beam(s) => d(&mut s.damage),
        Effect::Lunge(l) => d(&mut l.damage),
        Effect::Shield(s) => s.amount *= k,
        Effect::Support(s) => {
            s.heal *= k;
            s.shield *= k;
            s.heal_ap *= k;
            s.shield_ap *= k;
            s.heal_missing *= k;
        }
        // Movement: power is distance.
        Effect::Dash(x) => x.range = (x.range * k).clamp(100.0, 800.0),
        Effect::Blink(b) => b.range = (b.range * k).clamp(100.0, 600.0),
        Effect::Claim(_) => {}
    }
    a
}

/// A small deterministic generator for the creator's Randomize (xorshift).
pub struct Dice(u64);

impl Dice {
    pub fn new(seed: u64) -> Dice {
        Dice(seed.max(1))
    }

    /// Uniform in [-1, 1).
    pub fn signed(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 23) as f32 - 1.0
    }
}

/// Every slider (and proportion) set at random: mostly gentle, now and then extreme, so a
/// randomized champion is a starting point rather than a monster (20 §2).
pub fn randomize(looks: &mut Looks, sliders: &[String], seed: u64) {
    let mut d = Dice::new(seed);
    for id in sliders {
        let v = d.signed();
        looks.set_slider(id, v * v * v.signum() * 0.9);
    }
    for i in 0..looks::Proportions::NAMES.len() {
        let v = d.signed();
        looks.proportions.set(i, v * 0.6);
    }
    looks.skin = ((d.signed() + 1.0) * 4.0) as u8 % 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_recipe_saves_and_loads() {
        let mut r = Recipe { name: "Nettle".into(), ..Recipe::default() };
        r.borrow_kit(ChampionId::Lumen);
        r.looks.gender = looks::Gender::Feminine;
        r.looks.set_slider("nose_reach", 0.75);
        r.looks.proportions.set(4, 0.25);
        assert!(r.problems().is_empty(), "{:?}", r.problems());
        let text = r.to_text();
        let back = Recipe::from_text(&text).unwrap();
        assert_eq!(back.name, "Nettle");
        assert_eq!(back.looks, r.looks);
        assert_eq!(back.kit_from(), Some(ChampionId::Lumen));
        // It's a champion file: the pack validator takes it, and the forge builds it.
        assert_eq!(mftr_pack::champion::validate_text(&text).errors(), 0);
    }

    #[test]
    fn moves_carry_their_numbers() {
        let mut r = Recipe { name: "Patchwork".into(), ..Recipe::default() };
        assert!(r.set_move("attack", "vesper.attack"));
        assert!(r.set_move("q", "ember.q"));
        assert!(!r.set_move("q", "ember.r"));
        assert_eq!(r.kit.attack, ChampionId::Vesper.def().attack);
        // The class follows the whole kit (a marksman's attack under a mage's Q reads as what it is).
        assert_eq!(r.kit.class, mftr_sim::balance::infer_class(&r.kit));
        assert_eq!(r.kit.abilities[0], ChampionId::Ember.def().abilities[0]);
        // A patchwork of three kits costs more than one: fit it to the budget.
        r.auto_fit();
        assert!(r.problems().is_empty(), "{:?}", r.problems());
        let back = Recipe::from_text(&r.to_text()).unwrap();
        assert_eq!(back.looks.moves, r.looks.moves);
        assert_eq!(back.kit_from(), None);
    }

    #[test]
    fn tuning_moves_the_price() {
        let mut r = Recipe { name: "Tuned".into(), ..Recipe::default() };
        r.borrow_kit(ChampionId::Ember);
        let base = mftr_sim::balance::price(&r.kit).total();
        assert_eq!(r.tuning("q"), (1.0, 1.0));
        r.tune("q", 1.5, 0.7);
        let (p, c) = r.tuning("q");
        assert!((p - 1.5).abs() < 0.01 && (c - 0.7).abs() < 0.01, "{p} {c}");
        let up = mftr_sim::balance::price(&r.kit).total();
        assert!(up > base + 3.0, "{base} -> {up}");
        // Pushed well over the budget, then fitted back under it.
        for s in ["q", "w", "e", "r"] {
            r.tune(s, 1.6, 0.6);
        }
        r.tune_stats([1.2, 1.2, 1.2, 1.06]);
        assert!(r.problems().iter().any(|p| p.contains("over the budget")));
        r.auto_fit();
        assert!(mftr_sim::balance::over(&r.kit).is_none());
    }

    #[test]
    fn bad_names_are_problems() {
        for name in ["", "Rook", "a\"b", &"x".repeat(40)] {
            let r = Recipe { name: name.to_string(), ..Recipe::default() };
            assert!(!r.problems().is_empty(), "{name}");
        }
    }

    #[test]
    fn randomize_is_seeded_and_gentle() {
        let ids: Vec<String> = ["jaw_width", "nose_reach", "body_bulk"].iter().map(|s| s.to_string()).collect();
        let (mut a, mut b) = (Looks::default(), Looks::default());
        randomize(&mut a, &ids, 7);
        randomize(&mut b, &ids, 7);
        assert_eq!(a, b);
        assert!(a.shape.iter().all(|(_, v)| v.abs() <= 0.9));
        let mut c = Looks::default();
        randomize(&mut c, &ids, 8);
        assert_ne!(a, c);
    }
}
