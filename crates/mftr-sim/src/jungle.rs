//! The jungle (01 §7, M4 slice 3): neutral camps between the lanes. Monsters stand at their
//! camp until hurt; then the whole camp turns on the attacker, and gives up (walking home,
//! healing) once the fight drags it, or the attacker, too far from home. A camp's monsters pay
//! gold and experience to the champion who lands the killing blow. The two buff camps give a
//! timed buff too, and a big monster taken counts toward the killer's Claim upgrade.
//!
//! Everything here is server-side simulation (clients don't predict other units).

use crate::champion::{AttackSpec, Stats};
use crate::map::Map;
use crate::math::{QPoint, Vec2};
use crate::time::{SimDuration, SimTime};
use crate::world::{Brain, Order, SimEvent, Team, Unit, UnitId, UnitKind};

/// Camps first spawn this long after the match starts.
pub const FIRST_SPAWN: SimDuration = SimDuration::from_millis(90_000);
/// A buff camp comes back this long after it's cleared; the others sooner.
pub const BUFF_RESPAWN: SimDuration = SimDuration::from_millis(300_000);
pub const CAMP_RESPAWN: SimDuration = SimDuration::from_millis(135_000);
/// A monster gives up its fight when it or its target is this far from home.
pub const LEASH: f32 = 800.0;
/// Out of a fight, a monster heals this share of its health a second.
pub const RESET_HEAL: f32 = 0.25;

/// The buffs (01 §7): Insight (from the Warden) shortens cooldowns; Cinder (from the Brute)
/// makes basic attacks burn and slow.
pub const BUFF_DURATION: SimDuration = SimDuration::from_millis(120_000);
pub const INSIGHT_HASTE: f32 = 20.0;
/// Cinder's burn per basic attack: true damage, `base + per_level × level`.
pub const CINDER_BURN: (f32, f32) = (6.0, 2.0);
/// Cinder's slow on whatever it hits: this share for this long.
pub const CINDER_SLOW: u8 = 15;
pub const CINDER_SLOW_FOR: SimDuration = SimDuration::from_millis(1000);

/// Claim (the jungler's utility spell): its damage rises once its caster has taken this many
/// big monsters (camp leaders and lone monsters).
pub const CLAIM_UPGRADE_CAMPS: u8 = 5;
/// Claiming a monster heals the caster this much.
pub const CLAIM_HEAL: f32 = 100.0;

/// A jungle monster.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MonsterKind {
    /// The Azure Warden: the Insight buff.
    Warden = 0,
    /// The Ember Brute: the Cinder buff.
    Brute = 1,
    HoundAlpha = 2,
    Hound = 3,
    /// A lone monster.
    Toad = 4,
    RavenAlpha = 5,
    Raven = 6,
    CrawlerElder = 7,
    Crawler = 8,
}

/// What a monster is: its stats, its attack, what it pays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MonsterDef {
    pub name: &'static str,
    pub health: f32,
    pub resist: f32,
    pub damage: f32,
    pub attack: AttackSpec,
    pub gold: f32,
    pub xp: u32,
    /// Collision and gameplay radius.
    pub radius: (f32, f32),
    /// Camp leaders and lone monsters: they count toward Claim's upgrade.
    pub big: bool,
}

const fn melee(attack_speed: f32) -> AttackSpec {
    AttackSpec { range: 175.0, attack_speed, windup_fraction: 0.3, bolt_speed: 0.0 }
}

const fn ranged(range: f32, attack_speed: f32) -> AttackSpec {
    AttackSpec { range, attack_speed, windup_fraction: 0.3, bolt_speed: 900.0 }
}

impl MonsterKind {
    pub const ALL: [MonsterKind; 9] = [
        MonsterKind::Warden,
        MonsterKind::Brute,
        MonsterKind::HoundAlpha,
        MonsterKind::Hound,
        MonsterKind::Toad,
        MonsterKind::RavenAlpha,
        MonsterKind::Raven,
        MonsterKind::CrawlerElder,
        MonsterKind::Crawler,
    ];

    pub fn from_wire(v: u8) -> Option<Self> {
        Self::ALL.get(v as usize).copied()
    }

    pub fn def(self) -> MonsterDef {
        let d = |name, health, resist, damage, attack, gold, xp, radius, big| MonsterDef {
            name,
            health,
            resist,
            damage,
            attack,
            gold,
            xp,
            radius,
            big,
        };
        // *(start)* A level-1 champion clears a buff camp alone in about 40 s for about half
        // its health; a small camp a little faster.
        match self {
            MonsterKind::Warden => d("Azure Warden", 1600.0, 15.0, 19.0, melee(0.5), 90.0, 95, (90.0, 115.0), true),
            MonsterKind::Brute => d("Ember Brute", 1600.0, 15.0, 19.0, melee(0.5), 90.0, 95, (90.0, 115.0), true),
            MonsterKind::HoundAlpha => {
                d("Thicket Hound Alpha", 900.0, 10.0, 14.0, melee(0.6), 55.0, 75, (60.0, 80.0), true)
            }
            MonsterKind::Hound => d("Thicket Hound", 300.0, 0.0, 4.0, melee(0.6), 15.0, 15, (40.0, 55.0), false),
            MonsterKind::Toad => d("Bog Toad", 1300.0, 10.0, 16.0, ranged(400.0, 0.6), 80.0, 120, (80.0, 100.0), true),
            MonsterKind::RavenAlpha => {
                d("Ravenhawk Matriarch", 700.0, 10.0, 12.0, ranged(300.0, 0.65), 35.0, 40, (50.0, 65.0), true)
            }
            MonsterKind::Raven => d("Ravenhawk", 200.0, 0.0, 4.0, ranged(300.0, 0.65), 15.0, 10, (30.0, 40.0), false),
            MonsterKind::CrawlerElder => {
                d("Elder Stone Crawler", 1000.0, 25.0, 18.0, melee(0.6), 65.0, 85, (75.0, 95.0), true)
            }
            MonsterKind::Crawler => d("Stone Crawler", 450.0, 15.0, 10.0, melee(0.6), 20.0, 20, (45.0, 60.0), false),
        }
    }

    pub fn stats(self) -> Stats {
        let d = self.def();
        Stats {
            max_health: d.health,
            armor: d.resist,
            magic_resist: d.resist,
            attack_damage: d.damage,
            move_speed: 400.0,
            ..Stats::NONE
        }
    }
}

/// A camp: which monsters, where (offsets from the camp's center), how soon it returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CampKind {
    Warden,
    Brute,
    Hounds,
    Toad,
    Ravens,
    Crawlers,
}

impl CampKind {
    pub fn members(self) -> &'static [(MonsterKind, f32, f32)] {
        match self {
            CampKind::Warden => &[(MonsterKind::Warden, 0.0, 0.0)],
            CampKind::Brute => &[(MonsterKind::Brute, 0.0, 0.0)],
            CampKind::Hounds => &[
                (MonsterKind::HoundAlpha, 0.0, 0.0),
                (MonsterKind::Hound, -150.0, 110.0),
                (MonsterKind::Hound, 150.0, 110.0),
            ],
            CampKind::Toad => &[(MonsterKind::Toad, 0.0, 0.0)],
            CampKind::Ravens => &[
                (MonsterKind::RavenAlpha, 0.0, 0.0),
                (MonsterKind::Raven, -120.0, 100.0),
                (MonsterKind::Raven, 120.0, 100.0),
                (MonsterKind::Raven, 0.0, 170.0),
            ],
            CampKind::Crawlers => &[(MonsterKind::CrawlerElder, 0.0, 0.0), (MonsterKind::Crawler, 150.0, 100.0)],
        }
    }

    pub fn respawn(self) -> SimDuration {
        match self {
            CampKind::Warden | CampKind::Brute => BUFF_RESPAWN,
            _ => CAMP_RESPAWN,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            CampKind::Warden => "Azure Warden",
            CampKind::Brute => "Ember Brute",
            CampKind::Hounds => "Thicket Hounds",
            CampKind::Toad => "Bog Toad",
            CampKind::Ravens => "Ravenhawks",
            CampKind::Crawlers => "Stone Crawlers",
        }
    }
}

/// A camp on a map: its kind and center. Its monsters stand at the center plus their offsets,
/// turned to face `facing` (a unit vector: +y offsets point that way).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camp {
    pub kind: CampKind,
    pub pos: Vec2,
    pub facing: Vec2,
}

impl Camp {
    /// Where each member stands.
    pub fn spots(&self) -> Vec<(MonsterKind, Vec2)> {
        let (f, side) = (self.facing, Vec2::new(-self.facing.y, self.facing.x));
        self.kind.members().iter().map(|&(m, x, y)| (m, self.pos + side * x + f * y)).collect()
    }
}

/// Decide for a monster (every tick): fight its target while both stay within the leash of
/// home, else walk home; out of a fight, heal.
pub fn monster_think(unit: &mut Unit, seen: &[crate::lane::Seen], dt: f32, map: &Map) {
    let Some(Brain::Monster { home, .. }) = unit.brain else { return };
    let home = home.to_vec2();
    let max = unit.stats.max_health;
    let st = &mut unit.state;
    match st.order {
        Order::Attack(target) => {
            let lost =
                seen.iter().find(|s| s.id == target).is_none_or(|s| !s.targetable || s.pos.distance(home) > LEASH);
            if lost || st.pos.distance(home) > LEASH + 200.0 {
                st.attack = None;
                st.set_order(Order::MoveTo(QPoint::from_vec2(home)), map);
            }
        }
        Order::MoveTo(_) if st.pos.distance(home) < 40.0 => st.set_order(Order::Idle, map),
        _ => {}
    }
    if !matches!(st.order, Order::Attack(_)) && st.health < max {
        st.health = (st.health + max * RESET_HEAL * dt).min(max);
    }
}

/// This tick's hits on monsters: the whole camp turns on the attacker (one already fighting
/// keeps its target).
pub fn aggro(units: &mut [Unit], events: &[SimEvent], map: &Map) {
    for e in events {
        let SimEvent::Damage { source, target, .. } = *e else { continue };
        let camp = units.iter().find(|u| u.id == target).and_then(|u| match u.brain {
            Some(Brain::Monster { camp, .. }) => Some(camp),
            _ => None,
        });
        let Some(camp) = camp else { continue };
        if !units.iter().any(|u| u.id == source && u.kind != UnitKind::Monster && u.state.alive()) {
            continue;
        }
        for u in units.iter_mut() {
            if matches!(u.brain, Some(Brain::Monster { camp: c, .. }) if c == camp)
                && u.state.alive()
                && !matches!(u.state.order, Order::Attack(_))
            {
                u.state.set_order(Order::Attack(source), map);
            }
        }
    }
}

/// A monster unit for a camp member.
pub fn monster_unit(id: UnitId, kind: MonsterKind, camp: u8, pos: Vec2) -> Unit {
    let def = kind.def();
    let mut u = Unit::new(id, UnitKind::Monster, Team::Neutral, pos, def.radius, kind.stats());
    u.attack = Some(def.attack);
    u.monster = Some(kind);
    u.brain = Some(Brain::Monster { camp, home: QPoint::from_vec2(pos) });
    u.state.health = def.health;
    u
}

/// Cinder's burn for a champion of `level`.
pub fn cinder_burn(level: u8) -> f32 {
    CINDER_BURN.0 + CINDER_BURN.1 * level as f32
}

/// When a camp cleared at `at` comes back.
pub fn respawn_at(kind: CampKind, at: SimTime) -> SimTime {
    at.plus(kind.respawn())
}
