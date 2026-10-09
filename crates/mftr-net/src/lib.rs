//! `mftr-net`: the MFTR wire protocol (`docs/design/03b-netcode-wire-protocol.md`).
//!
//! Everything here is sans-IO: it turns messages into bytes and back. [`secure`] wraps the game
//! packets for the wire (D40: a Noise handshake, then ChaCha20-Poly1305 on every packet).

pub mod bits;
pub mod clock;
pub mod conditioner;
pub mod delta;
pub mod msg;
pub mod packet;
pub mod secure;

/// Bumped on any wire-format change. Client and server must match exactly.
pub const PROTOCOL_VERSION: u16 = 18;

/// Packets above this size are a bug (IPv6-safe, see 03b §1).
pub const MAX_PACKET_BYTES: usize = 1200;

/// The largest game packet: what's left of a datagram after the secure transport's bytes.
pub const MAX_PAYLOAD_BYTES: usize = MAX_PACKET_BYTES - secure::OVERHEAD;
