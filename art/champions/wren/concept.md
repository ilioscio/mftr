# Wren — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from his kit (a mobile ranged skirmisher with caltrops, a pounce and a volley) and his name.

## Identity

- **Role / kit** (`mftr-sim`): skirmisher. A crossbow bolt (500 range, 0.75 attacks/s, 20% windup).
  - **Q Ricochet:** a quick line.
  - **W Caltrops:** a slowing area at range.
  - **E Pounce:** a lunge onto a target.
  - **R Hail of Arrows:** a wide delayed area that slows.
- **Personality in three words:** cheeky, quick, restless.
- **Shape (14 §2):** *a feather crest and a short cape of feathers*. Small and springy proportions on the `biped` bones (`SHAPE` in `build.py`: ×0.88, a big heroic head, hands ×1.35), about 1.65 m, 1.95 m to the crest's tips. A tall crest of russet feathers tipped in lime swept back from his brow; a capelet of layered feathers over his shoulders and down his back on an `extra_cape` chain (2 bones); a fan of tail feathers at the back of his belt; a russet tunic with a lime hem under a leather jerkin; feathered bracers and boot cuffs; a quiver of lime-fletched bolts at his thigh.
- **Weapon:** a hand crossbow in his right hand (skinned to it and laid along it, so his fist aims it): a wooden stock, a swept iron prod with lime tips, a crank, a lime bolt. He fires it one-handed like a pistol, arm straight out, canted on its side (`aim_gun`, the hand turned with `{"turn": …}`); Ricochet is braced low in both hands.
- **Head:** the shared sculpted head, young, heroic-sized (×1.2), auburn hair under the crest, green eyes: a lethal birdman.
- **Gait (14 §3.6):** light: a short contact and long airborne strides.

## Palette (14 §4)

Signature: **lime** (85°).

| Slot | Colors |
|---|---|
| skin | (0.80, 0.60, 0.46) |
| cloth | russet tunic and feathers (0.44, 0.17, 0.07), tan (0.54, 0.40, 0.24), leather (0.28, 0.17, 0.09), cream sleeves (0.68, 0.62, 0.48), dark leggings (0.14, 0.09, 0.05), lime cloth (0.30, 0.46, 0.08), crossbow wood (0.32, 0.20, 0.10), auburn hair |
| metal | the prod, crank and buckle (0.34, 0.35, 0.38) |
| emissive | lime (0.66, 1.0, 0.20): the bolts' fletching and the loaded bolt |
| accent | a band at his belt |

## Weight (14 §3)

Restless and bird-quick: his idle is stillness broken by snapped head flicks. Each shot is raised and aimed with his left hand under the stock, steadied, fired on `fire` with the crossbow kicking up, and the recoil held; Hail of Arrows fires three times at the sky, each kick landing. Pounce coils low, springs, and lands on all fours (hands and feet planted by IK), held a beat before he springs up. Feet planted by IK.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (draws a bolt from his quiver and loads it), idle_fidget_2 (blows the feather out of his eyes and tips his cap), idle_ready | Crossbow held low |
| run, run_fast | Crossbow carried forward |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | An aimed shot and a snap shot from the hip; `fire` 8 (sim 8.0) |
| q | Ricochet: a careful two-handed shot, leaning in; `fire` 8 |
| w | Caltrops: an underhand scatter from the left hand; `fire` 6 |
| e_start, e_travel, e_land | Pounce: a springing tucked leap, landing in a crouch with a point-blank shot |
| r | Hail of Arrows: the crossbow raised and loosed at the sky; `fire` 8 |
| recall, death, respawn, select | Kneels and checks his string; falls aside; springs up from a roll; spins the crossbow and tips his cap |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; shoots straight up and ducks; laughs; a heel-and-toe jig |

## Effects and sounds

- **VFX** (`export/wren.vfx.ron`), all lime (white → lime → deep lime): lime sparks and bright lime shafts, a star on hits; Ricochet's whip-crack `slash`; the caltrops land in lime sparks with a `shock` and a ring; Pounce lands with a star, `crack`s and dust; Hail of Arrows falls in a `pillar` of lime and bursts across the area.
- **SFX** (`sounds.ron`), 11 sounds:
  - a twang and a thunk;
  - Ricochet's shot and ping;
  - Caltrops' jingle and scatter;
  - Pounce's spring and landing;
  - Hail of Arrows' volley and drumming hail;
  - a whistle.
