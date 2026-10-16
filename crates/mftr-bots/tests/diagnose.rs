//! Diagnostics for tuning (run with `--ignored --nocapture`): small scenes with bots, measured.

use mftr_bots::{Bots, Tier};
use mftr_sim::map::MapId;
use mftr_sim::vision::Vision;
use mftr_sim::world::{PlayerId, Rules, World};
use mftr_sim::{ChampionId, SimEvent, Team, Tick, UnitId, UnitKind};

fn step(w: &mut World, bots: &mut Bots) -> Vec<SimEvent> {
    let k = Tick(w.tick().0 + 1);
    let cmds = bots.think(w, k);
    w.step(&cmds);
    let events = w.take_events();
    let visions = [Vision::of(w, Team::Blue), Vision::of(w, Team::Red)];
    for (i, team) in [Team::Blue, Team::Red].into_iter().enumerate() {
        let map = w.map().clone();
        let hidden: Vec<_> =
            w.units().iter().filter(|u| u.team != team && !visions[i].sees_unit(&map, u)).map(|u| u.id).collect();
        w.set_hidden(team, hidden);
    }
    bots.observe(w, &events, &visions);
    bots.take_pings();
    events
}

/// Per champion: last hits, and enemy minions that died within 900 u of it.
fn lane(champions: &[(ChampionId, Team, Tier)], minutes: u32, trace: Option<usize>) {
    let mut w = World::new(5);
    w.set_map(MapId::Crossroads.shared());
    w.set_rules(Rules::CLASSIC);
    w.start_match();
    let mut bots = Bots::new(3);
    let mut ids = Vec::new();
    for (i, (c, team, tier)) in champions.iter().enumerate() {
        let home = w.map().layout.champion_spawn[*team as usize];
        ids.push(w.spawn_champion(PlayerId(i as u8), *team, *c, home));
        bots.add(PlayerId(i as u8), *tier);
    }
    let mut near = vec![0u32; ids.len()];
    for _ in 0..(minutes * 60 * 30) {
        let events = step(&mut w, &mut bots);
        for e in &events {
            let SimEvent::Died { unit, killer, .. } = e else { continue };
            let Some(u) = w.unit(*unit) else { continue };
            if u.kind != UnitKind::Minion {
                continue;
            }
            for (i, id) in ids.iter().enumerate() {
                let me = w.unit(*id).unwrap();
                if me.team == u.team || !me.state.alive() || me.state.pos.distance(u.state.pos) > 900.0 {
                    continue;
                }
                near[i] += 1;
                if trace == Some(i) && *killer != *id {
                    let k = w.unit(*killer).map(|k| (k.kind, k.attack.map(|a| a.range)));
                    println!(
                        "{:>5} missed: killed by {k:?}; {:.0} away, order {:?}, winding up {:?}, ready in {:.2}s, hp {:.0}, {}",
                        w.tick().0,
                        me.state.pos.distance(u.state.pos),
                        me.state.order,
                        me.state.attack.map(|a| a.target == *unit),
                        me.state.attack_ready_at.secs_since(mftr_sim::SimTime::end_of(w.tick())),
                        me.state.health,
                        bots.describe(PlayerId(i as u8)).unwrap_or_default()
                    );
                }
            }
        }
    }
    let stats = bots.stats();
    for (i, id) in ids.iter().enumerate() {
        let u = w.unit(*id).unwrap();
        let s = stats.iter().find(|(p, _)| p.0 as usize == i).map(|(_, s)| *s).unwrap_or_default();
        println!(
            "{:?} {:?} ({}): {} CS of {} near, level {}, {}/{}/{} K/D/A, {} orders, modes {:?}",
            u.champion.unwrap(),
            u.team,
            champions[i].2.name(),
            u.state.progress.cs,
            near[i],
            u.state.progress.level,
            u.state.progress.kills,
            u.state.progress.deaths,
            u.state.progress.assists,
            s.orders,
            s.mode_ticks
        );
    }
    let _: Vec<UnitId> = ids;
}

/// One bot alone in its lane.
#[test]
#[ignore]
fn last_hits_alone_in_lane() {
    for (champion, tier) in
        [(ChampionId::Vesper, Tier::Veteran), (ChampionId::Ember, Tier::Elite), (ChampionId::Rook, Tier::Nightmare)]
    {
        lane(&[(champion, Team::Blue, tier)], 8, None);
    }
}

/// Two mid laners against each other.
#[test]
#[ignore]
fn mid_duel() {
    let trace = std::env::var("TRACE").ok().map(|v| v.parse().unwrap_or(0));
    for tier in std::env::var("TIERS").map_or(vec![Tier::Recruit, Tier::Veteran, Tier::Elite, Tier::Nightmare], |t| {
        t.split(",").map(|n| Tier::by_name(n).unwrap()).collect()
    }) {
        lane(&[(ChampionId::Ember, Team::Blue, tier), (ChampionId::Quill, Team::Red, tier)], 8, trace);
    }
}
