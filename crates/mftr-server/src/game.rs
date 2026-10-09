//! The match driver (M2 slice 5): everything the server does to the world each tick, apart from
//! networking. The live server and the replayer both run the same code, so a replay of the
//! recorded joins, leaves and commands re-simulates to the same state hashes.

use crate::{Scenario, ServerConfig};
use mftr_sim::vision::Vision;
use mftr_sim::{
    Brain, ChampionId, Command, CommandKind, MinionKind, PlayerId, QPoint, SimEvent, SimTime, SubTick, Team, Tick,
    UnitId, Vec2, World,
};
use std::collections::BTreeSet;
use std::fmt::Write;

/// A state hash is recorded this often (ticks) for replay verification.
pub const HASH_EVERY: u32 = 300;
/// A new match starts this long after a Base falls (ARAM).
const RESTART_AFTER_MS: u64 = 10_000;

/// What each team sees after a tick (03 §10).
pub struct Fog {
    pub visions: [Vision; 2],
    pub seen: [BTreeSet<UnitId>; 2],
}

/// One thing that changed the world, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum ReplayEntry {
    /// A player joined before tick `tick` was simulated.
    Join {
        tick: Tick,
        player: PlayerId,
        champion: Option<ChampionId>,
    },
    Leave {
        tick: Tick,
        player: PlayerId,
    },
    /// The commands handed to the world for tick `tick`.
    Commands {
        tick: Tick,
        commands: Vec<Command>,
    },
    /// A new match started (champions and all) before tick `tick + 1`: the server went
    /// back to champion select after a Base fell.
    Restart {
        tick: Tick,
    },
    /// The state hash after tick `tick`.
    Hash {
        tick: Tick,
        hash: u64,
    },
}

/// A recording: the server configuration plus every entry.
#[derive(Clone, Debug, PartialEq)]
pub struct Replay {
    pub cfg: ServerConfig,
    pub entries: Vec<ReplayEntry>,
}

pub struct Match {
    pub(crate) world: World,
    cfg: ServerConfig,
    players: BTreeSet<PlayerId>,
    log: Vec<ReplayEntry>,
}

impl Match {
    pub fn new(cfg: ServerConfig) -> Self {
        let mut world = World::new(cfg.seed);
        world.set_map(cfg.scenario.map().shared());
        match cfg.scenario {
            Scenario::Aram => world.set_rules(mftr_sim::world::Rules::ARAM),
            Scenario::Mayhem => world.set_rules(mftr_sim::world::Rules::MAYHEM),
            Scenario::Hyper => world.set_rules(mftr_sim::world::Rules::HYPER),
            _ => {}
        }
        populate(&mut world, cfg.scenario);
        Self { world, cfg, players: BTreeSet::new(), log: Vec::new() }
    }

    pub fn world(&self) -> &World {
        &self.world
    }

    pub fn cfg(&self) -> &ServerConfig {
        &self.cfg
    }

    /// The lowest player id not in use.
    pub fn free_player(&self) -> Option<PlayerId> {
        (0..=u8::MAX).map(PlayerId).find(|p| !self.players.contains(p))
    }

    pub fn player_count(&self) -> usize {
        self.players.len()
    }

    /// Add `player`'s champion: team by player id, champion as requested or assigned, spawn by
    /// scenario. Deterministic (the world RNG), so a replay places it identically.
    pub fn join(&mut self, player: PlayerId, champion: Option<ChampionId>) -> (UnitId, Team, ChampionId) {
        self.record(ReplayEntry::Join { tick: self.world.tick(), player, champion });
        self.players.insert(player);
        let scenario = self.cfg.scenario;
        let team = if player.0.is_multiple_of(2) || scenario == Scenario::DodgeRig { Team::Blue } else { Team::Red };
        // Without a preference: in ARAM, each of the six in turn; elsewhere alternate, so the
        // first duel is mage vs. marksman.
        let champion = champion.unwrap_or(if scenario.is_aram() {
            ChampionId::ALL[player.0 as usize % ChampionId::ALL.len()]
        } else {
            ChampionId::ALL[((player.0 / 2) as usize % 2) ^ (team == Team::Red) as usize]
        });
        let (lo, hi) = (self.cfg.arena_min, self.cfg.arena_max);
        // A random spot with nothing within 150 u (deterministic: the world RNG).
        let mut pos = Vec2::ZERO;
        for _ in 0..64 {
            let rng = self.world.rng();
            pos = QPoint::from_vec2(Vec2::new(rng.range_f32(lo, hi), rng.range_f32(lo, hi))).to_vec2();
            if self.world.map().walkable(pos, 60.0)
                && self.world.units().iter().all(|u| u.state.pos.distance(pos) > 150.0)
            {
                break;
            }
        }
        if scenario == Scenario::Duel {
            pos = duel_spawn(team, player);
        }
        if scenario.is_aram() {
            // In the fountain, spread out in a small arc per player.
            let base = self.world.map().layout.champion_spawn[team as usize];
            let k = (player.0 / 2) as f32;
            pos = base + Vec2::new(0.0, 90.0 * (k - 2.0));
        }
        let unit = self.world.spawn_champion(player, team, champion, pos);
        (unit, team, champion)
    }

    /// Everyone playing, humans and bots.
    pub fn players(&self) -> Vec<PlayerId> {
        self.players.iter().copied().collect()
    }

    /// A Base fell long enough ago that the next tick starts a new match.
    pub fn restart_due(&self) -> bool {
        self.world.game().winner.is_some_and(|(_, at)| {
            SimTime::end_of(self.world.tick()) >= at.plus(mftr_sim::SimDuration::from_millis(RESTART_AFTER_MS))
        })
    }

    /// Start a new match now, without waiting for `step` to (champion select between matches:
    /// everyone has left, and joins with a new champion before the next tick).
    pub fn restart(&mut self) {
        self.record(ReplayEntry::Restart { tick: self.world.tick() });
        self.world.restart_match();
    }

    pub fn leave(&mut self, player: PlayerId) {
        if !self.players.remove(&player) {
            return;
        }
        self.record(ReplayEntry::Leave { tick: self.world.tick(), player });
        let units: Vec<UnitId> = self.world.units().iter().filter(|u| u.owner == Some(player)).map(|u| u.id).collect();
        for u in units {
            self.world.despawn(u);
        }
    }

    /// Simulate the next tick with `due` (commands for it), then update fog: units a team can't
    /// see can't be targeted by its attacks next tick.
    pub fn step(&mut self, due: Vec<Command>) -> (Vec<SimEvent>, Fog) {
        // A new match 10 s after a Base falls (ARAM), with the same champions (servers with
        // champion select restart through `restart` instead).
        if self.restart_due() {
            self.world.restart_match();
        }
        let k = self.world.tick().next();
        self.world.step(&due);
        if !due.is_empty() {
            self.record(ReplayEntry::Commands { tick: k, commands: due });
        }
        let events = self.world.take_events();
        let map = self.world.map().clone();
        let visions = [Vision::of(&self.world, Team::Blue), Vision::of(&self.world, Team::Red)];
        let seen: [BTreeSet<UnitId>; 2] =
            [0, 1].map(|i| self.world.units().iter().filter(|u| visions[i].sees_unit(&map, u)).map(|u| u.id).collect());
        for (i, team) in [Team::Blue, Team::Red].into_iter().enumerate() {
            let hidden = self.world.units().iter().filter(|u| u.team != team && !seen[i].contains(&u.id)).map(|u| u.id);
            self.world.set_hidden(team, hidden.collect());
        }
        if self.cfg.record && k.0.is_multiple_of(HASH_EVERY) {
            self.record(ReplayEntry::Hash { tick: k, hash: self.world.state_hash() });
        }
        (events, Fog { visions, seen })
    }

    fn record(&mut self, e: ReplayEntry) {
        if self.cfg.record {
            self.log.push(e);
        }
    }

    /// Everything recorded so far (nothing unless `ServerConfig::record`), ending with the
    /// current state hash so a check covers every tick simulated.
    pub fn replay(&self) -> Replay {
        let mut entries = self.log.clone();
        let tick = self.world.tick();
        if self.cfg.record
            && !entries.last().is_some_and(|e| matches!(e, ReplayEntry::Hash { tick: t, .. } if *t == tick))
        {
            entries.push(ReplayEntry::Hash { tick, hash: self.world.state_hash() });
        }
        Replay { cfg: self.cfg.clone(), entries }
    }
}

/// What a replay check found.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayCheck {
    pub ticks: u32,
    pub hashes_checked: usize,
    /// The first recorded hash that didn't match: (tick, recorded, re-simulated).
    pub mismatch: Option<(Tick, u64, u64)>,
    pub final_hash: u64,
}

impl Replay {
    /// Re-simulate from scratch with the same driver and compare every recorded hash.
    pub fn verify(&self) -> ReplayCheck {
        let mut m = Match::new(self.cfg.clone());
        let mut check = ReplayCheck { ticks: 0, hashes_checked: 0, mismatch: None, final_hash: 0 };
        let mut i = 0;
        let last = self.entries.iter().map(|e| e.tick()).max().unwrap_or(Tick(0));
        while m.world.tick() < last {
            let k = m.world.tick().next();
            let mut due = Vec::new();
            // Joins and leaves recorded before tick k ran (stamped with an earlier tick), then
            // tick k's commands. A join stamped k happened after tick k ran.
            while let Some(e) = self.entries.get(i)
                && (e.tick() < k || matches!(e, ReplayEntry::Commands { tick, .. } if *tick == k))
            {
                match e {
                    ReplayEntry::Join { player, champion, .. } => {
                        m.join(*player, *champion);
                    }
                    ReplayEntry::Leave { player, .. } => m.leave(*player),
                    ReplayEntry::Restart { .. } => m.restart(),
                    ReplayEntry::Commands { commands, .. } => due = commands.clone(),
                    ReplayEntry::Hash { .. } => {}
                }
                i += 1;
            }
            m.step(due);
            check.ticks += 1;
            while let Some(ReplayEntry::Hash { tick, hash }) = self.entries.get(i)
                && *tick == k
            {
                let got = m.world.state_hash();
                check.hashes_checked += 1;
                if got != *hash && check.mismatch.is_none() {
                    check.mismatch = Some((*tick, *hash, got));
                }
                i += 1;
            }
        }
        check.final_hash = m.world.state_hash();
        check
    }

    /// A line-based text form (`mftr-replay 1`): one header line, then one line per entry.
    pub fn to_text(&self) -> String {
        let c = &self.cfg;
        let mut s = format!(
            "mftr-replay 1\nconfig {} {} {} {} {} {} {}\n",
            c.seed,
            c.max_players,
            c.bots,
            c.lobby as u8,
            c.arena_min,
            c.arena_max,
            c.scenario.name()
        );
        for e in &self.entries {
            match e {
                ReplayEntry::Join { tick, player, champion } => {
                    let name = champion.map_or("-", |c| c.def().name);
                    let _ = writeln!(s, "J {} {} {name}", tick.0, player.0);
                }
                ReplayEntry::Leave { tick, player } => {
                    let _ = writeln!(s, "L {} {}", tick.0, player.0);
                }
                ReplayEntry::Commands { tick, commands } => {
                    for cmd in commands {
                        let _ = writeln!(
                            s,
                            "C {} {} {} {} {} {}",
                            tick.0,
                            cmd.player.0,
                            cmd.seq,
                            cmd.tick.0,
                            cmd.sub.get(),
                            kind_text(cmd.kind)
                        );
                    }
                }
                ReplayEntry::Restart { tick } => {
                    let _ = writeln!(s, "R {}", tick.0);
                }
                ReplayEntry::Hash { tick, hash } => {
                    let _ = writeln!(s, "H {} {hash:016x}", tick.0);
                }
            }
        }
        s
    }

    pub fn from_text(text: &str) -> Result<Replay, String> {
        let mut lines = text.lines();
        if lines.next() != Some("mftr-replay 1") {
            return Err("not an mftr replay (version 1)".into());
        }
        let cfg_line = lines.next().ok_or("missing config")?;
        let f: Vec<&str> = cfg_line.split_whitespace().collect();
        if f.len() != 8 || f[0] != "config" {
            return Err(format!("bad config line: {cfg_line}"));
        }
        let num = |s: &str| s.parse::<f64>().map_err(|e| format!("{s}: {e}"));
        let cfg = ServerConfig {
            seed: f[1].parse().map_err(|e| format!("seed: {e}"))?,
            max_players: f[2].parse().map_err(|e| format!("max players: {e}"))?,
            bots: f[3].parse().map_err(|e| format!("bots: {e}"))?,
            lobby: f[4] == "1",
            record: true,
            arena_min: num(f[5])? as f32,
            arena_max: num(f[6])? as f32,
            scenario: Scenario::by_name(f[7]).ok_or(format!("scenario {}", f[7]))?,
        };
        let mut entries: Vec<ReplayEntry> = Vec::new();
        for (n, line) in lines.enumerate() {
            let bad = || format!("line {}: {line}", n + 3);
            let f: Vec<&str> = line.split_whitespace().collect();
            let int = |i: usize| f.get(i).and_then(|v| v.parse::<u64>().ok()).ok_or_else(bad);
            let tick = Tick(int(1)? as u32);
            match f.first().copied() {
                Some("J") => {
                    let champion = match f.get(3).copied() {
                        Some("-") => None,
                        Some(name) => Some(ChampionId::by_name(name).ok_or_else(bad)?),
                        None => return Err(bad()),
                    };
                    entries.push(ReplayEntry::Join { tick, player: PlayerId(int(2)? as u8), champion });
                }
                Some("L") => entries.push(ReplayEntry::Leave { tick, player: PlayerId(int(2)? as u8) }),
                Some("R") => entries.push(ReplayEntry::Restart { tick }),
                Some("H") => {
                    let hash = f.get(2).and_then(|h| u64::from_str_radix(h, 16).ok()).ok_or_else(bad)?;
                    entries.push(ReplayEntry::Hash { tick, hash });
                }
                Some("C") => {
                    let cmd = Command {
                        player: PlayerId(int(2)? as u8),
                        seq: int(3)? as u32,
                        tick: Tick(int(4)? as u32),
                        sub: SubTick::new(int(5)? as u8),
                        kind: parse_kind(&f[6..]).ok_or_else(bad)?,
                    };
                    match entries.last_mut() {
                        Some(ReplayEntry::Commands { tick: t, commands }) if *t == tick => commands.push(cmd),
                        _ => entries.push(ReplayEntry::Commands { tick, commands: vec![cmd] }),
                    }
                }
                _ => return Err(bad()),
            }
        }
        Ok(Replay { cfg, entries })
    }
}

impl ReplayEntry {
    pub fn tick(&self) -> Tick {
        match self {
            ReplayEntry::Join { tick, .. }
            | ReplayEntry::Leave { tick, .. }
            | ReplayEntry::Restart { tick }
            | ReplayEntry::Commands { tick, .. }
            | ReplayEntry::Hash { tick, .. } => *tick,
        }
    }
}

fn kind_text(k: CommandKind) -> String {
    match k {
        CommandKind::MoveTo(q) => format!("move {} {}", q.x, q.y),
        CommandKind::Stop => "stop".into(),
        CommandKind::Cast { slot, target } => format!("cast {slot} {} {}", target.x, target.y),
        CommandKind::Attack(id) => format!("attack {}", id.0),
        CommandKind::AttackMove(q) => format!("amove {} {}", q.x, q.y),
        CommandKind::LevelUp(slot) => format!("level {slot}"),
        CommandKind::Buy(item) => format!("buy {item}"),
        CommandKind::Sell(slot) => format!("sell {slot}"),
        CommandKind::Undo => "undo".into(),
        CommandKind::PickAugment(choice) => format!("augment {choice}"),
        CommandKind::RerollAugment(choice) => format!("reroll {choice}"),
        CommandKind::UseItem(slot) => format!("use {slot}"),
        CommandKind::BuyAnvil => "anvil".into(),
        CommandKind::PickAnvil(choice) => format!("pickanvil {choice}"),
    }
}

fn parse_kind(f: &[&str]) -> Option<CommandKind> {
    let n = |i: usize| f.get(i).and_then(|v| v.parse::<u32>().ok());
    let q = |i: usize| Some(QPoint { x: n(i)? as u16, y: n(i + 1)? as u16 });
    Some(match *f.first()? {
        "move" => CommandKind::MoveTo(q(1)?),
        "stop" => CommandKind::Stop,
        "cast" => CommandKind::Cast { slot: n(1)? as u8, target: q(2)? },
        "attack" => CommandKind::Attack(UnitId(n(1)?)),
        "amove" => CommandKind::AttackMove(q(1)?),
        "level" => CommandKind::LevelUp(n(1)? as u8),
        "buy" => CommandKind::Buy(n(1)? as u8),
        "sell" => CommandKind::Sell(n(1)? as u8),
        "undo" => CommandKind::Undo,
        "augment" => CommandKind::PickAugment(n(1)? as u8),
        "reroll" => CommandKind::RerollAugment(n(1).unwrap_or(0) as u8),
        "use" => CommandKind::UseItem(n(1)? as u8),
        "anvil" => CommandKind::BuyAnvil,
        "pickanvil" => CommandKind::PickAnvil(n(1)? as u8),
        _ => return None,
    })
}

/// Populate the scenario's map: rig turrets, minion clumps and waves, or an ARAM match.
fn populate(world: &mut World, scenario: Scenario) {
    if scenario == Scenario::DodgeRig {
        // Four turrets around the middle; together they cover most of the arena.
        for pos in
            [Vec2::new(900.0, 2000.0), Vec2::new(3100.0, 2000.0), Vec2::new(2000.0, 900.0), Vec2::new(2000.0, 3100.0)]
        {
            world.spawn_rig_turret(Team::Red, pos, 1100);
        }
        return;
    }
    if scenario.is_aram() {
        world.start_match();
        return;
    }
    if scenario == Scenario::Duel {
        for (center, team) in [(Vec2::new(1700.0, 2050.0), Team::Red), (Vec2::new(2300.0, 2150.0), Team::Blue)] {
            clump(world, center, 1, team);
        }
        return;
    }
    if scenario != Scenario::MinionSandbox {
        return;
    }
    // Static clumps.
    for (center, rings, team) in [
        (Vec2::new(1200.0, 1200.0), 1i32, Team::Red),
        (Vec2::new(2800.0, 1300.0), 2, Team::Blue),
        (Vec2::new(1300.0, 2800.0), 2, Team::Red),
        (Vec2::new(2900.0, 2900.0), 1, Team::Blue),
    ] {
        clump(world, center, rings, team);
    }
    // Two patrolling waves (2 rows of 3) crossing the arena through the middle.
    for (a, b, team) in [
        (Vec2::new(500.0, 2000.0), Vec2::new(3500.0, 2000.0), Team::Blue),
        (Vec2::new(2000.0, 500.0), Vec2::new(2000.0, 3500.0), Team::Red),
    ] {
        let along = (b - a).normalize_or_zero();
        let across = Vec2::new(-along.y, along.x);
        for i in 0..6 {
            let offset = along * (-60.0 * (i % 3) as f32) + across * (if i < 3 { -30.0 } else { 30.0 });
            let pa = QPoint::from_vec2(a + offset);
            let pb = QPoint::from_vec2(b + offset);
            let kind = if i < 3 { MinionKind::Melee } else { MinionKind::Caster };
            world.spawn_minion(kind, team, pa.to_vec2(), Some(Brain::Patrol { a: pa, b: pb, toward_b: true }));
        }
    }
}

/// A hex-packed minion clump, adjacent minions touching, like a wave fighting in lane.
fn clump(world: &mut World, center: Vec2, rings: i32, team: Team) {
    for q in -rings..=rings {
        for r in -rings..=rings {
            if (q + r).abs() > rings {
                continue;
            }
            let x = center.x + 50.0 * (q as f32 + r as f32 * 0.5);
            let y = center.y + 50.0 * 0.866_025_4 * r as f32;
            let kind = if (q + r) % 2 == 0 { MinionKind::Melee } else { MinionKind::Caster };
            world.spawn_minion(kind, team, Vec2::new(x, y), None);
        }
    }
}

/// Duel spawn points (west for blue, east for red), spread a little per player.
fn duel_spawn(team: Team, player: PlayerId) -> Vec2 {
    let y = 2000.0 + 150.0 * ((player.0 / 2) as f32) * if player.0 % 4 < 2 { 1.0 } else { -1.0 };
    Vec2::new(if team == Team::Blue { 500.0 } else { 3500.0 }, y)
}
