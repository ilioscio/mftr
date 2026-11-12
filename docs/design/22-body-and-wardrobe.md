# 22 — Body, Eyes and Wardrobe: the Quality Upgrade

**Status:** proposed (2026-11-12). Owner's brief, in their words: upgrade the body the way we
upgraded the head; fix the eyes; make brows and lips real, customizable features; and build a
base for a *large* catalog of clothing, armor and weapons that fits every body the creator can
make. Target: above LoL season-1 models, still playable on old hardware. Six builds to support.

This doc is the plan. [21](21-face-rig.md) stays the face's and the body's *rig*; this doc is
what the rig carries.

---

## 0. The design in one paragraph

Every humanoid base becomes one **CC0 artist-made body** (Blender Studio's realistic base mesh,
reduced to ~4k triangles below the neck), with the authored head on it, **underwear as material
regions** of that mesh, and the 24 body helpers of 21 §9 weighted on real anatomy instead of
lofted tubes. Male and female, slim and heavy are **one topology** with baked shapes, so
anything authored once fits all four. Eyes become real eyeballs with a smooth procedural iris,
**lid bones** that blink and follow the gaze, and lid-shape controls. Brows and lips become
**feature objects** with style catalogs and per-side controls. Clothing and armor become
**wardrobe items** authored once on a reference body and fitted to every build at build time
(surface binding, layer offsets, push-out against the body), with spring-bone secondary motion
for cloth and hair. A QA harness renders every item on every base at the slider extremes, so a
large catalog stays honest.

---

## 1. The body

### 1.1 Source

Blender Studio's **Human Base Meshes** bundle v1.4.1 (CC0, the head's source: `art/creator/heads/README.md`),
`GEO-body_male_realistic` and `GEO-body_female_realistic`:

| | Vertices | Faces | Notes |
|---|---|---|---|
| As shipped | 10,582 | 10,590 (10,570 quads) | Multires base level; eyes as separate objects |
| Un-subdivided twice | 3,201 | 6,398 triangles, head and hands included | Clean loops on the torso and limbs |

Checked on 2026-11-12:
- **Male and female share the exact topology** (the same face list). A base's sex is a blend
  between them: one mesh, one set of weights, one wardrobe fit.
- The stylized pair also share a topology (12,502 vertices), with cartoon proportions. We take
  the **realistic** pair: our heroic proportions come from the rig (14 §2.2) and the builds'
  shapes, and the realistic head family is the one the authored head comes from.

### 1.2 Reduction

1. Un-subdivide twice (Blender's Decimate, Un-Subdivide).
2. Drop the head: the authored head (21 §1) sits on the body's neck as it does now.
3. Keep the hands and feet: they are far better than ours.
4. Dissolve edge loops where the silhouette doesn't need them (the shins, the forearms' backs),
   by a list of loops kept per region rather than a blind collapse.

**Budget:** body ≤ 4,200 triangles below the neck, head 2,500, eyes 2 × 120, brows 2 × 40. A
bare champion lands near 7,000; clothes hide what they cover, so a dressed one stays within
the 10,000 target and the 14,000 cap.

### 1.3 Fitting to the rig

The clips are authored on the biped's rest (10 §7.2), so the body is fitted to the rig, not the
other way round:

- landmarks on the body (shoulder, elbow, wrist, hip, knee, ankle, the spine's segments) found
  by vertex position once, in the source's frame;
- a thin-plate warp (as 21's head parts) carrying each landmark to its joint in the build's
  shape (`rig.shaped`);
- weights from the joints' bone chains (Blender's automatic weights on the warped mesh, then
  limited to two biped influences), and the body helpers' weights on top (21 §9, already
  built in `body_rig.py`).

### 1.4 The four humanoid bases from one mesh

- **Sex:** the male and female sources as a blend (0 or 1 per base; in between later, as a
  slider, if the owner wants it).
- **Heavy:** one sculpted mass shape (broad, deep, short neck), made once in Blender on the
  shared topology, baked into the heavy bases.

Because all four are one topology, **a wardrobe item authored on the reference body fits each
base by the shape's delta** (§4.2).

### 1.5 Underwear

The base body wears **briefs** (all bases) and a **band top** (feminine bases), made as
**material regions** of the body mesh itself: polygons on the `cloth` slot with their own
colors. They:
- add no triangles;
- never clip, since they are the skin;
- are recolored by the paint step like any region.

A worn item hides the body polygons under it (§4.4), underwear included.

---

## 2. Eyes

**What's wrong today:** each eyeball is a 10 × 7 sphere with the iris painted on a few flat
facets, set deep behind a lid hole. At the creator's camera the eyes read as white slits. At
the gameplay camera they read as holes.

### 2.1 The eyeball

- A sphere of about 120 triangles per eye on its own material slot, **`eye`**, shaded smooth
  (the one smooth surface on a faceted model: the face is read through the eyes).
- **The iris and pupil are drawn by the shader per pixel**, from each vertex's position on the
  ball (carried in `UV`):
  - the sclera, a soft limbal ring;
  - the iris with radial streaks from noise;
  - the pupil, and a fixed catchlight so the eyes look alive under the arena's flat light.
- The forge writes the recipe's eye settings into the eye's vertices: the iris color in
  `COLOR`, the pupil and iris sizes and the style in `UV2.y`. This costs no uniforms, so it
  works the same in the game and the creator.

### 2.2 Options (`looks.eyes`)

| Control | Range |
|---|---|
| Iris color | a palette of naturals (browns, hazels, greens, blues, greys) and others (amber, red, violet, white) |
| Iris size | 80–120% |
| Pupil size | 60–160% |
| Style | round, slit (beasts, orcs), glowing (the signature hue), blank (no iris), heterochromia (a second color for the right eye) |
| Sclera | white, yellowed, dark |

### 2.3 Lids

Today the lid faces are deleted, leaving the hole. Instead:

- The source head's lid surface is **kept**, split along the eye's line into an upper and a
  lower lid.
- Two **lid bones** per eye (`lid_upper_l`, `lid_lower_l` and the right ones) sit at the
  eyeball's centre, under `face_eye_*`. The lids are weighted to them and turn about the
  eyeball, sliding over it like real lids.
- The lids' **rest** is sculpted like any face bone (21 §2.2):
  - openness;
  - tilt (up at the outer corner, down);
  - inner-corner height.
- **Shape keys** do what bones can't:
  - **epicanthic fold** (the inner corner covered by a fold, many East and Central Asian eyes);
  - **monolid** (no crease above the lid);
  - **hooded** (the brow's skin over the crease);
  - **deep-set**.

  These are the only face morphs left. A bone can turn a lid but can't make a fold.

### 2.4 Life: gaze and blinking

A client-side **face life** driver poses six bones per champion every frame. This is visual
only and never in the sim or the clips.

- **Gaze:** small saccades every 0.6–3 s. The eyes look at the camera in the creator, and at
  the move's target or the walk direction in the game.
- The upper lid follows the eye's pitch (looking down lowers it), as real lids do.
- **Blinks:** every 2–6 s, 120–160 ms, now and then a double blink. Closing is faster than
  opening.
- Within the gameplay camera's distance, it costs six bone writes per champion. Beyond it
  (minimap, far crowds) it stops.

New bones: `eye_look_l/r` (children of `face_eye_*`, carrying the balls) and the four lids.
Clips may key the lids (an emote's squint), unlike other face bones.

---

## 3. Brows and lips as features

Today they are painted skin: homogenous and blocky. Each becomes a **feature** with a style, a
shape and its own colors.

### 3.1 Brows

- A **strip mesh** (about 40 triangles a side) conformed to the brow ridge, skinned to the brow
  bones, so it follows every face sculpt.
- Its look is a **style** from a catalog, drawn by hand as a thickness profile along the brow:
  - straight, arched, angled, soft;
  - bushy, thin, tapered;
  - unibrow, scarred (a gap), plucked.
- Per brow, with symmetry on or off:

| Control | What it does |
|---|---|
| Thickness | how heavy the brow is |
| Length | how far it runs to the temple |
| Arch | the peak's height and where it sits |
| Tilt | up or down at the outer end |
| Gap | the space between the brows |
| Density | strands from noise in the shader: sparse to full |
| Color | follows the hair, or its own |

### 3.2 Lips

Lips must deform with the mouth, so they stay part of the head mesh, but they become a feature
of their own:

- **Their own loops and region:** the vermilion border is a real edge loop, not a paint line.
- **Bones:**
  - `face_lip_upper` and `face_lip_lower`, sculpted separately for fullness, protrusion and
    height;
  - two new **cupid's-bow** bones on the upper lip (`face_lip_upper_l/r`) for its peaks;
  - the mouth corners (existing) for width and a smile or frown.
- **Shading:** a smooth gradient from a darker meeting line to the border, in the lip color
  (natural tones or paint). The shader draws it, not a painted band.
- **Asymmetry:** every lip bone sculpts per side with symmetry off (a sneer, a scar's pull).

### 3.3 Reach: any people

The face shapes (21 FR5) grow into **ancestry presets** that set the bones, the eyes, the lids,
the brows and the lips together. Two clicks give a recognisable start, and every control stays
open from there:

- East Asian, South Asian, West African, East African, Northern European, Mediterranean,
  Indigenous American, Pacific Islander;
- orc, dwarf, elf.

The owner reviews each preset in captures; they are data (`faces.txt`), not code.

---

## 4. Wardrobe: clothing, armor and accessories

### 4.1 What an item is

An item lives in `art/wardrobe/<slot>/<id>/`:
- `item.blend`, authored on the **reference body**: the slim masculine base at rest, its rig
  and helpers;
- `item.ron`, which says:

| Field | What it says |
|---|---|
| Slot and layer | `under` (shirts, leggings), `over` (tunics, coats, robes), `armor` (plates), `accessory` (belts, pouches, jewellery) |
| Coverage | the body polygons it hides, as a mask painted on the reference body |
| Rigid zones | plates and buckles, which take ±10% of the body's growth (21 §9.2) |
| Cloth chains | spring-bone chains for its loose parts (§4.5) |
| Paint regions | which faces recolor together |
| Builds | which builds it fits: by default all four humanoids |

Items carry **no** morph targets, so they cost nothing per slider.

### 4.2 Fitting, at build time

`mftr_blender.wardrobe.fit(item, base)` makes each base's copy of each item:

1. **Shape:** each base's body is the reference plus a delta on the same topology (§1.4). The
   item is bound to the reference body's surface (Blender's Surface Deform) and carried to the
   base's. A coat fits the heavy feminine body without being authored for it.
2. **Weights:** from the nearest body surface, the helpers included (built: `body_rig.transfer`).
3. **Layer offset:** `under` sits 3 mm off the skin, `over` 8 mm off whatever is under it, and
   `armor` 12 mm.
4. **Push-out:** any item vertex inside the body, or inside a lower layer, goes out along the
   body's normal to its layer's offset.

### 4.3 Fitting, at forge time (what the player does)

| What the player does | How the items follow |
|---|---|
| Proportions | Bones, which items share |
| Body sculpt (helpers) | By the transferred weights |
| Free sculpt offsets | Each item vertex takes its nearest body weld's offset (as head parts do now) |
| Several layers worn | Re-pushed against each other (cheap: only layered pairs, only once at forge) |

### 4.4 Hiding what's covered

Hiding moves from "the bones it covers" to **the coverage mask**: the body polygons fully
under every worn item go. The body under a tight shirt never pokes through, and the triangles
are spent where they show.

### 4.5 Secondary motion

Capes, skirts, coat tails, loose sleeves, long hair and braids get **spring-bone chains**:

- Godot's `SpringBoneSimulator3D` (4.4+), with collision capsules on the thighs, the hips and
  the spine.
- Visual only, client side, and frozen beyond the gameplay camera's detail distance.
- Chains are `extra_<chain>_<n>` bones (10 §7.2). The extras cap rises from 16 to 32, for a
  coat and a cape and hair together.

### 4.6 Quality bar and budgets

Above LoL season 1:

- **Silhouettes:** readable from the outline at the gameplay camera.
- **Shape and detail:** bevels on plates; real thickness on cloth edges; seams and trims as
  geometry where they catch the light.
- **Colors:** two to four paint regions per item.

| Item | Triangles |
|---|---|
| Shirt or leggings | 300–700 |
| Coat or robe | 800–1,400 |
| Full armor set | 1,500–2,500, over hidden body |
| Accessory | 50–300 |

### 4.7 Weapons

- Per **weapon class**: sword, axe, maul, hammer, spear, staff, dagger, bow, shield, orb,
  lantern.
- A grip frame and a **weight class** (light, medium, heavy), which picks the clip set: a maul
  rests on the ground before it's lifted, and holds after impacts (14 §3).
- 300–900 triangles each: iconic silhouettes, bevels, and a glow region in the signature hue.

### 4.8 QA harness

`mftr-tools wardrobe check` renders every item on every base:
- at neutral, and at each body slider's extremes;
- at five clip frames (idle, run contact, cast, the heavy attack's hold, death).

It writes a contact sheet per item and two numbers:
- **clipping:** body vertices outside an item that covers them;
- **gaps:** item vertices more than 3 cm off any body under them.

An item ships when both are under their thresholds and the owner signs the sheet. This is what
keeps a large catalog honest without a hand check of every combination.

### 4.9 Six builds

- **The four humanoid bases** share the wardrobe through one topology.
- **The creatures** (beast, serpent, spectre, machine) each have their own reference body.
  Items declare their builds, and the creator offers only what fits.
- **Adapters:** the spectre takes humanoid `over` items (robes, hoods, mantles) through a
  humanoid-to-spectre landmark warp. The beast takes saddles, barding and collars of its own.

---

## 5. Generative models: the rule reconsidered

The standing rule (2026-11-07, after the procedural face was rejected): **no procedural or
parametric generators for the look of characters**, because code-made shapes read as
programmer-made. The owner now asks for moonshot proposals. The objection was to *shapes made
by code*, not to tools that produce artist-grade shapes. Each proposal below is judged on
that.

| # | Proposal | What it gives | Risks | Verdict |
|---|---|---|---|---|
| M1 | **Sewing-pattern garments with cloth simulation** (Marvelous Designer's way, in Blender's cloth sim): an item is 2D panels with seams, simulated once onto each base at build time, then reduced and baked | Real drape and folds that fit each body; one pattern gives a family (lengths, cuffs, collars); the folds are physics, not code shapes | Sim tuning; sims can tangle (the QA harness catches it) | **Do it**: the strongest lever for a large, good-looking cloth catalog |
| M2 | **Text to champion:** describe a champion ("a scarred dwarf, braided beard, heavy plate, ember eyes") and an LLM fills the recipe (sliders, shapes, eyes, items, colors) | Fast, delightful starts; any people within one sentence | Needs a network call; the recipe stays the data, so the game stays deterministic | **Do it** as an optional creator button |
| M3 | **Photo to face:** fit the 24 face bones, the eyes and the lids to a photo's facial landmarks (an Apache-2.0 landmark model), by least squares | "Make me" in one click | Privacy: run it on the device and keep no photo | **Prototype** |
| M4 | **AI-assisted sculpts** (image-to-3D models) as first-pass references for armor and weapons, retopologized onto our rules by hand or by script | Speeds up a large catalog | Model licenses (some forbid commercial use or whole regions), provenance, a "generated" look | **Experiment, gated:** only permissively licensed models; their output is a reference, never shipped directly; provenance noted in each item's README |
| M5 | **Corrective shapes from simulation:** for each item, simulate the body sliders' extremes at build time and bake corrections, blended by the helpers' deltas at forge | Items keep their drape on extreme bodies, at no runtime cost | Build time | **Do it** after M1 |
| M6 | **Material from words:** a prompt chooses the parameters of our procedural materials ("weathered bronze, green patina") | Fits D4: parameters, not bitmaps | Small value | Later |

**Not proposed:** runtime neural deformation or neural rendering. Old hardware can't afford
them, and they break the deterministic, data-only recipe.

**Proposed decision (D77):** generative and simulation tools are allowed where they produce
**artist-grade shapes that a person reviews**:
- cloth simulation (M1, M5);
- landmark fitting (M3);
- recipe filling (M2);
- references for hand finishing (M4).

Shapes made by code alone remain out.

---

## 6. Order (track BW, after FR6's mechanism)

FR6 built the body's helpers, their drag and the forge on the lofted bodies (21 §9). Its last
steps (the other three bases, the body sliders as presets) move onto the new body: re-weighting
lofted tubes that are about to be replaced would be wasted.

| Slice | What | Exit |
|---|---|---|
| **BW1 Eyes** | Eyeballs on the `eye` slot with the shader iris; `looks.eyes`; lid bones and the kept lid surface; face life (gaze, blinks); lid shapes | Captures: eyes alive at the creator and the gameplay camera, on all four bases; a blink sequence; the owner's look |
| **BW2 Brows and lips** | Brow strips and their catalog; lip loops, cupid's-bow bones, lip shading; per-side controls | Captures of every brow style and lip extreme |
| **BW3 The body** | The CC0 body reduced, fitted, weighted; underwear regions; the helpers re-weighted; slim masculine first | The owner signs front, ¾ and side captures, bare and in underwear |
| **BW4 Four bases** | Sex blend and heavy shape; the body sliders as presets on the helpers (FR6 step 4) | All four bases; old recipes load |
| **BW5 Wardrobe pipeline** | Item format, fitting, layers, push-out, coverage masks, the QA harness; the old catalog retired as new items land | Three items per slot pass the harness on all four bases |
| **BW6 Motion** | Spring-bone chains for cloth and hair | A cape, a skirt and braids in a run cycle |
| **BW7 Weapons** | The weapon classes, rebuilt, with weight classes and their clips | Each class in a captured attack |
| **BW8 Creatures' wardrobes** | Reference bodies and adapters for the four creature builds | Items on every build |
| **M1–M3** | §5's moonshots, alongside BW5 onward | Per proposal |

## 7. Open questions for the owner

1. **Realistic over stylized topology** (§1.1), with heroic proportions from the rig: agreed?
2. **Underwear as regions of the body** (§1.5), always on: agreed?
3. **D77** (§5): which moonshots to start, and the license stance for M4?
4. **Ancestry presets** (§3.3): the list, and who reviews them for respect and accuracy?
