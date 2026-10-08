# Cairn — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Designed without an owner brief, from his kit (a warden who peels and holds a choke, with stone spells) and the placeholder's green.

## Identity

- **Role / kit** (`mftr-sim`): warden. A melee staff strike (150 range, 0.62 attacks/s, 30% windup).
  - **Q Stone Lash:** a slowing line.
  - **W Shelter:** a shield on an ally or himself (instant).
  - **E Rockfall:** a delayed area at range that roots.
  - **R Monolith:** a delayed knock-up around himself.
- **Personality in three words:** patient, weathered, wry.
- **Silhouette class:** *antlered earth-warden*.
  - Tall and spare, with antlers rising from his brow. They read from the gameplay camera.
  - A moss-green cloak on an `extra_cloak` chain over bark-brown leathers, with a kilt of leather strips.
  - A staff topped with a small cairn of stacked stones, bound with team-colored ribbons.
  - Green paint in stripes down his cheeks and brow.
- **Head:** the shared sculpted head, long-faced and older, with grey hair tied back in a ponytail.
- **Archetype:** `biped` v1, plus `extra_cloak`. The staff lies along the right hand bone, so each pose aims it.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | (0.72, 0.56, 0.44), green paint (0.3, 0.55, 0.28) |
| cloth | moss cloak (0.24, 0.34, 0.18 / 0.16, 0.24, 0.12), bark leathers (0.32, 0.22, 0.14), leather (0.4, 0.28, 0.17), stones (0.5, 0.5, 0.47 / 0.36, 0.36, 0.34), antlers (0.78, 0.72, 0.6), grey hair |
| accent | the sash and the staff's ribbons |

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

- **VFX** (`export/cairn.vfx.ron`):
  - stone chips and moss dust where the staff lands;
  - Stone Lash's shards;
  - Shelter's green-gold ward;
  - Rockfall's falling stones and ring;
  - Monolith's eruption.
- **SFX** (`sounds.ron`), 11 sounds:
  - a creaking swing and two stone knocks;
  - Stone Lash's whip-crack and shards;
  - Shelter's warm chime;
  - Rockfall's call and heavy landing;
  - Monolith's strike and eruption;
  - a low chuckle.
