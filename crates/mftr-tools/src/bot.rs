//! UDP bot client: connects to a real `mftr-server`, clicks around (or, with `--duel`, fights
//! like the Netcode Lab's duel bot: a sparring partner for the Duel Sandbox), and reports
//! netcode stats. A local link conditioner can add latency, jitter and loss on top of the real
//! network.

use crate::report::{ClickBot, DuelBot, JumpMeter, Summary};
use mftr_client::{ClientSession, Phase};
use mftr_net::conditioner::{LinkProfile, SimLink};
use mftr_sim::world::CHAMPION_MOVE_SPEED;
use std::net::UdpSocket;
use std::time::{Duration, Instant};

pub struct BotConfig {
    pub server: String,
    pub profile: LinkProfile,
    pub seconds: f64,
    pub seed: u64,
    pub fps: f64,
    /// Fight with the whole kit instead of wandering.
    pub duel: bool,
    pub champion: Option<mftr_sim::ChampionId>,
}

pub fn run(cfg: &BotConfig) -> std::io::Result<Summary> {
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    socket.connect(&cfg.server)?;
    socket.set_nonblocking(true)?;

    let clock = Instant::now();
    let now = || clock.elapsed().as_secs_f64();
    let mut session = ClientSession::new();
    session.set_champion_request(cfg.champion);
    let mut bot = ClickBot::new(cfg.seed);
    let mut duel = DuelBot::new(cfg.seed, 0.25);
    let mut jumps = JumpMeter::default();
    // Extra simulated impairment in each direction, applied locally.
    let mut up = SimLink::new(cfg.profile, cfg.seed * 2 + 1);
    let mut down = SimLink::new(cfg.profile, cfg.seed * 2 + 2);
    let frame_dt = 1.0 / cfg.fps;
    let mut buf = [0u8; 2048];
    let (mut next_frame, mut next_hello) = (0.0, 0.0);
    let mut played_from = None;

    while now() < cfg.seconds + played_from.unwrap_or(0.0) {
        loop {
            match socket.recv(&mut buf) {
                Ok(n) => down.send(buf[..n].to_vec(), now()),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) => return Err(e),
            }
        }
        let t = now();
        while let Some(p) = down.recv(t) {
            session.handle_packet(&p, t);
        }
        while let Some(p) = up.recv(t) {
            let _ = socket.send(&p);
        }
        if t >= next_frame {
            next_frame = t + frame_dt;
            session.update(t);
            match session.phase() {
                // In champion select a bot just readies up (and keeps the session alive).
                Phase::Lobby if t >= next_hello => {
                    let ready = session.lobby_packet(mftr_net::msg::LobbyAction::Ready(true));
                    up.send(ready, t);
                    up.send(session.hello_packet(t), t);
                    next_hello = t + 0.5;
                }
                Phase::Connecting if t >= next_hello => {
                    up.send(session.hello_packet(t), t);
                    next_hello = t + 0.25;
                    if t > 10.0 {
                        return Err(std::io::Error::new(std::io::ErrorKind::TimedOut, "no reply from server"));
                    }
                }
                Phase::Playing => {
                    played_from.get_or_insert(t);
                    if cfg.duel {
                        duel.act(&mut session, t, t);
                    } else {
                        bot.act(&mut session, t, t);
                    }
                    if session.should_send(t) {
                        up.send(session.input_packet(t), t);
                    }
                    if let Some(pos) = session.own_render_position(t) {
                        jumps.observe(pos, session.visible_correction(), frame_dt, CHAMPION_MOVE_SPEED);
                    }
                }
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_micros(250));
    }
    let _ = socket.send(&session.bye_packet());
    let label = format!("udp+{}", cfg.profile.name);
    Ok(Summary::from_sessions(&label, f64::NAN, cfg.seconds, &[(&session, &jumps)]))
}
