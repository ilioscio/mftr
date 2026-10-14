# Cairn — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from his kit (a warden who peels and holds a choke, with stone spells) and the placeholder's green.

## Identity

- **Role / kit** (`mftr-sim`): warden. A melee staff strike (150 range, 0.62 attacks/s, 30% windup).
  - **Q Stone Lash:** a slowing line.
  - **W Shelter:** a shield on an ally or himself (instant).
  - **E Rockfall:** a delayed area at range that roots.
  - **R Monolith:** a delayed knock-up around himself.
- **Personality in three words:** patient, weathered, wry.
- **Shape (14 §2):** *antlers and a mossy stone mantle*. Broad, heavy proportions on the `biped` bones (`SHAPE` in `build.py`: ×1.15 wide, ×1.1 deep, shoulders out 5 cm, hands ×1.4), about 2.0 m, 2.45 m to the antlers' tips. Great branching antlers (a main beam and three tines each) from his brow with emerald beads at their roots; a mantle of granite slabs over his shoulders and around his neck, mossed on top; stone pauldrons; bark plates on his chest, bracers and greaves; a kilt of hanging bark slats and moss; a stripe of emerald paint across his eyes; a moss-green cloak on an `extra_cloak` chain (3 bones).
- **Weapon:** a gnarled, wandering staff crowned with a glowing emerald held in four twisted roots. Skinned to his right hand and held by IK; his blows take it in both hands (the left hand solved onto it).
- **Head:** the shared sculpted head, weathered and bearded.
- **Gait (14 §3.5):** heavy.

## Palette (14 §4)

Signature: **emerald** (135°).

| Slot | Colors |
|---|---|
| skin | weathered (0.62, 0.44, 0.32) |
| cloth | bark (0.26, 0.18, 0.12 / 0.15, 0.10, 0.07), granite (0.30, 0.30, 0.28 / 0.18, 0.18, 0.17), moss (0.16, 0.30, 0.10 / 0.09, 0.18, 0.06), antler bone (0.62, 0.56, 0.44), leather (0.30, 0.20, 0.12), grey-brown hair |
| emissive | emerald (0.30, 1.0, 0.55) and deep emerald (0.06, 0.62, 0.30): the staff's stone, the beads, the paint, a chest gem |
| accent | a band at his belt |

## Weight (14 §3)

Two-handed staff blows with a cock and a held impact (an overhead strike into the ground; a low sweep held out); Rockfall stamps the staff's foot down beside him; Monolith drives it point-first into the ground at his feet and he leans on it, holding, as the stone rises. Feet planted by IK.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (kneels and lays a palm on the ground to listen), idle_fidget_2 (sets a stone of the cairn straight), idle_ready | Staff planted |
| run, run_fast | Staff carried upright |
| walk, cast_utility, cc_* | Shared |
| attack_1, attack_2 | Overhead strike and side sweep; `fire` 14 (sim 14.5) |
| q | Stone Lash: the staff whipped forward; `fire` 8 |
| w | Shelter: instant, upper body, the free hand raised palm-out; `fire` 4 |
| e | Rockfall: the staff pointed at the sky, then brought down; `fire` 8 |
| r | Monolith: the staff raised in both hands and driven into the ground; `fire` 8 |
| recall, death, respawn, select | Kneels with head bowed; topples aside; rises; strikes the staff down and lifts his face |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; reaches to hang his cloak on his own antler; chuckles; a slow stamping dance |

## Effects and sounds

- **VFX** (`export/cairn.vfx.ron`), all emerald (white → emerald → deep emerald) with stone: an overhead `slash`, a star, `crack`s and stone chips for his strikes; Stone Lash flies as a green-cored shard; Shelter rings its ally in emerald; Rockfall falls in a `pillar` with a `shock` and stone; Monolith `crack`s the ground around him and erupts in emerald light and stone.
- **SFX** (`sounds.ron`), 11 sounds:
  - a creaking swing and two stone knocks;
  - Stone Lash's whip-crack and shards;
  - Shelter's warm chime;
  - Rockfall's call and heavy landing;
  - Monolith's strike and eruption;
  - a low chuckle.
