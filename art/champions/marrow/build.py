# SPDX-License-Identifier: AGPL-3.0-or-later
"""Marrow (rebuilt for 14): the generator for her model and clips.

    blender -b --python art/champions/marrow/build.py -- --out art/champions/marrow/marrow.blend

A hooded bone-witch (14 §5): a bent crone in a deep plum hood that frames her face, its peak
drooping down her back and its cowl over her shoulders; a ribcage corset over her robe, a ragged
skirt of tattered strips, a shawl on an `extra_cloak` chain, a skull lantern swinging at her
hip; in her right hand a crooked staff of black wood crowned with a horned skull whose eyes burn.
Her signature is magenta (14 §4). (The first version raised a fan of bone spikes behind the hood
and held up bone-clawed hands: they read as whiskers and as nails.)

Weight (14 §3): she's in no hurry: she leans on the staff at rest, both hands on it; she stalks
with it like a walking stick; her spells come out of the skull, jabbed, swept or driven into the
ground and held; the drain pulls in hard. Feet planted by IK.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import champion, clips, head, rig  # noqa: E402
from mftr_blender.champion import arm, feet, hand  # noqa: E402
from mftr_blender.clips import at, grip, ik, merge, sym  # noqa: E402
from mftr_blender.library import _clip, gait  # noqa: E402
from mftr_blender.mesh import box_part, path_part  # noqa: E402

SKIN = (0.56, 0.56, 0.50)
PLUM = (0.24, 0.08, 0.20)
PLUM_DARK = (0.12, 0.04, 0.11)
GRAVE = (0.24, 0.24, 0.26)
BONE = (0.64, 0.60, 0.50)
BONE_DARK = (0.42, 0.38, 0.30)
IRON = (0.18, 0.18, 0.20)
HAIR = (0.62, 0.62, 0.60)
MAGENTA = (1.0, 0.24, 0.72)
MAGENTA_DEEP = (0.66, 0.06, 0.44)
ACCENT = (0.18, 0.52, 0.95)

SHAPE = {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 0.95, "y": 0.95, "scale": 0.96, "shoulder": 0.02, "arm": 1.15, "hand": 1.4}
rig.use_shape(SHAPE)
SHAWL = [rig.shaped(p) for p in [(0, 0.13, 1.52), (0, 0.18, 1.22), (0, 0.22, 0.92), (0, 0.24, 0.66)]]
BONES = {b[0]: b for b in rig.bone_table()}
HAND_LEN = (Vector(BONES["hand_r"][3]) - Vector(BONES["hand_r"][2])).length
GRIP = 0.5 * HAND_LEN
STAFF_TOP = GRIP + 0.95     # the skull, along hand_r from the wrist
STAFF_FOOT = GRIP - 0.78    # the staff's foot
WOOD = (0.16, 0.11, 0.10)


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    strips = [(-0.20, -0.10, 0.26), (-0.10, -0.15, 0.20), (0.0, -0.17, 0.30), (0.11, -0.15, 0.18), (0.21, -0.09, 0.28),
              (0.21, 0.08, 0.22), (0.10, 0.15, 0.30), (-0.02, 0.17, 0.24), (-0.13, 0.14, 0.32), (-0.22, 0.06, 0.20)]
    return [
        ("pelvis", sp((0, 0, 0.92)), sp((0, 0, 1.12)), 10, [(0, .14, .11), (.5, .155, .12), (1, .145, .11)], "cloth", PLUM_DARK),
        ("pelvis", sp((0, 0, 1.04)), sp((0, 0, 1.10)), 10, [(0, .158, .122), (1, .152, .116)], "accent", ACCENT),
        *[box_part("pelvis", sp((x, -0.14, 0.98)), (0.04, 0.03, 0.06), "cloth", BONE, rot=(0, 45, x * 100), bevel=0.3) for x in (-0.10, -0.03, 0.06)],
        # The skirt: a robe to the shins and tattered strips hanging below it.
        path_part("pelvis", [sp((0, 0, 1.06)), sp((0, 0.01, 0.78)), sp((0, 0.02, 0.42))], 12, [(.16, .13), (.22, .19), (.27, .23)], "cloth", PLUM),
        *[box_part("pelvis", sp((x, y, z)), (0.08, 0.02, 0.30), "cloth", PLUM_DARK if k % 2 else GRAVE, rot=(-math.degrees(math.atan2(y, 0.6)), 0, -math.degrees(math.atan2(x, 0.6))),
                   taper=0.4) for k, (x, y, z) in enumerate(strips)],
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 10, [(0, .12, .09), (1, .125, .092)], "cloth", PLUM),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 10, [(0, .126, .092), (1, .14, .10)], "cloth", PLUM),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.60)), 10, [(0, .14, .10), (.5, .145, .104), (1, .085, .072)], "cloth", PLUM),
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .045, .045), (1, .04, .04)], "skin", SKIN),
        # A ribcage corset: a bone sternum and five pairs of ribs curving round her front.
        path_part("chest", [sp((0, -0.115, 1.22)), sp((0, -0.125, 1.52))], 6, [(.018, .014), (.018, .014)], "cloth", BONE),
        # The shawl.
        ("extra_cloak_1", SHAWL[0], SHAWL[1], 8, [(0, .18, .022), (1, .21, .022)], "cloth", GRAVE),
        ("extra_cloak_2", SHAWL[1], SHAWL[2], 8, [(0, .21, .022), (1, .22, .021)], "cloth", GRAVE),
        ("extra_cloak_3", SHAWL[2], SHAWL[3], 8, [(0, .22, .021), (.7, .20, .02), (1, .10, .016)], "cloth", PLUM_DARK),
    ] + _ribs() + _lantern()


def _ribs():
    out = []
    for k, z in enumerate((1.26, 1.32, 1.38, 1.44, 1.50)):
        w = 0.10 + 0.012 * k
        for s in (1, -1):
            pts = [sp((0.01 * s, -0.125, z)), sp((0.06 * s, -0.12, z + 0.005)), sp((w * s, -0.08, z - 0.01)), sp(((w + 0.025) * s, -0.02, z - 0.03))]
            out.append(path_part("chest" if z > 1.4 else "spine_02", pts, 5, [(.011, .009)] * 3 + [(0, 0)], "cloth", BONE))
    return out


def _lantern():
    # A skull lantern hung at her left hip on an iron chain, magenta light in its eyes.
    c = Vector(sp((0.22, -0.06, 0.82)))
    return [
        path_part("pelvis", [sp((0.17, -0.05, 1.04)), sp((0.21, -0.06, 0.92))], 4, [(.008, .008), (.008, .008)], "metal", IRON),
        box_part("pelvis", tuple(c), (0.10, 0.11, 0.10), "cloth", BONE, bevel=0.35),
        box_part("pelvis", tuple(c + Vector((0, -0.01, -0.06))), (0.07, 0.08, 0.04), "cloth", BONE_DARK, bevel=0.3),
        box_part("pelvis", tuple(c + Vector((0.022, -0.056, 0.01))), (0.025, 0.01, 0.025), "emissive", MAGENTA),
        box_part("pelvis", tuple(c + Vector((-0.022, -0.056, 0.01))), (0.025, 0.01, 0.025), "emissive", MAGENTA),
    ]


def _hood():
    """The hood: a deep cowl wrapped round the back and sides of her head, set back so her face
    shows in its opening; a heavy rolled rim framing the face; the peak drooping down her back;
    a cowl over her shoulders."""
    hb = "head"
    out = [
        # The shell: centered behind the head, its front just behind her face.
        path_part(hb, [sp((0, 0.07, 1.58)), sp((0, 0.075, 1.66)), sp((0, 0.075, 1.76)), sp((0, 0.065, 1.85)), sp((0, 0.045, 1.92)), sp((0, 0.03, 1.955))], 14,
                  [(.15, .125), (.165, .135), (.17, .138), (.16, .13), (.12, .10), (0, 0)], "cloth", PLUM, ref=(1, 0, 0)),
        # The rim round the face opening, from cheek to cheek over the brow, drooping a little.
        path_part(hb, [sp((0.115, -0.07, 1.60)), sp((0.135, -0.095, 1.72)), sp((0.11, -0.12, 1.85)), sp((0.05, -0.14, 1.92)), sp((0.0, -0.145, 1.93)),
                       sp((-0.05, -0.14, 1.92)), sp((-0.11, -0.12, 1.85)), sp((-0.135, -0.095, 1.72)), sp((-0.115, -0.07, 1.60))], 6,
                  [(.03, .024)] * 9, "cloth", PLUM_DARK),
        # The peak, drooping from the crown down her back.
        path_part(hb, [sp((0, 0.10, 1.90)), sp((0, 0.19, 1.84)), sp((0, 0.24, 1.72)), sp((0, 0.25, 1.60))], 8, [(.07, .05), (.06, .045), (.04, .03), (0, 0)], "cloth", PLUM),
    ]
    # The cowl: the hood's skirt over her shoulders, ragged at the edge.
    out.append(path_part("chest", [sp((0.0, 0.01, 1.66)), sp((0.0, 0.015, 1.56)), sp((0.0, 0.02, 1.47))], 14, [(.13, .12), (.20, .155), (.25, .17)], "cloth", PLUM))
    for k in range(10):
        a = math.radians(-160 + 32 * k + (8 if k % 2 else 0))
        base = Vector(sp((0.24 * math.cos(a), 0.02 - 0.16 * math.sin(a), 1.48)))
        out.append(box_part("chest", tuple(base + Vector((0, 0, -0.04))), (0.07, 0.02, 0.10), "cloth", PLUM_DARK if k % 2 else PLUM,
                            rot=(0, 0, 90 - math.degrees(a)), taper=0.4))
    return out


def _staff():
    """The crooked staff of black wood, skinned to the right hand: a horned skull with burning
    eyes on top, bone charms hanging under it."""
    h, t = Vector(BONES["hand_r"][2]), Vector(BONES["hand_r"][3])
    axis = (t - h).normalized()
    side = axis.cross(Vector((0, 1, 0))).normalized()
    fwd = side.cross(axis).normalized()

    def p(d, off=Vector()):
        return tuple(h + axis * d + off)

    hb = "hand_r"
    shaft = [p(STAFF_FOOT), p(STAFF_FOOT + 0.35, side * 0.03), p(GRIP - 0.1, -side * 0.01), p(GRIP + 0.3, fwd * 0.03), p(GRIP + 0.55, -side * 0.035 + fwd * 0.01),
             p(STAFF_TOP - 0.16, side * 0.01)]
    c = h + axis * STAFF_TOP
    out = [path_part(hb, shaft, 6, [(.024, .024), (.028, .028), (.03, .03), (.028, .028), (.032, .032), (.04, .04)], "cloth", WOOD),
           path_part(hb, [p(STAFF_FOOT - 0.04), p(STAFF_FOOT + 0.02)], 6, [(0, 0), (.03, .03)], "metal", IRON),
           path_part(hb, [p(GRIP + 0.12), p(GRIP + 0.20)], 6, [(.034, .034), (.034, .034)], "cloth", PLUM_DARK),
           # The skull: cranium, brow, jaw, eye sockets burning magenta, horns curling out and back.
           box_part(hb, tuple(c), (0.13, 0.14, 0.12), "cloth", BONE, rot=(0, 0, 0), bevel=0.35),
           box_part(hb, tuple(c + fwd * -0.06 - axis * 0.05), (0.09, 0.06, 0.06), "cloth", BONE_DARK, bevel=0.3),
           box_part(hb, tuple(c - fwd * 0.07 + axis * 0.005 + side * 0.03), (0.032, 0.012, 0.03), "emissive", MAGENTA),
           box_part(hb, tuple(c - fwd * 0.07 + axis * 0.005 - side * 0.03), (0.032, 0.012, 0.03), "emissive", MAGENTA)]
    for sgn in (1, -1):
        root = c + side * 0.055 * sgn + axis * 0.04
        out.append(path_part(hb, [tuple(root), tuple(root + side * 0.08 * sgn + axis * 0.06), tuple(root + side * 0.12 * sgn + axis * 0.02 + fwd * 0.06),
                                  tuple(root + side * 0.10 * sgn - axis * 0.06 + fwd * 0.08)], 6, [(.026, .026), (.02, .02), (.012, .012), (0, 0)], "cloth", BONE_DARK))
    # Bone charms on cords under the skull.
    for k, sgn in enumerate((1, -1, 0)):
        top = c - axis * 0.10 + side * 0.04 * sgn + fwd * 0.03
        out.append(path_part(hb, [tuple(top), tuple(top - axis * (0.12 + 0.04 * k))], 4, [(.004, .004), (.004, .004)], "metal", IRON))
        out.append(box_part(hb, tuple(top - axis * (0.15 + 0.04 * k)), (0.025, 0.02, 0.05), "cloth", BONE, bevel=0.3))
    out.append(box_part(hb, tuple(c - axis * 0.33 + fwd * 0.04), (0.02, 0.012, 0.02), "emissive", MAGENTA_DEEP, rot=(0, 45, 0)))
    return out


def _head(bm, layers, groups, mats):
    head.build(bm, layers, groups, mats, head.Head(
        base=sp((0, -0.01, 1.62)), height=0.25, scale=1.04, jaw=0.55, chin=1.1, eye=1.0,
        skin=SKIN, hair=HAIR, hair_style="short", eye_color=(0.9, 0.2, 0.6), lips=(0.36, 0.22, 0.26)))


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 10, [(0, .05, .05), (1, .044, .044)], "cloth", PLUM),
        # Ragged sleeves to the forearm, bone bangles, bare thin wrists.
        (fa, (fa, 0), (fa, .55), 10, [(0, .046, .046), (.5, .07, .066), (1, .09, .08)], "cloth", PLUM_DARK),
        (fa, (fa, .5), (fa, 1), 8, [(0, .036, .036), (1, .03, .03)], "skin", SKIN),
        *[(fa, (fa, t), (fa, t + 0.04), 8, [(0, .042, .042), (1, .042, .042)], "cloth", BONE) for t in (0.66, 0.78)],
        *champion.hand_parts("l", SKIN, "skin", scale=1.2),
        (th, sp((0.10, 0, 1.02)), (th, 1), 10, [(0, .08, .078), (1, .055, .055)], "cloth", PLUM_DARK),
        (ca, (ca, 0), (ca, 1), 8, [(0, .05, .05), (1, .042, .042)], "skin", SKIN),
        # Rag-wrapped feet.
        box_part("foot_l", ("foot_l", 0.45), (0.08, 0.16, 0.07), "cloth", GRAVE, bevel=0.3),
        (ca, (ca, .82), (ca, 1.0), 8, [(0, .052, .052), (1, .056, .056)], "cloth", GRAVE),
    ]


def parts():
    return _center() + champion.mirrored(_left()) + _hood() + _staff() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def shawl(back=0.0, sway=0.0):
    return {"extra_cloak_1": (back * 0.45, sway * 0.5, 0), "extra_cloak_2": (back * 0.3, sway * 0.3, 0),
            "extra_cloak_3": (back * 0.25, sway * 0.25, 0)}


def staff(top, foot=None, left=None, pole=(1.0, -0.3, -0.8)):
    """The skull at `top`, the foot toward `foot` (default: straight below), the right hand on
    the staff; `left` puts the left hand on it too, that far toward the skull."""
    tp = Vector(top)
    ft = Vector(foot) if foot else tp - Vector((0, 0, 1))
    axis = (tp - ft).normalized()
    out = {"ik_hand_r": ik(tuple(tp - axis * STAFF_TOP), aim=tuple(axis), pole=pole)}
    if left is not None:
        out["grip_l"] = grip("hand_r", at=GRIP + left, pole=(1.0, -0.2, -0.6))
    return out


OPEN_L = {"fingers_l": (-12, 0, 0), "thumb_l": (-10, 0, 0)}
BENT = {"spine_01": (14, 0, 0), "spine_02": (10, 0, 0), "chest": (6, 0, 0), "head": (-22, 0, 0)}
STAND = feet((0.13, 0.08), (0.13, -0.08))
# Leaning on the planted staff, both hands stacked on it under the skull.
LEAN = staff(at(0.18, 0.40, 1.62, "r"), at(0.20, 0.46, 0.0, "r"), left=0.11)
IDLE = merge(STAND, BENT, LEAN, shawl(3))
READY = merge(feet((0.18, 0.16), (0.18, -0.14)), BENT, {"spine_01": (18, 0, 0)}, staff(at(0.18, 0.72, 1.55, "r"), at(0.30, 0.0, 0.42, "r")),
              arm("l", 0.5, 0.55, -0.35, 0.3, 0.85, 0.05), OPEN_L, shawl(5))
DOWN = {"pelvis": (0, 0, -0.08)}


def idle(armature):
    # Leaning on the staff: a slow wheezing breath, the head lifting to peer and settling, the
    # top hand's fingers drumming once on the bone.
    peer = merge(IDLE, sym("clavicle", 0, -4, 0), {"head": (-30, 0, 10), "spine_02": (8, 0, 0)}, shawl(4, 2))
    keys = [(0, IDLE, {"pelvis": (0, 0, -0.05)}), (30, peer, {"pelvis": (0, 0, -0.04)}), (44, peer, {"pelvis": (0, 0, -0.04)})]
    for i, f in enumerate((50, 53, 56, 59)):
        keys.append((f, merge(peer, {"fingers_l": (15 if i % 2 else 55, 0, 0)}), {"pelvis": (0, 0, -0.04)}, "snap"))
    keys += [(66, IDLE, {"pelvis": (0, 0, -0.05)}), (96, IDLE, {"pelvis": (0, 0, -0.05)})]
    return _clip(armature, "idle", 96, keys, loop=True)


def idle_fidget_1(armature):
    # Lets go with her left hand and taps the skull lantern at her hip, then the staff again.
    tap = merge(STAND, BENT, staff(at(0.18, 0.40, 1.62, "r"), at(0.20, 0.46, 0.0, "r")), hand("l", at(0.22, 0.02, 0.98, "l"), aim=at(0.1, 0.3, -1, "l")),
                {"head": (6, 0, 14)}, shawl(3))
    keys = [(0, IDLE, None), (10, tap, None)]
    for i, f in enumerate(range(14, 34, 3)):
        keys.append((f, merge(tap, {"fingers_l": (20 if i % 2 else 55, 0, 0)}), None, "snap"))
    keys.append((44, IDLE, None))
    return _clip(armature, "idle_fidget_1", 44, keys)


def idle_fidget_2(armature):
    # Cranes her neck out of the hood, sniffing the air, a slow head turn.
    keys = [(0, IDLE, None), (14, merge(IDLE, {"head": (-36, 0, 30), "neck": (-10, 0, 10)}), None), (30, merge(IDLE, {"head": (-36, 0, -30), "neck": (-10, 0, -10)}), None),
            (46, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 46, keys)


def idle_ready(armature):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (21, 0, 0)}, shawl(7, 2)), {"pelvis": (0, 0, -0.10)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(trail, swing):
    # The staff carried like a walking stick, swung forward with the left foot; the left arm
    # swings against it, the hand open.
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(staff(at(0.30, 0.30 + 0.25 * swing * c, 1.45, "r"), at(0.30, 0.55 + 0.30 * swing * c, 0.20, "r")),
                     arm("l", 0.35, 0.3 - 0.35 * swing * c, -0.8, 0.2, 0.6 - 0.3 * swing * c, -0.4), OPEN_L,
                     shawl(trail + 5 * math.sin(4 * math.pi * p), 4 * math.sin(2 * math.pi * p)), {"head": (-26, 0, 0)})
    return upper


def walk(armature):
    # A walk (heavy slows, and the moment a run comes to a stop): the run's body type, slower,
    # the weapon carried as in it, never hanging into the ground (14 §3.6).
    keys = gait(32, 0.90, contact=0.6, lift=0.10, bob=0.02, lean=16, twist=6, width=0.13, crouch=0.06, upper=_run_upper(12, 0.4))
    return _clip(armature, "walk", 32, keys, loop=True, markers={"foot_l": 0, "foot_r": 16}, stride_speed=140.0)


def run(armature):
    # A hunched, stalking stride (14 §3.6).
    keys = gait(20, 0.96, contact=0.45, lift=0.16, bob=0.04, lean=22, twist=8, width=0.13, crouch=0.07, upper=_run_upper(30, 1.0))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    keys = gait(16, 0.96, contact=0.40, lift=0.20, bob=0.045, lean=28, twist=10, width=0.13, crouch=0.08, upper=_run_upper(40, 1.2))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(armature):
    # The skull jabbed at the target: drawn back to her shoulder, cocked, a one-frame jab on
    # `fire` (sim: 10) that spits the bolt from its jaws, held out.
    st = feet((0.17, 0.22), (0.17, -0.18))
    back = merge(st, BENT, staff(at(0.32, -0.10, 1.75, "r"), at(0.30, 0.45, 0.55, "r")), arm("l", 0.5, 0.5, -0.4, 0.3, 0.8, 0.0), OPEN_L,
                 {"spine_01": (10, 0, -18), "head": (-20, 0, 14)}, shawl(4))
    cock = merge(back, staff(at(0.34, -0.22, 1.72, "r"), at(0.30, 0.40, 0.60, "r")), {"spine_01": (10, 0, -24)})
    jab = merge(feet((0.17, 0.32), (0.17, -0.18)), BENT, staff(at(0.10, 1.15, 1.45, "r"), at(0.28, 0.05, 1.05, "r"), pole=(1.0, -0.4, -0.6)),
                arm("l", 0.6, -0.3, -0.5, 0.4, -0.1, -0.4), {"spine_01": (22, 0, 14), "head": (-22, 0, -10)}, shawl(9))
    keys = [(0, READY, DOWN), (6, back, DOWN), (9, cock, DOWN, "snap"), (10, jab, {"pelvis": (0, -0.04, -0.10)}),
            (16, merge(jab, {"spine_01": (24, 0, 16)}), {"pelvis": (0, -0.04, -0.11)}, "settle"), (26, READY, DOWN)]
    return _clip(armature, "attack_1", 26, keys, markers={"fire": 10})


def attack_2(armature):
    # The staff swept round flat from her right and stopped dead pointing at the target on
    # `fire`, the left palm thrown after it.
    st = feet((0.17, 0.22), (0.17, -0.18))
    wind = merge(st, BENT, staff(at(0.75, 0.20, 1.35, "r"), at(-0.10, 0.25, 1.05, "r")), arm("l", 0.6, 0.2, -0.3, 0.4, 0.4, -0.2), OPEN_L,
                 {"spine_01": (12, 0, -28), "head": (-20, 0, 22)}, shawl(4, -4))
    point = merge(feet((0.17, 0.30), (0.17, -0.18)), BENT, staff(at(-0.05, 1.10, 1.40, "r"), at(0.35, 0.10, 1.10, "r"), pole=(1.0, -0.4, -0.6)),
                  arm("l", 0.15, 0.9, 0.0, 0.0, 1, 0.05), OPEN_L, {"spine_01": (20, 0, 18), "head": (-22, 0, -12)}, shawl(9, 6))
    keys = [(0, READY, DOWN), (6, wind, DOWN), (8, merge(wind, {"spine_01": (12, 0, -32)}), DOWN, "whip"), (10, point, {"pelvis": (0, -0.04, -0.10)}),
            (16, merge(point, {"spine_01": (22, 0, 20)}), {"pelvis": (0, -0.04, -0.11)}, "settle"), (26, READY, DOWN)]
    return _clip(armature, "attack_2", 26, keys, markers={"fire": 10})


def q(armature):
    # Siphon: the staff flung up and her left arm thrown wide on `fire` 6, the hand open; then
    # she drags the life in, the hand clenched hard to her chest and the staff hugged close, and
    # holds.
    wide = merge(feet((0.20, 0.14), (0.20, -0.12)), {"spine_01": (2, 0, 0), "spine_02": (-8, 0, 0), "head": (-30, 0, 0)},
                 staff(at(0.55, 0.25, 2.25, "r"), at(0.40, 0.25, 1.0, "r")), arm("l", 1, 0.3, 0.2, 1, 0.3, 0.3), {"fingers_l": (-20, 0, 0)}, shawl(10))
    pull = merge(feet((0.20, 0.10), (0.20, -0.16)), BENT, {"spine_01": (24, 0, 0)}, staff(at(0.12, 0.30, 1.70, "r"), at(0.18, 0.30, 0.30, "r")),
                 hand("l", at(0.08, 0.20, 1.40, "l"), aim=at(-0.6, 0.4, 0.4, "l")), {"fingers_l": (85, 0, 0), "thumb_l": (50, 0, 0)}, shawl(2))
    keys = [(0, READY, DOWN), (4, merge(READY, {"fingers_l": (60, 0, 0)}), {"pelvis": (0, 0, -0.12)}, "snap"), (6, wide, {"pelvis": (0, 0, -0.02)}),
            (9, merge(wide, {"head": (-34, 0, 0)}), {"pelvis": (0, 0, -0.02)}, "whip"), (12, pull, {"pelvis": (0, 0, -0.12)}),
            (17, pull, {"pelvis": (0, 0, -0.13)}, "settle"), (22, READY, DOWN)]
    return _clip(armature, "q", 22, keys, markers={"fire": 6})


def w(armature):
    # Grasping Bones: a crouch, the staff raised in both hands, then driven foot-first into the
    # ground ahead on `fire` 9 and held there while the bones erupt down the line.
    crouch = merge(feet((0.20, 0.22), (0.20, -0.16)), BENT, {"spine_01": (20, 0, 0)}, staff(at(0.15, 0.10, 2.35, "r"), at(0.15, 0.45, 1.10, "r"), left=-0.20), shawl(4))
    plunge = merge(feet((0.20, 0.28), (0.20, -0.16)), {"spine_01": (36, 0, 0), "spine_02": (12, 0, 0), "head": (-30, 0, 0)},
                   staff(at(0.12, 0.45, 1.30, "r"), at(0.12, 0.85, 0.0, "r"), left=-0.18, pole=(1.0, -0.2, -0.6)), shawl(10))
    keys = [(0, READY, DOWN), (6, crouch, {"pelvis": (0, 0, -0.14)}), (8, merge(crouch, {"spine_02": (-6, 0, 0)}), {"pelvis": (0, 0, -0.12)}, "snap"),
            (9, plunge, {"pelvis": (0, -0.04, -0.32)}), (18, merge(plunge, {"spine_01": (38, 0, 0)}), {"pelvis": (0, -0.04, -0.33)}, "settle"), (28, READY, DOWN)]
    return _clip(armature, "w", 28, keys, markers={"fire": 9})


def e(armature):
    # Grave Pact (upper body, a pulse): the staff thrust high, the skull blazing, her left hand
    # clawing at her ribs and her head thrown back on `fire` 4; held, a ragged breath.
    pact = merge({"spine_01": (4, 0, 0), "spine_02": (-12, 0, 0), "head": (-44, 0, 0)}, staff(at(0.30, 0.25, 2.55, "r")),
                 hand("l", at(0.06, 0.14, 1.40, "l"), aim=at(-0.6, 0.5, 0.2, "l")), {"fingers_l": (70, 0, 0)}, shawl(8))
    keys = [(0, IDLE, None), (3, merge(pact, {"head": (-20, 0, 0)}, staff(at(0.30, 0.25, 2.20, "r"))), None, "snap"), (4, pact, None),
            (13, merge(pact, {"head": (-50, 0, 0)}), None, "settle"), (24, IDLE, None)]
    return _clip(armature, "e", 24, keys, layer="upper", markers={"fire": 4})


def r(armature):
    # Ossuary: the staff lifted high in both hands, cocked, then slammed foot-first into the
    # ground on `fire` 8; she hangs on it, low, while the boneyard rises; a cackle.
    st = feet((0.20, 0.18), (0.20, -0.16))
    high = merge(st, {"spine_01": (0, 0, 0), "spine_02": (-14, 0, 0), "head": (-34, 0, 0)}, staff(at(0.15, 0.20, 2.75, "r"), left=0.30), shawl(4))
    slam = merge(feet((0.22, 0.24), (0.22, -0.18)), {"spine_01": (30, 0, 0), "spine_02": (12, 0, 0), "head": (-24, 0, 0)},
                 staff(at(0.12, 0.42, 1.45, "r"), at(0.12, 0.55, 0.0, "r"), left=0.20, pole=(1.0, -0.2, -0.6)), shawl(10))
    keys = [(0, READY, DOWN), (5, high, {"pelvis": (0, 0, 0.03)}), (7, merge(high, {"spine_02": (-18, 0, 0)}), {"pelvis": (0, 0, 0.04)}, "snap"),
            (8, slam, {"pelvis": (0, -0.03, -0.30)}), (16, merge(slam, {"head": (-36, 0, 0)}), {"pelvis": (0, -0.03, -0.30)}),
            (20, merge(slam, {"head": (-28, 0, 0)}), {"pelvis": (0, -0.03, -0.29)}, "settle"), (30, READY, DOWN)]
    return _clip(armature, "r", 30, keys, markers={"fire": 8})


KNEEL = merge({"ik_foot_l": ik(at(0.14, 0.26, champion.ankle(), "l")), "thigh_r": (8, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
              BENT, {"spine_01": (24, 0, 0)}, staff(at(0.20, 0.45, 1.20, "r"), at(0.15, 0.65, 0.0, "r")), hand("l", at(0.25, 0.45, 0.15, "l"), aim=at(0, 0.5, -1, "l")),
              shawl(-4))


def recall(armature):
    # Crouches and scratches a circle into the ground with the staff's foot.
    def at_foot(x, y):
        return merge(KNEEL, staff(at(0.15, 0.30, 1.20, "r"), at(x, y, 0.0, "r")))
    keys = [(0, IDLE, None), (15, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (30, at_foot(0.45, 0.60), {"pelvis": (0, 0.04, -0.45)}),
            (45, at_foot(0.05, 0.75), {"pelvis": (0, 0.04, -0.45)}), (60, at_foot(-0.20, 0.55), {"pelvis": (0, 0.04, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    knees = merge({"thigh_l": (-6, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (110, 0, 0), "calf_r": (110, 0, 0)}, BENT, staff(at(0.20, 0.40, 1.10, "r"), at(0.20, 0.50, 0.0, "r")),
                  arm("l", 0.35, 0.45, -0.75, 0.15, 0.85, -0.3), {"spine_01": (30, 0, 0)}, shawl(-6))
    down = merge({"thigh_l": (-30, 0, 0), "thigh_r": (-14, 0, 0), "calf_l": (70, 0, 0), "calf_r": (40, 0, 0)}, arm("l", 0.8, 0.5, 0.0, 0.7, 0.6, 0.0),
                 arm("r", 0.6, 0.7, 0.1, 0.5, 0.8, 0.0), {"pelvis": (-70, -18, 0), "head": (20, 0, 16)}, shawl(-20))
    keys = [(0, IDLE, None), (8, merge(IDLE, {"spine_01": (-6, 0, 0), "head": (-40, 0, 0)}), {"pelvis": (0, 0.05, 0)}, "settle"),
            (16, knees, {"pelvis": (0, 0, -0.5)}, "snap"), (26, down, {"pelvis": (0, 0.44, -0.84)}, "snap"), (34, down, {"pelvis": (0, 0.45, -0.85)})]
    return _clip(armature, "death", 34, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "settle"), (14, merge(IDLE, {"head": (-30, 0, 0)}), {"pelvis": (0, 0, -0.06)}), (26, IDLE, None)]
    return _clip(armature, "respawn", 26, keys)


def select(armature):
    # Raises the staff, the skull blazing, her other hand spread beside her face, and cackles,
    # head thrown back.
    frame = merge(STAND, BENT, staff(at(0.30, 0.30, 2.40, "r")), hand("l", at(0.20, 0.18, 1.72, "l"), aim=at(0.3, 0.3, 0.9, "l")), {"fingers_l": (-20, 0, 0)}, shawl(4))
    keys = [(0, IDLE, None), (12, frame, None), (20, merge(frame, {"head": (-44, 0, 0)}), None), (24, merge(frame, {"head": (-36, 0, 0)}), None),
            (28, merge(frame, {"head": (-44, 0, 0)}), None), (40, IDLE, None)]
    return _clip(armature, "select", 40, keys)


def emote_taunt(armature):
    # Crooks one finger, beckoning, slowly.
    one = merge(STAND, BENT, staff(at(0.18, 0.40, 1.62, "r"), at(0.20, 0.46, 0.0, "r")), shawl(3))
    out = merge(one, arm("l", 0.2, 1, 0.0, 0.1, 1, 0.1), {"fingers_l": (0, 0, 0)})
    curl = merge(out, {"fingers_l": (90, 0, 0)})
    keys = [(0, IDLE, None), (10, out, None), (18, curl, None), (26, out, None), (34, curl, None), (46, IDLE, None)]
    return _clip(armature, "emote_taunt", 46, keys)


def emote_joke(armature):
    # Holds the staff's skull up to her face and makes it "talk", waggling it.
    talk = [merge(STAND, BENT, staff(at(0.10, 0.42, 1.78 + (0.03 if i % 2 else 0.0), "r"), at(0.35, 0.15, 0.70, "r")), arm("l", 0.5, 0.3, -0.6, 0.3, 0.5, -0.4),
                  {"head": (-14, 0, -20)}, shawl(3)) for i in range(2)]
    keys = [(0, IDLE, None), (12, talk[0], None)]
    for i, f in enumerate(range(16, 36, 4)):
        keys.append((f, talk[i % 2], None, "snap"))
    keys.append((48, IDLE, None))
    return _clip(armature, "emote_joke", 48, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, None)]
    for i in range(1, 9):
        s = 1.0 if i % 2 else 0.4
        keys.append((4 * i, merge(IDLE, {"spine_01": (-4 * s, 0, 0), "head": (-40 * s, 0, 0)}, sym("clavicle", 0, -6 * s, 0)), {"pelvis": (0, 0, -0.04)}))
    keys.append((44, IDLE, None))
    return _clip(armature, "emote_laugh", 44, keys)


def emote_dance(armature):
    # A crooked shuffle round the staff, her free hand waving overhead.
    a = merge(BENT, staff(at(0.25, 0.40, 1.70, "r"), at(0.25, 0.45, 0.0, "r")), arm("l", 0.4, 0.2, 0.85, 0.3, 0.1, 0.95), OPEN_L,
              feet((0.15, 0.08), (0.12, -0.08), lift_l=0.12), {"pelvis": (0, -8, 10)}, shawl(4, 8))
    b = merge(BENT, staff(at(0.25, 0.40, 1.70, "r"), at(0.25, 0.45, 0.0, "r")), arm("l", 0.6, 0.2, 0.75, 0.6, 0.1, 0.85), OPEN_L,
              feet((0.12, 0.08), (0.15, -0.08), lift_r=0.12), {"pelvis": (0, 8, -10)}, shawl(4, -8))
    mid = merge(BENT, STAND, staff(at(0.25, 0.40, 1.66, "r"), at(0.25, 0.45, 0.0, "r")), arm("l", 0.5, 0.2, 0.8, 0.45, 0.1, 0.9), OPEN_L)
    keys = [(0, a, {"pelvis": (0.03, 0, -0.04)}), (12, mid, {"pelvis": (0, 0, -0.07)}), (24, b, {"pelvis": (-0.03, 0, -0.04)}),
            (36, mid, {"pelvis": (0, 0, -0.07)}), (48, a, {"pelvis": (0.03, 0, -0.04)})]
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, walk, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    rig.add_chain(armature, "cloak", "chest", SHAWL)
    # Her bolts and the bones leave from the skull's jaws.
    h, t = Vector(BONES["hand_r"][2]), Vector(BONES["hand_r"][3])
    champion.place_socket(armature, "socket_projectile", tuple(h + (t - h).normalized() * STAFF_TOP))


def build(only=None):
    return champion.build("marrow", SHAPE, parts, OWN, 1.85, extras=_extras, only=only, shared=champion.SHARED + ["attack_melee_alt"], cloth={"cloak": dict(stiffness=0.3, inertia=0.84)})


if __name__ == "__main__":
    champion.main(build)
