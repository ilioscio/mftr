//! `mftr-server`: dedicated UDP match server (M0 prototype, unencrypted).
//!
//! Usage: mftr-server [--bind 0.0.0.0:7777] [--seed N] [--max-players N] [--bots N] [--replay FILE]
//!                    [--scenario duel|aram|minions|dodge|empty]
//!
//! `--bots N` adds N server bots at start (they count toward the player limit). `--replay FILE`
//! records the session and rewrites FILE every minute and whenever a match ends; check it with
//! `mftr-tools replay FILE`.

use mftr_server::{ClientKey, Scenario, ServerConfig, ServerCore};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

const USAGE: &str = "mftr-server [--bind ADDR] [--seed N] [--max-players N] [--bots N] [--replay FILE] [--scenario duel|aram|minions|dodge|empty]";

fn main() -> std::io::Result<()> {
    let mut bind = "0.0.0.0:7777".to_string();
    let mut replay_path: Option<String> = None;
    // The M1 Duel Sandbox is the default playground.
    let mut cfg = ServerConfig { scenario: Scenario::Duel, ..Default::default() };
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bind" => bind = args.next().expect("--bind needs an address"),
            "--seed" => cfg.seed = args.next().and_then(|v| v.parse().ok()).expect("--seed needs a number"),
            "--max-players" => {
                cfg.max_players = args.next().and_then(|v| v.parse().ok()).expect("--max-players needs a number")
            }
            "--bots" => cfg.bots = args.next().and_then(|v| v.parse().ok()).expect("--bots needs a number"),
            "--replay" => replay_path = Some(args.next().expect("--replay needs a file")),
            "--scenario" => {
                let name = args.next().expect("--scenario needs a name");
                cfg.scenario = Scenario::by_name(&name).unwrap_or_else(|| panic!("unknown scenario {name}\n{USAGE}"));
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => panic!("unknown argument {other}"),
        }
    }

    let socket = UdpSocket::bind(&bind)?;
    socket.set_nonblocking(true)?;
    println!("mftr-server listening on {} (protocol {})", socket.local_addr()?, mftr_net::PROTOCOL_VERSION);

    let clock = Instant::now();
    let now = || clock.elapsed().as_secs_f64();
    let mut core = ServerCore::new(cfg, now());
    let mut keys: HashMap<SocketAddr, ClientKey> = HashMap::new();
    let mut addrs: HashMap<ClientKey, SocketAddr> = HashMap::new();
    let mut buf = [0u8; 2048];
    let mut last_report = now();
    let mut tick_cost = 0.0f64;
    let mut tick_cost_max = 0.0f64;
    let mut last_save = now();
    let mut had_winner = false;

    loop {
        loop {
            match socket.recv_from(&mut buf) {
                Ok((n, from)) => {
                    let next_key = keys.len() as ClientKey + 1;
                    let key = *keys.entry(from).or_insert(next_key);
                    addrs.insert(key, from);
                    for (to, bytes) in core.handle_packet(key, &buf[..n], now()) {
                        if let Some(addr) = addrs.get(&to) {
                            let _ = socket.send_to(&bytes, addr);
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue, // Windows ICMP noise
                Err(e) => return Err(e),
            }
        }

        let t = now();
        if t >= core.next_tick_due() {
            let started = Instant::now();
            for (to, bytes) in core.step(t) {
                if let Some(addr) = addrs.get(&to) {
                    let _ = socket.send_to(&bytes, addr);
                }
            }
            let cost = started.elapsed().as_secs_f64();
            tick_cost += cost;
            tick_cost_max = tick_cost_max.max(cost);
            let winner = core.world().game().winner.is_some();
            if let Some(path) = &replay_path
                && ((winner && !had_winner) || t - last_save >= 60.0)
            {
                if let Err(e) = std::fs::write(path, core.game().replay().to_text()) {
                    eprintln!("could not write replay {path}: {e}");
                }
                last_save = t;
            }
            had_winner = winner;
        }

        if t - last_report >= 5.0 {
            let s = &core.stats;
            println!(
                "tick {:>6}  players {}  cmds {} (late {}, dropped {})  in {:.1} KB/s  out {:.1} KB/s  tick avg {:.3} ms max {:.3} ms",
                core.world().tick().0,
                core.player_count(),
                s.commands,
                s.commands_late,
                s.commands_dropped,
                s.bytes_in as f64 / 1024.0 / (t - last_report),
                s.bytes_out as f64 / 1024.0 / (t - last_report),
                tick_cost * 1e3 / (5.0 * 30.0),
                tick_cost_max * 1e3,
            );
            core.stats.bytes_in = 0;
            core.stats.bytes_out = 0;
            tick_cost = 0.0;
            tick_cost_max = 0.0;
            last_report = t;
        }

        let wait = (core.next_tick_due() - now()).clamp(0.0, 0.001);
        std::thread::sleep(Duration::from_secs_f64(wait));
    }
}
