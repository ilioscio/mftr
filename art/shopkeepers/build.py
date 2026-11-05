# SPDX-License-Identifier: AGPL-3.0-or-later
"""The shopkeepers' packs (19 §3, K slice 2): each keeper on the `biped` bones in its own shape,
with its nine clips, ready to export as kind `shopkeeper`.

    blender -b --python art/shopkeepers/build.py -- --key goldwick --out art/shopkeepers/goldwick/goldwick.blend
    blender -b art/shopkeepers/goldwick/goldwick.blend --python tools/blender/run.py -- export --id goldwick --kind shopkeeper

The body is the approved block-out (concepts.py) rounded off (`detail`), its colors carrying their
material slots, the team color white so `accent` shades take it. The bones are the biped's,
placed where the keeper's joints are, plus `extra_<name>_1` bones for what moves on its own (a
hat, a bow, a coat tail, a beard, a pack, the props in their hands). The clips are motion.py's
idle, fidgets and reactions, keyed every frame: the arms solved to reach their targets, the feet
planted, springs settled before the first frame.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, HERE)
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Quaternion, Vector  # noqa: E402

import concepts as C  # noqa: E402
import motion as M  # noqa: E402
from mftr_blender import clips, mesh, rig  # noqa: E402

FPS = M.FPS
UP, FORWARD = rig.UP, rig.FORWARD

# Our bone names (motion.py) -> the biped's; anything else is an extra.
BIPED = {
    "root": "root", "body": "pelvis", "head": "head",
    "up_L": "upperarm_l", "fore_L": "forearm_l", "hand_L": "hand_l", "fingers_L": "fingers_l",
    "up_R": "upperarm_r", "fore_R": "forearm_r", "hand_R": "hand_r", "fingers_R": "fingers_r",
    "leg_L": "calf_l", "foot_L": "foot_l", "leg_R": "calf_r", "foot_R": "foot_r",
}
VIRTUAL = ("ik_L", "ik_R")


def bone_name(n):
    if n in BIPED:
        return BIPED[n]
    if n == "world":
        return "root"
    if n[-1].isdigit():
        return "extra_%s_%s" % (n[:-1], n[-1])
    return "extra_%s_1" % n


# --- The skeleton ----------------------------------------------------------------------------

def table(kp):
    """The keeper's bones: (name, parent, head, tail, deform, roll axis), parents first."""
    pos = {n: Vector(h) for n, h, _ in kp.bones}
    par = {n: p for n, _, p in kp.bones}
    pelvis, head = pos["body"], pos["head"]
    sh_z = sum(a[0][2] for a in kp.arms.values()) / 2
    chest = Vector((0.0, pelvis.y + (head.y - pelvis.y) * 0.6, max(pelvis.z + 0.08, min(sh_z - 0.08, head.z - 0.08))))
    sp1, sp2 = pelvis.lerp(chest, 1 / 3), pelvis.lerp(chest, 2 / 3)
    neck = chest.lerp(head, 0.5)
    top = max(z for _, h, _ in kp.bones for z in [h[2]]) + 0.35
    out = [
        ("root", None, Vector((0, 0, 0)), Vector((0, -0.30, 0)), False, UP),
        ("pelvis", "root", pelvis, sp1, True, FORWARD),
        ("spine_01", "pelvis", sp1, sp2, True, FORWARD),
        ("spine_02", "spine_01", sp2, chest, True, FORWARD),
        ("chest", "spine_02", chest, neck, True, FORWARD),
        ("neck", "chest", neck, head, True, FORWARD),
        ("head", "neck", head, head + Vector((0, 0, 0.22)), True, FORWARD),
    ]
    for side, (sh, el, hd, _) in kp.arms.items():
        s = side.lower()
        sgn = 1 if side == "L" else -1
        sh, el, hd = Vector(sh), Vector(el), Vector(hd)
        d = (hd - el).normalized()
        fingers = pos.get("fingers_" + side)
        fhead = fingers if fingers is not None else hd + d * 0.06
        ftail = fhead + (Vector((0, -0.02, 0.10)) if fingers is not None else d * 0.06)
        out += [
            ("clavicle_" + s, "chest", chest + Vector((sgn * 0.03, 0, 0)), sh, True, FORWARD),
            ("upperarm_" + s, "clavicle_" + s, sh, el, True, FORWARD),
            ("forearm_" + s, "upperarm_" + s, el, hd, True, FORWARD),
            ("hand_" + s, "forearm_" + s, hd, fhead, True, FORWARD),
            ("fingers_" + s, "hand_" + s, fhead, ftail, True, FORWARD),
            ("thumb_" + s, "hand_" + s, hd + Vector((sgn * 0.015, -0.02, 0.01)), hd + Vector((sgn * 0.02, -0.06, 0.02)), True, FORWARD),
            ("prop_" + s, "hand_" + s, hd, hd + Vector((0, -0.15, 0)), False, UP),
        ]
    for side, (hip, knee, ankle, toe) in kp.legs.items():
        s = side.lower()
        hip, knee, ankle, toe = Vector(hip), Vector(knee), Vector(ankle), Vector(toe)
        ball = ankle.lerp(toe, 0.6)
        ball.z = toe.z
        out += [
            ("thigh_" + s, "pelvis", hip, knee, True, FORWARD),
            ("calf_" + s, "thigh_" + s, knee, ankle, True, FORWARD),
            ("foot_" + s, "calf_" + s, ankle, ball, True, UP),
            ("toe_" + s, "foot_" + s, ball, toe, True, UP),
        ]
    hr = Vector(kp.arms["R"][2])
    out += [
        ("socket_projectile", "prop_r", hr + Vector((0, -0.20, 0)), hr + Vector((0, -0.30, 0)), False, UP),
        ("socket_weapon_tip", "prop_r", hr + Vector((0, -0.35, 0)), hr + Vector((0, -0.45, 0)), False, UP),
        ("socket_cast", "hand_r", hr + Vector((0, -0.05, 0)), hr + Vector((0, -0.15, 0)), False, UP),
        ("socket_chest", "chest", chest + Vector((0, -0.16, 0)), chest + Vector((0, -0.26, 0)), False, UP),
        ("socket_overhead", "root", Vector((0, 0, top)), Vector((0, -0.10, top)), False, UP),
    ]
    for n, h, p in kp.bones:
        if n in BIPED or n in VIRTUAL or n.startswith(("leg_", "foot_", "hand_")):
            continue
        h = Vector(h)
        out.append((bone_name(n), bone_name(p), h, h + Vector((0, 0, 0.08)), True, FORWARD))
    return out


def armature(kp):
    data = bpy.data.armatures.new("shopkeeper_" + kp.key)
    obj = bpy.data.objects.new("shopkeeper_" + kp.key, data)
    bpy.context.scene.collection.objects.link(obj)
    data.display_type = "STICK"
    obj.show_in_front = True
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    for name, parent, head, tail, deform, axis in table(kp):
        eb = data.edit_bones.new(name)
        eb.head, eb.tail = head, tail
        eb.use_deform = deform
        eb.align_roll(axis)
        if parent:
            eb.parent = data.edit_bones[parent]
            eb.use_connect = (Vector(eb.head) - data.edit_bones[parent].tail).length < 1e-6
    bpy.ops.object.mode_set(mode="OBJECT")
    for pb in obj.pose.bones:
        pb.rotation_mode = "QUATERNION"
    obj["mftr_archetype"] = rig.ARCHETYPE
    obj["mftr_rig_version"] = rig.VERSION
    return obj


# --- The body --------------------------------------------------------------------------------

def body(kp, arm):
    """The block-out rounded off, every face with its slot, every vertex on its bones."""
    C.TEAM = (1.0, 1.0, 1.0)
    C.DETAIL = kp.detail
    fn, _, title = C.KEEPERS[kp.key]
    b = C.B(kp.key)
    fn(b)
    kp.props(b)
    extra = getattr(kp, "extra_parts", None)
    if extra:
        extra(b)
    C.DETAIL = 1.0
    bm = b.bm
    for f, slot in zip(bm.faces, b.slots):
        f.material_index = mesh.MATERIAL_SLOTS.index(slot)
        f.smooth = False
    # Ambient occlusion in the alpha: darker toward the ground (10 §2).
    top = max(v.co.z for v in bm.verts)
    for f in bm.faces:
        for lp in f.loops:
            c = lp[b.col]
            ao = max(0.55, min(1.0, 0.55 + 0.45 * lp.vert.co.z / top))
            lp[b.col] = (c[0], c[1], c[2], ao)
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces[:])
    me = bpy.data.meshes.new(kp.key)
    bm.to_mesh(me)
    bm.free()
    me.color_attributes.active_color = me.color_attributes["Col"]
    me.color_attributes.render_color_index = me.color_attributes.active_color_index
    obj = bpy.data.objects.new(kp.key, me)
    bpy.context.scene.collection.objects.link(obj)
    for slot in mesh.MATERIAL_SLOTS:
        me.materials.append(mesh.material(slot))
    groups = {}
    for i, w in enumerate(b.wts):
        ws = {w: 1.0} if isinstance(w, str) else (w or {"root": 1.0})
        merged = {}
        for n, k in ws.items():
            merged[bone_name(n)] = merged.get(bone_name(n), 0.0) + k
        total = sum(merged.values())
        for n, k in merged.items():
            if n not in groups:
                if n not in arm.data.bones:
                    raise RuntimeError("%s: vertices on `%s`, which the rig lacks" % (kp.key, n))
                groups[n] = obj.vertex_groups.new(name=n)
            groups[n].add([i], k / total, "REPLACE")
    used = {p.material_index for p in me.polygons}
    for i in reversed(range(len(me.materials))):
        if i not in used:
            me.materials.pop(index=i)
    obj.parent = arm
    mod = obj.modifiers.new("Armature", "ARMATURE")
    mod.object = arm
    print("%s: %d triangles" % (title, mesh.triangle_count(obj)))
    return obj


# --- The clips -------------------------------------------------------------------------------

def _pole(S, E, H):
    """The rest elbow's (or knee's) bend: its offset from the line through S and H."""
    axis = (H - S).normalized()
    off = (E - S) - axis * (E - S).dot(axis)
    return off.normalized() if off.length > 1e-6 else Vector((0, -1, 0))


def to_biped(kp, rig, P):
    """Our pose as the armature's: bone -> (turn, offset, scale) in armature axes relative to the
    parent, the arms solved for their targets and the feet planted."""
    W = rig.fk(P)
    out = {}
    for n in rig.names:
        if n in VIRTUAL or n.startswith(("leg_", "foot_")):
            continue
        out[bone_name(n)] = P[n]
    Qb = W["body"].to_quaternion()
    for side, (sh, el, hd, _) in kp.arms.items():
        s = side.lower()
        S = rig.at(W, "body", sh)
        T = W["ik_" + side].translation
        R_up, R_lo, _ = _solve_at(Vector(sh), Vector(el), Vector(hd), S, Qb, T)
        out["upperarm_" + s] = [R_up, Vector(), Vector((1, 1, 1))]
        out["forearm_" + s] = [R_lo, Vector(), Vector((1, 1, 1))]
    Qr = W["root"].to_quaternion()
    for side, (hip, knee, ankle, toe) in kp.legs.items():
        s = side.lower()
        S = rig.at(W, "body", hip)
        T = rig.at(W, "root", ankle)
        R_th, R_ca, Q_ca = _solve_at(Vector(hip), Vector(knee), Vector(ankle), S, Qb, T, pole=Vector((0, -1, 0.05)))
        out["thigh_" + s] = [R_th, Vector(), Vector((1, 1, 1))]
        out["calf_" + s] = [R_ca, Vector(), Vector((1, 1, 1))]
        out["foot_" + s] = [Q_ca.inverted() @ Qr, Vector(), Vector((1, 1, 1))]
    return out


def _solve_at(S0, E0, H0, S, Qp, T, pole=None):
    """Two-bone IK with the chain's rest S0-E0-H0, its root now at S under a parent turned Qp."""
    d0, e0 = E0 - S0, H0 - E0
    L1, L2 = d0.length, e0.length
    to = T - S
    D = max(min(to.length, (L1 + L2) * 0.999), abs(L1 - L2) + 1e-4)
    dirn = to.normalized() if to.length > 1e-6 else (Qp @ (H0 - S0)).normalized()
    pl = Qp @ (pole if pole is not None else _pole(S0, E0, H0))
    p = pl - dirn * pl.dot(dirn)
    p = p.normalized() if p.length > 1e-6 else Vector((0, 0, -1))
    a = (L1 * L1 - L2 * L2 + D * D) / (2 * D)
    h = math.sqrt(max(L1 * L1 - a * a, 0.0))
    mid = S + dirn * a + p * h
    end = S + dirn * D
    up_w = (Qp @ d0).rotation_difference(mid - S)
    R_up = Qp.inverted() @ up_w @ Qp
    Q_up = Qp @ R_up
    lo_w = (Q_up @ e0).rotation_difference(end - mid)
    R_lo = Q_up.inverted() @ lo_w @ Q_up
    return R_up, R_lo, Q_up @ R_lo


def sequence(kp, rig, act, frames, hold=False):
    """Our poses for one clip, frame 0..frames: the idle underneath (`act` None: the idle
    alone, looping), springs settled first."""
    springs = M.Springs(rig)
    dt = 1.0 / FPS
    loop = kp.loop
    warm = int(round(loop * FPS)) * (3 if act is None else 1)
    out = []
    for f in range(-warm, frames + 1):
        t = f * dt
        P = M.Pose(rig.names)
        q = 0.0
        if act is not None and f >= 0:
            u = t
            q = ss_in(u) if hold else min(ss_in(u), ss_in(frames * dt - u))
        kp.idle(t % loop if act is None else t, P, rig, q)
        if act is not None and f >= 0:
            act(t, P, rig)
        springs.apply(P, dt)
        if f >= 0:
            out.append(P)
    if act is None:
        out[-1] = out[0]
    return out


def ss_in(u):
    return M.ss(u / 0.4)


def _bag(arm, act):
    ad = arm.animation_data
    if ad.action_slot is None:
        slot = act.slots.new(id_type="OBJECT", name=arm.name)
        ad.action_slot = slot
    from bpy_extras import anim_utils
    return anim_utils.action_ensure_channelbag_for_slot(act, ad.action_slot).fcurves


def bake(kp, arm, rig, name, seq, loop=False):
    frames = len(seq) - 1
    act = clips.new_action(arm, name, frames, loop=loop)
    poses = [to_biped(kp, rig, P) for P in seq]
    fcs = _bag(arm, act)
    bones = sorted({n for p in poses for n in p})
    for n in bones:
        if n not in arm.data.bones:
            raise RuntimeError("%s: no bone %s" % (kp.key, n))
        M3 = arm.data.bones[n].matrix_local.to_3x3()
        Mi = M3.inverted()
        rq, lc, sc = [], [], []
        prev = None
        for p in poses:
            r, l, s = p.get(n, (Quaternion(), Vector(), Vector((1, 1, 1))))
            q = (Mi @ r.to_matrix() @ M3).to_quaternion()
            if prev is not None and prev.dot(q) < 0:
                q.negate()
            prev = q
            rq.append(q)
            lc.append(Mi @ l)
            sc.append(Vector([abs(x) for x in Mi @ s]))
        moved = lambda vals, rest: any((Vector(v) - Vector(rest)).length > 1e-5 for v in vals)
        for path, data, k, rest in (("rotation_quaternion", rq, 4, (1, 0, 0, 0)), ("location", lc, 3, (0, 0, 0)), ("scale", sc, 3, (1, 1, 1))):
            if not moved([tuple(v) for v in data], rest):
                continue
            for i in range(k):
                fc = fcs.new('pose.bones["%s"].%s' % (n, path), index=i, group_name=n)
                fc.keyframe_points.add(len(data))
                co = []
                for f, v in enumerate(data):
                    co += [f, v[i]]
                fc.keyframe_points.foreach_set("co", co)
                for kpnt in fc.keyframe_points:
                    kpnt.interpolation = "LINEAR"
                fc.update()
    if not loop:
        clips.set_marker(act, "end", frames)
    clips.stash(arm, act)
    return act


def build(key):
    kp = M.KEEPERS[key]
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = FPS
    arm = armature(kp)
    body(kp, arm)
    rig = M.Rig(kp.bones, kp.arms, kp.springs)
    made = [bake(kp, arm, rig, "idle", sequence(kp, rig, None, int(round(kp.loop * FPS))), loop=True)]
    for i, (_, dur, fn) in enumerate(kp.fidgets, start=1):
        made.append(bake(kp, arm, rig, "idle_fidget_%d" % i, sequence(kp, rig, fn, int(round(dur * FPS)))))
    for name, dur, fn in kp.reactions:
        made.append(bake(kp, arm, rig, name, sequence(kp, rig, fn, int(round(dur * FPS)), hold=(name == "defeat"))))
    clips.reset_pose(arm)
    print("%s: %d bones, clips %s" % (key, len(arm.data.bones), [a.name for a in made]))


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    key = argv[argv.index("--key") + 1]
    build(key)
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print("saved", out)
