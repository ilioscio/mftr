//! Headless Netcode Lab (03 §14): one server and N scripted clients over link-conditioned
//! links, in deterministic virtual time. Same seed → same result, so it's usable in CI.

use crate::report::{ClickBot, DodgeBot, DuelBot, JumpMeter, Summary};
use mftr_client::{ClientSession, Phase};
use mftr_net::conditioner::{LinkProfile, SimLink};
use mftr_net::msg::{ServerMessage, decode_server};
use mftr_server::{Scenario, ServerConfig, ServerCore};
use mftr_sim::Team;
use mftr_sim::world::CHAMPION_MOVE_SPEED;

#[derive(Clone, Debug)]
pub struct LabConfig {
    pub profile: LinkProfile,
    pub clients: usize,
    pub seconds: f64,
    pub seed: u64,
    pub fps: f64,
    /// Seconds of play before measuring, so control loops settle (a real match starts with
    /// time in the fountain). Measurement then runs for `seconds`.
    pub warmup: f64,
    pub scenario: Scenario,
    /// Client collision proxies and the minion bubble (03a §5); off = naive prediction.
    pub proxies: bool,
    /// Dodge-rig reaction time in seconds (scenario `dodge`).
    pub reaction: f64,
    /// Fault injection: force every client's input margin (validates the ghost-hit detector).
    pub margin_override: Option<f64>,
    /// Stress tests: every champion holds these augments (granted on the server, so the
    /// recording no longer re-simulates).
    pub augments: Option<[u8; mftr_sim::augments::SLOTS]>,
}

enum Bot {
    Click(ClickBot),
    Dodge(DodgeBot),
    Duel(DuelBot),
}

struct LabClient {
    session: ClientSession,
    up: SimLink,
    down: SimLink,
    bot: Bot,
    jumps: JumpMeter,
    /// Client clocks are deliberately offset from the server's, to exercise clock sync.
    clock_offset: f64,
    next_frame: f64,
    next_hello: f64,
}

pub struct LabResult {
    pub summary: Summary,
    pub server_hash: u64,
    pub server_ticks: u64,
    /// Fog audit (03 §10, slice 3 exit): units sent to a client its team could not see.
    pub fog_violations: u64,
    /// Unit-snapshots withheld by fog (sanity check that culling actually happens).
    pub fog_hidden: u64,
    /// Wall-clock cost of one server tick (simulation + snapshots), mean and max, in ms.
    pub tick_ms_mean: f64,
    pub tick_ms_max: f64,
    /// The server's recording of the session (M2 slice 5).
    pub replay: mftr_server::Replay,
}

pub fn run(cfg: &LabConfig) -> LabResult {
    const STEP: f64 = 0.0005;
    let mut server = ServerCore::new(
        ServerConfig { seed: cfg.seed, scenario: cfg.scenario, record: true, ..Default::default() },
        0.0,
    );
    let mut clients: Vec<LabClient> = (0..cfg.clients)
        .map(|i| {
            let s = cfg.seed.wrapping_mul(1000) + i as u64;
            LabClient {
                session: {
                    let mut s = ClientSession::new();
                    s.set_collision_proxies(cfg.proxies);
                    s.set_margin_override(cfg.margin_override);
                    s
                },
                up: SimLink::new(cfg.profile, s * 2 + 1),
                down: SimLink::new(cfg.profile, s * 2 + 2),
                bot: match cfg.scenario {
                    Scenario::DodgeRig => Bot::Dodge(DodgeBot::new(s, cfg.reaction)),
                    Scenario::Duel | Scenario::Aram | Scenario::Mayhem | Scenario::Hyper => {
                        Bot::Duel(DuelBot::new(s, cfg.reaction))
                    }
                    _ => Bot::Click(ClickBot::new(s)),
                },
                jumps: JumpMeter::default(),
                clock_offset: 1000.0 + 37.0 * i as f64,
                next_frame: 0.001 * i as f64,
                next_hello: 0.0,
            }
        })
        .collect();

    let frame_dt = 1.0 / cfg.fps;
    let mut t = 0.0;
    let (mut fog_violations, mut fog_hidden) = (0u64, 0u64);
    let (mut tick_cost, mut tick_max, mut ticks_timed) = (0.0f64, 0.0f64, 0u64);
    let mut measuring = cfg.warmup <= 0.0;
    while t < cfg.warmup + cfg.seconds {
        if !measuring && t >= cfg.warmup {
            measuring = true;
            for c in clients.iter_mut() {
                c.session.stats = Default::default();
                c.jumps = JumpMeter::default();
            }
        }
        // Client → server.
        for (i, c) in clients.iter_mut().enumerate() {
            while let Some(p) = c.up.recv(t) {
                for (to, bytes) in server.handle_packet(i as u64, &p, t) {
                    debug_assert_eq!(to, i as u64);
                    c.down.send(bytes, t);
                }
            }
        }
        // Server tick.
        if t >= server.next_tick_due() {
            if let Some(held) = cfg.augments {
                let champions: Vec<_> =
                    server.world().units().iter().filter(|u| u.champion.is_some()).map(|u| u.id).collect();
                for id in champions {
                    let p = &mut server.world_mut().unit_mut(id).unwrap().state.progress;
                    (p.augments, p.drafted, p.offer) = (held, mftr_sim::augments::SLOTS as u8, [0; 3]);
                }
            }
            let started = std::time::Instant::now();
            let packets = server.step(t);
            let cost = started.elapsed().as_secs_f64() * 1e3;
            if measuring {
                tick_cost += cost;
                tick_max = tick_max.max(cost);
                ticks_timed += 1;
            }
            let mut allowed: [Option<std::collections::BTreeSet<mftr_sim::UnitId>>; 2] = [None, None];
            for (to, bytes) in &packets {
                let (Some(team), Ok((_, ServerMessage::Snapshot(_)))) = (server.team_of(*to), decode_server(bytes))
                else {
                    continue;
                };
                // Everything the client now knows about (deltas and coasting included) must be
                // visible to its team.
                let set = allowed[(team == Team::Red) as usize].get_or_insert_with(|| server.visible_to(team));
                let known = server.last_sent(*to);
                fog_violations += known.iter().filter(|id| !set.contains(id)).count() as u64;
                fog_hidden += (server.world().units().len() - 1).saturating_sub(known.len()) as u64;
            }
            for (to, bytes) in packets {
                clients[to as usize].down.send(bytes, t);
            }
        }
        // Server → client, then client frames.
        for c in clients.iter_mut() {
            let local = t + c.clock_offset;
            while let Some(p) = c.down.recv(t) {
                c.session.handle_packet(&p, local);
            }
            if t < c.next_frame {
                continue;
            }
            c.next_frame += frame_dt;
            c.session.update(local);
            match c.session.phase() {
                Phase::Connecting | Phase::Lobby => {
                    if t >= c.next_hello {
                        let p = c.session.hello_packet(local);
                        c.up.send(p, t);
                        c.next_hello = t + 0.25;
                    }
                }
                Phase::Joining => {}
                Phase::Playing => {
                    match &mut c.bot {
                        Bot::Click(b) => b.act(&mut c.session, local, t),
                        Bot::Dodge(b) => b.act(&mut c.session, local, t),
                        Bot::Duel(b) => b.act(&mut c.session, local, t),
                    };
                    if c.session.should_send(local) {
                        let p = c.session.input_packet(local);
                        c.up.send(p, t);
                    }
                    // Dashes, lunges, pulls and death legitimately move faster than walking.
                    let walking = c.session.own_state_now().is_some_and(|s| s.alive() && s.dash.is_none());
                    match c.session.own_render_position(local) {
                        Some(pos) if walking => {
                            c.jumps.observe(pos, c.session.visible_correction(), frame_dt, CHAMPION_MOVE_SPEED)
                        }
                        _ => c.jumps.skip(),
                    }
                }
            }
        }
        t += STEP;
    }

    let refs: Vec<(&ClientSession, &JumpMeter)> = clients.iter().map(|c| (&c.session, &c.jumps)).collect();
    LabResult {
        summary: Summary::from_sessions(cfg.profile.name, cfg.profile.latency * 2.0, cfg.seconds, &refs),
        server_hash: server.world().state_hash(),
        server_ticks: server.stats.ticks,
        fog_violations,
        fog_hidden,
        tick_ms_mean: tick_cost / ticks_timed.max(1) as f64,
        tick_ms_max: tick_max,
        replay: server.game().replay(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab(profile: LinkProfile, seconds: f64) -> LabResult {
        lab_in(profile, seconds, Scenario::Empty, true)
    }

    fn lab_in(profile: LinkProfile, seconds: f64, scenario: Scenario, proxies: bool) -> LabResult {
        run(&LabConfig {
            profile,
            clients: 6,
            seconds,
            seed: 42,
            fps: 144.0,
            warmup: 10.0,
            scenario,
            proxies,
            reaction: 0.25,
            margin_override: None,
            augments: None,
        })
    }

    /// Alone on a perfect link, prediction is exact with nothing to collide with. Among minions
    /// it can't be bit-exact: proxies come from 0.25 u-quantized positions and straight-line
    /// extrapolation, while minions slide and detour. Corrections must stay tiny, though.
    /// (With several champions, they can bump each other with a re-click inside the predicted tick.)
    #[test]
    fn perfect_link_predicts_exactly_or_nearly_when_alone() {
        let solo_perfect = |scenario| {
            run(&LabConfig {
                profile: LinkProfile::PERFECT,
                clients: 1,
                seconds: 60.0,
                seed: 42,
                fps: 144.0,
                warmup: 10.0,
                scenario,
                proxies: true,
                reaction: 0.25,
                margin_override: None,
                augments: None,
            })
            .summary
        };
        let empty = solo_perfect(Scenario::Empty);
        assert!(empty.commands > 100);
        assert_eq!(empty.mismatch_pct, 0.0, "{}", empty.row());
        let minions = solo_perfect(Scenario::MinionSandbox);
        assert!(minions.corr_max < 5.0 && minions.visible_mean < 0.05, "{}", minions.row());
    }

    #[test]
    fn deterministic_for_a_given_seed() {
        let a = lab(LinkProfile::ROUGH, 10.0);
        let b = lab(LinkProfile::ROUGH, 10.0);
        assert_eq!(a.server_hash, b.server_hash);
    }

    /// 03 §1 targets: mean visible own-champion correction < 5 u at 60 ms RTT and < 15 u at
    /// 120 ms / 20 ms jitter / 2% loss (the M0 exit profile), with ~99% of commands on time.
    #[test]
    fn typical_profile_meets_correction_target() {
        let r = lab(LinkProfile::TYPICAL, 120.0);
        assert!(r.summary.visible_mean < 5.0, "{}", r.summary.row());
        assert!(r.summary.late_pct < 1.0, "{}", r.summary.row());
        assert_eq!(r.summary.hard_resets, 0, "{}", r.summary.row());
    }

    #[test]
    fn rough_profile_meets_m0_correction_target() {
        let r = lab(LinkProfile::ROUGH, 120.0);
        assert!(r.summary.visible_mean < 15.0, "{}", r.summary.row());
        assert!(r.summary.late_pct < 1.5, "{}", r.summary.row());
        assert_eq!(r.summary.hard_resets, 0, "{}", r.summary.row());
    }

    fn solo(scenario: Scenario, proxies: bool) -> LabResult {
        run(&LabConfig {
            profile: LinkProfile::MID,
            clients: 1,
            seconds: 900.0,
            seed: 7,
            fps: 144.0,
            warmup: 10.0,
            scenario,
            proxies,
            reaction: 0.25,
            margin_override: None,
            augments: None,
        })
    }

    /// M1 slice 1 (08 roadmap): with collision proxies, minion block adds well under one
    /// > 15 u correction per player-minute at 80 ms, and clearly beats naive prediction.
    #[test]
    fn minion_block_is_predicted_with_proxies() {
        let baseline = solo(Scenario::Empty, true).summary;
        let naive = solo(Scenario::MinionSandbox, false).summary;
        let proxied = solo(Scenario::MinionSandbox, true).summary;
        let caused = proxied.corr_per_min_over_15 - baseline.corr_per_min_over_15;
        assert!(caused < 1.0, "minion-caused corrections {caused:.2}/min\n{}", proxied.row());
        assert!(proxied.visible_mean < 0.2, "{}", proxied.row());
        assert!(proxied.visible_mean < naive.visible_mean / 3.0, "proxies {} vs naive {}", proxied.row(), naive.row());
        assert!(proxied.jumps_per_min < naive.jumps_per_min, "proxies {} vs naive {}", proxied.row(), naive.row());
    }

    fn dodge_rig(profile: LinkProfile, reaction: f64, margin_override: Option<f64>) -> Summary {
        run(&LabConfig {
            profile,
            clients: 6,
            seconds: 600.0,
            seed: 11,
            fps: 144.0,
            warmup: 10.0,
            scenario: Scenario::DodgeRig,
            proxies: true,
            reaction,
            margin_override,
            augments: None,
        })
        .summary
    }

    /// M1 slice 2 exit (03 §1): ghost hits (shown as a dodge, hit on the server) stay below
    /// 0.5% of near-misses at 60 ms and 2% at 120 ms / 20 ms jitter / 2% loss.
    #[test]
    fn dodge_rig_meets_ghost_hit_targets() {
        for (profile, limit) in [(LinkProfile::TYPICAL, 0.005), (LinkProfile::ROUGH, 0.02)] {
            let s = dodge_rig(profile, 0.30, None);
            assert!(s.dodge.near_misses > 50, "too few near-misses to judge: {}", s.dodge_row());
            assert!(s.ghost_rate() < limit, "{}", s.dodge_row());
            // Phantom hits (shown hit, server miss) stay rare once interceptions by allies are
            // predicted and drawn as unconfirmed (03a §7).
            assert!((s.dodge.phantom_hits as f64) < 0.02 * s.dodge.near_misses as f64, "{}", s.dodge_row());
        }
    }

    /// M1 slice 3 exit: over a long run with walls, brush, minions and 10 wandering players,
    /// no client is ever sent a unit its team can't see, while plenty are withheld.
    #[test]
    fn fog_audit_never_leaks_hidden_units() {
        let r = run(&LabConfig {
            profile: LinkProfile::TYPICAL,
            clients: 10,
            seconds: 300.0,
            seed: 5,
            fps: 60.0,
            warmup: 5.0,
            scenario: Scenario::MinionSandbox,
            proxies: true,
            reaction: 0.25,
            margin_override: None,
            augments: None,
        });
        assert_eq!(r.fog_violations, 0);
        assert!(r.fog_hidden > 10_000, "fog should be hiding things: {}", r.fog_hidden);
    }

    /// The detector itself must work: with the input margin forced negative (nearly every
    /// command late), ghost hits must show up.
    #[test]
    fn ghost_hit_detector_fires_under_fault_injection() {
        let s = dodge_rig(LinkProfile::AWFUL, 0.40, Some(-0.06));
        assert!(s.dodge.ghost_hits > 0, "{}", s.dodge_row());
    }

    /// M1 slice 4 exit, "duel playable end to end": mage vs. marksman bots fight with their
    /// whole kits at 80 ms. Both sides deal damage, kill and die, respawn and keep fighting,
    /// while prediction stays within the 03 §1 correction target and skillshot dodges stay
    /// honest (ghost hits).
    #[test]
    fn duel_is_playable_end_to_end() {
        let r = run(&LabConfig {
            profile: LinkProfile::MID,
            clients: 2,
            seconds: 300.0,
            seed: 3,
            fps: 144.0,
            warmup: 5.0,
            scenario: Scenario::Duel,
            proxies: true,
            reaction: 0.25,
            margin_override: None,
            augments: None,
        });
        let s = &r.summary;
        assert_eq!(s.hard_resets, 0, "{}", s.row());
        assert!(s.visible_mean < 15.0, "{}", s.row());
        assert!(s.kills.iter().all(|k| *k >= 2), "both sides score kills: {:?}", s.kills);
        assert!(s.deaths.iter().all(|d| *d >= 2), "both sides die and respawn: {:?}", s.deaths);
        assert!(s.damage_dealt.iter().all(|d| *d > 2000.0), "{:?}", s.damage_dealt);
        assert_eq!(r.fog_violations, 0);
        assert!(s.dodge.near_misses >= 20, "the bots should be dodging skillshots: {}", s.dodge_row());
        assert!(s.ghost_rate() < 0.02, "{}", s.dodge_row());
        assert!((s.dodge.phantom_hits as f64) <= 0.03 * s.dodge.near_misses as f64, "{}", s.dodge_row());
    }

    /// M3 slice 1: ARAM: Mayhem over a lossy link. Every client drafts augments with predicted
    /// picks, prediction stays stable, and the server's recording re-simulates exactly.
    #[test]
    fn mayhem_drafts_are_predicted_and_replayed() {
        let r = run(&LabConfig {
            profile: LinkProfile::MID,
            clients: 4,
            seconds: 90.0,
            seed: 4,
            fps: 60.0,
            warmup: 5.0,
            scenario: Scenario::Mayhem,
            proxies: true,
            reaction: 0.25,
            margin_override: None,
            augments: None,
        });
        let s = &r.summary;
        assert_eq!(s.hard_resets, 0, "{}", s.row());
        assert!(s.visible_mean < 15.0, "{}", s.row());
        let drafted: std::collections::BTreeSet<u8> = r
            .replay
            .entries
            .iter()
            .flat_map(|e| match e {
                mftr_server::ReplayEntry::Commands { commands, .. } => commands.clone(),
                _ => Vec::new(),
            })
            .filter(|c| matches!(c.kind, mftr_sim::CommandKind::PickAugment(_)))
            .map(|c| c.player.0)
            .collect();
        assert_eq!(drafted.len(), 4, "every client drafted: {drafted:?}");
        let check = r.replay.verify();
        assert_eq!(check.mismatch, None);
        assert_eq!(check.final_hash, r.server_hash);
    }

    /// M3 exit: ARAM: Mayhem under Hyper rules, every champion with Multishot, Echo and
    /// Broadside, ten clients over the MID link. Downstream stays within the 32 KB/s per client
    /// target and the server within its 3 ms tick budget, with no hard resets, no fog leaks,
    /// and dodges judged as the server judges them (a volley hits each champion once).
    #[test]
    fn hyper_multishot_stress_stays_within_budgets() {
        let r = run(&LabConfig {
            profile: LinkProfile::MID,
            clients: 10,
            seconds: 60.0,
            seed: 1,
            fps: 60.0,
            warmup: 5.0,
            scenario: Scenario::Hyper,
            proxies: true,
            reaction: 0.25,
            margin_override: None,
            augments: Some([24, 25, 26, 0]),
        });
        let s = &r.summary;
        assert_eq!(s.hard_resets, 0, "{}", s.row());
        assert_eq!(r.fog_violations, 0);
        assert!(s.dodge.enemy_missiles >= 600, "the stress happened: {}", s.dodge_row());
        assert!(s.down_kbps < 32.0, "{}", s.row());
        assert!(r.tick_ms_mean < 3.0, "tick {:.3} ms (max {:.3})", r.tick_ms_mean, r.tick_ms_max);
        assert!(s.ghost_rate() < 0.02, "{}", s.dodge_row());
        // Phantom hits (shown, not dealt) run near 4% here against 2% without Multishot: three
        // missiles per cast triple the chances that a proxy misses an interception.
        assert!((s.dodge.phantom_hits as f64) <= 0.06 * s.dodge.near_misses as f64, "{}", s.dodge_row());
    }

    /// M2 slice 1: ARAM on The Bridge with 3v3 bots (waves, turrets, relics, fountains): stable
    /// prediction, nothing leaked through fog, and every bot both kills and dies.
    #[test]
    fn aram_bridge_runs_cleanly() {
        let r = run(&LabConfig {
            profile: LinkProfile::MID,
            clients: 6,
            seconds: 240.0,
            seed: 3,
            fps: 60.0,
            warmup: 5.0,
            scenario: Scenario::Aram,
            proxies: true,
            reaction: 0.25,
            margin_override: None,
            augments: None,
        });
        let s = &r.summary;
        assert_eq!(s.hard_resets, 0, "{}", s.row());
        assert!(s.visible_mean < 15.0, "{}", s.row());
        assert_eq!(r.fog_violations, 0);
        assert!(s.deaths.iter().all(|d| *d >= 1), "{}", s.duel_row());
        assert!(s.ghost_rate() < 0.02, "{}", s.dodge_row());
        // M2 slice 2: everyone earns experience and gold (ARAM starts at level 3, 1,400 gold).
        assert!(s.levels.iter().all(|l| *l >= 5), "{}", s.duel_row());
        // M2 slice 3: bots follow their build paths whenever they respawn.
        assert!(s.items.iter().all(|n| *n >= 2), "{}", s.duel_row());
        // M2 slice 5: the server's recording of this networked session re-simulates exactly.
        let check = r.replay.verify();
        assert!(check.hashes_checked >= 20, "{check:?}");
        assert_eq!(check.mismatch, None);
        assert_eq!(check.final_hash, r.server_hash);
    }

    /// M2 slice 5: a real client session goes through champion select over a lossy link
    /// (rerolls, readies up), gets its Welcome when the match starts, and plays the champion
    /// it ended up with. A spectator joins too and sees both teams.
    #[test]
    fn champion_select_to_playing_end_to_end() {
        let cfg = ServerConfig { seed: 2, bots: 10, lobby: true, scenario: Scenario::Aram, ..Default::default() };
        let mut server = ServerCore::new(cfg, 0.0);
        let mut session = ClientSession::new();
        let mut watcher = ClientSession::new();
        watcher.set_spectate(true);
        let links = |a, b| (SimLink::new(LinkProfile::TYPICAL, a), SimLink::new(LinkProfile::TYPICAL, b));
        let ((mut up, mut down), (mut wup, mut wdown)) = (links(1, 2), links(3, 4));
        let (mut t, mut next_hello) = (0.0, 0.0);
        let (mut rerolled, mut readied, mut picked) = (false, false, None);
        while t < 12.0 {
            for (key, link) in [(1, &mut up), (2, &mut wup)] {
                while let Some(p) = link.recv(t) {
                    for (to, bytes) in server.handle_packet(key, &p, t) {
                        if to == 1 { down.send(bytes, t) } else { wdown.send(bytes, t) }
                    }
                }
            }
            if t >= server.next_tick_due() {
                for (to, bytes) in server.step(t) {
                    if to == 1 { down.send(bytes, t) } else { wdown.send(bytes, t) }
                }
            }
            while let Some(p) = down.recv(t) {
                session.handle_packet(&p, t);
            }
            while let Some(p) = wdown.recv(t) {
                watcher.handle_packet(&p, t);
            }
            session.update(t);
            watcher.update(t);
            if session.phase() == Phase::Lobby && t >= next_hello {
                let l = session.lobby().unwrap().clone();
                let me = *l.slots.iter().find(|s| s.player == l.you).unwrap();
                if !rerolled {
                    up.send(session.lobby_packet(mftr_net::msg::LobbyAction::Reroll), t);
                    rerolled = true;
                } else if me.rerolls < mftr_server::lobby::REROLLS && !readied {
                    picked = Some(me.champion);
                    up.send(session.lobby_packet(mftr_net::msg::LobbyAction::Ready(true)), t);
                    readied = true;
                }
            }
            if matches!(session.phase(), Phase::Connecting | Phase::Lobby) && t >= next_hello {
                up.send(session.hello_packet(t), t);
                wup.send(watcher.hello_packet(t), t);
                next_hello = t + 0.25;
            }
            if session.phase() == Phase::Playing && session.should_send(t) {
                up.send(session.input_packet(t), t);
            }
            if watcher.phase() == Phase::Playing && watcher.should_send(t) {
                wup.send(watcher.input_packet(t), t);
            }
            if matches!(watcher.phase(), Phase::Connecting | Phase::Joining) && t >= next_hello {
                wup.send(watcher.hello_packet(t), t);
            }
            t += 0.001;
        }
        assert_eq!(session.phase(), Phase::Playing, "the match started");
        assert!(readied && t > 4.0);
        assert_eq!(Some(session.champion()), picked, "the rerolled champion");
        assert!(session.token() != 0);
        assert_eq!(server.game().player_count(), 10);
        assert!(watcher.is_spectator() && watcher.phase() == Phase::Playing);
        let teams: std::collections::BTreeSet<_> = watcher
            .remote_render_units(t)
            .iter()
            .filter(|u| u.kind == mftr_sim::UnitKind::Champion)
            .map(|u| u.team as u8)
            .collect();
        assert_eq!(teams.len(), 2, "the spectator sees both teams' champions");
    }

    /// After a Base falls, the same client session goes back to champion select (no
    /// reconnect), rerolls, and plays the next match with its new champion. A spectator stays
    /// connected through it all.
    #[test]
    fn champion_select_after_each_match_end_to_end() {
        champion_select_after_a_match(0.0);
    }

    /// The same after a 20-minute match: the server has sent more than 32,768 packets since
    /// the Welcome, so 16-bit packet sequences no longer tell whether champion select is news.
    #[test]
    fn champion_select_after_a_long_match_end_to_end() {
        champion_select_after_a_match(20.0 * 60.0);
    }

    /// Plays a first match for `linger` seconds (its structures held whole, so nobody wins),
    /// then wins it at once.
    fn champion_select_after_a_match(linger: f64) {
        use mftr_net::msg::LobbyAction;
        use mftr_sim::{UnitId, UnitKind, Vec2};
        let cfg = ServerConfig { seed: 5, bots: 10, lobby: true, scenario: Scenario::Aram, ..Default::default() };
        let mut server = ServerCore::new(cfg, 0.0);
        let mut session = ClientSession::new();
        let mut watcher = ClientSession::new();
        watcher.set_spectate(true);
        let links = |a, b| (SimLink::new(LinkProfile::TYPICAL, a), SimLink::new(LinkProfile::TYPICAL, b));
        let ((mut up, mut down), (mut wup, mut wdown)) = (links(5, 6), links(7, 8));
        let (mut t, mut next_hello, mut next_attack) = (0.0, 0.0, 0.0);
        // Matches played, the champion of each, and the reroll in the second champion select.
        let (mut matches, mut champions, mut rerolled) = (0, Vec::new(), None);
        let (mut base, mut was_playing, mut rig_at) = (None, false, f64::INFINITY);
        while t < 60.0 + linger && champions.len() < 2 {
            for (key, link) in [(1, &mut up), (2, &mut wup)] {
                while let Some(p) = link.recv(t) {
                    for (to, bytes) in server.handle_packet(key, &p, t) {
                        if to == 1 { down.send(bytes, t) } else { wdown.send(bytes, t) }
                    }
                }
            }
            if t >= server.next_tick_due() {
                for (to, bytes) in server.step(t) {
                    if to == 1 { down.send(bytes, t) } else { wdown.send(bytes, t) }
                }
            }
            while let Some(p) = down.recv(t) {
                session.handle_packet(&p, t);
            }
            while let Some(p) = wdown.recv(t) {
                watcher.handle_packet(&p, t);
            }
            session.update(t);
            watcher.update(t);
            let entered = session.phase() == Phase::Playing && !was_playing;
            was_playing = session.phase() == Phase::Playing;
            match session.phase() {
                Phase::Lobby if t >= next_hello => {
                    let l = session.lobby().unwrap().clone();
                    let me = *l.slots.iter().find(|s| s.player == l.you).unwrap();
                    if matches == 1 && rerolled.is_none() && me.rerolls == mftr_server::lobby::REROLLS {
                        up.send(session.lobby_packet(LobbyAction::Reroll), t);
                    } else {
                        if matches == 1 && me.rerolls < mftr_server::lobby::REROLLS {
                            rerolled = Some(me.champion);
                        }
                        up.send(session.lobby_packet(LobbyAction::Ready(true)), t);
                    }
                }
                Phase::Playing if entered => {
                    champions.push(session.champion());
                    matches += 1;
                    if matches == 1 {
                        rig_at = t + linger;
                    }
                }
                Phase::Playing if matches == 1 && base.is_none() && t < rig_at => {
                    let w = server.world_mut();
                    let ids: Vec<UnitId> = w.units().iter().filter(|u| u.kind.is_structure()).map(|u| u.id).collect();
                    for id in ids {
                        let u = w.unit_mut(id).unwrap();
                        u.state.health = u.stats.max_health;
                    }
                }
                Phase::Playing if matches == 1 && base.is_none() => {
                    {
                        // End this match quickly: only the enemy Base is left, and we stand by it.
                        let team = session.team();
                        let w = server.world_mut();
                        let b = w.units().iter().find(|u| u.kind == UnitKind::Base && u.team != team).unwrap();
                        let (id, pos, tier) = (b.id, b.state.pos, b.tier);
                        let guards: Vec<UnitId> = w
                            .units()
                            .iter()
                            .filter(|u| u.team != team && u.kind.is_structure() && u.tier > 0 && u.tier < tier)
                            .map(|u| u.id)
                            .collect();
                        for g in guards {
                            w.despawn(g);
                        }
                        w.unit_mut(id).unwrap().state.health = 1.0;
                        let me = w.unit_mut(session.unit()).unwrap();
                        me.state.pos = pos + Vec2::new(if team == Team::Blue { -300.0 } else { 300.0 }, 0.0);
                        base = Some(id);
                    }
                }
                Phase::Playing if matches == 1 && t >= next_attack => {
                    if let Some(id) = base
                        && server.world().unit(id).is_some()
                    {
                        session.attack(id, t);
                    }
                    next_attack = t + 0.5;
                }
                _ => {}
            }
            if matches!(session.phase(), Phase::Connecting | Phase::Lobby) && t >= next_hello {
                up.send(session.hello_packet(t), t);
                next_hello = t + 0.25;
            }
            if session.phase() == Phase::Playing && session.should_send(t) {
                up.send(session.input_packet(t), t);
            }
            if matches!(watcher.phase(), Phase::Connecting | Phase::Joining) && t >= next_hello {
                wup.send(watcher.hello_packet(t), t);
            }
            if watcher.phase() == Phase::Playing && watcher.should_send(t) {
                wup.send(watcher.input_packet(t), t);
            }
            t += if matches == 1 && t < rig_at { 1.0 / 60.0 } else { 0.001 };
        }
        assert_eq!(champions.len(), 2, "two matches by {t:.1} s");
        assert!(rerolled.is_some(), "a second champion select, with rerolls");
        assert_eq!(champions[1], rerolled.unwrap(), "the next match is played with the new champion");
        assert_eq!(server.game().player_count(), 10);
        assert!(server.world().game().winner.is_none());
        assert_eq!(session.game_mode(), mftr_net::msg::GameMode::Aram);
        // Prediction works in the new match: our champion is where the server has it.
        for _ in 0..300 {
            t += 1.0 / 30.0;
            if t >= server.next_tick_due() {
                for (to, bytes) in server.step(t) {
                    if to == 1 {
                        down.send(bytes, t);
                    }
                }
            }
            while let Some(p) = down.recv(t) {
                session.handle_packet(&p, t);
            }
            session.update(t);
            if session.should_send(t) {
                up.send(session.input_packet(t), t);
            }
            while let Some(p) = up.recv(t) {
                for (to, bytes) in server.handle_packet(1, &p, t) {
                    if to == 1 {
                        down.send(bytes, t);
                    }
                }
            }
        }
        assert_eq!(session.phase(), Phase::Playing);
        let server_pos = server.world().unit(session.unit()).unwrap().state.pos;
        let drawn = session.own_render_position(t).unwrap();
        assert!(drawn.distance(server_pos) < 50.0, "drawn {drawn:?}, server {server_pos:?}");
        assert!(watcher.is_spectator(), "the spectator is still watching");
    }

    /// Nothing blocks where a destroyed structure stood, on the server or in prediction (dead
    /// units aren't sent, so they never become collision proxies): walking straight through
    /// its spot needs no correction.
    #[test]
    fn walking_through_a_destroyed_turret_is_predicted() {
        use mftr_sim::{SimTime, UnitKind, Vec2};
        let cfg = ServerConfig { seed: 3, scenario: Scenario::Aram, ..Default::default() };
        let mut server = ServerCore::new(cfg, 0.0);
        let mut session = ClientSession::new();
        let (mut up, mut down) = (SimLink::new(LinkProfile::GOOD, 9), SimLink::new(LinkProfile::GOOD, 10));
        let (mut t, mut next_hello) = (0.0, 0.0);
        let mut plan: Option<(Vec2, Vec2)> = None;
        let mut ordered = false;
        while t < 20.0 {
            while let Some(p) = up.recv(t) {
                for (to, bytes) in server.handle_packet(1, &p, t) {
                    if to == 1 {
                        down.send(bytes, t);
                    }
                }
            }
            if t >= server.next_tick_due() {
                for (to, bytes) in server.step(t) {
                    if to == 1 {
                        down.send(bytes, t);
                    }
                }
            }
            while let Some(p) = down.recv(t) {
                session.handle_packet(&p, t);
            }
            session.update(t);
            if matches!(session.phase(), Phase::Connecting) && t >= next_hello {
                up.send(session.hello_packet(t), t);
                next_hello = t + 0.25;
            }
            if session.phase() == Phase::Playing {
                if plan.is_none() {
                    // Our own outer turret falls, and we stand just behind where it stood.
                    let team = session.team();
                    let w = server.world_mut();
                    let turret =
                        w.units().iter().find(|u| u.kind == UnitKind::Turret && u.team == team && u.tier == 1).unwrap();
                    let (id, pos) = (turret.id, turret.state.pos);
                    let u = w.unit_mut(id).unwrap();
                    u.state.health = 0.0;
                    u.state.respawn_at = Some(SimTime(u64::MAX));
                    let ahead = if team == Team::Blue { 1.0 } else { -1.0 };
                    w.unit_mut(session.unit()).unwrap().state.pos = pos - Vec2::new(500.0 * ahead, 0.0);
                    plan = Some((pos, pos + Vec2::new(500.0 * ahead, 0.0)));
                    session.stats.corrections.clear();
                } else if let Some((_, goal)) = plan
                    && !ordered
                    && t > 3.0
                {
                    // Straight through the turret's spot, once the teleport has been absorbed.
                    session.stats.corrections.clear();
                    session.move_to(goal, t);
                    ordered = true;
                }
                if session.should_send(t) {
                    up.send(session.input_packet(t), t);
                }
            }
            t += 0.001;
        }
        let (spot, goal) = plan.expect("playing");
        let server_pos = server.world().unit(session.unit()).unwrap().state.pos;
        assert!(server_pos.distance(goal) < 5.0, "the server walked us through {spot:?}: at {server_pos:?}");
        let big: Vec<f32> = session.stats.corrections.iter().copied().filter(|c| *c > 1.0).collect();
        assert!(big.is_empty(), "prediction detoured around the dead turret: {big:?}");
    }

    /// The ultimate levels like any ability once it unlocks (level 6): the client's level-up
    /// order for R reaches the server and ranks it up, predicted and confirmed.
    #[test]
    fn the_ultimate_levels_up_at_six_end_to_end() {
        let cfg = ServerConfig { seed: 3, scenario: Scenario::Aram, ..Default::default() };
        let mut server = ServerCore::new(cfg, 0.0);
        let mut session = ClientSession::new();
        let (mut up, mut down) = (SimLink::new(LinkProfile::GOOD, 11), SimLink::new(LinkProfile::GOOD, 12));
        let (mut t, mut next_hello, mut leveled_at) = (0.0, 0.0, None);
        while t < 8.0 {
            while let Some(p) = up.recv(t) {
                for (_, bytes) in server.handle_packet(1, &p, t) {
                    down.send(bytes, t);
                }
            }
            if t >= server.next_tick_due() {
                for (_, bytes) in server.step(t) {
                    down.send(bytes, t);
                }
            }
            while let Some(p) = down.recv(t) {
                session.handle_packet(&p, t);
            }
            session.update(t);
            if matches!(session.phase(), Phase::Connecting) && t >= next_hello {
                up.send(session.hello_packet(t), t);
                next_hello = t + 0.25;
            }
            if session.phase() == Phase::Playing {
                if leveled_at.is_none() && t > 2.0 {
                    // Level 6 with one point to spend, as if it had just been reached.
                    let me = server.world_mut().unit_mut(session.unit()).unwrap();
                    me.state.progress.level = 6;
                    me.state.progress.points = 1;
                    leveled_at = Some(t);
                } else if let Some(at) = leveled_at
                    && t > at + 1.0
                    && t - 0.001 <= at + 1.0
                {
                    session.level_up(3, t);
                }
                if session.should_send(t) {
                    up.send(session.input_packet(t), t);
                }
            }
            t += 0.001;
        }
        let rank = server.world().unit(session.unit()).unwrap().state.progress.ranks[3];
        assert_eq!(rank, 1, "the server ranked R up");
        assert_eq!(session.own_state_now().unwrap().progress.ranks[3], 1, "and the client shows it");
    }

    /// M3 slices 2 and 3: a Titan with Multishot, Echo and Broadside casts over a jittery link.
    /// The client predicts the whole volley and the echo (keyed by cast and shot), each one is
    /// confirmed by the server's own, its hitbox grows with the server's, and prediction never
    /// corrects.
    #[test]
    fn transformed_casts_are_predicted() {
        use mftr_client::missiles::Side;
        let cfg = ServerConfig { seed: 3, scenario: Scenario::Mayhem, ..Default::default() };
        let mut server = ServerCore::new(cfg, 0.0);
        let mut session = ClientSession::new();
        session.set_champion_request(Some(mftr_sim::ChampionId::Ember));
        let (mut up, mut down) = (SimLink::new(LinkProfile::MID, 11), SimLink::new(LinkProfile::MID, 12));
        let (mut t, mut next_hello, mut augmented) = (0.0, 0.0, false);
        let (mut casts, mut most_predicted, mut most_confirmed) = (0, 0, 0);
        let mut next_cast = 0.0;
        while t < 30.0 {
            while let Some(p) = up.recv(t) {
                for (_, bytes) in server.handle_packet(1, &p, t) {
                    down.send(bytes, t);
                }
            }
            if t >= server.next_tick_due() {
                for (_, bytes) in server.step(t) {
                    down.send(bytes, t);
                }
            }
            while let Some(p) = down.recv(t) {
                session.handle_packet(&p, t);
            }
            session.update(t);
            match session.phase() {
                Phase::Connecting if t >= next_hello => {
                    up.send(session.hello_packet(t), t);
                    next_hello = t + 0.25;
                }
                Phase::Playing => {
                    if !augmented {
                        // Grant the augments on the server; the client learns them from its state.
                        let u = server.world_mut().unit_mut(session.unit()).unwrap();
                        u.state.progress.augments = [24, 25, 26, 27];
                        u.state.progress.drafted = 4;
                        u.state.progress.offer = [0; 3];
                        u.state.progress.ranks = [1; 4];
                        augmented = true;
                        next_cast = t + 2.0;
                    }
                    if t >= next_cast && session.own_state_now().is_some_and(|s| s.progress.augments[0] == 24) {
                        let own = session.own_render_position(t).unwrap();
                        if session.cast(0, own + mftr_sim::Vec2::new(0.0, 600.0), t).is_some() {
                            casts += 1;
                        }
                        next_cast = t + 5.0;
                    }
                    let own: Vec<_> = session.missiles_render(t).into_iter().filter(|m| m.side == Side::Own).collect();
                    let predicted = own.iter().filter(|m| m.key > u32::MAX / 2).count();
                    most_predicted = most_predicted.max(predicted);
                    most_confirmed = most_confirmed.max(own.len() - predicted);
                    if session.should_send(t) {
                        up.send(session.input_packet(t), t);
                    }
                }
                _ => {}
            }
            t += 0.002;
        }
        assert!(casts >= 4, "{casts} casts");
        assert_eq!(most_predicted, 3, "the volley is predicted at once");
        assert!(most_confirmed >= 3, "and confirmed by the server: {most_confirmed}");
        assert_eq!(session.stats.hard_resets, 0);
        assert!(session.stats.corrections.iter().all(|c| *c < 1.0), "{:?}", session.stats.corrections);
        assert!(session.book_is_settled(), "every predicted missile was confirmed");
        let titan = mftr_sim::world::CHAMPION_GAMEPLAY_RADIUS * mftr_sim::augments::TITAN_SCALE;
        assert_eq!(session.own_radius(), titan);
        assert_eq!(server.world().unit(session.unit()).unwrap().gameplay_radius, titan);
    }

    /// A2 (D52): buffered casts, cut follow-throughs and hard locks are predicted exactly over a
    /// jittery link, and the server confirms the buffered casts. A scripted Vesper repeats, every
    /// 11 s (so every cast is off cooldown): W then Q at once (Q waits in the buffer for W's windup), Tumble, Q then a move right
    /// after it fires (the cancel), and the ultimate with a cast and a move inside its hard lock.
    #[test]
    fn timing_contract_is_predicted() {
        use mftr_client::missiles::Side;
        use mftr_sim::Vec2;
        let cfg = ServerConfig { seed: 5, scenario: Scenario::Empty, ..Default::default() };
        let mut server = ServerCore::new(cfg, 0.0);
        let mut session = ClientSession::new();
        session.set_champion_request(Some(mftr_sim::ChampionId::Vesper));
        let (mut up, mut down) = (SimLink::new(LinkProfile::MID, 21), SimLink::new(LinkProfile::MID, 22));
        let (mut t, mut next_hello, mut start) = (0.0, 0.0, None);
        let (mut buffered_seen, mut recovering_seen, mut hard_seen) = (0, 0, 0);
        let mut confirmed: std::collections::BTreeSet<u32> = Default::default();
        let mut done: std::collections::BTreeSet<u32> = Default::default();
        while t < 45.0 {
            while let Some(p) = up.recv(t) {
                for (_, bytes) in server.handle_packet(1, &p, t) {
                    down.send(bytes, t);
                }
            }
            if t >= server.next_tick_due() {
                for (_, bytes) in server.step(t) {
                    down.send(bytes, t);
                }
            }
            while let Some(p) = down.recv(t) {
                session.handle_packet(&p, t);
            }
            session.update(t);
            match session.phase() {
                Phase::Connecting if t >= next_hello => {
                    up.send(session.hello_packet(t), t);
                    next_hello = t + 0.25;
                }
                Phase::Playing => {
                    let t0 = *start.get_or_insert(t + 1.0);
                    let own = session.own_render_position(t).unwrap();
                    let east = own + Vec2::new(600.0, 0.0);
                    // (offset in the 10 s cycle, step id): each step once per cycle.
                    let cycle = ((t - t0) / 11.0).floor();
                    let at = t - t0 - cycle * 11.0;
                    let id = |step: u32| cycle.max(0.0) as u32 * 16 + step;
                    let mut once = |step: u32, when: f64| t >= t0 && at >= when && done.insert(id(step));
                    if once(0, 0.0) {
                        session.cast(1, own + Vec2::new(500.0, 0.0), t); // W: delayed area
                    }
                    if once(1, 0.05) {
                        session.cast(0, east, t); // Q during W's windup: buffered
                    }
                    if once(2, 2.0) {
                        session.cast(2, own + Vec2::new(0.0, 300.0), t); // E: Tumble
                    }
                    if once(3, 5.5) {
                        session.cast(0, east, t);
                    }
                    if once(4, 5.9) {
                        session.move_to(own + Vec2::new(0.0, -200.0), t); // cut Q's follow-through
                    }
                    if once(5, 8.0) {
                        session.cast(3, east, t); // R: hard lock after it fires
                    }
                    if once(6, 8.33) {
                        session.cast(4, own + Vec2::new(-300.0, 0.0), t); // Blink, buffered in the lock
                        session.move_to(own + Vec2::new(0.0, 200.0), t);
                    }
                    if let Some((tick, s)) = session.own_state_latest() {
                        let now = mftr_sim::SimTime::end_of(tick);
                        buffered_seen += s.buffered.is_some() as u32;
                        recovering_seen += s.recovering(now) as u32;
                        hard_seen += s.hard_locked(now) as u32;
                    }
                    for m in session.missiles_render(t).into_iter().filter(|m| m.side == Side::Own) {
                        if m.key < u32::MAX / 2 {
                            confirmed.insert(m.key);
                        }
                    }
                    if session.should_send(t) {
                        up.send(session.input_packet(t), t);
                    }
                }
                _ => {}
            }
            t += 0.002;
        }
        // Four cycles: the buffered Q and the cancelled Q each fire a confirmed Longshot, and
        // the ultimate's net (every other cycle: 20 s cooldown) is a missile too.
        assert!(confirmed.len() >= 10, "{} confirmed own missiles", confirmed.len());
        assert!(
            buffered_seen > 0 && recovering_seen > 0 && hard_seen > 0,
            "{buffered_seen} {recovering_seen} {hard_seen}"
        );
        assert_eq!(session.stats.hard_resets, 0);
        assert!(session.stats.corrections.iter().all(|c| *c < 1.0), "{:?}", session.stats.corrections);
        let me = server.world().unit(session.unit()).unwrap().state;
        assert!(me.buffered.is_none(), "nothing left waiting");
    }

    /// Q13: over a lossy, jittery link with moving minions, every snapshot the client
    /// reconstructs from deltas equals, bit for bit, what the server recorded for it.
    #[test]
    fn delta_snapshots_reconstruct_exactly() {
        let mut server = ServerCore::new(ServerConfig { scenario: Scenario::MinionSandbox, ..Default::default() }, 0.0);
        let mut session = ClientSession::new();
        let (mut up, mut down) = (SimLink::new(LinkProfile::ROUGH, 1), SimLink::new(LinkProfile::ROUGH, 2));
        let mut bot = ClickBot::new(9);
        let (mut t, mut next_hello, mut checked) = (0.0, 0.0, 0u32);
        while t < 40.0 {
            while let Some(p) = up.recv(t) {
                for (_, bytes) in server.handle_packet(1, &p, t) {
                    down.send(bytes, t);
                }
            }
            if t >= server.next_tick_due() {
                for (_, bytes) in server.step(t) {
                    down.send(bytes, t);
                }
            }
            while let Some(p) = down.recv(t) {
                session.handle_packet(&p, t);
                if let Some((tick, units)) = session.reconstructed()
                    && let Some(sent) = server.sent_records(1, tick)
                {
                    assert_eq!(units, &sent, "tick {}", tick.0);
                    checked += 1;
                }
            }
            session.update(t);
            match session.phase() {
                Phase::Connecting | Phase::Lobby if t >= next_hello => {
                    up.send(session.hello_packet(t), t);
                    next_hello = t + 0.25;
                }
                Phase::Playing => {
                    bot.act(&mut session, t, t);
                    if session.should_send(t) {
                        up.send(session.input_packet(t), t);
                    }
                }
                _ => {}
            }
            t += 0.001;
        }
        assert!(checked > 800, "only {checked} snapshots checked");
    }
}
