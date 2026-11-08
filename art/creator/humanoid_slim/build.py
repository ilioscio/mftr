# SPDX-License-Identifier: AGPL-3.0-or-later
"""The slim humanoid build's bases (CR2, 20 §3–6): one per gender, each with its sliders as
shape keys (`mftr_blender.creator`).

    blender -b --python art/creator/humanoid_slim/build.py -- [--gender feminine|masculine] [--export]

Without `--gender`, builds both. Each lands in `humanoid_slim_<gender>.blend` next to this file;
`--export` also writes `export/humanoid_slim_<gender>.glb` and its `.anims.ron` (kind `base`).
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

import bpy  # noqa: E402

from mftr_blender import creator, export  # noqa: E402

BUILD = "humanoid_slim"


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    genders = [argv[argv.index("--gender") + 1]] if "--gender" in argv else ["feminine", "masculine"]
    for gender in genders:
        name = f"{BUILD}_{gender}"
        creator.build(gender, name)
        bpy.ops.wm.save_as_mainfile(filepath=os.path.join(HERE, f"{name}.blend"), compress=True)
        if "--export" in argv:
            print(export.export(os.path.join(HERE, "export"), name, "base"))


if __name__ == "__main__":
    main()
