# SPDX-License-Identifier: AGPL-3.0-or-later
"""A long coat, open at the front (22 §4, M1): a back panel and two front panels, sewn at the
shoulders and the sides, sleeveless, falling below the knee, simulated onto the reference body.
Its open front, hem and armholes are trimmed in the team accent.

    blender -b --python art/wardrobe/back/coat/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402

# The back, seen from the front (the character's right on -x, named `_l` as the tunic's).
BACK = [
    ("hem_l", -0.31, 0.40), ("hem_r", 0.31, 0.40), ("waist_r", 0.23, 0.95), ("under_r", 0.205, 1.10),
    ("arm_r", 0.135, 1.27), ("shoulder_r", 0.135, 1.42), ("neck_r", 0.09, 1.445), ("neck_l", -0.09, 1.445),
    ("shoulder_l", -0.135, 1.42), ("arm_l", -0.135, 1.27), ("under_l", -0.205, 1.10), ("waist_l", -0.23, 0.95),
]


def front(side):
    """One front panel: from the side seam to the open edge, a hand's width short of the middle."""
    s = 1 if side == "r" else -1
    pts = [("hem_side", 0.31, 0.40), ("waist", 0.23, 0.95), ("under", 0.205, 1.10), ("arm", 0.135, 1.27),
           ("shoulder", 0.135, 1.42), ("neck", 0.09, 1.445), ("lapel", 0.05, 1.30), ("hem_edge", 0.06, 0.40)]
    outline = [(f"{n}_{side}", s * x, z) for n, x, z in pts]
    # Counter-clockwise seen from the front.
    if s > 0:
        outline = list(reversed(outline))
    return sewing.Panel(f"front_{side}", outline, y=-0.18, front=True)


panels = [sewing.Panel("back", BACK, y=0.16, front=False), front("r"), front("l")]
seams = []
for side in ("r", "l"):
    seams.append(((f"front_{side}", f"shoulder_{side}", f"neck_{side}"), ("back", f"shoulder_{side}", f"neck_{side}")))
    seams.append(((f"front_{side}", f"hem_side_{side}", f"under_{side}"), ("back", f"hem_{side}", f"under_{side}")))
garment = sewing.Garment(panels, seams, budget=1800, cloth=(0.22, 0.24, 0.30), trim_color=(0.55, 0.45, 0.20),
                         thickness=0.005, mirror="-x")
sewing.build(garment, os.path.join(HERE, "item.blend"))
