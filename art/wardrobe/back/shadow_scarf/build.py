# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shadow scarf (Shade's): violet cloth wound thick round the base of the neck, and two long tails
from it down the back that stream behind him as he moves.

Plates (`mftr_blender.plates`), not sewn: the loop is lathed round the neck from rays to the skin,
each ring standing further off than the one below and the top one drawn in, so it rolls; the tails
are ribbons laid down the back, seen from behind, far enough off to lie over a jerkin. (Sewn, the
loop settled on the neck by the sim alone, and on the slim bases the neck came through it,
22 §4.8.) The tails hang on spring-bone chains from the chest (`item.ron`).

    blender -b --python art/wardrobe/back/shadow_scarf/build.py
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

VIOLET = ((0.26, 0.10, 0.38), "cloth")
FOLD = ((0.19, 0.07, 0.28), "cloth")

# The loop's rings (m up at the back; the front `DIP` lower), and how far each stands off the
# skin: out and back in, a roll of cloth.
RINGS = [1.42, 1.435, 1.45, 1.46]
LIFTS = [0.020, 0.034, 0.038, 0.030]
DIP = 0.045
SIDES = 16
# The tails, seen from behind (x as seen, the character's left on the viewer's left), from under
# the loop down the back; their width and how far off the skin.
TAILS = [[(0.035, 1.40), (0.042, 1.22), (0.050, 1.04), (0.056, 0.90)],
         [(-0.035, 1.40), (-0.040, 1.22), (-0.046, 1.06), (-0.050, 0.96)]]
TAIL_W, TAIL_LIFT = 0.058, 0.034


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    centres = {}

    def centre(z):
        k = round(z, 4)
        if k not in centres:
            centres[k] = plates.ray_centre(tree, z, (0.0, 0.0))
        return centres[k]

    def ring_z(r, a):
        # Lower at the front (a = 0), as a scarf falls over the collarbones.
        return RINGS[r] - DIP * (1 + math.cos(math.radians(a))) / 2

    bm, faces = bmesh.new(), {}
    plates.lathe(bm, faces, tree, len(RINGS) - 1, [360.0 * c / SIDES for c in range(SIDES)], ring_z, centre,
                 lambda r, _c: LIFTS[r], lambda r, _c: FOLD if r == 1 else VIOLET, smooth=2)
    loop = plates.finish(bm, faces, lambda c: centre(c.z), 0.005, "loop")
    bm, faces = bmesh.new(), {}
    for path in TAILS:
        plates.ribbon(bm, faces, tree, path, TAIL_W, TAIL_LIFT, VIOLET, toward=(0.0, -1.0, 0.0))
    tails = plates.finish(bm, faces, lambda c: Vector((0.0, 0.0, c.z)), 0.005, "tails")
    bpy.data.objects.remove(body)
    plates.save([loop, tails], os.path.join(HERE, "item.blend"))


build()
