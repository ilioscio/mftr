//! One server, many matches (13, D57): the hub offers queues, places each player in a match
//! instance of the queue they pick (an instance is a whole `ServerCore`), creates instances as
//! they fill and stops them once their humans are gone.

use crate::{ClientKey, Scenario, ServerConfig, ServerCore, ServerStats};
use mftr_net::PROTOCOL_VERSION;
use mftr_net::msg::{self, ClientMessage, QueueInfo, RejectReason, ServerMessage};
use mftr_net::packet::{PacketHeader, SendTracker};
use std::collections::BTreeMap;
use std::sync::Arc;

/// The modes a hosted server offers (13 §1): full matches only.
pub const HOSTED_MODES: [Scenario; 4] = [Scenario::Aram, Scenario::Mayhem, Scenario::Hyper, Scenario::Classic];
/// A connection that picked no queue is forgotten after this long without a word.
const FRONT_TIMEOUT: f64 = 30.0;

/// One queue: a scenario on a map, Versus or Co-op vs bots.
#[derive(Clone, Debug)]
pub struct QueueSpec {
    pub scenario: Scenario,
    pub coop: bool,
    /// The map's name, shown to players.
    pub map_name: String,
    /// A custom map's text (`*.mftrmap`), played instead of the scenario's map.
    pub map: Option<Arc<str>>,
}

impl QueueSpec {
    /// A short name for logs and replay files, like `classic-coop-crossroads`.
    pub fn slug(&self) -> String {
        let map: String = self
            .map_name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
            .collect();
        let map = map.split('-').filter(|w| !w.is_empty()).collect::<Vec<_>>().join("-");
        format!("{}-{}-{map}", self.scenario.name(), if self.coop { "coop" } else { "versus" })
    }
}

/// The name of a built-in scenario's map.
pub fn map_name(scenario: Scenario) -> &'static str {
    match scenario.map() {
        mftr_sim::map::MapId::Open => "Open ground",
        mftr_sim::map::MapId::Arena => "The Arena",
        mftr_sim::map::MapId::Bridge => "The Bridge",
        mftr_sim::map::MapId::Crossroads | mftr_sim::map::MapId::Custom => "Crossroads",
    }
}

/// A hosted server's queues: each mode Versus and Co-op, Classic on Crossroads and on each
/// of `maps` (name, text).
pub fn hosted_queues(modes: &[Scenario], maps: &[(String, Arc<str>)]) -> Vec<QueueSpec> {
    let mut queues = Vec::new();
    for &scenario in modes {
        let mut boards = vec![(map_name(scenario).to_string(), None)];
        if scenario == Scenario::Classic {
            boards.extend(maps.iter().map(|(name, text)| (name.clone(), Some(text.clone()))));
        }
        for (map_name, map) in boards {
            for coop in [false, true] {
                queues.push(QueueSpec { scenario, coop, map_name: map_name.clone(), map: map.clone() });
            }
        }
    }
    queues
}

#[derive(Clone, Debug)]
pub struct HubConfig {
    pub queues: Vec<QueueSpec>,
    /// Each instance's settings, apart from its scenario, map, side and seed.
    pub base: ServerConfig,
    /// Instances running at once, at most (13 §2: past it, players are told the server is
    /// busy).
    pub max_matches: usize,
}

/// One match instance.
pub struct Instance {
    pub id: u32,
    /// Index into the hub's queues.
    pub queue: usize,
    pub core: ServerCore,
    /// When it was created: server clock, and Unix seconds (replay file names).
    pub created: f64,
    pub created_unix: u64,
}

/// A connection that hasn't picked a queue yet.
struct Front {
    send: SendTracker,
    last_heard: f64,
}

pub struct Hub {
    cfg: HubConfig,
    instances: Vec<Instance>,
    /// Which instance each placed connection talks to.
    routes: BTreeMap<ClientKey, u32>,
    front: BTreeMap<ClientKey, Front>,
    next_id: u32,
    /// Instances that stopped since `take_stopped` (their replays).
    stopped: Vec<Instance>,
    /// Packets handled before a connection was placed.
    pub stats: ServerStats,
}

impl Hub {
    pub fn new(cfg: HubConfig) -> Self {
        assert!(!cfg.queues.is_empty(), "a server offers at least one queue");
        assert!(cfg.queues.len() <= msg::MAX_QUEUES, "at most {} queues", msg::MAX_QUEUES);
        Self {
            cfg,
            instances: Vec::new(),
            routes: BTreeMap::new(),
            front: BTreeMap::new(),
            next_id: 1,
            stopped: Vec::new(),
            stats: ServerStats::default(),
        }
    }

    pub fn queues(&self) -> &[QueueSpec] {
        &self.cfg.queues
    }

    pub fn instances(&self) -> &[Instance] {
        &self.instances
    }

    /// The instance connection `key` was placed in.
    pub fn instance_of(&self, key: ClientKey) -> Option<&Instance> {
        let id = self.routes.get(&key)?;
        self.instances.iter().find(|i| i.id == *id)
    }

    /// Instances stopped since the last call.
    pub fn take_stopped(&mut self) -> Vec<Instance> {
        std::mem::take(&mut self.stopped)
    }

    /// Handle one incoming datagram: placed connections talk to their instance; anyone else
    /// may only say hello (and pick a queue).
    pub fn handle_packet(&mut self, from: ClientKey, bytes: &[u8], now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        if let Some(&id) = self.routes.get(&from) {
            if let Some(inst) = self.instances.iter_mut().find(|i| i.id == id) {
                let out = inst.core.handle_packet(from, bytes, now);
                if !inst.core.knows(from) {
                    // Said goodbye (or was turned away): back to the front desk.
                    self.routes.remove(&from);
                }
                return out;
            }
            self.routes.remove(&from);
        }
        self.stats.bytes_in += bytes.len() as u64;
        let Ok((_, ClientMessage::Hello { protocol, resume, spectate, queue, .. })) = msg::decode_client(bytes) else {
            self.stats.decode_errors += 1;
            return Vec::new();
        };
        if protocol != PROTOCOL_VERSION {
            return vec![(from, self.reject(RejectReason::ProtocolMismatch))];
        }
        // A returning player goes back to the instance holding their seat.
        if let Some(i) = self.instances.iter().position(|i| i.core.holds_token(resume)) {
            let id = self.instances[i].id;
            return self.place(from, id, bytes, now);
        }
        let queue = match queue {
            Some(q) if (q as usize) < self.cfg.queues.len() => q as usize,
            None if self.cfg.queues.len() == 1 => 0,
            _ => return self.queues_packet(from, now).into_iter().collect(),
        };
        let target = if spectate { self.busiest(queue) } else { self.with_room(queue) };
        let id = match target {
            Some(id) => id,
            None if spectate => return vec![(from, self.reject(RejectReason::NothingToWatch))],
            None if self.instances.len() >= self.cfg.max_matches => {
                return vec![(from, self.reject(RejectReason::ServerBusy))];
            }
            None => self.create(queue, now),
        };
        self.place(from, id, bytes, now)
    }

    /// Route `from` to instance `id` and hand it the hello.
    fn place(&mut self, from: ClientKey, id: u32, hello: &[u8], now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        self.front.remove(&from);
        let Some(inst) = self.instances.iter_mut().find(|i| i.id == id) else { return Vec::new() };
        let out = inst.core.handle_packet(from, hello, now);
        if inst.core.knows(from) {
            self.routes.insert(from, id);
        }
        out
    }

    /// The queue's instance a new player joins (13 §2): of those with room, the one with the
    /// most humans; on a tie, one in champion select, then the oldest.
    fn with_room(&self, queue: usize) -> Option<u32> {
        self.instances
            .iter()
            .filter(|i| i.queue == queue && i.core.has_room())
            .max_by_key(|i| (i.core.humans(), i.core.in_champion_select(), std::cmp::Reverse(i.id)))
            .map(|i| i.id)
    }

    /// The queue's instance with the most humans (for a spectator).
    fn busiest(&self, queue: usize) -> Option<u32> {
        self.instances
            .iter()
            .filter(|i| i.queue == queue && i.core.connected_humans() > 0)
            .max_by_key(|i| (i.core.connected_humans(), std::cmp::Reverse(i.id)))
            .map(|i| i.id)
    }

    fn create(&mut self, queue: usize, now: f64) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let q = &self.cfg.queues[queue];
        let cfg = ServerConfig {
            scenario: q.scenario,
            map: q.map.clone(),
            coop: q.coop,
            // Each instance plays its own matches.
            seed: self.cfg.base.seed.wrapping_add((id as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)),
            ..self.cfg.base.clone()
        };
        let created_unix =
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        self.instances.push(Instance { id, queue, core: ServerCore::new(cfg, now), created: now, created_unix });
        id
    }

    fn queues_packet(&mut self, to: ClientKey, now: f64) -> Option<(ClientKey, Vec<u8>)> {
        let infos: Vec<QueueInfo> = self
            .cfg
            .queues
            .iter()
            .enumerate()
            .map(|(i, q)| {
                let mine = self.instances.iter().filter(|x| x.queue == i);
                let humans: usize = mine.clone().map(|x| x.core.connected_humans()).sum();
                QueueInfo {
                    mode: q.scenario.mode(),
                    coop: q.coop,
                    map: q.map_name.clone(),
                    humans: humans.min(u16::MAX as usize) as u16,
                    matches: mine.count().min(u16::MAX as usize) as u16,
                }
            })
            .collect();
        let front = self.front.entry(to).or_insert(Front { send: SendTracker::default(), last_heard: now });
        front.last_heard = now;
        let header = PacketHeader { seq: front.send.next_seq(), ack: 0, ack_bits: 0 };
        let bytes = msg::encode_server(&header, &ServerMessage::Queues(infos));
        self.stats.bytes_out += bytes.len() as u64;
        Some((to, bytes))
    }

    fn reject(&mut self, reason: RejectReason) -> Vec<u8> {
        let bytes = msg::encode_server(&PacketHeader::default(), &ServerMessage::Reject { reason });
        self.stats.bytes_out += bytes.len() as u64;
        bytes
    }

    /// When the next instance's tick is due (server clock); `None` with no instances.
    pub fn next_tick_due(&self) -> Option<f64> {
        self.instances.iter().map(|i| i.core.next_tick_due()).min_by(f64::total_cmp)
    }

    /// Step every instance whose tick is due, then stop those without humans (13 §5).
    pub fn step(&mut self, now: f64) -> Vec<(ClientKey, Vec<u8>)> {
        let mut out = Vec::new();
        for inst in &mut self.instances {
            if now >= inst.core.next_tick_due() {
                out.extend(inst.core.step(now));
            }
        }
        let (gone, kept): (Vec<Instance>, Vec<Instance>) =
            std::mem::take(&mut self.instances).into_iter().partition(|i| i.core.humans() == 0);
        self.instances = kept;
        self.stopped.extend(gone);
        let instances = &self.instances;
        self.routes.retain(|key, id| instances.iter().any(|i| i.id == *id && i.core.knows(*key)));
        self.front.retain(|_, f| now - f.last_heard < FRONT_TIMEOUT);
        out
    }

    /// Statistics summed over the front desk and every instance; `reset` zeroes the byte
    /// counts (for per-interval rates).
    pub fn stats(&mut self, reset: bool) -> ServerStats {
        let mut sum = self.stats.clone();
        for inst in &mut self.instances {
            sum.add(&inst.core.stats);
            if reset {
                inst.core.stats.bytes_in = 0;
                inst.core.stats.bytes_out = 0;
            }
        }
        if reset {
            self.stats.bytes_in = 0;
            self.stats.bytes_out = 0;
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mftr_net::msg::{decode_server, encode_client};

    fn hub(max_matches: usize) -> Hub {
        Hub::new(HubConfig {
            queues: hosted_queues(&HOSTED_MODES, &[]),
            base: ServerConfig { max_players: 10, bots: 10, lobby: true, ..Default::default() },
            max_matches,
        })
    }

    fn hello(queue: Option<u8>, resume: u64, spectate: bool) -> Vec<u8> {
        encode_client(
            &PacketHeader::default(),
            &ClientMessage::Hello {
                protocol: PROTOCOL_VERSION,
                client_time_us: 0,
                champion: None,
                resume,
                spectate,
                queue,
            },
        )
    }

    fn replies(out: Vec<(ClientKey, Vec<u8>)>, key: ClientKey) -> Vec<ServerMessage> {
        out.into_iter().filter(|(to, _)| *to == key).map(|(_, b)| decode_server(&b).unwrap().1).collect()
    }

    fn queue_index(hub: &Hub, scenario: Scenario, coop: bool) -> u8 {
        hub.queues().iter().position(|q| q.scenario == scenario && q.coop == coop).unwrap() as u8
    }

    /// 13 §2: a hello without a queue gets the list; picking one places the player in a new
    /// instance's champion select; others fill it before a second instance opens.
    #[test]
    fn players_pick_a_queue_and_fill_instances() {
        let mut hub = hub(8);
        let msgs = replies(hub.handle_packet(1, &hello(None, 0, false), 0.0), 1);
        let Some(ServerMessage::Queues(qs)) = msgs.first() else { panic!("{msgs:?}") };
        assert_eq!(qs.len(), 8, "four modes, Versus and Co-op");
        assert!(qs.iter().any(|q| q.mode == msg::GameMode::Classic && q.coop && q.map == "Crossroads"));
        assert!(hub.instances().is_empty(), "asking costs nothing");

        let aram = queue_index(&hub, Scenario::Aram, false);
        for key in 1..=10 {
            let msgs = replies(hub.handle_packet(key, &hello(Some(aram), 0, false), 0.0), key);
            assert!(matches!(msgs.first(), Some(ServerMessage::Lobby(_))), "{key}: {msgs:?}");
        }
        assert_eq!(hub.instances().len(), 1, "ten humans fit one match");
        assert_eq!(hub.instances()[0].core.humans(), 10);
        let msgs = replies(hub.handle_packet(11, &hello(Some(aram), 0, false), 0.0), 11);
        assert!(matches!(msgs.first(), Some(ServerMessage::Lobby(_))));
        assert_eq!(hub.instances().len(), 2, "the eleventh opens a second");
        assert_eq!(hub.instance_of(11).unwrap().id, hub.instances()[1].id);

        // The queue list counts them.
        let msgs = replies(hub.handle_packet(50, &hello(None, 0, false), 0.0), 50);
        let Some(ServerMessage::Queues(qs)) = msgs.first() else { panic!() };
        assert_eq!((qs[aram as usize].humans, qs[aram as usize].matches), (11, 2));
    }

    /// 13 §2: in co-op, five humans fill blue and the sixth opens another instance; in
    /// Versus, the newcomer goes to the team with fewer humans.
    #[test]
    fn coop_keeps_humans_on_blue() {
        let mut hub = hub(8);
        let coop = queue_index(&hub, Scenario::Classic, true);
        for key in 1..=6 {
            hub.handle_packet(key, &hello(Some(coop), 0, false), 0.0);
        }
        assert_eq!(hub.instances().len(), 2);
        assert_eq!(hub.instances()[0].core.humans(), 5);
        let lobby = |hub: &mut Hub, key| {
            replies(hub.handle_packet(key, &hello(Some(coop), 0, false), 0.1), key)
                .into_iter()
                .find_map(|m| if let ServerMessage::Lobby(l) = m { Some(l) } else { None })
                .unwrap()
        };
        let l = lobby(&mut hub, 1);
        assert!(l.slots.iter().filter(|s| !s.bot).all(|s| s.team == mftr_sim::Team::Blue));
    }

    /// 13 §2 and §5: past the limit, players are told the server is busy; an instance whose
    /// humans leave stops, making room again. Spectators watch the busiest instance and
    /// can't keep one running.
    #[test]
    fn busy_servers_and_instances_that_stop() {
        let mut hub = hub(1);
        let aram = queue_index(&hub, Scenario::Aram, false);
        let hyper = queue_index(&hub, Scenario::Hyper, false);
        hub.handle_packet(1, &hello(Some(aram), 0, false), 0.0);
        let msgs = replies(hub.handle_packet(2, &hello(Some(hyper), 0, false), 0.0), 2);
        assert_eq!(msgs, vec![ServerMessage::Reject { reason: RejectReason::ServerBusy }]);
        let msgs = replies(hub.handle_packet(3, &hello(Some(hyper), 0, true), 0.0), 3);
        assert_eq!(msgs, vec![ServerMessage::Reject { reason: RejectReason::NothingToWatch }]);
        let msgs = replies(hub.handle_packet(3, &hello(Some(aram), 0, true), 0.0), 3);
        assert!(msgs.iter().any(|m| matches!(m, ServerMessage::Welcome { spectator: true, .. })), "{msgs:?}");

        // The player leaves champion select: the instance stops with only a spectator left.
        let bye = encode_client(&PacketHeader::default(), &ClientMessage::Bye);
        hub.handle_packet(1, &bye, 0.5);
        hub.step(1.0);
        assert!(hub.instances().is_empty());
        assert_eq!(hub.take_stopped().len(), 1);
        assert!(hub.instance_of(3).is_none(), "the spectator is back at the front desk");
        let msgs = replies(hub.handle_packet(2, &hello(Some(hyper), 0, false), 1.0), 2);
        assert!(matches!(msgs.first(), Some(ServerMessage::Lobby(_))));
    }
}
