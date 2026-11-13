# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shirt with short sleeves (22 §4, M1): a front and a back, and two sleeves, each a front and a
back half turned to hang along the arm, sewn at the shoulders, the sides, the armholes and under
the sleeves, simulated onto the reference body. Hem, neckline and cuffs are trimmed.

    blender -b --python art/wardrobe/torso/shirt/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402


def body(neck_z):
    """The body panel, front or back (the character's right on -x)."""
    return [
        ("hem_l", -0.235, 0.86), ("hem_r", 0.235, 0.86), ("under_r", 0.205, 1.17), ("arm_r", 0.19, 1.30),
        ("shoulder_r", 0.20, 1.405), ("neck_r", 0.088, 1.44), ("neck_c", 0.0, neck_z), ("neck_l", -0.088, 1.44),
        ("shoulder_l", -0.20, 1.405), ("arm_l", -0.19, 1.30), ("under_l", -0.205, 1.17),
    ]


# A sleeve half, drawn along the reference arm (the CC0 male's, `cc0_body.PRIMITIVE`): from its
# armhole (the body panel's shoulder and underarm points) out along the arm's line from the
# shoulder joint toward the wrist, its cuff a band round the arm 24 cm down. The front half and
# the back half are sewn over and under the arm.
import math  # noqa: E402

SHOULDER = (0.215, 1.296)
WRIST = (0.469, 0.956)
LENGTH, RADIUS = 0.24, 0.055


def sleeve(side, y, front):
    s = -1 if side == "l" else 1  # the body panel names its -x side `_l`
    dx, dz = WRIST[0] - SHOULDER[0], WRIST[1] - SHOULDER[1]
    n = math.hypot(dx, dz)
    d = (dx / n, dz / n)
    up = (-d[1], d[0])  # across the arm, toward the top
    cx, cz = SHOULDER[0] + d[0] * LENGTH, SHOULDER[1] + d[1] * LENGTH
    pts = [("top", 0.20, 1.405), ("cuff_top", cx + up[0] * RADIUS, cz + up[1] * RADIUS),
           ("cuff_under", cx - up[0] * RADIUS, cz - up[1] * RADIUS), ("pit", 0.205, 1.17)]
    outline = [(name, s * x, z) for name, x, z in pts]
    # Counter-clockwise seen from the front: the right-hand (+x) sleeve runs top, pit, cuffs.
    if s > 0:
        outline = list(reversed(outline))
    return sewing.Panel(f"sleeve_{side}_{'f' if front else 'b'}", outline, y=y, front=front)


panels = [sewing.Panel("front", body(1.38), y=-0.17, front=True), sewing.Panel("back", body(1.44), y=0.15, front=False)]
for side in ("l", "r"):
    panels += [sleeve(side, -0.06, True), sleeve(side, 0.08, False)]
seams = [
    (("front", "shoulder_r", "neck_r"), ("back", "shoulder_r", "neck_r")),
    (("front", "neck_l", "shoulder_l"), ("back", "neck_l", "shoulder_l")),
    (("front", "hem_r", "under_r"), ("back", "hem_r", "under_r")),
    (("front", "under_l", "hem_l"), ("back", "under_l", "hem_l")),
]
for side in ("r", "l"):
    # Each sleeve half's top to its partner's (over the arm), its underside to its partner's
    # (under the arm), and its shoulder edge into the body's armhole.
    seams.append(((f"sleeve_{side}_f", "top", "cuff_top"), (f"sleeve_{side}_b", "top", "cuff_top")))
    seams.append(((f"sleeve_{side}_f", "pit", "cuff_under"), (f"sleeve_{side}_b", "pit", "cuff_under")))
    seams.append(((f"sleeve_{side}_f", "top", "pit"), ("front", f"shoulder_{side}", f"under_{side}")))
    seams.append(((f"sleeve_{side}_b", "top", "pit"), ("back", f"shoulder_{side}", f"under_{side}")))
garment = sewing.Garment(panels, seams, budget=1500, cloth=(0.70, 0.66, 0.58), trim_color=(0.30, 0.22, 0.14))
sewing.build(garment, os.path.join(HERE, "item.blend"))
