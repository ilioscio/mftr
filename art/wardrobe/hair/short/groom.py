# SPDX-License-Identifier: AGPL-3.0-or-later
"""Short hair (BH1): a full cap and short locks combed back from the brow, lying on the skull,
the back ones down to the nape. Places are in the source head's frame (`mftr_blender.grooming`)."""

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above


def flow(p):
    # Back over the crown, then down the back of the head; the sides back, behind the ears.
    back = max(0.0, p.y - 0.01) * 18.0
    return Vector((p.x * 2.0, 1.0, -0.25 - back))


def length(p):
    # Longest on top, short at the sides over the ears and at the nape.
    return 0.085 - min(abs(p.x) * 0.6, 0.035) - max(0.0, 0.03 - p.z) * 0.6


STYLE = Style(cap=0.010, locks=[
    Locks(where=lambda p: above(p) > 0.012, spacing=0.034, length=length, width=0.08, thick=0.012, flow=flow,
          stiff=0.6, seed=3, lift=0.002, seg=0.03, hug=0.06),
])
