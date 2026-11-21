# SPDX-License-Identifier: AGPL-3.0-or-later
"""A stone mantle (Cairn's): a collar of granite slabs laid round the base of the neck and over
the top of the chest and back, and a great granite pauldron domed over each shoulder above a
slab down the outside of the arm; moss grows over the tops of them all.

Modelled around the reference body (`mftr_blender.plates`):
- the collar is lathed over the chest and the back up to the neck's base (the pauldrons cover
  the sides), each slab a column of its own stone's grey, a little more or less proud than its
  neighbours;

- each pauldron is a dome tilted out over the shoulder, cut few-sided so it reads as cut
  stone, cut short on the neck side, over one slab on the outside;
- moss: every face looking up more than `MOSS_UP` is moss.

On the `shoulders` slot (fitted to the shoulder caps, not the arms), worn over the torso.

    blender -b --python art/wardrobe/shoulders/stone_mantle/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

from mftr_blender import cc0_body, plates  # noqa: E402

# Vertex colors are linear; the tones sit close, and the grain (`plates.grain`) does the rest.
GRANITE = ((0.30, 0.30, 0.28), "metal")
GRANITE_LIGHT = ((0.34, 0.335, 0.31), "metal")
GRANITE_DARK = ((0.25, 0.25, 0.235), "metal")
MOSS = ((0.070, 0.150, 0.040), "cloth")
MOSS_DARK = ((0.050, 0.110, 0.028), "cloth")
MOSS_UP = 0.62

# The collar: from over the chest and the shoulder blades up to the neck's base.
COLLAR_LOW, COLLAR_ROWS, COLLAR_COLS = 1.33, 3, 7
COLLAR_SPAN = 62.0     # each runs this far (degrees) either side of the front or the back
# The pauldrons: domes over the reference shoulder joints.
SHOULDER = Vector((0.205, 0.0, 1.29))
RADII = (0.125, 0.122, 0.14)   # along the axis, out, and front to back
TILT = 0.36


def collar_top(angle):
    """The collar's top at `angle` round the neck (0 the front): low in front, under the chin."""
    return 1.405 + 0.035 * (1 - math.cos(math.radians(angle))) / 2


def cap_end(phi):
    """How far down a pauldron runs at `phi` round it: low on the outside, short on the neck side."""
    return 0.80 + 0.42 * (1 + math.cos(phi)) / 2


def stone(c):
    return (GRANITE, GRANITE_LIGHT, GRANITE, GRANITE_DARK)[(c * 3) % 4]


def collar(tree):
    bm, faces = bmesh.new(), {}

    def z_at(r, a):
        return COLLAR_LOW + (collar_top(a) - COLLAR_LOW) * r / COLLAR_ROWS

    def lift(r, c):
        # Thick at its foot, closer at the neck; each slab a little more or less proud.
        return 0.040 - 0.016 * r / COLLAR_ROWS + 0.005 * ((c * 5) % 3 - 1)

    # Over the chest and over the back: the pauldrons cover the sides.
    for mid in (0.0, 180.0):
        angles = [mid - COLLAR_SPAN + 2 * COLLAR_SPAN * c / COLLAR_COLS for c in range(COLLAR_COLS + 1)]
        plates.lathe(bm, faces, tree, COLLAR_ROWS, angles, z_at, lambda z: Vector((0.0, -0.01, z)), lift,
                     lambda r, c: stone(c), loop=False, smooth=3)
    return bm, faces


def pauldrons(bm, faces):
    for s in (1, -1):
        d = plates.Dome((s * SHOULDER.x, SHOULDER.y, SHOULDER.z), (s * TILT, 0.0, 0.9), (s * 0.9, 0.0, -TILT), RADII)
        plates.dome(bm, faces, d, cap_end, 3, 8, GRANITE, rim=GRANITE_DARK)
        outer = [-1.0 + 2.0 * i / 4 for i in range(5)]
        plates.strip(bm, faces, d, outer, lambda ph: cap_end(ph) - 0.03, lambda ph: cap_end(ph) + 0.30, 2,
                     GRANITE_LIGHT, k=1.05)


def moss(faces):
    """Moss over whatever looks up; the tops of the slabs' rims darker."""
    for f in list(faces):
        f.normal_update()
        up = abs(f.normal.z)
        if up > MOSS_UP:
            faces[f] = MOSS if up > 0.8 else MOSS_DARK


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    bm, faces = collar(tree)
    collar_faces = set(faces)
    pauldrons(bm, faces)
    moss(faces)

    def inside(c):
        if abs(c.x) > 0.17 and c.z < 1.42:
            return Vector((math.copysign(SHOULDER.x, c.x), SHOULDER.y, SHOULDER.z))
        return Vector((0.0, -0.01, c.z))

    obj = plates.finish(bm, faces, inside, 0.012, "mantle")
    plates.grain(obj, plates.STONE_GRAIN)
    del collar_faces
    bpy.data.objects.remove(body)
    plates.save([obj], os.path.join(HERE, "item.blend"))


build()
