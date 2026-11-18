# SPDX-License-Identifier: AGPL-3.0-or-later
"""A bark kilt (Cairn's): a broad leather belt round the hips over a band of the team's color,
and hanging from it a kilt of bark slats to above the knee, each its own shade and length, moss
growing over their tops under the belt.

Plates (`mftr_blender.plates`): the belt lathed round the reference body; the slats hang from a
ring stood off the hips below it, flaring out as they fall so they clear the thighs, each slat a
strip of its own with a gap to the next.

    blender -b --python art/wardrobe/waist/bark_kilt/build.py
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

LEATHER = ((0.30, 0.20, 0.12), "cloth")
LEATHER_DARK = ((0.18, 0.12, 0.07), "cloth")
ACCENT = ((0.50, 0.50, 0.50), "accent")
BARK = [((0.26, 0.18, 0.12), "cloth"), ((0.33, 0.24, 0.16), "cloth"), ((0.15, 0.10, 0.07), "cloth"),
        ((0.22, 0.15, 0.10), "cloth")]
MOSS = ((0.16, 0.30, 0.10), "cloth")
MOSS_DARK = ((0.09, 0.18, 0.06), "cloth")

# The belt's rows (m up): dark leather, the team's band, leather, dark leather.
BELT = [0.950, 0.968, 0.990, 1.025, 1.045]
BELT_SIDES = 14
BELT_LIFT = 0.030
# The slats: hung from a ring at TOP stood SLAT_LIFT off the hips, to BOTTOM (each a little
# longer or shorter), widening by FLARE for each metre they fall; moss down to MOSS_TO.
SLATS = 16
TOP, BOTTOM, MOSS_TO = 0.965, 0.665, 0.83
SLAT_LIFT, FLARE = 0.034, 1.05
ROWS = 2
GAP = 0.08          # of each slat's share of the ring, left between it and the next


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = []
    bm, faces = bmesh.new(), {}
    plates.lathe(bm, faces, tree, len(BELT) - 1, [360.0 * c / BELT_SIDES for c in range(BELT_SIDES)],
                 lambda r, _a: BELT[r], lambda z: plates.ray_centre(tree, z, (0.0, 0.0)), lambda _r, _c: BELT_LIFT,
                 lambda r, _c: ACCENT if r == 1 else (LEATHER_DARK if r in (0, 3) else LEATHER), smooth=4)
    objs.append(plates.finish(bm, faces, lambda c: Vector((0.0, 0.0, c.z)), 0.007, "belt"))

    # The ring the slats hang from: fine round the hips, smoothed.
    centre = plates.ray_centre(tree, TOP, (0.0, 0.0))
    n = SLATS * 4
    ring = []
    for k in range(n):
        a = 2 * math.pi * k / n
        d = Vector((math.sin(a), -math.cos(a), 0.0))
        hit, _n, _i, _d = tree.ray_cast(centre, d)
        ring.append((hit - centre).length if hit is not None else 0.15)
    for _ in range(3):
        ring = [(ring[k - 1] + 2 * ring[k] + ring[(k + 1) % n]) / 4 for k in range(n)]

    def at(u, z):
        """The point at `u` round the ring (0 to 1 from the front toward +x) and height `z`."""
        f = u * n
        k = int(math.floor(f)) % n
        t = f - math.floor(f)
        r = ring[k] * (1 - t) + ring[(k + 1) % n] * t + SLAT_LIFT
        a = 2 * math.pi * u
        r *= 1.0 + FLARE * (TOP - z)
        return Vector((centre.x + math.sin(a) * r, centre.y - math.cos(a) * r, z))

    sm, sfaces = bmesh.new(), {}
    for s in range(SLATS):
        u0 = (s + GAP / 2) / SLATS
        u1 = (s + 1 - GAP / 2) / SLATS
        bottom = BOTTOM + 0.035 * ((s * 7) % 3) - (0.02 if s % 5 == 2 else 0.0)
        zs = [TOP + (bottom - TOP) * r / ROWS for r in range(ROWS + 1)]
        grid = [[sm.verts.new(at(u, z)) for u in (u0, u1)] for z in zs]
        for r in range(ROWS):
            mid = (zs[r] + zs[r + 1]) / 2
            for c in range(1):
                if mid > MOSS_TO:
                    col = MOSS if (s + c) % 3 else MOSS_DARK
                else:
                    col = BARK[s % 4]
                f = sm.faces.new((grid[r][c], grid[r][c + 1], grid[r + 1][c + 1], grid[r + 1][c]))
                sfaces[f] = col
    objs.append(plates.finish(sm, sfaces, lambda c: Vector((centre.x, centre.y, c.z)), 0.008, "slats"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
