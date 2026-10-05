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

## 4. Champions

- Low-poly meshes, **~5–10k triangles** at max LOD, with 2 LODs. Vertex colors and material-ID regions; no texture maps by default.
- Shading: toon ramp + rim light + team-accent mask region + procedural detail noise in object space.
- Skeletal animation: idle, run, attack ×2–3, cast per ability, death, recall, emotes. Exported as glTF with compressed animation tracks.
- Optional **outline** pass (inverted hull or post-process) to strengthen silhouettes. A user setting.

## 5. VFX

- Mostly **shader-driven meshes**: ribbons, cones, disks and spheres with scrolling noise, dissolves and SDF masks, plus GPU particles for sparks and dust.
- Each ability VFX reads its **gameplay shape from content data** (width, radius, length, duration), so it can't drift from the hitbox.
- A VFX "kit" library (impact, trail, burst, zone edge, ground warning) gives consistent grammar and saves size.

## 6. UI

- SVG icons (imported by Godot as scalable vectors), a single UI theme, procedural panel backgrounds.
- Ability and item icons as SVG: a strong silhouette shape plus an element/color code. Many can share motifs.
- Fonts: open-licensed (e.g. Inter, Noto for i18n), subset per language pack.

## 7. Audio

- **SFX:** a mix of procedurally generated (sfxr-style synthesis for UI and small impacts) and recorded/synthesized samples, compressed as **Ogg Vorbis/Opus**.
- **Gameplay audio cues are part of readability:** hard CC abilities and ultimates have distinct, learnable audio signatures.
- **Voice lines:** optional, downloadable per-language packs, not in the base download.
- **Music:** optional pack (or a small procedural/adaptive layer in base).

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
