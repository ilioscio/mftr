# SPDX-License-Identifier: AGPL-3.0-or-later
"""Wool trousers (22 §4, M1): four panels, a front and a back for each leg, sewn at the outer
and inner leg seams and down the middle front and back, simulated onto the reference body.
They sit on the hips; the hems and the waist are trimmed.

    blender -b --python art/wardrobe/legs/trousers/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402

# One leg's panel, the character's left (+x), seen from the front: the inner leg seam from the
# crotch to the hem, the outer from the hem up the hip to the waist, the middle seam down.
# The waist is cut a little under the body's, so the cloth's tension holds them up; the hips
# close, as an under layer should be, so an over layer lies smooth on them.
LEFT = [
    ("hem_in", 0.07, 0.12), ("hem_out", 0.265, 0.12), ("knee_out", 0.255, 0.47), ("hip_out", 0.245, 0.84),
    ("waist_out", 0.185, 1.00), ("waist_c", 0.0, 1.01), ("crotch", 0.0, 0.80),
]
panels = [
    sewing.Panel("front_l", LEFT, y=-0.16, front=True),
    sewing.Panel("front_r", sewing.mirrored(LEFT), y=-0.16, front=True),
    sewing.Panel("back_l", LEFT, y=0.17, front=False),
    sewing.Panel("back_r", sewing.mirrored(LEFT), y=0.17, front=False),
]
seams = []
for side in ("l", "r"):
    seams.append(((f"front_{side}", "hem_out", "waist_out"), (f"back_{side}", "hem_out", "waist_out")))
    seams.append(((f"front_{side}", "crotch", "hem_in"), (f"back_{side}", "crotch", "hem_in")))
seams.append((("front_l", "waist_c", "crotch"), ("front_r", "waist_c", "crotch")))
seams.append((("back_l", "waist_c", "crotch"), ("back_r", "waist_c", "crotch")))
garment = sewing.Garment(panels, seams, budget=1400, cloth=(0.30, 0.27, 0.24), trim_color=(0.20, 0.15, 0.11),
                         trim_slot="cloth", trim=0.025)
sewing.build(garment, os.path.join(HERE, "item.blend"))
