# SPDX-License-Identifier: AGPL-3.0-or-later
"""A ranger's cloak (22 §4, M1; Vesper's): forest green with a teal hem, hanging off the left
shoulder and falling across the back to the knee, held by a leather collar round the neck whose
left side spreads over the shoulder as a pauldron. Simulated onto the reference body. On the
`shoulders` slot, so a quiver can still go on the back.

    blender -b --python art/wardrobe/shoulders/ranger_cloak/build.py
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

from mftr_blender import sewing  # noqa: E402

GREEN = (0.10, 0.24, 0.17)
LEATHER = (0.36, 0.22, 0.12)
TEAL = (0.06, 0.42, 0.40)

# Seen from the front; her left is +x (`_p`), her right -x (`_n`).
# The collar: round the throat, narrow over the right shoulder, spreading over the left one
# down onto the upper arm (the pauldron).
COLLAR = [
    ("low_n", -0.12, 1.37), ("low_c", 0.0, 1.33), ("low_p", 0.16, 1.29), ("arm_p", 0.30, 1.24),
    ("sh_p", 0.265, 1.41), ("neck_p", 0.095, 1.43), ("throat", 0.0, 1.37), ("neck_n", -0.095, 1.43),
    ("sh_n", -0.13, 1.415),
]
# The cloak, behind: from the collar's back edge over both shoulders; long on the left, falling
# to the knee, its right edge running from the right shoulder in and down across the back.
CLOAK = [
    ("hem_n", 0.04, 0.55), ("hem_p", 0.44, 0.52), ("side_p", 0.34, 1.05), ("arm_p", 0.30, 1.24),
    ("sh_p", 0.265, 1.41), ("neck_p", 0.095, 1.43), ("neck_n", -0.095, 1.43), ("sh_n", -0.13, 1.415),
    ("edge_n", -0.11, 1.33), ("mid_n", 0.0, 1.0),
]

panels = [sewing.Panel("collar", COLLAR, y=-0.17, front=True, color=LEATHER),
          sewing.Panel("cloak", CLOAK, y=0.20, front=False)]
seams = [
    (("collar", "sh_n", "neck_n"), ("cloak", "sh_n", "neck_n")),
    (("collar", "neck_p", "arm_p"), ("cloak", "neck_p", "arm_p")),
]
garment = sewing.Garment(panels, seams, budget=700, cloth=GREEN, trim_color=TEAL, trim_slot="cloth",
                         thickness=0.005, mirror=None)
sewing.build(garment, os.path.join(HERE, "item.blend"))
