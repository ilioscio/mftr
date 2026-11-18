# SPDX-License-Identifier: AGPL-3.0-or-later
"""A flame crest (Ember's): burning hair, tongues of fire rising from the crown and sweeping up
and back, white-hot at the roots, flame orange, ember red at the tips; the tallest from the
hairline, shorter ones beside and behind them.

Worn on the head bone, rigid, scaled to each base's head (`grip_scale`), as the priestess's
circlet is, over a buzz cut. The roots are fitted here to the slim masculine base's head: rays
out from the skull's middle find the skin, and each tongue starts just off it. Built with the
armory's bending tubes (`Kit.branch`), each tongue in three lengths of its colors, waving a
little so they read as fire, not spikes.

Its frame is the head bone's at its joint (the neck's top): +Z up the head, +Y toward the back,
+X the character's left.

    blender -b --python art/wardrobe/head/flame_crest/build.py
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

# Hot at the roots, flame orange, ember red at the tips (14 §4: Ember's flame).
HOT = (1.0, 0.82, 0.30)
FLAME = (1.0, 0.50, 0.10)
EMBER = (0.80, 0.16, 0.04)

BASE = os.path.join(ROOT, "art", "creator", "humanoid_slim", "humanoid_slim_masculine.blend")
CENTRE = Vector((0.0, 0.012, 0.115))
LIFT = 0.003
# The tongues: (where the root is, a ray from the skull's middle: its angle over the top from
# the front and its turn out to the side, degrees), their root's radius, how high and how far
# back their tip reaches (m), how far out to the side, and their wave.
TONGUES = [
    ((34, 0), 0.034, 0.17, 0.26, 0.0, 1.0),
    ((56, 0), 0.036, 0.19, 0.22, 0.0, -1.0),
    ((80, 0), 0.034, 0.16, 0.20, 0.0, 1.0),
    ((106, 0), 0.030, 0.11, 0.18, 0.0, -1.0),
    ((128, 0), 0.024, 0.06, 0.14, 0.0, 1.0),
    ((44, 20), 0.028, 0.13, 0.22, 0.030, -1.0),
    ((70, 24), 0.028, 0.12, 0.19, 0.040, 1.0),
    ((96, 26), 0.024, 0.08, 0.16, 0.040, -1.0),
]
STEPS = 7
SIDES = 5
WAVE = 0.012


def head_tree():
    bpy.ops.wm.open_mainfile(filepath=BASE)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    inv = (arm.matrix_world @ arm.data.bones["head"].matrix_local).inverted()
    to_item = lambda p: Vector((p.x, -p.z, p.y))  # noqa: E731
    body = next(o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("part."))
    pts = [to_item(inv @ (body.matrix_world @ v.co)) for v in body.data.vertices]
    tree = BVHTree.FromPolygons(pts, [tuple(p.vertices) for p in body.data.polygons])
    return tree, arm.data.bones["head"].length


def ray(over, out):
    """From the skull's middle: `over` degrees over the top from the front, turned `out` to +x."""
    o, s = math.radians(over), math.radians(out)
    d = Vector((0.0, -math.cos(o), math.sin(o)))
    return (d * math.cos(s) + Vector((math.sin(s), 0.0, 0.0))).normalized()


tree, bone_length = head_tree()
k = armory.Kit()
for (over, out), r0, high, back, side, wave in TONGUES:
    for s in ((1, -1) if out else (1,)):
        hit, normal, _i, _d = tree.ray_cast(CENTRE, ray(over, out * s))
        root = hit + normal * LIFT
        path = [(root - normal * 0.010, r0)]
        for i in range(1, STEPS + 1):
            t = i / STEPS
            # Up first, then sweeping back: the tip trails.
            p = root + Vector((s * side * t, back * t ** 1.4, high * math.sin(t * math.pi / 2) + 0.03 * t ** 4))
            p.x += s * wave * WAVE * math.sin(t * math.pi * 1.6)
            r = r0 * (1 - t) ** 0.8 * (1 + 0.25 * math.sin(t * math.pi))
            path.append((p, 0.0 if i == STEPS else r))
        # Three lengths: white-hot, flame, ember (each starts where the last ends).
        for color, a, b in ((HOT, 0, 3), (FLAME, 2, 5), (EMBER, 4, STEPS + 1)):
            k.branch(path[a:b], color, "emissive", sides=SIDES)
k.save(os.path.join(HERE, "item.blend"))
print(f"head bone {bone_length:.4f} m")
