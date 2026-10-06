//! Little-endian bit-packing. Reads past the end return `DecodeError` instead of panicking;
//! every decoder in this crate is fed untrusted bytes.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DecodeError {
    UnexpectedEnd,
    Invalid(&'static str),
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecodeError::UnexpectedEnd => write!(f, "unexpected end of packet"),
            DecodeError::Invalid(what) => write!(f, "invalid {what}"),
        }
    }
}

impl std::error::Error for DecodeError {}

#[derive(Default)]
pub struct BitWriter {
    bytes: Vec<u8>,
    acc: u64,
    nbits: u32,
}

impl BitWriter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Write the low `bits` bits of `value` (`bits` ≤ 32).
    pub fn write(&mut self, value: u64, bits: u32) {
        assert!(bits <= 32);
        let v = value & ((1u64 << bits) - 1);
        self.acc |= v << self.nbits;
        self.nbits += bits;
        while self.nbits >= 8 {
            self.bytes.push(self.acc as u8);
            self.acc >>= 8;
            self.nbits -= 8;
        }
    }

    pub fn write_bool(&mut self, v: bool) {
        self.write(v as u64, 1);
    }

    pub fn write_u8(&mut self, v: u8) {
        self.write(v as u64, 8);
    }

    pub fn write_u16(&mut self, v: u16) {
        self.write(v as u64, 16);
    }

    pub fn write_u32(&mut self, v: u32) {
        self.write(v as u64, 32);
    }

    pub fn write_u64(&mut self, v: u64) {
        self.write(v & 0xffff_ffff, 32);
        self.write(v >> 32, 32);
    }

    pub fn write_i16(&mut self, v: i16) {
        self.write(v as u16 as u64, 16);
    }

    /// Lossless: the raw IEEE-754 bits.
    pub fn write_f32(&mut self, v: f32) {
        self.write_u32(v.to_bits());
    }

    pub fn bit_len(&self) -> usize {
        self.bytes.len() * 8 + self.nbits as usize
    }

    pub fn finish(mut self) -> Vec<u8> {
        if self.nbits > 0 {
            self.bytes.push(self.acc as u8);
        }
        self.bytes
    }
}

pub struct BitReader<'a> {
    data: &'a [u8],
    pos: usize,
    acc: u64,
    nbits: u32,
}

impl<'a> BitReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, acc: 0, nbits: 0 }
    }

    pub fn read(&mut self, bits: u32) -> Result<u64, DecodeError> {
        assert!(bits <= 32);
        while self.nbits < bits {
            let byte = *self.data.get(self.pos).ok_or(DecodeError::UnexpectedEnd)?;
            self.acc |= (byte as u64) << self.nbits;
            self.pos += 1;
            self.nbits += 8;
        }
        let v = self.acc & ((1u64 << bits) - 1);
        self.acc >>= bits;
        self.nbits -= bits;
        Ok(v)
    }

    pub fn read_bool(&mut self) -> Result<bool, DecodeError> {
        Ok(self.read(1)? != 0)
    }

    pub fn read_u8(&mut self) -> Result<u8, DecodeError> {
        Ok(self.read(8)? as u8)
    }

    pub fn read_u16(&mut self) -> Result<u16, DecodeError> {
        Ok(self.read(16)? as u16)
    }

    pub fn read_u32(&mut self) -> Result<u32, DecodeError> {
        Ok(self.read(32)? as u32)
    }

    pub fn read_u64(&mut self) -> Result<u64, DecodeError> {
        let lo = self.read(32)?;
        Ok(lo | (self.read(32)? << 32))
    }

    pub fn read_i16(&mut self) -> Result<i16, DecodeError> {
        Ok(self.read(16)? as u16 as i16)
    }

    pub fn read_f32(&mut self) -> Result<f32, DecodeError> {
        Ok(f32::from_bits(self.read_u32()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_mixed_widths() {
        let mut w = BitWriter::new();
        w.write(5, 3);
        w.write_bool(true);
        w.write_u16(0xBEEF);
        w.write(0x3F, 6);
        w.write_u32(0xDEAD_BEEF);
        w.write_i16(-1234);
        w.write_f32(-0.0);
        w.write_f32(1234.5678);
        let bytes = w.finish();
        let mut r = BitReader::new(&bytes);
        assert_eq!(r.read(3).unwrap(), 5);
        assert!(r.read_bool().unwrap());
        assert_eq!(r.read_u16().unwrap(), 0xBEEF);
        assert_eq!(r.read(6).unwrap(), 0x3F);
        assert_eq!(r.read_u32().unwrap(), 0xDEAD_BEEF);
        assert_eq!(r.read_i16().unwrap(), -1234);
        assert_eq!(r.read_f32().unwrap().to_bits(), (-0.0f32).to_bits());
        assert_eq!(r.read_f32().unwrap(), 1234.5678);
    }

    #[test]
    fn reading_past_end_is_an_error() {
        let mut r = BitReader::new(&[0xFF]);
        assert_eq!(r.read(8).unwrap(), 0xFF);
        assert_eq!(r.read(1), Err(DecodeError::UnexpectedEnd));
    }
}
