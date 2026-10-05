//! Packet header with sequence number and ack bitfield (03b §3).

use crate::bits::{BitReader, BitWriter, DecodeError};
use std::collections::VecDeque;

/// Wrap-around aware "a is newer than b" for 16-bit sequence numbers.
pub fn seq_greater(a: u16, b: u16) -> bool {
    (a > b && a - b <= 32768) || (a < b && b - a > 32768)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PacketHeader {
    pub seq: u16,
    /// Latest remote sequence received.
    pub ack: u16,
    /// Bit `i` set = remote sequence `ack - 1 - i` was received.
    pub ack_bits: u32,
}

impl PacketHeader {
    pub fn write(&self, w: &mut BitWriter) {
        w.write_u16(self.seq);
        w.write_u16(self.ack);
        w.write_u32(self.ack_bits);
    }

    pub fn read(r: &mut BitReader) -> Result<Self, DecodeError> {
        Ok(Self { seq: r.read_u16()?, ack: r.read_u16()?, ack_bits: r.read_u32()? })
    }
}

/// Remembers which remote packets arrived, to fill our outgoing `ack` / `ack_bits`.
#[derive(Clone, Debug, Default)]
pub struct ReceiveTracker {
    latest: Option<u16>,
    bits: u32,
}

impl ReceiveTracker {
    /// Record an incoming sequence. Returns `false` for duplicates and packets too old to track.
    pub fn record(&mut self, seq: u16) -> bool {
        let Some(latest) = self.latest else {
            self.latest = Some(seq);
            return true;
        };
        if seq == latest {
            return false;
        }
        if seq_greater(seq, latest) {
            let shift = seq.wrapping_sub(latest) as u32;
            self.bits = match shift {
                s if s > 32 => 0,
                32 => 1 << 31,
                s => (self.bits << s) | (1 << (s - 1)),
            };
            self.latest = Some(seq);
            true
        } else {
            let back = latest.wrapping_sub(seq) as u32;
            if back > 32 {
                return false;
            }
            let mask = 1u32 << (back - 1);
            let fresh = self.bits & mask == 0;
            self.bits |= mask;
            fresh
        }
    }

    pub fn ack_fields(&self) -> (u16, u32) {
        (self.latest.unwrap_or(0), self.bits)
    }
}

/// Assigns our outgoing sequence numbers and counts which ones the remote acked.
#[derive(Clone, Debug, Default)]
pub struct SendTracker {
    next: u16,
    in_flight: VecDeque<u16>,
    pub acked: u64,
    pub sent: u64,
}

impl SendTracker {
    pub fn next_seq(&mut self) -> u16 {
        let s = self.next;
        self.next = self.next.wrapping_add(1);
        self.in_flight.push_back(s);
        if self.in_flight.len() > 256 {
            self.in_flight.pop_front();
        }
        self.sent += 1;
        s
    }

    pub fn on_ack(&mut self, ack: u16, ack_bits: u32) {
        let before = self.in_flight.len();
        self.in_flight.retain(|&s| !is_acked(s, ack, ack_bits));
        self.acked += (before - self.in_flight.len()) as u64;
    }
}

fn is_acked(seq: u16, ack: u16, ack_bits: u32) -> bool {
    if seq == ack {
        return true;
    }
    if !seq_greater(ack, seq) {
        return false;
    }
    let back = ack.wrapping_sub(seq) as u32;
    back <= 32 && ack_bits & (1 << (back - 1)) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seq_wraps() {
        assert!(seq_greater(1, 0));
        assert!(seq_greater(0, 65535));
        assert!(!seq_greater(65535, 0));
    }

    #[test]
    fn receive_tracker_bits() {
        let mut t = ReceiveTracker::default();
        assert!(t.record(10));
        assert!(t.record(12)); // 11 missing
        assert!(t.record(13));
        assert_eq!(t.ack_fields(), (13, 0b101)); // 12 and 10 received, 11 not
        assert!(t.record(11)); // late arrival fills the hole
        assert_eq!(t.ack_fields(), (13, 0b111));
        assert!(!t.record(11)); // duplicate
    }

    #[test]
    fn receive_tracker_wraps_around() {
        let mut t = ReceiveTracker::default();
        assert!(t.record(65534));
        assert!(t.record(1)); // 65535 and 0 missing
        assert_eq!(t.ack_fields(), (1, 0b100));
    }

    #[test]
    fn send_tracker_counts_acks() {
        let mut s = SendTracker::default();
        let seqs: Vec<u16> = (0..5).map(|_| s.next_seq()).collect();
        assert_eq!(seqs, vec![0, 1, 2, 3, 4]);
        s.on_ack(4, 0b0101); // 4, 3, 1 acked
        assert_eq!(s.acked, 3);
        s.on_ack(4, 0b1111); // 2, 0 newly acked
        assert_eq!(s.acked, 5);
    }
}
