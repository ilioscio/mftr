# SPDX-License-Identifier: AGPL-3.0-or-later
"""Warden's antlers (Cairn's): great branching antlers of weathered bone rising from the brow, a
main beam sweeping out, up and back on each side with three tines reaching forward and up, a
knobbed burr at each root ringed with emerald beads.

Worn on the head bone, rigid, scaled to each base's head (`grip_scale`), as the priestess's
circlet is. The roots are fitted here to the heavy masculine base's head: a ray out from the
skull's middle finds the skin above each temple, and the beam starts just off it. Built with the
armory's bending tubes (`Kit.branch`), a little crooked so they read as grown, not turned.

Its frame is the head bone's at its joint (the neck's top): +Z up the head, +Y toward the back,
+X the character's left.

    blender -b --python art/wardrobe/head/cairn_antlers/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

from mftr_blender import armory  # noqa: E402

BONE = (0.62, 0.56, 0.44)
BONE_DARK = (0.40, 0.34, 0.25)
EMERALD = (0.30, 1.0, 0.55)
EMERALD_DEEP = (0.06, 0.62, 0.30)

BASE = os.path.join(ROOT, "art", "creator", "humanoid_heavy", "humanoid_heavy_masculine.blend")
CENTRE = Vector((0.0, 0.0, 0.12))
# Where each root is: a ray from the skull's middle (elevation, azimuth from the front toward
# +x, degrees); how far off the skin the beam starts.
ROOT_RAY = (52.0, 34.0)
LIFT = 0.004
SIDES = 6

# The left antler (+x), relative to its root: the beam's points and radii, out, up and back.
BEAM = [((0.0, 0.0, 0.0), 0.024), ((0.030, 0.005, 0.030), 0.022), ((0.075, 0.016, 0.066), 0.019),
        ((0.118, 0.036, 0.112), 0.016), ((0.148, 0.062, 0.170), 0.013), ((0.160, 0.090, 0.232), 0.011),
        ((0.156, 0.114, 0.290), 0.008), ((0.142, 0.128, 0.336), 0.005), ((0.122, 0.134, 0.372), 0.0)]
# Tines: (the beam's point they leave from, their path relative to it).
TINES = [
    # The brow tine: forward over the face, curling up.
    (2, [((0.0, 0.0, 0.0), 0.013), ((0.010, -0.040, 0.014), 0.011), ((0.016, -0.076, 0.044), 0.008),
         ((0.018, -0.094, 0.084), 0.005), ((0.014, -0.098, 0.112), 0.0)]),
    # The middle tine: up and forward.
    (4, [((0.0, 0.0, 0.0), 0.011), ((0.016, -0.034, 0.040), 0.009), ((0.026, -0.056, 0.088), 0.006),
         ((0.028, -0.062, 0.134), 0.0)]),
    # The back tine: out and up from high on the beam.
    (5, [((0.0, 0.0, 0.0), 0.009), ((0.040, -0.010, 0.030), 0.007), ((0.070, -0.012, 0.072), 0.005),
         ((0.080, -0.008, 0.106), 0.0)]),
]
BEADS = 5


def head_tree():
    bpy.ops.wm.open_mainfile(filepath=BASE)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    inv = (arm.matrix_world @ arm.data.bones["head"].matrix_local).inverted()
    to_item = lambda p: Vector((p.x, -p.z, p.y))  # noqa: E731
    body = next(o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("part."))
    pts = [to_item(inv @ (body.matrix_world @ v.co)) for v in body.data.vertices]
    tree = BVHTree.FromPolygons(pts, [tuple(p.vertices) for p in body.data.polygons])
    return tree, arm.data.bones["head"].length


def ray(e, a):
    e, a = math.radians(e), math.radians(a)
    return Vector((math.sin(a) * math.cos(e), -math.cos(a) * math.cos(e), math.sin(e)))


tree, bone_length = head_tree()
hit, normal, _i, _d = tree.ray_cast(CENTRE, ray(*ROOT_RAY))
root_l = hit + normal * LIFT

k = armory.Kit()
for s in (1, -1):
    root = Vector((root_l.x * s, root_l.y, root_l.z))
    n = Vector((normal.x * s, normal.y, normal.z))
    at = lambda p: root + Vector((p[0] * s, p[1], p[2]))  # noqa: E731
    # The beam, its first step out along the skin's normal so its base stands clear of it.
    beam = [(root, BEAM[0][1]), (root + n * 0.012, BEAM[0][1])] + [(at(p), r) for p, r in BEAM[1:]]
    k.branch(beam, BONE, "cloth", sides=SIDES)
    for i, path in TINES:
        base = at(BEAM[i][0])
        k.branch([(base + Vector((p[0] * s, p[1], p[2])), r) for p, r in path], BONE, "cloth", sides=5)
    # The burr: a knobbed ring of darker bone round the root, emerald beads set in it.
    burr = root + n * 0.010
    k.ring(burr, 0.027, 0.007, BONE_DARK, "cloth", axis=n, sides=10, tube_sides=4)
    t = n.orthogonal().normalized()
    for b in range(BEADS):
        a = 2 * math.pi * (b + 0.5) / BEADS
        d = (t * math.cos(a) + n.cross(t) * math.sin(a))
        k.gem(burr + d * 0.033 + n * 0.005, 0.009, EMERALD if b % 2 == 0 else EMERALD_DEEP)
k.save(os.path.join(HERE, "item.blend"))
print(f"head bone {bone_length:.4f} m, root {tuple(round(x, 3) for x in root_l)}")
