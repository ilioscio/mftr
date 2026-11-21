# SPDX-License-Identifier: AGPL-3.0-or-later
"""Bark bracers (Cairn's): a leather sleeve round each forearm from the elbow to the wrist,
bound at both ends, and over it strips of bark laid lengthwise round all but the inside of the
arm, each its own shade.

Tubes round the reference forearm's own line (`mftr_blender.plates`), as Shade's wraps are: the
leather stood off the skin far enough to go over a sleeve; the bark a part of a second, further
tube, left open on the inside of the forearm, against the body.

    blender -b --python art/wardrobe/arms/bark_bracers/build.py
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

LEATHER = ((0.20, 0.13, 0.072), "cloth")
LEATHER_DARK = ((0.12, 0.075, 0.042), "cloth")
# Vertex colors are linear; the tones sit close, and the grain (`plates.grain`) does the rest.
BARK = [((0.150, 0.088, 0.048), "cloth"), ((0.185, 0.112, 0.064), "cloth"), ((0.115, 0.066, 0.037), "cloth")]

# The left forearm's middle, elbow to wrist (the priestess's sleeves' line, measured on the CC0
# male); the right mirrors it. Rows between stations: which are leather bands.
FOREARM = [(0.283, 0.006, 1.106), (0.300, 0.000, 1.068), (0.316, -0.009, 1.030), (0.330, -0.018, 0.996),
           (0.345, -0.027, 0.962), (0.360, -0.041, 0.926)]
BANDS = {0, 4}
SIDES = 8
LIFT = 0.022
# The bark: from the second station to the fifth, all round but the inside of the arm (the
# columns facing the body: the tube's rays turn from the front toward -x on both arms).
PLATE_ROWS = range(1, 4)
PLATE_LIFT = 0.032


def inner(s, c):
    return c in ((2,) if s > 0 else (6,))


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
                    lambda i, _c: LEATHER_DARK if i in BANDS else LEATHER, smooth=3, caps=(False, False), reach=0.10)
        plates.tube(pm, pfaces, tree, pts, (0.0, -1.0, 0.0), SIDES, lambda _i, c: PLATE_LIFT + 0.003 * (c % 2),
                    lambda i, c, s=s: BARK[c % 3] if i in PLATE_ROWS and not inner(s, c) else None, smooth=3,
                    caps=(False, False), reach=0.10)

    def inside(c):
        return Vector(min(mids, key=lambda p: (Vector(p) - c).length))
    objs.append(plates.finish(bm, faces, inside, 0.004, "leather"))
    objs.append(plates.finish(pm, pfaces, inside, 0.006, "bark"))
    plates.grain(objs[-1], plates.BARK_GRAIN)
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
