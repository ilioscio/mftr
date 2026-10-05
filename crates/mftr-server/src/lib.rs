//! `mftr-server`: the authoritative match server core.
//!
//! Transport- and clock-agnostic: feed it packets with timestamps, call [`ServerCore::step`]
//! when [`ServerCore::next_tick_due`] has passed, and send the packets it returns. The UDP
//! binary (`main.rs`) and the virtual-time Netcode Lab both drive this same core.

use mftr_net::PROTOCOL_VERSION;
use mftr_net::msg::{self, ClientMessage, CommandReport, RejectReason, RemoteUnit, ServerMessage, Snapshot, TimeEcho};
use mftr_net::packet::{PacketHeader, ReceiveTracker, SendTracker};
use mftr_sim::{
    Brain, Command, MinionKind, PlayerId, QPoint, SubTick, TICK_DT_F64, TICK_HZ, Team, Tick, UnitId, Vec2, World,
};
use std::collections::{BTreeMap, VecDeque};

/// Opaque per-connection address key, assigned by the transport.
pub type ClientKey = u64;

/// Commands later than this are dropped instead of applied late (03a §10.2).
const MAX_LATENESS: f64 = 0.250;
/// Commands targeting further ahead than this are clamped (anti-abuse).
const MAX_LEAD_TICKS: u32 = 90;
const REPORT_REPEAT_TICKS: u32 = 15;
const TIMEOUT: f64 = 10.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    /// Champions only.
    Empty,
    /// M1 slice 1: static minion clumps plus two patrolling waves, to exercise minion block
    /// and client collision proxies (03a §5).
    MinionSandbox,
}

impl Scenario {
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "empty" => Some(Scenario::Empty),
            "minions" => Some(Scenario::MinionSandbox),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub seed: u64,
    pub max_players: u8,
    /// Spawn area: a square from `arena_min` to `arena_max` on both axes.
    pub arena_min: f32,
    pub arena_max: f32,
    pub scenario: Scenario,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self { seed: 1, max_players: 10, arena_min: 500.0, arena_max: 3500.0, scenario: Scenario::Empty }
    }
}

/// Populate the M1 minion sandbox (inside the default 0..4000 u arena).
fn populate(world: &mut World, scenario: Scenario) {
    if scenario != Scenario::MinionSandbox {
        return;
    }
    // Static clumps: hex-packed, adjacent minions touching, like a wave fighting in lane.
    for (center, rings, team) in [
        (Vec2::new(1200.0, 1200.0), 1i32, Team::Red),
        (Vec2::new(2800.0, 1300.0), 2, Team::Blue),
        (Vec2::new(1300.0, 2800.0), 2, Team::Red),
        (Vec2::new(2900.0, 2900.0), 1, Team::Blue),
    ] {
        for q in -rings..=rings {
            for r in -rings..=rings {
                if (q + r).abs() > rings {
                    continue;
                }
                let x = center.x + 50.0 * (q as f32 + r as f32 * 0.5);
                let y = center.y + 50.0 * 0.866_025_4 * r as f32;
                let kind = if (q + r) % 2 == 0 { MinionKind::Melee } else { MinionKind::Caster };
                world.spawn_minion(kind, team, Vec2::new(x, y), None);
            }
        }
    }
    // Two patrolling waves (2 rows of 3) crossing the arena through the middle.
    for (a, b, team) in [
        (Vec2::new(500.0, 2000.0), Vec2::new(3500.0, 2000.0), Team::Blue),
        (Vec2::new(2000.0, 500.0), Vec2::new(2000.0, 3500.0), Team::Red),
    ] {
        let along = (b - a).normalize_or_zero();
        let across = Vec2::new(-along.y, along.x);
        for i in 0..6 {
            let offset = along * (-60.0 * (i % 3) as f32) + across * (if i < 3 { -30.0 } else { 30.0 });
            let pa = QPoint::from_vec2(a + offset);
            let pb = QPoint::from_vec2(b + offset);
            let kind = if i < 3 { MinionKind::Melee } else { MinionKind::Caster };
            world.spawn_minion(kind, team, pa.to_vec2(), Some(Brain::Patrol { a: pa, b: pb, toward_b: true }));
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ServerStats {
    pub ticks: u64,
    pub commands: u64,
    pub commands_late: u64,
    pub commands_dropped: u64,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub decode_errors: u64,
}

struct Conn {
    player: PlayerId,
    unit: UnitId,
    recv: ReceiveTracker,
    send: SendTracker,
    highest_seq: u32,
    reports: VecDeque<(Tick, CommandReport)>,
    echo: Option<(u32, f64)>,
    last_heard: f64,
}

pub struct ServerCore {
    cfg: ServerConfig,
    world: World,
    start: f64,
    conns: BTreeMap<ClientKey, Conn>,
    queue: Vec<Command>,
    pub stats: ServerStats,
}

impl ServerCore {
    /// `start`: the server-clock time (seconds) at which tick 0 ends.
    pub fn new(cfg: ServerConfig, start: f64) -> Self {
        let mut world = World::new(cfg.seed);
        populate(&mut world, cfg.scenario);
        Self { cfg, world, start, conns: BTreeMap::new(), queue: Vec::new(), stats: ServerStats::default() }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn player_count(&self) -> usize {
        self.conns.len()
    }

    /// Wall time at which tick `k`'s interval ends, i.e. when it may be simulated.
    pub fn tick_time(&self, k: Tick) -> f64 {
        self.start + k.0 as f64 * TICK_DT_F64
    }

    pub fn next_tick_due(&self) -> f64 {
        self.tick_time(self.world.tick().next())
    }

    fn header(conn: &mut Conn) -> PacketHeader {
        let (ack, ack_bits) = conn.recv.ack_fields();
        PacketHeader { seq: conn.send.next_seq(), ack, ack_bits }
    }

    /// Handle one incoming datagram. Returns packets to send immediately.
    pub fn handle_packet(&mut self, from: ClientKey, bytes: &[u8], now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        self.stats.bytes_in += bytes.len() as u64;
        let (header, message) = match msg::decode_client(bytes) {
            Ok(v) => v,
            Err(_) => {
                self.stats.decode_errors += 1;
                return Vec::new();
            }
        };
        let mut out = Vec::new();
        match message {
            ClientMessage::Hello { protocol, client_time_us } => {
                if protocol != PROTOCOL_VERSION {
                    out.push((from, self.reject(RejectReason::ProtocolMismatch)));
                } else if !self.conns.contains_key(&from) && self.conns.len() >= self.cfg.max_players as usize {
                    out.push((from, self.reject(RejectReason::ServerFull)));
                } else {
                    if !self.conns.contains_key(&from) {
                        self.join(from, now);
                    }
                    let tick = self.world.tick();
                    let since = ((now - self.tick_time(tick)).max(0.0) * 1e6) as u32;
                    let team = self.world.unit(self.conns[&from].unit).map_or(Team::Blue, |u| u.team);
                    let conn = self.conns.get_mut(&from).unwrap();
                    conn.recv.record(header.seq);
                    conn.last_heard = now;
                    let msg = ServerMessage::Welcome {
                        player: conn.player,
                        unit: conn.unit,
                        team,
                        tick,
                        tick_hz: TICK_HZ as u8,
                        since_tick_us: since,
                        time_echo: TimeEcho { client_time_us, hold_us: 0 },
                    };
                    let h = Self::header(conn);
                    let bytes = msg::encode_server(&h, &msg);
                    self.stats.bytes_out += bytes.len() as u64;
                    out.push((from, bytes));
                }
            }
            ClientMessage::Input { client_time_us, commands } => {
                let world_tick = self.world.tick();
                let start = self.start;
                let Some(conn) = self.conns.get_mut(&from) else { return out };
                if !conn.recv.record(header.seq) {
                    return out;
                }
                conn.send.on_ack(header.ack, header.ack_bits);
                conn.last_heard = now;
                conn.echo = Some((client_time_us, now));
                let mut fresh: Vec<Command> = commands.into_iter().filter(|c| c.seq > conn.highest_seq).collect();
                fresh.sort_by_key(|c| c.seq);
                for mut c in fresh {
                    conn.highest_seq = c.seq;
                    c.player = conn.player;
                    self.stats.commands += 1;
                    let due = start + c.tick.0 as f64 * TICK_DT_F64;
                    let lead = due - now;
                    if c.tick <= world_tick {
                        // Already simulated: apply at the next tick, from its start (03a §10.2).
                        if -lead > MAX_LATENESS {
                            self.stats.commands_dropped += 1;
                            continue;
                        }
                        self.stats.commands_late += 1;
                        c.tick = world_tick.next();
                        c.sub = SubTick::START;
                    } else if c.tick.0 > world_tick.0 + MAX_LEAD_TICKS {
                        c.tick = Tick(world_tick.0 + MAX_LEAD_TICKS);
                    }
                    let report = CommandReport {
                        seq: c.seq,
                        applied_tick: c.tick,
                        applied_sub: c.sub,
                        lead_us: (lead * 1e6).clamp(i32::MIN as f64, i32::MAX as f64) as i32,
                    };
                    conn.reports.push_back((world_tick, report));
                    self.queue.push(c);
                }
            }
            ClientMessage::Bye => self.leave(from),
        }
        out
    }

    fn reject(&mut self, reason: RejectReason) -> Vec<u8> {
        let bytes = msg::encode_server(&PacketHeader::default(), &ServerMessage::Reject { reason });
        self.stats.bytes_out += bytes.len() as u64;
        bytes
    }

    fn join(&mut self, key: ClientKey, now: f64) {
        let used: Vec<u8> = self.conns.values().map(|c| c.player.0).collect();
        let player = PlayerId((0..=u8::MAX).find(|p| !used.contains(p)).unwrap());
        let team = if player.0 % 2 == 0 { Team::Blue } else { Team::Red };
        let (lo, hi) = (self.cfg.arena_min, self.cfg.arena_max);
        // A random spot with nothing within 150 u (deterministic: the world RNG).
        let mut pos = Vec2::ZERO;
        for _ in 0..64 {
            let rng = self.world.rng();
            pos = QPoint::from_vec2(Vec2::new(rng.range_f32(lo, hi), rng.range_f32(lo, hi))).to_vec2();
            if self.world.units().iter().all(|u| u.state.pos.distance(pos) > 150.0) {
                break;
            }
        }
        let unit = self.world.spawn_champion(player, team, pos);
        self.conns.insert(
            key,
            Conn {
                player,
                unit,
                recv: ReceiveTracker::default(),
                send: SendTracker::default(),
                highest_seq: 0,
                reports: VecDeque::new(),
                echo: None,
                last_heard: now,
            },
        );
    }

    fn leave(&mut self, key: ClientKey) {
        if let Some(c) = self.conns.remove(&key) {
            self.world.despawn(c.unit);
            self.queue.retain(|cmd| cmd.player != c.player);
        }
    }

    /// Simulate the next tick and build one snapshot per client.
    pub fn step(&mut self, now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        let timed_out: Vec<ClientKey> =
            self.conns.iter().filter(|(_, c)| now - c.last_heard > TIMEOUT).map(|(k, _)| *k).collect();
        for k in timed_out {
            self.leave(k);
        }

        let k = self.world.tick().next();
        let (due, later): (Vec<Command>, Vec<Command>) = self.queue.drain(..).partition(|c| c.tick <= k);
        self.queue = later;
        self.world.step(&due);
        self.stats.ticks += 1;

        let since = ((now - self.tick_time(k)).max(0.0) * 1e6) as u32;
        let units: Vec<(UnitId, mftr_sim::UnitState, RemoteUnit)> = self
            .world
            .units()
            .iter()
            .map(|u| {
                let remote = RemoteUnit {
                    id: u.id,
                    kind: u.kind,
                    team: u.team,
                    pos: QPoint::from_vec2(u.state.pos),
                    target: u.state.heading().map(QPoint::from_vec2),
                    speed: u.state.move_speed.round().clamp(0.0, 1023.0) as u16,
                    collision_radius: u.collision_radius.round().clamp(0.0, 255.0) as u8,
                };
                (u.id, u.state, remote)
            })
            .collect();
        let mut out = Vec::with_capacity(self.conns.len());
        for (key, conn) in self.conns.iter_mut() {
            while conn.reports.front().is_some_and(|(at, _)| k.0.saturating_sub(at.0) > REPORT_REPEAT_TICKS) {
                conn.reports.pop_front();
            }
            let own = units.iter().find(|(id, _, _)| *id == conn.unit).map(|(id, s, _)| (*id, *s));
            let others = units.iter().filter(|(id, _, _)| *id != conn.unit).map(|(_, _, r)| *r).collect();
            let reports: Vec<CommandReport> =
                conn.reports.iter().rev().take(msg::MAX_REPORTS_PER_SNAPSHOT).rev().map(|(_, r)| *r).collect();
            let snap = Snapshot {
                tick: k,
                since_tick_us: since,
                time_echo: conn
                    .echo
                    .take()
                    .map(|(t, at)| TimeEcho { client_time_us: t, hold_us: ((now - at).max(0.0) * 1e6) as u32 }),
                last_cmd_seq: conn.highest_seq,
                reports,
                own,
                others,
            };
            let h = Self::header(conn);
            let bytes = msg::encode_server(&h, &ServerMessage::Snapshot(snap));
            self.stats.bytes_out += bytes.len() as u64;
            out.push((*key, bytes));
        }
        out
    }
}
