# 03 — Netcode

This is the pillar the project lives or dies on. The goal, in one sentence:

> **When you see a skillshot miss you, it missed you — and when you see yourself dodge, the server agrees.**

## 1. Measurable targets

| Metric | Definition | Target @ 60 ms RTT | Target @ 120 ms RTT, 20 ms jitter, 1% loss |
|---|---|---|---|
| **Ghost hit rate** | Client showed a clean dodge, but the server registered a hit | < 0.5% of near-misses | < 2% |
| **Phantom dodge rate** | Client showed a hit, server registered a miss (own projectiles) | < 1% | < 3% |
| Own-champion correction | Mean visible position correction on own champion during normal play | < 5 u | < 15 u |
| Input-to-visual latency | Click → own champion responds on screen | ≤ 1 frame + render | same |
| Downstream bandwidth | Per player, late-game 5v5 teamfight | < 32 KB/s | same |
| Server CPU | Per match, late-game 5v5, one modern core | < 3 ms / tick | same |

These are measured continuously by the **Netcode Lab** (§14). A milestone doesn't ship until its netcode targets are met.

## 2. Model overview

- **Server-authoritative.** The server runs the only real simulation. Clients send **commands** ("move to (x, y)", "cast Q toward (x, y)", "attack unit #812") and receive **state**.
- **Fixed tick: 30 Hz** (33.3 ms), configurable. Genre precedent shows 30 Hz is enough for click-to-move games. The bottleneck for feel is *latency handling*, not tick rate. Raising the rate is a config change if measurements argue for it.
- **No lockstep, no rollback of the world.** The server never rewinds time for the attacker (there is no "lag compensation" for skillshots). This MOBA **favors the defender**: dodges are judged against the dodger's real server position.
- **Clients predict exactly one thing in full: their own champion** (movement, dashes, own casts and own projectiles). Everything else is interpolated or deterministically extrapolated.
- The client links the **same Rust simulation code** as the server (see [04](04-architecture.md)), so prediction runs identical logic, not an approximation.

## 3. The three client timelines

This is the key concept. The client shows different kinds of entities at **different points in time**, chosen so the player's decisions line up with how the server will judge them.

```
 server time  ──────────────────────────────────────────────────────────►
                     │                    │                    │
          T_interp = now − RTT/2 − buffer   T_now ≈ server "now"   T_input = now + RTT/2
                     │                                         │
         Remote champions, minions,                 Own champion (predicted),
         turrets: INTERPOLATED                      own casts, and ALL
         between received snapshots                 deterministic projectiles
                                                    (extrapolated)
```

- **T_input (the input timeline):** the server time at which a command you send *now* will be processed. Your own champion is shown here, because that is where your inputs land.
- **Projectiles are also shown on T_input.** Linear, arcing and delayed-area skillshots are fully determined by their spawn event (origin, direction, speed, width, spawn tick, and so on), so the client can compute *exactly* where they will be at T_input. Your champion and the incoming skillshot are therefore drawn **on the same timeline, the one the server will use to resolve the collision.** This is what makes dodging honest.
- **T_interp:** remote champions and minions are drawn slightly in the past, interpolated between two received snapshots. They're smooth and always correct, just late. An adaptive jitter buffer (§7) sets how far back.

### What this costs (and how we hide it)
- A projectile becomes visible having already traveled about **RTT × speed**. At 60 ms and 1600 u/s that is ~96 u, roughly one champion width. At 150 ms it's ~240 u. Mitigations:
  - **Cast windups are telegraphs.** The caster's windup animation and indicator start as soon as the cast-start event arrives, fast-forwarded by however late it arrived.
  - **Spawn streak:** on the first visible frame, draw a short motion streak from the caster's *rendered* hand to the projectile's true position. The eye registers "it just came out of them" without the projectile's hitbox being shown behind its true position.
  - **Reaction budget rule** (§9) keeps dodge-intended skills reactable at realistic pings.
- Remote champions are drawn ~(RTT/2 + buffer) in the past, so **your own skillshots will sometimes visually pass through an enemy who had already moved on the server.** That's the favor-the-defender trade. We never show hit effects or damage numbers for your projectiles until the server confirms, and the predicted projectile is destroyed when the server's hit or expire event arrives.

## 4. Clock synchronization

- Client and server exchange timestamped pings piggybacked on regular packets. The client keeps a filtered estimate of **RTT**, **jitter** and **server tick offset**: it uses the lowest-RTT samples in a sliding window (NTP-style) and rejects outliers.
- From these it derives `T_now`, `T_input = T_now + RTT/2 + input_margin` and `T_interp`.
- Changes to the estimate are applied with **time dilation**: the local sim speeds up or slows down by at most ~2–3%, rather than jumping. Visible snapping only happens when the error is > 250 ms.
- The server tells each client how early or late its commands arrive relative to the tick they targeted. The client adjusts `input_margin` to keep a small, stable cushion (target: commands arrive 0.5–1 tick early).

## 5. Upstream: commands

- Commands are **sent immediately** when the player acts, not batched to the next frame or tick.
- Each command carries a sequence number and its **target tick**. The server queues it and applies it at that tick if it arrives in time, otherwise at the next tick (and reports the lateness).
- **Redundancy:** each packet also includes the last N un-acked commands (N≈3–5), so a single lost packet never loses a click.
- Commands are small (≤ 16 bytes typical). Rate limits on the server reject spam (e.g., > 30 move commands/s are coalesced).
- **The server validates everything**: cooldowns, range, resource, CC state, ability ownership, and so on. Invalid commands are dropped, and the client prediction is corrected.

## 6. Downstream: snapshots and events

### Snapshots (unreliable, latest-wins)
- Sent at tick rate (30 Hz) to players, possibly 20 Hz to spectators.
- **Delta-compressed** against the most recent snapshot the client has acked.
- **Quantized:** positions to 0.25 u (16 bits per axis is enough for a 16,000 u map), facing to 8–10 bits, health to 16 bits, and so on.
- **Interest management:** a client only receives entities its team can see (§10). It gets full detail near the camera and the champion, and reduced rates for distant minions.

### Events (reliable, ordered)
Discrete things that must never be missed: cast started, projectile spawned, damage dealt, death, gold/XP, item bought, level up, chat, pings.
- Sent alongside snapshots and **repeated in every packet until acked**. This gives reliability over UDP with no head-of-line blocking for state.
- Events carry the **tick they happened on**. The client applies them on the correct timeline (projectile spawns on T_input, remote cast animations on T_interp, and so on).

## 7. Interpolation of remote entities

- Remote entities render at `T_interp = T_now − one_way_latency − buffer`.
- `buffer` adapts to measured jitter: typically 1–2 ticks (33–66 ms), and it grows under packet loss.
- If a snapshot is missing, **extrapolate up to 1 tick** along the last known path, then hold. Remote units never warp ahead.
- Movement in this genre is **path-based**: snapshots include the unit's current path waypoints. Interpolation follows the path instead of lerping straight across corners.

## 8. Own-champion prediction & reconciliation

- The client runs `mftr-sim` locally for **its own champion only**: pathfinding on the same navmesh, the same movement code, dashes and blinks, and its own cast state machine.
- Every command is applied locally at once (on T_input) and kept in a **pending buffer** until the server acks the tick it was processed on.
- When an authoritative snapshot arrives for tick `k`: reset the local champion to the server state at `k`, then **replay** the pending commands from `k+1` to the present.
- If the replayed position differs from what was being shown:
  - **< 5 u:** ignore it (absorbed).
  - **5–100 u:** visually blend the rendered position toward the corrected one over ~100 ms. The simulation state snaps; only the visual is smoothed.
  - **> 100 u**, or a server-applied displacement (knock-back, hook, stun): snap, and play the displacement animation.
- **Known sources of correction**, and how we minimize them:
  - *Collision with other units:* the predicted sim includes interpolated nearby units as soft obstacles. Remote units are drawn in the past, so how hard unit-vs-unit collision is directly affects how often you get corrected. How much minion-block to keep is an open question (see DECISIONS).
  - *CC hitting you:* you'll see a slight rubber-band when stunned mid-move. This can't be avoided with honest dodging, but it's mitigated by the projectile timeline above (you see the stun coming at the true time).
  - *Slows and speed buffs from others:* applied on receipt and replayed through.

## 9. Casting, telegraphs & the reaction budget

### Cast lifecycle (server)
`Command received → validated → Windup (cast time) → Fire (spawn projectile / apply effect) → optional Channel → Recovery`

- **Own casts are predicted.** The windup animation and indicator start immediately, and the projectile spawns locally at the predicted tick. If the server rejects the cast, the client rolls back the animation and cooldown, with a short "failed" cue.
- **Remote casts** show at T_interp, so their windup *begins* late by ~one-way latency + buffer. Their **projectile**, however, is placed exactly on T_input. The windup is a heads-up; the projectile is the truth.

### Reaction budget rule
Every ability intended to be dodgeable declares a **reaction class**. The design validator (CI) checks this:

```
reaction_time(range d) = windup + d / projectile_speed      (for linear skillshots)
reaction_time          = windup + detonation_delay          (for delayed ground AoEs)
```

| Class | Example | Minimum reaction time at 75% of max range |
|---|---|---|
| Hard CC skillshot (stun/root/hook/charm) | Hooks, long roots | ≥ 0.45 s |
| Burst skillshot | Large-damage line nukes | ≥ 0.40 s |
| Poke | Small-damage frequent skillshots | ≥ 0.30 s |
| Not dodge-intended (point-blank, point-and-click) | | n/a, must be balanced as unavoidable |

A 120 ms RTT player loses ~60–120 ms of that window, and must still have a human-scale reaction window left. These numbers are *(start)* values to be validated in the Netcode Lab with real players.

## 10. Fog-of-war culling

- **Vision is computed on the server every tick** (grid-based line of sight with walls and brush, 50–100 u cells), per team.
- A client **only receives entities its team currently sees**, plus a short grace period so units fade out smoothly instead of popping (the grace never shows a *position update* that wasn't actually visible).
- **Projectiles from unseen casters:** the spawn event is sent **when the projectile enters vision**, with its parameters re-based to that entry point. The caster's position is not revealed unless vision rules say so.
- Sounds and VFX of unseen units are culled server-side too.
- Map hacks become impossible by construction: the information never reaches the client.

## 11. Dashes, blinks and displacement

- **Own dashes and blinks:** predicted, including wall-crossing checks on the shared navmesh.
- **Remote dashes:** the event includes start, end, speed and start tick. They render on T_interp along the exact path.
- **Displacement applied to you** (hooks, knock-ups, pulls): authoritative from the server. They play as an animation from your shown position to the server path, catching up over ≤ 100 ms.

## 12. Packet loss, jitter, reconnect

- No single lost packet causes a lost command (redundancy) or a lost event (repeat-until-ack). Snapshot loss just delays interpolation.
- **Reconnect:** the server keeps the champion in the match (it stands still, and bot takeover is optional per server config). On reconnect the client receives a full snapshot plus a compact recent event history, and resumes in < 5 s.
- **Pause:** supported in custom games and tournaments, either admin-only or with a per-team pause budget.

## 13. Spectating & replays

- **Spectators** connect like clients with all-team vision **behind a delay** (configurable, e.g. 3 min for ranked/tournament, 0 for custom games with consent). They receive interpolated snapshots only.
- **Replays** record the authoritative **command log + RNG seed + build hash**, plus a **keyframe snapshot every 60 s** for seeking. Expected size: a few hundred KB per match.
- Replays play back by re-running `mftr-sim` (same build). For cross-version viewing, a replay can be "baked" to a snapshot stream (larger, but version-independent).
- The server records **per-tick state hashes**. A replay that diverges from its hashes is flagged, which doubles as our determinism canary.

## 14. Netcode Lab (tooling from day one)

- **Link conditioner** built into client and server: latency, jitter, loss, reordering and duplication, set per direction.
- **Headless match runner:** one server plus N scripted clients, run in CI with fixed seeds.
- **Dodge rig:** scripted turrets fire skillshots at a client whose input is driven by a "perfect dodger" bot that reacts only to what *its own client* renders. Ghost hits are counted automatically under each network profile. This is our regression test for Pillar 1.
- **Net graph overlay** in the client: RTT, jitter, loss, interp buffer, correction magnitude, bandwidth and snapshot age.
- **Blind playtest protocol:** players rate feel at random hidden latency profiles, so we tune against perception, not just numbers.

## 15. Transport & security

- **UDP**, using the netcode.io connection model: the lobby issues a **signed, encrypted connect token** for a specific server and match, and packets are encrypted and authenticated after the handshake. This stops spoofing and session hijacking with no central auth on the hot path.
- Candidate libraries (decide in M0 by prototype): `renet` + `renet_netcode` (game-oriented, channels built in), or `quinn` (QUIC datagrams + streams). Both are Rust, both run in-process on the client through gdext.
- The serialization format is our own: hand-written bit-packing (bitcode-style) with a protocol version in the handshake.

## 16. Bandwidth & CPU budget

Rough estimate for a late-game 5v5 fight: ~80 visible entities × ~10 bytes of delta × 30 Hz ≈ 24 KB/s per client before compression wins on static entities. Server upstream for a full match is ~250 KB/s (~2 Mbit/s), so a home connection can host a match.

Server tick budget: < 3 ms per match per tick on one core lets ~10 matches share a core with headroom.

## 17. Determinism policy

Determinism isn't required for server authority, but it buys us: (a) prediction that matches the server and needs fewer corrections, (b) tiny replays, (c) reproducible bot and balance simulations, and (d) desync detection.

Rules for `mftr-sim`:
- Pure function of `(state, commands, seed)` per tick. No wall-clock, no I/O, no threads inside a tick (or only order-independent parallelism).
- `f32` math under strict rules: no fast-math, no platform intrinsics in sim code, transcendentals via software `libm`, and no FMA unless it's explicit and used everywhere.
- No `HashMap` iteration in sim logic. Use ordered containers or fixed-seed hashers with stable ordering.
- Seeded RNG (PCG or ChaCha) owned by the sim state.
- **CI cross-platform check:** replay a recorded match on Linux x86_64, Windows x86_64 and macOS aarch64, and compare per-tick hashes.
- Fallback: if float determinism proves leaky, switch positions and timing to fixed-point (an open question, see DECISIONS).
