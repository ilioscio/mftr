# SPDX-License-Identifier: AGPL-3.0-or-later
"""A moss cloak (Cairn's): a heavy cloak of moss-green wool from under the stone mantle at the
shoulders to the calves, widening as it falls, its hem torn into points and its edges and hem in
darker moss.

Plates (`mftr_blender.plates`), not sewn: sewn, a cape held only at the neck bunched up where the
sim left it. Each column of the cloak is cast from behind onto the reference body worn with
Cairn's cuirass and kilt (`torso/bark_cuirass`, `waist/bark_kilt`), stood off it, and hangs
straight down from the furthest-back point above it (the shoulder blades, the kilt), so it falls
clear over them. Its top is under the mantle's back slab. It swings on spring-bone chains.

    blender -b --python art/wardrobe/back/moss_cloak/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
WARDROBE = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

from mftr_blender import cc0_body, plates  # noqa: E402

MOSS = ((0.16, 0.30, 0.10), "cloth")
MOSS_DARK = ((0.09, 0.18, 0.06), "cloth")
UNDER = ["torso/bark_cuirass", "waist/bark_kilt"]

# Rows (m up) from the top to the hem; its half-width at each (it widens as it falls).
TOP, HEM = 1.395, 0.45
ROWS, COLS = 10, 10
WIDTH = [(1.395, 0.165), (1.20, 0.195), (1.00, 0.215), (0.80, 0.27), (0.45, 0.36)]
LIFT = 0.030            # off what's under it (on a heavy base the cuirass stands further out)
UPPER = 0.045           # more over the shoulder blades, where a heavy base's back is fullest
DRIFT = 0.025           # how much further back it hangs for each metre it falls past the furthest point
TEAR = 0.055            # the torn hem's points
THICKNESS = 0.006


def half_width(z):
    for (z0, w0), (z1, w1) in zip(WIDTH, WIDTH[1:]):
        if z >= z1:
            t = (z0 - z) / (z0 - z1)
            return w0 + (w1 - w0) * t
    return WIDTH[-1][1]


def hem(u):
    """The hem's height at `u` across (-1 to 1): torn into points."""
    k = (u + 1) / 2 * 6
    return HEM + TEAR * abs((k % 2) - 1) + 0.012 * math.sin(7.3 * u)


def tree_of(objs):
    pts, polys = [], []
    for o in objs:
        me = o.data
        m = o.matrix_world
        base = len(pts)
        pts += [m @ v.co for v in me.vertices]
        polys += [tuple(base + i for i in p.vertices) for p in me.polygons]
    return BVHTree.FromPolygons(pts, polys)


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    under = []
    for rel in UNDER:
        with bpy.data.libraries.load(os.path.join(WARDROBE, rel, "item.blend"), link=False) as (_src, dst):
            dst.objects = ["item"]
        under.append(dst.objects[0])
    # The body without its arms (they hang beside the cloak, not under it).
    me = body.data
    keep = [p for p in me.polygons
            if not any(abs(me.vertices[i].co.x) > 0.20 and me.vertices[i].co.z < 1.36 and me.vertices[i].co.z > 0.75
                       for i in p.vertices)]
    pts = [v.co.copy() for v in me.vertices]
    trees = [BVHTree.FromPolygons(pts, [tuple(p.vertices) for p in keep]), tree_of(under)]

    def back(x, z):
        """The furthest back of what's under the cloak at (x, z), or None."""
        best = None
        for t in trees:
            hit, _n, _i, _d = t.ray_cast(Vector((x, 1.0, z)), Vector((0.0, -1.0, 0.0)))
            if hit is not None and (best is None or hit.y > best):
                best = hit.y
        return best

    bm, faces = bmesh.new(), {}
    us = [-1 + 2 * c / COLS for c in range(COLS + 1)]
    grid = []
    for c, u in enumerate(us):
        bottom = hem(u)
        col = []
        furthest, fz = None, TOP
        for r in range(ROWS + 1):
            z = TOP + (bottom - TOP) * r / ROWS
            x = u * half_width(z)
            y = back(x, z)
            if y is not None:
                y += LIFT + UPPER * min(max((z - 1.05) / 0.25, 0.0), 1.0)
            if furthest is None or (y is not None and y > furthest - DRIFT * (fz - z)):
                if y is not None:
                    furthest, fz = y, z
            hang = furthest + DRIFT * (fz - z) if furthest is not None else 0.12
            col.append(Vector((x, max(hang, y) if y is not None else hang, z)))
        grid.append(col)
    # Smooth the hang a little down and across (no kinks where it leaves the shoulder blades).
    for _ in range(2):
        new = [[p.copy() for p in col] for col in grid]
        for c in range(1, COLS):
            for r in range(1, ROWS):
                y = (grid[c][r].y * 2 + grid[c - 1][r].y + grid[c + 1][r].y + grid[c][r - 1].y + grid[c][r + 1].y) / 6
                new[c][r].y = max(y, grid[c][r].y)
        grid = new
    verts = [[bm.verts.new(p) for p in col] for col in grid]
    for c in range(COLS):
        for r in range(ROWS):
            edge = c in (0, COLS - 1) or r == ROWS - 1
            f = bm.faces.new((verts[c][r], verts[c + 1][r], verts[c + 1][r + 1], verts[c][r + 1]))
            faces[f] = MOSS_DARK if edge else MOSS
    obj = plates.finish(bm, faces, lambda p: Vector((0.0, p.y - 0.1, p.z)), THICKNESS, "cloak")
    for o in under:
        bpy.data.objects.remove(o)
    bpy.data.objects.remove(body)
    plates.save([obj], os.path.join(HERE, "item.blend"))


build()
