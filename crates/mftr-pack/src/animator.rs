//! The runtime layer stack of 10 §6, driven by simulation state each frame:
//!
//! ```text
//! locomotion (idle / walk / run by speed)  →  action (attack or cast; upper body or full)
//!   →  additive (rooted struggle)  →  override (stunned, dead)
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
    pub stunned: bool,
    pub rooted: bool,
    pub dead: bool,
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
}

#[derive(Clone, Debug)]
pub struct Animator {
    idle: Option<usize>,
    walk: Option<usize>,
    run: Option<usize>,
    stunned: Option<usize>,
    rooted: Option<usize>,
    death: Option<usize>,
    upper: Vec<bool>,
    idle_time: f32,
    /// 0..1 through the locomotion cycle, shared by walk and run so switching keeps the step.
    loco_phase: f32,
    move_w: f32,
    rooted_w: f32,
    action: Option<Playing>,
    over: Option<Playing>,
}

impl Animator {
    pub fn new(lib: &Library) -> Animator {
        Animator {
            idle: lib.clip("idle"),
            walk: lib.clip("walk"),
            run: lib.clip("run"),
            stunned: lib.clip("cc_stunned"),
            rooted: lib.clip("cc_rooted"),
            death: lib.clip("death"),
            upper: lib.rig.subtree("spine_01"),
            idle_time: 0.0,
            loco_phase: 0.0,
            move_w: 0.0,
            rooted_w: 0.0,
            action: None,
            over: None,
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
    pub fn update(&mut self, lib: &Library, d: &Drive) -> Pose {
        let dt = d.dt.max(0.0);
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
        self.update_override(d, dt);
        if let Some(o) = self.over {
            blend(&mut pose, &lib.sample(o.clip, o.time), o.weight, None);
        }
        pose
    }

    fn locomotion(&mut self, lib: &Library, d: &Drive, dt: f32) -> Pose {
        let moving = d.dashing || d.speed >= MOVING;
        self.move_w = approach(self.move_w, if moving { 1.0 } else { 0.0 }, dt / LOCO_BLEND);
        let cycle = if d.dashing || d.speed >= RUN_FROM { self.run.or(self.walk) } else { self.walk.or(self.run) };
        if let Some(c) = cycle {
            let clip = &lib.clips[c];
            let rate = if d.dashing {
                DASH_RATE
            } else {
                (d.speed / clip.stride_speed.unwrap_or(d.speed.max(1.0))).clamp(RATE_RANGE.0, RATE_RANGE.1)
            };
            if moving {
                self.loco_phase = (self.loco_phase + dt * rate / clip.length.max(1e-3)).fract();
            }
        }
        let mut pose = match self.idle {
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
                let mut p = if fresh {
                    Playing { clip, kind: Some(a.kind), time: start, weight: 1.0, leaving: false }
                } else {
                    self.action.unwrap_or(Playing {
                        clip,
                        kind: Some(a.kind),
                        time: start,
                        weight: 1.0,
                        leaving: false,
                    })
                };
                p.time = match (a.phase, a.progress) {
                    (Phase::Windup, Some(x)) => x.clamp(0.0, 1.0) * c.fire(),
                    (Phase::FollowThrough, Some(x)) => c.fire() + x.clamp(0.0, 1.0) * (c.end() - c.fire()),
                    (Phase::Windup, None) => (p.time + dt).min(c.fire()),
                    (Phase::FollowThrough, None) => (p.time.max(c.fire()) + dt).min(c.end()),
                };
                p.weight = 1.0;
                self.action = Some(p);
            }
            None => {
                if let Some(p) = &mut self.action {
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
            (Some(c), _) => self.over = Some(Playing { clip: c, kind: None, time: 0.0, weight: 1.0, leaving: false }),
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
