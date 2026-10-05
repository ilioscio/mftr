# MFTR Design Document

Status: **Draft v0.1** — 2026-10-05

This is a living document. Each file covers one area. Record decisions in [DECISIONS.md](DECISIONS.md) so we don't re-argue them; the unresolved questions are listed at the bottom of that file.

| # | Doc | What it covers |
|---|-----|----------------|
| 00 | [Vision & Pillars](00-vision.md) | Why MFTR exists, design pillars, non-goals |
| 01 | [Core Gameplay](01-gameplay.md) | Match flow, map, minions, economy, objectives, roles, champions, controls |
| 02 | [Combat Math](02-combat-math.md) | Stats, damage pipeline, resistances, penetration, haste, CC |
| 03 | [Netcode](03-netcode.md) | Server authority, prediction, projectile timelines, fog culling, replays |
| 03a | [Netcode: Time, Prediction & Hit Resolution](03a-netcode-time-and-prediction.md) | Timelines math, sub-tick commands, collision proxies (minion block), swept hits, display policy, reaction budget |
| 03b | [Netcode: Wire Protocol](03b-netcode-wire-protocol.md) | Packets, channels, commands, snapshots, events, bandwidth |
| 04 | [Architecture](04-architecture.md) | Rust sim core, Godot client, crates, ability/effect system, tooling |
| 05 | [Art & Assets](05-art-and-assets.md) | Visual style, readability rules, procedural materials, size budgets |
| 06 | [Game Modes & Augments](06-modes-and-augments.md) | Normal, ARAM, Mayhem augments, Arena, rotating modes |
| 07 | [Hosting, Identity & Trust](07-hosting-and-trust.md) | Self-hosting, lobby, identity, anti-cheat stance, federation path |
| 08 | [Roadmap](08-roadmap.md) | Milestones with exit criteria |
| 09 | [Naming & Legal](09-naming-and-legal.md) | Legally distinct names, licensing, IP hygiene |
| — | [Decisions & Open Questions](DECISIONS.md) | Decision log and unresolved questions |

## Conventions

- **Game units (u):** distances use a MOBA-familiar scale. A typical champion moves at ~330 u/s, and the 3-lane map is ~15,000 u across. The client renders 1 u = 1 cm (0.01 Godot meters).
- **Time:** the simulation runs on fixed ticks (30 Hz, 1 tick ≈ 33.3 ms). Design values are written in seconds and converted to ticks.
- **Tuning numbers** marked *(start)* are first-pass values borrowed from genre convention. They will change in playtesting.
- Reference-game names appear only in "inspired by" notes. Our own placeholder names are in [09](09-naming-and-legal.md).
