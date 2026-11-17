# SPDX-License-Identifier: AGPL-3.0-or-later
"""A shadow blade for the left hand (Shade's): the right hand's (`weapon.shadow_blade`), mirrored.

    blender -b --python art/wardrobe/offhand/shadow_blade/build.py
"""

import importlib.util
import os

HERE = os.path.dirname(os.path.abspath(__file__))
RIGHT = os.path.normpath(os.path.join(HERE, "..", "..", "weapon", "shadow_blade", "build.py"))

spec = importlib.util.spec_from_file_location("shadow_blade", RIGHT)
blade = importlib.util.module_from_spec(spec)
spec.loader.exec_module(blade)
blade.build(os.path.join(HERE, "item.blend"), side=-1)
