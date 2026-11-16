# SPDX-License-Identifier: AGPL-3.0-or-later
"""A maul: a thick banded haft and a great faceted iron head, its striking faces banded in gold, a glowing rune
stone in each cheek and a spike on top.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/weapon/maul/build.py
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
k.lathe([(-0.30, 0.0), (-0.28, 0.034), (-0.24, 0.04), (-0.22, 0.034)], IRON, "metal", sides=6)
k.lathe([(-0.22, 0.034), (1.14, 0.038)], WOOD, sides=6)
for z in (-0.02, 0.10, 0.55, 0.95):
    k.lathe([(z - 0.02, 0.043), (z + 0.02, 0.043)], LEATHER if z < 0.3 else IRON, "cloth" if z < 0.3 else "metal", sides=6)
k.lathe([(-0.27, 0.15), (-0.25, 0.19), (-0.12, 0.20), (-0.09, 0.17), (0.09, 0.17), (0.12, 0.20), (0.25, 0.19), (0.27, 0.15)],
        IRON, "metal", sides=8, at=(0, 0, 1.30), axis=(1, 0, 0), twist=math.pi / 8)
for sx in (-1, 1):
    k.lathe([(-0.0175, 0.205), (0.0175, 0.205)], GOLD, "metal", sides=8, at=(sx * 0.19, 0, 1.30), axis=(1, 0, 0), twist=math.pi / 8)
for sy in (-1, 1):
    k.gem((0, sy * 0.165, 1.30), 0.045, GLOW, stretch=0.6, turn=Matrix.Rotation(math.pi / 2, 3, "X"))
k.lathe([(1.14, 0.05), (1.50, 0.05), (1.53, 0.0)], DARK, "metal", sides=6)
k.save(os.path.join(HERE, "item.blend"))
