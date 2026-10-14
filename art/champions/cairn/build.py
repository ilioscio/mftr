# SPDX-License-Identifier: AGPL-3.0-or-later
"""Cairn (rebuilt for 14): the generator for his model and clips.

    blender -b --python art/champions/cairn/build.py -- --out art/champions/cairn/cairn.blend

Antlers and a mossy stone mantle (14 §5): a weathered earth-warden, broad and heavy, great
branching antlers rising from his brow, a mantle of mossy stone slabs over his shoulders, bark
plates on his arms and shins, a moss-green cloak on an `extra_cloak` chain, and a gnarled staff
crowned with a glowing emerald, held in both hands. His signature is emerald (14 §4).

Weight (14 §3): a heavy gait; his staff blows are two-handed, with a cock and a held impact; his
big spells drive the staff into the ground and leave it there. Feet planted by IK.
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

SKIN = (0.62, 0.44, 0.32)
BARK = (0.26, 0.18, 0.12)
BARK_DARK = (0.15, 0.10, 0.07)
GRANITE = (0.30, 0.30, 0.28)
GRANITE_DARK = (0.18, 0.18, 0.17)
MOSS = (0.16, 0.30, 0.10)
MOSS_DARK = (0.09, 0.18, 0.06)
ANTLER = (0.62, 0.56, 0.44)
LEATHER = (0.30, 0.20, 0.12)
HAIR = (0.30, 0.26, 0.22)
EMERALD = (0.30, 1.0, 0.55)
EMERALD_DEEP = (0.06, 0.62, 0.30)
ACCENT = (0.18, 0.52, 0.95)

SHAPE = {"heights": [(0.0, 0.0), (0.54, 0.51), (1.00, 0.96), (2.15, 2.12)], "x": 1.15, "y": 1.1, "scale": 1.03, "shoulder": 0.05, "arm": 1.05, "hand": 1.4}
rig.use_shape(SHAPE)
BONES = {b[0]: b for b in rig.bone_table()}
CLOAK = [rig.shaped(p) for p in [(0, 0.15, 1.52), (0, 0.20, 1.18), (0, 0.24, 0.84), (0, 0.26, 0.48)]]
HAND_LEN = (Vector(BONES["hand_r"][3]) - Vector(BONES["hand_r"][2])).length
GRIP = 0.5 * HAND_LEN
STAFF_TOP = GRIP + 1.0
STAFF_FOOT = GRIP - 0.75


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", sp((0, 0, 0.90)), sp((0, 0, 1.12)), 10, [(0, .16, .13), (.5, .18, .14), (1, .17, .135)], "cloth", LEATHER),
        ("pelvis", sp((0, 0, 1.04)), sp((0, 0, 1.11)), 10, [(0, .19, .15), (1, .185, .145)], "accent", ACCENT),
        # A kilt of hanging bark slats and moss.
        *[box_part("pelvis", sp((x, -0.15 + 0.1 * abs(x) ** 1.5, 0.80)), (0.10, 0.03, 0.34), "cloth", BARK if k % 2 else BARK_DARK, rot=(-8, 0, x * 50), taper=0.85, bevel=0.2)
          for k, x in enumerate((-0.17, -0.06, 0.06, 0.17))],
        *[box_part("pelvis", sp((x, 0.15, 0.80)), (0.12, 0.03, 0.34), "cloth", MOSS_DARK, rot=(8, 0, -x * 40), taper=0.85, bevel=0.2) for x in (-0.12, 0.0, 0.12)],
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 10, [(0, .15, .115), (1, .16, .12)], "cloth", LEATHER),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 10, [(0, .165, .125), (1, .185, .135)], "cloth", BARK),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.60)), 10, [(0, .19, .135), (.5, .195, .14), (1, .11, .09)], "cloth", BARK),
        # Bark plates over his chest.
        box_part("chest", sp((0.08, -0.13, 1.44)), (0.12, 0.05, 0.18), "cloth", BARK_DARK, rot=(0, -10, 0), bevel=0.3),
        box_part("chest", sp((-0.08, -0.13, 1.44)), (0.12, 0.05, 0.18), "cloth", BARK_DARK, rot=(0, 10, 0), bevel=0.3),
        box_part("chest", sp((0, -0.15, 1.32)), (0.06, 0.03, 0.06), "emissive", EMERALD, rot=(0, 45, 0)),
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .07, .07), (1, .062, .062)], "skin", SKIN),
        # The stone mantle: slabs over his shoulders and around his neck, mossed on top.
        path_part("chest", [sp((0.0, 0.0, 1.56)), sp((0.0, 0.0, 1.66))], 10, [(.22, .18), (.15, .13)], "cloth", GRANITE),
        box_part("chest", sp((0, 0.10, 1.62)), (0.36, 0.18, 0.10), "cloth", GRANITE_DARK, rot=(-12, 0, 0), bevel=0.35),
        box_part("chest", sp((0, 0.11, 1.685)), (0.30, 0.14, 0.04), "cloth", MOSS, rot=(-12, 0, 0), bevel=0.4),
        # The cloak.
        ("extra_cloak_1", CLOAK[0], CLOAK[1], 8, [(0, .20, .024), (1, .23, .024)], "cloth", MOSS),
        ("extra_cloak_2", CLOAK[1], CLOAK[2], 8, [(0, .23, .024), (1, .25, .024)], "cloth", MOSS),
        ("extra_cloak_3", CLOAK[2], CLOAK[3], 8, [(0, .25, .024), (.75, .24, .022), (1, .18, .02)], "cloth", MOSS_DARK),
    ]


def _antlers():
    # Great branching antlers from his brow: a main beam sweeping up and back with three tines.
    out = []
    for s in (1, -1):
        def q(x, y, z):
            return sp((x * s, y, z))
        beam = [q(0.07, -0.06, 1.82), q(0.16, -0.04, 1.92), q(0.27, 0.02, 2.04), q(0.34, 0.10, 2.18), q(0.36, 0.18, 2.32), q(0.33, 0.24, 2.42)]
        out.append(path_part("head", beam, 6, [(.035, .035), (.032, .032), (.028, .028), (.024, .024), (.018, .018), (0, 0)], "cloth", ANTLER))
        for base, tip in ((q(0.21, -0.02, 1.98), q(0.22, -0.12, 2.12)), (q(0.30, 0.05, 2.10), q(0.42, -0.02, 2.26)), (q(0.35, 0.14, 2.25), q(0.46, 0.14, 2.38))):
            mid = tuple((Vector(base) + Vector(tip)) / 2 + Vector((0, 0, 0.03)))
            out.append(path_part("head", [base, mid, tip], 5, [(.022, .022), (.016, .016), (0, 0)], "cloth", ANTLER))
        # A band of emerald beads at the antler's root.
        out.append(box_part("head", q(0.09, -0.07, 1.83), (0.035, 0.035, 0.035), "emissive", EMERALD, rot=(45, 0, 45)))
    return out


def _head(bm, layers, groups, mats):
    head.build(bm, layers, groups, mats, head.Head(
        base=sp((0, -0.01, 1.63)), height=0.25, scale=1.16, jaw=0.2, chin=1.15, eye=0.85,
        skin=SKIN, hair=HAIR, hair_style="short", eye_color=(0.2, 0.5, 0.25), lips=(0.5, 0.32, 0.28), beard=True))


def _paint():
    # A stripe of emerald paint across his eyes.
    return [box_part("head", sp((0, -0.115, 1.745)), (0.20, 0.02, 0.03), "emissive", EMERALD_DEEP, rot=(0, 0, 0))]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        # A stone pauldron with moss on top.
        path_part("clavicle_l", [("clavicle_l", 0.5, (0, 0, 0.07)), ("clavicle_l", 1.0, (0.04, 0, 0.12)), ("clavicle_l", 1.0, (0.16, 0, 0.08)),
                                 ("clavicle_l", 1.0, (0.22, 0, -0.04)), ("clavicle_l", 1.0, (0.23, 0, -0.12))],
                  6, [(.16, .06), (.20, .09), (.21, .10), (.19, .07), (.15, .03)], "cloth", GRANITE, ref=(0, 1, 0)),
        box_part("clavicle_l", ("clavicle_l", 1.0, (0.08, 0.0, 0.21)), (0.24, 0.26, 0.04), "cloth", MOSS, rot=(0, -20, 0), bevel=0.4),
        (ua, (ua, 0), (ua, 1), 10, [(0, .085, .085), (1, .075, .075)], "skin", SKIN),
        (ua, (ua, .30), (ua, .55), 10, [(0, .09, .09), (1, .088, .088)], "cloth", LEATHER),
        (fa, (fa, 0), (fa, 1), 10, [(0, .07, .07), (1, .062, .062)], "skin", SKIN),
        # Bark bracers.
        (fa, (fa, .35), (fa, .98), 6, [(0, .08, .078), (1, .072, .07)], "cloth", BARK),
        box_part(fa, (fa, 0.6, (0.06, 0.0, 0.0)), (0.04, 0.08, 0.18), "cloth", BARK_DARK, rot=(0, 30, 0), bevel=0.3),
        *champion.hand_parts("l", SKIN, "skin", scale=1.4),
        (th, sp((0.10, 0, 1.02)), (th, 1), 10, [(0, .12, .115), (.5, .105, .10), (1, .08, .08)], "cloth", LEATHER),
        (ca, (ca, 0), (ca, 1), 10, [(0, .08, .08), (.5, .072, .072), (1, .062, .062)], "cloth", LEATHER),
        # Bark greaves and wrapped feet.
        (ca, (ca, .15), (ca, .85), 6, [(0, .09, .088), (1, .078, .076)], "cloth", BARK),
        box_part(ca, (ca, 0.02, (0, -0.06, 0)), (0.11, 0.06, 0.12), "cloth", GRANITE, bevel=0.3),
        box_part("foot_l", ("foot_l", 0.45), (0.12, 0.20, 0.10), "cloth", LEATHER, bevel=0.3),
        box_part("toe_l", ("toe_l", 0.35), (0.11, 0.09, 0.07), "cloth", LEATHER, bevel=0.3),
    ]


def _staff():
    """The gnarled staff, skinned to the right hand and laid along it, crowned by an emerald held
    in twisted roots."""
    h, t = Vector(BONES["hand_r"][2]), Vector(BONES["hand_r"][3])
    axis = (t - h).normalized()
    side = axis.cross(Vector((0, 1, 0))).normalized()
    fwd = side.cross(axis).normalized()

    def p(d, off=Vector()):
        return tuple(h + axis * d + off)

    hb = "hand_r"
    # A wandering shaft.
    shaft = [p(STAFF_FOOT), p(STAFF_FOOT + 0.4, side * 0.02), p(GRIP, fwd * 0.01), p(GRIP + 0.4, -side * 0.025), p(STAFF_TOP - 0.25, fwd * 0.02)]
    c = h + axis * STAFF_TOP
    out = [path_part(hb, shaft, 6, [(.03, .03), (.034, .034), (.036, .036), (.034, .034), (.04, .04)], "cloth", BARK)]
    for k in range(4):
        a = 2 * math.pi * k / 4
        root = side * math.cos(a) + fwd * math.sin(a)
        out.append(path_part(hb, [p(STAFF_TOP - 0.26, root * 0.03), p(STAFF_TOP - 0.08, root * 0.11), p(STAFF_TOP + 0.10, root * 0.08), p(STAFF_TOP + 0.18, root * 0.02)], 5,
                             [(.022, .022), (.02, .02), (.015, .015), (0, 0)], "cloth", BARK_DARK))
    out.append(path_part(hb, [p(STAFF_TOP - 0.11), p(STAFF_TOP), p(STAFF_TOP + 0.13)], 6, [(.0, .0), (.075, .075), (0, 0)], "emissive", EMERALD))
    out.append(box_part(hb, tuple(c), (0.05, 0.05, 0.05), "emissive", EMERALD_DEEP, rot=(45, 45, 0)))
    out.append(box_part(hb, p(GRIP + 0.55, side * 0.04), (0.08, 0.06, 0.06), "cloth", MOSS, bevel=0.35))
    return out


def parts():
    return _center() + champion.mirrored(_left()) + _antlers() + _paint() + _staff() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def cloak(back=0.0, sway=0.0):
    return {"extra_cloak_1": (back * 0.45, sway * 0.5, 0), "extra_cloak_2": (back * 0.3, sway * 0.3, 0),
            "extra_cloak_3": (back * 0.25, sway * 0.25, 0)}


def staff(top, foot=None, left=None, pole=(1.0, -0.3, -0.8), left_pole=(1.0, -0.2, -0.6)):
    tp = Vector(top)
    ft = Vector(foot) if foot else tp - Vector((0, 0, 1))
    axis = (tp - ft).normalized()
    out = {"ik_hand_r": ik(tuple(tp - axis * STAFF_TOP), aim=tuple(axis), pole=pole)}
    if left is not None:
        out["grip_l"] = grip("hand_r", at=GRIP + left, pole=left_pole)
    return out


STAND = feet((0.18, 0.06), (0.18, -0.08))
IDLE = merge(STAND, staff(at(0.42, 0.22, 2.05, "r"), at(0.42, 0.28, 0.0, "r")), arm("l", 0.35, 0.25, -0.9, 0.25, 0.55, -0.6),
             {"spine_01": (6, 0, 0), "head": (-6, 0, 4)}, cloak(1))
GUARD = merge(feet((0.24, 0.18), (0.24, -0.14)), staff(at(0.40, 0.75, 1.85, "l"), at(0.20, 0.10, 0.65, "r"), left=0.42),
              {"spine_01": (12, 0, 8), "head": (-10, 0, -8)}, cloak(4))
DOWN = {"pelvis": (0, 0, -0.08)}


def idle(armature):
    b = merge(IDLE, sym("clavicle", 0, -4, 0), {"chest": (-3, 0, 0), "head": (-9, 0, 4)}, cloak(2, 2))
    return _clip(armature, "idle", 84, [(0, IDLE, {"pelvis": (0, 0, -0.02)}), (42, b, {"pelvis": (0, 0, -0.01)})], loop=True)


def idle_fidget_1(armature):
    # Scratches his beard, slowly, considering.
    s = merge(IDLE, hand("l", at(0.04, 0.16, 1.62, "l"), aim=at(-0.6, 0.4, 0.4, "l")), {"head": (6, 0, 10)})
    keys = [(0, IDLE, None), (12, s, None)]
    for i, f in enumerate(range(16, 36, 4)):
        keys.append((f, merge(s, hand("l", at(0.04 + (0.02 if i % 2 else -0.02), 0.16, 1.60, "l"), aim=at(-0.6, 0.4, 0.4, "l"))), None))
    keys.append((50, IDLE, None))
    return _clip(armature, "idle_fidget_1", 50, keys)


def idle_fidget_2(armature):
    # Taps the staff's foot on the ground twice: each tap lands and holds.
    lift = merge(IDLE, staff(at(0.42, 0.22, 2.20, "r"), at(0.42, 0.28, 0.15, "r")))
    keys = [(0, IDLE, None), (8, lift, None, "whip"), (11, IDLE, {"pelvis": (0, 0, -0.03)}), (18, IDLE, None), (24, lift, None, "whip"),
            (27, IDLE, {"pelvis": (0, 0, -0.03)}), (40, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 40, keys)


def idle_ready(armature):
    keys = [(0, GUARD, DOWN), (20, merge(GUARD, {"spine_01": (14, 0, 8)}, cloak(5, 2)), {"pelvis": (0, 0, -0.10)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(swing, trail):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(staff(at(0.38, 0.10 + 0.06 * c, 1.85, "r"), at(0.30, 0.65, 0.65, "r")),
                     arm("l", 0.35, -0.45 * swing * c, -0.75, 0.2, 0.5 - 0.35 * swing * c, -0.4),
                     cloak(trail + 5 * math.sin(4 * math.pi * p), 4 * math.sin(2 * math.pi * p)), {"head": (-10, 0, 0)})
    return upper


def run(armature):
    keys = gait(20, 0.94, contact=0.44, lift=0.20, bob=0.06, lean=12, twist=9, width=0.17, crouch=0.03, upper=_run_upper(1.0, 28))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    keys = gait(16, 0.96, contact=0.40, lift=0.24, bob=0.065, lean=16, twist=11, width=0.17, crouch=0.04, upper=_run_upper(1.3, 38))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(armature):
    # A two-handed overhead strike: raised, cocked back, down in one frame on `fire` (sim: 14),
    # the emerald end striking the ground in front and held.
    st = feet((0.24, 0.18), (0.24, -0.14))
    up = merge(st, staff(at(0.20, -0.45, 2.55, "r"), at(0.15, 0.20, 1.60, "r"), left=0.30, pole=(1.0, 0.1, 0.0)), {"spine_02": (-10, 0, 0), "head": (-10, 0, 0)}, cloak(2))
    cock = merge(st, staff(at(0.18, -0.85, 2.05, "r"), at(0.12, 0.10, 1.80, "r"), left=0.30, pole=(1.0, 0.1, 0.0)), {"spine_01": (-6, 0, 0), "spine_02": (-14, 0, 0)}, cloak(0))
    hit = merge(feet((0.26, 0.32), (0.26, -0.18)), staff(at(0.05, 1.45, 0.12, "r"), at(0.08, 0.35, 1.05, "r"), left=0.30, pole=(1.0, -0.2, -0.4)),
                {"spine_01": (24, 0, 0), "spine_02": (10, 0, 0), "head": (-14, 0, 0)}, cloak(8))
    keys = [(0, GUARD, DOWN), (7, up, {"pelvis": (0, 0.02, 0.0)}), (12, cock, {"pelvis": (0, 0.03, -0.04)}, "snap"),
            (13, merge(st, staff(at(0.06, 1.2, 2.0, "r"), at(0.08, 0.2, 1.7, "r"), left=0.30, pole=(1.0, 0.0, 0.0))), {"pelvis": (0, -0.03, -0.12)}, "snap"),
            (14, hit, {"pelvis": (0, -0.06, -0.22)}), (20, merge(hit, {"spine_01": (26, 0, 0)}), {"pelvis": (0, -0.06, -0.23)}, "settle"), (32, GUARD, DOWN)]
    return _clip(armature, "attack_1", 32, keys, markers={"fire": 14})


def attack_2(armature):
    # A low sweeping strike from right to left, cocked back, swept through on `fire`, held.
    st = feet((0.26, 0.20), (0.26, -0.16))
    wind = merge(st, staff(at(0.95, -0.55, 0.60, "r"), at(0.25, 0.15, 1.10, "r"), left=0.32), {"pelvis": (0, 0, -18), "spine_01": (10, 0, -22), "head": (-8, 0, 18)}, cloak(3, -3))
    cock = merge(st, staff(at(0.75, -0.95, 0.70, "r"), at(0.28, 0.0, 1.10, "r"), left=0.32), {"pelvis": (0, 0, -24), "spine_01": (10, 0, -30)}, cloak(2, -4))
    hit = merge(st, staff(at(1.15, 1.05, 0.55, "l"), at(0.20, 0.40, 1.05, "l"), left=0.32, pole=(1.0, 0.2, -0.6)),
                {"pelvis": (0, 0, 22), "spine_01": (12, 0, 28), "head": (-8, 0, -20)}, cloak(8, 5))
    keys = [(0, GUARD, DOWN), (8, wind, {"pelvis": (0, 0, -0.10)}), (12, cock, {"pelvis": (0, 0, -0.11)}, "snap"),
            (13, merge(st, staff(at(0.1, 1.55, 0.65, "r"), at(0.1, 0.45, 1.0, "r"), left=0.32)), {"pelvis": (0, -0.03, -0.12)}, "snap"),
            (14, hit, {"pelvis": (0, -0.04, -0.13)}), (20, merge(hit, {"spine_01": (13, 0, 34)}), {"pelvis": (0, -0.04, -0.13)}, "settle"), (32, GUARD, DOWN)]
    return _clip(armature, "attack_2", 32, keys, markers={"fire": 14})


def q(armature):
    # Stone Lash: the staff whipped forward and level from his right side on `fire` 8, sending
    # the lash down the line; held out.
    st = feet((0.24, 0.24), (0.24, -0.18))
    back = merge(st, staff(at(0.85, -0.40, 1.40, "r"), at(0.20, 0.20, 1.0, "r"), left=0.30), {"pelvis": (0, 0, -16), "spine_01": (8, 0, -20), "head": (-8, 0, 16)}, cloak(2))
    whip = merge(feet((0.24, 0.32), (0.24, -0.18)), staff(at(0.10, 1.55, 1.25, "r"), at(0.15, 0.40, 1.05, "r"), left=0.30, pole=(1.0, -0.3, -0.5)),
                 {"spine_01": (16, 0, 10), "head": (-12, 0, -6)}, cloak(8))
    keys = [(0, GUARD, DOWN), (5, back, {"pelvis": (0, 0.02, -0.09)}), (7, merge(back, {"spine_01": (8, 0, -26)}), {"pelvis": (0, 0.03, -0.09)}, "snap"),
            (8, whip, {"pelvis": (0, -0.05, -0.12)}), (14, merge(whip, {"spine_01": (18, 0, 12)}), {"pelvis": (0, -0.05, -0.13)}, "settle"), (22, GUARD, DOWN)]
    return _clip(armature, "q", 22, keys, markers={"fire": 8})


def w(armature):
    # Shelter (upper body, a pulse): the staff raised high and the free palm held out on `fire`
    # 4, a ward of stone and moss raised; held.
    ward = merge(staff(at(0.30, 0.30, 2.45, "r")), arm("l", 0.1, 1, 0.2, 0.0, 1, 0.3), {"head": (-8, 0, 0)}, cloak(2))
    keys = [(0, IDLE, None), (3, merge(ward, arm("l", 0.2, 0.6, 0.2, -0.4, 0.4, 0.6)), None, "snap"), (4, ward, None),
            (12, merge(ward, {"spine_02": (-6, 0, 0)}), None, "settle"), (22, IDLE, None)]
    return _clip(armature, "w", 22, keys, layer="upper", markers={"fire": 4})


def e(armature):
    # Rockfall: the free arm flung toward the target and the staff stamped down beside him on
    # `fire` 8; held as the rocks fall.
    st = feet((0.24, 0.20), (0.24, -0.16))
    raise_ = merge(st, staff(at(0.40, 0.25, 2.40, "r"), at(0.40, 0.30, 0.40, "r")), arm("l", 0.3, 0.3, 0.8, 0.2, 0.3, 0.9), {"spine_02": (-8, 0, 0), "head": (-14, 0, 0)}, cloak(2))
    stamp = merge(st, staff(at(0.40, 0.30, 1.85, "r"), at(0.40, 0.32, 0.0, "r")), arm("l", 0.2, 1, 0.3, 0.1, 1, 0.35), {"spine_01": (14, 0, 0), "head": (-12, 0, 0)}, cloak(6))
    keys = [(0, GUARD, DOWN), (6, raise_, {"pelvis": (0, 0, 0.03)}, "snap"), (8, stamp, {"pelvis": (0, -0.03, -0.12)}),
            (14, merge(stamp, {"spine_01": (16, 0, 0)}), {"pelvis": (0, -0.03, -0.13)}, "settle"), (22, GUARD, DOWN)]
    return _clip(armature, "e", 22, keys, markers={"fire": 8})


def r(armature):
    # Monolith: the staff lifted high in both hands, then driven point-first into the ground at
    # his feet on `fire` 8; he leans on it, holding, as the stone rises.
    st = feet((0.26, 0.20), (0.26, -0.16))
    high = merge(st, staff(at(0.10, 0.20, 2.80, "r"), at(0.10, 0.25, 1.30, "r"), left=0.30, pole=(1.0, 0.0, -0.4)), {"spine_02": (-12, 0, 0), "head": (-22, 0, 0)}, cloak(3))
    drive = merge(feet((0.28, 0.18), (0.28, -0.18)), staff(at(0.08, 0.45, 1.35, "r"), at(0.06, 0.50, -0.30, "r"), left=0.30, pole=(1.0, -0.2, -0.5)),
                  {"spine_01": (26, 0, 0), "spine_02": (10, 0, 0), "head": (-20, 0, 0)}, cloak(8))
    keys = [(0, GUARD, DOWN), (5, high, {"pelvis": (0, 0, 0.05)}), (7, merge(high, {"spine_02": (-16, 0, 0)}), {"pelvis": (0, 0, 0.06)}, "snap"),
            (8, drive, {"pelvis": (0, -0.04, -0.30)}), (18, merge(drive, {"spine_01": (28, 0, 0)}), {"pelvis": (0, -0.04, -0.31)}),
            (24, merge(drive, {"spine_01": (24, 0, 0), "head": (-26, 0, 0)}), {"pelvis": (0, -0.04, -0.28)}, "settle"), (34, GUARD, DOWN)]
    return _clip(armature, "r", 34, keys, markers={"fire": 8})


KNEEL = merge({"ik_foot_l": ik(at(0.18, 0.30, champion.ankle(), "l")), "thigh_r": (8, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
              staff(at(0.28, 0.35, 1.70, "r"), at(0.28, 0.38, 0.0, "r"), left=-0.12), {"spine_01": (10, 0, 0), "head": (20, 0, 0)}, cloak(-4))


def recall(armature):
    keys = [(0, IDLE, None), (15, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (45, merge(KNEEL, {"head": (24, 0, 0)}), {"pelvis": (0, 0.04, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    knees = merge({"thigh_l": (-6, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (110, 0, 0), "calf_r": (110, 0, 0)},
                  staff(at(0.30, 0.40, 1.50, "r"), at(0.30, 0.45, 0.0, "r")), arm("l", 0.3, 0.3, -0.9, 0.2, 0.4, -0.8), {"spine_01": (24, 0, 0), "head": (24, 0, 0)}, cloak(-6))
    down = merge({"thigh_l": (-14, 0, 0), "thigh_r": (-10, 0, 0), "calf_l": (30, 0, 0), "calf_r": (20, 0, 0)}, arm("l", 0.8, 0.5, 0.0, 0.7, 0.6, 0.0),
                 arm("r", 0.6, 0.7, 0.1, 0.5, 0.8, 0.0), {"pelvis": (84, 0, 0), "head": (-20, 0, 12)}, cloak(20))
    keys = [(0, IDLE, None), (8, merge(IDLE, {"spine_01": (-10, 0, 0), "head": (-26, 0, 0)}), {"pelvis": (0, 0.05, 0)}, "settle"),
            (16, knees, {"pelvis": (0, 0, -0.5)}, "snap"), (26, down, {"pelvis": (0, -0.64, -0.84)}, "snap"), (34, down, {"pelvis": (0, -0.65, -0.85)})]
    return _clip(armature, "death", 34, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "settle"), (14, merge(IDLE, {"head": (-10, 0, 0)}), {"pelvis": (0, 0, -0.03)}), (26, IDLE, None)]
    return _clip(armature, "respawn", 26, keys)


def select(armature):
    # Lifts the staff high, the emerald blazing, and brings it down with a thump that stays.
    up = merge(STAND, staff(at(0.30, 0.20, 2.75, "r"), at(0.30, 0.25, 1.0, "r")), arm("l", 0.9, 0.3, 0.3, 0.9, 0.3, 0.4), {"head": (-22, 0, 0)}, cloak(4))
    keys = [(0, IDLE, None), (14, up, {"pelvis": (0, 0, 0.02)}), (28, merge(up, {"head": (-26, 0, 0)}), {"pelvis": (0, 0, 0.02)}, "whip"),
            (32, IDLE, {"pelvis": (0, 0, -0.05)}), (42, IDLE, {"pelvis": (0, 0, -0.04)}), (52, IDLE, None)]
    return _clip(armature, "select", 52, keys)


def emote_taunt(armature):
    # Beckons slowly with two fingers.
    out = merge(IDLE, arm("l", 0.2, 1, 0.0, 0.1, 1, 0.1))
    curl = merge(IDLE, arm("l", 0.2, 1, 0.0, 0.1, 0.4, 0.9))
    keys = [(0, IDLE, None), (10, out, None), (16, curl, None), (22, out, None), (28, curl, None), (40, IDLE, None)]
    return _clip(armature, "emote_taunt", 40, keys)


def emote_joke(armature):
    # Hangs his cloak on his antler tine and looks innocent.
    reach = merge(IDLE, arm("l", 0.4, 0.2, 0.7, 0.2, -0.2, 1), {"head": (-6, 0, 20)}, cloak(-10, 10))
    keys = [(0, IDLE, None), (12, reach, None), (30, merge(reach, {"head": (-14, 0, -10)}), None), (44, IDLE, None)]
    return _clip(armature, "emote_joke", 44, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, None)]
    for i in range(1, 9):
        s = 1.0 if i % 2 else 0.5
        keys.append((4 * i, merge(IDLE, {"spine_01": (-6 * s, 0, 0), "head": (-20 * s, 0, 0)}, sym("clavicle", 0, -5 * s, 0)), {"pelvis": (0, 0, -0.01 * s)}))
    keys.append((44, IDLE, None))
    return _clip(armature, "emote_laugh", 44, keys)


def emote_dance(armature):
    # A slow stomping circle around his planted staff.
    keys = []
    for i, f in enumerate(range(0, 48, 6)):
        a = 2 * math.pi * i / 8
        keys.append((f, merge(feet((0.18, 0.06), (0.18, -0.08), lift_l=0.18 * (i % 2 == 0), lift_r=0.18 * (i % 2 == 1)),
                              staff(at(0.42, 0.22, 2.05, "r"), at(0.42, 0.28, 0.0, "r")), arm("l", 0.8, 0.2 * math.sin(a), 0.2, 0.8, 0.3, 0.3),
                              {"pelvis": (0, 0, 12 * math.sin(a)), "head": (-8, 0, -8 * math.sin(a))}, cloak(3, 5 * math.sin(a))), {"pelvis": (0, 0, -0.04 * (i % 2))}))
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    rig.add_chain(armature, "cloak", "chest", CLOAK)


def build(only=None):
    return champion.build("cairn", SHAPE, parts, OWN, 2.1, extras=_extras, only=only)


if __name__ == "__main__":
    champion.main(build)
