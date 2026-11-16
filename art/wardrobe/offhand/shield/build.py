# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shield on the forearm: an iron heater with a ground rim, a team-colored field and a gold boss with a
glowing stone.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7).

    blender -b --python art/wardrobe/offhand/shield/build.py
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
out = [(-0.21, 0.25), (-0.21, -0.02), (-0.12, -0.19), (0.0, -0.27), (0.12, -0.19), (0.21, -0.02), (0.21, 0.25)]
k.slab(out, 0.018, IRON, "metal", bevel=0.03, turn=T, at=(0.07, -0.02, 0.0))
k.slab([(x * 0.74, z * 0.74) for x, z in out], 0.008, ACCENT, "accent", bevel=0.02, turn=T, at=(0.092, -0.02, 0.0))
k.lathe([(0.0, 0.065), (0.02, 0.055), (0.045, 0.0)], GOLD, "metal", sides=8, at=(0.098, -0.02, 0.02), axis=(1, 0, 0))
k.gem((0.135, -0.02, 0.02), 0.022, GLOW)
k.save(os.path.join(HERE, "item.blend"))
