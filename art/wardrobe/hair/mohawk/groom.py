# SPDX-License-Identifier: AGPL-3.0-or-later
"""A crest (BH1): a strip down the middle from the brow to the nape, its locks standing up and
swept back; the sides shaved (a thin cap). Places are in the source head's frame
(`mftr_blender.grooming`)."""

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above

HALF = 0.024  # the strip's half width


def flow(p):
    # Up off the skull and back.
    return Vector((0.0, 0.7, 1.0 - max(0.0, p.y - 0.06) * 8.0))


STYLE = Style(cap=0.012, cap_where=lambda p: abs(p.x) < HALF + 0.006, locks=[
    Locks(where=lambda p: abs(p.x) < HALF and above(p) > 0.01, spacing=0.026, length=0.11, width=0.05, thick=0.03,
          flow=flow, stiff=8.0, seed=41, lift=0.004, seg=0.022, hug=0.0, taper=0.85),
])
