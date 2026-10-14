# Marrow — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from her kit (a battlemage who drains, roots and opens a pit of bone) and her name.

## Identity

- **Role / kit** (`mftr-sim`): battlemage. A ranged bolt (475 range, 0.65 attacks/s, 22% windup).
  - **Q Siphon:** a draining nova around her.
  - **W Grasping Bones:** a rooting line.
  - **E Grave Pact:** a self heal (instant).
  - **R Ossuary:** a large delayed area that slows.
- **Personality in three words:** sly, hungry, unhurried.
- **Shape (14 §2):** *a hooded crone*. Bent proportions on the `biped` bones (`SHAPE` in `build.py`: ×0.95 wide, arms ×1.15, hands ×1.4), about 1.8 m, always hunched. A round, heavy plum hood whose peak droops forward over her face; a high collar of nine curved bone spikes fanning up behind it; a ribcage corset (a bone sternum and five pairs of ribs) over her robe; ragged sleeves with bone bangles over thin bare wrists; long bone claws on her fingertips; a robe to the shins with tattered strips hanging below; a grave-grey shawl on an `extra_cloak` chain (3 bones); a skull lantern with magenta eyes at her hip.
- **Weapon:** her claws.
- **Head:** the shared sculpted head, aged, grey-haired, magenta-eyed, mostly hidden by the hood.
- **Gait (14 §3.5):** unhurried: a hunched, stalking stride, claws forward.

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

Bolts are flung with a clawed swipe: drawn back and cocked, one frame, held out with the claws spread. Siphon flings her arms wide, then drags the life in, fists pulled hard to her chest and held. Grasping Bones drives a claw into the ground and leaves it there. Ossuary raises both claws, cocks, sweeps them down and out and holds low while she cackles. Feet planted by IK.

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

- **VFX** (`export/marrow.vfx.ron`), all magenta (white → magenta → deep magenta) with bone: sickly orbs off her claw; Siphon rings her with a `slash` and a `shock` and draws motes in; Grasping Bones `crack`s the ground and flies as a bone shard; Grave Pact flares in her ribs; Ossuary opens a boneyard (`crack`s, a `shock`, a `pillar` of magenta and bone).
- **SFX** (`sounds.ron`), 9 sounds:
  - a hollow hiss and a dry crack;
  - Siphon's inward drone;
  - Grasping Bones' rattle and snap;
  - Grave Pact's chord;
  - Ossuary's groan and eruption;
  - a cackle.
