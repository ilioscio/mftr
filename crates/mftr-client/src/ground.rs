//! The ground control map (15 §3): what the ground is made of, per map, so every map the editor
//! makes is painted like an official one. One RGBA texel per [`TEXEL_U`] game units over the
//! map, read by the ground shader (which breaks every boundary with its own noise):
//!
//! - **R, dirt:** a band down every lane, the middle of the jungle paths, the camps' and
//!   objectives' clearings.
//! - **G, paving:** plinths under the structures and relic pads, the fountains' plazas.
//! - **B, water:** the river.
//! - **A, signed wall distance:** 0.5 on a wall's edge; above it, how far open ground is from
//!   the nearest wall (the cliffs' foot occlusion and the jungle's gloom); below it, how far a
//!   cliff's top is from its edge (the grassy lip, the forest's gloom), each saturating at
//!   [`WALL_REACH_U`]. See [`signed_alpha`].
//!
//! Built from the map's walls, strokes and placements only (`Layout::strokes`), in well under a
//! frame for the largest map.

use mftr_sim::map::{Map, StrokeKind};
use mftr_sim::{UnitKind, Vec2};

/// Game units per texel (the pathing grid's cell).
pub const TEXEL_U: f32 = 25.0;
/// Wall distance saturates here (A = 255 out in the open, 0 deep inside a wall).
pub const WALL_REACH_U: f32 = 1000.0;

pub struct GroundControl {
    pub width: usize,
    pub height: usize,
    /// RGBA8, row-major from the map's origin (+x right, +y down).
    pub rgba: Vec<u8>,
}

/// A plinth's radius under a structure (u): wider than the structure's footprint, so the paving
/// rings it (15 §5).
pub fn plinth_radius(kind: UnitKind) -> Option<f32> {
    match kind {
        UnitKind::Turret => Some(330.0),
        UnitKind::Gatehouse => Some(460.0),
        UnitKind::Base => Some(820.0),
        UnitKind::Relic => Some(210.0),
        _ => None,
    }
}

pub fn control_map(map: &Map) -> GroundControl {
    let w = (map.size.x / TEXEL_U).ceil().max(1.0) as usize;
    let h = (map.size.y / TEXEL_U).ceil().max(1.0) as usize;
    let blocked = rasterize_walls(map, w, h);
    let dist = wall_distance(&blocked, w, h);
    let open: Vec<bool> = blocked.iter().map(|b| !b).collect();
    let depth = wall_distance(&open, w, h);
    let mut dirt = vec![0.0f32; w * h];
    let mut paving = vec![0.0f32; w * h];
    let mut water = vec![0.0f32; w * h];
    let smooth = |e0: f32, e1: f32, x: f32| {
        let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    };
    for s in &map.layout.strokes {
        let hw = s.width * 0.5;
        // (the channel, where it's full, where it fades out)
        let (layer, inner, outer) = match s.kind {
            StrokeKind::Lane => (&mut dirt, (hw * 0.6).min(560.0), (hw * 0.9).min(760.0)),
            StrokeKind::Jungle => (&mut dirt, hw * 0.3, hw * 0.75),
            StrokeKind::Clearing => (&mut dirt, hw * 0.45, hw * 0.85),
            StrokeKind::River => (&mut water, hw * 0.7, hw * 0.98),
            StrokeKind::Grass => continue,
        };
        paint(layer, w, h, &s.points, outer, |d| 1.0 - smooth(inner, outer, d));
    }
    let mut plinths: Vec<(Vec2, f32)> =
        map.layout.placements.iter().filter_map(|p| plinth_radius(p.kind).map(|r| (p.pos, r))).collect();
    plinths.extend(map.layout.fountains.iter().flatten().map(|&(c, r)| (c, r * 1.05)));
    for (c, r) in plinths {
        paint(&mut paving, w, h, &[c], r, |d| 1.0 - smooth(r * 0.82, r, d));
    }
    let mut rgba = vec![0u8; w * h * 4];
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    for i in 0..w * h {
        rgba[i * 4] = byte(dirt[i]);
        rgba[i * 4 + 1] = byte(paving[i]);
        rgba[i * 4 + 2] = byte(water[i]);
        rgba[i * 4 + 3] = signed_alpha(if blocked[i] { -depth[i] } else { dist[i] });
    }
    GroundControl { width: w, height: h, rgba }
}

/// The alpha for a signed wall distance (u; negative inside a wall): 128..=255 out in the open,
/// 0..=127 inside, so a shader's linear filtering crosses a wall's edge smoothly at 0.5.
pub fn signed_alpha(d: f32) -> u8 {
    let t = (d.abs() / WALL_REACH_U).min(1.0);
    if d > 0.0 { 128 + (t * 127.0).round() as u8 } else { 127 - (t * 127.0).round() as u8 }
}

/// Max-blend `f(distance)` into `layer` for every texel within `reach` of the polyline `pts` (a
/// single point: a circle).
fn paint(layer: &mut [f32], w: usize, h: usize, pts: &[Vec2], reach: f32, f: impl Fn(f32) -> f32) {
    if pts.is_empty() {
        return;
    }
    let (mut lo, mut hi) = (pts[0], pts[0]);
    for p in pts {
        lo = lo.min(*p);
        hi = hi.max(*p);
    }
    let x0 = ((lo.x - reach) / TEXEL_U).floor().max(0.0) as usize;
    let y0 = ((lo.y - reach) / TEXEL_U).floor().max(0.0) as usize;
    let x1 = (((hi.x + reach) / TEXEL_U).ceil().max(0.0) as usize).min(w);
    let y1 = (((hi.y + reach) / TEXEL_U).ceil().max(0.0) as usize).min(h);
    for y in y0..y1 {
        for x in x0..x1 {
            let p = Vec2::new((x as f32 + 0.5) * TEXEL_U, (y as f32 + 0.5) * TEXEL_U);
            let d = if pts.len() == 1 {
                p.distance(pts[0])
            } else {
                pts.windows(2).map(|s| point_segment(p, s[0], s[1])).fold(f32::INFINITY, f32::min)
            };
            if d <= reach {
                let v = f(d);
                let cell = &mut layer[y * w + x];
                *cell = cell.max(v);
            }
        }
    }
}

fn point_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_sq().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// Which texel centers lie inside a wall (scanline, even-odd per polygon).
fn rasterize_walls(map: &Map, w: usize, h: usize) -> Vec<bool> {
    let mut blocked = vec![false; w * h];
    let mut xs: Vec<f32> = Vec::new();
    for poly in &map.walls {
        let n = poly.len();
        if n < 3 {
            continue;
        }
        let (mut ymin, mut ymax) = (f32::INFINITY, f32::NEG_INFINITY);
        for p in poly {
            ymin = ymin.min(p.y);
            ymax = ymax.max(p.y);
        }
        let r0 = ((ymin / TEXEL_U) - 0.5).floor().max(0.0) as usize;
        let r1 = (((ymax / TEXEL_U) - 0.5).ceil().max(0.0) as usize + 1).min(h);
        for row in r0..r1 {
            let cy = (row as f32 + 0.5) * TEXEL_U;
            xs.clear();
            for i in 0..n {
                let (a, b) = (poly[i], poly[(i + 1) % n]);
                if (a.y <= cy) != (b.y <= cy) {
                    xs.push(a.x + (cy - a.y) / (b.y - a.y) * (b.x - a.x));
                }
            }
            xs.sort_by(f32::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                // Texel centers strictly between the crossings.
                let c0 = ((pair[0] / TEXEL_U) - 0.5).ceil().max(0.0) as usize;
                let c1 = (((pair[1] / TEXEL_U) - 0.5).floor() + 1.0).clamp(0.0, w as f32) as usize;
                for c in c0..c1.max(c0) {
                    blocked[row * w + c] = true;
                }
            }
        }
    }
    blocked
}

/// Distance (game units) from each open texel to the nearest blocked one; 0 for blocked texels
/// (and [`WALL_REACH_U`] where nothing is blocked).
/// A two-pass chamfer (3-4) transform: within a few percent of Euclidean, which a shader's
/// shading can't tell apart.
fn wall_distance(blocked: &[bool], w: usize, h: usize) -> Vec<f32> {
    const FAR: u32 = u32::MAX / 4;
    let mut d: Vec<u32> = blocked.iter().map(|&b| if b { 0 } else { FAR }).collect();
    let at = |d: &[u32], x: isize, y: isize| -> u32 {
        if x < 0 || y < 0 || x >= w as isize || y >= h as isize { FAR } else { d[y as usize * w + x as usize] }
    };
    for y in 0..h as isize {
        for x in 0..w as isize {
            let i = y as usize * w + x as usize;
            let v = d[i]
                .min(at(&d, x - 1, y) + 3)
                .min(at(&d, x, y - 1) + 3)
                .min(at(&d, x - 1, y - 1) + 4)
                .min(at(&d, x + 1, y - 1) + 4);
            d[i] = v;
        }
    }
    for y in (0..h as isize).rev() {
        for x in (0..w as isize).rev() {
            let i = y as usize * w + x as usize;
            let v = d[i]
                .min(at(&d, x + 1, y) + 3)
                .min(at(&d, x, y + 1) + 3)
                .min(at(&d, x + 1, y + 1) + 4)
                .min(at(&d, x - 1, y + 1) + 4);
            d[i] = v;
        }
    }
    d.into_iter().map(|v| if v >= FAR { WALL_REACH_U } else { v as f32 / 3.0 * TEXEL_U }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::map::MapId;

    #[test]
    fn crossroads_paints_lanes_paving_water_and_wall_distance() {
        let map = MapId::Crossroads.shared();
        let g = control_map(&map);
        let px = |p: Vec2| {
            let (x, y) = ((p.x / TEXEL_U) as usize, (p.y / TEXEL_U) as usize);
            let i = (y * g.width + x) * 4;
            [g.rgba[i], g.rgba[i + 1], g.rgba[i + 2], g.rgba[i + 3]]
        };
        let l = &map.layout;
        // A lane's middle is dirt; a structure's center is paved; the river's course is water.
        let lane_mid = l.lanes[1][0][l.lanes[1][0].len() / 2];
        assert!(px(lane_mid)[0] > 200, "{:?}", px(lane_mid));
        let turret = l.placements.iter().find(|p| p.kind == UnitKind::Turret).unwrap().pos;
        assert!(px(turret)[1] > 200);
        let river_mid = l.river[l.river.len() / 4];
        assert!(px(river_mid)[2] > 200, "{:?}", px(river_mid));
        // Inside a wall the alpha is under the half; open ground is over it.
        let wall_p = map.walls[0][0] + (map.walls[0][1] - map.walls[0][0]) * 0.5;
        let probe = [Vec2::new(10.0, 10.0), wall_p];
        assert!(probe.iter().any(|p| px(*p)[3] < 128));
        assert!(px(lane_mid)[3] > 128);
        assert_eq!(g.rgba.len(), g.width * g.height * 4);
    }

    #[test]
    fn signed_alpha_crosses_the_half_at_a_walls_edge() {
        assert_eq!(signed_alpha(WALL_REACH_U * 2.0), 255);
        assert_eq!(signed_alpha(-WALL_REACH_U * 2.0), 0);
        assert!(signed_alpha(25.0) > 128 && signed_alpha(-25.0) < 128);
        assert!(signed_alpha(-25.0) > signed_alpha(-500.0));
    }

    #[test]
    fn a_map_without_strokes_is_grass_with_wall_distance() {
        let map = MapId::Arena.shared();
        let g = control_map(&map);
        assert!(g.rgba.chunks(4).all(|c| c[0] == 0 && c[1] == 0 && c[2] == 0));
    }
}
