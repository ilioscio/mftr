# SPDX-License-Identifier: AGPL-3.0-or-later
"""A travelling cape (22 §4, M1; BW6's swinging cloth): a wide back panel flaring from the
shoulders to the calves, held by a collar sewn over the shoulders and across the chest,
simulated onto the reference body. Its hem and edges are trimmed in the team accent.

    blender -b --python art/wardrobe/back/cape/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402

# The back, seen from the front (the character's right on -x, named `_l` as the tunic's): wide
# at the hem, its top along the shoulders to the neck.
BACK = [
    ("hem_l", -0.40, 0.42), ("hem_r", 0.40, 0.42), ("side_r", 0.27, 1.05), ("shoulder_r", 0.16, 1.415),
    ("neck_r", 0.095, 1.43), ("neck_l", -0.095, 1.43), ("shoulder_l", -0.16, 1.415), ("side_l", -0.27, 1.05),
]


# The collar, seen from the front: over both shoulders and across the chest under the throat, so
# the cape hangs from a ring round the neck (two yokes alone let the cape's weight pull it off).
COLLAR = [
    ("low_l", -0.15, 1.32), ("low_c", 0.0, 1.30), ("low_r", 0.15, 1.32), ("shoulder_r", 0.16, 1.415),
    ("neck_r", 0.095, 1.43), ("throat", 0.0, 1.37), ("neck_l", -0.095, 1.43), ("shoulder_l", -0.16, 1.415),
]

panels = [sewing.Panel("back", BACK, y=0.19, front=False), sewing.Panel("collar", COLLAR, y=-0.17, front=True)]
seams = [(("collar", f"shoulder_{s}", f"neck_{s}"), ("back", f"shoulder_{s}", f"neck_{s}")) for s in ("r", "l")]
garment = sewing.Garment(panels, seams, budget=650, cloth=(0.30, 0.12, 0.10), trim_color=(0.55, 0.45, 0.20),
                         thickness=0.005, mirror=None)
sewing.build(garment, os.path.join(HERE, "item.blend"))
