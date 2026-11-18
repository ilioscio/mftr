# SPDX-License-Identifier: AGPL-3.0-or-later
"""A veteran's boots (22 §4; Rook's): heavy dark leather to under the knee, a wide cuff at the
top, two iron bands round the shin, an iron toe cap and thick soles, and an iron knee cop domed
over each knee.

- The feet are tubes from the heel to the toes round the reference feet, and the shafts lathed
  round each leg from over the foot's top to under the knee (`mftr_blender.plates`), so their
  soles, bands and cuff are straight rings and lines.
- The knee cops are domes in front of the knees, brass-rimmed.

    blender -b --python art/wardrobe/feet/veteran_boots/build.py
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

LEATHER = ((0.31, 0.19, 0.10), "cloth")
CUFF = ((0.24, 0.15, 0.08), "cloth")
SOLE = ((0.10, 0.07, 0.05), "cloth")
IRON = ((0.30, 0.31, 0.35), "metal")
IRON_DARK = ((0.15, 0.16, 0.19), "metal")
BRASS = ((0.62, 0.45, 0.18), "metal")

# The foot: a tube from the heel to the toes, through the reference foot (its middle measured on
# the CC0 male, the right foot; the left mirrors it): (x, y, z) stations.
FOOT = [(0.180, 0.095, 0.045), (0.186, 0.04, 0.045), (0.210, -0.005, 0.036), (0.226, -0.05, 0.026),
        (0.229, -0.09, 0.016), (0.228, -0.122, 0.010)]
FOOT_SIDES = 8
TOE_CAP = 2   # the last stations are under the iron toe cap


def foot_color(i, c):
    a = 2 * math.pi * c / FOOT_SIDES
    if math.cos(a) < -0.55:
        return SOLE
    if i >= len(FOOT) - 1 - TOE_CAP and math.cos(a) > -0.3:
        return IRON_DARK
    return LEATHER


def foot_lift(i, c):
    a = 2 * math.pi * c / FOOT_SIDES
    # Thick soles; a broad toe box.
    return (0.015 + (0.006 if math.cos(a) < -0.55 else 0.0) + 0.014 * max(0, i - 2) / 3
            + (0.006 if i <= 0 else 0.0))


# The shaft's rings (m up), from over the foot's top to the cuff's; what's between two rings.
RINGS = [0.085, 0.14, 0.20, 0.225, 0.275, 0.30, 0.355, 0.41]
BANDS = ((0.20, 0.225), (0.275, 0.30))
CUFF_FROM, FLARE = 0.355, 0.016
SIDES = 10


def ring_color(r):
    lo, hi = RINGS[r], RINGS[r + 1]
    if lo >= CUFF_FROM:
        return CUFF
    if any(a <= lo and hi <= b for a, b in BANDS):
        return IRON_DARK
    return LEATHER


def ring_lift(r):
    z = RINGS[r]
    lift = 0.016
    if z <= 0.12:
        # Over the foot's top edge, which it covers.
        lift += 0.008 * (0.12 - z) / 0.035
    if z > CUFF_FROM:
        lift += FLARE * (z - CUFF_FROM) / (RINGS[-1] - CUFF_FROM)
    if any(a <= z <= b for a, b in BANDS):
        lift += 0.004
    return lift


# The knee cops: in front of each reference knee (its middle at x 0.125, y 0.016, z 0.485).
KNEE = Vector((0.125, 0.016, 0.485))
COP = (0.085, 0.062, 0.075)   # along its axis (forward), across, and up


def shafts(tree):
    bm = bmesh.new()
    faces = {}
    for s in (1, -1):
        centres = {z: plates.ray_centre(tree, z, (s * 0.165, 0.04)) for z in RINGS}
        plates.lathe(bm, faces, tree, len(RINGS) - 1, [360.0 * c / SIDES for c in range(SIDES)],
                     lambda r, _a: RINGS[r], lambda z, centres=centres: centres[z], lambda r, _c: ring_lift(r),
                     lambda r, _c: ring_color(r), smooth=4)
    return plates.finish(bm, faces, lambda c: plates.ray_centre(tree, c.z, (math.copysign(0.165, c.x), 0.04), rounds=1),
                         0.005, "shafts")


def feet(tree):
    bm = bmesh.new()
    faces = {}
    for s in (1, -1):
        stations = [(s * x, y, z) for x, y, z in FOOT]
        plates.tube(bm, faces, tree, stations, (0.0, 0.0, 1.0), FOOT_SIDES, foot_lift,
                    lambda i, c: foot_color(min(max(i, 0), len(FOOT) - 1), c), smooth=3, reach=0.07)
    return plates.finish(bm, faces, lambda c: _nearest(c), 0.005, "feet")


def _nearest(c):
    """The foot's middle nearest `c`."""
    best = min(FOOT, key=lambda p: (abs(c.x) - p[0]) ** 2 + (c.y - p[1]) ** 2)
    return Vector((math.copysign(best[0], c.x), best[1], best[2]))


def cops():
    bm = bmesh.new()
    faces = {}
    for s in (1, -1):
        d = plates.Dome((s * KNEE.x, KNEE.y, KNEE.z), (0.0, -1.0, -0.12), (1.0, 0.0, 0.0), COP)
        plates.dome(bm, faces, d, lambda _ph: 1.25, 3, 10, IRON, rim=BRASS)
    return plates.finish(bm, faces, lambda c: Vector((math.copysign(KNEE.x, c.x), KNEE.y, KNEE.z)), 0.006, "cops")


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = [feet(tree), shafts(tree), cops()]
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
