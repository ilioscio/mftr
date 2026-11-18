# SPDX-License-Identifier: AGPL-3.0-or-later
"""A feather belt (Wren's): a leather belt on a band of the team's color with an iron buckle, a fan
of russet and tan tail feathers tipped in lime spread from its back, and a leather quiver of bolts
at the right hip, their fletching glowing lime.

Plates (`mftr_blender.plates`), measured on the CC0 male: the belt lathed round the hips (the
astral belt's); the feathers closed blades with a ridge down their middle (the armory's), fanned
down and out from the belt's back; the quiver a closed tube slanting forward, the bolts thin shafts
out of its top.

    blender -b --python art/wardrobe/waist/feather_belt/build.py
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

LEATHER = ((0.28, 0.17, 0.09), "cloth")
LEATHER_DARK = ((0.16, 0.10, 0.05), "cloth")
ACCENT = ((0.50, 0.50, 0.50), "accent")
IRON = ((0.34, 0.35, 0.38), "metal")
RUSSET = ((0.44, 0.17, 0.07), "cloth")
TAN = ((0.54, 0.40, 0.24), "cloth")
LIME = ((0.42, 0.62, 0.10), "cloth")
GLOW = ((0.66, 1.0, 0.20), "emissive")
WOOD = ((0.32, 0.20, 0.10), "cloth")

BELT = [0.935, 0.955, 0.985, 1.008]
SIDES = 16
LIFT = 0.030
BUCKLE = 0.022
# The tail fan: feathers from the belt's back, each (degrees out from straight down, length).
FAN = [(-62, 0.21), (-38, 0.27), (-13, 0.30), (13, 0.30), (38, 0.27), (62, 0.21)]
FAN_OUT = 0.55          # how far back from straight down they splay (0 down, 1 straight back)
HALF = 0.026
# The quiver at the right hip.
QUIVER_AT = 250.0
QUIVER_LEN, QUIVER_R = 0.26, 0.032


def blade(bm, faces, root, d, across, stations, color, ridge=0.004):
    """A closed blade from `root` along `d`, `across` its width: [(along, half-width)], 0 a point."""
    thick = d.cross(across).normalized()
    rings = []
    for z, w in stations:
        c = root + d * z
        if w <= 1e-6:
            rings.append([bm.verts.new(c)])
            continue
        t = ridge * (w / stations[0][1]) ** 0.5 if stations[0][1] else ridge
        rings.append([bm.verts.new(c + v) for v in (across * w, thick * t, -across * w, -thick * t)])
    for lo, hi in zip(rings, rings[1:]):
        if len(hi) == 1:
            for k in range(4):
                faces[bm.faces.new((lo[k], lo[(k + 1) % 4], hi[0]))] = color
        else:
            for k in range(4):
                faces[bm.faces.new((lo[k], lo[(k + 1) % 4], hi[(k + 1) % 4], hi[k]))] = color
    faces[bm.faces.new(list(reversed(rings[0])))] = color


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
    d = plates.Dome(front, (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (0.008, BUCKLE, BUCKLE * 0.8))
    plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, 4, IRON)

    # The tail fan, from the belt's back.
    hit, _n, _i, _d = tree.ray_cast(Vector((0.0, 1.0, mid)), Vector((0, -1, 0)))
    root = hit + Vector((0.0, LIFT + 0.012, 0.0))
    for k, (angle, length) in enumerate(FAN):
        a = math.radians(angle)
        down = Vector((math.sin(a), 0.0, -math.cos(a)))
        dd = (down * (1 - FAN_OUT) + Vector((0.0, FAN_OUT, 0.0))).normalized()
        across = dd.cross(Vector((0.0, 1.0, 0.0))).normalized()
        r0 = root + Vector((0.0, 0.004 * (k % 2), 0.0))
        blade(bm, faces, r0, dd, across, [(0.0, HALF * 0.35), (length * 0.3, HALF), (length * 0.72, HALF * 0.9),
                                           (length, 0.0)], TAN if k % 2 else RUSSET)
        blade(bm, faces, r0, dd, across, [(length * 0.72, HALF * 0.95), (length * 0.88, HALF * 0.6), (length * 1.01, 0.0)],
              LIME, ridge=0.0055)

    # The quiver at the right hip, slanting forward, and its bolts.
    o = plates.ray_centre(tree, BELT[0], (0.0, 0.0))
    d = Vector((math.sin(math.radians(QUIVER_AT)), -math.cos(math.radians(QUIVER_AT)), 0.0))
    hit, _n, _i, _d = tree.ray_cast(Vector((o.x, o.y, BELT[0])), d)
    top = hit + d * (LIFT + QUIVER_R + 0.012) + Vector((0.0, 0.02, 0.0))
    bottom = top + Vector((0.0, -0.07, -QUIVER_LEN))
    tube(bm, faces, bottom, top, QUIVER_R, [(0.0, LEATHER_DARK), (0.08, LEATHER), (0.85, LEATHER_DARK),
                                            (0.93, LEATHER), (1.0, LEATHER)], sides=6)
    axis = (top - bottom).normalized()
    side = axis.cross(Vector((1.0, 0.0, 0.0))).normalized()
    for k, off in enumerate((-0.012, 0.0, 0.012, 0.006)):
        base = top + side * off + Vector((0.0, 0.006 * (k - 1), 0.0))
        tip = base + axis * (0.11 + 0.015 * (k % 2))
        tube(bm, faces, base - axis * 0.02, tip, 0.0035, [(0.0, WOOD), (0.6, WOOD), (1.0, WOOD)], sides=3)
        blade(bm, faces, tip - axis * 0.055, axis, side, [(0.0, 0.013), (0.04, 0.016), (0.06, 0.0)], GLOW, ridge=0.003)
    objs.append(plates.finish(bm, faces, None, None, "gear"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
