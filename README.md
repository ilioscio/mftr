# MFTR — Moba For The Rest (of us)

> Working title. A free, open-source, self-hostable 5v5 MOBA built on the Godot engine.

MFTR aims to deliver the thing players love about the best-known lane-based MOBA: the **realtime feel**. You can see skillshots coming and react to them, and good mechanics let you dodge them. It does this without kernel anti-cheat, paid skins, a launcher-sized download, or depending on one company's servers.

- **5v5, three lanes, jungle, epic objectives.** We start from the familiar formula, then make it our own.
- **Netcode first.** Server-authoritative, built so that what you see is what the server decides.
- **Self-host in one binary.** Run your own server for friends, a community or a tournament. Federation comes later.
- **Tiny download.** Stylized low-poly art with materials generated from noise in shaders, so there are almost no texture files.
- **Linux is first-class.** So are Windows and macOS.

## Status

**M0 (foundations)** is in place: a deterministic Rust simulation, the netcode core (sub-tick commands, own-champion prediction, reconciliation, clock and margin control loops), a dedicated UDP server, a headless Netcode Lab, and a Godot 4.7 client you can move around in. Design docs: start with the [design index](docs/design/README.md). M0 results: [roadmap](docs/design/08-roadmap.md#m0--foundations).

## Build & run

Requirements: Rust (stable, via [rustup](https://rustup.rs)) and [Godot 4.7](https://godotengine.org). On Windows, Rust needs the MSVC C++ build tools.

Start a server:

```bash
cargo run --release -p mftr-server -- --bind 127.0.0.1:7777
```

Build the Godot extension (once, and after Rust changes):

```bash
cargo build -p mftr-gdext
```

Then open `client/project.godot` in Godot 4.7 and press Play. Right-click moves, S stops and F1 toggles the net graph. To join another machine's server, pass its address as a user argument: `godot --path client -- 192.168.1.10:7777`.

Headless tools:

```bash
cargo test --workspace --release
```

```bash
cargo run --release -p mftr-tools -- netlab --profile all
```

```bash
cargo run --release -p mftr-tools -- bot --server 127.0.0.1:7777 --profile rough
```

## Repository layout

| Path | What |
|---|---|
| `crates/mftr-sim` | Deterministic simulation: ticks, movement, combat math, analytic projectiles |
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
