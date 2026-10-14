# Vesper — concept

Status: **rebuilt for [14](../../../docs/design/14-champion-art-direction.md)** (awaiting the owner's approval)

## Identity

- **Role / kit** (`mftr-sim`): marksman. Ranged bow attack (575 range, 0.8 attacks/s, 18% windup).
  - **Q Longshot:** a long, thin line shot (250 ms windup).
  - **W Shrapnel Charge:** a lobbed bomb that bursts after a delay (delayed area).
  - **E Tumble:** a short roll (dash).
  - **R Snare Net:** a wide thrown net that roots (hard CC line, ultimate).
- **Personality in three words:** patient, precise, wry.
- **Shape (14 §2):** *a huge recurve and a long one-sided cloak*. Athletic heroic proportions on the `biped` bones (`SHAPE` in `build.py`: ×1.05 wide, hands ×1.35), about 1.85 m. A 1.7 m recurve with teal-glowing tips, taller than her shoulders; a quiver across her back whose teal-fletched arrows rise over her right shoulder; a forest-green cloak with a teal hem hanging off her left shoulder on an `extra_cape` chain (3 bones); a leather pauldron on the bow shoulder; the hood down, a cowl. A feminine hourglass build.
- **Head:** the shared sculpted head, feminine and heroic-sized (×1.1): a narrower jaw, large green eyes, side-swept auburn bangs and a ponytail.
- **Gait (14 §3.5):** athletic, the bow steady and low in her left hand, the cloak streaming.

## Palette (14 §4)

Signature: **teal** (175°).

| Slot | Colors |
|---|---|
| skin | warm pale (0.80, 0.60, 0.48) |
| cloth | forest green cloak and cowl (0.10, 0.24, 0.17), dark forest trousers (0.06, 0.14, 0.10), cream shirt (0.70, 0.64, 0.50), leather (0.36, 0.22, 0.12 / 0.20, 0.12, 0.07), bow wood (0.30, 0.17, 0.08), teal cloth for the hem, sash and bracer bands (0.06, 0.42, 0.40), auburn hair |
| metal | the bow's silver collars and the quiver's bands (0.55, 0.58, 0.62) |
| emissive | teal (0.20, 0.95, 0.85): the bow's tips and the arrows' fletching |
| accent | a band at her belt |

## Weight (14 §3)

Every shot is a nock, a draw, a **held full draw** (her string hand at her jaw, an IK target), a one-frame release on `fire`, and a recoil that holds before she settles back to ready. The bow's orientation comes from the full-draw pose, so it stands vertical when she shoots. Feet planted by IK; side-on stance, left foot forward.

## Clip checklist (10 §5)

| Clip | Notes |
|---|---|
| idle, idle_fidget_1 (checks the string), idle_fidget_2 (scans the horizon), idle_ready | Bow held low in the left hand |
| run (330 u/s), run_fast (450 u/s) | Bow arm steady and low, right arm pumps, cloak streams back |
| walk | Shared |
| attack_1, attack_2 | `fire` 7 (sim: 6.75). Planted draw, a one-frame release, a recoil that cancels cleanly |
| attack_melee_alt | Shared (a bow-bash would replace it later) |
| q | Longshot: deeper draw, higher aim; `fire` 8 (sim 7.5) |
| w | Shrapnel Charge: overhand lob with the right hand; `fire` 8 |
| e_start, e_travel, e_land | Tumble: a dip and a dive; a full forward roll in place; out of it onto one knee, bow up, held |
| r | Snare Net: gathered in both hands and swung back, cocked, flung wide on `fire` 8, arms held open |
| cast_utility | Shared |
| recall | Kneels, bow planted, breathes (`loop_in`/`loop_out`) |
| death, respawn, select | Falls to her knees and sideways; rises from a crouch; showcase: a shot into the sky |
| cc_* | Shared |
| emote_taunt, emote_joke, emote_laugh, emote_dance | Two fingers at her eyes, then at you; balances an arrow on a fingertip; a dry chuckle; a bouncing two-step, the bow overhead |

## Definition of done

See [10 §10](../../../docs/design/10-characters-and-animation.md#10-pilots-vesper-and-rook-d50). A4a covers the model and every clip in game; A4b adds VFX, A4c SFX.

## Effects (14 §4)

`export/vesper.vfx.ron`, all teal (white → teal → deep teal): teal flares off the string and bright teal shafts; a white star and teal sparks on hits; Longshot's release adds a whip-crack `slash`; Shrapnel Charge bursts with a star, a `shock` to its edge and smoke; the net flies teal and snaps shut in a ring.
