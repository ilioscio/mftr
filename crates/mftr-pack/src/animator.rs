//! The runtime layer stack of 10 §6, driven by simulation state each frame:
//!
//! ```text
//! locomotion (idle / idle_ready / walk / run / run_fast by speed)  →  action (attack or cast;
//!   upper body or full)  →  additive (rooted struggle)  →  flourish (dash start / travel / land,
//!   idle fidgets)  →  flinch (minions, additive)  →  override (stunned, dead)
//! ```
//!
//! Actions are **retimed** to the sim (10 §4.3): a windup's progress maps to `[0, fire]` of
//! the clip and a follow-through's to `[fire, end]`, so the release lands on the sim's fire time
//! at any attack speed. Without a progress (remote attacks) the clip plays at its own rate.

use crate::pose::{Layer, Library, Pose, add, blend};

/// Moving slower than this (u/s) counts as standing.
const MOVING: f32 = 20.0;
/// Below this ground speed walk, at or above it run (u/s).
const RUN_FROM: f32 = 230.0;
/// The locomotion cycle stretches this far before looking wrong (10 §5.1 *(start)*).
const RATE_RANGE: (f32, f32) = (0.6, 1.6);
/// Dashes play the run cycle this fast.
const DASH_RATE: f32 = 1.8;
/// Blends (10 §6 *(start)*): idle ↔ run, an action cut short, leaving a CC pose.
const LOCO_BLEND: f32 = 0.08;
const ACTION_OUT: f32 = 0.05;
const CC_OUT: f32 = 0.1;
/// After an action, the combat-ready idle shows this long (10 §5.1).
const READY_FOR: f32 = 3.0;
/// Standing still this long plays an idle fidget (10 §5.1: 6–10 s *(start)*).
const FIDGET_AFTER: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionKind {
    /// Basic attacks started (the sim's counter): picks `attack_1` / `attack_2`.
    Attack { variant: u8 },
    /// Ability slot 0–5.
    Cast { slot: u8 },
    /// Keep playing whatever action is showing (a follow-through whose action the drive no
    /// longer knows, e.g. after a cast fired). Nothing if none is.
    Continue,
}

/// What happened during one update, for sounds (A4c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimEvent {
    /// The locomotion cycle crossed a `foot_l` / `foot_r` marker (not while dashing).
    Foot,
    /// An attack or cast started its windup.
    Start(ActionKind),
    /// An attack or cast passed its `fire` marker: the moment a melee blow lands, a nova sweeps,
    /// a heal pulses (A6: effects without a projectile hang off this).
    Fire(ActionKind),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Windup,
    FollowThrough,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Action {
    pub kind: ActionKind,
    pub phase: Phase,
    /// 0..1 through the phase, from sim time; `None` = unknown (play at the clip's rate).
    pub progress: Option<f32>,
}

/// What the simulation says about a unit this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Drive {
    pub dt: f32,
    /// Ground speed in u/s (as displayed).
    pub speed: f32,
    pub dashing: bool,
    /// Which ability slot the dash is (0 = q … 3 = r), when known: picks that slot's dash clips
    /// for kits with several (A10). `None` plays the first.
    pub dash_slot: Option<u8>,
    pub stunned: bool,
    pub rooted: bool,
    pub dead: bool,
    /// Took a hit this frame: units with a `flinch` clip (minions, 10 §5.4) play it.
    pub hit: bool,
    pub action: Option<Action>,
}

#[derive(Clone, Copy, Debug)]
struct Playing {
    clip: usize,
    kind: Option<ActionKind>,
    time: f32,
    weight: f32,
    /// Fading out: the drive no longer asks for it.
    leaving: bool,
    /// Passed its `fire` marker (its `Fire` event is out).
    fired: bool,
    /// An instant cast's `pulse`: plays to its end on its own.
    pulse: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Flourish {
    DashStart,
    DashTravel,
    DashLand,
    Fidget,
}

#[derive(Clone, Debug)]
pub struct Animator {
    idle: Option<usize>,
    idle_ready: Option<usize>,
    fidgets: Vec<usize>,
    walk: Option<usize>,
    run: Option<usize>,
    run_fast: Option<usize>,
    /// The kit's dash clips per slot: `(slot, <slot>_start, <slot>_travel, <slot>_land)`.
    dashes: Vec<(u8, Option<usize>, usize, Option<usize>)>,
    /// The dash playing (chosen when it starts, so its landing matches).
    dash: Option<(Option<usize>, usize, Option<usize>)>,
    stunned: Option<usize>,
    rooted: Option<usize>,
    death: Option<usize>,
    flinch: Option<usize>,
    /// Seconds into the flinch playing, if one is.
    flinch_t: Option<f32>,
    /// Minions (A5): an attack whose windup ended plays its follow-through to the end while the
    /// unit stands still, instead of being cut. Their attacks repeat with nothing else in between.
    pub finish_attacks: bool,
    upper: Vec<bool>,
    idle_time: f32,
    /// 0..1 through the locomotion cycle, shared by walk and run so switching keeps the step.
    loco_phase: f32,
    move_w: f32,
    rooted_w: f32,
    action: Option<Playing>,
    over: Option<Playing>,
    flourish: Option<(Flourish, Playing)>,
    was_dashing: bool,
    /// Seconds of the combat-ready idle left.
    ready_left: f32,
    /// Seconds standing still with nothing going on (fidgets).
    still: f32,
    next_fidget: usize,
    /// This update's events.
    pub events: Vec<AnimEvent>,
    /// Events raised between updates (a pulse), released by the next one.
    pending: Vec<AnimEvent>,
}

impl Animator {
    pub fn new(lib: &Library) -> Animator {
        let dashes: Vec<(u8, Option<usize>, usize, Option<usize>)> = ["q", "w", "e", "r"]
            .iter()
            .enumerate()
            .filter_map(|(i, s)| {
                let travel = lib.clip(&format!("{s}_travel"))?;
                Some((i as u8, lib.clip(&format!("{s}_start")), travel, lib.clip(&format!("{s}_land"))))
            })
            .collect();
        let dash = dashes.first().map(|&(_, s, t, l)| (s, t, l));
        Animator {
            idle: lib.clip("idle"),
            idle_ready: lib.clip("idle_ready"),
            fidgets: ["idle_fidget_1", "idle_fidget_2"].iter().filter_map(|n| lib.clip(n)).collect(),
            walk: lib.clip("walk"),
            run: lib.clip("run"),
            run_fast: lib.clip("run_fast"),
            dashes,
            dash,
            stunned: lib.clip("cc_stunned"),
            rooted: lib.clip("cc_rooted"),
            death: lib.clip("death"),
            flinch: lib.clip("flinch"),
            flinch_t: None,
            finish_attacks: false,
            upper: lib.rig.subtree("spine_01"),
            idle_time: 0.0,
            loco_phase: 0.0,
            move_w: 0.0,
            rooted_w: 0.0,
            action: None,
            over: None,
            flourish: None,
            was_dashing: false,
            ready_left: 0.0,
            still: 0.0,
            next_fidget: 0,
            events: Vec::new(),
            pending: Vec::new(),
        }
    }

    /// The clip an action plays: the champion's own, else the shared fallback.
    pub fn action_clip(lib: &Library, kind: ActionKind) -> Option<usize> {
        match kind {
            ActionKind::Attack { variant } => {
                let own = if variant % 2 == 1 { "attack_1" } else { "attack_2" };
                lib.clip(own).or(lib.clip("attack_1")).or(lib.clip("attack_melee_alt"))
            }
            ActionKind::Cast { slot } => {
                let own = ["q", "w", "e", "r"].get(slot as usize).and_then(|s| lib.clip(s));
                own.or(lib.clip("cast_utility"))
            }
            ActionKind::Continue => None,
        }
    }

    /// Advance by `d.dt` and return the pose to show.
    /// An instant cast (no windup, so the drive never shows it): play its clip from `fire` to the
    /// end, with its `Fire` event in the next update (A6).
    pub fn pulse(&mut self, lib: &Library, kind: ActionKind) {
        if let Some(clip) = Self::action_clip(lib, kind) {
            let fire = lib.clips[clip].fire().min(lib.clips[clip].end());
            self.action = Some(Playing {
                clip,
                kind: Some(kind),
                time: fire,
                weight: 1.0,
                leaving: false,
                fired: true,
                pulse: true,
            });
            self.pending.push(AnimEvent::Fire(kind));
        }
    }

    pub fn update(&mut self, lib: &Library, d: &Drive) -> Pose {
        let dt = d.dt.max(0.0);
        self.events.clear();
        self.events.append(&mut self.pending);
        self.idle_time += dt;
        let mut pose = self.locomotion(lib, d, dt);
        self.update_action(lib, d, dt);
        if let Some(a) = self.action {
            let clip = &lib.clips[a.clip];
            let top = lib.sample(a.clip, a.time);
            match clip.layer {
                Layer::Upper => blend(&mut pose, &top, a.weight, Some(&self.upper)),
                Layer::Additive => add(&mut pose, &top, &lib.rig.rest, a.weight),
                Layer::Full => blend(&mut pose, &top, a.weight, None),
            }
        }
        let rooted_target = if d.rooted && !d.dead { 1.0 } else { 0.0 };
        self.rooted_w = approach(self.rooted_w, rooted_target, dt / CC_OUT);
        if let Some(c) = self.rooted {
            add(&mut pose, &lib.sample(c, self.idle_time), &lib.rig.rest, self.rooted_w);
        }
        self.update_flourish(lib, d, dt);
        if let Some((_, f)) = self.flourish {
            blend(&mut pose, &lib.sample(f.clip, f.time), f.weight, None);
        }
        if let Some(c) = self.flinch {
            if d.hit && !d.dead {
                self.flinch_t = Some(0.0);
            }
            if let Some(t) = self.flinch_t {
                add(&mut pose, &lib.sample(c, t), &lib.rig.rest, 1.0);
                let t = t + dt;
                self.flinch_t = (t < lib.clips[c].length).then_some(t);
            }
        }
        self.update_override(d, dt);
        if let Some(o) = self.over {
            blend(&mut pose, &lib.sample(o.clip, o.time), o.weight, None);
        }
        pose
    }

    fn locomotion(&mut self, lib: &Library, d: &Drive, dt: f32) -> Pose {
        let moving = d.dashing || d.speed >= MOVING;
        self.move_w = approach(self.move_w, if moving { 1.0 } else { 0.0 }, dt / LOCO_BLEND);
        // run_fast above the midpoint between the two cycles' stride speeds (haste, 10 §5.1).
        let stride = |c: Option<usize>| c.and_then(|c| lib.clips[c].stride_speed);
        let fast = match (stride(self.run), stride(self.run_fast)) {
            (Some(a), Some(b)) if d.speed >= (a + b) / 2.0 && !d.dashing => self.run_fast,
            _ => None,
        };
        let cycle = if fast.is_some() {
            fast
        } else if d.dashing || d.speed >= RUN_FROM {
            self.run.or(self.walk)
        } else {
            self.walk.or(self.run)
        };
        if let Some(c) = cycle {
            let clip = &lib.clips[c];
            let rate = if d.dashing {
                DASH_RATE
            } else {
                (d.speed / clip.stride_speed.unwrap_or(d.speed.max(1.0))).clamp(RATE_RANGE.0, RATE_RANGE.1)
            };
            if moving {
                let before = self.loco_phase;
                self.loco_phase = (self.loco_phase + dt * rate / clip.length.max(1e-3)).fract();
                let after = self.loco_phase;
                let crossed =
                    |m: f32| if after >= before { before < m && m <= after } else { m > before || m <= after };
                let feet = ["foot_l", "foot_r"].iter().filter_map(|n| clip.marker(n));
                if !d.dashing && self.move_w > 0.5 && feet.map(|t| t / clip.length.max(1e-3)).any(crossed) {
                    self.events.push(AnimEvent::Foot);
                }
            }
        }
        if d.action.is_some() {
            self.ready_left = READY_FOR;
        } else {
            self.ready_left = (self.ready_left - dt).max(0.0);
        }
        let idle = if self.ready_left > 0.0 { self.idle_ready.or(self.idle) } else { self.idle };
        let mut pose = match idle {
            Some(i) => lib.sample(i, self.idle_time),
            None => lib.rig.rest.clone(),
        };
        if let Some(c) = cycle {
            let walking = lib.sample(c, self.loco_phase * lib.clips[c].length);
            blend(&mut pose, &walking, self.move_w, None);
        }
        pose
    }

    fn update_action(&mut self, lib: &Library, d: &Drive, dt: f32) {
        let mut want = if d.dead || d.stunned { None } else { d.action };
        if let Some(a) = want
            && a.kind == ActionKind::Continue
        {
            want = match self.action {
                Some(p) if !p.leaving => p.kind.map(|kind| Action { kind, ..a }),
                _ => None,
            };
        }
        // A pulse plays to its end unless a new action, a stun or death takes over.
        if let Some(p) = &mut self.action
            && p.pulse
            && !p.leaving
            && !d.dead
            && !d.stunned
            && want.is_none_or(|a| Some(a.kind) == p.kind)
        {
            let end = lib.clips[p.clip].end();
            p.time = (p.time + dt).min(end);
            p.leaving = p.time >= end;
            return;
        }
        match want {
            Some(a) => {
                let Some(clip) = Self::action_clip(lib, a.kind) else {
                    self.action = None;
                    return;
                };
                let c = &lib.clips[clip];
                let fresh = self.action.is_none_or(|p| {
                    p.leaving || p.kind != Some(a.kind) || (a.phase == Phase::Windup && p.time > c.fire())
                });
                let start = if a.phase == Phase::Windup { 0.0 } else { c.fire() };
                if fresh && a.phase == Phase::Windup {
                    self.events.push(AnimEvent::Start(a.kind));
                }
                let mut p = if fresh {
                    Playing {
                        clip,
                        kind: Some(a.kind),
                        time: start,
                        weight: 1.0,
                        leaving: false,
                        fired: false,
                        pulse: false,
                    }
                } else {
                    self.action.unwrap_or(Playing {
                        clip,
                        kind: Some(a.kind),
                        time: start,
                        weight: 1.0,
                        leaving: false,
                        fired: false,
                        pulse: false,
                    })
                };
                let before = if fresh { -1.0 } else { p.time };
                p.time = match (a.phase, a.progress) {
                    (Phase::Windup, Some(x)) => x.clamp(0.0, 1.0) * c.fire(),
                    (Phase::FollowThrough, Some(x)) => c.fire() + x.clamp(0.0, 1.0) * (c.end() - c.fire()),
                    (Phase::Windup, None) => (p.time + dt).min(c.fire()),
                    (Phase::FollowThrough, None) => (p.time.max(c.fire()) + dt).min(c.end()),
                };
                if !p.fired && before < c.fire() && p.time >= c.fire() {
                    p.fired = true;
                    self.events.push(AnimEvent::Fire(a.kind));
                }
                p.weight = 1.0;
                self.action = Some(p);
            }
            None => {
                if let Some(p) = &mut self.action {
                    let end = lib.clips[p.clip].end();
                    let follow = self.finish_attacks
                        && !p.leaving
                        && matches!(p.kind, Some(ActionKind::Attack { .. }))
                        && d.speed < MOVING
                        && !d.dead
                        && !d.stunned;
                    if follow && p.time < end {
                        p.time = (p.time.max(lib.clips[p.clip].fire()) + dt).min(end);
                        return;
                    }
                    // Ended at the very end of its windup: the action fired, though no
                    // follow-through showed it (a caster walking on skips it, 10 §4.1).
                    if !p.leaving
                        && !p.fired
                        && p.time >= 0.85 * lib.clips[p.clip].fire()
                        && let Some(kind) = p.kind
                    {
                        p.fired = true;
                        self.events.push(AnimEvent::Fire(kind));
                    }
                    // Cut short (a move, a stun) or finished: blend back out quickly.
                    p.leaving = true;
                    p.time = (p.time + dt).min(lib.clips[p.clip].end());
                    p.weight -= dt / ACTION_OUT;
                    if p.weight <= 0.0 {
                        self.action = None;
                    }
                }
            }
        }
    }

    /// Dashes play the kit's start → travel (looping) → land clips; standing still long enough
    /// plays a fidget. Anything else happening cuts a fidget short.
    fn update_flourish(&mut self, lib: &Library, d: &Drive, dt: f32) {
        let play = |clip: usize| Playing {
            clip,
            kind: None,
            time: 0.0,
            weight: 1.0,
            leaving: false,
            fired: false,
            pulse: false,
        };
        let blocked = d.dead || d.stunned;
        if d.dashing && !self.was_dashing {
            // A new dash: that slot's clips (or the kit's first set).
            let pick = self.dashes.iter().find(|(s, ..)| Some(*s) == d.dash_slot).or(self.dashes.first());
            self.dash = pick.map(|&(_, s, t, l)| (s, t, l));
        }
        if let Some((start, travel, land)) = self.dash.filter(|_| !blocked) {
            if d.dashing && !self.was_dashing {
                self.flourish = Some(match start {
                    Some(s) => (Flourish::DashStart, play(s)),
                    None => (Flourish::DashTravel, play(travel)),
                });
            } else if !d.dashing && self.was_dashing {
                self.flourish = land.map(|l| (Flourish::DashLand, play(l)));
            }
            if d.dashing
                && let Some((Flourish::DashStart, p)) = self.flourish
                && p.time >= lib.clips[p.clip].length
            {
                self.flourish = Some((Flourish::DashTravel, play(travel)));
            }
        }
        self.was_dashing = d.dashing;

        let busy = blocked || d.dashing || d.action.is_some() || d.speed >= MOVING || self.action.is_some();
        if busy {
            self.still = 0.0;
        } else if self.flourish.is_none() {
            self.still += dt;
            if self.still >= FIDGET_AFTER && !self.fidgets.is_empty() {
                let clip = self.fidgets[self.next_fidget % self.fidgets.len()];
                self.next_fidget += 1;
                self.still = 0.0;
                self.flourish = Some((Flourish::Fidget, play(clip)));
            }
        }

        if let Some((kind, p)) = &mut self.flourish {
            let len = lib.clips[p.clip].length;
            p.time += dt;
            let interrupted = blocked
                || (*kind == Flourish::Fidget && busy)
                || (*kind != Flourish::Fidget && !d.dashing && *kind != Flourish::DashLand);
            let finished = *kind != Flourish::DashTravel && p.time >= len;
            if interrupted || finished || p.leaving {
                p.leaving = true;
                p.weight -= dt / ACTION_OUT;
                if p.weight <= 0.0 {
                    self.flourish = None;
                }
            }
        }
    }

    /// The flourish clip showing (dash parts, fidgets), if any.
    pub fn flourish_clip(&self) -> Option<usize> {
        self.flourish.map(|(_, p)| p.clip)
    }

    fn update_override(&mut self, d: &Drive, dt: f32) {
        let want = if d.dead {
            self.death
        } else if d.stunned {
            self.stunned
        } else {
            None
        };
        match (want, &mut self.over) {
            (Some(c), Some(p)) if p.clip == c && !p.leaving => p.time += dt,
            (Some(c), _) => {
                self.over = Some(Playing {
                    clip: c,
                    kind: None,
                    time: 0.0,
                    weight: 1.0,
                    leaving: false,
                    fired: false,
                    pulse: false,
                })
            }
            (None, Some(p)) => {
                p.leaving = true;
                p.time += dt;
                p.weight -= dt / CC_OUT;
                if p.weight <= 0.0 {
                    self.over = None;
                }
            }
            (None, None) => {}
        }
    }

    /// The clip and time of the action showing, if any (for tests and debugging).
    pub fn action_time(&self) -> Option<(usize, f32)> {
        self.action.map(|a| (a.clip, a.time))
    }

    /// Blend weight of the moving cycle (0 standing, 1 moving).
    pub fn moving_weight(&self) -> f32 {
        self.move_w
    }
}

fn approach(x: f32, target: f32, step: f32) -> f32 {
    if x < target { (x + step).min(target) } else { (x - step).max(target) }
}
