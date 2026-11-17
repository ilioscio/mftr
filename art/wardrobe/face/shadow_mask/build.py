# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shadow mask (Shade's): a smooth black mask over the whole face, from the brow to under the
chin, with two narrow slits of violet burning where the eyes are, their outer corners raised.

Worn on the head bone, rigid, scaled to each base's head (`grip_scale`), as the shadow hood is;
fitted to the slim masculine base's face by rays out from the skull's middle, stood off the skin
to clear the nose and the eyes. Given thickness by `mftr_blender.plates`.

Its frame is the head bone's at its joint: +Z up the head, +Y toward the back, +X the
character's left.

    blender -b --python art/wardrobe/face/shadow_mask/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

from mftr_blender import plates  # noqa: E402

BLACK = ((0.06, 0.055, 0.09), "cloth")
VIOLET = ((0.78, 0.36, 1.0), "emissive")

BASE = os.path.join(ROOT, "art", "creator", "humanoid_slim", "humanoid_slim_masculine.blend")
CENTRE = Vector((0.0, 0.012, 0.115))
# Rows by elevation (degrees) from under the chin to the brow; columns by azimuth from the front.
ELEVATIONS = [-52, -36, -20, -6, 8, 24]
AZIMUTHS = [-75.0 + 15.0 * i for i in range(11)]
LIFT = 0.010
# The eyes (the base's eye bones, in this frame), and each slit's half-width and half-height
# (degrees round the skull's middle) and the rise of its outer corner.
EYES = [Vector((0.036, -0.071, 0.108)), Vector((-0.036, -0.071, 0.108))]
SLIT_W, SLIT_H, SLIT_RISE = 12.0, 2.8, 4.0


def head_tree():
    bpy.ops.wm.open_mainfile(filepath=BASE)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    inv = (arm.matrix_world @ arm.data.bones["head"].matrix_local).inverted()
    to_item = lambda p: Vector((p.x, -p.z, p.y))  # noqa: E731
    body = next(o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("part."))
    pts = [to_item(inv @ (body.matrix_world @ v.co)) for v in body.data.vertices]
    return BVHTree.FromPolygons(pts, [tuple(p.vertices) for p in body.data.polygons])


def ray(e, a):
    e, a = math.radians(e), math.radians(a)
    return Vector((math.sin(a) * math.cos(e), -math.cos(a) * math.cos(e), math.sin(e)))


def grid_of(tree, elevations, azimuths):
    grid = []
    for e in elevations:
        row = []
        for a in azimuths:
            d = ray(e, a)
            hit, _n, _i, _d = tree.ray_cast(CENTRE, d)
            row.append([CENTRE, d, (hit - CENTRE).length if hit is not None else 0.11])
        grid.append(row)
    return grid


def build():
    tree = head_tree()
    bm, faces = bmesh.new(), {}
    grid = grid_of(tree, ELEVATIONS, AZIMUTHS)
    plates._smooth(grid, False, 3)
    surface = {}
    verts = []
    for r, row in enumerate(grid):
        verts.append([bm.verts.new(o + d * (dist + LIFT)) for o, d, dist in row])
        for c, (_o, _d, dist) in enumerate(row):
            surface[(ELEVATIONS[r], AZIMUTHS[c])] = dist + LIFT
    for r in range(len(ELEVATIONS) - 1):
        for c in range(len(AZIMUTHS) - 1):
            faces[bm.faces.new((verts[r][c], verts[r][c + 1], verts[r + 1][c + 1], verts[r + 1][c]))] = BLACK
    mask = plates.finish(bm, faces, lambda _p: CENTRE, 0.004, "mask")

    def at(e, a):
        """A point a hair out of the mask's face, where (e, a) meets it (its distance read off
        the mask's own grid)."""
        rows = [x for x in ELEVATIONS]
        r = max(i for i in range(len(rows) - 1) if rows[i] <= e) if e >= rows[0] else 0
        c = max(i for i in range(len(AZIMUTHS) - 1) if AZIMUTHS[i] <= a)
        u = (e - rows[r]) / (rows[r + 1] - rows[r])
        v = (a - AZIMUTHS[c]) / (AZIMUTHS[c + 1] - AZIMUTHS[c])
        d = ((1 - u) * (1 - v) * surface[(rows[r], AZIMUTHS[c])] + (1 - u) * v * surface[(rows[r], AZIMUTHS[c + 1])]
             + u * (1 - v) * surface[(rows[r + 1], AZIMUTHS[c])] + u * v * surface[(rows[r + 1], AZIMUTHS[c + 1])])
        return CENTRE + ray(e, a) * (d + 0.0025)

    # The slits: a narrow diamond on the mask over each eye, its outer corner raised.
    bm, faces = bmesh.new(), {}
    for eye in EYES:
        d = (eye - CENTRE).normalized()
        a0 = math.degrees(math.atan2(d.x, -d.y))
        e0 = math.degrees(math.asin(d.z))
        out = 1.0 if a0 > 0 else -1.0
        inner = at(e0 - SLIT_RISE * 0.3, a0 - out * SLIT_W)
        outer = at(e0 + SLIT_RISE, a0 + out * SLIT_W)
        top = at(e0 + SLIT_H + SLIT_RISE * 0.45, a0 + out * SLIT_W * 0.15)
        bottom = at(e0 - SLIT_H + SLIT_RISE * 0.2, a0 + out * SLIT_W * 0.15)
        vs = [bm.verts.new(p) for p in (inner, bottom, outer, top)]
        faces[bm.faces.new(vs)] = VIOLET
    slits = plates.finish(bm, faces, lambda _p: CENTRE, 0.002, "slits")
    plates.save([mask, slits], os.path.join(HERE, "item.blend"))


build()
