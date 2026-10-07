//! Champion models from content packs (10 §6, 11 §4), on the Godot side.
//!
//! `MftrModel.load(path)` reads a pack through `mftr-pack` only: it is validated first, and
//! its bytes never reach Godot's `ResourceLoader` (11 §1). The mesh, skeleton and skin are
//! built here from the validated arrays. `MftrAnimator` then writes the poses `mftr-pack`'s
//! animator computes into the skeleton every frame. Sounds arrive the same way: `mftr-pack`
//! decodes the pack's Ogg files and Godot only ever sees the PCM (A4c).

use std::sync::Arc;

use godot::classes::audio_stream_wav::Format;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{ArrayMesh, AudioStreamWav, MeshInstance3D, Node3D, RefCounted, Skeleton3D};
use godot::prelude::*;
use mftr_pack::animator::{Action, ActionKind, AnimEvent, Animator, Drive, Phase};
use mftr_pack::pose::Trs;

#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrModel {
    base: Base<RefCounted>,
    pack: Arc<mftr_pack::Loaded>,
}

#[godot_api]
impl MftrModel {
    /// Loads and validates `<id>.glb` and its `<id>.anims.ron` (an absolute path). `null` if it
    /// fails validation or can't be read (the reason goes to the error log).
    #[func]
    fn load(path: GString) -> Option<Gd<MftrModel>> {
        match mftr_pack::load_file(std::path::Path::new(&path.to_string())) {
            Ok(pack) => Some(Gd::from_init_fn(|base| MftrModel { base, pack: Arc::new(pack) })),
            Err(e) => {
                godot_error!("MFTR model refused: {e}");
                None
            }
        }
    }

    /// The material slot of each surface of the mesh `instantiate` builds, in order.
    #[func]
    fn surface_slots(&self) -> PackedStringArray {
        let m = &self.pack.model;
        m.meshes
            .iter()
            .flatten()
            .map(|p| GString::from(p.material.and_then(|i| m.materials.get(i)).map_or("cloth", |s| s.as_str())))
            .collect()
    }

    /// A new `Node3D` with a `Skeleton3D` ("Skeleton", in the rest pose) and the skinned mesh
    /// under it ("Mesh"). Materials are left to the caller (`surface_slots`).
    #[func]
    fn instantiate(&self) -> Gd<Node3D> {
        let lib = &self.pack.library;
        let mut skeleton = Skeleton3D::new_alloc();
        skeleton.set_name("Skeleton");
        for (i, name) in lib.rig.names.iter().enumerate() {
            skeleton.add_bone(name.as_str());
            let rest = transform(&lib.rig.rest[i]);
            skeleton.set_bone_rest(i as i32, rest);
        }
        for (i, parent) in lib.rig.parents.iter().enumerate() {
            if let Some(p) = parent {
                skeleton.set_bone_parent(i as i32, *p as i32);
            }
        }
        skeleton.reset_bone_poses();
        let skin = skeleton.create_skin_from_rest_transforms();

        let mut mesh = ArrayMesh::new_gd();
        for prim in self.pack.model.meshes.iter().flatten() {
            let n = prim.positions.len();
            let normals = smooth_normals(&prim.positions, &prim.indices);
            let mut arrays = VarArray::new();
            arrays.resize(ArrayType::MAX.ord() as usize, &Variant::nil());
            let verts: PackedVector3Array = prim.positions.iter().map(|p| Vector3::new(p[0], p[1], p[2])).collect();
            let norms: PackedVector3Array = normals.iter().map(|p| Vector3::new(p[0], p[1], p[2])).collect();
            arrays.set(ArrayType::VERTEX.ord() as usize, &verts.to_variant());
            arrays.set(ArrayType::NORMAL.ord() as usize, &norms.to_variant());
            if prim.colors.len() == n {
                let colors: PackedColorArray =
                    prim.colors.iter().map(|c| Color::from_rgba(c[0], c[1], c[2], c[3])).collect();
                arrays.set(ArrayType::COLOR.ord() as usize, &colors.to_variant());
            }
            if prim.joints.len() == n && prim.weights.len() == n {
                let map = &lib.joint_to_bone;
                let bones: PackedInt32Array = prim
                    .joints
                    .iter()
                    .flat_map(|j| j.map(|x| map.get(x as usize).copied().unwrap_or(0) as i32))
                    .collect();
                let weights: PackedFloat32Array = prim.weights.iter().flat_map(|w| *w).collect();
                arrays.set(ArrayType::BONES.ord() as usize, &bones.to_variant());
                arrays.set(ArrayType::WEIGHTS.ord() as usize, &weights.to_variant());
            }
            // glTF fronts are counter-clockwise; Godot's are clockwise.
            let index: PackedInt32Array =
                prim.indices.chunks(3).flat_map(|t| [t[0] as i32, t[2] as i32, t[1] as i32]).collect();
            arrays.set(ArrayType::INDEX.ord() as usize, &index.to_variant());
            mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
        }
        let mut mesh_node = MeshInstance3D::new_alloc();
        mesh_node.set_name("Mesh");
        mesh_node.set_mesh(&mesh);
        if let Some(skin) = skin {
            mesh_node.set_skin(&skin);
        }
        mesh_node.set_skeleton_path("..");
        skeleton.add_child(&mesh_node);

        let mut root = Node3D::new_alloc();
        root.set_name("Model");
        root.add_child(&skeleton);
        root
    }

    /// The pack's VFX (A4b, `<id>.vfx.ron`): `[{ event, kit, ramp: PackedColorArray, count, size,
    /// speed, lifetime }]`, with -1 for knobs the pack leaves to the kit's defaults.
    #[func]
    fn vfx(&self) -> VarArray {
        let mut out = VarArray::new();
        for e in self.pack.vfx.iter().flat_map(|f| &f.effects) {
            let mut d = VarDictionary::new();
            d.set("event", e.event.as_str());
            d.set("kit", e.kit.as_str());
            let ramp: PackedColorArray = e.ramp.iter().map(|&(r, g, b)| Color::from_rgb(r, g, b)).collect();
            d.set("ramp", &ramp);
            d.set("count", e.count.map_or(-1, |c| c as i64));
            d.set("size", e.size.unwrap_or(-1.0));
            d.set("speed", e.speed.unwrap_or(-1.0));
            d.set("lifetime", e.lifetime.unwrap_or(-1.0));
            out.push(&d.to_variant());
        }
        out
    }

    /// The pack's sounds (A4c, `<id>.sfx.ron` and `sfx/*.ogg`, decoded by `mftr-pack`):
    /// `[{ name, events: PackedStringArray, volume, pitch, stream: AudioStreamWAV }]`.
    #[func]
    fn sounds(&self) -> VarArray {
        let mut out = VarArray::new();
        for s in &self.pack.sounds {
            let mut stream = AudioStreamWav::new_gd();
            stream.set_format(Format::FORMAT_16_BITS);
            stream.set_mix_rate(s.rate as i32);
            stream.set_stereo(s.channels == 2);
            let bytes: PackedByteArray = s.pcm.iter().flat_map(|v| v.to_le_bytes()).collect();
            stream.set_data(&bytes);
            let mut d = VarDictionary::new();
            d.set("name", s.spec.name.as_str());
            let events: PackedStringArray = s.spec.events.iter().map(|e| GString::from(e.as_str())).collect();
            d.set("events", &events);
            d.set("volume", s.spec.volume);
            d.set("pitch", s.spec.pitch);
            d.set("stream", &stream);
            out.push(&d.to_variant());
        }
        out
    }

    /// A new animator for one instance of this model.
    #[func]
    fn new_animator(&self) -> Gd<MftrAnimator> {
        let animator = Animator::new(&self.pack.library);
        Gd::from_init_fn(|base| MftrAnimator { base, pack: self.pack.clone(), animator })
    }
}

/// Drives one model's skeleton from simulation state (10 §6).
#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrAnimator {
    base: Base<RefCounted>,
    pack: Arc<mftr_pack::Loaded>,
    animator: Animator,
}

#[godot_api]
impl MftrAnimator {
    /// Advance `dt` seconds and pose `skeleton`. `unit` is an `own_status()` or
    /// `remote_units()` entry (`dead`, `stunned`, `rooted`, `dashing` and the `anim_*` keys);
    /// `speed` is the displayed ground speed in u/s. Returns what happened, for sounds (A4c):
    /// `"foot"` for a footstep, `"attack"` or a slot's action (`"q"` … `"f"`) for a windup's start.
    #[func]
    fn drive(&mut self, mut skeleton: Gd<Skeleton3D>, unit: VarDictionary, speed: f32, dt: f32) -> PackedStringArray {
        let flag = |k: &str| unit.get(k).and_then(|v| v.try_to::<bool>().ok()).unwrap_or(false);
        let int = |k: &str| unit.get(k).and_then(|v| v.try_to::<i64>().ok()).unwrap_or(0);
        let text =
            |k: &str| unit.get(k).and_then(|v| v.try_to::<GString>().ok()).map(|s| s.to_string()).unwrap_or_default();
        let kind = match text("anim_action").as_str() {
            "attack" => Some(ActionKind::Attack { variant: int("anim_variant") as u8 }),
            "cast" => Some(ActionKind::Cast { slot: int("anim_slot") as u8 }),
            "continue" => Some(ActionKind::Continue),
            _ => None,
        };
        let progress = unit.get("anim_progress").and_then(|v| v.try_to::<f64>().ok()).filter(|p| *p >= 0.0);
        let phase = if text("anim_phase") == "follow" { Phase::FollowThrough } else { Phase::Windup };
        let drive = Drive {
            dt,
            speed,
            dashing: flag("dashing"),
            stunned: flag("stunned"),
            rooted: flag("rooted"),
            dead: flag("dead"),
            action: kind.map(|kind| Action { kind, phase, progress: progress.map(|p| p as f32) }),
        };
        let pose = self.animator.update(&self.pack.library, &drive);
        for (i, p) in pose.iter().enumerate() {
            let i = i as i32;
            skeleton.set_bone_pose_position(i, Vector3::new(p.t[0], p.t[1], p.t[2]));
            skeleton.set_bone_pose_rotation(i, Quaternion::new(p.r[0], p.r[1], p.r[2], p.r[3]));
            skeleton.set_bone_pose_scale(i, Vector3::new(p.s[0], p.s[1], p.s[2]));
        }
        const SLOTS: [&str; 6] = ["q", "w", "e", "r", "d", "f"];
        self.animator
            .events
            .iter()
            .filter_map(|e| match e {
                AnimEvent::Foot => Some("foot"),
                AnimEvent::Start(ActionKind::Attack { .. }) => Some("attack"),
                AnimEvent::Start(ActionKind::Cast { slot }) => SLOTS.get(*slot as usize).copied(),
                AnimEvent::Start(ActionKind::Continue) => None,
            })
            .map(GString::from)
            .collect()
    }
}

fn transform(t: &Trs) -> Transform3D {
    let basis = Basis::from_quaternion(Quaternion::new(t.r[0], t.r[1], t.r[2], t.r[3]))
        .scaled(Vector3::new(t.s[0], t.s[1], t.s[2]));
    Transform3D::new(basis, Vector3::new(t.t[0], t.t[1], t.t[2]))
}

/// Area-weighted vertex normals. Shading stays faceted (derivatives in the shader); these only
/// grow the outline hull.
fn smooth_normals(pos: &[[f32; 3]], idx: &[u32]) -> Vec<[f32; 3]> {
    let mut n = vec![[0.0f32; 3]; pos.len()];
    for t in idx.chunks(3) {
        let (a, b, c) = (pos[t[0] as usize], pos[t[1] as usize], pos[t[2] as usize]);
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        let f = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
        for &i in t {
            for k in 0..3 {
                n[i as usize][k] += f[k];
            }
        }
    }
    for v in &mut n {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        if l > 0.0 {
            *v = [v[0] / l, v[1] / l, v[2] / l];
        } else {
            *v = [0.0, 1.0, 0.0];
        }
    }
    n
}
