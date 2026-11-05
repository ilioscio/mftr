# 19 — Shopkeepers

**Status:** Accepted (D67); the four keepers and their stalls are built and in game (K slices
1–4). Each base gets a shop you can see and click: a shopkeeper at a stall beside the fountain. Clicking your own opens the shop, so a new player finds it by looking, not by
knowing the G key. Four shopkeepers to start, a random pair each match.

## 1. Why

- **Onboarding:** the reference game puts its shop behind a key and an icon in a corner. A player
  who has never played expects to walk up to a shop. A stall with someone behind it says "buy
  here" without a word.
- **Life in the base:** the base is where every match starts and every death returns you. A
  character who fidgets, greets you and reacts when you buy gives it a host.
- **Identity:** four characters with names and habits are cheap to love, and new ones later are
  the kind of content packs can add (11).

## 2. Rules

1. **Minion kin, but adorned and affluent.** Shopkeepers keep the minions' family traits: the
   hood, no face but a shadowed void with two eyes glowing in the team color. Everything else is
   their own: their own proportions, clothes, props and silhouette. Never a scaled-up minion.
2. **A silhouette each** (14 §2), nameable in black from the gameplay camera: the lineup (§6).
3. **A signature hue each** (14 §4), away from the team colors (ally teal ~175°, enemy red ~8°)
   and from each other: gold (~45°), acid green (~85°), violet (~270°), rose (~325°). The
   signature lives in the character and its stall; the team color does not.
4. **Team color in accents only** (15 §2), by the viewer's ally and enemy rule like every
   structure: the eyes (`accent_glow`), the stall's awning or banner and one lantern or crystal
   (`accent`). A future colorblind mode recolors them with everything else.
5. **The stall grows out of the base** (15 §2): it stands on a plinth of the ground's paving,
   built in the paving's stone, with the same worn edge and darkened foot as the turrets' plinths.
   Nothing floats; the stall's legs, crates and sacks sit on the plinth. The **keeper stands at
   the plinth's front, in the open**: the gameplay camera looks down at 56°, so an awning over
   the keeper hides them. Awnings and canopies cover only the goods behind.
6. **Seasons** (15 §9): snow on every upward facet in winter comes from the prop material. On
   top of that each stall swaps dressing by season (§5), and each keeper wears a winter
   accessory (Goldwick's fur collar turned up, Rosette's earmuffs and muff, Slink's chunky
   knitted scarf, Folio's knitted cap with a bobble on his hood's tip) on `extra_winter` bones
   the client shrinks away in other seasons. The season can change mid-match (15 §9).
7. **More animated than minions.** Minions march; shopkeepers perform. Every keeper has a
   breathing idle with secondary motion (a hat, a bow, a coat tail, a pack) and three fidgets
   that say who they are, plus reactions (§4).
8. **Budget:** a keeper is 3,000–4,500 triangles (cap 4,500: the minions' 3,900 cap + 15%). A
   stall is a map prop: 4,000–9,000 triangles (props' cap is 18,000), including the plinth.

## 3. Rigs and packs

- **Keepers** stand on the `biped` bones placed where each keeper's joints are (a table of its
  own, `art/shopkeepers/build.py`), so the validator, exporter and review tools apply, plus
  `extra_<name>_1` bones for what moves on its own: a hat, a bow, a coat tail and its flap, a
  beard, a pack's tiers, a lantern, and the props in their hands (a monocle, a watch, a mirror, a
  dagger, a scroll). Kind `shopkeeper`, `art/shopkeepers/<key>/export/<key>.glb`; the validator
  knows the four ids, the clips and the budget.
- **The body is the approved block-out** (`concepts.py`), rounded off, each color carrying its
  material slot (the team color white, so `accent` takes the team's).
- **Motion is code** (`motion.py`), shared by the animated previews and the packs: poses as
  functions of time, arms on two-bone IK reaching for targets, feet planted with IK, and springs
  for everything that lags and settles. The build keys every frame; the idle loops exactly.
- **Stalls** are `prop` packs in two layers the client stacks: `stall_<key>` (the stall on its
  plinth, every season) and `stall_<key>_<season>` (summer, autumn, winter: what it wears,
  standing on the plinth's top, 0.18 m up). `art/shopkeepers/stalls.py` builds them into
  `art/props/`.
- **Clips:** `idle`, `idle_fidget_1`, `idle_fidget_2`, `idle_fidget_3`, `greet` (the shop opens
  while near), `purchase` (the viewer buys something), `farewell` (the shop closes), `victory`,
  `defeat` (held on its last frame). No sim timing: shopkeepers are client-only.

## 4. Behavior (client only)

The simulation never knows shopkeepers exist. The client:

- **places** each stall where the reference game has its shop: on the rim of the fountain's
  platform, above it and a little to the left on screen (the camera is the same for both teams),
  set into the wall behind it, its keeper facing the platform. From the map's data (the
  fountain and the walls), so editor maps get shops too: directions are tried from about 22°
  left of straight up (between the platform's flag and brazier), then about 65°, until one meets
  a wall 5.5–8.5 m away; the cliff's mesh is bent back around the stall into an alcove (the
  simulation's walls don't change: a Classic base's wall is the map's edge) and the scenery kept
  out of it. With no wall in reach (an ARAM court), the stall stands just past the platform.
  Horde maps have one fountain and one shop;
- **picks the pair** from the match itself: a hash of its champions and their unit ids (the Tab
  scoreboard, the same for every player), blue's keeper and red's, never the same one. No wire
  change. `--shopkeepers A[,B]` picks your base's keeper (and the enemy's) instead, for reviews;
- **opens the shop** when the player left-clicks their own keeper or stall (the cursor shows a
  coin bag over it); a right click still walks there. Clicking the enemy's shows its name and a
  line of banter;
- **plays reactions:** `greet` when the shop opens near the keeper, `purchase` on the viewer's own
  purchases, `farewell` when it closes, `victory` or `defeat` when a Base falls. Fidgets play
  every 8–15 s, never twice in a row. Clips crossfade (`MftrAnimator.pose_mix`);
- **turns** the keeper's head toward the viewer's champion when it's near.

## 5. The four

| | Goldwick | Madame Rosette | Slink | Old Folio |
|---|---|---|---|---|
| Is | The rich merchant | The perfumer and charmer | The black marketeer | The ancient archivist |
| Shape | A round coin purse on short legs under a tall stovepipe hood, a cane | A bell: a wide hooped gown, a huge bow on the hood, a parasol | Tall, thin and stooped, a long coat, a wide slouch brim | Tiny, under a backpack tower of scrolls twice his height |
| Signature | Gold | Rose | Acid green | Violet |
| Stall | A counting house: an oak counter, brass scales, coin stacks and sacks, a strongbox, a ledger | A perfumery: tiered shelves of bottles, a heart mirror, ribbons, a fainting couch | A cart under a half-pulled tarp: crates, vials, daggers, a caged something | A scriptorium: a desk of scrolls, stacked tomes, a globe, an hourglass, the Astrolabe |
| Summer | Bunting and a potted lemon tree | Roses in a trellis | A fruit crate as cover | Open windows of drying pages |
| Autumn | Gourds and a harvest wreath | Dried flowers, a shawl | Leaves on the tarp, a smoking brazier | Candles and fallen leaves on the desk |
| Winter | A fur collar, a steaming mug | A muff and earmuffs | A scarf over the void, a stove | A knitted cap, a lantern |

Each one's full brief is its `concept.md` in `art/shopkeepers/<key>`.

## 6. Review

- **Concepts:** a lineup sheet (black silhouettes, then colored, beside a lane minion and a
  champion for scale), a 3/4 close-up of each with its stall, and the gameplay camera's view of a
  base with an ally and an enemy shop. The owner approves the four before modeling continues.
- **Built:** the review renders of 10 §8.5 for every clip, each stall in each season at the
  gameplay camera, and a capture of each in game in an ally and an enemy base.
- **In game:** `--shopkeepers A[,B]` puts keeper A at your base (and B at the enemy's) and
  `--shopkeeper-tour` has both play every clip in turn, a couple of seconds apart, so each keeper
  can be checked without waiting on fidgets or events.
