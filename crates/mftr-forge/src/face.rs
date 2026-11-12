//! Bone-driven faces (21, D76): a face is a small transform delta per face bone, relative to
//! the bone's rest, quantized so a recipe stays small and every client bakes the same face.
//! `bake` puts the deltas into the face bones' rest and records where the mesh is bound
//! (`Model::bind`), so the client's skin deforms the head by exactly the deltas: no vertex is
//! moved on the CPU and nothing is rebuilt per frame.
//!
//! The body's helper bones (21 §9, `crate::body`) are sculpted the same way, so everything here
//! that isn't about the face itself works on a [`BoneSet`]: the face's bones or the body's.

use mftr_pack::face::FACE_RIG;
use mftr_pack::glb::{BindTrs, Model};
use mftr_sim::data::{self, DataError, Fields, Value};

pub use mftr_pack::face::RIG_VERSION;

pub const BONES: usize = FACE_RIG.len();

/// Fixed-point units: translation 0.1 mm, rotation 0.01° (a rotation vector), scale 0.1% in
/// log2 space (so 2× and ½× are symmetric).
pub const T_UNIT: f32 = 1.0e-4;
pub const R_UNIT: f32 = 0.01;
pub const S_UNIT: f32 = 1.0e-3;

pub const DOF_T: u8 = 1;
pub const DOF_R: u8 = 2;
pub const DOF_S: u8 = 4;

/// How far each bone may go, per component, in fixed-point units, and which of T, R, S it takes
/// (21 §1.3, §2.2) *(start: tuned on the authored head)*.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub t: i16,
    pub r: i16,
    pub s: i16,
    pub dofs: u8,
}

const FEATURE: Limits = Limits { t: 250, r: 2000, s: 550, dofs: DOF_T | DOF_R | DOF_S };
const BIG: Limits = Limits { t: 400, r: 3000, s: 750, dofs: DOF_T | DOF_R | DOF_S };
const FRAME: Limits = Limits { t: 200, r: 1200, s: 330, dofs: DOF_T | DOF_R | DOF_S };

/// A set of sculpt bones: the face's (21 §1.3) or the body's helpers (21 §9.1). The limits, the
/// bake, the editor's drag and the recipe's forms work on either.
#[derive(Debug)]
pub struct BoneSet {
    /// What a recipe calls it: `face` or `body`.
    pub name: &'static str,
    /// The compact form's first two bytes.
    pub magic: [u8; 2],
    pub version: u8,
    /// Each bone and its parent, parents first.
    pub rig: &'static [(&'static str, &'static str)],
    pub limits: &'static [Limits],
    /// What each bone's region is called on screen.
    pub regions: &'static [&'static str],
    /// A size drag grows a bone across (x and z) and leaves its length (y) alone: the body's
    /// helpers lie along their limbs.
    pub size_across: bool,
}

impl BoneSet {
    pub fn index(&self, bone: &str) -> Option<usize> {
        self.rig.iter().position(|(n, _)| *n == bone)
    }

    /// Bone `i`'s parent among the set's own bones (scale stays local: 21 §1.3).
    pub fn parent(&self, i: usize) -> Option<usize> {
        self.index(self.rig[i].1)
    }

    /// The other side's bone (itself on the middle line).
    pub fn mirror(&self, b: usize) -> usize {
        let name = self.rig[b].0;
        let other = match (name.strip_suffix("_l"), name.strip_suffix("_r")) {
            (Some(base), _) => format!("{base}_r"),
            (_, Some(base)) => format!("{base}_l"),
            _ => return b,
        };
        self.index(&other).unwrap_or(b)
    }
}

/// The face's bones.
pub static FACE: BoneSet = BoneSet {
    name: "face",
    magic: *b"FS",
    version: RIG_VERSION,
    rig: &FACE_RIG,
    limits: &LIMITS,
    regions: &REGION_NAMES,
    size_across: false,
};

/// A sculpt on a set of bones: one delta per bone, in the set's order.
pub trait Sculpt: Clone + Default + PartialEq {
    fn set() -> &'static BoneSet;
    fn bones(&self) -> &[BoneDelta];
    fn bones_mut(&mut self) -> &mut [BoneDelta];

    /// Inside every bone's limits.
    fn within_limits(&self) -> Self {
        let mut out = self.clone();
        for (d, l) in out.bones_mut().iter_mut().zip(Self::set().limits) {
            *d = d.clamped(l);
        }
        out
    }
}

/// A bone's frame: its rest axes in model space, as columns.
pub type Frame = [[f32; 3]; 3];

/// Per face bone, in `FACE_RIG` order.
pub const LIMITS: [Limits; BONES] = [
    FRAME,                                     // face_forehead
    Limits { dofs: DOF_T | DOF_R, ..FEATURE }, // face_brow_mid
    FEATURE,                                   // face_brow_l
    FEATURE,                                   // face_brow_r
    FEATURE,                                   // face_eye_l
    FEATURE,                                   // face_eye_r
    FEATURE,                                   // face_cheekbone_l
    FEATURE,                                   // face_cheekbone_r
    Limits { dofs: DOF_T | DOF_S, ..FEATURE }, // face_cheek_l
    Limits { dofs: DOF_T | DOF_S, ..FEATURE }, // face_cheek_r
    Limits { dofs: DOF_T | DOF_R, ..FEATURE }, // face_nose_bridge
    BIG,                                       // face_nose_tip
    Limits { dofs: DOF_T | DOF_S, ..FEATURE }, // face_nostril_l
    Limits { dofs: DOF_T | DOF_S, ..FEATURE }, // face_nostril_r
    FEATURE,                                   // face_lip_upper
    FRAME,                                     // jaw
    FEATURE,                                   // face_lip_lower
    Limits { dofs: DOF_T | DOF_R, ..FEATURE }, // face_mouth_l
    Limits { dofs: DOF_T | DOF_R, ..FEATURE }, // face_mouth_r
    BIG,                                       // face_chin
    Limits { dofs: DOF_T | DOF_S, ..FEATURE }, // face_jaw_angle_l
    Limits { dofs: DOF_T | DOF_S, ..FEATURE }, // face_jaw_angle_r
    BIG,                                       // face_ear_l
    BIG,                                       // face_ear_r
];

/// Each face bone's parent among the face bones (scale stays local: 21 §1.3).
pub fn face_parent(i: usize) -> Option<usize> {
    FACE.parent(i)
}

/// One bone's delta from rest, in the bone's own rest frame.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct BoneDelta {
    pub t: [i16; 3],
    pub r: [i16; 3],
    pub s: [i16; 3],
}

impl BoneDelta {
    pub const ZERO: BoneDelta = BoneDelta { t: [0; 3], r: [0; 3], s: [0; 3] };

    pub fn is_zero(&self) -> bool {
        *self == Self::ZERO
    }

    pub fn translation(&self) -> [f32; 3] {
        self.t.map(|v| v as f32 * T_UNIT)
    }

    /// The rotation as a quaternion (x, y, z, w), from the rotation vector.
    pub fn rotation(&self) -> [f32; 4] {
        let v = self.r.map(|d| (d as f32 * R_UNIT).to_radians());
        let angle = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if angle < 1e-7 {
            return [0.0, 0.0, 0.0, 1.0];
        }
        let (s, c) = (angle * 0.5).sin_cos();
        let k = s / angle;
        [v[0] * k, v[1] * k, v[2] * k, c]
    }

    pub fn scale(&self) -> [f32; 3] {
        self.s.map(|v| (v as f32 * S_UNIT).exp2())
    }

    /// The left/right mirror (X flipped): translation x and rotation y, z change sign.
    pub fn mirrored(&self) -> BoneDelta {
        BoneDelta { t: [-self.t[0], self.t[1], self.t[2]], r: [self.r[0], -self.r[1], -self.r[2]], s: self.s }
    }

    /// Inside `l`: components out of range clamped, DOFs the bone doesn't take zeroed.
    pub fn clamped(&self, l: &Limits) -> BoneDelta {
        let c = |v: [i16; 3], m: i16, on: bool| if on { v.map(|x| x.clamp(-m, m)) } else { [0; 3] };
        BoneDelta {
            t: c(self.t, l.t, l.dofs & DOF_T != 0),
            r: c(self.r, l.r, l.dofs & DOF_R != 0),
            s: c(self.s, l.s, l.dofs & DOF_S != 0),
        }
    }
}

/// A face: one delta per face bone, in `FACE_RIG` order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FaceSculpt {
    pub rig: u8,
    pub bones: [BoneDelta; BONES],
}

impl Default for FaceSculpt {
    fn default() -> Self {
        FaceSculpt { rig: RIG_VERSION, bones: [BoneDelta::ZERO; BONES] }
    }
}

impl FaceSculpt {
    pub fn is_neutral(&self) -> bool {
        self.bones.iter().all(BoneDelta::is_zero)
    }

    pub fn clamped(&self) -> FaceSculpt {
        self.within_limits()
    }
}

impl Sculpt for FaceSculpt {
    fn set() -> &'static BoneSet {
        &FACE
    }

    fn bones(&self) -> &[BoneDelta] {
        &self.bones
    }

    fn bones_mut(&mut self) -> &mut [BoneDelta] {
        &mut self.bones
    }
}

fn qmul(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn qrot(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let p = qmul(qmul(q, [v[0], v[1], v[2], 0.0]), [-q[0], -q[1], -q[2], q[3]]);
    [p[0], p[1], p[2]]
}

/// Bake a sculpt (a face, a body) into a forged model (21 §2.3): each of its bones' rest becomes
/// rest · delta, a bone whose parent in the set was scaled has that scale divided back out
/// (scale stays local), and `Model::bind` keeps every moved bone bound where it was, so the skin
/// deforms the mesh by exactly the deltas. Parents precede children in a set, so one pass
/// suffices.
pub fn bake<S: Sculpt>(model: &mut Model, sculpt: &S) {
    let set = S::set();
    let sculpt = sculpt.within_limits();
    let bones = sculpt.bones();
    if bones.iter().all(BoneDelta::is_zero) {
        return;
    }
    if model.bind.len() < model.nodes.len() {
        model.bind.resize(model.nodes.len(), None);
    }
    for (i, d) in bones.iter().enumerate() {
        let Some(node) = model.node_by_name(set.rig[i].0) else { continue };
        let parent = set.parent(i);
        let parent_scaled = parent.is_some_and(|p| !bones[p].s.iter().all(|&v| v == 0));
        if d.is_zero() && !parent_scaled {
            continue;
        }
        let n = &mut model.nodes[node];
        if model.bind[node].is_none() {
            model.bind[node] = Some((n.translation, n.rotation, n.scale));
        }
        let parent_scale = parent.map_or([1.0; 3], |p| bones[p].scale());
        (n.translation, n.rotation, n.scale) = baked_bone((n.translation, n.rotation, n.scale), d, parent_scale);
    }
}

/// One sculpt bone's local transform with its delta: `rest · delta`, the translation turned from
/// the bone's frame into its parent's, and a scaled sculpt-bone parent's scale divided back out
/// (scale stays local). The bake and the editor's live pose both use it.
pub fn baked_bone(rest: BindTrs, d: &BoneDelta, parent_scale: [f32; 3]) -> BindTrs {
    let (mut t, r, mut s) = rest;
    // Exact for uniform parent scale; along the parent's axes otherwise (the limits keep it
    // near-uniform).
    t = [0, 1, 2].map(|k| t[k] / parent_scale[k]);
    s = [0, 1, 2].map(|k| s[k] / parent_scale[k]);
    let dt = qrot(r, d.translation());
    let ds = d.scale();
    ([0, 1, 2].map(|k| t[k] + dt[k]), qmul(r, d.rotation()), [0, 1, 2].map(|k| s[k] * ds[k]))
}

// --- The face sliders as presets on the bones (21 §2.7, FR3) ----------------------------------

const PRESETS_SOURCE: &str = include_str!("../../../art/creator/heads/presets.txt");

/// A slider as a preset on the bones: the bone moves of its +1 end (its -1 end the opposite,
/// unless one-sided).
#[derive(Clone, Debug)]
pub struct Preset<S = FaceSculpt> {
    pub id: String,
    pub about: String,
    pub one_sided: bool,
    pub sculpt: S,
}

/// Every face slider, in the file's order (`art/creator/heads/presets.txt`).
pub fn presets() -> &'static [Preset] {
    static ALL: std::sync::OnceLock<Vec<Preset>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| read_presets(PRESETS_SOURCE).unwrap_or_else(|e| panic!("art/creator/heads/presets.txt: {e}")))
}

pub fn preset(id: &str) -> Option<&'static Preset> {
    presets().iter().find(|p| p.id == id)
}

/// Read a presets file (`[id] about`, then `bone t|r|s x y z [mirror]` lines) for the bones of
/// `S`. Strict: an unknown bone, a move past a bone's limits or degrees of freedom, or the same
/// move twice is an error.
pub(crate) fn read_presets<S: Sculpt>(text: &str) -> Result<Vec<Preset<S>>, String> {
    let set = S::set();
    let mut out: Vec<Preset<S>> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let at = format!("line {}", n + 1);
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            let (head, about) = rest.split_once(']').ok_or(format!("{at}: `[id] description`"))?;
            let (id, one_sided) = match head.strip_suffix('+') {
                Some(id) => (id, true),
                None => (head, false),
            };
            if out.iter().any(|p| p.id == id) {
                return Err(format!("{at}: `{id}` twice"));
            }
            out.push(Preset { id: id.into(), about: about.trim().into(), one_sided, sculpt: S::default() });
            continue;
        }
        let p = out.last_mut().ok_or(format!("{at}: a bone before any `[id]`"))?;
        let words: Vec<&str> = line.split_whitespace().collect();
        if !(5..=6).contains(&words.len()) || (words.len() == 6 && words[5] != "mirror") {
            return Err(format!("{at}: `bone t|r|s x y z [mirror]`"));
        }
        let i = set.index(words[0]).ok_or(format!("{at}: `{}` isn't a {} bone", words[0], set.name))?;
        let v: [i16; 3] = [2, 3, 4].map(|k| words[k].parse::<i16>().unwrap_or(i16::MIN));
        if v.contains(&i16::MIN) {
            return Err(format!("{at}: x y z are whole numbers"));
        }
        let mut put = |i: usize, d: BoneDelta| -> Result<(), String> {
            let b = &mut p.sculpt.bones_mut()[i];
            let slot = match words[1] {
                "t" => (&mut b.t, d.t),
                "r" => (&mut b.r, d.r),
                "s" => (&mut b.s, d.s),
                k => return Err(format!("{at}: kind `{k}` (t, r or s)")),
            };
            if *slot.0 != [0; 3] {
                return Err(format!("{at}: `{}` {} twice", set.rig[i].0, words[1]));
            }
            *slot.0 = slot.1;
            if b.clamped(&set.limits[i]) != *b {
                return Err(format!("{at}: outside `{}`'s limits or degrees of freedom", set.rig[i].0));
            }
            Ok(())
        };
        let d = BoneDelta { t: v, r: v, s: v };
        put(i, d)?;
        if words.len() == 6 {
            let name = words[0].strip_suffix("_l").ok_or(format!("{at}: `mirror` goes on a `_l` bone"))?;
            let j = set.index(&format!("{name}_r")).ok_or(format!("{at}: no `{name}_r`"))?;
            put(j, d.mirrored())?;
        }
    }
    Ok(out)
}

/// The face a recipe gets: its own `face` plus each face slider in `shape` times its preset,
/// summed and clamped. None when neither moves anything.
pub fn compose(face: Option<&FaceSculpt>, shape: &[(String, f32)]) -> Option<FaceSculpt> {
    compose_with(face, shape, presets())
}

/// A sculpt plus each slider in `shape` that `presets` knows times its preset, summed, rounded
/// and clamped to the bones' limits. None when nothing moves.
pub fn compose_with<S: Sculpt>(own: Option<&S>, shape: &[(String, f32)], presets: &[Preset<S>]) -> Option<S> {
    let mut acc = vec![[0f32; 9]; S::set().rig.len()];
    let mut add = |s: &S, k: f32| {
        for (a, d) in acc.iter_mut().zip(s.bones()) {
            for (i, v) in d.t.iter().chain(&d.r).chain(&d.s).enumerate() {
                a[i] += *v as f32 * k;
            }
        }
    };
    let mut any = false;
    if let Some(s) = own {
        add(s, 1.0);
        any = !s.bones().iter().all(BoneDelta::is_zero);
    }
    for (id, value) in shape {
        let Some(p) = presets.iter().find(|p| p.id == *id) else { continue };
        let v = if p.one_sided { value.max(0.0) } else { *value };
        if v == 0.0 {
            continue;
        }
        any = true;
        add(&p.sculpt, v);
    }
    if !any {
        return None;
    }
    let mut out = S::default();
    for (d, a) in out.bones_mut().iter_mut().zip(&acc) {
        let q = |k: usize| a[k].round().clamp(i16::MIN as f32, i16::MAX as f32) as i16;
        *d = BoneDelta { t: [q(0), q(1), q(2)], r: [q(3), q(4), q(5)], s: [q(6), q(7), q(8)] };
    }
    Some(out.within_limits())
}

// --- The editor (FR4, 21 §2.4–2.5) ------------------------------------------------------------

/// What each face bone's region is called on screen (21 §2.4): a region is the skin a face bone
/// owns, so the regions and the bones share their order. "Left" is the character's left.
pub const REGION_NAMES: [&str; BONES] = [
    "Forehead",
    "Brow (middle)",
    "Left brow",
    "Right brow",
    "Left eye",
    "Right eye",
    "Left cheekbone",
    "Right cheekbone",
    "Left cheek",
    "Right cheek",
    "Bridge of the nose",
    "Nose tip",
    "Left nostril",
    "Right nostril",
    "Upper lip",
    "Jaw",
    "Lower lip",
    "Left mouth corner",
    "Right mouth corner",
    "Chin",
    "Left jaw angle",
    "Right jaw angle",
    "Left ear",
    "Right ear",
];

/// A vertex belongs to the face bone that weighs most on it, when that is at least this much
/// (less is the cranium, the neck, the back of the head: not editable).
pub const REGION_MIN: f32 = 0.12;

/// The other side's face bone (itself on the middle line).
pub fn mirror_of(b: usize) -> usize {
    FACE.mirror(b)
}

/// Each body vertex's region, per mesh, per primitive: 0 for none, `r + 1` for region `r`: a
/// face bone's index, or `BODY_REGIONS` plus a body helper's (21 §2.4, §9; from the weights: no
/// exporter change). A vertex goes to the bone whose weight on it is the greatest share of that
/// bone's own peak: the eyes carry their lids whole, and by raw weight they would take the brows
/// too. Below `REGION_MIN` (the cranium, the hands, the joints) it has none; parts have none:
/// they're not the face or the body.
pub fn vertex_regions(model: &Model) -> Vec<Vec<Vec<u8>>> {
    let sets: [&BoneSet; 2] = [&FACE, &crate::body::BODY];
    let total: usize = sets.iter().map(|s| s.rig.len()).sum();
    let joints = model.skins.first().cloned().unwrap_or_default();
    let joint_region: Vec<Option<usize>> = joints
        .iter()
        .map(|&n| {
            let mut base = 0;
            for set in sets {
                if let Some(b) = set.index(&model.nodes[n].name) {
                    return Some(base + b);
                }
                base += set.rig.len();
            }
            None
        })
        .collect();
    let body: Vec<bool> = (0..model.meshes.len())
        .map(|mi| model.nodes.iter().any(|n| n.mesh == Some(mi) && !n.name.starts_with("part.")))
        .collect();
    // A vertex's (region, weight) per influence; `usize::MAX` where the joint has no region.
    let weights_of = |j: &[u32; 4], w: &[f32; 4]| {
        [0, 1, 2, 3].map(|k| (joint_region.get(j[k] as usize).copied().flatten().unwrap_or(usize::MAX), w[k]))
    };
    let usable = |mi: usize, p: &mftr_pack::glb::Primitive| {
        body[mi] && p.joints.len() == p.positions.len() && p.weights.len() == p.positions.len()
    };
    let mut peak = vec![0.0f32; total];
    for (mi, prims) in model.meshes.iter().enumerate() {
        for p in prims.iter().filter(|p| usable(mi, p)) {
            for (j, w) in p.joints.iter().zip(&p.weights) {
                for (r, x) in weights_of(j, w) {
                    if r < total {
                        peak[r] = peak[r].max(x);
                    }
                }
            }
        }
    }
    model
        .meshes
        .iter()
        .enumerate()
        .map(|(mi, prims)| {
            prims
                .iter()
                .map(|p| {
                    if !usable(mi, p) {
                        return vec![0; p.positions.len()];
                    }
                    p.joints
                        .iter()
                        .zip(&p.weights)
                        .map(|(j, w)| {
                            weights_of(j, w)
                                .into_iter()
                                .filter(|&(r, x)| r < total && x >= REGION_MIN)
                                .max_by(|a, b| (a.1 / peak[a.0]).total_cmp(&(b.1 / peak[b.0])))
                                .map_or(0, |(r, _)| r as u8 + 1)
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

/// Where the body's regions start among `vertex_regions`' indices: after the face's.
pub const BODY_REGIONS: usize = BONES;

/// The region (a face bone's, or `BODY_REGIONS` plus a body helper's) the ray `origin + t·dir`
/// (model space) meets first on the body, and the point:
/// the region most of the hit triangle's vertices share, else its nearest vertex's (21 §2.4).
/// `model` is the preview as shown (skinned), `regions` its `vertex_regions`. Parts in front
/// of the face hide it (a helmet's brim), and the cranium has none.
pub fn region_at(
    model: &Model,
    regions: &[Vec<Vec<u8>>],
    origin: [f32; 3],
    dir: [f32; 3],
) -> Option<(usize, [f32; 3])> {
    let mut best: Option<(f32, Option<usize>, [f32; 3])> = None;
    for (mi, prims) in model.meshes.iter().enumerate() {
        for (pi, p) in prims.iter().enumerate() {
            let reg = regions.get(mi).and_then(|m| m.get(pi));
            for t in p.indices.chunks(3) {
                let v = [t[0] as usize, t[1] as usize, t[2] as usize];
                let (a, b, c) = (p.positions[v[0]], p.positions[v[1]], p.positions[v[2]]);
                let Some((dist, u, w)) = ray_triangle(origin, dir, a, b, c) else { continue };
                if best.as_ref().is_some_and(|(d, _, _)| *d <= dist) {
                    continue;
                }
                let region = reg.and_then(|r| {
                    let ids = v.map(|i| r.get(i).copied().unwrap_or(0));
                    let bary = [1.0 - u - w, u, w];
                    let pick = if ids[0] == ids[1] || ids[0] == ids[2] {
                        ids[0]
                    } else if ids[1] == ids[2] {
                        ids[1]
                    } else {
                        ids[(0..3).max_by(|&x, &y| bary[x].total_cmp(&bary[y])).unwrap_or(0)]
                    };
                    (pick > 0).then(|| pick as usize - 1)
                });
                let point = [0, 1, 2].map(|k| origin[k] + dir[k] * dist);
                best = Some((dist, region, point));
            }
        }
    }
    let (_, region, point) = best?;
    region.map(|r| (r, point))
}

/// Möller–Trumbore: the distance along the ray and the hit's barycentric (u, w), front or back.
fn ray_triangle(o: [f32; 3], d: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> Option<(f32, f32, f32)> {
    let sub = |x: [f32; 3], y: [f32; 3]| [x[0] - y[0], x[1] - y[1], x[2] - y[2]];
    let cross =
        |x: [f32; 3], y: [f32; 3]| [x[1] * y[2] - x[2] * y[1], x[2] * y[0] - x[0] * y[2], x[0] * y[1] - x[1] * y[0]];
    let dot = |x: [f32; 3], y: [f32; 3]| x[0] * y[0] + x[1] * y[1] + x[2] * y[2];
    let (e1, e2) = (sub(b, a), sub(c, a));
    let h = cross(d, e2);
    let det = dot(e1, h);
    if det.abs() < 1e-12 {
        return None;
    }
    let s = sub(o, a);
    let u = dot(s, h) / det;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let w = dot(d, q) / det;
    if w < 0.0 || u + w > 1.0 {
        return None;
    }
    let dist = dot(e2, q) / det;
    (dist > 0.0).then_some((dist, u, w))
}

/// Where each bone of a sculpt's set sits on the forged `model` with `sculpt` instead of the one
/// it was forged with: (node, local transform), from where the mesh is bound (`Model::bind`, else the rest).
/// The editor poses the preview's skeleton with it while a drag lasts (21 §3.3), so nothing is
/// forged until the drag ends.
pub fn posed<S: Sculpt>(model: &Model, sculpt: &S) -> Vec<(usize, BindTrs)> {
    let set = S::set();
    let sculpt = sculpt.within_limits();
    let bones = sculpt.bones();
    (0..bones.len())
        .filter_map(|b| {
            let node = model.node_by_name(set.rig[b].0)?;
            let n = &model.nodes[node];
            let rest = model.bind.get(node).copied().flatten().unwrap_or((n.translation, n.rotation, n.scale));
            let parent_scale = set.parent(b).map_or([1.0; 3], |p| bones[p].scale());
            Some((node, baked_bone(rest, &bones[b], parent_scale)))
        })
        .collect()
}

/// Each bone of `set`'s frame for its delta, in model space: the columns are its rest axes (as
/// bound, before any sculpt) — what a world drag is projected onto (21 §2.5).
pub fn bone_frames(model: &Model, set: &BoneSet) -> Vec<Frame> {
    let n = model.nodes.len();
    let mut rot: Vec<Option<[f32; 4]>> = vec![None; n];
    fn solve(m: &Model, i: usize, rot: &mut Vec<Option<[f32; 4]>>) -> [f32; 4] {
        if let Some(q) = rot[i] {
            return q;
        }
        let local = m.bind.get(i).copied().flatten().map_or(m.nodes[i].rotation, |b| b.1);
        let q = match m.nodes[i].parent {
            Some(p) => qmul(solve(m, p, rot), local),
            None => local,
        };
        rot[i] = Some(q);
        q
    }
    let mut out = vec![[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]; set.rig.len()];
    for (b, frame) in out.iter_mut().enumerate() {
        if let Some(node) = model.node_by_name(set.rig[b].0) {
            let q = solve(model, node, &mut rot);
            *frame = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|e| qrot(q, e));
        }
    }
    out
}

/// What the creator's camera knows, enough to turn pixels into metres (model space).
#[derive(Clone, Copy, Debug)]
pub struct CameraFrame {
    pub origin: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub forward: [f32; 3],
    /// Vertical field of view, radians.
    pub fov_y: f32,
    pub viewport_h: f32,
}

/// The displacement under a mouse drag of `drag_px` (x right, y down) on the plane through
/// `anchor` facing the camera: the feature moves with the cursor.
pub fn drag_to_world(cam: &CameraFrame, drag_px: [f32; 2], anchor: [f32; 3]) -> [f32; 3] {
    let rel = [0, 1, 2].map(|k| anchor[k] - cam.origin[k]);
    let depth = (rel[0] * cam.forward[0] + rel[1] * cam.forward[1] + rel[2] * cam.forward[2]).max(0.05);
    let per_px = 2.0 * depth * (cam.fov_y * 0.5).tan() / cam.viewport_h.max(1.0);
    [0, 1, 2].map(|k| (cam.right[k] * drag_px[0] - cam.up[k] * drag_px[1]) * per_px)
}

/// What a drag edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DragMode {
    /// Move the feature in the camera's plane.
    Move,
    /// Turn it: a horizontal drag about the camera's up, a vertical one about its right.
    Turn,
    /// Grow or shrink it with a vertical drag (all three axes, or only across).
    Size { width_only: bool },
}

/// One bone's delta after a drag: `world` is `drag_to_world`'s displacement, `frame` the bone's
/// (`bone_frames`). Clamped to the bone's limits and degrees of freedom. With `across`, a size
/// drag leaves the bone's length (y) alone (`BoneSet::size_across`).
#[allow(clippy::too_many_arguments)]
pub fn apply_drag(
    d: BoneDelta,
    mode: DragMode,
    world: [f32; 3],
    drag_px: [f32; 2],
    cam: &CameraFrame,
    frame: &Frame,
    limits: &Limits,
    across: bool,
) -> BoneDelta {
    let to_local = |v: [f32; 3]| frame.map(|axis| axis[0] * v[0] + axis[1] * v[1] + axis[2] * v[2]);
    let add =
        |a: [i16; 3], v: [f32; 3], unit: f32| [0, 1, 2].map(|k| a[k].saturating_add((v[k] / unit).round() as i16));
    let mut out = d;
    match mode {
        DragMode::Move => out.t = add(d.t, to_local(world), T_UNIT),
        DragMode::Turn => {
            // 0.4° a pixel.
            let axis = [0, 1, 2].map(|k| (cam.up[k] * drag_px[0] + cam.right[k] * drag_px[1]) * 0.4);
            out.r = add(d.r, to_local(axis), R_UNIT);
        }
        DragMode::Size { width_only } => {
            // 0.5% a pixel up.
            let step = (-drag_px[1] * 0.005 / S_UNIT).round() as i16;
            out.s = if width_only {
                [d.s[0].saturating_add(step), d.s[1], d.s[2]]
            } else if across {
                [d.s[0].saturating_add(step), d.s[1], d.s[2].saturating_add(step)]
            } else {
                d.s.map(|v| v.saturating_add(step))
            };
        }
    }
    out.clamped(limits)
}

/// Apply a drag to bone `b` of a sculpt (a face, a body), with the other side following when
/// `symmetry` is on (the same change, mirrored; on the middle line, the change keeps to the
/// middle). Returns whether anything moved.
#[allow(clippy::too_many_arguments)]
pub fn drag<S: Sculpt>(
    sculpt: &mut S,
    b: usize,
    mode: DragMode,
    world: [f32; 3],
    drag_px: [f32; 2],
    cam: &CameraFrame,
    frames: &[Frame],
    symmetry: bool,
) -> bool {
    let set = S::set();
    let limits = set.limits;
    let bones = sculpt.bones_mut();
    let old = bones[b];
    let next = apply_drag(old, mode, world, drag_px, cam, &frames[b], &limits[b], set.size_across);
    let diff = |a: [i16; 3], o: [i16; 3]| [0, 1, 2].map(|k| a[k] - o[k]);
    let mut inc = BoneDelta { t: diff(next.t, old.t), r: diff(next.r, old.r), s: diff(next.s, old.s) };
    let m = set.mirror(b);
    if symmetry && m == b {
        inc.t[0] = 0;
        inc.r[1] = 0;
        inc.r[2] = 0;
    }
    if inc.is_zero() {
        return false;
    }
    let plus = |a: BoneDelta, i: BoneDelta, l: &Limits| {
        let sum = |x: [i16; 3], y: [i16; 3]| [0, 1, 2].map(|k| x[k].saturating_add(y[k]));
        BoneDelta { t: sum(a.t, i.t), r: sum(a.r, i.r), s: sum(a.s, i.s) }.clamped(l)
    };
    bones[b] = plus(old, inc, &limits[b]);
    if symmetry && m != b {
        bones[m] = plus(bones[m], inc.mirrored(), &limits[m]);
    }
    true
}

pub use self::drag as drag_face;

// --- Face shapes (21 §7, FR5) -----------------------------------------------------------------

const SHAPES_SOURCE: &str = include_str!("../../../art/creator/heads/faces.txt");

/// A whole face to start from: a value for some of the face sliders (the rest 0).
#[derive(Clone, Debug)]
pub struct Shape {
    pub id: String,
    pub name: String,
    pub about: String,
    pub sliders: Vec<(String, f32)>,
}

/// Every face shape, in the file's order (`art/creator/heads/faces.txt`).
pub fn shapes() -> &'static [Shape] {
    static ALL: std::sync::OnceLock<Vec<Shape>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| read_shapes(SHAPES_SOURCE).unwrap_or_else(|e| panic!("art/creator/heads/faces.txt: {e}")))
}

pub fn shape(id: &str) -> Option<&'static Shape> {
    shapes().iter().find(|s| s.id == id)
}

fn read_shapes(text: &str) -> Result<Vec<Shape>, String> {
    let mut out: Vec<Shape> = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let at = format!("line {}", n + 1);
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix('[') {
            let (id, title) = rest.split_once(']').ok_or(format!("{at}: `[id] Name: about`"))?;
            let (name, about) = title.split_once(':').ok_or(format!("{at}: `[id] Name: about`"))?;
            if out.iter().any(|s| s.id == id) {
                return Err(format!("{at}: `{id}` twice"));
            }
            out.push(Shape {
                id: id.into(),
                name: name.trim().into(),
                about: about.trim().into(),
                sliders: Vec::new(),
            });
            continue;
        }
        let s = out.last_mut().ok_or(format!("{at}: a slider before any `[id]`"))?;
        let words: Vec<&str> = line.split_whitespace().collect();
        let [slider, value] = words[..] else { return Err(format!("{at}: `slider value`")) };
        if preset(slider).is_none() {
            return Err(format!("{at}: `{slider}` isn't a face slider"));
        }
        let v: f32 = value.parse().map_err(|_| format!("{at}: `{value}` isn't a number"))?;
        if !(-1.0..=1.0).contains(&v) || s.sliders.iter().any(|(k, _)| k == slider) {
            return Err(format!("{at}: `{slider}` from -1 to 1, once"));
        }
        s.sliders.push((slider.into(), v));
    }
    Ok(out)
}

// --- Skinning on the CPU (tests now; the editor's hover raycast in FR4, 21 §4) ----------------

/// A node's global affine transform: a 3×3 linear part (columns) and a translation.
#[derive(Clone, Copy, Debug)]
struct Affine {
    m: [[f32; 3]; 3],
    t: [f32; 3],
}

impl Affine {
    fn of(t: [f32; 3], r: [f32; 4], s: [f32; 3]) -> Affine {
        let cols = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]].map(|e| qrot(r, e));
        Affine { m: [0, 1, 2].map(|c| cols[c].map(|x| x * s[c])), t }
    }

    /// The linear part times `v` (columns weighted by v's components).
    fn linear(&self, v: [f32; 3]) -> [f32; 3] {
        [0, 1, 2].map(|k| self.m[0][k] * v[0] + self.m[1][k] * v[1] + self.m[2][k] * v[2])
    }

    fn apply(&self, p: [f32; 3]) -> [f32; 3] {
        let l = self.linear(p);
        [0, 1, 2].map(|k| l[k] + self.t[k])
    }

    fn then(&self, child: &Affine) -> Affine {
        let m = child.m.map(|col| self.linear(col));
        Affine { m, t: self.apply(child.t) }
    }

    fn inverse(&self) -> Affine {
        let [a, b, c] = self.m;
        // Columns a, b, c: the inverse's rows are the cross products over the determinant.
        let cross = |u: [f32; 3], v: [f32; 3]| {
            [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]]
        };
        let det = a[0] * (b[1] * c[2] - b[2] * c[1]) - b[0] * (a[1] * c[2] - a[2] * c[1])
            + c[0] * (a[1] * b[2] - a[2] * b[1]);
        let rows = [cross(b, c), cross(c, a), cross(a, b)].map(|r| r.map(|x| x / det));
        let m = [0, 1, 2].map(|col| [rows[0][col], rows[1][col], rows[2][col]]);
        let inv = Affine { m, t: [0.0; 3] };
        let t = inv.apply(self.t);
        Affine { m, t: t.map(|x| -x) }
    }
}

/// Every vertex where the skin puts it at rest: each bone's rest (face sculpts included) over
/// where the mesh is bound (`Model::bind`). Per mesh, per primitive, per vertex.
pub fn skinned(model: &Model) -> Vec<Vec<Vec<[f32; 3]>>> {
    let n = model.nodes.len();
    let mut rest: Vec<Option<Affine>> = vec![None; n];
    let mut bind: Vec<Option<Affine>> = vec![None; n];
    fn global(model: &Model, i: usize, out: &mut Vec<Option<Affine>>, bound: bool) -> Affine {
        if let Some(a) = out[i] {
            return a;
        }
        let node = &model.nodes[i];
        let (t, r, s) = match (bound, model.bind.get(i).copied().flatten()) {
            (true, Some(b)) => b,
            _ => (node.translation, node.rotation, node.scale),
        };
        let local = Affine::of(t, r, s);
        let g = match node.parent {
            Some(p) => global(model, p, out, bound).then(&local),
            None => local,
        };
        out[i] = Some(g);
        g
    }
    let Some(joints) = model.skins.first() else {
        return model.meshes.iter().map(|ps| ps.iter().map(|p| p.positions.clone()).collect()).collect();
    };
    let skin: Vec<Affine> = joints
        .iter()
        .map(|&j| global(model, j, &mut rest, false).then(&global(model, j, &mut bind, true).inverse()))
        .collect();
    model
        .meshes
        .iter()
        .map(|prims| {
            prims
                .iter()
                .map(|p| {
                    p.positions
                        .iter()
                        .enumerate()
                        .map(|(v, &pos)| {
                            let (Some(js), Some(ws)) = (p.joints.get(v), p.weights.get(v)) else { return pos };
                            let mut acc = [0.0f32; 3];
                            let mut total = 0.0;
                            for k in 0..4 {
                                let (Some(m), w) = (skin.get(js[k] as usize), ws[k]) else { continue };
                                if w <= 0.0 {
                                    continue;
                                }
                                let q = m.apply(pos);
                                for c in 0..3 {
                                    acc[c] += q[c] * w;
                                }
                                total += w;
                            }
                            if total > 0.0 { acc.map(|x| x / total) } else { pos }
                        })
                        .collect()
                })
                .collect()
        })
        .collect()
}

// --- The recipe's form (RON, 21 §2.6) ---------------------------------------------------------

fn err<T>(at: &str, msg: impl Into<String>) -> Result<T, DataError> {
    Err(DataError { at: at.to_string(), msg: msg.into() })
}

/// Read `face: (rig: 1, bones: (face_nose_tip: (t: (0, -120, -40), s: (80, 80, 80)), …))`.
/// Strict: unknown bones, a rig this build doesn't know, values out of range and DOFs a bone
/// doesn't take are errors (clamping would let two clients disagree).
pub fn read(v: Value, at: &str) -> Result<FaceSculpt, DataError> {
    read_sculpt(v, at)
}

/// `read` for any set's sculpt (the recipe's `face:` or `body:`).
pub fn read_sculpt<S: Sculpt>(v: Value, at: &str) -> Result<S, DataError> {
    let set = S::set();
    let mut f = Fields::of(v, at, None)?;
    let (r, rat) = f.req("rig")?;
    let rig = data::int(r, &rat, 255)? as u8;
    if rig != set.version {
        return err(&rat, format!("{} rig {rig} isn't supported (this build reads {})", set.name, set.version));
    }
    let mut out = S::default();
    if let Some((b, bat)) = f.take("bones") {
        let Value::Fields(None, items) = b else {
            return err(&bat, "expected `(bone: (t: …, r: …, s: …), …)`");
        };
        for (name, v) in items {
            let bat = format!("{bat}.{name}");
            let Some(i) = set.index(&name) else {
                return err(&bat, format!("`{name}` isn't a {} bone", set.name));
            };
            let mut g = Fields::of(v, &bat, None)?;
            let l = set.limits[i];
            let mut comp = |key: &str, max: i16, dof: u8| -> Result<[i16; 3], DataError> {
                let Some((v, cat)) = g.take(key) else { return Ok([0; 3]) };
                if l.dofs & dof == 0 {
                    return err(&cat, format!("`{name}` doesn't take `{key}`"));
                }
                let Value::List(None, xs) = v else { return err(&cat, "expected `(x, y, z)`") };
                if xs.len() != 3 {
                    return err(&cat, "expected `(x, y, z)`");
                }
                let mut out = [0i16; 3];
                for (k, x) in xs.into_iter().enumerate() {
                    let Value::Num(s) = x else { return err(&cat, "expected whole numbers") };
                    let n: i32 = s.parse().or_else(|_| err(&cat, format!("`{s}` isn't a whole number")))?;
                    if n.abs() > max as i32 {
                        return err(&cat, format!("{n} is outside ±{max}"));
                    }
                    out[k] = n as i16;
                }
                Ok(out)
            };
            let t = comp("t", l.t, DOF_T)?;
            let r = comp("r", l.r, DOF_R)?;
            let s = comp("s", l.s, DOF_S)?;
            g.done()?;
            out.bones_mut()[i] = BoneDelta { t, r, s };
        }
    }
    f.done()?;
    Ok(out)
}

/// The recipe's text for a sculpt (only the bones that moved), as `read_sculpt` reads it.
pub fn write<S: Sculpt>(sculpt: &S) -> String {
    let set = S::set();
    let mut bones = Vec::new();
    for (i, d) in sculpt.bones().iter().enumerate() {
        if d.is_zero() {
            continue;
        }
        let mut parts = Vec::new();
        for (key, v) in [("t", d.t), ("r", d.r), ("s", d.s)] {
            if v != [0; 3] {
                parts.push(format!("{key}: ({}, {}, {})", v[0], v[1], v[2]));
            }
        }
        bones.push(format!("{}: ({})", set.rig[i].0, parts.join(", ")));
    }
    format!("(rig: {}, bones: ({}))", set.version, bones.join(", "))
}

// --- The compact form (sync, hashing, undo, 21 §2.6) ------------------------------------------

/// The set's two letters (`FS` a face, `BS` a body) + version + rig, a 32-bit mask of the bones
/// present, then per present bone a 9-bit mask of its non-zero components and those components
/// as little-endian i16. At most 8 + 24 × (2 + 18) = 488 bytes.
pub fn to_bytes<S: Sculpt>(sculpt: &S) -> Vec<u8> {
    let set = S::set();
    let mut out = vec![set.magic[0], set.magic[1], 1, set.version];
    let mut mask = 0u32;
    for (i, b) in sculpt.bones().iter().enumerate() {
        if !b.is_zero() {
            mask |= 1 << i;
        }
    }
    out.extend_from_slice(&mask.to_le_bytes());
    for b in sculpt.bones().iter().filter(|b| !b.is_zero()) {
        let comps = [b.t[0], b.t[1], b.t[2], b.r[0], b.r[1], b.r[2], b.s[0], b.s[1], b.s[2]];
        let cmask = comps.iter().enumerate().fold(0u16, |m, (k, v)| if *v != 0 { m | 1 << k } else { m });
        out.extend_from_slice(&cmask.to_le_bytes());
        for v in comps.iter().filter(|v| **v != 0) {
            out.extend_from_slice(&v.to_le_bytes());
        }
    }
    out
}

pub fn from_bytes(bytes: &[u8]) -> Result<FaceSculpt, String> {
    sculpt_from_bytes(bytes)
}

/// `from_bytes` for any set's sculpt.
pub fn sculpt_from_bytes<S: Sculpt>(bytes: &[u8]) -> Result<S, String> {
    let set = S::set();
    let name = set.name;
    let get = |at: usize, n: usize| bytes.get(at..at + n).ok_or_else(|| format!("{name}: truncated"));
    if get(0, 3)? != [set.magic[0], set.magic[1], 1] {
        return Err(format!("{name}: not a version-1 {name}"));
    }
    let rig = get(3, 1)?[0];
    if rig != set.version {
        return Err(format!("{name}: rig {rig} isn't supported"));
    }
    let short = |_| format!("{name}: truncated");
    let mask = u32::from_le_bytes(get(4, 4)?.try_into().map_err(short)?);
    let n = set.rig.len();
    if n < 32 && mask >> n != 0 {
        return Err(format!("{name}: unknown bones"));
    }
    let mut out = S::default();
    let mut at = 8;
    for i in 0..n {
        if mask & (1 << i) == 0 {
            continue;
        }
        let cmask = u16::from_le_bytes(get(at, 2)?.try_into().map_err(short)?);
        at += 2;
        if cmask >> 9 != 0 || cmask == 0 {
            return Err(format!("{name}: bad component mask"));
        }
        let mut c = [0i16; 9];
        for (k, x) in c.iter_mut().enumerate() {
            if cmask & (1 << k) != 0 {
                *x = i16::from_le_bytes(get(at, 2)?.try_into().map_err(short)?);
                at += 2;
            }
        }
        out.bones_mut()[i] = BoneDelta { t: [c[0], c[1], c[2]], r: [c[3], c[4], c[5]], s: [c[6], c[7], c[8]] };
    }
    if at != bytes.len() {
        return Err(format!("{name}: trailing bytes"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> FaceSculpt {
        let mut f = FaceSculpt::default();
        f.bones[11] = BoneDelta { t: [0, -120, -40], r: [0, 0, 0], s: [80, 80, 80] }; // nose tip
        f.bones[2] = BoneDelta { t: [0, 0, -35], r: [0, 0, 600], s: [0; 3] }; // brow l
        f.bones[3] = f.bones[2].mirrored();
        f.bones[19] = BoneDelta { t: [0, -90, 0], ..BoneDelta::ZERO }; // chin
        f
    }

    #[test]
    fn faces_round_trip_through_text_and_bytes() {
        let f = sample();
        let text = write(&f);
        let back = read(data::parse(&text).unwrap(), "face").unwrap();
        assert_eq!(back, f, "{text}");
        let bytes = to_bytes(&f);
        assert!(bytes.len() <= 8 + BONES * 20);
        assert_eq!(from_bytes(&bytes).unwrap(), f);
        assert_eq!(from_bytes(&to_bytes(&FaceSculpt::default())).unwrap(), FaceSculpt::default());
        assert!(from_bytes(&bytes[..bytes.len() - 1]).is_err());
        let m = f.bones[2].mirrored();
        assert_eq!(m.mirrored(), f.bones[2]);
    }

    #[test]
    fn the_presets_are_symmetric_and_within_limits() {
        let all = presets();
        assert_eq!(all.len(), 28);
        for p in all {
            assert!(!p.sculpt.is_neutral(), "{}", p.id);
            assert!(!p.about.is_empty(), "{}", p.id);
            assert_eq!(p.sculpt.clamped(), p.sculpt, "{}", p.id);
            // Every face slider moves both sides alike.
            for (i, (name, _)) in FACE_RIG.iter().enumerate() {
                if let Some(side) = name.strip_suffix("_l") {
                    let j = FACE_RIG.iter().position(|(b, _)| *b == format!("{side}_r")).unwrap();
                    assert_eq!(p.sculpt.bones[i].mirrored(), p.sculpt.bones[j], "{}: {name}", p.id);
                }
            }
        }
        assert_eq!(all.iter().filter(|p| p.one_sided).map(|p| p.id.as_str()).collect::<Vec<_>>(), ["ears_point"]);
    }

    #[test]
    fn the_shapes_reader_is_strict() {
        let bad = |t: &str| read_shapes(t).is_err();
        assert!(!bad("[a] A: one
nose_reach 0.5
[b] B: two
chin_reach -1"));
        assert!(bad("nose_reach 0.5"), "a slider before any shape");
        assert!(
            bad("[a] A
nose_reach 0.5"),
            "a name without a line on it"
        );
        assert!(
            bad("[a] A: one
nose 0.5"),
            "no such slider"
        );
        assert!(
            bad("[a] A: one
nose_reach 1.5"),
            "past 1"
        );
        assert!(
            bad("[a] A: one
nose_reach 0.5
nose_reach 0.2"),
            "the same slider twice"
        );
        assert!(bad("[a] A: one
[a] B: two"));
        assert_eq!(shape("brute").unwrap().name, "Brute");
    }

    #[test]
    fn the_presets_reader_is_strict() {
        let bad = |t: &str| read_presets::<FaceSculpt>(t).is_err();
        assert!(!bad("[a] A
face_chin t 0 100 0
[b+] B
face_ear_l s 0 0 100 mirror"));
        assert!(bad("face_chin t 0 100 0"), "a bone before any preset");
        assert!(
            bad("[a] A
face_nose t 0 100 0"),
            "no such bone"
        );
        assert!(
            bad("[a] A
face_chin t 0 9999 0"),
            "past the limits"
        );
        assert!(
            bad("[a] A
face_cheek_l r 0 100 0"),
            "cheeks don't turn"
        );
        assert!(
            bad("[a] A
face_chin t 0 100 0 mirror"),
            "mirror on a middle bone"
        );
        assert!(bad("[a] A
face_chin q 0 100 0"));
        assert!(
            bad("[a] A
face_chin t 0 100 0
face_chin t 0 50 0"),
            "the same move twice"
        );
        assert!(bad("[a] A
[a] B"));
    }

    #[test]
    fn sliders_compose_into_the_face() {
        let tip = FACE_RIG.iter().position(|(b, _)| *b == "face_nose_tip").unwrap();
        let chin = FACE_RIG.iter().position(|(b, _)| *b == "face_chin").unwrap();
        let reach = preset("nose_reach").unwrap().sculpt.bones[tip].t;
        let s = |v: &[(&str, f32)]| v.iter().map(|(k, x)| (k.to_string(), *x)).collect::<Vec<_>>();
        assert_eq!(compose(None, &[]), None);
        // Body sliders aren't the face's.
        assert_eq!(compose(None, &s(&[("body_bulk", 1.0)])), None);
        assert_eq!(compose(None, &s(&[("nose_reach", 0.5)])).unwrap().bones[tip].t, reach.map(|x| x / 2));
        assert_eq!(compose(None, &s(&[("nose_reach", -1.0)])).unwrap().bones[tip].t, reach.map(|x| -x));
        // One-sided: below zero does nothing.
        assert_eq!(compose(None, &s(&[("ears_point", -1.0)])), None);
        // Summed with the recipe's own sculpt, then clamped to the bone's limits.
        let f = sample();
        let both = compose(Some(&f), &s(&[("nose_reach", 1.0)])).unwrap();
        assert_eq!(both.bones[tip].t, [0, f.bones[tip].t[1] + reach[1], f.bones[tip].t[2]]);
        assert_eq!(both.bones[2], f.bones[2]);
        let far = compose(None, &s(&[("chin_reach", 3.0)])).unwrap();
        assert_eq!(far.bones[chin].t[1], LIMITS[chin].t);
        // What the sliders make saves and loads like any face.
        assert_eq!(read(data::parse(&write(&both)).unwrap(), "face").unwrap(), both);
        assert_eq!(from_bytes(&to_bytes(&both)).unwrap(), both);
    }

    #[test]
    fn the_reader_is_strict() {
        let bad = |t: &str| read(data::parse(t).unwrap(), "face").is_err();
        assert!(bad("(rig: 2)"));
        assert!(bad("(rig: 1, bones: (face_nose: (t: (0, 0, 0))))"));
        assert!(bad("(rig: 1, bones: (face_nose_tip: (t: (0, 0, 9999))))"));
        assert!(bad("(rig: 1, bones: (face_cheek_l: (r: (0, 0, 10))))"), "cheeks don't turn");
        assert!(!bad("(rig: 1, bones: (face_chin: (t: (0, -90, 0))))"));
    }

    #[test]
    fn baking_moves_the_rest_and_keeps_the_bind() {
        let mut m = Model::default();
        let node = |name: &str, parent: Option<usize>, t: [f32; 3]| mftr_pack::glb::Node {
            name: name.into(),
            parent,
            translation: t,
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            mesh: None,
            skin: None,
        };
        m.nodes.push(node("head", None, [0.0, 1.6, 0.0]));
        m.nodes.push(node("face_nose_tip", Some(0), [0.0, 0.1, 0.1]));
        m.nodes.push(node("face_nostril_l", Some(1), [0.02, -0.01, 0.0]));
        let mut f = FaceSculpt::default();
        f.bones[11].s = [300, 300, 300]; // the nose tip grows ~23%
        f.bones[12].t = [100, 0, 0]; // the left nostril flares out 1 cm
        f.bones[13].r = [0, 0, 500]; // the right one has no bone here: ignored
        bake(&mut m, &f);
        let tip = &m.nodes[1];
        assert!((tip.scale[0] - 2f32.powf(0.3)).abs() < 1e-4, "{:?}", tip.scale);
        let nostril = &m.nodes[2];
        // The tip's scale is divided back out of the nostril (scale stays local), and the
        // nostril moved 1 cm along its own x.
        assert!((nostril.scale[0] - 1.0 / 2f32.powf(0.3)).abs() < 1e-4, "{:?}", nostril.scale);
        assert!((nostril.translation[0] - (0.02 / 2f32.powf(0.3) + 0.01)).abs() < 1e-4, "{:?}", nostril.translation);
        assert_eq!(m.bind[2], Some(([0.02, -0.01, 0.0], [0.0, 0.0, 0.0, 1.0], [1.0; 3])));
        assert_eq!(m.bind[0], None, "head untouched");
    }
}
