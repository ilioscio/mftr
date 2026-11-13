# SPDX-License-Identifier: AGPL-3.0-or-later
"""Locs (BH1): rope-like locks, round and even, falling from the whole scalp to the shoulders.
Places are in the source head's frame (`mftr_blender.grooming`)."""

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above


def flow(p):
    # Out from the crown, then down.
    return Vector((p.x * 9.0, 0.3 + max(0.0, p.y) * 6.0, -0.8))


STYLE = Style(cap=0.008, locks=[
    Locks(where=lambda p: above(p) > 0.008, spacing=0.036, length=0.24, width=0.028, thick=0.028, flow=flow,
          stiff=0.25, seed=61, seg=0.06, lift=0.004, profile="round", sides=4, taper=0.2),
])
