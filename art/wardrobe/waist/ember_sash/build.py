# SPDX-License-Identifier: AGPL-3.0-or-later
"""An ember sash (Ember's): a sash of ember-red cloth round the waist over a coat, a band of the
team's color round its middle, closed at the front with a round gold buckle set with a flame
stone.

Plates (`mftr_blender.plates`): the sash lathed round the reference body, the buckle small domes
stood off its front.

    blender -b --python art/wardrobe/waist/ember_sash/build.py
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

EMBER = ((0.55, 0.10, 0.04), "cloth")
ACCENT = ((0.50, 0.50, 0.50), "accent")
GOLD = ((0.60, 0.40, 0.12), "metal")
FLAME = ((1.0, 0.50, 0.10), "emissive")

# The sash's rows (m up), over a coat: red, the team's band, red.
SASH = [0.950, 0.972, 1.006, 1.030]
SIDES = 16
LIFT = 0.030
BUCKLE = 0.024     # its radius


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = []
    bm, faces = bmesh.new(), {}
    plates.lathe(bm, faces, tree, len(SASH) - 1, [360.0 * c / SIDES for c in range(SIDES)],
                 lambda r, _a: SASH[r], lambda z: plates.ray_centre(tree, z, (0.0, 0.0)), lambda _r, _c: LIFT,
                 lambda r, _c: ACCENT if r == 1 else EMBER, smooth=4)
    objs.append(plates.finish(bm, faces, lambda c: Vector((0.0, 0.0, c.z)), 0.006, "sash"))
    # The buckle: a gold disc on the sash's front, a flame stone standing out of it.
    bm, faces = bmesh.new(), {}
    mid = (SASH[0] + SASH[-1]) / 2
    hit, _n, _i, _d = tree.ray_cast(Vector((0.0, -1.0, mid)), Vector((0, 1, 0)))
    front = hit - Vector((0.0, LIFT + 0.002, 0.0))
    d = plates.Dome(front, (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (0.008, BUCKLE, BUCKLE))
    plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, 8, GOLD)
    d = plates.Dome(front - Vector((0.0, 0.004, 0.0)), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (0.010, 0.011, 0.011))
    plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, 4, FLAME)
    objs.append(plates.finish(bm, faces, lambda c: Vector((c.x, 0.0, c.z)), 0.004, "buckle"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
