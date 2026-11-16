# SPDX-License-Identifier: AGPL-3.0-or-later
"""A sword: a broad double-edged blade with a diamond section, a gold crossguard with swept quillons, a leather
grip and a gold pommel with a glowing stone.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/weapon/sword/build.py
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
k.lathe([(-0.165, 0.0), (-0.15, 0.024), (-0.128, 0.024), (-0.12, 0.014)], GOLD, "metal", sides=6)
k.gem((0, -0.02, -0.142), 0.011, GLOW)
k.lathe([(-0.12, 0.016), (-0.05, 0.019), (0.02, 0.016)], LEATHER, sides=6)
k.slab([(-0.12, 0.0), (-0.105, -0.03), (-0.03, 0.012), (0.03, 0.012), (0.105, -0.03), (0.12, 0.0), (0.035, 0.05), (-0.035, 0.05)],
       0.014, GOLD, "metal", bevel=0.008)
k.blade([(0.045, 0.046), (0.30, 0.042), (0.70, 0.036), (0.84, 0.026), (0.94, 0.0)], STEEL, ridge=0.009)
k.save(os.path.join(HERE, "item.blend"))
