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

## 7. Open questions for the owner

1. **Realistic over stylized topology** (§1.1), with heroic proportions from the rig: agreed?
2. **Underwear as regions of the body** (§1.5), always on: agreed?
3. ~~**D77** (§5): which moonshots to start?~~ Answered: M1 now, M5 likely, the rest later.
   M4's license stance waits for M4.
4. **Ancestry presets** (§3.3): the list, and who reviews them for respect and accuracy?
5. **Hair** is the big missing piece. The owner will answer its design questions after the
   lids, brows, lips and the new body. It will lean on BW6's spring chains, the brows' strand
   shading (§3.1) and the wardrobe's fitting (§4.2).
