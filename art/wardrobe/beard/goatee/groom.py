# SPDX-License-Identifier: AGPL-3.0-or-later
"""A goatee (BH2): the chin and the lip above the mouth, the cheeks bare, a short tuft falling
from the chin. Places are in the source head's frame (`mftr_blender.grooming`)."""

import math

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, beard


def area(p):
    return min(beard(p), 0.031 - abs(p.x))


def lip(p):
    return Vector((math.copysign(1.0, p.x) * 0.9, -0.25, -0.8))


STYLE = Style(cap=0.006, ramp=0.008, area=area, blend=0.015, drape=False, locks=[
    Locks(where=lambda p: p.z < -0.066, spacing=0.016, length=0.05, width=0.04, thick=0.016,
          flow=lambda p: Vector((p.x * 2.0, -0.35, -1.0)), stiff=1.2, seed=83, seg=0.013, lift=0.002, hug=0.02),
    Locks(where=lambda p: p.z > -0.04 and p.y < -0.045, spacing=0.012, length=0.028,
          width=0.024, thick=0.01, flow=lip, stiff=1.5, seed=84, seg=0.009, lift=0.002, hug=0.02),
])
