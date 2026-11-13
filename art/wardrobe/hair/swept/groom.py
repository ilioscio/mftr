# SPDX-License-Identifier: AGPL-3.0-or-later
"""Swept (BH1): parted on the character's left, the top swept across to the right and back, the
sides and the back short. Places are in the source head's frame (`mftr_blender.grooming`)."""

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above

PART = 0.032  # the parting's x (the character's left is +x)


def flow(p):
    # Across the top away from the parting (most of it to the right), back over the crown and
    # down behind.
    across = -1.0 if p.x < PART else 1.0
    top = min(max((p.z - 0.07) / 0.04, 0.0), 1.0) * min(max((0.06 - p.y) / 0.06, 0.0), 1.0)
    back = max(0.0, p.y - 0.03) * 10.0
    return Vector((across * 1.4 * top + p.x * 2.0, 0.8, -0.3 - back - (1.0 - top) * 0.4))


def length(p):
    top = min(max((p.z - 0.06) / 0.05, 0.0), 1.0)
    return 0.045 + 0.085 * top


STYLE = Style(cap=0.009, locks=[
    Locks(where=lambda p: above(p) > 0.01, spacing=0.034, length=length, width=0.072, thick=0.018, flow=flow,
          stiff=0.7, seed=11, lift=0.002, seg=0.035, hug=0.04),
])
