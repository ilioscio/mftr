//! Champion models from content packs (10 §6, 11 §4), on the Godot side.
//!
//! `MftrModel.load(path)` reads a pack through `mftr-pack` only: it is validated first, and
//! its bytes never reach Godot's `ResourceLoader` (11 §1). The mesh, skeleton and skin are
//! built here from the validated arrays. `MftrAnimator` then writes the poses `mftr-pack`'s
//! animator computes into the skeleton every frame. Sounds arrive the same way: `mftr-pack`
//! decodes the pack's Ogg files and Godot only ever sees the PCM (A4c).

use std::sync::Arc;

use godot::classes::audio_stream_wav::Format;
use godot::classes::image::Format as ImageFormat;
use godot::classes::mesh::{ArrayType, PrimitiveType};
use godot::classes::{
    ArrayMesh, AudioStreamWav, Image, ImageTexture, MeshInstance3D, Node3D, RefCounted, Skeleton3D, Skin,
};
use godot::prelude::*;
use mftr_pack::animator::{Action, ActionKind, AnimEvent, Animator, Drive, Phase};
use mftr_pack::pose::Trs;

#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrModel {
    base: Base<RefCounted>,
    pack: Arc<mftr_pack::Loaded>,
    /// The creator's preview: each vertex carries its face region in UV2 (21 §3.2), which the
    /// shader lights under the cursor. Game models don't.
    regions: bool,
}

/// The champion creator's catalog (CR2), loaded once from the art folder.
static CATALOG: std::sync::Mutex<Option<Arc<mftr_forge::Catalog>>> = std::sync::Mutex::new(None);

pub(crate) fn catalog(art: &str) -> Option<Arc<mftr_forge::Catalog>> {
    let mut c = CATALOG.lock().unwrap_or_else(|e| e.into_inner());
    if c.is_none() {
        match mftr_forge::Catalog::load(std::path::Path::new(art)) {
            Ok(cat) => *c = Some(Arc::new(cat)),
            Err(e) => {
                godot_error!("MFTR creator catalog: {e}");
                return None;
            }
        }
    }
    c.clone()
}

/// A forge's result once done: the pack, `None` for a file without looks, or the error.
type Forged = Result<Option<Arc<mftr_pack::Loaded>>, String>;

/// One champion's forge: its state (`FORGE_*`) and, once done, its result.
pub(crate) struct ForgeSlot {
    state: std::sync::atomic::AtomicU8,
    result: std::sync::Mutex<Option<Forged>>,
    done: std::sync::Condvar,
}

/// Looking for it in the cache on disk.
const FORGE_CHECKING: u8 = 1;
/// Not in the cache (or stale): forging it, seconds.
const FORGE_BUILDING: u8 = 2;
const FORGE_DONE: u8 = 3;

/// Forges started, by champion and champion file (a custom champion's file can differ from one
/// server to the next).
type Forges = std::collections::HashMap<(String, String), Arc<ForgeSlot>>;
static FORGES: std::sync::Mutex<Option<Forges>> = std::sync::Mutex::new(None);

/// Where forged models are kept between launches (`MftrModel.set_forge_cache`); none: not kept.
static FORGE_CACHE: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

/// The art a forge reads, fingerprinted once per run (see `art_fingerprint`).
static ART_PRINT: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// The slot `champion`'s forge fills, starting it on a worker thread the first time; `None`
/// with no champion file to forge from.
fn start_forge(art: &str, champion: &str) -> Option<Arc<ForgeSlot>> {
    let text = mftr_client::champion_file(champion)
        .or_else(|| mftr_forge::official_recipe(std::path::Path::new(art), champion))?;
    let mut all = FORGES.lock().unwrap_or_else(|e| e.into_inner());
    let key = (champion.to_string(), text.clone());
    if let Some(slot) = all.get_or_insert_with(Default::default).get(&key) {
        return Some(slot.clone());
    }
    let slot = Arc::new(ForgeSlot {
        state: std::sync::atomic::AtomicU8::new(FORGE_CHECKING),
        result: std::sync::Mutex::new(None),
        done: std::sync::Condvar::new(),
    });
    all.as_mut().expect("made above").insert(key, slot.clone());
    let (art, name, out) = (art.to_string(), champion.to_string(), slot.clone());
    let cache = FORGE_CACHE.lock().unwrap_or_else(|e| e.into_inner()).clone();
    std::thread::spawn(move || {
        let r = forge_kept(&art, &name, &text, cache.as_deref(), &out);
        *out.result.lock().unwrap_or_else(|e| e.into_inner()) = Some(r);
        out.state.store(FORGE_DONE, std::sync::atomic::Ordering::Release);
        out.done.notify_all();
    });
    Some(slot)
}

/// The forge, through the cache: a kept model built from the same code, art and recipe is
/// read back; otherwise it's forged and kept for next time.
fn forge_kept(art: &str, champion: &str, text: &str, cache: Option<&std::path::Path>, slot: &ForgeSlot) -> Forged {
    let Some(dir) = cache else {
        slot.state.store(FORGE_BUILDING, std::sync::atomic::Ordering::Release);
        return forge_now(art, champion, text);
    };
    let print = ART_PRINT.get_or_init(|| art_fingerprint(std::path::Path::new(art)));
    let recipe = mftr_pack::cache::fnv1a64(text.as_bytes());
    let key = format!("{}|{print}|{recipe:016x}", env!("MFTR_FORGE_BUILD"));
    // One file per champion and recipe: a custom champion of the same name doesn't evict it.
    let stem = format!("{}-{recipe:016x}", champion.to_lowercase());
    let path = dir.join(format!("{stem}.forged"));
    if let Ok(bytes) = std::fs::read(&path)
        && let Ok(pack) = mftr_pack::cache::read(&bytes, &key)
    {
        return Ok(Some(Arc::new(pack)));
    }
    slot.state.store(FORGE_BUILDING, std::sync::atomic::Ordering::Release);
    let r = forge_now(art, champion, text);
    if let Ok(Some(pack)) = &r {
        // Written aside, then moved over: a half-written file is never read as a whole one.
        let tmp = dir.join(format!("{stem}.forged.tmp"));
        let _ = std::fs::create_dir_all(dir)
            .and_then(|_| std::fs::write(&tmp, mftr_pack::cache::write(pack, &key)))
            .and_then(|_| std::fs::rename(&tmp, &path));
    }
    r
}

/// The art the forge reads (the biped library, the creator bases and their parts, the
/// champions' move packs): every file's path, size and time, hashed. A self-update replaces
/// the art folder, so it changes this and the champions are rebuilt once.
fn art_fingerprint(art: &std::path::Path) -> String {
    fn walk(dir: &std::path::Path, out: &mut Vec<(String, u64, u128)>, art: &std::path::Path) {
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out, art);
                continue;
            }
            // Sources (Blender files, build scripts, notes) aren't what the game reads.
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(ext, "blend" | "blend1" | "py" | "md") {
                continue;
            }
            let Ok(m) = e.metadata() else { continue };
            let t = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_nanos());
            let rel = p.strip_prefix(art).unwrap_or(&p).to_string_lossy().replace('\\', "/");
            out.push((rel, m.len(), t));
        }
    }
    let mut all = Vec::new();
    for d in ["library", "creator", "wardrobe", "champions"] {
        walk(&art.join(d), &mut all, art);
    }
    all.sort();
    let mut bytes = Vec::new();
    for (p, len, t) in &all {
        bytes.extend_from_slice(p.as_bytes());
        bytes.extend_from_slice(&len.to_le_bytes());
        bytes.extend_from_slice(&t.to_le_bytes());
    }
    format!("{:016x}", mftr_pack::cache::fnv1a64(&bytes))
}

fn forge_now(art: &str, champion: &str, text: &str) -> Forged {
    let Some(looks) = mftr_forge::looks::of_champion_file(text).map_err(|e| format!("its looks: {e}"))? else {
        return Ok(None);
    };
    let cat = catalog_quiet(art)?;
    cat.forge(champion, &looks).map(|p| Some(Arc::new(p)))
}

/// `catalog` for a worker thread: the error comes back instead of going to Godot's log.
fn catalog_quiet(art: &str) -> Result<Arc<mftr_forge::Catalog>, String> {
    let mut c = CATALOG.lock().unwrap_or_else(|e| e.into_inner());
    if c.is_none() {
        let cat = mftr_forge::Catalog::load(std::path::Path::new(art)).map_err(|e| format!("catalog: {e}"))?;
        *c = Some(Arc::new(cat));
    }
    Ok(c.clone().expect("loaded above"))
}

impl MftrModel {
    /// A model of a pack built here (the creator's preview, CR3).
    pub(crate) fn from_pack(pack: Arc<mftr_pack::Loaded>) -> Gd<MftrModel> {
        Gd::from_init_fn(|base| MftrModel { base, pack, regions: true })
    }
}

#[godot_api]
impl MftrModel {
    /// Check and register a champion file from disk (`--champion-file`, the creator): its
    /// champion's name, or "" if it was refused (the reason goes to the error log).
    #[func]
    fn load_champion_file(path: GString) -> GString {
        let path = path.to_string();
        match std::fs::read_to_string(&path)
            .map_err(|e| e.to_string())
            .and_then(|t| mftr_client::load_champion_file(&t))
        {
            Ok(id) => GString::from(id.def().name),
            Err(e) => {
                godot_error!("MFTR champion file {path} refused: {e}");
                GString::new()
            }
        }
    }

    /// Forge the model of `champion` from its champion file's looks and the catalog under `art`
    /// (CR2, `mftr-forge`): a server's custom champion's file, else an official champion's recipe
    /// (`art/champions/<id>/<id>.champion.ron`, the redesigns); `null` with no file or no looks.
    /// Forging takes seconds, so it runs on a worker thread and is kept: one already started
    /// (`forge_start`) is waited for here, only for what's left of it.
    #[func]
    fn forge(art: GString, champion: GString) -> Option<Gd<MftrModel>> {
        let slot = start_forge(&art.to_string(), &champion.to_string())?;
        let mut r = slot.result.lock().unwrap_or_else(|e| e.into_inner());
        while r.is_none() {
            r = slot.done.wait(r).unwrap_or_else(|e| e.into_inner());
        }
        match r.as_ref().expect("waited for") {
            Ok(Some(pack)) => {
                let pack = pack.clone();
                Some(Gd::from_init_fn(|base| MftrModel { base, pack, regions: false }))
            }
            Ok(None) => None,
            Err(e) => {
                godot_error!("MFTR {champion} couldn't be forged: {e}");
                None
            }
        }
    }

    /// Start forging `champion` on a worker thread (see `forge`), if it has looks to forge
    /// and isn't started yet: the client does this for the whole roster as it opens.
    #[func]
    fn forge_start(art: GString, champion: GString) {
        start_forge(&art.to_string(), &champion.to_string());
    }

    /// Whether `forge` would return at once: the forge is done, or there's nothing to forge.
    #[func]
    fn forge_ready(art: GString, champion: GString) -> bool {
        matches!(Self::forge_state(art, champion), 0 | 3)
    }

    /// Where `champion`'s forge is: 0 nothing to forge, 1 looking in the cache, 2 forging
    /// (not kept, or kept from other code, art or recipe), 3 done. The launch screen shows
    /// while any is 2.
    #[func]
    fn forge_state(art: GString, champion: GString) -> i64 {
        match start_forge(&art.to_string(), &champion.to_string()) {
            None => 0,
            Some(slot) => slot.state.load(std::sync::atomic::Ordering::Acquire) as i64,
        }
    }

    /// Keep forged models in `dir` between launches (the player's data folder), from now on.
    #[func]
    fn set_forge_cache(dir: GString) {
        *FORGE_CACHE.lock().unwrap_or_else(|e| e.into_inner()) = Some(std::path::PathBuf::from(dir.to_string()));
    }

    /// Loads and validates `<id>.glb` and its `<id>.anims.ron` (an absolute path). `null` if it
    /// fails validation or can't be read (the reason goes to the error log).
    #[func]
    fn load(path: GString) -> Option<Gd<MftrModel>> {
        match mftr_pack::load_file(std::path::Path::new(&path.to_string())) {
            Ok(pack) => Some(Gd::from_init_fn(|base| MftrModel { base, pack: Arc::new(pack), regions: false })),
            Err(e) => {
                godot_error!("MFTR model refused: {e}");
                None
            }
        }
    }

    /// Triangles drawn (the creator's budget meter, 20 §7.2).
    #[func]
    fn triangles(&self) -> i64 {
        self.pack.model.triangles() as i64
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

    /// The paint image of surface `i` (CR5b, D70: a forged champion's brush strokes and
    /// decals), for the shader's `paint`; `null` for surfaces without one.
    #[func]
    fn surface_paint(&self, i: i64) -> Option<Gd<ImageTexture>> {
        let prim = self.pack.model.meshes.iter().flatten().nth(usize::try_from(i).ok()?)?;
        let p = prim.paint.as_ref()?;
        let data = PackedByteArray::from(p.rgba.as_slice());
        let img = Image::create_from_data(p.size as i32, p.size as i32, false, ImageFormat::RGBA8, &data)?;
        ImageTexture::create_from_image(&img)
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
        // The skin binds each bone where the mesh was built around it: its rest, except a face
        // bone whose rest carries a sculpted face (21 §2.3), which binds where it was before.
        // So the face shows at rest and in every pose, with no mesh rebuilt.
        let mut global: Vec<Transform3D> = Vec::with_capacity(lib.rig.bind.len());
        for (i, b) in lib.rig.bind.iter().enumerate() {
            let local = transform(b);
            let g = match lib.rig.parents[i] {
                Some(p) => global[p] * local,
                None => local,
            };
            global.push(g);
        }
        let mut skin = Skin::new_gd();
        for (i, g) in global.iter().enumerate() {
            skin.add_bind(i as i32, g.affine_inverse());
        }
        let skin = Some(skin);

        let mesh = self.build_mesh(true);
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

    /// A static copy of the mesh, without bone weights (map props: drawn in batches with
    /// `MultiMeshInstance3D`, they have nothing to animate).
    #[func]
    fn static_mesh(&self) -> Gd<ArrayMesh> {
        self.build_mesh(false)
    }

    /// The spring-bone chains (22 §4.5, BW6): a champion's `extra_<chain>_<n>` chains that no
    /// clip keys (a forged champion's hair and cloth; an official champion's cape is baked into
    /// its clips, and a siege cart's barrel is no cloth), as `[chain, root bone, end bone]`.
    #[func]
    fn spring_chains(&self) -> VarArray {
        if self.pack.kind != "champion" {
            return VarArray::new();
        }
        let m = &self.pack.model;
        let keyed: std::collections::HashSet<&str> = m
            .animations
            .iter()
            .flat_map(|a| a.channels.iter())
            .filter_map(|c| m.nodes.get(c.node).map(|n| n.name.as_str()))
            .collect();
        let mut chains: std::collections::BTreeMap<&str, (u32, u32, bool)> = Default::default();
        for name in &self.pack.library.rig.names {
            let Some((chain, n)) = name.strip_prefix("extra_").and_then(|r| r.rsplit_once('_')) else { continue };
            let Ok(n) = n.parse::<u32>() else { continue };
            let e = chains.entry(chain).or_insert((u32::MAX, 0, false));
            e.0 = e.0.min(n);
            e.1 = e.1.max(n);
            e.2 |= keyed.contains(name.as_str());
        }
        // A chain is a line, each bone under the one before: a bow's limbs (`extra_bow_1..4`, two
        // limbs off the hand) and its string aren't, and swing only with the clips that draw them.
        let line = |chain: &str, first: u32, last: u32| {
            (first + 1..=last).all(|n| {
                m.node_by_name(&format!("extra_{chain}_{n}"))
                    .and_then(|i| m.nodes[i].parent)
                    .is_some_and(|p| m.nodes[p].name == format!("extra_{chain}_{}", n - 1))
            })
        };
        let mut out = VarArray::new();
        for (chain, (first, last, keyed)) in chains {
            if keyed || first == u32::MAX || !line(chain, first, last) {
                continue;
            }
            let mut row = VarArray::new();
            row.push(&GString::from(chain).to_variant());
            row.push(&GString::from(format!("extra_{chain}_{first}").as_str()).to_variant());
            row.push(&GString::from(format!("extra_{chain}_{last}").as_str()).to_variant());
            out.push(&row.to_variant());
        }
        out
    }

    /// The sidecar's kind: `champion`, `library`, `minion`, `rig` or `prop`.
    #[func]
    fn kind(&self) -> GString {
        GString::from(self.pack.kind.as_str())
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
            d.set("arc", e.arc.unwrap_or(140.0));
            d.set("tilt", e.tilt.unwrap_or(0.0));
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
        let mut animator = Animator::new(&self.pack.library);
        // Minions' and monsters' attacks have no timing on the wire: they play out whole.
        animator.finish_attacks = matches!(self.pack.kind.as_str(), "minion" | "monster");
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
    /// An instant cast in `slot` (0 = q … 5 = f) or `-1` for an attack: play its clip from `fire`
    /// (A6; the drive never shows casts without a windup).
    #[func]
    fn pulse(&mut self, slot: i64) {
        let kind = if slot < 0 { ActionKind::Attack { variant: 1 } } else { ActionKind::Cast { slot: slot as u8 } };
        self.animator.pulse(&self.pack.library, kind);
    }

    /// Pose `skeleton` as clip `name` is at `time` seconds (reviews and lineups); false if the
    /// pack has no such clip.
    #[func]
    fn pose_clip(&mut self, mut skeleton: Gd<Skeleton3D>, name: GString, time: f32) -> bool {
        let lib = &self.pack.library;
        let Some(clip) = lib.clip(&name.to_string()) else { return false };
        for (i, p) in lib.sample(clip, time).iter().enumerate() {
            let i = i as i32;
            skeleton.set_bone_pose_position(i, Vector3::new(p.t[0], p.t[1], p.t[2]));
            skeleton.set_bone_pose_rotation(i, Quaternion::new(p.r[0], p.r[1], p.r[2], p.r[3]));
            skeleton.set_bone_pose_scale(i, Vector3::new(p.s[0], p.s[1], p.s[2]));
        }
        true
    }

    /// Pose `skeleton` as clip `a` at `time_a` crossfading into clip `b` at `time_b` by `w`
    /// (0 = all `a`, 1 = all `b`): characters the client drives itself, like the shopkeepers
    /// (19 §4), moving from one clip into the next. False if either clip is missing.
    #[func]
    fn pose_mix(
        &mut self,
        mut skeleton: Gd<Skeleton3D>,
        a: GString,
        time_a: f32,
        b: GString,
        time_b: f32,
        w: f32,
    ) -> bool {
        let lib = &self.pack.library;
        let (Some(ca), Some(cb)) = (lib.clip(&a.to_string()), lib.clip(&b.to_string())) else { return false };
        let mut pose = lib.sample(ca, time_a);
        mftr_pack::pose::blend(&mut pose, &lib.sample(cb, time_b), w.clamp(0.0, 1.0), None);
        for (i, p) in pose.iter().enumerate() {
            let i = i as i32;
            skeleton.set_bone_pose_position(i, Vector3::new(p.t[0], p.t[1], p.t[2]));
            skeleton.set_bone_pose_rotation(i, Quaternion::new(p.r[0], p.r[1], p.r[2], p.r[3]));
            skeleton.set_bone_pose_scale(i, Vector3::new(p.s[0], p.s[1], p.s[2]));
        }
        true
    }

    /// Clip `name`'s length in seconds (-1 if the pack lacks it).
    #[func]
    fn clip_length(&self, name: GString) -> f32 {
        let lib = &self.pack.library;
        lib.clip(&name.to_string()).map_or(-1.0, |c| lib.clips[c].length)
    }

    /// Advance `dt` seconds and pose `skeleton`. `unit` is an `own_status()` or
    /// `remote_units()` entry (`dead`, `stunned`, `rooted`, `dashing` and the `anim_*` keys);
    /// `speed` is the displayed ground speed in u/s. Returns what happened, for sounds (A4c):
    /// `"foot"` for a footstep, `"attack"` or a slot's action (`"q"` … `"f"`) for a windup's start,
    /// `"fire:<action>"` when that action passes its `fire` marker.
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
            dash_slot: (int("dash_slot") >= 0 && unit.contains_key("dash_slot")).then(|| int("dash_slot") as u8),
            stunned: flag("stunned"),
            rooted: flag("rooted"),
            dead: flag("dead"),
            hit: flag("hit"),
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
        let action = |k: &ActionKind| match k {
            ActionKind::Attack { .. } => Some("attack"),
            ActionKind::Cast { slot } => SLOTS.get(*slot as usize).copied(),
            ActionKind::Continue => None,
        };
        self.animator
            .events
            .iter()
            .filter_map(|e| match e {
                AnimEvent::Foot => Some("foot".to_string()),
                AnimEvent::Start(k) => action(k).map(str::to_string),
                AnimEvent::Fire(k) => action(k).map(|a| format!("fire:{a}")),
            })
            .map(|s| GString::from(s.as_str()))
            .collect()
    }
}

impl MftrModel {
    /// The mesh with its vertex colors and one surface per material slot (`surface_slots`);
    /// `skinned` adds the bone weights for the skeleton `instantiate` builds.
    fn build_mesh(&self, skinned: bool) -> Gd<ArrayMesh> {
        let lib = &self.pack.library;
        let mut mesh = ArrayMesh::new_gd();
        // Region k (face bone k - 1, 0 = none) as UV2.x = k / 256: the shader reads it back.
        let regions = if self.regions { mftr_forge::face::vertex_regions(&self.pack.model) } else { Vec::new() };
        let region_of = regions.iter().flatten();
        for (prim, reg) in
            self.pack.model.meshes.iter().flatten().zip(region_of.map(Some).chain(std::iter::repeat(None)))
        {
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
            if prim.uvs.len() == n {
                let uvs: PackedVector2Array = prim.uvs.iter().map(|u| Vector2::new(u[0], u[1])).collect();
                arrays.set(ArrayType::TEX_UV.ord() as usize, &uvs.to_variant());
            }
            if let Some(r) = reg.filter(|r| r.len() == n && r.iter().any(|&x| x > 0)) {
                let uv2: PackedVector2Array = r.iter().map(|&x| Vector2::new(x as f32 / 256.0, 0.0)).collect();
                arrays.set(ArrayType::TEX_UV2.ord() as usize, &uv2.to_variant());
            }
            if skinned && prim.joints.len() == n && prim.weights.len() == n {
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
        mesh
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
