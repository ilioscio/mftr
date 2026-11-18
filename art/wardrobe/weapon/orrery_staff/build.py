# SPDX-License-Identifier: AGPL-3.0-or-later
"""An orrery staff (Quill's): a tall shaft of dark wood shod and banded in brass, a leather grip,
and at its head an orrery: three brass rings crossing round a glowing star, two little planets
riding them.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7); held as the staff is. Its
foot is no further below the grip than the plain staff's: a staff's clips swing it past the head.

    blender -b --python art/wardrobe/weapon/orrery_staff/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import armory  # noqa: E402

WOOD = (0.26, 0.17, 0.10)
LEATHER = (0.28, 0.18, 0.10)
BRASS = (0.68, 0.50, 0.22)
STAR = (0.95, 0.92, 1.0)
PLANET = (0.30, 0.14, 0.44)
PLANET_2 = (0.62, 0.30, 0.34)

FOOT, NECK = -0.60, 0.98
ORB = Vector((0.0, 0.0, 1.10))       # the orrery's middle
RINGS = [(0.095, (0.0, 0.0, 1.0)), (0.085, (1.0, 0.0, 0.35)), (0.075, (-0.4, 1.0, 0.2))]

k = armory.Kit()
k.lathe([(FOOT - 0.04, 0.0), (FOOT, 0.020), (FOOT + 0.05, 0.022)], BRASS, slot="metal", sides=6)
k.lathe([(FOOT + 0.05, 0.021), (-0.10, 0.022), (0.10, 0.022), (NECK, 0.024)], WOOD, sides=6)
k.lathe([(-0.09, 0.026), (0.09, 0.026)], LEATHER, sides=6)
for z in (0.30, 0.62):
    k.lathe([(z - 0.012, 0.026), (z + 0.012, 0.026)], BRASS, slot="metal", sides=6)
# The head: a brass cup, a post up into the orrery.
k.lathe([(NECK - 0.02, 0.026), (NECK + 0.02, 0.034), (NECK + 0.035, 0.022)], BRASS, slot="metal", sides=8)
k.lathe([(NECK + 0.03, 0.007), (ORB.z - 0.02, 0.007)], BRASS, slot="metal", sides=4)
# The rings, crossing round the star, and two planets on them.
for r, axis in RINGS:
    k.ring(tuple(ORB), r, 0.0045, BRASS, axis=axis, sides=20, tube_sides=4)
k.gem(tuple(ORB), 0.034, STAR, stretch=1.3)
k.gem(tuple(ORB), 0.020, STAR, stretch=1.0, turn=None)
for (r, axis), colour, at in ((RINGS[1], PLANET, 0.6), (RINGS[2], PLANET_2, 2.4)):
    a = Vector(axis).normalized()
    e1 = a.orthogonal().normalized()
    e2 = a.cross(e1)
    p = ORB + (e1 * math.cos(at) + e2 * math.sin(at)) * r
    prof = [(-0.016 * math.cos(math.pi * t / 4), 0.016 * math.sin(math.pi * t / 4)) for t in range(5)]
    k.lathe(prof, colour, sides=6, at=tuple(p))
k.save(os.path.join(HERE, "item.blend"))
