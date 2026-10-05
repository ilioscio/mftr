# R02 — Reference capture analysis: allied champion pathing

**Source:** a local 11.1 s clip (practice tool, 1920×1080 at 60 fps, ~63 ms ping) of the own champion (Ezreal) moving around and through an **allied** Xin Zhao bot while both fight a jungle camp. The media stays in the git-ignored `fb/` folder; only measurements are recorded here.

**Question:** do champions block allied champions (DECISIONS Q12)?

## Method

Both overhead health bars were tracked across all 666 frames: the own bar is yellow, the allied champion bar is blue with a 10 px fill. Bars from the camp's blue glow and the UI were rejected by size and position. The ally bot stands still while attacking the camp for long stretches, and the own champion walks past it several times.

## Observations

| Frames | Own champion's motion | Allied champion |
|---|---|---|
| f387 → f405 | Straight diagonal at constant velocity (~3.2, 1.8 px/frame). At f399 its bar is directly below the ally's (same x, 38 px lower) | Stationary at the camp |
| f429 → f447 | Straight back along the same line, constant velocity | Stationary |
| f450 → f462 | Straight up beside the ally (45 px to the side, bars at the same height), constant velocity | Stationary |

- No deflection, slow-down or slide anywhere near the ally. With collision radii of ~35 u each (70 u contact distance), passing within roughly 50–65 u would have forced a visible path change under our D18 rules.
- The models visibly interpenetrate during the pass (f390–f398).
- Uncertainty: each champion's bar sits at a different height above its feet, so the absolute separation is known to only ±15 u or so. The total absence of any path change is the stronger evidence.

## Conclusion → D20

**Allied champions do not block each other.** MFTR adopts this: champions pass through allied champions; enemy champions and all minions (both teams) still block.

**Still unverified:** enemy champion body-blocking. It's widely known from play, but not in this clip. A short capture of walking into a stationary **enemy** champion (practice tool, enemy bot) would confirm the enemy rule and help tune the radii.

## Effect in the Netcode Lab (10 bots, 80 ms / 10 ms / 1%, 5 min)

| Scenario | Corrections > 15 u per player-min (before → after) | Mean visible correction |
|---|---|---|
| Empty arena | 4.08 → **2.56** | 0.24 → **0.13 u** |
| Minion sandbox | 5.06 → **2.36** | 0.32 → **0.19 u** |

The remaining champion-bump corrections come from enemy champions, whose next click can't be predicted. About 0.6 per minute of the total is the late-command baseline that exists even when alone.
