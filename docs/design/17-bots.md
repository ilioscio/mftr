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
- **Shopping:** the champion's build path, with its boots and defensive legendary chosen
  against the enemy's damage mix ([18](18-items.md) §9). A jungler starts on a hatchling, a
  support on the Pilgrim's Ledger and takes its lantern when the quest is done; a starter is
  sold for room late. Classic starters include potions; supports and junglers add control
  wards. Stat Anvils once the build is complete.
- **Item actives:** by rule: Encase about to die, Petrify through a burst, Embrace when allies
  are hurt, Absolve on a held or dying ally, Reaping Arc, Bloodletting and Quake with enemies in
  reach, Plunge and Surge onto a champion worth chasing, Fleet to chase or flee.
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

Dev switches, for A/B runs (production servers set neither):
- `MFTR_BOTS_OFF=name,name` turns a feature off: `siege`, `engage`, `aram-recall`,
  `lone-gank`, and Nightmare's `trade`, `base`, `invade` and `trapped` (§12).
- `MFTR_BOTS_TUNE=tier.field=value,...` overrides a tier's number (any field of `Params`, e.g.
  `nightmare.think_fight=4`), to find which ones matter.
- `MFTR_BOTS_PERSONA=kind` makes every bot one persona (§13); `MFTR_BOTS_OFF=personas` makes
  them all the even-keeled baseline, and `MFTR_BOTS_OFF=persona-builds` keeps their play but
  puts every persona on the balanced build (§14).

On a one-lane map `botmatch` also reports how the teams stood, in the units of the reference
footage (R07): the gap between the teams' median champions and where the front was, the time
the front spent between the outer turrets and each team behind its own, and kills a minute.
Per persona it reports K/D/A, damage a minute, time dead and the relics taken (and at what
health).

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
- **Defense decides Classic.** Matches between good bots end in base races, so defending is
  where tiers separate. Every structure that can be hurt is a candidate (once a Gatehouse is
  down, the base turrets can be hurt alongside each lane's own). The most urgent is served
  first, each bot defends one at most, and a laner isn't pulled off its own turret to another
  lane's. Inner structures (tier 3 and in) draw defenders from anywhere on the map: Veteran
  sends one more than the attackers, Elite two more.
- **Nightmare's edges** are decisions, not reflexes. An experiment that set each of its
  numbers back to Elite's in turn (reaction, think rate, aim, tracking, ability use) moved
  nothing: between bots, faster hands don't win matches. So the top tier gets:
  - **Base defense:** a Gatehouse or base structure with an enemy at it brings every bot
    home, except one about to finish the enemy's base.
  - **Objective trades:** an outer turret (tier 1–2) that two or more enemies are taking and
    the fight model says can't be held isn't fed. The bots that would come take the next
    structure in a lane with no enemy seen or likely near instead.
  - **Trapped targets:** it tracks spells as well as abilities, so it knows when a laner's
    dashes and Blink are all down. Then a smaller edge in the fight model (0.15 rather than
    0.35) is enough to go all in.
  - **Counter-jungling:** its jungler takes an enemy camp when the enemy jungler was seen
    within 20 s too far away to come back in time (or is dead), and no other enemy seen in
    the last 8 s could get there first. By honest information that's rare: the enemy jungler
    is seldom in sight.
  - **Lone ganks** on a half-dead target.

**Measured** (`mftr-tools botmatch`; paired seeds swap sides, lineups shuffled per seed):

| Matchup | ARAM | Classic |
|---|---|---|
| Veteran vs legacy | 33–4 (3 undecided) | 37–3 |
| Nightmare vs legacy | 37–3 | 39–1 |
| Veteran vs Recruit | 27–4 (9) | 37–3 |
| Elite vs Veteran | 25–11 (4) | 30–10; 59–21 over 80 |
| Nightmare vs Elite | 19–15 (6) | 23–17; 50–30 over 80 |

Each cell is 40 games (seeds 1–20, sides swapped); the 80-game totals add seeds 201–220.
Classic Nightmare vs Elite with all of Nightmare's own rules off was 23–17 on seeds 201–220
(27–13 with them).

Classic laners take 3–5 CS a minute, about 60% of the minions that die near them (supports and
junglers bring the team average to about 2.2). **Horde** (two seeds a tier, 40 minutes): the
legacy bot fell at wave 19 on one seed and barely cleared the campaign on the other; every
tier from Recruit up cleared it on both and was still holding at waves 36–40.

**ARAM** separates Nightmare from Elite least (19–15): it's one lane of teamfights, where
the two play alike. Its macro edges are Classic's.

**Dodging** (`tests/diagnose.rs dodging_by_tier`, 18 shots each, the bot walking at the
shooter): Ember's Lance (a line from 1,050 u) is dodged 0% by Recruit, 6% by Veteran, 78% by
Elite and 94% by Nightmare; Bloom (a delayed area at 850 u) 17%, 78%, 89% and 100%.

**Cost:** bots cost 6–14 µs a tick in Classic against about 100 µs for the simulation,
3–8 µs against about 19 µs in ARAM, and 2–4 µs against 11–14 µs in Horde (every champion a
bot). The worst bot tick on an otherwise idle machine was 61–443 µs; runs sharing the machine
with others see the scheduler's pauses as worst ticks of many milliseconds. The knowledge
fields for a map are built once (warmed at server start).

**Checks:** unit tests in each module (`memory.rs`: when an enemy could reach a point);
`tests/behavior.rs` (bots answer a Group ping and say
On my way, bots call their plans, and an Elite bot leaves a low champion to a human ally);
`tests/diagnose.rs` (ignored, run by hand: lone last hits, a mid duel, teamfight calibration,
knowledge cost, a lone jungler, dodging by tier); the server's ping and tier-vote tests; the
client captures of the lobby's Bots row and a ping answered by four bots.

`botmatch` also reports, per tier and game, the structures taken and the monsters killed on
the enemy's side.

**Not yet:** vision denial (sweeping or killing the enemy's wards), which would also feed
counter-jungling the sightings it needs; ARAM edges for Nightmare; per-role reports and
skillshot counts in `botmatch` (it reports per champion; dodging is measured by the diagnostic
above).

## 13. Personas, and play like people's (2026-10-18)

The tiers made bots strong, and the owner found them no fun: risk-averse and inhuman, huddled
under their turrets. One recorded ARAM: Mayhem match of the reference game, between players of
about plat to low diamond rank, set the bar ([R07](reference/R07-video-aram-match.md)).

**What people do that the bots didn't:**
- **They fight in the middle:** the median gap between the teams was 0.19 of the
  center-to-Base line (about 900 u), and each team was behind its own outer turret 25% of the
  time.
- **They fight more:** 6.6 kills a minute, and a 19-minute match.
- **They turn a won fight into a tower.**
- **They don't walk back to shop.**
- **They dive and chase.**
- **They leave a relic to whoever needs it.**
- **They play differently from each other.**

The map was part of it: The Bridge's outer turrets stood so close to the center that 450 u of
open ground lay between their reaches, and it had no brush at all (R07 §1).

**Personas** (`persona.rs`): each bot rolls one when it first plays a champion, from the
match's seed, and keeps it all game. A tier is how well a bot plays; a persona is how it likes
to play. Personas change choices, never hands: reaction, aim and reads stay the tier's.

| Persona | Nerve | Greed | Vision | Flair | Plays like |
|---|---|---|---|---|---|
| Daredevil | 0.8 | 0.2 | 0.2 | 0.5 | starts fights, dives, chases |
| Carry | 0.25 | 0.8 | 0.2 | 0.3 | farms, finishes, takes the relic |
| Guardian | 0.05 | −0.8 | 0.9 | 0.2 | peels, leaves the relic, wards |
| Cautious | −0.6 | 0 | 0.6 | 0.1 | waits for an edge, backs off early |
| Trickster | 0.4 | 0.1 | 0.4 | 0.9 | waits in brush, comes from it |
| Steady | 0.1 | 0 | 0.4 | 0.3 | the even-keeled team player |

- **The roll:**
  - Weights 18/20/14/12/15/21%, leaning by class. An enchanter is three times as likely to
    be a guardian.
  - A marksman, mage or assassin leans carry. A tank or bruiser leans daredevil.
  - Each trait is then jittered by up to ±0.15, so no two daredevils are alike.
- **Nerve** (risk appetite) moves:
  - the bar to fight (−0.22 × nerve) and to fight beside a teammate;
  - when a losing fight is left, and when a laner gives ground;
  - how long a chase lasts (overchase × (1 + 0.6 × nerve));
  - the health it flees at where the fountain heals;
  - how sure a turret dive must be (0.45 − 0.2 × nerve). Veteran daredevils dive too.
  - In the coach, the team's average nerve lowers the bar to engage (−0.12 × nerve). A bold
    ARAM team pushes on a single kill.
- **Greed** (selfish or selfless) moves:
  - the health a relic is taken at (below 65% + 20% × greed);
  - target choice: a selfish persona weighs hurt targets and kills more. A selfless one peels
    whatever its class.
  - **Charity:** unless it's selfish (or below 30%), a bot leaves a relic to a teammate at least
    20 points lower who is about as close.
  - Bots still leave a low champion to a human who's finishing it (§12), whatever their
    persona.
- **Vision:** Classic wards. Above 0.6 it carries a control ward, above 0.85 two, from Veteran
  up; supports and junglers keep theirs from Elite.
- **Flair:** in ARAM, while the team waits, a persona with flair stands in brush near the spot,
  not past the front. It comes out of the brush when someone walks by.

**For every persona, after the footage:**
- **A step back, not the turret:** on maps whose fountain doesn't heal, a bot losing a fight
  backs off 700 u while it has health (above 30%, less with nerve) rather than to its turret.
- **Waiting in the open:** between waves, the ARAM coach holds the team short of the center
  instead of at its outer turret.
- **Towers after a won fight** (ARAM): on a siege, a bot in reach of the structure hits it while our
  wave takes its shots (two minions in its range, or no one to defend it), unless it's being
  hit hard. Before, it traded with whoever came to defend and the towers stood: one match had
  204 deaths and 2 structures in 40 minutes.
- **Shopping when dead:** in ARAM, a bot walks back to shop only with 3,000 gold to spend
  (1,800 before). It shops when it dies, as players do.

**The Bridge, re-laid** (12 §1, R07 §1):
- **Turrets:** the outer turrets stand a third of the way from the center to the Base, 3,000 u
  apart (they were 2,000). All four lane structures move to match (`ARAM_STRUCTURE_SHARES`,
  also the default for new ARAM maps).
- **Relics:** two to a side, one in front of the outer turret and one between it and the inner.
- **Brush:** six patches to a side, in alcoves along both walls.
- **Rocks:** two to a side in the open middle.
- **Brush tracing fixed:** the outline tracer dropped every point of a smooth outline with
  nothing pinned, so lone brush patches vanished. Crossroads gets back the 11 of its 18 it had
  lost.
- **Protocol 27:** clients build the map themselves.

**Measured:**
- **Spacing:** ARAM, 12 seeds a tier, the same tier on both sides; the reference is R07's
  match.

  | | Before | New map | New map and personas | Reference |
  |---|---|---|---|---|
  | Kills a minute | 3.8–4.1 | 5.4–5.9 | 5.7–6.1 | 6.6 |
  | Behind own outer turret | 37–40% | 28–34% | 29–33% | 25% |
  | Median gap between the teams | 0.28–0.31 | 0.23–0.27 | 0.23–0.27 | 0.19 |
  | Front, 10th/50th/90th | −0.27/0.00/0.43 | −0.30/0.04/0.42 | −0.31/0.04/0.43 | −0.42/0.12/0.50 |
  | Match length | 29–32 min | 30–32 min | 27–28 min | 19 min |

  The teams' gap at the 90th percentile stays 0.6–0.8 against the reference's 0.38. Teams
  walking out at the start, and the ones respawning walking back, make most of it.
- **Personas,** ARAM, Elite against Elite over 20 seeds (200 bots):

  | Persona | Kills | Deaths | Assists | Damage/min | Relics/10 min | Health at a relic |
  |---|---|---|---|---|---|---|
  | Carry | 19.4 | 16.8 | 32.9 | 1,472 | 4.8 | 52% |
  | Steady | 17.0 | 15.8 | 33.3 | 1,354 | 3.2 | 43% |
  | Cautious | 17.8 | 13.7 | 33.6 | 1,394 | 4.0 | 42% |
  | Daredevil | 16.0 | 17.7 | 30.8 | 1,166 | 3.3 | 47% |
  | Guardian | 13.7 | 16.8 | 35.4 | 944 | 3.0 | 38% |
  | Trickster | 12.4 | 19.6 | 33.6 | 952 | 3.7 | 50% |

  - Carries take the kills and the relics.
  - Guardians take the assists and leave relics for the lowest.
  - The cautious die least.
  - Daredevils and tricksters die most: their risks don't always pay, as people's don't.
  - Elite against Elite, kills a minute were 5.9 with personas off and on; match length 26:08
    off, 28:09 on. Personas change how bots play more than how much they fight.
- **The ladder** (40 games a cell, seeds 1–20, sides swapped):

  | Matchup | ARAM | Classic |
  |---|---|---|
  | Veteran vs legacy | 34–5 | 38–2 |
  | Veteran vs Recruit | 36–2 | 37–3 |
  | Elite vs Veteran | 24–9 | 24–16; 51–29 over 80 |
  | Nightmare vs Elite | 16–14 (10 undecided) | 22–18; 48–32 over 80 |

  - The 80-game totals add seeds 201–220. Before: Elite over Veteran 59–21, Nightmare over Elite
    50–30.
  - Elite's Classic edge narrowed, with personas off too (24–16 on seeds 1–20). That points to
    Crossroads' recovered brush, not the personas.
  - Classic's Veteran rows were played before the siege rule was limited to ARAM.
  - Every tier still clears the Horde campaign on both seeds (waves 29–40 by the 40-minute
    cap).

**Not yet:**
- **Brush for every persona:** only flair uses it. Brush checks before walking past it, and
  hiding from a chase, would make everyone read more like a player.
- **Baits and flanks:** flair waits in brush but doesn't bait, or come at a fight from the
  side.
- **More footage:** one match is one lobby. The owner has more recorded, which would show
  which of R07's numbers are ARAM's and which were these ten players'.

## 14. Builds by persona (2026-10-18)

The item overhaul ([18](18-items.md)) gave each class 3–7 legendaries that are real choices,
balanced with bots on one path per champion. A player's build says as much about them as their
play, so each persona now builds its own way (`builds.rs`).

**The rule:**
- **Steady** keeps the champion's balanced path (`items::build_path`) exactly.
- **The other personas** put their class's legendaries in their own order and keep the path's
  count (at least three). The champion's own legendaries fill in after. Boots, starter, lantern
  and hatchling are the persona's too.
- **Assembly:** the starter, Boots, the first legendary from its components up, the boots
  upgrade (without a second Boots), then the rest.
- **Still answering the enemy:** boots and the resist legendary still answer the enemy's
  damage. A persona that wants defensive boots gets Ironshod or Stalwart by it, and Wardstone
  and Briarhide swap past the same lines as before.

| Class | Daredevil | Carry | Guardian | Cautious | Trickster |
|---|---|---|---|---|---|
| Mage | Windstride, Inferno, Grimoire; Swift; Tally Charm | Grimoire, Hollow Scepter, Inferno; Spellstriders; Tally Charm | Blightcaller, Amber, Inferno; Sage | Amber, Inferno, Windstride; defensive | Windstride, Hollow Scepter, Grimoire; Swift |
| Marksman | Gale Saber, Crimson Fang, Starsteel | Starsteel, Fanfire, Titanbane | Titanbane, Crimson Fang | Crimson Fang, Gale Saber, Starsteel; defensive | Gale Saber, Fanfire, Starsteel; Swift |
| Assassin | Shrikeblade, Duskfang, Veilrunner; Swift | Duskfang, Shrikeblade, Veilrunner | Shrikeblade, Lionheart, Duskfang; defensive | Veilrunner, Duskfang, Lionheart; defensive | Veilrunner, Duskfang, Shrikeblade; Swift |
| Fighter | Threefold, Bloodletter's, Lionheart; Swift | Reaping Cleaver, Sundering Axe, Threefold | Lionheart, Kinship Locket, Bloodletter's; defensive | Lionheart, Bloodletter's, Sundering; defensive | Threefold, Reaping Cleaver, Sundering; Swift |
| Tank | Stonewall, Furnace, Stoneskin; defensive | Furnace, Briarhide, Heartstone; defensive | Kinship Locket, Stonewall, Wardstone; Sage | Heartstone, Stoneskin, Wardstone; defensive | Stonewall, Furnace, Briarhide; Swift |
| Enchanter | Kinship Locket, Windstride, Inferno; Swift | Inferno, Grimoire, Chalice | Chalice, Kinship Locket, Wardstone | Kinship Locket, Chalice, Amber; defensive | Windstride, Chalice, Inferno; Swift |

(Boots the champion's own unless named; "defensive" is Ironshod or Stalwart by the enemy's
damage.)

- **Why these:**
  - The daredevil buys what gets it in and keeps it there: dashes and engage actives, sticking
    power, sustain mid-fight.
  - The carry buys the amplifiers first. A carry or daredevil mage snowballs on the Tally Charm.
  - The guardian buys what helps the team: the Locket and the Chalice where the class can
    carry them, anti-heal, tank-shredding, Quake's peel.
  - The cautious buys what keeps it alive: Encase, Lionheart, Crimson Fang's shield,
    Veilrunner's Fleet, Heartstone.
  - The trickster buys burst and mobility.
- **Support lanterns:** a daredevil or trickster takes the Hunter's Lantern unless it's an
  enchanter. A guardian or cautious front-liner takes the Bulwark. Others keep their class's.
- **Jungle hatchlings:** a daredevil raises a Flintling (Claim on champions), a trickster a
  Windling (speed to roam), a guardian or cautious bot a Barkling (its shield). Others keep
  their class's.
- **Actives follow the persona:**
  - Encase and Petrify come sooner for the timid (Encase below 22% − 8 points × nerve).
  - Plunge goes onto healthier targets and on a thinner edge for the bold.
  - Surge and Fleet's chase go in on worse odds; Fleet's escape comes sooner for the timid.
  - Quake opens a fight for a daredevil (one champion in reach, the fight even or better).
  - Embrace shields smaller bursts for the selfless.

**Measured** (Elite against Elite, `botmatch --items`; 120 ARAM games and 30 Classic, persona
builds on and then off with `MFTR_BOTS_OFF=persona-builds`, same seeds):

- **The match doesn't move:**
  - ARAM ran 19:44 against 19:38 and 6.4 kills a minute against 6.5.
  - Classic's quest stages (Astrolabe 12:37 against 12:51) and hatchlings (8:20 against 8:37)
    are where they were.
- **Personas stay level:** each persona's ARAM win rate is within 4 points of the same persona
  on the balanced builds, about the method's resolution.

  | Persona | Balanced builds | Persona builds |
  |---|---|---|
  | Daredevil | 51% | 55% |
  | Carry | 51% | 53% |
  | Steady | 51% | 51% |
  | Cautious | 47% | 46% |
  | Guardian | 50% | 46% |
  | Trickster | 48% | 45% |

  Thirty Classic games are too few to read a persona from.
- **The whole catalog plays:** six items the balanced paths never bought are now built
  (Threefold Brand, Reaping Cleaver, Blightcaller Tome, Heartstone, Stoneskin Ward, the Tally
  Charm: 43–131 holders each in ARAM). In ARAM their holders won 43–56%, inside their classes'
  ranges.
- **Within a class, ARAM:**

  | Class | Holders' win rates | Lowest |
  |---|---|---|
  | Fighters | 44–53% | |
  | Mages | 44–55% | Amber Reliquary |
  | Marksmen | 66–76% | Gale Saber |
  | Assassins | 28–34% | |
  | Tanks | 33–61% | Stonewall Aegis (33%, taken first by bold tanks), Briarhide Plate (35%); Wardstone Mantle at 61% |

  - Assassins sit low because of Shade, as 18 §12 found.
  - The tanks' spread was there on the balanced builds too (37–61%).
  - A class's items now also differ by who buys them, so a low one may be its buyers' play as
    much as the item.
- **Sides:** red won 56–59% of those 120 matches, but over the 1,600 below the sides were even
  (blue 46–52%): those seeds, not the map.

**The extended pass** (`botmatch --items`, now with a line per champion). It ran 400 ARAM
Elite mirrors at a time, on persona builds and on shuffled builds (each bot's legendaries drawn
from its class, so an item's results aren't its champion's).

- **Items within a class:**
  - Fighters (43–52%), assassins (39–41%), marksmen (61–71%) and mages (49–57%) were each
    level.
  - Tanks spread 12 points: Stonewall Aegis 39% against Briarhide's 51%.
  - Kinship Locket was 43% against Purifying Chalice's 52%.
  - Both were buffed (18 §12). After the buffs the tanks' legendaries sit within 7 points of
    each other, and the Locket reads 63–65% against the Chalice's 54–57%. Those are ~125
    holders each (only Lumen builds support items on shuffled builds), so ±4.5.
- **Champions were the real outlier:**

  | Champion | Before | After |
  |---|---|---|
  | Vesper | 87% | 71–77% |
  | Quill, Ember, Lumen | 52–54% | 55–60% |
  | Wren | 48% | 54–56% |
  | Rook | 48% | 46–48% |
  | Marrow | 41% | 38–39% |
  | Bastion | 39% | 36% |
  | Cairn | 39% | 31–37% |
  | Shade | 36% | 32–37% |

  - **Vesper** dealt 3,000 damage a minute and died 5 times a match (the next champion: 2,100
    and 7.5). She now has a 525 attack range (575: the longest), 61 base AD (66), Longshot at
    0.9 AD (1.1), Tumble every 9 s (7 s), and Snare Net 2 s faster a rank instead of 3.
  - What didn't help, and was left out:
    - Longshot's cooldown, Snare Net's root and cooldown and Shrapnel's ratio (no change in
      400 games);
    - +60 health and +4 resists on Bastion, Cairn and Shade (+2 points);
    - a calmer daredevil.
- **What's left is how bots play the front line, not numbers:**
  - Back-line ranged champions win.
  - Front-liners lose, whatever their stats.
  - The daredevil, who dives, stays at 44–46%.
  - Bot front-liners walk into poke and die reaching their targets. The fix is in engage
    timing, peel and how melee picks its moment (§7.2), a piece of its own.


## 15. The front line (2026-10-18)

After §14, back-line ranged champions won ARAM and front-liners lost, whatever their stats:
Bastion 36%, Cairn 31–37%, Shade 32–37%, Marrow 38%, and the diving daredevil 44–46%. Each
idea below was tried for this pass and measured over 400 ARAM Elite mirrors (seeds 2001–2400).

| Tried | What it does | Front-liners |
|---|---|---|
| Fight reads by reach | A fighter's damage counts by how much of a fight it spends in reach, so a melee fighter that must walk in reads weaker | +1–2 points |
| Melee patience | Outside a called engage, melee waits for a clearer edge and leaves a bad fight sooner | none |
| Divers go for carries | Assassins, bruisers, skirmishers and daredevils weigh back-line carries ×1.6 and reach 400 u farther for them | none |
| Fewer dodges | The same matches at Veteran (78% of areas dodged) and Recruit (17%) | tanks 31–36% at every tier |
| No collision with minions | Champions walk through minions (an experiment only) | none |
| Shade's veil | 01 §8. Bots veil when focused (15% of their health in 1.5 s, more for the bold) or below 35% with enemies near | Shade 34–36% |

- **Where front-liners die:** a temporary death log (1,047 deaths over 8 matches) showed them
  dying in team fights, not alone. About 2.4 allies were within 1,500 u, against 3.2
  enemies.
- **Only champion numbers have moved win rates** (Vesper's in §14). The front line's deficit is
  in what the kits contribute to a fight between bots, not in how bots decide. Bots are
  equally good shots on both sides, so a ranged kit's safe damage outweighs a tank's soak and
  crowd control.
- **The veil works as built:** in one match Shade veiled 33 times and was hidden from the
  enemy on 95% of veiled ticks (turrets saw the rest). It keeps Shade alive but adds no
  damage.
- **Kept:** divers going for carries, because that's how players dive and it makes fights
  livelier, and bots using the veil.
- **Left out:** the reach-weighted reads and melee patience. They made melee bots more
  passive for no gain, against what 17 §13 is for.

**Next:** front-line kits. Tanks' crowd control should hold fights in place (longer roots and
knock-ups, a team engage) and Shade needs damage, measured the same way. Players may value
the tanks more than bots do: their crowd control lands on players who dodge less than Elite
bots, and their peel saves real carries.
