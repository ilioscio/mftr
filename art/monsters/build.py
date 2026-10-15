# SPDX-License-Identifier: AGPL-3.0-or-later
"""Jungle monsters (16): the block-out generators, one module per camp.

    blender -b --python art/monsters/build.py -- --id warden --out art/monsters/warden/warden.blend
    blender -b art/monsters/warden/warden.blend --python tools/blender/run.py -- export --id warden --kind monster

Ids are the sim's monster keys: warden, brute, hound_alpha, hound, toad, raven_alpha, raven,
crawler_elder, crawler. Like the other block-outs, a starting point: run once, then polish the
.blend.
"""

import importlib
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))
sys.path.insert(0, HERE)

import bpy  # noqa: E402

CAMPS = {
    "warden": "warden",
    "brute": "brute",
    "hound_alpha": "hounds",
    "hound": "hounds",
    "toad": "toad",
    "raven_alpha": "ravens",
    "raven": "ravens",
    "crawler_elder": "crawlers",
    "crawler": "crawlers",
}


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    pid = argv[argv.index("--id") + 1]
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    importlib.import_module(CAMPS[pid]).build(pid)
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
