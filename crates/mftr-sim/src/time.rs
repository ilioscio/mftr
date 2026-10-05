//! Fixed-tick time. Tick `k` simulates the interval `(t_{k-1}, t_k]`, where `t_k = k * TICK_DT`.
//! See `docs/design/03a-netcode-time-and-prediction.md` §1 and §3.

pub const TICK_HZ: u32 = 30;
pub const TICK_DT: f32 = 1.0 / TICK_HZ as f32;
pub const TICK_DT_F64: f64 = 1.0 / TICK_HZ as f64;

/// Sub-tick resolution: commands are placed inside a tick in 1/64 steps (~0.5 ms).
pub const SUBTICKS: u8 = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(pub u32);

impl Tick {
    pub fn next(self) -> Self {
        Tick(self.0 + 1)
    }

    /// Server time at the *end* of this tick, in seconds.
    pub fn end_seconds(self) -> f64 {
        self.0 as f64 * TICK_DT_F64
    }
}

/// Position of an event inside its tick interval, in `1/SUBTICKS` steps.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubTick(u8);

impl SubTick {
    pub const START: Self = SubTick(0);

    pub fn new(v: u8) -> Self {
        SubTick(v.min(SUBTICKS - 1))
    }

    pub fn get(self) -> u8 {
        self.0
    }

    /// Fraction of the tick interval elapsed before the event, in `[0, 1)`.
    pub fn fraction(self) -> f32 {
        self.0 as f32 / SUBTICKS as f32
    }
}

/// Map a continuous server time, in fractional ticks, to the tick whose interval contains it
/// and the sub-tick position inside that interval.
pub fn tick_at(time_in_ticks: f64) -> (Tick, SubTick) {
    let t = time_in_ticks.max(0.0);
    let base = t.floor();
    let sub = ((t - base) * SUBTICKS as f64).floor() as u8;
    (Tick(base as u32 + 1), SubTick::new(sub))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_at_maps_into_interval() {
        assert_eq!(tick_at(0.0), (Tick(1), SubTick::START));
        assert_eq!(tick_at(9.5), (Tick(10), SubTick::new(32)));
        assert_eq!(tick_at(9.999_999), (Tick(10), SubTick::new(63)));
        assert_eq!(tick_at(10.0), (Tick(11), SubTick::START));
    }

    #[test]
    fn subtick_saturates() {
        assert_eq!(SubTick::new(200).get(), SUBTICKS - 1);
    }
}
