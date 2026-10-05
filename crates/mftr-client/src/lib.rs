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
//!   blended toward `T_input` (the "minion bubble").
//!
//! Time is passed in explicitly (`now`, local seconds), so the same code runs under the Godot
//! client, the UDP bot, and the virtual-time Netcode Lab.

use mftr_net::PROTOCOL_VERSION;
use mftr_net::clock::ClockSync;
use mftr_net::msg::{self, ClientMessage, CommandReport, RemoteUnit, ServerMessage, Snapshot};
use mftr_net::packet::{PacketHeader, ReceiveTracker, SendTracker};
use mftr_sim::time::tick_at;
use mftr_sim::{
    Command, CommandKind, PlayerId, QPoint, TICK_DT_F64, Team, Tick, Unit, UnitId, UnitKind, UnitState, Vec2, World,
};
use std::collections::{BTreeMap, VecDeque};

/// Units farther than this from the own champion can't matter within a prediction horizon.
const PROXY_RANGE: f32 = 600.0;
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
    fn extrapolate(&self, at: f64) -> Vec2 {
        let p = self.latest.pos.to_vec2();
        let Some(target) = self.latest.target.map(QPoint::to_vec2) else { return p };
        let elapsed = (at - self.latest_tick.0 as f64).max(0.0);
        let dist = (self.latest.speed as f64 * elapsed * TICK_DT_F64) as f32;
        let to = target - p;
        let len = to.length();
        if len <= 0.0 || dist >= len { target } else { p + to * (dist / len) }
    }

    fn proxy(&self, at: f64) -> Unit {
        let r = self.latest.collision_radius as f32;
        Unit {
            id: self.latest.id,
            kind: self.latest.kind,
            owner: None,
            team: self.latest.team,
            // Static during the predicted tick: the server resolves movers against
            // start-of-tick positions, which is exactly what this is.
            state: UnitState::new(self.extrapolate(at), 0.0),
            collision_radius: r,
            gameplay_radius: r,
            brain: None,
        }
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Connecting,
    /// Welcomed, waiting for the first snapshot that carries our own unit.
    Joining,
    Playing,
}

pub struct ClientSession {
    phase: Phase,
    player: PlayerId,
    unit: UnitId,
    team: Team,
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
    leads: VecDeque<f64>,
    last_send: Option<f64>,
    last_command_at: Option<f64>,
    unsent_command: bool,
    remote: BTreeMap<UnitId, RemoteTrack>,
    collision_proxies: bool,
    render_offset: Vec2,
    last_update: Option<f64>,
    send: SendTracker,
    recv: ReceiveTracker,
    pub stats: ClientStats,
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
            leads: VecDeque::new(),
            last_send: None,
            last_command_at: None,
            unsent_command: false,
            remote: BTreeMap::new(),
            collision_proxies: true,
            render_offset: Vec2::ZERO,
            last_update: None,
            send: SendTracker::default(),
            recv: ReceiveTracker::default(),
            stats: ClientStats::default(),
        }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn player(&self) -> PlayerId {
        self.player
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
        let bytes =
            msg::encode_client(&h, &ClientMessage::Hello { protocol: PROTOCOL_VERSION, client_time_us: time_us(now) });
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
        let bytes = msg::encode_client(&h, &ClientMessage::Input { client_time_us: time_us(now), commands: unacked });
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
        self.now_ticks(now).map(|t| t + (self.clock.rtt() / 2.0 + self.margin) / TICK_DT_F64)
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
    }

    fn step_prediction(&mut self) {
        let k = self.world.tick().next();
        let cmds: Vec<Command> = self.commands.iter().filter(|c| c.tick == k).copied().collect();
        if self.collision_proxies {
            // Proxies at the start of tick k, i.e. extrapolated to the end of tick k-1.
            let at = (k.0 - 1) as f64;
            let own = self.own_state().pos;
            let proxies: Vec<Unit> = self
                .remote
                .values()
                .filter(|t| t.latest.collision_radius > 0)
                .map(|t| t.proxy(at))
                .filter(|u| u.state.pos.distance(own) <= PROXY_RANGE)
                .collect();
            self.world.replace_others(self.unit, proxies);
        }
        self.world.step(&cmds);
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
        }
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
            ServerMessage::Welcome { player, unit, team, tick, since_tick_us, time_echo, .. } => {
                if self.phase == Phase::Connecting {
                    self.player = player;
                    self.unit = unit;
                    self.team = team;
                    self.phase = Phase::Joining;
                    let rtt = rtt_from_echo(now, time_echo.client_time_us, time_echo.hold_us);
                    self.clock.add_sample(now, rtt, tick.end_seconds() + since_tick_us as f64 * 1e-6);
                }
            }
            ServerMessage::Snapshot(s) => self.on_snapshot(s, now),
            ServerMessage::Reject { .. } => {}
        }
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

        for o in &s.others {
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
        let alive: Vec<UnitId> = s.others.iter().map(|o| o.id).collect();
        self.remote.retain(|id, _| alive.contains(id));

        let mut resim_from: Option<Tick> = None;
        // New information about nearby units changes the collision proxies: re-predict
        // everything after this snapshot so the obstacles we assume are always the freshest.
        if self.collision_proxies && self.phase == Phase::Playing {
            let own = self.history_at(s.tick).map(|st| st.pos);
            let near = own.is_some_and(|p| {
                self.remote.values().any(|t| t.latest.pos.to_vec2().distance(p) <= PROXY_RANGE + 200.0)
            });
            if near {
                resim_from = Some(s.tick.next());
            }
        }
        for rep in &s.reports {
            if let Some(t) = self.apply_report(rep) {
                resim_from = Some(resim_from.map_or(t, |r: Tick| r.min(t)));
            }
        }

        let Some((id, server_state)) = s.own else { return };
        if id != self.unit {
            return;
        }
        match self.phase {
            Phase::Connecting => {}
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
                if changed {
                    if let (Some(b), Some(a)) = (before, self.own_render_position_raw(now)) {
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
        let unit = Unit {
            id: self.unit,
            kind: UnitKind::Champion,
            owner: Some(self.player),
            team: self.team, // allied champions pass through each other (D20)
            state,
            collision_radius: mftr_sim::world::CHAMPION_COLLISION_RADIUS,
            gameplay_radius: mftr_sim::world::CHAMPION_GAMEPLAY_RADIUS,
            brain: None,
        };
        self.world = World::from_units(tick, vec![unit]);
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
                if self.collision_proxies && track.latest.kind == UnitKind::Minion {
                    let w = own.map_or(0.0, |o| smoothstep(BUBBLE_OUTER, BUBBLE_INNER, interp.distance(o)));
                    if w > 0.0 {
                        pos = interp.lerp(track.extrapolate(t_input), w);
                    }
                }
                Some(RemoteRender {
                    id: track.latest.id,
                    kind: track.latest.kind,
                    team: track.latest.team,
                    collision_radius: track.latest.collision_radius as f32,
                    pos,
                })
            })
            .collect()
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
