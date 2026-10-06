# 08 — Roadmap

No dates yet: milestones finish when their **exit criteria** pass. The order is chosen to prove the hardest, most differentiating thing first (netcode feel), then grow scope from the smallest complete game (ARAM) to the full one.

## M0 — Foundations
**Goal:** the skeleton every later milestone builds on.
- Cargo workspace and Godot project. CI on Linux, Windows and macOS. License files.
- `mftr-sim` fixed-tick loop, entity storage, seeded RNG, state hashing.
- `mftr-net` prototype on the chosen transport, link conditioner, clock sync.
- `mftr-server` headless binary; `mftr-gdext` with a `MatchClient` node; a capsule "champion" moving on a flat plane via right-click with prediction.
- Golden tests for [02](02-combat-math.md) formulas.
- Transport decision (renet/netcode vs. quinn) and ECS decision, recorded in DECISIONS.

**Exit:** two clients on different machines move champions with the link conditioner at 120 ms / 20 ms jitter / 2% loss, own-champion correction < 15 u, and the cross-platform hash check passes.

### M0 status (2026-10-05)

| Item | Status |
|---|---|
| Cargo workspace, Godot project, CI (Linux / Windows / macOS-arm64), AGPL license | ✅ |
| `mftr-sim`: fixed 30 Hz tick, sub-tick commands, constant-speed movement, seeded PCG32, FNV state hash, combat math with golden tests, analytic projectiles with exact swept hits | ✅ |
| `mftr-net`: bit-packing, packet header/acks, messages, link conditioner, min-RTT clock sync | ✅ |
| `mftr-client`: input-timeline stamping, bit-exact reconciliation, smoothed corrections, margin loop with time dilation, fast re-send of young commands, remote interpolation | ✅ |
| `mftr-server`: authoritative core + 270 KB UDP binary, late-command handling and arrival-lead reports | ✅ |
| Netcode Lab (headless, deterministic) and UDP bot | ✅ |
| `mftr-gdext` (godot-rust 0.5.5, Godot 4.7) + client: camera per D13, noise-shader ground, right-click move, click indicator, net graph | ✅ |
| Cross-platform hash check | ⏳ runs on first CI push (golden hash recorded on Windows x86_64; debug and release builds agree) |
| Two clients on **different machines** | ⏳ verified on one machine over real UDP (server + Godot client + bots); needs a second machine |
| Secure transport (netcode.io-style tokens, AEAD) and the transport decision | ⏳ deferred to the lobby work (before M2); M0 uses plain UDP |

**Netcode Lab results** (10 clients, 5 min after 10 s warm-up, scripted clicking at ~2.5 commands/s):

| Profile (RTT / jitter / loss) | Late commands | Corrections > 15 u per player-minute | Mean visible correction | Down / up per client |
|---|---|---|---|---|
| good (30 ms / 2 / 0%) | 0.00% | 0 | 0.00 u | 5.0 / 0.6 KB/s |
| typical (60 ms / 5 / 0.5%) | 0.05% | 0.16 | 0.00 u | 5.0 / 0.7 KB/s |
| **rough (120 ms / 20 / 2%)** | **0.20%** | **0.48** | **0.01 u** | 4.9 / 0.9 KB/s |
| awful (200 ms / 40 / 5%) | 0.42% | 0.72 | 0.01 u | 4.8 / 1.0 KB/s |

With no unit collision yet, the only source of corrections is a command that arrives late. The server applies it at the next tick, so the turn happens later than predicted (03a §9). Those corrections average ~60 u at 120 ms and are blended over ~50 ms. M1 adds collision, which brings in the proxy work of 03a §5.

## M1 — Duel Sandbox ("does it feel right?")
**Goal:** prove Pillar 1.
- Small arena with walls and brush, navmesh, and vision with fog culling.
- 2 placeholder champions (a skillshot mage and a marksman) with basic attacks, windups, attack-move, and 4 abilities each: linear skillshot, delayed AoE, dash/blink, and a hard-CC skillshot.
- Projectile timeline rendering, spawn streak, telegraph grammar, hitbox-honest VFX.
- Sub-tick commands, margin and interpolation control loops, analytic projectiles with swept hit tests ([03a](03a-netcode-time-and-prediction.md)).
- Unit collision with **minion-dummy clumps** in the arena to exercise collision proxies and the minion bubble.
- A/B experiments from [03a §12](03a-netcode-time-and-prediction.md#12-open-questions-for-m1-experiments).
- Utility spells: Blink, Barrier.
- Netcode Lab: dodge rig, net graph, blind playtest protocol.
- First pass of the noise-shader art style on the arena and the two champions.

**Exit:** ghost-hit and correction targets from [03 §1](03-netcode.md#1-measurable-targets) met; blind playtest testers rate dodge feel at 80 ms as "fair" ≥ 80% of the time; server < 1 ms per tick for 3v3.

### M1 slices (in order of netcode risk)

| # | Slice | Contents | Done when |
|---|---|---|---|
| 1 | **Collision & minion block** | D11 unit collision in `mftr-sim` (order-independent, no shoving, local detours around clumps); server-driven minion dummies (static clumps and patrolling waves); client collision proxies on T_input and the minion bubble (03a §5); Netcode Lab A/B of proxies on/off | Lab: collision-caused corrections > 20 u below 1 per player-minute at 80 ms |
| 2 | **Skillshots & the dodge rig** | Reliable events channel; cast windups; analytic linear skillshots with swept hits; display policy (enemy projectiles on T_input, own on Option B, predicted self-hits, spawn streak); dodge rig measuring ghost hits | Ghost-hit targets of 03 §1 met in the lab |
| 3 | **Arena, pathing & vision** | Vector arena (walls, brush), navmesh + funnel pathing, server-side vision grid and fog culling, projectile re-basing on vision entry | No hidden unit ever reaches a client (lab assertion) |
| 4 | **Champions & combat** | Two placeholder kits, basic attacks with windup, attack-move, delayed AoE, dash, hard-CC skillshot, Blink and Barrier, health and damage pipeline | Duel playable end to end |
| 5 | **Feel & look** | Telegraph grammar and D14 VFX, noise-shader art pass, blind-test harness (randomized hidden latency profiles, A/B toggles) | Blind playtest run |

### Slice 1 status (2026-10-05): ✅ done

- `mftr-sim`: order-independent unit collision against start-of-tick positions (D18), sliding, no shoving, stuck → detour around clumps, clicking into a clump stops at its edge. Minions (melee/caster/siege radii) with patrol brains.
- `mftr-client`: collision proxies on the input timeline, re-prediction on every snapshot near units, and the minion bubble render.
- Server scenario `minions` (the binary's default): 4 static clumps and 2 crossing patrol waves. The Godot client draws minions with their collision rings; **F2** toggles proxies for side-by-side feel.
- Unit test: with exact proxies, prediction is bit-exact through collisions.

**Netcode Lab, 1 player, 80 ms / 10 ms / 1%, 30 min:**

| | Corrections > 15 u per min | Mean visible correction | Visible jumps/min |
|---|---|---|---|
| empty arena (late-command baseline) | 0.60 | 0.01 u | 0 |
| minions, proxies off (naive) | 1.90 | 0.43 u | 0.80 |
| **minions, proxies on** | **0.87** (≈0.27 caused by minions) | **0.05 u** | **0** |

### Slice 2 status (2026-10-05): ✅ done

- `mftr-sim`: an exact integer timeline (`SimTime`, 1/1920 s, D21). Line skillshots have rooted windups at exact sub-tick instants and cooldowns. Analytic missiles use exact swept hits against units' motion, so the first enemy unit in the path takes it: minions body-block skillshots. Hard CC stun interrupts casts. Dodge-rig turrets aim directly or with lead.
- Protocol v4: a reliable, ordered events channel (cumulative ack, resend until acked); lossless cast, stun and cooldown own state; remote casting and stunned flags.
- `mftr-client`: own casts predicted (windup plus missile at once, Option B display). Enemy missiles are on `T_input` with predicted self-hits, enemy windups on `T_input`. Predicted interceptions by allies or minions are shown **unconfirmed** (dimmed, never hidden). The ghost/phantom measurement freezes what was shown and compares it with the server.
- Server scenario `dodge`: 4 turrets firing at the players. The Godot client draws missiles per D14 (slim core plus full-width sheath), windup aim lines and stun rings; **Q** casts.
- Validation: under fault injection (input margin forced negative), the detector does report ghost hits (2% at 200 ms), so the zeros below are real.

**Dodge rig, 6 scripted dodgers reacting to what their own client shows (0.25 s human reaction), 10 min:**

| Profile | Enemy missiles | Near-misses | Ghost hits (target) | Phantom hits |
|---|---|---|---|---|
| typical (60 ms / 5 / 0.5%) | 12,024 | 1,778 | **0 (0.00%)** (< 0.5%) | 8 |
| rough (120 ms / 20 / 2%) | 11,952 | 1,731 | **1 (0.06%)** (< 2%) | 9 |

Alone (no allies to intercept), 1 player, 1 hour: 0.05% ghost hits at 60 ms, 0.10% at 120 ms.

**Open issues found:**
1. **Champion-vs-champion bumps** dominate corrections when many champions crowd together: ~4 per player-minute with 10 click-spamming bots in a 3,000 u arena. Their proxies can't anticipate the other player's next click. Options are recorded as Q12 in DECISIONS.
2. **Bandwidth** rose to ~23–27 KB/s per player with ~70 units, because every unit's full state is sent every tick. Path-coasting and baseline deltas (03b §6) come next, before the unit count grows.

## M2 — ARAM ("a real game")
**Goal:** the smallest complete MOBA match.
- The Bridge map: minions, turrets, Gatehouse, Base, health relics.
- Gold, XP, levels 1–18, shop, ~25 items, death and respawn.
- **6 champions** covering archetypes: Mage, Marksman, Tank/Engage, Bruiser, Enchanter, Assassin (all original kits).
- Bots (basic), lobby, custom games, spectating, replays, reconnect.
- Self-host packaging: Docker compose, binaries, config docs.
- Size report and budgets enforced in CI.

**Exit:** 10 humans finish full matches on a community-hosted server with no desyncs or crashes; total download ≤ 60 MB; a 4-core VPS hosts 10 concurrent bot matches within budget.

## M3 — ARAM: Mayhem
- Augment draft system with ~60 augments across tiers and archetypes (rule breakers, size, ability transformers, quests, spell replacements).
- Delivery transformers, batched projectile spawns, size scaling with honest hitboxes.
- Roster to ~10 champions.

**Exit:** Hyper + Multishot stress scenario stays within bandwidth and tick budgets; playtesters rate Mayhem "fun" (the most important metric we'll ever track).

## M4 — Crossroads (full 5v5)
- 3-lane map, jungle camps, river, wyrms and Wyrm Soul, Elder, Mites, Siege Beast, Colossus.
- Claim (smite), wards and vision items, support item quest, turret plating.
- Blind and Draft pick with bans, roles.
- Roster to ~16–20 champions; item list to ~80.
- Bots that can play roles adequately.

**Exit:** full-length matches show the intended rhythm (laning → skirmish → teamfight) in telemetry from community playtests; late-game teamfight stays within performance targets on minimum spec.

## M5 — Community
- Matchmaking + OpenSkill rating per instance, Ranked queues.
- Public server list, moderation tools, blocklist sharing.
- Mod and content pipeline (community champions, skins within readability rules, custom modes).
- Localization framework.

## M6 — Federation
- Instance trust, cross-instance queues, federated ladders ([07 §7](07-hosting-and-trust.md#7-federation-post-m5-sketch)).

## Later / ideas
Arena mode, rotating modes, tutorial and practice tool, observer and caster tools, tournament bracket integration, "Classic-ish" retro map variant.
