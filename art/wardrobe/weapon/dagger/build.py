# SPDX-License-Identifier: AGPL-3.0-or-later
"""A dagger: a leaf-shaped blade, a short dark guard with a glowing stone, a wrapped grip and an iron pommel.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/weapon/dagger/build.py
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
k.lathe([(-0.125, 0.0), (-0.115, 0.016), (-0.10, 0.016), (-0.095, 0.011)], IRON, "metal", sides=6)
k.lathe([(-0.095, 0.013), (-0.045, 0.015), (0.0, 0.013)], LEATHER, sides=6)
k.slab([(-0.055, 0.0), (-0.045, -0.015), (0.045, -0.015), (0.055, 0.0), (0.02, 0.022), (-0.02, 0.022)], 0.011, DARK, "metal", bevel=0.006)
k.blade([(0.02, 0.022), (0.12, 0.032), (0.30, 0.024), (0.47, 0.0)], STEEL, ridge=0.007)
k.gem((0, -0.012, 0.005), 0.008, GLOW)
k.save(os.path.join(HERE, "item.blend"))
