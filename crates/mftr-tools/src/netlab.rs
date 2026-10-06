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
}

pub fn run(cfg: &LabConfig) -> LabResult {
    const STEP: f64 = 0.0005;
    let mut server =
        ServerCore::new(ServerConfig { seed: cfg.seed, scenario: cfg.scenario, ..Default::default() }, 0.0);
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
                    Scenario::Duel | Scenario::Aram => Bot::Duel(DuelBot::new(s, cfg.reaction)),
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
                Phase::Connecting => {
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
                    if let Some(pos) = c.session.own_render_position(local) {
                        c.jumps.observe(pos, c.session.visible_correction(), frame_dt, CHAMPION_MOVE_SPEED);
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
                Phase::Connecting if t >= next_hello => {
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
