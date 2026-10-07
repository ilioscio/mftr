//! `mftr-server`: the authoritative match server core.
//!
//! Transport- and clock-agnostic: feed it packets with timestamps, call [`ServerCore::step`]
//! when [`ServerCore::next_tick_due`] has passed, and send the packets it returns. The UDP
//! binary (`main.rs`) and the virtual-time Netcode Lab both drive this same core.

use mftr_net::PROTOCOL_VERSION;
use mftr_net::delta::{self, UnitUpdate};
use mftr_net::msg::{self, ClientMessage, CommandReport, RejectReason, RemoteUnit, ServerMessage, Snapshot, TimeEcho};
use mftr_net::packet::{PacketHeader, ReceiveTracker, SendTracker};
use mftr_sim::ability::LineSkillshot;
use mftr_sim::map::MapId;
use mftr_sim::vision::Vision;
use mftr_sim::{
    Area, ChampionId, Command, Missile, PlayerId, QPoint, SimEvent, SimTime, SubTick, TICK_DT_F64, TICK_HZ, Team, Tick,
    UnitId, Vec2, World,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub mod bots;
pub mod game;
pub mod lobby;
pub use bots::Bot;
pub use game::{Fog, Match, Replay, ReplayCheck, ReplayEntry};
use lobby::Lobby;

/// Opaque per-connection address key, assigned by the transport.
pub type ClientKey = u64;

/// Commands later than this are dropped instead of applied late (03a §10.2).
const MAX_LATENESS: f64 = 0.250;
/// Commands targeting further ahead than this are clamped (anti-abuse).
const MAX_LEAD_TICKS: u32 = 90;
const REPORT_REPEAT_TICKS: u32 = 15;
const TIMEOUT: f64 = 10.0;
/// A disconnected player's champion waits this long for a reconnect (M2 slice 5).
pub const RECONNECT_GRACE: f64 = 60.0;
/// Spectators a server accepts (they don't count toward the player limit).
pub const MAX_SPECTATORS: usize = 8;
/// Spectator connections use these ids (they have no unit and issue no commands).
const SPECTATOR: (PlayerId, UnitId) = (PlayerId(u8::MAX), UnitId(u32::MAX));
/// Snapshots remembered per client as possible delta baselines (~2 s).
const SENT_RING: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scenario {
    /// Champions only.
    Empty,
    /// M1 slice 2: turrets around the arena fire skillshots at the players (the dodge rig,
    /// 03 §14). All players are on the blue team.
    DodgeRig,
    /// M1 slice 1: static minion clumps plus two patrolling waves, to exercise minion block
    /// and client collision proxies (03a §5).
    MinionSandbox,
    /// M1 slice 4, the Duel Sandbox: blue spawns west, red east, with a few minion clumps in
    /// between to block skillshots. Minions and champions respawn.
    Duel,
    /// M2: an ARAM match on The Bridge: structures, minion waves, relics, a winner. A new
    /// match starts 10 s after a Base falls.
    Aram,
    /// M3: ARAM: Mayhem, ARAM with augment drafts (06 §3).
    Mayhem,
}

impl Scenario {
    /// The map each scenario is played on: the open plane for `empty`, the arena otherwise.
    pub fn map(self) -> MapId {
        match self {
            Scenario::Empty => MapId::Open,
            Scenario::MinionSandbox | Scenario::DodgeRig | Scenario::Duel => MapId::Arena,
            Scenario::Aram | Scenario::Mayhem => MapId::Bridge,
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "empty" => Some(Scenario::Empty),
            "minions" => Some(Scenario::MinionSandbox),
            "dodge" => Some(Scenario::DodgeRig),
            "duel" => Some(Scenario::Duel),
            "aram" => Some(Scenario::Aram),
            "mayhem" => Some(Scenario::Mayhem),
            _ => None,
        }
    }

    /// What the client is told it joined.
    pub fn mode(self) -> msg::GameMode {
        match self {
            Scenario::Empty => msg::GameMode::Empty,
            Scenario::MinionSandbox => msg::GameMode::Minions,
            Scenario::DodgeRig => msg::GameMode::Dodge,
            Scenario::Duel => msg::GameMode::Duel,
            Scenario::Aram => msg::GameMode::Aram,
            Scenario::Mayhem => msg::GameMode::Mayhem,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Scenario::Empty => "empty",
            Scenario::MinionSandbox => "minions",
            Scenario::DodgeRig => "dodge",
            Scenario::Duel => "duel",
            Scenario::Aram => "aram",
            Scenario::Mayhem => "mayhem",
        }
    }

    /// ARAM and its variants: The Bridge, a full match with champion select.
    pub fn is_aram(self) -> bool {
        matches!(self, Scenario::Aram | Scenario::Mayhem)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ServerConfig {
    pub seed: u64,
    pub max_players: u8,
    /// Server-side bots that join at start (M2 slice 5); they count toward `max_players`.
    pub bots: u8,
    /// Champion select before the match (ARAM all-random with rerolls); humans replace bots.
    pub lobby: bool,
    /// Record a replay (joins, leaves, commands, hashes) in memory, about 5 MB per hour of a
    /// busy 5v5 server. Off by default so a long-running server doesn't grow.
    pub record: bool,
    /// Spawn area: a square from `arena_min` to `arena_max` on both axes.
    pub arena_min: f32,
    pub arena_max: f32,
    pub scenario: Scenario,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            seed: 1,
            max_players: 10,
            bots: 0,
            lobby: false,
            record: false,
            arena_min: 500.0,
            arena_max: 3500.0,
            scenario: Scenario::Empty,
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
    /// Reliable events not yet acknowledged, with their sequence numbers (03b §7).
    events: VecDeque<(u32, SimEvent)>,
    next_event_seq: u32,
    team: Team,
    /// Missiles, areas and bolts this client has been told about (03 §10: enemy ones only
    /// once they enter vision), so it also gets their ends.
    revealed: BTreeSet<u32>,
    /// What this client reconstructed for other units at each recent tick (delta baselines,
    /// 03b §6), oldest first.
    sent: VecDeque<(Tick, BTreeMap<UnitId, RemoteUnit>)>,
    /// Newest snapshot tick the client reports having reconstructed.
    snapshot_ack: Tick,
    last_heard: f64,
    /// Proof of identity for a reconnect.
    token: u64,
    /// Watching: no unit, sees everything.
    spectator: bool,
}

impl Conn {
    fn new(player: PlayerId, unit: UnitId, team: Team, token: u64, spectator: bool, now: f64) -> Self {
        Conn {
            player,
            unit,
            recv: ReceiveTracker::default(),
            send: SendTracker::default(),
            highest_seq: 0,
            reports: VecDeque::new(),
            echo: None,
            events: VecDeque::new(),
            next_event_seq: 1,
            team,
            revealed: BTreeSet::new(),
            sent: VecDeque::new(),
            snapshot_ack: Tick(0),
            last_heard: now,
            token,
            spectator,
        }
    }
}

/// A connection in champion select: its player, when last heard, and its packet sequence
/// (carried into the connection when the match starts, so the client accepts the Welcome).
struct Pending {
    player: PlayerId,
    last_heard: f64,
    send: SendTracker,
}

/// A player whose connection dropped: the champion stays until `deadline`.
struct Dropped {
    token: u64,
    player: PlayerId,
    unit: UnitId,
    team: Team,
    deadline: f64,
}

pub struct ServerCore {
    cfg: ServerConfig,
    game: Match,
    bots: Vec<Bot>,
    /// Champion select, until the match starts.
    lobby: Option<Lobby>,
    /// Connections in champion select.
    pending: BTreeMap<ClientKey, Pending>,
    dropped: Vec<Dropped>,
    /// Session tokens (independent of the world RNG, so replays are unaffected).
    tokens: mftr_sim::rng::Pcg32,
    lobby_steps: u32,
    /// Matches finished: each champion select after one draws from a new seed.
    rounds: u64,
    start: f64,
    conns: BTreeMap<ClientKey, Conn>,
    queue: Vec<Command>,
    /// Live missiles and areas (for fog-of-war reveal), by id.
    missiles: BTreeMap<u32, Missile>,
    areas: BTreeMap<u32, Area>,
    pub stats: ServerStats,
}

impl ServerCore {
    /// `start`: the server-clock time (seconds) at which tick 0 ends.
    pub fn new(cfg: ServerConfig, start: f64) -> Self {
        let mut game = Match::new(cfg.clone());
        let mut bots = Vec::new();
        let lobby = cfg.lobby.then(|| Lobby::new(cfg.seed, cfg.max_players, cfg.bots));
        if !cfg.lobby {
            for _ in 0..cfg.bots.min(cfg.max_players) {
                let Some(player) = game.free_player() else { break };
                game.join(player, None);
                bots.push(Bot::new(player, cfg.seed));
            }
        }
        Self {
            game,
            bots,
            lobby,
            pending: BTreeMap::new(),
            dropped: Vec::new(),
            tokens: mftr_sim::rng::Pcg32::new(cfg.seed ^ 0x746f_6b65_6e73, 7),
            lobby_steps: 0,
            rounds: 0,
            cfg,
            start,
            conns: BTreeMap::new(),
            queue: Vec::new(),
            missiles: BTreeMap::new(),
            areas: BTreeMap::new(),
            stats: ServerStats::default(),
        }
    }

    pub fn world(&self) -> &World {
        &self.game.world
    }

    pub fn player_count(&self) -> usize {
        self.conns.len()
    }

    /// Server bots playing (champion-select bots count once the match starts).
    pub fn bot_count(&self) -> usize {
        self.bots.len()
    }

    /// The match driver (replay recording, joins).
    pub fn game(&self) -> &Match {
        &self.game
    }

    /// Wall time at which tick `k`'s interval ends, i.e. when it may be simulated.
    pub fn tick_time(&self, k: Tick) -> f64 {
        self.start + k.0 as f64 * TICK_DT_F64
    }

    pub fn next_tick_due(&self) -> f64 {
        self.tick_time(self.game.world.tick().next())
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
            ClientMessage::Hello { protocol, client_time_us, champion, resume, spectate } => {
                if protocol != PROTOCOL_VERSION {
                    out.push((from, self.reject(RejectReason::ProtocolMismatch)));
                    return out;
                }
                if let Some(p) = self.pending.get_mut(&from) {
                    p.last_heard = now;
                    let player = p.player;
                    out.extend(self.lobby_packet(from, player, now));
                    return out;
                }
                if !self.conns.contains_key(&from) {
                    let spectators = self.conns.values().filter(|c| c.spectator).count();
                    let live = self.conns.iter().find(|(_, c)| resume != 0 && c.token == resume).map(|(k, _)| *k);
                    if spectate {
                        if spectators >= MAX_SPECTATORS {
                            out.push((from, self.reject(RejectReason::ServerFull)));
                            return out;
                        }
                        let token = self.new_token();
                        self.conns.insert(from, Conn::new(SPECTATOR.0, SPECTATOR.1, Team::Blue, token, true, now));
                    } else if let Some(i) = self.dropped.iter().position(|d| resume != 0 && d.token == resume) {
                        // Reconnect: the champion is still there.
                        let d = self.dropped.remove(i);
                        self.conns.insert(from, Conn::new(d.player, d.unit, d.team, d.token, false, now));
                    } else if let Some(old) = live {
                        // The same player from a new address (a restarted client): take over.
                        let c = self.conns.remove(&old).unwrap();
                        self.conns.insert(from, Conn::new(c.player, c.unit, c.team, c.token, false, now));
                    } else if let Some(lobby) = &mut self.lobby {
                        let Some(player) = lobby.add_human(now) else {
                            out.push((from, self.reject(RejectReason::ServerFull)));
                            return out;
                        };
                        self.pending.insert(from, Pending { player, last_heard: now, send: SendTracker::default() });
                        out.extend(self.lobby_packet(from, player, now));
                        return out;
                    } else if self.game.player_count() >= self.cfg.max_players as usize {
                        out.push((from, self.reject(RejectReason::ServerFull)));
                        return out;
                    } else {
                        self.join(from, now, champion);
                    }
                }
                if let Some(conn) = self.conns.get_mut(&from) {
                    conn.recv.record(header.seq);
                    conn.last_heard = now;
                }
                out.extend(self.welcome(from, client_time_us, now));
            }
            ClientMessage::Lobby(action) => {
                if let Some(p) = self.pending.get_mut(&from)
                    && let Some(lobby) = &mut self.lobby
                {
                    p.last_heard = now;
                    let player = p.player;
                    lobby.act(player, action);
                    out.extend(self.lobby_packet(from, player, now));
                }
            }
            ClientMessage::Input { client_time_us, event_ack, snapshot_ack, commands } => {
                let world_tick = self.game.world.tick();
                let start = self.start;
                let Some(conn) = self.conns.get_mut(&from) else { return out };
                if !conn.recv.record(header.seq) {
                    return out;
                }
                conn.send.on_ack(header.ack, header.ack_bits);
                conn.last_heard = now;
                conn.echo = Some((client_time_us, now));
                while conn.events.front().is_some_and(|(seq, _)| *seq <= event_ack) {
                    conn.events.pop_front();
                }
                conn.snapshot_ack = conn.snapshot_ack.max(Tick(snapshot_ack));
                // Spectators watch; anything they send besides acks is ignored.
                let commands = if conn.spectator { Vec::new() } else { commands };
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
            ClientMessage::Bye => {
                if let Some(p) = self.pending.remove(&from)
                    && let Some(lobby) = &mut self.lobby
                {
                    lobby.remove(p.player);
                }
                self.leave(from);
            }
        }
        out
    }

    fn reject(&mut self, reason: RejectReason) -> Vec<u8> {
        let bytes = msg::encode_server(&PacketHeader::default(), &ServerMessage::Reject { reason });
        self.stats.bytes_out += bytes.len() as u64;
        bytes
    }

    fn join(&mut self, key: ClientKey, now: f64, champion: Option<ChampionId>) {
        let Some(player) = self.game.free_player() else { return };
        let (unit, team, _) = self.game.join(player, champion);
        let token = self.new_token();
        self.conns.insert(key, Conn::new(player, unit, team, token, false, now));
    }

    /// A fresh, nonzero session token.
    fn new_token(&mut self) -> u64 {
        loop {
            let t = ((self.tokens.next_u32() as u64) << 32) | self.tokens.next_u32() as u64;
            if t != 0 {
                return t;
            }
        }
    }

    /// The Welcome for connection `key` (player, spectator or reconnect).
    fn welcome(&mut self, key: ClientKey, client_time_us: u32, now: f64) -> Option<(ClientKey, Vec<u8>)> {
        let tick = self.game.world.tick();
        let since = ((now - self.tick_time(tick)).max(0.0) * 1e6) as u32;
        let conn = self.conns.get(&key)?;
        let me = self.game.world.unit(conn.unit);
        let team = me.map_or(Team::Blue, |u| u.team);
        let champion = me.and_then(|u| u.champion).unwrap_or(ChampionId::Ember);
        let home = me.map_or(Vec2::ZERO, |u| u.home);
        let msg = ServerMessage::Welcome {
            player: conn.player,
            unit: conn.unit,
            team,
            map: self.game.world.map().id,
            champion,
            home,
            rules: self.game.world.rules(),
            tick,
            tick_hz: TICK_HZ as u8,
            since_tick_us: since,
            time_echo: TimeEcho { client_time_us, hold_us: 0 },
            token: conn.token,
            spectator: conn.spectator,
            mode: self.cfg.scenario.mode(),
        };
        let conn = self.conns.get_mut(&key)?;
        let h = Self::header(conn);
        let bytes = msg::encode_server(&h, &msg);
        self.stats.bytes_out += bytes.len() as u64;
        Some((key, bytes))
    }

    /// Champion select as `player` sees it.
    fn lobby_packet(&mut self, key: ClientKey, player: PlayerId, now: f64) -> Option<(ClientKey, Vec<u8>)> {
        let lobby = self.lobby.as_mut()?;
        let starts_in = lobby.starts_in(now).unwrap_or(lobby::LOBBY_SECONDS);
        let state = lobby.state_for(player, starts_in);
        let seq = self.pending.get_mut(&key)?.send.next_seq();
        let h = PacketHeader { seq, ack: 0, ack_bits: 0 };
        let bytes = msg::encode_server(&h, &ServerMessage::Lobby(Box::new(state)));
        self.stats.bytes_out += bytes.len() as u64;
        Some((key, bytes))
    }

    /// Champion select is over: everyone joins the match in slot order; bots get brains, and
    /// humans their connection and a Welcome.
    fn start_from_lobby(&mut self, now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        let Some(lobby) = self.lobby.take() else { return Vec::new() };
        let mut out = Vec::new();
        for slot in lobby.slots() {
            let (unit, team, _) = self.game.join(slot.player, Some(slot.champion));
            if slot.bot {
                self.bots.push(Bot::new(slot.player, self.cfg.seed));
                continue;
            }
            let Some(key) = self.pending.iter().find(|(_, p)| p.player == slot.player).map(|(k, _)| *k) else {
                continue;
            };
            let token = self.new_token();
            let mut conn = Conn::new(slot.player, unit, team, token, false, now);
            conn.send = self.pending.remove(&key).map(|p| p.send).unwrap_or_default();
            self.conns.insert(key, conn);
            out.extend(self.welcome(key, 0, now));
        }
        self.pending.clear();
        out
    }

    /// A Base fell: every connected player goes back to champion select (new random
    /// champions, rerolls and bench), bots refill the free slots, and the world restarts empty
    /// until the next match starts. Spectators stay connected and see the next match.
    fn return_to_lobby(&mut self, now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        self.rounds += 1;
        let seed = self.cfg.seed.wrapping_add(self.rounds);
        let mut lobby = Lobby::new(seed, self.cfg.max_players, self.cfg.bots);
        let players: Vec<ClientKey> = self.conns.iter().filter(|(_, c)| !c.spectator).map(|(k, _)| *k).collect();
        for key in players {
            let c = self.conns.remove(&key).expect("listed");
            if let Some(player) = lobby.add_human(now) {
                // The packet sequence carries over, so the client accepts what follows.
                self.pending.insert(key, Pending { player, last_heard: c.last_heard, send: c.send });
            }
        }
        for player in self.game.players() {
            self.game.leave(player);
        }
        self.bots.clear();
        self.dropped.clear();
        self.queue.clear();
        self.missiles.clear();
        self.areas.clear();
        self.game.restart();
        self.lobby = Some(lobby);
        self.lobby_steps = 0;
        let pending: Vec<(ClientKey, PlayerId)> = self.pending.iter().map(|(k, p)| (*k, p.player)).collect();
        pending.into_iter().filter_map(|(key, player)| self.lobby_packet(key, player, now)).collect()
    }

    fn leave(&mut self, key: ClientKey) {
        if let Some(c) = self.conns.remove(&key)
            && !c.spectator
        {
            self.game.leave(c.player);
            self.queue.retain(|cmd| cmd.player != c.player);
        }
    }

    /// Simulate the next tick and build one snapshot per client.
    pub fn step(&mut self, now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        // Silent connections: spectators go, players' champions wait for a reconnect.
        let timed_out: Vec<ClientKey> =
            self.conns.iter().filter(|(_, c)| now - c.last_heard > TIMEOUT).map(|(k, _)| *k).collect();
        for key in timed_out {
            let c = self.conns.remove(&key).unwrap();
            if !c.spectator {
                self.queue.retain(|cmd| cmd.player != c.player);
                let deadline = now + RECONNECT_GRACE;
                self.dropped.push(Dropped { token: c.token, player: c.player, unit: c.unit, team: c.team, deadline });
            }
        }
        let (gone, kept): (Vec<Dropped>, Vec<Dropped>) = self.dropped.drain(..).partition(|d| now > d.deadline);
        self.dropped = kept;
        for d in gone {
            self.game.leave(d.player);
        }
        // After a match, servers with champion select hold another one for everyone still here.
        if self.cfg.lobby && self.lobby.is_none() && self.game.restart_due() {
            return self.return_to_lobby(now);
        }
        // Champion select: no simulation (time is held), state to everyone a few times a second.
        if self.lobby.is_some() {
            let silent: Vec<(ClientKey, PlayerId)> = self
                .pending
                .iter()
                .filter(|(_, p)| now - p.last_heard > TIMEOUT)
                .map(|(k, p)| (*k, p.player))
                .collect();
            for (key, player) in silent {
                self.pending.remove(&key);
                if let Some(lobby) = &mut self.lobby {
                    lobby.remove(player);
                }
            }
            self.start += TICK_DT_F64;
            let starts_in = self.lobby.as_mut().and_then(|l| l.starts_in(now));
            if starts_in == Some(0.0) {
                return self.start_from_lobby(now);
            }
            self.lobby_steps += 1;
            if !self.lobby_steps.is_multiple_of(10) {
                return Vec::new();
            }
            let pending: Vec<(ClientKey, PlayerId)> = self.pending.iter().map(|(k, p)| (*k, p.player)).collect();
            return pending.into_iter().filter_map(|(key, player)| self.lobby_packet(key, player, now)).collect();
        }

        let k = self.game.world.tick().next();
        let (mut due, later): (Vec<Command>, Vec<Command>) = self.queue.drain(..).partition(|c| c.tick <= k);
        self.queue = later;
        for bot in &mut self.bots {
            due.extend(bot.think(&self.game.world, k));
        }
        let (events, fog) = self.game.step(due);
        self.stats.ticks += 1;

        let since = ((now - self.tick_time(k)).max(0.0) * 1e6) as u32;
        let (s0, s1) = (SimTime::end_of(Tick(k.0 - 1)), SimTime::end_of(k));
        let units: Vec<(UnitId, mftr_sim::UnitState, RemoteUnit)> = self
            .game
            .world
            .units()
            .iter()
            .map(|u| {
                let st = &u.state;
                let hp = |v: f32| v.round().clamp(0.0, u16::MAX as f32) as u16;
                let remote = RemoteUnit {
                    id: u.id,
                    kind: u.kind,
                    team: u.team,
                    pos: QPoint::from_vec2(st.pos),
                    target: st.heading().map(QPoint::from_vec2),
                    speed: st.speed_at(s1).round().clamp(0.0, 1023.0) as u16,
                    collision_radius: u.collision_radius.round().clamp(0.0, 255.0) as u8,
                    gameplay_radius: u.gameplay_radius.round().clamp(0.0, 255.0) as u8,
                    protected: u.protected,
                    champion: u.champion,
                    health: hp(st.health.max(if st.alive() { 1.0 } else { 0.0 })),
                    max_health: hp(u.stats.max_health),
                    shield: hp(if st.shield_until > s1 { st.shield } else { 0.0 }),
                    level: if u.champion.is_some() { st.progress.level } else { 0 },
                    casting: st.cast.is_some(),
                    attacking: st.attack.is_some(),
                    stunned: st.stunned_until > s1,
                    rooted: st.rooted_until > s1,
                    dashing: st.dash.is_some(),
                    slowed: st.slow > 0 && st.slowed_until > s1,
                };
                (u.id, u.state, remote)
            })
            .collect();
        // Fog of war (03 §10): what each team sees this tick.
        let map = self.game.world.map().clone();
        let Fog { visions, seen } = fog;
        // View 2 is a spectator's: everything either team sees, every event.
        let seen_all: BTreeSet<UnitId> = seen[0].union(&seen[1]).copied().collect();
        let sees = |view: usize, p: Vec2| view == 2 || visions[view].sees(&map, p);
        // Whether team `i` may hear about something happening to `unit` at its position this
        // tick (also for units that just died, which vision no longer lists).
        let knows = |i: usize, unit: UnitId| {
            self.game.world.unit(unit).is_some_and(|u| {
                u.team == [Team::Blue, Team::Red][i]
                    || u.kind == mftr_sim::UnitKind::RigTurret
                    || visions[i].sees(&map, u.state.pos)
            })
        };
        let mut ended: BTreeMap<u32, SimTime> = BTreeMap::new();
        for e in &events {
            match *e {
                SimEvent::MissileSpawned(m) => {
                    self.missiles.insert(m.id, m);
                }
                SimEvent::AreaSpawned(a) => {
                    self.areas.insert(a.id, a);
                }
                SimEvent::MissileHit { id, at, .. }
                | SimEvent::MissileExpired { id, at }
                | SimEvent::AreaDetonated { id, at } => {
                    ended.insert(id, at);
                }
                _ => {}
            }
        }
        // Per team: the events it may receive, in order (the per-client parts follow).
        let mut team_events: [Vec<SimEvent>; 3] = [Vec::new(), Vec::new(), Vec::new()];
        for (i, list) in team_events.iter_mut().enumerate() {
            let seen_i = if i == 2 { &seen_all } else { &seen[i] };
            let knows = |i: usize, unit: UnitId| i == 2 || knows(i, unit);
            for e in &events {
                let ok = match *e {
                    SimEvent::CastStarted { unit, .. }
                    | SimEvent::AttackLaunched(mftr_sim::Bolt { owner: unit, .. }) => seen_i.contains(&unit),
                    SimEvent::Damage { target: unit, .. }
                    | SimEvent::Died { unit, .. }
                    | SimEvent::Respawned { unit, .. }
                    | SimEvent::Dashed { unit, .. }
                    | SimEvent::Shielded { unit, .. }
                    | SimEvent::Healed { unit, .. }
                    | SimEvent::Blinked { unit, .. } => knows(i, unit),
                    SimEvent::MatchEnded { .. } => true,
                    _ => false,
                };
                if !ok {
                    continue;
                }
                // A blink or dash seen only at its end doesn't reveal where it started.
                list.push(match *e {
                    SimEvent::Blinked { unit, from, to, at } if !sees(i, from) => {
                        SimEvent::Blinked { unit, from: to, to, at }
                    }
                    SimEvent::Dashed { unit, from, to, at, end_at } if !sees(i, from) => {
                        let start = self.game.world.unit(unit).map_or(to, |u| u.state.pos);
                        SimEvent::Dashed { unit, from: start, to, at, end_at }
                    }
                    other => other,
                });
            }
        }

        let mut out = Vec::with_capacity(self.conns.len());
        for (key, conn) in self.conns.iter_mut() {
            while conn.reports.front().is_some_and(|(at, _)| k.0.saturating_sub(at.0) > REPORT_REPEAT_TICKS) {
                conn.reports.pop_front();
            }
            let own = units.iter().find(|(id, _, _)| *id == conn.unit).map(|(id, s, _)| (*id, *s));
            let ti = if conn.spectator { 2 } else { (conn.team == Team::Red) as usize };
            let seen_ti = if ti == 2 { &seen_all } else { &seen[ti] };
            // Missiles and areas a client gets at spawn: its team's, or all for a spectator.
            let (spectator, own_team) = (conn.spectator, conn.team);
            let at_spawn = move |team: Team| spectator || team == own_team;
            let push = |conn: &mut Conn, e: SimEvent| {
                conn.events.push_back((conn.next_event_seq, e));
                conn.next_event_seq += 1;
            };
            for e in &team_events[ti] {
                if let SimEvent::AttackLaunched(b) = e {
                    conn.revealed.insert(b.id);
                }
                push(conn, *e);
            }
            // Own-team missiles and areas: at spawn, unmodified. Rewards: only to the earner.
            for e in &events {
                match *e {
                    SimEvent::Reward { unit, .. } if unit == conn.unit => push(conn, *e),
                    SimEvent::MissileSpawned(m) if at_spawn(m.team) => {
                        conn.revealed.insert(m.id);
                        push(conn, *e);
                    }
                    SimEvent::AreaSpawned(a) if at_spawn(a.team) => {
                        conn.revealed.insert(a.id);
                        push(conn, *e);
                    }
                    _ => {}
                }
            }
            // Enemy missiles: revealed when they enter vision, re-based to that point so the
            // caster's position isn't leaked (03 §10).
            for m in self.missiles.values() {
                if at_spawn(m.team) || conn.revealed.contains(&m.id) {
                    continue;
                }
                let a = m.spawn_at.max(s0);
                let b = ended.get(&m.id).copied().unwrap_or(m.end_at()).min(s1);
                let at = [a, b].into_iter().find(|&t| sees(ti, m.position_at(t)));
                if let Some(t) = at {
                    conn.revealed.insert(m.id);
                    push(conn, SimEvent::MissileSpawned(rebase(m, t)));
                }
            }
            // Enemy areas: the telegraph shows once its center is in vision.
            for a in self.areas.values() {
                if !at_spawn(a.team) && !conn.revealed.contains(&a.id) && sees(ti, a.center) {
                    conn.revealed.insert(a.id);
                    push(conn, SimEvent::AreaSpawned(*a));
                }
            }
            // Ends: only for things this client knows about.
            for e in &events {
                if let SimEvent::MissileHit { id, .. }
                | SimEvent::MissileExpired { id, .. }
                | SimEvent::AreaDetonated { id, .. }
                | SimEvent::AttackLanded { id, .. } = *e
                    && conn.revealed.remove(&id)
                {
                    push(conn, *e);
                }
            }
            // Other units as deltas against what the client last reconstructed (03b §6).
            while conn.sent.front().is_some_and(|(t, _)| *t < conn.snapshot_ack) || conn.sent.len() > SENT_RING {
                conn.sent.pop_front();
            }
            let baseline = conn.sent.front().filter(|(t, _)| *t == conn.snapshot_ack).cloned();
            let base_map = baseline.as_ref().map(|(_, m)| m.clone()).unwrap_or_default();
            let ticks = baseline.as_ref().map_or(0, |(t, _)| k.0 - t.0);
            let own_pos = own.map_or(Vec2::ZERO, |(_, st)| st.pos);
            let mut visible: Vec<&RemoteUnit> =
                units.iter().filter(|(id, _, _)| *id != conn.unit && seen_ti.contains(id)).map(|(_, _, r)| r).collect();
            // Most important first (champions, then nearest), so a size cut drops the least useful.
            visible.sort_by(|a, b| {
                let key = |r: &RemoteUnit| (r.kind != mftr_sim::UnitKind::Champion, r.pos.to_vec2().distance(own_pos));
                key(a).partial_cmp(&key(b)).unwrap()
            });
            let mut updates: Vec<UnitUpdate> = Vec::new();
            let mut records: BTreeMap<UnitId, RemoteUnit> = BTreeMap::new();
            for r in visible {
                let (u, rec) = delta::diff(base_map.get(&r.id), r, ticks);
                updates.extend(u);
                records.insert(r.id, rec);
            }
            let removed: Vec<UnitId> = base_map.keys().filter(|id| !records.contains_key(id)).copied().collect();
            let reports: Vec<CommandReport> =
                conn.reports.iter().rev().take(msg::MAX_REPORTS_PER_SNAPSHOT).rev().map(|(_, r)| *r).collect();
            let mut snap = Snapshot {
                tick: k,
                since_tick_us: since,
                time_echo: conn
                    .echo
                    .take()
                    .map(|(t, at)| TimeEcho { client_time_us: t, hold_us: ((now - at).max(0.0) * 1e6) as u32 }),
                last_cmd_seq: conn.highest_seq,
                reports,
                own,
                events: conn.events.iter().take(msg::MAX_EVENTS_PER_SNAPSHOT).copied().collect(),
                baseline: baseline.as_ref().map(|(t, _)| *t),
                others: updates,
                removed,
            };
            let h = Self::header(conn);
            // Stay under the packet limit (03b §1): the least important unit updates wait for a
            // later snapshot (those units coast, or appear later), then a backlog of reliable
            // events does.
            let mut bytes = msg::encode_server(&h, &ServerMessage::Snapshot(Box::new(snap.clone())));
            while bytes.len() > mftr_net::MAX_PAYLOAD_BYTES && !snap.others.is_empty() {
                let keep = snap.others.len() * 3 / 4;
                for dropped in snap.others.drain(keep..) {
                    let id = dropped.unit.id;
                    match delta::apply(base_map.get(&id), None, ticks) {
                        Some(coasted) => records.insert(id, coasted),
                        None => records.remove(&id),
                    };
                }
                bytes = msg::encode_server(&h, &ServerMessage::Snapshot(Box::new(snap.clone())));
            }
            while bytes.len() > mftr_net::MAX_PAYLOAD_BYTES && !snap.events.is_empty() {
                snap.events.truncate(snap.events.len() / 2);
                bytes = msg::encode_server(&h, &ServerMessage::Snapshot(Box::new(snap.clone())));
            }
            conn.sent.push_back((k, records));
            self.stats.bytes_out += bytes.len() as u64;
            out.push((*key, bytes));
        }
        self.missiles.retain(|id, _| !ended.contains_key(id));
        self.areas.retain(|id, _| !ended.contains_key(id));
        out
    }

    /// Units the client was told about in its latest snapshot (fog audit, tests): what it
    /// reconstructs, deltas and coasting included.
    pub fn last_sent(&self, key: ClientKey) -> Vec<UnitId> {
        self.conns.get(&key).and_then(|c| c.sent.back()).map(|(_, m)| m.keys().copied().collect()).unwrap_or_default()
    }

    /// What the client was sent to reconstruct for other units at `tick`, if still remembered
    /// (delta consistency tests).
    pub fn sent_records(&self, key: ClientKey, tick: Tick) -> Option<BTreeMap<UnitId, RemoteUnit>> {
        self.conns.get(&key)?.sent.iter().find(|(t, _)| *t == tick).map(|(_, m)| m.clone())
    }

    /// The team a connection plays on (tests, the Netcode Lab's fog audit).
    pub fn team_of(&self, key: ClientKey) -> Option<Team> {
        self.conns.get(&key).map(|c| c.team)
    }

    /// Direct world access for tests and scripted scenarios.
    pub fn world_mut(&mut self) -> &mut World {
        &mut self.game.world
    }

    /// Unit ids a client on `team` may currently know about (tests, debugging).
    pub fn visible_to(&self, team: Team) -> BTreeSet<UnitId> {
        let map = self.game.world.map().clone();
        let v = Vision::of(&self.game.world, team);
        self.game.world.units().iter().filter(|u| v.sees_unit(&map, u)).map(|u| u.id).collect()
    }
}

/// The same missile, as first seen at `t`: identical path from there on, but starting at its
/// position at `t`, so where it was fired from stays hidden.
/// The outcome of [`run_bot_match`].
#[derive(Clone, Debug)]
pub struct BotMatch {
    /// The winner and the tick the match ended (None: still going at the limit).
    pub winner: Option<(Team, Tick)>,
    pub ticks: u32,
    pub champion_kills: u32,
    pub structures_destroyed: u32,
    /// Structures that fell: when, whose, what, tier.
    pub falls: Vec<(Tick, Team, mftr_sim::UnitKind, u8)>,
    pub replay: Replay,
}

/// Play a match with server bots only, as fast as possible (no network, no clients), until a
/// Base falls or `max_ticks` pass.
pub fn run_bot_match(cfg: ServerConfig, max_ticks: u32) -> BotMatch {
    let mut core = ServerCore::new(ServerConfig { record: true, ..cfg }, 0.0);
    let (mut kills, mut structures) = (0, 0);
    let mut falls = Vec::new();
    for _ in 0..max_ticks {
        let k = core.game.world.tick().next();
        let mut due = Vec::new();
        for bot in &mut core.bots {
            due.extend(bot.think(&core.game.world, k));
        }
        let (events, _) = core.game.step(due);
        for e in &events {
            if let SimEvent::Died { unit, .. } = e
                && let Some(u) = core.game.world.unit(*unit)
            {
                match u.kind {
                    mftr_sim::UnitKind::Champion => kills += 1,
                    kind if kind.is_structure() => {
                        structures += 1;
                        falls.push((k, u.team, kind, u.tier));
                    }
                    _ => {}
                }
            }
        }
        if let Some((team, at)) = core.game.world.game().winner {
            let tick = Tick((at.0 / mftr_sim::SUBTICKS as u64) as u32);
            return BotMatch {
                winner: Some((team, tick)),
                ticks: core.game.world.tick().0,
                champion_kills: kills,
                structures_destroyed: structures,
                falls,
                replay: core.game.replay(),
            };
        }
    }
    BotMatch {
        winner: None,
        ticks: core.game.world.tick().0,
        champion_kills: kills,
        structures_destroyed: structures,
        falls,
        replay: core.game.replay(),
    }
}

fn rebase(m: &Missile, t: SimTime) -> Missile {
    if t <= m.spawn_at {
        return *m;
    }
    let traveled = m.spec.speed * t.secs_since(m.spawn_at);
    Missile {
        origin: m.position_at(t),
        spawn_at: t,
        spec: LineSkillshot { range: (m.spec.range - traveled).max(0.0), ..m.spec },
        ..*m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_net::msg::{decode_server, encode_client};

    fn hello(core: &mut ServerCore, key: ClientKey) {
        let bytes = encode_client(
            &PacketHeader::default(),
            &ClientMessage::Hello {
                protocol: PROTOCOL_VERSION,
                client_time_us: 0,
                champion: None,
                resume: 0,
                spectate: false,
            },
        );
        core.handle_packet(key, &bytes, 0.0);
    }

    fn others_seen_by(core: &mut ServerCore, key: ClientKey, now: f64) -> Vec<UnitId> {
        let mut seen = Vec::new();
        for (to, bytes) in core.step(now) {
            if to == key
                && let Ok((_, ServerMessage::Snapshot(s))) = decode_server(&bytes)
            {
                seen = s.others.iter().map(|o| o.unit.id).collect();
            }
        }
        seen
    }

    fn send(core: &mut ServerCore, key: ClientKey, msg: ClientMessage, now: f64) -> Vec<ServerMessage> {
        let bytes = encode_client(&PacketHeader::default(), &msg);
        core.handle_packet(key, &bytes, now)
            .into_iter()
            .filter(|(to, _)| *to == key)
            .map(|(_, b)| decode_server(&b).unwrap().1)
            .collect()
    }

    fn hello_msg(resume: u64, spectate: bool) -> ClientMessage {
        ClientMessage::Hello { protocol: PROTOCOL_VERSION, client_time_us: 0, champion: None, resume, spectate }
    }

    fn welcome_of(msgs: &[ServerMessage]) -> Option<(UnitId, u64, bool)> {
        msgs.iter().find_map(|m| match m {
            ServerMessage::Welcome { unit, token, spectator, .. } => Some((*unit, *token, *spectator)),
            _ => None,
        })
    }

    /// M2 slice 5: champion select. A human takes a bot's slot, rerolls (the old champion goes
    /// to the bench), readies up, and the match starts 3 s later with everyone in it.
    #[test]
    fn champion_select_rerolls_and_starts_the_match() {
        let cfg = ServerConfig { seed: 4, bots: 10, lobby: true, scenario: Scenario::Aram, ..Default::default() };
        let mut core = ServerCore::new(cfg, 0.0);
        let lobby = |msgs: &[ServerMessage]| {
            msgs.iter()
                .find_map(|m| match m {
                    ServerMessage::Lobby(l) => Some((**l).clone()),
                    _ => None,
                })
                .expect("a lobby state")
        };
        let l = lobby(&send(&mut core, 1, hello_msg(0, false), 0.0));
        assert_eq!(l.slots.len(), 10);
        assert_eq!(l.slots.iter().filter(|s| s.bot).count(), 9);
        let me = *l.slots.iter().find(|s| s.player == l.you).unwrap();
        assert!(!me.bot && me.rerolls == lobby::REROLLS);
        let l = lobby(&send(&mut core, 1, ClientMessage::Lobby(msg::LobbyAction::Reroll), 1.0));
        let after = *l.slots.iter().find(|s| s.player == l.you).unwrap();
        assert_ne!(after.champion, me.champion);
        assert!(l.bench.contains(&me.champion));
        let l = lobby(&send(&mut core, 1, ClientMessage::Lobby(msg::LobbyAction::Ready(true)), 2.0));
        assert!(l.starts_in_ms <= 3000);
        // The world holds still in champion select, then the match starts with a Welcome.
        let mut welcome = None;
        let mut t = 2.0;
        while welcome.is_none() && t < 10.0 {
            t += 1.0 / 30.0;
            assert_eq!(core.world().tick(), Tick(0));
            for (to, b) in core.step(t) {
                if to == 1
                    && let Ok((_, ServerMessage::Welcome { unit, .. })) = decode_server(&b)
                {
                    welcome = Some(unit);
                }
            }
        }
        let unit = welcome.expect("the match starts");
        assert!((4.9..5.2).contains(&t), "3 s after everyone is ready: {t}");
        assert_eq!(core.world().unit(unit).unwrap().champion, Some(after.champion));
        assert_eq!(core.game().player_count(), 10);
        core.step(t + 1.0 / 30.0);
        assert_eq!(core.world().tick(), Tick(1), "now the clock runs");
    }

    /// A dropped player's champion waits; the token brings the player back to it, even from
    /// a new address. After the grace period the champion is gone.
    #[test]
    fn reconnecting_with_the_token_takes_the_champion_back() {
        let mut core = ServerCore::new(ServerConfig { scenario: Scenario::Duel, ..Default::default() }, 0.0);
        let (unit, token, spectator) = welcome_of(&send(&mut core, 1, hello_msg(0, false), 0.0)).unwrap();
        assert!(token != 0 && !spectator);
        let (other, other_token, _) = welcome_of(&send(&mut core, 2, hello_msg(0, false), 0.0)).unwrap();
        let mut t = 0.0;
        while t < 12.0 {
            t += 1.0 / 30.0;
            core.step(t);
        }
        assert_eq!(core.player_count(), 0, "both timed out");
        assert!(core.world().unit(unit).is_some(), "the champion waits");
        // A wrong token is a new player; the right one is the old one, from a new address.
        let (fresh, _, _) = welcome_of(&send(&mut core, 3, hello_msg(12345, false), t)).unwrap();
        assert!(fresh != unit && fresh != other);
        let (back, same, _) = welcome_of(&send(&mut core, 4, hello_msg(token, false), t)).unwrap();
        assert_eq!((back, same), (unit, token));
        while t < 12.0 + RECONNECT_GRACE + 1.0 {
            t += 1.0 / 30.0;
            for key in [3, 4] {
                send(&mut core, key, hello_msg(0, false), t);
            }
            core.step(t);
        }
        assert!(core.world().unit(other).is_none(), "the other player never came back");
        assert!(core.world().unit(unit).is_some());
        assert!(welcome_of(&send(&mut core, 5, hello_msg(other_token, false), t)).is_some_and(|w| w.0 != other));
    }

    /// Spectators see every unit (no fog), have no unit and don't take a player slot.
    #[test]
    fn spectators_see_everything() {
        let mut core = ServerCore::new(ServerConfig { scenario: Scenario::Duel, ..Default::default() }, 0.0);
        hello(&mut core, 1);
        hello(&mut core, 2);
        let (unit, _, spectator) = welcome_of(&send(&mut core, 9, hello_msg(0, true), 0.0)).unwrap();
        assert!(spectator);
        assert_eq!(core.game().player_count(), 2);
        let mut snap = None;
        for (to, b) in core.step(1.0 / 30.0) {
            if to == 9
                && let Ok((_, ServerMessage::Snapshot(s))) = decode_server(&b)
            {
                snap = Some(s);
            }
        }
        let s = snap.unwrap();
        assert!(s.own.is_none() && core.world().unit(unit).is_none());
        let champions = s.others.iter().filter(|o| o.unit.kind == mftr_sim::UnitKind::Champion).count();
        assert_eq!(champions, 2, "both duelists, across the arena from each other");
        assert_eq!(s.others.len(), core.world().units().len(), "every unit");
        // Commands from a spectator do nothing.
        let input = ClientMessage::Input {
            client_time_us: 0,
            event_ack: 0,
            snapshot_ack: 0,
            commands: vec![Command {
                player: PlayerId(0),
                seq: 1,
                tick: Tick(3),
                sub: SubTick::START,
                kind: mftr_sim::CommandKind::Stop,
            }],
        };
        send(&mut core, 9, input, 0.05);
        assert!(core.queue.is_empty());
    }

    /// Slice 3 exit: hidden units never reach a client. Behind a wall, in a bush, or out of
    /// range: absent from the snapshot. Visible: present.
    #[test]
    fn fog_of_war_culls_hidden_units() {
        let mut core = ServerCore::new(ServerConfig { scenario: Scenario::MinionSandbox, ..Default::default() }, 0.0);
        hello(&mut core, 1); // player 0, blue
        hello(&mut core, 2); // player 1, red
        let (me, enemy) = (core.conns[&1].unit, core.conns[&2].unit);
        // Only the two champions provide vision in this test.
        let minions: Vec<UnitId> = core.game.world.units().iter().filter(|u| u.owner.is_none()).map(|u| u.id).collect();
        for id in minions {
            core.world_mut().despawn(id);
        }
        let place = |core: &mut ServerCore, id: UnitId, x: f32, y: f32| {
            let u = core.world_mut().unit_mut(id).unwrap();
            u.state =
                mftr_sim::UnitState { health: u.stats.max_health, ..mftr_sim::UnitState::new(Vec2::new(x, y), 325.0) };
        };
        place(&mut core, me, 2200.0, 1200.0);
        let mut t = 0.0;
        let mut check = |core: &mut ServerCore, ex: f32, ey: f32, visible: bool| {
            place(core, enemy, ex, ey);
            t += 1.0 / 30.0;
            let seen = others_seen_by(core, 1, t);
            assert_eq!(seen.contains(&enemy), visible, "enemy at ({ex}, {ey}) visible={visible}");
            // Nothing sent may be hidden from the team, ever.
            let allowed = core.visible_to(Team::Blue);
            assert!(seen.iter().all(|id| allowed.contains(id)));
        };
        check(&mut core, 2700.0, 1200.0, false); // behind the long wall
        check(&mut core, 2200.0, 1900.0, true); // in plain view
        check(&mut core, 2775.0, 1765.0, false); // inside brush 1, observer outside
        check(&mut core, 3900.0, 100.0, false); // out of range
    }

    fn hello_as(core: &mut ServerCore, key: ClientKey, champion: ChampionId) {
        let bytes = encode_client(
            &PacketHeader::default(),
            &ClientMessage::Hello {
                protocol: PROTOCOL_VERSION,
                client_time_us: 0,
                champion: Some(champion),
                resume: 0,
                spectate: false,
            },
        );
        core.handle_packet(key, &bytes, 0.0);
    }

    /// Duel: players get alternating champions at their team's spawn by default.
    /// M2 slice 5 exit: ten server bots finish an ARAM match, and its replay (through the text
    /// format) re-simulates to the same hashes.
    #[test]
    fn ten_bots_finish_a_match_and_its_replay_matches() {
        let cfg = ServerConfig { seed: 6, bots: 10, scenario: Scenario::Aram, ..Default::default() };
        let m = run_bot_match(cfg, 60 * 60 * 30);
        let (_, at) = m.winner.expect("a Base falls within an hour");
        assert!(at.0 > 5 * 60 * 30, "not before five minutes: {}", at.0);
        assert!(
            m.champion_kills > 10 && m.structures_destroyed >= 7,
            "{} kills, {} structures",
            m.champion_kills,
            m.structures_destroyed
        );
        assert_eq!(m.replay.entries.iter().filter(|e| matches!(e, ReplayEntry::Join { .. })).count(), 10);
        let text = m.replay.to_text();
        let parsed = Replay::from_text(&text).unwrap();
        assert_eq!(parsed, m.replay);
        let check = parsed.verify();
        assert!(check.hashes_checked >= 80, "{check:?}");
        assert_eq!(check.mismatch, None);
        // A tampered command changes the outcome, and the check notices.
        let mut tampered = parsed.clone();
        let cmds = tampered.entries.iter_mut().find_map(|e| match e {
            ReplayEntry::Commands { commands, tick } if tick.0 > 3000 => Some(commands),
            _ => None,
        });
        cmds.unwrap()[0].kind = mftr_sim::CommandKind::MoveTo(QPoint::from_vec2(Vec2::new(6000.0, 300.0)));
        assert!(tampered.verify().mismatch.is_some());
    }

    /// Joins and leaves at any point (before, between and after commands) replay exactly.
    #[test]
    fn replays_handle_joins_and_leaves() {
        let mut m = Match::new(ServerConfig { scenario: Scenario::Duel, record: true, ..Default::default() });
        let mut bots: Vec<Bot> = Vec::new();
        for k in 1..=900u32 {
            if k == 1 || k == 200 || k == 450 {
                let p = m.free_player().unwrap();
                m.join(p, None);
                bots.push(Bot::new(p, 7));
            }
            if k == 600 {
                let gone = bots.remove(0).player;
                m.leave(gone);
            }
            let mut due = Vec::new();
            for b in &mut bots {
                due.extend(b.think(m.world(), Tick(k)));
            }
            m.step(due);
            if k == 700 {
                let p = m.free_player().unwrap();
                m.join(p, Some(ChampionId::Shade));
                bots.push(Bot::new(p, 8));
            }
        }
        let replay = Replay::from_text(&m.replay().to_text()).unwrap();
        let check = replay.verify();
        assert_eq!(check.mismatch, None);
        assert_eq!(check.final_hash, m.world().state_hash());
        assert_eq!(check.hashes_checked, 3);
    }

    /// Champion select between matches: everyone leaves, the world restarts empty, and new
    /// champions join before the next tick. The replay holds the restart and re-simulates.
    #[test]
    fn replays_handle_a_restart_between_matches() {
        let mut m = Match::new(ServerConfig { scenario: Scenario::Aram, record: true, ..Default::default() });
        let mut bots: Vec<Bot> = (0..4u8).map(|p| Bot::new(PlayerId(p), 3)).collect();
        for b in &bots {
            m.join(b.player, None);
        }
        for k in 1..=700u32 {
            if k == 400 {
                for p in m.players() {
                    m.leave(p);
                }
                m.restart();
                bots = (0..4u8).map(|p| Bot::new(PlayerId(p), 5)).collect();
                for (i, b) in bots.iter().enumerate() {
                    m.join(b.player, Some(ChampionId::ALL[5 - i]));
                }
            }
            let mut due = Vec::new();
            for b in &mut bots {
                due.extend(b.think(m.world(), Tick(k)));
            }
            m.step(due);
        }
        let text = m.replay().to_text();
        assert!(text.contains("\nR 399\n"), "the restart is recorded");
        let replay = Replay::from_text(&text).unwrap();
        assert_eq!(replay, m.replay());
        // Hashes after ticks 300 and 600 (before and after the restart) and the final one.
        let check = replay.verify();
        assert_eq!(check.mismatch, None);
        assert_eq!(check.hashes_checked, 3);
        assert_eq!(check.final_hash, m.world().state_hash());
    }

    /// After a Base falls, a server with champion select holds it again for everyone still
    /// connected: a new random champion, fresh rerolls, bots in the free slots, and then a new
    /// match from scratch. Players aren't dropped in between.
    #[test]
    fn champion_select_again_after_each_match() {
        let cfg = ServerConfig { seed: 4, bots: 10, lobby: true, scenario: Scenario::Aram, ..Default::default() };
        let mut core = ServerCore::new(cfg, 0.0);
        let mut seq = 0u16;
        let mut packet = |core: &mut ServerCore, msg: ClientMessage, t: f64| {
            seq += 1;
            let bytes = encode_client(&PacketHeader { seq, ack: 0, ack_bits: 0 }, &msg);
            core.handle_packet(1, &bytes, t)
                .into_iter()
                .filter(|(to, _)| *to == 1)
                .map(|(_, b)| decode_server(&b).unwrap().1)
                .collect::<Vec<_>>()
        };
        let input = |commands: Vec<Command>| ClientMessage::Input {
            client_time_us: 0,
            event_ack: 0,
            snapshot_ack: 0,
            commands,
        };
        // Steps until key 1 hears `want`, keeping the connection alive.
        let mut t = 0.0;
        let until = |core: &mut ServerCore,
                     t: &mut f64,
                     packet: &mut dyn FnMut(&mut ServerCore, ClientMessage, f64) -> Vec<ServerMessage>,
                     want: &dyn Fn(&ServerMessage) -> bool| {
            let deadline = *t + 90.0;
            while *t < deadline {
                *t += 1.0 / 30.0;
                if ((*t * 30.0) as u32).is_multiple_of(15) {
                    packet(core, input(Vec::new()), *t);
                }
                for (to, b) in core.step(*t) {
                    if to == 1 {
                        let m = decode_server(&b).unwrap().1;
                        if want(&m) {
                            return m;
                        }
                    }
                }
            }
            panic!("nothing arrived by {t} (tick {:?}, winner {:?})", core.world().tick(), core.world().game().winner);
        };
        let is_welcome = |m: &ServerMessage| matches!(m, ServerMessage::Welcome { .. });
        let is_lobby = |m: &ServerMessage| matches!(m, ServerMessage::Lobby(_));

        packet(&mut core, hello_msg(0, false), t);
        packet(&mut core, ClientMessage::Lobby(msg::LobbyAction::Ready(true)), t);
        let ServerMessage::Welcome { unit, .. } = until(&mut core, &mut t, &mut packet, &is_welcome) else {
            unreachable!()
        };
        // Win quickly: the enemy's other structures are gone (the Base is protected until they
        // fall) and our champion stands next to its Base.
        let team = core.world().unit(unit).unwrap().team;
        let base = core.world().units().iter().find(|u| u.kind == mftr_sim::UnitKind::Base && u.team != team).unwrap();
        let (base_id, base_pos) = (base.id, base.state.pos);
        let guards: Vec<UnitId> = core
            .world()
            .units()
            .iter()
            .filter(|u| u.team != team && u.kind.is_structure() && u.tier > 0 && u.tier < base.tier)
            .map(|u| u.id)
            .collect();
        let w = core.world_mut();
        for g in guards {
            w.despawn(g);
        }
        w.unit_mut(base_id).unwrap().state.health = 1.0;
        let me = w.unit_mut(unit).unwrap();
        me.state.pos = base_pos + Vec2::new(if team == Team::Blue { -300.0 } else { 300.0 }, 0.0);
        me.stats.attack_damage = 5000.0;
        let k = core.world().tick().0 + 3;
        let attack = Command {
            player: PlayerId(0),
            seq: 1,
            tick: Tick(k),
            sub: SubTick::START,
            kind: mftr_sim::CommandKind::Attack(base_id),
        };
        packet(&mut core, input(vec![attack]), t);
        let ServerMessage::Lobby(l) = until(&mut core, &mut t, &mut packet, &is_lobby) else { unreachable!() };
        let ended = core.world().game().winner;
        assert!(ended.is_none(), "the world restarted");
        assert_eq!(core.game().player_count(), 0, "nobody plays during champion select");
        let me = *l.slots.iter().find(|s| s.player == l.you).unwrap();
        assert!(!me.bot && !me.ready && me.rerolls == lobby::REROLLS, "{me:?}");
        assert_eq!(l.slots.len(), 10);
        assert!(l.starts_in_ms > 50_000, "a full champion select");

        let l = packet(&mut core, ClientMessage::Lobby(msg::LobbyAction::Reroll), t);
        let rerolled = l
            .iter()
            .find_map(|m| match m {
                ServerMessage::Lobby(l) => l.slots.iter().find(|s| s.player == l.you).map(|s| s.champion),
                _ => None,
            })
            .unwrap();
        packet(&mut core, ClientMessage::Lobby(msg::LobbyAction::Ready(true)), t);
        let ServerMessage::Welcome { unit, .. } = until(&mut core, &mut t, &mut packet, &is_welcome) else {
            unreachable!()
        };
        assert_eq!(core.world().unit(unit).unwrap().champion, Some(rerolled));
        assert_eq!(core.game().player_count(), 10);
        let base = core.world().unit(base_id);
        assert!(base.is_none(), "structures were rebuilt as new units");
        assert_eq!(core.world().units().iter().filter(|u| u.kind == mftr_sim::UnitKind::Base).count(), 2);
    }

    #[test]
    fn duel_assigns_champions_and_spawns() {
        let mut core = ServerCore::new(ServerConfig { scenario: Scenario::Duel, ..Default::default() }, 0.0);
        hello(&mut core, 1);
        hello(&mut core, 2);
        let unit = |core: &ServerCore, key| core.game.world.unit(core.conns[&key].unit).unwrap().clone();
        assert_eq!(unit(&core, 1).champion, Some(ChampionId::Ember));
        assert_eq!(unit(&core, 2).champion, Some(ChampionId::Vesper));
        assert_eq!(unit(&core, 1).state.pos, Vec2::new(500.0, 2000.0));
        assert_eq!(unit(&core, 2).state.pos, Vec2::new(3500.0, 2000.0));
    }

    /// Fog applies to combat events: blue lobs an area into a bush where red hides. The server
    /// damages red and red's client hears it, but blue's client learns nothing about red.
    #[test]
    fn damage_to_a_hidden_unit_is_not_sent_to_the_attacker() {
        let mut core = ServerCore::new(ServerConfig { scenario: Scenario::Duel, ..Default::default() }, 0.0);
        hello_as(&mut core, 1, ChampionId::Vesper);
        hello_as(&mut core, 2, ChampionId::Ember);
        let (blue, red) = (core.conns[&1].unit, core.conns[&2].unit);
        let minions: Vec<UnitId> = core.game.world.units().iter().filter(|u| u.owner.is_none()).map(|u| u.id).collect();
        for id in minions {
            core.world_mut().despawn(id);
        }
        core.world_mut().unit_mut(red).unwrap().state.pos = Vec2::new(1015.0, 1675.0); // brush 2
        core.world_mut().unit_mut(blue).unwrap().state.pos = Vec2::new(1015.0, 1250.0); // outside, 425 u away
        let cast = ClientMessage::Input {
            client_time_us: 0,
            event_ack: 0,
            snapshot_ack: 0,
            commands: vec![Command {
                player: PlayerId(0),
                seq: 1,
                tick: Tick(2),
                sub: SubTick::START,
                kind: mftr_sim::CommandKind::Cast { slot: 1, target: QPoint::from_vec2(Vec2::new(1015.0, 1675.0)) },
            }],
        };
        core.handle_packet(1, &encode_client(&PacketHeader { seq: 1, ack: 0, ack_bits: 0 }, &cast), 0.0);
        let (mut blue_heard, mut red_heard, mut blue_saw_red) = (false, false, false);
        for i in 1..=60 {
            for (to, bytes) in core.step(i as f64 / 30.0) {
                let Ok((_, ServerMessage::Snapshot(s))) = decode_server(&bytes) else { continue };
                let about_red =
                    s.events.iter().any(|(_, e)| matches!(e, SimEvent::Damage { target, .. } if *target == red));
                if to == 1 {
                    blue_heard |= about_red;
                    blue_saw_red |= s.others.iter().any(|o| o.unit.id == red);
                } else {
                    red_heard |= about_red;
                }
            }
        }
        let hp = core.game.world.unit(red).unwrap().state.health;
        assert!(hp < ChampionId::Ember.def().stats.max_health - 50.0, "red took the area: {hp}");
        assert!(red_heard, "the victim's client hears its own damage");
        assert!(!blue_saw_red && !blue_heard, "nothing about the hidden unit reaches blue");
    }
}
