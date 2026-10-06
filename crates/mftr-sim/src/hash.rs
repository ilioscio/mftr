//! FNV-1a 64-bit state hashing. Used for per-tick state hashes (desync detection, replays,
//! the cross-platform determinism check), never for anything security-relevant.

#[derive(Clone, Debug)]
pub struct StateHasher(u64);

impl Default for StateHasher {
    fn default() -> Self {
        Self::new()
    }
}

/// A byte sink for state serialization. The same `hash_into` code feeds the state hash and
/// the bit-exact comparison prediction reconciliation uses (a `Vec<u8>`).
pub trait StateSink {
    fn write_bytes(&mut self, bytes: &[u8]);

    fn write_u8(&mut self, v: u8) {
        self.write_bytes(&[v]);
    }

    fn write_u16(&mut self, v: u16) {
        self.write_bytes(&v.to_le_bytes());
    }

    fn write_u32(&mut self, v: u32) {
        self.write_bytes(&v.to_le_bytes());
    }

    fn write_u64(&mut self, v: u64) {
        self.write_bytes(&v.to_le_bytes());
    }

    fn write_f32(&mut self, v: f32) {
        self.write_u32(v.to_bits());
    }
}

impl StateHasher {
    pub fn new() -> Self {
        Self(0xcbf2_9ce4_8422_2325)
    }

    pub fn finish(&self) -> u64 {
        self.0
    }
}

impl StateSink for StateHasher {
    fn write_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
}

impl StateSink for Vec<u8> {
    fn write_bytes(&mut self, bytes: &[u8]) {
        self.extend_from_slice(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_reference_vectors() {
        let mut h = StateHasher::new();
        assert_eq!(h.finish(), 0xcbf2_9ce4_8422_2325);
        h.write_bytes(b"a");
        assert_eq!(h.finish(), 0xaf63_dc4c_8601_ec8c);
        let mut h = StateHasher::new();
        h.write_bytes(b"foobar");
        assert_eq!(h.finish(), 0x8594_4171_f739_67e8);
    }
}
