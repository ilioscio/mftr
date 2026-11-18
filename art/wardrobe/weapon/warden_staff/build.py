# SPDX-License-Identifier: AGPL-3.0-or-later
"""A warden's staff (Cairn's): a gnarled, wandering shaft of dark wood with knots along it, a
leather wrap at the grip, and at its head four twisted roots curling up round a glowing emerald,
holding it.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7); held as the staff is. The
shaft wanders little where the hands go (the grasp's span) and more beyond.

    blender -b --python art/wardrobe/weapon/warden_staff/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import armory  # noqa: E402

WOOD = (0.26, 0.18, 0.12)
WOOD_DARK = (0.15, 0.10, 0.07)
LEATHER = (0.30, 0.20, 0.12)
EMERALD = (0.30, 1.0, 0.55)
EMERALD_DEEP = (0.06, 0.62, 0.30)

FOOT, NECK = -0.66, 0.96
GEM_Z = 1.07


def wander(z):
    """The shaft's line at `z`: a little crooked, more so away from the hands."""
    k = 0.004 if -0.5 <= z <= 0.85 else 0.012
    return Vector((k * math.sin(6.1 * z + 0.4), k * 0.8 * math.sin(4.3 * z + 1.9), z))


def radius(z):
    # Thicker toward the head, swelling at the knots.
    r = 0.020 + 0.006 * (z - FOOT) / (NECK - FOOT)
    for knot in (-0.38, 0.52, 0.78):
        r += 0.006 * max(0.0, 1.0 - abs(z - knot) / 0.03)
    return r


k = armory.Kit()
steps = 26
k.branch([(wander(FOOT + (NECK - FOOT) * i / steps), radius(FOOT + (NECK - FOOT) * i / steps) * (0.8 if i == 0 else 1.0))
          for i in range(steps + 1)], WOOD, sides=6)
k.lathe([(-0.09, 0.026), (0.09, 0.026)], LEATHER, sides=6, at=(wander(0.0).x, wander(0.0).y, 0.0))
# The roots: four, out of the shaft's head, twisting up round the stone and closing over it.
for i in range(4):
    a0 = 2 * math.pi * i / 4
    path = []
    for j in range(8):
        t = j / 7
        a = a0 + 1.1 * t
        r = 0.020 + 0.040 * math.sin(math.pi * min(t * 1.15, 1.0)) - 0.012 * t
        z = NECK - 0.02 + 0.17 * t
        path.append(((r * math.cos(a), r * math.sin(a), z), 0.011 * (1 - t) + 0.003 if j < 7 else 0.0))
    k.branch(path, WOOD_DARK if i % 2 else WOOD, sides=5)
# The emerald, upright in the roots, its heart deeper.
k.gem((0.0, 0.0, GEM_Z), 0.040, EMERALD, stretch=1.5)
k.gem((0.0, 0.0, GEM_Z), 0.026, EMERALD_DEEP, stretch=1.8, turn=None)
k.save(os.path.join(HERE, "item.blend"))
