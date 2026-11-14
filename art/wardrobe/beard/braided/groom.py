# SPDX-License-Identifier: AGPL-3.0-or-later
"""A braided beard (BH2): a full beard whose chin is gathered into a plaited braid hanging onto the
chest, a dwarf's. Places are in the source head's frame (`mftr_blender.grooming`)."""

import math

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, beard


def lip(p):
    return Vector((math.copysign(1.0, p.x) * 0.9, -0.25, -0.8))


STYLE = Style(cap=0.009, ramp=0.012, area=beard, blend=0.015, drape=False, band=None, locks=[
    Locks(where=lambda p: p.z < -0.06 and abs(p.x) > 0.02 and p.y < -0.01, spacing=0.022, length=0.05, width=0.05, thick=0.018,
          flow=lambda p: Vector((p.x * 3.0, -0.4, -1.0)), stiff=1.2, seed=85, seg=0.014, lift=0.002, hug=0.02),
    Locks(where=lambda p: p.z > -0.04 and abs(p.x) < 0.03 and p.y < -0.045, spacing=0.012, length=0.036,
          width=0.028, thick=0.012, flow=lip, stiff=1.5, seed=86, seg=0.01, lift=0.002, hug=0.02),
    # The braid, from under the chin.
    Locks(roots=[Vector((0.0, -0.05, -0.094))], length=0.2, width=0.034, thick=0.034,
          flow=lambda p: Vector((0.0, -0.3, -1.0)), stiff=0.6, seed=87, seg=0.018, lift=0.012, taper=0.45,
          plait=True, profile="round", sides=5, hug=0.0, chain="braid"),
])
