//! Augments (M3, 06 §3): permanent modifiers drafted during an ARAM: Mayhem match.
//!
//! At scheduled levels a champion is offered three augments of one tier and keeps one; each
//! offer can be rerolled once. Offers come from the champion's own augment seed and the draft
//! and reroll counts, never from the world RNG, so picking and rerolling are predicted on the
//! client exactly like shopping. Stat augments join the item stat stack (02 §10): flat bonuses
//! with the items, then conversions, then percent bonuses, then caps.

use crate::ability::{
    Ability, Cc, Damage, DamageKind, Dash, DelayedArea, Effect as Shape, RankScaling, ReactionClass, Support,
    Transforms,
};
use crate::items::Bonus;
use crate::rng::Pcg32;
use crate::time::SimDuration;
use crate::world::Progress;

/// Augments a champion can hold.
pub const SLOTS: usize = 4;
/// Choices in each offer.
pub const CHOICES: usize = 3;
/// A draft opens when the champion reaches each of these levels (ARAM starts at level 3, so
/// the first opens at once) *(start values)*.
pub const DRAFT_LEVELS: [u8; SLOTS] = [1, 7, 11, 15];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    /// Simple stat conversions and consistent passives.
    Silver,
    /// Playstyle adjustments and strong conditionals.
    Gold,
    /// Identity-rewriting game-breakers.
    Prismatic,
}

impl Tier {
    pub fn name(self) -> &'static str {
        match self {
            Tier::Silver => "Silver",
            Tier::Gold => "Gold",
            Tier::Prismatic => "Prismatic",
        }
    }
}

/// What an augment does beyond its stat bonus.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    None,
    /// Bonus attack damage (from items and augments) becomes ability power at this rate.
    AdToAp(f32),
    /// Bonus ability power becomes attack damage at this rate.
    ApToAd(f32),
    /// Line skillshots that accept it fire three projectiles in a spread.
    Multishot,
    /// Skillshots and areas that accept it repeat after a delay at reduced power.
    Echo,
    /// Wider skillshots and larger areas (where accepted).
    Wide,
    /// Grow: a larger hitbox, more health and adaptive force.
    Titan,
    /// Shrink: a smaller hitbox, more speed, more damage to larger targets.
    Pebble,
    /// Titan or Pebble, rolled again at every respawn.
    Unstable,
    /// More damage to targets below a share of their health.
    Executioner,
    /// More damage to targets at full health.
    FirstStrike,
    /// More damage the lower your own health.
    LastStand,
    /// Ability hits can critically strike.
    Spellcrit,
    /// Ability hits on champions grant permanent ability power.
    Spellhunger,
    /// Abilities heal for a share of the damage they deal.
    SpellVamp,
    /// Champions' basic attacks on you take a share back as magic damage.
    Thorns,
    /// The next basic attack after an ability hits harder.
    Spellblade,
    /// Takedowns refresh Q, W and E.
    Reset,
    /// Stat Anvils roll higher tiers.
    AnvilLuck,
    /// A quest: enough takedowns grant a large stat reward.
    ChampionOfChaos,
    /// No ultimate; Q, W and E hit harder.
    Fundamentals,
    /// A ranged champion fights in melee, with stats to compensate.
    CloseQuarters,
    /// Longer basic-attack range for ranged champions.
    Sharpshooter,
    /// The F utility spell is replaced by this ability.
    Spell(Ability),
}

/// The mechanic augments' numbers *(start values)*.
pub const EXECUTE_BELOW: f32 = 0.35;
pub const EXECUTE_AMP: f32 = 1.2;
pub const FIRST_STRIKE_AMP: f32 = 1.12;
/// Last Stand: up to this much more damage, growing as health falls below the threshold.
pub const LAST_STAND_MAX: f32 = 0.25;
pub const LAST_STAND_BELOW: f32 = 0.6;
/// Spellcrit: one ability hit in this many crits, for this much more damage.
pub const SPELLCRIT_ONE_IN: u32 = 4;
pub const SPELLCRIT_AMP: f32 = 1.6;
/// Spellhunger: ability power per ability hit on a champion, up to a cap.
pub const SPELLHUNGER_CAP: u16 = 80;
pub const SPELL_VAMP: f32 = 0.12;
pub const THORNS: f32 = 0.25;
/// Thorns's id (its damage is attributed to it).
pub const THORNS_ID: u8 = 46;
/// Spellblade: an ability arms it for this long; the next attack adds the base attack damage.
pub const SPELLBLADE_MS: u64 = 4000;
/// Champion of Chaos: takedowns to complete the quest.
pub const CHAOS_TAKEDOWNS: u8 = 8;
pub const FUNDAMENTALS_AMP: f32 = 1.35;
/// Close Quarters: melee reach; Sharpshooter: extra range.
pub const CLOSE_QUARTERS_RANGE: f32 = 175.0;
pub const SHARPSHOOTER_RANGE: f32 = 100.0;

const fn ms(v: u64) -> SimDuration {
    SimDuration::from_millis(v)
}

/// Vault (replaces F): a long dash.
pub const VAULT: Ability = Ability {
    name: "Vault",
    cooldown: ms(25_000),
    effect: Shape::Dash(Dash { range: 550.0, speed: 1400.0 }),
    reaction: ReactionClass::None,
    per_rank: RankScaling::NONE,
    transforms: Transforms::NONE,
};

/// Stormcall (replaces F): a telegraphed nova that knocks up everyone around you.
pub const STORMCALL: Ability = Ability {
    name: "Stormcall",
    cooldown: ms(40_000),
    effect: Shape::Area(DelayedArea {
        windup: ms(0),
        range: 0.0,
        radius: 300.0,
        delay: ms(600),
        damage: Damage { kind: DamageKind::Magic, base: 80.0, ad_ratio: 0.0, ap_ratio: 0.5 },
        cc: Cc::Knockup(ms(750)),
    }),
    reaction: ReactionClass::None,
    per_rank: RankScaling::NONE,
    transforms: Transforms::NONE,
};

/// Mend (replaces F): heal an ally near the cursor, or yourself.
pub const MEND: Ability = Ability {
    name: "Mend",
    cooldown: ms(60_000),
    effect: Shape::Support(Support {
        range: 800.0,
        heal: 150.0,
        heal_ap: 0.3,
        heal_missing: 0.0,
        shield: 0.0,
        shield_ap: 0.0,
        duration: ms(0),
    }),
    reaction: ReactionClass::None,
    per_rank: RankScaling::NONE,
    transforms: Transforms::NONE,
};

/// What a champion's augments have built up, for the stat stack: the Unstable roll,
/// Spellhunger's stacks and Champion of Chaos's completed quest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Growth {
    pub unstable_tiny: bool,
    pub stacks: u16,
    pub chaos_done: bool,
    /// Hyper rules (not an augment, but part of the same stat stack).
    pub hyper: bool,
    /// Stat Anvils kept: steps of each stat (`anvils::STATS` order).
    pub anvil: [u16; 8],
    /// The Insight jungle buff (not an augment either).
    pub insight: bool,
}

impl Growth {
    pub const NONE: Growth =
        Growth { unstable_tiny: false, stacks: 0, chaos_done: false, hyper: false, anvil: [0; 8], insight: false };
}

/// Champion of Chaos's reward.
pub const CHAOS_REWARD: Bonus = Bonus { attack_damage: 50.0, ability_power: 80.0, health: 500.0, ..Bonus::NONE };

/// The mechanic flags a set of augments grants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Mods {
    pub executioner: bool,
    pub first_strike: bool,
    pub last_stand: bool,
    pub spellcrit: bool,
    pub spellhunger: bool,
    pub spell_vamp: bool,
    pub thorns: bool,
    pub spellblade: bool,
    pub reset: bool,
    pub chaos: bool,
    pub fundamentals: bool,
    pub anvil_luck: bool,
}

pub fn mods(slots: &[u8; SLOTS]) -> Mods {
    let mut m = Mods::default();
    for a in held(slots) {
        match a.effect {
            Effect::Executioner => m.executioner = true,
            Effect::FirstStrike => m.first_strike = true,
            Effect::LastStand => m.last_stand = true,
            Effect::Spellcrit => m.spellcrit = true,
            Effect::Spellhunger => m.spellhunger = true,
            Effect::SpellVamp => m.spell_vamp = true,
            Effect::Thorns => m.thorns = true,
            Effect::Spellblade => m.spellblade = true,
            Effect::Reset => m.reset = true,
            Effect::AnvilLuck => m.anvil_luck = true,
            Effect::ChampionOfChaos => m.chaos = true,
            Effect::Fundamentals => m.fundamentals = true,
            _ => {}
        }
    }
    m
}

/// The ability replacing the F utility spell, if an augment grants one.
pub fn spell(slots: &[u8; SLOTS]) -> Option<Ability> {
    held(slots).find_map(|a| match a.effect {
        Effect::Spell(ability) => Some(ability),
        _ => None,
    })
}

/// A champion's size form (06 §3 physical transformations).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    Normal,
    Huge,
    Tiny,
}

/// Hitbox scale per form (the gameplay radius and the model; collision with units and walls
/// keeps the champion size, which paths and wall clearance are tuned for).
pub const TITAN_SCALE: f32 = 1.5;
pub const PEBBLE_SCALE: f32 = 0.6;
/// Huge: +30% health and 40 adaptive force. Tiny: +12% movement speed, +20% damage to targets
/// with a larger hitbox.
pub const TITAN_HEALTH: f32 = 0.30;
pub const TITAN_FORCE: f32 = 40.0;
pub const PEBBLE_SPEED: f32 = 0.12;
pub const PEBBLE_AMP: f32 = 1.2;

/// The form from held augments (Unstable: as last rolled).
pub fn form(slots: &[u8; SLOTS], unstable_tiny: bool) -> Form {
    let mut f = Form::Normal;
    for a in held(slots) {
        match a.effect {
            Effect::Titan => f = Form::Huge,
            Effect::Pebble => f = Form::Tiny,
            Effect::Unstable => f = if unstable_tiny { Form::Tiny } else { Form::Huge },
            _ => {}
        }
    }
    f
}

pub fn scale(form: Form) -> f32 {
    match form {
        Form::Normal => 1.0,
        Form::Huge => TITAN_SCALE,
        Form::Tiny => PEBBLE_SCALE,
    }
}

/// Unstable Experiment's roll for a seed and an instant (each respawn, and when picked).
pub fn unstable_roll(seed: u32, at: u64) -> bool {
    Pcg32::new(seed as u64 ^ at, 0x7369_7a65).next_u32() & 1 == 1
}

/// Multishot: projectiles per volley and the angle between neighbors (15°, as exact
/// constants: no trigonometry in the sim).
pub const MULTISHOT_COUNT: u8 = 3;
pub const SPREAD_COS: f32 = 0.965_925_8;
pub const SPREAD_SIN: f32 = 0.258_819_04;
/// Echo: the repeat comes this much later, at this share of the power.
pub const ECHO_DELAY_MS: u64 = 750;
pub const ECHO_POWER: f32 = 0.4;
/// Echo shots are numbered from here (volley shots below it).
pub const ECHO_SHOT: u8 = 4;
/// Broadside: projectile width and area radius factors.
pub const WIDE_LINE: f32 = 1.5;
pub const WIDE_AREA: f32 = 1.25;

/// The delivery transformers a set of augments grants.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Delivery {
    pub multishot: bool,
    pub echo: bool,
    pub wide: bool,
}

pub fn delivery(slots: &[u8; SLOTS]) -> Delivery {
    let mut d = Delivery::default();
    for a in held(slots) {
        match a.effect {
            Effect::Multishot => d.multishot = true,
            Effect::Echo => d.echo = true,
            Effect::Wide => d.wide = true,
            _ => {}
        }
    }
    d
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Augment {
    /// Stable on the wire; 0 = none.
    pub id: u8,
    pub name: &'static str,
    pub tier: Tier,
    /// For the draft cards.
    pub text: &'static str,
    pub bonus: Bonus,
    pub effect: Effect,
}

const fn b() -> Bonus {
    Bonus::NONE
}

macro_rules! aug {
    ($id:expr, $tier:ident, $name:expr, $text:expr, $bonus:expr) => {
        Augment { id: $id, name: $name, tier: Tier::$tier, text: $text, bonus: $bonus, effect: Effect::None }
    };
    ($id:expr, $tier:ident, $name:expr, $text:expr, $bonus:expr, $effect:expr) => {
        Augment { id: $id, name: $name, tier: Tier::$tier, text: $text, bonus: $bonus, effect: $effect }
    };
}

/// Every augment (original names; *(start values)*).
pub const CATALOG: &[Augment] = &[
    // Silver: stats and conversions.
    aug!(
        1,
        Silver,
        "Thick Skin",
        "+350 health and +3 health regeneration per second.",
        Bonus { health: 350.0, health_regen: 3.0, ..b() }
    ),
    aug!(2, Silver, "Whetstone", "+20 attack damage.", Bonus { attack_damage: 20.0, ..b() }),
    aug!(3, Silver, "Spark", "+35 ability power.", Bonus { ability_power: 35.0, ..b() }),
    aug!(4, Silver, "Quickstep", "+8% movement speed.", Bonus { move_speed_pct: 0.08, ..b() }),
    aug!(5, Silver, "Plating", "+25 armor and +25 magic resist.", Bonus { armor: 25.0, magic_resist: 25.0, ..b() }),
    aug!(6, Silver, "Swift Hands", "+30% attack speed.", Bonus { attack_speed: 0.30, ..b() }),
    aug!(7, Silver, "Clear Mind", "+20 ability haste.", Bonus { ability_haste: 20.0, ..b() }),
    aug!(8, Silver, "Bloodthirst", "+10% life steal.", Bonus { life_steal: 0.10, ..b() }),
    aug!(9, Silver, "Conversion", "Your bonus attack damage becomes ability power at 110%.", b(), Effect::AdToAp(1.1)),
    aug!(10, Silver, "Reversal", "Your bonus ability power becomes attack damage at 60%.", b(), Effect::ApToAd(0.6)),
    // Gold: bigger packages.
    aug!(
        11,
        Gold,
        "Brute Force",
        "+30 attack damage and +20 ability haste.",
        Bonus { attack_damage: 30.0, ability_haste: 20.0, ..b() }
    ),
    aug!(
        12,
        Gold,
        "Arcane Surge",
        "+60 ability power, then +10% ability power.",
        Bonus { ability_power: 60.0, ability_power_pct: 0.10, ..b() }
    ),
    aug!(13, Gold, "Juggernaut", "+15% health and +40 armor.", Bonus { health_pct: 0.15, armor: 40.0, ..b() }),
    aug!(
        14,
        Gold,
        "Fleet",
        "+12% movement speed and +30% attack speed.",
        Bonus { move_speed_pct: 0.12, attack_speed: 0.30, ..b() }
    ),
    aug!(
        15,
        Gold,
        "Vampiric Pact",
        "+15% life steal and +20 attack damage.",
        Bonus { life_steal: 0.15, attack_damage: 20.0, ..b() }
    ),
    aug!(
        16,
        Gold,
        "Glass Cannon",
        "+25% attack damage and ability power, but 20% less health.",
        Bonus { attack_damage_pct: 0.25, ability_power_pct: 0.25, health_pct: -0.20, ..b() }
    ),
    aug!(26, Gold, "Broadside", "Your skillshots are 50% wider and your ground areas 25% larger.", b(), Effect::Wide),
    aug!(
        20,
        Gold,
        "Sorcery Engine",
        "+40 ability power and +25 ability haste.",
        Bonus { ability_power: 40.0, ability_haste: 25.0, ..b() }
    ),
    // Prismatic.
    aug!(
        17,
        Prismatic,
        "Apex Form",
        "+25% health, attack damage, ability power, armor and magic resist.",
        Bonus { health_pct: 0.25, attack_damage_pct: 0.25, ability_power_pct: 0.25, resist_pct: 0.25, ..b() }
    ),
    aug!(
        18,
        Prismatic,
        "Celerity",
        "+60 ability haste and +15% movement speed.",
        Bonus { ability_haste: 60.0, move_speed_pct: 0.15, ..b() }
    ),
    aug!(
        19,
        Prismatic,
        "Bulwark",
        "+1000 health, +60 armor and +60 magic resist.",
        Bonus { health: 1000.0, armor: 60.0, magic_resist: 60.0, ..b() }
    ),
    aug!(
        21,
        Prismatic,
        "Overload",
        "+100 ability power, then +20% ability power.",
        Bonus { ability_power: 100.0, ability_power_pct: 0.20, ..b() }
    ),
    aug!(
        22,
        Prismatic,
        "Warlord",
        "+60 attack damage and +40% attack speed.",
        Bonus { attack_damage: 60.0, attack_speed: 0.40, ..b() }
    ),
    aug!(
        23,
        Prismatic,
        "Undying",
        "+500 health and +25% life steal.",
        Bonus { health: 500.0, life_steal: 0.25, ..b() }
    ),
    aug!(
        27,
        Prismatic,
        "Titan",
        "Grow by 50%: a larger hitbox, +30% health and 40 adaptive force.",
        b(),
        Effect::Titan
    ),
    aug!(
        28,
        Prismatic,
        "Pebble",
        "Shrink by 40%: a smaller hitbox, +12% movement speed and 20% more damage to larger targets.",
        b(),
        Effect::Pebble
    ),
    aug!(
        29,
        Prismatic,
        "Unstable Experiment",
        "Every time you respawn, you become a Titan or a Pebble at random.",
        b(),
        Effect::Unstable
    ),
    aug!(
        24,
        Prismatic,
        "Multishot",
        "Your line skillshots fire three projectiles in a spread. Each enemy is hit by one at most.",
        b(),
        Effect::Multishot
    ),
    aug!(
        25,
        Prismatic,
        "Echo",
        "Your skillshots and ground areas repeat 0.75 s later at 40% power.",
        b(),
        Effect::Echo
    ),
    // M3 slice 4: more stats, conditionals, rule breakers, quests and spell replacements.
    aug!(30, Silver, "Fleetfoot", "+30 movement speed.", Bonus { move_speed: 30.0, ..b() }),
    aug!(
        31,
        Silver,
        "Arcane Battery",
        "+200 health and +20 ability power.",
        Bonus { health: 200.0, ability_power: 20.0, ..b() }
    ),
    aug!(
        32,
        Silver,
        "Duelist's Edge",
        "+15 attack damage and +15% attack speed.",
        Bonus { attack_damage: 15.0, attack_speed: 0.15, ..b() }
    ),
    aug!(33, Silver, "Warded", "+45 magic resist.", Bonus { magic_resist: 45.0, ..b() }),
    aug!(34, Silver, "Stoneskin", "+45 armor.", Bonus { armor: 45.0, ..b() }),
    aug!(35, Silver, "Recovery", "+6 health regeneration per second.", Bonus { health_regen: 6.0, ..b() }),
    aug!(
        36,
        Silver,
        "Scholar",
        "+20 ability power and +10 ability haste.",
        Bonus { ability_power: 20.0, ability_haste: 10.0, ..b() }
    ),
    aug!(
        37,
        Silver,
        "Toughness",
        "+150 health and +15 armor and magic resist.",
        Bonus { health: 150.0, armor: 15.0, magic_resist: 15.0, ..b() }
    ),
    aug!(38, Silver, "Executioner", "+20% damage to enemies below 35% health.", b(), Effect::Executioner),
    aug!(39, Silver, "First Strike", "+12% damage to enemies at full health.", b(), Effect::FirstStrike),
    aug!(
        40,
        Silver,
        "Mend",
        "Your F spell becomes Mend: heal an ally near the cursor, or yourself, for 150 (+30% AP).",
        b(),
        Effect::Spell(MEND)
    ),
    aug!(
        41,
        Gold,
        "Spellhunger",
        "Each ability hit on a champion grants 1 ability power, up to 80.",
        b(),
        Effect::Spellhunger
    ),
    aug!(
        42,
        Gold,
        "Spellblade",
        "After you cast an ability, your next basic attack within 4 s deals bonus damage equal to your base attack damage.",
        b(),
        Effect::Spellblade
    ),
    aug!(43, Gold, "Last Stand", "Deal up to 25% more damage as your health falls below 60%.", b(), Effect::LastStand),
    aug!(44, Gold, "Spell Vamp", "Your abilities heal you for 12% of the damage they deal.", b(), Effect::SpellVamp),
    aug!(45, Gold, "Reset", "Takedowns refresh your Q, W and E.", b(), Effect::Reset),
    aug!(
        46,
        Gold,
        "Thorns",
        "Champions that basic-attack you take 25% of the damage back as magic damage.",
        Bonus { armor: 20.0, ..b() },
        Effect::Thorns
    ),
    aug!(
        47,
        Gold,
        "Sharpshooter",
        "Ranged champions: +100 attack range and +10% attack speed.",
        Bonus { attack_speed: 0.10, ..b() },
        Effect::Sharpshooter
    ),
    aug!(48, Gold, "Vault", "Your F spell becomes Vault: a 550 u dash.", b(), Effect::Spell(VAULT)),
    aug!(49, Gold, "Iron Heart", "+400 health and +10% health.", Bonus { health: 400.0, health_pct: 0.10, ..b() }),
    aug!(
        50,
        Gold,
        "Battle Trance",
        "+25 attack damage and +25% attack speed.",
        Bonus { attack_damage: 25.0, attack_speed: 0.25, ..b() }
    ),
    aug!(
        51,
        Gold,
        "Mind Over Matter",
        "+50 ability power and +30 magic resist.",
        Bonus { ability_power: 50.0, magic_resist: 30.0, ..b() }
    ),
    aug!(
        52,
        Prismatic,
        "Spellcrit",
        "Your ability hits can critically strike: one in four deals 60% more damage.",
        b(),
        Effect::Spellcrit
    ),
    aug!(
        53,
        Prismatic,
        "Close Quarters",
        "A ranged champion fights in melee, with +400 health, +40 armor and magic resist and +25% attack speed.",
        Bonus { health: 400.0, armor: 40.0, magic_resist: 40.0, attack_speed: 0.25, ..b() },
        Effect::CloseQuarters
    ),
    aug!(
        54,
        Prismatic,
        "Fundamentals",
        "Your ultimate is disabled; Q, W and E deal 35% more damage, and +40 ability haste.",
        Bonus { ability_haste: 40.0, ..b() },
        Effect::Fundamentals
    ),
    aug!(
        55,
        Prismatic,
        "Champion of Chaos",
        "Quest: score 8 takedowns. Reward: +50 attack damage, +80 ability power and +500 health.",
        b(),
        Effect::ChampionOfChaos
    ),
    aug!(
        56,
        Prismatic,
        "Stormcall",
        "Your F spell becomes Stormcall: after 0.6 s, a 300 u nova around you knocks up enemies and deals 80 (+50% AP) magic damage.",
        b(),
        Effect::Spell(STORMCALL)
    ),
    aug!(
        57,
        Silver,
        "Light Armor",
        "+25 armor and +4% movement speed.",
        Bonus { armor: 25.0, move_speed_pct: 0.04, ..b() }
    ),
    aug!(
        58,
        Silver,
        "Glint",
        "+12 attack damage and +12 ability power.",
        Bonus { attack_damage: 12.0, ability_power: 12.0, ..b() }
    ),
    aug!(
        59,
        Gold,
        "Heavy Hitter",
        "+40 attack damage, then +8% attack damage.",
        Bonus { attack_damage: 40.0, attack_damage_pct: 0.08, ..b() }
    ),
    aug!(
        60,
        Prismatic,
        "Eternal Engine",
        "+800 health, +60 ability power and +30 ability haste.",
        Bonus { health: 800.0, ability_power: 60.0, ability_haste: 30.0, ..b() }
    ),
    aug!(
        61,
        Gold,
        "Blacksmith's Blessing",
        "Stat Anvils roll higher: 20% Silver, 50% Gold, 30% Prismatic. +10 ability haste.",
        Bonus { ability_haste: 10.0, ..b() },
        Effect::AnvilLuck
    ),
];

pub fn augment(id: u8) -> Option<&'static Augment> {
    CATALOG.iter().find(|a| a.id == id)
}

/// The augments in a holder's slots.
pub fn held(slots: &[u8; SLOTS]) -> impl Iterator<Item = &'static Augment> + '_ {
    slots.iter().filter_map(|id| augment(*id))
}

/// The tier of draft `draft` (0-based) for a champion with this seed: Silver first, one
/// guaranteed Prismatic at the second or third draft, Gold otherwise.
pub fn tier_of(seed: u32, draft: u8) -> Tier {
    let prismatic = 1 + (seed & 1) as u8;
    match draft {
        0 => Tier::Silver,
        d if d == prismatic => Tier::Prismatic,
        _ => Tier::Gold,
    }
}

/// Three distinct augments of the draft's tier that the champion doesn't hold (and, for a
/// reroll, that weren't just offered when the tier has enough of them).
pub fn make_offer(seed: u32, draft: u8, roll: u8, held: &[u8; SLOTS], previous: &[u8; CHOICES]) -> [u8; CHOICES] {
    let tier = tier_of(seed, draft);
    let mut pool: Vec<u8> = CATALOG.iter().filter(|a| a.tier == tier && !held.contains(&a.id)).map(|a| a.id).collect();
    let fresh: Vec<u8> = pool.iter().copied().filter(|id| !previous.contains(id)).collect();
    if fresh.len() >= CHOICES {
        pool = fresh;
    }
    let mut rng = Pcg32::new(seed as u64, 1 + draft as u64 * 2 + roll as u64);
    let mut offer = [0u8; CHOICES];
    for slot in offer.iter_mut() {
        if pool.is_empty() {
            break;
        }
        *slot = pool.swap_remove(rng.next_u32() as usize % pool.len());
    }
    offer
}

/// One draft in this many (Silver or Gold) has a golden reroll on one of its choices: it
/// rolls that choice one tier up.
pub const GOLDEN_ONE_IN: u32 = 4;

/// The tier above.
pub fn tier_up(t: Tier) -> Tier {
    match t {
        Tier::Silver => Tier::Gold,
        Tier::Gold | Tier::Prismatic => Tier::Prismatic,
    }
}

/// Open the next draft once the champion's level reaches it (one at a time).
pub fn update_draft(p: &mut Progress) {
    let d = p.drafted as usize;
    if p.offer[0] != 0 || d >= SLOTS || p.level < DRAFT_LEVELS[d] {
        return;
    }
    p.offer = make_offer(p.augment_seed, p.drafted, 0, &p.augments, &[0; CHOICES]);
    // Sometimes a Silver or Gold draft brings a golden reroll on one choice (06 §3, like the
    // reference game's): shown before it's used, so it's a decision, not a surprise.
    let mut rng = Pcg32::new(p.augment_seed as u64, 1000 + p.drafted as u64);
    let roll = rng.next_u32();
    p.golden = if tier_of(p.augment_seed, p.drafted) != Tier::Prismatic && roll.is_multiple_of(GOLDEN_ONE_IN) {
        1 + (rng.next_u32() % CHOICES as u32) as u8
    } else {
        0
    };
    p.drafted += 1;
    p.rerolled = 0;
}

/// Keep choice `choice` of the open offer.
pub fn pick(p: &mut Progress, choice: u8) {
    let Some(&id) = p.offer.get(choice as usize) else { return };
    let Some(slot) = p.augments.iter().position(|a| *a == 0) else { return };
    if id == 0 {
        return;
    }
    p.augments[slot] = id;
    p.offer = [0; CHOICES];
    if augment(id).is_some_and(|a| a.effect == Effect::Unstable) {
        p.unstable_tiny = unstable_roll(p.augment_seed, p.drafted as u64);
    }
}

/// Replace one choice of the open offer (each choice once per draft, 06 §3): another
/// augment of the same tier that isn't held, offered or the one replaced; the golden reroll
/// rolls one tier up.
pub fn reroll(p: &mut Progress, choice: u8) {
    let c = choice as usize;
    if c >= CHOICES || p.offer[c] == 0 || p.rerolled & (1 << c) != 0 || p.drafted == 0 {
        return;
    }
    let draft = p.drafted - 1;
    let mut tier = augment(p.offer[c]).map_or(tier_of(p.augment_seed, draft), |a| a.tier);
    if p.golden == choice + 1 {
        tier = tier_up(tier);
    }
    let pool: Vec<u8> = CATALOG
        .iter()
        .filter(|a| a.tier == tier && !p.augments.contains(&a.id) && !p.offer.contains(&a.id))
        .map(|a| a.id)
        .collect();
    if pool.is_empty() {
        return;
    }
    let mut rng = Pcg32::new(p.augment_seed as u64, 2000 + draft as u64 * 8 + c as u64);
    p.offer[c] = pool[rng.next_u32() as usize % pool.len()];
    p.rerolled |= 1 << c;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        for (i, a) in CATALOG.iter().enumerate() {
            assert!(a.id != 0);
            assert_eq!(augment(a.id), Some(a), "id {} at {i}", a.id);
            assert!(!a.text.is_empty());
        }
        // An offer and its reroll show six different augments, even in the second Gold draft
        // (one Gold augment already held).
        for (tier, need) in [(Tier::Silver, 6), (Tier::Gold, 7), (Tier::Prismatic, 6)] {
            let n = CATALOG.iter().filter(|a| a.tier == tier).count();
            assert!(n >= need, "{tier:?} has {n}");
        }
    }

    /// M3 slice 4: sixty augments (and one more since), every id from 1 once, spread over
    /// the tiers.
    #[test]
    fn catalog_has_every_augment_id_once() {
        let mut ids: Vec<u8> = CATALOG.iter().map(|a| a.id).collect();
        ids.sort();
        assert_eq!(ids, (1..=61).collect::<Vec<u8>>());
        for tier in [Tier::Silver, Tier::Gold, Tier::Prismatic] {
            assert!(CATALOG.iter().filter(|a| a.tier == tier).count() >= 15, "{tier:?}");
        }
    }

    /// A golden reroll rolls its choice one tier up; some Silver and Gold drafts have one,
    /// Prismatic ones never.
    #[test]
    fn a_golden_reroll_rolls_one_tier_up() {
        let mut p = Progress { level: 3, augment_seed: 5, ..Progress::SANDBOX };
        update_draft(&mut p);
        p.golden = 3;
        reroll(&mut p, 2);
        assert_eq!(augment(p.offer[2]).unwrap().tier, Tier::Gold);
        assert_eq!(augment(p.offer[0]).unwrap().tier, Tier::Silver);
        let mut seen = [false; 2];
        for seed in 0..200 {
            for draft in 0..SLOTS as u8 {
                let level = DRAFT_LEVELS[draft as usize];
                let mut q = Progress { level, augment_seed: seed, drafted: draft, ..Progress::SANDBOX };
                update_draft(&mut q);
                if q.golden != 0 {
                    assert_ne!(tier_of(seed, draft), Tier::Prismatic);
                    seen[(q.golden - 1).min(1) as usize] = true;
                }
            }
        }
        assert!(seen.iter().any(|s| *s), "golden rerolls happen");
    }

    #[test]
    fn drafts_open_by_level_and_picks_fill_slots() {
        let mut p = Progress { level: 3, augment_seed: 77, ..Progress::SANDBOX };
        update_draft(&mut p);
        assert_eq!(p.drafted, 1);
        let first = p.offer;
        assert!(first.iter().all(|id| augment(*id).is_some_and(|a| a.tier == Tier::Silver)));
        update_draft(&mut p);
        assert_eq!(p.offer, first, "one draft at a time");
        // Each choice rerolls once, into one that wasn't shown, of the same tier.
        p.golden = 0;
        reroll(&mut p, 1);
        assert_eq!(p.rerolled, 0b010);
        assert_eq!((p.offer[0], p.offer[2]), (first[0], first[2]), "only that choice changes");
        assert!(!first.contains(&p.offer[1]), "a reroll shows a new choice");
        assert_eq!(augment(p.offer[1]).unwrap().tier, Tier::Silver);
        let again = p.offer;
        reroll(&mut p, 1);
        assert_eq!(p.offer, again, "one reroll per choice");
        reroll(&mut p, 0);
        reroll(&mut p, 2);
        assert_eq!(p.rerolled, 0b111, "three rerolls a draft, one per choice");
        let again = p.offer;
        pick(&mut p, 1);
        assert_eq!(p.augments, [again[1], 0, 0, 0]);
        assert_eq!(p.offer, [0; CHOICES]);
        update_draft(&mut p);
        assert_eq!(p.offer, [0; CHOICES], "the next draft waits for level 7");
        p.level = 11;
        update_draft(&mut p);
        assert_eq!(p.drafted, 2);
        pick(&mut p, 0);
        update_draft(&mut p);
        assert_eq!(p.drafted, 3, "a draft skipped by fast leveling opens right after");
        // The same seed always gives the same offers (prediction depends on it).
        let mut q = Progress { level: 3, augment_seed: 77, ..Progress::SANDBOX };
        update_draft(&mut q);
        assert_eq!(q.offer, first);
    }

    #[test]
    fn every_seed_gets_one_prismatic_draft() {
        for seed in 0..64 {
            let tiers: Vec<Tier> = (0..SLOTS as u8).map(|d| tier_of(seed, d)).collect();
            assert_eq!(tiers.iter().filter(|t| **t == Tier::Prismatic).count(), 1, "{tiers:?}");
            assert_eq!(tiers[0], Tier::Silver);
        }
    }
}
