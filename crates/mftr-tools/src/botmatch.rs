//! `mftr-tools botmatch`: server bots play whole matches, as fast as they can, and we measure
//! them (17 §11): who wins, how each champion did, what the bots cost; every replay must
//! re-simulate exactly.

use mftr_server::{BotMatch, BotMatchOptions, Scenario, ServerConfig, Spacing, Tier};
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
    pub trace_falls: bool,
    /// Print what the items did (18's balance measurements), over every game.
    pub items: bool,
    /// Each bot's legendaries drawn at random from its class's (the build-shuffle experiment).
    pub shuffle_builds: bool,
}

/// What one item did over a set of games: champions who built it, the second they first held
/// it (summed), seconds held, actives used, damage its effects dealt to champions, and of the
/// champions holding it at the end, how many were on the winning side.
#[derive(Default, Clone, Copy)]
struct ItemTally {
    built: u32,
    first: f64,
    held: f64,
    uses: u32,
    damage: f64,
    end: u32,
    end_won: u32,
    /// The end holders' games: damage to champions, minutes, kills, deaths.
    holder_damage: f64,
    holder_minutes: f64,
    holder_kills: u32,
    holder_deaths: u32,
}

#[derive(Default)]
struct ItemReport {
    items: std::collections::BTreeMap<u8, ItemTally>,
    attacks: u64,
    crits: u64,
    quest: Vec<(Option<u32>, Option<u32>)>,
    grown: Vec<Option<u32>>,
}

impl ItemReport {
    fn add(&mut self, m: &BotMatch) {
        for c in &m.champions {
            let log = &c.item_log;
            let won = m.winner.is_some_and(|(t, _)| t == c.team);
            for &(item, first, held) in &log.held {
                let t = self.items.entry(item).or_default();
                t.built += 1;
                t.first += first as f64;
                t.held += held as f64;
            }
            for &(item, n) in &log.uses {
                self.items.entry(item).or_default().uses += n;
            }
            for &(item, d) in &log.damage {
                self.items.entry(item).or_default().damage += d as f64;
            }
            let minutes = m.ticks as f64 / 1800.0;
            for &item in c.items.iter().filter(|i| **i != 0) {
                let t = self.items.entry(item).or_default();
                t.end += 1;
                t.end_won += won as u32;
                t.holder_damage += c.damage_dealt as f64;
                t.holder_minutes += minutes;
                t.holder_kills += c.kills as u32;
                t.holder_deaths += c.deaths as u32;
            }
            self.attacks += log.attacks as u64;
            self.crits += log.crits as u64;
            if c.items.iter().any(|i| {
                *i == mftr_sim::items::PILGRIMS_ASTROLABE
                    || mftr_sim::items::item(*i)
                        .is_some_and(|it| it.recipe.contains(&mftr_sim::items::PILGRIMS_ASTROLABE))
            }) || log.quest_stage.is_some()
                || c.items.contains(&mftr_sim::items::PILGRIMS_LEDGER)
            {
                self.quest.push((log.quest_stage, log.quest_done));
            }
            if c.items
                .iter()
                .any(|i| mftr_sim::items::item(*i).is_some_and(|it| it.tier == mftr_sim::items::Tier::Jungle))
            {
                self.grown.push(log.grown);
            }
        }
    }

    fn print(&self, games: u64) {
        use mftr_sim::items::{self, Tier};
        let mm = |s: f64| format!("{}:{:02}", (s / 60.0) as u32, (s % 60.0) as u32);
        println!("
Items over {games} games (built: champions who held it; uses and damage to champions per minute held; won: share of end holders on the winning side)");
        println!(
            "  {:<22} {:<10} {:>5} {:>7} {:>9} {:>9} {:>6} {:>8}  {:>12} {:>9}",
            "item", "class", "built", "first", "uses/min", "dmg/min", "won", "of", "holder d/min", "K/D"
        );
        let mut rows: Vec<&items::Item> = Vec::new();
        for class in ["Fighter", "Assassin", "Marksman", "Mage", "Tank", "Support"] {
            rows.extend(items::CATALOG.iter().filter(|i| items::class_of(i.id) == Some(class)));
        }
        for tier in [Tier::Boots, Tier::Epic, Tier::Starter, Tier::Support, Tier::Jungle] {
            rows.extend(items::CATALOG.iter().filter(|i| i.tier == tier));
        }
        {
            for it in rows {
                let tier = it.tier;
                let Some(t) = self.items.get(&it.id) else {
                    if tier == Tier::Legendary {
                        println!("  {:<22} {:<10} {:>5}", it.name, tier.name(), 0);
                    }
                    continue;
                };
                let mins = (t.held / 60.0).max(1e-6);
                println!(
                    "  {:<22} {:<10} {:>5} {:>7} {:>9} {:>9} {:>6} {:>8}  {:>12} {:>9}",
                    it.name,
                    items::class_of(it.id).unwrap_or(tier.name()),
                    t.built,
                    mm(t.first / t.built.max(1) as f64),
                    if t.uses > 0 { format!("{:.2}", t.uses as f64 / mins) } else { "".into() },
                    if t.damage > 0.0 { format!("{:.0}", t.damage / mins) } else { "".into() },
                    if t.end > 0 { format!("{:.0}%", 100.0 * t.end_won as f64 / t.end as f64) } else { "".into() },
                    format!("{}/{}", t.end_won, t.end),
                    if t.end > 0 { format!("{:.0}", t.holder_damage / t.holder_minutes.max(1e-6)) } else { "".into() },
                    if t.end > 0 {
                        format!(
                            "{:.1}/{:.1}",
                            t.holder_kills as f64 / t.end as f64,
                            t.holder_deaths as f64 / t.end as f64
                        )
                    } else {
                        "".into()
                    },
                );
            }
        }
        println!(
            "  crits: {} of {} attacks on champions ({:.1}%)",
            self.crits,
            self.attacks,
            100.0 * self.crits as f64 / self.attacks.max(1) as f64
        );
        let avg = |v: Vec<u32>| {
            if v.is_empty() {
                "never".to_string()
            } else {
                mm(v.iter().map(|x| *x as f64).sum::<f64>() / v.len() as f64)
            }
        };
        let stage: Vec<u32> = self.quest.iter().filter_map(|q| q.0).collect();
        let done: Vec<u32> = self.quest.iter().filter_map(|q| q.1).collect();
        println!(
            "  quest: {} supports; Astrolabe {} of them, at {} on average; done {}, at {}",
            self.quest.len(),
            stage.len(),
            avg(stage.clone()),
            done.len(),
            avg(done.clone())
        );
        let grown: Vec<u32> = self.grown.iter().filter_map(|g| *g).collect();
        println!("  hatchlings: {}; grown {}, at {} on average", self.grown.len(), grown.len(), avg(grown.clone()));
    }
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
        "  champion    team  tier       K/D/A     CS (near)  CS/min  worth@10  worth@15   dead   dealt  taken (minions turrets)  orders/min  time by mode (idle retreat recall fight farm jungle travel siege ward)"
    );
    let minutes = (m.ticks as f32 / 30.0 / 60.0).max(0.1);
    for c in &m.champions {
        println!(
            "  {:<10}  {:<4}  {:<9}  {:>2}/{:>2}/{:>2}  {:>4} ({:>4})  {:>6.1}  {:>8.0}  {:>8.0}  {:>4.0} s  {:>6.0} {:>6.0} ({:>6.0} {:>6.0})  {:>8.0}",
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
            c.damage_dealt,
            c.damage_taken,
            c.taken_minions,
            c.taken_structures,
            c.stats.map_or(0.0, |s| s.orders as f32 / minutes),
        );
        if let Some(s) = c.stats {
            let total: u32 = s.mode_ticks.iter().sum::<u32>().max(1);
            let pct: Vec<String> = s.mode_ticks.iter().map(|t| format!("{:>2}", t * 100 / total)).collect();
            println!("{:>110}{}", "", pct.join(" "));
        }
        let names: Vec<&str> = c.items.iter().filter_map(|i| mftr_sim::items::item(*i)).map(|i| i.name).collect();
        println!("{:>14}items: {}", "", names.join(", "));
    }
    let calls: Vec<String> = mftr_sim::ping::PingKind::ALL
        .iter()
        .zip(m.pings)
        .filter(|(_, n)| *n > 0)
        .map(|(k, n)| format!("{} {n}", k.text()))
        .collect();
    println!("bot pings: {}", if calls.is_empty() { "none".to_string() } else { calls.join(", ") });
    let c = &m.cost;
    let n = c.ticks.max(1) as f64;
    println!(
        "bots: {:.1} µs a tick on average (worst {:.0} µs), simulation {:.1} µs",
        c.bots_us / n,
        c.bots_worst_us,
        c.sim_us / n
    );
}

/// How the teams stood on a one-lane map (17 §13): the gap between their medians and where the
/// front (the point between them) was, as 10th/50th/90th percentiles in half Base-to-Base
/// lengths, the share of time the front was between the outer turrets, and the share each
/// team's median was behind its own outer turret. The reference footage (plat to low diamond
/// ARAM) measured gap 0.04/0.19/0.38, front -0.42/0.12/0.50, between 65%, behind 25%.
fn spacing_line(all: &[Spacing], kills: u32, ticks: u64) -> Option<String> {
    let outer = all.first()?.outer_turret;
    let samples: Vec<[f32; 4]> = all.iter().flat_map(|s| s.samples.iter().copied()).collect();
    if samples.is_empty() {
        return None;
    }
    let pct = |mut v: Vec<f32>| {
        v.sort_by(f32::total_cmp);
        let at = |q: f32| v[((v.len() as f32 * q) as usize).min(v.len() - 1)];
        format!("{:.2}/{:.2}/{:.2}", at(0.1), at(0.5), at(0.9))
    };
    let n = samples.len() as f32;
    let between = samples.iter().filter(|[b, r, ..]| ((b + r) * 0.5).abs() < outer).count() as f32 / n;
    let behind =
        samples.iter().map(|[b, r, ..]| (*b < -outer) as u32 + (*r > outer) as u32).sum::<u32>() as f32 / (2.0 * n);
    // The same gap with four or more alive on each side: teams at full strength.
    let full: Vec<f32> = samples.iter().filter(|s| s[2] >= 4.0 && s[3] >= 4.0).map(|[b, r, ..]| r - b).collect();
    Some(format!(
        "spacing: gap {} ({} with 4+ alive each)  front {}  between outer turrets {:.0}%  behind own {:.0}%  kills/min {:.1}",
        pct(samples.iter().map(|[b, r, ..]| r - b).collect()),
        if full.is_empty() { "-".to_string() } else { pct(full) },
        pct(samples.iter().map(|[b, r, ..]| (b + r) * 0.5).collect()),
        between * 100.0,
        behind * 100.0,
        kills as f64 / (ticks as f64 / 1800.0).max(0.1)
    ))
}

/// The champions for a seed: the ones asked for, else the roster shuffled by the seed (so a
/// series measures the bots, not one lineup).
fn lineup(cfg: &BotmatchConfig, seed: u64) -> Vec<ChampionId> {
    if !cfg.team.is_empty() {
        return cfg.team.clone();
    }
    let mut all = ChampionId::ALL.to_vec();
    let mut rng = mftr_sim::rng::Pcg32::new(seed ^ 0x6c69_6e65, 1);
    for i in (1..all.len()).rev() {
        all.swap(i, rng.next_u32() as usize % (i + 1));
    }
    all
}

/// Swap each pair of seats (blue i, red i): the same champions on the other side.
fn swapped(lineup: &[ChampionId]) -> Vec<ChampionId> {
    let mut out = lineup.to_vec();
    for pair in out.chunks_mut(2) {
        pair.swap(0, pair.len() - 1);
    }
    out
}

pub fn run(cfg: &BotmatchConfig) -> bool {
    // Two different tiers play each seed twice, sides swapped, so each tier plays each lineup
    // from each side once; wins are counted by tier.
    let paired = cfg.tiers[0] != cfg.tiers[1] && cfg.team.is_empty() && cfg.scenario != Scenario::Horde;
    let mut wins = [0u32; 2];
    let mut undecided = 0;
    let mut lengths = Vec::new();
    let mut cost = (0.0f64, 0.0f64, 0.0f64, 0u64);
    let mut ok = true;
    // Per tier (0: --blue's, 1: --red's): kills, deaths, cs/min, worth@15, champions, enemy
    // camps taken, structures taken.
    let mut side = [[0f64; 7]; 2];
    let mut spacings = Vec::new();
    // Per persona (17 §13): champions, kills, deaths, assists, damage dealt, seconds dead,
    // relics, health share at a relic summed, minutes played.
    let mut personas: Vec<(&'static str, [f64; 9])> = Vec::new();
    let mut kills = 0u32;
    let games = cfg.seeds * if paired { 2 } else { 1 };
    let mut report = ItemReport::default();
    for game in 0..games {
        let seed = cfg.seed + if paired { game / 2 } else { game };
        let flip = paired && game % 2 == 1;
        let lineup = if flip { swapped(&lineup(cfg, seed)) } else { lineup(cfg, seed) };
        if cfg.shuffle_builds {
            mftr_bots::shop::SHUFFLE.store(seed * 7919 + game + 1, std::sync::atomic::Ordering::Relaxed);
        }
        let tiers = if flip { [cfg.tiers[1], cfg.tiers[0]] } else { cfg.tiers };
        let server = ServerConfig { seed, bots: 10, scenario: cfg.scenario, ..Default::default() };
        let started = std::time::Instant::now();
        let team = if cfg.scenario == Scenario::Horde { cfg.team.clone() } else { lineup };
        let opts = BotMatchOptions {
            team,
            tiers: Some(tiers),
            trace: cfg.trace.map(mftr_sim::PlayerId),
            trace_falls: cfg.trace_falls,
        };
        let m = mftr_server::run_bot_match_with(server, (cfg.minutes * 60.0 * 30.0) as u32, &opts);
        let secs = started.elapsed().as_secs_f64();
        if games == 1 {
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
        } else if games == 1 {
            println!("replay: {} ticks, {} hashes checked, all match", check.ticks, check.hashes_checked);
        }
        // Which of the asked tiers won (0: --blue's, 1: --red's).
        let tier_of = |team: Team| (team == Team::Red) != flip;
        match m.winner {
            Some((team, _)) => wins[tier_of(team) as usize] += 1,
            None => undecided += 1,
        }
        lengths.push(m.ticks);
        report.add(&m);
        kills += m.champion_kills;
        spacings.extend(m.spacing.clone());
        if games == 1
            && let Some(line) = spacing_line(&spacings, kills, m.ticks as u64)
        {
            println!("{line}");
        }
        cost.0 += m.cost.bots_us;
        cost.1 = cost.1.max(m.cost.bots_worst_us);
        cost.2 += m.cost.sim_us;
        cost.3 += m.cost.ticks as u64;
        for c in &m.champions {
            if let Some(name) = c.persona {
                let i = match personas.iter().position(|(n, _)| *n == name) {
                    Some(i) => i,
                    None => {
                        personas.push((name, [0.0; 9]));
                        personas.len() - 1
                    }
                };
                let v = &mut personas[i].1;
                for (j, x) in [
                    1.0,
                    c.kills as f64,
                    c.deaths as f64,
                    c.assists as f64,
                    c.damage_dealt as f64,
                    c.dead_secs as f64,
                    c.relics as f64,
                    c.relic_share as f64,
                    m.ticks as f64 / 1800.0,
                ]
                .into_iter()
                .enumerate()
                {
                    v[j] += x;
                }
            }
            let s = &mut side[tier_of(c.team) as usize];
            s[0] += c.kills as f64;
            s[1] += c.deaths as f64;
            s[2] += c.cs as f64 / (m.ticks as f64 / 1800.0).max(0.1);
            s[3] += c.worth[1] as f64;
            s[4] += 1.0;
            s[5] += c.monsters_theirs as f64;
        }
        for f in &m.falls {
            // A structure of `f.1`'s fell: the other side took it.
            side[!tier_of(f.1) as usize][6] += 1.0;
        }
        if games > 1 {
            let towers = |team: Team| m.falls.iter().filter(|f| f.1 == team).count();
            let names: Vec<String> = lineup_names(&m);
            println!(
                "seed {seed:>3}{}: {:<22} {:>6}  kills {:>3}  structures lost blue {:>2} red {:>2}  ({:.1} s)  {}",
                if flip { "'" } else { " " },
                m.winner.map_or("undecided".to_string(), |(t, _)| format!(
                    "{t:?} ({}) won",
                    tiers[(t == Team::Red) as usize].name()
                )),
                clock(m.ticks),
                m.champion_kills,
                towers(Team::Blue),
                towers(Team::Red),
                secs,
                names.join(" ")
            );
        }
    }
    if games > 1 {
        let n = games as f64;
        println!(
            "
{} won {}, {} won {}, undecided {}  —  average length {}",
            cfg.tiers[0].name(),
            wins[0],
            cfg.tiers[1].name(),
            wins[1],
            undecided,
            clock((lengths.iter().map(|t| *t as f64).sum::<f64>() / n) as u32)
        );
        for (i, tier) in cfg.tiers.iter().enumerate() {
            let s = side[i];
            let k = s[4].max(1.0);
            println!(
                "  {:<9} per champion: kills {:.1}  deaths {:.1}  CS/min {:.1}  worth@15 {:.0}  |  a game: structures {:.1}, enemy monsters {:.1}",
                tier.name(),
                s[0] / k,
                s[1] / k,
                s[2] / k,
                s[3] / k,
                s[6] / n,
                s[5] / n
            );
        }
        if let Some(line) = spacing_line(&spacings, kills, lengths.iter().map(|t| *t as u64).sum()) {
            println!("  {line}");
        }
        personas.sort_by_key(|(n, _)| *n);
        for (name, v) in &personas {
            let n = v[0].max(1.0);
            let minutes = v[8].max(0.1);
            println!(
                "  {:<9} x{:<3} K/D/A {:.1}/{:.1}/{:.1}  damage/min {:>4.0}  dead {:>2.0}%  relics/10 min {:.1} (at {:.0}% health)",
                name,
                v[0],
                v[1] / n,
                v[2] / n,
                v[3] / n,
                v[4] / minutes,
                v[5] / (minutes * 60.0) * 100.0,
                v[6] / minutes * 10.0,
                if v[6] > 0.0 { v[7] / v[6] * 100.0 } else { 0.0 }
            );
        }
        let t = cost.3.max(1) as f64;
        println!("  bots {:.1} µs a tick (worst {:.0} µs), simulation {:.1} µs", cost.0 / t, cost.1, cost.2 / t);
    }
    if cfg.items {
        report.print(games);
    }
    ok
}

/// "blue: A B C D E / red: …" in short.
fn lineup_names(m: &BotMatch) -> Vec<String> {
    let mut blue: Vec<String> = Vec::new();
    let mut red: Vec<String> = Vec::new();
    for c in &m.champions {
        let n = format!("{:?}", c.champion);
        if c.team == Team::Blue { blue.push(n) } else { red.push(n) }
    }
    vec![format!("[{}]", blue.join(",")), "v".into(), format!("[{}]", red.join(","))]
}
