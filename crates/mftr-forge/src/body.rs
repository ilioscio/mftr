//! Bone-driven bodies (21 §9, FR6): a body is a small transform delta per helper bone, the way a
//! face is one per face bone (`crate::face`, whose bake, drag and recipe forms this shares
//! through [`BoneSet`]). The helpers are leaves under the biped bones that carry the body's mass
//! (the chest, the belly, the arms' and legs' muscle), so a delta thickens, moves or slims one
//! mass and nothing else: no clip keys them, and the proportions (bone lengths) are untouched.

use mftr_pack::body::BODY_RIG;
use mftr_pack::glb::Model;
use mftr_sim::data::{DataError, Value};

use crate::face::{self, BoneDelta, BoneSet, DOF_S, DOF_T, Limits, Preset, Sculpt};

pub use mftr_pack::body::RIG_VERSION;

pub const BONES: usize = BODY_RIG.len();

/// A helper moves and grows; it never turns (a mass has no facing). `t` in 0.1 mm, `s` in 0.1%
/// of a doubling: 600 is ×1.52 or ×0.66.
const fn mass(t: i16, s: i16) -> Limits {
    Limits { t, r: 0, s, dofs: DOF_T | DOF_S }
}

/// Per helper, in `BODY_RIG` order *(start: tuned on the four humanoid bases)*. They keep the
/// silhouette inside what 20 §6.2 asks: limbs stay over 6 cm across, and the footprint within
/// its band of the hitbox.
pub const LIMITS: [Limits; BONES] = [
    mass(300, 500), // body_neck
    mass(400, 450), // body_trap_l
    mass(400, 450), // body_trap_r
    mass(300, 550), // body_delt_l
    mass(300, 550), // body_delt_r
    mass(500, 600), // body_pec_l
    mass(500, 600), // body_pec_r
    mass(400, 500), // body_lat_l
    mass(400, 500), // body_lat_r
    mass(600, 700), // body_belly
    mass(400, 500), // body_waist_l
    mass(400, 500), // body_waist_r
    mass(400, 500), // body_hip_l
    mass(400, 500), // body_hip_r
    mass(400, 550), // body_glute_l
    mass(400, 550), // body_glute_r
    mass(250, 600), // body_bicep_l
    mass(250, 600), // body_bicep_r
    mass(200, 500), // body_forearm_l
    mass(200, 500), // body_forearm_r
    mass(300, 550), // body_thigh_l
    mass(300, 550), // body_thigh_r
    mass(250, 550), // body_calf_l
    mass(250, 550), // body_calf_r
];

/// What each helper's region is called on screen. "Left" is the character's left.
pub const REGION_NAMES: [&str; BONES] = [
    "Neck",
    "Left trapezius",
    "Right trapezius",
    "Left shoulder",
    "Right shoulder",
    "Left chest",
    "Right chest",
    "Left side of the back",
    "Right side of the back",
    "Belly",
    "Left waist",
    "Right waist",
    "Left hip",
    "Right hip",
    "Left glute",
    "Right glute",
    "Left upper arm",
    "Right upper arm",
    "Left forearm",
    "Right forearm",
    "Left thigh",
    "Right thigh",
    "Left calf",
    "Right calf",
];

/// The body's helper bones.
pub static BODY: BoneSet = BoneSet {
    name: "body",
    magic: *b"BS",
    version: RIG_VERSION,
    rig: &BODY_RIG,
    limits: &LIMITS,
    regions: &REGION_NAMES,
    size_across: true,
};

/// A body: one delta per helper, in `BODY_RIG` order.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct BodySculpt {
    pub rig: u8,
    pub bones: [BoneDelta; BONES],
}

impl Default for BodySculpt {
    fn default() -> Self {
        BodySculpt { rig: RIG_VERSION, bones: [BoneDelta::ZERO; BONES] }
    }
}

impl BodySculpt {
    pub fn is_neutral(&self) -> bool {
        self.bones.iter().all(BoneDelta::is_zero)
    }

    pub fn clamped(&self) -> BodySculpt {
        self.within_limits()
    }
}

impl Sculpt for BodySculpt {
    fn set() -> &'static BoneSet {
        &BODY
    }

    fn bones(&self) -> &[BoneDelta] {
        &self.bones
    }

    fn bones_mut(&mut self) -> &mut [BoneDelta] {
        &mut self.bones
    }
}

/// Whether `model` (a base, a forged champion) carries the helpers.
pub fn rigged(model: &Model) -> bool {
    model.node_by_name(BODY_RIG[0].0).is_some()
}

// --- The body sliders as presets on the helpers (21 §9.2, FR6) --------------------------------

const PRESETS_SOURCE: &str = include_str!("../../../art/creator/bodies/presets.txt");

/// Every body slider, in the file's order (`art/creator/bodies/presets.txt`).
pub fn presets() -> &'static [Preset<BodySculpt>] {
    static ALL: std::sync::OnceLock<Vec<Preset<BodySculpt>>> = std::sync::OnceLock::new();
    ALL.get_or_init(|| {
        face::read_presets(PRESETS_SOURCE).unwrap_or_else(|e| panic!("art/creator/bodies/presets.txt: {e}"))
    })
}

pub fn preset(id: &str) -> Option<&'static Preset<BodySculpt>> {
    presets().iter().find(|p| p.id == id)
}

/// The body a recipe gets: its own `body` plus each body slider in `shape` times its preset,
/// summed and clamped. None when neither moves anything.
pub fn compose(body: Option<&BodySculpt>, shape: &[(String, f32)]) -> Option<BodySculpt> {
    face::compose_with(body, shape, presets())
}

/// Read a recipe's `body: (rig: 1, bones: (body_pec_l: (s: (120, 0, 120)), …))`, as strictly as
/// a face.
pub fn read(v: Value, at: &str) -> Result<BodySculpt, DataError> {
    face::read_sculpt(v, at)
}

pub fn from_bytes(bytes: &[u8]) -> Result<BodySculpt, String> {
    face::sculpt_from_bytes(bytes)
}

// --- Rigid parts (21 §9.2) --------------------------------------------------------------------

/// How much of the body's growth under it a rigid part takes as its own: ±10%.
pub const RIGID_SCALE: f32 = 0.1;

/// Settle the worn `rigid` parts (ids) of a forged, baked `model`: a plate or a pack doesn't
/// swell with the muscle under it. Each connected piece of a rigid part moves with the body
/// beneath it (the mean of where the helpers carry its vertices) and takes at most
/// `RIGID_SCALE` of their stretch about its own middle; its helper weights then go back to the
/// helpers' parents, so the skin doesn't stretch it again. Run on every forge of a base with
/// helpers, sculpted or not, so a rigid part is skinned the same either way.
pub fn settle_rigid(model: &mut Model, rigid: &[&str]) {
    if rigid.is_empty() || !rigged(model) {
        return;
    }
    let Some(joints) = model.skins.first().cloned() else { return };
    // Per joint: the joint of its parent, when it's a helper.
    let fold: Vec<Option<u32>> = joints
        .iter()
        .map(|&n| {
            let node = &model.nodes[n];
            let parent = node.parent.filter(|_| BODY.index(&node.name).is_some())?;
            joints.iter().position(|&j| j == parent).map(|j| j as u32)
        })
        .collect();
    let shown = face::skinned(model);
    for ni in 0..model.nodes.len() {
        let Some(mi) = model.nodes[ni].mesh else { continue };
        if !model.nodes[ni].name.strip_prefix("part.").is_some_and(|id| rigid.contains(&id)) {
            continue;
        }
        let prims = &mut model.meshes[mi];
        // The part's connected pieces, across its primitives (split vertices welded).
        let mut weld: std::collections::HashMap<[i64; 3], usize> = std::collections::HashMap::new();
        let mut place: Vec<Vec<usize>> = Vec::new();
        for p in prims.iter() {
            place.push(
                p.positions
                    .iter()
                    .map(|q| {
                        let n = weld.len();
                        *weld.entry(q.map(|v| (v * 1.0e4).round() as i64)).or_insert(n)
                    })
                    .collect(),
            );
        }
        let mut piece: Vec<usize> = (0..weld.len()).collect();
        fn root(piece: &mut [usize], mut a: usize) -> usize {
            while piece[a] != a {
                piece[a] = piece[piece[a]];
                a = piece[a];
            }
            a
        }
        for (p, ids) in prims.iter().zip(&place) {
            for t in p.indices.chunks(3) {
                let a = root(&mut piece, ids[t[0] as usize]);
                for &v in &t[1..] {
                    let b = root(&mut piece, ids[v as usize]);
                    piece[b] = a;
                }
            }
        }
        // Per piece: its middle and how far the body carries it.
        let n = weld.len();
        let mut count = vec![0f32; n];
        let mut middle = vec![[0f32; 3]; n];
        let mut carried = vec![[0f32; 3]; n];
        for (pi, p) in prims.iter().enumerate() {
            for (vi, pos) in p.positions.iter().enumerate() {
                let r = root(&mut piece, place[pi][vi]);
                count[r] += 1.0;
                for k in 0..3 {
                    middle[r][k] += pos[k];
                    carried[r][k] += shown[mi][pi][vi][k] - pos[k];
                }
            }
        }
        for r in 0..n {
            if count[r] > 0.0 {
                middle[r] = middle[r].map(|x| x / count[r]);
                carried[r] = carried[r].map(|x| x / count[r]);
            }
        }
        // How much each piece would stretch about its middle, as a share of its size.
        let mut stretch = vec![0f32; n];
        for (pi, p) in prims.iter().enumerate() {
            for (vi, pos) in p.positions.iter().enumerate() {
                let r = root(&mut piece, place[pi][vi]);
                let own = [0, 1, 2].map(|k| shown[mi][pi][vi][k] - pos[k] - carried[r][k]);
                let arm = [0, 1, 2].map(|k| pos[k] - middle[r][k]);
                let len = |v: [f32; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                stretch[r] = stretch[r].max(len(own) / len(arm).max(0.01));
            }
        }
        for (pi, p) in prims.iter_mut().enumerate() {
            for vi in 0..p.positions.len() {
                let r = root(&mut piece, place[pi][vi]);
                let keep = if stretch[r] > RIGID_SCALE { RIGID_SCALE / stretch[r] } else { 1.0 };
                let pos = p.positions[vi];
                p.positions[vi] =
                    [0, 1, 2].map(|k| pos[k] + carried[r][k] + keep * (shown[mi][pi][vi][k] - pos[k] - carried[r][k]));
                let (Some(js), Some(ws)) = (p.joints.get_mut(vi), p.weights.get_mut(vi)) else { continue };
                for k in 0..4 {
                    let Some(Some(parent)) = fold.get(js[k] as usize) else { continue };
                    if ws[k] <= 0.0 {
                        continue;
                    }
                    match (0..4).find(|&o| o != k && js[o] == *parent && ws[o] > 0.0) {
                        Some(o) => {
                            ws[o] += ws[k];
                            ws[k] = 0.0;
                            js[k] = 0;
                        }
                        None => js[k] = *parent,
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn the_body_sliders_are_presets_on_the_helpers() {
        // 22 BW4: the old morph sliders' ids, now on the helpers; symmetric and within limits.
        let ids: Vec<&str> = presets().iter().map(|p| p.id.as_str()).collect();
        for id in
            ["body_bulk", "body_belly", "body_chest", "body_hips", "body_shoulders", "body_muscle", "neck_thickness"]
        {
            assert!(ids.contains(&id), "{id}");
        }
        for p in presets() {
            assert!(!p.sculpt.is_neutral(), "{}", p.id);
            assert_eq!(p.sculpt.clamped(), p.sculpt, "{}", p.id);
            for (i, (name, _)) in BODY_RIG.iter().enumerate() {
                if let Some(side) = name.strip_suffix("_l") {
                    let j = BODY_RIG.iter().position(|(b, _)| *b == format!("{side}_r")).unwrap();
                    assert_eq!(p.sculpt.bones[i].mirrored(), p.sculpt.bones[j], "{}: {name}", p.id);
                }
            }
        }
        // A recipe's sliders compose onto the helpers: a bigger belly moves the belly forward.
        let b = compose(None, &[("body_belly".into(), 1.0)]).unwrap();
        let belly = BODY.index("body_belly").unwrap();
        assert!(b.bones[belly].t[2] > 0);
        assert!(compose(None, &[]).is_none());
    }

    use super::*;
    use mftr_sim::data;

    fn sample() -> BodySculpt {
        let mut b = BodySculpt::default();
        let i = |n: &str| BODY.index(n).unwrap();
        b.bones[i("body_pec_l")] = BoneDelta { t: [20, 0, 150], r: [0; 3], s: [200, 0, 300] };
        b.bones[i("body_pec_r")] = b.bones[i("body_pec_l")].mirrored();
        b.bones[i("body_belly")] = BoneDelta { s: [-150, 0, -300], ..BoneDelta::ZERO };
        b.bones[i("body_calf_r")] = BoneDelta { s: [400, 0, 400], ..BoneDelta::ZERO };
        b
    }

    #[test]
    fn bodies_round_trip_through_text_and_bytes() {
        let b = sample();
        let text = face::write(&b);
        assert!(text.starts_with("(rig: 1, bones: (body_pec_l: (t: (20, 0, 150), s: (200, 0, 300))"), "{text}");
        assert_eq!(read(data::parse(&text).unwrap(), "body").unwrap(), b);
        let bytes = face::to_bytes(&b);
        assert_eq!(&bytes[..2], b"BS");
        assert_eq!(from_bytes(&bytes).unwrap(), b);
        // A body isn't a face, and the other way round.
        assert!(face::from_bytes(&bytes).is_err());
        assert!(from_bytes(&face::to_bytes(&face::FaceSculpt::default())).is_err());
        assert!(from_bytes(&bytes[..bytes.len() - 1]).is_err());
    }

    #[test]
    fn the_reader_is_strict() {
        let bad = |t: &str| read(data::parse(t).unwrap(), "body").is_err();
        assert!(bad("(rig: 2)"));
        assert!(bad("(rig: 1, bones: (body_knee_l: (s: (10, 10, 10))))"), "no such helper");
        assert!(bad("(rig: 1, bones: (face_chin: (t: (0, 10, 0))))"), "a face bone isn't the body's");
        assert!(bad("(rig: 1, bones: (body_belly: (s: (0, 0, 9999))))"), "past the limit");
        assert!(bad("(rig: 1, bones: (body_belly: (r: (0, 0, 10))))"), "helpers don't turn");
        assert!(!bad("(rig: 1, bones: (body_belly: (s: (0, 0, 300))))"));
    }

    #[test]
    fn the_limits_and_names_cover_every_helper() {
        assert_eq!(BODY.rig.len(), BODY.limits.len());
        assert_eq!(BODY.rig.len(), BODY.regions.len());
        for (b, (name, _)) in BODY_RIG.iter().enumerate() {
            // Every helper is a leaf under a biped bone, and each side has its other.
            assert_eq!(BODY.parent(b), None, "{name}");
            let m = BODY.mirror(b);
            assert_eq!(m == b, !name.ends_with("_l") && !name.ends_with("_r"), "{name}");
            assert_eq!(LIMITS[b].t, LIMITS[m].t, "{name}");
            assert_eq!(REGION_NAMES[b].starts_with("Left"), name.ends_with("_l"), "{name}");
        }
    }

    #[test]
    fn a_size_drag_grows_a_helper_across_and_not_along() {
        let cam = face::CameraFrame {
            origin: [0.0, 1.0, 3.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, -1.0],
            fov_y: 0.6,
            viewport_h: 800.0,
        };
        let frames = vec![[[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]; BONES];
        let thigh = BODY.index("body_thigh_l").unwrap();
        let mut b = BodySculpt::default();
        let size = face::DragMode::Size { width_only: false };
        assert!(face::drag(&mut b, thigh, size, [0.0; 3], [0.0, -40.0], &cam, &frames, true));
        assert_eq!(b.bones[thigh].s, [200, 0, 200]);
        assert_eq!(b.bones[BODY.mirror(thigh)].s, [200, 0, 200], "the other leg follows");
        // Helpers don't turn: a turn drag changes nothing.
        assert!(!face::drag(&mut b, thigh, face::DragMode::Turn, [0.0; 3], [30.0, 0.0], &cam, &frames, true));
        // A move goes to the other side mirrored, and stops at the limit.
        assert!(face::drag(&mut b, thigh, face::DragMode::Move, [0.5, 0.0, 0.0], [0.0; 2], &cam, &frames, true));
        assert_eq!(b.bones[thigh].t[0], LIMITS[thigh].t);
        assert_eq!(b.bones[BODY.mirror(thigh)].t[0], -LIMITS[thigh].t);
    }
}
