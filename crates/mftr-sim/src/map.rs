//! Map geometry (04 §3 "Navigation"): walls and brush as vector polygons, a navigation grid
//! derived from them, deterministic pathfinding (jump-point search) with string pulling, and
//! line-of-sight.
//!
//! A fine grid (25 u cells) plus string pulling yields the same any-angle waypoint paths a
//! navmesh with a funnel pass would. Jump-point search finds the same shortest grid paths as
//! A* over every cell, touching a fraction of them (13 §8). A true navmesh can replace the grid
//! behind this API when a map needs it (D24).

use crate::math::Vec2;
use crate::world::{Team, UnitKind};
use std::collections::BinaryHeap;

/// Which built-in map a match uses. Server and client must agree (sent in the welcome).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MapId {
    /// No walls, no brush: the open plane of M0.
    Open = 0,
    /// M1 sandbox arena: 4,000 u square with rocks, walls and brush patches.
    Arena = 1,
    /// ARAM map (M2): a single lane, 12,000 × 3,000 u, structures at both ends.
    Bridge = 2,
    /// The 5v5 map (M4): three lanes, a river and jungle on a 14,500 u square.
    Crossroads = 3,
    /// A map built from a spec (12): never built by id (`mapspec::build`).
    Custom = 4,
    /// The Mayhem map adapted for five champions defending against a horde.
    HordeBridge = 5,
}

impl MapId {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(MapId::Open),
            1 => Some(MapId::Arena),
            2 => Some(MapId::Bridge),
            3 => Some(MapId::Crossroads),
            4 => Some(MapId::Custom),
            5 => Some(MapId::HordeBridge),
            _ => None,
        }
    }

    /// The built map, constructed once per process and shared (maps are immutable).
    pub fn shared(self) -> std::sync::Arc<Map> {
        use std::sync::{Arc, OnceLock};
        static OPEN: OnceLock<Arc<Map>> = OnceLock::new();
        static ARENA: OnceLock<Arc<Map>> = OnceLock::new();
        static BRIDGE: OnceLock<Arc<Map>> = OnceLock::new();
        static CROSSROADS: OnceLock<Arc<Map>> = OnceLock::new();
        static HORDE_BRIDGE: OnceLock<Arc<Map>> = OnceLock::new();
        let cell = match self {
            MapId::Open => &OPEN,
            MapId::Arena => &ARENA,
            MapId::Bridge => &BRIDGE,
            MapId::Crossroads => &CROSSROADS,
            MapId::HordeBridge => &HORDE_BRIDGE,
            // Built from its spec, not by id: the open plane stands in.
            MapId::Custom => &OPEN,
        };
        cell.get_or_init(|| Arc::new(self.build())).clone()
    }

    pub fn build(self) -> Map {
        match self {
            MapId::Open => Map::new(MapId::Open, Vec::new(), Vec::new(), None),
            MapId::Arena => arena(),
            MapId::Bridge => bridge(),
            MapId::Crossroads => crossroads(),
            MapId::Custom => Map::new(MapId::Custom, Vec::new(), Vec::new(), None),
            MapId::HordeBridge => {
                let mut map = for_horde(bridge());
                map.id = MapId::HordeBridge;
                map
            }
        }
    }
}

/// Keep the Mayhem terrain and blue defenses, remove the red structures and fountain, and
/// let defenders recover at home between waves. Also applies to custom ARAM maps.
pub fn for_horde(mut map: Map) -> Map {
    map.layout.placements.retain(|p| p.team == Team::Blue || p.kind == UnitKind::Relic);
    map.layout.fountains[Team::Red as usize] = None;
    map.layout.fountain_heals = true;
    map
}

/// Grid cell size for navigation.
pub const NAV_CELL: f32 = 25.0;
/// Bucket size of the wall-edge index (sight and clearance tests).
pub const EDGE_CELL: f32 = 500.0;
/// Paths keep this clearance from walls (the champion collision radius).
pub const NAV_CLEARANCE: f32 = 35.0;
/// Extent of the M1 arena (0..MAP_SIZE on both axes), and of the open plane's nav grid.
pub const MAP_SIZE: f32 = 4000.0;

/// A structure or pickup placed by the map (01 §3).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub kind: UnitKind,
    pub team: Team,
    pub pos: Vec2,
    /// Destruction order within the team and lane (1 = first to fall); 0 for relics. Tiers 1–4
    /// fall in order down their lane; base turrets (5) once a Gatehouse (4) is down; the Base (6)
    /// once both base turrets are too.
    pub tier: u8,
    /// Which lane (index into `Layout::lanes`) a lane structure stands in.
    pub lane: u8,
}

/// What a lane map adds to geometry: lanes, spawn points, fountains and structures.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    /// The lanes; each is a path per team, from its own base to the enemy's (index = `Team as
    /// usize`).
    pub lanes: Vec<[Vec<Vec2>; 2]>,
    /// Where each lane's waves spawn, per team.
    pub wave_spawn: Vec<[Vec2; 2]>,
    pub champion_spawn: [Vec2; 2],
    /// Fountain circles (center, radius) per team.
    pub fountains: [Option<(Vec2, f32)>; 2],
    /// Whether a fountain heals its team. Not on The Bridge: ARAM has no fountain healing after
    /// you leave base (06 §2), and a champion only gets hurt after leaving and respawns whole.
    pub fountain_heals: bool,
    pub placements: Vec<Placement>,
    /// How waves are paced on this map.
    pub pacing: crate::lane::Pacing,
    /// The river's course, if the map has one (drawn by the client; walls bound it).
    pub river: Vec<Vec2>,
    /// Outer turrets carry plates until 14:00 (`lane::PLATES`).
    pub plating: bool,
    /// Structures have backdoor protection (`lane::BACKDOOR_RANGE`).
    pub backdoor: bool,
    /// Jungle camps (01 §7).
    pub camps: Vec<crate::jungle::Camp>,
    /// Champions carry a trinket and can place wards (01 §8).
    pub wards: bool,
}

#[derive(Clone, Debug)]
pub struct Map {
    pub id: MapId,
    /// Simple polygons (counter-clockwise or clockwise), impassable and vision-blocking.
    pub walls: Vec<Vec<Vec2>>,
    /// Brush polygons: walkable, hide units inside from observers outside.
    pub brush: Vec<Vec<Vec2>>,
    /// Walled in on 0..size (else the open plane, limited only by the point encoding).
    pub bounded: bool,
    /// Width and height of the playable area.
    pub size: Vec2,
    /// Lanes, structures and spawn points (empty on sandbox maps).
    pub layout: Layout,
    edges: Vec<(Vec2, Vec2)>,
    /// The edges by `EDGE_CELL` buckets over the map (edges past it in the border buckets):
    /// sight and clearance tests only look at the buckets a segment crosses.
    edge_cells: Vec<Vec<u32>>,
    /// Each wall's bounding box (inside tests skip walls whose box misses the point).
    wall_boxes: Vec<(Vec2, Vec2)>,
    edge_cols: usize,
    edge_rows: usize,
    grid_w: usize,
    grid_h: usize,
    blocked: Vec<bool>,
    /// Each cell's nearest walkable cell (itself if walkable; `u32::MAX` if none is).
    nearest_open: Vec<u32>,
}

impl Map {
    /// A map of `walls` and `brush`; `bounds` walls it in on `0..bounds` (else it's open).
    pub fn new(id: MapId, mut walls: Vec<Vec<Vec2>>, brush: Vec<Vec<Vec2>>, bounds: Option<Vec2>) -> Self {
        // A bounded map has a wall around it: four thin slabs just outside 0..size.
        let bounded = bounds.is_some();
        let size = bounds.unwrap_or(Vec2::new(MAP_SIZE, MAP_SIZE));
        let (w, h, t) = (size.x, size.y, 50.0);
        let slabs = if bounded {
            vec![(-t, -t, w + t, 0.0), (-t, h, w + t, h + t), (-t, 0.0, 0.0, h), (w, 0.0, w + t, h)]
        } else {
            Vec::new()
        };
        for (x0, y0, x1, y1) in slabs {
            walls.push(vec![Vec2::new(x0, y0), Vec2::new(x1, y0), Vec2::new(x1, y1), Vec2::new(x0, y1)]);
        }
        let edges: Vec<(Vec2, Vec2)> =
            walls.iter().flat_map(|poly| (0..poly.len()).map(move |i| (poly[i], poly[(i + 1) % poly.len()]))).collect();
        let grid_w = (size.x / NAV_CELL).ceil() as usize;
        let grid_h = (size.y / NAV_CELL).ceil() as usize;
        let layout = Layout::default();
        let (edge_cols, edge_rows) =
            ((size.x / EDGE_CELL).ceil().max(1.0) as usize, (size.y / EDGE_CELL).ceil().max(1.0) as usize);
        let mut map = Map {
            id,
            walls,
            brush,
            bounded,
            size,
            layout,
            edges,
            edge_cells: vec![Vec::new(); edge_cols * edge_rows],
            wall_boxes: Vec::new(),
            edge_cols,
            edge_rows,
            grid_w,
            grid_h,
            blocked: Vec::new(),
            nearest_open: Vec::new(),
        };
        map.wall_boxes =
            map.walls.iter().map(|w| w.iter().fold((w[0], w[0]), |(lo, hi), &v| (lo.min(v), hi.max(v)))).collect();
        for i in 0..map.edges.len() {
            let (a, b) = map.edges[i];
            let cells: Vec<usize> = map.buckets_along(a, b, 0.0).collect();
            for c in cells {
                map.edge_cells[c].push(i as u32);
            }
        }
        map.blocked = (0..grid_w * grid_h).map(|i| !map.walkable(map.cell_center(i), NAV_CLEARANCE)).collect();
        // Breadth-first from every walkable cell at once (in cell order, deterministic).
        let mut nearest = vec![u32::MAX; map.blocked.len()];
        let mut queue = std::collections::VecDeque::new();
        for i in (0..map.blocked.len()).filter(|&i| !map.blocked[i]) {
            nearest[i] = i as u32;
            queue.push_back(i);
        }
        while let Some(i) = queue.pop_front() {
            for n in map.neighbors4(i) {
                if nearest[n] == u32::MAX {
                    nearest[n] = nearest[i];
                    queue.push_back(n);
                }
            }
        }
        map.nearest_open = nearest;
        map
    }

    /// Wall edges, for collision sweeps.
    pub fn edges(&self) -> &[(Vec2, Vec2)] {
        &self.edges
    }

    /// The wall edges that might come within `reach` of `center` (every edge that does, maybe
    /// a few more), in `edges()` order: what a sweep from `center` needs, cheaply.
    pub fn edges_within(&self, center: Vec2, reach: f32) -> Vec<(Vec2, Vec2)> {
        let pad = Vec2::new(reach, reach);
        let (c0, c1, r0, r1) = self.edge_span(center - pad, center + pad);
        let mut ids: Vec<u32> = Vec::new();
        for r in r0..=r1 {
            for c in c0..=c1 {
                ids.extend_from_slice(&self.edge_cells[r * self.edge_cols + c]);
            }
        }
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter().map(|i| self.edges[i as usize]).collect()
    }

    /// True if a circle of `radius` at `p` touches no wall and isn't inside one.
    pub fn walkable(&self, p: Vec2, radius: f32) -> bool {
        let inside = |(lo, hi): &(Vec2, Vec2)| p.x >= lo.x && p.y >= lo.y && p.x <= hi.x && p.y <= hi.y;
        if self.walls.iter().zip(&self.wall_boxes).any(|(w, b)| inside(b) && point_in_polygon(p, w)) {
            return false;
        }
        let pad = Vec2::new(radius, radius);
        self.edges_near(p - pad, p + pad).all(|(a, b)| dist_point_segment(p, a, b) >= radius)
    }

    /// A circle of `radius` at `p` lies inside the playable area (blinks never leave it).
    pub fn in_bounds(&self, p: Vec2, radius: f32) -> bool {
        let max =
            if self.bounded { self.size } else { Vec2::new(1.0, 1.0) * (u16::MAX as f32 * crate::math::QPoint::STEP) };
        p.x >= radius && p.y >= radius && p.x <= max.x - radius && p.y <= max.y - radius
    }

    /// Line of sight between two points: no wall edge crossed (vision, 03 §10).
    pub fn line_of_sight(&self, a: Vec2, b: Vec2) -> bool {
        !self.edges_along(a, b, 0.0).any(|(p, q)| segments_intersect(a, b, p, q))
    }

    /// A straight move from `a` to `b` keeps `clearance` from every wall.
    pub fn segment_clear(&self, a: Vec2, b: Vec2, clearance: f32) -> bool {
        self.line_of_sight(a, b)
            && self.edges_along(a, b, clearance).all(|(p, q)| dist_segment_segment(a, b, p, q) >= clearance)
    }

    /// Every edge in the buckets within `pad` of the segment from `a` to `b` (every edge
    /// that comes that close, and some more; an edge may come more than once). Unlike the
    /// segment's bounding box, a long diagonal only visits the buckets along it.
    fn edges_along(&self, a: Vec2, b: Vec2, pad: f32) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        self.buckets_along(a, b, pad).flat_map(move |i| self.edge_cells[i].iter().map(move |&e| self.edges[e as usize]))
    }

    /// The buckets within `pad` of the segment from `a` to `b` (and a unit more: the buckets
    /// are a superset, so rounding can't lose one), row by row.
    fn buckets_along(&self, a: Vec2, b: Vec2, pad: f32) -> impl Iterator<Item = usize> + '_ {
        let pad = pad + 1.0;
        let (_, _, r0, r1) = self.edge_span(a.min(b) - Vec2::new(pad, pad), a.max(b) + Vec2::new(pad, pad));
        let last = self.edge_rows - 1;
        (r0..=r1).flat_map(move |r| {
            // The part of the segment within `pad` of this row of buckets (border rows
            // reach past the map, as their buckets do).
            let lo = if r == 0 { f32::NEG_INFINITY } else { r as f32 * EDGE_CELL - pad };
            let hi = if r == last { f32::INFINITY } else { (r + 1) as f32 * EDGE_CELL + pad };
            let (x0, x1) = if a.y == b.y {
                (a.x.min(b.x), a.x.max(b.x))
            } else {
                let at = |y: f32| a.x + (b.x - a.x) * ((y - a.y) / (b.y - a.y)).clamp(0.0, 1.0);
                let (xa, xb) = (at(lo.max(a.y.min(b.y))), at(hi.min(a.y.max(b.y))));
                (xa.min(xb), xa.max(xb))
            };
            let (c0, c1, _, _) = self.edge_span(Vec2::new(x0 - pad, 0.0), Vec2::new(x1 + pad, 0.0));
            (c0..=c1).map(move |c| r * self.edge_cols + c)
        })
    }

    /// The edge buckets (column and row ranges, inclusive) a box from `lo` to `hi` touches,
    /// clamped to the grid.
    fn edge_span(&self, lo: Vec2, hi: Vec2) -> (usize, usize, usize, usize) {
        let cell = |v: f32, n: usize| ((v / EDGE_CELL).floor().max(0.0) as usize).min(n - 1);
        (cell(lo.x, self.edge_cols), cell(hi.x, self.edge_cols), cell(lo.y, self.edge_rows), cell(hi.y, self.edge_rows))
    }

    /// Every edge in the buckets a box from `lo` to `hi` touches (an edge may come more than
    /// once: callers only ask whether any or all pass a test).
    fn edges_near(&self, lo: Vec2, hi: Vec2) -> impl Iterator<Item = (Vec2, Vec2)> + '_ {
        let (c0, c1, r0, r1) = self.edge_span(lo, hi);
        (r0..=r1)
            .flat_map(move |r| (c0..=c1).map(move |c| r * self.edge_cols + c))
            .flat_map(move |i| self.edge_cells[i].iter().map(move |&e| self.edges[e as usize]))
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

    /// Nearest walkable cell to `p` (fewest grid steps, found when the map was built).
    fn nearest_open_cell(&self, p: Vec2) -> Option<usize> {
        let clamped = Vec2::new(p.x.clamp(0.0, self.size.x - 1.0), p.y.clamp(0.0, self.size.y - 1.0));
        let i = self.nearest_open[self.cell_of(clamped)?];
        (i != u32::MAX).then_some(i as usize)
    }

    fn neighbors4(&self, i: usize) -> impl Iterator<Item = usize> + '_ {
        let (x, y) = ((i % self.grid_w) as isize, (i / self.grid_w) as isize);
        [(1, 0), (-1, 0), (0, 1), (0, -1)].into_iter().filter_map(move |(dx, dy)| {
            let (nx, ny) = (x + dx, y + dy);
            (nx >= 0 && ny >= 0 && (nx as usize) < self.grid_w && (ny as usize) < self.grid_h)
                .then(|| ny as usize * self.grid_w + nx as usize)
        })
    }

    /// Any-angle path from `from` to `to`: a grid search (jump-point search: integer costs,
    /// deterministic tie-breaks) followed by string pulling against wall clearance. Returns the
    /// waypoints after `from`; the last one is `to`, or the nearest reachable point when `to`
    /// is inside a wall.
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
        ASTAR.with(|scratch| self.astar_in(&mut scratch.borrow_mut(), start, goal))
    }

    /// Whether grid cell (x, y) is on the grid and walkable.
    fn open_xy(&self, x: isize, y: isize) -> bool {
        x >= 0
            && y >= 0
            && (x as usize) < self.grid_w
            && (y as usize) < self.grid_h
            && !self.blocked[y as usize * self.grid_w + x as usize]
    }

    /// Jump-point search (Harabor & Grastien, with diagonal steps only between two open
    /// sides, so no corner cutting): A* over the cells where an optimal path may turn, with the
    /// same octile costs (10 straight, 14 diagonal) and the same shortest length as A* over
    /// every cell, at a fraction of the cells touched. Returns every cell of the path.
    fn astar_in(&self, s: &mut AstarScratch, start: usize, goal: usize) -> Vec<usize> {
        let n = self.blocked.len();
        s.begin(n);
        let w = self.grid_w;
        let xy = |i: usize| ((i % w) as isize, (i / w) as isize);
        let (gx, gy) = xy(goal);
        let octile = |dx: isize, dy: isize| -> u32 {
            let (dx, dy) = (dx.unsigned_abs() as u32, dy.unsigned_abs() as u32);
            10 * (dx + dy) - 6 * dx.min(dy)
        };
        let h = |i: usize| {
            let (x, y) = xy(i);
            octile(x - gx, y - gy)
        };
        // Max-heap of Reverse-ordered keys: (f, h, cell) — deterministic tie-breaking.
        let mut open = BinaryHeap::new();
        s.set_g(start, 0, usize::MAX);
        open.push(std::cmp::Reverse((h(start), h(start), start)));
        let mut dirs: Vec<(isize, isize)> = Vec::with_capacity(8);
        while let Some(std::cmp::Reverse((_, _, cur))) = open.pop() {
            if s.closed(cur) {
                continue;
            }
            if cur == goal {
                return self.unpack(s, start, goal);
            }
            s.close(cur);
            let (x, y) = xy(cur);
            dirs.clear();
            let from = s.came[cur] as usize;
            if cur == start {
                dirs.extend([(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)]);
            } else {
                let (px, py) = xy(from);
                let (dx, dy) = ((x - px).signum(), (y - py).signum());
                if dx != 0 && dy != 0 {
                    dirs.extend([(dx, 0), (0, dy), (dx, dy)]);
                } else if dx != 0 {
                    dirs.push((dx, 0));
                    for side in [1, -1] {
                        if self.open_xy(x, y + side) && !self.open_xy(x - dx, y + side) {
                            dirs.extend([(0, side), (dx, side)]);
                        }
                    }
                } else {
                    dirs.push((0, dy));
                    for side in [1, -1] {
                        if self.open_xy(x + side, y) && !self.open_xy(x + side, y - dy) {
                            dirs.extend([(side, 0), (side, dy)]);
                        }
                    }
                }
            }
            for &(dx, dy) in &dirs {
                let Some(jp) = self.jump(x, y, dx, dy, goal) else { continue };
                if s.closed(jp) {
                    continue;
                }
                let (jx, jy) = xy(jp);
                let ng = s.g(cur) + octile(jx - x, jy - y);
                if ng < s.g(jp) {
                    s.set_g(jp, ng, cur);
                    open.push(std::cmp::Reverse((ng + h(jp), h(jp), jp)));
                }
            }
        }
        Vec::new()
    }

    /// The next jump point from (x, y) stepping (dx, dy): the goal, a cell with a forced
    /// neighbor (an optimal path may turn there), or, going diagonally, a cell from which a
    /// straight jump finds one. None when the way is blocked first.
    fn jump(&self, mut x: isize, mut y: isize, dx: isize, dy: isize, goal: usize) -> Option<usize> {
        let w = self.grid_w as isize;
        loop {
            if dx != 0 && dy != 0 && !(self.open_xy(x + dx, y) && self.open_xy(x, y + dy)) {
                return None; // no corner cutting
            }
            x += dx;
            y += dy;
            if !self.open_xy(x, y) {
                return None;
            }
            let i = (y * w + x) as usize;
            if i == goal {
                return Some(i);
            }
            if dx != 0 && dy != 0 {
                if self.jump(x, y, dx, 0, goal).is_some() || self.jump(x, y, 0, dy, goal).is_some() {
                    return Some(i);
                }
            } else if dx != 0 {
                if (self.open_xy(x, y + 1) && !self.open_xy(x - dx, y + 1))
                    || (self.open_xy(x, y - 1) && !self.open_xy(x - dx, y - 1))
                {
                    return Some(i);
                }
            } else if (self.open_xy(x + 1, y) && !self.open_xy(x + 1, y - dy))
                || (self.open_xy(x - 1, y) && !self.open_xy(x - 1, y - dy))
            {
                return Some(i);
            }
        }
    }

    /// The cells of a found path: its jump points, joined by the straight or diagonal runs
    /// between them.
    fn unpack(&self, s: &AstarScratch, start: usize, goal: usize) -> Vec<usize> {
        let w = self.grid_w as isize;
        let mut path = vec![goal];
        let mut c = goal;
        while c != start {
            let p = s.came[c] as usize;
            let (cx, cy, px, py) = ((c as isize % w), (c as isize / w), (p as isize % w), (p as isize / w));
            let (dx, dy) = ((px - cx).signum(), (py - cy).signum());
            let (mut x, mut y) = (cx, cy);
            while (x, y) != (px, py) {
                x += dx;
                y += dy;
                path.push((y * w + x) as usize);
            }
            c = p;
        }
        path.reverse();
        path
    }
}

/// A* bookkeeping kept between searches (per thread): a cell's entries count only when its
/// stamp is the current search's, so a search costs what it explores, not the whole grid.
#[derive(Default)]
struct AstarScratch {
    search: u32,
    stamp: Vec<u32>,
    g: Vec<u32>,
    came: Vec<u32>,
    closed: Vec<u32>,
}

impl AstarScratch {
    fn begin(&mut self, n: usize) {
        if self.stamp.len() != n || self.search == u32::MAX {
            *self = AstarScratch { search: 0, stamp: vec![0; n], g: vec![0; n], came: vec![0; n], closed: vec![0; n] };
        }
        self.search += 1;
    }

    fn g(&self, i: usize) -> u32 {
        if self.stamp[i] == self.search { self.g[i] } else { u32::MAX }
    }

    fn set_g(&mut self, i: usize, g: u32, from: usize) {
        self.stamp[i] = self.search;
        self.g[i] = g;
        self.came[i] = from as u32;
    }

    fn closed(&self, i: usize) -> bool {
        self.closed[i] == self.search
    }

    fn close(&mut self, i: usize) {
        self.closed[i] = self.search;
    }
}

thread_local! {
    static ASTAR: std::cell::RefCell<AstarScratch> = std::cell::RefCell::new(AstarScratch::default());
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
    Map::new(MapId::Arena, walls, brush, Some(Vec2::new(MAP_SIZE, MAP_SIZE)))
}

/// The Bridge is authored along a straight lane (`x` along it, 12,000 u; `y` across, 3,000 u),
/// then laid diagonally on a square map: blue at the bottom left, red at the top right, the lane
/// canted across the screen like the reference ARAM's (06 §2).
pub const BRIDGE_LANE: Vec2 = Vec2::new(12_000.0, 3_000.0);
/// The square the lane is laid on: its bounding box, (12,000 + 3,000) / √2.
pub const BRIDGE_SIZE: Vec2 = Vec2::new(10_607.0, 10_607.0);

/// A point of The Bridge's lane coordinates on the map: a rotation by 45° about the centers,
/// so the lane runs from the bottom left (blue) to the top right (red).
pub fn bridge_point(p: Vec2) -> Vec2 {
    let (u, v) = (p.x - BRIDGE_LANE.x / 2.0, p.y - BRIDGE_LANE.y / 2.0);
    let k = std::f32::consts::FRAC_1_SQRT_2;
    Vec2::new(BRIDGE_SIZE.x / 2.0 + (u + v) * k, BRIDGE_SIZE.y / 2.0 + (v - u) * k)
}

/// The inverse of `bridge_point`: a map point in The Bridge's lane coordinates.
pub fn bridge_lane_point(p: Vec2) -> Vec2 {
    let (x, y) = (p.x - BRIDGE_SIZE.x / 2.0, p.y - BRIDGE_SIZE.y / 2.0);
    let k = std::f32::consts::FRAC_1_SQRT_2;
    Vec2::new(BRIDGE_LANE.x / 2.0 + (x - y) * k, BRIDGE_LANE.y / 2.0 + (x + y) * k)
}

/// The Bridge's spec (an ARAM map, 12 §1): one lane from blue to red with cliffs along both
/// sides, brush alcoves, a rock for cover mid-lane, health relics, and each team's fountain,
/// Base, two base turrets, Gatehouse, gatehouse turret, inner and outer turret. Point-symmetric,
/// so neither side is favored by the layout. The lane is canted on the screen (D54): the
/// camera's up/down asymmetry (D13) gives blue, looking up-screen, a little more view ahead, as
/// in the reference game. Drawn in its lane coordinates (`bridge_point`).
pub const BRIDGE_SPEC: &str = include_str!("../maps/bridge.mftrmap");

fn bridge() -> Map {
    let spec = crate::mapspec::MapSpec::parse(BRIDGE_SPEC).expect("The Bridge's spec reads");
    crate::mapspec::build(&spec, MapId::Bridge)
}

/// Crossroads (M4, the 5v5 map): built from its spec (`maps/crossroads.mftrmap`, 12): three
/// lanes, a river corner to corner and the jungle between, on a 14,500 u square, blue's base in
/// the bottom-left corner and red's, its half-turn image, in the top right.
pub const CROSSROADS_SIZE: Vec2 = Vec2::new(14_500.0, 14_500.0);

/// Crossroads' spec.
pub const CROSSROADS_SPEC: &str = include_str!("../maps/crossroads.mftrmap");

/// Blue's corner coordinates on Crossroads (`dy` measured up from the bottom edge).
pub fn crossroads_point(dx: f32, dy: f32) -> Vec2 {
    Vec2::new(dx, CROSSROADS_SIZE.y - dy)
}

fn crossroads() -> Map {
    let spec = crate::mapspec::MapSpec::parse(CROSSROADS_SPEC).expect("Crossroads' spec reads");
    crate::mapspec::build(&spec, MapId::Crossroads)
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

    /// A built map's geometry, bit for bit: walls, brush, lane paths, spawns, fountains,
    /// structures, relics and camps (with their facing). FNV-1a over every float's bits.
    fn geometry_hash(m: &Map) -> u64 {
        let mut h = 0xcbf2_9ce4_8422_2325u64;
        let mut eat = |x: u64| {
            for b in x.to_le_bytes() {
                h = (h ^ b as u64).wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        let pt = |eat: &mut dyn FnMut(u64), p: Vec2| {
            eat(p.x.to_bits() as u64);
            eat(p.y.to_bits() as u64);
        };
        for poly in m.walls.iter().chain(&m.brush) {
            eat(poly.len() as u64);
            for p in poly {
                pt(&mut eat, *p);
            }
        }
        let l = &m.layout;
        for pair in &l.lanes {
            for path in pair {
                eat(path.len() as u64);
                for p in path {
                    pt(&mut eat, *p);
                }
            }
        }
        for pair in &l.wave_spawn {
            pt(&mut eat, pair[0]);
            pt(&mut eat, pair[1]);
        }
        pt(&mut eat, l.champion_spawn[0]);
        pt(&mut eat, l.champion_spawn[1]);
        for (c, r) in l.fountains.iter().flatten() {
            pt(&mut eat, *c);
            eat(r.to_bits() as u64);
        }
        for p in &l.placements {
            eat(((p.kind as u64) << 24) | ((p.team as u64) << 16) | ((p.tier as u64) << 8) | p.lane as u64);
            pt(&mut eat, p.pos);
        }
        for c in &l.camps {
            eat(c.kind as u64);
            pt(&mut eat, c.pos);
            pt(&mut eat, c.facing);
        }
        h
    }

    /// Cross-platform canary for map building (03 §17): the maps built from specs must come out
    /// bit for bit the same on every OS (CI runs this on Linux, Windows and macOS), since each
    /// client rebuilds the map the server plays. A change on one platform means map building
    /// used platform math; a change everywhere means the build changed (update on purpose).
    #[test]
    fn built_maps_are_the_same_everywhere() {
        let got = [MapId::Crossroads, MapId::Bridge].map(|id| geometry_hash(&id.shared()));
        assert_eq!(got, [GEOMETRY_CROSSROADS, GEOMETRY_BRIDGE], "{got:#018x?}");
    }

    const GEOMETRY_CROSSROADS: u64 = 0x6277_b7c8_01c8_6368;
    const GEOMETRY_BRIDGE: u64 = 0x0428_bc04_0f06_2fdc;

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

    /// The Bridge lies diagonally on its square (blue bottom left, red top right), stays
    /// point-symmetric, and every structure, spawn and relic stands on walkable ground.
    #[test]
    fn the_bridge_is_canted_symmetric_and_walkable() {
        let m = MapId::Bridge.shared();
        let back = bridge_lane_point(bridge_point(Vec2::new(1234.0, 567.0)));
        assert!((back - Vec2::new(1234.0, 567.0)).length() < 0.01);
        let (blue, red) = (m.layout.champion_spawn[0], m.layout.champion_spawn[1]);
        assert!(blue.x < red.x && blue.y > red.y, "blue bottom left, red top right: {blue:?} {red:?}");
        let center = BRIDGE_SIZE * 0.5;
        for p in &m.layout.placements {
            let twin = center * 2.0 - p.pos;
            assert!(
                m.layout
                    .placements
                    .iter()
                    .any(|q| q.kind == p.kind && q.tier == p.tier && (q.pos - twin).length() < 0.1),
                "{p:?} has no mirror image"
            );
            if p.kind != UnitKind::Relic {
                assert!(m.walkable(p.pos + Vec2::new(0.0, 0.0), 1.0), "{p:?} stands in a wall");
            }
        }
        for s in m.layout.champion_spawn.iter().chain(m.layout.wave_spawn.iter().flatten()) {
            assert!(m.in_bounds(*s, 100.0) && m.walkable(*s, 60.0), "{s:?}");
        }
    }

    /// Crossroads: blue bottom left, red top right; turned half a turn (every structure and camp
    /// has its twin; red's top lane is blue's bottom lane turned); its spec has no problems; every lane walks from spawn to the enemy
    /// Base; each lane's turrets fall in order from the river back home.
    #[test]
    fn crossroads_is_turned_walkable_and_ordered() {
        let t = std::time::Instant::now();
        let m = MapId::Crossroads.shared();
        let built = t.elapsed();
        let spec = crate::mapspec::MapSpec::parse(CROSSROADS_SPEC).unwrap();
        assert_eq!(crate::mapspec::problems(&spec), Vec::<String>::new());
        let l = &m.layout;
        assert_eq!((l.lanes.len(), l.wave_spawn.len(), l.camps.len()), (3, 3, 12));
        let (blue, red) = (l.champion_spawn[0], l.champion_spawn[1]);
        assert!(blue.x < red.x && blue.y > red.y, "blue bottom left, red top right: {blue:?} {red:?}");
        let turn = |p: Vec2| CROSSROADS_SIZE - p;
        for p in &l.placements {
            assert!(
                l.placements.iter().any(|q| q.kind == p.kind
                    && q.tier == p.tier
                    && q.team != p.team
                    && (p.tier > crate::lane::GATEHOUSE_TIER || q.lane == 2 - p.lane)
                    && (q.pos - turn(p.pos)).length() < 0.1),
                "{p:?} has no twin"
            );
            assert!(m.walkable(p.pos, 1.0), "{p:?} stands in a wall");
        }
        for c in &l.camps {
            assert!(l.camps.iter().any(|d| d.kind == c.kind && (d.pos - turn(c.pos)).length() < 0.1));
            for (kind, p) in c.spots() {
                assert!(m.walkable(p, kind.def().radius.0), "{c:?}: {kind:?} at {p:?}");
            }
        }
        for (li, lane) in l.lanes.iter().enumerate() {
            for team in [Team::Blue, Team::Red] {
                let from = l.wave_spawn[li][team as usize];
                let to = *lane[team as usize].last().unwrap();
                let path = m.find_path(from, to);
                assert!(path.last().is_some_and(|p| p.distance(to) < 1.0), "lane {li} {team:?}: {path:?}");
                let home = l.champion_spawn[team as usize];
                let mut own: Vec<&Placement> = l
                    .placements
                    .iter()
                    .filter(|p| p.team == team && p.lane as usize == li && p.tier <= crate::lane::GATEHOUSE_TIER)
                    .collect();
                own.sort_by_key(|p| p.tier);
                assert_eq!(own.iter().map(|p| p.tier).collect::<Vec<_>>(), vec![1, 2, 3, 4], "lane {li} {team:?}");
                for w in own.windows(2) {
                    assert!(w[0].pos.distance(home) > w[1].pos.distance(home), "lane {li} {team:?}: {w:?}");
                }
            }
        }
        eprintln!("crossroads: built in {built:?}, {} walls, {} brush", m.walls.len(), m.brush.len());
    }

    /// `n` pairs of points on `m`: a walkable start and any goal (as a click might be).
    fn random_pairs(m: &Map, n: usize) -> Vec<(Vec2, Vec2)> {
        let mut rng = crate::rng::Pcg32::new(7, 1);
        let mut pt = || Vec2::new(rng.range_f32(0.0, m.size.x), rng.range_f32(0.0, m.size.y));
        let mut pairs = Vec::new();
        while pairs.len() < n {
            let (a, b) = (pt(), pt());
            if m.walkable(a, NAV_CLEARANCE) {
                pairs.push((a, b));
            }
        }
        pairs
    }

    /// Paths across Crossroads keep their clearance from walls on every leg, and end at the
    /// goal or, for a goal in a wall, near it.
    #[test]
    fn crossroads_paths_keep_clear_of_walls() {
        let m = MapId::Crossroads.shared();
        for (a, b) in random_pairs(&m, 150) {
            let path = m.find_path(a, b);
            let mut at = a;
            for &p in &path {
                assert!(m.segment_clear(at, p, NAV_CLEARANCE - 1.0), "{a:?} -> {b:?}: {at:?} -> {p:?} clips a wall");
                at = p;
            }
            let end = *path.last().unwrap();
            assert!(end == b || !m.walkable(b, NAV_CLEARANCE), "{a:?} -> {b:?} ends at {end:?}");
        }
    }

    /// Paths between random points: time per path (all of `find_path`, and the grid search
    /// alone) and their total length. `cargo test --release -p mftr-sim path_bench --
    /// --ignored --nocapture`.
    #[test]
    #[ignore]
    fn path_bench() {
        for id in [MapId::Bridge, MapId::Crossroads] {
            let m = id.shared();
            let pairs = random_pairs(&m, 2000);
            let (mut total, mut worst, mut search, mut search_worst, mut length) = (0.0, 0.0f64, 0.0, 0.0f64, 0.0f64);
            for &(a, b) in &pairs {
                let t = std::time::Instant::now();
                let path = m.find_path(a, b);
                let dt = t.elapsed().as_secs_f64() * 1000.0;
                (total, worst) = (total + dt, worst.max(dt));
                let mut at = a;
                for &p in &path {
                    length += at.distance(p) as f64;
                    at = p;
                }
                if let (Some(s), Some(g)) = (m.nearest_open_cell(a), m.nearest_open_cell(b)) {
                    let t = std::time::Instant::now();
                    std::hint::black_box(m.astar(s, g));
                    let dt = t.elapsed().as_secs_f64() * 1000.0;
                    (search, search_worst) = (search + dt, search_worst.max(dt));
                }
            }
            let n = pairs.len() as f64;
            eprintln!(
                "{id:?}: find_path {:.3} ms avg, {:.2} ms worst; search {:.3} ms avg, {:.2} ms worst; length {length:.0}",
                total / n,
                worst,
                search / n,
                search_worst,
            );
        }
    }

    #[test]
    fn pathfinding_is_deterministic() {
        let m = MapId::Arena.build();
        let a = m.find_path(Vec2::new(600.0, 2600.0), Vec2::new(1000.0, 2600.0));
        let b = m.find_path(Vec2::new(600.0, 2600.0), Vec2::new(1000.0, 2600.0));
        assert_eq!(a, b);
    }
}
