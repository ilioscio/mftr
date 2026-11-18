# SPDX-License-Identifier: AGPL-3.0-or-later
"""Shadow wraps (Shade's): grey-violet cloth wound round each forearm from the elbow to the
wrist, bound by black leather bands, a dark steel plate strapped along the outside of each.

Tubes round the reference forearm's own line (`mftr_blender.plates`), as the priestess's sleeves
are: rings of rays out across the arm find the skin, the wraps stood off it far enough to go over
a sleeve; the plate a part of a second, further tube on the arm's outside.

    blender -b --python art/wardrobe/arms/shadow_wraps/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

from mftr_blender import cc0_body, plates  # noqa: E402

WRAP = ((0.24, 0.21, 0.28), "cloth")
LEATHER = ((0.07, 0.06, 0.08), "cloth")
STEEL_DARK = ((0.20, 0.21, 0.25), "metal")

# The left forearm's middle, elbow to wrist (the priestess's sleeves' line, measured on the CC0
# male); the right mirrors it. Rows between stations: which are leather bands.
FOREARM = [(0.283, 0.006, 1.106), (0.300, 0.000, 1.068), (0.316, -0.009, 1.030), (0.330, -0.018, 0.996),
           (0.345, -0.027, 0.962), (0.360, -0.041, 0.926)]
BANDS = {0, 2, 4}
SIDES = 8
LIFT = 0.024
# The plate: from the second station to the fifth, on the arm's outside (the columns facing
# away from the body: the tube's rays turn from the front toward -x on both arms).
PLATE_ROWS = range(1, 4)
PLATE_LIFT = 0.036


def outside(s, c):
    return c in ((5, 6) if s > 0 else (1, 2))


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = []
    mids = []
    bm, faces = bmesh.new(), {}
    pm, pfaces = bmesh.new(), {}
    for s in (1, -1):
        pts = [(s * x, y, z) for x, y, z in FOREARM]
        mids += pts
        plates.tube(bm, faces, tree, pts, (0.0, -1.0, 0.0), SIDES, lambda _i, _c: LIFT,
                    lambda i, _c: LEATHER if i in BANDS else WRAP, smooth=3, caps=(False, False), reach=0.10)
        plates.tube(pm, pfaces, tree, pts, (0.0, -1.0, 0.0), SIDES, lambda _i, _c: PLATE_LIFT,
                    lambda i, c, s=s: STEEL_DARK if i in PLATE_ROWS and outside(s, c) else None, smooth=3,
                    caps=(False, False), reach=0.10)

    def inside(c):
        return Vector(min(mids, key=lambda p: (Vector(p) - c).length))
    objs.append(plates.finish(bm, faces, inside, 0.004, "wraps"))
    objs.append(plates.finish(pm, pfaces, inside, 0.006, "plates"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
