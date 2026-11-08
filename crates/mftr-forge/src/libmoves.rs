//! The creator's own moves (20 §8.2): moves no official champion carries, from
//! `art/library/biped/moves.ron` (compiled in). Each brings an ability (its default numbers), a
//! clip from the shared biped library, its effects by phase, and the sounds of an official move.
//! A recipe names one as `lib.<id>`.

use std::sync::OnceLock;

use mftr_pack::vfx;
use mftr_sim::ability::Ability;
use mftr_sim::data::{self, Fields, Value};

const SOURCE: &str = include_str!("../../../art/library/biped/moves.ron");

pub struct LibMove {
    /// `lib.<id>`.
    pub id: String,
    pub about: String,
    /// The library clip it plays; a dash plays `<clip>_start`, `_travel` and `_land`.
    pub clip: String,
    /// The official move whose sounds it borrows (`champion.slot`).
    pub sounds_like: String,
    pub ultimate: bool,
    pub ability: Ability,
    /// Its effects, each `event` its phase alone (`release`, `detonate`…): the slot's name goes
    /// in front when a recipe takes it.
    pub effects: Vec<vfx::Effect>,
}

/// Every library move, in the file's order.
pub fn all() -> &'static [LibMove] {
    static ALL: OnceLock<Vec<LibMove>> = OnceLock::new();
    ALL.get_or_init(|| read(SOURCE).unwrap_or_else(|e| panic!("art/library/biped/moves.ron: {e}")))
}

/// The library move `id` names (`lib.whirlwind`), if any.
pub fn get(id: &str) -> Option<&'static LibMove> {
    all().iter().find(|m| m.id == id)
}

fn read(text: &str) -> Result<Vec<LibMove>, String> {
    let Value::List(None, items) = data::parse(text).map_err(|e| e.to_string())? else {
        return Err("expected a list of moves".into());
    };
    let mut out: Vec<LibMove> = Vec::new();
    for (i, item) in items.into_iter().enumerate() {
        let at = format!("[{i}]");
        let mut f = Fields::of(item, &at, None).map_err(|e| e.to_string())?;
        let text = |f: &mut Fields, key: &str| -> Result<String, String> {
            match f.req(key).map_err(|e| e.to_string())? {
                (Value::Str(s), _) => Ok(s),
                (_, a) => Err(format!("{a}: expected a string")),
            }
        };
        let id = format!("lib.{}", text(&mut f, "id")?);
        if out.iter().any(|m| m.id == id) {
            return Err(format!("{at}: `{id}` is given twice"));
        }
        let about = text(&mut f, "about")?;
        let clip = text(&mut f, "clip")?;
        let sounds_like = text(&mut f, "sounds_like")?;
        let ultimate = match f.take("ultimate") {
            Some((Value::Ident(b), _)) if b == "true" => true,
            Some((Value::Ident(b), _)) if b == "false" => false,
            Some((_, a)) => return Err(format!("{a}: expected true or false")),
            None => false,
        };
        let (a, aat) = f.req("ability").map_err(|e| e.to_string())?;
        let ability = data::read_ability(a, &aat).map_err(|e| e.to_string())?;
        let effects = match f.take("effects") {
            Some((Value::List(None, list), a)) => list
                .into_iter()
                .enumerate()
                .map(|(j, e)| effect(e, &format!("{a}[{j}]")))
                .collect::<Result<Vec<_>, _>>()?,
            Some((_, a)) => return Err(format!("{a}: expected a list of effects")),
            None => Vec::new(),
        };
        f.done().map_err(|e| e.to_string())?;
        // The effects must be valid once a slot's name is in front.
        let check = vfx::VfxFile {
            effects: effects.iter().map(|e| vfx::Effect { event: format!("q.{}", e.event), ..e.clone() }).collect(),
        };
        if let Some(p) = vfx::check(&check).first() {
            return Err(format!("{at} ({id}): {p}"));
        }
        out.push(LibMove { id, about, clip, sounds_like, ultimate, ability, effects });
    }
    Ok(out)
}

fn effect(v: Value, at: &str) -> Result<vfx::Effect, String> {
    let mut f = Fields::of(v, at, None).map_err(|e| e.to_string())?;
    let mut s = |key: &str| -> Result<String, String> {
        match f.req(key).map_err(|e| e.to_string())? {
            (Value::Str(s), _) => Ok(s),
            (_, a) => Err(format!("{a}: expected a string")),
        }
    };
    let phase = s("phase")?;
    let kit = s("kit")?;
    let ramp = match f.req("ramp").map_err(|e| e.to_string())? {
        (Value::List(None, cs), a) => cs
            .into_iter()
            .enumerate()
            .map(|(k, c)| match c {
                Value::List(None, v) if v.len() == 3 => {
                    let n = |x: &Value| data::num(x.clone(), &format!("{a}[{k}]")).map_err(|e| e.to_string());
                    Ok((n(&v[0])?, n(&v[1])?, n(&v[2])?))
                }
                _ => Err(format!("{a}[{k}]: expected (r, g, b)")),
            })
            .collect::<Result<Vec<_>, _>>()?,
        (_, a) => return Err(format!("{a}: expected a list of colors")),
    };
    let mut opt = |key: &str| -> Result<Option<f32>, String> {
        match f.take(key) {
            Some((v, a)) => data::num(v, &a).map(Some).map_err(|e| e.to_string()),
            None => Ok(None),
        }
    };
    let count = opt("count")?.map(|c| c.max(0.0) as u32);
    let (size, speed, lifetime, arc, tilt) = (opt("size")?, opt("speed")?, opt("lifetime")?, opt("arc")?, opt("tilt")?);
    f.done().map_err(|e| e.to_string())?;
    Ok(vfx::Effect { event: phase, kit, ramp, count, size, speed, lifetime, arc, tilt })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_library_moves_read_and_fit_the_kit_limits() {
        assert!(all().len() >= 8);
        for m in all() {
            assert!(m.id.starts_with("lib."));
            assert!(!m.effects.is_empty(), "{}: no effects", m.id);
            assert!(crate::moves::parse(&m.sounds_like).is_some(), "{}: `{}` isn't a move", m.id, m.sounds_like);
            // Every move fits a kit within the limits as its numbers stand.
            let mut def = *mftr_sim::ChampionId::Rook.def();
            def.abilities[if m.ultimate { 3 } else { 0 }] = m.ability;
            assert!(mftr_sim::limits::problems(&def).is_empty(), "{}: {:?}", m.id, mftr_sim::limits::problems(&def));
        }
        assert!(read("[(id: \"x\")]").is_err());
    }
}
