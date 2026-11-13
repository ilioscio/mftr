# SPDX-License-Identifier: AGPL-3.0-or-later
"""Braids (BH1): the hair parted in the middle and drawn to two ties behind the ears, a braid
falling from each over the front of the shoulder. Places are in the source head's frame
(`mftr_blender.grooming`)."""

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above

TIES = [Vector((s * 0.068, 0.062, -0.005)) for s in (1, -1)]
locks = []
for tie in TIES:
    s = 1 if tie.x > 0 else -1
    locks.append(Locks(where=lambda p, s=s, tie=tie: above(p) > 0.012 and p.x * s > 0.0 and (p - tie).length > 0.035,
                       spacing=0.04, width=0.07, thick=0.012, tie=tie, stiff=0.3, seed=51, lift=0.001, seg=0.04,
                       hug=0.06))
    locks.append(Locks(roots=[tie + Vector((s * 0.008, 0.0, 0.0))], length=0.30, width=0.038, thick=0.03,
                       flow=lambda p, s=s: Vector((s * 0.15, -0.35, -1.0)), stiff=0.6, seed=52, seg=0.022,
                       lift=0.012, taper=0.45, plait=True, profile="round", sides=5, hug=0.0))

STYLE = Style(cap=0.006, band=(0.55, 0.42, 0.16), locks=locks)
