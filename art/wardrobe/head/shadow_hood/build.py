# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shadow hood (Shade's): a tall hood of indigo cloth, close round the head and the neck, closed
under the chin, open at the face with a violet rim, sweeping up and back into a point.

Worn on the head bone, rigid, scaled to each base's head (`grip_scale`), as the priestess's
circlet is: the reference body has no head of its own to fit it to. It is fitted here to the slim
masculine base's head: rays out from the skull's middle find the skin, the hood stood off it far
enough to clear the hair (which hides under it, 22 §7 item 5); then rings narrowing to the point
above and behind the crown. Given thickness by `mftr_blender.plates`.

Its frame is the head bone's at its joint (the neck's top): +Z up the head, +Y toward the back,
+X the character's left.

    blender -b --python art/wardrobe/head/shadow_hood/build.py
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

INDIGO = ((0.12, 0.09, 0.20), "cloth")
VIOLET = ((0.26, 0.10, 0.38), "cloth")

BASE = os.path.join(ROOT, "art", "creator", "humanoid_slim", "humanoid_slim_masculine.blend")
# The skull's middle, where the rays start (the head bone's frame).
CENTRE = Vector((0.0, 0.012, 0.115))
# Rows of rays by elevation (degrees, from under the jaw to over the crown), columns round from
# the front (-y) toward +x.
ELEVATIONS = [-76, -52, -28, -4, 20, 42, 62]
SIDES = 16
# The face's opening: between these rows, the columns whose middle is this near the front.
OPEN_ROWS, OPEN_HALF = (1, 4), 50.0
# The point: above and behind the crown, reached by rings narrowing along a curve.
TIP = Vector((0.0, 0.27, 0.34))
PEAK = [0.35, 0.70]
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
    """How far off the skin: roomy, more so over the crown; the rim over the brow juts forward."""
    a = abs(((azimuth(c) + 180.0) % 360.0) - 180.0)
    e = ELEVATIONS[r]
    out = 0.026 + 0.012 * max(0.0, e) / 60.0
    if e < 0 and a < 90:
        # Loose under the chin, falling to the neck.
        out += 0.012 * (1.0 - a / 90.0) * min(1.0, -e / 50.0)
    if a < OPEN_HALF + 25 and e >= 20:
        out += 0.016 * (1.0 - a / (OPEN_HALF + 25))
    return out


def colour(r, c):
    """The quad's from row r to r + 1, column c to c + 1: None in the face's opening, violet at
    its rim."""
    mid = abs(((azimuth(c) + 180.0 / SIDES + 180.0) % 360.0) - 180.0)
    lo, hi = OPEN_ROWS
    if lo <= r < hi and mid < OPEN_HALF:
        return None
    if (lo <= r < hi and mid < OPEN_HALF + 20) or (r == hi and mid < OPEN_HALF):
        return VIOLET
    return INDIGO


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
    # The point: the top ring narrowing along a curve up and then back to the tip.
    top = rings[-1]
    base = sum(top, Vector()) / len(top)
    bend = Vector((0.0, base.y + 0.07, base.z + 0.09))
    axis = [CENTRE, base]
    for t in PEAK:
        mid = base * (1 - t) ** 2 + bend * 2 * t * (1 - t) + TIP * t * t
        axis.append(mid)
        rings.append([mid + (p - base) * (1 - t) ** 0.9 for p in top])
    axis.append(TIP)

    bm, faces = bmesh.new(), {}
    verts = [[bm.verts.new(p) for p in ring] for ring in rings]
    tip = bm.verts.new(TIP)
    for r in range(len(verts) - 1):
        for c in range(SIDES):
            c2 = (c + 1) % SIDES
            col = colour(r, c) if r < len(ELEVATIONS) - 1 else INDIGO
            if col is None:
                continue
            faces[bm.faces.new((verts[r][c], verts[r][c2], verts[r + 1][c2], verts[r + 1][c]))] = col
    for c in range(SIDES):
        faces[bm.faces.new((verts[-1][c], verts[-1][(c + 1) % SIDES], tip))] = INDIGO

    def inside(p):
        """The hood's middle at p's height: the skull's, then the point's curve."""
        if p.z <= base.z:
            return Vector((0.0, CENTRE.y, p.z))
        for a, b in zip(axis[1:], axis[2:]):
            if p.z <= b.z or b is axis[-1]:
                t = min(max((p.z - a.z) / max(b.z - a.z, 1e-6), 0.0), 1.0)
                return a.lerp(b, t)
        return TIP

    obj = plates.finish(bm, faces, inside, THICKNESS, "hood")
    plates.save([obj], os.path.join(HERE, "item.blend"))


build()
