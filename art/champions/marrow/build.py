# SPDX-License-Identifier: AGPL-3.0-or-later
"""Marrow (rebuilt for 14): the generator for her model and clips.

    blender -b --python art/champions/marrow/build.py -- --out art/champions/marrow/marrow.blend

A hooded crone (14 §5): a bent bone-witch under a deep plum hood, a high collar of curved bone
spikes rising behind it, a ribcage corset over her robe, long arms ending in bone claws, a ragged
skirt of tattered strips, a shawl on an `extra_cloak` chain and a skull lantern swinging at her
hip. Her signature is magenta (14 §4).

Weight (14 §3): she's in no hurry: a hunched, stalking walk-run; her spells are flung with a
claw and held, and the drain pulls in hard. Feet planted by IK.
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
    """The deep hood, peaked at the back, and the bone collar rising behind it."""
    hb = "head"
    # A round, heavy hood whose peak droops forward over her face (not a point like Shade's).
    out = [path_part(hb, [sp((0, 0.02, 1.58)), sp((0, 0.02, 1.68)), sp((0, 0.02, 1.78)), sp((0, 0.0, 1.86)), sp((0, -0.05, 1.91)), sp((0, -0.13, 1.90)),
                          sp((0, -0.19, 1.84))], 14,
                     [(.15, .16), (.165, .175), (.165, .175), (.15, .155), (.11, .11), (.06, .06), (0, 0)], "cloth", PLUM, ref=(1, 0, 0)),
           path_part(hb, [sp((0.10, -0.08, 1.62)), sp((0.12, -0.13, 1.75)), sp((0.06, -0.15, 1.87)), sp((0.0, -0.155, 1.89)),
                          sp((-0.06, -0.15, 1.87)), sp((-0.12, -0.13, 1.75)), sp((-0.10, -0.08, 1.62))], 6, [(.022, .016)] * 7, "cloth", PLUM_DARK)]
    # The collar: curved bone spikes fanning up behind her head.
    for k in range(9):
        a = math.radians(-88 + 22 * k)
        base = Vector(sp((0.15 * math.sin(a), 0.03 + 0.11 * math.cos(a), 1.55)))
        d = Vector((0.55 * math.sin(a), 0.35 * math.cos(a) + 0.08, 1.0)).normalized()
        L = 0.56 - 0.16 * abs(k - 4) / 4
        pts = [tuple(base), tuple(base + d * L * 0.35), tuple(base + d * L * 0.7 + Vector((0, 0.06, 0))), tuple(base + d * L + Vector((0, 0.13, -0.02)))]
        out.append(path_part("chest", pts, 6, [(.034, .03), (.028, .025), (.016, .014), (0, 0)], "cloth", BONE if k % 2 == 0 else BONE_DARK))
    out.append(path_part("chest", [sp((0.0, 0.0, 1.56)), sp((0.0, 0.0, 1.61))], 12, [(.15, .13), (.14, .12)], "cloth", BONE_DARK))
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
        # Bone claws on the fingertips.
        *[path_part("fingers_l", [("fingers_l", 1.0, (0.0, y, 0.0)), ("fingers_l", 1.5, (0.0, y, -0.01)), ("fingers_l", 1.9, (0.0, y * 1.2, -0.03))], 4,
                    [(.009, .009), (.006, .006), (0, 0)], "cloth", BONE) for y in (-0.032, -0.011, 0.011, 0.032)],
        (th, sp((0.10, 0, 1.02)), (th, 1), 10, [(0, .08, .078), (1, .055, .055)], "cloth", PLUM_DARK),
        (ca, (ca, 0), (ca, 1), 8, [(0, .05, .05), (1, .042, .042)], "skin", SKIN),
        # Rag-wrapped feet.
        box_part("foot_l", ("foot_l", 0.45), (0.08, 0.16, 0.07), "cloth", GRAVE, bevel=0.3),
        (ca, (ca, .82), (ca, 1.0), 8, [(0, .052, .052), (1, .056, .056)], "cloth", GRAVE),
    ]


def parts():
    return _center() + champion.mirrored(_left()) + _hood() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def shawl(back=0.0, sway=0.0):
    return {"extra_cloak_1": (back * 0.45, sway * 0.5, 0), "extra_cloak_2": (back * 0.3, sway * 0.3, 0),
            "extra_cloak_3": (back * 0.25, sway * 0.25, 0)}


BENT = {"spine_01": (14, 0, 0), "spine_02": (10, 0, 0), "chest": (6, 0, 0), "head": (-22, 0, 0)}
STAND = feet((0.13, 0.08), (0.13, -0.08))
CLAWS = merge(arm("l", 0.35, 0.45, -0.75, 0.15, 0.85, -0.3), arm("r", 0.35, 0.45, -0.75, 0.15, 0.85, -0.3), sym("fingers", 30, 0, 0))
IDLE = merge(STAND, BENT, CLAWS, shawl(3))
READY = merge(feet((0.18, 0.16), (0.18, -0.14)), BENT, {"spine_01": (18, 0, 0)}, arm("l", 0.5, 0.6, -0.3, 0.3, 0.9, 0.0),
              arm("r", 0.5, 0.6, -0.3, 0.3, 0.9, 0.0), sym("fingers", 40, 0, 0), shawl(5))
DOWN = {"pelvis": (0, 0, -0.08)}


def idle(armature):
    b = merge(IDLE, sym("clavicle", 0, -4, 0), {"head": (-26, 0, 8)}, sym("fingers", 45, 0, 0), shawl(4, 2))
    return _clip(armature, "idle", 80, [(0, IDLE, {"pelvis": (0, 0, -0.05)}), (40, b, {"pelvis": (0, 0, -0.04)})], loop=True)


def idle_fidget_1(armature):
    # Drums her claws on the skull lantern.
    tap = merge(IDLE, hand("l", at(0.22, 0.02, 0.98, "l"), aim=at(0.1, 0.3, -1, "l")), {"head": (6, 0, 14)})
    keys = [(0, IDLE, None), (10, tap, None)]
    for i, f in enumerate(range(14, 34, 3)):
        keys.append((f, merge(tap, {"fingers_l": (20 if i % 2 else 55, 0, 0)}), None, "snap"))
    keys.append((44, IDLE, None))
    return _clip(armature, "idle_fidget_1", 44, keys)


def idle_fidget_2(armature):
    # Cranes her neck, sniffing the air, a slow head turn.
    keys = [(0, IDLE, None), (14, merge(IDLE, {"head": (-36, 0, 30), "neck": (-10, 0, 10)}), None), (30, merge(IDLE, {"head": (-36, 0, -30), "neck": (-10, 0, -10)}), None),
            (46, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 46, keys)


def idle_ready(armature):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (21, 0, 0)}, shawl(7, 2)), {"pelvis": (0, 0, -0.10)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(trail):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(arm("l", 0.35, 0.35 - 0.2 * c, -0.8, 0.2, 0.75, -0.35), arm("r", 0.35, 0.35 + 0.2 * c, -0.8, 0.2, 0.75, -0.35),
                     sym("fingers", 35, 0, 0), shawl(trail + 5 * math.sin(4 * math.pi * p), 4 * math.sin(2 * math.pi * p)), {"head": (-26, 0, 0)})
    return upper


def run(armature):
    # A hunched, stalking stride (14 §3.6).
    keys = gait(20, 0.96, contact=0.45, lift=0.16, bob=0.04, lean=22, twist=8, width=0.13, crouch=0.07, upper=_run_upper(30))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    keys = gait(16, 0.96, contact=0.40, lift=0.20, bob=0.045, lean=28, twist=10, width=0.13, crouch=0.08, upper=_run_upper(40))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _fling(armature, name, side):
    # A bolt flung with a clawed swipe: claw drawn back and up, cocked, a one-frame swipe on
    # `fire` (sim: 10), held out with the claws spread.
    other = "l" if side == "r" else "r"
    turn = 1 if side == "r" else -1
    st = feet((0.17, 0.22), (0.17, -0.18))
    back = merge(st, BENT, hand(side, at(0.35, -0.15, 1.65, side), aim=at(0.2, 0.2, 0.8, side)), arm(other, 0.5, 0.5, -0.4, 0.3, 0.8, 0.0),
                 {"spine_01": (10, 0, -18 * turn), "head": (-20, 0, 14 * turn)}, {f"fingers_{side}": (10, 0, 0)}, shawl(4))
    cock = merge(back, hand(side, at(0.38, -0.28, 1.62, side), aim=at(0.3, 0.0, 0.8, side)), {"spine_01": (10, 0, -24 * turn)})
    swipe = merge(feet((0.17, 0.32), (0.17, -0.18)), BENT, arm(side, -0.1, 1, -0.05, -0.2, 1, -0.15), arm(other, 0.6, -0.3, -0.5, 0.4, -0.1, -0.4),
                  {"spine_01": (20, 0, 16 * turn), "head": (-22, 0, -10 * turn)}, {f"fingers_{side}": (-10, 0, 0)}, shawl(9))
    keys = [(0, READY, DOWN), (6, back, DOWN), (9, cock, DOWN, "snap"), (10, swipe, {"pelvis": (0, -0.04, -0.10)}),
            (16, merge(swipe, {"spine_01": (22, 0, 18 * turn)}), {"pelvis": (0, -0.04, -0.11)}, "settle"), (26, READY, DOWN)]
    return _clip(armature, name, 26, keys, markers={"fire": 10})


def attack_1(armature):
    return _fling(armature, "attack_1", "r")


def attack_2(armature):
    return _fling(armature, "attack_2", "l")


def q(armature):
    # Siphon: arms flung wide on `fire` 6, claws spread; then she drags the life in, fists
    # pulled hard to her chest, and holds.
    wide = merge(feet((0.20, 0.14), (0.20, -0.12)), {"spine_01": (2, 0, 0), "spine_02": (-8, 0, 0), "head": (-30, 0, 0)},
                 arm("l", 1, 0.3, 0.2, 1, 0.3, 0.3), arm("r", 1, 0.3, 0.2, 1, 0.3, 0.3), sym("fingers", -10, 0, 0), shawl(10))
    pull = merge(feet((0.20, 0.10), (0.20, -0.16)), BENT, {"spine_01": (24, 0, 0)}, hand("l", at(0.08, 0.20, 1.40, "l"), aim=at(-0.6, 0.4, 0.4, "l")),
                 hand("r", at(0.08, 0.20, 1.40, "r"), aim=at(-0.6, 0.4, 0.4, "r")), sym("fingers", 80, 0, 0), shawl(2))
    keys = [(0, READY, DOWN), (4, merge(READY, sym("fingers", 60, 0, 0)), {"pelvis": (0, 0, -0.12)}, "snap"), (6, wide, {"pelvis": (0, 0, -0.02)}),
            (9, merge(wide, {"head": (-34, 0, 0)}), {"pelvis": (0, 0, -0.02)}, "whip"), (12, pull, {"pelvis": (0, 0, -0.12)}),
            (16, pull, {"pelvis": (0, 0, -0.13)}, "settle"), (20, READY, DOWN)]
    return _clip(armature, "q", 20, keys, markers={"fire": 6})


def w(armature):
    # Grasping Bones: a crouch, a claw raised, then driven into the ground on `fire` 9 and held
    # there while the bones erupt down the line.
    crouch = merge(feet((0.20, 0.22), (0.20, -0.16)), BENT, {"spine_01": (24, 0, 0)}, arm("r", 0.4, 0.2, 0.75, 0.3, 0.2, 0.9),
                   arm("l", 0.5, 0.4, -0.5, 0.3, 0.7, -0.2), shawl(4))
    plunge = merge(feet((0.20, 0.26), (0.20, -0.16)), {"spine_01": (40, 0, 0), "spine_02": (14, 0, 0), "head": (-30, 0, 0)},
                   hand("r", at(0.15, 0.72, 0.12, "r"), aim=at(0, 0.5, -1, "r")), arm("l", 0.6, 0.0, -0.4, 0.4, 0.3, -0.3), sym("fingers", 20, 0, 0), shawl(10))
    keys = [(0, READY, DOWN), (6, crouch, {"pelvis": (0, 0, -0.20)}), (8, merge(crouch, arm("r", 0.4, 0.0, 0.85, 0.3, -0.1, 0.95)), {"pelvis": (0, 0, -0.20)}, "snap"),
            (9, plunge, {"pelvis": (0, -0.04, -0.36)}), (17, merge(plunge, {"spine_01": (42, 0, 0)}), {"pelvis": (0, -0.04, -0.37)}, "settle"), (26, READY, DOWN)]
    return _clip(armature, "w", 26, keys, markers={"fire": 9})


def e(armature):
    # Grave Pact (upper body, a pulse): a claw to her ribs, her head thrown back on `fire` 4,
    # held, a ragged breath.
    pact = merge(BENT, {"spine_02": (-10, 0, 0), "head": (-44, 0, 0)}, hand("l", at(0.06, 0.14, 1.40, "l"), aim=at(-0.6, 0.5, 0.2, "l")),
                 arm("r", 0.8, 0.0, 0.2, 0.7, -0.2, 0.4), sym("fingers", 70, 0, 0))
    keys = [(0, IDLE, None), (3, merge(pact, {"head": (-20, 0, 0)}), None, "snap"), (4, pact, None), (12, merge(pact, {"head": (-48, 0, 0)}), None, "settle"), (22, IDLE, None)]
    return _clip(armature, "e", 22, keys, layer="upper", markers={"fire": 4})


def r(armature):
    # Ossuary: both claws raised high and spread, cocked, then swept down and out on `fire` 8,
    # held low while the boneyard rises; a cackle.
    st = feet((0.20, 0.18), (0.20, -0.16))
    high = merge(st, {"spine_01": (0, 0, 0), "spine_02": (-14, 0, 0), "head": (-34, 0, 0)}, arm("l", 0.5, 0.0, 0.9, 0.4, -0.1, 1),
                 arm("r", 0.5, 0.0, 0.9, 0.4, -0.1, 1), sym("fingers", -15, 0, 0), shawl(4))
    sweep = merge(feet((0.22, 0.24), (0.22, -0.18)), {"spine_01": (30, 0, 0), "spine_02": (12, 0, 0), "head": (-24, 0, 0)},
                  arm("l", 0.9, 0.5, -0.5, 0.9, 0.5, -0.4), arm("r", 0.9, 0.5, -0.5, 0.9, 0.5, -0.4), sym("fingers", -10, 0, 0), shawl(10))
    keys = [(0, READY, DOWN), (5, high, {"pelvis": (0, 0, 0.03)}), (7, merge(high, {"spine_02": (-18, 0, 0)}), {"pelvis": (0, 0, 0.04)}, "snap"),
            (8, sweep, {"pelvis": (0, -0.03, -0.24)}), (16, merge(sweep, {"head": (-36, 0, 0)}), {"pelvis": (0, -0.03, -0.24)}),
            (20, merge(sweep, {"head": (-28, 0, 0)}), {"pelvis": (0, -0.03, -0.23)}, "settle"), (28, READY, DOWN)]
    return _clip(armature, "r", 28, keys, markers={"fire": 8})


KNEEL = merge({"ik_foot_l": ik(at(0.14, 0.26, champion.ankle(), "l")), "thigh_r": (8, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
              BENT, {"spine_01": (24, 0, 0)}, hand("l", at(0.25, 0.45, 0.15, "l"), aim=at(0, 0.5, -1, "l")), hand("r", at(0.15, 0.40, 0.15, "r"), aim=at(0, 0.5, -1, "r")),
              shawl(-4))


def recall(armature):
    # Crouches and scratches a circle into the ground with her claws.
    keys = [(0, IDLE, None), (15, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (30, merge(KNEEL, hand("r", at(0.35, 0.55, 0.15, "r"), aim=at(0, 0.5, -1, "r"))), {"pelvis": (0, 0.04, -0.45)}),
            (45, merge(KNEEL, hand("r", at(0.05, 0.62, 0.15, "r"), aim=at(0, 0.5, -1, "r"))), {"pelvis": (0, 0.04, -0.45)}),
            (60, merge(KNEEL, hand("r", at(-0.15, 0.45, 0.15, "r"), aim=at(0, 0.5, -1, "r"))), {"pelvis": (0, 0.04, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    knees = merge({"thigh_l": (-6, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (110, 0, 0), "calf_r": (110, 0, 0)}, BENT, CLAWS, {"spine_01": (30, 0, 0)}, shawl(-6))
    down = merge({"thigh_l": (-30, 0, 0), "thigh_r": (-14, 0, 0), "calf_l": (70, 0, 0), "calf_r": (40, 0, 0)}, arm("l", 0.8, 0.5, 0.0, 0.7, 0.6, 0.0),
                 arm("r", 0.6, 0.7, 0.1, 0.5, 0.8, 0.0), {"pelvis": (-70, -18, 0), "head": (20, 0, 16)}, sym("fingers", 70, 0, 0), shawl(-20))
    keys = [(0, IDLE, None), (8, merge(IDLE, {"spine_01": (-6, 0, 0), "head": (-40, 0, 0)}), {"pelvis": (0, 0.05, 0)}, "settle"),
            (16, knees, {"pelvis": (0, 0, -0.5)}, "snap"), (26, down, {"pelvis": (0, 0.44, -0.84)}, "snap"), (34, down, {"pelvis": (0, 0.45, -0.85)})]
    return _clip(armature, "death", 34, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "settle"), (14, merge(IDLE, {"head": (-30, 0, 0)}), {"pelvis": (0, 0, -0.06)}), (26, IDLE, None)]
    return _clip(armature, "respawn", 26, keys)


def select(armature):
    # Spreads her claws to frame her face and cackles, head thrown back.
    frame = merge(STAND, BENT, hand("l", at(0.18, 0.15, 1.75, "l"), aim=at(0.3, 0.3, 0.9, "l")), hand("r", at(0.18, 0.15, 1.75, "r"), aim=at(0.3, 0.3, 0.9, "r")),
                  sym("fingers", -20, 0, 0), shawl(4))
    keys = [(0, IDLE, None), (12, frame, None), (20, merge(frame, {"head": (-44, 0, 0)}), None), (24, merge(frame, {"head": (-36, 0, 0)}), None),
            (28, merge(frame, {"head": (-44, 0, 0)}), None), (40, IDLE, None)]
    return _clip(armature, "select", 40, keys)


def emote_taunt(armature):
    # Curls one long claw, beckoning, slowly.
    out = merge(IDLE, arm("r", 0.2, 1, 0.0, 0.1, 1, 0.1), {"fingers_r": (0, 0, 0)})
    curl = merge(out, {"fingers_r": (90, 0, 0)})
    keys = [(0, IDLE, None), (10, out, None), (18, curl, None), (26, out, None), (34, curl, None), (46, IDLE, None)]
    return _clip(armature, "emote_taunt", 46, keys)


def emote_joke(armature):
    # Holds the skull lantern up to her face and makes it "talk".
    talk = merge(IDLE, hand("l", at(0.12, 0.30, 1.50, "l"), aim=at(-0.3, 0.6, 0.6, "l")), {"head": (-14, 0, 20)})
    keys = [(0, IDLE, None), (12, talk, None)]
    for i, f in enumerate(range(16, 36, 4)):
        keys.append((f, merge(talk, {"fingers_l": (20 if i % 2 else 50, 0, 0)}), None, "snap"))
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
    # A crooked shuffle, claws wiggling overhead.
    up = merge(arm("l", 0.4, 0.2, 0.85, 0.3, 0.1, 0.95), arm("r", 0.4, 0.2, 0.85, 0.3, 0.1, 0.95))
    a = merge(up, BENT, feet((0.15, 0.08), (0.12, -0.08), lift_l=0.12), {"pelvis": (0, -8, 10)}, sym("fingers", 10, 0, 0), shawl(4, 8))
    b = merge(up, BENT, feet((0.12, 0.08), (0.15, -0.08), lift_r=0.12), {"pelvis": (0, 8, -10)}, sym("fingers", 60, 0, 0), shawl(4, -8))
    keys = [(0, a, {"pelvis": (0.03, 0, -0.04)}), (12, merge(up, BENT, STAND), {"pelvis": (0, 0, -0.07)}), (24, b, {"pelvis": (-0.03, 0, -0.04)}),
            (36, merge(up, BENT, STAND), {"pelvis": (0, 0, -0.07)})]
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    rig.add_chain(armature, "cloak", "chest", SHAWL)


def build(only=None):
    return champion.build("marrow", SHAPE, parts, OWN, 1.85, extras=_extras, only=only, shared=champion.SHARED + ["attack_melee_alt"], cloth={"cloak": dict(stiffness=0.3, inertia=0.84)})


if __name__ == "__main__":
    champion.main(build)
