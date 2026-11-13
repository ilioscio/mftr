# SPDX-License-Identifier: AGPL-3.0-or-later
"""A moustache (BH2): the lip above the mouth, its locks swept out from the middle and drooping
past the mouth's corners. Places are in the source head's frame (`mftr_blender.grooming`)."""

import math

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, beard


def area(p):
    return min(beard(p), p.z + 0.041, 0.038 - abs(p.x))


def lip(p):
    return Vector((math.copysign(1.0, p.x) * 1.0, -0.2, -0.6))


STYLE = Style(cap=0.005, ramp=0.006, area=area, blend=0.015, drape=False, locks=[
    Locks(where=lambda p: p.y < -0.045, spacing=0.011, length=0.04, width=0.026, thick=0.011, flow=lip,
          stiff=1.5, seed=88, seg=0.01, lift=0.002, hug=0.02),
])
