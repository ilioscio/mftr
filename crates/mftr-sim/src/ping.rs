//! Pings (01 §14, 17 §8): a player's (or a bot's) call to its team. Not simulated: the server
//! relays them to the pinger's team, and bots send and obey them. They live here so every
//! crate that talks about them (the server, the wire, bots, the client) shares one type.

use crate::math::Vec2;
use crate::world::UnitId;

/// What a ping says.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PingKind {
    /// Look here.
    Alert = 0,
    /// Back off: it's dangerous here.
    Danger = 1,
    /// Coming here.
    OnMyWay = 2,
    /// Help me here.
    Assist = 3,
    /// An enemy went missing from here.
    Missing = 4,
    /// Attack this (a unit, a structure, or here).
    Attack = 5,
    /// Defend this.
    Defend = 6,
    /// Gather here.
    Group = 7,
}

impl PingKind {
    pub const ALL: [PingKind; 8] = [
        PingKind::Alert,
        PingKind::Danger,
        PingKind::OnMyWay,
        PingKind::Assist,
        PingKind::Missing,
        PingKind::Attack,
        PingKind::Defend,
        PingKind::Group,
    ];

    pub fn from_wire(v: u8) -> Option<PingKind> {
        PingKind::ALL.get(v as usize).copied()
    }

    pub fn wire(self) -> u8 {
        self as u8
    }

    /// How the feed words it.
    pub fn text(self) -> &'static str {
        match self {
            PingKind::Alert => "Look here",
            PingKind::Danger => "Danger! Back off",
            PingKind::OnMyWay => "On my way",
            PingKind::Assist => "Assist me",
            PingKind::Missing => "Enemy missing",
            PingKind::Attack => "Attack",
            PingKind::Defend => "Defend",
            PingKind::Group => "Group up",
        }
    }
}

/// A ping: what, where, optionally on which unit, and from whose champion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ping {
    pub kind: PingKind,
    pub at: Vec2,
    pub target: Option<UnitId>,
    /// The pinger's champion (`None` for a spectator or before it spawned).
    pub from: Option<UnitId>,
}

/// A player may ping once every this many seconds, and at most `PING_BURST` times in
/// `PING_WINDOW_S` (01 §14: pings are rate-limited).
pub const PING_GAP_S: f64 = 0.5;
pub const PING_BURST: usize = 5;
pub const PING_WINDOW_S: f64 = 5.0;
