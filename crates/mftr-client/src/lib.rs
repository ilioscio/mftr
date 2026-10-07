//! `mftr-client`: the engine-independent client runtime.
//!
//! Implements the own-champion prediction loop of `docs/design/03a-netcode-time-and-prediction.md`:
//! - commands are stamped on the **input timeline** (`T_input`) with sub-tick precision,
//! - the own champion is predicted with the same `mftr-sim` code the server runs,
//! - authoritative snapshots are compared bit-exactly and mismatches are re-simulated,
//! - the input margin is steered by the server's arrival-lead reports,
//! - nearby units become **collision proxies**, extrapolated to the input timeline, so minion
//!   block is predicted (03a §5),
//! - remote units are interpolated on `T_interp`; minions near the own champion are drawn
//!   blended toward `T_input` (the "minion bubble"),
//! - combat (slice 4): own casts, attacks, dashes, blinks and shields are predicted; damage,
//!   CC and deaths come from the server and are reconciled like any other state change.
//!
//! Time is passed in explicitly (`now`, local seconds), so the same code runs under the Godot
//! client, the UDP bot, and the virtual-time Netcode Lab.

use mftr_net::PROTOCOL_VERSION;
use mftr_net::clock::ClockSync;
use mftr_net::delta;
use mftr_net::msg::{self, ClientMessage, CommandReport, RemoteUnit, ServerMessage, Snapshot};
use mftr_net::packet::{PacketHeader, ReceiveTracker, SendTracker, seq_greater};
use mftr_sim::ability::DamageKind;
use mftr_sim::time::tick_at;
use mftr_sim::{
    ChampionId, Command, CommandKind, Order, PlayerId, QPoint, SUBTICKS, SimEvent, SimTime, TICK_DT_F64, Team, Tick,
    Unit, UnitId, UnitKind, UnitState, Vec2, World,
};
use std::collections::{BTreeMap, VecDeque};

pub mod blind;
pub mod effects;
pub mod missiles;
use effects::EffectBook;
pub use effects::{AreaRender, BoltRender};
use missiles::MissileBook;
pub use missiles::{DodgeStats, MissileRender, OwnMissileDisplay, Side, Threat};

/// Units farther than this from the own champion can't block it within a prediction horizon.
const PROXY_RANGE: f32 = 600.0;
/// Attack targets and attack-move candidates: every visible unit this close is a proxy, so
/// chasing and target acquisition are predicted too.
const TARGET_PROXY_RANGE: f32 = 1500.0;
/// Reconstructed snapshots kept as possible delta baselines (~2 s).
const RECONSTRUCTED_RING: usize = 64;
/// Interpolation never smears a jump this large in one snapshot interval (a blink): it steps.
const TELEPORT_DISTANCE: f32 = 150.0;
/// Minion bubble: fully on `T_input` inside the inner radius, fully on `T_interp` outside the outer.
const BUBBLE_INNER: f32 = 400.0;
const BUBBLE_OUTER: f32 = 900.0;

/// What we know about another unit: interpolation samples plus its latest replicated state.
struct RemoteTrack {
    samples: VecDeque<(Tick, Vec2)>,
    latest: RemoteUnit,
    latest_tick: Tick,
}

impl RemoteTrack {
    /// Position at `at` (fractional ticks), extrapolated from the latest snapshot along the
    /// replicated heading at constant speed, stopping at the waypoint. Never before the snapshot.
    /// Best estimate of the unit's position at `at` (fractional ticks): interpolated from
    /// snapshots where we have them, extrapolated beyond the newest.
    fn pos_at(&self, at: f64) -> Vec2 {
        if at <= self.latest_tick.0 as f64 {
            interpolate(&self.samples, at).unwrap_or(self.latest.pos.to_vec2())
        } else {
            self.extrapolate(at)
        }
    }

    /// Gameplay (hitbox) radius (01 §4).
    fn gameplay_radius(&self) -> f32 {
        self.latest.gameplay_radius as f32
    }

    /// Can be hit by skillshots, as the server decides it (no structures, relics or rigs).
    fn takes_skillshots(&self) -> bool {
        !self.latest.kind.is_structure()
    }

    fn extrapolate(&self, at: f64) -> Vec2 {
        let p = self.latest.pos.to_vec2();
        let Some(target) = self.latest.target.map(QPoint::to_vec2) else { return p };
        let elapsed = (at - self.latest_tick.0 as f64).max(0.0);
        let dist = (self.latest.speed as f64 * elapsed * TICK_DT_F64) as f32;
        let to = target - p;
        let len = to.length();
        if len <= 0.0 || dist >= len { target } else { p + to * (dist / len) }
    }

    fn proxy(&self, at: f64, collide: bool) -> Unit {
        // Static during the predicted tick: the server resolves movers (and judges attack range)
        // against start-of-tick positions, which is exactly what this is.
        let pos = self.extrapolate(at);
        let collision = if collide { self.latest.collision_radius as f32 } else { 0.0 };
        let stats = mftr_sim::champion::Stats { max_health: 1.0, ..mftr_sim::champion::Stats::NONE };
        let radii = (collision, self.gameplay_radius());
        let mut u = Unit::new(self.latest.id, self.latest.kind, self.latest.team, pos, radii, stats);
        u.protected = self.latest.protected;
        u
    }
}

/// A remote unit as the client should draw it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemoteRender {
    pub id: UnitId,
    pub kind: UnitKind,
    pub team: Team,
    pub collision_radius: f32,
    pub pos: Vec2,
    /// Cast windup in progress: `(progress 0..1, aim direction)`. Enemy windups are shown on
    /// `T_input` so they line up with their missiles (03a §7).
    pub windup: Option<(f32, Vec2)>,
    pub champion: Option<ChampionId>,
    /// Held augments (indicators above the health bar).
    pub augments: [u8; mftr_sim::augments::SLOTS],
    pub gameplay_radius: f32,
    /// Confirmed health (03a §7: never predicted for others).
    pub health: f32,
    pub max_health: f32,
    pub shield: f32,
    pub level: u8,
    pub attacking: bool,
    pub stunned: bool,
    pub rooted: bool,
    pub dashing: bool,
    pub slowed: bool,
    /// A structure that can't be hurt yet.
    pub protected: bool,
    /// Facing in radians, counter-clockwise from +x (10 bits on the wire, 10 §3).
    pub facing: f32,
    /// Basic attacks started, modulo 4: which attack animation to play.
    pub attack_variant: u8,
    /// In an ability's follow-through (10 §4.1).
    pub recovering: bool,
}

/// A confirmed damage instance, for floating numbers and the combat log.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombatText {
    pub target: UnitId,
    pub source: UnitId,
    pub kind: DamageKind,
    pub amount: f32,
    pub absorbed: f32,
    /// A heal (relic) rather than damage.
    pub heal: bool,
}

/// Kills and respawns of units this client knows about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Notice {
    Died {
        unit: UnitId,
        killer: UnitId,
    },
    Respawned {
        unit: UnitId,
    },
    /// A blink: own ones from prediction (at once), others' when the server reports them.
    Blinked {
        unit: UnitId,
        from: Vec2,
        to: Vec2,
    },
    /// A Base fell (a new match starts shortly).
    MatchEnded {
        winner: Team,
    },
    /// Gold and experience the own champion earned.
    Reward {
        gold: f32,
        xp: u32,
    },
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

const HISTORY_TICKS: usize = 128;
/// Target for the 1st-percentile arrival lead (03a §10.2).
const TARGET_MIN_LEAD: f64 = 0.002;
const LEAD_WINDOW: usize = 128;
/// Margin changes are applied by time dilation: the input timeline runs at most 10% fast
/// while growing the margin (after late commands) and 1% slow while shrinking it.
const MARGIN_GROW_PER_S: f64 = 0.10;
const MARGIN_SHRINK_PER_S: f64 = 0.01;
/// Regular send interval, and the fast re-send cadence for young unacknowledged commands
/// (so a single lost packet doesn't delay a click by a whole send interval).
const SEND_INTERVAL: f64 = 1.0 / 30.0;
const RESEND_INTERVAL: f64 = 0.010;
const RESEND_WINDOW: f64 = 0.150;
const MARGIN_MIN: f64 = 0.002;
const MARGIN_MAX: f64 = 0.250;
/// Visual correction smoothing half-life (03a §4.2).
const CORRECTION_HALF_LIFE: f64 = 0.050;
const CORRECTION_ABSORB: f32 = 2.0;
const CORRECTION_SNAP: f32 = 100.0;

#[derive(Clone, Debug, Default)]
pub struct ClientStats {
    /// Magnitude (u) of every server-driven change to the displayed own position: state
    /// mismatches and late-applied commands (before visual smoothing).
    pub corrections: Vec<f32>,
    pub reconciliations: u64,
    /// Snapshots whose own state differed bit-wise from the prediction.
    pub mismatches: u64,
    pub commands_issued: u64,
    pub commands_late: u64,
    pub leads_us: Vec<i32>,
    pub bytes_up: u64,
    pub bytes_down: u64,
    pub packets_up: u64,
    pub packets_down: u64,
    pub decode_errors: u64,
    pub hard_resets: u64,
    /// Combat: damage dealt by / taken by the own champion, kills and deaths (confirmed).
    pub damage_dealt: f64,
    pub damage_taken: f64,
    /// Champion kills (`minion_kills` counts the rest).
    pub kills: u64,
    pub minion_kills: u64,
    pub deaths: u64,
    /// Matches this client's team won and lost (lane maps).
    pub matches_won: u64,
    pub matches_lost: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Connecting,
    /// In champion select (keep sending Hello as a keepalive until the Welcome).
    Lobby,
    /// Welcomed, waiting for the first snapshot that carries our own unit.
    Joining,
    Playing,
}

pub struct ClientSession {
    phase: Phase,
    player: PlayerId,
    unit: UnitId,
    team: Team,
    champion: ChampionId,
    home: Vec2,
    rules: mftr_sim::world::Rules,
    /// Requested in the hello (the server picks when `None`).
    champion_request: Option<ChampionId>,
    /// Token to resume a dropped session with (0 = none), and the one from our Welcome.
    resume: u64,
    token: u64,
    /// Ask to watch, and whether we are watching.
    spectate: bool,
    spectator: bool,
    lobby: Option<msg::LobbyState>,
    /// What the server runs (from the welcome).
    mode: msg::GameMode,
    /// The welcome's packet sequence: champion select sent after it means the match is over.
    welcome_seq: u16,
    /// The map from the welcome: prediction runs with the same walls and pathing as the server.
    map: std::sync::Arc<mftr_sim::map::Map>,
    /// Predicted world containing only our own unit.
    world: World,
    /// Predicted own state at the end of each tick.
    history: VecDeque<(Tick, UnitState)>,
    /// Recent commands (unconfirmed, or still needed for redundancy).
    commands: VecDeque<Command>,
    next_seq: u32,
    acked_seq: u32,
    seen_reports: BTreeMap<u32, (Tick, mftr_sim::SubTick)>,
    last_snapshot_tick: Option<Tick>,
    last_stamp: f64,
    clock: ClockSync,
    margin: f64,
    margin_target: f64,
    /// Test knob: force the input margin (fault injection in the Netcode Lab).
    margin_override: Option<f64>,
    leads: VecDeque<f64>,
    last_send: Option<f64>,
    last_command_at: Option<f64>,
    unsent_command: bool,
    remote: BTreeMap<UnitId, RemoteTrack>,
    collision_proxies: bool,
    /// Minions near the own champion drawn blended toward `T_input` (03a §5; A/B in M1).
    minion_bubble: bool,
    render_offset: Vec2,
    last_update: Option<f64>,
    send: SendTracker,
    recv: ReceiveTracker,
    pub stats: ClientStats,
    book: MissileBook,
    last_event_seq: u32,
    /// Other units as reconstructed at recent snapshot ticks (delta baselines, 03b §6).
    reconstructed: VecDeque<(Tick, BTreeMap<UnitId, RemoteUnit>)>,
    /// Newest of those, reported to the server in every input packet.
    snapshot_ack: Tick,
    /// Remote cast windups from `CastStarted` events: `(start, fire, aim)`.
    remote_casts: BTreeMap<UnitId, (SimTime, SimTime, Vec2)>,
    effects: EffectBook,
    combat_text: Vec<CombatText>,
    notices: Vec<Notice>,
    /// Latest own blink already announced (re-simulation replays it).
    last_own_blink: SimTime,
}

impl Default for ClientSession {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientSession {
    pub fn new() -> Self {
        Self {
            phase: Phase::Connecting,
            player: PlayerId(0),
            unit: UnitId(0),
            team: Team::Blue,
            champion: ChampionId::Ember,
            home: Vec2::ZERO,
            rules: mftr_sim::world::Rules::SANDBOX,
            champion_request: None,
            resume: 0,
            token: 0,
            spectate: false,
            spectator: false,
            lobby: None,
            mode: msg::GameMode::Empty,
            welcome_seq: 0,
            map: mftr_sim::map::MapId::Open.shared(),
            world: World::from_units(Tick(0), Vec::new()),
            history: VecDeque::new(),
            commands: VecDeque::new(),
            next_seq: 1,
            acked_seq: 0,
            seen_reports: BTreeMap::new(),
            last_snapshot_tick: None,
            last_stamp: 0.0,
            clock: ClockSync::default(),
            margin: 0.030,
            margin_target: 0.030,
            margin_override: None,
            leads: VecDeque::new(),
            last_send: None,
            last_command_at: None,
            unsent_command: false,
            remote: BTreeMap::new(),
            collision_proxies: true,
            minion_bubble: true,
            render_offset: Vec2::ZERO,
            last_update: None,
            send: SendTracker::default(),
            recv: ReceiveTracker::default(),
            stats: ClientStats::default(),
            book: MissileBook::default(),
            last_event_seq: 0,
            remote_casts: BTreeMap::new(),
            reconstructed: VecDeque::new(),
            snapshot_ack: Tick(0),
            effects: EffectBook::default(),
            combat_text: Vec::new(),
            notices: Vec::new(),
            last_own_blink: SimTime(0),
        }
    }

    /// Ask for a champion in the hello (before connecting).
    /// Reconnect: present this token (from an earlier session's Welcome) in the Hello.
    pub fn set_resume(&mut self, token: u64) {
        self.resume = token;
    }

    /// The session token from the Welcome (0 before it): keep it to reconnect.
    pub fn token(&self) -> u64 {
        self.token
    }

    /// Ask to watch instead of play.
    pub fn set_spectate(&mut self, spectate: bool) {
        self.spectate = spectate;
    }

    pub fn is_spectator(&self) -> bool {
        self.spectator
    }

    /// Champion select, while in it.
    pub fn lobby(&self) -> Option<&msg::LobbyState> {
        self.lobby.as_ref()
    }

    /// A champion-select action, as a packet to send.
    pub fn lobby_packet(&mut self, action: msg::LobbyAction) -> Vec<u8> {
        let h = self.header();
        let bytes = msg::encode_client(&h, &ClientMessage::Lobby(action));
        self.finish_packet(bytes)
    }

    pub fn set_champion_request(&mut self, champion: Option<ChampionId>) {
        self.champion_request = champion;
    }

    pub fn champion(&self) -> ChampionId {
        self.champion
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn player(&self) -> PlayerId {
        self.player
    }

    /// The match map (walls, brush) for rendering.
    pub fn map(&self) -> &mftr_sim::map::Map {
        &self.map
    }

    pub fn team(&self) -> Team {
        self.team
    }

    pub fn unit(&self) -> UnitId {
        self.unit
    }

    pub fn rtt(&self) -> f64 {
        self.clock.rtt()
    }

    pub fn margin(&self) -> f64 {
        self.margin
    }

    pub fn interp_buffer(&self) -> f64 {
        (TICK_DT_F64 + 2.0 * self.clock.jitter()).clamp(TICK_DT_F64, 6.0 * TICK_DT_F64)
    }

    // ---- outgoing ---------------------------------------------------------------------

    fn header(&mut self) -> PacketHeader {
        let (ack, ack_bits) = self.recv.ack_fields();
        PacketHeader { seq: self.send.next_seq(), ack, ack_bits }
    }

    fn finish_packet(&mut self, bytes: Vec<u8>) -> Vec<u8> {
        self.stats.bytes_up += bytes.len() as u64;
        self.stats.packets_up += 1;
        bytes
    }

    pub fn hello_packet(&mut self, now: f64) -> Vec<u8> {
        let h = self.header();
        let bytes = msg::encode_client(
            &h,
            &ClientMessage::Hello {
                protocol: PROTOCOL_VERSION,
                client_time_us: time_us(now),
                champion: self.champion_request,
                resume: self.resume,
                spectate: self.spectate,
            },
        );
        self.finish_packet(bytes)
    }

    /// Whether an input packet should go out now: immediately after a new command, every
    /// `SEND_INTERVAL`, and every `RESEND_INTERVAL` while a recent command is unacknowledged.
    pub fn should_send(&self, now: f64) -> bool {
        if self.phase != Phase::Playing {
            return false;
        }
        let since = self.last_send.map_or(f64::INFINITY, |t| now - t);
        let young_unacked = self.last_command_at.is_some_and(|t| now - t < RESEND_WINDOW)
            && self.commands.iter().any(|c| c.seq > self.acked_seq);
        self.unsent_command || since >= SEND_INTERVAL || (young_unacked && since >= RESEND_INTERVAL)
    }

    /// Input packet carrying every command the server hasn't acknowledged yet (redundancy).
    pub fn input_packet(&mut self, now: f64) -> Vec<u8> {
        self.last_send = Some(now);
        self.unsent_command = false;
        let unacked: Vec<Command> = self.commands.iter().filter(|c| c.seq > self.acked_seq).copied().collect();
        let h = self.header();
        let input = ClientMessage::Input {
            client_time_us: time_us(now),
            event_ack: self.last_event_seq,
            snapshot_ack: self.snapshot_ack.0,
            commands: unacked,
        };
        let bytes = msg::encode_client(&h, &input);
        self.finish_packet(bytes)
    }

    pub fn bye_packet(&mut self) -> Vec<u8> {
        let h = self.header();
        let bytes = msg::encode_client(&h, &ClientMessage::Bye);
        self.finish_packet(bytes)
    }

    // ---- timelines --------------------------------------------------------------------

    /// `T_now` in fractional ticks.
    fn now_ticks(&self, now: f64) -> Option<f64> {
        self.clock.server_time(now).map(|t| t / TICK_DT_F64)
    }

    /// `T_input`: the server time at which a command sent now takes effect.
    pub fn input_time(&self, now: f64) -> Option<f64> {
        let margin = self.margin_override.unwrap_or(self.margin);
        self.now_ticks(now).map(|t| t + (self.clock.rtt() / 2.0 + margin) / TICK_DT_F64)
    }

    /// `T_interp`: the server time remote units are drawn at.
    pub fn interp_time(&self, now: f64) -> Option<f64> {
        self.now_ticks(now).map(|t| t - (self.clock.rtt() / 2.0 + self.interp_buffer()) / TICK_DT_F64)
    }

    // ---- input ------------------------------------------------------------------------

    /// Issue a move order. Applied to the prediction immediately (03a §4).
    pub fn move_to(&mut self, target: Vec2, now: f64) -> Option<Command> {
        self.issue(CommandKind::MoveTo(QPoint::from_vec2(target)), now)
    }

    /// Cast the ability in `slot` (0–5 = Q W E R D F) toward a ground point. Windups, missiles,
    /// areas, dashes, blinks and shields are predicted at once.
    pub fn cast(&mut self, slot: u8, target: Vec2, now: f64) -> Option<Command> {
        self.issue(CommandKind::Cast { slot, target: QPoint::from_vec2(target) }, now)
    }

    /// Spend an ability point on slot 0–3 (Q W E R).
    pub fn level_up(&mut self, slot: u8, now: f64) -> Option<Command> {
        self.issue(CommandKind::LevelUp(slot), now)
    }

    /// Buy an item (predicted: the sim checks gold, slots and the fountain rule).
    pub fn buy(&mut self, item: u8, now: f64) -> Option<Command> {
        self.issue(CommandKind::Buy(item), now)
    }

    /// Sell the item in inventory slot 0–5.
    pub fn sell(&mut self, slot: u8, now: f64) -> Option<Command> {
        self.issue(CommandKind::Sell(slot), now)
    }

    /// Undo the last buy or sell (while the shop is still open).
    pub fn undo_trade(&mut self, now: f64) -> Option<Command> {
        self.issue(CommandKind::Undo, now)
    }

    /// Keep choice 0–2 of the open augment draft (ARAM: Mayhem). Predicted.
    pub fn pick_augment(&mut self, choice: u8, now: f64) -> Option<Command> {
        self.issue(CommandKind::PickAugment(choice), now)
    }

    /// Reroll the open augment draft (once per draft). Predicted.
    pub fn reroll_augments(&mut self, now: f64) -> Option<Command> {
        self.issue(CommandKind::RerollAugments, now)
    }

    /// Whether the shop is open for the own champion right now (predicted state).
    pub fn can_shop(&self) -> bool {
        self.world.unit(self.unit).is_some_and(|u| mftr_sim::world::can_shop(u, self.world.map(), &self.rules))
    }

    /// The match rules from the welcome.
    pub fn rules(&self) -> mftr_sim::world::Rules {
        self.rules
    }

    /// Basic-attack a visible unit (chasing it into range).
    pub fn attack(&mut self, target: UnitId, now: f64) -> Option<Command> {
        self.issue(CommandKind::Attack(target), now)
    }

    /// Walk toward a point, attacking the nearest enemy that comes into range.
    pub fn attack_move(&mut self, target: Vec2, now: f64) -> Option<Command> {
        self.issue(CommandKind::AttackMove(QPoint::from_vec2(target)), now)
    }

    pub fn stop(&mut self, now: f64) -> Option<Command> {
        self.issue(CommandKind::Stop, now)
    }

    fn issue(&mut self, kind: CommandKind, now: f64) -> Option<Command> {
        if self.phase != Phase::Playing {
            return None;
        }
        // Stamps never go backwards, so commands keep their issue order on the server.
        let t = self.input_time(now)?.max(self.last_stamp);
        self.last_stamp = t;
        let (tick, sub) = tick_at(t);
        let cmd = Command { player: self.player, seq: self.next_seq, tick, sub, kind };
        self.next_seq += 1;
        self.commands.push_back(cmd);
        self.stats.commands_issued += 1;
        self.last_command_at = Some(now);
        self.unsent_command = true;
        self.update(now);
        if self.world.tick() >= tick {
            self.resimulate_from(tick);
        }
        Some(cmd)
    }

    // ---- per-frame update -------------------------------------------------------------

    /// Advance clocks and predict up to the tick containing `T_input`.
    pub fn update(&mut self, now: f64) {
        let dt = self.last_update.map_or(0.0, |l| (now - l).max(0.0));
        self.last_update = Some(now);
        self.clock.update(dt);
        self.dilate_margin(dt);
        self.render_offset = self.render_offset * (0.5f64.powf(dt / CORRECTION_HALF_LIFE) as f32);
        if self.phase != Phase::Playing {
            return;
        }
        let Some(t) = self.input_time(now) else { return };
        let target = tick_at(t).0;
        while self.world.tick() < target {
            self.step_prediction();
        }
        // Freeze what we showed for enemy missiles and compare with the server (ghost hits);
        // forget own-cast predictions the server never confirmed within a second.
        let icpt = self.interceptions();
        let mut book = std::mem::take(&mut self.book);
        let last = self.world.tick();
        book.update_outcomes(t, self.unit, self.own_radius(), &|k| self.history_at(k), last, &icpt);
        if let Some(t_now) = self.now_ticks(now) {
            let stale = SimTime(((t_now - 30.0).max(0.0) * SUBTICKS as f64) as u64);
            book.prune_predicted(SimTime(u64::MAX), stale);
        }
        self.book = book;
        if let Some(t_interp) = self.interp_time(now) {
            self.effects.forget_before(t_interp);
        }
    }

    fn step_prediction(&mut self) {
        if self.spectator {
            // Nothing to predict: the (empty) world only keeps the clock.
            self.world.step(&[]);
            self.world.take_events();
            return;
        }
        let k = self.world.tick().next();
        let cmds: Vec<Command> = self.commands.iter().filter(|c| c.tick == k).copied().collect();
        // Proxies at the start of tick k, i.e. extrapolated to the end of tick k-1: blockers
        // near us (when collision proxies are on) and anything we might attack.
        let at = (k.0 - 1) as f64;
        let own_state = self.own_state();
        let own = own_state.pos;
        let chasing = match own_state.order {
            Order::Attack(id) => Some(id),
            _ => own_state.attack.map(|w| w.target),
        };
        let collide = self.collision_proxies;
        let proxies: Vec<Unit> = self
            .remote
            .values()
            .map(|t| t.proxy(at, collide))
            .filter(|u| {
                let d = u.state.pos.distance(own);
                (u.collision_radius > 0.0 && d <= PROXY_RANGE)
                    || (u.targetable() && d <= TARGET_PROXY_RANGE)
                    || Some(u.id) == chasing
            })
            .collect();
        self.world.replace_others(self.unit, proxies);
        self.world.step(&cmds);
        for e in self.world.take_events() {
            match e {
                SimEvent::MissileSpawned(m) if m.owner == self.unit => {
                    self.book.predicted_own.insert((m.cast_seq, m.shot), m);
                }
                SimEvent::AreaSpawned(a) if a.owner == self.unit => self.effects.predict_area(a),
                SimEvent::AttackLaunched(b) if b.owner == self.unit => self.effects.predict_bolt(b),
                SimEvent::Blinked { unit, from, to, at } if unit == self.unit && at > self.last_own_blink => {
                    self.last_own_blink = at;
                    self.notices.push(Notice::Blinked { unit, from, to });
                }
                _ => {}
            }
        }
        let st = self.own_state();
        self.history.push_back((k, st));
        while self.history.len() > HISTORY_TICKS {
            self.history.pop_front();
        }
    }

    fn own_state(&self) -> UnitState {
        self.world.unit(self.unit).map(|u| u.state).expect("own unit present while playing")
    }

    fn history_at(&self, k: Tick) -> Option<UnitState> {
        let first = self.history.front()?.0;
        if k < first {
            return None;
        }
        self.history.get((k.0 - first.0) as usize).filter(|(t, _)| *t == k).map(|(_, s)| *s)
    }

    /// Re-run prediction from tick `from` (inclusive) up to the current predicted tick.
    fn resimulate_from(&mut self, from: Tick) {
        let end = self.world.tick();
        let base_tick = Tick(from.0.saturating_sub(1));
        let Some(base) = self.history_at(base_tick) else { return };
        while self.history.back().is_some_and(|(t, _)| *t > base_tick) {
            self.history.pop_back();
        }
        self.world.set_tick(base_tick);
        if let Some(u) = self.world.unit_mut(self.unit) {
            u.state = base;
            // Stats follow the level and items in the state (the server already applied them).
            u.reset_stats();
        }
        // Own missiles, areas and bolts predicted after the base tick are re-created by the
        // re-simulation.
        self.book.prune_predicted(SimTime::end_of(base_tick), SimTime(0));
        self.effects.prune_predicted(SimTime::end_of(base_tick), SimTime(0));
        while self.world.tick() < end {
            self.step_prediction();
        }
    }

    // ---- incoming ---------------------------------------------------------------------

    pub fn handle_packet(&mut self, bytes: &[u8], now: f64) {
        self.stats.bytes_down += bytes.len() as u64;
        self.stats.packets_down += 1;
        let (header, message) = match msg::decode_server(bytes) {
            Ok(v) => v,
            Err(_) => {
                self.stats.decode_errors += 1;
                return;
            }
        };
        if !self.recv.record(header.seq) {
            return; // duplicate
        }
        self.send.on_ack(header.ack, header.ack_bits);
        match message {
            ServerMessage::Welcome {
                player,
                unit,
                team,
                map,
                champion,
                home,
                rules,
                tick,
                since_tick_us,
                time_echo,
                token,
                spectator,
                mode,
                ..
            } => {
                if matches!(self.phase, Phase::Connecting | Phase::Lobby) {
                    self.welcome_seq = header.seq;
                    self.mode = mode;
                    self.token = token;
                    self.spectator = spectator;
                    self.lobby = None;
                    self.rules = rules;
                    self.player = player;
                    self.unit = unit;
                    self.team = team;
                    self.champion = champion;
                    self.home = home;
                    self.map = map.shared();
                    self.phase = Phase::Joining;
                    let rtt = rtt_from_echo(now, time_echo.client_time_us, time_echo.hold_us);
                    self.clock.add_sample(now, rtt, tick.end_seconds() + since_tick_us as f64 * 1e-6);
                }
            }
            ServerMessage::Snapshot(s) => self.on_snapshot(*s, now),
            ServerMessage::Reject { .. } => {}
            ServerMessage::Lobby(l) => {
                // The match ended and the server holds champion select again (packets from the
                // select before our welcome may still arrive late: those don't count).
                if matches!(self.phase, Phase::Joining | Phase::Playing) && seq_greater(header.seq, self.welcome_seq) {
                    self.back_to_lobby();
                }
                if matches!(self.phase, Phase::Connecting | Phase::Lobby) {
                    self.phase = Phase::Lobby;
                    self.lobby = Some(*l);
                }
            }
        }
    }

    /// Forget the finished match and wait in champion select. The connection (packet
    /// sequences), settings and cumulative stats stay; clock sync starts over, because the
    /// server's clock stood still during champion select.
    fn back_to_lobby(&mut self) {
        let old = std::mem::take(self);
        *self = Self {
            phase: Phase::Lobby,
            champion_request: old.champion_request,
            spectate: old.spectate,
            mode: old.mode,
            next_seq: old.next_seq,
            margin: old.margin,
            margin_target: old.margin_target,
            margin_override: old.margin_override,
            collision_proxies: old.collision_proxies,
            minion_bubble: old.minion_bubble,
            send: old.send,
            recv: old.recv,
            stats: old.stats,
            book: MissileBook { stats: old.book.stats, own_display: old.book.own_display, ..Default::default() },
            ..Self::new()
        };
    }

    /// What the server runs (known once welcomed).
    pub fn game_mode(&self) -> msg::GameMode {
        self.mode
    }

    fn on_snapshot(&mut self, s: Snapshot, now: f64) {
        if self.last_snapshot_tick.is_some_and(|t| s.tick <= t) {
            return; // stale or reordered
        }
        self.last_snapshot_tick = Some(s.tick);
        if let Some(e) = s.time_echo {
            let rtt = rtt_from_echo(now, e.client_time_us, e.hold_us);
            self.clock.add_sample(now, rtt, s.tick.end_seconds() + s.since_tick_us as f64 * 1e-6);
        }
        self.acked_seq = self.acked_seq.max(s.last_cmd_seq);

        // Reliable events, strictly in order; anything after a gap is resent by the server.
        for (seq, e) in &s.events {
            if *seq != self.last_event_seq + 1 {
                if *seq > self.last_event_seq + 1 {
                    break;
                }
                continue;
            }
            self.last_event_seq = *seq;
            match *e {
                SimEvent::CastStarted { unit, at, dir, fire_at, .. } if unit != self.unit => {
                    self.remote_casts.insert(unit, (at, fire_at, dir));
                }
                SimEvent::MissileSpawned(m) => self.book.on_spawn(m, self.unit, self.team, now),
                SimEvent::MissileHit { id, target, at } => self.book.on_end(id, at, Some(target)),
                SimEvent::MissileExpired { id, at } => self.book.on_end(id, at, None),
                SimEvent::AreaSpawned(a) => self.effects.on_area(a, self.unit, self.team),
                SimEvent::AttackLaunched(b) => self.effects.on_bolt(b, self.unit, self.team),
                SimEvent::AttackLanded { id, at, .. } => self.effects.on_bolt_landed(id, at),
                SimEvent::Damage { source, target, kind, amount, absorbed, .. } => {
                    if source == self.unit {
                        self.stats.damage_dealt += (amount + absorbed) as f64;
                    }
                    if target == self.unit {
                        self.stats.damage_taken += (amount + absorbed) as f64;
                    }
                    self.combat_text.push(CombatText { target, source, kind, amount, absorbed, heal: false });
                }
                SimEvent::Died { unit, killer, .. } => {
                    let champion = self.remote.get(&unit).is_some_and(|t| t.latest.kind == UnitKind::Champion);
                    self.stats.kills += (killer == self.unit && champion) as u64;
                    self.stats.minion_kills += (killer == self.unit && !champion && unit != self.unit) as u64;
                    self.stats.deaths += (unit == self.unit) as u64;
                    self.notices.push(Notice::Died { unit, killer });
                }
                SimEvent::Respawned { unit, .. } => self.notices.push(Notice::Respawned { unit }),
                SimEvent::Healed { unit, amount, .. } => self.combat_text.push(CombatText {
                    target: unit,
                    source: unit,
                    kind: DamageKind::True,
                    amount,
                    absorbed: 0.0,
                    heal: true,
                }),
                SimEvent::Reward { unit, gold, xp, .. } if unit == self.unit => {
                    self.notices.push(Notice::Reward { gold, xp })
                }
                SimEvent::MatchEnded { winner, .. } => {
                    self.stats.matches_won += (winner == self.team) as u64;
                    self.stats.matches_lost += (winner != self.team) as u64;
                    self.notices.push(Notice::MatchEnded { winner });
                }
                SimEvent::Blinked { unit, from, to, .. } if unit != self.unit => {
                    self.notices.push(Notice::Blinked { unit, from, to })
                }
                _ => {}
            }
        }
        // Bounded even if nobody drains them (headless bots).
        if self.combat_text.len() > 256 {
            self.combat_text.drain(..128);
        }
        if self.notices.len() > 64 {
            self.notices.drain(..32);
        }

        // Other units arrive as deltas against a snapshot we reconstructed earlier (03b §6).
        let base = match s.baseline {
            None => Some(BTreeMap::new()),
            Some(b) => self.reconstructed.iter().find(|(t, _)| *t == b).map(|(_, m)| m.clone()),
        };
        let (others, fresh): (Vec<RemoteUnit>, bool) = match base {
            Some(base) => {
                let ticks = s.baseline.map_or(0, |b| s.tick.0 - b.0);
                let units = delta::reconstruct(&base, &s.others, &s.removed, ticks);
                let list = units.values().copied().collect();
                self.reconstructed.push_back((s.tick, units));
                while self.reconstructed.len() > RECONSTRUCTED_RING {
                    self.reconstructed.pop_front();
                }
                self.snapshot_ack = s.tick;
                (list, true)
            }
            // We no longer have that baseline: keep the units we have; the server falls back
            // to a full snapshot once our ack is older than its ring.
            None => (self.remote.values().map(|t| t.latest).collect(), false),
        };
        for o in others.iter().filter(|_| fresh) {
            let track = self.remote.entry(o.id).or_insert_with(|| RemoteTrack {
                samples: VecDeque::new(),
                latest: *o,
                latest_tick: s.tick,
            });
            track.samples.push_back((s.tick, o.pos.to_vec2()));
            while track.samples.len() > 32 {
                track.samples.pop_front();
            }
            track.latest = *o;
            track.latest_tick = s.tick;
        }
        let alive: Vec<UnitId> = others.iter().map(|o| o.id).collect();
        self.remote.retain(|id, _| alive.contains(id));

        let mut resim_from: Option<Tick> = None;
        // New information about nearby units changes the proxies (blockers, and targets while
        // attacking): re-predict everything after this snapshot so they are always the freshest.
        if self.phase == Phase::Playing {
            let own = self.history_at(s.tick);
            let attacking = own.is_some_and(|st| matches!(st.order, Order::Attack(_) | Order::AttackMove(_)));
            let range = if attacking {
                TARGET_PROXY_RANGE + 200.0
            } else if self.collision_proxies {
                PROXY_RANGE + 200.0
            } else {
                -1.0
            };
            let near =
                own.is_some_and(|st| self.remote.values().any(|t| t.latest.pos.to_vec2().distance(st.pos) <= range));
            if near {
                resim_from = Some(s.tick.next());
            }
        }
        for rep in &s.reports {
            if let Some(t) = self.apply_report(rep) {
                resim_from = Some(resim_from.map_or(t, |r: Tick| r.min(t)));
            }
        }

        if self.spectator {
            // Watching: nothing to predict; just keep the (empty) world on the clock.
            if self.phase == Phase::Joining {
                self.world = World::from_units(s.tick, Vec::new());
                self.world.set_map(self.map.clone());
                self.world.set_prediction_mode(true);
                self.phase = Phase::Playing;
            }
            self.update(now);
            return;
        }
        let Some((id, server_state)) = s.own else { return };
        if id != self.unit {
            return;
        }
        match self.phase {
            Phase::Connecting | Phase::Lobby => {}
            Phase::Joining => {
                self.start_playing(s.tick, server_state);
                self.update(now);
            }
            Phase::Playing => {
                // Retire commands the snapshot already covers (keeping unacked ones for redundancy).
                let acked = self.acked_seq;
                self.commands.retain(|c| c.tick > s.tick || c.seq > acked);
                if s.tick > self.world.tick() {
                    // We fell behind the server (e.g. a long stall). Restart from authority.
                    self.stats.hard_resets += 1;
                    self.start_playing(s.tick, server_state);
                    self.update(now);
                    return;
                }
                let Some(predicted) = self.history_at(s.tick) else {
                    self.stats.hard_resets += 1;
                    self.start_playing(s.tick, server_state);
                    self.update(now);
                    return;
                };
                self.stats.reconciliations += 1;
                // Every server-driven change to the prediction (a state mismatch, or a command
                // the server applied later than predicted) goes through the same smoothing.
                let before = self.own_render_position_raw(now);
                let mut changed = false;
                if !predicted.bits_eq(&server_state) {
                    self.stats.mismatches += 1;
                    let idx = (s.tick.0 - self.history.front().unwrap().0.0) as usize;
                    self.history[idx].1 = server_state;
                    let from = s.tick.next();
                    resim_from = Some(resim_from.map_or(from, |r| r.min(from)));
                    changed = true;
                }
                // Ticks up to `s.tick` are authoritative now; re-simulate only what follows.
                if let Some(r) = resim_from.map(|r| r.max(s.tick.next())).filter(|r| *r <= self.world.tick()) {
                    self.resimulate_from(r);
                    changed = true;
                }
                if changed && let (Some(b), Some(a)) = (before, self.own_render_position_raw(now)) {
                    let delta = b - a;
                    let len = delta.length();
                    if len > 0.01 {
                        self.stats.corrections.push(len);
                    }
                    if len >= CORRECTION_SNAP {
                        self.render_offset = Vec2::ZERO;
                    } else if len >= CORRECTION_ABSORB {
                        self.render_offset += delta;
                    }
                }
            }
        }
    }

    /// Record a command report. If the server applied the command somewhere other than where
    /// we predicted it (late arrival), fix our copy and return the earliest affected tick.
    fn apply_report(&mut self, rep: &CommandReport) -> Option<Tick> {
        if self.seen_reports.insert(rep.seq, (rep.applied_tick, rep.applied_sub)).is_some() {
            return None;
        }
        while self.seen_reports.len() > 256 {
            self.seen_reports.pop_first();
        }
        let lead = rep.lead_us as f64 * 1e-6;
        self.stats.leads_us.push(rep.lead_us);
        if rep.lead_us < 0 {
            self.stats.commands_late += 1;
        }
        self.leads.push_back(lead);
        while self.leads.len() > LEAD_WINDOW {
            self.leads.pop_front();
        }
        self.steer_margin();
        let cmd = self.commands.iter_mut().find(|c| c.seq == rep.seq)?;
        if cmd.tick == rep.applied_tick && cmd.sub == rep.applied_sub {
            return None;
        }
        let earliest = cmd.tick.min(rep.applied_tick);
        cmd.tick = rep.applied_tick;
        cmd.sub = rep.applied_sub;
        Some(earliest)
    }

    /// Keep the 1st-percentile arrival lead at ≥ 2 ms (03a §10.2). This sets the *target*;
    /// [`Self::dilate_margin`] moves the applied margin there smoothly.
    ///
    /// The low tail is estimated as the lower of the window minimum and `mean − 2.6σ`.
    /// A minimum over a few dozen samples is only a ~2nd percentile, and arrival jitter is
    /// roughly normal, so the σ term is what actually holds the 1% line.
    fn steer_margin(&mut self) {
        let n = self.leads.len() as f64;
        let min_lead = self.leads.iter().copied().fold(f64::INFINITY, f64::min);
        if !min_lead.is_finite() {
            return;
        }
        let mean = self.leads.iter().sum::<f64>() / n;
        let var = self.leads.iter().map(|l| (l - mean) * (l - mean)).sum::<f64>() / n;
        let low = if n >= 8.0 { min_lead.min(mean - 2.6 * var.sqrt()) } else { min_lead };
        self.margin_target = (self.margin - (low - TARGET_MIN_LEAD)).clamp(MARGIN_MIN, MARGIN_MAX);
    }

    /// Move the applied margin toward its target by time dilation, never by a jump.
    fn dilate_margin(&mut self, dt: f64) {
        let step = (self.margin_target - self.margin).clamp(-MARGIN_SHRINK_PER_S * dt, MARGIN_GROW_PER_S * dt);
        if step != 0.0 {
            // Leads in the window were measured under the old margin; shift them to what they
            // would have been under the new one, so one late report isn't counted again and again.
            for l in self.leads.iter_mut() {
                *l += step;
            }
            self.margin += step;
        }
    }

    fn start_playing(&mut self, tick: Tick, state: UnitState) {
        let stats = self.champion.def().stats;
        let mut unit = Unit::champion(self.unit, self.player, self.team, self.champion, state.pos, self.home, stats);
        unit.state = state;
        unit.reset_stats();
        self.world = World::from_units(tick, vec![unit]);
        self.world.set_rules(self.rules);
        self.world.set_prediction_mode(true);
        self.world.set_map(self.map.clone());
        self.history.clear();
        self.history.push_back((tick, state));
        self.render_offset = Vec2::ZERO;
        self.phase = Phase::Playing;
    }

    // ---- rendering --------------------------------------------------------------------

    fn own_render_position_raw(&self, now: f64) -> Option<Vec2> {
        let t = self.input_time(now)?;
        let (k, _) = tick_at(t);
        let frac = (t - t.floor()) as f32;
        let b = self.history_at(k).or_else(|| self.history.back().map(|(_, s)| *s))?;
        let a = self.history_at(Tick(k.0.saturating_sub(1))).unwrap_or(b);
        Some(a.pos.lerp(b.pos, frac))
    }

    /// Size of the correction currently being smoothed out on screen (u).
    pub fn visible_correction(&self) -> f32 {
        self.render_offset.length()
    }

    /// Own champion's display position on `T_input`, including correction smoothing.
    pub fn own_render_position(&self, now: f64) -> Option<Vec2> {
        self.own_render_position_raw(now).map(|p| p + self.render_offset)
    }

    /// Predicted own position at the end of a tick (Netcode Lab ground-truth comparisons).
    pub fn predicted_at(&self, tick: Tick) -> Option<Vec2> {
        self.history_at(tick).map(|s| s.pos)
    }

    /// Fault injection for the Netcode Lab: pin the input margin (e.g. to 0) instead of
    /// letting the control loop steer it.
    pub fn set_margin_override(&mut self, margin: Option<f64>) {
        self.margin_override = margin;
    }

    /// Draw minions near the own champion blended toward `T_input` (only with collision
    /// proxies on: the blend shows where the proxies are).
    pub fn set_minion_bubble(&mut self, enabled: bool) {
        self.minion_bubble = enabled;
    }

    /// Option A or B for own missiles (D12).
    pub fn set_own_missile_display(&mut self, display: OwnMissileDisplay) {
        self.book.own_display = display;
    }

    pub fn set_collision_proxies(&mut self, enabled: bool) {
        self.collision_proxies = enabled;
    }

    /// Remote units for drawing: interpolated on `T_interp`, except minions near the own
    /// champion, which blend toward their `T_input` extrapolation so the minion you bump
    /// into is drawn where you bump into it (03a §5, the minion bubble).
    pub fn remote_render_units(&self, now: f64) -> Vec<RemoteRender> {
        let (Some(t_interp), Some(t_input)) = (self.interp_time(now), self.input_time(now)) else {
            return Vec::new();
        };
        let own = self.own_render_position(now);
        self.remote
            .values()
            .filter_map(|track| {
                let interp = interpolate(&track.samples, t_interp)?;
                let mut pos = interp;
                if self.collision_proxies && self.minion_bubble && track.latest.kind == UnitKind::Minion {
                    let w = own.map_or(0.0, |o| smoothstep(BUBBLE_OUTER, BUBBLE_INNER, interp.distance(o)));
                    if w > 0.0 {
                        pos = interp.lerp(track.extrapolate(t_input), w);
                    }
                }
                let windup = self.remote_casts.get(&track.latest.id).and_then(|&(at, fire, dir)| {
                    let t = if track.latest.team == self.team { t_interp } else { t_input } * SUBTICKS as f64;
                    let p = (t - at.0 as f64) / (fire.0 - at.0).max(1) as f64;
                    (0.0..1.0).contains(&p).then_some((p as f32, dir))
                });
                let l = &track.latest;
                Some(RemoteRender {
                    id: l.id,
                    kind: l.kind,
                    team: l.team,
                    collision_radius: l.collision_radius as f32,
                    pos,
                    windup,
                    champion: l.champion,
                    augments: l.augments,
                    gameplay_radius: track.gameplay_radius(),
                    health: l.health as f32,
                    max_health: l.max_health as f32,
                    shield: l.shield as f32,
                    level: l.level,
                    attacking: l.attacking,
                    stunned: l.stunned,
                    rooted: l.rooted,
                    dashing: l.dashing,
                    slowed: l.slowed,
                    protected: l.protected,
                    facing: mftr_net::msg::facing_from_wire(l.facing),
                    attack_variant: l.attack_variant,
                    recovering: l.recovering,
                })
            })
            .collect()
    }

    /// Missiles to draw this frame, each on its display timeline (03a §7).
    pub fn missiles_render(&self, now: f64) -> Vec<MissileRender> {
        let (Some(t_input), Some(t_interp)) = (self.input_time(now), self.interp_time(now)) else {
            return Vec::new();
        };
        let icpt = self.interceptions();
        self.book.render(t_input, t_interp, self.own_radius(), &|k| self.history_at(k), self.world.tick(), &icpt)
    }

    /// Delayed ground areas to draw this frame, each on its display timeline (03a §7).
    pub fn areas_render(&self, now: f64) -> Vec<AreaRender> {
        let (Some(t_input), Some(t_interp)) = (self.input_time(now), self.interp_time(now)) else {
            return Vec::new();
        };
        self.effects.areas(t_input, t_interp)
    }

    /// Basic-attack bolts in flight, aimed at wherever their targets are drawn.
    pub fn bolts_render(&self, now: f64) -> Vec<BoltRender> {
        let (Some(t_input), Some(t_interp)) = (self.input_time(now), self.interp_time(now)) else {
            return Vec::new();
        };
        let remotes = self.remote_render_units(now);
        let own = self.own_render_position(now);
        let drawn = |id: UnitId| {
            if id == self.unit { own } else { remotes.iter().find(|r| r.id == id).map(|r| r.pos) }
        };
        self.effects.bolts(t_input, t_interp, &drawn)
    }

    /// Other units as last reconstructed from a snapshot: `(tick, units)` (tests, debugging).
    pub fn reconstructed(&self) -> Option<(Tick, &BTreeMap<UnitId, RemoteUnit>)> {
        self.reconstructed.back().map(|(t, m)| (*t, m))
    }

    /// Confirmed damage since the last call (floating numbers).
    pub fn take_combat_text(&mut self) -> Vec<CombatText> {
        std::mem::take(&mut self.combat_text)
    }

    /// Kills and respawns since the last call.
    pub fn take_notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }

    /// The unit under a ground point, for right-click attacks: the closest visible enemy whose
    /// drawn hitbox (plus `slack`) contains the point. Turrets are immune for now.
    pub fn pick_enemy(&self, at: Vec2, slack: f32, now: f64) -> Option<UnitId> {
        self.remote_render_units(now)
            .into_iter()
            .filter(|r| r.team != self.team && !matches!(r.kind, UnitKind::RigTurret | UnitKind::Relic))
            .map(|r| (r.pos.distance(at) - r.gameplay_radius, r.id))
            .filter(|(gap, _)| *gap <= slack)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, id)| id)
    }

    /// For each enemy missile, the earliest predicted contact with another unit of our team
    /// (allied champions and minions as proxies, 03a §7), over the ticks we have predicted.
    fn interceptions(&self) -> BTreeMap<u32, SimTime> {
        let mut out = BTreeMap::new();
        let last = self.world.tick();
        for (id, t) in self.book.tracked.iter().filter(|(_, t)| t.side == Side::Enemy) {
            let m = &t.m;
            let end = m.end_at();
            let mut k = Tick((m.spawn_at.0 / SUBTICKS as u64) as u32 + 1);
            'ticks: while k <= last {
                let s0 = SimTime::end_of(Tick(k.0 - 1));
                if s0 >= end {
                    break;
                }
                let (a, b) = (m.spawn_at.max(s0), end.min(SimTime::end_of(k)));
                let mut best: Option<SimTime> = None;
                for track in self.remote.values() {
                    if track.latest.team != self.team || !track.takes_skillshots() {
                        continue;
                    }
                    let (q0, q1) = (track.pos_at((k.0 - 1) as f64), track.pos_at(k.0 as f64));
                    if let Some(at) = m.first_hit(a, b, s0, q0, q1, track.gameplay_radius()) {
                        best = Some(best.map_or(at, |b: SimTime| b.min(at)));
                    }
                }
                if let Some(at) = best {
                    out.insert(*id, at);
                    break 'ticks;
                }
                k = k.next();
            }
        }
        out
    }

    /// Enemy missiles still in flight, as the player sees them (scripted dodgers).
    pub fn threats(&self, now: f64) -> Vec<Threat> {
        let Some(t_input) = self.input_time(now) else { return Vec::new() };
        self.book.threats(t_input, now, self.own_radius(), &|k| self.history_at(k), self.world.tick())
    }

    /// No own missile or area is still waiting for the server's confirmation.
    pub fn book_is_settled(&self) -> bool {
        self.book.predicted_own.is_empty() && self.effects.predicted_area_count() == 0
    }

    /// The own champion's hitbox (augments can grow or shrink it).
    pub fn own_radius(&self) -> f32 {
        self.world.unit(self.unit).map_or(mftr_sim::world::CHAMPION_GAMEPLAY_RADIUS, |u| u.gameplay_radius)
    }

    pub fn dodge_stats(&self) -> &DodgeStats {
        &self.book.stats
    }

    /// Own state on the input timeline (cooldowns, cast, stun) for HUD display.
    pub fn own_state_now(&self) -> Option<UnitState> {
        self.history.back().map(|(_, s)| *s)
    }

    /// The latest predicted own state and the tick it is at the end of.
    pub fn own_state_latest(&self) -> Option<(Tick, UnitState)> {
        self.history.back().copied()
    }

    /// The input timeline as a `SimTime` (for comparing against cooldown/stun instants).
    pub fn input_sim_time(&self, now: f64) -> Option<SimTime> {
        self.input_time(now).map(|t| SimTime((t * SUBTICKS as f64) as u64))
    }
}

/// Interpolate between buffered snapshots; hold at the newest, never extrapolate (03a §10.3).
fn interpolate(buf: &VecDeque<(Tick, Vec2)>, t: f64) -> Option<Vec2> {
    let newest = buf.back()?;
    if t >= newest.0.0 as f64 {
        return Some(newest.1);
    }
    let i = buf.iter().position(|(k, _)| k.0 as f64 > t)?;
    if i == 0 {
        return Some(buf[0].1);
    }
    let (ka, pa) = buf[i - 1];
    let (kb, pb) = buf[i];
    if pa.distance(pb) > TELEPORT_DISTANCE * (kb.0 - ka.0) as f32 {
        return Some(pb); // a blink (or a respawn): step, don't smear it across the gap
    }
    let f = ((t - ka.0 as f64) / (kb.0 - ka.0) as f64) as f32;
    Some(pa.lerp(pb, f))
}

fn time_us(now: f64) -> u32 {
    (now * 1e6) as u64 as u32
}

fn rtt_from_echo(now: f64, client_time_us: u32, hold_us: u32) -> f64 {
    let elapsed = time_us(now).wrapping_sub(client_time_us);
    elapsed.saturating_sub(hold_us) as f64 * 1e-6
}
