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

/// Sub-ticks per second (64 × 30 = 1920).
pub const SUBTICKS_PER_SECOND: u64 = SUBTICKS as u64 * TICK_HZ as u64;

/// An instant on the exact integer simulation timeline, in sub-ticks (1/1920 s) since the end
/// of tick 0. Casts, missiles, stuns and cooldowns all use it, so timings are identical on the
/// server and in client prediction with no float drift (03a §3).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimTime(pub u64);

impl SimTime {
    /// The instant of a command at `(tick, sub)`: inside tick `k`'s interval `(t_{k-1}, t_k]`.
    pub fn at(tick: Tick, sub: SubTick) -> Self {
        SimTime(tick.0.saturating_sub(1) as u64 * SUBTICKS as u64 + sub.get() as u64)
    }

    /// The end of tick `k`, i.e. `t_k`.
    pub fn end_of(tick: Tick) -> Self {
        SimTime(tick.0 as u64 * SUBTICKS as u64)
    }

    pub fn plus(self, duration: SimDuration) -> Self {
        SimTime(self.0 + duration.0)
    }

    /// Seconds since `earlier` (exact integer difference, then one float conversion).
    pub fn secs_since(self, earlier: SimTime) -> f32 {
        self.0.saturating_sub(earlier.0) as f32 / SUBTICKS_PER_SECOND as f32
    }

    /// In fractional ticks (client display timelines).
    pub fn as_ticks(self) -> f64 {
        self.0 as f64 / SUBTICKS as f64
    }

    /// Wire form: whole ticks (32 bits) and the 6-bit remainder.
    pub fn split(self) -> (u32, u8) {
        ((self.0 / SUBTICKS as u64) as u32, (self.0 % SUBTICKS as u64) as u8)
    }

    pub fn join(ticks: u32, sub: u8) -> Self {
        SimTime(ticks as u64 * SUBTICKS as u64 + (sub % SUBTICKS) as u64)
    }
}

/// A duration in sub-ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SimDuration(pub u64);

impl SimDuration {
    /// From design seconds (exact for multiples of 1/1920 s, e.g. 0.25 s = 480).
    pub const fn from_millis(ms: u64) -> Self {
        SimDuration(ms * SUBTICKS_PER_SECOND / 1000)
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
