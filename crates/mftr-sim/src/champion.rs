//! Champions as data (CR1, D8, D73): stats, a basic attack and four abilities, read from
//! champion files ([`crate::data`]). The official ten are `champions/<id>.ron`, embedded at
//! build time; the champion creator's recipes carry the same kit (20 §11).
//!
//! A [`ChampionId`] is a handle into a process-wide **registry** of champion definitions. The
//! official ten always hold ids 0–9, in the order of [`ChampionId::ALL`], so the wire and
//! replays see the same numbers as before. Any other definition is added with
//! [`ChampionId::register`], which returns the same id for the same definition, so a recipe
//! read twice is one champion. Definitions are never removed: the registry is append-only and
//! capped ([`REGISTRY_CAPACITY`]).
//!
//! Stats are level-1 values that grow with level (02 §1); ability numbers are rank-1 values that
//! grow with rank (M2 slice 2). Sandboxes play everything at level 1, rank 1.

use std::fmt;
use std::sync::{Mutex, Once, OnceLock};

use crate::ability::{Ability, BARRIER, BLINK};
use crate::combat::stat_at_level;
use crate::data;
use crate::time::SimDuration;

/// A champion: an index into the registry. The official ten are named constants.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ChampionId(u16);

/// How many distinct champion definitions one process can hold.
pub const REGISTRY_CAPACITY: usize = 4096;

/// The official champions' files, in id order.
const OFFICIAL: [&str; 10] = [
    include_str!("../champions/ember.ron"),
    include_str!("../champions/vesper.ron"),
    include_str!("../champions/bastion.ron"),
    include_str!("../champions/rook.ron"),
    include_str!("../champions/lumen.ron"),
    include_str!("../champions/shade.ron"),
    include_str!("../champions/quill.ron"),
    include_str!("../champions/cairn.ron"),
    include_str!("../champions/marrow.ron"),
    include_str!("../champions/wren.ron"),
];

static SLOTS: [OnceLock<&'static ChampionDef>; REGISTRY_CAPACITY] = [const { OnceLock::new() }; REGISTRY_CAPACITY];
/// How many slots are taken; held while adding one.
static TAKEN: Mutex<usize> = Mutex::new(0);
static OFFICIAL_LOADED: Once = Once::new();

/// The official champions are read the first time any champion is looked up. They are part of
/// the build, so a broken file is a bug that tests catch, not something to recover from.
fn load_official() {
    OFFICIAL_LOADED.call_once(|| {
        for (i, text) in OFFICIAL.iter().enumerate() {
            let def = data::read_champion(text).unwrap_or_else(|e| panic!("official champion {i}: {e}"));
            let id = add(def).expect("the registry holds the official champions");
            assert_eq!(id.0 as usize, i, "official champions take ids 0–9");
        }
    });
}

/// Add `def`, or find it if it's there already.
fn add(def: ChampionDef) -> Option<ChampionId> {
    let mut taken = TAKEN.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(i) = (0..*taken).find(|&i| SLOTS[i].get().is_some_and(|d| **d == def)) {
        return Some(ChampionId(i as u16));
    }
    if *taken >= REGISTRY_CAPACITY {
        return None;
    }
    let i = *taken;
    let leaked: &'static ChampionDef = Box::leak(Box::new(def));
    let _ = SLOTS[i].set(leaked);
    *taken += 1;
    Some(ChampionId(i as u16))
}

/// The registry is full ([`REGISTRY_CAPACITY`] distinct champions).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RegistryFull;

#[allow(non_upper_case_globals)]
impl ChampionId {
    /// Skillshot mage.
    pub const Ember: ChampionId = ChampionId(0);
    /// Marksman.
    pub const Vesper: ChampionId = ChampionId(1);
    /// Tank / engage (melee).
    pub const Bastion: ChampionId = ChampionId(2);
    /// Bruiser (melee).
    pub const Rook: ChampionId = ChampionId(3);
    /// Enchanter.
    pub const Lumen: ChampionId = ChampionId(4);
    /// Assassin (melee).
    pub const Shade: ChampionId = ChampionId(5);
    /// Artillery mage.
    pub const Quill: ChampionId = ChampionId(6);
    /// Warden: tank and protector (melee).
    pub const Cairn: ChampionId = ChampionId(7);
    /// Battlemage: short-range sustained magic.
    pub const Marrow: ChampionId = ChampionId(8);
    /// Skirmisher: mobile ranged physical damage.
    pub const Wren: ChampionId = ChampionId(9);

    /// The official roster, in id order.
    pub const ALL: [ChampionId; 10] = [
        ChampionId::Ember,
        ChampionId::Vesper,
        ChampionId::Bastion,
        ChampionId::Rook,
        ChampionId::Lumen,
        ChampionId::Shade,
        ChampionId::Quill,
        ChampionId::Cairn,
        ChampionId::Marrow,
        ChampionId::Wren,
    ];

    /// The registry index.
    pub const fn index(self) -> u16 {
        self.0
    }

    /// An official champion from its wire number. Other champions don't travel as numbers:
    /// their ids differ from process to process (CR1 brings a match roster for them).
    pub fn from_u8(v: u8) -> Option<Self> {
        Self::ALL.get(v as usize).copied()
    }

    /// Whether this is one of the official ten.
    pub fn is_official(self) -> bool {
        (self.0 as usize) < Self::ALL.len()
    }

    /// The first registered champion with this name (official ones first), ignoring case.
    pub fn by_name(name: &str) -> Option<Self> {
        load_official();
        let taken = *TAKEN.lock().unwrap_or_else(|e| e.into_inner());
        (0..taken).map(|i| ChampionId(i as u16)).find(|c| c.def().name.eq_ignore_ascii_case(name))
    }

    /// Every registered champion, the official ten first.
    pub fn registered() -> Vec<ChampionId> {
        load_official();
        let taken = *TAKEN.lock().unwrap_or_else(|e| e.into_inner());
        (0..taken).map(|i| ChampionId(i as u16)).collect()
    }

    /// Add a champion definition (a recipe's kit) and return its id: the existing one if the
    /// same definition is registered already.
    pub fn register(def: ChampionDef) -> Result<ChampionId, RegistryFull> {
        load_official();
        add(def).ok_or(RegistryFull)
    }

    pub fn def(self) -> &'static ChampionDef {
        load_official();
        SLOTS
            .get(self.0 as usize)
            .and_then(|s| s.get())
            .unwrap_or_else(|| panic!("champion {} isn't registered", self.0))
    }

    /// Ability in `slot` (0–3 = Q W E R, 4 = D Blink, 5 = F Barrier).
    pub fn ability(self, slot: u8) -> Option<Ability> {
        match slot {
            0..=3 => Some(self.def().abilities[slot as usize]),
            4 => Some(BLINK),
            5 => Some(BARRIER),
            _ => None,
        }
    }
}

/// Prints the champion's name, as the old enum's variants did (logs, reports, test output).
impl fmt::Debug for ChampionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match SLOTS.get(self.0 as usize).and_then(|s| s.get()) {
            Some(d) => f.write_str(d.name),
            None if (self.0 as usize) < Self::ALL.len() => f.write_str(self.def().name),
            None => write!(f, "ChampionId({})", self.0),
        }
    }
}

/// Combat stats (02 §1), fixed per unit for now.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub max_health: f32,
    /// Health per second.
    pub health_regen: f32,
    pub armor: f32,
    pub magic_resist: f32,
    pub attack_damage: f32,
    pub ability_power: f32,
    pub move_speed: f32,
    /// Ability haste (items): cooldowns × 100 / (100 + haste) (02 §8).
    pub ability_haste: f32,
    /// Share of basic-attack damage dealt returned as health (items).
    pub life_steal: f32,
    /// Item stats (18 §3): chance a basic attack crits, and crit damage on top of 175%.
    pub crit_chance: f32,
    pub crit_damage: f32,
    /// Lethality (flat armor penetration by level, 02 §4) and percent armor penetration.
    pub lethality: f32,
    pub armor_pen: f32,
    /// Flat and percent magic penetration.
    pub magic_pen: f32,
    pub magic_pen_pct: f32,
    /// Share of all damage dealt returned as health.
    pub omnivamp: f32,
    /// Shortens stuns, roots and slows (02 §9); weakens slows.
    pub tenacity: f32,
    pub slow_resist: f32,
    /// Heals and shields given to allies, and those received, are worth this much more.
    pub heal_power: f32,
    pub heal_received: f32,
}

impl Stats {
    /// Proxies and other units without combat stats.
    pub const NONE: Stats = Stats {
        max_health: 0.0,
        health_regen: 0.0,
        armor: 0.0,
        magic_resist: 0.0,
        attack_damage: 0.0,
        ability_power: 0.0,
        move_speed: 0.0,
        ability_haste: 0.0,
        life_steal: 0.0,
        crit_chance: 0.0,
        crit_damage: 0.0,
        lethality: 0.0,
        armor_pen: 0.0,
        magic_pen: 0.0,
        magic_pen_pct: 0.0,
        omnivamp: 0.0,
        tenacity: 0.0,
        slow_resist: 0.0,
        heal_power: 0.0,
        heal_received: 0.0,
    };
}

/// A ranged basic attack (02 §7): windup, then a homing bolt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackSpec {
    /// Center to the target's edge: in range when `distance ≤ range + target gameplay radius`.
    pub range: f32,
    /// Attacks per second.
    pub attack_speed: f32,
    /// Part of the attack timer before the bolt launches.
    pub windup_fraction: f32,
    pub bolt_speed: f32,
}

impl AttackSpec {
    /// Attack timer `1 / AS`, exact on the sub-tick timeline.
    pub fn period(&self) -> SimDuration {
        SimDuration((1920.0 / self.attack_speed).round() as u64)
    }

    pub fn windup(&self) -> SimDuration {
        SimDuration((1920.0 * self.windup_fraction / self.attack_speed).round() as u64)
    }
}

/// What a champion is in a fight (01 §9's classes): the bots play it and shop for it by its
/// class (17 §2), and its file says which it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Class {
    Mage,
    Marksman,
    Tank,
    Bruiser,
    Enchanter,
    Assassin,
    Artillery,
    Warden,
    Battlemage,
    Skirmisher,
}

impl Class {
    pub const ALL: [Class; 10] = [
        Class::Mage,
        Class::Marksman,
        Class::Tank,
        Class::Bruiser,
        Class::Enchanter,
        Class::Assassin,
        Class::Artillery,
        Class::Warden,
        Class::Battlemage,
        Class::Skirmisher,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Class::Mage => "Mage",
            Class::Marksman => "Marksman",
            Class::Tank => "Tank",
            Class::Bruiser => "Bruiser",
            Class::Enchanter => "Enchanter",
            Class::Assassin => "Assassin",
            Class::Artillery => "Artillery",
            Class::Warden => "Warden",
            Class::Battlemage => "Battlemage",
            Class::Skirmisher => "Skirmisher",
        }
    }

    pub fn by_name(name: &str) -> Option<Class> {
        Class::ALL.into_iter().find(|c| c.name() == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChampionDef {
    pub name: &'static str,
    pub class: Class,
    /// Level 1.
    pub stats: Stats,
    /// Added per level on the 02 §1 curve (move speed doesn't grow).
    pub growth: Stats,
    pub attack: AttackSpec,
    pub abilities: [Ability; 4],
    /// What the abilities cost (01 §9, 20 §8.1). `Resource::NONE` (costless) for the official
    /// ten for now.
    pub resource: Resource,
    /// The champion's own passive (01 §9, 20 §8.1): one of the items' unique passives (18 §4),
    /// innate. `Passive::None` for the official ten for now. Unique as items' are: an item of the
    /// same kind counts instead only if it's stronger.
    pub passive: crate::items::Passive,
}

/// A kit's resource (01 §9, 20 §8.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceKind {
    /// Costless.
    None,
    /// A pool that grows with level and refills slowly (and at the fountain).
    Mana,
    /// A small pool that refills fast.
    Energy,
    /// Starts empty, builds on basic attacks that land (`regen` a hit), drains out of use.
    Rage,
    /// Abilities cost health; no pool.
    Health,
}

/// The resource, its pool and each ability's cost (Q W E R). In the unit's state,
/// `UnitState::resource` is what's been **spent** of a mana or energy pool (0 is full, so a
/// costless unit's state is unchanged) or the rage built up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resource {
    pub kind: ResourceKind,
    /// The pool at level 1 (mana grows 8% of it a level; energy and rage don't).
    pub max: f32,
    /// A second (mana, energy), or a basic attack that lands (rage).
    pub regen: f32,
    pub costs: [f32; 4],
}

/// Rage drains this much a second.
pub const RAGE_DECAY: f32 = 4.0;

impl Resource {
    pub const NONE: Resource = Resource { kind: ResourceKind::None, max: 0.0, regen: 0.0, costs: [0.0; 4] };

    /// The pool at `level`.
    pub fn max_at(&self, level: u8) -> f32 {
        match self.kind {
            ResourceKind::Mana => self.max * (1.0 + 0.08 * (level.max(1) - 1) as f32),
            _ => self.max,
        }
    }

    /// What there is to spend now: `state` is `UnitState::resource`, `health` the unit's.
    pub fn available(&self, state: f32, health: f32, level: u8) -> f32 {
        match self.kind {
            ResourceKind::None => f32::INFINITY,
            ResourceKind::Mana | ResourceKind::Energy => (self.max_at(level) - state).max(0.0),
            ResourceKind::Rage => state,
            // Never down to zero by a cast.
            ResourceKind::Health => (health - 1.0).max(0.0),
        }
    }

    /// The cost of `slot` (Q W E R; utility spells are free).
    pub fn cost(&self, slot: u8) -> f32 {
        if self.kind == ResourceKind::None { 0.0 } else { self.costs.get(slot as usize).copied().unwrap_or(0.0) }
    }
}

impl ChampionDef {
    /// Stats at `level` (1–18).
    pub fn stats_at(&self, level: u8) -> Stats {
        let (b, g) = (&self.stats, &self.growth);
        let at = |base: f32, growth: f32| stat_at_level(base, growth, level.max(1));
        Stats {
            max_health: at(b.max_health, g.max_health),
            health_regen: at(b.health_regen, g.health_regen),
            armor: at(b.armor, g.armor),
            magic_resist: at(b.magic_resist, g.magic_resist),
            attack_damage: at(b.attack_damage, g.attack_damage),
            ability_power: at(b.ability_power, g.ability_power),
            move_speed: b.move_speed,
            ..Stats::NONE
        }
    }
}

/// Highest rank of slot `slot` (0–3) at `level`: basic abilities ⌈level / 2⌉ up to 5, the
/// ultimate 1 / 2 / 3 at levels 6 / 11 / 16.
pub fn max_rank(slot: u8, level: u8) -> u8 {
    if slot == 3 { (level.saturating_sub(1) / 5).min(3) } else { level.div_ceil(2).min(5) }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ten archetypes (M2's six, M3's four), each with a distinct kit.
    #[test]
    fn ten_champions_with_unique_names() {
        let names: std::collections::BTreeSet<&str> =
            ChampionId::ALL.iter().flat_map(|c| c.def().abilities.map(|a| a.name)).collect();
        assert_eq!(names.len(), 40);
        for (i, c) in ChampionId::ALL.into_iter().enumerate() {
            assert_eq!(ChampionId::from_u8(i as u8), Some(c));
            assert_eq!(ChampionId::by_name(c.def().name), Some(c));
        }
    }

    #[test]
    fn attack_timers_are_exact_subticks() {
        let a = ChampionId::Vesper.def().attack;
        assert_eq!(a.period().0, 2400); // 1 / 0.8 s
        assert_eq!(a.windup().0, 432); // 18% of it
        assert_eq!(ChampionId::by_name("ember"), Some(ChampionId::Ember));
        assert_eq!(ChampionId::from_u8(1), Some(ChampionId::Vesper));
    }
}

#[cfg(test)]
mod registry_tests {
    use super::*;

    /// A champion that isn't one of the ten: Rook's kit under another name, slower.
    fn custom(name: &'static str) -> ChampionDef {
        let mut d = *ChampionId::Rook.def();
        d.name = name;
        d.stats.move_speed = 300.0;
        d
    }

    #[test]
    fn the_official_ten_hold_ids_0_to_9() {
        for (i, c) in ChampionId::ALL.into_iter().enumerate() {
            assert_eq!(c.index() as usize, i);
            assert!(c.is_official());
            assert_eq!(format!("{c:?}"), c.def().name);
        }
        assert_eq!(ChampionId::from_u8(10), None);
    }

    #[test]
    fn registering_a_champion_twice_gives_one_id() {
        let a = ChampionId::register(custom("Registry Test A")).unwrap();
        assert!(!a.is_official());
        assert_eq!(ChampionId::register(custom("Registry Test A")), Ok(a));
        let b = ChampionId::register(custom("Registry Test B")).unwrap();
        assert_ne!(a, b);
        assert_eq!(ChampionId::by_name("registry test a"), Some(a));
        assert_eq!(a.def().stats.move_speed, 300.0);
        assert_eq!(format!("{a:?}"), "Registry Test A");
        // An official definition registers as itself.
        assert_eq!(ChampionId::register(*ChampionId::Wren.def()), Ok(ChampionId::Wren));
    }

    #[test]
    fn a_registered_champion_plays() {
        use crate::math::Vec2;
        use crate::world::{PlayerId, Team, World};
        let id = ChampionId::register(custom("Registry Test Player")).unwrap();
        let mut w = World::new(1);
        let unit = w.spawn_champion(PlayerId(0), Team::Blue, id, Vec2::new(1000.0, 1000.0));
        let u = w.unit(unit).unwrap();
        assert_eq!(u.champion, Some(id));
        assert_eq!(u.stats.move_speed, 300.0);
    }
}

#[cfg(test)]
mod rank_tests {
    use super::*;

    #[test]
    fn rank_gates_and_growth() {
        assert_eq!((max_rank(0, 1), max_rank(0, 2), max_rank(0, 3), max_rank(0, 9), max_rank(0, 18)), (1, 1, 2, 5, 5));
        assert_eq!(
            (max_rank(3, 5), max_rank(3, 6), max_rank(3, 11), max_rank(3, 16), max_rank(3, 18)),
            (0, 1, 2, 3, 3)
        );
        let ember = ChampionId::Ember.def();
        assert_eq!(ember.stats_at(1), ember.stats);
        let l18 = ChampionId::Vesper.def().stats_at(18);
        assert!((l18.max_health - (620.0 + 96.0 * 17.0)).abs() < 1e-3);
        let q = ember.abilities[0];
        assert_eq!(q.cooldown_at(5).0, q.cooldown.0 - 4 * SimDuration::from_millis(500).0);
        assert_eq!(q.bonus_damage_at(3), 80.0);
    }
}
