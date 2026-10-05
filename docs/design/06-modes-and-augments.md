# 06 — Game Modes & Augments

## 1. Mode framework

A **mode** is data: map, team sizes, champion selection rules, economy multipliers, rule toggles, augment schedule, win condition, and optional rule behaviors. New modes should rarely need engine changes.

```ron
Mode(
    id: "aram_mayhem",
    map: "bridge",
    teams: [5, 5],
    selection: AllRandom(rerolls: 2, bench: true),
    rules: [NoFountainHealAfterLeaving, ShopOnlyWhenDead, HealthRelics],
    utility_spells: Restricted(["blink", "mark", "mend", "barrier", "ignite", "exhaust", "purge", "ghost"]),
    augments: Some(AugmentSchedule(at_levels: [1, 7, 11, 15], choices: 3, rerolls: 1)),
)
```

## 2. Modes

| Mode | Map | Summary | Milestone |
|---|---|---|---|
| **Duel Sandbox** | Flat test arena | 1v1–3v3 skillshot dueling, no economy. Netcode proving ground and practice tool | M1 |
| **ARAM** | The Bridge (single lane) | All random champions, one lane, no jungle, no fountain healing after you leave base, shop only when dead | M2 |
| **ARAM: Mayhem** | The Bridge | ARAM + augment drafting | M3 |
| **Normal (Blind / Draft)** | Crossroads (3-lane) | Standard 5v5 | M4 |
| **Ranked (Solo/Duo, Flex)** | Crossroads | Draft pick with bans; rating per server, federated later | M5+ |
| **Arena** | Small arenas | 2v2v2v2 rounds with augments between rounds | Post-M5 |
| **Rotating modes** | various | Hyper (URF-like: huge haste, no costs), One-For-All, event modes | Post-M4 |
| **Custom games** | any | Any mode plus rule overrides, bots, spectators, tournament settings | M2 onward |

### ARAM specifics
- **The Bridge:** one lane, both teams' turrets plus Gatehouse plus Base, narrow sides with brush, and **health relics** that respawn on a timer.
- Random champion assignment from each player's pool (in open source, everyone owns everything, so the pool is the full roster minus server-disabled champions), with **rerolls** and a shared **bench** to swap from.
- Faster passive gold, starting gold for an early full item, and a **Mark** utility spell (throw, then recast to dash).

### Rotating mode notes
- **Hyper (URF-like):** +300 AH on basic abilities, no resource costs, faster attack speed. Great for stress-testing the netcode and VFX clutter.
- **One-For-All:** a team votes on one champion. Requires mirror matches and per-player colors to keep readability.

## 3. Augments (Mayhem & Arena)

Augments are permanent modifiers drafted during the match. They're built from the same primitives as items and abilities ([04](04-architecture.md#4-ability--effect-system)), which is why we can have hundreds without hundreds of code paths.

### Draft flow *(start)*
- At scheduled levels, each player gets **3 choices** of one rarity tier and can **reroll** a limited number of times.
- Tier per draft slot can be fixed or rolled (e.g. guaranteed one Prismatic by mid-game).
- Drafting happens live: while the draft window is open the player is untargetable in base, or it happens during death, depending on mode config.

### Tiers
| Tier | Role | Examples (working names) |
|---|---|---|
| **Silver** | Simple stat conversions and consistent passives | *Conversion* (AD → AP at 110%); *Resonance* (your heals and shields deal damage and slow around the target) |
| **Gold** | Playstyle adjustments and strong conditionals | *Spellhunger* (permanent AP on each ability hit); *Brute Force* (flat AD + lethality + haste) |
| **Prismatic** | Identity-rewriting game-breakers | see below |

### Prismatic archetypes
1. **Rule breakers:** flip a fundamental rule.
   - *Spellcrit* — abilities can critically strike.
   - *Close Quarters* — ranged becomes melee, with large stat compensation scaled by the range lost.
   - *Fundamentals* — ultimate disabled, with large bonuses to basic-ability damage, healing, shielding and haste.
2. **Physical transformations:** change size, and so hitbox and stats.
   - *Pebble* — shrink a lot, gain movement speed, and deal bonus damage to larger targets.
   - *Titan* — grow by 50%, gain health and adaptive force.
   - *Unstable Experiment* — on each respawn, randomly huge or tiny.
   - **Readability note:** size changes alter the *gameplay radius* and the visuals together. A tiny champion's hitbox really is tiny, and dodging honestly still works.
3. **Ability augments:** target a champion's defining ability via delivery transformers.
   - *Multishot* — linear projectiles fire 3 in a spread.
   - *Echo* — the ability repeats after a delay at reduced power.
   - *Chain Reaction* — on hit, the effect bounces or splits.
   - Requires abilities to declare which transformers they accept (data). Not every ability makes sense tripled.
4. **Quest augments:** objectives with an overpowered reward.
   - *Champion of Chaos* — reach N takedowns → receive a legendary all-stats relic item.
   - *Fusion* — own items A + B → they fuse into a legacy super-item.
5. **Utility spell replacements:** replace your second utility spell with a special active.
   - *Blade Dance* — become briefly untargetable and dash between nearby enemies.
   - *Trickster* — teleport and go invisible; on death, leave a trap that fears and deals true damage.
   - *Megamark* — the Mark spell becomes huge, pierces minions and knocks up champions.

### Augment design guidance
Good augments either **cover a champion's weakness** (e.g. a dash for an immobile tank) or **hilariously over-index on a strength** (e.g. Spellcrit on a burst assassin). Both are fun. What isn't fun is invisible power, so every augment with combat impact must have a **visible indicator** (icon above the health bar, VFX tint, or size change) that enemies can learn to read.

### Netcode implications
- Augments can multiply projectiles (Multishot × Hyper mode). The projectile spawn event supports **batched spawns** (one event, N projectiles, shared parameters plus per-projectile angle) to keep bandwidth flat.
- Size changes are a replicated stat. Prediction uses the new gameplay radius from the moment of the server event.
