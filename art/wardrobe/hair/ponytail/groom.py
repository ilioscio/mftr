# SPDX-License-Identifier: AGPL-3.0-or-later
"""A ponytail (BH1): the hair drawn back over the skull to a tie behind the crown, the tail
hanging from it onto the back. Places are in the source head's frame (`mftr_blender.grooming`)."""

import math

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above

TIE = Vector((0.0, 0.132, 0.055))
# The tail's roots: round the tie, on the skull.
ROOTS = [TIE + Vector((math.cos(a) * 0.014, -0.004, math.sin(a) * 0.012)) for a in
         [2 * math.pi * k / 7 for k in range(7)]]

STYLE = Style(cap=0.006, band=(0.30, 0.18, 0.10), locks=[
    # Drawn to the tie, lying on the skull.
    Locks(where=lambda p: above(p) > 0.012 and (p - TIE).length > 0.04, spacing=0.04, width=0.07, thick=0.012,
          tie=TIE, stiff=0.3, seed=21, lift=0.001, seg=0.04, hug=0.06),
    # The tail.
    Locks(roots=ROOTS, length=0.26, width=0.034, thick=0.034, flow=lambda p: Vector((0.0, 0.45, -1.0)), stiff=0.2,
          seed=22, seg=0.05, lift=0.01, taper=0.8, profile="round", sides=5, chain="tail"),
])
