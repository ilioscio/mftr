# SPDX-License-Identifier: AGPL-3.0-or-later
"""An orb: a faceted glowing crystal in a gold cage of three claws on a short handle.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/offhand/orb/build.py
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
k.lathe([(-0.07, 0.0), (-0.06, 0.014), (0.02, 0.016), (0.03, 0.028), (0.04, 0.0)], GOLD, "metal", sides=6, at=(0, -0.06, 0))
k.lathe([(-0.06, 0.0), (-0.04, 0.038), (0.0, 0.058), (0.04, 0.038), (0.06, 0.0)], GLOW, "emissive", sides=6, at=(0, -0.06, 0.10))
for i in range(3):
    a = 2 * math.pi * i / 3 + 0.5
    c, s_ = math.cos(a), math.sin(a)
    pts = [(0.02, 0.03), (0.062, 0.07), (0.066, 0.12), (0.04, 0.155)]
    for (r0, z0), (r1, z1) in zip(pts, pts[1:]):
        k.lathe([(0.0, 0.008), (math.dist((r0, z0), (r1, z1)), 0.006)], GOLD, "metal", sides=4,
                at=(c * r0, -0.06 + s_ * r0, z0), axis=(c * (r1 - r0), s_ * (r1 - r0), z1 - z0))
k.save(os.path.join(HERE, "item.blend"))
