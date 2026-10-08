# Bastion — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Designed without an owner brief, from the placeholder's "the tank reads as a wall".

## Identity

- **Role / kit** (`mftr-sim`): tank / engage. A melee punch (150 range, 0.65 attacks/s, 30% windup).
  - **Q Grapple:** a hooked chain thrown down a line that pulls its target in.
  - **W Bulwark:** a self shield, instant.
  - **E Tremor:** a slowing stomp around him.
  - **R Upheaval:** a delayed area that knocks up everyone in it (ultimate, hard CC).
- **Personality in three words:** steadfast, patient, immovable.
- **Silhouette class:** *stone guardian*.
  - A golem of rounded masonry, the widest silhouette on the roster: the `biped` bones in the `large` shape, ×1.08 and ×1.12 wider again.
  - Boulder shoulders with moss.
  - A stone slab shield on his left forearm, and a chain wound round his right fist ending in a hook.
  - No face: a rough rock head set low between the shoulders, with a glowing rune visor under a heavy brow.
- **Archetype:** `biped` bones in the `large` shape, no extra bones; the chain and hook are skinned to the right hand.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| cloth | slate stone in three shades (0.28–0.50), moss (0.28, 0.40, 0.20) |
| metal | iron bands, belt and chain (0.26, 0.27, 0.30) |
| emissive | a pale rune glow in the visor, on his chest and along the forearms (0.55, 0.88, 1.0) |
| accent | a hanging banner, the emblem on his shield, and bands of paint on the shoulders |

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (grinds his head side to side), idle_fidget_2 (raps his fist on his shield), idle_ready | A slow, heavy breath |
| run, run_fast | Shorter strides, a forward lean |
| walk, cast_utility, cc_* | Shared |
| attack_1, attack_2 | `fire` 14 (sim 13.8): a hammer-fist; a shield bash |
| q | Grapple: the hook swung back, flung forward on `fire` 11 (sim 10.5), then hauled in |
| w | Bulwark: instant, a pulse from `fire` 4: the shield slams up in front (upper body) |
| e | Tremor: knee raised high, a stomp on `fire` 8 |
| r | Upheaval: both fists raised and driven into the ground on `fire` 8; the eruption follows on the area's delay |
| recall, death, respawn, select | Kneels still; topples forward like a felled wall; rises; strikes his raised shield |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; plays statue; a rumbling laugh; a ground-shaking stomp |

## Effects and sounds

- **VFX** (`export/bastion.vfx.ron`):
  - stone chips and dust on his blows;
  - an iron chain for Grapple;
  - a rune-blue shimmer for Bulwark;
  - Tremor's dust ring on its true radius;
  - Upheaval's eruption of rock.
- **SFX** (`sounds.ron`), 13 sounds:
  - grinding windups and falling-rock blows;
  - the chain's rattle and the hook's clank;
  - Bulwark's grind and shimmer;
  - Tremor's stomp and rumble;
  - Upheaval's build, slam and eruption;
  - a stone grumble emote.
