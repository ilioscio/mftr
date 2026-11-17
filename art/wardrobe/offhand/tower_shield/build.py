# SPDX-License-Identifier: AGPL-3.0-or-later
"""A tower shield on the forearm: a tall iron slab with clipped corners, a team-colored stripe, gold studs and
a glowing sigil.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/offhand/tower_shield/build.py
"""

import math  # noqa: F401
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mathutils import Matrix  # noqa: E402, F401

from mftr_blender import armory  # noqa: E402

IRON = (0.42, 0.44, 0.48)
STEEL = (0.66, 0.68, 0.72)
DARK = (0.20, 0.21, 0.24)
GOLD = (0.78, 0.60, 0.22)
WOOD = (0.34, 0.20, 0.10)
LEATHER = (0.16, 0.10, 0.06)
GLOW = (0.20, 0.95, 0.85)
ACCENT = (0.25, 0.45, 0.85)

k = armory.Kit()
T = Matrix.Rotation(math.pi / 2, 3, "Z")
out = [(-0.28, 0.36), (-0.28, -0.34), (-0.20, -0.43), (0.20, -0.43), (0.28, -0.34), (0.28, 0.36), (0.20, 0.43), (-0.20, 0.43)]
k.slab(out, 0.024, IRON, "metal", bevel=0.035, turn=T, at=(0.08, -0.02, 0.0))
k.slab([(-0.05, 0.37), (-0.05, -0.37), (0.05, -0.37), (0.05, 0.37)], 0.008, ACCENT, "accent", bevel=0.012, turn=T, at=(0.108, -0.02, 0.0))
for sy in (-0.19, 0.19):
    for z in (-0.28, 0.0, 0.28):
        k.gem((0.106, -0.02 + sy, z), 0.018, GOLD, "metal")
k.gem((0.122, -0.02, 0.0), 0.04, GLOW, stretch=1.4)
k.save(os.path.join(HERE, "item.blend"))
