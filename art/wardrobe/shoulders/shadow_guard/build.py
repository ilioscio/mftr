# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shadow guard (Shade's): a spiked dark steel guard on the right shoulder only, a steel-rimmed
dome over one lame, three spikes rising from it up and back, held by a black leather strap across
the chest and the back to under the left arm.

Plates (`mftr_blender.plates`), as the veteran's pauldrons are: the guard a dome tilted out over
the reference body's right shoulder joint, cut short on the neck side; the strap ribbons laid on
the body seen from the front and from the back.

    blender -b --python art/wardrobe/shoulders/shadow_guard/build.py
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

from mftr_blender import cc0_body, plates  # noqa: E402

STEEL_DARK = ((0.20, 0.21, 0.25), "metal")
STEEL = ((0.42, 0.44, 0.50), "metal")
LEATHER = ((0.07, 0.06, 0.08), "cloth")

# The character's right is -x. The guard: a dome over the reference body's right shoulder joint.
SHOULDER = Vector((-0.215, 0.0, 1.285))
RADII = (0.118, 0.112, 0.122)  # along the axis, out, and front to back
TILT = 0.43                    # the axis leans out by this for 0.9 up
# The spikes: where on the dome (its angle from the pole, and round it: 0 the outside, pi/2 the
# back), how tall and how thick at the root, leaning back by this much.
SPIKES = [(0.10, 0.0, 0.085, 0.020), (0.52, 0.5, 0.065, 0.017), (0.92, 0.4, 0.045, 0.014)]
LEAN_BACK = 0.45
# The strap, seen from the front (x, z) from the guard's front edge to under the left arm, and
# from the back (x as seen from behind, the character's left on the viewer's left).
STRAP_FRONT = [(-0.15, 1.36), (-0.06, 1.28), (0.05, 1.20), (0.15, 1.13)]
STRAP_BACK = [(0.15, 1.36), (0.06, 1.28), (-0.05, 1.20), (-0.15, 1.13)]
STRAP_W, STRAP_LIFT = 0.032, 0.034


def cap_end(phi):
    """How far down the dome runs at `phi` round it: low on the outside, short on the neck side."""
    return 0.95 + 0.45 * (1 + math.cos(phi)) / 2


def guard():
    bm, faces = bmesh.new(), {}
    d = plates.Dome(SHOULDER, (-TILT, 0.0, 0.9), (-0.9, 0.0, -TILT), RADII)
    plates.dome(bm, faces, d, cap_end, 4, 12, STEEL_DARK, rim=STEEL)
    outer = [-1.2 + 2.4 * i / 5 for i in range(6)]
    plates.strip(bm, faces, d, outer, lambda ph: cap_end(ph) - 0.02, lambda ph: cap_end(ph) + 0.26, 2, STEEL_DARK,
                 k=1.05)
    shell = plates.finish(bm, faces, lambda _c: SHOULDER, 0.007, "guard")
    # The spikes: four-sided, from the dome's face up along it and leaning back.
    bm, faces = bmesh.new(), {}
    for theta, phi, h, r in SPIKES:
        base = d.at(theta, phi)
        n = (base - SHOULDER).normalized()
        up = (n + Vector((0.0, LEAN_BACK, 0.25))).normalized()
        a = up.orthogonal().normalized()
        b = up.cross(a)
        ring = [bm.verts.new(base - n * 0.01 + (a * math.cos(k * math.pi / 2) + b * math.sin(k * math.pi / 2)) * r)
                for k in range(4)]
        tip = bm.verts.new(base + up * h)
        for k in range(4):
            faces[bm.faces.new((ring[k], ring[(k + 1) % 4], tip))] = STEEL
        faces[bm.faces.new(list(reversed(ring)))] = STEEL
    spikes = plates.finish(bm, faces, lambda _c: SHOULDER, None, "spikes")
    return [shell, spikes]


def strap(tree):
    bm, faces = bmesh.new(), {}
    plates.ribbon(bm, faces, tree, STRAP_FRONT, STRAP_W, STRAP_LIFT, LEATHER)
    plates.ribbon(bm, faces, tree, STRAP_BACK, STRAP_W, STRAP_LIFT, LEATHER, toward=(0.0, -1.0, 0.0))
    return plates.finish(bm, faces, lambda c: Vector((0.0, 0.0, c.z)), 0.004, "strap")


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = guard() + [strap(tree)]
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
