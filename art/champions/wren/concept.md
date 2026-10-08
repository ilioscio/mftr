# Wren — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Designed without an owner brief, from her kit (a mobile ranged skirmisher with caltrops, a pounce and a volley) and her name.

## Identity

- **Role / kit** (`mftr-sim`): skirmisher. A crossbow bolt (500 range, 0.75 attacks/s, 20% windup).
  - **Q Ricochet:** a quick line.
  - **W Caltrops:** a slowing area at range.
  - **E Pounce:** a lunge onto a target.
  - **R Hail of Arrows:** a wide delayed area that slows.
- **Personality in three words:** cheeky, quick, restless.
- **Silhouette class:** *feathered scout*.
  - A peaked russet cap with a long team-colored feather. It reads from the gameplay camera.
  - A tan leather jerkin over a cream shirt, and leather bracers.
  - An olive capelet on an `extra_cape` chain.
  - Tall boots, and a quiver of bolts at her right hip.
  - A hand crossbow in her right hand.
- **Head:** the shared sculpted head, feminine and young, with auburn hair in a ponytail and hazel eyes.
- **Archetype:** `biped` v1, plus `extra_cape`. The crossbow lies along the right hand bone, so each pose aims it.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | (0.86, 0.66, 0.52) |
| cloth | russet cap (0.58, 0.2, 0.12), tan jerkin (0.56, 0.38, 0.2), cream shirt (0.82, 0.76, 0.62), olive capelet (0.32, 0.34, 0.16 / 0.22, 0.24, 0.11), trousers (0.26, 0.24, 0.16), boots and quiver (0.28, 0.17, 0.1), crossbow wood (0.36, 0.22, 0.12), auburn hair |
| metal | the crossbow's prod, the loaded bolt, the belt buckle (0.7, 0.72, 0.76) |
| accent | the cap's feather, the belt and the quiver's fletching |

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (draws a bolt from her quiver and loads it), idle_fidget_2 (blows the feather out of her eyes and tips her cap), idle_ready | Crossbow held low |
| run, run_fast | Crossbow carried forward |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | An aimed shot and a snap shot from the hip; `fire` 8 (sim 8.0) |
| q | Ricochet: a careful two-handed shot, leaning in; `fire` 8 |
| w | Caltrops: an underhand scatter from the left hand; `fire` 6 |
| e_start, e_travel, e_land | Pounce: a springing tucked leap, landing in a crouch with a point-blank shot |
| r | Hail of Arrows: the crossbow raised and loosed at the sky; `fire` 8 |
| recall, death, respawn, select | Kneels and checks her string; falls aside; springs up from a roll; spins the crossbow and tips her cap |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; shoots straight up and ducks; laughs; a heel-and-toe jig |

## Effects and sounds

- **VFX** (`export/wren.vfx.ron`):
  - crossbow bolts;
  - Ricochet's bright bolt and spark;
  - Caltrops' lob and scatter;
  - Pounce's dust and landing;
  - Hail of Arrows' volley.
- **SFX** (`sounds.ron`), 11 sounds:
  - a twang and a thunk;
  - Ricochet's shot and ping;
  - Caltrops' jingle and scatter;
  - Pounce's spring and landing;
  - Hail of Arrows' volley and drumming hail;
  - a whistle.
