//! Shared statistics and report formatting for the Netcode Lab and the UDP bot.

use mftr_client::ClientSession;

/// Scripted "player": clicks around the arena like an impatient human (3–8 clicks/s bursts).
pub struct ClickBot {
    rng: mftr_sim::rng::Pcg32,
    next_click: f64,
    pub arena_min: f32,
    pub arena_max: f32,
}

impl ClickBot {
    pub fn new(seed: u64) -> Self {
        Self { rng: mftr_sim::rng::Pcg32::new(seed, 0x626f74), next_click: 0.5, arena_min: 500.0, arena_max: 3500.0 }
    }

    /// Issue commands that are due. Returns true if any command was issued.
    pub fn act(&mut self, session: &mut ClientSession, now: f64, elapsed: f64) -> bool {
        if elapsed < self.next_click {
            return false;
        }
        let r = self.rng.next_f32();
        let issued = if r < 0.04 {
            session.stop(now).is_some()
        } else {
            let (lo, hi) = (self.arena_min, self.arena_max);
            let target = mftr_sim::Vec2::new(self.rng.range_f32(lo, hi), self.rng.range_f32(lo, hi));
            session.move_to(target, now).is_some()
        };
        // Mostly rapid re-clicks, sometimes a pause.
        let gap = if self.rng.next_f32() < 0.8 { self.rng.range_f32(0.12, 0.35) } else { self.rng.range_f32(0.5, 1.5) };
        self.next_click = elapsed + gap as f64;
        issued
    }
}

/// Tracks visible jumps in a rendered position: movement beyond what speed allows in one frame.
/// Also integrates the visible correction offset over time (03 §1 "mean visible position
/// correction during normal play").
#[derive(Default)]
pub struct JumpMeter {
    last: Option<mftr_sim::Vec2>,
    pub jumps: Vec<f32>,
    pub offset_sum: f64,
    pub frames: u64,
}

impl JumpMeter {
    /// Jumps are frame-to-frame steps more than 5 u beyond what move speed allows: a visible
    /// pop. Smoothed blends of large corrections can still register for a frame or two.
    pub fn observe(&mut self, pos: mftr_sim::Vec2, visible_correction: f32, frame_dt: f64, max_speed: f32) {
        if let Some(prev) = self.last {
            let excess = prev.distance(pos) - max_speed * frame_dt as f32;
            if excess > 5.0 {
                self.jumps.push(excess);
            }
        }
        self.last = Some(pos);
        self.offset_sum += visible_correction as f64;
        self.frames += 1;
    }
}

fn percentile(sorted: &[f32], p: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let i = ((sorted.len() - 1) as f64 * p).round() as usize;
    sorted[i]
}

#[derive(Clone, Debug, Default)]
pub struct Summary {
    pub label: String,
    pub true_rtt_ms: f64,
    pub est_rtt_ms: f64,
    pub margin_ms: f64,
    pub commands: u64,
    pub late_pct: f64,
    pub lead_p01_ms: f64,
    pub reconciliations: u64,
    pub mismatch_pct: f64,
    pub corr_mean: f32,
    pub corr_p95: f32,
    pub corr_max: f32,
    pub corr_per_min_over_15: f64,
    /// Time-averaged size of the on-screen correction offset (03 §1 metric).
    pub visible_mean: f64,
    pub jumps_per_min: f64,
    pub jump_max: f32,
    pub up_kbps: f64,
    pub down_kbps: f64,
    pub hard_resets: u64,
}

impl Summary {
    pub fn from_sessions(label: &str, true_rtt: f64, seconds: f64, sessions: &[(&ClientSession, &JumpMeter)]) -> Self {
        let n = sessions.len().max(1) as f64;
        let mut corr: Vec<f32> = sessions.iter().flat_map(|(s, _)| s.stats.corrections.iter().copied()).collect();
        corr.sort_by(f32::total_cmp);
        let mut leads: Vec<f32> =
            sessions.iter().flat_map(|(s, _)| s.stats.leads_us.iter().map(|&l| l as f32 / 1000.0)).collect();
        leads.sort_by(f32::total_cmp);
        let jumps: Vec<f32> = sessions.iter().flat_map(|(_, j)| j.jumps.iter().copied()).collect();
        let commands: u64 = sessions.iter().map(|(s, _)| s.stats.commands_issued).sum();
        let late: u64 = sessions.iter().map(|(s, _)| s.stats.commands_late).sum();
        let recon: u64 = sessions.iter().map(|(s, _)| s.stats.reconciliations).sum();
        let mism: u64 = sessions.iter().map(|(s, _)| s.stats.mismatches).sum();
        let minutes = seconds / 60.0 * n;
        Summary {
            label: label.to_string(),
            true_rtt_ms: true_rtt * 1e3,
            est_rtt_ms: sessions.iter().map(|(s, _)| s.rtt()).sum::<f64>() / n * 1e3,
            margin_ms: sessions.iter().map(|(s, _)| s.margin()).sum::<f64>() / n * 1e3,
            commands,
            late_pct: if commands > 0 { late as f64 * 100.0 / commands as f64 } else { 0.0 },
            lead_p01_ms: percentile(&leads, 0.01) as f64,
            reconciliations: recon,
            mismatch_pct: if recon > 0 { mism as f64 * 100.0 / recon as f64 } else { 0.0 },
            corr_mean: if corr.is_empty() { 0.0 } else { corr.iter().sum::<f32>() / corr.len() as f32 },
            corr_p95: percentile(&corr, 0.95),
            corr_max: corr.last().copied().unwrap_or(0.0),
            corr_per_min_over_15: corr.iter().filter(|&&c| c > 15.0).count() as f64 / minutes.max(1e-9),
            visible_mean: sessions.iter().map(|(_, j)| j.offset_sum).sum::<f64>()
                / sessions.iter().map(|(_, j)| j.frames).sum::<u64>().max(1) as f64,
            jumps_per_min: jumps.len() as f64 / minutes.max(1e-9),
            jump_max: jumps.iter().copied().fold(0.0, f32::max),
            up_kbps: sessions.iter().map(|(s, _)| s.stats.bytes_up).sum::<u64>() as f64 / 1024.0 / seconds / n,
            down_kbps: sessions.iter().map(|(s, _)| s.stats.bytes_down).sum::<u64>() as f64 / 1024.0 / seconds / n,
            hard_resets: sessions.iter().map(|(s, _)| s.stats.hard_resets).sum(),
        }
    }

    pub fn header() -> String {
        format!(
            "{:<10} {:>7} {:>7} {:>6} {:>6} {:>6} {:>8} {:>7} {:>7} {:>7} {:>7} {:>8} {:>7} {:>7} {:>7} {:>6} {:>6} {:>5}",
            "profile",
            "rtt",
            "rtt min",
            "margin",
            "cmds",
            "late%",
            "lead p1",
            "mism%",
            "corr~",
            "c p95",
            "c max",
            ">15u/min",
            "visible",
            "jump/m",
            "jmp max",
            "up KB",
            "dn KB",
            "reset"
        )
    }

    pub fn row(&self) -> String {
        format!(
            "{:<10} {:>6.0}ms {:>6.1}ms {:>4.1}ms {:>6} {:>6.2} {:>6.1}ms {:>7.2} {:>6.1}u {:>6.1}u {:>6.1}u {:>8.2} {:>6.2}u {:>7.2} {:>6.1}u {:>6.2} {:>6.2} {:>5}",
            self.label,
            self.true_rtt_ms,
            self.est_rtt_ms,
            self.margin_ms,
            self.commands,
            self.late_pct,
            self.lead_p01_ms,
            self.mismatch_pct,
            self.corr_mean,
            self.corr_p95,
            self.corr_max,
            self.corr_per_min_over_15,
            self.visible_mean,
            self.jumps_per_min,
            self.jump_max,
            self.up_kbps,
            self.down_kbps,
            self.hard_resets,
        )
    }
}
