//! What a map is, to a bot (17 §2): its lanes as full paths, travel distances that respect walls,
//! brush and ward spots. Built once per map and shared by every match on it, so a server pays
//! for it once.
//!
//! Travel distances come from fields: for each key point (a structure, a camp, a fountain, points
//! along each lane), the walking distance from every cell of a coarse grid, found by a Dijkstra
//! search the first time that key is asked about. A bot never runs the pathfinder.

use mftr_sim::Vec2;
use mftr_sim::jungle::Camp;
use mftr_sim::map::Map;
use mftr_sim::world::Team;
use std::collections::BinaryHeap;
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// The coarse grid's cell (u).
const CELL: f32 = 80.0;
/// Clearance a cell's center needs to count as open.
const OPEN_CLEARANCE: f32 = 20.0;
/// Key points along each lane, this far apart.
const LANE_KEY_EVERY: f32 = 1200.0;
/// Unreachable, in a field.
const FAR: u16 = u16::MAX;
/// A field stores distances in steps of this many units.
const FIELD_UNIT: f32 = 4.0;

/// Where a ward does the most good.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WardSpot {
    pub pos: Vec2,
    /// The side of the map it's on (`None`: the middle, by the river).
    pub side: Option<Team>,
    /// The lane it watches (the nearest), if any is within reach.
    pub lane: Option<usize>,
}

pub struct Knowledge {
    pub size: Vec2,
    /// Each lane's path per team, from the wave's spawn to the enemy's base.
    pub lanes: Vec<[Vec<Vec2>; 2]>,
    pub fountains: [Option<Vec2>; 2],
    pub homes: [Vec2; 2],
    /// Brush patches: center and rough radius.
    pub brush: Vec<(Vec2, f32)>,
    pub camps: Vec<Camp>,
    pub ward_spots: Vec<WardSpot>,
    /// The river's course, if any.
    pub river: Vec<Vec2>,
    cols: usize,
    rows: usize,
    open: Vec<bool>,
    keys: Vec<Vec2>,
    fields: Vec<OnceLock<Box<[u16]>>>,
}

static CACHE: Mutex<Vec<(Weak<Map>, Arc<Knowledge>)>> = Mutex::new(Vec::new());

impl Knowledge {
    /// The knowledge of `map`, built on first use and shared while the map lives.
    pub fn of(map: &Arc<Map>) -> Arc<Knowledge> {
        let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
        cache.retain(|(m, _)| m.strong_count() > 0);
        if let Some((_, k)) = cache.iter().find(|(m, _)| m.as_ptr() == Arc::as_ptr(map)) {
            return k.clone();
        }
        let k = Arc::new(Knowledge::build(map));
        cache.push((Arc::downgrade(map), k.clone()));
        k
    }

    fn build(map: &Map) -> Knowledge {
        let layout = &map.layout;
        let size = map.size;
        let lanes: Vec<[Vec<Vec2>; 2]> = layout
            .lanes
            .iter()
            .enumerate()
            .map(|(li, paths)| {
                [0, 1].map(|t| {
                    let mut p = Vec::with_capacity(paths[t].len() + 1);
                    if let Some(spawn) = layout.wave_spawn.get(li) {
                        p.push(spawn[t]);
                    }
                    p.extend(paths[t].iter().copied());
                    p
                })
            })
            .collect();
        let fountains = layout.fountains.map(|f| f.map(|(c, _)| c));
        let homes = layout.champion_spawn;
        let brush: Vec<(Vec2, f32)> = map
            .brush
            .iter()
            .filter(|b| !b.is_empty())
            .map(|b| {
                let c = b.iter().fold(Vec2::ZERO, |s, v| s + *v) * (1.0 / b.len() as f32);
                let r = b.iter().map(|v| v.distance(c)).fold(0.0, f32::max);
                (c, r)
            })
            .collect();
        let (cols, rows) = ((size.x / CELL).ceil().max(1.0) as usize, (size.y / CELL).ceil().max(1.0) as usize);
        let open: Vec<bool> = (0..cols * rows)
            .map(|i| {
                let c = Vec2::new(((i % cols) as f32 + 0.5) * CELL, ((i / cols) as f32 + 0.5) * CELL);
                map.walkable(c, OPEN_CLEARANCE)
            })
            .collect();
        let mut keys: Vec<Vec2> = Vec::new();
        keys.extend(layout.placements.iter().map(|p| p.pos));
        keys.extend(layout.camps.iter().map(|c| c.pos));
        keys.extend(fountains.iter().flatten().copied());
        keys.extend(homes);
        for lane in &lanes {
            let path = &lane[0];
            let mut walked = 0.0;
            let mut next = 0.0;
            for w in path.windows(2) {
                let len = w[0].distance(w[1]);
                while next <= walked + len {
                    keys.push(w[0].lerp(w[1], ((next - walked) / len.max(1e-3)).clamp(0.0, 1.0)));
                    next += LANE_KEY_EVERY;
                }
                walked += len;
            }
            if let Some(&end) = path.last() {
                keys.push(end);
            }
        }
        let ward_spots = ward_spots(map, &lanes, &brush);
        let fields = (0..keys.len()).map(|_| OnceLock::new()).collect();
        Knowledge {
            size,
            lanes,
            fountains,
            homes,
            brush,
            camps: layout.camps.clone(),
            ward_spots,
            river: layout.river.clone(),
            cols,
            rows,
            open,
            keys,
            fields,
        }
    }

    fn cell(&self, p: Vec2) -> usize {
        let x = ((p.x / CELL).floor().max(0.0) as usize).min(self.cols - 1);
        let y = ((p.y / CELL).floor().max(0.0) as usize).min(self.rows - 1);
        y * self.cols + x
    }

    /// The open cell nearest `p`'s (itself if open), searched in growing rings.
    fn open_cell(&self, p: Vec2) -> Option<usize> {
        let c = self.cell(p);
        if self.open[c] {
            return Some(c);
        }
        let (cx, cy) = ((c % self.cols) as isize, (c / self.cols) as isize);
        for r in 1..12isize {
            let mut best: Option<(f32, usize)> = None;
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    let (x, y) = (cx + dx, cy + dy);
                    if x < 0 || y < 0 || x as usize >= self.cols || y as usize >= self.rows {
                        continue;
                    }
                    let i = y as usize * self.cols + x as usize;
                    if self.open[i] {
                        let d = (dx * dx + dy * dy) as f32;
                        if best.is_none_or(|(bd, _)| d < bd) {
                            best = Some((d, i));
                        }
                    }
                }
            }
            if let Some((_, i)) = best {
                return Some(i);
            }
        }
        None
    }

    fn field(&self, key: usize) -> &[u16] {
        self.fields[key].get_or_init(|| self.dijkstra(self.keys[key]))
    }

    /// Walking distance (in `FIELD_UNIT`s) from every cell to `to`, around walls.
    fn dijkstra(&self, to: Vec2) -> Box<[u16]> {
        let mut dist = vec![FAR; self.cols * self.rows].into_boxed_slice();
        let Some(start) = self.open_cell(to) else { return dist };
        let straight = (CELL / FIELD_UNIT).round() as u32;
        let diagonal = (CELL * std::f32::consts::SQRT_2 / FIELD_UNIT).round() as u32;
        let mut heap = BinaryHeap::new();
        dist[start] = 0;
        heap.push(std::cmp::Reverse((0u32, start)));
        while let Some(std::cmp::Reverse((d, i))) = heap.pop() {
            if d > dist[i] as u32 {
                continue;
            }
            let (x, y) = ((i % self.cols) as isize, (i / self.cols) as isize);
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)] {
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0 || ny < 0 || nx as usize >= self.cols || ny as usize >= self.rows {
                    continue;
                }
                let n = ny as usize * self.cols + nx as usize;
                if !self.open[n] {
                    continue;
                }
                let diag = dx != 0 && dy != 0;
                // No cutting a wall's corner.
                if diag
                    && (!self.open[y as usize * self.cols + nx as usize]
                        || !self.open[ny as usize * self.cols + x as usize])
                {
                    continue;
                }
                let nd = d + if diag { diagonal } else { straight };
                if nd < dist[n] as u32 && nd < FAR as u32 {
                    dist[n] = nd as u16;
                    heap.push(std::cmp::Reverse((nd, n)));
                }
            }
        }
        dist
    }

    /// The key point nearest `p` (straight line).
    fn key_near(&self, p: Vec2) -> usize {
        let mut best = (f32::INFINITY, 0);
        for (i, k) in self.keys.iter().enumerate() {
            let d = k.distance(p);
            if d < best.0 {
                best = (d, i);
            }
        }
        best.1
    }

    /// Walking distance from `from` to `to`, around walls: through the key point nearest `to`,
    /// so a little more than the true distance for a `to` far from every key. Straight-line for
    /// maps without lanes.
    pub fn travel(&self, from: Vec2, to: Vec2) -> f32 {
        if self.keys.is_empty() {
            return from.distance(to);
        }
        let key = self.key_near(to);
        let last = self.keys[key].distance(to);
        let d = self.open_cell(from).map_or(FAR, |c| self.field(key)[c]);
        if d == FAR {
            return from.distance(to) * 1.5;
        }
        (d as f32 * FIELD_UNIT + last).max(from.distance(to))
    }

    /// How far along lane `lane`'s path for `team` the point nearest `p` is, the path's
    /// direction there, and how far `p` is from the path.
    pub fn along(&self, lane: usize, team: Team, p: Vec2) -> (f32, Vec2, f32) {
        let Some(path) = self.lanes.get(lane).map(|l| &l[team as usize]) else {
            return (0.0, Vec2::new(1.0, 0.0), 0.0);
        };
        along(path, p)
    }

    /// The point `dist` along lane `lane`'s path for `team` (clamped to its ends).
    pub fn lane_point(&self, lane: usize, team: Team, dist: f32) -> Vec2 {
        let Some(path) = self.lanes.get(lane).map(|l| &l[team as usize]) else { return self.size * 0.5 };
        let mut walked = 0.0;
        for w in path.windows(2) {
            let len = w[0].distance(w[1]);
            if walked + len >= dist {
                return w[0].lerp(w[1], ((dist - walked) / len.max(1e-3)).clamp(0.0, 1.0));
            }
            walked += len;
        }
        path.last().copied().unwrap_or(self.size * 0.5)
    }

    /// A lane path's length.
    pub fn lane_length(&self, lane: usize, team: Team) -> f32 {
        self.lanes.get(lane).map_or(0.0, |l| l[team as usize].windows(2).map(|w| w[0].distance(w[1])).sum())
    }

    /// The lane whose path passes nearest `p`, and how far.
    pub fn nearest_lane(&self, p: Vec2) -> Option<(usize, f32)> {
        (0..self.lanes.len()).map(|l| (l, self.along(l, Team::Blue, p).2)).min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Inside a brush patch (roughly).
    pub fn in_brush(&self, p: Vec2) -> bool {
        self.brush.iter().any(|(c, r)| c.distance(p) < r * 0.8)
    }

    /// The side of the map `p` is on: nearer one team's home than the other's by a margin.
    pub fn side_of(&self, p: Vec2) -> Option<Team> {
        let (b, r) = (p.distance(self.homes[0]), p.distance(self.homes[1]));
        if b + 600.0 < r {
            Some(Team::Blue)
        } else if r + 600.0 < b {
            Some(Team::Red)
        } else {
            None
        }
    }
}

/// How far along `path` the point nearest `p` is, the path's direction there, and the distance
/// from `p` to the path.
pub fn along(path: &[Vec2], p: Vec2) -> (f32, Vec2, f32) {
    let mut best = (f32::INFINITY, 0.0, Vec2::new(1.0, 0.0));
    let mut walked = 0.0;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let seg = b - a;
        let len = seg.length().max(1e-3);
        let dir = seg * (1.0 / len);
        let t = (p - a).dot(dir).clamp(0.0, len);
        let d = (a + dir * t).distance(p);
        if d < best.0 {
            best = (d, walked + t, dir);
        }
        walked += len;
    }
    if path.len() == 1 {
        return (0.0, Vec2::new(1.0, 0.0), path[0].distance(p));
    }
    (best.1, best.2, best.0)
}

/// Ward spots: brush near a lane or the river (where ganks come from), and river points between
/// lanes.
fn ward_spots(map: &Map, lanes: &[[Vec<Vec2>; 2]], brush: &[(Vec2, f32)]) -> Vec<WardSpot> {
    if !map.layout.wards {
        return Vec::new();
    }
    let homes = map.layout.champion_spawn;
    let side = |p: Vec2| {
        let (b, r) = (p.distance(homes[0]), p.distance(homes[1]));
        if b + 600.0 < r {
            Some(Team::Blue)
        } else if r + 600.0 < b {
            Some(Team::Red)
        } else {
            None
        }
    };
    let nearest_lane = |p: Vec2| {
        lanes
            .iter()
            .enumerate()
            .map(|(i, l)| (i, along(&l[0], p).2))
            .filter(|(_, d)| *d < 2500.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    };
    let mut spots: Vec<WardSpot> =
        brush.iter().map(|(c, _)| WardSpot { pos: *c, side: side(*c), lane: nearest_lane(*c) }).collect();
    // River points, a third and two thirds of the way between lanes.
    let river = &map.layout.river;
    if river.len() >= 2 {
        let len: f32 = river.windows(2).map(|w| w[0].distance(w[1])).sum();
        for f in [0.2, 0.35, 0.65, 0.8] {
            let mut walked = 0.0;
            let target = len * f;
            for w in river.windows(2) {
                let l = w[0].distance(w[1]);
                if walked + l >= target {
                    let p = w[0].lerp(w[1], ((target - walked) / l.max(1e-3)).clamp(0.0, 1.0));
                    if map.walkable(p, 30.0) {
                        spots.push(WardSpot { pos: p, side: None, lane: nearest_lane(p) });
                    }
                    break;
                }
                walked += l;
            }
        }
    }
    spots
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::map::MapId;

    #[test]
    fn travel_goes_around_walls_and_is_shared_per_map() {
        let map = MapId::Crossroads.shared();
        let k = Knowledge::of(&map);
        assert!(Arc::ptr_eq(&k, &Knowledge::of(&map)), "built once per map");
        let (blue, red) = (k.homes[0], k.homes[1]);
        let d = k.travel(blue, red);
        assert!(d >= blue.distance(red) && d < blue.distance(red) * 1.6, "{d} vs {}", blue.distance(red));
        // A camp is reachable from home, and nearer from our side's camps than the enemy's.
        let camp = k.camps.iter().min_by(|a, b| a.pos.distance(blue).total_cmp(&b.pos.distance(blue))).unwrap();
        assert!(k.travel(blue, camp.pos) < k.travel(red, camp.pos));
        assert!(!k.ward_spots.is_empty());
        let (len, _, off) = k.along(1, Team::Blue, k.lane_point(1, Team::Blue, 3000.0));
        assert!((len - 3000.0).abs() < 1.0 && off < 1.0);
    }
}
