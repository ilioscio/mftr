# SPDX-License-Identifier: AGPL-3.0-or-later
"""Animated previews of the shopkeeper concepts (19 §6, K slice 1): each block-out on a quick rig,
playing its idle and its three fidgets, so the owner can judge personality and motion before
modeling starts.

    blender -b --python art/shopkeepers/previews.py -- --out DIR [--only KEY,KEY] [--still SECONDS] [--reactions]

Throwaway like the block-outs (concepts.py): rigid parts on a few bones, the arms on two-bone IK,
and springs for everything that should lag and settle (hats, bows, coat tails, beards, packs,
lanterns). Nothing here is the real rig: that's the `biped` bones of 19 §3, after approval.

Writes into DIR one video per keeper, `<key>_preview.mp4` (3/4 close-up at their stall, labeled
with what's playing), or with --still a PNG per keeper at that many seconds. --reactions plays
the five reactions (greet, purchase, farewell, victory, defeat) instead of the fidgets.
"""

import math
import os
import sys

import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import concepts as C  # noqa: E402
from motion import FPS, KEEPERS, Pose, Rig, Springs, ss  # noqa: E402

FIRST_IDLE, GAP, LAST_IDLE = 3.5, 2.2, 2.5
REACTIONS = False
SEASON = "summer"


# --- Building ------------------------------------------------------------------------------

def _clear(sc):
    for ob in list(sc.collection.all_objects):
        bpy.data.objects.remove(ob, do_unlink=True)
    for coll in (bpy.data.meshes, bpy.data.armatures, bpy.data.curves, bpy.data.cameras, bpy.data.actions):
        for d in list(coll):
            if d.users == 0:
                coll.remove(d)


def _setup(sc):
    sc.render.engine = "BLENDER_WORKBENCH"
    sc.display_settings.display_device = "sRGB"
    sc.view_settings.view_transform = "Standard"
    sc.view_settings.exposure = 0.75
    sh = sc.display.shading
    sh.light = "STUDIO"
    sh.color_type = "VERTEX"
    sh.show_shadows = True
    sh.shadow_intensity = 0.45
    sh.show_cavity = True
    sh.cavity_type = "WORLD"
    sh.show_object_outline = True
    sh.object_outline_color = (0.02, 0.02, 0.02)
    sc.display.shadow_focus = 0.2
    sc.display.light_direction = (0.35, -0.45, 0.82)
    sc.render.film_transparent = False
    if sc.world is None:
        sc.world = bpy.data.worlds.new("previews")
    sc.world.color = (0.45, 0.47, 0.50)
    sc.render.fps = FPS


def _armature(sc, kp, rig):
    data = bpy.data.armatures.new(kp.key + "_rig")
    ob = bpy.data.objects.new(kp.key + "_rig", data)
    sc.collection.objects.link(ob)
    bpy.context.view_layer.objects.active = ob
    ob.select_set(True)
    bpy.ops.object.mode_set(mode="EDIT")
    eb = data.edit_bones
    w = eb.new("world")
    w.head, w.tail = (0, 0, 0), (0, 0, 0.1)
    made = {"world": w}
    pending = list(kp.bones)
    arms_made = False
    while pending:
        progressed = False
        for item in list(pending):
            name, head, parent = item
            if parent not in made:
                continue
            b = eb.new(name)
            b.head = Vector(head)
            b.tail = Vector(head) + Vector((0, 0, 0.08))
            b.parent = made[parent]
            b.inherit_scale = "NONE"
            made[name] = b
            pending.remove(item)
            progressed = True
        if not arms_made and all(par in made for (_, _, _, par) in kp.arms.values()):
            for side, (sh, el, hd, par) in kp.arms.items():
                u = eb.new("up_" + side)
                u.head, u.tail = Vector(sh), Vector(el)
                u.parent = made[par]
                u.inherit_scale = "NONE"
                f = eb.new("fore_" + side)
                f.head, f.tail = Vector(el), Vector(hd)
                f.parent = u
                f.inherit_scale = "NONE"
                made["up_" + side], made["fore_" + side] = u, f
            arms_made = True
            progressed = True
        if not progressed:
            raise RuntimeError("unresolved bones: %s" % pending)
    bpy.ops.object.mode_set(mode="OBJECT")
    for pb in ob.pose.bones:
        pb.rotation_mode = "QUATERNION"
    for side in kp.arms:
        c = ob.pose.bones["fore_" + side].constraints.new("IK")
        c.target = ob
        c.subtarget = "ik_" + side
        c.chain_count = 2
        c.use_tail = True
        c.use_stretch = False
    return ob


def build(sc, kp, team=C.ALLY):
    """The stall, and the keeper on its rig standing at the plinth's front."""
    C.TEAM = team
    fn, stall_fn, title = C.KEEPERS[kp.key]
    s = C.B(kp.key + "_stall")
    stall_fn(s)
    C.DRESSING[kp.key][SEASON](s)
    so = s.done(sc.collection)
    b = C.B(kp.key)
    fn(b)
    kp.props(b)
    me = b.done(sc.collection)
    for o in (so, me):
        for p in o.data.polygons:
            p.use_smooth = False
    rig = Rig(kp.bones, kp.arms, kp.springs)
    arm = _armature(sc, kp, rig)
    missing = [g.name for g in me.vertex_groups if g.name not in arm.data.bones]
    if missing:
        raise RuntimeError("%s: vertex groups without bones: %s" % (kp.key, missing))
    me.parent = arm
    mod = me.modifiers.new("rig", "ARMATURE")
    mod.object = arm
    arm.parent = so
    arm.location = (*C.KEEP_AT[kp.key], 0.18)
    return so, arm, me, rig, title


def _acts(kp):
    return kp.reactions if REACTIONS else kp.fidgets


def timeline(kp):
    segs, t = [], FIRST_IDLE
    for i, (label, dur, _) in enumerate(_acts(kp)):
        segs.append((t, t + dur, i))
        t += dur + GAP
    return segs, t - GAP + LAST_IDLE


def poses(kp, rig, segs, n_frames):
    springs = Springs(rig)
    out = []
    for f in range(n_frames):
        t = f / FPS
        P = Pose(rig.names)
        active = None
        for a, b, i in segs:
            if a <= t < b:
                active = (i, t - a, b - a)
        q = 0.0
        if active:
            i, u, d = active
            q = min(ss(u / 0.4), ss((d - u) / 0.4))
        kp.idle(t, P, rig, q)
        if active:
            _acts(kp)[active[0]][2](active[1], P, rig)
        springs.apply(P, 1.0 / FPS)
        out.append(P)
    return out


def _channelbag(ob, name):
    ad = ob.animation_data_create()
    act = bpy.data.actions.new(name)
    try:
        slot = act.slots.new(id_type="OBJECT", name=ob.name)
        ad.action = act
        ad.action_slot = slot
        from bpy_extras import anim_utils
        return anim_utils.action_ensure_channelbag_for_slot(act, slot).fcurves
    except AttributeError:
        ad.action = act
        return act.fcurves


def bake(arm, rig, seq):
    """Key every frame of `seq` (frame 1 on) onto the armature."""
    names = [n for n in rig.names]
    M = {n: arm.data.bones[n].matrix_local.to_3x3() for n in names}
    Mi = {n: M[n].inverted() for n in names}
    fcs = _channelbag(arm, arm.name + "_preview")
    prev = {}
    for n in names:
        rq, lc, sc = [], [], []
        for f, P in enumerate(seq):
            r, l, s = P[n]
            qq = (Mi[n] @ r.to_matrix() @ M[n]).to_quaternion()
            if n in prev and prev[n].dot(qq) < 0:
                qq.negate()
            prev[n] = qq
            rq.append(qq)
            lc.append(Mi[n] @ l)
            sv = Mi[n] @ s
            sc.append(Vector([abs(x) for x in sv]))
        for path_, data, k in (("rotation_quaternion", rq, 4), ("location", lc, 3), ("scale", sc, 3)):
            for i in range(k):
                fc = fcs.new('pose.bones["%s"].%s' % (n, path_), index=i)
                co = []
                for f, v in enumerate(data):
                    co += [f + 1, v[i]]
                fc.keyframe_points.add(len(data))
                fc.keyframe_points.foreach_set("co", co)
                for kp in fc.keyframe_points:
                    kp.interpolation = "LINEAR"
                fc.update()


def _camera(sc, look, dist, yaw=-28.0, pitch=16.0, lens=50.0):
    cam = bpy.data.cameras.new("cam")
    cam.lens = lens
    ob = bpy.data.objects.new("cam", cam)
    sc.collection.objects.link(ob)
    y, p = math.radians(yaw), math.radians(pitch)
    look = Vector(look)
    ob.location = look + Vector((math.sin(y) * math.cos(p), -math.cos(y) * math.cos(p), math.sin(p))) * dist
    ob.rotation_euler = (look - ob.location).to_track_quat("-Z", "Y").to_euler()
    sc.camera = ob
    return ob


def _label(sc, cam, text, pos, size):
    cu = bpy.data.curves.new("label", "FONT")
    cu.body = text
    cu.size = size
    tmp = bpy.data.objects.new("label_tmp", cu)
    sc.collection.objects.link(tmp)
    bpy.context.view_layer.update()
    dg = bpy.context.evaluated_depsgraph_get()
    me = bpy.data.meshes.new_from_object(tmp.evaluated_get(dg))
    bpy.data.objects.remove(tmp, do_unlink=True)
    col = me.color_attributes.new("Col", "FLOAT_COLOR", "CORNER")
    for d in col.data:
        d.color = (0.95, 0.95, 0.92, 1.0)
    me.color_attributes.active_color_name = "Col"
    ob = bpy.data.objects.new("label", me)
    sc.collection.objects.link(ob)
    ob.parent = cam
    ob.location = pos
    return ob


def _show_between(ob, f0, f1):
    ob.hide_render = True
    ob.keyframe_insert("hide_render", frame=1)
    ob.hide_render = False
    ob.keyframe_insert("hide_render", frame=f0)
    ob.hide_render = True
    ob.keyframe_insert("hide_render", frame=f1)


def preview(kp, out, still=None):
    sc = bpy.context.scene
    _clear(sc)
    _setup(sc)
    g = C.B("ground")
    C.box(g, (0, 0, -0.05), (80, 80, 0.1), (0.07, 0.14, 0.035))
    g.done(sc.collection)
    so, arm, me, rig, title = build(sc, kp)
    segs, total = timeline(kp)
    n = int(round(total * FPS))
    if still is not None:
        n = int(round(still * FPS)) + 1
    seq = poses(kp, rig, segs, n)
    bake(arm, rig, seq)
    kx, ky = C.KEEP_AT[kp.key]
    cam = _camera(sc, (kx, ky, 0.18 + kp.look), kp.dist)
    sens_h = 36.0 * 9 / 16
    hh = sens_h / 2 / cam.data.lens          # half the view height at 1 m
    hw = 36.0 / 2 / cam.data.lens
    _label(sc, cam, title, (-hw + 0.02, hh - 0.045, -1), 0.032)
    idle = _label(sc, cam, "Idle", (-hw + 0.02, -hh + 0.02, -1), 0.024)
    shown = []
    for a, b, i in segs:
        lb = _label(sc, cam, _acts(kp)[i][0].capitalize(), (-hw + 0.02, -hh + 0.02, -1), 0.024)
        _show_between(lb, int(a * FPS) + 1, int(b * FPS) + 1)
        shown.append((int(a * FPS) + 1, int(b * FPS) + 1))
    idle.hide_render = False
    idle.keyframe_insert("hide_render", frame=1)
    for f0, f1 in shown:
        idle.hide_render = True
        idle.keyframe_insert("hide_render", frame=f0)
        idle.hide_render = False
        idle.keyframe_insert("hide_render", frame=f1)
    sc.render.resolution_x, sc.render.resolution_y = 1280, 720
    sc.render.resolution_percentage = 100
    sc.frame_start, sc.frame_end = 1, n
    if still is not None:
        sc.frame_set(n)
        sc.render.image_settings.file_format = "PNG"
        sc.render.filepath = os.path.join(out, "%s_still_%.1f.png" % (kp.key, still))
        bpy.ops.render.render(write_still=True)
        print("wrote", sc.render.filepath)
        return
    s = sc.render
    try:
        s.image_settings.media_type = "VIDEO"
    except (AttributeError, TypeError):
        pass
    s.image_settings.file_format = "FFMPEG"
    s.ffmpeg.format = "MPEG4"
    s.ffmpeg.codec = "H264"
    s.ffmpeg.constant_rate_factor = "HIGH"
    s.ffmpeg.ffmpeg_preset = "GOOD"
    s.filepath = os.path.join(out, kp.key + ("_reactions.mp4" if REACTIONS else "_preview.mp4"))
    bpy.ops.render.render(animation=True)
    print("wrote", s.filepath, n, "frames")


def main(out, only=None, still=None):
    os.makedirs(out, exist_ok=True)
    for key in (only or list(KEEPERS)):
        preview(KEEPERS[key], out, still)


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.join(HERE, "review")
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    still = float(argv[argv.index("--still") + 1]) if "--still" in argv else None
    REACTIONS = "--reactions" in argv
    SEASON = argv[argv.index("--season") + 1] if "--season" in argv else "summer"
    main(out, only, still)
