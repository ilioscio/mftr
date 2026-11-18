# SPDX-License-Identifier: AGPL-3.0-or-later
"""A grave shawl (Marrow's): a ragged shawl of grave-grey wool round the neck and over the
shoulders, to the chest in front and in a long point down the back, its hem torn into points and
darker; a bone toggle closing it at the throat.

Plates (`mftr_blender.plates`), measured on the CC0 male. Lathed round the body as a collar is,
but each ray is cast in from outside rather than out from the middle, so it meets the outermost
skin: below the shoulders it lies over the arms, not between them and the chest. It falls looser
as it goes down (clear of a robe's sleeves and a corset's top at the back).

    blender -b --python art/wardrobe/shoulders/grave_shawl/build.py
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

GRAVE = ((0.24, 0.24, 0.26), "cloth")
GRAVE_DARK = ((0.14, 0.14, 0.16), "cloth")
BONE = ((0.64, 0.60, 0.50), "cloth")
BONE_DARK = ((0.42, 0.38, 0.30), "cloth")

# Its top round the neck's base (lower at the throat, under the jaw), its hem by angle: to the
# chest in front, over the arms at the sides, in a point down the back.
TOP, DIP = 1.448, 0.045
FRONT, SIDE, BACK = 1.20, 1.21, 1.00
COLS, ROWS = 28, 6
TEAR, POINTS = 0.055, 13       # the torn hem's points round it
LIFT_TOP, LIFT_HEM = 0.018, 0.040
LIFT_BACK = 0.022               # more down the back, over a corset's top
THICKNESS = 0.006


def top_z(a):
    return TOP - DIP * ((1 + math.cos(math.radians(a))) / 2) ** 2


def hem_z(a):
    """The hem at angle `a` (degrees from the front toward +x): a point down the back, torn."""
    k = (1 - math.cos(math.radians(a))) / 2          # 0 at the front, 1 at the back
    side = math.sin(math.radians(a)) ** 2
    z = FRONT + (SIDE - FRONT) * side
    z += (BACK - z) * max(0.0, (k - 0.5) / 0.5) ** 1.4
    t = (a / 360.0) * POINTS
    # Each point its own length: torn, not cut.
    tear = TEAR * (0.6 + 0.8 * ((int(t) * 7) % 5) / 4)
    z -= tear * (1 - abs((t % 1) * 2 - 1)) - TEAR * 0.5
    return z + 0.012 * math.sin(math.radians(a) * 5.0 + 0.7)


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
    grid = []
    for r in range(ROWS + 1):
        row = []
        for a in angles:
            z = top_z(a) + (hem_z(a) - top_z(a)) * (r / ROWS) ** 0.9
            d = Vector((math.sin(math.radians(a)), -math.cos(math.radians(a)), 0.0))
            o = centre(z)
            # In from outside: the outermost skin (an arm, where it hangs beside the chest).
            far = o + d * 0.8
            hit, _n, _i, _d = tree.ray_cast(far, -d)
            dist = (hit - o).dot(d) if hit is not None else 0.12
            row.append([o, d, max(dist, 0.04)])
        grid.append(row)
    plates._smooth(grid, True, 6)

    def lift(r, c):
        t = r / ROWS
        k = max(0.0, -math.cos(math.radians(angles[c])))
        return LIFT_TOP + (LIFT_HEM - LIFT_TOP) * t + LIFT_BACK * k * t

    bm, faces = bmesh.new(), {}
    verts = [[bm.verts.new(o + d * (dist + lift(r, c))) for c, (o, d, dist) in enumerate(row)]
             for r, row in enumerate(grid)]
    for r in range(ROWS):
        for c in range(COLS):
            c2 = (c + 1) % COLS
            col = GRAVE_DARK if r == ROWS - 1 else GRAVE
            f = bm.faces.new((verts[r][c], verts[r][c2], verts[r + 1][c2], verts[r + 1][c]))
            faces[f] = col
    # The toggle's place: on the shawl's front, between its first rows.
    front = Vector((0.0, min(verts[1][0].co.y, verts[0][0].co.y) - 0.010, (verts[0][0].co.z + verts[1][0].co.z) / 2))
    objs = [plates.finish(bm, faces, lambda p: centre(p.z), THICKNESS, "shawl")]

    # The toggle at the throat: a bone peg across, on a dark loop.
    bm, faces = bmesh.new(), {}
    peg = plates.Dome(front, (1.0, 0.0, 0.0), (0.0, -1.0, 0.0), (0.030, 0.008, 0.008))
    plates.dome(bm, faces, peg, lambda _ph: math.pi, 4, 6, BONE)
    loop = plates.Dome(front + Vector((0.0, 0.004, 0.0)), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (0.006, 0.016, 0.016))
    plates.dome(bm, faces, loop, lambda _ph: math.pi / 2, 2, 6, BONE_DARK)
    objs.append(plates.finish(bm, faces, None, None, "toggle"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
