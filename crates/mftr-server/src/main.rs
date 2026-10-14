//! `mftr-server`: dedicated UDP match server, encrypted with the secure transport (D40). One
//! process on one port plays whatever its players ask for (13): they pick a queue, and the
//! server runs match instances of it, creating and stopping them as players come and go.
//!
//! Usage: mftr-server [--bind 0.0.0.0:7777] [--key FILE] [--seed N] [--modes aram,mayhem-coop,hyper,classic-versus,...]
//!                    [--map FILE.mftrmap]... [--max-matches N] [--workers N] [--replay DIR]
//!        mftr-server --scenario NAME [--bots N] [--lobby] [--max-players N] [--map FILE.mftrmap] [...]
//!        mftr-server [--key FILE] --fingerprint
//!
//! `--key FILE` (default `server.key`) holds the server's key; it is created on first start.
//! Clients pin it, so keep the file: a new key makes every client that met the server refuse
//! it. `--fingerprint` prints the key's fingerprint (creating the key if needed) and exits.
//!
//! By default the server offers every full-match mode (ARAM, Mayhem, Hyper, Classic), each
//! Versus and Co-op vs bots, with champion select and bots in every seat a human doesn't take;
//! `--modes` narrows the list (`aram` offers both sides, `aram-versus` or `aram-coop` one).
//! Each `--map FILE` (from the map editor) adds its modes on that map: Classic on a Classic
//! map, the ARAM modes on an ARAM map. `--max-matches N` caps the instances running at once;
//! `--workers N` threads step them (default: one per core). `--replay DIR` records every match
//! into DIR (rewritten every minute and when a match ends); check one with
//! `mftr-tools replay FILE`.
//!
//! `--scenario NAME` (duel, aram, mayhem, hyper, classic, minions, dodge, empty) offers that
//! scenario alone, for development: `--bots N` adds server bots (they count toward
//! `--max-players`), `--lobby` holds champion select first. Spectators can watch bots alone
//! there: a match starts for them, and runs while anyone watches.

use mftr_net::secure::{Fingerprint, Identity, Received, SecureServer};
use mftr_server::hub::{self, HOSTED_MODES, ModeOffer};
use mftr_server::{ClientKey, Hub, HubConfig, QueueSpec, Scenario, ServerConfig};
use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

const USAGE: &str = "mftr-server [--bind ADDR] [--key FILE] [--fingerprint] [--seed N] [--modes aram,mayhem-coop,hyper,classic-versus,...] [--map FILE.mftrmap]... [--max-matches N] [--workers N] [--replay DIR]\n       mftr-server --scenario duel|aram|mayhem|hyper|classic|minions|dodge|empty [--bots N] [--lobby] [--max-players N] [...]";
/// Instances running at once, by default.
const MAX_MATCHES: usize = 32;

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

/// A map file from the map editor: checked now (the server refuses one that breaks a rule).
fn read_map(path: &str) -> hub::HostedMap {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    if text.len() > mftr_sim::mapspec::MAX_TEXT_BYTES {
        panic!("{path}: a map file holds at most {} bytes", mftr_sim::mapspec::MAX_TEXT_BYTES);
    }
    let spec = mftr_sim::mapspec::MapSpec::parse(&text).unwrap_or_else(|e| panic!("{path}: {e}"));
    let problems = mftr_sim::mapspec::problems(&spec);
    if !problems.is_empty() {
        panic!("{path} can't be played yet:\n  {}", problems.join("\n  "));
    }
    println!("map: {} ({}, {path})", spec.name, spec.kind.key());
    hub::HostedMap { name: spec.name.clone(), kind: spec.kind, text: Arc::from(text.as_str()) }
}

/// Where an instance's replay goes: `DIR/<started>-<queue>-<instance>.replay`.
fn replay_file(dir: &str, inst: &hub::Instance, queue: &QueueSpec) -> PathBuf {
    PathBuf::from(dir).join(format!("{}-{}-{}.replay", inst.created_unix, queue.slug(), inst.id))
}

fn main() -> std::io::Result<()> {
    let mut bind = "0.0.0.0:7777".to_string();
    let mut replay_dir: Option<String> = None;
    let mut key_path = PathBuf::from("server.key");
    let mut fingerprint_only = false;
    let mut base = ServerConfig::default();
    let mut scenario: Option<Scenario> = None;
    let mut modes: Option<Vec<ModeOffer>> = None;
    let mut maps: Vec<hub::HostedMap> = Vec::new();
    let mut max_matches = MAX_MATCHES;
    let mut workers: Option<usize> = None;
    let (mut bots, mut lobby, mut max_players) = (None, false, None);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--bind" => bind = args.next().expect("--bind needs an address"),
            "--seed" => base.seed = args.next().and_then(|v| v.parse().ok()).expect("--seed needs a number"),
            "--max-players" => {
                max_players = Some(args.next().and_then(|v| v.parse().ok()).expect("--max-players needs a number"))
            }
            "--bots" => bots = Some(args.next().and_then(|v| v.parse().ok()).expect("--bots needs a number")),
            "--lobby" => lobby = true,
            "--key" => key_path = args.next().expect("--key needs a file").into(),
            "--fingerprint" => fingerprint_only = true,
            "--replay" => {
                replay_dir = Some(args.next().expect("--replay needs a directory"));
                base.record = true;
            }
            "--max-matches" => {
                max_matches = args.next().and_then(|v| v.parse().ok()).expect("--max-matches needs a number")
            }
            "--workers" => workers = Some(args.next().and_then(|v| v.parse().ok()).expect("--workers needs a number")),
            "--scenario" => {
                let name = args.next().expect("--scenario needs a name");
                scenario = Some(Scenario::by_name(&name).unwrap_or_else(|| panic!("unknown scenario {name}\n{USAGE}")));
            }
            "--modes" => {
                let list = args.next().expect("--modes needs a list, like aram,classic-coop");
                modes = Some(
                    list.split(',')
                        .map(|name| ModeOffer::parse(name).unwrap_or_else(|e| panic!("{e}\n{USAGE}")))
                        .collect(),
                );
            }
            "--map" => maps.push(read_map(&args.next().expect("--map needs a file"))),
            "-h" | "--help" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => panic!("unknown argument {other}\n{USAGE}"),
        }
    }

    let queues = match scenario {
        // One scenario, for development: as configured.
        Some(scenario) => {
            if modes.is_some() || maps.len() > 1 {
                panic!("--scenario offers one scenario: no --modes, and at most one --map\n{USAGE}");
            }
            base.bots = bots.unwrap_or(0);
            base.lobby = lobby;
            base.max_players = max_players.unwrap_or(base.max_players);
            let map = maps.pop();
            if let Some(m) = &map
                && hub::map_kind(scenario) != Some(m.kind)
            {
                panic!("{} is a {} map: --scenario {} can't be played on it", m.name, m.kind.key(), scenario.name());
            }
            let name = map.as_ref().map_or(hub::map_name(scenario).to_string(), |m| m.name.clone());
            vec![QueueSpec { scenario, coop: false, map_name: name, map: map.map(|m| m.text) }]
        }
        // Hosted: every full-match queue, with champion select and bots in every free seat.
        None => {
            if bots.is_some() || lobby || max_players.is_some() {
                panic!("--bots, --lobby and --max-players go with --scenario\n{USAGE}");
            }
            base.max_players = 10;
            base.bots = 10;
            base.lobby = true;
            let modes = modes.unwrap_or_else(|| HOSTED_MODES.map(ModeOffer::both).to_vec());
            hub::hosted_queues(&modes, &maps)
        }
    };
    if let Some(dir) = &replay_dir {
        std::fs::create_dir_all(dir)?;
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
    println!("{} queues, up to {max_matches} matches at once:", queues.len());
    for q in &queues {
        println!("  {}", q.slug());
    }

    let clock = Instant::now();
    let now = || clock.elapsed().as_secs_f64();
    let workers = workers.unwrap_or_else(|| std::thread::available_parallelism().map_or(1, |n| n.get()));
    let development = scenario.is_some();
    let mut hub = Hub::new(HubConfig { queues, base, max_matches, workers, development });
    let mut secure = SecureServer::new(identity);
    let mut peers = Peers::default();
    let mut buf = [0u8; 2048];
    let mut last_expire = now();
    let mut last_report = now();
    // Time spent stepping the matches each tick (all of them, on every worker), against the
    // tick's 33 ms.
    let (mut busy, mut steps, mut step_max) = (0.0f64, 0u32, 0.0f64);
    // Per instance: when its replay was last written, and whether its match had a winner.
    let mut saves: HashMap<u32, (f64, bool)> = HashMap::new();
    let mut seen: HashMap<u32, f64> = HashMap::new();

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
                            let out = hub.handle_packet(key, &packet, now());
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
        if hub.next_tick_due().is_some_and(|due| t >= due) {
            let started = Instant::now();
            let out = hub.step(t);
            send_all(&socket, &mut secure, &peers, out);
            let cost = started.elapsed().as_secs_f64();
            busy += cost;
            steps += 1;
            step_max = step_max.max(cost);
        }

        // Matches that started and stopped; replays.
        for inst in hub.instances() {
            if let std::collections::hash_map::Entry::Vacant(e) = seen.entry(inst.id) {
                e.insert(t);
                println!("match {} started: {}", inst.id, hub.queues()[inst.queue].slug());
            }
            if let Some(dir) = &replay_dir {
                let winner = inst.core.world().game().winner.is_some();
                let (last, had) = saves.entry(inst.id).or_insert((t, false));
                if (winner && !*had) || t - *last >= 60.0 {
                    let path = replay_file(dir, inst, &hub.queues()[inst.queue]);
                    if let Err(e) = std::fs::write(&path, inst.core.game().replay().to_text()) {
                        eprintln!("could not write replay {}: {e}", path.display());
                    }
                    *last = t;
                }
                *had = winner;
            }
        }
        for inst in hub.take_stopped() {
            let ran = t - seen.remove(&inst.id).unwrap_or(t);
            println!("match {} stopped after {}:{:02}: nobody left", inst.id, ran as u32 / 60, ran as u32 % 60);
            saves.remove(&inst.id);
            if let Some(dir) = &replay_dir {
                let path = replay_file(dir, &inst, &hub.queues()[inst.queue]);
                if let Err(e) = std::fs::write(&path, inst.core.game().replay().to_text()) {
                    eprintln!("could not write replay {}: {e}", path.display());
                }
            }
        }

        if t - last_expire >= 1.0 {
            for addr in secure.expire(t) {
                peers.close(addr);
            }
            last_expire = t;
        }

        if t - last_report >= 5.0 {
            let span = t - last_report;
            if !hub.instances().is_empty() || busy > 0.0 {
                let humans: usize = hub.instances().iter().map(|i| i.core.connected_humans()).sum();
                let bots: usize = hub.instances().iter().map(|i| i.core.bot_count()).sum();
                let s = hub.stats(true);
                println!(
                    "matches {}  players {humans} (+{bots} bots)  cmds {} (late {}, dropped {})  in {:.1} KB/s  out {:.1} KB/s  tick {:.2} ms (max {:.2}) of {:.0}",
                    hub.instances().len(),
                    s.commands,
                    s.commands_late,
                    s.commands_dropped,
                    s.bytes_in as f64 / 1024.0 / span,
                    s.bytes_out as f64 / 1024.0 / span,
                    busy * 1e3 / steps.max(1) as f64,
                    step_max * 1e3,
                    mftr_sim::TICK_DT_F64 * 1e3,
                );
            }
            (busy, steps, step_max) = (0.0, 0, 0.0);
            last_report = t;
        }

        let wait = hub.next_tick_due().map_or(0.001, |due| (due - now()).clamp(0.0, 0.001));
        std::thread::sleep(Duration::from_secs_f64(wait));
    }
}
