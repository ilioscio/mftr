# SPDX-License-Identifier: AGPL-3.0-or-later
"""Headless entry point for the MFTR Blender tools.

    blender -b [FILE.blend] --python tools/blender/run.py -- <command> [options]

Commands:
    new-rig     --out FILE.blend                Generate the biped v1 rig + template mesh
    new-library --out FILE.blend                Same, plus the shared library block-out clips
    export      --id ID --kind champion|library|rig [--out DIR]
                                                Export the open file (default DIR: ./export
                                                next to the .blend)
    review      [--out DIR] [--clips a,b]       Review renders (default DIR: ./review next to
                                                the .blend, git-ignored)
"""

import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import bpy  # noqa: E402

from mftr_blender import export, library, mesh, review, rig  # noqa: E402


def _args():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    if not argv:
        print(__doc__)
        sys.exit(2)
    cmd, opts, i = argv[0], {}, 1
    while i < len(argv):
        if not argv[i].startswith("--"):
            sys.exit(f"unexpected argument {argv[i]}")
        opts[argv[i][2:]] = argv[i + 1]
        i += 2
    return cmd, opts


def _here(sub):
    return os.path.join(os.path.dirname(bpy.data.filepath), sub)


def _new_file(with_library):
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    arm = rig.build_armature()
    man = mesh.build_mannequin(arm)
    names = library.blockout(arm) if with_library else []
    print(f"rig: {len(arm.data.bones)} bones, mesh: {mesh.triangle_count(man)} triangles, clips: {names}")
    return arm


def main():
    cmd, opts = _args()
    if cmd in ("new-rig", "new-library"):
        _new_file(cmd == "new-library")
        out = os.path.abspath(opts["out"])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
    elif cmd == "export":
        r = export.export(opts.get("out") or _here("export"), opts["id"], opts["kind"])
        print(f"exported {r}")
    elif cmd == "review":
        names = opts["clips"].split(",") if "clips" in opts else None
        out = opts.get("out") or _here("review")
        print(f"rendered {review.render_clips(out, names)} into {out}")
    else:
        sys.exit(f"unknown command {cmd}")


main()
