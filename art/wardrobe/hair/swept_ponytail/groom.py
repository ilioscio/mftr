# SPDX-License-Identifier: AGPL-3.0-or-later
"""A swept ponytail (Vesper's, 22 §7 item 5): side-swept bangs combed across the forehead to the
character's right, the rest drawn back to a tie behind the crown and the tail hanging on its
spring-bone chain. Places are in the source head's frame (`mftr_blender.grooming`)."""

import math

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above

TIE = Vector((0.0, 0.132, 0.055))
ROOTS = [TIE + Vector((math.cos(a) * 0.014, -0.004, math.sin(a) * 0.012)) for a in
         [2 * math.pi * k / 7 for k in range(7)]]


def fringe(p):
    """The front of the scalp, just behind the forehead's hairline: where the bangs grow."""
    return p.y < -0.03 and 0.004 < above(p) < 0.04


def sweep(p):
    # Across the forehead to her right (-x), forward over the brow and down to the temple.
    return Vector((-1.0, -0.35, -0.55))


STYLE = Style(cap=0.007, band=(0.30, 0.18, 0.10), locks=[
    Locks(where=fringe, spacing=0.024, length=0.11, width=0.05, thick=0.012, flow=sweep, stiff=0.6, seed=91,
          lift=0.002, seg=0.025, hug=0.04),
    Locks(where=lambda p: above(p) > 0.012 and (p - TIE).length > 0.04 and not fringe(p), spacing=0.04, width=0.07,
          thick=0.012, tie=TIE, stiff=0.3, seed=92, lift=0.001, seg=0.04, hug=0.06),
    Locks(roots=ROOTS, length=0.28, width=0.034, thick=0.034, flow=lambda p: Vector((0.0, 0.45, -1.0)), stiff=0.2,
          seed=93, seg=0.05, lift=0.01, taper=0.8, profile="round", sides=5, chain="tail"),
])
