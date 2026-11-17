# SPDX-License-Identifier: AGPL-3.0-or-later
"""A priestess's boots (Lumen's): soft ivory leather to above the ankle, thin pale soles, two gold
straps across the foot, a gold cuff, and a gold anklet above it.

Made as the veteran's boots are (`mftr_blender.plates`): the feet are tubes from the heel to the
toes round the reference feet, the shafts lathed round each leg from over the foot's top to above
the ankle. (Open sandals came first, a sole and straps; on the slim masculine base the skin by
their edges came through them, 22 §4.8.)

    blender -b --python art/wardrobe/feet/priestess_boots/build.py
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

IVORY = ((0.60, 0.57, 0.52), "cloth")
SOLE = ((0.42, 0.34, 0.24), "cloth")
GOLD = ((0.70, 0.54, 0.22), "metal")

# The foot's middle, heel to toes (the veteran's boots': the CC0 male's right foot).
FOOT = [(0.180, 0.095, 0.045), (0.186, 0.04, 0.045), (0.210, -0.005, 0.036), (0.226, -0.05, 0.026),
        (0.229, -0.09, 0.016), (0.228, -0.122, 0.010)]
SIDES = 8
# Stations along the foot (0 the heel, 1 the toes); the rows between those listed in STRAPS are
# gold straps across it.
ALONG = [0.0, 0.30, 0.36, 0.56, 0.68, 0.73, 1.0]
STRAPS = {1, 4}

# The shaft's rings (m up), from over the foot's top to the cuff; the cuff's row gold, and an
# anklet above it, apart.
RINGS = [0.085, 0.12, 0.155, 0.17]
ANKLET = (0.20, 0.212)
SHAFT_SIDES = 8


def along(t):
    """The foot's path at `t` (0 the heel, 1 the toes), by length."""
    pts = [Vector(p) for p in FOOT]
    lens = [(b - a).length for a, b in zip(pts, pts[1:])]
    d = t * sum(lens)
    for a, b, n in zip(pts, pts[1:], lens):
        if d <= n:
            return a.lerp(b, d / n)
        d -= n
    return pts[-1]


def bottom(c):
    return math.cos(2 * math.pi * (c + 0.5) / SIDES) < -0.55


def foot_color(i, c):
    if bottom(c):
        return SOLE
    if i in STRAPS:
        return GOLD
    return IVORY


def foot_lift(i, c):
    t = ALONG[min(max(i, 0), len(ALONG) - 1)]
    # Soft leather, close; a little fuller at the heel and over the toes.
    return 0.013 + (0.004 if bottom(c) else 0.0) + 0.008 * max(0.0, t - 0.6) / 0.4 + (0.005 if t <= 0.0 else 0.0)


def ring_lift(r):
    z = RINGS[r]
    lift = 0.014
    if z <= 0.12:
        # Over the foot's top edge, which it covers.
        lift += 0.008 * (0.12 - z) / 0.035
    return lift


def nearest_mid(c):
    best = min(FOOT, key=lambda p: (abs(c.x) - p[0]) ** 2 + (c.y - p[1]) ** 2)
    return Vector((math.copysign(best[0], c.x), best[1], best[2]))


def leg_centre(tree, z, s):
    return plates.ray_centre(tree, z, (s * 0.165, 0.04))


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = []
    bm, faces = bmesh.new(), {}
    for s in (1, -1):
        stations = [tuple(along(t) * Vector((s, 1, 1))) for t in ALONG]
        plates.tube(bm, faces, tree, stations, (0.0, 0.0, 1.0), SIDES, foot_lift,
                    lambda i, c: foot_color(min(max(i, 0), len(ALONG) - 2), c), smooth=3, reach=0.07)
    objs.append(plates.finish(bm, faces, nearest_mid, 0.005, "feet"))
    bm, faces = bmesh.new(), {}
    for s in (1, -1):
        centres = {z: leg_centre(tree, z, s) for z in RINGS + list(ANKLET)}
        plates.lathe(bm, faces, tree, len(RINGS) - 1, [360.0 * c / SHAFT_SIDES for c in range(SHAFT_SIDES)],
                     lambda r, _a: RINGS[r], lambda z, centres=centres: centres[z], lambda r, _c: ring_lift(r),
                     lambda r, _c: GOLD if r == len(RINGS) - 2 else IVORY, smooth=4)
        plates.lathe(bm, faces, tree, 1, [360.0 * c / SHAFT_SIDES for c in range(SHAFT_SIDES)],
                     lambda r, _a: ANKLET[r], lambda z, centres=centres: centres[z], lambda _r, _c: 0.010,
                     lambda _r, _c: GOLD, smooth=2)
    objs.append(plates.finish(bm, faces, lambda c: leg_centre(tree, c.z, math.copysign(1.0, c.x)), 0.004, "shafts"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
