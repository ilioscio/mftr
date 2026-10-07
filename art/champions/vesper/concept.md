# Vesper — concept

Status: **draft** (awaiting the owner's approval, 10 §10)

## Identity

- **Role / kit** (`mftr-sim`): marksman. Ranged bow attack (575 range, 0.8 attacks/s, 18% windup).
  - **Q Longshot:** a long, thin line shot (250 ms windup).
  - **W Shrapnel Charge:** a lobbed bomb that bursts after a delay (delayed area).
  - **E Tumble:** a short roll (dash).
  - **R Snare Net:** a wide thrown net that roots (hard CC line, ultimate).
- **Personality in three words:** patient, precise, wry.
- **Silhouette class:** *ranger archer*. A long recurve bow taller than her shoulders, a cloak tail that trails her movement, an auburn ponytail. Her hood is **down**, a cowl around her neck and over her upper back (a hood standing up read as confusing). A feminine hourglass build: a shaped bust, a narrow waist, fuller hips. Slim overall.
- **Head:** the shared sculpted head (`tools/blender/mftr_blender/head.py`) with feminine settings: slimmer, a narrower jaw and smaller chin, larger green eyes; side-swept bangs and a ponytail.
- **Archetype:** `biped` v1, plus an `extra_cape` chain (3 bones) for the cloak.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | warm pale (0.84, 0.66, 0.54) |
| cloth | auburn hair (0.50, 0.22, 0.12), dusk teal cloak and hood (0.14, 0.32, 0.30), dark slate trousers (0.16, 0.17, 0.20), sleeves (0.20, 0.24, 0.22), leather jerkin, belt, quiver and boots (0.42, 0.27, 0.16 / 0.30, 0.19, 0.11), gloves (0.25, 0.16, 0.10), bow wood (0.36, 0.21, 0.10) |
| metal | bow tips and buckle, worn silver (0.74, 0.76, 0.80) |
| emissive | none (she reads by shape, not glow) |
| accent | bracers, quiver fletching and the bow grip's band. Bracers and fletching read from above |

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (checks the string), idle_fidget_2 (scans the horizon), idle_ready | Bow held low in the left hand |
| run (330 u/s), run_fast (450 u/s) | Bow arm steady and low, right arm pumps, cloak streams back |
| walk | Shared |
| attack_1, attack_2 | `fire` 7 (sim: 6.75). Planted draw, a one-frame release, a recoil that cancels cleanly |
| attack_melee_alt | Shared (a bow-bash would replace it later) |
| q | Longshot: deeper draw, higher aim; `fire` 8 (sim 7.5) |
| w | Shrapnel Charge: overhand lob with the right hand; `fire` 8 |
| e_start, e_travel, e_land | Tumble: crouch, forward roll (in place), land in a crouch |
| r | Snare Net: two-handed spinning throw; `fire` 8; a heavier follow-through |
| cast_utility | Shared |
| recall | Kneels, bow planted, breathes (`loop_in`/`loop_out`) |
| death, respawn, select | Falls to her knees and sideways; rises from a crouch; showcase: a shot into the sky |
| cc_* | Shared |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; spins the bow; a dry chuckle; a light side-step dance |

## Definition of done

See [10 §10](../../../docs/design/10-characters-and-animation.md#10-pilots-vesper-and-rook-d50). A4a covers the model and every clip in game; A4b adds VFX, A4c SFX.
