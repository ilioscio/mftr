# 10 — Characters & Animation

How champions look, move and animate, and the pipeline that builds them. This expands [05 §4](05-art-and-assets.md#4-champions). It's written so that first-party champions and community champions ([11](11-content-packs-and-mods.md)) go through the **same** rig, rules, tools and validator.

**Quality bar:** every champion ships with its *complete* animation set (§5). Not a placeholder set to finish later. Genre entries that cut corners here (Dawngate, Paragon, Infinite Crisis) felt floaty and unreadable, and players notice that in the first minute. Most of the "feel" lives in the timing contract (§4) and the release and impact frames (§4.4), not in the polygon count.

## 1. Principles

1. **The sim owns time; the animation fits it.** Gameplay timings (windup, fire time, locks) are data in the sim. An animation is *retimed* to fit them (§4.3), never the other way round. The validator rejects clips whose markers don't line up.
2. **Animate for the gameplay camera.** At default zoom a champion is ~110–130 px tall at 1080p (camera from [R01](reference/R01-video-ezreal-flash-barrier-q.md)). Poses must read at that size: strong lines of action, wide arcs, big weapon silhouettes. Close-up quality is a bonus. Reviews happen at the gameplay camera (§8.5).
3. **Release and impact are punctuation.** Anticipation goes in the windup, the release is 1–2 frames, and the follow-through is decoration that any order can cut ([R04 §2](reference/R04-video-facing-and-attack-animation.md#2-basic-attack-ranged-bow)).
4. **Facing snaps** (§3). No turn-in-place, pivot, start or stop animations.
5. **No root motion.** The sim moves the unit and animations play in place. Dashes and lunges animate in place while the sim carries the body.
6. **One rig standard, many champions.** Shared bone names make shared clips (CC states, generic casts) and community templates possible (§7).
7. **Same pipeline for everyone.** First-party champions are built as content packs ([11](11-content-packs-and-mods.md)), so the mod path is never a second-class copy.

## 2. Character visual style ("faceted PS1")

Decided with the owner (D48). The reference is a reduced-poly reimagining of the genre: faceted, flat-shaded, chunky shapes, with crunchy pixel-art VFX.

| Aspect | Rule |
|---|---|
| Geometry | **2,500–4,000 triangles** per champion including weapons *(start)*, hard cap 6,000. Large forms are blocky and faceted. Detail comes from silhouette and color blocking, not geometry. |
| Shading | **Faceted** (flat per triangle), computed in the shader from screen-space derivatives so meshes stay smooth-indexed (fewer vertices, smaller files). Toon ramp with 2–3 bands, rim light and object-space detail noise ([05 §2](05-art-and-assets.md#2-procedural-materials)). |
| Color | **Vertex colors**: RGB = albedo, A = baked cavity/AO. No texture maps. A palette of ≤ 12 colors per champion. |
| Materials | At most 4 material slots from a fixed set: `skin`, `cloth`, `metal`, `emissive`, plus `accent` (the team-accent region, tinted ally/enemy at runtime, [05 §1.3](05-art-and-assets.md#1-readability-rules-non-negotiable)) and `accent_glow` (the team color, glowing: minions' eyes and orbs, A5). Each slot maps to a parameter set of the one champion shader. |
| Silhouette | Identifiable from silhouette alone at default zoom (05 §1.5). Weapon and head shapes do most of the work. Each champion's `concept.md` states its silhouette class. |
| Scale | 1 Blender unit = 1 m = 100 u. A standard biped stands **~1.9 m** (190 u) *(start)*. At idle, the model's footprint spans 0.8–1.6× the gameplay hitbox diameter (130 u at the default 65 u radius), so what you see is what can be hit. |
| VFX | **Pixel style:** particles and sprites snap to a screen-space pixel grid with nearest filtering, use 4–6 color ramps per effect, and animate "on twos" (stepped at 15–20 fps). **Exception:** gameplay-relevant edges (hitbox sheaths, telegraphs) stay smooth, exact and full rate. Readability beats style. |
| Environment | Faceted geometry plus nearest-filtered procedural noise quantized to world-space "texels" (~4 cm) for the crunchy ground and stone look, with no textures. |
| PS1 extras | Vertex snapping, affine warping and dithering are **optional settings, off by default**. |

## 3. Facing

Measured in [R04 §1](reference/R04-video-facing-and-attack-animation.md#1-facing): the reference turns ~150–180° in ~2 frames (≈ 33 ms) with one in-between pose. The run cycle keeps playing through the turn, and nothing turns before the server confirms the order.

- **Facing is sim state** (`facing`, a unit vector; exact for the own champion, 10 bits on the wire for others, as planned in [03](03-netcode.md)). It's deterministic, predicted for our own champion and available to future mechanics (backstabs, facing-based shields, "turn away" fears).
- **Sim rules:** facing snaps to the current path segment while moving; to the target at windup start for attacks and targeted casts; to the aim direction at windup start for skillshots; and to the travel direction for dashes and lunges. During a windup it tracks a committed target. It doesn't change while stunned, rooted (unless casting) or idle.
- **Display:** the drawn facing approaches the sim facing at **4,500 °/s** *(start)*, so 180° takes ≈ 40 ms, matching the reference's one in-between frame. There's no easing and no lean. A **run-lean** (≤ 8° roll into the turn, decaying over 120 ms) is an optional per-champion flourish.
- **Our champion turns on the click frame** because movement is predicted, ~100 ms sooner than the reference. Remote champions turn on T_interp with their position.
- **Locomotion keeps its phase** through turns. The run cycle never restarts on a direction change.

## 4. Action timing contract

Every basic attack and ability is an **action** with phases defined in sim data. Animations carry **markers** that the client lines up with those phases.

### 4.1 Phases (sim data)

```
order ─► WINDUP ──► FIRE ──► FOLLOW-THROUGH ──► end
          │          │        │
          │          │        └ soft lock: rooted; a move, attack-move, stop or cast ends it at
          │          │          once (no loss); an attack order waits until it ends
          │          └ projectile spawns / melee lands / effect applies (exact sub-tick time)
          └ rooted (unless the ability is `mobile`); facing tracks the target;
            casts during it are BUFFERED; a move CANCELS a basic attack (§4.2)
```

| Field | Meaning | Status |
|---|---|---|
| `windup` | Order → fire. Attacks: `windup_fraction × period`, scaled by attack speed. Abilities: fixed ms. | Exists ([02 §7](02-combat-math.md)) |
| `follow_through` | Fire (or a dash's landing) → end. A **soft lock**: the caster stays put. A move, attack-move, stop or new cast ends it at once with no loss; an **attack order waits** for it (so cutting it takes a deliberate move: the Riven/Caitlyn tech). A caster still walking somewhere when it fires skips it. Its job is to punish *idle* players, not active ones. | A2 (`Recovery` in the unit state) |
| `hard_lock` | The start of the follow-through that nothing ends early: casts are buffered, moves take effect when it ends. Rare, for heavy finishers and ultimates. Default 0. | A2 |
| `windup_cancel` | By action kind, not per ability (nothing needs otherwise yet): a move during a **basic attack**'s windup cancels it (orb-walking); **abilities** buffer. | A2 |
| `mobile` | The caster keeps moving during the windup (upper-body animation layer, §6). | A2 (no current ability uses it) |
| `stages` | Recast chains (Riven-style 3-part abilities): stage counter, recast window, a separate clip per stage. | With the first champion that needs it |

**Input buffer:** one slot for a **cast** ordered while a cast is winding up, a dash is travelling or a hard lock holds. It starts the instant that ends, instead of being dropped. A newer order (a move, stop, attack or another cast) replaces it, and hard CC clears it. Moves need no buffer: one issued during a windup sets the order at once and the caster, rooted anyway, walks when the cast fires. This is what makes combos (dash → skillshot, the Caitlyn E→Q pattern) and quick follow-ups feel crisp instead of eaten. It's also the skill ceiling: a player who cancels the follow-through gains time, and a player who buffers well never loses inputs.

### 4.2 Defaults *(start)*

| Action kind | windup_cancel | follow_through | Notes |
|---|---|---|---|
| Basic attack (ranged) | Cancel | `period − windup`, implicit | Purely cosmetic: it ends at the attack timer, which already gates the next attack, and an attack order chases a target leaving range at once (as in R04). Cut it with a move to orb-walk |
| Basic attack (melee) | Cancel | `period − windup`, implicit | Hit lands on fire |
| Line / area ability | Buffer | **200 ms** | The follow-through sells the cast; cancelling it is the "animation cancel" tech |
| Dash / lunge | Buffer | **80 ms** after landing | Landing pose, cancellable |
| Blink, self buff / heal / shield | — (instant), `mobile` | 0 | Upper-body only |
| Ultimate (line / area) | Buffer | **350 ms**, the first **150 ms** a `hard_lock` | Weight and commitment |
| Utility spells (D, F) | — | 0 | Instant |

These are `Ability::timing` in `mftr-sim`, by effect kind and slot; per-ability tuning arrives with the pilots.

### 4.3 Retiming

A clip is authored at its **reference timing** (the champion's base attack speed, or the ability's data windup). At runtime the client maps sim phase progress to clip time *piecewise*:

```
[0, windup]          → [0, marker(fire)]            (attack-speed scaling compresses this)
[windup, windup+ft]  → [marker(fire), marker(end)]
```

So a 2.0 attack-speed champion still releases on the exact fire time. The draw just gets faster. **The pose is a function of sim time**, so remote windups that are fast-forwarded on T_input ([03a §7](03a-netcode-time-and-prediction.md#7-display-policy-what-is-drawn-when)) and rolled-back predictions land on the right pose without any special handling. A cancelled follow-through blends to locomotion over **50 ms**.

### 4.4 Release and impact

- **Release:** the projectile spawns at the `socket_projectile` bone on the fire marker, at full speed from frame one. A ≤ 1-frame **smear** (bone scale on a `smear` bone, or a weapon-trail ribbon) is allowed and encouraged on fast swings.
- **Projectile:** a thin, very bright head with a fading tail of up to ⅓ of the flight, never ahead of or wider than the hitbox ([05 §1.1](05-art-and-assets.md#1-readability-rules-non-negotiable)).
- **Impact (on the target):** 2–3 frames of additive white flash in the champion shader, a spark burst from the VFX kit at `socket_chest`, and the damage number ≈ 3 frames later. Minions flinch; champions **don't** (readability, matching the reference). Crits get a bigger flash, a distinct spark ramp and a distinct sound.
- **Hover:** attackable targets under the cursor get an outline (red for enemies) and an attack cursor.

### 4.5 Markers

Authored as Blender pose markers on each action, exported to the `.anims.ron` sidecar (§8.3):

| Marker | Required for | Meaning |
|---|---|---|
| `fire` | Attacks, abilities with a windup | The sim fire time |
| `hit_1`…`hit_n` | Multi-hit actions | Each sim hit time (must match the data) |
| `end` | All one-shots | The end of the follow-through |
| `loop_in` / `loop_out` | Channels, recall, airborne | Loop region of a clip with an intro and outro |
| `foot_l` / `foot_r` | Locomotion | Contact frames, for footstep SFX, dust VFX and stride sync |
| `fx_<name>` / `sfx_<name>` | Optional | Cosmetic triggers (trail on/off, glints, whooshes) |

## 5. The complete animation set

Every champion ships everything in the **Required** column. Clips marked *(shared)* have a default in the archetype library (§7.3) that is retargeted automatically. A champion may override them, and should when its personality calls for it.

### 5.1 Locomotion and idle

| Clip | Required | Notes |
|---|---|---|
| `idle` | ✅ | Loop, 2–4 s. Breathing, weight, personality. |
| `idle_fidget_1`, `idle_fidget_2` | ✅ | One-shots after 6–10 s without orders *(start)*. Character moments. |
| `idle_ready` | ✅ | A combat-ready loop for 3 s after an action, so the champion doesn't relax mid-fight. |
| `run` | ✅ | Loop with `foot_l`/`foot_r`. Authored stride speed recorded in the sidecar. |
| `run_fast` | ✅ | Speed ≥ 1.35× base *(start)*: haste, fountain, speed buffs. |
| `walk` | ✅ *(shared)* | Speed ≤ 0.6× base: heavy slows, so slowed champions don't moonwalk. |

The run playback rate is `current speed ÷ authored stride speed`, clamped to 0.75–1.35 *(start)*. Outside that range the champion switches to `walk` or `run_fast`. Transitions are 80 ms blends.

### 5.2 Basic attacks

| Clip | Required | Notes |
|---|---|---|
| `attack_1`, `attack_2` | ✅ | Alternating, chosen by the sim's attack counter (deterministic, so replays and spectators match). A third is optional. |
| `attack_crit` | ✅ | When critical strikes exist (items, M4). The crit is rolled at windup start so the right clip plays from frame one (Q16). |
| `attack_empowered_*` | Per kit | Empowered next-attacks and passives. |
| `attack_melee_alt` | ✅ *(shared)* for ranged champions | Augments that turn ranged attacks into melee strikes (Close Quarters, D46). |

### 5.3 Abilities

Each of Q, W, E and R maps to clips according to its effect kind, and the validator derives the required list from the sim data (§8.4):

| Effect kind | Clips |
|---|---|
| Line skillshot / targeted | `<slot>` (windup → `fire` → follow-through) |
| Delayed area | `<slot>` (gesture toward the target point; `fire` = area placed) |
| Self area (nova) | `<slot>` (`fire` = the area applies) |
| Dash / lunge | `<slot>_start` (launch pose, `fire` = departure), `<slot>_travel` (loop, in place), `<slot>_land` (one-shot, cancellable) |
| Blink | `<slot>` (vanish pose; the arrival is VFX plus the `idle_ready` pose) |
| Shield / heal / buff | `<slot>`; upper-body when `mobile` |
| Channel | `<slot>` with `loop_in`/`loop_out` |
| Recast stages | `<slot>_1`, `<slot>_2`, `<slot>_3` |
| Transform / stance | `<slot>` plus a full alternate locomotion set if the stance changes movement |

Delivery transformers (Multishot, Echo, Broadside, D44) reuse the same clips: an Echo replays the cast's VFX without the body animation.

### 5.4 Utility and system

| Clip | Required | Notes |
|---|---|---|
| `cast_utility` | ✅ *(shared)* | Blink, Barrier, Vault, Stormcall, Mend and future utility spells. Short, upper-body, `mobile`. |
| `recall` | ✅ | Channel with `loop_in`/`loop_out` (~8 s), then a teleport-out pose. A signature character moment. |
| `death` | ✅ | One-shot ending in a held ground pose. |
| `respawn` | ✅ | ≤ 1 s at the fountain. |
| `select` | ✅ | Showcase one-shot for champion select and the start menu. |
| `victory`, `defeat` | Optional | End-of-match poses. |

### 5.5 Crowd control (readability)

CC poses are gameplay information: "that champion can't act" must read at a glance across a teamfight.

| Clip | Required | Notes |
|---|---|---|
| `cc_stunned` | ✅ *(shared)* | Loop: dazed sway. Plus the hard-CC accent VFX. |
| `cc_rooted` | ✅ *(shared)* | Additive lower-body struggle over idle, attacks and casts (rooted units can still act). |
| `cc_airborne` | ✅ *(shared)* | Knock-ups: `loop_in` launch, loop, `loop_out` land. The height curve comes from the sim. |
| `cc_knockback` | ✅ *(shared)* | Slide pose while displaced. |
| `cc_suppressed` | ✅ *(shared)* | Loop. |
| `cc_sleep` | ✅ *(shared)* | Loop. |
| `cc_forced_move` | ✅ *(shared)* | Additive overlay on `walk` for fear, charm and taunt. The flavor comes from VFX. |
| Hit reaction | — | **None** on champions (the impact flash only, §4.4). Minions have a flinch. |

### 5.6 Emotes

| Clip | Required | Notes |
|---|---|---|
| `emote_taunt`, `emote_joke`, `emote_laugh` | ✅ | One-shots, cancellable by any order. Emote-cancelling is allowed tech: an emote order cancels a follow-through like a move does. |
| `emote_dance` | ✅ | Loop. |

**Count:** ≈ 35–45 clips per champion, of which ~10 come from the shared library by default. That's in line with what the genre's leader ships, and it's what this section exists to protect.

## 6. Runtime blending

The layer stack is evaluated in Rust (`mftr-pack`'s `animator`), not by a Godot AnimationTree: the pose is a pure function of sim state and time, testable without the engine, and the same code will serve tools and replays. The Godot extension only copies the resulting local transforms into the model's `Skeleton3D` (A3).

```
Locomotion   idle ─ walk ─ run by displayed speed; playback rate = speed ÷ stride speed, one
             shared cycle phase so switching walk ↔ run keeps the step
Action       attack or cast; clip time from the sim phase (§4.3), never free-running playback
             (upper-layer clips masked to spine_01 and below it; full clips over everything)
Additive     cc_rooted (and later cc_forced_move, run-lean)
Override     death (holds its last frame), cc_stunned; later recall and emotes
```

**Events** (A4c, A6): the animator reports what happened each update, for sounds and effects:
- a footstep when the walk or run crosses `foot_l`/`foot_r`;
- a windup's start;
- **`fire`** when an action passes its `fire` marker. That is the moment a melee blow lands, a slam hits, a nova sweeps or a heal pulses, so effects with no projectile hang off it as `<action>.fire`. A cast that ends at the very end of its windup (a caster walking on skips the follow-through) still fires.
- **Instant casts** (supports, shields) have no windup, so the drive never shows them. The sim announces them with a `CastStarted` whose `fire_at` equals its start, and the client **pulses** the animator: the clip plays from `fire` to its end, and fires.

Fallbacks: a champion without its own `idle`, `run`, `death`, attack or ability clips plays the shared library's (`idle`, `run`, `death`, `attack_melee_alt`, `cast_utility`), so a pack with gaps, or the template, still animates. Inputs come from the client's state: the own champion's windup and follow-through progress are exact (predicted sim times); other units' cast windups come from their `CastStarted` events, their follow-throughs from the status flag, and their attacks (no timing on the wire yet) play at the clip's own rate.

| Transition | Blend *(start)* |
|---|---|
| idle ↔ run | 80 ms |
| locomotion → action | **0–1 frame** (attacks start planted immediately, R04) |
| action follow-through → locomotion (cancel) | 50 ms |
| action → action | 0 ms (the next action's first pose takes over) |
| anything → CC override | 0 ms (CC must read instantly) |
| CC → locomotion | 100 ms |

## 7. Rig standard

### 7.1 Archetypes

| Archetype | For | Notes |
|---|---|---|
| `biped` | Most champions | The reference skeleton below |
| `biped_large` | Brutes, golems | Same bone names and proportions profile; shared clips retarget. Built as the `biped` bones in the `large` shape (`rig.use_shape("large")`: ~2.05 m, ×1.25 wide, ×1.2 deep), so the sidecar says `biped` (A6, Rook) |
| `biped_small` | Tiny champions | Same names; shared clips retarget with a compressed stride |
| `biped`, minion shape | Lane minions (A5) | The `biped` bones placed by `rig.use_shape("minion")`: chibi proportions (a big head on short legs, ~1.05 m; the super minion ×1.6). Same names, so the validator, animator and clip tools apply unchanged |
| `quadruped` | Beasts, mounts | Own shared library |
| `floater` | Spirits, serpents, constructs | Spine chain plus sockets, no legs |
| `custom` | Anything else | Must provide the required sockets (§7.2) and every clip itself |

### 7.2 `biped` v1

```
root                         (ground, origin; never translated in XY by clips)
└─ pelvis
   ├─ spine_01 ─ spine_02 ─ chest
   │                         ├─ neck ─ head ─ [jaw]
   │                         ├─ clavicle_l ─ upperarm_l ─ forearm_l ─ hand_l ─ [fingers_l, thumb_l] ─ prop_l
   │                         └─ clavicle_r ─ upperarm_r ─ forearm_r ─ hand_r ─ [fingers_r, thumb_r] ─ prop_r
   ├─ thigh_l ─ calf_l ─ foot_l ─ toe_l
   └─ thigh_r ─ calf_r ─ foot_r ─ toe_r
extras: extra_<chain>_<n>    (capes, hair, tails, wings: ≤ 16 bones)
sockets (non-deforming): socket_projectile, socket_weapon_tip, socket_cast, socket_chest, socket_overhead
optional: smear (scale-only, for 1-frame smears)
```

- **~30 deforming bones plus extras and sockets; ≤ 64 total**, hard cap 80. Fingers are "mittens": one bone for the fingers and one for the thumb, which fits the faceted style.
- **Weights:** at most **2 influences** per vertex *(start)*, hard cap 4. Armor plates are rigid (1 influence), the authentic PS1 look and the crispest facets.
- **Props** (weapons) are separate meshes parented to `prop_l`/`prop_r`, so they can be thrown, swapped or hidden by clips.
- **Axes:** the character faces **−Y in Blender** (looking at you in Front view), which exports to glTF +Z = Godot's `MODEL_FRONT`. Z is up. The rest pose is an A-pose.
- `socket_projectile` is where projectiles spawn (bow nock, muzzle, casting focus). `socket_overhead` sets the health-bar height. `socket_chest` is the impact point.

### 7.3 Shared library

`art/library/<archetype>/` holds the shared clips (`walk`, `cast_utility`, `attack_melee_alt`, all `cc_*`) authored on the reference skeleton. The exporter **bakes** them onto each champion's skeleton (the same names, so it's a direct copy plus proportion correction), so the shipped pack is self-contained and has no runtime retargeting.

## 8. Pipeline

### 8.1 Layout

```
art/
├─ README.md                 # how to set up Blender 5.2+, the add-on and the MCP server
├─ rigs/biped_v1.blend       # reference skeleton + template mesh, generated by tools/blender
├─ library/biped/*.blend     # shared clips
└─ champions/<id>/
   ├─ concept.md             # silhouette class, palette, personality, clip checklist with status
   ├─ <id>.blend             # source: mesh, rig, every action (pose markers inside)
   └─ export/
      ├─ <id>.glb            # generated, committed
      └─ <id>.anims.ron      # generated markers sidecar, committed
tools/blender/mftr_blender/   # Blender add-on: rig generator, action naming, marker helpers,
                              # export, validation, review renders (also callable headless)
crates/mftr-pack/             # pack format + validator (Rust), shared by server, client and tools
```

`.blend` sources are committed directly (low-poly blends with Blender's compression are ~1–5 MB). There's no Git LFS, which keeps contribution friction low (Q15).

### 8.2 Authoring workflow (hybrid, D49)

1. **Concept** (`concept.md`): silhouette class, palette, personality, and the clip checklist generated from the sim kit.
2. **Block-out** (Claude via the Blender MCP server): the rig from the template, a blocked faceted mesh, and key poses plus timing for every clip, already obeying the markers and the timing contract.
3. **Review** at the gameplay camera (§8.5). The owner or artists comment.
4. **Polish** (human in Blender): arcs, overlap, spacing and personality. The add-on panel keeps the same validate, export and preview buttons.
5. **Export + validate** (`export` → `mftr-tools pack validate`), then a PR with the review sheets attached.

Claude's MCP session can drive all of steps 2–5, and a human can take over any clip at any time, because everything lives in the `.blend` and the conventions.

### 8.3 Export

`blender -b art/champions/<id>/<id>.blend --python tools/blender/run.py -- export --id <id> --kind champion` (or the add-on button) writes:
- **`<id>.glb`**: one skinned mesh plus prop meshes, vertex colors, the material-slot names, every clip (one action per NLA track) sampled at 30 fps, and no images, normals or UVs (shading is faceted from screen-space derivatives). It's deterministic: the same `.blend` gives the same bytes.
- **`<id>.anims.ron`**: per clip, the frame count, loop flag, layer (`full`, `upper`, `additive`), markers and authored stride speed (locomotion). glTF has no markers, so they travel in this sidecar.
- **Key reduction** at export (tolerance 0.1° / 0.5 mm *(start)*), and channels that never leave rest are dropped: **a missing channel means "at rest"** for the runtime. The runtime stores animations in Godot's compressed form.

### 8.4 Validation (CI and add-on)

`mftr-tools pack validate` (the same code the client and server run on packs, [11 §4](11-content-packs-and-mods.md#4-validation)) checks:
- triangle, bone, influence, material-slot and palette caps;
- bone names match the archetype, and the required sockets are present;
- **required clips** exist for the champion's kit (derived from the sim data: effect kinds → §5.3 clips, plus the §5 required set);
- **markers:** `fire` exists where needed and matches the data windup at the reference attack speed within ±1 frame (retiming handles the rest); `hit_n` count and times match the data; one-shots have `end`; locomotion has foot markers;
- no root XY motion beyond 1 cm; loops are seamless (the pose delta between the first and last frame is under tolerance);
- the facing axis (the model faces +Z after export) and ground contact (the lowest foot at idle sits at z ≈ 0);
- the per-champion size budget (§9).

### 8.5 Review renders

The add-on renders, headless, a **contact sheet and a looping GIF per clip** from (a) the gameplay camera (56° pitch, R01) at 1080p scale and (b) a ¾ close-up, plus an ally/enemy accent turntable. These are the PR review artifacts. Nobody should have to open Blender to review timing.

## 9. Budgets

Per champion, within the 1.5 MB of [05 §8](05-art-and-assets.md#8-size-budgets-compressed-per-platform):

| Part | Budget *(start)* |
|---|---|
| Mesh, skin, props | ≤ 200 KB |
| Animations (~40 clips) | ≤ 600 KB |
| VFX parameters | ≤ 50 KB |
| SFX | ≤ 550 KB |
| Icons (SVG) | ≤ 50 KB |
| **Total** | **≤ 1.45 MB** |

Animation estimate: 40 clips × ~1.2 s × 30 fps × ~45 bones ≈ 65k bone keys, ≈ 390 KB at 6 B per quantized rotation key before key reduction, and roughly 150–250 KB after it.

## 10. Pilots: Vesper and Rook (D50)

One ranged and one melee champion prove the rig, the timing contract, the shared library and the pipeline end to end before the other eight follow.

| | **Vesper**: marksman | **Rook**: bruiser |
|---|---|---|
| Silhouette | Slim, hooded, a long recurve bow taller than the shoulders, a tattered cloak tail (`extra_cape` chain) | Stocky `biped_large`, a huge hammer over the shoulder, broad pauldrons |
| Attack | Ranged bow, windup 18% (225 ms at 0.8 AS); the R04 reference shape: draw → crisp release → cancellable follow-through | Melee overhead hammer, windup 28%; the hit lands on `fire` with a ground-crack impact |
| Q | **Longshot** (line): a deep draw, release, recoil follow-through | **Cleave** (nova): a 360° hammer sweep, `fire` at mid-sweep |
| W | **Shrapnel Charge** (delayed area): a lobbed throw toward the point | **Second Wind** (heal): a chest-thump and deep breath, `mobile` upper-body |
| E | **Tumble** (dash): `e_start`, `e_travel` (a roll, in place), `e_land` | **Lunge** (lunge): a crouch launch, airborne travel, a hammer-first landing |
| R | **Snare Net** (hard-CC line): a wide two-hand throw, hard-CC accent | **Shockwave** (line, ult): a two-handed overhead slam, `fire` on contact, a short `hard_lock` |
| Proves | Ranged attack crispness, orb-walk cancel, projectile sockets, cloak secondary motion | Melee impact weight, lunge travel loops, `biped_large` retarget of the shared library |

### Definition of done (per champion)

- [ ] `concept.md` approved (silhouette, palette, personality)
- [ ] Mesh within caps and faceted-style review passed
- [ ] Every §5 clip present, reviewed at the gameplay camera, with markers validated
- [ ] VFX for the attack, every ability, the impact and the crit, from the kit with data-driven sizes
- [ ] SFX for attack, abilities, footsteps, recall, death and emotes
- [ ] In game: facing, retimed attacks at 0.6–2.5 AS, cancel windows, CC poses, death and recall all verified on a client
- [ ] `mftr-tools pack validate` clean; size under budget
