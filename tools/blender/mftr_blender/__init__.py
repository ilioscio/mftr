# SPDX-License-Identifier: AGPL-3.0-or-later
"""MFTR Blender tools: the `biped` rig standard, clip and marker helpers, deterministic export,
the marker sidecar and review renders (docs/design/10-characters-and-animation.md §7–§8).

Works as a Blender extension (an "MFTR" tab in the 3D view sidebar) and headless:
    blender -b FILE.blend --python tools/blender/run.py -- <command> [args]
"""


def register():
    from . import ui

    ui.register()


def unregister():
    from . import ui

    ui.unregister()
