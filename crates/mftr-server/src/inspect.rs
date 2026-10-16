// SPDX-License-Identifier: AGPL-3.0-or-later
//! The inspect window's answers (01 §13): a unit's live stats, inventory, augments and what's on
//! it, for a player whose team sees it (spectators see everything).

use mftr_net::msg::{InfoEffect, InfoEffectKind, MAX_INFO_EFFECTS, UnitInfo};
use mftr_sim::SimTime;
use mftr_sim::world::{Team, UnitId, UnitKind, World};

/// `id`'s details as `team` may see them now (`None` for a spectator: everything), or `None` if
/// it's gone or hidden from that team.
pub fn unit_info(world: &World, id: UnitId, team: Option<Team>) -> Option<UnitInfo> {
    if team.is_some_and(|t| world.hidden(t).binary_search(&id).is_ok()) {
        return None;
    }
    let u = world.units().iter().find(|u| u.id == id)?;
    let t = SimTime::end_of(world.tick());
    let st = &u.state;
    let p = &st.progress;
    let whole = |v: f32| v.round().clamp(0.0, u16::MAX as f32) as u16;
    let tenths = |until: SimTime| (until.secs_since(t) * 10.0).ceil().min(u16::MAX as f32) as u16;
    let mut effects = Vec::new();
    let mut add = |kind, tenths: u16, value: u16| {
        if effects.len() < MAX_INFO_EFFECTS {
            effects.push(InfoEffect { kind, tenths, value });
        }
    };
    if let Some(at) = st.respawn_at.filter(|_| !st.alive()) {
        add(InfoEffectKind::Dead, tenths(at), 0);
    }
    if world.game().horde.as_ref().and_then(|h| h.boss) == Some(id) {
        add(InfoEffectKind::Boss, 0, 0);
    }
    if st.stunned_until > t {
        add(InfoEffectKind::Stunned, tenths(st.stunned_until), 0);
    }
    if st.rooted_until > t {
        add(InfoEffectKind::Rooted, tenths(st.rooted_until), 0);
    }
    if st.slow > 0 && st.slowed_until > t {
        add(InfoEffectKind::Slowed, tenths(st.slowed_until), st.slow as u16);
    }
    if st.shield > 0.0 && st.shield_until > t {
        add(InfoEffectKind::Shield, tenths(st.shield_until), whole(st.shield));
    }
    if st.potion_until > t {
        add(InfoEffectKind::Potion, tenths(st.potion_until), whole(st.potion_rate));
    }
    if st.recalling() {
        add(InfoEffectKind::Recalling, tenths(st.recall_until), 0);
    }
    if st.spellblade_until > t {
        add(InfoEffectKind::Spellblade, tenths(st.spellblade_until), 0);
    }
    if p.insight_until > t {
        add(InfoEffectKind::Insight, tenths(p.insight_until), 0);
    }
    if p.cinder_until > t {
        add(InfoEffectKind::Cinder, tenths(p.cinder_until), 0);
    }
    if p.stacks > 0 {
        add(InfoEffectKind::Stacks, 0, p.stacks);
    }
    if u.kind.is_structure() {
        if u.protected {
            add(InfoEffectKind::Protected, 0, 0);
        }
        if u.kind == UnitKind::Turret && u.plates > 0 {
            add(InfoEffectKind::Plates, 0, u.plates as u16);
        }
        if (u.champion_damage_taken - 1.0).abs() > 0.005 {
            add(InfoEffectKind::ChampionDamage, 0, whole(u.champion_damage_taken * 100.0));
        }
    }
    let attack = u.attack;
    Some(UnitInfo {
        unit: id,
        attack_damage: whole(u.stats.attack_damage),
        ability_power: whole(u.stats.ability_power),
        armor: whole(u.stats.armor),
        magic_resist: whole(u.stats.magic_resist),
        attack_speed_milli: attack.map_or(0, |a| whole(a.attack_speed * 1000.0)),
        attack_range: attack.map_or(0, |a| whole(a.range)),
        move_speed: whole(u.stats.move_speed),
        ability_haste: whole(u.stats.ability_haste),
        health_regen_tenths: whole(u.stats.health_regen * 10.0),
        life_steal_pct: (u.stats.life_steal * 100.0).round().clamp(0.0, 255.0) as u8,
        items: p.items,
        augments: p.augments,
        anvils: p.anvil,
        effects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_sim::ChampionId;
    use mftr_sim::map::MapId;
    use mftr_sim::world::{PlayerId, Rules};

    /// A champion's details are its live stats, and a team that can't see it gets nothing.
    #[test]
    fn answers_with_live_stats_and_hides_what_the_team_cannot_see() {
        let mut w = World::new(3);
        w.set_map(MapId::Crossroads.shared());
        w.set_rules(Rules::CLASSIC);
        w.start_match();
        let home = w.map().layout.champion_spawn[1];
        let id = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Vesper, home);
        w.step(&[]);
        let u = w.unit(id).unwrap().clone();
        let info = unit_info(&w, id, None).expect("a spectator sees everything");
        assert_eq!(info.unit, id);
        assert_eq!(info.armor, u.stats.armor.round() as u16);
        assert_eq!(info.attack_range, u.attack.unwrap().range.round() as u16);
        assert!(info.attack_speed_milli > 0 && info.move_speed > 0);
        assert!(unit_info(&w, id, Some(Team::Red)).is_some());
        let mut hidden = w.hidden(Team::Blue).to_vec();
        hidden.push(id);
        w.set_hidden(Team::Blue, hidden);
        assert!(unit_info(&w, id, Some(Team::Blue)).is_none());
        assert!(unit_info(&w, UnitId(999_999), None).is_none());
    }

    /// A turret shows its plates; an inner one shows it's protected.
    #[test]
    fn structures_show_plates_and_protection() {
        let mut w = World::new(3);
        w.set_map(MapId::Crossroads.shared());
        w.set_rules(Rules::CLASSIC);
        w.start_match();
        w.step(&[]);
        let turrets: Vec<_> =
            w.units().iter().filter(|u| u.kind == UnitKind::Turret).map(|u| (u.id, u.protected)).collect();
        let kinds = |id| unit_info(&w, id, None).unwrap().effects.iter().map(|e| e.kind).collect::<Vec<_>>();
        let (outer, _) = turrets.iter().find(|(_, p)| !p).unwrap();
        let (inner, _) = turrets.iter().find(|(_, p)| *p).unwrap();
        assert!(kinds(*outer).contains(&InfoEffectKind::Plates), "{:?}", kinds(*outer));
        assert!(kinds(*inner).contains(&InfoEffectKind::Protected), "{:?}", kinds(*inner));
    }
}
