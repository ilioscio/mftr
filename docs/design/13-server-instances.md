# 13 — One server, many matches

A server process listens on one port and plays whatever its players ask for: they connect, pick
a mode, and land in a **match instance** of it. Instances come and go with their players; a
busy server runs many at once (D57). This replaces one process (and one port) per mode.

## 1. Queues

What a server offers is a list of **queues**, each a mode on a map, played one of two ways:

| Part | Values |
|---|---|
| **Mode** | ARAM, ARAM: Mayhem, ARAM: Hyper (on The Bridge), Classic (on Crossroads, or on each map file the host lists: one queue per map) |
| **Side** | **Versus**: humans on both teams, bots filling in. **Co-op vs bots**: humans on blue, bots on red |

So a server with one extra map offers 5 × 2 = 10 queues. The host can narrow the modes, and
offer a mode on one side only (`aram-coop`, `classic-versus`). The
sandboxes (Duel, Minions, Dodge, Empty) aren't offered by hosted servers; `--scenario NAME`
still runs a single-queue server of any scenario for development and tests.

## 2. Joining

1. The client connects (the secure handshake, D40) and says hello without a queue. The server
   answers with its queues: for each, its name, its map, and how many humans are playing it.
2. The player picks one. The client says hello again with it.
3. The server **places** the player:
   - Among the queue's instances with room for a human, the one with the most humans; on a
     tie, one still in champion select, then the oldest. Room: a bot to replace that isn't
     holding a dropped player's seat (§4); in co-op, on blue.
   - None has room: a **new instance**, in champion select. At the host's limit of instances,
     the player is told the server is busy.
   - The team (Versus): the one with fewer humans that has a bot to replace; on a tie, blue.
4. In champion select, the player gets a slot as today. In a match already playing, the player
   gets a short champion select of their own (§3), then takes over a bot.

A spectator picks a queue too and watches its busiest instance.

## 3. Champions

- **ARAM modes** stay all-random: champion select deals each player a random champion their
  team doesn't have, with rerolls and the team bench (06 §2).
- **Classic** becomes a pick: any champion the team doesn't have yet (the opposing team may
  have it too). A player who doesn't pick before the countdown ends keeps the one dealt to
  them at the start.
- **Joining mid-match**, the player has 20 s to choose: in ARAM modes, a dealt champion with
  one reroll; in Classic, a pick. Then the bot's champion leaves and the player's appears in
  the fountain with the bot's experience (so its level and ability points), and gold equal to
  the bot's gold plus the full price of its items (a refund: the player buys their own build).
  It's recorded in the replay as a *takeover*, which re-simulates like a join.

## 4. Drops

- A player whose connection goes silent (10 s) is replaced by a bot at once. The bot keeps
  the champion, its items and its gold, and plays on (shopping only steps of its own build
  path that still fit).
- The seat stays the player's for the rest of the match: nobody joining takes it over. When
  the player reconnects (the same session token, from any address), the bot hands the
  champion back as it is then.
- When a match ends and champion select starts again, dropped players' seats are freed.

## 5. Instances

- An instance is what one server process ran before: a match, its bots, champion select, its
  connections and its replay. The server routes each connection to its instance.
- An instance with no humans (connected, in champion select, choosing a champion, or holding
  a dropped seat) stops. One whose humans have all dropped stops 60 s after the last one did,
  so a solo player with a flaky connection can still come back.
- On a `--scenario` server (development) a spectator can watch bots alone: a match starts for
  them if none runs, and it keeps running while anyone watches. When its bots take every seat
  (`--bots` counts toward `--max-players`), a player is told the server is full; no match is
  started for them.
- When a match ends, its players go back to champion select together, as today.
- Instances tick on one shared grid, so they're due together, and step in parallel on a pool
  of worker threads (one per core by default, `--workers N`). The threads live as long as the
  server: what a thread keeps between ticks (pathfinding's scratch grid, megabytes) is built
  once. The results are exactly those of stepping the matches in turn.
- Each one's replay is written to the replay directory as
  `<started>-<queue>-<instance>.replay`.

## 6. Cost

A Classic match with ten bots simulates a tick in about 0.1 ms (release build). That's 0.3%
of a core at 30 ticks a second, so the simulation isn't what limits the count of matches. The
per-player work is: fog of war, snapshot deltas, encryption and bandwidth (§8 has the
measurements).

## 7. Hosting

`mftr-server` offers every full-match queue by default:

```
mftr-server [--bind ADDR] [--key FILE] [--modes aram,mayhem,hyper,classic]
            [--map FILE.mftrmap]... [--max-matches N] [--replay DIR]
mftr-server --scenario NAME [...]   # one queue, any scenario (development)
```

NixOS: one `services.mftr` server replaces `services.mftr.servers.<name>`, and
`services.mftr.sandboxes.<name>` runs the Duel Sandbox, the dodge rig and the other test
grounds as servers of their own (hosting.md).

## 8. Measurements

`mftr-tools loadtest` runs many full matches in one hub, in process: ten simulated players per
match click around the whole map (moves and stops, 3–8 clicks a second in bursts) while server
bots fight in the other seats. It times only the server's work (no sockets or encryption).
Measured on a 16-thread desktop (release build, Windows), 20 s per run, with jump-point
search (D58):

| Mode | Matches | Workers | Time per tick (avg / worst) | Share of one core | Per match | Down per player |
|---|---|---|---|---|---|---|
| ARAM | 1 | 1 | 0.11 / 5.3 ms | 0.3% | 0.3% | 8.1 KB/s |
| ARAM | 8 | 1 | 0.87 / 7.4 ms | 2.7% | 0.3% | 8.2 KB/s |
| ARAM | 32 | 1 | 3.5 / 10 ms | 11% | 0.3% | 8.3 KB/s |
| Classic | 1 | 1 | 0.23 / 12 ms | 0.7% | 0.7% | 8.5 KB/s |
| Classic | 8 | 1 | 1.9 / 20 ms | 5.9% | 0.7% | 8.6 KB/s |
| Classic | 8 | 8 | 1.1 / 17 ms | (wall) 3.4% | | 8.6 KB/s |
| Classic | 32 | 8 | 2.9 / 22 ms | (wall) 9.0% | | 8.7 KB/s |

Before D58 (A* over every cell), the same machine:

| Mode | Matches | Workers | Time per tick (avg / worst) | Share of one core | Per match | Down per player |
|---|---|---|---|---|---|---|
| ARAM | 1 | 1 | 0.25 / 13 ms | 0.8% | 0.8% | 8.1 KB/s |
| ARAM | 8 | 1 | 2.1 / 30 ms | 6.2% | 0.8% | 8.2 KB/s |
| ARAM | 8 | 8 | 1.5 / 27 ms | (wall) 4.4% | | 8.2 KB/s |
| ARAM | 32 | 1 | 8.6 / 37 ms | 26% | 0.8% | 8.3 KB/s |
| ARAM | 32 | 8 | 3.7 / 30 ms | (wall) 11% | | 8.3 KB/s |
| Classic | 1 | 1 | 1.1 / 80 ms | 3.3% | 3.3% | 8.5 KB/s |
| Classic | 8 | 1 | 9.4 / 131 ms | 28% | 3.5% | 8.6 KB/s |

A real server (encrypted UDP on loopback) with two full ARAM matches of headless clients used
3.1% of one core in all, about 1.6% per match: sockets and encryption double the in-process
cost. It sent 172 KB/s (8.6 KB/s per player) and its ticks took 0.7 ms (worst 2.5 ms).

What it means:
- **The simulation is cheap; players cost.** Ten bots alone simulate an ARAM tick in 0.015 ms
  and a Classic one in 0.034 ms. A match's cost is its players' snapshots (vision, deltas,
  encoding) and **pathfinding** for their clicks. With A* over every cell, a path to a random
  point averaged 0.27 ms on The Bridge and 1.2 ms on Crossroads (worst 10 ms), on grids of 425²
  and 580² cells; jump-point search (D58) brings that to 0.07 ms and 0.20 ms (worst 1.2 ms):
  `cargo test --release -p mftr-sim path_bench -- --ignored --nocapture`.
- **Spikes come from clicks landing together.** Each test match has one big tick, when all ten
  players' first clicks arrive at once and ten long paths are searched out of the fountain:
  12 ms on Crossroads now (80 ms with A*), 5 ms on The Bridge. Real players don't click in
  lockstep.
- **Capacity:** per core, about 150 ARAM matches or 70 Classic ones (with networking, which
  about doubles the in-process cost, against the 33 ms tick). Bandwidth: 85 KB/s per match
  (0.7 Mbit/s), so a 100 Mbit/s uplink carries about 140 matches. On a small VPS, bandwidth or the default `--max-matches 32` comes first.
