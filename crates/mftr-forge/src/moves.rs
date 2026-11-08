//! The moveset (CR6, 20 §8): the humanoid move library. A **move** is an official champion's
//! basic attack or ability (`rook.attack`, `ember.q`): its animation clips, its effects and
//! sounds, the shape and numbers of its ability, and the weapon class it is animated with. A
//! recipe picks one move for its attack and one for each of Q W E R (`looks.moves`); the forge
//! takes the clips, effects and sounds from the move's champion pack, renamed for the slot it
//! fills, and paints the effects in the recipe's signature hue.
//!
//! The attack's move also sets the **stance**: idle, run, walk, death, recall, emotes and fidgets
//! come from its champion, so a bow is held like a bow between shots.

use mftr_pack::glb::{Animation, Model};
use mftr_pack::{Loaded, sfx, vfx};
use mftr_sim::ChampionId;

/// The slots a recipe fills: the attack, then Q W E R.
pub const SLOTS: [&str; 5] = ["attack", "q", "w", "e", "r"];

/// A recipe's moves, one per slot (`champion.slot` ids).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Moves {
    pub ids: [String; 5],
}

impl Moves {
    /// One champion's own moves.
    pub fn of(champion: ChampionId) -> Moves {
        let n = champion.def().name.to_lowercase();
        Moves { ids: SLOTS.map(|s| format!("{n}.{s}")) }
    }

    pub fn get(&self, slot: &str) -> &str {
        SLOTS.iter().position(|s| *s == slot).map_or("", |i| self.ids[i].as_str())
    }

    pub fn set(&mut self, slot: &str, id: &str) {
        if let Some(i) = SLOTS.iter().position(|s| *s == slot) {
            self.ids[i] = id.to_string();
        }
    }
}

/// A move id's champion and slot, if it names one: `ember.q` → (Ember, "q").
pub fn parse(id: &str) -> Option<(ChampionId, &'static str)> {
    let (who, slot) = id.split_once('.')?;
    let c = ChampionId::ALL.into_iter().find(|c| c.def().name.eq_ignore_ascii_case(who))?;
    let slot = SLOTS.into_iter().find(|s| *s == slot)?;
    Some((c, slot))
}

/// Whether a move may go in `slot`: the attack's moves are attacks; abilities go in any of
/// Q W E R, but ultimates only in R and R only takes ultimates (their numbers are an ultimate's).
pub fn fits(id: &str, slot: &str) -> bool {
    let Some((_, from)) = parse(id) else { return false };
    match (from, slot) {
        ("attack", "attack") => true,
        ("attack", _) | (_, "attack") => false,
        ("r", s) | (s, "r") => s == "r" && from == "r",
        _ => true,
    }
}

/// Every move, in roster order: (id, the champion's name and the ability's).
pub fn library() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for c in ChampionId::ALL {
        let d = c.def();
        let n = d.name.to_lowercase();
        let kind = if d.attack.bolt_speed == 0.0 { "melee attack" } else { "ranged attack" };
        out.push((format!("{n}.attack"), format!("{}: {kind}", d.name)));
        for (i, s) in ["q", "w", "e", "r"].iter().enumerate() {
            out.push((format!("{n}.{s}"), format!("{}: {}", d.name, d.abilities[i].name)));
        }
    }
    out
}

/// The weapon class a champion's moves are animated with (20 §7.3), and the catalog part that
/// carries it ("" for none: fists, or a crossbow the catalog lacks yet).
pub fn weapon(champion: ChampionId) -> (&'static str, &'static str) {
    match champion {
        ChampionId::Vesper => ("bow", "offhand.bow"),
        ChampionId::Rook => ("heavy", "weapon.maul"),
        ChampionId::Shade => ("blades", "weapon.dagger"),
        ChampionId::Lumen | ChampionId::Quill | ChampionId::Cairn | ChampionId::Marrow => ("staff", "weapon.staff"),
        ChampionId::Wren => ("crossbow", ""),
        _ => ("unarmed", ""),
    }
}

/// The official champion packs, by champion (loaded with the catalog when present).
pub struct Packs(pub Vec<(ChampionId, Loaded)>);

impl Packs {
    pub fn get(&self, c: ChampionId) -> Option<&Loaded> {
        self.0.iter().find(|(k, _)| *k == c).map(|(_, p)| p)
    }
}

/// The clip a build with its own move library (CR10: a beast's) plays for an ability, by its
/// shape: projectiles, beams and buffs a howl, areas a slam, lunges a pounce; dashes the dash set
/// (`dash_start`, `dash_travel`, `dash_land`).
pub fn own_clip_for(effect: &mftr_sim::ability::Effect) -> &'static str {
    use mftr_sim::ability::Effect;
    match effect {
        Effect::Area(_) => "slam",
        Effect::Lunge(_) => "pounce",
        Effect::Dash(_) => "dash",
        _ => "howl",
    }
}

/// The clips of a base with its own (CR10): the stance as authored, and Q W E R from the
/// clips their moves' shapes call for, renamed for the slot. Translations scale with the body.
pub fn own_clips(
    moves: &Moves,
    base: &Loaded,
    translation_scale: f32,
) -> Result<(Vec<Animation>, Vec<mftr_pack::sidecar::Clip>), String> {
    let scaled = |a: &Animation, name: &str| {
        let mut a = a.clone();
        a.name = name.to_string();
        for c in a.channels.iter_mut().filter(|c| c.path == "translation") {
            for v in &mut c.values {
                for x in v.iter_mut() {
                    *x *= translation_scale;
                }
            }
        }
        a
    };
    let mut anims: Vec<Animation> = base.model.animations.iter().map(|a| scaled(a, &a.name)).collect();
    let mut side: Vec<mftr_pack::sidecar::Clip> = base.library.clips.iter().map(|c| sidecar_clip(c, &c.name)).collect();
    let mut take = |from: &str, to: &str| -> Result<(), String> {
        let a =
            base.model.animations.iter().find(|a| a.name == from).ok_or(format!("the base has no `{from}` clip"))?;
        anims.push(scaled(a, to));
        if let Some(c) = base.library.clip(from).map(|i| &base.library.clips[i]) {
            side.push(sidecar_clip(c, to));
        }
        Ok(())
    };
    for (i, slot) in ["q", "w", "e", "r"].iter().enumerate() {
        let id = moves.get(slot);
        let (c, from) = parse(id).ok_or(format!("`{id}` isn't a move"))?;
        let j = ["q", "w", "e", "r"].iter().position(|s| *s == from).unwrap_or(i);
        match own_clip_for(&c.def().abilities[j].effect) {
            "dash" => {
                for part in ["start", "travel", "land"] {
                    take(&format!("dash_{part}"), &format!("{slot}_{part}"))?;
                }
            }
            clip => take(clip, slot)?,
        }
    }
    Ok((anims, side))
}

fn is_ability_clip(name: &str) -> bool {
    ["q", "w", "e", "r"].iter().any(|s| name == *s || name.starts_with(&format!("{s}_")))
}

/// The clip names (and their sidecar entries) that `moves` gives the model: the stance from the
/// attack's champion, each ability slot from its move's champion. Returns the animations and the
/// sidecar clips, renamed.
pub fn clips(
    moves: &Moves,
    packs: &Packs,
    model: &Model,
    translation_scale: f32,
) -> Result<(Vec<Animation>, Vec<mftr_pack::sidecar::Clip>), String> {
    let mut anims = Vec::new();
    let mut side = Vec::new();
    let mut take = |pack: &Loaded, from_name: &str, to_name: &str| {
        if let Some(a) = pack.model.animations.iter().find(|a| a.name == from_name) {
            anims.push(crate::Retarget::new(&pack.model, model).clip(a, to_name, translation_scale));
            if let Some(c) = pack.library.clip(from_name).map(|i| &pack.library.clips[i]) {
                side.push(sidecar_clip(c, to_name));
            }
        }
    };
    let (stance, _) = parse(moves.get("attack")).ok_or(format!("`{}` isn't an attack move", moves.get("attack")))?;
    let stance_pack = packs.get(stance).ok_or(format!("{}'s pack isn't in the catalog", stance.def().name))?;
    for a in &stance_pack.model.animations {
        if !is_ability_clip(&a.name) {
            take(stance_pack, &a.name, &a.name);
        }
    }
    for slot in ["q", "w", "e", "r"] {
        let id = moves.get(slot);
        let (c, from) = parse(id).ok_or(format!("`{id}` isn't a move"))?;
        if !fits(id, slot) {
            return Err(format!("`{id}` can't go on {}", slot.to_uppercase()));
        }
        let pack = packs.get(c).ok_or(format!("{}'s pack isn't in the catalog", c.def().name))?;
        for a in &pack.model.animations {
            let suffix =
                if a.name == from { Some("") } else { a.name.strip_prefix(from).filter(|r| r.starts_with('_')) };
            if let Some(rest) = suffix {
                take(pack, &a.name.clone(), &format!("{slot}{rest}"));
            }
        }
    }
    Ok((anims, side))
}

fn sidecar_clip(c: &mftr_pack::pose::Clip, name: &str) -> mftr_pack::sidecar::Clip {
    mftr_pack::sidecar::Clip {
        name: name.to_string(),
        frames: (c.length * 30.0).round() as u32,
        looping: c.looping,
        layer: match c.layer {
            mftr_pack::pose::Layer::Full => "full",
            mftr_pack::pose::Layer::Upper => "upper",
            mftr_pack::pose::Layer::Additive => "additive",
        }
        .to_string(),
        stride_speed: c.stride_speed,
        markers: c
            .markers
            .iter()
            .map(|(n, t)| mftr_pack::sidecar::Marker { name: n.clone(), frame: (t * 30.0).round() as u32 })
            .collect(),
    }
}

/// Rename an event's action: `q.release` from `w` → `w.release`.
fn rename(event: &str, from: &str, to: &str) -> Option<String> {
    let (action, phase) = event.split_once('.')?;
    (action == from).then(|| format!("{to}.{phase}"))
}

/// A color in the signature hue: colored ramps (not whites, greys or blacks) turn to `hue`,
/// keeping their saturation and brightness.
fn rehue(c: (f32, f32, f32), hue: u16) -> (f32, f32, f32) {
    let (r, g, b) = c;
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let s = if max > 0.0 { (max - min) / max } else { 0.0 };
    if s < 0.2 {
        return c;
    }
    let h = hue as f32 % 360.0;
    let k = |n: f32| {
        let k = (n + h / 60.0) % 6.0;
        max - max * s * k.min(4.0 - k).clamp(0.0, 1.0)
    };
    (k(5.0), k(3.0), k(1.0))
}

/// The effects the moves bring, renamed for their slots, in the signature hue.
pub fn effects(moves: &Moves, packs: &Packs, hue: u16) -> vfx::VfxFile {
    let mut out = Vec::new();
    for slot in SLOTS {
        let Some((c, from)) = parse(moves.get(slot)) else { continue };
        let Some(file) = packs.get(c).and_then(|p| p.vfx.as_ref()) else { continue };
        for e in &file.effects {
            let event = if slot == "attack" && e.event.starts_with("*.") {
                Some(e.event.clone())
            } else {
                rename(&e.event, from, slot)
            };
            if let Some(event) = event {
                let mut e = e.clone();
                e.event = event;
                e.ramp = e.ramp.iter().map(|c| rehue(*c, hue)).collect();
                out.push(e);
            }
        }
    }
    vfx::VfxFile { effects: out }
}

/// The sounds the moves bring, their events renamed for their slots; the stance's unit sounds
/// (footsteps, death, recall, emotes) too.
pub fn sounds(moves: &Moves, packs: &Packs) -> Vec<sfx::Sound> {
    let mut out = Vec::new();
    for slot in SLOTS {
        let Some((c, from)) = parse(moves.get(slot)) else { continue };
        let Some(pack) = packs.get(c) else { continue };
        for s in &pack.sounds {
            let mut events: Vec<String> = s.spec.events.iter().filter_map(|e| rename(e, from, slot)).collect();
            if slot == "attack" {
                events.extend(s.spec.events.iter().filter(|e| e.starts_with("unit.") || e.starts_with("cc.")).cloned());
            }
            if !events.is_empty() {
                let mut s = s.clone();
                s.spec.events = events;
                out.push(s);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_ids_and_slots() {
        assert_eq!(parse("ember.q"), Some((ChampionId::Ember, "q")));
        assert_eq!(parse("ember.z"), None);
        assert!(fits("ember.q", "w"));
        assert!(!fits("ember.r", "q"));
        assert!(!fits("ember.q", "r"));
        assert!(fits("rook.r", "r"));
        assert!(!fits("rook.attack", "q"));
        assert_eq!(library().len(), 50);
        assert_eq!(rename("q.release", "q", "w").as_deref(), Some("w.release"));
        assert_eq!(rename("w.release", "q", "w"), None);
        let red = rehue((0.2, 0.9, 0.8), 0);
        assert!(red.0 > red.1 && red.0 > red.2, "{red:?}");
        assert_eq!(rehue((1.0, 1.0, 1.0), 120), (1.0, 1.0, 1.0));
    }
}
