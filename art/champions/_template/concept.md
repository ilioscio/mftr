# <Champion> — concept

Status: draft | approved

## Identity

- **Role / kit:** (from `mftr-sim`: attack style, Q W E R in one line each)
- **Personality in three words:**
- **Silhouette class:** (what reads at the gameplay camera: weapon shape, head shape, mass)
- **Archetype:** biped | biped_large | biped_small | quadruped | floater | custom

## Palette (≤ 12 colors, 10 §2)

| Slot | Colors |
|---|---|
| skin | |
| cloth | |
| metal | |
| emissive | |
| accent | (team-tinted region: where it sits, and that it reads from above) |

## Clip checklist (10 §5)

Required for every champion, plus the kit's ability clips (`mftr-tools pack validate` lists any missing).

| Clip | Status | Notes |
|---|---|---|
| idle, idle_fidget_1, idle_fidget_2, idle_ready | | |
| run, run_fast, walk (shared) | | |
| attack_1, attack_2 (`fire` = sim windup) | | |
| attack_melee_alt (ranged only, shared) | | |
| q / w / e / r (or `<slot>_start/_travel/_land`, stages) | | |
| cast_utility (shared) | | |
| recall (`loop_in`/`loop_out`), death, respawn, select | | |
| cc_* (shared; override if the personality calls for it) | | |
| emote_taunt, emote_joke, emote_laugh, emote_dance | | |

## Definition of done

See [10 §10](../../../docs/design/10-characters-and-animation.md#10-pilots-vesper-and-rook-d50).
