//! PCG32 (O'Neill, `pcg32_srandom_r` seeding). Owned by the simulation state, so every
//! random outcome is reproducible from the match seed.

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pcg32 {
    state: u64,
    inc: u64,
}

impl Pcg32 {
    pub fn new(seed: u64, stream: u64) -> Self {
        let mut r = Self { state: 0, inc: (stream << 1) | 1 };
        r.next_u32();
        r.state = r.state.wrapping_add(seed);
        r.next_u32();
        r
    }

    pub fn next_u32(&mut self) -> u32 {
        let old = self.state;
        self.state = old.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(self.inc);
        let xorshifted = (((old >> 18) ^ old) >> 27) as u32;
        let rot = (old >> 59) as u32;
        xorshifted.rotate_right(rot)
    }

    /// Uniform in `[0, 1)` with 24 bits of precision.
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 * (1.0 / (1u32 << 24) as f32)
    }

    /// Uniform in `[0, 1)` with 53 bits of precision. Tools only; keep `f64` out of sim logic.
    pub fn next_f64(&mut self) -> f64 {
        let hi = (self.next_u32() >> 5) as u64;
        let lo = (self.next_u32() >> 6) as u64;
        ((hi << 26) | lo) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    pub fn range_f32(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.next_f32()
    }

    pub fn state_parts(&self) -> (u64, u64) {
        (self.state, self.inc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference output of the PCG reference implementation's `pcg32-demo` (seed 42, stream 54).
    #[test]
    fn matches_reference_sequence() {
        let mut r = Pcg32::new(42, 54);
        let expected = [0xa15c02b7, 0x7b47f409, 0xba1d3330, 0x83d2f293, 0xbfa4784b, 0xcbed606e];
        for e in expected {
            assert_eq!(r.next_u32(), e);
        }
    }

    #[test]
    fn f32_in_unit_interval() {
        let mut r = Pcg32::new(1, 1);
        for _ in 0..10_000 {
            let v = r.next_f32();
            assert!((0.0..1.0).contains(&v));
        }
    }
}
