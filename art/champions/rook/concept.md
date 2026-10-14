# Rook — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Look chosen by the owner: a grizzled veteran, iron and oxblood, a stone-headed maul; v2 makes him a block on legs with a crimson signature.

## Identity

- **Role / kit** (`mftr-sim`): bruiser. Melee maul attack (175 range, 0.7 attacks/s, 28% windup).
  - **Q Cleave:** a 360° sweep around him (a centered area).
  - **W Second Wind:** a self-heal (40 + 12% of missing health), instant and mobile.
  - **E Lunge:** a leap onto the enemy nearest the cursor that strikes and slows on arrival.
  - **R Shockwave:** a two-handed slam that sends a slowing shockwave down a line (ultimate).
- **Personality in three words:** dependable, gruff, unstoppable.
- **Shape (14 §2):** *a block on legs*. Heroic brute proportions on the `biped` bones (`SHAPE` in `build.py`: low hips, a short neck, ×1.26 wide, ×1.22 deep, shoulders pushed out 8 cm, long arms, hands ×1.5), about 2.0 m. Crimson-lacquered dome pauldrons stand above a head sunk into an iron gorget; a barrel breastplate wider at the top; short thick legs in heavy boots; a long crimson tabard.
- **Weapon:** a two-handed maul, 1.4 m from the fist to the head, the head a 64 × 36 × 36 cm stone block banded in iron and capped in brass, cut with crimson runes on all four faces. Skinned to the right hand and laid along it; the left hand is solved onto the haft (14 §3.3).
- **Head:** the shared sculpted head, weathered: a square jaw, a heavy chin, small hard eyes, shaved bald, a braided beard, a scar over the left brow.
- **Gait (14 §3.6):** heavy: long contact, a deep bob, wide feet, the maul braced on his shoulder.

## Palette (14 §4)

Signature: **crimson** (0°). Neutrals are dark, because the arena's sun lights the top faces the gameplay camera sees.

| Slot | Colors |
|---|---|
| skin | weathered (0.70, 0.48, 0.36) |
| cloth | beard (0.30, 0.24, 0.20), gambeson (0.36, 0.28, 0.19), crimson tabard (0.55, 0.06, 0.06) and its dark hem (0.28, 0.035, 0.04), dark trousers (0.13, 0.11, 0.10), leather (0.31, 0.19, 0.10), the maul's stone (0.40, 0.38, 0.35) |
| metal | iron (0.30, 0.31, 0.35), dark iron (0.15, 0.16, 0.19), brass (0.62, 0.45, 0.18), the crimson lacquer of the pauldrons |
| emissive | the maul's runes (1.0, 0.22, 0.16) |
| accent | a band around the belt |

## Weight (14 §3)

- At rest he **leans on the maul**: its head on the ground in front of his right foot, both hands stacked on the haft. It stays planted while he breathes (IK).
- Every blow has a **cock** (the maul keeps going back as he coils), a **one-frame strike**, contact on `fire`, and an **impact hold** with the head left in the ground before he hauls it up.
- His feet are planted by IK in every pose; his runs are built by `library.gait` so his feet push off the ground.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1, idle_fidget_2, idle_ready | Leaning on the maul; rolls his neck; hauls it onto his shoulder, stretches, drops it head-first with a thud (it stays); ready: a two-handed batter's guard, the maul cocked over his right shoulder |
| run (340 u/s), run_fast (450 u/s) | Heavy gait, the maul on his shoulder, the left arm pumping |
| walk | Shared |
| attack_1, attack_2 | `fire` 12 (sim: 12). The overhead slam (rise, cock, strike, held impact, a haul up the right side); the sidelong sweep (wound right, cocked, swept across, held out left) |
| q | Cleave: wound right, one full turn with the maul out, `fire` 6 at mid-sweep, the head dragging to the ground and held |
| w | Second Wind: a chest thump and a deep breath (upper body, a pulse) |
| e_start, e_travel, e_land | Lunge: a deep coil and launch; airborne, maul high behind his head; a maul-first landing held a beat |
| r | Shockwave: up on his toes with the maul high, a held beat, cocked further, the slam on `fire` 14 (sim 13.5), the head left in the ground through the hard lock and a long heave, then hauled out |
| cast_utility | Shared |
| recall | Kneels, the maul upright and head-down, both hands on the pommel (`loop_in`/`loop_out`) |
| death, respawn, select | Staggers, drops to his knees with the maul's head striking the ground, pitches forward; rises from a crouch to his lean; hoists the maul overhead and roars, then plants it with a thud |
| cc_* | Shared |
| emote_taunt, emote_joke, emote_laugh, emote_dance | One hand on the maul: beckons; flexes; a belly laugh; a stomping two-step with the maul overhead |

## Effects and sounds (A6)

- **VFX** (`export/rook.vfx.ron`), all crimson (white → crimson → dark crimson):
  - slam (`attack.fire`): an overhead `slash` ending on the impact, a `flash`, `crack`s, stone chips;
  - Cleave: a ring `slash` around him, a `shock` to the area's edge, dust;
  - Second Wind: crimson-gold motes;
  - Lunge: dust at takeoff; on landing a crimson `pillar`, `crack`s, dust;
  - Shockwave: the biggest slam (a wide `slash`, `flash`, long `crack`s, a `shock`), a crimson orb down the line, crimson bursts on hits.
- **SFX** (`sounds.ron`), 13 sounds: swings, two maul impacts, Cleave's whoosh and thud, two chest thumps and a breath for Second Wind, the Lunge's launch and landing, Shockwave's rise, boom, roll and crack, a plate-drum emote.

## Definition of done

See [10 §10](../../../docs/design/10-characters-and-animation.md#10-pilots-vesper-and-rook-d50).
