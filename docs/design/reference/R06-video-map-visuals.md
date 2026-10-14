# R06 — Reference capture analysis: the reference 5v5 map's visual update

**Source:** a local copy of the reference studio's showcase of its 5v5 map's visual update,
*summ rift visuals* (305 s, 9,157 frames, 1280×720 at 30 fps): fly-overs of the bases and the
pits, a lane laning phase, the jungle camps, the river and fights at the gameplay camera. Picked
by the owner as the bar for the map, its structures and its minions.

**Method:** contact sheets every 5 s (150 frames) of the whole video to map its segments, then
full frames for the base (f300, f5550, f5700), a lane emblem (f6000), a jungle camp (f4950) and a
pit (f6750). The point is what makes it read, not its details: we keep our own faceted style.

## 1. The ground is a painting, in four materials

- **Grass:** two or three greens in large soft patches (light where the sun falls, darker toward
  the jungle), with scattered specks of flowers (purple, blue, yellow) in small clusters, never
  evenly sprinkled.
- **Dirt:** the lanes and jungle paths, warm brown, with thin dark crack lines and darker
  wheel-worn bands; edges are ragged and broken by grass tongues, never a clean line.
- **Paving:** cool blue-grey flagstones in the bases, under every structure and at the lanes'
  emblems: big irregular slabs with dark joints, worn rounder at the edges, **grass growing in the
  joints** and over the outer slabs, loose slabs scattered off into the grass and dirt.
- **Water:** the river and the pits: teal, darker and bluer in the middle, lighter and greener at
  the banks, with soft bright ripple bands, foam where it meets rock, and wet dark mud at its
  edge.

Each transition is irregular and layered: dirt under grass tongues, paving under dirt, the
paving's edge stones loose.

## 2. Structures stand on the ground's own stone

- Every turret, the inhibitors and the nexus stand on a **round plinth of the same paving** as
  the base floor: a ring of curved slabs, a step, then the structure. The plinth is part of the
  ground, so the structure grows out of the map rather than sitting on it.
- The structures' stone is **the same blue-grey** as the paving, with the same wear: chipped
  edges, moss in the crevices, grass at the foot. Their color is in a few accents (team-colored
  crystals and metal), not the mass.
- Braziers on stone posts line the lanes and the bases, warm fire against the cool stone: points
  of light and life.
- Fallen structures leave rubble on their plinth (the plinth stays).

## 3. Walls are raised land, not boxes

- The jungle's walls read as **mounds**: grass and moss running up and over, dense pines and broad
  trees on top, roots and big rocks at the foot. Where a wall meets the lane it's a rock face,
  stepped and broken, with moss on every ledge.
- **Rocks sit on the ground.** Boulders are half buried at the foot of the walls, leaning into
  them, with a dark contact shadow; slabs lie flat on the wall tops. Nothing hangs over a drop.
- A soft dark band (occlusion) runs along the foot of every wall, and the ground under the
  canopy is darker and bluer: the jungle is a darker place than the lanes.

## 4. Light makes the low detail feel finished

- A warm key light from the upper left; shadows are cool and blue, never black.
- Strong ambient occlusion: dark contact shadows under rocks, at the foot of structures and walls,
  inside camps; this is most of what gives the scene depth at the top-down camera.
- Local lights: braziers glow orange on the stone around them; blue-glowing plants and crystals
  light the jungle; the pits are lit cool.
- A gentle vignette: the screen's corners and the unwalked jungle are darker, the lane and the
  fight brighter.

## 5. It's alive

- Tall grass (brush) sways; flowers and small glowing plants dot the jungle; lily pads, reeds and
  frogs in and by the river; butterflies and small birds near the brush; fireflies or spores in
  the dark jungle. Never many at once: a detail every few screens.
- The water moves, the braziers flicker, the plants glow and pulse.

## 6. Minions are small and squat

- Lane minions are short and wide (head ≈ ⅓ of the height, short legs), about a third of a
  champion's height, and read as a wave of team color rather than individuals.

## 7. What we take

| Reference | Ours |
|---|---|
| Painted ground in four materials with layered, ragged transitions | A baked ground control map per map (dirt, paving, water, wall distance) read by one ground shader with noise-broken edges: works for every map the editor makes |
| Structures on plinths of the ground's paving, in the paving's stone | Plinths painted by the control map under every structure; structures built in the paving's palette, team color only in accents |
| Rocks on the ground at the walls' foot, contact shadows | Rocks placed from the walls' faces at ground level, sunk into the cliff, never over a drop |
| Strong occlusion, cool shadows, local warm lights, vignette | SSAO, a cool fill, wall-foot occlusion in the ground shader, brazier and crystal lights, a vignette |
| Small life in places | Butterflies, frogs, lily pads, flowers, fireflies: few, placed from the map's own features |
| Squat minions | Shorter legs and hood, wider bodies |
