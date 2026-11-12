//! Free sculpting (CR9, 20 §5.3): what the sliders don't reach. A sculpt is **sparse offsets on
//! the base's vertices**, quantized to 1 mm, on top of the sliders: it only reshapes what is
//! there, never adds geometry (the triangle budget and the faceted look stay).
//!
//! - **Welds:** the base's mesh splits a vertex wherever its color or UVs change; a weld is one
//!   place on the surface, all its copies moving together. Offsets are by weld id, which is
//!   stable for a given base export.
//! - **Brushes** (push, pull, grab, inflate, smooth, flatten) act on the welds within a radius
//!   of where a ray meets the forged body, with a smooth falloff, and **mirrored by default**.
//! - **Bounded:** no weld moves more than [`MAX_OFFSET`] from where the sliders put it, so
//!   silhouettes and hitboxes stay honest (20 §6.2) and the mesh can't break.
//! - **Parts follow** (20 §7.1): each part vertex is bound to the nearest welds of the body (as
//!   the parts' shape keys were made, `mftr_blender.parts`) and moves as they do.

use std::collections::HashMap;

use mftr_pack::glb::Model;

use crate::image::{meshes_of, raycast};

/// The furthest a weld may move (m): about half a head (20 §5.3's envelope, *(start)*).
pub const MAX_OFFSET: f32 = 0.12;
/// Welds a recipe may move.
pub const MAX_WELDS: usize = 1000;
/// Part vertices further than this from the body don't follow it (a sword's tip).
const BIND_REACH: f32 = 0.25;
/// Part vertices this close follow the body fully.
const FULL_FOLLOW: f32 = 0.1;

/// The welds of a base's body.
pub struct Welds {
    /// Per body mesh (in `meshes_of(model, "body")` order), per primitive, per vertex: its weld.
    pub of: Vec<Vec<Vec<u32>>>,
    /// Each weld's rest position (the base's, before sliders).
    pub rest: Vec<[f32; 3]>,
    /// Each weld's mirror across X (itself on the middle line).
    pub mirror: Vec<u32>,
    /// Each weld's neighbours along the surface.
    pub neighbours: Vec<Vec<u32>>,
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn scale(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    scale(a, 1.0 / len(a).max(1e-9))
}

fn key(p: [f32; 3]) -> (i32, i32, i32) {
    // 0.1 mm cells: copies of a vertex are exactly equal.
    ((p[0] * 1e4).round() as i32, (p[1] * 1e4).round() as i32, (p[2] * 1e4).round() as i32)
}

impl Welds {
    pub fn of_model(model: &Model) -> Welds {
        let mut ids: HashMap<(i32, i32, i32), u32> = HashMap::new();
        let mut rest = Vec::new();
        let mut of = Vec::new();
        let mut edges: Vec<(u32, u32)> = Vec::new();
        for mi in meshes_of(model, "body") {
            let mut per_mesh = Vec::new();
            for p in &model.meshes[mi] {
                let w: Vec<u32> = p
                    .positions
                    .iter()
                    .map(|&pos| {
                        *ids.entry(key(pos)).or_insert_with(|| {
                            rest.push(pos);
                            rest.len() as u32 - 1
                        })
                    })
                    .collect();
                for t in p.indices.chunks(3) {
                    let (a, b, c) = (w[t[0] as usize], w[t[1] as usize], w[t[2] as usize]);
                    edges.extend([(a, b), (b, c), (c, a)]);
                }
                per_mesh.push(w);
            }
            of.push(per_mesh);
        }
        let mut neighbours = vec![Vec::new(); rest.len()];
        for (a, b) in edges {
            for (x, y) in [(a, b), (b, a)] {
                if x != y && !neighbours[x as usize].contains(&y) {
                    neighbours[x as usize].push(y);
                }
            }
        }
        // Mirrors: the nearest weld to (-x, y, z), within 2 mm (the bases are built symmetric).
        let mut grid: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
        let cell =
            |p: [f32; 3]| ((p[0] * 200.0).floor() as i32, (p[1] * 200.0).floor() as i32, (p[2] * 200.0).floor() as i32);
        for (i, p) in rest.iter().enumerate() {
            grid.entry(cell(*p)).or_default().push(i as u32);
        }
        let mirror = rest
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let q = [-p[0], p[1], p[2]];
                let c = cell(q);
                let mut best = (0.002f32, i as u32);
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        for dz in -1..=1 {
                            for &j in grid.get(&(c.0 + dx, c.1 + dy, c.2 + dz)).map_or(&[][..], Vec::as_slice) {
                                let d = len(sub(rest[j as usize], q));
                                if d < best.0 {
                                    best = (d, j);
                                }
                            }
                        }
                    }
                }
                best.1
            })
            .collect();
        Welds { of, rest, mirror, neighbours }
    }
}

/// A brush (20 §5.3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Brush {
    /// Out along the surface where it was touched (`strength` < 0 pulls in).
    Push,
    /// Along `pull` (a drag, in the model's space).
    Grab([f32; 3]),
    /// Each weld out along its own normal (swells, keeping the shape round).
    Inflate,
    /// Toward its neighbours (softens ridges, a sculpt's eraser of sorts).
    Smooth,
    /// Toward the plane where it was touched.
    Flatten,
}

/// A sculpt: offsets by weld, in millimetres (the recipe's form).
pub type Offsets = Vec<(u32, [i16; 3])>;

fn smooth_falloff(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    1.0 - u * u * (3.0 - 2.0 * u)
}

/// Each weld's position in `model` (the forged one), and its normal there.
fn forged(model: &Model, welds: &Welds) -> (Vec<[f32; 3]>, Vec<[f32; 3]>) {
    let mut pos = welds.rest.clone();
    let mut nrm = vec![[0.0f32; 3]; welds.rest.len()];
    for (k, mi) in meshes_of(model, "body").into_iter().enumerate() {
        let Some(per_mesh) = welds.of.get(k) else { continue };
        for (p, w) in model.meshes[mi].iter().zip(per_mesh) {
            for (v, &id) in w.iter().enumerate() {
                pos[id as usize] = p.positions[v];
            }
            for t in p.indices.chunks(3) {
                let (a, b, c) = (p.positions[t[0] as usize], p.positions[t[1] as usize], p.positions[t[2] as usize]);
                let n = cross(sub(b, a), sub(c, a));
                for &i in t {
                    let id = w[i as usize] as usize;
                    nrm[id] = add(nrm[id], n);
                }
            }
        }
    }
    (pos, nrm.into_iter().map(norm).collect())
}

/// One stroke on `offsets` where the ray `origin + t·dir` meets the forged body `model`: the
/// welds within `radius` (m) move by `strength` (m at the centre), mirrored if `symmetric`.
/// `height` is the proportions' height factor (offsets are kept at the base's scale). Returns
/// whether it touched the body.
#[allow(clippy::too_many_arguments)]
pub fn stroke(
    offsets: &mut Offsets,
    welds: &Welds,
    model: &Model,
    origin: [f32; 3],
    dir: [f32; 3],
    brush: Brush,
    radius: f32,
    strength: f32,
    symmetric: bool,
    height: f32,
) -> bool {
    // The body only: parts are moved through it.
    let mut body = model.clone();
    for n in &model.nodes {
        if let (Some(mi), true) = (n.mesh, n.name.starts_with("part.")) {
            body.meshes[mi].clear();
        }
    }
    let Some((_, hit, n)) = raycast(&body, origin, dir) else { return false };
    let (pos, nrm) = forged(model, welds);
    let mut map: HashMap<u32, [f32; 3]> = offsets.iter().map(|(w, o)| (*w, o.map(|x| x as f32 / 1000.0))).collect();
    let mut moved: Vec<(u32, [f32; 3])> = Vec::new();
    for (i, p) in pos.iter().enumerate() {
        let d = len(sub(*p, hit));
        if d >= radius || dot(nrm[i], n) < -0.2 {
            continue;
        }
        let w = smooth_falloff(d / radius);
        let delta = match brush {
            Brush::Push => scale(n, strength * w),
            Brush::Grab(pull) => scale(pull, w),
            Brush::Inflate => scale(nrm[i], strength * w),
            Brush::Smooth => {
                let nb = &welds.neighbours[i];
                if nb.is_empty() {
                    continue;
                }
                let avg = nb.iter().fold([0.0; 3], |acc, &j| add(acc, pos[j as usize]));
                scale(sub(scale(avg, 1.0 / nb.len() as f32), *p), (strength.abs() * 20.0).min(1.0) * w)
            }
            Brush::Flatten => scale(n, -dot(sub(*p, hit), n) * (strength.abs() * 20.0).min(1.0) * w),
        };
        moved.push((i as u32, scale(delta, 1.0 / height.max(0.1))));
    }
    if moved.is_empty() {
        return false;
    }
    for (i, delta) in moved {
        let mut targets = vec![(i, delta)];
        let m = welds.mirror[i as usize];
        if symmetric && m != i {
            targets.push((m, [-delta[0], delta[1], delta[2]]));
        } else if symmetric {
            // On the middle line: stay on it.
            targets[0].1[0] = 0.0;
        }
        for (w, d) in targets {
            let o = map.entry(w).or_insert([0.0; 3]);
            *o = add(*o, d);
            let l = len(*o);
            if l > MAX_OFFSET {
                *o = scale(*o, MAX_OFFSET / l);
            }
        }
    }
    let mut out: Offsets = map
        .into_iter()
        .map(|(w, o)| (w, o.map(|x| (x * 1000.0).round() as i16)))
        .filter(|(_, o)| *o != [0, 0, 0])
        .collect();
    if out.len() > MAX_WELDS {
        // Over the cap: keep the strongest offsets, a weld and its mirror together (a pair has
        // the same strength), so a symmetric stroke stays symmetric on a dense mesh.
        let strength = |o: &[i16; 3]| o.iter().map(|x| (*x as i32).pow(2)).sum::<i32>();
        let pair = |w: u32| w.min(welds.mirror[w as usize]);
        out.sort_by(|a, b| strength(&b.1).cmp(&strength(&a.1)).then(pair(a.0).cmp(&pair(b.0))));
        out.truncate(MAX_WELDS);
        if symmetric {
            let kept: std::collections::HashSet<u32> = out.iter().map(|(w, _)| *w).collect();
            out.retain(|(w, _)| {
                let m = welds.mirror[*w as usize];
                m == *w || kept.contains(&m)
            });
        }
    }
    out.sort_by_key(|(w, _)| *w);
    *offsets = out;
    true
}

/// Move the body by `offsets` and the worn parts with it (in the base's space, after the
/// sliders, before the proportions). `base` is the unforged base, whose rest positions bind the
/// parts.
pub fn apply(model: &mut Model, base: &Model, welds: &Welds, offsets: &Offsets) {
    if offsets.is_empty() {
        return;
    }
    let mut by = vec![[0.0f32; 3]; welds.rest.len()];
    for (w, o) in offsets {
        if let Some(slot) = by.get_mut(*w as usize) {
            *slot = o.map(|x| (x as f32 / 1000.0).clamp(-MAX_OFFSET, MAX_OFFSET));
        }
    }
    for (k, mi) in meshes_of(model, "body").into_iter().enumerate() {
        let Some(per_mesh) = welds.of.get(k) else { continue };
        for (p, w) in model.meshes[mi].iter_mut().zip(per_mesh) {
            for (v, &id) in w.iter().enumerate() {
                p.positions[v] = add(p.positions[v], by[id as usize]);
            }
        }
    }
    // Parts: the three nearest welds at rest, by inverse distance.
    let mut grid: HashMap<(i32, i32, i32), Vec<u32>> = HashMap::new();
    let cell = |p: [f32; 3]| ((p[0] * 10.0).floor() as i32, (p[1] * 10.0).floor() as i32, (p[2] * 10.0).floor() as i32);
    for (i, p) in welds.rest.iter().enumerate() {
        grid.entry(cell(*p)).or_default().push(i as u32);
    }
    for node in &model.nodes {
        let Some(mi) = node.mesh else { continue };
        if !node.name.starts_with("part.") {
            continue;
        }
        for (pi, p) in model.meshes[mi].iter_mut().enumerate() {
            let Some(rest_prim) = base.meshes.get(mi).and_then(|m| m.get(pi)) else { continue };
            for (v, pos) in p.positions.iter_mut().enumerate() {
                let r = rest_prim.positions[v];
                let c = cell(r);
                let mut near: Vec<(f32, u32)> = Vec::new();
                for dx in -2..=2 {
                    for dy in -2..=2 {
                        for dz in -2..=2 {
                            for &w in grid.get(&(c.0 + dx, c.1 + dy, c.2 + dz)).map_or(&[][..], Vec::as_slice) {
                                let d = len(sub(welds.rest[w as usize], r));
                                if d < BIND_REACH {
                                    near.push((d, w));
                                }
                            }
                        }
                    }
                }
                if near.is_empty() {
                    continue;
                }
                near.sort_by(|a, b| a.0.total_cmp(&b.0));
                near.truncate(3);
                let total: f32 = near.iter().map(|(d, _)| 1.0 / d.max(1e-4)).sum();
                let mut delta = [0.0; 3];
                for (d, w) in &near {
                    delta = add(delta, scale(by[*w as usize], (1.0 / d.max(1e-4)) / total));
                }
                // Close to the body, follow fully; further out, less (a hat's brim, a sword).
                let k = 1.0 - ((near[0].0 - FULL_FOLLOW) / (BIND_REACH - FULL_FOLLOW)).clamp(0.0, 1.0);
                *pos = add(*pos, scale(delta, k));
            }
        }
    }
}
