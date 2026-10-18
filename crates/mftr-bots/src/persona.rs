//! Personas (17 §13): the kind of player a bot plays as, rolled at the start of a match and kept
//! all game, so a lobby of bots has the variety of a lobby of people. A tier is how well a bot
//! plays (its hands and its reads); a persona is how it likes to play: how much risk it takes,
//! whether it plays for itself or for its team, how much it wards, and how much it plays
//! around brush and baits. Personas never change what a bot can do, only what it chooses.
//!
//! Each persona is a point in four traits, jittered a little per bot:
//! - **nerve** (−1 to 1): risk appetite. Fights at worse odds, flees later, chases longer,
//!   dives turrets for kills; a timid bot does the opposite.
//! - **greed** (−1 to 1): selfish or selfless. Takes kills and relics for itself; a selfless
//!   bot leaves the relic to a hurt teammate and peels for its carries.
//! - **vision** (0 to 1): how much it wards (control wards, trinket use).
//! - **flair** (0 to 1): how much it plays around brush: waits in it by the fight, and comes
//!   out of it to start one.

use mftr_sim::ChampionId;
use mftr_sim::rng::Pcg32;

use crate::kit::Archetype;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// Starts fights, dives, chases: the playmaker who sometimes overreaches.
    Daredevil,
    /// Plays for its own gold and kills: farms, finishes, takes the relic.
    Carry,
    /// Plays for its team: peels, leaves the relic to whoever needs it, wards.
    Guardian,
    /// Waits for a clear edge and backs off early.
    Cautious,
    /// Lurks in brush, baits and pokes, comes from an angle.
    Trickster,
    /// The even-keeled team player.
    Steady,
}

impl Kind {
    pub const ALL: [Kind; 6] =
        [Kind::Daredevil, Kind::Carry, Kind::Guardian, Kind::Cautious, Kind::Trickster, Kind::Steady];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Daredevil => "daredevil",
            Kind::Carry => "carry",
            Kind::Guardian => "guardian",
            Kind::Cautious => "cautious",
            Kind::Trickster => "trickster",
            Kind::Steady => "steady",
        }
    }

    pub fn by_name(s: &str) -> Option<Kind> {
        Kind::ALL.into_iter().find(|k| k.name().eq_ignore_ascii_case(s.trim()))
    }

    /// The persona's traits: nerve, greed, vision, flair.
    fn traits(self) -> [f32; 4] {
        match self {
            Kind::Daredevil => [0.8, 0.2, 0.2, 0.5],
            Kind::Carry => [0.25, 0.8, 0.2, 0.3],
            Kind::Guardian => [0.05, -0.8, 0.9, 0.2],
            Kind::Cautious => [-0.6, 0.0, 0.6, 0.1],
            Kind::Trickster => [0.4, 0.1, 0.4, 0.9],
            Kind::Steady => [0.1, 0.0, 0.4, 0.3],
        }
    }

    /// How often a champion of `a`'s class is played this way (relative weights): anyone can
    /// be anything, but tanks lean to starting fights and enchanters to their team.
    fn weight(self, a: Archetype) -> f32 {
        let base = match self {
            Kind::Daredevil => 18.0,
            Kind::Carry => 20.0,
            Kind::Guardian => 14.0,
            Kind::Cautious => 12.0,
            Kind::Trickster => 15.0,
            Kind::Steady => 21.0,
        };
        let fit = match (self, a) {
            (Kind::Guardian, a) if a.support() => 3.0,
            (Kind::Carry, a) if a.support() => 0.3,
            (Kind::Daredevil, a) if a.front() => 1.8,
            (Kind::Carry, Archetype::Marksman | Archetype::Mage | Archetype::Assassin) => 1.6,
            (Kind::Trickster, Archetype::Assassin | Archetype::Skirmisher | Archetype::Artillery) => 1.8,
            (Kind::Cautious, a) if a.back() => 1.3,
            _ => 1.0,
        };
        base * fit
    }
}

/// A bot's persona: its kind and its traits (17 §13).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Persona {
    pub kind: Kind,
    pub nerve: f32,
    pub greed: f32,
    pub vision: f32,
    pub flair: f32,
}

impl Persona {
    /// The even-keeled baseline every trait is measured from.
    pub const STEADY: Persona = Persona { kind: Kind::Steady, nerve: 0.0, greed: 0.0, vision: 0.4, flair: 0.0 };

    pub fn of(kind: Kind) -> Persona {
        let [nerve, greed, vision, flair] = kind.traits();
        Persona { kind, nerve, greed, vision, flair }
    }

    /// Roll a persona for a bot playing `champion`, from the match's random stream. A
    /// development override (`MFTR_BOTS_PERSONA=daredevil`) makes every bot one kind, and
    /// `MFTR_BOTS_OFF=personas` makes them all the baseline.
    pub fn roll(rng: &mut Pcg32, champion: Option<ChampionId>) -> Persona {
        if crate::tier::off("personas") {
            return Persona::STEADY;
        }
        if let Some(kind) = forced() {
            return Persona::of(kind);
        }
        let class = champion.map(Archetype::of).unwrap_or(Archetype::Bruiser);
        let total: f32 = Kind::ALL.iter().map(|k| k.weight(class)).sum();
        let mut pick = rng.next_f32() * total;
        let mut kind = Kind::Steady;
        for k in Kind::ALL {
            pick -= k.weight(class);
            if pick <= 0.0 {
                kind = k;
                break;
            }
        }
        // No two players of a kind are quite alike.
        let mut p = Persona::of(kind);
        let mut jitter = |v: f32, lo: f32, hi: f32| (v + (rng.next_f32() * 2.0 - 1.0) * 0.15).clamp(lo, hi);
        p.nerve = jitter(p.nerve, -1.0, 1.0);
        p.greed = jitter(p.greed, -1.0, 1.0);
        p.vision = jitter(p.vision, 0.0, 1.0);
        p.flair = jitter(p.flair, 0.0, 1.0);
        p
    }

    pub fn name(&self) -> &'static str {
        self.kind.name()
    }
}

/// `MFTR_BOTS_PERSONA`: every bot one kind (development, for A/B runs). Read once.
fn forced() -> Option<Kind> {
    static FORCED: std::sync::OnceLock<Option<Kind>> = std::sync::OnceLock::new();
    *FORCED.get_or_init(|| std::env::var("MFTR_BOTS_PERSONA").ok().and_then(|v| Kind::by_name(&v)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolls_are_varied_reproducible_and_lean_by_class() {
        let roll_many = |champion: ChampionId, seed: u64| {
            let mut rng = Pcg32::new(seed, 7);
            (0..600).map(|_| Persona::roll(&mut rng, Some(champion))).collect::<Vec<_>>()
        };
        let a = roll_many(ChampionId::Lumen, 1);
        assert_eq!(a, roll_many(ChampionId::Lumen, 1), "the same seed rolls the same personas");
        for k in Kind::ALL {
            assert!(a.iter().any(|p| p.kind == k), "{k:?} comes up");
        }
        // An enchanter is more often a guardian than a marksman is.
        let share = |v: &[Persona], k: Kind| v.iter().filter(|p| p.kind == k).count() as f32 / v.len() as f32;
        let b = roll_many(ChampionId::Vesper, 1);
        assert!(share(&a, Kind::Guardian) > 2.0 * share(&b, Kind::Guardian));
        assert!(share(&b, Kind::Carry) > share(&a, Kind::Carry));
        for p in a.iter().chain(&b) {
            assert!((-1.0..=1.0).contains(&p.nerve) && (-1.0..=1.0).contains(&p.greed));
            assert!((0.0..=1.0).contains(&p.vision) && (0.0..=1.0).contains(&p.flair));
        }
        // The kinds really differ where it matters.
        assert!(Persona::of(Kind::Daredevil).nerve > Persona::of(Kind::Cautious).nerve + 1.0);
        assert!(Persona::of(Kind::Carry).greed > Persona::of(Kind::Guardian).greed + 1.0);
        assert!(Persona::of(Kind::Trickster).flair > 0.8);
    }
}
