# Marrow — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Designed without an owner brief, from her kit (a battlemage who drains, roots and opens a pit of bone) and her name.

## Identity

- **Role / kit** (`mftr-sim`): battlemage. A ranged bolt (475 range, 0.65 attacks/s, 22% windup).
  - **Q Siphon:** a draining nova around her.
  - **W Grasping Bones:** a rooting line.
  - **E Grave Pact:** a self heal (instant).
  - **R Ossuary:** a large delayed area that slows.
- **Personality in three words:** sly, hungry, unhurried.
- **Silhouette class:** *bone-witch*.
  - A crown of bone spikes, tallest at the front. It reads from the gameplay camera.
  - Skull-capped pauldrons with eye sockets.
  - A corset of bone ribs over an ash-plum bodice, and a ragged skirt to the shins.
  - A tattered shawl on an `extra_cloak` chain.
  - Grave-green light at her wrists. She casts from clawed bare hands, with no staff.
- **Head:** the shared sculpted head, feminine and gaunt, with long black hair and green eyes.
- **Archetype:** `biped` v1, plus `extra_cloak`.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | pale (0.82, 0.76, 0.74) |
| cloth | ash-plum robes (0.2, 0.09, 0.19 / 0.11, 0.05, 0.11), shin wraps (0.3, 0.26, 0.24), bone (0.86, 0.82, 0.7 / 0.66, 0.62, 0.52), black hair |
| emissive | the wrist light and the choker's bead (0.55, 1.0, 0.5) |
| accent | the sash and the skirt's hem |

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (rolls a knucklebone across her knuckles), idle_fidget_2 (cracks her neck), idle_ready | A slight hunch, fingers curled |
| run, run_fast | Shared run, claws, the shawl trailing |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | A clawed backhand and a palm thrust; `fire` 10 (sim 10.2) |
| q | Siphon: claws flung wide, then dragged to her chest; `fire` 6 |
| w | Grasping Bones: drops to a knee and drives a claw into the ground; `fire` 9 |
| e | Grave Pact: instant, upper body, a hand clutched to her chest, head thrown back; `fire` 4 |
| r | Ossuary: both arms raised, then thrust toward the far ground; `fire` 8 |
| recall, death, respawn, select | Kneels with palms flat on the ground; falls; claws her way up; rises from a hunch with a crooked smile |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; snaps a rib like a wishbone; laughs; a puppet-jointed shuffle |

## Effects and sounds

- **VFX** (`export/marrow.vfx.ron`):
  - grave-green bolts;
  - Siphon's ring;
  - Grasping Bones' shards and snap;
  - Grave Pact's green rush;
  - Ossuary's pit of bone.
- **SFX** (`sounds.ron`), 9 sounds:
  - a hollow hiss and a dry crack;
  - Siphon's inward drone;
  - Grasping Bones' rattle and snap;
  - Grave Pact's chord;
  - Ossuary's groan and eruption;
  - a cackle.
