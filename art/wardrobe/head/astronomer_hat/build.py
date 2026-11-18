# SPDX-License-Identifier: AGPL-3.0-or-later
"""An astronomer's hat (Quill's): a wide brim and a tall crown of deep purple felt bending back to
a point, a violet band round it, brass goggles with glowing lenses pushed up on the band at the
front, and a star burning at the tip.

Worn on the head bone, rigid, scaled to each base's head (`grip_scale`), as the shadow hood is,
modelled round the slim masculine base's head with the armory's pieces (`mftr_blender.armory`):
the brim and the band lathed, the crown a bending tube (`Kit.branch`) narrowing to its point, the
goggles' rims rings, the lenses and the star stones. The crown sits on the head above the brow,
measured by rays from the skull's middle; the whole hat leans back a little.

Its frame is the head bone's at its joint (the neck's top): +Z up the head, +Y toward the back,
+X the character's left.

    blender -b --python art/wardrobe/head/astronomer_hat/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Matrix, Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

from mftr_blender import armory  # noqa: E402

FELT = (0.14, 0.07, 0.22)
FELT_DARK = (0.08, 0.04, 0.12)
VIOLET = (0.30, 0.14, 0.44)
BRASS = (0.68, 0.50, 0.22)
LENS = (0.80, 0.70, 1.0)
STAR = (0.95, 0.92, 1.0)

BASE = os.path.join(ROOT, "art", "creator", "humanoid_slim", "humanoid_slim_masculine.blend")
CENTRE = Vector((0.0, 0.012, 0.115))
# Where the crown sits: its rim this far out from the skull at the brow, at this height.
SEAT, ROOM = 0.215, 0.014
BRIM, BRIM_DROP = 0.210, 0.010      # the brim's radius and how far its edge droops
BAND = 0.040
# The crown: up from the seat, bending back to the tip.
TIP = Vector((0.0, 0.20, 0.64))
LEAN = 17.0                          # degrees the whole hat leans back
SIDES = 16


def head_tree():
    bpy.ops.wm.open_mainfile(filepath=BASE)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    inv = (arm.matrix_world @ arm.data.bones["head"].matrix_local).inverted()
    to_item = lambda p: Vector((p.x, -p.z, p.y))  # noqa: E731
    body = next(o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("part."))
    pts = [to_item(inv @ (body.matrix_world @ v.co)) for v in body.data.vertices]
    return BVHTree.FromPolygons(pts, [tuple(p.vertices) for p in body.data.polygons])


tree = head_tree()
# The skull's widest round the seat: the crown's radius there.
reach = 0.0
mid = Vector((0.0, CENTRE.y, SEAT))
for a in range(0, 360, 15):
    d = Vector((math.sin(math.radians(a)), -math.cos(math.radians(a)), 0.0))
    hit, _n, _i, _d = tree.ray_cast(mid, d)
    if hit is not None:
        reach = max(reach, (hit - mid).length)
r0 = reach + ROOM

k = armory.Kit()
seat = Vector((0.0, CENTRE.y, SEAT))
# The brim: a wide, thin disc, its edge drooping.
k.lathe([(-0.006 - BRIM_DROP, BRIM), (-0.004 - BRIM_DROP * 0.4, BRIM * 0.85), (0.004, r0 * 0.98)], FELT_DARK,
        sides=SIDES, at=tuple(seat))
# The crown: a bending tube from the seat, narrowing to the tip.
path = []
n = 12
for i in range(n + 1):
    t = i / n
    # Up first, bending back more toward the tip.
    p = seat + Vector((0.0, (TIP.y - seat.y) * t ** 2.2, (TIP.z - seat.z) * t))
    r = r0 * (1.0 - t) ** 1.1 * (1.0 + 0.08 * math.sin(math.pi * min(t * 2.5, 1.0)))
    path.append((tuple(p), r if i < n else 0.0))
k.branch(path, FELT, sides=SIDES)
# The band round the crown's foot.
k.lathe([(0.002, r0 + 0.004), (BAND, r0 * 0.93 + 0.004)], VIOLET, sides=SIDES, at=tuple(seat))
# The goggles on the band at the front: two brass rims, glowing lenses, a bridge.
for s in (1, -1):
    c = seat + Vector((s * 0.034, -(r0 + 0.010), BAND * 0.55))
    k.ring(tuple(c), 0.021, 0.006, BRASS, axis=(0.0, 1.0, 0.0), sides=10, tube_sides=4)
    k.lathe([(-0.004, 0.017), (0.002, 0.017)], LENS, slot="emissive", sides=10, at=tuple(c), axis=(0.0, -1.0, 0.0))
k.lathe([(-0.012, 0.004), (0.012, 0.004)], BRASS, slot="metal", sides=4,
        at=tuple(seat + Vector((0.0, -(r0 + 0.012), BAND * 0.55))), axis=(1.0, 0.0, 0.0))
# The star at the tip.
k.gem(tuple(TIP), 0.024, STAR, stretch=1.6)
k.gem(tuple(TIP), 0.024, STAR, stretch=1.6, turn=Matrix.Rotation(math.pi / 2, 3, "Y"))
# The brass slot names: the rims and the bridge are metal.
k.faces = [(f, c, "metal" if c == BRASS else s) for f, c, s in k.faces]
# Lean the hat back round the seat.
rot = Matrix.Rotation(math.radians(LEAN), 3, "X")
for v in k.bm.verts:
    v.co = seat + rot @ (v.co - seat)
k.save(os.path.join(HERE, "item.blend"))
print("seat radius", round(r0, 4))
