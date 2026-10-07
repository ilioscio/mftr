//! Checks an exported model and its sidecar against the rig standard, the caps, the clip
//! catalogue and the timing markers (10 §8.4, 11 §4).

use std::fmt;

use mftr_sim::ChampionId;

use crate::glb::{self, Model, quat_angle_deg};
use crate::rules;
use crate::sidecar::{self, Sidecar};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub level: Level,
    pub msg: String,
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tag = match self.level {
            Level::Error => "error",
            Level::Warning => "warning",
        };
        write!(f, "{tag}: {}", self.msg)
    }
}

#[derive(Debug, Default)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// Facts worth printing even when everything passes.
    pub summary: Vec<String>,
}

impl Report {
    fn err(&mut self, msg: impl Into<String>) {
        self.findings.push(Finding { level: Level::Error, msg: msg.into() });
    }

    fn warn(&mut self, msg: impl Into<String>) {
        self.findings.push(Finding { level: Level::Warning, msg: msg.into() });
    }

    pub fn errors(&self) -> usize {
        self.findings.iter().filter(|f| f.level == Level::Error).count()
    }

    pub fn warnings(&self) -> usize {
        self.findings.iter().filter(|f| f.level == Level::Warning).count()
    }
}

/// Validates the bytes of `<id>.glb` and the text of `<id>.anims.ron`.
pub fn validate(glb_bytes: &[u8], sidecar_text: &str) -> Report {
    let mut r = Report::default();
    let model = match glb::parse(glb_bytes) {
        Ok(m) => m,
        Err(e) => {
            r.err(format!("model: {e}"));
            return r;
        }
    };
    let side = match sidecar::parse(sidecar_text) {
        Ok(s) => s,
        Err(e) => {
            r.err(e);
            return r;
        }
    };
    validate_model(&model, glb_bytes.len(), &side)
}

/// The checks after parsing: `glb_len` is the file size, for the budget.
pub fn validate_model(model: &Model, glb_len: usize, side: &Sidecar) -> Report {
    let mut r = Report::default();
    r.summary.push(format!(
        "{} `{}`: {} triangles, {} bones, {} clips, {} bytes",
        side.kind,
        side.id,
        model.triangles(),
        model.skins.first().map_or(0, Vec::len),
        model.animations.len(),
        glb_len
    ));
    check_header(&mut r, side);
    check_mesh(&mut r, model, glb_len);
    let joints = check_skeleton(&mut r, model);
    check_clips(&mut r, model, side, &joints);
    r
}

fn check_header(r: &mut Report, side: &Sidecar) {
    if !matches!(side.kind.as_str(), "champion" | "library" | "rig") {
        r.err(format!("sidecar kind `{}` must be champion, library or rig", side.kind));
    }
    if side.archetype != "biped" {
        r.err(format!("archetype `{}`: only `biped` exists so far (10 §7.1)", side.archetype));
    }
    if side.rig_version != 1 {
        r.err(format!("biped rig version {} is unknown (expected 1)", side.rig_version));
    }
    if side.fps != rules::FPS {
        r.err(format!("clips must be authored at {} fps, not {}", rules::FPS, side.fps));
    }
}

fn check_mesh(r: &mut Report, m: &Model, bytes: usize) {
    let tris = m.triangles();
    if tris > rules::TRIANGLES_MAX {
        r.err(format!("{tris} triangles, over the {} cap", rules::TRIANGLES_MAX));
    } else if tris < rules::TRIANGLES_TARGET.0 || tris > rules::TRIANGLES_TARGET.1 {
        r.warn(format!(
            "{tris} triangles, outside the {}–{} target",
            rules::TRIANGLES_TARGET.0,
            rules::TRIANGLES_TARGET.1
        ));
    }
    if bytes > rules::MODEL_BYTES_BUDGET {
        r.warn(format!("{bytes} bytes, over the {} byte mesh + animation budget", rules::MODEL_BYTES_BUDGET));
    }

    // Material slots (10 §2).
    let mut non_accent = 0;
    for name in &m.materials {
        if !rules::MATERIAL_SLOTS.contains(&name.as_str()) {
            r.err(format!("material `{name}` is not one of the slots {:?}", rules::MATERIAL_SLOTS));
        } else if name != "accent" {
            non_accent += 1;
        }
    }
    if non_accent > rules::MAX_NON_ACCENT_SLOTS {
        r.err(format!("{non_accent} material slots besides `accent`, at most {}", rules::MAX_NON_ACCENT_SLOTS));
    }
    let accent = m.materials.iter().position(|n| n == "accent");
    let accent_used =
        m.meshes.iter().flatten().any(|p| p.material.is_some() && p.material == accent && p.triangles > 0);
    if !accent_used {
        r.err("no triangles use the `accent` (team color) slot (05 §1.3)");
    }

    let mut over_target = 0usize;
    for (mi, prims) in m.meshes.iter().enumerate() {
        for p in prims {
            if !p.has_color {
                r.err(format!("mesh {mi}: a primitive has no COLOR_0 (vertex colors carry the albedo)"));
            }
            if p.material.is_none() {
                r.err(format!("mesh {mi}: a primitive has no material slot"));
            }
            for w in &p.weights {
                let used = w.iter().filter(|&&x| x > 1e-3).count();
                if used > rules::INFLUENCES_TARGET {
                    over_target += 1;
                }
                let sum: f32 = w.iter().sum();
                if (sum - 1.0).abs() > 0.01 {
                    r.err(format!("mesh {mi}: vertex weights sum to {sum:.3}, not 1"));
                    break;
                }
            }
        }
    }
    if over_target > 0 {
        r.warn(format!("{over_target} vertices use more than {} bone influences", rules::INFLUENCES_TARGET));
    }

    // Ground contact and facing in the rest pose.
    let min_y = m.meshes.iter().flatten().flat_map(|p| p.positions.iter().map(|v| v[1])).fold(f32::INFINITY, f32::min);
    if min_y.is_finite() && min_y.abs() > rules::GROUND_TOLERANCE_M {
        r.err(format!("the model's lowest point is at {min_y:.3} m; it should stand on y = 0"));
    }
}

/// Returns the skin's joint node indices (empty if the skin is unusable).
fn check_skeleton(r: &mut Report, m: &Model) -> Vec<usize> {
    let skinned: Vec<_> = m.nodes.iter().filter(|n| n.skin.is_some() && n.mesh.is_some()).collect();
    if m.skins.len() != 1 || skinned.len() != 1 {
        r.err(format!("expected one skinned mesh and one skin, found {} and {}", skinned.len(), m.skins.len()));
        return Vec::new();
    }
    let joints = m.skins[0].clone();
    let names: Vec<&str> = joints.iter().map(|&j| m.nodes[j].name.as_str()).collect();
    if names.len() > rules::BONES_MAX {
        r.err(format!("{} bones, over the {} cap", names.len(), rules::BONES_MAX));
    } else if names.len() > rules::BONES_TARGET {
        r.warn(format!("{} bones, over the {} target", names.len(), rules::BONES_TARGET));
    }
    for n in &names {
        if !rules::biped_bone_allowed(n) {
            r.err(format!("bone `{n}` is not part of the biped standard (extras are `extra_<chain>_<n>`)"));
        }
    }
    for need in rules::BIPED_DEFORM.iter().chain(rules::BIPED_OTHER).chain(rules::SOCKETS) {
        if !names.contains(need) {
            r.err(format!("required bone `{need}` is missing"));
        }
    }
    let extras = names.iter().filter(|n| n.starts_with("extra_")).count();
    if extras > rules::EXTRA_BONES_MAX {
        r.err(format!("{extras} extra bones, at most {}", rules::EXTRA_BONES_MAX));
    }
    let max_joint = joints.len() as u32;
    if m.meshes.iter().flatten().flat_map(|p| p.joints.iter()).any(|j| j.iter().any(|&i| i >= max_joint)) {
        r.err("a vertex references a joint outside the skin");
    }

    // Facing (10 §7.2): toes ahead of the ankles along +Z means the model faces glTF +Z.
    let world = m.world_positions();
    let pos = |name: &str| m.node_by_name(name).map(|i| world[i]);
    if let (Some(foot), Some(toe)) = (pos("foot_l"), pos("toe_l"))
        && toe[2] <= foot[2]
    {
        r.err("the model does not face +Z (glTF): author it facing -Y in Blender");
    }
    if let (Some(l), Some(rr)) = (pos("hand_l"), pos("hand_r"))
        && l[0] <= rr[0]
    {
        r.err("left and right are swapped: `_l` bones must be on the character's left (+X)");
    }
    joints
}

fn check_clips(r: &mut Report, m: &Model, side: &Sidecar, joints: &[usize]) {
    // Clip catalogue.
    let names: Vec<&str> = m.animations.iter().map(|a| a.name.as_str()).collect();
    let mut required: Vec<String> = Vec::new();
    match side.kind.as_str() {
        "library" => required.extend(rules::SHARED_LIBRARY.iter().map(|s| s.to_string())),
        "champion" => {
            required.extend(rules::REQUIRED_CHAMPION.iter().map(|s| s.to_string()));
            match ChampionId::by_name(&side.id) {
                Some(id) => required.extend(rules::kit_clips(id.def())),
                None => {
                    r.warn(format!("`{}` is not a champion in mftr-sim: kit clips and fire times not checked", side.id))
                }
            }
        }
        _ => {
            if !names.is_empty() {
                r.warn("a rig export should carry no clips");
            }
        }
    }
    let missing: Vec<&String> = required.iter().filter(|c| !names.contains(&c.as_str())).collect();
    if !missing.is_empty() {
        r.err(format!("missing clips: {}", missing.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
    }

    let champion = (side.kind == "champion").then(|| ChampionId::by_name(&side.id)).flatten();
    let root = m.node_by_name("root");
    for a in &m.animations {
        let Some(clip) = side.clips.iter().find(|c| c.name == a.name) else {
            r.err(format!("clip `{}` has no sidecar entry (re-export)", a.name));
            continue;
        };
        let ctx = |msg: String| format!("clip `{}`: {msg}", a.name);
        if !rules::LAYERS.contains(&clip.layer.as_str()) {
            r.err(ctx(format!("layer `{}` must be full, upper or additive", clip.layer)));
        }
        if clip.frames == 0 || clip.frames > rules::MAX_CLIP_FRAMES {
            r.err(ctx(format!("{} frames (1–{})", clip.frames, rules::MAX_CLIP_FRAMES)));
        }
        let last = a.channels.iter().filter_map(|c| c.times.last()).fold(0.0f32, |x, &t| x.max(t));
        let frames_in_glb = (last * rules::FPS as f32).round() as u32;
        if a.channels.iter().any(|c| c.times.len() > 2) && frames_in_glb.abs_diff(clip.frames) > 1 {
            r.err(ctx(format!("{} frames in the sidecar but {} in the model", clip.frames, frames_in_glb)));
        }
        if a.channels.iter().any(|c| !joints.contains(&c.node)) {
            r.err(ctx("animates a node that is not a joint of the skin".into()));
        }

        // Markers.
        let mut seen: Vec<&str> = Vec::new();
        for mk in &clip.markers {
            if !rules::marker_allowed(&mk.name) {
                r.err(ctx(format!("unknown marker `{}`", mk.name)));
            }
            if seen.contains(&mk.name.as_str()) {
                r.err(ctx(format!("marker `{}` appears twice", mk.name)));
            }
            seen.push(&mk.name);
            if mk.frame > clip.frames {
                r.err(ctx(format!("marker `{}` at frame {} is past the end ({})", mk.name, mk.frame, clip.frames)));
            }
        }
        for need in rules::required_markers(&clip.name, clip.looping) {
            if clip.marker(need).is_none() {
                r.err(ctx(format!("needs a `{need}` marker")));
            }
        }
        if let (Some(fire), Some(end)) = (clip.marker("fire"), clip.marker("end"))
            && fire >= end
        {
            r.err(ctx("`fire` must come before `end`".into()));
        }
        if let (Some(a_in), Some(a_out)) = (clip.marker("loop_in"), clip.marker("loop_out"))
            && a_in >= a_out
        {
            r.err(ctx("`loop_in` must come before `loop_out`".into()));
        }
        let hits: Vec<u32> = clip.markers.iter().filter_map(|m| m.name.strip_prefix("hit_")?.parse().ok()).collect();
        if !hits.is_empty() && (1..=hits.len() as u32).any(|n| !hits.contains(&n)) {
            r.err(ctx("`hit_<n>` markers must be numbered 1, 2, 3… without gaps".into()));
        }
        if rules::LOCOMOTION.contains(&clip.name.as_str()) && clip.stride_speed.is_none_or(|s| s <= 0.0) {
            r.err(ctx("locomotion needs a positive stride_speed".into()));
        }

        // Fire time against the sim data (10 §4.3, §8.4).
        if let Some(def) = champion.map(|c| c.def())
            && let (Some(want), Some(fire)) = (rules::expected_fire_frame(def, &clip.name), clip.marker("fire"))
            && (fire as f32 - want).abs() > 1.0
        {
            r.err(ctx(format!("`fire` at frame {fire}, but the sim fires at {want:.1} (±1 frame)")));
        }

        // No root motion: the sim moves the unit.
        if let Some(root) = root {
            let rest = m.nodes[root].translation;
            for c in a.channels.iter().filter(|c| c.node == root && c.path == "translation") {
                let drift = c
                    .values
                    .iter()
                    .map(|v| ((v[0] - rest[0]).powi(2) + (v[2] - rest[2]).powi(2)).sqrt())
                    .fold(0.0, f32::max);
                if drift > rules::ROOT_XZ_TOLERANCE_M {
                    r.err(ctx(format!("root moves {drift:.3} m along the ground; clips play in place")));
                }
            }
        }

        // Seamless loops: the last pose equals the first.
        if clip.looping {
            for c in &a.channels {
                let (Some(first), Some(last)) = (c.values.first(), c.values.last()) else { continue };
                let off = match c.path.as_str() {
                    "rotation" => quat_angle_deg(first, last) > rules::LOOP_ROT_TOLERANCE_DEG,
                    _ => {
                        first.iter().zip(last).map(|(a, b)| (a - b).powi(2)).sum::<f32>().sqrt()
                            > rules::LOOP_POS_TOLERANCE_M
                    }
                };
                if off {
                    r.err(ctx(format!(
                        "loops, but `{}` {} differs between its first and last frame",
                        m.nodes[c.node].name, c.path
                    )));
                    break;
                }
            }
        }
    }
    for c in &side.clips {
        if !names.contains(&c.name.as_str()) {
            r.err(format!("sidecar lists clip `{}`, which the model lacks", c.name));
        }
    }
}
