# 16 — Jungle Monster Art

**Status:** Proposed (D61). The jungle's six camps ([01 §7](01-gameplay.md)) get real models and
animation, to the bar [14](14-champion-art-direction.md) sets for champions and on the map of
[15](15-map-art-direction.md). Until now they were placeholders: a faceted sphere sized to the
hitbox, a head, horn cones and a gold ring.

## 1. What the reference does

The reference map's camps (the reference video of R06, its jungle frames) read at the gameplay
camera because:
- **one glow per camp:** a buff monster's core burns in its buff's color, small camps carry a
  smaller glint of theirs, so a camp is named from across the screen;
- **size says rank:** buff monsters stand ~1.5–2× a champion's height, a camp's leader is plainly
  bigger than its followers, and the little ones are little;
- **they sit in their clearing:** creatures of the jungle (stone, bark, moss, fur, feathers) in its
  colors, darker than the champions, lit by their own glow.

## 2. Rules

1. **A shape per camp.** Six silhouettes nameable in black (the lineup, §6): an upright monolith,
   a knuckle-walking hulk, a pack of lean four-legged hunters, a squat round toad, crested birds,
   six-legged boulders.
2. **A signature glow per camp,** in the `accent` and `accent_glow` slots (the client paints them
   in the camp's color instead of a team's): eyes, a crystal heart, magma cracks, glowing warts.
   The rest is the jungle's palette, linear vertex colors as on the map's props.
3. **Weight, as champions** (14 §3): heavy blows cock, strike in 1–2 frames on the sim's `fire`
   and hold; big monsters move slowly and land hard; small ones are quick and twitchy.
4. **Alive while waiting.** Camps spend most of a match idle: every monster has an idle with
   breath in it and a fidget (a stretch, a scratch, a preen, a croak) the animator plays now and
   then.
5. **The gold ring stays** at their feet: neutral, "not a champion", for any team.

## 3. Rigs

- **Biped monsters** (Warden, Brute) use the `biped` bones in a large shape (10 §7.1), so the
  clip tools and IK apply.
- **`creature`** (new, 10 §7.1): a monster's own skeleton for four legs, wings or many legs.
  Lowercase snake-case names, `_l`/`_r` pairs on the left (+X) and right; it must have `root`,
  `head`, `socket_chest` and `socket_overhead` (and `socket_projectile` for the ranged), face +Z
  (the head ahead of the root), ≤ 64 bones. Built in Blender from a bone table
  (`rig.use_table`).

## 4. Packs and clips

`art/monsters/<key>` (`warden`, `brute`, `hound_alpha`, `hound`, `toad`, `raven_alpha`, `raven`,
`crawler_elder`, `crawler`), kind `monster`. Required clips: `idle`, `idle_fidget_1`, `run`,
`attack_1`, `death`, `flinch` (additive); `attack_2` for the bigger ones (the animator alternates
them). `fire` on an attack is the sim's windup (validated ±1 frame).

| Budget | Triangles |
|---|---|
| Buff monsters | 4,000–6,000 |
| Camp leaders, the toad | 2,500–4,000 |
| Followers | 1,200–2,500 |

## 5. The camps

| Camp | Read | Size | Glow | Moves |
|---|---|---|---|---|
| **Azure Warden** (buff) | A monolith of the paving's stone on two stumps: shoulders like lintels, a small head sunk between them, pillar forearms to block fists; a crack down its chest shows an **azure crystal heart**; three runestones orbit over its shoulders; moss on its top. | ~2.7 m | azure | Slow and planted. Idle: the runes turn, it breathes stone. Fidget: rolls a shoulder, the runes spin up. Attacks: both fists overhead, a slam that holds (1); a backhanded sweep (2). Death: to its knees, then face down. |
| **Ember Brute** (buff) | A boar-ape of charred bark: a hunched back of smoldering thorn stumps, a low tusked head, forearms longer than its legs, knuckles on the ground; **magma** in every crack. | ~2.2 m tall, very wide | ember red | Knuckle-walks. Idle: smoke-breathing, knuckles shifting. Fidget: beats its chest. Attacks: a two-fisted pound (1); a swinging haymaker (2). Death: falls on its side. |
| **Thicket Hounds** (alpha + 2) | Lean wolves, bark plates down the spine, a ruff of leaves; the alpha bigger with **bramble antlers**. | alpha 0.8 m at the shoulder, pups 0.55 m | moss green eyes | Lope. Idle: panting, tail. Fidget: sniff and scratch; the alpha howls. Attack: a lunging bite. Death: crumples on its side. |
| **Bog Toad** | A huge round toad, warty olive over an ochre belly, a lily pad and bloom on its head, **glowing warts** down its back. | ~1.0 m tall, 1.6 m wide | bog teal | Hops. Idle: the throat pulses. Fidget: a tongue snaps a fly. Attack (ranged): a throat-swell and a spat glob. Death: rolls onto its back. |
| **Ravenhawks** (matriarch + 3) | Crested raptor-birds on long legs, wings folded high; the matriarch bigger with a tall crest and a mantle of quills. | matriarch 1.3 m, the others 0.8 m | violet | Hop-flap. Idle: head-cocks. Fidget: a preen and a wing stretch. Attack (ranged): a wing flick that flings a quill. Death: a fall, wings splayed. |
| **Stone Crawlers** (elder + 1) | Six-legged crabs under a boulder shell, two heavy claws, **amber crystals** grown from the shell's cracks. | elder 1.0 m tall, 1.7 m wide; young 0.6 m | amber | Skitter. Idle: legs shift, claws flex. Fidget: claws clack. Attack: a crushing claw snap. Death: legs curl, the shell drops. |

## 6. Review

A lineup in color and in black silhouettes beside a champion and a minion; each camp at the
gameplay camera in its clearing, idle and in a fight; `pack validate` clean.
