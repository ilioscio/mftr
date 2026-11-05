# SPDX-License-Identifier: AGPL-3.0-or-later
"""The shopkeepers' stalls (19 §3, K slice 3): map props, like the turrets and the fountain.

    blender -b --python art/shopkeepers/stalls.py -- --key goldwick [--season autumn]
    blender -b art/props/stall_goldwick/stall_goldwick.blend --python tools/blender/run.py -- export --id stall_goldwick --kind prop

Each keeper's stall is two packs the client stacks: `stall_<key>`, the stall on its plinth of the
base's paving (the same in every season), and `stall_<key>_<season>`, what it wears that season
(summer, autumn, winter), its ground the plinth's top (the client lifts it PLINTH_M = 0.18 m).
Snow on the upward facets in winter comes from the prop material.
Built from the approved block-out (concepts.py), rounded off, colors carrying their slots, the
team color white so `accent` takes it. Writes `art/props/<id>/<id>.blend`.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bmesh  # noqa: E402
import bpy  # noqa: E402

import concepts as C  # noqa: E402
from mftr_blender import mesh, prop  # noqa: E402

SEASONS = ("summer", "autumn", "winter")
DETAIL = {"goldwick": 1.8, "rosette": 1.15, "slink": 2.0, "folio": 1.9}
AO_HEIGHT = 2.4


def finish(b, name):
    """The block-out's bmesh as a prop: one mesh on the `root` bone, each face in its slot,
    occlusion in the alpha, faces wound outward."""
    arm = prop._armature(name)
    bm = b.bm
    for f, slot in zip(bm.faces, b.slots):
        f.material_index = mesh.MATERIAL_SLOTS.index(slot)
        f.smooth = False
        for lp in f.loops:
            c = lp[b.col]
            lp[b.col] = (c[0], c[1], c[2], max(0.5, min(1.0, 0.5 + 0.5 * lp.vert.co.z / AO_HEIGHT)))
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    me.color_attributes.active_color = me.color_attributes["Col"]
    me.color_attributes.render_color_index = me.color_attributes.active_color_index
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    for slot in mesh.MATERIAL_SLOTS:
        me.materials.append(mesh.material(slot))
    g = obj.vertex_groups.new(name="root")
    g.add(list(range(len(me.vertices))), 1.0, "REPLACE")
    used = {p.material_index for p in me.polygons}
    for i in reversed(range(len(me.materials))):
        if i not in used:
            me.materials.pop(index=i)
    obj.parent = arm
    mod = obj.modifiers.new("Armature", "ARMATURE")
    mod.object = arm
    print("%s: %d triangles" % (name, mesh.triangle_count(obj)))
    return obj


def build(key, season=None):
    prop.new_scene()
    C.TEAM = (1.0, 1.0, 1.0)
    C.DETAIL = DETAIL[key]
    name = "stall_%s" % key if season is None else "stall_%s_%s" % (key, season)
    b = C.B(name)
    if season is None:
        C.KEEPERS[key][1](b)
    else:
        # A dressing stands on the plinth: authored on its top, placed there by the client.
        b.push((0, 0, -C.H))
        C.DRESSING[key][season](b)
        b.pop()
    C.DETAIL = 1.0
    finish(b, name)
    out = os.path.join(ROOT, "art", "props", name, name + ".blend")
    os.makedirs(os.path.dirname(out), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
    print("saved", out)


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    key = argv[argv.index("--key") + 1]
    build(key, argv[argv.index("--season") + 1] if "--season" in argv else None)
