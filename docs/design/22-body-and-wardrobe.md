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

*(As built, BW6: each part has chains of its own, `extra_<part>_<chain>_<n>`; the base carries
them all and the forge keeps the worn parts'. They swing in the champion's own space, with a
headwind from its velocity: see the BW6 status below.)*

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

**The owner's call (2026-11-12):**

- **M1 goes ahead**, with the wardrobe pipeline (BW5).
- **M5 is likely.** It depends on how the slider extremes treat the wardrobe in the QA harness
  (§4.8).
- **The LLM tie-ins are for later:** M2, M3, M4 and M6, and an MCP server that lets a model
  make assets and characters from text.

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
| **BH1 Hair** | Hairstyles groomed by physics on every base (§7 item 5's answers); strands in the shader; hair hidden under headwear | Ten styles on all four bases; captures |
| **BH2 Hair controls** | A length control; beards and moustaches on the same system; the shared hair, brow and beard color with a separate hair color | Captures of each length and beard |
| **BW6 Motion** | Spring-bone chains for cloth and hair | A cape, a skirt and braids in a run cycle |
| **BW7 Weapons** | The weapon classes, rebuilt, with weight classes and their clips | Each class in a captured attack |
| **BW8 Creatures' wardrobes** | Reference bodies and adapters for the four creature builds | Items on every build |
| **M1–M3** | §5's moonshots, alongside BW5 onward | Per proposal |

### BW1 status (2026-11-12): eyes built, lid shapes not yet

**The eyeballs.** Each is a smooth-shaded sphere on its own look bone (`eye_look_*`), with the
back cut away. It sits 10.8 mm round its centre, just behind the lids' rim. **Departs from
§2.1:** no new material slot. An eye vertex's `UV` (2 and up, past any paint UV) marks it and
carries its place on the ball, its style and its iris size. Its `COLOR` carries the iris and
the pupil. So the champion shader draws the eye in the game, the creator and the portraits
alike, with no uniform and no buffer.

**The shader** draws, per pixel:

- the sclera, shadowed toward the corners;
- the iris, with radial streaks, lighter round the pupil and a dark limbal ring;
- a round or slit pupil, and a catchlight;
- glowing (the signature hue) and blank styles.

The eye is lit softly, never banded.

**`looks.eyes`** (`mftr_forge::eyes`), written only when it differs from the default:

| Setting | Range |
|---|---|
| Iris color | 16: naturals, then amber, gold, red, violet, white, teal |
| Right eye | its own color (heterochromia), optional |
| Iris size | 80–125% |
| Pupil | 0–10 |
| Style | round, slit, glow, blank |
| Sclera | white, yellowed, dark |

The skin tone leaves the eyes alone.

**The lids** are shells: spherical bands round each eyeball on `lid_upper_*` and `lid_lower_*`.
The upper shell rests over the iris's top, its margin darkened as a lash line. A blink turns it
0.86 rad down to meet the lower, which rises 0.12 rad.

**Socket walls** run from each opening's rim in behind the eyeball. Before them, a look past the
ball at the corners saw through the head to the background. They are weighted on `head`, so
the face rig smooths them with the skin.

**Life** (`client/scripts/face_life.gd`) poses the six bones each frame:

- saccades every 0.6–3 s;
- a blink every 2–6 s (60 ms closing, 100 ms opening), sometimes double;
- the upper lids following the eyes' pitch.

In the creator the eyes find the camera. In a match they look ahead. `--creator-life
BLINK,YAW,PITCH` holds a pose for captures.

**The creator's Eyes panel** on the Face step: iris swatches (per eye with "Each eye its own
color"), iris size and pupil sliders, style and sclera rows.

**Checked:**

- Captures:
  - open, half-blink, closed, and a look up and to the side;
  - three-quarter view;
  - six settings across the four bases (green; heterochromia; a large blue iris; blank on dark;
    amber slit on yellowed; glowing);
  - a practice match with the HUD portrait.
- All eight creator bases and the neo set validate.
- Tests: the eyes' text round-trip and strict reader; `apply` writes each eye's iris and keeps
  its place on the ball.

**Cost:** about 520 more triangles than the old eyes (a bare slim masculine is 8,576). The
forge test's bound is 9,000 until the new body resets the budget (§1.2).

**Not yet:**

- **The owner's look.**
- **Clips may key the lids** (§2.4), but no clip does yet.

### Lid shapes status (2026-11-12): built

**The opening, redone.** The lids used to open by an ellipse cut across the source's eye loops,
so the rim zigzagged between two loops: notched lids with flaps of white at the corners. Now:

- The opening follows the source's own topology: the faces within one edge of the seam where the
  closed lids meet go, so the rim is the first eye loop, whole.
- Each rim vertex is set on one almond contour (`authored_head.RIM`), with:
  - the upper lid highest a little toward the nose, the lower lowest toward the temple;
  - the outer corner 1.2 mm above the inner.
- The rim stays at least 12 mm in front of the eyeball's centre, clear of the lid shells.
- The lid shells' margins follow the same contour (`rim_z`).
- The rim is darkened through the occlusion alpha: a lash line on the upper lid, a waterline on
  the lower.
- A new loop between the rim and the next ring gives the crease, fold and hood their geometry:
  +88 triangles a head.

**The sliders** (the Face step's **Lids** group):

| Slider | What | How |
|---|---|---|
| Openness | narrow ↔ wide | morph `lids_open+` / `-`: the rim and the skin round it slide over the ball; the shells turn with it |
| Crease | none (monolid) ↔ deep | morph `lids_crease-` / `+`: the new loop sinks into a fold line, or the skin above comes down full to the lash line |
| Inner fold | one-sided: epicanthic | morph `lids_fold+`: the upper lid's skin comes down and across the inner corner, a soft ridge into it |
| Hood | one-sided: hooded | morph `lids_hood+`: the skin under the brow hangs over the crease, more at the outer corner |
| Inner corners | low ↔ high | morph `lids_inner~` |
| Depth (Eyes group) | prominent ↔ deep-set | a preset on `face_eye_*` (`eyes_depth`): the whole eye moves back under the brow |

The tilt was already `eyes_tilt`, a turn of the eye bones.

**How the shapes are made.** Each morph is a field over the skin round the opening, in
millimetres against the opening's ellipse, like a modeler's proportional edit (`LID_SHAPES`):

- The skin slides over the eyeball as it moves, so it never cuts into it.
- The shells turn about the ball by the rim's move over their columns.
- The eyeballs stay put.

**Changes elsewhere:**

- **Pack rules:** a lone `<id>+` morph is now allowed, a one-sided slider. A `-` still needs its
  `+`.
- **The creator:** a one-sided slider runs from none to full, not −1 to 1.
- **Captures:** `--creator-zoom Z` takes close-ups.

**Checked:**

- Captures of the slim masculine base, every slider at its ends, front and three-quarter.
- An East Asian combination (inner fold 0.8, crease −0.8, openness −0.35) on the slim masculine
  and feminine bases.
- A hooded, deep-set heavy masculine base, and a monolid heavy feminine one.
- Half and full blinks, with and without the shapes.
- A test: all four bases have the six sliders, and each lid morph moves only vertices within
  4 cm of an eye, by at most 6 mm.

**Left:**

- The thin light sliver at the inner brow (the painted brow's split seam) goes with BW2's brow
  strips.
- The ancestry presets (§3.3) will set these sliders. The face shapes' reader then has to accept
  morph sliders too.

### BW2 status (2026-11-12): brows and lips built

**Brows are strips, not paint.** Each humanoid base carries a strip over each eye: 13 columns
from over the nose to the temple and 4 rows across the brow line (72 triangles a brow), found on
the head's surface by rays and lifted 0.6 mm off it. The strip is an envelope, wide and tall
enough for any brow. `mftr_forge::brows` shapes it at forge time:

- it slides the columns between the brow's start and its tip;
- it sets the rows by the style's thickness and arch;
- it stands the middle rows off the skin for the brow's volume.

So the silhouette is real geometry, with no transparency, which suits old hardware. The champion
shader draws the hairs: strokes leaning up at the start and lying flatter toward the tip, darker
roots, lighter ends. The strip rides the face bones as the skin under it does, so the brow
presets and drags carry it.

- **Styles are data** (`art/creator/heads/brows.txt`): soft (the default), straight, arched,
  angled, tapered, bushy, heavy, thin, plucked, unibrow (it runs in to meet its twin), scarred
  (split by a cut) and none. Each is two seven-point profiles, thickness and arch.
- **Per brow** (`looks.brows`, 0–10, 5 the style as drawn): thickness, length, arch, peak (toward
  the nose or the temple), tilt, gap and height. **Each brow its own shape** gives the right one
  its own settings, for asymmetry. Old recipes read the default.
- **Color:** the brows stay the body's paint region 3, as recipes painted them before (now 2:
  `brows::color_region` finds it; see Quill, 2026-11-18). The Brows
  panel's swatches set that fill.
- **Departs from §3.1:** no density slider. A sparse brow needs the skin's color behind the
  strokes, which needs transparency or another color per vertex. The strokes' contrast stands in
  for it.

**Lips are the mouth's own loops.** The lips used to be a box of faces split from the skin, with
blocky corners. Now:

- The lips are the faces within two edges of the closed mouth's seam.
- Each ring is set on one outline, its upper and lower lips meeting at the corners, on the
  head's own surface. The source's corner loops run unevenly, and that forked the corners.
- The mouth is 20% wider than the source's seam.
- The lips are smooth-shaded, the one smooth skin on the faceted face.
- The shader draws their gradient (`UV` = 12 + across, 3 + up): darker where they meet, fullest
  in each lip's middle, a sheen on the lower lip.
- The skin tone tints them, so darker skin has darker lips.

The Mouth group gains three morphs; the skin within 7 mm of the lips follows their border:

- **Upper lip**, thin to full (`lips_upper~`);
- **Lower lip**, thin to full (`lips_lower~`);
- **Cupid's bow** (`lips_bow+`).

Width, overall fullness, height and the corners stay the bone presets (`mouth_*`). Asymmetry
(a sneer, a scar's pull) is the Features drag with Symmetry off. **Departs from §3.2:** no new
cupid's-bow bones. A morph gives the bow without changing the face rig's bone set and its saved
forms.

**Feature UVs:** eyes 2–10, brows 10–12, lips 12–14. `eyes::is_eye` now checks its own range.

**Checked:**

- Captures:
  - every brow style; thick and tilted; asymmetric;
  - the lips at default, thin, cupid's bow and full, front and three-quarter;
  - an arched brow with a cupid's bow on the slim feminine base, heavy brows on the heavy
    masculine (orc mix), tapered brows and full lips on the heavy feminine;
  - the Brows panel;
  - a practice match and its portrait.
- Tests:
  - the styles file's reader;
  - the brows' text round-trip and strict reader;
  - every style stays in its envelope, and each setting moves the brow its way;
  - every base has strips, its brow region at index 3, and the lip sliders;
  - a thicker brow reaches higher, and `brow_height` lifts the strips.

**Not yet:**

- Free sculpting (the brushes) doesn't move the brow strips; the face bones do.
- The ancestry presets (§3.3) need the face shapes' reader to accept morph sliders.

### BW3 status (2026-11-12): the CC0 body on all four bases

**All four humanoid bases now wear the artist-made body** in place of the lofted tubes
(`mftr_blender.cc0_body`; `creator.BODY = "lofted"` brings the old one back).

**The source.** `art/creator/bodies/body_source.blend`: the realistic male and female, each
un-subdivided twice (3,201 vertices, the same topology), made by `tools/blender/extract_bodies.py`
from the bundle. The head's README covers the license.

**The joints** come from the bundle's primitive bodies, whose parts' origins are the joints. The
male primitive's arms hang at another angle from the realistic mesh's, so each arm's chain is
turned onto the mesh's own arm (a line fit through it).

**Departs from §1.3: skinning, then a bone-by-bone fit, not a thin-plate warp.** A warp across
arms that hang 20° apart bent the shoulders. So:

1. A skeleton with the biped's names goes on the source's joints, and Blender's automatic weights
   skin the mesh to it: at most two bones a vertex; the underwear's split seams share one set.
2. Each vertex goes by those weights onto the build's rig. Every bone is turned onto the rig's
   bone of the same name, stretched to its length and thickened by the build's girth. The
   torso widens by height, from the hips' ratio to the shoulders'.
3. The weights come along unchanged, since they are already the rig's.
4. The soles are squashed onto the floor below the ankle.

So the heavy builds come from the same mesh by the rig and the girth. Their own sculpted mass
shape (§1.4) is still to come.

**The neck and the head.** The authored head is closed underneath. The body is cut a little
above its neck joint, and `raise_neck` raises the neck's top in two rings into the head:

- the top ring is 12 mm above where a line straight up meets the head's underside, capped at
  the head's joint so a line past the chin never finds the lips;
- the rings narrow 18% toward the neck's axis, so the back of the neck slopes into the skull;
- the middle ring is 60% the neck's and 40% the head's.

The face rig's seam rings start at the neck as before, and head parts don't take the neck's
weights.

**The helpers on two bones a vertex.** On the lofted body every vertex had one bone; now it can
have two. So `body_rig` shares what the helpers leave among the vertex's own bones, and caps each
vertex's helpers at four influences in all. Before, glTF dropped the fifth influence and
renormalized, and that broke the weight-step rule. The head, hands and feet stay in the
step-limit graph at zero.

**Underwear (§1.5)** is regions of the body on the cloth slot: briefs on every base, a band top
on the feminine ones, each with a trim. They are the body's paint regions 0 and 1, so recipes
that painted the old undersuit's two still forge. The brows are region 2 now (they were 3).

**Triangles:** about 4,600 below the neck. A whole base is 7,722 (masculine) or 7,858
(feminine, with the band top's split), eyes, brows and lips included: under the old lofted ones
(8,768) and well inside the 5,000–10,000 target.

**Checked:**

- Captures of all four bases, front and three-quarter.
- The neck from the front, side, back and three-quarter; no open edge below the head but the
  hidden top ring.
- Cairn-, Bastion- and Rook-neo dressed in the old catalog.
- A practice match.
- All eight creator bases and the neo set validate.
- Two forge tests were loosened, with the reason noted in each: a chest drag now moves 15 mm or
  more (it was 20), and each helper owns 5 vertices or more (it was 8). The CC0 chest and hips are
  a few big faces and their helpers peak lower; BW4 retunes the patches with the presets.

**Not yet:**

- **The heavy builds' own mass shape** (§1.4). Today they are the slim mesh, broadened by the rig
  and the girth.
- **A notch at the throat** on the feminine and heavy bases, where the neck's raise meets the
  front of the cut.
- **The briefs' leg openings** zigzag at the back. They follow faces, not a drawn line.
- **The old catalog sits loose on the new limbs**, as it was made for the tubes. It is replaced
  in BW5.
- **The body sliders and the helpers' presets** (BW4).

### BW4 status (2026-11-12): the body sliders on the helpers

**The seven body sliders are presets on the helpers**, the face sliders' way (21 §2.7):
`art/creator/bodies/presets.txt`, read by `mftr_forge::body::presets`. That is FR6's step 4.

- **Same ids as the old morphs**: `body_bulk`, `body_belly`, `body_chest`, `body_hips`,
  `body_shoulders`, `body_muscle` and `neck_thickness`. A recipe's `shape: (body_bulk: 0.6)` now
  moves the helpers, so old recipes forge unchanged. The Cairn-, Bastion- and Rook-neo tests pass.
- **Helper frames:** the torso's helpers stand upright (+x out to the left, +y up, +z forward);
  the limbs' lie along the limb. The file says so.
- **Close to the limits.** On the CC0 body the helpers' weights peak at 0.3–0.6 on the torso and
  0.6–0.9 on the limbs, so each preset runs close to its helper's limits, which keep the
  silhouette inside 20 §6.2's bounds. A slider at 1 is a clear change, and sliders summed together
  are clamped.
- **Fewer morph targets:** a base with the helpers no longer carries the body sliders as morphs.
  It keeps the face's ten.

**The helpers retuned on the new body:**

- The torso patches grow with the build's shoulder spread (×0.8–1.35). On the heavy masculine
  base, the chest's peak rose from 0.31 to 0.41 and the trapezius's from 0.22 to 0.40.
- The glutes' patch moved down onto the CC0 buttocks. Every helper again owns 8 vertices or
  more, so that forge test is back at its old bar.
- The chest test stays at 15 mm (it was 20). The masculine chest helper peaks at about 0.47,
  shared with the back and the trapezius.

**Checked:**

- Captures of every slider at its end on the slim masculine base, and bulk, belly, chest and
  hips on the slim feminine.
- A test: the seven presets exist, are symmetric and within limits, and compose (a belly comes
  forward).
- All eight bases and the neo set validate.

**The gaps closed (2026-11-12), at the owner's call before BW5:**

- **The heavy builds' own mass (§1.4)**, baked in: each heavy build has a mix of the body sliders'
  presets in `creator.BUILDS` (`mass`). The masculine's is muscle, shoulders, a thick neck and a
  little belly; the feminine's is muscle, hips, neck and chest. `cc0_body.bake_mass` carries it
  through the helpers' weights, the forge's way, into the body and every part, and moves every
  shape key alike. The shape is the presets' data, not code. The heavy masculine now reads as a
  brute, the heavy feminine as strong and full.
- **The throat notch:** the neck's top ring now sits 20 mm inside the head (it was 12) and is
  capped with a fan, so where a jaw's corner leaves it showing, it shows skin.
- **Clean underwear edges:** the garments' edges are cut into the mesh along planes (`_cuts`,
  bmesh's bisect) before skinning: the waist, the leg openings rising from the crotch to the
  hips, the band top's edges. Each face is then skin, cloth or trim by those planes, so a
  waistband and the band top's hem are crisp 12 mm trims, and the leg openings are clean lines
  front and back. About 220 triangles more.

Bases are now 7,938 (masculine) and 8,284 (feminine) triangles. Left: the sex blend as a
slider, if the owner wants one (§1.4).

### Quality pass (2026-11-12): shoulders, the neck from the source, a sculptable chest

The owner's review: the feminine shoulders looked pulled in; the neck's join was low quality;
the reduction had cost the torso the polygons a chest needs.

**Shoulders.** Two causes:
- The fit mapped the source's neck joint (12–15 cm above its shoulders) onto the rig's, which
  sits only 3 cm above them, so it lifted the shoulders into a shrug, worst on the feminine body.
- The automatic weights tied the skin by the spine to the clavicles.

Now:
- The source's spine runs to a top set at the rig's own shoulder gap (`spine_top`).
- The rig has a `drop` shape parameter that lowers the clavicles' tips and the arms: 5 cm slim
  masculine, 6 feminine, 3.5 and 4.5 for the heavy builds.
- Near the spine, the clavicles' weight goes to the chest (`_spine_off_clavicles`).

The shoulders now slope from the neck as a real body's do.

**The neck is the source's own.** The authored head replaces only the source's head:
- the source keeps its neck and is fitted with it;
- `marry_neck` drops every vertex the head bone weighs at half or more, so the face, jaw and chin go;
- the neck's top edge is seated into the head's underside: each point goes up a line to 12 mm
  inside it, drawn in toward the neck's middle where a line would pass the chin or reach up
  under an ear, its neighbours along the edge drawn in evenly, and capped inside.

The neck keeps the artist's throat, nape and tendons instead of a raised tube.

**A sculptable chest.**
- `body_source.blend` now holds each body's sculpted surface (the multires at its first level,
  about 42k vertices) as `body_<sex>_hi`, and the reduced body is made from the cage after
  Blender's "Apply Base" fits it to the sculpt.
- `densify` projects every reduced vertex along its normal onto the sculpted surface, then
  subdivides the chest and bust (from below the bust to under the collarbones, round the sides)
  once, smoothly, projecting the new points too.
- The fit uses every bone's weight, and the two-bone budget is applied after: cutting first made
  the skin step and crumple where a third bone dropped out.
- The body is triangulated along each quad's flatter diagonal, so curved quads don't fold the
  wrong way on the faceted skin.

The chest and bust carry about four times the faces they did, on the artist's surface.

**Other changes:**
- The band top's sides are cut on a vertical plane at the armpit.
- The heavy builds' neck girth is lower (1.2 and 1.15, from 1.55 and 1.4), and their baked neck
  thickness too, since the realistic neck is thick already.
- Parts fade a helper that doesn't fit under four influences, rather than dropping it.

Bases are 8,588–8,818 triangles.

**Checked:** captures of all four bases; the necks front, three-quarter and back; the chest at
rest and with the chest slider at full on both slim bases; all bases validate.

**Left:**
- A small notch under the heavy masculine's chin, where its wide jaw (the head's Ork mix) leaves
  the neck's front edge showing.
- The band top over the bust shows its cut slivers a little.

### Second review (2026-11-12): the throat, head size, underwear, the heavy mass

The owner's notes on the quality pass:
- the throat read as a frilled collar on the masculine bodies, with a gap at the feminine's side;
- the heads looked right only with Head size at -0.8 (slim) or -0.5 (heavy), and the slider
  hardened the feminine jaw;
- the underwear still had gaps;
- the heavy builds' mass needed work.

**The throat.**
- `marry_neck` now cuts the source's neck on one plane, running from 3.5 cm above the neck joint
  at the throat (never higher than just under the chin) to behind the skull's base. The source's
  jaw floor, which made the frill, goes with the head.
- The front of the neck's top edge rises straight up into the jaw's underside. A point whose
  line misses the jaw is drawn back toward the neck's middle, as far as its neighbours are, so
  the front edge stays even. The back and sides go to the nearest point of the head's lower
  surface.
- The back and sides rise straight up into the skull's base, drawn in toward the middle only
  where a line would miss or reach past the ears, so the nape keeps its line. Snapping them to
  the head's lower surface had pulled the nape forward, under the jaw (the owner's third note).
- The seam's vertices are wholly the head's.
- Neck girths are 1.0 or less: the realistic neck is thick already.

**Head size**, built in rather than slid: the base heads are 0.8 of their old size on the slim
builds and 0.875 on the heavy ones, the owner's -0.8 and -0.5. The slider shrank the head bone
and left the jaw's skin, shared with the neck, big; building the head at that size keeps the
face's proportions, so the jaw isn't hardened. Head size 0 is now the right head.

**The briefs' sides** are at least 5 cm deep (`BRIEF_SIDE`, its own cut plane): the leg
openings' climb stops there, so the briefs no longer thin to a strip, a gap, at the hips.

**The heavy builds:**
- shoulders lowered more (`drop` 7.5 and 7 cm);
- the torso's girth tapered (1.12 and 1.1, from 1.3 and 1.2);
- the masculine's mass mix is muscle, chest and shoulders, with little bulk and no belly: a
  muscled brute, not a fat one;
- the feminine's is muscle, hips, chest and a little shoulder.

**Also:**
- The body is made mirror-exact at the end of the build (`cc0_body.symmetrize`): the
  projection and the neck's seating left a few millimetres of drift, past the brushes' 2 mm
  mirror pairing.
- **A bug fixed:** a sculpt stroke over the 1,000-weld cap was cut by weld id, which could drop a
  weld's mirror. It now keeps the strongest offsets, a pair together.

### BW5 status (2026-11-12): the wardrobe pipeline, sewn garments, the QA harness

**Items** live in `art/wardrobe/<slot>/<name>/` (`art/wardrobe/README.md`):
- `item.ron` holds the id, slot, name, layer, accent and rigid flags;
- `item.blend` holds one mesh, `item`, authored around the reference body (`body_male` in
  `body_source.blend`);
- an optional `build.py` makes it.

**Fitting at build time** (`mftr_blender.wardrobe`), as §4.2 proposed, with one change. The
bases don't share a topology after the neck's marriage and the underwear's cuts, so Surface
Deform's same-topology binding doesn't apply. Instead:

- each base body records every vertex's place on its source body before the fit (`src_co`);
- an item vertex is bound to its nearest few faces (barycentric place, offset in each face's
  frame) and carried, first onto the female source body for the feminine bases (it shares the
  male's topology), then onto the base, from its source places to its fitted ones;
- then pushed out to its layer's distance from the skin, and weighted from the body under it:
  two biped bones, then the helpers (`body_rig.transfer`), so it follows the body sliders and
  the sculpting.

It runs after the heavy builds' mass is baked, so their cloth sits on their own bulk. In the
base, an item is a catalog part flagged `covers`, with its `layer`.

**At forge time** (`mftr_forge::wardrobe`):

- **Coverage (§4.4):** the body triangles under a worn covering item are hidden, by geometry. A
  vertex is covered when its outward ray meets the item within 12 cm, on an item face looking
  the same way, so a tunic hides the torso it covers but not the inner arms beside it.
- **Layers (§4.3):** for each pair worn together, the outer garment is pushed 6 mm clear of the
  inner. The push is a smooth field, each vertex taking the strongest push within 5 cm, fading:
  pushing single vertices folded faces over and opened holes. The inner garment's faces under the
  outer are then hidden, as the body's are. A garment's lining (its inside near the openings,
  marked by a dark occlusion alpha) is left out of the push, since it faces the wrong way.
  **Tucked in:** over an item of a tight slot (`feet`, `arms`, `hands`: boots, bracers, gloves),
  it goes the other way. The inner garment is taken in under the outer, up to 8 cm out, in three
  rounds of the same smooth field, and the outer keeps its shape. A shell a few millimetres off
  the skin pushed out over loose trousers went lumpy, and the trousers still showed through.

**M1: sewn garments** (`mftr_blender.sewing`). A pattern is data: flat panels (named outline
points) and seams (outline stretches, sewn start to start). The build:

1. Panels are meshed by a constrained Delaunay of their outlines and an inner grid (3.5 cm),
   and set around the reference body.
2. Seams become Blender cloth's sewing springs. The body collides. Gravity is off while the
   seams close, then the cloth falls and drapes, over 120 frames.
3. Every stitch's ends are merged at their mean. The trims (the faces near the open edges) go on
   their own slot and color.
4. A lining 4 mm in is added only within 7 cm of the openings: thickness where a look into a
   neckline sees it, at about a third of a full solidify's cost.

Each item's `build.py` runs the sim in Blender and writes its `item.blend`.

| Item | Pattern | Triangles |
|---|---|---|
| `torso.linen_tunic` (over, accent trims) | front and back, shoulder and side seams, flaring to mid-thigh | 1,693 |
| `legs.wool_trousers` (under) | four panels, outer and inner leg seams, middle front and back | 1,723 |

**The QA harness (§4.8)**, `mftr-tools wardrobe check [art]`, puts every covering item, and
each pair of them in layers, on every humanoid base, at neutral and at each body slider's ends:
180 checks, about a second in release. Per check:

- **poke:** skin poking through the outermost item (a shown body vertex whose inward ray meets
  the item within 1.5 cm);
- **gaps:** item vertices more than 3 cm off the body (drape counts, so this is information);
- an outfit fails on more than 12 pokes. The command exits non-zero, so a large catalog stays
  honest.

**Today's harness, at neutral** (pokes; 12 or fewer passes):

| | Trousers | Tunic | Both |
|---|---|---|---|
| Slim masculine | 5 | 9 | 7 |
| Slim feminine | 1 | 17 | 9 |
| Heavy masculine | 9 | 24 | 23 |
| Heavy feminine | 2 | 66 | 63 |

The harness found the tunic's first cut too tight round the arms, so its armholes were cut deeper
and its straps narrower, which took the slim masculine under the line. The trousers pass
everywhere. The tunic's pokes on the other three bases are at the armholes, more on the
right: the sim drapes the two sides a little differently. A thin spike at its right shoulder
seam is a weld artifact, and a cluster at chin height on the heavy feminine is likely that
spike's reach. The body sliders change the numbers little. 86 of the 180 checks fail, nearly all
the tunic's.

**Checked:**
- Captures of both items on all four bases, alone and together, in the creator (animating) and
  in a practice match.
- Tests: coverage hides the skin under an item and nothing else; the tunic is a covering part
  of every base and hides the torso but not the head or feet; layered, the trousers under the
  tunic go, and the trousers alone pass the harness.

**Not yet:**
- **The tunic passing the harness:** the shoulder seam's spike (a stitch group welding vertices
  that aren't neighbours), a symmetric drape (simulate one half and mirror it), and the heavy
  feminine's armholes. Then the harness can become a test.
- **More items:** three per slot is the exit, so a sleeved shirt (panels in any plane, for the
  sleeves), a skirt, a coat; armour and accessories modelled by hand.
- **The old catalog** is still offered beside the new items. It retires as each slot fills.
- **M5's simulated correctives** if the harness shows the slider ends need them. So far they
  barely change its numbers.
- **The small hole where the trousers' four seams meet at the crotch.**

### BW5 second pass (2026-11-12): five items, the tunic passing, the trousers closed

**Items** (all sewn, M1):

| Item | Slot, layer | Pattern | Triangles |
|---|---|---|---|
| Linen tunic | torso, over | front and back, mirrored half | 1,651 |
| Wool trousers | legs, under | four panels with a tailor's crotch extension | 1,619 |
| Sleeved shirt | torso, over | front, back, two sleeves of two halves each, drawn along the arm | 2,078 |
| Wrap skirt | waist, over | front and back, flared to the knee | 1,688 |
| Long coat | back, armor | back and two open front panels, below the knee, mirrored half | 3,300 |

**Sewing** (`mftr_blender.sewing`):
- panels can be turned about a pivot (sleeves);
- symmetric garments keep one simulated half, mirrored, since the sim drapes the two sides a
  little differently;
- seams are welded by stitch groups;
- small holes where seams meet are filled.

**The trousers' hole** was the four seams meeting at the crotch with too little cloth between
the legs. A crotch extension (the panels reach past the middle, more at the back, as a tailor's
pattern does) closes it.

**Fitting, fixed after the harness and the captures:**
- **Peaks:** a body point under one of an item's big faces (a bust's tip) pushes that face out.
  This applies only to points directly under the face, with 2.5 cm at most over all rounds;
  points beside a face pushed a coat's free-hanging sides out into "wings".
- **Arms:** garments on the torso, waist, legs and back are carried, pushed and weighted
  without the arms below the shoulder (by bone on the base, by place on the source bodies). The
  arms hang at each body's own angle and would carry a coat's sides with them.
- **The carry:** far from the skin a vertex follows the surface's move, not its turn, and only
  the truly nearest faces count: a wider net reached the waist's other side, whose move
  cancelled the narrowing.
- **The heavy builds** fit with 1.8× the layers' distances off the skin.
- **Layers at forge time:** an outer garment's vertex is pushed only by an inner face looking the
  same way, never more than 2 cm in.

**The harness's measure** now counts skin under a garment's outside only. Skin over its lining
(4 mm in, near the openings) can't be seen, and was counted before.

**The harness** (720 checks): every item and layered pair passes at neutral on every base but
one, the shirt on the heavy masculine (22 pokes): its sleeves, simulated on the slim arms, are
too narrow for that build's arms, and want their own fit per build. A test,
`every_wardrobe_item_passes_the_harness_at_neutral`, holds the rest and names that exception.
At the body sliders' far ends 46 checks miss narrowly, M5's to correct.

**Release:** `scripts/package-client.sh` ships `art/creator/*/export` (its own check fails on
any export folder it doesn't list); the Nix package's source includes the five art files the
forge embeds.

### BW5 third pass (2026-11-12): sleeves along the arm, all items passing at rest

- **The shirt's sleeves** are drawn along the reference arm's own line (from the shoulder joint
  toward the wrist, a band 24 cm down), not turned about a guessed pivot. They hang with the arm,
  with cuffs, instead of standing out puffed.
- **The peak push** goes up to 5 cm over eight rounds. With it acting only on body points
  directly under a face, it can safely reach the heavy masculine's deltoids under the sleeve
  caps.
- **The coat** is one simulated half, mirrored: the sim's spike at one waist is gone.

**The harness:** every item and layered pair passes at neutral on all four bases, and the test
has no exceptions now. 14 of the 720 checks miss narrowly at the body sliders' far ends.

**Left:** a small slit under each sleeve where its two halves meet the body.

### BH1 status (2026-11-12): ten hairstyles, groomed by physics

The owner's answers (§7 item 5) asked for shaped hair meshes. Blender's CC0 bundle has no hair,
and shapes made by code alone stay out (D77), so hair is made the way garments are sewn (M1): a
style is a **grooming pattern**, and physics does the shaping (`mftr_blender.grooming`,
`art/wardrobe/hair/<style>/groom.py`).

**The pattern:**
- **A cap:** the scalp above a hairline. The hairline is drawn once on the source head (the
  forehead, the temples, the sideburns, round the ears, the nape). The cap is lifted off the skin
  by the style's volume, thin at its edge, and the edge is cut along the hairline itself, not round
  the skin's faces.
- **Locks:** they root on the scalp at a spacing, or at given points. Each pattern sets the locks'
  length (by where they root), width, thickness, the way they're combed, and their stiffness.
- **Ties:** locks can be drawn to a tie and held there (a ponytail, a topknot, braids), with a
  band round it.

**Grown on each base, at build time:**
1. Each lock is combed from its root along its flow, over the skin.
2. Blender's cloth simulation drops it under gravity onto that base's own head and shoulders,
   with its root and its tie held.
3. The settled ribbon gives the lock its body: a lens or a round across, tapering to the tip, or
   a braid's plait.

The cap and the locks carry the strands' coordinates (UV 14–16), which the champion shader draws
as strands, darker at the roots and lighter at the tips, like the brows' hairs. The hair takes
the head skin's weights, face bones included, so it follows the sculpt. A lock that reaches the
neck and back takes the body's weights there, without face bones (21 §1.5).

**The styles:**

| Style | Triangles (slim masculine) |
|---|---|
| Buzz cut | 385 |
| Short | 1,276 |
| Swept | 1,485 |
| Long | 1,836 |
| Ponytail | 1,781 |
| Topknot | 1,647 |
| Crest (mohawk) | 1,088 |
| Braids | 1,730 |
| Locs | 1,716 |
| Coils | 1,284 |

The old block-out hair parts are gone. The new styles keep their ids (`hair.short`, `hair.long`,
`hair.ponytail`, `hair.topknot`, `hair.mohawk`, `hair.braids`), so recipes still load.

**Under headwear** (the owner's call: hair hides under a helmet), the forge hides the hair a
headwear part covers (`wardrobe::hide_hair`). A hair vertex is covered when the line from the
skull's middle out through it meets the headwear. So a helmet hides the hair under it and any
that would poke through, while long hair falling below it stays. A hood with a neck hides a
ponytail whole.

**Left for the look:**
- the heavy builds' braids kink where they meet the bigger shoulders;
- coils read as knots more than as a rounded mass;
- the short style's back is still a little ragged.

### BH2 status (2026-11-13): length, beards, one color

**The length control.** Each style is grown three times on each base: as patterned, shorter
(0.55×) and longer (1.7×). The locks get longer segments, not more of them, so all three have the
same mesh. The differences are the hair's morph targets `hair_length-` and `hair_length+`, and the
forge blends them like the body's sliders. A tie's locks always reach the tie, and a topknot's
coil keeps its size. The buzz cut and the coils have no length control.

**Beards** are grown the same way, on a `beard` slot of their own (next to the hair, before the
head; a mask stays on `face`):
- **Where they grow:** the beard's area is drawn once on the source head: below a line from the
  sideburns' foot to under the nose, in front of the ears, with the lips left bare.
- **Weights:** a beard takes the weights of the skin right under it (within 1.5 cm), so it opens
  with the jaw. It never takes the neck's weights.
- **Length:** `beard_length` works as the hair's length does.

| Beard | Triangles (slim masculine) |
|---|---|
| Full beard (jaw, chin, a moustache) | 1,001 |
| Goatee | 958 |
| Braided beard (a plaited braid from the chin) | 1,008 |
| Moustache | 250 |

The old block-out `face.beard` is gone.

**Under headwear:** beards hide under masks and headwear as hair does: the forge checks hair and
beards against the `head` and `face` slots.

**One color.** The creator's Face step has a **Hair** panel. It holds:
- the hairstyle and the beard, each with its length;
- one **hair color**, which also colors the brows and the beard.

A box gives the hair a color of its own. The brows and the beard then keep sharing theirs, which
the Brows panel sets. A new hairstyle or beard keeps the old one's color. The color is an ordinary
fill on the part's first paint region, so the Paint step still reaches it.

**Left:**
- a dwarf crest;
- two-tone tips;
- stubble shaded on the skin;
- spring bones on the long locks (BW6).

### BW6 status (2026-11-13): hair and cloth on spring bones

What hangs now swings: long hair, tails and braids, a beard's braid, the tunic's hem, the skirt,
the long coat's skirts and a new travelling cape. A coat, a skirt and long hair can be worn together.

**Chains, at build time** (`mftr_blender.chains`). Each hanging part gets chains of its own,
`extra_<part>_<chain>_<n>`, two or three bones laid along where it settled on that base:

| Part | Chains |
|---|---|
| `hair.long`, `hair.locs` | three: each side and down the back, from where the locks leave the skull |
| `hair.ponytail` | the tail, from the tie |
| `hair.braids` | one a braid |
| `beard.braided` | the braid, from the chin |
| `torso.linen_tunic` | four round the hips, 50% of its mid-thigh hem |
| `waist.wrap_skirt` | four round the hips (front and back on each side), from the hip joints; they carry 60% of the hem, the legs the rest |
| `back.long_coat` | four round the hips, 90% of the hem |
| `back.travel_cape` (new) | three across the back, from the chest, all of it |

A part's vertices blend from their own weights onto the chain over its first bone, then from
bone to bone. A grooming pattern marks its hanging locks (`Locks(chain=…)`); an item says its
chains in `item.ron` (`chains`, `chain_turn`, `chain_arc`, `chain_from`, `swing`).

**The cape** is sewn like the coat (M1): a back panel flaring from the shoulders to the calves,
on a collar sewn over both shoulders and across the chest. (The first version hung from two
small yokes, and the cape's weight pulled it off the body in the simulation.) Its shoulders sit
inside the shoulder line and its neckline low, so the heavy feminine build's shoulders and jaw
don't poke through it. It isn't mirrored: a fold down the middle of the back tore at the mirror
line. The sewing's last weld now merges only near the seams, so a fold that lies on itself stays
two layers instead of tearing (garments already built are unchanged until rebuilt). 1,292
triangles. The block-out `back.cape` stays for the recipes that wear it.

**In the pack and the forge.** A base carries every part's chains (they don't count toward its
bones). The forge drops the chains of parts not worn. Over 32 chain bones (10 §7.2's cap, raised
from 16), the innermost garments go still first, their chains folded onto the bone they hang
from, and hair last. So no outfit is refused for swinging too much.

**In the client** (`client/scripts/springs.gd`). A champion's chains that no clip keys get a
`SpringBoneSimulator3D`. Its colliders are the head, the chest, the hips, the upper arms and the
legs, scaled with the build's hips. Official champions' capes are baked into their clips, so they
are left alone.
- **Champion space:** the chains swing relative to the champion's body node, not the world. In
  world space a champion running at 3.5 m/s streamed everything out flat behind it, whatever
  the stiffness, and a blink would have yanked it.
- **Headwind:** moving adds a wind of 0.1 per m/s against the champion's ground velocity (none
  for a jump in place). The stride, the turns and the clips swing the chains, and running blows
  them back.
- **Feel:** hair is loose, a beard stiffer, cloth heavier. Standing, everything hangs exactly as
  built.
- **Detail:** a champion out of the camera's view or over 45 m from it rests as built; its chains
  start again from rest when it comes back into view.
- **Testing:** `--no-springs` turns it off. `--lineup-move M/S` makes a lineup run ahead with the
  camera, for captures.

**The long coat** was sewn again with the fixed weld: its two small holes at the sides are closed.

**In play:** a practice duel as a caped champion (`--practice FILE --shot-charge`) runs clean.
At the gameplay camera's distance the swing is subtle; it shows most on the cape, the ponytail
and long hair.

**Left:** the owner's look in play.

### Vesper in the wardrobe (2026-11-14): an official champion rebuilt in the new clothes

The owner asked for one of the official ten to be rebuilt in the creator, its clothes made in
the new system. Vesper went first, for her cloak and quiver. `vesper.champion.ron` and
`vesper-neo.champion.ron` now wear three new items, beside the catalog's trousers, boots,
bracers and bow:

| Item | How | Like the hand-made Vesper |
|---|---|---|
| `torso.ranger_tunic` | sewn: forest-green body to the upper thigh, cream sleeves to the elbow, teal trims; four chains on the hem | her tunic over a cream shirt |
| `shoulders.ranger_cloak` | sewn: a leather collar spreading over the left shoulder as a pauldron, holding a green cloak with a teal hem that falls off the left shoulder and diagonally across the back; three chains | her one-sided cloak and pauldron |
| `back.ranger_quiver` | modelled by its `build.py`: a leather quiver from the left hip to over the right shoulder, steel bands, teal-fletched arrows, a strap laid on the body across the chest | her quiver |

Her trousers are painted dark forest, her ponytail auburn. The cloak is on the `shoulders` slot
so the quiver keeps the `back` one.

**What it took:**
- **Panel colors** (`sewing.Panel(color=…)`): a panel can have its own color, a paint region of
  its own. Panels are split apart after the lining pass. Before the lining, the split seam was
  taken for an opening and lined with a rim that sat under the skin.
- **`sleeves: true`:** fits a torso item to the arms. Only an id containing "sleeve" did before,
  so the tunic's sleeves were fitted to an armless body and the shoulders poked through.
- **`hold: "chest"`:** what stands off the body (the quiver, the arrows) rides that bone whole,
  blending from the body's weights from 4 cm off the skin. Without it, the arrows by the right
  shoulder would swing with the arm.
- **Chain labels** close together across an arc (the cloak's) are numbered apart: two came out
  `bl`, and Blender renamed one.

**A bug fixed on the way:** every sewn garment came out on the `skin` material slot. The sewing
wrote the faces' slots before the mesh had materials, and Blender reset them to the first slot.
So the skin tone tinted them, the Paint step couldn't reach them, and their trims never showed
the team color. Every garment is on `cloth` now, and the trims the build scripts asked for are
on `accent`. The others were sewn again; the sleeved shirt kept main's shape (sewn again, it
draped a little differently and failed the harness on the heavy masculine base), its faces moved
onto the right slots by their colors. The linen tunic's, the trousers', the shirt's, the skirt's,
the coat's and the cape's trims now take the team color in play.

**Second pass: all new, the bow aside.** The owner asked for the rest of her in the new
system too. Each rebuilt champion's things also join the creator's catalog, so the catalog grows
as the official ten are rebuilt. She now wears:
- `feet.ranger_boots`: knee-high leather with a flared cuff, an ankle strap and dark soles.
- `arms.ranger_bracers`: leather forearms with two teal bands.
- `hair.swept_ponytail`: side-swept bangs over the ponytail. A new style in the hair catalog.

The boots and bracers are **shells** (`mftr_blender.shell`): a region of the reference body's
skin is lifted off it, smoothed, and thickened. They also *envelop* the body under them: each
vertex moves out until every body point within 2.5 cm is inside it. Without that, the toes
poked through the smoothed toe box. Fitted, no item reaches more than 8 mm below the body's
lowest point (`wardrobe.FLOOR`): pushed off a heavy foot, the soles had sunk 3.8 cm into the
floor.

**Third pass: the owner's look in the creator** (2026-11-15).
- **The trousers showed through the boots** in long strips down the shins. The layering pushed
  the boots out over the trousers, as a coat over a shirt. But a boot is a tight shell, and the
  loose trousers stood up to 5 cm out from it. Boots, bracers and gloves now take what's under
  them in (§4.3, "Tucked in"; `mftr_forge::wardrobe::tuck_in`). Small flecks can still show at
  the front of the ankle, where the hem meets the instep.
- **The cloak flared out from her side, with holes from the front.** It wasn't the springs: it
  flared as much with them off. Its pattern hung wide, round the reference body's raised arm,
  and the fit bound its side to the forearm and the hand there. When the arm came down to the
  base's own angle and the idle pose, it stayed spread, pulled out to a point. Three changes:
  - its pattern hangs behind the back now;
  - a `shoulders` item is fitted and weighted to the shoulder caps only, not the arm below
    them (`wardrobe._arms`): its pauldron still rides the shoulder; its side no longer follows
    the elbow or the hand;
  - it is lined all over (`lining=1.0`). Its inside is seen whenever it swings out, and a lining
    only near the openings left the rest see-through from the front. 1,156 triangles, down from
    1,369.
- **The shoulder bump** is the collar's leather pauldron: it holds the cloak on the left
  shoulder, as the hand-made Vesper's pauldron does. Kept.

**Left:**
- The bow waits for BW7.
- The bases are over the 4 MB budget (a validator warning, not a CI failure). The catalog's
  growth will need them slimmed.
- The owner's look at the third pass.

### BW7 status (2026-11-15): held items, and the ranger's bow first

**Held items.** A weapon isn't fitted to the body: `wardrobe._place_grip` puts it on the hand
(`art/wardrobe/README.md`, "Held items"). It is modelled in a grip frame: the grip at the origin,
+Z along it, +Y its back. Its `item.ron` names the hand bone (`grip`), where along it the grip
sits (`grip_at`), and where the item's +Z and +Y point in that bone's own frame (`grip_up`,
`grip_back`). It rides that bone alone, rigid, and covers nothing. The frame is the official
champion's whose moves the weapon class plays, so their clips aim it as they aim their own.

**The ranger's bow** (`offhand.ranger_bow`, 472 triangles), Vesper's, and now hers and
Vesper-neo's in place of the block-out:
- a tall recurve (1.6 m), its flat, wide limbs curving back toward her and flicking forward at
  the tips;
- a leather grip between silver bands, a teal stone in its face, silver caps and teal-glowing
  tips (her signature);
- the string on her side.

Its frame was read off the hand-made Vesper's bow (`art/champions/vesper`): the limbs and back
at rest in `hand_l`'s frame. So at her full draw it stands upright, its string toward her, as
hers does; the block-out had been upright at rest and leaned at the draw. Her moves' weapon
(`moves::weapon`) is the new bow, so a recipe that takes her attack takes it too.

**The bases' size.** With the bow, the heavy feminine base went over the 4 MiB container cap
(`glb::MAX_GLB_BYTES`) and the creator's catalog stopped loading. Skin weights are now written
as normalized bytes, not floats (`mftr_blender.glb._quantize_weights`): 4 bytes a vertex, not 16.
Each vertex's four still sum to exactly one. The bases went from 4.07–4.20 MB to 3.51–3.60 MB,
under the 4 MB budget again, with room for the rest of the weapons.

**The draw.** A held item can bring bones of its own (`mftr_bones` on its `item.blend` object,
placed with it under the hand, and its vertex groups). The ranger's bow has the hand-made
Vesper's, by name: two on each limb (`extra_bow_1..4`) and one on each half of the string, from
the tip to the nocking point (`extra_string_1..2`). Her shots' baked draws are clips like any
other, so `Retarget` plays their channels on these bones by name, corrected for their rest: the
string comes back to her fingers at her cheek, the limbs bend toward her, and on the release it
snaps straight and shivers. Two consequences:
- The spring bones take only a real chain, each bone under the one before (`spring_chains`):
  the bow's limbs aren't one, so on a stance whose clips don't key them it stays still.
- The forge counts every extra bone the outfit uses against the cap of 32 before it folds the
  garments' chains: the bow's six don't fold, so the garments give way first.

**The other classes** (second pass, 2026-11-15). Eleven held items replace the block-outs of
`parts.py`, under the same ids, so recipes and the moves that equip them are unchanged:

| Item | Class, weight | What | Triangles |
|---|---|---|---|
| `weapon.dagger` | dagger, light | a leaf blade, a dark guard with a glowing stone, a wrapped grip | 128 |
| `weapon.sword` | sword, medium | a broad diamond-section blade, a gold crossguard with swept quillons, a pommel stone | 148 |
| `weapon.hammer` | hammer, medium | a beveled iron head with a gold striking face and a back spike | 198 |
| `weapon.staff` | staff, medium | an iron-shod shaft, three iron prongs holding a glowing crystal | 246 |
| `weapon.spear` | spear, medium | a leaf head with glowing stones, a team-colored collar, a butt spike | 144 |
| `weapon.great_axe` | axe, heavy | a crescent blade ground to its edge, a back spike and a top spike | 188 |
| `weapon.maul` | maul, heavy | a great faceted head with gold-banded faces, a rune stone in each cheek | 352 |
| `offhand.shield` | shield, medium | an iron heater, a team-colored field, a gold boss | 114 |
| `offhand.tower_shield` | shield, heavy | a tall slab with clipped corners, a team stripe, gold studs, a sigil | 120 |
| `offhand.orb` | orb, light | a faceted crystal in a gold cage of three claws | 180 |
| `offhand.lantern` | lantern, light | a chain, an iron cap and base, four posts, a glowing core | 184 |

- **The kit** (`mftr_blender.armory`): lathes, slabs with a ground rim, diamond-section blades,
  beveled boxes and cut stones, in the grip frame. Each item's `build.py` is a dozen lines.
- **Where they sit.** Each takes the frame its block-out was built in, so the clips made round
  the block-outs (Rook's maul, Shade's dagger, the casters' staff) hold them the same: along the
  right hand, upright at the left hand's prop point (orb, lantern), or on the left forearm
  (shields).
- **Classes and weights** are recorded in each `item.ron` (`class`, `weight`). Nothing reads
  them yet.
- Most are under §4.7's 300-triangle floor: they read at the gameplay camera, and the bases
  have little room (3.59 to 3.69 MB of 4).

**Hands that hold** (third pass, 2026-11-16; the owner's look: "sloppy", hands flat beside the
hafts). Three things were wrong, and each has its fix:

1. *The hands had no joints.* The rig's hand is a mitten (one `fingers` bone, one `thumb`), and
   Blender's automatic weights had given that one bone the index and middle fingers only: the
   ring and little fingers and most of the thumb were welded to the palm. A creator base's
   hands are now read off its mesh and rigged (`mftr_blender.hands`):
   - the five tips are the far points of the skin from the wrist, each digit the skin nearer
     its tip than any other's, out to the nearest web;
   - `fingers_<s>`, `fingers_02_<s>`, `fingers_03_<s>` are the four fingers' three rows of
     joints (they close together: one bone a row), `thumb_<s>` the thumb from its root at the
     wrist and `thumb_02_<s>` its last joint. Six new optional `biped` bones
     (`rules::BIPED_OPTIONAL`); a forged champion has at most 120, the cap;
   - each of those bones lies across its row: local X is the hinge, Z the back of the hand, so
     a negative turn about X closes it, on both sides;
   - the four fingers are brought side by side at rest (a fifth of their spread stays) and slid
     so each row of joints is straight: fingers that share a hinge only close into a fist if
     they lie alongside each other;
   - every digit is weighted by where it lies along its joints, the ball of the thumb with the
     thumb, two bones a vertex. What is worn on the hands is weighted the same way
     (`hands.glove`), and the gauntlets, which are the hand's own surface, take all of it and
     hide the hand under them.
2. *The weapons lay along the hand.* The official champions' weapons run out of the wrist like
   a sixth finger, and their clips aim them by turning the hand (14 §3.3's pass then keys the
   prop bone to hold the weapon where it was aimed). A held item with `grasp_radius` rides the
   **prop bone** now, not the hand, and the forge re-poses every clip (`mftr_forge::grip`):
   - **the weapon keeps its line**: the point the fist holds and the haft's direction, each
     frame, are the clip's;
   - **the fist closes on it**: the haft lies against the palm in the crook of the fingers,
     each phalanx tangent to it (the turn at a joint is twice the angle its tangent length
     subtends, so a thinner haft is wrapped tighter), the thumb round the other way, found by a
     small search; the knuckles lead an edge;
   - **the arm comes to the hand**: two freedoms are left, where round the haft the palm sits
     and how far the haft tilts across the palm (a hammer's grip to a sabre's, 0 to 38°). Each
     frame takes the pair that bends the wrist least (a bend past 26° and a turn past 75° cost;
     so does stretching the arm), the elbow re-solved to reach it, bent the way the clip bent
     it; the choice is the cheapest path through the whole clip, so the hand never jumps, and
     a loop is walked twice so its ends meet;
   - the arm's bones are turned by its joints, not by the clip's own twists: each lies along
     its bone with the elbow's hinge across the plane the arm folds in, and the forearm takes
     half of the wrist's turn;
   - **a second hand** closes on a haft too, wherever the clip brings its palm within 7.5 cm of
     it along the item's `grasp_span` (Rook's left on the maul), easing in from 15 cm;
   - **free hands relax**: every joint a little bent; a clip's mitten curl becomes a curl of
     every joint, to a closed fist.
3. *The off-hand items had nothing to hold.* The orb has a stem and the lantern a leather grip,
   both gripped in the fist and carried upright whatever the hand does (`grasp_upright`). A
   shield stays strapped to the forearm (`grasp_strap`): the fit gives it a grip bar on two
   posts where the closed fist is, moves it off the back of the hand if it lay through it, and
   the forge closes the fist and holds the wrist straight.

What the look found on the way:
- The staff's diagonal carry was the rig step's doing, not the clips': placing the knuckle bone
  dragged the hand bone's tail (a connected child), which turned every weapon's frame 41°. The
  finger, thumb and prop bones are cut loose from the hand before they move.
- The official staves are held a third of the way down, the head past the fingers: the new
  staff's origin was already there.
- Vesper's bow hangs on her hand itself, not the prop bone (its bones are the hand's children):
  `grasp_on_hand` takes its line from the clip's hand.
- The maul's and the great axe's hafts were 7.6 cm through, more than a hand closes on: 5.6 cm.

Checked in captures (idle and mid-attack, four sides of each hand): Rook's maul in both hands,
bare and gauntleted; Shade's dagger; Lumen's staff; Vesper's bow hand at rest and at the full
draw. The sword, hammer, spear and great axe were checked at body distance, the two shields,
the orb and the lantern at rest only. `grip`'s tests hold the fists closed through the clips,
the second hand on the maul, and the wrist's turn under 2.2 rad through Shade's and Rook's
attacks (it reached 3.0 before the arm was turned by its joints).

**Arms, the bow and the shields** (fourth pass, 2026-11-16; the owner's look at the third:
Rook's and Lumen's arms went through their bodies, the bowstring didn't come back to the draw
hand, the shields floated off their grips, and Vesper's string hand):

1. *Arms out of the trunk.* The clips were made on the hand-made champions' bodies; a creator
   body is broader, and Lumen's robe wider still, so an arm the clip lays across the chest goes
   into it (Lumen's free hand 14 cm deep in her robe at rest, Rook's forearm in his chest
   through his swings). `grip` now reads the trunk off the skin: the body's and what is worn
   on it (not what swings on springs: a cloak, a skirt's flaps), weighted to the pelvis and the
   spine, cut in four slices round each of those bones, each slice an ellipse; the arms'
   thickness from their own skin.
   - The grip solver keeps a held arm out of it: for each pose it tries, the elbow may swing
     out round the line from shoulder to wrist (up to 54°), and what still goes in costs.
   - Then every arm, held or free, gets a last pass: where it still goes in, its elbow swings
     (up to 97°) and its wrist is pushed clear (up to 20 cm), the hand turned as it was. Two
     hands on one haft go together, the haft with them (Lumen's idle holds her staff in both
     hands in front of her chest now, where the clip had the left hand inside her).
   - The change is held a few frames either side of where it is needed and faded, as the
     elbow's and the wrist's places (smoothing the swing angles mixed poses that didn't fit),
     and the pass is run twice.
   An arm touching the trunk is allowed (1.2 cm); a hand meant to rest on it (on the chest, at
   the chin at the full draw) still does.
2. *The bow keeps its roll.* Vesper's bow hangs on her hand's prop bone, so the solver's free
   choice of where round the grip the palm sits turned the whole bow, the string's draw point
   with it. A bow (`grasp_on_hand`) now keeps the clip's roll: its back to the mark, the palm a
   step either way of it at most.
3. *The string comes to the fingers.* Wherever the clip draws the string (its nock more than
   1 cm off its rest, wholly from 5 cm), the string's two halves are turned and stretched so
   the nock is in the draw hand's fingers: in the crease of their last joints, on the palm side.
4. *The string hand hooks.* As far as the string is drawn, that hand's fingers take a
   bowstring hook (the knuckles nearly straight, the last two joints curled round the string)
   and the thumb tucks in.
5. *The shields are on their grips.* The shield had been put where the official clips carry
   it and its grip bar where the fist was, the posts out along the back of the hand; the face
   was tilted from that, so the posts stopped short and the fist hung at its lower edge. The
   fit now finds the shield's plane (the way its body is thinnest), slides it in that plane
   till the fist is behind its middle (the boss: a centre grip), on the back of the hand's
   side, its back 5.5 cm from the fist's middle (a gauntlet's knuckles clear it), and runs the
   posts from the bar's ends to the back they hit. The forge holds the strapped hand on the
   forearm after the trunk pass too.

`grip`'s tests: the arms keep out of the trunk through Lumen's and Rook's idles and attacks
(under 8 cm at the deepest, from 14), and the string is in the fingers wherever the attack has
it drawn.

**Every clip, every champion** (fifth pass, 2026-11-17; the owner: make sure the spells and
emotes are good). Measured through every clip of all ten champions, the trunk pass had left
the library's `whirlwind` through the shoulders on every build (31 cm), Rook's second fidget in
his chest (21 cm), and some thirty spells, emotes and attacks 8–20 cm in. Four causes:

1. *Arms keyed sparser than the body.* The pass keyed an arm at the arm's own keys; the
   whirlwind's arm is keyed every other frame, its spine every frame, so between two arm keys
   the body had turned 60° on and the arm, eased between its two poses, went through it. The
   pass keys every frame at least.
2. *Moves carried round a turning body.* The fix found at one frame was held and faded into
   its neighbours in the world's frame; a body turning fast took the arm round while the move
   stayed put. The moves are the chest's now (in its frame), and a frame whose smoothed move
   would put the arm in where its own move doesn't takes its own.
3. *A second hand off the haft.* A second hand moved only as far as it followed the haft's
   push; let go of the haft (Rook's fidget), it couldn't move at all. It pushes itself clear for
   what it doesn't follow.
4. *Only level pushes.* The wrist was pushed only round the level: an arm over the shoulders
   needs up, one across the belly down. The pushes go up and down too (45° and straight), as
   far as 35 cm.

The neck and the head are the trunk's now too, for the arm (the hand and the wrist may touch
them: a hand at the chin, at the full draw), and what a hand holds keeps out of the head (Rook's
maul went through it in his overhead windup), measured on the held item's own skin: a weapon
may rest on a shoulder, by the neck.

`grip`'s tests: through every clip of all ten champions, an arm is under 6 cm into the trunk
(4.4 at the deepest) and a held item under 7 cm into the head (6.3, a maul on the shoulder by the
neck).

**Left:**
- Weight classes picking the clips.
- A held weapon may pass through hair (not part of the head the pass keeps out of: Rook's maul
  through the back of his hair in his overhead windup).
- The hand-made champions keep their mitten hands: this is the creator bases' (and so every
  rebuilt champion's).
- The old block-out `offhand.bow` stays in the catalog for recipes that name it: the forge
  refuses an outfit id it doesn't know.
- The owner's look at the arms, the bow and the shields, and at the spells and emotes.

### Rook in the wardrobe (2026-11-17): the second official champion rebuilt

Rook-neo (and the `rook` recipe) now wears new-system items only: four new ones, beside the
catalog's gauntlets, wool trousers, braided beard and maul. Shaved bald, as the hand-made Rook.

| Item | How | Like the hand-made Rook |
|---|---|---|
| `torso.veteran_gambeson` | sewn: padded brown, a high round neck, long sleeves to the wrist, bound in dark leather | his gambeson, at the arms and the belly |
| `shoulders.veteran_plate` | plates: an iron breastplate and a dark backplate, open at the sides, a brass-rimmed gorget, crimson-lacquered pauldrons domed over the shoulders with brass rims, two lames each; rigid | his barrel breastplate, gorget and slab pauldrons |
| `waist.veteran_tabard` | sewn: crimson panels front and back to the knee from a band at the waist, open at the sides, a darker hem, lined; a chain down each panel | his long tabard |
| `feet.veteran_boots` | plates: heavy leather from heel to toe and up to under the knee, two iron bands, an iron toe cap, thick soles, domed knee cops | his heavy boots and knee cops |

Trousers dark, beard grey-brown. The plate is on the `shoulders` slot so the gambeson keeps the
torso one; the tabard is on the `armor` layer so the forge pushes it clear of the gambeson's
hem (items on one layer aren't pushed apart).

**Plates, a new way to make an item** (`mftr_blender.plates`). The first plate and boots were
shells of the reference body's skin, and came out ragged: the CC0 body is low-poly (3,201
vertices), so a region cut from it has its facets' sawtooth edges, and decimating it to its
budget before painting it ran its bands across the triangles diagonally. A plate is instead
built on its own grid:
- *lathed* round an upright axis through the body (the torso's middle, a leg's): ring by ring,
  rays out from the axis find the skin; each point stands its lift off it, the distances smoothed
  across the grid (a plate, not the muscles under it) and never inside the skin;
- a *tube* round a path through the body that needn't be upright: the boot's foot, heel to toe,
  capped at both ends; a ray that meets no skin within its reach stops there (up the heel,
  inside the leg);
- *domes* (an ellipsoid's cap, cut where it should end: a pauldron short on the neck side, so it
  stands clear of the trapezius) and *strips* of them (the lames).
Edges and bands are the grid's rows and columns: straight. `shell.Shell` also takes `cuts` now
(the body cut finer first).

**Sleeves to the wrist.** The sleeved shirt's sleeve line ran from the shoulder toward a wrist
at (0.469, 0.956), outside the reference arm: fine for a short sleeve, but a long one was sewn
round empty air and fell away. Measured on the CC0 body, the forearm's middle is at x 0.25 at
1.2 m and 0.345 at the wrist; a long sleeve also needs the arm's girth (about 30 cm round), not
a shirt cuff's.

**The bases' size.** With Rook's items the bases reached 4.03–4.09 MB, a hair under the 4 MiB
container cap. Vertex colors are now written as normalized bytes, not shorts
(`mftr_blender.glb._quantize_colors`, as the skin weights were): a paint region is a color of
its own, told apart at a thousandth, and a 1/255 step keeps them apart. The bases are
3.69–3.76 MB with Rook's items (after the triangle cut below).

**The triangle cap.** First built, Rook-neo came to 16,128 triangles, over the creator's 14,000
(20 §7.2); the four items were brought down to about 1,600–1,800 each, and he is
13,589 now: under the cap, over the 5,000–10,000 target.

**Left:**
- The sewn trims (a hem's darker band) are jagged: they are the faces near the open edge of a
  decimated sim mesh. A trim cut along its line would be cleaner, for every sewn item.
- The pauldrons ride the shoulders' weights; the hand-made Rook's ride the clavicle whole.
- The gambeson's sleeves are roomy and faceted on the heavy build; a slimmer sleeve (7.2 cm
  across) fell off the arm in the sim.
- Seen in creator captures at rest only, not in a match or in motion.
- The owner's look.

### Lumen in the wardrobe (2026-11-17): the third official champion rebuilt

Lumen-neo (and the `lumen` recipe) now wears new-system items only: eight new ones, beside the
catalog's ponytail, painted gold. Like the hand-made Lumen: a halo, a sky collar laced in gold
over a white bodice, bell sleeves, a long white skirt hemmed in gold with three tails streaming
from a sky sash, ivory boots strapped in gold (her sandals, below), a lantern staff.

| Item | How | Like the hand-made Lumen |
|---|---|---|
| `head.priestess_circlet` | modelled round the slim feminine base's head: a gold band at the brow, higher at the back, a sun stone in a gold setting; a double halo of sun gold floating behind the head | her circlet and halo |
| `torso.priestess_bodice` | sewn: white linen on narrow straps to just below the waist, edged in ivory; the `under` layer, so the collar and the skirt clear it | her fitted bodice |
| `shoulders.priestess_collar` | plates: a sky collar round the neck and over the shoulders, a gold rim, a sun stone; gold laces down the front in a V to a second stone at the waist | her sky collar, gold V and sun stones |
| `arms.priestess_sleeves` | plates: gold armlets on the biceps; ivory sleeves belling open from the elbow to the wrist, a gold band at the top and a sky hem | her armlets and bell sleeves |
| `waist.priestess_skirt` | sewn: white linen from the waist to mid-shin, flaring, a gold hem; four chains | her underskirt |
| `back.priestess_tails` | sewn: a sky sash round the waist and three white tails finely edged in sky, down the back and behind each hip to above its hem, laid over the skirt; a chain down each | her dress's three tails |
| `feet.priestess_boots` | plates: soft ivory leather to above the ankle, thin pale soles, two gold straps across the foot, a gold cuff and anklet | her gold anklets; not her sandals (below) |
| `weapon.lantern_staff` | modelled: a pale shaft shod and banded in gold, a sun ring with rays round a caged lantern burning sun-white; held as the staff, so her clips aim it | her lantern staff |

**What it took:**
- **A head item.** The reference body has no head of its own to fit to (each base's head is
  authored), so the circlet is worn on the `head` bone as a held item is on its hand, rigid, and
  scaled to each base's head by that bone's length (`grip_scale` in its `item.ron`: the heavy
  masculine head is a fifth bigger than the slim feminine one it was fitted on). The band stands
  16 mm off the skin to clear hair drawn back over the skull; the halo stands far enough behind
  the head that the forge's hair hiding (22 §7 item 5) leaves the hair under it.
- **Clean trims, for every sewn item.** A trim was the faces near an open edge, so on a decimated
  sim mesh it ran as a sawtooth (Rook's tabard, every hem). Now the faces are cut along the trim's
  own line, a set width from the edges (`sewing._cut_trims`), and the trim runs smooth beside
  the hem. The cut also showed an old waste: the trim is split off for its color, and that split
  was taken for an opening, lined and rimmed. Only the garment's real openings are now
  (`_line(openings=…)`); the bodice came down from 1,820 to 1,214 triangles at one budget. The catalog's other
  sewn items keep their saved meshes until they are sewn again.
- **One garment over another.** The tails must hang over the skirt's flare, further out than the
  forge's layering pushes (2 cm, `wardrobe::MAX_PUSH`). With the skirt as a collider in the sim,
  the tails caught on it and bunched at the waist (and the skirt, sewn over the bodice that way,
  came out at the knee). So a garment `over` another is simulated on the body alone and then laid
  over it: each point out along its level line from the body's middle to 8 mm past the other's
  outside.
- **Plates:** *ribbons* (straps laid on the body seen from the front: the collar's laces); a
  face whose color is `None` is left out; `armory.Kit.ring` (a torus: the halo, the staff's sun
  ring).
- **Sandals didn't pass.** Built first as a sole and straps of a tube round the foot, they failed
  the harness (22 §4.8) on the slim masculine base: 95 skin points through them, the limit 12,
  where the sole's edges met the toes and the heel and the instep strap crossed the steep top of
  the foot. Raising them off the skin, dropping the sole's ends and the heel cup, moving the
  straps: still 40–50. Closed, like the veteran's boots, they pass, so she wears soft ivory boots
  with gold straps and anklets. Open footwear needs the fitting to keep a strap's shape over a
  foot it wasn't made on.
- **The harness on the sleeves.** Armlets level with the armpit let the chest beside the arm
  through on the slim masculine base; on the biceps below it, they pass.
- **Helpers after the chains.** A long skirt's chains take up to four bones a vertex, so a body
  helper dropped out at one vertex below the hip and stayed on its neighbour (`body_hip_r`
  jumping 0.36 across an edge, over 21 §1.4's 0.35): a hanging item's helper weights are smoothed
  again after its chains.

**The bases' size.** With Lumen's items the bases reached 4.00–4.06 MB, about 130 KB under the
old 4 MiB container cap. The owner chose detail over slimming for now: the glb container cap
(`mftr_pack::glb::MAX_GLB_BYTES`) and the base budget (`rules::BASE_BYTES_BUDGET`) are both
7 MiB (2026-11-17), room for about forty more champions' worth of items at Lumen's ~65 KB a base.
Bases ship with the game and never cross the network, and one base's meshes are shared by every
champion built on it, so the cost is install size and a little load time. Slimming (normals as
bytes, or the catalog split out of the bases) stays open for later.

**The look, second pass.** The owner's first look in the creator found three faults:
- *The tails' sky edging read heavy:* a 22 mm trim on 75 mm tails left them more blue than white.
  It is 10 mm now.
- *The middle tail looked short:* the tails ran past the skirt's hem, and there the middle one
  folded up between the calves. All three now end above the hem, the middle one longest, so the
  gold hem shows below them.
- *The bodice showed through the collar at the back:* its neckline rose to 1.44 m, over the
  collar's top (1.425 m). It now ends level with that top at the sides and at 1.41 m at the
  back, inside the collar's 16 mm lift. Lower (1.405 m) the slim masculine base's skin came
  through it (32 pokes), and at 1.418 m the heavy feminine base's shoulders did (13).

**Left:**
- Seen in creator captures at rest only (13,900 triangles), not in a match or in motion; the
  tails' and skirt's chains not yet seen swinging.
- The circlet is fitted to one head and scaled: a face sculpted far from it may meet the band.
- Sandals: the fitting to keep open footwear's straps in shape (above).
- The catalog's other sewn items to be sewn again for clean trims (Vesper's, Rook's, the tunic,
  the skirt…): their looks change a little, for the owner to see.
- The owner's look.

### Shade in the wardrobe (2026-11-17): the fourth official champion rebuilt

Shade-neo (and the `shade` recipe) now wears new-system items only: eleven new ones, after his
concept (`art/champions/shade/concept.md`) and the hand-made Shade. A tall indigo hood sweeping back
into a point over a black mask with burning violet eyes, a violet scarf with two tails down the
back, an indigo jerkin, a spiked steel guard on the right shoulder only, wrapped forearms, a
team-colored belt with kunai strapped to the thighs, black leggings into wrapped boots, and a
curved blade in each hand held forward. 11,483 triangles in the creator, no paint.

| Item | How | Like the hand-made Shade |
|---|---|---|
| `head.shadow_hood` | plates round the slim masculine head on the head bone, as the circlet is: a roomy hood closed under the chin, a violet rim at the face, rings narrowing up and back to the point | his tall pointed hood |
| `face.shadow_mask` | plates round the slim masculine face on the head bone: a smooth black mask, brow to chin, two violet slits at the eyes, their outer corners raised | his mask and violet eyes |
| `torso.shadow_jerkin` | sewn: the veteran's gambeson's pattern cut lean and short, thin indigo cloth, bound in black | his indigo torso |
| `shoulders.shadow_guard` | plates: a dark steel dome on the right shoulder over one lame, three spikes up and back; a strap across the chest and back | his right shoulder's spiked guard |
| `arms.shadow_wraps` | plates: wraps round each forearm between black bands, a dark steel plate on the outside | his wrapped forearms and their guards |
| `waist.shadow_belt` | plates: black leather over a band of the team's color; a strap round each thigh with three kunai, points down | the accent band, the thigh kunai |
| `back.shadow_scarf` | plates: a roll of violet cloth round the base of the neck, two tails laid down the back; a chain down each | his streaming scarf |
| `legs.shadow_leggings` | sewn: the wool trousers' pattern narrow down the leg, black, to the middle of the calf | his black legs |
| `feet.shadow_boots` | plates: the priestess's boots' build in black, wraps across the foot, shafts wound to the middle of the calf | his wrapped calves |
| `weapon.shadow_blade`, `offhand.shadow_blade` | modelled in the dagger's grip frame: a long blade sweeping out past the knuckles over a guard, its outer edge violet; the left mirrors the right | his two forward hand blades |

**What it took:**
- **Two held items.** Shade's moves now equip both blades: `moves::weapon` names a champion's
  weapon parts (none, one or two), and the creator puts all of them on when his attack is picked.
- **Head items from plates.** The hood and the mask are cast from the skull's middle in its own
  frame, row by row of rays by elevation, and worn on the head bone with `grip_scale`; the hair
  hides under the hood (§7 item 5).
- **Closed pieces in plates.** A spike or a kunai is already a closed shape: `plates.finish` takes
  `thickness=None` to turn its faces out on their own instead of thickening it (that doubled the
  kunai).
- **A scarf that isn't sewn.** Sewn, the neck loop settled where the sim left it, and on the slim
  and heavy bases the neck came through it (29 to 83 skin points, the limit 12) however it was
  cut. Lathed from rays between the shoulders and the chin, standing off the neck by a set
  distance and dipping in front under the jaw, it passes on all four; its tails are ribbons.
- **The harness, item by item.** The guard's strap stands 34 mm off the chest (nearer, the slim
  masculine armpit came through its end); the leggings have a deeper crotch than the trousers'
  pattern (the heavy masculine base came through it); the jerkin's armholes are roomier than the
  gambeson's and its neck is scooped in front (the heavy feminine throat). Sewing is a sim: each
  re-cut drapes a little differently, so each change was checked on every base again.

**The bases' size.** With Shade's items, 4.32–4.38 MB, under the 7 MiB cap.

**Left:**
- Seen in creator captures at rest only, not in a match or in motion: the blades' frame (the
  dagger's) not yet seen through his slashes, the scarf's tails not yet seen swinging.
- The spare blades crossed on his back (the back slot is the scarf's).
- The jerkin shows its sim's facets; the scarf's tails are flat ribbons, not draped.
- The owner's look.

### Cairn in the wardrobe (2026-11-17): the fifth official champion rebuilt

Cairn-neo (and the `cairn` recipe) now wears new-system items: eight new ones after his concept
(`art/champions/cairn/concept.md`) and the catalog's full beard in grey-brown, bald under the
antlers, bare-legged under the kilt. Great branching antlers of weathered bone with
emerald beads at their roots, a mantle of granite slabs and pauldrons grown over with moss, a bark
cuirass with an emerald at the breast, bark bracers, a bark kilt on a team-colored belt, a moss
cloak torn at the hem, bark greaves over leather boots, and a gnarled staff whose four roots hold
a glowing emerald. The body keeps his old paint. 13,358 triangles in the creator (the cap
14,000).

| Item | How | Like the hand-made Cairn |
|---|---|---|
| `head.cairn_antlers` | modelled with the armory's new bending tubes (`Kit.branch`) on the heavy masculine head, worn on the head bone with `grip_scale`: a beam out, up and back on each side, three tines, a knobbed burr at the root set with beads | his antlers and their emerald beads |
| `shoulders.stone_mantle` | plates: slabs lathed over the chest and the back to the neck, each column its own grey and a little more or less proud; a few-sided granite dome over each shoulder above a slab; any face looking up is moss | his mossy stone mantle and pauldrons |
| `torso.bark_cuirass` | plates: a lathe of bark strips close round the chest and the belly, open under the arms top to bottom; a granite diamond with an emerald at the breastbone | his bark chest plates and chest gem |
| `arms.bark_bracers` | plates: Shade's wraps' tubes, leather under bark strips open on the inside of the arm | his bark bracers |
| `waist.bark_kilt` | plates: a belt with the team's band; slats hung from a ring round the hips, each its own strip, length and shade, flaring as they fall to clear the thighs, moss at the tops | his kilt of bark slats and moss, his accent band |
| `back.moss_cloak` | plates: each column cast from behind onto the body in his cuirass and kilt, hanging straight from the furthest point above it, a torn hem; the travelling cape's chains | his moss-green cloak |
| `feet.bark_greaves` | plates: Shade's boots' build in brown to under the knee, a bark greave lathed over the front of each shin | his bark greaves |
| `weapon.warden_staff` | modelled in the staff's grip frame: a wandering shaft (straighter where the hands go), knots, four roots twisting up round the emerald | his gnarled staff and its stone |

**What it took:**
- **A cloak that isn't sewn.** Sewn from the travelling cape's pattern, a cloak held only at the
  neck bunched at the hips where the sim left it (the cape does too). Cast onto the body worn with
  the cuirass and the kilt instead, it falls clear over both. It is cut for them: over other
  clothes it stands a little off the back.
- **Bending tubes.** `Kit.branch` sweeps a ring along a path, its frame carried along so it
  doesn't twist: the antlers' beams and tines, the staff's shaft and roots.
- **The weapon's mapping.** `moves::weapon` names Cairn's staff, and now Lumen's lantern staff
  too: picking her attack in the creator had put the plain staff in her hand.
- **The triangle cap.** First built, he forged to 18,184 triangles, over the 14,000 cap
  (20 §7.2): his items alone came to about 10,900. The kilt has fewer, plainer slats, the cuirass,
  bracers, greaves, cloak and mantle fewer rows and sides, and he wears no hair (the concept
  names a weathered, bearded head and no hairstyle; the antlers rise from the bare brow) and no
  trousers under the kilt.
- **The arms test** (`grip`): what's worn is part of the trunk the arms must keep out of. The
  cuirass wrapped under the arms, and his whirling blow took the right arm 10 cm into it; it is
  now a front and a back, close to the chest (stood further off, his Q's left arm went 7 cm in).
  Its rows are fine enough to follow the heavy feminine bust rather than cut across it (the
  harness: 18 skin points, the limit 12, with fewer).
- **The cloak over the cuirass.** On the heavy base the cuirass's top edge at the back came
  through the cloak's top; the cuirass is lower at the back (1.27 m, 1.345 m in front) and the
  cloak stands 30 mm off what's under it, 75 mm over the shoulder blades.

**The bases' size.** With Cairn's items, about 4.7 MB, under the 7 MiB cap.

**Left:**
- Seen in creator captures at rest only: the staff not yet seen through his blows, the cloak not
  swinging.
- The war paint on his face (the catalog has no face paint yet).
- The owner's look.

### Ember in the wardrobe (2026-11-18): the sixth official champion rebuilt

Ember-neo (and the `ember` recipe) now wears seven new items after his concept
(`art/champions/ember/concept.md`), over the catalog's buzz cut in soot black and Shade's black
leggings. A crest of flame sweeping up and back, a tall collar of soot-black petals lined in ember
red flaring behind his head, a charcoal coat with ember-red lapels, gold buttons and tails, bell
sleeves with glowing cuffs, an ember-red sash on the team's band, and pointed boots with gold
knee plates. His hands stay bare: the fire in his palms is his effects'. 10,994 triangles in the
creator (the cap 14,000).

| Item | How | Like the hand-made Ember |
|---|---|---|
| `head.flame_crest` | modelled with the armory's bending tubes on the slim masculine head, worn on the head bone with `grip_scale`: eleven tongues from the hairline to behind the crown, each in three lengths, white-hot, flame and ember, every step dimmer so the signature's glow keeps the roots brightest | his burning hair |
| `shoulders.ember_collar` | plates: a band lathed round the neck's base with a gold brooch; petals rising from its top ring round the back, standing further out as they climb, a red lining sheet just inside the black, the top cut into points; lapels as ribbons, buttons as small domes | his tall flared collar, lapels, buttons and brooch |
| `torso.ember_coat` | sewn: the shadow jerkin's pattern a little longer, roomier sleeves, charcoal edged in ember red | his charcoal coat and its red lining |
| `arms.ember_sleeves` | plates: the priestess's bell sleeves without the armlets, wider, a red band and a glowing cuff at the hem | his bell sleeves and glowing cuffs |
| `waist.ember_sash` | plates: a sash lathed round the waist over the coat on the team's band, a gold buckle with a flame stone | his accent band |
| `back.ember_tails` | plates: the moss cloak's build (each column cast from behind onto the body in his coat, hanging from the furthest point above it), cut into two tails narrowing to the knees, with a chain each | his coat tails on their chain |
| `feet.ember_boots` | plates: Shade's boots' build, plain and fuller toward the top, closed points past the toes, shallow gold caps over the knees | his pointed boots and gold knee plates |

**What it took:**
- **Glows take the signature's hue** (`paint::glow`), keeping only their brightness: the crest's
  three colors become one orange, so each step is also dimmer than the last.
- **The harness** (skin through an item at rest, 12 points at most): the collar's band rose into
  a slim base's jaw at the throat (it now dips 35 mm there), and its petals ran too close to the
  back of the skull (they stand 25 mm off the band and flare 20 cm out). The coat's sleeves, the
  jerkin's, hugged a heavy base's deltoids (16 points); they are cut roomier.
- **Tails, not a sewn coat.** The long coat is sewn whole (3,292 triangles); a fitted coat and two
  cast tails are about half that, and the tails swing on their own chains.

**Left:**
- Seen in creator captures at rest only: the tails not swinging, the collar not seen against his
  head turning in his clips.
- The crest reads pale in the creator's light: its glow may want to be less bright.
- The owner's look.

### Marrow in the wardrobe (2026-11-18): the seventh official champion rebuilt

Marrow-neo (and the `marrow` recipe) now wears six new items after her concept
(`art/champions/marrow/concept.md`), over the catalog's long hair in grey (hidden under the hood
but at the face) and Shade's black boots. A deep plum hood set back from her face with a heavy
rolled rim and a drooping peak, a ragged grave-grey shawl over her shoulders, a plum robe, a
corset with a ribcage of bone and a skull lantern at her hip, a skirt torn into strips, and a
crooked staff crowned with a horned skull. 12,143 triangles in the creator (the cap 14,000; over
the 10,000 target, as Ember is).

| Item | How | Like the hand-made Marrow |
|---|---|---|
| `head.marrow_hood` | plates: the shadow hood's build, roomier, its opening wider and higher, the peak's rings drooping back behind the nape; the rim a closed tube laid round the opening's edge | her hood, its rolled rim and drooping peak |
| `shoulders.grave_shawl` | plates lathed round the neck and shoulders, each ray cast in from outside so below the shoulders it lies over the arms, not between them and the chest; a torn hem, a point down the back, a bone toggle | her ragged cowl and grave-grey shawl |
| `torso.marrow_robe` | sewn: the shadow jerkin's pattern with the ember coat's roomy sleeves, plum | her plum robe |
| `waist.bone_corset` | plates: a corset lathed from the hips to under the chest on the team's band; a breastbone and four pairs of ribs as narrow lathed strips curving out and down, the lower ones opening an arch; the skull lantern a few closed domes on a chain | her ribcage corset, belt band and skull lantern |
| `legs.rag_skirt` | plates: the bark kilt's slats, long and overlapping, each its own length and torn on a slant, hung from a ring stood clear of the seat as well as the hips; chains round the hips | her ragged skirt of tattered strips |
| `weapon.bone_staff` | modelled with the armory: a black shaft crooked over the hands, a lathed skull with magenta fire in its sockets, ram's horns as bending tubes, bone charms on cords | her staff, skull, horns and charms |

**What it took:**
- **A skirt in the legs slot.** The torso slot holds the robe and the waist the corset, so the long
  skirt is a `legs` item on the `over` layer: the corset (`armor`) is pushed clear of its top.
- **Clear of the seat.** Hung from a ring at the hips, the strips flaring as they fell still cut
  through the buttocks below it; the ring now stands out as far as the body does anywhere in the
  25 cm below it, as the flare would carry it.
- **Over the arms.** A shawl lathed out from the body's middle hugs the chest under the arms; cast
  in from outside it drapes over them.
- **The staff's foot.** Crooked all along and longer than the plain staff, its foot swung up to
  85 mm into her head in her clips (`what_a_hand_holds_keeps_out_of_the_head` allows 70): below
  the hands it is now as long as the plain staff's and near straight; the crook is over the hands.
- **The robe** is the ember coat's cut exactly: lowered 4 cm with a wider trim, it let 19 points
  of a slim masculine base's skin through (the harness allows 12).

**Left:**
- Seen in creator captures at rest only: the skirt's strips, the shawl and the lantern not seen
  in her clips (her hunch, her kneeling recall, her staff slam).
- She stands upright in the creator: her hunch is her clips'.
- The owner's look.

### Quill in the wardrobe (2026-11-18): the eighth official champion, and dark skin

Quill-neo (and the `quill` recipe) is now a Black woman, and her navy and indigo are purple and
violet; the rest of her concept stands (`art/champions/quill/concept.md` still describes the
hand-made Quill). She wears six new items over the catalog's coils, Shade's leggings and Vesper's
boots: a wide astronomer's hat bending back to a star, brass goggles on its band, a deep purple
coat with two violet capes, violet lapels and brass buttons, long star-flecked tails, a belt
with a scroll case and a spyglass, and an orrery staff. Her signature is violet (255). 12,700
triangles in the creator (the cap 14,000; over the 10,000 target).

| Item | How |
|---|---|
| `head.astronomer_hat` | the armory's pieces on the slim masculine head, worn on the head bone with `grip_scale`: a lathed brim and band, the crown a bending tube, the goggles rings and lenses, the star two stones; the whole hat leans back 17° |
| `shoulders.astral_capes` | plates: the grave shawl's build (each ray cast in from outside), two capes; the lapels and buttons the ember collar's |
| `torso.astral_coat` | sewn: the ember coat's cut, purple |
| `waist.astral_belt` | plates: the ember sash's belt; the scroll case and the spyglass closed tubes |
| `back.astral_tails` | plates: the ember tails' build, longer and widening to the shins, stars as small glowing diamonds just off the cloth |
| `weapon.orrery_staff` | the armory: the plain staff's shaft, three crossing rings, two planets, a star |

**Dark skin rendered as tan (fixed).** The skin tones (`looks::SKIN_TONES`) are colors as a
screen shows them: the creator's swatches draw them so. But the forge scaled the base's linear
vertex colors by the tones' plain ratios, so the dark tones rendered far lighter than their
swatches: the darkest natural tone, a deep brown, as a light tan, and tones 5 to 7 nearly alike.
No tone could show a deeply dark-skinned person. Now `skin_ratio` replaces the base's paint with
the tone's linear color (times the base's baked shading): each tone renders as its swatch, and
the eight natural tones run from fair to very dark. A test pins it. Every forged champion's skin
moves to its swatch: tones 0 to 2 a little darker than before, 3 to 7 clearly darker.

**Her face, from the sliders (§3.3).** The aim is a realistic face of West African descent, not
a type: features vary widely among Black people, and a caricature is what to avoid. So no slider
goes past half its range, and the changes are the ones that most often set such a face apart
from the default:
- **Nose:** broader (`nose_width` 0.6), a lower bridge (`nose_reach` −0.3, `nose_hook` −0.2), a
  slightly upturned tip showing the nostrils (`nose_tip` 0.2).
- **Mouth:** fuller lips, the lower fuller than the upper (`lips_upper` 0.45, `lips_lower` 0.5,
  `mouth_lips` 0.35), a marked cupid's bow (`lips_bow` 0.35), a little wider.
- **Brow and eyes:** a rounder, more upright forehead (`head_forehead` 0.35), a smoother brow
  ridge, eyes a little larger with a defined crease.
- **Cheeks:** high cheekbones, full cheeks.

Reviewed in front, three-quarter and profile captures against her old face at the same tone.

**Found on the way:**
- **The brows' paint region is 2, not 3**, on every humanoid base now: a recipe painting
  `("body", 3, …)` paints nothing, silently.
- **The default brow color is a light grey-brown** that stands out pale on dark skin. Quill's
  are painted dark brown; the default should follow the hair, or darken with the skin.
- **Form on very dark skin reads weakly** in the flat, unlit-specular style: real dark skin shows
  its form through sheen. A soft skin sheen would help every tone.
- **A wide brim casts a hard band of shadow** over the eyes; the hat sits high and leans back so
  her face shows.

**Left:**
- Seen in creator captures at rest only.
- The owner's look: the face, the skin tone, the purple.
- The ancestry presets themselves (§3.3, open question 4): Quill's values are one data point for
  a West African preset, and the presets need reviewers from the communities they depict.

## 7. Open questions for the owner

1. **Realistic over stylized topology** (§1.1), with heroic proportions from the rig: agreed?
2. **Underwear as regions of the body** (§1.5), always on: agreed?
3. ~~**D77** (§5): which moonshots to start?~~ Answered: M1 now, M5 likely, the rest later.
   M4's license stance waits for M4.
4. **Ancestry presets** (§3.3): the list, and who reviews them for respect and accuracy?
5. ~~**Hair**: how?~~ Answered 2026-11-12:
   - **Method:** shaped hair meshes worn on the head, spring bones on the long locks, braids and
     ponytails (BW6), strands drawn by the shader as on the brows; no transparency.
   - **Styles:** about twelve to start: shaved, buzz, short, swept, long, ponytail, topknot,
     braids, locs, coils and afro (their own shapes, not recolored straight hair), a mohawk, a
     dwarf crest. **A length control** where it suits the style.
   - **Beards and moustaches:** the same system (full, goatee, braided dwarf beard; stubble
     shaded on the skin).
   - **Color:** hair, brows and beard share one color, with an option to color the hair apart
     (and two-tone tips).
   - **Helmets:** the hair hides under a helmet. If that reads badly, flattened versions of
     each style for headgear come next.
