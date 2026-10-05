# 00 — Vision & Pillars

## Pitch

A 5v5 lane MOBA that feels like the genre's best in your hands. Skillshots are readable and dodgeable, combat is tight and responsive, and teamfights are high-stakes. Everything else is open: the code, the servers, the data formats and the art pipeline. Anyone can host it, mod it, and play it on Linux, with no kernel anti-cheat and no cosmetic store.

## Why this exists

Of the major MOBAs, only one has really nailed the "visible skillshot you can react to" realtime feel. That game requires a kernel-level anti-cheat (which shuts out Linux), and it monetizes heavily to pay for a global server fleet. We can't out-infrastructure it. We can make a game that:

1. puts its engineering effort into **netcode quality per match**, not data centers,
2. lets **communities run their own servers** close to their players, and
3. stays **small, open and respectful** of the player's machine, privacy and wallet.

## Pillars

Every feature is judged against these, in priority order.

### 1. Dodgeable by design
- **What you see is what the server decides.** The netcode renders threats on the same timeline your inputs act on (see [03](03-netcode.md)).
- **Hitbox honesty.** A skillshot's visual extent *is* its hitbox. No hidden padding, no VFX that hides the real edge.
- **Every dodge-intended ability telegraphs.** Windups, travel time and ground indicators follow a consistent visual language, with a minimum "reaction budget".
- **Responsive controls.** Your own champion is predicted locally, so clicks and casts respond on the next frame.

### 2. Yours to run
- One server binary, one config file, and a Docker image. A gaming PC can host a match on a home connection.
- No mandatory central account service. Identity is a keypair you own.
- Open protocols, designed so federation (shared identity, matchmaking and ladders across instances) can be added later without breaking anything.

### 3. Small and light
- Materials come from noise and math in shaders, so there are almost no bitmap textures (see [05](05-art-and-assets.md)).
- Target: v1.0 with a full roster ships as a **total download under 150 MB**.
- Runs at 60 fps on integrated graphics. Linux is a first-class platform, not a port.

### 4. Familiar depth first, our identity second
- Start close to the established formula (lanes, last-hitting, resistances, epic objectives) so experienced players feel at home and we have a known-good baseline to tune against.
- Then diverge deliberately, based on playtest evidence.

### 5. Data-driven and moddable
- Champions, abilities, items, augments, maps and modes are **data** composed from reusable effect primitives. Code is the escape hatch, not the default.
- Custom game modes and community content are first-class. Mayhem-style augments rely on this.

## Non-goals

- Pay-to-win, loot boxes, paid cosmetics with gameplay-relevant visuals, or battle passes.
- Kernel anti-cheat or any invasive client-side monitoring.
- Photorealism or texture-heavy art.
- Mobile clients, at least before v1.0.
- A huge launch roster. Quality and readability come before count.
- Competing on global infrastructure. We compete on netcode quality and hosting freedom.

## How we differ from the reference game

| Area | Reference game | MFTR |
|---|---|---|
| Source | Proprietary | Open source (AGPL code, CC BY-SA content — proposed) |
| Servers | Company-run only | Self-hosted, federated later |
| Anti-cheat | Kernel driver | Server authority + server-side fog culling + review tools |
| Linux | Unsupported | First-class |
| Download | Many GB | Under 150 MB target |
| Monetization | Skins, passes, gacha | None required; donations / community funding |
| Content | Closed | Data-driven, moddable |

## Target platforms

- **Client:** Linux (x86_64, aarch64), Windows (x86_64), macOS (Apple Silicon). Distributed as Flatpak, AppImage, a Windows installer and a macOS .app, plus itch.io and possibly Steam.
- **Server:** Linux first (x86_64, aarch64, so cheap ARM VPS work), plus Windows and macOS for local hosting.
- **Minimum spec (target):** 4-core CPU from ~2018, an integrated GPU with Vulkan 1.1 / Metal or an OpenGL 3.3 fallback, 4 GB RAM, at 1080p / 60 fps.
