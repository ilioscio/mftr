# Lumen — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Designed without an owner brief, from the placeholder's "slender, with a glowing halo".

## Identity

- **Role / kit** (`mftr-sim`): enchanter. A ranged mote of light (550 range, 0.65 attacks/s, 20% windup).
  - **Q Mending Light:** heals an ally, instant.
  - **W Aegis:** shields an ally, instant.
  - **E Lull:** a slowing line.
  - **R Binding Halo:** a delayed area that roots (ultimate, hard CC).
- **Personality in three words:** serene, warm, unshakable.
- **Silhouette class:** *light-priestess*.
  - A floor-length ivory robe flaring to a gold hem (a bell shape no one else has) and a short capelet.
  - Wide bell sleeves with gold cuffs.
  - A ring of gold light floating behind her head: a halo of twelve glowing segments.
  - She works from open hands; no staff.
- **Head:** the shared sculpted head, feminine and calm, with pale gold hair in a ponytail and pale blue eyes.
- **Archetype:** `biped` v1. The legs move under the robe, which is skinned to the pelvis; the shins and slippers show at the hem.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | (0.88, 0.72, 0.62) |
| cloth | ivory robe (0.86, 0.83, 0.74) with a shaded lower tier (0.72, 0.69, 0.62), pale gold hair (0.86, 0.76, 0.52), slippers (0.62, 0.52, 0.38) |
| metal | gold hem, belt and cuffs (0.86, 0.68, 0.30) |
| emissive | the halo and a gem at her throat (1.0, 0.88, 0.55) |
| accent | the stole: two bands from her shoulders down the front of the robe |

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (cups a mote of light, lets it rise), idle_fidget_2 (tucks back a strand of hair), idle_ready | Hands folded before her |
| run, run_fast | Light, upright |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | `fire` 9 (sim 9.2): a flick from the right palm; a backhand from the left |
| q, w | Mending Light, Aegis: instant, upper-body pulses from `fire` 4: palms open toward the ally; arms crossed, then spread |
| e | Lull: a slow sweep of the right arm; `fire` 8 |
| r | Binding Halo: arms rise to draw a circle overhead and sweep it down; `fire` 8 |
| recall, death, respawn, select | Kneels in prayer; sinks to her knees and folds aside; rises; opens her arms to the light |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Wags a finger; props up her "heavy" halo; laughs behind her hand; a slow twirl |

## Effects and sounds

- **VFX** (`export/lumen.vfx.ron`):
  - gold motes;
  - a rising golden glow for Mending Light and a pale blue one for Aegis;
  - a lilac orb for Lull;
  - Binding Halo's ring of gold on its true radius.
- **SFX** (`sounds.ron`), 9 sounds, bell-like and soft:
  - chimes for her motes;
  - a rising arpeggio for Mending Light;
  - a shimmering pad for Aegis;
  - Lull's falling tones;
  - Binding Halo's ringing bell;
  - a hummed emote.
