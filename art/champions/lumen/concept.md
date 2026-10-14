# Lumen — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from the placeholder's "slender, with a glowing halo".

## Identity

- **Role / kit** (`mftr-sim`): enchanter. A ranged mote of light (550 range, 0.65 attacks/s, 20% windup).
  - **Q Mending Light:** heals an ally, instant.
  - **W Aegis:** shields an ally, instant.
  - **E Lull:** a slowing line.
  - **R Binding Halo:** a delayed area that roots (ultimate, hard CC).
- **Personality in three words:** serene, warm, unshakable.
- **Shape (14 §2):** *a bell*. Serene proportions on the `biped` bones (`SHAPE` in `build.py`: ×0.96, hands ×1.3), about 1.8 m. A long white robe flaring from her waist to the ground in two gold-hemmed tiers, a sky-edged ivory front panel and sash tails; a gold-edged mantle and a sun brooch; wide sleeves; a great sun-gold halo ring standing behind her head; a gold circlet; a tall lantern-staff in her left hand, its sun-bright core in a gold cage crowned by a crescent.
- **Weapon:** the lantern-staff, skinned to her left hand and held upright by IK (`staff()` in `build.py`).
- **Gait (14 §3.5):** floating: she hovers 10 cm up and glides, no footfalls, a slow bob, the staff and her free arm trailing.

## Palette (14 §4)

Signature: **pale sun gold** (50°). The whites are darker than they look: the arena's sun lifts them.

| Slot | Colors |
|---|---|
| skin | fair (0.80, 0.62, 0.52) |
| cloth | white robe (0.70, 0.68, 0.64), ivory panel, tier and mantle (0.60, 0.56, 0.48), sky trim, sash and cuffs (0.20, 0.32, 0.50), staff wood (0.36, 0.26, 0.16), golden hair |
| metal | gold hems, cage, circlet and clasps (0.70, 0.54, 0.22) |
| emissive | sun gold (1.0, 0.86, 0.42) and sun white (1.0, 0.96, 0.78): the halo, the lantern, the brooch |
| accent | a band at her waist |

## Weight (14 §3)

Her weight is in the staff: held upright by IK at rest, swung forward to point the lantern, planted with its foot on the ground for Binding Halo and held there. Gestures are slow in, quick out, and held a beat.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (cups a mote of light, lets it rise), idle_fidget_2 (tucks back a strand of hair), idle_ready | Hands folded before her |
| run, run_fast | Light, upright |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | `fire` 9 (sim 9.2): a flick from the right palm; a backhand from the left |
| q, w | Mending Light, Aegis: instant, upper-body pulses from `fire` 4: palms open toward the ally; arms crossed, then spread |
| e | Lull: a slow sweep of the right arm; `fire` 8 |
| r | Binding Halo: arms rise to draw a circle overhead and sweep it down; `fire` 8 |
| recall, death, respawn, select | Kneels in prayer; sinks to her knees and folds aside; rises; opens her arms to the light |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Wags a finger; props up her "heavy" halo; laughs behind her hand; a slow twirl |

## Effects and sounds

- **VFX** (`export/lumen.vfx.ron`), all sun gold (white → gold → deep gold): soft golden orbs and a gentle star for her motes; Mending Light rises in a `pillar`; Aegis presses out a `shock` of gold; Lull sweeps a soft `slash` and drifts as an orb; Binding Halo flares (`flash`) as the staff is planted and closes on the area in `pillar`s, a `shock` and a ring.
- **SFX** (`sounds.ron`), 9 sounds, bell-like and soft:
  - chimes for her motes;
  - a rising arpeggio for Mending Light;
  - a shimmering pad for Aegis;
  - Lull's falling tones;
  - Binding Halo's ringing bell;
  - a hummed emote.
