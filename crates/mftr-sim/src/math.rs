//! Deterministic 2D math.
//!
//! Only IEEE-754 correctly-rounded operations (`+ - * /`, `sqrt`, `round`) are used, so
//! results are bit-identical on every platform. Rust never contracts `a * b + c` into an
//! FMA on its own; don't call `mul_add` in simulation code.

use core::ops::{Add, AddAssign, Mul, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn dot(self, o: Self) -> f32 {
        self.x * o.x + self.y * o.y
    }

    pub fn length_sq(self) -> f32 {
        self.dot(self)
    }

    pub fn length(self) -> f32 {
        self.length_sq().sqrt()
    }

    pub fn distance(self, o: Self) -> f32 {
        (o - self).length()
    }

    /// Unit vector in the same direction, or zero for a zero-length input.
    pub fn normalize_or_zero(self) -> Self {
        let l = self.length();
        if l > 0.0 { Self::new(self.x / l, self.y / l) } else { Self::ZERO }
    }

    pub fn lerp(self, o: Self, t: f32) -> Self {
        self + (o - self) * t
    }

    /// Raw bit patterns, for exact comparison and hashing.
    pub fn to_bits(self) -> [u32; 2] {
        [self.x.to_bits(), self.y.to_bits()]
    }
}

impl Add for Vec2 {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl AddAssign for Vec2 {
    fn add_assign(&mut self, o: Self) {
        *self = *self + o;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

impl SubAssign for Vec2 {
    fn sub_assign(&mut self, o: Self) {
        *self = *self - o;
    }
}

impl Mul<f32> for Vec2 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

/// A map point quantized to 0.25 u: 16 bits per axis, covering 0..16384 u.
///
/// Commands carry quantized points, so the client's prediction and the server
/// simulate with bit-identical targets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct QPoint {
    pub x: u16,
    pub y: u16,
}

impl QPoint {
    pub const STEP: f32 = 0.25;

    pub fn from_vec2(v: Vec2) -> Self {
        Self { x: quantize(v.x), y: quantize(v.y) }
    }

    pub fn to_vec2(self) -> Vec2 {
        Vec2::new(self.x as f32 * Self::STEP, self.y as f32 * Self::STEP)
    }
}

fn quantize(c: f32) -> u16 {
    // NaN casts to 0; out-of-range values clamp to the map edge.
    (c / QPoint::STEP).round().clamp(0.0, u16::MAX as f32) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qpoint_round_trip_is_exact_on_grid() {
        let q = QPoint::from_vec2(Vec2::new(1234.25, 15000.75));
        assert_eq!(q.to_vec2(), Vec2::new(1234.25, 15000.75));
        assert_eq!(QPoint::from_vec2(q.to_vec2()), q);
    }

    #[test]
    fn qpoint_clamps_out_of_range() {
        assert_eq!(QPoint::from_vec2(Vec2::new(-5.0, 99999.0)), QPoint { x: 0, y: u16::MAX });
        assert_eq!(QPoint::from_vec2(Vec2::new(f32::NAN, 0.1)), QPoint { x: 0, y: 0 });
    }

    #[test]
    fn normalize_zero_is_zero() {
        assert_eq!(Vec2::ZERO.normalize_or_zero(), Vec2::ZERO);
        let n = Vec2::new(3.0, 4.0).normalize_or_zero();
        assert_eq!(n, Vec2::new(0.6, 0.8));
    }
}
