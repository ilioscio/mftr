//! Map specs (12, D56): a lane map as a map maker draws it. Lines with widths (lanes, the
//! river, jungle paths, tall grass), clearings, camps and great-objective points, on blue's half
//! of a square map (below the river line, the diagonal from the top-left corner to the bottom
//! right). Red's half is blue's mirrored across the river. Everything not drawn is wall.
//!
//! Building a map from a spec (`build`) is deterministic, so a server can send just the spec and
//! every client builds the same map:
//! 1. The lines become capsules on a grid of `CELL` cells, their edges gently wavy.
//! 2. The grid is mirrored across the river; ground cut off from the base becomes wall.
//! 3. Wall outlines are traced tile by tile (each outline a simple polygon, as the simulation
//!    needs), smoothed and simplified.

use crate::jungle::{Camp, CampKind};
use crate::lane::{BASE_TURRET_TIER, GATEHOUSE_TIER, Pacing};
use crate::map::{Layout, Map, MapId, Placement};
use crate::math::Vec2;
use crate::world::{Team, UnitKind};

/// Grid cell of the build.
pub const CELL: f32 = 25.0;
/// Wall outlines are traced in square tiles of this many cells.
const TILE: usize = 32;
/// The sizes a map can have (square, game units).
pub const SIZES: [f32; 3] = [10_000.0, 14_500.0, 16_000.0];
pub const MAX_LANES: usize = 5;
/// The base: blue's corner square, this share of the map's side.
pub const BASE_SHARE: f32 = 0.27;
/// Clearings carved around camps and great objectives.
pub const BUFF_CLEARING: f32 = 520.0;
pub const CAMP_CLEARING: f32 = 460.0;
pub const OBJECTIVE_CLEARING: f32 = 700.0;
/// A lane ends this close to the river line (and is joined to its mirror there).
pub const LANE_END_SLACK: f32 = 450.0;
/// Rules: camps on a side, jungle ground as a share of a half.
pub const MIN_CAMPS: usize = 3;
pub const MIN_JUNGLE_SHARE: f32 = 0.05;
/// Default structure spots along a lane, as shares of its length from the base: the Gatehouse,
/// the gatehouse turret, the inner turret, the outer turret.
pub const STRUCTURE_SHARES: [f32; 4] = [0.08, 0.16, 0.45, 0.78];

/// A line with a width (a lane, river, jungle path or tall grass stroke).
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub width: f32,
    pub points: Vec<Vec2>,
}

/// A map as drawn: blue's half (map coordinates, y down; blue's half has y > x).
#[derive(Clone, Debug, PartialEq)]
pub struct MapSpec {
    pub name: String,
    pub size: f32,
    /// From the base to the river line, in the order lanes are numbered (top, mid, bottom…).
    pub lanes: Vec<Line>,
    pub river: Vec<Line>,
    pub jungle: Vec<Line>,
    pub grass: Vec<Line>,
    /// Open ground: center and radius.
    pub clearings: Vec<(Vec2, f32)>,
    pub camps: Vec<(CampKind, Vec2)>,
    /// Great objectives (both of a pair, in the river).
    pub objectives: Vec<Vec2>,
    /// Structures moved along their lane: (lane, tier 1–4, distance from the lane's start).
    pub structures: Vec<(u8, u8, f32)>,
}

impl MapSpec {
    pub fn new(name: &str, size: f32) -> Self {
        MapSpec {
            name: name.to_string(),
            size,
            lanes: Vec::new(),
            river: Vec::new(),
            jungle: Vec::new(),
            grass: Vec::new(),
            clearings: Vec::new(),
            camps: Vec::new(),
            objectives: Vec::new(),
            structures: Vec::new(),
        }
    }

    /// The base square's side.
    pub fn base_side(&self) -> f32 {
        self.size * BASE_SHARE
    }

    /// Red's image of a point: mirrored across the river.
    pub fn mirror(p: Vec2) -> Vec2 {
        Vec2::new(p.y, p.x)
    }

    /// A point on blue's half (or on the river line).
    pub fn on_blue_half(p: Vec2) -> bool {
        p.y >= p.x
    }

    /// Blue's corner coordinates (`dx` right, `dy` up from blue's corner) on the map.
    pub fn corner(&self, dx: f32, dy: f32) -> Vec2 {
        Vec2::new(dx, self.size - dy)
    }

    // ---- text ---------------------------------------------------------------------------

    /// The spec as text (`*.mftrmap`): one item a line.
    pub fn to_text(&self) -> String {
        let pt = |p: Vec2| format!("{},{}", p.x.round() as i64, p.y.round() as i64);
        let line = |kind: &str, l: &Line| {
            let pts: Vec<String> = l.points.iter().map(|p| pt(*p)).collect();
            format!("{kind} {} {}\n", l.width.round() as i64, pts.join(" "))
        };
        let mut s = format!("mftr-map 1\nname {}\nsize {}\n", self.name, self.size.round() as i64);
        for l in &self.lanes {
            s += &line("lane", l);
        }
        for l in &self.river {
            s += &line("river", l);
        }
        for l in &self.jungle {
            s += &line("jungle", l);
        }
        for l in &self.grass {
            s += &line("grass", l);
        }
        for (c, r) in &self.clearings {
            s += &format!("clearing {} {}\n", pt(*c), r.round() as i64);
        }
        for (k, p) in &self.camps {
            s += &format!("camp {} {}\n", camp_key(*k), pt(*p));
        }
        for p in &self.objectives {
            s += &format!("objective {}\n", pt(*p));
        }
        for (lane, tier, d) in &self.structures {
            s += &format!("structure {lane} {tier} {}\n", d.round() as i64);
        }
        s
    }

    /// Read a spec from its text. Unknown lines are errors (with their line number).
    pub fn parse(text: &str) -> Result<MapSpec, String> {
        let mut spec = MapSpec::new("Untitled", SIZES[1]);
        let mut header = false;
        for (n, raw) in text.lines().enumerate() {
            let at = |e: &str| format!("line {}: {e}", n + 1);
            let raw = raw.trim();
            if raw.is_empty() || raw.starts_with('#') {
                continue;
            }
            let mut words = raw.split_whitespace();
            let key = words.next().unwrap_or("");
            let rest: Vec<&str> = words.collect();
            let num = |s: &str| s.parse::<f32>().map_err(|_| at(&format!("not a number: {s}")));
            let point = |s: &str| -> Result<Vec2, String> {
                let (x, y) = s.split_once(',').ok_or_else(|| at(&format!("not a point: {s}")))?;
                Ok(Vec2::new(num(x)?, num(y)?))
            };
            let line = || -> Result<Line, String> {
                let (w, pts) = rest.split_first().ok_or_else(|| at("a width and points"))?;
                let points = pts.iter().map(|p| point(p)).collect::<Result<Vec<_>, _>>()?;
                if points.len() < 2 {
                    return Err(at("a line needs at least two points"));
                }
                Ok(Line { width: num(w)?, points })
            };
            match key {
                "mftr-map" => {
                    if rest.first() != Some(&"1") {
                        return Err(at("unknown format version"));
                    }
                    header = true;
                }
                "name" => spec.name = rest.join(" "),
                "size" => spec.size = num(rest.first().ok_or_else(|| at("a size"))?)?,
                "lane" => spec.lanes.push(line()?),
                "river" => spec.river.push(line()?),
                "jungle" => spec.jungle.push(line()?),
                "grass" => spec.grass.push(line()?),
                "clearing" if rest.len() == 2 => spec.clearings.push((point(rest[0])?, num(rest[1])?)),
                "camp" if rest.len() == 2 => {
                    let kind = camp_from_key(rest[0]).ok_or_else(|| at(&format!("unknown camp: {}", rest[0])))?;
                    spec.camps.push((kind, point(rest[1])?));
                }
                "objective" if rest.len() == 1 => spec.objectives.push(point(rest[0])?),
                "structure" if rest.len() == 3 => {
                    let (lane, tier) = (num(rest[0])? as u8, num(rest[1])? as u8);
                    spec.structures.push((lane, tier, num(rest[2])?));
                }
                _ => return Err(at(&format!("can't read: {raw}"))),
            }
        }
        if !header {
            return Err("not a map file (no `mftr-map 1` line)".into());
        }
        Ok(spec)
    }

    // ---- geometry -----------------------------------------------------------------------

    /// Each lane's full path for blue, base to red's Base: blue's half, then red's half
    /// backwards (joined on the river line).
    pub fn lane_path(&self, lane: usize) -> Vec<Vec2> {
        let Some(l) = self.lanes.get(lane) else { return Vec::new() };
        let mut path = l.points.clone();
        let last = *path.last().unwrap_or(&Vec2::ZERO);
        // Onto the river line.
        let on_axis = Vec2::new((last.x + last.y) / 2.0, (last.x + last.y) / 2.0);
        if on_axis.distance(last) > 1.0 {
            path.push(on_axis);
        }
        let back: Vec<Vec2> = l.points.iter().rev().map(|p| MapSpec::mirror(*p)).collect();
        path.extend(back.into_iter().filter(|p| p.distance(on_axis) > 1.0));
        path
    }

    /// Where structure `tier` (1–4) of `lane` stands on blue's half.
    pub fn structure_spot(&self, lane: usize, tier: u8) -> Option<Vec2> {
        let l = self.lanes.get(lane)?;
        let length = polyline_length(&l.points);
        let d = self
            .structures
            .iter()
            .find(|(sl, st, _)| *sl as usize == lane && *st == tier)
            .map(|(_, _, d)| *d)
            .unwrap_or(STRUCTURE_SHARES[(4 - tier.clamp(1, 4)) as usize] * length);
        Some(point_along(&l.points, d.clamp(0.0, length)))
    }

    /// The clearing every camp and objective carves (center, radius), drawn ones included.
    pub fn all_clearings(&self) -> Vec<(Vec2, f32)> {
        let mut out = self.clearings.clone();
        for (k, p) in &self.camps {
            let r = if matches!(k, CampKind::Warden | CampKind::Brute) { BUFF_CLEARING } else { CAMP_CLEARING };
            out.push((*p, r));
        }
        out.extend(self.objectives.iter().map(|p| (*p, OBJECTIVE_CLEARING)));
        out
    }
}

pub fn camp_key(k: CampKind) -> &'static str {
    match k {
        CampKind::Warden => "warden",
        CampKind::Brute => "brute",
        CampKind::Hounds => "hounds",
        CampKind::Toad => "toad",
        CampKind::Ravens => "ravens",
        CampKind::Crawlers => "crawlers",
    }
}

pub fn camp_from_key(s: &str) -> Option<CampKind> {
    Some(match s {
        "warden" => CampKind::Warden,
        "brute" => CampKind::Brute,
        "hounds" => CampKind::Hounds,
        "toad" => CampKind::Toad,
        "ravens" => CampKind::Ravens,
        "crawlers" => CampKind::Crawlers,
        _ => return None,
    })
}

pub fn polyline_length(pts: &[Vec2]) -> f32 {
    pts.windows(2).map(|w| w[0].distance(w[1])).sum()
}

/// The point `d` along a polyline.
pub fn point_along(pts: &[Vec2], d: f32) -> Vec2 {
    let mut left = d;
    for w in pts.windows(2) {
        let len = w[0].distance(w[1]);
        if left <= len && len > 0.0 {
            return w[0] + (w[1] - w[0]) * (left / len);
        }
        left -= len;
    }
    *pts.last().unwrap_or(&Vec2::ZERO)
}

// ---- the grid -----------------------------------------------------------------------------

/// The built grid: what's open ground, and what's tall grass.
pub struct Grid {
    pub n: usize,
    pub open: Vec<bool>,
    pub grass: Vec<bool>,
    /// Open cells drawn but cut off from the base (validation).
    pub cut_off: usize,
    /// Jungle cells (paths, clearings, camps) on blue's half (validation).
    pub jungle_cells: usize,
}

/// Smooth noise in [-1, 1] along a line (`t` in lengths of a wave), from a seed.
fn wave(t: f32, seed: u32) -> f32 {
    let hash = |i: i64| {
        let mut x = (i as u64 ^ (seed as u64).wrapping_mul(0x9E37_79B9)).wrapping_mul(0x2545_F491_4F6C_DD1D);
        x ^= x >> 29;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 32;
        (x & 0xFFFF) as f32 / 32767.5 - 1.0
    };
    let i = t.floor();
    let f = t - i;
    let s = f * f * (3.0 - 2.0 * f);
    let (a, b) = (hash(i as i64), hash(i as i64 + 1));
    a + (b - a) * s
}

/// Paint a line's capsules into `cells`: its width wobbles by `wobble` (a share) along it.
fn paint_line(cells: &mut [bool], n: usize, l: &Line, wobble: f32, seed: u32) {
    let mut walked = 0.0;
    for w in l.points.windows(2) {
        let (a, b) = (w[0], w[1]);
        let seg = b - a;
        let len = seg.length();
        let reach = l.width * 0.5 * (1.0 + wobble) + CELL;
        let (lo, hi) = (a.min(b) - Vec2::new(reach, reach), a.max(b) + Vec2::new(reach, reach));
        let (c0, c1) = (((lo.x / CELL).floor().max(0.0)) as usize, ((hi.x / CELL).ceil() as usize).min(n));
        let (r0, r1) = (((lo.y / CELL).floor().max(0.0)) as usize, ((hi.y / CELL).ceil() as usize).min(n));
        for r in r0..r1 {
            for c in c0..c1 {
                let p = Vec2::new((c as f32 + 0.5) * CELL, (r as f32 + 0.5) * CELL);
                let t = if len > 0.0 { ((p - a).dot(seg) / (len * len)).clamp(0.0, 1.0) } else { 0.0 };
                let d = p.distance(a + seg * t);
                let half = l.width * 0.5 * (1.0 + wobble * wave((walked + t * len) / 420.0, seed));
                if d <= half {
                    cells[r * n + c] = true;
                }
            }
        }
        walked += len;
    }
}

fn paint_circle(cells: &mut [bool], n: usize, c: Vec2, radius: f32) {
    let (c0, c1) =
        ((((c.x - radius) / CELL).floor().max(0.0)) as usize, (((c.x + radius) / CELL).ceil() as usize).min(n));
    let (r0, r1) =
        ((((c.y - radius) / CELL).floor().max(0.0)) as usize, (((c.y + radius) / CELL).ceil() as usize).min(n));
    for r in r0..r1 {
        for col in c0..c1 {
            let p = Vec2::new((col as f32 + 0.5) * CELL, (r as f32 + 0.5) * CELL);
            if p.distance(c) <= radius {
                cells[r * n + col] = true;
            }
        }
    }
}

/// Union each cell with its mirror across the river (the grid's diagonal).
fn mirror_cells(cells: &mut [bool], n: usize) {
    for r in 0..n {
        for c in 0..r {
            let v = cells[r * n + c] || cells[c * n + r];
            cells[r * n + c] = v;
            cells[c * n + r] = v;
        }
    }
}

impl Grid {
    pub fn build(spec: &MapSpec) -> Grid {
        let n = (spec.size / CELL).ceil() as usize;
        let mut open = vec![false; n * n];
        // The base.
        let b = spec.base_side();
        paint_rect(&mut open, n, Vec2::new(0.0, spec.size - b), Vec2::new(b, spec.size));
        for (i, l) in spec.lanes.iter().enumerate() {
            paint_line(&mut open, n, l, 0.05, 100 + i as u32);
        }
        for (i, l) in spec.river.iter().enumerate() {
            paint_line(&mut open, n, l, 0.18, 200 + i as u32);
        }
        let mut jungle = vec![false; n * n];
        for (i, l) in spec.jungle.iter().enumerate() {
            paint_line(&mut jungle, n, l, 0.16, 300 + i as u32);
        }
        for (c, r) in spec.all_clearings() {
            paint_circle(&mut jungle, n, c, r);
        }
        let jungle_cells = (0..n * n).filter(|i| jungle[*i] && !open[*i] && i / n > i % n).count();
        for (o, j) in open.iter_mut().zip(&jungle) {
            *o |= *j;
        }
        mirror_cells(&mut open, n);
        // Open ground cut off from the base is wall.
        let start = {
            let f = spec.corner(600.0, 600.0);
            ((f.y / CELL) as usize).min(n - 1) * n + ((f.x / CELL) as usize).min(n - 1)
        };
        let mut reached = vec![false; n * n];
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            if reached[i] || !open[i] {
                continue;
            }
            reached[i] = true;
            let (r, c) = (i / n, i % n);
            if r > 0 {
                stack.push(i - n);
            }
            if r + 1 < n {
                stack.push(i + n);
            }
            if c > 0 {
                stack.push(i - 1);
            }
            if c + 1 < n {
                stack.push(i + 1);
            }
        }
        let cut_off = (0..n * n).filter(|i| open[*i] && !reached[*i]).count();
        let open = reached;
        let mut grass = vec![false; n * n];
        for (i, l) in spec.grass.iter().enumerate() {
            paint_line(&mut grass, n, l, 0.2, 400 + i as u32);
        }
        mirror_cells(&mut grass, n);
        for (g, o) in grass.iter_mut().zip(&open) {
            *g &= *o;
        }
        Grid { n, open, grass, cut_off, jungle_cells }
    }

    pub fn open_at(&self, p: Vec2) -> bool {
        let (c, r) = ((p.x / CELL).floor(), (p.y / CELL).floor());
        c >= 0.0
            && r >= 0.0
            && (c as usize) < self.n
            && (r as usize) < self.n
            && self.open[r as usize * self.n + c as usize]
    }
}

fn paint_rect(cells: &mut [bool], n: usize, lo: Vec2, hi: Vec2) {
    let (c0, c1) = (((lo.x / CELL).floor().max(0.0)) as usize, ((hi.x / CELL).ceil() as usize).min(n));
    let (r0, r1) = (((lo.y / CELL).floor().max(0.0)) as usize, ((hi.y / CELL).ceil() as usize).min(n));
    for r in r0..r1 {
        for c in c0..c1 {
            cells[r * n + c] = true;
        }
    }
}

// ---- tracing ------------------------------------------------------------------------------

/// The outlines of the `true` cells of `cells` within the tile `[x0, x1) × [y0, y1)`: one simple
/// polygon per 4-connected piece (no holes: open ground inside a tile always reaches its edge),
/// in grid corner coordinates. Corners on the tile's edge are flagged (they stay put when
/// smoothing, so neighboring tiles' outlines still meet).
fn trace_tile(cells: &[bool], n: usize, x0: usize, y0: usize, x1: usize, y1: usize) -> Vec<Vec<(i32, i32, bool)>> {
    let inside = |c: i64, r: i64| {
        c >= x0 as i64 && r >= y0 as i64 && c < x1 as i64 && r < y1 as i64 && cells[r as usize * n + c as usize]
    };
    let mut seen = vec![false; (x1 - x0) * (y1 - y0)];
    let local = |c: usize, r: usize| (r - y0) * (x1 - x0) + (c - x0);
    let mut out = Vec::new();
    for r in y0..y1 {
        for c in x0..x1 {
            if !cells[r * n + c] || seen[local(c, r)] {
                continue;
            }
            // The piece.
            let mut piece = Vec::new();
            let mut stack = vec![(c, r)];
            seen[local(c, r)] = true;
            while let Some((pc, pr)) = stack.pop() {
                piece.push((pc, pr));
                for (dc, dr) in [(1i64, 0i64), (-1, 0), (0, 1), (0, -1)] {
                    let (qc, qr) = (pc as i64 + dc, pr as i64 + dr);
                    if inside(qc, qr) && !seen[local(qc as usize, qr as usize)] {
                        seen[local(qc as usize, qr as usize)] = true;
                        stack.push((qc as usize, qr as usize));
                    }
                }
            }
            // Its boundary edges, clockwise around each cell (y down).
            let mut edges: std::collections::BTreeMap<(i32, i32), Vec<(i32, i32)>> = std::collections::BTreeMap::new();
            let mut add = |a: (i32, i32), b: (i32, i32)| edges.entry(a).or_default().push(b);
            for &(pc, pr) in &piece {
                let (x, y) = (pc as i32, pr as i32);
                let (ci, ri) = (pc as i64, pr as i64);
                if !inside(ci, ri - 1) {
                    add((x, y), (x + 1, y));
                }
                if !inside(ci + 1, ri) {
                    add((x + 1, y), (x + 1, y + 1));
                }
                if !inside(ci, ri + 1) {
                    add((x + 1, y + 1), (x, y + 1));
                }
                if !inside(ci - 1, ri) {
                    add((x, y + 1), (x, y));
                }
            }
            // Link them into loops; where two leave a corner (pieces touching diagonally), take
            // the sharper turn so loops stay apart.
            while let Some((&start, _)) = edges.iter().find(|(_, v)| !v.is_empty()) {
                let mut lp = vec![start];
                let (mut at, mut dir) = (start, (0, 0));
                loop {
                    let outs = edges.get_mut(&at).expect("an outgoing edge");
                    let pick = (0..outs.len())
                        .max_by_key(|&i| {
                            let d = (outs[i].0 - at.0, outs[i].1 - at.1);
                            dir.0 * d.1 - dir.1 * d.0
                        })
                        .expect("an edge");
                    let next = outs.swap_remove(pick);
                    dir = (next.0 - at.0, next.1 - at.1);
                    at = next;
                    if at == start {
                        break;
                    }
                    lp.push(at);
                }
                // Corners only (drop points in the middle of straight runs).
                let m = lp.len();
                let corners: Vec<(i32, i32, bool)> = (0..m)
                    .filter(|&i| {
                        let (p, q, s) = (lp[(i + m - 1) % m], lp[i], lp[(i + 1) % m]);
                        (q.0 - p.0) * (s.1 - q.1) - (q.1 - p.1) * (s.0 - q.0) != 0
                    })
                    .map(|i| {
                        let (x, y) = lp[i];
                        let edge = x == x0 as i32 || x == x1 as i32 || y == y0 as i32 || y == y1 as i32;
                        (x, y, edge)
                    })
                    .collect();
                if corners.len() >= 3 {
                    out.push(corners);
                }
            }
        }
    }
    out
}

/// Round a traced outline: two Chaikin passes (pinned corners stay), then drop points that
/// deviate less than `tolerance` from the line through their neighbors.
fn smooth(corners: &[(i32, i32, bool)], tolerance: f32) -> Vec<Vec2> {
    let mut pts: Vec<(Vec2, bool)> =
        corners.iter().map(|&(x, y, pin)| (Vec2::new(x as f32, y as f32) * CELL, pin)).collect();
    for _ in 0..2 {
        let m = pts.len();
        let mut next: Vec<(Vec2, bool)> = Vec::with_capacity(m * 2);
        for i in 0..m {
            let ((p, pp), (q, qp)) = (pts[i], pts[(i + 1) % m]);
            let a = if pp { (p, true) } else { (p * 0.75 + q * 0.25, false) };
            let b = if qp { (q, true) } else { (p * 0.25 + q * 0.75, false) };
            for v in [a, b] {
                if next.last().is_none_or(|l: &(Vec2, bool)| l.0.distance(v.0) > 0.01) {
                    next.push(v);
                }
            }
        }
        if next.len() > 1 && next[0].0.distance(next[next.len() - 1].0) < 0.01 {
            next.pop();
        }
        pts = next;
    }
    // Drop near-straight points (never pinned ones), one pass.
    let m = pts.len();
    let mut out: Vec<Vec2> = Vec::with_capacity(m);
    for i in 0..m {
        let (p, pin) = pts[i];
        let prev = out.last().copied().unwrap_or(pts[(i + m - 1) % m].0);
        let next = pts[(i + 1) % m].0;
        if pin || crate::map::dist_point_segment(p, prev, next) >= tolerance {
            out.push(p);
        }
    }
    out
}

/// The wall polygons of a built grid.
pub fn walls(grid: &Grid) -> Vec<Vec<Vec2>> {
    let n = grid.n;
    let wall: Vec<bool> = grid.open.iter().map(|o| !o).collect();
    let mut out = Vec::new();
    for ty in (0..n).step_by(TILE) {
        for tx in (0..n).step_by(TILE) {
            for outline in trace_tile(&wall, n, tx, ty, (tx + TILE).min(n), (ty + TILE).min(n)) {
                let poly = smooth(&outline, 6.0);
                if poly.len() >= 3 {
                    out.push(poly);
                }
            }
        }
    }
    out
}

/// The tall-grass polygons of a built grid (traced whole: grass patches have no holes).
pub fn grass(grid: &Grid) -> Vec<Vec<Vec2>> {
    trace_tile(&grid.grass, grid.n, 0, 0, grid.n, grid.n)
        .iter()
        .map(|o| smooth(o, 6.0))
        .filter(|p| p.len() >= 3)
        .collect()
}

// ---- the map ------------------------------------------------------------------------------

/// Build the map a spec describes (whatever its problems: `problems` lists them).
pub fn build(spec: &MapSpec, id: MapId) -> Map {
    let grid = Grid::build(spec);
    let size = Vec2::new(spec.size, spec.size);
    let mut map = Map::new(id, walls(&grid), grass(&grid), Some(size));
    let red = |p: Vec2| MapSpec::mirror(p);
    // Structures: per lane on blue's half (bases fixed in the corner), mirrored for red.
    let mut blue: Vec<(UnitKind, Vec2, u8, u8)> = Vec::new();
    for lane in 0..spec.lanes.len() {
        for tier in 1..=GATEHOUSE_TIER {
            let kind = if tier == GATEHOUSE_TIER { UnitKind::Gatehouse } else { UnitKind::Turret };
            if let Some(p) = spec.structure_spot(lane, tier) {
                blue.push((kind, p, tier, lane as u8));
            }
        }
    }
    let mid = (spec.lanes.len() / 2) as u8;
    blue.push((UnitKind::Turret, spec.corner(2500.0, 2050.0), BASE_TURRET_TIER, mid));
    blue.push((UnitKind::Turret, spec.corner(2050.0, 2500.0), BASE_TURRET_TIER, mid));
    blue.push((UnitKind::Base, spec.corner(1800.0, 1800.0), BASE_TURRET_TIER + 1, mid));
    let mut placements = Vec::new();
    for (kind, pos, tier, lane) in blue {
        placements.push(Placement { kind, team: Team::Blue, pos, tier, lane });
        placements.push(Placement { kind, team: Team::Red, pos: red(pos), tier, lane });
    }
    let red_base = red(spec.corner(1800.0, 1800.0));
    let mut lanes = Vec::new();
    let mut wave_spawn = Vec::new();
    for lane in 0..spec.lanes.len() {
        let mut path = spec.lane_path(lane);
        let start = path[0];
        let mut blue_path: Vec<Vec2> = path.iter().skip(1).copied().collect();
        blue_path.push(red_base);
        path.reverse();
        let mut red_path: Vec<Vec2> = path.iter().skip(1).copied().collect();
        red_path.push(spec.corner(1800.0, 1800.0));
        lanes.push([blue_path, red_path]);
        wave_spawn.push([start, red(start)]);
    }
    // Camps on both halves, facing the nearest open ground away from their wall.
    let mut camps = Vec::new();
    for &(kind, pos) in &spec.camps {
        for p in [pos, red(pos)] {
            camps.push(Camp { kind, pos: p, facing: open_side(&grid, p) });
        }
    }
    let (home, fountain) = (spec.corner(800.0, 800.0), spec.corner(600.0, 600.0));
    let river = spec.river.first().map(|l| {
        let mut pts = l.points.clone();
        pts.extend(l.points.iter().rev().map(|p| red(*p)));
        pts
    });
    map.layout = Layout {
        lanes,
        wave_spawn,
        champion_spawn: [home, red(home)],
        fountains: [Some((fountain, 750.0)), Some((red(fountain), 750.0))],
        fountain_heals: true,
        placements,
        pacing: Pacing::Classic,
        river: river.unwrap_or_default(),
        plating: true,
        backdoor: true,
        camps,
        wards: true,
    };
    map
}

/// The direction from `p` toward the most open ground around it (a camp's front).
fn open_side(grid: &Grid, p: Vec2) -> Vec2 {
    let mut best = (0, Vec2::new(0.0, 1.0));
    for i in 0..16 {
        let a = i as f32 * std::f32::consts::TAU / 16.0;
        let dir = Vec2::new(a.cos(), a.sin());
        let reach = (1..=40).take_while(|s| grid.open_at(p + dir * (*s as f32 * CELL))).count();
        if reach > best.0 {
            best = (reach, dir);
        }
    }
    best.1
}

/// What's wrong with a spec, in words (empty: it can be exported and played).
pub fn problems(spec: &MapSpec) -> Vec<String> {
    let mut out = Vec::new();
    if !SIZES.contains(&spec.size) {
        out.push(format!("The size must be one of {SIZES:?}."));
    }
    if spec.lanes.is_empty() || spec.lanes.len() > MAX_LANES {
        out.push(format!("A map needs 1 to {MAX_LANES} lanes (it has {}).", spec.lanes.len()));
    }
    let b = spec.base_side();
    let in_base = |p: Vec2| p.x <= b && p.y >= spec.size - b;
    for (i, l) in spec.lanes.iter().enumerate() {
        let (first, last) = (l.points[0], *l.points.last().unwrap());
        if !in_base(first) {
            out.push(format!("Lane {} must start in the base.", i + 1));
        }
        if (last.x - last.y).abs() / std::f32::consts::SQRT_2 > LANE_END_SLACK {
            out.push(format!("Lane {} must end on the river line.", i + 1));
        }
    }
    let grid = Grid::build(spec);
    if grid.cut_off > 0 {
        out.push(format!("Some drawn ground ({} cells) doesn't connect to the lanes or the base.", grid.cut_off));
    }
    let half = (grid.n * grid.n / 2) as f32;
    if (grid.jungle_cells as f32) < MIN_JUNGLE_SHARE * half {
        out.push(format!(
            "The jungle is too small ({:.1}% of a half; at least {:.0}%).",
            100.0 * grid.jungle_cells as f32 / half,
            100.0 * MIN_JUNGLE_SHARE
        ));
    }
    let blue_camps: Vec<CampKind> =
        spec.camps.iter().filter(|(_, p)| MapSpec::on_blue_half(*p)).map(|(k, _)| *k).collect();
    if blue_camps.len() < MIN_CAMPS || !blue_camps.contains(&CampKind::Warden) || !blue_camps.contains(&CampKind::Brute)
    {
        out.push(format!("Each side needs at least {MIN_CAMPS} camps, including an Azure Warden and an Ember Brute."));
    }
    for (k, p) in &spec.camps {
        if !MapSpec::on_blue_half(*p) {
            out.push(format!("The {} camp must be on blue's half (below the river line).", camp_key(*k)));
        }
    }
    // Great objectives: 2 or 4 in pairs through the center, or 1 on a 1-lane map.
    let n = spec.objectives.len();
    let paired = spec
        .objectives
        .iter()
        .all(|p| spec.objectives.iter().any(|q| q.distance(Vec2::new(spec.size, spec.size) - *p) < 50.0));
    match n {
        1 if spec.lanes.len() != 1 => out.push("A single great objective is only for 1-lane maps.".into()),
        1 | 2 | 4 if n == 1 || paired => {}
        _ => out.push("Great objectives come as 1 (1-lane maps), 2 or 4, each with its twin across the center.".into()),
    }
    let mut stands = |what: String, p: Vec2| {
        if !grid.open_at(p) {
            out.push(format!("{what} is in a wall."));
        }
    };
    for (k, p) in &spec.camps {
        stands(format!("The {} camp", camp_key(*k)), *p);
    }
    for (i, p) in spec.objectives.iter().enumerate() {
        stands(format!("Great objective {}", i + 1), *p);
    }
    for lane in 0..spec.lanes.len() {
        for tier in 1..=GATEHOUSE_TIER {
            if let Some(p) = spec.structure_spot(lane, tier) {
                stands(format!("Lane {}'s tier-{tier} structure", lane + 1), p);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny() -> MapSpec {
        let mut s = MapSpec::new("Test", SIZES[0]);
        let c = |x: f32, y: f32| Vec2::new(x, y);
        // One mid lane, the river, a jungle loop with camps, and objectives.
        s.lanes.push(Line { width: 1000.0, points: vec![c(2400.0, 7600.0), c(5000.0, 5000.0)] });
        s.river.push(Line { width: 900.0, points: vec![c(1000.0, 1000.0), c(9000.0, 9000.0)] });
        s.jungle.push(Line { width: 450.0, points: vec![c(1500.0, 6000.0), c(2500.0, 4000.0), c(4000.0, 4100.0)] });
        s.camps.push((CampKind::Warden, c(2500.0, 4300.0)));
        s.camps.push((CampKind::Brute, c(5700.0, 7500.0)));
        s.camps.push((CampKind::Toad, c(1600.0, 5800.0)));
        s.jungle.push(Line { width: 450.0, points: vec![c(3500.0, 6800.0), c(5700.0, 7500.0), c(6600.0, 6600.0)] });
        s.objectives.push(c(3000.0, 3000.0));
        s.objectives.push(c(7000.0, 7000.0));
        s
    }

    #[test]
    fn specs_round_trip_as_text() {
        let s = tiny();
        let back = MapSpec::parse(&s.to_text()).unwrap();
        assert_eq!(back, s);
        assert!(MapSpec::parse("lane 100 1,2 3,4").is_err(), "no header");
        assert!(MapSpec::parse("mftr-map 1\ncamp dragon 1,2").is_err());
    }

    #[test]
    fn a_built_map_is_mirrored_connected_and_valid() {
        let s = tiny();
        assert_eq!(problems(&s), Vec::<String>::new());
        let m = build(&s, MapId::Custom);
        // Mirrored: open where its mirror is open.
        for p in [Vec2::new(2500.0, 4300.0), Vec2::new(5000.0, 5000.0), Vec2::new(3000.0, 6500.0)] {
            assert_eq!(m.walkable(p, 1.0), m.walkable(MapSpec::mirror(p), 1.0), "{p:?}");
        }
        assert!(m.walkable(Vec2::new(2500.0, 4300.0), 60.0), "the camp's clearing");
        assert!(!m.walkable(Vec2::new(500.0, 5000.0), 1.0), "undrawn is wall");
        // Every structure stands in the open, and a champion paths base to base.
        for p in &m.layout.placements {
            assert!(m.walkable(p.pos, 1.0), "{p:?}");
        }
        let (a, b) = (m.layout.champion_spawn[0], m.layout.champion_spawn[1]);
        assert!(m.find_path(a, b).last().is_some_and(|p| p.distance(b) < 1.0));
        assert_eq!(m.layout.camps.len(), 6);
        assert_eq!(m.layout.lanes.len(), 1);
    }

    #[test]
    fn problems_are_reported() {
        let mut s = tiny();
        s.jungle.push(Line { width: 300.0, points: vec![Vec2::new(6000.0, 9500.0), Vec2::new(6500.0, 9500.0)] });
        s.objectives.push(Vec2::new(4000.0, 4000.0));
        s.camps.retain(|(k, _)| *k != CampKind::Brute);
        let p = problems(&s);
        assert!(p.iter().any(|e| e.contains("doesn't connect")), "{p:?}");
        assert!(p.iter().any(|e| e.contains("Great objectives come as")), "{p:?}");
        assert!(p.iter().any(|e| e.contains("Ember Brute")), "{p:?}");
    }
}
