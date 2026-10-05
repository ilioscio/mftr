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
