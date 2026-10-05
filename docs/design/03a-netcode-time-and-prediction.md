# 03a — Netcode: Time, Prediction & Hit Resolution

Detailed spec behind [03 — Netcode](03-netcode.md). The wire format is in [03b](03b-netcode-wire-protocol.md).

## 1. Notation

| Symbol | Meaning |
|---|---|
| `Δ` | Tick length, 1/30 s ≈ 33.3 ms |
| `k` | Tick index. Tick `k` simulates the interval `(t_{k−1}, t_k]`, where `t_k = k·Δ` |
| `ℓ↑`, `ℓ↓` | One-way latency client→server and server→client. `RTT = ℓ↑ + ℓ↓` |
| `m` | **Input margin:** how early commands should arrive before their tick is simulated, to absorb jitter |
| `B` | **Interpolation buffer** for remote entities |
| `T_now` | Client's estimate of current server time (fractional ticks) |
| `T_input` | `T_now + ℓ↑ + m`: the server time at which a command sent *now* takes effect |
| `T_interp` | `T_now − ℓ↓ − B`: the server time remote entities are drawn at |
| `Δtl` | **Timeline gap** `T_input − T_interp = RTT + m + B` |

Typical values:

| Network | RTT | m | B | Δtl |
|---|---|---|---|---|
| Good | 30 ms | 5 ms | 33 ms | ~70 ms |
| Typical | 60 ms | 8 ms | 33 ms | ~100 ms |
| Rough | 120 ms, 20 ms jitter | 25 ms | 66 ms | ~210 ms |

## 2. The key property: tick-indexed consistency

Commands carry an **explicit target tick and sub-tick time** (§3). If a command arrives in time, the server applies it **exactly** where the client said. As a result:

- The client's predicted own-champion state for tick `k` is the server's state for tick `k`, as long as nothing the client doesn't know about yet interferes (CC, collisions, slows).
- An enemy projectile's position at tick `k` is a closed-form function of its spawn event, so it is also exactly the server's value.

**Errors in the wall-clock estimate only shift *when* you see a frame, not *what* the server will decide about it.** If the clock estimate is 10 ms off, you see the right outcome 10 ms early or late; you don't see a wrong one.

What actually breaks dodge honesty:
1. **Late commands:** the server has to apply them at a later tick than predicted. → The margin control loop (§10) is the most important tuning loop in the netcode.
2. **Server-side influences on your champion the client doesn't know yet:** CC, slows, displacement, unit collision. → §4 and §5.
3. **Projectile changes the client can't see coming:** blocked by an unseen wall ability, a spell shield, and so on. → §7.

Enemy projectiles are **never speculative**. The client only learns about a projectile from the server's spawn event, after it was really fired. The client extrapolates it in *time*; it never predicts that one *exists*.

## 3. Sub-tick command timing

At 30 Hz, quantizing inputs to tick boundaries would add 0–33 ms of random input delay. Instead each command carries:

- `target_tick`: the tick whose interval contains the command time, and
- `sub`: the position inside that interval in 1/64 steps (~0.5 ms resolution).

The server applies the command at time `t_{k−1} + sub/64 · Δ`. Within tick `k`, a unit moves under its old order until that instant and under the new order afterwards. A cast's windup starts at that instant, so the fire time and the projectile spawn time are exact real-valued times, not tick-rounded.

- **Cost:** movement integration splits at most a few times per tick, and the work is trivial.
- **Cheating:** none possible. A command must still *arrive* before its tick is simulated, so a client can't retroactively act on information. Choosing an earlier sub-tick is the same as running a tighter margin, which only risks late commands.

## 4. Own-champion prediction loop

```text
every render frame:
    T_input  = clock.input_time()                  // fractional ticks
    k_target = ceil(T_input)

    for each new player input:
        cmd = Command { tick: k_target, sub: frac(T_input) * 64, kind }
        net.send(cmd)                              // immediately, not at frame end
        pending.push(cmd)
        mark ticks >= k_target dirty               // re-sim the current partial tick

    predict_until(k_target)                        // own champion + collision proxies (§5)
    render own champion at lerp(state[floor(T_input)], state[k_target], frac(T_input))
    render_offset = decay(render_offset)           // §4.2

on authoritative snapshot for tick s:
    pending.remove_where(cmd.tick <= s)
    if history[s] == server_own_state(s):          // bit-exact; own state is sent lossless (03b)
        return                                     // the common case: nothing to do
    old_pos = rendered_position()
    state[s] = server_own_state(s)
    resimulate s+1 .. predicted_tick with pending commands and known world events
    render_offset += old_pos − rendered_position()
```

### 4.1 History
The client keeps a ring buffer (~1 s) of predicted own state per tick: position, path cursor, cast/attack state, cooldowns, resource, active statuses. The comparison in `on snapshot` is exact. That's possible because the own champion's authoritative state is sent **lossless** (no quantization), and the sim is deterministic (see [03 §17](03-netcode.md#17-determinism-policy)).

### 4.2 Correction smoothing
| Correction size | Treatment |
|---|---|
| < 2 u | Absorb silently, no visible change |
| 2–100 u | Exponential visual blend, half-life ~50 ms. The sim state is already correct; only the visual is smoothed |
| > 100 u, or caused by a server displacement event | Snap; the displacement plays as an animation |

The server tells the client the **cause** of every own-champion state change it originated (status applied, displacement, collision), so the client can also tag corrections by cause for the Netcode Lab.

### 4.3 Own casts
- Windup animation, cooldown, resource cost and indicator start **immediately** on T_input.
- Own projectiles spawn at the predicted fire time. Their parameters are computed with the same sim code, so if the cast is accepted the server's projectile is bit-identical.
- If the server rejects the cast (silenced, insufficient resource after a server-side drain, out of range after a correction): roll back cooldown and resource, cancel the animation, and play a short "fizzle" cue.

## 5. Predicting unit collision (minion block)

### The problem
Collision radii are small (champion 35 u, minion 25 u), so champion–minion contact distance is ~60 u. But remote minions are known on T_interp, a full Δtl behind the predicted champion:

| Network | Δtl | Minion displacement in Δtl (325 u/s) |
|---|---|---|
| Typical | ~100 ms | ~33 u |
| Rough | ~210 ms | ~68 u |

The positional error is about the same size as the contact distance. A naïve prediction (collide with minions where they're *drawn*) gets blocks wrong constantly: you get stopped by empty space, or walk through a clump and then get rubber-banded back.

### The approach

**1. Collision proxies on T_input.** For prediction, every unit within ~600 u of the own champion gets a *proxy* extrapolated to each predicted tick using its **replicated movement intent**:
- **Minions:** along their replicated path. If attacking or stationary, they stay put. Minion AI is very predictable over 100–200 ms horizons (walking the lane, standing to attack, stepping into a free attack slot).
- **Champions** (ally and enemy): along the waypoints of their current move order, capped at Δtl. Humans re-click often, so these are less reliable, but champion body-blocks are rarer than minion blocks.
- Proxies feed into the **same collision code the server runs** (below). Correct proxies give correct blocks.

**2. Draw the local bubble on the same timeline.** Minions near the own champion are drawn at a **blended display time**:

```text
display_time(unit) = lerp(T_interp, T_input, w),   w = smoothstep(900 u, 400 u, distance_to_own_champion)
```

So the minion you bump into is drawn exactly where you bump into it. Because minions move at steady speeds, the blend only shows as a slight speed change while a minion crosses the 400–900 u band. That's acceptable for minions. **Champions are not blended:** they're the focus of aiming and targeting, and their extrapolation is unreliable, so they stay on T_interp (their proxies are still extrapolated).

Health bars always show **confirmed** values, so last-hitting reads true health (see open question in §12).

**3. Prediction-friendly server collision.** The collision algorithm itself must be deterministic, local and order-independent within a tick:
- Each moving unit computes its desired motion for the tick.
- It resolves contacts against other units' **start-of-tick** positions, sliding along the contact tangent when blocked. The result doesn't depend on processing order.
- A second pass catches mover–mover overlaps: each overlapping pair pulls back to the largest non-overlapping fraction of their motion (deterministic, by entity ID).
- A unit that makes < 20% progress for 3 ticks **re-paths** around the clump. It uses a local occupancy grid built from collision circles (deterministic A*, radius ~400 u), then rejoins its navmesh path.
- No shoving, ever. Dashes, blinks and ghosted units skip unit contacts.

**4. Measure it.** The Netcode Lab tags every own-champion correction by cause. *(start)* target: at 80 ms RTT, fewer than one collision-caused correction > 20 u per minute of laning.

## 6. Hit resolution (server)

### Analytic projectiles
A linear projectile is a closed-form function of its spawn parameters:

```text
p(t) = origin + dir · speed · (t − t_spawn),   for t ∈ [t_spawn, t_spawn + range / speed]
```

Arcing and delayed-area deliveries have their own closed forms. Projectile positions never accumulate per-tick error, and the client computes exactly the same values.

### Swept, simultaneous collision
For each tick `k`, over `(t_{k−1}, t_k]`:
- The projectile center moves along a segment (clipped to its lifetime).
- Each candidate target's center moves along its **piecewise-linear motion for that tick**, as recorded by the movement system. Segments split at waypoint turns and sub-tick command times.
- For each linear piece, solve **closest approach in the relative frame**. With relative position `r(τ) = a + b·τ`, the squared distance `|a + bτ|²` is a quadratic in `τ`, minimized on the piece's interval. A hit occurs if `min |r| ≤ r_proj + r_gameplay(target)`.
- **Exact and tunnel-free:** fast projectiles can't skip over targets, and the result doesn't depend on update order.
- With several candidates in one tick, the **earliest contact time** wins (ties: lower entity ID). This decides "the first minion blocks the hook".
- Width convention: an ability's `width` is the projectile's **diameter**, so `r_proj = width / 2`.

The client runs **the same function** for "enemy projectile vs. my own champion" (§7), on the same timeline the server uses.

## 7. Display policy: what is drawn when

**Principle: never show the local player a favorable outcome the server hasn't confirmed, and never hide a threat on prediction alone.**

| Thing | Drawn on | Predicted? | If the server disagrees |
|---|---|---|---|
| Own champion | T_input | Fully | Blend or snap (§4.2) |
| Own cooldowns, resource | T_input | Yes, on cast | Roll back on reject |
| **Enemy projectiles** | **T_input**, from the first visible frame | Path: exact. Hit on own champion: exact. Interception by other units: via proxies | — |
| Predicted enemy hit **on own champion** | Impact VFX at the predicted tick; a non-piercing projectile is hidden | Health change waits for the server | Server says miss: the projectile reappears at its true position (already past you, so no threat was hidden) |
| Predicted enemy **interception by minion/ally** | Impact VFX plays; the projectile turns into a dim "unconfirmed" outline that **keeps moving** | — | Server says no interception: restore it at full strength. The threat was never hidden |
| Enemy delayed ground AoE | Indicator from the cast event; detonation exact on T_input | Damage on you: same as projectiles | — |
| Remote cast windups | T_input (fast-forwarded by however late the event arrived) | No | Cast-cancel event stops the animation |
| **Own projectiles** | Spawn on T_input; then Option A or B (below) | Path: exact. Hits: **never** predicted | Hit VFX and damage numbers only on confirmation |
| Ally projectiles | T_interp (cast by and hitting units drawn on T_interp) | No | — |
| Remote champions, far minions, monsters | T_interp | Interpolation | — |
| Minions near you | Blended toward T_input (§5) | Extrapolation | Smooth |
| Health, gold, levels, others' cooldowns | Confirmed values | No | — |

### Remote windups go on T_input
Enemy projectiles are drawn on T_input, which is Δtl *ahead* of where the caster is drawn. If the caster's windup played on T_interp, the projectile would fly out before the caster finished winding up. So remote **cast animations** play on T_input, fast-forwarded by how late the event arrived. The caster's *position* stays on T_interp; casters are almost always rooted during windup, so the two agree. The **spawn streak** (a one-frame motion smear from the caster's drawn hand to the projectile's true position) ties them together visually.

### Own projectiles: Option A vs. Option B (decide in M1)
- **A — stay on T_input.** Always honest about where your projectile really is. But enemies are drawn Δtl in the past. Your projectile visibly passes through an enemy it hit, and the hit flash appears ~Δtl later; it also visibly "hits" enemies who had already dodged on the server.
- **B — blend from T_input to T_interp over the flight.** At cast, the projectile leaves your hand snappily (T_input). By the time it reaches enemies it's on their timeline, so the moment it touches a drawn enemy is approximately when the server confirmation arrives. The cost: the projectile appears to decelerate by `Δtl / flight_time` (≈ 15% at typical ping for a 0.7 s flight), starting after the first ~300 u.
- **Decision (D12): B for now**, to be confirmed by a blind A/B test in the M1 duel sandbox. Option B never affects threats to *you*, only how your own shots look.

## 8. Reaction budget: what the player actually gets

The player first learns about a skillshot when the cast-start event arrives. Their dodge command takes effect `ℓ↑ + m` later. Add local input-to-photon latency (input sampling, rendering, display: ~25 ms). The **latency loss** is:

```text
L = RTT + m + local ≈ RTT + m + 25 ms
```

| Network | L |
|---|---|
| 30 ms RTT | ~60 ms |
| 60 ms RTT | ~95 ms |
| 120 ms RTT, 20 ms jitter | ~170 ms |
| 180 ms RTT | ~235 ms |

### Why not a flat number: dodging also takes *movement* time
To dodge a centered linear skillshot, the target must move its center out of the hit corridor:
`t_move = (r_proj + r_gameplay) / MS`. For a 70-width shot vs. a 65-radius champion at 335 MS, that's `(35 + 65) / 335 ≈ 0.30 s`. **Walking out of a perfectly centered shot needs ~0.30 s of movement even with zero reaction time and zero ping.** A flat 0.45 s total leaves ~0.15 s for human reaction plus latency. That's below human visual reaction time (~0.2–0.25 s for a practiced player watching for a learned cue), even on LAN.

**The rule (D10, accepted):** derive the minimum from the ability's own geometry:

```text
T_needed = t_human + t_net + (r_proj + r_ref) / MS_ref
           t_human = 0.25 s      (practiced reaction to a learned cue)
           t_net   = 0.17 s      (the 120 ms RTT row above)
           r_ref   = 65 u, MS_ref = 335 u/s

reaction_time(d) = windup + d / speed           (linear)
reaction_time    = windup + detonation_delay    (delayed ground AoE; t_move uses the AoE radius)

rule: reaction_time(d_class) ≥ T_needed
      d_class = 80% of max range for hard CC, 90% for burst, 100% for poke
```

For a 70-width shot, `T_needed ≈ 0.72 s`. A wide (140) hook needs ≈ 0.82 s. The content linter also reports each ability's **guaranteed-dodge range**, the distance beyond which a centered shot is always walkable at reference ping. Inside it, the skillshot is a positioning check, not a reflex check, which is fine as long as it's deliberate.

Sanity check against the genre: common reference-game line skillshots (0.25 s windups, 1,200–2,000 u/s speeds, 1,000–1,300 u range) land around **0.7–1.3 s** at 80–100% range. The derived rule puts us in the same "dodgeable if you're paying attention" band, which is where the familiar feel lives. 0.45 s would make our hard CC noticeably *less* dodgeable than players expect. Measured confirmation: [R01 §4](reference/R01-video-ezreal-flash-barrier-q.md#4-mystic-shot-reference-line-skillshot).

## 9. Ghost-hit error budget

Sources of "I saw a dodge, the server hit me", their size at rough network conditions (120 ms RTT), and mitigations:

| Source | Typical size | Mitigation |
|---|---|---|
| Late command (applied a tick later than predicted) | 1 tick of movement ≈ 11 u | Margin loop: target ≥ 99% of commands on time |
| Slow applied by the server that the client learns late | `slow × MS × RTT` ≈ 0.3 × 335 × 0.12 ≈ 12 u | Unavoidable; small. Slows are visible on the caster's projectile, so players learn them |
| Hard CC on you while dodging something else | Large | Unavoidable; it's a real hit by something else |
| Unit-collision mispredict | 0–60 u | §5 proxies + bubble rendering |
| Projectile parameter mismatch | 0 | Lossless spawn parameters + deterministic sim |
| Clock estimate error | 0 for outcomes (§2) | Only affects display timing |
| Unseen projectile blockers / spell shields | Rare | Events carry ticks. The client applies known blockers to projectile prediction |

The **dodge rig** (03 §14) measures the actual rate per network profile, and every ghost hit is logged with its attributed cause.

## 10. Control loops

### 10.1 Clock
- Every server packet carries `server_tick + sub` at send time and an echo of the client's last timestamp plus the server's hold time (03b).
- Offset samples are filtered over a 2 s window, keeping the **lowest-RTT samples** (they have the least queuing noise). `ℓ↓` is estimated as `RTT_min / 2`. Path asymmetry doesn't matter, because the margin loop measures command arrival directly.
- The local sim clock runs at rate `1 ± ε` (|ε| ≤ 0.03) to converge on the estimate. It hard-resyncs only past 250 ms of error.

### 10.2 Input margin `m`
- The server reports for every command its **arrival lead**: how long before its target tick began simulating it arrived (0.5 ms units, negative = late).
- The client tracks the distribution and steers `m` so the **1st percentile of lead stays ≥ 2 ms** (≥ 99% of commands on time).
- Asymmetric gain: grow `m` fast after a late command, shrink it slowly (over seconds) when there's excess lead. Changes go through time dilation, never a jump.
- Late commands are still applied, at the earliest unsimulated tick with `sub = 0`, and reported. Commands more than 250 ms late are dropped.

### 10.3 Interpolation buffer `B`
- `B = Δ + 2σ`, where σ is the standard deviation of snapshot inter-arrival times over the last ~2 s. It's clamped to `[1, 6]` ticks and adapts by time dilation.
- On an empty buffer, extrapolate ≤ 1 tick along the path, then hold (never warp ahead).

## 11. Edge cases

- **Projectile enters vision mid-flight:** the spawn event is re-based to the entry point, keeping the same timing and path (the caster isn't revealed). It's drawn on T_input from there.
- **Caster dies or is interrupted during windup:** the server sends cast-cancel and no projectile ever spawns. The T_input windup animation just stops. Nothing speculative was shown.
- **Projectile blockers / spell shields:** their state changes are events with ticks. The client's enemy-projectile prediction honors blockers it knows about. Unseen blockers fall under the "unconfirmed outline" rule.
- **Displacement on own champion** (hook, knock-up): authoritative. The client receives the start tick, curve and duration, re-simulates, and plays it as an animation.
- **Server tick overrun:** the server runs catch-up ticks, and clients absorb the burst through B and m. Overruns are logged with profiling data.
- **Packet-loss burst:** commands survive through redundancy (03b); prediction keeps going. When snapshots resume, reconciliation corrects any drift.

## 12. Open questions for M1 experiments

1. Own-projectile display: confirm Option B over A in a blind test (§7, D12).
2. Minion bubble blending on vs. off, and the band distances.
3. Should the minion health bar show a **predicted** value on T_input (including in-flight minion attacks), as a last-hit aid? Risk: mispredicted last hits feel awful.
4. 30 Hz vs. 60 Hz tick with sub-tick commands: is there any perceptible difference?
5. Validate `t_human = 0.25 s` and the class range fractions (§8) with blind playtests.
6. Collision proxy confidence: should recently re-pathed proxies be shrunk to bias toward late blocks over false blocks?
