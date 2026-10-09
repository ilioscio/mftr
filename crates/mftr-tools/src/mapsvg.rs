//! `mftr-tools map svg NAME`: a map's layout as an SVG, for reviewing a map's design: walls,
//! brush, lanes, fountains, wave spawns and structures (blue and red, labeled by lane and tier).

use mftr_sim::map::{Map, MapId};
use mftr_sim::{Team, UnitKind};
use std::fmt::Write;

pub fn by_name(name: &str) -> Option<MapId> {
    Some(match name {
        "open" => MapId::Open,
        "arena" => MapId::Arena,
        "bridge" => MapId::Bridge,
        "crossroads" => MapId::Crossroads,
        _ => return None,
    })
}

pub fn render(map: &Map) -> String {
    let (w, h) = (map.size.x, map.size.y);
    let mut s = String::new();
    let _ = writeln!(
        s,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="1200" height="{}">"#,
        1200.0 * h / w
    );
    let _ = writeln!(s, r##"<rect width="{w}" height="{h}" fill="#7d8c5a"/>"##);
    let points =
        |p: &[mftr_sim::math::Vec2]| p.iter().map(|v| format!("{:.0},{:.0}", v.x, v.y)).collect::<Vec<_>>().join(" ");
    for wall in &map.walls {
        let _ = writeln!(s, r##"<polygon points="{}" fill="#3b3a35"/>"##, points(wall));
    }
    for b in &map.brush {
        let _ = writeln!(s, r##"<polygon points="{}" fill="#2f6b2a" opacity="0.8"/>"##, points(b));
    }
    let team_color = |t: Team| if t == Team::Blue { "#3a7bd5" } else { "#d5453a" };
    for (i, f) in map.layout.fountains.iter().enumerate() {
        if let Some((c, r)) = f {
            let color = team_color(if i == 0 { Team::Blue } else { Team::Red });
            let _ = writeln!(s, r#"<circle cx="{}" cy="{}" r="{r}" fill="{color}" opacity="0.35"/>"#, c.x, c.y);
        }
    }
    for lane in &map.layout.lanes {
        for (t, path) in lane.iter().enumerate() {
            let team = if t == 0 { Team::Blue } else { Team::Red };
            let _ = writeln!(
                s,
                r#"<polyline points="{}" fill="none" stroke="{}" stroke-width="40" stroke-dasharray="200 150" opacity="0.7"/>"#,
                points(path),
                team_color(team)
            );
        }
    }
    for spawns in &map.layout.wave_spawn {
        for (t, p) in spawns.iter().enumerate() {
            let color = team_color(if t == 0 { Team::Blue } else { Team::Red });
            let _ = writeln!(
                s,
                r##"<circle cx="{}" cy="{}" r="90" fill="none" stroke="{color}" stroke-width="30"/>"##,
                p.x, p.y
            );
        }
    }
    for p in &map.layout.placements {
        let (r, label) = match p.kind {
            UnitKind::Turret => (110.0, format!("{}{}", ["T", "M", "B"].get(p.lane as usize).unwrap_or(&"?"), p.tier)),
            UnitKind::Gatehouse => (170.0, format!("G{}", p.lane)),
            UnitKind::Base => (300.0, "BASE".into()),
            _ => (80.0, String::new()),
        };
        let color = team_color(p.team);
        let _ = writeln!(
            s,
            r##"<circle cx="{}" cy="{}" r="{r}" fill="{color}" stroke="#fff" stroke-width="15"/>"##,
            p.pos.x, p.pos.y
        );
        let _ = writeln!(
            s,
            r##"<text x="{}" y="{}" font-size="170" fill="#fff" font-family="sans-serif">{label}</text>"##,
            p.pos.x + r + 20.0,
            p.pos.y + 60.0
        );
    }
    s.push_str("</svg>\n");
    s
}
