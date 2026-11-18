# SPDX-License-Identifier: AGPL-3.0-or-later
"""Ember's boots: slim soot-black leather to under the knee, pointed toes turning up a little,
thin soles, a gold hem round the top and a gold plate domed over each knee.

Made as the shadow boots are (`feet/shadow_boots`, `mftr_blender.plates`): the feet are tubes
from the heel to the toes round the reference feet, the shafts lathed round each leg from over
the foot's top, a little fuller toward the top; the points are closed pieces past the toes, the
knee plates domes in front of the knees (the veteran's boots' cops, smaller).

    blender -b --python art/wardrobe/feet/ember_boots/build.py
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

BLACK = ((0.05, 0.045, 0.045), "cloth")
SOLE = ((0.11, 0.07, 0.05), "cloth")
GOLD = ((0.60, 0.40, 0.12), "metal")

# The foot's middle, heel to toes (the veteran's boots': the CC0 male's right foot).
FOOT = [(0.180, 0.095, 0.045), (0.186, 0.04, 0.045), (0.210, -0.005, 0.036), (0.226, -0.05, 0.026),
        (0.229, -0.09, 0.016), (0.228, -0.122, 0.010)]
SIDES = 8
# Stations along the foot (0 the heel, 1 the toes).
ALONG = [0.0, 0.30, 0.56, 0.73, 1.0]
# The point past the toes: how far forward and up its tip is from the toes' station, and the
# half-width and half-height of its base there.
POINT = (0.055, 0.018)
POINT_BASE = (0.044, 0.026)

# The shaft's rings (m up), from over the foot's top to under the knee, a gold hem at the top.
RINGS = [0.085, 0.13, 0.24, 0.35, 0.415, 0.435]
SHAFT_SIDES = 8
# The knee plates: a shallow cap in front of each reference knee (its middle at x 0.125, y 0.016,
# z 0.485), a little behind and below it so the cap lies flat over the kneecap.
KNEE = Vector((0.125, 0.030, 0.470))
PLATE = (0.098, 0.056, 0.074)   # along its axis (forward), across, and up


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
    return BLACK


def foot_lift(i, c):
    t = ALONG[min(max(i, 0), len(ALONG) - 1)]
    # Soft leather, close; a little fuller at the heel and over the toes.
    return 0.013 + (0.004 if bottom(c) else 0.0) + 0.008 * max(0.0, t - 0.6) / 0.4 + (0.005 if t <= 0.0 else 0.0)


def ring_lift(r):
    z = RINGS[r]
    lift = 0.012 + 0.010 * max(0.0, z - 0.13) / 0.30
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
        centres = {z: leg_centre(tree, z, s) for z in RINGS}
        plates.lathe(bm, faces, tree, len(RINGS) - 1, [360.0 * c / SHAFT_SIDES for c in range(SHAFT_SIDES)],
                     lambda r, _a: RINGS[r], lambda z, centres=centres: centres[z], lambda r, _c: ring_lift(r),
                     lambda r, _c: GOLD if r == len(RINGS) - 2 else BLACK, smooth=4)
    objs.append(plates.finish(bm, faces, lambda c: leg_centre(tree, c.z, math.copysign(1.0, c.x)), 0.004, "shafts"))
    # The points: four-sided, closed, from inside the toes forward and a little up.
    bm, faces = bmesh.new(), {}
    for s in (1, -1):
        toe = along(0.86) * Vector((s, 1, 1)) + Vector((0.0, 0.0, 0.004))
        tip = along(1.0) * Vector((s, 1, 1)) + Vector((0.0, -POINT[0], POINT[1]))
        w, h = POINT_BASE
        base = [bm.verts.new(toe + Vector(v)) for v in ((w, 0, h), (-w, 0, h), (-w, 0, -h), (w, 0, -h))]
        t = bm.verts.new(tip)
        for k in range(4):
            faces[bm.faces.new((base[k], base[(k + 1) % 4], t))] = BLACK if k != 2 else SOLE
        faces[bm.faces.new(list(reversed(base)))] = BLACK
    objs.append(plates.finish(bm, faces, None, None, "points"))
    # The knee plates.
    bm, faces = bmesh.new(), {}
    for s in (1, -1):
        d = plates.Dome((s * KNEE.x, KNEE.y, KNEE.z), (0.0, -1.0, -0.12), (1.0, 0.0, 0.0), PLATE)
        plates.dome(bm, faces, d, lambda _ph: 0.95, 3, 8, GOLD)
    objs.append(plates.finish(bm, faces, lambda c: Vector((math.copysign(KNEE.x, c.x), KNEE.y, KNEE.z)), 0.004, "plates"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
