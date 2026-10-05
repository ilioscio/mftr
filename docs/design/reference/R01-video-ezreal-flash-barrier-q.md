# R01 — Reference capture analysis: Ezreal, Barrier, Flash, Mystic Shot, minions

**Source:** a local 8.4 s clip of the reference game (practice tool, Summoner's Rift, ~1:55 game time). Recorded with NVIDIA ShadowPlay at 1920×1080, 60 fps; the game itself rendered at ~730 fps with **61 ms ping**. The clip and its extracted frames live in the git-ignored `fb/` folder. **No frames or other reference-game media are committed to this repo.** Only our own measurements are recorded here.

**Method:** all 505 frames extracted. The champion was tracked via the overhead health-bar anchor, minions via their health bars, and the projectile head via color segmentation. A pinhole ground-plane camera model was fitted to motions of known length: walking at 325 u/s, Flash at 400 u, minions at 325 u/s. The fit was then cross-checked against quantities that were *not* used in fitting (Mystic Shot speed, the minimap camera box). Scripts were throwaway; the numbers below are what matter.

## 1. Camera (the most useful result)

| Quantity | Value | Confidence |
|---|---|---|
| Projection | **Perspective**, not orthographic | High: the same 325 u/s reads ~40% faster in screen px at the bottom of the screen than mid-screen |
| Pitch (below horizontal) | **≈ 55–58°** | Medium-high: vertical foreshortening at center = 0.83 ≈ sin 56° |
| Vertical FOV | ≈ 45–50° | Medium: fit error is flat above 40°; below 40° it degrades fast |
| Camera distance to look-at point | ≈ 1,750–1,950 u | Medium |
| Scale at screen center (1080p) | **0.67 ± 0.02 px/u horizontal, 0.55 px/u vertical** | High |
| Scale variation top → bottom | horizontal 0.55 → 0.80 px/u (±20%) | Medium-high |
| Visible ground at 1080p | **top edge ≈ 4,000 u wide, bottom edge ≈ 2,230 u wide, depth ≈ 2,100 u** | High: matches the minimap camera box (≈ 4,030 × 2,290 u) independently |
| Framing around screen center | **≈ 1,360 u visible upward, ≈ 750 u downward** | Medium-high |

**Implications for MFTR:**
- Use a perspective camera with ~56° pitch and ~45° vertical FOV, at a distance giving ~2,100 u of visible depth at default zoom. This matches what players' spatial intuition is calibrated to (D9 familiarity).
- The asymmetric framing (far more visible "up-screen" than "down-screen") is a feature of this camera. Ability ranges of 1,000–1,300 u fit on screen upward but not downward. Keep it, and remember the blue/red side asymmetry it creates.
- Our client must do **all ground picking via ray casts** (never assume a fixed px/u). Tooltips and range indicators are drawn in world space, so they foreshorten correctly.

## 2. Movement

| Observation | Value |
|---|---|
| Champion move speed (HUD) | 325 |
| Motion profile | Constant velocity, no acceleration or deceleration, instant facing changes. Stops are clean, with no overshoot |
| Minion walking speed | ≈ 325 u/s (four independent tracks, ±4% after perspective correction) |
| Path shape | Straight segments; facing snaps within 1–2 frames |

## 3. Input latency & own-champion handling (61 ms ping)

| Observation | Value |
|---|---|
| Move click → move-click indicator appears | same frame (client-side, immediate) |
| Move click → champion starts moving | **~6 frames ≈ 100 ms** (≈ RTT + one 30 Hz server tick) |
| First moving frame | Champion **jumps ~70–80 u forward along the new path** (≈ 0.22 s of travel). Seen at all four movement starts in the clip |
| Stop at destination | No backward correction |

**Interpretation (hypothesis, from one capture):** the reference client does not predict its own champion's movement. It waits for the server to confirm the new path, then shows the champion **fast-forwarded** along that path to an estimate of its current server position. That's why there's a visible lurch at the start of each move and none at the end, since the path endpoint is known.

**Implications for MFTR:** our design (03a §4) predicts own movement immediately, so click → motion is **~0 ms with no lurch**. That should feel strictly snappier than what players are used to. It also means our own champion is drawn ~one RTT "ahead" of where reference-game players are used to seeing it, which is exactly what makes our dodge timing honest. Worth asking blind testers whether anything feels "too fast" or "floaty".

**Follow-up capture wanted:** the same click test at a different ping (e.g. a far-away server), to confirm the lurch scales with latency.

## 4. Mystic Shot (reference line skillshot)

| Observation | Value | Official |
|---|---|---|
| Measured speed | 1,950–2,230 u/s, depending on assumed missile height (0–150 u above ground) | 2,000 u/s |
| Cast to projectile visible and moving | ~2 frames after the cast glow (~33 ms) | 0.25 s cast time (most of the windup is the arm animation before the glow) |
| **Visual core width** | **≈ 35 px ≈ 55 u** | Hitbox width 120 u (radius 60), plus target gameplay radius ~65 |
| Hit → damage number | Same frame as the visible impact | — |

**The reference VFX is roughly half as wide as the hitbox.** The real hit corridor for a champion's center is 60 + 65 = 125 u either side of the missile's center line, but the visible core is only ~27 u either side. Experienced players have learned "it's bigger than it looks".

**Implications for MFTR:** our hitbox-honesty rule ([05 §1](../05-art-and-assets.md#1-readability-rules-non-negotiable)) draws the projectile's core at its full gameplay width. With the same tuning numbers, **our skillshots will look about twice as wide** as players expect, while behaving the same. Options:
1. **Honest core** (current rule): the core is full width with a soft falloff at the edge. Truthful, but looks chunky.
2. **Honest edge, slim core:** a bright slim core (reference-like look) plus a faint but always-visible full-width "edge sheath" that marks the real hitbox. Familiar look *and* honest.
3. Copy the reference look (slim only). This rejects Pillar 1, so no.

**Recommendation: option 2**, validated in the M1 blind test.

**Reaction-budget sanity check:** Mystic Shot is width 120, 0.25 s cast, 2,000 u/s, 1,200 range. Under the width-based rule ([03a §8](../03a-netcode-time-and-prediction.md#8-reaction-budget-what-the-player-actually-gets)), `t_move = (60 + 65)/335 ≈ 0.37 s`, so `T_needed ≈ 0.79 s`. Its reaction time at max range is `0.25 + 1200/2000 = 0.85 s`, which **passes as a poke (100% range) with very little margin**. That matches how this ability feels in practice ("dodgeable at max range if you're paying attention"). It's good evidence the rule's constants are well calibrated.

## 5. Utility spells

| Spell | Observation |
|---|---|
| Flash | Instant, between two consecutive frames (≤ 16 ms visual). ~400 u. The origin leaves a gold sparkle that persists ~1.5 s, so the "where did they flash from" information is readable. The arrival has a bright burst and a vertical light pillar (~0.3 s). Cooldown readout consistent with 300 s |
| Barrier | Translucent bubble around the champion, visible ≈ 2.1–2.3 s in this clip. The shield shows as a white segment appended to the health bar, compressing the yellow fill. Cooldown readout consistent with 180 s |

## 6. UI metrics at 1080p (default HUD scale)

| Element | Size / position |
|---|---|
| Own overhead bar | HP fill **104 × 11 px**, level box on the left, resource bar ~4 px below, name label above. Anchored **~125 px above the champion's feet** |
| Minion bar | **62 × 6 px** incl. 1 px dark border, ~4 px fill. ~35 px above the minion's feet |
| Main HUD | Bottom center-left, **x ≈ 520–1330 (810 px) × ~105 px**. Ability icons ≈ 45 px, utility spells ≈ 36 px, item slots ≈ 32 px, stats panel left of the portrait |
| Minimap | Bottom-right, **≈ 250 px square** (map area ≈ 240 px ≈ 62 u/px). Camera box drawn as a rectangle ≈ the top-edge width × visible depth |
| Move-click indicator | Ring ≈ 120 px (~180 u) wide, collapsing to four arrows over ~6 frames (~100 ms) |
| Top-right | K/D/A, CS, clock; FPS and ping readout below |

**Implications for MFTR:** treat these as **familiarity targets**, not things to copy. Our HUD can differ in style but should keep similar information density, bar sizes and minimap scale, so the screen "reads" the same at a glance.

## 7. Capture-pipeline note

Per-frame projectile steps varied ±30% (e.g. 1,100–2,550 u/s frame to frame) while 8–12 frame averages were stable. That's ShadowPlay sampling a 730 fps game at irregular intervals. Average over ≥ 8 frames when measuring from captures, and record future reference clips with **a frame-rate cap equal to the capture rate** (e.g. 60 fps cap with 60 fps recording) for cleaner per-frame data.

## 8. Wanted next captures

1. A move-click test at high ping (confirms §3).
2. A minion wave clumping and blocking a champion (D11 tuning: collision radii, slide behavior).
3. A slow, wide skillshot (a hook) dodged at the last moment, to calibrate `t_human`.
4. A delayed ground AoE (e.g. a circle that detonates), to measure telegraph timing.
5. Camera zoomed in and out, to bound the zoom range.
