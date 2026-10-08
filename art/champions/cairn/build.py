# SPDX-License-Identifier: AGPL-3.0-or-later
"""Cairn (A12): the block-out generator for his model and clips.

    blender -b --python art/champions/cairn/build.py -- --out art/champions/cairn/cairn.blend

An antlered earth-warden on `biped` v1: tall and spare, long grey hair tied back, green stripes
of paint across his eyes, antlers rising from his brow, a moss-green cloak on an `extra_cloak`
chain over bark-brown leathers, and a staff topped with a small cairn of stacked stones bound
with ribbons in the team color (laid along his right hand bone, aimed per pose). Like the other
block-outs this is a starting point.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

from mftr_blender import clips, head, mesh, rig  # noqa: E402
from mftr_blender.clips import merge, sym  # noqa: E402
from mftr_blender.library import _clip, arms, cycle, run_half  # noqa: E402

# Palette (concept.md): moss and bark, stone, bone.
SKIN = (0.72, 0.56, 0.44)
HAIR = (0.68, 0.66, 0.6)
MOSS = (0.24, 0.34, 0.18)
MOSS_DARK = (0.16, 0.24, 0.12)
BARK = (0.32, 0.22, 0.14)
LEATHER = (0.4, 0.28, 0.17)
STONE = (0.5, 0.5, 0.47)
STONE_DARK = (0.36, 0.36, 0.34)
ANTLER = (0.78, 0.72, 0.6)
PAINT = (0.3, 0.55, 0.28)
ACCENT = (0.18, 0.52, 0.95)

CLOAK = [(0, 0.14, 1.52), (0, 0.19, 1.2), (0, 0.22, 0.86), (0, 0.24, 0.5)]


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", (0, 0, 0.9), (0, 0, 1.13), 12, [(0, .13, .1), (.4, .16, .11), (1, .155, .11)], "cloth", BARK),
        ("pelvis", (0, 0, 1.06), (0, 0, 1.12), 12, [(0, .162, .114), (1, .158, .112)], "accent", ACCENT),
        # A kilt of hanging leather strips over the hips.
        ("pelvis", (0, -0.1, 1.06), (0, -0.13, 0.78), 4, [(0, .15, .02), (1, .16, .02)], "cloth", LEATHER),
        ("spine_01", (0, 0, 1.11), (0, 0, 1.28), 12, [(0, .13, .094), (.5, .122, .09), (1, .13, .094)], "cloth", LEATHER),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .13, .094), (1, .155, .104)], "cloth", LEATHER),
        ("chest", (0, 0, 1.40), (0, 0, 1.61), 12, [(0, .158, .106), (.35, .168, .11), (.7, .158, .104), (1, .09, .074)], "cloth", BARK),
        # The cloak's mantle and a stone clasp.
        ("chest", (0, 0.015, 1.47), (0, 0.005, 1.6), 14, [(0, .19, .135), (.55, .165, .12), (1, .1, .085)], "cloth", MOSS),
        ("chest", (0, -0.1, 1.56), (0, -0.125, 1.56), 6, [(0, .03, .03), (.5, .034, .034), (1, 0, 0)], "cloth", STONE),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.66), 8, [(0, .05, .05), (1, .045, .045)], "skin", SKIN),
        ("extra_cloak_1", CLOAK[0], CLOAK[1], 6, [(0, .17, .022), (1, .2, .022)], "cloth", MOSS),
        ("extra_cloak_2", CLOAK[1], CLOAK[2], 6, [(0, .2, .022), (1, .22, .022)], "cloth", MOSS),
        ("extra_cloak_3", CLOAK[2], CLOAK[3], 6, [(0, .22, .022), (.7, .2, .02), (1, .14, .016)], "cloth", MOSS_DARK),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 8, [(0, .054, .054), (1, .046, .046)], "cloth", LEATHER),
        (fa, (fa, 0), (fa, 1), 8, [(0, .044, .044), (1, .038, .038)], "skin", SKIN),
        (fa, (fa, .45), (fa, .95), 8, [(0, .05, .05), (1, .046, .046)], "cloth", BARK),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .038, .024), (1, .042, .021)], "skin", SKIN),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .042, .021), (1, .028, .015)], "skin", SKIN),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .017, .017), (1, .011, .011)], "skin", SKIN),
        (th, (0.10, 0, 1.02), (th, 1), 10, [(0, .084, .084), (.5, .072, .072), (1, .055, .055)], "cloth", BARK),
        (ca, (ca, 0), (ca, .5), 10, [(0, .056, .056), (1, .052, .052)], "cloth", BARK),
        (ca, (ca, .35), (ca, 1), 10, [(0, .062, .062), (.15, .058, .058), (1, .05, .054)], "cloth", LEATHER),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .052, .046), (1, .046, .032)], "cloth", LEATHER),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .046, .028), (1, .03, .018)], "cloth", LEATHER),
    ]


def _staff():
    """The warden's staff along the right hand bone, a small cairn of stacked stones at its head,
    bound with team-colored ribbons."""
    h = Vector(next(b[2] for b in rig.bone_table() if b[0] == "hand_r"))
    t = Vector(next(b[3] for b in rig.bone_table() if b[0] == "hand_r"))
    axis = (t - h).normalized()
    grip = h + (t - h) * 0.6
    out = [("hand_r", tuple(grip - axis * 0.6), tuple(grip + axis * 0.9), 6, [(0, .016, .016), (.5, .02, .02), (1, .022, .022)], "cloth", BARK)]
    base = grip + axis * 0.9
    for i, (r, hgt, color) in enumerate(((0.065, 0.07, STONE_DARK), (0.055, 0.06, STONE), (0.042, 0.055, STONE_DARK), (0.03, 0.045, STONE))):
        lift = sum(x[1] for x in ((0.065, 0.07), (0.055, 0.06), (0.042, 0.055), (0.03, 0.045))[:i])
        a = base + axis * lift
        out.append(("hand_r", tuple(a), tuple(a + axis * hgt), 6, [(0, r * 0.8, r * 0.75), (.5, r, r * 0.9), (1, r * 0.75, r * 0.7)], "cloth", color))
    out.append(("hand_r", tuple(base - axis * 0.05), tuple(base - axis * 0.12), 6, [(0, .03, .03), (1, .028, .028)], "accent", ACCENT))
    # Ribbon tails hanging from the binding.
    side = axis.cross(Vector((0, 1, 0)))
    side = side.normalized() if side.length > 1e-3 else Vector((1, 0, 0))
    for s in (1, -1):
        a = base - axis * 0.08 + side * 0.03 * s
        out.append(("hand_r", tuple(a), tuple(a - axis * 0.22 + side * 0.04 * s), 4, [(0, .02, .005), (1, .018, .004)], "accent", ACCENT))
    return out


def _head(bm, layers, groups, mats):
    # The shared sculpted head, weathered and long-faced: grey hair tied back, green paint
    # across the eyes, antlers rising from the brow.
    hd = head.Head(base=(0, -0.01, 1.64), height=0.26, scale=1.0, jaw=0.4, chin=1.05, eye=0.9,
                   skin=SKIN, hair=HAIR, hair_style="ponytail", eye_color=(0.35, 0.45, 0.25), lips=(0.56, 0.38, 0.32))
    head.build(bm, layers, groups, mats, hd)
    import bmesh
    deform, col = layers

    def paint(verts, faces, color, slot):
        for v in verts:
            v[deform][groups["head"]] = 1.0
            v[col] = (*color, 1.0)
        for f in faces:
            f.material_index = mats[slot]
            f.smooth = False

    # Paint: two green stripes down each cheek and one down the brow.
    k = hd.scale
    for x, t, rot in ((0.03, 0.44, 8), (0.046, 0.43, 14), (-0.03, 0.44, -8), (-0.046, 0.43, -14), (0.0, 0.8, 0)):
        p = hd.front(t, x * k) + Vector((0, -0.002, 0))
        vs = head._box(bm, p, Vector((0.007, 0.004, 0.036 if x else 0.06)) * k, rot=(0, rot, 0))
        paint(vs, {f for v in vs for f in v.link_faces}, PAINT, "skin")
    # Antlers: a main beam each side with two tines.
    for s in (1, -1):
        root = hd.surface(0.86, 0 if s > 0 else math.pi) + Vector((0, -0.01, 0))
        beam = [root, root + Vector((0.08 * s, 0.02, 0.1)), root + Vector((0.12 * s, 0.05, 0.22)), root + Vector((0.11 * s, 0.08, 0.32))]
        tines = [(beam[1], beam[1] + Vector((0.02 * s, -0.06, 0.1))), (beam[2], beam[2] + Vector((-0.03 * s, -0.05, 0.1)))]
        for a, b, r0, r1 in [(beam[0], beam[1], 0.018, 0.015), (beam[1], beam[2], 0.015, 0.012), (beam[2], beam[3], 0.012, 0.0)] + \
                [(ta, tb, 0.01, 0.0) for ta, tb in tines]:
            vs, fs = _prism(bm, a, b, r0, r1)
            paint(vs, fs, ANTLER, "cloth")


def _prism(bm, a, b, r0, r1):
    """A small 5-sided tapering prism from `a` to `b` (antler segments)."""
    import bmesh
    a, b = Vector(a), Vector(b)
    axis = (b - a).normalized()
    ref = Vector((1, 0, 0)) if abs(axis.x) < 0.9 else Vector((0, 0, 1))
    u = (ref - axis * ref.dot(axis)).normalized()
    v = axis.cross(u)
    r0v = [bm.verts.new(a + (u * math.cos(2 * math.pi * k / 5) + v * math.sin(2 * math.pi * k / 5)) * r0) for k in range(5)]
    if r1 > 0:
        r1v = [bm.verts.new(b + (u * math.cos(2 * math.pi * k / 5) + v * math.sin(2 * math.pi * k / 5)) * r1) for k in range(5)]
        faces = [bm.faces.new((r0v[k], r0v[(k + 1) % 5], r1v[(k + 1) % 5], r1v[k])) for k in range(5)]
        faces.append(bm.faces.new(r1v))
        verts = r0v + r1v
    else:
        tip = bm.verts.new(b)
        faces = [bm.faces.new((r0v[k], r0v[(k + 1) % 5], tip)) for k in range(5)]
        verts = r0v + [tip]
    faces.append(bm.faces.new(list(reversed(r0v))))
    bmesh.ops.recalc_face_normals(bm, faces=faces)
    return verts, faces


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + _staff() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def cloak(back=0.0, sway=0.0):
    return {"extra_cloak_1": (back * 0.5, sway * 0.5, 0), "extra_cloak_2": (back * 0.3, sway * 0.3, 0),
            "extra_cloak_3": (back * 0.25, sway * 0.25, 0)}


def STAFF(out, fwd, up):
    return {"hand_r": clips.aim(out, fwd, up, "r")}


STAND = merge(sym("thigh", -3, 0, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"spine_01": (2, 0, 0)})
CROUCH = merge(sym("thigh", -16, 0, 0), sym("calf", 28, 0, 0), sym("foot", -12, 0, 0), {"spine_01": (10, 0, 0), "spine_02": (3, 0, 0)})
PLANTED = merge(arms((0.3, 0.1, -1), (0.25, 0.3, -1), (0.4, 0.15, -0.9), (0.2, 0.5, -0.1)), STAFF(0.05, 0.05, 1))
GUARD = merge(arms((0.2, 0.6, -0.5), (-0.4, 0.8, 0.1), (0.3, 0.5, -0.7), (0.0, 0.9, -0.1)), STAFF(-0.6, 0.6, 0.5))
READY = merge(CROUCH, GUARD, cloak(4))
DOWN = {"pelvis": (0, 0, -0.06)}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, PLANTED, cloak(0), {"head": (-2, 0, 0)})
    b = merge(STAND, PLANTED, cloak(1.5, 2), sym("clavicle", 0, -3, 0), {"head": (-5, 0, 5)})
    return _clip(arm, "idle", 80, [(0, a, None), (40, b, {"pelvis": (0, 0, 0.005)})], loop=True)


def idle_fidget_1(arm):
    # Kneels to lay a palm on the ground and listen.
    base = merge(STAND, PLANTED)
    low = merge(CROUCH, sym("thigh", -40, 0, 0), sym("calf", 70, 0, 0), PLANTED, arm_l(0.3, 0.6, -0.8, 0.1, 0.4, -0.9), {"spine_01": (30, 0, 0), "head": (-10, 0, 14)})
    keys = [(0, base, None), (14, low, {"pelvis": (0, 0, -0.25)}), (40, merge(low, {"head": (-14, 0, -10)}), {"pelvis": (0, 0, -0.25)}), (58, base, None)]
    return _clip(arm, "idle_fidget_1", 58, keys)


def idle_fidget_2(arm):
    # Turns a stone of the staff's cairn, setting it straight.
    base = merge(STAND, PLANTED)
    fix = merge(STAND, PLANTED, arm_l(0.2, 0.4, 0.6, -0.5, 0.1, 0.9), {"head": (-18, 0, -8)})
    keys = [(0, base, None), (12, fix, None), (24, merge(fix, {"hand_l": (0, 0, 30)}), None), (32, fix, None), (44, base, None)]
    return _clip(arm, "idle_fidget_2", 44, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (22, merge(READY, {"spine_01": (13, 0, 0)}, cloak(6, 1)), {"pelvis": (0, 0, -0.08)})]
    return _clip(arm, "idle_ready", 44, keys, loop=True)


def run(arm):
    keys = []
    for f, p, l in cycle(run_half(), 20):
        p = {k: v for k, v in p.items() if not (k.endswith("_r") and k.split("_")[0] in ("upperarm", "forearm"))}
        keys.append((f, merge(p, arm_r(0.3, 0.2, -0.9, 0.1, 0.8, 0.0), STAFF(0.05, 0.5, 0.85), cloak(24 + (4 if f % 10 < 5 else 0))), l))
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(arm):
    def scale(pose):
        out = {k: ((v[0] * 1.2, v[1], v[2]) if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple) else v) for k, v in pose.items()}
        out["spine_01"] = (14, 0, pose.get("spine_01", (0, 0, 0))[2])
        return {k: v for k, v in out.items() if not (k.endswith("_r") and k.split("_")[0] in ("upperarm", "forearm"))}

    half = [(round(f * 0.8), scale(p), l) for f, p, l in run_half()]
    keys = [(f, merge(p, arm_r(0.3, 0.2, -0.9, 0.1, 0.8, 0.0), STAFF(0.05, 0.6, 0.8), cloak(34)), l) for f, p, l in cycle(half, 16)]
    return _clip(arm, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(arm):
    # An overhead strike with the stone-headed staff (`fire` 14, sim 14.5).
    up = merge(CROUCH, arms((0.25, 0.0, 1), (0.1, -0.4, 0.9), (0.3, -0.1, 1), (0.1, -0.5, 0.8)), STAFF(0.0, -0.7, 0.7), {"spine_02": (-10, 0, 0)})
    hit = merge(CROUCH, arms((0.1, 0.9, -0.3), (0.0, 1, -0.4), (0.2, 0.9, -0.3), (0.0, 1, -0.4)), STAFF(0.0, 0.8, -0.6), {"spine_01": (16, 0, 0), "spine_02": (8, 0, 0)}, cloak(-16))
    keys = [(0, READY, DOWN), (9, up, {"pelvis": (0, 0.02, 0.02)}), (14, hit, {"pelvis": (0, -0.05, -0.1)}), (20, hit, {"pelvis": (0, -0.05, -0.1)}), (32, READY, DOWN)]
    return _clip(arm, "attack_1", 32, keys, markers={"fire": 14})


def attack_2(arm):
    # A sweeping side blow.
    wind = merge(CROUCH, arms((0.5, -0.4, -0.2), (0.6, -0.3, 0.2), (0.7, -0.5, 0.0), (0.7, -0.4, 0.3)), STAFF(0.8, -0.5, 0.2), {"spine_01": (6, 0, -30)})
    sweep = merge(CROUCH, arms((0.6, 0.6, -0.1), (0.8, 0.5, 0.0), (-0.3, 0.9, -0.2), (-0.5, 0.8, 0.0)), STAFF(-0.85, 0.5, -0.1), {"spine_01": (8, 0, 28)})
    keys = [(0, READY, DOWN), (9, wind, DOWN), (14, sweep, {"pelvis": (0, -0.03, -0.08)}), (20, sweep, DOWN), (32, READY, DOWN)]
    return _clip(arm, "attack_2", 32, keys, markers={"fire": 14})


def q(arm):
    # Stone Lash: the staff whipped forward, sending a line of stone shards (`fire` 8).
    wind = merge(CROUCH, arm_r(0.5, -0.3, 0.4, 0.4, -0.2, 0.7), arm_l(0.3, 0.6, -0.5, 0.0, 0.9, 0.0), STAFF(0.2, -0.6, 0.7), {"spine_02": (-4, 0, -18)})
    lash = merge(CROUCH, arm_r(0.1, 1, 0.0, 0.0, 1, -0.1), arm_l(0.4, 0.2, -0.8, 0.2, 0.5, -0.8), STAFF(0.0, 1, -0.3), {"spine_02": (8, 0, 14)})
    keys = [(0, READY, DOWN), (4, wind, DOWN), (8, lash, {"pelvis": (0, -0.04, -0.06)}), (14, lash, DOWN), (22, READY, DOWN)]
    return _clip(arm, "q", 22, keys, markers={"fire": 8})


def w(arm):
    # Shelter: instant, a pulse from `fire` 4: the free hand raised toward the ally, palm out.
    give = merge(arm_l(0.2, 0.9, 0.2, 0.0, 0.9, 0.4), {"spine_02": (4, 0, 0), "head": (-4, 0, 0)})
    keys = [(0, READY, DOWN), (4, merge(READY, give), DOWN), (14, merge(READY, give), DOWN), (22, READY, DOWN)]
    return _clip(arm, "w", 22, keys, layer="upper", markers={"fire": 4})


def e(arm):
    # Rockfall: he points the staff at the sky over the target and calls the stones down (`fire` 8).
    call = merge(STAND, arm_r(0.2, 0.6, 0.8, 0.1, 0.5, 0.9), arm_l(0.4, 0.4, 0.4, 0.2, 0.5, 0.7), STAFF(0.0, 0.6, 0.8), {"head": (-14, 0, 0)})
    down = merge(CROUCH, arm_r(0.2, 0.9, 0.0, 0.0, 1, -0.2), arm_l(0.3, 0.7, -0.4, 0.0, 0.9, -0.3), STAFF(0.0, 1, -0.2), {"spine_01": (10, 0, 0)})
    keys = [(0, READY, DOWN), (5, call, {"pelvis": (0, 0, 0.02)}), (8, down, {"pelvis": (0, -0.03, -0.06)}), (14, down, DOWN), (22, READY, DOWN)]
    return _clip(arm, "e", 22, keys, markers={"fire": 8})


def r(arm):
    # Monolith: the staff raised in both hands and driven into the ground; stone erupts around
    # him on the area's delay (`fire` 8).
    raise_ = merge(STAND, arms((0.3, 0.1, 1), (0.1, 0.2, 1), (0.25, 0.1, 1), (0.05, 0.2, 1)), STAFF(0.0, 0.0, 1), {"head": (-14, 0, 0), "spine_02": (-8, 0, 0)})
    drive = merge(sym("thigh", -50, 0, 0), sym("calf", 80, 0, 0), sym("foot", -26, 0, 0), arms((0.15, 0.7, -0.6), (0.0, 0.5, -0.9), (0.2, 0.7, -0.6), (0.0, 0.5, -0.9)),
                  STAFF(0.0, 0.15, 1), {"spine_01": (24, 0, 0), "spine_02": (12, 0, 0)}, cloak(-30))
    keys = [(0, READY, DOWN), (5, raise_, {"pelvis": (0, 0, 0.04)}), (8, drive, {"pelvis": (0, -0.04, -0.34)}), (22, drive, {"pelvis": (0, -0.04, -0.35)}), (32, READY, DOWN)]
    return _clip(arm, "r", 32, keys, markers={"fire": 8})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (10, 0, 0), "head": (16, 0, 0)}, PLANTED, cloak(-6))


def recall(arm):
    keys = [(0, merge(STAND, PLANTED), None), (15, KNEEL, {"pelvis": (0, 0, -0.46)}), (45, merge(KNEEL, {"head": (22, 0, 0)}), {"pelvis": (0, 0, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.46)}), (90, merge(STAND, PLANTED), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    keys = [
        (0, merge(STAND, PLANTED), None),
        (8, merge(STAND, arms((0.5, 0.3, -0.4), (0.3, 0.6, 0.0)), STAFF(0.3, 0.2, 0.9), {"spine_01": (-8, 0, 0), "head": (-16, 0, 0)}), None),
        (18, merge(sym("thigh", 0, 0, 0), sym("calf", 95, 0, 0), arms((0.2, 0.4, -1), (0.15, 0.5, -1)), STAFF(0.4, 0.4, -0.8), {"spine_01": (12, 0, 0), "head": (24, 0, 0)}),
         {"pelvis": (0, 0, -0.48)}),
        (30, merge(sym("thigh", -20, 0, 0), sym("calf", 100, 0, 0), arms((0.6, 0.1, -0.5), (0.5, 0.2, -0.6)), STAFF(0.9, 0.2, -0.3), {"pelvis": (0, 70, 0), "spine_01": (4, 12, 0)},
                   cloak(10, 10)), {"pelvis": (0.3, 0, -0.72)}),
        (38, merge(sym("thigh", -30, 0, 0), sym("calf", 80, 0, 0), arms((0.8, 0.1, -0.2), (0.7, 0.2, -0.2)), STAFF(1, 0.1, 0.0), {"pelvis": (0, 86, 0), "head": (0, 10, 0)},
                   cloak(20, 15)), {"pelvis": (0.45, 0, -0.86)}),
    ]
    return _clip(arm, "death", 38, keys)


def respawn(arm):
    keys = [(0, KNEEL, {"pelvis": (0, 0, -0.46)}), (12, merge(STAND, PLANTED, {"head": (-12, 0, 0)}, cloak(8)), {"pelvis": (0, 0, -0.05)}), (24, merge(STAND, PLANTED), None)]
    return _clip(arm, "respawn", 24, keys)


def select(arm):
    # Strikes the staff on the ground and lifts his face; the antlers silhouette against the sky.
    keys = [(0, merge(STAND, PLANTED), None), (10, merge(STAND, arm_r(0.3, 0.3, 0.2, 0.1, 0.5, 0.6), STAFF(0.0, 0.2, 1)), {"pelvis": (0, 0, 0.02)}),
            (14, merge(STAND, PLANTED, {"head": (-10, 0, 0)}), {"pelvis": (0, 0, -0.02)}), (34, merge(STAND, PLANTED, {"head": (-22, 0, 0)}), None),
            (45, merge(STAND, PLANTED), None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, PLANTED)
    out = merge(base, arm_l(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-5, 0, 0)})
    curl = merge(base, arm_l(0.2, 1, 0.0, 0.1, 0.5, 0.8), {"head": (-5, 0, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Hangs his cloak on his own antler, then thinks better of it.
    base = merge(STAND, PLANTED)
    reach = merge(STAND, PLANTED, arm_l(0.3, 0.1, 0.8, -0.3, 0.0, 1), {"head": (0, 0, 14)}, cloak(-10, 10))
    keys = [(0, base, None), (14, reach, None), (30, reach, None), (40, merge(base, {"head": (6, 0, -6)}), None), (50, base, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    belly = arm_l(0.15, 0.3, -0.9, -0.7, 0.6, 0.2)
    keys = [(0, merge(STAND, PLANTED), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, PLANTED, belly, {"spine_01": (-6 * abs(s), 0, 0), "head": (-14 * abs(s), 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, merge(STAND, PLANTED), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    # A slow stamping circle dance around his planted staff.
    a = merge(PLANTED, {"thigh_l": (-30, 0, 0), "calf_l": (40, 0, 0), "pelvis": (0, -4, 30)}, cloak(6, -6))
    b = merge(PLANTED, {"thigh_r": (-30, 0, 0), "calf_r": (40, 0, 0), "pelvis": (0, 4, -30)}, cloak(6, 6))
    keys = [(0, a, {"pelvis": (0, 0, 0.02)}), (12, merge(STAND, PLANTED), {"pelvis": (0, 0, -0.03)}), (24, b, {"pelvis": (0, 0, 0.02)}),
            (36, merge(STAND, PLANTED), {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]
SHARED = ["walk", "cast_utility", "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed", "cc_sleep", "cc_forced_move"]


def bake_shared(arm):
    """Copy the shared library's clips in (10 §7.3): the same skeleton, so a direct copy."""
    path = os.path.join(ROOT, "art", "library", "biped", "biped_library.blend")
    with bpy.data.libraries.load(path, link=False) as (src, dst):
        dst.actions = [n for n in src.actions if n in SHARED]
    for act in dst.actions:
        act.use_fake_user = True
        clips.stash(arm, act)
    return [a.name for a in dst.actions]


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig.use_shape("standard", scale=1.0)
    arm = rig.build_armature(name="cairn")
    rig.add_chain(arm, "cloak", "chest", CLOAK)
    for f in OWN:
        f(arm)
    body = mesh.build_mannequin(arm, name="cairn", part_list=parts())
    shared = bake_shared(arm)
    print(f"cairn: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(bpy.data.actions)} actions, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
