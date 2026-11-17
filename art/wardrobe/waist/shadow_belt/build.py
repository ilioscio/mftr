# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shadow belt (Shade's): black leather round the hips over a band of the team's color, and a
strap round each thigh with three kunai in it along the outside, their points down.

Plates (`mftr_blender.plates`): the belt and the straps lathed round the reference body, the
kunai small four-sided blades and grips stood off each strap.

    blender -b --python art/wardrobe/waist/shadow_belt/build.py
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

LEATHER = ((0.07, 0.06, 0.08), "cloth")
ACCENT = ((0.50, 0.50, 0.50), "accent")
STEEL = ((0.42, 0.44, 0.50), "metal")
WRAP = ((0.24, 0.21, 0.28), "cloth")

# The belt's rows (m up), over a jerkin's hem: leather, the team's band, leather.
BELT = [0.955, 0.975, 1.010, 1.030]
BELT_SIDES = 16
BELT_LIFT = 0.026
# The thigh straps: their rows, round each thigh's middle; the kunai's angles round from the
# front toward the outside (degrees), and their length above and below the strap.
STRAP = [0.615, 0.645]
STRAP_SIDES = 10
STRAP_LIFT = 0.020
KUNAI = [60.0, 90.0, 120.0]
GRIP_UP, BLADE_DOWN = 0.075, 0.085


def kunai(bm, faces, at, out, size=1.0):
    """A kunai standing `out` (unit, level) off the strap at `at`, its point down: a wrapped grip
    and a ring above, a leaf blade below."""
    up = Vector((0.0, 0.0, 1.0))
    side = up.cross(out).normalized()
    root = at + out * 0.008

    def prism(z0, z1, w, t, color):
        ring = lambda z: [bm.verts.new(root + up * z + side * (w * sx) + out * (t * sy))  # noqa: E731
                          for sx, sy in ((1, 1), (-1, 1), (-1, -1), (1, -1))]
        lo, hi = ring(z0), ring(z1)
        for k in range(4):
            faces[bm.faces.new((lo[k], lo[(k + 1) % 4], hi[(k + 1) % 4], hi[k]))] = color
        faces[bm.faces.new(list(reversed(lo)))] = color
        faces[bm.faces.new(hi)] = color

    prism(0.010, GRIP_UP, 0.007, 0.006, WRAP)
    prism(GRIP_UP, GRIP_UP + 0.018, 0.010, 0.003, STEEL)
    # The blade: from the guard down to its point, widest a third of the way.
    w = 0.016
    top = [bm.verts.new(root + up * 0.010 + v) for v in (side * w * 0.6, out * 0.004, -side * w * 0.6, -out * 0.004)]
    mid = [bm.verts.new(root - up * (BLADE_DOWN * 0.35) + v) for v in (side * w, out * 0.005, -side * w, -out * 0.005)]
    tip = bm.verts.new(root - up * BLADE_DOWN)
    for k in range(4):
        faces[bm.faces.new((top[k], top[(k + 1) % 4], mid[(k + 1) % 4], mid[k]))] = STEEL
        faces[bm.faces.new((mid[(k + 1) % 4], mid[k], tip))] = STEEL
    faces[bm.faces.new(top)] = STEEL


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = []
    bm, faces = bmesh.new(), {}
    plates.lathe(bm, faces, tree, len(BELT) - 1, [360.0 * c / BELT_SIDES for c in range(BELT_SIDES)],
                 lambda r, _a: BELT[r], lambda z: plates.ray_centre(tree, z, (0.0, 0.0)), lambda _r, _c: BELT_LIFT,
                 lambda r, _c: ACCENT if r == 1 else LEATHER, smooth=4)
    objs.append(plates.finish(bm, faces, lambda c: Vector((0.0, 0.0, c.z)), 0.006, "belt"))
    bm, faces = bmesh.new(), {}
    km, kfaces = bmesh.new(), {}
    centres = []
    for s in (1, -1):
        cz = {z: plates.ray_centre(tree, z, (s * 0.10, 0.0)) for z in STRAP}
        centres += list(cz.values())
        angles = [360.0 * c / STRAP_SIDES for c in range(STRAP_SIDES)]
        plates.lathe(bm, faces, tree, 1, angles, lambda r, _a: STRAP[r], lambda z, cz=cz: cz[z],
                     lambda _r, _c: STRAP_LIFT, lambda _r, _c: LEATHER, smooth=2)
        mid = (STRAP[0] + STRAP[1]) / 2
        o = plates.ray_centre(tree, mid, (s * 0.10, 0.0))
        for a in KUNAI:
            d = Vector((s * math.sin(math.radians(a)), -math.cos(math.radians(a)), 0.0))
            hit, _n, _i, _d = tree.ray_cast(o, d)
            at = (hit if hit is not None else o + d * 0.08) + d * (STRAP_LIFT + 0.004)
            kunai(km, kfaces, at, d)

    def thigh(c):
        return min(centres, key=lambda p: (p - c).length)
    objs.append(plates.finish(bm, faces, thigh, 0.005, "straps"))
    objs.append(plates.finish(km, kfaces, thigh, None, "kunai"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
