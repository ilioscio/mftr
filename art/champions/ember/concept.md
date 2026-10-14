# Ember — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief while the owner tested Rook; everything here is open to change.

## Identity

- **Role / kit** (`mftr-sim`): burst mage. A ranged fire bolt (525 range, 0.65 attacks/s, 20% windup).
  - **Q Ember Lance:** a long, fast line of fire (250 ms windup).
  - **W Cinder Bloom:** a delayed area that erupts.
  - **E Flicker:** a short blink.
  - **R Binding Sigil:** a thrown sigil that stuns (hard-CC line, ultimate).
- **Personality in three words:** quick, cocky, volatile.
- **Shape (14 §2):** *a flame*. Slim, top-heavy proportions on the `biped` bones (`SHAPE` in `build.py`: ×0.97, shoulders out 4 cm, arms ×1.04, hands ×1.3), about 1.8 m, 2.2 m to the tips of his hair. Burning hair sweeping up and back from his crown; a tall flared collar of soot-black petals lined with ember red behind his head; a charcoal coat with lapels over an ember-red lining, its tails on an `extra_coat` chain (3 bones); bell sleeves flaring wide at the wrists; fire burning in both palms; slim legs in pointed boots.
- **Weapon:** his hands.
- **Gait (14 §3.6):** light: a short contact, a long airborne phase, the coat tails streaming.

## Palette (14 §4)

Signature: **flame orange** (25°).

| Slot | Colors |
|---|---|
| skin | warm (0.76, 0.54, 0.40) |
| cloth | charcoal coat (0.11, 0.10, 0.10), soot black (0.05, 0.045, 0.045), ember red lining, petals, cuffs and the coat's hem (0.55, 0.10, 0.04) |
| metal | gold buttons, brooch, hem and knee plates (0.60, 0.40, 0.12) |
| emissive | flame (1.0, 0.50, 0.10) to hot yellow at the roots (1.0, 0.82, 0.30): his hair and the fire in his palms |
| accent | a band at his belt |

## Weight (14 §3)

Every cast coils (the palm drawn back by his ear, or both palms to his hip for the lance), cocks, thrusts in one frame on `fire`, and holds; Ember Lance's recoil rocks him back a step. Feet planted by IK.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (bounces a flame on his palm), idle_fidget_2 (snaps and blows on his fingers), idle_ready | Hands relaxed, coat tails settling |
| run, run_fast | The shared run poses, with coat tails streaming back |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | `fire` 9 (sim 9.2): an overhand throw; an underhand flick |
| q | Ember Lance: both hands draw back and thrust; `fire` 8 (sim 7.5) |
| w | Cinder Bloom: hands rise, then push down toward the far ground; `fire` 8 |
| e | Flicker: a blink, so it plays as a pulse from `fire` 3: he reappears crouched and rises |
| r | Binding Sigil: the right hand traces a circle, then thrusts; `fire` 9 (sim 9) |
| recall | Kneels with palms together, a flame between them |
| death, respawn, select | Falls back and to the side; rises; arms flung wide as both hands catch fire |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; blows out his fingertip like a candle; laughs; a quick spinning dance |

## Effects and sounds

- **VFX** (`export/ember.vfx.ron`), all flame (white → orange → ember red): flares and hot orbs for his bolts, a white star and embers on hits; Ember Lance's release flares with a whip-crack `slash` and flies white-hot; Cinder Bloom erupts in a fire `pillar` with a `shock` to its edge; Flicker vanishes in a puff and a short column; the sigil flies burning and binds in a ring of fire.
- **SFX** (`sounds.ron`), 14 sounds:
  - a crackle as each flame catches, whooshing and popping fireballs;
  - the Lance's roar and crack;
  - Cinder Bloom's build and burst;
  - a flick of air for Flicker;
  - the Sigil's chime, whirr and bind;
  - a fizzle emote.

## Definition of done

See [10 §10](../../../docs/design/10-characters-and-animation.md#10-pilots-vesper-and-rook-d50).
