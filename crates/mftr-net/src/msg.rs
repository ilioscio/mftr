//! Messages and their encoding (M0 subset of 03b §5–§8).
//!
//! Every packet: magic byte, packet header, 2-bit message kind, payload. M0 uses absolute
//! 32-bit ticks and sequence numbers for simplicity. Delta-coding (03b) comes once the
//! formats settle.

use crate::bits::{BitReader, BitWriter, DecodeError};
use crate::packet::PacketHeader;
use mftr_sim::ability::LineSkillshot;
use mftr_sim::{
    Cast, Command, CommandKind, Missile, Order, PlayerId, QPoint, SimDuration, SimEvent, SimTime, SubTick, Team, Tick,
    UnitId, UnitKind, UnitState, Vec2,
};

const MAGIC: u8 = 0x4D; // 'M'

/// Max commands carried per input packet (redundancy window, 03b §4).
pub const MAX_COMMANDS_PER_PACKET: usize = 8;
/// Max command reports repeated per snapshot.
pub const MAX_REPORTS_PER_SNAPSHOT: usize = 8;
/// Max reliable events carried per snapshot; the rest wait for the next one (03b §7).
pub const MAX_EVENTS_PER_SNAPSHOT: usize = 32;

#[derive(Clone, Debug, PartialEq)]
pub enum ClientMessage {
    Hello {
        protocol: u16,
        client_time_us: u32,
    },
    /// The `player` field of each command is ignored by the server (taken from the connection).
    Input {
        client_time_us: u32,
        /// Highest event sequence received in order (cumulative ack for the events channel).
        event_ack: u32,
        commands: Vec<Command>,
    },
    Bye,
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
    /// Status flags for display (windup animation, stun indicator).
    pub casting: bool,
    pub stunned: bool,
}

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
    pub others: Vec<RemoteUnit>,
    /// Reliable ordered events `(seq, event)`, repeated until acknowledged (03b §7).
    pub events: Vec<(u32, SimEvent)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ServerMessage {
    Welcome {
        player: PlayerId,
        unit: UnitId,
        /// Own team: missile sides (own/ally/enemy) and ally/enemy display.
        team: Team,
        tick: Tick,
        tick_hz: u8,
        since_tick_us: u32,
        time_echo: TimeEcho,
    },
    Snapshot(Snapshot),
    Reject {
        reason: RejectReason,
    },
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
            w.write(0, 2);
            write_qpoint(w, q);
        }
        CommandKind::Stop => w.write(1, 2),
        CommandKind::CastQ(q) => {
            w.write(2, 2);
            write_qpoint(w, q);
        }
    }
}

fn read_command(r: &mut BitReader) -> Result<Command, DecodeError> {
    let seq = r.read_u32()?;
    let tick = Tick(r.read_u32()?);
    let sub = SubTick::new(r.read(6)? as u8);
    let kind = match r.read(2)? {
        0 => CommandKind::MoveTo(read_qpoint(r)?),
        1 => CommandKind::Stop,
        2 => CommandKind::CastQ(read_qpoint(r)?),
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

fn write_vec2(w: &mut BitWriter, v: Vec2) {
    w.write_f32(v.x);
    w.write_f32(v.y);
}

fn read_vec2(r: &mut BitReader) -> Result<Vec2, DecodeError> {
    let v = Vec2::new(r.read_f32()?, r.read_f32()?);
    if v.x.is_finite() && v.y.is_finite() { Ok(v) } else { Err(DecodeError::Invalid("vector")) }
}

fn write_event(w: &mut BitWriter, e: &SimEvent) {
    match e {
        SimEvent::CastStarted { unit, at, dir, fire_at, seq } => {
            w.write(0, 2);
            w.write_u32(unit.0);
            write_time(w, *at);
            write_vec2(w, *dir);
            write_time(w, *fire_at);
            w.write_u32(*seq);
        }
        SimEvent::MissileSpawned(m) => {
            w.write(1, 2);
            w.write_u32(m.id);
            w.write_u32(m.owner.0);
            w.write_bool(m.team == Team::Red);
            write_vec2(w, m.origin);
            write_vec2(w, m.dir);
            w.write_u32(m.spec.windup.0 as u32);
            w.write_u32(m.spec.cooldown.0 as u32);
            w.write_f32(m.spec.speed);
            w.write_f32(m.spec.radius);
            w.write_f32(m.spec.range);
            w.write_u32(m.spec.stun.0 as u32);
            write_time(w, m.spawn_at);
            w.write_u32(m.cast_seq);
        }
        SimEvent::MissileHit { id, target, at } => {
            w.write(2, 2);
            w.write_u32(*id);
            w.write_u32(target.0);
            write_time(w, *at);
        }
        SimEvent::MissileExpired { id, at } => {
            w.write(3, 2);
            w.write_u32(*id);
            write_time(w, *at);
        }
    }
}

fn read_event(r: &mut BitReader) -> Result<SimEvent, DecodeError> {
    Ok(match r.read(2)? {
        0 => SimEvent::CastStarted {
            unit: UnitId(r.read_u32()?),
            at: read_time(r)?,
            dir: read_vec2(r)?,
            fire_at: read_time(r)?,
            seq: r.read_u32()?,
        },
        1 => {
            let id = r.read_u32()?;
            let owner = UnitId(r.read_u32()?);
            let team = if r.read_bool()? { Team::Red } else { Team::Blue };
            let origin = read_vec2(r)?;
            let dir = read_vec2(r)?;
            let windup = SimDuration(r.read_u32()? as u64);
            let cooldown = SimDuration(r.read_u32()? as u64);
            let (speed, radius, range) = (r.read_f32()?, r.read_f32()?, r.read_f32()?);
            let stun = SimDuration(r.read_u32()? as u64);
            if !(speed > 0.0 && speed.is_finite() && radius.is_finite() && range.is_finite()) {
                return Err(DecodeError::Invalid("missile spec"));
            }
            let spec = LineSkillshot { windup, cooldown, speed, radius, range, stun };
            let spawn_at = read_time(r)?;
            let cast_seq = r.read_u32()?;
            SimEvent::MissileSpawned(Missile { id, owner, team, origin, dir, spec, spawn_at, cast_seq })
        }
        2 => SimEvent::MissileHit { id: r.read_u32()?, target: UnitId(r.read_u32()?), at: read_time(r)? },
        _ => SimEvent::MissileExpired { id: r.read_u32()?, at: read_time(r)? },
    })
}

fn write_unit_state(w: &mut BitWriter, s: &UnitState) {
    w.write_f32(s.pos.x);
    w.write_f32(s.pos.y);
    w.write_f32(s.move_speed);
    match s.order {
        Order::Idle => w.write_bool(false),
        Order::MoveTo(q) => {
            w.write_bool(true);
            write_qpoint(w, q);
        }
    }
    w.write_bool(s.detour.is_some());
    if let Some(d) = s.detour {
        w.write_f32(d.x);
        w.write_f32(d.y);
    }
    w.write_u8(s.stuck);
    w.write_bool(s.cast.is_some());
    if let Some(c) = s.cast {
        write_vec2(w, c.dir);
        write_time(w, c.fire_at);
        w.write_u32(c.seq);
    }
    write_time(w, s.stunned_until);
    write_time(w, s.q_ready_at);
}

fn read_unit_state(r: &mut BitReader) -> Result<UnitState, DecodeError> {
    let pos = read_vec2(r)?;
    let move_speed = r.read_f32()?;
    let order = if r.read_bool()? { Order::MoveTo(read_qpoint(r)?) } else { Order::Idle };
    let detour = if r.read_bool()? { Some(read_vec2(r)?) } else { None };
    let stuck = r.read_u8()?;
    let cast = if r.read_bool()? {
        Some(Cast { dir: read_vec2(r)?, fire_at: read_time(r)?, seq: r.read_u32()? })
    } else {
        None
    };
    let stunned_until = read_time(r)?;
    let q_ready_at = read_time(r)?;
    if !move_speed.is_finite() {
        return Err(DecodeError::Invalid("unit state"));
    }
    Ok(UnitState { pos, order, move_speed, detour, stuck, cast, stunned_until, q_ready_at })
}

fn write_remote(w: &mut BitWriter, o: &RemoteUnit) {
    w.write_u32(o.id.0);
    w.write(
        match o.kind {
            UnitKind::Champion => 0,
            UnitKind::Minion => 1,
            UnitKind::Turret => 2,
        },
        2,
    );
    w.write_bool(o.team == Team::Red);
    write_qpoint(w, o.pos);
    w.write_bool(o.target.is_some());
    if let Some(t) = o.target {
        write_qpoint(w, t);
    }
    w.write(o.speed.min(1023) as u64, 10);
    w.write_u8(o.collision_radius);
    w.write_bool(o.casting);
    w.write_bool(o.stunned);
}

fn read_remote(r: &mut BitReader) -> Result<RemoteUnit, DecodeError> {
    let id = UnitId(r.read_u32()?);
    let kind = match r.read(2)? {
        0 => UnitKind::Champion,
        1 => UnitKind::Minion,
        2 => UnitKind::Turret,
        _ => return Err(DecodeError::Invalid("unit kind")),
    };
    let team = if r.read_bool()? { Team::Red } else { Team::Blue };
    let pos = read_qpoint(r)?;
    let target = if r.read_bool()? { Some(read_qpoint(r)?) } else { None };
    let speed = r.read(10)? as u16;
    let collision_radius = r.read_u8()?;
    let casting = r.read_bool()?;
    let stunned = r.read_bool()?;
    Ok(RemoteUnit { id, kind, team, pos, target, speed, collision_radius, casting, stunned })
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
        ClientMessage::Hello { protocol, client_time_us } => {
            let mut w = begin(header, 0);
            w.write_u16(*protocol);
            w.write_u32(*client_time_us);
            w.finish()
        }
        ClientMessage::Input { client_time_us, event_ack, commands } => {
            let mut w = begin(header, 1);
            w.write_u32(*client_time_us);
            w.write_u32(*event_ack);
            let n = commands.len().min(MAX_COMMANDS_PER_PACKET);
            w.write(n as u64, 4);
            for c in &commands[commands.len() - n..] {
                write_command(&mut w, c);
            }
            w.finish()
        }
        ClientMessage::Bye => begin(header, 2).finish(),
    }
}

pub fn decode_client(bytes: &[u8]) -> Result<(PacketHeader, ClientMessage), DecodeError> {
    let (mut r, header, kind) = open(bytes)?;
    let msg = match kind {
        0 => ClientMessage::Hello { protocol: r.read_u16()?, client_time_us: r.read_u32()? },
        1 => {
            let client_time_us = r.read_u32()?;
            let event_ack = r.read_u32()?;
            let n = r.read(4)? as usize;
            if n > MAX_COMMANDS_PER_PACKET {
                return Err(DecodeError::Invalid("command count"));
            }
            let commands = (0..n).map(|_| read_command(&mut r)).collect::<Result<_, _>>()?;
            ClientMessage::Input { client_time_us, event_ack, commands }
        }
        2 => ClientMessage::Bye,
        _ => return Err(DecodeError::Invalid("client message kind")),
    };
    Ok((header, msg))
}

// ---- server → client --------------------------------------------------------------------

pub fn encode_server(header: &PacketHeader, msg: &ServerMessage) -> Vec<u8> {
    match msg {
        ServerMessage::Welcome { player, unit, team, tick, tick_hz, since_tick_us, time_echo } => {
            let mut w = begin(header, 0);
            w.write_u8(player.0);
            w.write_u32(unit.0);
            w.write_bool(*team == Team::Red);
            w.write_u32(tick.0);
            w.write_u8(*tick_hz);
            w.write_u32(*since_tick_us);
            write_echo(&mut w, time_echo);
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
            w.write_u8(s.others.len().min(255) as u8);
            for o in s.others.iter().take(255) {
                write_remote(&mut w, o);
            }
            let n = s.events.len().min(MAX_EVENTS_PER_SNAPSHOT);
            w.write(n as u64, 6);
            for (seq, e) in &s.events[..n] {
                w.write_u32(*seq);
                write_event(&mut w, e);
            }
            w.finish()
        }
        ServerMessage::Reject { reason } => {
            let mut w = begin(header, 2);
            w.write_u8(*reason as u8);
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
            tick: Tick(r.read_u32()?),
            tick_hz: r.read_u8()?,
            since_tick_us: r.read_u32()?,
            time_echo: read_echo(&mut r)?,
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
            let count = r.read_u8()? as usize;
            let mut others = Vec::with_capacity(count);
            for _ in 0..count {
                others.push(read_remote(&mut r)?);
            }
            let n = r.read(6)? as usize;
            if n > MAX_EVENTS_PER_SNAPSHOT {
                return Err(DecodeError::Invalid("event count"));
            }
            let mut events = Vec::with_capacity(n);
            for _ in 0..n {
                let seq = r.read_u32()?;
                events.push((seq, read_event(&mut r)?));
            }
            ServerMessage::Snapshot(Snapshot {
                tick,
                since_tick_us,
                time_echo,
                last_cmd_seq,
                reports,
                own,
                others,
                events,
            })
        }
        2 => ServerMessage::Reject {
            reason: match r.read_u8()? {
                0 => RejectReason::ProtocolMismatch,
                1 => RejectReason::ServerFull,
                _ => return Err(DecodeError::Invalid("reject reason")),
            },
        },
        _ => return Err(DecodeError::Invalid("server message kind")),
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
        let commands = vec![
            Command {
                player: PlayerId(0),
                seq: 41,
                tick: Tick(1234),
                sub: SubTick::new(17),
                kind: CommandKind::MoveTo(QPoint { x: 4000, y: 60000 }),
            },
            Command { player: PlayerId(0), seq: 42, tick: Tick(1235), sub: SubTick::new(63), kind: CommandKind::Stop },
            Command {
                player: PlayerId(0),
                seq: 43,
                tick: Tick(1236),
                sub: SubTick::new(5),
                kind: CommandKind::CastQ(QPoint { x: 7, y: 9 }),
            },
        ];
        let msg = ClientMessage::Input { client_time_us: 0xABCD_1234, event_ack: 99, commands };
        let bytes = encode_client(&hdr(), &msg);
        assert_eq!(decode_client(&bytes).unwrap(), (hdr(), msg));
    }

    #[test]
    fn snapshot_round_trip_is_lossless_for_own_state() {
        let own_state = UnitState {
            pos: Vec2::new(1_234.567_9, 9_876.543),
            order: Order::MoveTo(QPoint { x: 1, y: 2 }),
            move_speed: 325.0,
            detour: Some(Vec2::new(1_300.125, 9_800.5)),
            stuck: 2,
            cast: Some(Cast { dir: Vec2::new(0.6, -0.8), fire_at: SimTime(123_457), seq: 41 }),
            stunned_until: SimTime(99_999),
            q_ready_at: SimTime(124_000),
        };
        let missile = Missile {
            id: 9,
            owner: UnitId(3),
            team: Team::Red,
            origin: Vec2::new(10.5, 20.25),
            dir: Vec2::new(0.0, 1.0),
            spec: mftr_sim::ability::SANDBOX_LANCE,
            spawn_at: SimTime(5_000),
            cast_seq: 41,
        };
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
            others: vec![
                RemoteUnit {
                    id: UnitId(4),
                    kind: UnitKind::Champion,
                    team: Team::Blue,
                    pos: QPoint { x: 10, y: 20 },
                    target: None,
                    speed: 325,
                    collision_radius: 35,
                    casting: true,
                    stunned: false,
                },
                RemoteUnit {
                    id: UnitId(5),
                    kind: UnitKind::Turret,
                    team: Team::Red,
                    pos: QPoint { x: 30, y: 40 },
                    target: Some(QPoint { x: 50, y: 60 }),
                    speed: 1023,
                    collision_radius: 25,
                    casting: false,
                    stunned: true,
                },
            ],
            events: vec![
                (
                    1,
                    SimEvent::CastStarted {
                        unit: UnitId(3),
                        at: SimTime(4_520),
                        dir: Vec2::new(0.0, 1.0),
                        fire_at: SimTime(5_000),
                        seq: 41,
                    },
                ),
                (2, SimEvent::MissileSpawned(missile)),
                (3, SimEvent::MissileHit { id: 9, target: UnitId(4), at: SimTime(5_500) }),
                (4, SimEvent::MissileExpired { id: 10, at: SimTime(6_000) }),
            ],
        };
        let msg = ServerMessage::Snapshot(snap);
        let bytes = encode_server(&hdr(), &msg);
        let (_, back) = decode_server(&bytes).unwrap();
        assert_eq!(back, msg);
        if let ServerMessage::Snapshot(s) = back {
            assert!(s.own.unwrap().1.bits_eq(&own_state));
        }
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
