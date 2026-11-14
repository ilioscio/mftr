# SPDX-License-Identifier: AGPL-3.0-or-later
"""A ranger's bracers (22 §4; Vesper's): leather from below the elbow to above the wrist, two
teal bands, flaring a little at the elbow end. A shell of the reference body's forearms
(`mftr_blender.shell`).

    blender -b --python art/wardrobe/arms/ranger_bracers/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import shell  # noqa: E402

LEATHER = (0.36, 0.22, 0.12)
TEAL = (0.06, 0.42, 0.40)
# On the reference body the forearms are what lies out past the torso's sides at their height:
# from just above the wrist to below the elbow.
SIDE = 0.23
LOW, HIGH = 0.93, 1.09
BANDS = ((1.045, 1.07), (0.945, 0.965))
FLARE = 0.008


def keep(p):
    return abs(p.x) > SIDE and LOW <= p.z <= HIGH


def shape(p, n):
    # A little flare at the elbow end.
    if p.z > HIGH - 0.02:
        return n * FLARE * (p.z - (HIGH - 0.02)) / 0.02
    return Vector()


def paint(p):
    if any(a <= p.z <= b for a, b in BANDS):
        return TEAL, "cloth"
    return LEATHER, "cloth"


spec = shell.Shell(keep, lift=0.007, thickness=0.004, smooth=3, shape=shape, paint=paint, budget=800)
shell.build(spec, os.path.join(HERE, "item.blend"))
