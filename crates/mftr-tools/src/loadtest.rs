//! Load test (13 §8): many matches in one hub, in process, with simulated players clicking
//! around the map (server bots fill the other seats and fight). Clients and links run on a
//! virtual clock with perfect links; only the hub's own work is timed, so the result is what
//! a server spends per match, without encryption or sockets.

use crate::report::ClickBot;
use mftr_client::{ClientSession, Phase};
use mftr_server::hub::{self, Hub, HubConfig};
use mftr_server::{Scenario, ServerConfig};
use std::time::Instant;

pub struct LoadConfig {
    pub matches: usize,
    /// Simulated players per match (bots take the other seats).
    pub humans: usize,
    pub scenario: Scenario,
    /// Seconds measured, after every match has started.
    pub seconds: f64,
    pub workers: usize,
    pub seed: u64,
}

#[derive(Debug)]
pub struct LoadResult {
    pub matches: usize,
    pub clients: usize,
    pub ticks: u64,
    /// Hub time per tick (all matches): mean and worst, in milliseconds of wall time.
    pub step_ms: f64,
    pub step_ms_max: f64,
    /// Packets handled from clients, total milliseconds over the measured span.
    pub handle_ms: f64,
    /// Share of one core the hub kept busy (step + packets, over the measured span).
    pub busy: f64,
    /// Bytes per second to and from each client.
    pub down_per_client: f64,
    pub up_per_client: f64,
}

impl LoadResult {
    pub fn header() -> &'static str {
        "matches clients workers  tick avg  tick max   busy/core  per match   down/client  up/client"
    }

    pub fn row(&self, workers: usize) -> String {
        format!(
            "{:>7} {:>7} {:>7}  {:>6.2}ms  {:>6.2}ms  {:>8.1}%  {:>8.2}%  {:>8.1}KB/s {:>6.2}KB/s",
            self.matches,
            self.clients,
            workers,
            self.step_ms,
            self.step_ms_max,
            self.busy * 100.0,
            self.busy * 100.0 / self.matches.max(1) as f64,
            self.down_per_client / 1024.0,
            self.up_per_client / 1024.0,
        )
    }
}

struct Client {
    session: ClientSession,
    bot: ClickBot,
    joins_at: f64,
    next_hello: f64,
    readied: bool,
}

pub fn run(cfg: &LoadConfig) -> LoadResult {
    let queues = hub::hosted_queues(&[cfg.scenario], &[]);
    let queue = queues.iter().position(|q| !q.coop).expect("a Versus queue") as u8;
    let mut hub = Hub::new(HubConfig {
        queues,
        base: ServerConfig { seed: cfg.seed, max_players: 10, bots: 10, lobby: true, ..Default::default() },
        max_matches: cfg.matches,
        workers: cfg.workers,
        development: false,
    });
    let size = mftr_sim::map::MapId::shared(cfg.scenario.map()).size;
    let n = cfg.matches * cfg.humans;
    let mut clients: Vec<Client> = (0..n)
        .map(|i| {
            let mut session = ClientSession::new();
            session.choose_queue(queue);
            let mut bot = ClickBot::new(cfg.seed.wrapping_mul(1000) + i as u64);
            (bot.arena_min, bot.arena_max) = (0.0, size.x.min(size.y));
            // One match fills before the next opens: humans arrive a match at a time.
            Client { session, bot, joins_at: i as f64 * 0.02, next_hello: 0.0, readied: false }
        })
        .collect();

    let dt = 1.0 / 30.0;
    let mut t = 0.0;
    let mut measuring_from: Option<f64> = None;
    let (mut step_s, mut step_max, mut handle_s, mut ticks) = (0.0f64, 0.0f64, 0.0f64, 0u64);
    let (mut down, mut up) = (0u64, 0u64);
    let end_by = 120.0 + cfg.seconds;
    while t < end_by {
        t += dt;
        let measuring = measuring_from.is_some();
        // Clients act; their packets reach the hub at once, and its replies them.
        let mut replies: Vec<(u64, Vec<u8>)> = Vec::new();
        for (i, c) in clients.iter_mut().enumerate() {
            if t < c.joins_at {
                continue;
            }
            let key = i as u64 + 1;
            c.session.update(t);
            let mut packets = Vec::new();
            match c.session.phase() {
                Phase::Connecting | Phase::Lobby if t >= c.next_hello => {
                    packets.push(c.session.hello_packet(t));
                    if c.session.phase() == Phase::Lobby && !c.readied {
                        packets.push(c.session.lobby_packet(mftr_net::msg::LobbyAction::Ready(true)));
                        c.readied = true;
                    }
                    c.next_hello = t + 0.25;
                }
                Phase::Playing => {
                    c.bot.act(&mut c.session, t, t);
                    if c.session.should_send(t) {
                        packets.push(c.session.input_packet(t));
                    }
                }
                _ => {}
            }
            for p in packets {
                if measuring {
                    up += p.len() as u64;
                }
                let started = Instant::now();
                let out = hub.handle_packet(key, &p, t);
                if measuring {
                    handle_s += started.elapsed().as_secs_f64();
                }
                replies.extend(out);
            }
        }
        for (to, bytes) in replies {
            if measuring {
                down += bytes.len() as u64;
            }
            if let Some(c) = clients.get_mut(to as usize - 1) {
                c.session.handle_packet(&bytes, t);
            }
        }
        // The hub ticks.
        while hub.next_tick_due().is_some_and(|due| t >= due) {
            let started = Instant::now();
            let out = hub.step(t);
            let cost = started.elapsed().as_secs_f64();
            if measuring {
                step_s += cost;
                step_max = step_max.max(cost);
                ticks += 1;
            }
            for (to, bytes) in out {
                if measuring {
                    down += bytes.len() as u64;
                }
                if let Some(c) = clients.get_mut(to as usize - 1) {
                    c.session.handle_packet(&bytes, t);
                }
            }
            if hub.next_tick_due().is_some_and(|due| due <= t - 1.0) {
                break; // far behind: catch up next frame
            }
        }
        if measuring_from.is_none()
            && clients.iter().all(|c| c.session.phase() == Phase::Playing)
            && hub.instances().len() == cfg.matches
        {
            measuring_from = Some(t);
        }
        if measuring_from.is_some_and(|from| t - from >= cfg.seconds) {
            break;
        }
    }
    let span = measuring_from.map_or(0.0, |from| t - from).max(1e-9);
    // Steps happen once per tick for all matches together (they share the tick grid).
    let tick_count = ticks.max(1) as f64;
    LoadResult {
        matches: hub.instances().len(),
        clients: n,
        ticks,
        step_ms: step_s * 1e3 / tick_count,
        step_ms_max: step_max * 1e3,
        handle_ms: handle_s * 1e3,
        busy: (step_s + handle_s) / span,
        down_per_client: down as f64 / span / n.max(1) as f64,
        up_per_client: up as f64 / span / n.max(1) as f64,
    }
}
