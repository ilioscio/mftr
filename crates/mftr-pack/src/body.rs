//! The body's helper bones (21 §9, FR6): 24 leaves under the biped bones of the champion
//! creator's humanoid bases that carry the body's mass. A body is sculpted by moving and
//! scaling them; `mftr-forge` bakes the moves into their rest transforms, as it does a face's.

use crate::glb::Model;

pub const RIG_VERSION: u8 = 1;

/// The helpers in rig order (21 §9.1), each with its parent: a biped bone, so every helper is
/// a leaf and a scale never cascades.
pub const BODY_RIG: [(&str, &str); 24] = [
    ("body_neck", "neck"),
    ("body_trap_l", "clavicle_l"),
    ("body_trap_r", "clavicle_r"),
    ("body_delt_l", "upperarm_l"),
    ("body_delt_r", "upperarm_r"),
    ("body_pec_l", "chest"),
    ("body_pec_r", "chest"),
    ("body_lat_l", "spine_02"),
    ("body_lat_r", "spine_02"),
    ("body_belly", "spine_01"),
    ("body_waist_l", "spine_01"),
    ("body_waist_r", "spine_01"),
    ("body_hip_l", "pelvis"),
    ("body_hip_r", "pelvis"),
    ("body_glute_l", "pelvis"),
    ("body_glute_r", "pelvis"),
    ("body_bicep_l", "upperarm_l"),
    ("body_bicep_r", "upperarm_r"),
    ("body_forearm_l", "forearm_l"),
    ("body_forearm_r", "forearm_r"),
    ("body_thigh_l", "thigh_l"),
    ("body_thigh_r", "thigh_r"),
    ("body_calf_l", "calf_l"),
    ("body_calf_r", "calf_r"),
];

/// Whether `name` is a body helper.
pub fn is_body_bone(name: &str) -> bool {
    BODY_RIG.iter().any(|(n, _)| *n == name)
}

/// Every problem with a model's body rig, if it has one (empty when it has none, or a valid
/// one): every helper present under its parent with no child, no clip keying one, and no
/// helper's weight jumping along an edge.
pub fn problems(model: &Model) -> Vec<String> {
    let mut out = Vec::new();
    if !BODY_RIG.iter().any(|(n, _)| model.node_by_name(n).is_some()) {
        return out;
    }
    for (name, parent) in BODY_RIG {
        let Some(node) = model.node_by_name(name) else {
            out.push(format!("body rig: no `{name}`"));
            continue;
        };
        let actual = model.nodes[node].parent.map(|q| model.nodes[q].name.as_str());
        if actual != Some(parent) {
            out.push(format!("body rig: `{name}` must be a child of `{parent}`, not {actual:?}"));
        }
        if let Some(child) = model.nodes.iter().find(|n| n.parent == Some(node)) {
            out.push(format!("body rig: `{name}` must be a leaf, but `{}` hangs from it", child.name));
        }
    }
    // Clips never key helpers: a body is sculpted into their rest (21 §9.2).
    for a in &model.animations {
        if let Some(c) = a.channels.iter().find(|c| is_body_bone(&model.nodes[c.node].name)) {
            out.push(format!(
                "body rig: clip `{}` keys `{}` (helpers are never animated)",
                a.name, model.nodes[c.node].name
            ));
        }
    }
    out.extend(crate::face::weight_steps(model, "body rig", &|n| is_body_bone(n)));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glb::{Animation, Channel, Node, Primitive};

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

    /// The helpers' parents under a root, then the helpers.
    fn rigged() -> Model {
        let mut m = Model::default();
        m.nodes.push(node("root", None));
        for (_, parent) in BODY_RIG {
            if m.node_by_name(parent).is_none() {
                m.nodes.push(node(parent, Some(0)));
            }
        }
        for (name, parent) in BODY_RIG {
            let p = m.node_by_name(parent);
            m.nodes.push(node(name, p));
        }
        m.skins.push((0..m.nodes.len()).collect());
        m
    }

    /// A strip of triangles down a thigh: per vertex, the helper's weight (the thigh keeps the
    /// rest).
    fn strip(m: &mut Model, w: &[f32]) {
        let thigh = m.node_by_name("thigh_l").unwrap() as u32;
        let helper = m.node_by_name("body_thigh_l").unwrap() as u32;
        let mut p = Primitive::default();
        for (i, &x) in w.iter().enumerate() {
            p.positions.push([(i % 2) as f32 * 0.01, 0.0, (i / 2) as f32 * 0.01]);
            p.joints.push([thigh, helper, 0, 0]);
            p.weights.push([1.0 - x, x, 0.0, 0.0]);
        }
        for i in 0..w.len().saturating_sub(2) {
            p.indices.extend([i as u32, i as u32 + 1, i as u32 + 2]);
        }
        m.meshes.push(vec![p]);
    }

    #[test]
    fn a_full_rig_with_gentle_weights_passes() {
        let mut m = rigged();
        strip(&mut m, &[0.0, 0.0, 0.3, 0.3, 0.6, 0.6, 0.9, 0.9, 0.6, 0.6]);
        assert!(problems(&m).is_empty(), "{:?}", problems(&m));
        // A model with no helper has no body problems.
        let mut plain = Model::default();
        plain.nodes.push(node("pelvis", None));
        assert!(problems(&plain).is_empty());
    }

    #[test]
    fn missing_helpers_children_keys_and_pinches_are_caught() {
        let mut m = rigged();
        let calf = m.node_by_name("body_calf_r").unwrap();
        m.nodes[calf].name = "calf_renamed".into();
        assert!(problems(&m).iter().any(|p| p.contains("no `body_calf_r`")));
        let mut m = rigged();
        let pec = m.node_by_name("body_pec_l").unwrap();
        m.nodes.push(node("badge", Some(pec)));
        assert!(problems(&m).iter().any(|p| p.contains("must be a leaf")), "{:?}", problems(&m));
        let mut m = rigged();
        let belly = m.node_by_name("body_belly").unwrap();
        let pelvis = m.node_by_name("pelvis");
        m.nodes[belly].parent = pelvis;
        assert!(problems(&m).iter().any(|p| p.contains("child of `spine_01`")), "{:?}", problems(&m));
        let mut m = rigged();
        let thigh = m.node_by_name("body_thigh_l").unwrap();
        m.animations.push(Animation {
            name: "run".into(),
            channels: vec![Channel {
                node: thigh,
                path: "scale".into(),
                step: false,
                times: vec![0.0],
                values: vec![vec![1.0, 1.0, 1.0]],
            }],
        });
        assert!(problems(&m).iter().any(|p| p.contains("keys `body_thigh_l`")), "{:?}", problems(&m));
        let mut m = rigged();
        strip(&mut m, &[0.0, 0.0, 0.9, 0.9]);
        assert!(problems(&m).iter().any(|p| p.contains("jumps")), "{:?}", problems(&m));
    }
}
