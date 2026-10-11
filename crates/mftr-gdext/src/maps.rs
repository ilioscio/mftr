//! Map files for the map editor (12): `MftrMaps` reads, checks and builds `*.mftrmap` text with
//! the simulation's own code, so the editor shows exactly the map the game will build.

use godot::prelude::*;
use mftr_sim::Vec2;
use mftr_sim::map::{Map, MapId};
use mftr_sim::mapspec::{self, Line, MapSpec};

#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrMaps {
    base: Base<RefCounted>,
}

fn v(p: Vec2) -> Vector2 {
    Vector2::new(p.x, p.y)
}

fn points(pts: &[Vec2]) -> PackedVector2Array {
    pts.iter().map(|p| v(*p)).collect()
}

fn polygons(polys: &[Vec<Vec2>]) -> VarArray {
    let mut out = VarArray::new();
    for p in polys {
        out.push(&points(p).to_variant());
    }
    out
}

fn lines(ls: &[Line]) -> VarArray {
    let mut out = VarArray::new();
    for l in ls {
        let mut d = VarDictionary::new();
        d.set("width", l.width);
        d.set("points", &points(&l.points));
        out.push(&d.to_variant());
    }
    out
}

#[godot_api]
impl MftrMaps {
    /// The built-in Crossroads' text (a starting point to edit).
    #[func]
    fn crossroads() -> GString {
        mftr_sim::map::CROSSROADS_SPEC.into()
    }

    /// The map sizes there are (small, medium, large).
    #[func]
    fn sizes() -> PackedFloat32Array {
        mapspec::SIZES.iter().copied().collect()
    }

    /// Read map text: `{ ok, error }` and, when it reads, the spec as data: `name`, `size`,
    /// `lane`/`river`/`jungle`/`grass` (each `[{ width, points }]`), `clearings`
    /// (`[{ pos, radius }]`), `camps` (`[{ kind, pos }]`), `objectives` (points) and
    /// `structures` (`[{ lane, tier, distance }]`).
    #[func]
    fn parse(text: GString) -> VarDictionary {
        let mut d = VarDictionary::new();
        let spec = match MapSpec::parse(&text.to_string()) {
            Ok(s) => s,
            Err(e) => {
                d.set("ok", false);
                d.set("error", e.as_str());
                return d;
            }
        };
        d.set("ok", true);
        d.set("name", spec.name.as_str());
        d.set("size", spec.size);
        d.set("lane", &lines(&spec.lanes));
        d.set("river", &lines(&spec.river));
        d.set("jungle", &lines(&spec.jungle));
        d.set("grass", &lines(&spec.grass));
        let mut clearings = VarArray::new();
        for (c, r) in &spec.clearings {
            let mut e = VarDictionary::new();
            e.set("pos", v(*c));
            e.set("radius", *r);
            clearings.push(&e.to_variant());
        }
        d.set("clearings", &clearings);
        let mut camps = VarArray::new();
        for (k, p) in &spec.camps {
            let mut e = VarDictionary::new();
            e.set("kind", mapspec::camp_key(*k));
            e.set("pos", v(*p));
            camps.push(&e.to_variant());
        }
        d.set("camps", &camps);
        let objectives: PackedVector2Array = spec.objectives.iter().map(|p| v(*p)).collect();
        d.set("objectives", &objectives);
        let mut structures = VarArray::new();
        for (lane, tier, dist) in &spec.structures {
            let mut e = VarDictionary::new();
            e.set("lane", *lane as i64);
            e.set("tier", *tier as i64);
            e.set("distance", *dist);
            structures.push(&e.to_variant());
        }
        d.set("structures", &structures);
        d
    }

    /// Everything wrong with map text, in words (a read error is one); empty when it can be
    /// exported and played.
    #[func]
    fn problems(text: GString) -> PackedStringArray {
        match MapSpec::parse(&text.to_string()) {
            Ok(spec) => mapspec::problems(&spec).iter().map(|s| GString::from(s.as_str())).collect(),
            Err(e) => [GString::from(e.as_str())].into_iter().collect(),
        }
    }

    /// The map the game builds from map text, for drawing: `walls` and `brush` (polygons),
    /// `size`, `lanes` (blue's full paths, top first), `structures` (`[{ kind, team, tier, lane,
    /// spec_lane, pos }]`),
    /// `camps` (`[{ kind, pos }]`, both halves), `fountains` (centers), `river` (the river's
    /// course) and `ok` (false when the text doesn't read).
    #[func]
    fn build(text: GString) -> VarDictionary {
        let mut d = VarDictionary::new();
        let Ok(spec) = MapSpec::parse(&text.to_string()) else {
            d.set("ok", false);
            return d;
        };
        let map: Map = mapspec::build(&spec, MapId::Custom);
        d.set("ok", true);
        d.set("size", spec.size);
        d.set("walls", &polygons(&map.walls));
        d.set("brush", &polygons(&map.brush));
        let l = &map.layout;
        let mut lanes = VarArray::new();
        for lane in 0..spec.lanes.len() {
            lanes.push(&points(&spec.lane_path(lane)).to_variant());
        }
        d.set("lanes", &lanes);
        let order = spec.lane_order();
        let mut structures = VarArray::new();
        for p in &l.placements {
            let mut e = VarDictionary::new();
            e.set(
                "kind",
                match p.kind {
                    mftr_sim::UnitKind::Gatehouse => "gatehouse",
                    mftr_sim::UnitKind::Base => "base",
                    _ => "turret",
                },
            );
            e.set("team", if p.team == mftr_sim::Team::Blue { "blue" } else { "red" });
            e.set("tier", p.tier as i64);
            e.set("lane", p.lane as i64);
            // The lane's index in the spec (as drawn), for moving blue's structures.
            e.set("spec_lane", order.get(p.lane as usize).copied().unwrap_or(0) as i64);
            e.set("pos", v(p.pos));
            structures.push(&e.to_variant());
        }
        d.set("structures", &structures);
        let mut camps = VarArray::new();
        for c in &l.camps {
            let mut e = VarDictionary::new();
            e.set("kind", mapspec::camp_key(c.kind));
            e.set("pos", v(c.pos));
            camps.push(&e.to_variant());
        }
        d.set("camps", &camps);
        let fountains: PackedVector2Array = l.fountains.iter().flatten().map(|(c, _)| v(*c)).collect();
        d.set("fountains", &fountains);
        d.set("river", &points(&l.river));
        d.set("base_side", spec.base_side());
        d
    }
}
