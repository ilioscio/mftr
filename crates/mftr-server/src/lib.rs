//! `mftr-server`: the authoritative match server core.
//!
//! Transport- and clock-agnostic: feed it packets with timestamps, call [`ServerCore::step`]
//! when [`ServerCore::next_tick_due`] has passed, and send the packets it returns. The
//! virtual-time Netcode Lab drives one core; the UDP binary (`main.rs`) drives a [`Hub`] of
//! them, one per match instance (13).

use mftr_net::PROTOCOL_VERSION;
use mftr_net::delta::{self, UnitUpdate};
use mftr_net::msg::{self, ClientMessage, CommandReport, RejectReason, RemoteUnit, ServerMessage, Snapshot, TimeEcho};
use mftr_net::packet::{PacketHeader, ReceiveTracker, SendTracker};
use mftr_sim::ability::LineSkillshot;
use mftr_sim::map::MapId;
use mftr_sim::vision::Vision;
use mftr_sim::{
    Area, ChampionId, Channel, Command, MinionKind, Missile, PlayerId, QPoint, SimEvent, SimTime, SubTick, TICK_DT_F64,
    TICK_HZ, Team, Tick, UnitId, UnitKind, Vec2, World,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub mod game;
pub mod hub;
pub mod inspect;
pub mod lobby;
pub use game::{Fog, Match, Replay, ReplayCheck, ReplayEntry};
pub use hub::{Hub, HubConfig, QueueSpec};
use lobby::Lobby;
pub use mftr_bots::{Bots, Tier};

/// Opaque per-connection address key, assigned by the transport.
pub type ClientKey = u64;

/// Commands later than this are dropped instead of applied late (03a §10.2).
const MAX_LATENESS: f64 = 0.250;
/// Commands targeting further ahead than this are clamped (anti-abuse).
const MAX_LEAD_TICKS: u32 = 90;
const REPORT_REPEAT_TICKS: u32 = 15;
const TIMEOUT: f64 = 10.0;
/// A disconnected player's champion waits this long for a reconnect in the sandboxes (M2
/// slice 5). In a match a bot plays it until the player returns, and a match whose humans
/// have all dropped stops this long after the last one did (13 §4–5).
pub const RECONNECT_GRACE: f64 = 60.0;
/// Seconds a player joining a match in progress has to choose a champion (13 §3).
pub const MIDJOIN_SECONDS: f64 = 20.0;
/// Rerolls a player joining an ARAM match in progress gets.
const MIDJOIN_REROLLS: u8 = 1;
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
    /// M3: ARAM: Mayhem under Hyper rules (06 §2): much faster basic abilities and attacks.
    Hyper,
    /// M4: Classic 5v5 on Crossroads: three lanes, level 1 and 500 gold, fountains that heal.
    Classic,
    /// Five champions defend against 20 Mayhem horde waves, then endless survival.
    Horde,
}

impl Scenario {
    /// The map each scenario is played on: the open plane for `empty`, the arena otherwise.
    pub fn map(self) -> MapId {
        match self {
            Scenario::Empty => MapId::Open,
            Scenario::MinionSandbox | Scenario::DodgeRig | Scenario::Duel => MapId::Arena,
            Scenario::Aram | Scenario::Mayhem | Scenario::Hyper => MapId::Bridge,
            Scenario::Classic => MapId::Crossroads,
            Scenario::Horde => MapId::HordeBridge,
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
            "hyper" => Some(Scenario::Hyper),
            "classic" => Some(Scenario::Classic),
            "horde" => Some(Scenario::Horde),
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
            Scenario::Hyper => msg::GameMode::Hyper,
            Scenario::Classic => msg::GameMode::Classic,
            Scenario::Horde => msg::GameMode::Horde,
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
            Scenario::Hyper => "hyper",
            Scenario::Classic => "classic",
            Scenario::Horde => "horde",
        }
    }

    /// A full match with champion select, structures and a winner: ARAM and its variants on
    /// The Bridge, Classic on Crossroads.
    pub fn is_match(self) -> bool {
        matches!(self, Scenario::Aram | Scenario::Mayhem | Scenario::Hyper | Scenario::Classic | Scenario::Horde)
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
    /// A custom map's text (`*.mftrmap`, 12 §4) to play instead of the scenario's map; sent to
    /// each player as they join.
    pub map: Option<std::sync::Arc<str>>,
    /// Co-op vs bots (13 §1): humans play blue only. Teams follow player ids, so replays
    /// don't need it.
    pub coop: bool,
    /// How well bots play (17 §3): the starting choice in co-op champion select, and the tier
    /// of Versus fill-ins and of bots that take over dropped players.
    pub bot_tier: Tier,
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
            map: None,
            coop: false,
            bot_tier: Tier::Veteran,
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

impl ServerStats {
    pub fn add(&mut self, o: &ServerStats) {
        self.ticks += o.ticks;
        self.commands += o.commands;
        self.commands_late += o.commands_late;
        self.commands_dropped += o.commands_dropped;
        self.bytes_in += o.bytes_in;
        self.bytes_out += o.bytes_out;
        self.decode_errors += o.decode_errors;
    }
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
    /// When this client last got an inspect answer (they're rate limited).
    inspected_at: f64,
    /// When this client's recent pings were relayed (01 §14: rate limited).
    pinged: VecDeque<f64>,
}

/// Fastest a client gets inspect answers (seconds apart): the window asks a few times a second.
const INSPECT_EVERY: f64 = 0.1;

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
            inspected_at: f64::NEG_INFINITY,
            pinged: VecDeque::new(),
        }
    }

    /// Whether a ping at `now` is within the limits (`mftr_sim::ping`), and if so note it.
    fn may_ping(&mut self, now: f64) -> bool {
        use mftr_sim::ping::{PING_BURST, PING_GAP_S, PING_WINDOW_S};
        while self.pinged.front().is_some_and(|t| now - t > PING_WINDOW_S) {
            self.pinged.pop_front();
        }
        if self.pinged.back().is_some_and(|t| now - t < PING_GAP_S) || self.pinged.len() >= PING_BURST {
            return false;
        }
        self.pinged.push_back(now);
        true
    }
}

/// A connection in champion select: its player, when last heard, and its packet sequence
/// (carried into the connection when the match starts, so the client accepts the Welcome).
struct Pending {
    player: PlayerId,
    last_heard: f64,
    send: SendTracker,
    /// Joining the match in progress instead (13 §3): `player` is the bot's seat to take.
    mid: Option<MidJoin>,
}

/// A champion being chosen to join a match in progress with.
#[derive(Clone, Copy, Debug)]
struct MidJoin {
    champion: ChampionId,
    rerolls: u8,
    ready: bool,
    /// When the player joins, ready or not (server clock).
    deadline: f64,
}

/// A player whose connection dropped (at `since`): their seat, until they're back.
struct Dropped {
    token: u64,
    player: PlayerId,
    unit: UnitId,
    team: Team,
    since: f64,
}

pub struct ServerCore {
    cfg: ServerConfig,
    game: Match,
    bots: Bots,
    /// How well this match's bots play (co-op: as voted in champion select, 17 §9).
    tier: Tier,
    /// Champion select, until the match starts.
    lobby: Option<Lobby>,
    /// Connections in champion select.
    pending: BTreeMap<ClientKey, Pending>,
    dropped: Vec<Dropped>,
    /// Session tokens (independent of the world RNG, so replays are unaffected).
    tokens: mftr_sim::rng::Pcg32,
    lobby_steps: u32,
    /// The map's name, for champion select.
    map_name: String,
    /// Matches finished: each champion select after one draws from a new seed.
    rounds: u64,
    start: f64,
    conns: BTreeMap<ClientKey, Conn>,
    queue: Vec<Command>,
    /// Live missiles, areas and beams (for fog-of-war reveal), by id.
    missiles: BTreeMap<u32, Missile>,
    areas: BTreeMap<u32, Area>,
    channels: BTreeMap<u32, Channel>,
    pub stats: ServerStats,
}

impl ServerCore {
    /// `start`: the server-clock time (seconds) at which tick 0 ends.
    pub fn new(mut cfg: ServerConfig, start: f64) -> Self {
        if cfg.scenario == Scenario::Horde {
            cfg.max_players = mftr_sim::horde::PLAYERS;
            cfg.bots = cfg.bots.min(cfg.max_players);
            cfg.coop = true;
        }
        let mut game = Match::new(cfg.clone());
        let mut bots = Bots::new(cfg.seed);
        let lobby = cfg.lobby.then(|| Self::make_lobby(&cfg, cfg.seed));
        if !cfg.lobby {
            for _ in 0..cfg.bots.min(cfg.max_players) {
                let Some(player) = game.free_player() else { break };
                game.join(player, None);
                bots.add(player, cfg.bot_tier);
            }
        }
        Self {
            game,
            bots,
            tier: cfg.bot_tier,
            lobby,
            pending: BTreeMap::new(),
            dropped: Vec::new(),
            tokens: mftr_sim::rng::Pcg32::new(cfg.seed ^ 0x746f_6b65_6e73, 7),
            lobby_steps: 0,
            map_name: match &cfg.map {
                Some(text) => mftr_sim::mapspec::MapSpec::parse(text).map_or_else(|_| "?".into(), |s| s.name),
                None => hub::map_name(cfg.scenario).to_string(),
            },
            rounds: 0,
            cfg,
            start,
            conns: BTreeMap::new(),
            queue: Vec::new(),
            missiles: BTreeMap::new(),
            areas: BTreeMap::new(),
            channels: BTreeMap::new(),
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

    /// Humans with a stake in this match (13 §5): playing, in champion select, or holding a
    /// dropped seat. Spectators don't count.
    pub fn humans(&self) -> usize {
        self.conns.values().filter(|c| !c.spectator).count() + self.pending.len() + self.dropped.len()
    }

    /// Humans connected and playing or in champion select.
    pub fn connected_humans(&self) -> usize {
        self.conns.values().filter(|c| !c.spectator).count() + self.pending.len()
    }

    /// Nobody to play for (13 §5): no human connected, and none dropped within
    /// `RECONNECT_GRACE`. The hub stops such a match.
    pub fn abandoned(&self, now: f64) -> bool {
        self.connected_humans() == 0 && self.dropped.iter().all(|d| now - d.since >= RECONNECT_GRACE)
    }

    pub fn spectators(&self) -> usize {
        self.conns.values().filter(|c| c.spectator).count()
    }

    /// Whether connection `key` is here (playing, watching or in champion select).
    pub fn knows(&self, key: ClientKey) -> bool {
        self.conns.contains_key(&key) || self.pending.contains_key(&key)
    }

    /// Whether a player with session token `token` is here or holds a dropped seat.
    pub fn holds_token(&self, token: u64) -> bool {
        token != 0 && (self.conns.values().any(|c| c.token == token) || self.dropped.iter().any(|d| d.token == token))
    }

    /// Still in champion select (no match playing yet).
    pub fn in_champion_select(&self) -> bool {
        self.lobby.is_some()
    }

    /// Whether a new human can join now (13 §2): a seat in champion select, a bot's seat in
    /// the match being played, or a free slot on a server without champion select.
    pub fn has_room(&self) -> bool {
        match &self.lobby {
            Some(lobby) => lobby.team_for_human(self.cfg.coop).is_some(),
            None if self.cfg.lobby => self.free_seat().is_some(),
            None => self.game.player_count() < self.cfg.max_players as usize,
        }
    }

    pub fn cfg(&self) -> &ServerConfig {
        &self.cfg
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

    /// Every champion's score at `t`, and when the match began.
    fn scoreboard(&self, t: SimTime) -> msg::Scoreboard {
        let world = &self.game.world;
        let rows = world
            .units()
            .iter()
            .filter_map(|u| {
                let champion = u.champion?;
                let p = &u.state.progress;
                let respawn = u.state.respawn_at.map_or(0.0, |r| r.secs_since(t).max(0.0));
                Some(msg::ScoreRow {
                    unit: u.id,
                    champion,
                    team: u.team,
                    bot: u.owner.is_some_and(|o| self.bots.contains(o)),
                    level: p.level,
                    kills: p.kills,
                    deaths: p.deaths,
                    assists: p.assists,
                    cs: p.cs,
                    items: p.items,
                    augments: p.augments,
                    respawn_ds: (respawn * 10.0).ceil().min(u16::MAX as f32) as u16,
                    spell_f: u.state.progress.spell_f,
                })
            })
            .take(msg::MAX_SCORE_ROWS)
            .collect();
        // Turrets down on each side: blue destroyed red's, and the other way round.
        let down = |team: Team| {
            world
                .units()
                .iter()
                .filter(|u| u.kind == mftr_sim::UnitKind::Turret && u.team == team && !u.state.alive())
                .count() as u8
        };
        msg::Scoreboard { started_at: world.game().started_at, towers: [down(Team::Red), down(Team::Blue)], rows }
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
            ClientMessage::Hello { protocol, client_time_us, champion, resume, spectate, .. } => {
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
                        // Reconnect: the champion is still there (in a match, the bot playing
                        // it hands it back).
                        let d = self.dropped.remove(i);
                        self.bots.remove(d.player);
                        self.conns.insert(from, Conn::new(d.player, d.unit, d.team, d.token, false, now));
                    } else if let Some(old) = live {
                        // The same player from a new address (a restarted client): take over.
                        let c = self.conns.remove(&old).unwrap();
                        self.conns.insert(from, Conn::new(c.player, c.unit, c.team, c.token, false, now));
                    } else if let Some(lobby) = &mut self.lobby {
                        let Some(player) = lobby.add_human(now, self.cfg.coop) else {
                            out.push((from, self.reject(RejectReason::ServerFull)));
                            return out;
                        };
                        self.pending
                            .insert(from, Pending { player, last_heard: now, send: SendTracker::default(), mid: None });
                        out.extend(self.lobby_packet(from, player, now));
                        return out;
                    } else if let Some(seat) = self.free_seat() {
                        // A match in progress: choose a champion, then take over a bot (13 §3).
                        let champion = self.deal(seat, &[]);
                        let rerolls = if self.pick() { 0 } else { MIDJOIN_REROLLS };
                        let mid = MidJoin { champion, rerolls, ready: false, deadline: now + MIDJOIN_SECONDS };
                        self.pending.insert(
                            from,
                            Pending { player: seat, last_heard: now, send: SendTracker::default(), mid: Some(mid) },
                        );
                        out.extend(self.lobby_packet(from, seat, now));
                        return out;
                    } else if self.cfg.lobby || self.game.player_count() >= self.cfg.max_players as usize {
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
                if let Some(p) = self.pending.get_mut(&from) {
                    p.last_heard = now;
                    let player = p.player;
                    if p.mid.is_some() {
                        self.midjoin_act(from, action);
                    } else if let Some(lobby) = &mut self.lobby {
                        lobby.act(player, action);
                    }
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
            ClientMessage::MapRequest { hash, have } => {
                // A piece of the custom map, to players and those in champion select only.
                let Some(text) = self.cfg.map.clone() else { return out };
                if mftr_sim::mapspec::text_hash(&text) != hash || have as usize > text.len() {
                    return out;
                }
                let header = if let Some(conn) = self.conns.get_mut(&from) {
                    conn.last_heard = now;
                    Self::header(conn)
                } else if let Some(p) = self.pending.get_mut(&from) {
                    p.last_heard = now;
                    PacketHeader::default()
                } else {
                    return out;
                };
                let start = have as usize;
                let end = (start + msg::MAP_CHUNK_BYTES).min(text.len());
                let chunk = ServerMessage::MapChunk {
                    hash,
                    total: text.len() as u32,
                    offset: have,
                    bytes: text.as_bytes()[start..end].to_vec(),
                };
                let bytes = msg::encode_server(&header, &chunk);
                self.stats.bytes_out += bytes.len() as u64;
                out.push((from, bytes));
            }
            ClientMessage::Ping { kind, at, target } => {
                // A ping (17 §8): to the pinger's team (and spectators), and its bots.
                let Some(conn) = self.conns.get_mut(&from) else { return out };
                conn.last_heard = now;
                if conn.spectator || !conn.may_ping(now) {
                    return out;
                }
                let (team, unit) = (conn.team, conn.unit);
                let at = Vec2::new(at.x.clamp(0.0, 65_000.0), at.y.clamp(0.0, 65_000.0));
                self.relay_ping(team, mftr_sim::ping::Ping { kind, at, target, from: Some(unit) }, true);
            }
            ClientMessage::Inspect { unit } => {
                // The inspect window (01 §13): a unit's details, if the asker's team sees it.
                let Some(conn) = self.conns.get_mut(&from) else { return out };
                conn.last_heard = now;
                if now - conn.inspected_at < INSPECT_EVERY {
                    return out;
                }
                conn.inspected_at = now;
                let header = Self::header(conn);
                let team = (!conn.spectator).then_some(conn.team);
                if let Some(info) = inspect::unit_info(&self.game.world, unit, team) {
                    let bytes = msg::encode_server(&header, &ServerMessage::Inspect(Box::new(info)));
                    self.stats.bytes_out += bytes.len() as u64;
                    out.push((from, bytes));
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
            map_spec: self.cfg.map.as_ref().map(|t| (mftr_sim::mapspec::text_hash(t), t.len() as u32)),
        };
        let conn = self.conns.get_mut(&key)?;
        let h = Self::header(conn);
        let bytes = msg::encode_server(&h, &msg);
        self.stats.bytes_out += bytes.len() as u64;
        Some((key, bytes))
    }

    /// Champion select as `player` sees it.
    fn lobby_packet(&mut self, key: ClientKey, player: PlayerId, now: f64) -> Option<(ClientKey, Vec<u8>)> {
        let mut state = match self.pending.get(&key)?.mid {
            Some(mid) => self.midjoin_state(player, mid, now),
            None => {
                let lobby = self.lobby.as_mut()?;
                let starts_in = lobby.starts_in(now).unwrap_or(lobby::LOBBY_SECONDS);
                lobby.state_for(player, starts_in)
            }
        };
        state.mode = self.cfg.scenario.mode();
        state.map = self.map_name.clone();
        let seq = self.pending.get_mut(&key)?.send.next_seq();
        let h = PacketHeader { seq, ack: 0, ack_bits: 0 };
        let bytes = msg::encode_server(&h, &ServerMessage::Lobby(Box::new(state)));
        self.stats.bytes_out += bytes.len() as u64;
        Some((key, bytes))
    }

    /// Send a ping to `team`'s players and every spectator; a human's also goes to its bots.
    fn relay_ping(&mut self, team: Team, ping: mftr_sim::ping::Ping, human: bool) {
        for conn in self.conns.values_mut().filter(|c| c.spectator || c.team == team) {
            conn.events.push_back((conn.next_event_seq, SimEvent::Ping(ping)));
            conn.next_event_seq += 1;
        }
        if human {
            self.bots.hear(team, ping, &self.game.world);
        }
    }

    /// Champion select picks (Classic) instead of dealing (ARAM modes).
    fn pick(&self) -> bool {
        matches!(self.cfg.scenario, Scenario::Classic | Scenario::Horde)
    }

    fn make_lobby(cfg: &ServerConfig, seed: u64) -> Lobby {
        let lobby = if cfg.scenario == Scenario::Horde {
            Lobby::blue_only(seed, cfg.max_players, cfg.bots, true)
        } else {
            Lobby::new(seed, cfg.max_players, cfg.bots, cfg.scenario == Scenario::Classic)
        };
        // Co-op humans choose how well the bots play (17 §9).
        if cfg.coop { lobby.with_bot_vote(cfg.bot_tier) } else { lobby }
    }

    /// The seat a new human takes in the match being played (13 §2–3): a bot's that isn't
    /// held for a dropped player or being taken by another joiner, on the team with fewer
    /// humans (in Co-op, blue). None once the match is decided.
    fn free_seat(&self) -> Option<PlayerId> {
        if !self.cfg.lobby || !self.cfg.scenario.is_match() || self.lobby.is_some() {
            return None;
        }
        if self.game.world.game().winner.is_some() {
            return None;
        }
        let held: Vec<PlayerId> = self
            .dropped
            .iter()
            .map(|d| d.player)
            .chain(self.pending.values().filter(|p| p.mid.is_some()).map(|p| p.player))
            .collect();
        let humans = |team: Team| {
            self.conns.values().filter(|c| !c.spectator && c.team == team).count()
                + held.iter().filter(|p| lobby::team_of(**p) == team).count()
        };
        let seat = |team: Team| self.bots.players().filter(|p| !held.contains(p) && lobby::team_of(*p) == team).min();
        let mut teams = if self.cfg.coop { vec![Team::Blue] } else { vec![Team::Blue, Team::Red] };
        teams.sort_by_key(|t| humans(*t));
        teams.into_iter().find_map(seat)
    }

    /// Champions `seat`'s team has, apart from the seat's own: on the field, and chosen by
    /// other joiners.
    fn team_champions(&self, seat: PlayerId) -> Vec<ChampionId> {
        let team = lobby::team_of(seat);
        let field = self
            .game
            .world
            .units()
            .iter()
            .filter(|u| u.team == team && u.kind == UnitKind::Champion && u.owner != Some(seat))
            .filter_map(|u| u.champion);
        let joining = self
            .pending
            .values()
            .filter(|p| p.player != seat && lobby::team_of(p.player) == team)
            .filter_map(|p| p.mid.map(|m| m.champion));
        field.chain(joining).collect()
    }

    /// A random champion `seat`'s team doesn't have, other than `besides` (not part of the
    /// simulation: the takeover records the choice).
    fn deal(&mut self, seat: PlayerId, besides: &[ChampionId]) -> ChampionId {
        let taken = self.team_champions(seat);
        let free: Vec<ChampionId> =
            ChampionId::ALL.into_iter().filter(|c| !taken.contains(c) && !besides.contains(c)).collect();
        let pool = if free.is_empty() { ChampionId::ALL.to_vec() } else { free };
        pool[self.tokens.next_u32() as usize % pool.len()]
    }

    /// A joiner's champion select action: ready, a reroll (ARAM modes), a pick (Classic).
    fn midjoin_act(&mut self, key: ClientKey, action: msg::LobbyAction) {
        let Some((seat, mut mid)) = self.pending.get(&key).and_then(|p| Some((p.player, p.mid?))) else { return };
        match action {
            msg::LobbyAction::Ready(r) => mid.ready = r,
            msg::LobbyAction::Reroll if !self.pick() && mid.rerolls > 0 => {
                mid.champion = self.deal(seat, &[mid.champion]);
                mid.rerolls -= 1;
            }
            msg::LobbyAction::Take(c) if self.pick() && !self.team_champions(seat).contains(&c) => mid.champion = c,
            _ => {}
        }
        if let Some(p) = self.pending.get_mut(&key) {
            p.mid = Some(mid);
        }
    }

    /// Champion select as a joiner sees it: the match's champions, our seat with our choice.
    fn midjoin_state(&self, you: PlayerId, mid: MidJoin, now: f64) -> msg::LobbyState {
        let slots = self
            .game
            .players()
            .into_iter()
            .map(|p| {
                let current = self
                    .game
                    .world
                    .units()
                    .iter()
                    .find(|u| u.owner == Some(p) && u.kind == UnitKind::Champion)
                    .and_then(|u| u.champion);
                let mine = p == you;
                msg::LobbySlot {
                    player: p,
                    team: lobby::team_of(p),
                    champion: if mine { mid.champion } else { current.unwrap_or(ChampionId::Ember) },
                    rerolls: if mine { mid.rerolls } else { 0 },
                    ready: if mine { mid.ready } else { true },
                    bot: !mine && self.bots.contains(p),
                }
            })
            .collect();
        msg::LobbyState {
            you,
            slots,
            bench: Vec::new(),
            starts_in_ms: ((mid.deadline - now).max(0.0) * 1000.0) as u32,
            mode: msg::GameMode::Empty,
            map: String::new(),
            pick: self.pick(),
            midgame: true,
            bots: None,
        }
    }

    /// A joiner is ready (or out of time): their champion takes the bot's seat (13 §3), and
    /// they get their connection and a Welcome.
    fn finish_midjoin(&mut self, key: ClientKey, now: f64) -> Option<(ClientKey, Vec<u8>)> {
        let p = self.pending.remove(&key)?;
        let mid = p.mid?;
        let (unit, team) = self.game.takeover(p.player, mid.champion)?;
        self.bots.remove(p.player);
        let token = self.new_token();
        let mut conn = Conn::new(p.player, unit, team, token, false, now);
        conn.send = p.send;
        self.conns.insert(key, conn);
        self.welcome(key, 0, now)
    }

    /// Champion select is over: everyone joins the match in slot order; bots get brains, and
    /// humans their connection and a Welcome.
    fn start_from_lobby(&mut self, now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        let Some(lobby) = self.lobby.take() else { return Vec::new() };
        let mut out = Vec::new();
        self.tier = lobby.bot_tier().unwrap_or(self.cfg.bot_tier);
        for slot in lobby.slots() {
            let (unit, team, _) = self.game.join(slot.player, Some(slot.champion));
            if slot.bot {
                self.bots.add(slot.player, self.tier);
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
        let mut lobby = Self::make_lobby(&self.cfg, seed);
        let players: Vec<ClientKey> = self.conns.iter().filter(|(_, c)| !c.spectator).map(|(k, _)| *k).collect();
        // Those who were still choosing a champion to join with come along.
        let joining = std::mem::take(&mut self.pending);
        for key in players {
            let c = self.conns.remove(&key).expect("listed");
            if let Some(player) = lobby.add_human(now, self.cfg.coop) {
                // The packet sequence carries over, so the client accepts what follows.
                self.pending.insert(key, Pending { player, last_heard: c.last_heard, send: c.send, mid: None });
            }
        }
        for (key, p) in joining {
            if let Some(player) = lobby.add_human(now, self.cfg.coop) {
                self.pending.insert(key, Pending { player, last_heard: p.last_heard, send: p.send, mid: None });
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
        self.channels.clear();
        self.game.restart();
        self.lobby = Some(lobby);
        self.lobby_steps = 0;
        let pending: Vec<(ClientKey, PlayerId)> = self.pending.iter().map(|(k, p)| (*k, p.player)).collect();
        pending.into_iter().filter_map(|(key, player)| self.lobby_packet(key, player, now)).collect()
    }

    /// A player said goodbye. In a match a bot plays on with their champion (13 §4; the seat
    /// isn't held, they chose to leave); elsewhere the champion goes.
    fn leave(&mut self, key: ClientKey) {
        if let Some(c) = self.conns.remove(&key)
            && !c.spectator
        {
            self.queue.retain(|cmd| cmd.player != c.player);
            if self.cfg.scenario.is_match() {
                self.bots.add(c.player, self.tier);
            } else {
                self.game.leave(c.player);
            }
        }
    }

    /// Simulate the next tick and build one snapshot per client.
    pub fn step(&mut self, now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        // Silent connections: spectators go, players' champions wait for a reconnect. In a
        // match a bot plays the champion meanwhile (13 §4).
        let timed_out: Vec<ClientKey> =
            self.conns.iter().filter(|(_, c)| now - c.last_heard > TIMEOUT).map(|(k, _)| *k).collect();
        for key in timed_out {
            let c = self.conns.remove(&key).unwrap();
            if !c.spectator {
                self.queue.retain(|cmd| cmd.player != c.player);
                if self.cfg.scenario.is_match() {
                    self.bots.add(c.player, self.tier);
                }
                self.dropped.push(Dropped { token: c.token, player: c.player, unit: c.unit, team: c.team, since: now });
            }
        }
        // The sandboxes: a champion waits `RECONNECT_GRACE`, then goes.
        if !self.cfg.scenario.is_match() {
            let (gone, kept): (Vec<Dropped>, Vec<Dropped>) =
                self.dropped.drain(..).partition(|d| now - d.since > RECONNECT_GRACE);
            self.dropped = kept;
            for d in gone {
                self.game.leave(d.player);
            }
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
        // Players joining the match in progress (13 §3): silent ones go (the seat stays a
        // bot's); ready ones, or those out of time, take over their seat before this tick; the
        // rest see their champion select a few times a second.
        let mut joined = Vec::new();
        let joining: Vec<(ClientKey, PlayerId, f64, MidJoin)> =
            self.pending.iter().filter_map(|(key, p)| Some((*key, p.player, p.last_heard, p.mid?))).collect();
        for (key, player, heard, mid) in joining {
            if now - heard > TIMEOUT {
                self.pending.remove(&key);
            } else if mid.ready || now >= mid.deadline {
                joined.extend(self.finish_midjoin(key, now));
            } else if k.0.is_multiple_of(10) {
                joined.extend(self.lobby_packet(key, player, now));
            }
        }
        let (mut due, later): (Vec<Command>, Vec<Command>) = self.queue.drain(..).partition(|c| c.tick <= k);
        self.queue = later;
        due.extend(self.bots.think(&self.game.world, k));
        let (events, fog) = self.game.step(due);
        self.bots.observe(&self.game.world, &events, &fog.visions);
        for (team, ping) in self.bots.take_pings() {
            self.relay_ping(team, ping, false);
        }
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
                    // A unit that can't walk this tick (rooted, stunned, winding up, in a
                    // follow-through) sends no heading, so clients don't extrapolate it walking.
                    target: if st.dash.is_some() || st.can_move(s1) {
                        st.heading().map(QPoint::from_vec2)
                    } else {
                        None
                    },
                    speed: st.speed_at(s1).round().clamp(0.0, 1023.0) as u16,
                    collision_radius: u.collision_radius.round().clamp(0.0, 255.0) as u8,
                    gameplay_radius: u.gameplay_radius.round().clamp(0.0, 255.0) as u8,
                    protected: u.protected,
                    recalling: st.recalling(),
                    plates: u.plates,
                    champion: u.champion,
                    monster: u.monster,
                    ward: u.ward,
                    minion: (u.kind == UnitKind::Minion)
                        .then(|| MinionKind::from_attack_range(u.attack.map_or(0.0, |a| a.range))),
                    augments: if u.champion.is_some() { st.progress.augments } else { [0; mftr_sim::augments::SLOTS] },
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
                    recovering: st.recovering(s1),
                    attack_variant: st.attacks & 3,
                    facing: mftr_net::msg::facing_to_wire(st.facing),
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
                SimEvent::BeamStarted(c) => {
                    self.channels.insert(c.id, c);
                }
                SimEvent::MissileHit { id, at, .. }
                | SimEvent::MissileExpired { id, at }
                | SimEvent::AreaDetonated { id, at }
                | SimEvent::BeamInterrupted { id, at } => {
                    ended.insert(id, at);
                }
                _ => {}
            }
        }
        // The scoreboard, about once a second (the same for everyone: it's what the Tab
        // breakdown shows, items and augments included, as in the reference game).
        let scoreboard = k.0.is_multiple_of(mftr_sim::TICK_HZ).then(|| Box::new(self.scoreboard(SimTime::end_of(k))));
        let horde = self.game.world.game().horde.as_ref().map(|h| {
            let enemies = self
                .game
                .world
                .units()
                .iter()
                .filter(|u| {
                    u.team == Team::Red && u.state.alive() && matches!(u.kind, UnitKind::Minion | UnitKind::Monster)
                })
                .count() as u16;
            let boss = h.boss.and_then(|id| self.game.world.unit(id)).filter(|u| u.state.alive()).map(|u| {
                (u.id, u.state.health.round().clamp(1.0, u16::MAX as f32) as u16, u.stats.max_health.round() as u16)
            });
            let base = self
                .game
                .world
                .units()
                .iter()
                .find(|u| u.kind == UnitKind::Base && u.team == Team::Blue)
                .map_or((0, 0), |u| (u.state.health.max(0.0).round() as u16, u.stats.max_health.round() as u16));
            msg::HordeStatus {
                wave: h.wave,
                cleared: h.cleared,
                enemies,
                incoming: if h.wave > h.cleared { mftr_sim::horde::Wave::new(h.wave).count() - h.spawned } else { 0 },
                next_wave_at: self.game.world.game().next_wave_at,
                boss,
                base,
                won: h.won_at.is_some(),
                ended: self.game.world.game().winner.is_some(),
            }
        });
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
                    SimEvent::MatchEnded { .. } | SimEvent::HordeWon { .. } => true,
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

        let mut out = joined;
        out.reserve(self.conns.len());
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
                    SimEvent::BeamStarted(c) if at_spawn(c.team) => {
                        conn.revealed.insert(c.id);
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
            // Enemy beams: shown once any part of the line is in vision (its caster may not be).
            for c in self.channels.values() {
                if !at_spawn(c.team)
                    && !conn.revealed.contains(&c.id)
                    && !ended.contains_key(&c.id)
                    && [0.0, 0.25, 0.5, 0.75, 1.0].iter().any(|f| sees(ti, c.origin + c.dir * (c.spec.range * f)))
                {
                    conn.revealed.insert(c.id);
                    push(conn, SimEvent::BeamStarted(*c));
                }
            }
            // Ends: only for things this client knows about.
            for e in &events {
                if let SimEvent::MissileHit { id, .. }
                | SimEvent::MissileExpired { id, .. }
                | SimEvent::AreaDetonated { id, .. }
                | SimEvent::BeamInterrupted { id, .. }
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
                let key = |r: &RemoteUnit| {
                    let rank = if r.kind == UnitKind::Champion {
                        0
                    } else if horde.as_ref().and_then(|h| h.boss).is_some_and(|b| b.0 == r.id) {
                        1
                    } else {
                        2
                    };
                    (rank, r.pos.to_vec2().distance(own_pos))
                };
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
                horde: horde.clone(),
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
                scoreboard: scoreboard.clone(),
            };
            let h = Self::header(conn);
            // Stay under the packet limit (03b §1): the least important unit updates wait for a
            // later snapshot (those units coast, or appear later), then a backlog of reliable
            // events does.
            let mut bytes = msg::encode_server(&h, &ServerMessage::Snapshot(Box::new(snap.clone())));
            if bytes.len() > mftr_net::MAX_PAYLOAD_BYTES && snap.scoreboard.is_some() {
                // The scoreboard can wait for the next second.
                snap.scoreboard = None;
                bytes = msg::encode_server(&h, &ServerMessage::Snapshot(Box::new(snap.clone())));
            }
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
        // A beam that ran its course: clients draw it to its `end_at` on their own.
        let done: Vec<u32> =
            self.channels.values().filter(|c| ended.contains_key(&c.id) || c.end_at <= s1).map(|c| c.id).collect();
        for id in done {
            self.channels.remove(&id);
            for conn in self.conns.values_mut() {
                conn.revealed.remove(&id);
            }
        }
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
    /// Jungle monsters killed by each team's champions (blue, red).
    pub monsters_killed: [u32; 2],
    /// Structures that fell: when, whose, what, tier.
    pub falls: Vec<(Tick, Team, mftr_sim::UnitKind, u8)>,
    pub replay: Replay,
    /// Horde tuning checkpoints, after each clear (empty for ordinary matches).
    pub horde_waves: Vec<HordeWaveReport>,
    /// Each champion's match (17 §11).
    pub champions: Vec<ChampionReport>,
    /// What the bots cost, against the simulation.
    pub cost: BotCost,
}

/// One champion's match, for the measurements (17 §11).
#[derive(Clone, Debug)]
pub struct ChampionReport {
    pub player: PlayerId,
    pub team: Team,
    pub champion: ChampionId,
    pub tier: Option<Tier>,
    pub kills: u16,
    pub deaths: u16,
    pub assists: u16,
    pub cs: u16,
    /// Net worth (gold plus items bought) at 10 and 15 minutes and at the end.
    pub worth: [f32; 3],
    pub dead_secs: f32,
    pub monsters: u32,
    /// Enemy lane minions that died within 900 u of it while it lived (what it could have
    /// last-hit).
    pub minions_near: u32,
    /// Damage dealt to enemy champions, and taken from them.
    pub damage_dealt: f32,
    pub damage_taken: f32,
    /// Damage taken from minions and from structures.
    pub taken_minions: f32,
    pub taken_structures: f32,
    pub stats: Option<mftr_bots::agent::AgentStats>,
}

/// Time spent in bots and in the simulation over a bot match (microseconds).
#[derive(Clone, Copy, Debug, Default)]
pub struct BotCost {
    pub ticks: u32,
    pub bots_us: f64,
    pub bots_worst_us: f64,
    pub sim_us: f64,
}

#[derive(Clone, Debug)]
pub struct HordeWaveReport {
    pub wave: u32,
    pub tick: Tick,
    pub levels: Vec<u8>,
    /// Wallet + inventory purchase value, per champion (not counting consumed potions).
    pub build_gold: Vec<f32>,
    pub base_health: f32,
}

/// How a bot match is set up beyond its config.
#[derive(Clone, Debug, Default)]
pub struct BotMatchOptions {
    /// Champions for the first bot seats.
    pub team: Vec<ChampionId>,
    /// Each team's bot tier (else the config's for both).
    pub tiers: Option<[Tier; 2]>,
    /// Print what this player's bot does every second (development).
    pub trace: Option<PlayerId>,
    /// Print every champion's state when a structure falls (development).
    pub trace_falls: bool,
}

/// Play a match with server bots only, as fast as possible (no network, no clients), until a
/// Base falls or `max_ticks` pass.
pub fn run_bot_match(cfg: ServerConfig, max_ticks: u32) -> BotMatch {
    run_bot_match_with(cfg, max_ticks, &BotMatchOptions::default())
}

/// The same balance/replay check with explicit champions for the first bot seats.
pub fn run_bot_match_with_team(cfg: ServerConfig, max_ticks: u32, team: &[ChampionId]) -> BotMatch {
    run_bot_match_with(cfg, max_ticks, &BotMatchOptions { team: team.to_vec(), ..Default::default() })
}

fn net_worth(u: &mftr_sim::Unit) -> f32 {
    u.state.progress.gold
        + u.state.progress.items.iter().filter_map(|id| mftr_sim::items::item(*id)).map(|item| item.cost).sum::<f32>()
}

/// A bot match with each team's tier and champions chosen (17 §11's measurements).
pub fn run_bot_match_with(cfg: ServerConfig, max_ticks: u32, opts: &BotMatchOptions) -> BotMatch {
    let mut core = ServerCore::new(ServerConfig { record: true, ..cfg }, 0.0);
    // As a hosting server does at start: what bots know of the map, ready before the match.
    mftr_bots::warm_up(core.game.world.map());
    let team = &opts.team;
    assert!(team.len() <= core.bots.len(), "the selected team needs that many bot seats");
    let seats: Vec<PlayerId> = core.bots.players().collect();
    for (player, &champion) in seats.iter().zip(team) {
        core.game.takeover(*player, champion).expect("bot seat");
    }
    if let Some(tiers) = opts.tiers {
        for player in &seats {
            let team = core
                .game
                .world
                .units()
                .iter()
                .find(|u| u.owner == Some(*player) && u.kind == UnitKind::Champion)
                .map_or(Team::Blue, |u| u.team);
            core.bots.add(*player, tiers[(team == Team::Red) as usize]);
        }
    }
    let (mut kills, mut structures) = (0, 0);
    let mut monsters = [0u32; 2];
    let mut monsters_by: Vec<(UnitId, u32)> = Vec::new();
    let mut near_by: Vec<(UnitId, u32)> = Vec::new();
    let mut dealt: Vec<(UnitId, f32, f32)> = Vec::new();
    let mut taken_other: Vec<(UnitId, f32, f32)> = Vec::new();
    let mut dead: Vec<(UnitId, u32)> = Vec::new();
    let mut worth: Vec<(UnitId, [f32; 3])> = Vec::new();
    let mut falls = Vec::new();
    let mut horde_waves = Vec::new();
    let mut cost = BotCost::default();
    let mut winner = None;
    for _ in 0..max_ticks {
        let k = core.game.world.tick().next();
        let t0 = std::time::Instant::now();
        let due = core.bots.think(&core.game.world, k);
        let t1 = std::time::Instant::now();
        let (events, fog) = core.game.step(due);
        let t2 = std::time::Instant::now();
        core.bots.observe(&core.game.world, &events, &fog.visions);
        core.bots.take_pings();
        let t3 = std::time::Instant::now();
        let bots_us = ((t1 - t0) + (t3 - t2)).as_secs_f64() * 1e6;
        cost.ticks += 1;
        cost.bots_us += bots_us;
        cost.bots_worst_us = cost.bots_worst_us.max(bots_us);
        cost.sim_us += (t2 - t1).as_secs_f64() * 1e6;
        for e in &events {
            if let SimEvent::Damage { source, target, amount, absorbed, .. } = e
                && let (Some(s), Some(t)) = (core.game.world.unit(*source), core.game.world.unit(*target))
                && s.kind == UnitKind::Champion
                && t.kind == UnitKind::Champion
                && s.team != t.team
            {
                let d = amount + absorbed;
                match dealt.iter_mut().find(|(id, ..)| *id == s.id) {
                    Some(e) => e.1 += d,
                    None => dealt.push((s.id, d, 0.0)),
                }
                match dealt.iter_mut().find(|(id, ..)| *id == t.id) {
                    Some(e) => e.2 += d,
                    None => dealt.push((t.id, 0.0, d)),
                }
            }
            if let SimEvent::Damage { source, target, amount, absorbed, .. } = e
                && let (Some(s), Some(t)) = (core.game.world.unit(*source), core.game.world.unit(*target))
                && t.kind == UnitKind::Champion
                && matches!(s.kind, UnitKind::Minion | UnitKind::Turret)
            {
                let (m, st) =
                    if s.kind == UnitKind::Minion { (amount + absorbed, 0.0) } else { (0.0, amount + absorbed) };
                match taken_other.iter_mut().find(|(id, ..)| *id == t.id) {
                    Some(e) => {
                        e.1 += m;
                        e.2 += st;
                    }
                    None => taken_other.push((t.id, m, st)),
                }
            }
            if let SimEvent::Died { unit, killer, .. } = e
                && let Some(u) = core.game.world.unit(*unit)
            {
                match u.kind {
                    mftr_sim::UnitKind::Champion => kills += 1,
                    mftr_sim::UnitKind::Monster => {
                        if let Some(k) = core.game.world.unit(*killer).filter(|k| k.team != Team::Neutral) {
                            monsters[k.team as usize] += 1;
                            match monsters_by.iter_mut().find(|(id, _)| *id == k.id) {
                                Some(e) => e.1 += 1,
                                None => monsters_by.push((k.id, 1)),
                            }
                        }
                    }
                    kind if kind.is_structure() => {
                        structures += 1;
                        falls.push((k, u.team, kind, u.tier));
                        if opts.trace_falls {
                            let s = k.0 / mftr_sim::TICK_HZ;
                            eprintln!("{:>2}:{:02} {:?} {:?} tier {} falls", s / 60, s % 60, u.team, kind, u.tier);
                            for c in core.game.world.units().iter().filter(|c| c.kind == UnitKind::Champion) {
                                eprintln!(
                                    "      {:?} {:<8} {:>5.0} u away, hp {:>4.0}/{:<4.0} {}  {}",
                                    c.team,
                                    format!("{:?}", c.champion.unwrap_or(ChampionId::Ember)),
                                    c.state.pos.distance(u.state.pos),
                                    c.state.health,
                                    c.stats.max_health,
                                    if c.state.alive() { "" } else { "DEAD" },
                                    c.owner.and_then(|p| core.bots.describe(p)).unwrap_or_default()
                                );
                            }
                        }
                    }
                    mftr_sim::UnitKind::Minion => {
                        for c in core.game.world.units().iter().filter(|c| {
                            c.kind == UnitKind::Champion
                                && c.team != u.team
                                && c.state.alive()
                                && c.state.pos.distance(u.state.pos) < 900.0
                        }) {
                            match near_by.iter_mut().find(|(id, _)| *id == c.id) {
                                Some(e) => e.1 += 1,
                                None => near_by.push((c.id, 1)),
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        let secs = k.0 / mftr_sim::TICK_HZ;
        if opts.trace_falls && k.0.is_multiple_of(30 * mftr_sim::TICK_HZ) {
            let w = &core.game.world;
            let line = |team: Team| {
                let champs: Vec<_> =
                    w.units().iter().filter(|u| u.kind == UnitKind::Champion && u.team == team).collect();
                let alive = champs.iter().filter(|u| u.state.alive()).count();
                let hp: f32 =
                    champs.iter().filter(|u| u.state.alive()).map(|u| u.state.health / u.stats.max_health).sum::<f32>()
                        / alive.max(1) as f32;
                let gold: f32 = champs.iter().map(|u| u.state.progress.gold).sum::<f32>() / champs.len().max(1) as f32;
                let structures: f32 = w
                    .units()
                    .iter()
                    .filter(|u| u.team == team && u.kind.is_structure() && u.kind != UnitKind::Relic && u.state.alive())
                    .map(|u| u.state.health / u.stats.max_health)
                    .sum();
                let minions = w
                    .units()
                    .iter()
                    .filter(|u| u.team == team && u.kind == UnitKind::Minion && u.state.alive())
                    .count();
                format!(
                    "{alive} alive at {:>3.0}% hp, gold {:>5.0}, structures {structures:.2}, minions {minions:>2}",
                    hp * 100.0,
                    gold
                )
            };
            eprintln!("{:>2}:{:02} blue: {}  |  red: {}", secs / 60, secs % 60, line(Team::Blue), line(Team::Red));
        }
        if let Some(p) = opts.trace
            && k.0.is_multiple_of(mftr_sim::TICK_HZ)
            && let Some(u) = core.game.world.units().iter().find(|u| u.owner == Some(p) && u.kind == UnitKind::Champion)
        {
            let st = &u.state;
            eprintln!(
                "{:>2}:{:02} {:?} ({:.0},{:.0}) hp {:.0}/{:.0} gold {:.0} cs {} lvl {} order {:?}  {}",
                secs / 60,
                secs % 60,
                u.champion,
                st.pos.x,
                st.pos.y,
                st.health,
                u.stats.max_health,
                st.progress.gold,
                st.progress.cs,
                st.progress.level,
                st.order,
                core.bots.describe(p).unwrap_or_default()
            );
        }
        for u in core.game.world.units().iter().filter(|u| u.kind == UnitKind::Champion) {
            if !u.state.alive() {
                match dead.iter_mut().find(|(id, _)| *id == u.id) {
                    Some(e) => e.1 += 1,
                    None => dead.push((u.id, 1)),
                }
            }
            let i = match worth.iter().position(|(id, _)| *id == u.id) {
                Some(i) => i,
                None => {
                    worth.push((u.id, [0.0; 3]));
                    worth.len() - 1
                }
            };
            let w = &mut worth[i].1;
            if secs <= 600 {
                w[0] = net_worth(u);
            }
            if secs <= 900 {
                w[1] = net_worth(u);
            }
            w[2] = net_worth(u);
        }
        if let Some(h) = &core.game.world.game().horde
            && h.cleared > horde_waves.last().map_or(0, |r: &HordeWaveReport| r.wave)
        {
            let champions: Vec<_> = core.game.world.units().iter().filter(|u| u.kind == UnitKind::Champion).collect();
            let base_health = core
                .game
                .world
                .units()
                .iter()
                .find(|u| u.kind == UnitKind::Base && u.team == Team::Blue)
                .map_or(0.0, |u| u.state.health);
            horde_waves.push(HordeWaveReport {
                wave: h.cleared,
                tick: k,
                levels: champions.iter().map(|u| u.state.progress.level).collect(),
                build_gold: champions.iter().map(|u| net_worth(u)).collect(),
                base_health,
            });
        }
        if let Some((team, at)) = core.game.world.game().winner {
            winner = Some((team, Tick((at.0 / mftr_sim::SUBTICKS as u64) as u32)));
            break;
        }
    }
    let stats = core.bots.stats();
    let champions = core
        .game
        .world
        .units()
        .iter()
        .filter(|u| u.kind == UnitKind::Champion)
        .filter_map(|u| {
            let player = u.owner?;
            let p = &u.state.progress;
            Some(ChampionReport {
                player,
                team: u.team,
                champion: u.champion?,
                tier: core.bots.tier(player),
                kills: p.kills,
                deaths: p.deaths,
                assists: p.assists,
                cs: p.cs,
                worth: worth.iter().find(|(id, _)| *id == u.id).map_or([0.0; 3], |(_, w)| *w),
                dead_secs: dead.iter().find(|(id, _)| *id == u.id).map_or(0.0, |(_, n)| *n as f32 / 30.0),
                monsters: monsters_by.iter().find(|(id, _)| *id == u.id).map_or(0, |(_, n)| *n),
                minions_near: near_by.iter().find(|(id, _)| *id == u.id).map_or(0, |(_, n)| *n),
                damage_dealt: dealt.iter().find(|(id, ..)| *id == u.id).map_or(0.0, |d| d.1),
                damage_taken: dealt.iter().find(|(id, ..)| *id == u.id).map_or(0.0, |d| d.2),
                taken_minions: taken_other.iter().find(|(id, ..)| *id == u.id).map_or(0.0, |d| d.1),
                taken_structures: taken_other.iter().find(|(id, ..)| *id == u.id).map_or(0.0, |d| d.2),
                stats: stats.iter().find(|(q, _)| *q == player).map(|(_, s)| *s),
            })
        })
        .collect();
    BotMatch {
        winner,
        ticks: core.game.world.tick().0,
        champion_kills: kills,
        structures_destroyed: structures,
        monsters_killed: monsters,
        falls,
        replay: core.game.replay(),
        horde_waves,
        champions,
        cost,
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
                queue: None,
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
        ClientMessage::Hello {
            protocol: PROTOCOL_VERSION,
            client_time_us: 0,
            champion: None,
            resume,
            spectate,
            queue: None,
        }
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

    /// 17 §8: a ping goes to the pinger's team (and spectators), not the enemy's, at most one
    /// every half second.
    #[test]
    fn pings_reach_the_team_and_are_rate_limited() {
        let cfg = ServerConfig { scenario: Scenario::Aram, max_players: 10, ..Default::default() };
        let mut core = ServerCore::new(cfg, 0.0);
        for key in [1, 2, 3] {
            hello(&mut core, key);
        }
        let (t1, t2, t3) = (core.team_of(1).unwrap(), core.team_of(2).unwrap(), core.team_of(3).unwrap());
        assert!(t1 == t3 && t1 != t2, "1 and 3 play together, 2 against them");
        let ping = |kind| ClientMessage::Ping { kind, at: Vec2::new(5000.0, 5000.0), target: None };
        send(&mut core, 1, ping(mftr_sim::ping::PingKind::OnMyWay), 0.1);
        send(&mut core, 1, ping(mftr_sim::ping::PingKind::Danger), 0.2); // too soon: dropped
        send(&mut core, 1, ping(mftr_sim::ping::PingKind::Group), 0.8);
        let mut got: BTreeMap<ClientKey, Vec<mftr_sim::ping::PingKind>> = BTreeMap::new();
        for (to, b) in core.step(1.0) {
            if let Ok((_, ServerMessage::Snapshot(s))) = decode_server(&b) {
                for (_, e) in s.events {
                    if let SimEvent::Ping(p) = e {
                        got.entry(to).or_default().push(p.kind);
                        assert_eq!(p.from, core.conns.get(&1).map(|c| c.unit), "the pinger's champion");
                    }
                }
            }
        }
        use mftr_sim::ping::PingKind::{Group, OnMyWay};
        assert_eq!(got.get(&1), Some(&vec![OnMyWay, Group]), "the pinger sees its own");
        assert_eq!(got.get(&3), Some(&vec![OnMyWay, Group]), "a teammate");
        assert_eq!(got.get(&2), None, "never the enemy");
    }

    /// 17 §9: co-op champion select votes how well the bots play; the most votes wins.
    #[test]
    fn coop_champion_select_votes_the_bots_tier() {
        let cfg =
            ServerConfig { seed: 4, bots: 10, lobby: true, coop: true, scenario: Scenario::Aram, ..Default::default() };
        let mut core = ServerCore::new(cfg, 0.0);
        let state = |msgs: &[ServerMessage]| {
            msgs.iter()
                .find_map(|m| match m {
                    ServerMessage::Lobby(l) => Some((**l).clone()),
                    _ => None,
                })
                .expect("a lobby state")
        };
        let l = state(&send(&mut core, 1, hello_msg(0, false), 0.0));
        let bots = l.bots.expect("co-op shows the vote");
        assert_eq!((bots.tier, bots.mine), (Tier::Veteran.wire(), None), "the server's default to start");
        let l = state(&send(&mut core, 1, ClientMessage::Lobby(msg::LobbyAction::Bots(3)), 0.5));
        assert_eq!(l.bots.unwrap().tier, Tier::Nightmare.wire());
        assert_eq!(l.bots.unwrap().mine, Some(3));
        send(&mut core, 1, ClientMessage::Lobby(msg::LobbyAction::Ready(true)), 1.0);
        let mut t = 1.0;
        while core.lobby.is_some() && t < 10.0 {
            t += 1.0 / 30.0;
            core.step(t);
        }
        let bot = core.bots.players().next().expect("bots play");
        assert_eq!(core.bots.tier(bot), Some(Tier::Nightmare));
        // Versus has no vote.
        let mut versus = ServerCore::new(
            ServerConfig { lobby: true, bots: 10, scenario: Scenario::Aram, ..Default::default() },
            0.0,
        );
        assert!(state(&send(&mut versus, 1, hello_msg(0, false), 0.0)).bots.is_none());
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

    /// 13 §4: in a match, a bot plays a dropped player's champion (and its commands are
    /// recorded like anyone's) until the player returns with their token, from any address;
    /// the match counts as abandoned only once its last human has been gone a while. A player
    /// who says goodbye leaves their champion to a bot as well.
    #[test]
    fn a_bot_plays_a_dropped_champion_until_its_player_returns() {
        let cfg = ServerConfig { seed: 3, bots: 9, record: true, scenario: Scenario::Aram, ..Default::default() };
        let mut core = ServerCore::new(cfg, 0.0);
        let (unit, token, _) = welcome_of(&send(&mut core, 1, hello_msg(0, false), 0.0)).unwrap();
        let player = core.world().unit(unit).unwrap().owner.unwrap();
        assert_eq!(core.bot_count(), 9);
        let mut t = 0.0;
        while t < 12.0 {
            t += 1.0 / 30.0;
            core.step(t);
        }
        assert_eq!(core.bot_count(), 10, "a bot took over");
        assert_eq!((core.connected_humans(), core.humans()), (0, 1), "the seat is held");
        assert!(core.holds_token(token));
        let dropped_at = core.world().tick();
        while t < 40.0 {
            t += 1.0 / 30.0;
            core.step(t);
        }
        assert!(!core.abandoned(t), "within the grace period");
        assert!(core.abandoned(t + RECONNECT_GRACE), "nobody came back");
        let bot_orders = core.game().replay().entries.iter().any(|e| match e {
            ReplayEntry::Commands { tick, commands } => {
                *tick > dropped_at && commands.iter().any(|c| c.player == player)
            }
            _ => false,
        });
        assert!(bot_orders, "the bot plays the champion");

        let (back, same, _) = welcome_of(&send(&mut core, 7, hello_msg(token, false), t)).unwrap();
        assert_eq!((back, same), (unit, token), "the same champion, from a new address");
        assert_eq!(core.bot_count(), 9, "the bot handed it back");
        assert_eq!(core.humans(), 1);

        let bye = encode_client(&PacketHeader::default(), &ClientMessage::Bye);
        core.handle_packet(7, &bye, t);
        assert_eq!(core.bot_count(), 10, "a bot plays on for the leaver");
        assert!(core.world().unit(unit).is_some());
        assert_eq!(core.humans(), 0, "a goodbye holds no seat");
        assert!(core.abandoned(t));
    }

    /// Play `core` until `until`, keeping connections `keys` alive.
    fn run_until(core: &mut ServerCore, keys: &[ClientKey], t: &mut f64, until: f64) {
        while *t < until {
            *t += 1.0 / 30.0;
            if ((*t * 30.0) as u32).is_multiple_of(30) {
                for key in keys {
                    send(core, *key, hello_msg(0, false), *t);
                }
            }
            core.step(*t);
        }
    }

    fn lobby_of(msgs: &[ServerMessage]) -> Option<msg::LobbyState> {
        msgs.iter().find_map(|m| if let ServerMessage::Lobby(l) = m { Some((**l).clone()) } else { None })
    }

    /// 13 §3: a player joining an ARAM match in progress gets a champion select of their own
    /// (a dealt champion, one reroll), then takes over a bot on the team with fewer humans:
    /// in the fountain, at the bot's level with every point unspent, with its gold plus the
    /// price of its items. The match goes on meanwhile, and its replay re-simulates exactly.
    #[test]
    fn a_player_joins_a_match_in_progress_by_taking_over_a_bot() {
        let cfg = ServerConfig {
            seed: 5,
            bots: 10,
            lobby: true,
            record: true,
            scenario: Scenario::Aram,
            ..Default::default()
        };
        let mut core = ServerCore::new(cfg, 0.0);
        let first = lobby_of(&send(&mut core, 1, hello_msg(0, false), 0.0)).unwrap();
        assert!(!first.midgame && !first.pick && first.mode == msg::GameMode::Aram && first.map == "The Bridge");
        send(&mut core, 1, ClientMessage::Lobby(msg::LobbyAction::Ready(true)), 0.0);
        let mut t = 0.0;
        run_until(&mut core, &[1], &mut t, 120.0);
        assert!(!core.in_champion_select() && core.has_room());

        let l = lobby_of(&send(&mut core, 2, hello_msg(0, false), t)).unwrap();
        assert!(l.midgame);
        let me = *l.slots.iter().find(|s| s.player == l.you).unwrap();
        assert_eq!(me.team, Team::Red, "the team without a human");
        assert_eq!(me.rerolls, MIDJOIN_REROLLS);
        let seat = l.you;
        let old = core.world().units().iter().find(|u| u.owner == Some(seat)).unwrap().clone();
        let was = old.state.progress;
        assert!(was.items.iter().any(|i| *i != 0), "the bot bought something");

        let l = lobby_of(&send(&mut core, 2, ClientMessage::Lobby(msg::LobbyAction::Reroll), t)).unwrap();
        let mine = l.slots.iter().find(|s| s.player == seat).unwrap();
        assert!(mine.champion != me.champion && mine.rerolls == 0);
        let teammates: Vec<ChampionId> =
            l.slots.iter().filter(|s| s.team == Team::Red && s.player != seat).map(|s| s.champion).collect();
        assert!(!teammates.contains(&mine.champion), "not one the team has");
        send(&mut core, 2, ClientMessage::Lobby(msg::LobbyAction::Ready(true)), t);
        let gold = core.world().units().iter().find(|u| u.owner == Some(seat)).unwrap().state.progress.gold;
        t += 1.0 / 30.0;
        let welcome = core.step(t).into_iter().filter(|(to, _)| *to == 2).map(|(_, b)| decode_server(&b).unwrap().1);
        let welcome: Vec<ServerMessage> = welcome.collect();
        let (unit, token, _) = welcome_of(&welcome).expect("welcomed into the match");
        assert!(token != 0);
        assert_eq!(core.bot_count(), 8);
        let new = core.world().unit(unit).unwrap();
        assert_eq!((new.owner, new.champion), (Some(seat), Some(mine.champion)));
        assert!(core.world().unit(old.id).is_none(), "the bot's champion left");
        let p = new.state.progress;
        assert_eq!((p.level, p.xp, p.points, p.ranks), (was.level, was.xp, was.level, [0; 4]));
        assert!(p.items.iter().all(|i| *i == 0));
        let refund: f32 = was.items.iter().filter_map(|i| mftr_sim::items::item(*i)).map(|i| i.cost).sum();
        assert!((p.gold - (gold + refund)).abs() < 50.0, "{} vs {} + {refund}", p.gold, gold);
        let fountain = core.world().map().layout.champion_spawn[Team::Red as usize];
        assert!(new.state.pos.distance(fountain) < 400.0);

        run_until(&mut core, &[1], &mut t, 140.0);
        let replay = Replay::from_text(&core.game().replay().to_text()).unwrap();
        assert!(replay.entries.iter().any(|e| matches!(e, ReplayEntry::Takeover { .. })));
        let check = replay.verify();
        assert_eq!(check.mismatch, None);
        assert_eq!(check.final_hash, core.world().state_hash());
    }

    /// 13 §3: in Classic, a joiner picks any champion their team doesn't have.
    #[test]
    fn classic_joiners_pick() {
        let cfg = ServerConfig { seed: 2, bots: 10, lobby: true, scenario: Scenario::Classic, ..Default::default() };
        let mut core = ServerCore::new(cfg, 0.0);
        let first = lobby_of(&send(&mut core, 1, hello_msg(0, false), 0.0)).unwrap();
        assert!(first.pick && first.map == "Crossroads");
        send(&mut core, 1, ClientMessage::Lobby(msg::LobbyAction::Ready(true)), 0.0);
        let mut t = 0.0;
        run_until(&mut core, &[1], &mut t, 5.0);
        let l = lobby_of(&send(&mut core, 2, hello_msg(0, false), t)).unwrap();
        assert!(l.midgame && l.pick);
        let team = l.slots.iter().find(|s| s.player == l.you).unwrap().team;
        let mate = l.slots.iter().find(|s| s.team == team && s.player != l.you).unwrap().champion;
        let l2 = lobby_of(&send(&mut core, 2, ClientMessage::Lobby(msg::LobbyAction::Take(mate)), t)).unwrap();
        assert_ne!(l2.slots.iter().find(|s| s.player == l.you).unwrap().champion, mate, "a teammate has it");
        let free =
            ChampionId::ALL.into_iter().find(|c| !l.slots.iter().any(|s| s.team == team && s.champion == *c)).unwrap();
        let l3 = lobby_of(&send(&mut core, 2, ClientMessage::Lobby(msg::LobbyAction::Take(free)), t)).unwrap();
        assert_eq!(l3.slots.iter().find(|s| s.player == l.you).unwrap().champion, free);
        // Out of time, joining anyway.
        run_until(&mut core, &[1, 2], &mut t, 5.0 + MIDJOIN_SECONDS + 1.0);
        assert!(core.world().units().iter().any(|u| u.owner == Some(l.you) && u.champion == Some(free)));
        assert_eq!(core.connected_humans(), 2);
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
                queue: None,
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
        // Every movement order between ticks 3000 and 3600 is sent elsewhere.
        let far = mftr_sim::map::bridge_point(Vec2::new(9000.0, 1000.0));
        for e in tampered.entries.iter_mut() {
            if let ReplayEntry::Commands { commands, tick } = e
                && (3000..3600).contains(&tick.0)
            {
                for c in commands.iter_mut() {
                    if matches!(c.kind, mftr_sim::CommandKind::MoveTo(_) | mftr_sim::CommandKind::AttackMove(_)) {
                        c.kind = mftr_sim::CommandKind::MoveTo(QPoint::from_vec2(far));
                    }
                }
            }
        }
        assert!(tampered.verify().mismatch.is_some());
    }

    /// ARAM has no fountain healing after leaving base, so a bot low on health must not sit
    /// out the fight waiting for health that never comes (behind a turret or in the fountain):
    /// it takes a relic or keeps fighting from behind its wave. Measured as the longest time
    /// any bot spends alive, below half health, with no enemy unit within 1,500 u.
    #[test]
    fn aram_bots_low_on_health_do_not_sit_out_the_fight() {
        let cfg = ServerConfig { seed: 11, bots: 10, scenario: Scenario::Aram, ..Default::default() };
        let mut core = ServerCore::new(cfg, 0.0);
        let mut streak: std::collections::HashMap<mftr_sim::UnitId, u32> = Default::default();
        let mut worst = (0u32, mftr_sim::UnitId(0));
        for _ in 0..(10 * 60 * 30) {
            let k = core.game.world.tick().next();
            let due = core.bots.think(&core.game.world, k);
            let (events, fog) = core.game.step(due);
            core.bots.observe(&core.game.world, &events, &fog.visions);
            if core.game.world.game().winner.is_some() {
                break;
            }
            if !k.0.is_multiple_of(30) {
                continue;
            }
            let units = core.game.world.units();
            for c in units.iter().filter(|u| u.kind == mftr_sim::UnitKind::Champion) {
                let out = c.state.alive()
                    && c.state.health < 0.5 * c.stats.max_health
                    && !units.iter().any(|e| {
                        e.team != c.team
                            && e.state.alive()
                            && matches!(e.kind, mftr_sim::UnitKind::Champion | mftr_sim::UnitKind::Minion)
                            && e.state.pos.distance(c.state.pos) <= 1500.0
                    });
                let n = streak.entry(c.id).or_default();
                *n = if out { *n + 1 } else { 0 };
                if *n > worst.0 {
                    worst = (*n, c.id);
                }
            }
        }
        assert!(worst.0 <= 40, "a bot sat out the fight low on health for {} s ({:?})", worst.0, worst.1);
    }

    /// A match that ends the real way (bots push until a Base falls, with a connected player
    /// who idles) starts again: the server keeps serving through the restart, with and without
    /// champion select.
    #[test]
    fn a_match_that_ends_naturally_starts_again_with_a_player_connected() {
        for lobby in [false, true] {
            let bots = if lobby { 10 } else { 9 };
            let cfg = ServerConfig { seed: 6, bots, lobby, scenario: Scenario::Aram, ..Default::default() };
            let mut core = ServerCore::new(cfg, 0.0);
            let mut seq = 0u16;
            let mut packet = |core: &mut ServerCore, msg: ClientMessage, t: f64| {
                seq += 1;
                let bytes = encode_client(&PacketHeader { seq, ack: 0, ack_bits: 0 }, &msg);
                core.handle_packet(1, &bytes, t);
            };
            let idle = ClientMessage::Input { client_time_us: 0, event_ack: 0, snapshot_ack: 0, commands: Vec::new() };
            packet(&mut core, hello_msg(0, false), 0.0);
            if lobby {
                packet(&mut core, ClientMessage::Lobby(msg::LobbyAction::Ready(true)), 0.0);
            }
            let mut t = 0.0;
            let (mut ended, mut after) = (None, Vec::new());
            for _ in 0..(60 * 60 * 30) {
                t += 1.0 / 30.0;
                if ((t * 30.0) as u32).is_multiple_of(15) {
                    packet(&mut core, idle.clone(), t);
                }
                let out = core.step(t);
                if ended.is_none() && core.world().game().winner.is_some() {
                    ended = Some(t);
                }
                if let Some(e) = ended {
                    after.extend(out.into_iter().filter(|(to, _)| *to == 1).map(|(_, b)| decode_server(&b).unwrap().1));
                    if t > e + 30.0 {
                        break;
                    }
                }
            }
            assert!(ended.is_some(), "a Base falls");
            let lobbies = after.iter().filter(|m| matches!(m, ServerMessage::Lobby(_))).count();
            let snapshots = after.iter().filter(|m| matches!(m, ServerMessage::Snapshot(_))).count();
            if lobby {
                assert!(lobbies > 0, "champion select again");
            } else {
                assert!(core.world().game().winner.is_none(), "a new match");
                assert!(snapshots > 500, "still served: {snapshots}");
            }
        }
    }

    /// Joins and leaves at any point (before, between and after commands) replay exactly.
    #[test]
    fn replays_handle_joins_and_leaves() {
        let mut m = Match::new(ServerConfig { scenario: Scenario::Duel, record: true, ..Default::default() });
        let mut bots = Bots::new(7);
        for k in 1..=900u32 {
            if k == 1 || k == 200 || k == 450 {
                let p = m.free_player().unwrap();
                m.join(p, None);
                bots.add(p, Tier::Veteran);
            }
            if k == 600 {
                let gone = bots.players().next().unwrap();
                bots.remove(gone);
                m.leave(gone);
            }
            let due = bots.think(m.world(), Tick(k));
            let (events, fog) = m.step(due);
            bots.observe(m.world(), &events, &fog.visions);
            if k == 700 {
                let p = m.free_player().unwrap();
                m.join(p, Some(ChampionId::Shade));
                bots.add(p, Tier::Elite);
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
        let mut bots = Bots::new(3);
        for p in 0..4u8 {
            bots.add(PlayerId(p), Tier::Veteran);
            m.join(PlayerId(p), None);
        }
        for k in 1..=700u32 {
            if k == 400 {
                for p in m.players() {
                    m.leave(p);
                }
                m.restart();
                bots = Bots::new(5);
                for i in 0..4u8 {
                    bots.add(PlayerId(i), Tier::Recruit);
                    m.join(PlayerId(i), Some(ChampionId::ALL[5 - i as usize]));
                }
            }
            let due = bots.think(m.world(), Tick(k));
            let (events, fog) = m.step(due);
            bots.observe(m.world(), &events, &fog.visions);
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
