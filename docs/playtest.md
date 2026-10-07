# Blind playtest (M1 exit)

We tune the netcode against how it *feels*, not only against numbers. In a blind playtest you
play short rounds while the client secretly adds network delay (none up to 120 ms round trip)
and switches two display options. After each round you answer two questions. You never see
which condition was active; the answers file records it for us.

**What we need to know:** at 80 ms, does dodging skillshots feel fair in at least 80% of rounds?
And which own-missile display (A or B) and minion setting feels better? (Decisions D12, D30.)

## For testers (about 12 minutes)

1. Get the client: download `mftr-client-<version>-<os>` from the releases page and unpack it
   (or run from source: open `client/project.godot` in Godot 4.5+ and press Play).
2. Start it (`mftr.exe` on Windows, `./mftr.x86_64` on Linux). In the start menu, enter the
   address the organizer gave you, set **Join as** to **Blind playtest**, and press **Join**.
   The part after `#` in the address is the server's key fingerprint: with it, the client only
   talks to that server. The client remembers the server for next time.

   From a terminal instead: `./mftr.x86_64 -- --blind playtest.example.org:7777#8a0bb4a2aa55a37cf673e459749c6705`.

3. Play 10 rounds of one minute. Dodge the skillshots and fight back. Right-click moves, Q W E R
   cast at the cursor, D blinks, F shields.
4. After each round, answer honestly:
   - **Did dodging feel fair?** When something hit you, did it look like it should have? When
     you dodged, did it count?
   - **How responsive did your champion feel?** 1 (sluggish) to 5 (instant).
   - Optional notes: anything odd ("snapped back", "hit from far away", "felt floaty").
5. At the end the client shows where it saved `blind_results.tsv` (Godot's user data folder,
   e.g. `~/.local/share/godot/app_userdata/MFTR/` on Linux). Send that file to the organizer.

Please play from your normal setup (home internet is fine). The test adds its delay on top of
your real connection, and it measures that real connection too.

## For organizers

- Host a server people can reach (see [hosting](hosting.md)). The **dodge rig** is the best
  test of dodging, and a duel against the sparring bot covers fights:

  ```sh
  mftr-server --scenario dodge --bind 0.0.0.0:7777
  # or: mftr-server --scenario duel  +  mftr-tools bot --server 127.0.0.1:7777 --duel --seconds 3600
  ```

- Give testers the address with the key fingerprint, `host:7777#…`: the server prints it at
  start (`mftr-server --fingerprint` prints it too).
- Pick testers close to the server (a real round trip under ~30 ms keeps the added profiles
  meaningful).
- Collect the files and summarize them all at once:

  ```sh
  mftr-tools blind-report results/*.tsv
  ```

  It prints fairness and responsiveness per latency profile, per missile option and per minion
  setting, and the M1 verdict: fair in ≥ 80% of rounds at 80 ms. Around 10 testers × 10 rounds
  gives about 20 rounds at the 80 ms profile.
