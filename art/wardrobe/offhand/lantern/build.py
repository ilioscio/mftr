# SPDX-License-Identifier: AGPL-3.0-or-later
"""A lantern hanging from the hand: a ring and a short chain, an iron cap and base, four posts and a
glowing core.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/offhand/lantern/build.py
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
# A grip for the fist (at the origin), leather between iron caps; the chain hangs from its foot.
k.lathe([(-0.07, 0.0), (-0.06, 0.017), (-0.05, 0.012)], IRON, "metal", sides=6)
k.lathe([(-0.05, 0.012), (0.05, 0.012)], LEATHER, sides=6)
k.lathe([(0.05, 0.012), (0.06, 0.017), (0.07, 0.0)], IRON, "metal", sides=6)
k.lathe([(-0.16, 0.006), (-0.07, 0.006)], IRON, "metal", sides=4)
k.lathe([(-0.205, 0.07), (-0.19, 0.075), (-0.17, 0.03), (-0.16, 0.012)], IRON, "metal", sides=6)
k.lathe([(-0.355, 0.05), (-0.34, 0.072), (-0.325, 0.06)], IRON, "metal", sides=6)
for i in range(4):
    a = 2 * math.pi * i / 4 + math.pi / 4
    k.lathe([(-0.33, 0.007), (-0.20, 0.007)], DARK, "metal", sides=4, at=(math.cos(a) * 0.052, math.sin(a) * 0.052, 0))
k.lathe([(-0.32, 0.0), (-0.30, 0.036), (-0.24, 0.042), (-0.215, 0.022), (-0.205, 0.0)], GLOW, "emissive", sides=6)
k.save(os.path.join(HERE, "item.blend"))
