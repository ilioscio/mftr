# SPDX-License-Identifier: AGPL-3.0-or-later
"""A feather capelet (Wren's): three layers of russet feathers round the neck and over the
shoulders, the middle one darker, each hem cut into feather points, the lowest falling in a point down the
back to the shoulder blades.

Plates (`mftr_blender.plates`): the astral capes' build (each layer lathed round the body, each
ray cast in from outside so it lies over the arms, standing further out over the shoulders), the
hems cut deep into points.

    blender -b --python art/wardrobe/shoulders/feather_capelet/build.py
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

# Darker than the tunic under it, so the two read apart.
RUSSET = ((0.32, 0.12, 0.05), "cloth")
RUSSET_DARK = ((0.22, 0.08, 0.035), "cloth")

TOP, DIP = 1.448, 0.045
COLS = 20
# The capes, under first: (hem in front, at the sides, at the back; rows; lift at the top and hem).
CAPES = [((1.225, 1.215, 1.060), 5, 0.018, 0.036, RUSSET, RUSSET_DARK),
         ((1.285, 1.275, 1.170), 3, 0.024, 0.044, RUSSET_DARK, RUSSET_DARK),
         ((1.345, 1.335, 1.270), 2, 0.030, 0.050, RUSSET, RUSSET_DARK)]
SCALLOP = 0.040                     # the feathers' points
POINTS = 10                         # one point per two columns (finer ones alias away)
SHOULDER = 0.010                    # further out over the shoulders (a slim feminine base's deltoids)
THICKNESS = 0.005


def top_z(a):
    return TOP - DIP * ((1 + math.cos(math.radians(a))) / 2) ** 2


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
    for (front, side, back), rows, lift_top, lift_hem, body_col, hem_col in CAPES:
        def hem_z(a, front=front, side=side, back=back):
            k = (1 - math.cos(math.radians(a))) / 2
            s = math.sin(math.radians(a)) ** 2
            z = front + (side - front) * s
            z += (back - z) * max(0.0, (k - 0.5) / 0.5) ** 1.5
            t = (a / 360.0) * POINTS
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
                col = hem_col if r == rows - 1 else body_col
                f = bm.faces.new((verts[r][c], verts[r][c2], verts[r + 1][c2], verts[r + 1][c]))
                faces[f] = col
        objs.append(plates.finish(bm, faces, lambda p: centre(p.z), THICKNESS, "cape"))

    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
