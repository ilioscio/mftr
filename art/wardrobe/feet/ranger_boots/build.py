# SPDX-License-Identifier: AGPL-3.0-or-later
"""A ranger's boots (22 §4; Vesper's): knee-high leather, a folded cuff flaring at the top, a
darker strap round the ankle and dark soles. A shell of the reference body's legs below the knee
(`mftr_blender.shell`), the toes smoothed into the boot's toe box.

    blender -b --python art/wardrobe/feet/ranger_boots/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import shell  # noqa: E402

LEATHER = (0.36, 0.22, 0.12)
CUFF = (0.48, 0.33, 0.20)
STRAP = (0.20, 0.12, 0.07)
SOLE = (0.12, 0.08, 0.05)
TOP = 0.44        # the cuff's top, under the knee (the reference knee is at 0.477)
CUFF_FROM = 0.39  # where the folded cuff starts
FLARE = 0.012     # how far the cuff stands out at its top


def keep(p):
    return p.z < TOP and abs(p.x) < 0.35


def shape(p, n):
    # The cuff flares out toward its top.
    if p.z > CUFF_FROM:
        t = (p.z - CUFF_FROM) / (TOP - CUFF_FROM)
        return Vector((n.x, n.y, 0.0)).normalized() * FLARE * t
    return Vector()


def paint(p):
    if p.z < 0.022:
        return SOLE, "cloth"
    if p.z > CUFF_FROM:
        return CUFF, "cloth"
    if 0.13 < p.z < 0.16:
        return STRAP, "cloth"
    return LEATHER, "cloth"


spec = shell.Shell(keep, lift=0.009, thickness=0.004, smooth=14, smooth_where=lambda p: p.z < 0.10, shape=shape,
                   paint=paint, budget=1000)
shell.build(spec, os.path.join(HERE, "item.blend"))
