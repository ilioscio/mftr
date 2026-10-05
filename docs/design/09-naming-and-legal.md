# 09 — Naming & Legal

*Not legal advice. Before a public release, have someone qualified review the names and the champion roster.*

## 1. Principles

- **Game mechanics and genre conventions** (lanes, last-hitting, the resistance formula, towers, jungle camps) are broadly shared across the genre. We build on them freely.
- **Names, characters, artwork, lore, audio, UI layouts, logos and tooltip text** belong to their owners. We use **none** of them: no copied text, no traced art, no sampled audio.
- **Goal (decided): as familiar as possible to players of the reference game, while respecting its owner's intellectual property and following copyright law.** Familiarity comes from shared *systems and conventions*: controls, map structure, economy, stat math, ability vocabulary (hooks, line skillshots, dashes, knock-ups). It never comes from their characters or expression.
- **Champion kits:** every champion is an original character with an original kit built for an archetype. Reference champions are fine as *tuning benchmarks* ("our hook should feel about as punishing as X's"), not as blueprints. We don't make a renamed 1:1 copy of any specific character's kit, look or theme.
- Inspired-by references stay in design docs only, never in game files, store pages or marketing.

## 2. Placeholder name map

All names are working names. Change them freely, but keep this table updated.

| Reference concept | MFTR working name | Notes |
|---|---|---|
| Nexus | **Base** | |
| Inhibitor | **Gatehouse** | |
| Super minion | **Elite minion** | |
| Summoner's Rift | **Crossroads** | Main 3-lane map |
| Howling Abyss | **The Bridge** | ARAM map |
| Summoner spells | **Utility spells** | |
| Flash | **Blink** | |
| Smite | **Claim** | Avoids SMITE (another MOBA's trademark) |
| Snowball (ARAM) | **Mark** | |
| Elemental Dragons / Dragon Soul / Elder | **Elemental Wyrms / Wyrm Soul / Elder Wyrm** | |
| Voidgrubs | **Mites** | |
| Rift Herald | **Siege Beast** | |
| Baron Nashor | **The Colossus** | |
| Champions | Champions (generic term) | Open question: own term? |
| ARAM | ARAM | Genre-generic acronym; low risk |
| URF | **Hyper** | |
| Arena augments | Augments (generic term) | |
| ARAM: Mayhem | **ARAM: Mayhem** (working) | Consider renaming, e.g. "ARAM: Chaos" |
| Lord Dominik's / Void Staff / Black Cleaver etc. | Original item names TBD | Mechanics only |
| Jeweled Gauntlet, Giant Slayer, Goliath… | Spellcrit, Pebble, Titan… | See [06](06-modes-and-augments.md) |

The project name itself, **MFTR (Moba For The Rest)**, is a working title. Check for trademark conflicts before branding work.

## 3. Licensing (decided — D7)

| Part | License | Why |
|---|---|---|
| Code (Rust crates, GDScript, shaders) | **AGPL-3.0-or-later** | Network copyleft: anyone running a modified server for others must share the modifications. That keeps hosted and federated instances open. |
| Art, audio, content data, docs | **CC BY-SA 4.0** | Share-alike, and one-way compatible with GPLv3 |
| Third-party | Godot (MIT), Rust crates (MIT/Apache etc.) | All compatible with AGPL; checked with `cargo deny` |

- Contributions use a **DCO** (`Signed-off-by`) rather than a CLA. No single party can relicense the project to proprietary.
- **Trademark policy** for the project name and logo (once chosen): forks are welcome but must rename if they diverge significantly. The same model as many open-source games.
