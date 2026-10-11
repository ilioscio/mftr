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

So a server with one extra map offers 5 × 2 = 10 queues. The host can narrow the modes. The
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
- When a match ends, its players go back to champion select together, as today.
- Instances step in parallel on worker threads (one per core).
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

NixOS: one `services.mftr` server (§ hosting.md) replaces `services.mftr.servers.<name>`.

## 8. Measurements

To be filled in by slice 4.
