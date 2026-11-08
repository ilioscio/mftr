# SPDX-License-Identifier: AGPL-3.0-or-later
"""Creator builds on their own skeletons (CR10, 20 §3): the steps every non-humanoid base
shares. A build script gives its `creature` bone table, its body (mesh parts), its sliders, its
catalog parts and its clips; `run` builds the base, its shape keys, its parts (following the
sliders), the paint UVs, and exports it all (kind `base`, its clips included).

A build's clips are its move library: besides its stance (`idle`, `run`, `attack_1`, `death`
…) it carries the clips abilities take by their shape (`mftr_forge::moves::own_clip_for`):
`howl` (projectiles, beams, buffs), `slam` (areas), `pounce` (lunges) and the dash set
(`dash_start`, `dash_travel`, `dash_land`).
"""

import os

import bpy

from . import creator, export, mesh, parts, rig


def add_shape_keys(armature, body, sliders):
    """`sliders`: [(id, groups, plus(region) -> deltas, minus or None)]. Face sliders (ids
    starting head, jaw, ears) take the face gain, the rest the body's."""
    body.shape_key_add(name="Basis", from_mix=False)
    ids = []
    for sid, groups, plus, minus in sliders:
        reg = creator.Region(body, set(groups))
        reg.armature = armature.name
        gain = creator.GAIN["face" if sid.startswith(("head", "jaw", "ears")) else "body"]
        up = {i: v * gain for i, v in plus(reg).items()}
        down = {i: v * gain for i, v in minus(reg).items()} if minus else {i: -v for i, v in up.items()}
        for suffix, deltas in (("+", up), ("-", down)):
            key = body.shape_key_add(name=sid + suffix, from_mix=False)
            for i, d in deltas.items():
                key.data[i].co = reg.cos[i] + d
        ids.append(sid)
    return ids


def _lowest(name, bone_table, body):
    """How far the body's lowest point sits above the ground, built on `bone_table`."""
    bpy.ops.wm.read_homefile(use_empty=True)
    rig.use_table(bone_table)
    arm = rig.build_armature(name=name)
    obj = mesh.build_mannequin(arm, name=name, part_list=body())
    return min(v.co.z for v in obj.data.vertices)


def _shifted(bone_table, dz):
    return [(n, p, (h[0], h[1], h[2] - dz), (t[0], t[1], t[2] - dz), d, a) if p is not None else (n, p, h, t, d, a)
            for n, p, h, t, d, a in bone_table]


def run(here, name, bone_table, body, sliders, catalog, clips, ao_height, argv):
    # Stand on the ground (the validator's rule): shift everything but the root down or up.
    dz = _lowest(name, bone_table, body)
    if abs(dz) > 0.002:
        bone_table = _shifted(bone_table, dz)
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig.use_table(bone_table)
    mesh.AO_HEIGHT = ao_height
    arm = rig.build_armature(name=name)
    made_clips = clips(arm)
    body_obj = mesh.build_mannequin(arm, name=name, part_list=body())
    ids = add_shape_keys(arm, body_obj, sliders())
    made = []
    for pid, slot, label, hides, accent, items in catalog():
        obj = mesh.build_mannequin(arm, name=f"part.{pid}", part_list=items)
        n = parts.transfer_shape_keys(body_obj, obj)
        made.append((pid, slot, label, hides, accent, mesh.triangle_count(obj), n))
    head = "head" if any(g.name == "head" for g in body_obj.vertex_groups) else None
    creator.unwrap(body_obj, head_group=head)
    for obj in [o for o in bpy.data.objects if o.name.startswith("part.")]:
        creator.unwrap(obj)
    arm["mftr_build"] = name
    arm["mftr_sliders"] = ",".join(ids)
    print(f"{name}: {mesh.triangle_count(body_obj)} triangles, {len(ids)} sliders, {len(made)} parts, clips {[a.name for a in made_clips]}")
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(here, f"{name}.blend"), compress=True)
    if "--export" in argv:
        print(export.export(os.path.join(here, "export"), name, "base"))
        parts.write_catalog(os.path.join(here, "export", f"{name}.parts.ron"), name, made)
    rig.use_table(None)
