# Quill — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from her kit (long-range artillery, a "Starfall Lance").

## Identity

- **Role / kit** (`mftr-sim`): artillery mage. A ranged bolt (550 range, 0.62 attacks/s, 22% windup).
  - **Q Arc Shot:** a long-range delayed area.
  - **W Static Field:** a slowing delayed area.
  - **E Recoil:** a hop back (dash).
  - **R Starfall Lance:** a map-length line (2,500 range, 600 ms windup).
- **Personality in three words:** curious, meticulous, unflappable.
- **Shape (14 §2):** *a tall pointed hat and a long coat*. Gangly proportions on the `biped` bones (`SHAPE` in `build.py`: long legs, ×0.97 wide, arms ×1.08, hands ×1.3), about 1.85 m, 2.35 m to the star on her hat's tip. A wide-brimmed astronomer's hat bending back to a point, brass goggles with glowing lenses on its band; a long navy coat flaring to her shins, its hem flecked with stars and its tails on an `extra_coat` chain; indigo lapels and shoulder capes; two rows of brass buttons; tall buckled boots; a scroll case and a brass spyglass at her hips.
- **Weapon:** a tall staff topped by an orrery: three crossing brass rings and two little planets around a glowing star. Skinned to her right hand and held by IK (`staff()` in `build.py`); her big casts take it in both hands.
- **Head:** the shared sculpted head, feminine, a short dark bob, indigo eyes.
- **Gait (14 §3.5):** athletic, the staff carried at a slant.

## Palette (14 §4)

Signature: **starlight indigo** (235°).

| Slot | Colors |
|---|---|
| skin | (0.80, 0.60, 0.48) |
| cloth | navy coat and hat (0.08, 0.10, 0.22 / 0.04, 0.05, 0.12), indigo lapels, capes, hat band and coat tails (0.18, 0.16, 0.42), a parchment shirt (0.66, 0.60, 0.46), leather boots and case (0.28, 0.18, 0.10), staff wood (0.26, 0.17, 0.10), dark hair |
| metal | brass: buttons, buckles, hems, goggles, the orrery's rings (0.68, 0.50, 0.22) |
| emissive | starlight (0.62, 0.66, 1.0) and star white (0.92, 0.94, 1.0): the orrery's star, the hat's star, the hem's stars, the lenses |
| accent | a band at her belt |

## Weight (14 §3)

The orrery is thrust at her targets (drawn back, cocked, a one-frame thrust, held). Arc Shot swings the staff from low behind her up and over in both hands. Static Field plants the staff's foot. Recoil jabs the staff into the ground and vaults back off it, landing in a held skid. **Starfall Lance** is a long planted charge: the staff raised to the sky in both hands, lowered to aim down the line, trembling for four frames as it builds, released on frame 18, and the recoil throws her back a step and holds through the hard lock.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (peers at the sky through her goggles), idle_fidget_2 (taps the rings to set them turning), idle_ready | Staff planted |
| run, run_fast | Staff carried forward |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | `fire` 11 (sim 10.6): the staff levelled at the target |
| q | Arc Shot: the staff swept up and over; `fire` 8 |
| w | Static Field: the staff's foot struck down; `fire` 8 |
| e_start, e_travel, e_land | Recoil: a hop backward, the coat flaring |
| r | Starfall Lance: the staff raised to the sky for a long beat, then levelled; `fire` 18 (sim 18) |
| recall, death, respawn, select | Kneels and studies the sky; falls aside; rises; raises the staff to the stars |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; counts the stars on her fingers and loses count; laughs; spins with the staff as a partner |

## Effects and sounds

- **VFX** (`export/quill.vfx.ron`), all starlight (white → indigo → deep indigo): starlight sprays and pale-star bolts; Arc Shot falls as a star and lands in a `flash`, a star-burst, a `shock` and a ring; Static Field crackles in an indigo ring with `crack`s where the staff is planted; Starfall Lance flares white with a `slash` on release, flies as a white-hot shaft and strikes in a `pillar` of starlight.
- **SFX** (`sounds.ron`), 10 sounds:
  - arcane zaps;
  - Arc Shot's whistle and starburst;
  - Static Field's crackle and hum;
  - the Recoil hop;
  - Starfall Lance's gathering hum, searing release and hit;
  - a star-counting emote.
