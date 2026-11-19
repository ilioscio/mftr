# Bastion — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from the placeholder's "the tank reads as a wall"; v2 makes him a boulder crowned with ice.

## Identity

- **Role / kit** (`mftr-sim`): tank / engage. A melee punch (150 range, 0.65 attacks/s, 30% windup).
  - **Q Grapple:** a hooked chain thrown down a line that pulls its target in.
  - **W Bulwark:** a self shield, instant.
  - **E Tremor:** a slowing stomp around him.
  - **R Upheaval:** a delayed area that knocks up everyone in it (ultimate, hard CC).
- **Personality in three words:** steadfast, patient, immovable.
- **Shape (14 §2):** *a boulder*. Hulking proportions on the `biped` bones (`SHAPE` in `build.py`: low hips and short legs, the neck below the shoulder line, ×1.45 wide, ×1.35 deep, shoulders pushed out 16 cm, long arms, hands ×1.9), about 2.1 m to the shoulders' crystals' tips. Boulder shoulders stand above a craggy face (stern brow stones, slit eyes glowing cold, a nose stone, a heavy jaw with a beard of icicles); ice crystals grow from his back, his shoulders and his forearms; a crystal core is set in his chest; boulder fists; stumpy legs on boulder feet.
- **Weapon:** his fists. A chain wound round the right fist ends in the grapple's hook.
- **Gait (14 §3.6):** heavy: wide feet, a deep bob, a strong lean, his fists swinging like pendulums.

**The stone pass (2026-11-19).** Rebuilt from chipped stones (`rock_part`, the jungle monsters'
stones) in place of boxes and rings, to sit beside the redesigned champions: heaps of boulders on
the shoulders, interlocking stones on the chest, and arms and legs of stones around a core of ice
that shows through their gaps: the ice holds him together. The owner's notes on the first pass:
- **Ice reads as ice, not magic:** pale and lit like the stone (no glow), white rime on the
  upturned faces in place of the moss, icicles hanging under the shoulders and from the jaw.
  Only his eyes glow.
- **No forward crystal on the chest:** the cone that stuck out of it is gone; a chunk of
  frosted ice sits flush in the stone instead.
- **A face:** stern brow stones angled down to the middle, slit eyes under them, a nose stone,
  a heavy jaw with a beard of icicles, pushed forward from the shoulders.

Every shard and icicle is rooted in the stone it grows from (its first point sunk into it); the
icicle beard follows the jaw's curve under it. About 9,900 triangles (was 5,000); same bones, clips, effects and sounds.

## Palette (14 §4)

Signature: **ice blue** (205°). Warm granite neutrals, so the ice reads against them.

| Slot | Colors |
|---|---|
| cloth | granite in three shades (0.13–0.32); ice (0.50, 0.72, 0.86), deep ice (0.26, 0.48, 0.66), rime (0.82, 0.88, 0.92) |
| metal | iron chain, bands and the core's setting (0.11, 0.11, 0.13) |
| emissive | his eyes only (0.62, 0.90, 1.0) |
| accent | a band around his hips |

## Weight (14 §3)

- He stands hunched, fists hanging low in front of his thighs (IK), breathing slowly.
- Blows wind up with the whole body (the hips turn before the arm), land in 1–2 frames on `fire` and hold; slams and stomps leave his fists in the ground a beat.
- His feet are planted by IK; his runs are built by `library.gait`.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1, idle_fidget_2, idle_ready | Hunched, fists low; rolls his shoulders; pounds his fists together twice; a boxer's guard |
| run (335 u/s), run_fast (450 u/s) | Heavy gait, fists swinging |
| walk, cast_utility, cc_* | Shared |
| attack_1, attack_2 | `fire` 14 (sim 13.8): a right hook (wound back, cocked, held out); both fists hammered into the ground and held |
| q | Grapple: the hook swung back and cocked, hurled on `fire` 11 (sim 10.5), then hauled in hand over fist |
| w | Bulwark: instant, a pulse from `fire` 4: forearms slammed together across his face, held (upper body) |
| e | Tremor: rears up with the right knee high and both fists overhead, stamps and drives both fists into the ground on `fire` 8, held |
| r | Upheaval: fists into the ground on `fire` 8, held, then a heave upward as the earth erupts on the area's delay |
| recall, death, respawn, select | Kneels with both fists planted; his legs give, he drops onto his fists and topples forward; rises from the kneel; beats his chest and roars, arms wide |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Pounds the ground twice; a double-biceps flex, held; a rumbling laugh; a slow ground-shaking stomp |

## Effects and sounds

- **VFX** (`export/bastion.vfx.ron`), all ice (white → ice → deep ice):
  - blows: a `flash`, ice shards, frost `crack`s;
  - Grapple: frost off the fist, an ice-cored hook, a star and shards on the catch;
  - Bulwark: a frost `shock` around him, rising motes;
  - Tremor: `crack`s, a `shock` to the slow's edge, dust;
  - Upheaval: `crack`s and a star as his fists go in; ice `pillar`s, shards, a ring and dust when it erupts.
- **SFX** (`sounds.ron`), 13 sounds: grinding windups and falling-rock blows; the chain's rattle and the hook's clank; Bulwark's grind and shimmer; Tremor's stomp and rumble; Upheaval's build, slam and eruption; a stone grumble emote.
