# SPDX-License-Identifier: AGPL-3.0-or-later
"""The "MFTR" tab in the 3D view sidebar: rig generation, clip and marker helpers, export,
validation and review renders, the same operations as the headless commands."""

import os
import shutil
import subprocess

import bpy

from . import clips, export, mesh, review, rig


def _repo_root():
    d = os.path.dirname(bpy.data.filepath) if bpy.data.filepath else ""
    while d and os.path.dirname(d) != d:
        if os.path.exists(os.path.join(d, "Cargo.toml")) and os.path.isdir(os.path.join(d, "art")):
            return d
        d = os.path.dirname(d)
    return None


def _mftr_tools(root):
    exe = "mftr-tools.exe" if os.name == "nt" else "mftr-tools"
    for sub in ("target/release", "target/debug", "target/dist"):
        p = os.path.join(root, sub, exe)
        if os.path.exists(p):
            return [p]
    if shutil.which("cargo"):
        return ["cargo", "run", "-q", "--release", "-p", "mftr-tools", "--"]
    return None


class MFTR_OT_new_rig(bpy.types.Operator):
    bl_idname = "mftr.new_rig"
    bl_label = "Add biped v1 rig + template"
    bl_description = "Add the biped v1 reference skeleton and the faceted template mesh"
    bl_options = {"REGISTER", "UNDO"}

    def execute(self, context):
        arm = rig.build_armature()
        man = mesh.build_mannequin(arm)
        self.report({"INFO"}, f"{len(arm.data.bones)} bones, {mesh.triangle_count(man)} triangles")
        return {"FINISHED"}


class MFTR_OT_new_clip(bpy.types.Operator):
    bl_idname = "mftr.new_clip"
    bl_label = "New clip"
    bl_description = "Create a clip on the MFTR armature and stash it on its own NLA track"
    bl_options = {"REGISTER", "UNDO"}

    name: bpy.props.StringProperty(name="Name", default="idle")
    frames: bpy.props.IntProperty(name="Frames", default=30, min=1)
    loop: bpy.props.BoolProperty(name="Loops", default=False)
    layer: bpy.props.EnumProperty(name="Layer", items=[(x, x, "") for x in clips.LAYERS])

    def invoke(self, context, event):
        return context.window_manager.invoke_props_dialog(self)

    def execute(self, context):
        arm = export.find_armature()
        act = clips.new_action(arm, self.name, self.frames, loop=self.loop, layer=self.layer)
        if not self.loop:
            clips.set_marker(act, "end", self.frames)
        clips.stash(arm, act)
        arm.animation_data.action = act
        return {"FINISHED"}


class MFTR_OT_marker(bpy.types.Operator):
    bl_idname = "mftr.marker"
    bl_label = "Set marker"
    bl_description = "Put the marker on the current frame of the active clip"
    bl_options = {"REGISTER", "UNDO"}

    name: bpy.props.StringProperty(name="Marker", default="fire")

    def execute(self, context):
        arm = export.find_armature()
        act = arm.animation_data.action if arm.animation_data else None
        if not act:
            self.report({"ERROR"}, "no active clip on the armature")
            return {"CANCELLED"}
        clips.set_marker(act, self.name, context.scene.frame_current - int(act.frame_range[0]))
        return {"FINISHED"}


class MFTR_OT_export(bpy.types.Operator):
    bl_idname = "mftr.export"
    bl_label = "Export + validate"
    bl_description = "Export <id>.glb and <id>.anims.ron next to the .blend, then run mftr-tools pack validate"

    def execute(self, context):
        if not bpy.data.filepath:
            self.report({"ERROR"}, "save the .blend first")
            return {"CANCELLED"}
        s = context.scene.mftr
        r = export.export(os.path.join(os.path.dirname(bpy.data.filepath), "export"), s.pack_id, s.kind)
        root = _repo_root()
        cmd = _mftr_tools(root) if root else None
        if not cmd:
            self.report({"WARNING"}, f"exported {r['bytes']} bytes; mftr-tools not found, run `mftr-tools pack validate`")
            return {"FINISHED"}
        proc = subprocess.run(cmd + ["pack", "validate", r["glb"]], cwd=root, capture_output=True, text=True)
        print(proc.stdout + proc.stderr)
        level = {"INFO"} if proc.returncode == 0 else {"ERROR"}
        self.report(level, (proc.stdout.strip().splitlines() or ["validate: no output"])[-1])
        return {"FINISHED"}


class MFTR_OT_review(bpy.types.Operator):
    bl_idname = "mftr.review"
    bl_label = "Review renders (active clip)"
    bl_description = "Contact sheet + GIF of the active clip from the gameplay and close-up cameras"

    def execute(self, context):
        arm = export.find_armature()
        act = arm.animation_data.action if arm.animation_data else None
        out = os.path.join(os.path.dirname(bpy.data.filepath) or bpy.app.tempdir, "review")
        names = review.render_clips(out, [act.name] if act else None)
        self.report({"INFO"}, f"rendered {', '.join(names)} into {out}")
        return {"FINISHED"}


class MFTR_Settings(bpy.types.PropertyGroup):
    pack_id: bpy.props.StringProperty(name="Id", default="biped_library")
    kind: bpy.props.EnumProperty(name="Kind", items=[(x, x, "") for x in ("champion", "library", "rig")])


class MFTR_PT_panel(bpy.types.Panel):
    bl_label = "MFTR"
    bl_space_type = "VIEW_3D"
    bl_region_type = "UI"
    bl_category = "MFTR"

    def draw(self, context):
        col = self.layout.column(align=True)
        col.operator(MFTR_OT_new_rig.bl_idname, icon="ARMATURE_DATA")
        col.separator()
        col.operator(MFTR_OT_new_clip.bl_idname, icon="ACTION")
        row = col.row(align=True)
        for m in ("fire", "end", "loop_in", "loop_out", "foot_l", "foot_r"):
            if m in ("loop_in", "foot_l"):
                row = col.row(align=True)
            row.operator(MFTR_OT_marker.bl_idname, text=m).name = m
        col.separator()
        col.prop(context.scene.mftr, "pack_id")
        col.prop(context.scene.mftr, "kind")
        col.operator(MFTR_OT_export.bl_idname, icon="EXPORT")
        col.operator(MFTR_OT_review.bl_idname, icon="RENDER_ANIMATION")


_classes = (MFTR_Settings, MFTR_OT_new_rig, MFTR_OT_new_clip, MFTR_OT_marker, MFTR_OT_export, MFTR_OT_review,
            MFTR_PT_panel)


def register():
    for c in _classes:
        bpy.utils.register_class(c)
    bpy.types.Scene.mftr = bpy.props.PointerProperty(type=MFTR_Settings)


def unregister():
    del bpy.types.Scene.mftr
    for c in reversed(_classes):
        bpy.utils.unregister_class(c)
