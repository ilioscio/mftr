# SPDX-License-Identifier: AGPL-3.0-or-later
"""Quill (rebuilt for 14): the generator for her model and clips.

    blender -b --python art/champions/quill/build.py -- --out art/champions/quill/quill.blend

A tall pointed hat and a long coat (14 §5): a gangly stargazer-scholar under a wide-brimmed,
pointed astronomer's hat with brass goggles on its band, a long navy coat flecked with stars at
the hem whose tails trail on an `extra_coat` chain, and a tall staff topped by an orrery, brass
rings and little planets around a glowing star. Her signature is starlight indigo (14 §4).

Weight (14 §3): the staff is held by IK; her big casts take it in both hands. Starfall Lance is a
long, planted charge, the staff trembling as it builds, then one frame of release and a recoil
that holds through the hard lock. Feet planted by IK.
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

SKIN = (0.80, 0.60, 0.48)
NAVY = (0.08, 0.10, 0.22)
NAVY_DARK = (0.04, 0.05, 0.12)
INDIGO_CLOTH = (0.18, 0.16, 0.42)
PARCHMENT = (0.66, 0.60, 0.46)
LEATHER = (0.28, 0.18, 0.10)
BRASS = (0.68, 0.50, 0.22)
WOOD = (0.26, 0.17, 0.10)
HAIR = (0.12, 0.09, 0.08)
STAR = (0.62, 0.66, 1.0)
STAR_WHITE = (0.92, 0.94, 1.0)
ACCENT = (0.18, 0.52, 0.95)

# Gangly (14 §2.2): long legs and arms, narrow, big hands.
SHAPE = {"heights": [(0.0, 0.0), (0.54, 0.57), (1.00, 1.04), (2.15, 2.17)], "x": 0.97, "y": 0.96, "scale": 0.98, "shoulder": 0.02, "arm": 1.08, "hand": 1.3}
rig.use_shape(SHAPE)
BONES = {b[0]: b for b in rig.bone_table()}
COAT = [rig.shaped(p) for p in [(0, 0.12, 1.04), (0, 0.16, 0.78), (0, 0.19, 0.50), (0, 0.21, 0.24)]]
HAND_LEN = (Vector(BONES["hand_r"][3]) - Vector(BONES["hand_r"][2])).length
GRIP = 0.5 * HAND_LEN
STAFF_TOP = GRIP + 1.05     # the orrery's star, along hand_r from the wrist
STAFF_FOOT = GRIP - 0.70


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    stars = [((0.20, -0.14, 0.42), 0.022), ((-0.16, -0.17, 0.50), 0.018), ((0.05, -0.20, 0.36), 0.02), ((-0.22, 0.02, 0.40), 0.02),
             ((0.21, 0.10, 0.46), 0.018), ((-0.05, 0.20, 0.38), 0.022), ((0.12, 0.19, 0.52), 0.016)]
    return [
        ("pelvis", sp((0, 0, 0.92)), sp((0, 0, 1.12)), 10, [(0, .13, .10), (.5, .15, .11), (1, .14, .105)], "cloth", NAVY_DARK),
        ("pelvis", sp((0, 0, 1.04)), sp((0, 0, 1.10)), 10, [(0, .152, .112), (1, .148, .108)], "accent", ACCENT),
        box_part("pelvis", sp((0, -0.115, 1.07)), (0.06, 0.025, 0.05), "metal", BRASS, bevel=0.3),
        # The coat: a long skirt to the shins, flaring, its hem flecked with stars.
        path_part("pelvis", [sp((0, 0.0, 1.06)), sp((0, 0.01, 0.80)), sp((0, 0.02, 0.50)), sp((0, 0.02, 0.36))], 12,
                  [(.16, .125), (.21, .17), (.26, .22), (.28, .24)], "cloth", NAVY),
        path_part("pelvis", [sp((0, 0.02, 0.385)), sp((0, 0.02, 0.35))], 12, [(.275, .235), (.285, .245)], "metal", BRASS),
        *[box_part("pelvis", sp(p), (r, r, r), "emissive", STAR, rot=(0, 45, 45)) for p, r in stars],
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 10, [(0, .118, .088), (1, .124, .092)], "cloth", NAVY),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 10, [(0, .126, .092), (1, .146, .102)], "cloth", NAVY),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.60)), 10, [(0, .148, .104), (.5, .154, .108), (1, .088, .074)], "cloth", NAVY),
        # The parchment shirt at the open collar, indigo lapels, two rows of brass buttons.
        box_part("chest", sp((0, -0.10, 1.50)), (0.08, 0.03, 0.16), "cloth", PARCHMENT),
        box_part("chest", sp((0.07, -0.105, 1.45)), (0.05, 0.025, 0.26), "cloth", INDIGO_CLOTH, rot=(0, -14, 0)),
        box_part("chest", sp((-0.07, -0.105, 1.45)), (0.05, 0.025, 0.26), "cloth", INDIGO_CLOTH, rot=(0, 14, 0)),
        *[box_part("spine_02", sp((x, -0.12, z)), (0.022, 0.015, 0.022), "metal", BRASS) for x in (0.05, -0.05) for z in (1.28, 1.36)],
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .048, .048), (1, .044, .044)], "skin", SKIN),
        # A scroll case and a brass spyglass at her hips.
        path_part("pelvis", [sp((0.17, 0.04, 0.82)), sp((0.19, 0.08, 1.12))], 8, [(.035, .035), (.035, .035)], "cloth", LEATHER),
        path_part("pelvis", [sp((-0.17, -0.06, 0.92)), sp((-0.18, -0.02, 1.10))], 8, [(.022, .022), (.03, .03)], "metal", BRASS),
        # Coat tails on the chain.
        ("extra_coat_1", COAT[0], COAT[1], 6, [(0, .15, .024), (1, .17, .024)], "cloth", NAVY),
        ("extra_coat_2", COAT[1], COAT[2], 6, [(0, .17, .024), (1, .18, .024)], "cloth", NAVY),
        ("extra_coat_3", COAT[2], COAT[3], 6, [(0, .18, .024), (.8, .17, .022), (1, .12, .018)], "cloth", INDIGO_CLOTH),
        # Shoulder capes, brass-edged.
        path_part("clavicle_l", [("clavicle_l", 0.5, (0, 0, 0.05)), ("clavicle_l", 1.0, (0.03, 0, 0.08)), ("clavicle_l", 1.0, (0.12, 0, 0.02)),
                                 ("clavicle_l", 1.0, (0.15, 0, -0.07))], 8, [(.12, .04), (.14, .055), (.14, .055), (.12, .03)], "cloth", INDIGO_CLOTH, ref=(0, 1, 0)),
        path_part("clavicle_r", [("clavicle_r", 0.5, (0, 0, 0.05)), ("clavicle_r", 1.0, (-0.03, 0, 0.08)), ("clavicle_r", 1.0, (-0.12, 0, 0.02)),
                                 ("clavicle_r", 1.0, (-0.15, 0, -0.07))], 8, [(.12, .04), (.14, .055), (.14, .055), (.12, .03)], "cloth", INDIGO_CLOTH, ref=(0, 1, 0)),
    ]


def _hat():
    """The astronomer's hat: a wide brim, a tall cone bending back to a point, a brass band with
    goggles, a star at the tip."""
    hb = "head"
    tip = sp((0, 0.22, 2.36))
    return [
        path_part(hb, [sp((0, -0.01, 1.81)), sp((0, -0.008, 1.825))], 16, [(.27, .27), (.26, .26)], "cloth", NAVY_DARK),
        path_part(hb, [sp((0, -0.01, 1.80)), sp((0, -0.01, 1.82))], 16, [(.28, .28), (.275, .275)], "metal", BRASS),
        path_part(hb, [sp((0, -0.005, 1.82)), sp((0, 0.0, 1.92)), sp((0, 0.03, 2.04)), sp((0, 0.09, 2.18)), sp((0, 0.16, 2.29)), tip], 14,
                  [(.125, .13), (.115, .12), (.095, .10), (.068, .072), (.04, .042), (0, 0)], "cloth", NAVY),
        path_part(hb, [sp((0, -0.005, 1.835)), sp((0, -0.004, 1.87))], 14, [(.126, .131), (.122, .127)], "cloth", INDIGO_CLOTH),
        box_part(hb, tip, (0.06, 0.06, 0.06), "emissive", STAR_WHITE, rot=(45, 45, 0)),
        # Brass goggles on the band, lenses glowing.
        path_part(hb, [sp((0.045, -0.12, 1.86)), sp((0.045, -0.145, 1.86))], 8, [(.03, .03), (.03, .03)], "metal", BRASS, ref=(1, 0, 0)),
        path_part(hb, [sp((-0.045, -0.12, 1.86)), sp((-0.045, -0.145, 1.86))], 8, [(.03, .03), (.03, .03)], "metal", BRASS, ref=(1, 0, 0)),
        box_part(hb, sp((0.045, -0.148, 1.86)), (0.035, 0.008, 0.035), "emissive", STAR, rot=(0, 0, 45)),
        box_part(hb, sp((-0.045, -0.148, 1.86)), (0.035, 0.008, 0.035), "emissive", STAR, rot=(0, 0, 45)),
    ]


def _head(bm, layers, groups, mats):
    head.build(bm, layers, groups, mats, head.Head(
        base=sp((0, -0.008, 1.62)), height=0.25, scale=1.06, jaw=0.46, chin=0.86, eye=1.2,
        skin=SKIN, hair=HAIR, hair_style="short", eye_color=(0.36, 0.38, 0.8), lips=(0.66, 0.38, 0.38)))


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 10, [(0, .056, .056), (1, .048, .048)], "cloth", NAVY),
        (fa, (fa, 0), (fa, 1), 10, [(0, .05, .05), (.6, .056, .056), (1, .066, .062)], "cloth", NAVY),
        (fa, (fa, .82), (fa, .97), 10, [(0, .068, .065), (1, .07, .066)], "cloth", INDIGO_CLOTH),
        (fa, (fa, .95), (fa, 1.0), 10, [(0, .071, .067), (1, .071, .067)], "metal", BRASS),
        *champion.hand_parts("l", SKIN, "skin", scale=1.25),
        (th, sp((0.10, 0, 1.02)), (th, 1), 10, [(0, .085, .082), (.5, .07, .07), (1, .056, .056)], "cloth", NAVY_DARK),
        (ca, (ca, 0), (ca, .4), 10, [(0, .056, .056), (1, .05, .05)], "cloth", NAVY_DARK),
        # Tall boots with brass buckles.
        (ca, (ca, .3), (ca, 1), 10, [(0, .07, .07), (.12, .066, .066), (1, .056, .058)], "cloth", LEATHER),
        (ca, (ca, .27), (ca, .33), 10, [(0, .078, .078), (1, .078, .078)], "cloth", LEATHER),
        box_part(ca, (ca, 0.55, (0.0, -0.065, 0)), (0.045, 0.02, 0.035), "metal", BRASS),
        box_part(ca, (ca, 0.75, (0.0, -0.062, 0)), (0.045, 0.02, 0.035), "metal", BRASS),
        box_part("foot_l", ("foot_l", 0.45), (0.09, 0.16, 0.08), "cloth", LEATHER, bevel=0.25),
        box_part("toe_l", ("toe_l", 0.35), (0.085, 0.08, 0.06), "cloth", LEATHER, bevel=0.25),
    ]


def _staff():
    """The orrery staff, skinned to the right hand and laid along it."""
    h, t = Vector(BONES["hand_r"][2]), Vector(BONES["hand_r"][3])
    axis = (t - h).normalized()
    side = axis.cross(Vector((0, 1, 0))).normalized()
    fwd = side.cross(axis).normalized()

    def p(d, off=Vector()):
        return tuple(h + axis * d + off)

    hb = "hand_r"
    c = h + axis * STAFF_TOP
    out = [
        path_part(hb, [p(STAFF_FOOT), p(STAFF_TOP - 0.20)], 6, [(.024, .024), (.028, .028)], "cloth", WOOD),
        path_part(hb, [p(STAFF_FOOT - 0.04), p(STAFF_FOOT + 0.03)], 6, [(0, 0), (.032, .032)], "metal", BRASS),
        path_part(hb, [p(GRIP - 0.07), p(GRIP + 0.10)], 6, [(.03, .03), (.03, .03)], "cloth", LEATHER),
        path_part(hb, [p(STAFF_TOP - 0.22), p(STAFF_TOP - 0.16)], 8, [(.04, .04), (.05, .05)], "metal", BRASS),
        # The star.
        path_part(hb, [p(STAFF_TOP - 0.07), p(STAFF_TOP), p(STAFF_TOP + 0.07)], 8, [(.05, .05), (.075, .075), (.05, .05)], "emissive", STAR_WHITE),
    ]
    # Three crossing brass rings around it, and two little planets on them.
    for k, (u, v) in enumerate(((side, fwd), (axis, side), ((fwd + axis).normalized(), (side - axis * 0.3).normalized()))):
        r = 0.17 - 0.015 * k
        pts = [tuple(c + u * math.cos(a) * r + v * math.sin(a) * r) for a in [2 * math.pi * i / 14 for i in range(15)]]
        out.append(path_part(hb, pts, 4, [(.011, .011)] * 15, "metal", BRASS))
    out.append(box_part(hb, tuple(c + side * 0.17), (0.05, 0.05, 0.05), "emissive", STAR, rot=(45, 45, 0)))
    out.append(box_part(hb, tuple(c + axis * 0.155), (0.04, 0.04, 0.04), "cloth", INDIGO_CLOTH, rot=(45, 0, 45)))
    return out


def parts():
    return _center() + champion.mirrored(_left()) + _hat() + _staff() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def coat(back=0.0, sway=0.0):
    return {"extra_coat_1": (back * 0.45, sway * 0.5, 0), "extra_coat_2": (back * 0.3, sway * 0.3, 0),
            "extra_coat_3": (back * 0.25, sway * 0.25, 0)}


def staff(top, foot=None, left=None, pole=(1.0, -0.3, -0.8)):
    """The staff's star at `top`, its foot toward `foot` (default: straight below), the right
    hand on it; `left` puts the left hand on it too, that far from the right hand toward the star
    (negative: toward the foot)."""
    tp = Vector(top)
    ft = Vector(foot) if foot else tp - Vector((0, 0, 1))
    axis = (tp - ft).normalized()
    out = {"ik_hand_r": ik(tuple(tp - axis * STAFF_TOP), aim=tuple(axis), pole=pole)}
    if left is not None:
        out["grip_l"] = grip("hand_r", at=GRIP + left, pole=(1.0, -0.2, -0.6))
    return out


STAND = feet((0.12, 0.05), (0.13, -0.06))
IDLE = merge(STAND, staff(at(0.40, 0.18, 2.12, "r")), arm("l", 0.3, 0.2, -0.9, 0.2, 0.5, -0.6), {"head": (-4, 0, 6)}, coat(1))
READY = merge(feet((0.16, 0.16), (0.16, -0.14)), staff(at(0.34, 0.45, 2.0, "r"), at(0.30, 0.0, 0.6, "r")), arm("l", 0.4, 0.5, -0.5, 0.1, 0.9, 0.0),
              {"spine_01": (6, 0, 0), "head": (-6, 0, 0)}, coat(3))
DOWN = {"pelvis": (0, 0, -0.05)}


def idle(armature):
    b = merge(IDLE, sym("clavicle", 0, -3, 0), {"head": (-12, 0, 10)}, staff(at(0.40, 0.18, 2.14, "r")))
    return _clip(armature, "idle", 80, [(0, IDLE, {"pelvis": (0, 0, -0.01)}), (40, b, {"pelvis": (0, 0, 0.0)})], loop=True)


def idle_fidget_1(armature):
    # Pulls the goggles down, peers at the sky, pushes them back up.
    peer = merge(IDLE, arm("l", 0.2, 0.6, 0.4, -0.6, 0.2, 0.8), {"head": (-34, 0, 4)})
    keys = [(0, IDLE, None), (10, merge(IDLE, arm("l", 0.2, 0.6, 0.4, -0.6, 0.2, 0.85), {"head": (-6, 0, 0)}), None),
            (16, peer, None, "snap"), (34, merge(peer, {"head": (-36, 0, -14)}), None), (42, merge(IDLE, arm("l", 0.2, 0.6, 0.4, -0.6, 0.2, 0.9)), None, "snap"),
            (54, IDLE, None)]
    return _clip(armature, "idle_fidget_1", 54, keys)


def idle_fidget_2(armature):
    # Spins the orrery with a flick of her finger and watches it turn.
    spin_ = merge(IDLE, hand("l", at(0.30, 0.32, 2.05, "r"), aim=at(0.4, 0.4, 0.6, "r")), {"head": (-24, 0, -10)})
    keys = [(0, IDLE, None), (12, spin_, None), (16, merge(spin_, hand("l", at(0.24, 0.30, 2.08, "r"), aim=at(0.4, 0.4, 0.6, "r"))), None, "snap"),
            (34, merge(IDLE, {"head": (-26, 0, -12)}), None), (50, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 50, keys)


def idle_ready(armature):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (8, 0, 0)}, coat(4, 2)), {"pelvis": (0, 0, -0.07)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(swing, trail):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(staff(at(0.36, 0.05 + 0.08 * c, 1.95, "r"), at(0.30, 0.55, 0.75, "r")),
                     arm("l", 0.3, -0.45 * swing * c, -0.8, 0.2, 0.55 - 0.35 * swing * c, -0.35),
                     coat(trail + 5 * math.sin(4 * math.pi * p), 4 * math.sin(2 * math.pi * p)), {"head": (-8, 0, 0)})
    return upper



def walk(armature):
    # A walk (heavy slows, and the moment a run comes to a stop): the run's body type, slower,
    # the weapon carried as in it, never hanging into the ground (14 §3.6).
    keys = gait(32, 0.90, contact=0.6, lift=0.10, bob=0.02, lean=4, twist=6, width=0.12, crouch=0.0, upper=_run_upper(0.4, 10))
    return _clip(armature, "walk", 32, keys, loop=True, markers={"foot_l": 0, "foot_r": 16}, stride_speed=140.0)

def run(armature):
    keys = gait(20, 0.95, contact=0.43, lift=0.24, bob=0.05, lean=10, twist=8, width=0.12, upper=_run_upper(1.0, 30))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    keys = gait(16, 0.96, contact=0.40, lift=0.28, bob=0.055, lean=15, twist=10, width=0.12, upper=_run_upper(1.3, 40))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _point(armature, name, lean_side=1):
    # The orrery thrust at the target: drawn back and up, cocked, a one-frame thrust on `fire`
    # (sim: 11), held out.
    st = feet((0.15, 0.22), (0.16, -0.18))
    back = merge(st, staff(at(0.30, -0.35, 2.15, "r"), at(0.35, 0.30, 1.0, "r")), arm("l", 0.6, 0.3, -0.3, 0.4, 0.5, 0.0),
                 {"spine_01": (-4, 0, -16 * lean_side), "head": (-8, 0, 14 * lean_side)}, coat(3))
    cock = merge(back, staff(at(0.28, -0.48, 2.05, "r"), at(0.36, 0.35, 1.05, "r")), {"spine_01": (-6, 0, -20 * lean_side)})
    thrust = merge(feet((0.15, 0.32), (0.16, -0.18)), staff(at(0.10, 1.30, 1.70, "r"), at(0.28, 0.0, 1.15, "r"), pole=(1.0, -0.4, -0.6)),
                   arm("l", 0.6, -0.3, -0.4, 0.4, -0.1, -0.3), {"spine_01": (12, 0, 10 * lean_side), "head": (-10, 0, -6 * lean_side)}, coat(8))
    keys = [(0, READY, DOWN), (7, back, DOWN), (10, cock, DOWN, "snap"), (11, thrust, {"pelvis": (0, -0.04, -0.07)}),
            (16, merge(thrust, {"spine_01": (14, 0, 12 * lean_side)}), {"pelvis": (0, -0.04, -0.08)}, "settle"), (26, READY, DOWN)]
    return _clip(armature, name, 26, keys, markers={"fire": 11})


def attack_1(armature):
    return _point(armature, "attack_1", 1)


def attack_2(armature):
    return _point(armature, "attack_2", -1)


def q(armature):
    # Arc Shot: the staff swung from low behind her up and over in both hands, lobbing the
    # shot high on `fire` 8, held up at the top.
    st = feet((0.16, 0.26), (0.16, -0.20))
    low = merge(st, staff(at(0.55, -0.85, 0.55, "r"), at(0.20, 0.2, 1.1, "r"), left=0.22), {"spine_01": (14, 0, -14), "head": (-10, 0, 14)}, coat(2))
    cock = merge(low, staff(at(0.50, -1.0, 0.70, "r"), at(0.18, 0.1, 1.1, "r"), left=0.22), {"spine_01": (16, 0, -18)})
    up = merge(feet((0.16, 0.30), (0.16, -0.20)), staff(at(0.10, 0.95, 2.45, "r"), at(0.18, 0.15, 1.30, "r"), left=0.22, pole=(1.0, 0.0, -0.4)),
               {"spine_01": (-6, 0, 6), "spine_02": (-10, 0, 0), "head": (-24, 0, 0)}, coat(8))
    keys = [(0, READY, DOWN), (5, low, {"pelvis": (0, 0.03, -0.12)}), (7, cock, {"pelvis": (0, 0.04, -0.13)}, "snap"), (8, up, {"pelvis": (0, -0.03, 0.0)}),
            (14, merge(up, {"head": (-28, 0, 0)}), {"pelvis": (0, -0.03, 0.0)}, "settle"), (22, READY, DOWN)]
    return _clip(armature, "q", 22, keys, markers={"fire": 8})


def w(armature):
    # Static Field: the staff's foot planted beside her on `fire` 8 while her free hand flings
    # the field out; both held.
    st = feet((0.16, 0.22), (0.16, -0.18))
    lift = merge(st, staff(at(0.35, 0.25, 2.45, "r")), arm("l", 0.4, 0.2, 0.3, -0.4, 0.4, 0.6), {"spine_02": (-6, 0, 0), "head": (-10, 0, 0)}, coat(2))
    plant = merge(st, staff(at(0.40, 0.42, 1.80, "r"), at(0.40, 0.45, 0.0, "r")), arm("l", 0.9, 0.6, -0.1, 0.8, 0.6, 0.0),
                  {"spine_01": (12, 0, 10), "head": (-12, 0, -10)}, coat(6))
    keys = [(0, READY, DOWN), (6, lift, {"pelvis": (0, 0, 0.02)}, "snap"), (8, plant, {"pelvis": (0, -0.03, -0.08)}),
            (14, merge(plant, {"spine_01": (14, 0, 12)}), {"pelvis": (0, -0.03, -0.09)}, "settle"), (22, READY, DOWN)]
    return _clip(armature, "w", 22, keys, markers={"fire": 8})


def e_start(armature):
    # Recoil, a starlight slip (not a vault): she sweeps the staff back so the orrery flares
    # behind her, flings the left hand ahead with the fingers spread, and rises off her toes.
    gather = merge(feet((0.16, 0.14), (0.15, -0.16)), staff(at(0.42, -0.55, 1.45, "r"), at(0.32, 0.55, 0.95, "r")),
                   arm("l", 0.25, 0.9, 0.0, 0.05, 1.0, 0.1), {"fingers_l": (-25, 0, 0), "thumb_l": (-20, 0, 0)},
                   {"spine_01": (18, 0, 0), "head": (-12, 0, 0)}, coat(-8))
    return _clip(armature, "e_start", 4, [(0, gather, {"pelvis": (0, 0, -0.12)}, "snap"), (4, merge(gather, coat(-30)), {"pelvis": (0, 0.0, 0.08)})],
                 markers={"fire": 2})


TRAVEL = merge(staff(at(0.40, -1.45, 1.30, "r"), at(0.34, 0.30, 1.05, "r")), arm("l", 0.2, 1.0, 0.15, 0.0, 1.0, 0.15),
               {"fingers_l": (-25, 0, 0), "thumb_l": (-20, 0, 0), "thigh_l": (-14, 0, 0), "thigh_r": (-24, 0, 0), "calf_l": (24, 0, 0), "calf_r": (40, 0, 0),
                "spine_01": (20, 0, 0), "head": (-16, 0, 0)}, coat(-55))


def e_travel(armature):
    # Gliding a hand's breadth off the ground, legs trailing, the orrery behind her like a
    # comet's head and the coat streaming; a slow float, not a run.
    keys = [(0, TRAVEL, {"pelvis": (0, 0.0, 0.22)}), (5, merge(TRAVEL, coat(-62), {"spine_01": (22, 0, 0)}), {"pelvis": (0, 0.0, 0.26)})]
    return _clip(armature, "e_travel", 10, keys, loop=True)


def e_land(armature):
    # Touches down light on the toes, low, the left hand sweeping down to steady her; the staff
    # swings back upright in front without touching the ground; held.
    touch = merge(feet((0.18, 0.24), (0.16, -0.18)), staff(at(0.38, 0.45, 1.95, "r"), at(0.30, 0.15, 0.55, "r")), arm("l", 0.7, 0.4, -0.4, 0.6, 0.6, -0.6),
                  {"fingers_l": (-10, 0, 0), "spine_01": (14, 0, 0), "head": (-10, 0, 0)}, coat(12))
    keys = [(0, touch, {"pelvis": (0, 0.02, -0.20)}, "settle"), (6, merge(touch, coat(4)), {"pelvis": (0, 0.02, -0.18)}), (12, READY, DOWN)]
    return _clip(armature, "e_land", 12, keys)


def r(armature):
    # Starfall Lance, a channel (the sim: an 0.85 s aim, then 1.5 s of beam): feet planted wide,
    # the staff raised to the sky in both hands, then lowered to aim down the line, trembling as
    # the lance builds; on `fire` 26 the beam bursts out and she braces against it, leaning in,
    # shaking, the coat streaming back, for the whole channel; then she lets it go.
    st = feet((0.22, 0.30), (0.22, -0.26))
    sky = merge(st, staff(at(0.15, 0.25, 2.75, "r"), at(0.20, 0.05, 1.30, "r"), left=0.25, pole=(1.0, 0.0, -0.4)),
                {"spine_02": (-12, 0, 0), "head": (-30, 0, 0)}, coat(4))
    aim = merge(st, staff(at(0.06, 1.35, 1.55, "r"), at(0.20, 0.0, 1.20, "r"), left=0.30), {"spine_01": (14, 0, 0), "head": (-10, 0, 0)}, coat(6))
    keys = [(0, READY, DOWN), (8, sky, {"pelvis": (0, 0, -0.06)}), (14, merge(sky, {"head": (-32, 0, 0)}), {"pelvis": (0, 0, -0.07)}),
            (19, aim, {"pelvis": (0, 0, -0.12)})]
    for i, f in enumerate(range(20, 26)):
        j = 0.012 if i % 2 else -0.012
        keys.append((f, merge(aim, staff(at(0.06 + j, 1.35, 1.55 + j, "r"), at(0.20, 0.0, 1.20, "r"), left=0.30)), {"pelvis": (0, 0.01 * (i + 1), -0.13)}, "snap"))
    # The channel: braced, the staff thrust at the line, shaking with the beam's push.
    st2 = feet((0.24, 0.36), (0.22, -0.30))
    brace = merge(st2, staff(at(0.04, 1.45, 1.50, "r"), at(0.18, 0.10, 1.18, "r"), left=0.34), {"spine_01": (20, 0, 0), "spine_02": (4, 0, 0), "head": (-14, 0, 0)},
                  coat(-24))
    keys.append((26, merge(brace, coat(-36)), {"pelvis": (0, 0.06, -0.18)}))
    for i, f in enumerate(range(29, 72, 3)):
        j = 0.016 if i % 2 else -0.016
        keys.append((f, merge(brace, staff(at(0.04 + j, 1.45, 1.50 - j, "r"), at(0.18, 0.10, 1.18, "r"), left=0.34), coat(-28 - 6 * (i % 2))),
                     {"pelvis": (0, 0.06 + 0.004 * (i % 2), -0.18)}, "snap"))
    release = merge(feet((0.22, 0.24), (0.22, -0.34)), staff(at(0.10, 1.05, 2.05, "r"), at(0.22, -0.1, 1.25, "r"), left=0.30),
                    {"spine_01": (-8, 0, 0), "spine_02": (-8, 0, 0), "head": (-12, 0, 0)}, coat(-6))
    keys += [(72, release, {"pelvis": (0, 0.10, -0.10)}, "settle"), (80, READY, DOWN)]
    return _clip(armature, "r", 80, keys, markers={"fire": 26})


KNEEL = merge({"ik_foot_l": ik(at(0.14, 0.28, champion.ankle(), "l")), "thigh_r": (8, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
              staff(at(0.30, 0.35, 1.70, "r"), at(0.30, 0.38, 0.0, "r")), arm("l", 0.3, 0.6, -0.5, 0.0, 0.6, 0.6), {"spine_01": (6, 0, 0), "head": (-20, 0, 0)})


def recall(armature):
    # Kneels and reads the stars through her goggles.
    keys = [(0, IDLE, None), (15, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (45, merge(KNEEL, {"head": (-28, 0, 10)}), {"pelvis": (0, 0.04, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    hit = merge(feet((0.12, -0.04), (0.13, -0.22)), arm("l", 0.6, -0.2, 0.0, 0.5, 0.1, 0.2), {"spine_01": (-14, 0, 0), "head": (-30, 0, 0)}, coat(-20))
    knees = merge({"thigh_l": (-6, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (110, 0, 0), "calf_r": (110, 0, 0)}, arm("l", 0.3, 0.3, -0.9, 0.2, 0.4, -0.8),
                  {"spine_01": (24, 0, 0), "head": (20, 0, 0)}, coat(-6))
    down = merge({"thigh_l": (-20, 0, 0), "thigh_r": (-12, 0, 0), "calf_l": (40, 0, 0), "calf_r": (24, 0, 0)}, arm("l", 0.9, 0.2, 0.2, 0.8, 0.4, 0.1),
                 {"pelvis": (-76, 16, 0), "head": (18, 0, -14)}, coat(-30))
    keys = [(0, IDLE, None), (6, hit, {"pelvis": (0, 0.06, -0.02)}, "settle"), (15, knees, {"pelvis": (0, 0.02, -0.5)}, "snap"),
            (25, down, {"pelvis": (0, 0.45, -0.82)}, "snap"), (34, down, {"pelvis": (0, 0.47, -0.84)})]
    return _clip(armature, "death", 34, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "settle"), (12, merge(IDLE, {"head": (-10, 0, 0)}), {"pelvis": (0, 0, -0.02)}), (24, IDLE, None)]
    return _clip(armature, "respawn", 24, keys)


def select(armature):
    # Plants the staff, the orrery whirling, and points to a star with her free hand.
    plant = merge(STAND, staff(at(0.30, 0.25, 2.05, "r"), at(0.30, 0.28, 0.0, "r")), arm("l", 0.3, 0.6, 0.75, 0.2, 0.5, 0.9), {"head": (-34, 0, -10)}, coat(4))
    keys = [(0, IDLE, None), (12, plant, None), (36, merge(plant, {"head": (-38, 0, -16)}), None), (50, IDLE, None)]
    return _clip(armature, "select", 50, keys)


def emote_taunt(armature):
    # Measures you against the staff, thumb out, and shakes her head.
    meas = merge(IDLE, arm("l", 0.2, 1, 0.3, 0.05, 1, 0.3), {"head": (-6, 0, 10)})
    keys = [(0, IDLE, None), (10, meas, None), (24, meas, None)]
    for i, f in enumerate((28, 32, 36)):
        keys.append((f, merge(meas, {"head": (-6, 0, 14 if i % 2 else -6)}), None))
    keys.append((48, IDLE, None))
    return _clip(armature, "emote_taunt", 48, keys)


def emote_joke(armature):
    # Tips her hat, the brim swept low.
    tip = merge(IDLE, arm("l", 0.3, 0.6, 0.5, -0.4, 0.2, 0.9), {"spine_01": (24, 0, 0), "head": (20, 0, 0)})
    keys = [(0, IDLE, None), (12, merge(IDLE, arm("l", 0.3, 0.6, 0.5, -0.4, 0.2, 0.9)), None), (20, tip, None), (32, tip, None), (44, IDLE, None)]
    return _clip(armature, "emote_joke", 44, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, None)]
    for i in range(1, 8):
        s = 1.0 if i % 2 else 0.4
        keys.append((5 * i, merge(IDLE, arm("l", 0.2, 0.5, -0.2, -0.5, 0.2, 0.85), {"head": (10 * s, 0, 0), "spine_01": (6 * s, 0, 0)}), None))
    keys.append((44, IDLE, None))
    return _clip(armature, "emote_laugh", 44, keys)


def emote_dance(armature):
    # A jaunty step around the planted staff.
    keys = []
    for i, f in enumerate(range(0, 48, 6)):
        a = 2 * math.pi * i / 8
        keys.append((f, merge(feet((0.12 + 0.05 * math.cos(a), 0.06 + 0.06 * math.sin(a)), (0.13, -0.06), lift_l=0.1 * (i % 2)),
                              staff(at(0.40, 0.25, 2.05, "r"), at(0.40, 0.28, 0.0, "r")), arm("l", 0.9, 0.2 * math.sin(a), 0.3, 0.9, 0.3, 0.4),
                              {"pelvis": (0, 0, 14 * math.sin(a)), "head": (-8, 0, -10 * math.sin(a))}, coat(4, 6 * math.sin(a))), {"pelvis": (0, 0, -0.03 * (i % 2))}))
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, walk, run, run_fast, attack_1, attack_2, q, w, e_start, e_travel, e_land, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    rig.add_chain(armature, "coat", "pelvis", COAT)
    # Bolts and the lance leave from the orrery's star.
    h, t = Vector(BONES["hand_r"][2]), Vector(BONES["hand_r"][3])
    champion.place_socket(armature, "socket_projectile", tuple(h + (t - h).normalized() * STAFF_TOP))


def build(only=None):
    return champion.build("quill", SHAPE, parts, OWN, 2.0, extras=_extras, only=only, shared=champion.SHARED + ["attack_melee_alt"], cloth={"coat": dict(stiffness=0.24, inertia=0.86)})


if __name__ == "__main__":
    champion.main(build)
