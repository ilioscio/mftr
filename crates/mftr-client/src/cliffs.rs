//! The cliffs' meshes (15 §4), built from the map's walls.
//!
//! The simulation traces its walls tile by tile, so one cliff is often several polygons that
//! meet along straight tile seams. Drawn piece by piece, those seams showed: a small piece of a
//! long cliff passed for a lone outcrop, faces stood on (or went missing along) seams, a piece
//! whose outline touched itself lost its top. Here the pieces are merged first: every edge two
//! pieces share (split where the other piece has a vertex) cancels out, and what is left, chained
//! into loops, is the cliffs' real outline, with one face all the way along it. The tops are
//! each piece's own triangulation (pieces have no holes), its vertices on the outline moved in
//! to the top of the face.
//!
//! - **Faces:** a broken profile (foot, the lip and back of a ledge, the top edge), each ring
//!   stepped in from the one below by at most a share of the wall's thickness there, so a thin
//!   wall never folds over itself; vertices jittered so the low-poly cliff looks cut, not built.
//! - **Outcrops:** a group of pieces small and on its own (no shared edge with the rest) is a
//!   boulder in the field; the client draws those with rock props, not as cliffs.
//!
//! Output is in meters (x, height, y), the client's world space.

use std::collections::HashMap;

use mftr_sim::Vec2;
use mftr_sim::map::Map;

/// Game units to meters.
const U: f32 = 0.01;
/// How far past the map's edges the land is drawn (u; the client's ground reaches as far). It is
/// rock at the cliffs' height: four frame pieces join the walls, so a wall along the edge has
/// no drop behind it, and open ground that reaches the edge meets a cliff.
pub const MARGIN_U: f32 = 2600.0;
/// A group of pieces smaller than this (u, both sides of its bounds) and on its own is an outcrop.
pub const OUTCROP_U: f32 = 500.0;
/// The face's profile: (meters in from the foot, share of the height).
const PROFILE: [(f32, f32); 4] = [(0.0, 0.0), (0.1, 0.42), (0.22, 0.47), (0.32, 1.0)];
/// Outline steps (u): no longer than this, so it can be smoothed.
const STEP: f32 = 30.0;
/// How far smoothing may move a point of the outline from the simulation's edge (u).
const SMOOTH_U: f32 = 18.0;
/// At most this share of the wall's thickness (measured inward from a vertex) is used by the
/// profile's step in.
const THICKNESS_SHARE: f32 = 0.4;

pub struct Cliffs {
    /// Triangles, three positions each, in meters: one list per group of pieces (for culling).
    pub groups: Vec<Vec<[f32; 3]>>,
    /// Indices into `map.walls` of the pieces that are outcrops (in no group).
    pub outcrops: Vec<usize>,
}

type Key = (i64, i64);

/// A vertex's key: exact for the tracer's shared corners (multiples of the grid's cell).
fn key(p: Vec2) -> Key {
    ((p.x * 64.0).round() as i64, (p.y * 64.0).round() as i64)
}

pub fn build(map: &Map, height_m: f32) -> Cliffs {
    // Every piece wound the same way (positive area, y down: inside to the right of travel). The
    // walls, then the frame past the map's edges.
    let (sx, sy, m) = (map.size.x, map.size.y, MARGIN_U);
    let rect = |x0: f32, y0: f32, x1: f32, y1: f32| {
        vec![Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)]
    };
    let frame =
        [rect(-m, -m, 0.0, sy + m), rect(sx, -m, sx + m, sy + m), rect(0.0, -m, sx, 0.0), rect(0.0, sy, sx, sy + m)];
    let walls_n = map.walls.len();
    let polys: Vec<Vec<Vec2>> = map
        .walls
        .iter()
        .chain(frame.iter())
        .map(|w| {
            let mut w = w.clone();
            if signed_area(&w) < 0.0 {
                w.reverse();
            }
            w
        })
        .collect();
    let n = polys.len();

    // Split each edge where another piece has a vertex on it (seams run along tile lines, where
    // the two sides' outlines turn at different points).
    const CELL: f32 = 50.0;
    let mut grid: HashMap<(i32, i32), Vec<(usize, Vec2)>> = HashMap::new();
    for (pi, poly) in polys.iter().enumerate() {
        for &v in poly {
            grid.entry(((v.x / CELL).floor() as i32, (v.y / CELL).floor() as i32)).or_default().push((pi, v));
        }
    }
    // Each piece's outline, cut where another piece has a vertex.
    let mut cut: Vec<Vec<Vec2>> = vec![Vec::new(); n];
    for (pi, poly) in polys.iter().enumerate() {
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            let ab = b - a;
            let len2 = ab.length_sq();
            if len2 < 1e-6 {
                continue;
            }
            let mut cuts: Vec<(f32, Vec2)> = Vec::new();
            let (lo, hi) = (a.min(b), a.max(b));
            for cy in ((lo.y - 1.0) / CELL).floor() as i32..=((hi.y + 1.0) / CELL).floor() as i32 {
                for cx in ((lo.x - 1.0) / CELL).floor() as i32..=((hi.x + 1.0) / CELL).floor() as i32 {
                    for &(qi, v) in grid.get(&(cx, cy)).into_iter().flatten() {
                        if qi == pi {
                            continue;
                        }
                        let t = (v - a).dot(ab) / len2;
                        if t > 1e-4 && t < 1.0 - 1e-4 && (a + ab * t).distance(v) < 0.05 {
                            cuts.push((t, v));
                        }
                    }
                }
            }
            cuts.sort_by(|x, y| x.0.total_cmp(&y.0));
            let out = &mut cut[pi];
            let mut from = a;
            for v in cuts.into_iter().map(|c| c.1).chain([b]) {
                if key(v) == key(from) {
                    continue;
                }
                out.push(from);
                from = v;
            }
        }
    }
    let mut edges: Vec<(usize, Vec2, Vec2)> = Vec::new();
    for (pi, c) in cut.iter().enumerate() {
        for i in 0..c.len() {
            edges.push((pi, c[i], c[(i + 1) % c.len()]));
        }
    }

    // Cancel shared edges (one piece's a→b against another's b→a) and group the pieces they join.
    let mut parent: Vec<usize> = (0..n).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let mut by_key: HashMap<(Key, Key), Vec<usize>> = HashMap::new();
    for (ei, &(_, a, b)) in edges.iter().enumerate() {
        by_key.entry((key(a), key(b))).or_default().push(ei);
    }
    let mut gone = vec![false; edges.len()];
    for ei in 0..edges.len() {
        if gone[ei] {
            continue;
        }
        let (pi, a, b) = edges[ei];
        if let Some(rev) = by_key.get_mut(&(key(b), key(a)))
            && let Some(pos) = rev.iter().position(|&r| !gone[r])
        {
            let ri = rev.swap_remove(pos);
            gone[ei] = true;
            gone[ri] = true;
            let (x, y) = (find(&mut parent, pi), find(&mut parent, edges[ri].0));
            parent[x] = y;
        }
    }
    let root: Vec<usize> = (0..n).map(|i| find(&mut parent, i)).collect();

    // The outline's edges in steps of at most STEP (so it can be smoothed); the pieces get the
    // same points (so their tops follow it). Seams stay whole.
    let mut dense: Vec<Vec<Vec2>> = vec![Vec::new(); n];
    let mut oedges: Vec<(usize, Vec2, Vec2)> = Vec::new();
    for (ei, &(pi, a, b)) in edges.iter().enumerate() {
        dense[pi].push(a);
        if gone[ei] {
            continue;
        }
        let k = ((b - a).length() / STEP).ceil().max(1.0) as usize;
        let mut from = a;
        for s in 1..=k {
            let to = if s == k { b } else { a + (b - a) * (s as f32 / k as f32) };
            if s < k {
                dense[pi].push(to);
            }
            oedges.push((pi, from, to));
            from = to;
        }
    }

    // Outcrops: groups small enough and on their own.
    let mut bounds: HashMap<usize, (Vec2, Vec2, usize)> = HashMap::new();
    for (pi, poly) in polys.iter().enumerate() {
        let e = bounds.entry(root[pi]).or_insert((poly[0], poly[0], 0));
        for &v in poly {
            e.0 = e.0.min(v);
            e.1 = e.1.max(v);
        }
        e.2 += 1;
    }
    let is_outcrop = |pi: usize| {
        let (lo, hi, _) = bounds[&root[pi]];
        hi.x - lo.x < OUTCROP_U && hi.y - lo.y < OUTCROP_U
    };
    let outcrops: Vec<usize> = (0..walls_n).filter(|&pi| is_outcrop(pi)).collect();

    // The outline: what's left, chained into loops per group.
    let mut outs: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (ei, e) in oedges.iter().enumerate() {
        if !is_outcrop(e.0) {
            outs.entry(key(e.1)).or_default().push(ei);
        }
    }
    let mut used = vec![false; oedges.len()];
    let mut loops: Vec<(usize, Vec<Vec2>)> = Vec::new();
    for start in 0..oedges.len() {
        if used[start] || is_outcrop(oedges[start].0) {
            continue;
        }
        let mut lp = vec![oedges[start].1];
        used[start] = true;
        let mut cur = start;
        loop {
            let (_, a, b) = oedges[cur];
            if key(b) == key(lp[0]) {
                break;
            }
            lp.push(b);
            // Where two edges leave a corner (pieces touching diagonally), the sharper turn keeps
            // the loops apart.
            let dir = b - a;
            let next = outs.get(&key(b)).into_iter().flatten().copied().filter(|&e| !used[e]).max_by(|&x, &y| {
                let cx = cross(dir, oedges[x].2 - oedges[x].1);
                let cy = cross(dir, oedges[y].2 - oedges[y].1);
                cx.total_cmp(&cy)
            });
            let Some(next) = next else { break };
            used[next] = true;
            cur = next;
            if lp.len() > 200_000 {
                break;
            }
        }
        if lp.len() >= 3 {
            loops.push((root[oedges[start].0], lp));
        }
    }
    let loop_origs = loops.clone();

    // Smooth the outline (the tracer keeps corners on its tile lines sharp, so pieces meet):
    // a few passes toward the neighbors' middle, each point kept within SMOOTH_U of where the
    // simulation has it, so the cliff's foot stays on the walkable edge.
    for (_, lp) in loops.iter_mut() {
        let m = lp.len();
        let orig = lp.clone();
        for _ in 0..8 {
            let prev = lp.clone();
            for i in 0..m {
                let mid = (prev[(i + m - 1) % m] + prev[(i + 1) % m]) * 0.5;
                let p = prev[i] + (mid - prev[i]) * 0.5;
                let d = p - orig[i];
                lp[i] = if d.length() > SMOOTH_U { orig[i] + d.normalize_or_zero() * SMOOTH_U } else { p };
            }
        }
    }
    // Edges of the outline in buckets, for measuring a wall's thickness from a vertex.
    let mut seg_grid: HashMap<(i32, i32), Vec<(Vec2, Vec2)>> = HashMap::new();
    const SEG_CELL: f32 = 100.0;
    for (_, lp) in &loops {
        for i in 0..lp.len() {
            let (a, b) = (lp[i], lp[(i + 1) % lp.len()]);
            let (lo, hi) = (a.min(b), a.max(b));
            for cy in (lo.y / SEG_CELL).floor() as i32..=(hi.y / SEG_CELL).floor() as i32 {
                for cx in (lo.x / SEG_CELL).floor() as i32..=(hi.x / SEG_CELL).floor() as i32 {
                    seg_grid.entry((cx, cy)).or_default().push((a, b));
                }
            }
        }
    }
    let reach = PROFILE[PROFILE.len() - 1].0 / U / THICKNESS_SHARE * 1.5;
    let thickness = |p: Vec2, dir: Vec2| -> f32 {
        // Distance along `dir` (inward) to the first outline edge, up to `reach` (u).
        let mut best = reach;
        let q = p + dir * reach;
        let (lo, hi) = (p.min(q), p.max(q));
        for cy in (lo.y / SEG_CELL).floor() as i32..=(hi.y / SEG_CELL).floor() as i32 {
            for cx in (lo.x / SEG_CELL).floor() as i32..=(hi.x / SEG_CELL).floor() as i32 {
                for &(a, b) in seg_grid.get(&(cx, cy)).into_iter().flatten() {
                    if let Some(t) = ray_segment(p, dir, a, b)
                        && t > 0.5
                    {
                        best = best.min(t);
                    }
                }
            }
        }
        best
    };

    // Faces, and where each outline vertex's top lands.
    let h = height_m;
    let mut groups: HashMap<usize, Vec<[f32; 3]>> = HashMap::new();
    let mut top_at: HashMap<(i64, i64), [f32; 3]> = HashMap::new();
    for ((g, lp), (_, orig_lp)) in loops.iter().zip(loop_origs.iter()) {
        let m = lp.len();
        let normal = |i: usize| {
            let d = lp[(i + 1) % m] - lp[i];
            Vec2::new(-d.y, d.x).normalize_or_zero()
        };
        let mut rings: Vec<Vec<[f32; 3]>> = vec![Vec::with_capacity(m); PROFILE.len()];
        for i in 0..m {
            let (n0, n1) = (normal((i + m - 1) % m), normal(i));
            let s = (n0 + n1).normalize_or_zero();
            let inward = if s == Vec2::ZERO { n1 } else { s * (1.0 / s.dot(n1).max(0.5)) };
            let dir = inward.normalize_or_zero();
            let room = thickness(lp[i], dir) * THICKNESS_SHARE * U;
            let p = lp[i];
            for (k, &(d, share)) in PROFILE.iter().enumerate() {
                let j = if k > 0 { jitter(p * U, k) } else { 0.0 };
                let step = ((d + j * 0.08).max(0.0) * inward.length()).min(room);
                let at = p * U + dir * step;
                let y = h * share + if k == 1 || k == 2 { j * 0.12 } else { 0.0 };
                rings[k].push([at.x, y.min(h), at.y]);
            }
            top_at.insert(key(orig_lp[i]), rings[PROFILE.len() - 1][i]);
        }
        let tris = groups.entry(*g).or_default();
        for k in 1..PROFILE.len() {
            for i in 0..m {
                let i2 = (i + 1) % m;
                let (a, b, c, d) = (rings[k - 1][i], rings[k - 1][i2], rings[k][i2], rings[k][i]);
                // Out of the rock (to the left of travel): front faces wind clockwise.
                tris.extend([a, b, c, a, c, d]);
            }
        }
    }

    // Tops: each piece's own triangulation, outline vertices moved to the top of the face.
    for (pi, poly) in dense.iter().enumerate() {
        if is_outcrop(pi) || poly.len() < 3 {
            continue;
        }
        let tris = groups.entry(root[pi]).or_default();
        let place = |v: Vec2| top_at.get(&key(v)).copied().map(|t| [t[0], h, t[2]]).unwrap_or([v.x * U, h, v.y * U]);
        // Triangulated where the points end up, so a top never folds over where they moved.
        let placed: Vec<[f32; 3]> = poly.iter().map(|&v| place(v)).collect();
        let mut flat: Vec<Vec2> = placed.iter().map(|p| Vec2::new(p[0], p[2])).collect();
        let flip = signed_area(&flat) < 0.0;
        if flip {
            flat.reverse();
        }
        let at = |i: usize| placed[if flip { placed.len() - 1 - i } else { i }];
        for [a, b, c] in triangulate(&flat) {
            tris.extend([at(a), at(b), at(c)]);
        }
    }

    let mut keys: Vec<usize> = groups.keys().copied().collect();
    keys.sort_unstable();
    Cliffs { groups: keys.into_iter().map(|k| groups.remove(&k).unwrap_or_default()).collect(), outcrops }
}

/// The 2D cross product (z of a × b).
fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

fn signed_area(p: &[Vec2]) -> f32 {
    (0..p.len()).map(|i| cross(p[i], p[(i + 1) % p.len()])).sum::<f32>() * 0.5
}

/// Where the ray `p + dir·t` crosses the segment `a`–`b`, as `t`.
fn ray_segment(p: Vec2, dir: Vec2, a: Vec2, b: Vec2) -> Option<f32> {
    let e = b - a;
    let den = cross(dir, e);
    if den.abs() < 1e-9 {
        return None;
    }
    let ap = a - p;
    let t = cross(ap, e) / den;
    let s = cross(ap, dir) / den;
    (t >= 0.0 && (0.0..=1.0).contains(&s)).then_some(t)
}

/// A repeatable wobble in [-1, 1] for a point (meters) and ring.
fn jitter(p: Vec2, k: usize) -> f32 {
    let h = ((p.x as f64) * 12.9898 + (p.y as f64) * 78.233 + k as f64 * 37.719).sin() * 43758.5453;
    ((h - h.floor()) * 2.0 - 1.0) as f32
}

/// Ear clipping for a polygon of positive area (y down), tolerant of what tracing makes: an
/// outline that touches itself at a corner, collinear runs. When no clean ear is left (a
/// degenerate remainder), the most convex corner is clipped anyway, so it always finishes.
pub fn triangulate(poly: &[Vec2]) -> Vec<[usize; 3]> {
    let mut idx: Vec<usize> = (0..poly.len()).collect();
    let mut out = Vec::with_capacity(poly.len().saturating_sub(2));
    let turn_at = |a: Vec2, b: Vec2, c: Vec2| cross(b - a, c - b);
    let inside = |p: Vec2, a: Vec2, b: Vec2, c: Vec2| {
        let (d1, d2, d3) = (cross(b - a, p - a), cross(c - b, p - b), cross(a - c, p - c));
        d1 >= -1e-6 && d2 >= -1e-6 && d3 >= -1e-6
    };
    let mut guard = 0usize;
    while idx.len() > 3 {
        let m = idx.len();
        let mut clipped = false;
        let mut best = (f32::NEG_INFINITY, 0usize);
        for i in 0..m {
            let (ia, ib, ic) = (idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]);
            let (a, b, c) = (poly[ia], poly[ib], poly[ic]);
            let turn = turn_at(a, b, c);
            if turn > best.0 {
                best = (turn, i);
            }
            if turn <= 1e-6 {
                continue;
            }
            let blocked = idx.iter().any(|&j| {
                j != ia && j != ib && j != ic && {
                    let q = poly[j];
                    key(q) != key(a) && key(q) != key(b) && key(q) != key(c) && inside(q, a, b, c)
                }
            });
            if !blocked {
                out.push([ia, ib, ic]);
                idx.remove(i);
                clipped = true;
                break;
            }
        }
        if !clipped {
            let i = best.1;
            let m = idx.len();
            if best.0 > 1e-6 {
                out.push([idx[(i + m - 1) % m], idx[i], idx[(i + 1) % m]]);
            }
            idx.remove(i);
        }
        guard += 1;
        if guard > poly.len() * 4 {
            break;
        }
    }
    if idx.len() == 3 && turn_at(poly[idx[0]], poly[idx[1]], poly[idx[2]]) > 1e-6 {
        out.push([idx[0], idx[1], idx[2]]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::map::MapId;

    fn tri_area(t: &[[f32; 3]]) -> f32 {
        t.chunks(3)
            .map(|c| {
                ((c[1][0] - c[0][0]) * (c[2][2] - c[0][2]) - (c[1][2] - c[0][2]) * (c[2][0] - c[0][0])).abs() * 0.5
            })
            .sum()
    }

    #[test]
    fn ear_clipping_covers_a_concave_polygon() {
        // An L, positive area (y down).
        let l: Vec<Vec2> = [(0.0, 0.0), (2.0, 0.0), (2.0, 1.0), (1.0, 1.0), (1.0, 2.0), (0.0, 2.0)]
            .iter()
            .map(|&(x, y)| Vec2::new(x, y))
            .collect();
        let l: Vec<Vec2> = if signed_area(&l) < 0.0 { l.into_iter().rev().collect() } else { l };
        let t = triangulate(&l);
        assert_eq!(t.len(), 4);
        let area: f32 = t.iter().map(|&[a, b, c]| cross(l[b] - l[a], l[c] - l[a]).abs() * 0.5).sum();
        assert!((area - 3.0).abs() < 1e-4, "{area}");
    }

    #[test]
    fn crossroads_cliffs_are_whole() {
        let map = MapId::Crossroads.shared();
        let c = build(&map, 1.4);
        assert!(!c.groups.is_empty());
        // The tops cover the walls and the frame past the map's edges: their area, give or take
        // the faces' step in (the tops are what's flat at full height).
        let walls: f32 = map
            .walls
            .iter()
            .enumerate()
            .filter(|(i, _)| !c.outcrops.contains(i))
            .map(|(_, w)| signed_area(w).abs() * U * U)
            .sum::<f32>()
            + ((map.size.x + 2.0 * MARGIN_U) * (map.size.y + 2.0 * MARGIN_U) - map.size.x * map.size.y) * U * U;
        let tops: f32 = c
            .groups
            .iter()
            .flat_map(|g| g.chunks(3))
            .filter(|t| t.iter().all(|v| (v[1] - 1.4).abs() < 1e-4))
            .map(tri_area)
            .sum();
        assert!(tops > walls * 0.9 && tops < walls * 1.01, "tops {tops} walls {walls}");
        // No top folds over: every flat triangle faces up (clockwise from above, the faces' way).
        let flipped = c
            .groups
            .iter()
            .flat_map(|g| g.chunks(3))
            .filter(|t| t.iter().all(|v| (v[1] - 1.4).abs() < 1e-4))
            .filter(|t| (t[1][0] - t[0][0]) * (t[2][2] - t[0][2]) - (t[1][2] - t[0][2]) * (t[2][0] - t[0][0]) < -1e-6)
            .count();
        assert_eq!(flipped, 0);
        // A long cliff's pieces are never outcrops: only small groups on their own are.
        for &i in &c.outcrops {
            let w = &map.walls[i];
            let (lo, hi) = w.iter().fold((w[0], w[0]), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
            assert!(hi.x - lo.x < OUTCROP_U && hi.y - lo.y < OUTCROP_U);
        }
        let pieces_small = map
            .walls
            .iter()
            .filter(|w| {
                let (lo, hi) = w.iter().fold((w[0], w[0]), |(lo, hi), v| (lo.min(*v), hi.max(*v)));
                hi.x - lo.x < OUTCROP_U && hi.y - lo.y < OUTCROP_U
            })
            .count();
        assert!(c.outcrops.len() < pieces_small, "some small pieces belong to cliffs");
    }
}
