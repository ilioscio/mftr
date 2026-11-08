//! `mftr-forge`: builds a champion's model from its recipe (CR2, 20 §11.3, D69).
//!
//! A champion made in the creator has no model file: its champion file says how it looks
//! ([`looks::Looks`]), and the forge builds the model from the **catalog**, which ships with the
//! game:
//! - a **base** per build and gender (`art/creator/<build>/export/<build>_<gender>.glb`, a
//!   validated pack of kind `base`): the body, with every slider's two extremes as morph targets;
//! - the shared **clip library** (`art/library/biped`), whose clips play on any `biped`.
//!
//! The forge blends the morph targets by the slider values, scales the skeleton for the
//! proportions (bones get longer or bigger and the mesh follows them), recolors the skin and
//! attaches the library's clips. The result is an ordinary validated pack (`mftr_pack::Loaded`)
//! that the client draws and animates like any other. It is deterministic and has no Godot in it.

pub mod looks;
pub mod moves;
pub mod paint;
pub mod recipe;

use std::path::Path;

use mftr_pack::glb::{Animation, Model, qmul, rotate};
use mftr_pack::{Loaded, pose, sidecar};

pub use looks::Looks;

/// What the forge builds from: the bases it has (each with its catalog parts) and the clip
/// library.
pub struct Catalog {
    bases: Vec<(String, Loaded, Vec<Part>)>,
    library: Loaded,
    /// The official champions' packs: the move library's clips, effects and sounds (CR6).
    packs: moves::Packs,
}

/// A catalog part (CR4, 20 §7), as `<base>.parts.ron` lists it.
#[derive(Clone, Debug, nanoserde::DeRon)]
pub struct Part {
    pub id: String,
    pub slot: String,
    pub name: String,
    /// Body bones whose triangles it hides (the undersuit beneath it).
    pub hides: Vec<String>,
    /// It shows the team accent itself (a sash, a band).
    pub accent: bool,
    pub triangles: u32,
}

#[derive(Clone, Debug, nanoserde::DeRon)]
struct PartList {
    base: String,
    parts: Vec<Part>,
}

/// The order parts are offered in, slot by slot.
pub const SLOTS: [&str; 12] =
    ["hair", "head", "face", "torso", "shoulders", "arms", "waist", "legs", "feet", "back", "weapon", "offhand"];

impl Catalog {
    /// Load the catalog under `art` (the folder the packs live in): every base that's there,
    /// validated, and the clip library. Fails without the library or any base.
    pub fn load(art: &Path) -> Result<Catalog, String> {
        let library = mftr_pack::load_file(&art.join("library/biped/export/biped_library.glb"))?;
        let mut bases = Vec::new();
        for (build, gendered) in looks::BUILDS {
            let keys: Vec<String> = if gendered {
                ["masculine", "feminine"].iter().map(|g| format!("{build}_{g}")).collect()
            } else {
                vec![build.to_string()]
            };
            for key in keys {
                let path = art.join(format!("creator/{build}/export/{key}.glb"));
                if path.exists() {
                    let base = mftr_pack::load_file(&path)?;
                    if base.kind != "base" {
                        return Err(format!("{}: not a creator base", path.display()));
                    }
                    let list = path.with_extension("parts.ron");
                    let parts = if list.exists() {
                        let text = std::fs::read_to_string(&list).map_err(|e| e.to_string())?;
                        let l: PartList =
                            nanoserde::DeRon::deserialize_ron(&text).map_err(|e| format!("{}: {e}", list.display()))?;
                        if l.base != key {
                            return Err(format!("{}: lists parts for `{}`", list.display(), l.base));
                        }
                        l.parts
                    } else {
                        Vec::new()
                    };
                    bases.push((key, base, parts));
                }
            }
        }
        if bases.is_empty() {
            return Err(format!("no creator bases under {}", art.join("creator").display()));
        }
        let mut packs = Vec::new();
        for c in mftr_sim::ChampionId::ALL {
            let id = c.def().name.to_lowercase();
            let path = art.join(format!("champions/{id}/export/{id}.glb"));
            if path.exists() {
                packs.push((c, mftr_pack::load_file(&path)?));
            }
        }
        Ok(Catalog { bases, library, packs: moves::Packs(packs) })
    }

    pub fn base(&self, key: &str) -> Option<&Loaded> {
        self.bases.iter().find(|(k, _, _)| k == key).map(|(_, b, _)| b)
    }

    /// The parts a base offers.
    pub fn parts(&self, key: &str) -> &[Part] {
        self.bases.iter().find(|(k, _, _)| k == key).map_or(&[], |(_, _, p)| p.as_slice())
    }

    /// The sliders a base offers, in its order (each target pair `<id>+`, `<id>-`).
    pub fn sliders(&self, key: &str) -> Vec<String> {
        let Some(b) = self.base(key) else { return Vec::new() };
        let mut out: Vec<String> = Vec::new();
        for n in b.model.target_names.iter().flatten() {
            let id = n.trim_end_matches(['+', '-']).to_string();
            if !out.contains(&id) {
                out.push(id);
            }
        }
        out
    }

    /// Build the champion `name` as `looks` describes it.
    pub fn forge(&self, name: &str, looks: &Looks) -> Result<Loaded, String> {
        let key = looks.base_key();
        let base = self.base(&key).ok_or(format!("no base `{key}` in the catalog"))?;
        let sliders = self.sliders(&key);
        for (id, _) in &looks.shape {
            if !sliders.contains(id) {
                return Err(format!("`{key}` has no slider `{id}`"));
            }
        }
        let mut model = base.model.clone();
        dress(&mut model, self.parts(&key), &looks.outfit)?;
        shape(&mut model, looks);
        proportion(&mut model, &looks.proportions);
        skin(&mut model, looks::SKIN_TONES[looks.skin as usize % looks::SKIN_TONES.len()]);
        for (target, region, color) in &looks.paint {
            let Some(&from) = paint::regions(&base.model, target).get(*region as usize) else {
                return Err(format!("`{target}` has no paint region {region}"));
            };
            paint::fill(&mut model, target, from, paint::PALETTE[*color as usize % paint::PALETTE.len()]);
        }
        paint::glow(&mut model, looks.signature);
        let scale = looks.proportions.factor(0) * looks.proportions.factor(1).sqrt();
        attach_clips(&mut model, &self.library.model, scale);
        let mut clips = library_clips(&self.library);
        let (mut vfx, mut sounds) = (self.library.vfx.clone(), self.library.sounds.clone());
        if let Some(m) = &looks.moves {
            // The moves' clips first; the library's fill in the rest (CC poses, utility casts).
            let (anims, side) = moves::clips(m, &self.packs, &model, scale)?;
            let names: Vec<String> = anims.iter().map(|a| a.name.clone()).collect();
            model.animations.retain(|a| !names.contains(&a.name));
            clips.retain(|c| !names.contains(&c.name));
            model.animations.extend(anims);
            clips.extend(side);
            vfx = Some(moves::effects(m, &self.packs, looks.signature));
            let own = moves::sounds(m, &self.packs);
            let covered: Vec<String> = own.iter().flat_map(|s| s.spec.events.clone()).collect();
            sounds.retain(|s| !s.spec.events.iter().any(|e| covered.contains(e)));
            sounds.extend(own);
        }
        let side = sidecar::Sidecar {
            id: name.to_string(),
            kind: "champion".to_string(),
            archetype: "biped".to_string(),
            rig_version: 1,
            fps: 30,
            clips,
        };
        let library = pose::Library::new(&model, &side)?;
        Ok(Loaded { kind: "champion".to_string(), model, library, vfx, sounds })
    }
}

/// The library pack's clips as sidecar entries (the forged model plays them all).
fn library_clips(lib: &Loaded) -> Vec<sidecar::Clip> {
    lib.library
        .clips
        .iter()
        .map(|c| sidecar::Clip {
            name: c.name.clone(),
            frames: (c.length * 30.0).round() as u32,
            looping: c.looping,
            layer: match c.layer {
                pose::Layer::Full => "full",
                pose::Layer::Upper => "upper",
                pose::Layer::Additive => "additive",
            }
            .to_string(),
            stride_speed: c.stride_speed,
            markers: c
                .markers
                .iter()
                .map(|(n, t)| sidecar::Marker { name: n.clone(), frame: (t * 30.0).round() as u32 })
                .collect(),
        })
        .collect()
}

/// Put on the outfit (CR4, 20 §7.2): keep the body and the worn parts' meshes, drop the rest of
/// the catalog, hide the body's triangles under the parts that cover them, and keep the team
/// accent showing and the champion within the triangle cap.
fn dress(model: &mut Model, catalog: &[Part], outfit: &[String]) -> Result<(), String> {
    let mut worn: Vec<&Part> = Vec::new();
    for id in outfit {
        let part = catalog.iter().find(|p| p.id == *id).ok_or(format!("no part `{id}` for this base"))?;
        if let Some(other) = worn.iter().find(|w| w.slot == part.slot) {
            return Err(format!("`{}` and `{}` both go on the {}", other.id, part.id, part.slot));
        }
        worn.push(part);
    }
    let hidden: Vec<&str> = worn.iter().flat_map(|p| p.hides.iter().map(String::as_str)).collect();
    if hidden.contains(&"pelvis") && !worn.iter().any(|p| p.accent) {
        return Err("the outfit covers the team accent: wear a part that shows it (20 §7.2)".into());
    }
    let joints = model.skins.first().cloned().unwrap_or_default();
    let hide_joint: Vec<bool> = joints.iter().map(|&n| hidden.contains(&model.nodes[n].name.as_str())).collect();
    for node in &model.nodes {
        let Some(mi) = node.mesh else { continue };
        let prims = &mut model.meshes[mi];
        match node.name.strip_prefix("part.") {
            Some(id) => {
                if !outfit.iter().any(|o| o == id) {
                    prims.clear();
                }
            }
            None if !hidden.is_empty() => {
                for p in prims.iter_mut() {
                    let under = |v: u32| {
                        p.joints
                            .get(v as usize)
                            .is_some_and(|j| hide_joint.get(j[0] as usize).copied().unwrap_or(false))
                    };
                    let kept: Vec<u32> = p
                        .indices
                        .chunks(3)
                        .filter(|t| !(under(t[0]) && under(t[1]) && under(t[2])))
                        .flatten()
                        .copied()
                        .collect();
                    p.triangles = kept.len() / 3;
                    p.indices = kept;
                }
            }
            None => {}
        }
    }
    let tris = model.triangles();
    if tris > mftr_pack::rules::TRIANGLES_MAX {
        return Err(format!("{tris} triangles: over the {} cap (20 §7.2)", mftr_pack::rules::TRIANGLES_MAX));
    }
    Ok(())
}

/// Blend the morph targets: each slider's `+` by its value above 0, its `-` below.
fn shape(model: &mut Model, looks: &Looks) {
    for (mi, prims) in model.meshes.iter_mut().enumerate() {
        let names = &model.target_names[mi];
        for p in prims.iter_mut() {
            for (ti, name) in names.iter().enumerate() {
                let id = name.trim_end_matches(['+', '-']);
                let v = looks.slider(id);
                let w = if name.ends_with('+') { v.max(0.0) } else { (-v).max(0.0) };
                if w == 0.0 {
                    continue;
                }
                for (pos, d) in p.positions.iter_mut().zip(&p.targets[ti]) {
                    for k in 0..3 {
                        pos[k] += d[k] * w;
                    }
                }
            }
            // The forged model is a champion: it carries no targets.
            p.targets.clear();
        }
    }
    for n in &mut model.target_names {
        n.clear();
    }
}

/// Which proportion (index into `Proportions::NAMES`) scales a bone, and how: `true` stretches
/// it along its length, `false` scales it whole.
fn proportion_of(bone: &str) -> Option<(usize, bool)> {
    let base = bone.trim_end_matches("_l").trim_end_matches("_r");
    match base {
        "thigh" | "calf" => Some((1, true)),
        "upperarm" | "forearm" => Some((2, true)),
        "spine_01" | "spine_02" | "chest" => Some((3, true)),
        "head" => Some((4, false)),
        "hand" => Some((5, false)),
        _ => None,
    }
}

fn add3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn inv(q: [f32; 4]) -> [f32; 4] {
    [-q[0], -q[1], -q[2], q[3]]
}

/// World rest transforms (translation, rotation) of every node; scales are 1 in our rigs.
fn world(model: &Model) -> Vec<([f32; 3], [f32; 4])> {
    let mut out: Vec<Option<([f32; 3], [f32; 4])>> = vec![None; model.nodes.len()];
    fn solve(m: &Model, i: usize, out: &mut Vec<Option<([f32; 3], [f32; 4])>>) -> ([f32; 3], [f32; 4]) {
        if let Some(w) = out[i] {
            return w;
        }
        let n = &m.nodes[i];
        let w = match n.parent {
            None => (n.translation, n.rotation),
            Some(p) => {
                let (pt, pr) = solve(m, p, out);
                (add3(pt, rotate(pr, n.translation)), qmul(pr, n.rotation))
            }
        };
        out[i] = Some(w);
        w
    }
    (0..model.nodes.len()).map(|i| solve(model, i, &mut out)).collect()
}

/// How a bone scales, in its own frame: along `axis` by `along` (a stretched bone: limbs,
/// spine) and by `whole` in every direction (head, hands).
#[derive(Clone, Copy)]
struct BoneScale {
    axis: [f32; 3],
    along: f32,
    whole: f32,
}

impl BoneScale {
    const NONE: BoneScale = BoneScale { axis: [0.0, 1.0, 0.0], along: 1.0, whole: 1.0 };

    /// `v` (in the bone's frame) scaled.
    fn apply(&self, v: [f32; 3]) -> [f32; 3] {
        let d = v[0] * self.axis[0] + v[1] * self.axis[1] + v[2] * self.axis[2];
        let k = d * (self.along - 1.0);
        [
            (v[0] + self.axis[0] * k) * self.whole,
            (v[1] + self.axis[1] * k) * self.whole,
            (v[2] + self.axis[2] * k) * self.whole,
        ]
    }
}

/// Scale the skeleton (20 §6.1) and carry the mesh with it. A stretched bone (limbs, spine)
/// scales along its length (toward its longest child, whatever the export's axis convention),
/// a scaled one (head, hands) whole. A child's offset from its parent scales by the parent's
/// scale, so longer legs lift everything above them; `height` scales it all. Every vertex is
/// carried by its bones (linear blend): into a bone's old frame, scaled, out of its new one.
fn proportion(model: &mut Model, p: &looks::Proportions) {
    let height = p.factor(0);
    let n = model.nodes.len();
    let scales: Vec<BoneScale> = (0..n)
        .map(|b| {
            let Some((i, stretch)) = proportion_of(&model.nodes[b].name) else { return BoneScale::NONE };
            if !stretch {
                return BoneScale { whole: p.factor(i), ..BoneScale::NONE };
            }
            let child = (0..n)
                .filter(|&c| model.nodes[c].parent == Some(b))
                .map(|c| model.nodes[c].translation)
                .max_by(|a, b| len(*a).total_cmp(&len(*b)));
            match child {
                Some(t) if len(t) > 1e-6 => {
                    let l = len(t);
                    BoneScale { axis: [t[0] / l, t[1] / l, t[2] / l], along: p.factor(i), whole: 1.0 }
                }
                _ => BoneScale::NONE,
            }
        })
        .collect();
    let old = world(model);
    for i in 0..n {
        let parent = model.nodes[i].parent.map_or(BoneScale::NONE, |q| scales[q]);
        let t = parent.apply(model.nodes[i].translation);
        model.nodes[i].translation = [t[0] * height, t[1] * height, t[2] * height];
    }
    // The legs hang from the pelvis: longer legs would push the feet into the ground, so lift
    // the pelvis until the feet stand where they stood (scaled by height).
    if let (Some(pelvis), Some(foot)) = (model.node_by_name("pelvis"), model.node_by_name("foot_l")) {
        let sunk = old[foot].0[1] * height - world(model)[foot].0[1];
        let frame = model.nodes[pelvis].parent.map_or([0.0, 0.0, 0.0, 1.0], |q| world(model)[q].1);
        let up = rotate(inv(frame), [0.0, sunk, 0.0]);
        model.nodes[pelvis].translation = add3(model.nodes[pelvis].translation, up);
    }
    let new = world(model);
    let Some(joints) = model.skins.first().cloned() else { return };
    for prims in &mut model.meshes {
        for prim in prims.iter_mut() {
            for (vi, pos) in prim.positions.iter_mut().enumerate() {
                let (js, ws) = match (prim.joints.get(vi), prim.weights.get(vi)) {
                    (Some(j), Some(w)) => (*j, *w),
                    _ => continue,
                };
                let mut acc = [0.0f32; 3];
                let mut total = 0.0;
                for k in 0..4 {
                    let (j, w) = (js[k] as usize, ws[k]);
                    let Some(&node) = joints.get(j) else { continue };
                    if w <= 0.0 {
                        continue;
                    }
                    let (ot, or) = old[node];
                    let (nt, nr) = new[node];
                    let local = scales[node].apply(rotate(inv(or), sub3(*pos, ot)));
                    let moved = add3(nt, rotate(nr, [local[0] * height, local[1] * height, local[2] * height]));
                    for c in 0..3 {
                        acc[c] += moved[c] * w;
                    }
                    total += w;
                }
                if total > 0.0 {
                    *pos = [acc[0] / total, acc[1] / total, acc[2] / total];
                }
            }
        }
    }
}

fn len(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Recolor the skin slot: each color times the tone over the base's skin, so the base's
/// shading (a darker nose, cavity) stays.
fn skin(model: &mut Model, tone: [f32; 3]) {
    let Some(slot) = model.materials.iter().position(|m| m == "skin") else { return };
    let ratio = [tone[0] / looks::BASE_SKIN[0], tone[1] / looks::BASE_SKIN[1], tone[2] / looks::BASE_SKIN[2]];
    for prims in &mut model.meshes {
        for p in prims.iter_mut().filter(|p| p.material == Some(slot)) {
            for c in &mut p.colors {
                // The eyes' whites share the slot: leave near-white alone.
                if c[0] > 0.9 && c[1] > 0.88 {
                    continue;
                }
                for k in 0..3 {
                    c[k] = (c[k] * ratio[k]).min(1.0);
                }
            }
        }
    }
}

/// Copy the library's clips onto `model`, by bone name. Translation tracks (the pelvis's bob,
/// the root's slides) scale with the body, so a taller champion's hips bob from its own height.
fn attach_clips(model: &mut Model, lib: &Model, translation_scale: f32) {
    let rig = Retarget::new(lib, model);
    model.animations = lib.animations.iter().map(|a| rig.clip(a, &a.name, translation_scale)).collect();
}

/// Moving clips from one rig onto another by bone name (the library's, or a move's champion's,
/// onto a forged model). The rigs share the `biped` bones but not always their rest angles:
/// Bastion's neck hangs down from his chest where every other rig's stands up (14 §2.2's
/// hulking shape). A clip's rotation is relative to its own rest, so each key is corrected by
/// the difference in rest: with `C(b)` the turn from the source's rest frame of bone `b` to the
/// target's (both in world space), a key `q` becomes `C(parent)⁻¹ · q · C(b)`, and the bone's
/// world turn is then the source's times `C(b)`: the pose is the same, relative to each rest.
/// Translations (in the parent's frame) turn by `C(parent)⁻¹` and scale with the body.
pub(crate) struct Retarget<'a> {
    from: &'a Model,
    to: &'a Model,
    /// Per target node: (C(parent)⁻¹, C(node)), for nodes the source has.
    fix: Vec<Option<([f32; 4], [f32; 4])>>,
}

impl<'a> Retarget<'a> {
    pub(crate) fn new(from: &'a Model, to: &'a Model) -> Retarget<'a> {
        let (ws, wt) = (world(from), world(to));
        let turn = |t: usize| -> Option<[f32; 4]> {
            let s = from.node_by_name(&to.nodes[t].name)?;
            Some(qmul(inv(ws[s].1), wt[t].1))
        };
        let fix = (0..to.nodes.len())
            .map(|t| {
                let own = turn(t)?;
                let parent = to.nodes[t].parent.and_then(turn).unwrap_or([0.0, 0.0, 0.0, 1.0]);
                Some((inv(parent), own))
            })
            .collect();
        Retarget { from, to, fix }
    }

    /// `a` on the target rig, named `name`: channels for bones the target lacks are dropped.
    pub(crate) fn clip(&self, a: &Animation, name: &str, translation_scale: f32) -> Animation {
        let mut a = a.clone();
        a.name = name.to_string();
        a.channels.retain_mut(|c| {
            let Some(node) = self.to.node_by_name(&self.from.nodes[c.node].name) else { return false };
            c.node = node;
            let (pre, post) = self.fix[node].unwrap_or(([0.0, 0.0, 0.0, 1.0], [0.0, 0.0, 0.0, 1.0]));
            match c.path.as_str() {
                "rotation" => {
                    for v in &mut c.values {
                        if v.len() == 4 {
                            let q = qmul(qmul(pre, [v[0], v[1], v[2], v[3]]), post);
                            let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt().max(1e-9);
                            *v = q.iter().map(|x| x / n).collect();
                        }
                    }
                }
                "translation" => {
                    for v in &mut c.values {
                        if v.len() == 3 {
                            let t = rotate(pre, [v[0], v[1], v[2]]);
                            *v = t.iter().map(|x| x * translation_scale).collect();
                        }
                    }
                }
                _ => {}
            }
            true
        });
        a
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog() -> Catalog {
        Catalog::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art")).unwrap()
    }

    fn head_top(m: &Model) -> f32 {
        m.meshes.iter().flatten().flat_map(|p| p.positions.iter().map(|v| v[1])).fold(f32::MIN, f32::max)
    }

    #[test]
    fn the_catalog_has_both_slim_bases_and_their_sliders() {
        let c = catalog();
        for key in ["humanoid_slim_masculine", "humanoid_slim_feminine"] {
            let s = c.sliders(key);
            assert!(s.len() >= 20, "{key}: {s:?}");
            assert!(s.contains(&"nose_reach".to_string()));
        }
    }

    #[test]
    fn the_heavy_bases_are_broader_with_the_same_sliders_and_parts() {
        let c = catalog();
        let width = |key: &str| {
            let b = c.base(key).unwrap();
            b.model.meshes[0].iter().flat_map(|p| p.positions.iter().map(|v| v[0].abs())).fold(0.0, f32::max)
        };
        for g in ["masculine", "feminine"] {
            let (slim, heavy) = (format!("humanoid_slim_{g}"), format!("humanoid_heavy_{g}"));
            assert_eq!(c.sliders(&heavy), c.sliders(&slim));
            let ids = |k: &str| c.parts(k).iter().map(|p| p.id.clone()).collect::<Vec<_>>();
            assert_eq!(ids(&heavy), ids(&slim));
            assert!(ids(&heavy).contains(&"head.great_helm".to_string()));
            assert!(width(&heavy) > width(&slim) * 1.1, "{g}: {} vs {}", width(&heavy), width(&slim));
        }
    }

    /// World transforms with each channel's first key applied.
    fn posed(m: &Model, a: &Animation) -> Vec<([f32; 3], [f32; 4])> {
        let mut m = m.clone();
        for c in &a.channels {
            let v = &c.values[0];
            match c.path.as_str() {
                "rotation" => m.nodes[c.node].rotation = [v[0], v[1], v[2], v[3]],
                "translation" => m.nodes[c.node].translation = [v[0], v[1], v[2]],
                _ => {}
            }
        }
        world(&m)
    }

    #[test]
    fn moves_from_a_rig_with_other_rest_angles_keep_their_pose() {
        // Bastion's neck hangs down from his chest; the bases' stand up. His idle on a base
        // must still hold the head above the chest, and turn each bone as his own rig does.
        let c = catalog();
        let looks = Looks { moves: Some(moves::Moves::of(mftr_sim::ChampionId::Bastion)), ..Looks::default() };
        let f = c.forge("Hunch", &looks).unwrap();
        let idle = f.model.animations.iter().find(|a| a.name == "idle").unwrap();
        let w = posed(&f.model, idle);
        let (head, chest) = (f.model.node_by_name("head").unwrap(), f.model.node_by_name("chest").unwrap());
        assert!(w[head].0[1] > w[chest].0[1] + 0.1, "head {:?} chest {:?}", w[head].0, w[chest].0);
        // The same pose relative to rest: the head's world turn from its rest matches Bastion's.
        let src = &c.packs.get(mftr_sim::ChampionId::Bastion).unwrap().model;
        let ws = posed(src, src.animations.iter().find(|a| a.name == "idle").unwrap());
        let (rs, rt) = (world(src), world(&f.model));
        let sh = src.node_by_name("head").unwrap();
        let delta = |w: [f32; 4], r: [f32; 4]| qmul(w, inv(r));
        let (a, b) = (delta(ws[sh].1, rs[sh].1), delta(w[head].1, rt[head].1));
        let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3]).abs();
        assert!(dot > 0.999, "{a:?} vs {b:?}");
    }

    #[test]
    fn rook_bastion_and_cairn_are_recipes_on_the_heavy_build() {
        let c = catalog();
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art");
        for champ in [mftr_sim::ChampionId::Rook, mftr_sim::ChampionId::Bastion, mftr_sim::ChampionId::Cairn] {
            let id = champ.def().name.to_lowercase();
            let text = std::fs::read_to_string(art.join(format!("champions/{id}/{id}.champion.ron"))).unwrap();
            let r = recipe::Recipe::from_text(&text).unwrap();
            // The kit is the official one, exactly.
            assert_eq!(&r.kit, champ.def(), "{id}");
            assert_eq!(r.looks.build, "humanoid_heavy");
            let f = c.forge(&r.name, &r.looks).unwrap();
            for clip in ["idle", "run", "attack_1", "q", "w", "e", "r"] {
                let has = |a: &Animation| a.name == clip || a.name.starts_with(&format!("{clip}_"));
                assert!(f.model.animations.iter().any(has), "{id}: no `{clip}`");
            }
            // The pack validator takes it as a champion file (an official name only warns).
            let report = mftr_pack::champion::validate_text(&text);
            assert_eq!(report.errors(), 0, "{id}: {:?}", report.findings);
        }
    }

    #[test]
    fn a_plain_forge_is_the_base_with_clips() {
        let c = catalog();
        let looks = Looks { skin: 2, ..Looks::default() };
        let f = c.forge("Plain", &looks).unwrap();
        let base = c.base("humanoid_slim_masculine").unwrap();
        assert_eq!(f.kind, "champion");
        assert!(f.library.clip("walk").is_some());
        for (a, b) in f.model.meshes.iter().flatten().zip(base.model.meshes.iter().flatten()) {
            for (p, q) in a.positions.iter().zip(&b.positions) {
                assert!((0..3).all(|k| (p[k] - q[k]).abs() < 1e-5), "{p:?} {q:?}");
            }
        }
    }

    #[test]
    fn sliders_and_proportions_change_the_model() {
        let c = catalog();
        let plain = c.forge("Plain", &Looks::default()).unwrap();
        let mut long_nose = Looks::default();
        long_nose.set_slider("nose_reach", 1.0);
        let nosed = c.forge("Nosed", &long_nose).unwrap();
        // Faces −Z in Blender, +Z in glTF: the nose tip reaches further toward +Z.
        let front = |m: &Model| {
            m.meshes.iter().flatten().flat_map(|p| p.positions.iter().map(|v| v[2])).fold(f32::MIN, f32::max)
        };
        assert!(front(&nosed.model) > front(&plain.model) + 0.02);
        let mut tall = Looks::default();
        tall.proportions.set(0, 1.0);
        tall.proportions.set(1, 1.0);
        let t = c.forge("Tall", &tall).unwrap();
        let ratio = head_top(&t.model) / head_top(&plain.model);
        assert!(ratio > 1.2 && ratio < 1.35, "{ratio}");
        // The skeleton moved with it.
        let hb = |m: &Model| world(m)[m.node_by_name("head").unwrap()].0[1];
        assert!(hb(&t.model) / hb(&plain.model) > 1.2);
    }

    #[test]
    fn unknown_sliders_and_bases_fail() {
        let c = catalog();
        let mut l = Looks::default();
        l.set_slider("tail_length", 1.0);
        assert!(c.forge("X", &l).err().unwrap().contains("tail_length"));
        let l = Looks { build: "quadruped".into(), ..Looks::default() };
        assert!(c.forge("X", &l).is_err());
    }

    #[test]
    fn an_outfit_goes_on_and_covers_the_body() {
        let c = catalog();
        let key = "humanoid_slim_masculine";
        assert!(c.parts(key).len() >= 30);
        let plain = c.forge("Plain", &Looks::default()).unwrap();
        let mut l = Looks {
            outfit: vec!["torso.breastplate".into(), "head.helmet".into(), "weapon.maul".into()],
            ..Looks::default()
        };
        let dressed = c.forge("Knight", &l).unwrap();
        let part_tris: u32 = c.parts(key).iter().filter(|p| l.outfit.contains(&p.id)).map(|p| p.triangles).sum();
        let hidden = plain.model.triangles() + part_tris as usize - dressed.model.triangles();
        assert!(hidden > 100, "the breastplate hides the undersuit beneath it: {hidden}");
        // Undressed, the catalog's other parts are gone.
        assert!(plain.model.triangles() < 6000);
        // One part per slot; a part from nowhere; the accent rule.
        l.outfit.push("torso.tunic".into());
        assert!(c.forge("X", &l).err().unwrap().contains("both go on"));
        let l = Looks { outfit: vec!["cape.dragon".into()], ..Looks::default() };
        assert!(c.forge("X", &l).err().unwrap().contains("no part"));
    }

    #[test]
    fn parts_follow_the_sliders() {
        let c = catalog();
        let mut l = Looks { outfit: vec!["head.helmet".into()], ..Looks::default() };
        let narrow = c.forge("A", &l).unwrap();
        l.set_slider("head_width", 1.0);
        let wide = c.forge("B", &l).unwrap();
        let width = |m: &Model| {
            let node = m.nodes.iter().find(|n| n.name == "part.head.helmet").unwrap();
            let xs: Vec<f32> =
                m.meshes[node.mesh.unwrap()].iter().flat_map(|p| p.positions.iter().map(|v| v[0])).collect();
            xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min)
        };
        assert!(width(&wide.model) > width(&narrow.model) * 1.08);
    }

    #[test]
    fn paint_fills_a_region() {
        let c = catalog();
        let base = c.base("humanoid_slim_masculine").unwrap();
        let regions = paint::regions(&base.model, "torso.tunic");
        assert!(!regions.is_empty());
        let mut l = Looks { outfit: vec!["torso.tunic".into()], ..Looks::default() };
        l.paint.push(("torso.tunic".into(), 0, 18));
        let f = c.forge("Painted", &l).unwrap();
        let tunic = f.model.nodes.iter().find(|n| n.name == "part.torso.tunic").unwrap().mesh.unwrap();
        let red = paint::PALETTE[18];
        assert!(f.model.meshes[tunic].iter().flat_map(|p| &p.colors).any(|c| (c[0] - red[0]).abs() < 1e-4));
        // A region that isn't there.
        l.paint = vec![("torso.tunic".into(), 15, 1)];
        assert!(c.forge("X", &l).err().unwrap().contains("no paint region"));
    }

    #[test]
    fn moves_bring_their_clips_effects_and_sounds() {
        let c = catalog();
        let mut m = moves::Moves::of(mftr_sim::ChampionId::Vesper);
        m.set("q", "ember.w");
        m.set("e", "shade.e");
        let l = Looks { moves: Some(m), signature: 0, ..Looks::default() };
        let f = c.forge("Mixed", &l).unwrap();
        for clip in ["idle", "attack_1", "q", "w", "e_start", "e_travel", "e_land", "r", "cc_stunned"] {
            assert!(f.library.clip(clip).is_some(), "{clip}");
        }
        let fx = f.vfx.as_ref().unwrap();
        // Ember's W (a delayed area) now on Q, in the signature hue (red).
        assert!(fx.effects.iter().any(|e| e.event.starts_with("q.")));
        assert!(!fx.effects.iter().any(|e| e.event.starts_with("q.") && e.kit == "arrow"));
        assert!(f.sounds.iter().any(|s| s.spec.events.iter().any(|e| e.starts_with("attack."))));
        // A misplaced ultimate.
        let mut bad = moves::Moves::of(mftr_sim::ChampionId::Rook);
        bad.set("q", "rook.r");
        assert!(c.forge("X", &Looks { moves: Some(bad), ..Looks::default() }).is_err());
    }

    #[test]
    fn forging_is_deterministic() {
        let c = catalog();
        let mut l = Looks::default();
        l.set_slider("jaw_width", 0.7);
        l.proportions.set(4, -0.5);
        let a = c.forge("A", &l).unwrap();
        let b = c.forge("A", &l).unwrap();
        let flat = |m: &Model| m.meshes.iter().flatten().flat_map(|p| p.positions.clone()).collect::<Vec<_>>();
        assert_eq!(flat(&a.model), flat(&b.model));
    }
}
