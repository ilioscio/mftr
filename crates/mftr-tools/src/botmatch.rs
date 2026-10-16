//! `mftr-tools botmatch`: server bots play whole matches, as fast as they can, and we measure
//! them (17 §11): who wins, how each champion did, what the bots cost; every replay must
//! re-simulate exactly.

use mftr_server::{BotMatch, BotMatchOptions, Scenario, ServerConfig, Tier};
use mftr_sim::{ChampionId, Team};

pub struct BotmatchConfig {
    pub scenario: Scenario,
    pub seed: u64,
    pub seeds: u64,
    pub minutes: f64,
    pub team: Vec<ChampionId>,
    pub tiers: [Tier; 2],
    pub replay: Option<String>,
    pub trace: Option<u8>,
}

fn clock(ticks: u32) -> String {
    let s = ticks / 30;
    format!("{}:{:02}", s / 60, s % 60)
}

/// One match in full (a single seed).
fn detail(cfg: &BotmatchConfig, m: &BotMatch, secs: f64) {
    if cfg.scenario == Scenario::Horde {
        let cleared = m.horde_waves.last().map_or(0, |w| w.wave);
        println!(
            "Horde: {} waves cleared; {} at {}",
            cleared,
            if m.winner.is_some() { "Base fell" } else { "still defending" },
            clock(m.ticks)
        );
    } else {
        match m.winner {
            Some((team, _)) => println!("{team:?} won after {}", clock(m.ticks)),
            None => println!("no winner after {} minutes", cfg.minutes),
        }
    }
    println!(
        "champion deaths {}  structures destroyed {}  simulated in {:.1} s",
        m.champion_kills, m.structures_destroyed, secs
    );
    if m.monsters_killed != [0, 0] {
        println!("jungle monsters killed: blue {}, red {}", m.monsters_killed[0], m.monsters_killed[1]);
    }
    if !m.horde_waves.is_empty() {
        println!("Horde clears (wallet + inventory value, per champion):");
        for w in &m.horde_waves {
            let gold: Vec<u32> = w.build_gold.iter().map(|g| g.round() as u32).collect();
            println!(
                "  wave {:>2}  {:>5}  levels {:?}  gold {:?}  Base {:.0}",
                w.wave,
                clock(w.tick.0),
                w.levels,
                gold,
                w.base_health
            );
        }
        println!(
            "campaign {}",
            if m.horde_waves.iter().any(|w| w.wave >= mftr_sim::horde::GOAL) {
                "won; continued into endless"
            } else {
                "not cleared"
            }
        );
    }
    for (t, team, kind, tier) in &m.falls {
        println!("  {:>5}  {team:?} {kind:?} (tier {tier})", clock(t.0));
    }
    println!(
        "  champion    team  tier       K/D/A     CS (near)  CS/min  worth@10  worth@15   dead    orders/min  time by mode (idle retreat recall fight farm jungle travel siege ward)"
    );
    let minutes = (m.ticks as f32 / 30.0 / 60.0).max(0.1);
    for c in &m.champions {
        println!(
            "  {:<10}  {:<4}  {:<9}  {:>2}/{:>2}/{:>2}  {:>4} ({:>4})  {:>6.1}  {:>8.0}  {:>8.0}  {:>4.0} s  {:>8.0}",
            format!("{:?}", c.champion),
            format!("{:?}", c.team),
            c.tier.map_or("human".into(), |t| t.name().to_string()),
            c.kills,
            c.deaths,
            c.assists,
            c.cs,
            c.minions_near,
            c.cs as f32 / minutes,
            c.worth[0],
            c.worth[1],
            c.dead_secs,
            c.stats.map_or(0.0, |s| s.orders as f32 / minutes),
        );
        if let Some(s) = c.stats {
            let total: u32 = s.mode_ticks.iter().sum::<u32>().max(1);
            let pct: Vec<String> = s.mode_ticks.iter().map(|t| format!("{:>2}", t * 100 / total)).collect();
            println!("{:>110}{}", "", pct.join(" "));
        }
    }
    let c = &m.cost;
    let n = c.ticks.max(1) as f64;
    println!(
        "bots: {:.1} µs a tick on average (worst {:.0} µs), simulation {:.1} µs",
        c.bots_us / n,
        c.bots_worst_us,
        c.sim_us / n
    );
}

pub fn run(cfg: &BotmatchConfig) -> bool {
    let mut wins = [0u32; 2];
    let mut undecided = 0;
    let mut lengths = Vec::new();
    let mut cost = (0.0f64, 0.0f64, 0.0f64, 0u64);
    let mut ok = true;
    // Per side: kills, deaths, cs, worth@15, matches.
    let mut side = [[0f64; 5]; 2];
    for seed in cfg.seed..cfg.seed + cfg.seeds {
        let server = ServerConfig { seed, bots: 10, scenario: cfg.scenario, ..Default::default() };
        let started = std::time::Instant::now();
        let opts = BotMatchOptions {
            team: cfg.team.clone(),
            tiers: Some(cfg.tiers),
            trace: cfg.trace.map(mftr_sim::PlayerId),
        };
        let m = mftr_server::run_bot_match_with(server, (cfg.minutes * 60.0 * 30.0) as u32, &opts);
        let secs = started.elapsed().as_secs_f64();
        if cfg.seeds == 1 {
            detail(cfg, &m, secs);
            if let Some(path) = &cfg.replay {
                std::fs::write(path, m.replay.to_text()).expect("write replay");
                println!("replay written to {path}");
            }
        }
        let check = m.replay.verify();
        if let Some((t, a, b)) = check.mismatch {
            println!("seed {seed}: replay MISMATCH at tick {}: {a:016x} vs {b:016x}", t.0);
            ok = false;
        } else if cfg.seeds == 1 {
            println!("replay: {} ticks, {} hashes checked, all match", check.ticks, check.hashes_checked);
        }
        match m.winner {
            Some((Team::Blue, _)) => wins[0] += 1,
            Some(_) => wins[1] += 1,
            None => undecided += 1,
        }
        lengths.push(m.ticks);
        cost.0 += m.cost.bots_us;
        cost.1 = cost.1.max(m.cost.bots_worst_us);
        cost.2 += m.cost.sim_us;
        cost.3 += m.cost.ticks as u64;
        for c in &m.champions {
            let s = &mut side[(c.team == Team::Red) as usize];
            s[0] += c.kills as f64;
            s[1] += c.deaths as f64;
            s[2] += c.cs as f64 / (m.ticks as f64 / 1800.0).max(0.1);
            s[3] += c.worth[1] as f64;
            s[4] += 1.0;
        }
        if cfg.seeds > 1 {
            let towers = |team: Team| m.falls.iter().filter(|f| f.1 == team).count();
            println!(
                "seed {seed:>3}: {:<10} {:>6}  kills {:>3}  structures lost blue {:>2} red {:>2}  ({:.1} s)",
                m.winner.map_or("undecided".to_string(), |(t, _)| format!("{t:?} won")),
                clock(m.ticks),
                m.champion_kills,
                towers(Team::Blue),
                towers(Team::Red),
                secs
            );
        }
    }
    if cfg.seeds > 1 {
        let n = cfg.seeds as f64;
        println!(
            "\nblue ({}) won {}, red ({}) won {}, undecided {}  —  average length {}",
            cfg.tiers[0].name(),
            wins[0],
            cfg.tiers[1].name(),
            wins[1],
            undecided,
            clock((lengths.iter().map(|t| *t as f64).sum::<f64>() / n) as u32)
        );
        for (i, name) in ["blue", "red"].into_iter().enumerate() {
            let s = side[i];
            let k = s[4].max(1.0);
            println!(
                "  {name:<4} per champion: kills {:.1}  deaths {:.1}  CS/min {:.1}  worth@15 {:.0}",
                s[0] / k,
                s[1] / k,
                s[2] / k,
                s[3] / k
            );
        }
        let t = cost.3.max(1) as f64;
        println!("  bots {:.1} µs a tick (worst {:.0} µs), simulation {:.1} µs", cost.0 / t, cost.1, cost.2 / t);
    }
    ok
}
