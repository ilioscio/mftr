# SPDX-License-Identifier: AGPL-3.0-or-later
"""A priestess's underskirt (22 §4, M1; Lumen's): white linen from the waist to mid-shin, flaring,
sewn at the sides, a gold hem. Its waist covers the priestess's bodice's hem (the forge pushes
it clear of the bodice).

    blender -b --python art/wardrobe/waist/priestess_skirt/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mftr_blender import sewing  # noqa: E402

WHITE = (0.58, 0.56, 0.52)
GOLD = (0.70, 0.54, 0.22)

# A panel seen from the front: the wrap skirt's, long, flaring from the hips to mid-shin.
PANEL = [("hem_l", -0.36, 0.30), ("hem_r", 0.36, 0.30), ("knee_r", 0.30, 0.55), ("hip_r", 0.24, 0.86),
         ("waist_r", 0.195, 1.0), ("waist_l", -0.195, 1.0), ("hip_l", -0.24, 0.86), ("knee_l", -0.30, 0.55)]
front = sewing.Panel("front", PANEL, y=-0.17, front=True)
back = sewing.Panel("back", PANEL, y=0.18, front=False)
seams = [
    (("front", "hem_r", "waist_r"), ("back", "hem_r", "waist_r")),
    (("front", "hem_l", "waist_l"), ("back", "hem_l", "waist_l")),
]
garment = sewing.Garment([front, back], seams, budget=420, cloth=WHITE, trim_color=GOLD, trim_slot="metal",
                         trim=0.03, lining=0.4)
sewing.build(garment, os.path.join(HERE, "item.blend"))
