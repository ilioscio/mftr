# SPDX-License-Identifier: AGPL-3.0-or-later
"""A lantern staff (Lumen's): a pale wooden shaft shod in gold, a leather wrap at the grip and gold
bands, and at its head a gold sun ring round a caged lantern burning sun-white, a spike above it.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7); held as the staff is.

    blender -b --python art/wardrobe/weapon/lantern_staff/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import armory  # noqa: E402

WOOD = (0.46, 0.34, 0.22)
GOLD = (0.78, 0.60, 0.22)
GOLD_DARK = (0.52, 0.38, 0.14)
LEATHER = (0.30, 0.20, 0.12)
SUN = (1.0, 0.86, 0.42)
SUN_WHITE = (1.0, 0.96, 0.78)

k = armory.Kit()
# The shaft, shod in gold, wrapped at the grip, banded.
k.lathe([(-0.66, 0.0), (-0.64, 0.016), (-0.58, 0.022), (-0.56, 0.024)], GOLD, "metal", sides=6)
k.lathe([(-0.56, 0.020), (-0.10, 0.021), (0.10, 0.021), (0.90, 0.023)], WOOD, sides=6)
k.lathe([(-0.09, 0.025), (0.09, 0.025)], LEATHER, sides=6)
for z in (-0.30, 0.38, 0.62):
    k.lathe([(z - 0.012, 0.026), (z + 0.012, 0.026)], GOLD, "metal", sides=6)
# The collar the head stands on.
k.lathe([(0.88, 0.026), (0.92, 0.036), (0.96, 0.030), (0.99, 0.020)], GOLD, "metal", sides=6)
# The sun ring, upright across the staff's face (the lantern seen through it from the front).
RING_Z, RING_R = 1.13, 0.115
k.ring((0, 0, RING_Z), RING_R, 0.011, GOLD, axis=(0, 1, 0), sides=20, tube_sides=4, squash=1.6)
# Its rays: eight short spikes out from the ring, the top one long (a finial).
for i in range(8):
    t = 2 * math.pi * i / 8
    c, s = math.sin(t), math.cos(t)
    length = 0.075 if i == 0 else 0.04
    k.lathe([(0.0, 0.012), (length, 0.0)], GOLD, "metal", sides=4,
            at=(c * (RING_R + 0.008), 0.0, RING_Z + s * (RING_R + 0.008)), axis=(c, 0.0, s))
# The lantern hangs in the ring's middle from its top: a cap and a base of gold, four ribs, the
# light between.
k.lathe([(RING_Z + 0.05, 0.034), (RING_Z + 0.07, 0.026), (RING_Z + RING_R - 0.01, 0.006)], GOLD, "metal", sides=6)
k.lathe([(RING_Z - 0.075, 0.0), (RING_Z - 0.065, 0.024), (RING_Z - 0.05, 0.032)], GOLD_DARK, "metal", sides=6)
for i in range(4):
    t = 2 * math.pi * (i + 0.5) / 4
    x, y = 0.032 * math.cos(t), 0.032 * math.sin(t)
    k.lathe([(0.0, 0.004), (0.10, 0.004)], GOLD, "metal", sides=3, at=(x, y, RING_Z - 0.05), axis=(0, 0, 1))
k.lathe([(RING_Z - 0.05, 0.0), (RING_Z - 0.03, 0.022), (RING_Z, 0.026), (RING_Z + 0.03, 0.022), (RING_Z + 0.05, 0.0)], SUN_WHITE,
        "emissive", sides=6)
# The ring's foot: the ring stands on the collar.
k.lathe([(0.99, 0.018), (RING_Z - RING_R + 0.004, 0.012)], GOLD, "metal", sides=4)
k.save(os.path.join(HERE, "item.blend"))
