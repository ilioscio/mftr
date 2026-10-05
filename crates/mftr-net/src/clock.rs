//! Client clock synchronization (03a §10.1).
//!
//! Each sample pairs a measured round trip with the server time at which the server sent the
//! packet. Within a sliding window, the lowest-RTT sample has the least queuing noise, so it
//! sets the target offset. The applied offset converges by time dilation (≤ 3%) and only
//! hard-resyncs on large errors.

use std::collections::VecDeque;

const WINDOW_SECONDS: f64 = 2.0;
const MAX_DILATION: f64 = 0.03;
const HARD_RESYNC: f64 = 0.25;

#[derive(Clone, Debug, Default)]
pub struct ClockSync {
    samples: VecDeque<Sample>,
    offset: Option<f64>,
    target: f64,
    rtt_min: f64,
    jitter: f64,
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    at_local: f64,
    rtt: f64,
    offset: f64,
}

impl ClockSync {
    /// `server_send_time`: server clock (seconds) when the echoing packet left the server.
    pub fn add_sample(&mut self, now_local: f64, rtt: f64, server_send_time: f64) {
        let rtt = rtt.max(0.0);
        let offset = server_send_time + rtt / 2.0 - now_local;
        self.samples.push_back(Sample { at_local: now_local, rtt, offset });
        while self.samples.len() > 1 && self.samples.front().is_some_and(|s| now_local - s.at_local > WINDOW_SECONDS) {
            self.samples.pop_front();
        }
        let best = self.samples.iter().min_by(|a, b| a.rtt.total_cmp(&b.rtt)).copied().unwrap();
        self.target = best.offset;
        self.rtt_min = best.rtt;
        // Jitter: smoothed absolute deviation above the window minimum.
        self.jitter += ((rtt - best.rtt) - self.jitter) * 0.1;
        match self.offset {
            None => self.offset = Some(self.target),
            Some(o) if (o - self.target).abs() > HARD_RESYNC => self.offset = Some(self.target),
            _ => {}
        }
    }

    /// Converge the applied offset toward the target. Call once per frame with the local dt.
    pub fn update(&mut self, dt_local: f64) {
        if let Some(o) = self.offset.as_mut() {
            let step = MAX_DILATION * dt_local.max(0.0);
            *o += (self.target - *o).clamp(-step, step);
        }
    }

    pub fn is_synced(&self) -> bool {
        self.offset.is_some()
    }

    /// Estimated current server time, in seconds.
    pub fn server_time(&self, now_local: f64) -> Option<f64> {
        self.offset.map(|o| now_local + o)
    }

    pub fn rtt(&self) -> f64 {
        self.rtt_min
    }

    pub fn jitter(&self) -> f64 {
        self.jitter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converges_on_min_rtt_sample() {
        let mut c = ClockSync::default();
        // True offset: server = local + 10.0. RTT 100 ms, symmetric. A noisy sample first.
        let true_offset = 10.0;
        let send = |local_recv: f64, rtt: f64| local_recv + true_offset - rtt / 2.0;
        c.add_sample(1.0, 0.180, send(1.0, 0.180) - 0.03); // asymmetric queuing on the way back
        c.add_sample(1.1, 0.100, send(1.1, 0.100));
        assert!((c.target - true_offset).abs() < 1e-9);
        for _ in 0..200 {
            c.update(0.016);
        }
        assert!((c.server_time(5.0).unwrap() - 15.0).abs() < 1e-6);
        assert!((c.rtt() - 0.100).abs() < 1e-9);
    }
}
