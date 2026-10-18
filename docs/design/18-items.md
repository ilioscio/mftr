# 18 — Items

**Status:** Proposed (D63). This replaces the first item set (D35: 29 items, two passives,
potions and a control ward as the only actives) with a full shop: starters, components, epics,
legendaries, boots, a support quest and jungle companions, ~75 items in all. Every number is a
*(start)* value.

The reference game's item history is our precedent, not our source: we take its *patterns*
(what a stat or a mechanic is for, what went wrong when it went wrong), never its names, text,
icons, or a name and mechanic pair that would read as a copy (09).

## 1. What the reference teaches

From thirteen years of the reference shop:

1. **Simple stats plus one memorable mechanic,** or moderate stats plus a strong situational
   one. The items players remember define a play pattern; an efficient stat bundle doesn't.
2. **Every stat has a job.** Penetration beats armor stacking, anti-heal beats sustain, tenacity
   beats crowd control, crit makes marksmen scale. A stat with no counter or no job is noise.
3. **No item is mandatory for a class.** The mythic era (one build-defining first item per
   champion) gave identity but welded champions to one purchase; it was removed. We have no
   mythic tier, and each class gets 3–5 legendaries that are real choices.
4. **Actives are skill expression,** and the ones that last have readable counterplay: a stasis
   you can wait out, a slow you can see coming, a dash that commits you.
5. **Movement is the strongest stat** and is priced that way.
6. **Gold generation distorts builds;** keep it in the starters and support items that exist for
   it, and cap it.
7. **Items that touch structures, minions or vision are balanced apart** from champion combat
   (our turrets already take champion damage on their own curve, D55).
8. **Retired items are evidence:** keep why they were loved, avoid why they were cut (an
   unreadable proc, a stat that made one class unkillable, an effect that hard-countered by
   accident).

Our own rule on top (D36): **point-and-click never carries hard crowd control.** Item actives
may target a unit, but stuns, roots, knock-ups and pulls stay on skillshots and telegraphed
areas. Item actives slow, shield, heal, dash, cleanse and burst; they never lock.

## 2. Tiers and buying rules

| Tier | What it is | Price | Rules |
|---|---|---|---|
| **Starter** | A lane opener: stats for its class and one small passive | 350–450 | One starter at a time |
| **Component** | One stat, cheap | 300–1300 | Builds into epics and legendaries |
| **Epic** | Two stats or a stat and a passive; sets up a lineage | 700–1200 | |
| **Legendary** | The finished items: stats and one or two unique passives or an active | 2300–3600 | One of each (no duplicates) |
| **Boots** | Movement; upgrades carry a defensive or offensive stat | 300–1100 | One pair |
| **Consumable** | Potions, a flask, control wards | 50–150 | Stack in a slot |
| **Support** | The Pilgrim's quest and its three lanterns (§6) | 400 | One per champion |
| **Jungle** | The three hatchlings (§7) | 450 | One, and only with Claim |

**Unique passives** don't stack with themselves: two items with the same passive give it once,
the stronger version. **Actives** share nothing: each item's active has its own cooldown,
counted per inventory slot (it moves when the item is dragged to another slot), and item
cooldowns survive death. Ability haste does not shorten item cooldowns.

The shop shows tiers as columns (starters and consumables, components, epics, legendaries,
boots, role items), and the HUD shows an active's cooldown as a sweep on its slot, with its key.

## 3. New stats

All combat math lives in `mftr-sim` (02). New stats join the 02 §10 stack as flat bonuses;
percent stats add.

| Stat | Effect | Where it applies |
|---|---|---|
| **Critical strike chance** | A basic attack crits: × (1.75 + bonus crit damage) | Basic attacks only, rolled by the server when the attack lands (§3.1) |
| **Crit damage** | Adds to the 175% | One legendary (Starsteel Blade) |
| **Lethality** | Flat armor penetration, `× (0.6 + 0.4 × level / 18)` (02 §4) | Physical damage |
| **Armor penetration %** | 02 §4 step 3 | Physical damage |
| **Magic penetration** (flat, %) | 02 §4 steps 3–4 | Magic damage |
| **Omnivamp** | Heals for a share of *all* damage dealt | After damage, before the next tick |
| **Tenacity** | Stun, root and slow durations × (1 − t), multiplicative across sources | Not knock-ups or pulls (02 §9) |
| **Slow resistance** | Slows' strength × (1 − r) | |
| **Heal & shield power** | Heals and shields an ability gives × (1 + p) | Support abilities, on allies or the caster |
| **Healing received** | Heals and shields *received* × (1 + h) | All healing but the fountain |

**Reductions** (target-side, they help the whole team and can push below 0, 02 §4): *Sunder*
(armor), *Expose* (armor and magic resist). Applied before the attacker's penetration.

**Wounds** (anti-heal, 02 §6): a unit with Wounds heals `× (1 − w)` from everything (life
steal, omnivamp, potions, regeneration, allies, relics). The strongest applies; tiers 25% and
40%, 3 s, refreshed by each hit. Shown over the unit as a broken-heart icon.

### 3.1 Crits (closes open question 16)

Crits are **rolled by the server when an attack lands**, from the world RNG, never predicted.
A seeded per-champion stream would have let our own client play a crit animation from the
first frame, but it would also tell a modified client which attacks will crit. Instead a crit
shows at impact: a larger damage number with a burst behind it and a sharper hit sound. There
is no separate crit attack animation; the impact sells it.

## 4. Mechanics vocabulary

The passives and actives are built from a small set of shapes, so a modded item (D8) is data:

| Shape | What it does | Items |
|---|---|---|
| **On-hit** | Attacks deal extra damage: flat, % of the target's current health, or every third attack | Arc Bow, Arc Tempest, Gale Saber |
| **Spellblade** | After an ability, the next attack within 10 s deals bonus damage (1.5 s cooldown) | Runed Hilt, Threefold Brand |
| **Cleave** | Attacks also hit enemies near the target | Splitting Axe, Reaping Cleaver |
| **Fanfire** | Ranged attacks fire extra bolts at nearby enemies | Fanfire Quiver |
| **Quicken** | Attacks grant brief movement speed | Rage Knot, Threefold Brand |
| **Sunder / Expose** | Hits reduce the target's resistances (stacking or flat) | Sundering Axe, Hunter's Lantern |
| **Wounds** | Hits apply anti-heal | Barbed Hook, Blight Orb, Briarhide Plate, Blightcaller Tome |
| **Thorns** | Attackers take damage and Wounds | Briarhide Plate |
| **Searing** | Ability hits burn for % max health over 3 s | Inferno Diadem |
| **Smolder** | Burn nearby enemies while in combat | Furnace Plate |
| **Ambush** | Two separate hits on a champion within 2 s: bonus damage and a shield | Duskfang |
| **Rally** | Dropping low grants a shield (a long cooldown) | Lionheart Talisman |
| **Overflow** | Life steal past full health becomes a shield | Crimson Fang |
| **Warmth** | Out of combat, regenerate fast | Heartstone |
| **Topple** | More damage to targets with more max health | Titanbane Bow |
| **Light feet** | Movement speed out of combat | Veilrunner Dagger |
| **Kindle** | Heals and shields on allies empower their attacks | Sunward Lantern |
| **Bulwark** | Immobilizing a champion shields nearby allies | Bulwark Lantern |
| **Tally** | Takedowns stack ability power; deaths lose some | Tally Charm |

| Active | What it does | Targeting | Cooldown | Item |
|---|---|---|---|---|
| **Reaping Arc** | Physical damage around you; resets the attack timer | Self | 10 s | Reaping Cleaver |
| **Bloodletting** | Damage around you; heal per champion hit | Self | 15 s | Bloodletter's Maul |
| **Fleet** | +20% movement speed for 6 s | Self | 45 s | Veilrunner Dagger |
| **Plunge** | Dash to the enemy champion nearest the cursor; bonus damage, and more of yours on it for 3 s | Unit, 500 | 60 s | Shrikeblade |
| **Encase** | Stasis: invulnerable and untargetable, unable to act, 2.5 s | Self | 120 s | Amber Reliquary |
| **Surge** | Dash 275 toward the cursor and fire a fan of 5 bolts | Direction | 40 s | Windstride Belt |
| **Quake** | Slow enemies around you 55% for 2 s | Self | 90 s | Stonewall Aegis |
| **Petrify** | A big shield for 2.5 s; you deal 60% less damage meanwhile | Self | 90 s | Stoneskin Ward |
| **Embrace** | Shield yourself and nearby allied champions | Self | 90 s | Kinship Locket |
| **Absolve** | Cleanse an ally (stuns, roots, slows; not knock-ups or pulls), heal them, speed them | Ally nearest the cursor, 650 | 120 s | Purifying Chalice |
| **Ward** | Place a stealth ward; charges refill at the fountain | Point | — | Pilgrim's Astrolabe, the lanterns |

Item actives are **quick-cast at the cursor**: pressing the item's key (1–6) uses it toward the
cursor at once, like a smart-cast ability. They cancel a recall and obey crowd control (a
stunned or dashing champion can't use an active with a cooldown; potions still work).
Attackers lose a target in stasis: it isn't targetable, so attack orders on it end. Self actives are predicted for our own
champion; damage and effects on others are the server's.

## 5. The catalog

`ID | NAME | COST | STATS | MECHANIC | RECIPE | LINEAGE`. Stats: AD, AP, HP, AR (armor), MR,
AS (attack speed %), AH (haste), MS (flat or %), crit %, LS (life steal %), OV (omnivamp %),
leth (lethality), APen % (armor pen), MPen (flat) / MPen % , ten (tenacity %), SR (slow
resist %), HSP (heal & shield power %), regen (HP/s).

### Starters

| ID | Name | Cost | Stats | Mechanic | Lineage |
|---|---|---|---|---|---|
| 30 | Squire's Blade | 450 | 8 AD, 80 HP, 3% OV | — | fighter/marksman opener |
| 31 | Squire's Ring | 400 | 16 AP, 60 HP, 0.5 regen | — | mage opener |
| 32 | Squire's Buckler | 450 | 130 HP, 6 AR, 1.5 regen | — | tank/melee opener |
| 33 | Tally Charm | 350 | 15 AP, 40 HP | **Tally:** +3 AP a stack; 2 stacks a kill, 1 an assist, up to 10; a death loses 4 | snowball opener |
| 34 | Pilgrim's Ledger | 400 | 30 HP, 1 regen, 5 AH | **Quest** (§6) | support quest |

### Components

| ID | Name | Cost | Stats |
|---|---|---|---|
| 1 | Long Knife | 350 | 10 AD |
| 2 | Spark Shard | 435 | 20 AP |
| 3 | Vital Crystal | 400 | 150 HP |
| 4 | Padded Vest | 300 | 15 AR |
| 5 | Warding Cloak | 450 | 25 MR |
| 6 | Quick Dagger | 300 | 12% AS |
| 8 | Focus Charm | 400 | 10 AH |
| 43 | Fortune's Die | 600 | 15% crit |
| 9 | Heavy Pick | 875 | 25 AD |
| 10 | Charged Wand | 850 | 40 AP |
| 44 | Greatsword | 1300 | 40 AD |
| 45 | Elder Staff | 1250 | 60 AP |

### Epics

| ID | Name | Cost | Stats | Mechanic | Recipe |
|---|---|---|---|---|---|
| 11 | Titan Belt | 900 | 350 HP | — | Vital Crystal |
| 12 | Leech Fang | 900 | 15 AD, 7% LS | — | Long Knife |
| 13 | Arc Bow | 700 | 25% AS | **On-hit:** 15 magic | Quick Dagger |
| 14 | Chain Coat | 800 | 40 AR | — | Padded Vest |
| 46 | Spellward Mantle | 900 | 45 MR | — | Warding Cloak |
| 47 | Runed Hilt | 700 | — | **Spellblade:** +100% base AD physical | — |
| 48 | Splitting Axe | 1200 | 20 AD | **Cleave:** 40% AD to enemies within 300 of the target (ranged: 20%) | Long Knife ×2 |
| 49 | Whetted Kris | 1000 | 20 AD, 10 leth | — | Long Knife ×2 |
| 50 | Gale Cloak | 1050 | 18% AS, 15% crit, 5% MS | — | Quick Dagger, Fortune's Die |
| 51 | Glowing Garnet | 800 | 200 HP, 10 AH | — | Vital Crystal |
| 52 | Barbed Hook | 800 | 20 AD | **Wounds** 25% from physical damage to champions | Long Knife |
| 53 | Blight Orb | 800 | 30 AP | **Wounds** 25% from magic damage to champions | Spark Shard |
| 54 | Rage Knot | 1100 | 200 HP, 15 AD | **Quicken:** attacks grant 20 MS for 2 s | Vital Crystal, Long Knife |

### Legendaries

**Fighter**

| ID | Name | Cost | Stats | Mechanic | Recipe | Lineage |
|---|---|---|---|---|---|---|
| 55 | Threefold Brand | 3300 | 35 AD, 30% AS, 300 HP, 15 AH | **Spellblade** +200% base AD; **Quicken** | Runed Hilt, Rage Knot, Quick Dagger | spellblade hybrid |
| 56 | Sundering Axe | 3000 | 40 AD, 400 HP, 20 AH | **Sunder:** physical damage to a champion reduces its armor 5% a stack, 6 stacks, 6 s | Rage Knot, Focus Charm, Long Knife | anti-tank fighter |
| 57 | Reaping Cleaver | 3200 | 60 AD, 15 AH, 10% LS | **Cleave** 60% (ranged 30%); **Active: Reaping Arc** 100% AD to enemies within 350, attack reset (10 s) | Splitting Axe, Leech Fang, Focus Charm | cleave line |
| 26 | Lionheart Talisman | 3100 | 45 AD, 400 HP, 20% ten | **Rally:** below 30% HP, a shield of 150 + 60% bonus HP for 4 s (60 s) | Titan Belt, Heavy Pick | anti-burst fighter |
| 58 | Bloodletter's Maul | 3100 | 45 AD, 400 HP, 15 AH | **Active: Bloodletting** 100% AD physical within 450; heal 10% missing HP per champion hit (15 s) | Rage Knot, Heavy Pick | drain fighter |

**Assassin**

| ID | Name | Cost | Stats | Mechanic | Recipe | Lineage |
|---|---|---|---|---|---|---|
| 59 | Veilrunner Dagger | 2900 | 55 AD, 18 leth, 10 AH | **Light feet:** +20 MS after 5 s out of champion combat; **Active: Fleet** +20% MS for 6 s (45 s) | Whetted Kris, Focus Charm, Long Knife | roam |
| 60 | Duskfang | 3000 | 60 AD, 15 leth, 15 AH | **Ambush:** two separate hits on a champion within 2 s deal 6% of its max HP physical and shield you 80 + 25% bonus AD for 2 s (8 s) | Whetted Kris, Heavy Pick, Focus Charm | burst |
| 61 | Shrikeblade | 3100 | 60 AD, 18 leth | **Active: Plunge** dash to the enemy champion nearest the cursor (500), 75 + 30% bonus AD physical, then 15% more of your damage on it for 3 s (60 s) | Whetted Kris, Heavy Pick | engage |

**Marksman**

| ID | Name | Cost | Stats | Mechanic | Recipe | Lineage |
|---|---|---|---|---|---|---|
| 62 | Starsteel Blade | 3400 | 65 AD, 25% crit | +40% crit damage | Greatsword, Heavy Pick, Fortune's Die | crit amplifier |
| 63 | Fanfire Quiver | 2800 | 40% AS, 25% crit, 5% MS | **Fanfire:** ranged attacks also fire at up to 2 enemies within 350 of the target for 45% (on-hit applies) | Gale Cloak, Arc Bow | multi-target |
| 64 | Titanbane Bow | 3000 | 35 AD, 25% crit, 30% APen | **Topple:** up to +15% damage to targets with more max HP (full at +1,500) | Greatsword, Fortune's Die | anti-tank |
| 20 | Crimson Fang | 3100 | 60 AD, 15% LS | **Overflow:** life steal past full health becomes a shield, up to 50 + 10/level, for 10 s | Leech Fang, Greatsword | sustain carry |
| 25 | Gale Saber | 2900 | 40 AD, 30% AS, 7% MS | **On-hit:** every third attack deals 60 + 50% bonus AD physical | Heavy Pick, Arc Bow | skirmisher |

**Mage**

| ID | Name | Cost | Stats | Mechanic | Recipe | Lineage |
|---|---|---|---|---|---|---|
| 19 | Grand Grimoire | 3600 | 120 AP | +35% AP | Elder Staff, Charged Wand | AP amplifier |
| 65 | Amber Reliquary | 3000 | 100 AP, 45 AR, 10 AH | **Active: Encase** 2.5 s stasis (120 s) | Charged Wand, Chain Coat | anti-dive |
| 66 | Hollow Scepter | 3000 | 80 AP, 40% MPen | — | Elder Staff, Spark Shard | anti-tank mage |
| 18 | Inferno Diadem | 3000 | 80 AP, 300 HP, 15 AH | **Searing:** ability damage to champions burns 1.5% of their max HP as magic each second for 3 s | Charged Wand, Glowing Garnet | sustained mage |
| 24 | Arc Tempest | 3000 | 70 AP, 40% AS | **On-hit:** 20 + 25% AP magic | Arc Bow, Charged Wand | on-hit AP |
| 67 | Windstride Belt | 2800 | 75 AP, 300 HP, 10 AH | **Active: Surge** dash 275 toward the cursor; 5 bolts in a 40° fan, 110 + 15% AP magic, one hit per target (40 s) | Charged Wand, Glowing Garnet | mobility mage |
| 68 | Blightcaller Tome | 2800 | 75 AP, 20 AH | **Wounds** 40% from magic damage to champions | Blight Orb, Spark Shard, Focus Charm | anti-heal mage |

**Tank**

| ID | Name | Cost | Stats | Mechanic | Recipe | Lineage |
|---|---|---|---|---|---|---|
| 21 | Heartstone | 3000 | 800 HP, 1 regen | **Warmth:** after 6 s without taking damage, regenerate 3% max HP a second | Titan Belt, Vital Crystal | health stacker |
| 22 | Briarhide Plate | 2700 | 70 AR, 350 HP | **Thorns:** an attacker takes 10 + 15% bonus AR magic and 40% Wounds | Chain Coat, Vital Crystal | anti-attack sustain |
| 69 | Furnace Plate | 2800 | 450 HP, 45 AR | **Smolder:** while in combat, 15 + 1.5% bonus HP magic a second to enemies within 325 | Titan Belt, Chain Coat | waveclear tank |
| 70 | Stonewall Aegis | 2800 | 450 HP, 60 AR | Crits deal 30% less to you; **Active: Quake** slow enemies within 500 by 55% for 2 s (90 s) | Chain Coat, Titan Belt | anti-crit |
| 23 | Wardstone Mantle | 2800 | 60 MR, 400 HP, 10 AH, 2 regen | +25% healing and shields received | Spellward Mantle, Glowing Garnet | sustain tank |
| 71 | Stoneskin Ward | 3000 | 60 AR, 60 MR, 15 AH | **Active: Petrify** a shield of 100 + 100% bonus HP for 2.5 s; you deal 60% less damage meanwhile (90 s) | Chain Coat, Spellward Mantle, Focus Charm | frontline |

**Support**

| ID | Name | Cost | Stats | Mechanic | Recipe |
|---|---|---|---|---|---|
| 72 | Kinship Locket | 2500 | 200 HP, 30 AR, 30 MR, 10 AH | **Active: Embrace** shield you and allied champions within 800 for 120 + 10/level, 2.5 s (90 s) | Glowing Garnet, Padded Vest, Warding Cloak |
| 73 | Purifying Chalice | 2300 | 50 MR, 15 AH, 15% HSP, 1 regen | **Active: Absolve** on the ally nearest the cursor (650; else you): cleanse, heal 100 + 10% missing HP, +40% MS fading over 2 s (120 s) | Spellward Mantle, Focus Charm |

### Boots

| ID | Name | Cost | Stats | Recipe |
|---|---|---|---|---|
| 7 | Boots | 300 | 25 MS | — |
| 15 | Swift Boots | 900 | 60 MS, 25% SR | Boots |
| 16 | Battle Boots | 1100 | 45 MS, 25% AS | Boots, Quick Dagger |
| 17 | Sage Boots | 950 | 45 MS, 15 AH | Boots |
| 40 | Ironshod Boots | 1100 | 45 MS, 20 AR; basic attacks deal 12% less to you | Boots, Padded Vest |
| 41 | Stalwart Sabatons | 1100 | 45 MS, 25 MR, 30% ten | Boots, Warding Cloak |
| 42 | Spellstriders | 1100 | 45 MS, 15 MPen | Boots |

### Consumables

| ID | Name | Cost | Use |
|---|---|---|---|
| 27 | Health Potion | 50 | Heals 120 over 15 s; 5 to a slot |
| 28 | Refillable Flask | 150 | Heals 100 over 12 s; 2 charges, refilled at the fountain |
| 29 | Control Ward | 75 | Placed at the cursor (01 §8); 2 to a slot |

## 6. The Pilgrim's quest (support)

A support earns gold by fighting, not farming, and its item grows into one of three lanterns:

1. **Pilgrim's Ledger** (starter, 400). *Tithe:* hitting an enemy champion or structure with an
   attack or ability, or healing or shielding an allied champion, earns 18 gold, on 3 charges
   that refill one every 8 s. *Spoils:* attacking
   an enemy minion below 40% health (60% for melee) with an allied champion within 1,000 kills
   it and gives that ally the gold; the holder gets its own share too; 3 charges, one per 30 s.
   Holding the Ledger, the holder earns only half the gold of minions it kills itself (the
   reference's anti-farm rule: the quest pays a support to support).
2. At **400 quest gold** it becomes **Pilgrim's Astrolabe** (the Ledger's stats, +100 HP, 10
   AH; *Ward:* 3 stealth wards, placed with its key, refilled at the fountain).
3. At **1,000 quest gold** it can be **upgraded for free** in the shop into one lantern (each keeps the
   Ward active, 4 charges):

| ID | Lantern | Stats | Passive |
|---|---|---|---|
| 36 | Sunward Lantern (enchanter) | 35 AP, 150 HP, 10 AH, 10% HSP | **Kindle:** your heals and shields on allies make their next 4 s of attacks deal +20 magic |
| 37 | Bulwark Lantern (warden) | 250 HP, 15 AR, 15 MR, 10 AH | **Bulwark:** immobilizing an enemy champion shields allies within 600 for 60 + 5% your bonus HP (8 s) |
| 38 | Hunter's Lantern (engage) | 25 AD, 200 HP, 10 AH | **Expose:** your hits on champions reduce their armor and MR by 12% for 4 s |

## 7. Hatchlings (jungle)

A jungler with Claim (01 §7) buys one hatchling (450). Each makes monsters easier: **+25% damage
to monsters, 20% less damage taken from them.** Every monster the holder kills feeds it
(1 a small one, 2 a big one); at **32** it grows up, and each grown hatchling changes Claim:

| ID | Hatchling | Grown up |
|---|---|---|
| 39 | Windling | Claiming grants +40% MS fading over 2 s |
| 74 | Barkling | Claiming, or 10 s out of champion combat, shields you for 150 + 10% bonus HP |
| 75 | Flintling | Claim can target enemy champions: 60 + 4% of their max HP true damage and a 20% slow for 2 s |

Flintling's Claim is point-and-click with a slow only (D36). Claim's own cooldown applies.

## 8. Art: item models and their icons

Every item is a **model** first (05 §6), from which its icon is rendered:

- **Source:** `art/items/build.py` builds every item model from code on the prop toolkit
  (`mftr_blender.item`, faceted and vertex-colored like the champions, 05 §3), one `.blend` per
  item in `art/items/<key>/`, exported as a `prop` pack (`export/<key>.glb` and its sidecar).
  An item is about 150–800 triangles: it's only ever seen as an icon.
- **Shape first** (14 §2): each item reads from its outline alone; families share a motif
  (blades for AD, staffs and crystals for AP, plates for armor, cloth and runes for MR, gems
  for health), and the epic and legendary of a lineage share a shape that grows (the Long
  Knife's blade is in the Whetted Kris and the Veilrunner Dagger).
- **Color code by stat**, in the material slots: steel and red for AD, violet glow for AP
  (`emissive`), ruby for health, bronze for armor, teal for magic resist, gold for attack speed
  and crit, cyan for haste. Darker neutrals than you'd think: the arena's sun blows out light
  colors, and so does the icon key light.
- **Icons are rendered by the client** from the models, like champion portraits: a
  three-quarter view from above, a warm key and a cool rim light, on the item's tier backdrop
  (components slate, epics teal, legendaries gold on violet, boots green, starters and role
  items bronze) with its rim. Rendered once per session, cached, and a pack can replace any
  model. Until a model loads (or for a modded item with none), the SVG in `art/items/icons/` or
  its main stat's glyph stands in.

## 9. Bots

Bots (17) buy from per-champion build paths over the new catalog (a starter, boots by the
enemy's damage, the defensive legendary by the enemy's damage as today), and use actives by
rule: Encase when about to die, Fleet and Surge to chase or escape, Reaping Arc and Bloodletting
with enemies in reach, Quake when engaged, Petrify and Embrace when allies take burst, Absolve
on a stunned ally, Plunge on a killable champion. Supports run the quest; junglers buy a
hatchling.

## 10. Implementation notes

- **Data:** `mftr-sim/items.rs` holds the catalog as Rust data until RON (D8): an item has stats
  (`Bonus`), up to two unique passives, an active, a tier and a recipe. Unique passives are
  collected once per inventory (`items::passives`), the strongest of each kind.
- **State:** per-champion item state lives in `Progress::gear` (survives death: active and
  passive cooldowns, Tally stacks, quest gold, hatchling food) and per-unit timed effects in
  `UnitState::fx` (Wounds, burns, Sunder stacks, Expose, movement buffs, stasis, Petrify, the
  Ambush window, Plunge's mark). Both are hashed and sent with the own champion's state, so our
  own actives are predicted.
- **The damage pipeline** (02 §5) gains the attacker: `deal_damage` takes a `Hitter` (the
  source's penetration, level and crit) and applies reductions, penetration, Topple and
  Petrify; after-damage effects (omnivamp, Wounds, Searing, Sunder, Expose, Ambush) run once
  per tick over that tick's damage events, so one hit never triggers another's item effects.
- **Heals and shields** go through one function each, which applies Wounds, healing received
  and heal and shield power.
- **Wire (protocol 27):** the own champion's state carries `Gear` and `Fx` (one bit each when
  empty, unset times one bit), damage events a crit flag, missiles the item that fired them,
  dash strikes a 4-bit slot (8 + the inventory slot for an item's), other units three display
  flags (in stasis, Wounded, burning), and a new event `ItemUsed` (22: unit, item, time,
  point, command sequence) plays an active's effect for everyone.
- **Prediction:** our own actives on ourselves (Fleet, Encase, Petrify, Embrace's own shield,
  Reaping Arc's reset, Plunge's and Surge's dashes) are predicted bit-exactly (a parity test
  drives them at random); what they do to others is the server's. The combat clocks some
  passives read (Smolder, Warmth, Light feet, a Barkling) are kept only by their holders, so
  champions without those items hash exactly as before.

## 11. As built

- **The sim:** `mftr-sim/items.rs` (the catalog, tiers, buying rules, the stat stack, bot build
  paths), `gear.rs` (item state, `Hitter`, mitigation, heals and shields, after-damage effects,
  per-tick effects), the actives in `world.rs` (`use_item`, `item_nova`, `absolve`).
  `world_item_tests.rs` covers penetration, crits, Wounds, tenacity, every active, Spellblade
  and Cleave, the quest, Tally, and prediction with actives.
- **The client:** the shop in five sections (starters and consumables, components, epics,
  legendaries in class order, boots with this map's role items) on twelve columns; tooltips
  generated from the data (`mftr-gdext/itemtext.rs`), with what an item has built up (Tally
  stacks, quest gold, hatchling food); the item bar's cooldown sweeps and counters; crit
  numbers bigger and gold with a "!"; icons over health bars for stasis, Wounds and burns; an
  amber shell around an encased champion; each active's effect on `ItemUsed`.
- **Art:** `art/items/build.py` builds all 75 models on `mftr_blender.item` (sweeps, lathes,
  extrusions, cut gems), saves each `.blend`, exports its prop pack and renders a review sheet
  (`blender -b --python art/items/build.py -- [--only key,key] [--review DIR]`). Models are
  130–750 triangles. The client renders icons from them (`portraits.gd`'s "item:<key>"),
  composites them on the tier backdrop (`item_icons.gd`), and `--dump-item-icons FILE` writes
  the whole set as the game draws it. A test checks every catalog item has a valid model.
- **Bots:** buy by role (a hatchling for the jungler, the Ledger for the support, then its
  lantern), swap boots and the defensive legendary by the enemy's damage, sell a starter for
  room late, and use actives by the rules of §9; `mftr-tools botmatch` lists each champion's
  final items.

## 12. Balance pass (2026-10-18)

**Method.** Two tools, both in `mftr-tools`:

- `mftr-tools items`: the set on paper. Each item's stats priced at what the components charge
  for them (AD 35, AP 21.75, health 2.67, armor 20, MR 18, attack speed 25 a percent, haste 40,
  crit 40 a percent, move speed 12; the rest by convention), against its cost; and what each
  legendary adds to a level-13 champion of its class with a fixed core, against a reference
  bruiser (2,400 health, 110 armor, 70 MR): attack damage a second, an ability combo, effective
  health, each per 1,000 gold.
- `mftr-tools botmatch --items [--shuffle-builds]`: what items did in bot matches (first held,
  actives used and item damage per minute held, holders' win rate, damage and K/D, crit rate,
  the quest's stages, hatchlings growing up). `--shuffle-builds` draws each bot's legendaries at
  random from its class's, so over many games an item's results aren't its champion's.

**What it showed.**

- Components price exactly; epics 58–119% in stats (the low ones carry Spellblade, Cleave or
  Wounds); legendaries 74–125% before their passives and actives.
- Over 400 shuffled ARAM games, each class's legendaries won within about 7 points of each
  other. Re-running the same 400 seeds moved an unchanged item 3–4 points, so that spread is
  about the method's resolution: the five holders of a game share its result. No legendary
  stands out above it.
- Against the old item set on the same 100 seeds and lineups, the new set narrows champion
  win rates (old 22–92%, new 27–80%; tanks up, Quill down) and ends games: 12% reached the
  30-minute limit, against 40%.
- **Outside the items:** Shade (the only assassin) wins about 22–27% and the tanks about 30%,
  on the old items too, and each class's legendaries are level: that's the champions or how
  bots play them, for a champion pass.

**Changes.**

| Item | Was | Now | Why |
|---|---|---|---|
| Squire's Ring | 18 AP, 80 HP, 1 regen | 16 AP, 60 HP, 0.5 regen | 174% stat value for a starter; now ~140% like the others |
| Squire's Buckler | 110 HP | 130 HP | the weakest starter (122%) |
| Barbed Hook | 15 AD | 20 AD | 66% stats for an epic |
| Blight Orb | 25 AP | 30 AP | 68% stats |
| Spellstriders | 12 MPen | 15 MPen | the weakest boots (83%) |
| Fanfire Quiver | 50% a bolt | 45% | led marksmen by ~5 points in ARAM's grouped fights |
| Arc Tempest | 60 AP, 15 + 20% AP | 70 AP, 20 + 25% AP | last among mages |
| Windstride Belt | 70 AP, 100-damage bolts | 75 AP, 110 | second-last among mages |
| Stonewall Aegis | 400 HP | 450 HP | last among tanks |
| The quest | Tithe 15 gold, a charge per 10 s, only for hits; stages at 500 and 1,500 | 18 gold, 8 s, healing and shielding allies counts; 400 and 1,000 | bots reached the Astrolabe at 18:30 and never finished; enchanters, who heal, starved |
| Hatchlings | grown at 20 | 32 | grew up at 5:37 |

**Pacing after (40 classic games, ~23 minutes each):** supports reach the Astrolabe at 12:17
and finish the quest at 19:04 (45 of 80 within the match); hatchlings grow up at 8:03. Bots use
Tithe at about a third of its cap; a human support poking and healing steadily should reach the
stages around 5 and 12 minutes.
