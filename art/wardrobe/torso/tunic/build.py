# SPDX-License-Identifier: AGPL-3.0-or-later
"""The linen tunic (22 §4, M1): two panels, front and back, sewn at the shoulders and the sides,
flaring to a hem at mid-thigh, simulated onto the reference body. Its hem and openings are
trimmed in the team accent.

    blender -b --python art/wardrobe/torso/tunic/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402


def outline(neck_z):
    """One panel, seen from the front (metres on the reference body: x across, z up). The armholes
    are cut deep and the straps narrow, clear of the shoulders and the arms (the QA harness)."""
    return [
        ("hem_l", -0.27, 0.66), ("hem_r", 0.27, 0.66),
        ("waist_r", 0.205, 0.95), ("under_r", 0.185, 1.09), ("arm_r", 0.14, 1.27), ("shoulder_r", 0.14, 1.41),
        ("neck_r", 0.088, 1.44), ("neck_c", 0.0, neck_z), ("neck_l", -0.088, 1.44),
        ("shoulder_l", -0.14, 1.41), ("arm_l", -0.14, 1.27), ("under_l", -0.185, 1.09), ("waist_l", -0.205, 0.95),
    ]


front = sewing.Panel("front", outline(1.37), y=-0.17, front=True)
back = sewing.Panel("back", outline(1.43), y=0.15, front=False)
seams = [
    (("front", "shoulder_r", "neck_r"), ("back", "shoulder_r", "neck_r")),
    (("front", "neck_l", "shoulder_l"), ("back", "neck_l", "shoulder_l")),
    (("front", "hem_r", "under_r"), ("back", "hem_r", "under_r")),
    (("front", "under_l", "hem_l"), ("back", "under_l", "hem_l")),
]
garment = sewing.Garment([front, back], seams, budget=900, cloth=(0.55, 0.50, 0.40))
sewing.build(garment, os.path.join(HERE, "item.blend"))
