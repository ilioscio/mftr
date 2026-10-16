//! mftr-tools: developer CLI.
//!
//!   mftr-tools netlab [--profile NAME|all] [--clients N] [--seconds S] [--warmup S] [--seed N] [--fps F]
//!                     [--scenario empty|minions|dodge|duel] [--no-proxies] [--reaction S]
//!   mftr-tools loadtest [--mode aram|mayhem|hyper|classic] [--matches N,N...] [--humans N] [--workers N,N...] [--seconds S]
//!   mftr-tools bot --server HOST:PORT [--queue MODE[-coop]] [--profile NAME] [--seconds S] [--seed N] [--duel] [--champion NAME]
//!
//!   mftr-tools blind-report FILE.tsv...
//!   mftr-tools pack validate FILE.glb|DIR...
//!   mftr-tools sfx build [sounds.ron|DIR...]   (default: art)
//!   mftr-tools map svg open|arena|bridge|crossroads|FILE.mftrmap   (an SVG of the layout, on stdout)
//!
//! Profiles: perfect, lan, good, typical, mid, rough, awful.

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
                    // `--augments 24,25,26`: every champion holds these (stress runs).
                    augments: get("--augments").map(|v| {
                        let mut held = [0u8; mftr_sim::augments::SLOTS];
                        for (slot, id) in held.iter_mut().zip(v.split(',')) {
                            *slot = id.trim().parse().expect("augment id");
                        }
                        held
                    }),
                    map: None,
                };
                let r = netlab::run(&cfg);
                println!(
                    "{}   ticks {} hash {:#018x}  fog: {} withheld, {} leaked  tick {:.3} ms (max {:.3})",
                    r.summary.row(),
                    r.server_ticks,
                    r.server_hash,
                    r.fog_hidden,
                    r.fog_violations,
                    r.tick_ms_mean,
                    r.tick_ms_max
                );
                if matches!(
                    cfg.scenario,
                    Scenario::DodgeRig
                        | Scenario::Duel
                        | Scenario::Aram
                        | Scenario::Mayhem
                        | Scenario::Hyper
                        | Scenario::Classic
                ) {
                    println!("{}", r.summary.dodge_row());
                }
                if matches!(
                    cfg.scenario,
                    Scenario::Duel | Scenario::Aram | Scenario::Mayhem | Scenario::Hyper | Scenario::Classic
                ) {
                    println!("{}", r.summary.duel_row());
                }
            }
        }
        Some("botmatch") => {
            // 17 §11: server bots play whole matches; the replays must re-simulate exactly.
            let scenario = if args.iter().any(|a| a == "--horde") {
                Scenario::Horde
            } else if args.iter().any(|a| a == "--classic") {
                Scenario::Classic
            } else if args.iter().any(|a| a == "--hyper") {
                Scenario::Hyper
            } else if args.iter().any(|a| a == "--mayhem") {
                Scenario::Mayhem
            } else {
                Scenario::Aram
            };
            let tier = |flag: &str| {
                get(flag).map(|t| mftr_server::Tier::by_name(&t).unwrap_or_else(|| panic!("unknown tier {t}")))
            };
            let both = tier("--tier").unwrap_or(mftr_server::Tier::Veteran);
            let team: Vec<_> = get("--team")
                .map(|names| {
                    names
                        .split(',')
                        .map(|name| {
                            mftr_sim::ChampionId::by_name(name.trim())
                                .unwrap_or_else(|| panic!("unknown champion {name}"))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let cfg = mftr_tools::botmatch::BotmatchConfig {
                scenario,
                seed: num("--seed", 1.0) as u64,
                seeds: num("--seeds", 1.0) as u64,
                minutes: num("--minutes", 40.0),
                team,
                tiers: [tier("--blue").unwrap_or(both), tier("--red").unwrap_or(both)],
                replay: get("--replay"),
                trace: get("--trace").map(|p| p.parse().expect("--trace PLAYER")),
            };
            if !mftr_tools::botmatch::run(&cfg) {
                std::process::exit(1);
            }
        }
        Some("loadtest") => {
            // 13 §8: many matches in one hub, in process; what a server spends per match.
            let scenario = match get("--mode").as_deref() {
                None | Some("aram") => Scenario::Aram,
                Some(name) => Scenario::by_name(name).unwrap_or_else(|| panic!("unknown mode {name}")),
            };
            let list = |name: &str, default: Vec<usize>| -> Vec<usize> {
                get(name).map_or(default, |w| w.split(',').map(|n| n.parse().expect("N[,N...]")).collect())
            };
            println!("{}", mftr_tools::loadtest::LoadResult::header());
            for m in list("--matches", vec![1, 4, 16]) {
                for w in list("--workers", vec![1]) {
                    let r = mftr_tools::loadtest::run(&mftr_tools::loadtest::LoadConfig {
                        matches: m,
                        humans: num("--humans", 10.0) as usize,
                        scenario,
                        seconds: num("--seconds", 30.0),
                        workers: w,
                        seed: num("--seed", 1.0) as u64,
                    });
                    println!("{}", r.row(w));
                }
            }
        }
        Some("size-report") => {
            // M2 slice 6: sizes of what we ship against their budgets (exit 1 when over).
            let package = get("--client-package").map(std::path::PathBuf::from);
            let items = mftr_tools::size::report(std::path::Path::new("."), package.as_deref());
            print!("{}", mftr_tools::size::table(&items));
            if items.iter().any(|i| i.over()) {
                std::process::exit(1);
            }
        }
        Some("replay") => {
            // Re-simulate a replay file and check its hashes.
            let path = args.get(1).expect("replay FILE");
            let text = std::fs::read_to_string(path).expect("read replay");
            let replay = mftr_server::Replay::from_text(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
            let check = replay.verify();
            println!(
                "{} ticks, {} hashes checked, final hash {:016x}",
                check.ticks, check.hashes_checked, check.final_hash
            );
            if let Some((t, a, b)) = check.mismatch {
                println!("MISMATCH at tick {}: recorded {a:016x}, re-simulated {b:016x}", t.0);
                std::process::exit(1);
            }
            println!("all hashes match");
        }
        Some("bot") => {
            let cfg = bot::BotConfig {
                server: get("--server").expect("--server HOST:PORT required"),
                profile: profiles("perfect")[0],
                seconds: num("--seconds", 30.0),
                seed: num("--seed", 1.0) as u64,
                fps: num("--fps", 144.0),
                duel: args.iter().any(|a| a == "--duel"),
                champion: get("--champion")
                    .map(|c| mftr_sim::ChampionId::by_name(&c).unwrap_or_else(|| panic!("unknown champion {c}"))),
                queue: get("--queue"),
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
        Some("blind-report") => {
            let mut records = Vec::new();
            for path in &args[1..] {
                let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
                records.extend(text.lines().filter_map(mftr_client::blind::BlindRecord::from_tsv));
            }
            print!("{}", mftr_tools::report::blind_report(&records));
        }
        Some("pack") if args.get(1).map(String::as_str) == Some("validate") => {
            // A1 (10 §8.4): every `<id>.glb` given, or found under a given directory, with its
            // `<id>.anims.ron` sidecar. Exits non-zero on any error.
            let mut files = Vec::new();
            for arg in &args[2..] {
                let path = std::path::Path::new(arg);
                if path.is_dir() {
                    collect_glbs(path, &mut files);
                } else {
                    files.push(path.to_path_buf());
                }
            }
            if files.is_empty() {
                eprintln!("usage: mftr-tools pack validate FILE.glb|DIR...");
                std::process::exit(2);
            }
            let mut errors = 0;
            for f in &files {
                let report = mftr_pack::validate_file(f);
                println!("{}", f.display());
                for line in &report.summary {
                    println!("  {line}");
                }
                for finding in &report.findings {
                    println!("  {finding}");
                }
                println!("  {} errors, {} warnings", report.errors(), report.warnings());
                errors += report.errors();
            }
            if errors > 0 {
                std::process::exit(1);
            }
        }
        Some("sfx") if args.get(1).map(String::as_str) == Some("build") => {
            // A4c (05 §7): render every `sounds.ron` recipe into its pack's `export/`.
            let mut recipes = Vec::new();
            let roots: Vec<&str> =
                if args.len() > 2 { args[2..].iter().map(String::as_str).collect() } else { vec!["art"] };
            for arg in roots {
                let path = std::path::Path::new(arg);
                if path.is_dir() {
                    mftr_tools::sfx::find_recipes(path, &mut recipes);
                } else {
                    recipes.push(path.to_path_buf());
                }
            }
            for r in &recipes {
                match mftr_tools::sfx::build(r) {
                    Ok(b) => {
                        println!("{}: {} sounds, {:.1} KB", b.binding.display(), b.sounds, b.bytes as f32 / 1000.0)
                    }
                    Err(e) => {
                        eprintln!("{}: {e}", r.display());
                        std::process::exit(1);
                    }
                }
            }
        }
        Some("map") if args.get(1).map(String::as_str) == Some("svg") => {
            // A map's layout, for design review: `mftr-tools map svg crossroads > crossroads.svg`.
            // A built-in map by name, or a map file (`*.mftrmap`), its problems on stderr.
            let name = args.get(2).map(String::as_str).unwrap_or("crossroads");
            let map = match mftr_tools::mapsvg::by_name(name) {
                Some(id) => id.build(),
                None => {
                    let text = std::fs::read_to_string(name).unwrap_or_else(|e| panic!("{name}: {e}"));
                    let spec = mftr_sim::mapspec::MapSpec::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
                    for p in mftr_sim::mapspec::problems(&spec) {
                        eprintln!("problem: {p}");
                    }
                    mftr_sim::mapspec::build(&spec, mftr_sim::map::MapId::Custom)
                }
            };
            print!("{}", mftr_tools::mapsvg::render(&map));
        }
        _ => {
            eprintln!("usage: mftr-tools netlab [--profile NAME|all] [--clients N] [--seconds S] [--seed N]");
            eprintln!(
                "       mftr-tools loadtest [--mode aram|mayhem|hyper|classic] [--matches N,N...] [--humans N] [--workers N,N...] [--seconds S]
       mftr-tools bot --server HOST:PORT [--queue MODE[-coop]] [--profile NAME] [--seconds S] [--seed N] [--duel] [--champion NAME]"
            );
            eprintln!("       mftr-tools blind-report FILE.tsv...");
            eprintln!(
                "       mftr-tools botmatch [--horde|--mayhem|--hyper|--classic] [--team NAME,...] [--seed N] [--seeds N] [--minutes N] [--tier|--blue|--red recruit|veteran|elite|nightmare|legacy]"
            );
            eprintln!("       mftr-tools pack validate FILE.glb|DIR...");
            eprintln!("       mftr-tools sfx build [sounds.ron|DIR...]");
            eprintln!("       mftr-tools map svg open|arena|bridge|crossroads");
            std::process::exit(2);
        }
    }
}

/// Every `.glb` under `dir`, recursively, in name order.
fn collect_glbs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect_glbs(&p, out);
        } else if p.extension().is_some_and(|e| e == "glb") {
            out.push(p);
        }
    }
}
