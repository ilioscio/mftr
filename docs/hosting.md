# Hosting an MFTR server

An MFTR server is one small program (under 1 MB) on one UDP port. Players connect, pick what to
play (ARAM, ARAM: Mayhem, ARAM: Hyper or Classic 5v5, each Versus or Co-op vs bots), and the
server puts them in a match of it, starting a new match whenever the others are full and
stopping one when its players leave ([13](design/13-server-instances.md)). It keeps every match
authoritative and needs no database, accounts or other services. This guide gets it running on
a fresh Linux machine (a VPS or a spare PC).

> **Status:** M4 in progress (ARAM, ARAM: Mayhem and Hyper on The Bridge; Classic 5v5 on Crossroads; ten champions, bots). Every packet is encrypted and
> authenticated (D40), and the server has a key that players' clients pin (see [The server key](#the-server-key)).

## Quick start with Docker

```sh
git clone -b release https://github.com/ilioscio/mftr.git && cd mftr
docker compose -f deploy/compose.yaml up -d
```

The `release` branch is the newest release (see [Updating](#updating)); `git pull` and the same
`docker compose ... up -d --build` move to the next one.

This builds the server image and starts it on UDP 7777, recording every match. Players type
`your.server.address:7777` into the client's start menu (it remembers the servers they joined),
or start it with the address: `./mftr.x86_64 -- your.server.address:7777` (`mftr.exe` on
Windows; `godot --path client -- your.server.address:7777` from the source), then pick what to
play. Open the port in your firewall, e.g. `ufw allow 7777/udp`.

Without compose: `docker build -t mftr-server . && docker run -d -p 7777:7777/udp mftr-server`.
Anything after the image name replaces the default options (see below), for example
`docker run -d -p 7777:7777/udp mftr-server --modes aram,classic --max-matches 8`.

## Without Docker

Download `mftr-server-<version>-linux-x86_64.tar.gz` from the releases page (Windows and macOS
builds are there too), or build it yourself with Rust 1.88 or newer:

```sh
cargo build --profile dist -p mftr-server -p mftr-tools   # → target/dist/
./target/dist/mftr-server
```

To keep it running, a systemd unit (`/etc/systemd/system/mftr.service`):

```ini
[Unit]
Description=MFTR game server
After=network-online.target

[Service]
ExecStart=/opt/mftr/mftr-server --bind 0.0.0.0:7777 --replay /var/lib/mftr/replays
DynamicUser=yes
StateDirectory=mftr
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Then `systemctl enable --now mftr`.

## NixOS

The repository is a flake with a NixOS module. In the flake that holds your machine's
configuration:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    mftr = {
      url = "github:ilioscio/mftr/release";   # the newest release (see Updating)
      inputs.nixpkgs.follows = "nixpkgs";   # build with your nixpkgs (needs Rust 1.88+)
    };
  };

  outputs = { nixpkgs, mftr, ... }: {
    nixosConfigurations.myserver = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        mftr.nixosModules.default
        {
          services.mftr.enable = true;        # every mode on UDP 7777, as in Docker
          services.mftr.openFirewall = true;
        }
      ];
    };
  };
}
```

`nixos-rebuild switch` builds the server from source and starts `mftr.service`. To deploy
the newest release, update the input and rebuild; the service restarts on the new build:

```sh
nix flake update mftr && sudo nixos-rebuild switch --flake .#myserver
```

To test a branch, point the input at it (`url = "github:ilioscio/mftr/my-branch";`), or at a
local checkout with `--override-input mftr path:/home/me/mftr`. Players can only join a server
on their own protocol, so a server on `main` turns away everyone on the latest release whenever
main has a wire change the release doesn't.

A fuller setup: the main server with the modes you want, plus the Duel Sandbox and the dodge
rig as servers of their own.

```nix
services.mftr = {
  enable = true;
  openFirewall = true;                   # the main port, and sandboxes' unless they say otherwise
  port = 7777;
  modes = [ "aram" "mayhem" "classic" "hyper-coop" ];   # "-versus" / "-coop": one side only
  maps = [ ./maps/twin-rivers.mftrmap ]; # Classic on a map from the map editor, too
  maxMatches = 32;
  replay = true;                         # every match → /var/lib/mftr/server/replays
  sandboxes = {
    duel = { port = 7780; scenario = "duel"; maxPlayers = 4; };
    dodge = { port = 7781; scenario = "dodge"; };   # the blind playtest's dodge rig
  };
};
```

| Option | Default | Meaning |
|---|---|---|
| `address`, `port` | `0.0.0.0`, `7777` | Where the main server listens (`::` for IPv6 as well). |
| `openFirewall` | `false` | Open the main server's UDP port (and sandboxes', by default). |
| `modes` | `[ "aram" "mayhem" "hyper" "classic" "horde" ]` | As `--modes`: the existing modes offer Versus and Co-op, or one side with a suffix. `horde`/`horde-coop` offers five blue champions defending against minion waves; `horde-versus` is invalid. Other matches have 10 seats; all use champion select and bots in the empty seats. |
| `maps` | `[ ]` | Map files from the map editor (`*.mftrmap`): each adds its modes on that map (Classic on a Classic map; ARAM, Mayhem, Hyper and Horde on an ARAM map). |
| `maxMatches`, `workers` | `32`, one per core | As `--max-matches` and `--workers`. |
| `botTier` | `"veteran"` | As `--bot-tier`. |
| `seed` | `1` | As `--seed`. |
| `replay` | `false` | Record every match into `/var/lib/mftr/server/replays`. |
| `adoptKeyFrom` | `null` | The name of an older server whose key the main server takes on its first start (see below). |
| `keyFile` | `/var/lib/mftr/server/server.key` | The main server's key (see [The server key](#the-server-key)). |
| `extraArgs` | `[ ]` | More `mftr-server` options. |
| `sandboxes.<name>` | `{ }` | A server running one scenario: `port`, `scenario` (`duel`, `dodge`, `minions`, `empty`, or a full-match one as a single game), `maxPlayers` (`10`), `bots` (`0`), `lobby`, `seed`, `replay` (into `/var/lib/mftr/<name>/replays`), `address`, `openFirewall`, `extraArgs`, `enable`. One service `mftr-<name>` each. |

Every server runs as an unprivileged, sandboxed user and keeps its key (and replays) in
`/var/lib/mftr/<name>`, the main one in `/var/lib/mftr/server`; these survive rebuilds and
upgrades. The fingerprints to give players:

```sh
journalctl -u mftr | grep fingerprint          # the main server; mftr-duel for a sandbox
# or: sudo mftr-server --key /var/lib/mftr/server/server.key --fingerprint
```

`mftr-tools` is installed too, e.g. `sudo mftr-tools replay /var/lib/mftr/server/replays/<file>`,
or a bot to check a server from the machine:
`mftr-tools bot --server '127.0.0.1:7777#<fingerprint>' --queue classic-coop`.
Without NixOS, `nix build github:ilioscio/mftr` gives `result/bin/mftr-server`.

**From `services.mftr.servers` (one service per mode, before 0.11):** that option is gone, and
evaluating a configuration that sets it says so. The old servers kept their keys in
`/var/lib/mftr/<name>`, and the new layout leaves those alone:
- Full-match servers become `modes` on the main server. Players pinned the key of the one on
  the main port: set `adoptKeyFrom` to that server's name, and the main server copies its key
  on its first start (the log says "adopted the server key"). Check the fingerprint after the
  switch.
- Sandboxes become `sandboxes.<name>`. Give each the old server's name and it keeps its key.

For example, this old configuration

```nix
services.mftr.servers = {
  aram = { port = 7778; replay = true; };
  mayhem = { port = 7777; scenario = "mayhem"; replay = true; };
  classic = { port = 7779; scenario = "classic"; replay = true; };
  duel = { port = 7780; scenario = "duel"; maxPlayers = 4; };
  dodge = { port = 7781; scenario = "dodge"; };
};
```

becomes

```nix
services.mftr = {
  port = 7777;                       # players who used :7778 or :7779 switch to :7777
  adoptKeyFrom = "mayhem";           # the key players pinned on :7777
  modes = [ "aram" "mayhem" "classic" ];
  replay = true;
  sandboxes = {
    duel = { port = 7780; scenario = "duel"; maxPlayers = 4; };   # keeps duel's key
    dodge = { port = 7781; scenario = "dodge"; };                 # keeps dodge's key
  };
};
```

0.11.0's module kept the main server in `/var/lib/mftr` itself; a server set up with it takes
that key over the same way, without `adoptKeyFrom`.

## Options

| Option | Default | Meaning |
|---|---|---|
| `--bind ADDR` | `0.0.0.0:7777` | Address and UDP port to listen on. |
| `--key FILE` | `server.key` | The server's key, created on first start (see below). |
| `--fingerprint` | | Print the key's fingerprint and exit. |
| `--modes LIST` | `aram,mayhem,hyper,classic,horde` | What players can pick: the existing modes offer Versus and Co-op, or one side with a suffix. `horde`/`horde-coop`: [Mayhem: Horde](horde.md), five champions defending against 20 waves, then endless survival, co-op only. `aram`: The Bridge. `mayhem`: ARAM with augment drafts. `hyper`: Mayhem with faster basic abilities and attacks. `classic`: Classic 5v5 on Crossroads. |
| `--map FILE` | none | Add a map from the map editor (`*.mftrmap`): Classic on it if it's a Classic map, the ARAM modes if it's an ARAM map; repeat for more maps. The server checks each at start (and refuses a map that breaks a rule, listing what's wrong); players download it as they join (a few hundred milliseconds), and replays carry it. |
| `--max-matches N` | `32` | Matches running at once, at most. Past it, a player who'd need a new match is told the server is busy, and their client keeps asking. |
| `--workers N` | one per core | Threads that step the matches. |
| `--seed N` | `1` | Seeds everything random (each match draws its own seed from it). |
| `--bot-tier T` | `veteran` | How well bots play: `recruit`, `veteran`, `elite` or `nightmare` (see below). |
| `--replay DIR` | off | Record every match into DIR, as `<started>-<queue>-<match>.replay`. A file is rewritten every minute, when its match ends and when it stops. |
| `--scenario NAME` | | For development: offer this one scenario instead, any of the above or `duel` (the Duel Sandbox), `minions`, `dodge`, `empty` (test grounds). With it, `--bots N` adds server bots (they count toward `--max-players N`, default 10: leave a seat for each player) and `--lobby` holds champion select first. A spectating client can watch a match of bots alone. |

**How matches fill.** A player who picks a queue joins the match of it with the most players
that still has a bot's seat to take, or starts a new match. Versus puts them on the team with
fewer humans; Co-op puts every human on blue, against bots. Bots take every seat no human has.
Joining a match in progress, the player chooses a champion (ARAM: dealt, one reroll; Classic:
picked) and takes over a bot: in the fountain, at its level, with its gold plus a refund of
its items. In Co-op champion select, players vote how well the bots play (Recruit, Veteran,
Elite or Nightmare; the most votes wins, a tie goes to the easier, `--bot-tier` is where it
starts); the tier changes only how the bots play, never their gold or stats. Versus fill-ins
play at `--bot-tier`. When a match ends, its players go to champion select together for the next one. A match
with no players left stops (one whose players all lost their connection waits 60 s for them
first). A player who loses their connection keeps their seat for the rest of the match while a
bot plays their champion, and gets it back when their client reconnects, even from a new
address. A player who leaves leaves their champion to a bot.

## The server key

On first start the server creates its key (`--key`, default `server.key` in the working
directory; `/data/server.key` in Docker, `/var/lib/mftr/server/server.key` on NixOS) and prints its
fingerprint:

```text
key fingerprint 8a0bb4a2aa55a37cf673e459749c6705: players can pin it with HOST:7777#8a0bb4a2aa55a37cf673e459749c6705
```

A client remembers the key of every server it meets and refuses one whose key changed, so
**keep the key file** across restarts and upgrades (back it up with the server). Give players the
address with the fingerprint, `your.server.address:7777#8a0b…`, and their client only talks to
the real server, even on the first connection. Without the `#…` part they trust whatever key they
see first. If you ever replace the key, the client tells players the key changed, shows both
fingerprints and warns that this can also mean an impostor; they can **trust the new key** with
a button and connect, or connect once with the new `#fingerprint`. Tell them it's coming.

## Replays

With `--replay DIR`, every match is recorded in its own file. A replay holds every command of
the match plus a state hash every 10 seconds, about 5 MB per hour for a busy 5v5.
`mftr-tools replay FILE` re-simulates it and checks every hash. Re-simulating is deterministic
and exact across Linux, Windows and macOS, so a replay is also a good bug report. In Docker,
copy them out with `docker compose -f deploy/compose.yaml cp mftr:/data/replays .`.

## What it costs

Measured with `mftr-tools loadtest` and a real server ([13 §8](design/13-server-instances.md#8-measurements)):

| | Per match (10 players) | 32 matches on one machine |
|---|---|---|
| CPU | about 1.6% of one core for ARAM, about 4% for Classic (with sockets and encryption) | ARAM: about half a core; spread over every core (`--workers`) |
| Upload | about 85 KB/s (8.5 KB/s per player) | about 22 Mbit/s |
| Memory | a few MB (more with `--replay`, about 5 MB per hour) | |

A 1-vCPU VPS holds a few dozen ARAM matches, or about 20 Classic ones; bandwidth usually runs
out first. They all share the one process and port; `--max-matches` caps them, and
`--workers N` sets how many threads step them (default: one per core).

## Troubleshooting

- **The client stays on "connecting":** the UDP port isn't reachable. Check the firewall and
  your provider's security group. A client that reaches the server but can't connect says why on
  its connection line: another protocol version (the server log prints its protocol number), or a
  server key that differs from the pinned one.
- **"Every match of this kind is full":** the server runs `--max-matches` matches already. The
  client keeps asking and joins once a match stops; raise the limit if the machine has room
  (the log's `tick` figure is the time all matches take per tick, against the 33 ms a tick
  has: keep the max well under it).
- **Bots don't move in champion select:** they wait for the match to start. Someone has to join
  and ready up, or wait out the 60 s countdown once a human has joined.
- **No replay files:** a match's file is written once a minute; in Docker, the directory must
  be inside the `/data` volume.

## Updating

Follow the `release` branch, not `main`. The release workflow moves `release` to a version's tag
once that release is published, with its client downloads and the manifest clients check for
updates. A server updated from it never runs a version players can't download yet: NixOS hosts
with the input above get it from `nix flake update mftr`, Docker hosts from `git pull` in a
checkout of `release`.

Players see a version mismatch plainly. A client on another protocol is turned away, and from
0.17 on the server tells it which release it runs: the player reads *"This server runs MFTR
0.17.0, and you have 0.16.1"* with a button to get it (the download on Windows and macOS, the
release page on Linux). Clients also check for a newer release when they start and say so in
the start menu. `mftr-server --version` prints the server's release and protocol; the startup
log has them too.
