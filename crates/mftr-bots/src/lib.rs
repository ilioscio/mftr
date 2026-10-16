//! `mftr-bots`: server bots (17). Each team has a coach and a memory; each bot an agent. They
//! see only what their team sees (D37), react no faster than their tier allows, and play
//! through ordinary commands, so replays record them like anyone else.
//!
//! The server owns one [`Bots`] per match: it adds and removes bots as seats change hands,
//! asks for commands before each tick ([`Bots::think`]), shows them what each tick did
//! ([`Bots::observe`]), relays human pings to them ([`Bots::hear`]) and theirs to players
//! ([`Bots::take_pings`]).

pub mod agent;
pub mod aim;
pub mod board;
pub mod coach;
pub mod dodge;
pub mod fight;
pub mod jungle;
pub mod kit;
pub mod knowledge;
pub mod laning;
pub mod legacy;
pub mod memory;
pub mod micro;
pub mod shop;
pub mod tier;

use agent::{Agent, AgentStats, Ctx};
use board::Board;
use coach::{Coach, Member, Mode};
use knowledge::Knowledge;
use legacy::LegacyBot;
use memory::Memory;
use mftr_sim::ping::Ping;
use mftr_sim::vision::Vision;
use mftr_sim::{Command, PlayerId, SimEvent, SimTime, SubTick, Team, Tick, UnitKind, World};
use std::sync::Arc;
pub use tier::Tier;

/// The other team.
pub fn other(team: Team) -> Team {
    match team {
        Team::Blue => Team::Red,
        _ => Team::Blue,
    }
}

enum Brain {
    Legacy(LegacyBot),
    Agent(Box<Agent>),
}

struct Seat {
    player: PlayerId,
    brain: Brain,
    seq: u32,
}

/// A team's shared mind: what it remembers and its coach.
struct Mind {
    memory: Memory,
    coach: Coach,
}

/// Every bot in a match.
pub struct Bots {
    seed: u64,
    seats: Vec<Seat>,
    minds: [Option<Mind>; 2],
    knowledge: Option<Arc<Knowledge>>,
    map: usize,
    pings: Vec<(Team, Ping)>,
}

impl Bots {
    pub fn new(seed: u64) -> Self {
        Bots { seed, seats: Vec::new(), minds: [None, None], knowledge: None, map: 0, pings: Vec::new() }
    }

    /// A bot takes `player`'s seat (replacing any bot already there).
    pub fn add(&mut self, player: PlayerId, tier: Tier) {
        self.remove(player);
        let brain = match tier {
            Tier::Legacy => Brain::Legacy(LegacyBot::new(player, self.seed)),
            t => Brain::Agent(Box::new(Agent::new(player, t, self.seed))),
        };
        self.seats.push(Seat { player, brain, seq: 1 });
        self.seats.sort_by_key(|s| s.player);
    }

    pub fn remove(&mut self, player: PlayerId) {
        self.seats.retain(|s| s.player != player);
    }

    pub fn clear(&mut self) {
        self.seats.clear();
        self.minds = [None, None];
        self.pings.clear();
    }

    pub fn contains(&self, player: PlayerId) -> bool {
        self.seats.iter().any(|s| s.player == player)
    }

    pub fn players(&self) -> impl Iterator<Item = PlayerId> + '_ {
        self.seats.iter().map(|s| s.player)
    }

    pub fn len(&self) -> usize {
        self.seats.len()
    }

    pub fn is_empty(&self) -> bool {
        self.seats.is_empty()
    }

    /// A bot's tier.
    pub fn tier(&self, player: PlayerId) -> Option<Tier> {
        self.seats.iter().find(|s| s.player == player).map(|s| match &s.brain {
            Brain::Legacy(_) => Tier::Legacy,
            Brain::Agent(a) => a.tier,
        })
    }

    /// What a bot is up to, for traces: its mode, the coach's task and its role.
    pub fn describe(&self, player: PlayerId) -> Option<String> {
        let seat = self.seats.iter().find(|s| s.player == player)?;
        let Brain::Agent(a) = &seat.brain else { return Some("legacy".into()) };
        let (task, role) = self
            .minds
            .iter()
            .flatten()
            .find_map(|m| m.coach.task(player).map(|t| (Some(t), m.coach.role(player))))
            .unwrap_or((None, None));
        Some(format!("{:?} {:?} {:?}", a.mode, role, task))
    }

    /// Each bot's counters (17 §11).
    pub fn stats(&self) -> Vec<(PlayerId, AgentStats)> {
        self.seats
            .iter()
            .filter_map(|s| match &s.brain {
                Brain::Agent(a) => Some((s.player, a.stats)),
                Brain::Legacy(_) => None,
            })
            .collect()
    }

    fn knowledge(&mut self, world: &World) -> Arc<Knowledge> {
        let id = Arc::as_ptr(world.map()) as usize;
        if self.map != id || self.knowledge.is_none() {
            self.map = id;
            self.knowledge = Some(Knowledge::of(world.map()));
            self.minds = [None, None];
        }
        self.knowledge.clone().expect("set above")
    }

    fn mode(world: &World, k: &Knowledge) -> Mode {
        if world.rules().horde {
            Mode::Horde
        } else if k.lanes.len() >= 2 {
            Mode::Classic
        } else if !k.lanes.is_empty() {
            Mode::Aram
        } else {
            Mode::Open
        }
    }

    fn mind(&mut self, world: &World, team: Team, k: &Knowledge) -> &mut Mind {
        let mode = Self::mode(world, k);
        let seed = self.seed;
        self.minds[team as usize]
            .get_or_insert_with(|| Mind { memory: Memory::new(team), coach: Coach::new(team, mode, seed) })
    }

    /// The teams some agent plays on.
    fn teams(&self, world: &World) -> [bool; 2] {
        let mut t = [false, false];
        for s in self.seats.iter().filter(|s| matches!(s.brain, Brain::Agent(_))) {
            if let Some(u) = world.units().iter().find(|u| u.owner == Some(s.player) && u.kind == UnitKind::Champion)
                && (u.team as usize) < 2
            {
                t[u.team as usize] = true;
            }
        }
        t
    }

    /// Learn from the tick just simulated (call after every step): its events, and each team's
    /// vision after it.
    pub fn observe(&mut self, world: &World, events: &[SimEvent], visions: &[Vision; 2]) {
        if self.seats.is_empty() {
            return;
        }
        let k = self.knowledge(world);
        let teams = self.teams(world);
        for (i, team) in [Team::Blue, Team::Red].into_iter().enumerate() {
            if teams[i] {
                self.mind(world, team, &k).memory.observe(world, events, &visions[i], &k);
            }
        }
    }

    /// A human on `team` pinged (bots act on it, 17 §8).
    pub fn hear(&mut self, team: Team, ping: Ping, world: &World) {
        if (team as usize) < 2
            && let Some(m) = self.minds[team as usize].as_mut()
        {
            m.coach.hear(ping, SimTime::end_of(world.tick()));
        }
    }

    /// Pings the bots sent since the last call, with their team.
    pub fn take_pings(&mut self) -> Vec<(Team, Ping)> {
        for (i, team) in [Team::Blue, Team::Red].into_iter().enumerate() {
            if let Some(m) = self.minds[i].as_mut() {
                self.pings.extend(m.coach.out.drain(..).map(|p| (team, p)));
            }
        }
        std::mem::take(&mut self.pings)
    }

    /// Commands for tick `k` (the tick about to be simulated).
    pub fn think(&mut self, world: &World, k: Tick) -> Vec<Command> {
        let mut out = Vec::new();
        if self.seats.is_empty() {
            return out;
        }
        let know = self.knowledge(world);
        let teams = self.teams(world);
        for (i, team) in [Team::Blue, Team::Red].into_iter().enumerate() {
            if !teams[i] {
                continue;
            }
            // Whose turn is it?
            let due: Vec<usize> = (0..self.seats.len())
                .filter(|&s| matches!(&self.seats[s].brain, Brain::Agent(a) if a.due(k)))
                .filter(|&s| {
                    world.units().iter().any(|u| {
                        u.owner == Some(self.seats[s].player) && u.kind == UnitKind::Champion && u.team == team
                    })
                })
                .collect();
            let coach_due = self.minds[i].as_ref().is_none_or(|m| m.coach.due(k));
            if due.is_empty() && !coach_due {
                continue;
            }
            self.mind(world, team, &know);
            let members = self.members(world, team);
            let bots: Vec<mftr_sim::UnitId> = members.iter().filter(|m| m.bot).map(|m| m.unit).collect();
            let mind = self.minds[i].as_mut().expect("made above");
            let tracking = members
                .iter()
                .filter(|m| m.bot)
                .filter_map(|m| self.seats.iter().find(|s| s.player == m.player))
                .filter_map(|s| match &s.brain {
                    Brain::Agent(a) => Some(a.p.tracking),
                    Brain::Legacy(_) => None,
                })
                .max()
                .unwrap_or(tier::Tracking::None);
            let board = Board::build(world, team, &mind.memory, &know, tracking);
            if coach_due {
                mind.coach.update(&board, &mind.memory, &know, &members, k);
            }
            for s in due {
                let seat = &mut self.seats[s];
                let Brain::Agent(agent) = &mut seat.brain else { continue };
                let Some(me) =
                    world.units().iter().find(|u| u.owner == Some(seat.player) && u.kind == UnitKind::Champion)
                else {
                    continue;
                };
                let ctx = Ctx {
                    b: &board,
                    memory: &mind.memory,
                    k: &know,
                    coach: &mind.coach,
                    me,
                    now: board.now,
                    strategy: agent.p.strategy,
                    bots: &bots,
                    role: mind.coach.role(seat.player),
                };
                let (menu, action) = agent.think(&ctx, k);
                for kind in [menu, action].into_iter().flatten() {
                    seat.seq += 1;
                    out.push(Command { player: seat.player, seq: seat.seq, tick: k, sub: SubTick::START, kind });
                }
            }
        }
        // The baseline bots think on their own.
        for seat in self.seats.iter_mut() {
            if let Brain::Legacy(b) = &mut seat.brain {
                out.extend(b.think(world, k));
            }
        }
        out
    }

    /// The team's members as the coach sees them: every champion, with its bot's tier if any.
    fn members(&self, world: &World, team: Team) -> Vec<Member> {
        let mut out: Vec<Member> = world
            .units()
            .iter()
            .filter(|u| u.kind == UnitKind::Champion && u.team == team)
            .filter_map(|u| {
                let player = u.owner?;
                let seat = self.seats.iter().find(|s| s.player == player);
                let (bot, strategy, pings) = match seat.map(|s| &s.brain) {
                    Some(Brain::Agent(a)) => (true, a.p.strategy, a.p.pings),
                    Some(Brain::Legacy(_)) => (false, 0, 0.0),
                    None => (false, 0, 0.0),
                };
                Some(Member { player, unit: u.id, champion: u.champion?, bot, strategy, pings })
            })
            .collect();
        out.sort_by_key(|m| m.player);
        out
    }
}
