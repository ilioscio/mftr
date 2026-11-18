# SPDX-License-Identifier: AGPL-3.0-or-later
"""Astral capes (Quill's): two short capes of violet wool, one over the other, round the neck and
over the shoulders, each hemmed in darker violet; violet lapels down the chest in a V and two
rows of brass buttons between them.

Plates (`mftr_blender.plates`), measured on the CC0 male: each cape lathed round the body as the
grave shawl is, each ray cast in from outside so it lies over the arms; the upper cape shorter and
standing further out. The lapels are ribbons laid on the body from the front, the buttons small
domes (the ember collar's).

    blender -b --python art/wardrobe/shoulders/astral_capes/build.py
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

VIOLET = ((0.30, 0.14, 0.44), "cloth")
VIOLET_DARK = ((0.18, 0.08, 0.28), "cloth")
BRASS = ((0.68, 0.50, 0.22), "metal")

TOP, DIP = 1.448, 0.045
COLS = 20
# The capes, under first: (hem in front, at the sides, at the back; rows; lift at the top and hem).
CAPES = [((1.215, 1.205, 1.165), 5, 0.018, 0.034), ((1.300, 1.285, 1.255), 3, 0.026, 0.046)]
SCALLOP = 0.012                     # the hem's soft points
SHOULDER = 0.010                    # further out over the shoulders (a slim feminine base's deltoids)
THICKNESS = 0.005
# The lapels, the right one (the left mirrors it): from under the capes down to the waist.
LAPEL = [(0.050, 1.25), (0.072, 1.18), (0.060, 1.10), (0.030, 1.00)]
LAPEL_WIDTH = 0.040
BUTTONS = [(0.034, 1.175), (0.030, 1.115), (0.026, 1.055)]


def top_z(a):
    return TOP - DIP * ((1 + math.cos(math.radians(a))) / 2) ** 2


def stud(bm, faces, tree, at, lift, size, color, sides=6):
    """A small dome on the body's front at (x, z), `lift` off the skin."""
    hit, _n, _i, _d = tree.ray_cast(Vector((at[0], -1.0, at[1])), Vector((0, 1, 0)))
    d = plates.Dome(hit - Vector((0, lift, 0)), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (size * 0.6, size, size))
    plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, sides, color)


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    centres = {}

    def centre(z):
        k = round(z, 3)
        if k not in centres:
            centres[k] = plates.ray_centre(tree, max(z, 1.30), (0.0, 0.0))
        c = centres[k]
        return Vector((c.x, c.y, z))

    angles = [360.0 * c / COLS for c in range(COLS)]
    objs = []
    for (front, side, back), rows, lift_top, lift_hem in CAPES:
        def hem_z(a, front=front, side=side, back=back):
            k = (1 - math.cos(math.radians(a))) / 2
            s = math.sin(math.radians(a)) ** 2
            z = front + (side - front) * s
            z += (back - z) * max(0.0, (k - 0.5) / 0.5)
            t = (a / 360.0) * 14
            return z - SCALLOP * (1 - abs((t % 1) * 2 - 1))

        grid = []
        for r in range(rows + 1):
            row = []
            for a in angles:
                z = top_z(a) + (hem_z(a) - top_z(a)) * (r / rows) ** 0.9
                d = Vector((math.sin(math.radians(a)), -math.cos(math.radians(a)), 0.0))
                o = centre(z)
                hit, _n, _i, _d = tree.ray_cast(o + d * 0.8, -d)
                dist = (hit - o).dot(d) if hit is not None else 0.12
                row.append([o, d, max(dist, 0.04)])
            grid.append(row)
        plates._smooth(grid, True, 6)
        bm, faces = bmesh.new(), {}
        verts = [[bm.verts.new(o + d * (dist + lift_top + (lift_hem - lift_top) * r / rows
                                        + SHOULDER * math.sin(math.radians(angles[c])) ** 2 * min(1.0, 2.0 * r / rows)))
                  for c, (o, d, dist) in enumerate(row)] for r, row in enumerate(grid)]
        for r in range(rows):
            for c in range(COLS):
                c2 = (c + 1) % COLS
                col = VIOLET_DARK if r == rows - 1 else VIOLET
                f = bm.faces.new((verts[r][c], verts[r][c2], verts[r + 1][c2], verts[r + 1][c]))
                faces[f] = col
        objs.append(plates.finish(bm, faces, lambda p: centre(p.z), THICKNESS, "cape"))

    bm, faces = bmesh.new(), {}
    for s in (1, -1):
        plates.ribbon(bm, faces, tree, [(s * x, z) for x, z in LAPEL], LAPEL_WIDTH, 0.020, VIOLET)
    objs.append(plates.finish(bm, faces, lambda c: Vector((c.x, 0.0, c.z)), 0.003, "lapels"))
    bm, faces = bmesh.new(), {}
    for s in (1, -1):
        for x, z in BUTTONS:
            stud(bm, faces, tree, (s * x, z), 0.024, 0.010, BRASS)
    objs.append(plates.finish(bm, faces, lambda c: Vector((c.x, 0.0, c.z)), 0.003, "buttons"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
