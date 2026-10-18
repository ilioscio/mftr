//! Items in a match (18): penetration, crits, Wounds, tenacity, the actives, the passives that
//! act on hits, the support quest, Tally, and prediction of our own actives.

use super::*;
use crate::items::*;

fn command(player: u8, seq: u32, tick: u32, kind: CommandKind) -> Command {
    Command { player: PlayerId(player), seq, tick: Tick(tick), sub: SubTick::START, kind }
}

fn at(x: f32, y: f32) -> QPoint {
    QPoint::from_vec2(Vec2::new(x, y))
}

fn run(w: &mut World, ticks: u32) -> Vec<SimEvent> {
    let mut ev = Vec::new();
    for _ in 0..ticks {
        w.step(&[]);
        ev.extend(w.take_events());
    }
    ev
}

/// Two champions 500 apart; `a` holds `items`. Stats are refreshed.
fn duel(a: ChampionId, items: &[u8], b: ChampionId) -> (World, UnitId, UnitId) {
    let mut w = World::new(3);
    let me = w.spawn_champion(PlayerId(0), Team::Blue, a, Vec2::new(1000.0, 1000.0));
    let them = w.spawn_champion(PlayerId(1), Team::Red, b, Vec2::new(1500.0, 1000.0));
    let inv = &mut w.unit_mut(me).unwrap().state.progress.items;
    inv[..items.len()].copy_from_slice(items);
    w.step(&[]);
    w.take_events();
    (w, me, them)
}

fn damage_on(ev: &[SimEvent], who: UnitId) -> Vec<(f32, bool, DamageOrigin)> {
    ev.iter()
        .filter_map(|e| match *e {
            SimEvent::Damage { target, amount, absorbed, crit, origin, .. } if target == who => {
                Some((amount + absorbed, crit, origin))
            }
            _ => None,
        })
        .collect()
}

fn close(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-2, "{a} != {b}");
}

/// 02 §4 with items: percent and flat magic penetration, lethality by level.
#[test]
fn penetration_follows_the_order() {
    let (w, me, them) = duel(ChampionId::Ember, &[HOLLOW_SCEPTER, SPELLSTRIDERS], ChampionId::Vesper);
    let by = Hitter::of(w.unit(me).unwrap(), SimTime(0));
    let t = w.unit(them).unwrap();
    // 30 MR × (1 − 40%) = 18, less the boots' flat penetration.
    let flat = item(SPELLSTRIDERS).unwrap().bonus.magic_pen;
    let mr = (18.0 - flat).max(0.0);
    close(gear::mitigate(t, &by, 100.0, DamageKind::Magic, false, SimTime(0)), 100.0 * 100.0 / (100.0 + mr));
    let (w, me, them) = duel(ChampionId::Vesper, &[WHETTED_KRIS], ChampionId::Ember);
    let by = Hitter::of(w.unit(me).unwrap(), SimTime(0));
    let armor = 22.0 - 10.0 * (0.6 + 0.4 / 18.0);
    close(
        gear::mitigate(w.unit(them).unwrap(), &by, 100.0, DamageKind::Physical, false, SimTime(0)),
        10_000.0 / (100.0 + armor),
    );
}

/// Crits (18 §3.1): rolled by the server on landing, × (1.75 + bonus crit damage), flagged.
#[test]
fn crits_multiply_attacks_and_are_flagged() {
    let inv = [STARSTEEL_BLADE, FORTUNES_DIE, FORTUNES_DIE, FORTUNES_DIE, FORTUNES_DIE, FORTUNES_DIE];
    let (mut w, me, them) = duel(ChampionId::Vesper, &inv, ChampionId::Ember);
    assert_eq!(w.unit(me).unwrap().stats.crit_chance, 1.0);
    w.step(&[command(0, 1, 2, CommandKind::Attack(them))]);
    let hits = damage_on(&run(&mut w, 90), them);
    assert!(!hits.is_empty());
    let ad = 66.0 + 65.0;
    for (amount, crit, _) in hits {
        assert!(crit);
        close(amount, ad * 2.15 * 100.0 / 122.0);
    }
}

/// Wounds: a Barbed Hook's attacks cut the target's healing by 25% for 3 s.
#[test]
fn wounds_cut_healing() {
    let (mut w, me, them) = duel(ChampionId::Vesper, &[BARBED_HOOK], ChampionId::Ember);
    w.step(&[command(0, 1, 2, CommandKind::Attack(them))]);
    run(&mut w, 60);
    let t = w.unit(them).unwrap();
    assert_eq!(t.state.fx.wounds, 25);
    let now = SimTime::end_of(w.tick());
    close(gear::heal_factor(t, now), 0.75);
    let _ = me;
}

/// Tenacity shortens stuns, roots and slows (not knock-ups); slow resistance weakens slows.
#[test]
fn tenacity_and_slow_resistance() {
    let (mut w, me, _) = duel(ChampionId::Ember, &[STALWART_SABATONS, SWIFT_BOOTS], ChampionId::Vesper);
    // Two pairs of boots don't add stats (the first counts), but tenacity is the Sabatons'.
    let u = w.unit_mut(me).unwrap();
    assert!((u.stats.tenacity - 0.3).abs() < 1e-6);
    let t = SimTime(1000);
    apply_cc(u, Cc::Stun(SimDuration::from_millis(1000)), t, Vec2::ZERO, t, &mut Vec::new());
    assert_eq!(u.state.stunned_until, t.plus(SimDuration::from_millis(700)));
    apply_cc(u, Cc::Knockup(SimDuration::from_millis(1000)), t, Vec2::ZERO, t, &mut Vec::new());
    assert_eq!(u.state.stunned_until, t.plus(SimDuration::from_millis(1000)));
    u.stats.slow_resist = 0.25;
    apply_cc(u, Cc::Slow { pct: 40, duration: SimDuration::from_millis(1000) }, t, Vec2::ZERO, t, &mut Vec::new());
    assert_eq!(u.state.slow, 30);
}

/// Encase: untargetable and still for 2.5 s; attacks in flight miss; then back to normal.
#[test]
fn encase_is_a_stasis() {
    let (mut w, me, them) = duel(ChampionId::Ember, &[AMBER_RELIQUARY], ChampionId::Vesper);
    w.step(&[command(1, 1, 2, CommandKind::Attack(me))]);
    run(&mut w, 40);
    let use_at = w.tick().0 + 1;
    w.step(&[
        command(0, 1, use_at, CommandKind::UseItem(0)),
        command(0, 2, use_at, CommandKind::MoveTo(at(2000.0, 2000.0))),
    ]);
    let ev = w.take_events();
    assert!(ev.iter().any(|e| matches!(e, SimEvent::ItemUsed { item: AMBER_RELIQUARY, .. })));
    let pos = w.unit(me).unwrap().state.pos;
    assert!(w.unit(me).unwrap().encased);
    let during = run(&mut w, 70);
    assert!(damage_on(&during, me).is_empty(), "no damage in stasis");
    assert_eq!(w.unit(me).unwrap().state.pos, pos, "can't move");
    // Attackers lose an encased target (it isn't targetable); a new order finds it again.
    run(&mut w, 8);
    assert!(!w.unit(me).unwrap().encased);
    w.step(&[command(1, 2, w.tick().0 + 1, CommandKind::Attack(me))]);
    let after = run(&mut w, 120);
    assert!(!damage_on(&after, me).is_empty(), "targetable again");
    let ready = w.unit(me).unwrap().state.progress.gear.ready[0];
    assert!(ready > SimTime::end_of(w.tick()), "on cooldown");
    let _ = them;
}

/// Reaping Arc: damages enemies around the user and resets the attack timer; Quake slows.
#[test]
fn self_centered_actives_reach_enemies_around() {
    let (mut w, me, them) = duel(ChampionId::Rook, &[REAPING_CLEAVER, STONEWALL_AEGIS], ChampionId::Ember);
    w.unit_mut(them).unwrap().state.pos = Vec2::new(1300.0, 1000.0);
    let k = w.tick().0 + 1;
    w.step(&[command(0, 1, k, CommandKind::UseItem(0)), command(0, 2, k, CommandKind::UseItem(1))]);
    let ev = w.take_events();
    let hits = damage_on(&ev, them);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].2, DamageOrigin::Item(REAPING_CLEAVER));
    let t = &w.unit(them).unwrap().state;
    assert_eq!(t.slow, 55);
    assert!(w.unit(me).unwrap().state.attack_ready_at <= SimTime::end_of(w.tick()));
}

/// Plunge: a dash onto the enemy champion near the cursor, a strike, then a mark.
#[test]
fn plunge_dashes_strikes_and_marks() {
    let (mut w, me, them) = duel(ChampionId::Shade, &[SHRIKEBLADE], ChampionId::Ember);
    let k = w.tick().0 + 1;
    w.step(&[command(0, 1, k, CommandKind::UseItemAt { slot: 0, target: at(1500.0, 1000.0) })]);
    assert!(w.unit(me).unwrap().state.dash.is_some());
    let ev = run(&mut w, 20);
    let hits = damage_on(&ev, them);
    assert!(hits.iter().any(|h| h.2 == DamageOrigin::Item(SHRIKEBLADE)));
    let fx = w.unit(them).unwrap().state.fx;
    assert_eq!((fx.mark_by, fx.mark_amp), (me, 15));
    // Nobody near the cursor: no dash, no cooldown.
    let (mut w, me, _) = duel(ChampionId::Shade, &[SHRIKEBLADE], ChampionId::Ember);
    let k = w.tick().0 + 1;
    w.step(&[command(0, 1, k, CommandKind::UseItemAt { slot: 0, target: at(1000.0, 3000.0) })]);
    assert!(w.unit(me).unwrap().state.dash.is_none());
    assert_eq!(w.unit(me).unwrap().state.progress.gear.ready[0], SimTime(0));
}

/// Surge: a dash and a fan of five bolts; an enemy is hit once.
#[test]
fn surge_dashes_and_fires_a_fan() {
    let (mut w, me, them) = duel(ChampionId::Ember, &[WINDSTRIDE_BELT], ChampionId::Vesper);
    let k = w.tick().0 + 1;
    w.step(&[command(0, 1, k, CommandKind::UseItemAt { slot: 0, target: at(1500.0, 1000.0) })]);
    let ev = w.take_events();
    let bolts = ev.iter().filter(|e| matches!(e, SimEvent::MissileSpawned(m) if m.item == WINDSTRIDE_BELT)).count();
    assert_eq!(bolts, 5);
    assert!(w.unit(me).unwrap().state.dash.is_some());
    let hits = damage_on(&run(&mut w, 30), them);
    assert_eq!(hits.len(), 1, "{hits:?}");
}

/// Embrace shields allies around; Absolve frees an ally from a stun (not a knock-up).
#[test]
fn support_actives_protect_allies() {
    let mut w = World::new(3);
    let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Lumen, Vec2::new(1000.0, 1000.0));
    let ally = w.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Vesper, Vec2::new(1300.0, 1000.0));
    w.unit_mut(me).unwrap().state.progress.items[..2].copy_from_slice(&[KINSHIP_LOCKET, PURIFYING_CHALICE]);
    w.step(&[]);
    let k = w.tick().0 + 1;
    let now = SimTime::end_of(w.tick());
    w.unit_mut(ally).unwrap().state.stunned_until = now.plus(SimDuration::from_millis(2000));
    w.step(&[
        command(0, 1, k, CommandKind::UseItem(0)),
        command(0, 2, k, CommandKind::UseItemAt { slot: 1, target: at(1300.0, 1000.0) }),
    ]);
    let a = &w.unit(ally).unwrap().state;
    assert!(a.shield > 0.0, "embraced");
    assert!(a.stunned_until <= SimTime::end_of(w.tick()), "absolved");
    assert!(a.fx.haste > 0);
    assert!(w.unit(me).unwrap().state.shield > 0.0);
}

/// Spellblade: an ability arms the next attack; Cleave splashes melee attacks.
#[test]
fn spellblade_and_cleave() {
    let (mut w, me, them) = duel(ChampionId::Rook, &[RUNED_HILT, SPLITTING_AXE], ChampionId::Ember);
    let other = w.spawn_minion(MinionKind::Melee, Team::Red, Vec2::new(1350.0, 1080.0), None);
    w.unit_mut(them).unwrap().state.pos = Vec2::new(1250.0, 1000.0);
    // Q, then attack: the first hit carries Spellblade.
    let k = w.tick().0 + 1;
    w.unit_mut(me).unwrap().state.progress.ranks = [1; 4];
    w.step(&[Command {
        player: PlayerId(0),
        seq: 1,
        tick: Tick(k),
        sub: SubTick::START,
        kind: CommandKind::Cast { slot: 1, target: at(1250.0, 1000.0) },
    }]);
    assert!(w.unit(me).unwrap().state.spellblade_until > SimTime::end_of(w.tick()));
    let k = w.tick().0 + 1;
    w.step(&[command(0, 2, k, CommandKind::Attack(them))]);
    let ev = run(&mut w, 60);
    let splash = damage_on(&ev, other);
    assert!(splash.iter().any(|h| h.2 == DamageOrigin::Item(SPLITTING_AXE)), "{splash:?}");
    assert!(w.unit(me).unwrap().state.progress.gear.passive_ready[gear::SPELLBLADE].0 > 0, "spellblade used");
}

/// The Pilgrim's quest (18 §6): Tithe pays on hits on champions, three charges; at 500 quest
/// gold the Ledger becomes the Astrolabe, its wards full.
#[test]
fn the_pilgrims_quest() {
    let mut w = World::new(3);
    w.set_map(MapId::Crossroads.shared());
    w.set_rules(Rules::CLASSIC);
    let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Lumen, Vec2::new(3000.0, 3000.0));
    let them = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Vesper, Vec2::new(3400.0, 3000.0));
    w.unit_mut(me).unwrap().state.progress.items[0] = PILGRIMS_LEDGER;
    w.step(&[]);
    w.step(&[command(0, 1, w.tick().0 + 1, CommandKind::Attack(them))]);
    let ev = run(&mut w, 200);
    let tithes = ev
        .iter()
        .filter(|e| matches!(e, SimEvent::Reward { unit, gold, .. } if *unit == me && *gold == quest::TITHE_GOLD))
        .count();
    assert!(tithes >= 3, "{tithes}");
    let g = w.unit(me).unwrap().state.progress.gear;
    assert!(g.quest >= 3 * quest::TITHE_GOLD as u16);
    w.unit_mut(me).unwrap().state.progress.gear.quest = quest::STAGE;
    w.step(&[]);
    let p = w.unit(me).unwrap().state.progress;
    assert_eq!(p.items[0], PILGRIMS_ASTROLABE);
    assert_eq!(p.charges[0], 3);
}

/// Supporting pays the quest too: healing an allied champion earns a Tithe charge's gold.
#[test]
fn healing_an_ally_pays_tithe() {
    let mut w = World::new(3);
    w.set_map(MapId::Crossroads.shared());
    w.set_rules(Rules::CLASSIC);
    let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Lumen, Vec2::new(3000.0, 3000.0));
    let ally = w.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Vesper, Vec2::new(3300.0, 3000.0));
    w.unit_mut(me).unwrap().state.progress.items[0] = PILGRIMS_LEDGER;
    w.unit_mut(me).unwrap().state.progress.ranks = [1; 4];
    w.step(&[]);
    w.unit_mut(ally).unwrap().state.health = 200.0;
    let k = w.tick().0 + 1;
    w.step(&[command(0, 1, k, CommandKind::Cast { slot: 0, target: at(3300.0, 3000.0) })]);
    let ev = run(&mut w, 5);
    let tithe = ev
        .iter()
        .any(|e| matches!(e, SimEvent::Reward { unit, gold, .. } if *unit == me && *gold == quest::TITHE_GOLD));
    assert!(tithe, "healing an ally pays");
    assert_eq!(w.unit(me).unwrap().state.progress.gear.quest, quest::TITHE_GOLD as u16);
}

/// Tally stacks on takedowns.
#[test]
fn tally_stacks_ability_power() {
    let (mut w, me, them) = duel(ChampionId::Ember, &[TALLY_CHARM], ChampionId::Vesper);
    w.set_rules(Rules::ARAM);
    let base = w.unit(me).unwrap().stats.ability_power;
    w.unit_mut(them).unwrap().state.health = 1.0;
    w.step(&[command(0, 1, w.tick().0 + 1, CommandKind::Attack(them))]);
    run(&mut w, 90);
    assert_eq!(w.unit(me).unwrap().state.progress.gear.tally, 2);
    close(w.unit(me).unwrap().stats.ability_power, base + 6.0);
}

/// Our own actives on ourselves are predicted bit-exactly (Fleet, Encase, Surge's dash).
#[test]
fn prediction_matches_with_item_actives() {
    for (champ, inv) in [
        (ChampionId::Shade, [VEILRUNNER_DAGGER, AMBER_RELIQUARY, WINDSTRIDE_BELT, STONESKIN_WARD, 0, 0]),
        (ChampionId::Rook, [REAPING_CLEAVER, THREEFOLD_BRAND, KINSHIP_LOCKET, PURIFYING_CHALICE, 0, 0]),
    ] {
        let mut full = World::new(12);
        full.set_map(MapId::Arena.shared());
        let me = full.spawn_champion(PlayerId(0), Team::Blue, champ, Vec2::new(1000.0, 3300.0));
        full.unit_mut(me).unwrap().state.progress.items = inv;
        for i in 0..3 {
            full.spawn_minion(MinionKind::Siege, Team::Red, Vec2::new(1300.0 + 350.0 * i as f32, 3100.0), None);
        }
        let mut rng = Pcg32::new(9, 9);
        for k in 1..=900u32 {
            let mut c = Vec::new();
            if rng.next_u32().is_multiple_of(7) {
                let t = at(rng.range_f32(600.0, 2600.0), rng.range_f32(2800.0, 3700.0));
                c.push(command(
                    0,
                    k,
                    k,
                    match rng.next_u32() % 6 {
                        0 | 1 => CommandKind::MoveTo(t),
                        2 => CommandKind::UseItem((rng.next_u32() % 4) as u8),
                        3 => CommandKind::UseItemAt { slot: (rng.next_u32() % 4) as u8, target: t },
                        _ => CommandKind::Cast { slot: (rng.next_u32() % 4) as u8, target: t },
                    },
                ));
            }
            let own = full.unit(me).unwrap().clone();
            let proxies = full.units().iter().filter(|u| u.id != me && u.state.alive()).map(|u| Unit {
                state: UnitState::new(u.state.pos, 0.0),
                brain: None,
                ..u.clone()
            });
            let mut predicted = World::from_units(full.tick(), vec![own]);
            predicted.set_map(MapId::Arena.shared());
            predicted.set_prediction_mode(true);
            predicted.replace_others(me, proxies);
            predicted.step(&c);
            full.step(&c);
            full.take_events();
            assert!(
                predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state),
                "{champ:?} tick {k}\n{:?}\n{:?}",
                predicted.unit(me).unwrap().state,
                full.unit(me).unwrap().state
            );
        }
    }
}
