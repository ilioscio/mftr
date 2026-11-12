//! The brows (22 §3.1, BW2): what a recipe says of them (`looks.brows`), and how the forge
//! shapes them. Each humanoid base carries a brow strip over each eye: 13 columns along the brow
//! ridge, from over the nose to the temple, and 4 rows across the brow line, an *envelope* wide
//! and tall enough for any brow. A strip vertex's UV is `(UV_BASE + along, across)`, its place
//! in the envelope (0–1 each). The forge slides the columns between the brow's start and its
//! tip, and sets the rows by the style's thickness and arch, so every style and size is the same
//! mesh, its silhouette real geometry. Afterwards the UV is `(UV_BASE + along the brow, 2 +
//! across it)`, which the champion shader reads for the hairs' strokes.
//!
//! Styles are data: `art/creator/heads/brows.txt`.

use std::sync::OnceLock;

use mftr_pack::glb::{Model, Primitive};
use mftr_sim::data::{self, DataError, Fields, Value};

/// Where brow UVs start: past the eyes' (2–10).
pub const UV_BASE: f32 = 10.0;
/// The envelope's columns, and its rows' places across the brow line (mm, from the base's
/// brow line: `mftr_blender.authored_head.BROW_ROWS`).
const COLS: usize = 13;
const ROWS: [f32; 4] = [-8.0, -2.5, 3.5, 11.0];
/// The default brow's thickness (mm), top to bottom; a style's `thick` scales it.
const THICK: f32 = 6.5;
/// How much of a brow lies below its middle line (the rest is above).
const BELOW: f32 = 0.45;
/// How far a brow's middle rows stand off the strip (a fraction of its thickness): its volume.
const VOLUME: f32 = 0.16;

const SOURCE: &str = include_str!("../../../art/creator/heads/brows.txt");

/// A brow style (`brows.txt`).
#[derive(Clone, Debug, PartialEq)]
pub struct Style {
    pub id: String,
    pub name: String,
    pub about: String,
    /// Runs in to meet its twin over the nose.
    pub joined: bool,
    pub thick: [f32; 7],
    pub arch: [f32; 7],
}

/// The styles, in the file's order (the first is the default).
pub fn styles() -> &'static [Style] {
    static S: OnceLock<Vec<Style>> = OnceLock::new();
    S.get_or_init(|| parse_styles(SOURCE).unwrap_or_else(|e| panic!("brows.txt: {e}")))
}

fn parse_styles(text: &str) -> Result<Vec<Style>, String> {
    let mut out: Vec<Style> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let at = n + 1;
        if let Some(rest) = line.strip_prefix('[') {
            let (id, head) = rest.split_once(']').ok_or(format!("line {at}: `[id] Name: …`"))?;
            let (name, about) = head.trim().split_once(':').ok_or(format!("line {at}: `Name: a line on it`"))?;
            if out.iter().any(|s| s.id == id) {
                return Err(format!("line {at}: `{id}` twice"));
            }
            out.push(Style {
                id: id.to_string(),
                name: name.trim().to_string(),
                about: about.trim().to_string(),
                joined: false,
                thick: [f32::NAN; 7],
                arch: [f32::NAN; 7],
            });
            continue;
        }
        let s = out.last_mut().ok_or(format!("line {at}: before any `[style]`"))?;
        if line == "joined" {
            s.joined = true;
            continue;
        }
        let mut words = line.split_whitespace();
        let key = words.next().unwrap_or_default();
        let vals: Vec<f32> = words
            .map(|w| w.parse::<f32>().map_err(|_| format!("line {at}: `{w}` isn't a number")))
            .collect::<Result<_, _>>()?;
        let vals: [f32; 7] = vals.try_into().map_err(|_| format!("line {at}: seven values"))?;
        match key {
            "thick" if vals.iter().all(|v| (0.0..=2.0).contains(v)) => s.thick = vals,
            "arch" if vals.iter().all(|v| (-3.0..=3.0).contains(v)) => s.arch = vals,
            "thick" | "arch" => return Err(format!("line {at}: a value out of range")),
            _ => return Err(format!("line {at}: `thick`, `arch` or `joined`")),
        }
    }
    if let Some(s) = out.iter().find(|s| s.thick[0].is_nan() || s.arch[0].is_nan()) {
        return Err(format!("`{}` needs both `thick` and `arch`", s.id));
    }
    if out.is_empty() {
        return Err("no styles".into());
    }
    Ok(out)
}

/// The most a brow setting goes to; 5 is the middle (the style as drawn).
pub const MAX: u8 = 10;

/// One brow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Brow {
    /// `styles()` index.
    pub style: u8,
    /// How heavy, 0–10.
    pub thick: u8,
    /// How far it runs toward the temple.
    pub length: u8,
    /// How high its arch rises (0 flattens it, 10 doubles it).
    pub arch: u8,
    /// Where the arch peaks: toward the nose (0) or the temple (10).
    pub peak: u8,
    /// Its tip up (10) or down (0).
    pub tilt: u8,
    /// How far from its twin it starts: close (0) or far (10).
    pub gap: u8,
    /// How high it sits over the eye.
    pub raise: u8,
}

impl Default for Brow {
    fn default() -> Self {
        Brow { style: 0, thick: 5, length: 5, arch: 5, peak: 5, tilt: 5, gap: 5, raise: 5 }
    }
}

/// The settings' keys in a recipe, in order, and their fields.
pub const KEYS: [&str; 7] = ["thick", "length", "arch", "peak", "tilt", "gap", "raise"];

impl Brow {
    pub fn get(&self, key: &str) -> Option<u8> {
        Some(match key {
            "thick" => self.thick,
            "length" => self.length,
            "arch" => self.arch,
            "peak" => self.peak,
            "tilt" => self.tilt,
            "gap" => self.gap,
            "raise" => self.raise,
            _ => return None,
        })
    }

    pub fn set(&mut self, key: &str, v: u8) -> bool {
        let v = v.min(MAX);
        match key {
            "thick" => self.thick = v,
            "length" => self.length = v,
            "arch" => self.arch = v,
            "peak" => self.peak = v,
            "tilt" => self.tilt = v,
            "gap" => self.gap = v,
            "raise" => self.raise = v,
            _ => return false,
        }
        true
    }

    fn style(&self) -> &'static Style {
        let all = styles();
        &all[self.style as usize % all.len()]
    }

    /// Where the brow starts and ends along the envelope (0 over the nose, 1 at the temple).
    fn span(&self) -> (f32, f32) {
        let start = if self.style().joined { 0.0 } else { 0.19 + (self.gap as f32 - 5.0) * 0.03 };
        (start, 0.84 + (self.length as f32 - 5.0) * 0.028)
    }

    /// The brow's bottom and top (mm across the brow line) at `u` along it (0 start, 1 tip).
    fn edges(&self, u: f32) -> (f32, f32) {
        let st = self.style();
        // The peak moves by warping where along the profile each point reads.
        let w = u.clamp(0.0, 1.0).powf(1.7f32.powf((self.peak as f32 - 5.0) / 5.0));
        // The brow's head is rounded, not cut square.
        let head = 0.55 + 0.45 * smooth(u / 0.12);
        let th = THICK * profile(&st.thick, w) * (0.45 + 0.11 * self.thick as f32) * head;
        let mid = profile(&st.arch, w) * self.arch as f32 / 5.0
            + (self.tilt as f32 - 5.0) * 0.5 * u
            + (self.raise as f32 - 5.0) * 0.5;
        let lo = (mid - BELOW * th).clamp(ROWS[0], ROWS[3]);
        let hi = (mid + (1.0 - BELOW) * th).clamp(ROWS[0], ROWS[3]);
        (lo, hi.max(lo))
    }
}

fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

/// A seven-point profile at `u` (0–1), linearly between its points.
fn profile(p: &[f32; 7], u: f32) -> f32 {
    let x = u.clamp(0.0, 1.0) * 6.0;
    let i = (x.floor() as usize).min(5);
    let f = x - i as f32;
    p[i] + (p[i + 1] - p[i]) * f
}

/// A champion's brows: the left one, and the right if it has its own shape.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Brows {
    pub left: Brow,
    pub right: Option<Brow>,
}

impl Brows {
    pub fn right(&self) -> Brow {
        self.right.unwrap_or(self.left)
    }
}

/// Whether vertex `v` of `p` is a brow strip's.
pub fn is_brow(p: &Primitive, v: usize) -> bool {
    p.uvs.get(v).is_some_and(|u| u[0] >= UV_BASE - 0.01 && u[0] < UV_BASE + 1.99)
}

/// Whether `model` has brow strips (a humanoid base's).
pub fn present(model: &Model) -> bool {
    model.meshes.iter().flatten().any(|p| (0..p.uvs.len()).any(|v| is_brow(p, v)))
}

/// The brows' paint region on the body (`paint::regions(model, "body")`): their color's index.
pub fn color_region(model: &Model) -> Option<usize> {
    let p = model.meshes.iter().flatten().find(|p| (0..p.uvs.len()).any(|v| is_brow(p, v)))?;
    let v = (0..p.uvs.len()).find(|&v| is_brow(p, v))?;
    let c = p.colors.get(v)?;
    crate::paint::regions(model, "body").iter().position(|r| (0..3).all(|k| (r[k] - c[k]).abs() < 1e-3))
}

/// The strip's envelope on one side: each column's four rows.
type Envelope = [[[f32; 3]; 4]; COLS];

fn row_of(across: f32) -> usize {
    // The base writes each row's place in the envelope (0–1) into its UV.
    let t = ROWS[0] + across * (ROWS[3] - ROWS[0]);
    (0..4).min_by(|&a, &b| (ROWS[a] - t).abs().total_cmp(&(ROWS[b] - t).abs())).unwrap()
}

/// The envelope's point at `s` along (0–1) and `t` across (mm).
fn at(env: &Envelope, s: f32, t: f32) -> [f32; 3] {
    let x = s.clamp(0.0, 1.0) * (COLS - 1) as f32;
    let c = (x.floor() as usize).min(COLS - 2);
    let fc = x - c as f32;
    let t = t.clamp(ROWS[0], ROWS[3]);
    let r = (0..3).find(|&r| t <= ROWS[r + 1]).unwrap_or(2);
    let fr = (t - ROWS[r]) / (ROWS[r + 1] - ROWS[r]);
    let mut out = [0.0; 3];
    for (k, o) in out.iter_mut().enumerate() {
        let a = env[c][r][k] + (env[c + 1][r][k] - env[c][r][k]) * fc;
        let b = env[c][r + 1][k] + (env[c + 1][r + 1][k] - env[c][r + 1][k]) * fc;
        *o = a + (b - a) * fr;
    }
    out
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn len(a: [f32; 3]) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

/// Shape every brow strip of a forged `model` (before its bones are baked: the strip is in the
/// base's bind pose). The left brow is the one on +X. Each column `i` of 13 goes to `i / 12` of
/// the way along the brow; its rows run from the brow's bottom to its top, the middle two
/// standing off the strip by its volume. A strip shaped already (its UV's across past 2) is left
/// alone.
pub fn apply(model: &mut Model, brows: &Brows) {
    for p in model.meshes.iter_mut().flatten() {
        let mut env: [Envelope; 2] = [[[[f32::NAN; 3]; 4]; COLS]; 2];
        let mut any = false;
        for v in 0..p.positions.len() {
            if !is_brow(p, v) || p.uvs[v][1] >= 1.5 {
                continue;
            }
            let side = usize::from(p.positions[v][0] < 0.0);
            let col = ((p.uvs[v][0] - UV_BASE) * (COLS - 1) as f32).round() as usize;
            env[side][col.min(COLS - 1)][row_of(p.uvs[v][1])] = p.positions[v];
            any = true;
        }
        if !any || env.iter().flatten().flatten().flatten().any(|x| x.is_nan()) {
            continue;
        }
        for v in 0..p.positions.len() {
            if !is_brow(p, v) || p.uvs[v][1] >= 1.5 {
                continue;
            }
            let side = usize::from(p.positions[v][0] < 0.0);
            let brow = if side == 0 { brows.left } else { brows.right() };
            let col = (((p.uvs[v][0] - UV_BASE) * (COLS - 1) as f32).round() as usize).min(COLS - 1);
            let row = row_of(p.uvs[v][1]);
            let e = &env[side];
            let u = col as f32 / (COLS - 1) as f32;
            let (s0, s1) = brow.span();
            let s = s0 + (s1 - s0) * u;
            let (lo, hi) = brow.edges(u);
            let t = lo + (hi - lo) * row as f32 / 3.0;
            let mut q = at(e, s, t);
            if row == 1 || row == 2 {
                // Off the strip along its outward normal (glTF faces +Z), by the brow's volume.
                let ds = sub(at(e, (s + 0.02).min(1.0), t), at(e, (s - 0.02).max(0.0), t));
                let dt = sub(at(e, s, ROWS[3]), at(e, s, ROWS[0]));
                let mut n =
                    [ds[1] * dt[2] - ds[2] * dt[1], ds[2] * dt[0] - ds[0] * dt[2], ds[0] * dt[1] - ds[1] * dt[0]];
                let l = len(n).max(1e-9);
                let flip = if n[2] < 0.0 { -1.0 } else { 1.0 };
                n = [n[0] / l * flip, n[1] / l * flip, n[2] / l * flip];
                // The envelope's millimetre, in the model's units.
                let mm = len(dt) / (ROWS[3] - ROWS[0]);
                let off = VOLUME * (hi - lo) * mm;
                q = [q[0] + n[0] * off, q[1] + n[1] * off, q[2] + n[2] * off];
            }
            p.positions[v] = q;
            p.uvs[v] = [UV_BASE + u.min(0.999), 2.0 + row as f32 / 3.0];
        }
    }
}

fn err<T>(at: &str, msg: impl Into<String>) -> Result<T, DataError> {
    Err(DataError { at: at.to_string(), msg: msg.into() })
}

fn read_brow(v: Value, at: &str, base: Brow) -> Result<Brow, DataError> {
    let mut f = Fields::of(v, at, None)?;
    let mut b = base;
    if let Some((v, a)) = f.take("style") {
        let Value::Str(s) = v else { return err(&a, "a style's id, in quotes") };
        b.style = match styles().iter().position(|st| st.id == s) {
            Some(i) => i as u8,
            None => return err(&a, format!("`{s}` isn't a brow style")),
        };
    }
    for key in KEYS {
        if let Some((v, a)) = f.take(key) {
            b.set(key, data::int(v, &a, MAX as u64)? as u8);
        }
    }
    f.done()?;
    Ok(b)
}

/// Read `brows: (style: "arched", thick: 6, …, right: (thick: 3))`, every field optional; the
/// right brow's fields start from the left's. Strict: values out of range and unknown styles are
/// errors.
pub fn read(v: Value, at: &str) -> Result<Brows, DataError> {
    let Value::Fields(_, mut fields) = v else { return err(at, "brows: `(style: …, thick: …)`") };
    let right = fields.iter().position(|(k, _)| k == "right").map(|i| fields.remove(i).1);
    let left = read_brow(Value::Fields(None, fields), at, Brow::default())?;
    let right = match right {
        Some(r) => Some(read_brow(r, &format!("{at}.right"), left)?),
        None => None,
    };
    Ok(Brows { left, right })
}

fn write_brow(b: &Brow, from: &Brow) -> Vec<String> {
    let mut parts = Vec::new();
    if b.style != from.style {
        parts.push(format!("style: \"{}\"", styles()[b.style as usize % styles().len()].id));
    }
    for key in KEYS {
        if b.get(key) != from.get(key) {
            parts.push(format!("{key}: {}", b.get(key).unwrap_or(5)));
        }
    }
    parts
}

/// The recipe's text for `brows` (only what differs from the default), as `read` reads it.
pub fn write(b: &Brows) -> String {
    let mut parts = write_brow(&b.left, &Brow::default());
    if let Some(r) = &b.right {
        parts.push(format!("right: ({})", write_brow(r, &b.left).join(", ")));
    }
    format!("({})", parts.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_styles_parse_and_the_default_is_first() {
        let s = styles();
        assert!(s.len() >= 10);
        assert_eq!(s[0].id, "soft");
        assert!(s.iter().any(|x| x.id == "unibrow" && x.joined));
        assert!(parse_styles("[a] A: b\nthick 1 1 1\narch 0 0 0 0 0 0 0").is_err());
        assert!(parse_styles("[a] A: b\nthick 1 1 1 1 1 1 1").is_err());
        assert!(parse_styles("[a] A: b\nthick 9 1 1 1 1 1 1\narch 0 0 0 0 0 0 0").is_err());
    }

    #[test]
    fn brows_round_trip_and_the_reader_is_strict() {
        let mut left = Brow { style: 2, thick: 7, ..Brow::default() };
        left.set("tilt", 8);
        let right = Brow { thick: 3, ..left };
        let b = Brows { left, right: Some(right) };
        let text = write(&b);
        assert_eq!(text, "(style: \"arched\", thick: 7, tilt: 8, right: (thick: 3))");
        assert_eq!(read(data::parse(&text).unwrap(), "brows").unwrap(), b);
        assert_eq!(read(data::parse(&write(&Brows::default())).unwrap(), "brows").unwrap(), Brows::default());
        let bad = |t: &str| read(data::parse(t).unwrap(), "brows").is_err();
        assert!(bad("(thick: 11)"));
        assert!(bad("(style: \"square\")"));
        assert!(bad("(curl: 3)"));
        assert!(bad("(right: (gap: 12))"));
    }

    #[test]
    fn a_brow_fits_its_envelope_and_its_settings_move_it() {
        let d = Brow::default();
        for st in 0..styles().len() as u8 {
            for u in [0.0, 0.3, 0.6, 1.0] {
                let (lo, hi) = Brow { style: st, thick: 10, arch: 10, raise: 10, ..d }.edges(u);
                assert!(lo >= ROWS[0] && hi <= ROWS[3] && lo <= hi);
            }
        }
        let th = |b: Brow| {
            let (lo, hi) = b.edges(0.4);
            hi - lo
        };
        assert!(th(Brow { thick: 9, ..d }) > th(d) * 1.3);
        assert!(Brow { length: 9, ..d }.span().1 > d.span().1);
        assert!(Brow { gap: 0, ..d }.span().0 < d.span().0);
        assert_eq!(Brow { style: 9, ..d }.span().0, 0.0, "the unibrow runs to the middle");
        assert!(Brow { tilt: 10, ..d }.edges(1.0).0 > d.edges(1.0).0);
    }
}
