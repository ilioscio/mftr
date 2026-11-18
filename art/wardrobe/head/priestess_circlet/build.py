# SPDX-License-Identifier: AGPL-3.0-or-later
"""A priestess's circlet and halo (Lumen's): a gold band round the brow, a little higher at the
back, with a sun stone burning at the forehead in a gold setting; and behind the head, floating, a
double halo of sun gold.

Worn on the head bone, rigid, as a held item is placed on its hand (`wardrobe._place_grip`), and
scaled to each base's head by the bone's length (`grip_scale`): the reference body has no head of
its own to fit it to (each base's head is authored). The band is fitted here to the slim feminine
base's head: rings of rays out from its middle find the skin; the band stands off it far enough
to clear hair drawn back over the skull.

Its frame is the head bone's at its joint (the neck's top): +Z up the head, +Y toward the back,
+X the character's left.

    blender -b --python art/wardrobe/head/priestess_circlet/build.py
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

GOLD = (0.78, 0.60, 0.22)
GOLD_DARK = (0.52, 0.38, 0.14)
SUN = (1.0, 0.86, 0.42)
SUN_WHITE = (1.0, 0.96, 0.78)

BASE = os.path.join(ROOT, "art", "creator", "humanoid_slim", "humanoid_slim_feminine.blend")
# Where the band runs (m up the head from its joint): over the brow at the front, higher at the
# back; how far it stands off the skin, and how tall it is.
FRONT_Z, BACK_Z = 0.150, 0.172
LIFT, HEIGHT, THICK = 0.016, 0.014, 0.004
SIDES = 20


def head_rays():
    """The slim feminine base's head in the head bone's frame (as `item`'s): for each of the
    band's angles, the skin's distance from the head's middle at the band's height."""
    bpy.ops.wm.open_mainfile(filepath=BASE)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    m = arm.matrix_world @ arm.data.bones["head"].matrix_local
    # The bone's +Y is up the head, its +Z the face's front: the item's frame turns them.
    to_item = lambda p: Vector((p.x, -p.z, p.y))  # noqa: E731
    inv = m.inverted()
    body = next(o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("part."))
    pts = [to_item(inv @ (body.matrix_world @ v.co)) for v in body.data.vertices]
    tree = BVHTree.FromPolygons(pts, [tuple(p.vertices) for p in body.data.polygons])
    out = []
    for i in range(SIDES):
        a = 2 * math.pi * i / SIDES
        z = FRONT_Z + (BACK_Z - FRONT_Z) * (1 - math.cos(a)) / 2
        # The middle of the skull at this height, found from its front and back.
        f, _n, _i, _d = tree.ray_cast(Vector((0, 0.0, z)), Vector((0, -1, 0)))
        b, _n, _i, _d = tree.ray_cast(Vector((0, 0.0, z)), Vector((0, 1, 0)))
        cy = (f.y + b.y) / 2 if f and b else 0.0
        d = Vector((math.sin(a), -math.cos(a), 0.0))
        o = Vector((0.0, cy, z))
        hit, _n, _i, _d = tree.ray_cast(o, d)
        out.append((o, d, (hit - o).length if hit else 0.09))
    bone = arm.data.bones["head"].length
    return out, bone


rays, bone_length = head_rays()
k = armory.Kit()
# The band: a flat ring of gold, its section a thin rectangle stood off the skin.
ring = []
for o, d, dist in rays:
    r0, r1 = dist + LIFT, dist + LIFT + THICK
    lo, hi = Vector((0, 0, -HEIGHT / 2)), Vector((0, 0, HEIGHT / 2))
    ring.append([k.bm.verts.new(o + d * r1 + lo), k.bm.verts.new(o + d * r1 + hi),
                 k.bm.verts.new(o + d * r0 + hi), k.bm.verts.new(o + d * r0 + lo)])
for i in range(SIDES):
    a, b = ring[i], ring[(i + 1) % SIDES]
    for j in range(4):
        j2 = (j + 1) % 4
        k._face((a[j], b[j], b[j2], a[j2]), GOLD, "metal")
# The setting at the forehead: a gold diamond on the band, the sun stone in it, gold rays.
o, d, dist = rays[0]
front = o + d * (dist + LIFT + THICK)
k.gem(front + Vector((0, -0.004, 0.006)), 0.024, GOLD_DARK, "metal", stretch=0.35,
      turn=Matrix.Rotation(math.radians(90), 3, "X"))
k.gem(front + Vector((0, -0.007, 0.006)), 0.013, SUN_WHITE, stretch=0.6,
      turn=Matrix.Rotation(math.radians(90), 3, "X"))
for s in (1, -1):
    k.lathe([(0.0, 0.004), (0.022, 0.0)], GOLD, "metal", sides=3, at=front + Vector((s * 0.018, -0.002, 0.012)),
            axis=(s * 0.7, 0.0, 0.7))
# The halo: two rings of sun gold, upright behind the head, floating.
HALO = Vector((0.0, 0.17, 0.16))
k.ring(HALO, 0.155, 0.007, SUN, "emissive", axis=(0, 1, 0), sides=20, tube_sides=4, squash=0.6)
k.ring(HALO, 0.128, 0.004, SUN_WHITE, "emissive", axis=(0, 1, 0), sides=16, tube_sides=3)
k.save(os.path.join(HERE, "item.blend"))
print(f"head bone {bone_length:.4f} m")
