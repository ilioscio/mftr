# SPDX-License-Identifier: AGPL-3.0-or-later
"""A priestess's collar (Lumen's): a sky-blue collar standing round the neck and spreading over
the shoulders, rimmed in gold, a sun stone at its front; from it two gold laces run down the
bodice in a V to a second sun stone at the waist.

Lathed round the torso's middle (`mftr_blender.plates`): rings of rays find the skin from the
neck's base to over the shoulders, the collar standing further off ring by ring; the laces are
ribbons laid on the body, seen from the front; the stones small domes.

    blender -b --python art/wardrobe/shoulders/priestess_collar/build.py
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

SKY = ((0.20, 0.32, 0.50), "cloth")
GOLD = ((0.70, 0.54, 0.22), "metal")
SUN = ((1.0, 0.86, 0.42), "emissive")

TOP = 1.425               # the collar's top, round the neck's base
BOTTOM, SHOULDER = 1.315, 1.36   # its lower edge at the front and back, and over the shoulders
# Each ring's place from the top (0) to the lower edge (1); the last row is the gold rim.
RINGS = [0.0, 0.33, 0.66, 0.86, 1.0]
ANGLES = [360.0 * c / 20 for c in range(20)]
# The laces, the right one (the left mirrors it): from under the collar's front to the waist.
LACE = [(0.072, 1.335), (0.052, 1.24), (0.030, 1.14), (0.006, 1.045)]
STONE_TOP, STONE_WAIST = (0.0, 1.372), (0.0, 1.035)


def ring_z(r, a):
    low = BOTTOM + (SHOULDER - BOTTOM) * math.sin(math.radians(a)) ** 2
    return TOP + (low - TOP) * RINGS[r]


def ring_lift(r, _c):
    return 0.016 + 0.010 * RINGS[r]


def stone(bm, faces, tree, at, lift, size):
    """A sun stone in a gold setting (little four-sided domes) on the body's front at (x, z),
    `lift` off the skin."""
    start = Vector((at[0], -1.0, at[1]))
    hit, _n, _i, _d = tree.ray_cast(start, Vector((0, 1, 0)))
    # A gold setting, and the stone standing out of it.
    d = plates.Dome(hit - Vector((0, lift - 0.004, 0)), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0),
                    (size * 0.35, size * 1.45, size * 1.15))
    plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, 4, GOLD)
    d = plates.Dome(hit - Vector((0, lift, 0)), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (size * 0.6, size, size * 0.8))
    plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, 4, SUN)


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    centres = {}

    def centre(z):
        k = round(z, 4)
        if k not in centres:
            centres[k] = plates.ray_centre(tree, z, (0.0, 0.0))
        return centres[k]

    objs = []
    bm, faces = bmesh.new(), {}
    plates.lathe(bm, faces, tree, len(RINGS) - 1, ANGLES, ring_z, centre, ring_lift,
                 lambda r, _c: GOLD if r == len(RINGS) - 2 else SKY, smooth=6)
    objs.append(plates.finish(bm, faces, lambda c: centre(c.z), 0.005, "collar"))
    bm, faces = bmesh.new(), {}
    for s in (1, -1):
        plates.ribbon(bm, faces, tree, [(s * x, z) for x, z in LACE], 0.013, 0.016, GOLD)
    objs.append(plates.finish(bm, faces, lambda c: Vector((c.x, 0.0, c.z)), 0.003, "laces"))
    bm, faces = bmesh.new(), {}
    stone(bm, faces, tree, STONE_TOP, 0.026, 0.022)
    stone(bm, faces, tree, STONE_WAIST, 0.020, 0.018)
    objs.append(plates.finish(bm, faces, lambda c: Vector((c.x, 0.0, c.z)), 0.004, "stones"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
