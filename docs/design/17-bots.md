# 17 — Bots

**Status:** Proposed (D62); built, as §12 records. This replaces the M2 slice 5 bot (a single list of rules in
`mftr-server/src/bots.rs`) with the `mftr-bots` crate foreseen in [04 §6](04-architecture.md).
The owner's brief: bots carry the PvE experience the game launches on, so they should play
**better than the reference game's bots**: more nuanced and more strategic, readable and fun
to play with and against, and cheap enough that a server still hosts many matches.

## 1. Pillars

1. **Honest.** A bot knows only what its team sees (D37), plus what it remembers seeing. It never
   reads a hidden unit, a hidden missile or a hidden cooldown. It tracks enemy cooldowns by
   watching casts, and keeps camp timers by watching camps, as a good player does.
2. **Human-limited.** Every tier reacts with a delay, aims with error and is capped in actions a
   second. The top tier wins by deciding better, not by being a machine (§3).
3. **Strategic.** Each team has a **coach** that reads the map and gives every bot a task: lane,
   gank, group, siege, defend, take an objective. The bots then choose between that task and
   what's in front of them (§5–§6).
4. **Readable and social.** Bots announce their plans with pings and follow human teammates'
   pings and leads (§8). A bot ally shouldn't leave a human to fight alone, steal a human's lane
   or ignore a call.
5. **Cheap.** All bot work in a match should cost less than its simulation, and no bot tick
   should spike (§10). Bots think in staggered slots, share one team snapshot per tick, and
   never run a pathfinding query themselves.

## 2. Layers

```
              ┌──────────────── Coach (per team, ~1 Hz) ─────────────────┐
 Memory ───►  │ phase · lane states · team strength · opportunities ·     │ ──► pings
 (vision,     │ human pings → tasks for each bot                          │
  events)     └──────────────────────────┬────────────────────────────────┘
                                         │ Task
              ┌──────────── Agent (per bot, 2–10 Hz) ────────────────────┐
 Board ────►  │ behaviors scored (utility, with commitment):              │
 (this tick's │ survive · recall · shop · farm · trade · kill · fight ·   │
  team view)  │ gank · jungle · objective · siege · defend · ward · follow│
              └──────────────────────────┬────────────────────────────────┘
                                         │ intent
              ┌───────────── Micro (same think) ─────────────────────────┐
              │ dodge · last hits · kiting · targeting · combos · aim ·   │ ──► Commands
              │ spells, potions, items                                    │
              └───────────────────────────────────────────────────────────┘
```

- **Knowledge** (`knowledge.rs`): what a map is, computed once per map and shared by every
  match on it: the lanes, a coarse travel-time field from each key point (towers, camps,
  fountains, lane points; 100 u cells, walls respected), brush, ward spots and chokes.
- **Memory** (`memory.rs`, per team): enemy champions last seen (where, when, health, level,
  items), enemy casts seen (so cooldowns can be estimated), camp timers seen or caused, enemy
  wards seen, and when each threat (missile, area, wind-up) was **first** seen. It's fed after
  every tick with the team's own fog (the same `Fog` the server culls snapshots with) and the
  events its team was sent.
- **Board** (`board.rs`, per team per tick, built lazily): every unit the team can see,
  summarized (damage per second, effective health, burst ready, threat range), plus each lane's
  wave state.
- **Kit** (`kit.rs`): what each ability is for, derived from the ability data (so augments and
  new champions work without code): poke, engage, escape, peel, sustain, finisher, zone. Also
  expected damage, and **hit probability** (§4).
- **Fight model** (`fight.rs`, §5).
- **Coach** (`coach.rs`, §6), **agent** (`agent.rs`, §7), and the micro modules: `micro.rs`
  (targeting, combos, kiting), `laning.rs` (last hits, trades, waves), `jungle.rs`, `dodge.rs`,
  `aim.rs`, `shop.rs` (items, augments, anvils, spells, trinkets).

## 3. Tiers

Chosen in co-op champion select (§9). Tiers change how well a bot plays, never what it gets:
gold, experience and stats are everyone's.

| | Recruit | Veteran | Elite | Nightmare |
|---|---|---|---|---|
| Reaction (first sight → response) | 650 ± 150 ms | 450 ± 120 ms | 300 ± 80 ms | 220 ± 50 ms |
| Thinks a second (fighting / calm) | 4 / 2 | 6 / 3 | 8 / 4 | 10 / 5 |
| Commands a second (cap) | 3 | 5 | 7 | 9 |
| Leads a moving target | no | half | mostly | fully |
| Aim error (σ, across the shot) | 110 u | 70 u | 40 u | 25 u |
| Tries to dodge what it sees | 35% | 60% | 85% | 95% |
| Last-hit timing error | ±250 ms | ±130 ms | ±60 ms | ±30 ms |
| Fight read (noise on §5) | ±30% | ±15% | ±7% | ±3% |
| Tracks enemy cooldowns | – | ultimates | all | all + spells |
| Ganks, rotations, objective trades | – | basic | full | full + counter-jungle |
| Kites | – | – | yes | yes |
| Mistakes on purpose | over-chases, walks into fog | sometimes | rarely | never |

Reaction: a threat (a missile, an area, a beam's aim, an enemy stepping out of fog) gets a
response no earlier than its first sight plus the tier's reaction. Mistakes are made by
choice, not chance: a Recruit commits too long to a chase, for example, so its play reads as a
learner's, not as a broken machine's.

## 4. Hit probability

Every damaging ability gets a chance to land on a given target, worked out from the ability's
own data:
- **time to impact** `T`: wind-up plus travel (lines) or plus delay (areas);
- **escape distance** `D`: its half-width plus the target's radius;
- **how long the target can't move**: stuns, roots, knock-ups and pulls left, an attack or cast
  wind-up, a channel or a recall;
- the target's speed with its slows, and a human reaction allowance (0.25 s).

If the target can't cover `D` in the time it has to move, the hit is near certain (0.95).
Otherwise the chance falls with the distance it could cover. A line missile blocked by an enemy
minion or monster in its path won't reach the target (chance 0).

Combos fall out of this with no per-champion scripts. Ember's Sigil stuns, so Bloom's delayed
area becomes certain and goes next. Bastion's Grapple pulls a target into Tremor's nova. A
slowed target in Wren's Hail stays in it. The same estimate decides whether an ultimate is worth
using now: expected damage × chance, a kill it would secure, or the champions an area catches.

Areas aim where they catch the most: the candidate centers are the enemies' predicted
positions and the midpoints between pairs, scored by the value of who'd be inside (champions
far above minions, except in Horde).

## 5. Fight model

"If we fight here now, who wins?" Asked by the coach (team fights, ganks, dives, objectives)
and by agents (trades, all-ins, chases, retreats).

For each side of an engagement (the champions within ~1,400 u of it, plus turrets that would
shoot and the share of minion fire that would turn on champions):
- **sustained damage per second** against the other side's resistances: basic attacks
  (damage × attack speed × the resist multiplier) plus each ability's damage over its cooldown,
  times its typical hit chance;
- **burst ready now**: the damage of every ability off cooldown, times its hit chance;
- **effective health**: health and shields times `1 + resist/100`, the resist weighted by the
  other side's mix of physical and magic damage;
- **control**: hard crowd control ready, valued as the time it takes away times the damage that
  lands meanwhile.

Each side's time to kill the other is the other side's effective health, less the ready burst,
over its sustained damage per second. The outcome is `(T_them − T_us) / (T_them + T_us)`,
from −1 (we lose badly) to +1. One-on-one questions ask the same thing of one target: can I kill
it before it, its allies and its turret kill me, or before it reaches safety?

## 6. The coach

Once a second per team (staggered between teams), the coach reads:
- **the phase:** Classic lanes until about 14:00 or two outer turrets have fallen, then
  mid-game (group for objectives), then late game (group five, siege with waves); ARAM and
  Horde are always "teamfight";
- **each lane's state:** where both waves are, their strength, the structures, and who is or
  could be there (visible, or missing with a radius that grows at their speed since last seen);
- **team strength:** levels, the value of items seen, who's alive, ultimates up;
- **opportunities:** enemies dead (a window to push), an overextended enemy laner (a gank), a
  camp or buff due, an enemy jungler seen far away (counter-jungle), a structure under threat
  (defend), and humans' pings (§8).

It gives every bot a **task** (`Lane`, `Jungle`, `Gank`, `Siege`, `Defend`, `Group`,
`Objective`, `Split` or `Hold`), with a goal point and target. It assigns roles at the start
(Classic: top, jungle, mid, bottom, support), by each champion's kit and the humans' choices:
the lane a human goes to is theirs, and bots take the roles left.

**Horde:** before a wave, the team stands in a formation in front of the frontmost turret, with
melee in front and ranged behind, and shops. When the wave comes, it meets it at the hold line,
the farthest point where the fight model with the minions counted is still favorable, and falls
back toward the turret when it isn't. It focuses siege minions (they outrange turrets) and
casters, lets a tank hold the boss, and kites with the rest.

**ARAM:** the team moves as one around the wave. It pokes while poke wins, engages when the
fight model and key ultimates say so, takes relics in turn, and only dives a turret with
numbers.

## 7. The agent

Every think, a bot scores its behaviors from 0 to 1 and does the best one. The current behavior
keeps a commitment bonus and a minimum time, so it doesn't dither:

| Behavior | Rises with |
|---|---|
| Survive | danger (fight model, health, missing enemies), incoming burst; uses escapes, Barrier, potions |
| Recall | low health, gold for a power spike, a shoved wave; only when it's safe (walks to safety first) |
| Shop | being in the fountain or dead |
| Farm | its task is a lane; last hits (§7.1) |
| Trade | an enemy laner in reach, our cooldowns up and theirs down, little minion aggro to pay |
| Kill | the 1v1 model says kill, with the chase limited by turrets, fog and the tier |
| Fight | allies engaged nearby; targets and positions by archetype (§7.2) |
| Gank, Jungle, Objective, Siege, Defend, Ward, Follow | the coach's task, humans' pings and leads |

### 7.1 Laning
- **Last hits:** the bot predicts each minion's health when its own attack would land
  (wind-up, missile travel, walking into range), from the attacks already coming at it: every
  minion and turret attacking it, each hit at its own time. It attacks when that hit kills and
  nothing kills sooner. Abilities pick up the last hits it would miss (Veteran and up).
- **Wave:** neutral (last hits only), push (before a recall, a roam, a gank, or for plates and
  turrets) or hold (stand back when the enemy jungler is missing and we're ahead).
- **Trades:** when the enemy laner steps up for a last hit or has used its key ability, unless
  the minions that would turn on us cost more than the trade gains. Ranged bots step back
  after.
- **Turrets:** never stand in an enemy turret's range unless our minions are taking its shots
  and the bot isn't hitting champions. It leaves when the turret turns to it. Dives (Elite and
  up) only when the kill comes before the turret's shots finish it.

### 7.2 Fights
- **Targets:** in reach, scored by how fast we can kill them, how much damage they deal, who
  allies are already hitting (focus), and the cost of reaching them past their front line.
- **Positions:** tanks and fighters stand between the enemy and our damage. Marksmen, mages and
  enchanters keep their range from the nearest threat and kite: attack, then step between
  attacks (Elite and up). Assassins flank to the back line when their kit is up.
- **Peel:** enchanters and wardens shield the most threatened ally and crowd-control whoever
  dives it.
- **Engage:** the engager goes in when the fight model is favorable, its key ability is up and
  targets are bunched; the rest follow up in the same think.
- **Disengage:** when the fight turns, slow or stop the chasers and use escapes.

### 7.3 Utility
- **Dodging:** areas (visible from their telegraph), beams (their aim is shown) and missiles
  that enter vision. After the reaction delay, the bot steps out the shorter way that isn't into
  a wall, a turret or the enemy.
- **Blink:** to live through a fight it would otherwise die in, or to dodge a big crowd-control
  shot (Elite and up), or to finish a kill just out of reach (Elite and up).
- **Barrier:** when incoming damage would kill. **Potions:** below 65% outside the fountain.
- **Shopping:** the champion's build path, with its defensive item chosen against the enemy's
  damage mix. Classic starters include potions; supports and junglers add control wards. Stat
  Anvils once the build is complete.
- **Augments:** scored by how well each fits the champion (its stat profile and kit), rerolling
  the worst offer when none fits.
- **Wards:** the coach keeps a list of spots that matter now: river brush near a laner, jungle
  entrances when ahead, a camp before it spawns. Bots ward them on their way.

## 8. Pings

Players and bots ping (01 §14; protocol 26). There are eight kinds: **Alert** (look here),
**Danger** (back off), **On my way**, **Assist me**, **Enemy missing**, **Attack**, **Defend**
and **Group**. A ping is a point, optionally on a unit. Only the pinger's team sees it, on the
ground and the minimap, with a sound and a line in the feed. The server limits each player to
one ping every 0.5 s and five in 5 s.

**Bots ping** what the coach decides, once per decision: a jungler going to gank pings **On my
way** at the lane; a siege pings **Attack** on the structure; a buff or camp the team should take
pings **Group** at it; a laner whose opponent vanished pings **Enemy missing**; a lost fight
pings **Danger**. Recruits ping less.

**Bots listen** to their team's human pings, and trust them more than their own read:
- **Attack** on a unit or structure: the bots in reach go, unless the fight is hopeless. Each
  one that goes answers with **On my way**.
- **Group** or **On my way** at a point: bots join there.
- **Danger:** bots near the point back off for a few seconds.
- **Assist me:** the nearest one or two bots come.
- **Enemy missing:** bots treat that area as dangerous for a while.
- **Defend:** bots go defend that structure.

Bots also follow a human's lead without a ping: a human who engages near bots gets help unless
the fight is hopeless, and a bot support lets a human carry take the last hits.

## 9. Choosing a tier

Co-op champion select shows a **Bots** setting (Recruit, Veteran, Elite or Nightmare; in Horde
it's the allies' skill). Each human votes; the most-voted tier wins, and a tie goes to the
easier one. The server's `--bot-tier` (default Veteran) sets the starting choice, and the tier
for Versus fill-ins and for bots that take over dropped players.

## 10. Cost

Budget: all of a match's bot work under its simulation's cost on average, and no tick of bot
work over 1 ms. How:
- bots think in their tier's slots, staggered by player, and sooner only when something is due
  (an attack ready, a threat's reaction time up);
- one board per team per tick, built only when a bot of that team thinks;
- the coach once a second;
- travel times come from the knowledge fields, never from the pathfinder; bots repeat an order
  only when its goal really moved (each new order costs the simulation a path);
- no allocation in hot loops, and no hash maps (bots stay deterministic: the same match plays
  the same way).

`mftr-tools botmatch` reports what bots cost per tick (average and worst) next to the
simulation.

## 11. Measuring

`mftr-tools botmatch --blue TIER --red TIER --seeds N` plays N matches and reports:
- the win rate and match length;
- per role: CS a minute, gold at 10 and 15 minutes, kills, deaths and assists;
- structures, camps and buffs taken;
- skillshots hit and dodged;
- time spent dead and idle;
- bot cost.

The `legacy` tier is the old bot, kept as a baseline. The bar for this work:
- **Veteran beats legacy** in at least 80% of seeds, Classic and ARAM.
- **Each tier beats the one below** in most seeds.
- **Horde:** Veteran allies clear the campaign.
- **Reproducible:** replays still re-simulate exactly.

Dev switches: `MFTR_BOTS_OFF=name,name` turns a feature off for an A/B run (`siege`,
`engage`, `aram-recall`, `lone-gank`). Production servers don't set it.

## 12. As built (2026-10-16)

**Code:** `crates/mftr-bots`, about 6,500 lines in the modules of §2, plus `legacy.rs` (the M2
bot, kept as the `legacy` tier and the baseline). The server owns one `Bots` per match: it adds
and removes bots with players, calls `observe` after every step (with the events and the same
team fog it culls snapshots with) and `think` before the next, and relays the bots' pings to
their team. `mftr_bots::warm_up` builds a hosted map's knowledge on a thread at start, so the
first match on a map doesn't pay for it.

**What changed from the plan, and why:**
- **Commitment by tasks, not scores.** The agent's behaviors are scored as §7 says, but the
  coach's task carries most of the commitment: a defend, an engage or an answer to a ping holds
  for a set time (8 s for a defend), and the agent's own pull away from it is a fixed penalty
  per task. Pure score commitment made junglers flap between camps and lanes.
- **ARAM is its own coach mode.** The coach calls an **engage** when the fight model passes
  0.25 (Recruit 0.35) and keeps it for 3 s while it stays above 0.05; a **siege** window (two
  enemies dead, or one and three of us at the wave) takes priority. Back-liners don't chase
  past the front line, bots don't flee on low health alone, and they walk home to shop only
  with gold for an item plus a margin (ARAM's fountain doesn't heal).
- **Laning is timed, not reactive.** Last hits come from each minion's predicted health when
  our hit would land, with a wake-up just before the window rather than a think every tick.
  A swing is committed; a stale attack order gets a Stop. Supports leave last hits to any ally
  near them, and pushing happens only on purpose (a recall, a roam, a siege) or alone.
- **Dodges are decided once per cast.** A missile, area or beam inherits the first sighting of
  the wind-up it came from, so the reaction clock starts at the wind-up and doesn't restart.
  A dodge aims 45 u past the edge but counts as possible if 10 u clear is reachable in time;
  Elite and Nightmare dash or blink out of hard crowd control they can't walk out of.
- **Playing with humans.** Bots answer pings by kind and reach (Attack 9,000 u, Group, On my
  way and Defend 12,000 u, Assist the nearest within 6,000 u, Danger backs off within 1,800 u
  for 4 s), and each one that goes answers **On my way**. A human finishing a low champion
  keeps the kill: bots (Veteran and up) hold their casts and attacks on it. Supports and
  front-liners peel: their target score rises with the damage a target dealt allies in the
  last 2 s.
- **Calls are rate-limited** per team: 6 s between a team's bot calls, the same kind within
  1,500 u not repeated for 30 s, and **Enemy missing** only after 3–4 s gone.
- **Order budget.** Each tier has its commands a second (§3) as a refilling budget. An order
  that repeats what the champion is already doing isn't sent; a bot out of orders thinks again
  the moment one refills (otherwise a dodge could wait a whole think).

**Measured** (`mftr-tools botmatch`; paired seeds swap sides, lineups shuffled per seed):

| Matchup | ARAM (40 games) | Classic (20 games) |
|---|---|---|
| Veteran vs legacy | 33–4 (3 undecided) | 18–2 |
| Veteran vs Recruit | 27–4 (9) | 17–3 |
| Elite vs Veteran | 25–11 (4) | 14–6 |
| Nightmare vs Elite | 17–14 (9) | 9–11; 31–29 over 60 |
| Nightmare vs legacy | 37–3 | – |

Classic laners take 3–5 CS a minute, about 60% of the minions that die near them (supports and
junglers bring the team average to about 2.2). **Horde** (two seeds a tier, 40 minutes): the
legacy bot fell at wave 19 on one seed and barely cleared the campaign on the other; every
tier from Recruit up cleared it on both and was still holding at waves 36–40.

**Nightmare isn't clearly above Elite yet.** The two make the same decisions (only a lone
gank on a half-dead target is Nightmare's, and an A/B run of it was even: 22–18 with it, 18–22 without); faster
reactions and better aim alone don't win matches between bots. Its edge has to come from the
strategy row of §3, counter-jungling and objective trades, which aren't built.

**Dodging** (`tests/diagnose.rs dodging_by_tier`, 18 shots each, the bot walking at the
shooter): Ember's Lance (a line from 1,050 u) is dodged 0% by Recruit, 6% by Veteran, 78% by
Elite and 94% by Nightmare; Bloom (a delayed area at 850 u) 17%, 78%, 89% and 100%.

**Cost:** bots cost 6–12 µs a tick in Classic against 94–100 µs for the simulation, 3–8 µs against
19–21 µs in ARAM, and 2–4 µs against 11–13 µs in Horde (every champion a bot). The worst bot
tick in runs on an otherwise idle machine was 70–340 µs; two ARAM runs that overlapped a
compile saw 6 ms. The knowledge fields for a map are built once (warmed at server start).

**Checks:** unit tests in each module; `tests/behavior.rs` (bots answer a Group ping and say
On my way, bots call their plans, and an Elite bot leaves a low champion to a human ally);
`tests/diagnose.rs` (ignored, run by hand: lone last hits, a mid duel, teamfight calibration,
knowledge cost, a lone jungler, dodging by tier); the server's ping and tier-vote tests; the
client captures of the lobby's Bots row and a ping answered by four bots.

**Not yet:** counter-jungling (taking the enemy's camps; Nightmare's row in §3) and objective
trades; vision denial (sweeping or killing the enemy's wards); per-role reports and skillshot
counts in `botmatch` (it reports per champion; dodging is measured by the diagnostic above).
