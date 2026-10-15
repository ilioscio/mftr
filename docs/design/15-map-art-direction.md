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

The structures have room to grow into; as built they use 2,450–3,624 triangles (the faceted look
reads best with big, simple shapes). Scenery is drawn many times, so a rock or a tree stays in the
hundreds, but now has the room for real shapes (a rock ≈ 230–450 triangles, not 80). Small life
is tens to hundreds (a lily pad 34, a frog 260).

## 3. The ground

One shader, four materials (R06 §1): **grass**, **dirt**, **paving**, **water**. Which material is
where comes from a **ground control map**, baked per map when the world is built (the match and the
editor alike), in Rust from the map itself:

| Channel | What | From |
|---|---|---|
| R | dirt | lanes (wide), jungle paths and camps (the walkable ground that isn't lane), fading at the edges |
| G | paving | plinths under every structure, the bases' plazas around the fountains and the base, emblem slabs where lanes meet |
| B | water | the river, by its distance to the river's course |
| A | signed wall distance | 0.5 on a wall's edge; above, how far open ground is from the nearest wall (cliff-foot occlusion, the jungle's gloom); below, how far a cliff's top is from its edge (the grassy lip, the forest's gloom); 10 m full scale either way |

The shader breaks every boundary with noise (grass tongues over dirt, loose slabs off the paving,
ragged banks), adds the grain and detail of each material (cracks in the dirt, joints with grass
in the paving, ripples and foam in the water), and darkens the foot of every wall. Nothing is
limited to a fixed number of segments, so any map works.

## 4. Walls, cliffs and rocks

- Cliffs keep their 1.4 m height and their polygons (the sim's walls). Their faces get stepped
  strata, moss on the ledges and grass running over the top. The tops are the ground's own grass
  (one shared shader include): a lighter lip where it rolls over the drop, the forest's gloom
  deeper in, tufts of grass and bushes along the edge, each set back past its own radius.
- **One outline per cliff** (`mftr_client::cliffs`): the simulation traces walls in tiles, so a
  cliff is several pieces meeting at straight seams. The client merges them (shared edges
  cancel), smooths the merged outline (each point within 0.18 m of the walkable edge) and draws
  one face along it; the tops are the pieces' own, following the face. Only a group of pieces
  that is small and on its own is an outcrop (drawn with rock props). The land past the map's
  edges is rock at the cliffs' height, so a wall along the edge never shows a drop behind it.
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
  (`part_*`, each pivoting at its foot) on the structure's prop; the client computes the fall
  from them (the crystal shatters first, then each part from the top shudders, tips and drops
  with gravity, throwing up a soft, heavy dust cloud and stone chips where it lands), so any
  structure with part bones falls the same way and the props stay clipless. What a fall uncovers
  is modeled: a turret's torn top is broken masonry around a charred floor.

## 6. Minions

Squatter (R06 §6): shorter legs and hood, a wider body, the head about a third of the height; a
little more detail with the larger budget. They stay hooded and faceless, in team color.

## 7. Light and shaders

- **One world lighting model** shared by ground, cliffs and props: a wrapped diffuse with a cool
  tint in the shadowed side, the cloud shadows, and occlusion (vertex occlusion on props, the wall
  distance on the ground).
- **Occlusion** from what the map knows: props' baked vertex occlusion, the ground darkened at
  every wall's foot and in the forest's gloom (the control map's wall distance). (The client's
  Mobile renderer has no screen-space ambient occlusion.) Faces turned from the sun take a little
  of it bounced off the ground, so shaded stone reads warm, not the sky's blue.
- **Local lights:** small flames over every ember bowl and a warm, flickering pool on the ground
  under each; a pool in team color under each structure's crystal (out when it falls); glowing
  spores in the jungle. Found from each prop's own `emissive` and `accent_glow` surfaces. The
  pools are drawn by the ground shader (the 16 nearest the camera's look point), not as lights:
  one big ground mesh would hit the renderer's per-object light limit.
- **A vignette** darkening the screen's corners.

## 8. Life

| Life | Where | How |
|---|---|---|
| Flowers | grass near brush and walls | clusters of small faceted blooms in the season's colors |
| Butterflies | over flower patches | fluttering in loose loops (scattering from units: later) |
| Lily pads | the river's edges | on the water, turning and bobbing slightly |
| Frogs | river banks | sitting, hopping now and then |
| Fireflies or spores | the jungle's darker ground | slow glowing motes |

All placed from a fixed seed from the map's ground control map (`life.gd`), purely visual, never
on a lane, a plinth or a camp's dirt, and not drawn on the minimap.

## 9. Seasons

A season (`summer`, `autumn`, `winter`; `client/scripts/seasons.gd`) is a set of: the ground's four
material palettes, the cliffs' rock and moss, the scenery's tint and prop substitutions (pines →
snowy pines, when such props exist), snow on every prop's upward facets (winter), the life
(autumn keeps lily pads and spores; winter has none), the ambient motes (pollen, falling leaves,
snow), and the light's color and strength. A map names its default season (`season autumn` in
its file, [12](12-map-editor.md); summer without it), picked in the editor, whose Actual view
follows; a player can override it in Settings → Graphics → Season. Looks only: the simulation
never reads it. Adding a season is a new entry in `seasons.gd`, plus any seasonal props.

## 10. Review

Before and after at the gameplay camera on every map (The Bridge, Crossroads, a fresh editor map):
a lane with a turret, a base, the river, a jungle camp; a minion wave; a structure falling; each
season.
