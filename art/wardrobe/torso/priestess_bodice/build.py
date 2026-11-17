# SPDX-License-Identifier: AGPL-3.0-or-later
"""A priestess's bodice (22 §4, M1; Lumen's): fitted white linen from narrow straps over the
shoulders to just below the waist, a round neck kept under the collar's top, edged in ivory. Worn under the collar and the
skirt (the `under` layer: they're pushed clear of it). The linen tunic's pattern, cut shorter.

    blender -b --python art/wardrobe/torso/priestess_bodice/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mftr_blender import sewing  # noqa: E402

WHITE = (0.58, 0.56, 0.52)
IVORY = (0.50, 0.46, 0.38)


def outline(neck_z):
    """One panel, seen from the front: the linen tunic's armholes and straps, to a hem over the
    hips."""
    return [
        ("hem_l", -0.235, 0.88), ("hem_r", 0.235, 0.88),
        ("waist_r", 0.205, 0.97), ("under_r", 0.185, 1.09), ("arm_r", 0.14, 1.27), ("shoulder_r", 0.15, 1.41),
        ("neck_r", 0.088, 1.425), ("neck_c", 0.0, neck_z), ("neck_l", -0.088, 1.425),
        ("shoulder_l", -0.15, 1.41), ("arm_l", -0.14, 1.27), ("under_l", -0.185, 1.09), ("waist_l", -0.205, 0.97),
    ]


front = sewing.Panel("front", outline(1.38), y=-0.17, front=True)
back = sewing.Panel("back", outline(1.41), y=0.15, front=False)
seams = [
    (("front", "shoulder_r", "neck_r"), ("back", "shoulder_r", "neck_r")),
    (("front", "neck_l", "shoulder_l"), ("back", "neck_l", "shoulder_l")),
    (("front", "hem_r", "under_r"), ("back", "hem_r", "under_r")),
    (("front", "under_l", "hem_l"), ("back", "under_l", "hem_l")),
]
garment = sewing.Garment([front, back], seams, budget=340, cloth=WHITE, trim_color=IVORY, trim_slot="cloth",
                         trim=0.025)
sewing.build(garment, os.path.join(HERE, "item.blend"))
