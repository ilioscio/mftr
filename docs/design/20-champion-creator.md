# 20 — Champion Creator

Status: **Draft v0.1** (2026-11-07). Decisions D68–D73; open questions in §15.

Players make their own champions: a body, a face, clothes they paint themselves, a kit picked
from a catalog of moves, and numbers they tune within a fair budget. The official roster is made
the same way, with the same parts (D68). This changes where champions come from. Today each one is
a Blender file and a block of Rust data. After this, each one is a **recipe**: a small data file of
choices that the game builds into a model and a kit at load time.

The creator is inspired by the character creators of open-world RPGs (Oblivion, Skyrim), which let
players make almost any face from sliders. Ours has to go further: a player should be able to
make Abraham Lincoln, Pinocchio or an ogre, and then make that character play well in a MOBA.

## 1. Goals and principles

1. **Make the champion in your head.** The range must reach caricature and non-human shapes
   (a nose a hand long, a jaw wider than the skull, tusks, a snout), not only handsome variations
   of one face.
2. **Results always look made.** Sliders blend between shapes that artists sculpted, so a
   player who moves sliders at random still gets something with good topology and good facets.
   Free sculpting goes beyond them, inside bounds (§5.3).
3. **The official roster is built in it** (D68). If the creator can't make Rook as well as the
   hand-made Rook, it isn't finished. Players can open any official champion and remix it.
4. **Everything a player makes is data.** A recipe holds choices and numbers, plus small paint
   images. It never holds meshes, scripts or shaders. The parts, moves, effects and sounds live in a
   **catalog** that ships with the game (§11). This keeps recipes tiny (§11.2) and safe to push
   to everyone in a match ([11](11-content-packs-and-mods.md)).
5. **Fair by construction.** Every number that changes a fight has a price, and every champion
   spends the same budget (§9). The rules no budget can buy out are 05 §1 (readability), D10
   (reaction budget) and D36 (no point-and-click hard CC).
6. **The rules don't change for customs.** The timing contract ([10 §4](10-characters-and-animation.md#4-action-timing-contract)),
   facing, budgets, silhouette readability and the team accent all apply, checked by the same
   validator.
7. **Quick to try.** Every step previews live, and **Test** (§10) drops the champion into a
   practice room from any step.

## 2. The flow

Nine steps. The player can go back to any earlier step; later choices survive where they still
make sense (§2.1).

| # | Step | The player… | Produces (recipe, §11) |
|---|---|---|---|
| 1 | **Build** | picks a body type: slim humanoid, heavy humanoid, four-legged beast, serpentoid, floating spectre, rolling machine… | `build` |
| 2 | **Gender** | picks feminine or masculine (only on builds that have them) | `gender` |
| 3 | **Face** | shapes the head with sliders and free sculpting, picks eyes, skin, hair and brows | `face` |
| 4 | **Body** | shapes the body the same way: proportions, build, bulk | `body` |
| 5 | **Clothes** | equips clothing, armor, accessories and weapons from the catalog | `outfit` |
| 6 | **Paint** | colors every part: fill, brush, decals; picks the signature color | `paint`, `signature` |
| 7 | **Moveset** | picks the basic attack, passive, resource and Q W E R moves; picks their effects and sounds; tunes how they look | `kit` |
| 8 | **Balance** | spends the point budget on stats and ability numbers | `kit` numbers, `stats` |
| 9 | **Name** | names the champion and saves it | `name`, `title` |

**Test** is a button on every step (§10). **Randomize** and **Presets** (start from any official
champion or a saved one) are available on steps 1–6.

### 2.1 Going back
- Changing the **build** keeps nothing the new build can't use: sliders and parts are per build.
  The creator warns first. The kit's numbers survive when the new build's moves have the same shapes.
- Changing the **gender** keeps every slider (gender only sets the base shape, §4).
- Changing **clothes** keeps paint on parts that stay. A removed weapon flags the moves that
  needed it (§8.2).
- Changing **moves** keeps the balance spent on matching numbers where the shapes match.

## 3. Builds

A build is a body plan. It brings a rig archetype ([10 §7.1](10-characters-and-animation.md#71-archetypes)),
sculpted base meshes, the slider set, the catalog parts made for it, its **move library** and its
gait.

| Build | Rig | Gender | Notes |
|---|---|---|---|
| **Slim humanoid** | `biped` | ✓ | Ember, Lumen, Marrow, Quill, Shade, Vesper, Wren today |
| **Heavy humanoid** | `biped`, `large` shape | ✓ | Rook, Bastion, Cairn today; brute gait |
| Four-legged beast | `quadruped` | ✓ (subtle) | Own move library |
| Serpentoid | `serpent` (new) | — | A long spine chain, no legs; slithers |
| Floating spectre | `floater` | optional | No legs; hovers |
| Rolling machine | `roller` (new) | — | A wheel or tread body with arms |
| *More later* | | | Winged, small (`biped_small`), many-armed… |

**First version: the two humanoid builds** (D72). They share the `biped` skeleton, the shared
clip library and all ten current kits, so the whole flow is proven on one skeleton before new
rigs and move libraries arrive. Each later build is its own slice of work (§14).

Builds don't carry gameplay advantages of their own. What a build changes in a fight (hitbox
size, for one) is priced in the balance step (§9.3).

## 4. Gender

On builds that have it, gender picks the **base shape**: a feminine or masculine base mesh with
its default body and face sliders (shoulders, chest, hips, waist, jaw, brow). Every slider stays
available after the choice, so anything in between can be made. Builds without it (machines,
serpents) skip the step.

## 5. Face

### 5.1 How shapes are built (D69)

Each build and gender has a **base head** sculpted in Blender, faceted, about 800–1,500 triangles
*(start)*. Each slider is a **morph target**: an artist sculpts what the slider does at its
extremes (−1 and +1), and the slider blends between them. Sliders add together. The game
computes the result in Rust when a champion loads (§11.3). The idea comes from FaceGen
(Oblivion) and Skyrim's sliders. Sliders are the backbone: they keep results good-looking and are easy for
casual players. **Free sculpting** (§5.3) is on top of them for the extremes.

### 5.2 Sliders

About **70 sliders at first** *(start)*, grouped by region. Every group has a "reset" and a
"randomize".

| Region | Sliders (examples) |
|---|---|
| Head | width, length, depth, crown height, back of skull, temples |
| Forehead and brow | slope, height, brow ridge, brow-ridge width, glabella |
| Eyes | size, spacing, height, depth (sockets), tilt, opening, upper lid, lower lid, inner corner, outer corner |
| Nose | length, bridge height, bridge width, hook, tip size, tip droop/upturn, nostril flare, crook, nose width at the base |
| Cheeks | cheekbone height, cheekbone width, cheekbone prominence, hollows, fullness |
| Mouth | width, height, upper lip, lower lip, protrusion, corners (smile/frown), smirk (one side) |
| Jaw and chin | jaw width, jaw corner (angle and height), jaw line, chin width, chin length, chin reach, cleft, underbite |
| Ears | size, angle, point, lobe, height |
| Neck | width, length, Adam's apple |
| Extras | tusks, snout, horn nubs, fangs (morphs on builds that support them) |

Ranges go past natural faces on purpose. A Pinocchio nose is a slider at +1, not a hack.

Features that aren't shape are picked here too: **eye color**, **skin tone**, **hair** (a catalog
of styles fitted to the head, §7.1, with color), **brows** and **facial hair** (catalog parts with
color), **resting expression** (sliders that set brows, lids and mouth together: scowl, smirk,
calm, curious…).

### 5.3 Free sculpting

For what the sliders don't reach. Brushes: **push/pull**, **grab**, **smooth**, **inflate**,
**flatten**, with **symmetry on by default** and an adjustable radius and strength. Sculpting moves
the base mesh's vertices, so it only reshapes what is there. It never adds geometry, which keeps
the triangle budget and the faceted look.

- Stored as **sparse per-vertex offsets** on top of the sliders, quantized to 1 mm *(start)*.
- **Bounded** by the build's sculpt envelope: a cage around the head beyond which nothing moves
  *(start: 60% of the head's size outward, 30% inward)*. This keeps silhouettes in the readable
  range, keeps hitboxes honest (§6.2) and makes broken meshes impossible.
- Parts that sit on the head (hair, hats, masks) follow the sculpt (§7.1).

### 5.4 Faces in motion *(later)*

The faceted style has no facial animation today (10 §2). Blinks and emote expressions can come
later as morphs that the animator drives (§11.3), using the same sliders.

## 6. Body

### 6.1 Proportions and shape

Two kinds of sliders, both previewed in motion (an idle and a run loop beside the editor):

- **Proportions** change the **skeleton**: height, leg length, arm length, torso length, neck
  length, head size, hand size, foot size, shoulder width, hip width. They change bone lengths in
  the rest pose. Animations follow at runtime (§8.4).
- **Shape** changes the **mesh** through morph targets, like the face: muscle, fat, chest, waist,
  hips, belly, hunch, neck bulk, limb thickness (upper arm, forearm, thigh, calf).

Free sculpting (§5.3) works on the body too, with the same bounds.

### 6.2 What the body must keep

- **Readable size.** Height *(start)* 1.5–2.6 m for humanoids (14 §2.2's heroic range, widened),
  and limbs at least 0.06 m across (14 §2.3).
- **The footprint rule** ([10 §2](10-characters-and-animation.md#2-character-visual-style-faceted-ps1)):
  the model's footprint stays 0.8–1.6× the gameplay hitbox. The **hitbox follows the body**:
  the creator derives a hitbox size class (small / medium / large) from the finished shape, and
  balance prices it (§9.3). A giant is easier to hit, and a gnome costs points.
- **Animations stay sane.** Some proportions break clips (arms too short for a two-handed grip,
  for one). The editor shows a warning when a selected move can't adapt (§8.4).

## 7. Clothes and accessories

### 7.1 Parts

A **catalog** of parts per build, in slots:

| Slot | Examples |
|---|---|
| Head | hats, helmets, hoods, crowns, circlets, antlers, halos |
| Hair, brows, facial hair | styles (from step 3, re-listed here) |
| Face | masks, goggles, eyepatches, veils |
| Torso (inner, outer) | shirts, tunics, bodices, breastplates, coats, robes |
| Shoulders | pauldrons, mantles, collars, capelets |
| Arms, hands | sleeves, bracers, gloves, gauntlets |
| Waist | belts, sashes, tabards, kilts, skirts |
| Legs, feet | trousers, greaves, boots, sandals |
| Back | capes, cloaks, quivers, wings, banners, packs |
| Weapons (each hand, or two-handed) | bows, crossbows, staves, mauls, swords, blades, wands, shields, orbs, none |
| Trinkets | lanterns, scroll cases, pouches, tails, chains |

Every part is sculpted to the build's base mesh in Blender (the existing add-on gains a **catalog
part** export, §11.4) and carries:

- the build's **morph targets**, transferred when it's exported, so sliders reshape it with the
  body;
- a **binding** to the body's surface, so free-sculpt offsets carry over to it at load;
- **skin weights** to the build's bones, plus an optional **cloth chain** (capes, scarves, tails:
  the `extra_*` bones and the rope simulation of 14 §3.5);
- a **body mask**: the body faces it hides. Hidden faces are dropped from the mesh, which saves
  triangles and stops skin poking through;
- **paint regions** and a UV layout for its paint image (§7.3);
- **conflicts**: a hood hides hair, a helmet hides a hat, and so on.

### 7.2 Rules

- **Triangle budget:** the base mesh plus parts, minus hidden faces, stays within 12,000 (the cap).
  The target is 5,000–8,000 (10 §2). A meter in the corner shows the total, and the creator won't save over the cap.
- **Bones:** cloth chains share the 64-bone budget (10 §7.2). The meter shows that too.
- **The team accent** ([05 §1.3](05-art-and-assets.md#1-readability-rules-non-negotiable)): every
  outfit shows the `accent` band (a belt, a sash, a bracer). Waist parts carry an accent region.
  If the outfit leaves none visible at the gameplay camera, an accent band is added and can't be
  removed.
- **Silhouette:** big shapes on the outline (14 §2.3) are what a champion is recognized by. The
  creator shows a **silhouette preview** at the gameplay camera's size beside the close-up.

### 7.3 Weapons and moves

A weapon has a **class** (bow, crossbow, staff, heavy two-hander, one-hander, paired blades,
wand/orb, shield, unarmed). Moves name the classes they work with (§8.2), so the weapon picked
here filters the moves in step 7. From step 7, "change weapon" jumps back here with the move
list remembered.

## 8. Moveset

### 8.1 The kit

Every champion has a **basic attack**, a **passive**, a **resource** and **Q W E R**, as in
[01 §9](01-gameplay.md#9-champions). All of them come from catalogs:

| Choice | From | Examples |
|---|---|---|
| Basic attack | the build's attack moves | bow shot, crossbow bolt, staff bolt, maul blow, blade slash, punch |
| Passive | the passive catalog (shared; D74: the items' unique passives, nine to start) | on-hit magic, empowered every third hit, spellblade, cleave, a speed burst on hits, sunder, thorns, shield at low health, regeneration out of combat |
| Resource | fixed list (D75) | mana, energy, rage, health cost, none |
| Q W E R | the build's move library | line skillshots, lobbed areas, novas, dashes, lunges, blinks, shields, heals, beams, pulls, nets, slams |

### 8.2 Moves

A **move** pairs an animation with a gameplay **shape**:

- **The animation:** a clip (or the dash set: start, travel, land) on the build's reference
  skeleton, with the timing markers of [10 §4.5](10-characters-and-animation.md#45-markers).
- **The shape:** one of the sim's effects (04 §4; today `Line`, `Area`, `Dash`, `Blink`, `Shield`,
  `Support`, `Lunge`, `Beam`, plus what the catalog adds), with default numbers.
- **Its requirements:** the weapon classes it works with; the slots it can take (some moves are
  ultimate-only); the CC it can carry (hard CC only on skillshots and telegraphed areas, D36).

The move library starts from what exists: the ten champions' attacks and abilities and the
shared library's clips become the first **~50 moves** for the humanoid builds. Their animation
work carries over (§12).

### 8.3 Effects and sounds

- **Effects:** each phase of a move (release, projectile, impact, telegraph, land…) takes an
  effect from the **VFX catalog** (today's kit, [05 §5.1](05-art-and-assets.md#51-the-kit-a4b),
  growing), shared by every build. The player picks the kit and its free knobs (count, speed,
  lifetime, trail). The colors come from the **signature color** (14 §4: white → signature →
  dark signature), which the player can override per effect. **Sizes are never a choice**: they
  come from the gameplay numbers (hitbox honesty, 05 §1.1).
- **Sounds:** each event takes sounds from the **SFX catalog** (the synthesized recipes of 05 §7.1),
  shared by every build, with a volume and a pitch spread.

### 8.4 Tuning in the preview

Step 7's screen is mostly a **preview stage**: the champion on a pad facing a dummy, looping the
selected move, with the gameplay shape drawn on the ground.

- **Visual knobs** (free): the animation's **speed curve** inside its phases (how fast the
  windup builds, how long the follow-through lingers), trail length, particle counts, shake,
  effect timing offsets.
- **Gameplay knobs** (priced in step 8): windup time, projectile speed, width, range, area size,
  cooldown, damage, CC duration. They can be set here for the look and feel, and the balance
  step shows what they cost.
- **Adapting to the body:** clips store rotations, so they carry over to new bone lengths. On
  top of that, at load the animator (in Rust, [10 §6](10-characters-and-animation.md#6-runtime-blending)):
  - scales the pelvis height and stride with leg length (the run's playback rate already comes
    from stride speed);
  - plants the feet with IK on the `foot_l`/`foot_r` contacts;
  - puts both hands on two-handed weapons with IK (14 §3.3);
  - moves the projectile socket with the weapon.

  A move that can't adapt within its limits (a grip out of reach, for one) is flagged in step 7.

## 9. Balance

The most important system in the creator, and the one that has to be designed most carefully.
A custom champion has to be able to sit in a normal match on a server that allows customs (D68)
without being an obvious pick or an obvious loss.

### 9.1 The budget (D71)

Every champion gets the same **budget** of power points *(start: 100)*. Every number that
changes what happens in a fight has a **price**:

- base stats and their growth (health, armor, magic resist, attack damage, attack speed, move
  speed, regen);
- the basic attack: range, windup, damage;
- the hitbox size class (§6.2);
- the resource: cost and regeneration (a costless kit pays for it);
- the passive;
- each ability: damage and ratios, cooldown, range, speed, width, area size, windup, CC kind and
  duration, shield and heal amounts, dash distance.

Each value has a **baseline** that costs nothing. Going above it costs points, and going below
refunds some. **Prices grow faster than linearly** toward the extremes, so spreading power is
cheaper than stacking it in one place. Refunds shrink toward the extremes, so a
"dump stat" doesn't pay for a monster.

### 9.2 Prices from expected effect, not raw numbers

An ability isn't priced by its damage alone but by **what it's expected to do**:

> value ≈ expected outcome per cast × chance to land ÷ cooldown

- **Chance to land** comes from the hit-probability model the bots already use (D62, 17): width,
  speed, range and windup in, a probability out. A wide, slow skillshot and a thin, fast one with
  the same damage get different prices, and the right ones.
- **Outcomes** are damage, CC seconds (hard CC weighted above soft), mobility (distance per
  cooldown) and protection (shield and heal per cooldown), each with a price weight.

### 9.3 Calibrated by simulation

The price weights are **fitted, not guessed**:

1. **Anchors:** the ten official champions, rebuilt in the creator (D68), each fit the budget.
   They set the scale.
2. **The random league:** `mftr-tools` generates thousands of random legal champions, plays them
   in headless bot matches (the bots already play any kit built from the sim's shapes) and fits
   prices so that spending the budget any way gives about the same win rate *(target: every
   generated champion 45–55% at equal bot tier)*.
3. **Each release** may ship a new **price table**, versioned. A recipe stores its numbers, not
   points, so it is re-priced under the new table. A champion that ends up over budget is
   flagged in the creator, with **Auto-fit** (scale the most expensive numbers down to fit).
4. **Live data** *(later, opt-in)*: servers can report pick and win rates per move to the project
   for the next table.

### 9.4 What no budget buys

Hard limits, checked by the validator whatever the points say:

- Hard CC only on dodgeable deliveries (D36), and each one passes the reaction budget (D10).
- Per kit: total hard CC seconds, total dash distance and shield uptime have caps *(start values
  in the price table)*.
- Minimum cooldowns, maximum ranges and speeds, minimum windups for skillshots.
- Everything visible stays honest (05 §1).

### 9.5 The screen

A list of every priced value grouped by kit part, with the points left in a bar at the top, and a
**power profile**: damage, burst, durability, mobility, CC and utility on one chart, beside the
average of the official champions of the closest class. **Presets** spend the budget in a class's
typical way (mage, marksman, tank…) as a starting point.

## 10. Test: the practice room

**Test** loads the champion into a **practice room** without leaving the creator: a local
sandbox server (the Duel Sandbox, run in-process) with:

- **dummies** that stand still, walk, dodge with bot micro, or fight back; set their armor, magic
  resist and health;
- a **damage meter**: DPS, burst over a window, damage by ability, time to kill each dummy;
- controls to set the level and items, reset cooldowns and toggle infinite resource;
- a **back to the creator** button that keeps the step the player came from.

## 11. Data and runtime

### 11.1 The recipe

A recipe is RON. A sketch, not the final schema:

```ron
Champion(
    format: 1,
    catalog: "2026.11",              // the catalog version its ids come from
    name: "Old Grumble", title: "the Bridge Troll",
    build: "humanoid_heavy", gender: Masculine,
    face: (
        sliders: {"nose.length": 0.8, "jaw.width": 0.6, "extras.tusks": 1.0},
        sculpt: [(vertex: 412, offset: (0, -12, 4)), (vertex: 413, offset: (0, -10, 3))],   // mm
        eyes: (color: 3), skin: 17, hair: ("hair.shaggy", color: 4), brows: ("brows.heavy", color: 4),
    ),
    body: (sliders: {"height": 0.4, "arms.length": 0.7, "bulk": 0.9}, sculpt: []),
    outfit: ["torso.patched_tunic", "waist.rope_belt", "back.club_sling", "weapon.two_handed.club"],
    paint: {"torso.patched_tunic": (fill: [(region: 0, color: 21)], image: "paint/0.idx")},
    signature: (hue: 95),
    kit: (
        attack: (move: "heavy.club_swing"),
        passive: (passive: "rage_on_hit"),
        resource: Rage,
        q: (move: "heavy.ground_slam", numbers: {"damage": 90, "radius": 300, "cooldown": 9.0},
            vfx: {"fire": ("crack", count: 18)}, sfx: {"fire": ["thud_heavy_2"]}),
        // w, e, r…
    ),
    stats: {"health": 640, "armor": 34, "move_speed": 335},
)
```

- Colors are **indices into the game's palette** *(start: 64 colors)*, not free RGB. A shared,
  tuned palette keeps champions looking like one game, and the toon shader's bands stay clean.
- **Paint images** are palette-indexed (4 bits a pixel, 16 colors each) at 64×64 or 128×128
  *(start)*, stored raw in the recipe. They need no image decoder, which keeps 11's rule of
  memory-safe, minimal parsers (D70).
- **Caps:** a recipe ≤ 256 KB uncompressed *(start)*, nearly all of it paint. A champion with no
  brush paint is a few KB.
- **Identity:** the recipe's id is its BLAKE3 hash, signed by its author's key (11 §5, 07 §3).

### 11.2 Why recipes, not models

A hand-made champion today exports about 0.6 MB (model, clips, effects, sounds), within a
1.7 MB budget (05 §8). A recipe is 2–200 KB, and everything it points to is already on every
player's machine. This matters for:

- **Joining:** ten customs in a match cost at most a couple of MB to push, usually a few hundred
  KB, against the 40 MB per match that 11 §6 allows for packs.
- **Safety:** customs carry no glTF, so the biggest parser in the threat model (11 §2) isn't
  exposed to them.
- **Size:** the catalog is shared. A roster of 40 official champions as recipes costs far less
  than 40 × 1.7 MB (05 §8), and every new part serves every champion.

### 11.3 Building a champion at load (`mftr-forge`)

A new Rust crate, `mftr-forge`, turns a validated recipe into what the client draws and the sim
needs. It is deterministic and has no Godot in it, so the server, the client, the tools and
tests all run it:

1. **Skeleton:** the build's skeleton, with bone lengths from the proportion sliders.
2. **Mesh:** base mesh + slider morphs + sculpt offsets, then each part with the same morphs
   and its binding to the sculpt; hidden faces dropped; skinning.
3. **Colors:** vertex colors and paint images per part; slots per paint region.
4. **Animation:** the build's clips for the chosen moves (plus locomotion, CC and emotes), with
   the §8.4 fix-ups; markers retimed by the sim's numbers as today.
5. **Kit:** the sim's champion data (stats, attack, passive, resource, abilities) compiled from
   the kit, priced (§9) and checked (§9.4).
6. **Derived gameplay values:** the hitbox size class, the projectile socket's height.

The client hands steps 1–4 to Godot through gdext as arrays, the same way validated packs reach
it today (11 §4). The server only needs steps 5–6. *Target: ≤ 50 ms per champion on the minimum
spec, so a ten-champion match loads in under half a second of building.*

### 11.4 The catalog

Base meshes, morph targets, parts, moves, passives, effects and sounds, with an id and a version
each. It ships with the game, and grows with each release.

- **Authored in Blender** with the existing add-on, which gains **base mesh**, **morph**,
  **catalog part** and **move** exports. The validator checks them like packs today (10 §8.4).
- **Versioned:** a recipe names the catalog version it was made with. Ids never change meaning;
  a reworked part gets a new id, and the old one stays (or maps to its replacement).
- **Size:** counts toward the download target (00 §3). *Estimate per humanoid build: base meshes
  and morphs ~1 MB, ~150 parts ~6 MB, ~50 moves ~4 MB.*
- **Community catalog packs** *(later)*: new parts and moves as signed packs (11), validated
  like everything else.

### 11.5 In the sim

Champions can't stay a fixed list of Rust constants (`ChampionDef` in `mftr-sim/champion.rs`,
D26). The sim takes champion data at runtime (D8 at last, D73): champion files are read by the
sim's own strict reader (`mftr-sim/src/data.rs`), and a `ChampionId` is a handle into a registry
that any definition can join, with the official ten at ids 0–9. A champion's identity in a match
is its recipe hash, and the server's Welcome carries the compiled kits (or their hashes, which
clients check against their own compile). Client prediction runs the same compiled kit, so it
stays exact.

## 12. What happens to today's roster and pipeline

- **The ten champions are rebuilt as recipes** (D68), their Blender models used as references
  for the base meshes, the parts and the presets. The goal is to match or beat them.
- **Their animations become moves:** every attack and ability clip, the shared library and the
  locomotion sets move into the humanoid move libraries. Little animation work is lost.
- **Their kits move to data** and into the catalog as moves and passives (Shade's veil, D65,
  becomes a move; Quill's beam is one already).
- **Bastion** (a block head with a visor slit) needs heavy-humanoid parts that cover the face.
  **Shade** (a masked face under a hood) needs a mask and a hood. Both are ordinary parts.
- **The Blender pipeline stays**, aimed at the catalog instead of at whole champions.
  `art/champions/<id>/` becomes the source for each official champion's recipe and references.
  Their kits are data already (`crates/mftr-sim/champions/<id>.ron`, D73).

## 13. Hosting, trust and moderation

- **Host setting:** `champions = official | customs | both` *(default: official)*. Hosts can also
  pin the price table version and allow or refuse community catalog packs.
- **Delivery:** the server pushes the match's recipes during champion select (11 §6). Every
  client validates and prices them itself, and refuses to join if one fails.
- **Bots** play customs like any champion (D62: their fight model reads any kit built from the
  sim's shapes).
- **Names and paint** are free text and free drawing, so they can be offensive. Player controls:
  a **"show default looks"** setting that draws customs in neutral paint with generated names,
  per-player mute of a champion's look, and the project's blocklist of recipe hashes and author
  keys (11 §5). Reporting comes with M5's moderation work (07 §5).
- **Likenesses:** see §15.

## 14. Rollout (track CR)

| Slice | What | Exit |
|---|---|---|
| **CR1** Data-driven champions | The sim takes champion data at runtime; the ten kits move to RON (D8, D73); the recipe format v0 (kit and stats only) and `mftr-pack` validation; per-champion code becomes data; the server pushes recipes; client prediction on compiled kits | Every current champion plays from data with golden hashes unchanged; a match with a custom champion replays to the same hashes |
| **CR2** `mftr-forge` | Mesh, skeleton, colors and animation from a recipe; base meshes and the first 20 face and body sliders for slim humanoid (both genders); the new Blender exports; the client draws forged champions | A forged slim humanoid runs, casts and dies in a match |
| **CR3** The creator shell | The step flow, preview stage, save, load and presets; build, gender, face and body editors with sliders; Test into the practice room | Make and test a champion without leaving the game |
| **CR4** Clothes | The part catalog's first ~60 parts, fitting, body masks, conflicts, cloth chains, budget meters | The ten champions' outfits rebuilt from parts |
| **CR5** Paint | Paint images, fill, brush, decals, the palette, signature color; D70's paint images in the shader | The ten champions' colors repainted in the creator |
| **CR6** Moveset | The humanoid move library from the ten kits (~50 moves), passives, resources, the VFX and SFX catalogs, the preview stage's tuning, runtime IK fix-ups | The ten kits rebuilt from moves play and look as they do today |
| **CR7** Balance | The price table, the hit-probability pricing, the random league, Auto-fit, caps; the ten official recipes fit the budget | Random-league champions win 45–55% at equal tier |
| **CR8** Heavy humanoid | Its base meshes, sliders, parts and moves | Rook, Bastion and Cairn rebuilt |
| **CR9** Free sculpting | Brushes, envelopes, sculpt bindings for parts | A nose and a jaw past the sliders' range, with a hat and a beard following them |
| **CR10+** More builds | Beast, serpentoid, spectre, machine: each a rig, a base mesh, sliders, parts and a move library | Per build: a champion of it plays a full match |

**Track exit:** the ten official champions are recipes and the owner prefers them to the
hand-made ones; a new player makes a recognizable famous figure in under 20 minutes (playtest);
a match of ten customs loads and runs within the budgets of 05 §8–9.

The roster's growth from 10 to 16–20 (M4) waits for CR: new official champions are made in the
creator (D72).

## 15. Open questions

1. **Gender options:** feminine and masculine as base shapes (§4). Is a third, neutral base shape wanted?
2. **Likenesses:** players will make real people (Lincoln) and other studios' characters (an ogre
   that's obviously Shrek). Privately that's their business. What is the policy for customs
   pushed by community servers, and for any future shared gallery? (09 has to answer this before
   any sharing beyond servers.)
3. **Ranked:** customs are allowed on servers that allow them (D68). Is a ranked queue ever
   allowed customs, or only official recipes?
4. **Duplicates and copies:** can two players bring the same recipe into one match? Can a player
   bring someone else's recipe (it's signed by its author, not by who brings it)?
5. **Sharing:** beyond server push and files, is there ever a gallery? A central one conflicts
   with "no mandatory central service" (00 §2); a federated one waits on M6.
6. **Recasts and extra abilities:** do moves with stages (10 §4.1 `stages`) or a fifth ability fit
   the kit, or does every kit stay attack + passive + Q W E R?
7. **The price table's authority:** project-only, or can hosts edit prices for their servers?
8. **Voices:** the flow has no voice step. Generated barks from the SFX synthesizer, or none?
