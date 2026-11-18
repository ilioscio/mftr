# SPDX-License-Identifier: AGPL-3.0-or-later
"""Ember's bell sleeves: from the elbow an open sleeve of charcoal cloth belling wide over the
forearm to the wrist (over the coat's sleeve), a gold band at its top, its ember-red lining
turned back at the hem and a cuff of embers glowing round its edge. The priestess's sleeves
(`arms/priestess_sleeves`) without the armlets, flaring wider.

Tubes round the reference arm's own line (`mftr_blender.plates`), measured on the CC0 male: rings
of rays out across the arm find the skin; the sleeve stands further off it ring by ring, so it
bells. Its end is open.

    blender -b --python art/wardrobe/arms/ember_sleeves/build.py
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

CHARCOAL = ((0.11, 0.10, 0.10), "cloth")
EMBER = ((0.55, 0.10, 0.04), "cloth")
GOLD = ((0.60, 0.40, 0.12), "metal")
GLOW = ((1.0, 0.50, 0.10), "emissive")

# The right arm's middle, shoulder to wrist (measured on the CC0 male: rays across the arm, step
# by step down it); the left mirrors it.
# The sleeve from the elbow; past the wrist its last ring (the cuff) is carried on along the arm.
SLEEVE = [(0.283, 0.006, 1.106), (0.304, -0.001, 1.060), (0.322, -0.013, 1.014), (0.342, -0.025, 0.968),
          (0.361, -0.043, 0.922), (0.368, -0.050, 0.904)]
SIDES = 8
FLARE = 0.072     # how far the hem stands further off than the top


def sleeve_color(i, _c):
    if i == 0:
        return GOLD
    if i == len(SLEEVE) - 3:
        return EMBER
    if i == len(SLEEVE) - 2:
        return GLOW
    return CHARCOAL


def sleeve_lift(i, _c):
    t = i / (len(SLEEVE) - 1)
    return 0.016 + FLARE * t ** 1.6


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = []
    for part, stations, sides, lift, color in (("sleeves", SLEEVE, SIDES, sleeve_lift, sleeve_color),):
        bm = bmesh.new()
        faces = {}
        mids = []
        for s in (1, -1):
            pts = [(s * x, y, z) for x, y, z in stations]
            mids += pts
            plates.tube(bm, faces, tree, pts, (0.0, -1.0, 0.0), sides, lift, color, smooth=3,
                        caps=(False, False), reach=0.10)

        def inside(c, mids=mids):
            return Vector(min(mids, key=lambda p: (Vector(p) - c).length))
        objs.append(plates.finish(bm, faces, inside, 0.004, part))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
