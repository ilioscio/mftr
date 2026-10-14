# Marrow — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from her kit (a battlemage who drains, roots and opens a pit of bone) and her name.

## Identity

- **Role / kit** (`mftr-sim`): battlemage. A ranged bolt (475 range, 0.65 attacks/s, 22% windup).
  - **Q Siphon:** a draining nova around her.
  - **W Grasping Bones:** a rooting line.
  - **E Grave Pact:** a self heal (instant).
  - **R Ossuary:** a large delayed area that slows.
- **Personality in three words:** sly, hungry, unhurried.
- **Shape (14 §2):** *a hooded crone*. Bent proportions on the `biped` bones (`SHAPE` in `build.py`: ×0.95 wide, arms ×1.15, hands ×1.4), about 1.8 m, always hunched. A deep plum hood set back so her face shows in its opening, a heavy rolled rim framing it, the peak drooping down her back and a ragged cowl over her shoulders; a ribcage corset over her robe, a ragged skirt of tattered strips, a grave-grey shawl on an `extra_cloak` chain, a skull lantern at her left hip. (The first version fanned bone spikes up behind the hood and held up bone-clawed hands: they read as whiskers and as showing her nails.)
- **Weapon:** a crooked staff of black wood in her right hand, crowned with a horned skull whose eye sockets burn magenta, bone charms hanging under it; she leans on it at rest with both hands. Her bolts and the bones leave from the skull's jaws (`socket_projectile` placed there).
- **Head:** the shared sculpted head, aged, grey-haired, magenta-eyed, mostly hidden by the hood.
- **Gait (14 §3.6):** unhurried: a hunched, stalking stride, the staff swung like a walking stick, the free hand open.

## Palette (14 §4)

Signature: **magenta** (320°).

| Slot | Colors |
|---|---|
| skin | grave-pale (0.56, 0.56, 0.50) |
| cloth | plum robe and hood (0.24, 0.08, 0.20 / 0.12, 0.04, 0.11), grave grey shawl and rags (0.24, 0.24, 0.26), bone (0.64, 0.60, 0.50 / 0.42, 0.38, 0.30), grey hair |
| metal | the lantern's iron chain (0.18, 0.18, 0.20) |
| emissive | magenta (1.0, 0.24, 0.72): the lantern's eyes |
| accent | a band at her belt |

## Weight (14 §3)

Bolts are jabbed from the skull (drawn back, cocked, one frame, held out) or swept round flat and stopped dead on the target. Siphon flings the staff up and her free arm wide, then drags the life in, the hand clenched hard to her chest, and holds. Grasping Bones drives the staff foot-first into the ground ahead and holds it there while the bones run. Grave Pact thrusts the staff high, the skull blazing, her head thrown back. Ossuary lifts the staff in both hands and slams it into the ground; she hangs on it while the boneyard rises.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (taps the skull lantern), idle_fidget_2 (cranes her neck out of the hood, sniffing), idle_ready | Leaning on the staff, both hands on it |
| run, run_fast | Hunched, the staff swung like a walking stick, the shawl trailing |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | A clawed backhand and a palm thrust; `fire` 10 (sim 10.2) |
| q | Siphon: claws flung wide, then dragged to her chest; `fire` 6 |
| w | Grasping Bones: drops to a knee and drives a claw into the ground; `fire` 9 |
| e | Grave Pact: instant, upper body, a hand clutched to her chest, head thrown back; `fire` 4 |
| r | Ossuary: both arms raised, then thrust toward the far ground; `fire` 8 |
| recall, death, respawn, select | Kneels with palms flat on the ground; falls; claws her way up; rises from a hunch with a crooked smile |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; snaps a rib like a wishbone; laughs; a puppet-jointed shuffle |

## Effects and sounds

- **VFX** (`export/marrow.vfx.ron`), all magenta (white → magenta → deep magenta) with bone: sickly orbs off her claw; Siphon rings her with a `slash` and a `shock` and draws motes in; Grasping Bones `crack`s the ground and flies as a bone shard; Grave Pact flares in her ribs; Ossuary opens a boneyard (`crack`s, a `shock`, a `pillar` of magenta and bone).
- **SFX** (`sounds.ron`), 9 sounds:
  - a hollow hiss and a dry crack;
  - Siphon's inward drone;
  - Grasping Bones' rattle and snap;
  - Grave Pact's chord;
  - Ossuary's groan and eruption;
  - a cackle.
