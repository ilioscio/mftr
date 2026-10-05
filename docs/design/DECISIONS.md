# Decisions & Open Questions

## Decision log

| ID | Date | Decision | Rationale | Status |
|---|---|---|---|---|
| D1 | 2026-10-05 | **Simulation core in Rust** (`mftr-sim`), engine-independent. Godot client links it via godot-rust/gdext. The dedicated server is a standalone Rust binary with no Godot. | Cheap self-hosting, headless testing, deterministic replays, and client prediction running the *same* code as the server | Accepted |
| D2 | 2026-10-05 | **Milestone order:** Duel Sandbox → ARAM → Mayhem → full 3-lane map | Prove netcode feel first; ARAM is the smallest complete MOBA; Mayhem reuses the ARAM map | Accepted |
| D3 | 2026-10-05 | **Self-host first, federate later.** Keypair identity and signed match records from day one | Ship playable sooner without closing the door on federation | Accepted |
| D4 | 2026-10-05 | **Art: stylized low-poly with procedural noise shaders**, near-zero bitmap textures | Readability + tiny download | Accepted |
| D5 | 2026-10-05 | **Server-authoritative, 30 Hz**, own-champion prediction only, enemy projectiles rendered on the input timeline, no shooter-side lag compensation (favor the defender) | Honest dodging; see [03](03-netcode.md), [03a](03a-netcode-time-and-prediction.md) | Proposed |
| D6 | 2026-10-05 | **Server-side fog-of-war culling**; no kernel anti-cheat | Removes map hacks by construction; respects users | Proposed |
| D7 | 2026-10-05 | **AGPL-3.0-or-later code, CC BY-SA 4.0 content, DCO** | Keeps hosted and federated forks open; see [09](09-naming-and-legal.md) | Accepted |
| D8 | 2026-10-05 | **Abilities, items, augments and modes as data (RON)** with Rust "behaviors" as escape hatch | Moddability; makes augments generic | Proposed |
| D9 | 2026-10-05 | **Familiar to players of the reference game, with fully original IP.** Shared systems and conventions; original names, characters, kits, art, audio and text; follow copyright law | Lowest learning curve for the target audience, without legal or ethical baggage | Accepted |
| D10 | 2026-10-05 | **Reaction budget starts at 0.45 s for hard-CC skillshots** (burst 0.40 s, poke 0.30 s), measured at 75% of max range | Starting point to validate in M1 playtests | Accepted — **under review**: [03a §8](03a-netcode-time-and-prediction.md#8-reaction-budget-what-the-player-actually-gets) shows a flat 0.45 s ignores movement time; proposes a geometry-derived rule |
| D11 | 2026-10-05 | **Unit collision like the reference game:** small collision radii separate from gameplay radii, solid (no shoving), so clumped minions block champions. Dashes, blinks and ghosted units ignore it | Minion-block and body-block are familiar skill expression; see [01 §4](01-gameplay.md#4-minions) | Accepted |

## Open questions

1. **Project name:** keep "MFTR" as the public name or only as a codename? And our own term for "champions"?
2. **Numeric determinism:** strict-`f32` discipline (current plan) vs. fixed-point. Decide after the M0 cross-platform hash test.
3. **Transport:** `renet` + netcode protocol vs. `quinn` (QUIC). Decide in M0 by prototype.
4. **ECS vs. arenas** for `mftr-sim` (`bevy_ecs` standalone / `hecs` / hand-rolled).
5. **Mod scripting language** (post-M3): Rhai, Lua, or WASM?
6. **Cosmetics policy:** community skins allowed? Under what readability review? Client-side "show default models" toggle?
7. **Funding:** donations (Open Collective / Liberapay), grants, or optional paid convenience (e.g. hosted servers), with nothing that affects gameplay.
8. **Tick rate:** stay at 30 Hz or test 60 Hz in M1? (The plan is to measure, not guess.)
9. **Godot renderer default:** Mobile vs. Forward+ on desktop. Measure on the minimum-spec iGPU in M1.
10. **Tuning baseline:** how closely to match reference-game numbers (stats, gold values, timers) for the first playable, before diverging.
