# SPDX-License-Identifier: AGPL-3.0-or-later
"""A feather crest (Wren's): a wren's cap of russet and tan feathers tipped in lime, lying swept
back from the crown over the back of the head, the longest in the middle, shorter ones beside them.
They lie back like a bird's crest rather than standing up: upright, round a brown-skinned face, a
ring of feathers read as a stereotyped headdress.

Worn on the head bone, rigid, scaled to each base's head (`grip_scale`), as the flame crest is,
over the hair. The roots are fitted to the slim masculine base's head by rays out from the skull's
middle; each feather is the armory's blade (`Kit.blade`: flat, with a ridge down its middle),
standing edge-on to the front, a lime tip laid over its last quarter.

Its frame is the head bone's at its joint (the neck's top): +Z up the head, +Y toward the back,
+X the character's left.

    blender -b --python art/wardrobe/head/feather_crest/build.py
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

RUSSET = (0.44, 0.17, 0.07)
RUSSET_DARK = (0.30, 0.11, 0.05)
TAN = (0.54, 0.40, 0.24)
LIME = (0.42, 0.62, 0.10)

BASE = os.path.join(ROOT, "art", "creator", "humanoid_slim", "humanoid_slim_masculine.blend")
CENTRE = Vector((0.0, 0.012, 0.115))
# Each feather: where it roots ((degrees over the top from the front, degrees out to the side),
# mirrored when out isn't 0), its length, its half-width, how far back it leans from the skull's
# outward ray toward straight back (0 to 1), and its color.
FEATHERS = [
    ((62, 0), 0.15, 0.022, 0.80, RUSSET),
    ((80, 0), 0.19, 0.025, 0.85, RUSSET_DARK),
    ((98, 0), 0.20, 0.025, 0.90, RUSSET),
    ((116, 0), 0.16, 0.022, 0.95, TAN),
    ((84, 14), 0.15, 0.020, 0.88, TAN),
    ((104, 16), 0.13, 0.019, 0.93, RUSSET_DARK),
]
SINK = 0.012          # how far each root sinks under the skin (into the hair)
TIP = 0.28            # the lime tip's share of the length


def head_tree():
    bpy.ops.wm.open_mainfile(filepath=BASE)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    inv = (arm.matrix_world @ arm.data.bones["head"].matrix_local).inverted()
    to_item = lambda p: Vector((p.x, -p.z, p.y))  # noqa: E731
    body = next(o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("part."))
    pts = [to_item(inv @ (body.matrix_world @ v.co)) for v in body.data.vertices]
    return BVHTree.FromPolygons(pts, [tuple(p.vertices) for p in body.data.polygons])


def ray(over, out):
    """From the skull's middle: `over` degrees over the top from the front, turned `out` to +x."""
    o, s = math.radians(over), math.radians(out)
    d = Vector((0.0, -math.cos(o), math.sin(o)))
    return (d * math.cos(s) + Vector((math.sin(s), 0.0, 0.0))).normalized()


tree = head_tree()
k = armory.Kit()
for (over, out), length, half, back, colour in FEATHERS:
    for s in ((1, -1) if out else (1,)):
        r = ray(over, out * s)
        hit, _n, _i, _d = tree.ray_cast(CENTRE, r)
        # Along the skull's ray, swept well back: lying close, like a bird's crest.
        d = (r.lerp(Vector((0.0, 1.0, 0.12)).normalized(), back)).normalized()
        d = (d + Vector((s * 0.15 * (out / 18.0), 0.0, 0.0))).normalized()
        root = hit - d * SINK
        # Edge-on to the front: the blade's width runs front to back, its flat faces sideways.
        side = Vector((1.0, 0.0, 0.0))
        across = side.cross(d).normalized()
        thick = d.cross(across).normalized()
        turn = Matrix((across, thick, d)).transposed()
        k.blade([(0.0, half * 0.4), (length * 0.3, half), (length * 0.75, half * 0.85), (length, 0.0)], colour,
                slot="cloth", ridge=0.004, at=tuple(root), turn=turn)
        a = length * (1.0 - TIP)
        k.blade([(a, half * 0.95), (length * 0.88, half * 0.62), (length * 1.01, 0.0)], LIME, slot="cloth",
                ridge=0.0055, at=tuple(root), turn=turn)
k.save(os.path.join(HERE, "item.blend"))
