# SPDX-License-Identifier: AGPL-3.0-or-later
"""A wrap skirt (22 §4, M1): a front and a back panel, flared, sewn at the sides, from the waist
to the knee, simulated onto the reference body. The waistband and the hem are trimmed.

    blender -b --python art/wardrobe/waist/skirt/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402

# A panel seen from the front: narrow at the waist, flaring to a wide hem at the knee.
PANEL = [("hem_l", -0.33, 0.48), ("hem_r", 0.33, 0.48), ("hip_r", 0.235, 0.86), ("waist_r", 0.19, 1.0),
         ("waist_l", -0.19, 1.0), ("hip_l", -0.235, 0.86)]
front = sewing.Panel("front", PANEL, y=-0.16, front=True)
back = sewing.Panel("back", PANEL, y=0.17, front=False)
seams = [
    (("front", "hem_r", "waist_r"), ("back", "hem_r", "waist_r")),
    (("front", "hem_l", "waist_l"), ("back", "hem_l", "waist_l")),
]
garment = sewing.Garment([front, back], seams, budget=1100, cloth=(0.36, 0.20, 0.16), trim_color=(0.62, 0.50, 0.24))
sewing.build(garment, os.path.join(HERE, "item.blend"))
