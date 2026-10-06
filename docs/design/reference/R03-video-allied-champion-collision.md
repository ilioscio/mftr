# R03 — Reference capture analysis: allied champion collision (supersedes R02)

**Source:** a local 3.2 s clip (practice tool, 1920×1080 at 60 fps). The own champion (Lee Sin, yellow bar) walks at a **stationary allied** Xin Zhao bot standing against a ledge, then back again. The media stays in the git-ignored `fb/` folder; only measurements are recorded here.

## Method

Both overhead bars were tracked in all 192 frames. Here, unlike R02, the ally stands perfectly still for the whole clip, and the own champion walks **on the same horizontal line** as the ally (identical bar height), straight at it. That removes R02's main uncertainty, the different bar heights per champion.

## Observations (own bar, left edge; ally bar fixed at x = 1220, y = 272)

| Frames | Own champion | Reading |
|---|---|---|
| f0 → f33 | Straight left along y = 273 (the ally's line) at constant speed (~3.7 px/frame) | Heading straight at the ally |
| f33 → f36 | **Stops** with bars ~43–48 px apart horizontally | Contact: ≈ 75–85 u center to center at this screen height (R01 camera model) |
| f39 → f51 | Moves up and slightly left | Path around the obstacle begins |
| f54 → f78 | Left along y ≈ 220, passing **above** the ally (~50 px clearance) at constant speed | Going around |
| f81 → f96 | Down-left back to the ally's line on the far side (x 1151, y 268) and stops | Arrives behind the ally |
| f114 → f180 | The same detour in reverse (up, right above the ally, down) | Repeatable |

Each change of path segment shows a 20–30 px snap of the bar, the same client-side lurch R01 §3 saw at movement starts. It happens when the client receives a new server path.

## Conclusion → D23 (supersedes D20)

**Allied champions do block each other.** A champion walking into a standing ally stops at contact and the pathfinder routes it around. MFTR returns to "every unit blocks every unit" (D11 as originally written); only ghosting effects opt out.

**Why R02 looked different:** there, the own champion passed *beside* a fighting ally, never straight at it. The ally's bar sat at a different height, so the separation was known only to ±15 u, and the true clearance may simply have been at or above contact distance. The straight-line path was weak evidence, and R02 over-read it.

**Side finding:** the measured contact distance (≈ 75–85 u center to center) suggests a champion collision radius closer to **~40 u** than our 35 u *(start)* value. That's within the measurement error; logged as an open question rather than changed.

## Effect in the Netcode Lab

Champion bumps return to the pre-D20 level: ~4 corrections > 15 u per player-minute among 10 click-spamming bots in a 3,000 u arena at 80 ms. Allied bumps are back as a correction source, so Q12's mitigation options (proxy shrinking, better crowd behavior) matter again.
