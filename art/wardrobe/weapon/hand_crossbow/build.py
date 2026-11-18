# SPDX-License-Identifier: AGPL-3.0-or-later
"""A hand crossbow (Wren's): a wooden stock laid along the hand, forward from the fist, an iron
prod swept back across its front with lime-wrapped tips, the string drawn back to a nut, an iron
crank on its side, and a bolt on top with glowing lime fletching.

Modelled in its grip frame with `mftr_blender.armory` (22 §4.7, BW7), as the hand-made Wren's is
laid on his hand: +Z forward along the hand (the fist aims it), the top (where the bolt lies)
toward `UP`.

    blender -b --python art/wardrobe/weapon/hand_crossbow/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import armory  # noqa: E402

WOOD = (0.32, 0.20, 0.10)
WOOD_DARK = (0.20, 0.12, 0.06)
IRON = (0.34, 0.35, 0.38)
LIME = (0.42, 0.62, 0.10)
CREAM = (0.68, 0.62, 0.48)
GLOW = (0.66, 1.0, 0.20)

UP = Vector((0.0, -1.0, 0.0))     # the crossbow's top, in the grip frame
SIDE = Vector((1.0, 0.0, 0.0))
FWD = Vector((0.0, 0.0, 1.0))
PROD = 0.30                       # how far forward of the fist the prod sits
SPAN = 0.22                       # each limb's reach


def at(z, up=0.0, side=0.0):
    return FWD * z + UP * up + SIDE * side


k = armory.Kit()
# The stock: from behind the fist, thickening a little toward the front, and its iron nose.
k.branch([(tuple(at(-0.12, 0.012)), 0.020), (tuple(at(0.0, 0.020)), 0.019), (tuple(at(0.20, 0.028)), 0.017),
          (tuple(at(PROD, 0.030)), 0.016)], WOOD, sides=6)
k.branch([(tuple(at(PROD - 0.02, 0.030)), 0.019), (tuple(at(PROD + 0.04, 0.030)), 0.017)], IRON, slot="metal", sides=6)
# The prod: two limbs out from the nose and swept back, lime at the tips.
for s in (1, -1):
    path = [(tuple(at(PROD, 0.032, 0.0)), 0.011), (tuple(at(PROD - 0.01, 0.032, s * SPAN * 0.5)), 0.009),
            (tuple(at(PROD - 0.05, 0.032, s * SPAN * 0.85)), 0.007), (tuple(at(PROD - 0.08, 0.032, s * SPAN)), 0.006)]
    k.branch(path, IRON, slot="metal", sides=4)
    k.branch([(tuple(at(PROD - 0.065, 0.032, s * SPAN * 0.93)), 0.009), (tuple(at(PROD - 0.085, 0.032, s * SPAN * 1.02)), 0.0)],
             LIME, sides=4)
    # The string, from the tip back to the nut.
    k.branch([(tuple(at(PROD - 0.08, 0.032, s * SPAN)), 0.0018), (tuple(at(0.06, 0.044, 0.0)), 0.0018)], CREAM, sides=3)
# The nut and the crank.
k.box(tuple(at(0.06, 0.042)), (0.018, 0.016, 0.016), IRON)
k.branch([(tuple(at(0.10, 0.028, 0.0)), 0.008), (tuple(at(0.10, 0.028, 0.045)), 0.008)], IRON, slot="metal", sides=4)
k.branch([(tuple(at(0.10, 0.028, 0.045)), 0.005), (tuple(at(0.05, 0.060, 0.045)), 0.005)], IRON, slot="metal", sides=4)
# The bolt on top: a dark shaft, an iron head, glowing fletching at its back.
k.branch([(tuple(at(0.05, 0.050)), 0.004), (tuple(at(PROD + 0.06, 0.050)), 0.004)], WOOD_DARK, sides=4)
k.branch([(tuple(at(PROD + 0.06, 0.050)), 0.007), (tuple(at(PROD + 0.09, 0.050)), 0.0)], IRON, slot="metal", sides=4)
k.slab([(-0.014, 0.0), (0.0, 0.0), (0.0, 0.05), (-0.008, 0.045)], 0.0015, GLOW, slot="emissive",
       at=tuple(at(0.05, 0.052)), turn=None)
k.faces = [(f, c, "metal" if c == IRON else s) for f, c, s in k.faces]
k.save(os.path.join(HERE, "item.blend"))
