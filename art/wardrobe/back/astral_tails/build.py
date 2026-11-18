# SPDX-License-Identifier: AGPL-3.0-or-later
"""Quill's coat tails: the long skirts of her coat, deep purple, from under the belt at the back to
the shins, widening as they fall round the backs of the legs, a vent between them, violet edges, a
brass hem and stars glowing over the cloth above it.

Plates (`mftr_blender.plates`), cast from behind as the moss cloak is (`back/moss_cloak`): each
column cast onto the reference body worn with Quill's coat (`torso/astral_coat`), stood off it,
hanging straight down from the furthest-back point above it, so the tails fall clear of the
seat. The stars are small diamonds standing just off the cloth. They swing on a spring-bone chain
each.

    blender -b --python art/wardrobe/back/astral_tails/build.py
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

PURPLE = ((0.14, 0.07, 0.22), "cloth")
VIOLET = ((0.30, 0.14, 0.44), "cloth")
BRASS = ((0.68, 0.50, 0.22), "metal")
STAR = ((0.92, 0.90, 1.0), "emissive")
UNDER = ["torso/astral_coat"]

# Rows (m up) from the top to the hem; the tails' half-width at each (they narrow as they fall).
TOP, HEM = 1.000, 0.34
ROWS = 9
# Each tail's columns across it (0 at the vent, 1 at its outside): the edges narrow, the lining.
ACROSS = [0.0, 0.08, 0.36, 0.64, 0.92, 1.0]
COLS = len(ACROSS) - 1
WIDTH = [(1.000, 0.190), (0.75, 0.235), (0.34, 0.300)]
VENT = 0.08             # the vent between them, a fraction of the half-width
LIFT = 0.022            # off what's under them
UPPER = 0.0
DRIFT = 0.04            # how much further back they hang for each metre they fall past the furthest point
SWALLOW = 0.03          # each tail's hem rises this much to its outside
HEM_BAND = 0.024        # the brass hem's height
# The stars: (column, row, where across and down the cell) over the rows above the hem.
STARS = [(1, 6, 0.3, 0.4), (2, 7, 0.6, 0.3), (3, 6, 0.7, 0.7), (2, 5, 0.2, 0.6), (3, 7, 0.2, 0.5), (1, 7, 0.8, 0.8)]
STAR_SIZE = 0.012
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
        objs += tail(bm, faces, us, back)
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
                color = BRASS
            elif c in (0, COLS - 1):
                color = VIOLET
            else:
                color = PURPLE
            f = bm.faces.new((verts[c][r], verts[c + 1][r], verts[c + 1][r + 1], verts[c][r + 1]))
            faces[f] = color
    # The stars: small four-pointed diamonds just off the cloth.
    sm, sfaces = bmesh.new(), {}
    for c, r, u, v in STARS:
        a, b = grid[c][r].lerp(grid[c + 1][r], u), grid[c][r + 1].lerp(grid[c + 1][r + 1], u)
        p = a.lerp(b, v) + Vector((0.0, 0.004, 0.0))
        pts = [p + Vector(d) * STAR_SIZE for d in ((0, 0, 1.4), (0.55, 0, 0), (0, 0, -1.4), (-0.55, 0, 0))]
        q = [sm.verts.new(x) for x in pts]
        sfaces[sm.faces.new(q)] = STAR
    stars = plates.finish(sm, sfaces, lambda p: Vector((p.x, p.y - 0.1, p.z)), 0.001, "stars")
    return [plates.finish(bm, faces, lambda p: Vector((p.x, p.y - 0.1, p.z)), THICKNESS, "tail"), stars]


build()
