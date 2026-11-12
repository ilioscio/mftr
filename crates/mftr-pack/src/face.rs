//! The face rig standard (21 §1.3, D76): 24 face bones under `head` on the champion creator's
//! humanoid bases, and what a base's face rig must satisfy (21 §6). A face is sculpted by
//! moving these bones; `mftr-forge` bakes the moves into their rest transforms.

use std::collections::HashMap;

use crate::glb::Model;

pub const RIG_VERSION: u8 = 1;

/// The face bones in rig order (21 §1.3), each with its parent. Parents come before children.
pub const FACE_RIG: [(&str, &str); 24] = [
    ("face_forehead", "head"),
    ("face_brow_mid", "head"),
    ("face_brow_l", "head"),
    ("face_brow_r", "head"),
    ("face_eye_l", "head"),
    ("face_eye_r", "head"),
    ("face_cheekbone_l", "head"),
    ("face_cheekbone_r", "head"),
    ("face_cheek_l", "head"),
    ("face_cheek_r", "head"),
    ("face_nose_bridge", "head"),
    ("face_nose_tip", "face_nose_bridge"),
    ("face_nostril_l", "face_nose_tip"),
    ("face_nostril_r", "face_nose_tip"),
    ("face_lip_upper", "head"),
    ("jaw", "head"),
    ("face_lip_lower", "jaw"),
    ("face_mouth_l", "head"),
    ("face_mouth_r", "head"),
    ("face_chin", "jaw"),
    ("face_jaw_angle_l", "jaw"),
    ("face_jaw_angle_r", "jaw"),
    ("face_ear_l", "head"),
    ("face_ear_r", "head"),
];

/// Along any edge, no face bone's weight may change by more than this (21 §1.4: bigger jumps
/// pinch).
pub const MAX_WEIGHT_STEP: f32 = 0.35;
/// Rings of vertices above the neck seam that face bones may not weigh on (21 §1.5).
pub const SEAM_RINGS: usize = 2;

/// Whether `name` is a face bone (`jaw` included).
pub fn is_face_bone(name: &str) -> bool {
    FACE_RIG.iter().any(|(n, _)| *n == name)
}

/// Every problem with a model's face rig, if it has one (empty when it has none, or a valid one).
pub fn problems(model: &Model) -> Vec<String> {
    let mut out = Vec::new();
    let present: Vec<bool> = FACE_RIG.iter().map(|(n, _)| model.node_by_name(n).is_some()).collect();
    // `jaw` alone is an ordinary optional bone (champions' mouths); the rig is the face_* bones.
    let has_rig = FACE_RIG.iter().zip(&present).any(|((n, _), p)| *p && *n != "jaw");
    if !has_rig {
        return out;
    }
    for (name, parent) in FACE_RIG {
        let Some(node) = model.node_by_name(name) else {
            out.push(format!("face rig: no `{name}`"));
            continue;
        };
        let actual = model.nodes[node].parent.map(|q| model.nodes[q].name.as_str());
        if actual != Some(parent) {
            out.push(format!("face rig: `{name}` must be a child of `{parent}`, not {actual:?}"));
        }
    }
    // Clips never key face bones (the face is sculpted into their rest); the jaw may open.
    for a in &model.animations {
        for c in &a.channels {
            let name = model.nodes[c.node].name.as_str();
            if is_face_bone(name) && name != "jaw" {
                out.push(format!("face rig: clip `{}` keys `{name}` (face bones are never animated)", a.name));
                break;
            }
        }
    }
    let Some(joints) = model.skins.first() else { return out };
    let face_joint: Vec<bool> = joints.iter().map(|&n| is_face_bone(&model.nodes[n].name)).collect();
    let neck_joint: Vec<bool> = joints.iter().map(|&n| model.nodes[n].name == "neck").collect();
    for (mi, prims) in model.meshes.iter().enumerate() {
        // Weld split vertices (color and UV seams) by position, so rings and edges cross them.
        let key = |p: [f32; 3]| [p[0], p[1], p[2]].map(|v| (v * 1.0e4).round() as i64);
        for (pi, p) in prims.iter().enumerate() {
            if p.joints.is_empty() || p.weights.is_empty() {
                continue;
            }
            let mut weld: HashMap<[i64; 3], usize> = HashMap::new();
            let ids: Vec<usize> = p
                .positions
                .iter()
                .map(|&q| {
                    let n = weld.len();
                    *weld.entry(key(q)).or_insert(n)
                })
                .collect();
            let n = weld.len();
            let mut neighbours: Vec<Vec<usize>> = vec![Vec::new(); n];
            for t in p.indices.chunks(3) {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    let (a, b) = (ids[a as usize], ids[b as usize]);
                    if a != b && !neighbours[a].contains(&b) {
                        neighbours[a].push(b);
                        neighbours[b].push(a);
                    }
                }
            }
            let weight_of = |v: usize, pred: &dyn Fn(usize) -> bool| -> f32 {
                (0..4).filter(|&k| pred(p.joints[v][k] as usize)).map(|k| p.weights[v][k]).sum()
            };
            let face_w: Vec<f32> = (0..p.positions.len())
                .map(|v| weight_of(v, &|j| face_joint.get(j).copied().unwrap_or(false)))
                .collect();
            // The seam: welded vertices the neck weighs on, and SEAM_RINGS rings above them.
            let mut seam = vec![usize::MAX; n];
            let mut frontier: Vec<usize> = Vec::new();
            for v in 0..p.positions.len() {
                if weight_of(v, &|j| neck_joint.get(j).copied().unwrap_or(false)) > 0.0 && seam[ids[v]] == usize::MAX {
                    seam[ids[v]] = 0;
                    frontier.push(ids[v]);
                }
            }
            for ring in 1..=SEAM_RINGS {
                let mut next = Vec::new();
                for &w in &frontier {
                    for &u in &neighbours[w] {
                        if seam[u] == usize::MAX {
                            seam[u] = ring;
                            next.push(u);
                        }
                    }
                }
                frontier = next;
            }
            if let Some(v) = (0..p.positions.len()).find(|&v| seam[ids[v]] != usize::MAX && face_w[v] > 0.0) {
                out.push(format!(
                    "face rig: mesh {mi} primitive {pi}: face bones weigh {:.2} on vertex {v}, at or near the neck seam (21 §1.5)",
                    face_w[v]
                ));
            }
        }
    }
    out.extend(weight_steps(model, "face rig", &|n| is_face_bone(n)));
    out
}

/// No pinching (21 §1.4): for every bone `of` picks, its weight changes by at most
/// `MAX_WEIGHT_STEP` along every edge of every skinned mesh. One problem per primitive.
pub(crate) fn weight_steps(model: &Model, what: &str, of: &dyn Fn(&str) -> bool) -> Vec<String> {
    let mut out = Vec::new();
    let Some(joints) = model.skins.first() else { return out };
    let picked: Vec<bool> = joints.iter().map(|&n| of(&model.nodes[n].name)).collect();
    for (mi, prims) in model.meshes.iter().enumerate() {
        for (pi, p) in prims.iter().enumerate() {
            if p.joints.is_empty() || p.weights.is_empty() {
                continue;
            }
            let weight_of = |v: usize, j: usize| -> f32 {
                (0..4).filter(|&k| p.joints[v][k] as usize == j).map(|k| p.weights[v][k]).sum()
            };
            'edges: for t in p.indices.chunks(3) {
                for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                    for (v, o) in [(a as usize, b as usize), (b as usize, a as usize)] {
                        for k in 0..4 {
                            let j = p.joints[v][k] as usize;
                            if p.weights[v][k] <= 0.0 || !picked.get(j).copied().unwrap_or(false) {
                                continue;
                            }
                            let step = (weight_of(v, j) - weight_of(o, j)).abs();
                            if step > MAX_WEIGHT_STEP + 1.0 / 255.0 {
                                out.push(format!(
                                    "{what}: mesh {mi} primitive {pi}: `{}` jumps {step:.2} along an edge (at most {MAX_WEIGHT_STEP}, 21 §1.4)",
                                    model.nodes[joints[j]].name,
                                ));
                                break 'edges;
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glb::{Node, Primitive};

    fn node(name: &str, parent: Option<usize>) -> Node {
        Node {
            name: name.into(),
            parent,
            translation: [0.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            scale: [1.0; 3],
            mesh: None,
            skin: None,
        }
    }

    /// root, neck, head, then the face rig under them.
    fn rigged() -> Model {
        let mut m = Model::default();
        m.nodes.push(node("root", None));
        m.nodes.push(node("neck", Some(0)));
        m.nodes.push(node("head", Some(1)));
        for (name, parent) in FACE_RIG {
            let p = m.node_by_name(parent);
            m.nodes.push(node(name, p));
        }
        m.skins.push((0..m.nodes.len()).collect());
        m
    }

    /// A strip of triangles up the neck into the face: weights per vertex (neck, head, nose).
    fn strip(m: &mut Model, w: &[(f32, f32, f32)]) {
        let neck = 1u32;
        let head = 2u32;
        let nose = m.node_by_name("face_nose_tip").unwrap() as u32;
        let mut p = Primitive::default();
        for (i, &(wn, wh, wf)) in w.iter().enumerate() {
            p.positions.push([(i % 2) as f32 * 0.01, 0.0, (i / 2) as f32 * 0.01]);
            p.joints.push([neck, head, nose, 0]);
            p.weights.push([wn, wh, wf, 0.0]);
        }
        for i in 0..w.len().saturating_sub(2) {
            p.indices.extend([i as u32, i as u32 + 1, i as u32 + 2]);
        }
        m.meshes.push(vec![p]);
    }

    #[test]
    fn a_full_rig_with_smooth_weights_away_from_the_seam_passes() {
        let mut m = rigged();
        // Neck, then head, then the nose fading in gently, rows of two.
        strip(
            &mut m,
            &[
                (1.0, 0.0, 0.0),
                (1.0, 0.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 0.7, 0.3),
                (0.0, 0.7, 0.3),
                (0.0, 0.4, 0.6),
                (0.0, 0.4, 0.6),
            ],
        );
        assert!(problems(&m).is_empty(), "{:?}", problems(&m));
    }

    #[test]
    fn missing_bones_seam_weights_and_pinches_are_caught() {
        let mut m = rigged();
        let chin = m.node_by_name("face_chin").unwrap();
        m.nodes[chin].name = "chin_renamed".into();
        assert!(problems(&m).iter().any(|p| p.contains("no `face_chin`")));
        let mut m = rigged();
        strip(&mut m, &[(1.0, 0.0, 0.0), (1.0, 0.0, 0.0), (0.0, 0.8, 0.2), (0.0, 0.8, 0.2)]);
        assert!(problems(&m).iter().any(|p| p.contains("neck seam")), "{:?}", problems(&m));
        let mut m = rigged();
        strip(
            &mut m,
            &[
                (1.0, 0.0, 0.0),
                (1.0, 0.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 1.0, 0.0),
                (0.0, 0.1, 0.9),
                (0.0, 0.1, 0.9),
            ],
        );
        assert!(problems(&m).iter().any(|p| p.contains("jumps")), "{:?}", problems(&m));
        // A model without a face rig has no face problems; `jaw` alone isn't a rig.
        let mut plain = Model::default();
        plain.nodes.push(node("head", None));
        plain.nodes.push(node("jaw", Some(0)));
        assert!(problems(&plain).is_empty());
    }
}
