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
| Secure transport and the transport decision | ✅ D40: Noise XX handshake and ChaCha20-Poly1305 on every packet, pinned server keys (protocol 13). Lobby-signed connect tokens come with the lobby |

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

- `mftr-sim`: an exact integer timeline (`SimTime`, 1/1920 s, D21). Line skillshots have rooted windups at exact sub-tick instants and cooldowns. Analytic missiles use exact swept hits against units' motion, so the first enemy unit in the path takes it: minions body-block skillshots. Hard CC stun interrupts casts. Dodge-rig turrets aim directly or with lead. Their shots stun nobody (a hit shows the impact flash and counts in the playtest stats), since chained stuns made the next shots undodgeable during the first playtest.
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

### Slice 3 status (2026-10-06): ✅ done

- `mftr-sim/map`: walls and brush as vector polygons, with the **arena** map (rock, long wall, L-wall, block, four brush patches, bounded edges) and the unbounded **open** plane. Deterministic grid A* (25 u cells, integer costs, fixed tie-breaks) plus string pulling gives any-angle waypoint paths with 35 u wall clearance, and goals inside walls snap to the nearest reachable point (D24).
- Movement: circle-vs-segment sweeps with sliding and rounded corners, in the same order-independent pass as unit collision. Units carry their path (lossless own state) and re-plan after detours or truncated paths. Unit tests: paths around walls never enter them, and prediction with the shared map stays bit-exact through walls, paths and collisions. A second golden hash covers the arena.
- `mftr-sim/vision` + server: per-team vision every tick (champions 1,200 u, minions 900 u; walls block; brush hides unless you share it; structures always visible, D25). Snapshots contain only visible units. Casts are sent only if the caster is visible. Enemy missiles are revealed when they enter vision, **re-based** to that point so the caster isn't leaked, and their end events go only to clients that saw them.
- Protocol v5: the welcome carries the map id; own state carries the path. The Godot client draws walls and brush from the same polygons.

**Fog audit** (Netcode Lab decodes every snapshot and checks it against the server's vision at that tick): minion arena, 10 players, 5 min at 80 ms: **530,920 unit-snapshots withheld, 0 leaked.** Also a server unit test for hiding behind a wall, in brush and out of range.

**Prediction unaffected by walls:** one player in the minion arena at 80 ms has 0.83 corrections > 15 u per minute (0.87 before walls). With 10 players it rises to ~6.9/min: champions bumping each other in the funnels the walls create, and enemies hidden in brush that can't be proxied (Q12).

**Open issues found:**
1. **Champion-vs-champion bumps** dominate corrections when many champions crowd together: ~4 per player-minute with 10 click-spamming bots in a 3,000 u arena. Their proxies can't anticipate the other player's next click. Options are recorded as Q12 in DECISIONS.
2. **Bandwidth** rose to ~23–27 KB/s per player with ~70 units, because every unit's full state is sent every tick. Path-coasting and baseline deltas (03b §6) come next, before the unit count grows.

### Slice 4 status (2026-10-06): ✅ done

- `mftr-sim/champion`: two original placeholder kits (D26). **Ember** (skillshot mage): Ember Lance (line, burst), Cinder Bloom (delayed AoE), Flicker (blink), Binding Sigil (hard-CC stun skillshot). **Vesper** (marksman): Longshot (line, poke), Shrapnel Charge (delayed AoE), Tumble (dash), Snare Net (hard-CC root skillshot). Utility spells: **Blink** (D) and **Barrier** (F). A unit test checks every dodge-intended ability against the D10 reaction budget at its class distance.
- `mftr-sim/world`: health, the 02 §5 damage pipeline (armor/MR mitigation, shields, death), respawns, stuns and roots. Ranged basic attacks: chase into range, rooted windup, a homing bolt, the attack timer; a new order during the windup cancels it with no damage (orb-walking, 02 §7). Attack-move picks the nearest visible enemy in range (D27). Delayed areas detonate at an exact instant against the units' motion. Dashes ignore units and slide on walls; blinks cross thin walls and never land inside one.
- Prediction (D28): own casts, attack windups and timers, dashes, blinks and Barrier are predicted bit-exactly (unit test with the whole kit against proxies). Damage, CC and deaths come from the server. Enemy telegraphs are drawn on `T_input`, so the detonation you see is the one the server judges.
- Protocol v6: Cast (6 slots), Attack and AttackMove commands; area, bolt, damage, death, respawn, blink, dash and shield events, filtered by fog (D29); vitals and status flags for other units; the welcome carries the champion and its home. A snapshot drops events to the next packet rather than exceed 1,200 bytes.
- Godot client: health and shield bars, the ability bar with cooldowns, area telegraphs that fill toward detonation, bolts, Barrier bubble, stun/root indicators, a hard-CC accent ring on stunning/rooting missiles, floating damage numbers, kill feed, death screen. Right-click an enemy to attack, A + left-click to attack-move, Q W E R D F to cast at the cursor, `--champion ember|vesper`.
- Server scenario `duel` (now the binary's default) and `mftr-tools bot --duel`: a scripted sparring partner over real UDP.

**Duel in the Netcode Lab** (DuelBots fighting with their whole kits and dodging what their own client shows, 10 min):

| | Kills (each side) | Visible correction (mean) | Corrections > 15 u per min | Ghost hits | Phantom hits | Server tick |
|---|---|---|---|---|---|---|
| 1v1, 80 ms / 10 ms / 1% | 22 / 17 | 0.33 u | 4.9 | **0 of 127** near-misses | 0 | 0.016 ms |
| 1v1, 120 ms / 20 ms / 2% | 22 / 15 | 0.46 u | 8.1 | **0 of 124** | 0 | 0.016 ms |
| 3v3, 80 ms | 17–32 per player | 0.50 u | 7.8 | **0 of 420** | 5 (1.2%) | **0.038 ms** (M1 target < 1 ms) |

Fog audit: 0 leaks. Downstream ~12–14 KB/s per player. The larger corrections have inherent causes: being stunned or rooted mid-move, dying, and chasing or bumping a champion whose next click can't be predicted (Q12). Missiles that resolve after the server already killed us (the missile passes a corpse) are counted as "died first", not as phantom hits.

**Exit check:** the duel is playable end to end in the lab (test `duel_is_playable_end_to_end`: both sides kill, die and respawn, corrections and ghost hits within the 03 §1 targets) and in the Godot client against a second client or `mftr-tools bot --duel`.

### Slice 5 status (2026-10-06): harness built, playtest pending

- **Blind-test harness** (03 §14, D30): `godot --path client -- --blind` runs a session of rounds (default 10 × 60 s) under hidden conditions: extra latency, jitter and loss added on top of the real link (none, 30, 60, 80 or 120 ms profiles), crossed with the A/B switches of 03a §12: own missiles **Option A vs. B** (D12) and the **minion bubble** on/off. Every profile appears with both missile options, in a seeded random order. The net graph is hidden; after each round the tester answers "did dodging feel fair?" and "how responsive (1–5)?". Each answer is appended to `blind_results.tsv` (in Godot's user data folder) with the condition and the client's own measurements for that round (RTT estimate, near-misses, ghost and phantom hits, corrections). `mftr-tools blind-report FILE...` summarizes any number of sessions per profile and per switch, and gives the M1 exit verdict (fair in ≥ 80% of rounds at 80 ms).
- **Telegraph grammar and D14 VFX** as shaders (all Compatibility-safe): linear skillshots have a rounded leading edge at the true hitbox front, a constant-width body with an always-visible edge sheath, a slim noisy core and a trail that fades *behind* the hitbox only. Hard-CC missiles carry moving chevrons in the shared accent color. Area telegraphs show a crisp outline from cast and a fill that reaches the edge exactly at detonation, then flash. Enemy missiles get a spawn streak from the caster's drawn hand (03a §7). Blink leaves a golden mark at its origin for 1.5 s and a burst where it lands (R01 §5).
- **Noise-shader art pass**: rock walls (world-space noise, strata, mossy tops), swaying brush, and champions with toon-banded light, rim light, object-space noise detail, a per-champion identity color and a team-accent band at the feet (05 §1, §4). Still no textures.

**Exit (blind playtest run):** needs human testers. The [playtest guide](../playtest.md) has the steps for testers and organizers: run it with `-- --blind`, collect the `blind_results.tsv` files and summarize them with `mftr-tools blind-report`. The transport decision left from M0 is D40, now implemented. The harness itself is verified end to end with `--blind-auto` (automatic answers) against a real server.

## M2 — ARAM ("a real game")
**Goal:** the smallest complete MOBA match.
- The Bridge map: minions, turrets, Gatehouse, Base, health relics.
- Gold, XP, levels 1–18, shop, ~25 items, death and respawn.
- **6 champions** covering archetypes: Mage, Marksman, Tank/Engage, Bruiser, Enchanter, Assassin (all original kits).
- Bots (basic), lobby, custom games, spectating, replays, reconnect.
- Self-host packaging: Docker compose, binaries, config docs.
- Size report and budgets enforced in CI.

**Exit:** 10 humans finish full matches on a community-hosted server with no desyncs or crashes; total download ≤ 60 MB; a 4-core VPS hosts 10 concurrent bot matches within budget.

### M2 slices (in order of dependency)

| # | Slice | Contents | Done when |
|---|---|---|---|
| 1 | **The Bridge & the match loop** | Single-lane map with per-map size; turrets, Gatehouse and Base with a destruction order; minion waves with lane AI, aggro and call-for-help; turret targeting priorities and ramping shots; melee attacks; fountain; health relics; win condition and match reset | A scripted push destroys every structure in order and ends the match (sim test); bot ARAM matches run in the Netcode Lab |
| 2 | **Economy & progression** | Gold (passive, last hits, kills/bounties, assists), XP sharing, levels 1–18 with stat growth, ability ranks and level-up commands, respawn timers by level | Lab bots level to 18 and the economy curve is reported |
| 3 | **Items & shop** | Ordered stat-modifier stack (02 §10), ~25 items with a few unique passives, shop while dead or in base, undo | Items change combat numbers exactly as 02 predicts (golden tests) |
| 4 | **Four more champions** | Tank/Engage, Bruiser, Enchanter, Assassin; new shapes: knock-ups and displacement, ally heals and shields, slows, targeted dashes, melee kits | All six kits pass the D10 linter; prediction parity through the new shapes |
| 5 | **Match flow** | Basic bots, lobby and custom games, ARAM all-random with rerolls and bench, reconnect, spectating, replays | 10-player bot matches finish; a replay re-simulates to the same hashes |
| 6 | **Packaging** | Docker compose, release binaries, config docs, size report and budgets in CI | A clean VPS runs a match from the docs alone |

### M2 slice 1 status (2026-10-06): ✅ done

- **The Bridge** (`mftr-sim/map`): 12,000 × 3,000 u, one straight lane from blue (west) to red (east) with cliffs, brush alcoves, two rocks and four health relics. It's point-symmetric, and the lane runs across the screen, so neither the layout nor the camera favors a side. Maps now carry their own size and a **layout**: lanes, wave and champion spawns, fountains, structures.
- **Structures and their order** (`mftr-sim/lane`, D31): per team an outer, inner and gatehouse turret, a Gatehouse (respawns after 5 min), two base turrets and the Base. A structure can only be hurt when every lower tier of its team is down; the client shows protected structures under a dome. Destroying a Base ends the match, and the server starts a new one 10 s later.
- **Waves and minion AI:** a wave of 3 melee and 3 casters every 30 s (a siege minion every third wave) walks its lane. Minions answer an enemy champion attacking an allied champion first, else keep a valid target, else take the closest minion, then structure, then champion. Melee attacks land at the end of the windup.
- **Turret AI** (01 §3): a champion attacking an allied champion in range first, then the current target, then siege > caster > melee minions, then champions. Consecutive shots on the same champion ramp +35% each; shots at minions take a share of max health.
- **Fountain and relics:** the fountain heals its team (predicted too, since it's map data) and burns enemies; relics heal 25% and return after 40 s. Skillshots and areas pass through structures (D32).
- Protocol v7: 7 unit kinds, protected flags, gameplay radius on the wire, heal and match-end events. The Godot client draws turrets, Gatehouses, Bases, relics, fountains and protected domes, plus structure health bars and a victory/defeat banner. Server: `--scenario aram`.

**Tests:** a pushing champion takes every red structure strictly in tier order and the match ends; turrets answer an attack on an allied champion and ramp up; relics and the fountain heal; waves meet and fight for 2.5 minutes with every structure standing. A third golden hash covers the lane match loop.

**Netcode Lab, ARAM 3v3 at 80 ms, 10 min:** 0 hard resets, 0 fog leaks, 0 ghost hits of 388 near-misses, mean visible correction 0.22 u, server tick 0.15 ms. Downstream was 27 KB/s per player, which would have put 5v5 over the 32 KB/s budget. Snapshot deltas (below) brought it to 8.7 KB/s. Bot matches don't end yet: the bots walk into turrets, and they have no levels or items until slices 2–3 and smarter bots until slice 5.

### Snapshot deltas (Q13, between M2 slices 1 and 2): ✅ done

Other units are now sent as deltas against the newest snapshot the client reconstructed, with **path coasting**: a unit walking its replicated path costs nothing until it turns, stops or changes (D33, protocol v8). Prediction quality is unchanged; this is pure bandwidth.

| Netcode Lab, 80 ms, 5 min | Before (KB/s down per player) | After |
|---|---|---|
| Duel 1v1 | ~11.6 | **4.7** |
| 10 players in the minion arena (~70 units) | ~25 | **6.4** |
| Dodge rig, 6 players | — | **6.5** |
| ARAM 3v3 on The Bridge | 24.6 | **8.7** |

A lossy-link lab test (120 ms, 2% loss) checks every reconstructed snapshot against what the server recorded for that client: bit-exact. The fog audit now checks everything each client can reconstruct, not just what one packet carries: still 0 leaks.

### M2 slice 2 status (2026-10-06): ✅ done

- **Levels 1–18** (D34): experience to the next level is 180 + 100 × level. Champion stats grow on the 02 §1 curve from per-champion growth values, and current health rises with max health on level-up. Respawn takes 6 s + 1.5 s per level.
- **Ability ranks:** a point per level, spent with **Ctrl + Q/W/E/R** (`LevelUp` command, predicted). Basic abilities go up to rank ⌈level / 2⌉ (max 5); the ultimate unlocks at 6 / 11 / 16. Each rank adds base damage and shortens the cooldown (per-ability data). Unlearned abilities can't be cast.
- **Gold:**
  - Passive income.
  - Last hits: 21 / 14 / 60 for melee / caster / siege minions.
  - Champion kills: 300 gold, up to +500 on a kill streak, down to 140 on a death streak. Champions who hurt the victim in the last 10 s split half of the kill value as assists. If a turret or minion finishes the kill, credit goes to the last champion who hurt the victim.
  - 150 gold to every champion on the team that destroys a turret.
- **Experience** is shared by enemy champions within 1,400 u of a death, +15% per extra sharer. Rewards go only to the client who earned them.
- **Match rules** come in the welcome (`Rules`: start level, start gold, passive gold, ranked), so prediction applies passive gold and rank gates too. Sandboxes keep every ability at rank 1 with no economy, so all M1 numbers and tests are unchanged. ARAM starts at level 3 with 1,400 gold and earns 4 g/s.
- Protocol v9. HUD: level, XP bar, gold, rank pips, a "+" on abilities that can be ranked, gold popups, and enemy champion levels next to their health bars. Bots spend their points (ultimate first).

**Netcode Lab, ARAM 3v3, 15 min:** bots reach levels 9–15 and 10–15k gold, with 0 ghost hits, 0 fog leaks and 0 hard resets. Gold piles up until the shop exists (slice 3).

### M2 slice 3 status (2026-10-06): ✅ done

- **26 items** (D35, `mftr-sim/items.rs`), all original:
  - Components: Long Knife, Spark Shard, Vital Crystal, Padded Vest, Warding Cloak, Quick Dagger, Boots, Focus Charm, Heavy Pick, Charged Wand.
  - Upgrades: Titan Belt, Leech Fang, Arc Bow, Chain Coat.
  - Boots upgrades: Swift, Battle and Sage Boots.
  - Legendaries: Inferno Diadem, Grand Grimoire (+35% AP), Crimson Fang, Heartstone, Bramble Plate, Wardstone Mantle, Arc Tempest (on-hit magic), Gale Saber, Lifeline Talisman (a shield when low).
- **Stat stack** (02 §10): level stats, then flat item bonuses, then percent bonuses, then caps. New stats: ability haste (Q W E R cooldowns × 100 / (100 + haste)), life steal on basic attacks, bonus attack speed (into the attack period, capped at 2.5/s), flat and percent move speed (soft-capped).
- **Shop:** six inventory slots in the progression state, so items survive death and prediction handles them like any other state. `Buy`, `Sell` and `Undo` commands are predicted. The shop works only in ranked matches, while dead or inside your fountain. Recipes use owned components and cost the difference. Selling refunds 70%, and the last 4 trades can be undone until you leave.
- Protocol v10 (4-bit command kinds). Godot client: a shop panel on **P** (grouped by tier, price after owned components, a stats line, click an inventory slot to sell, Undo) and an inventory strip next to the ability bar. Bots follow a build path per champion whenever they respawn.

**Tests:**
- 02 §10 golden case: flat AP before Grand Grimoire's +35%.
- Attack speed cap; boots don't stack.
- Recipe discounts.
- Fountain/death shop rule, refunds and undo.
- Exact damage per hit with AD items and Arc Tempest's on-hit magic.
- Life steal amounts; the attack period follows bonus attack speed; haste on cooldowns; Lifeline fires once.
- The Bridge golden hash now includes random purchases and undos.

**Netcode Lab, ARAM 3v3, 5 min:** bots hold 2–4 items each, with 0 ghost hits, 0 fog leaks and 0 hard resets on every profile. Corrections are in line with the previous build.

### M2 slice 4 status (2026-10-06): ✅ done

Four more placeholder champions (original kits, D36), so all six M2 archetypes exist:

| Champion | Role | Q | W | E | R |
|---|---|---|---|---|---|
| **Bastion** | Tank / engage (melee) | Grapple: pulling skillshot | Bulwark: self shield | Tremor: slowing nova | Upheaval: delayed knock-up area |
| **Rook** | Bruiser (melee) | Cleave: nova | Second Wind: missing-health heal | Lunge: slowing targeted dash | Shockwave: slowing skillshot |
| **Lumen** | Enchanter | Mending Light: ally heal | Aegis: ally shield | Lull: slowing skillshot | Binding Halo: delayed root area |
| **Shade** | Assassin (melee) | Shadow Step: lunge | Fan of Blades: slowing nova | Veil Step: dash | Execution: lunge |

- **New shapes:** slows, knock-ups, pulls (forced movement), self-centered novas, area CC, ally heals and shields, and lunges (targeted dashes that strike on arrival). Melee champions reuse the minions' melee hits, now with on-hit items and life steal.
- **Rules:** point-and-click effects never carry hard CC (test-enforced), so every stun, root, knock-up and pull can be dodged. All hard-CC skillshots and areas pass the D10 reaction budget.
- Protocol v11: every CC kind on the wire, area CC, lunge strikes in the own state, slows in the own state and as a flag on others, and others' speed sent after slows (so coasting stays exact).
- Godot client: distinct silhouettes for all six, a cold-blue slow ring, gold rims on hard-CC areas. ARAM hands out the six in turn; bots use heals, shields, novas and lunges and have a build path each.

**Tests:**
- One test per new shape: pull distance and stun, knock-up area, slow speed and expiry, ally and self support targeting, lunge strike and no-target refusal, melee hits and novas.
- The D10 linter and the hard-CC classification check cover all six kits.
- Prediction stays bit-exact through every kit, including heals and shields on an ally.
- Every spec and CC kind round-trips the wire.
- The Bridge golden hash now runs a 3v3 with all six.

**Netcode Lab, ARAM 3v3 with all six, 5 min:** 0 ghost hits, 0 fog leaks and 0 hard resets on every profile. Being pulled, or lunging at a target known only from interpolated positions, adds corrections: largest correction 23 u with no added latency (it was 3.5 u). The lab's jump meter now ignores dashes, which it used to count as pops.

### M2 slice 5 status (2026-10-06): ✅ done

**Bots and replays** (D37), the slice's exit criteria:
- **Server bots** (`--bots N`): they follow their wave and siege structures when it's safe (minions tank the turret, or they outnumber the defenders by two). They fight enemy champions with their whole kit, heal and shield allies, fall back to relics or turret cover when low, shop their build path and spend points. They issue ordinary commands, so the replay records them like anyone else.
- **Replays** (`--replay FILE`, `mftr-tools replay FILE`): one match driver is shared by the server and the replayer. It records joins, leaves, commands and a hash every 10 s.
- **10-bot ARAM matches finish:** 9 seeds out of 9 within an hour (14–46 min), simulated in about 2 s each. Every replay re-simulates to the recorded hashes, a tampered command is detected, and a networked Netcode Lab session's recording re-simulates to the server's final hash.

**Match flow** (D38, protocol v12):
- **Champion select** (`--lobby`): ARAM all-random with 2 rerolls each and a team bench to swap with. Ready up or wait out the 60 s countdown; bots fill free slots, and a joining human replaces a bot.
- **Custom games:** server flags set the scenario, bots, player limit, champion select and replay recording.
- **Reconnect:** a session token in the Welcome. A dropped champion waits 60 s; the Godot client saves its token and takes the champion back after a restart.
- **Spectating** (`--spectate` on the client): full vision, no unit, Tab cycles through the champions to follow.
- Godot: a champion-select panel (teams, rerolls, ready, bench) and a spectator camera.

**Tests:** champion select rerolls, readies and starts with ten players; a real client session goes from champion select to playing over a lossy link, with a spectator alongside that sees both teams; reconnecting by token, from a new address, until the grace period ends; spectators see every unit and can't issue commands.

### M2 slice 6 status (2026-10-06): ✅ done (one check left)

- **[Hosting guide](../hosting.md):** Docker, Compose, the binary with a systemd unit, every option, replays, measured costs, troubleshooting.
- **Server image** (`Dockerfile`): distroless, non-root, 40 MB. **Compose** (`deploy/compose.yaml`): an ARAM server with champion select, bots and replay recording, plus a duel server. Tested here: the container serves a lab client, and its recorded replay re-simulates exactly.
- **Release workflow** (`.github/workflows/release.yml`, on `v*` tags): server and tools for Linux, Windows and macOS, the Godot client exported for all three (`scripts/package-client.sh`), a GitHub release, and the image on ghcr.io.
- **Size budgets in CI** (`mftr-tools size-report`, D39):

| Item | Size | Budget |
|---|---:|---:|
| Server binary | 0.79 MiB | 5 MiB |
| Tools binary | 1.13 MiB | 5 MiB |
| Godot extension | 3.46 MiB | 15 MiB |
| Client project (scripts, shaders) | 0.07 MiB | 20 MiB |
| Linux client download (export, compressed) | 26.6 MiB | 60 MiB |

- **Capacity:** one 10-player ARAM costs 0.18 ms per 33 ms tick (about 0.5% of one core) and about 105 KB/s upload. Ten concurrent matches fit on one core of a 4-core VPS many times over.

**Left for the M2 exit:** the release workflow runs on GitHub for the first time with the first tag, and ten humans still need to finish a match on a community-hosted server.

## M3 — ARAM: Mayhem
- Augment draft system with ~60 augments across tiers and archetypes (rule breakers, size, ability transformers, quests, spell replacements).
- Delivery transformers, batched projectile spawns, size scaling with honest hitboxes.
- Roster to ~10 champions.

**Exit:** Hyper + Multishot stress scenario stays within bandwidth and tick budgets; playtesters rate Mayhem "fun" (the most important metric we'll ever track).

### M3 slices (in order of dependency)
1. Augment framework and live draft (D43).
2. Delivery transformers (Multishot, Echo, wider shots) and batched missile spawns.
3. Size changes with honest hitboxes (Titan, Pebble, Unstable Experiment).
4. Augments to ~60: rule breakers, conditionals, quests, utility spell replacements.
5. Roster to ~10 champions.
6. Mayhem end to end: augment indicators, Hyper rules, the Hyper + Multishot stress scenario.

### M3 slice 1 status (2026-10-07): ✅ done
- `--scenario mayhem`: ARAM with augment drafts at levels 1, 7, 11 and 15 (three cards, one reroll each; Silver first, one Prismatic guaranteed). 23 augments so far: stat packages, AD↔AP conversions and percent bonuses through the stat stack.
- Picks and rerolls are predicted commands (offers are seeded per champion); bots and the lab's duel bots draft too. Lab: 4 clients over the MID link draft with no hard resets, and the recording re-simulates exactly (replays now end with a final state hash, so a check covers every simulated tick).
- Client: draft cards above the ability bar while play goes on; held augments listed above the inventory.

### M3 slice 2 status (2026-10-07): ✅ done
- Delivery transformers (D44): **Multishot** (three projectiles 15° apart, each enemy hit once per volley), **Echo** (repeat after 0.75 s at 40% power), **Broadside** (wider lines, larger areas). Abilities declare which they accept.
- Batched spawns: a volley is one event record on the wire (about 9 bytes per extra missile).
- Lab: a Multishot + Echo + Broadside champion casting over the MID link has every missile predicted at once, confirmed by the server, and no corrections. 26 augments.

### M3 slice 3 status (2026-10-07): ✅ done
- **Titan**, **Pebble** and **Unstable Experiment** (D45): the hitbox and the model change together; collision stays champion-sized. Tests: a skillshot 110 u beside a champion misses a normal hitbox and hits a Titan's; 80 u beside hits a normal one and misses a Pebble's; Pebble deals exactly 20% more to larger targets; Unstable re-rolls on every respawn (both forms come up).
- The client checks missile outcomes against its predicted hitbox; the lab Titan's hitbox grows on both ends with no corrections. 29 augments.

### M3 slice 4 status (2026-10-07): ✅ done
- **60 augments** (D46): 23 Silver, 20 Gold, 17 Prismatic. Conditionals (Executioner, First Strike, Last Stand, Spellcrit, Fundamentals), after-hit effects (Spellhunger, Spell Vamp, Thorns), rule breakers (Close Quarters, Sharpshooter, Spellblade, Reset), a quest (Champion of Chaos) and F replacements (Vault, Stormcall, Mend).
- Tests check each mechanic's numbers against an unaugmented baseline. The client shows Spellhunger stacks and quest progress next to the held augment.

### M3 slice 5 status (2026-10-07): ✅ done
Four more placeholder champions (original kits, D36), built from the existing shapes:

| Champion | Role | Q | W | E | R |
|---|---|---|---|---|---|
| **Quill** | Artillery mage | Arc Shot: long-range area | Static Field: slowing area | Recoil: dash | Starfall Lance: 2500 u skillshot |
| **Cairn** | Warden (melee) | Stone Lash: slowing skillshot | Shelter: ally shield | Rockfall: delayed root area | Monolith: delayed knock-up around itself |
| **Marrow** | Battlemage | Siphon: nova | Grasping Bones: rooting skillshot | Grave Pact: self heal | Ossuary: large slowing area |
| **Wren** | Skirmisher | Ricochet: quick poke | Caltrops: slowing area | Pounce: lunge | Hail of Arrows: slowing area |

- All four pass the D10 reaction budget and the hard-CC classification; prediction stays bit-exact through every kit. Bots play them with build paths of their own.
- The wire's champion field grows to 5 bits (room for 31). Godot client: a silhouette and identity color each; the menu and ARAM pick from all ten.

### M3 slice 6 status (2026-10-07): ✅ done
- **Hyper** (D47): `--scenario hyper` is Mayhem with +300 ability haste on Q, W and E and +50% attack speed.
- **Augment indicators:** every champion's augments show as tier-colored diamonds above its health bar; the client also shows Spellhunger stacks and quest progress.
- **Volley rule on the client:** the display predicts one hit on us per enemy volley, as the server rules. Before, Multishot made every missile of a volley look like a hit.

**Stress exit (Netcode Lab, release build, MID link, 10 clients under Hyper, every champion with Multishot + Echo + Broadside, 60 s):**

| Metric | Result | Budget |
|---|---|---|
| Download per client | 11.0 KB/s | 32 KB/s |
| Upload per client | 1.1 KB/s | — |
| Server tick (mean / max) | 0.18 ms / 1.6 ms | 3 ms |
| Enemy missiles judged | 1,635 | — |
| Ghost hits | 0 on seeds 1–3 | < 2% of near-misses |
| Phantom hits | 2–5% of near-misses (2% without Multishot) | — |
| Hard resets, fog leaks | 0, 0 | 0 |

The same check runs in CI as a 40 s lab test. **Left for the M3 exit:** playtesters rating Mayhem fun.

## A — Characters & Animation (parallel track)
Runs alongside M3/M4 and replaces the placeholder shapes with real champions ([10](10-characters-and-animation.md), [11](11-content-packs-and-mods.md)).

**Exit:** all ten champions ship their complete animation set as validated first-party packs. In a blind side-by-side at the gameplay camera, playtesters rate facing, attack crispness and readability no worse than the reference game.

### A slices (in order of dependency)
1. **Pipeline:** `art/` layout, the `mftr_blender` add-on (rig generator for `biped` v1, export, marker sidecar, review renders) and the shared library block-out (CC, `walk`, `cast_utility`, `attack_melee_alt`). Pack format v0 and the `mftr-pack` validator in CI.
2. **Sim contract (D52):** `facing` state and its wire field, action phases (`follow_through`, `hard_lock`, `windup_cancel`, `mobile`), the input buffer and the attack-variant counter. Unit tests plus a Netcode Lab check that cancelling and buffering survive prediction and reconciliation.
3. **Client runtime:** pack loading through gdext (no `ResourceLoader`), the AnimationTree layout, piecewise retiming from sim time, facing display, socket-spawned projectiles, impact flash, hover outline.
4. **Pilot: Vesper** (ranged), complete set, VFX and SFX, in game. Split in three: **A4a** the model and every clip, in game; **A4b** the VFX kit (05 §5) and her VFX; **A4c** the SFX pipeline (05 §7) and her sounds.
5. **Pilot: Rook** (melee, `biped_large`), complete set, VFX and SFX, in game.
6. **The other eight champions**, one slice each, against the definition of done in 10 §10.

### A slice 1 status (2026-10-07): ✅ done
- **Blender tools** (`tools/blender/mftr_blender`, a Blender 5.2 extension that also runs headless via `tools/blender/run.py`): the `biped` v1 rig generator (34 bones: 26 deforming, root, 2 props, 5 sockets; ≤ 64), the faceted template mesh, clip and marker helpers with an *aim* pose API (point a limb in character space, mirror-safe), deterministic export and the `.anims.ron` sidecar, and review renders. Everything in this slice was generated by Claude through the Blender Lab MCP server, driving the owner's running Blender.
- **Template mesh:** 3,248 triangles from lofted segments, each weighted rigidly to one bone (1 influence); vertex colors with height occlusion in alpha; all five material slots.
- **Export:** sampled at 30 fps, then keys within 0.1° / 0.5 mm of the interpolation dropped (2,750 → 1,574 keys for the library), channels that never leave rest dropped (889 of ~1,000), and identical accessors shared. No normals or UVs: shading is faceted from screen-space derivatives (D48). Library: 174 KB; rig: 104 KB. Two exports of the same `.blend` are byte-identical.
- **Shared library** (`art/library/biped`): block-outs of `walk` (140 u/s stride), `cast_utility` (upper body, `fire` 7), `attack_melee_alt` (`fire` 9), and the seven `cc_*` clips (`cc_airborne` with a `loop_in`/`loop_out` region; `cc_rooted` and `cc_forced_move` additive). Arm placements verified numerically (e.g. the cast's hand is 55 cm in front of the shoulder on `fire`). These are starting points for human polish (D49).
- **Review renders:** per clip, a contact sheet and a GIF from the R01 gameplay camera at its true 1080p pixel size, next to a ¾ close-up; the marker strip under each tile shows `fire`, loops, foot contacts and the playhead. All ten clips render headless in under a minute.
- **`mftr-pack`** (new crate): a strict JSON reader, the glTF subset reader (no images, textures, cameras, extensions or URIs; bounds-checked accessors; TRS nodes; cycle detection) and the validator: triangle, bone, influence, material and size caps; bone names and sockets; facing (+Z) and sides; ground contact; required clips (champion packs derive their ability clips from the kit in `mftr-sim`); markers (names, order, ranges, required per clip); `fire` against the sim's windup (±1 frame); no root motion; seamless loops; sidecar/model agreement. `mftr-tools pack validate FILE|DIR` runs it, CI runs it on `art/`, and 12 tests break real exports on purpose (each error is caught).

**Left for later:** re-exporting in CI to prove `export/` matches its `.blend` (needs Blender in CI); the fire-time check against champion data gets its first real use with the Vesper pilot (slice 4).

### A slice 2 status (2026-10-07): ✅ done (D52, protocol 16)
- **Facing** is sim state: a unit vector that snaps to the walking direction, the attack target (tracked through the windup) and the aim of casts, dashes and blinks. Exact for the own champion; a 10-bit angle (half-step error ≤ 0.18°) with the motion group for others.
- **Follow-through** (`Recovery`): after a line or area ability fires (200 ms; ultimates 350 ms, the first 150 ms a hard lock) and after a dash lands (80 ms). The caster stays put; a move, attack-move, stop or cast ends the soft part at once, an attack order waits for it, and a caster walking on skips it. Timings are `Ability::timing` defaults by effect kind *(start)*.
- **Input buffer:** a cast ordered during a windup, a dash or a hard lock starts the instant that ends (one slot; a newer order replaces it; hard CC clears it). Previously such casts were dropped.
- **Attack counter** (wrapping, 2 bits on the wire) picks the attack animation identically on every client; the follow-through state is a status flag. All of it is exposed to GDScript for slice 3.
- **Tests:** 9 sim tests (facing, holding and cutting the follow-through, walking on, the attack order waiting versus the move-cancel tech, buffering behind windups and dashes, the latest order winning, hard locks, stuns, the counter), a 10-bit facing round trip, and a Netcode Lab check: a scripted Vesper buffering, cancelling and hard-locking over the `mid` link for 45 s predicts with **0 corrections** in 1,331 reconciliations, and the server confirms every buffered cast.
- **Proxies:** a follow-through made a long-standing gap common: the server sent a heading for units that couldn't walk (an attack order waiting out a follow-through, but also stunned, rooted or winding-up units), so clients extrapolated them walking. It now sends none for them. In the Hyper + Multishot stress (seed 1) phantom hits went from 3–4 to **0**, at 690 enemy missiles, 10.4 KB/s down and 0.069 ms per tick.
- The golden state hashes changed (new state), recorded on Windows; CI checks Linux and macOS.

**Left for later:** recast `stages` (with the first champion that needs them); per-ability timing overrides (with the pilots).

### A slice 3 status (2026-10-07): ✅ done
- **Pose runtime in Rust** (`mftr-pack`: `pose`, `animator`): rig and clips from a validated pack, sampling (slerp), masked and additive blending, and the 10 §6 layer stack driven by sim state. Actions are retimed piecewise (windup progress → `[0, fire]`, follow-through → `[fire, end]`), so the release lands on the fire time at any attack speed. 8 tests on the committed library.
- **Packs load through gdext only:** `MftrModel.load` validates (`mftr_pack::load_file`), then builds the `Skeleton3D`, the skinned `ArrayMesh` (Godot winding, joints remapped parents-first, smooth normals for the outline hull) and the skin; `ResourceLoader` never sees pack bytes. `MftrAnimator` poses the skeleton each frame from `own_status()` / `remote_units()` (`anim_*` keys; the own champion's are exact predicted sim times).
- **Client:** champions wear the template (identity tint on cloth, team accent) until they ship their own models; **F3** toggles the placeholder shapes. Facing turns at 4,500 °/s, confirmed damage flashes the target white for ~3 frames, the enemy under the cursor gets a red outline, and projectile streaks leave from `socket_projectile`. The model shader is faceted (screen-space derivatives) with a three-band ramp. Our own champion's death now plays instead of the body vanishing.
- **Library** gains fallback `idle`, `run` (330 u/s stride) and `death` block-outs.
- **Checked in Godot 4.7.2** against a duel server: walking and running facing the path, casting, the hover outline and the enemy's red accent. `--zoom <factor>` brings the camera closer for such review captures.

**Left for later:** remote champions' death (dead units leave the snapshot); remote attack timing (the wire has none, so those clips play at their own rate); shipping packs inside exported builds (the client reads them from `art/` next to the project for now).

### A slice 4a status (2026-10-07): ✅ done (concept awaiting approval)
- **Heads** (`head.py`, shared by every model): a sculpted parametric skull (a V-shaped jaw, chin, cheekbones, a fuller back of the skull) with eyes (whites and pupils), brows, a nose, a mouth and ears, and hair that follows the skull from a hairline higher at the front than the nape; `short` (the template, which lost its headband) or `ponytail`. After the owner's review: the egg-shaped heads read as "an egg with two dots".
- **Vesper's model** (`art/champions/vesper`): a slim archer on `biped` v1 plus an `extra_cape` chain (3 bones) for her cloak; 3,280 triangles; a feminine hourglass build, auburn hair with bangs and a ponytail, her hood down (a cowl and a back drape), dusk-teal cloak, leather, accent bracers and fletching. Her 1.5 m recurve is part of the skinned mesh on her left hand, built in the rest frame the full draw turns into *vertical*, so it stands upright whenever she shoots.
- **Every clip** (10 §5): 22 of her own (idle, two fidgets, idle_ready, run, run_fast, two attacks, Q, W, the Tumble start/travel/land trio, R, recall, death, respawn, select and four emotes) plus the 10 shared ones copied in from the library. `mftr-tools pack validate` checks them all against her kit, each `fire` within a frame of her sim windup. Block-outs from `build.py` (a starting point, like the library).
- **Runtime:** the client loads a champion's own pack when it ships one (else the template). The animator gains the dash trio (`<slot>_start` → looping `_travel` → `_land`), `run_fast` above the midpoint of the two strides, `idle_ready` for 3 s after an action and idle fidgets after 8 s still; 3 new tests on her pack.
- **Checked in Godot** against a duel server: she faces her target and draws with the bow vertical.

**Left for later:** the owner's approval of her concept; polish (the block-outs' draw hand and run arms); emotes, recall and select have no in-game trigger yet.

### A slice 4b status (2026-10-07): ✅ done
- **The VFX kit** (05 §5.1):
  - nine kits: `flare`, `burst`, `ring`, `dust` and `trail`, plus the projectile styles `arrow`, `net`, `orb` and `lob`;
  - pixel-style particles in one MultiMesh, snapped to screen pixels by `pixel_vfx.gdshader`, stepped at 20 Hz through stepped colour ramps;
  - projectile styles fitted inside the gameplay body;
  - detonation rings sized from the area's radius.
- **Format:**
  - `<id>.vfx.ron` next to the model (11 §3.1), parsed and checked by `mftr-pack` (kits per phase, ramps, bounded knobs, caps);
  - loaded with the pack, and exposed to the client as `MftrModel.vfx()`.
- **Events:** the render state now carries each area's, missile's and bolt's owner, champion and action. The action is resolved by matching radius against the owner's kit, including augment widening; dashes use `MatchClient.dash_action()`. A champion's effects fall back to the library's `biped_library.vfx.ron`.
- **Vesper:**
  - gold arrows (flare, shaft and trail, spark burst);
  - a teal-white Longshot;
  - a lobbed Shrapnel Charge that sizzles, then detonates in shrapnel, a ring and smoke;
  - Tumble dust;
  - a spinning gold Snare Net.
- **Checked in Godot:** W, Q, E and R were checked against a duel server; the flare core and the smoke were toned down after the first captures.

**Left for later (A4b):**
- crit sparks and the impact flash's spark burst at `socket_chest` (10 §6), which need crit and target info in the render state;
- the allied-VFX opacity setting (05 §1.6);
- footstep dust from `foot_l`/`foot_r` markers;
- the basic-attack path was not captured on screen (no enemy came into range in the scripted duel).

### A slice 4c status (2026-10-07): ✅ done
- **SFX pipeline** (05 §7.1):
  - `mftr-tools sfx build` renders hand-written `sounds.ron` recipes with a layered synthesizer into deterministic Ogg Vorbis files and a generated `<id>.sfx.ron`.
  - The encoder is `vorbis_rs`, a tools-only dependency.
  - A test fails when a recipe changed without a rebuild.
- **Format** (11 §3.2):
  - `mftr-pack` checks bindings and caps, rejects stray files, and decodes every file with lewton (memory-safe; caps checked before allocation; trimmed to the stream's true end).
  - Packs load with their PCM, which `MftrModel.sounds()` hands to Godot as `AudioStreamWAV`.
- **Events:**
  - The animator reports footsteps (crossing `foot_l`/`foot_r`, not while dashing) and windup starts; `MftrAnimator.drive()` returns them.
  - Death and respawn come from the unit's `dead` edge.
  - Every VFX event also plays its sound, and a hard-CC hit or detonation adds the shared `cc.hard` accent.
- **Client:** `sfx.gd` has 24 positional voices with stealing, variants that don't repeat, and a listener above the camera's look point.
- **Sounds:**
  - The library has 14 in 83 KB: a generic cast, release, hits, explosion, dash and land, three footsteps, death, respawn, recall and the hard-CC accent.
  - Vesper has 17 in 84 KB (of her 550 KB budget):
    - a bow-draw creak, twangs and thunks;
    - Longshot's draw, release and crack;
    - a fuse, a toss and a rattling blast for Shrapnel Charge;
    - Tumble's whoosh and landing;
    - Snare Net's swing, throw and two-note "caught" jangle;
    - a whistle emote.
- **Checked in Godot** against a duel server, from the client log:
  - every sound fired on its event;
  - the bot fell back to the library;
  - footstep variants alternated;
  - the net's hit played with the hard-CC accent.

**Left for later:**
- the owner's listening review (an audition page goes with the PR);
- a volume setting and an SFX bus;
- crit sounds (no crit info in the render state yet);
- triggers for recall and emotes;
- sounds for a unit drawn without its model (F3): they come from the animator, so none play.

### A slice 5 status (2026-10-07): lane minions ✅ done
Minions as tiny hooded figures, distinct from champions at a glance (the owner's brief: faceless like a black mage, four kinds, team-colored).
- **Sim:** super minions (01 §4).
  - While a team has an enemy Gatehouse down, each of its waves brings one in front, until that Gatehouse respawns. This was stated before but not built.
  - Their numbers are in 01 §4.
- **Wire:** protocol 17. Remote units carry the minion kind in their static group: 2 bits, on minions only.
- **Art** (`art/minions/build.py`):
  - One block-out generator for four packs on the `biped` bones in a new chibi "minion" shape (`rig.use_shape`), 1,186–1,558 triangles each.
  - Each has a pointed hood and robe in the team color (`accent`), and a dark void for a face with two eyes glowing in the team color (`accent_glow`, a new slot).
  - **Melee:** sword and buckler.
  - **Caster:** a staff with a glowing orb; it throws spells from its free hand.
  - **Siege:** a gunner pushing a two-wheeled bronze cannon (extra bones for the carriage, a recoiling barrel and wheels that roll on the run).
  - **Super:** ×1.6 and armored, with a horned crown and a great maul.
  - Clips: idle, run, attack_1 (`fire` on the sim's windup, checked), death and an additive flinch.
- **Format:**
  - A `minion` pack kind with its own catalogue, triangle target (600–2,600), and fire check against `lane::minion_attack`.
  - `Loaded.kind` records which kind of pack it is.
- **Runtime:**
  - The animator plays `flinch` on a hit (minions flinch, champions don't, 10 §5.4) and, for minion packs, lets a swing play out after its windup instead of cutting it.
  - The client draws each minion with its kind's model in its team's color, and animates it like a champion.
  - A minion last seen at 0 health plays its death where it fell, then sinks away.
  - Release packages ship `art/minions/*/export`.
- **Checked in Godot** on an ARAM server: blue and red waves meet and fight, with the casters' bolts, the hit flash, and the siege cart in the third wave.

**Left for later:**
- the super minion's buff aura (01 §4);
- minion sounds and VFX (they use none yet);
- a super minion seen in a live match (it needs a Gatehouse down; covered by a sim test and the renders);
- instancing, if many waves get costly (every minion has its own skeleton today).

### A slice 6 status (2026-10-07): pilot Rook ✅ done (concept awaiting approval)
- **The owner's brief:** a grizzled veteran in iron and oxblood, with a stone-headed maul.
- **Rook's model** (`art/champions/rook/build.py`):
  - The `biped` bones in the new `large` shape (`biped_large`, 10 §7.1: ~2.05 m, broad and deep); 3,194 triangles.
  - Battered iron plate (breastplate, gorget, layered pauldrons, gauntlets, knee cops) over a quilted gambeson and an oxblood tabard. The team accent is on the sash, the tabard hems and the pauldron rims.
  - The shared head grows a `bald` style, a beard (full, with a braid, an iron bead and a moustache) and a scar.
  - The 1.4 m maul lies along the right hand bone, so every pose aims it exactly: on the shoulder, high in the windups, into the ground on the slams.
- **Every clip** (31): 22 of his own and 9 shared. Each `fire` was checked against his kit, and the pack validates clean.
- **Events** (10 §6):
  - The animator reports `fire` when an action passes its `fire` marker, even when a walking caster skips the follow-through. Melee blows, novas and heals (no projectile) get effects and sounds from it as `<action>.fire`, placed from the kit (`MatchClient.action_info`).
  - Instant casts (supports, shields) now emit `CastStarted` with `fire_at == at`, and the client pulses the animator so their clip and effects play. This covers Second Wind and any future support.
- **VFX and SFX:**
  - VFX: stone chips and dust where the maul lands, Cleave's ring on its true radius, gold motes for Second Wind, Lunge dust, a dusty Shockwave.
  - 13 sounds, 77 KB: swings, maul impacts, Cleave, chest thumps and a breath, the Lunge, and Shockwave's rise, boom, roll and crack.
- **Checked in Godot** against a duel server: the carry, Cleave's spin and slam, and Shockwave's slam on screen. The client log showed every effect and sound firing, including Second Wind's pulse.

**Left for later:**
- the owner's approval of his concept;
- polish of the block-out swings;
- the maul's head twisting about its haft between keys (the aim fixes the haft, not the roll).

### A slice 7 status (2026-10-07): Ember ✅ done (concept awaiting approval)
The first of the other eight champions (6 in the A list). Designed without an owner brief; `concept.md` is a draft.
- **Look:** a lean pyromancer on `biped` v1, 2,920 triangles.
  - A charcoal long coat with gold-trimmed edges, and coat tails on an `extra_coat` chain (2 bones).
  - A tall ember-orange collar flaring behind his head.
  - Glowing ember cuffs and a chest rune (`emissive`); the team accent on a sash and forearm bands.
  - Short dark hair and amber eyes.
- **Clips:** all 30, every `fire` checked against his kit. Flicker (a blink) plays as a pulse.
- **Effects and sounds:**
  - VFX: fireballs, a white-hot lance, Cinder Bloom's embers and ring, the Flicker flash, a spinning gold Binding Sigil.
  - 14 sounds, 75 KB.
- **Runtime:** blinks now announce themselves as instant casts too, so Flicker and the Blink utility spell animate and get their `fire` effects.
- **Checked in Godot** in an Ember mirror duel: his whole kit fired its effects and sounds (fireball, Bloom, Lance, both blinks, Sigil), with the Sigil's spinning frame on screen.

**Left for later:** the owner's approval of his concept; polish.

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
- Mod and content pipeline (community champions, skins within readability rules, custom modes): server-pushed packs with signatures, the trust prompt, cache and blocklist ([11](11-content-packs-and-mods.md), D51).
- Localization framework.

## M6 — Federation
- Instance trust, cross-instance queues, federated ladders ([07 §7](07-hosting-and-trust.md#7-federation-post-m5-sketch)).

## Later / ideas
Arena mode, rotating modes, tutorial and practice tool, observer and caster tools, tournament bracket integration, "Classic-ish" retro map variant.
