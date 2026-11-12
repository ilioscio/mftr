//! The eyes (22 §2, BW1): what a recipe says of them (`looks.eyes`), and where it goes. Each
//! eyeball of a humanoid base carries its own place on the ball in its UV; the forge writes the
//! recipe's iris, pupil, size, style and sclera into those vertices, and the champion shader
//! draws the iris from them per pixel. Nothing else changes: no uniform, no extra buffer.
//!
//! An eye vertex's UV is `(UV_BASE + 2·style + u, 2·code + v)`: `(u, v)` its place on the ball
//! seen from the front (0–1), `style` the iris's kind, `code` its size plus ten times the
//! sclera. Its color is the iris, its alpha the pupil's size (0–1).

use mftr_pack::glb::{Model, Primitive};
use mftr_sim::data::{self, DataError, Fields, Value};

/// Where eye UVs start: no paint UV reaches it.
pub const UV_BASE: f32 = 2.0;

/// The iris colors (22 §2.2): naturals first, then the others.
pub const IRIS: [[f32; 3]; 16] = [
    [0.22, 0.12, 0.06], // dark brown
    [0.36, 0.20, 0.09], // brown
    [0.48, 0.32, 0.14], // light brown
    [0.42, 0.36, 0.16], // hazel
    [0.24, 0.38, 0.18], // green
    [0.30, 0.42, 0.36], // grey-green
    [0.20, 0.36, 0.56], // blue
    [0.38, 0.52, 0.66], // light blue
    [0.42, 0.44, 0.47], // grey
    [0.05, 0.04, 0.04], // black
    [0.80, 0.52, 0.10], // amber
    [0.62, 0.62, 0.20], // gold
    [0.66, 0.10, 0.08], // red
    [0.44, 0.22, 0.62], // violet
    [0.90, 0.90, 0.92], // white
    [0.10, 0.60, 0.55], // teal
];

pub const IRIS_NAMES: [&str; 16] = [
    "Dark brown",
    "Brown",
    "Light brown",
    "Hazel",
    "Green",
    "Grey-green",
    "Blue",
    "Light blue",
    "Grey",
    "Black",
    "Amber",
    "Gold",
    "Red",
    "Violet",
    "White",
    "Teal",
];

/// What kind of iris: round, a cat's slit, glowing in the signature hue, or blank (one color,
/// no pupil).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Style {
    #[default]
    Round,
    Slit,
    Glow,
    Blank,
}

impl Style {
    pub const ALL: [Style; 4] = [Style::Round, Style::Slit, Style::Glow, Style::Blank];

    pub fn key(self) -> &'static str {
        match self {
            Style::Round => "round",
            Style::Slit => "slit",
            Style::Glow => "glow",
            Style::Blank => "blank",
        }
    }

    pub fn from_key(k: &str) -> Option<Style> {
        Style::ALL.into_iter().find(|s| s.key() == k)
    }
}

/// The sclera's three kinds.
pub const SCLERA: [&str; 3] = ["white", "yellowed", "dark"];

/// A champion's eyes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Eyes {
    /// `IRIS` index.
    pub iris: u8,
    /// The right eye's own iris (heterochromia), or the same as the left.
    pub right: Option<u8>,
    /// The iris's size: 0 is 80%, 4 is 100%, 9 is 125%.
    pub size: u8,
    /// The pupil's size: 0 (a pinpoint) to 10 (wide).
    pub pupil: u8,
    pub style: Style,
    /// `SCLERA` index.
    pub sclera: u8,
}

impl Default for Eyes {
    fn default() -> Self {
        Eyes { iris: 1, right: None, size: 4, pupil: 5, style: Style::Round, sclera: 0 }
    }
}

pub const SIZE_MAX: u8 = 9;
pub const PUPIL_MAX: u8 = 10;

/// Whether vertex `v` of `p` is an eyeball's.
pub fn is_eye(p: &Primitive, v: usize) -> bool {
    p.uvs.get(v).is_some_and(|u| u[0] >= UV_BASE - 0.01)
}

/// Write `eyes` into every eyeball vertex of a forged `model`. `glow` is the signature color,
/// a glowing iris's. The left eye is the one on +X (the character's left).
pub fn apply(model: &mut Model, eyes: &Eyes, glow: [f32; 3]) {
    let code = (eyes.size.min(SIZE_MAX) + 10 * eyes.sclera.min(2)) as f32;
    let style = eyes.style as u8 as f32;
    let color = |i: u8| IRIS[i as usize % IRIS.len()];
    let (left, right) = match eyes.style {
        Style::Glow => (glow, glow),
        _ => (color(eyes.iris), color(eyes.right.unwrap_or(eyes.iris))),
    };
    let pupil = eyes.pupil.min(PUPIL_MAX) as f32 / PUPIL_MAX as f32;
    for p in model.meshes.iter_mut().flatten() {
        for v in 0..p.positions.len() {
            if !is_eye(p, v) || v >= p.colors.len() {
                continue;
            }
            let [x, y] = p.uvs[v];
            let u = (x - UV_BASE).rem_euclid(2.0).min(0.999);
            let w = y.rem_euclid(2.0).min(0.999);
            p.uvs[v] = [UV_BASE + 2.0 * style + u, 2.0 * code + w];
            let c = if p.positions[v][0] >= 0.0 { left } else { right };
            p.colors[v] = [c[0], c[1], c[2], pupil];
        }
    }
}

fn err<T>(at: &str, msg: impl Into<String>) -> Result<T, DataError> {
    Err(DataError { at: at.to_string(), msg: msg.into() })
}

/// Read `eyes: (iris: 6, right: 3, size: 4, pupil: 5, style: "slit", sclera: "white")`, every
/// field optional. Strict: values out of range and unknown styles are errors.
pub fn read(v: Value, at: &str) -> Result<Eyes, DataError> {
    let mut f = Fields::of(v, at, None)?;
    let mut e = Eyes::default();
    if let Some((v, a)) = f.take("iris") {
        e.iris = data::int(v, &a, IRIS.len() as u64 - 1)? as u8;
    }
    if let Some((v, a)) = f.take("right") {
        e.right = Some(data::int(v, &a, IRIS.len() as u64 - 1)? as u8);
    }
    if let Some((v, a)) = f.take("size") {
        e.size = data::int(v, &a, SIZE_MAX as u64)? as u8;
    }
    if let Some((v, a)) = f.take("pupil") {
        e.pupil = data::int(v, &a, PUPIL_MAX as u64)? as u8;
    }
    if let Some((v, a)) = f.take("style") {
        let Value::Str(s) = v else { return err(&a, "a style: \"round\", \"slit\", \"glow\" or \"blank\"") };
        e.style = match Style::from_key(&s) {
            Some(st) => st,
            None => return err(&a, format!("`{s}` isn't a style (round, slit, glow, blank)")),
        };
    }
    if let Some((v, a)) = f.take("sclera") {
        let Value::Str(s) = v else { return err(&a, "a sclera: \"white\", \"yellowed\" or \"dark\"") };
        e.sclera = match SCLERA.iter().position(|k| *k == s) {
            Some(i) => i as u8,
            None => return err(&a, format!("`{s}` isn't a sclera (white, yellowed, dark)")),
        };
    }
    f.done()?;
    Ok(e)
}

/// The recipe's text for `eyes` (only what differs from the default), as `read` reads it.
pub fn write(e: &Eyes) -> String {
    let d = Eyes::default();
    let mut parts = Vec::new();
    if e.iris != d.iris {
        parts.push(format!("iris: {}", e.iris));
    }
    if let Some(r) = e.right {
        parts.push(format!("right: {r}"));
    }
    if e.size != d.size {
        parts.push(format!("size: {}", e.size));
    }
    if e.pupil != d.pupil {
        parts.push(format!("pupil: {}", e.pupil));
    }
    if e.style != d.style {
        parts.push(format!("style: \"{}\"", e.style.key()));
    }
    if e.sclera != d.sclera {
        parts.push(format!("sclera: \"{}\"", SCLERA[e.sclera as usize]));
    }
    format!("({})", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eyes_round_trip_and_the_reader_is_strict() {
        let e = Eyes { iris: 6, right: Some(3), size: 7, pupil: 2, style: Style::Slit, sclera: 1 };
        let text = write(&e);
        assert_eq!(text, "(iris: 6, right: 3, size: 7, pupil: 2, style: \"slit\", sclera: \"yellowed\")");
        assert_eq!(read(data::parse(&text).unwrap(), "eyes").unwrap(), e);
        assert_eq!(read(data::parse(&write(&Eyes::default())).unwrap(), "eyes").unwrap(), Eyes::default());
        let bad = |t: &str| read(data::parse(t).unwrap(), "eyes").is_err();
        assert!(bad("(iris: 16)"));
        assert!(bad("(pupil: 11)"));
        assert!(bad("(style: \"square\")"));
        assert!(bad("(sclera: \"blue\")"));
        assert!(bad("(lashes: 3)"));
    }

    #[test]
    fn apply_writes_the_iris_into_each_eye() {
        let mut m = Model::default();
        // A left eye vertex, a right eye vertex, and a skin vertex.
        let p = Primitive {
            positions: vec![[0.03, 1.7, 0.1], [-0.03, 1.7, 0.1], [0.0, 1.6, 0.1]],
            colors: vec![[0.3, 0.2, 0.1, 0.5], [0.3, 0.2, 0.1, 0.5], [0.8, 0.6, 0.5, 1.0]],
            uvs: vec![[UV_BASE + 0.25, 8.75], [UV_BASE + 0.5, 8.5], [0.4, 0.6]],
            ..Primitive::default()
        };
        m.meshes.push(vec![p]);
        let e = Eyes { iris: 6, right: Some(4), size: 9, pupil: 10, style: Style::Slit, sclera: 2 };
        apply(&mut m, &e, [1.0, 0.0, 0.0]);
        let p = &m.meshes[0][0];
        assert_eq!(p.uvs[0], [UV_BASE + 2.0 + 0.25, 2.0 * 29.0 + 0.75]);
        assert_eq!(&p.colors[0][..3], &IRIS[6]);
        assert_eq!(&p.colors[1][..3], &IRIS[4]);
        assert_eq!(p.colors[0][3], 1.0);
        assert_eq!(p.uvs[2], [0.4, 0.6], "skin untouched");
        assert_eq!(p.colors[2], [0.8, 0.6, 0.5, 1.0]);
        // Applying again keeps each vertex's place on the ball.
        apply(&mut m, &Eyes::default(), [1.0, 0.0, 0.0]);
        assert_eq!(m.meshes[0][0].uvs[0], [UV_BASE + 0.25, 8.75]);
    }
}
