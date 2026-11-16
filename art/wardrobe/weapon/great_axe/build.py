# SPDX-License-Identifier: AGPL-3.0-or-later
"""A great axe: a long banded haft, a broad crescent blade ground to its edge, a back spike, a top spike and a
glowing stone in the blade.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/weapon/great_axe/build.py
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
k.lathe([(-0.32, 0.0), (-0.30, 0.032), (-0.26, 0.036), (-0.24, 0.032)], IRON, "metal", sides=6)
k.lathe([(-0.24, 0.032), (1.20, 0.034)], WOOD, sides=6)
for z in (-0.02, 0.12):
    k.lathe([(z - 0.02, 0.04), (z + 0.02, 0.04)], LEATHER, sides=6)
k.lathe([(0.90, 0.042), (1.22, 0.042)], IRON, "metal", sides=6)
k.slab([(0.03, 0.95), (0.20, 0.80), (0.36, 0.86), (0.42, 1.06), (0.36, 1.26), (0.20, 1.32), (0.03, 1.17)], 0.016, STEEL, "metal", bevel=0.05)
k.slab([(-0.03, 1.12), (-0.20, 1.06), (-0.03, 1.00)], 0.014, IRON, "metal", bevel=0.02)
k.lathe([(1.22, 0.03), (1.40, 0.0)], IRON, "metal", sides=4)
for sy in (-1, 1):
    k.gem((0.16, sy * 0.02, 1.06), 0.03, GLOW, stretch=0.4, turn=Matrix.Rotation(math.pi / 2, 3, "X"))
k.save(os.path.join(HERE, "item.blend"))
