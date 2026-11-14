# SPDX-License-Identifier: AGPL-3.0-or-later
"""Long hair (BH1): parted in the middle, swept to the sides off the face and falling behind the
ears past the shoulders onto the back. Soft locks, so they lie where the head and the shoulders
let them. Places are in the source head's frame (`mftr_blender.grooming`)."""

import math

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above


def flow(p):
    # Away from the parting and back, so the front locks frame the face rather than cover it;
    # then down.
    # The parting runs from the brow to the crown; behind it the hair falls straight.
    side = math.copysign(1.0, p.x) if abs(p.x) > 0.004 else 0.0
    side *= min(max((0.07 - p.y) / 0.05, 0.0), 1.0)
    front = max(0.0, 0.02 - p.y) * 20.0
    return Vector((side * (0.8 + front) + p.x * 4.0, 0.4 + front, -1.0))


STYLE = Style(cap=0.011, locks=[
    Locks(where=lambda p: above(p) > 0.008, spacing=0.04, length=0.30, width=0.09, thick=0.018, flow=flow,
          stiff=0.08, seed=5, seg=0.06, lift=0.002, chain="hang", split=True),
])
