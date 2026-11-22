//! Content ids (D83, 11 §5): a custom map or a champion file is named by the **BLAKE2s-256**
//! hash of its text. The wire, the server's offers and the client's cache all use it, so a
//! crafted file can't pose as another: FNV-64, used before, is easy to collide on purpose.
//! BLAKE2s is the transport's own hash (D40), so this adds no dependency.

use std::fmt;

use blake2::{Blake2s256, Digest};

use crate::bits::{BitReader, BitWriter, DecodeError};

/// A file's content id: BLAKE2s-256 of its text.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct ContentId(pub [u8; 32]);

impl ContentId {
    pub fn of(text: &str) -> ContentId {
        ContentId(Blake2s256::digest(text.as_bytes()).into())
    }

    /// The first eight bytes, for a short name (logs, `Hello::roster`).
    pub fn short(&self) -> u64 {
        u64::from_le_bytes(self.0[..8].try_into().expect("8 bytes"))
    }

    pub fn write(&self, w: &mut BitWriter) {
        for b in self.0 {
            w.write_u8(b);
        }
    }

    pub fn read(r: &mut BitReader<'_>) -> Result<ContentId, DecodeError> {
        let mut id = [0u8; 32];
        for b in &mut id {
            *b = r.read_u8()?;
        }
        Ok(ContentId(id))
    }
}

impl fmt::Debug for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in &self.0[..8] {
            write!(f, "{b:02x}")?;
        }
        f.write_str("…")
    }
}

impl fmt::Display for ContentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_blake2s_of_the_text() {
        // BLAKE2s-256 of "abc" (RFC 7693, appendix B).
        let id = ContentId::of("abc");
        assert_eq!(id.to_string(), "508c5e8c327c14e2e1a72ba34eeb452f37458b209ed63a294d999b4c86675982");
        assert_ne!(ContentId::of("abd"), id);
        assert_eq!(id.short(), u64::from_le_bytes([0x50, 0x8c, 0x5e, 0x8c, 0x32, 0x7c, 0x14, 0xe2]));
    }
}
