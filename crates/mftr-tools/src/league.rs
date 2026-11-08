//! The random league (CR7, 20 §9.3, D71): does the balance budget make champions fair?
//!
//! It generates random legal champions the way the creator would (random moves for the attack
//! and Q W E R, random power, cooldowns and stats, then Auto-fit to the budget), plays them
//! against each other in ARAM bot matches with random lineups, and reports each champion's
//! price and win rate, and how each part of the price relates to winning. A part that wins more
//! the more points go into it is priced too cheap; one that wins less, too dear. The target
//! (20 §9.3): every champion within 45–55% at equal bot tier, given enough games.

use mftr_forge::moves;
use mftr_forge::recipe::{Dice, Recipe};
use mftr_server::{BotMatchOptions, Scenario, ServerConfig};
use mftr_sim::balance;
use mftr_sim::{ChampionId, Team};

pub struct LeagueConfig {
    pub champions: usize,
    pub matches: usize,
    pub minutes: f64,
    pub seed: u64,
    /// Write every match (its lineups and winner) and every champion's price parts here, as CSV,
    /// for a refit by regression (`tools/league_fit.py`).
    pub log: Option<String>,
    /// Seed the matches (lineups and match seeds) apart from the champions: the same roster
    /// replayed in other matches, to see whether a champion's win rate repeats.
    pub match_seed: Option<u64>,
    /// Calibration rounds before the measured one (20 §9.3, `calibrate`), of this many matches.
    pub calibrate: usize,
    pub calibrate_matches: usize,
}

/// One random champion, registered.
fn random_champion(i: usize, d: &mut Dice) -> ChampionId {
    // A kit that can't be brought to the budget inside the limits (scaling it would break one)
    // is drawn again: every league champion spends the same budget.
    for _ in 0..60 {
        let id = draw_champion(i, d);
        let p = balance::price(id.def()).total();
        if (balance::BUDGET - 2.0..=balance::BUDGET + 0.5).contains(&p)
            && mftr_sim::limits::problems(id.def()).is_empty()
        {
            return id;
        }
    }
    draw_champion(i, d)
}

fn draw_champion(i: usize, d: &mut Dice) -> ChampionId {
    let lib = moves::library();
    let mut taken: Vec<String> = Vec::new();
    let mut pick = |slot: &str, d: &mut Dice| {
        // Each ability once (a kit's abilities have their own names).
        let fits: Vec<&String> =
            lib.iter().map(|(id, _)| id).filter(|id| moves::fits(id, slot) && !taken.contains(id)).collect();
        let k = (((d.signed() + 1.0) * 0.5) * fits.len() as f32) as usize;
        let id = fits[k.min(fits.len() - 1)].clone();
        taken.push(id.clone());
        id
    };
    let mut r = Recipe { name: format!("League {i:02}"), ..Recipe::default() };
    for slot in moves::SLOTS {
        let id = pick(slot, d);
        r.set_move(slot, &id);
    }
    // A passive from the catalog, or none (one chance in ten).
    let all = mftr_forge::passives::PASSIVES;
    let k = (((d.signed() + 1.0) * 0.5) * (all.len() + 1) as f32) as usize;
    if let Some((id, ..)) = all.get(k) {
        r.set_passive(id);
    }
    // A resource, or costless (one chance in five).
    let res = mftr_forge::passives::RESOURCES;
    let k = (((d.signed() + 1.0) * 0.5) * (res.len() + 1) as f32) as usize;
    if let Some((id, ..)) = res.get(k) {
        r.set_resource(id);
    }
    for slot in moves::SLOTS {
        r.tune(slot, 1.0 + d.signed() * 0.35, 1.0 + d.signed() * 0.3);
    }
    r.tune_stats([1.0 + d.signed() * 0.18, 1.0 + d.signed() * 0.18, 1.0 + d.signed() * 0.18, 1.0 + d.signed() * 0.05]);
    // Everyone spends the same budget: what wins is then what the price misses.
    r.kit = balance::fit_to(&r.kit, balance::BUDGET);
    let def = mftr_sim::champion::ChampionDef { name: Box::leak(r.name.clone().into_boxed_str()), ..r.kit };
    ChampionId::register(def).expect("room in the registry")
}

/// Pearson correlation.
fn correlation(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len() as f64;
    if n < 3.0 {
        return 0.0;
    }
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in xs.iter().zip(ys) {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx) * (x - mx);
        syy += (y - my) * (y - my);
    }
    if sxx <= 0.0 || syy <= 0.0 { 0.0 } else { sxy / (sxx * syy).sqrt() }
}

/// Play `matches` ARAM bot matches of ten of `roster` at random (lineups from `d`, match seeds
/// from `base`): each champion's (games, wins) over decided matches, and the log lines.
fn play(
    roster: &[ChampionId],
    matches: usize,
    d: &mut Dice,
    base: u64,
    ticks: u32,
    log: &mut String,
) -> Vec<(u32, u32)> {
    let mut tally: Vec<(u32, u32)> = vec![(0, 0); roster.len()];
    for m in 0..matches {
        // Ten different champions, at random.
        let mut pool: Vec<usize> = (0..roster.len()).collect();
        for i in (1..pool.len()).rev() {
            let j = (((d.signed() + 1.0) * 0.5) * (i + 1) as f32) as usize;
            pool.swap(i, j.min(i));
        }
        let lineup: Vec<ChampionId> = pool[..10].iter().map(|&i| roster[i]).collect();
        let server = ServerConfig { seed: base + m as u64, bots: 10, scenario: Scenario::Aram, ..Default::default() };
        let result =
            mftr_server::run_bot_match_with(server, ticks, &BotMatchOptions { team: lineup, ..Default::default() });
        let winner = result.winner.map(|(t, _)| t);
        if let Some(w) = winner {
            let side = |t: Team| -> String {
                result
                    .champions
                    .iter()
                    .filter(|c| c.team == t)
                    .filter_map(|c| roster.iter().position(|r| *r == c.champion))
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            *log += &format!(
                "{m},{},{},{}\n",
                if w == Team::Blue { "blue" } else { "red" },
                side(Team::Blue),
                side(Team::Red)
            );
        }
        // Only decided matches count: a draw says nothing about who's stronger.
        for c in result.champions.iter().filter(|_| winner.is_some()) {
            if let Some(i) = roster.iter().position(|r| *r == c.champion) {
                tally[i].0 += 1;
                if winner == Some(c.team) {
                    tally[i].1 += 1;
                }
            }
        }
        println!(
            "match {:>3}: {} after {:.1} min",
            m + 1,
            match winner {
                Some(Team::Blue) => "blue won",
                Some(Team::Red) => "red won",
                _ => "undecided",
            },
            result.ticks as f64 / mftr_sim::TICK_HZ as f64 / 60.0
        );
    }
    tally
}

/// Calibration by simulation (20 §9.3): a champion's strength lives in how its moves combine,
/// which no sum of parts prices (08 CR7: a replayed roster repeats its win rates, r 0.85, yet no
/// per-move model predicts another league). So each champion's power is tuned to its measured
/// win rate: scaled by `1 − 0.9 × (rate − ½)`, at most 25% down or 30% up a round, inside the
/// limits every kit keeps.
fn calibrate(def: &mftr_sim::champion::ChampionDef, games: u32, wins: u32) -> mftr_sim::champion::ChampionDef {
    if games < 8 {
        return *def;
    }
    let rate = wins as f32 / games as f32;
    let mut k = (1.0 - 0.9 * (rate - 0.5)).clamp(0.75, 1.3);
    for _ in 0..8 {
        let d = balance::rescale(def, k);
        if mftr_sim::limits::problems(&d).is_empty() {
            return d;
        }
        k = 1.0 + (k - 1.0) * 0.5;
    }
    *def
}

/// Play the league and print its report. Returns each champion's (price, games, wins).
pub fn run(cfg: &LeagueConfig) -> Vec<(ChampionId, f32, u32, u32)> {
    let mut d = Dice::new(cfg.seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    let mut roster: Vec<ChampionId> = (0..cfg.champions.max(10)).map(|i| random_champion(i, &mut d)).collect();
    if let Some(s) = cfg.match_seed {
        d = Dice::new(s.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1);
    }
    let match_base = cfg.match_seed.unwrap_or(cfg.seed);
    let ticks = (cfg.minutes * 60.0 * mftr_sim::TICK_HZ as f64) as u32;
    let mut log = String::from("match,winner,blue,red\n");
    for round in 0..cfg.calibrate {
        let mut scratch = String::new();
        let t = play(
            &roster,
            cfg.calibrate_matches,
            &mut d,
            match_base + 100_000 * (round as u64 + 1),
            ticks,
            &mut scratch,
        );
        let rms =
            (t.iter().filter(|g| g.0 > 0).map(|&(g, w)| (100.0 * w as f64 / g as f64 - 50.0).powi(2)).sum::<f64>()
                / t.iter().filter(|g| g.0 > 0).count().max(1) as f64)
                .sqrt();
        println!("calibration round {}: rms {rms:.1} points from 50%", round + 1);
        for (i, c) in roster.iter_mut().enumerate() {
            let def = calibrate(c.def(), t[i].0, t[i].1);
            println!(
                "  {:<10} {:>3} games {:>3}%  {:>6.1} -> {:>6.1} points",
                c.def().name,
                t[i].0,
                (100 * t[i].1).checked_div(t[i].0).unwrap_or(0),
                balance::price(c.def()).total(),
                balance::price(&def).total()
            );
            *c = ChampionId::register(def).expect("room in the registry");
        }
    }
    let tally = play(&roster, cfg.matches, &mut d, match_base, ticks, &mut log);
    println!("\n{:<10} {:>6} {:>6} {:>6}   moves", "champion", "price", "games", "won%");
    let mut out = Vec::new();
    for (i, c) in roster.iter().enumerate() {
        let (g, w) = tally[i];
        let price = balance::price(c.def()).total();
        let pct = if g > 0 { 100.0 * w as f64 / g as f64 } else { f64::NAN };
        let abilities: Vec<&str> = c.def().abilities.iter().map(|a| a.name).collect();
        let passive = mftr_forge::passives::of(c.def().passive).map_or("no passive", |e| e.1);
        println!("{:<10} {:>6.1} {:>6} {:>5.0}%   {} · {passive}", c.def().name, price, g, pct, abilities.join(" / "));
        out.push((*c, price, g, w));
    }
    // How each part of the price relates to winning, over champions that played.
    let played: Vec<&(ChampionId, f32, u32, u32)> = out.iter().filter(|r| r.2 > 0).collect();
    let rates: Vec<f64> = played.iter().map(|r| r.3 as f64 / r.2 as f64).collect();
    println!("\nwin rate against each part of the price (r > 0: too cheap; r < 0: too dear):");
    for (k, name) in ["durability", "attack", "speed", "damage", "control", "mobility", "protection"].iter().enumerate()
    {
        let xs: Vec<f64> = played.iter().map(|r| balance::price(r.0.def()).parts()[k].1 as f64).collect();
        println!("  {name:<11} r = {:+.2}", correlation(&xs, &rates));
    }
    if let Some(path) = &cfg.log {
        let mut parts =
            String::from("champion,durability,attack,speed,damage,control,mobility,protection,moves,passive\n");
        for (i, c) in roster.iter().enumerate() {
            let p = balance::price(c.def()).parts();
            let moves: Vec<&str> = c.def().abilities.iter().map(|a| a.name).collect();
            let passive = mftr_forge::passives::of(c.def().passive).map_or("no passive", |e| e.1);
            let nums: Vec<String> = p.iter().take(7).map(|(_, v)| format!("{v:.3}")).collect();
            parts += &format!("{i},{},{},{passive}\n", nums.join(","), moves.join("|"));
        }
        let _ = std::fs::write(format!("{path}.matches.csv"), &log);
        let _ = std::fs::write(format!("{path}.champions.csv"), parts);
    }
    let spread: Vec<f64> = rates.iter().map(|r| r * 100.0).collect();
    if !spread.is_empty() {
        let inside = spread.iter().filter(|r| (45.0..=55.0).contains(*r)).count();
        println!("{inside} of {} champions within 45–55% (the target, 20 §9.3)", spread.len());
    }
    out
}
