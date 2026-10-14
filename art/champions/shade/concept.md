# Shade — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from the placeholder's "a sharp blade of a body with two forward blades".

## Identity

- **Role / kit** (`mftr-sim`): assassin. A quick melee slash (125 range, 0.72 attacks/s, 25% windup).
  - **Q Shadow Step:** a lunge onto a target that strikes on arrival.
  - **W Fan of Blades:** a slowing nova.
  - **E Veil Step:** a short dash out.
  - **R Execution:** a lunging finisher (ultimate).
- **Personality in three words:** quiet, precise, cold.
- **Shape (14 §2):** *a blade*. Lean proportions on the `biped` bones (`SHAPE` in `build.py`: narrow, ×0.95, arms ×1.08, hands ×1.35), about 1.75 m to the hood's tip, always hunched forward. A tall pointed hood sweeping back into a point, a black mask with violet eyes burning out of it, a violet scarf streaming behind on an `extra_scarf` chain (3 bones), a spiked steel guard on his right shoulder only, two spare blades sheathed crossed on his back, kunai strapped to his thighs.
- **Weapons:** two long curved knives held **reversed**: the blade leaves the bottom of the fist and runs back along the forearm, curving out, its edge glowing violet.
- **Gait (14 §3.6):** sneaky: low (a 14 cm crouch), leaning 28°, arms swept back with the blades trailing.

## Palette (14 §4)

Signature: **violet** (275°).

| Slot | Colors |
|---|---|
| cloth | black (0.06, 0.055, 0.09), indigo (0.12, 0.09, 0.20), violet cloth for the scarf and front flap (0.26, 0.10, 0.38), wraps (0.24, 0.21, 0.28), leather (0.18, 0.12, 0.10) |
| metal | steel blades (0.42, 0.44, 0.50), dark steel guards (0.20, 0.21, 0.25) |
| emissive | violet (0.78, 0.36, 1.0): eyes, blade edges, the scarf's end |
| accent | a band at his waist |

## Weight (14 §3)

Slashes coil the cutting arm across the body, cock, and cut through in one frame on `fire`, the cut held out. Each of his three dashes ends with a hold: a cross-slash (Shadow Step), a long skid (Veil Step), both knives plunged down and left there (Execution). Feet planted by IK.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (spins a dagger), idle_fidget_2 (glances over each shoulder), idle_ready | Daggers low, a low crouch when ready |
| run, run_fast | Blades held back, the scarf streaming |
| walk, cast_utility, cc_* | Shared |
| attack_1, attack_2 | `fire` 10 (sim 10.4): a cross-body slash with each blade |
| q_start, q_travel, q_land | Shadow Step: a low lunge with both blades leading, a strike on landing |
| w | Fan of Blades: a crouched spin, arms flung wide; `fire` 4 (sim 4.5) |
| e_start, e_travel, e_land | Veil Step: a low smoky glide |
| r_start, r_travel, r_land | Execution: a leap with both blades raised, driven down on landing |
| recall, death, respawn, select | Kneels; falls aside; rises; twirls and crosses both blades before his masked face |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; juggles his daggers; a silent laugh; a low swaying dance |

## Effects and sounds

- **VFX** (`export/shade.vfx.ron`), all violet (white → violet → deep violet): a thin level `slash`, a star and sparks on his cuts; a column of shadow (`pillar`) as he dashes away; a cross-`slash` and star when Shadow Step arrives; Fan of Blades rings him with a `slash` and runs a `shock` to its edge; Execution lands in a big `flash`, a `shock` and `crack`s.
- **SFX** (`sounds.ron`), 10 sounds:
  - swishes and thin metallic slices;
  - a hushed rush of air for each step;
  - a whirling ring of steel for Fan of Blades;
  - Execution's rising hiss and ringing strike;
  - a blade-spin emote.
