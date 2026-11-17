# SPDX-License-Identifier: AGPL-3.0-or-later
"""Shadow leggings (22 §4, M1; Shade's): close black leggings, cut as the wool trousers are (four
panels, a crotch extension) but narrow down the leg, to a hem at the middle of the calf that
goes into the boots.

    blender -b --python art/wardrobe/legs/shadow_leggings/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402

# One leg's panel, the character's left (+x), seen from the front: the inner leg seam from the
# crotch to the hem, the outer from the hem up the hip to the waist, the middle seam down.
# The waist is cut a little under the body's, so the cloth's tension holds them up; the hips
# close, as an under layer should be, so an over layer lies smooth on them. The crotch extends past
# the middle (a little at the front, more at the back, as a tailor's pattern does): the fabric
# that passes between the legs, where four seams meet.
def leg(crotch_x, crotch_z, waist_z):
    return [
        ("hem_in", 0.075, 0.24), ("hem_out", 0.205, 0.24), ("knee_out", 0.235, 0.47), ("hip_out", 0.240, 0.84),
        ("waist_out", 0.185, waist_z), ("waist_c", 0.0, waist_z + 0.01), ("seat", -0.004, 0.88),
        ("crotch", crotch_x, crotch_z),
    ]


FRONT = leg(-0.055, 0.785, 1.00)
BACK = leg(-0.10, 0.775, 1.03)
panels = [
    sewing.Panel("front_l", FRONT, y=-0.16, front=True),
    sewing.Panel("front_r", sewing.mirrored(FRONT), y=-0.16, front=True),
    sewing.Panel("back_l", BACK, y=0.17, front=False),
    sewing.Panel("back_r", sewing.mirrored(BACK), y=0.17, front=False),
]
seams = []
for side in ("l", "r"):
    seams.append(((f"front_{side}", "hem_out", "waist_out"), (f"back_{side}", "hem_out", "waist_out")))
    seams.append(((f"front_{side}", "crotch", "hem_in"), (f"back_{side}", "crotch", "hem_in")))
seams.append((("front_l", "waist_c", "crotch"), ("front_r", "waist_c", "crotch")))
seams.append((("back_l", "waist_c", "crotch"), ("back_r", "waist_c", "crotch")))
garment = sewing.Garment(panels, seams, budget=900, cloth=(0.07, 0.065, 0.09), trim_color=(0.12, 0.09, 0.20),
                         trim_slot="cloth", trim=0.012, mirror=None)
sewing.build(garment, os.path.join(HERE, "item.blend"))
