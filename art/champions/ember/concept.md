# Ember — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Designed without an owner brief while the owner tested Rook; everything here is open to change.

## Identity

- **Role / kit** (`mftr-sim`): burst mage. A ranged fire bolt (525 range, 0.65 attacks/s, 20% windup).
  - **Q Ember Lance:** a long, fast line of fire (250 ms windup).
  - **W Cinder Bloom:** a delayed area that erupts.
  - **E Flicker:** a short blink.
  - **R Binding Sigil:** a thrown sigil that stuns (hard-CC line, ultimate).
- **Personality in three words:** quick, cocky, volatile.
- **Silhouette class:** *pyromancer*.
  - A lean figure in a charcoal long coat.
  - Coat tails trail behind him (an `extra_coat` chain, 2 bones).
  - A tall ember-orange collar wraps the back of his neck and flares up behind his head; it reads from the gameplay camera.
  - No staff or wand: fire comes from his bare hands, with glowing ember cuffs at the wrists.
- **Head:** the shared sculpted head, young and lean, with short dark hair and amber eyes.
- **Archetype:** `biped` v1, plus the `extra_coat` chain.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | (0.80, 0.60, 0.47) |
| cloth | charcoal coat (0.17, 0.15, 0.15), ember collar and tail lining (0.78, 0.36, 0.14), trousers (0.20, 0.17, 0.16), leather belt (0.34, 0.21, 0.13), boots (0.20, 0.13, 0.09), dark hair (0.15, 0.10, 0.09) |
| metal | gold trim on the coat's front edges and the buckle (0.82, 0.62, 0.28) |
| emissive | glowing ember cuffs and a rune on his chest (1.0, 0.55, 0.15) |
| accent | a sash over the belt and bands on the forearms |

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

- **VFX** (`export/ember.vfx.ron`):
  - fireballs (flare, orb, burst);
  - a white-hot lance;
  - Cinder Bloom's embers, ring and ash on its true radius;
  - a flash where he Flickers in;
  - a spinning gold sigil.
- **SFX** (`sounds.ron`), 14 sounds:
  - a crackle as each flame catches, whooshing and popping fireballs;
  - the Lance's roar and crack;
  - Cinder Bloom's build and burst;
  - a flick of air for Flicker;
  - the Sigil's chime, whirr and bind;
  - a fizzle emote.

## Definition of done

See [10 §10](../../../docs/design/10-characters-and-animation.md#10-pilots-vesper-and-rook-d50).
