# 04 — Architecture

## 1. Big picture

```
                         ┌──────────────────────────────┐
                         │        mftr-lobby (Rust)      │  parties, custom games,
                         │  HTTP/WebSocket + signed      │  champ select, connect tokens,
                         │  connect tokens               │  server list (opt-in)
                         └──────────────┬───────────────┘
                                        │ token
   ┌────────────────────────────────────┼─────────────────────────────────────┐
   │ Godot client                       │                                     │
   │  ┌────────────────────────┐   UDP  ▼          ┌──────────────────────┐   │
   │  │ GDScript / scenes      │◄──────────────────►│ mftr-server (Rust)   │   │
   │  │ rendering, VFX, UI,    │                    │  headless, no Godot  │   │
   │  │ audio, input, camera   │                    │  ┌────────────────┐  │   │
   │  └──────────▲─────────────┘                    │  │   mftr-sim     │  │   │
   │             │ render state / commands          │  │ authoritative  │  │   │
   │  ┌──────────┴─────────────┐                    │  └────────────────┘  │   │
   │  │ mftr-gdext (Rust)      │                    │  mftr-net, bots,     │   │
   │  │ mftr-net + mftr-sim    │                    │  replay recorder     │   │
   │  │ (prediction, interp,   │                    └──────────────────────┘   │
   │  │  projectile timeline)  │                                               │
   │  └────────────────────────┘                                               │
   └───────────────────────────────────────────────────────────────────────────┘
```

**Rule:** gameplay logic lives in `mftr-sim` and nowhere else. Godot never decides a gameplay outcome. It draws what the Rust side tells it and turns input into commands.

## 2. Repository layout (proposed)

```
mftr/
├─ crates/
│  ├─ mftr-sim/       # Deterministic simulation. No I/O, no networking, no Godot.
│  ├─ mftr-data/      # Content schema (champions, abilities, items, augments, maps, modes),
│  │                  # loading, validation (incl. reaction-budget checks), hot reload
│  ├─ mftr-nav/       # Navmesh generation from vector map data, pathfinding, LoS grid
│  ├─ mftr-net/       # Protocol, bit-packing, snapshot deltas, events, clock sync,
│  │                  # link conditioner, transport wrapper
│  ├─ mftr-client/    # Engine-independent client runtime: prediction, reconciliation,
│  │                  # margin loop, interpolation (used by gdext, bots and the Netcode Lab)
│  ├─ mftr-bots/      # Bot AI (uses the same command interface as players)
│  ├─ mftr-server/    # Dedicated match server binary
│  ├─ mftr-lobby/     # Lobby / custom games / token service binary
│  ├─ mftr-gdext/     # Godot GDExtension (godot-rust): MatchClient node & friends
│  └─ mftr-tools/     # CLI: headless matches, Netcode Lab, replay tools, balance sims,
│                     # content linter, asset size reporter
├─ client/            # Godot 4 project: scenes, shaders, UI, audio, input maps
├─ content/           # Game data (RON): champions/, items/, augments/, maps/, modes/
├─ maps-src/          # Vector map sources (lanes, walls, brush, heights) → baked meshes
└─ docs/
```

Cargo workspace for all crates. The Godot project loads `mftr-gdext` as a GDExtension built per platform in CI.

## 3. Simulation core (`mftr-sim`)

### Tick order (every 33.3 ms)
1. **Apply commands** that target this tick (validate; reject invalid).
2. **AI decisions:** minions, turrets, monsters, bots.
3. **Cast state machines:** windup → fire → channel → recovery; attack timers.
4. **Movement:** path following, dashes, displacement, unit collision and separation.
5. **Projectiles and areas:** advance, then run **swept** collision tests (capsule sweep per tick, so fast projectiles can't tunnel past a target).
6. **Effect resolution:** a queue of damage / heal / shield / CC events through the pipeline in [02](02-combat-math.md).
7. **Status ticking:** buffs, debuffs, DoTs, regen, cooldowns.
8. **Deaths and rewards:** gold, XP, bounties, respawns, structure destruction, objective events.
9. **Vision:** per-team visibility grid.
10. **Snapshot and event emission:** handed to `mftr-net` per client (server) or to the render bridge (client).

### Data model
- Entities stored in dense arenas or a lightweight ECS (`bevy_ecs` standalone or `hecs`). Decide in M0, keeping determinism rules in mind (stable iteration order).
- Stable **entity IDs** (generation + index) are used directly on the wire.
- Static content (champion definitions, ability specs) is immutable and shared. Live state refers to it by ID.

### Navigation (`mftr-nav`)
- Maps are authored as **vector data**: polygons for walls and brush, plus lane splines.
- They're baked to a **navmesh** for pathing (A* over polygons + funnel/string-pulling) and a **vision grid** for line of sight.
- Unit collision: circles with gameplay radius. Separation/avoidance between moving units (RVO-lite). Structures are static obstacles.
- Dash and blink wall-crossing checks run against the same navmesh, so client prediction and server agree.

## 4. Ability & effect system

The goal is that **most champions, all items and all augments are pure data**, composed from a library of primitives. This is what makes Mayhem-style augments ("duplicate this skillshot", "abilities can crit", "become huge") generic rather than hand-coded per champion.

### Building blocks
- **Targeting:** `Direction(range)`, `Point(range)`, `Unit(range, filter)`, `Self`, `Vector(range, length)`.
- **Delivery:** `LinearProjectile`, `HomingProjectile`, `ArcProjectile`, `DelayedArea`, `InstantArea(shape)`, `Dash`, `Blink`, `Zone(duration, tick)`, `Tether`, `Attach`.
- **Effects:** `Damage`, `Heal`, `Shield`, `ApplyStatus(cc | buff | debuff)`, `Displace`, `ModifyStat`, `Summon`, `Reset(cooldown)`, `GrantResource`, `Reveal`.
- **Triggers:** `OnCast`, `OnHit`, `OnKill`, `OnTakedown`, `OnDamaged`, `OnBasicAttack`, `OnRecast(window)`, `Periodic`, `OnStatusApplied` …
- **Conditions and scaling:** ratios over any stat (self or target), ability rank, level, stacks, target state (e.g. "if slowed").

### Example (RON)

```ron
Ability(
    id: "ember_lance",
    slot: Q,
    targeting: Direction(range: 1100),
    cast_time: 0.25,
    cooldown: [9.0, 8.0, 7.0, 6.0, 5.0],
    cost: Mana([50, 55, 60, 65, 70]),
    reaction_class: Burst,
    delivery: LinearProjectile(
        speed: 1600,
        width: 70,
        pierce: false,
        collides_with: [EnemyChampion, EnemyMinion, EnemyMonster],
    ),
    on_hit: [
        Damage(kind: Magic, base: [80, 125, 170, 215, 260], ratios: [(AP, 0.70)]),
        ApplyStatus(Slow(amount: 0.30, duration: 1.5)),
    ],
    visuals: "fx/ember_lance",   // VFX/SFX binding; hitbox width is pulled from `width`
)
```

### Modifier hooks (what augments and items act on)
- **Stat modifiers and conversions** (e.g. "convert bonus AD to AP at 110%").
- **Event listeners** (any trigger above).
- **Delivery transformers:** take any `delivery` and alter it, e.g. `Multishot(count: 3, spread_deg: 15)`, `Echo(delay: 0.75, power: 0.4)`, `Split on hit`, `Scale(width: 1.5)`.
- **Rule flags:** `AbilitiesCanCrit`, `UltimateDisabled`, `ForcedMelee`.
- **Physical scale:** a single `size` multiplier that changes the model scale, gameplay radius, and optionally range and speed.
- **Spell-slot replacement:** swap a utility spell for an ability definition.

### Escape hatch: behaviors
Mechanics that can't be expressed in data are written as **Rust behaviors**: small, registered, deterministic modules referenced by ID from data (`behavior: "tidecaller_passive"`). A later **embedded scripting** layer for community mods is an open question (candidates: Rhai, Lua via `mlua`, or WASM via `wasmtime` with fuel limits). Determinism and sandboxing are the hard constraints.

### Validation
`mftr-tools lint-content` runs in CI. It checks schema, references, reaction-budget classes, hitbox vs. VFX metadata, tooltip generation, and balance sanity bounds.

## 5. Client (Godot 4)

- **Godot 4.x latest stable**, with `godot-rust` (gdext) for the extension.
- **`MatchClient` node (Rust):** owns the connection, clock sync, prediction and interpolation. Each frame it exposes a **render state**: for each visible entity, its archetype ID, transform, animation state, health/resource, status icons and VFX triggers.
- **Godot side (GDScript):**
  - spawns a visual scene per archetype ID (pooled) and applies transforms and animation states,
  - renders ability indicators from the same content data (true range and width),
  - handles input mapping (rebinding, cast modes) and turns it into command calls on `MatchClient`,
  - provides UI (HUD, shop, scoreboard, minimap, settings) using Control nodes and SVG icons,
  - plays audio, and runs the camera (locked / free / edge pan).
- **Fixed-step sim, variable-rate rendering:** the render frame interpolates between sim ticks. 144 Hz monitors get smooth motion from a 30 Hz sim.
- **Shader warm-up:** compile and cache all gameplay shaders during loading, so the first appearance of a VFX never stutters.

## 6. Bots

- Bots run **server-side** in `mftr-bots` and issue the same commands as players.
- By default they only use their team's vision (no wallhacks). Difficulty comes from reaction delay, decision quality and mechanical accuracy.
- Uses: filling custom games, ARAM practice, tutorial, automated playtesting (thousands of headless bot matches for balance stats), and stress tests.

## 7. Build, CI & testing

- **CI:** `cargo test` (unit + golden math tests), content lint, headless bot matches (crash and desync detection), Netcode Lab dodge rig under fixed network profiles, cross-platform determinism hash check, gdext builds for all platforms, Godot export, and a **download size report**. The build fails if a budget is exceeded (see [05](05-art-and-assets.md)).
- **Benchmarks:** server tick time for a scripted late-game 5v5 fight. Regressions fail CI.
- **Fuzzing:** the packet parser and command validator.

## 8. Dependencies policy

- Prefer permissive or copyleft-compatible licenses (MIT, Apache-2.0, BSD, Zlib, MPL-2.0, (A)GPL). Verify with `cargo deny` in CI.
- No proprietary SDKs in the default build. Optional platform integrations (e.g. Steam) live behind feature flags.
