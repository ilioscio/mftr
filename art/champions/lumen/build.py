# SPDX-License-Identifier: AGPL-3.0-or-later
"""Lumen (rebuilt for 14): the generator for her model and clips.

    blender -b --python art/champions/lumen/build.py -- --out art/champions/lumen/lumen.blend

A bell (14 §5): a serene light-priestess in a long robe that flares to the ground, wide sleeves,
a great halo ring behind her head, and a tall lantern-staff in her left hand. She doesn't walk so
much as glide. Her signature is a pale sun gold (14 §4).

Weight (14 §3): her weight is in the staff: it's planted when she casts her big spell, held
upright by IK when she stands; her gestures are slow in and quick out, each one held a beat.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import champion, clips, head, rig  # noqa: E402
from mftr_blender.champion import arm, feet, hand  # noqa: E402
from mftr_blender.clips import at, ik, merge, sym  # noqa: E402
from mftr_blender.library import _clip  # noqa: E402
from mftr_blender.mesh import box_part, path_part  # noqa: E402

SKIN = (0.80, 0.62, 0.52)
IVORY = (0.50, 0.46, 0.38)
WHITE = (0.58, 0.56, 0.52)
GOLD = (0.70, 0.54, 0.22)
SKY = (0.20, 0.32, 0.50)
HAIR = (0.78, 0.66, 0.40)
WOOD = (0.36, 0.26, 0.16)
SUN = (1.0, 0.86, 0.42)
SUN_WHITE = (1.0, 0.96, 0.78)
ACCENT = (0.18, 0.52, 0.95)

SHAPE = {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 1.0, "y": 1.0, "scale": 0.96, "shoulder": 0.0, "hand": 1.3}
rig.use_shape(SHAPE)
BONES = {b[0]: b for b in rig.bone_table()}
HAND_LEN = (Vector(BONES["hand_l"][3]) - Vector(BONES["hand_l"][2])).length
GRIP = 0.5 * HAND_LEN
STAFF_TOP = GRIP + 1.10    # the lantern, along hand_l from the wrist
STAFF_FOOT = GRIP - 0.62   # the staff's foot


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        # The bell: a robe flaring from her waist to the ground, white over ivory, gold-hemmed.
        path_part("pelvis", [sp((0, 0, 1.12)), sp((0, 0.01, 0.90)), sp((0, 0.02, 0.55)), sp((0, 0.03, 0.18)), sp((0, 0.03, 0.04))], 12,
                  [(.15, .12), (.20, .17), (.28, .25), (.38, .34), (.42, .38)], "cloth", WHITE),
        path_part("pelvis", [sp((0, 0.03, 0.12)), sp((0, 0.03, 0.03))], 12, [(.405, .365), (.43, .39)], "metal", GOLD),
        # A long ivory panel down the front, sky-edged.
        box_part("pelvis", sp((0, -0.215, 0.62)), (0.14, 0.02, 0.86), "cloth", IVORY, rot=(-13, 0, 0), taper=1.8),
        box_part("pelvis", sp((0, -0.30, 0.22)), (0.25, 0.025, 0.05), "cloth", SKY, rot=(-13, 0, 0)),
        # A second, shorter tier over the robe, and a sash with long tails at her back.
        path_part("pelvis", [sp((0, 0, 1.06)), sp((0, 0.01, 0.86)), sp((0, 0.015, 0.66))], 12, [(.165, .135), (.23, .20), (.30, .27)], "cloth", IVORY),
        path_part("pelvis", [sp((0, 0.015, 0.68)), sp((0, 0.015, 0.64))], 12, [(.30, .27), (.305, .275)], "metal", GOLD),
        box_part("pelvis", sp((0.05, 0.17, 0.80)), (0.07, 0.02, 0.50), "cloth", SKY, rot=(8, 0, 4)),
        box_part("pelvis", sp((-0.05, 0.17, 0.82)), (0.07, 0.02, 0.46), "cloth", SKY, rot=(8, 0, -4)),
        ("pelvis", sp((0, 0, 1.04)), sp((0, 0, 1.10)), 10, [(0, .16, .13), (1, .155, .125)], "accent", ACCENT),
        box_part("pelvis", sp((0, -0.14, 1.07)), (0.08, 0.03, 0.07), "metal", GOLD, bevel=0.3),
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 10, [(0, .12, .09), (1, .125, .094)], "cloth", IVORY),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 10, [(0, .126, .094), (1, .148, .104)], "cloth", WHITE),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.60)), 10, [(0, .15, .105), (.4, .156, .11), (1, .088, .074)], "cloth", WHITE),
        # A gold-edged mantle over the shoulders and a sun brooch.
        ("chest", sp((0, 0.0, 1.47)), sp((0, -0.005, 1.60)), 12, [(0, .205, .15), (.6, .17, .125), (1, .10, .088)], "cloth", IVORY),
        ("chest", sp((0, 0.0, 1.455)), sp((0, 0.0, 1.475)), 12, [(0, .21, .155), (1, .21, .155)], "metal", GOLD),
        box_part("chest", sp((0, -0.13, 1.52)), (0.07, 0.03, 0.07), "emissive", SUN, rot=(0, 45, 0)),
        box_part("chest", sp((0.06, -0.12, 1.36)), (0.03, 0.02, 0.18), "cloth", SKY, rot=(0, -5, 0)),
        box_part("chest", sp((-0.06, -0.12, 1.36)), (0.03, 0.02, 0.18), "cloth", SKY, rot=(0, 5, 0)),
        box_part("chest", sp((0.06, -0.125, 1.26)), (0.035, 0.025, 0.035), "metal", GOLD),
        box_part("chest", sp((-0.06, -0.125, 1.26)), (0.035, 0.025, 0.035), "metal", GOLD),
        path_part("pelvis", [sp((0, 0.01, 0.80)), sp((0, 0.02, 0.62)), sp((0, 0.025, 0.42))], 12, [(.22, .19), (.28, .25), (.34, .30)], "cloth", WHITE),
        path_part("pelvis", [sp((0, 0.025, 0.44)), sp((0, 0.025, 0.41))], 12, [(.34, .30), (.345, .305)], "cloth", SKY),
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .048, .048), (1, .044, .044)], "skin", SKIN),
    ]


def _halo():
    # The halo: a broad ring standing behind her head, its spokes to a gold boss at her nape.
    c = Vector(sp((0, 0.20, 1.86)))
    r = 0.30
    pts = [tuple(c + Vector((math.cos(a) * r, 0.0, math.sin(a) * r))) for a in [2 * math.pi * k / 16 for k in range(17)]]
    inner = [tuple(c + Vector((math.cos(a) * (r - 0.06), 0.01, math.sin(a) * (r - 0.06)))) for a in [2 * math.pi * k / 16 for k in range(17)]]
    mid = [tuple(c + Vector((math.cos(a) * (r - 0.11), 0.02, math.sin(a) * (r - 0.11)))) for a in [2 * math.pi * k / 16 for k in range(17)]]
    out = [path_part("chest", pts, 4, [(.04, .022)] * 17, "emissive", SUN, ref=(0, 1, 0)),
           path_part("chest", mid, 4, [(.016, .012)] * 17, "emissive", SUN_WHITE, ref=(0, 1, 0)),
           path_part("chest", inner, 4, [(.012, .012)] * 17, "metal", GOLD, ref=(0, 1, 0))]
    out.append(path_part("chest", [sp((0, 0.12, 1.62)), sp((0, 0.19, 1.62 + 0.04)), sp((0, 0.20, 1.86 - r))], 6, [(.03, .03), (.025, .025), (.02, .02)], "metal", GOLD))
    return out


def _circlet():
    return [path_part("head", [sp((0, 0.0, 1.80)), sp((0, -0.001, 1.825))], 12, [(.105, .118), (.105, .118)], "metal", GOLD),
            box_part("head", sp((0, -0.115, 1.83)), (0.04, 0.02, 0.05), "emissive", SUN, rot=(0, 45, 0))]


def _head(bm, layers, groups, mats):
    head.build(bm, layers, groups, mats, head.Head(
        base=sp((0, -0.008, 1.63)), height=0.25, scale=1.08, jaw=0.5, chin=0.82, eye=1.2,
        skin=SKIN, hair=HAIR, hair_style="ponytail", eye_color=(0.3, 0.42, 0.6), lips=(0.72, 0.42, 0.42)))


def _left():
    ua, fa = "upperarm_l", "forearm_l"
    return [
        (ua, (ua, 0), (ua, 1), 10, [(0, .058, .058), (1, .05, .05)], "cloth", WHITE),
        # Wide sleeves hanging from the forearm, gold-hemmed.
        (fa, (fa, 0), (fa, 1.0), 10, [(0, .055, .055), (.5, .075, .075), (1, .11, .10)], "cloth", WHITE),
        (fa, (fa, 0.97), (fa, 1.01), 10, [(0, .113, .103), (1, .113, .103)], "cloth", SKY),
        (fa, (fa, 0.55), (fa, 0.98), 12, [(0, .07, .068), (.5, .085, .082), (1, .098, .09)], "cloth", IVORY),
        box_part(fa, (fa, 0.35, (0.0, -0.05, 0.0)), (0.05, 0.03, 0.05), "metal", GOLD, rot=(0, 0, 45), bevel=0.3),
        box_part("upperarm_l", ("upperarm_l", 0.15, (0.03, 0, 0.0)), (0.05, 0.05, 0.05), "metal", GOLD, rot=(0, 45, 0), bevel=0.3),
        *champion.hand_parts("l", SKIN, "skin", scale=1.25),
        # Legs under the robe (seen only in a stride), soft slippers.
        ("thigh_l", sp((0.10, 0, 1.02)), ("thigh_l", 1), 8, [(0, .08, .08), (1, .056, .056)], "cloth", IVORY),
        ("calf_l", ("calf_l", 0), ("calf_l", 1), 8, [(0, .055, .055), (1, .045, .045)], "cloth", IVORY),
        box_part("foot_l", ("foot_l", 0.45), (0.08, 0.15, 0.07), "cloth", GOLD, bevel=0.3),
    ]


def _staff():
    """The lantern-staff, skinned to the left hand and laid along it."""
    h, t = Vector(BONES["hand_l"][2]), Vector(BONES["hand_l"][3])
    axis = (t - h).normalized()

    def p(d, off=(0, 0, 0)):
        return tuple(h + axis * d + Vector(off))

    hb = "hand_l"
    top = h + axis * STAFF_TOP
    side = axis.cross(Vector((0, 1, 0))).normalized()
    out = [
        path_part(hb, [p(STAFF_FOOT), p(STAFF_TOP - 0.12)], 6, [(.026, .026), (.03, .03)], "cloth", WOOD),
        path_part(hb, [p(STAFF_FOOT - 0.05), p(STAFF_FOOT + 0.04)], 6, [(.0, .0), (.035, .035)], "metal", GOLD),
        path_part(hb, [p(GRIP - 0.08), p(GRIP + 0.10)], 6, [(.034, .034), (.034, .034)], "metal", GOLD),
        # The lantern: a gold cage around a sun-bright core, crowned by a crescent.
        path_part(hb, [p(STAFF_TOP - 0.14), p(STAFF_TOP - 0.10), p(STAFF_TOP + 0.08), p(STAFF_TOP + 0.12)], 6,
                  [(.03, .03), (.075, .075), (.075, .075), (.03, .03)], "metal", GOLD),
        path_part(hb, [p(STAFF_TOP - 0.08), p(STAFF_TOP), p(STAFF_TOP + 0.07)], 6, [(.06, .06), (.085, .085), (.05, .05)], "emissive", SUN_WHITE),
        # Cage bars around the light, and carved gold rings down the staff.
        *[path_part(hb, [tuple(h + axis * (STAFF_TOP - 0.10) + v * 0.09), tuple(h + axis * (STAFF_TOP + 0.08) + v * 0.09)], 5,
                    [(.008, .008), (.008, .008)], "metal", GOLD)
          for v in (side, -side, side.cross(axis).normalized(), -side.cross(axis).normalized())],
        *[path_part(hb, [p(d), p(d + 0.03)], 8, [(.036, .036), (.036, .036)], "metal", GOLD) for d in (GRIP + 0.30, GRIP + 0.55, GRIP + 0.80)],
        path_part(hb, [tuple(top + axis * 0.12 + side * 0.10), tuple(top + axis * 0.22 + side * 0.06), tuple(top + axis * 0.26),
                       tuple(top + axis * 0.22 - side * 0.06), tuple(top + axis * 0.12 - side * 0.10)], 4,
                  [(0, 0), (.025, .015), (.03, .015), (.025, .015), (0, 0)], "metal", GOLD),
    ]
    return out


def parts():
    return _center() + champion.mirrored(_left()) + _halo() + _staff() + _circlet() + [_head]


# --- Poses ----------------------------------------------------------------------------------

FLOAT = 0.10  # she hovers this far up; the robe still brushes the ground


def staff(top, foot=None, pole=(1.0, -0.2, -0.8)):
    """The staff's lantern at `top` (armature space), its foot toward `foot` (default: straight
    below); the left hand on it at GRIP."""
    tp = Vector(top)
    ft = Vector(foot) if foot else tp - Vector((0, 0, 1))
    axis = (tp - ft).normalized()
    wrist = tp - axis * STAFF_TOP
    return {"ik_hand_l": ik(tuple(wrist), aim=tuple(axis), pole=pole)}


LEGS = merge({"thigh_l": (-6, 0, 0), "thigh_r": (4, 0, 0), "calf_l": (16, 0, 0), "calf_r": (14, 0, 0), "foot_l": (30, 0, 0), "foot_r": (30, 0, 0)})
IDLE = merge(LEGS, staff(at(0.42, 0.20, 2.20, "l")), arm("r", 0.35, 0.35, -0.8, 0.25, 0.8, -0.2), {"head": (-4, 0, 0)})
READY = merge(LEGS, staff(at(0.40, 0.35, 2.15, "l")), arm("r", 0.4, 0.6, -0.4, 0.2, 0.9, 0.1), {"spine_01": (4, 0, 0), "head": (-6, 0, 0)})
UP = {"pelvis": (0, 0, FLOAT)}


def idle(armature):
    b = merge(IDLE, sym("clavicle", 0, -3, 0), {"head": (-6, 0, 4)}, staff(at(0.42, 0.20, 2.22, "l")))
    return _clip(armature, "idle", 80, [(0, IDLE, {"pelvis": (0, 0, FLOAT)}), (40, b, {"pelvis": (0, 0, FLOAT + 0.04)})], loop=True)


def idle_fidget_1(armature):
    # Cups a mote of light in her free hand and lets it drift up.
    cup = merge(IDLE, hand("r", at(0.12, 0.30, 1.30, "r"), aim=at(-0.4, 0.6, 0.6, "r")), {"head": (12, 0, -10)})
    up = merge(IDLE, arm("r", 0.3, 0.5, 0.4, 0.1, 0.4, 0.9), {"head": (-20, 0, -6)})
    keys = [(0, IDLE, UP), (14, cup, UP), (30, cup, {"pelvis": (0, 0, FLOAT + 0.02)}), (40, up, {"pelvis": (0, 0, FLOAT + 0.04)}), (60, IDLE, UP)]
    return _clip(armature, "idle_fidget_1", 60, keys)


def idle_fidget_2(armature):
    # Bows her head, eyes closed, a slow breath.
    bow = merge(IDLE, {"head": (24, 0, 0), "neck": (8, 0, 0), "spine_02": (6, 0, 0)})
    keys = [(0, IDLE, UP), (16, bow, {"pelvis": (0, 0, FLOAT - 0.03)}), (36, bow, {"pelvis": (0, 0, FLOAT - 0.02)}), (52, IDLE, UP)]
    return _clip(armature, "idle_fidget_2", 52, keys)


def idle_ready(armature):
    keys = [(0, READY, UP), (20, merge(READY, {"spine_01": (6, 0, 0)}), {"pelvis": (0, 0, FLOAT + 0.03)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _glide(frames, lean, trail):
    # The glide (14 §3.6): no footfalls, a slow hover bob, leaning into it, the free arm and
    # the staff trailing.
    keys = []
    for f in range(0, frames, 2):
        p = f / frames
        bob = 0.035 * math.sin(2 * math.pi * p)
        pose = merge({"thigh_l": (-10, 0, 0), "thigh_r": (14, 0, 0), "calf_l": (20, 0, 0), "calf_r": (34, 0, 0), "foot_l": (36, 0, 0), "foot_r": (40, 0, 0)},
                     staff(at(0.40, -0.10 - trail * 0.2, 2.10 + bob, "l"), at(0.40, 0.20 + trail * 0.2, 0.9 + bob, "l")),
                     arm("r", 0.5, -0.5 - trail * 0.3, -0.7, 0.3, -0.3, -0.6),
                     {"spine_01": (lean, 0, 0), "spine_02": (lean * 0.3, 0, 4 * math.sin(2 * math.pi * p)), "head": (-lean * 0.8, 0, 0)})
        keys.append((f, pose, {"pelvis": (0, 0, FLOAT + bob)}))
    return keys


def run(armature):
    return _clip(armature, "run", 20, _glide(20, 12, 0.3), loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    return _clip(armature, "run_fast", 16, _glide(16, 18, 0.6), loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(armature):
    # The lantern swung forward to point at the target: drawn back, cocked, a one-frame thrust
    # on `fire` (sim: 9) that sends the mote, held.
    back = merge(READY, staff(at(0.30, -0.25, 2.25, "l"), at(0.40, 0.20, 1.0, "l")), {"spine_01": (-4, 0, 12), "head": (-8, 0, -10)})
    cock = merge(back, staff(at(0.25, -0.40, 2.15, "l"), at(0.40, 0.25, 1.05, "l")), {"spine_01": (-6, 0, 16)})
    thrust = merge(READY, staff(at(0.10, 1.25, 1.75, "l"), at(0.30, 0.0, 1.10, "l"), pole=(1.0, -0.4, -0.6)), {"spine_01": (10, 0, -10), "head": (-10, 0, 8)},
                   arm("r", 0.5, -0.3, -0.5, 0.3, -0.1, -0.4))
    keys = [(0, READY, UP), (5, back, UP), (8, cock, UP, "snap"), (9, thrust, {"pelvis": (0, -0.04, FLOAT - 0.02)}),
            (14, merge(thrust, {"spine_01": (12, 0, -12)}), {"pelvis": (0, -0.04, FLOAT - 0.02)}, "settle"), (24, READY, UP)]
    return _clip(armature, "attack_1", 24, keys, markers={"fire": 9})


def attack_2(armature):
    # A mote pushed from her open palm: drawn in to her shoulder, cocked, pushed out on `fire`.
    draw = merge(READY, hand("r", at(0.18, -0.05, 1.50, "r"), aim=at(0.0, 0.6, 0.6, "r")), {"spine_01": (0, 0, -14), "head": (-6, 0, 12)})
    cock = merge(draw, hand("r", at(0.22, -0.15, 1.52, "r"), aim=at(0.1, 0.4, 0.8, "r")), {"spine_01": (0, 0, -18)})
    push = merge(READY, arm("r", 0.1, 1, 0.1, 0.02, 1, 0.12), {"spine_01": (8, 0, 12), "head": (-8, 0, -8)})
    keys = [(0, READY, UP), (5, draw, UP), (8, cock, UP, "snap"), (9, push, {"pelvis": (0, -0.03, FLOAT - 0.02)}),
            (14, merge(push, {"spine_01": (10, 0, 14)}), {"pelvis": (0, -0.03, FLOAT - 0.02)}, "settle"), (24, READY, UP)]
    return _clip(armature, "attack_2", 24, keys, markers={"fire": 9})


def q(armature):
    # Mending Light (upper body, a pulse): the free hand raised, palm up, light offered on
    # `fire` 4, held a moment.
    offer = merge(staff(at(0.40, 0.30, 2.20, "l")), arm("r", 0.3, 0.7, 0.3, 0.1, 0.8, 0.55), {"head": (-10, 0, 0)})
    keys = [(0, merge(IDLE), None), (3, merge(offer, arm("r", 0.3, 0.6, 0.0, 0.1, 0.8, 0.2)), None, "snap"), (4, offer, None),
            (12, merge(offer, arm("r", 0.3, 0.7, 0.4, 0.1, 0.8, 0.65)), None, "settle"), (22, IDLE, None)]
    return _clip(armature, "q", 22, keys, layer="upper", markers={"fire": 4})


def w(armature):
    # Aegis (upper body, a pulse): the staff raised high and the free palm thrown forward on
    # `fire` 4, a ward pressed into place, held.
    ward = merge(staff(at(0.30, 0.40, 2.45, "l")), arm("r", 0.1, 1, 0.25, 0.0, 1, 0.3), {"head": (-8, 0, 0), "spine_02": (-4, 0, 0)})
    keys = [(0, IDLE, None), (3, merge(ward, arm("r", 0.2, 0.6, 0.2, -0.4, 0.4, 0.6)), None, "snap"), (4, ward, None),
            (12, merge(ward, {"spine_02": (-6, 0, 0)}), None, "settle"), (22, IDLE, None)]
    return _clip(armature, "w", 22, keys, layer="upper", markers={"fire": 4})


def e(armature):
    # Lull: a slow, wide sweep of her free arm across the front, quickening into the release
    # on `fire` 8, the arm held out at the end.
    start = merge(READY, arm("r", 0.9, -0.3, -0.1, 0.9, -0.2, 0.0), {"spine_01": (2, 0, -16), "head": (-6, 0, 14)})
    end = merge(READY, arm("r", -0.3, 0.95, -0.05, -0.4, 0.9, -0.05), {"spine_01": (6, 0, 18), "head": (-8, 0, -12)})
    keys = [(0, READY, UP), (5, start, UP, "whip"), (8, end, {"pelvis": (0, -0.03, FLOAT - 0.02)}), (14, merge(end, {"spine_01": (8, 0, 22)}), UP, "settle"), (22, READY, UP)]
    return _clip(armature, "e", 22, keys, markers={"fire": 8})


def r(armature):
    # Binding Halo: the staff lifted high in both hands and held as the halo flares, then driven
    # down, its foot planted, on `fire` 8; she holds it there.
    high = merge(LEGS, staff(at(0.15, 0.30, 2.85, "l")), hand("r", at(0.05, 0.28, 2.0, "r"), aim=at(0.4, 0.2, 1, "r")), {"spine_02": (-10, 0, 0), "head": (-18, 0, 0)})
    plant = merge(LEGS, staff(at(0.15, 0.55, 2.05, "l"), at(0.15, 0.62, 0.0, "l"), pole=(1.0, -0.2, -0.6)),
                  hand("r", at(0.08, 0.50, 1.35, "r"), aim=at(-0.3, 0.3, -1, "r")), {"spine_01": (14, 0, 0), "head": (-12, 0, 0)})
    keys = [(0, READY, UP), (5, high, {"pelvis": (0, 0, FLOAT + 0.08)}), (7, merge(high, {"spine_02": (-14, 0, 0)}), {"pelvis": (0, 0, FLOAT + 0.10)}, "snap"),
            (8, plant, {"pelvis": (0, -0.02, FLOAT - 0.06)}), (16, merge(plant, {"spine_01": (16, 0, 0)}), {"pelvis": (0, -0.02, FLOAT - 0.07)}, "settle"), (26, READY, UP)]
    return _clip(armature, "r", 26, keys, markers={"fire": 8})


KNEEL = merge({"thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0), "foot_l": (0, 0, 0), "thigh_r": (6, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
              staff(at(0.25, 0.30, 1.75, "l"), at(0.25, 0.32, 0.0, "l")), hand("r", at(0.18, 0.28, 1.15, "l"), aim=at(-0.3, 0.3, -1, "l")),
              {"spine_01": (6, 0, 0), "head": (22, 0, 0)})


def recall(armature):
    keys = [(0, IDLE, UP), (15, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (45, merge(KNEEL, {"head": (26, 0, 0)}), {"pelvis": (0, 0.04, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (90, IDLE, UP)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    # Her light fails: she sinks, kneels, and slumps sideways onto the staff.
    sink = merge(LEGS, staff(at(0.40, 0.30, 1.90, "l")), arm("r", 0.5, 0.2, -0.7, 0.4, 0.3, -0.6), {"spine_01": (14, 0, 0), "head": (30, 0, 0)})
    down = merge({"thigh_l": (-20, 0, 0), "thigh_r": (-10, 0, 0), "calf_l": (60, 0, 0), "calf_r": (40, 0, 0)}, arm("l", 0.9, 0.3, 0.1, 0.9, 0.3, 0.0),
                 arm("r", 0.6, 0.4, 0.4, 0.4, 0.6, 0.4), {"pelvis": (-70, -16, 0), "head": (20, 0, 16)})
    keys = [(0, IDLE, UP), (10, sink, {"pelvis": (0, 0, -0.10)}), (18, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "snap"),
            (28, down, {"pelvis": (0, 0.42, -0.84)}, "snap"), (36, down, {"pelvis": (0, 0.44, -0.85)})]
    return _clip(armature, "death", 36, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "settle"), (14, merge(IDLE, {"head": (-10, 0, 0)}), {"pelvis": (0, 0, FLOAT + 0.06)}), (26, IDLE, UP)]
    return _clip(armature, "respawn", 26, keys)


def select(armature):
    # Rises a little higher, the staff lifted, the halo blazing, her free arm opening wide.
    up = merge(LEGS, staff(at(0.30, 0.25, 2.70, "l")), arm("r", 1, 0.3, 0.35, 1, 0.3, 0.4), {"head": (-16, 0, 0), "spine_02": (-6, 0, 0)})
    keys = [(0, IDLE, UP), (16, up, {"pelvis": (0, 0, FLOAT + 0.12)}), (36, merge(up, {"head": (-20, 0, 6)}), {"pelvis": (0, 0, FLOAT + 0.14)}), (52, IDLE, UP)]
    return _clip(armature, "select", 52, keys)


def emote_taunt(armature):
    # Wags a finger, gently disapproving.
    f = merge(IDLE, arm("r", 0.2, 0.7, 0.3, -0.3, 0.3, 0.9), {"head": (-6, 0, 8)})
    keys = [(0, IDLE, UP), (8, f, UP)]
    for i, fr in enumerate(range(12, 30, 4)):
        keys.append((fr, merge(f, arm("r", 0.2, 0.7, 0.3, -0.3 + (0.2 if i % 2 else -0.2), 0.3, 0.9)), UP))
    keys.append((40, IDLE, UP))
    return _clip(armature, "emote_taunt", 40, keys)


def emote_joke(armature):
    # Uses her halo as a hand mirror, tidying her hair.
    mirror = merge(IDLE, arm("r", 0.6, 0.5, 0.6, -0.6, 0.0, 0.8), {"head": (-6, 0, 30)})
    keys = [(0, IDLE, UP), (12, mirror, UP), (20, merge(mirror, {"head": (-6, 0, 20)}), UP), (28, mirror, UP), (44, IDLE, UP)]
    return _clip(armature, "emote_joke", 44, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, UP)]
    for i in range(1, 7):
        s = 1.0 if i % 2 else 0.4
        keys.append((5 * i, merge(IDLE, arm("r", 0.15, 0.6, -0.1, -0.5, 0.2, 0.85), {"head": (8 * s, 0, 0)}, sym("clavicle", 0, -3 * s, 0)), UP))
    keys.append((40, IDLE, UP))
    return _clip(armature, "emote_laugh", 40, keys)


def emote_dance(armature):
    # A slow floating turn, the staff swaying overhead.
    keys = []
    for i, f in enumerate(range(0, 48, 6)):
        a = 2 * math.pi * i / 8
        keys.append((f, merge(LEGS, staff(at(0.3 + 0.15 * math.cos(a), 0.2 + 0.15 * math.sin(a), 2.5, "l")), arm("r", 0.9, 0.2 * math.cos(a), 0.4, 0.9, 0.3, 0.4),
                              {"pelvis": (0, 0, 20 * math.sin(a)), "head": (-10, 0, -10 * math.sin(a))}), {"pelvis": (0, 0, FLOAT + 0.04 + 0.03 * math.sin(a))}))
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def build(only=None):
    return champion.build("lumen", SHAPE, parts, OWN, 1.85, only=only, shared=champion.SHARED + ["attack_melee_alt"])


if __name__ == "__main__":
    champion.main(build)
