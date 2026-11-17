# SPDX-License-Identifier: AGPL-3.0-or-later
"""A priestess's tails (22 §4, M1; Lumen's): a sky-blue sash round the waist, and from it three
long tails of white linen edged in sky blue, one down the back to the ankles and one behind each
hip, that stream as she moves. The sash's front and back are sewn at the sides; the tails are
sewn to the back's lower edge and hang free. Sewn over the priestess's skirt (it collides in the
sim), so they fall over it.

    blender -b --python art/wardrobe/back/priestess_tails/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mftr_blender import sewing  # noqa: E402

WHITE = (0.58, 0.56, 0.52)
SKY = (0.20, 0.32, 0.50)

# The sash, seen from the front: from the waist to over the hips; the back's lower edge is
# marked where the tails are sewn on.
SASH_FRONT = [("hip_l", -0.235, 0.91), ("hip_r", 0.235, 0.91), ("waist_r", 0.185, 1.02), ("waist_l", -0.185, 1.02)]
SASH_BACK = [("hip_l", -0.235, 0.91), ("lt_in", -0.16, 0.91), ("ct_l", -0.075, 0.91), ("ct_r", 0.075, 0.91),
             ("rt_in", 0.16, 0.91), ("hip_r", 0.235, 0.91), ("waist_r", 0.185, 1.02), ("waist_l", -0.185, 1.02)]


def tail(name, x0, x1, bottom, tip):
    """A tail from the sash's edge between x0 and x1 down to `bottom`, its end cut on a slant
    (`tip`: the outer corner's drop) or to a point in the middle (`tip` None)."""
    if tip is None:
        mid = (x0 + x1) / 2
        pts = [("bl", x0, bottom + 0.04), ("tip", mid, bottom), ("br", x1, bottom + 0.04)]
    else:
        lo_l, lo_r = (bottom, bottom + tip) if x0 < 0 else (bottom + tip, bottom)
        pts = [("bl", x0, lo_l), ("br", x1, lo_r)]
    return sewing.Panel(name, pts + [("tr", x1, 0.91), ("tl", x0, 0.91)], y=0.21, front=False)


panels = [
    sewing.Panel("sash_f", SASH_FRONT, y=-0.17, front=True, color=SKY),
    sewing.Panel("sash_b", SASH_BACK, y=0.19, front=False, color=SKY),
    tail("tail_l", -0.235, -0.16, 0.30, 0.05),
    tail("tail_c", -0.075, 0.075, 0.20, None),
    tail("tail_r", 0.16, 0.235, 0.30, 0.05),
]
seams = [
    (("sash_f", "hip_r", "waist_r"), ("sash_b", "hip_r", "waist_r")),
    (("sash_f", "waist_l", "hip_l"), ("sash_b", "waist_l", "hip_l")),
    (("tail_l", "tl", "tr"), ("sash_b", "hip_l", "lt_in")),
    (("tail_c", "tl", "tr"), ("sash_b", "ct_l", "ct_r")),
    (("tail_r", "tl", "tr"), ("sash_b", "rt_in", "hip_r")),
]
garment = sewing.Garment(panels, seams, budget=240, cloth=WHITE, trim_color=SKY, trim_slot="cloth",
                         trim=0.022, lining=1.0, thickness=0.004,
                         over=[os.path.join(ROOT, "art", "wardrobe", "waist", "priestess_skirt")])
sewing.build(garment, os.path.join(HERE, "item.blend"))
