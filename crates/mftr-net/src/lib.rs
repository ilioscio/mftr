//! `mftr-net`: the MFTR wire protocol (`docs/design/03b-netcode-wire-protocol.md`).
//!
//! M0 prototype: plain UDP payloads with no encryption yet. The netcode.io-style secure
//! transport lands with the transport decision (DECISIONS open question 3). Everything here is
//! transport-agnostic: it turns messages into bytes and back.

pub mod bits;
pub mod clock;
pub mod conditioner;
pub mod delta;
pub mod msg;
pub mod packet;

/// Bumped on any wire-format change. Client and server must match exactly.
pub const PROTOCOL_VERSION: u16 = 10;

/// Packets above this size are a bug (IPv6-safe, see 03b §1).
pub const MAX_PACKET_BYTES: usize = 1200;
