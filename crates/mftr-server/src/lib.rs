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
    Area, Brain, ChampionId, Command, MinionKind, Missile, PlayerId, QPoint, SimEvent, SimTime, SubTick, TICK_DT_F64,
    TICK_HZ, Team, Tick, UnitId, Vec2, World,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Opaque per-connection address key, assigned by the transport.
pub type ClientKey = u64;

/// Commands later than this are dropped instead of applied late (03a §10.2).
const MAX_LATENESS: f64 = 0.250;
/// Commands targeting further ahead than this are clamped (anti-abuse).
const MAX_LEAD_TICKS: u32 = 90;
const REPORT_REPEAT_TICKS: u32 = 15;
const TIMEOUT: f64 = 10.0;
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
}

impl Scenario {
    /// The map each scenario is played on: the open plane for `empty`, the arena otherwise.
    pub fn map(self) -> MapId {
        match self {
            Scenario::Empty => MapId::Open,
            Scenario::MinionSandbox | Scenario::DodgeRig | Scenario::Duel => MapId::Arena,
            Scenario::Aram => MapId::Bridge,
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "empty" => Some(Scenario::Empty),
            "minions" => Some(Scenario::MinionSandbox),
            "dodge" => Some(Scenario::DodgeRig),
            "duel" => Some(Scenario::Duel),
            "aram" => Some(Scenario::Aram),
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
    if scenario == Scenario::DodgeRig {
        // Four turrets around the middle; together they cover most of the arena.
        for pos in
            [Vec2::new(900.0, 2000.0), Vec2::new(3100.0, 2000.0), Vec2::new(2000.0, 900.0), Vec2::new(2000.0, 3100.0)]
        {
            world.spawn_rig_turret(Team::Red, pos, 1100);
        }
        return;
    }
    if scenario == Scenario::Aram {
        world.start_match();
        return;
    }
    if scenario == Scenario::Duel {
        for (center, team) in [(Vec2::new(1700.0, 2050.0), Team::Red), (Vec2::new(2300.0, 2150.0), Team::Blue)] {
            clump(world, center, 1, team);
        }
        return;
    }
    if scenario != Scenario::MinionSandbox {
        return;
    }
    // Static clumps.
    for (center, rings, team) in [
        (Vec2::new(1200.0, 1200.0), 1i32, Team::Red),
        (Vec2::new(2800.0, 1300.0), 2, Team::Blue),
        (Vec2::new(1300.0, 2800.0), 2, Team::Red),
        (Vec2::new(2900.0, 2900.0), 1, Team::Blue),
    ] {
        clump(world, center, rings, team);
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

/// A hex-packed minion clump, adjacent minions touching, like a wave fighting in lane.
fn clump(world: &mut World, center: Vec2, rings: i32, team: Team) {
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

/// Duel spawn points (west for blue, east for red), spread a little per player.
fn duel_spawn(team: Team, player: PlayerId) -> Vec2 {
    let y = 2000.0 + 150.0 * ((player.0 / 2) as f32) * if player.0 % 4 < 2 { 1.0 } else { -1.0 };
    Vec2::new(if team == Team::Blue { 500.0 } else { 3500.0 }, y)
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
}

pub struct ServerCore {
    cfg: ServerConfig,
    world: World,
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
        let mut world = World::new(cfg.seed);
        world.set_map(cfg.scenario.map().shared());
        if cfg.scenario == Scenario::Aram {
            world.set_rules(mftr_sim::world::Rules::ARAM);
        }
        populate(&mut world, cfg.scenario);
        Self {
            cfg,
            world,
            start,
            conns: BTreeMap::new(),
            queue: Vec::new(),
            missiles: BTreeMap::new(),
            areas: BTreeMap::new(),
            stats: ServerStats::default(),
        }
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
            ClientMessage::Hello { protocol, client_time_us, champion } => {
                if protocol != PROTOCOL_VERSION {
                    out.push((from, self.reject(RejectReason::ProtocolMismatch)));
                } else if !self.conns.contains_key(&from) && self.conns.len() >= self.cfg.max_players as usize {
                    out.push((from, self.reject(RejectReason::ServerFull)));
                } else {
                    if !self.conns.contains_key(&from) {
                        self.join(from, now, champion);
                    }
                    let tick = self.world.tick();
                    let since = ((now - self.tick_time(tick)).max(0.0) * 1e6) as u32;
                    let me = self.world.unit(self.conns[&from].unit);
                    let team = me.map_or(Team::Blue, |u| u.team);
                    let champ = me.and_then(|u| u.champion).unwrap_or(ChampionId::Ember);
                    let home = me.map_or(Vec2::ZERO, |u| u.home);
                    let conn = self.conns.get_mut(&from).unwrap();
                    conn.recv.record(header.seq);
                    conn.last_heard = now;
                    let msg = ServerMessage::Welcome {
                        player: conn.player,
                        unit: conn.unit,
                        team,
                        map: self.world.map().id,
                        champion: champ,
                        home,
                        rules: self.world.rules(),
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
            ClientMessage::Input { client_time_us, event_ack, snapshot_ack, commands } => {
                let world_tick = self.world.tick();
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

    fn join(&mut self, key: ClientKey, now: f64, champion: Option<ChampionId>) {
        let used: Vec<u8> = self.conns.values().map(|c| c.player.0).collect();
        let player = PlayerId((0..=u8::MAX).find(|p| !used.contains(p)).unwrap());
        let team = if player.0 % 2 == 0 || self.cfg.scenario == Scenario::DodgeRig { Team::Blue } else { Team::Red };
        // Without a preference, alternate: the first duel is mage vs. marksman.
        let champion =
            champion.unwrap_or(ChampionId::ALL[((player.0 / 2) as usize % 2) ^ (team == Team::Red) as usize]);
        let (lo, hi) = (self.cfg.arena_min, self.cfg.arena_max);
        // A random spot with nothing within 150 u (deterministic: the world RNG).
        let mut pos = Vec2::ZERO;
        for _ in 0..64 {
            let rng = self.world.rng();
            pos = QPoint::from_vec2(Vec2::new(rng.range_f32(lo, hi), rng.range_f32(lo, hi))).to_vec2();
            if self.world.map().walkable(pos, 60.0)
                && self.world.units().iter().all(|u| u.state.pos.distance(pos) > 150.0)
            {
                break;
            }
        }
        if self.cfg.scenario == Scenario::Duel {
            pos = duel_spawn(team, player);
        }
        if self.cfg.scenario == Scenario::Aram {
            // In the fountain, spread out in a small arc per player.
            let base = self.world.map().layout.champion_spawn[team as usize];
            let k = (player.0 / 2) as f32;
            pos = base + Vec2::new(0.0, 90.0 * (k - 2.0));
        }
        let unit = self.world.spawn_champion(player, team, champion, pos);
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
                events: VecDeque::new(),
                next_event_seq: 1,
                team,
                revealed: BTreeSet::new(),
                sent: VecDeque::new(),
                snapshot_ack: Tick(0),
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

        // A new match 10 s after a Base falls (ARAM).
        if let Some((_, at)) = self.world.game().winner
            && SimTime::end_of(self.world.tick()) >= at.plus(mftr_sim::SimDuration::from_millis(10_000))
        {
            self.world.restart_match();
        }
        let k = self.world.tick().next();
        let (due, later): (Vec<Command>, Vec<Command>) = self.queue.drain(..).partition(|c| c.tick <= k);
        self.queue = later;
        self.world.step(&due);
        let events = self.world.take_events();
        self.stats.ticks += 1;

        let since = ((now - self.tick_time(k)).max(0.0) * 1e6) as u32;
        let (s0, s1) = (SimTime::end_of(Tick(k.0 - 1)), SimTime::end_of(k));
        let units: Vec<(UnitId, mftr_sim::UnitState, RemoteUnit)> = self
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
                    speed: st.move_speed.round().clamp(0.0, 1023.0) as u16,
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
                };
                (u.id, u.state, remote)
            })
            .collect();
        // Fog of war (03 §10): what each team sees this tick.
        let map = self.world.map().clone();
        let visions = [Vision::of(&self.world, Team::Blue), Vision::of(&self.world, Team::Red)];
        let seen: [BTreeSet<UnitId>; 2] =
            [0, 1].map(|i| self.world.units().iter().filter(|u| visions[i].sees_unit(&map, u)).map(|u| u.id).collect());
        // Units a team can't see can't be targeted by its attacks next tick.
        for (i, team) in [Team::Blue, Team::Red].into_iter().enumerate() {
            let hidden = self.world.units().iter().filter(|u| u.team != team && !seen[i].contains(&u.id)).map(|u| u.id);
            let hidden = hidden.collect();
            self.world.set_hidden(team, hidden);
        }
        // Whether team `i` may hear about something happening to `unit` at its position this
        // tick (also for units that just died, which vision no longer lists).
        let knows = |i: usize, unit: UnitId| {
            self.world.unit(unit).is_some_and(|u| {
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
        let mut team_events: [Vec<SimEvent>; 2] = [Vec::new(), Vec::new()];
        for (i, list) in team_events.iter_mut().enumerate() {
            for e in &events {
                let ok = match *e {
                    SimEvent::CastStarted { unit, .. }
                    | SimEvent::AttackLaunched(mftr_sim::Bolt { owner: unit, .. }) => seen[i].contains(&unit),
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
                    SimEvent::Blinked { unit, from, to, at } if !visions[i].sees(&map, from) => {
                        SimEvent::Blinked { unit, from: to, to, at }
                    }
                    SimEvent::Dashed { unit, from, to, at, end_at } if !visions[i].sees(&map, from) => {
                        let start = self.world.unit(unit).map_or(to, |u| u.state.pos);
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
            let ti = (conn.team == Team::Red) as usize;
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
                    SimEvent::MissileSpawned(m) if m.team == conn.team => {
                        conn.revealed.insert(m.id);
                        push(conn, *e);
                    }
                    SimEvent::AreaSpawned(a) if a.team == conn.team => {
                        conn.revealed.insert(a.id);
                        push(conn, *e);
                    }
                    _ => {}
                }
            }
            // Enemy missiles: revealed when they enter vision, re-based to that point so the
            // caster's position isn't leaked (03 §10).
            for m in self.missiles.values() {
                if m.team == conn.team || conn.revealed.contains(&m.id) {
                    continue;
                }
                let a = m.spawn_at.max(s0);
                let b = ended.get(&m.id).copied().unwrap_or(m.end_at()).min(s1);
                let at = [a, b].into_iter().find(|&t| visions[ti].sees(&map, m.position_at(t)));
                if let Some(t) = at {
                    conn.revealed.insert(m.id);
                    push(conn, SimEvent::MissileSpawned(rebase(m, t)));
                }
            }
            // Enemy areas: the telegraph shows once its center is in vision.
            for a in self.areas.values() {
                if a.team != conn.team && !conn.revealed.contains(&a.id) && visions[ti].sees(&map, a.center) {
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
            let mut visible: Vec<&RemoteUnit> = units
                .iter()
                .filter(|(id, _, _)| *id != conn.unit && seen[ti].contains(id))
                .map(|(_, _, r)| r)
                .collect();
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
            while bytes.len() > mftr_net::MAX_PACKET_BYTES && !snap.others.is_empty() {
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
            while bytes.len() > mftr_net::MAX_PACKET_BYTES && !snap.events.is_empty() {
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
        &mut self.world
    }

    /// Unit ids a client on `team` may currently know about (tests, debugging).
    pub fn visible_to(&self, team: Team) -> BTreeSet<UnitId> {
        let map = self.world.map().clone();
        let v = Vision::of(&self.world, team);
        self.world.units().iter().filter(|u| v.sees_unit(&map, u)).map(|u| u.id).collect()
    }
}

/// The same missile, as first seen at `t`: identical path from there on, but starting at its
/// position at `t`, so where it was fired from stays hidden.
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
            &ClientMessage::Hello { protocol: PROTOCOL_VERSION, client_time_us: 0, champion: None },
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

    /// Slice 3 exit: hidden units never reach a client. Behind a wall, in a bush, or out of
    /// range: absent from the snapshot. Visible: present.
    #[test]
    fn fog_of_war_culls_hidden_units() {
        let mut core = ServerCore::new(ServerConfig { scenario: Scenario::MinionSandbox, ..Default::default() }, 0.0);
        hello(&mut core, 1); // player 0, blue
        hello(&mut core, 2); // player 1, red
        let (me, enemy) = (core.conns[&1].unit, core.conns[&2].unit);
        // Only the two champions provide vision in this test.
        let minions: Vec<UnitId> = core.world.units().iter().filter(|u| u.owner.is_none()).map(|u| u.id).collect();
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
            &ClientMessage::Hello { protocol: PROTOCOL_VERSION, client_time_us: 0, champion: Some(champion) },
        );
        core.handle_packet(key, &bytes, 0.0);
    }

    /// Duel: players get alternating champions at their team's spawn by default.
    #[test]
    fn duel_assigns_champions_and_spawns() {
        let mut core = ServerCore::new(ServerConfig { scenario: Scenario::Duel, ..Default::default() }, 0.0);
        hello(&mut core, 1);
        hello(&mut core, 2);
        let unit = |core: &ServerCore, key| core.world.unit(core.conns[&key].unit).unwrap().clone();
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
        let minions: Vec<UnitId> = core.world.units().iter().filter(|u| u.owner.is_none()).map(|u| u.id).collect();
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
        let hp = core.world.unit(red).unwrap().state.health;
        assert!(hp < ChampionId::Ember.def().stats.max_health - 50.0, "red took the area: {hp}");
        assert!(red_heard, "the victim's client hears its own damage");
        assert!(!blue_saw_red && !blue_heard, "nothing about the hidden unit reaches blue");
    }
}
