//! A practice server inside the client's own process (CR3, 20 §10): the champion creator's
//! **Test**. It listens on a free port of 127.0.0.1 with a fresh key, offers one queue, and stops
//! when dropped. The client joins it like any server, with the key pinned in the address
//! (`127.0.0.1:port#fingerprint`), so nothing is trusted on first use.

use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use mftr_net::secure::{Identity, Received, SecureServer};

use crate::hub::{self, Hub, HubConfig, QueueSpec};
use crate::{ClientKey, ServerConfig};

pub struct LocalServer {
    address: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl LocalServer {
    /// Start serving `cfg` (its scenario is the one queue) on a free local port.
    pub fn start(cfg: ServerConfig) -> std::io::Result<LocalServer> {
        let socket = UdpSocket::bind("127.0.0.1:0")?;
        socket.set_nonblocking(true)?;
        let port = socket.local_addr()?.port();
        let identity = Identity::generate();
        let address = format!("127.0.0.1:{port}#{}", identity.fingerprint());
        let scenario = cfg.scenario;
        let queues =
            vec![QueueSpec { scenario, coop: false, map_name: hub::map_name(scenario).to_string(), map: None }];
        let hub = Hub::new(HubConfig { queues, base: cfg, max_matches: 1, workers: 1, development: true });
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::Builder::new()
            .name("mftr-practice".into())
            .spawn(move || serve(socket, identity, hub, &flag))?;
        Ok(LocalServer { address, stop, thread: Some(thread) })
    }

    /// Where to join: `127.0.0.1:port#fingerprint`.
    pub fn address(&self) -> &str {
        &self.address
    }
}

impl Drop for LocalServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

/// The server loop (`mftr-server`'s main loop without its logs and replays).
fn serve(socket: UdpSocket, identity: Identity, mut hub: Hub, stop: &AtomicBool) {
    let mut secure = SecureServer::new(identity);
    let mut keys: HashMap<SocketAddr, ClientKey> = HashMap::new();
    let mut addrs: HashMap<ClientKey, SocketAddr> = HashMap::new();
    let mut next: ClientKey = 0;
    let clock = Instant::now();
    let now = || clock.elapsed().as_secs_f64();
    let mut buf = [0u8; 2048];
    let mut last_expire = 0.0;
    let send = |secure: &mut SecureServer, addrs: &HashMap<ClientKey, SocketAddr>, out: Vec<(ClientKey, Vec<u8>)>| {
        for (to, bytes) in out {
            if let Some(addr) = addrs.get(&to)
                && let Some(datagram) = secure.seal(*addr, &bytes)
            {
                let _ = socket.send_to(&datagram, addr);
            }
        }
    };
    while !stop.load(Ordering::Relaxed) {
        loop {
            match socket.recv_from(&mut buf) {
                Ok((n, from)) => match secure.receive(from, &buf[..n], now()) {
                    Received::Reply(reply) => {
                        let _ = socket.send_to(&reply, from);
                    }
                    Received::Replies(replies) => {
                        for reply in replies {
                            let _ = socket.send_to(&reply, from);
                        }
                    }
                    Received::Connected { .. } => {
                        next += 1;
                        if let Some(old) = keys.insert(from, next) {
                            addrs.remove(&old);
                        }
                        addrs.insert(next, from);
                    }
                    Received::Data(packet) => {
                        if let Some(&key) = keys.get(&from) {
                            let out = hub.handle_packet(key, &packet, now());
                            send(&mut secure, &addrs, out);
                        }
                    }
                    Received::Nothing => {}
                },
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => continue,
                Err(_) => return,
            }
        }
        let t = now();
        if hub.next_tick_due().is_some_and(|due| t >= due) {
            let out = hub.step(t);
            send(&mut secure, &addrs, out);
        }
        if t - last_expire >= 1.0 {
            for addr in secure.expire(t) {
                if let Some(k) = keys.remove(&addr) {
                    addrs.remove(&k);
                }
            }
            last_expire = t;
        }
        let wait = hub.next_tick_due().map_or(0.002, |due| (due - now()).clamp(0.0, 0.002));
        std::thread::sleep(Duration::from_secs_f64(wait));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_practice_server_starts_and_stops() {
        let cfg = ServerConfig { scenario: crate::Scenario::Duel, bots: 1, max_players: 2, ..ServerConfig::default() };
        let s = LocalServer::start(cfg).unwrap();
        assert!(s.address().starts_with("127.0.0.1:") && s.address().contains('#'));
        drop(s); // joins the thread
    }
}
