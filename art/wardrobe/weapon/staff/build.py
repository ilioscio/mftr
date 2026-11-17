# SPDX-License-Identifier: AGPL-3.0-or-later
"""A staff: a long wooden shaft shod in iron, a leather wrap at the grip, a gold collar and a head of three iron
prongs holding a glowing crystal.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/weapon/staff/build.py
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
k.lathe([(-0.64, 0.0), (-0.62, 0.02), (-0.56, 0.024), (-0.55, 0.021)], IRON, "metal", sides=6)
k.lathe([(-0.55, 0.021), (-0.10, 0.022), (0.10, 0.022), (0.92, 0.024)], WOOD, sides=6)
k.lathe([(-0.09, 0.026), (0.09, 0.026)], LEATHER, sides=6)
k.lathe([(0.92, 0.03), (0.95, 0.036), (0.98, 0.03)], GOLD, "metal", sides=6)
for i in range(3):
    a = 2 * math.pi * i / 3
    c, s_ = math.cos(a), math.sin(a)
    pts = [(0.03, 0.97), (0.075, 1.04), (0.07, 1.12), (0.035, 1.19)]
    for (r0, z0), (r1, z1) in zip(pts, pts[1:]):
        k.lathe([(0.0, 0.011), (math.dist((r0, z0), (r1, z1)), 0.009)], IRON, "metal", sides=4,
                at=(c * r0, s_ * r0, z0), axis=(c * (r1 - r0), s_ * (r1 - r0), z1 - z0))
k.gem((0, 0, 1.09), 0.05, GLOW, stretch=1.7)
k.save(os.path.join(HERE, "item.blend"))
