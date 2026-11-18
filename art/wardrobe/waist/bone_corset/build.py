# SPDX-License-Identifier: AGPL-3.0-or-later
"""A bone corset (Marrow's): a corset of black-plum leather from the hips to under the chest, a
band of the team's color at its foot, and over its front a ribcage of bone: a breastbone down
the middle and four pairs of ribs curving out and down round the sides. A skull lantern hangs at
the left hip on an iron chain, its eyes burning in her signature color.

Plates (`mftr_blender.plates`), measured on the CC0 male: the corset lathed round the body; each
rib a narrow lathed strip standing off it, tapering toward its end; the skull a few closed domes
(cranium, jaw, eyes).

    blender -b --python art/wardrobe/waist/bone_corset/build.py
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

LEATHER = ((0.10, 0.05, 0.09), "cloth")
LEATHER_DARK = ((0.06, 0.03, 0.05), "cloth")
ACCENT = ((0.50, 0.50, 0.50), "accent")
BONE = ((0.64, 0.60, 0.50), "cloth")
BONE_DARK = ((0.42, 0.38, 0.30), "cloth")
IRON = ((0.18, 0.18, 0.20), "metal")
EYES = ((1.0, 0.24, 0.72), "emissive")

# The corset's rings (m up): dark leather, the team's band, leather up to under the chest (higher
# at the back).
RINGS = [0.930, 0.952, 0.978, 1.04, 1.10, 1.16]
TOP_BACK = 0.035
SIDES = 18
LIFT = 0.026
# The ribs: where each starts (m up) and how far round from the front (degrees: the lower ones
# further out, opening the arch under the breastbone), each curving out and DROP down to SPAN
# degrees round; a strip WIDTH tall standing RIB_LIFT off the corset.
RIBS = [(1.150, 5.0), (1.110, 5.0), (1.075, 13.0), (1.045, 22.0)]
SPAN, DROP = 80.0, 0.075
WIDTH, RIB_LIFT = 0.017, 0.010
STERNUM = (1.165, 1.070, 5.0)    # top, bottom, half-width (degrees)
# The skull lantern: hung at the left hip (+x) on a chain this long.
HANG_AT, CHAIN = 75.0, 0.085
SKULL = 0.040


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    centres = {}

    def centre(z):
        k = round(z, 3)
        if k not in centres:
            centres[k] = plates.ray_centre(tree, z, (0.0, 0.0))
        c = centres[k]
        return Vector((c.x, c.y, z))

    def ring_z(r, a):
        z = RINGS[r]
        if r == len(RINGS) - 1:
            z += TOP_BACK * ((1 - math.cos(math.radians(a))) / 2)
        return z

    objs = []
    bm, faces = bmesh.new(), {}
    plates.lathe(bm, faces, tree, len(RINGS) - 1, [360.0 * c / SIDES for c in range(SIDES)], ring_z, centre,
                 lambda _r, _c: LIFT,
                 lambda r, _c: ACCENT if r == 1 else (LEATHER_DARK if r == 0 else LEATHER), smooth=4)
    objs.append(plates.finish(bm, faces, lambda c: centre(c.z), 0.006, "corset"))

    # The ribs and the breastbone.
    bm, faces = bmesh.new(), {}
    for z0, a0 in RIBS:
        for s in (1, -1):
            n = 8
            angles = [s * (a0 + (SPAN - a0) * k / n) for k in range(n + 1)]

            def rib_z(r, a, z0=z0, a0=a0):
                t = (abs(a) - a0) / (SPAN - a0)
                w = WIDTH * (1.0 - 0.55 * t)
                return z0 - DROP * t ** 0.9 + (w / 2 if r == 0 else -w / 2)

            plates.lathe(bm, faces, tree, 1, angles, rib_z, centre, lambda _r, _c: LIFT + RIB_LIFT,
                         lambda _r, c: BONE, loop=False, smooth=2)
    top, bottom, half = STERNUM
    plates.lathe(bm, faces, tree, 4, [-half, 0.0, half], lambda r, _a: top + (bottom - top) * r / 4, centre,
                 lambda _r, _c: LIFT + RIB_LIFT + 0.002, lambda _r, _c: BONE_DARK, loop=False, smooth=2)
    objs.append(plates.finish(bm, faces, lambda c: centre(c.z), 0.007, "ribs"))

    # The skull lantern: an iron chain from the band, the skull under it facing forward and out.
    bm, faces = bmesh.new(), {}
    a = math.radians(HANG_AT)
    d = Vector((math.sin(a), -math.cos(a), 0.0))
    z = RINGS[1]
    o = centre(z)
    hit, _n, _i, _d = tree.ray_cast(o, d)
    hook = hit + d * (LIFT + 0.006)
    low = hook + Vector((0.0, 0.0, -CHAIN)) + d * 0.030
    chain = plates.Dome((hook + low) / 2, (low - hook).normalized(), (1.0, 0.0, 0.0), ((low - hook).length / 2, 0.005, 0.005))
    plates.dome(bm, faces, chain, lambda _ph: math.pi, 4, 4, IRON)
    head = low + Vector((0.0, 0.0, -SKULL * 0.9))
    face = Vector((d.x * 0.5, -0.85, 0.0)).normalized()
    cran = plates.Dome(head, (0.0, 0.0, 1.0), face, (SKULL, SKULL * 1.05, SKULL * 0.9))
    plates.dome(bm, faces, cran, lambda _ph: math.pi * 0.8, 5, 8, BONE)
    jaw = plates.Dome(head + face * SKULL * 0.45 + Vector((0.0, 0.0, -SKULL * 0.55)), (0.0, 0.0, -1.0), face,
                      (SKULL * 0.45, SKULL * 0.55, SKULL * 0.6))
    plates.dome(bm, faces, jaw, lambda _ph: math.pi * 0.6, 3, 6, BONE_DARK)
    side = Vector((0.0, 0.0, 1.0)).cross(face).normalized()
    for k in (1, -1):
        eye = head + face * SKULL * 0.88 + side * k * SKULL * 0.38 + Vector((0.0, 0.0, -SKULL * 0.05))
        e = plates.Dome(eye, face, Vector((0.0, 0.0, 1.0)), (SKULL * 0.22, SKULL * 0.32, SKULL * 0.30))
        plates.dome(bm, faces, e, lambda _ph: math.pi * 0.7, 2, 6, EYES)
    objs.append(plates.finish(bm, faces, None, None, "lantern"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
