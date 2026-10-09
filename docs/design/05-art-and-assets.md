# 05 — Art & Assets

**Direction:** stylized low-poly models, flat or cel-shaded, with **materials generated procedurally in shaders** from noise, gradients and geometry data. Bitmap textures are the exception and need justification.

The style serves two goals, in this order:
1. **Readability:** gameplay must be legible at a glance in a 10-champion teamfight.
2. **Size:** the whole game downloads in under 150 MB.

## 1. Readability rules (non-negotiable)

These rules apply to every asset, including future skins and mods.

1. **Hitbox honesty.** A projectile's or area's visual extent matches its gameplay shape (±5%). Per D14 this may be a slim bright core plus a faint, always-visible full-width edge sheath (the reference game's core is only ~half its hitbox, see [R01 §4](reference/R01-video-ezreal-flash-barrier-q.md#4-mystic-shot-reference-line-skillshot)). Trails, sparks and glow may extend *behind* a projectile but never *ahead* of it or *wider* than it. The ability's `width` in data drives the VFX scale. Artists do not set it by eye.
2. **Value hierarchy.** Ground and environment are lowest contrast and slightly desaturated. Units sit in the middle. **Ability VFX and telegraphs are the highest contrast on screen.**
3. **Team color language.** Enemy threats carry a consistent **enemy accent** (default: warm red/orange rim or edge), and allied ones the **ally accent** (default: cool blue/teal). It applies to projectile edges, ground indicators and health bars. Ships with colorblind presets (deuteranopia, protanopia, tritanopia) and custom colors.
4. **Telegraph grammar.**
   - *Delayed ground AoE:* outline appears at cast start, and the inside fills toward detonation, so you can read how long you have.
   - *Linear skillshot:* a distinct leading edge, a constant-width body, and a trail behind.
   - *Zones:* persistent edge shimmer, with an animated interior that shows the tick rhythm.
   - *Hard CC* abilities share a recognizable accent shape (e.g. a chevron or ring motif), so "this one stuns" is learnable across champions.
5. **Silhouettes.** Each champion is identifiable from its silhouette alone at default zoom. Skins may never change the silhouette class, hitbox, telegraph shapes or audio cues.
6. **Clutter control.** Allied VFX opacity can be reduced (setting), and enemy VFX always render at full strength. Particle counts are capped per ability.

## 2. Procedural materials

| Technique | Use |
|---|---|
| **Noise in shaders** (simplex, value, Worley/cellular, fBm, domain warp) | Grass, stone, dirt, water, bark, fabric, metal wear |
| **Triplanar mapping** | Terrain and props with no UV texture work |
| **Gradient ramps** (tiny 256×1 textures or uniforms) | Toon shading bands, color palettes per material |
| **Vertex colors and baked AO/curvature in vertices** | Edge highlights, cavity darkening on champions and props |
| **SDF shapes in shaders** | Ability indicators, VFX shapes, UI elements |
| **Godot `NoiseTexture2D` / `FastNoiseLite` resources** | Stored as a handful of parameters, generated at load and cached to disk on first run |

**Tools:** [Material Maker](https://www.materialmaker.org/) (open source and Godot-based) for authoring procedural materials that export as shaders, and Blender for modeling and animation. Everything in the pipeline is open source.

### Texture policy
Allowed without review: gradient ramps (≤ 256×4), LUTs (≤ 32³), font atlases, SDF icon/glyph atlases, the few UI images that must be bitmaps. Anything else needs a **texture budget review** in the PR, with the size impact reported by CI.

## 3. Map construction

- Authored as **vector data** in `maps-src/`: lane splines, wall and brush polygons, river path, height control points.
- A bake step generates: terrain mesh (height = SDF falloff from walls + low-frequency noise), wall meshes (extruded polygons with procedural rock detail), navmesh and vision grid ([04](04-architecture.md)), and prop scatter points (seeded).
- Visual detail comes from shaders, so the shipped map is mostly **meshes + parameters**. Target: ≤ 8 MB for the main map.
- Elemental-wyrm terrain changes are shader parameter sets plus a few prop variants, not new textures.
- **As built on The Bridge:** the structures, landmarks and scenery are map props (kind `prop`,
  [11 §3](11-content-packs-and-mods.md#3-what-a-pack-may-contain)), built by scripts in `art/props`
  with `mftr_blender.prop` in the champions' faceted style (every face its own shade, the team
  color on `accent`). Turrets are round stone towers with team-colored roofs, the Gatehouse a gate
  between two towers, the Base a crystal shrine; the fountain is a platform, relics float over
  pads, fallen structures leave rubble. The client scatters the scenery from a fixed seed in
  batches (pines on the cliffs and past the map, rocks on the cliff edges, tall grass in the
  brush), never close to the lane on the camera's side. The ground shader draws grass with
  chunky texels, a dirt road with ruts, pebbles and a worn cobbled spine; cliffs have strata,
  moss and grassy tops, and the land past a lane map's edges is raised to them.
- **Light, as built:** a warm late-afternoon sun with soft shadows over the view, a cool sky
  fill (shade reads blue, not grey), a filmic tonemap with a gentle grade, bloom on what glows,
  and haze only toward the far edge of the view. Cloud shadows drift across the map: a shared
  shader include (`clouds.gdshaderinc`) dims the sun, not the fill, on terrain, props and
  champions alike, procedurally (no textures). Pollen motes float in the light near the camera.
  Shadows, bloom, and clouds with motes can each be turned off in Settings → Graphics.

## 4. Champions

Full specification: **[10 — Characters & Animation](10-characters-and-animation.md)**. In short:

- "Faceted PS1" low-poly meshes, **2,500–4,000 triangles** (cap 6,000), with vertex colors and ≤ 4 material slots including the team accent; no texture maps (D48).
- Shading: faceted toon ramp + rim light + team-accent region + procedural detail noise in object space.
- Skeletal animation on a shared rig standard, with the **complete animation set** per champion (~35–45 clips: locomotion, attacks, every ability phase, CC poses, recall, death, emotes), timed by the sim's action contract. Exported as glTF plus a marker sidecar, and shipped inside content packs ([11](11-content-packs-and-mods.md)).
- Optional **outline** pass (inverted hull or post-process) to strengthen silhouettes. A user setting.

## 5. VFX

- Mostly **shader-driven meshes**: ribbons, cones, disks and spheres with scrolling noise, dissolves and SDF masks, plus GPU particles for sparks and dust.
- **Pixel style** (D48): particles snap to a screen-pixel grid with nearest filtering and short color ramps, and animate on twos. Gameplay edges and telegraphs stay smooth and exact.
- Each ability VFX reads its **gameplay shape from content data** (width, radius, length, duration), so it can't drift from the hitbox.
- A VFX "kit" library (impact, trail, burst, zone edge, ground warning) gives consistent grammar and saves size.

### 5.1 The kit (A4b)
The client's built-in kit (`client/scripts/vfx.gd`, `client/shaders/pixel_vfx.gdshader`). Packs only pick kits and tune them, in `<id>.vfx.ron` ([11 §3.1](11-content-packs-and-mods.md#31-vfx-files)).

| Kit | Phase | What it draws |
|---|---|---|
| `flare` | `release` | A spark at the projectile socket and a spray along the shot |
| `burst` | `impact`, `detonate`, `fire` | Sparks that fly out and fall |
| `ring` | `impact`, `detonate`, `fire` | Pixels on the area's edge, at the **radius from gameplay data**, drifting outward |
| `dust` | `start`, `land`, `detonate` | Slow, growing ground puffs |
| `trail` | `projectile` | Sparks shed behind the projectile |
| `arrow`, `net`, `orb` | `projectile` | A style *inside* the projectile's gameplay body (a shaft, a spinning frame, nothing) plus a trail behind it |
| `lob` | `projectile` (areas) | A bomb that arcs from the socket to the area's centre and sizzles until it detonates |

- **Pixel style:** all particles share one MultiMesh of camera-facing squares whose corners snap to a grid of screen pixels (2 px at 1080p). They step at **20 Hz** ("on twos") and walk their 2–6 color ramp in whole steps, with no blending.
- **Honest sizes:** projectile kits can't set a size. Shafts and frames are fitted inside the body's gameplay width, and trails only fall *behind* it (§1.1). Rings take the area's radius from the render state.
- **`fire`** (A6): when an action's animation passes its `fire` marker. It is centered and sized for novas, at reach in front for melee blows and slams, and at the chest otherwise; any particle kit plays there (10 §6).
- **Events:** the client fires `<action>.<phase>`. It finds the action from the kit: an area's or missile's radius is matched against the owner's abilities (augment-widened too), bolts are `attack`, and dashes are the kit's dash slot. A champion's own `action.phase` wins, then its `*.phase`, then the shared library's (`biped_library.vfx.ron`).
- **Caps:** ≤ 48 particles per effect, ≤ 4 effects per event, ≤ 64 effects per pack, lifetimes ≤ 1.5 s, and 2,048 live particles client-wide.

## 6. UI

- SVG icons (imported by Godot as scalable vectors), a single UI theme, procedural panel backgrounds.
- Ability and item icons as SVG: a strong silhouette shape plus an element/color code. Many can share motifs.
  **As built:** item icons are `art/items/icons/<name>.svg`, drawn on a backdrop in their tier's
  colors (components slate, upgrades teal, legendaries gold on purple); an item without one gets
  its main stat's glyph. Planned: icons rendered from item models.
- **Champion portraits** are rendered by the client from each champion's own model (its idle
  pose, a warm key and cool rim light, a backdrop in its identity color), so they follow every
  change to a model: a bust, a round crop, a minimap icon and a full-body card. They're used in
  champion select (styled after the reference game's ARAM select: our team as cards, our
  champion large, the enemy hidden), on the loading screen that reveals both teams, the HUD and
  the minimap.
- Fonts: open-licensed (e.g. Inter, Noto for i18n), subset per language pack.
- **Minimap** (bottom right; shown and sized in Settings): the map painted by one top-down render
  of its ground, cliffs and scenery, the fog of war over it, icons for champions (their initial
  on their identity color, ringed by team), minions, structures and relics, and the camera's view
  on the ground. Left click or drag looks there while held; right click moves.
- **Fog of war** in the world as on the minimap: what the team doesn't see is darkened and cooled
  (the client computes its team's vision with the sim's rules; units in it are never sent). The
  vision grid stores how far inside each vision circle a cell lies, so its edges are smooth
  curves rather than the cells' stair steps.
- **As built (the theme):** ink panels with a thin gold rim, slate buttons that warm to gold on
  hover, one gold primary button per screen, recessed fields; one theme for every menu and
  panel (`ui_theme.gd`), the same colors in the HUD (`hud.gd`). Menus sit on a procedural dusk
  backdrop with the logo; champion select shows each team's champions as cards.
- **The HUD:** one bottom panel scaled with the window: stats, a portrait with the level and an
  XP ring, ability icons (a glyph per kind of effect on the champion's color) with cooldown
  sweeps, key tabs, rank pips and level-up tabs, a segmented health bar, items and gold. Hovering
  an ability shows its tooltip, generated from the sim's data: what it does, the numbers at the
  current rank with their AD/AP scaling (colored by damage type; status effects with icons),
  every rank, and what the next rank changes. The top right has K/D, the clock, fps and ping;
  the full net graph is on F1.
- **Windows** (the augment draft, the shop, the anvil, the match breakdown, the death recap,
  settings) are dragged by any spot that isn't a button and stay where they were left (saved as
  fractions of the screen). Tooltips draw above them all: items (icon, cost, stats colored by
  stat, passive, active) and augments (on hover above the HUD) as well as abilities.
- **The cursor** is a steel gauntlet drawn as SVG, pointing like the classic hand cursor: the
  index finger up, the other three curled beside it, the thumb wrapped around the front: red plates over an enemy that can be attacked, teal over an ally, a red
  reticle while an attack-move waits for its click.
- **Augment icons** are generated: a glyph for the augment's mechanic (or a stat augment's main
  stat) on a backdrop in its tier's colors. They show in the draft, above the HUD, in the
  breakdown and in tooltips.
- **Over units:** health bars with a frame, 100-health ticks, a level box, a draining damage
  chip; champions' names, and icons for stun, root and slow. Damage numbers show only what
  concerns the player (dealt, taken, healed), colored by damage type, sized by the hit.

## 7. Audio

- **SFX:** a mix of procedurally generated (sfxr-style synthesis for UI and small impacts) and recorded/synthesized samples, compressed as **Ogg Vorbis/Opus**.
- **Gameplay audio cues are part of readability:** hard CC abilities and ultimates have distinct, learnable audio signatures.
- **Voice lines:** optional, downloadable per-language packs, not in the base download.
- **Music:** optional pack (or a small procedural/adaptive layer in base).

### 7.1 The SFX pipeline (A4c)

**Authoring.** First-party sounds are synthesized.
- A hand-written `sounds.ron` recipe sits next to each `.blend` and is the source of truth.
- `mftr-tools sfx build` renders each sound from layered voices: sine, triangle, square, saw, noise and NES-style "metal".
- Each voice has an exponential pitch slide, an attack/sustain/decay envelope with punch, vibrato, a swept low-pass, a high-pass and drive.
- Each sound can add an echo and a PS1-era crunch (bit depth and sample-and-hold).
- Output is mono at 22.05 kHz, in Ogg Vorbis (aoTuV, quality 0.3, a fixed stream serial), so the same recipe always gives the same bytes.
- The build writes `export/<id>.sfx.ron` (event bindings plus the recipe's hash) and `export/sfx/*.ogg`.
- A community pack may ship recorded Ogg files and a hand-written binding file instead.

**Events.**
- The VFX events (`<action>.<phase>`, §5.1), plus `cast`: a windup starting.
- `unit.foot`, from the walk and run clips' `foot_l`/`foot_r` markers, never while dashing.
- `unit.death` and `unit.respawn`.
- `unit.recall` and `unit.emote`. These are authored but have no in-game trigger yet.
- `cc.hard`: the **shared hard-CC accent**, a clang over a low gong. It plays on top of any hard-CC hit or detonation, the same for every champion, so lockdown has one learnable sound.
- `match.victory` and `match.defeat`: a fanfare when a Base falls, from the shared library, played flat (not placed in the world).

**Variants.** Several sounds on one event are variants. One plays at random, never the same twice in a row, with the sound's random pitch spread. A champion's own `action.phase` wins, then its `*.phase`, then the shared library's (`biped_library.sfx.ron`).

**Playback** (`client/scripts/sfx.gd`).
- `mftr-pack` decodes the Ogg files with lewton under the caps. Godot receives 16-bit PCM as `AudioStreamWAV`, so pack bytes never reach it.
- A pool of 24 positional voices plays them, stealing the oldest when all are busy.
- The listener sits 6 m above the point the camera looks at, so units on screen sound near and off-screen ones fade.
- Attenuation is inverse distance, with a 9 m reference, a 40 m cutoff and no distance muffling.

**Caps** (11 §3.2):
- ≤ 3 s per sound; mono or stereo; 8–48 kHz;
- ≤ 128 KB per file;
- ≤ 4 variants per event and ≤ 48 sounds;
- ≤ 550 KB in total (10 §9).

## 8. Size budgets (compressed, per platform)

| Component | Budget |
|---|---|
| Engine + `mftr-gdext` + client scripts (custom Godot export template with unused modules stripped) | ≤ 40 MB |
| ARAM map | ≤ 4 MB |
| Main map | ≤ 8 MB |
| Per champion (meshes, anims, VFX, SFX, icons) | ≤ 1.5 MB |
| UI, fonts (base language) | ≤ 5 MB |
| **v1.0 total with 40 champions** | **≤ 120 MB** (150 MB hard cap) |
| Optional: music pack, voice packs, HD outlines, etc. | separate downloads |

CI produces a **size report per PR**, and the build fails if a budget is exceeded without an approved exception.

## 9. Performance targets

- 1080p / 60 fps on integrated graphics in a 10-champion teamfight. Stretch goal: 144 fps on a mid-range discrete GPU.
- Godot **Mobile** renderer (Vulkan) as default for its cost/quality balance, Forward+ optional, and **Compatibility** (OpenGL 3.3) as fallback for old hardware. All shaders must have Compatibility-safe variants.
- Bounded draw calls: instancing for minions and props, merged static map geometry.
- No shader compilation hitches during a match (pipeline warm-up at load, see [04](04-architecture.md#5-client-godot-4)).
