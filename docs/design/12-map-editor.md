# 12 — Map editor

An in-game tool, opened from the main menu, for making lane maps: the official 5v5 map is made
with it (D56). It draws a simplified view of one team's half of the map; the other half is that
half turned half a turn about the map's center, so both teams get the same map, as in the
reference 5v5 map: blue's top-side jungle is red's bottom side. An **Actual view** button shows
the map the game will build: walls as cliffs, the ground, trees, rocks and props.

## 1. What a map is

A map is a small **spec** (`mftr-sim/src/mapspec.rs`), saved as a text file (`*.mftrmap`). The
game builds everything else from it, the same way on the server and on every client.

| Part | Drawn as | Notes |
|---|---|---|
| **Size** | a preset | Small 10,000, Medium 14,500 (the default), Large 16,000 u square |
| **Base** | fixed | Blue's corner (bottom left): fountain, Base, two base turrets; turned for red |
| **Lanes** (1–5) | a line with a width | From blue's base to the river line. Lanes pair up down the river line (top with bottom; a middle lane with itself): each runs on into its partner's half, turned, so paired lanes end at matching spots (the editor snaps them; a middle lane ends at the center). Each gets outer, inner and gatehouse turrets and a Gatehouse, placed along it and draggable along it |
| **River** | lines with widths | Must lie along the river line (top-left to bottom-right corner) |
| **Jungle paths** | lines with widths | Must connect to a lane, the river or the base |
| **Clearings** | circles | Open ground (a camp's clearing is added automatically) |
| **Camps** | points | Each carves a clearing around itself, so it's always reachable |
| **Tall grass** | lines with widths | Brush |
| **Great objectives** | points | In the river. Pairs turn through the map's center (one toward each end of the river, like the reference's Baron and Dragon): 2 or 4. A single one only on a 1-lane map |

Everything not drawn is wall. Building:
1. Lines become capsules on a grid (25 u cells), with their edges gently wavy (deterministic
   noise), so walls look grown rather than cut.
2. The grid is turned onto red's half (half a turn about the center).
3. Walls are traced (marching squares, smoothed, simplified) into the polygons the simulation
   already uses for collision, pathing and sight.

## 2. Rules (validation)

The editor shows every broken rule; a map that breaks one can be saved but not exported.
- 1–5 lanes, each from the base to the river line.
- All walkable ground is one connected area: no jungle cut off from the lanes.
- At least 3 camps on a side, including one of each buff camp; enough jungle (area off the
  lanes, river and base).
- Camps, structures, objectives and spawns stand on open ground.
- Great objectives: as above.

## 3. The editor

- **Canvas:** the half-map from above, red's (turned) half shaded. Zoom and pan.
- **Tools:** lane, jungle path, river, clearing, camp (each kind), tall grass, great objective,
  select and move (points, structures), delete. Line tools: click to add points, double-click
  or Enter to finish, a width slider.
- **Actual view:** the built map as the game draws it (the same code builds a match's world):
  cliffs, roads, the river, trees, rocks, tall grass, fountains, structures and camps, from
  straight above or from the game's camera angle.
- **Files:** new, open, save, export (a validated copy to share).
- **Dev overlay** (development builds, or with `--dev-overlay FILE`): a reference image drawn
  under the canvas, see-through, with handles to align it. The image is the developer's own local
  file and is never shipped.

## 4. Playing a map

`mftr-server --scenario classic --map FILE` plays a map file. The server sends the spec to each
player as they join, and their client builds the same map. The official Classic map is one of
these, built in.
