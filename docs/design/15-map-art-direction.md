# 15 — Map, Structure and Minion Art Direction

**Status:** Proposed (D60). Measured against the reference map's visual update in
[R06](reference/R06-video-map-visuals.md). Builds on [05](05-art-and-assets.md) (readability,
procedural materials) and sits beside [14](14-champion-art-direction.md) (the champions).

The map is where every match happens, so it has to look finished. It also has to be honest
(05 §1: the ground and scenery are the lowest-contrast layer), it has to work for **every map
the map editor makes** ([12](12-map-editor.md), D56), and it has to take **seasonal variants**.
The style stays ours: faceted low-poly models, procedural materials, no texture files. What makes
it feel modern is the light and the shaders, not more polygons.

## 1. Rules

1. **Everything comes from the map's own data.** Nothing is placed by hand for one map: the ground's
   materials, the plinths, the rocks, the forest, the life and the lights are all derived from a
   map's walls, brush, lanes, river, structures, camps and fountains. A map made in the editor
   looks as finished as an official one, and the editor's Actual view shows exactly that.
2. **Structures grow out of the ground.** A structure stands on a plinth of the ground's own paving
   and is built in the paving's stone; its team color lives in a few accents (crystals, metal
   trim), never in its mass.
3. **Nothing floats.** Props stand on the ground at the height they're placed on; scenery along a
   cliff sits at the foot of its face or set back from its top edge, never across the drop.
4. **Light carries the finish.** Ambient occlusion, cool shadows, a warm key, local lights and
   a vignette give depth at the top-down camera; models stay intentionally low-poly and faceted.
5. **Alive, sparingly.** Small life (butterflies, frogs, lily pads, flowers, fireflies) placed
   from the map's features, a detail every few screens, never on the lane's fighting space.
6. **Seasons are data.** A season is a palette and a set of substitutions, not new code: every
   color the ground, walls, props and life use comes from the active season.

## 2. Budgets (D60: 1.5× for the map, its structures and the minions)

| Asset | Before | Now |
|---|---|---|
| Lane minions | 600–2,600 triangles | **900–3,900** |
| Map props (structures, scenery) | 20–8,000, cap 12,000 | **20–12,000, cap 18,000** |

Structures should use their room (a turret ≈ 4,000–6,000 triangles; the base ≈ 8,000+); scenery
is drawn many times, so a rock or a tree stays in the hundreds, but now has the room for real
shapes (a rock ≈ 300–600 triangles, not 80).

## 3. The ground

One shader, four materials (R06 §1): **grass**, **dirt**, **paving**, **water**. Which material is
where comes from a **ground control map**, baked per map when the world is built (the match and the
editor alike), in Rust from the map itself:

| Channel | What | From |
|---|---|---|
| R | dirt | lanes (wide), jungle paths and camps (the walkable ground that isn't lane), fading at the edges |
| G | paving | plinths under every structure, the bases' plazas around the fountains and the base, emblem slabs where lanes meet |
| B | water | the river, by its distance to the river's course |
| A | wall distance | how far a point is from the nearest wall (cliff-foot occlusion, the jungle's gloom) |

The shader breaks every boundary with noise (grass tongues over dirt, loose slabs off the paving,
ragged banks), adds the grain and detail of each material (cracks in the dirt, joints with grass
in the paving, ripples and foam in the water), and darkens the foot of every wall. Nothing is
limited to a fixed number of segments, so any map works.

## 4. Walls, cliffs and rocks

- Cliffs keep their 1.4 m height and their polygons (the sim's walls). Their faces get stepped
  strata, moss on the ledges and grass running over the top.
- **Rocks:** boulders are placed from each wall's edges *at ground level*, their centers inside
  the wall so they lean into the cliff face and rise up it; on the tops they're set back from the
  edge by their own radius. Rocks are rebuilt with the larger budget: several shapes, chipped
  facets, moss caps.

## 5. Structures and their fall

- **Palette:** the paving's blue-grey stone, worn and mossy; team color in crystals, banners and
  trim only.
- **Plinths:** the control map paves a round plinth under each structure, a ring of slabs wider
  than its footprint; the model's own base is a step on top of it.
- **Destruction:** a structure that falls plays a **collapse**: its upper parts crack, tilt and
  fall in stages, heavy (slow start, fast fall, a bounce of dust), with stone chips and a dust
  cloud on each impact, ending in its rubble on the plinth. The plinth stays. Built as part bones
  on the structure's prop and a baked `collapse` clip.

## 6. Minions

Squatter (R06 §6): shorter legs and hood, a wider body, the head about a third of the height; a
little more detail with the larger budget. They stay hooded and faceless, in team color.

## 7. Light and shaders

- **One world lighting model** shared by ground, cliffs and props: a wrapped diffuse with a cool
  tint in the shadowed side, the cloud shadows, and occlusion (vertex occlusion on props, the wall
  distance on the ground).
- **Screen-space ambient occlusion** for contact shadows under props, structures and units (a
  graphics setting).
- **Local lights:** braziers along the lanes and in the bases (flickering, warm), glowing plants in
  the jungle (cool), the structures' crystals in team color.
- **A vignette** darkening the screen's corners.

## 8. Life

| Life | Where | How |
|---|---|---|
| Flowers | grass near brush and walls | clusters of small faceted blooms in the season's colors |
| Butterflies | over grass near brush, by day | fluttering in loose loops, scattering from units nearby |
| Lily pads and reeds | the river's edges | on the water, bobbing slightly |
| Frogs | river banks | sitting, hopping now and then |
| Fireflies or spores | the jungle's darker ground | slow glowing motes |

All placed from a fixed seed from the map's features, purely visual, never on a lane's center.

## 9. Seasons

A season (`summer` first; `autumn` and `winter` planned) is a set of: the ground's four material
palettes, the cliffs' rock and moss, the scenery's tints and prop substitutions (pines → snowy
pines), the life (butterflies → falling leaves, frogs → none), the ambient particles (pollen →
snow), and the light's color and strength. A map names its default season; the client can override
it (a setting).

## 10. Review

Before and after at the gameplay camera on every map (The Bridge, Crossroads, a fresh editor map):
a lane with a turret, a base, the river, a jungle camp; a minion wave; a structure falling.
