//! Champion select for ARAM (M2 slice 5, 06 §2): all-random with rerolls and a team bench.
//!
//! Every player gets a random champion no teammate has. A reroll trades it for another random
//! one the team doesn't have (on the field or on the bench); the old one goes to the team bench,
//! where any teammate can take it (swapping their own onto the bench). The match starts when
//! every human is ready, or when the countdown runs out. Bots fill the remaining slots.

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
}

impl Slot {
    pub fn team(&self) -> Team {
        team_of(self.player)
    }
}

/// Teams alternate by player id, as in the match itself.
pub fn team_of(player: PlayerId) -> Team {
    if player.0 % 2 == 0 { Team::Blue } else { Team::Red }
}

pub struct Lobby {
    slots: Vec<Slot>,
    bench: [Vec<ChampionId>; 2],
    rng: Pcg32,
    max_players: u8,
    /// When the match starts (wall clock); set when the first human joins.
    starts_at: Option<f64>,
}

impl Lobby {
    pub fn new(seed: u64, max_players: u8, bots: u8) -> Self {
        let mut lobby = Self {
            slots: Vec::new(),
            bench: [Vec::new(), Vec::new()],
            rng: Pcg32::new(seed, 0x6c6f_6262),
            max_players,
            starts_at: None,
        };
        for _ in 0..bots.min(max_players) {
            lobby.add(true);
        }
        lobby
    }

    pub fn slots(&self) -> &[Slot] {
        &self.slots
    }

    /// Add a player (a human replaces a bot when the lobby is full). Returns its id.
    pub fn add_human(&mut self, now: f64) -> Option<PlayerId> {
        if self.slots.len() >= self.max_players as usize {
            // Humans take bots' places (the bot's champion goes back to the pool, not the
            // bench: nobody rolled it).
            let bot = self.slots.iter().rposition(|s| s.bot)?;
            self.slots.remove(bot);
        }
        let player = self.add(false)?;
        self.starts_at.get_or_insert(now + LOBBY_SECONDS);
        Some(player)
    }

    fn add(&mut self, bot: bool) -> Option<PlayerId> {
        let used: Vec<u8> = self.slots.iter().map(|s| s.player.0).collect();
        // Fill the smaller team first: the lowest free id of that parity.
        let blue = self.slots.iter().filter(|s| s.team() == Team::Blue).count();
        let red = self.slots.len() - blue;
        let parity = if blue <= red { 0 } else { 1 };
        let player = (0..=u8::MAX).filter(|p| p % 2 == parity).find(|p| !used.contains(p)).map(PlayerId)?;
        let champion = self.draw(team_of(player))?;
        self.slots.push(Slot { player, champion, rerolls: if bot { 0 } else { REROLLS }, ready: bot, bot });
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

    pub fn remove(&mut self, player: PlayerId) {
        if let Some(i) = self.slots.iter().position(|s| s.player == player) {
            let s = self.slots.remove(i);
            self.bench[s.team() as usize].push(s.champion);
        }
    }

    pub fn act(&mut self, player: PlayerId, action: LobbyAction) {
        let Some(i) = self.slots.iter().position(|s| s.player == player && !s.bot) else { return };
        let team = self.slots[i].team() as usize;
        match action {
            LobbyAction::Ready(r) => self.slots[i].ready = r,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_random_with_rerolls_and_a_bench() {
        let mut l = Lobby::new(5, 10, 4);
        assert_eq!(l.slots().len(), 4);
        let me = l.add_human(0.0).unwrap();
        let other = l.add_human(0.0).unwrap();
        let mate = l.add_human(0.0).unwrap();
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
        let mut l = Lobby::new(1, 10, 9);
        assert_eq!(l.starts_in(0.0), None, "nothing starts without a human");
        let me = l.add_human(10.0).unwrap();
        assert_eq!(l.slots().len(), 10, "the human took a bot's place");
        assert_eq!(l.starts_in(20.0), Some(50.0));
        l.act(me, LobbyAction::Ready(true));
        assert_eq!(l.starts_in(20.0), Some(3.0));
        assert_eq!(l.starts_in(23.5), Some(0.0));
    }
}
