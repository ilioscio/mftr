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
- **Color:** the brows stay the body's paint region 3, as recipes painted them before. The Brows
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
| Sleeved shirt | torso, over | front, back, two sleeves of two halves each, turned along the arm | 2,214 |
| Wrap skirt | waist, over | front and back, flared to the knee | 1,688 |
| Long coat | back, armor | back and two open front panels, below the knee | 3,175 |

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
