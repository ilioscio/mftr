//! Paint (CR5a, 20 §9, D70): fill painting. Every part (and the body) is made of a few flat
//! color **regions**, the distinct colors it was built in (a tunic's cloth, its sash); the paint
//! step fills a region with a color from the shared [`PALETTE`]. The team accent can't be painted
//! (05 §1.3), skin takes its tone from the face step, and glowing parts take the champion's
//! **signature** hue (14 §4). Brush strokes and decals in paint images come with CR5b.

use mftr_pack::glb::Model;

/// The shared palette (D70): 64 colors, tuned dark enough for the arena's sun (14 §4). Eight
/// families of eight, light to dark: neutrals, browns, reds, oranges and golds, greens, teals and
/// blues, violets and magentas, metals.
pub const PALETTE: [[f32; 3]; 64] = [
    // Neutrals.
    [0.82, 0.80, 0.76],
    [0.66, 0.64, 0.60],
    [0.52, 0.50, 0.48],
    [0.40, 0.39, 0.38],
    [0.30, 0.30, 0.31],
    [0.22, 0.22, 0.24],
    [0.14, 0.14, 0.16],
    [0.07, 0.07, 0.08],
    // Browns and leathers.
    [0.70, 0.58, 0.42],
    [0.58, 0.44, 0.30],
    [0.48, 0.34, 0.22],
    [0.40, 0.27, 0.16],
    [0.34, 0.22, 0.12],
    [0.28, 0.18, 0.10],
    [0.20, 0.12, 0.07],
    [0.13, 0.08, 0.05],
    // Reds.
    [0.86, 0.36, 0.32],
    [0.74, 0.20, 0.18],
    [0.62, 0.10, 0.10],
    [0.52, 0.07, 0.07],
    [0.42, 0.06, 0.08],
    [0.32, 0.05, 0.07],
    [0.62, 0.30, 0.34],
    [0.40, 0.14, 0.20],
    // Oranges and golds.
    [0.92, 0.62, 0.26],
    [0.84, 0.46, 0.14],
    [0.70, 0.34, 0.08],
    [0.54, 0.24, 0.06],
    [0.86, 0.72, 0.30],
    [0.74, 0.58, 0.20],
    [0.60, 0.44, 0.14],
    [0.46, 0.32, 0.10],
    // Greens.
    [0.56, 0.72, 0.36],
    [0.40, 0.60, 0.24],
    [0.28, 0.48, 0.16],
    [0.18, 0.36, 0.12],
    [0.12, 0.26, 0.10],
    [0.36, 0.42, 0.22],
    [0.24, 0.30, 0.16],
    [0.16, 0.30, 0.22],
    // Teals and blues.
    [0.30, 0.66, 0.66],
    [0.10, 0.48, 0.50],
    [0.06, 0.34, 0.38],
    [0.40, 0.58, 0.82],
    [0.22, 0.40, 0.70],
    [0.14, 0.24, 0.52],
    [0.10, 0.14, 0.36],
    [0.06, 0.08, 0.20],
    // Violets and magentas.
    [0.66, 0.50, 0.82],
    [0.48, 0.32, 0.70],
    [0.34, 0.20, 0.54],
    [0.24, 0.12, 0.40],
    [0.80, 0.36, 0.66],
    [0.60, 0.18, 0.48],
    [0.42, 0.10, 0.34],
    [0.26, 0.06, 0.22],
    // Metals: steel, iron, bronze, brass, gold, silver, copper, dark iron.
    [0.62, 0.64, 0.68],
    [0.42, 0.43, 0.47],
    [0.56, 0.38, 0.20],
    [0.68, 0.52, 0.24],
    [0.76, 0.60, 0.22],
    [0.74, 0.76, 0.80],
    [0.66, 0.36, 0.22],
    [0.22, 0.23, 0.26],
];

/// Material slots a fill never touches: skin (its tone), the team accent (05 §1.3), emissive
/// (the signature).
const UNPAINTED: [&str; 4] = ["skin", "accent", "accent_glow", "emissive"];

/// A paintable region: what it is (`"body"` or a part id), its index there, its color now.
#[derive(Clone, Debug, PartialEq)]
pub struct Region {
    pub target: String,
    pub index: u8,
    pub color: [f32; 3],
}

fn same(a: [f32; 3], b: [f32; 3]) -> bool {
    (0..3).all(|k| (a[k] - b[k]).abs() < 1e-3)
}

/// The meshes of `target` in `model`: the body's (every node not a part) or a part's.
fn meshes_of<'a>(model: &'a Model, target: &'a str) -> impl Iterator<Item = usize> + 'a {
    model.nodes.iter().filter_map(move |n| {
        let mi = n.mesh?;
        let is_target = match n.name.strip_prefix("part.") {
            Some(id) => id == target,
            None => target == "body",
        };
        is_target.then_some(mi)
    })
}

/// The regions of `target` (its distinct colors outside the unpainted slots), in the order they
/// first appear: a region's index is stable for a given catalog.
pub fn regions(model: &Model, target: &str) -> Vec<[f32; 3]> {
    let unpainted: Vec<usize> =
        model.materials.iter().enumerate().filter(|(_, m)| UNPAINTED.contains(&m.as_str())).map(|(i, _)| i).collect();
    let mut out: Vec<[f32; 3]> = Vec::new();
    for mi in meshes_of(model, target) {
        for p in &model.meshes[mi] {
            if p.material.is_some_and(|m| unpainted.contains(&m)) {
                continue;
            }
            for (v, c) in p.colors.iter().enumerate() {
                if crate::eyes::is_eye(p, v) {
                    continue;
                }
                let rgb = [c[0], c[1], c[2]];
                if out.len() < 16 && !out.iter().any(|o| same(*o, rgb)) {
                    out.push(rgb);
                }
            }
        }
    }
    out
}

/// Fill the region of `target` whose color is `from` (from [`regions`] of the unpainted model)
/// with `color`, keeping each vertex's occlusion (alpha).
pub fn fill(model: &mut Model, target: &str, from: [f32; 3], color: [f32; 3]) {
    let unpainted: Vec<usize> =
        model.materials.iter().enumerate().filter(|(_, m)| UNPAINTED.contains(&m.as_str())).map(|(i, _)| i).collect();
    let meshes: Vec<usize> = meshes_of(model, target).collect();
    for mi in meshes {
        for p in &mut model.meshes[mi] {
            if p.material.is_some_and(|m| unpainted.contains(&m)) {
                continue;
            }
            for (v, c) in p.colors.iter_mut().enumerate() {
                if p.uvs.get(v).is_some_and(|u| crate::eyes::is_eye_uv(*u)) {
                    continue;
                }
                if same([c[0], c[1], c[2]], from) {
                    c[0] = color[0];
                    c[1] = color[1];
                    c[2] = color[2];
                }
            }
        }
    }
}

/// The signature color (14 §4) of hue `deg`: saturated and bright, as glows are.
pub fn signature(deg: u16) -> [f32; 3] {
    hsv(deg as f32 % 360.0, 0.75, 1.0)
}

fn hsv(h: f32, s: f32, v: f32) -> [f32; 3] {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m]
}

/// Glowing parts take the signature hue, keeping their brightness.
pub fn glow(model: &mut Model, hue: u16) {
    let Some(slot) = model.materials.iter().position(|m| m == "emissive") else { return };
    let sig = signature(hue);
    for p in model.meshes.iter_mut().flatten().filter(|p| p.material == Some(slot)) {
        for c in &mut p.colors {
            let v = c[0].max(c[1]).max(c[2]);
            c[0] = sig[0] * v;
            c[1] = sig[1] * v;
            c[2] = sig[2] * v;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_palette_is_in_range_and_the_hues_wrap() {
        assert!(PALETTE.iter().flatten().all(|c| (0.0..=1.0).contains(c)));
        assert_eq!(signature(0), signature(360));
        let teal = signature(175);
        assert!(teal[1] > teal[0] && teal[2] > teal[0]);
    }
}
