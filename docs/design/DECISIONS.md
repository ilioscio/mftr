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
| D10 | 2026-10-05 | **Width-based reaction budget:** `reaction_time(d_class) ≥ t_human + t_net + (r_proj + 65)/335`, with `t_human` = 0.25 s, `t_net` = 0.17 s, `d_class` = 80% / 90% / 100% of max range for hard CC / burst / poke. *(Replaces the earlier flat 0.45 s, which ignored movement time.)* | Geometry-correct; reproduces the reference game's dodgeability band (Mystic Shot passes at max range with little margin, see [R01 §4](reference/R01-video-ezreal-flash-barrier-q.md#4-mystic-shot-reference-line-skillshot)) | Accepted |
| D12 | 2026-10-05 | **Own projectiles use Option B:** spawn on T_input, blend toward T_interp over the flight | Impacts line up with how enemies are drawn; settle by blind A/B in the M1 duel sandbox | Accepted (to validate) |
| D13 | 2026-10-05 | **Camera:** perspective, ~56° pitch, ~45° vertical FOV, ~2,100 u visible depth at default zoom, asymmetric framing (~1,360 u up / ~750 u down from center) | Measured from a reference capture ([R01 §1](reference/R01-video-ezreal-flash-barrier-q.md#1-camera-the-most-useful-result)); matches players' spatial intuition | Accepted |
| D14 | 2026-10-05 | **Skillshot VFX: slim bright core plus a faint, always-visible full-width edge sheath** marking the true hitbox | Reference VFX is ~half the hitbox width ([R01 §4](reference/R01-video-ezreal-flash-barrier-q.md#4-mystic-shot-reference-line-skillshot)); keeps the familiar look while staying honest | Proposed |
| D11 | 2026-10-05 | **Unit collision like the reference game:** small collision radii separate from gameplay radii, solid (no shoving), so clumped minions block champions. Dashes, blinks and ghosted units ignore it | Minion-block and body-block are familiar skill expression; see [01 §4](01-gameplay.md#4-minions) | Accepted |
| D15 | 2026-10-05 | **M0 networking runs on plain `std` UDP behind transport-agnostic cores** (`ServerCore`, `ClientSession` take bytes + timestamps). No external crates in the sim/net/client/server crates yet | Keeps the core dependency-free and testable in virtual time; the secure-transport choice (Q3) happens with the lobby/token work before M2 | Accepted |
| D16 | 2026-10-05 | **Toolchain baseline:** Rust 2024 edition, core crates MSRV 1.85; `mftr-gdext` on godot-rust 0.5.5 (MSRV 1.94) targeting the Godot 4.7 API; client renderer "Mobile" for now | Latest stable bindings for the installed Godot 4.7.2; core stays buildable on older toolchains | Accepted |
| D17 | 2026-10-05 | **Late-command policy (M0):** the server applies late commands at the next tick start (never rewinds); the client re-sends young unacked commands every 10 ms, and the margin loop targets `min(window min, mean − 2.6σ)` ≥ 2 ms lead, changed only by time dilation (≤ 10% fast / 1% slow) | Measured in the Netcode Lab: late commands 0.20% and 0.48 corrections > 15 u per player-minute at 120 ms / 20 ms / 2% ([08 M0 status](08-roadmap.md#m0-status-2026-10-05)) | Accepted |

## Open questions

1. **Project name:** keep "MFTR" as the public name or only as a codename? And our own term for "champions"?
2. **Numeric determinism:** strict-`f32` discipline (current plan) vs. fixed-point. Decide after the M0 cross-platform hash test.
3. **Transport:** `renet` + netcode protocol vs. `quinn` (QUIC) vs. our own AEAD layer on the existing UDP code. Decide with the lobby/token work, before M2 (see D15).
4. **ECS vs. arenas** for `mftr-sim` (`bevy_ecs` standalone / `hecs` / hand-rolled). M0 uses a plain `Vec<Unit>`; revisit when minions arrive (M2).
11. **Bounded own-movement rewind for late commands?** If a late *move* command's lateness window touched no interactions (hits, CC, collisions), the server could apply it on time by re-simulating only that unit, which would remove most remaining corrections. It's a narrow, input-favoring form of lag compensation, so it needs a careful look at abuse (e.g. deliberately delaying inputs) before M1.
5. **Mod scripting language** (post-M3): Rhai, Lua, or WASM?
6. **Cosmetics policy:** community skins allowed? Under what readability review? Client-side "show default models" toggle?
7. **Funding:** donations (Open Collective / Liberapay), grants, or optional paid convenience (e.g. hosted servers), with nothing that affects gameplay.
8. **Tick rate:** stay at 30 Hz or test 60 Hz in M1? (The plan is to measure, not guess.)
9. **Godot renderer default:** Mobile vs. Forward+ on desktop. Measure on the minimum-spec iGPU in M1.
10. **Tuning baseline:** how closely to match reference-game numbers (stats, gold values, timers) for the first playable, before diverging.
