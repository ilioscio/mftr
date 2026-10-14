# R05 — Reference capture analysis: a heavy melee champion's kit (Wukong)

**Source:** a local clip of the reference game, *wukong abilities* (21.0 s, 1,262 frames, 1920×1080
at 60 fps): Wukong using his basic attacks, Q, E, W and R on a minion wave in the top lane. Picked
by the owner as the bar for character design and animation.

**Method:** contact sheets of the whole clip every 20 frames to find the segments, then
champion-centred crops every 2 frames (30 fps, our clip rate) for each move, and full-size crops
for the model. Frame numbers below are the clip's (60 fps); durations are in ms.

## 1. The model

- **Outline:** an inverted triangle. Shoulders ≈ 2× the head's width, with huge round gold
  pauldrons; a crown with flared side pieces and red plumes; a tail curling up behind; thick
  legs in a wide stance. The staff is longer than he is tall, with red-and-gold ends.
- **Proportions:** heroic: the head ≈ ⅙ of the height, big hands, thick limbs. Nothing on the
  outline is thinner than a few pixels at the gameplay camera except the staff, which reads by
  length and its bright ends.
- **Color blocking:** large areas of gold plate, oxblood cloth and dark leather; the bright colors
  sit on the outline (pauldrons, crown, staff ends).

## 2. The staff swing (Q, f108–f170)

| Frames | ms | What happens |
|---|---|---|
| f108–f114 | 100 | Staff raised high behind him, held two-handed, body upright |
| f116–f122 | 100 | **Cock:** the staff keeps going back, the body coils low and leans in |
| f122–f126 | 33–67 | **Strike:** 2 frames from cocked to extended, under a large orange arc smear covering the whole swing |
| f128 | — | **Impact:** a white flash on the target, then sparks; the damage number ~3 frames later |
| f130–f138 | 130 | **Hold:** the extended pose barely moves; sparks and the smear fade |
| f140–f154 | 230 | Recovery to the stance, the staff back to a diagonal across the body |

The orange crescent of each swing stays on the ground for several frames as a fading mark
(f156–f216).

## 3. The dash (E, f216–f264)

| Frames | ms | What happens |
|---|---|---|
| f216–f220 | 70 | Ready stance: wide, low, staff two-handed across the body, embers drifting |
| f222–f226 | 70 | Launch: he rises toward the camera (the silhouette grows), staff swung high behind, legs tucked |
| f228 | 17 | A gold streak along the dash |
| f230–f232 | 33 | Arrival: a deep crouch, the staff driven down; an orange-white burst at the staff's tip |
| f234–f246 | **200** | **Hold:** the crouch doesn't move while a gold light column fades |
| f248–f252 | 70 | Rises, the staff swung out and back |
| f252+ | — | Runs with the staff level across his back, both hands, perpendicular to the path |

A second dash (f402–f442) ends in a large gold, white and orange burst, then separate flashes on
each target the clones hit.

## 4. The spin (R, f764–f880)

- The brightest element is a **crisp gold ring at the area's true radius**, visible the whole time.
- Inside: translucent grey wind and red streaks along the staff's path; the body itself is a blur.
- Every minion knocked up gets its own orange-white flash (f864–f876).

## 5. Run and idle

- Run: the staff rides level across his back, both hands on it, a forward lean, a clear bob.
- Idle: wide stance, the staff a long diagonal; floating embers in his gold/orange.

## 6. Implications

- **A timing shape for blows:** slow anticipation, a cock, a 1–2 frame strike, a held impact,
  then recovery ([14 §3.1](../14-champion-art-direction.md#31-the-timing-shape-of-a-blow)). Our
  clips are 30 fps, so the strike is 1–2 of our frames.
- **Holds sell weight:** the landing crouch holds ~200 ms, the swing's extension ~130 ms.
- **Two hands on a working weapon**, and the weapon is carried on the body, not held out.
- **One palette for the model and every effect**, with the area's true edge as the brightest line.
- **Silhouette first:** the outline is built from big, character-specific shapes.
