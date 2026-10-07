# 01 — Core Gameplay

Placeholder names are used throughout; see [09](09-naming-and-legal.md) for the mapping.

## 1. Win condition

Two teams of five start in opposite corners of a closed map. The first team to destroy the enemy **Base** wins. The Base can only be attacked once that team has lost at least one Gatehouse and both of its base turrets. Teams can surrender by vote (4/5, or 5/5 before a minimum time).

## 2. Match rhythm

| Phase | Approx. time | Feel | Primary decisions |
|---|---|---|---|
| **Laning** | 0–14 min | Tactical, isolated 1v1 / 2v2. Cooldown and positioning trades, last-hitting, tracking the enemy jungler. | Farm vs. trade, wave management, recall timing, ward placement |
| **Mid game** | 14–25 min | Volatile. Outer turrets fall, the map opens, and 3v3 / 4v4 skirmishes break out over vision and objectives. | Rotations, objective trades, using gold leads, vision control |
| **Late game** | 25 min+ | High-stakes 5v5 teamfights. Death timers reach 40 s+, so one mistake can end the game. | Engage/disengage, Colossus/Elder Wyrm calls, split-push vs. group |

This rhythm is a **design target**, not a scripted event. It comes from economy scaling, turret durability, objective spawn times and death timers. Playtest telemetry should confirm the phases appear naturally.

## 3. The main map (codename "Crossroads")

Inspired by the classic 3-lane layout:

- **Three lanes:** Top, Mid and Bottom, joining each team's base along the diagonals and edges.
- **Jungle:** four quadrants between the lanes, split by a **river** running corner to corner.
- **Brush:** patches of tall grass that block vision from outside. Units inside are hidden unless an enemy is also inside or has a ward there.
- **Walls:** impassable terrain that blocks vision. Some are thin enough for dashes and blinks to cross.

### Structures (per team)

| Structure | Count | Notes |
|---|---|---|
| Outer turret | 1 per lane | Has **plating** (start: 5 plates, each grants gold when broken) until 14:00 |
| Inner turret | 1 per lane | |
| Gatehouse turret | 1 per lane | Guards the Gatehouse |
| **Gatehouse** | 1 per lane | When destroyed: allied **Elite minions** spawn in that lane; respawns after 5:00 |
| Base turret | 2 | Guard the Base |
| **Base** | 1 | Spawns minion waves; destroying it wins the game |
| Fountain | 1 | Spawn point. Fast regen, shop access, and a lethal defensive laser |

Turrets must be destroyed in order down each lane. **Turret AI** priority:
1. An enemy champion that damages an allied champion within turret range.
2. The closest enemy minion, with siege minions before casters and melees.
3. The closest enemy champion.

Consecutive shots on a champion ramp up in damage. Turrets take reduced damage when no enemy minions are nearby (backdoor protection).

## 4. Minions

- The first wave spawns at **~1:00**, then one wave every **30 s** per lane.
- A wave is 3 melee + 3 caster. A **siege** minion joins every 3rd wave (every 2nd later in the game).
- Minions walk their lane, fight what they meet (target priority mirrors turrets), and **aggro onto enemy champions that attack allied champions** nearby. That aggro rule is what makes trading in lane a decision.
- Minion stats scale with game time.
- **Super minions** (the elite minions; after a Gatehouse falls) are tanky and hit hard. While a team has an enemy Gatehouse down, each of its waves brings one in front until that Gatehouse respawns. *(start, A5)* 1,500 health, 100 armor, 190 damage, 170 range, 0.85 attacks/s; turret shots take 7% of their health; 60 gold. The planned aura that buffs nearby allied minions comes later.

### Unit collision (decided: like the reference game)

Every unit has **two radii**:

| Radius | Used for | *(start)* champion | *(start)* melee / caster / siege / super minion |
|---|---|---|---|
| **Gameplay radius** | Hitboxes for abilities, attack range, targeting | 65 u | 48 / 48 / 65 / 80 u |
| **Collision radius** | Unit-vs-unit movement blocking | 35 u | 25 / 25 / 35 / 45 u |

- Collision radii are **small**: a single minion is easy to walk around, and units slide past each other when there's a gap.
- But they are **solid**. When minions clump (fighting in a wave, or bunched in a narrow spot), the gaps close and they **block** champions. Minion-block and body-blocking are intended skill expression.
- **No shoving.** A moving unit never pushes another unit. It slides along the blocker, waits, or re-paths around it.
- **Every unit blocks every unit:** allied and enemy champions, and minions of either team (D23, measured in [R03](reference/R03-video-allied-champion-collision.md)). A champion walking into a standing ally stops at contact and paths around it.
- **Ignoring collision:** dashes, blinks, ghosted units (Ghost spell, some abilities) and dead units ignore unit collision. Terrain and structures **always** block.
- Minions look for free attack positions around their target. That is what naturally forms the clumps.
- Netcode consequences (predicting your champion when minions block it) are covered in [03a §5](03a-netcode-time-and-prediction.md#5-predicting-unit-collision-minion-block).

## 5. Economy

Gold is the only currency. It is spent at the shop while in the fountain (or anywhere during ARAM death timers).

| Source | *(start)* values |
|---|---|
| Starting gold | 500 |
| Passive income | ~2 g/s from first wave spawn |
| Melee / caster / siege / super minion (last hit) | 21 / 14 / 60→90 (scales with time) / 60 |
| Champion kill | 300 base. **Bounty** rises with kill streaks and falls with death streaks |
| Assist | Kill gold split among assisters (50% of the kill value, shared) |
| Turret plating | 125 per plate to the local damager(s) |
| Turret destroyed | Local gold to damagers + team-wide gold |
| Jungle camps | Varies by camp |
| Support item quest | Support-specific income from poking and executing minions, capped |

**Last-hitting is the core micro-skill of the economy.** Minion deaths without a last hit grant no gold. A minion's health bar shows a last-hit threshold tick (accessibility option, on by default). Last-hitting has to stay a timing skill, not a reaction-speed lottery: see the netcode notes on remote-unit interpolation.

## 6. Experience & levels

- Champions within **1400 u** of a dying enemy unit share its XP, with a bonus when there are fewer sharers. This makes solo lanes level faster than duo lanes.
- Levels **1–18**. Each level-up grants one ability point. The ultimate can be ranked at 6 / 11 / 16; basic abilities go to rank 5.
- Kills on higher-level champions grant bonus XP (comeback mechanic).

## 7. Jungle & epic monsters

### Camps
Neutral camps in each quadrant respawn on timers. Two **buff camps** per side give the killer a temporary buff (start: one grants mana/energy regen + ability haste, the other grants on-hit slow + damage over time). Smaller camps give gold and XP.

### Epic objectives (placeholder names)

| Objective | Inspired by | Spawns | Reward |
|---|---|---|---|
| **Elemental Wyrms** | Elemental Dragons | ~5:00, respawn ~5:00 | Permanent, stacking team-wide stat buff by element. The map's terrain shifts to match the element after the 2nd wyrm |
| **Wyrm Soul** | Dragon Soul | When a team takes 4 wyrms | Powerful permanent team effect matching the soul's element |
| **Elder Wyrm** | Elder Dragon | After a Soul is claimed | Temporary buff: execute low-health enemies, damage over time |
| **Mites** | Voidgrubs | ~6:00, two waves | Stacking structure-damage buff; at 3+ stacks, attacks on structures spawn small mites that also hit |
| **Siege Beast** | Rift Herald | ~14:00 | Summonable battering ram that charges a turret for heavy damage |
| **The Colossus** | Baron Nashor | ~20:00 | Temporary team buff: large AD/AP, faster recall, and **empowered nearby minions** for sieging |

Epic monsters are secured with the jungler's **Claim** utility spell (Smite-equivalent): true damage to monsters, usable as a finisher.

## 8. Vision

- Each unit has a vision radius. **Walls and brush block line of sight.**
- **Wards** are placeable vision sources: trinket wards (limited charges), control wards (reveal and disable enemy wards, one at a time), and support-item wards.
- **Sweeper** trinket reveals and disables wards in an area.
- Vision is computed **on the server** and only visible entities are sent to each team. Map hacks are impossible by construction (see [03](03-netcode.md#10-fog-of-war-culling)).

## 9. Champions

### Classes
| Class | Identity | Typical role |
|---|---|---|
| Tank | Durability, crowd control, engage | Top, Support, Jungle |
| Bruiser / Fighter | Sustained damage + survivability | Top, Jungle |
| Mage | Ranged AoE magic damage, skillshots | Mid, Support |
| Assassin | Mobility, single-target burst | Mid, Jungle |
| Marksman | Sustained ranged physical DPS via basic attacks; fragile; gold-hungry | Bottom |
| Enchanter | Heals, shields, buffs | Support |
| Engage support | Hooks, knock-ups, peel | Support |

Standard layout: **Top / Jungle / Mid / Bottom (Marksman) / Support** (1-1-1-2).

### Kit structure
Every champion has:
- **Passive** (innate)
- **Q / W / E** basic abilities (rank 1–5)
- **R** ultimate (rank 1–3)
- A **resource:** Mana, Energy, Fury/Rage, Health cost, or none
- **Basic attack:** melee or ranged, with an attack windup (see [02](02-combat-math.md))

All abilities are built from the data-driven effect system ([04](04-architecture.md#4-ability--effect-system)). Kits should use **skillshots for high-value effects**: hard CC and big burst should usually be dodgeable.

### Crowd control
Stun, Root, Slow, Silence, Disarm, Ground (no dashes), Blind (attacks miss), Knock-up / Knock-back (airborne), Fear/Flee, Taunt, Charm, Sleep, Polymorph, Suppression. See [02](02-combat-math.md#9-crowd-control--tenacity) for tenacity and cleanse rules.

## 10. Utility spells (Summoner-spell equivalents)

Two per champion, chosen before the match. *(start)* list:

| Name | Inspired by | Effect |
|---|---|---|
| Blink | Flash | Short-range instant teleport, ~400 u, long cooldown |
| Claim | Smite | True damage to monsters/minions. Upgrades as you clear camps. Required for junglers |
| Ignite | Ignite | True damage over time + healing reduction |
| Mend | Heal | Heal self + ally, brief haste |
| Barrier | Barrier | Short self shield |
| Purge | Cleanse | Removes most CC and debuffs |
| Exhaust | Exhaust | Slows and reduces the damage dealt by a target |
| Ghost | Ghost | Large movement speed boost, ghosted |
| Teleport | Teleport | Channel to an allied structure (later: wards/minions) |
| Mark | Snowball (ARAM) | Throw a mark; recast to dash to the marked enemy |

## 11. Items

- **Components → Epic → Legendary** build paths. Six inventory slots plus a trinket slot.
- Starter items, boots tier, consumables (potions, elixirs, control wards).
- Legendary items have **unique passives** that can't stack with themselves.
- Supports have a **support item quest** that upgrades as they earn income, and it grants wards.
- The shop has recommended builds (data-driven, community-editable). Undo is allowed until you leave the fountain.

## 12. Death, respawn & recall

- **Respawn timer:** grows with level (start: ~6 s at L1 to ~50 s at L18) and a late-game multiplier.
- **Recall:** an 8 s channel to the fountain, cancelled by moving, taking damage or casting.
- Gray-screen spectating while dead. The shop works while dead in the fountain.

## 13. Controls

Genre-standard by default, fully rebindable:

| Action | Default |
|---|---|
| Move | Right-click ground (hold to keep moving toward the cursor) |
| Attack | Right-click enemy |
| Attack-move | A + left click, or "attack-move on right-click" option |
| Abilities | Q W E R |
| Utility spells | D F |
| Items / trinket | 1–6 / 4 |
| Stop | S |
| Recall | B |
| Level-up ability | Ctrl + Q/W/E/R |
| Camera lock toggle / center | Y / Space |
| Ping wheel | G (alt + click quick ping) |

**Cast modes** per key: Normal (press → show indicator → click), Quick cast (cast on press at the cursor), and Quick cast with indicator (cast on release). Ability **range indicators** use the true range and the true hitbox width.

## 14. Communication

Context pings (danger, on my way, missing, assist, objective timers, item/spell cooldowns via alt-click). Team chat on by default; all-chat off by default. Pings are rate-limited.
