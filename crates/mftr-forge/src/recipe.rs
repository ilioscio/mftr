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
    /// attack's move also sets the class the bots play it as. False if it doesn't fit.
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
                self.kit.attack = src.attack;
                self.kit.class = src.class;
            }
            _ => {
                let i = ["q", "w", "e", "r"].iter().position(|s| *s == slot).unwrap_or(0);
                let j = ["q", "w", "e", "r"].iter().position(|s| *s == from).unwrap_or(0);
                self.kit.abilities[i] = src.abilities[j];
            }
        }
        true
    }

    /// The official champion whose kit this is, if it's one of theirs unchanged.
    pub fn kit_from(&self) -> Option<ChampionId> {
        ChampionId::ALL.into_iter().find(|c| {
            let d = c.def();
            d.stats == self.kit.stats && d.attack == self.kit.attack && d.abilities == self.kit.abilities
        })
    }
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
        assert_eq!(r.kit.class, ChampionId::Vesper.def().class);
        assert_eq!(r.kit.abilities[0], ChampionId::Ember.def().abilities[0]);
        assert!(r.problems().is_empty(), "{:?}", r.problems());
        let back = Recipe::from_text(&r.to_text()).unwrap();
        assert_eq!(back.looks.moves, r.looks.moves);
        assert_eq!(back.kit_from(), None);
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
