//! Blind playtest protocol (03 §14, 03a §12, M1 exit criterion).
//!
//! A session is a series of short rounds. Each round runs under a hidden **condition**: extra
//! network latency/jitter/loss added on top of the real link, plus the A/B switches under
//! test (own-missile display Option A vs. B, D12; minion bubble on/off, 03a §12). The tester
//! never sees which condition is active (the net graph is hidden) and rates every round:
//! did dodging feel **fair**, and how responsive did the champion feel (1–5)?
//!
//! Conditions are balanced: every latency profile appears with both missile options, in a
//! seeded random order. Results are tab-separated lines with the condition revealed, so a
//! session can be summarized afterwards (`mftr-tools blind-report`). The M1 exit asks for
//! dodge feel rated "fair" in ≥ 80% of rounds at 80 ms.

use crate::missiles::OwnMissileDisplay;
use mftr_net::conditioner::LinkProfile;
use mftr_sim::rng::Pcg32;

/// Latency added on top of the real link (the real one is ~0 on a LAN test setup).
pub const BLIND_PROFILES: [LinkProfile; 5] =
    [LinkProfile::PERFECT, LinkProfile::GOOD, LinkProfile::TYPICAL, LinkProfile::MID, LinkProfile::ROUGH];

/// The profile the M1 exit criterion is judged at (80 ms).
pub const EXIT_PROFILE: &str = "mid";
pub const EXIT_FAIR_SHARE: f64 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlindCondition {
    pub profile: LinkProfile,
    pub own_missiles: OwnMissileDisplay,
    pub minion_bubble: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlindPlan {
    pub seed: u64,
    pub conditions: Vec<BlindCondition>,
}

impl BlindPlan {
    /// `rounds` conditions: every profile × missile option, repeated as needed, shuffled with
    /// `seed`. The minion bubble is on in half the rounds, independently shuffled.
    pub fn new(seed: u64, rounds: usize) -> Self {
        let mut rng = Pcg32::new(seed, 0x626c_696e);
        let combos: Vec<(LinkProfile, OwnMissileDisplay)> = BLIND_PROFILES
            .iter()
            .flat_map(|p| [(*p, OwnMissileDisplay::InputTimeline), (*p, OwnMissileDisplay::Blend)])
            .collect();
        let mut picks: Vec<(LinkProfile, OwnMissileDisplay)> = Vec::with_capacity(rounds);
        while picks.len() < rounds {
            let mut block = combos.clone();
            shuffle(&mut block, &mut rng);
            picks.extend(block.into_iter().take(rounds - picks.len()));
        }
        let mut bubble: Vec<bool> = (0..rounds).map(|i| i % 2 == 0).collect();
        shuffle(&mut bubble, &mut rng);
        let conditions = picks
            .into_iter()
            .zip(bubble)
            .map(|((profile, own_missiles), minion_bubble)| BlindCondition { profile, own_missiles, minion_bubble })
            .collect();
        Self { seed, conditions }
    }
}

fn shuffle<T>(v: &mut [T], rng: &mut Pcg32) {
    for i in (1..v.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        v.swap(i, j);
    }
}

/// What the tester answered after a round.
#[derive(Clone, Debug, PartialEq)]
pub struct BlindRating {
    pub dodge_fair: bool,
    /// 1 (sluggish) – 5 (instant).
    pub responsiveness: u8,
    pub notes: String,
}

/// What the client measured during the round (never shown to the tester).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RoundStats {
    pub seconds: f64,
    /// Measured round trip, including the added latency.
    pub rtt_ms: f64,
    pub near_misses: u64,
    pub ghost_hits: u64,
    pub phantom_hits: u64,
    pub corrections_over_15: u64,
}

/// One rated round, condition revealed.
#[derive(Clone, Debug, PartialEq)]
pub struct BlindRecord {
    pub seed: u64,
    pub round: usize,
    pub profile: String,
    pub own_missiles: OwnMissileDisplay,
    pub minion_bubble: bool,
    pub rating: BlindRating,
    pub stats: RoundStats,
}

pub const TSV_HEADER: &str = "seed\tround\tprofile\town_missiles\tminion_bubble\tdodge_fair\tresponsiveness\tseconds\trtt_ms\tnear_misses\tghost_hits\tphantom_hits\tcorrections_over_15\tnotes";

fn option_name(o: OwnMissileDisplay) -> &'static str {
    match o {
        OwnMissileDisplay::InputTimeline => "A",
        OwnMissileDisplay::Blend => "B",
    }
}

impl BlindRecord {
    pub fn new(plan: &BlindPlan, round: usize, rating: BlindRating, stats: RoundStats) -> Self {
        let c = plan.conditions[round];
        Self {
            seed: plan.seed,
            round,
            profile: c.profile.name.to_string(),
            own_missiles: c.own_missiles,
            minion_bubble: c.minion_bubble,
            rating,
            stats,
        }
    }

    pub fn to_tsv(&self) -> String {
        let notes: String = self.rating.notes.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
        let s = &self.stats;
        format!(
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.1}\t{:.1}\t{}\t{}\t{}\t{}\t{}",
            self.seed,
            self.round + 1,
            self.profile,
            option_name(self.own_missiles),
            self.minion_bubble as u8,
            if self.rating.dodge_fair { "fair" } else { "unfair" },
            self.rating.responsiveness,
            s.seconds,
            s.rtt_ms,
            s.near_misses,
            s.ghost_hits,
            s.phantom_hits,
            s.corrections_over_15,
            notes.trim(),
        )
    }

    /// Parse one line written by [`Self::to_tsv`] (header and malformed lines give `None`).
    pub fn from_tsv(line: &str) -> Option<Self> {
        let f: Vec<&str> = line.splitn(14, '\t').collect();
        if f.len() < 13 || f[0] == "seed" {
            return None;
        }
        Some(Self {
            seed: f[0].parse().ok()?,
            round: f[1].parse::<usize>().ok()?.checked_sub(1)?,
            profile: f[2].to_string(),
            own_missiles: match f[3] {
                "A" => OwnMissileDisplay::InputTimeline,
                "B" => OwnMissileDisplay::Blend,
                _ => return None,
            },
            minion_bubble: f[4] == "1",
            rating: BlindRating {
                dodge_fair: match f[5] {
                    "fair" => true,
                    "unfair" => false,
                    _ => return None,
                },
                responsiveness: f[6].parse().ok()?,
                notes: f.get(13).unwrap_or(&"").to_string(),
            },
            stats: RoundStats {
                seconds: f[7].parse().ok()?,
                rtt_ms: f[8].parse().ok()?,
                near_misses: f[9].parse().ok()?,
                ghost_hits: f[10].parse().ok()?,
                phantom_hits: f[11].parse().ok()?,
                corrections_over_15: f[12].parse().ok()?,
            },
        })
    }
}

/// Ratings grouped by one key: rounds, share rated fair, mean responsiveness.
#[derive(Clone, Debug, PartialEq)]
pub struct Tally {
    pub key: String,
    pub rounds: usize,
    pub fair_share: f64,
    pub responsiveness: f64,
}

fn tally<'a>(key: String, records: impl Iterator<Item = &'a BlindRecord>) -> Tally {
    let (mut n, mut fair, mut resp) = (0usize, 0usize, 0u64);
    for r in records {
        n += 1;
        fair += r.rating.dodge_fair as usize;
        resp += r.rating.responsiveness as u64;
    }
    let d = n.max(1) as f64;
    Tally { key, rounds: n, fair_share: fair as f64 / d, responsiveness: resp as f64 / d }
}

/// Per latency profile (in [`BLIND_PROFILES`] order), per missile option, and per bubble setting.
pub fn summarize(records: &[BlindRecord]) -> (Vec<Tally>, Vec<Tally>, Vec<Tally>) {
    let by_profile = BLIND_PROFILES
        .iter()
        .map(|p| tally(p.name.to_string(), records.iter().filter(|r| r.profile == p.name)))
        .filter(|t| t.rounds > 0)
        .collect();
    let by_option = [OwnMissileDisplay::InputTimeline, OwnMissileDisplay::Blend]
        .into_iter()
        .map(|o| tally(format!("option {}", option_name(o)), records.iter().filter(|r| r.own_missiles == o)))
        .collect();
    let by_bubble = [true, false]
        .into_iter()
        .map(|b| {
            let key = if b { "bubble on" } else { "bubble off" }.to_string();
            tally(key, records.iter().filter(|r| r.minion_bubble == b))
        })
        .collect();
    (by_profile, by_option, by_bubble)
}

/// The M1 exit check: `Some(passed)` once there are rounds at the exit profile.
pub fn exit_verdict(records: &[BlindRecord]) -> Option<(bool, Tally)> {
    let t = tally(EXIT_PROFILE.to_string(), records.iter().filter(|r| r.profile == EXIT_PROFILE));
    (t.rounds > 0).then_some((t.fair_share >= EXIT_FAIR_SHARE, t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plans_are_balanced_and_seeded() {
        let a = BlindPlan::new(7, 10);
        assert_eq!(a, BlindPlan::new(7, 10));
        assert_ne!(a, BlindPlan::new(8, 10));
        for p in BLIND_PROFILES {
            for o in [OwnMissileDisplay::InputTimeline, OwnMissileDisplay::Blend] {
                let n = a.conditions.iter().filter(|c| c.profile == p && c.own_missiles == o).count();
                assert_eq!(n, 1, "{} {o:?}", p.name);
            }
        }
        assert_eq!(a.conditions.iter().filter(|c| c.minion_bubble).count(), 5);
        assert_eq!(BlindPlan::new(1, 23).conditions.len(), 23);
    }

    #[test]
    fn records_round_trip_through_tsv_and_summarize() {
        let plan = BlindPlan::new(3, 10);
        let mut lines = vec![TSV_HEADER.to_string()];
        for round in 0..10 {
            let fair = plan.conditions[round].profile.latency < 0.035; // 80 ms and up "unfair"
            let rating = BlindRating { dodge_fair: fair, responsiveness: 4, notes: "ok\tfine\n".into() };
            let stats = RoundStats { seconds: 60.0, rtt_ms: 81.5, near_misses: 12, ..Default::default() };
            lines.push(BlindRecord::new(&plan, round, rating, stats).to_tsv());
        }
        let records: Vec<BlindRecord> = lines.iter().filter_map(|l| BlindRecord::from_tsv(l)).collect();
        assert_eq!(records.len(), 10);
        assert_eq!(records[0].rating.notes, "ok fine");
        assert_eq!(records[0].to_tsv(), lines[1]);
        let (by_profile, by_option, _) = summarize(&records);
        assert_eq!(by_profile.len(), 5);
        assert!(by_option.iter().all(|t| t.rounds == 5));
        let (passed, t) = exit_verdict(&records).unwrap();
        assert_eq!(t.rounds, 2);
        assert!(!passed, "80 ms rated unfair in this fake session");
    }
}
