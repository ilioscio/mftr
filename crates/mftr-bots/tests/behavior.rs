//! Bots' behavior in small scenes (17): what they do with pings, and what they call.

use mftr_bots::{Bots, Tier};
use mftr_sim::map::MapId;
use mftr_sim::ping::{Ping, PingKind};
use mftr_sim::vision::Vision;
use mftr_sim::world::{PlayerId, Rules, World};
use mftr_sim::{ChampionId, SimEvent, Team, Tick, UnitId, Vec2};

/// One tick, as the server runs it: bots think, the world steps, fog updates, bots observe.
fn step(w: &mut World, bots: &mut Bots, pings: &mut Vec<(Team, Ping)>) -> Vec<SimEvent> {
    let k = Tick(w.tick().0 + 1);
    let cmds = bots.think(w, k);
    w.step(&cmds);
    let events = w.take_events();
    let visions = [Vision::of(w, Team::Blue), Vision::of(w, Team::Red)];
    let map = w.map().clone();
    for (i, team) in [Team::Blue, Team::Red].into_iter().enumerate() {
        let hidden: Vec<_> =
            w.units().iter().filter(|u| u.team != team && !visions[i].sees_unit(&map, u)).map(|u| u.id).collect();
        w.set_hidden(team, hidden);
    }
    bots.observe(w, &events, &visions);
    pings.extend(bots.take_pings());
    events
}

/// A Classic match: player 0 (blue) is a human who stands still; everyone else is a bot.
fn classic(tier: Tier) -> (World, Bots, UnitId) {
    let mut w = World::new(11);
    w.set_map(MapId::Crossroads.shared());
    w.set_rules(Rules::CLASSIC);
    w.start_match();
    let mut bots = Bots::new(5);
    let mut human = UnitId(0);
    for (i, c) in ChampionId::ALL.iter().enumerate() {
        let team = if i % 2 == 0 { Team::Blue } else { Team::Red };
        let id = w.spawn_champion(PlayerId(i as u8), team, *c, w.map().layout.champion_spawn[team as usize]);
        if i == 0 {
            human = id;
        } else {
            bots.add(PlayerId(i as u8), tier);
        }
    }
    (w, bots, human)
}

/// 17 §8: a human's "group" ping draws the team's bots there, and they answer it.
#[test]
fn bots_answer_a_group_ping() {
    let (mut w, mut bots, human) = classic(Tier::Veteran);
    let mut pings = Vec::new();
    for _ in 0..(100 * 30) {
        step(&mut w, &mut bots, &mut pings);
    }
    // Mid lane's river crossing.
    let at = w.map().size * 0.5;
    let dist = |w: &World| -> Vec<(UnitId, f32)> {
        w.units()
            .iter()
            .filter(|u| u.team == Team::Blue && u.id != human && u.kind == mftr_sim::UnitKind::Champion)
            .map(|u| (u.id, u.state.pos.distance(at)))
            .collect()
    };
    let before = dist(&w);
    pings.clear();
    bots.hear(Team::Blue, Ping { kind: PingKind::Group, at, target: None, from: Some(human) }, &w);
    for i in 0..(20 * 30) {
        step(&mut w, &mut bots, &mut pings);
        if std::env::var("TRACE").is_ok() && i % 60 == 0 {
            for u in w
                .units()
                .iter()
                .filter(|u| u.team == Team::Blue && u.id != human && u.kind == mftr_sim::UnitKind::Champion)
            {
                let p = u.owner.unwrap();
                println!(
                    "{i}: {:?} {:.0} away, hp {:.0}, order {:?}: {}",
                    u.champion,
                    u.state.pos.distance(at),
                    u.state.health,
                    u.state.order,
                    bots.describe(p).unwrap()
                );
            }
        }
    }
    let after = dist(&w);
    let came = before.iter().zip(&after).filter(|((_, b), (_, a))| *a < 1600.0 || b - a > 3000.0).count();
    assert!(came >= 3, "bots came: {before:?} -> {after:?}");
    let answers = pings.iter().filter(|(t, p)| *t == Team::Blue && p.kind == PingKind::OnMyWay).count();
    assert!(answers >= 1, "they said they were coming: {pings:?}");
}

/// 17 §8: over a match, bots call what their coach decides: ganks, sieges, defenses, missing
/// enemies.
#[test]
fn bots_call_their_plans() {
    let (mut w, mut bots, _) = classic(Tier::Elite);
    let mut pings = Vec::new();
    for _ in 0..(16 * 60 * 30) {
        step(&mut w, &mut bots, &mut pings);
        if w.game().winner.is_some() {
            break;
        }
    }
    let kinds: Vec<PingKind> = pings.iter().map(|(_, p)| p.kind).collect();
    let count = |k: PingKind| kinds.iter().filter(|x| **x == k).count();
    assert!(!kinds.is_empty(), "no pings at all");
    assert!(count(PingKind::Attack) + count(PingKind::Defend) + count(PingKind::OnMyWay) > 0, "{kinds:?}");
    // Not spam: a few a minute per team at most.
    assert!(kinds.len() < 16 * 2 * 8, "{} pings", kinds.len());
    // Every bot ping names its champion and a point on the map.
    for (_, p) in &pings {
        assert!(p.from.is_some());
        assert!(p.at.x >= 0.0 && p.at.y >= 0.0 && p.at.x <= w.map().size.x && p.at.y <= w.map().size.y);
    }
    let _ = Vec2::ZERO;
}
