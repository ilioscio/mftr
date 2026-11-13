# SPDX-License-Identifier: AGPL-3.0-or-later
"""A topknot (BH1): the hair drawn up over the skull to a tie on the crown, the knot coiled
round above it. Places are in the source head's frame (`mftr_blender.grooming`)."""

import math

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above

TIE = Vector((0.0, 0.055, 0.138))
ROOTS = [TIE + Vector((math.cos(a) * 0.013, math.sin(a) * 0.013, 0.004)) for a in
         [2 * math.pi * k / 5 for k in range(5)]]


def coil(p):
    # Round the knot's axis, climbing a little.
    r = p - TIE
    return Vector((-r.y, r.x, 0.0)).normalized() * 1.0 + Vector((0, 0, 0.25)) - Vector((r.x, r.y, 0.0)) * 12.0


STYLE = Style(cap=0.006, band=(0.55, 0.42, 0.16), locks=[
    Locks(where=lambda p: above(p) > 0.012 and (p - TIE).length > 0.035, spacing=0.04, width=0.07, thick=0.012,
          tie=TIE, stiff=0.3, seed=31, lift=0.001, seg=0.04, hug=0.06),
    # The knot: thick locks wound round the tie.
    Locks(roots=ROOTS, length=0.10, width=0.036, thick=0.036, flow=coil, stiff=10.0, seed=32, seg=0.02,
          lift=0.02, taper=0.5, hug=0.0, profile="round", sides=5),
])
