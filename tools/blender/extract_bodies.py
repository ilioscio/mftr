# SPDX-License-Identifier: AGPL-3.0-or-later
"""Extract the creator's body source (22 §1.1, BW3) from Blender Studio's Human Base Meshes bundle
(CC0, v1.4.1): the realistic male and female bodies, un-subdivided twice, centred, in
`art/creator/bodies/body_source.blend`, their base cages first fitted to the sculpt (with the
sculpted surface at its first level as `body_<sex>_hi`); and each one's joints, from the bundle's "primitive"
body of the same sex (a body cut into parts whose origins are its joints' pivots), printed as
the `JOINTS` table of `mftr_blender.cc0_body`.

    blender -b BUNDLE.blend --python tools/blender/extract_bodies.py

Run once; the output is committed. The bundle isn't (49 MB): see `art/creator/bodies/README.md`.
"""

import os

import bpy

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.normpath(os.path.join(HERE, "..", "..", "art", "creator", "bodies", "body_source.blend"))

# The primitive parts whose origins are the joints, by the names `cc0_body.JOINTS` uses.
PARTS = {
    "pelvis": "pelvis", "belly": "spine", "chest": "chest", "neck": "neck", "head": "head",
    "shoulder.L": "clavicle_l", "arm_upper.L": "shoulder_l", "arm_lower.L": "elbow_l", "hand.L": "wrist_l",
    "finger_middle.L": "knuckles_l", "thumb.L": "thumb_l", "leg_upper.L": "hip_l", "leg_lower.L": "knee_l",
    "foot.005.L": "ankle_l", "toe_middle.L": "toes_l",
}


def main():
    bodies = []
    for sex in ("male", "female"):
        src = bpy.data.objects[f"GEO-body_{sex}_realistic"]
        x0 = src.location.x
        # The sculpted surface: the multires at its first level (about 42k vertices), the
        # surface the reduced body is fitted to and its chest is projected onto
        # (`cc0_body.densify`). Its vertices and faces only.
        multi = next(m for m in src.modifiers if m.type == "MULTIRES")
        multi.levels = 1
        dg = bpy.context.evaluated_depsgraph_get()
        ev = bpy.data.meshes.new_from_object(src.evaluated_get(dg))
        hi_me = bpy.data.meshes.new(f"body_{sex}_hi")
        hi_me.from_pydata([v.co.copy() for v in ev.vertices], [], [list(p.vertices) for p in ev.polygons])
        bpy.data.meshes.remove(ev)
        hi = bpy.data.objects.new(f"body_{sex}_hi", hi_me)
        bpy.context.scene.collection.objects.link(hi)
        bodies.append(hi)
        # The base cage fitted to the sculpt (Blender's "Apply Base"): the multires base's own
        # vertices sit off the sculpted surface, which made the reduced body lumpy.
        for o in bpy.context.view_layer.objects:
            o.select_set(False)
        bpy.context.view_layer.objects.active = src
        src.select_set(True)
        bpy.ops.object.multires_base_apply(modifier=multi.name)
        me = src.data.copy()
        obj = bpy.data.objects.new(f"body_{sex}", me)
        bpy.context.scene.collection.objects.link(obj)
        # Un-subdivided twice (21 §1.2's way for the head).
        mod = obj.modifiers.new("unsub", "DECIMATE")
        mod.decimate_type = "UNSUBDIV"
        mod.iterations = 2
        dg = bpy.context.evaluated_depsgraph_get()
        reduced = bpy.data.meshes.new_from_object(obj.evaluated_get(dg))
        obj.modifiers.clear()
        obj.data = reduced
        reduced.name = f"body_{sex}"
        # The mesh is centred on its own middle already; the bundle lays the objects out along X.
        obj.location = (0.0, 0.0, 0.0)
        bodies.append(obj)
        # The joints: the primitive body's part origins, from its pelvis's middle.
        pelvis = bpy.data.objects[f"GEO-pelvis_{sex}_primitive_realistic"].matrix_world.translation
        print(f"JOINTS {sex} = {{")
        for part, name in PARTS.items():
            base, side = (part[:-2], part[-1]) if part.endswith((".L", ".R")) else (part, "")
            tail = f".{side}" if side else ""
            # The male parts are `GEO-<part>_male_primitive_realistic`, the female's
            # `GEO-<part>_primitive_female_realistic`; the feet `foot.005` and `foot`.
            o = None
            for b in (base, base.replace(".005", "")):
                o = (bpy.data.objects.get(f"GEO-{b}_{sex}_primitive_realistic{tail}")
                     or bpy.data.objects.get(f"GEO-{b}_primitive_{sex}_realistic{tail}")) or o
            if o is None:
                print(f"    # {name}: no part {part}")
                continue
            p = o.matrix_world.translation
            print(f'    "{name}": ({p.x - pelvis.x:.4f}, {p.y:.4f}, {p.z:.4f}),')
        print("}")
        print(f"{sex}: {len(reduced.vertices)} vertices, {sum(len(p.vertices) - 2 for p in reduced.polygons)} triangles,"
              f" from x {x0:.3f}")
    # Only the two bodies in the file.
    keep = {o.name for o in bodies}
    for o in list(bpy.data.objects):
        if o.name not in keep:
            bpy.data.objects.remove(o)
    for coll in list(bpy.data.collections):
        bpy.data.collections.remove(coll)
    bpy.ops.outliner.orphans_purge(do_recursive=True)
    bpy.ops.wm.save_as_mainfile(filepath=OUT, compress=True)
    print("saved", OUT)


main()
