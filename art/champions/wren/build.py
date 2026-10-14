# SPDX-License-Identifier: AGPL-3.0-or-later
"""Wren (rebuilt for 14): the generator for her model and clips.

    blender -b --python art/champions/wren/build.py -- --out art/champions/wren/wren.blend

A feather crest and a short cape of feathers (14 §5): a small, quick scout with a tall crest of
lime-tipped feathers swept back from her head, a capelet of layered feathers on an `extra_cape`
chain, a fan of tail feathers at her belt, a russet tunic and a hand crossbow in her right hand.
Her signature is lime (14 §4).

Weight (14 §3): she's light and restless: a springing run, bird-quick head flicks at idle; each
shot kicks the crossbow up and she holds the recoil a beat; Pounce lands her on all fours and she
stays there a moment before springing up. Feet (and, for Pounce, hands) planted by IK.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import champion, clips, head, mesh, rig  # noqa: E402
from mftr_blender.champion import arm, feet, hand  # noqa: E402
from mftr_blender.clips import at, grip, ik, merge, sym  # noqa: E402
from mftr_blender.library import _clip, gait  # noqa: E402
from mftr_blender.mesh import box_part, path_part  # noqa: E402

SKIN = (0.80, 0.60, 0.46)
RUSSET = (0.44, 0.17, 0.07)
TAN = (0.54, 0.40, 0.24)
LEATHER = (0.28, 0.17, 0.09)
CREAM = (0.68, 0.62, 0.48)
DARK = (0.14, 0.09, 0.05)
WOOD = (0.32, 0.20, 0.10)
IRON = (0.34, 0.35, 0.38)
HAIR = (0.44, 0.20, 0.10)
LIME = (0.66, 1.0, 0.20)
LIME_CLOTH = (0.30, 0.46, 0.08)
ACCENT = (0.18, 0.52, 0.95)

# Small and springy (14 §2.2): short, a big heroic head, big hands.
SHAPE = {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 1.0, "y": 1.0, "scale": 0.88, "shoulder": 0.02, "hand": 1.35}
rig.use_shape(SHAPE)
BONES = {b[0]: b for b in rig.bone_table()}
CAPE = [rig.shaped(p) for p in [(0, 0.12, 1.53), (0, 0.18, 1.30), (0, 0.21, 1.08)]]
HAND_LEN = (Vector(BONES["hand_r"][3]) - Vector(BONES["hand_r"][2])).length
GRIP = 0.45 * HAND_LEN


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _feather(bone, base, d, length, width, color, tip=None):
    """A feather: a flat blade from `base` along `d`, its last third in `tip` color."""
    d = Vector(d).normalized()
    b = mesh._resolve(base)
    pts = [tuple(b), tuple(b + d * length * 0.35), tuple(b + d * length * 0.7), tuple(b + d * length)]
    cols = [color, color, tip or color, tip or color]
    return path_part(bone, pts, 4, [(width * 0.5, width * 0.12), (width, width * 0.15), (width * 0.85, width * 0.12), (0, 0)], "cloth", color,
                     ref=(1, 0, 0) if abs(d.x) < 0.9 else (0, 1, 0), colors=cols)


def _center():
    return [
        ("pelvis", sp((0, 0, 0.92)), sp((0, 0, 1.12)), 10, [(0, .13, .10), (.5, .15, .11), (1, .14, .105)], "cloth", TAN),
        ("pelvis", sp((0, 0, 1.04)), sp((0, 0, 1.10)), 10, [(0, .152, .112), (1, .148, .108)], "accent", ACCENT),
        # A russet tunic to mid-thigh, lime-hemmed.
        path_part("pelvis", [sp((0, 0, 1.06)), sp((0, 0.0, 0.88)), sp((0, 0.01, 0.74))], 12, [(.15, .115), (.18, .14), (.20, .16)], "cloth", RUSSET),
        path_part("pelvis", [sp((0, 0.01, 0.76)), sp((0, 0.01, 0.73))], 12, [(.20, .16), (.205, .165)], "cloth", LIME_CLOTH),
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 10, [(0, .118, .088), (1, .122, .09)], "cloth", RUSSET),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 10, [(0, .122, .09), (1, .138, .098)], "cloth", RUSSET),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.60)), 10, [(0, .14, .10), (.5, .145, .104), (1, .085, .072)], "cloth", TAN),
        # A leather jerkin with a lime-stitched lace.
        box_part("chest", sp((0, -0.10, 1.40)), (0.18, 0.04, 0.26), "cloth", LEATHER, taper=1.15, bevel=0.3),
        box_part("chest", sp((0, -0.125, 1.40)), (0.02, 0.02, 0.22), "cloth", LIME_CLOTH),
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .045, .045), (1, .042, .042)], "skin", SKIN),
        # Belt pouches, and a quiver of bolts at her right thigh.
        box_part("pelvis", sp((0.15, -0.06, 1.02)), (0.07, 0.06, 0.08), "cloth", LEATHER, rot=(0, 0, 25), bevel=0.3),
        box_part("pelvis", sp((0.04, -0.13, 1.03)), (0.06, 0.05, 0.07), "cloth", LEATHER, bevel=0.3),
        box_part("pelvis", sp((-0.05, -0.13, 1.06)), (0.04, 0.02, 0.04), "metal", IRON, rot=(0, 45, 0), bevel=0.3),
        path_part("pelvis", [sp((-0.17, 0.03, 0.78)), sp((-0.18, 0.06, 1.02))], 8, [(.04, .035), (.045, .04)], "cloth", LEATHER),
        *[path_part("pelvis", [sp((-0.17 + dx, 0.06 + dy, 1.02)), sp((-0.18 + dx, 0.07 + dy, 1.10))], 4, [(.012, .004), (0, 0)], "emissive", LIME)
          for dx, dy in ((0.015, 0.0), (-0.015, 0.01), (0.0, -0.015))],
        # A fan of tail feathers at the back of her belt.
        *[_feather("pelvis", sp((x * 0.3, 0.13, 1.02)), (x, 0.55, -0.6), 0.30 - abs(x) * 0.12, 0.06, RUSSET, LIME) for x in (-0.35, -0.12, 0.12, 0.35)],
        # The capelet: layered feathers over her shoulders and down her back, on the chain.
        *[_feather("chest", sp((x, 0.04, 1.58)), (x * 1.6, 0.25, -0.5), 0.22, 0.07, TAN if k % 2 else RUSSET, LIME) for k, x in enumerate((-0.15, -0.08, 0.0, 0.08, 0.15))],
        *[_feather("extra_cape_1", (CAPE[0][0] + x, CAPE[0][1] + 0.01, CAPE[0][2] - 0.04), (x * 1.5, 0.30, -1), 0.26, 0.075, RUSSET if k % 2 else TAN, LIME)
          for k, x in enumerate((-0.12, -0.04, 0.04, 0.12))],
        *[_feather("extra_cape_2", (CAPE[1][0] + x, CAPE[1][1] + 0.01, CAPE[1][2] - 0.02), (x * 1.5, 0.25, -1), 0.24, 0.07, TAN if k % 2 else RUSSET, LIME)
          for k, x in enumerate((-0.10, -0.03, 0.03, 0.10))],
    ]


def _crest():
    # A tall crest of feathers swept back from her brow, lime at the tips.
    out = []
    for k, (x, ang, L) in enumerate(((0.0, 60, 0.38), (0.04, 52, 0.32), (-0.04, 52, 0.32), (0.075, 40, 0.26), (-0.075, 40, 0.26), (0.0, 72, 0.30),
                                     (0.10, 28, 0.20), (-0.10, 28, 0.20))):
        a = math.radians(ang)
        out.append(_feather("head", sp((x, -0.05, 1.84)), (x * 1.2, math.cos(a) * 0.9, math.sin(a)), L, 0.055, RUSSET if k % 2 else HAIR, LIME))
    return out


def _head(bm, layers, groups, mats):
    head.build(bm, layers, groups, mats, head.Head(
        base=sp((0, -0.008, 1.62)), height=0.25, scale=1.2, jaw=0.5, chin=0.82, eye=1.3,
        skin=SKIN, hair=HAIR, hair_style="short", eye_color=(0.36, 0.5, 0.14), lips=(0.72, 0.40, 0.38)))


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 10, [(0, .052, .052), (1, .046, .046)], "cloth", CREAM),
        (fa, (fa, 0), (fa, 1), 10, [(0, .045, .045), (1, .04, .04)], "cloth", CREAM),
        (fa, (fa, .35), (fa, .98), 10, [(0, .054, .054), (1, .048, .048)], "cloth", LEATHER),
        (fa, (fa, .60), (fa, .66), 10, [(0, .057, .057), (1, .056, .056)], "cloth", LIME_CLOTH),
        _feather(fa, (fa, 0.75, (0.04, 0.02, 0.0)), (0.5, 0.6, -0.3), 0.14, 0.04, TAN, LIME),
        _feather(fa, (fa, 0.55, (0.04, 0.03, 0.0)), (0.6, 0.5, -0.4), 0.12, 0.04, RUSSET, LIME),
        *champion.hand_parts("l", LEATHER, "cloth", scale=1.3),
        (th, sp((0.10, 0, 1.02)), (th, 1), 10, [(0, .085, .082), (.5, .072, .072), (1, .056, .056)], "cloth", DARK),
        (ca, (ca, 0), (ca, .4), 10, [(0, .054, .054), (1, .05, .05)], "cloth", DARK),
        # Tall soft boots with feather tufts at the cuffs.
        (ca, (ca, .35), (ca, 1), 10, [(0, .066, .066), (.12, .062, .062), (1, .054, .056)], "cloth", LEATHER),
        (ca, (ca, .32), (ca, .38), 10, [(0, .072, .072), (1, .072, .072)], "cloth", TAN),
        _feather(ca, ("calf_l", 0.33), (0.6, 0.6, 0.5), 0.12, 0.04, RUSSET, LIME),
        box_part("foot_l", ("foot_l", 0.45), (0.08, 0.15, 0.07), "cloth", LEATHER, bevel=0.3),
        box_part(ca, (ca, 0.03, (0, -0.05, 0)), (0.08, 0.04, 0.09), "cloth", TAN, bevel=0.3),
        box_part(th, (th, 0.35, (0.07, -0.02, 0)), (0.06, 0.07, 0.09), "cloth", LEATHER, rot=(0, 0, 10), bevel=0.3),
        box_part("toe_l", ("toe_l", 0.35), (0.075, 0.07, 0.055), "cloth", LEATHER, bevel=0.3),
    ]


def _crossbow():
    """The hand crossbow, skinned to the right hand and laid along it (the fist aims it)."""
    h, t = Vector(BONES["hand_r"][2]), Vector(BONES["hand_r"][3])
    axis = (t - h).normalized()
    side = axis.cross(Vector((0, 1, 0))).normalized()
    up = side.cross(axis).normalized()

    def p(d, off=Vector()):
        return tuple(h + axis * d + off)

    hb = "hand_r"
    prod = h + axis * (GRIP + 0.34) + up * 0.035
    return [
        # The stock, from behind the fist forward, and a lime-tipped bolt on top.
        path_part(hb, [p(GRIP - 0.14, up * 0.02), p(GRIP + 0.02, up * 0.03), p(GRIP + 0.36, up * 0.035)], 6, [(.03, .04), (.028, .036), (.022, .03)], "cloth", WOOD, ref=tuple(side)),
        path_part(hb, [p(GRIP + 0.30, up * 0.035), p(GRIP + 0.38, up * 0.035)], 6, [(.03, .034), (.026, .03)], "metal", IRON, ref=tuple(side)),
        # A brass-and-iron crank on the side.
        path_part(hb, [p(GRIP + 0.10, up * 0.035 + side * 0.03), p(GRIP + 0.10, up * 0.035 + side * 0.08)], 8, [(.02, .02), (.02, .02)], "metal", IRON),
        path_part(hb, [p(GRIP + 0.10, up * 0.035 + side * 0.08), p(GRIP + 0.04, up * 0.08 + side * 0.08)], 5, [(.008, .008), (.008, .008)], "metal", IRON),
        path_part(hb, [p(GRIP + 0.0, up * 0.06), p(GRIP + 0.36, up * 0.06), p(GRIP + 0.40, up * 0.06)], 4, [(.006, .006), (.006, .006), (0, 0)], "emissive", LIME),
        # The prod: limbs swept back from the front, lime at the tips, and the string.
        path_part(hb, [tuple(prod - side * 0.27 - axis * 0.09), tuple(prod - side * 0.13 - axis * 0.01), tuple(prod), tuple(prod + side * 0.13 - axis * 0.01),
                       tuple(prod + side * 0.27 - axis * 0.09)], 6, [(.011, .011), (.018, .018), (.024, .024), (.018, .018), (.011, .011)], "metal", IRON,
                  colors=[LIME, IRON, IRON, IRON, LIME]),
        path_part(hb, [tuple(prod - side * 0.27 - axis * 0.09), tuple(h + axis * (GRIP + 0.08) + up * 0.05), tuple(prod + side * 0.27 - axis * 0.09)], 4,
                  [(.003, .003)] * 3, "cloth", CREAM),
    ]


def parts():
    return _center() + champion.mirrored(_left()) + _crest() + _crossbow() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def cape(back=0.0, sway=0.0):
    return {"extra_cape_1": (back * 0.5, sway * 0.5, 0), "extra_cape_2": (back * 0.35, sway * 0.35, 0)}


def aim_bow(target, pole=(1.0, -0.4, -0.8), support=False):
    """The crossbow aimed along `target` (armature-space direction) from her right shoulder; the
    left hand under its stock when `support`."""
    d = Vector(target).normalized()
    shoulder = Vector(BONES["upperarm_r"][2])
    wrist = shoulder + d * 0.48 + Vector((0.06, 0, -0.04))
    out = {"ik_hand_r": ik(tuple(wrist), aim=tuple(d), pole=pole)}
    if support:
        out["grip_l"] = grip("hand_r", at=GRIP + 0.05, pole=(1.0, -0.2, -0.8))
    return out


STAND = feet((0.11, 0.06), (0.13, -0.08))
# Weight on one leg, the crossbow propped on her shoulder, a hand on her hip.
IDLE = merge(STAND, {"pelvis": (0, -5, 0), "spine_01": (0, 4, 0)}, arm("r", 0.3, 0.2, -0.8, -0.25, 0.75, 0.6), {"hand_r": clips.aim(-0.1, -0.3, 1, "r")},
             arm("l", 0.5, -0.1, -0.6, -0.4, 0.4, 0.3), {"head": (-4, 0, 0)}, cape(2))
READY = merge(feet((0.15, 0.16), (0.15, -0.14)), {"spine_01": (12, 0, 0), "head": (-8, 0, 0)}, aim_bow((0, -1, -0.25), support=True), cape(4))
DOWN = {"pelvis": (0, 0, -0.08)}


def idle(armature):
    # Bird-quick: still, then a flick of the head, still again.
    keys = [(0, IDLE, None), (14, IDLE, None, "snap"), (16, merge(IDLE, {"head": (-4, 0, 30)}), None), (30, merge(IDLE, {"head": (-4, 0, 30)}), None, "snap"),
            (32, merge(IDLE, {"head": (-12, 0, -18)}), None), (46, merge(IDLE, {"head": (-12, 0, -18)}), None, "snap"), (48, IDLE, None)]
    return _clip(armature, "idle", 64, keys, loop=True)


def idle_fidget_1(armature):
    # Spins the crossbow round her finger and catches it on her shoulder.
    out = merge(IDLE, arm("r", 0.6, 0.6, -0.1, 0.5, 0.8, 0.0))
    keys = [(0, IDLE, None), (8, out, None)]
    for i, f in enumerate((11, 14, 17, 20)):
        keys.append((f, merge(out, {"hand_r": clips.aim(*((0.3, 1, 0) if i % 2 else (0.3, -0.3, -1)), "r")}), None, "snap"))
    keys += [(26, IDLE, None, "snap"), (38, IDLE, None)]
    return _clip(armature, "idle_fidget_1", 38, keys)


def idle_fidget_2(armature):
    # Preens a crest feather back into place, then shakes out her capelet.
    preen = merge(IDLE, arm("l", 0.3, 0.3, 0.6, -0.4, 0.0, 0.9), {"head": (-18, 0, 12)})
    keys = [(0, IDLE, None), (12, preen, None), (22, merge(preen, {"head": (-22, 0, 8)}), None), (28, merge(IDLE, cape(10, 6)), None, "snap"),
            (30, merge(IDLE, cape(10, -6)), None, "snap"), (32, merge(IDLE, cape(6, 4)), None, "snap"), (44, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 44, keys)


def idle_ready(armature):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (15, 0, 0)}, cape(6, 2)), {"pelvis": (0, 0, -0.10)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(swing, trail):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(arm("r", 0.3, 0.3 + 0.25 * swing * c, -0.75, 0.1, 0.9, -0.2), {"hand_r": clips.aim(0.0, 1, -0.3, "r")},
                     arm("l", 0.3, -0.45 * swing * c, -0.8, 0.2, 0.55 - 0.35 * swing * c, -0.3), cape(trail + 6 * math.sin(4 * math.pi * p)), {"head": (-8, 0, 0)})
    return upper


def run(armature):
    # Light (14 §3.5): short contact, long airborne strides.
    keys = gait(20, 0.75, contact=0.34, lift=0.26, bob=0.06, lean=12, twist=10, width=0.11, upper=_run_upper(1.0, 30))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    keys = gait(16, 0.82, contact=0.33, lift=0.30, bob=0.065, lean=17, twist=12, width=0.11, upper=_run_upper(1.3, 42))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _shot(armature, name, frames, fire, flat=False):
    """A crossbow shot: raised and aimed (the left hand under the stock), steadied, fired on
    `fire` with the bow kicking up, the recoil held."""
    st = feet((0.15, 0.22), (0.15, -0.18))
    aimed = merge(st, aim_bow((0, -1, 0.0), support=True), {"spine_01": (6, 0, -10), "head": (-6, 0, 10)}, cape(3))
    if flat:
        aimed = merge(aimed, {"spine_01": (8, 0, -24), "head": (-8, 0, 22)})
    kick = merge(st, aim_bow((0, -1, 0.45), support=True), {"spine_01": (-2, 0, -10), "head": (-10, 0, 8)}, cape(6))
    keys = [(0, READY, DOWN), (fire - 3, aimed, DOWN), (fire - 1, aimed, DOWN, "snap"), (fire, kick, {"pelvis": (0, 0.03, -0.07)}),
            (fire + 4, merge(kick, {"spine_01": (0, 0, -10)}), {"pelvis": (0, 0.03, -0.07)}, "settle"), (frames, READY, DOWN)]
    return _clip(armature, name, frames, keys, markers={"fire": fire})


def attack_1(armature):
    return _shot(armature, "attack_1", 22, 8)


def attack_2(armature):
    return _shot(armature, "attack_2", 22, 8, flat=True)


def q(armature):
    # Ricochet: a trick shot, the crossbow turned flat and fired from the hip on `fire` 8.
    st = feet((0.16, 0.24), (0.16, -0.18))
    hip = merge(st, aim_bow((0, -1, -0.1), pole=(1.0, 0.0, -1.0)), {"hand_r": None}, {"spine_01": (8, 0, -26), "head": (-8, 0, 24)},
                arm("l", 0.5, 0.2, -0.3, 0.4, 0.4, 0.2), cape(4))
    hip = {k: v for k, v in hip.items() if v is not None}
    kick = merge(hip, aim_bow((0.2, -1, 0.35), pole=(1.0, 0.0, -1.0)), {"spine_01": (2, 0, -22)})
    keys = [(0, READY, DOWN), (5, hip, DOWN), (7, hip, DOWN, "snap"), (8, kick, {"pelvis": (0, 0.03, -0.07)}),
            (13, merge(kick, {"head": (-12, 0, 20)}), {"pelvis": (0, 0.03, -0.07)}, "settle"), (24, READY, DOWN)]
    return _clip(armature, "q", 24, keys, markers={"fire": 8})


def w(armature):
    # Caltrops: a pouch scooped up and scattered underhand with the left hand on `fire` 6.
    st = feet((0.15, 0.22), (0.15, -0.18))
    scoop = merge(st, IDLE, arm("l", 0.4, -0.5, -0.7, 0.3, -0.2, -0.7), {"spine_01": (14, 0, 10)}, cape(3))
    toss = merge(st, IDLE, arm("l", 0.3, 0.85, 0.2, 0.2, 0.9, 0.35), {"spine_01": (6, 0, -10), "head": (-10, 0, -6)}, cape(6))
    keys = [(0, READY, DOWN), (4, scoop, {"pelvis": (0, 0, -0.10)}, "whip"), (6, toss, {"pelvis": (0, -0.03, -0.05)}),
            (11, merge(toss, {"spine_01": (8, 0, -12)}), {"pelvis": (0, -0.03, -0.05)}, "settle"), (20, READY, DOWN)]
    return _clip(armature, "w", 20, keys, markers={"fire": 6})


FOURS = merge(feet((0.15, 0.10), (0.15, -0.18)), hand("l", at(0.22, 0.48, 0.12, "l"), aim=at(0.1, 0.4, -1, "l")),
              aim_bow((0.2, -1, -0.9)), {"spine_01": (40, 0, 0), "spine_02": (16, 0, 0), "head": (-40, 0, 0)}, cape(10))


def e_start(armature):
    # Pounce: a low crouch, then the spring.
    coil = merge(FOURS, {"spine_01": (44, 0, 0)}, cape(4))
    spring = merge(arm("l", 0.3, 0.9, 0.0, 0.2, 1, 0.1), aim_bow((0, -1, -0.1)), sym("thigh", 10, 0, 0), sym("calf", 10, 0, 0), sym("foot", 30, 0, 0),
                   {"spine_01": (24, 0, 0), "head": (-24, 0, 0)}, cape(30))
    return _clip(armature, "e_start", 5, [(0, coil, {"pelvis": (0, 0, -0.42)}, "snap"), (2, spring, {"pelvis": (0, -0.1, -0.10)}), (5, spring, {"pelvis": (0, -0.1, 0.0)})],
                 markers={"fire": 2})


def e_travel(armature):
    air = merge(arm("l", 0.3, 0.9, 0.1, 0.2, 1, 0.0), aim_bow((0, -1, -0.2)), {"thigh_l": (-70, 0, 0), "thigh_r": (-40, 0, 0), "calf_l": (100, 0, 0), "calf_r": (80, 0, 0),
                                                                              "spine_01": (20, 0, 0), "head": (-22, 0, 0)}, cape(50))
    return _clip(armature, "e_travel", 10, [(0, air, {"pelvis": (0, 0, 0.0)}), (5, merge(air, cape(60)), {"pelvis": (0, 0, 0.04)})], loop=True)


def e_land(armature):
    # On all fours, held, then up.
    keys = [(0, FOURS, {"pelvis": (0, -0.04, -0.46)}, "settle"), (5, merge(FOURS, cape(14)), {"pelvis": (0, -0.04, -0.44)}), (12, READY, DOWN)]
    return _clip(armature, "e_land", 12, keys)


def r(armature):
    # Hail of Arrows: the crossbow aimed at the sky, fired on `fire` 8 and twice more, each kick
    # landing; held up, then down.
    st = feet((0.17, 0.20), (0.17, -0.18))
    sky = merge(st, aim_bow((0, -0.45, 1), pole=(1.0, -0.2, -0.6), support=True), {"spine_02": (-10, 0, 0), "head": (-30, 0, 0)}, cape(3))
    kick = merge(st, aim_bow((0, -0.15, 1), pole=(1.0, -0.2, -0.6), support=True), {"spine_02": (-14, 0, 0), "head": (-32, 0, 0)}, cape(6))
    keys = [(0, READY, DOWN), (5, sky, DOWN), (7, sky, DOWN, "snap"), (8, kick, {"pelvis": (0, 0.02, -0.10)}), (11, sky, DOWN, "snap"),
            (12, kick, {"pelvis": (0, 0.02, -0.10)}), (15, sky, DOWN, "snap"), (16, kick, {"pelvis": (0, 0.02, -0.10)}),
            (20, merge(kick, {"head": (-28, 0, 0)}), {"pelvis": (0, 0.02, -0.10)}, "settle"), (26, READY, DOWN)]
    return _clip(armature, "r", 26, keys, markers={"fire": 8})


KNEEL = merge({"ik_foot_l": ik(at(0.14, 0.26, champion.ankle(), "l")), "thigh_r": (8, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
              arm("r", 0.3, 0.4, -0.7, -0.2, 0.8, 0.2), arm("l", 0.3, 0.6, -0.5, -0.3, 0.6, 0.4), {"spine_01": (12, 0, 0), "head": (14, 0, 0)}, cape(-4))


def recall(armature):
    # Crouches and tinkers with her crossbow's string.
    keys = [(0, IDLE, None), (15, KNEEL, {"pelvis": (0, 0.04, -0.42)}), (30, merge(KNEEL, {"head": (18, 0, 8)}), {"pelvis": (0, 0.04, -0.41)}),
            (45, merge(KNEEL, arm("l", 0.3, 0.6, -0.4, -0.3, 0.6, 0.5)), {"pelvis": (0, 0.04, -0.41)}), (60, merge(KNEEL, {"head": (18, 0, -8)}), {"pelvis": (0, 0.04, -0.41)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.42)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    hit = merge(feet((0.11, -0.04), (0.13, -0.22)), arm("l", 0.6, -0.2, 0.0, 0.5, 0.1, 0.2), arm("r", 0.6, -0.2, 0.0, 0.5, 0.1, 0.2), {"spine_01": (-16, 0, 0), "head": (-32, 0, 0)}, cape(-20))
    down = merge({"thigh_l": (-30, 0, 0), "thigh_r": (-14, 0, 0), "calf_l": (70, 0, 0), "calf_r": (40, 0, 0)}, arm("l", 0.9, 0.2, 0.2, 0.8, 0.4, 0.1),
                 arm("r", 0.5, 0.6, 0.4, 0.3, 0.8, 0.3), {"pelvis": (-76, 16, 0), "head": (18, 0, -14)}, cape(-30))
    keys = [(0, IDLE, None), (6, hit, {"pelvis": (0, 0.06, -0.02)}, "settle"), (14, merge(hit, {"spine_01": (10, 0, 0)}), {"pelvis": (0, 0.2, -0.4)}, "snap"),
            (22, down, {"pelvis": (0, 0.42, -0.76)}, "snap"), (32, down, {"pelvis": (0, 0.43, -0.77)})]
    return _clip(armature, "death", 32, keys)


def respawn(armature):
    keys = [(0, FOURS, {"pelvis": (0, -0.04, -0.46)}, "settle"), (8, merge(IDLE, {"head": (-10, 0, 0)}), {"pelvis": (0, 0, 0.04)}), (20, IDLE, None)]
    return _clip(armature, "respawn", 20, keys)


def select(armature):
    # A spin on one heel, the crossbow snapped up to aim at you, and a wink.
    aimed = merge(STAND, aim_bow((0, -1, 0.05), support=True), {"head": (-6, 0, 6)}, cape(6))
    keys = [(0, IDLE, None), (6, merge(IDLE, {"pelvis": (0, 0, 120)}, cape(20, 10)), {"pelvis": (0, 0, 0.03)}),
            (12, merge(IDLE, {"pelvis": (0, 0, 240)}, cape(20, -10)), {"pelvis": (0, 0, 0.03)}), (16, merge(aimed, {"pelvis": (0, 0, 360)}), None, "snap"),
            (34, merge(aimed, {"pelvis": (0, 0, 360), "head": (-6, 0, 14)}), None), (46, merge(IDLE, {"pelvis": (0, 0, 360)}), None)]
    return _clip(armature, "select", 46, keys)


def emote_taunt(armature):
    # Blows a raspberry, thumbs at her ears, fingers wiggling.
    t = merge(IDLE, hand("l", at(0.12, 0.0, 1.80, "l"), aim=at(0.2, 0.2, 1, "l")), {"head": (-6, 0, 0)})
    keys = [(0, IDLE, None), (8, t, None)]
    for i, f in enumerate(range(12, 30, 3)):
        keys.append((f, merge(t, {"fingers_l": (40 if i % 2 else 0, 0, 0)}), None, "snap"))
    keys.append((40, IDLE, None))
    return _clip(armature, "emote_taunt", 40, keys)


def emote_joke(armature):
    # Flaps her capelet like wings and hops.
    flap_up = merge(IDLE, arm("l", 1, 0.0, 0.4, 1, 0.0, 0.3), cape(-20))
    flap_dn = merge(IDLE, arm("l", 1, 0.0, -0.3, 1, 0.0, -0.4), cape(20))
    keys = [(0, IDLE, None)]
    for i, f in enumerate(range(6, 34, 4)):
        keys.append((f, flap_up if i % 2 == 0 else flap_dn, {"pelvis": (0, 0, 0.08 if i % 2 == 0 else 0.0)}, "snap"))
    keys.append((44, IDLE, None))
    return _clip(armature, "emote_joke", 44, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, None)]
    for i in range(1, 9):
        s = 1.0 if i % 2 else 0.4
        keys.append((4 * i, merge(IDLE, arm("l", 0.15, 0.3, -0.9, -0.7, 0.6, 0.2), {"spine_01": (10 * s, 0, 0), "head": (16 * s, 0, 0)}, sym("clavicle", 0, -4 * s, 0)),
                     {"pelvis": (0, 0, -0.02 * s)}))
    keys.append((40, IDLE, None))
    return _clip(armature, "emote_laugh", 40, keys)


def emote_dance(armature):
    # A hopping jig, the crossbow held aloft.
    up = merge(arm("r", 0.3, 0.1, 0.95, 0.2, 0.0, 1), {"hand_r": clips.aim(0, 0, 1, "r")}, arm("l", 0.6, 0.2, 0.0, 0.2, 0.8, 0.3))
    a = merge(up, feet((0.11, 0.06), (0.13, -0.08), lift_r=0.20), {"pelvis": (0, -6, 10)}, cape(6, 8))
    b = merge(up, feet((0.11, 0.06), (0.13, -0.08), lift_l=0.20), {"pelvis": (0, 6, -10)}, cape(6, -8))
    keys = [(0, a, {"pelvis": (0.03, 0, 0.06)}), (6, merge(up, STAND), {"pelvis": (0, 0, -0.04)}, "snap"), (12, b, {"pelvis": (-0.03, 0, 0.06)}),
            (18, merge(up, STAND), {"pelvis": (0, 0, -0.04)}, "snap"), (24, a, {"pelvis": (0.03, 0, 0.06)}), (30, merge(up, STAND), {"pelvis": (0, 0, -0.04)}, "snap"),
            (36, b, {"pelvis": (-0.03, 0, 0.06)}), (42, merge(up, STAND), {"pelvis": (0, 0, -0.04)}, "snap")]
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e_start, e_travel, e_land, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    rig.add_chain(armature, "cape", "chest", CAPE)


def build(only=None):
    return champion.build("wren", SHAPE, parts, OWN, 1.7, extras=_extras, only=only, shared=champion.SHARED + ["attack_melee_alt"])


if __name__ == "__main__":
    champion.main(build)
