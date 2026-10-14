# SPDX-License-Identifier: AGPL-3.0-or-later
"""Lumen (rebuilt for 14): the generator for her model and clips.

    blender -b --python art/champions/lumen/build.py -- --out art/champions/lumen/lumen.blend

A wind-blown dress (14 §5): a serene light-priestess who rides the air. A fitted bodice laced in
gold, an underskirt cut to mid-shin, and three long tails of the dress (one down her back, one
at each hip) on cloth chains, always lifting in a wind only she feels; bare arms with gold
armlets and open bell sleeves; a small halo behind her head; a tall lantern-staff in her right
hand, usually held in both. Her signature is a pale sun gold (14 §4). (The first version was a
bell robe to the ground: it read as a cone with a stick.)

Weight (14 §3): her weight is in the staff and her arms: both hands on it when she casts, the
free hand sweeping wide when she doesn't; Lull is a two-handed thrust of the lantern, the whole
floating body behind it; every gesture slow in, quick out, held a beat. The dress's tails trail
every move (secondary motion, 14 §3.5) and drift in an authored wind at rest.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import champion, clips, head, rig  # noqa: E402
from mftr_blender.champion import arm, hand  # noqa: E402
from mftr_blender.clips import at, grip, ik, merge, sym  # noqa: E402
from mftr_blender.library import _clip  # noqa: E402
from mftr_blender.mesh import box_part, path_part  # noqa: E402

SKIN = (0.80, 0.62, 0.52)
IVORY = (0.50, 0.46, 0.38)
WHITE = (0.58, 0.56, 0.52)
GOLD = (0.70, 0.54, 0.22)
SKY = (0.20, 0.32, 0.50)
SKY_PALE = (0.34, 0.48, 0.64)
HAIR = (0.78, 0.66, 0.40)
WOOD = (0.36, 0.26, 0.16)
SUN = (1.0, 0.86, 0.42)
SUN_WHITE = (1.0, 0.96, 0.78)
ACCENT = (0.18, 0.52, 0.95)

SHAPE = {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 1.0, "y": 1.0, "scale": 0.96, "shoulder": 0.0, "hand": 1.3}
rig.use_shape(SHAPE)
BONES = {b[0]: b for b in rig.bone_table()}
HAND_LEN = (Vector(BONES["hand_r"][3]) - Vector(BONES["hand_r"][2])).length
GRIP = 0.5 * HAND_LEN
STAFF_TOP = GRIP + 1.10    # the lantern, along hand_r from the wrist
STAFF_FOOT = GRIP - 0.62   # the staff's foot

# The dress's tails (cloth chains): down her back, and at each hip.
SKIRT = [rig.shaped(p) for p in [(0, 0.12, 1.04), (0, 0.19, 0.74), (0, 0.24, 0.44), (0, 0.28, 0.12)]]
PANEL_L = [rig.shaped(p) for p in [(0.15, 0.0, 1.04), (0.19, 0.04, 0.74), (0.21, 0.08, 0.44), (0.22, 0.12, 0.14)]]
PANEL_R = [(-x, y, z) for x, y, z in PANEL_L]


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        # The underskirt: fitted at the hips, a soft A-line to mid-shin, gold-hemmed (no bell).
        path_part("pelvis", [sp((0, 0, 1.10)), sp((0, 0.003, 1.00)), sp((0, 0.005, 0.90)), sp((0, 0.008, 0.78)), sp((0, 0.01, 0.66)), sp((0, 0.013, 0.53)),
                             sp((0, 0.015, 0.40))], 16,
                  [(.15, .12), (.162, .132), (.174, .144), (.19, .158), (.21, .175), (.224, .186), (.235, .195)], "cloth", WHITE),
        path_part("pelvis", [sp((0, 0.015, 0.42)), sp((0, 0.015, 0.37))], 12, [(.237, .197), (.24, .20)], "metal", GOLD),
        # A sky sash at the waist, knotted at her left hip with two short tails.
        ("pelvis", sp((0, 0, 1.03)), sp((0, 0, 1.11)), 10, [(0, .152, .122), (1, .148, .118)], "accent", ACCENT),
        box_part("pelvis", sp((0.12, -0.09, 1.06)), (0.07, 0.04, 0.07), "metal", GOLD, rot=(0, 0, 35), bevel=0.3),
        box_part("pelvis", sp((0.13, -0.10, 0.92)), (0.04, 0.015, 0.20), "cloth", SKY, rot=(0, 8, 30), taper=0.6),
        box_part("pelvis", sp((0.09, -0.12, 0.94)), (0.04, 0.015, 0.16), "cloth", SKY, rot=(0, -6, 30), taper=0.6),
        # The bodice: fitted, laced in a gold V down the front.
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 10, [(0, .112, .086), (1, .115, .088)], "cloth", IVORY),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 10, [(0, .118, .088), (1, .14, .10)], "cloth", WHITE),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.58)), 10, [(0, .144, .104), (.45, .148, .108), (1, .085, .07)], "cloth", WHITE),
        box_part("spine_02", sp((0.032, -0.094, 1.33)), (0.018, 0.015, 0.17), "metal", GOLD, rot=(0, -14, 0)),
        box_part("spine_02", sp((-0.032, -0.094, 1.33)), (0.018, 0.015, 0.17), "metal", GOLD, rot=(0, 14, 0)),
        box_part("spine_01", sp((0, -0.09, 1.20)), (0.035, 0.015, 0.035), "metal", GOLD, rot=(0, 45, 0)),
        # A sky collar with a gold torc and a sun stone at the throat.
        ("chest", sp((0, 0, 1.51)), sp((0, -0.005, 1.60)), 12, [(0, .165, .118), (1, .095, .08)], "cloth", SKY),
        ("chest", sp((0, 0, 1.50)), sp((0, 0, 1.52)), 12, [(0, .17, .122), (1, .17, .122)], "metal", GOLD),
        box_part("chest", sp((0, -0.12, 1.49)), (0.06, 0.03, 0.06), "emissive", SUN, rot=(0, 45, 0)),
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .046, .046), (1, .042, .042)], "skin", SKIN),
    ]


def _tail(chain, points, widths, ref):
    """One of the dress's tails: a flat ribbon of cloth, a segment per chain bone, white with a
    sky edge at the end."""
    out = []
    n = len(points) - 1
    for i, (a, b) in enumerate(zip(points, points[1:]), start=1):
        colors = [WHITE, SKY_PALE] if i == n else None
        out.append(path_part(f"extra_{chain}_{i}", [a, (Vector(a) + Vector(b)) / 2, b], 6, [(widths[i - 1], .012), ((widths[i - 1] + widths[i]) / 2, .014), (widths[i], .012)], "cloth", WHITE, ref=ref,
                             colors=colors and [WHITE, WHITE, SKY_PALE]))
    return out


def _tails():
    return (_tail("skirt", SKIRT, (0.13, 0.16, 0.17, 0.10), (1, 0, 0))
            + _tail("panel_l", PANEL_L, (0.07, 0.10, 0.11, 0.06), (0, 1, 0))
            + _tail("panel_r", PANEL_R, (0.07, 0.10, 0.11, 0.06), (0, 1, 0)))


def _halo():
    # A small halo standing behind her head, on a gold stem from her nape.
    c = Vector(sp((0, 0.18, 1.86)))
    r = 0.22
    ring = [2 * math.pi * k / 16 for k in range(17)]
    pts = [tuple(c + Vector((math.cos(a) * r, 0.0, math.sin(a) * r))) for a in ring]
    inner = [tuple(c + Vector((math.cos(a) * (r - 0.05), 0.01, math.sin(a) * (r - 0.05)))) for a in ring]
    return [path_part("chest", pts, 4, [(.032, .018)] * 17, "emissive", SUN, ref=(0, 1, 0)),
            path_part("chest", inner, 4, [(.012, .01)] * 17, "emissive", SUN_WHITE, ref=(0, 1, 0)),
            path_part("chest", [sp((0, 0.11, 1.62)), sp((0, 0.17, 1.66)), sp((0, 0.18, 1.86 - r))], 6, [(.026, .026), (.022, .022), (.018, .018)], "metal", GOLD)]


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
        # Bare arms, gold armlets; open bell sleeves from the elbow, sky-hemmed.
        (ua, (ua, 0), (ua, 1), 10, [(0, .052, .052), (1, .044, .044)], "skin", SKIN),
        (ua, (ua, 0.0), (ua, 0.22), 10, [(0, .066, .066), (1, .062, .062)], "cloth", WHITE),
        (ua, (ua, 0.55), (ua, 0.65), 10, [(0, .05, .05), (1, .05, .05)], "metal", GOLD),
        (fa, (fa, 0), (fa, 1), 8, [(0, .042, .042), (1, .034, .034)], "skin", SKIN),
        (fa, (fa, 0.05), (fa, 0.95), 12, [(0, .05, .05), (.5, .085, .08), (1, .125, .11)], "cloth", WHITE),
        (fa, (fa, 0.93), (fa, 0.99), 12, [(0, .127, .112), (1, .13, .115)], "cloth", SKY),
        (fa, (fa, 0.86), (fa, 0.92), 8, [(0, .038, .038), (1, .038, .038)], "metal", GOLD),
        *champion.hand_parts("l", SKIN, "skin", scale=1.25),
        # Legs below the underskirt: skin, gold-laced sandals.
        ("thigh_l", sp((0.10, 0, 1.02)), ("thigh_l", 1), 8, [(0, .08, .08), (1, .056, .056)], "cloth", IVORY),
        ("calf_l", ("calf_l", 0), ("calf_l", 1), 8, [(0, .05, .05), (.5, .046, .046), (1, .036, .036)], "skin", SKIN),
        ("calf_l", ("calf_l", 0.78), ("calf_l", 0.86), 8, [(0, .042, .042), (1, .04, .04)], "metal", GOLD),
        box_part("foot_l", ("foot_l", 0.45), (0.07, 0.14, 0.075), "cloth", GOLD, bevel=0.3),
    ]


def _staff():
    """The lantern-staff, skinned to the right hand and laid along it."""
    h, t = Vector(BONES["hand_r"][2]), Vector(BONES["hand_r"][3])
    axis = (t - h).normalized()

    def p(d, off=(0, 0, 0)):
        return tuple(h + axis * d + Vector(off))

    hb = "hand_r"
    top = h + axis * STAFF_TOP
    side = axis.cross(Vector((0, 1, 0))).normalized()
    return [
        path_part(hb, [p(STAFF_FOOT), p(STAFF_TOP - 0.12)], 6, [(.026, .026), (.03, .03)], "cloth", WOOD),
        path_part(hb, [p(STAFF_FOOT - 0.05), p(STAFF_FOOT + 0.04)], 6, [(.0, .0), (.035, .035)], "metal", GOLD),
        path_part(hb, [p(GRIP - 0.08), p(GRIP + 0.10)], 6, [(.034, .034), (.034, .034)], "metal", GOLD),
        # The lantern: a gold cage around a sun-bright core, crowned by a crescent.
        path_part(hb, [p(STAFF_TOP - 0.14), p(STAFF_TOP - 0.10), p(STAFF_TOP + 0.08), p(STAFF_TOP + 0.12)], 6,
                  [(.03, .03), (.075, .075), (.075, .075), (.03, .03)], "metal", GOLD),
        path_part(hb, [p(STAFF_TOP - 0.08), p(STAFF_TOP), p(STAFF_TOP + 0.07)], 6, [(.06, .06), (.085, .085), (.05, .05)], "emissive", SUN_WHITE),
        *[path_part(hb, [tuple(h + axis * (STAFF_TOP - 0.10) + v * 0.09), tuple(h + axis * (STAFF_TOP + 0.08) + v * 0.09)], 5,
                    [(.008, .008), (.008, .008)], "metal", GOLD)
          for v in (side, -side, side.cross(axis).normalized(), -side.cross(axis).normalized())],
        *[path_part(hb, [p(d), p(d + 0.03)], 8, [(.036, .036), (.036, .036)], "metal", GOLD) for d in (GRIP + 0.30, GRIP + 0.55, GRIP + 0.80)],
        path_part(hb, [tuple(top + axis * 0.12 + side * 0.10), tuple(top + axis * 0.22 + side * 0.06), tuple(top + axis * 0.26),
                       tuple(top + axis * 0.22 - side * 0.06), tuple(top + axis * 0.12 - side * 0.10)], 4,
                  [(0, 0), (.025, .015), (.03, .015), (.025, .015), (0, 0)], "metal", GOLD),
        # Two sky ribbons tied under the lantern.
        path_part(hb, [p(STAFF_TOP - 0.16), p(STAFF_TOP - 0.30, side * 0.06), p(STAFF_TOP - 0.46, side * 0.04)], 4, [(.02, .006), (.022, .006), (.012, .006)],
                  "cloth", SKY, ref=tuple(side)),
    ]


def parts():
    return _center() + champion.mirrored(_left()) + _tails() + _halo() + _staff() + _circlet() + [_head]


# --- Poses ----------------------------------------------------------------------------------

FLOAT = 0.10  # she hovers this far up


def staff(top, foot=None, left=None, pole=(1.0, -0.2, -0.8)):
    """The staff's lantern at `top` (armature space), its foot toward `foot` (default: straight
    below), the right hand on it at GRIP; `left` puts the left hand on it too, that far from the
    right hand toward the lantern (negative: toward the foot)."""
    tp = Vector(top)
    ft = Vector(foot) if foot else tp - Vector((0, 0, 1))
    axis = (tp - ft).normalized()
    out = {"ik_hand_r": ik(tuple(tp - axis * STAFF_TOP), aim=tuple(axis), pole=pole)}
    if left is not None:
        out["grip_l"] = grip("hand_r", at=GRIP + left, pole=(1.0, -0.3, -0.7))
    return out


def wind(back=0.0, sway=0.0):
    """The dress's tails: `back` lifts them behind her (degrees), `sway` swings them sideways."""
    out = {}
    for chain, k in (("skirt", 1.0), ("panel_l", 0.8), ("panel_r", 0.8)):
        for i, f in enumerate((0.45, 0.35, 0.3), start=1):
            out[f"extra_{chain}_{i}"] = (back * f * k, sway * f, 0)
    return out


LEGS = {"thigh_l": (-6, 0, 0), "thigh_r": (4, 0, 0), "calf_l": (16, 0, 0), "calf_r": (14, 0, 0), "foot_l": (36, 0, 0), "foot_r": (36, 0, 0)}
# Both hands on the staff, upright at her right side and tipped a little out: the right hand
# low, the left reaching across her body to hold it at chest height; her face stays clear.
IDLE = merge(LEGS, staff(at(0.42, 0.26, 2.06, "r"), at(0.26, 0.20, 0.28, "r"), left=0.44), {"head": (-4, 0, -6), "spine_02": (0, 0, 4)}, wind(14, 6))
READY = merge(LEGS, staff(at(0.30, 0.55, 2.0, "r"), at(0.24, 0.05, 0.42, "r"), left=0.46), {"spine_01": (5, 0, 0), "head": (-6, 0, 0)}, wind(18, 4))
UP = {"pelvis": (0, 0, FLOAT)}


def idle(armature):
    # A slow breath on the wind: she rises and settles, the dress's tails lifting and drifting
    # across, the staff tipping with her, her head turning into the breeze and back.
    keys = []
    for i, f in enumerate(range(0, 96, 12)):
        a = 2 * math.pi * f / 96
        s, c = math.sin(a), math.cos(a)
        pose = merge(LEGS, staff(at(0.42 + 0.03 * s, 0.26, 2.06 + 0.02 * c, "r"), at(0.26, 0.20, 0.28 + 0.02 * c, "r"), left=0.44),
                     {"head": (-4 - 3 * c, 0, -6 + 8 * s), "spine_02": (0, 3 * s, 4), "chest": (-2 * c, 0, 0)},
                     sym("clavicle", 0, -2 - 2 * c, 0), wind(16 + 8 * c, 10 * s))
        keys.append((f, pose, {"pelvis": (0, 0, FLOAT + 0.03 * (1 - c))}))
    keys.append((96, keys[0][1], keys[0][2]))
    return _clip(armature, "idle", 96, keys, loop=True)


def idle_fidget_1(armature):
    # Lets go of the staff with her left hand, cups a mote of light, and lets it drift up out of
    # her open fingers, watching it go; then takes the staff again.
    one = merge(LEGS, staff(at(0.30, 0.25, 2.04, "r")), wind(16, 4))
    cup = merge(one, hand("l", at(0.10, 0.32, 1.30, "l"), aim=at(-0.3, 0.6, 0.6, "l")), {"fingers_l": (40, 0, 0), "head": (14, 0, 8)})
    up = merge(one, arm("l", 0.3, 0.5, 0.5, 0.1, 0.3, 0.95), {"fingers_l": (-15, 0, 0), "head": (-24, 0, 6)}, wind(22, -6))
    keys = [(0, IDLE, UP), (10, one, UP), (20, cup, UP), (32, cup, {"pelvis": (0, 0, FLOAT + 0.02)}), (40, up, {"pelvis": (0, 0, FLOAT + 0.05)}, "settle"),
            (54, merge(up, {"head": (-28, 0, 10)}), {"pelvis": (0, 0, FLOAT + 0.05)}), (68, IDLE, UP)]
    return _clip(armature, "idle_fidget_1", 68, keys)


def idle_fidget_2(armature):
    # Twirls the staff once round her right hand, catches it in both and bows her head.
    spin = [merge(LEGS, staff(at(0.30 + 0.45 * math.sin(a), 0.30 + 0.45 * math.cos(a) * 0.3, 1.40 + 0.70 * math.cos(a), "r"),
                              at(0.30 - 0.45 * math.sin(a), 0.30, 1.40 - 0.70 * math.cos(a), "r")),
                  arm("l", 0.7, 0.1, -0.2, 0.7, 0.2, 0.2), wind(20, 8), {"head": (-6, 0, -12)}) for a in (0.0, math.pi / 2, math.pi, 3 * math.pi / 2)]
    bow = merge(IDLE, {"head": (24, 0, 0), "neck": (8, 0, 0), "spine_02": (6, 0, 0)}, wind(10, 4))
    keys = [(0, IDLE, UP), (8, spin[0], UP), (11, spin[1], UP, "snap"), (14, spin[2], UP, "snap"), (17, spin[3], UP, "snap"), (20, spin[0], UP),
            (26, IDLE, UP, "settle"), (36, bow, {"pelvis": (0, 0, FLOAT - 0.03)}), (52, bow, {"pelvis": (0, 0, FLOAT - 0.02)}), (64, IDLE, UP)]
    return _clip(armature, "idle_fidget_2", 64, keys)


def idle_ready(armature):
    keys = [(0, READY, UP), (20, merge(READY, {"spine_01": (7, 0, 0)}, wind(24, -4)), {"pelvis": (0, 0, FLOAT + 0.03)}), (40, READY, UP)]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _glide(frames, lean, trail):
    # The glide (14 §3.6): no footfalls, a slow hover bob, leaning into it, the staff trailing in
    # her right hand and the left arm swept back like a wing; the dress streams.
    keys = []
    for f in range(0, frames, 2):
        p = f / frames
        bob = 0.035 * math.sin(2 * math.pi * p)
        pose = merge({"thigh_l": (-10, 0, 0), "thigh_r": (14, 0, 0), "calf_l": (20, 0, 0), "calf_r": (34, 0, 0), "foot_l": (40, 0, 0), "foot_r": (44, 0, 0)},
                     staff(at(0.40, -0.10 - trail * 0.25, 2.05 + bob, "r"), at(0.36, 0.25 + trail * 0.25, 0.9 + bob, "r")),
                     arm("l", 0.6, -0.55 - trail * 0.3, -0.45 + 0.1 * math.sin(2 * math.pi * p), 0.45, -0.6, -0.25),
                     {"spine_01": (lean, 0, 0), "spine_02": (lean * 0.3, 0, 4 * math.sin(2 * math.pi * p)), "head": (-lean * 0.8, 0, 0)},
                     wind(25 + 50 * trail + 6 * math.sin(4 * math.pi * p), 6 * math.sin(2 * math.pi * p)))
        keys.append((f, pose, {"pelvis": (0, 0, FLOAT + bob)}))
    return keys


def walk(armature):
    # A slow glide (heavy slows, and the moment a run stops): the staff upright (14 §3.6).
    return _clip(armature, "walk", 32, _glide(32, 5, 0.05), loop=True, markers={"foot_l": 0, "foot_r": 16}, stride_speed=140.0)


def run(armature):
    return _clip(armature, "run", 20, _glide(20, 12, 0.3), loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    return _clip(armature, "run_fast", 16, _glide(16, 18, 0.6), loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(armature):
    # The lantern swung forward in both hands to point at the target: drawn back, cocked, a
    # one-frame thrust on `fire` (sim: 9) that sends the mote, held.
    back = merge(READY, staff(at(0.30, -0.25, 2.20, "r"), at(0.35, 0.25, 0.95, "r"), left=0.40), {"spine_01": (-4, 0, -12), "head": (-8, 0, 10)}, wind(14, 6))
    cock = merge(back, staff(at(0.25, -0.40, 2.10, "r"), at(0.35, 0.30, 1.00, "r"), left=0.40), {"spine_01": (-6, 0, -16)})
    thrust = merge(READY, staff(at(0.08, 1.25, 1.75, "r"), at(0.25, 0.0, 1.10, "r"), left=0.45, pole=(1.0, -0.4, -0.6)), {"spine_01": (10, 0, 10), "head": (-10, 0, -8)},
                   wind(30, -4))
    keys = [(0, READY, UP), (5, back, UP), (8, cock, UP, "snap"), (9, thrust, {"pelvis": (0, -0.04, FLOAT - 0.02)}),
            (15, merge(thrust, {"spine_01": (12, 0, 12)}), {"pelvis": (0, -0.04, FLOAT - 0.02)}, "settle"), (24, READY, UP)]
    return _clip(armature, "attack_1", 24, keys, markers={"fire": 9})


def attack_2(armature):
    # A mote pushed from her open left palm, the staff held off to the side: the hand drawn in
    # to her shoulder, cocked, pushed out on `fire`.
    one = merge(LEGS, staff(at(0.42, 0.30, 2.0, "r")), {"spine_01": (4, 0, 0)})
    draw = merge(one, hand("l", at(0.18, -0.05, 1.50, "l"), aim=at(0.0, 0.6, 0.6, "l")), {"spine_01": (0, 0, 14), "head": (-6, 0, -12)}, wind(14, -4))
    cock = merge(draw, hand("l", at(0.22, -0.15, 1.52, "l"), aim=at(0.1, 0.4, 0.8, "l")), {"spine_01": (0, 0, 18)})
    push = merge(one, arm("l", 0.1, 1, 0.1, 0.02, 1, 0.12), {"fingers_l": (-20, 0, 0), "spine_01": (8, 0, -12), "head": (-8, 0, 8)}, wind(28, 6))
    keys = [(0, READY, UP), (5, draw, UP), (8, cock, UP, "snap"), (9, push, {"pelvis": (0, -0.03, FLOAT - 0.02)}),
            (15, merge(push, {"spine_01": (10, 0, -14)}), {"pelvis": (0, -0.03, FLOAT - 0.02)}, "settle"), (24, READY, UP)]
    return _clip(armature, "attack_2", 24, keys, markers={"fire": 9})


def q(armature):
    # Mending Light (upper body, a pulse): the staff lifted, the left hand raised palm up and
    # the light offered on `fire` 4, fingers opening, held a moment.
    offer = merge(staff(at(0.40, 0.30, 2.25, "r")), arm("l", 0.35, 0.75, 0.3, 0.1, 0.8, 0.55), {"fingers_l": (-20, 0, 0), "head": (-12, 0, 0)}, wind(22, 4))
    keys = [(0, IDLE, None), (3, merge(offer, arm("l", 0.3, 0.6, 0.0, 0.1, 0.8, 0.2), {"fingers_l": (50, 0, 0)}), None, "snap"), (4, offer, None),
            (12, merge(offer, arm("l", 0.35, 0.7, 0.45, 0.1, 0.8, 0.7)), None, "settle"), (24, IDLE, None)]
    return _clip(armature, "q", 24, keys, layer="upper", markers={"fire": 4})


def w(armature):
    # Aegis (upper body, a pulse): the staff raised high and the left palm thrown forward on
    # `fire` 4, a ward pressed into place, held.
    ward = merge(staff(at(0.30, 0.40, 2.50, "r")), arm("l", 0.1, 1, 0.25, 0.0, 1, 0.3), {"fingers_l": (-25, 0, 0), "head": (-8, 0, 0), "spine_02": (-4, 0, 0)}, wind(26, 0))
    keys = [(0, IDLE, None), (3, merge(ward, arm("l", 0.2, 0.6, 0.2, -0.4, 0.4, 0.6)), None, "snap"), (4, ward, None),
            (12, merge(ward, {"spine_02": (-6, 0, 0)}), None, "settle"), (24, IDLE, None)]
    return _clip(armature, "w", 24, keys, layer="upper", markers={"fire": 4})


def e(armature):
    # Lull, her central strike: both hands on the staff, she rises and draws it back over her
    # right shoulder, the lantern blazing behind her; on `fire` 8 she drives it forward at the
    # target, her whole floating body behind the thrust, the dress flung back; the thrust is
    # held long while the light pours out, then she drifts back to ready.
    gather = merge(LEGS, staff(at(0.40, -0.50, 2.20, "r"), at(0.05, 0.40, 1.10, "l"), left=0.36), {"spine_01": (-6, 0, -22), "spine_02": (-6, 0, -10), "head": (-12, 0, 20)},
                   wind(10, 10))
    cock = merge(gather, staff(at(0.42, -0.62, 2.12, "r"), at(0.02, 0.45, 1.20, "l"), left=0.36), {"spine_01": (-8, 0, -26)})
    thrust = merge({"thigh_l": (-20, 0, 0), "thigh_r": (16, 0, 0), "calf_l": (24, 0, 0), "calf_r": (40, 0, 0), "foot_l": (40, 0, 0), "foot_r": (44, 0, 0)},
                   staff(at(0.02, 1.40, 1.62, "r"), at(0.15, 0.20, 1.25, "r"), left=0.50, pole=(1.0, -0.4, -0.5)),
                   {"spine_01": (16, 0, 6), "spine_02": (6, 0, 4), "head": (-14, 0, -4)}, wind(55, -6))
    keys = [(0, READY, UP), (5, gather, {"pelvis": (0, 0.06, FLOAT + 0.10)}), (7, cock, {"pelvis": (0, 0.08, FLOAT + 0.12)}, "snap"),
            (8, thrust, {"pelvis": (0, -0.16, FLOAT + 0.02)}), (13, merge(thrust, wind(60, -2)), {"pelvis": (0, -0.18, FLOAT + 0.02)}),
            (20, merge(thrust, {"spine_01": (14, 0, 6)}, wind(40, 4)), {"pelvis": (0, -0.17, FLOAT + 0.03)}, "settle"), (30, READY, UP)]
    return _clip(armature, "e", 30, keys, markers={"fire": 8})


def r(armature):
    # Binding Halo: the staff lifted high in both hands and held as the halo flares, then driven
    # down, its foot planted, on `fire` 8; she holds it there.
    high = merge(LEGS, staff(at(0.15, 0.30, 2.85, "r"), left=0.55), {"spine_02": (-10, 0, 0), "head": (-18, 0, 0)}, wind(30, 0))
    plant = merge(LEGS, staff(at(0.15, 0.55, 2.05, "r"), at(0.15, 0.62, 0.0, "r"), left=0.40, pole=(1.0, -0.2, -0.6)),
                  {"spine_01": (14, 0, 0), "head": (-12, 0, 0)}, wind(8, 0))
    keys = [(0, READY, UP), (5, high, {"pelvis": (0, 0, FLOAT + 0.08)}), (7, merge(high, {"spine_02": (-14, 0, 0)}), {"pelvis": (0, 0, FLOAT + 0.10)}, "snap"),
            (8, plant, {"pelvis": (0, -0.02, FLOAT - 0.06)}), (16, merge(plant, {"spine_01": (16, 0, 0)}, wind(14, 0)), {"pelvis": (0, -0.02, FLOAT - 0.07)}, "settle"),
            (26, READY, UP)]
    return _clip(armature, "r", 26, keys, markers={"fire": 8})


KNEEL = merge({"thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0), "foot_l": (0, 0, 0), "thigh_r": (6, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
              staff(at(0.25, 0.30, 1.75, "r"), at(0.25, 0.32, 0.0, "r"), left=0.35), {"spine_01": (6, 0, 0), "head": (22, 0, 0)}, wind(-10, 0))


def recall(armature):
    keys = [(0, IDLE, UP), (15, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (45, merge(KNEEL, {"head": (26, 0, 0)}), {"pelvis": (0, 0.04, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (90, IDLE, UP)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    # Her light fails: she sinks, kneels, and slumps sideways onto the staff.
    sink = merge(LEGS, staff(at(0.40, 0.30, 1.90, "r")), arm("l", 0.5, 0.2, -0.7, 0.4, 0.3, -0.6), {"spine_01": (14, 0, 0), "head": (30, 0, 0)}, wind(-6, 0))
    down = merge({"thigh_l": (-20, 0, 0), "thigh_r": (-10, 0, 0), "calf_l": (60, 0, 0), "calf_r": (40, 0, 0)}, arm("r", 0.9, 0.3, 0.1, 0.9, 0.3, 0.0),
                 arm("l", 0.6, 0.4, 0.4, 0.4, 0.6, 0.4), {"pelvis": (-70, 16, 0), "head": (20, 0, -16)}, wind(-20, 0))
    keys = [(0, IDLE, UP), (10, sink, {"pelvis": (0, 0, -0.10)}), (18, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "snap"),
            (28, down, {"pelvis": (0, 0.42, -0.84)}, "snap"), (36, down, {"pelvis": (0, 0.44, -0.85)})]
    return _clip(armature, "death", 36, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "settle"), (14, merge(IDLE, {"head": (-10, 0, 0)}, wind(30, 0)), {"pelvis": (0, 0, FLOAT + 0.06)}), (26, IDLE, UP)]
    return _clip(armature, "respawn", 26, keys)


def select(armature):
    # Rises higher, the staff lifted, the halo blazing, her left arm opening wide to the wind.
    up = merge(LEGS, staff(at(0.30, 0.25, 2.70, "r")), arm("l", 1, 0.3, 0.35, 1, 0.3, 0.4), {"fingers_l": (-20, 0, 0), "head": (-16, 0, 0), "spine_02": (-6, 0, 0)},
               wind(40, 12))
    keys = [(0, IDLE, UP), (16, up, {"pelvis": (0, 0, FLOAT + 0.12)}), (36, merge(up, {"head": (-20, 0, -6)}, wind(34, -8)), {"pelvis": (0, 0, FLOAT + 0.14)}), (52, IDLE, UP)]
    return _clip(armature, "select", 52, keys)


def emote_taunt(armature):
    # Wags a finger, gently disapproving.
    one = merge(LEGS, staff(at(0.40, 0.25, 2.04, "r")), wind(14, 4))
    f = merge(one, arm("l", 0.2, 0.7, 0.3, -0.3, 0.3, 0.9), {"head": (-6, 0, -8)})
    keys = [(0, IDLE, UP), (8, f, UP)]
    for i, fr in enumerate(range(12, 30, 4)):
        keys.append((fr, merge(f, arm("l", 0.2, 0.7, 0.3, -0.3 + (0.2 if i % 2 else -0.2), 0.3, 0.9)), UP))
    keys.append((40, IDLE, UP))
    return _clip(armature, "emote_taunt", 40, keys)


def emote_joke(armature):
    # Uses the lantern as a hand mirror, tidying her hair.
    mirror = merge(LEGS, staff(at(0.10, 0.45, 1.85, "r"), at(0.40, 0.10, 0.60, "r")), arm("l", 0.6, 0.5, 0.6, -0.6, 0.0, 0.8), {"head": (-6, 0, -20)}, wind(12, 4))
    keys = [(0, IDLE, UP), (12, mirror, UP), (20, merge(mirror, {"head": (-6, 0, -10)}), UP), (28, mirror, UP), (44, IDLE, UP)]
    return _clip(armature, "emote_joke", 44, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, UP)]
    one = merge(LEGS, staff(at(0.40, 0.25, 2.04, "r")), wind(14, 4))
    for i in range(1, 7):
        s = 1.0 if i % 2 else 0.4
        keys.append((5 * i, merge(one, arm("l", 0.15, 0.6, -0.1, -0.5, 0.2, 0.85), {"head": (8 * s, 0, 0)}, sym("clavicle", 0, -3 * s, 0)), UP))
    keys.append((40, IDLE, UP))
    return _clip(armature, "emote_laugh", 40, keys)


def emote_dance(armature):
    # A slow floating turn, the staff swaying overhead, the dress swinging out.
    keys = []
    for i, f in enumerate(range(0, 48, 6)):
        a = 2 * math.pi * i / 8
        keys.append((f, merge(LEGS, staff(at(0.3 + 0.15 * math.cos(a), 0.2 + 0.15 * math.sin(a), 2.5, "r")), arm("l", 0.9, 0.2 * math.cos(a), 0.4, 0.9, 0.3, 0.4),
                              {"pelvis": (0, 0, 20 * math.sin(a)), "head": (-10, 0, -10 * math.sin(a))}, wind(30, 25 * math.cos(a))),
                     {"pelvis": (0, 0, FLOAT + 0.04 + 0.03 * math.sin(a))}))
    keys.append((48, keys[0][1], keys[0][2]))
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, walk, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    rig.add_chain(armature, "skirt", "pelvis", SKIRT)
    rig.add_chain(armature, "panel_l", "pelvis", PANEL_L)
    rig.add_chain(armature, "panel_r", "pelvis", PANEL_R)
    # Motes and Lull leave from the lantern.
    h, t = Vector(BONES["hand_r"][2]), Vector(BONES["hand_r"][3])
    champion.place_socket(armature, "socket_projectile", tuple(h + (t - h).normalized() * STAFF_TOP))


def build(only=None):
    tail = dict(stiffness=0.20, inertia=0.88)
    return champion.build("lumen", SHAPE, parts, OWN, 1.85, extras=_extras, only=only, shared=champion.SHARED + ["attack_melee_alt"],
                          cloth={"skirt": tail, "panel_l": tail, "panel_r": tail})


if __name__ == "__main__":
    champion.main(build)
