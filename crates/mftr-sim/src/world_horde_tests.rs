use super::*;
use crate::horde::{GOAL, Wave};

fn world() -> World {
    let mut w = World::new(7);
    w.set_map(MapId::HordeBridge.shared());
    w.set_rules(Rules::HORDE);
    w.start_match();
    for (i, champion) in ChampionId::ALL[..5].iter().enumerate() {
        w.spawn_champion(PlayerId(2 * i as u8), Team::Blue, *champion, w.map.layout.champion_spawn[0]);
    }
    w
}

fn kill_minions(w: &mut World) {
    let ids: Vec<_> = w.units.iter().filter(|u| u.kind == UnitKind::Minion).map(|u| u.id).collect();
    for id in ids {
        w.despawn(id);
    }
}

fn start_wave(w: &mut World) {
    let at = w.game.next_wave_at.expect("between waves");
    w.update_horde(at);
}

fn spawn_all(w: &mut World) {
    loop {
        let h = w.game.horde.as_ref().unwrap();
        if h.spawned == Wave::new(h.wave).count() {
            break;
        }
        let at = h.next_pack_at;
        w.update_horde(at);
    }
}

#[test]
fn waves_wait_for_every_spawn_and_both_bosses_before_endless() {
    let mut w = world();
    let champions: Vec<_> = w.units.iter().filter(|u| u.champion.is_some()).map(|u| u.id).collect();
    let mut victory_events = 0;
    for wave in 1..=21 {
        start_wave(&mut w);
        assert_eq!(w.game.horde.as_ref().unwrap().wave, wave);
        kill_minions(&mut w);
        w.update_horde(SimTime(0));
        assert_eq!(w.game.horde.as_ref().unwrap().cleared, wave - 1, "a partial spawn is never a clear");
        spawn_all(&mut w);
        kill_minions(&mut w);
        w.update_horde(SimTime(0));
        if wave == 10 || wave == 20 {
            assert_eq!(w.game.horde.as_ref().unwrap().cleared, wave - 1, "boss still alive");
            let id = w.game.horde.as_ref().unwrap().boss.unwrap();
            let boss = w.unit(id).unwrap();
            assert_eq!(boss.team, Team::Red);
            assert!(boss.stats.max_health >= 10_000.0);
            assert_eq!(boss.monster, Wave::new(wave).boss());
            w.despawn(id);
            w.update_horde(SimTime(0));
        }
        assert_eq!(w.game.horde.as_ref().unwrap().cleared, wave);
        assert_eq!(w.game.horde.as_ref().unwrap().won_at.is_some(), wave >= GOAL);
        assert!(w.game.winner.is_none(), "campaign victory doesn't freeze or restart the run");
        victory_events += w.take_events().iter().filter(|e| matches!(e, SimEvent::HordeWon { .. })).count();
    }
    assert_eq!(victory_events, 1);
    assert!(champions.iter().all(|id| w.unit(*id).is_some()));
    assert!(w.units.iter().all(|u| u.team == Team::Blue), "no red champions or structures");
    w.restart_match();
    assert_eq!(w.game.horde.as_ref().unwrap().cleared, 0);
    assert!(w.game.horde.as_ref().unwrap().won_at.is_none());
    assert_eq!(w.units.iter().filter(|u| u.champion.is_some()).count(), 5);
}

#[test]
fn drafts_follow_waves_even_at_max_level_and_queue_unpicked_offers() {
    let mut w = world();
    let id = w.units.iter().find(|u| u.champion.is_some()).unwrap().id;
    w.unit_mut(id).unwrap().state.progress.level = 18;
    w.step(&[]);
    let first = w.unit(id).unwrap().state.progress.offer;
    assert_ne!(first, [0; 3]);
    assert_eq!(w.unit(id).unwrap().state.progress.drafted, 1);
    w.game.horde.as_mut().unwrap().cleared = 15;
    w.update_horde(SimTime(0));
    assert_eq!(w.unit(id).unwrap().state.progress.offer, first, "a missed choice isn't overwritten");
    for draft in 1..=4 {
        assert_eq!(w.unit(id).unwrap().state.progress.drafted, draft);
        augments::pick(&mut w.unit_mut(id).unwrap().state.progress, 0);
        w.update_horde(SimTime(0));
    }
    let p = w.unit(id).unwrap().state.progress;
    assert!(p.augments.iter().all(|a| *a != 0));
    assert_eq!(p.offer, [0; 3]);
}

#[test]
fn shared_rewards_include_supports_dead_players_and_tower_kills() {
    let mut w = world();
    let champions: Vec<_> = w.units.iter().filter(|u| u.champion.is_some()).map(|u| u.id).collect();
    w.unit_mut(champions[4]).unwrap().state.respawn_at = Some(SimTime(u64::MAX));
    let before: Vec<_> = champions.iter().map(|&id| w.unit(id).unwrap().state.progress).collect();
    let minion = w.spawn_minion(
        MinionKind::Siege,
        Team::Red,
        Vec2::new(6000.0, 6000.0),
        Some(Brain::Laner { lane: 0, next: 0 }),
    );
    let tower = w.units.iter().find(|u| u.kind == UnitKind::Turret).unwrap().id;
    let at = SimTime(0);
    let events = [SimEvent::Died { unit: minion, killer: tower, at, respawn_at: SimTime(u64::MAX) }];
    rewards(&mut w.units, &w.game, &events, &mut Vec::new());
    for (id, was) in champions.iter().zip(before) {
        let p = w.unit(*id).unwrap().state.progress;
        assert_eq!(p.gold - was.gold, 12.0);
        assert_eq!(p.xp - was.xp, lane::shared_xp(90, 5));
    }
}

#[test]
fn base_loss_ends_the_run_and_horde_state_is_hashed() {
    let mut w = world();
    let before = w.state_hash();
    w.game.horde.as_mut().unwrap().spawned = 3;
    assert_ne!(before, w.state_hash());
    let base = w.units.iter().find(|u| u.kind == UnitKind::Base).unwrap().id;
    let death = [SimEvent::Died { unit: base, killer: UnitId(99), at: SimTime(3), respawn_at: SimTime(u64::MAX) }];
    note_outcomes(&w.units, &mut w.game, &death, SimTime(4));
    assert_eq!(w.game.winner, Some((Team::Red, SimTime(3))));
    let wave = w.game.horde.as_ref().unwrap().wave;
    w.update_horde(SimTime(100_000));
    assert_eq!(w.game.horde.as_ref().unwrap().wave, wave);
}
