//! The simulation world: units, commands and the fixed-tick step.
//!
//! Tick phases (04 §3):
//! 0. Respawns that are due.
//! 1. AI decisions for server-driven units (patrols, turrets).
//! 2. A per-unit timeline on the exact [`SimTime`] axis: movement with unit collision against
//!    start-of-tick positions (`collision.rs`), sub-tick commands, cast and attack windups
//!    (rooted), dashes, blinks, stuns and roots, each taking effect at its exact instant.
//!    Attack targets are judged against start-of-tick positions too, so the outcome is
//!    order-independent and client prediction reproduces it from proxies.
//! 3. Effects: missiles spawn at their fire instant and resolve with exact swept hits
//!    (`projectile::first_contact`), delayed areas detonate, homing attack bolts fly. Damage goes
//!    through the 02 §5 pipeline (resistances, shields, health, death).
//! 4. Regeneration and shield expiry.
//!
//! In **prediction mode** (the client's world: own unit plus proxies) phase 3 doesn't run:
//! hits on others are never predicted (03a §7). Own missiles, areas and bolts are still
//! announced (id 0) so the client can draw them at once.

use crate::ability::{
    Ability, Cc, DamageKind, Effect, LUNGE_PICK, LineSkillshot, SLOTS, SUPPORT_PICK, TURRET_SHOT, Timing, Transforms,
};
use crate::augments;
use crate::champion::{AttackSpec, ChampionId, Stats};
use crate::collision::{Obstacle, choose_detour, constrained_move};
use crate::combat::resist_multiplier;
use crate::hash::{StateHasher, StateSink};
use crate::items::{self, INVENTORY};
use crate::lane::{self, MatchState, Seen};
use crate::map::{Map, MapId};
use crate::math::{QPoint, Vec2};
use crate::projectile::first_contact;
use crate::rng::Pcg32;
use crate::time::{SUBTICKS, SUBTICKS_PER_SECOND, SimDuration, SimTime, SubTick, TICK_DT, Tick};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(pub u8);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Team {
    Blue,
    Red,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnitKind {
    Champion,
    Minion,
    /// The dodge rig's skillshot shooter (M1). Immune to everything.
    RigTurret,
    /// Lane turret (01 §3): attacks with turret priorities, destroyed in lane order.
    Turret,
    /// Gatehouse: respawns; while it's down the other team's waves bring a super minion.
    Gatehouse,
    /// Destroying it wins the match.
    Base,
    /// Health relic: walk over it to heal; respawns on a timer.
    Relic,
}

impl UnitKind {
    /// Map structures and pickups: always visible, never move.
    pub fn is_structure(self) -> bool {
        matches!(self, UnitKind::RigTurret | UnitKind::Turret | UnitKind::Gatehouse | UnitKind::Base | UnitKind::Relic)
    }

    pub fn wire(self) -> u8 {
        self as u8
    }

    pub fn from_wire(v: u8) -> Option<Self> {
        Some(match v {
            0 => UnitKind::Champion,
            1 => UnitKind::Minion,
            2 => UnitKind::RigTurret,
            3 => UnitKind::Turret,
            4 => UnitKind::Gatehouse,
            5 => UnitKind::Base,
            6 => UnitKind::Relic,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MinionKind {
    Melee,
    Caster,
    Siege,
    /// Joins its team's waves while an enemy Gatehouse is down (01 §3): tanky, hits hard.
    Super,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Order {
    Idle,
    MoveTo(QPoint),
    /// Chase a unit until in attack range, then attack it on every attack timer.
    Attack(UnitId),
    /// Walk toward a point, attacking the nearest enemy that comes into range on the way.
    AttackMove(QPoint),
}

/// Ticks of poor progress before a blocked unit looks for a detour (03a §5).
const STUCK_TICKS: u8 = 3;
const STUCK_PROGRESS: f32 = 0.2;
/// Only units this close can matter for one tick of movement or a detour probe.
const BROADPHASE: f32 = 300.0;
/// A blocked unit this close to its goal counts as arrived.
const GIVE_UP_DISTANCE: f32 = 80.0;
/// Float slack for "in attack range" after halting at the range circle.
const RANGE_SLACK: f32 = 0.5;
/// An attack target that moved this far from the chase path's end gets a new path.
const REPATH_DISTANCE: f32 = 25.0;

/// A cast in progress: the caster is rooted until it fires.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cast {
    pub slot: u8,
    pub dir: Vec2,
    /// Target point (areas: already clamped to the ability's range).
    pub point: Vec2,
    pub fire_at: SimTime,
    /// Sequence number of the command that started it (0 for AI casts).
    pub seq: u32,
    /// The caster keeps moving while it winds up (`Timing::mobile`).
    pub mobile: bool,
}

/// After an ability fires (10 §4.1): rooted until `until`; nothing ends it before `hard_until`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Recovery {
    pub hard_until: SimTime,
    pub until: SimTime,
}

/// A cast ordered while another action was still busy (a windup, a dash, a hard lock): it
/// starts the moment that ends (10 §4.1, the input buffer). One slot; the latest wins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BufferedCast {
    pub slot: u8,
    pub target: QPoint,
    pub seq: u32,
}

/// A basic attack winding up: the attacker is rooted; the bolt launches at `fire_at`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackWindup {
    pub target: UnitId,
    pub fire_at: SimTime,
}

/// A dash in progress: constant speed toward `to`, ignoring units, sliding on walls.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DashMove {
    pub dir: Vec2,
    pub to: Vec2,
    pub speed: f32,
    pub end_at: SimTime,
    /// Lunges: the enemy struck on arrival, and the ability slot that strikes.
    pub strike: Option<(UnitId, u8)>,
    /// The landing's follow-through (10 §4.2; 0 for forced movement such as pulls).
    pub recover: SimDuration,
}

/// A champion's progression (M2): survives death and respawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Progress {
    /// 1–18.
    pub level: u8,
    /// Experience into the current level.
    pub xp: u32,
    pub gold: f32,
    /// Ranks of Q W E R (0 = not learned).
    pub ranks: [u8; 4],
    /// Unspent ability points.
    pub points: u8,
    /// Kill streak (> 0) or death streak (< 0), for bounties.
    pub streak: i8,
    /// Inventory (item ids, 0 = empty).
    pub items: [u8; INVENTORY],
    /// Charges of each slot's consumable (stacked potions, a flask's charges); 0 otherwise.
    pub charges: [u8; INVENTORY],
    /// When the Lifeline passive is ready again (item cooldowns survive death).
    pub lifeline_ready: SimTime,
    /// Trades that can still be undone, oldest first (`undo_len` valid). Cleared when the
    /// shop closes (01 §11: undo until you leave the fountain).
    pub undo: [Trade; UNDO],
    pub undo_len: u8,
    /// Augments held (ids, 0 = empty slot; ARAM: Mayhem, 06 §3).
    pub augments: [u8; augments::SLOTS],
    /// The open draft's choices (all 0 when none is open).
    pub offer: [u8; augments::CHOICES],
    /// Drafts opened so far (the open one included).
    pub drafted: u8,
    /// Which choices of the open draft were rerolled (bit per choice: each once).
    pub rerolled: u8,
    /// The choice (1–3) whose reroll is golden (one tier up) this draft; 0 for none.
    pub golden: u8,
    /// Seeds this champion's offers, so picks and rerolls are predicted exactly.
    pub augment_seed: u32,
    /// Unstable Experiment's current roll: tiny (else huge).
    pub unstable_tiny: bool,
    /// Spellhunger's ability power stacks.
    pub stacks: u16,
    /// Champion takedowns (kills and assists) this match: Champion of Chaos counts them.
    pub takedowns: u8,
    /// Plays under Hyper rules (set from the rules at spawn).
    pub hyper: bool,
    /// Stat Anvils (Mayhem): steps of each stat kept, the open anvil's choices (packed,
    /// `anvils::pack`; all 0 when none is open), and how many were bought.
    pub anvil: [u16; 8],
    pub anvil_offer: [u8; crate::anvils::CHOICES],
    pub anvils: u8,
    /// This match's score (the scoreboard): champion kills, deaths, assists, and minions
    /// killed (last hits).
    pub kills: u16,
    pub deaths: u16,
    pub assists: u16,
    pub cs: u16,
}

/// One buy or sell: the inventory before it and the gold it changed.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Trade {
    pub items: [u8; INVENTORY],
    pub charges: [u8; INVENTORY],
    pub gold: f32,
}

/// How many trades can be undone.
pub const UNDO: usize = 4;

/// Level, items, augments and what augments have built up: what a champion's stats (and size)
/// are computed from.
pub type StatsKey = (u8, [u8; INVENTORY], [u8; augments::SLOTS], augments::Growth);

impl Progress {
    /// Sandbox default: level 1, every ability at rank 1, nothing to spend.
    pub const SANDBOX: Progress = Progress {
        level: 1,
        xp: 0,
        gold: 0.0,
        ranks: [1; 4],
        points: 0,
        streak: 0,
        items: [0; INVENTORY],
        charges: [0; INVENTORY],
        lifeline_ready: SimTime(0),
        undo: [Trade { items: [0; INVENTORY], charges: [0; INVENTORY], gold: 0.0 }; UNDO],
        undo_len: 0,
        augments: [0; augments::SLOTS],
        offer: [0; augments::CHOICES],
        drafted: 0,
        rerolled: 0,
        golden: 0,
        augment_seed: 0,
        unstable_tiny: false,
        stacks: 0,
        takedowns: 0,
        hyper: false,
        anvil: [0; 8],
        anvil_offer: [0; crate::anvils::CHOICES],
        anvils: 0,
        kills: 0,
        deaths: 0,
        assists: 0,
        cs: 0,
    };

    pub fn hash_into(&self, h: &mut impl StateSink) {
        h.write_u8(self.level);
        h.write_u32(self.xp);
        h.write_f32(self.gold);
        for r in self.ranks {
            h.write_u8(r);
        }
        h.write_u8(self.points);
        h.write_u8(self.streak as u8);
        for i in self.items {
            h.write_u8(i);
        }
        for c in self.charges {
            h.write_u8(c);
        }
        h.write_u64(self.lifeline_ready.0);
        h.write_u8(self.undo_len);
        for t in &self.undo[..self.undo_len as usize] {
            for i in t.items {
                h.write_u8(i);
            }
            for c in t.charges {
                h.write_u8(c);
            }
            h.write_f32(t.gold);
        }
        for a in self.augments {
            h.write_u8(a);
        }
        for a in self.offer {
            h.write_u8(a);
        }
        h.write_u8(self.drafted);
        h.write_u8(self.rerolled);
        h.write_u8(self.golden);
        h.write_u32(self.augment_seed);
        h.write_u8(self.unstable_tiny as u8);
        h.write_u32(self.stacks as u32);
        h.write_u8(self.takedowns);
        h.write_u8(self.hyper as u8);
        for n in self.anvil {
            h.write_u32(n as u32);
        }
        for c in self.anvil_offer {
            h.write_u8(c);
        }
        h.write_u8(self.anvils);
        for n in [self.kills, self.deaths, self.assists, self.cs] {
            h.write_u32(n as u32);
        }
    }

    /// What a champion's stats are computed from.
    pub fn stats_key(&self) -> StatsKey {
        let growth = augments::Growth {
            unstable_tiny: self.unstable_tiny,
            stacks: self.stacks,
            chaos_done: self.takedowns >= augments::CHAOS_TAKEDOWNS,
            hyper: self.hyper,
            anvil: self.anvil,
        };
        (self.level, self.items, self.augments, growth)
    }

    fn push_trade(&mut self, t: Trade) {
        if self.undo_len as usize == UNDO {
            self.undo.copy_within(1.., 0);
            self.undo_len -= 1;
        }
        self.undo[self.undo_len as usize] = t;
        self.undo_len += 1;
    }
}

pub const MAX_LEVEL: u8 = 18;

/// Experience from `level` to the next (02/01 §6 *(start)*): 280 at level 1, +100 per level.
pub fn xp_to_next(level: u8) -> u32 {
    180 + 100 * level as u32
}

/// Match rules that prediction must know too (sent in the welcome).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rules {
    pub start_level: u8,
    pub start_gold: f32,
    /// Gold per second for every champion.
    pub passive_gold: f32,
    /// Ability ranks are learned with points (else every ability is rank 1).
    pub ranked: bool,
    /// Augment drafts (ARAM: Mayhem, 06 §3).
    pub augments: bool,
    /// Hyper (06 §2): basic abilities and attacks much faster.
    pub hyper: bool,
}

impl Rules {
    /// Sandboxes: level 1, every ability at rank 1, no economy.
    pub const SANDBOX: Rules =
        Rules { start_level: 1, start_gold: 0.0, passive_gold: 0.0, ranked: false, augments: false, hyper: false };
    /// ARAM (06 §2): a quick start and faster gold *(start values)*.
    pub const ARAM: Rules =
        Rules { start_level: 3, start_gold: 1400.0, passive_gold: 4.0, ranked: true, augments: false, hyper: false };
    /// ARAM: Mayhem (06 §2): ARAM with augment drafts.
    pub const MAYHEM: Rules = Rules { augments: true, ..Rules::ARAM };
    /// ARAM: Mayhem under Hyper rules: the stress mode (M3 exit).
    pub const HYPER: Rules = Rules { hyper: true, ..Rules::MAYHEM };

    pub fn progress(&self) -> Progress {
        if !self.ranked {
            return Progress { gold: self.start_gold, ..Progress::SANDBOX };
        }
        Progress {
            level: self.start_level,
            gold: self.start_gold,
            ranks: [0; 4],
            points: self.start_level,
            hyper: self.hyper,
            ..Progress::SANDBOX
        }
    }
}

/// Hyper (06 §2) *(start values)*: ability haste for Q, W and E, and bonus attack speed.
pub const HYPER_HASTE: f32 = 300.0;
pub const HYPER_ATTACK_SPEED: f32 = 0.5;

/// Most waypoints a path keeps; longer paths are re-planned when they run out.
pub const MAX_PATH: usize = 12;

/// An any-angle path toward the current move goal (from [`Map::find_path`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Path {
    pub points: [Vec2; MAX_PATH],
    pub len: u8,
    pub next: u8,
    /// The last point is the final destination (else the path was truncated).
    pub complete: bool,
}

impl Path {
    pub const EMPTY: Path = Path { points: [Vec2::ZERO; MAX_PATH], len: 0, next: 0, complete: true };

    fn from_points(points: &[Vec2]) -> Path {
        let mut p = Path::EMPTY;
        let n = points.len().min(MAX_PATH);
        p.points[..n].copy_from_slice(&points[..n]);
        p.len = n as u8;
        p.complete = points.len() <= MAX_PATH;
        p
    }

    pub fn waypoints(&self) -> &[Vec2] {
        &self.points[self.next as usize..self.len as usize]
    }

    fn goal(&self) -> Option<Vec2> {
        (self.len > 0).then(|| self.points[self.len as usize - 1])
    }
}

/// Everything client prediction needs to reproduce a unit bit-exactly (sent lossless for the
/// own champion, 03b §6).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UnitState {
    pub pos: Vec2,
    pub order: Order,
    pub move_speed: f32,
    /// Waypoints toward the order's goal, around walls.
    pub path: Path,
    /// Temporary waypoint around blocking units, taken before resuming the path.
    pub detour: Option<Vec2>,
    /// Consecutive ticks of poor progress while moving.
    pub stuck: u8,
    pub cast: Option<Cast>,
    pub attack: Option<AttackWindup>,
    /// When the next basic attack may start (the attack timer, 02 §7).
    pub attack_ready_at: SimTime,
    pub dash: Option<DashMove>,
    pub stunned_until: SimTime,
    pub rooted_until: SimTime,
    /// Movement slow in percent, active until `slowed_until`.
    pub slow: u8,
    pub slowed_until: SimTime,
    /// When each slot (Q W E R D F) is off cooldown.
    pub cooldowns: [SimTime; SLOTS],
    pub health: f32,
    /// Shield points (Barrier), valid until `shield_until`.
    pub shield: f32,
    pub shield_until: SimTime,
    /// Dead until this instant (then respawns at home).
    pub respawn_at: Option<SimTime>,
    pub progress: Progress,
    /// A cast that repeats at `at` (the Echo augment, M3).
    pub echo: Option<EchoCast>,
    /// Spellblade is armed until this instant.
    pub spellblade_until: SimTime,
    /// Unit vector the unit faces (10 §3): snaps to its path, its attack target and its aim.
    pub facing: Vec2,
    /// An ability's follow-through in progress.
    pub recovery: Option<Recovery>,
    /// The input buffer: a cast waiting for the current action to end.
    pub buffered: Option<BufferedCast>,
    /// Basic attacks started (wrapping): picks the attack animation, the same on every client.
    pub attacks: u8,
    /// A consumable healing over time: this much health a second until `potion_until`.
    pub potion_rate: f32,
    pub potion_until: SimTime,
}

/// A cast waiting to repeat (Echo): lines fly again from the caster's position then, areas land
/// on the same point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EchoCast {
    pub slot: u8,
    pub dir: Vec2,
    pub point: Vec2,
    pub at: SimTime,
    pub seq: u32,
}

impl UnitState {
    pub fn new(pos: Vec2, move_speed: f32) -> Self {
        Self {
            pos,
            order: Order::Idle,
            move_speed,
            path: Path::EMPTY,
            detour: None,
            stuck: 0,
            cast: None,
            attack: None,
            attack_ready_at: SimTime(0),
            dash: None,
            stunned_until: SimTime(0),
            rooted_until: SimTime(0),
            slow: 0,
            slowed_until: SimTime(0),
            cooldowns: [SimTime(0); SLOTS],
            health: 0.0,
            shield: 0.0,
            shield_until: SimTime(0),
            respawn_at: None,
            progress: Progress::SANDBOX,
            echo: None,
            spellblade_until: SimTime(0),
            facing: Vec2::new(1.0, 0.0),
            recovery: None,
            buffered: None,
            attacks: 0,
            potion_rate: 0.0,
            potion_until: SimTime(0),
        }
    }

    pub fn alive(&self) -> bool {
        self.respawn_at.is_none()
    }

    /// Where the unit is currently heading (detour first, then the path), if anywhere.
    pub fn heading(&self) -> Option<Vec2> {
        if let Some(d) = self.detour {
            return Some(d);
        }
        match self.order {
            Order::Idle => None,
            _ => self.path.waypoints().first().copied(),
        }
    }

    /// Replace the order (a command or AI decision) and plan the route on `map`.
    pub fn set_order(&mut self, order: Order, map: &Map) {
        self.order = order;
        self.detour = None;
        self.stuck = 0;
        self.route(map);
    }

    /// (Re)plan the path from the current position toward the order's goal. Attack orders
    /// plan toward their target's position when the timeline next looks at it.
    fn route(&mut self, map: &Map) {
        self.path = match self.order {
            Order::MoveTo(q) | Order::AttackMove(q) => Path::from_points(&map.find_path(self.pos, q.to_vec2())),
            Order::Idle | Order::Attack(_) => Path::EMPTY,
        };
        if matches!(self.order, Order::MoveTo(_) | Order::AttackMove(_)) && self.path.len == 0 {
            self.order = Order::Idle; // unreachable
        }
    }

    /// Movement speed at `at`, after any slow (then the soft caps and floor, 02 §9).
    pub fn speed_at(&self, at: SimTime) -> f32 {
        if self.slow > 0 && self.slowed_until > at {
            crate::combat::soft_capped_move_speed(self.move_speed * (1.0 - self.slow as f32 / 100.0))
        } else {
            self.move_speed
        }
    }

    /// Apply a slow: the strongest one applies; an equal one extends it (02 §9).
    pub fn apply_slow(&mut self, pct: u8, until: SimTime, at: SimTime) {
        if self.slowed_until <= at || pct > self.slow {
            (self.slow, self.slowed_until) = (pct, until);
        } else if pct == self.slow {
            self.slowed_until = self.slowed_until.max(until);
        }
    }

    pub fn can_move(&self, at: SimTime) -> bool {
        self.alive()
            && self.cast.is_none_or(|c| c.mobile)
            && self.recovery.is_none_or(|r| r.until <= at)
            && self.attack.is_none()
            && self.dash.is_none()
            && self.stunned_until <= at
            && self.rooted_until <= at
    }

    /// Whether `slot` could be cast at `at` (alive, not stunned, not busy, off cooldown).
    pub fn can_cast(&self, at: SimTime, slot: u8) -> bool {
        let learned = self.progress.ranks.get(slot as usize).is_none_or(|r| *r > 0);
        learned
            && self.alive()
            && self.cast.is_none()
            && self.dash.is_none()
            && self.stunned_until <= at
            && self.cooldowns.get(slot as usize).is_some_and(|c| *c <= at)
    }

    /// An attack windup cancelled by a new order resets the attack timer, with no damage (02 §7).
    fn cancel_attack(&mut self, at: SimTime) {
        if self.attack.take().is_some() {
            self.attack_ready_at = at;
        }
    }

    /// A move or stop ends the soft part of a follow-through (the hard lock holds).
    fn end_soft_recovery(&mut self, at: SimTime) {
        if !self.hard_locked(at) {
            self.recovery = None;
        }
    }

    /// Inside a follow-through's hard lock at `at`.
    pub fn hard_locked(&self, at: SimTime) -> bool {
        self.recovery.is_some_and(|r| r.hard_until > at)
    }

    /// Inside a follow-through at `at` (soft or hard).
    pub fn recovering(&self, at: SimTime) -> bool {
        self.recovery.is_some_and(|r| r.until > at)
    }

    /// Enter an action's follow-through at `t` (skipped at once by a unit that's walking on).
    fn start_recovery(&mut self, timing: Timing, t: SimTime) {
        if timing.follow_through.0 == 0 {
            return;
        }
        self.recovery = Some(Recovery { hard_until: t.plus(timing.hard_lock), until: t.plus(timing.follow_through) });
        self.settle_recovery(t);
    }

    /// Face along `dir` (ignored when zero).
    fn face(&mut self, dir: Vec2) {
        let n = dir.normalize_or_zero();
        if n != Vec2::ZERO {
            self.facing = n;
        }
    }

    /// The soft part of a follow-through ends when the unit has somewhere to go (a move or
    /// attack-move order with a heading): an active player never waits for it (10 §4.1).
    fn settle_recovery(&mut self, at: SimTime) {
        if let Some(r) = self.recovery
            && (r.until <= at
                || (r.hard_until <= at
                    && matches!(self.order, Order::MoveTo(_) | Order::AttackMove(_))
                    && self.heading().is_some()))
        {
            self.recovery = None;
        }
    }

    /// Advance by `dt` seconds toward the current heading at constant speed (instant turns,
    /// R01 §2), blocked by `obstacles` and the map's walls. A `halt` circle (attack range
    /// around a target) stops the move where it enters the circle. Returns
    /// `(desired, achieved, fraction of dt used)`.
    pub fn advance(
        &mut self,
        dt: f32,
        radius: f32,
        obstacles: &[Obstacle],
        map: &Map,
        halt: Option<(Vec2, f32)>,
        at: SimTime,
    ) -> (f32, f32, f32) {
        if dt <= 0.0 {
            return (0.0, 0.0, 1.0);
        }
        let Some(target) = self.heading() else { return (0.0, 0.0, 1.0) };
        if let Some((c, reach)) = halt
            && (self.pos - c).length_sq() <= reach * reach
        {
            return (0.0, 0.0, 1.0);
        }
        let to = target - self.pos;
        let dist = to.length();
        self.face(to);
        let step = self.speed_at(at) * dt;
        let (mut delta, mut arrives) = if step >= dist { (to, true) } else { (to * (step / dist), false) };
        let mut used = 1.0;
        if let Some((c, reach)) = halt
            && let Some(tau) = first_contact(self.pos, self.pos + delta, c, c, reach)
            && tau < 1.0
        {
            delta = delta * tau;
            arrives = false;
            used = tau;
        }
        let new_pos = constrained_move(self.pos, delta, radius, obstacles, map.edges());
        let achieved = (new_pos - self.pos).length();
        self.pos = new_pos;
        if arrives && new_pos == target {
            self.stuck = 0;
            if self.detour.is_some() {
                // Off the planned path now: re-plan from here.
                self.detour = None;
                self.route(map);
            } else {
                self.path.next += 1;
                if self.path.next >= self.path.len {
                    if matches!(self.order, Order::Attack(_)) {
                        self.path = Path::EMPTY; // the target moved on: re-plan toward it
                    } else if self.path.complete {
                        self.order = Order::Idle;
                        self.path = Path::EMPTY;
                    } else {
                        self.route(map);
                    }
                }
            }
        }
        (delta.length(), achieved, used)
    }

    /// One dash segment of `dt` seconds.
    fn dash_advance(&mut self, dt: f32, radius: f32, map: &Map) {
        let Some(d) = self.dash else { return };
        let rest = d.to - self.pos;
        let len = rest.length();
        let step = d.speed * dt;
        let delta = if step >= len { rest } else { d.dir * step };
        self.pos = constrained_move(self.pos, delta, radius, &[], map.edges());
    }

    /// Bit-exact equality: what prediction reconciliation compares.
    pub fn bits_eq(&self, other: &Self) -> bool {
        let (mut a, mut b) = (Vec::with_capacity(256), Vec::with_capacity(256));
        self.hash_into(&mut a);
        other.hash_into(&mut b);
        a == b
    }

    /// Serialize every field (state hash and [`Self::bits_eq`]).
    pub fn hash_into(&self, h: &mut impl StateSink) {
        h.write_f32(self.pos.x);
        h.write_f32(self.pos.y);
        h.write_f32(self.move_speed);
        match self.order {
            Order::Idle => h.write_u8(0),
            Order::MoveTo(q) => {
                h.write_u8(1);
                h.write_u16(q.x);
                h.write_u16(q.y);
            }
            Order::Attack(id) => {
                h.write_u8(2);
                h.write_u32(id.0);
            }
            Order::AttackMove(q) => {
                h.write_u8(3);
                h.write_u16(q.x);
                h.write_u16(q.y);
            }
        }
        h.write_u8(self.path.len);
        h.write_u8(self.path.next);
        h.write_u8(self.path.complete as u8);
        for p in &self.path.points[..self.path.len as usize] {
            h.write_f32(p.x);
            h.write_f32(p.y);
        }
        match self.detour {
            None => h.write_u8(0),
            Some(d) => {
                h.write_u8(1);
                h.write_f32(d.x);
                h.write_f32(d.y);
            }
        }
        h.write_u8(self.stuck);
        match self.cast {
            None => h.write_u8(0),
            Some(c) => {
                h.write_u8(1);
                h.write_u8(c.slot);
                h.write_f32(c.dir.x);
                h.write_f32(c.dir.y);
                h.write_f32(c.point.x);
                h.write_f32(c.point.y);
                h.write_u64(c.fire_at.0);
                h.write_u32(c.seq);
                h.write_u8(c.mobile as u8);
            }
        }
        match self.attack {
            None => h.write_u8(0),
            Some(a) => {
                h.write_u8(1);
                h.write_u32(a.target.0);
                h.write_u64(a.fire_at.0);
            }
        }
        h.write_u64(self.attack_ready_at.0);
        match self.dash {
            None => h.write_u8(0),
            Some(d) => {
                h.write_u8(1);
                h.write_f32(d.dir.x);
                h.write_f32(d.dir.y);
                h.write_f32(d.to.x);
                h.write_f32(d.to.y);
                h.write_f32(d.speed);
                h.write_u64(d.end_at.0);
                h.write_u64(d.recover.0);
                match d.strike {
                    None => h.write_u8(0),
                    Some((id, slot)) => {
                        h.write_u8(1);
                        h.write_u32(id.0);
                        h.write_u8(slot);
                    }
                }
            }
        }
        h.write_u64(self.stunned_until.0);
        h.write_u64(self.rooted_until.0);
        h.write_u8(self.slow);
        h.write_u64(self.slowed_until.0);
        for c in self.cooldowns {
            h.write_u64(c.0);
        }
        h.write_f32(self.health);
        h.write_f32(self.shield);
        h.write_u64(self.shield_until.0);
        match self.respawn_at {
            None => h.write_u8(0),
            Some(t) => {
                h.write_u8(1);
                h.write_u64(t.0);
            }
        }
        self.progress.hash_into(h);
        match self.echo {
            None => h.write_u8(0),
            Some(e) => {
                h.write_u8(1);
                h.write_u8(e.slot);
                h.write_f32(e.dir.x);
                h.write_f32(e.dir.y);
                h.write_f32(e.point.x);
                h.write_f32(e.point.y);
                h.write_u64(e.at.0);
                h.write_u32(e.seq);
            }
        }
        h.write_u64(self.spellblade_until.0);
        h.write_f32(self.facing.x);
        h.write_f32(self.facing.y);
        match self.recovery {
            None => h.write_u8(0),
            Some(r) => {
                h.write_u8(1);
                h.write_u64(r.hard_until.0);
                h.write_u64(r.until.0);
            }
        }
        match self.buffered {
            None => h.write_u8(0),
            Some(b) => {
                h.write_u8(1);
                h.write_u8(b.slot);
                h.write_u16(b.target.x);
                h.write_u16(b.target.y);
                h.write_u32(b.seq);
            }
        }
        h.write_u8(self.attacks);
        h.write_f32(self.potion_rate);
        h.write_u64(self.potion_until.0);
    }
}

/// Server-side decision making for units without a player.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Brain {
    /// Walk back and forth between two points (minion-dummy waves, M1 sandbox).
    Patrol { a: QPoint, b: QPoint, toward_b: bool },
    /// Fire the turret shot at the nearest enemy champion in range (the dodge rig, 03 §14).
    /// Half the shots aim at the target's position, half lead its movement.
    RigTurret { range: u16 },
    /// Lane minion: walk the team's lane, fight what it meets (01 §4).
    Laner { lane: u8, next: u8 },
    /// Lane turret (01 §3): keeps its target while valid; `heat` counts consecutive shots at
    /// champions (each one hits harder) until it cools at `cools_at`.
    Tower { heat: u8, cools_at: SimTime },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Unit {
    pub id: UnitId,
    pub kind: UnitKind,
    pub owner: Option<PlayerId>,
    pub team: Team,
    pub state: UnitState,
    /// Unit-vs-unit movement blocking (D11). Zero = doesn't collide.
    pub collision_radius: f32,
    /// Hitbox for abilities and targeting.
    pub gameplay_radius: f32,
    pub brain: Option<Brain>,
    pub champion: Option<ChampionId>,
    pub stats: Stats,
    /// Respawn point.
    pub home: Vec2,
    /// Basic attack (champions, lane minions, turrets).
    pub attack: Option<AttackSpec>,
    /// Structures: destruction order within the team (01 §3); 0 for everything else.
    pub tier: u8,
    /// A structure behind one that still stands: can't be hurt. Recomputed every tick.
    pub protected: bool,
    /// The lane a lane structure stands in (`Placement::lane`).
    pub lane: u8,
    /// Damage from champions is multiplied by this (structures, late in a match: D55).
    /// Recomputed every tick; 1 for everything else.
    pub champion_damage_taken: f32,
    /// The level, items and augments `stats` and `attack` were computed for (champions:
    /// recomputed when any of them changes).
    pub stats_for: StatsKey,
}

impl Unit {
    /// A unit with no brain, owner or champion, at full health.
    pub fn new(id: UnitId, kind: UnitKind, team: Team, pos: Vec2, radii: (f32, f32), stats: Stats) -> Unit {
        let mut state = UnitState::new(pos, stats.move_speed);
        state.health = stats.max_health;
        Unit {
            id,
            kind,
            owner: None,
            team,
            state,
            collision_radius: radii.0,
            gameplay_radius: radii.1,
            brain: None,
            champion: None,
            stats,
            home: pos,
            attack: None,
            tier: 0,
            protected: false,
            lane: 0,
            champion_damage_taken: 1.0,
            stats_for: (1, [0; INVENTORY], [0; augments::SLOTS], augments::Growth::NONE),
        }
    }

    /// The ability in `slot`: a champion's kit and utility spells, or the turret's shot.
    pub fn ability(&self, slot: u8) -> Option<Ability> {
        if slot == 5
            && self.champion.is_some()
            && let Some(spell) = augments::spell(&self.state.progress.augments)
        {
            return Some(spell);
        }
        match (self.champion, self.kind) {
            (Some(c), _) => c.ability(slot),
            (None, UnitKind::RigTurret) if slot == 0 => Some(TURRET_SHOT),
            _ => None,
        }
    }

    pub fn attack_spec(&self) -> Option<AttackSpec> {
        self.attack
    }

    /// Champions: recompute stats and attack from the level and items in the progression
    /// state (02 §10 stat stack), if they changed. Current health rises with max health and
    /// is capped by it. Returns whether anything was recomputed.
    pub fn refresh_stats(&mut self) -> bool {
        let Some(c) = self.champion else { return false };
        let key = self.state.progress.stats_key();
        if key == self.stats_for {
            return false;
        }
        let (stats, attack) = items::champion_stats(c.def(), &key);
        self.gameplay_radius = CHAMPION_GAMEPLAY_RADIUS * augments::scale(augments::form(&key.2, key.3.unstable_tiny));
        if self.state.alive() {
            self.state.health =
                (self.state.health + (stats.max_health - self.stats.max_health).max(0.0)).min(stats.max_health);
        }
        self.stats = stats;
        self.attack = Some(attack);
        self.state.move_speed = stats.move_speed;
        self.stats_for = key;
        true
    }

    /// Recompute stats for the progression state now, without touching health (prediction
    /// re-syncs, spawns).
    pub fn reset_stats(&mut self) {
        let Some(c) = self.champion else { return };
        let key = self.state.progress.stats_key();
        let (stats, attack) = items::champion_stats(c.def(), &key);
        self.gameplay_radius = CHAMPION_GAMEPLAY_RADIUS * augments::scale(augments::form(&key.2, key.3.unstable_tiny));
        self.stats = stats;
        self.attack = Some(attack);
        self.stats_for = key;
    }

    /// Can be hit by skillshots, areas and attacks: alive, not a protected structure, not the
    /// dodge rig's shooter or a relic.
    pub fn targetable(&self) -> bool {
        !matches!(self.kind, UnitKind::RigTurret | UnitKind::Relic) && self.state.alive() && !self.protected
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CommandKind {
    MoveTo(QPoint),
    Stop,
    /// Cast the ability in `slot` (0–5 = Q W E R D F) toward a ground point.
    Cast {
        slot: u8,
        target: QPoint,
    },
    /// Basic-attack a unit (chasing it into range).
    Attack(UnitId),
    AttackMove(QPoint),
    /// Spend an ability point on slot 0–3 (Q W E R).
    LevelUp(u8),
    /// Buy an item by id (recipes consume their components).
    Buy(u8),
    /// Sell the item in an inventory slot (0–5).
    Sell(u8),
    /// Undo the last buy or sell while the shop is still open.
    Undo,
    /// Keep choice 0–2 of the open augment draft.
    PickAugment(u8),
    /// Replace choice 0–2 of the open draft (each once per draft).
    RerollAugment(u8),
    /// Use the active of the item in an inventory slot (0–5): drink a potion.
    UseItem(u8),
    /// Buy a Stat Anvil (Mayhem, level 9+, 750 gold, while shopping): it offers three stats.
    BuyAnvil,
    /// Keep choice 0–2 of the open anvil.
    PickAnvil(u8),
}

/// A player command, applied at `tick` at sub-tick position `sub` (03a §3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Command {
    pub player: PlayerId,
    pub seq: u32,
    pub tick: Tick,
    pub sub: SubTick,
    pub kind: CommandKind,
}

/// An analytic line missile (03a §6): position is a closed-form function of these fields.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Missile {
    pub id: u32,
    pub owner: UnitId,
    pub team: Team,
    pub origin: Vec2,
    pub dir: Vec2,
    pub spec: LineSkillshot,
    pub spawn_at: SimTime,
    /// Sequence number of the cast command (lets the caster's client match its prediction).
    pub cast_seq: u32,
    /// Raw damage, from the caster's stats at fire time.
    pub power: f32,
    /// Index within the cast's volley (Multishot), plus `augments::ECHO_SHOT` for an echo.
    pub shot: u8,
    /// The ability slot that fired it (0–5).
    pub slot: u8,
}

impl Missile {
    pub fn end_at(&self) -> SimTime {
        let life = (self.spec.range / self.spec.speed * SUBTICKS_PER_SECOND as f32) as u64;
        SimTime(self.spawn_at.0 + life)
    }

    /// Center position at `t`, clamped to the missile's lifetime.
    pub fn position_at(&self, t: SimTime) -> Vec2 {
        let s = t.secs_since(self.spawn_at).min(self.spec.range / self.spec.speed);
        self.origin + self.dir * (self.spec.speed * s)
    }

    /// Earliest contact in `[a, b]` with a target moving linearly from `q0` (at `s0`) to `q1`
    /// (at `s0 + 1 tick`). The exact rule the server uses, shared with client display code.
    pub fn first_hit(&self, a: SimTime, b: SimTime, s0: SimTime, q0: Vec2, q1: Vec2, radius: f32) -> Option<SimTime> {
        if a > b {
            return None;
        }
        let at = |t: SimTime| q0.lerp(q1, (t.0 - s0.0) as f32 / SUBTICKS as f32);
        let tau = first_contact(self.position_at(a), self.position_at(b), at(a), at(b), self.spec.radius + radius)?;
        Some(SimTime(a.0 + (tau * (b.0 - a.0) as f32) as u64))
    }
}

/// A delayed ground AoE on its way to detonation (telegraphed from `spawn_at`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub id: u32,
    pub owner: UnitId,
    pub team: Team,
    pub center: Vec2,
    pub radius: f32,
    pub spawn_at: SimTime,
    pub detonate_at: SimTime,
    pub kind: DamageKind,
    pub power: f32,
    pub cast_seq: u32,
    pub cc: Cc,
    /// 0, or `augments::ECHO_SHOT` for an echo.
    pub shot: u8,
    /// The ability slot that cast it (0–5).
    pub slot: u8,
}

/// A homing basic-attack bolt: not dodgeable, flies at the target until it lands.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bolt {
    pub id: u32,
    pub owner: UnitId,
    pub team: Team,
    pub target: UnitId,
    pub origin: Vec2,
    pub pos: Vec2,
    pub speed: f32,
    pub launched_at: SimTime,
    pub power: f32,
    /// Physical for attacks; true for a turret's share-of-health shot at a minion.
    pub kind: DamageKind,
}

/// A target a volley (owner, cast, echo or not) already hit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VolleyHit {
    pub owner: UnitId,
    pub cast_seq: u32,
    pub echo: bool,
    pub target: UnitId,
}

impl Missile {
    /// Which volley the missile belongs to: a volley hits each unit once.
    pub fn volley(&self) -> (UnitId, u32, bool) {
        (self.owner, self.cast_seq, self.shot >= augments::ECHO_SHOT)
    }
}

/// What dealt a hit: death recaps and stats say exactly which attack, ability or effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageOrigin {
    /// A basic attack (a champion's, a minion's or a structure's: the source unit says).
    Attack,
    /// An ability by slot (0–5 = Q W E R D F).
    Ability(u8),
    /// An item's effect (on-hit damage), by item id.
    Item(u8),
    /// An augment's effect (Thorns), by augment id.
    Augment(u8),
    /// An enemy fountain.
    Fountain,
}

/// Things that happened during a step, for the network layer and the client display.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SimEvent {
    CastStarted {
        unit: UnitId,
        slot: u8,
        at: SimTime,
        dir: Vec2,
        point: Vec2,
        fire_at: SimTime,
        seq: u32,
    },
    /// In a world in prediction mode the id is 0.
    MissileSpawned(Missile),
    MissileHit {
        id: u32,
        target: UnitId,
        at: SimTime,
    },
    MissileExpired {
        id: u32,
        at: SimTime,
    },
    AreaSpawned(Area),
    AreaDetonated {
        id: u32,
        at: SimTime,
    },
    AttackLaunched(Bolt),
    /// `hit` is false when the target died or vanished first.
    AttackLanded {
        id: u32,
        target: UnitId,
        at: SimTime,
        hit: bool,
    },
    /// Post-mitigation damage: `amount` reached health, `absorbed` was taken by shields.
    Damage {
        source: UnitId,
        target: UnitId,
        /// What hit: an attack, which ability, an item or augment effect.
        origin: DamageOrigin,
        kind: DamageKind,
        amount: f32,
        absorbed: f32,
        at: SimTime,
    },
    Died {
        unit: UnitId,
        killer: UnitId,
        at: SimTime,
        respawn_at: SimTime,
    },
    Respawned {
        unit: UnitId,
        pos: Vec2,
        at: SimTime,
    },
    Blinked {
        unit: UnitId,
        from: Vec2,
        to: Vec2,
        at: SimTime,
    },
    Dashed {
        unit: UnitId,
        from: Vec2,
        to: Vec2,
        at: SimTime,
        end_at: SimTime,
    },
    Shielded {
        unit: UnitId,
        amount: f32,
        at: SimTime,
        until: SimTime,
    },
    /// Health restored by a relic or the fountain (relics only: the fountain is continuous).
    Healed {
        unit: UnitId,
        amount: f32,
        at: SimTime,
    },
    /// Gold and experience earned (sent to the earner only).
    Reward {
        unit: UnitId,
        gold: f32,
        xp: u32,
        at: SimTime,
    },
    /// A Base fell: the match is over.
    MatchEnded {
        winner: Team,
        at: SimTime,
    },
}

pub const CHAMPION_MOVE_SPEED: f32 = 325.0;
pub const CHAMPION_COLLISION_RADIUS: f32 = 35.0;
pub const CHAMPION_GAMEPLAY_RADIUS: f32 = 65.0;
pub const MINION_MOVE_SPEED: f32 = 325.0;
pub const TURRET_COLLISION_RADIUS: f32 = 60.0;
pub const TURRET_GAMEPLAY_RADIUS: f32 = 80.0;
/// Champion respawn at level 1 (the Duel Sandbox, where champions stay level 1): 6 s.
pub const CHAMPION_RESPAWN: SimDuration = SimDuration::from_millis(6000);
/// Respawn by level from 2 (D55, the reference ARAM's timers): 13 s at level 2 up to 40 s at
/// 16, then 2 s more per level.
const RESPAWN_S: [u8; 16] = [11, 13, 15, 17, 19, 21, 22, 24, 26, 28, 30, 32, 34, 36, 38, 40];

pub fn respawn_time(level: u8) -> SimDuration {
    if level <= 1 {
        return CHAMPION_RESPAWN;
    }
    let i = level as usize - 1;
    let s = RESPAWN_S.get(i).copied().map_or(40 + 2 * (i as u64 + 1 - RESPAWN_S.len() as u64), u64::from);
    SimDuration::from_millis(s * 1000)
}
pub const MINION_RESPAWN: SimDuration = SimDuration::from_millis(12_000);

impl MinionKind {
    pub fn from_attack_range(range: f32) -> MinionKind {
        if range >= 500.0 {
            MinionKind::Caster
        } else if range >= 200.0 {
            MinionKind::Siege
        } else if range >= 150.0 {
            MinionKind::Super
        } else {
            MinionKind::Melee
        }
    }

    /// (collision radius, gameplay radius), 01 §4 *(start)* values.
    pub fn radii(self) -> (f32, f32) {
        match self {
            MinionKind::Melee | MinionKind::Caster => (25.0, 48.0),
            MinionKind::Siege => (35.0, 65.0),
            MinionKind::Super => (45.0, 80.0),
        }
    }

    pub fn stats(self) -> Stats {
        let (max_health, armor) = match self {
            MinionKind::Melee => (480.0, 0.0),
            MinionKind::Caster => (300.0, 0.0),
            MinionKind::Siege => (900.0, 15.0),
            MinionKind::Super => (1500.0, 100.0),
        };
        Stats {
            max_health,
            armor,
            move_speed: MINION_MOVE_SPEED,
            attack_damage: lane::minion_damage(self),
            ..Stats::NONE
        }
    }
}

/// A unit as seen by attackers and acquisition during phase 2: its start-of-tick state.
#[derive(Clone, Copy, Debug)]
struct Target {
    id: UnitId,
    team: Team,
    pos: Vec2,
    radius: f32,
    kind: UnitKind,
    /// Attack range (tells minion types apart for turret shots).
    range: f32,
    max_health: f32,
}

/// Something that fired during phase 2, created in phase 3.
enum Fired {
    Missile(Missile),
    Area(Area),
    Bolt(Bolt),
    /// A melee hit, landing at the end of the windup.
    Melee {
        owner: UnitId,
        target: UnitId,
        power: f32,
        at: SimTime,
    },
    /// A lunge arriving at its target.
    Strike {
        owner: UnitId,
        slot: u8,
        target: UnitId,
        power: f32,
        kind: DamageKind,
        cc: Cc,
        at: SimTime,
    },
    /// A heal and/or shield on an ally (on the caster itself it applies at once, predicted).
    Support {
        owner: UnitId,
        target: UnitId,
        heal: f32,
        heal_missing: f32,
        shield: f32,
        until: SimTime,
        at: SimTime,
    },
}

#[derive(Clone, Debug)]
pub struct World {
    tick: Tick,
    units: Vec<Unit>,
    next_unit: u32,
    rng: Pcg32,
    missiles: Vec<Missile>,
    areas: Vec<Area>,
    bolts: Vec<Bolt>,
    map: Arc<Map>,
    /// Id counter shared by missiles, areas and bolts.
    next_missile: u32,
    /// Targets each live Multishot volley has hit (one hit per volley and target).
    struck: Vec<VolleyHit>,
    prediction: bool,
    events: Vec<SimEvent>,
    /// Per team (blue, red): enemy units it can't see, which its units can't target with
    /// attack orders or attack-move. Set by the server from its vision every tick; client
    /// prediction only knows visible units anyway.
    hidden: [Vec<UnitId>; 2],
    /// The match on a lane map (waves, aggression, winner); idle on sandbox maps.
    game: MatchState,
    rules: Rules,
}

impl World {
    pub fn new(seed: u64) -> Self {
        Self {
            tick: Tick(0),
            units: Vec::new(),
            next_unit: 1,
            rng: Pcg32::new(seed, 0x4d46_5452),
            missiles: Vec::new(),
            areas: Vec::new(),
            bolts: Vec::new(),
            next_missile: 1,
            struck: Vec::new(),
            prediction: false,
            events: Vec::new(),
            hidden: [Vec::new(), Vec::new()],
            game: MatchState::default(),
            rules: Rules::SANDBOX,
            map: MapId::Open.shared(),
        }
    }

    /// The last simulated tick.
    pub fn tick(&self) -> Tick {
        self.tick
    }

    pub fn rng(&mut self) -> &mut Pcg32 {
        &mut self.rng
    }

    pub fn units(&self) -> &[Unit] {
        &self.units
    }

    pub fn unit(&self, id: UnitId) -> Option<&Unit> {
        self.units.iter().find(|u| u.id == id)
    }

    pub fn unit_mut(&mut self, id: UnitId) -> Option<&mut Unit> {
        self.units.iter_mut().find(|u| u.id == id)
    }

    pub fn missiles(&self) -> &[Missile] {
        &self.missiles
    }

    pub fn areas(&self) -> &[Area] {
        &self.areas
    }

    pub fn bolts(&self) -> &[Bolt] {
        &self.bolts
    }

    /// Client prediction mode: no effects on others are simulated (03a §7), but own casts
    /// still emit `MissileSpawned` / `AreaSpawned` / `AttackLaunched` (id 0) for drawing.
    pub fn set_prediction_mode(&mut self, prediction: bool) {
        self.prediction = prediction;
        if prediction {
            self.missiles.clear();
            self.areas.clear();
            self.bolts.clear();
        }
    }

    /// Events produced since the last call.
    pub fn take_events(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
    }

    fn next_id(&mut self) -> UnitId {
        let id = UnitId(self.next_unit);
        self.next_unit += 1;
        id
    }

    pub fn spawn_champion(&mut self, owner: PlayerId, team: Team, champion: ChampionId, pos: Vec2) -> UnitId {
        let id = self.next_id();
        let mut progress = self.rules.progress();
        progress.augment_seed = self.rng.next_u32();
        let stats = champion.def().stats_at(progress.level);
        let mut u = Unit::champion(id, owner, team, champion, pos, pos, stats);
        u.state.progress = progress;
        u.reset_stats();
        self.units.push(u);
        id
    }

    pub fn spawn_minion(&mut self, kind: MinionKind, team: Team, pos: Vec2, brain: Option<Brain>) -> UnitId {
        let id = self.next_id();
        let mut u = Unit::new(id, UnitKind::Minion, team, pos, kind.radii(), kind.stats());
        u.brain = brain;
        u.attack = Some(lane::minion_attack(kind));
        self.units.push(u);
        id
    }

    pub fn spawn_rig_turret(&mut self, team: Team, pos: Vec2, range: u16) -> UnitId {
        let id = self.next_id();
        let mut u = Unit::new(
            id,
            UnitKind::RigTurret,
            team,
            pos,
            (TURRET_COLLISION_RADIUS, TURRET_GAMEPLAY_RADIUS),
            Stats::NONE,
        );
        u.brain = Some(Brain::RigTurret { range });
        self.units.push(u);
        id
    }

    /// A structure or relic from a map layout (01 §3).
    pub fn spawn_placement(&mut self, p: crate::map::Placement) -> UnitId {
        let id = self.next_id();
        let (radii, stats) = match p.kind {
            UnitKind::Turret => ((TURRET_COLLISION_RADIUS, TURRET_GAMEPLAY_RADIUS), lane::TURRET_STATS),
            UnitKind::Gatehouse => ((120.0, 140.0), lane::GATEHOUSE_STATS),
            UnitKind::Base => ((200.0, 220.0), lane::BASE_STATS),
            _ => ((0.0, 50.0), Stats { max_health: 1.0, ..Stats::NONE }),
        };
        let mut u = Unit::new(id, p.kind, p.team, p.pos, radii, stats);
        u.tier = p.tier;
        u.lane = p.lane;
        if p.kind == UnitKind::Turret {
            u.attack = Some(lane::TURRET_ATTACK);
            u.brain = Some(Brain::Tower { heat: 0, cools_at: SimTime(0) });
        }
        self.units.push(u);
        id
    }

    /// Start a match on the current map's layout: its structures and relics, and the first
    /// wave timer. A no-op on maps without a layout.
    pub fn start_match(&mut self) {
        let placements = self.map.layout.placements.clone();
        for p in placements {
            self.spawn_placement(p);
        }
        self.game = MatchState { started_at: SimTime::end_of(self.tick), ..MatchState::default() };
        if !self.map.layout.lanes.is_empty() {
            self.game.next_wave_at = Some(SimTime::end_of(self.tick).plus(lane::FIRST_WAVE));
        }
    }

    /// Reset for a new match after one ended: structures, relics and minions are replaced,
    /// champions return to their spawn at full health (tick and unit ids keep counting).
    pub fn restart_match(&mut self) {
        self.units.retain(|u| u.kind == UnitKind::Champion);
        self.missiles.clear();
        self.struck.clear();
        self.areas.clear();
        self.bolts.clear();
        for u in self.units.iter_mut() {
            let mut progress = self.rules.progress();
            progress.augment_seed = self.rng.next_u32();
            u.state.progress = progress;
            u.reset_stats();
            u.state = UnitState::new(u.home, u.stats.move_speed);
            u.state.health = u.stats.max_health;
            u.state.progress = progress;
        }
        self.start_match();
    }

    /// Seconds since the match began, at `t`.
    pub fn match_secs(&self, t: SimTime) -> f32 {
        t.secs_since(self.game.started_at).max(0.0)
    }

    /// The match on a lane map: waves, recent aggression, winner.
    pub fn game(&self) -> &MatchState {
        &self.game
    }

    pub fn rules(&self) -> Rules {
        self.rules
    }

    /// Set before spawning champions (they start with the rules' level, gold and points).
    pub fn set_rules(&mut self, rules: Rules) {
        self.rules = rules;
    }

    /// One wave per team at its spawn point, in a column along the lane (melee in front).
    fn spawn_wave(&mut self) {
        let n = self.game.waves_spawned;
        let layout = self.map.layout.clone();
        // Minions grow stronger and faster as the match goes on (D55).
        let secs = self.match_secs(SimTime::end_of(self.tick));
        let (upgrades, speed) = (lane::minion_upgrades(secs) as f32, lane::minion_speed(secs));
        for (li, lane) in layout.lanes.iter().enumerate() {
            for team in [Team::Blue, Team::Red] {
                // That lane's enemy Gatehouse down (until it respawns): the wave brings a super
                // minion.
                let empowered = self.units.iter().any(|u| {
                    u.kind == UnitKind::Gatehouse && u.team != team && u.lane as usize == li && !u.state.alive()
                });
                let spawn = layout.wave_spawn[li][team as usize];
                let ahead =
                    lane[team as usize].first().map_or(Vec2::new(1.0, 0.0), |&p| (p - spawn).normalize_or_zero());
                self.spawn_lane_wave(n, empowered, team, li as u8, spawn, ahead, upgrades, speed);
            }
        }
        self.game.waves_spawned += 1;
        self.game.next_wave_at = self.game.next_wave_at.map(|t| t.plus(lane::wave_interval(secs)));
        // Turrets hit harder as the match goes on.
        let damage = lane::turret_damage(self.game.waves_spawned);
        for u in self.units.iter_mut().filter(|u| u.kind == UnitKind::Turret) {
            u.stats.attack_damage = damage;
        }
    }

    /// One team's wave in one lane, in a column along the lane (melee in front).
    #[allow(clippy::too_many_arguments)]
    fn spawn_lane_wave(
        &mut self,
        n: u32,
        empowered: bool,
        team: Team,
        lane: u8,
        spawn: Vec2,
        ahead: Vec2,
        upgrades: f32,
        speed: f32,
    ) {
        {
            let side = Vec2::new(-ahead.y, ahead.x);
            for (i, kind) in MatchState::wave(n, empowered).into_iter().enumerate() {
                let (row, col) = ((i / 3) as f32, (i % 3) as f32 - 1.0);
                let pos = spawn - ahead * (row * 80.0) + side * (col * 70.0);
                let id = self.spawn_minion(kind, team, pos, Some(Brain::Laner { lane, next: 0 }));
                let (health, damage) = lane::minion_upgrade(kind);
                if let Some(u) = self.units.iter_mut().find(|u| u.id == id) {
                    u.stats.max_health += health * upgrades;
                    u.stats.attack_damage += damage * upgrades;
                    u.stats.move_speed = speed;
                    u.state.health = u.stats.max_health;
                    u.state.move_speed = speed;
                }
            }
        }
    }

    pub fn despawn(&mut self, id: UnitId) {
        self.units.retain(|u| u.id != id);
    }

    /// Build a partial world, as client prediction does (own unit plus collision proxies).
    pub fn from_units(tick: Tick, units: Vec<Unit>) -> Self {
        let next_unit = units.iter().map(|u| u.id.0 + 1).max().unwrap_or(1);
        Self { tick, units, next_unit, ..World::new(0) }
    }

    /// Replace every unit except `keep` (client prediction refreshes its collision proxies).
    pub fn replace_others(&mut self, keep: UnitId, others: impl IntoIterator<Item = Unit>) {
        self.units.retain(|u| u.id == keep);
        self.units.extend(others.into_iter().filter(|u| u.id != keep));
        self.units.sort_by_key(|u| u.id);
    }

    pub fn map(&self) -> &Arc<Map> {
        &self.map
    }

    /// Use a map (server: at match start; client: from the welcome). Walls and paths then
    /// apply to every unit.
    pub fn set_map(&mut self, map: Arc<Map>) {
        self.map = map;
    }

    /// Units `team` can't see (and so can't attack) from the next tick on.
    /// Enemy units `team` can't see this tick (sorted).
    pub fn hidden(&self, team: Team) -> &[UnitId] {
        &self.hidden[team as usize]
    }

    pub fn set_hidden(&mut self, team: Team, mut ids: Vec<UnitId>) {
        ids.sort();
        self.hidden[team as usize] = ids;
    }

    /// Rewind or fast-forward the tick counter (prediction reconciliation only).
    pub fn set_tick(&mut self, tick: Tick) {
        self.tick = tick;
    }

    /// Simulate the next tick. Only commands whose `tick` equals the new tick are applied, in
    /// `(sub, player, seq)` order, each at its exact sub-tick instant.
    pub fn step(&mut self, commands: &[Command]) {
        let k = self.tick.next();
        let s0 = SimTime::end_of(self.tick);
        let s1 = SimTime::end_of(k);
        let mut cmds: Vec<&Command> = commands.iter().filter(|c| c.tick == k).collect();
        cmds.sort_by_key(|c| (c.sub, c.player, c.seq));
        let prediction = self.prediction;

        // Phase 0: lane minions that died are gone; respawns; structure protection; waves.
        self.units.retain(|u| u.state.alive() || !matches!(u.brain, Some(Brain::Laner { .. })));
        for u in self.units.iter_mut() {
            if u.state.respawn_at.is_some_and(|r| r <= s0) {
                let mut progress = u.state.progress;
                // Unstable Experiment: huge or tiny again, at random, at every respawn.
                progress.unstable_tiny = augments::unstable_roll(progress.augment_seed, s0.0);
                u.state = UnitState::new(u.home, u.stats.move_speed);
                u.state.health = u.stats.max_health;
                u.state.progress = progress;
                match u.brain {
                    Some(Brain::Patrol { a, b, .. }) => u.brain = Some(Brain::Patrol { a, b, toward_b: true }),
                    Some(Brain::Tower { .. }) => u.brain = Some(Brain::Tower { heat: 0, cools_at: SimTime(0) }),
                    _ => {}
                }
                self.events.push(SimEvent::Respawned { unit: u.id, pos: u.home, at: s0 });
            }
        }
        for u in self.units.iter_mut() {
            if self.rules.augments && u.kind == UnitKind::Champion {
                augments::update_draft(&mut u.state.progress);
            }
            u.refresh_stats(); // level-ups, purchases and augments
            if u.state.progress.undo_len > 0 && !can_shop(u, &self.map, &self.rules) {
                u.state.progress.undo_len = 0; // left the fountain: trades are final
            }
        }
        lane::update_protection(&mut self.units);
        let secs = self.match_secs(SimTime::end_of(self.tick));
        lane::update_structure_amp(&mut self.units, secs);
        if !prediction && self.game.winner.is_none() && self.game.next_wave_at.is_some_and(|w| w <= s0) {
            self.spawn_wave();
        }

        let World {
            units, rng, missiles, areas, bolts, next_missile, struck, events, map, hidden, game, rules, ..
        } = self;
        let rules = *rules;
        let map: &Map = map;

        // Phase 1: AI.
        let champions: Vec<(Team, Vec2, Option<Vec2>, f32)> = units
            .iter()
            .filter(|u| u.kind == UnitKind::Champion && u.state.alive())
            .map(|u| (u.team, u.state.pos, u.state.heading(), u.state.move_speed))
            .collect();
        for unit in units.iter_mut() {
            match unit.brain {
                Some(Brain::Patrol { a, b, toward_b }) if unit.state.order == Order::Idle && unit.state.alive() => {
                    unit.state.set_order(Order::MoveTo(if toward_b { b } else { a }), map);
                    unit.brain = Some(Brain::Patrol { a, b, toward_b: !toward_b });
                }
                Some(Brain::RigTurret { range }) if unit.state.can_cast(s0, 0) => {
                    let me = unit.state.pos;
                    let mut best: Option<(f32, Vec2, Option<Vec2>, f32)> = None;
                    for &(team, pos, heading, speed) in &champions {
                        let d = pos.distance(me);
                        if team != unit.team && d <= range as f32 && best.is_none_or(|(bd, ..)| d < bd) {
                            best = Some((d, pos, heading, speed));
                        }
                    }
                    let Some((d, pos, heading, speed)) = best else { continue };
                    let Effect::Line(spec) = TURRET_SHOT.effect else { continue };
                    let mut aim = pos;
                    if rng.next_u32() % 2 == 1
                        && let Some(h) = heading
                    {
                        // Lead the target: where it will be when the missile arrives.
                        let t = spec.windup.0 as f32 / SUBTICKS_PER_SECOND as f32 + d / spec.speed;
                        let to = h - pos;
                        let len = to.length();
                        let travel = (speed * t).min(len);
                        if len > 0.0 {
                            aim = pos + to * (travel / len);
                        }
                    }
                    let ctx = CastContext { roster: &[], hidden: &[], fired: &mut Vec::new() };
                    try_cast(unit, 0, aim, s0, 0, map, ctx, events);
                }
                _ => {}
            }
        }
        if !prediction && !map.layout.lanes.is_empty() {
            let seen: Vec<Seen> = units.iter().filter(|u| u.state.alive()).map(Seen::of).collect();
            let since = SimTime(s0.0.saturating_sub(lane::AGGRESSION_MEMORY.0));
            let recent: Vec<(UnitId, UnitId, SimTime)> =
                game.aggression.iter().filter(|(_, _, t)| *t >= since).copied().collect();
            for unit in units.iter_mut() {
                let hide = &hidden[unit.team as usize];
                match unit.brain {
                    Some(Brain::Laner { lane, .. }) => {
                        let lane = &map.layout.lanes[lane as usize][unit.team as usize];
                        lane::laner_think(unit, &seen, lane, &recent, hide, map);
                    }
                    Some(Brain::Tower { .. }) => lane::tower_think(unit, &seen, &recent, hide, map),
                    _ => {}
                }
            }
        }

        // Phase 2: per-unit timelines against start-of-tick positions (order-independent).
        let starts: Vec<(UnitId, UnitKind, Team, Obstacle)> = units
            .iter()
            .filter(|u| u.collision_radius > 0.0 && u.state.alive() && u.state.dash.is_none())
            .map(|u| (u.id, u.kind, u.team, Obstacle { pos: u.state.pos, radius: u.collision_radius }))
            .collect();
        let roster: Vec<Target> = units
            .iter()
            .filter(|u| u.targetable())
            .map(|u| Target {
                id: u.id,
                team: u.team,
                pos: u.state.pos,
                radius: u.gameplay_radius,
                kind: u.kind,
                range: u.attack.map_or(0.0, |a| a.range),
                max_health: u.stats.max_health,
            })
            .collect();
        let start_pos: Vec<(UnitId, Vec2)> = units.iter().map(|u| (u.id, u.state.pos)).collect();
        let mut fired: Vec<Fired> = Vec::new();
        for unit in units.iter_mut() {
            let mine: Vec<&Command> = match unit.owner {
                Some(owner) => cmds.iter().copied().filter(|c| c.player == owner).collect(),
                None => Vec::new(),
            };
            if !unit.state.alive() {
                // Commands while dead are dropped, except shopping.
                for c in &mine {
                    shop(unit, c.kind, map, &rules);
                }
                continue;
            }
            let here = unit.state.pos;
            let (kind, team) = (unit.kind, unit.team);
            let obstacles: Vec<Obstacle> = starts
                .iter()
                .filter(|(id, k, t, o)| {
                    *id != unit.id && blocks(kind, team, *k, *t) && (o.pos - here).length_sq() < BROADPHASE * BROADPHASE
                })
                .map(|(_, _, _, o)| *o)
                .collect();
            let radius = unit.collision_radius;
            let (mut desired, mut achieved) = (0.0f32, 0.0f32);
            let mut t = s0;
            let mut next_cmd = 0;
            loop {
                // Instants at `t`: windups and dashes ending, then commands.
                if t > s0 {
                    fire_due(unit, t, &roster, &mut fired, map);
                    unit.state.settle_recovery(t);
                    // The input buffer: a cast ordered while busy starts once the action ends.
                    let st = &unit.state;
                    if let Some(b) = st.buffered
                        && st.cast.is_none()
                        && st.dash.is_none()
                        && !st.hard_locked(t)
                    {
                        unit.state.buffered = None;
                        let ctx =
                            CastContext { roster: &roster, hidden: &hidden[unit.team as usize], fired: &mut fired };
                        try_cast(unit, b.slot, b.target.to_vec2(), t, b.seq, map, ctx, events);
                    }
                }
                while let Some(c) = mine.get(next_cmd)
                    && SimTime::at(k, c.sub) == t
                {
                    let ctx = CastContext { roster: &roster, hidden: &hidden[unit.team as usize], fired: &mut fired };
                    apply_command(unit, c, t, map, &rules, ctx, events);
                    next_cmd += 1;
                }
                if t >= s1 {
                    break;
                }
                let halt = think(unit, &roster, &hidden[unit.team as usize], t, map);
                let cmd_at = mine.get(next_cmd).map(|c| SimTime::at(k, c.sub));
                let st = &unit.state;
                let waits = [
                    cmd_at,
                    st.cast.map(|c| c.fire_at),
                    st.echo.map(|e| e.at),
                    st.attack.map(|a| a.fire_at),
                    st.dash.map(|d| d.end_at),
                    st.recovery.map(|r| r.hard_until),
                    st.recovery.map(|r| r.until),
                    Some(st.stunned_until),
                    Some(st.rooted_until),
                    Some(st.slowed_until),
                    halt.map(|_| st.attack_ready_at),
                ];
                let mut next = waits.into_iter().flatten().filter(|w| *w > t && *w <= s1).min().unwrap_or(s1);
                let span = next.0 - t.0;
                let dt = span as f32 / SUBTICKS_PER_SECOND as f32;
                if unit.state.dash.is_some() {
                    unit.state.dash_advance(dt, radius, map);
                } else if unit.state.can_move(t) {
                    let (d, a, used) = unit.state.advance(dt, radius, &obstacles, map, halt, t);
                    desired += d;
                    achieved += a;
                    if used < 1.0 {
                        // Reached attack range part-way: look again from there.
                        let part = ((used * span as f32).ceil() as u64).clamp(1, span);
                        next = SimTime(t.0 + part);
                    }
                }
                t = next;
            }
            update_stuck(&mut unit.state, desired, achieved, radius, &obstacles, map);
        }

        // Phase 3: effects.
        let mut melee: Vec<(UnitId, UnitId, f32, SimTime)> = Vec::new();
        let mut direct: Vec<Fired> = Vec::new();
        for f in fired {
            if let Fired::Melee { owner, target, power, at } = f {
                melee.push((owner, target, power, at));
                continue;
            }
            if matches!(f, Fired::Strike { .. } | Fired::Support { .. }) {
                direct.push(f);
                continue;
            }
            let id = if prediction {
                0
            } else {
                *next_missile += 1;
                *next_missile - 1
            };
            match f {
                Fired::Missile(m) => {
                    let m = Missile { id, ..m };
                    events.push(SimEvent::MissileSpawned(m));
                    if !prediction {
                        missiles.push(m);
                    }
                }
                Fired::Area(a) => {
                    let a = Area { id, ..a };
                    events.push(SimEvent::AreaSpawned(a));
                    if !prediction {
                        areas.push(a);
                    }
                }
                Fired::Bolt(b) => {
                    let b = Bolt { id, ..b };
                    events.push(SimEvent::AttackLaunched(b));
                    if !prediction {
                        bolts.push(b);
                    }
                }
                Fired::Melee { .. } | Fired::Strike { .. } | Fired::Support { .. } => {}
            }
        }
        if !prediction {
            let first_event = events.len();
            melee.sort_by_key(|m| (m.3, m.0));
            let mut landed = Vec::new();
            for (owner, target, power, at) in melee {
                let amp = damage_amp(units, owner, target, false, rng);
                if let Some(u) = units.iter_mut().find(|u| u.id == target) {
                    let dealt =
                        deal_damage(u, owner, DamageOrigin::Attack, power * amp, DamageKind::Physical, at, events);
                    landed.push((owner, target, dealt, at));
                }
            }
            on_hit(units, &landed, events);
            resolve_direct(units, &mut direct, rng, s1, events);
            resolve_effects(units, missiles, areas, bolts, struck, rng, &start_pos, s0, s1, events);
            let had_winner = game.winner.is_some();
            note_outcomes(units, game, &events[first_event..], s1);
            if rules.ranked {
                let tick_events: Vec<SimEvent> = events[first_event..].to_vec();
                rewards(units, game, &tick_events, events);
            }
            if let (false, Some((winner, at))) = (had_winner, game.winner) {
                events.push(SimEvent::MatchEnded { winner, at });
            }
        }

        // Phase 4: fountains, relics, passive gold, regeneration and shield expiry.
        fountains_and_relics(units, map, prediction, s1, events);
        for u in units.iter_mut().filter(|u| u.kind == UnitKind::Champion) {
            let st = &mut u.state;
            if st.alive() && st.potion_until > s0 {
                let secs = (st.potion_until.min(s1).0 - s0.0) as f32 / SUBTICKS_PER_SECOND as f32;
                st.health = (st.health + st.potion_rate * secs).min(u.stats.max_health);
            }
            if st.potion_until <= s1 {
                st.potion_rate = 0.0;
            }
            if can_shop(u, map, &rules) {
                let p = &mut u.state.progress;
                for s in 0..INVENTORY {
                    if let Some((_, _, most, true)) = items::consumable(p.items[s]) {
                        p.charges[s] = most;
                    }
                }
            }
        }
        if rules.passive_gold > 0.0 {
            for u in units.iter_mut().filter(|u| u.kind == UnitKind::Champion) {
                u.state.progress.gold += rules.passive_gold * TICK_DT;
            }
        }
        for u in units.iter_mut().filter(|u| u.state.alive()) {
            let max = u.stats.max_health;
            if u.state.health < max && u.stats.health_regen > 0.0 {
                u.state.health = (u.state.health + u.stats.health_regen * TICK_DT).min(max);
            }
            if u.state.shield > 0.0 && u.state.shield_until <= s1 {
                u.state.shield = 0.0;
            }
        }
        self.tick = k;
    }

    pub fn state_hash(&self) -> u64 {
        let mut h = StateHasher::new();
        h.write_u32(self.tick.0);
        let (s, i) = self.rng.state_parts();
        h.write_u64(s);
        h.write_u64(i);
        for u in &self.units {
            h.write_u32(u.id.0);
            h.write_u8(u.owner.map_or(0xff, |p| p.0));
            h.write_u8(u.team as u8);
            u.state.hash_into(&mut h);
            match u.brain {
                None => h.write_u8(0),
                Some(Brain::Patrol { toward_b, .. }) => h.write_u8(1 + toward_b as u8),
                Some(Brain::RigTurret { range }) => {
                    h.write_u8(3);
                    h.write_u16(range);
                }
                Some(Brain::Laner { lane, next }) => {
                    h.write_u8(4);
                    h.write_u8(next);
                    // Lane 0 (single-lane maps) hashes as before.
                    if lane > 0 {
                        h.write_u8(lane);
                    }
                }
                Some(Brain::Tower { heat, cools_at }) => {
                    h.write_u8(5);
                    h.write_u8(heat);
                    h.write_u64(cools_at.0);
                }
            }
        }
        for ids in &self.hidden {
            h.write_u32(ids.len() as u32);
            for id in ids {
                h.write_u32(id.0);
            }
        }
        self.game.hash_into(&mut h);
        h.write_u32(self.next_missile);
        for m in &self.missiles {
            h.write_u32(m.id);
            h.write_f32(m.origin.x);
            h.write_f32(m.origin.y);
            h.write_f32(m.dir.x);
            h.write_f32(m.dir.y);
            h.write_u64(m.spawn_at.0);
            h.write_f32(m.power);
            h.write_f32(m.spec.radius);
            h.write_u8(m.shot);
        }
        for a in &self.areas {
            h.write_u32(a.id);
            h.write_f32(a.center.x);
            h.write_f32(a.center.y);
            h.write_u64(a.detonate_at.0);
            h.write_f32(a.power);
            h.write_f32(a.radius);
            h.write_u8(a.shot);
        }
        for v in &self.struck {
            h.write_u32(v.owner.0);
            h.write_u32(v.cast_seq);
            h.write_u8(v.echo as u8);
            h.write_u32(v.target.0);
        }
        for b in &self.bolts {
            h.write_u32(b.id);
            h.write_u32(b.target.0);
            h.write_f32(b.pos.x);
            h.write_f32(b.pos.y);
            h.write_f32(b.power);
        }
        h.finish()
    }
}

impl Unit {
    /// A champion at full health (also used by client prediction for its own unit).
    pub fn champion(
        id: UnitId,
        owner: PlayerId,
        team: Team,
        champion: ChampionId,
        pos: Vec2,
        home: Vec2,
        stats: Stats,
    ) -> Unit {
        let radii = (CHAMPION_COLLISION_RADIUS, CHAMPION_GAMEPLAY_RADIUS);
        let mut u = Unit::new(id, UnitKind::Champion, team, pos, radii, stats);
        u.owner = Some(owner);
        u.champion = Some(champion);
        u.home = home;
        u.attack = Some(champion.def().attack);
        u
    }
}

/// Attack logic at instant `t` (Attack and AttackMove orders): start a windup when a valid
/// target is in range and the attack timer allows, chase otherwise. Returns the range circle
/// the unit's movement must halt at, if it has a target.
fn think(unit: &mut Unit, roster: &[Target], hidden: &[UnitId], t: SimTime, map: &Map) -> Option<(Vec2, f32)> {
    let atk = unit.attack_spec()?;
    let (me, my_team) = (unit.id, unit.team);
    let st = &mut unit.state;
    if let Some(w) = st.attack {
        // Winding up: rooted; keep facing the committed target.
        let target = roster.iter().find(|r| r.id == w.target)?;
        st.face(target.pos - st.pos);
        return Some((target.pos, atk.range + target.radius));
    }
    let enemy = |r: &&Target| r.team != my_team && r.id != me && hidden.binary_search(&r.id).is_err();
    let target = match st.order {
        Order::Attack(id) => match roster.iter().filter(enemy).find(|r| r.id == id) {
            Some(r) => *r,
            None => {
                st.set_order(Order::Idle, map); // dead or gone
                return None;
            }
        },
        Order::AttackMove(_) => {
            let mut best: Option<(f32, Target)> = None;
            for r in roster.iter().filter(enemy) {
                let gap = r.pos.distance(st.pos) - (atk.range + r.radius);
                if gap <= RANGE_SLACK && best.is_none_or(|(g, b)| (gap, r.id) < (g, b.id)) {
                    best = Some((gap, *r));
                }
            }
            best?.1
        }
        Order::Idle | Order::MoveTo(_) => return None,
    };
    let reach = atk.range + target.radius;
    if target.pos.distance(st.pos) <= reach + RANGE_SLACK {
        // An attack order waits for a follow-through (10 §4.1): only moves and casts cut it.
        if st.cast.is_none()
            && st.dash.is_none()
            && st.stunned_until <= t
            && st.attack_ready_at <= t
            && !st.recovering(t)
        {
            st.attack = Some(AttackWindup { target: target.id, fire_at: t.plus(atk.windup()) });
            st.attack_ready_at = t.plus(atk.period());
            st.attacks = st.attacks.wrapping_add(1);
            st.face(target.pos - st.pos);
        }
    } else if matches!(st.order, Order::Attack(_))
        && st.detour.is_none()
        && st.path.goal().is_none_or(|g| g.distance(target.pos) > REPATH_DISTANCE)
    {
        st.path = Path::from_points(&map.find_path(st.pos, target.pos));
    }
    Some((target.pos, reach))
}

/// Fire the line or area of the cast `c` at `t`, transformed by the caster's augments (06 §3:
/// Multishot, Echo, Broadside, where the ability accepts them). An echo fires once more at
/// reduced power. Returns whether the cast should echo.
fn deliver(unit: &Unit, c: EchoCast, t: SimTime, echo: bool, fired: &mut Vec<Fired>) -> bool {
    let Some(ability) = unit.ability(c.slot) else { return false };
    let (id, team, stats) = (unit.id, unit.team, unit.stats);
    let bonus = ability.bonus_damage_at(unit.state.progress.ranks.get(c.slot as usize).copied().unwrap_or(1));
    let d = augments::delivery(&unit.state.progress.augments);
    let tf = ability.transforms;
    let wide = d.wide && tf.accepts(Transforms::WIDE.0);
    let fundamentals = c.slot < 3 && augments::mods(&unit.state.progress.augments).fundamentals;
    let scale =
        if echo { augments::ECHO_POWER } else { 1.0 } * if fundamentals { augments::FUNDAMENTALS_AMP } else { 1.0 };
    let first_shot = if echo { augments::ECHO_SHOT } else { 0 };
    match ability.effect {
        Effect::Line(mut spec) => {
            if wide {
                spec.radius *= augments::WIDE_LINE;
            }
            let power = (spec.damage.raw(stats.attack_damage, stats.ability_power) + bonus) * scale;
            let base = Missile {
                id: 0,
                owner: id,
                team,
                origin: unit.state.pos,
                dir: c.dir,
                spec,
                spawn_at: t,
                cast_seq: c.seq,
                power,
                shot: first_shot,
                slot: c.slot,
            };
            fired.push(Fired::Missile(base));
            if d.multishot && tf.accepts(Transforms::MULTISHOT) {
                // One to each side of the aim, 15° apart.
                let (cs, sn) = (augments::SPREAD_COS, augments::SPREAD_SIN);
                let left = Vec2::new(c.dir.x * cs - c.dir.y * sn, c.dir.x * sn + c.dir.y * cs);
                let right = Vec2::new(c.dir.x * cs + c.dir.y * sn, -c.dir.x * sn + c.dir.y * cs);
                fired.push(Fired::Missile(Missile { dir: left, shot: first_shot + 1, ..base }));
                fired.push(Fired::Missile(Missile { dir: right, shot: first_shot + 2, ..base }));
            }
        }
        Effect::Area(a) => {
            let radius = if wide { a.radius * augments::WIDE_AREA } else { a.radius };
            fired.push(Fired::Area(Area {
                id: 0,
                owner: id,
                team,
                center: c.point,
                radius,
                spawn_at: t,
                detonate_at: t.plus(a.delay),
                kind: a.damage.kind,
                power: (a.damage.raw(stats.attack_damage, stats.ability_power) + bonus) * scale,
                cast_seq: c.seq,
                cc: a.cc,
                shot: first_shot,
                slot: c.slot,
            }));
        }
        _ => return false,
    }
    !echo && d.echo && tf.accepts(Transforms::ECHO)
}

/// Windups, casts and dashes whose instant is `t`.
fn fire_due(unit: &mut Unit, t: SimTime, roster: &[Target], fired: &mut Vec<Fired>, map: &Map) {
    let (id, team, stats) = (unit.id, unit.team, unit.stats);
    if let Some(c) = unit.state.cast
        && c.fire_at == t
    {
        unit.state.cast = None;
        let echo = EchoCast {
            slot: c.slot,
            dir: c.dir,
            point: c.point,
            at: t.plus(SimDuration::from_millis(augments::ECHO_DELAY_MS)),
            seq: c.seq,
        };
        if deliver(unit, echo, t, false, fired) {
            unit.state.echo = Some(echo);
        }
        if unit.kind == UnitKind::Champion
            && let Some(a) = unit.ability(c.slot)
        {
            unit.state.start_recovery(a.timing(c.slot), t);
        }
    }
    if let Some(e) = unit.state.echo
        && e.at == t
    {
        unit.state.echo = None;
        if unit.state.alive() {
            deliver(unit, e, t, true, fired);
        }
    }
    if let Some(w) = unit.state.attack
        && w.fire_at == t
    {
        unit.state.attack = None;
        if let Some(atk) = unit.attack_spec()
            && let Some(target) = roster.iter().find(|r| r.id == w.target)
        {
            let (mut power, share) = lane::turret_shot(
                &mut unit.brain,
                stats.attack_damage,
                t,
                target.kind,
                target.range,
                target.max_health,
            );
            // Spellblade: an armed attack adds the base attack damage, and disarms.
            if unit.state.spellblade_until > t
                && let Some(c) = unit.champion
            {
                power += c.def().stats_at(unit.state.progress.level).attack_damage;
                unit.state.spellblade_until = SimTime(0);
            }
            if atk.bolt_speed <= 0.0 {
                fired.push(Fired::Melee { owner: id, target: w.target, power, at: t });
            } else {
                fired.push(Fired::Bolt(Bolt {
                    id: 0,
                    owner: id,
                    team,
                    target: w.target,
                    origin: unit.state.pos,
                    pos: unit.state.pos,
                    speed: atk.bolt_speed,
                    launched_at: t,
                    power,
                    kind: if share { DamageKind::True } else { DamageKind::Physical },
                }));
            }
        }
    }
    if let Some(d) = unit.state.dash
        && d.end_at == t
    {
        if let Some((target, slot)) = d.strike
            && let Some(a) = unit.ability(slot)
            && let Effect::Lunge(l) = a.effect
        {
            let rank = unit.state.progress.ranks.get(slot as usize).copied().unwrap_or(1);
            let mut power = l.damage.raw(stats.attack_damage, stats.ability_power) + a.bonus_damage_at(rank);
            if slot < 3 && augments::mods(&unit.state.progress.augments).fundamentals {
                power *= augments::FUNDAMENTALS_AMP;
            }
            fired.push(Fired::Strike { owner: id, slot, target, power, kind: l.damage.kind, cc: l.cc, at: t });
        }
        unit.state.dash = None;
        unit.state.detour = None;
        unit.state.route(map);
        let landing = Timing { follow_through: d.recover, ..Timing::NONE };
        unit.state.start_recovery(landing, t);
    }
}

/// What a cast may look at and create besides the caster: start-of-tick units (targets of
/// lunges and ally effects), what the caster's team can't see, and effects on others.
struct CastContext<'a> {
    roster: &'a [Target],
    hidden: &'a [UnitId],
    fired: &'a mut Vec<Fired>,
}

fn apply_command(
    unit: &mut Unit,
    c: &Command,
    t: SimTime,
    map: &Map,
    rules: &Rules,
    ctx: CastContext,
    events: &mut Vec<SimEvent>,
) {
    let may_shop = can_shop(unit, map, rules);
    let st = &mut unit.state;
    match c.kind {
        // A newer order replaces a buffered cast (one slot, the latest wins: 10 §4.1).
        CommandKind::MoveTo(q) => {
            st.cancel_attack(t);
            st.end_soft_recovery(t);
            st.buffered = None;
            st.set_order(Order::MoveTo(q), map);
        }
        CommandKind::AttackMove(q) => {
            st.cancel_attack(t);
            st.end_soft_recovery(t);
            st.buffered = None;
            st.set_order(Order::AttackMove(q), map);
        }
        CommandKind::Attack(target) => {
            if target == unit.id {
                return;
            }
            st.buffered = None;
            if st.attack.is_some_and(|w| w.target != target) {
                st.cancel_attack(t);
            }
            // An attack order doesn't cut a follow-through: it waits for it (the cancel tech
            // needs a move or a cast).
            if st.order != Order::Attack(target) {
                st.set_order(Order::Attack(target), map);
            }
        }
        CommandKind::Stop => {
            st.cancel_attack(t);
            st.end_soft_recovery(t);
            st.buffered = None;
            st.set_order(Order::Idle, map);
        }
        CommandKind::Cast { slot, target } => {
            // Abilities buffer (10 §4.2): a cast ordered during a windup, a dash or a hard
            // lock waits for it to end instead of being dropped.
            if st.alive() && (st.cast.is_some() || st.dash.is_some() || st.hard_locked(t)) {
                st.buffered = Some(BufferedCast { slot, target, seq: c.seq });
            } else {
                try_cast(unit, slot, target.to_vec2(), t, c.seq, map, ctx, events);
            }
        }
        CommandKind::LevelUp(slot) => {
            let p = &mut st.progress;
            if p.points > 0
                && let Some(rank) = p.ranks.get_mut(slot as usize)
                && *rank < crate::champion::max_rank(slot, p.level)
            {
                *rank += 1;
                p.points -= 1;
            }
        }
        CommandKind::Buy(_) | CommandKind::Sell(_) | CommandKind::Undo => shop(unit, c.kind, map, rules),
        CommandKind::UseItem(slot) => use_item(unit, slot, t),
        CommandKind::BuyAnvil if rules.augments && may_shop => {
            let p = &mut st.progress;
            if p.level >= crate::anvils::MIN_LEVEL && p.gold >= crate::anvils::COST && p.anvil_offer[0] == 0 {
                let lucky = augments::mods(&p.augments).anvil_luck;
                p.anvil_offer = crate::anvils::roll(p.augment_seed, p.anvils, lucky);
                p.anvils = p.anvils.saturating_add(1);
                p.gold -= crate::anvils::COST;
            }
        }
        CommandKind::PickAnvil(choice) => {
            let p = &mut st.progress;
            if let Some(&c) = p.anvil_offer.get(choice as usize)
                && let Some((tier, stat)) = crate::anvils::unpack(c)
            {
                let i = crate::anvils::STATS.iter().position(|s| *s == stat).unwrap_or(0);
                p.anvil[i] = p.anvil[i].saturating_add(crate::anvils::TIER_UNITS[tier.min(2) as usize]);
                p.anvil_offer = [0; crate::anvils::CHOICES];
            }
        }
        CommandKind::BuyAnvil => {}
        CommandKind::PickAugment(choice) if rules.augments => augments::pick(&mut st.progress, choice),
        CommandKind::RerollAugment(choice) if rules.augments => augments::reroll(&mut st.progress, choice),
        CommandKind::PickAugment(_) | CommandKind::RerollAugment(_) => {}
    }
}

/// Whether `unit` may shop now (01 §11, ARAM rule 06 §2): ranked matches only, while dead or
/// inside its own fountain.
pub fn can_shop(unit: &Unit, map: &Map, rules: &Rules) -> bool {
    rules.ranked
        && unit.kind == UnitKind::Champion
        && (!unit.state.alive()
            || map.layout.fountains[unit.team as usize].is_some_and(|(c, r)| (unit.state.pos - c).length() <= r))
}

/// Buy or sell (stats follow at the next tick's recompute). Invalid requests do nothing.
fn shop(unit: &mut Unit, kind: CommandKind, map: &Map, rules: &Rules) {
    if !matches!(kind, CommandKind::Buy(_) | CommandKind::Sell(_) | CommandKind::Undo) || !can_shop(unit, map, rules) {
        return;
    }
    let p = &mut unit.state.progress;
    match kind {
        CommandKind::Buy(id) if items::consumable(id).is_some() => {
            // Potions stack in a slot (up to its charges); a flask is one per champion, full.
            let (Some(item), Some((_, _, most, refills))) = (items::item(id), items::consumable(id)) else { return };
            if item.cost > p.gold {
                return;
            }
            let held = p.items.iter().position(|i| *i == id);
            let before = Trade { items: p.items, charges: p.charges, gold: -item.cost };
            match held {
                Some(_) if refills => return,
                Some(s) if p.charges[s] < most => p.charges[s] += 1,
                _ => {
                    let Some(s) = p.items.iter().position(|i| *i == 0) else { return };
                    p.items[s] = id;
                    p.charges[s] = if refills { most } else { 1 };
                }
            }
            p.push_trade(before);
            p.gold -= item.cost;
        }
        CommandKind::Buy(id) => {
            let (Some(item), Some((cost, used))) = (items::item(id), items::price(id, &p.items)) else { return };
            let mut inv = p.items;
            for s in used {
                inv[s] = 0;
            }
            // One pair of boots; a free slot once the components are gone.
            let boots = inv.iter().any(|i| items::item(*i).is_some_and(|i| i.boots));
            let Some(slot) = inv.iter().position(|i| *i == 0) else { return };
            if cost > p.gold || (item.boots && boots) {
                return;
            }
            inv[slot] = id;
            p.push_trade(Trade { items: p.items, charges: p.charges, gold: -cost });
            // Slots whose item changed hold no charges.
            for ((c, new), old) in p.charges.iter_mut().zip(inv).zip(p.items) {
                if new != old {
                    *c = 0;
                }
            }
            p.items = inv;
            p.gold -= cost;
        }
        CommandKind::Sell(slot) => {
            let (before, charges) = (p.items, p.charges);
            let s = slot as usize;
            if let Some(i) = p.items.get_mut(s)
                && let Some(item) = items::item(*i)
            {
                // A stack of potions sells by the potion; anything else whole.
                let n = match items::consumable(item.id) {
                    Some((_, _, _, false)) => charges[s].max(1) as f32,
                    _ => 1.0,
                };
                let refund = item.cost * n * items::SELL_REFUND;
                *i = 0;
                p.charges[s] = 0;
                p.gold += refund;
                p.push_trade(Trade { items: before, charges, gold: refund });
            }
        }
        CommandKind::Undo if p.undo_len > 0 => {
            p.undo_len -= 1;
            let t = p.undo[p.undo_len as usize];
            p.items = t.items;
            p.charges = t.charges;
            p.gold -= t.gold;
        }
        _ => {}
    }
}

/// Drink the consumable in `slot` at `t`: it heals over time from now (a second one while one
/// runs adds its time). A potion stack shrinks by one and is gone at zero; a flask keeps its
/// slot and refills at the fountain.
fn use_item(unit: &mut Unit, slot: u8, t: SimTime) {
    let st = &mut unit.state;
    let s = slot as usize;
    if !st.alive() || s >= INVENTORY || st.progress.charges[s] == 0 {
        return;
    }
    let Some((heal, duration_ms, _, refills)) = items::consumable(st.progress.items[s]) else { return };
    st.progress.charges[s] -= 1;
    if st.progress.charges[s] == 0 && !refills {
        st.progress.items[s] = 0;
    }
    let duration = SimDuration::from_millis(duration_ms);
    let rate = heal / duration.0 as f32 * SUBTICKS_PER_SECOND as f32;
    st.potion_until = if st.potion_until > t { st.potion_until.plus(duration) } else { t.plus(duration) };
    st.potion_rate = st.potion_rate.max(rate);
}

/// Validate and start a cast at `t` (03 §5: the sim validates everything). Skillshots and
/// areas wind up (rooted); dashes, blinks and shields take effect at once.
#[allow(clippy::too_many_arguments)]
fn try_cast(
    unit: &mut Unit,
    slot: u8,
    target: Vec2,
    t: SimTime,
    seq: u32,
    map: &Map,
    ctx: CastContext,
    events: &mut Vec<SimEvent>,
) {
    let Some(ability) = unit.ability(slot) else { return };
    let mods = augments::mods(&unit.state.progress.augments);
    if slot == 3 && mods.fundamentals {
        return; // Fundamentals: no ultimate
    }
    let (id, team, radius) = (unit.id, unit.team, unit.collision_radius);
    let hyper = if slot < 3 && unit.state.progress.hyper { HYPER_HASTE } else { 0.0 };
    let haste = unit.stats.ability_haste + hyper;
    let (stats, gameplay_radius) = (unit.stats, unit.gameplay_radius);
    let timing = if unit.kind == UnitKind::Champion { ability.timing(slot) } else { Timing::NONE };
    let st = &mut unit.state;
    if !st.can_cast(t, slot) {
        return;
    }
    let to = target - st.pos;
    let dir = to.normalize_or_zero();
    let len = to.length();
    match ability.effect {
        Effect::Line(spec) => {
            if dir == Vec2::ZERO {
                return;
            }
            st.cancel_attack(t);
            st.face(dir);
            let fire_at = t.plus(spec.windup);
            st.cast = Some(Cast { slot, dir, point: target, fire_at, seq, mobile: timing.mobile });
            events.push(SimEvent::CastStarted { unit: id, slot, at: t, dir, point: target, fire_at, seq });
        }
        Effect::Area(a) => {
            st.cancel_attack(t);
            let point = if len > a.range { st.pos + dir * a.range } else { target };
            st.face(dir);
            let fire_at = t.plus(a.windup);
            st.cast = Some(Cast { slot, dir, point, fire_at, seq, mobile: timing.mobile });
            events.push(SimEvent::CastStarted { unit: id, slot, at: t, dir, point, fire_at, seq });
        }
        Effect::Dash(d) => {
            if dir == Vec2::ZERO || st.rooted_until > t {
                return;
            }
            st.cancel_attack(t);
            let dist = len.min(d.range);
            let to = st.pos + dir * dist;
            let end_at = SimTime(t.0 + ((dist / d.speed * SUBTICKS_PER_SECOND as f32).ceil() as u64).max(1));
            st.dash = Some(DashMove { dir, to, speed: d.speed, end_at, strike: None, recover: timing.follow_through });
            st.face(dir);
            st.detour = None;
            events.push(SimEvent::Dashed { unit: id, from: st.pos, to, at: t, end_at });
        }
        Effect::Lunge(l) => {
            if st.rooted_until > t {
                return;
            }
            // The visible enemy champion or minion closest to the cursor, in range.
            let pick = ctx
                .roster
                .iter()
                .filter(|r| {
                    r.team != team
                        && matches!(r.kind, UnitKind::Champion | UnitKind::Minion)
                        && !ctx.hidden.contains(&r.id)
                        && (r.pos - st.pos).length() <= l.range + r.radius
                        && (r.pos - target).length() <= LUNGE_PICK + r.radius
                })
                .min_by(|a, b| {
                    (a.pos - target).length_sq().total_cmp(&(b.pos - target).length_sq()).then(a.id.cmp(&b.id))
                });
            let Some(victim) = pick else { return };
            st.cancel_attack(t);
            let to_victim = victim.pos - st.pos;
            let dir = to_victim.normalize_or_zero();
            let dist = (to_victim.length() - victim.radius - gameplay_radius * 0.5).max(0.0);
            let to = st.pos + dir * dist;
            let end_at = SimTime(t.0 + ((dist / l.speed * SUBTICKS_PER_SECOND as f32).ceil() as u64).max(1));
            st.dash = Some(DashMove {
                dir,
                to,
                speed: l.speed,
                end_at,
                strike: Some((victim.id, slot)),
                recover: timing.follow_through,
            });
            st.face(dir);
            st.detour = None;
            events.push(SimEvent::Dashed { unit: id, from: st.pos, to, at: t, end_at });
        }
        Effect::Support(sup) => {
            let rank = st.progress.ranks.get(slot as usize).copied().unwrap_or(1);
            let bonus = ability.bonus_damage_at(rank);
            let ally = if sup.range > 0.0 {
                ctx.roster
                    .iter()
                    .filter(|r| {
                        r.team == team
                            && r.id != id
                            && r.kind == UnitKind::Champion
                            && (r.pos - st.pos).length() <= sup.range + r.radius
                            && (r.pos - target).length() <= SUPPORT_PICK + r.radius
                    })
                    .min_by(|a, b| {
                        (a.pos - target).length_sq().total_cmp(&(b.pos - target).length_sq()).then(a.id.cmp(&b.id))
                    })
            } else {
                None
            };
            let heal = if sup.heal > 0.0 || sup.heal_missing > 0.0 {
                sup.heal + sup.heal_ap * stats.ability_power + if sup.shield > 0.0 { 0.0 } else { bonus }
            } else {
                0.0
            };
            let shield = if sup.shield > 0.0 { sup.shield + sup.shield_ap * stats.ability_power + bonus } else { 0.0 };
            let until = t.plus(sup.duration);
            match ally {
                Some(a) => ctx.fired.push(Fired::Support {
                    owner: id,
                    target: a.id,
                    heal,
                    heal_missing: sup.heal_missing,
                    shield,
                    until,
                    at: t,
                }),
                None => {
                    // On the caster itself: at once, so prediction shows it.
                    let max = stats.max_health;
                    let amount = (heal + sup.heal_missing * (max - st.health)).min(max - st.health);
                    if amount > 0.0 {
                        st.health += amount;
                        events.push(SimEvent::Healed { unit: id, amount, at: t });
                    }
                    if shield > 0.0 {
                        st.shield = if st.shield_until > t { st.shield + shield } else { shield };
                        st.shield_until = st.shield_until.max(until);
                        events.push(SimEvent::Shielded { unit: id, amount: shield, at: t, until: st.shield_until });
                    }
                }
            }
        }
        Effect::Blink(b) => {
            if dir == Vec2::ZERO {
                return;
            }
            st.cancel_attack(t);
            st.face(dir);
            let from = st.pos;
            st.pos = blink_landing(map, from, dir, len.min(b.range), radius);
            st.detour = None;
            st.stuck = 0;
            st.route(map);
            events.push(SimEvent::Blinked { unit: id, from, to: st.pos, at: t });
        }
        Effect::Shield(s) => {
            st.shield = s.amount;
            st.shield_until = t.plus(s.duration);
            events.push(SimEvent::Shielded { unit: id, amount: s.amount, at: t, until: st.shield_until });
        }
    }
    // Instant casts (supports, shields, blinks, dashes, lunges) still tell clients a cast
    // happened, so they can animate it (and tell one dash from another, A10); `fire_at == at`
    // marks it instant.
    if matches!(
        ability.effect,
        Effect::Support(_) | Effect::Shield(_) | Effect::Blink(_) | Effect::Dash(_) | Effect::Lunge(_)
    ) {
        events.push(SimEvent::CastStarted { unit: id, slot, at: t, dir, point: target, fire_at: t, seq });
    }
    // A new cast ends any follow-through still running (10 §4.1).
    st.recovery = None;
    let rank = st.progress.ranks.get(slot as usize).copied().unwrap_or(1);
    let mut cooldown = ability.cooldown_at(rank);
    if slot < 4 && haste > 0.0 {
        // Ability haste (02 §8) shortens Q W E R, not utility spells; whole sub-ticks.
        cooldown = SimDuration((cooldown.0 as f64 * 100.0 / (100.0 + haste as f64)).round() as u64);
    }
    st.cooldowns[slot as usize] = t.plus(cooldown);
    if mods.spellblade && slot < 4 {
        st.spellblade_until = t.plus(SimDuration::from_millis(augments::SPELLBLADE_MS));
    }
}

/// Where a blink toward `dir` lands: the farthest walkable, in-bounds point on the line, so a
/// blink can cross a thin wall but never ends inside one.
fn blink_landing(map: &Map, from: Vec2, dir: Vec2, dist: f32, radius: f32) -> Vec2 {
    let mut d = dist;
    loop {
        let p = from + dir * d;
        if map.walkable(p, radius) && map.in_bounds(p, radius) {
            return p;
        }
        if d <= 0.0 {
            return from;
        }
        d = (d - 10.0).max(0.0);
    }
}

/// Phase 3 on the server: missiles, areas and bolts against the units' motion this tick.
#[allow(clippy::too_many_arguments)]
fn resolve_effects(
    units: &mut [Unit],
    missiles: &mut Vec<Missile>,
    areas: &mut Vec<Area>,
    bolts: &mut Vec<Bolt>,
    struck: &mut Vec<VolleyHit>,
    rng: &mut Pcg32,
    start_pos: &[(UnitId, Vec2)],
    s0: SimTime,
    s1: SimTime,
    events: &mut Vec<SimEvent>,
) {
    // Skillshots and areas hit units, not structures (those take attacks only).
    let motion: Vec<(UnitId, Team, f32, Vec2, Vec2)> = units
        .iter()
        .filter(|u| u.targetable() && !u.kind.is_structure())
        .map(|u| {
            let start = start_pos.iter().find(|(id, _)| *id == u.id).map_or(u.state.pos, |(_, p)| *p);
            (u.id, u.team, u.gameplay_radius, start, u.state.pos)
        })
        .collect();
    let positions: Vec<(UnitId, Vec2)> = units.iter().map(|u| (u.id, u.state.pos)).collect();
    let pos_of = |id: UnitId| positions.iter().find(|(i, _)| *i == id).map(|(_, p)| *p);
    missiles.retain(|m| {
        let a = m.spawn_at.max(s0);
        let b = m.end_at().min(s1);
        let volley = m.volley();
        let mut hits: Vec<(SimTime, UnitId)> = motion
            .iter()
            .filter(|&&(id, team, ..)| team != m.team && id != m.owner)
            .filter(|&&(id, ..)| !struck.iter().any(|h| (h.owner, h.cast_seq, h.echo) == volley && h.target == id))
            .filter_map(|&(id, _, r, q0, q1)| m.first_hit(a, b, s0, q0, q1, r).map(|at| (at, id)))
            .collect();
        hits.sort();
        // The earliest contact with a unit that is still alive (an earlier effect this tick may
        // have killed the first candidate).
        let hit = hits.into_iter().find(|(_, id)| units.iter().any(|u| u.id == *id && u.targetable()));
        if let Some((at, target)) = hit {
            struck.push(VolleyHit { owner: volley.0, cast_seq: volley.1, echo: volley.2, target });
            events.push(SimEvent::MissileHit { id: m.id, target, at });
            let from = pos_of(m.owner).unwrap_or(m.origin);
            let amp = damage_amp(units, m.owner, target, true, rng);
            if let Some(u) = units.iter_mut().find(|u| u.id == target) {
                apply_cc(u, m.spec.cc, at, from, s1, events);
                let dealt = deal_damage(
                    u,
                    m.owner,
                    DamageOrigin::Ability(m.slot),
                    m.power * amp,
                    m.spec.damage.kind,
                    at,
                    events,
                );
                after_ability_hit(units, m.owner, target, dealt, at, events);
            }
            return false;
        }
        if m.end_at() <= s1 {
            events.push(SimEvent::MissileExpired { id: m.id, at: m.end_at() });
            return false;
        }
        true
    });
    // A volley's hits matter only while some of its missiles still fly.
    struck.retain(|h| missiles.iter().any(|m| m.volley() == (h.owner, h.cast_seq, h.echo)));
    areas.retain(|a| {
        if a.detonate_at > s1 {
            return true;
        }
        let frac = (a.detonate_at.0.saturating_sub(s0.0)) as f32 / SUBTICKS as f32;
        events.push(SimEvent::AreaDetonated { id: a.id, at: a.detonate_at });
        for &(id, team, r, q0, q1) in &motion {
            let reach = a.radius + r;
            if team != a.team && (q0.lerp(q1, frac) - a.center).length_sq() <= reach * reach {
                let amp = damage_amp(units, a.owner, id, true, rng);
                let Some(u) = units.iter_mut().find(|u| u.id == id) else { continue };
                apply_cc(u, a.cc, a.detonate_at, a.center, s1, events);
                let dealt = deal_damage(
                    u,
                    a.owner,
                    DamageOrigin::Ability(a.slot),
                    a.power * amp,
                    a.kind,
                    a.detonate_at,
                    events,
                );
                after_ability_hit(units, a.owner, id, dealt, a.detonate_at, events);
            }
        }
        false
    });
    let mut landed: Vec<(UnitId, UnitId, f32, SimTime)> = Vec::new();
    bolts.retain_mut(|b| {
        let amp = damage_amp(units, b.owner, b.target, false, rng);
        let Some(target) = units.iter_mut().find(|u| u.id == b.target && u.targetable()) else {
            events.push(SimEvent::AttackLanded { id: b.id, target: b.target, at: s1, hit: false });
            return false;
        };
        let from = b.launched_at.max(s0);
        let dt = (s1.0 - from.0) as f32 / SUBTICKS_PER_SECOND as f32;
        let to = target.state.pos - b.pos;
        let gap = (to.length() - target.gameplay_radius).max(0.0);
        let step = b.speed * dt;
        if gap <= step {
            let at = SimTime(from.0 + (gap / b.speed * SUBTICKS_PER_SECOND as f32) as u64).min(s1);
            events.push(SimEvent::AttackLanded { id: b.id, target: b.target, at, hit: true });
            let dealt = deal_damage(target, b.owner, DamageOrigin::Attack, b.power * amp, b.kind, at, events);
            landed.push((b.owner, b.target, dealt, at));
            return false;
        }
        b.pos += to.normalize_or_zero() * step;
        true
    });
    on_hit(units, &landed, events);
}

/// Item effects of landed basic attacks: on-hit magic damage, then life steal on the attack's
/// damage (02 §10 order).
fn on_hit(units: &mut [Unit], landed: &[(UnitId, UnitId, f32, SimTime)], events: &mut Vec<SimEvent>) {
    for &(owner, target, dealt, at) in landed {
        let Some(o) = units.iter().find(|u| u.id == owner && u.kind == UnitKind::Champion) else { continue };
        let (passives, stats) = (items::passives(&o.state.progress.items), o.stats);
        if let Some((base, ap_ratio, item)) = passives.on_hit_magic
            && let Some(t) = units.iter_mut().find(|u| u.id == target)
        {
            let raw = base + ap_ratio * stats.ability_power;
            deal_damage(t, owner, DamageOrigin::Item(item), raw, DamageKind::Magic, at, events);
        }
        if stats.life_steal > 0.0
            && dealt > 0.0
            && let Some(o) = units.iter_mut().find(|u| u.id == owner && u.state.alive())
        {
            let amount = (stats.life_steal * dealt).min(o.stats.max_health - o.state.health);
            if amount > 0.0 {
                o.state.health += amount;
                events.push(SimEvent::Healed { unit: owner, amount, at });
            }
        }
        // Thorns: the attacked champion returns a share as magic damage.
        let thorns = units
            .iter()
            .find(|u| u.id == target && u.kind == UnitKind::Champion && u.state.alive())
            .is_some_and(|t| augments::mods(&t.state.progress.augments).thorns);
        if thorns
            && dealt > 0.0
            && let Some(o) = units.iter_mut().find(|u| u.id == owner)
        {
            let thorns = DamageOrigin::Augment(augments::THORNS_ID);
            deal_damage(o, target, thorns, augments::THORNS * dealt, DamageKind::Magic, at, events);
        }
    }
}

/// Augment effects of an ability's damage (server only): Spellhunger stacks on champion hits,
/// then Spell Vamp heals.
fn after_ability_hit(
    units: &mut [Unit],
    owner: UnitId,
    target: UnitId,
    dealt: f32,
    at: SimTime,
    events: &mut Vec<SimEvent>,
) {
    if dealt <= 0.0 {
        return;
    }
    let on_champion = units.iter().any(|u| u.id == target && u.kind == UnitKind::Champion);
    let Some(o) = units.iter_mut().find(|u| u.id == owner && u.kind == UnitKind::Champion && u.state.alive()) else {
        return;
    };
    let m = augments::mods(&o.state.progress.augments);
    let p = &mut o.state.progress;
    if m.spellhunger && on_champion {
        p.stacks = (p.stacks + 1).min(augments::SPELLHUNGER_CAP);
    }
    if m.spell_vamp {
        let amount = (augments::SPELL_VAMP * dealt).min(o.stats.max_health - o.state.health);
        if amount > 0.0 {
            o.state.health += amount;
            events.push(SimEvent::Healed { unit: owner, amount, at });
        }
    }
}

/// Crowd control on `u` at `at` (`from`: where a pull drags it toward). Forced movement starts
/// with the next tick (`s1`): units only move during phase 2.
fn apply_cc(u: &mut Unit, cc: Cc, at: SimTime, from: Vec2, s1: SimTime, events: &mut Vec<SimEvent>) {
    if !u.targetable() {
        return;
    }
    let st = &mut u.state;
    let hard_stop = |st: &mut UnitState, until: SimTime| {
        st.stunned_until = st.stunned_until.max(until);
        st.cast = None; // hard CC interrupts casts, attacks, follow-throughs and the buffer
        st.attack = None;
        st.recovery = None;
        st.buffered = None;
    };
    match cc {
        Cc::None => {}
        Cc::Stun(d) | Cc::Knockup(d) => hard_stop(st, at.plus(d)),
        Cc::Root(d) => st.rooted_until = st.rooted_until.max(at.plus(d)),
        Cc::Slow { pct, duration } => st.apply_slow(pct, at.plus(duration), at),
        Cc::Pull(stop) => {
            // Dragged (a forced dash, ignoring units) to `stop` units from the puller.
            let away = st.pos - from;
            let len = away.length();
            if len <= stop as f32 || u.kind.is_structure() {
                return;
            }
            let dir = away * (1.0 / len);
            let to = from + dir * stop as f32;
            let dist = len - stop as f32;
            let end_at = SimTime(s1.0 + ((dist / PULL_SPEED * SUBTICKS_PER_SECOND as f32).ceil() as u64).max(1));
            st.dash =
                Some(DashMove { dir: -dir, to, speed: PULL_SPEED, end_at, strike: None, recover: SimDuration(0) });
            st.detour = None;
            hard_stop(st, end_at);
            events.push(SimEvent::Dashed { unit: u.id, from: st.pos, to, at, end_at });
        }
    }
}

/// Augment damage modifiers of `owner`'s hit on `target` (06 §3), multiplied: Pebble against
/// larger hitboxes, Executioner, First Strike, Last Stand, and Spellcrit on abilities (server
/// only: the crit draws from the world rng).
fn damage_amp(units: &[Unit], owner: UnitId, target: UnitId, ability: bool, rng: &mut Pcg32) -> f32 {
    let find = |id: UnitId| units.iter().find(|u| u.id == id);
    let (Some(o), Some(t)) = (find(owner), find(target)) else { return 1.0 };
    if o.kind != UnitKind::Champion {
        return 1.0;
    }
    // Structures take more from champions late in a match (D55).
    let mut amp = t.champion_damage_taken;
    if o.gameplay_radius < CHAMPION_GAMEPLAY_RADIUS && t.gameplay_radius > o.gameplay_radius {
        amp *= augments::PEBBLE_AMP;
    }
    let m = augments::mods(&o.state.progress.augments);
    let share = |u: &Unit| u.state.health / u.stats.max_health.max(1.0);
    if m.executioner && share(t) < augments::EXECUTE_BELOW {
        amp *= augments::EXECUTE_AMP;
    }
    if m.first_strike && share(t) >= 1.0 {
        amp *= augments::FIRST_STRIKE_AMP;
    }
    if m.last_stand {
        let low = ((augments::LAST_STAND_BELOW - share(o)) / augments::LAST_STAND_BELOW).clamp(0.0, 1.0);
        amp *= 1.0 + augments::LAST_STAND_MAX * low;
    }
    if m.spellcrit && ability && rng.next_u32().is_multiple_of(augments::SPELLCRIT_ONE_IN) {
        amp *= augments::SPELLCRIT_AMP;
    }
    amp
}

/// How fast a pull drags its target.
pub const PULL_SPEED: f32 = 1800.0;

/// Lunge strikes and ally heals/shields, in time order (server only).
fn resolve_direct(units: &mut [Unit], direct: &mut [Fired], rng: &mut Pcg32, s1: SimTime, events: &mut Vec<SimEvent>) {
    let at_of = |f: &Fired| match f {
        Fired::Strike { at, owner, .. } | Fired::Support { at, owner, .. } => (*at, *owner),
        _ => (SimTime(0), UnitId(0)),
    };
    direct.sort_by_key(at_of);
    for f in direct.iter() {
        match *f {
            Fired::Strike { owner, slot, target, power, kind, cc, at } => {
                let power = power * damage_amp(units, owner, target, true, rng);
                let from = units.iter().find(|u| u.id == owner).map(|u| (u.state.pos, u.gameplay_radius));
                if let (Some((p, r)), Some(u)) = (from, units.iter_mut().find(|u| u.id == target)) {
                    // Still within reach on arrival (it may have dashed or blinked away).
                    if (u.state.pos - p).length() <= r + u.gameplay_radius + STRIKE_SLACK {
                        apply_cc(u, cc, at, p, s1, events);
                        let dealt = deal_damage(u, owner, DamageOrigin::Ability(slot), power, kind, at, events);
                        after_ability_hit(units, owner, target, dealt, at, events);
                    }
                }
            }
            Fired::Support { target, heal, heal_missing, shield, until, at, .. } => {
                let Some(u) = units.iter_mut().find(|u| u.id == target && u.state.alive()) else { continue };
                let max = u.stats.max_health;
                let st = &mut u.state;
                let amount = (heal + heal_missing * (max - st.health)).min(max - st.health);
                if amount > 0.0 {
                    st.health += amount;
                    events.push(SimEvent::Healed { unit: target, amount, at });
                }
                if shield > 0.0 {
                    st.shield = if st.shield_until > at { st.shield + shield } else { shield };
                    st.shield_until = st.shield_until.max(until);
                    events.push(SimEvent::Shielded { unit: target, amount: shield, at, until: st.shield_until });
                }
            }
            _ => {}
        }
    }
}

/// A lunge still connects if its target moved up to this far away during the dash.
pub const STRIKE_SLACK: f32 = 100.0;

/// The damage pipeline (02 §5), M1 subset: resistance mitigation, shields, health, death.
/// Returns the damage dealt (shields included).
fn deal_damage(
    u: &mut Unit,
    source: UnitId,
    origin: DamageOrigin,
    raw: f32,
    kind: DamageKind,
    at: SimTime,
    events: &mut Vec<SimEvent>,
) -> f32 {
    if raw <= 0.0 || !u.targetable() {
        return 0.0;
    }
    let resist = match kind {
        DamageKind::Physical => u.stats.armor,
        DamageKind::Magic => u.stats.magic_resist,
        DamageKind::True => 0.0,
    };
    let mut amount = if kind == DamageKind::True { raw } else { raw * resist_multiplier(resist) };
    let st = &mut u.state;
    let mut absorbed = 0.0;
    if st.shield > 0.0 && st.shield_until > at {
        absorbed = st.shield.min(amount);
        st.shield -= absorbed;
        amount -= absorbed;
    }
    st.health -= amount;
    events.push(SimEvent::Damage { source, target: u.id, origin, kind, amount, absorbed, at });
    let dealt = amount + absorbed;
    if st.health > 0.0
        && u.kind == UnitKind::Champion
        && let Some((shield, threshold, duration_ms, cooldown_ms)) = items::passives(&st.progress.items).lifeline
        && st.health < threshold * u.stats.max_health
        && st.progress.lifeline_ready <= at
    {
        // Lifeline (item passive): a shield when dropping low, on a long cooldown.
        st.shield = if st.shield_until > at { st.shield + shield } else { shield };
        st.shield_until = st.shield_until.max(at.plus(SimDuration::from_millis(duration_ms)));
        st.progress.lifeline_ready = at.plus(SimDuration::from_millis(cooldown_ms));
        events.push(SimEvent::Shielded { unit: u.id, amount: shield, at, until: st.shield_until });
    }
    if st.health <= 0.0 {
        let respawn_at = match u.kind {
            UnitKind::Champion => at.plus(respawn_time(st.progress.level)),
            UnitKind::Gatehouse => at.plus(lane::GATEHOUSE_RESPAWN),
            UnitKind::Turret | UnitKind::Base => SimTime(u64::MAX), // destroyed for good
            _ => at.plus(MINION_RESPAWN),
        };
        *st =
            UnitState { respawn_at: Some(respawn_at), progress: st.progress, ..UnitState::new(st.pos, st.move_speed) };
        events.push(SimEvent::Died { unit: u.id, killer: source, at, respawn_at });
    }
    dealt
}

/// After this tick's effects: remember champion-on-champion attacks (turrets and minions answer
/// them, 01 §3–§4) and end the match when a Base falls.
fn note_outcomes(units: &[Unit], game: &mut MatchState, events: &[SimEvent], s1: SimTime) {
    let kind = |id: UnitId| units.iter().find(|u| u.id == id).map(|u| (u.kind, u.team));
    for e in events {
        match *e {
            SimEvent::Damage { source, target, at, .. } => {
                if let (Some((UnitKind::Champion, a)), Some((UnitKind::Champion, v))) = (kind(source), kind(target))
                    && a != v
                {
                    game.aggression.push((source, target, at));
                }
            }
            SimEvent::Died { unit, at, .. } => {
                if let Some((UnitKind::Base, team)) = kind(unit)
                    && game.winner.is_none()
                {
                    let winner = if team == Team::Blue { Team::Red } else { Team::Blue };
                    game.winner = Some((winner, at));
                    game.next_wave_at = None;
                }
            }
            _ => {}
        }
    }
    let horizon = SimTime(s1.0.saturating_sub(lane::ASSIST_MEMORY.0));
    game.aggression.retain(|(_, _, t)| *t >= horizon);
}

/// Gold and experience for this tick's deaths (01 §5–§6; ranked matches only): last hits,
/// shared experience, kill bounties and assists, turret gold.
fn rewards(units: &mut [Unit], game: &MatchState, tick_events: &[SimEvent], events: &mut Vec<SimEvent>) {
    let mut pay: Vec<(UnitId, f32, u32, SimTime)> = Vec::new();
    for e in tick_events {
        let SimEvent::Died { unit, killer, at, .. } = *e else { continue };
        let Some(victim) = units.iter().find(|u| u.id == unit) else { continue };
        let (vteam, vpos) = (victim.team, victim.state.pos);
        let enemy_champ = |id: UnitId| {
            units.iter().find(|u| u.id == id).filter(|u| u.kind == UnitKind::Champion && u.team != vteam).map(|u| u.id)
        };
        let nearby: Vec<UnitId> = units
            .iter()
            .filter(|u| u.kind == UnitKind::Champion && u.team != vteam && u.state.alive())
            .filter(|u| (u.state.pos - vpos).length() <= lane::XP_RANGE)
            .map(|u| u.id)
            .collect();
        match victim.kind {
            UnitKind::Minion => {
                let (gold, xp) = lane::minion_reward(victim.attack.map_or(0.0, |a| a.range));
                if let Some(k) = enemy_champ(killer) {
                    pay.push((k, gold, 0, at));
                    if let Some(u) = units.iter_mut().find(|u| u.id == k) {
                        u.state.progress.cs = u.state.progress.cs.saturating_add(1);
                    }
                }
                let share = lane::shared_xp(xp, nearby.len());
                pay.extend(nearby.iter().map(|id| (*id, 0.0, share, at)));
            }
            UnitKind::Champion => {
                // Credit: the killer if it's a champion, else the latest champion to hurt them.
                let hurt: Vec<UnitId> =
                    game.aggression.iter().rev().filter(|(_, v, _)| *v == unit).map(|(a, ..)| *a).collect();
                let credit = enemy_champ(killer).or_else(|| hurt.iter().find_map(|a| enemy_champ(*a)));
                let (vstreak, vlevel) = (victim.state.progress.streak, victim.state.progress.level);
                let mut assists: Vec<UnitId> = Vec::new();
                for a in hurt.iter().filter_map(|a| enemy_champ(*a)) {
                    if Some(a) != credit && !assists.contains(&a) {
                        assists.push(a);
                    }
                }
                let gold = lane::bounty(vstreak);
                if let Some(k) = credit {
                    pay.push((k, gold, 0, at));
                    if let Some(u) = units.iter_mut().find(|u| u.id == k) {
                        u.state.progress.streak = u.state.progress.streak.max(0).saturating_add(1);
                        u.state.progress.kills = u.state.progress.kills.saturating_add(1);
                    }
                }
                for a in &assists {
                    if let Some(u) = units.iter_mut().find(|u| u.id == *a) {
                        u.state.progress.assists = u.state.progress.assists.saturating_add(1);
                    }
                }
                // Takedowns: Champion of Chaos counts them, Reset refreshes Q, W and E.
                for id in credit.iter().chain(&assists) {
                    let Some(u) = units.iter_mut().find(|u| u.id == *id) else { continue };
                    u.state.progress.takedowns = u.state.progress.takedowns.saturating_add(1);
                    if augments::mods(&u.state.progress.augments).reset {
                        for c in &mut u.state.cooldowns[..3] {
                            *c = (*c).min(at);
                        }
                    }
                }
                if !assists.is_empty() {
                    let each = gold * 0.5 / assists.len() as f32;
                    pay.extend(assists.iter().map(|a| (*a, each, 0, at)));
                }
                if let Some(v) = units.iter_mut().find(|u| u.id == unit) {
                    v.state.progress.streak = v.state.progress.streak.min(0).saturating_sub(1);
                    v.state.progress.deaths = v.state.progress.deaths.saturating_add(1);
                }
                let share = lane::shared_xp(140 + 30 * vlevel as u32, nearby.len());
                pay.extend(nearby.iter().map(|id| (*id, 0.0, share, at)));
            }
            UnitKind::Turret => {
                let team: Vec<UnitId> =
                    units.iter().filter(|u| u.kind == UnitKind::Champion && u.team != vteam).map(|u| u.id).collect();
                pay.extend(team.into_iter().map(|id| (id, lane::TURRET_GOLD, 0, at)));
            }
            _ => {}
        }
    }
    // One reward event per earner per tick.
    let mut earners: Vec<UnitId> = pay.iter().map(|p| p.0).collect();
    earners.sort();
    earners.dedup();
    for id in earners {
        let (gold, xp, at) =
            pay.iter().filter(|p| p.0 == id).fold((0.0, 0, SimTime(0)), |(g, x, t), p| (g + p.1, x + p.2, t.max(p.3)));
        if let Some(u) = units.iter_mut().find(|u| u.id == id) {
            let p = &mut u.state.progress;
            p.gold += gold;
            gain_xp(p, xp);
            events.push(SimEvent::Reward { unit: id, gold, xp, at });
        }
    }
}

/// Add experience, leveling up (with an ability point each level) as thresholds pass.
pub fn gain_xp(p: &mut Progress, xp: u32) {
    p.xp += xp;
    while p.level < MAX_LEVEL && p.xp >= xp_to_next(p.level) {
        p.xp -= xp_to_next(p.level);
        p.level += 1;
        p.points += 1;
    }
    if p.level == MAX_LEVEL {
        p.xp = 0;
    }
}

/// Phase 4 on lane maps: fountains heal their team's champions where the map says so (predicted
/// too: it's map data) and burn enemies (server only); a champion touching a relic takes it.
fn fountains_and_relics(units: &mut [Unit], map: &Map, prediction: bool, s1: SimTime, events: &mut Vec<SimEvent>) {
    let layout = &map.layout;
    if layout.fountains.iter().all(Option::is_none) {
        return;
    }
    for u in units.iter_mut().filter(|u| u.kind == UnitKind::Champion && u.state.alive()) {
        for (i, f) in layout.fountains.iter().enumerate() {
            let Some((c, r)) = *f else { continue };
            if (u.state.pos - c).length_sq() > r * r {
                continue;
            }
            if u.team as usize == i {
                if !layout.fountain_heals {
                    continue;
                }
                let max = u.stats.max_health;
                u.state.health = (u.state.health + max * lane::FOUNTAIN_HEAL * TICK_DT).min(max);
            } else if !prediction {
                deal_damage(
                    u,
                    UnitId(0),
                    DamageOrigin::Fountain,
                    lane::FOUNTAIN_DPS * TICK_DT,
                    DamageKind::True,
                    s1,
                    events,
                );
            }
        }
    }
    if prediction {
        return;
    }
    let relics: Vec<(UnitId, Vec2, f32)> = units
        .iter()
        .filter(|u| u.kind == UnitKind::Relic && u.state.alive())
        .map(|u| (u.id, u.state.pos, u.gameplay_radius))
        .collect();
    for (relic, pos, r) in relics {
        let taker = units.iter_mut().find(|u| {
            u.kind == UnitKind::Champion && u.state.alive() && (u.state.pos - pos).length() <= r + u.gameplay_radius
        });
        let Some(c) = taker else { continue };
        let max = c.stats.max_health;
        let amount = (max * lane::RELIC_HEAL).min(max - c.state.health);
        c.state.health += amount;
        events.push(SimEvent::Healed { unit: c.id, amount, at: s1 });
        if let Some(rel) = units.iter_mut().find(|u| u.id == relic) {
            rel.state.respawn_at = Some(s1.plus(lane::RELIC_RESPAWN));
        }
    }
}

/// Whether a unit of `other_kind`/`other_team` blocks a mover of `kind`/`team` (D11, D23).
/// Every unit with a collision radius blocks every other: allied and enemy champions, all
/// minions, turrets. Measured in the reference game (R03: a champion stops at contact with a
/// standing ally and paths around it). Kept as a function so ghosting effects can opt out.
pub fn blocks(_kind: UnitKind, _team: Team, _other_kind: UnitKind, _other_team: Team) -> bool {
    true
}

/// Stuck detection and detours (03a §5): poor progress for a few ticks → take a short detour
/// around the blockers. If the goal itself is occupied (clicked into a clump) or already
/// within reach, stop where we are, like the reference game does. A chase never gives up
/// this way: its goal is the (occupied) target itself.
fn update_stuck(state: &mut UnitState, desired: f32, achieved: f32, radius: f32, obstacles: &[Obstacle], map: &Map) {
    if desired <= 0.5 || achieved >= desired * STUCK_PROGRESS {
        state.stuck = 0;
        return;
    }
    state.stuck = state.stuck.saturating_add(1);
    if state.stuck < STUCK_TICKS {
        return;
    }
    state.stuck = 0;
    let goal = match state.order {
        Order::MoveTo(q) | Order::AttackMove(q) => q.to_vec2(),
        Order::Attack(_) => match state.path.goal() {
            Some(g) => g,
            None => return,
        },
        Order::Idle => return,
    };
    let chasing = matches!(state.order, Order::Attack(_));
    let occupied = obstacles.iter().any(|o| (goal - o.pos).length_sq() < (radius + o.radius) * (radius + o.radius));
    if !chasing && (occupied || (goal - state.pos).length() < GIVE_UP_DISTANCE) {
        state.set_order(Order::Idle, map);
        return;
    }
    let reach = 2.0 * (radius + 35.0);
    state.detour = choose_detour(state.pos, goal, radius, reach, obstacles, map.edges());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::Cc;
    use crate::champion::EMBER;
    use crate::lane;
    use crate::map::bridge_point;
    use crate::time::SUBTICKS;

    fn cmd(player: u8, seq: u32, tick: u32, sub: u8, target: (f32, f32)) -> Command {
        Command {
            player: PlayerId(player),
            seq,
            tick: Tick(tick),
            sub: SubTick::new(sub),
            kind: CommandKind::MoveTo(QPoint::from_vec2(Vec2::new(target.0, target.1))),
        }
    }

    /// A tight hex clump of minions around `center` (adjacent minions touch).
    fn clump(w: &mut World, center: Vec2, rings: i32) {
        let s = 50.0;
        for q in -rings..=rings {
            for r in -rings..=rings {
                if (q + r).abs() > rings {
                    continue;
                }
                let x = center.x + s * (q as f32 + r as f32 * 0.5);
                let y = center.y + s * 0.866_025_4 * r as f32;
                w.spawn_minion(MinionKind::Melee, Team::Red, Vec2::new(x, y), None);
            }
        }
    }

    #[test]
    fn moves_at_constant_speed_and_stops_at_target() {
        let mut w = World::new(1);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (2000.0, 1000.0))]);
        let x = w.unit(id).unwrap().state.pos.x;
        assert!((x - (1000.0 + 325.0 / 30.0)).abs() < 1e-3, "{x}");
        for _ in 0..200 {
            w.step(&[]);
        }
        let s = w.unit(id).unwrap().state;
        assert_eq!(s.pos, Vec2::new(2000.0, 1000.0));
        assert_eq!(s.order, Order::Idle);
    }

    #[test]
    fn subtick_command_moves_only_for_the_rest_of_the_tick() {
        let mut w = World::new(1);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, SUBTICKS / 2, (2000.0, 1000.0))]);
        let x = w.unit(id).unwrap().state.pos.x;
        assert!((x - (1000.0 + 325.0 / 60.0)).abs() < 1e-3, "{x}");
    }

    #[test]
    fn commands_for_other_ticks_are_ignored() {
        let mut w = World::new(1);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 5, 0, (2000.0, 1000.0))]);
        assert_eq!(w.unit(id).unwrap().state.order, Order::Idle);
    }

    #[test]
    fn champion_walks_around_a_minion_clump() {
        let mut w = World::new(1);
        clump(&mut w, Vec2::new(1500.0, 1000.0), 2);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (2000.0, 1000.0))]);
        let mut min_gap = f32::INFINITY;
        for _ in 0..300 {
            w.step(&[]);
            let me = w.unit(id).unwrap().state.pos;
            for u in w.units().iter().filter(|u| u.id != id) {
                min_gap = min_gap.min(me.distance(u.state.pos) - 60.0);
            }
        }
        let s = w.unit(id).unwrap().state;
        assert!(min_gap > -0.05, "penetrated a minion by {}", -min_gap);
        assert_eq!(s.order, Order::Idle, "never arrived: {s:?}");
        assert!(s.pos.distance(Vec2::new(2000.0, 1000.0)) < 1.0, "{s:?}");
    }

    #[test]
    fn clicking_into_a_clump_stops_at_its_edge() {
        let mut w = World::new(1);
        clump(&mut w, Vec2::new(1500.0, 1000.0), 2);
        let id = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (1500.0, 1000.0))]); // the clump's center
        for _ in 0..200 {
            w.step(&[]);
        }
        let s = w.unit(id).unwrap().state;
        assert_eq!(s.order, Order::Idle, "{s:?}");
        assert!(s.pos.x < 1400.0, "{s:?}");
    }

    #[test]
    fn patrol_minions_walk_back_and_forth() {
        let mut w = World::new(1);
        let a = QPoint::from_vec2(Vec2::new(1000.0, 1000.0));
        let b = QPoint::from_vec2(Vec2::new(1300.0, 1000.0));
        let id =
            w.spawn_minion(MinionKind::Caster, Team::Blue, a.to_vec2(), Some(Brain::Patrol { a, b, toward_b: true }));
        let mut max_x: f32 = 0.0;
        for _ in 0..90 {
            w.step(&[]);
            max_x = max_x.max(w.unit(id).unwrap().state.pos.x);
        }
        assert_eq!(max_x, 1300.0);
        assert!(w.unit(id).unwrap().state.pos.x < 1300.0, "should be heading back");
    }

    /// R03: walking at a standing champion, ally or enemy, stops at contact and then paths
    /// around it to the far side.
    #[test]
    fn champions_path_around_allied_and_enemy_champions() {
        for other_team in [Team::Blue, Team::Red] {
            let mut w = World::new(1);
            let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
            let other = w.spawn_champion(PlayerId(1), other_team, ChampionId::Ember, Vec2::new(1200.0, 1000.0)); // in the way
            w.step(&[cmd(0, 1, 1, 0, (1400.0, 1000.0))]);
            let (mut max_dev, mut min_gap) = (0.0f32, f32::INFINITY);
            for _ in 0..90 {
                w.step(&[]);
                let p = w.unit(me).unwrap().state.pos;
                max_dev = max_dev.max((p.y - 1000.0).abs());
                min_gap = min_gap.min(p.distance(w.unit(other).unwrap().state.pos));
            }
            let end = w.unit(me).unwrap().state.pos;
            assert!(min_gap >= 70.0 - 0.05, "{other_team:?}: walked through (gap {min_gap})");
            assert!(max_dev > 30.0, "{other_team:?}: should path around: {end:?}");
            assert!(end.distance(Vec2::new(1400.0, 1000.0)) < 1.0, "{other_team:?}: should arrive: {end:?}");
        }
    }

    /// Re-simulating the same commands from a snapshot must be bit-identical:
    /// this is what client prediction relies on. Players stay in separate halves of the map so
    /// the partial world (own unit only) sees the same obstacles as the full one.
    #[test]
    fn partial_world_replay_is_bit_exact() {
        let mut full = World::new(7);
        let a = full.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(500.0, 500.0));
        full.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(7000.0, 7000.0));
        let mut rng = Pcg32::new(99, 1);
        let mut cmds = Vec::new();
        for i in 0..300u32 {
            if rng.next_u32().is_multiple_of(5) {
                let p = (rng.next_u32() % 2) as u8;
                let sub = (rng.next_u32() % SUBTICKS as u32) as u8;
                let base = if p == 0 { 0.0 } else { 6000.0 };
                let t = (base + rng.range_f32(0.0, 3000.0), base + rng.range_f32(0.0, 3000.0));
                cmds.push(cmd(p, i, i + 1, sub, t));
            }
        }
        for k in 1..=100 {
            full.step(&cmds.iter().copied().filter(|c| c.tick == Tick(k)).collect::<Vec<_>>());
        }
        let mut partial = World::from_units(full.tick(), vec![full.unit(a).unwrap().clone()]);
        for k in 101..=300 {
            let tick_cmds: Vec<Command> = cmds.iter().copied().filter(|c| c.tick == Tick(k)).collect();
            full.step(&tick_cmds);
            partial.step(&tick_cmds);
            assert!(partial.unit(a).unwrap().state.bits_eq(&full.unit(a).unwrap().state), "diverged at {k}");
        }
    }

    /// Prediction with *exact* proxies (the other units' true start-of-tick positions) must
    /// reproduce the server bit-exactly, even while colliding: the property 03a §5 builds on.
    #[test]
    fn prediction_with_exact_proxies_matches_through_collisions() {
        let mut full = World::new(3);
        clump(&mut full, Vec2::new(1500.0, 1000.0), 2);
        let a = QPoint::from_vec2(Vec2::new(1200.0, 700.0));
        let b = QPoint::from_vec2(Vec2::new(1200.0, 1300.0));
        full.spawn_minion(MinionKind::Siege, Team::Blue, a.to_vec2(), Some(Brain::Patrol { a, b, toward_b: true }));
        let me = full.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        let mut rng = Pcg32::new(5, 5);
        for k in 1..=600u32 {
            let tick_cmds: Vec<Command> = if rng.next_u32().is_multiple_of(9) {
                let t = (rng.range_f32(900.0, 2100.0), rng.range_f32(600.0, 1400.0));
                vec![cmd(0, k, k, (rng.next_u32() % 64) as u8, t)]
            } else {
                Vec::new()
            };
            // Client-style prediction: own unit + every other unit as a static proxy.
            let own = full.unit(me).unwrap().clone();
            let proxies = full.units().iter().filter(|u| u.id != me).map(|u| Unit {
                state: UnitState::new(u.state.pos, 0.0),
                brain: None,
                ..u.clone()
            });
            let mut predicted = World::from_units(full.tick(), vec![own]);
            predicted.replace_others(me, proxies);
            predicted.step(&tick_cmds);
            full.step(&tick_cmds);
            assert!(predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state), "diverged at tick {k}");
        }
    }

    fn cast(player: u8, seq: u32, tick: u32, sub: u8, target: (f32, f32)) -> Command {
        cast_slot(player, seq, tick, sub, 0, target)
    }

    fn cast_slot(player: u8, seq: u32, tick: u32, sub: u8, slot: u8, target: (f32, f32)) -> Command {
        Command {
            player: PlayerId(player),
            seq,
            tick: Tick(tick),
            sub: SubTick::new(sub),
            kind: CommandKind::Cast { slot, target: QPoint::from_vec2(Vec2::new(target.0, target.1)) },
        }
    }

    fn run_until_quiet(w: &mut World, ticks: u32) -> Vec<SimEvent> {
        let mut ev = Vec::new();
        for _ in 0..ticks {
            w.step(&[]);
            ev.extend(w.take_events());
        }
        ev
    }

    #[test]
    fn cast_roots_for_the_windup_then_resumes_the_move() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (3000.0, 1000.0)), cast(0, 2, 1, 32, (1000.0, 3000.0))]);
        let ev = w.take_events();
        let fire_at = match ev[0] {
            SimEvent::CastStarted { at, fire_at, .. } => {
                assert_eq!(at, SimTime(32));
                fire_at
            }
            e => panic!("{e:?}"),
        };
        assert_eq!(fire_at, SimTime(32 + 480), "0.25 s windup = 480 sub-ticks");
        // Moved for half a tick, then rooted.
        let x_after_first = w.unit(me).unwrap().state.pos.x;
        assert!((x_after_first - (1000.0 + 325.0 / 60.0)).abs() < 1e-3);
        for _ in 0..6 {
            w.step(&[]);
        }
        assert_eq!(w.unit(me).unwrap().state.pos.x, x_after_first, "rooted during windup");
        let ev = run_until_quiet(&mut w, 3);
        assert!(
            ev.iter().any(|e| matches!(e, SimEvent::MissileSpawned(m) if m.spawn_at == fire_at && m.cast_seq == 2))
        );
        assert!(w.unit(me).unwrap().state.pos.x > x_after_first, "resumes the queued move after firing");
    }

    #[test]
    fn missile_hits_and_stuns_a_stationary_enemy() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1800.0, 1000.0));
        w.step(&[cast_slot(0, 1, 1, 0, 3, (1800.0, 1000.0))]); // Binding Sigil
        let ev = run_until_quiet(&mut w, 40);
        let hit = ev.iter().find_map(|e| match e {
            SimEvent::MissileHit { target, at, .. } => Some((*target, *at)),
            _ => None,
        });
        let (target, at) = hit.expect("should hit");
        assert_eq!(target, enemy);
        // Contact when the gap is 60 + 65 = 125 u: 675 u at 1300 u/s after the 0.3 s windup.
        let expected = 576.0 + 675.0 / 1300.0 * 1920.0;
        assert!((at.0 as f32 - expected).abs() <= 1.0, "{} vs {expected}", at.0);
        let Effect::Line(sigil) = EMBER.abilities[3].effect else { panic!() };
        let Cc::Stun(stun) = sigil.cc else { panic!() };
        let e = w.unit(enemy).unwrap().state;
        assert_eq!(e.stunned_until, at.plus(stun));
        // 100 + 0.5 × 80 AP = 140 magic damage into 30 MR.
        let dealt = ev.iter().find_map(|e| match e {
            SimEvent::Damage { target, amount, .. } if *target == enemy => Some(*amount),
            _ => None,
        });
        assert!((dealt.unwrap() - 140.0 * 100.0 / 130.0).abs() < 1e-3, "{dealt:?}");
        assert!((e.health - (600.0 - 140.0 * 100.0 / 130.0)).abs() < 1.5, "{} (plus regen)", e.health);
    }

    /// M2 slice 4: a pull drags its target to the puller, stunned until it arrives.
    #[test]
    fn grapple_pulls_its_target_in() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Bastion, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Vesper, Vec2::new(1700.0, 1000.0));
        w.step(&[cast_slot(0, 1, 1, 0, 0, (1700.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 60);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::MissileHit { target, .. } if *target == enemy)));
        let (e, b) = (w.unit(enemy).unwrap().state, w.unit(me).unwrap().state);
        assert!((e.pos.distance(b.pos) - 150.0).abs() < 1.0, "pulled to 150 u: {:?} {:?}", e.pos, b.pos);
        let dashed = ev.iter().find_map(|e| match e {
            SimEvent::Dashed { unit, end_at, .. } if *unit == enemy => Some(*end_at),
            _ => None,
        });
        assert_eq!(e.stunned_until, dashed.expect("a forced dash"), "stunned for the trip");
    }

    /// Delayed knock-up area: everyone inside at detonation is airborne (stunned) for 1 s.
    #[test]
    fn upheaval_knocks_up_everyone_inside() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Bastion, Vec2::new(1000.0, 1000.0));
        let a = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1500.0, 1000.0));
        let b = w.spawn_champion(PlayerId(2), Team::Red, ChampionId::Ember, Vec2::new(1500.0, 1200.0));
        let out = w.spawn_champion(PlayerId(3), Team::Red, ChampionId::Ember, Vec2::new(1500.0, 1700.0));
        w.step(&[cast_slot(0, 1, 1, 0, 3, (1500.0, 1100.0))]);
        let ev = run_until_quiet(&mut w, 50);
        let at = ev
            .iter()
            .find_map(|e| match e {
                SimEvent::AreaDetonated { at, .. } => Some(*at),
                _ => None,
            })
            .unwrap();
        for id in [a, b] {
            assert_eq!(w.unit(id).unwrap().state.stunned_until, at.plus(SimDuration::from_millis(1000)));
        }
        assert_eq!(w.unit(out).unwrap().state.stunned_until, SimTime(0));
    }

    /// Slows: the target walks 40% slower until the slow ends, then at full speed again.
    #[test]
    fn lull_slows_its_target() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Lumen, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1600.0, 1000.0));
        w.step(&[cast_slot(0, 1, 1, 0, 2, (1600.0, 1000.0))]);
        run_until_quiet(&mut w, 20);
        let st = w.unit(enemy).unwrap().state;
        assert_eq!(st.slow, 40);
        let now = SimTime::end_of(w.tick());
        assert!(st.slowed_until > now);
        // 325 × 0.6 = 195, under the 220 soft cap: 195 × 0.5 + 110 (02 §9).
        assert_eq!(st.speed_at(now), 207.5);
        // Walk while slowed, then after it ends.
        let x0 = st.pos.x;
        w.step(&[cmd(1, 1, w.tick().0 + 1, 0, (1600.0, 3000.0))]);
        run_until_quiet(&mut w, 29);
        let moved = w.unit(enemy).unwrap().state.pos.y - 1000.0;
        assert!((moved - 207.5).abs() < 2.0, "one second at 207.5 u/s: {moved}");
        assert_eq!(w.unit(enemy).unwrap().state.pos.x, x0);
        assert_eq!(w.unit(enemy).unwrap().state.speed_at(st.slowed_until), 325.0);
    }

    /// Ally effects pick the allied champion closest to the cursor, else the caster.
    #[test]
    fn support_heals_and_shields_the_ally_near_the_cursor() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Lumen, Vec2::new(1000.0, 1000.0));
        let ally = w.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Vesper, Vec2::new(1500.0, 1000.0));
        w.unit_mut(ally).unwrap().state.health = 300.0;
        w.unit_mut(me).unwrap().state.health = 300.0;
        w.step(&[cast_slot(0, 1, 1, 0, 0, (1520.0, 1050.0)), cast_slot(0, 2, 1, 1, 1, (1500.0, 1000.0))]);
        let ev = w.take_events();
        // 70 + 0.35 × 60 AP = 91 heal; 80 + 0.4 × 60 = 104 shield.
        assert!(ev.iter().any(
            |e| matches!(e, SimEvent::Healed { unit, amount, .. } if *unit == ally && (*amount - 91.0).abs() < 1e-3)
        ));
        assert!(ev.iter().any(
            |e| matches!(e, SimEvent::Shielded { unit, amount, .. } if *unit == ally && (*amount - 104.0).abs() < 1e-3)
        ));
        // Cursor far from any ally: the caster.
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Rook, Vec2::new(1000.0, 1000.0));
        w.unit_mut(me).unwrap().state.health = 250.0;
        w.step(&[cast_slot(0, 1, 1, 0, 1, (3000.0, 3000.0))]);
        // Second Wind: 40 + 12% of the 400 missing.
        let ev = w.take_events();
        let healed = ev.iter().find_map(|e| match e {
            SimEvent::Healed { unit, amount, .. } if *unit == me => Some(*amount),
            _ => None,
        });
        assert!((healed.unwrap() - 88.0).abs() < 1e-3, "{healed:?}");
        // A6: an instant cast still announces itself (fire_at == at) so clients can animate it.
        assert!(ev.iter().any(
            |e| matches!(e, SimEvent::CastStarted { unit, slot: 1, at, fire_at, .. } if *unit == me && at == fire_at)
        ));
    }

    /// Lunges dash to the enemy nearest the cursor and strike on arrival; with no enemy there
    /// the cast does nothing (no cooldown).
    #[test]
    fn lunges_strike_on_arrival_and_need_a_target() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Shade, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1500.0, 1000.0));
        w.step(&[cast_slot(0, 1, 1, 0, 0, (1200.0, 1500.0))]);
        assert_eq!(w.unit(me).unwrap().state.cooldowns[0], SimTime(0), "no enemy near the cursor");
        w.step(&[cast_slot(0, 2, 2, 0, 0, (1550.0, 1050.0))]);
        assert!(w.unit(me).unwrap().state.dash.is_some_and(|d| d.strike == Some((enemy, 0))));
        let ev = run_until_quiet(&mut w, 15);
        // 60 + 0.8 × 70 AD = 116 physical into 22 armor.
        let hit = damage_to(&ev, enemy);
        assert_eq!(hit.len(), 1, "{hit:?}");
        assert!((hit[0].1 - 116.0 * 100.0 / 122.0).abs() < 1e-3, "{hit:?}");
        let gap = w.unit(me).unwrap().state.pos.distance(Vec2::new(1500.0, 1000.0));
        assert!(gap < 65.0 + 65.0, "ends at the target's edge: {gap}");
    }

    /// Melee champions hit at the end of their windup; novas hit everything around them.
    #[test]
    fn melee_attacks_and_novas() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Rook, Vec2::new(1000.0, 1000.0));
        let a = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1200.0, 1000.0));
        let b = w.spawn_champion(PlayerId(2), Team::Red, ChampionId::Ember, Vec2::new(1000.0, 1250.0));
        w.step(&[attack(0, 1, 1, a)]);
        let ev = run_until_quiet(&mut w, 30);
        let hits = damage_to(&ev, a);
        assert!(!hits.is_empty() && hits.iter().all(|(_, d)| (d - 68.0 * 100.0 / 122.0).abs() < 1e-3), "{hits:?}");
        assert!(w.bolts.is_empty(), "melee: no bolts");
        let k = w.tick().0 + 1;
        w.step(&[cast_slot(0, 2, k, 0, 0, (1000.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 10);
        // Cleave: 40 + 1.0 × 68 AD = 108 to both, around Rook.
        for id in [a, b] {
            assert!(damage_to(&ev, id).iter().any(|(_, d)| (d - 108.0 * 100.0 / 122.0).abs() < 1e-3), "{id:?}");
        }
        assert!(w.unit(me).unwrap().state.alive());
    }

    #[test]
    fn walking_out_in_time_dodges_and_too_late_is_hit() {
        for (react_tick, expect_hit) in [(6u32, false), (20u32, true)] {
            let mut w = World::new(1);
            w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
            w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1800.0, 1000.0));
            let mut ev = Vec::new();
            for k in 1..=45u32 {
                let mut c = Vec::new();
                if k == 1 {
                    c.push(cast(0, 1, 1, 0, (1800.0, 1000.0)));
                }
                if k == react_tick {
                    c.push(cmd(1, 2, k, 0, (1800.0, 1400.0)));
                }
                w.step(&c);
                ev.extend(w.take_events());
            }
            let hit = ev.iter().any(|e| matches!(e, SimEvent::MissileHit { .. }));
            assert_eq!(hit, expect_hit, "reacting at tick {react_tick}");
        }
    }

    #[test]
    fn an_enemy_minion_in_the_way_takes_the_skillshot() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        let minion = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1400.0, 1010.0), None);
        w.spawn_minion(MinionKind::Caster, Team::Blue, Vec2::new(1200.0, 1000.0), None); // allied: ignored
        w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1800.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (1800.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 40);
        let targets: Vec<UnitId> = ev
            .iter()
            .filter_map(|e| match e {
                SimEvent::MissileHit { target, .. } => Some(*target),
                _ => None,
            })
            .collect();
        assert_eq!(targets, vec![minion]);
    }

    #[test]
    fn cooldown_blocks_recast_and_expired_missiles_report() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (2000.0, 1000.0))]);
        w.step(&[cast(0, 2, 2, 0, (2000.0, 1000.0))]); // on cooldown (and still casting)
        let ev = run_until_quiet(&mut w, 60);
        let mut all = w.take_events();
        all.extend(ev);
        let spawned = all.iter().filter(|e| matches!(e, SimEvent::MissileSpawned(_))).count();
        assert_eq!(spawned, 1);
        assert!(all.iter().any(|e| matches!(e, SimEvent::MissileExpired { .. })));
    }

    #[test]
    fn turret_shoots_the_nearest_enemy_champion() {
        let mut w = World::new(1);
        w.spawn_rig_turret(Team::Red, Vec2::new(1000.0, 1000.0), 1100);
        let target = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1600.0, 1000.0));
        w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1300.0, 1000.0)); // ally of the turret
        let ev = run_until_quiet(&mut w, 40);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::MissileHit { target: t, .. } if *t == target)));
    }

    /// Client-style prediction (missiles disabled) must still reproduce the caster exactly.
    #[test]
    fn prediction_without_missiles_matches_the_caster() {
        let mut full = World::new(9);
        let me = full.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        full.spawn_rig_turret(Team::Red, Vec2::new(5000.0, 5000.0), 1100); // out of range
        let mut predicted = World::from_units(full.tick(), vec![full.unit(me).unwrap().clone()]);
        predicted.set_prediction_mode(true);
        let mut rng = Pcg32::new(3, 3);
        for k in 1..=900u32 {
            let mut c = Vec::new();
            if rng.next_u32().is_multiple_of(7) {
                let t = (rng.range_f32(500.0, 2500.0), rng.range_f32(500.0, 2500.0));
                let sub = (rng.next_u32() % 64) as u8;
                c.push(if rng.next_u32().is_multiple_of(3) {
                    cast_slot(0, k, k, sub, (rng.next_u32() % 6) as u8, t)
                } else {
                    cmd(0, k, k, sub, t)
                });
            }
            full.step(&c);
            predicted.step(&c);
            assert!(predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state), "tick {k}");
        }
    }

    fn attack(player: u8, seq: u32, tick: u32, target: UnitId) -> Command {
        Command {
            player: PlayerId(player),
            seq,
            tick: Tick(tick),
            sub: SubTick::START,
            kind: CommandKind::Attack(target),
        }
    }

    fn damage_to(ev: &[SimEvent], who: UnitId) -> Vec<(SimTime, f32)> {
        ev.iter()
            .filter_map(|e| match e {
                SimEvent::Damage { target, amount, absorbed, at, .. } if *target == who => {
                    Some((*at, amount + absorbed))
                }
                _ => None,
            })
            .collect()
    }

    /// 02 §7: chase into range, wind up (rooted), launch a homing bolt, repeat on the timer.
    #[test]
    fn basic_attacks_chase_wind_up_and_hit_on_the_timer() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1900.0, 1000.0));
        w.step(&[attack(0, 1, 1, enemy)]);
        let mut ev = w.take_events();
        ev.extend(run_until_quiet(&mut w, 150)); // 5 s
        let reach = 575.0 + 65.0;
        let p = w.unit(me).unwrap().state.pos;
        assert!((p.distance(Vec2::new(1900.0, 1000.0)) - reach).abs() < 1.0, "stops at range: {p:?}");
        let hits = damage_to(&ev, enemy);
        // Walk 260 u (0.8 s), wind up 0.225 s, fly 575 u at 2200 u/s: first hit at ~1.29 s,
        // then one every 1.25 s.
        assert_eq!(hits.len(), 3, "{hits:?}");
        assert!((hits[0].0.0 as f32 - (1536.0 + 432.0 + 575.0 / 2200.0 * 1920.0)).abs() < 2.0, "{hits:?}");
        let per_hit = 66.0 * 100.0 / 122.0;
        assert!(hits.iter().all(|(_, a)| (a - per_hit).abs() < 1e-3), "{hits:?}");
        let gaps: Vec<u64> = hits.windows(2).map(|h| h[1].0.0 - h[0].0.0).collect();
        assert!(gaps.iter().all(|g| *g == 2400), "attack period 1.25 s: {gaps:?}");
    }

    /// Orb-walk cancel: a move order during the windup cancels the attack, with no damage.
    #[test]
    fn moving_during_the_windup_cancels_the_attack() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1500.0, 1000.0));
        w.step(&[attack(0, 1, 1, enemy)]);
        assert!(w.unit(UnitId(1)).unwrap().state.attack.is_some(), "in range: winds up at once");
        w.step(&[cmd(0, 2, 2, 0, (900.0, 1000.0))]); // windup is 0.225 s: still winding up
        let ev = run_until_quiet(&mut w, 60);
        assert!(damage_to(&ev, enemy).is_empty());
        assert!(!ev.iter().any(|e| matches!(e, SimEvent::AttackLaunched(_))));
    }

    // A2 (D52): facing, follow-throughs, the input buffer and the attack counter.

    fn state(w: &World, id: UnitId) -> UnitState {
        w.unit(id).unwrap().state
    }

    fn step_events(w: &mut World, cmds: &[Command]) -> Vec<SimEvent> {
        w.step(cmds);
        w.take_events()
    }

    /// Steps until `stop` holds for the unit (at most `ticks`), returning the events.
    fn run_while(w: &mut World, id: UnitId, ticks: u32, keep: impl Fn(&UnitState) -> bool) -> Vec<SimEvent> {
        let mut ev = Vec::new();
        for _ in 0..ticks {
            if !keep(&state(w, id)) {
                break;
            }
            ev.extend(step_events(w, &[]));
        }
        ev
    }

    fn cast_started(ev: &[SimEvent], slot: u8) -> Option<(SimTime, SimTime)> {
        ev.iter().find_map(|e| match *e {
            SimEvent::CastStarted { slot: s, at, fire_at, .. } if s == slot => Some((at, fire_at)),
            _ => None,
        })
    }

    #[test]
    fn facing_follows_the_path_the_attack_target_and_the_aim() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (1000.0, 2000.0))]);
        assert_eq!(state(&w, me).facing, Vec2::new(0.0, 1.0), "walking: faces the path");

        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1400.0, 1033.0));
        w.step(&[attack(0, 2, 2, enemy)]);
        let st = state(&w, me);
        assert!(st.attack.is_some());
        let to_enemy = (Vec2::new(1400.0, 1033.0) - st.pos).normalize_or_zero();
        assert!(st.facing.distance(to_enemy) < 1e-6, "attacking: faces the target");

        w.step(&[cast_slot(0, 3, 3, 0, 0, (st.pos.x, 0.0))]);
        assert_eq!(state(&w, me).facing, Vec2::new(0.0, -1.0), "casting: faces the aim");
    }

    #[test]
    fn an_idle_caster_holds_the_follow_through_and_a_move_cuts_it() {
        for cut in [false, true] {
            let mut w = World::new(1);
            let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
            let ev = step_events(&mut w, &[cast_slot(0, 1, 1, 0, 0, (2000.0, 1000.0))]);
            let (_, fire_at) = cast_started(&ev, 0).unwrap();
            run_while(&mut w, me, 30, |s| s.cast.is_some());
            let r = state(&w, me).recovery.expect("Longshot's follow-through");
            assert_eq!(r.until, fire_at.plus(SimDuration::from_millis(200)));
            assert_eq!(r.hard_until, fire_at, "no hard lock on a basic ability");
            let before = state(&w, me).pos;
            if cut {
                let k = w.tick().next().0;
                w.step(&[cmd(0, 2, k, 0, (1000.0, 1500.0))]);
                assert!(state(&w, me).recovery.is_none(), "a move ends it");
                assert!(state(&w, me).pos.distance(before) > 5.0, "and the caster walks at once");
            } else {
                // Without orders the caster waits it out in place, then is free.
                run_while(&mut w, me, 30, |s| s.recovery.is_some());
                assert!(SimTime::end_of(w.tick()) >= r.until);
                assert_eq!(state(&w, me).pos, before);
            }
            assert!(state(&w, me).recovery.is_none());
        }
    }

    #[test]
    fn a_caster_walking_on_skips_the_follow_through() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        w.step(&[cmd(0, 1, 1, 0, (3000.0, 1000.0))]);
        w.step(&[cast_slot(0, 2, 2, 0, 0, (1000.0, 2000.0))]);
        run_while(&mut w, me, 30, |s| s.cast.is_some());
        assert!(state(&w, me).recovery.is_none(), "still walking somewhere: no follow-through");
        let x = state(&w, me).pos.x;
        w.step(&[]);
        assert!(state(&w, me).pos.x > x, "walks on toward the old destination");
    }

    #[test]
    fn an_attack_order_waits_for_the_follow_through_and_a_move_first_is_faster() {
        // When does the first attack start after a cast, with and without the move-cancel?
        let first_attack = |cancel: bool| {
            let mut w = World::new(1);
            let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
            let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1450.0, 1000.0));
            w.step(&[cast_slot(0, 1, 1, 0, 0, (2000.0, 1000.0))]);
            w.step(&[attack(0, 2, 2, enemy)]); // during the windup: the attack order waits
            run_while(&mut w, me, 30, |s| s.cast.is_some());
            let until = state(&w, me).recovery.map(|r| r.until);
            if cancel {
                // The tech: a move (on the spot) cuts the follow-through, then attack again.
                let k = w.tick().next().0;
                let here = state(&w, me).pos;
                w.step(&[cmd(0, 3, k, 0, (here.x, here.y)), attack(0, 4, k, enemy)]);
            }
            for _ in 0..30 {
                if state(&w, me).attack.is_some() {
                    break;
                }
                w.step(&[]);
            }
            (w.tick(), until.unwrap())
        };
        let (waited, until) = first_attack(false);
        let (cut, _) = first_attack(true);
        assert!(SimTime::end_of(waited) >= until, "the attack started only after the follow-through");
        assert!(cut < waited, "cancelling is faster: {cut:?} vs {waited:?}");
    }

    #[test]
    fn casts_during_a_windup_or_a_dash_are_buffered() {
        // Q during W's windup starts the moment W fires.
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let ev = step_events(&mut w, &[cast_slot(0, 1, 1, 0, 1, (1500.0, 1000.0))]);
        let (_, w_fires) = cast_started(&ev, 1).unwrap();
        let ev = step_events(&mut w, &[cast_slot(0, 2, 2, 0, 0, (2000.0, 1000.0))]);
        assert!(cast_started(&ev, 0).is_none(), "busy: buffered, not started");
        assert_eq!(state(&w, me).buffered.map(|b| b.slot), Some(0));
        let ev = run_until_quiet(&mut w, 15);
        assert_eq!(cast_started(&ev, 0).map(|c| c.0), Some(w_fires), "Q starts as W fires");
        assert!(state(&w, me).buffered.is_none());

        // Q during Tumble (the dash) starts on landing.
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let ev = step_events(&mut w, &[cast_slot(0, 1, 1, 0, 2, (1300.0, 1000.0))]);
        let lands = ev.iter().find_map(|e| match *e {
            SimEvent::Dashed { end_at, .. } => Some(end_at),
            _ => None,
        });
        w.step(&[cast_slot(0, 2, 2, 0, 0, (2000.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 15);
        assert_eq!(cast_started(&ev, 0).map(|c| c.0), lands, "Q starts as Tumble lands");
        assert!(state(&w, me).buffered.is_none());

        // A newer order replaces the buffered cast (one slot, the latest wins).
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        w.step(&[cast_slot(0, 1, 1, 0, 1, (1500.0, 1000.0))]);
        w.step(&[cast_slot(0, 2, 2, 0, 0, (2000.0, 1000.0)), cmd(0, 3, 2, 10, (1000.0, 1500.0))]);
        let ev = run_until_quiet(&mut w, 15);
        assert!(cast_started(&ev, 0).is_none(), "the move replaced the buffered Q");
    }

    #[test]
    fn an_ultimate_hard_locks_then_releases_to_buffered_orders() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let ev = step_events(&mut w, &[cast_slot(0, 1, 1, 0, 3, (2000.0, 1000.0))]);
        let (_, fires) = cast_started(&ev, 3).unwrap();
        run_while(&mut w, me, 30, |s| s.cast.is_some());
        let r = state(&w, me).recovery.unwrap();
        assert_eq!(r.hard_until, fires.plus(SimDuration::from_millis(150)));
        assert_eq!(r.until, fires.plus(SimDuration::from_millis(350)));
        // During the hard lock: a move doesn't free the caster, a cast waits in the buffer.
        let k = w.tick().next().0;
        let here = state(&w, me).pos;
        let ev = step_events(&mut w, &[cmd(0, 2, k, 0, (1000.0, 2000.0)), cast_slot(0, 3, k, 1, 0, (2000.0, 1000.0))]);
        assert!(cast_started(&ev, 0).is_none());
        assert!(state(&w, me).hard_locked(SimTime::end_of(w.tick())));
        assert_eq!(state(&w, me).pos, here, "rooted in the hard lock");
        let ev = run_until_quiet(&mut w, 10);
        assert_eq!(cast_started(&ev, 0).map(|c| c.0), Some(r.hard_until), "the buffered cast goes as it ends");
    }

    #[test]
    fn stuns_clear_the_buffer_and_the_follow_through() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        w.step(&[cast_slot(0, 1, 1, 0, 1, (1500.0, 1000.0))]);
        w.step(&[cast_slot(0, 2, 2, 0, 0, (2000.0, 1000.0))]);
        let s1 = SimTime::end_of(w.tick());
        let u = w.unit_mut(me).unwrap();
        apply_cc(u, Cc::Stun(SimDuration::from_millis(500)), s1, Vec2::ZERO, s1, &mut Vec::new());
        assert!(u.state.buffered.is_none() && u.state.recovery.is_none() && u.state.cast.is_none());
    }

    #[test]
    fn attacks_are_counted_for_their_animation() {
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1450.0, 1000.0));
        w.step(&[attack(0, 1, 1, enemy)]);
        assert_eq!(state(&w, me).attacks, 1);
        run_until_quiet(&mut w, 80); // every 1.25 s (0.8 per second): two more in 2.67 s
        assert_eq!(state(&w, me).attacks, 3);
    }

    #[test]
    fn attack_move_attacks_the_nearest_enemy_in_range() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let near = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1700.0, 1100.0), None);
        let far = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1900.0, 900.0), None);
        w.spawn_minion(MinionKind::Caster, Team::Blue, Vec2::new(1400.0, 1000.0), None); // ally: ignored
        let q = QPoint::from_vec2(Vec2::new(3000.0, 1000.0));
        w.step(&[Command {
            player: PlayerId(0),
            seq: 1,
            tick: Tick(1),
            sub: SubTick::START,
            kind: CommandKind::AttackMove(q),
        }]);
        let ev = run_until_quiet(&mut w, 90);
        assert!(!damage_to(&ev, near).is_empty());
        assert!(damage_to(&ev, far).is_empty(), "the nearer one first");
        assert!(w.unit(UnitId(1)).unwrap().state.pos.x < 1500.0, "stopped to attack");
    }

    #[test]
    fn delayed_area_hits_who_stays_and_misses_who_walks_out() {
        for (walk, expect_hit) in [(false, true), (true, false)] {
            let mut w = World::new(1);
            w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
            let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1700.0, 1000.0));
            w.step(&[cast_slot(0, 1, 1, 0, 1, (1700.0, 1000.0))]); // Cinder Bloom on the target
            let mut ev = w.take_events();
            for k in 2..=60 {
                // React after 0.4 s and walk straight out (needs (160 + 65) / 325 ≈ 0.69 s).
                let c = if walk && k == 13 { vec![cmd(1, 2, k, 0, (1700.0, 1400.0))] } else { vec![] };
                w.step(&c);
                ev.extend(w.take_events());
            }
            let det = ev.iter().find_map(|e| match e {
                SimEvent::AreaDetonated { at, .. } => Some(*at),
                _ => None,
            });
            assert_eq!(det, Some(SimTime(480 + 1632)), "0.25 s windup + 0.85 s delay");
            assert_eq!(!damage_to(&ev, enemy).is_empty(), expect_hit, "walk={walk}");
        }
    }

    #[test]
    fn dash_ignores_units_and_blink_crosses_a_thin_wall() {
        let mut w = World::new(1);
        let v = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        clump(&mut w, Vec2::new(1160.0, 1000.0), 1); // right in the dash path
        w.step(&[cast_slot(0, 1, 1, 0, 2, (2000.0, 1000.0))]); // Tumble: 325 u at 1000 u/s
        // A10: a dash announces its slot as an instant cast, so clients pick its clips.
        assert!(w.take_events().iter().any(
            |ev| matches!(ev, SimEvent::CastStarted { unit, slot: 2, at, fire_at, .. } if *unit == v && at == fire_at)
        ));
        for _ in 0..12 {
            w.step(&[]);
        }
        let s = w.unit(v).unwrap().state;
        assert!(s.dash.is_none() && (s.pos.x - 1325.0).abs() < 0.01 && s.pos.y == 1000.0, "{:?}", s.pos);

        let mut w = arena_world(1);
        let e = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(2300.0, 1200.0));
        w.step(&[cast_slot(0, 1, 1, 0, 4, (2900.0, 1200.0))]); // Blink 400 u over the 80 u wall
        let s = w.unit(e).unwrap().state;
        assert_eq!(s.pos, Vec2::new(2700.0, 1200.0));
        // A7: blinks are instant casts and announce themselves (fire_at == at) for the animation.
        assert!(w.take_events().iter().any(
            |ev| matches!(ev, SimEvent::CastStarted { unit, slot: 4, at, fire_at, .. } if *unit == e && at == fire_at)
        ));
        assert!(s.cooldowns[4] > SimTime(0));
        // Into the wall: lands short of it, never inside.
        let e2 = w.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Ember, Vec2::new(2100.0, 1000.0));
        w.step(&[cast_slot(1, 2, 2, 0, 4, (2420.0, 1000.0))]);
        let p = w.unit(e2).unwrap().state.pos;
        assert!(w.map().walkable(p, CHAMPION_COLLISION_RADIUS) && p.x < 2380.0 && p.x > 2300.0, "{p:?}");
    }

    #[test]
    fn barrier_absorbs_damage_then_expires() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1800.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (1800.0, 1000.0)), cast_slot(1, 2, 1, 0, 5, (1800.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 30);
        let lance = 118.0 * 100.0 / 130.0;
        let hit = ev.iter().find_map(|e| match e {
            SimEvent::Damage { amount, absorbed, .. } => Some((*amount, *absorbed)),
            _ => None,
        });
        assert_eq!(hit.map(|h| h.0), Some(0.0));
        assert!((hit.unwrap().1 - lance).abs() < 1e-3);
        let s = w.unit(enemy).unwrap().state;
        assert!((s.shield - (150.0 - lance)).abs() < 1e-3 && s.health == 600.0, "{s:?}");
        run_until_quiet(&mut w, 60);
        assert_eq!(w.unit(enemy).unwrap().state.shield, 0.0, "expired after 2.5 s");
    }

    #[test]
    fn root_stops_movement_but_not_attacks() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Vesper, Vec2::new(1600.0, 1000.0));
        let me = UnitId(1);
        w.step(&[cast_slot(0, 1, 1, 0, 3, (1600.0, 1000.0))]); // Snare Net
        let ev = run_until_quiet(&mut w, 30);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::MissileHit { target, .. } if *target == enemy)));
        let rooted = w.unit(enemy).unwrap().state;
        assert!(rooted.rooted_until > SimTime::end_of(w.tick()));
        let k = w.tick().0 + 1;
        w.step(&[cmd(1, 2, k, 0, (1700.0, 2000.0)), attack(1, 3, k, me)]);
        let ev = run_until_quiet(&mut w, 15);
        assert_eq!(w.unit(enemy).unwrap().state.pos, rooted.pos, "rooted: can't walk");
        assert!(!damage_to(&ev, me).is_empty(), "rooted: can still attack");
    }

    #[test]
    fn units_die_and_respawn_at_home_with_full_health() {
        let mut w = World::new(1);
        w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let minion = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1500.0, 1000.0), None);
        w.step(&[attack(0, 1, 1, minion)]);
        let ev = run_until_quiet(&mut w, 200);
        let died = ev.iter().find_map(|e| match e {
            SimEvent::Died { unit, respawn_at, .. } if *unit == minion => Some(*respawn_at),
            _ => None,
        });
        // 300 HP, 66 damage per attack: the 5th attack kills it.
        assert_eq!(damage_to(&ev, minion).len(), 5);
        let respawn_at = died.expect("minion should die");
        assert_eq!(w.unit(UnitId(1)).unwrap().state.order, Order::Idle, "target gone: stop attacking");
        assert!(!w.unit(minion).unwrap().state.alive());
        let ev = run_until_quiet(&mut w, 400);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::Respawned { unit, .. } if *unit == minion)));
        let m = w.unit(minion).unwrap();
        assert!(m.state.alive() && m.state.health == 300.0 && m.state.pos == m.home);
        assert!(SimTime::end_of(w.tick()) >= respawn_at);
    }

    /// Prediction stays bit-exact for every champion using its whole kit (skillshots, areas,
    /// novas, dashes, lunges, blinks, heals and shields on itself or an ally) and attacking units
    /// it only knows as proxies.
    #[test]
    fn prediction_matches_through_the_whole_kit_and_attacks() {
        for champ in ChampionId::ALL {
            let mut full = arena_world(12);
            let me = full.spawn_champion(PlayerId(0), Team::Blue, champ, Vec2::new(1000.0, 3300.0));
            for i in 0..4 {
                full.spawn_minion(MinionKind::Siege, Team::Red, Vec2::new(1300.0 + 350.0 * i as f32, 3100.0), None);
            }
            // An ally for heals and shields to pick (M2 slice 4).
            full.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Vesper, Vec2::new(1500.0, 3450.0));
            let mut rng = Pcg32::new(4, 4);
            for k in 1..=2400u32 {
                let mut c = Vec::new();
                if rng.next_u32().is_multiple_of(9) {
                    let t = (rng.range_f32(600.0, 2600.0), rng.range_f32(2800.0, 3700.0));
                    let sub = (rng.next_u32() % 64) as u8;
                    let r = rng.next_u32() % 10;
                    c.push(match r {
                        0..=3 => cmd(0, k, k, sub, t),
                        4 | 5 => {
                            let targets: Vec<UnitId> = full
                                .units()
                                .iter()
                                .filter(|u| u.team == Team::Red && u.state.alive())
                                .map(|u| u.id)
                                .collect();
                            let mut a = attack(0, k, k, targets[(rng.next_u32() as usize) % targets.len()]);
                            a.sub = SubTick::new(sub);
                            a
                        }
                        6 => Command {
                            kind: CommandKind::AttackMove(QPoint::from_vec2(Vec2::new(t.0, t.1))),
                            ..cmd(0, k, k, sub, t)
                        },
                        _ => cast_slot(0, k, k, sub, (rng.next_u32() % 6) as u8, t),
                    });
                }
                let own = full.unit(me).unwrap().clone();
                let proxies = full.units().iter().filter(|u| u.id != me && u.state.alive()).map(|u| Unit {
                    state: UnitState::new(u.state.pos, 0.0),
                    brain: None,
                    ..u.clone()
                });
                let mut predicted = World::from_units(full.tick(), vec![own]);
                predicted.set_map(MapId::Arena.shared());
                predicted.set_prediction_mode(true);
                predicted.replace_others(me, proxies);
                predicted.step(&c);
                full.step(&c);
                full.take_events();
                assert!(
                    predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state),
                    "{champ:?} tick {k}\n{:?}\n{:?}",
                    predicted.unit(me).unwrap().state,
                    full.unit(me).unwrap().state
                );
            }
        }
    }

    fn bridge_world() -> World {
        let mut w = World::new(5);
        w.set_map(MapId::Bridge.shared());
        w.start_match();
        w
    }

    fn deaths(ev: &[SimEvent], w: &World) -> Vec<(UnitKind, Team, u8)> {
        ev.iter()
            .filter_map(|e| match e {
                SimEvent::Died { unit, .. } => w.unit(*unit).map(|u| (u.kind, u.team, u.tier)),
                _ => None,
            })
            .collect()
    }

    /// M2 slice 1: waves meet mid-lane and fight; turrets defend; with no champions around,
    /// no structure falls and nothing gets stuck.
    #[test]
    fn bridge_waves_meet_fight_and_turrets_defend() {
        let mut w = bridge_world();
        let mut ev = Vec::new();
        for _ in 0..(30 * 150) {
            w.step(&[]);
            ev.extend(w.take_events());
        }
        // Dead lane minions are removed the next tick, so anything that died and is gone was one.
        let died = ev.iter().filter(|e| matches!(e, SimEvent::Died { .. })).count();
        let d = deaths(&ev, &w);
        assert!(died - d.len() > 30, "waves should fight: {} minion deaths", died - d.len());
        assert!(d.iter().all(|(k, ..)| *k == UnitKind::Minion), "only minions die: {d:?}");
        assert!(w.game().winner.is_none());
        assert_eq!(w.game().waves_spawned, 4, "0:50, 1:15, 1:40, 2:05");
        // Lane minions stay on the lane (nobody wandered off into a corner).
        for u in w.units().iter().filter(|u| u.kind == UnitKind::Minion) {
            let across = crate::map::bridge_lane_point(u.state.pos).y;
            assert!(across > 600.0 && across < 2400.0, "{:?}", u.state.pos);
        }
    }

    #[test]
    fn turrets_answer_a_champion_attacking_an_allied_champion() {
        let mut w = bridge_world();
        let ally =
            w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, bridge_point(Vec2::new(4700.0, 1500.0)));
        let enemy =
            w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Vesper, bridge_point(Vec2::new(5300.0, 1500.0)));
        w.step(&[attack(1, 1, 1, ally)]);
        let mut ev = w.take_events();
        ev.extend(run_until_quiet(&mut w, 90));
        let shots_at_enemy = ev
            .iter()
            .filter(|e| matches!(e, SimEvent::AttackLaunched(b) if b.target == enemy && w.unit(b.owner).is_some_and(|u| u.kind == UnitKind::Turret)))
            .count();
        assert!(shots_at_enemy >= 2, "the blue outer turret should shoot the aggressor");
        // Consecutive shots ramp up.
        let dmg: Vec<f32> = damage_to(&ev, enemy).into_iter().map(|(_, a)| a).filter(|a| *a > 60.0).collect();
        assert!(dmg.windows(2).any(|p| p[1] > p[0]), "{dmg:?}");
    }

    /// D53: +50% per consecutive champion shot up to +150%; the heat survives a switch of target
    /// and minion shots, and cools 5 s after the last champion shot. Damage grows per wave.
    #[test]
    fn turret_heat_ramps_to_two_and_a_half_times_and_cools_after_five_seconds() {
        let mut brain = Some(Brain::Tower { heat: 0, cools_at: SimTime(0) });
        let at = |ms: u64| SimTime(ms * crate::time::SUBTICKS_PER_SECOND / 1000);
        let mut shot = |ms: u64, kind: UnitKind| lane::turret_shot(&mut brain, 100.0, at(ms), kind, 550.0, 1000.0).0;
        let ramp: Vec<f32> =
            [1_000, 2_200, 3_400, 4_600, 5_800].iter().map(|&ms| shot(ms, UnitKind::Champion)).collect();
        assert_eq!(ramp, vec![100.0, 150.0, 200.0, 250.0, 250.0]);
        // A minion shot in between leaves the heat alone.
        assert_eq!(shot(7_000, UnitKind::Minion), 700.0, "70% of a caster");
        assert_eq!(shot(8_000, UnitKind::Champion), 250.0);
        // Five seconds without a champion shot: cold again.
        assert_eq!(shot(13_000, UnitKind::Champion), 100.0);
        assert_eq!(lane::turret_damage(0), 185.0);
        assert_eq!(lane::turret_damage(10), 230.0);
        assert_eq!(lane::turret_damage(40), 293.0);
    }

    /// A strong champion pushing alone takes every red structure strictly in lane order, and
    /// the Base falling ends the match.
    #[test]
    fn a_push_destroys_structures_in_order_and_ends_the_match() {
        let mut w = bridge_world();
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, bridge_point(Vec2::new(450.0, 1500.0)));
        {
            let u = w.unit_mut(me).unwrap();
            u.stats.attack_damage = 900.0;
            u.stats.max_health = 1.0e7;
            u.stats.health_regen = 1.0e5;
            u.state.health = 1.0e7;
        }
        let goal = QPoint::from_vec2(bridge_point(Vec2::new(10_750.0, 1500.0)));
        let mut ev = Vec::new();
        let mut seq = 0;
        for k in 1..=(30 * 600u32) {
            let mut c = Vec::new();
            if k % 60 == 1 {
                seq += 1;
                c.push(Command {
                    player: PlayerId(0),
                    seq,
                    tick: Tick(k),
                    sub: SubTick::START,
                    kind: CommandKind::AttackMove(goal),
                });
            }
            w.step(&c);
            ev.extend(w.take_events());
            if w.game().winner.is_some() {
                break;
            }
        }
        let tiers: Vec<u8> = deaths(&ev, &w)
            .into_iter()
            .filter(|(k, t, _)| *t == Team::Red && *k != UnitKind::Minion)
            .map(|(.., tier)| tier)
            .collect();
        assert_eq!(tiers, vec![1, 2, 3, 4, 5, 5, 6], "structures fall in lane order");
        assert!(matches!(w.game().winner, Some((Team::Blue, _))));
        assert!(ev.iter().any(|e| matches!(e, SimEvent::MatchEnded { winner: Team::Blue, .. })));
    }

    /// While a red Gatehouse is down, blue waves bring a super minion; red's don't.
    #[test]
    fn a_fallen_gatehouse_empowers_the_other_teams_waves() {
        let mut w = bridge_world();
        let gate = w.units().iter().find(|u| u.kind == UnitKind::Gatehouse && u.team == Team::Red).unwrap().id;
        w.unit_mut(gate).unwrap().state.respawn_at = Some(SimTime(u64::MAX));
        let supers = |w: &World, team: Team| {
            w.units()
                .iter()
                .filter(|u| {
                    u.kind == UnitKind::Minion
                        && u.team == team
                        && u.attack.is_some_and(|a| MinionKind::from_attack_range(a.range) == MinionKind::Super)
                })
                .count()
        };
        while w.game().waves_spawned == 0 {
            w.step(&[]);
        }
        assert_eq!(supers(&w, Team::Blue), 1);
        assert_eq!(supers(&w, Team::Red), 0);
        let s = w.units().iter().find(|u| u.attack.is_some_and(|a| a.range == 170.0)).unwrap();
        assert_eq!(
            (s.stats.max_health, s.stats.armor, s.collision_radius, s.gameplay_radius),
            // The first wave comes at 0:50, after one upgrade (+100 health for a super).
            (1600.0, 100.0, 45.0, 80.0)
        );
    }

    #[test]
    fn relics_heal_and_respawn_and_the_bridge_fountain_does_not_heal() {
        let mut w = bridge_world();
        let relic = w.units().iter().find(|u| u.kind == UnitKind::Relic).unwrap().clone();
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, relic.state.pos + Vec2::new(-200.0, 0.0));
        w.unit_mut(me).unwrap().state.health = 100.0;
        w.step(&[cmd(0, 1, 1, 0, (relic.state.pos.x + 200.0, relic.state.pos.y))]);
        let ev = run_until_quiet(&mut w, 30);
        assert!(
            ev.iter().any(|e| matches!(e, SimEvent::Healed { unit, amount, .. } if *unit == me && *amount == 150.0))
        );
        assert!(!w.unit(relic.id).unwrap().state.alive(), "taken");
        run_until_quiet(&mut w, 30 * 41);
        assert!(w.unit(relic.id).unwrap().state.alive(), "back after 40 s");
        // The Bridge is ARAM's: back in the fountain, only regeneration.
        let u = w.unit_mut(me).unwrap();
        u.state = UnitState { health: 100.0, ..UnitState::new(bridge_point(Vec2::new(400.0, 1500.0)), 325.0) };
        run_until_quiet(&mut w, 30);
        let hp = w.unit(me).unwrap().state.health;
        assert!((hp - (100.0 + 1.5)).abs() < 1.0, "{hp}");
    }

    /// On a map whose fountains heal: 15% of max health per second.
    #[test]
    fn a_healing_fountain_heals_its_own_team() {
        let mut map = (*MapId::Bridge.shared()).clone();
        map.layout.fountain_heals = true;
        let mut w = World::new(5);
        w.set_map(Arc::new(map));
        w.start_match();
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, bridge_point(Vec2::new(400.0, 1500.0)));
        w.unit_mut(me).unwrap().state.health = 100.0;
        run_until_quiet(&mut w, 30);
        let hp = w.unit(me).unwrap().state.health;
        assert!((hp - (100.0 + 90.0 + 1.5)).abs() < 1.0, "{hp}");
    }

    fn ranked_world() -> World {
        let mut w = World::new(2);
        w.set_rules(Rules::ARAM);
        w
    }

    fn level_up(player: u8, seq: u32, tick: u32, slot: u8) -> Command {
        Command {
            player: PlayerId(player),
            seq,
            tick: Tick(tick),
            sub: SubTick::START,
            kind: CommandKind::LevelUp(slot),
        }
    }

    /// M3 slice 1: in ARAM: Mayhem a champion is offered three Silver augments at once (it
    /// starts at level 3), keeps one with a command, and its stats include it from the next
    /// tick. Without Mayhem rules nothing is offered and the commands do nothing.
    #[test]
    fn mayhem_drafts_augments_into_the_stat_stack() {
        let mut w = World::new(2);
        w.set_rules(Rules::MAYHEM);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Bastion, Vec2::new(1000.0, 1000.0));
        let before = w.unit(me).unwrap().stats;
        w.step(&[]);
        let offer = w.unit(me).unwrap().state.progress.offer;
        assert!(offer.iter().all(|id| augments::augment(*id).is_some_and(|a| a.tier == augments::Tier::Silver)));
        let pick = |seq, tick, kind| Command { player: PlayerId(0), seq, tick: Tick(tick), sub: SubTick::START, kind };
        w.step(&[pick(1, 2, CommandKind::PickAugment(2))]);
        w.step(&[]);
        let u = w.unit(me).unwrap();
        assert_eq!(u.state.progress.augments, [offer[2], 0, 0, 0]);
        assert_eq!(u.state.progress.offer, [0; augments::CHOICES]);
        let expected = items::champion_stats(
            ChampionId::Bastion.def(),
            &(3, [0; INVENTORY], [offer[2], 0, 0, 0], augments::Growth::NONE),
        )
        .0;
        assert_eq!(u.stats, expected);
        assert_ne!(u.stats, before, "every Silver augment changes some stat");

        let mut plain = ranked_world();
        let other = plain.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Bastion, Vec2::new(1000.0, 1000.0));
        plain.step(&[]);
        plain.step(&[pick(1, 2, CommandKind::PickAugment(0)), pick(2, 2, CommandKind::RerollAugment(0))]);
        let p = plain.unit(other).unwrap().state.progress;
        assert_eq!((p.offer, p.augments, p.drafted), ([0; 3], [0; 4], 0), "ARAM without Mayhem has no drafts");
    }

    /// A Mayhem champion holding `augments`, all abilities learned, and an enemy dummy.
    fn augmented(augments: [u8; augments::SLOTS], champion: ChampionId, enemy_at: Vec2) -> (World, UnitId, UnitId) {
        let mut w = World::new(5);
        w.set_rules(Rules::MAYHEM);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, champion, Vec2::new(1000.0, 1000.0));
        let them = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Bastion, enemy_at);
        for id in [me, them] {
            let p = &mut w.unit_mut(id).unwrap().state.progress;
            p.augments = augments;
            p.ranks = [1; 4];
            p.drafted = augments::SLOTS as u8; // no more drafts
            p.level = 11;
        }
        (w, me, them)
    }

    /// M3 slice 2: Multishot fires three projectiles 15° apart; at point-blank range all three
    /// cross the target, but a volley hits each enemy once.
    #[test]
    fn multishot_fires_a_spread_that_hits_each_enemy_once() {
        let (mut w, _, them) = augmented([24, 0, 0, 0], ChampionId::Ember, Vec2::new(1150.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (2000.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 60);
        let spawned: Vec<Missile> = ev
            .iter()
            .filter_map(|e| match e {
                SimEvent::MissileSpawned(m) => Some(*m),
                _ => None,
            })
            .collect();
        assert_eq!(spawned.len(), 3);
        assert_eq!(spawned.iter().map(|m| m.shot).collect::<Vec<_>>(), vec![0, 1, 2]);
        assert_eq!(spawned[1].dir, Vec2::new(augments::SPREAD_COS, augments::SPREAD_SIN));
        assert_eq!(spawned[2].dir, Vec2::new(augments::SPREAD_COS, -augments::SPREAD_SIN));
        let hits = ev.iter().filter(|e| matches!(e, SimEvent::MissileHit { target, .. } if *target == them)).count();
        assert_eq!(hits, 1, "one hit per volley and target");
        assert!(w.struck.is_empty(), "forgotten once the volley is gone");
    }

    /// Echo repeats a skillshot 0.75 s later from where the caster stands, at 40% power; a
    /// Broadside line is 50% wider. Bastion's pull accepts Broadside but not Multishot or Echo.
    #[test]
    fn echo_repeats_and_broadside_widens() {
        let (mut w, _, _) = augmented([25, 26, 0, 0], ChampionId::Ember, Vec2::new(4000.0, 4000.0));
        w.step(&[cast(0, 1, 1, 0, (2000.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 60);
        let spawned: Vec<Missile> = ev
            .iter()
            .filter_map(|e| match e {
                SimEvent::MissileSpawned(m) => Some(*m),
                _ => None,
            })
            .collect();
        assert_eq!(spawned.len(), 2);
        let (first, echo) = (spawned[0], spawned[1]);
        assert_eq!(echo.shot, augments::ECHO_SHOT);
        assert_eq!(echo.spawn_at.0 - first.spawn_at.0, 750 * SUBTICKS_PER_SECOND / 1000);
        assert!((echo.power - first.power * augments::ECHO_POWER).abs() < 1e-3);
        let Effect::Line(lance) = ChampionId::Ember.def().abilities[0].effect else { panic!() };
        assert_eq!(first.spec.radius, lance.radius * augments::WIDE_LINE);

        let (mut w, _, _) = augmented([24, 25, 26, 0], ChampionId::Bastion, Vec2::new(4000.0, 4000.0));
        let pull =
            ChampionId::Bastion.def().abilities.iter().position(|a| matches!(a.effect, Effect::Line(_))).unwrap();
        w.step(&[cast_slot(0, 1, 1, 0, pull as u8, (2000.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 60);
        let spawned: Vec<Missile> = ev
            .iter()
            .filter_map(|e| match e {
                SimEvent::MissileSpawned(m) => Some(*m),
                _ => None,
            })
            .collect();
        assert_eq!(spawned.len(), 1, "the pull fires once");
        let Effect::Line(hook) = ChampionId::Bastion.def().abilities[pull].effect else { panic!() };
        assert_eq!(spawned[0].spec.radius, hook.radius * augments::WIDE_LINE);
    }

    /// An Ember (red) firing its Q along y = 1000 + `offset`, past a blue Bastion at (2000,
    /// 1000) that holds `target_augments`; the Ember holds `shooter_augments`. Returns the
    /// damage the Bastion took (0 for a miss).
    fn lance_past(offset: f32, target_augments: [u8; 4], shooter_augments: [u8; 4]) -> f32 {
        lance_with(offset, target_augments, shooter_augments, |_, _, _| {})
    }

    /// `lance_past` with `setup(world, target, shooter)` run just before the cast.
    fn lance_with(
        offset: f32,
        target_augments: [u8; 4],
        shooter_augments: [u8; 4],
        setup: impl FnOnce(&mut World, UnitId, UnitId),
    ) -> f32 {
        let mut w = World::new(9);
        w.set_rules(Rules::MAYHEM);
        let target = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Bastion, Vec2::new(2000.0, 1000.0));
        let shooter = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1000.0, 1000.0 + offset));
        for (id, augments) in [(target, target_augments), (shooter, shooter_augments)] {
            let p = &mut w.unit_mut(id).unwrap().state.progress;
            p.augments = augments;
            p.ranks = [1; 4];
            p.drafted = augments::SLOTS as u8;
        }
        w.step(&[]);
        setup(&mut w, target, shooter);
        w.step(&[cast(1, 1, 2, 0, (3000.0, 1000.0 + offset))]);
        run_until_quiet(&mut w, 60)
            .iter()
            .map(|e| match e {
                SimEvent::Damage { target: t, amount, absorbed, .. } if *t == target => amount + absorbed,
                _ => 0.0,
            })
            .sum()
    }

    /// M3 slice 6: Hyper rules give Q, W and E 300 ability haste (a quarter of the cooldown)
    /// and attacks 50% more speed; the ultimate keeps its cooldown.
    #[test]
    fn hyper_rules_speed_up_basic_abilities_and_attacks() {
        let cooldowns = |rules: Rules| {
            let mut w = World::new(5);
            w.set_rules(rules);
            let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
            {
                let p = &mut w.unit_mut(me).unwrap().state.progress;
                (p.level, p.ranks) = (11, [1; 4]);
            }
            w.step(&[]);
            w.step(&[cast_slot(0, 1, 2, 0, 0, (2000.0, 1000.0))]);
            run_until_quiet(&mut w, 30);
            let r_tick = w.tick().0 + 1;
            w.step(&[cast_slot(0, 2, r_tick, 0, 3, (2000.0, 1000.0))]);
            let u = w.unit(me).unwrap();
            let (q_at, r_at) = (SimTime::end_of(Tick(1)).0, SimTime::end_of(Tick(r_tick - 1)).0);
            let (q, r) = (u.state.cooldowns[0].0 - q_at, u.state.cooldowns[3].0.saturating_sub(r_at));
            (q, r, u.attack.unwrap().attack_speed, u.state.progress.hyper)
        };
        let (aram, hyper) = (cooldowns(Rules::ARAM), cooldowns(Rules::HYPER));
        assert!(hyper.3 && !aram.3);
        assert!(hyper.0.abs_diff(aram.0 / 4) <= 1, "Q: {} → {} sub-ticks", aram.0, hyper.0);
        assert!(aram.1 > 0, "R was cast");
        assert_eq!(aram.1, hyper.1, "R keeps its cooldown");
        assert!((hyper.2 / aram.2 - (1.0 + HYPER_ATTACK_SPEED)).abs() < 1e-4, "{} → {}", aram.2, hyper.2);
    }

    /// M3 slice 4: conditional damage augments multiply onto the hit (server side).
    #[test]
    fn conditional_augments_amp_damage() {
        let normal = lance_past(0.0, [0; 4], [0; 4]);
        let ratio = |d: f32| d / normal;
        // First Strike: the Bastion is at full health.
        assert!((ratio(lance_past(0.0, [0; 4], [39, 0, 0, 0])) - augments::FIRST_STRIKE_AMP).abs() < 1e-4);
        // Executioner: below 35% health, and only then.
        let low = |w: &mut World, t: UnitId, _: UnitId| {
            let u = w.unit_mut(t).unwrap();
            u.state.health = u.stats.max_health * 0.3;
        };
        let (plain_low, exec_low) = (lance_with(0.0, [0; 4], [0; 4], low), lance_with(0.0, [0; 4], [38, 0, 0, 0], low));
        assert!((exec_low / plain_low - augments::EXECUTE_AMP).abs() < 1e-4);
        assert!((ratio(lance_past(0.0, [0; 4], [38, 0, 0, 0])) - 1.0).abs() < 1e-4);
        // Last Stand: at 30% health, halfway below 60%.
        let hurt = |w: &mut World, _: UnitId, s: UnitId| {
            let u = w.unit_mut(s).unwrap();
            u.state.health = u.stats.max_health * 0.3;
        };
        let stand = lance_with(0.0, [0; 4], [43, 0, 0, 0], hurt);
        assert!((ratio(stand) - (1.0 + augments::LAST_STAND_MAX * 0.5)).abs() < 1e-3, "{}", ratio(stand));
        // Fundamentals: Q, W and E hit harder.
        assert!((ratio(lance_past(0.0, [0; 4], [54, 0, 0, 0])) - augments::FUNDAMENTALS_AMP).abs() < 1e-4);
    }

    /// Spellcrit crits about one ability hit in four, and never a basic attack.
    #[test]
    fn spellcrit_crits_abilities_only() {
        let (w, me, them) = augmented([52, 0, 0, 0], ChampionId::Ember, Vec2::new(1400.0, 1000.0));
        let mut rng = Pcg32::new(1, 2);
        let amps: Vec<f32> = (0..400).map(|_| damage_amp(&w.units, me, them, true, &mut rng)).collect();
        assert!(amps.iter().all(|a| *a == 1.0 || *a == augments::SPELLCRIT_AMP));
        let crits = amps.iter().filter(|a| **a > 1.0).count();
        assert!((60..140).contains(&crits), "{crits}");
        assert!((0..100).all(|_| damage_amp(&w.units, me, them, false, &mut rng) == 1.0));
    }

    /// Fundamentals has no ultimate; an augment spell replaces F.
    #[test]
    fn fundamentals_refuses_r_and_spells_replace_f() {
        let (mut w, me, _) = augmented([54, 0, 0, 0], ChampionId::Ember, Vec2::new(4000.0, 4000.0));
        w.step(&[cast_slot(0, 1, 1, 0, 3, (1500.0, 1000.0))]);
        let u = w.unit(me).unwrap();
        assert!(u.state.cast.is_none() && u.state.cooldowns[3] == SimTime(0), "R is refused");

        let (mut w, me, _) = augmented([48, 0, 0, 0], ChampionId::Ember, Vec2::new(4000.0, 4000.0));
        assert_eq!(w.unit(me).unwrap().ability(5), Some(augments::VAULT));
        w.step(&[cast_slot(0, 1, 1, 0, 5, (1500.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 30);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::Dashed { unit, .. } if *unit == me)), "Vault dashes");
    }

    /// The power of the first bolt `champion` (holding `augments`) attacks with, after its Q.
    fn bolt_after_q(augments: [u8; 4]) -> f32 {
        let (mut w, _, them) = augmented(augments, ChampionId::Ember, Vec2::new(1400.0, 1000.0));
        w.step(&[cast(0, 1, 1, 0, (1000.0, 2000.0))]);
        run_until_quiet(&mut w, 20);
        let tick = w.tick().0 + 1;
        w.step(&[attack(0, 2, tick, them)]);
        run_until_quiet(&mut w, 40)
            .iter()
            .find_map(|e| match e {
                SimEvent::AttackLaunched(b) => Some(b.power),
                _ => None,
            })
            .expect("an attack")
    }

    /// Spellblade: the attack after an ability adds the base attack damage.
    #[test]
    fn spellblade_charges_the_next_attack() {
        let (plain, blade) = (bolt_after_q([0; 4]), bolt_after_q([42, 0, 0, 0]));
        let base = ChampionId::Ember.def().stats_at(11).attack_damage;
        assert!((blade - plain - base).abs() < 1e-3, "{plain} → {blade} (base {base})");
    }

    /// Close Quarters turns a ranged attack into a melee strike; Sharpshooter adds range.
    #[test]
    fn close_quarters_and_sharpshooter_change_the_attack() {
        let ranged = ChampionId::Ember.def().attack;
        let (mut w, me, _) = augmented([53, 0, 0, 0], ChampionId::Ember, Vec2::new(4000.0, 4000.0));
        w.step(&[]);
        let a = w.unit(me).unwrap().attack.unwrap();
        assert_eq!((a.range, a.bolt_speed), (augments::CLOSE_QUARTERS_RANGE, 0.0));
        let (mut w, me, _) = augmented([47, 0, 0, 0], ChampionId::Ember, Vec2::new(4000.0, 4000.0));
        w.step(&[]);
        assert_eq!(w.unit(me).unwrap().attack.unwrap().range, ranged.range + augments::SHARPSHOOTER_RANGE);

        let (mut w, me, them) = augmented([53, 0, 0, 0], ChampionId::Ember, Vec2::new(1150.0, 1000.0));
        w.step(&[]);
        w.step(&[attack(0, 1, 2, them)]);
        let ev = run_until_quiet(&mut w, 40);
        assert!(!ev.iter().any(|e| matches!(e, SimEvent::AttackLaunched(_))), "no bolt");
        assert!(
            ev.iter().any(|e| matches!(e, SimEvent::Damage { source, target, .. } if (*source, *target) == (me, them)))
        );
    }

    /// Spellhunger stacks ability power on champion hits; Spell Vamp heals from ability damage.
    #[test]
    fn spellhunger_and_spell_vamp_feed_on_ability_hits() {
        let (mut w, me, them) = augmented([41, 44, 0, 0], ChampionId::Ember, Vec2::new(1400.0, 1000.0));
        let ap = w.unit(me).unwrap().stats.ability_power;
        let half = w.unit(me).unwrap().stats.max_health * 0.5;
        w.unit_mut(me).unwrap().state.health = half;
        w.step(&[cast(0, 1, 1, 0, (2000.0, 1000.0))]);
        let ev = run_until_quiet(&mut w, 40);
        let dealt: f32 = ev
            .iter()
            .map(|e| match e {
                SimEvent::Damage { source, target, amount, absorbed, .. } if (*source, *target) == (me, them) => {
                    amount + absorbed
                }
                _ => 0.0,
            })
            .sum();
        let healed: f32 = ev
            .iter()
            .map(|e| match e {
                SimEvent::Healed { unit, amount, .. } if *unit == me => *amount,
                _ => 0.0,
            })
            .sum();
        assert!(dealt > 0.0);
        assert!((healed - augments::SPELL_VAMP * dealt).abs() < 1e-2, "{healed} of {dealt}");
        let u = w.unit(me).unwrap();
        assert_eq!(u.state.progress.stacks, 1);
        assert!((u.stats.ability_power - ap - 1.0).abs() < 1e-3, "{ap} → {}", u.stats.ability_power);
    }

    /// Thorns returns a share of a champion's attack damage as magic damage.
    #[test]
    fn thorns_reflect_attacks() {
        let (mut w, me, them) = augmented([0; 4], ChampionId::Ember, Vec2::new(1400.0, 1000.0));
        w.unit_mut(them).unwrap().state.progress.augments = [46, 0, 0, 0];
        w.step(&[attack(0, 1, 1, them)]);
        let ev = run_until_quiet(&mut w, 60);
        let hit = ev
            .iter()
            .find_map(|e| match e {
                SimEvent::Damage { source, target, amount, absorbed, .. } if (*source, *target) == (me, them) => {
                    Some(amount + absorbed)
                }
                _ => None,
            })
            .expect("the attack lands");
        let back = ev
            .iter()
            .find_map(|e| match e {
                SimEvent::Damage { source, target, kind: DamageKind::Magic, amount, .. }
                    if (*source, *target) == (them, me) =>
                {
                    Some(*amount)
                }
                _ => None,
            })
            .expect("thorns");
        let mr = w.unit(me).unwrap().stats.magic_resist;
        assert!((back - augments::THORNS * hit * resist_multiplier(mr)).abs() < 1e-2, "{back} of {hit}");
    }

    /// Takedowns count toward Champion of Chaos (its reward lands at eight) and Reset refreshes
    /// Q, W and E.
    #[test]
    fn takedowns_reset_cooldowns_and_complete_chaos() {
        let (mut w, me, them) = augmented([45, 55, 0, 0], ChampionId::Ember, Vec2::new(1400.0, 1000.0));
        w.step(&[]);
        let before = w.unit(me).unwrap().stats;
        {
            let u = w.unit_mut(me).unwrap();
            u.state.cooldowns = [SimTime(u64::MAX / 4); SLOTS];
            u.state.progress.takedowns = augments::CHAOS_TAKEDOWNS - 1;
        }
        w.unit_mut(them).unwrap().state.health = 1.0;
        w.step(&[attack(0, 1, 2, them)]);
        let ev = run_until_quiet(&mut w, 60);
        assert!(ev.iter().any(|e| matches!(e, SimEvent::Died { unit, .. } if *unit == them)));
        w.step(&[]);
        let u = w.unit(me).unwrap();
        assert_eq!(u.state.progress.takedowns, augments::CHAOS_TAKEDOWNS);
        assert!(u.state.cooldowns[..3].iter().all(|c| *c <= SimTime::end_of(w.tick())), "Q, W and E are ready");
        assert_eq!(u.state.cooldowns[3], SimTime(u64::MAX / 4), "R is not");
        assert!((u.stats.attack_damage - before.attack_damage - augments::CHAOS_REWARD.attack_damage).abs() < 1e-2);
        assert!((u.stats.max_health - before.max_health - augments::CHAOS_REWARD.health).abs() < 1.0);
    }

    /// M3 slice 3: Titan and Pebble change the hitbox the sim judges hits against, along with
    /// the stats; collision with walls and units keeps the champion size.
    #[test]
    fn titan_and_pebble_change_hitboxes_honestly() {
        let (mut w, titan, pebble) = augmented([27, 0, 0, 0], ChampionId::Bastion, Vec2::new(3000.0, 3000.0));
        w.unit_mut(pebble).unwrap().state.progress.augments = [28, 0, 0, 0];
        let plain =
            items::champion_stats(ChampionId::Bastion.def(), &(11, [0; INVENTORY], [0; 4], augments::Growth::NONE)).0;
        w.step(&[]);
        let (t, p) = (w.unit(titan).unwrap(), w.unit(pebble).unwrap());
        assert_eq!(t.gameplay_radius, CHAMPION_GAMEPLAY_RADIUS * augments::TITAN_SCALE);
        assert_eq!(p.gameplay_radius, CHAMPION_GAMEPLAY_RADIUS * augments::PEBBLE_SCALE);
        assert_eq!((t.collision_radius, p.collision_radius), (CHAMPION_COLLISION_RADIUS, CHAMPION_COLLISION_RADIUS));
        assert!((t.stats.max_health - plain.max_health * 1.3).abs() < 1e-2);
        assert!(p.stats.move_speed > plain.move_speed);

        // 110 u beside the center: past a normal hitbox (65 + 35), into a Titan's (97.5 + 35).
        assert_eq!(lance_past(110.0, [0; 4], [0; 4]), 0.0);
        assert!(lance_past(110.0, [27, 0, 0, 0], [0; 4]) > 0.0);
        // 80 u beside: into a normal hitbox, past a Pebble's (39 + 35).
        assert!(lance_past(80.0, [0; 4], [0; 4]) > 0.0);
        assert_eq!(lance_past(80.0, [28, 0, 0, 0], [0; 4]), 0.0);
        // A Pebble hits larger targets 20% harder.
        let (normal, pebble) = (lance_past(0.0, [0; 4], [0; 4]), lance_past(0.0, [0; 4], [28, 0, 0, 0]));
        assert!((pebble / normal - augments::PEBBLE_AMP).abs() < 1e-4, "{normal} → {pebble}");
    }

    /// Unstable Experiment rolls huge or tiny when picked and again at every respawn, from the
    /// champion's seed and the respawn instant (so prediction rolls the same).
    #[test]
    fn unstable_experiment_rerolls_at_each_respawn() {
        let (mut w, me, _) = augmented([0; 4], ChampionId::Rook, Vec2::new(3000.0, 3000.0));
        {
            let p = &mut w.unit_mut(me).unwrap().state.progress;
            p.drafted = 1;
            p.offer = [29, 0, 0];
        }
        w.step(&[Command {
            player: PlayerId(0),
            seq: 1,
            tick: Tick(1),
            sub: SubTick::START,
            kind: CommandKind::PickAugment(0),
        }]);
        w.step(&[]);
        let mut forms = std::collections::BTreeSet::new();
        for _ in 0..12 {
            let u = w.unit(me).unwrap();
            let expected = if u.state.progress.unstable_tiny { augments::PEBBLE_SCALE } else { augments::TITAN_SCALE };
            assert_eq!(u.gameplay_radius, CHAMPION_GAMEPLAY_RADIUS * expected);
            forms.insert(u.state.progress.unstable_tiny);
            // Die and come back at the start of the next tick.
            let at = SimTime::end_of(w.tick());
            let u = w.unit_mut(me).unwrap();
            u.state.respawn_at = Some(at);
            let seed = u.state.progress.augment_seed;
            w.step(&[]);
            assert_eq!(w.unit(me).unwrap().state.progress.unstable_tiny, augments::unstable_roll(seed, at.0));
        }
        assert_eq!(forms.len(), 2, "both forms come up");
    }

    /// M2 slice 2: abilities must be learned; ranks are gated by level (R at 6 / 11 / 16) and
    /// raise damage and cut cooldowns.
    #[test]
    fn abilities_are_learned_with_points_and_gated_by_level() {
        let mut w = ranked_world();
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        assert_eq!(w.unit(me).unwrap().state.progress.points, 3, "ARAM starts at level 3");
        w.step(&[cast(0, 1, 1, 0, (2000.0, 1000.0))]);
        assert!(w.take_events().is_empty(), "Q not learned yet");
        w.step(&[level_up(0, 2, 2, 3)]); // R at level 3: refused
        w.step(&[level_up(0, 3, 3, 0), level_up(0, 4, 3, 0)]); // Q to 2 (level 3 allows 2)
        w.step(&[level_up(0, 5, 4, 0)]); // Q to 3: refused at level 3
        let p = w.unit(me).unwrap().state.progress;
        assert_eq!((p.ranks, p.points), ([2, 0, 0, 0], 1));
        w.step(&[cast(0, 6, 5, 0, (2000.0, 1000.0))]);
        let ev = w.take_events();
        assert!(matches!(ev[0], SimEvent::CastStarted { slot: 0, .. }));
        let q = EMBER.abilities[0];
        assert_eq!(w.unit(me).unwrap().state.cooldowns[0], SimTime::at(Tick(5), SubTick::START).plus(q.cooldown_at(2)));
    }

    fn shop(player: u8, seq: u32, tick: u32, kind: CommandKind) -> Command {
        Command { player: PlayerId(player), seq, tick: Tick(tick), sub: SubTick::START, kind }
    }

    /// Stat Anvils (Mayhem): from level 9, 750 gold in the fountain buys three stat choices of
    /// a rolled tier; the kept one adds to the champion's stats for the match. Plain ARAM has
    /// none.
    #[test]
    fn stat_anvils_buy_stats_late_in_mayhem() {
        let fountain = bridge_point(Vec2::new(400.0, 1500.0));
        let mut aram = ranked_world();
        aram.set_map(MapId::Bridge.shared());
        let plain = aram.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, fountain);
        aram.unit_mut(plain).unwrap().state.progress.level = 12;
        aram.step(&[shop(0, 1, 1, CommandKind::BuyAnvil)]);
        assert_eq!(aram.unit(plain).unwrap().state.progress.anvils, 0, "no anvils in plain ARAM");

        let mut w = World::new(9);
        w.set_rules(Rules::MAYHEM);
        w.set_map(MapId::Bridge.shared());
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, fountain);
        w.unit_mut(me).unwrap().state.progress.drafted = augments::SLOTS as u8; // no drafts
        w.step(&[shop(0, 1, 1, CommandKind::BuyAnvil)]);
        assert_eq!(w.unit(me).unwrap().state.progress.anvils, 0, "not before level 9");
        w.unit_mut(me).unwrap().state.progress.level = 9;
        let gold = w.unit(me).unwrap().state.progress.gold;
        w.step(&[shop(0, 2, 2, CommandKind::BuyAnvil)]);
        let p = w.unit(me).unwrap().state.progress;
        assert_eq!(p.anvils, 1);
        assert!(p.gold < gold - crate::anvils::COST + 1.0);
        let (tier, stat) = crate::anvils::unpack(p.anvil_offer[1]).unwrap();
        let before = w.unit(me).unwrap().stats;
        w.step(&[shop(0, 3, 3, CommandKind::PickAnvil(1))]);
        w.step(&[]);
        let u = w.unit(me).unwrap();
        assert_eq!(u.state.progress.anvil_offer, [0; crate::anvils::CHOICES]);
        let units = crate::anvils::TIER_UNITS[tier as usize];
        let i = crate::anvils::STATS.iter().position(|s| *s == stat).unwrap();
        assert_eq!(u.state.progress.anvil[i], units);
        assert_ne!(u.stats, before, "the kept stat counts");
    }

    /// Consumables (keys 1–6): potions stack in one slot and heal 120 over 15 s each, used up;
    /// the flask heals 100 over 12 s, twice, keeps its slot and refills at the fountain. Undo
    /// gives a stack back whole.
    #[test]
    fn potions_stack_heal_over_time_and_the_flask_refills() {
        use crate::items::*;
        let mut w = ranked_world();
        w.set_map(MapId::Bridge.shared());
        let fountain = bridge_point(Vec2::new(400.0, 1500.0));
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, fountain);
        w.step(&[shop(0, 1, 1, CommandKind::Buy(HEALTH_POTION)), shop(0, 2, 1, CommandKind::Buy(HEALTH_POTION))]);
        w.step(&[shop(0, 3, 2, CommandKind::Buy(REFILLABLE_FLASK)), shop(0, 4, 2, CommandKind::Buy(REFILLABLE_FLASK))]);
        let p = w.unit(me).unwrap().state.progress;
        assert_eq!((p.items[0], p.charges[0]), (HEALTH_POTION, 2), "two potions, one slot");
        assert_eq!((p.items[1], p.charges[1]), (REFILLABLE_FLASK, 2), "one flask, full");
        assert_eq!(p.items[2], 0, "a second flask isn't sold");
        // Out in the lane and hurt: drink a potion. A twin who doesn't drink shows what the
        // potion added on top of health regeneration.
        let lane = bridge_point(Vec2::new(4000.0, 1500.0));
        let twin =
            w.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Vesper, bridge_point(Vec2::new(4000.0, 1700.0)));
        for (id, pos) in [(me, lane), (twin, bridge_point(Vec2::new(4000.0, 1700.0)))] {
            let u = w.unit_mut(id).unwrap();
            u.state.pos = pos;
            u.state.health = 200.0;
        }
        let k = w.tick().0 + 1;
        w.step(&[shop(0, 5, k, CommandKind::UseItem(0))]);
        for _ in 0..(crate::time::TICK_HZ * 16) {
            w.step(&[]);
        }
        let u = w.unit(me).unwrap();
        let healed = u.state.health - w.unit(twin).unwrap().state.health;
        assert!((healed - 120.0).abs() < 0.01, "120 over 15 s: {healed}");
        assert_eq!((u.state.progress.items[0], u.state.progress.charges[0]), (HEALTH_POTION, 1));
        // The flask twice, then empty, then refilled at the fountain.
        let k = w.tick().0 + 1;
        w.step(&[shop(0, 6, k, CommandKind::UseItem(1)), shop(0, 7, k, CommandKind::UseItem(1))]);
        let p = w.unit(me).unwrap().state.progress;
        assert_eq!((p.items[1], p.charges[1]), (REFILLABLE_FLASK, 0), "an empty flask keeps its slot");
        w.unit_mut(me).unwrap().state.pos = fountain;
        w.step(&[]);
        assert_eq!(w.unit(me).unwrap().state.progress.charges[1], 2);
        // The last potion: used up, the slot frees.
        let k = w.tick().0 + 1;
        w.step(&[shop(0, 8, k, CommandKind::UseItem(0))]);
        assert_eq!(w.unit(me).unwrap().state.progress.items[0], 0);
    }

    /// M2 slice 3: buying only in the own fountain or while dead; recipes consume components
    /// and cost the difference; selling refunds 70%; stats follow the next tick.
    #[test]
    fn shopping_follows_the_fountain_rule_and_recipes() {
        use crate::items::*;
        let mut w = ranked_world();
        w.set_map(MapId::Bridge.shared());
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, bridge_point(Vec2::new(400.0, 1500.0)));
        let ad = w.unit(me).unwrap().stats.attack_damage;
        w.step(&[shop(0, 1, 1, CommandKind::Buy(LONG_KNIFE))]);
        w.step(&[]);
        let u = w.unit(me).unwrap();
        assert_eq!(u.stats.attack_damage, ad + 10.0);
        assert!((u.state.progress.gold - (1400.0 + 2.0 * 4.0 * TICK_DT - 350.0)).abs() < 1e-3);
        let gold = u.state.progress.gold;
        w.step(&[shop(0, 2, 3, CommandKind::Buy(LEECH_FANG)), shop(0, 3, 3, CommandKind::Buy(GRAND_GRIMOIRE))]);
        let p = w.unit(me).unwrap().state.progress;
        assert_eq!(p.items, [LEECH_FANG, 0, 0, 0, 0, 0], "the knife went into the fang; no gold for the grimoire");
        assert!((p.gold - (gold + 4.0 * TICK_DT - 550.0)).abs() < 1e-3);
        // Out of the fountain: no shopping. Dead: shopping again.
        w.unit_mut(me).unwrap().state.pos = bridge_point(Vec2::new(3000.0, 1500.0));
        w.step(&[shop(0, 4, 4, CommandKind::Sell(0)), shop(0, 5, 4, CommandKind::Buy(BOOTS))]);
        assert_eq!(w.unit(me).unwrap().state.progress.items, [LEECH_FANG, 0, 0, 0, 0, 0]);
        w.unit_mut(me).unwrap().state.respawn_at = Some(SimTime::end_of(Tick(200)));
        let gold = w.unit(me).unwrap().state.progress.gold;
        w.step(&[shop(0, 6, 5, CommandKind::Sell(0)), shop(0, 7, 5, CommandKind::Buy(BOOTS))]);
        w.step(&[shop(0, 8, 6, CommandKind::Buy(SAGE_BOOTS)), shop(0, 9, 6, CommandKind::Buy(BOOTS))]);
        let p = w.unit(me).unwrap().state.progress;
        assert_eq!(p.items, [SAGE_BOOTS, 0, 0, 0, 0, 0], "boots upgrade; a second pair is refused");
        let expect = gold + 2.0 * 4.0 * TICK_DT + 900.0 * SELL_REFUND - 300.0 - 650.0;
        assert!((p.gold - expect).abs() < 1e-3, "{} vs {expect}", p.gold);
        // Undo walks the trades back (boots upgrade, boots, the sale), not the refused ones.
        assert_eq!(p.undo_len, 3);
        w.step(&[shop(0, 10, 7, CommandKind::Undo), shop(0, 11, 7, CommandKind::Undo)]);
        let q = w.unit(me).unwrap().state.progress;
        assert_eq!(q.items, [0; INVENTORY]);
        assert!((q.gold - (p.gold + 4.0 * TICK_DT + 950.0)).abs() < 1e-3);
        w.step(&[shop(0, 12, 8, CommandKind::Undo), shop(0, 13, 8, CommandKind::Undo)]);
        let q = w.unit(me).unwrap().state.progress;
        assert_eq!((q.items, q.undo_len), ([LEECH_FANG, 0, 0, 0, 0, 0], 0), "the sale undone; nothing more");
        // Trades are final once the shop closes.
        w.step(&[shop(0, 14, 9, CommandKind::Sell(0))]);
        assert_eq!(w.unit(me).unwrap().state.progress.undo_len, 1);
        w.unit_mut(me).unwrap().state.respawn_at = None;
        w.step(&[]);
        w.step(&[shop(0, 15, 11, CommandKind::Undo)]);
        let q = w.unit(me).unwrap().state.progress;
        assert_eq!((q.items, q.undo_len), ([0; INVENTORY], 0));
        // Not ranked: no shop at all.
        let mut sandbox = World::new(3);
        let me = sandbox.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(400.0, 1500.0));
        sandbox.unit_mut(me).unwrap().state.progress.gold = 5000.0;
        sandbox.step(&[shop(0, 1, 1, CommandKind::Buy(LONG_KNIFE))]);
        assert_eq!(sandbox.unit(me).unwrap().state.progress.items, [0; INVENTORY]);
    }

    /// Item stats and passives change combat numbers exactly (02 §5, §7, §8, §10): flat AD,
    /// attack speed into the attack period, life steal, on-hit magic, ability haste, Lifeline.
    #[test]
    fn items_change_combat_numbers_exactly() {
        use crate::items::*;
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let enemy = w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(1500.0, 1000.0));
        {
            let u = w.unit_mut(me).unwrap();
            u.state.progress.items = [LEECH_FANG, QUICK_DAGGER, ARC_TEMPEST, 0, 0, 0];
            u.state.health = 300.0;
        }
        w.step(&[]);
        let u = w.unit(me).unwrap();
        assert_eq!(u.stats.attack_damage, 66.0 + 15.0);
        assert_eq!(u.stats.ability_power, 50.0);
        let spec = u.attack.unwrap();
        assert!((spec.attack_speed - 0.8 * (1.0 + 0.12 + 0.40)).abs() < 1e-6);
        assert_eq!(spec.period(), SimDuration((1920.0f32 / spec.attack_speed).round() as u64));
        w.step(&[attack(0, 1, 2, enemy)]);
        let ev = run_until_quiet(&mut w, 120);
        let hits: Vec<(DamageKind, f32)> = ev
            .iter()
            .filter_map(|e| match e {
                SimEvent::Damage { target, kind, amount, .. } if *target == enemy => Some((*kind, *amount)),
                _ => None,
            })
            .collect();
        let physical = 81.0 * 100.0 / 122.0;
        let magic = (15.0 + 0.15 * 50.0) * 100.0 / 130.0;
        assert!(hits.len() >= 6, "{hits:?}");
        for pair in hits.chunks(2) {
            assert_eq!(pair[0].0, DamageKind::Physical);
            assert!((pair[0].1 - physical).abs() < 1e-3, "{pair:?}");
            assert_eq!(pair[1].0, DamageKind::Magic);
            assert!((pair[1].1 - magic).abs() < 1e-3, "{pair:?}");
        }
        let heals: Vec<f32> = ev
            .iter()
            .filter_map(|e| match e {
                SimEvent::Healed { unit, amount, .. } if *unit == me => Some(*amount),
                _ => None,
            })
            .collect();
        assert_eq!(heals.len(), hits.len() / 2);
        assert!(heals.iter().all(|h| (h - 0.07 * physical).abs() < 1e-3), "life steal on the attack: {heals:?}");

        // Haste: Ember's Q with a Focus Charm (10 haste) recharges in 100/110 of the time.
        let mut w = World::new(1);
        let ember = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1000.0));
        w.unit_mut(ember).unwrap().state.progress.items[0] = FOCUS_CHARM;
        w.step(&[]);
        w.step(&[cast(0, 1, 2, 0, (2000.0, 1000.0))]);
        let q = EMBER.abilities[0].cooldown_at(1);
        let expect = SimDuration((q.0 as f64 * 100.0 / 110.0).round() as u64);
        assert_eq!(w.unit(ember).unwrap().state.cooldowns[0], SimTime::at(Tick(2), SubTick::START).plus(expect));

        // Lifeline: dropping under 30% grants 250 shield once, then it is on cooldown.
        let mut w = World::new(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        w.spawn_champion(PlayerId(1), Team::Red, ChampionId::Vesper, Vec2::new(1500.0, 1000.0));
        w.unit_mut(me).unwrap().state.progress.items[0] = LIFELINE_TALISMAN;
        w.step(&[]);
        let max = w.unit(me).unwrap().stats.max_health;
        assert_eq!(max, 620.0 + 400.0);
        w.unit_mut(me).unwrap().state.health = 0.3 * max + 20.0;
        w.step(&[attack(1, 1, 2, me)]);
        let ev = run_until_quiet(&mut w, 150);
        let shields: Vec<&SimEvent> = ev.iter().filter(|e| matches!(e, SimEvent::Shielded { .. })).collect();
        assert_eq!(shields.len(), 1, "{shields:?}");
        assert!(matches!(shields[0], SimEvent::Shielded { unit, amount, .. } if *unit == me && *amount == 250.0));
    }

    #[test]
    fn last_hits_pay_gold_and_nearby_champions_share_experience() {
        let mut w = ranked_world();
        let a = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let b = w.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Ember, Vec2::new(1000.0, 1300.0));
        let far = w.spawn_champion(PlayerId(2), Team::Blue, ChampionId::Ember, Vec2::new(5000.0, 5000.0));
        let minion = w.spawn_minion(MinionKind::Caster, Team::Red, Vec2::new(1500.0, 1000.0), None);
        w.step(&[attack(0, 1, 1, minion)]);
        let ev = run_until_quiet(&mut w, 200);
        let reward = |id| {
            ev.iter()
                .filter_map(|e| match e {
                    SimEvent::Reward { unit, gold, xp, .. } if *unit == id => Some((*gold, *xp)),
                    _ => None,
                })
                .fold((0.0, 0), |(g, x), (dg, dx)| (g + dg, x + dx))
        };
        assert_eq!(reward(a), (14.0, lane::shared_xp(30, 2)), "last hit: 14 gold, half of 30 xp +15%");
        assert_eq!(reward(b), (0.0, lane::shared_xp(30, 2)));
        assert_eq!(reward(far), (0.0, 0), "too far to share");
        let gold = w.unit(a).unwrap().state.progress.gold;
        assert!((gold - (1400.0 + 14.0 + 4.0 * 201.0 / 30.0)).abs() < 0.05, "start + last hit + passive: {gold}");
    }

    #[test]
    fn experience_levels_champions_up_and_stats_grow() {
        let mut p = Rules::ARAM.progress();
        gain_xp(&mut p, xp_to_next(3) + 10);
        assert_eq!((p.level, p.xp, p.points), (4, 10, 4));
        let mut w = ranked_world();
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let before = w.unit(me).unwrap().clone();
        w.unit_mut(me).unwrap().state.progress.level = 10;
        w.step(&[]);
        let after = w.unit(me).unwrap();
        assert_eq!(after.stats, VESPER_STATS_AT_10());
        assert!(
            (after.state.health - before.state.health - (after.stats.max_health - before.stats.max_health)).abs() < 1.0
        );
    }

    #[allow(non_snake_case)]
    fn VESPER_STATS_AT_10() -> crate::champion::Stats {
        crate::champion::VESPER.stats_at(10)
    }

    #[test]
    fn champion_kills_pay_bounty_and_assists() {
        let mut w = ranked_world();
        let a = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1000.0));
        let b = w.spawn_champion(PlayerId(1), Team::Blue, ChampionId::Vesper, Vec2::new(1000.0, 1300.0));
        let victim = w.spawn_champion(PlayerId(2), Team::Red, ChampionId::Ember, Vec2::new(1500.0, 1150.0));
        w.unit_mut(victim).unwrap().state.health = 150.0;
        w.step(&[attack(0, 1, 1, victim), attack(1, 2, 1, victim)]);
        let ev = run_until_quiet(&mut w, 90);
        let died = ev.iter().find_map(|e| match e {
            SimEvent::Died { unit, killer, .. } if *unit == victim => Some(*killer),
            _ => None,
        });
        let killer = died.expect("victim dies");
        let assister = if killer == a { b } else { a };
        let gold_of = |id| {
            ev.iter()
                .filter_map(|e| match e {
                    SimEvent::Reward { unit, gold, .. } if *unit == id => Some(*gold),
                    _ => None,
                })
                .sum::<f32>()
        };
        assert_eq!(gold_of(killer), lane::KILL_GOLD);
        assert_eq!(gold_of(assister), lane::KILL_GOLD * 0.5);
        assert_eq!(w.unit(killer).unwrap().state.progress.streak, 1);
        assert_eq!(w.unit(victim).unwrap().state.progress.streak, -1);
        // The scoreboard: a kill, an assist, a death.
        let score = |id: UnitId| {
            let p = w.unit(id).unwrap().state.progress;
            (p.kills, p.deaths, p.assists)
        };
        assert_eq!(score(killer), (1, 0, 0));
        assert_eq!(score(assister), (0, 0, 1));
        assert_eq!(score(victim), (0, 1, 0));
        assert_eq!(lane::bounty(4), 450.0);
        assert_eq!(lane::bounty(-3), 220.0);
    }

    fn arena_world(seed: u64) -> World {
        let mut w = World::new(seed);
        w.set_map(MapId::Arena.shared());
        w
    }

    #[test]
    fn champion_paths_around_a_wall_and_never_enters_one() {
        let mut w = arena_world(1);
        let me = w.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(2200.0, 1200.0));
        w.step(&[cmd(0, 1, 1, 0, (2700.0, 1200.0))]); // straight line crosses the long wall
        let map = w.map().clone();
        for _ in 0..600 {
            w.step(&[]);
            let p = w.unit(me).unwrap().state.pos;
            assert!(map.walkable(p, CHAMPION_COLLISION_RADIUS - 0.05), "inside a wall at {p:?}");
        }
        let s = w.unit(me).unwrap().state;
        assert_eq!(s.order, Order::Idle);
        assert!(s.pos.distance(Vec2::new(2700.0, 1200.0)) < 1.0, "{s:?}");
    }

    /// Prediction with the shared map and exact proxies stays bit-exact through walls, paths,
    /// re-plans and unit collision.
    #[test]
    fn prediction_with_map_matches_the_server() {
        let mut full = arena_world(4);
        clump(&mut full, Vec2::new(1200.0, 1200.0), 1);
        let me = full.spawn_champion(PlayerId(0), Team::Blue, ChampionId::Ember, Vec2::new(2000.0, 2000.0));
        full.spawn_champion(PlayerId(1), Team::Red, ChampionId::Ember, Vec2::new(2200.0, 2100.0));
        let mut rng = Pcg32::new(8, 8);
        for k in 1..=1500u32 {
            let mut c = Vec::new();
            if rng.next_u32().is_multiple_of(11) {
                let t = (rng.range_f32(300.0, 3700.0), rng.range_f32(300.0, 3700.0));
                c.push(cmd(0, k, k, (rng.next_u32() % 64) as u8, t));
            }
            if rng.next_u32().is_multiple_of(13) {
                let t = (rng.range_f32(300.0, 3700.0), rng.range_f32(300.0, 3700.0));
                c.push(cmd(1, k, k, 0, t));
            }
            let own = full.unit(me).unwrap().clone();
            let proxies = full.units().iter().filter(|u| u.id != me).map(|u| Unit {
                state: UnitState::new(u.state.pos, 0.0),
                brain: None,
                ..u.clone()
            });
            let mut predicted = World::from_units(full.tick(), vec![own]);
            predicted.set_map(MapId::Arena.shared());
            predicted.replace_others(me, proxies);
            predicted.step(&c);
            full.step(&c);
            assert!(predicted.unit(me).unwrap().state.bits_eq(&full.unit(me).unwrap().state), "tick {k}");
        }
    }

    /// Determinism canary for pathfinding and wall collision (arena map).
    #[test]
    fn golden_state_hash_arena() {
        let mut w = arena_world(0xA4E7A);
        for p in 0..6u8 {
            let team = if p % 2 == 0 { Team::Blue } else { Team::Red };
            w.spawn_champion(PlayerId(p), team, ChampionId::Ember, Vec2::new(600.0 + p as f32 * 500.0, 3300.0));
        }
        clump(&mut w, Vec2::new(1200.0, 1200.0), 1);
        let mut rng = Pcg32::new(77, 7);
        let mut seq = 0;
        for k in 1..=6000u32 {
            let mut cmds = Vec::new();
            for p in 0..6u8 {
                if rng.next_u32().is_multiple_of(23) {
                    seq += 1;
                    let t = (rng.range_f32(0.0, 4000.0), rng.range_f32(0.0, 4000.0));
                    cmds.push(cmd(p, seq, k, (rng.next_u32() % SUBTICKS as u32) as u8, t));
                }
            }
            w.step(&cmds);
            w.take_events();
        }
        assert_eq!(w.state_hash(), GOLDEN_HASH_ARENA, "hash = {:#018x}", w.state_hash());
    }

    const GOLDEN_HASH_ARENA: u64 = 0xe845_b3e4_e35b_90c9;

    /// Determinism canary for the lane match loop: waves, minion and turret AI, relics and
    /// fountains on The Bridge, with four champions fighting through it.
    #[test]
    fn golden_state_hash_bridge() {
        let mut w = bridge_world();
        w.set_rules(Rules::ARAM);
        // 3v3 with all six champions: every effect shape is part of the canary.
        for p in 0..6u8 {
            let team = if p % 2 == 0 { Team::Blue } else { Team::Red };
            let home = w.map().layout.champion_spawn[team as usize];
            w.spawn_champion(
                PlayerId(p),
                team,
                ChampionId::ALL[p as usize],
                home + Vec2::new(0.0, 60.0 * p as f32 - 150.0),
            );
        }
        let mut rng = Pcg32::new(31, 3);
        let mut seq = 0;
        let mut died = 0;
        for k in 1..=9000u32 {
            let mut cmds = Vec::new();
            for p in 0..6u8 {
                if rng.next_u32().is_multiple_of(29) {
                    seq += 1;
                    let t = bridge_point(Vec2::new(rng.range_f32(3000.0, 9000.0), rng.range_f32(900.0, 2100.0)));
                    let t = (t.x, t.y);
                    let sub = (rng.next_u32() % SUBTICKS as u32) as u8;
                    cmds.push(match rng.next_u32() % 7 {
                        0 => cast_slot(p, seq, k, sub, (rng.next_u32() % 4) as u8, t),
                        4 => level_up(p, seq, k, (rng.next_u32() % 4) as u8),
                        // Shopping works while dead (and in the fountain): items, recipes and
                        // their stats and passives are part of the canary.
                        5 => shop(p, seq, k, CommandKind::Buy(1 + (rng.next_u32() % 26) as u8)),
                        6 => shop(p, seq, k, CommandKind::Undo),
                        1 => Command {
                            kind: CommandKind::AttackMove(QPoint::from_vec2(Vec2::new(t.0, t.1))),
                            ..cmd(p, seq, k, sub, t)
                        },
                        _ => cmd(p, seq, k, sub, t),
                    });
                }
            }
            w.step(&cmds);
            died += w.take_events().iter().filter(|e| matches!(e, SimEvent::Died { .. })).count();
        }
        assert!(died > 50, "waves and champions should be fighting: {died}");
        let levels: Vec<u8> =
            w.units().iter().filter(|u| u.kind == UnitKind::Champion).map(|u| u.state.progress.level).collect();
        assert!(levels.iter().all(|l| *l > 3), "experience should level everyone: {levels:?}");
        let owned: usize = w.units().iter().map(|u| u.state.progress.items.iter().filter(|i| **i != 0).count()).sum();
        assert!(owned >= 4, "champions should have bought items: {owned}");
        assert_eq!(w.state_hash(), GOLDEN_HASH_BRIDGE, "hash = {:#018x}", w.state_hash());
    }

    const GOLDEN_HASH_BRIDGE: u64 = 0xb3d5_88f8_9bd9_b6b1;

    /// Cross-platform determinism canary: a scripted match must hash to the same value on
    /// every OS and CPU. If this fails on one platform, the sim used non-deterministic math.
    #[test]
    fn golden_state_hash() {
        let mut w = World::new(0xC0FFEE);
        for p in 0..10u8 {
            let team = if p < 5 { Team::Blue } else { Team::Red };
            let x = 500.0 + p as f32 * 1300.0;
            w.spawn_champion(PlayerId(p), team, ChampionId::ALL[p as usize % 2], Vec2::new(x, 7000.0));
        }
        for i in 0..4 {
            clump(&mut w, Vec2::new(2000.0 + 3000.0 * i as f32, 5000.0), 2);
            let a = QPoint::from_vec2(Vec2::new(1000.0 + 3000.0 * i as f32, 9000.0));
            let b = QPoint::from_vec2(Vec2::new(3000.0 + 3000.0 * i as f32, 9000.0));
            for j in 0..6 {
                let start = a.to_vec2() + Vec2::new(0.0, 55.0 * j as f32);
                w.spawn_minion(MinionKind::Caster, Team::Blue, start, Some(Brain::Patrol { a, b, toward_b: true }));
            }
            w.spawn_rig_turret(Team::Red, Vec2::new(1500.0 + 3500.0 * i as f32, 7600.0), 1100);
        }
        // A duelling pair that only attacks each other: deaths and respawns in the hash.
        let duel_a = w.spawn_champion(PlayerId(10), Team::Blue, ChampionId::Vesper, Vec2::new(7000.0, 12_000.0));
        let duel_b = w.spawn_champion(PlayerId(11), Team::Red, ChampionId::Ember, Vec2::new(7400.0, 12_000.0));
        let mut rng = Pcg32::new(2024, 7);
        let mut seen = [0u32; 6];
        let mut seq = 0;
        for k in 1..=9000u32 {
            let mut cmds = Vec::new();
            if k % 150 == 1 {
                seq += 2;
                cmds.push(attack(10, seq - 1, k, duel_b));
                cmds.push(attack(11, seq, k, duel_a));
            }
            for p in 0..10u8 {
                if rng.next_u32().is_multiple_of(17) {
                    seq += 1;
                    let sub = (rng.next_u32() % SUBTICKS as u32) as u8;
                    let t = (rng.range_f32(0.0, 14_800.0), rng.range_f32(0.0, 14_800.0));
                    // Mostly moves, some casts of every slot and some attacks (missiles, areas,
                    // bolts, dashes, blinks, shields, damage, deaths and respawns in the hash).
                    let r = rng.next_u32() % 8;
                    cmds.push(match r {
                        0 | 1 => cast_slot(p, seq, k, sub, (rng.next_u32() % 6) as u8, t),
                        2 => attack(p, seq, k, UnitId(1 + rng.next_u32() % 70)),
                        _ => cmd(p, seq, k, sub, t),
                    });
                }
            }
            w.step(&cmds);
            for e in w.take_events() {
                let i = match e {
                    SimEvent::Damage { .. } => 0,
                    SimEvent::Died { .. } => 1,
                    SimEvent::AreaDetonated { .. } => 2,
                    SimEvent::AttackLanded { hit: true, .. } => 3,
                    SimEvent::Dashed { .. } | SimEvent::Blinked { .. } => 4,
                    SimEvent::Respawned { .. } => 5,
                    _ => continue,
                };
                seen[i] += 1;
            }
        }
        assert!(seen.iter().all(|n| *n > 0), "damage, deaths, areas, bolts, dashes, respawns: {seen:?}");
        assert_eq!(w.tick(), Tick(9000));
        assert_eq!(w.state_hash(), GOLDEN_HASH, "hash = {:#018x}", w.state_hash());
    }

    /// Recorded on x86_64-pc-windows-msvc when facing, follow-throughs and the input buffer
    /// joined the state (A2). CI checks Linux, macOS (aarch64) and Windows.
    const GOLDEN_HASH: u64 = 0xe606_4e94_327a_2611;
}
