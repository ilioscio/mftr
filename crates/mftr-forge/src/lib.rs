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

pub mod body;
pub mod eyes;
pub mod face;
pub mod image;
pub mod libmoves;
pub mod looks;
pub mod moves;
pub mod paint;
pub mod passives;
pub mod recipe;
pub mod sculpt;

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
    /// Each base's welds for sculpting (CR9), worked out when first needed.
    welds: Vec<std::sync::OnceLock<sculpt::Welds>>,
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
    /// A plate, a pack: it moves with the body under it but takes little of its growth
    /// (21 §9.2, `body::settle_rigid`).
    #[nserde(default)]
    pub rigid: bool,
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
        let welds = bases.iter().map(|_| std::sync::OnceLock::new()).collect();
        Ok(Catalog { bases, library, packs: moves::Packs(packs), welds })
    }

    pub fn base(&self, key: &str) -> Option<&Loaded> {
        self.bases.iter().find(|(k, _, _)| k == key).map(|(_, b, _)| b)
    }

    /// A base's welds (CR9, `sculpt`).
    pub fn welds(&self, key: &str) -> Option<&sculpt::Welds> {
        let i = self.bases.iter().position(|(k, _, _)| k == key)?;
        Some(self.welds[i].get_or_init(|| sculpt::Welds::of_model(&self.bases[i].1.model)))
    }

    /// The parts a base offers.
    pub fn parts(&self, key: &str) -> &[Part] {
        self.bases.iter().find(|(k, _, _)| k == key).map_or(&[], |(_, _, p)| p.as_slice())
    }

    /// The sliders a base offers, in its order (each a target pair `<id>+`, `<id>-`, or one
    /// signed target `<id>~`).
    pub fn sliders(&self, key: &str) -> Vec<String> {
        let Some(b) = self.base(key) else { return Vec::new() };
        let mut out: Vec<String> = Vec::new();
        // On a base with a face rig, the face sliders are presets on its bones (21 §2.7).
        if b.model.node_by_name("face_forehead").is_some() {
            out.extend(face::presets().iter().map(|p| p.id.clone()));
        }
        // And on one with the body's helpers, the body sliders are presets on those (21 §9.2).
        if body::rigged(&b.model) {
            out.extend(body::presets().iter().map(|p| p.id.clone()));
        }
        for n in b.model.target_names.iter().flatten() {
            let id = n.trim_end_matches(['+', '-', '~']).to_string();
            if !out.contains(&id) {
                out.push(id);
            }
        }
        out
    }

    /// Make `looks` fit its base after the build or gender changed (from the base `was`): the
    /// gender fits the build, and what the new base doesn't have comes off (sliders, parts, fills
    /// on missing regions). Sculpting and body brushwork go when the base changes: they're tied
    /// to the old base's vertices and UVs. Returns what was dropped, for the screen to say.
    pub fn fit(&self, looks: &mut Looks, was: &str) -> Vec<String> {
        looks.fit_gender();
        let key = looks.base_key();
        let Some(base) = self.base(&key) else { return Vec::new() };
        let mut dropped = Vec::new();
        let sliders = self.sliders(&key);
        let n = looks.shape.len();
        looks.shape.retain(|(id, _)| sliders.contains(id));
        if looks.shape.len() < n {
            dropped.push(format!("{} sliders this body doesn't have", n - looks.shape.len()));
        }
        let parts = self.parts(&key);
        let mut slots: Vec<&str> = Vec::new();
        let before: Vec<String> = looks.outfit.clone();
        looks.outfit.retain(|id| match parts.iter().find(|p| p.id == *id) {
            Some(p) if !slots.contains(&p.slot.as_str()) => {
                slots.push(p.slot.as_str());
                true
            }
            _ => false,
        });
        for id in before.iter().filter(|id| !looks.outfit.contains(id)) {
            dropped.push(format!("`{id}`, which this body has no fitting for"));
        }
        let worn = looks.outfit.clone();
        let n = looks.paint.len();
        looks.paint.retain(|(target, region, _)| {
            (target == "body" || worn.contains(target))
                && (*region as usize) < paint::regions(&base.model, target).len()
        });
        if looks.paint.len() < n {
            dropped.push("fills on colors this body doesn't have".into());
        }
        looks.images.retain(|i| worn.contains(&i.target) || (i.target == "body" && key == was));
        if key != was && !looks.sculpt.is_empty() {
            looks.sculpt.clear();
            dropped.push("the sculpting (it was shaped on the other body)".into());
        }
        dropped
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
        if !looks.sculpt.is_empty() {
            let welds = self.welds(&key).ok_or(format!("no base `{key}`"))?;
            sculpt::apply(&mut model, &base.model, welds, &looks.sculpt);
        }
        proportion(&mut model, &looks.proportions);
        skin(&mut model, looks::SKIN_TONES[looks.skin as usize % looks::SKIN_TONES.len()]);
        for (target, region, color) in &looks.paint {
            let Some(&from) = paint::regions(&base.model, target).get(*region as usize) else {
                return Err(format!("`{target}` has no paint region {region}"));
            };
            paint::fill(&mut model, target, from, paint::PALETTE[*color as usize % paint::PALETTE.len()]);
        }
        paint::glow(&mut model, looks.signature);
        eyes::apply(&mut model, &looks.eyes.unwrap_or_default(), paint::signature(looks.signature));
        let worn: Vec<image::Image> =
            looks.images.iter().filter(|i| i.target == "body" || looks.outfit.contains(&i.target)).cloned().collect();
        image::apply(&mut model, &worn);
        let scale = looks.proportions.factor(0) * looks.proportions.factor(1).sqrt();
        let (clips, vfx, sounds) = if base.library.clips.is_empty() {
            // A humanoid: the shared biped library, then the moves' clips from their champions.
            attach_clips(&mut model, &self.library.model, scale);
            let mut clips = library_clips(&self.library);
            let (mut vfx, mut sounds) = (self.library.vfx.clone(), self.library.sounds.clone());
            if let Some(m) = &looks.moves {
                // The moves' clips first; the library's fill in the rest (CC poses, utility casts).
                let (anims, side) = moves::clips(m, &self.packs, &self.library, &model, scale)?;
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
            (clips, vfx, sounds)
        } else {
            // A build with its own move library (CR10): its clips, Q W E R by the moves' shapes;
            // the moves' effects and sounds as for anyone.
            let m = looks.moves.clone().unwrap_or_else(|| moves::Moves::of(mftr_sim::ChampionId::Rook));
            let (anims, side) = moves::own_clips(&m, base, scale)?;
            model.animations = anims;
            let mut sounds = self.library.sounds.clone();
            let own = moves::sounds(&m, &self.packs);
            let covered: Vec<String> = own.iter().flat_map(|s| s.spec.events.clone()).collect();
            sounds.retain(|s| !s.spec.events.iter().any(|e| covered.contains(e)));
            sounds.extend(own);
            (side, Some(moves::effects(&m, &self.packs, looks.signature)), sounds)
        };
        // The face last (21 §2.3): clips were retargeted from the bones' unsculpted rest. The
        // face sliders add their presets to the recipe's own face (21 §2.7).
        if model.node_by_name("face_forehead").is_some()
            && let Some(face) = face::compose(looks.face.as_ref(), &looks.shape)
        {
            face::bake(&mut model, &face);
        }
        // The body likewise (21 §9): its helpers' deltas are relative to the proportioned rest,
        // and its sliders are presets on them. Rigid parts then settle on the body.
        if body::rigged(&model) {
            if let Some(b) = body::compose(looks.body.as_ref(), &looks.shape) {
                face::bake(&mut model, &b);
            }
            let rigid: Vec<&str> = self
                .parts(&key)
                .iter()
                .filter(|p| p.rigid && looks.outfit.contains(&p.id))
                .map(|p| p.id.as_str())
                .collect();
            body::settle_rigid(&mut model, &rigid);
        }
        let side = sidecar::Sidecar {
            id: name.to_string(),
            kind: "champion".to_string(),
            archetype: if base.library.clips.is_empty() { "biped" } else { "creature" }.to_string(),
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
    // A body helper (21 §9) hides with its parent: it's that bone's mass.
    let hide_joint: Vec<bool> = joints
        .iter()
        .map(|&n| {
            let node = &model.nodes[n];
            let owner = match node.parent {
                Some(p) if mftr_pack::body::is_body_bone(&node.name) => &model.nodes[p].name,
                _ => &node.name,
            };
            hidden.contains(&owner.as_str())
        })
        .collect();
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
                    // Under a part when most of the vertex's weight is on hidden bones.
                    let under = |v: u32| {
                        let (Some(j), Some(w)) = (p.joints.get(v as usize), p.weights.get(v as usize)) else {
                            return false;
                        };
                        let on: f32 = (0..4)
                            .filter(|&k| hide_joint.get(j[k] as usize).copied().unwrap_or(false))
                            .map(|k| w[k])
                            .sum();
                        on > 0.5
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

/// Blend the morph targets: each slider's `+` by its value above 0, its `-` below, a signed
/// `~` by the value itself (a slider whose two ends mirror each other ships one target). Face
/// sliders on a base with a face rig move bones instead (`face::compose`).
fn shape(model: &mut Model, looks: &Looks) {
    let rigged = model.node_by_name("face_forehead").is_some();
    let helpers = body::rigged(model);
    for (mi, prims) in model.meshes.iter_mut().enumerate() {
        let names = &model.target_names[mi];
        for p in prims.iter_mut() {
            for (ti, name) in names.iter().enumerate() {
                let id = name.trim_end_matches(['+', '-', '~']);
                if (rigged && face::preset(id).is_some()) || (helpers && body::preset(id).is_some()) {
                    continue;
                }
                let v = looks.slider(id);
                let w = if name.ends_with('+') {
                    v.max(0.0)
                } else if name.ends_with('-') {
                    (-v).max(0.0)
                } else {
                    v
                };
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
        // A beast's (CR10): its hind legs are legs, its forelegs arms.
        "thigh" | "calf" | "shin" | "hock" => Some((1, true)),
        "upperarm" | "forearm" | "upper_front" | "lower_front" => Some((2, true)),
        "spine_01" | "spine_02" | "chest" | "spine" => Some((3, true)),
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
            let node = &model.nodes[b];
            // A body helper (21 §9) carries its vertices as its parent would: a longer thigh
            // stretches the thigh's mass with it. The parent's axis, in the helper's frame.
            if let Some(parent) = node.parent.filter(|_| mftr_pack::body::is_body_bone(&node.name)) {
                let Some((i, true)) = proportion_of(&model.nodes[parent].name) else { return BoneScale::NONE };
                let Some(axis) = stretch_axis(model, parent) else { return BoneScale::NONE };
                return BoneScale { axis: rotate(inv(node.rotation), axis), along: p.factor(i), whole: 1.0 };
            }
            let Some((i, stretch)) = proportion_of(&node.name) else { return BoneScale::NONE };
            if !stretch {
                return BoneScale { whole: p.factor(i), ..BoneScale::NONE };
            }
            match stretch_axis(model, b) {
                Some(axis) => BoneScale { axis, along: p.factor(i), whole: 1.0 },
                None => BoneScale::NONE,
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
    if let (Some(pelvis), Some(foot)) =
        (model.node_by_name("pelvis"), model.node_by_name("foot_l").or_else(|| model.node_by_name("paw_hind_l")))
    {
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

/// The axis a stretched bone scales along, in its own frame: toward its furthest child that
/// isn't a body helper (those sit on the bone, not at its end).
fn stretch_axis(model: &Model, b: usize) -> Option<[f32; 3]> {
    let t = model
        .nodes
        .iter()
        .filter(|c| c.parent == Some(b) && !mftr_pack::body::is_body_bone(&c.name))
        .map(|c| c.translation)
        .max_by(|a, b| len(*a).total_cmp(&len(*b)))?;
    let l = len(t);
    (l > 1e-6).then(|| [t[0] / l, t[1] / l, t[2] / l])
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
            for (v, c) in p.colors.iter_mut().enumerate() {
                // The eyes share the slot (their color is the iris, 22 §2.1), and older bases'
                // eyes are near-white: leave both alone.
                if (c[0] > 0.9 && c[1] > 0.88) || p.uvs.get(v).is_some_and(|u| u[0] >= eyes::UV_BASE - 0.01) {
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
            let source = c.node;
            let Some(node) = self.to.node_by_name(&self.from.nodes[source].name) else { return false };
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
                    // A key is the source's rest plus a motion (the pelvis's bob): the target
                    // keeps its own rest and the motion scales with the body. Scaling the whole
                    // key put long legs' hips below their lifted rest, feet through the floor
                    // (21 §8 #8).
                    let src = rotate(pre, self.from.nodes[source].translation);
                    let rest = self.to.nodes[node].translation;
                    for v in &mut c.values {
                        if v.len() == 3 {
                            let t = rotate(pre, [v[0], v[1], v[2]]);
                            *v = (0..3).map(|k| rest[k] + (t[k] - src[k]) * translation_scale).collect();
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
    fn the_official_ten_are_recipes() {
        let c = catalog();
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art");
        for champ in mftr_sim::ChampionId::ALL {
            let id = champ.def().name.to_lowercase();
            let text = std::fs::read_to_string(art.join(format!("champions/{id}/{id}.champion.ron"))).unwrap();
            let r = recipe::Recipe::from_text(&text).unwrap();
            // The kit is the official one, exactly.
            assert_eq!(&r.kit, champ.def(), "{id}");
            // The brutes on the heavy build (CR8), the rest on the slim one.
            let heavy = matches!(
                champ,
                mftr_sim::ChampionId::Rook | mftr_sim::ChampionId::Bastion | mftr_sim::ChampionId::Cairn
            );
            assert_eq!(r.looks.build, if heavy { "humanoid_heavy" } else { "humanoid_slim" }, "{id}");
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
    fn the_brush_paints_where_it_touches_and_the_image_survives_a_file() {
        let c = catalog();
        let mut r = recipe::Recipe::default();
        r.looks.outfit = vec!["torso.tunic".into()];
        let f = c.forge("Painted", &r.looks).unwrap();
        // Straight at the chest from the front (glTF +Z is the front).
        let (mi, hit, n) = image::raycast(&f.model, [0.0, 1.35, 3.0], [0.0, 0.0, -1.0]).expect("hits the chest");
        assert_eq!(image::target_of(&f.model, mi).as_deref(), Some("torso.tunic"));
        let texels = image::Texels::of(&f.model, "torso.tunic");
        assert!(texels.at.iter().filter(|t| t.is_some()).count() > 400, "the tunic covers its image");
        let mut img = image::Image::new("torso.tunic");
        assert!(image::stroke(&mut img, &texels, hit, n, 0.08, image::Tool::Decal("star", 18)));
        assert!(image::stroke(
            &mut img,
            &texels,
            [hit[0] + 0.1, hit[1] - 0.1, hit[2]],
            n,
            0.03,
            image::Tool::Brush(Some(5))
        ));
        let painted = img.pixels.iter().filter(|p| **p != 0).count();
        assert!((10..1500).contains(&painted), "{painted} pixels");
        assert_eq!(img.colors, vec![18, 5]);
        r.looks.images = vec![img.clone()];
        // Through the champion file and back.
        let back = recipe::Recipe::from_text(&r.to_text()).unwrap();
        assert_eq!(back.looks.images, vec![img]);
        // The forged tunic carries it; the body doesn't.
        let f = c.forge("Painted", &back.looks).unwrap();
        let painted_prims = |target: &str| {
            image::meshes_of(&f.model, target)
                .iter()
                .flat_map(|&m| &f.model.meshes[m])
                .filter(|p| p.paint.is_some())
                .count()
        };
        assert!(painted_prims("torso.tunic") > 0);
        assert_eq!(painted_prims("body"), 0);
        // Erasing it all leaves nothing to write.
        let mut img = back.looks.images[0].clone();
        image::stroke(&mut img, &texels, hit, n, 2.0, image::Tool::Brush(None));
        assert!(img.is_empty() && img.colors.is_empty());
    }

    #[test]
    fn sculpting_goes_past_the_sliders_and_parts_follow() {
        let c = catalog();
        let key = "humanoid_slim_masculine";
        let welds = c.welds(key).unwrap();
        assert!(welds.rest.len() > 500);
        let mut looks = Looks { outfit: vec!["head.wide_hat".into(), "face.beard".into()], ..Looks::default() };
        looks.set_slider("nose_reach", 1.0);
        let plain = c.forge("Sculpted", &looks).unwrap().model;
        let top = |m: &Model, target: &str| {
            image::meshes_of(m, target)
                .iter()
                .flat_map(|&i| &m.meshes[i])
                .flat_map(|p| p.positions.iter().map(|v| v[1]))
                .fold(f32::MIN, f32::max)
        };
        let (mut offsets, height) = (Vec::new(), looks.proportions.factor(0));
        // Pull the nose further than the slider's +1: a grab straight out from the face.
        let head_y = top(&plain, "body") - 0.13;
        // The face's furthest point forward (glTF +Z), around the head's height.
        let front = |m: &Model| {
            image::meshes_of(m, "body")
                .iter()
                .flat_map(|&i| &m.meshes[i])
                .flat_map(|p| p.positions.iter().filter(|v| (v[1] - head_y).abs() < 0.12).map(|v| v[2]))
                .fold(f32::MIN, f32::max)
        };
        let nose = front(&plain);
        // Aim at the nose's tip.
        let tip_y = image::meshes_of(&plain, "body")
            .iter()
            .flat_map(|&i| &plain.meshes[i])
            .flat_map(|p| p.positions.iter().filter(|v| (v[1] - head_y).abs() < 0.12))
            .fold((f32::MIN, 0.0), |b, v| if v[2] > b.0 { (v[2], v[1]) } else { b })
            .1;
        for _ in 0..4 {
            assert!(sculpt::stroke(
                &mut offsets,
                welds,
                &plain,
                [0.0, tip_y, 3.0],
                [0.0, 0.0, -1.0],
                sculpt::Brush::Grab([0.0, 0.0, 0.02]),
                0.05,
                0.0,
                true,
                height
            ));
        }
        // A bigger skull: the hat must ride up with it.
        let crown = top(&plain, "body");
        assert!(sculpt::stroke(
            &mut offsets,
            welds,
            &plain,
            [0.0, crown + 1.0, -0.01],
            [0.0, -1.0, 0.0],
            sculpt::Brush::Inflate,
            0.22,
            0.05,
            true,
            height
        ));
        looks.sculpt = offsets.clone();
        let carved = c.forge("Sculpted", &looks).unwrap().model;
        assert!(front(&carved) > nose + 0.03, "{} vs {nose}", front(&carved));
        // The hat's cone round the skull moves with it, about as far.
        let rise = top(&carved, "body") - crown;
        assert!(rise > 0.02, "the crown rises {rise}");
        let mut lifts = Vec::new();
        for &mi in &image::meshes_of(&plain, "head.wide_hat") {
            for (p, q) in plain.meshes[mi].iter().zip(&carved.meshes[mi]) {
                for (a, b) in p.positions.iter().zip(&q.positions) {
                    // The hat's cone round the skull (not its brim, out past the head).
                    if (a[0] * a[0] + a[2] * a[2]).sqrt() < 0.2 && a[1] > crown - 0.06 && a[1] < crown + 0.1 {
                        lifts.push(((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt());
                    }
                }
            }
        }
        assert!(!lifts.is_empty());
        let lift = lifts.iter().sum::<f32>() / lifts.len() as f32;
        assert!(lift > rise * 0.4, "the hat's cone moves {lift} with the skull's {rise}");
        // A heavier jaw: the beard goes with it.
        assert!(sculpt::stroke(
            &mut offsets,
            welds,
            &plain,
            [0.0, tip_y - 0.09, 3.0],
            [0.0, 0.0, -1.0],
            sculpt::Brush::Inflate,
            0.08,
            0.03,
            true,
            height
        ));
        looks.sculpt = offsets.clone();
        let jawed = c.forge("Sculpted", &looks).unwrap().model;
        let moved = |target: &str| {
            let mut d = Vec::new();
            for &mi in &image::meshes_of(&plain, target) {
                for (p, q) in plain.meshes[mi].iter().zip(&jawed.meshes[mi]) {
                    for (a, b) in p.positions.iter().zip(&q.positions) {
                        d.push(((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt());
                    }
                }
            }
            d.iter().sum::<f32>() / d.len().max(1) as f32
        };
        assert!(moved("face.beard") > 0.01, "the beard follows the jaw: {}", moved("face.beard"));
        // Symmetric: every moved weld off the middle line has its mirror moved too.
        for (w, o) in &offsets {
            let m = welds.mirror[*w as usize];
            if m != *w && o[0] != 0 {
                assert!(offsets.iter().any(|(x, _)| *x == m), "weld {w}'s mirror {m}");
            }
        }
        // Bounded, and through the file and back.
        assert!(offsets.iter().all(|(_, o)| o.iter().all(|x| x.unsigned_abs() as f32 <= sculpt::MAX_OFFSET * 1000.0)));
        let r = recipe::Recipe { looks: looks.clone(), ..recipe::Recipe::default() };
        assert_eq!(recipe::Recipe::from_text(&r.to_text()).unwrap().looks.sculpt, looks.sculpt);
    }

    /// Made Thornmaw, the beast example (`art/champions/examples/thornmaw.champion.ron`).
    #[test]
    #[ignore]
    fn print_thornmaw() {
        let mut r = recipe::Recipe { name: "Thornmaw".into(), ..recipe::Recipe::default() };
        r.looks.build = "beast".into();
        r.looks.gender = looks::Gender::None;
        for (slot, id) in
            [("attack", "rook.attack"), ("q", "wren.q"), ("w", "rook.w"), ("e", "wren.e"), ("r", "bastion.r")]
        {
            assert!(r.set_move(slot, id), "{slot} {id}");
        }
        assert!(r.set_passive("sunder"));
        assert!(r.set_resource("rage"));
        for (k, v) in [
            ("head_snout", 0.6),
            ("jaw_heavy", 0.8),
            ("ears_size", -0.4),
            ("body_bulk", 0.5),
            ("body_chest", 0.6),
            ("body_tail", -0.3),
        ] {
            r.looks.set_slider(k, v);
        }
        r.looks.outfit = vec!["shoulders.collar".into(), "head.horns".into()];
        r.looks.paint = vec![("body".into(), 0, 5), ("body".into(), 1, 6)];
        r.looks.signature = 30;
        r.auto_fit();
        assert!(r.problems().is_empty(), "{:?}", r.problems());
        let c = catalog();
        c.forge("Thornmaw", &r.looks).unwrap();
        println!("{}", r.to_text());
    }

    /// Wrote `art/champions/examples/stormwarden.champion.ron`: a heavy humanoid made of the
    /// library's own moves (`libmoves`).
    #[test]
    #[ignore]
    fn write_stormwarden() {
        let mut r = recipe::Recipe { name: "Stormwarden".into(), ..recipe::Recipe::default() };
        r.looks.build = "humanoid_heavy".into();
        for (slot, id) in [
            ("attack", "rook.attack"),
            ("q", "lib.chain_hook"),
            ("w", "lib.battle_cry"),
            ("e", "lib.leap_strike"),
            ("r", "lib.meteor_call"),
        ] {
            assert!(r.set_move(slot, id), "{slot} {id}");
        }
        assert!(r.set_passive("cleave"));
        assert!(r.set_resource("rage"));
        for (k, v) in
            [("jaw_width", 0.7), ("brow_ridge", 0.8), ("brow_angle", 0.6), ("nose_width", 0.4), ("head_crown", -0.3)]
        {
            r.looks.set_slider(k, v);
        }
        r.looks.outfit =
            vec!["weapon.maul".into(), "shoulders.pauldrons".into(), "torso.tunic".into(), "feet.boots".into()];
        r.looks.signature = 205;
        r.auto_fit();
        assert!(r.problems().is_empty(), "{:?}", r.problems());
        catalog().forge("Stormwarden", &r.looks).unwrap();
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art/champions/examples/stormwarden.champion.ron");
        std::fs::write(&art, r.to_text()).unwrap();
    }

    /// Wrote the examples of the serpentoid, spectre and machine builds
    /// (`art/champions/examples/<name>.champion.ron`).
    #[test]
    #[ignore]
    fn write_build_examples() {
        let c = catalog();
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art/champions/examples");
        type Example = (
            &'static str,
            &'static str,
            &'static str,
            [&'static str; 5],
            &'static str,
            &'static str,
            &'static [(&'static str, f32)],
            &'static [&'static str],
            u16,
        );
        let examples: [Example; 3] = [
            (
                "Venomcoil",
                "serpent",
                "A serpent who spits from range: a burning spit, a draining coil, a veiled slither and a firestorm.",
                ["vesper.attack", "ember.q", "marrow.w", "shade.e", "ember.r"],
                "arcane_strikes",
                "mana",
                &[("head_hood", 0.8), ("head_size", 0.3), ("body_tail", 0.4)],
                &["head.crown", "shoulders.collar"],
                140,
            ),
            (
                "Hollow Warden",
                "spectre",
                "A wraith of the old wards: a falling star, a soothing light, a tumble through the air and a bone cage.",
                ["lumen.attack", "quill.q", "lumen.w", "vesper.e", "marrow.r"],
                "last_stand",
                "mana",
                &[("head_hood", 0.6), ("body_arms", 0.5), ("body_tail", 0.6)],
                &["face.mask", "back.lantern"],
                260,
            ),
            (
                "Gearback",
                "machine",
                "A siege engine on a wheel: a ricochet bolt, a bulwark, a charging lunge and the earth heaved up.",
                ["rook.attack", "wren.q", "bastion.w", "rook.e", "cairn.r"],
                "third_blow",
                "energy",
                &[("body_wheel", 0.5), ("body_bulk", 0.4), ("head_size", -0.3)],
                &["shoulders.plates", "back.stack"],
                25,
            ),
        ];
        for (name, build, blurb, moves, passive, resource, shape, outfit, hue) in examples {
            let mut r = recipe::Recipe { name: name.into(), ..recipe::Recipe::default() };
            r.looks.build = build.into();
            r.looks.gender = looks::Gender::None;
            for (slot, id) in moves::SLOTS.iter().zip(moves) {
                assert!(r.set_move(slot, id), "{name}: {slot} {id}");
            }
            assert!(r.set_passive(passive) && r.set_resource(resource));
            for (k, v) in shape {
                r.looks.set_slider(k, *v);
            }
            r.looks.outfit = outfit.iter().map(|s| s.to_string()).collect();
            r.looks.signature = hue;
            r.auto_fit();
            assert!(r.problems().is_empty(), "{name}: {:?}", r.problems());
            c.forge(name, &r.looks).unwrap();
            let file = name.to_lowercase().replace(' ', "_");
            let text = format!(
                "// {name}: an example champion of the {build} build (CR10, 20 §3), written by mftr-forge's
// write_build_examples test. {blurb}
{}",
                r.to_text().lines().filter(|l| !l.starts_with("//")).collect::<Vec<_>>().join(
                    "
"
                )
            );
            std::fs::write(
                art.join(format!("{file}.champion.ron")),
                text + "
",
            )
            .unwrap();
        }
    }

    #[test]
    #[ignore]
    fn print_grumbles_war_paint() {
        let c = catalog();
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art");
        let text = std::fs::read_to_string(art.join("champions/examples/grumble.champion.ron")).unwrap();
        let mut r = recipe::Recipe::from_text(&text).unwrap();
        let f = c.forge("Grumble", &r.looks).unwrap();
        let texels = image::Texels::of(&f.model, "body");
        let mut img = image::Image::new("body");
        let top = f.model.meshes.iter().flatten().flat_map(|p| p.positions.iter().map(|v| v[1])).fold(0.0, f32::max);
        for (y, x, radius, tool) in [
            (top * 0.70, 0.0, 0.13, image::Tool::Decal("chevron", 63)),
            (top * 0.925, 0.03, 0.05, image::Tool::Decal("stripes", 17)),
            (top * 0.925, -0.03, 0.05, image::Tool::Decal("stripes", 17)),
        ] {
            let (mi, hit, n) = image::raycast(&f.model, [x, y, 3.0], [0.0, 0.0, -1.0]).unwrap();
            println!("hit {:?} at {hit:?}", image::target_of(&f.model, mi));
            image::stroke(&mut img, &texels, hit, n, radius, tool);
        }
        r.looks.images = vec![img];
        println!("{}", looks::write(&r.looks));
    }

    #[test]
    fn every_face_bone_owns_a_region_and_the_ray_finds_it() {
        // FR4 (21 §2.4): regions come from the weights; a ray from the front meets the nose.
        let c = catalog();
        let l = Looks { outfit: vec!["head.helmet".into()], ..Looks::default() };
        let m = c.forge("Plain", &l).unwrap().model;
        let regions = face::vertex_regions(&m);
        let mut owned = [0usize; face::BONES + body::BONES];
        for (mi, prims) in regions.iter().enumerate() {
            let part = m.nodes.iter().any(|n| n.mesh == Some(mi) && n.name.starts_with("part."));
            for r in prims.iter().flatten() {
                assert!(!part || *r == 0, "a part has no face regions");
                if *r > 0 {
                    owned[*r as usize - 1] += 1;
                }
            }
        }
        for (b, n) in owned.iter().enumerate().take(face::BONES) {
            assert!(*n >= 3, "{} owns {n} vertices", face::REGION_NAMES[b]);
        }
        // FR6 (21 §9): the body's helpers own regions after the face's.
        for (b, n) in owned.iter().skip(face::BODY_REGIONS).enumerate() {
            assert!(*n >= 8, "{} owns {n} vertices", body::REGION_NAMES[b]);
        }
        // Straight at the nose tip from in front (+Z in glTF), and over the cranium: none.
        let w = world(&m);
        let tip = w[m.node_by_name("face_nose_tip").unwrap()].0;
        let (hit, _) = face::region_at(&m, &regions, [tip[0], tip[1] - 0.01, tip[2] + 1.0], [0.0, 0.0, -1.0]).unwrap();
        assert!(
            ["Nose tip", "Bridge of the nose", "Left nostril", "Right nostril"].contains(&face::REGION_NAMES[hit]),
            "{}",
            face::REGION_NAMES[hit]
        );
        let top = head_top(&m);
        assert!(
            face::region_at(&m, &regions, [0.0, top + 1.0, 0.0], [0.0, -1.0, 0.0]).is_none(),
            "the crown isn't a feature"
        );
    }

    /// Every vertex of the forged model where the skin shows it, with the node of its mesh.
    fn shown(m: &Model) -> Vec<(String, [f32; 3])> {
        let skinned = face::skinned(m);
        let mut out = Vec::new();
        for n in &m.nodes {
            let Some(mi) = n.mesh else { continue };
            for v in skinned[mi].iter().flatten() {
                out.push((n.name.clone(), *v));
            }
        }
        out
    }

    #[test]
    fn the_body_is_sculpted_on_its_helpers() {
        // FR6 (21 §9): a ray at the chest finds the chest; a body's deltas are baked into the
        // helpers' rest, move the mass they own and nothing far from it, and leave the mesh
        // buffer and the feet alone.
        let c = catalog();
        let plain = c.forge("Plain", &Looks::default()).unwrap().model;
        assert!(body::rigged(&plain));
        let regions = face::vertex_regions(&plain);
        let w = world(&plain);
        let pec = w[plain.node_by_name("body_pec_l").unwrap()].0;
        let (hit, _) = face::region_at(&plain, &regions, [pec[0], pec[1], pec[2] + 1.0], [0.0, 0.0, -1.0]).unwrap();
        assert_eq!(body::REGION_NAMES[hit - face::BODY_REGIONS], "Left chest");
        let calf = w[plain.node_by_name("body_calf_r").unwrap()].0;
        let (hit, _) = face::region_at(&plain, &regions, [calf[0], calf[1], calf[2] + 1.0], [0.0, 0.0, -1.0]).unwrap();
        assert_eq!(body::REGION_NAMES[hit - face::BODY_REGIONS], "Right calf");

        let i = |n: &str| body::BODY.index(n).unwrap();
        let mut b = body::BodySculpt::default();
        b.bones[i("body_calf_l")].s = [400, 0, 400];
        b.bones[i("body_calf_r")].s = [400, 0, 400];
        b.bones[i("body_pec_l")] = face::BoneDelta { t: [0, 0, 200], r: [0; 3], s: [200, 0, 200] };
        b.bones[i("body_pec_r")] = b.bones[i("body_pec_l")].mirrored();
        let looks = Looks { body: Some(b.clone()), ..Looks::default() };
        let sculpted = c.forge("Sculpted", &looks).unwrap().model;
        // The live pose is the bake (the editor poses the helpers while a drag lasts).
        for (node, (t, r, s)) in face::posed(&plain, &b) {
            let n = &sculpted.nodes[node];
            for k in 0..3 {
                assert!((n.translation[k] - t[k]).abs() < 1e-6 && (n.scale[k] - s[k]).abs() < 1e-6, "{}", n.name);
            }
            assert!((0..4).all(|k| (n.rotation[k] - r[k]).abs() < 1e-6), "{}", n.name);
        }
        // Nothing is moved on the CPU: the buffers are the plain model's.
        for (a, b) in plain.meshes.iter().flatten().zip(sculpted.meshes.iter().flatten()) {
            assert_eq!(a.positions, b.positions);
        }
        let (was, now) = (shown(&plain), shown(&sculpted));
        let (mut calf_out, mut chest_out, mut far) = (0.0f32, 0.0f32, 0.0f32);
        for ((_, a), (_, b)) in was.iter().zip(&now) {
            let d = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
            if a[1] < 0.5 && a[1] > 0.15 {
                calf_out = calf_out.max(d);
            } else if a[1] > 1.3 && a[1] < 1.5 && a[2] > 0.05 {
                chest_out = chest_out.max(d);
            }
            // The feet, the knees, the hands and the head stay where they were.
            if a[1] < 0.08 || (a[1] > 0.52 && a[1] < 0.56) || a[1] > 1.62 || a[0].abs() > 0.55 {
                far = far.max(d);
            }
        }
        assert!(calf_out > 0.015, "the calves grew: {calf_out}");
        assert!(chest_out > 0.02, "the chest came forward: {chest_out}");
        assert!(far < 1e-4, "feet, knees, hands and head held: {far}");
        // The feet stand where they stood.
        let floor = |m: &[(String, [f32; 3])]| m.iter().map(|(_, v)| v[1]).fold(f32::MAX, f32::min);
        assert!((floor(&was) - floor(&now)).abs() < 1e-5);
        // A body survives its file.
        let r = recipe::Recipe { looks: looks.clone(), ..recipe::Recipe::default() };
        assert_eq!(recipe::Recipe::from_text(&r.to_text()).unwrap().looks, looks);
    }

    #[test]
    fn the_live_pose_is_the_bake() {
        // FR4 (21 §3.3): posing the bones with a face shows what forging with it bakes.
        let c = catalog();
        let plain = c.forge("Plain", &Looks::default()).unwrap().model;
        let mut f = face::FaceSculpt::default();
        f.bones[11] = face::BoneDelta { t: [0, 150, -60], r: [400, 0, 0], s: [100, 100, 100] };
        f.bones[10].s = [0; 3];
        f.bones[19] = face::BoneDelta { t: [0, 120, -90], ..face::BoneDelta::ZERO };
        f.bones[15].s = [120, 0, 0];
        let baked = c.forge("Baked", &Looks { face: Some(f.clone()), ..Looks::default() }).unwrap().model;
        for (node, (t, r, s)) in face::posed(&plain, &f) {
            let n = &baked.nodes[baked.node_by_name(&plain.nodes[node].name).unwrap()];
            let close = |a: &[f32], b: &[f32]| a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-5);
            assert!(
                close(&t, &n.translation) && close(&r, &n.rotation) && close(&s, &n.scale),
                "{}",
                plain.nodes[node].name
            );
        }
    }

    #[test]
    fn a_drag_moves_the_feature_with_the_cursor_and_its_mirror() {
        // FR4 (21 §2.5): a camera looking at the face (−Z in glTF, the face toward +Z).
        let c = catalog();
        let m = c.forge("Plain", &Looks::default()).unwrap().model;
        let frames = face::bone_frames(&m, &face::FACE);
        let w = world(&m);
        let brow = w[m.node_by_name("face_brow_l").unwrap()].0;
        let cam = face::CameraFrame {
            origin: [brow[0], brow[1], brow[2] + 1.0],
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            forward: [0.0, 0.0, -1.0],
            fov_y: 30f32.to_radians(),
            viewport_h: 1000.0,
        };
        // 20 px up at 1 m: 2·tan(15°)/1000·20 ≈ 10.7 mm.
        let world_move = face::drag_to_world(&cam, [0.0, -20.0], brow);
        assert!((world_move[1] - 0.0107).abs() < 0.0005, "{world_move:?}");
        let (l, r) = (2, 3);
        let mut f = face::FaceSculpt::default();
        assert!(face::drag_face(&mut f, l, face::DragMode::Move, world_move, [0.0, -20.0], &cam, &frames, true));
        // Up is the brow bones' +z: ~107 units of 0.1 mm, both brows, mirrored.
        assert!((f.bones[l].t[2] - 107).abs() <= 2 && f.bones[l].t[0] == 0, "{:?}", f.bones[l]);
        assert_eq!(f.bones[r], f.bones[l].mirrored());
        // Sideways on the middle line stays on it with symmetry, not without.
        let side = face::drag_to_world(&cam, [30.0, 0.0], brow);
        let mut g = face::FaceSculpt::default();
        assert!(!face::drag_face(&mut g, 11, face::DragMode::Move, side, [30.0, 0.0], &cam, &frames, true));
        assert!(face::drag_face(&mut g, 11, face::DragMode::Move, side, [30.0, 0.0], &cam, &frames, false));
        assert_ne!(g.bones[11].t[0], 0);
        // Limits hold however far the drag goes.
        let far = face::drag_to_world(&cam, [0.0, -5000.0], brow);
        face::drag_face(&mut f, l, face::DragMode::Move, far, [0.0, -5000.0], &cam, &frames, true);
        assert_eq!(f.bones[l].t[2], face::LIMITS[l].t);
        // Size: a drag up grows it, on every axis the bone takes.
        let mut h = face::FaceSculpt::default();
        face::drag_face(
            &mut h,
            4,
            face::DragMode::Size { width_only: false },
            [0.0; 3],
            [0.0, -10.0],
            &cam,
            &frames,
            true,
        );
        assert!(h.bones[4].s.iter().all(|&s| s > 0) && h.bones[5].s == h.bones[4].s);
    }

    #[test]
    fn a_baked_face_moves_the_face_and_nothing_else() {
        // FR1 (21 §7): a face bone's rest change deforms the skinned head, by the bone's
        // weights alone. The nose tip out 2 cm, the brows up 1 cm.
        let c = catalog();
        let plain = c.forge("Plain", &Looks::default()).unwrap().model;
        let mut face = face::FaceSculpt::default();
        face.bones[11].t = [0, 0, 200];
        face.bones[2].t = [0, 0, 100];
        face.bones[3].t = [0, 0, 100];
        let looks = Looks { face: Some(face), ..Looks::default() };
        let sculpted = c.forge("Sculpted", &looks).unwrap().model;
        // The mesh itself isn't touched: only the face bones' rest and where they bind.
        assert_eq!(plain.meshes[0][0].positions, sculpted.meshes[0][0].positions);
        let (a, b) = (face::skinned(&plain), face::skinned(&sculpted));
        let tip = sculpted.node_by_name("face_nose_tip").unwrap();
        let bind_tip = sculpted.bind[tip].unwrap().0;
        assert_ne!(sculpted.nodes[tip].translation, bind_tip, "the tip's rest moved");
        let (mut moved, mut most, mut still_low) = (0usize, 0.0f32, true);
        for (mi, prims) in a.iter().enumerate() {
            for (pi, verts) in prims.iter().enumerate() {
                for (vi, p) in verts.iter().enumerate() {
                    let q = b[mi][pi][vi];
                    let d = ((q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) + (q[2] - p[2]).powi(2)).sqrt();
                    if d > 1e-4 {
                        moved += 1;
                        most = most.max(d);
                        // Nothing below the chin moves: the neck and the seam stay put.
                        if p[1] < 1.45 {
                            still_low = false;
                        }
                    }
                }
            }
        }
        assert!(moved > 50, "{moved} vertices moved");
        // A clear move, never more than the bone's own 2 cm. (The block-out head's weights are
        // smoothed to its long edges, so they peak low: FR2's authored head carries more.)
        assert!(most > 0.005 && most < 0.021, "the most a vertex moved: {most}");
        assert!(still_low, "something below the face moved");
        // A neutral face changes nothing at all.
        let neutral = Looks { face: Some(face::FaceSculpt::default()), ..Looks::default() };
        let n = c.forge("Neutral", &neutral).unwrap().model;
        assert!(n.bind.iter().all(Option::is_none));
    }

    #[test]
    fn library_moves_bring_their_clips_effects_sounds_and_numbers() {
        let c = catalog();
        let ids = ["rook.attack", "lib.whirlwind", "lib.battle_cry", "lib.leap_strike", "lib.meteor_call"];
        let looks = Looks { moves: Some(moves::Moves { ids: ids.map(String::from) }), ..Looks::default() };
        let f = c.forge("Librarian", &looks).unwrap();
        let has = |n: &str| f.library.clip(n).is_some();
        for clip in ["q", "w", "e_start", "e_travel", "e_land", "r", "idle", "attack_1"] {
            assert!(has(clip), "no `{clip}`");
        }
        let len = |lib: &mftr_pack::pose::Library, n: &str| lib.clip(n).map(|i| lib.clips[i].length);
        assert_eq!(len(&f.library, "r"), len(&c.library.library, "move_meteor_call"));
        let vfx = f.vfx.as_ref().unwrap();
        for event in ["q.detonate", "r.telegraph", "r.detonate", "e.land"] {
            assert!(vfx.effects.iter().any(|e| e.event == event), "no `{event}` effect");
        }
        assert!(f.sounds.iter().any(|s| s.spec.events.iter().any(|e| e.starts_with("r."))), "R borrows its sounds");
        // The recipe takes the move's numbers.
        let mut r =
            recipe::Recipe::from_text(&mftr_sim::data::write_champion(mftr_sim::ChampionId::Rook.def(), "")).unwrap();
        assert!(r.set_move("q", "lib.chain_hook"));
        assert_eq!(r.kit.abilities[0].name, "Chain Hook");
        assert!(!r.set_move("q", "lib.meteor_call"), "an ultimate only goes on R");
        // A creature plays a library move by its shape: the hook is a skillshot, a bolt.
        let beast = Looks {
            build: "beast".into(),
            gender: looks::Gender::None,
            moves: Some(moves::Moves {
                ids: ["rook.attack", "lib.chain_hook", "rook.w", "rook.e", "lib.lance_of_light"].map(String::from),
            }),
            ..Looks::default()
        };
        let b = c.forge("Hookbeast", &beast).unwrap();
        let base = c.base("beast").unwrap();
        assert_eq!(len(&b.library, "q"), len(&base.library, "bolt"));
        assert_eq!(len(&b.library, "r"), len(&base.library, "beam"));
    }

    #[test]
    fn creature_moves_play_their_shape_in_the_builds_own_way() {
        // A skillshot plays the build's bolt and a beam its beam, not the general howl.
        let c = catalog();
        for build in ["beast", "serpent", "spectre", "machine"] {
            let looks = Looks {
                build: build.into(),
                gender: looks::Gender::None,
                moves: Some(moves::Moves {
                    ids: ["rook.attack", "ember.q", "rook.w", "shade.e", "quill.r"].map(String::from),
                }),
                ..Looks::default()
            };
            let f = c.forge("Creature", &looks).unwrap();
            let base = c.base(build).unwrap();
            let len = |lib: &mftr_pack::pose::Library, n: &str| lib.clip(n).map(|i| lib.clips[i].length);
            for (slot, own) in [("q", "bolt"), ("r", "beam")] {
                assert!(len(&base.library, own).is_some(), "{build} has no `{own}`");
                assert_eq!(len(&f.library, slot), len(&base.library, own), "{build}: {slot} plays `{own}`");
                assert_ne!(len(&base.library, own), len(&base.library, "howl"), "{build}: `{own}` is its own clip");
            }
        }
    }

    #[test]
    fn a_beast_forges_with_its_own_clips_for_any_kit() {
        let c = catalog();
        assert!(c.sliders("beast").len() >= 8);
        assert!(c.parts("beast").iter().any(|p| p.id == "back.saddle"));
        let looks = Looks {
            build: "beast".into(),
            gender: looks::Gender::None,
            outfit: vec!["shoulders.collar".into(), "head.horns".into()],
            // A projectile, a self heal, a dash and a delayed area: a howl, a howl, the dash set, a slam.
            moves: Some(moves::Moves {
                ids: ["rook.attack", "ember.q", "rook.w", "shade.e", "quill.r"].map(String::from),
            }),
            ..Looks::default()
        };
        let f = c.forge("Beastie", &looks).unwrap();
        let has = |n: &str| f.model.animations.iter().any(|a| a.name == n);
        for clip in
            ["idle", "run", "walk", "attack_1", "death", "q", "w", "e_start", "e_travel", "e_land", "r", "recall"]
        {
            assert!(has(clip), "no `{clip}`");
        }
        assert!(f.library.clip("q").is_some_and(|i| f.library.clips[i].fire() > 0.0), "q fires on its marker");
        assert!(f.vfx.is_some());
        let mut l2 = looks.clone();
        l2.set_slider("head_snout", 1.0);
        l2.proportions.set(0, 0.5);
        assert!(c.forge("Beastie", &l2).is_ok());
    }

    #[test]
    fn every_build_forges_any_kit_and_takes_its_parts() {
        let c = catalog();
        for (build, gendered) in looks::BUILDS {
            let key = if gendered { format!("{build}_masculine") } else { build.to_string() };
            assert!(c.base(&key).is_some(), "no base for {build}");
            let looks = Looks {
                build: build.to_string(),
                gender: if gendered { looks::Gender::Masculine } else { looks::Gender::None },
                // One part from each of its first two slots.
                outfit: {
                    let mut seen: Vec<&str> = Vec::new();
                    let mut out = Vec::new();
                    for p in c.parts(&key) {
                        if out.len() < 2 && !seen.contains(&p.slot.as_str()) {
                            seen.push(&p.slot);
                            out.push(p.id.clone());
                        }
                    }
                    out
                },
                moves: Some(moves::Moves {
                    ids: ["vesper.attack", "ember.q", "bastion.w", "wren.e", "quill.r"].map(String::from),
                }),
                ..Looks::default()
            };
            let f = c.forge("Anyone", &looks).unwrap_or_else(|e| panic!("{build}: {e}"));
            for clip in ["idle", "run", "q", "w", "r"] {
                assert!(
                    f.model.animations.iter().any(|a| a.name == clip || a.name.starts_with(&format!("{clip}_"))),
                    "{build}: no `{clip}`"
                );
            }
            for s in c.sliders(&key) {
                let mut l = looks.clone();
                l.set_slider(&s, 1.0);
                c.forge("Anyone", &l).unwrap_or_else(|e| panic!("{build} {s}: {e}"));
            }
        }
    }

    #[test]
    fn the_neo_set_matches_the_recipes_under_new_names() {
        // art/champions/neo: the official ten rebuilt, to play beside the hand-made ones.
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art/champions");
        let c = catalog();
        for champ in mftr_sim::ChampionId::ALL {
            let id = champ.def().name.to_lowercase();
            let neo = std::fs::read_to_string(art.join(format!("neo/{id}-neo.champion.ron"))).unwrap();
            let src = std::fs::read_to_string(art.join(format!("{id}/{id}.champion.ron"))).unwrap();
            let (n, r) = (recipe::Recipe::from_text(&neo).unwrap(), recipe::Recipe::from_text(&src).unwrap());
            assert_eq!(n.name, format!("{}-neo", champ.def().name));
            assert_eq!(n.looks, r.looks, "{id}: regenerate art/champions/neo from the recipe");
            assert_eq!(mftr_sim::champion::ChampionDef { name: champ.def().name, ..n.kit }, *champ.def(), "{id}");
            assert!(n.problems().is_empty(), "{id}: {:?}", n.problems());
            assert_eq!(mftr_pack::champion::validate_text(&neo).errors(), 0, "{id}");
            c.forge(&n.name, &n.looks).unwrap();
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
        // Faces −Z in Blender, +Z in glTF: the nose tip reaches further toward +Z. On a rigged
        // face the slider moves the nose's bones (FR3), so where the skin puts the vertices.
        let front = |m: &Model| face::skinned(m).iter().flatten().flatten().map(|v| v[2]).fold(f32::MIN, f32::max);
        let reach = front(&nosed.model) - front(&plain.model);
        assert!(reach > 0.01, "{reach}");
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
    fn a_recipe_from_before_the_face_rig_gets_its_face_from_the_bones() {
        // FR3: Cairn-neo's recipe was written for the face morphs (`brow_ridge: 1`, `nose_length:
        // 0.5`…). It loads unchanged, and its face sliders now move the face bones' rest.
        let art = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../art/champions/neo");
        let r =
            recipe::Recipe::from_text(&std::fs::read_to_string(art.join("cairn-neo.champion.ron")).unwrap()).unwrap();
        assert!(r.looks.face.is_none());
        let m = catalog().forge(&r.name, &r.looks).unwrap().model;
        let moved = |name: &str| {
            let i = m.node_by_name(name).unwrap();
            m.bind.get(i).copied().flatten().is_some_and(|b| b.0 != m.nodes[i].translation)
        };
        assert!(moved("face_brow_l") && moved("face_brow_r") && moved("face_nose_tip"));
        assert!(!moved("face_eye_l"), "no eye slider in the recipe");
    }

    #[test]
    fn a_new_body_takes_off_what_it_has_no_place_for() {
        // 21 §8 #1, #9: switching build kept the old body's parts, sliders, fills and sculpt;
        // the forge refused them and the preview stopped changing.
        let c = catalog();
        let mut l = Looks {
            outfit: vec!["torso.tunic".into(), "head.helmet".into()],
            paint: vec![("body".into(), 3, 18), ("torso.tunic".into(), 0, 5)],
            sculpt: vec![(10, [5, 0, 0])],
            ..Looks::default()
        };
        l.set_slider("nose_reach", 0.5);
        l.set_slider("body_bulk", 0.5);
        let was = l.base_key();
        l.build = "machine".into();
        let dropped = c.fit(&mut l, &was);
        assert_eq!(l.gender, looks::Gender::None);
        assert!(l.outfit.is_empty() && l.sculpt.is_empty(), "{l:?}");
        assert!(l.shape.iter().all(|(id, _)| c.sliders("machine").contains(id)));
        assert!(l.paint.iter().all(|(t, _, _)| t == "body"));
        assert!(!dropped.is_empty());
        c.forge("Cog", &l).unwrap();
        // The same body keeps everything.
        let mut same = Looks { outfit: vec!["torso.tunic".into()], sculpt: vec![(10, [5, 0, 0])], ..Looks::default() };
        let key = same.base_key();
        assert!(c.fit(&mut same, &key).is_empty());
        assert_eq!(same.sculpt.len(), 1);
    }

    #[test]
    fn long_legs_keep_the_feet_on_the_floor_in_the_clips() {
        // 21 §8 #8: the clips' pelvis keys were the library's scaled by √legs, below the
        // lifted rest: long legs stood the feet in the floor once the idle played.
        let c = catalog();
        for legs in [-1.0, 1.0] {
            let mut l = Looks::default();
            l.proportions.set(1, legs);
            let f = c.forge("Legs", &l).unwrap();
            let pelvis = f.model.node_by_name("pelvis").unwrap();
            let idle = f.model.animations.iter().find(|a| a.name == "idle").unwrap();
            let ch = idle.channels.iter().find(|ch| ch.node == pelvis && ch.path == "translation").unwrap();
            let rest = f.model.nodes[pelvis].translation[1];
            let first = ch.values[0][1];
            assert!((first - rest).abs() < 0.03, "legs {legs}: idle pelvis {first} vs rest {rest}");
        }
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
        // Within the bare champion's budget (22 §1.2: ~7,000 once the new body lands).
        assert!(plain.model.triangles() < 9000, "{}", plain.model.triangles());
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
        // The helmet is weighted to the face bones under it (FR2), so it widens with them; a
        // little, as the face widens more than the skull it covers.
        let width = |m: &Model| {
            let node = m.nodes.iter().find(|n| n.name == "part.head.helmet").unwrap();
            let xs: Vec<f32> = face::skinned(m)[node.mesh.unwrap()].iter().flatten().map(|v| v[0]).collect();
            xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min)
        };
        let gain = width(&wide.model) / width(&narrow.model);
        assert!(gain > 1.008, "{gain}");
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
