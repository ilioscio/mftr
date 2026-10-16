//! Difficulty tiers (17 §3). A tier changes how well a bot plays (how fast it reacts, how well it
//! aims, how often it thinks and acts, how much of the game it reads), never what it gets.

/// How strong a bot plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Tier {
    /// The M2 bot, kept as a baseline for measurements (17 §11). Not offered to players.
    Legacy,
    Recruit,
    #[default]
    Veteran,
    Elite,
    Nightmare,
}

/// What a cooldown tracker remembers of enemy casts (17 §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tracking {
    /// Nothing: an enemy's abilities are assumed ready.
    None,
    /// Ultimates only.
    Ultimates,
    /// Every ability.
    Abilities,
    /// Every ability and the utility spells.
    Everything,
}

/// A tier's numbers.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Params {
    /// From first sight of a threat or an opening to the earliest response (ms), and its
    /// spread either way.
    pub reaction_ms: f32,
    pub reaction_spread_ms: f32,
    /// Ticks between thinks while fighting (an enemy champion or a threat near) and otherwise.
    pub think_fight: u32,
    pub think_calm: u32,
    /// Orders a second, at most (shopping and leveling don't count: they're menu clicks).
    pub orders_per_second: f32,
    /// How much of a moving target's motion an aim leads (0: aims at where it stands).
    pub lead: f32,
    /// Aim error across the shot (u, one standard deviation, at 1,000 u).
    pub aim_error: f32,
    /// Chance to try to dodge a threat it sees in time.
    pub dodge: f32,
    /// Last-hit timing error, either way (ms).
    pub last_hit_error_ms: f32,
    /// Noise on its read of a fight (17 §5): the outcome is moved by up to this, either way.
    pub read_noise: f32,
    pub tracking: Tracking,
    /// How much of the map game it plays (0: its lane only; 1: basic ganks, objectives and
    /// grouping; 2: full; 3: also counter-jungling and objective trades).
    pub strategy: u8,
    /// Steps between attacks to keep its distance (ranged champions).
    pub kites: bool,
    /// Seconds it keeps chasing after a chase stops paying (a learner's over-chase).
    pub overchase: f32,
    /// Share of good ability openings it takes.
    pub ability_use: f32,
    /// Share of its plans it announces with a ping.
    pub pings: f32,
    /// Health share below which it heads home (lower: greedier).
    pub retreat_health: f32,
}

impl Tier {
    /// The tiers players choose from, easiest first.
    pub const PLAYABLE: [Tier; 4] = [Tier::Recruit, Tier::Veteran, Tier::Elite, Tier::Nightmare];

    pub fn name(self) -> &'static str {
        match self {
            Tier::Legacy => "Legacy",
            Tier::Recruit => "Recruit",
            Tier::Veteran => "Veteran",
            Tier::Elite => "Elite",
            Tier::Nightmare => "Nightmare",
        }
    }

    /// A tier by name (case doesn't matter).
    pub fn by_name(name: &str) -> Option<Tier> {
        [Tier::Legacy, Tier::Recruit, Tier::Veteran, Tier::Elite, Tier::Nightmare]
            .into_iter()
            .find(|t| t.name().eq_ignore_ascii_case(name.trim()))
    }

    /// On the wire: 0–3 for the playable tiers (the legacy bot is never sent).
    pub fn wire(self) -> u8 {
        match self {
            Tier::Legacy | Tier::Recruit => 0,
            Tier::Veteran => 1,
            Tier::Elite => 2,
            Tier::Nightmare => 3,
        }
    }

    pub fn from_wire(v: u8) -> Option<Tier> {
        Tier::PLAYABLE.get(v as usize).copied()
    }

    pub fn params(self) -> Params {
        tune(self, self.table())
    }

    fn table(self) -> Params {
        match self {
            Tier::Legacy | Tier::Recruit => Params {
                reaction_ms: 650.0,
                reaction_spread_ms: 150.0,
                think_fight: 8,
                think_calm: 15,
                orders_per_second: 3.0,
                lead: 0.0,
                aim_error: 110.0,
                dodge: 0.35,
                last_hit_error_ms: 250.0,
                read_noise: 0.3,
                tracking: Tracking::None,
                strategy: 0,
                kites: false,
                overchase: 2.5,
                ability_use: 0.55,
                pings: 0.4,
                retreat_health: 0.2,
            },
            Tier::Veteran => Params {
                reaction_ms: 450.0,
                reaction_spread_ms: 120.0,
                think_fight: 5,
                think_calm: 10,
                orders_per_second: 5.0,
                lead: 0.5,
                aim_error: 70.0,
                dodge: 0.6,
                last_hit_error_ms: 130.0,
                read_noise: 0.15,
                tracking: Tracking::Ultimates,
                strategy: 1,
                kites: false,
                overchase: 1.0,
                ability_use: 0.8,
                pings: 0.8,
                retreat_health: 0.28,
            },
            Tier::Elite => Params {
                reaction_ms: 300.0,
                reaction_spread_ms: 80.0,
                think_fight: 4,
                think_calm: 8,
                orders_per_second: 7.0,
                lead: 0.85,
                aim_error: 40.0,
                dodge: 0.85,
                last_hit_error_ms: 60.0,
                read_noise: 0.07,
                tracking: Tracking::Abilities,
                strategy: 2,
                kites: true,
                overchase: 0.3,
                ability_use: 0.95,
                pings: 1.0,
                retreat_health: 0.3,
            },
            Tier::Nightmare => Params {
                reaction_ms: 220.0,
                reaction_spread_ms: 50.0,
                think_fight: 3,
                think_calm: 6,
                orders_per_second: 9.0,
                lead: 1.0,
                aim_error: 25.0,
                dodge: 0.95,
                last_hit_error_ms: 30.0,
                read_noise: 0.03,
                tracking: Tracking::Everything,
                strategy: 3,
                kites: true,
                overchase: 0.0,
                ability_use: 1.0,
                pings: 1.0,
                retreat_health: 0.3,
            },
        }
    }
}

/// Development overrides for tuning (`MFTR_BOTS_TUNE=nightmare.think_fight=4,elite.dodge=0.9`):
/// a tier's number set for an experiment. Read once; unknown names are ignored.
fn tune(tier: Tier, mut p: Params) -> Params {
    static TUNE: std::sync::OnceLock<Vec<(String, String, f32)>> = std::sync::OnceLock::new();
    let all = TUNE.get_or_init(|| {
        std::env::var("MFTR_BOTS_TUNE")
            .map(|v| {
                v.split(',')
                    .filter_map(|kv| {
                        let (k, v) = kv.split_once('=')?;
                        let (t, f) = k.trim().split_once('.')?;
                        Some((t.to_ascii_lowercase(), f.to_string(), v.trim().parse().ok()?))
                    })
                    .collect()
            })
            .unwrap_or_default()
    });
    for (_, f, v) in all.iter().filter(|(t, ..)| *t == tier.name().to_ascii_lowercase()) {
        let v = *v;
        match f.as_str() {
            "reaction_ms" => p.reaction_ms = v,
            "reaction_spread_ms" => p.reaction_spread_ms = v,
            "think_fight" => p.think_fight = v as u32,
            "think_calm" => p.think_calm = v as u32,
            "orders_per_second" => p.orders_per_second = v,
            "lead" => p.lead = v,
            "aim_error" => p.aim_error = v,
            "dodge" => p.dodge = v,
            "last_hit_error_ms" => p.last_hit_error_ms = v,
            "read_noise" => p.read_noise = v,
            "tracking" => {
                p.tracking = match v as u8 {
                    0 => Tracking::None,
                    1 => Tracking::Ultimates,
                    2 => Tracking::Abilities,
                    _ => Tracking::Everything,
                }
            }
            "strategy" => p.strategy = v as u8,
            "kites" => p.kites = v != 0.0,
            "overchase" => p.overchase = v,
            "ability_use" => p.ability_use = v,
            "pings" => p.pings = v,
            "retreat_health" => p.retreat_health = v,
            _ => {}
        }
    }
    p
}

/// Development switches for experiments (`MFTR_BOTS_OFF=name,name`): turn a feature off to
/// measure it. Read once.
pub fn off(feature: &str) -> bool {
    static OFF: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    OFF.get_or_init(|| {
        std::env::var("MFTR_BOTS_OFF").map(|v| v.split(',').map(|s| s.trim().to_string()).collect()).unwrap_or_default()
    })
    .iter()
    .any(|f| f == feature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stronger_tiers_react_faster_aim_better_and_read_more() {
        let p: Vec<Params> = Tier::PLAYABLE.iter().map(|t| t.params()).collect();
        for w in p.windows(2) {
            assert!(w[1].reaction_ms < w[0].reaction_ms);
            assert!(w[1].aim_error < w[0].aim_error);
            assert!(w[1].dodge > w[0].dodge);
            assert!(w[1].think_fight <= w[0].think_fight);
            assert!(w[1].orders_per_second > w[0].orders_per_second);
            assert!(w[1].tracking >= w[0].tracking);
            assert!(w[1].strategy > w[0].strategy);
        }
        // Human-limited (17 §1): even the top tier reacts no faster than a quick player.
        assert!(p[3].reaction_ms - p[3].reaction_spread_ms >= 150.0);
        for t in Tier::PLAYABLE {
            assert_eq!(Tier::from_wire(t.wire()), Some(t));
            assert_eq!(Tier::by_name(t.name()), Some(t));
        }
    }
}
