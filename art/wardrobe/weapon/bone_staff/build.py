# SPDX-License-Identifier: AGPL-3.0-or-later
"""A bone staff (Marrow's): a crooked shaft of black wood shod in iron, a plum wrap at the grip,
crowned with a horned skull whose eye sockets burn magenta, its horns curling out, back and down
like a ram's; bone charms hang on cords under it.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7); held as the staff is. The
skull looks along -Y (the item's front, away from its back).

    blender -b --python art/wardrobe/weapon/bone_staff/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import armory  # noqa: E402

WOOD = (0.07, 0.06, 0.07)
WRAP = (0.24, 0.08, 0.20)
IRON = (0.18, 0.18, 0.20)
BONE = (0.64, 0.60, 0.50)
BONE_DARK = (0.42, 0.38, 0.30)
MAGENTA = (1.0, 0.24, 0.72)
SOCKET = (0.05, 0.03, 0.04)

# Its foot no further below the grip than the plain staff's: her clips swing it past her head.
FOOT, NECK = -0.60, 0.98
SKULL = Vector((0.0, -0.005, 1.055))     # the cranium's middle
R = 0.050                                # its radius


def crook(z):
    """The shaft's line at `z`: crooked over the hands, bending one way then the other; little
    below them (its foot swings past her head in her clips)."""
    k = 0.022 if z > 0.85 else 0.006
    return Vector((k * math.sin(4.7 * z + 0.9), k * 0.7 * math.sin(3.1 * z + 2.3), z))


def radius(z):
    r = 0.019 + 0.005 * (z - FOOT) / (NECK - FOOT)
    for knot in (-0.30, 0.60):
        r += 0.005 * max(0.0, 1.0 - abs(z - knot) / 0.03)
    return r


k = armory.Kit()
steps = 24
k.branch([(crook(FOOT + (NECK - FOOT) * i / steps), radius(FOOT + (NECK - FOOT) * i / steps))
          for i in range(steps + 1)], WOOD, sides=6)
k.lathe([(-0.05, 0.0), (0.0, 0.023), (0.04, 0.023)], IRON, slot="metal", sides=6, at=(crook(FOOT).x, crook(FOOT).y, FOOT))
k.lathe([(-0.10, 0.025), (0.10, 0.025)], WRAP, sides=6, at=(crook(0.0).x, crook(0.0).y, 0.0))
top = crook(NECK)
k.lathe([(-0.03, 0.027), (0.03, 0.030)], IRON, slot="metal", sides=6, at=(top.x, top.y, NECK - 0.01))

# The skull: the cranium (a lathe round its upright), the face and the jaw forward and below, the
# eye sockets dark with a magenta fire in each.
prof = [(-R * math.cos(math.pi * t / 6), R * math.sin(math.pi * t / 6) * (1.0 if t < 6 else 0.0)) for t in range(7)]
k.lathe([(z * 0.95, r) for z, r in prof], BONE, sides=8, at=tuple(SKULL))
face = SKULL + Vector((0.0, -0.040, -0.040))
k.lathe([(-0.035, 0.0), (-0.025, 0.030), (0.020, 0.038), (0.035, 0.0)], BONE, sides=6, at=tuple(face), axis=(0, -0.3, 1))
jaw = SKULL + Vector((0.0, -0.035, -0.085))
k.lathe([(-0.018, 0.0), (-0.010, 0.026), (0.012, 0.030), (0.020, 0.0)], BONE_DARK, sides=6, at=tuple(jaw), axis=(0, -0.5, 1))
for s in (1, -1):
    eye = SKULL + Vector((s * 0.023, -0.050, -0.012))
    k.lathe([(-0.008, 0.0), (0.0, 0.016), (0.006, 0.0)], SOCKET, sides=6, at=tuple(eye), axis=(0, -1, 0))
    k.gem(tuple(eye + Vector((0.0, -0.007, 0.0))), 0.010, MAGENTA, turn=None)
    # The horns: from the temples, out, back and curling down and forward like a ram's.
    path = []
    for j in range(9):
        t = j / 8
        a = math.pi * 1.4 * t
        rr = 0.038 * (1 - 0.35 * t)
        p = SKULL + Vector((s * (0.040 + 0.035 * t), 0.010 + rr * math.sin(a), 0.035 + rr * (math.cos(a) - 1) * 0.9 + 0.03 * t))
        path.append((tuple(p), 0.017 * (1 - t) + 0.003 if j < 8 else 0.0))
    k.branch(path, BONE_DARK if s > 0 else BONE, sides=6)

# Bone charms on cords under the skull.
for i, (x, y, drop) in enumerate(((0.032, 0.012, 0.13), (-0.030, 0.018, 0.17), (0.004, 0.034, 0.10))):
    a = Vector((top.x + x, top.y + y, NECK - 0.02))
    b = a + Vector((0.0, 0.0, -drop))
    k.branch([(tuple(a), 0.003), (tuple(b), 0.003)], IRON, slot="metal", sides=3)
    k.gem(tuple(b + Vector((0.0, 0.0, -0.022))), 0.012, BONE, slot="cloth", stretch=2.0)
k.gem((top.x + 0.004, top.y + 0.034, NECK - 0.15), 0.008, MAGENTA, stretch=1.4)
k.save(os.path.join(HERE, "item.blend"))
