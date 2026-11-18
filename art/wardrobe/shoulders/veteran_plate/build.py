# SPDX-License-Identifier: AGPL-3.0-or-later
"""A veteran's plate (22 §4; Rook's): a barrel breastplate and backplate in iron, open at the
sides, a dark iron gorget rising from them round the neck, and crimson-lacquered pauldrons domed
high over the shoulders, brass-rimmed, each over two dark iron lames down the outside of the arm.

Modelled here around the reference body (a hard object: there is nothing to simulate):
- the plates and the gorget are lathed round the torso and the neck (`mftr_blender.plates`), the
  chest a barrel;
- each pauldron is a dome tilted out over the shoulder, cut short on the neck side so it stands
  clear of the trapezius, its lames below it on the outside.

On the `shoulders` slot (fitted to the shoulder caps, not the arms below them), so it is worn
over a gambeson on the torso.

    blender -b --python art/wardrobe/shoulders/veteran_plate/build.py
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

IRON = ((0.30, 0.31, 0.35), "metal")
IRON_DARK = ((0.15, 0.16, 0.19), "metal")
BRASS = ((0.62, 0.45, 0.18), "metal")
CRIMSON = ((0.55, 0.06, 0.06), "metal")

# On the reference body (the CC0 male, `art/creator/bodies`): the torso's middle is near y = 0;
# |x| under 0.19 is the torso; the chin comes down to about 1.45 m in front of the neck.
PLATE_LOW, PLATE_TOP = 1.05, 1.39
PLATE_SPAN = 72.0      # each plate runs this far (degrees) either side of the front or the back
PLATE_ROWS, PLATE_COLS = 7, 7
LIFT = 0.024           # off the skin (a gambeson goes under it)
BARREL = 0.012         # the chest's extra fullness
GORGET_LOW = 1.355
GORGET_ROWS, GORGET_COLS = 3, 16

# The pauldrons: domes over the reference shoulder joints.
SHOULDER = Vector((0.225, 0.0, 1.28))
RADII = (0.15, 0.135, 0.145)   # along the axis, out, and front to back
TILT = 0.43                    # the axis leans out by this (x) for 0.9 up


def gorget_top(angle):
    """The gorget's top at `angle` round the neck (0 the front): under the chin in front, to the
    skull's base behind."""
    return 1.43 + 0.05 * (1 - math.cos(math.radians(angle))) / 2


def cap_end(phi):
    """How far down a pauldron runs (its angle from the pole) at `phi` round it: low on the
    outside (0), short on the neck side (pi), where the trapezius rises under it."""
    return 0.6 + 0.45 * (1 + math.cos(phi)) / 2


def torso(tree):
    bm = bmesh.new()
    faces = {}

    def z_plate(r, _a):
        return PLATE_LOW + (PLATE_TOP - PLATE_LOW) * r / PLATE_ROWS

    def lift_plate(r, _c):
        return LIFT + BARREL * max(0.0, 1.0 - abs(z_plate(r, 0) - 1.27) / 0.18)

    for mid, face in ((0.0, IRON), (180.0, IRON_DARK)):
        angles = [mid - PLATE_SPAN + 2 * PLATE_SPAN * c / PLATE_COLS for c in range(PLATE_COLS + 1)]
        plates.lathe(bm, faces, tree, PLATE_ROWS, angles, z_plate, lambda z: Vector((0.0, 0.0, z)), lift_plate,
                     lambda r, _c, face=face: BRASS if r == 0 else face, loop=False)
    # The gorget: all round the neck, standing off it and flaring toward its brass-rimmed top.
    plates.lathe(bm, faces, tree, GORGET_ROWS, [360.0 * c / GORGET_COLS for c in range(GORGET_COLS)],
                 lambda r, a: GORGET_LOW + (gorget_top(a) - GORGET_LOW) * r / GORGET_ROWS,
                 lambda z: Vector((0.0, -0.02, z)), lambda r, _c: 0.016 + 0.01 * r / GORGET_ROWS,
                 lambda r, _c: BRASS if r == GORGET_ROWS - 1 else IRON_DARK, smooth=3)
    return plates.finish(bm, faces, lambda c: Vector((0.0, -0.01, c.z)), 0.007, "torso")


def pauldrons():
    bm = bmesh.new()
    faces = {}
    for s in (1, -1):
        d = plates.Dome((s * SHOULDER.x, SHOULDER.y, SHOULDER.z), (s * TILT, 0.0, 0.9), (s * 0.9, 0.0, -TILT), RADII)
        plates.dome(bm, faces, d, cap_end, 4, 12, CRIMSON, rim=BRASS)
        # Two lames down the outside, each a little wider than the one above so they overlap.
        outer = [-1.25 + 2.5 * i / 5 for i in range(6)]
        plates.strip(bm, faces, d, outer, lambda ph: cap_end(ph) - 0.02, lambda ph: cap_end(ph) + 0.24, 2,
                     IRON_DARK, k=1.04)
        plates.strip(bm, faces, d, outer, lambda ph: cap_end(ph) + 0.20, lambda ph: cap_end(ph) + 0.46, 2,
                     IRON_DARK, k=1.07)
    return plates.finish(bm, faces, lambda c: Vector((math.copysign(SHOULDER.x, c.x), SHOULDER.y, SHOULDER.z)),
                         0.008, "pauldrons")


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    objs = [torso(tree), pauldrons()]
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
