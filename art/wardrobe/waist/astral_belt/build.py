# SPDX-License-Identifier: AGPL-3.0-or-later
"""An astral belt (Quill's): a brown leather belt over her coat on a band of the team's color, a
square brass buckle; a leather scroll case slung at the right hip and a brass spyglass at the left.

Plates (`mftr_blender.plates`), measured on the CC0 male: the belt lathed round the waist (the
ember sash's); the buckle a flat dome; the case and the spyglass closed tubes hung from the belt
by short straps, slanting forward.

    blender -b --python art/wardrobe/waist/astral_belt/build.py
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

LEATHER = ((0.28, 0.18, 0.10), "cloth")
LEATHER_DARK = ((0.16, 0.10, 0.06), "cloth")
ACCENT = ((0.50, 0.50, 0.50), "accent")
BRASS = ((0.68, 0.50, 0.22), "metal")
GLASS = ((0.80, 0.70, 1.0), "emissive")

# The belt's rows (m up), over a coat: leather, the team's band, leather.
BELT = [0.950, 0.970, 1.000, 1.024]
SIDES = 16
LIFT = 0.030
BUCKLE = 0.026


def tube(bm, faces, a, b, radius, rings, sides=6):
    """A closed tube from `a` to `b`: [(t, color)] rings along it, each band its color."""
    axis = (b - a).normalized()
    u = axis.orthogonal().normalized()
    w = axis.cross(u)
    loops = []
    for t, _c in rings:
        c = a.lerp(b, t)
        loops.append([bm.verts.new(c + (u * math.cos(2 * math.pi * k / sides) + w * math.sin(2 * math.pi * k / sides)) * radius)
                      for k in range(sides)])
    for (lo, hi), (_t, color) in zip(zip(loops, loops[1:]), rings):
        for k in range(sides):
            faces[bm.faces.new((lo[k], lo[(k + 1) % sides], hi[(k + 1) % sides], hi[k]))] = color
    faces[bm.faces.new(list(reversed(loops[0])))] = rings[0][1]
    faces[bm.faces.new(loops[-1])] = rings[-2][1]
    return loops


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = []
    bm, faces = bmesh.new(), {}
    plates.lathe(bm, faces, tree, len(BELT) - 1, [360.0 * c / SIDES for c in range(SIDES)],
                 lambda r, _a: BELT[r], lambda z: plates.ray_centre(tree, z, (0.0, 0.0)), lambda _r, _c: LIFT,
                 lambda r, _c: ACCENT if r == 1 else LEATHER, smooth=4)
    objs.append(plates.finish(bm, faces, lambda c: Vector((0.0, 0.0, c.z)), 0.006, "belt"))

    bm, faces = bmesh.new(), {}
    mid = (BELT[0] + BELT[-1]) / 2
    hit, _n, _i, _d = tree.ray_cast(Vector((0.0, -1.0, mid)), Vector((0, 1, 0)))
    front = hit - Vector((0.0, LIFT + 0.002, 0.0))
    d = plates.Dome(front, (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (0.008, BUCKLE, BUCKLE * 0.85))
    plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, 4, BRASS)
    objs.append(plates.finish(bm, faces, lambda c: Vector((c.x, 0.0, c.z)), 0.004, "buckle"))

    # The scroll case (right hip, -x) and the spyglass (left, +x), hung off the belt's side.
    bm, faces = bmesh.new(), {}
    o = plates.ray_centre(tree, BELT[0], (0.0, 0.0))
    for s, angle in ((-1, 255.0), (1, 105.0)):
        d = Vector((math.sin(math.radians(angle)), -math.cos(math.radians(angle)), 0.0))
        hit, _n, _i, _d = tree.ray_cast(Vector((o.x, o.y, BELT[0])), d)
        hang = hit + d * (LIFT + 0.030)
        if s < 0:
            # The case: a leather tube, brass-capped, slanting forward and down.
            a = hang + Vector((0.0, 0.03, 0.02))
            b = hang + Vector((0.0, -0.08, -0.24))
            tube(bm, faces, a, b, 0.030, [(0.0, BRASS), (0.08, LEATHER), (0.45, LEATHER_DARK), (0.52, LEATHER),
                                          (0.92, BRASS), (1.0, BRASS)], sides=6)
        else:
            # The spyglass: brass, two draws, a glinting lens at its front end.
            a = hang + Vector((0.0, 0.05, 0.01))
            b = hang + Vector((0.0, -0.12, -0.17))
            tube(bm, faces, a, a.lerp(b, 0.55), 0.020, [(0.0, BRASS), (0.15, LEATHER_DARK), (0.85, BRASS), (1.0, BRASS)])
            tube(bm, faces, a.lerp(b, 0.5), b, 0.025, [(0.0, BRASS), (0.9, BRASS), (1.0, GLASS)])
    objs.append(plates.finish(bm, faces, None, None, "gear"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
