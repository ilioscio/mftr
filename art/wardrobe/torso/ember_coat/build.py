# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shadow jerkin (22 §4, M1; Shade's): close-fitting indigo cloth from a round neck to a hem
over the hips, long sleeves to the wrist, its edges bound in black. The veteran's gambeson's
pattern, cut lean and short, and thin rather than padded.

    blender -b --python art/wardrobe/torso/shadow_jerkin/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402

CHARCOAL = (0.11, 0.10, 0.10)
EMBER = (0.55, 0.10, 0.04)


def body(neck_z):
    """The body panel, front or back (the character's right on -x): fitted, to a hem over the hips."""
    return [
        ("hem_l", -0.25, 0.82), ("hem_r", 0.25, 0.82), ("hip_r", 0.24, 0.92), ("under_r", 0.22, 1.16),
        ("arm_r", 0.205, 1.30), ("shoulder_r", 0.20, 1.405), ("neck_r", 0.082, 1.45), ("neck_c", 0.0, neck_z),
        ("neck_l", -0.082, 1.45), ("shoulder_l", -0.20, 1.405), ("arm_l", -0.205, 1.30), ("under_l", -0.22, 1.16),
        ("hip_l", -0.235, 0.92),
    ]


# The sleeves: the sleeved shirt's (`art/wardrobe/torso/shirt/build.py`), drawn to the wrist along
# the reference arm's own line (measured on the CC0 male: the forearm's middle is at x 0.25 at
# 1.2 m and 0.345 at the wrist): roomier than the jerkin's, clear of a heavy base's deltoids.
SHOULDER = (0.215, 1.296)
WRIST = (0.345, 0.965)
LENGTH, RADIUS = 0.40, 0.097


def sleeve(side, y, front):
    s = -1 if side == "l" else 1  # the body panel names its -x side `_l`
    dx, dz = WRIST[0] - SHOULDER[0], WRIST[1] - SHOULDER[1]
    n = math.hypot(dx, dz)
    d = (dx / n, dz / n)
    up = (-d[1], d[0])
    cx, cz = SHOULDER[0] + d[0] * LENGTH, SHOULDER[1] + d[1] * LENGTH
    pts = [("top", 0.20, 1.405), ("cuff_top", cx + up[0] * RADIUS, cz + up[1] * RADIUS),
           ("cuff_under", cx - up[0] * RADIUS, cz - up[1] * RADIUS), ("pit", 0.22, 1.16)]
    outline = [(name, s * x, z) for name, x, z in pts]
    if s > 0:
        outline = list(reversed(outline))
    return sewing.Panel(f"sleeve_{side}_{'f' if front else 'b'}", outline, y=y, front=front)


# A round neck, scooped lower in front than the gambeson's (on the heavy feminine base the throat
# came through the jerkin's), under the collar's band.
panels = [sewing.Panel("front", body(1.37), y=-0.17, front=True), sewing.Panel("back", body(1.45), y=0.15, front=False)]
for side in ("l", "r"):
    panels += [sleeve(side, -0.06, True), sleeve(side, 0.08, False)]
seams = [
    (("front", "shoulder_r", "neck_r"), ("back", "shoulder_r", "neck_r")),
    (("front", "neck_l", "shoulder_l"), ("back", "neck_l", "shoulder_l")),
    (("front", "hem_r", "under_r"), ("back", "hem_r", "under_r")),
    (("front", "under_l", "hem_l"), ("back", "under_l", "hem_l")),
]
for side in ("r", "l"):
    seams.append(((f"sleeve_{side}_f", "top", "cuff_top"), (f"sleeve_{side}_b", "top", "cuff_top")))
    seams.append(((f"sleeve_{side}_f", "pit", "cuff_under"), (f"sleeve_{side}_b", "pit", "cuff_under")))
    seams.append(((f"sleeve_{side}_f", "top", "pit"), ("front", f"shoulder_{side}", f"under_{side}")))
    seams.append(((f"sleeve_{side}_b", "top", "pit"), ("back", f"shoulder_{side}", f"under_{side}")))
garment = sewing.Garment(panels, seams, budget=820, cloth=CHARCOAL, trim_color=EMBER, trim_slot="cloth",
                         trim=0.016, thickness=0.004)
sewing.build(garment, os.path.join(HERE, "item.blend"))
