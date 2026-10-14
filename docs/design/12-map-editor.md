# 12 — Map editor

An in-game tool, opened from the main menu, for making lane maps: the official 5v5 map is made
with it (D56), and The Bridge (ARAM) is one of its maps too. It draws a simplified view of one team's half of the map; the other half is that
half turned half a turn about the map's center, so both teams get the same map, as in the
reference 5v5 map: blue's top-side jungle is red's bottom side. An **Actual view** button shows
the map the game will build: walls as cliffs, the ground, trees, rocks and props.

## 1. What a map is

A map is a small **spec** (`mftr-sim/src/mapspec.rs`), saved as a text file (`*.mftrmap`). The
game builds everything else from it, the same way on the server and on every client.

A map is one of two **kinds** (`kind aram` in the file; Classic without it):
- **Classic:** 1–5 lanes, a jungle with camps, great objectives; the base in blue's corner; the
  fountain heals; outer turrets have plates; wards. Crossroads is one.
- **ARAM:** one lane whose start is the base, no jungle camps or great objectives, health
  relics, a fountain that doesn't heal, ARAM's minion pacing. The Bridge is one
  (`maps/bridge.mftrmap`, drawn in its old lane coordinates so its fountain, Base, turrets,
  relics and brush stand where they did).

| Part | Drawn as | Notes |
|---|---|---|
| **Size** | a preset | Classic: Small 10,000, Medium 14,500 (the default), Large 16,000 u square. ARAM: 9,000, 10,607 (The Bridge), 12,000 |
| **Base** | fixed | Classic: blue's corner (bottom left): fountain, Base, two base turrets. ARAM: the lane's start, The Bridge's spacing along it (fountain 300 u in, champions 450, Base 1,250, base turrets side by side at 1,650, minion waves from 1,700) in a round court. Turned for red |
| **Lanes** (1–5) | a line with a width | From blue's base to the river line. Lanes pair up down the river line (top with bottom; a middle lane with itself): each runs on into its partner's half, turned, so paired lanes end at matching spots (the editor snaps them; a middle lane ends at the center). Each gets outer, inner and gatehouse turrets and a Gatehouse, placed along it and draggable along it |
| **River** | lines with widths | Must lie along the river line (top-left to bottom-right corner) |
| **Jungle paths** | lines with widths | Must connect to a lane, the river or the base |
| **Clearings** | circles | Open ground (a camp's clearing is added automatically) |
| **Camps** | points | Each carves a clearing around itself, so it's always reachable |
| **Tall grass** | lines with widths | Brush |
| **Great objectives** | points | In the river. Pairs turn through the map's center (one toward each end of the river, like the reference's Baron and Dragon): 2 or 4. A single one only on a 1-lane map. Classic only |
| **Rocks** | circles | Small walls in the open, for cover. Added as exact octagons, not traced (a traced outline that small would be simplified away) |
| **Health relics** | points | ARAM only. Neutral; each has its turned twin |

Everything not drawn is wall. Building:
1. Lines become capsules on a grid (25 u cells), with their edges gently wavy (deterministic
   noise), so walls look grown rather than cut.
2. The grid is turned onto red's half (half a turn about the center).
3. Walls are traced (marching squares, smoothed, simplified) into the polygons the simulation
   already uses for collision, pathing and sight.

## 2. Rules (validation)

The editor shows every broken rule; a map that breaks one can be saved but not exported.
- The size is one of its kind's.
- Classic: 1–5 lanes, each from the base to the river line. ARAM: exactly one, at least
  4,000 u from its start to the center; no camps or great objectives; relics on blue's half
  and in the open. (The jungle and camp rules below are Classic's.)
- All walkable ground is one connected area: no jungle cut off from the lanes.
- At least 3 camps on a side, including one of each buff camp; enough jungle (area off the
  lanes, river and base).
- Camps, structures, objectives and spawns stand on open ground.
- Great objectives: as above.

## 3. The editor

- **Canvas:** the half-map from above, red's (turned) half shaded. Zoom and pan.
- **Kind:** Classic or ARAM, beside the size; switching moves the size to the kind's nearest
  (everything scales) and shows only the kind's tools.
- **Tools:** lane, jungle path, river, clearing, camp (each kind), tall grass, great objective,
  rock, health relic, select and move (points, structures), delete. Line tools: click to add points, double-click
  or Enter to finish, a width slider.
- **Actual view:** the built map as the game draws it (the same code builds a match's world):
  cliffs, roads, the river, trees, rocks, tall grass, fountains, structures and camps, from
  straight above or from the game's camera angle.
- **Files:** new, open, save, export (a validated copy to share). **Load default map** (at the
  top) opens a built-in map, Crossroads or The Bridge, to edit or start from.
- **Dev overlay** (development builds, or with `--dev-overlay FILE`): a reference image drawn
  under the canvas, see-through, with handles to align it. The image is the developer's own local
  file and is never shipped.

## 4. Playing a map

`mftr-server --map FILE` adds a map file's modes on it: Classic on a Classic map, ARAM,
Mayhem and Hyper on an ARAM map (the server refuses one that breaks a rule). The welcome names it by its text's hash and length; the client asks for the text piece by
piece (each request answered with the next 900 bytes, so a lost packet just gets asked for
again), checks the hash, builds the map with the server's own code and joins. Replays carry the
map. The official maps, Crossroads and The Bridge, are built-in map files.
