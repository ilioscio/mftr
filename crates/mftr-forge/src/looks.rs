//! A recipe's looks (CR2, 20 §11.1): how the creator built the champion's body, as the
//! `looks: (...)` field of its champion file.
//!
//! ```ron
//! looks: (
//!     build: "humanoid_slim",
//!     gender: Masculine,
//!     shape: (nose_reach: 0.8, jaw_width: 0.5, body_bulk: -0.2),
//!     proportions: (height: 0.3, legs: -0.1, head: 0.2),
//!     skin: 9,
//! ),
//! ```
//!
//! `shape` holds slider values (-1 to 1) by id; the base's morph targets say which ids exist
//! (unknown ones fail when the champion is forged). `proportions` scale the skeleton (-1 to 1,
//! see [`Proportions`]). `skin` is a tone from [`SKIN_TONES`].

use mftr_sim::data::{self, DataError, Fields, Value};

/// The builds the catalog has, and whether each has genders (20 §3–4).
pub const BUILDS: [(&str, bool); 1] = [("humanoid_slim", true)];

/// The skin tones (20 §5.2), natural and otherwise: an index in a recipe.
pub const SKIN_TONES: [[f32; 3]; 16] = [
    [0.93, 0.76, 0.64],
    [0.86, 0.66, 0.52],
    [0.80, 0.60, 0.48],
    [0.70, 0.50, 0.36],
    [0.58, 0.40, 0.28],
    [0.46, 0.31, 0.21],
    [0.35, 0.23, 0.16],
    [0.25, 0.17, 0.12],
    [0.62, 0.62, 0.56], // grave-pale
    [0.45, 0.62, 0.36], // troll green
    [0.36, 0.48, 0.70], // frost blue
    [0.62, 0.30, 0.26], // ember red
    [0.55, 0.42, 0.66], // dusk violet
    [0.70, 0.66, 0.40], // sallow gold
    [0.30, 0.34, 0.36], // slate
    [0.88, 0.86, 0.82], // chalk
];

/// The tone the bases are painted in (`mftr_blender.creator.SKIN`).
pub const BASE_SKIN: [f32; 3] = [0.80, 0.60, 0.48];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gender {
    #[default]
    Masculine,
    Feminine,
    /// Builds without genders (20 §4).
    None,
}

impl Gender {
    pub fn key(self) -> Option<&'static str> {
        match self {
            Gender::Masculine => Some("masculine"),
            Gender::Feminine => Some("feminine"),
            Gender::None => None,
        }
    }
}

/// Skeleton scaling, each -1 to 1 (20 §6.1). The ranges below are *(start)*.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Proportions {
    /// ±15% overall.
    pub height: f32,
    /// ±25% thigh and calf length.
    pub legs: f32,
    /// ±25% upper arm and forearm length.
    pub arms: f32,
    /// ±20% spine length.
    pub torso: f32,
    /// ±25% head size.
    pub head: f32,
    /// ±35% hand size.
    pub hands: f32,
}

impl Proportions {
    pub const NAMES: [&'static str; 6] = ["height", "legs", "arms", "torso", "head", "hands"];
    pub const RANGE: [f32; 6] = [0.15, 0.25, 0.25, 0.20, 0.25, 0.35];

    pub fn get(&self, i: usize) -> f32 {
        [self.height, self.legs, self.arms, self.torso, self.head, self.hands][i]
    }

    pub fn set(&mut self, i: usize, v: f32) {
        let v = v.clamp(-1.0, 1.0);
        match i {
            0 => self.height = v,
            1 => self.legs = v,
            2 => self.arms = v,
            3 => self.torso = v,
            4 => self.head = v,
            _ => self.hands = v,
        }
    }

    /// The scale factor of proportion `i`.
    pub fn factor(&self, i: usize) -> f32 {
        1.0 + self.get(i) * Self::RANGE[i]
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Looks {
    pub build: String,
    pub gender: Gender,
    /// Slider id and value, in the order written.
    pub shape: Vec<(String, f32)>,
    pub proportions: Proportions,
    pub skin: u8,
    /// Catalog parts worn (CR4, 20 §7): ids like `torso.tunic`, one per slot.
    pub outfit: Vec<String>,
    /// Fills (CR5a, 20 §9): `"body"` or a worn part's id, its region, a palette index.
    pub paint: Vec<(String, u8, u8)>,
    /// The signature hue (14 §4), 0–359 degrees: glowing parts, and the effects (CR6).
    pub signature: u16,
    /// The moves the champion is animated with (CR6, 20 §8): without them, the shared clip
    /// library's.
    pub moves: Option<crate::moves::Moves>,
}

impl Default for Looks {
    fn default() -> Self {
        Looks {
            build: "humanoid_slim".into(),
            gender: Gender::Masculine,
            shape: Vec::new(),
            proportions: Proportions::default(),
            skin: 2,
            outfit: Vec::new(),
            paint: Vec::new(),
            signature: 200,
            moves: None,
        }
    }
}

impl Looks {
    /// A slider's value (0 if unset).
    pub fn slider(&self, id: &str) -> f32 {
        self.shape.iter().find(|(k, _)| k == id).map_or(0.0, |(_, v)| *v)
    }

    pub fn set_slider(&mut self, id: &str, v: f32) {
        let v = v.clamp(-1.0, 1.0);
        match self.shape.iter_mut().find(|(k, _)| k == id) {
            Some(s) => s.1 = v,
            None => self.shape.push((id.to_string(), v)),
        }
    }

    /// The base's file stem: `humanoid_slim_masculine`.
    pub fn base_key(&self) -> String {
        match self.gender.key() {
            Some(g) => format!("{}_{g}", self.build),
            None => self.build.clone(),
        }
    }
}

fn err<T>(at: &str, msg: impl Into<String>) -> Result<T, DataError> {
    Err(DataError { at: at.to_string(), msg: msg.into() })
}

/// A number from -1 to 1.
fn signed(v: Value, at: &str) -> Result<f32, DataError> {
    let Value::Num(t) = v else { return err(at, "expected a number") };
    match t.parse::<f32>() {
        Ok(x) if (-1.0..=1.0).contains(&x) => Ok(x),
        Ok(_) => err(at, "from -1 to 1"),
        Err(_) => err(at, format!("`{t}` isn't a number")),
    }
}

/// Read a `looks` value (`at` is where it sits, for errors).
pub fn read(v: Value, at: &str) -> Result<Looks, DataError> {
    let mut f = Fields::of(v, at, None)?;
    let (b, bat) = f.req("build")?;
    let Value::Str(build) = b else { return err(&bat, "expected a string") };
    let Some(&(_, gendered)) = BUILDS.iter().find(|(n, _)| *n == build) else {
        let all: Vec<&str> = BUILDS.iter().map(|b| b.0).collect();
        return err(&bat, format!("`{build}`: expected one of {}", all.join(", ")));
    };
    let gender = match f.take("gender") {
        Some((g, gat)) => match data::unit(g, &gat)?.as_str() {
            "Masculine" if gendered => Gender::Masculine,
            "Feminine" if gendered => Gender::Feminine,
            other => return err(&gat, format!("`{other}`: this build takes Masculine or Feminine")),
        },
        None if gendered => Gender::Masculine,
        None => Gender::None,
    };
    let mut shape = Vec::new();
    if let Some((s, sat)) = f.take("shape") {
        let Value::Fields(None, list) = s else { return err(&sat, "expected `(slider: value, ...)`") };
        for (k, v) in list {
            let kat = format!("{sat}.{k}");
            if shape.len() >= 128 {
                return err(&kat, "too many sliders");
            }
            shape.push((k, signed(v, &kat)?));
        }
    }
    let mut proportions = Proportions::default();
    if let Some((p, pat)) = f.take("proportions") {
        let mut pf = Fields::of(p, &pat, None)?;
        for (i, name) in Proportions::NAMES.iter().enumerate() {
            if let Some((v, vat)) = pf.take(name) {
                proportions.set(i, signed(v, &vat)?);
            }
        }
        pf.done()?;
    }
    let mut outfit = Vec::new();
    if let Some((o, oat)) = f.take("outfit") {
        let Value::List(None, items) = o else { return err(&oat, "expected a list of part ids: `[\"torso.tunic\"]`") };
        for (i, item) in items.into_iter().enumerate() {
            let iat = format!("{oat}[{i}]");
            let Value::Str(id) = item else { return err(&iat, "expected a part id") };
            if id.len() > 48
                || !id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'.')
            {
                return err(&iat, "part ids are a–z, 0–9, `_` and `.`");
            }
            if outfit.contains(&id) {
                return err(&iat, format!("`{id}` is worn twice"));
            }
            outfit.push(id);
        }
    }
    let skin = match f.take("skin") {
        Some((s, sat)) => data::int(s, &sat, SKIN_TONES.len() as u64 - 1)? as u8,
        None => 2,
    };
    let mut paint = Vec::new();
    if let Some((v, pat)) = f.take("paint") {
        let Value::List(None, items) = v else { return err(&pat, "expected a list of `(\"target\", region, color)`") };
        if items.len() > 128 {
            return err(&pat, "too many fills");
        }
        for (i, item) in items.into_iter().enumerate() {
            let iat = format!("{pat}[{i}]");
            let Value::List(None, mut t) = item else { return err(&iat, "expected `(\"target\", region, color)`") };
            if t.len() != 3 {
                return err(&iat, "expected `(\"target\", region, color)`");
            }
            let color = data::int(t.pop().unwrap(), &iat, crate::paint::PALETTE.len() as u64 - 1)? as u8;
            let region = data::int(t.pop().unwrap(), &iat, 15)? as u8;
            let Value::Str(target) = t.pop().unwrap() else { return err(&iat, "the target is a string") };
            if target != "body" && !outfit.contains(&target) {
                return err(&iat, format!("`{target}` isn't worn"));
            }
            paint.retain(|(t, r, _): &(String, u8, u8)| !(*t == target && *r == region));
            paint.push((target, region, color));
        }
    }
    let signature = match f.take("signature") {
        Some((v, sat)) => data::int(v, &sat, 359)? as u16,
        None => 200,
    };
    let moves = match f.take("moves") {
        Some((v, mat)) => {
            let mut mf = Fields::of(v, &mat, None)?;
            let mut ids: [String; 5] = Default::default();
            for (i, slot) in crate::moves::SLOTS.iter().enumerate() {
                let (m, at) = mf.req(slot)?;
                let Value::Str(id) = m else { return err(&at, "expected a move id like \"ember.q\"") };
                if !crate::moves::fits(&id, slot) {
                    return err(&at, format!("`{id}` isn't a move for {slot}"));
                }
                ids[i] = id;
            }
            mf.done()?;
            Some(crate::moves::Moves { ids })
        }
        None => None,
    };
    f.done()?;
    Ok(Looks { build, gender, shape, proportions, skin, outfit, paint, signature, moves })
}

/// A champion file's looks, if it has any.
pub fn of_champion_file(text: &str) -> Result<Option<Looks>, DataError> {
    match data::read_champion_and_looks(text)?.1 {
        Some((v, at)) => read(v, &at).map(Some),
        None => Ok(None),
    }
}

/// `looks: (...)` as a champion file's field (indented four spaces, ending in a comma).
pub fn write(l: &Looks) -> String {
    let mut out = String::from("    looks: (\n");
    out += &format!("        build: \"{}\",\n", l.build);
    match l.gender {
        Gender::Masculine => out += "        gender: Masculine,\n",
        Gender::Feminine => out += "        gender: Feminine,\n",
        Gender::None => {}
    }
    let shape: Vec<String> =
        l.shape.iter().filter(|(_, v)| *v != 0.0).map(|(k, v)| format!("{k}: {}", round(*v))).collect();
    if !shape.is_empty() {
        out += &format!("        shape: ({}),\n", shape.join(", "));
    }
    let props: Vec<String> = Proportions::NAMES
        .iter()
        .enumerate()
        .filter(|(i, _)| l.proportions.get(*i) != 0.0)
        .map(|(i, n)| format!("{n}: {}", round(l.proportions.get(i))))
        .collect();
    if !props.is_empty() {
        out += &format!("        proportions: ({}),\n", props.join(", "));
    }
    out += &format!("        skin: {},\n", l.skin);
    if !l.outfit.is_empty() {
        let ids: Vec<String> = l.outfit.iter().map(|p| format!("\"{p}\"")).collect();
        out += &format!("        outfit: [{}],\n", ids.join(", "));
    }
    if !l.paint.is_empty() {
        let fills: Vec<String> = l.paint.iter().map(|(t, r, c)| format!("(\"{t}\", {r}, {c})")).collect();
        out += &format!("        paint: [{}],\n", fills.join(", "));
    }
    out += &format!("        signature: {},\n", l.signature);
    if let Some(m) = &l.moves {
        let pairs: Vec<String> =
            crate::moves::SLOTS.iter().zip(&m.ids).map(|(s, id)| format!("{s}: \"{id}\"")).collect();
        out += &format!("        moves: ({}),\n", pairs.join(", "));
    }
    out += "    ),\n";
    out
}

/// Slider values are kept to hundredths in files.
fn round(v: f32) -> f32 {
    (v * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn looks_text(inner: &str) -> String {
        let kit = mftr_sim::data::write_champion(mftr_sim::ChampionId::Rook.def(), "");
        kit.replacen("Champion(\n", &format!("Champion(\n    looks: ({inner}),\n"), 1)
    }

    #[test]
    fn looks_read_and_write_back() {
        let mut l = Looks { gender: Gender::Feminine, skin: 9, ..Looks::default() };
        l.set_slider("nose_reach", 0.8);
        l.set_slider("jaw_width", -0.35);
        l.proportions.set(0, 0.3);
        let kit = mftr_sim::data::write_champion(mftr_sim::ChampionId::Rook.def(), "");
        let text = kit.replacen("Champion(\n", &format!("Champion(\n{}", write(&l)), 1);
        assert_eq!(of_champion_file(&text).unwrap(), Some(l));
        // The kit still reads, looks or not.
        assert_eq!(mftr_sim::data::read_champion(&text).unwrap(), *mftr_sim::ChampionId::Rook.def());
    }

    #[test]
    fn looks_are_strict() {
        let bad = |inner: &str, what: &str| {
            let e = of_champion_file(&looks_text(inner)).unwrap_err();
            assert!(e.to_string().contains(what), "{e}");
        };
        bad("build: \"humanoid_huge\"", "humanoid_huge");
        bad("build: \"humanoid_slim\", gender: Other", "Other");
        bad("build: \"humanoid_slim\", shape: (nose_reach: 1.5)", "-1 to 1");
        bad("build: \"humanoid_slim\", proportions: (wings: 1)", "unknown field");
        bad("build: \"humanoid_slim\", skin: 99", "at most");
        bad("build: \"humanoid_slim\", hat: 1", "unknown field");
        assert_eq!(
            of_champion_file(&mftr_sim::data::write_champion(mftr_sim::ChampionId::Rook.def(), "")).unwrap(),
            None
        );
    }
}
