//! Paint images (CR5b, D70): the creator's brush and decals. Each image belongs to one target
//! (`"body"` or a worn part's id) and covers its UVs (TEXCOORD_0, from the creator bases): 128×128
//! pixels for the body (face, torso and limbs share it), 64×64 for a part, each unpainted (0) or
//! one of up to 15 colors from the shared [`PALETTE`]. In a
//! champion file an image is `("target", [palette indices], ["row", ...])`: one hex digit a pixel,
//! rows from the top, trailing zeros and empty trailing rows left out, so an image is a few
//! hundred bytes to a few kilobytes and needs no decoder.
//!
//! Strokes are painted **on the model, not on the image**: a brush covers every texel whose
//! place on the surface lies within its radius of where it touched ([`Texels`]), so a stroke
//! crosses UV seams cleanly and never bleeds into a neighbouring island.

use std::sync::Arc;

use mftr_pack::glb::{Model, Paint};

use crate::paint::PALETTE;

/// Pixels a side: the body's, a part's.
pub const BODY_SIZE: usize = 128;
pub const PART_SIZE: usize = 64;

/// Pixels a side of `target`'s image.
pub fn size_for(target: &str) -> usize {
    if target == "body" { BODY_SIZE } else { PART_SIZE }
}
/// Images a recipe may carry.
pub const MAX_IMAGES: usize = 8;
/// Colors an image may use.
pub const MAX_COLORS: usize = 15;

#[derive(Clone, Debug, PartialEq)]
pub struct Image {
    pub target: String,
    /// Palette indices; pixel `k` (1–15) is `colors[k - 1]`.
    pub colors: Vec<u8>,
    /// `size × size`, rows from the top.
    pub pixels: Vec<u8>,
    pub size: usize,
}

impl Image {
    pub fn new(target: &str) -> Image {
        let size = size_for(target);
        Image { target: target.to_string(), colors: Vec::new(), pixels: vec![0; size * size], size }
    }

    /// The pixel value for palette color `color`, adding it if there's room.
    fn slot(&mut self, color: u8) -> Option<u8> {
        if let Some(i) = self.colors.iter().position(|c| *c == color) {
            return Some(i as u8 + 1);
        }
        if self.colors.len() >= MAX_COLORS {
            return None;
        }
        self.colors.push(color);
        Some(self.colors.len() as u8)
    }

    /// Drop colors no pixel uses (and renumber).
    pub fn compact(&mut self) {
        let mut used = [false; MAX_COLORS + 1];
        for &p in &self.pixels {
            used[p as usize] = true;
        }
        let mut map = [0u8; MAX_COLORS + 1];
        let mut colors = Vec::new();
        for (k, c) in self.colors.iter().enumerate() {
            if used[k + 1] {
                colors.push(*c);
                map[k + 1] = colors.len() as u8;
            }
        }
        for p in &mut self.pixels {
            *p = map[*p as usize];
        }
        self.colors = colors;
    }

    pub fn is_empty(&self) -> bool {
        self.pixels.iter().all(|p| *p == 0)
    }

    /// The rows as written in a champion file.
    pub fn rows(&self) -> Vec<String> {
        let mut rows: Vec<String> = self
            .pixels
            .chunks(self.size)
            .map(|r| {
                let s: String = r.iter().map(|p| char::from_digit(*p as u32, 16).unwrap_or('0')).collect();
                s.trim_end_matches('0').to_string()
            })
            .collect();
        while rows.last().is_some_and(String::is_empty) {
            rows.pop();
        }
        rows
    }

    /// Read an image back from its parts as written. `Err` says what's wrong.
    pub fn from_rows(target: &str, colors: Vec<u8>, rows: &[String]) -> Result<Image, String> {
        if colors.len() > MAX_COLORS {
            return Err(format!("at most {MAX_COLORS} colors"));
        }
        if colors.iter().any(|c| *c as usize >= PALETTE.len()) {
            return Err(format!("colors are palette indices, 0–{}", PALETTE.len() - 1));
        }
        let size = size_for(target);
        if rows.len() > size {
            return Err(format!("at most {size} rows"));
        }
        let mut img = Image { target: target.to_string(), colors, pixels: vec![0; size * size], size };
        for (y, row) in rows.iter().enumerate() {
            if row.len() > size {
                return Err(format!("row {y}: at most {size} pixels"));
            }
            for (x, ch) in row.chars().enumerate() {
                let v = ch.to_digit(16).ok_or(format!("row {y}: pixels are hex digits"))? as usize;
                if v > img.colors.len() {
                    return Err(format!("row {y}: pixel `{ch}` has no color"));
                }
                img.pixels[y * size + x] = v as u8;
            }
        }
        Ok(img)
    }

    /// RGBA for the shader (alpha 0 unpainted).
    pub fn paint(&self) -> Paint {
        let mut rgba = Vec::with_capacity(self.pixels.len() * 4);
        for &p in &self.pixels {
            match p {
                0 => rgba.extend([0, 0, 0, 0]),
                k => {
                    let c = PALETTE[self.colors[k as usize - 1] as usize % PALETTE.len()];
                    rgba.extend([(c[0] * 255.0) as u8, (c[1] * 255.0) as u8, (c[2] * 255.0) as u8, 255]);
                }
            }
        }
        Paint { size: self.size as u32, rgba }
    }
}

/// The decals (20 §7: stamps a brush puts down whole), each a shape on the unit square
/// (x, y from -1 to 1, y up).
pub const DECALS: [(&str, &str); 6] = [
    ("ring", "Ring"),
    ("diamond", "Diamond"),
    ("cross", "Cross"),
    ("chevron", "Chevron"),
    ("star", "Star"),
    ("stripes", "Stripes"),
];

fn decal_covers(id: &str, x: f32, y: f32) -> bool {
    let r = (x * x + y * y).sqrt();
    match id {
        "ring" => (0.6..0.95).contains(&r),
        "diamond" => x.abs() + y.abs() < 0.95,
        "cross" => (x.abs() < 0.22 || y.abs() < 0.22) && x.abs() < 0.95 && y.abs() < 0.95,
        "chevron" => (y - (x.abs() * 0.9 - 0.35)).abs() < 0.22 && x.abs() < 0.9,
        "star" => {
            let a = y.atan2(x) - std::f32::consts::FRAC_PI_2;
            let k = ((5.0 * a).cos() + 1.0) * 0.5;
            r < 0.38 + 0.57 * k * k
        }
        "stripes" => r < 0.95 && ((x + y) * 2.5).rem_euclid(1.0) < 0.5,
        _ => false,
    }
}

/// Where each texel of a target lies on the model: its position on the surface and the normal
/// there (rest pose, as forged), for the texels some triangle of the target covers.
pub struct Texels {
    pub at: Vec<Option<([f32; 3], [f32; 3])>>,
    pub size: usize,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt().max(1e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// The meshes of `target` in `model` (the body's: every node not a part).
pub fn meshes_of(model: &Model, target: &str) -> Vec<usize> {
    model
        .nodes
        .iter()
        .filter_map(|n| {
            let mi = n.mesh?;
            let hit = match n.name.strip_prefix("part.") {
                Some(id) => id == target,
                None => target == "body",
            };
            hit.then_some(mi)
        })
        .collect()
}

/// Which target a mesh belongs to.
pub fn target_of(model: &Model, mesh: usize) -> Option<String> {
    let n = model.nodes.iter().find(|n| n.mesh == Some(mesh))?;
    Some(n.name.strip_prefix("part.").map_or("body".to_string(), str::to_string))
}

impl Texels {
    /// Rasterize `target`'s triangles in UV space.
    pub fn of(model: &Model, target: &str) -> Texels {
        let size = size_for(target);
        let mut at = vec![None; size * size];
        for mi in meshes_of(model, target) {
            for p in &model.meshes[mi] {
                if p.uvs.len() != p.positions.len() {
                    continue;
                }
                for t in p.indices.chunks(3) {
                    let (a, b, c) = (t[0] as usize, t[1] as usize, t[2] as usize);
                    let uv = [p.uvs[a], p.uvs[b], p.uvs[c]].map(|u| [u[0] * size as f32, u[1] * size as f32]);
                    let pos = [p.positions[a], p.positions[b], p.positions[c]];
                    let n = norm(cross(sub(pos[1], pos[0]), sub(pos[2], pos[0])));
                    let lo_x = uv.iter().map(|u| u[0]).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
                    let hi_x = (uv.iter().map(|u| u[0]).fold(f32::MIN, f32::max).ceil() as usize).min(size);
                    let lo_y = uv.iter().map(|u| u[1]).fold(f32::MAX, f32::min).floor().max(0.0) as usize;
                    let hi_y = (uv.iter().map(|u| u[1]).fold(f32::MIN, f32::max).ceil() as usize).min(size);
                    let area =
                        (uv[1][0] - uv[0][0]) * (uv[2][1] - uv[0][1]) - (uv[2][0] - uv[0][0]) * (uv[1][1] - uv[0][1]);
                    if area.abs() < 1e-9 {
                        continue;
                    }
                    for y in lo_y..hi_y {
                        for x in lo_x..hi_x {
                            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                            let w1 = ((px - uv[0][0]) * (uv[2][1] - uv[0][1])
                                - (uv[2][0] - uv[0][0]) * (py - uv[0][1]))
                                / area;
                            let w2 = ((uv[1][0] - uv[0][0]) * (py - uv[0][1])
                                - (px - uv[0][0]) * (uv[1][1] - uv[0][1]))
                                / area;
                            let w0 = 1.0 - w1 - w2;
                            // A little past the edges, so seams' texels are covered.
                            let e = -0.6 / area.abs().sqrt();
                            if w0 >= e && w1 >= e && w2 >= e {
                                let q = [0, 1, 2].map(|k| pos[0][k] * w0 + pos[1][k] * w1 + pos[2][k] * w2);
                                let slot = &mut at[y * size + x];
                                if slot.is_none() || (w0 >= 0.0 && w1 >= 0.0 && w2 >= 0.0) {
                                    *slot = Some((q, n));
                                }
                            }
                        }
                    }
                }
            }
        }
        Texels { at, size }
    }
}

/// What a stroke puts down.
#[derive(Clone, Copy, Debug)]
pub enum Tool<'a> {
    /// A round brush in palette color `.0`; `None` erases.
    Brush(Option<u8>),
    /// A decal by id in a palette color.
    Decal(&'a str, u8),
}

/// Paint on `img` around `hit` (a point on the target's surface, with its normal), `radius` in
/// meters. Returns whether any pixel changed; false too when the image has no room for the color.
pub fn stroke(img: &mut Image, texels: &Texels, hit: [f32; 3], normal: [f32; 3], radius: f32, tool: Tool) -> bool {
    if texels.size != img.size {
        return false;
    }
    let value = match tool {
        Tool::Brush(None) => 0,
        Tool::Brush(Some(c)) | Tool::Decal(_, c) => match img.slot(c) {
            Some(v) => v,
            None => return false,
        },
    };
    // The decal's frame on the surface: up as near the model's up as the surface allows.
    let n = norm(normal);
    let up0 = if n[1].abs() > 0.95 { [0.0, 0.0, -1.0] } else { [0.0, 1.0, 0.0] };
    let up = norm(sub(up0, n.map(|k| k * dot(up0, n))));
    let right = cross(up, n);
    let mut changed = false;
    for (i, t) in texels.at.iter().enumerate() {
        let Some((q, qn)) = t else { continue };
        let d = sub(*q, hit);
        if dot(d, d) > radius * radius || dot(*qn, n) < 0.0 {
            continue;
        }
        if let Tool::Decal(id, _) = tool
            && !decal_covers(id, dot(d, right) / radius, dot(d, up) / radius)
        {
            continue;
        }
        if img.pixels[i] != value {
            img.pixels[i] = value;
            changed = true;
        }
    }
    img.compact();
    changed
}

/// Put each image on its target's primitives (those with UVs).
pub fn apply(model: &mut Model, images: &[Image]) {
    for img in images.iter().filter(|i| !i.is_empty()) {
        let paint = Arc::new(img.paint());
        for mi in meshes_of(model, &img.target) {
            for p in &mut model.meshes[mi] {
                if !p.uvs.is_empty() {
                    p.paint = Some(paint.clone());
                }
            }
        }
    }
}

/// The nearest triangle `origin + t·dir` hits (rest pose, as forged): the mesh, the point and
/// the triangle's normal.
pub fn raycast(model: &Model, origin: [f32; 3], dir: [f32; 3]) -> Option<(usize, [f32; 3], [f32; 3])> {
    let mut best: Option<(f32, usize, [f32; 3], [f32; 3])> = None;
    for (mi, prims) in model.meshes.iter().enumerate() {
        for p in prims {
            for t in p.indices.chunks(3) {
                let (a, b, c) = (p.positions[t[0] as usize], p.positions[t[1] as usize], p.positions[t[2] as usize]);
                let (e1, e2) = (sub(b, a), sub(c, a));
                let h = cross(dir, e2);
                let det = dot(e1, h);
                if det.abs() < 1e-9 {
                    continue;
                }
                let s = sub(origin, a);
                let u = dot(s, h) / det;
                if !(0.0..=1.0).contains(&u) {
                    continue;
                }
                let q = cross(s, e1);
                let v = dot(dir, q) / det;
                if v < 0.0 || u + v > 1.0 {
                    continue;
                }
                let dist = dot(e2, q) / det;
                if dist > 0.0 && best.is_none_or(|b| dist < b.0) {
                    let point = [0, 1, 2].map(|k| origin[k] + dir[k] * dist);
                    let mut n = norm(cross(e1, e2));
                    if dot(n, dir) > 0.0 {
                        n = n.map(|k| -k);
                    }
                    best = Some((dist, mi, point, n));
                }
            }
        }
    }
    best.map(|(_, mi, p, n)| (mi, p, n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_round_trip_and_trim() {
        let mut img = Image::new("body");
        let v = img.slot(18).unwrap();
        img.pixels[3] = v;
        img.pixels[img.size * 5] = v;
        let rows = img.rows();
        assert_eq!(rows.len(), 6);
        assert_eq!(rows[0], "0001");
        assert_eq!(rows[1], "");
        assert_eq!(Image::from_rows("body", img.colors.clone(), &rows).unwrap(), img);
        assert!(Image::from_rows("body", vec![1], &["2".into()]).is_err());
    }

    #[test]
    fn decals_have_shapes() {
        for (id, _) in DECALS {
            assert!((0..400).any(|i| decal_covers(id, (i % 20) as f32 / 10.0 - 1.0, (i / 20) as f32 / 10.0 - 1.0)));
        }
    }
}
