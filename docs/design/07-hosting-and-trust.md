# 07 — Hosting, Identity & Trust

## 1. Principles

- **Anyone can host.** A friend group, a community, a LAN party or a tournament organizer can run the full stack with no permission from us.
- **No mandatory central service.** The project may run a reference lobby and public server list, but the game works fully without them.
- **Your identity is yours:** a keypair, not an account in our database.
- **Privacy by default:** no telemetry without opt-in, no PII required to play.

## 2. Components

| Component | What it does | Runs where |
|---|---|---|
| `mftr-server` | Runs matches (one process can host many matches). UDP. | Any Linux/Windows/macOS box, VPS, or home PC |
| `mftr-lobby` | Parties, custom game rooms, champion select / draft, issues connect tokens, assigns matches to servers, stores match history | Same box or separate |
| Server list (optional) | Opt-in public directory of lobbies/communities | Project-run instance + self-hostable |

### Self-host story (v1 target)
```bash
# single box, everything in one command
docker compose up -d        # lobby + 1 match server, ports 7777/udp, 8080/tcp
```
Or the bare binaries plus a `mftr.toml` config. The config sets: region label, max concurrent matches, allowed modes, enabled champions, spectator delay, bot settings, admin keys, moderation policy, and whether to announce to public lists.

### Resource targets
- One 5v5 match: < 3 ms per tick on one core (≈ 10% of a core), < 100 MB RAM, ~2 Mbit/s up.
- A 4-core VPS should host **20+ concurrent matches** at 30 Hz.

## 3. Identity

- On first launch the client generates a **keypair**. The public key is your identity. A display name and profile are signed by that key. *(Implemented, D40: an X25519 key in the client's user folder, `identity.key`, used as the Noise static key, so every server learns which key connected. Signing for profiles and results comes with federation.)*
- Optional **recovery and multi-device:** export or import the key, or encrypt it with a passphrase for backup.
- Lobbies may require **account binding** (e.g. email or invite) for ranked, as their own policy.
- Match results are **signed by the server's key**. This is the foundation for federation.

## 4. Anti-cheat stance

**No kernel anti-cheat. Ever.** Instead:

| Threat | Mitigation |
|---|---|
| Speed, cooldown, damage or teleport hacks | **Impossible:** the server is authoritative and validates every command |
| Map hacks / ESP | **Impossible by construction:** server-side fog culling, so hidden data never reaches the client ([03](03-netcode.md#10-fog-of-war-culling)) |
| Packet spoofing / session hijack | Encrypted, authenticated transport with signed connect tokens |
| Input spam / bot farming | Server rate limits, lobby-level policies, proof-of-work or invite gating for ranked as a server option |
| **Scripting** (auto-dodge, perfect orb-walk, auto-combos) | **Not fully preventable in any open client, and only partly in closed ones.** Mitigated with: server-side statistical detection (reaction-time distributions on dodges relative to when the projectile became visible, inhuman input regularity); replay review tools; a report → replay → moderator workflow; and per-server trust and bans |
| Ghosting via spectator | Spectator delay for ranked/tournament |

Design choice: where a "script advantage" is cheap to give everyone, **build it into the game** as a fair accessibility feature (e.g. a last-hit indicator, attack-move-click, ability range rings). This shrinks the space where scripts help.

## 5. Moderation

- Moderation is **per server/community**: ban lists by public key, chat filters (configurable), report queue with replay attachments.
- Shareable **blocklists** (fediverse-style). Communities can subscribe to each other's lists.
- Admin tools: kick, mute, ban, pause, remake, spectate, and download a replay.

## 6. Rating & matchmaking (M5)

- **OpenSkill (Weng-Lin)** rating: open, team-aware and party-aware. Ratings are per lobby instance.
- Matchmaking optimizes rating balance, role preferences (Draft), party size limits and **ping** (region-aware: pick the server that minimizes the worst player's ping).

## 7. Federation (post-M5 sketch)

Goal: many independent instances that can, by mutual agreement, share players, matchmaking and ladders.

- **Instance identity:** each lobby has a keypair and a public metadata document.
- **Trust relationships:** instance A trusts B's signed match results for rating. Trust is explicit and revocable.
- **Cross-instance play:** a player's key works everywhere. Instances exchange match offers so queues can merge across trusted peers, with the server chosen by lowest max ping.
- **Federated ladders:** ratings computed from the union of signed match records among trusted instances.
- **Protocol:** custom, documented, versioned (HTTP + signed JSON/CBOR). Evaluate ActivityPub only for social features (profiles, follow, community announcements), not for matchmaking.

Nothing in M0–M5 may make this impossible. In particular: signed match records, stable public-key identity, and versioned protocols from day one.

## 8. Telemetry & privacy

- Off by default. Opt-in crash reports and anonymized performance and netcode stats (helps tune [03](03-netcode.md) targets).
- Server operators see IPs, as any server does. The privacy policy template in the repo explains this to players.
