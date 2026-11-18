# SPDX-License-Identifier: AGPL-3.0-or-later
"""A bark cuirass (Cairn's): plates of bark round the chest and the belly, ridged in strips up
and down like a tree's, open under the arms, on a leather band at the waist; an
emerald set in a granite boss at the breastbone: a front and a back.

Lathed round the torso from the waist to the top of the chest (`mftr_blender.plates`): each
column a strip of bark of its own shade, standing a little more or less proud than its
neighbours; the columns under the arms left out, top to bottom, where the arm hangs beside the
chest. Worn bare-armed, under the stone mantle.

    blender -b --python art/wardrobe/torso/bark_cuirass/build.py
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

BARK = ((0.26, 0.18, 0.12), "cloth")
BARK_LIGHT = ((0.33, 0.24, 0.16), "cloth")
BARK_DARK = ((0.15, 0.10, 0.07), "cloth")
LEATHER = ((0.30, 0.20, 0.12), "cloth")
GRANITE = ((0.30, 0.30, 0.28), "metal")
EMERALD = ((0.30, 1.0, 0.55), "emissive")

LOW, TOP, ROWS, COLS = 0.95, 1.345, 9, 20
BACK_TOP = 1.27
# The sides under the arms are open, top to bottom: within this many degrees of straight out
# (the arm tucked in at the side, as in a whirling blow, would go through them).
OPEN = 40.0
LIFT = 0.012
BARREL = 0.006
GEM_Z = 1.25


def z_at(r, a):
    # Lower at the back (BACK_TOP), under a cloak's top, which it came through on a heavy base.
    top = BACK_TOP + (TOP - BACK_TOP) * (1 + math.cos(math.radians(a))) / 2
    return LOW + (top - LOW) * r / ROWS


def angle(c):
    return 360.0 * c / COLS


def lift(r, c):
    z = z_at(r, angle(c))
    ridge = 0.004 * ((c * 7) % 3 - 1)
    # Close: the arms pass in front of and beside it (the harness's arms test); its rows are fine
    # enough to follow a bust rather than cut across it.
    return LIFT + ridge + BARREL * max(0.0, 1.0 - abs(z - 1.24) / 0.14)


def color(r, c):
    mid = angle(c + 0.5)
    side = min(abs(mid - 90.0), abs(mid - 270.0))
    if side < OPEN:
        return None
    if r == 0:
        return LEATHER
    return (BARK, BARK_LIGHT, BARK, BARK_DARK)[(c * 5) % 4]


def boss(bm, faces, at):
    """A granite diamond at the breastbone, an emerald standing out of it: closed pieces."""
    for scale, out, col in ((0.034, 0.012, GRANITE), (0.018, 0.018, EMERALD)):
        ring = [bm.verts.new(at + Vector(p) * scale) for p in ((1, 0, 0), (0, 0, 1.3), (-1, 0, 0), (0, 0, -1.3))]
        tip = bm.verts.new(at + Vector((0.0, -out, 0.0)))
        back = bm.verts.new(at + Vector((0.0, 0.004, 0.0)))
        for k in range(4):
            faces[bm.faces.new((ring[k], ring[(k + 1) % 4], tip))] = col
            faces[bm.faces.new((ring[(k + 1) % 4], ring[k], back))] = col
        at = at + Vector((0.0, -0.008, 0.0))


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    bm, faces = bmesh.new(), {}
    plates.lathe(bm, faces, tree, ROWS, [angle(c) for c in range(COLS)], z_at,
                 lambda z: plates.ray_centre(tree, z, (0.0, 0.0)), lift, color, smooth=4)
    obj = plates.finish(bm, faces, lambda c: Vector((0.0, 0.0, c.z)), 0.008, "cuirass")
    # The boss: on the front's skin at its height, out past the bark.
    o = Vector((0.0, 0.0, GEM_Z))
    hit, _n, _i, _d = tree.ray_cast(o, Vector((0.0, -1.0, 0.0)))
    bm2, faces2 = bmesh.new(), {}
    boss(bm2, faces2, hit + Vector((0.0, -(LIFT + BARREL + 0.010), 0.0)))
    gem = plates.finish(bm2, faces2, None, None, "boss")
    bpy.data.objects.remove(body)
    plates.save([obj, gem], os.path.join(HERE, "item.blend"))


build()
