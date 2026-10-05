# 03b — Netcode: Wire Protocol

Wire-level spec behind [03 — Netcode](03-netcode.md) and [03a](03a-netcode-time-and-prediction.md). Field sizes are *(start)* values. The protocol is versioned, so they can change freely before 1.0.

## 1. Layers

```text
┌──────────────────────────────────────────────┐
│ Messages: commands, snapshot, events, time    │
├──────────────────────────────────────────────┤
│ Channels: redundant / latest-wins / reliable  │
├──────────────────────────────────────────────┤
│ Packet layer: sequence, ack, ack bitfield     │
├──────────────────────────────────────────────┤
│ Secure transport: netcode.io-style AEAD, UDP  │
└──────────────────────────────────────────────┘
```

- **Max packet:** 1200 bytes including headers. That's safe under the IPv6 minimum MTU of 1280, so we never rely on IP fragmentation. Typical gameplay packets are 80–500 bytes.
- **Send rate:** server → client once per tick (30 Hz). Client → server once per tick **plus** an immediate extra packet whenever a new command is issued.

## 2. Connection & versioning

1. The client asks the **lobby** to join a match. The lobby returns a **connect token**:
   - public part: protocol ID, expiry, server address list, session keys (client↔server, encrypted for the client over the lobby's TLS),
   - private part (sealed for the match server): player public key, match ID, team/slot, session keys, **build hash**, **content hash**.
2. Handshake: `ConnectionRequest(token) → Challenge → ChallengeResponse → Accepted(client_index, server_tick)`.
3. After the handshake, the server sends the initial full state over the bulk channel. Then normal snapshots begin.

**Exact-match requirement:** client and server must have the identical **build hash** (sim code) and **content hash** (game data). Prediction is bit-exact re-simulation, so "close enough" versions are not allowed. On a mismatch the client gets a clear "update required (server runs X)" error.

**Key model:**
- *v1 (self-host):* lobby and match servers belong to one operator and share a symmetric private-token key, the same as netcode.io.
- *Federation (later):* the lobby signs tokens with its Ed25519 key, and session keys are agreed by X25519. Trust in the lobby key is configured per server.

## 3. Packet layer

```text
packet       := prefix | sequence | ack | ack_bits | channel_blocks… | aead_tag
prefix       : u8      packet type (handshake kinds, payload, keep-alive, disconnect)
sequence     : u16 on the wire (expanded to u64 for the AEAD nonce / replay window)
ack          : u16     latest remote sequence received
ack_bits     : u32     receipt of the 32 sequences before `ack`
aead_tag     : 16 bytes
```

Overhead: ~25 bytes per packet. At 30 Hz that's ~0.75 KB/s per direction.

**Acks drive everything:** when packet N is acked, every message carried in N counts as delivered. That's how events get reliability and how snapshots get baselines, with no per-message ack traffic.

## 4. Channels

| Channel | Direction | Delivery | Contents |
|---|---|---|---|
| **Commands** | C → S | **Redundant:** each packet repeats all un-acked commands (cap 8) | Player commands |
| **Snapshot** | S → C | **Unreliable, latest-wins**, delta vs. the last acked snapshot | Entity state |
| **Events** | both | **Reliable ordered:** each event is repeated in every packet until a packet carrying it is acked | Gameplay events, chat, pings |
| **Time** | both | Unreliable, in every packet | Clock sync, arrival-lead reports |
| **Bulk** | both | Reliable, fragmented, lowest priority | Initial state, scoreboard, large UI data |

**Packing priority per packet:** Time → Commands / Events → Snapshot (fills the remaining budget) → Bulk.

## 5. Commands (client → server)

```text
command := cmd_seq: u16 | tick_delta: i8 (vs. the packet's base tick) | sub: u6 | kind: u5 | payload
```

| Kind | Payload | ≈ bits |
|---|---|---|
| `MoveTo` | point | 67 |
| `AttackUnit` | entity id (varint, typically 12 bits) | 47 |
| `AttackMove` | point | 67 |
| `Stop` / `Hold` | — | 35 |
| `Cast` | slot u3 + target: point / entity / vector (2 points) / none | 40–105 |
| `UtilitySpell` | slot u1 + target | 40–100 |
| `UseItem` | inventory slot u3 + target | 40–105 |
| `LevelUp` | slot u2 | 37 |
| `Buy` / `Sell` / `Undo` | item id u12 | 47 |
| `Recall` | — | 35 |
| `MapPing` | kind u4 + point | 71 |

- **Point:** x, y quantized to **0.25 u**, 16 bits each. That covers 16,384 u per axis, and the main map is ~15,000 u.
- A full command packet with 3 redundant commands is ~60 bytes. Even at 10 clicks per second that's ~1–2 KB/s up.
- Chat goes over the events channel, not commands.

## 6. Snapshots (server → client)

```text
snapshot      := tick: varint | baseline: u8 (ticks back, 0 = none) | own_state | entity_update… | removed…
own_state     := lossless authoritative own-champion state (03a §4.1)
entity_update := id_delta: varint | group_mask: u8 | groups…
```

### Lossless vs. quantized
- **Own champion: lossless** (raw f32 positions, exact path cursor, cast state, cooldowns, statuses). The client compares its predicted state bit-for-bit (03a §4). Quantizing would cause a constant drip of tiny "corrections". Cost: ~40–60 bytes per tick, the largest single item, and worth it.
- **Everyone else: quantized.** They're interpolated or used as approximate collision proxies anyway.

### Field groups (others)

| Group | Fields | ≈ bits when changed |
|---|---|---|
| Transform | x, y (16 + 16 or a short delta), facing u9 | 20–41 |
| Movement intent | speed u10, destination point, up to 4 waypoints (delta-coded) | 40–150 |
| Vitals | HP u16 (scaled to max), resource u12, shield total u12 | 16–40 |
| Statuses | visible status bitset + remaining time u8 each | variable |
| Action | cast/attack slot, phase, phase start tick delta | 16–24 |
| Appearance | size scale, team tint, augment indicators | rare |

### Path-based movement saves most of the bandwidth
Units following a path don't need a position every tick. The client advances them along the replicated path. The server sends **Movement intent when the path changes**, plus a **Transform refresh** only every ~10 ticks or when error exceeds 2 u (e.g. after a collision slide). Lane minions walking in a line then cost close to nothing.

### Priority accumulator
Each `(client, entity)` pair accumulates priority every tick. The snapshot is filled highest-first until the packet budget runs out, and sent entities reset to 0.

| Entity | Priority per tick *(start)* |
|---|---|
| Entities in the own champion's collision bubble (≤ 600 u) | 10 |
| Champions in view | 8 |
| Minions/monsters in view | 3 |
| Visible entities off-camera | 1 |
| Structures | 0.5 (plus immediately on change) |

Dirty groups that miss one packet just accumulate and go next tick. Nothing is ever lost, only delayed.

### Baselines
- The server keeps the last 32 snapshots per client (~1 s). A delta encodes against the newest snapshot the client has acked.
- If the client's last ack is older than the ring (heavy loss), the server sends a keyframe for the affected entities.
- An entity entering vision is always sent in full on its first snapshot.

## 7. Events

```text
event := event_seq: varint | tick_delta: i8 | sub: u6 | kind: u7 | payload
```

| Event | Key payload | Notes |
|---|---|---|
| `CastStart` | caster, slot, target, windup | Remote windups are drawn on T_input (03a §7) |
| `CastCancel` | caster, reason | |
| `ProjectileSpawn` | **lossless** origin & direction, speed, width, range, owner, ability ref, `count` + spread for batches | One event can carry a Multishot volley |
| `ProjectileEnd` | projectile, reason (hit / expired / blocked), hit entity | Confirms or denies predicted interceptions |
| `AreaSpawn` | shape, position, detonation time | Ground telegraphs |
| `Damage` | source, target, amount, type, flags | Drives health changes, numbers and the combat log |
| `Heal` / `Shield` | source, target, amount | |
| `StatusApplied` / `StatusRemoved` | target, status, duration, source | Start tick matters for re-sim |
| `Displacement` | target, curve kind, start, end, duration | Hooks, knock-ups, dashes |
| `Death` / `Respawn` | unit, killer, assists, bounty | |
| `Reward` | gold/XP deltas | Own champion only |
| `LevelUp`, `ItemChange` | | |
| `StructureDestroyed`, `Objective` | | Global announcements |
| `Chat`, `MapPing`, `Announce` | | Team-filtered |

- **Fog filtering:** an event goes to a team only if it's visible to that team at its tick (03 §10), or it's a global announcement. A projectile entering vision gets a synthetic, re-based `ProjectileSpawn`.
- **Delivery:** receivers dedupe by `event_seq` and apply in order. Events are applied on the timeline appropriate to their kind, using their tick and sub-tick.
- **Volume:** a 5v5 teamfight produces roughly 30–80 events per second per client, about 0.5–1.5 KB/s.

## 8. Time messages

| Direction | Fields |
|---|---|
| C → S | `client_time`: u32 µs, truncated |
| S → C | echo of `client_time`, `server_hold`: u16 µs (receive → send), `server_tick` + `sub` at send |
| S → C | `arrival_leads`: for recently received commands, `(cmd_seq, lead)` in 0.5 ms units, i16 |

These feed the clock filter (03a §10.1) and the margin controller (03a §10.2).

## 9. Bandwidth estimate (late-game teamfight, per client)

| Item | Bytes per tick | KB/s |
|---|---|---|
| Packet overhead | 25 | 0.75 |
| Own champion (lossless) | 50 | 1.5 |
| 9 other champions (transform/intent + vitals + action) | 9 × 12 ≈ 110 | 3.3 |
| ~30 minions/monsters (mostly path-coasting; priority-limited) | ~60 | 1.8 |
| Structures, wards, misc. | ~10 | 0.3 |
| Events (≈ 60/s × ~14 B) | ~28 | 0.85 |
| Time | 12 | 0.35 |
| **Total** | **~295** | **~9 KB/s** |

That's well under the 32 KB/s target. The headroom covers Mayhem chaos (Hyper mode + Multishot) and spectator streams. The server sends ~90 KB/s (~0.75 Mbit/s) for a full match. Measure, don't trust this table: the Netcode Lab records real numbers per scenario.

## 10. Server bookkeeping (per client)

- Snapshot ring (32) for delta baselines, with a per-entity "last acked state" view.
- Un-acked event queue, with a per-event list of packets it rode in.
- Command queue keyed by `(tick, sub, cmd_seq)`, with dedupe of redundant copies.
- Priority accumulators and team visibility set.
- Arrival-lead stats for reporting.

## 11. Robustness & security

- **AEAD on every packet** (ChaCha20-Poly1305), with a replay window on the sequence number.
- **Rate limits:** packets per second, commands per second, events per second (chat, pings). Excess is dropped and counted; sustained abuse disconnects.
- **Strict decoding:** bounded lengths, no allocations sized from untrusted input, unknown kinds rejected.
- **Fuzz targets** in CI: packet decode, command decode, event decode, snapshot apply.
- All gameplay validation happens in the sim (03 §5). The protocol layer only guarantees well-formedness.

## 12. Rust sketch

```rust
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Tick(pub u32);

/// Position inside a tick interval, in 1/64 steps.
#[derive(Clone, Copy)]
pub struct SubTick(pub u8);

/// 0.25-unit quantized map point.
#[derive(Clone, Copy)]
pub struct QPoint { pub x: u16, pub y: u16 }

pub struct Command {
    pub seq: u16,
    pub tick: Tick,
    pub sub: SubTick,
    pub kind: CommandKind,
}

pub enum CommandKind {
    MoveTo(QPoint),
    AttackUnit(EntityId),
    AttackMove(QPoint),
    Stop,
    Cast { slot: AbilitySlot, target: CastTarget },
    UtilitySpell { slot: u8, target: CastTarget },
    UseItem { slot: u8, target: CastTarget },
    LevelUp(AbilitySlot),
    Buy(ItemId), Sell(ItemId), Undo,
    Recall,
    MapPing { kind: PingKind, at: QPoint },
}

pub enum CastTarget { None, Point(QPoint), Unit(EntityId), Vector(QPoint, QPoint) }
```

Encoding is hand-written bit-packing (a `BitWriter`/`BitReader` pair). Every message type has round-trip property tests and a fuzz target.
