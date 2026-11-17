# SPDX-License-Identifier: AGPL-3.0-or-later
"""A veteran's tabard (22 §4, M1; Rook's): long crimson panels front and back, hanging from a
band round the waist to below the knee, open at the sides from the hip down, their edges in a
darker crimson. The band is sewn at the sides like a skirt's waist; the panels below it hang
free. Simulated onto the reference body.

    blender -b --python art/wardrobe/waist/veteran_tabard/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402

CRIMSON = (0.55, 0.06, 0.06)
CRIMSON_DARK = (0.28, 0.035, 0.04)

# A panel seen from the front: a band from the waist over the hips, sewn to its partner at the
# sides, and below it a straight panel to below the knee, narrower than the hips (the legs
# show at the sides).
PANEL = [("hem_l", -0.165, 0.42), ("hem_r", 0.165, 0.42), ("flap_r", 0.17, 0.86), ("hip_r", 0.235, 0.88),
         ("waist_r", 0.19, 1.02), ("waist_l", -0.19, 1.02), ("hip_l", -0.235, 0.88), ("flap_l", -0.17, 0.86)]
front = sewing.Panel("front", PANEL, y=-0.16, front=True)
back = sewing.Panel("back", PANEL, y=0.17, front=False)
seams = [
    (("front", "hip_r", "waist_r"), ("back", "hip_r", "waist_r")),
    (("front", "waist_l", "hip_l"), ("back", "waist_l", "hip_l")),
]
garment = sewing.Garment([front, back], seams, budget=650, cloth=CRIMSON, trim_color=CRIMSON_DARK,
                         trim_slot="cloth", trim=0.035, lining=1.0, thickness=0.005)
sewing.build(garment, os.path.join(HERE, "item.blend"))
