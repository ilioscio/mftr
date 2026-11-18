# SPDX-License-Identifier: AGPL-3.0-or-later
"""A rag skirt (Marrow's): the long skirt of her robe, torn into strips from the hips to the
shins, each strip its own length and color (plum, darker plum, grave grey), its end torn on a
slant, flaring as they fall.

Plates (`mftr_blender.plates`), as the bark kilt's slats are: the strips hang from a ring stood
off the hips under the corset's foot, flaring out so they clear the thighs; side by side they
overlap a little (every other one stands further out), so no leg shows between them. They swing
on spring-bone chains round the hips.

    blender -b --python art/wardrobe/legs/rag_skirt/build.py
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

RAGS = [((0.24, 0.08, 0.20), "cloth"), ((0.12, 0.04, 0.11), "cloth"), ((0.24, 0.24, 0.26), "cloth"),
        ((0.18, 0.06, 0.15), "cloth")]
# Which rag each strip is, round from the front: mostly plum, now and then grey.
ORDER = [0, 1, 0, 3, 2, 0, 1, 2, 0, 3, 1, 0, 3, 2, 0, 1, 3, 1]
STRIPS = len(ORDER)
# Hung from a ring at TOP stood LIFT off the hips, to about BOTTOM (each its own length), widening
# by FLARE for each metre they fall; every other strip OVER further out, each WIDE of its share.
TOP, BOTTOM = 0.975, 0.30
LIFT, FLARE = 0.030, 0.60
OVER, WIDE = 0.008, 1.18
ROWS = 5
THICKNESS = 0.005


def length(s):
    """How much shorter or longer than BOTTOM strip `s` ends: torn, not cut."""
    return 0.07 * (((s * 7) % 5) / 4 - 0.5) + (0.04 if s % 6 == 4 else 0.0)


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])

    # The ring the strips hang from: fine round the hips, smoothed.
    centre = plates.ray_centre(tree, TOP, (0.0, 0.0))
    n = STRIPS * 4
    ring = []
    for k in range(n):
        a = 2 * math.pi * k / n
        d = Vector((math.sin(a), -math.cos(a), 0.0))
        # The furthest the hips and the seat stand out below the ring, as the flare would carry
        # it: the strips fall clear of the buttocks, not through them.
        need = 0.0
        for j in range(10):
            z = TOP - 0.025 * j
            o = Vector((centre.x, centre.y, z))
            hit, _n, _i, _d = tree.ray_cast(o, d)
            if hit is not None:
                need = max(need, ((hit - o).length + LIFT) / (1.0 + FLARE * (TOP - z)) - LIFT)
        ring.append(need if need > 0 else 0.15)
    for _ in range(3):
        ring = [(ring[k - 1] + 2 * ring[k] + ring[(k + 1) % n]) / 4 for k in range(n)]

    def at(u, z, out):
        """The point at `u` round the ring (0 to 1 from the front toward +x), height `z`."""
        f = (u % 1.0) * n
        k = int(math.floor(f)) % n
        t = f - math.floor(f)
        r = ring[k] * (1 - t) + ring[(k + 1) % n] * t + LIFT + out
        a = 2 * math.pi * u
        r *= 1.0 + FLARE * (TOP - z)
        return Vector((centre.x + math.sin(a) * r, centre.y - math.cos(a) * r, z))

    bm, faces = bmesh.new(), {}
    for s in range(STRIPS):
        half = WIDE / 2 / STRIPS
        mid = (s + 0.5) / STRIPS
        u0, u1 = mid - half, mid + half
        out = OVER if s % 2 else 0.0
        bottom = BOTTOM + length(s)
        # Torn on a slant: one corner lower than the other.
        slant = 0.05 * (1 if (s * 5) % 3 else -1)
        grid = []
        for r in range(ROWS + 1):
            t = r / ROWS
            row = []
            for side, u in ((0, u0), (1, u1)):
                end = bottom + (slant / 2 if side else -slant / 2)
                z = TOP + (end - TOP) * t
                # The strip narrows a little toward its torn end.
                uu = mid + (u - mid) * (1.0 - 0.25 * t ** 2)
                row.append(bm.verts.new(at(uu, z, out)))
            grid.append(row)
        for r in range(ROWS):
            f = bm.faces.new((grid[r][0], grid[r][1], grid[r + 1][1], grid[r + 1][0]))
            faces[f] = RAGS[ORDER[s]]
    obj = plates.finish(bm, faces, lambda c: Vector((centre.x, centre.y, c.z)), THICKNESS, "rags")
    bpy.data.objects.remove(body)
    plates.save([obj], os.path.join(HERE, "item.blend"))


build()
