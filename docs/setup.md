# Setting up to develop MFTR

What you need: **Git**, **Rust** (through rustup), **Godot 4.7.2** and, on Windows, the
**Microsoft C++ build tools** (Rust links with them). **Blender 5.2** only if you work on
champion models and animations ([art guide](../art/README.md)).

## Windows

1. Install Git, if you don't have it: `winget install Git.Git` (or [git-scm.com](https://git-scm.com/download/win)).
   Open a new terminal afterwards.
2. Get the code, into a short path such as `C:\dev\mftr`:
   ```bash
   git clone https://github.com/ilioscio/mftr.git C:\dev\mftr
   ```
   The build writes paths up to ~135 characters inside the checkout and the C++ linker can't
   use one over Windows' 260, so a deep folder fails to build (`LNK1104: cannot open file`).
3. Run the setup from that folder:
   ```bash
   scripts\setup.cmd
   ```

The setup checks each tool and, for anything missing, asks to install it with winget: the C++
build tools (a few GB, in their own installer window), Rust and Godot 4.7.2. It remembers where
Godot is (the `GODOT` user variable), builds the client library and the server, and imports the
Godot project, so the first run starts quickly. The first build takes a few minutes. It ends
with what's missing, or with what to run next.

Run it again any time: it's also the check for a machine that stopped building.
`scripts\setup.cmd -Yes` installs without asking, `-Art` adds Blender, `-NoBuild` skips the
build.

### Running it

| Command | What it does |
|---|---|
| `scripts\play.cmd` | Builds, starts a server on `127.0.0.1:7777` in its own window, opens the client's menu. Closing the client stops the server |
| `scripts\play.cmd -Join` | The same, joining the server at once |
| `scripts\play.cmd -MapEditor` | The map editor (no server) |
| `scripts\play.cmd -Editor` | The Godot editor on the project (press Play to run it) |
| `scripts\play.cmd -ServerArgs '--scenario','duel'` | A server with other options (`mftr-server --help`) |
| `scripts\play.cmd -NoServer` | Just the client, for a server that's already running |
| `cargo test --workspace` | The Rust tests |
| `cargo fmt --all` and `cargo clippy --workspace --all-targets -- -D warnings` | Formatting and the lints CI checks |

The server and client from `play.cmd` are this checkout's: it builds `target\` beside it.

### The scripts

Our PowerShell scripts (`scripts\*.ps1`) have a `.cmd` launcher beside them: `setup.cmd`,
`play.cmd` (for `play-local.ps1`) and `bump-version.cmd`. Windows doesn't run `.ps1` scripts
by default (PowerShell's execution policy; running `play-local.ps1` directly says *running
scripts is disabled on this system*). A launcher runs its script through Windows PowerShell for
that one run whatever the policy, without changing it, so it works from cmd, PowerShell or a
double-click.

A few tools are shell scripts (`scripts\*.sh`, such as `package-client.sh`): run them from Git
Bash, which comes with Git: `bash scripts/package-client.sh windows`.

### Installing by hand

The setup does these for you; they're here if you'd rather, or if winget isn't available.

| Tool | Why | winget | Download |
|---|---|---|---|
| Git | the code, and Git Bash | `Git.Git` | [git-scm.com](https://git-scm.com/download/win) |
| C++ build tools | Rust's linker on Windows | `Microsoft.VisualStudio.2022.BuildTools` with the *Desktop development with C++* workload | [visualstudio.microsoft.com](https://visualstudio.microsoft.com/visual-cpp-build-tools/) |
| Rust | the game, server and tools | `Rustlang.Rustup` | [rustup.rs](https://rustup.rs) |
| Godot 4.7.2 | the client (the standard build, not .NET) | `GodotEngine.GodotEngine --version 4.7.2` | [godotengine.org](https://godotengine.org/download/archive/4.7.2-stable/) |
| Blender 5.2 | champion art only | `BlenderFoundation.Blender` | [blender.org](https://www.blender.org/download/) |

`rust-toolchain.toml` names the Rust version and its components; the setup installs them (by hand: `rustup toolchain install stable --component rustfmt --component clippy`).

## Linux and macOS

Install Git, [rustup](https://rustup.rs) and [Godot 4.7.2](https://godotengine.org/download/archive/4.7.2-stable/)
(on Linux, your distribution's C compiler too). Then:

```bash
cargo build -p mftr-gdext
```

```bash
cargo run --release -p mftr-server -- --bind 127.0.0.1:7777
```

```bash
godot --path client
```

The first builds the client library (again after Rust changes), the second starts a server and
the third opens the client (`godot --path client -- --map-editor` for the map editor,
`godot --path client --editor` for the Godot editor). **NixOS / Nix:** `nix develop` gives you all of it; inside the shell `$MFTR_GODOT` is
Godot. The [README](../README.md#build--run) has the rest: game modes, scenarios, bots, tools.

## Godot's version

Godot 4.5 or newer runs the project, and release builds export with 4.5. We edit with
**4.7.2**: a different version rewrites files the other saved (`.uid` files, scene formats), which
shows up as changes nobody made. `play.cmd` removes `.uid` files Godot makes during a run.

## Editor

Any editor works. VS Code offers the extensions the repo recommends when you open the folder:
rust-analyzer, the Godot tools (GDScript) and TOML support.

## Testing a branch

Switch to it in your checkout (`git fetch`, then `git switch <branch>`) and run as usual: the
build picks up the changes.

Claude Code sessions work in **worktrees**: separate checkouts of the repo in
`.claude\worktrees\<name>`, each on its own branch. Your main checkout stays on its branch, so
you won't see a worktree's changes there, and git won't switch your checkout to a branch a
worktree has checked out. Run that worktree's own launcher instead:

```bash
.claude\worktrees\<name>\scripts\play.cmd -MapEditor
```

Each checkout builds into its own `target\`, so its first build is a full one.
