# SPDX-License-Identifier: AGPL-3.0-or-later
"""An ember collar (Ember's): a tall collar of soot-black petals standing up round the back of the
neck and flaring out behind the head, lined in ember red, its top cut into flame-like points; a
band round the neck's base closing it at the throat with a gold brooch; ember-red lapels down the
chest in a V, and two rows of gold buttons between them.

Plates (`mftr_blender.plates`), measured on the CC0 male: the band is lathed round the neck; the
petals rise from the band's top ring, each column standing further out as it climbs (clear of the
head when it turns), the lining a second sheet just inside the black; the lapels are ribbons laid
on the body from the front, the brooch and buttons small domes.

    blender -b --python art/wardrobe/shoulders/ember_collar/build.py
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

SOOT = ((0.05, 0.045, 0.045), "cloth")
EMBER = ((0.55, 0.10, 0.04), "cloth")
GOLD = ((0.60, 0.40, 0.12), "metal")
FLAME = ((1.0, 0.50, 0.10), "emissive")

# The band round the neck's base: its rings (m up), all round.
BAND = [1.395, 1.425, 1.450]
BAND_LIFT = 0.016
DIP = 0.035                # how much lower it is at the throat (under the jaw of a slim base)
# The petals: round the back from this angle to its mirror (degrees from the front toward +x),
# in columns; rows up from the band's top.
SPAN = (80.0, 280.0)
COLS, ROWS = 16, 3
HIGH, LOW = 0.27, 0.13      # how high they rise at the back and at the ends of the span
OUT = 0.20                  # how far out from the band their top stands (clear of the back of a slim head)
POINTS = 8                  # petals across the span
PETAL = 0.05                # how far each petal's point rises over the notches beside it
STAND = 0.025              # how far out from the band's top the first row above it stands (clear
                           # of the back of the skull over the nape)
LINING = 0.006              # the red lining just inside the black
# The lapels, the right one (the left mirrors it): from the band at the throat down to the waist.
LAPEL = [(0.060, 1.385), (0.085, 1.30), (0.075, 1.20), (0.035, 1.075)]
LAPEL_WIDTH = 0.042
BUTTONS = [(0.040, 1.235), (0.034, 1.165), (0.028, 1.095)]


def band_z(r, a):
    """The band's ring `r` at angle `a`: lower at the throat."""
    return BAND[r] - DIP * ((1 + math.cos(math.radians(a))) / 2) ** 2


def height(a):
    """How high the petals rise at angle `a`: tallest at the back."""
    k = (1 + math.cos(math.radians(a - 180.0))) / 2
    return LOW + (HIGH - LOW) * k ** 0.7


def stud(bm, faces, tree, at, lift, size, color, setting=None, sides=6):
    """A small dome on the body's front at (x, z), `lift` off the skin (in a gold setting)."""
    hit, _n, _i, _d = tree.ray_cast(Vector((at[0], -1.0, at[1])), Vector((0, 1, 0)))
    if setting:
        d = plates.Dome(hit - Vector((0, lift - 0.004, 0)), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0),
                        (size * 0.35, size * 1.5, size * 1.5))
        plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, 6, setting)
    d = plates.Dome(hit - Vector((0, lift, 0)), (0.0, -1.0, 0.0), (0.0, 0.0, 1.0), (size * 0.6, size, size))
    plates.dome(bm, faces, d, lambda _ph: math.pi / 2, 2, sides, color)


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

    objs = []
    # The band.
    bm, faces = bmesh.new(), {}
    angles = [360.0 * c / 16 for c in range(16)]
    plates.lathe(bm, faces, tree, len(BAND) - 1, angles, band_z, centre,
                 lambda _r, _c: BAND_LIFT, lambda r, _c: SOOT if r == 0 else EMBER, smooth=4)
    objs.append(plates.finish(bm, faces, lambda c: centre(c.z), 0.004, "band"))

    # The petals: black outside, red lining inside.
    top = BAND[-1]
    o = centre(top)
    cols = [SPAN[0] + (SPAN[1] - SPAN[0]) * c / COLS for c in range(COLS + 1)]
    base = []
    for a in cols:
        d = Vector((math.sin(math.radians(a)), -math.cos(math.radians(a)), 0.0))
        hit, _n, _i, _d = tree.ray_cast(Vector((o.x, o.y, top)), d)
        base.append((hit + d * BAND_LIFT, d))

    def point(c, r, inset):
        p, d = base[c]
        t = r / ROWS
        a = cols[c]
        rise = height(a)
        if r == ROWS:
            # Flame-like points: each petal's middle rises, the notches between them dip.
            k = (c / COLS) * POINTS
            rise += PETAL * (1 - abs((k % 1) * 2 - 1)) - PETAL * 0.5
        return p + d * (STAND * min(t * ROWS, 1.0) + OUT * t ** 0.9 - inset) + Vector((0.0, 0.0, rise * t))

    for name, color, inset, thick in (("petals", SOOT, 0.0, 0.004), ("lining", EMBER, LINING, 0.002)):
        bm, faces = bmesh.new(), {}
        grid = [[bm.verts.new(point(c, r, inset)) for r in range(ROWS + 1)] for c in range(COLS + 1)]
        for c in range(COLS):
            for r in range(ROWS):
                f = bm.faces.new((grid[c][r], grid[c + 1][r], grid[c + 1][r + 1], grid[c][r + 1]))
                faces[f] = color
        axis = lambda q: Vector((o.x, o.y, q.z))  # noqa: E731
        objs.append(plates.finish(bm, faces, axis, thick, name))

    # The lapels, the brooch and the buttons.
    bm, faces = bmesh.new(), {}
    for s in (1, -1):
        plates.ribbon(bm, faces, tree, [(s * x, z) for x, z in LAPEL], LAPEL_WIDTH, 0.020, EMBER)
    objs.append(plates.finish(bm, faces, lambda c: Vector((c.x, 0.0, c.z)), 0.003, "lapels"))
    bm, faces = bmesh.new(), {}
    stud(bm, faces, tree, (0.0, 1.372), 0.024, 0.016, FLAME, setting=GOLD)
    for s in (1, -1):
        for x, z in BUTTONS:
            stud(bm, faces, tree, (s * x, z), 0.024, 0.009, GOLD, sides=4)
    objs.append(plates.finish(bm, faces, lambda c: Vector((c.x, 0.0, c.z)), 0.003, "studs"))
    bpy.data.objects.remove(body)
    plates.save(objs, os.path.join(HERE, "item.blend"))


build()
