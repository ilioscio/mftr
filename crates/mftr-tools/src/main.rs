//! mftr-tools: developer CLI.
//!
//!   mftr-tools netlab [--profile NAME|all] [--clients N] [--seconds S] [--warmup S] [--seed N] [--fps F]
//!   mftr-tools bot --server HOST:PORT [--profile NAME] [--seconds S] [--seed N]
//!
//! Profiles: perfect, lan, good, typical, rough, awful.

use mftr_net::conditioner::LinkProfile;
use mftr_server::Scenario;
use mftr_tools::report::Summary;
use mftr_tools::{bot, netlab};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let get = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let num = |flag: &str, default: f64| get(flag).map(|v| v.parse::<f64>().expect("number")).unwrap_or(default);
    let profiles = |default: &str| -> Vec<LinkProfile> {
        let name = get("--profile").unwrap_or_else(|| default.to_string());
        if name == "all" {
            LinkProfile::ALL.to_vec()
        } else {
            vec![LinkProfile::by_name(&name).unwrap_or_else(|| panic!("unknown profile {name}"))]
        }
    };

    match args.first().map(String::as_str) {
        Some("netlab") => {
            println!("{}", Summary::header());
            for profile in profiles("all") {
                let cfg = netlab::LabConfig {
                    profile,
                    clients: num("--clients", 10.0) as usize,
                    seconds: num("--seconds", 60.0),
                    seed: num("--seed", 1.0) as u64,
                    fps: num("--fps", 144.0),
                    warmup: num("--warmup", 10.0),
                    scenario: get("--scenario")
                        .map(|s| Scenario::by_name(&s).unwrap_or_else(|| panic!("unknown scenario {s}")))
                        .unwrap_or(Scenario::Empty),
                    proxies: !args.iter().any(|a| a == "--no-proxies"),
                    reaction: num("--reaction", 0.25),
                    margin_override: get("--margin-override").map(|v| v.parse::<f64>().expect("seconds")),
                };
                let r = netlab::run(&cfg);
                println!("{}   ticks {} hash {:#018x}", r.summary.row(), r.server_ticks, r.server_hash);
                if cfg.scenario == Scenario::DodgeRig {
                    println!("{}", r.summary.dodge_row());
                }
            }
        }
        Some("bot") => {
            let cfg = bot::BotConfig {
                server: get("--server").expect("--server HOST:PORT required"),
                profile: profiles("perfect")[0],
                seconds: num("--seconds", 30.0),
                seed: num("--seed", 1.0) as u64,
                fps: num("--fps", 144.0),
            };
            match bot::run(&cfg) {
                Ok(s) => {
                    println!("{}", Summary::header());
                    println!("{}", s.row());
                }
                Err(e) => {
                    eprintln!("bot failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("usage: mftr-tools netlab [--profile NAME|all] [--clients N] [--seconds S] [--seed N]");
            eprintln!("       mftr-tools bot --server HOST:PORT [--profile NAME] [--seconds S] [--seed N]");
            std::process::exit(2);
        }
    }
}
