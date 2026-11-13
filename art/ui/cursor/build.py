# SPDX-License-Identifier: AGPL-3.0-or-later
"""The game cursor (05 §6): the steel gauntlet (`mftr_blender.gauntlet`) on the slim masculine
base's right hand, posed pointing, rendered in Blender and outlined.

    blender -b art/creator/humanoid_slim/humanoid_slim_masculine.blend --python art/ui/cursor/build.py

Writes `client/cursor/<state>.png` (128 px masters; the game scales them to the screen) and
`client/cursor/hotspots.gd` (each image's click point, its fingertip, in its pixels).
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402

from mftr_blender import cursor_render  # noqa: E402

OUT = os.path.join(ROOT, "client", "cursor")

if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = argv[0] if argv else OUT
    cursor_render.render_all(out)
