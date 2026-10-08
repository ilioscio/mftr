# Shade — concept

Status: **draft** (awaiting the owner's approval, 10 §10). Designed without an owner brief, from the placeholder's "a sharp blade of a body with two forward blades".

## Identity

- **Role / kit** (`mftr-sim`): assassin. A quick melee slash (125 range, 0.72 attacks/s, 25% windup).
  - **Q Shadow Step:** a lunge onto a target that strikes on arrival.
  - **W Fan of Blades:** a slowing nova.
  - **E Veil Step:** a short dash out.
  - **R Execution:** a lunging finisher (ultimate).
- **Personality in three words:** quiet, precise, cold.
- **Silhouette class:** *shadow assassin*.
  - Lean and fitted, in violet-black leathers, with a violet sash across the chest.
  - A curved dagger in each hand.
  - A long scarf in the team color on an `extra_scarf` chain (3 bones) that streams behind him: the team read, and his motion trail.
- **Head:** the shared sculpted head, lean, with short dark hair and violet eyes. A dark mask covers his face from the nose down.
- **Archetype:** `biped` v1, plus the `extra_scarf` chain. The daggers lie along the hand bones, so each pose aims them.

## Palette (10 §2)

| Slot | Colors |
|---|---|
| skin | (0.78, 0.6, 0.5) |
| cloth | violet-black leathers (0.17, 0.13, 0.2 / 0.1, 0.08, 0.12), a violet sash and cuffs (0.32, 0.22, 0.42), wraps (0.24, 0.2, 0.26), dark hair |
| metal | steel blades (0.74, 0.76, 0.82) |
| emissive | none: he hides in the dark |
| accent | the scarf, wound at the neck and streaming behind |

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (spins a dagger), idle_fidget_2 (glances over each shoulder), idle_ready | Daggers low, a low crouch when ready |
| run, run_fast | Blades held back, the scarf streaming |
| walk, cast_utility, cc_* | Shared |
| attack_1, attack_2 | `fire` 10 (sim 10.4): a cross-body slash with each blade |
| q_start, q_travel, q_land | Shadow Step: a low lunge with both blades leading, a strike on landing |
| w | Fan of Blades: a crouched spin, arms flung wide; `fire` 4 (sim 4.5) |
| e_start, e_travel, e_land | Veil Step: a low smoky glide |
| r_start, r_travel, r_land | Execution: a leap with both blades raised, driven down on landing |
| recall, death, respawn, select | Kneels; falls aside; rises; twirls and crosses both blades before his masked face |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Beckons; juggles his daggers; a silent laugh; a low swaying dance |

## Effects and sounds

- **VFX** (`export/shade.vfx.ron`):
  - violet-white glints on his slashes;
  - smoke for Veil Step;
  - Fan of Blades' ring of glinting steel on its true radius;
  - bursts where Shadow Step and Execution strike.
- **SFX** (`sounds.ron`), 10 sounds:
  - swishes and thin metallic slices;
  - a hushed rush of air for each step;
  - a whirling ring of steel for Fan of Blades;
  - Execution's rising hiss and ringing strike;
  - a blade-spin emote.
