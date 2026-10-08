# Quill — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Designed without an owner brief, from her kit (long-range artillery, a "Starfall Lance") and the placeholder's teal.

## Identity

- **Role / kit** (`mftr-sim`): artillery mage. A ranged bolt (550 range, 0.62 attacks/s, 22% windup).
  - **Q Arc Shot:** a long-range delayed area.
  - **W Static Field:** a slowing delayed area.
  - **E Recoil:** a hop back (dash).
  - **R Starfall Lance:** a map-length line (2,500 range, 600 ms windup).
- **Personality in three words:** curious, meticulous, unflappable.
- **Silhouette class:** *stargazer scholar*.
  - A long teal coat with brass buttons, team-colored lapels and coat tails (an `extra_coat` chain).
  - Tall boots.
  - A tall staff topped with a brass armillary sphere: three crossing rings around a glowing star. It reads from the gameplay camera.
  - Brass goggles pushed up on her forehead.
- **Head:** the shared sculpted head, feminine, with a short dark bob and teal eyes.
- **Archetype:** `biped` v1, plus `extra_coat`. The staff lies along the right hand bone, so each pose aims it.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | (0.82, 0.62, 0.5) |
| cloth | deep teal coat (0.12, 0.34, 0.38 / 0.08, 0.22, 0.25), a pale shirt (0.82, 0.78, 0.68), trousers (0.2, 0.18, 0.2), boots and goggle strap (0.24, 0.16, 0.1), staff wood (0.3, 0.2, 0.12), dark hair |
| metal | brass buttons, cuffs, goggles and the armillary rings (0.8, 0.62, 0.3) |
| emissive | the star at the staff's head and the goggle lenses (0.82, 0.97, 1.0) |
| accent | the lapels and the belt |

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

- **VFX** (`export/quill.vfx.ron`):
  - star-white bolts;
  - Arc Shot's lobbed star and starburst;
  - Static Field's cyan ring;
  - Recoil dust;
  - Starfall Lance's white-hot shaft and blinding burst.
- **SFX** (`sounds.ron`), 10 sounds:
  - arcane zaps;
  - Arc Shot's whistle and starburst;
  - Static Field's crackle and hum;
  - the Recoil hop;
  - Starfall Lance's gathering hum, searing release and hit;
  - a star-counting emote.
