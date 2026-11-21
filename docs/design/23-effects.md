# 23 — Effects

How the game draws its effects: fire, lightning, lasers, smears, sparks, smoke and the rest.
Since D79 they're modern: soft, glowing and smooth, made by shaders from noise and shapes, with
no texture files. This replaces the pixel style of D48, which snapped square particles to the
screen's pixels and stepped them "on twos".

The rules effects serve haven't changed ([05 §1](05-art-and-assets.md#1-readability-rules-non-negotiable)):
hitboxes are honest, ability effects are the brightest thing on screen, edges are the team's
color and interiors the champion's, and sizes that matter come from gameplay data. Packs still
only choose kits and tune their knobs, in `<id>.vfx.ron` ([05 §5.1](05-art-and-assets.md#51-the-kit-a4b));
this doc is about how the kit draws them.

## 1. Goals

1. **Look like a modern game.** Light that glows and blooms, fire that licks, lightning that
   forks and flickers, smoke that's lit by the sun, debris that bounces.
2. **Stay small.** No flipbooks and no texture files: every shape is computed in a shader, so
   the whole system ships as two shaders and two scripts.
3. **Stay cheap.** The target is an integrated GPU at 1080p in a ten-champion teamfight.
4. **Be a base to build on.** The same pieces will drive a particle editor in the champion
   creator (§8).

## 2. The pieces

| Piece | File | Role |
|---|---|---|
| The kit | `client/scripts/vfx.gd` | Each kit's pattern (where particles are born, how they move, what they look like), ribbons, projectile trails, lobbed bombs, rains, fires and lightning strikes |
| The simulation | `crates/mftr-gdext/src/particles.rs` (`MftrParticles`) | Every live particle: moves them each frame and packs the two GPU buffers |
| Particles | `client/shaders/vfx_particle.gdshader` | Draws each particle as one of eight procedural shapes |
| Ribbons | `client/shaders/vfx_ribbon.gdshader` | Draws strips: lightning, lasers, swing smears, cracks, columns of light |
| Projectile bodies | `client/shaders/missile.gdshader` | The gameplay-sized body of a skillshot (05 §1.1); its core now glows bright enough to bloom |
| The gallery | `--vfx-gallery` (`main.gd`) | Plays effects over and over for review (§7) |

## 3. Particles

**Drawing.** All particles are camera-facing quads in two `MultiMesh`es: the **covering** layer
(smoke, dust, debris) is drawn first, the **glowing** layer (sparks, flames, flashes) over it.
Both blend with **premultiplied alpha**, so one shader both adds light (alpha 0) and covers
what's behind (alpha 1), and anything in between. Each instance carries its own data in its
transform: position, velocity, size, turn, streak length, shape, seed and depth-fade distance.
Its color and custom data carry the ramp's color, opacity, age, intensity and how additive it is.

**Shapes** (all procedural, no textures):

| # | Shape | Looks like | Used by |
|---|---|---|---|
| 0 | Glow | A soft halo round a hot core | flare heads, motes, embers, projectile heads |
| 1 | Spark | A capsule stretched along its motion on screen, white-hot in the middle | bursts, flares, rain, vortex streaks |
| 2 | Flame | A tall teardrop eaten away by rising noise, white-yellow at heart | `fire`, `flame` trails |
| 3 | Smoke | A ragged puff, lit from the sun's direction | dust, smoke, landings, fires' smoke |
| 4 | Shard | A chipped, faceted crystal or stone, lit, with a glint | rocks, shards, debris from a dark `burst`, bones |
| 5 | Star | Four rays and a hot core | flashes, lightning strikes |
| 6 | Ring | Flat on the ground: a shock front with a bright edge and faint wake | `shock`, `ring`, strikes |
| 7 | Disc | Flat on the ground: a pool of light, or a scorch | flashes, fires, pillars, strikes |

**Light.** Ramp colors are written in sRGB (as players and artists pick them) and converted to
linear light before drawing; without this every orange came out peach. A particle's
**intensity** starts above 1 when it's hot (sparks 2.4, stars 2.6) and eases to 1 as it ages, so
the scene's bloom catches what's hot and lets go as it cools. The first ramp color is the hottest.

**Softness.** Particles fade where they meet the ground or a body, over a few tenths of a metre,
by comparing their depth with the scene's (soft particles). Without it, every glow near the ground
showed a hard horizontal cut.

**Noise** is value noise over an integer hash, exact at any coordinate. A floating-point hash
showed seams, rectangles inside the flames.

**Motion.** At the frame rate, not stepped: gravity (negative floats things up), drag, spin and
size growth eased out. Flags let a particle **bounce** off the ground, **splat** (die where it lands,
like an arrow), raise a **puff** of dust where it first lands, or **twinkle**.

**Cost.** The simulation is Rust (`MftrParticles`); the kit in GDScript only decides what to emit.
In the gallery's stress test (8 big effects every second, about 1,100 live particles and 7
ribbons), the whole kit took about **1.0–1.2 ms a frame**, in a debug build. The same simulation
in GDScript took 2.3–3.5 ms. The GPU draws the particles in two draw calls and the ribbons in one.

## 4. Ribbons

Strips rebuilt every frame into one `ImmediateMesh`. A ribbon is either **camera-facing** (a spine
of points, widened across the view in the shader) or **flat** (both edges given: on the ground,
or in a swing's plane).

| Style | What | How it moves |
|---|---|---|
| Lightning | A white-hot thread in a colored glow, forked | Re-drawn every 45 ms (midpoint displacement, three forks), flickering, for 0.15–0.6 s |
| Laser | A steady white core, a colored glow pulsing down it | Follows a `beam` projectile from where it was first seen to its head (at most 7 m), inside the body's width; fades in 0.15 s when the projectile ends |
| Smear | A swing's band, brightest at the blade's edge, streaked | Revealed from the swing's start to its end in a blink, eaten from the tail as it fades; an overhead blow comes down from over the shoulder |
| Crack | A dark split in the ground; a bright ramp burns in its core | Spreads out in the first 15 % of its life, lingers, fades |
| Column | A soft column of light, streaks rising inside | Rises in the first 12 %, fades |

## 5. The kit's effects

Every kit of [05 §5.1](05-art-and-assets.md#51-the-kit-a4b) keeps its name, phases, knobs and
honest sizes; only how it draws changed. Two kits are new:

- **`fire`**: a fire burning across the area for its lifetime (up to 4 s, like `rain`): flames
  licking up off the ground, taller in the middle, with embers flying and smoke over it, on a
  glowing patch of ground; fiercest at first, dying down at the end. `count` is the flames every
  tenth of a second. Ember's Cinder Bloom now erupts in one.
- **`lightning`**: `count` bolts from the sky onto points across the area, the first at once and
  the rest over the next half of its lifetime, each with a star, a flash on the ground, a small
  shock ring, sparks, a wisp of smoke and a scorch that lingers. The creator's new move **Storm
  Call** (`lib.storm_call`) calls three.

And projectiles got their own: a **`beam`** is a laser down its length, a **`flame`** sheds licks
of fire that rise as they fall behind, and every projectile has a soft glow riding its head,
never wider than its body.

**Hot or not.** A `burst` whose ramp starts bright throws sparks; one whose ramp starts dark
(stone chips, soot) throws tumbling debris that bounces. A `crack` in a bright ramp glows in its
core; in a dark one it's just a split.

**Caps** (05 §5.1): 2,048 live particles and 64 ribbons client-wide; past them, new ones are
dropped.

## 6. What doesn't change

- **Telegraphs and hitbox sheaths** stay exact and full-rate, in the team's colors (05 §1).
- **Readability first:** effects are tuned at the gameplay camera, not up close. Interiors may
  glow; edges stay crisp.
- **No textures** (05 §2's texture policy): shapes come from math, not images.

## 7. Reviewing effects: the gallery

```
godot --path client -- --vfx-gallery "ember:w.detonate@1.6,lumen:e.projectile@0.55,kit:lightning#215@1.6" --zoom 1.6 --shot out.png --shot-at 2.95
```

Each entry is a station on the ground, under the game's light and camera: a champion's event
(`champion:action.phase`: its particle effects, or for `projectile` its missile flying across), or
a kit (`kit:<name>` in the ramp of a hue, `#degrees`). `@metres` is the area's radius (for a
projectile, its hitbox radius). All stations fire together every `--vfx-every` seconds (2.5),
starting at 0.3 s, `--vfx-gap` metres apart (4.6). `--lineup` can stand champions beside them, and
the usual `--shot`, `--shot-at` and `--shot-seq` capture it. Each cycle it prints the busiest
moment: live particles, ribbons and the kit's milliseconds per frame.

## 8. Next

In rough order:

1. **Look knobs in `vfx.ron`:** let an effect choose its shape, glow and softness, not only its
   kit. This is the data a particle editor writes.
2. **A particle editor in the champion creator** ([20 §8.3](20-champion-creator.md#83-effects-and-sounds)):
   compose an effect from emitters (where particles are born, how they move, how they look over
   their life), with a live preview at the gameplay camera and the caps and honest sizes enforced
   as you go. Effects saved in the recipe like everything else.
3. **Telegraph interiors** (`frost`, `flames`, `sigil`…) in the same light: they're still flat.
4. **Better fire:** more variety in the licks, a hotter core at the base, heat shimmer (needs the
   screen texture: measure its cost on the Mobile renderer first).
5. **Debris as meshes** (chipped stones, bones) instead of flat shards.
6. **Quality settings:** fewer, larger particles on weak machines.
7. **Emission in Rust** too, if a teamfight's GDScript emission ever shows in a profile.
