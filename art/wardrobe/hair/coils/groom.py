# SPDX-License-Identifier: AGPL-3.0-or-later
"""Coils (BH1): a rounded crown of tight coils standing off the skull: a deep cap, and round
knots of coils standing a little out of it all over. Places are in the source head's frame
(`mftr_blender.grooming`)."""

from mathutils import Vector

from mftr_blender.grooming import Locks, Style, above

CENTRE = Vector((0.0, 0.03, 0.03))


def flow(p):
    # Straight out from the head's middle.
    return p - CENTRE


STYLE = Style(cap=0.038, ramp=0.045, lengths=None, locks=[
    Locks(where=lambda p: above(p) > 0.025, spacing=0.034, length=0.024, width=0.05, thick=0.05, flow=flow,
          stiff=8.0, seed=71, seg=0.012, lift=0.026, profile="round", sides=5, taper=0.35, hug=0.0),
])
