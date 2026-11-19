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
//! - **The arms keep out of the trunk.** A creator body is broader than the one a clip was made
//!   on: an arm the clip lays across the chest is swung and pushed clear of it (`Trunk`), two
//!   hands on one haft together. The neck and the head are the trunk's too, for the arm, and
//!   what a hand holds keeps out of the head.
//! - **A drawn bow** keeps its roll, and its string comes back to the fingers that hook it.
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
    scl: Vec<Option<&'a Channel>>,
}

impl<'a> Tracks<'a> {
    fn new(model: &Model, a: &'a Animation) -> Tracks<'a> {
        let n = model.nodes.len();
        let mut t = Tracks { rot: vec![None; n], pos: vec![None; n], scl: vec![None; n] };
        for c in &a.channels {
            match c.path.as_str() {
                "rotation" => t.rot[c.node] = Some(c),
                "translation" => t.pos[c.node] = Some(c),
                "scale" => t.scl[c.node] = Some(c),
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

    fn scale(&self, model: &Model, node: usize, t: f32) -> V {
        match self.scl[node] {
            Some(c) if !c.values.is_empty() => {
                let (a, b, u) = Self::key(c, t);
                let v = |i: usize| [c.values[i][0], c.values[i][1], c.values[i][2]];
                add(mul(v(a), 1.0 - u), mul(v(b), u))
            }
            _ => model.nodes[node].scale,
        }
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

/// The trunk's bones, and where an arm's bones stop being trunk.
const TRUNK: [&str; 6] = ["pelvis", "spine_01", "spine_02", "chest", "neck", "head"];
/// Of those, what a hand may touch (a chin, a cheek at the full draw) but an arm keeps out of.
const HEAD: [&str; 2] = ["neck", "head"];
const LIMBS: [&str; 5] = ["clavicle_l", "clavicle_r", "thigh_l", "thigh_r", "root"];
/// Each trunk bone's skin in this many slices along it.
const SLICES: usize = 4;

/// One slice of the trunk round a bone: an ellipse across it (in the bone's frame, at rest).
#[derive(Clone, Copy, Debug)]
struct Slice {
    y0: f32,
    y1: f32,
    cx: f32,
    cz: f32,
    rx: f32,
    rz: f32,
}

/// The trunk, as the arms must keep out of it: the body's skin and what is worn on it, sliced
/// round the spine's bones. An arm whose solved pose would pass through it is turned out.
struct Trunk {
    bones: Vec<(usize, Vec<Slice>)>,
    /// Per bone: the neck or the head (a hand may touch it).
    head: Vec<bool>,
    /// The arm's half-thickness, upper and fore, per side (l, r).
    arm: [[f32; 2]; 2],
}

/// The trunk bone a node hangs from, if it does (not through an arm, the neck or a leg).
fn trunk_of(model: &Model, mut node: usize) -> Option<usize> {
    loop {
        let n = &model.nodes[node];
        if TRUNK.contains(&n.name.as_str()) {
            return Some(node);
        }
        // An arm, a leg, the head; or what swings on springs (a cloak, a skirt's flaps).
        if LIMBS.contains(&n.name.as_str()) || n.name.starts_with("upperarm") || n.name.starts_with("extra_") {
            return None;
        }
        node = n.parent?;
    }
}

/// The arm bone (upper 0, fore 1) and side a node hangs from.
fn arm_of(model: &Model, mut node: usize) -> Option<(usize, usize)> {
    loop {
        let n = &model.nodes[node];
        let side = if n.name.ends_with("_l") { 0 } else { 1 };
        if n.name.starts_with("hand_") || TRUNK.contains(&n.name.as_str()) {
            return None;
        }
        if n.name.starts_with("forearm_") {
            return Some((1, side));
        }
        if n.name.starts_with("upperarm_") {
            return Some((0, side));
        }
        node = n.parent?;
    }
}

fn quantile(v: &mut [f32], q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(f32::total_cmp);
    v[((v.len() - 1) as f32 * q).round() as usize]
}

impl Trunk {
    /// Read off the skinned meshes: the body's (a node named `humanoid_*`) and `worn`'s.
    fn read(model: &Model, worn: &[usize]) -> Trunk {
        let mut points: Vec<Vec<V>> = vec![Vec::new(); model.nodes.len()];
        let mut arm: [[Vec<f32>; 2]; 2] = Default::default();
        let rest: Vec<(V, Q)> = (0..model.nodes.len()).map(|i| rest_world(model, i)).collect();
        for (i, n) in model.nodes.iter().enumerate() {
            let (Some(mesh), Some(skin)) = (n.mesh, n.skin) else { continue };
            if !(n.name.starts_with("humanoid_") || worn.contains(&i)) {
                continue;
            }
            let joints = &model.skins[skin];
            for prim in &model.meshes[mesh] {
                for (k, p) in prim.positions.iter().enumerate() {
                    let (Some(j), Some(w)) = (prim.joints.get(k), prim.weights.get(k)) else { continue };
                    let top = (0..4).max_by(|a, b| w[*a].total_cmp(&w[*b])).unwrap_or(0);
                    let Some(&node) = joints.get(j[top] as usize) else { continue };
                    if let Some(b) = trunk_of(model, node) {
                        let (t, r) = rest[b];
                        points[b].push(rotate(qinv(r), sub(*p, t)));
                    } else if n.name.starts_with("humanoid_")
                        && let Some((bone, side)) = arm_of(model, node)
                    {
                        // How far the arm's skin is from its bone's line.
                        let b = if bone == 0 {
                            format!("upperarm_{}", ["l", "r"][side])
                        } else {
                            format!("forearm_{}", ["l", "r"][side])
                        };
                        if let Some(b) = model.node_by_name(&b) {
                            let (t, r) = rest[b];
                            let q = rotate(qinv(r), sub(*p, t));
                            arm[side][bone].push((q[0] * q[0] + q[2] * q[2]).sqrt());
                        }
                    }
                }
            }
        }
        let mut bones = Vec::new();
        for (b, mut pts) in points.into_iter().enumerate() {
            if pts.len() < 40 {
                continue;
            }
            pts.sort_by(|a, b| a[1].total_cmp(&b[1]));
            let per = pts.len().div_ceil(SLICES);
            let slices = pts
                .chunks(per)
                .map(|c| {
                    let mut xs: Vec<f32> = c.iter().map(|p| p[0]).collect();
                    let mut zs: Vec<f32> = c.iter().map(|p| p[2]).collect();
                    let (x0, x1) = (quantile(&mut xs, 0.06), quantile(&mut xs, 0.94));
                    let (z0, z1) = (quantile(&mut zs, 0.06), quantile(&mut zs, 0.94));
                    Slice {
                        y0: c[0][1],
                        y1: c[c.len() - 1][1],
                        cx: (x0 + x1) * 0.5,
                        cz: (z0 + z1) * 0.5,
                        rx: (x1 - x0) * 0.5,
                        rz: (z1 - z0) * 0.5,
                    }
                })
                .collect();
            bones.push((b, slices));
        }
        let head = bones.iter().map(|(b, _)| HEAD.contains(&model.nodes[*b].name.as_str())).collect();
        let arm = arm.map(|s| s.map(|mut v| if v.is_empty() { 0.04 } else { quantile(&mut v, 0.4) }));
        Trunk { bones, head, arm }
    }

    /// The trunk bones' world places in a frame.
    fn at(&self, tracks: &Tracks, model: &Model, t: f32) -> Vec<(V, Q)> {
        self.bones.iter().map(|(b, _)| tracks.world(model, *b, t)).collect()
    }

    /// How deep a ball (`p`, radius `r`) is in the trunk (m), the trunk's bones at `at`.
    fn depth(&self, at: &[(V, Q)], p: V, r: f32) -> f32 {
        self.depth_in(at, p, r, |_| true)
    }

    /// How deep a ball is in the neck or the head alone (m): what a haft keeps out of.
    fn head_depth(&self, at: &[(V, Q)], p: V, r: f32) -> f32 {
        self.depth_in(at, p, r, |head| head)
    }

    /// As `depth`, in the bones `keep` keeps (it's told whether a bone is the neck or the head).
    fn depth_in(&self, at: &[(V, Q)], p: V, r: f32, keep: impl Fn(bool) -> bool) -> f32 {
        let mut deepest = 0.0f32;
        for (((_, slices), (t, q)), &head) in self.bones.iter().zip(at).zip(&self.head) {
            if !keep(head) {
                continue;
            }
            let l = rotate(qinv(*q), sub(p, *t));
            for s in slices {
                if l[1] < s.y0 - r * 0.5 || l[1] > s.y1 + r * 0.5 {
                    continue;
                }
                let (ax, az) = (s.rx + r, s.rz + r);
                let e = (((l[0] - s.cx) / ax).powi(2) + ((l[2] - s.cz) / az).powi(2)).sqrt();
                if e < 1.0 {
                    deepest = deepest.max((1.0 - e) * ax.min(az));
                }
            }
        }
        deepest
    }

    /// How far an arm (shoulder, elbow, wrist; the hand's middle) is in the trunk: the deepest
    /// of points along it, past the shoulder's own joint. The hand and the wrist may touch the
    /// head (a hand at the chin), the rest of the arm keeps out of it.
    fn arm_depth(&self, at: &[(V, Q)], side: usize, shoulder: V, elbow: V, wrist: V, hand: V) -> f32 {
        let [ru, rf] = self.arm[side];
        let mut d = 0.0f32;
        for u in [0.55, 0.8, 1.0] {
            d = d.max(self.depth(at, add(shoulder, mul(sub(elbow, shoulder), u)), ru));
        }
        for u in [0.33, 0.66, 1.0] {
            d = d.max(self.depth_in(at, add(elbow, mul(sub(wrist, elbow), u)), rf, |head| !head || u < 0.9));
        }
        d.max(self.depth_in(at, hand, rf, |head| !head))
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
#[derive(Clone)]
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
/// A hand hooked on a bowstring: the knuckles nearly straight, the fingers' last two joints
/// curled round the string (rad).
const HOOK: [f32; 3] = [0.3, 1.45, 0.85];

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
/// How far round the shoulder-to-wrist line the elbow may swing (rad), and what that costs.
const SWIVELS: [f32; 7] = [0.0, -0.3, 0.3, -0.6, 0.6, -0.95, 0.95];
const SWIVEL_COST: f32 = 0.4;
/// An arm this deep in the trunk (m) only touches it; past that, depth costs this (per m²).
const CONTACT: f32 = 0.012;
const DEEP: f32 = 900.0;
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
    /// The trunk's bones' places.
    trunk: Vec<(V, Q)>,
    /// Where the knuckles must face, if the item's own roll is kept (a bow's back to the mark).
    roll: Option<V>,
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
    trunk: &'a Trunk,
    side: usize,
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
        let reached = add(f.shoulder, mul(dir, dc));
        // The elbow swings out round the line from shoulder to wrist as far as it must to keep
        // the arm out of the trunk.
        let mut elbow = add(add(f.shoulder, mul(dir, a)), mul(pole, up));
        let mut clear = f32::MAX;
        for phi in SWIVELS {
            let e = add(add(f.shoulder, mul(dir, a)), mul(rotate(axis_angle(dir, phi), pole), up));
            let deep = self.trunk.arm_depth(&f.trunk, self.side, f.shoulder, e, reached, f.point);
            let c = SWIVEL_COST * phi * phi + DEEP * (deep - CONTACT).max(0.0).powi(2);
            if c < clear {
                (clear, elbow) = (c, e);
            }
        }
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
            + 0.15 * (tilt - TILT_EASY).powi(2)
            + clear;
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
            let own = perp(f.roll.unwrap_or_else(|| rotate(f.hand, self.fist.front)), f.along);
            let e1 = if len(own) > 0.2 {
                norm(own)
            } else if let Some(prev) = e1s.last() {
                norm(perp(*prev, f.along))
            } else {
                norm(perp(sub(f.point, f.shoulder), f.along))
            };
            e1s.push(e1);
        }
        let cost_of = |t: usize, k: usize| {
            let i = k / TILTS.len();
            // A kept roll: the palm a step either way of it at most.
            if frames[t].roll.is_some() && i > 1 && i < ROUND - 1 {
                return 1e6;
            }
            self.pose(&frames[t], e1s[t], i as f32 * step, k % TILTS.len()).1
        };
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

/// The times a clip is re-keyed at: 30 a second, or its arm's own keys if they're denser.
fn times(a: &Animation, nodes: &[usize]) -> Vec<f32> {
    let dense = a
        .channels
        .iter()
        .filter(|c| c.path == "rotation" && nodes.contains(&c.node))
        .max_by_key(|c| c.times.len())
        .map(|c| c.times.clone())
        .unwrap_or_default();
    let end = a.channels.iter().filter_map(|c| c.times.last().copied()).fold(0.0f32, f32::max);
    // Every frame at least: an arm keyed sparser than the body it hangs from (a whirlwind's
    // spine keyed each frame, its arms each other) swings through the body between its keys.
    let n = (end * 30.0).round().max(1.0) as usize;
    if dense.len() > n + 1 {
        return dense;
    }
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
    back_on_hand: V,
}

/// A bow with a string to draw (Vesper's bones: a string half off each limb's tip, meeting at
/// the nock).
struct Bow {
    /// The hand that holds it, and the bone it is carried by.
    side: usize,
    frame: usize,
    strings: [usize; 2],
    /// A string half's length at rest, and the nock there, in `frame`'s frame.
    half: f32,
    rest: V,
}

impl Bow {
    fn read(model: &Model) -> Option<Bow> {
        let strings = [model.node_by_name("extra_string_1")?, model.node_by_name("extra_string_2")?];
        let mut frame = model.nodes[strings[0]].parent?;
        while !(model.nodes[frame].name.starts_with("prop_") || model.nodes[frame].name.starts_with("hand_")) {
            frame = model.nodes[frame].parent?;
        }
        let side = if model.nodes[frame].name.ends_with("_l") { 0 } else { 1 };
        let ((a, ra), (b, _)) = (rest_world(model, strings[0]), rest_world(model, strings[1]));
        let half = len(sub(a, b)) * 0.5;
        let (fp, fq) = rest_world(model, frame);
        let nock = add(a, rotate(ra, [0.0, half, 0.0]));
        Some(Bow { side, frame, strings, half, rest: rotate(qinv(fq), sub(nock, fp)) })
    }

    /// How far the clip draws the string (0 to 1) and where it has the nock, in the bow's frame.
    fn nock(&self, model: &Model, tracks: &Tracks, t: f32) -> (f32, V) {
        let (p, q) = tracks.world(model, self.strings[0], t);
        let s = tracks.scale(model, self.strings[0], t);
        let nock = add(p, rotate(q, [0.0, self.half * s[1], 0.0]));
        let (fp, fq) = tracks.world(model, self.frame, t);
        let local = rotate(qinv(fq), sub(nock, fp));
        (smooth((len(sub(local, self.rest)) - 0.01) / 0.04), local)
    }
}

/// The elbow for a wrist: two bones from the shoulder, bent toward `pole` turned `phi` round the
/// shoulder-to-wrist line. And the wrist as far as the arm reaches.
fn elbow_for(h: &Hand, shoulder: V, wrist: V, pole: V, phi: f32) -> (V, V) {
    let (l1, l2) = (h.len_upper, h.len_fore);
    let to = sub(wrist, shoulder);
    let dc = len(to).clamp((l1 - l2).abs() + 1e-3, (l1 + l2) * 0.999);
    let dir = norm(to);
    let mut p = perp(pole, dir);
    if len(p) < 1e-4 {
        p = perp([0.0, -1.0, 0.0], dir);
    }
    let a = (l1 * l1 - l2 * l2 + dc * dc) / (2.0 * dc);
    let up = (l1 * l1 - a * a).max(0.0).sqrt();
    let elbow = add(add(shoulder, mul(dir, a)), mul(rotate(axis_angle(dir, phi), norm(p)), up));
    (elbow, add(shoulder, mul(dir, dc)))
}

/// How far the wrist may be pushed out of the trunk (m), and what that costs (per m²).
const PUSHES: [f32; 9] = [0.0, 0.02, 0.04, 0.07, 0.1, 0.14, 0.2, 0.27, 0.35];
/// The elbow's swings the trunk pass tries (rad): further than the grip solver's.
const SWINGS: [f32; 11] = [0.0, -0.3, 0.3, -0.6, 0.6, -0.95, 0.95, -1.3, 1.3, -1.7, 1.7];
const PUSH_COST: f32 = 30.0;
/// A held item is kept out of the head at this many of its points.
const HELD_POINTS: usize = 48;
/// Frames either side a push holds and fades over.
const PUSH_HOLD: i32 = 3;

/// What rides `prop` of the `worn` meshes (a held item's skin), as points on the bone at rest:
/// at most `HELD_POINTS`, spread over it.
fn held_points(model: &Model, worn: &[usize], prop: usize) -> Vec<V> {
    let mut points = Vec::new();
    let (t, q) = rest_world(model, prop);
    for &i in worn {
        let n = &model.nodes[i];
        let (Some(mesh), Some(skin)) = (n.mesh, n.skin) else { continue };
        for prim in &model.meshes[mesh] {
            for (k, p) in prim.positions.iter().enumerate() {
                let (Some(j), Some(w)) = (prim.joints.get(k), prim.weights.get(k)) else { continue };
                let top = (0..4).max_by(|a, b| w[*a].total_cmp(&w[*b])).unwrap_or(0);
                if model.skins[skin].get(j[top] as usize) == Some(&prop) {
                    points.push(rotate(qinv(q), sub(*p, t)));
                }
            }
        }
    }
    let step = points.len().div_ceil(HELD_POINTS).max(1);
    points.into_iter().step_by(step).collect()
}

/// An arm the trunk pass moves: its hand, its side, and how far it follows the group's push,
/// frame by frame (a second hand on a haft goes where the haft goes, as far as it holds it).
struct Mover<'a> {
    h: &'a Hand,
    side: usize,
    follow: Vec<f32>,
    /// What the hand holds, frame by frame: points of it from the wrist (the hand keeps its
    /// turn, so they move with the wrist). It keeps out of the head.
    haft: Option<Vec<Vec<V>>>,
}

impl Mover<'_> {
    /// How deep the arm, and what it holds, are in the trunk at frame `i` (the haft in the
    /// head only: a maul may rest on a shoulder, it doesn't go through a skull). `arm`: the
    /// shoulder, the elbow, the wrist and the hand's middle.
    fn depth(&self, trunk: &Trunk, at: &[(V, Q)], i: usize, arm: (V, V, V, V)) -> f32 {
        let (s, e, w, hp) = arm;
        let mut d = trunk.arm_depth(at, self.side, s, e, w, hp);
        if let Some(points) = &self.haft {
            for o in &points[i] {
                d = d.max(trunk.head_depth(at, add(w, *o), 0.0));
            }
        }
        d
    }
}

/// Keep arms out of the trunk through a clip (`tracks`, keyed at `ts`). The arms of a group
/// move together: two hands on one haft are pushed as one, the haft with them. Where one passes
/// through, its elbow swings out and the group's wrists are pushed clear, the hands turned as
/// they were; the change held and faded over a few frames either side. Each arm's new local
/// rotations (upper, fore, hand), or None if the group never goes in.
fn clear_arms(
    model: &Model,
    group: &[Mover],
    trunk: &Trunk,
    tracks: &Tracks,
    ts: &[f32],
    looping: bool,
) -> Option<Vec<[Vec<Vec<f32>>; 3]>> {
    let n = ts.len();
    let m = group.len();
    // Per frame: the group's push, each arm's elbow swing and its own push (for what it doesn't
    // follow of the group's: a second hand off the haft).
    let mut moves: Vec<(V, Vec<f32>, Vec<V>)> = vec![([0.0; 3], vec![0.0; m], vec![[0.0; 3]; m]); n];
    // Round, level and 45° up and down; straight up and down. An arm over the head is cleared
    // upward, one across the belly down.
    let mut ways: Vec<V> = vec![[0.0, 1.0, 0.0], [0.0, -1.0, 0.0]];
    for k in 0..8 {
        let ang = k as f32 * std::f32::consts::FRAC_PI_4;
        for y in [0.0f32, 1.0, -1.0] {
            ways.push(norm([ang.cos(), y, ang.sin()]));
        }
    }
    let mut any = false;
    let joints = |mv: &Mover, t: f32| {
        let h = mv.h;
        (
            tracks.world(model, h.upper, t).0,
            tracks.world(model, h.fore, t).0,
            tracks.world(model, h.hand, t).0,
            tracks.world(model, h.prop, t).0,
        )
    };
    for (i, &t) in ts.iter().enumerate() {
        let at = trunk.at(tracks, model, t);
        let arms: Vec<(V, V, V, V)> = group.iter().map(|mv| joints(mv, t)).collect();
        if group.iter().zip(&arms).all(|(mv, &(s, e, w, hp))| mv.depth(trunk, &at, i, (s, e, w, hp)) <= CONTACT) {
            continue;
        }
        let mut best = (f32::MAX, [0.0; 3], vec![0.0; m]);
        for (k, &way) in ways.iter().enumerate() {
            for (pi, &amount) in PUSHES.iter().enumerate() {
                if pi == 0 && k > 0 {
                    continue;
                }
                let push = mul(way, amount);
                let mut cost = PUSH_COST * amount * amount;
                let mut phis = vec![0.0; m];
                for (a, (mv, &(s, e, w, hp))) in group.iter().zip(&arms).enumerate() {
                    let follow = mul(push, mv.follow[i]);
                    let mut arm = f32::MAX;
                    for phi in SWINGS {
                        let (e2, w2) = elbow_for(mv.h, s, add(w, follow), sub(e, s), phi);
                        let moved = sub(w2, w);
                        let deep = mv.depth(trunk, &at, i, (s, e2, w2, add(hp, moved)));
                        // Short of reach, the hand comes off what it holds.
                        let off = len(sub(moved, follow));
                        let c = DEEP * (deep - CONTACT).max(0.0).powi(2)
                            + SWIVEL_COST * phi * phi
                            + PUSH_COST * 4.0 * off * off;
                        if c < arm {
                            (arm, phis[a]) = (c, phi);
                        }
                    }
                    cost += arm;
                }
                if cost < best.0 {
                    best = (cost, push, phis);
                }
            }
        }
        // An arm that only partly follows the group pushes itself clear for the rest.
        let mut owns = vec![[0.0; 3]; m];
        for (a, (mv, &(s, e, w, hp))) in group.iter().zip(&arms).enumerate() {
            let f = mv.follow[i];
            if f > 0.99 {
                continue;
            }
            let follow = mul(best.1, f);
            let mut arm = (f32::MAX, [0.0; 3], best.2[a]);
            for (k, &way) in ways.iter().enumerate() {
                for (pi, &amount) in PUSHES.iter().enumerate() {
                    if pi == 0 && k > 0 {
                        continue;
                    }
                    let own = mul(way, amount);
                    let goal = add(follow, mul(own, 1.0 - f));
                    for phi in SWINGS {
                        let (e2, w2) = elbow_for(mv.h, s, add(w, goal), sub(e, s), phi);
                        let moved = sub(w2, w);
                        let deep = mv.depth(trunk, &at, i, (s, e2, w2, add(hp, moved)));
                        let off = len(sub(moved, goal));
                        let c = DEEP * (deep - CONTACT).max(0.0).powi(2)
                            + SWIVEL_COST * phi * phi
                            + PUSH_COST * amount * amount
                            + PUSH_COST * 4.0 * off * off;
                        if c < arm.0 {
                            arm = (c, own, phi);
                        }
                    }
                }
            }
            owns[a] = arm.1;
            best.2[a] = arm.2;
        }
        moves[i] = (best.1, best.2, owns);
        any = true;
    }
    if !any {
        return None;
    }
    // Held, then faded, as places: the elbows' and the wrists' moves (what the swings and the
    // push did), the largest near each frame held, then smoothed. The moves are the chest's
    // (in its frame): a body turning fast (a whirlwind) takes them round with it, where the
    // world's would carry one frame's way out into the next frame's way in.
    let chest = model.node_by_name("chest");
    let frame_of = |t: f32| chest.map(|c| tracks.world(model, c, t).1).unwrap_or(ID);
    let arm_moves = |i: usize| -> Vec<(V, V)> {
        let t = ts[i];
        let (push, phis, owns) = &moves[i];
        let to_chest = qinv(frame_of(t));
        group
            .iter()
            .enumerate()
            .map(|(a, mv)| {
                let h = mv.h;
                let (s, e, w) = (
                    tracks.world(model, h.upper, t).0,
                    tracks.world(model, h.fore, t).0,
                    tracks.world(model, h.hand, t).0,
                );
                let f = mv.follow[i];
                let goal = add(mul(*push, f), mul(owns[a], 1.0 - f));
                let (e2, w2) = elbow_for(h, s, add(w, goal), sub(e, s), phis[a]);
                (rotate(to_chest, sub(e2, e)), rotate(to_chest, sub(w2, w)))
            })
            .collect()
    };
    let raw: Vec<Vec<(V, V)>> = (0..n).map(arm_moves).collect();
    let size = |m: &Vec<(V, V)>| m.iter().map(|(e, w)| len(*e) + len(*w)).sum::<f32>();
    let at = |i: i32| if looping { i.rem_euclid(n as i32) as usize } else { i.clamp(0, n as i32 - 1) as usize };
    let held: Vec<&Vec<(V, V)>> = (0..n as i32)
        .map(|i| {
            (-PUSH_HOLD..=PUSH_HOLD)
                .map(|d| &raw[at(i + d)])
                .fold(&raw[at(i)], |a, b| if size(b) > size(a) { b } else { a })
        })
        .collect();
    // A frame's arms given their moves (in the chest's frame): the elbow, the wrist, and how deep
    // the group then is.
    let place = |i: usize, mv_of: &[(V, V)]| -> (Vec<(V, V)>, f32) {
        let t = ts[i];
        let q = frame_of(t);
        let trunk_at = trunk.at(tracks, model, t);
        let mut deep = 0.0f32;
        let placed = group
            .iter()
            .zip(mv_of)
            .map(|(mv, (de, dw))| {
                let h = mv.h;
                let s = tracks.world(model, h.upper, t).0;
                let e = tracks.world(model, h.fore, t).0;
                let w = tracks.world(model, h.hand, t).0;
                let hp = tracks.world(model, h.prop, t).0;
                let (de, dw) = (rotate(q, *de), rotate(q, *dw));
                let (e2, w2) = elbow_for(h, s, add(w, dw), sub(add(e, de), s), 0.0);
                deep = deep.max(mv.depth(trunk, &trunk_at, i, (s, e2, w2, add(hp, sub(w2, w)))));
                (e2, w2)
            })
            .collect();
        (placed, deep)
    };
    let mut out: Vec<[Vec<Vec<f32>>; 3]> = (0..m).map(|_| Default::default()).collect();
    for (i, &t) in ts.iter().enumerate() {
        let mut sm = vec![([0.0; 3], [0.0; 3]); m];
        let mut sum = 0.0;
        for d in -PUSH_HOLD..=PUSH_HOLD {
            let wgt = (PUSH_HOLD + 1 - d.abs()) as f32;
            for (a, (de, dw)) in held[at(i as i32 + d)].iter().enumerate() {
                sm[a] = (add(sm[a].0, mul(*de, wgt)), add(sm[a].1, mul(*dw, wgt)));
            }
            sum += wgt;
        }
        let sm: Vec<(V, V)> = sm.iter().map(|(e, w)| (mul(*e, 1.0 / sum), mul(*w, 1.0 / sum))).collect();
        // The smoothed moves, unless they'd put the arm in where this frame's own move doesn't
        // (a neighbour's move, where the arm has swung somewhere else).
        let (smoothed, deep_sm) = place(i, &sm);
        let (own, deep_own) = place(i, &raw[i]);
        let placed = if deep_sm <= CONTACT.max(deep_own + 0.01) { smoothed } else { own };
        for (a, mv) in group.iter().enumerate() {
            let h = mv.h;
            let (s, u) = tracks.world(model, h.upper, t);
            let (e, f) = tracks.world(model, h.fore, t);
            let (w, hq) = tracks.world(model, h.hand, t);
            let base = model.nodes[h.upper].parent.map(|p| tracks.world(model, p, t).1).unwrap_or(ID);
            let (e2, w2) = placed[a];
            let u2 = qmul(between(norm(sub(e, s)), norm(sub(e2, s))), u);
            let f2 = qmul(between(norm(sub(w, e)), norm(sub(w2, e2))), f);
            out[a][0].push(qnorm(qmul(qinv(base), u2)).to_vec());
            out[a][1].push(qnorm(qmul(qinv(u2), f2)).to_vec());
            out[a][2].push(qnorm(qmul(qinv(f2), hq)).to_vec());
        }
    }
    Some(out)
}

/// Re-pose `model`'s clips for what its hands hold (`held`: the worn items gripped in a fist),
/// relax its free hands and keep its arms out of its trunk (the body's and the `worn` mesh
/// nodes'). `kind` says how a clip plays. A model without finger joints is left as it is.
pub fn apply(model: &mut Model, held: &[&Grasp], worn: &[usize], kind: &dyn Fn(&str) -> Kind) {
    let hands: Vec<Hand> = ["l", "r"].iter().filter_map(|s| Hand::read(model, s)).collect();
    if hands.len() != 2 {
        return;
    }
    let trunk = Trunk::read(model, worn);
    let bow = Bow::read(model);
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
            back_on_hand: back,
        });
    }
    let empty: Vec<Fist> = hands.iter().map(|h| Fist::new(model, h, EMPTY)).collect();
    let held: Vec<Vec<V>> = hands.iter().map(|h| held_points(model, worn, h.prop)).collect();
    let mut animations = std::mem::take(&mut model.animations);
    for a in &mut animations {
        let Kind::Pose { looping } = kind(&a.name) else { continue };
        let arm: Vec<usize> = hands.iter().flat_map(|h| [h.upper, h.fore, h.hand]).collect();
        let ts = times(a, &arm);
        let source = a.clone();
        let tracks = Tracks::new(model, &source);
        let frame = |h: &Hand, t: f32, point: V, along: V, roll: Option<V>| {
            let (shoulder, upper) = tracks.world(model, h.upper, t);
            let (elbow, fore) = tracks.world(model, h.fore, t);
            let (wrist, hand) = tracks.world(model, h.hand, t);
            let roll = roll.map(|b| mul(rotate(hand, b), -1.0));
            Frame { shoulder, elbow, wrist, upper, fore, hand, point, along, trunk: trunk.at(&tracks, model, t), roll }
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
                    // A bow keeps its own roll: its back to the mark, its string to the archer.
                    frame(h, t, p, along, item.grasp.on_hand.then_some(item.back_on_hand))
                })
                .collect();
            let solver = Solver {
                h,
                fist: &item.fist,
                trunk: &trunk,
                side: item.side,
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
            let frames: Vec<Frame> =
                ts.iter().zip(&near).map(|(&t, n)| frame(h, t, n.1, mul(n.2, flip), None)).collect();
            let solver = Solver {
                h,
                fist,
                trunk: &trunk,
                side,
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
        }
        // Arms out of the trunk: what the solver couldn't swing clear, and the free arms.
        // Two hands on one haft go together; every other arm goes alone.
        let second = (0..2).find(|&s| arms[s].is_some() && !items.iter().any(|i| i.side == s));
        // What the arms carry: its points from each hand's wrist, as solved.
        let keyed = a.clone();
        let tracks_keyed = Tracks::new(model, &keyed);
        let haft = |side: usize| -> Option<Vec<Vec<V>>> {
            items.iter().find(|i| i.side == side && !i.grasp.strap && !i.grasp.on_hand)?;
            let h = &hands[side];
            let points = &held[side];
            if points.is_empty() {
                return None;
            }
            Some(
                ts.iter()
                    .map(|&t| {
                        let w = tracks_keyed.world(model, h.hand, t).0;
                        let (p, r) = tracks_keyed.world(model, h.prop, t);
                        points.iter().map(|o| sub(add(p, rotate(r, *o)), w)).collect()
                    })
                    .collect(),
            )
        };
        let mut groups: Vec<Vec<Mover>> = Vec::new();
        for side in 0..2 {
            if Some(side) == second {
                continue;
            }
            let mut g = vec![Mover { h: &hands[side], side, follow: vec![1.0; ts.len()], haft: haft(side) }];
            if let Some(s2) = second.filter(|&s2| s2 != side) {
                let follow = arms[s2].as_ref().map(|v| v.iter().map(|x| x.1).collect()).unwrap_or_default();
                g.push(Mover { h: &hands[s2], side: s2, follow, haft: None });
            }
            groups.push(g);
        }
        for g in &groups {
            // Twice: the second pass clears what the first's smoothing left.
            for _ in 0..2 {
                let now = a.clone();
                let tracks_now = Tracks::new(model, &now);
                if let Some(keys) = clear_arms(model, g, &trunk, &tracks_now, &ts, looping) {
                    for (mv, [u, f, hd]) in g.iter().zip(keys) {
                        // A strapped shield's hand stays as it rests on the forearm, the shield's
                        // grip in its fist.
                        let hd = if items.iter().any(|i| i.side == mv.side && i.grasp.strap) {
                            vec![model.nodes[mv.h.hand].rotation.to_vec(); ts.len()]
                        } else {
                            hd
                        };
                        put(a, mv.h.upper, "rotation", &ts, u);
                        put(a, mv.h.fore, "rotation", &ts, f);
                        put(a, mv.h.hand, "rotation", &ts, hd);
                    }
                }
            }
        }
        // A drawn bow: how far the clip draws it, frame by frame (its string hand hooks it).
        let draw = bow.as_ref().filter(|b| source.channels.iter().any(|c| c.node == b.strings[0])).map(|b| {
            let pulls: Vec<(f32, V)> = ts.iter().map(|&t| b.nock(model, &tracks, t)).collect();
            (b, pulls)
        });
        for side in 0..2 {
            let h = &hands[side];
            let hook = draw.as_ref().filter(|(b, _)| b.side != side).map(|(_, p)| p);
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
                let joints = match hook {
                    Some(pulls) if pulls[i].0 > 0.0 => {
                        let hooked = Fist { bends: HOOK, ..empty[side].clone() }.joints(model, h, &EASE, 1.0);
                        let thumb = empty[side].joints(model, h, &EASE, 0.55);
                        let hooked = [hooked[0], hooked[1], hooked[2], thumb[3], thumb[4]];
                        [0, 1, 2, 3, 4].map(|k| slerp(joints[k], hooked[k], pulls[i].0))
                    }
                    _ => joints,
                };
                for k in 0..5 {
                    keys[k].push(qnorm(joints[k]).to_vec());
                }
            }
            for (k, node) in h.fingers.into_iter().chain(h.thumb).enumerate() {
                put(a, node, "rotation", &ts, std::mem::take(&mut keys[k]));
            }
        }
        // The string comes back to the fingers that hook it, as far as the clip draws it.
        if let Some((b, pulls)) = &draw {
            let now = a.clone();
            let tracks_now = Tracks::new(model, &now);
            let h = &hands[1 - b.side];
            let mut rots: [Vec<Vec<f32>>; 2] = Default::default();
            let mut scls: [Vec<Vec<f32>>; 2] = Default::default();
            for (i, &t) in ts.iter().enumerate() {
                let (w, local) = pulls[i];
                let (pp, pq) = tracks_now.world(model, b.frame, t);
                let nock = add(pp, rotate(pq, local));
                let (fp, fq) = tracks_now.world(model, h.fingers[2], t);
                let hook = add(fp, rotate(fq, [0.0, h.thick * 0.5, -h.thick * 1.2]));
                let target = add(mul(nock, 1.0 - w), mul(hook, w));
                for k in 0..2 {
                    let node = b.strings[k];
                    let parent = model.nodes[node].parent.unwrap_or(b.frame);
                    let (hp, hq) = tracks_now.world(model, parent, t);
                    let (lp, lq) = tracks_now.local(model, node, t);
                    let head = add(hp, rotate(hq, lp));
                    let was = qmul(hq, lq);
                    let to = sub(target, head);
                    let turned = qmul(between(norm(rotate(was, [0.0, 1.0, 0.0])), norm(to)), was);
                    rots[k].push(qnorm(qmul(qinv(hq), turned)).to_vec());
                    let mut sc = tracks_now.scale(model, node, t);
                    sc[1] = len(to) / b.half;
                    scls[k].push(sc.to_vec());
                }
            }
            for k in 0..2 {
                put(a, b.strings[k], "rotation", &ts, std::mem::take(&mut rots[k]));
                put(a, b.strings[k], "scale", &ts, std::mem::take(&mut scls[k]));
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
        let text = std::fs::read_to_string(art.join(format!("champions/{who}/{who}.champion.ron"))).unwrap();
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

    /// The deepest an arm goes into the trunk through a clip (m), sampled 30 times a second.
    fn deepest(who: &str, m: &Model, clip: &str, side: usize) -> f32 {
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art");
        let text = std::fs::read_to_string(art.join(format!("champions/{who}/{who}.champion.ron"))).unwrap();
        let r = crate::recipe::Recipe::from_text(&text).unwrap();
        let worn: Vec<usize> = r.looks.outfit.iter().filter_map(|id| m.node_by_name(&format!("part.{id}"))).collect();
        let trunk = Trunk::read(m, &worn);
        let a = m.animations.iter().find(|a| a.name == clip).unwrap();
        let tracks = Tracks::new(m, a);
        let end = a.channels.iter().filter_map(|c| c.times.last().copied()).fold(0.0f32, f32::max);
        let s = ["l", "r"][side];
        let node = |n: &str| m.node_by_name(&format!("{n}_{s}")).unwrap();
        (0..=(end * 30.0) as usize)
            .map(|k| {
                let t = k as f32 / 30.0;
                let at = trunk.at(&tracks, m, t);
                let p = |n: &str| tracks.world(m, node(n), t).0;
                trunk.arm_depth(&at, side, p("upperarm"), p("forearm"), p("hand"), p("prop"))
            })
            .fold(0.0, f32::max)
    }

    #[test]
    fn the_arms_keep_out_of_the_trunk() {
        // Lumen's free hand reached into her robe and Rook's swing folded his forearm into his
        // chest (both a good 14 cm deep); the whirlwind's arm went through the shoulders on every
        // build (31 cm), Rook's fidget into his chest (21 cm). Through every clip of every
        // champion, an arm now touches the trunk, at most.
        let mut worst = Vec::new();
        for who in ["bastion", "cairn", "ember", "lumen", "marrow", "quill", "rook", "shade", "vesper", "wren"] {
            let m = forged(who);
            for a in &m.animations {
                for side in 0..2 {
                    let d = deepest(who, &m, &a.name, side);
                    if d > 0.06 {
                        worst.push(format!("{who} {} {}: {d:.3} m", a.name, ["l", "r"][side]));
                    }
                }
            }
        }
        assert!(worst.is_empty(), "arms in the trunk: {worst:#?}");
    }

    /// The deepest what a hand holds goes into the head through a clip (m): the held items'
    /// skin (what rides a prop bone), sampled 30 times a second.
    fn held_in_head(who: &str, m: &Model, clip: &str) -> f32 {
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art");
        let text = std::fs::read_to_string(art.join(format!("champions/{who}/{who}.champion.ron"))).unwrap();
        let r = crate::recipe::Recipe::from_text(&text).unwrap();
        let worn: Vec<usize> = r.looks.outfit.iter().filter_map(|id| m.node_by_name(&format!("part.{id}"))).collect();
        let trunk = Trunk::read(m, &worn);
        let props: Vec<usize> = ["prop_l", "prop_r"].iter().filter_map(|n| m.node_by_name(n)).collect();
        // The held items' points, on their prop bone at rest.
        let mut held: Vec<(usize, V)> = Vec::new();
        for &i in &worn {
            let n = &m.nodes[i];
            let (Some(mesh), Some(skin)) = (n.mesh, n.skin) else { continue };
            for prim in &m.meshes[mesh] {
                for (k, p) in prim.positions.iter().enumerate().step_by(3) {
                    let (Some(j), Some(w)) = (prim.joints.get(k), prim.weights.get(k)) else { continue };
                    let top = (0..4).max_by(|a, b| w[*a].total_cmp(&w[*b])).unwrap_or(0);
                    let Some(&node) = m.skins[skin].get(j[top] as usize) else { continue };
                    if props.contains(&node) {
                        let (t, q) = rest_world(m, node);
                        held.push((node, rotate(qinv(q), sub(*p, t))));
                    }
                }
            }
        }
        let a = m.animations.iter().find(|a| a.name == clip).unwrap();
        let tracks = Tracks::new(m, a);
        let end = a.channels.iter().filter_map(|c| c.times.last().copied()).fold(0.0f32, f32::max);
        (0..=(end * 30.0) as usize)
            .map(|k| {
                let t = k as f32 / 30.0;
                let at = trunk.at(&tracks, m, t);
                held.iter()
                    .map(|&(node, p)| {
                        let (pt, pq) = tracks.world(m, node, t);
                        trunk.head_depth(&at, add(pt, rotate(pq, p)), 0.0)
                    })
                    .fold(0.0, f32::max)
            })
            .fold(0.0, f32::max)
    }

    #[test]
    fn what_a_hand_holds_keeps_out_of_the_head() {
        // Rook's maul went through his head in his overhead windup (attack_2): a weapon may
        // brush a jaw or rest by the neck on a shoulder, no more.
        let mut worst = Vec::new();
        for who in ["bastion", "cairn", "ember", "lumen", "marrow", "quill", "rook", "shade", "vesper", "wren"] {
            let m = forged(who);
            for a in &m.animations {
                let d = held_in_head(who, &m, &a.name);
                if d > 0.07 {
                    worst.push(format!("{who} {}: {d:.3} m", a.name));
                }
            }
        }
        assert!(worst.is_empty(), "held items in the head: {worst:#?}");
    }

    #[test]
    fn the_bowstring_comes_back_to_the_fingers_that_draw_it() {
        let m = forged("vesper");
        let b = Bow::read(&m).expect("the bow's string");
        let h = Hand::read(&m, "r").unwrap();
        let a = m.animations.iter().find(|a| a.name == "attack_1").unwrap();
        let tracks = Tracks::new(&m, a);
        let end = a.channels.iter().filter_map(|c| c.times.last().copied()).fold(0.0f32, f32::max);
        let mut drawn = 0;
        for k in 0..=(end * 30.0) as usize {
            let t = k as f32 / 30.0;
            let (p, q) = tracks.world(&m, b.strings[0], t);
            let nock = add(p, rotate(q, [0.0, b.half * tracks.scale(&m, b.strings[0], t)[1], 0.0]));
            let (fp, fq) = tracks.world(&m, h.fingers[2], t);
            let hook = add(fp, rotate(fq, [0.0, h.thick * 0.5, -h.thick * 1.2]));
            let (lp, lq) = tracks.world(&m, b.frame, t);
            if len(sub(rotate(qinv(lq), sub(nock, lp)), b.rest)) > 0.15 {
                drawn += 1;
                assert!(
                    len(sub(nock, hook)) < 0.01,
                    "at {t}: the string is {} m off the fingers",
                    len(sub(nock, hook))
                );
            }
        }
        assert!(drawn > 3, "the attack never draws the bow");
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
