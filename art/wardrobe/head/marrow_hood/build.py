# SPDX-License-Identifier: AGPL-3.0-or-later
"""A grave hood (Marrow's): a deep hood of plum wool, roomy over the crown, set back so the face
shows in its wide opening, a heavy rolled rim of lighter plum framing it, and its peak drooping
over the back of the head and down behind the nape.

The shadow hood's build (`head/shadow_hood/build.py`): worn on the head bone, rigid, scaled to
each base's head (`grip_scale`), fitted to the slim masculine base's head by rays out from the
skull's middle; the rings past the crown narrow along a curve up, back and down to the peak's
tip. The rim is a closed tube laid round the opening's edge. Given thickness by
`mftr_blender.plates`.

Its frame is the head bone's at its joint (the neck's top): +Z up the head, +Y toward the back,
+X the character's left.

    blender -b --python art/wardrobe/head/marrow_hood/build.py
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

PLUM_DARK = ((0.12, 0.04, 0.11), "cloth")
PLUM = ((0.24, 0.08, 0.20), "cloth")

BASE = os.path.join(ROOT, "art", "creator", "humanoid_slim", "humanoid_slim_masculine.blend")
# The skull's middle, where the rays start (the head bone's frame).
CENTRE = Vector((0.0, 0.012, 0.115))
# Rows of rays by elevation (degrees, from under the jaw to over the crown), columns round from
# the front (-y) toward +x.
ELEVATIONS = [-76, -54, -32, -10, 12, 34, 54, 70]
SIDES = 18
# The face's opening: between these rows, the columns whose middle is this near the front. Wider
# and higher than the shadow hood's: set back, the brow and the cheeks show.
OPEN_ROWS, OPEN_HALF = (1, 5), 62.0
# The peak: over the crown, then back and down behind the nape, reached by rings narrowing along a
# curve.
BEND = Vector((0.0, 0.17, 0.10))
TIP = Vector((0.0, 0.25, -0.02))
PEAK = [0.2, 0.4, 0.6, 0.8]
# The rolled rim round the opening: a tube this thick, its middle this far out from the hood.
RIM, RIM_OUT = 0.016, 0.008
THICKNESS = 0.007


def head_tree():
    """The slim masculine base's head and neck in the head bone's frame."""
    bpy.ops.wm.open_mainfile(filepath=BASE)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    inv = (arm.matrix_world @ arm.data.bones["head"].matrix_local).inverted()
    # The bone's +Y is up the head, its +Z the face's front: the item's frame turns them.
    to_item = lambda p: Vector((p.x, -p.z, p.y))  # noqa: E731
    body = next(o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("part."))
    pts = [to_item(inv @ (body.matrix_world @ v.co)) for v in body.data.vertices]
    return BVHTree.FromPolygons(pts, [tuple(p.vertices) for p in body.data.polygons])


def azimuth(c):
    return 360.0 * c / SIDES


def ray(e, a):
    e, a = math.radians(e), math.radians(a)
    return Vector((math.sin(a) * math.cos(e), -math.cos(a) * math.cos(e), math.sin(e)))


def lift(r, c):
    """How far off the skin: roomy, more so over the crown and round the back (a deep hood)."""
    a = abs(((azimuth(c) + 180.0) % 360.0) - 180.0)
    e = ELEVATIONS[r]
    out = 0.030 + 0.020 * max(0.0, e) / 70.0 + 0.012 * max(0.0, a - 90.0) / 90.0
    if e < 0 and a < 90:
        # Loose under the chin, falling to the neck.
        out += 0.014 * (1.0 - a / 90.0) * min(1.0, -e / 50.0)
    if a < OPEN_HALF + 30 and e >= 12:
        # The brow of the hood juts forward over the face.
        out += 0.014 * (1.0 - a / (OPEN_HALF + 30))
    return out


def opening(r, c):
    """Whether the quad from row r to r + 1, column c to c + 1 is in the face's opening."""
    mid = abs(((azimuth(c) + 180.0 / SIDES + 180.0) % 360.0) - 180.0)
    lo, hi = OPEN_ROWS
    return lo <= r < hi and mid < OPEN_HALF


def colour(r, c):
    if opening(r, c):
        return None
    return PLUM_DARK


def loop_tube(bm, faces, path, radius, color, sides=6):
    """A closed tube round the closed loop `path` (the rolled rim)."""
    n = len(path)
    rings = []
    for i, p in enumerate(path):
        t = (path[(i + 1) % n] - path[i - 1]).normalized()
        # Across: away from the loop's middle, then round the tube.
        mid = sum(path, Vector()) / n
        u = (p - mid)
        u = (u - t * t.dot(u)).normalized()
        w = t.cross(u)
        rings.append([bm.verts.new(p + (u * math.cos(2 * math.pi * k / sides)
                                        + w * math.sin(2 * math.pi * k / sides)) * radius) for k in range(sides)])
    for i in range(n):
        lo, hi = rings[i], rings[(i + 1) % n]
        for k in range(sides):
            f = bm.faces.new((lo[k], lo[(k + 1) % sides], hi[(k + 1) % sides], hi[k]))
            faces[f] = color


def build():
    tree = head_tree()
    grid = []
    for e in ELEVATIONS:
        row = []
        for c in range(SIDES):
            d = ray(e, azimuth(c))
            hit, _n, _i, _d = tree.ray_cast(CENTRE, d)
            row.append([CENTRE, d, (hit - CENTRE).length if hit is not None else 0.11])
        grid.append(row)
    plates._smooth(grid, True, 4)
    rings = [[o + d * (dist + lift(r, c)) for c, (o, d, dist) in enumerate(row)] for r, row in enumerate(grid)]
    # The peak: the top ring narrowing along a curve up, back and down to the tip.
    top = rings[-1]
    base = sum(top, Vector()) / len(top)
    bend = base + BEND
    axis = [base]
    for t in PEAK:
        mid = base * (1 - t) ** 2 + bend * 2 * t * (1 - t) + TIP * t * t
        axis.append(mid)
        rings.append([mid + (p - base) * (1 - t) ** 0.8 for p in top])
    axis.append(TIP)

    bm, faces = bmesh.new(), {}
    verts = [[bm.verts.new(p) for p in ring] for ring in rings]
    tip = bm.verts.new(TIP)
    for r in range(len(verts) - 1):
        for c in range(SIDES):
            c2 = (c + 1) % SIDES
            col = colour(r, c) if r < len(ELEVATIONS) - 1 else PLUM_DARK
            if col is None:
                continue
            faces[bm.faces.new((verts[r][c], verts[r][c2], verts[r + 1][c2], verts[r + 1][c]))] = col
    for c in range(SIDES):
        faces[bm.faces.new((verts[-1][c], verts[-1][(c + 1) % SIDES], tip))] = PLUM_DARK

    def inside(p):
        """The hood's middle near p: the skull's, then the peak's curve."""
        if p.z <= base.z and p.y < base.y + 0.05:
            return Vector((0.0, CENTRE.y, p.z))
        best = min(axis, key=lambda q: (q - p).length)
        return best

    objs = [plates.finish(bm, faces, inside, THICKNESS, "hood")]

    # The rolled rim: round the opening's edge, a little out from the hood.
    lo, hi = OPEN_ROWS
    cols = [c for c in range(SIDES) if opening(lo, c)]
    # The opening's columns run across the front, wrapping past 0: in order from its -x edge.
    start = next(c for c in cols if not opening(lo, (c - 1) % SIDES))
    cols = [(start + k) % SIDES for k in range(len(cols))]
    first, last = cols[0], (cols[-1] + 1) % SIDES
    edge = [rings[lo][c] for c in cols] + [rings[lo][last]]
    edge += [rings[r][last] for r in range(lo + 1, hi + 1)]
    edge += [rings[hi][c] for c in reversed(cols)]
    edge += [rings[r][first] for r in range(hi - 1, lo, -1)]
    mid = sum(edge, Vector()) / len(edge)

    def out(p):
        return p + (p - CENTRE).normalized() * RIM_OUT

    path = [out(p) for p in edge]
    # Smoothed round the loop: a rolled edge, not the grid's corners.
    for _ in range(3):
        path = [(path[i - 1] + 2 * path[i] + path[(i + 1) % len(path)]) / 4 for i in range(len(path))]
    del mid
    bm, faces = bmesh.new(), {}
    loop_tube(bm, faces, path, RIM, PLUM, sides=6)
    objs.append(plates.finish(bm, faces, None, None, "rim"))
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
