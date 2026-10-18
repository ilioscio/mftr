# R07 — Reference capture analysis: a whole ARAM: Mayhem match

**Source:** the owner's local recording of one ARAM: Mayhem match in the reference game, *real
aram 1* (19:04, 1920×1080 at 30 fps), played at the gameplay camera by the owner with nine other
players of roughly plat to low diamond rank. Picked as the bar for how people play ARAM, to set
against our bots (17 §13), and for the shape of the reference ARAM map (its new single-lane map,
not The Bridge's old model).

**Method:**
- **Minimap, once a second (1,144 frames):** the minimap's 256 px square, read by color.
  - Champion icons carry a team ring (blue or red). Each team's position is the median of its
    ring pixels' places along the line from blue's nexus to red's, from −1 to 1 with the center
    at 0, the structure icons masked out.
  - Health relics are a green cross; their four spots and when each was up come from those
    pixels.
- **Gameplay, every 5 s:** 2×2 contact sheets at half resolution (58 sheets), plus one-second
  strips around moments of interest.
- **Map proportions:** enlarged minimap stills, measured in the same −1 to 1 units.

Player names stay out of this note.

## 1. The map

| On the line from the center (0) to a nexus (1) | Reference | The Bridge before | The Bridge now |
|---|---|---|---|
| Outer turret | 0.32 | 0.21 | 0.32 |
| Next turret (theirs: the inhibitor's turret) | 0.56 | 0.44 | 0.53 |
| Gatehouse turret (none in the reference) | – | 0.69 | 0.66 |
| Inhibitor / Gatehouse | 0.73 | 0.80 | 0.77 |
| Nexus turrets / base turrets | 0.90 | 0.92 | 0.92 |
| Health relics | 0.20 and 0.45, both a little to one side of the lane | 0.08 (all four by the center) | 0.20 and 0.45 |

- **Turrets:** the outer turrets stand a third of the way from the center to the nexus. With
  turrets that reach 775 u, the old Bridge left 450 u of open ground between the two outer
  turrets' reach. Now there are 1,500 u, and that is where fights happen.
- **Relics:** two on each side. One stands in front of the outer turret, the other between it
  and the next one. The nearer one is fought over; the farther one is where a hurt player
  backs off to.
- **Lane width:** about 0.3 of the center-to-nexus line, much like The Bridge's 1,560 u.
- **Brush:** the lane has patches of brush along both walls, many sitting in shallow alcoves.
  They're used for waiting, for dodging out of sight and for coming out at someone.
- **Our map was missing all of it.** The Bridge's two grass strokes were traced away: a smooth
  outline with no pinned corner lost every point when it was simplified. Crossroads lost 11 of
  its 18 patches the same way.

## 2. How the match went

- **Length and pace:**
  - 19 minutes and 123 kills (72 to 51): 6.6 kills a minute.
  - The first blood came at 1:08.
  - The winning side broke the nexus at 19:04.
- **Where the teams stand:**
  - The gap between the two teams' medians was 0.04 / 0.19 / 0.38 (10th, 50th and 90th
    percentiles). The median gap is about 900 u, within poke range. The teams were in
    contact three quarters of the time.
  - The front (the point between them) was at −0.42 / 0.12 / 0.50. It was between the outer
    turrets 65% of the time.
  - Each team's median was behind its own outer turret 16% of the time for the winners and 34%
    for the losers: 25% on average.
- **Fights, not sieges:** most of the match is skirmishing over the middle with the wave. A won
  fight is followed by a push: at 7:40–8:30 and 11:20–12:50 the winning team stood at and past
  the losing team's outer turret. The losing team pushed back twice, once into the winners' base
  (13:50 and 15:40–18:10), before the last push ended it.
- **Risk:**
  - Players dive turrets for kills, and some die doing it: the recorder dove the outer turret
    at 1:40 and died.
  - They chase into the enemy half. They stay in fights at low health.
  - They don't walk back to shop; they shop when they die.
- **Relics:** the one in front of each outer turret is taken within seconds of appearing, mid
  fight. The farther ones sometimes stay up for a minute or more while the fight is elsewhere:
  players leave them for whoever needs them, rather than taking them on the way past.
- **Variety:**
  - Ten players play ten ways. Some start every fight.
  - Some sit at the back.
  - The supports end with more assists than kills, and the carry ends 19/8/24.
  - Some take relics they barely need.
- **Chat:** people talk ("why", "we were cool", "gg"). Bots don't need to, but the pings carry
  some of it (17 §8).

## 3. What it set for our bots (17 §13)

- **Personas:** ten kinds of player, not ten copies.
- **Fights in the middle:** hold between the outer turrets between waves, and back off a few
  steps when losing, rather than back to the turret.
- **Take towers after a won fight:** hit the structure while the wave takes its shots.
- **No walking back to shop:** in ARAM, only for a big purchase.
- **Measured against it:** `botmatch` reports the same spacing numbers for one-lane maps (17 §11).

**More footage:** the owner has more recorded matches. One match is one set of ten players, so
a second and a third would show which of these numbers are ARAM's and which were this lobby's.
