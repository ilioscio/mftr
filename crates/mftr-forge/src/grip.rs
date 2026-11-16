// SPDX-License-Identifier: AGPL-3.0-or-later
//! Hands that hold (22 §4.7, BW7): the grip pass over a forged champion's clips.
//!
//! The clips come from the official champions, whose weapons were laid along the hand that
//! holds them, mitten-fisted: a haft ran out of the wrist like a sixth finger. A creator base
//! has finger joints (`tools/blender/mftr_blender/hands.py`), and a held item says how it is
//! gripped (`Grasp`, from its `item.ron`). Here every clip is re-posed so the hand holds it as a
//! hand does:
//!
//! - **The weapon keeps its line.** Where the clip put the haft (a point and a direction, each
//!   frame) is where it stays: swings, thrusts and the bow's draw aim as authored.
//! - **The hand closes on it.** The haft lies across the palm in the crook of the fingers, which
//!   wrap it joint by joint (each phalanx tangent to it); the thumb comes round the other way.
//!   The knuckles lead an edge.
//! - **The arm comes to the hand.** Two freedoms are left: where round the haft the palm sits,
//!   and how far the haft tilts across the palm (a hammer's grip to a sabre's). Each frame takes
//!   the pair that bends the wrist least, the elbow re-solved to reach, and the choice is
//!   smoothed through the clip so the hand never jumps.
//! - **A second hand** whose palm the clip brings to the haft closes on it too, where it is.
//!
//! Free hands relax: the clips' mitten curl becomes a curl of every joint.

use mftr_pack::glb::{Animation, Channel, Model, qmul, rotate};
use mftr_pack::pose::slerp;

type V = [f32; 3];
type Q = [f32; 4];

/// How a held item is gripped, as `<base>.parts.ron` lists it (from the item's `item.ron`).
#[derive(Clone, Debug, nanoserde::DeRon)]
pub struct Grasp {
    /// "l" or "r": the hand that holds it.
    pub hand: String,
    /// The haft's radius where it is held (m).
    pub radius: f32,
    /// Where along the item (its +Z, from its origin) the fist's middle sits (m).
    pub at: f32,
    /// The item's +Z and +Y at rest, in the hand bone's frame (`grip_up`, `grip_back`).
    pub up: (f32, f32, f32),
    pub back: (f32, f32, f32),
    /// How far along the item a hand may hold (m, from its origin): a second hand grips there.
    pub span: (f32, f32),
    /// It has an edge to line up (unused by the solver: the knuckles lead every haft).
    pub edge: bool,
    /// The clips carry it on the hand bone itself, not the prop bone (a bow whose own bones
    /// hang on the hand).
    #[nserde(default)]
    pub on_hand: bool,
    /// Carried upright whatever the hand does (an orb on its stem, a lantern on its grip).
    #[nserde(default)]
    pub upright: bool,
    /// Strapped to the forearm (a shield): the fist closes on its grip bar, the wrist held
    /// straight; the arm is the clip's.
    #[nserde(default)]
    pub strap: bool,
}

/// A clip's layer, as the pass needs it.
#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    /// Plays whole (or over the lower body): re-posed.
    Pose { looping: bool },
    /// Adds its difference from rest: left alone.
    Additive,
}

const ID: Q = [0.0, 0.0, 0.0, 1.0];

fn add(a: V, b: V) -> V {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V, b: V) -> V {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V, s: f32) -> V {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn dot(a: V, b: V) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V, b: V) -> V {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn len(a: V) -> f32 {
    dot(a, a).sqrt()
}
fn norm(a: V) -> V {
    let l = len(a);
    if l > 1e-9 { mul(a, 1.0 / l) } else { [0.0, 1.0, 0.0] }
}
/// `a` without its part along the unit `n`.
fn perp(a: V, n: V) -> V {
    sub(a, mul(n, dot(a, n)))
}
fn qinv(q: Q) -> Q {
    [-q[0], -q[1], -q[2], q[3]]
}
fn qnorm(q: Q) -> Q {
    let l = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt().max(1e-12);
    [q[0] / l, q[1] / l, q[2] / l, q[3] / l]
}
fn axis_angle(axis: V, angle: f32) -> Q {
    let (s, c) = (angle * 0.5).sin_cos();
    let a = norm(axis);
    [a[0] * s, a[1] * s, a[2] * s, c]
}
/// The shortest turn taking unit `a` to unit `b`.
fn between(a: V, b: V) -> Q {
    let d = dot(a, b);
    if d < -0.9999 {
        let side = if a[0].abs() < 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
        return axis_angle(cross(a, side), std::f32::consts::PI);
    }
    let c = cross(a, b);
    qnorm([c[0], c[1], c[2], 1.0 + d])
}
/// The rotation whose columns are the right-handed orthonormal `x`, `y`, `z`.
fn from_basis(x: V, y: V, z: V) -> Q {
    let (m00, m01, m02) = (x[0], y[0], z[0]);
    let (m10, m11, m12) = (x[1], y[1], z[1]);
    let (m20, m21, m22) = (x[2], y[2], z[2]);
    let tr = m00 + m11 + m22;
    let q = if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        [(m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, 0.25 * s]
    } else if m00 > m11 && m00 > m22 {
        let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
        [0.25 * s, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s]
    } else if m11 > m22 {
        let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
        [(m01 + m10) / s, 0.25 * s, (m12 + m21) / s, (m02 - m20) / s]
    } else {
        let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
        [(m02 + m20) / s, (m12 + m21) / s, 0.25 * s, (m10 - m01) / s]
    };
    qnorm(q)
}
fn smooth(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// A clip's channels by node, sampled.
struct Tracks<'a> {
    rot: Vec<Option<&'a Channel>>,
    pos: Vec<Option<&'a Channel>>,
}

impl<'a> Tracks<'a> {
    fn new(model: &Model, a: &'a Animation) -> Tracks<'a> {
        let mut t = Tracks { rot: vec![None; model.nodes.len()], pos: vec![None; model.nodes.len()] };
        for c in &a.channels {
            match c.path.as_str() {
                "rotation" => t.rot[c.node] = Some(c),
                "translation" => t.pos[c.node] = Some(c),
                _ => {}
            }
        }
        t
    }

    fn key(c: &Channel, t: f32) -> (usize, usize, f32) {
        let i = c.times.partition_point(|&x| x <= t);
        if i == 0 {
            return (0, 0, 0.0);
        }
        if i >= c.times.len() {
            let l = c.times.len() - 1;
            return (l, l, 0.0);
        }
        if c.step {
            return (i - 1, i - 1, 0.0);
        }
        (i - 1, i, (t - c.times[i - 1]) / (c.times[i] - c.times[i - 1]).max(1e-6))
    }

    fn local(&self, model: &Model, node: usize, t: f32) -> (V, Q) {
        let n = &model.nodes[node];
        let r = match self.rot[node] {
            Some(c) if !c.values.is_empty() => {
                let (a, b, u) = Self::key(c, t);
                let q = |i: usize| [c.values[i][0], c.values[i][1], c.values[i][2], c.values[i][3]];
                if a == b { q(a) } else { slerp(q(a), q(b), u) }
            }
            _ => n.rotation,
        };
        let p = match self.pos[node] {
            Some(c) if !c.values.is_empty() => {
                let (a, b, u) = Self::key(c, t);
                let v = |i: usize| [c.values[i][0], c.values[i][1], c.values[i][2]];
                add(mul(v(a), 1.0 - u), mul(v(b), u))
            }
            _ => n.translation,
        };
        (p, r)
    }

    fn world(&self, model: &Model, node: usize, t: f32) -> (V, Q) {
        let (p, r) = self.local(model, node, t);
        match model.nodes[node].parent {
            None => (p, r),
            Some(parent) => {
                let (pp, pr) = self.world(model, parent, t);
                (add(pp, rotate(pr, p)), qmul(pr, r))
            }
        }
    }
}

fn rest_world(model: &Model, node: usize) -> (V, Q) {
    let n = &model.nodes[node];
    match n.parent {
        None => (n.translation, n.rotation),
        Some(p) => {
            let (pp, pr) = rest_world(model, p);
            (add(pp, rotate(pr, n.translation)), qmul(pr, n.rotation))
        }
    }
}

/// A hand with finger joints, read off the rest pose. Points and directions are in the hand
/// bone's frame.
struct Hand {
    upper: usize,
    fore: usize,
    hand: usize,
    fingers: [usize; 3],
    thumb: [usize; 2],
    prop: usize,
    /// The knuckles' row: its middle, the hinge line, down the fingers, the back of the hand.
    mcp: V,
    fx: V,
    fy: V,
    fz: V,
    /// The first two phalanges' lengths; half a finger's thickness.
    l1: f32,
    l2: f32,
    thick: f32,
    /// +1 if the thumb is toward +`fx` of the knuckles' middle.
    thumb_side: f32,
    len_upper: f32,
    len_fore: f32,
    /// The elbow's hinge (the way it bends at rest, across the arm) in the upper arm's frame
    /// and the forearm's: an elbow only folds one way, so the arm's bones are turned by it.
    hinge_upper: V,
    hinge_fore: V,
}

/// The finger and thumb joints' share of a finger's first phalanx: half its thickness, the
/// thumb's last bone over its first.
const THICK: f32 = 0.2;
const THUMB_TIP: f32 = 0.43;

impl Hand {
    fn read(model: &Model, side: &str) -> Option<Hand> {
        let n = |name: &str| model.node_by_name(&format!("{name}_{side}"));
        let hand = n("hand")?;
        let fore = model.nodes[hand].parent?;
        let upper = model.nodes[fore].parent?;
        let fingers = [n("fingers")?, n("fingers_02")?, n("fingers_03")?];
        let thumb = [n("thumb")?, n("thumb_02")?];
        let prop = n("prop")?;
        if model.nodes[fingers[0]].parent != Some(hand) || model.nodes[thumb[0]].parent != Some(hand) {
            return None;
        }
        let f = &model.nodes[fingers[0]];
        let l1 = len(model.nodes[fingers[1]].translation);
        let l2 = len(model.nodes[fingers[2]].translation);
        if l1 < 1e-4 || l2 < 1e-4 {
            return None;
        }
        let fx = rotate(f.rotation, [1.0, 0.0, 0.0]);
        let ((su, ru), (sf, rf), (sh, _)) =
            (rest_world(model, upper), rest_world(model, fore), rest_world(model, hand));
        let fold = cross(norm(sub(sf, su)), norm(sub(sh, sf)));
        let hinge = if len(fold) > 0.02 { norm(fold) } else { rotate(ru, [1.0, 0.0, 0.0]) };
        Some(Hand {
            upper,
            fore,
            hand,
            fingers,
            thumb,
            prop,
            mcp: f.translation,
            fx,
            fy: rotate(f.rotation, [0.0, 1.0, 0.0]),
            fz: rotate(f.rotation, [0.0, 0.0, 1.0]),
            l1,
            l2,
            thick: l1 * THICK,
            thumb_side: if dot(sub(model.nodes[thumb[0]].translation, f.translation), fx) >= 0.0 { 1.0 } else { -1.0 },
            len_upper: len(model.nodes[fore].translation),
            len_fore: len(model.nodes[hand].translation),
            hinge_upper: rotate(qinv(ru), hinge),
            hinge_fore: rotate(qinv(rf), hinge),
        })
    }
}

/// The knuckles' bend in a fist (rad): the haft lies that far into the palm.
const KNUCKLE: f32 = 1.22;
/// The least the last joints bend round a haft, and the most any joint bends.
const LAST_MIN: f32 = 0.4;
const JOINT_MAX: f32 = 1.9;

/// A hand closed on a haft of some radius: where the haft runs and every joint's turn.
struct Fist {
    /// The haft's middle in the hand's frame, and the way out of the fist past the knuckles
    /// (unit, across the haft): an edge points there.
    centre: V,
    front: V,
    /// The three finger joints' bends (rad), and the thumb's two local rotations (on their rest).
    bends: [f32; 3],
    thumb: [Q; 2],
}

impl Fist {
    fn new(model: &Model, h: &Hand, radius: f32) -> Fist {
        let r = radius + h.thick;
        // Each phalanx lies tangent to the haft: the turn at a joint is twice the angle its
        // tangent length subtends.
        let d0 = r * (KNUCKLE * 0.5).tan();
        let d0 = d0.min(h.l1 * 0.8);
        let t1 = 2.0 * (d0 / r).atan();
        let d1 = (h.l1 - d0).max(0.0);
        let t2 = (2.0 * (d1 / r).atan()).min(JOINT_MAX);
        let d2 = h.l2 - d1;
        let t3 = if d2 > 0.0 { 2.0 * (d2 / r).atan() } else { 0.0 }.clamp(LAST_MIN, JOINT_MAX);
        let centre = sub(sub(h.mcp, mul(h.fy, d0)), mul(h.fz, r));
        let p1 = sub(mul(h.fy, t1.cos()), mul(h.fz, t1.sin()));
        let pip = add(h.mcp, mul(p1, h.l1));
        let front = norm(perp(sub(pip, centre), h.fx));
        let mut fist = Fist { centre, front, bends: [t1, t2, t3], thumb: [ID, ID] };
        fist.thumb = fist.wrap_thumb(model, h, radius);
        fist
    }

    /// The thumb round the haft the other way: its tip toward the fingertips, off the haft.
    fn wrap_thumb(&self, model: &Model, h: &Hand, radius: f32) -> [Q; 2] {
        let (t1, t2) = (&model.nodes[h.thumb[0]], &model.nodes[h.thumb[1]]);
        let ft = h.thick * 1.1;
        let side = mul(h.fx, h.thumb_side);
        let to = norm(perp(sub(mul(h.fy, -1.0), mul(h.fz, 0.6)), h.fx));
        let target = add(add(self.centre, mul(to, radius + ft)), mul(side, h.thick * 3.0));
        let off = |p: V| len(perp(sub(p, self.centre), h.fx));
        let tip_len = len(t2.translation) * THUMB_TIP;
        let mut best = (f32::MAX, ID, ID);
        for ax in (-90..=60).step_by(15) {
            for az in (-90..=90).step_by(15) {
                for ay in (-60..=60).step_by(20) {
                    let q = qmul(
                        qmul(
                            axis_angle([0.0, 1.0, 0.0], (ay as f32).to_radians()),
                            axis_angle([0.0, 0.0, 1.0], (az as f32).to_radians()),
                        ),
                        axis_angle([1.0, 0.0, 0.0], (ax as f32).to_radians()),
                    );
                    let r1 = qmul(t1.rotation, q);
                    let ip = add(t1.translation, rotate(r1, t2.translation));
                    for a2 in [0, 20, 40, 60] {
                        let q2 = axis_angle([1.0, 0.0, 0.0], -(a2 as f32).to_radians());
                        let r2 = qmul(qmul(r1, t2.rotation), q2);
                        let tip = add(ip, rotate(r2, [0.0, tip_len, 0.0]));
                        let d = sub(tip, target);
                        let mut cost = dot(d, d);
                        for p in [ip, mul(add(ip, tip), 0.5), tip, mul(add(t1.translation, ip), 0.5)] {
                            let pen = (radius + ft - off(p)).max(0.0);
                            cost += 4.0 * pen * pen;
                        }
                        cost += 3e-8 * (ax * ax + az * az + 2 * ay * ay) as f32;
                        if cost < best.0 {
                            best = (cost, q, q2);
                        }
                    }
                }
            }
        }
        [best.1, best.2]
    }

    /// The joints' local rotations (fingers ×3, thumb ×2), `amount` of the way from `open`.
    fn joints(&self, model: &Model, h: &Hand, open: &[f32; 3], amount: f32) -> [Q; 5] {
        let bend = |i: usize| {
            let a = open[i] + (self.bends[i] - open[i]) * amount;
            qmul(model.nodes[h.fingers[i]].rotation, axis_angle([1.0, 0.0, 0.0], -a))
        };
        let thumb = |i: usize| qmul(model.nodes[h.thumb[i]].rotation, slerp(ID, self.thumb[i], amount));
        [bend(0), bend(1), bend(2), thumb(0), thumb(1)]
    }
}

/// A free hand's joints at ease (rad), and the mitten curl (rad) that is a closed fist.
const EASE: [f32; 3] = [0.2, 0.3, 0.17];
const MITTEN_FIST: f32 = 1.5;
/// The mitten curl a hand at ease already has: the joints start closing past it.
const MITTEN_EASE: f32 = 0.35;
/// The radius an empty fist closes on (m).
const EMPTY: f32 = 0.011;

/// How far the haft may tilt across the palm, toward the fingers (rad): a hammer's grip, to a
/// sabre's. And where round the haft the palm may sit: steps of a full turn.
const TILTS: [f32; 4] = [0.0, 0.22, 0.44, 0.66];
const TILT_EASY: f32 = 0.22;
const ROUND: usize = 36;
/// The wrist bends this far freely (rad), and turns (pronates) this far.
const SWING_FREE: f32 = 0.45;
const TWIST_FREE: f32 = 1.3;
/// A second hand closes on a haft its palm comes this near (m), wholly at `NEAR`.
const NEAR: f32 = 0.075;
const FAR: f32 = 0.15;

/// The item's frame in the hand for a tilt: (rotation, the fist's middle is the point `at` up it).
fn item_in_hand(h: &Hand, fist: &Fist, tilt: f32) -> Q {
    let z = norm(add(mul(h.fx, h.thumb_side * tilt.cos()), mul(h.fy, tilt.sin())));
    let y = norm(perp(mul(fist.front, -1.0), z));
    from_basis(cross(y, z), y, z)
}

/// One frame of a clip, as the solver needs it (world space).
struct Frame {
    shoulder: V,
    elbow: V,
    wrist: V,
    upper: Q,
    fore: Q,
    hand: Q,
    /// The haft: the point the fist holds and its direction.
    point: V,
    along: V,
}

/// A frame's answer: the arm's world rotations and the tilt.
#[derive(Clone, Copy)]
struct Solved {
    upper: Q,
    fore: Q,
    hand: Q,
    tilt: f32,
}

struct Solver<'a> {
    h: &'a Hand,
    fist: &'a Fist,
    /// The hand's rest rotation on the forearm.
    rest: Q,
    frames: [Q; 4],
}

impl Solver<'_> {
    /// The pose for the palm sitting `round` the haft (rad, from where the clip's hand had it)
    /// at tilt `j`, and what it costs.
    fn pose(&self, f: &Frame, e1: V, round: f32, j: usize) -> (Solved, f32) {
        let h = self.h;
        let e2 = cross(f.along, e1);
        let front = add(mul(e1, round.cos()), mul(e2, round.sin()));
        let y = mul(front, -1.0);
        let item = from_basis(cross(y, f.along), y, f.along);
        let hand = qmul(item, qinv(self.frames[j]));
        let wrist = sub(f.point, rotate(hand, self.fist.centre));
        // The elbow: where two bones reach the wrist, bent the way the clip bent it.
        let (l1, l2) = (h.len_upper, h.len_fore);
        let to = sub(wrist, f.shoulder);
        let d = len(to);
        let dc = d.clamp((l1 - l2).abs() + 1e-3, (l1 + l2) * 0.999);
        let dir = norm(to);
        let mut pole = perp(sub(f.elbow, f.shoulder), dir);
        if len(pole) < 1e-4 {
            pole = perp([0.0, -1.0, 0.0], dir);
        }
        let pole = norm(pole);
        let a = (l1 * l1 - l2 * l2 + dc * dc) / (2.0 * dc);
        let up = (l1 * l1 - a * a).max(0.0).sqrt();
        let elbow = add(add(f.shoulder, mul(dir, a)), mul(pole, up));
        let reached = add(f.shoulder, mul(dir, dc));
        // The arm's bones by its joints, not by the clip's own twists (a mitten hand's wrist was
        // turned any way that aimed its weapon): each lies along its bone with the elbow's hinge
        // across the plane the arm folds in.
        let (yu, yf) = (norm(sub(elbow, f.shoulder)), norm(sub(reached, elbow)));
        let fold = cross(yu, yf);
        let bone = |y: V, hinge: V, was: Q, hinge_in: V| {
            // Nearly straight: the fold is unsure, so the hinge stays where the clip had it.
            let n = if len(fold) > 0.05 { hinge } else { rotate(was, hinge_in) };
            let x = norm(perp(n, y));
            let lx = norm(perp(hinge_in, [0.0, 1.0, 0.0]));
            qmul(from_basis(x, y, cross(x, y)), qinv(from_basis(lx, [0.0, 1.0, 0.0], cross(lx, [0.0, 1.0, 0.0]))))
        };
        let upper = bone(yu, norm(fold), qmul(between(norm(sub(f.elbow, f.shoulder)), yu), f.upper), h.hinge_upper);
        let fore = bone(yf, norm(fold), qmul(between(norm(sub(f.wrist, f.elbow)), yf), f.fore), h.hinge_fore);
        // The wrist: how far the hand is from where the forearm carries it, as a turn about the
        // hand's own axis (the forearm's: palm up to palm down) and a bend.
        let rel = qmul(qinv(qmul(fore, self.rest)), hand);
        let rel = if rel[3] < 0.0 { [-rel[0], -rel[1], -rel[2], -rel[3]] } else { rel };
        let twist = 2.0 * rel[1].atan2(rel[3]);
        let twist = if twist > std::f32::consts::PI { twist - std::f32::consts::TAU } else { twist };
        let tq = qnorm([0.0, rel[1], 0.0, rel[3]]);
        let swing_q = qmul(rel, qinv(tq));
        let swing = 2.0 * swing_q[3].abs().min(1.0).acos();
        let over = |x: f32, free: f32| (x.abs() - free).max(0.0);
        let stretch = (d - (l1 + l2) * 0.98).max(0.0);
        let moved = len(sub(reached, f.wrist));
        let tilt = TILTS[j];
        let cost = 8.0 * over(swing, SWING_FREE).powi(2)
            + 0.2 * swing * swing
            + 12.0 * over(twist, TWIST_FREE).powi(2)
            + 0.05 * twist * twist
            + 300.0 * stretch * stretch
            + 1.5 * moved * moved
            + 0.15 * (tilt - TILT_EASY).powi(2);
        // The forearm takes half the turn (its two bones cross along its length; one bone here).
        let fore = qmul(fore, axis_angle([0.0, 1.0, 0.0], twist * 0.5));
        (Solved { upper, fore, hand, tilt }, cost)
    }

    /// Every frame's pose: the cheapest path through the clip, changing little frame to frame.
    fn solve(&self, frames: &[Frame], looping: bool) -> Vec<Solved> {
        let n = frames.len();
        if n == 0 {
            return Vec::new();
        }
        let states = ROUND * TILTS.len();
        let step = std::f32::consts::TAU / ROUND as f32;
        // Where the clip's own hand has the palm, each frame: the angles count from there.
        let mut e1s: Vec<V> = Vec::with_capacity(n);
        for f in frames {
            let own = perp(rotate(f.hand, self.fist.front), f.along);
            let e1 = if len(own) > 0.2 {
                norm(own)
            } else if let Some(prev) = e1s.last() {
                norm(perp(*prev, f.along))
            } else {
                norm(perp(sub(f.point, f.shoulder), f.along))
            };
            e1s.push(e1);
        }
        let cost_of =
            |t: usize, k: usize| self.pose(&frames[t], e1s[t], (k / TILTS.len()) as f32 * step, k % TILTS.len()).1;
        let costs: Vec<Vec<f32>> = (0..n).map(|t| (0..states).map(|k| cost_of(t, k)).collect()).collect();
        // A loop is walked twice and its second lap kept, so its ends meet.
        let laps = if looping && n > 2 { 2 } else { 1 };
        let total = n * laps;
        let mut best = costs[0].clone();
        let mut from: Vec<Vec<u16>> = Vec::with_capacity(total);
        from.push((0..states as u16).collect());
        for t in 1..total {
            let c = &costs[t % n];
            let mut next = vec![f32::MAX; states];
            let mut back = vec![0u16; states];
            for k in 0..states {
                let (i, j) = (k / TILTS.len(), k % TILTS.len());
                for di in -2i32..=2 {
                    let pi = (i as i32 + di).rem_euclid(ROUND as i32) as usize;
                    for dj in -1i32..=1 {
                        let pj = j as i32 + dj;
                        if pj < 0 || pj >= TILTS.len() as i32 {
                            continue;
                        }
                        let p = pi * TILTS.len() + pj as usize;
                        let turn = di as f32 * step;
                        let v = best[p] + 1.2 * turn * turn + 0.6 * (dj as f32 * 0.22).powi(2);
                        if v < next[k] {
                            next[k] = v;
                            back[k] = p as u16;
                        }
                    }
                }
                next[k] += c[k];
            }
            best = next;
            from.push(back);
        }
        let mut k = (0..states).min_by(|a, b| best[*a].total_cmp(&best[*b])).unwrap_or(0);
        let mut path = vec![0usize; total];
        for t in (0..total).rev() {
            path[t] = k;
            k = from[t][k] as usize;
        }
        let path = &path[total - n..];
        (0..n)
            .map(|t| {
                let k = path[t];
                self.pose(&frames[t], e1s[t], (k / TILTS.len()) as f32 * step, k % TILTS.len()).0
            })
            .collect()
    }
}

/// The times a clip is re-keyed at: its arm's own keys if it has them, else 30 a second.
fn times(a: &Animation, nodes: &[usize]) -> Vec<f32> {
    let dense = a
        .channels
        .iter()
        .filter(|c| c.path == "rotation" && nodes.contains(&c.node))
        .max_by_key(|c| c.times.len())
        .map(|c| c.times.clone())
        .unwrap_or_default();
    let end = a.channels.iter().filter_map(|c| c.times.last().copied()).fold(0.0f32, f32::max);
    if dense.len() >= 2 && (dense.len() as f32) >= end * 20.0 {
        return dense;
    }
    let n = (end * 30.0).round().max(1.0) as usize;
    (0..=n).map(|i| i as f32 / 30.0).collect()
}

fn put(a: &mut Animation, node: usize, path: &str, times: &[f32], values: Vec<Vec<f32>>) {
    a.channels.retain(|c| !(c.node == node && c.path == path));
    a.channels.push(Channel { node, path: path.to_string(), step: false, times: times.to_vec(), values });
}

/// A held item on its hand: the grasp, the hand's fist on it and the item's frame on the prop
/// bone at rest.
struct Held<'a> {
    grasp: &'a Grasp,
    side: usize,
    fist: Fist,
    /// The other hand's fist on the same haft (a second hand).
    other: Fist,
    /// The item's rotation on the prop bone, and its +Z there and on the hand.
    on_prop: Q,
    up_on_prop: V,
    up_on_hand: V,
}

/// Re-pose `model`'s clips for what its hands hold (`held`: the worn items gripped in a fist)
/// and relax its free hands. `kind` says how a clip plays. A model without finger joints is
/// left as it is.
pub fn apply(model: &mut Model, held: &[&Grasp], kind: &dyn Fn(&str) -> Kind) {
    let hands: Vec<Hand> = ["l", "r"].iter().filter_map(|s| Hand::read(model, s)).collect();
    if hands.len() != 2 {
        return;
    }
    let mut items: Vec<Held> = Vec::new();
    for g in held {
        let side = if g.hand == "l" { 0 } else { 1 };
        if items.iter().any(|i| i.side == side) {
            continue;
        }
        let h = &hands[side];
        let up = norm([g.up.0, g.up.1, g.up.2]);
        let back = norm(perp([g.back.0, g.back.1, g.back.2], up));
        let (_, hand_rest) = rest_world(model, h.hand);
        let (_, prop_rest) = rest_world(model, h.prop);
        let item_rest = qmul(hand_rest, from_basis(cross(back, up), back, up));
        let on_prop = qmul(qinv(prop_rest), item_rest);
        let other = Fist::new(model, &hands[1 - side], g.radius);
        items.push(Held {
            grasp: g,
            side,
            fist: Fist::new(model, h, g.radius),
            other,
            on_prop,
            up_on_prop: rotate(on_prop, [0.0, 0.0, 1.0]),
            up_on_hand: up,
        });
    }
    let empty: Vec<Fist> = hands.iter().map(|h| Fist::new(model, h, EMPTY)).collect();
    let mut animations = std::mem::take(&mut model.animations);
    for a in &mut animations {
        let Kind::Pose { looping } = kind(&a.name) else { continue };
        let arm: Vec<usize> = hands.iter().flat_map(|h| [h.upper, h.fore, h.hand]).collect();
        let ts = times(a, &arm);
        let source = a.clone();
        let tracks = Tracks::new(model, &source);
        let frame = |h: &Hand, t: f32, point: V, along: V| {
            let (shoulder, upper) = tracks.world(model, h.upper, t);
            let (elbow, fore) = tracks.world(model, h.fore, t);
            let (wrist, hand) = tracks.world(model, h.hand, t);
            Frame { shoulder, elbow, wrist, upper, fore, hand, point, along }
        };
        // The haft's line through the clip, per held item: where the fist holds it and its way.
        let line = |item: &Held, t: f32| {
            let h = &hands[item.side];
            if item.grasp.on_hand {
                // On the clip's hand, where the prop bone rests on it.
                let (p, r) = tracks.world(model, h.hand, t);
                return (add(p, rotate(r, model.nodes[h.prop].translation)), norm(rotate(r, item.up_on_hand)));
            }
            let (p, r) = tracks.world(model, h.prop, t);
            if item.grasp.upright {
                return (p, [0.0, 1.0, 0.0]);
            }
            (p, norm(rotate(r, item.up_on_prop)))
        };
        // Each hand's joints, and how much of a fist they are, frame by frame.
        let mut closed: [Vec<(f32, bool)>; 2] = [Vec::new(), Vec::new()];
        let mut arms: [Option<Vec<(Solved, f32)>>; 2] = [None, None];
        let mut fists: [Option<&Fist>; 2] = [None, None];
        for item in &items {
            let h = &hands[item.side];
            if item.grasp.strap {
                fists[item.side] = Some(&item.fist);
                closed[item.side] = vec![(1.0, true); ts.len()];
                let rest = model.nodes[h.hand].rotation.to_vec();
                put(a, h.hand, "rotation", &ts, vec![rest; ts.len()]);
                continue;
            }
            let frames: Vec<Frame> = ts
                .iter()
                .map(|&t| {
                    let (p, along) = line(item, t);
                    frame(h, t, p, along)
                })
                .collect();
            let solver = Solver {
                h,
                fist: &item.fist,
                rest: model.nodes[h.hand].rotation,
                frames: TILTS.map(|tilt| item_in_hand(h, &item.fist, tilt)),
            };
            let solved = solver.solve(&frames, looping);
            arms[item.side] = Some(solved.iter().map(|s| (*s, 1.0)).collect());
            fists[item.side] = Some(&item.fist);
            closed[item.side] = vec![(1.0, true); ts.len()];
            // The prop bone carries the item as the hand holds it.
            let mut rots = Vec::with_capacity(ts.len());
            let mut poss = Vec::with_capacity(ts.len());
            for s in &solved {
                let in_hand = item_in_hand(h, &item.fist, s.tilt);
                let prop = qmul(in_hand, qinv(item.on_prop));
                // The fist's middle is the prop bone's own place on the item (its head).
                rots.push(prop.to_vec());
                poss.push(item.fist.centre.to_vec());
            }
            put(a, h.prop, "rotation", &ts, rots);
            put(a, h.prop, "translation", &ts, poss);
        }
        // A second hand: one with nothing of its own, where the clip brings its palm to a haft.
        for side in 0..2 {
            if arms[side].is_some() || items.iter().any(|i| i.side == side) {
                continue;
            }
            let h = &hands[side];
            let Some(item) = items.iter().find(|i| i.side != side && i.grasp.span.1 - i.grasp.span.0 > 0.25) else {
                continue;
            };
            let fist = &item.other;
            let (lo, hi) = (item.grasp.span.0 - item.grasp.at, item.grasp.span.1 - item.grasp.at);
            let mut near = Vec::with_capacity(ts.len());
            let mut ups = 0.0;
            for &t in &ts {
                let (p, along) = line(item, t);
                let (palm, _) = tracks.world(model, h.prop, t);
                let s = dot(sub(palm, p), along).clamp(lo, hi);
                let on = add(p, mul(along, s));
                let g = smooth((FAR - len(sub(palm, on))) / (FAR - NEAR));
                let (_, hand) = tracks.world(model, h.hand, t);
                ups += g * dot(rotate(hand, mul(h.fx, h.thumb_side)), along);
                near.push((g, on, along));
            }
            if near.iter().all(|n| n.0 < 0.05) {
                continue;
            }
            let flip = if ups < 0.0 { -1.0 } else { 1.0 };
            let frames: Vec<Frame> = ts.iter().zip(&near).map(|(&t, n)| frame(h, t, n.1, mul(n.2, flip))).collect();
            let solver = Solver {
                h,
                fist,
                rest: model.nodes[h.hand].rotation,
                frames: TILTS.map(|tilt| item_in_hand(h, fist, tilt)),
            };
            let solved = solver.solve(&frames, looping);
            arms[side] = Some(solved.iter().zip(&near).map(|(s, n)| (*s, n.0)).collect());
            fists[side] = Some(fist);
            closed[side] = near.iter().map(|n| (n.0, true)).collect();
        }
        for side in 0..2 {
            let h = &hands[side];
            // The arm: the solved pose, as far as the hand holds.
            if let Some(solved) = &arms[side] {
                let mut keys: [Vec<Vec<f32>>; 3] = [Vec::new(), Vec::new(), Vec::new()];
                for (i, (s, g)) in solved.iter().enumerate() {
                    let t = ts[i];
                    let base = model.nodes[h.upper].parent.map(|p| tracks.world(model, p, t).1).unwrap_or(ID);
                    let want = [qmul(qinv(base), s.upper), qmul(qinv(s.upper), s.fore), qmul(qinv(s.fore), s.hand)];
                    for (k, node) in [h.upper, h.fore, h.hand].into_iter().enumerate() {
                        let was = tracks.local(model, node, t).1;
                        keys[k].push(qnorm(slerp(was, want[k], *g)).to_vec());
                    }
                }
                let [u, f, hd] = keys;
                put(a, h.upper, "rotation", &ts, u);
                put(a, h.fore, "rotation", &ts, f);
                put(a, h.hand, "rotation", &ts, hd);
            }
            // The joints: the clip's mitten curl as a curl of every joint; the fist where it holds.
            let mut keys: [Vec<Vec<f32>>; 5] = Default::default();
            for (i, &t) in ts.iter().enumerate() {
                let was = tracks.local(model, h.fingers[0], t).1;
                let rel = qmul(qinv(model.nodes[h.fingers[0]].rotation), was);
                let curl = 2.0 * rel[3].abs().min(1.0).acos();
                let free = empty[side].joints(
                    model,
                    h,
                    &EASE,
                    ((curl - MITTEN_EASE) / (MITTEN_FIST - MITTEN_EASE)).clamp(0.0, 1.0),
                );
                let g = closed[side].get(i).map(|c| c.0).unwrap_or(0.0);
                let joints = match fists[side] {
                    Some(fist) if g > 0.0 => {
                        let grip = fist.joints(model, h, &EASE, 1.0);
                        [0, 1, 2, 3, 4].map(|k| slerp(free[k], grip[k], g))
                    }
                    _ => free,
                };
                for k in 0..5 {
                    keys[k].push(qnorm(joints[k]).to_vec());
                }
            }
            for (k, node) in h.fingers.into_iter().chain(h.thumb).enumerate() {
                put(a, node, "rotation", &ts, std::mem::take(&mut keys[k]));
            }
        }
    }
    model.animations = animations;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn forged(who: &str) -> Model {
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art");
        let text = std::fs::read_to_string(art.join(format!("champions/neo/{who}-neo.champion.ron"))).unwrap();
        let r = crate::recipe::Recipe::from_text(&text).unwrap();
        crate::Catalog::load(&art).unwrap().forge(&r.name, &r.looks).unwrap().model
    }

    /// The most a joint bends from its rest through a clip (rad).
    fn bend(m: &Model, clip: &str, bone: &str) -> (f32, f32) {
        let node = m.node_by_name(bone).unwrap();
        let a = m.animations.iter().find(|a| a.name == clip).unwrap_or_else(|| panic!("no {clip}"));
        let c = a
            .channels
            .iter()
            .find(|c| c.node == node && c.path == "rotation")
            .unwrap_or_else(|| panic!("{bone} isn't keyed"));
        let rest = m.nodes[node].rotation;
        let of = |v: &Vec<f32>| 2.0 * qmul(qinv(rest), [v[0], v[1], v[2], v[3]])[3].abs().min(1.0).acos();
        c.values.iter().map(of).fold((f32::MAX, 0.0), |(lo, hi), x| (lo.min(x), hi.max(x)))
    }

    #[test]
    fn a_hand_closes_on_the_staff_it_holds_and_the_other_stays_open() {
        let m = forged("lumen");
        for clip in ["idle", "run", "attack_1"] {
            // The right hand's fingers wrap the staff, every joint, all the way through.
            assert!(bend(&m, clip, "fingers_r").0 > 0.9, "{clip}: the knuckles are open");
            assert!(bend(&m, clip, "fingers_02_r").0 > 0.9, "{clip}: the middle joints are open");
            assert!(bend(&m, clip, "thumb_r").0 > 0.2, "{clip}: the thumb hasn't come round");
            // The staff rides the prop bone, keyed with the hand.
            let prop = m.node_by_name("prop_r").unwrap();
            let a = m.animations.iter().find(|a| a.name == clip).unwrap();
            assert!(
                a.channels.iter().any(|c| c.node == prop && c.path == "translation"),
                "{clip}: the staff isn't carried"
            );
        }
    }

    #[test]
    fn the_wrist_stays_within_a_wrists_reach_of_the_forearm() {
        // The hand's turn off its rest on the forearm, through Shade's stabs and Rook's swings:
        // a bend and a twist together, never folded back on the arm.
        for (who, clips) in [("shade", ["idle", "attack_1", "run"]), ("rook", ["idle", "attack_1", "run"])] {
            let m = forged(who);
            for clip in clips {
                let (_, hi) = bend(&m, clip, "hand_r");
                assert!(hi < 2.2, "{who} {clip}: the wrist turns {hi} rad");
            }
        }
    }

    #[test]
    fn a_second_hand_closes_on_a_two_handed_haft() {
        let m = forged("rook");
        assert!(bend(&m, "attack_1", "fingers_02_l").1 > 0.9, "Rook's left hand never closes on the maul");
    }

    #[test]
    fn a_fist_wraps_tighter_round_a_thinner_haft() {
        let m = forged("lumen");
        let h = Hand::read(&m, "r").unwrap();
        let (thin, thick) = (Fist::new(&m, &h, 0.012), Fist::new(&m, &h, 0.03));
        let total = |f: &Fist| f.bends.iter().sum::<f32>();
        assert!(total(&thin) > total(&thick), "{} vs {}", total(&thin), total(&thick));
        // The haft lies on the palm's side of the knuckles, further in for a thicker one.
        let depth = |f: &Fist| -dot(sub(f.centre, h.mcp), h.fz);
        assert!(depth(&thick) > depth(&thin) && depth(&thin) > 0.0);
    }
}
