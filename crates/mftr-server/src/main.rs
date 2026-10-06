//! `mftr-server`: dedicated UDP match server, encrypted with the secure transport (D40).
//!
//! Usage: mftr-server [--bind 0.0.0.0:7777] [--key FILE] [--seed N] [--max-players N] [--bots N]
//!                    [--lobby] [--replay FILE] [--scenario duel|aram|minions|dodge|empty]
//!        mftr-server [--key FILE] --fingerprint
//!
//! `--key FILE` (default `server.key`) holds the server's key; it is created on first start.
//! Clients pin it, so keep the file: a new key makes every client that met the server refuse
//! it. `--fingerprint` prints the key's fingerprint (creating the key if needed) and exits.
//! `--bots N` adds N server bots at start (they count toward the player limit). `--lobby` starts
//! with champion select (ARAM all-random with rerolls and a bench; humans replace bots). `--replay FILE`
//! records the session and rewrites FILE every minute and whenever a match ends; check it with
//! `mftr-tools replay FILE`.

use mftr_net::secure::{Fingerprint, Identity, Received, SecureServer};
use mftr_server::{ClientKey, Scenario, ServerConfig, ServerCore};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::time::{Duration, Instant};

const USAGE: &str = "mftr-server [--bind ADDR] [--key FILE] [--fingerprint] [--seed N] [--max-players N] [--bots N] [--lobby] [--replay FILE] [--scenario duel|aram|minions|dodge|empty]";

/// Client sessions: which core connection each address is, and back.
#[derive(Default)]
struct Peers {
    keys: HashMap<SocketAddr, ClientKey>,
    addrs: HashMap<ClientKey, SocketAddr>,
    next: ClientKey,
}

impl Peers {
    /// A fresh connection for `addr` (a new session replaces the old one).
    fn connect(&mut self, addr: SocketAddr) {
        self.next += 1;
        if let Some(old) = self.keys.insert(addr, self.next) {
            self.addrs.remove(&old);
        }
        self.addrs.insert(self.next, addr);
    }

    fn close(&mut self, addr: SocketAddr) {
        if let Some(k) = self.keys.remove(&addr) {
            self.addrs.remove(&k);
        }
    }
}

fn send_all(socket: &UdpSocket, secure: &mut SecureServer, peers: &Peers, packets: Vec<(ClientKey, Vec<u8>)>) {
    for (to, bytes) in packets {
        if let Some(addr) = peers.addrs.get(&to)
            && let Some(datagram) = secure.seal(*addr, &bytes)
        {
            let _ = socket.send_to(&datagram, addr);
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut bind = "0.0.0.0:7777".to_string();
    let mut replay_path: Option<String> = None;
    let mut key_path = PathBuf::from("server.key");
    let mut fingerprint_only = false;
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
            "--lobby" => cfg.lobby = true,
            "--key" => key_path = args.next().expect("--key needs a file").into(),
            "--fingerprint" => fingerprint_only = true,
            "--replay" => {
                replay_path = Some(args.next().expect("--replay needs a file"));
                cfg.record = true;
            }
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

    let (identity, created) = Identity::load_or_create(&key_path, "server key")?;
    if fingerprint_only {
        println!("{}", identity.fingerprint());
        return Ok(());
    }
    if created {
        println!("created a new server key in {}", key_path.display());
    }

    let socket = UdpSocket::bind(&bind)?;
    socket.set_nonblocking(true)?;
    let local = socket.local_addr()?;
    println!("mftr-server listening on {local} (protocol {})", mftr_net::PROTOCOL_VERSION);
    println!(
        "key fingerprint {}: players can pin it with HOST:{}#{}",
        identity.fingerprint(),
        local.port(),
        identity.fingerprint()
    );

    let clock = Instant::now();
    let now = || clock.elapsed().as_secs_f64();
    let mut core = ServerCore::new(cfg, now());
    let mut secure = SecureServer::new(identity);
    let mut peers = Peers::default();
    let mut buf = [0u8; 2048];
    let mut last_expire = now();
    let mut last_report = now();
    let mut tick_cost = 0.0f64;
    let mut tick_cost_max = 0.0f64;
    let mut last_save = now();
    let mut had_winner = false;

    loop {
        loop {
            match socket.recv_from(&mut buf) {
                Ok((n, from)) => match secure.receive(from, &buf[..n], now()) {
                    Received::Reply(reply) => {
                        let _ = socket.send_to(&reply, from);
                    }
                    Received::Connected { client } => {
                        println!("player {} connected from {from}", Fingerprint::of(&client));
                        peers.connect(from);
                    }
                    Received::Data(packet) => {
                        if let Some(&key) = peers.keys.get(&from) {
                            let out = core.handle_packet(key, &packet, now());
                            send_all(&socket, &mut secure, &peers, out);
                        }
                    }
                    Received::Nothing => {}
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue, // Windows ICMP noise
                Err(e) => return Err(e),
            }
        }

        let t = now();
        if t >= core.next_tick_due() {
            let started = Instant::now();
            let out = core.step(t);
            send_all(&socket, &mut secure, &peers, out);
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

        if t - last_expire >= 1.0 {
            for addr in secure.expire(t) {
                peers.close(addr);
            }
            last_expire = t;
        }

        if t - last_report >= 5.0 {
            let s = &core.stats;
            println!(
                "tick {:>6}  players {} (+{} bots)  cmds {} (late {}, dropped {})  in {:.1} KB/s  out {:.1} KB/s  tick avg {:.3} ms max {:.3} ms",
                core.world().tick().0,
                core.player_count(),
                core.bot_count(),
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
