# Rook — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Look chosen by the owner: a grizzled veteran, iron and oxblood, a stone-headed maul.

## Identity

- **Role / kit** (`mftr-sim`): bruiser. Melee maul attack (175 range, 0.7 attacks/s, 28% windup).
  - **Q Cleave:** a 360° sweep around him (a centered area).
  - **W Second Wind:** a self-heal (40 + 12% of missing health), instant and mobile.
  - **E Lunge:** a leap onto the enemy nearest the cursor that strikes and slows on arrival.
  - **R Shockwave:** a two-handed slam that sends a slowing shockwave down a line (ultimate).
- **Personality in three words:** dependable, gruff, unstoppable.
- **Silhouette class:** *brute*. Broad and deep (`biped_large`, ~2.05 m), huge layered pauldrons, a 1.4 m stone-headed maul resting on his right shoulder. Reads as "the big one" at a glance next to Vesper.
- **Head:** the shared sculpted head, weathered: a square jaw and heavy chin, small hard eyes, shaved bald, a full grey-brown beard ending in a braid with an iron bead, a moustache, a pale scar over the left brow.
- **Archetype:** `biped` bones in the `large` shape (`rig.use_shape("large")`), no extra bones. The maul is part of the skinned mesh on his right hand, laid along the hand bone so each pose aims it exactly (`hand_r` aim).

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | weathered (0.76, 0.56, 0.44) |
| cloth | grey-brown beard (0.36, 0.31, 0.27), quilted tan gambeson (0.40, 0.33, 0.24), oxblood tabard (0.36, 0.08, 0.07), dark trousers (0.18, 0.15, 0.13), leather belt and grip (0.38, 0.24, 0.14), boots (0.22, 0.15, 0.10), haft wood (0.36, 0.22, 0.11), the maul's stone (0.56, 0.54, 0.49) |
| metal | battered iron plate (0.44, 0.45, 0.48) and darker iron for gauntlets, gorget and bands (0.27, 0.28, 0.31) |
| emissive | none |
| accent | the sash across his breastplate, the tabard hems and the pauldron rims; all read from the gameplay camera |

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (rolls his neck), idle_fidget_2 (a big stretch, maul overhead), idle_ready | Maul on the right shoulder; ready: two-handed guard |
| run (340 u/s), run_fast (450 u/s) | Maul stays on the shoulder, the left arm pumps, a forward lean |
| walk | Shared |
| attack_1, attack_2 | `fire` 12 (sim: 12). An overhead slam into the ground; a sidelong sweep |
| q | Cleave: a full spin with the maul held out; `fire` 6 |
| w | Second Wind: thumps his chest, a deep breath (upper body, plays as a pulse: the cast is instant) |
| e_start, e_travel, e_land | Lunge: a crouched launch, airborne with the maul high, landing maul-first |
| r | Shockwave: both hands high, a held beat, a ground-shaking slam; `fire` 14 (sim: 13.5) |
| cast_utility | Shared |
| recall | Kneels, the maul planted head-down (`loop_in`/`loop_out`) |
| death, respawn, select | Drops to his knees and falls forward; rises from a crouch; hoists the maul overhead and roars |
| cc_* | Shared |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; flexes; a belly laugh; a heavy stomping two-step |

## Effects and sounds (A6)

- **VFX** (`export/rook.vfx.ron`):
  - stone chips and dust where the maul lands (`attack.fire`, `r.fire`);
  - Cleave's dust ring on the area's true radius;
  - gold motes for Second Wind;
  - Lunge dust;
  - a dusty Shockwave.
- **SFX** (`sounds.ron`), 13 sounds:
  - swings, two maul impacts, Cleave's whoosh and thud;
  - two chest thumps and a breath for Second Wind;
  - the Lunge's launch and landing;
  - Shockwave's rise, boom, roll and crack;
  - a plate-drum emote.

## Definition of done

See [10 §10](../../../docs/design/10-characters-and-animation.md#10-pilots-vesper-and-rook-d50).
