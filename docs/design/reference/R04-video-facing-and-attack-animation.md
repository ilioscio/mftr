# R04 — Reference capture analysis: facing and basic-attack animation

**Source:** two local clips of the reference game (practice tool, a level-1 ranged bow champion, fixed camera): *movement facing* (8.8 s, 528 frames) and *attack animations* (6.8 s, 411 frames). Both were recorded at 1920×1080, 60 fps, with **~61 ms ping**. The clips live in the git-ignored `fb/` folder. **No frames or other reference-game media are committed.** Only our measurements are recorded here.

**Method:** the champion was tracked every frame by template-matching its overhead name and health bar (match score ≥ 0.9 throughout). Champion-centred crops were laid out in sheets every 1 and every 3 frames. Click times come from the move-click ring, which the client draws on the frame of the click, and attack-click times from the attack cursor. Turns and animation phases were read by eye at 1-frame (16.7 ms) resolution, so all values are ±1 frame.

## 1. Facing

| Event | Click frame | Facing starts changing | Turn complete | Reaction | Turn duration |
|---|---|---|---|---|---|
| Reversal: running up → click down-left | f238 | f245 (one in-between frame) | f246 | 7 frames, **117 ms** | **≈ 2 frames, 33 ms** for ~160° |
| Reversal: running down-left → click right | f258 | f263 | f265 | 5 frames, **83 ms** | **≈ 2–3 frames, 33–50 ms** for ~150° |
| Run → attack a target | (attack click) | — | 1 frame | — | ≤ 17 ms |

- **The rotation is fast, not instant.** A 180° turn shows one in-between pose: a turn rate of very roughly 3,000–5,000 °/s. At game zoom that reads as a snap, but it isn't a one-frame pop.
- **The reaction delay is network latency, not animation.** 83–117 ms ≈ ping + one server tick + interpolation. The reference client doesn't predict its own movement, so a click never turns the champion before the server confirms it.
- **No turn, pivot, brake or start animations.** The run cycle keeps playing through the turn and only the facing changes. Run → idle and idle → run are short blends.
- **Facing follows the move path continuously while right-click is held** (a stream of move orders, no click rings).
- **Facing on attack:** the champion faces the target on the first frame of the attack, without slowing down first.

**Implications:** facing is a *snap with a ~2-frame smoothing*. Because MFTR predicts its own movement ([03a](../03a-netcode-time-and-prediction.md)), our own champion can turn on the click frame, ~100 ms sooner than the reference. See [10 §3](../10-characters-and-animation.md#3-facing).

## 2. Basic attack (ranged, bow)

Two attacks measured frame by frame. At level 1 the attack timer is ≈ 1.5 s.

| Phase | Attack A | Attack B | Notes |
|---|---|---|---|
| Face target | ≤ 1 frame | 1 frame (f96, from a run) | No deceleration: run → planted draw instantly |
| **Windup** (draw until release) | ≥ 12 frames (already drawing at clip start) | **11 frames, ≈ 183 ms** (f96 → f107) | Planted the whole time: the "small animation lock" |
| Release | f12 | f107 | One crisp frame: the projectile leaves the bow at full speed |
| Follow-through before the player moved | **8 frames, 133 ms** (f12 → f20) | **≤ 7 frames, ≤ 117 ms** (f107 → f114) | The player cut the follow-through both times (orb-walking) |
| Projectile flight | 13 frames (~350 px) | 6 frames (short range) | |
| Impact | f26–f28 | f114–f116 | 2–3 frames of **white additive flash** on the target, a small spark burst and a flinch on the minion |
| Damage number | — | f117 (≈ 3 frames after impact) | |

- Windup ≈ 183 ms of ≈ 1.5 s ≈ **12–13% of the attack timer** at level 1.
- **The release is the punctuation.** Bow at full draw on frame N, bolt in flight on frame N+1. There's no anticipation hold after the draw and no slow launch.
- **Projectile look:** a thin, very bright head with a long tail of ~⅓ of the flight distance that fades behind it. Nothing extends ahead of or wider than the head, which already matches 05 §1 hitbox honesty.
- **Cancel window:** the follow-through is pure decoration. The player was moving again ≤ 7–8 frames after release with no lost damage. Moving *before* the release cancels the attack (02 §7 already models this).
- **Target feedback:** hovering an attackable target outlines it in red, and the cursor changes to an attack cursor.
- **Hit feedback is on the target, not the attacker:** a flash, sparks and (for minions) a flinch. The champion itself doesn't react to dealing damage.

**Implications:** see [10 §4](../10-characters-and-animation.md#4-action-timing-contract). The animation must hit its release marker exactly on the sim's fire time. The follow-through must be cancellable by any order without loss. The projectile and impact VFX carry most of the "crisp" feel.
