//! Team vision (03 §10): computed on the server every tick and used to cull what each client
//! receives, so hidden information never reaches a client (map hacks impossible by construction).
//!
//! Rules (M1):
//! - Each unit of a team is a vision source with a radius by kind.
//! - Walls block line of sight.
//! - A point inside a brush polygon is visible only to sources inside the same brush.
//! - Structures and relics are always visible to everyone, like structures in the reference game.
//! - Wards (M4) are vision sources too. An enemy stealth ward is seen only where a control ward
//!   or a sweep reveals it, and gives its own team no vision there.

use crate::map::Map;
use crate::math::Vec2;
use crate::time::SimTime;
use crate::wards::WardKind;
use crate::world::{Team, Unit, UnitKind, World};

pub const VISION_CHAMPION: f32 = 1200.0;
pub const VISION_MINION: f32 = 900.0;
pub const VISION_TURRET: f32 = 1300.0;

pub const VISION_STRUCTURE: f32 = 800.0;

pub fn vision_radius(kind: UnitKind) -> f32 {
    match kind {
        UnitKind::Champion => VISION_CHAMPION,
        UnitKind::Minion => VISION_MINION,
        UnitKind::RigTurret | UnitKind::Turret => VISION_TURRET,
        UnitKind::Gatehouse | UnitKind::Base => VISION_STRUCTURE,
        UnitKind::Relic | UnitKind::Monster => 0.0,
        UnitKind::Ward => crate::wards::WARD_VISION,
    }
}

/// One team's vision at one instant.
#[derive(Clone, Debug)]
pub struct Vision {
    pub team: Team,
    sources: Vec<(Vec2, f32, Option<u8>)>,
    /// Where the team reveals enemy stealth wards (control wards, sweeps).
    reveal: Vec<(Vec2, f32)>,
}

impl Vision {
    pub fn of(world: &World, team: Team) -> Self {
        let map = world.map();
        let now = SimTime::end_of(world.tick());
        // The enemy disables our stealth wards where it reveals them: they see nothing there.
        let disabled = crate::wards::revealers(world.units(), crate::wards::enemy_of(team), now);
        let sources = world
            .units()
            .iter()
            .filter(|u| u.team == team && u.state.alive() && vision_radius(u.kind) > 0.0)
            .filter(|u| u.ward != Some(WardKind::Stealth) || !covered(&disabled, u.state.pos))
            .map(|u| (u.state.pos, vision_radius(u.kind), map.brush_at(u.state.pos)))
            .collect();
        let reveal = crate::wards::revealers(world.units(), team, now);
        Vision { team, sources, reveal }
    }

    /// Whether the team sees the point `p`.
    pub fn sees(&self, map: &Map, p: Vec2) -> bool {
        let brush = map.brush_at(p);
        self.sources.iter().any(|&(s, r, sb)| {
            (s - p).length_sq() <= r * r && (brush.is_none() || sb == brush) && map.line_of_sight(s, p)
        })
    }

    /// Whether the team sees `unit` (own units and structures always; dead units never).
    pub fn sees_unit(&self, map: &Map, unit: &Unit) -> bool {
        if !unit.state.alive() {
            return false;
        }
        if unit.team == self.team || unit.kind.is_structure() {
            return true;
        }
        // An enemy stealth ward only where we reveal it.
        if unit.ward == Some(WardKind::Stealth) {
            return covered(&self.reveal, unit.state.pos);
        }
        self.sees(map, unit.state.pos)
    }
}

fn covered(areas: &[(Vec2, f32)], p: Vec2) -> bool {
    areas.iter().any(|&(c, r)| (c - p).length_sq() <= r * r)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::champion::ChampionId;
    use crate::map::MapId;
    use crate::world::PlayerId;

    fn world() -> World {
        let mut w = World::new(1);
        w.set_map(MapId::Arena.shared());
        w
    }

    #[test]
    fn walls_and_range_hide_units() {
        let mut w = world();
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(2200.0, 1200.0));
        let behind_wall = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(2700.0, 1200.0));
        let in_view = w.spawn_champion(PlayerId(2), Team::Red, ChampionId::Ember, Vec2::new(2200.0, 1900.0));
        let far = w.spawn_champion(PlayerId(3), Team::Red, ChampionId::Ember, Vec2::new(3800.0, 3800.0));
        let v = Vision::of(&w, Team::Blue);
        let seen = |id| v.sees_unit(w.map(), w.unit(id).unwrap());
        assert!(!seen(behind_wall));
        assert!(seen(in_view));
        assert!(!seen(far));
    }

    #[test]
    fn brush_hides_unless_you_share_it() {
        let mut w = world();
        let bush_center = Vec2::new(1800.0, 2530.0); // brush 0
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1800.0, 2200.0)); // outside, close
        let hider = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, bush_center);
        let v = Vision::of(&w, Team::Blue);
        assert!(!v.sees_unit(w.map(), w.unit(hider).unwrap()), "hidden in brush");
        // Step into the same brush: now visible.
        w.spawn_champion(PlayerId(2), Team::Blue, ChampionId::Ember, bush_center + Vec2::new(60.0, 0.0));
        let v = Vision::of(&w, Team::Blue);
        assert!(v.sees_unit(w.map(), w.unit(hider).unwrap()), "same brush sees it");
        // And the one in the brush still sees out.
        let red = Vision::of(&w, Team::Red);
        assert!(red.sees(w.map(), Vec2::new(1800.0, 2200.0)));
    }
}
