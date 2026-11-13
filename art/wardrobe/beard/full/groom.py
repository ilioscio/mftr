# SPDX-License-Identifier: AGPL-3.0-or-later
"""A full beard (BH2): the jaw, the chin, the cheeks below the beard line and the lip above the
mouth, with locks falling from the chin and the jaw and a moustache over the lip. It follows the
jaw (its weights are the skin's right under it). Places are in the source head's frame
(`mftr_blender.grooming`)."""

import math

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, beard


def lip(p):
    # Out from the middle of the lip and down past the mouth's corners.
    return Vector((math.copysign(1.0, p.x) * 0.9, -0.25, -0.8))


STYLE = Style(cap=0.007, ramp=0.012, area=beard, blend=0.015, drape=False, locks=[
    # From the chin and the jaw, down and a little forward.
    Locks(where=lambda p: p.z < -0.062 and p.y < -0.01, spacing=0.022, length=0.06, width=0.05, thick=0.016,
          flow=lambda p: Vector((p.x * 3.0, -0.4, -1.0)), stiff=1.2, seed=81, seg=0.015, lift=0.002, hug=0.02),
    # The moustache.
    Locks(where=lambda p: p.z > -0.04 and abs(p.x) < 0.03 and p.y < -0.045, spacing=0.012, length=0.032,
          width=0.026, thick=0.011, flow=lip, stiff=1.5, seed=82, seg=0.009, lift=0.002, hug=0.02),
])
