# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shadow blade (Shade's, the right hand's): a long curved hand blade held forward, punch-dagger
style. It leaves the front of the fist past the knuckles over a dark steel guard and sweeps out
to the side, its outer edge burning violet; a wrapped grip and a violet stone for a pommel.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7): the grip at the origin, +Z
along the hand toward the knuckles and on along the blade. The left hand's blade
(`offhand.shadow_blade`) is this one mirrored: `build(path, side=-1)`.

    blender -b --python art/wardrobe/weapon/shadow_blade/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import armory  # noqa: E402

STEEL = (0.42, 0.44, 0.50)
STEEL_DARK = (0.20, 0.21, 0.25)
LEATHER = (0.18, 0.12, 0.10)
VIOLET = (0.78, 0.36, 1.0)

# The blade from the guard to the point: (z, half-width), and how far it has swept out (+x) by
# each station's z.
BLADE = [(0.035, 0.022), (0.12, 0.030), (0.24, 0.031), (0.36, 0.026), (0.46, 0.016), (0.53, 0.0)]
SWEEP = 0.10


def sweep(z):
    t = max(0.0, (z - BLADE[0][0]) / (BLADE[-1][0] - BLADE[0][0]))
    return SWEEP * t * t


def build(path, side=1):
    k = armory.Kit()
    # The grip across the fist (the fist's middle at the origin), a violet stone below it.
    k.lathe([(-0.07, 0.0), (-0.065, 0.016), (-0.05, 0.016), (0.0, 0.015), (0.02, 0.016)], LEATHER, sides=6)
    k.gem((0, 0, -0.082), 0.012, VIOLET, stretch=1.3)
    # The guard over the knuckles: a dark steel plate across the blade's root, flared at the sides.
    k.slab([(-0.050, 0.020), (-0.030, 0.012), (0.030, 0.012), (0.050, 0.020), (0.036, 0.040), (-0.036, 0.040)],
           0.010, STEEL_DARK, "metal", bevel=0.005)
    # The blade, swept out to the side; then its outer edge, a thin line of violet.
    rings = k.blade(BLADE, STEEL, ridge=0.007)
    edge = k.blade([(z, w * 0.35) for z, w in BLADE[1:]], VIOLET, slot="emissive", ridge=0.0025)
    for ring in rings:
        for v in ring:
            v.co.x += sweep(v.co.z)
    for (z, w), ring in zip(BLADE[1:], edge):
        for v in ring:
            # Along the outer edge, standing a hair out of it.
            v.co.x += sweep(v.co.z) + w * 0.80
    if side < 0:
        for v in k.bm.verts:
            v.co.x = -v.co.x
        # Mirrored, the faces turn inside out: `save` recalculates their normals.
    k.save(path)


if __name__ == "__main__":
    build(os.path.join(HERE, "item.blend"))
