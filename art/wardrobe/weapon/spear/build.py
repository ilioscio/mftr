# SPDX-License-Identifier: AGPL-3.0-or-later
"""A spear, carried point up (its head on the end above the fist: -Z in its frame): a long shaft, a
team-colored collar, a broad leaf head with glowing stones and an iron butt spike.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/weapon/spear/build.py
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
k.lathe([(-1.20, 0.022), (0.86, 0.024)], WOOD, sides=6)
k.lathe([(0.86, 0.028), (0.92, 0.02), (1.00, 0.0)], IRON, "metal", sides=6)
k.lathe([(-0.08, 0.027), (0.08, 0.027)], LEATHER, sides=6)
k.lathe([(-1.26, 0.03), (-1.23, 0.036), (-1.17, 0.036), (-1.14, 0.03)], ACCENT, "accent", sides=6)
k.blade([(1.24, 0.03), (1.30, 0.062), (1.40, 0.05), (1.58, 0.0)], STEEL, ridge=0.012, turn=Matrix.Rotation(math.pi, 3, "X"))
k.gem((0, -0.016, -1.31), 0.014, GLOW, stretch=1.5)
k.gem((0, 0.016, -1.31), 0.014, GLOW, stretch=1.5)
k.save(os.path.join(HERE, "item.blend"))
