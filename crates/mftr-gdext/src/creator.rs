//! The champion creator (CR3, 20 §2) on the Godot side: `MftrCreator` holds the champion being
//! made (a `mftr_forge::recipe::Recipe`), changes it as the screen's controls move, forges its
//! preview, and saves and loads its champion file. The screen itself is `creator.gd`.

use std::sync::Arc;

use godot::classes::Skeleton3D;
use godot::prelude::*;
use mftr_forge::face::{BoneSet, FaceSculpt, Frame, Sculpt};
use mftr_forge::looks::{Gender, Looks, Proportions, SKIN_TONES};
use mftr_forge::recipe::{self, Recipe};
use mftr_forge::{body, brows, eyes, face, paint};
use mftr_sim::ChampionId;

use crate::model::{MftrModel, catalog};

/// The slots grown by grooming (22 §7 item 5): their parts' color is the hair's.
const GROWN: [&str; 2] = ["hair", "beard"];

#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrCreator {
    base: Base<RefCounted>,
    recipe: Recipe,
    /// The art folder (the catalog's), set by `set_art`.
    art: String,
    /// The last preview forged: what the brush paints on (CR5b).
    last: Option<Arc<mftr_pack::Loaded>>,
    /// The last preview where the skin puts it (its face bones moved, FR3): what the brush and
    /// the sculpt aim at. None when nothing is moved (the preview's own model, then).
    aim: Option<Arc<mftr_pack::glb::Model>>,
    /// Why the last preview couldn't be forged, or what a change of body dropped: for the
    /// screen's status line ("" when nothing).
    note: String,
    /// The last preview's regions (`face::vertex_regions`): the face's, then the body's.
    regions: Vec<Vec<Vec<u8>>>,
    /// The face editor (FR4, 21 §2.4–3.3) and the body's (FR6, 21 §9): the same drag on two
    /// sets of bones.
    face: Editor<FaceSculpt>,
    body: Editor<body::BodySculpt>,
}

/// One set of bones' editor: the last preview's bone frames (None when the base has none of the
/// set's bones), the bone under the cursor and the point there, the drag under way, and undo.
struct Editor<S> {
    frames: Option<Vec<Frame>>,
    hover: Option<usize>,
    anchor: [f32; 3],
    drag: Option<(usize, face::DragMode, bool)>,
    undo: Vec<Option<S>>,
}

impl<S> Default for Editor<S> {
    fn default() -> Self {
        Editor { frames: None, hover: None, anchor: [0.0; 3], drag: None, undo: Vec::new() }
    }
}

/// A sculpt the creator edits by dragging: where the recipe keeps it, where its regions start
/// among the preview's, and which editor is its own.
trait Edited: Sculpt + 'static {
    /// Its first region among `face::vertex_regions`' (the shader's region ids).
    const BASE: usize;
    fn of(looks: &Looks) -> &Option<Self>;
    fn of_mut(looks: &mut Looks) -> &mut Option<Self>;
    fn editor(c: &MftrCreator) -> &Editor<Self>;
    fn editor_mut(c: &mut MftrCreator) -> &mut Editor<Self>;
    /// What the forge bakes: the recipe's own sculpt with its sliders' presets.
    fn shown(looks: &Looks) -> Option<Self>;
}

impl Edited for FaceSculpt {
    const BASE: usize = 0;

    fn of(looks: &Looks) -> &Option<Self> {
        &looks.face
    }

    fn of_mut(looks: &mut Looks) -> &mut Option<Self> {
        &mut looks.face
    }

    fn editor(c: &MftrCreator) -> &Editor<Self> {
        &c.face
    }

    fn editor_mut(c: &mut MftrCreator) -> &mut Editor<Self> {
        &mut c.face
    }

    fn shown(looks: &Looks) -> Option<Self> {
        face::compose(looks.face.as_ref(), &looks.shape)
    }
}

impl Edited for body::BodySculpt {
    const BASE: usize = face::BODY_REGIONS;

    fn of(looks: &Looks) -> &Option<Self> {
        &looks.body
    }

    fn of_mut(looks: &mut Looks) -> &mut Option<Self> {
        &mut looks.body
    }

    fn editor(c: &MftrCreator) -> &Editor<Self> {
        &c.body
    }

    fn editor_mut(c: &mut MftrCreator) -> &mut Editor<Self> {
        &mut c.body
    }

    fn shown(looks: &Looks) -> Option<Self> {
        body::compose(looks.body.as_ref(), &looks.shape)
    }
}

/// The set whose regions hold region id `region` (the face's first, then the body's).
fn set_of(region: i64) -> Option<(&'static BoneSet, usize)> {
    let r = usize::try_from(region).ok()?;
    if r < face::BODY_REGIONS {
        Some((&face::FACE, r))
    } else {
        (r - face::BODY_REGIONS < body::BONES).then_some((&body::BODY, r - face::BODY_REGIONS))
    }
}

#[godot_api]
impl MftrCreator {
    /// A new champion: the defaults (a slim masculine base, Rook's kit).
    #[func]
    fn create() -> Gd<MftrCreator> {
        Gd::from_init_fn(|base| MftrCreator {
            base,
            recipe: Recipe::default(),
            art: String::new(),
            last: None,
            aim: None,
            note: String::new(),
            regions: Vec::new(),
            face: Editor::default(),
            body: Editor::default(),
        })
    }

    /// Where the catalog is (`art/`): the bases and the clip library.
    #[func]
    fn set_art(&mut self, art: GString) {
        self.art = art.to_string();
    }

    /// The builds the catalog offers: `[[id, has genders]]`.
    #[func]
    fn builds(&self) -> VarArray {
        let mut out = VarArray::new();
        for (id, gendered) in mftr_forge::looks::BUILDS {
            let mut row = VarArray::new();
            row.push(&GString::from(id).to_variant());
            row.push(&gendered.to_variant());
            out.push(&row.to_variant());
        }
        out
    }

    #[func]
    fn build(&self) -> GString {
        GString::from(self.recipe.looks.build.as_str())
    }

    /// Change the build: the gender fits it, and what the new body has no place for comes off
    /// (see `note`).
    #[func]
    fn set_build(&mut self, build: GString) {
        let was = self.recipe.looks.base_key();
        self.recipe.looks.build = build.to_string();
        self.refit(&was);
    }

    /// After the body changed: fit the looks to it and say what came off.
    fn refit(&mut self, was: &str) {
        let Some(cat) = catalog(&self.art) else {
            self.recipe.looks.fit_gender();
            return;
        };
        let dropped = cat.fit(&mut self.recipe.looks, was);
        self.note = if dropped.is_empty() { String::new() } else { format!("Taken off: {}.", dropped.join("; ")) };
    }

    /// What the screen should know: why the preview couldn't be made, or what a change of body
    /// took off ("" when nothing).
    #[func]
    fn note(&self) -> GString {
        GString::from(self.note.as_str())
    }

    /// "masculine", "feminine" or "" (a build without genders).
    #[func]
    fn gender(&self) -> GString {
        GString::from(self.recipe.looks.gender.key().unwrap_or(""))
    }

    /// Ignored on a build without genders.
    #[func]
    fn set_gender(&mut self, gender: GString) {
        let was = self.recipe.looks.base_key();
        self.recipe.looks.gender = match gender.to_string().as_str() {
            "feminine" => Gender::Feminine,
            "masculine" => Gender::Masculine,
            _ => Gender::None,
        };
        self.refit(&was);
    }

    /// The sliders the current base offers, in order.
    #[func]
    fn sliders(&self) -> PackedStringArray {
        let Some(cat) = catalog(&self.art) else { return PackedStringArray::new() };
        cat.sliders(&self.recipe.looks.base_key()).iter().map(|s| GString::from(s.as_str())).collect()
    }

    #[func]
    fn slider(&self, id: GString) -> f32 {
        self.recipe.looks.slider(&id.to_string())
    }

    #[func]
    fn set_slider(&mut self, id: GString, value: f32) {
        self.recipe.looks.set_slider(&id.to_string(), value);
    }

    /// The proportions' names, in order.
    #[func]
    fn proportions(&self) -> PackedStringArray {
        Proportions::NAMES.iter().map(|s| GString::from(*s)).collect()
    }

    #[func]
    fn proportion(&self, name: GString) -> f32 {
        let name = name.to_string();
        let i = Proportions::NAMES.iter().position(|n| *n == name).unwrap_or(0);
        self.recipe.looks.proportions.get(i)
    }

    #[func]
    fn set_proportion(&mut self, name: GString, value: f32) {
        let name = name.to_string();
        if let Some(i) = Proportions::NAMES.iter().position(|n| *n == name) {
            self.recipe.looks.proportions.set(i, value);
        }
    }

    /// Whether the body has skin to tone (the beast, serpent and machine don't: their colors
    /// are fills).
    #[func]
    fn has_skin(&self) -> bool {
        let Some(cat) = catalog(&self.art) else { return false };
        cat.base(&self.recipe.looks.base_key()).is_some_and(|b| b.model.materials.iter().any(|m| m == "skin"))
    }

    /// The skin tones, as colors.
    /// The iris colors (22 §2.2), in `eyes::IRIS` order.
    #[func]
    fn eye_colors(&self) -> PackedColorArray {
        eyes::IRIS.iter().map(|c| Color::from_rgb(c[0], c[1], c[2])).collect()
    }

    #[func]
    fn eye_color_names(&self) -> PackedStringArray {
        eyes::IRIS_NAMES.iter().map(|n| GString::from(*n)).collect()
    }

    /// Whether the base has eyes to set (the humanoids': eyeballs on look bones).
    #[func]
    fn has_eyes(&self) -> bool {
        catalog(&self.art)
            .and_then(|c| c.base(&self.recipe.looks.base_key()).map(|b| b.model.node_by_name("eye_look_l").is_some()))
            .unwrap_or(false)
    }

    /// The eyes: `{iris, right (-1: the same), size 0–9, pupil 0–10, style, sclera}`, style
    /// "round", "slit", "glow" or "blank", sclera "white", "yellowed" or "dark".
    #[func]
    fn eyes(&self) -> VarDictionary {
        let e = self.recipe.looks.eyes.unwrap_or_default();
        let mut d = VarDictionary::new();
        d.set("iris", e.iris as i64);
        d.set("right", e.right.map_or(-1, |r| r as i64));
        d.set("size", e.size as i64);
        d.set("pupil", e.pupil as i64);
        d.set("style", &GString::from(e.style.key()).to_variant());
        d.set("sclera", &GString::from(eyes::SCLERA[e.sclera as usize % eyes::SCLERA.len()]).to_variant());
        d
    }

    /// Set one of `eyes`' keys (a number, or a style or sclera name).
    #[func]
    fn set_eye(&mut self, key: GString, value: Variant) {
        let mut e = self.recipe.looks.eyes.unwrap_or_default();
        let n = |max: u8| value.try_to::<i64>().ok().map(|v| v.clamp(0, max as i64) as u8);
        let s = || value.try_to::<GString>().ok().map(|g| g.to_string());
        match key.to_string().as_str() {
            "iris" => e.iris = n(eyes::IRIS.len() as u8 - 1).unwrap_or(e.iris),
            "right" => {
                e.right = match value.try_to::<i64>() {
                    Ok(v) if v >= 0 => Some(v.min(eyes::IRIS.len() as i64 - 1) as u8),
                    _ => None,
                }
            }
            "size" => e.size = n(eyes::SIZE_MAX).unwrap_or(e.size),
            "pupil" => e.pupil = n(eyes::PUPIL_MAX).unwrap_or(e.pupil),
            "style" => e.style = s().and_then(|k| eyes::Style::from_key(&k)).unwrap_or(e.style),
            "sclera" => {
                e.sclera = s().and_then(|k| eyes::SCLERA.iter().position(|x| *x == k)).map_or(e.sclera, |i| i as u8)
            }
            _ => {}
        }
        self.recipe.looks.eyes = (e != eyes::Eyes::default()).then_some(e);
    }

    /// Whether the base has brow strips to shape (the humanoids').
    #[func]
    fn has_brows(&self) -> bool {
        catalog(&self.art)
            .and_then(|c| c.base(&self.recipe.looks.base_key()).map(|b| brows::present(&b.model)))
            .unwrap_or(false)
    }

    /// The brow styles' names, in order.
    #[func]
    fn brow_styles(&self) -> PackedStringArray {
        brows::styles().iter().map(|s| GString::from(s.name.as_str())).collect()
    }

    /// The brow styles' lines (tooltips), in order.
    #[func]
    fn brow_style_notes(&self) -> PackedStringArray {
        brows::styles().iter().map(|s| GString::from(s.about.as_str())).collect()
    }

    /// The brows: `{own: whether the right has its own shape, left: {style, thick, length, arch,
    /// peak, tilt, gap, raise}, right: {…}}`, style an index into `brow_styles`, the rest 0–10.
    #[func]
    fn brows(&self) -> VarDictionary {
        let b = self.recipe.looks.brows.unwrap_or_default();
        let one = |w: &brows::Brow| {
            let mut d = VarDictionary::new();
            d.set("style", w.style as i64);
            for k in brows::KEYS {
                d.set(k, w.get(k).unwrap_or(5) as i64);
            }
            d
        };
        let mut d = VarDictionary::new();
        d.set("own", b.right.is_some());
        d.set("left", &one(&b.left).to_variant());
        d.set("right", &one(&b.right()).to_variant());
        d
    }

    /// Set `key` (`style` or one of the sizes) of the `side` brow ("left" or "right"); with the
    /// brows alike, either sets both.
    #[func]
    fn set_brow(&mut self, side: GString, key: GString, value: i64) {
        let mut b = self.recipe.looks.brows.unwrap_or_default();
        let right = side == "right" && b.right.is_some();
        let w = if right { b.right.as_mut().unwrap() } else { &mut b.left };
        let key = key.to_string();
        if key == "style" {
            w.style = value.clamp(0, brows::styles().len() as i64 - 1) as u8;
        } else {
            w.set(&key, value.clamp(0, brows::MAX as i64) as u8);
        }
        self.recipe.looks.brows = (b != brows::Brows::default()).then_some(b);
    }

    /// Give the right brow its own shape (a copy of the left's), or make it the left's again.
    #[func]
    fn set_brows_own(&mut self, own: bool) {
        let mut b = self.recipe.looks.brows.unwrap_or_default();
        b.right = own.then_some(b.right());
        self.recipe.looks.brows = (b != brows::Brows::default()).then_some(b);
    }

    /// The brows' color: a palette index, or -1 for the base's.
    #[func]
    fn brow_color(&self) -> i64 {
        let Some(r) = self.brow_region() else { return -1 };
        self.recipe
            .looks
            .paint
            .iter()
            .find(|(t, reg, _)| t == "body" && *reg as usize == r)
            .map_or(-1, |(_, _, c)| *c as i64)
    }

    /// Color the brows (a palette index; below 0 back to the base's). It is the body's brow
    /// region, as the Paint step fills it.
    #[func]
    fn set_brow_color(&mut self, color: i64) {
        let Some(r) = self.brow_region() else { return };
        let paint = &mut self.recipe.looks.paint;
        paint.retain(|(t, reg, _)| !(t == "body" && *reg as usize == r));
        if color >= 0 {
            paint.push(("body".into(), r as u8, color.min(paint::PALETTE.len() as i64 - 1) as u8));
        }
    }

    #[func]
    fn skin_tones(&self) -> PackedColorArray {
        SKIN_TONES.iter().map(|c| Color::from_rgb(c[0], c[1], c[2])).collect()
    }

    #[func]
    fn skin(&self) -> i64 {
        self.recipe.looks.skin as i64
    }

    #[func]
    fn set_skin(&mut self, i: i64) {
        self.recipe.looks.skin = i.clamp(0, SKIN_TONES.len() as i64 - 1) as u8;
    }

    #[func]
    fn name(&self) -> GString {
        GString::from(self.recipe.name.as_str())
    }

    #[func]
    fn set_name(&mut self, name: GString) {
        self.recipe.name = name.to_string();
    }

    /// The official champions whose kits can be borrowed (until the moveset step, CR6).
    #[func]
    fn kits(&self) -> PackedStringArray {
        ChampionId::ALL.iter().map(|c| GString::from(c.def().name)).collect()
    }

    /// The official champion whose kit this is ("" once it's changed).
    #[func]
    fn kit(&self) -> GString {
        GString::from(self.recipe.kit_from().map_or("", |c| c.def().name))
    }

    #[func]
    fn set_kit(&mut self, champion: GString) {
        if let Some(c) = ChampionId::by_name(&champion.to_string()).filter(|c| c.is_official()) {
            self.recipe.borrow_kit(c);
        }
    }

    /// The kit's abilities, one line each, for the screen.
    #[func]
    fn kit_summary(&self) -> PackedStringArray {
        let k = &self.recipe.kit;
        let mut out = PackedStringArray::new();
        out.push(&GString::from(&format!(
            "{} · {}",
            k.class.name(),
            if k.attack.bolt_speed == 0.0 { "melee" } else { "ranged" }
        )));
        if let Some((_, label, _, _)) = mftr_forge::passives::of(k.passive) {
            out.push(&GString::from(&format!("Passive  {label}")));
        }
        for (slot, a) in ["Q", "W", "E", "R"].iter().zip(&k.abilities) {
            out.push(&GString::from(&format!("{slot}  {}", a.name)));
        }
        out
    }

    /// The slots parts go on, in order.
    #[func]
    fn slots(&self) -> PackedStringArray {
        mftr_forge::SLOTS.iter().map(|s| GString::from(*s)).collect()
    }

    /// The parts the current base offers: `[{id, slot, name, triangles}]`.
    #[func]
    fn parts(&self) -> VarArray {
        let mut out = VarArray::new();
        let Some(cat) = catalog(&self.art) else { return out };
        for p in cat.parts(&self.recipe.looks.base_key()) {
            let mut d = VarDictionary::new();
            d.set("id", p.id.as_str());
            d.set("slot", p.slot.as_str());
            d.set("name", p.name.as_str());
            d.set("triangles", p.triangles as i64);
            out.push(&d.to_variant());
        }
        out
    }

    /// The parts worn.
    #[func]
    fn outfit(&self) -> PackedStringArray {
        self.recipe.looks.outfit.iter().map(|s| GString::from(s.as_str())).collect()
    }

    /// Wear `id` on `slot` (whatever was there comes off); "" takes the slot's part off.
    #[func]
    fn set_part(&mut self, slot: GString, id: GString) {
        let (slot, id) = (slot.to_string(), id.to_string());
        // A new hairstyle or beard keeps the old one's color (22 §7 item 5).
        let kept = if GROWN.contains(&slot.as_str()) { self.grown_color(&slot) } else { None };
        let prefix = format!("{slot}.");
        self.recipe.looks.outfit.retain(|p| !p.starts_with(&prefix));
        if !id.is_empty() {
            self.recipe.looks.outfit.push(id.clone());
        }
        // Fills on parts no longer worn go with them.
        let worn = self.recipe.looks.outfit.clone();
        self.recipe.looks.paint.retain(|(t, _, _)| t == "body" || worn.contains(t));
        if let (Some(c), false) = (kept, id.is_empty()) {
            self.recipe.looks.paint.push((id, 0, c));
        }
    }

    /// The color of the hair (`slot` "hair") or the beard ("beard") worn: its first region's
    /// palette index, -1 as built or with none worn (22 §7 item 5).
    #[func]
    fn hair_color(&self, slot: GString) -> i64 {
        self.grown_color(&slot.to_string()).map_or(-1, i64::from)
    }

    /// Color the hair or the beard worn (a palette index; below 0 back to as built).
    #[func]
    fn set_hair_color(&mut self, slot: GString, color: i64) {
        let Some(id) = self.worn_on(&slot.to_string()) else { return };
        let paint = &mut self.recipe.looks.paint;
        paint.retain(|(t, r, _)| !(*t == id && *r == 0));
        if color >= 0 {
            paint.push((id, 0, color.min(paint::PALETTE.len() as i64 - 1) as u8));
        }
    }

    /// Whether part `id` has a length control (`<slot>_length`, 22 §7 item 5).
    #[func]
    fn has_length(&self, id: GString) -> bool {
        let id = id.to_string();
        let Some(slot) = id.split('.').next() else { return false };
        let Some(cat) = catalog(&self.art) else { return false };
        let Some(base) = cat.base(&self.recipe.looks.base_key()) else { return false };
        let m = &base.model;
        let want = format!("{slot}_length+");
        m.nodes
            .iter()
            .find(|n| n.name.strip_prefix("part.") == Some(id.as_str()))
            .and_then(|n| n.mesh)
            .is_some_and(|mi| m.target_names[mi].contains(&want))
    }

    /// What can be painted: "body", then each worn part.
    #[func]
    fn paint_targets(&self) -> PackedStringArray {
        let mut out = PackedStringArray::new();
        out.push("body");
        for p in &self.recipe.looks.outfit {
            out.push(p.as_str());
        }
        out
    }

    /// `target`'s paint regions as they look now (filled, or as built).
    #[func]
    fn paint_regions(&self, target: GString) -> PackedColorArray {
        let target = target.to_string();
        let Some(cat) = catalog(&self.art) else { return PackedColorArray::new() };
        let Some(base) = cat.base(&self.recipe.looks.base_key()) else { return PackedColorArray::new() };
        mftr_forge::paint::regions(&base.model, &target)
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let filled = self.recipe.looks.paint.iter().find(|(t, r, _)| *t == target && *r as usize == i);
                let c = filled.map_or(*c, |(_, _, k)| mftr_forge::paint::PALETTE[*k as usize]);
                Color::from_rgb(c[0], c[1], c[2])
            })
            .collect()
    }

    /// The shared palette.
    #[func]
    fn palette(&self) -> PackedColorArray {
        mftr_forge::paint::PALETTE.iter().map(|c| Color::from_rgb(c[0], c[1], c[2])).collect()
    }

    /// Fill `target`'s region with palette color `color`.
    #[func]
    fn fill(&mut self, target: GString, region: i64, color: i64) {
        let (target, region, color) = (target.to_string(), region.clamp(0, 15) as u8, color.clamp(0, 63) as u8);
        let paint = &mut self.recipe.looks.paint;
        paint.retain(|(t, r, _)| !(*t == target && *r == region));
        paint.push((target, region, color));
    }

    /// Every fill undone.
    #[func]
    fn clear_paint(&mut self) {
        self.recipe.looks.paint.clear();
    }

    #[func]
    fn signature(&self) -> i64 {
        self.recipe.looks.signature as i64
    }

    #[func]
    fn set_signature(&mut self, hue: i64) {
        self.recipe.looks.signature = hue.rem_euclid(360) as u16;
    }

    /// The signature hue as a color.
    #[func]
    fn signature_color(&self) -> Color {
        let c = mftr_forge::paint::signature(self.recipe.looks.signature);
        Color::from_rgb(c[0], c[1], c[2])
    }

    /// The moves that fit `slot` ("attack", "q" … "r"): `[{id, label}]`.
    #[func]
    fn move_library(&self, slot: GString) -> VarArray {
        let slot = slot.to_string();
        let mut out = VarArray::new();
        for (id, label) in mftr_forge::moves::library() {
            if mftr_forge::moves::fits(&id, &slot) {
                let mut d = VarDictionary::new();
                d.set("id", id.as_str());
                d.set("label", label.as_str());
                out.push(&d.to_variant());
            }
        }
        out
    }

    /// The passive catalog (20 §8.1): `[{id, label, hint}]`.
    #[func]
    fn passives(&self) -> VarArray {
        let mut out = VarArray::new();
        for (id, label, hint, _) in mftr_forge::passives::PASSIVES {
            let mut d = VarDictionary::new();
            d.set("id", id);
            d.set("label", label);
            d.set("hint", hint);
            out.push(&d.to_variant());
        }
        out
    }

    /// The resources (01 §9): `[{id, label, hint}]`.
    #[func]
    fn resources(&self) -> VarArray {
        let mut out = VarArray::new();
        for (id, label, hint, _) in mftr_forge::passives::RESOURCES {
            let mut d = VarDictionary::new();
            d.set("id", id);
            d.set("label", label);
            d.set("hint", hint);
            out.push(&d.to_variant());
        }
        out
    }

    /// The kit's resource's id ("" costless).
    #[func]
    fn resource(&self) -> GString {
        GString::from(mftr_forge::passives::resource_id(&self.recipe.kit.resource))
    }

    #[func]
    fn set_resource(&mut self, id: GString) {
        self.recipe.set_resource(&id.to_string());
    }

    /// The kit's passive's id ("" for none).
    #[func]
    fn passive(&self) -> GString {
        GString::from(self.recipe.passive())
    }

    #[func]
    fn set_passive(&mut self, id: GString) {
        self.recipe.set_passive(&id.to_string());
    }

    /// The move on `slot`.
    #[func]
    fn move_on(&self, slot: GString) -> GString {
        GString::from(self.recipe.moves().get(&slot.to_string()))
    }

    /// Put a move on a slot. An attack also equips the weapon its animations hold (when the
    /// catalog has it).
    #[func]
    fn set_move(&mut self, slot: GString, id: GString) {
        let (slot, id) = (slot.to_string(), id.to_string());
        if !self.recipe.set_move(&slot, &id) {
            return;
        }
        if slot == "attack"
            && let Some((c, _)) = mftr_forge::moves::parse(&id)
        {
            let (_, part) = mftr_forge::moves::weapon(c);
            self.recipe.looks.outfit.retain(|p| !p.starts_with("weapon.") && !p.starts_with("offhand."));
            if !part.is_empty() {
                self.recipe.looks.outfit.push(part.to_string());
            }
        }
    }

    /// The price (20 §9): `{total, budget, cap, parts: [[name, points]]}`.
    #[func]
    fn price(&self) -> VarDictionary {
        let p = mftr_sim::balance::price(&self.recipe.kit);
        let mut d = VarDictionary::new();
        d.set("total", p.total());
        d.set("budget", mftr_sim::balance::BUDGET);
        d.set("cap", mftr_sim::balance::cap());
        let mut parts = VarArray::new();
        for (name, pts) in p.parts() {
            let mut row = VarArray::new();
            row.push(&GString::from(name).to_variant());
            row.push(&pts.to_variant());
            parts.push(&row.to_variant());
        }
        d.set("parts", &parts);
        d
    }

    /// A slot's power and cooldown, against its move's numbers: `[power, cooldown]`.
    #[func]
    fn tuning(&self, slot: GString) -> PackedFloat32Array {
        let (p, c) = self.recipe.tuning(&slot.to_string());
        PackedFloat32Array::from(&[p, c][..])
    }

    #[func]
    fn tune(&mut self, slot: GString, power: f32, cooldown: f32) {
        self.recipe.tune(&slot.to_string(), power, cooldown);
    }

    /// Health, armor, magic resist and move speed against the stance champion's.
    #[func]
    fn stat_tuning(&self) -> PackedFloat32Array {
        PackedFloat32Array::from(&self.recipe.stat_tuning()[..])
    }

    #[func]
    fn tune_stats(&mut self, k: PackedFloat32Array) {
        let v = k.as_slice();
        if v.len() == 4 {
            self.recipe.tune_stats([v[0], v[1], v[2], v[3]]);
        }
    }

    /// Scale everything down until it fits the budget.
    #[func]
    fn auto_fit(&mut self) {
        self.recipe.auto_fit();
    }

    /// Every slider and proportion at random (a seeded roll).
    #[func]
    fn randomize(&mut self, seed: i64) {
        let ids: Vec<String> = self.sliders().as_slice().iter().map(|s| s.to_string()).collect();
        recipe::randomize(&mut self.recipe.looks, &ids, seed as u64);
    }

    /// The face shapes (21 §7 FR5): `[{id, name, about}]`, empty on a base without a face rig.
    #[func]
    fn face_shapes(&self) -> VarArray {
        let mut out = VarArray::new();
        if !self.sliders().as_slice().iter().any(|s| mftr_forge::face::preset(&s.to_string()).is_some()) {
            return out;
        }
        for s in mftr_forge::face::shapes() {
            let mut d = VarDictionary::new();
            d.set("id", s.id.as_str());
            d.set("name", s.name.as_str());
            d.set("about", s.about.as_str());
            out.push(&d.to_variant());
        }
        out
    }

    /// The face shape the face sliders are set to ("" when they've moved off every one).
    #[func]
    fn face_shape(&self) -> GString {
        GString::from(recipe::face_shape_of(&self.recipe.looks).unwrap_or(""))
    }

    /// Set the face sliders to face shape `id`.
    #[func]
    fn set_face_shape(&mut self, id: GString) {
        recipe::apply_face_shape(&mut self.recipe.looks, &id.to_string());
    }

    /// A face at random from the shapes (the body left as it is).
    #[func]
    fn random_face(&mut self, seed: i64) {
        let ids: Vec<String> = self.sliders().as_slice().iter().map(|s| s.to_string()).collect();
        recipe::randomize_face(&mut self.recipe.looks, &ids, seed as u64);
    }

    /// Back to the plain base: every slider and proportion 0.
    #[func]
    fn reset_shape(&mut self) {
        self.recipe.looks.shape.clear();
        self.recipe.looks.proportions = Proportions::default();
    }

    /// The preview: the champion forged as it is now (`null` if it can't be).
    #[func]
    fn preview(&mut self) -> Option<Gd<MftrModel>> {
        let cat = catalog(&self.art)?;
        match cat.forge(&self.recipe.name, &self.recipe.looks) {
            Ok(pack) => {
                let pack = Arc::new(pack);
                self.last = Some(pack.clone());
                self.aim = aimed(&pack.model);
                self.regions = face::vertex_regions(&pack.model);
                self.face.frames =
                    pack.model.node_by_name("face_forehead").map(|_| face::bone_frames(&pack.model, &face::FACE));
                self.body.frames = body::rigged(&pack.model).then(|| face::bone_frames(&pack.model, &body::BODY));
                if self.note.starts_with("Can't show") {
                    self.note.clear();
                }
                Some(MftrModel::from_pack(pack))
            }
            Err(e) => {
                godot_error!("MFTR creator preview: {e}");
                self.note = format!("Can't show this: {e}");
                None
            }
        }
    }

    /// The decals (20 §7): `[{id, label}]`.
    #[func]
    fn decals(&self) -> VarArray {
        let mut out = VarArray::new();
        for (id, label) in mftr_forge::image::DECALS {
            let mut d = VarDictionary::new();
            d.set("id", id);
            d.set("label", label);
            out.push(&d.to_variant());
        }
        out
    }

    /// Paint where the ray `origin + t·dir` (in the preview model's space, rest pose) meets the
    /// last preview (CR5b): a round brush of `radius` meters in palette `color` (-1 erases), or
    /// the decal `decal` that size. Returns what was painted (`"body"` or a part id), or "" when
    /// nothing was (a miss, or an image with all its colors used).
    #[func]
    fn brush(&mut self, origin: Vector3, dir: Vector3, radius: f32, color: i64, decal: GString) -> GString {
        use mftr_forge::image::{self, Image, Texels, Tool};
        let Some(last) = self.last.clone() else { return GString::new() };
        let aim = self.aim.clone();
        let model = aim.as_deref().unwrap_or(&last.model);
        let Some((mi, hit, normal)) = image::raycast(model, [origin.x, origin.y, origin.z], [dir.x, dir.y, dir.z])
        else {
            return GString::new();
        };
        let Some(target) = image::target_of(model, mi) else { return GString::new() };
        let looks = &mut self.recipe.looks;
        if !looks.images.iter().any(|i| i.target == target) {
            if color < 0 {
                return GString::new();
            }
            if looks.images.len() >= image::MAX_IMAGES {
                // Say why nothing happened (painting "stopped working", 21 §8 #9).
                self.note = format!(
                    "Brushwork goes on {} parts at most: clear the paint on one to paint this one.",
                    image::MAX_IMAGES
                );
                return GString::new();
            }
            looks.images.push(Image::new(&target));
        }
        if color >= 0
            && let Some(img) = looks.images.iter().find(|i| i.target == target)
            && img.colors.len() >= image::MAX_COLORS
            && !img.colors.contains(&(color.clamp(0, 63) as u8))
        {
            self.note = format!(
                "This already has {} brush colors: paint with one of them, or erase some first.",
                image::MAX_COLORS
            );
            return GString::new();
        }
        if self.note.starts_with("Brushwork") || self.note.starts_with("This already") {
            self.note.clear();
        }
        let decal = decal.to_string();
        let tool = match (color, decal.is_empty()) {
            (c, _) if c < 0 => Tool::Brush(None),
            (c, true) => Tool::Brush(Some(c.clamp(0, 63) as u8)),
            (c, false) => Tool::Decal(&decal, c.clamp(0, 63) as u8),
        };
        let texels = Texels::of(model, &target);
        let img = looks.images.iter_mut().find(|i| i.target == target).expect("added above");
        let changed = image::stroke(img, &texels, hit, normal, radius, tool);
        looks.images.retain(|i| !i.is_empty());
        if changed { GString::from(target.as_str()) } else { GString::new() }
    }

    /// Sculpt (CR9, 20 §5.3) where the ray `origin + t·dir` (model space, rest pose) meets the
    /// last preview's body: `brush` is "push", "pull", "grab" (along `pull`, model space),
    /// "inflate", "smooth" or "flatten"; `radius` and `strength` in meters; mirrored if
    /// `symmetric`. True if it touched the body.
    #[func]
    #[allow(clippy::too_many_arguments)]
    fn sculpt(
        &mut self,
        origin: Vector3,
        dir: Vector3,
        brush: GString,
        radius: f32,
        strength: f32,
        pull: Vector3,
        symmetric: bool,
    ) -> bool {
        use mftr_forge::sculpt::{self, Brush};
        let (Some(cat), Some(last)) = (catalog(&self.art), self.last.clone()) else { return false };
        let aim = self.aim.clone();
        let key = self.recipe.looks.base_key();
        let Some(welds) = cat.welds(&key) else { return false };
        let brush = brush.to_string();
        let b = match brush.as_str() {
            "push" => Brush::Push,
            "pull" => Brush::Push,
            "grab" => Brush::Grab([pull.x, pull.y, pull.z]),
            "inflate" => Brush::Inflate,
            "smooth" => Brush::Smooth,
            "flatten" => Brush::Flatten,
            _ => return false,
        };
        let strength = if brush == "pull" { -strength } else { strength };
        let height = self.recipe.looks.proportions.factor(0);
        sculpt::stroke(
            &mut self.recipe.looks.sculpt,
            welds,
            aim.as_deref().unwrap_or(&last.model),
            [origin.x, origin.y, origin.z],
            [dir.x, dir.y, dir.z],
            b,
            radius,
            strength,
            symmetric,
            height,
        )
    }

    // --- The face editor (FR4) and the body's (FR6) -----------------------------------------
    // Regions are ids among the preview's (the shader's `hover_region`): a face bone's index,
    // or 24 plus a body helper's.

    /// The face region under the ray `origin + t·dir` (the preview's model space, rest pose), or
    /// -1: its name from `region_name`. Remembers it for a drag.
    #[func]
    fn face_hover(&mut self, origin: Vector3, dir: Vector3) -> i64 {
        self.hover::<FaceSculpt>(origin, dir)
    }

    /// The other side's region (itself on the middle line), which follows with symmetry.
    #[func]
    fn face_mirror(&self, region: i64) -> i64 {
        self.region_mirror(region)
    }

    #[func]
    fn face_region_name(&self, region: i64) -> GString {
        self.region_name(region)
    }

    /// Start dragging the hovered region: `mode` "move", "turn", "size" or "size_width". False
    /// when nothing is under the cursor.
    #[func]
    fn face_begin(&mut self, mode: GString, symmetry: bool) -> bool {
        self.begin::<FaceSculpt>(&mode.to_string(), symmetry)
    }

    /// A mouse move of `relative` pixels while dragging. `camera` is the stage camera in the
    /// preview's model space (`origin`, `right`, `up`, `forward`, `fov_y` radians,
    /// `viewport_h`). Poses `skeleton`'s face bones at once (no forge until `face_end`).
    #[func]
    fn face_drag(&mut self, relative: Vector2, camera: VarDictionary, skeleton: Gd<Skeleton3D>) -> bool {
        self.drag::<FaceSculpt>(relative, &camera, skeleton)
    }

    /// The drag is over: true when the face changed (the screen then forges the preview).
    #[func]
    fn face_end(&mut self) -> bool {
        self.end::<FaceSculpt>()
    }

    /// Back one drag: false when there's nothing to undo.
    #[func]
    fn face_undo(&mut self) -> bool {
        self.undo::<FaceSculpt>()
    }

    /// Every feature back where the sliders put it (undoable).
    #[func]
    fn face_clear(&mut self) {
        self.clear::<FaceSculpt>();
    }

    /// Whether the face has been dragged at all (for the screen's buttons).
    #[func]
    fn face_edited(&self) -> bool {
        self.recipe.looks.face.is_some()
    }

    #[func]
    fn face_can_undo(&self) -> bool {
        !self.face.undo.is_empty()
    }

    /// The body region under the ray, or -1 (as `face_hover`; a body helper's region id).
    #[func]
    fn body_hover(&mut self, origin: Vector3, dir: Vector3) -> i64 {
        self.hover::<body::BodySculpt>(origin, dir)
    }

    /// The other side's region for any region id (itself on the middle line).
    #[func]
    fn region_mirror(&self, region: i64) -> i64 {
        match set_of(region) {
            Some((set, b)) => region - b as i64 + set.mirror(b) as i64,
            None => -1,
        }
    }

    /// What a region id is called on screen ("" for none).
    #[func]
    fn region_name(&self, region: i64) -> GString {
        GString::from(set_of(region).map_or("", |(set, b)| set.regions[b]))
    }

    /// Start dragging the hovered body region: `mode` "move", "size" or "size_width".
    #[func]
    fn body_begin(&mut self, mode: GString, symmetry: bool) -> bool {
        self.begin::<body::BodySculpt>(&mode.to_string(), symmetry)
    }

    /// As `face_drag`, on the body's helpers.
    #[func]
    fn body_drag(&mut self, relative: Vector2, camera: VarDictionary, skeleton: Gd<Skeleton3D>) -> bool {
        self.drag::<body::BodySculpt>(relative, &camera, skeleton)
    }

    #[func]
    fn body_end(&mut self) -> bool {
        self.end::<body::BodySculpt>()
    }

    #[func]
    fn body_undo(&mut self) -> bool {
        self.undo::<body::BodySculpt>()
    }

    /// Every mass back where the sliders put it (undoable).
    #[func]
    fn body_clear(&mut self) {
        self.clear::<body::BodySculpt>();
    }

    #[func]
    fn body_edited(&self) -> bool {
        self.recipe.looks.body.is_some()
    }

    #[func]
    fn body_can_undo(&self) -> bool {
        !self.body.undo.is_empty()
    }

    /// Whether the body can be dragged at all: the base has the helper bones.
    #[func]
    fn body_rigged(&self) -> bool {
        self.body.frames.is_some()
    }

    /// Undo every sculpt stroke.
    #[func]
    fn clear_sculpt(&mut self) {
        self.recipe.looks.sculpt.clear();
    }

    /// Wipe every brush stroke and decal.
    #[func]
    fn clear_images(&mut self) {
        self.recipe.looks.images.clear();
    }

    /// What stops the champion from being saved (empty when nothing does).
    #[func]
    fn problems(&self) -> PackedStringArray {
        self.recipe.problems().iter().map(|s| GString::from(s.as_str())).collect()
    }

    /// The champion file.
    #[func]
    fn text(&self) -> GString {
        GString::from(self.recipe.to_text().as_str())
    }

    /// Save to `path` and register it here (so it can be played and tested): "" or the reason
    /// it wasn't.
    #[func]
    fn save(&self, path: GString) -> GString {
        let problems = self.recipe.problems();
        if !problems.is_empty() {
            return GString::from(problems.join("; ").as_str());
        }
        let text = self.recipe.to_text();
        if let Err(e) = std::fs::write(path.to_string(), &text) {
            return GString::from(e.to_string().as_str());
        }
        match mftr_client::load_champion_file(&text) {
            Ok(_) => GString::new(),
            Err(e) => GString::from(e.as_str()),
        }
    }

    /// Load a champion file: "" or the reason it couldn't be.
    #[func]
    fn load(&mut self, path: GString) -> GString {
        match std::fs::read_to_string(path.to_string()).map_err(|e| e.to_string()).and_then(|t| Recipe::from_text(&t)) {
            Ok(r) => {
                self.recipe = r;
                let key = self.recipe.looks.base_key();
                self.refit(&key);
                GString::new()
            }
            Err(e) => GString::from(e.as_str()),
        }
    }
}

/// `model` with its vertices where the skin puts them, when its face bones were moved (their
/// rest differs from where the mesh is bound): the brush and the sculpt then land where the face
/// shows, not where it was built.
fn aimed(model: &mftr_pack::glb::Model) -> Option<Arc<mftr_pack::glb::Model>> {
    if !model.bind.iter().any(Option::is_some) {
        return None;
    }
    let skinned = mftr_forge::face::skinned(model);
    let mut m = model.clone();
    for (prims, s) in m.meshes.iter_mut().zip(skinned) {
        for (p, pos) in prims.iter_mut().zip(s) {
            p.positions = pos;
        }
    }
    Some(Arc::new(m))
}

impl MftrCreator {
    /// The base's brow paint region (22 §3.1), if it has brows.
    /// The part worn on `slot`, if any.
    fn worn_on(&self, slot: &str) -> Option<String> {
        let prefix = format!("{slot}.");
        self.recipe.looks.outfit.iter().find(|p| p.starts_with(&prefix)).cloned()
    }

    /// The palette index the part worn on `slot` has on its first region, if filled.
    fn grown_color(&self, slot: &str) -> Option<u8> {
        let id = self.worn_on(slot)?;
        self.recipe.looks.paint.iter().find(|(t, r, _)| *t == id && *r == 0).map(|(_, _, c)| *c)
    }

    fn brow_region(&self) -> Option<usize> {
        catalog(&self.art)
            .and_then(|c| c.base(&self.recipe.looks.base_key()).and_then(|b| brows::color_region(&b.model)))
    }

    fn hover<S: Edited>(&mut self, origin: Vector3, dir: Vector3) -> i64 {
        let Some(last) = self.last.clone() else { return -1 };
        if S::editor(self).frames.is_none() {
            return -1;
        }
        let aim = self.aim.clone();
        let model = aim.as_deref().unwrap_or(&last.model);
        let own = S::BASE..S::BASE + S::set().rig.len();
        let hit = face::region_at(model, &self.regions, [origin.x, origin.y, origin.z], [dir.x, dir.y, dir.z])
            .filter(|(r, _)| own.contains(r));
        let e = S::editor_mut(self);
        e.hover = hit.map(|(r, _)| r - S::BASE);
        if let Some((_, p)) = hit {
            e.anchor = p;
        }
        e.hover.map_or(-1, |b| (b + S::BASE) as i64)
    }

    fn begin<S: Edited>(&mut self, mode: &str, symmetry: bool) -> bool {
        let Some(bone) = S::editor(self).hover else { return false };
        let mode = match mode {
            "turn" => face::DragMode::Turn,
            "size" => face::DragMode::Size { width_only: false },
            "size_width" => face::DragMode::Size { width_only: true },
            _ => face::DragMode::Move,
        };
        let was = S::of(&self.recipe.looks).clone();
        let e = S::editor_mut(self);
        e.undo.push(was);
        if e.undo.len() > 64 {
            e.undo.remove(0);
        }
        e.drag = Some((bone, mode, symmetry));
        true
    }

    fn drag<S: Edited>(&mut self, relative: Vector2, camera: &VarDictionary, skeleton: Gd<Skeleton3D>) -> bool {
        let e = S::editor(self);
        let (Some((bone, mode, symmetry)), Some(frames), Some(last)) = (e.drag, e.frames.clone(), self.last.clone())
        else {
            return false;
        };
        let anchor = e.anchor;
        let v = |k: &str| camera.get(k).and_then(|x| x.try_to::<Vector3>().ok()).map_or([0.0; 3], |v| [v.x, v.y, v.z]);
        let f = |k: &str| camera.get(k).and_then(|x| x.try_to::<f32>().ok()).unwrap_or(0.0);
        let cam = face::CameraFrame {
            origin: v("origin"),
            right: v("right"),
            up: v("up"),
            forward: v("forward"),
            fov_y: f("fov_y"),
            viewport_h: f("viewport_h"),
        };
        let px = [relative.x, relative.y];
        let world = face::drag_to_world(&cam, px, anchor);
        let mut sculpt = S::of(&self.recipe.looks).clone().unwrap_or_default();
        if !face::drag(&mut sculpt, bone, mode, world, px, &cam, &frames, symmetry) {
            return false;
        }
        if mode == face::DragMode::Move {
            // The plane follows the feature, so the next pixel is measured where it now is.
            S::editor_mut(self).anchor = [0, 1, 2].map(|k| anchor[k] + world[k]);
        }
        let neutral = sculpt.bones().iter().all(face::BoneDelta::is_zero);
        *S::of_mut(&mut self.recipe.looks) = (!neutral).then_some(sculpt);
        self.pose::<S>(&last.model, skeleton);
        true
    }

    fn end<S: Edited>(&mut self) -> bool {
        let now = S::of(&self.recipe.looks).clone();
        let e = S::editor_mut(self);
        let Some(_) = e.drag.take() else { return false };
        let changed = e.undo.last() != Some(&now);
        if !changed {
            e.undo.pop();
        }
        changed
    }

    fn undo<S: Edited>(&mut self) -> bool {
        match S::editor_mut(self).undo.pop() {
            Some(was) => {
                *S::of_mut(&mut self.recipe.looks) = was;
                true
            }
            None => false,
        }
    }

    fn clear<S: Edited>(&mut self) {
        if let Some(was) = S::of_mut(&mut self.recipe.looks).take() {
            S::editor_mut(self).undo.push(Some(was));
        }
    }

    /// Pose `skeleton`'s bones of a sculpt's set as the recipe has them (its own sculpt and its
    /// sliders) on the forged `model`: what the bake would make, set as the bones' pose.
    fn pose<S: Edited>(&self, model: &mftr_pack::glb::Model, mut skeleton: Gd<Skeleton3D>) {
        let shown = S::shown(&self.recipe.looks).unwrap_or_default();
        for (node, (t, r, s)) in face::posed(model, &shown) {
            let idx = skeleton.find_bone(model.nodes[node].name.as_str());
            if idx < 0 {
                continue;
            }
            skeleton.set_bone_pose_position(idx, Vector3::new(t[0], t[1], t[2]));
            skeleton.set_bone_pose_rotation(idx, Quaternion::new(r[0], r[1], r[2], r[3]));
            skeleton.set_bone_pose_scale(idx, Vector3::new(s[0], s[1], s[2]));
        }
    }
}
