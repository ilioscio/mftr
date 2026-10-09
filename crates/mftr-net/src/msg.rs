//! Messages and their encoding (M0 subset of 03b §5–§8).
//!
//! Every packet: magic byte, packet header, 2-bit message kind, payload. M0 uses absolute
//! 32-bit ticks and sequence numbers for simplicity. Delta-coding (03b) comes once the
//! formats settle.

use crate::bits::{BitReader, BitWriter, DecodeError};
use crate::delta::{self, UnitUpdate};
use crate::packet::PacketHeader;
use mftr_sim::ability::{Cc, Damage, DamageKind, LineSkillshot, SLOTS};
use mftr_sim::items::INVENTORY;
use mftr_sim::map::MapId;
use mftr_sim::world::{BufferedCast, DamageOrigin, EchoCast, MAX_PATH, Path, Progress, Recovery, Rules, Trade, UNDO};
use mftr_sim::{
    Area, AttackWindup, Bolt, Cast, ChampionId, Command, CommandKind, DashMove, MinionKind, Missile, Order, PlayerId,
    QPoint, SimDuration, SimEvent, SimTime, SubTick, Team, Tick, UnitId, UnitKind, UnitState, Vec2,
};

const MAGIC: u8 = 0x4D; // 'M'

/// Max commands carried per input packet (redundancy window, 03b §4).
pub const MAX_COMMANDS_PER_PACKET: usize = 8;
/// Champion select lists at most this many players (and bench champions).
pub const MAX_LOBBY_SLOTS: usize = 31;
/// Max command reports repeated per snapshot.
pub const MAX_REPORTS_PER_SNAPSHOT: usize = 8;
/// Max reliable events carried per snapshot; the rest wait for the next one (03b §7).
pub const MAX_EVENTS_PER_SNAPSHOT: usize = 32;
/// Bound on units (updates or removals) per snapshot (strict decoding, 03b §11).
pub const MAX_UNITS_PER_SNAPSHOT: usize = 1024;

#[derive(Clone, Debug, PartialEq)]
pub enum ClientMessage {
    Hello {
        protocol: u16,
        client_time_us: u32,
        /// Preferred champion (the server may assign another when none is asked for).
        champion: Option<ChampionId>,
        /// A session token from an earlier Welcome: take that champion back (reconnect).
        /// 0 = a new player.
        resume: u64,
        /// Watch instead of play: full vision, no unit.
        spectate: bool,
    },
    /// The `player` field of each command is ignored by the server (taken from the connection).
    Input {
        client_time_us: u32,
        /// Highest event sequence received in order (cumulative ack for the events channel).
        event_ack: u32,
        /// Tick of the newest snapshot the client fully reconstructed (its delta baseline,
        /// 03b §6); 0 = none yet.
        snapshot_ack: u32,
        commands: Vec<Command>,
    },
    Bye,
    /// Champion select (M2 slice 5).
    Lobby(LobbyAction),
}

/// What a player can do in champion select (ARAM all-random, 06 §2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LobbyAction {
    /// Trade the current champion for a random unused one; it goes to the team bench.
    Reroll,
    /// Swap with a champion on the team bench.
    Take(ChampionId),
    Ready(bool),
}

/// One player in champion select.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LobbySlot {
    pub player: PlayerId,
    pub team: Team,
    pub champion: ChampionId,
    pub rerolls: u8,
    pub ready: bool,
    pub bot: bool,
}

/// Champion select as the receiving player sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct LobbyState {
    pub you: PlayerId,
    pub slots: Vec<LobbySlot>,
    /// Each team's bench of rerolled champions (only the own team's is sent).
    pub bench: Vec<ChampionId>,
    /// Milliseconds until the match starts.
    pub starts_in_ms: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeEcho {
    pub client_time_us: u32,
    /// How long the server held the client's packet before replying.
    pub hold_us: u32,
}

/// Where and when the server actually applied a command, and how early it arrived (03a §10.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandReport {
    pub seq: u32,
    pub applied_tick: Tick,
    pub applied_sub: SubTick,
    /// Arrival lead in microseconds, quantized to 0.5 ms on the wire. Negative = late.
    pub lead_us: i32,
}

/// Another unit, quantized. Carries what the client needs to interpolate it and to use it as
/// a collision proxy on the input timeline (03a §5): where it's heading and how fast.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RemoteUnit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub team: Team,
    pub pos: QPoint,
    /// Current heading (detour waypoint first), if moving.
    pub target: Option<QPoint>,
    /// Units per second (10 bits on the wire).
    pub speed: u16,
    /// Collision radius in whole units (8 bits).
    pub collision_radius: u8,
    /// Hitbox radius in whole units (8 bits): targeting and skillshot hits.
    pub gameplay_radius: u8,
    /// A structure that can't be hurt yet (an earlier one in its lane still stands).
    pub protected: bool,
    pub champion: Option<ChampionId>,
    /// A minion's kind (2 bits, minions only): which model the client draws (A5).
    pub minion: Option<MinionKind>,
    /// Held augments (ARAM: Mayhem): shown as indicators above the health bar (06 §3).
    pub augments: [u8; mftr_sim::augments::SLOTS],
    /// Whole health points (16 bits each): confirmed values only (03a §7).
    pub health: u16,
    pub max_health: u16,
    pub shield: u16,
    /// Champion level (1–18; 0 for other units).
    pub level: u8,
    /// Status flags for display (windup animations, CC indicators).
    pub casting: bool,
    pub attacking: bool,
    pub stunned: bool,
    pub rooted: bool,
    pub dashing: bool,
    pub slowed: bool,
    /// In an ability's follow-through (10 §4.1).
    pub recovering: bool,
    /// Basic attacks started, modulo 4: picks the attack animation (10 §5.2).
    pub attack_variant: u8,
    /// Facing angle, 10 bits (see [`facing_to_wire`]).
    pub facing: u16,
}

/// Facing for display, quantized to 1024 steps (0.35°) for other units (10 §3). Server only:
/// the quantization uses `atan2`, which is fine because nothing simulates with the result.
pub fn facing_to_wire(facing: Vec2) -> u16 {
    let turns = facing.y.atan2(facing.x) / std::f32::consts::TAU;
    ((turns.rem_euclid(1.0) * 1024.0).round() as u16) & 1023
}

/// The facing angle in radians (counter-clockwise from +x, in game coordinates).
pub fn facing_from_wire(q: u16) -> f32 {
    (q & 1023) as f32 / 1024.0 * std::f32::consts::TAU
}

/// One champion on the scoreboard (the Tab breakdown, 01 §13).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScoreRow {
    pub unit: UnitId,
    pub champion: ChampionId,
    pub team: Team,
    pub bot: bool,
    pub level: u8,
    pub kills: u16,
    pub deaths: u16,
    pub assists: u16,
    /// Minions killed (last hits).
    pub cs: u16,
    pub items: [u8; INVENTORY],
    pub augments: [u8; mftr_sim::augments::SLOTS],
    /// Tenths of a second until it respawns; 0 while alive.
    pub respawn_ds: u16,
}

/// The match as a whole: when it began (the clock) and every champion's score. Sent with a
/// snapshot about once a second, to players and spectators alike.
#[derive(Clone, Debug, PartialEq)]
pub struct Scoreboard {
    pub started_at: SimTime,
    pub rows: Vec<ScoreRow>,
}

/// Most rows a scoreboard carries.
pub const MAX_SCORE_ROWS: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub tick: Tick,
    /// Server time elapsed since the end of `tick` when this snapshot was sent.
    pub since_tick_us: u32,
    pub time_echo: Option<TimeEcho>,
    /// Highest command sequence received from this client.
    pub last_cmd_seq: u32,
    pub reports: Vec<CommandReport>,
    /// The receiving client's own unit, lossless (03b §6).
    pub own: Option<(UnitId, UnitState)>,
    /// The snapshot `others` is relative to (a tick the client reported); `None` = full.
    pub baseline: Option<Tick>,
    /// Other units that changed since the baseline (all of them without one).
    pub others: Vec<UnitUpdate>,
    /// Units in the baseline that are no longer visible (or gone).
    pub removed: Vec<UnitId>,
    /// Reliable ordered events `(seq, event)`, repeated until acknowledged (03b §7).
    pub events: Vec<(u32, SimEvent)>,
    /// The scoreboard, about once a second.
    pub scoreboard: Option<Box<Scoreboard>>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerMessage {
    Welcome {
        player: PlayerId,
        unit: UnitId,
        /// Own team: missile sides (own/ally/enemy) and ally/enemy display.
        team: Team,
        /// The map both sides simulate on (walls, brush, pathing).
        map: MapId,
        /// Own champion and respawn point (prediction rebuilds the unit from these).
        champion: ChampionId,
        home: Vec2,
        /// Match rules prediction applies too (passive gold, ranks).
        rules: Rules,
        tick: Tick,
        tick_hz: u8,
        since_tick_us: u32,
        time_echo: TimeEcho,
        /// Present this token in a later Hello to take the champion back (reconnect).
        token: u64,
        /// Watching: no own unit, full vision.
        spectator: bool,
        /// What the server runs (shown in the client's server list).
        mode: GameMode,
    },
    Snapshot(Box<Snapshot>),
    Reject {
        reason: RejectReason,
    },
    /// Champion select, sent a few times a second until the match starts.
    Lobby(Box<LobbyState>),
}

/// The kind of game a server runs (its scenario).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GameMode {
    #[default]
    Empty = 0,
    Minions = 1,
    Dodge = 2,
    Duel = 3,
    Aram = 4,
    Mayhem = 5,
    Hyper = 6,
}

impl GameMode {
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0 => GameMode::Empty,
            1 => GameMode::Minions,
            2 => GameMode::Dodge,
            3 => GameMode::Duel,
            4 => GameMode::Aram,
            5 => GameMode::Mayhem,
            6 => GameMode::Hyper,
            _ => return None,
        })
    }

    /// For players: "ARAM", "Duel Sandbox", ...
    pub fn label(self) -> &'static str {
        match self {
            GameMode::Empty => "Open sandbox",
            GameMode::Minions => "Minion sandbox",
            GameMode::Dodge => "Dodge rig",
            GameMode::Duel => "Duel Sandbox",
            GameMode::Aram => "ARAM",
            GameMode::Mayhem => "ARAM: Mayhem",
            GameMode::Hyper => "ARAM: Mayhem (Hyper)",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RejectReason {
    ProtocolMismatch = 0,
    ServerFull = 1,
}

// ---- encoding helpers -------------------------------------------------------------------

fn write_qpoint(w: &mut BitWriter, q: QPoint) {
    w.write_u16(q.x);
    w.write_u16(q.y);
}

fn read_qpoint(r: &mut BitReader) -> Result<QPoint, DecodeError> {
    Ok(QPoint { x: r.read_u16()?, y: r.read_u16()? })
}

fn write_command(w: &mut BitWriter, c: &Command) {
    w.write_u32(c.seq);
    w.write_u32(c.tick.0);
    w.write(c.sub.get() as u64, 6);
    match c.kind {
        CommandKind::MoveTo(q) => {
            w.write(0, 4);
            write_qpoint(w, q);
        }
        CommandKind::Stop => w.write(1, 4),
        CommandKind::Cast { slot, target } => {
            w.write(2, 4);
            w.write(slot as u64, 3);
            write_qpoint(w, target);
        }
        CommandKind::Attack(id) => {
            w.write(3, 4);
            w.write_u32(id.0);
        }
        CommandKind::AttackMove(q) => {
            w.write(4, 4);
            write_qpoint(w, q);
        }
        CommandKind::LevelUp(slot) => {
            w.write(5, 4);
            w.write(slot as u64, 2);
        }
        CommandKind::Buy(item) => {
            w.write(6, 4);
            w.write_u8(item);
        }
        CommandKind::Sell(slot) => {
            w.write(7, 4);
            w.write(slot as u64, 3);
        }
        CommandKind::Undo => w.write(8, 4),
        CommandKind::PickAugment(choice) => {
            w.write(9, 4);
            w.write(choice as u64, 2);
        }
        CommandKind::RerollAugment(choice) => {
            w.write(10, 4);
            w.write(choice as u64, 2);
        }
        CommandKind::UseItem(slot) => {
            w.write(11, 4);
            w.write(slot as u64, 3);
        }
    }
}

fn read_command(r: &mut BitReader) -> Result<Command, DecodeError> {
    let seq = r.read_u32()?;
    let tick = Tick(r.read_u32()?);
    let sub = SubTick::new(r.read(6)? as u8);
    let kind = match r.read(4)? {
        0 => CommandKind::MoveTo(read_qpoint(r)?),
        1 => CommandKind::Stop,
        2 => {
            let slot = r.read(3)? as u8;
            if slot as usize >= SLOTS {
                return Err(DecodeError::Invalid("ability slot"));
            }
            CommandKind::Cast { slot, target: read_qpoint(r)? }
        }
        3 => CommandKind::Attack(UnitId(r.read_u32()?)),
        4 => CommandKind::AttackMove(read_qpoint(r)?),
        5 => CommandKind::LevelUp(r.read(2)? as u8),
        6 => CommandKind::Buy(r.read_u8()?),
        7 => CommandKind::Sell(r.read(3)? as u8),
        8 => CommandKind::Undo,
        9 => CommandKind::PickAugment(r.read(2)? as u8),
        10 => CommandKind::RerollAugment(r.read(2)? as u8),
        11 => CommandKind::UseItem(r.read(3)? as u8),
        _ => return Err(DecodeError::Invalid("command kind")),
    };
    Ok(Command { player: PlayerId(0), seq, tick, sub, kind })
}

fn write_time(w: &mut BitWriter, t: SimTime) {
    let (ticks, sub) = t.split();
    w.write_u32(ticks);
    w.write(sub as u64, 6);
}

fn read_time(r: &mut BitReader) -> Result<SimTime, DecodeError> {
    let ticks = r.read_u32()?;
    Ok(SimTime::join(ticks, r.read(6)? as u8))
}

fn write_opt_time(w: &mut BitWriter, t: Option<SimTime>) {
    w.write_bool(t.is_some());
    if let Some(t) = t {
        write_time(w, t);
    }
}

fn read_opt_time(r: &mut BitReader) -> Result<Option<SimTime>, DecodeError> {
    Ok(if r.read_bool()? { Some(read_time(r)?) } else { None })
}

fn write_duration(w: &mut BitWriter, d: SimDuration) {
    w.write_u32(d.0.min(u32::MAX as u64) as u32);
}

fn read_duration(r: &mut BitReader) -> Result<SimDuration, DecodeError> {
    Ok(SimDuration(r.read_u32()? as u64))
}

/// A finite f32 (untrusted input never yields NaN or infinities in the sim).
fn read_finite(r: &mut BitReader) -> Result<f32, DecodeError> {
    let v = r.read_f32()?;
    if v.is_finite() { Ok(v) } else { Err(DecodeError::Invalid("non-finite number")) }
}

fn write_vec2(w: &mut BitWriter, v: Vec2) {
    w.write_f32(v.x);
    w.write_f32(v.y);
}

fn read_vec2(r: &mut BitReader) -> Result<Vec2, DecodeError> {
    let v = Vec2::new(r.read_f32()?, r.read_f32()?);
    if v.x.is_finite() && v.y.is_finite() { Ok(v) } else { Err(DecodeError::Invalid("vector")) }
}

fn write_team(w: &mut BitWriter, t: Team) {
    w.write_bool(t == Team::Red);
}

fn read_team(r: &mut BitReader) -> Result<Team, DecodeError> {
    Ok(if r.read_bool()? { Team::Red } else { Team::Blue })
}

fn write_damage_kind(w: &mut BitWriter, k: DamageKind) {
    w.write(k as u64, 2);
}

fn read_damage_kind(r: &mut BitReader) -> Result<DamageKind, DecodeError> {
    Ok(match r.read(2)? {
        0 => DamageKind::Physical,
        1 => DamageKind::Magic,
        2 => DamageKind::True,
        _ => return Err(DecodeError::Invalid("damage kind")),
    })
}

fn write_cc(w: &mut BitWriter, cc: Cc) {
    match cc {
        Cc::None => w.write(0, 3),
        Cc::Stun(d) => {
            w.write(1, 3);
            write_duration(w, d);
        }
        Cc::Root(d) => {
            w.write(2, 3);
            write_duration(w, d);
        }
        Cc::Knockup(d) => {
            w.write(3, 3);
            write_duration(w, d);
        }
        Cc::Pull(stop) => {
            w.write(4, 3);
            w.write_u16(stop);
        }
        Cc::Slow { pct, duration } => {
            w.write(5, 3);
            w.write(pct as u64, 7);
            write_duration(w, duration);
        }
    }
}

fn read_cc(r: &mut BitReader) -> Result<Cc, DecodeError> {
    Ok(match r.read(3)? {
        0 => Cc::None,
        1 => Cc::Stun(read_duration(r)?),
        2 => Cc::Root(read_duration(r)?),
        3 => Cc::Knockup(read_duration(r)?),
        4 => Cc::Pull(r.read_u16()?),
        5 => Cc::Slow { pct: r.read(7)? as u8, duration: read_duration(r)? },
        _ => return Err(DecodeError::Invalid("cc kind")),
    })
}

fn write_line_spec(w: &mut BitWriter, s: &LineSkillshot) {
    write_duration(w, s.windup);
    w.write_f32(s.speed);
    w.write_f32(s.radius);
    w.write_f32(s.range);
    write_damage_kind(w, s.damage.kind);
    w.write_f32(s.damage.base);
    w.write_f32(s.damage.ad_ratio);
    w.write_f32(s.damage.ap_ratio);
    write_cc(w, s.cc);
}

fn read_line_spec(r: &mut BitReader) -> Result<LineSkillshot, DecodeError> {
    let windup = read_duration(r)?;
    let (speed, radius, range) = (read_finite(r)?, read_finite(r)?, read_finite(r)?);
    if speed <= 0.0 {
        return Err(DecodeError::Invalid("missile spec"));
    }
    let kind = read_damage_kind(r)?;
    let damage = Damage { kind, base: read_finite(r)?, ad_ratio: read_finite(r)?, ap_ratio: read_finite(r)? };
    let cc = read_cc(r)?;
    Ok(LineSkillshot { windup, speed, radius, range, damage, cc })
}

fn write_missile(w: &mut BitWriter, m: &Missile) {
    w.write_u32(m.id);
    w.write_u32(m.owner.0);
    write_team(w, m.team);
    write_vec2(w, m.origin);
    write_vec2(w, m.dir);
    write_line_spec(w, &m.spec);
    write_time(w, m.spawn_at);
    w.write_u32(m.cast_seq);
    w.write_f32(m.power);
    w.write(m.shot as u64, 4);
    w.write(m.slot as u64, 3);
}

fn read_missile(r: &mut BitReader) -> Result<Missile, DecodeError> {
    Ok(Missile {
        id: r.read_u32()?,
        owner: UnitId(r.read_u32()?),
        team: read_team(r)?,
        origin: read_vec2(r)?,
        dir: read_vec2(r)?,
        spec: read_line_spec(r)?,
        spawn_at: read_time(r)?,
        cast_seq: r.read_u32()?,
        power: read_finite(r)?,
        shot: r.read(4)? as u8,
        slot: r.read(3)? as u8,
    })
}

/// Event-list record kind for a volley: several missiles from one cast (Multishot) share all
/// fields but the direction and shot index, so one record carries them (06 §3).
const VOLLEY: u64 = 17;
/// Most missiles in one volley record.
const MAX_VOLLEY: usize = 8;

/// How many events from `i` on form one volley: consecutive missile spawns from the same cast,
/// instant, origin and spec, with consecutive ids and event sequences.
fn volley_len(events: &[(u32, SimEvent)], i: usize) -> usize {
    let (seq, SimEvent::MissileSpawned(first)) = events[i] else { return 1 };
    let mut n = 1;
    while n < MAX_VOLLEY
        && let Some((s, SimEvent::MissileSpawned(m))) = events.get(i + n)
        && *s == seq.wrapping_add(n as u32)
        && m.id == first.id.wrapping_add(n as u32)
        && (Missile { dir: first.dir, shot: first.shot, id: first.id, ..*m }) == first
    {
        n += 1;
    }
    n
}

/// What dealt a hit: 3 bits of kind, then a slot (3 bits) or an item or augment id (8).
fn write_origin(w: &mut BitWriter, o: DamageOrigin) {
    match o {
        DamageOrigin::Attack => w.write(0, 3),
        DamageOrigin::Ability(slot) => {
            w.write(1, 3);
            w.write(slot as u64, 3);
        }
        DamageOrigin::Item(id) => {
            w.write(2, 3);
            w.write_u8(id);
        }
        DamageOrigin::Augment(id) => {
            w.write(3, 3);
            w.write_u8(id);
        }
        DamageOrigin::Fountain => w.write(4, 3),
    }
}

fn read_origin(r: &mut BitReader) -> Result<DamageOrigin, DecodeError> {
    Ok(match r.read(3)? {
        0 => DamageOrigin::Attack,
        1 => DamageOrigin::Ability(r.read(3)? as u8),
        2 => DamageOrigin::Item(r.read_u8()?),
        3 => DamageOrigin::Augment(r.read_u8()?),
        4 => DamageOrigin::Fountain,
        _ => return Err(DecodeError::Invalid("damage origin")),
    })
}

fn write_event(w: &mut BitWriter, e: &SimEvent) {
    match e {
        SimEvent::CastStarted { unit, slot, at, dir, point, fire_at, seq } => {
            w.write(0, 5);
            w.write_u32(unit.0);
            w.write(*slot as u64, 3);
            write_time(w, *at);
            write_vec2(w, *dir);
            write_vec2(w, *point);
            write_time(w, *fire_at);
            w.write_u32(*seq);
        }
        SimEvent::MissileSpawned(m) => {
            w.write(1, 5);
            write_missile(w, m);
        }
        SimEvent::MissileHit { id, target, at } => {
            w.write(2, 5);
            w.write_u32(*id);
            w.write_u32(target.0);
            write_time(w, *at);
        }
        SimEvent::MissileExpired { id, at } => {
            w.write(3, 5);
            w.write_u32(*id);
            write_time(w, *at);
        }
        SimEvent::AreaSpawned(a) => {
            w.write(4, 5);
            w.write_u32(a.id);
            w.write_u32(a.owner.0);
            write_team(w, a.team);
            write_vec2(w, a.center);
            w.write_f32(a.radius);
            write_time(w, a.spawn_at);
            write_time(w, a.detonate_at);
            write_damage_kind(w, a.kind);
            w.write_f32(a.power);
            w.write_u32(a.cast_seq);
            write_cc(w, a.cc);
            w.write(a.shot as u64, 4);
            w.write(a.slot as u64, 3);
        }
        SimEvent::AreaDetonated { id, at } => {
            w.write(5, 5);
            w.write_u32(*id);
            write_time(w, *at);
        }
        SimEvent::AttackLaunched(b) => {
            w.write(6, 5);
            w.write_u32(b.id);
            w.write_u32(b.owner.0);
            write_team(w, b.team);
            w.write_u32(b.target.0);
            write_vec2(w, b.origin);
            write_vec2(w, b.pos);
            w.write_f32(b.speed);
            write_time(w, b.launched_at);
            w.write_f32(b.power);
            write_damage_kind(w, b.kind);
        }
        SimEvent::AttackLanded { id, target, at, hit } => {
            w.write(7, 5);
            w.write_u32(*id);
            w.write_u32(target.0);
            write_time(w, *at);
            w.write_bool(*hit);
        }
        SimEvent::Damage { source, target, origin, kind, amount, absorbed, at } => {
            w.write(8, 5);
            w.write_u32(source.0);
            w.write_u32(target.0);
            write_origin(w, *origin);
            write_damage_kind(w, *kind);
            w.write_f32(*amount);
            w.write_f32(*absorbed);
            write_time(w, *at);
        }
        SimEvent::Died { unit, killer, at, respawn_at } => {
            w.write(9, 5);
            w.write_u32(unit.0);
            w.write_u32(killer.0);
            write_time(w, *at);
            write_time(w, *respawn_at);
        }
        SimEvent::Respawned { unit, pos, at } => {
            w.write(10, 5);
            w.write_u32(unit.0);
            write_vec2(w, *pos);
            write_time(w, *at);
        }
        SimEvent::Blinked { unit, from, to, at } => {
            w.write(11, 5);
            w.write_u32(unit.0);
            write_vec2(w, *from);
            write_vec2(w, *to);
            write_time(w, *at);
        }
        SimEvent::Dashed { unit, from, to, at, end_at } => {
            w.write(12, 5);
            w.write_u32(unit.0);
            write_vec2(w, *from);
            write_vec2(w, *to);
            write_time(w, *at);
            write_time(w, *end_at);
        }
        SimEvent::Shielded { unit, amount, at, until } => {
            w.write(13, 5);
            w.write_u32(unit.0);
            w.write_f32(*amount);
            write_time(w, *at);
            write_time(w, *until);
        }
        SimEvent::Healed { unit, amount, at } => {
            w.write(14, 5);
            w.write_u32(unit.0);
            w.write_f32(*amount);
            write_time(w, *at);
        }
        SimEvent::MatchEnded { winner, at } => {
            w.write(15, 5);
            write_team(w, *winner);
            write_time(w, *at);
        }
        SimEvent::Reward { unit, gold, xp, at } => {
            w.write(16, 5);
            w.write_u32(unit.0);
            w.write_f32(*gold);
            w.write_u32(*xp);
            write_time(w, *at);
        }
    }
}

fn read_event_of(kind: u64, r: &mut BitReader) -> Result<SimEvent, DecodeError> {
    let unit = |r: &mut BitReader| -> Result<UnitId, DecodeError> { Ok(UnitId(r.read_u32()?)) };
    Ok(match kind {
        0 => SimEvent::CastStarted {
            unit: unit(r)?,
            slot: r.read(3)? as u8,
            at: read_time(r)?,
            dir: read_vec2(r)?,
            point: read_vec2(r)?,
            fire_at: read_time(r)?,
            seq: r.read_u32()?,
        },
        1 => SimEvent::MissileSpawned(read_missile(r)?),
        2 => SimEvent::MissileHit { id: r.read_u32()?, target: unit(r)?, at: read_time(r)? },
        3 => SimEvent::MissileExpired { id: r.read_u32()?, at: read_time(r)? },
        4 => SimEvent::AreaSpawned(Area {
            id: r.read_u32()?,
            owner: unit(r)?,
            team: read_team(r)?,
            center: read_vec2(r)?,
            radius: read_finite(r)?,
            spawn_at: read_time(r)?,
            detonate_at: read_time(r)?,
            kind: read_damage_kind(r)?,
            power: read_finite(r)?,
            cast_seq: r.read_u32()?,
            cc: read_cc(r)?,
            shot: r.read(4)? as u8,
            slot: r.read(3)? as u8,
        }),
        5 => SimEvent::AreaDetonated { id: r.read_u32()?, at: read_time(r)? },
        6 => SimEvent::AttackLaunched(Bolt {
            id: r.read_u32()?,
            owner: unit(r)?,
            team: read_team(r)?,
            target: unit(r)?,
            origin: read_vec2(r)?,
            pos: read_vec2(r)?,
            speed: read_finite(r)?,
            launched_at: read_time(r)?,
            power: read_finite(r)?,
            kind: read_damage_kind(r)?,
        }),
        7 => SimEvent::AttackLanded { id: r.read_u32()?, target: unit(r)?, at: read_time(r)?, hit: r.read_bool()? },
        8 => SimEvent::Damage {
            source: unit(r)?,
            target: unit(r)?,
            origin: read_origin(r)?,
            kind: read_damage_kind(r)?,
            amount: read_finite(r)?,
            absorbed: read_finite(r)?,
            at: read_time(r)?,
        },
        9 => SimEvent::Died { unit: unit(r)?, killer: unit(r)?, at: read_time(r)?, respawn_at: read_time(r)? },
        10 => SimEvent::Respawned { unit: unit(r)?, pos: read_vec2(r)?, at: read_time(r)? },
        11 => SimEvent::Blinked { unit: unit(r)?, from: read_vec2(r)?, to: read_vec2(r)?, at: read_time(r)? },
        12 => SimEvent::Dashed {
            unit: unit(r)?,
            from: read_vec2(r)?,
            to: read_vec2(r)?,
            at: read_time(r)?,
            end_at: read_time(r)?,
        },
        13 => SimEvent::Shielded { unit: unit(r)?, amount: read_finite(r)?, at: read_time(r)?, until: read_time(r)? },
        14 => SimEvent::Healed { unit: unit(r)?, amount: read_finite(r)?, at: read_time(r)? },
        15 => SimEvent::MatchEnded { winner: read_team(r)?, at: read_time(r)? },
        16 => SimEvent::Reward { unit: unit(r)?, gold: read_finite(r)?, xp: r.read_u32()?, at: read_time(r)? },
        _ => return Err(DecodeError::Invalid("event kind")),
    })
}

fn write_order(w: &mut BitWriter, o: Order) {
    match o {
        Order::Idle => w.write(0, 2),
        Order::MoveTo(q) => {
            w.write(1, 2);
            write_qpoint(w, q);
        }
        Order::Attack(id) => {
            w.write(2, 2);
            w.write_u32(id.0);
        }
        Order::AttackMove(q) => {
            w.write(3, 2);
            write_qpoint(w, q);
        }
    }
}

fn read_order(r: &mut BitReader) -> Result<Order, DecodeError> {
    Ok(match r.read(2)? {
        0 => Order::Idle,
        1 => Order::MoveTo(read_qpoint(r)?),
        2 => Order::Attack(UnitId(r.read_u32()?)),
        _ => Order::AttackMove(read_qpoint(r)?),
    })
}

fn write_unit_state(w: &mut BitWriter, s: &UnitState) {
    write_vec2(w, s.pos);
    w.write_f32(s.move_speed);
    write_order(w, s.order);
    w.write(s.path.len as u64, 4);
    w.write(s.path.next as u64, 4);
    w.write_bool(s.path.complete);
    for p in &s.path.points[..s.path.len as usize] {
        write_vec2(w, *p);
    }
    w.write_bool(s.detour.is_some());
    if let Some(d) = s.detour {
        write_vec2(w, d);
    }
    w.write_u8(s.stuck);
    w.write_bool(s.cast.is_some());
    if let Some(c) = s.cast {
        w.write(c.slot as u64, 3);
        write_vec2(w, c.dir);
        write_vec2(w, c.point);
        write_time(w, c.fire_at);
        w.write_u32(c.seq);
        w.write_bool(c.mobile);
    }
    w.write_bool(s.attack.is_some());
    if let Some(a) = s.attack {
        w.write_u32(a.target.0);
        write_time(w, a.fire_at);
    }
    write_time(w, s.attack_ready_at);
    w.write_bool(s.dash.is_some());
    if let Some(d) = s.dash {
        write_vec2(w, d.dir);
        write_vec2(w, d.to);
        w.write_f32(d.speed);
        write_time(w, d.end_at);
        w.write_bool(d.strike.is_some());
        if let Some((id, slot)) = d.strike {
            w.write_u32(id.0);
            w.write(slot as u64, 3);
        }
        w.write_u32(d.recover.0 as u32);
    }
    write_time(w, s.stunned_until);
    write_time(w, s.rooted_until);
    w.write(s.slow as u64, 7);
    write_time(w, s.slowed_until);
    for c in s.cooldowns {
        write_time(w, c);
    }
    w.write_f32(s.health);
    w.write_f32(s.shield);
    write_time(w, s.shield_until);
    write_opt_time(w, s.respawn_at);
    match s.echo {
        None => w.write_bool(false),
        Some(e) => {
            w.write_bool(true);
            w.write(e.slot as u64, 3);
            write_vec2(w, e.dir);
            write_vec2(w, e.point);
            write_time(w, e.at);
            w.write_u32(e.seq);
        }
    }
    let p = &s.progress;
    w.write(p.level as u64, 5);
    w.write_u32(p.xp);
    w.write_f32(p.gold);
    for r in p.ranks {
        w.write(r as u64, 3);
    }
    w.write(p.points as u64, 5);
    w.write_u8(p.streak as u8);
    for i in p.items {
        w.write_u8(i);
    }
    for c in p.charges {
        w.write(c as u64, 4);
    }
    write_time(w, p.lifeline_ready);
    w.write(p.undo_len as u64, 3);
    for t in &p.undo[..p.undo_len as usize] {
        for i in t.items {
            w.write_u8(i);
        }
        for c in t.charges {
            w.write(c as u64, 4);
        }
        w.write_f32(t.gold);
    }
    for a in p.augments.iter().chain(&p.offer) {
        w.write_u8(*a);
    }
    w.write(p.drafted as u64, 3);
    w.write(p.rerolled as u64, 3);
    w.write(p.golden as u64, 2);
    w.write_u32(p.augment_seed);
    w.write_bool(p.unstable_tiny);
    w.write(p.stacks as u64, 16);
    w.write_u8(p.takedowns);
    for n in [p.kills, p.deaths, p.assists, p.cs] {
        w.write(n as u64, 16);
    }
    w.write_bool(p.hyper);
    write_time(w, s.spellblade_until);
    // A2 (D52): facing, the follow-through, the input buffer, the attack counter.
    write_vec2(w, s.facing);
    w.write_bool(s.recovery.is_some());
    if let Some(r) = s.recovery {
        write_time(w, r.hard_until);
        write_time(w, r.until);
    }
    w.write_bool(s.buffered.is_some());
    if let Some(b) = s.buffered {
        w.write(b.slot as u64, 3);
        write_qpoint(w, b.target);
        w.write_u32(b.seq);
    }
    w.write_u8(s.attacks);
    w.write_f32(s.potion_rate);
    write_time(w, s.potion_until);
}

fn read_unit_state(r: &mut BitReader) -> Result<UnitState, DecodeError> {
    let pos = read_vec2(r)?;
    let move_speed = read_finite(r)?;
    let order = read_order(r)?;
    let mut path = Path::EMPTY;
    path.len = r.read(4)? as u8;
    path.next = r.read(4)? as u8;
    path.complete = r.read_bool()?;
    if path.len as usize > MAX_PATH || path.next > path.len {
        return Err(DecodeError::Invalid("path"));
    }
    for i in 0..path.len as usize {
        path.points[i] = read_vec2(r)?;
    }
    let detour = if r.read_bool()? { Some(read_vec2(r)?) } else { None };
    let stuck = r.read_u8()?;
    let cast = if r.read_bool()? {
        Some(Cast {
            slot: r.read(3)? as u8,
            dir: read_vec2(r)?,
            point: read_vec2(r)?,
            fire_at: read_time(r)?,
            seq: r.read_u32()?,
            mobile: r.read_bool()?,
        })
    } else {
        None
    };
    let attack = if r.read_bool()? {
        Some(AttackWindup { target: UnitId(r.read_u32()?), fire_at: read_time(r)? })
    } else {
        None
    };
    let attack_ready_at = read_time(r)?;
    let dash = if r.read_bool()? {
        let (dir, to, speed, end_at) = (read_vec2(r)?, read_vec2(r)?, read_finite(r)?, read_time(r)?);
        let strike = if r.read_bool()? { Some((UnitId(r.read_u32()?), r.read(3)? as u8)) } else { None };
        let recover = SimDuration(r.read_u32()? as u64);
        Some(DashMove { dir, to, speed, end_at, strike, recover })
    } else {
        None
    };
    let stunned_until = read_time(r)?;
    let rooted_until = read_time(r)?;
    let slow = r.read(7)? as u8;
    let slowed_until = read_time(r)?;
    let mut cooldowns = [SimTime(0); SLOTS];
    for c in cooldowns.iter_mut() {
        *c = read_time(r)?;
    }
    let health = read_finite(r)?;
    let shield = read_finite(r)?;
    let shield_until = read_time(r)?;
    let respawn_at = read_opt_time(r)?;
    let echo = if r.read_bool()? {
        Some(EchoCast {
            slot: r.read(3)? as u8,
            dir: read_vec2(r)?,
            point: read_vec2(r)?,
            at: read_time(r)?,
            seq: r.read_u32()?,
        })
    } else {
        None
    };
    let level = r.read(5)? as u8;
    let xp = r.read_u32()?;
    let gold = read_finite(r)?;
    let mut ranks = [0u8; 4];
    for x in ranks.iter_mut() {
        *x = r.read(3)? as u8;
    }
    let points = r.read(5)? as u8;
    let streak = r.read_u8()? as i8;
    let mut items = [0u8; INVENTORY];
    for i in items.iter_mut() {
        *i = r.read_u8()?;
    }
    let mut charges = [0u8; INVENTORY];
    for c in charges.iter_mut() {
        *c = r.read(4)? as u8;
    }
    let lifeline_ready = read_time(r)?;
    let undo_len = r.read(3)? as u8;
    if undo_len as usize > UNDO {
        return Err(DecodeError::Invalid("undo"));
    }
    let mut undo = [Trade { items: [0; INVENTORY], charges: [0; INVENTORY], gold: 0.0 }; UNDO];
    for t in undo.iter_mut().take(undo_len as usize) {
        for i in t.items.iter_mut() {
            *i = r.read_u8()?;
        }
        for c in t.charges.iter_mut() {
            *c = r.read(4)? as u8;
        }
        t.gold = read_finite(r)?;
    }
    let mut augments = [0u8; mftr_sim::augments::SLOTS];
    for a in augments.iter_mut() {
        *a = r.read_u8()?;
    }
    let mut offer = [0u8; mftr_sim::augments::CHOICES];
    for a in offer.iter_mut() {
        *a = r.read_u8()?;
    }
    let drafted = r.read(3)? as u8;
    let rerolled = r.read(3)? as u8;
    let golden = r.read(2)? as u8;
    let augment_seed = r.read_u32()?;
    let unstable_tiny = r.read_bool()?;
    let stacks = r.read(16)? as u16;
    let takedowns = r.read_u8()?;
    let (kills, deaths, assists, cs) = (r.read(16)? as u16, r.read(16)? as u16, r.read(16)? as u16, r.read(16)? as u16);
    let hyper = r.read_bool()?;
    let spellblade_until = read_time(r)?;
    let facing = read_vec2(r)?;
    let recovery =
        if r.read_bool()? { Some(Recovery { hard_until: read_time(r)?, until: read_time(r)? }) } else { None };
    let buffered = if r.read_bool()? {
        Some(BufferedCast { slot: r.read(3)? as u8, target: read_qpoint(r)?, seq: r.read_u32()? })
    } else {
        None
    };
    let attacks = r.read_u8()?;
    let (potion_rate, potion_until) = (read_finite(r)?, read_time(r)?);
    let progress = Progress {
        level,
        xp,
        gold,
        ranks,
        points,
        streak,
        items,
        charges,
        lifeline_ready,
        undo,
        undo_len,
        augments,
        offer,
        drafted,
        rerolled,
        golden,
        augment_seed,
        unstable_tiny,
        stacks,
        takedowns,
        hyper,
        kills,
        deaths,
        assists,
        cs,
    };
    Ok(UnitState {
        pos,
        order,
        move_speed,
        path,
        detour,
        stuck,
        cast,
        attack,
        attack_ready_at,
        dash,
        stunned_until,
        rooted_until,
        slow,
        slowed_until,
        cooldowns,
        health,
        shield,
        shield_until,
        respawn_at,
        progress,
        echo,
        spellblade_until,
        facing,
        recovery,
        buffered,
        attacks,
        potion_rate,
        potion_until,
    })
}

fn minion_to_wire(k: MinionKind) -> u64 {
    match k {
        MinionKind::Melee => 0,
        MinionKind::Caster => 1,
        MinionKind::Siege => 2,
        MinionKind::Super => 3,
    }
}

fn minion_from_wire(v: u64) -> MinionKind {
    match v {
        0 => MinionKind::Melee,
        1 => MinionKind::Caster,
        2 => MinionKind::Siege,
        _ => MinionKind::Super,
    }
}

/// Five bits: room for 31 champions (31 means none).
fn write_champion(w: &mut BitWriter, c: Option<ChampionId>) {
    w.write(c.map_or(31, |c| c as u64), 5);
}

fn read_champion(r: &mut BitReader) -> Result<Option<ChampionId>, DecodeError> {
    match r.read(5)? {
        31 => Ok(None),
        v => ChampionId::from_u8(v as u8).map(Some).ok_or(DecodeError::Invalid("champion")),
    }
}

fn write_update(w: &mut BitWriter, u: &UnitUpdate) {
    let o = &u.unit;
    w.write_u32(o.id.0);
    w.write(u.mask as u64, 5);
    if u.mask & delta::STATIC != 0 {
        w.write(o.kind.wire() as u64, 3);
        write_team(w, o.team);
        w.write_u8(o.collision_radius);
        w.write_u8(o.gameplay_radius);
        write_champion(w, o.champion);
        if o.kind == UnitKind::Minion {
            w.write(minion_to_wire(o.minion.unwrap_or(MinionKind::Melee)), 2);
        }
        if o.champion.is_some() {
            let held = o.augments != [0; mftr_sim::augments::SLOTS];
            w.write_bool(held);
            if held {
                for a in o.augments {
                    w.write_u8(a);
                }
            }
        }
    }
    if u.mask & delta::POS != 0 {
        write_qpoint(w, o.pos);
    }
    if u.mask & delta::MOTION != 0 {
        w.write_bool(o.target.is_some());
        if let Some(t) = o.target {
            write_qpoint(w, t);
        }
        w.write(o.speed.min(1023) as u64, 10);
        w.write((o.facing & 1023) as u64, 10);
    }
    if u.mask & delta::VITALS != 0 {
        w.write_u16(o.health);
        w.write_u16(o.max_health);
        w.write_u16(o.shield);
        w.write(o.level as u64, 5);
    }
    if u.mask & delta::FLAGS != 0 {
        for f in [o.casting, o.attacking, o.stunned, o.rooted, o.dashing, o.protected, o.slowed, o.recovering] {
            w.write_bool(f);
        }
        w.write((o.attack_variant & 3) as u64, 2);
    }
}

/// An update; groups not in the mask are left at defaults (the client takes them from its
/// baseline, `delta::apply`).
fn read_update(r: &mut BitReader) -> Result<UnitUpdate, DecodeError> {
    let id = UnitId(r.read_u32()?);
    let mask = r.read(5)? as u8;
    let mut o = RemoteUnit {
        id,
        kind: UnitKind::Minion,
        team: Team::Blue,
        pos: QPoint::default(),
        target: None,
        speed: 0,
        collision_radius: 0,
        gameplay_radius: 0,
        protected: false,
        champion: None,
        minion: None,
        augments: [0; mftr_sim::augments::SLOTS],
        health: 0,
        max_health: 0,
        shield: 0,
        level: 0,
        casting: false,
        attacking: false,
        stunned: false,
        rooted: false,
        dashing: false,
        slowed: false,
        recovering: false,
        attack_variant: 0,
        facing: 0,
    };
    if mask & delta::STATIC != 0 {
        o.kind = UnitKind::from_wire(r.read(3)? as u8).ok_or(DecodeError::Invalid("unit kind"))?;
        o.team = read_team(r)?;
        o.collision_radius = r.read_u8()?;
        o.gameplay_radius = r.read_u8()?;
        o.champion = read_champion(r)?;
        if o.kind == UnitKind::Minion {
            o.minion = Some(minion_from_wire(r.read(2)?));
        }
        if o.champion.is_some() && r.read_bool()? {
            for a in o.augments.iter_mut() {
                *a = r.read_u8()?;
            }
        }
    }
    if mask & delta::POS != 0 {
        o.pos = read_qpoint(r)?;
    }
    if mask & delta::MOTION != 0 {
        o.target = if r.read_bool()? { Some(read_qpoint(r)?) } else { None };
        o.speed = r.read(10)? as u16;
        o.facing = r.read(10)? as u16;
    }
    if mask & delta::VITALS != 0 {
        (o.health, o.max_health, o.shield) = (r.read_u16()?, r.read_u16()?, r.read_u16()?);
        o.level = r.read(5)? as u8;
    }
    if mask & delta::FLAGS != 0 {
        let mut f = [false; 8];
        for b in f.iter_mut() {
            *b = r.read_bool()?;
        }
        [o.casting, o.attacking, o.stunned, o.rooted, o.dashing, o.protected, o.slowed, o.recovering] = f;
        o.attack_variant = r.read(2)? as u8;
    }
    Ok(UnitUpdate { mask, unit: o })
}

fn write_echo(w: &mut BitWriter, e: &TimeEcho) {
    w.write_u32(e.client_time_us);
    w.write_u32(e.hold_us);
}

fn read_echo(r: &mut BitReader) -> Result<TimeEcho, DecodeError> {
    Ok(TimeEcho { client_time_us: r.read_u32()?, hold_us: r.read_u32()? })
}

fn begin(header: &PacketHeader, kind: u64) -> BitWriter {
    let mut w = BitWriter::new();
    w.write_u8(MAGIC);
    header.write(&mut w);
    w.write(kind, 2);
    w
}

fn open(bytes: &[u8]) -> Result<(BitReader<'_>, PacketHeader, u64), DecodeError> {
    let mut r = BitReader::new(bytes);
    if r.read_u8()? != MAGIC {
        return Err(DecodeError::Invalid("magic"));
    }
    let header = PacketHeader::read(&mut r)?;
    let kind = r.read(2)?;
    Ok((r, header, kind))
}

// ---- client → server --------------------------------------------------------------------

pub fn encode_client(header: &PacketHeader, msg: &ClientMessage) -> Vec<u8> {
    match msg {
        ClientMessage::Hello { protocol, client_time_us, champion, resume, spectate } => {
            let mut w = begin(header, 0);
            w.write_u16(*protocol);
            w.write_u32(*client_time_us);
            write_champion(&mut w, *champion);
            w.write_u64(*resume);
            w.write_bool(*spectate);
            w.finish()
        }
        ClientMessage::Input { client_time_us, event_ack, snapshot_ack, commands } => {
            let mut w = begin(header, 1);
            w.write_u32(*client_time_us);
            w.write_u32(*event_ack);
            w.write_u32(*snapshot_ack);
            let n = commands.len().min(MAX_COMMANDS_PER_PACKET);
            w.write(n as u64, 4);
            for c in &commands[commands.len() - n..] {
                write_command(&mut w, c);
            }
            w.finish()
        }
        ClientMessage::Bye => begin(header, 2).finish(),
        ClientMessage::Lobby(a) => {
            let mut w = begin(header, 3);
            match a {
                LobbyAction::Reroll => w.write(0, 2),
                LobbyAction::Take(c) => {
                    w.write(1, 2);
                    write_champion(&mut w, Some(*c));
                }
                LobbyAction::Ready(r) => {
                    w.write(2, 2);
                    w.write_bool(*r);
                }
            }
            w.finish()
        }
    }
}

pub fn decode_client(bytes: &[u8]) -> Result<(PacketHeader, ClientMessage), DecodeError> {
    let (mut r, header, kind) = open(bytes)?;
    let msg = match kind {
        0 => {
            let protocol = r.read_u16()?;
            let client_time_us = r.read_u32()?;
            // An older client stops after the clock: nothing more to read (it gets a reject).
            let current = protocol == crate::PROTOCOL_VERSION;
            let champion = if current { read_champion(&mut r)? } else { None };
            let resume = if current { r.read_u64()? } else { 0 };
            let spectate = current && r.read_bool()?;
            ClientMessage::Hello { protocol, client_time_us, champion, resume, spectate }
        }
        1 => {
            let client_time_us = r.read_u32()?;
            let event_ack = r.read_u32()?;
            let snapshot_ack = r.read_u32()?;
            let n = r.read(4)? as usize;
            if n > MAX_COMMANDS_PER_PACKET {
                return Err(DecodeError::Invalid("command count"));
            }
            let commands = (0..n).map(|_| read_command(&mut r)).collect::<Result<_, _>>()?;
            ClientMessage::Input { client_time_us, event_ack, snapshot_ack, commands }
        }
        2 => ClientMessage::Bye,
        3 => ClientMessage::Lobby(match r.read(2)? {
            0 => LobbyAction::Reroll,
            1 => LobbyAction::Take(read_champion(&mut r)?.ok_or(DecodeError::Invalid("champion"))?),
            2 => LobbyAction::Ready(r.read_bool()?),
            _ => return Err(DecodeError::Invalid("lobby action")),
        }),
        _ => unreachable!("two-bit message kind"),
    };
    Ok((header, msg))
}

fn write_scoreboard(w: &mut BitWriter, b: &Scoreboard) {
    write_time(w, b.started_at);
    w.write(b.rows.len().min(MAX_SCORE_ROWS) as u64, 5);
    for row in b.rows.iter().take(MAX_SCORE_ROWS) {
        w.write_u32(row.unit.0);
        write_champion(w, Some(row.champion));
        write_team(w, row.team);
        w.write_bool(row.bot);
        w.write(row.level as u64, 5);
        for n in [row.kills, row.deaths, row.assists, row.cs] {
            w.write(n as u64, 16);
        }
        for i in row.items {
            w.write_u8(i);
        }
        for a in row.augments {
            w.write_u8(a);
        }
        w.write(row.respawn_ds as u64, 16);
    }
}

fn read_scoreboard(r: &mut BitReader) -> Result<Scoreboard, DecodeError> {
    let started_at = read_time(r)?;
    let n = r.read(5)? as usize;
    if n > MAX_SCORE_ROWS {
        return Err(DecodeError::Invalid("score rows"));
    }
    let mut rows = Vec::with_capacity(n);
    for _ in 0..n {
        let unit = UnitId(r.read_u32()?);
        let champion = read_champion(r)?.ok_or(DecodeError::Invalid("champion"))?;
        let team = read_team(r)?;
        let bot = r.read_bool()?;
        let level = r.read(5)? as u8;
        let (kills, deaths, assists, cs) =
            (r.read(16)? as u16, r.read(16)? as u16, r.read(16)? as u16, r.read(16)? as u16);
        let mut items = [0u8; INVENTORY];
        for i in items.iter_mut() {
            *i = r.read_u8()?;
        }
        let mut augments = [0u8; mftr_sim::augments::SLOTS];
        for a in augments.iter_mut() {
            *a = r.read_u8()?;
        }
        let respawn_ds = r.read(16)? as u16;
        rows.push(ScoreRow {
            unit,
            champion,
            team,
            bot,
            level,
            kills,
            deaths,
            assists,
            cs,
            items,
            augments,
            respawn_ds,
        });
    }
    Ok(Scoreboard { started_at, rows })
}

// ---- server → client --------------------------------------------------------------------

pub fn encode_server(header: &PacketHeader, msg: &ServerMessage) -> Vec<u8> {
    match msg {
        ServerMessage::Welcome {
            player,
            unit,
            team,
            map,
            champion,
            home,
            rules,
            tick,
            tick_hz,
            since_tick_us,
            time_echo,
            token,
            spectator,
            mode,
        } => {
            let mut w = begin(header, 0);
            w.write_u8(player.0);
            w.write_u32(unit.0);
            w.write_bool(*team == Team::Red);
            w.write_u8(*map as u8);
            write_champion(&mut w, Some(*champion));
            write_vec2(&mut w, *home);
            w.write(rules.start_level as u64, 5);
            w.write_f32(rules.start_gold);
            w.write_f32(rules.passive_gold);
            w.write_bool(rules.ranked);
            w.write_bool(rules.augments);
            w.write_bool(rules.hyper);
            w.write_u32(tick.0);
            w.write_u8(*tick_hz);
            w.write_u32(*since_tick_us);
            write_echo(&mut w, time_echo);
            w.write_u64(*token);
            w.write_bool(*spectator);
            w.write(*mode as u64, 3);
            w.finish()
        }
        ServerMessage::Snapshot(s) => {
            let mut w = begin(header, 1);
            w.write_u32(s.tick.0);
            w.write_u32(s.since_tick_us);
            w.write_bool(s.time_echo.is_some());
            if let Some(e) = &s.time_echo {
                write_echo(&mut w, e);
            }
            w.write_u32(s.last_cmd_seq);
            let n = s.reports.len().min(MAX_REPORTS_PER_SNAPSHOT);
            w.write(n as u64, 4);
            for rep in &s.reports[s.reports.len() - n..] {
                w.write_u32(rep.seq);
                w.write_u32(rep.applied_tick.0);
                w.write(rep.applied_sub.get() as u64, 6);
                let half_ms = (rep.lead_us as f64 / 500.0).round().clamp(i16::MIN as f64, i16::MAX as f64) as i16;
                w.write_i16(half_ms);
            }
            w.write_bool(s.own.is_some());
            if let Some((id, st)) = &s.own {
                w.write_u32(id.0);
                write_unit_state(&mut w, st);
            }
            w.write_bool(s.baseline.is_some());
            if let Some(b) = s.baseline {
                w.write_u32(b.0);
            }
            let n = s.others.len().min(MAX_UNITS_PER_SNAPSHOT);
            w.write_u16(n as u16);
            for o in &s.others[..n] {
                write_update(&mut w, o);
            }
            let n = s.removed.len().min(MAX_UNITS_PER_SNAPSHOT);
            w.write_u16(n as u16);
            for id in &s.removed[..n] {
                w.write_u32(id.0);
            }
            let n = s.events.len().min(MAX_EVENTS_PER_SNAPSHOT);
            let events = &s.events[..n];
            let mut records = Vec::new();
            let mut i = 0;
            while i < n {
                let len = volley_len(events, i);
                records.push((i, len));
                i += len;
            }
            w.write(records.len() as u64, 6);
            for (i, len) in records {
                let (seq, e) = &events[i];
                w.write_u32(*seq);
                match e {
                    SimEvent::MissileSpawned(first) if len > 1 => {
                        w.write(VOLLEY, 5);
                        write_missile(&mut w, first);
                        w.write((len - 1) as u64, 3);
                        for (_, extra) in &events[i + 1..i + len] {
                            if let SimEvent::MissileSpawned(m) = extra {
                                write_vec2(&mut w, m.dir);
                                w.write(m.shot as u64, 4);
                            }
                        }
                    }
                    _ => write_event(&mut w, e),
                }
            }
            w.write_bool(s.scoreboard.is_some());
            if let Some(b) = &s.scoreboard {
                write_scoreboard(&mut w, b);
            }
            w.finish()
        }
        ServerMessage::Reject { reason } => {
            let mut w = begin(header, 2);
            w.write_u8(*reason as u8);
            w.finish()
        }
        ServerMessage::Lobby(l) => {
            let mut w = begin(header, 3);
            w.write_u8(l.you.0);
            w.write(l.slots.len().min(MAX_LOBBY_SLOTS) as u64, 5);
            for s in l.slots.iter().take(MAX_LOBBY_SLOTS) {
                w.write_u8(s.player.0);
                w.write_bool(s.team == Team::Red);
                write_champion(&mut w, Some(s.champion));
                w.write(s.rerolls as u64, 3);
                w.write_bool(s.ready);
                w.write_bool(s.bot);
            }
            w.write(l.bench.len().min(MAX_LOBBY_SLOTS) as u64, 5);
            for c in l.bench.iter().take(MAX_LOBBY_SLOTS) {
                write_champion(&mut w, Some(*c));
            }
            w.write_u32(l.starts_in_ms);
            w.finish()
        }
    }
}

pub fn decode_server(bytes: &[u8]) -> Result<(PacketHeader, ServerMessage), DecodeError> {
    let (mut r, header, kind) = open(bytes)?;
    let msg = match kind {
        0 => ServerMessage::Welcome {
            player: PlayerId(r.read_u8()?),
            unit: UnitId(r.read_u32()?),
            team: if r.read_bool()? { Team::Red } else { Team::Blue },
            map: MapId::from_u8(r.read_u8()?).ok_or(DecodeError::Invalid("map id"))?,
            champion: read_champion(&mut r)?.ok_or(DecodeError::Invalid("champion"))?,
            home: read_vec2(&mut r)?,
            rules: Rules {
                start_level: r.read(5)? as u8,
                start_gold: read_finite(&mut r)?,
                passive_gold: read_finite(&mut r)?,
                ranked: r.read_bool()?,
                augments: r.read_bool()?,
                hyper: r.read_bool()?,
            },
            tick: Tick(r.read_u32()?),
            tick_hz: r.read_u8()?,
            since_tick_us: r.read_u32()?,
            time_echo: read_echo(&mut r)?,
            token: r.read_u64()?,
            spectator: r.read_bool()?,
            mode: GameMode::from_u8(r.read(3)? as u8).ok_or(DecodeError::Invalid("game mode"))?,
        },
        1 => {
            let tick = Tick(r.read_u32()?);
            let since_tick_us = r.read_u32()?;
            let time_echo = if r.read_bool()? { Some(read_echo(&mut r)?) } else { None };
            let last_cmd_seq = r.read_u32()?;
            let n = r.read(4)? as usize;
            if n > MAX_REPORTS_PER_SNAPSHOT {
                return Err(DecodeError::Invalid("report count"));
            }
            let mut reports = Vec::with_capacity(n);
            for _ in 0..n {
                let seq = r.read_u32()?;
                let applied_tick = Tick(r.read_u32()?);
                let applied_sub = SubTick::new(r.read(6)? as u8);
                let lead_us = r.read_i16()? as i32 * 500;
                reports.push(CommandReport { seq, applied_tick, applied_sub, lead_us });
            }
            let own = if r.read_bool()? { Some((UnitId(r.read_u32()?), read_unit_state(&mut r)?)) } else { None };
            let baseline = if r.read_bool()? { Some(Tick(r.read_u32()?)) } else { None };
            let count = r.read_u16()? as usize;
            if count > MAX_UNITS_PER_SNAPSHOT {
                return Err(DecodeError::Invalid("unit count"));
            }
            let mut others = Vec::with_capacity(count);
            for _ in 0..count {
                others.push(read_update(&mut r)?);
            }
            let count = r.read_u16()? as usize;
            if count > MAX_UNITS_PER_SNAPSHOT {
                return Err(DecodeError::Invalid("removed count"));
            }
            let mut removed = Vec::with_capacity(count);
            for _ in 0..count {
                removed.push(UnitId(r.read_u32()?));
            }
            let n = r.read(6)? as usize;
            if n > MAX_EVENTS_PER_SNAPSHOT {
                return Err(DecodeError::Invalid("event count"));
            }
            let mut events = Vec::with_capacity(n);
            for _ in 0..n {
                let seq = r.read_u32()?;
                let kind = r.read(5)?;
                if kind != VOLLEY {
                    events.push((seq, read_event_of(kind, &mut r)?));
                    continue;
                }
                let first = read_missile(&mut r)?;
                events.push((seq, SimEvent::MissileSpawned(first)));
                for k in 1..=r.read(3)? as u32 {
                    let (dir, shot) = (read_vec2(&mut r)?, r.read(4)? as u8);
                    let m = Missile { id: first.id.wrapping_add(k), dir, shot, ..first };
                    events.push((seq.wrapping_add(k), SimEvent::MissileSpawned(m)));
                }
                if events.len() > MAX_EVENTS_PER_SNAPSHOT {
                    return Err(DecodeError::Invalid("event count"));
                }
            }
            let scoreboard = if r.read_bool()? { Some(Box::new(read_scoreboard(&mut r)?)) } else { None };
            ServerMessage::Snapshot(Box::new(Snapshot {
                tick,
                since_tick_us,
                time_echo,
                last_cmd_seq,
                reports,
                own,
                baseline,
                others,
                removed,
                events,
                scoreboard,
            }))
        }
        2 => ServerMessage::Reject {
            reason: match r.read_u8()? {
                0 => RejectReason::ProtocolMismatch,
                1 => RejectReason::ServerFull,
                _ => return Err(DecodeError::Invalid("reject reason")),
            },
        },
        3 => {
            let you = PlayerId(r.read_u8()?);
            let n = r.read(5)? as usize;
            let mut slots = Vec::with_capacity(n);
            for _ in 0..n {
                slots.push(LobbySlot {
                    player: PlayerId(r.read_u8()?),
                    team: if r.read_bool()? { Team::Red } else { Team::Blue },
                    champion: read_champion(&mut r)?.ok_or(DecodeError::Invalid("champion"))?,
                    rerolls: r.read(3)? as u8,
                    ready: r.read_bool()?,
                    bot: r.read_bool()?,
                });
            }
            let n = r.read(5)? as usize;
            let bench = (0..n)
                .map(|_| read_champion(&mut r)?.ok_or(DecodeError::Invalid("champion")))
                .collect::<Result<_, _>>()?;
            ServerMessage::Lobby(Box::new(LobbyState { you, slots, bench, starts_in_ms: r.read_u32()? }))
        }
        _ => unreachable!("two-bit message kind"),
    };
    Ok((header, msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hdr() -> PacketHeader {
        PacketHeader { seq: 7, ack: 3, ack_bits: 0b1011 }
    }

    #[test]
    fn input_round_trip() {
        let c =
            |seq: u32, kind| Command { player: PlayerId(0), seq, tick: Tick(1234 + seq), sub: SubTick::new(17), kind };
        let commands = vec![
            c(41, CommandKind::MoveTo(QPoint { x: 4000, y: 60000 })),
            c(42, CommandKind::Stop),
            c(43, CommandKind::Cast { slot: 5, target: QPoint { x: 7, y: 9 } }),
            c(44, CommandKind::Attack(UnitId(812))),
            c(45, CommandKind::AttackMove(QPoint { x: 1, y: 65535 })),
            c(46, CommandKind::LevelUp(3)),
            c(47, CommandKind::Buy(25)),
            c(48, CommandKind::Sell(5)),
            c(49, CommandKind::Undo),
            c(50, CommandKind::PickAugment(2)),
            c(51, CommandKind::RerollAugment(2)),
        ];
        // At most 8 commands per packet: two packets cover every kind.
        for commands in [commands[..8].to_vec(), commands[8..].to_vec()] {
            let msg = ClientMessage::Input { client_time_us: 0xABCD_1234, event_ack: 99, snapshot_ack: 1230, commands };
            let bytes = encode_client(&hdr(), &msg);
            assert_eq!(decode_client(&bytes).unwrap(), (hdr(), msg));
        }
        let hello = ClientMessage::Hello {
            protocol: crate::PROTOCOL_VERSION,
            client_time_us: 5,
            champion: Some(ChampionId::Vesper),
            resume: 0xdead_beef_0123_4567,
            spectate: true,
        };
        assert_eq!(decode_client(&encode_client(&hdr(), &hello)).unwrap().1, hello);
        for a in [LobbyAction::Reroll, LobbyAction::Take(ChampionId::Shade), LobbyAction::Ready(true)] {
            let msg = ClientMessage::Lobby(a);
            assert_eq!(decode_client(&encode_client(&hdr(), &msg)).unwrap().1, msg);
        }
    }

    #[test]
    fn lobby_round_trip() {
        let slot = |p: u8, c: ChampionId| LobbySlot {
            player: PlayerId(p),
            team: if p.is_multiple_of(2) { Team::Blue } else { Team::Red },
            champion: c,
            rerolls: p % 3,
            ready: p % 2 == 1,
            bot: p > 6,
        };
        let msg = ServerMessage::Lobby(Box::new(LobbyState {
            you: PlayerId(4),
            slots: ChampionId::ALL.iter().enumerate().map(|(i, c)| slot(i as u8 + 2, *c)).collect(),
            bench: vec![ChampionId::Lumen, ChampionId::Ember],
            starts_in_ms: 42_000,
        }));
        assert_eq!(decode_server(&encode_server(&hdr(), &msg)).unwrap().1, msg);
    }

    /// Facing travels in 10 bits: every direction comes back within half a step (0.18°).
    #[test]
    fn facing_round_trips_within_half_a_step() {
        for i in 0..3600 {
            let a = i as f32 / 3600.0 * std::f32::consts::TAU;
            let q = facing_to_wire(Vec2::new(a.cos(), a.sin()));
            assert!(q < 1024);
            let back = facing_from_wire(q);
            let err = (back - a).rem_euclid(std::f32::consts::TAU);
            let err = err.min(std::f32::consts::TAU - err);
            assert!(err.to_degrees() <= 360.0 / 1024.0 / 2.0 + 1e-3, "{a} -> {q} -> {back}");
        }
        assert_eq!(facing_to_wire(Vec2::new(1.0, 0.0)), 0);
        assert_eq!(facing_to_wire(Vec2::new(0.0, -1.0)), 768);
    }

    fn sample_state() -> UnitState {
        UnitState {
            pos: Vec2::new(1_234.567_9, 9_876.543),
            order: Order::Attack(UnitId(17)),
            move_speed: 325.0,
            detour: Some(Vec2::new(1_300.125, 9_800.5)),
            stuck: 2,
            path: {
                let mut p = Path::EMPTY;
                p.points[..3].copy_from_slice(&[Vec2::new(5.5, 6.25), Vec2::new(7.0, 8.0), Vec2::new(9.5, 1.0)]);
                p.len = 3;
                p.next = 1;
                p.complete = false;
                p
            },
            cast: Some(Cast {
                slot: 3,
                dir: Vec2::new(0.6, -0.8),
                point: Vec2::new(10.1, 20.2),
                fire_at: SimTime(123_457),
                seq: 41,
                mobile: true,
            }),
            attack: Some(AttackWindup { target: UnitId(17), fire_at: SimTime(123_460) }),
            attack_ready_at: SimTime(124_000),
            dash: Some(DashMove {
                dir: Vec2::new(0.0, 1.0),
                to: Vec2::new(5.0, 330.0),
                speed: 1000.0,
                end_at: SimTime(124_100),
                strike: Some((UnitId(23), 2)),
                recover: SimDuration(154),
            }),
            stunned_until: SimTime(99_999),
            rooted_until: SimTime(99_998),
            slow: 40,
            slowed_until: SimTime(99_997),
            cooldowns: [1, 2, 3, 4, 5, 600_000].map(SimTime),
            health: 412.333_3,
            shield: 77.7,
            shield_until: SimTime(130_000),
            respawn_at: Some(SimTime(140_000)),
            echo: Some(EchoCast {
                slot: 2,
                dir: Vec2::new(0.6, 0.8),
                point: Vec2::new(5.5, 6.25),
                at: SimTime(77_777),
                seq: 4242,
            }),
            progress: Progress {
                level: 17,
                xp: 1234,
                gold: 2875.25,
                ranks: [5, 3, 1, 2],
                points: 2,
                streak: -3,
                items: [19, 0, 7, 26, 0, 3],
                charges: [0, 0, 2, 0, 0, 5],
                lifeline_ready: SimTime(98_765),
                undo: [
                    Trade { items: [10, 0, 7, 26, 0, 3], charges: [0, 0, 0, 0, 0, 0], gold: -2750.0 },
                    Trade { items: [10, 0, 7, 26, 0, 27], charges: [0, 0, 0, 0, 0, 4], gold: 280.0 },
                    Trade { items: [0; INVENTORY], charges: [0; INVENTORY], gold: 0.0 },
                    Trade { items: [0; INVENTORY], charges: [0; INVENTORY], gold: 0.0 },
                ],
                undo_len: 2,
                augments: [17, 2, 0, 0],
                offer: [11, 13, 16],
                drafted: 3,
                rerolled: 0b101,
                golden: 2,
                augment_seed: 0xdead_beef,
                unstable_tiny: true,
                stacks: 37,
                takedowns: 5,
                hyper: true,
                kills: 12,
                deaths: 3,
                assists: 9,
                cs: 140,
            },
            spellblade_until: SimTime(88_888),
            facing: Vec2::new(0.6, -0.8),
            recovery: Some(Recovery { hard_until: SimTime(123_500), until: SimTime(123_800) }),
            buffered: Some(BufferedCast { slot: 4, target: QPoint { x: 1234, y: 5678 }, seq: 77 }),
            attacks: 201,
            potion_rate: 8.0,
            potion_until: SimTime(77_000),
        }
    }

    /// Every skillshot spec and CC kind of the six kits survives the wire exactly.
    #[test]
    fn every_kit_spec_and_cc_round_trips() {
        use mftr_sim::ability::Effect;
        let mut ccs = vec![Cc::None, Cc::Pull(150), Cc::Slow { pct: 100, duration: SimDuration::from_millis(1) }];
        let mut w = BitWriter::new();
        let mut specs = Vec::new();
        for c in ChampionId::ALL {
            for a in c.def().abilities {
                match a.effect {
                    Effect::Line(s) => {
                        write_line_spec(&mut w, &s);
                        specs.push(s);
                    }
                    Effect::Area(s) => ccs.push(s.cc),
                    Effect::Lunge(l) => ccs.push(l.cc),
                    _ => {}
                }
            }
        }
        for cc in &ccs {
            write_cc(&mut w, *cc);
        }
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        for s in &specs {
            assert_eq!(read_line_spec(&mut r).unwrap(), *s);
        }
        for cc in &ccs {
            assert_eq!(read_cc(&mut r).unwrap(), *cc);
        }
        assert!(specs.len() >= 7 && ccs.len() >= 10);
    }

    #[test]
    fn snapshot_round_trip_is_lossless_for_own_state_and_events() {
        let own_state = sample_state();
        let spec = match mftr_sim::champion::EMBER.abilities[3].effect {
            mftr_sim::ability::Effect::Line(s) => s,
            _ => unreachable!(),
        };
        let missile = Missile {
            id: 9,
            owner: UnitId(3),
            team: Team::Red,
            origin: Vec2::new(10.5, 20.25),
            dir: Vec2::new(0.0, 1.0),
            spec,
            spawn_at: SimTime(5_000),
            cast_seq: 41,
            power: 140.0,
            shot: 2,
            slot: 5,
        };
        let area = Area {
            id: 11,
            owner: UnitId(3),
            team: Team::Blue,
            center: Vec2::new(1.5, 2.5),
            radius: 160.0,
            spawn_at: SimTime(6_000),
            detonate_at: SimTime(7_632),
            kind: DamageKind::Magic,
            power: 146.0,
            cast_seq: 42,
            cc: Cc::Knockup(SimDuration::from_millis(1000)),
            shot: 4,
            slot: 1,
        };
        let bolt = Bolt {
            id: 12,
            owner: UnitId(4),
            team: Team::Red,
            target: UnitId(3),
            origin: Vec2::new(3.0, 4.0),
            pos: Vec2::new(3.0, 4.0),
            speed: 2200.0,
            launched_at: SimTime(6_100),
            power: 66.0,
            kind: DamageKind::True,
        };
        let remote = |id, kind, team, champion| RemoteUnit {
            id: UnitId(id),
            kind,
            team,
            pos: QPoint { x: 10, y: 20 },
            target: Some(QPoint { x: 50, y: 60 }),
            speed: 325,
            collision_radius: 35,
            gameplay_radius: 220,
            protected: true,
            champion,
            minion: (kind == UnitKind::Minion).then_some(MinionKind::Super),
            augments: if champion.is_some() { [24, 52, 0, 0] } else { [0; 4] },
            health: 512,
            max_health: 620,
            shield: 150,
            level: 12,
            casting: true,
            attacking: false,
            stunned: true,
            rooted: false,
            dashing: true,
            slowed: true,
            recovering: true,
            attack_variant: 3,
            facing: 1001,
        };
        let events = vec![
            SimEvent::CastStarted {
                unit: UnitId(3),
                slot: 1,
                at: SimTime(4_520),
                dir: Vec2::new(0.0, 1.0),
                point: Vec2::new(8.0, 9.0),
                fire_at: SimTime(5_000),
                seq: 41,
            },
            SimEvent::MissileSpawned(missile),
            SimEvent::MissileHit { id: 9, target: UnitId(4), at: SimTime(5_500) },
            SimEvent::MissileExpired { id: 10, at: SimTime(6_000) },
            SimEvent::AreaSpawned(area),
            SimEvent::AreaDetonated { id: 11, at: SimTime(7_632) },
            SimEvent::AttackLaunched(bolt),
            SimEvent::AttackLanded { id: 12, target: UnitId(3), at: SimTime(6_300), hit: true },
            SimEvent::Damage {
                source: UnitId(4),
                target: UnitId(3),
                kind: DamageKind::Physical,
                amount: 40.25,
                absorbed: 11.0,
                origin: mftr_sim::world::DamageOrigin::Ability(2),
                at: SimTime(6_300),
            },
            SimEvent::Died { unit: UnitId(3), killer: UnitId(4), at: SimTime(6_300), respawn_at: SimTime(17_820) },
            SimEvent::Respawned { unit: UnitId(3), pos: Vec2::new(600.0, 2000.0), at: SimTime(17_856) },
            SimEvent::Blinked { unit: UnitId(4), from: Vec2::new(1.0, 2.0), to: Vec2::new(3.0, 4.0), at: SimTime(1) },
            SimEvent::Dashed {
                unit: UnitId(4),
                from: Vec2::new(1.0, 2.0),
                to: Vec2::new(3.0, 4.0),
                at: SimTime(1),
                end_at: SimTime(625),
            },
            SimEvent::Shielded { unit: UnitId(4), amount: 150.0, at: SimTime(2), until: SimTime(4_802) },
            SimEvent::Healed { unit: UnitId(4), amount: 150.0, at: SimTime(3) },
            SimEvent::MatchEnded { winner: Team::Red, at: SimTime(99_000) },
            SimEvent::Reward { unit: UnitId(3), gold: 21.5, xp: 60, at: SimTime(99_001) },
        ];
        let snap = Snapshot {
            tick: Tick(99),
            since_tick_us: 1500,
            time_echo: Some(TimeEcho { client_time_us: 5, hold_us: 1200 }),
            last_cmd_seq: 77,
            reports: vec![CommandReport {
                seq: 77,
                applied_tick: Tick(98),
                applied_sub: SubTick::new(3),
                lead_us: -2500,
            }],
            own: Some((UnitId(3), own_state)),
            baseline: Some(Tick(96)),
            others: vec![
                UnitUpdate {
                    mask: delta::ALL,
                    unit: remote(4, UnitKind::Champion, Team::Blue, Some(ChampionId::Vesper)),
                },
                UnitUpdate { mask: delta::ALL, unit: remote(5, UnitKind::Minion, Team::Red, None) },
                UnitUpdate { mask: delta::ALL, unit: remote(6, UnitKind::Relic, Team::Blue, None) },
            ],
            removed: vec![UnitId(11), UnitId(12)],
            events: events.into_iter().enumerate().map(|(i, e)| (i as u32 + 1, e)).collect(),
            scoreboard: Some(Box::new(Scoreboard {
                started_at: SimTime(48_000),
                rows: vec![ScoreRow {
                    unit: UnitId(4),
                    champion: ChampionId::Vesper,
                    team: Team::Red,
                    bot: true,
                    level: 14,
                    kills: 9,
                    deaths: 2,
                    assists: 11,
                    cs: 87,
                    items: [18, 0, 7, 0, 0, 3],
                    augments: [46, 0, 12, 0],
                    respawn_ds: 215,
                }],
            })),
        };
        let msg = ServerMessage::Snapshot(Box::new(snap));
        let bytes = encode_server(&hdr(), &msg);
        assert!(bytes.len() <= crate::MAX_PAYLOAD_BYTES, "{} bytes", bytes.len());
        let (_, back) = decode_server(&bytes).unwrap();
        assert_eq!(back, msg);
        if let ServerMessage::Snapshot(s) = back {
            assert!(s.own.unwrap().1.bits_eq(&own_state));
        }
    }

    /// A Multishot volley (consecutive spawns from one cast) travels as one record and decodes
    /// back to the same events; anything that differs breaks the volley.
    #[test]
    fn volleys_are_batched_losslessly() {
        let spec = match mftr_sim::champion::EMBER.abilities[0].effect {
            mftr_sim::ability::Effect::Line(s) => s,
            _ => unreachable!(),
        };
        let m = |id: u32, dir: Vec2, shot: u8| Missile {
            id,
            owner: UnitId(3),
            team: Team::Red,
            origin: Vec2::new(10.5, 20.25),
            dir,
            spec,
            spawn_at: SimTime(5_000),
            cast_seq: 41,
            power: 140.0,
            shot,
            slot: 0,
        };
        let volley = [
            m(20, Vec2::new(1.0, 0.0), 0),
            m(21, Vec2::new(0.965_925_8, 0.258_819_04), 1),
            m(22, Vec2::new(0.965_925_8, -0.258_819_04), 2),
        ];
        let other = Missile { power: 99.0, ..m(23, Vec2::new(0.0, 1.0), 0) };
        let events: Vec<(u32, SimEvent)> = volley
            .iter()
            .chain([other].iter())
            .enumerate()
            .map(|(i, m)| (100 + i as u32, SimEvent::MissileSpawned(*m)))
            .chain([(104, SimEvent::MissileExpired { id: 20, at: SimTime(9) })])
            .collect();
        let snap = |events: Vec<(u32, SimEvent)>| Snapshot {
            tick: Tick(7),
            since_tick_us: 0,
            time_echo: None,
            last_cmd_seq: 0,
            reports: Vec::new(),
            own: None,
            baseline: None,
            others: Vec::new(),
            removed: Vec::new(),
            events,
            scoreboard: None,
        };
        let bytes = encode_server(&hdr(), &ServerMessage::Snapshot(Box::new(snap(events.clone()))));
        let ServerMessage::Snapshot(back) = decode_server(&bytes).unwrap().1 else { panic!() };
        assert_eq!(back.events, events);
        // Batched, the two extra missiles cost a direction each instead of a whole event.
        let single = encode_server(&hdr(), &ServerMessage::Snapshot(Box::new(snap(events[..1].to_vec())))).len();
        let three = encode_server(&hdr(), &ServerMessage::Snapshot(Box::new(snap(events[..3].to_vec())))).len();
        assert!(three - single <= 20, "{single} bytes for one missile, {three} for the volley");
    }

    #[test]
    fn welcome_round_trip() {
        let msg = ServerMessage::Welcome {
            player: PlayerId(3),
            unit: UnitId(40),
            team: Team::Red,
            map: MapId::Arena,
            champion: ChampionId::Wren,
            home: Vec2::new(3400.0, 2000.0),
            rules: Rules::HYPER,
            tick: Tick(77),
            tick_hz: 30,
            since_tick_us: 12,
            time_echo: TimeEcho { client_time_us: 1, hold_us: 2 },
            token: u64::MAX - 5,
            spectator: false,
            mode: GameMode::Aram,
        };
        assert_eq!(decode_server(&encode_server(&hdr(), &msg)).unwrap().1, msg);
    }

    #[test]
    fn garbage_is_rejected_not_panicking() {
        assert!(decode_client(&[]).is_err());
        assert!(decode_client(&[0x00, 1, 2, 3]).is_err());
        assert!(decode_server(&[MAGIC, 0, 0, 0, 0, 0, 0, 0, 0, 0xFF]).is_err());
        // A truncated snapshot.
        let bytes = encode_server(&hdr(), &ServerMessage::Reject { reason: RejectReason::ServerFull });
        assert!(decode_server(&bytes[..bytes.len() - 1]).is_err());
    }
}
