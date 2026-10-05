//! Headless Netcode Lab (03 §14): one server and N scripted clients over link-conditioned
//! links, in deterministic virtual time. Same seed → same result, so it's usable in CI.

use crate::report::{ClickBot, JumpMeter, Summary};
use mftr_client::{ClientSession, Phase};
use mftr_net::conditioner::{LinkProfile, SimLink};
use mftr_server::{ServerConfig, ServerCore};
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
}

struct LabClient {
    session: ClientSession,
    up: SimLink,
    down: SimLink,
    bot: ClickBot,
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
}

pub fn run(cfg: &LabConfig) -> LabResult {
    const STEP: f64 = 0.0005;
    let mut server = ServerCore::new(ServerConfig { seed: cfg.seed, ..Default::default() }, 0.0);
    let mut clients: Vec<LabClient> = (0..cfg.clients)
        .map(|i| {
            let s = cfg.seed.wrapping_mul(1000) + i as u64;
            LabClient {
                session: ClientSession::new(),
                up: SimLink::new(cfg.profile, s * 2 + 1),
                down: SimLink::new(cfg.profile, s * 2 + 2),
                bot: ClickBot::new(s),
                jumps: JumpMeter::default(),
                clock_offset: 1000.0 + 37.0 * i as f64,
                next_frame: 0.001 * i as f64,
                next_hello: 0.0,
            }
        })
        .collect();

    let frame_dt = 1.0 / cfg.fps;
    let mut t = 0.0;
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
            for (to, bytes) in server.step(t) {
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
                    c.bot.act(&mut c.session, local, t);
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lab(profile: LinkProfile, seconds: f64) -> LabResult {
        run(&LabConfig { profile, clients: 6, seconds, seed: 42, fps: 144.0, warmup: 10.0 })
    }

    #[test]
    fn perfect_link_has_no_corrections() {
        let r = lab(LinkProfile::PERFECT, 20.0);
        assert!(r.summary.commands > 100);
        assert_eq!(r.summary.mismatch_pct, 0.0, "{:?}", r.summary);
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
}
