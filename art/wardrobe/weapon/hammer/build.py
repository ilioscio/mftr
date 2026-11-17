# SPDX-License-Identifier: AGPL-3.0-or-later
"""A war hammer: a one-handed haft with a wrapped grip, a beveled iron head with a gold striking face, a back
spike and a glowing stone.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/weapon/hammer/build.py
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
k.lathe([(-0.14, 0.0), (-0.125, 0.026), (-0.11, 0.026), (-0.10, 0.02)], IRON, "metal", sides=6)
k.lathe([(-0.10, 0.022), (0.05, 0.024)], LEATHER, sides=6)
k.lathe([(0.05, 0.02), (0.56, 0.022)], WOOD, sides=6)
k.box((0.03, 0, 0.62), (0.17, 0.12, 0.12), IRON, "metal", bevel=0.18)
k.box((0.125, 0, 0.62), (0.03, 0.135, 0.135), GOLD, "metal", bevel=0.15)
k.lathe([(0.0, 0.045), (0.14, 0.0)], IRON, "metal", sides=4, at=(-0.055, 0, 0.62), axis=(-1, 0, 0), twist=math.pi / 4)
k.lathe([(0.56, 0.03), (0.70, 0.03), (0.74, 0.0)], DARK, "metal", sides=6)
k.gem((0.03, -0.065, 0.62), 0.022, GLOW, stretch=0.5, turn=Matrix.Rotation(math.pi / 2, 3, "X"))
k.save(os.path.join(HERE, "item.blend"))
