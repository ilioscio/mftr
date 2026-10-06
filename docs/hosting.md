# Hosting an MFTR server

An MFTR server is one small program (under 1 MB) that runs one game. It talks UDP, keeps the whole
match authoritative, and needs no database, accounts or other services. This guide gets a match
running on a fresh Linux machine (a VPS or a spare PC).

> **Status:** M2 (ARAM on The Bridge, six champions, bots). The protocol isn't encrypted yet
> (decision Q3 is still open), so don't host anything you'd mind strangers watching.

## Quick start with Docker

```sh
git clone https://github.com/ilioscio/mftr.git && cd mftr
docker compose -f deploy/compose.yaml up -d
```

This builds the server image and starts two games:

| Service | Port | What it runs |
|---|---|---|
| `aram` | UDP 7777 | ARAM on The Bridge with champion select; bots fill empty slots, so you can start with friends or alone. Records a replay of the session. |
| `duel` | UDP 7778 | The Duel Sandbox for up to 4 players: practice skillshots. |

Players connect with the client: `./mftr.x86_64 -- your.server.address:7777` (`mftr.exe` on
Windows; `godot --path client -- your.server.address:7777` from the source). Open the ports in your firewall, e.g.
`ufw allow 7777:7778/udp`.

Without compose: `docker build -t mftr-server . && docker run -d -p 7777:7777/udp mftr-server`.
Anything after the image name replaces the default options (see below), for example
`docker run -d -p 7777:7777/udp mftr-server --scenario aram --bots 4`.

## Without Docker

Download `mftr-server-<version>-linux-x86_64.tar.gz` from the releases page (Windows and macOS
builds are there too), or build it yourself with Rust 1.85 or newer:

```sh
cargo build --profile dist -p mftr-server -p mftr-tools   # → target/dist/
./target/dist/mftr-server --scenario aram --bots 10 --lobby
```

To keep it running, a systemd unit (`/etc/systemd/system/mftr.service`):

```ini
[Unit]
Description=MFTR game server
After=network-online.target

[Service]
ExecStart=/opt/mftr/mftr-server --bind 0.0.0.0:7777 --scenario aram --bots 10 --lobby --replay /var/lib/mftr/session.replay
DynamicUser=yes
StateDirectory=mftr
Restart=on-failure

[Install]
WantedBy=multi-user.target
```

Then `systemctl enable --now mftr`.

## Options

| Option | Default | Meaning |
|---|---|---|
| `--bind ADDR` | `0.0.0.0:7777` | Address and UDP port to listen on. |
| `--scenario NAME` | `duel` (`aram` in Docker) | `aram`: The Bridge, a full match with a winner. `duel`: the Duel Sandbox. `minions`, `dodge`, `empty`: test grounds. |
| `--bots N` | `0` (`10` in Docker) | Server bots. They count toward the player limit, and with `--lobby` a joining human takes a bot's place. |
| `--lobby` | off (on in Docker) | Champion select before the match: ARAM all-random, 2 rerolls each, a team bench. Starts 3 s after everyone is ready, or after 60 s. |
| `--max-players N` | `10` | Players (humans and bots) per game. Up to 8 spectators come on top. |
| `--seed N` | `1` | Seeds everything random in the match (bots, champion select, spawns). |
| `--replay FILE` | off | Record the session. The file is rewritten every minute and at each match end. |

A match restarts 10 s after a Base falls. Players who lose their connection keep their champion
for 60 s and get it back when their client reconnects, even from a new address.

## Replays

A replay holds every command of the session plus a state hash every 10 seconds, about 5 MB
per hour for a busy 5v5. `mftr-tools replay FILE` re-simulates it and checks every hash.
Re-simulating is deterministic and exact across Linux, Windows and macOS, so a replay is also a
good bug report. In Docker, copy it out with
`docker compose -f deploy/compose.yaml cp aram:/data/session.replay .`.

## What it costs

Measured in the Netcode Lab (one ARAM match, 10 players at 80 ms):

| | Per match | 10 matches on one machine |
|---|---|---|
| CPU | 0.18 ms per 33 ms tick (about 0.5% of one core) | about 5% of one core |
| Upload | about 105 KB/s (10.5 KB/s per player) | about 8.5 Mbit/s |
| Memory | a few MB (more with `--replay`, about 5 MB per hour) | |

A 1-vCPU VPS holds several matches; a 4-core one holds ten with plenty of room. Each match is
its own process and port, so run more with more ports (`--bind 0.0.0.0:7779` and so on).

## Troubleshooting

- **The client stays on "connecting":** the UDP port isn't reachable. Check the firewall and
  your provider's security group, and that the client and server versions match (the server log
  prints its protocol number, and a mismatched client is rejected).
- **Bots don't move in champion select:** they wait for the match to start. Someone has to join
  and ready up, or wait out the 60 s countdown once a human has joined.
- **The replay file is empty or missing:** the server writes it once a minute; in Docker, the
  path must be inside the `/data` volume.
