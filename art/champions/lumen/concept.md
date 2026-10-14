# Lumen — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval). Designed without an owner brief, from the placeholder's "slender, with a glowing halo".

## Identity

- **Role / kit** (`mftr-sim`): enchanter. A ranged mote of light (550 range, 0.65 attacks/s, 20% windup).
  - **Q Mending Light:** heals an ally, instant.
  - **W Aegis:** shields an ally, instant.
  - **E Lull:** a slowing line.
  - **R Binding Halo:** a delayed area that roots (ultimate, hard CC).
- **Personality in three words:** serene, warm, unshakable.
- **Shape (14 §2):** *a wind-blown dress* (the first version, a bell robe to the ground, read as "a floating traffic cone with a stick"). Serene proportions on the `biped` bones (`SHAPE` in `build.py`: ×0.96, hands ×1.3), about 1.8 m. A fitted bodice laced in a gold V, a sky collar and sun stone, an underskirt cut to mid-shin so her legs and gold sandals show, and three long tails of the dress (down her back, at each hip) on cloth chains (`extra_skirt`, `extra_panel_l`, `extra_panel_r`) that always lift in a wind only she feels. Bare arms with gold armlets and open bell sleeves; a small halo; golden hair in a ponytail.
- **Weapon:** the lantern-staff, skinned to her right hand (`staff()` in `build.py`, `left` puts her left hand on it too): both hands on it at rest and for her strikes; the free hand sweeps wide for the support spells. Her projectiles leave from the lantern (`socket_projectile` placed there).
- **Gait (14 §3.6):** floating: she hovers 10 cm up and glides, no footfalls, a slow bob, the staff trailing in her right hand and the left arm swept back like a wing; the dress streams.

## Palette (14 §4)

Signature: **pale sun gold** (50°). The whites are darker than they look: the arena's sun lifts them.

| Slot | Colors |
|---|---|
| skin | fair (0.80, 0.62, 0.52) |
| cloth | white bodice, underskirt and tails (0.58, 0.56, 0.52), ivory (0.50, 0.46, 0.38), sky collar, sash and sleeve hems (0.20, 0.32, 0.50), pale sky tail ends (0.34, 0.48, 0.64), staff wood (0.36, 0.26, 0.16), golden hair |
| metal | gold hems, cage, circlet and clasps (0.70, 0.54, 0.22) |
| emissive | sun gold (1.0, 0.86, 0.42) and sun white (1.0, 0.96, 0.78): the halo, the lantern, the brooch |
| accent | a band at her waist |

## Weight (14 §3)

Her weight is in the staff and her arms: both hands on it at rest, upright at her right side; Lull is a two-handed thrust of the lantern with her whole floating body behind it, held while the light pours out; Binding Halo plants its foot on the ground and holds it there. Gestures are slow in, quick out, and held a beat; the dress tails follow through every move (14 §3.5) and drift in an authored wind at rest, so even her idle breathes.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (cups a mote of light, lets it rise), idle_fidget_2 (twirls the staff, catches it, bows her head), idle_ready | Both hands on the staff; a slow breath on the wind |
| run, run_fast | Light, upright |
| walk, cast_utility, attack_melee_alt, cc_* | Shared |
| attack_1, attack_2 | `fire` 9 (sim 9.2): the lantern thrust in both hands; a mote pushed from her left palm |
| q, w | Mending Light, Aegis: instant, upper-body pulses from `fire` 4: palms open toward the ally; arms crossed, then spread |
| e | Lull: drawn back over her right shoulder, then a two-handed thrust of the lantern on `fire` 8, held long |
| r | Binding Halo: the staff lifted high in both hands, then planted; `fire` 8 |
| recall, death, respawn, select | Kneels in prayer; sinks to her knees and folds aside; rises; opens her arms to the light |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Wags a finger; props up her "heavy" halo; laughs behind her hand; a slow twirl |

## Effects and sounds

- **VFX** (`export/lumen.vfx.ron`), all sun gold (white → gold → deep gold): soft golden orbs from the lantern and a gentle star for her motes; Mending Light rises in a `pillar`; Aegis presses out a `shock` of gold; Lull, her strike, bursts from the lantern (`flash`, `flare`, a short `slash`) and pours out as a `beam` that hits in a `flash`, a `burst` and a `shock`; Binding Halo's indicator burns with a `sigil`, the staff is planted in a `flash`, and the area closes in `pillar`s, a `shock` and a ring.
- **SFX** (`sounds.ron`), 9 sounds, bell-like and soft:
  - chimes for her motes;
  - a rising arpeggio for Mending Light;
  - a shimmering pad for Aegis;
  - Lull's falling tones;
  - Binding Halo's ringing bell;
  - a hummed emote.
