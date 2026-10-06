# MFTR — Moba For The Rest (of us)

> Working title. A free, open-source, self-hostable 5v5 MOBA built on the Godot engine.

MFTR aims to deliver the thing players love about the best-known lane-based MOBA: the **realtime feel**. You can see skillshots coming and react to them, and good mechanics let you dodge them. It does this without kernel anti-cheat, paid skins, a launcher-sized download, or depending on one company's servers.

- **5v5, three lanes, jungle, epic objectives.** We start from the familiar formula, then make it our own.
- **Netcode first.** Server-authoritative, built so that what you see is what the server decides.
- **Self-host in one binary.** Run your own server for friends, a community or a tournament. Federation comes later.
- **Tiny download.** Stylized low-poly art with materials generated from noise in shaders, so there are almost no texture files.
- **Linux is first-class.** So are Windows and macOS.

## Status

**M0 (foundations)** is in place: a deterministic Rust simulation, the netcode core (sub-tick commands, own-champion prediction, reconciliation, clock and margin control loops), a dedicated UDP server, a headless Netcode Lab, and a Godot client. **M1 (Duel Sandbox)** is in progress: unit collision, skillshots and the dodge rig, an arena with walls, brush and fog of war, and placeholder champions you can duel with (slices 1–4; six since M2). Design docs: start with the [design index](docs/design/README.md). Results so far: [roadmap](docs/design/08-roadmap.md).

## Build & run

Requirements: Rust (stable, via [rustup](https://rustup.rs); `rust-toolchain.toml` pins the channel) and [Godot 4.5 or newer](https://godotengine.org). On Windows, Rust needs the MSVC C++ build tools.

**NixOS / Nix:** `nix develop` (flakes enabled) gives you the pinned Rust toolchain, Godot and the runtime libraries Godot needs. The first run creates `flake.lock`; commit it. Inside the shell, `$MFTR_GODOT` points at the Godot binary.

Start a server:

```bash
cargo run --release -p mftr-server -- --bind 127.0.0.1:7777
```

Build the Godot extension (once, and after Rust changes):

```bash
cargo build -p mftr-gdext
```

Then open `client/project.godot` in Godot (4.5+) and press Play. The server starts the **Duel Sandbox** by default: blue spawns west, red east, with minion clumps in between. Six placeholder champions: **Ember** (skillshot mage), **Vesper** (marksman), **Bastion** (tank: pull, knock-up), **Rook** (bruiser: cleave, heal, lunge), **Lumen** (enchanter: heals and shields allies) and **Shade** (assassin: lunges). Duels alternate Ember and Vesper and ARAM hands them out in turn; or pick one with a user argument: `godot --path client -- --champion shade`.

| Input | Action |
|---|---|
| Right-click ground / enemy | Move / attack |
| A, then left-click | Attack-move |
| Q W E R | Abilities, cast at the cursor (skillshot, delayed area, dash or blink, hard-CC skillshot) |
| D / F | Blink / Barrier |
| Ctrl + Q / W / E / R | Spend an ability point (ARAM) |
| P | Shop (ARAM: buy while dead or in your fountain) |
| S | Stop |
| F1 / F2 | Net graph / client collision proxies (to feel the difference) |

**Blind playtest** (helps us tune netcode against how it *feels*): start the client with `godot --path client -- --blind` (add a server address if it isn't local). You'll play 10 one-minute rounds under hidden network conditions and rate each one. Your answers go to `blind_results.tsv` in Godot's user data folder (the path is shown at the end). Send us that file; `mftr-tools blind-report blind_results.tsv` summarizes it. The [playtest guide](docs/playtest.md) has the full steps for testers and organizers.

**ARAM on The Bridge** (M2, in progress): start the server with `--scenario aram`. You get one lane with turrets, a Gatehouse and a Base per team, minion waves every 30 s, health relics and a fountain; destroy the enemy Base to win. You start at level 3 with 1,400 gold: press **P** in the fountain to shop. Bots (`mftr-tools bot --duel`) also play it, though not well yet.

**Hosting:** see the [hosting guide](docs/hosting.md). The short version: `docker compose -f deploy/compose.yaml up -d` runs an ARAM server (champion select, bots filling empty slots) on UDP 7777 and a duel server on 7778. On NixOS, import the flake's module and set `services.mftr.enable = true` (see the guide).

**Releasing:** run `scripts/bump-version.sh X.Y.Z` (it updates `Cargo.toml`, `Cargo.lock` and the macOS export preset together; CI builds with `--locked`, so a hand-edited version fails), merge that to `main`, then tag the merge commit on `main`: `git tag vX.Y.Z && git push origin vX.Y.Z`. The tag builds and publishes the release from exactly that commit.

**Champion select, spectating, reconnect:** add `--lobby` to an ARAM server (`--scenario aram --bots 10 --lobby`) for all-random champion select with rerolls and a team bench (humans replace bots). Start the client with `-- --spectate` to watch (Tab cycles champions). If the client crashes or the connection drops, restart it within a minute and you get your champion back.

**Bots and replays:** `--bots N` fills N slots with server bots (e.g. `--scenario aram --bots 9` for a full match against bots), and `--replay match.replay` records the session. `mftr-tools replay match.replay` re-simulates a recording and checks it, and `mftr-tools botmatch --seed 3` plays a 10-bot ARAM match headless in a couple of seconds.

No one to duel? Start a sparring bot: `cargo run --release -p mftr-tools -- bot --server 127.0.0.1:7777 --duel --seconds 600`. Other scenarios: `--scenario aram` (see above), `--scenario minions` (minion-block sandbox), `--scenario dodge` (turrets that fire skillshots at you) and `--scenario empty`. To join another machine's server, pass its address as a user argument: `godot --path client -- 192.168.1.10:7777`.

Headless tools:

```bash
cargo test --workspace --release
```

```bash
cargo run --release -p mftr-tools -- netlab --profile all --scenario dodge
cargo run --release -p mftr-tools -- netlab --profile mid --clients 2 --scenario duel
cargo run --release -p mftr-tools -- blind-report blind_results.tsv
```

```bash
cargo run --release -p mftr-tools -- bot --server 127.0.0.1:7777 --profile rough
```

## Repository layout

| Path | What |
|---|---|
| `crates/mftr-sim` | Deterministic simulation: ticks, movement, pathing, vision, champions, combat, analytic projectiles |
| `crates/mftr-net` | Wire protocol: bit-packing, packets, messages, link conditioner, clock sync |
| `crates/mftr-client` | Engine-independent client runtime: prediction, reconciliation, interpolation |
| `crates/mftr-server` | Authoritative server core and the `mftr-server` UDP binary |
| `crates/mftr-tools` | Netcode Lab (headless, link-conditioned) and UDP bot |
| `crates/mftr-gdext` | Godot GDExtension (`MatchClient` node) |
| `client/` | Godot project: scenes, shaders, GDScript |
| `docs/design/` | Design documents and decision log |

## License

Code: **AGPL-3.0-or-later**. Art and content: **CC BY-SA 4.0**. See [naming & legal](docs/design/09-naming-and-legal.md).

MFTR is an independent project, not affiliated with or endorsed by Riot Games. It aims to feel familiar to players of the genre while using only original names, characters, art, audio and text.
