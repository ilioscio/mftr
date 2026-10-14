//! A pack's VFX (11 §3, 05 §5): instances of the client's built-in **VFX kit**, by event, in
//! `<id>.vfx.ron` next to the model. Data only: a kit name, a color ramp and bounded knobs. Sizes
//! that matter for gameplay (projectile widths, area radii) always come from the gameplay data,
//! never from here (05 §1.1).
//!
//! Events are `<action>.<phase>`: actions `attack`, `q`, `w`, `e`, `r`, `d`, `f` or `*` (any);
//! phases `release` (leaving the projectile socket), `projectile` (the projectile's style),
//! `impact`, `detonate` (delayed areas), `start` and `land` (dashes), `fire` (the animation
//! passing its `fire` marker: melee blows, novas, heals; A6), and `telegraph` (the look of a
//! delayed area's ground indicator, 14 §4.4).

use nanoserde::DeRon;

/// 10 §9: VFX parameters ≤ 50 KB.
pub const MAX_VFX_BYTES: usize = 50 * 1024;
pub const MAX_EFFECTS: usize = 64;
pub const MAX_PER_EVENT: usize = 4;
pub const MAX_COUNT: u32 = 48;
pub const MAX_LIFETIME: f32 = 1.5;
/// A `rain` keeps falling across its area for its lifetime: a volley lasts longer.
pub const MAX_RAIN_LIFETIME: f32 = 4.0;

/// Particle kits (any phase but `projectile`).
pub const PARTICLE_KITS: &[&str] =
    &["flare", "burst", "ring", "dust", "trail", "slash", "flash", "crack", "pillar", "shock", "rocks", "rain"];
/// Projectile styles (the `projectile` phase): they decorate and color the gameplay-sized body;
/// its sheath keeps the team's color (14 §4.4).
pub const PROJECTILE_KITS: &[&str] =
    &["orb", "arrow", "net", "lob", "wave", "shard", "beam", "bones", "flame", "bolt", "grab"];
/// Ground-indicator motifs (the `telegraph` phase): the inside of a delayed area in the
/// champion's colors; its outline keeps the team's color.
pub const TELEGRAPH_KITS: &[&str] =
    &["plain", "cracks", "rocks", "frost", "flames", "bones", "stars", "arrows", "sigil", "shadow"];
pub const ACTIONS: &[&str] = &["attack", "q", "w", "e", "r", "d", "f", "*"];
/// `travel` streams a particle kit off a dashing champion every step until it lands.
pub const PHASES: &[&str] =
    &["release", "projectile", "impact", "detonate", "start", "travel", "land", "fire", "telegraph"];

#[derive(Clone, Debug, DeRon)]
pub struct VfxFile {
    pub effects: Vec<Effect>,
}

#[derive(Clone, Debug, DeRon)]
pub struct Effect {
    pub event: String,
    pub kit: String,
    /// 2–6 colors (RGB, 0–1), from birth to death (pixel-style ramps, 10 §2).
    pub ramp: Vec<(f32, f32, f32)>,
    /// Particles (particle kits).
    #[nserde(default)]
    pub count: Option<u32>,
    /// Particle size multiplier (0.25–3); projectile styles ignore it.
    #[nserde(default)]
    pub size: Option<f32>,
    /// Particle speed in m/s (0–12).
    #[nserde(default)]
    pub speed: Option<f32>,
    /// Particle lifetime in seconds (up to 1.5).
    #[nserde(default)]
    pub lifetime: Option<f32>,
    /// `slash` only: the arc's sweep in degrees (10–360; default 140).
    #[nserde(default)]
    pub arc: Option<f32>,
    /// `slash` only: the arc's plane, 0 = a level sweep, 90 = straight down onto the point (0–90).
    #[nserde(default)]
    pub tilt: Option<f32>,
}

pub fn parse(text: &str) -> Result<VfxFile, String> {
    if text.len() > MAX_VFX_BYTES {
        return Err(format!("vfx file is {} bytes, over the {} byte cap", text.len(), MAX_VFX_BYTES));
    }
    VfxFile::deserialize_ron(text).map_err(|e| format!("vfx: {e}"))
}

/// Every problem with a VFX file, as messages (empty = valid).
pub fn check(f: &VfxFile) -> Vec<String> {
    let mut out = Vec::new();
    if f.effects.len() > MAX_EFFECTS {
        out.push(format!("{} effects, at most {MAX_EFFECTS}", f.effects.len()));
    }
    for (i, e) in f.effects.iter().enumerate() {
        let at = |msg: String| format!("vfx effect {i} (`{}`): {msg}", e.event);
        let Some((action, phase)) = e.event.split_once('.') else {
            out.push(at("events are `<action>.<phase>`".into()));
            continue;
        };
        if !ACTIONS.contains(&action) {
            out.push(at(format!("unknown action `{action}`")));
        }
        if !PHASES.contains(&phase) {
            out.push(at(format!("unknown phase `{phase}`")));
        }
        let projectile = phase == "projectile";
        let kits = match phase {
            "projectile" => PROJECTILE_KITS,
            "telegraph" => TELEGRAPH_KITS,
            _ => PARTICLE_KITS,
        };
        if !kits.contains(&e.kit.as_str()) {
            out.push(at(format!("kit `{}` can't play on `{phase}` (allowed: {})", e.kit, kits.join(", "))));
        }
        if projectile && e.size.is_some() {
            out.push(at("projectile styles take their size from the gameplay data (05 §1.1), not `size`".into()));
        }
        if !(2..=6).contains(&e.ramp.len()) {
            out.push(at(format!("{} ramp colors (2–6)", e.ramp.len())));
        }
        if e.ramp.iter().any(|&(r, g, b)| ![r, g, b].iter().all(|c| (0.0..=1.0).contains(c))) {
            out.push(at("ramp colors are 0–1".into()));
        }
        if e.count.is_some_and(|c| c == 0 || c > MAX_COUNT) {
            out.push(at(format!("count 1–{MAX_COUNT}")));
        }
        if e.size.is_some_and(|s| !(0.25..=3.0).contains(&s)) {
            out.push(at("size 0.25–3".into()));
        }
        if e.speed.is_some_and(|s| !(0.0..=12.0).contains(&s)) {
            out.push(at("speed 0–12 m/s".into()));
        }
        let max_life = if e.kit == "rain" { MAX_RAIN_LIFETIME } else { MAX_LIFETIME };
        if e.lifetime.is_some_and(|l| !(0.02..=max_life).contains(&l)) {
            out.push(at(format!("lifetime 0.02–{max_life} s")));
        }
        if (e.arc.is_some() || e.tilt.is_some()) && e.kit != "slash" {
            out.push(at("`arc` and `tilt` are for `slash`".into()));
        }
        if e.arc.is_some_and(|a| !(10.0..=360.0).contains(&a)) {
            out.push(at("arc 10–360°".into()));
        }
        if e.tilt.is_some_and(|t| !(0.0..=90.0).contains(&t)) {
            out.push(at("tilt 0–90°".into()));
        }
    }
    let mut events: Vec<&str> = f.effects.iter().map(|e| e.event.as_str()).collect();
    events.sort();
    for w in events.chunk_by(|a, b| a == b) {
        if w.len() > MAX_PER_EVENT {
            out.push(format!("vfx event `{}` has {} effects, at most {MAX_PER_EVENT}", w[0], w.len()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_valid_file_passes_and_mistakes_are_named() {
        let ok = r#"(effects: [
            (event: "attack.impact", kit: "burst", ramp: [(1.0, 0.9, 0.6), (0.9, 0.4, 0.1)], count: 12, speed: 3.0),
            (event: "q.projectile", kit: "arrow", ramp: [(1.0, 1.0, 0.8), (1.0, 0.6, 0.2)]),
            (event: "*.release", kit: "flare", ramp: [(1.0, 1.0, 1.0), (1.0, 0.8, 0.4)], lifetime: 0.12),
        ])"#;
        assert!(check(&parse(ok).unwrap()).is_empty());
        for (bad, needle) in [
            (
                r#"(effects: [(event: "q.impact", kit: "laser", ramp: [(1.0, 1.0, 1.0), (0.0, 0.0, 0.0)])])"#,
                "kit `laser`",
            ),
            (
                r#"(effects: [(event: "q.projectile", kit: "arrow", ramp: [(1.0, 1.0, 1.0), (0.0, 0.0, 0.0)], size: 2.0)])"#,
                "gameplay data",
            ),
            (r#"(effects: [(event: "q.impact", kit: "burst", ramp: [(1.0, 1.0, 1.0)])])"#, "ramp colors (2–6)"),
            (
                r#"(effects: [(event: "q.impact", kit: "burst", ramp: [(1.0, 1.0, 1.0), (0.0, 0.0, 0.0)], count: 500)])"#,
                "count",
            ),
            (
                r#"(effects: [(event: "z.impact", kit: "burst", ramp: [(1.0, 1.0, 1.0), (0.0, 0.0, 0.0)])])"#,
                "unknown action",
            ),
            (
                r#"(effects: [(event: "q.explode", kit: "burst", ramp: [(1.0, 1.0, 1.0), (0.0, 0.0, 0.0)])])"#,
                "unknown phase",
            ),
            (r#"(effects: [(event: "q.impact", kit: "burst", ramp: [(2.0, 1.0, 1.0), (0.0, 0.0, 0.0)])])"#, "0–1"),
        ] {
            let errs = check(&parse(bad).unwrap());
            assert!(errs.iter().any(|e| e.contains(needle)), "{needle}: {errs:?}");
        }
    }
}
