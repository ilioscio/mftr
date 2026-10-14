# 14 — Champion Art Direction

How a champion is designed so it reads, feels heavy where it should, and owns its colors. This
sets the bar the [10](10-characters-and-animation.md) pipeline builds to; the first ten champions
are rebuilt against it (D59, Proposed). Measured against the reference champion the owner picked
as the target: [R05](reference/R05-video-wukong-abilities.md), a 21 s, 60 fps capture of the
reference game's Wukong using his kit on minions.

**Why:** the first block-outs had kits and complete clip sets, but six of ten shared one outline
(realistic 7½-head proportions, thin round limbs, arms hanging at the sides), their weapons
floated in one outstretched hand, every move eased in and out the same way, and their effects
were grey-brown dust in no one's colors. At the gameplay camera they read as a crowd of the same
person.

## 1. What the reference does (R05, frame by frame)

| | What we measured | Rule it gives |
|---|---|---|
| **Silhouette** | Shoulders ≈ 2× the head's width; huge round pauldrons, a crown with flared side pieces, a curled tail, a staff longer than he is tall. An inverted triangle on thick, planted legs. | §2: a shape, not a person |
| **Proportions** | Head ≈ ⅙ of the height; hands and feet oversized; limbs thick enough to read as solids at 120 px. | §2.2 heroic proportions |
| **Weapon** | Two hands on the staff whenever it does work; at rest it's a long diagonal across or behind him; while running it rides level across his back, perpendicular to the path. | §3.3 grips |
| **Basic/Q swing** | Windup ~230 ms: the staff goes high behind, then *cocks further* while the body coils low. The swing itself is **2 frames** (33 ms) under an orange arc smear. Impact flash on the target. The extended pose then **holds ~130 ms**, still, while sparks fly. ~250 ms recovery. | §3.1 timing shape |
| **E dash** | Leaps up (the silhouette grows toward the camera), staff high behind; a gold streak; lands in a **deep crouch with the staff planted** and **holds it ~200 ms** while the light fades; only then rises, swinging the staff back to the carry. | §3.2 holds |
| **R spin** | The brightest thing on screen is a crisp **ring at the area's true radius**; inside it, translucent grey wind and red staff streaks. Every knocked-up minion gets its own flash. | §4.2 edges vs interiors |
| **Color** | Gold plate, oxblood cloth, red-and-gold staff ends. His smears, sparks, dash streak, ring and ambient embers are **all gold/orange**. The kit is one palette. | §4 signature colors |

## 2. Silhouette and shape

### 2.1 One shape per champion

Each champion is built around a **dominant primitive** and a **signature read** (the one or two
shapes you'd draw first). No two champions share both. Checked on the black-silhouette lineup
(§6): from the front and from the side, each must be nameable.

### 2.2 Heroic proportions

The standard biped is ~7½ heads; ours become **~6 heads** (the head scaled up ~1.2×), with hands
~1.35× and feet ~1.25× of realistic, and limbs at least **0.08 m thick** (≥ 5 px at the gameplay
camera) except deliberately thin ones. Shoulders follow the character: brutes ≥ 3 head-widths,
casters may stay narrow if something else (a hat, sleeves, a halo) widens the top. Height varies
by champion (1.7–2.3 m) within the footprint rule of [10 §2](10-characters-and-animation.md#2-character-visual-style-faceted-ps1).

### 2.3 Break the tube

Round lofted limbs everywhere are the "inflatable tube man". Instead:
- **Masses, not cylinders:** boxy or wedge cross-sections for armor and torsos (4–6 sides with
  uneven radii), tapered forearms and calves, big hands.
- **Silhouette breakers** that belong to the character: pauldrons, collars, hats, horns, halos,
  capes, scarves, quivers, tails, plumes. They sit on the outline (shoulders, head, back), where
  the gameplay camera sees them.
- **Weapons are big and solid**: ≥ 0.06 m across the haft or blade, heads and blades exaggerated.
- **Detail budget goes to the outline first**, then color blocking, never to small interior bits.
  The budget is **5,000–8,000 triangles** (D59, double the first block-outs'): spend it on the
  silhouette's shapes, rounder masses where they're round, sculpted hands and layered armor and
  cloth, not on interior noise the gameplay camera can't see.

## 3. Weight and feel

### 3.1 The timing shape of a blow

```
 anticipation ──► (cock) ──► STRIKE ──► impact HOLD ──► recovery
 most of windup    2–4 fr     1–2 fr      3–6 fr          the rest
 slow in           held       linear      still/drift     ease out
```

At 30 fps: the strike is **1–2 frames** between the cocked pose and the contact pose, never an
eased tween. Heavy blows add a **cock** (the weapon keeps going back for 2–4 frames past the top
of the windup) and an **impact hold** (3–6 frames with the weapon at the contact point, drifting
a few degrees at most). The `fire` marker sits on the contact frame. Light, fast attacks (daggers,
bows) shorten every part but keep the shape.

### 3.2 Holds tell weight

A heavy thing stays where it lands: Rook's maul head rests on the ground before he hauls it up,
Bastion's fists stay planted after a stomp, a landing crouch holds before rising. Use a **moving
hold** (two keys a few frames apart with a small drift) rather than a frozen one.

### 3.3 Grips

- Two-handed weapons have **two hands on them** in every working pose: the off hand is solved onto
  the haft (`clips.grip`, an analytic two-bone IK), so it can't drift off.
- **IK targets** place hands and feet in armature space (`clips.ik`, `ik_hand_*` and `ik_foot_*`
  in a pose): a weapon's head rests on the ground while the body breathes, and feet stay planted
  while the hips drop into a crouch. Locomotion is built the same way (`library.gait`), so feet
  push off the ground instead of skating.
- Carry poses put the weight on the skeleton: on the shoulder, across the back, or resting on the
  ground, never held out at arm's length.
- Heavy weapons lead with the body: hips and chest turn before the arms, the arms before the
  weapon.

### 3.4 Interpolation

Every key says how it **leaves** for the next one (`mftr_blender.clips.EASES`): `ease` (the default
Bézier), `snap` (linear: strikes and impacts), `whip` (slow out of the key, accelerating into the
next: swings), `settle` (fast out of the key, settling into the next: recoveries), `hold`
(constant: a frame-perfect freeze, rare).

### 3.5 Cloth follows through

Capes, scarves, coat tails and shawls (`extra_<chain>` bones) aren't keyed by hand: after the
clips are built, `champion.secondary` simulates each chain as a rope of points that lags behind
the authored pose and springs back to it, and bakes it as a key on every frame. Spins flare the
cloth, stops overshoot, slams whip it through. Each chain has a `stiffness` and `inertia` (a
loose scarf, a heavy cloak); loops are simulated for three cycles so they stay seamless.

### 3.6 Locomotion has a body type

Runs are built per **gait**: *heavy* (long contact, low bob, wide stance, the weapon braced),
*athletic* (the default), *light* (on the toes, long airborne phase), *sneaky* (low, forward,
small arms), *floating* (no foot strike, a hover bob). The gait is in each concept.

## 4. Signature color

### 4.1 One palette, model and effects

Each champion owns a **signature hue**, used on the model (trim, emissive bits, the weapon's
business end) and in **all** of its effects (smears, sparks, impacts, projectile cores, auras).
Neutrals (skin, cloth, metal) support it. The ten hues are spread around the wheel so a teamfight
reads by color as well as shape:

| Champion | Signature | Supporting neutrals |
|---|---|---|
| Rook | crimson (0°) | iron, oxblood, old brass |
| Ember | flame orange (25°) | charcoal, soot black, ember red |
| Lumen | sun gold, pale (50°) | ivory, white, pale gold |
| Wren | lime (85°) | russet, tan leather |
| Cairn | emerald (135°) | bark, granite, moss |
| Vesper | teal (175°) | forest green, cream, leather |
| Bastion | ice blue (205°) | slate stone, dark iron |
| Quill | starlight indigo (235°) | navy, parchment, brass |
| Shade | violet (275°) | indigo-black, steel |
| Marrow | magenta (320°) | bone, plum, grave grey |

Effect ramps run **white → signature → dark signature**: the white core is the hit, the color says
whose it is.

### 4.2 What team color keeps

The team accent ([05 §1.3](05-art-and-assets.md#1-readability-rules-non-negotiable)) owns the
*edges*: projectile sheaths, ground indicators, rings at true radii, health bars. The signature
owns the *interiors*. On the model, the `accent` slot is one small, consistent band (belt or
bracer), so team color never fights the signature for the silhouette.

### 4.3 Effects that sell the hit

The kit gains (05 §5): **`slash`**, an arc smear in front of the champion along the swing, for 2–4
frames; **`flash`**, a 2–3 frame star at the impact point; **`crack`**, radial ground lines that
linger; **`pillar`**, a short vertical streak (landings, dash arrivals); **`shock`**, a ring that
expands to a radius. All pixel-style, all colored by the ramp.

## 5. The roster

Each champion's `concept.md` holds the full brief; the summary:

| Champion | Shape / signature read | Proportions & gait | Weapon & grip | Must-feel moment |
|---|---|---|---|---|
| **Rook** | A block on legs: slab pauldrons, a head sunk into the shoulders, a maul with a huge square head | Brute, ~2.15 m, heavy gait | Two-handed maul; idles with its head on the ground, both hands on the pommel; runs with it on his shoulder, braced | R: the maul held high a beat, slammed down, the head **left in the ground** a long moment |
| **Bastion** | A boulder: shoulders above the head, giant fists, ice-blue crystal core | Hulking, ~2.3 m, heavy gait, knuckles low | His fists | E: both fists into the ground and held, cracks spreading |
| **Vesper** | A huge recurve bow and a long one-sided cloak; quiver feathers over the shoulder | Athletic, ~1.85 m | Bow in the left hand, draws to the cheek | Q: a deep draw held, a snap release, recoil |
| **Ember** | A flame: high flared collar, burning hair, bell sleeves; top-heavy | Slim, ~1.8 m, light gait | Bare hands with fire | Q: both palms drawn back, then thrust, the lance leaving with recoil |
| **Lumen** | A bell: a long robe, a big halo ring behind the head, wide sleeves | Serene, ~1.8 m, floating gait | A lantern-staff | R: arms raised, the halo flaring |
| **Shade** | A blade: hunched, a long scarf tail, two long curved knives held reversed, a pointed hood | Lean, ~1.75 m, sneaky gait | Twin knives, reverse grip | Q: vanish-crouch, a streak, a cross-slash on arrival |
| **Quill** | A tall pointed hat with a brim and a long coat; a staff topped by an orrery ring | Gangly, ~1.85 m (the hat to ~2.3 m) | Two-handed staff for the big casts | R: the staff planted, the lance charged and held, then fired |
| **Cairn** | Antlers and a mossy stone mantle; a gnarled staff | Broad, ~2.0 m, heavy gait | Two-handed staff | E: the staff driven down, rocks falling |
| **Marrow** | A hooded crone: a high bone collar, a ribcage corset, long claws, ragged skirt | Bent, ~1.8 m, unhurried gait | Bare clawed hands, a skull lantern at the hip | Q: arms flung wide, the drain pulled in |
| **Wren** | A feather crest and a short cape of feathers; a crossbow at the hip | Small, ~1.65 m, light gait | One-handed crossbow, the other hand balances | E: a crouch, a spring, a landing on all fours |

## 6. Review

Besides 10 §8.5's per-clip sheets: a **lineup** of all champions at idle, in color and as black
silhouettes from the front and the side, and the gameplay camera at its true size. A redesign is
done when the silhouettes are distinct, the timing shapes (§3.1) are visible in the contact sheets,
and the champion's effects are in its signature color.
