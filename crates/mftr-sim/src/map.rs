//! Map geometry (04 §3 "Navigation"): walls and brush as vector polygons, a navigation grid
//! derived from them, deterministic A* pathfinding with string pulling, and line-of-sight.
//!
//! M1 uses a fine grid (25 u cells) plus string pulling, which yields the same any-angle
//! waypoint paths a navmesh with a funnel pass would. A true navmesh can replace the grid
//! behind this API when the full map needs it (D24).

use crate::math::Vec2;
use std::collections::BinaryHeap;

/// Which built-in map a match uses. Server and client must agree (sent in the welcome).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapId {
    /// No walls, no brush: the open plane of M0.
    Open = 0,
    /// M1 sandbox arena: 4,000 u square with rocks, walls and brush patches.
    Arena = 1,
}

impl MapId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(MapId::Open),
            1 => Some(MapId::Arena),
            _ => None,
        }
    }

    /// The built map, constructed once per process and shared (maps are immutable).
    pub fn shared(self) -> std::sync::Arc<Map> {
        use std::sync::{Arc, OnceLock};
        static OPEN: OnceLock<Arc<Map>> = OnceLock::new();
        static ARENA: OnceLock<Arc<Map>> = OnceLock::new();
        let cell = match self {
            MapId::Open => &OPEN,
            MapId::Arena => &ARENA,
        };
        cell.get_or_init(|| Arc::new(self.build())).clone()
    }

    pub fn build(self) -> Map {
        match self {
            MapId::Open => Map::new(MapId::Open, Vec::new(), Vec::new(), false),
            MapId::Arena => arena(),
        }
    }
}

/// Grid cell size for navigation.
pub const NAV_CELL: f32 = 25.0;
/// Paths keep this clearance from walls (the champion collision radius).
pub const NAV_CLEARANCE: f32 = 35.0;
/// Extent of the navigable area (0..MAP_SIZE on both axes).
pub const MAP_SIZE: f32 = 4000.0;

#[derive(Clone, Debug)]
pub struct Map {
    pub id: MapId,
    /// Simple polygons (counter-clockwise or clockwise), impassable and vision-blocking.
    pub walls: Vec<Vec<Vec2>>,
    /// Brush polygons: walkable, hide units inside from observers outside.
    pub brush: Vec<Vec<Vec2>>,
    /// Walled in on 0..MAP_SIZE (else the open plane, limited only by the point encoding).
    pub bounded: bool,
    edges: Vec<(Vec2, Vec2)>,
    grid_w: usize,
    grid_h: usize,
    blocked: Vec<bool>,
}

impl Map {
    pub fn new(id: MapId, mut walls: Vec<Vec<Vec2>>, brush: Vec<Vec<Vec2>>, bounded: bool) -> Self {
        // A bounded map has a wall around it: four thin slabs just outside 0..MAP_SIZE.
        let (s, t) = (MAP_SIZE, 50.0);
        let slabs = if bounded {
            vec![(-t, -t, s + t, 0.0), (-t, s, s + t, s + t), (-t, 0.0, 0.0, s), (s, 0.0, s + t, s)]
        } else {
            Vec::new()
        };
        for (x0, y0, x1, y1) in slabs {
            walls.push(vec![Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)]);
        }
        let edges: Vec<(Vec2, Vec2)> =
            walls.iter().flat_map(|poly| (0..poly.len()).map(move |i| (poly[i], poly[(i + 1) % poly.len()]))).collect();
        let grid_w = (MAP_SIZE / NAV_CELL) as usize;
        let grid_h = grid_w;
        let mut map = Map { id, walls, brush, bounded, edges, grid_w, grid_h, blocked: Vec::new() };
        map.blocked = (0..grid_w * grid_h).map(|i| !map.walkable(map.cell_center(i), NAV_CLEARANCE)).collect();
        map
    }

    /// Wall edges, for collision sweeps.
    pub fn edges(&self) -> &[(Vec2, Vec2)] {
        &self.edges
    }

    /// True if a circle of `radius` at `p` touches no wall and isn't inside one.
    pub fn walkable(&self, p: Vec2, radius: f32) -> bool {
        if self.walls.iter().any(|w| point_in_polygon(p, w)) {
            return false;
        }
        self.edges.iter().all(|&(a, b)| dist_point_segment(p, a, b) >= radius)
    }

    /// A circle of `radius` at `p` lies inside the playable area (blinks never leave it).
    pub fn in_bounds(&self, p: Vec2, radius: f32) -> bool {
        let max = if self.bounded { MAP_SIZE } else { u16::MAX as f32 * crate::math::QPoint::STEP };
        p.x >= radius && p.y >= radius && p.x <= max - radius && p.y <= max - radius
    }

    /// Line of sight between two points: no wall edge crossed (vision, 03 §10).
    pub fn line_of_sight(&self, a: Vec2, b: Vec2) -> bool {
        !self.edges.iter().any(|&(p, q)| segments_intersect(a, b, p, q))
    }

    /// A straight move from `a` to `b` keeps `clearance` from every wall.
    pub fn segment_clear(&self, a: Vec2, b: Vec2, clearance: f32) -> bool {
        self.line_of_sight(a, b) && self.edges.iter().all(|&(p, q)| dist_segment_segment(a, b, p, q) >= clearance)
    }

    /// Index of the brush polygon containing `p`, if any.
    pub fn brush_at(&self, p: Vec2) -> Option<u8> {
        self.brush.iter().position(|b| point_in_polygon(p, b)).map(|i| i as u8)
    }

    fn cell_of(&self, p: Vec2) -> Option<usize> {
        let (cx, cy) = ((p.x / NAV_CELL).floor(), (p.y / NAV_CELL).floor());
        if cx < 0.0 || cy < 0.0 || cx >= self.grid_w as f32 || cy >= self.grid_h as f32 {
            return None;
        }
        Some(cy as usize * self.grid_w + cx as usize)
    }

    fn cell_center(&self, i: usize) -> Vec2 {
        let (x, y) = (i % self.grid_w, i / self.grid_w);
        Vec2::new((x as f32 + 0.5) * NAV_CELL, (y as f32 + 0.5) * NAV_CELL)
    }

    /// Nearest walkable cell to `p` (breadth-first over the grid, deterministic).
    fn nearest_open_cell(&self, p: Vec2) -> Option<usize> {
        let clamped = Vec2::new(p.x.clamp(0.0, MAP_SIZE - 1.0), p.y.clamp(0.0, MAP_SIZE - 1.0));
        let start = self.cell_of(clamped)?;
        if !self.blocked[start] {
            return Some(start);
        }
        let mut seen = vec![false; self.blocked.len()];
        let mut queue = std::collections::VecDeque::from([start]);
        seen[start] = true;
        while let Some(i) = queue.pop_front() {
            if !self.blocked[i] {
                return Some(i);
            }
            for n in self.neighbors4(i) {
                if !seen[n] {
                    seen[n] = true;
                    queue.push_back(n);
                }
            }
        }
        None
    }

    fn neighbors4(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let (x, y) = ((i % self.grid_w) as isize, (i / self.grid_w) as isize);
        [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().filter_map(move |(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            (nx >= 0 && ny >= 0 && (nx as usize) < self.grid_w && (ny as usize) < self.grid_h)
                .then(|| ny as usize * self.grid_w + nx as usize)
        })
    }

    /// Any-angle path from `from` to `to`: grid A* (integer costs, deterministic tie-breaks)
    /// followed by string pulling against wall clearance. Returns the waypoints after `from`;
    /// the last one is `to`, or the nearest reachable point when `to` is inside a wall.
    /// Returns `vec![to]` when the straight line is already clear (the common case).
    pub fn find_path(&self, from: Vec2, to: Vec2) -> Vec<Vec2> {
        if self.walls.is_empty() || (self.segment_clear(from, to, NAV_CLEARANCE) && self.walkable(to, NAV_CLEARANCE)) {
            return vec![to];
        }
        let Some(goal_cell) = self.nearest_open_cell(to) else { return Vec::new() };
        let goal_pt = if self.walkable(to, NAV_CLEARANCE) { to } else { self.cell_center(goal_cell) };
        let Some(start_cell) = self.nearest_open_cell(from) else { return vec![goal_pt] };
        let cells = self.astar(start_cell, goal_cell);
        if cells.is_empty() {
            return vec![goal_pt];
        }
        // String pulling: from the current point, jump to the farthest cell center still in
        // clear view, then continue from there.
        let mut nodes: Vec<Vec2> = cells.iter().map(|&c| self.cell_center(c)).collect();
        *nodes.last_mut().unwrap() = goal_pt;
        let mut out = Vec::new();
        let mut at = from;
        let mut i = 0;
        while i < nodes.len() {
            let mut far = i;
            for j in (i..nodes.len()).rev() {
                if self.segment_clear(at, nodes[j], NAV_CLEARANCE - 1.0) {
                    far = j;
                    break;
                }
            }
            at = nodes[far];
            out.push(at);
            i = far + 1;
        }
        out
    }

    fn astar(&self, start: usize, goal: usize) -> Vec<usize> {
        let n = self.blocked.len();
        let mut g = vec![u32::MAX; n];
        let mut came = vec![usize::MAX; n];
        let mut closed = vec![false; n];
        let (gx, gy) = ((goal % self.grid_w) as i64, (goal / self.grid_w) as i64);
        let h = |i: usize| -> u32 {
            let (dx, dy) = (((i % self.grid_w) as i64 - gx).abs(), ((i / self.grid_w) as i64 - gy).abs());
            (10 * (dx + dy) - 6 * dx.min(dy)) as u32 // octile with costs 10 / 14
        };
        // Max-heap of Reverse-ordered keys: (f, h, cell) — deterministic tie-breaking.
        let mut open = BinaryHeap::new();
        g[start] = 0;
        open.push(std::cmp::Reverse((h(start), h(start), start)));
        while let Some(std::cmp::Reverse((_, _, cur))) = open.pop() {
            if closed[cur] {
                continue;
            }
            if cur == goal {
                let mut path = vec![goal];
                let mut c = goal;
                while c != start {
                    c = came[c];
                    path.push(c);
                }
                path.reverse();
                return path;
            }
            closed[cur] = true;
            let (x, y) = ((cur % self.grid_w) as isize, (cur / self.grid_w) as isize);
            for (dx, dy, cost) in
                [(1, 0, 10), (-1, 0, 10), (0, 1, 10), (0, -1, 10), (1, 1, 14), (1, -1, 14), (-1, 1, 14), (-1, -1, 14)]
            {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx as usize >= self.grid_w || ny as usize >= self.grid_h {
                    continue;
                }
                let nb = ny as usize * self.grid_w + nx as usize;
                if self.blocked[nb] || closed[nb] {
                    continue;
                }
                // No corner cutting.
                if dx != 0 && dy != 0 {
                    let a = y as usize * self.grid_w + nx as usize;
                    let b = ny as usize * self.grid_w + x as usize;
                    if self.blocked[a] || self.blocked[b] {
                        continue;
                    }
                }
                let ng = g[cur] + cost;
                if ng < g[nb] {
                    g[nb] = ng;
                    came[nb] = cur;
                    open.push(std::cmp::Reverse((ng + h(nb), h(nb), nb)));
                }
            }
        }
        Vec::new()
    }
}

fn poly(points: &[(f32, f32)]) -> Vec<Vec2> {
    points.iter().map(|&(x, y)| Vec2::new(x, y)).collect()
}

fn rect(x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<Vec2> {
    poly(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

/// The M1 sandbox arena. Walls stay clear of the minion clumps, the patrol lines (x = 2000,
/// y = 2000) and the dodge-rig turrets.
fn arena() -> Map {
    let walls = vec![
        // A rock between the north-west clump and the center.
        poly(&[
            (1560.0, 1480.0),
            (1720.0, 1450.0),
            (1800.0, 1580.0),
            (1720.0, 1720.0),
            (1560.0, 1700.0),
            (1500.0, 1590.0),
        ]),
        // A long wall east of center, north half: forces paths around it.
        rect(2380.0, 650.0, 2460.0, 1750.0),
        // An L-shaped wall south-west.
        poly(&[(650.0, 2300.0), (1550.0, 2300.0), (1550.0, 2380.0), (730.0, 2380.0), (730.0, 2800.0), (650.0, 2800.0)]),
        // A block south-east of center.
        rect(2300.0, 2450.0, 2520.0, 2850.0),
    ];
    let brush = vec![
        rect(1650.0, 2380.0, 1950.0, 2680.0),
        rect(2600.0, 1650.0, 2950.0, 1880.0),
        rect(880.0, 1500.0, 1150.0, 1850.0),
        rect(3150.0, 2450.0, 3500.0, 2800.0),
    ];
    Map::new(MapId::Arena, walls, brush, true)
}

// ---- geometry ---------------------------------------------------------------------------

pub fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

pub fn closest_point_on_segment(p: Vec2, a: Vec2, b: Vec2) -> Vec2 {
    let ab = b - a;
    let len2 = ab.length_sq();
    if len2 == 0.0 {
        return a;
    }
    let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
    a + ab * t
}

pub fn dist_point_segment(p: Vec2, a: Vec2, b: Vec2) -> f32 {
    p.distance(closest_point_on_segment(p, a, b))
}

fn cross(a: Vec2, b: Vec2) -> f32 {
    a.x * b.y - a.y * b.x
}

/// Proper or touching intersection of segments `a-b` and `c-d`.
pub fn segments_intersect(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> bool {
    let (r, s) = (b - a, d - c);
    let denom = cross(r, s);
    if denom == 0.0 {
        return false; // parallel: treat as non-crossing (thin-wall grazing is fine for LoS)
    }
    let t = cross(c - a, s) / denom;
    let u = cross(c - a, r) / denom;
    (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)
}

pub fn dist_segment_segment(a: Vec2, b: Vec2, c: Vec2, d: Vec2) -> f32 {
    if segments_intersect(a, b, c, d) {
        return 0.0;
    }
    dist_point_segment(a, c, d)
        .min(dist_point_segment(b, c, d))
        .min(dist_point_segment(c, a, b))
        .min(dist_point_segment(d, a, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry_basics() {
        let sq = rect(0.0, 0.0, 10.0, 10.0);
        assert!(point_in_polygon(Vec2::new(5.0, 5.0), &sq));
        assert!(!point_in_polygon(Vec2::new(15.0, 5.0), &sq));
        assert!(segments_intersect(
            Vec2::new(-1.0, 5.0),
            Vec2::new(11.0, 5.0),
            Vec2::new(10.0, 0.0),
            Vec2::new(10.0, 10.0)
        ));
        assert_eq!(dist_point_segment(Vec2::new(5.0, 3.0), Vec2::new(0.0, 0.0), Vec2::new(10.0, 0.0)), 3.0);
    }

    #[test]
    fn open_map_paths_are_straight() {
        let m = MapId::Open.build();
        assert_eq!(m.find_path(Vec2::new(100.0, 100.0), Vec2::new(3000.0, 2000.0)), vec![Vec2::new(3000.0, 2000.0)]);
    }

    #[test]
    fn arena_path_goes_around_the_long_wall_with_clearance() {
        let m = MapId::Arena.build();
        let (from, to) = (Vec2::new(2200.0, 1200.0), Vec2::new(2700.0, 1200.0));
        assert!(!m.segment_clear(from, to, NAV_CLEARANCE));
        let path = m.find_path(from, to);
        assert_eq!(*path.last().unwrap(), to);
        assert!(path.len() >= 2 && path.len() <= 6, "{path:?}");
        let mut at = from;
        for &p in &path {
            assert!(m.segment_clear(at, p, NAV_CLEARANCE - 1.0), "{at:?} -> {p:?} clips a wall");
            at = p;
        }
    }

    #[test]
    fn goal_inside_a_wall_snaps_to_nearest_reachable_point() {
        let m = MapId::Arena.build();
        let path = m.find_path(Vec2::new(2000.0, 1200.0), Vec2::new(2420.0, 1200.0)); // inside the long wall
        let end = *path.last().unwrap();
        assert!(m.walkable(end, NAV_CLEARANCE - 1.0) && end.distance(Vec2::new(2420.0, 1200.0)) < 120.0, "{end:?}");
    }

    #[test]
    fn walls_block_sight_and_brush_is_found() {
        let m = MapId::Arena.build();
        assert!(!m.line_of_sight(Vec2::new(2200.0, 1200.0), Vec2::new(2700.0, 1200.0)));
        assert!(m.line_of_sight(Vec2::new(2200.0, 1900.0), Vec2::new(2700.0, 1900.0)));
        assert_eq!(m.brush_at(Vec2::new(1800.0, 2500.0)), Some(0));
        assert_eq!(m.brush_at(Vec2::new(2000.0, 2000.0)), None);
    }

    #[test]
    fn pathfinding_is_deterministic() {
        let m = MapId::Arena.build();
        let a = m.find_path(Vec2::new(600.0, 2600.0), Vec2::new(1000.0, 2600.0));
        let b = m.find_path(Vec2::new(600.0, 2600.0), Vec2::new(1000.0, 2600.0));
        assert_eq!(a, b);
    }
}
