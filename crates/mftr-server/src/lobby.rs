//! Champion select (M2 slice 5, 06 §2; 13 §3).
//!
//! ARAM modes are all random: every player gets a random champion no teammate has. A reroll
//! trades it for another random one the team doesn't have (on the field or on the bench); the
//! old one goes to the team bench, where any teammate can take it (swapping their own onto the
//! bench). Classic is a pick: each human starts with a random champion and may take any other
//! no human teammate has (a bot teammate holding it gets theirs in exchange). The match starts
//! when every human is ready, or when the countdown runs out. Bots fill the remaining slots.

use mftr_bots::Tier;
use mftr_net::msg::{LobbyAction, LobbySlot, LobbyState};
use mftr_sim::rng::Pcg32;
use mftr_sim::{ChampionId, PlayerId, Team};

/// Seconds of champion select once the first human is in.
pub const LOBBY_SECONDS: f64 = 60.0;
/// Once every human is ready, the match starts this soon.
pub const READY_SECONDS: f64 = 3.0;
/// Rerolls each player starts with.
pub const REROLLS: u8 = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Slot {
    pub player: PlayerId,
    pub champion: ChampionId,
    pub rerolls: u8,
    pub ready: bool,
    pub bot: bool,
    /// A human's vote for the bots' tier (co-op, 17 §9; `Tier::wire`).
    pub vote: Option<u8>,
}

impl Slot {
    pub fn team(&self) -> Team {
        team_of(self.player)
    }
}

/// Teams alternate by player id, as in the match itself.
pub fn team_of(player: PlayerId) -> Team {
    if player.0.is_multiple_of(2) { Team::Blue } else { Team::Red }
}

pub struct Lobby {
    slots: Vec<Slot>,
    bench: [Vec<ChampionId>; 2],
    rng: Pcg32,
    max_players: u8,
    /// A pick instead of all random (Classic).
    pick: bool,
    /// Horde has five blue seats and no opposing champion team.
    blue_only: bool,
    /// When the match starts (wall clock); set when the first human joins.
    starts_at: Option<f64>,
    /// Co-op: the bots' tier is voted (17 §9), starting from this one.
    bot_vote: Option<Tier>,
}

impl Lobby {
    pub fn new(seed: u64, max_players: u8, bots: u8, pick: bool) -> Self {
        let mut lobby = Self {
            slots: Vec::new(),
            bench: [Vec::new(), Vec::new()],
            rng: Pcg32::new(seed, 0x6c6f_6262),
            max_players,
            pick,
            blue_only: false,
            starts_at: None,
            bot_vote: None,
        };
        for _ in 0..bots.min(max_players) {
            lobby.add(true);
        }
        lobby
    }

    pub fn blue_only(seed: u64, max_players: u8, bots: u8, pick: bool) -> Self {
        let mut lobby = Self::new(seed, max_players, 0, pick);
        lobby.blue_only = true;
        for _ in 0..bots.min(max_players) {
            lobby.add_to(Team::Blue, true);
        }
        lobby
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    /// Let humans vote for the bots' tier, starting from `default` (co-op).
    pub fn with_bot_vote(mut self, default: Tier) -> Self {
        self.bot_vote = Some(default);
        self
    }

    /// The bots' tier: the most-voted (a tie goes to the easier), else the default; `None`
    /// where it isn't voted.
    pub fn bot_tier(&self) -> Option<Tier> {
        let default = self.bot_vote?;
        let votes = self.votes();
        let most = *votes.iter().max().unwrap_or(&0);
        if most == 0 {
            return Some(default);
        }
        let i = votes.iter().position(|v| *v == most).unwrap_or(1);
        Tier::from_wire(i as u8).or(Some(default))
    }

    /// Humans' votes for each tier (Recruit to Nightmare).
    fn votes(&self) -> [u8; 4] {
        let mut v = [0u8; 4];
        for s in self.slots.iter().filter(|s| !s.bot) {
            if let Some(t) = s.vote {
                v[(t as usize).min(3)] += 1;
            }
        }
        v
    }

    /// Add a human (13 §2) on `team_for_human`'s team, taking a bot's place when the team is
    /// full. Returns its id.
    pub fn add_human(&mut self, now: f64, coop: bool) -> Option<PlayerId> {
        let team = self.team_for_human(coop)?;
        if self.on_team(team) >= self.team_size(team) {
            // Humans take bots' places (the bot's champion goes back to the pool, not the
            // bench: nobody rolled it).
            let bot = self.slots.iter().rposition(|s| s.bot && s.team() == team)?;
            self.slots.remove(bot);
        }
        let player = self.add_to(team, false)?;
        self.starts_at.get_or_insert(now + LOBBY_SECONDS);
        Some(player)
    }

    /// The team a new human would join, if either has room (a free slot or a bot to
    /// replace): the one with fewer humans; on a tie, one with a free slot, then blue. In
    /// co-op, humans play blue only.
    pub fn team_for_human(&self, coop: bool) -> Option<Team> {
        let humans = |t: Team| self.slots.iter().filter(|s| !s.bot && s.team() == t).count();
        let free = |t: Team| self.on_team(t) < self.team_size(t);
        let room = |t: Team| free(t) || self.slots.iter().any(|s| s.bot && s.team() == t);
        let mut teams = if coop { vec![Team::Blue] } else { vec![Team::Blue, Team::Red] };
        teams.sort_by_key(|t| (humans(*t), !free(*t)));
        teams.into_iter().find(|t| room(*t))
    }

    /// Humans in champion select.
    pub fn humans(&self) -> usize {
        self.slots.iter().filter(|s| !s.bot).count()
    }

    fn on_team(&self, team: Team) -> usize {
        self.slots.iter().filter(|s| s.team() == team).count()
    }

    /// Slots per team: blue gets the odd one out.
    fn team_size(&self, team: Team) -> usize {
        let n = self.max_players as usize;
        if self.blue_only {
            return if team == Team::Blue { n } else { 0 };
        }
        if team == Team::Blue { n.div_ceil(2) } else { n / 2 }
    }

    fn add(&mut self, bot: bool) -> Option<PlayerId> {
        if self.blue_only {
            return self.add_to(Team::Blue, bot);
        }
        // Fill the smaller team first.
        let blue = self.on_team(Team::Blue);
        let red = self.slots.len() - blue;
        self.add_to(if blue <= red { Team::Blue } else { Team::Red }, bot)
    }

    /// A new slot on `team`: the lowest free id of its parity.
    fn add_to(&mut self, team: Team, bot: bool) -> Option<PlayerId> {
        let used: Vec<u8> = self.slots.iter().map(|s| s.player.0).collect();
        let parity = if team == Team::Blue { 0 } else { 1 };
        let player = (0..=u8::MAX).filter(|p| p % 2 == parity).find(|p| !used.contains(p)).map(PlayerId)?;
        let champion = self.draw(team_of(player))?;
        let rerolls = if bot || self.pick { 0 } else { REROLLS };
        self.slots.push(Slot { player, champion, rerolls, ready: bot, bot, vote: None });
        self.slots.sort_by_key(|s| s.player);
        Some(player)
    }

    /// A random champion `team` has neither in play nor on its bench (bench champions are
    /// taken back first if nothing else is left).
    fn draw(&mut self, team: Team) -> Option<ChampionId> {
        let taken = |c: &ChampionId, lobby: &Lobby| {
            lobby.slots.iter().any(|s| s.team() == team && s.champion == *c) || lobby.bench[team as usize].contains(c)
        };
        let free: Vec<ChampionId> = ChampionId::ALL.into_iter().filter(|c| !taken(c, self)).collect();
        if !free.is_empty() {
            return Some(free[self.rng.next_u32() as usize % free.len()]);
        }
        let bench = &mut self.bench[team as usize];
        if bench.is_empty() {
            // More players than champions on one team: duplicates are unavoidable.
            return Some(ChampionId::ALL[self.rng.next_u32() as usize % ChampionId::ALL.len()]);
        }
        Some(bench.remove(0))
    }

    /// A human left champion select: their champion goes to the bench and a bot takes the
    /// slot.
    pub fn remove(&mut self, player: PlayerId) {
        if let Some(i) = self.slots.iter().position(|s| s.player == player) {
            let s = self.slots.remove(i);
            self.bench[s.team() as usize].push(s.champion);
            if !s.bot {
                self.add_to(s.team(), true);
            }
        }
    }

    pub fn act(&mut self, player: PlayerId, action: LobbyAction) {
        let Some(i) = self.slots.iter().position(|s| s.player == player && !s.bot) else { return };
        let team = self.slots[i].team() as usize;
        match action {
            LobbyAction::Ready(r) => self.slots[i].ready = r,
            LobbyAction::Bots(t) => {
                if self.bot_vote.is_some() {
                    self.slots[i].vote = Some(t.min(3));
                }
            }
            LobbyAction::Reroll => {
                if self.slots[i].rerolls == 0 {
                    return;
                }
                let old = self.slots[i].champion;
                // Draw before benching the old one, so a reroll never gives it back.
                let free: Vec<ChampionId> = ChampionId::ALL
                    .into_iter()
                    .filter(|c| {
                        !self.slots.iter().any(|s| s.team() as usize == team && s.champion == *c)
                            && !self.bench[team].contains(c)
                    })
                    .collect();
                if free.is_empty() {
                    return;
                }
                let new = free[self.rng.next_u32() as usize % free.len()];
                self.slots[i].champion = new;
                self.slots[i].rerolls -= 1;
                self.bench[team].push(old);
            }
            // A pick: any champion no human teammate has; a bot teammate trades.
            LobbyAction::Take(c) if self.pick => {
                let old = self.slots[i].champion;
                let holder = self.slots.iter().position(|s| s.team() as usize == team && s.champion == c);
                match holder {
                    Some(h) if !self.slots[h].bot => {}
                    Some(h) => {
                        self.slots[h].champion = old;
                        self.slots[i].champion = c;
                    }
                    None => self.slots[i].champion = c,
                }
            }
            LobbyAction::Take(c) => {
                let Some(b) = self.bench[team].iter().position(|x| *x == c) else { return };
                let old = self.slots[i].champion;
                self.bench[team][b] = old;
                self.slots[i].champion = c;
            }
        }
    }

    /// Whether the match should start now: the countdown ran out, or every human is ready
    /// (then after a short delay).
    pub fn starts_in(&mut self, now: f64) -> Option<f64> {
        let at = self.starts_at?;
        let humans: Vec<&Slot> = self.slots.iter().filter(|s| !s.bot).collect();
        let at = if !humans.is_empty() && humans.iter().all(|s| s.ready) { at.min(now + READY_SECONDS) } else { at };
        self.starts_at = Some(at);
        Some((at - now).max(0.0))
    }

    /// What `you` sees: every slot, and its own team's bench.
    pub fn state_for(&self, you: PlayerId, starts_in: f64) -> LobbyState {
        LobbyState {
            you,
            slots: self
                .slots
                .iter()
                .map(|s| LobbySlot {
                    player: s.player,
                    team: s.team(),
                    champion: s.champion,
                    rerolls: s.rerolls,
                    ready: s.ready,
                    bot: s.bot,
                })
                .collect(),
            bench: self.bench[team_of(you) as usize].clone(),
            starts_in_ms: (starts_in * 1000.0) as u32,
            // The server fills in what's played and where.
            mode: mftr_net::msg::GameMode::Empty,
            map: String::new(),
            pick: self.pick,
            midgame: false,
            bots: self.bot_tier().map(|tier| mftr_net::msg::BotsChoice {
                tier: tier.wire(),
                votes: self.votes(),
                mine: self.slots.iter().find(|s| s.player == you).and_then(|s| s.vote),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_random_with_rerolls_and_a_bench() {
        let mut l = Lobby::new(5, 10, 4, false);
        assert_eq!(l.slots().len(), 4);
        let me = l.add_human(0.0, false).unwrap();
        let other = l.add_human(0.0, false).unwrap();
        let mate = l.add_human(0.0, false).unwrap();
        assert_ne!(team_of(me), team_of(other), "teams are kept even");
        assert_eq!(team_of(me), team_of(mate));
        // No two teammates share a champion.
        for team in [Team::Blue, Team::Red] {
            let mut champs: Vec<ChampionId> =
                l.slots().iter().filter(|s| s.team() == team).map(|s| s.champion).collect();
            let n = champs.len();
            champs.sort_by_key(|c| *c as u8);
            champs.dedup();
            assert_eq!(champs.len(), n, "{team:?}");
        }
        let slot = |l: &Lobby, p| *l.slots().iter().find(|s| s.player == p).unwrap();
        let before = slot(&l, me).champion;
        l.act(me, LobbyAction::Reroll);
        let after = slot(&l, me);
        assert_ne!(after.champion, before);
        assert_eq!(after.rerolls, REROLLS - 1);
        assert_eq!(l.state_for(mate, 0.0).bench, vec![before], "the teammate sees it on the bench");
        // The teammate takes it; their old champion goes to the bench.
        let theirs = slot(&l, mate).champion;
        l.act(mate, LobbyAction::Take(before));
        assert_eq!(slot(&l, mate).champion, before);
        assert_eq!(l.state_for(me, 0.0).bench, vec![theirs]);
        // Out of rerolls: nothing happens.
        l.act(me, LobbyAction::Reroll);
        l.act(me, LobbyAction::Reroll);
        assert_eq!(slot(&l, me).rerolls, 0);
        let fixed = slot(&l, me).champion;
        l.act(me, LobbyAction::Reroll);
        assert_eq!(slot(&l, me).champion, fixed);
    }

    #[test]
    fn countdown_and_ready() {
        let mut l = Lobby::new(1, 10, 9, false);
        assert_eq!(l.starts_in(0.0), None, "nothing starts without a human");
        let me = l.add_human(10.0, false).unwrap();
        assert_eq!(l.slots().len(), 10, "the human took a bot's place");
        assert_eq!(l.starts_in(20.0), Some(50.0));
        l.act(me, LobbyAction::Ready(true));
        assert_eq!(l.starts_in(20.0), Some(3.0));
        assert_eq!(l.starts_in(23.5), Some(0.0));
    }

    /// 13 §3: Classic is a pick: no rerolls; take any champion no human teammate has (a bot
    /// teammate holding it gets ours); one a human teammate holds stays theirs.
    #[test]
    fn classic_picks() {
        let mut l = Lobby::new(8, 10, 10, true);
        let me = l.add_human(0.0, false).unwrap();
        let mate = l.add_human(0.0, false).unwrap();
        let mate = if team_of(mate) == team_of(me) { mate } else { l.add_human(0.0, false).unwrap() };
        assert_eq!(team_of(mate), team_of(me));
        let slot = |l: &Lobby, p| *l.slots().iter().find(|s| s.player == p).unwrap();
        assert_eq!(slot(&l, me).rerolls, 0);
        let mine = slot(&l, me).champion;
        let theirs = slot(&l, mate).champion;
        l.act(me, LobbyAction::Take(theirs));
        assert_eq!(slot(&l, me).champion, mine, "a human teammate's champion stays theirs");
        let bot = *l.slots().iter().find(|s| s.bot && s.team() == team_of(me)).unwrap();
        l.act(me, LobbyAction::Take(bot.champion));
        assert_eq!(slot(&l, me).champion, bot.champion);
        assert_eq!(slot(&l, bot.player).champion, mine, "the bot took ours");
        let free = ChampionId::ALL
            .into_iter()
            .find(|c| !l.slots().iter().any(|s| s.team() == team_of(me) && s.champion == *c))
            .unwrap();
        l.act(me, LobbyAction::Take(free));
        assert_eq!(slot(&l, me).champion, free);
        assert!(l.state_for(me, 0.0).pick);
    }

    /// 13 §2: humans go to the team with fewer humans (blue on a tie), taking bots' places;
    /// in co-op they all play blue, and a full blue has no room. A human who leaves leaves a
    /// bot behind.
    #[test]
    fn humans_balance_or_play_together() {
        let mut l = Lobby::new(2, 10, 10, false);
        let teams: Vec<Team> = (0..4).map(|_| team_of(l.add_human(0.0, false).unwrap())).collect();
        assert_eq!(teams, [Team::Blue, Team::Red, Team::Blue, Team::Red]);
        assert_eq!(l.slots().len(), 10);
        assert_eq!(l.humans(), 4);

        let mut l = Lobby::new(3, 10, 10, false);
        for _ in 0..5 {
            assert_eq!(team_of(l.add_human(0.0, true).unwrap()), Team::Blue);
        }
        assert_eq!(l.team_for_human(true), None, "blue is all human");
        assert_eq!(l.add_human(0.0, true), None);
        assert_eq!(l.team_for_human(false), Some(Team::Red), "versus would still fit on red");
        let gone = l.slots().iter().find(|s| !s.bot).unwrap().player;
        l.remove(gone);
        assert_eq!(l.slots().len(), 10, "a bot took the slot");
        assert_eq!(l.humans(), 4);
        assert_eq!(l.team_for_human(true), Some(Team::Blue));
    }
}
