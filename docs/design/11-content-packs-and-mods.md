# 11 — Content Packs & Mods

Community champions (and later skins, maps and modes) reach players as **content packs that the server pushes on join** (D51). Joining a community server must be as easy as joining an official one, and **must never put a player's computer at risk**, even if the server or the pack author is malicious.

## 1. Why this is designed defensively

In July 2026, *Meccha Chameleon* (UE5) had a custom map pass its Workshop review while carrying hidden Blueprint logic. On load it ran a shell command that fetched a remote-access trojan, and the campaign ended with the game's Discord server hijacked ([Notebookcheck](https://www.notebookcheck.net/Meccha-Chameleon-Workshop-malware-led-to-Discord-server-hijack-RAT-infections.1355649.0.html), [Digital Trends](https://www.digitaltrends.com/gaming/meccha-chameleon-steam-workshop-malware-custom-map/)). The root cause: **mod content could contain executable logic with the game's full privileges**, and a human review was the only gate.

Godot has the same trap. Its native resources (`.tscn`, `.tres`, `.res`, `.scn`) can embed scripts that run when loaded, and `.gdshader` code runs on the GPU driver. Loading an untrusted Godot resource is equivalent to running untrusted code.

## 2. Threat model

| Threat | Example | Defense |
|---|---|---|
| Malicious pack author | A pack that tries to run code on clients | **Packs contain no executable content** (§3); strict parsers (§4) |
| **Malicious server** | A server that pushes crafted bytes to everyone who joins | The client validates everything itself and **trusts nothing the server says** about a pack |
| Parser exploits | Crafted glTF, audio or SVG that triggers a memory bug | Memory-safe Rust parsers only, a minimal format subset, hard size caps (§4) |
| Resource exhaustion | A decompression bomb, 10M triangles, a 4 GB audio file | Caps checked *before* allocation; total cache quota (§6) |
| Filesystem abuse | Path traversal ("zip-slip"), overwriting files | Pack bytes are never written by their own names; the cache is keyed by hash only (§6) |
| GPU abuse | Shaders that hang or crash the driver | **No pack shaders**: parameters of our shader kit only (§3) |
| Readability abuse | Invisible projectiles, hitbox-mismatched VFX, misleading team colors | The readability validator (§4) enforces 05 §1 |
| Gameplay abuse | A champion whose data hangs the sim or desyncs it | Data-only effect primitives with bounded counts (§3); behavior code only via the sandbox (§7) |
| Impersonation | A pack posing as first-party or as another author | Packs are signed by the author's key (§5); the UI shows the author identity |

## 3. What a pack may contain

**Data, never code.** A pack is a single archive in our own container format:

| Content | Format | Notes |
|---|---|---|
| Manifest | RON | Id, version, author key, signature, dependencies, declared sizes |
| Gameplay | RON | Champion definition from effect primitives ([04 §4](04-architecture.md#4-ability--effect-system), D8): stats, attack spec, abilities, action phases ([10 §4](10-characters-and-animation.md#4-action-timing-contract)) |
| Model and animations | glTF **binary subset**: meshes, vertex colors, one skin, animations | No images, no external URIs, no extensions, no cameras or lights |
| Markers | `.anims.ron` | [10 §4.5](10-characters-and-animation.md#45-markers) |
| Map props | glTF binary subset, kind `prop` | Static models on a one-bone `prop` rig (`root`, plus up to 8 `part_*` bones), no clips, the champion material slots (`accent` optional), ≤ 12,000 triangles: structures, trees, rocks, platforms |
| Materials | RON | Slot → parameters of the **built-in** champion shader ([10 §2](10-characters-and-animation.md#2-character-visual-style-faceted-ps1)) |
| VFX | RON | Instances of the built-in **VFX kit** (05 §5) with parameters; sizes come from the gameplay data |
| Audio | Ogg Vorbis + `.sfx.ron` | Mono or stereo, 8–48 kHz, ≤ 3 s per sound; bound to events by `<id>.sfx.ron` (§3.2) |
| Icons | SVG **subset** | Paths, solid fills and gradients; no scripts, images, fonts, external references or filters |
| Text | RON | Names, tooltips, per-language strings |

### 3.1 VFX files
`<id>.vfx.ron` sits next to `<id>.glb` and is authored by hand. It is optional: a pack without one plays the shared library's effects.

```ron
(effects: [
    (event: "q.release", kit: "flare", ramp: [(0.92, 1.0, 1.0), (0.55, 0.95, 0.9), (0.18, 0.6, 0.6)], count: 10, size: 1.4),
    (event: "q.projectile", kit: "arrow", ramp: [(0.9, 1.0, 1.0), (0.5, 0.95, 0.88)]),
    (event: "*.impact", kit: "burst", ramp: [(1.0, 1.0, 0.85), (1.0, 0.8, 0.4), (0.4, 0.22, 0.12)]),
])
```

- `event` is `<action>.<phase>`. The action is `attack`, `q`, `w`, `e`, `r`, `d`, `f` or `*`. The phase is `release`, `projectile`, `impact`, `detonate`, `start`, `land` or `fire` (the animation passing its `fire` marker, A6).
- `kit` must be one the phase allows ([05 §5.1](05-art-and-assets.md#51-the-kit-a4b)).
- `ramp` has 2–6 RGB colours in 0–1.
- The optional knobs are bounded:
  - `count`: 1–48;
  - `size`: a multiplier from 0.25 to 3, and never allowed on projectile kits;
  - `speed`: 0–12 m/s;
  - `lifetime`: 0.02–1.5 s.
- Caps: ≤ 4 effects per event, ≤ 64 per file, and ≤ 50 KB.

`mftr-pack` validates the file with the model. A broken VFX file fails the pack.

### 3.2 Sound files
`<id>.sfx.ron` sits next to `<id>.glb` and binds events to the Ogg Vorbis files in `sfx/`. It is optional: without it, a pack plays the shared library's sounds. First-party packs generate it from a recipe (05 §7.1); a community pack may write it by hand.

```ron
(sounds: [
    (name: "twang_1", events: ["attack.release"], volume: 0.5, pitch: 0.05),
    (name: "twang_2", events: ["attack.release"], volume: 0.5, pitch: 0.05),
    (name: "net_snare", events: ["r.impact"], volume: 0.65),
])
```

- `name` is the file `sfx/<name>.ogg`: 1–32 characters from `a–z`, `0–9` and `_`, unique within the pack.
- `events` are any of:
  - the VFX events (§3.1), with the extra phase `cast`;
  - `unit.<foot|death|respawn|recall|emote>`;
  - `cc.hard`;
  - `match.<victory|defeat>`.
- `volume` is 0–1.
- `pitch` is the random spread per play, 0–0.25.
- Up to 4 sounds may share an event, as variants.

Every file is decoded by lewton when the pack is validated. Before any samples are allocated, the validator checks the file size, channels, sample rate and length (read from the last Ogg page). The pack fails if `sfx/` holds an `.ogg` that no sound names, or any other file.

**Never accepted:** GDScript, C#, native libraries, Godot resources or scenes, shader source, HTML, any other file type, or anything that isn't listed above. First-party champions ship as packs too, so the official content proves the format is enough.

## 4. Validation

`mftr-pack` (Rust) is the **only** code that reads pack bytes, on the server, the client and in `mftr-tools`. The client builds Godot meshes, skeletons and animations **from validated arrays through gdext**. Pack bytes never reach Godot's `ResourceLoader`, and a CI test enforces that.

1. **Container:** total size ≤ 4 MB compressed *(start)*. Every entry's uncompressed size is declared and checked before decompressing, and the decompressed total is capped at ≤ 16 MB. Duplicate or unknown entries fail.
2. **Manifest and signature** (§5).
3. **Parsers:** glTF via a strict subset reader (`gltf`/`gltf-json`, buffers bounds-checked); Ogg via `lewton`; SVG via `usvg` with the subset enforced; RON via `serde` with no unknown fields. All are memory-safe Rust with no C decoders.
4. **Caps** ([10 §9](10-characters-and-animation.md#9-budgets)): triangles, bones, influences, clips, clip duration, audio duration, VFX particle counts and effect counts.
5. **Rig and animation rules** ([10 §8.4](10-characters-and-animation.md#84-validation-ci-and-add-on)): archetype, sockets, required clips and marker ↔ data consistency.
6. **Readability** ([05 §1](05-art-and-assets.md#1-readability-rules-non-negotiable)): VFX sizes come from the gameplay shape and can't be overridden; telegraphs use the built-in grammar; the team-accent region exists and is visible; enemy VFX can't drop below minimum opacity or brightness; hard-CC abilities carry the accent motif.
7. **Gameplay data:** schema-valid, values in sane bounds (no 0 ms cooldowns with 10,000 u ranges unless the server's mode explicitly allows them), and finite effect chains.

**The server validates before offering a pack, and every client validates again.** A failed client validation means the client refuses to join and shows why. It never "tries anyway".

## 5. Identity, integrity and trust

- **Content addressing:** a pack's id is the **BLAKE3 hash** of its canonical bytes. The server's Welcome lists `(name, hash, size)` for every pack in the match. The client downloads only the missing hashes and verifies each one.
- **Signatures:** the author signs the hash with their identity key (Ed25519, the identity of [07 §3](07-hosting-and-trust.md#3-identity)). The client shows the author and caches trust decisions per author key.
- **Player control:** a setting with **Ask (default) / Allow from trusted authors / Never**. The first time a server pushes a pack from a new author, the client asks, showing the pack name, author, size and hash. "Never" means you only join servers that run official content.
- **Official content** is signed by the project key and is always trusted. Community servers can run first-party-only modes with zero prompts.
- **Revocation:** the project can publish a signed blocklist of pack hashes and author keys, the same mechanism as the moderation blocklists of [07 §5](07-hosting-and-trust.md#5-moderation). Clients refuse blocked packs even if the server offers them.

## 6. Transfer and cache

- Packs travel on the existing encrypted connection (a reliable bulk channel, rate-limited so gameplay traffic never suffers) during champion select, with a progress bar. Total per match ≤ 40 MB *(start)*.
- **Cache:** `user://packs/<blake3>.pack`. Files are named by hash only, never by anything inside the pack, and reads are re-verified. LRU eviction with a 1 GB default quota.
- **Fallback:** the "Show default models" setting (05 §1 / Q6) renders community champions with a generic model of their archetype and silhouette class plus the built-in VFX. The gameplay is identical.

## 7. Scripted behaviors (later)

Data covers most champions. For mechanics that need code (Q5), the only acceptable form is **WebAssembly in a capability-free sandbox** (`wasmtime`):
- runs inside the sim, deterministically, on both the server and predicting clients;
- **no imports** except a narrow, versioned sim API (read unit state, emit effects); no filesystem, network, clock, threads or randomness except the sim's;
- fuel-metered per tick and memory-capped; on a trap the behavior is disabled for the match and the server logs it;
- a pack with WASM is labelled as such in the join prompt.

Until that ships, packs that need code aren't supported. We don't add a weaker interim scripting path.

## 8. Rollout

| Milestone | What |
|---|---|
| A1 (art pilots) | Pack format v0 and `mftr-pack` validator. Vesper and Rook ship as first-party packs, loaded from disk |
| M5 | Server push, signatures, trust prompt, cache, blocklist, fallback models |
| Post-M5 | WASM behaviors (§7), skins (readability-locked, 05 §1.5), maps and modes |
