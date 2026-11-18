# SPDX-License-Identifier: AGPL-3.0-or-later
"""Ember's coat tails: two long charcoal tails from under the sash at the back to the backs of the
knees, a vent between them, their edges showing the ember-red lining and a gold hem.

Plates (`mftr_blender.plates`), cast from behind as the moss cloak is (`back/moss_cloak`): each
column cast onto the reference body worn with Ember's coat (`torso/ember_coat`), stood off it,
hanging straight down from the furthest-back point above it, so the tails fall clear of the
seat. They swing on a spring-bone chain each.

    blender -b --python art/wardrobe/back/ember_tails/build.py
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

CHARCOAL = ((0.11, 0.10, 0.10), "cloth")
EMBER = ((0.55, 0.10, 0.04), "cloth")
GOLD = ((0.60, 0.40, 0.12), "metal")
UNDER = ["torso/ember_coat"]

# Rows (m up) from the top to the hem; the tails' half-width at each (they narrow as they fall).
TOP, HEM = 1.000, 0.50
ROWS = 7
# Each tail's columns across it (0 at the vent, 1 at its outside): the edges narrow, the lining.
ACROSS = [0.0, 0.12, 0.5, 0.88, 1.0]
COLS = len(ACROSS) - 1
WIDTH = [(1.000, 0.170), (0.75, 0.165), (0.50, 0.140)]
VENT = 0.16             # the vent between them, a fraction of the half-width
LIFT = 0.022            # off what's under them
UPPER = 0.0
DRIFT = 0.04            # how much further back they hang for each metre they fall past the furthest point
SWALLOW = 0.07          # each tail's hem rises this much to its outside: cut like a swallow's tail
HEM_BAND = 0.022        # the gold hem's height
THICKNESS = 0.005


def half_width(z):
    for (z0, w0), (z1, w1) in zip(WIDTH, WIDTH[1:]):
        if z >= z1:
            t = (z0 - z) / (z0 - z1)
            return w0 + (w1 - w0) * t
    return WIDTH[-1][1]


def hem(u):
    """The hem's height at `u` across (-1 to 1): lowest at the vent."""
    return HEM + SWALLOW * abs(u)


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
    objs = []
    for side in (1, -1):
        us = [side * (VENT + (1 - VENT) * t) for t in ACROSS]
        if side < 0:
            us.reverse()
        objs.append(tail(bm, faces, us, back))
        bm, faces = bmesh.new(), {}
    for o in under:
        bpy.data.objects.remove(o)
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


def tail(bm, faces, us, back):
    """One tail across `us` (a fraction of the half-width, left to right)."""
    grid = []
    for u in us:
        bottom = hem(u)
        col = []
        furthest, fz = None, TOP
        for r in range(ROWS + 1):
            # The last row a narrow gold hem.
            f = 1.0 if r == ROWS else (r / (ROWS - 1)) * (1 - HEM_BAND / (TOP - bottom))
            z = TOP + (bottom - TOP) * f
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
        for c in range(1, len(us) - 1):
            for r in range(1, ROWS):
                y = (grid[c][r].y * 2 + grid[c - 1][r].y + grid[c + 1][r].y + grid[c][r - 1].y + grid[c][r + 1].y) / 6
                new[c][r].y = max(y, grid[c][r].y)
        grid = new
    verts = [[bm.verts.new(p) for p in col] for col in grid]
    for c in range(COLS):
        for r in range(ROWS):
            if r == ROWS - 1:
                color = GOLD
            elif c in (0, COLS - 1):
                color = EMBER
            else:
                color = CHARCOAL
            f = bm.faces.new((verts[c][r], verts[c + 1][r], verts[c + 1][r + 1], verts[c][r + 1]))
            faces[f] = color
    return plates.finish(bm, faces, lambda p: Vector((p.x, p.y - 0.1, p.z)), THICKNESS, "tail")


build()
