//! Wardrobe items (22 §4, BW5): what the forge does for an item beyond an ordinary part.
//!
//! An item is authored once on the reference body and fitted to every base at build time
//! (`mftr_blender.wardrobe`); in a base it is a catalog part like any other (`part.<slot>.<id>`),
//! flagged `covers` in the base's parts list. Where the old parts hid the body by bone (`hides`),
//! a covering item hides **the body under it, by its geometry** (22 §4.4): a body triangle goes
//! when each of its corners, looking out along its normal, meets the item within `REACH`. So a
//! shirt hides the torso it covers and nothing else, whatever the body's shape, and the body's
//! skin never pokes through it in motion.
//!
//! The same measure gives the QA harness (22 §4.8) its numbers: [`fit`].

use mftr_pack::glb::{Model, Primitive};

/// How far out from the skin an item still covers it (m): a draped hem hangs well off the body.
/// Only the item's faces that look the same way as the skin count (`FACING`), so the inside of
/// an arm, which faces a tunic's side, isn't hidden by it.
pub const REACH: f32 = 0.12;
/// How closely an item's face must look the way the skin does to cover it (cosine).
pub const FACING: f32 = 0.3;

type V3 = [f32; 3];

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: V3) -> V3 {
    let l = dot(a, a).sqrt().max(1e-12);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// Item triangles in a uniform grid, for short rays.
struct Grid {
    cell: f32,
    tris: Vec<[V3; 3]>,
    cells: std::collections::HashMap<(i32, i32, i32), Vec<u32>>,
    lo: V3,
    hi: V3,
}

impl Grid {
    fn new(tris: Vec<[V3; 3]>, cell: f32) -> Grid {
        let mut cells: std::collections::HashMap<(i32, i32, i32), Vec<u32>> = std::collections::HashMap::new();
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for (i, t) in tris.iter().enumerate() {
            let (mut a, mut b) = ([f32::MAX; 3], [f32::MIN; 3]);
            for p in t {
                for k in 0..3 {
                    a[k] = a[k].min(p[k]);
                    b[k] = b[k].max(p[k]);
                }
            }
            for k in 0..3 {
                lo[k] = lo[k].min(a[k]);
                hi[k] = hi[k].max(b[k]);
            }
            let c0 = a.map(|x| (x / cell).floor() as i32);
            let c1 = b.map(|x| (x / cell).floor() as i32);
            for x in c0[0]..=c1[0] {
                for y in c0[1]..=c1[1] {
                    for z in c0[2]..=c1[2] {
                        cells.entry((x, y, z)).or_default().push(i as u32);
                    }
                }
            }
        }
        Grid { cell, tris, cells, lo, hi }
    }

    /// The nearest hit along `d` from `o` within `len` on a triangle whose normal looks along
    /// `look` by at least `min` (cosine).
    fn first(&self, o: V3, d: V3, len: f32, look: V3, min: f32) -> Option<f32> {
        let e = [o[0] + d[0] * len, o[1] + d[1] * len, o[2] + d[2] * len];
        let c0 = [0, 1, 2].map(|k| (o[k].min(e[k]) / self.cell).floor() as i32);
        let c1 = [0, 1, 2].map(|k| (o[k].max(e[k]) / self.cell).floor() as i32);
        let mut seen = std::collections::HashSet::new();
        let mut best: Option<f32> = None;
        for x in c0[0]..=c1[0] {
            for y in c0[1]..=c1[1] {
                for z in c0[2]..=c1[2] {
                    for &t in self.cells.get(&(x, y, z)).map_or(&[][..], Vec::as_slice) {
                        if !seen.insert(t) {
                            continue;
                        }
                        let tri = &self.tris[t as usize];
                        if dot(norm(cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]))), look) < min {
                            continue;
                        }
                        if let Some(s) = ray_tri(o, d, tri).filter(|s| *s <= len) {
                            best = Some(best.map_or(s, |b: f32| b.min(s)));
                        }
                    }
                }
            }
        }
        best
    }

    /// Whether the segment from `o` along unit `d` for `len` meets a triangle; with `facing`,
    /// only a triangle whose normal looks along `d` by at least that cosine.
    fn hits(&self, o: V3, d: V3, len: f32, facing: Option<f32>) -> bool {
        let e = [o[0] + d[0] * len, o[1] + d[1] * len, o[2] + d[2] * len];
        for k in 0..3 {
            if o[k].max(e[k]) < self.lo[k] || o[k].min(e[k]) > self.hi[k] {
                return false;
            }
        }
        let c0 = [0, 1, 2].map(|k| (o[k].min(e[k]) / self.cell).floor() as i32);
        let c1 = [0, 1, 2].map(|k| (o[k].max(e[k]) / self.cell).floor() as i32);
        let mut seen = std::collections::HashSet::new();
        for x in c0[0]..=c1[0] {
            for y in c0[1]..=c1[1] {
                for z in c0[2]..=c1[2] {
                    for &t in self.cells.get(&(x, y, z)).map_or(&[][..], Vec::as_slice) {
                        if !seen.insert(t) {
                            continue;
                        }
                        let tri = &self.tris[t as usize];
                        if let Some(f) = facing
                            && dot(norm(cross(sub(tri[1], tri[0]), sub(tri[2], tri[0]))), d) < f
                        {
                            continue;
                        }
                        if ray_tri(o, d, tri).is_some_and(|s| s <= len) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

/// Möller–Trumbore: the distance along `d` from `o` to triangle `t`, either face.
fn ray_tri(o: V3, d: V3, t: &[V3; 3]) -> Option<f32> {
    let e1 = sub(t[1], t[0]);
    let e2 = sub(t[2], t[0]);
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1e-10 {
        return None;
    }
    let inv = 1.0 / det;
    let s = sub(o, t[0]);
    let u = dot(s, p) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    let dist = dot(e2, q) * inv;
    (dist > 0.0).then_some(dist)
}

/// Each vertex's normal: the area-weighted mean of its triangles'.
fn normals(p: &Primitive) -> Vec<V3> {
    let mut n = vec![[0.0f32; 3]; p.positions.len()];
    for t in p.indices.chunks(3) {
        let (a, b, c) = (p.positions[t[0] as usize], p.positions[t[1] as usize], p.positions[t[2] as usize]);
        let f = cross(sub(b, a), sub(c, a));
        for &i in t {
            for k in 0..3 {
                n[i as usize][k] += f[k];
            }
        }
    }
    n.into_iter().map(norm).collect()
}

/// A garment's lining (its inside near the open edges, `mftr_blender.sewing`) is marked by its
/// vertices' occlusion alpha at most this.
pub const LINING: f32 = 0.6;

/// The triangles of the meshes `of`, as they are in `model`; without the lining's when
/// `outside` (its faces look inward, the wrong way for which side of a garment is out).
fn triangles(model: &Model, of: &[usize], outside: bool) -> Vec<[V3; 3]> {
    let mut out = Vec::new();
    for &mi in of {
        for p in &model.meshes[mi] {
            for t in p.indices.chunks(3) {
                let lining = t.iter().all(|&i| p.colors.get(i as usize).is_some_and(|c| c[3] <= LINING));
                if outside && lining {
                    continue;
                }
                out.push([p.positions[t[0] as usize], p.positions[t[1] as usize], p.positions[t[2] as usize]]);
            }
        }
    }
    out
}

/// Per primitive of `body`, whether each vertex lies under an item (its outward ray meets one
/// within `REACH`). The vertex starts a hair inside the skin, so an item touching it counts.
fn under(body: &[Primitive], items: &Grid) -> Vec<Vec<bool>> {
    body.iter()
        .map(|p| {
            let n = normals(p);
            p.positions
                .iter()
                .zip(&n)
                .map(|(v, d)| {
                    let o = [v[0] - d[0] * 0.002, v[1] - d[1] * 0.002, v[2] - d[2] * 0.002];
                    items.hits(o, *d, REACH, Some(FACING))
                })
                .collect()
        })
        .collect()
}

/// Hide the triangles of the body meshes (`body`, mesh indices) under the covering items
/// (`items`, mesh indices) of a dressed, unposed `model`: those whose three corners are all
/// under one.
pub fn hide_covered(model: &mut Model, body: &[usize], items: &[usize]) {
    if items.is_empty() {
        return;
    }
    let grid = Grid::new(triangles(model, items, false), 0.04);
    for &mi in body {
        let masks = under(&model.meshes[mi], &grid);
        for (p, mask) in model.meshes[mi].iter_mut().zip(masks) {
            let kept: Vec<u32> =
                p.indices.chunks(3).filter(|t| !t.iter().all(|&i| mask[i as usize])).flatten().copied().collect();
            p.triangles = kept.len() / 3;
            p.indices = kept;
        }
    }
}

/// The layers, inner to outer (22 §4.1): an item of a later layer is pushed clear of a worn item
/// of an earlier one. Accessories and parts without a layer aren't pushed.
pub const LAYERS: [&str; 3] = ["under", "over", "armor"];

/// The deepest an outer garment's vertex may sit inside an inner one and still be pushed out (m):
/// past it, the vertex isn't over that garment at all.
pub const MAX_PUSH: f32 = 0.02;

/// How far an outer layer stands clear of the inner one it covers (m).
pub const LAYER_GAP: f32 = 0.006;

/// The closest point of triangle `t` to `p` (Ericson, Real-Time Collision Detection 5.1.5).
fn closest(p: V3, t: &[V3; 3]) -> V3 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        return [a[0] + ab[0] * v, a[1] + ab[1] * v, a[2] + ab[2] * v];
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        return [a[0] + ac[0] * w, a[1] + ac[1] * w, a[2] + ac[2] * w];
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        return [b[0] + (c[0] - b[0]) * w, b[1] + (c[1] - b[1]) * w, b[2] + (c[2] - b[2]) * w];
    }
    let den = 1.0 / (va + vb + vc);
    let (v, w) = (vb * den, vc * den);
    [a[0] + ab[0] * v + ac[0] * w, a[1] + ab[1] * v + ac[1] * w, a[2] + ab[2] * v + ac[2] * w]
}

/// Push the outer item's vertices (`outer` meshes) clear of the inner item (`inner` meshes) of
/// a dressed, unposed `model`: a vertex within `reach` of the inner surface, inside it or nearer
/// than `LAYER_GAP` outside, goes out along the inner face's normal (22 §4.3). Only the pairs
/// worn together, once, at forge.
pub fn layer(model: &mut Model, inner: &[usize], outer: &[usize], reach: f32) {
    // The outer garment out past the inner; then what of the inner it covers goes, as the body
    // under a garment does (the inner's vertices poking through the middle of the outer's large
    // faces would show).
    clear(model, inner, outer, reach, 1.0);
    tuck(model, outer, inner);
    hide_covered(model, inner, outer);
}

/// Pull the inner garment's vertices that poke through the outer garment back in under it: a
/// vertex whose inward ray (back along its normal) meets the outer's outside within `POKE`, the
/// outer face looking the way the vertex does, goes in past it by `LAYER_GAP`; its neighbours
/// follow smoothly. Vertices below a hem never meet the outer that way and stay.
fn tuck(model: &mut Model, outer: &[usize], inner: &[usize]) {
    const POKE: f32 = 0.04;
    let tris = triangles(model, outer, true);
    if tris.is_empty() {
        return;
    }
    let grid = Grid::new(tris, 0.04);
    let mut pulls: Vec<(V3, V3)> = Vec::new();
    for &mi in inner {
        for p in &model.meshes[mi] {
            let n = normals(p);
            for (i, (v, d)) in p.positions.iter().zip(&n).enumerate() {
                if p.colors.get(i).is_some_and(|c| c[3] <= LINING) {
                    continue;
                }
                let back = [-d[0], -d[1], -d[2]];
                let o = [v[0] + d[0] * 0.001, v[1] + d[1] * 0.001, v[2] + d[2] * 0.001];
                if let Some(s) = grid.first(o, back, POKE, *d, 0.3) {
                    let k = s + LAYER_GAP;
                    pulls.push((*v, [back[0] * k, back[1] * k, back[2] * k]));
                }
            }
        }
    }
    spread(model, inner, &pulls);
}

/// Move the vertices of `meshes` by a smooth field made of `moves` (place, displacement): each
/// takes the strongest one near it, fading to nothing `SPREAD` away (neighbours move together,
/// no face folds over; a lining a few millimetres in moves with its outside).
fn spread(model: &mut Model, meshes: &[usize], moves: &[(V3, V3)]) {
    const SPREAD: f32 = 0.05;
    if moves.is_empty() {
        return;
    }
    let mut near: std::collections::HashMap<(i32, i32, i32), Vec<usize>> = std::collections::HashMap::new();
    for (i, (p, _)) in moves.iter().enumerate() {
        let c = p.map(|x| (x / SPREAD).floor() as i32);
        near.entry((c[0], c[1], c[2])).or_default().push(i);
    }
    for &mi in meshes {
        for p in model.meshes[mi].iter_mut() {
            for v in p.positions.iter_mut() {
                let c = v.map(|x| (x / SPREAD).floor() as i32);
                let mut best = [0.0f32; 3];
                for x in -1..=1 {
                    for y in -1..=1 {
                        for z in -1..=1 {
                            for &i in near.get(&(c[0] + x, c[1] + y, c[2] + z)).map_or(&[][..], Vec::as_slice) {
                                let (q, d) = moves[i];
                                let r2 = dot(sub(*v, q), sub(*v, q)) / (SPREAD * SPREAD);
                                if r2 < 1.0 {
                                    let w = (1.0 - r2) * (1.0 - r2);
                                    let d = [d[0] * w, d[1] * w, d[2] * w];
                                    if dot(d, d) > dot(best, best) {
                                        best = d;
                                    }
                                }
                            }
                        }
                    }
                }
                for k in 0..3 {
                    v[k] += best[k];
                }
            }
        }
    }
}

/// Move the vertices of `moving` clear of the surface of `fixed`: outside it by `LAYER_GAP`
/// (`side` 1) or inside it (`side` -1), those within `reach` of it.
fn clear(model: &mut Model, fixed: &[usize], moving: &[usize], reach: f32, side: f32) {
    let (inner, outer) = (fixed, moving);
    let tris = triangles(model, inner, true);
    if tris.is_empty() {
        return;
    }
    let grid = Grid::new(tris, 0.04);
    let span = (reach / grid.cell).ceil() as i32;
    // Each vertex's push first; then each takes the largest push within `TWIN` of it, so a
    // garment's outside and its lining (a few millimetres in) move as one.
    let mut pushes: Vec<(V3, V3)> = Vec::new();
    for &mi in outer {
        for p in model.meshes[mi].iter() {
            let own = normals(p);
            for (v, vn) in p.positions.iter().zip(&own) {
                let c = v.map(|x| (x / grid.cell).floor() as i32);
                let mut best: Option<(f32, V3, V3)> = None;
                let mut seen = std::collections::HashSet::new();
                for x in -span..=span {
                    for y in -span..=span {
                        for z in -span..=span {
                            for &t in grid.cells.get(&(c[0] + x, c[1] + y, c[2] + z)).map_or(&[][..], Vec::as_slice) {
                                if !seen.insert(t) {
                                    continue;
                                }
                                let tri = &grid.tris[t as usize];
                                let q = closest(*v, tri);
                                let d = dot(sub(*v, q), sub(*v, q));
                                if d < reach * reach && best.is_none_or(|b| d < b.0) {
                                    best = Some((d, q, norm(cross(sub(tri[1], tri[0]), sub(tri[2], tri[0])))));
                                }
                            }
                        }
                    }
                }
                if let Some((_, q, n)) = best {
                    // Along the fixed surface's normal, toward the side wanted; only by a face
                    // looking the way this vertex does (a coat beside the legs isn't inside the
                    // trousers' far side), and never far.
                    let n = [n[0] * side, n[1] * side, n[2] * side];
                    let s = dot(sub(*v, q), n);
                    if dot(n, *vn) < 0.3 || s < -MAX_PUSH {
                        continue;
                    }
                    if s < LAYER_GAP {
                        pushes.push((*v, [n[0] * (LAYER_GAP - s), n[1] * (LAYER_GAP - s), n[2] * (LAYER_GAP - s)]));
                    }
                }
            }
        }
    }
    spread(model, outer, &pushes);
}

/// The QA harness's two numbers for an item on a posed or sculpted body (22 §4.8), both from
/// skinned positions (`face::skinned`):
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fit {
    /// Body vertices still shown (not hidden under the item) with the item's outside (not its
    /// lining) less than 1.5 cm in under them: skin through cloth.
    pub poke: usize,
    /// Item vertices farther than `GAP` from any body vertex: cloth floating off the body.
    pub gaps: usize,
    pub item_vertices: usize,
}

/// The places of the body vertices `fit` counts as poking through (for finding them).
pub fn poke_points(model: &Model, pos: &[Vec<Vec<V3>>], body: &[usize], item: &[usize]) -> Vec<V3> {
    let item_tris: Vec<[V3; 3]> = item
        .iter()
        .flat_map(|&mi| {
            model.meshes[mi].iter().zip(&pos[mi]).flat_map(|(p, q)| {
                // The garment's outside only: skin over its lining can't be seen.
                p.indices
                    .chunks(3)
                    .filter(|t| !t.iter().all(|&i| p.colors.get(i as usize).is_some_and(|c| c[3] <= LINING)))
                    .map(move |t| [q[t[0] as usize], q[t[1] as usize], q[t[2] as usize]])
            })
        })
        .collect();
    let grid = Grid::new(item_tris, 0.04);
    let mut out = Vec::new();
    for &mi in body {
        for (p, q) in model.meshes[mi].iter().zip(&pos[mi]) {
            let posed = Primitive { positions: q.clone(), indices: p.indices.clone(), ..Primitive::default() };
            let n = normals(&posed);
            let shown: std::collections::HashSet<u32> = p.indices.iter().copied().collect();
            for (i, (v, d)) in q.iter().zip(&n).enumerate() {
                if shown.contains(&(i as u32)) && grid.hits(*v, [-d[0], -d[1], -d[2]], 0.015, None) {
                    out.push(*v);
                }
            }
        }
    }
    out
}

/// How far off the body an item vertex may float before it counts as a gap (m).
pub const GAP: f32 = 0.03;

/// Measure an item (`item` meshes) against the body (`body` meshes) at `pos` (skinned positions
/// per mesh, primitive and vertex, as `face::skinned` gives them).
pub fn fit(model: &Model, pos: &[Vec<Vec<V3>>], body: &[usize], item: &[usize]) -> Fit {
    let item_tris: Vec<[V3; 3]> = item
        .iter()
        .flat_map(|&mi| {
            model.meshes[mi].iter().zip(&pos[mi]).flat_map(|(p, q)| {
                // The garment's outside only: skin over its lining can't be seen.
                p.indices
                    .chunks(3)
                    .filter(|t| !t.iter().all(|&i| p.colors.get(i as usize).is_some_and(|c| c[3] <= LINING)))
                    .map(move |t| [q[t[0] as usize], q[t[1] as usize], q[t[2] as usize]])
            })
        })
        .collect();
    let grid = Grid::new(item_tris, 0.04);
    // Skin that pokes through: a shown body vertex whose inward ray (back along its normal)
    // meets the item close by, i.e. the item's surface lies just *inside* the skin there.
    let mut poke = 0;
    let mut body_pts: Vec<V3> = Vec::new();
    for &mi in body {
        for (p, q) in model.meshes[mi].iter().zip(&pos[mi]) {
            let posed = Primitive { positions: q.clone(), indices: p.indices.clone(), ..Primitive::default() };
            let n = normals(&posed);
            let shown: std::collections::HashSet<u32> = p.indices.iter().copied().collect();
            for (i, (v, d)) in q.iter().zip(&n).enumerate() {
                body_pts.push(*v);
                if !shown.contains(&(i as u32)) {
                    continue;
                }
                let back = [-d[0], -d[1], -d[2]];
                if grid.hits(*v, back, 0.015, None) {
                    poke += 1;
                }
            }
        }
    }
    // Gaps: item vertices far from every body vertex (a coarse grid of the body's points).
    let cell = GAP;
    let mut cells: std::collections::HashMap<(i32, i32, i32), Vec<V3>> = std::collections::HashMap::new();
    for b in &body_pts {
        cells.entry((b.map(|x| (x / cell).floor() as i32)).into()).or_default().push(*b);
    }
    let (mut gaps, mut n) = (0, 0);
    for &mi in item {
        for q in &pos[mi] {
            for v in q {
                n += 1;
                let c = v.map(|x| (x / cell).floor() as i32);
                let mut near = false;
                'search: for x in -1..=1 {
                    for y in -1..=1 {
                        for z in -1..=1 {
                            for b in cells.get(&(c[0] + x, c[1] + y, c[2] + z)).map_or(&[][..], Vec::as_slice) {
                                if dot(sub(*b, *v), sub(*b, *v)) < GAP * GAP {
                                    near = true;
                                    break 'search;
                                }
                            }
                        }
                    }
                }
                if !near {
                    gaps += 1;
                }
            }
        }
    }
    Fit { poke, gaps, item_vertices: n }
}

/// One QA harness measure (22 §4.8): an outfit on a base, the body as one slider set it.
#[derive(Clone, Debug)]
pub struct Check {
    pub base: String,
    /// The items worn, by id.
    pub outfit: Vec<String>,
    /// What the body is at: `neutral`, or a body slider at an end (`body_bulk +1`).
    pub body: String,
    /// The outermost item's measure.
    pub fit: Fit,
}

/// How many body vertices may poke through an item before it fails the harness.
pub const POKE_MAX: usize = 12;

/// The QA harness (22 §4.8): every covering item, and each pair of them in layers, on every
/// humanoid base, at neutral and at each body slider's ends. Its numbers are `Fit`'s for the
/// outermost item worn; an outfit fails on more than `POKE_MAX` vertices poking through.
pub fn check(c: &crate::Catalog) -> Vec<Check> {
    check_bodies(c, true)
}

/// `check`, at neutral only when `sliders` is false (the quick form the tests run).
pub fn check_bodies(c: &crate::Catalog, sliders: bool) -> Vec<Check> {
    use crate::looks::{Gender, Looks};
    let mut out = Vec::new();
    let sliders_too = sliders;
    let sliders: Vec<String> = crate::body::presets().iter().map(|p| p.id.clone()).collect();
    for (build, gendered) in crate::looks::BUILDS {
        if !gendered || !build.starts_with("humanoid") {
            continue;
        }
        for gender in [Gender::Masculine, Gender::Feminine] {
            let base = Looks { build: build.to_string(), gender, ..Looks::default() };
            let key = base.base_key();
            let items: Vec<&crate::Part> = c.parts(&key).iter().filter(|p| p.covers).collect();
            let mut outfits: Vec<Vec<&crate::Part>> = items.iter().map(|p| vec![*p]).collect();
            for a in &items {
                for b in &items {
                    let (ra, rb) =
                        (LAYERS.iter().position(|l| *l == a.layer), LAYERS.iter().position(|l| *l == b.layer));
                    if a.slot != b.slot && ra.is_some() && rb.is_some() && ra < rb {
                        outfits.push(vec![*a, *b]);
                    }
                }
            }
            for outfit in outfits {
                let ids: Vec<String> = outfit.iter().map(|p| p.id.clone()).collect();
                let mut bodies = vec![("neutral".to_string(), Vec::new())];
                for s in sliders.iter().filter(|_| sliders_too) {
                    for v in [-1.0f32, 1.0] {
                        bodies.push((format!("{s} {v:+}"), vec![(s.clone(), v)]));
                    }
                }
                for (name, shape) in bodies {
                    let looks = Looks { outfit: ids.clone(), shape, ..base.clone() };
                    let Ok(f) = c.forge("Check", &looks) else { continue };
                    let m = &f.model;
                    let pos = crate::face::skinned(m);
                    let body: Vec<usize> =
                        m.nodes.iter().filter(|n| !n.name.starts_with("part.")).filter_map(|n| n.mesh).collect();
                    let top = outfit.iter().max_by_key(|p| LAYERS.iter().position(|l| *l == p.layer)).unwrap();
                    let item: Vec<usize> = m
                        .nodes
                        .iter()
                        .filter(|n| n.name.strip_prefix("part.") == Some(top.id.as_str()))
                        .filter_map(|n| n.mesh)
                        .collect();
                    out.push(Check {
                        base: key.clone(),
                        outfit: ids.clone(),
                        body: name,
                        fit: fit(m, &pos, &body, &item),
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(z: f32, s: f32) -> Primitive {
        // A square at height z facing +z (counter-clockwise from above), split in two.
        Primitive {
            positions: vec![[-s, -s, z], [s, -s, z], [s, s, z], [-s, s, z]],
            indices: vec![0, 1, 2, 0, 2, 3],
            triangles: 2,
            ..Primitive::default()
        }
    }

    #[test]
    fn a_covering_item_hides_the_skin_under_it_and_nothing_else() {
        let mut m = Model::default();
        // The body: a big square; the item: a small one 1 cm above its middle, and nothing
        // over its corners.
        m.meshes.push(vec![quad(0.0, 0.2)]);
        m.meshes.push(vec![quad(0.01, 0.5)]);
        hide_covered(&mut m, &[0], &[1]);
        assert_eq!(m.meshes[0][0].triangles, 0, "under the item");
        let mut m = Model::default();
        m.meshes.push(vec![quad(0.0, 0.2)]);
        m.meshes.push(vec![quad(0.01, 0.05)]);
        hide_covered(&mut m, &[0], &[1]);
        assert_eq!(m.meshes[0][0].triangles, 2, "the item covers none of the corners");
        // Too far out to cover.
        let mut m = Model::default();
        m.meshes.push(vec![quad(0.0, 0.2)]);
        m.meshes.push(vec![quad(0.2, 0.5)]);
        hide_covered(&mut m, &[0], &[1]);
        assert_eq!(m.meshes[0][0].triangles, 2);
    }
}
