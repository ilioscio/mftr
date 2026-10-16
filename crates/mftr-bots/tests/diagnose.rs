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

/// Teamfights in the open: what the fight model predicts against what happens.
#[test]
#[ignore]
fn teamfight_calibration() {
    use mftr_bots::fight::{self, Side};
    let comps: [[ChampionId; 5]; 2] = [
        [ChampionId::Ember, ChampionId::Bastion, ChampionId::Lumen, ChampionId::Quill, ChampionId::Marrow],
        [ChampionId::Vesper, ChampionId::Rook, ChampionId::Shade, ChampionId::Cairn, ChampionId::Wren],
    ];
    for level in [3u8, 9, 15] {
        for tier in [Tier::Legacy, Tier::Nightmare] {
            for swap in [false, true] {
                let mut w = World::new(9);
                w.set_map(MapId::Open.shared());
                w.set_rules(Rules::ARAM);
                let mut bots = Bots::new(1);
                let mut ids = Vec::new();
                for i in 0..10u8 {
                    let team = if i % 2 == 0 { Team::Blue } else { Team::Red };
                    let side = (i % 2 == 0) == !swap;
                    let c = comps[if side { 0 } else { 1 }][i as usize / 2];
                    let x = if team == Team::Blue { 1400.0 } else { 2600.0 };
                    let pos = mftr_sim::Vec2::new(x, 1200.0 + (i / 2) as f32 * 400.0);
                    let id = w.spawn_champion(PlayerId(i), team, c, pos);
                    let u = w.unit_mut(id).unwrap();
                    let p = &mut u.state.progress;
                    p.level = level;
                    p.points = 0;
                    p.ranks = [
                        mftr_sim::champion::max_rank(0, level),
                        mftr_sim::champion::max_rank(1, level)
                            .min(level.saturating_sub(mftr_sim::champion::max_rank(0, level))),
                        1,
                        mftr_sim::champion::max_rank(3, level),
                    ];
                    // Items worth what a champion has by then.
                    let budget = (level as f32 - 3.0) * 900.0;
                    let mut spent = 0.0;
                    let mut slot = 0;
                    for &it in mftr_sim::items::build_path(c) {
                        let cost = mftr_sim::items::item(it).unwrap().cost;
                        if spent + cost > budget || slot >= 6 {
                            break;
                        }
                        if mftr_sim::items::item(it).unwrap().recipe.is_empty() {
                            p.items[slot] = it;
                            slot += 1;
                            spent += cost;
                        }
                    }
                    u.refresh_stats();
                    u.state.health = u.stats.max_health;
                    ids.push(id);
                    bots.add(PlayerId(i), tier);
                }
                let now = mftr_sim::SimTime::end_of(w.tick());
                let (mut a, mut b) = (Side::default(), Side::default());
                for id in &ids {
                    let u = w.unit(*id).unwrap();
                    let side = if u.team == Team::Blue { &mut a } else { &mut b };
                    side.add(&fight::power(u, now, true, |_| true), 1.0);
                }
                let predicted = fight::outcome(&a, &b);
                for _ in 0..(40 * 30) {
                    step(&mut w, &mut bots);
                    let alive = |t: Team| {
                        ids.iter().filter(|id| w.unit(**id).is_some_and(|u| u.team == t && u.state.alive())).count()
                    };
                    if alive(Team::Blue) == 0 || alive(Team::Red) == 0 {
                        break;
                    }
                }
                let hp = |t: Team| -> f32 {
                    ids.iter()
                        .filter_map(|id| w.unit(*id))
                        .filter(|u| u.team == t && u.state.alive())
                        .map(|u| u.state.health / u.stats.max_health)
                        .sum()
                };
                let alive = |t: Team| {
                    ids.iter().filter(|id| w.unit(**id).is_some_and(|u| u.team == t && u.state.alive())).count()
                };
                println!(
                    "level {level:>2} {:<9} blue={}: predicted {predicted:+.2}; after {:>4.1} s blue {} alive ({:.1} hp), red {} alive ({:.1} hp)",
                    tier.name(),
                    if swap { "melee" } else { "mages" },
                    w.tick().0 as f32 / 30.0,
                    alive(Team::Blue),
                    hp(Team::Blue),
                    alive(Team::Red),
                    hp(Team::Red)
                );
            }
        }
    }
}

/// What building a map's knowledge costs, and each travel field.
#[test]
#[ignore]
fn knowledge_cost() {
    for id in [MapId::Crossroads, MapId::Bridge, MapId::HordeBridge] {
        let map = id.shared();
        let t = std::time::Instant::now();
        let k = mftr_bots::knowledge::Knowledge::of(&map);
        let built = t.elapsed();
        let t = std::time::Instant::now();
        k.prepare();
        println!("{id:?}: built in {built:?}, all fields in {:?} ({} keys)", t.elapsed(), k.key_count());
    }
}

/// A jungler alone on the map: how its clear goes.
#[test]
#[ignore]
fn jungler_alone() {
    for (c, tier) in [(ChampionId::Shade, Tier::Elite), (ChampionId::Rook, Tier::Veteran), (ChampionId::Marrow, Tier::Elite)] {
        let mut w = World::new(5);
        w.set_map(MapId::Crossroads.shared());
        w.set_rules(Rules::CLASSIC);
        w.start_match();
        let home = w.map().layout.champion_spawn[0];
        let me = w.spawn_champion(PlayerId(0), Team::Blue, c, home);
        let mut bots = Bots::new(3);
        bots.add(PlayerId(0), tier);
        let mut monsters = 0;
        let mut deaths = 0;
        let mut first_clear = None;
        for t in 0..(10 * 60 * 30) {
            let events = step(&mut w, &mut bots);
            for e in &events {
                if let SimEvent::Died { unit, killer, .. } = e {
                    if *killer == me && w.unit(*unit).is_some_and(|u| u.kind == UnitKind::Monster) {
                        monsters += 1;
                        if monsters == 1 {
                            first_clear = Some(t / 30);
                        }
                    }
                    if *unit == me {
                        deaths += 1;
                    }
                }
            }
            if std::env::var("TRACE").is_ok() && t % (15 * 30) == 0 {
                let u = w.unit(me).unwrap();
                println!(
                    "  {:>3} s: lvl {} hp {:.0}/{:.0} gold {:.0} at ({:.0},{:.0}) {}",
                    t / 30,
                    u.state.progress.level,
                    u.state.health,
                    u.stats.max_health,
                    u.state.progress.gold,
                    u.state.pos.x,
                    u.state.pos.y,
                    bots.describe(PlayerId(0)).unwrap_or_default()
                );
            }
        }
        let u = w.unit(me).unwrap();
        println!(
            "{c:?} ({}): 10 min: level {}, {monsters} monsters (first at {first_clear:?} s), {deaths} deaths, gold+items {:.0}, modes {:?}",
            tier.name(),
            u.state.progress.level,
            u.state.progress.gold,
            bots.stats()[0].1.mode_ticks
        );
    }
}
