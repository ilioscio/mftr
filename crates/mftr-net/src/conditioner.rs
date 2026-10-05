//! Link conditioner: simulated latency, jitter, loss and duplication (03 §14).
//! Used by the headless Netcode Lab (virtual time) and by the UDP bot (wall time).

use mftr_sim::rng::Pcg32;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinkProfile {
    pub name: &'static str,
    /// One-way base latency, seconds.
    pub latency: f64,
    /// Standard deviation of extra one-way delay, seconds.
    pub jitter: f64,
    /// Probability a packet is dropped.
    pub loss: f64,
    /// Probability a packet is delivered twice.
    pub duplicate: f64,
}

impl LinkProfile {
    /// A profile described by round-trip time, split evenly into two one-way legs.
    pub const fn rtt(name: &'static str, rtt_ms: f64, jitter_ms: f64, loss: f64) -> Self {
        Self { name, latency: rtt_ms / 2000.0, jitter: jitter_ms / 1000.0, loss, duplicate: 0.0 }
    }

    pub const PERFECT: Self = Self::rtt("perfect", 0.0, 0.0, 0.0);
    pub const LAN: Self = Self::rtt("lan", 2.0, 0.5, 0.0);
    pub const GOOD: Self = Self::rtt("good", 30.0, 2.0, 0.0);
    pub const TYPICAL: Self = Self::rtt("typical", 60.0, 5.0, 0.005);
    pub const ROUGH: Self = Self::rtt("rough", 120.0, 20.0, 0.02);
    pub const AWFUL: Self = Self::rtt("awful", 200.0, 40.0, 0.05);

    pub const ALL: [Self; 6] = [Self::PERFECT, Self::LAN, Self::GOOD, Self::TYPICAL, Self::ROUGH, Self::AWFUL];

    pub fn by_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|p| p.name == name)
    }
}

pub struct LinkConditioner {
    profile: LinkProfile,
    rng: Pcg32,
}

impl LinkConditioner {
    pub fn new(profile: LinkProfile, seed: u64) -> Self {
        Self { profile, rng: Pcg32::new(seed, 0x6c69_6e6b) }
    }

    pub fn profile(&self) -> LinkProfile {
        self.profile
    }

    /// Delivery times for a packet sent at `now`. Empty = lost; two entries = duplicated.
    pub fn schedule(&mut self, now: f64) -> Vec<f64> {
        if self.rng.next_f64() < self.profile.loss {
            return Vec::new();
        }
        let mut out = vec![now + self.delay()];
        if self.rng.next_f64() < self.profile.duplicate {
            out.push(now + self.delay());
        }
        out
    }

    fn delay(&mut self) -> f64 {
        // Approximately normal (Irwin–Hall, 4 uniforms), clamped so delays never go below
        // half the base latency.
        let n: f64 = (0..4).map(|_| self.rng.next_f64()).sum::<f64>() - 2.0;
        let extra = n * self.profile.jitter * 3f64.sqrt();
        (self.profile.latency + extra).max(self.profile.latency * 0.5)
    }
}

/// A one-directional simulated link carrying byte packets in virtual time.
pub struct SimLink {
    cond: LinkConditioner,
    queue: BinaryHeap<Reverse<(u64, u64, Vec<u8>)>>,
    counter: u64,
    pub bytes: u64,
    pub packets: u64,
}

impl SimLink {
    pub fn new(profile: LinkProfile, seed: u64) -> Self {
        Self { cond: LinkConditioner::new(profile, seed), queue: BinaryHeap::new(), counter: 0, bytes: 0, packets: 0 }
    }

    pub fn send(&mut self, bytes: Vec<u8>, now: f64) {
        self.bytes += bytes.len() as u64;
        self.packets += 1;
        for at in self.cond.schedule(now) {
            self.counter += 1;
            let at_ns = (at * 1e9) as u64;
            self.queue.push(Reverse((at_ns, self.counter, bytes.clone())));
        }
    }

    /// Next packet whose delivery time has passed.
    pub fn recv(&mut self, now: f64) -> Option<Vec<u8>> {
        let now_ns = (now * 1e9) as u64;
        if self.queue.peek().is_some_and(|Reverse((at, _, _))| *at <= now_ns) {
            self.queue.pop().map(|Reverse((_, _, b))| b)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loss_rate_and_latency_are_roughly_right() {
        let mut c = LinkConditioner::new(LinkProfile::ROUGH, 1);
        let mut delivered = 0;
        let mut total_delay = 0.0;
        for _ in 0..20_000 {
            let d = c.schedule(0.0);
            if let Some(at) = d.first() {
                delivered += 1;
                total_delay += at;
            }
        }
        let loss = 1.0 - delivered as f64 / 20_000.0;
        assert!((loss - 0.02).abs() < 0.005, "loss {loss}");
        let mean = total_delay / delivered as f64;
        assert!((mean - 0.060).abs() < 0.003, "mean one-way {mean}");
    }

    #[test]
    fn sim_link_delivers_in_time_order() {
        let mut l = SimLink::new(LinkProfile::GOOD, 3);
        l.send(vec![1], 0.0);
        assert!(l.recv(0.001).is_none());
        assert_eq!(l.recv(1.0), Some(vec![1]));
        assert!(l.recv(1.0).is_none());
    }
}
