//! The champion creator (CR3, 20 §2) on the Godot side: `MftrCreator` holds the champion being
//! made (a `mftr_forge::recipe::Recipe`), changes it as the screen's controls move, forges its
//! preview, and saves and loads its champion file. The screen itself is `creator.gd`.

use std::sync::Arc;

use godot::prelude::*;
use mftr_forge::looks::{Gender, Proportions, SKIN_TONES};
use mftr_forge::recipe::{self, Recipe};
use mftr_sim::ChampionId;

use crate::model::{MftrModel, catalog};

#[derive(GodotClass)]
#[class(base = RefCounted, no_init)]
pub struct MftrCreator {
    base: Base<RefCounted>,
    recipe: Recipe,
    /// The art folder (the catalog's), set by `set_art`.
    art: String,
}

#[godot_api]
impl MftrCreator {
    /// A new champion: the defaults (a slim masculine base, Rook's kit).
    #[func]
    fn create() -> Gd<MftrCreator> {
        Gd::from_init_fn(|base| MftrCreator { base, recipe: Recipe::default(), art: String::new() })
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

    #[func]
    fn set_build(&mut self, build: GString) {
        self.recipe.looks.build = build.to_string();
    }

    /// "masculine", "feminine" or "" (a build without genders).
    #[func]
    fn gender(&self) -> GString {
        GString::from(self.recipe.looks.gender.key().unwrap_or(""))
    }

    #[func]
    fn set_gender(&mut self, gender: GString) {
        self.recipe.looks.gender = match gender.to_string().as_str() {
            "feminine" => Gender::Feminine,
            "masculine" => Gender::Masculine,
            _ => Gender::None,
        };
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

    /// The skin tones, as colors.
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
        let prefix = format!("{slot}.");
        self.recipe.looks.outfit.retain(|p| !p.starts_with(&prefix));
        if !id.is_empty() {
            self.recipe.looks.outfit.push(id);
        }
        // Fills on parts no longer worn go with them.
        let worn = self.recipe.looks.outfit.clone();
        self.recipe.looks.paint.retain(|(t, _, _)| t == "body" || worn.contains(t));
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

    /// Every slider and proportion at random (a seeded roll).
    #[func]
    fn randomize(&mut self, seed: i64) {
        let ids: Vec<String> = self.sliders().as_slice().iter().map(|s| s.to_string()).collect();
        recipe::randomize(&mut self.recipe.looks, &ids, seed as u64);
    }

    /// Back to the plain base: every slider and proportion 0.
    #[func]
    fn reset_shape(&mut self) {
        self.recipe.looks.shape.clear();
        self.recipe.looks.proportions = Proportions::default();
    }

    /// The preview: the champion forged as it is now (`null` if it can't be).
    #[func]
    fn preview(&self) -> Option<Gd<MftrModel>> {
        let cat = catalog(&self.art)?;
        match cat.forge(&self.recipe.name, &self.recipe.looks) {
            Ok(pack) => Some(MftrModel::from_pack(Arc::new(pack))),
            Err(e) => {
                godot_error!("MFTR creator preview: {e}");
                None
            }
        }
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
                GString::new()
            }
            Err(e) => GString::from(e.as_str()),
        }
    }
}
