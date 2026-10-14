# SPDX-License-Identifier: AGPL-3.0-or-later
"""Bastion (rebuilt for 14): the generator for his model and clips.

    blender -b --python art/champions/bastion/build.py -- --out art/champions/bastion/bastion.blend

A boulder (14 §5): a stone guardian whose shoulders stand above his head, on the `biped` bones
in hulking proportions (short stumpy legs, a huge chest, long arms ending in boulder fists). Ice
crystals grow from his back and shoulders and glow in his chest and visor: his signature is ice
blue (14 §4). A chain wound round his right fist ends in the grapple's hook.

Weight (14 §3): he stands hunched with his fists hanging low; blows wind up with the whole body,
land in 1–2 frames and hold; the stomps and slams leave his fists in the ground a beat. Feet are
planted by IK.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import champion, clips, rig  # noqa: E402
from mftr_blender.champion import arm, feet, hand  # noqa: E402
from mftr_blender.clips import at, merge, sym  # noqa: E402
from mftr_blender.library import _clip, gait  # noqa: E402
from mftr_blender.mesh import box_part, path_part  # noqa: E402

# Warm granite, so the ice reads against it.
STONE = (0.24, 0.22, 0.21)
STONE_LIGHT = (0.32, 0.30, 0.27)
STONE_DARK = (0.13, 0.12, 0.12)
MOSS = (0.16, 0.27, 0.10)
IRON = (0.11, 0.11, 0.13)
ICE = (0.36, 0.78, 1.0)
ICE_DEEP = (0.10, 0.40, 0.88)
ACCENT = (0.18, 0.52, 0.95)

# Hulking (14 §2.2): low hips and short legs, the neck below the shoulder line, very broad and
# deep, the shoulders pushed far out, long arms, huge hands.
SHAPE = {
    "heights": [(0.0, 0.0), (0.03, 0.03), (0.09, 0.09), (0.54, 0.42), (1.00, 0.84), (1.12, 0.98),
                (1.26, 1.14), (1.40, 1.32), (1.56, 1.52), (1.64, 1.47), (1.88, 1.70), (2.15, 1.98)],
    "x": 1.45, "y": 1.35, "scale": 1.12, "shoulder": 0.16, "arm": 1.22, "hand": 1.9,
}
rig.use_shape(SHAPE)


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        # Boulder hips, belly and the great chest, broader at the top.
        box_part("pelvis", sp((0, 0, 0.98)), (0.50, 0.42, 0.30), "cloth", STONE_DARK, bevel=0.35),
        ("pelvis", sp((0, 0, 1.00)), sp((0, 0, 1.06)), 8, [(0, .29, .25), (1, .29, .25)], "accent", ACCENT),
        box_part("spine_01", sp((0, 0, 1.18)), (0.58, 0.46, 0.30), "cloth", STONE, bevel=0.35),
        box_part("spine_02", sp((0, 0.02, 1.40)), (0.84, 0.62, 0.40), "cloth", STONE, taper=1.12, bevel=0.38),
        box_part("chest", sp((0, 0.04, 1.58)), (0.90, 0.62, 0.20), "cloth", STONE_LIGHT, taper=0.85, bevel=0.38),
        # The crystal core in his chest, set in an iron ring.
        path_part("chest", [sp((0, -0.26, 1.44)), sp((0, -0.38, 1.44)), sp((0, -0.48, 1.44))], 6,
                  [(.15, .19), (.13, .16), (0, 0)], "emissive", ICE, ref=(1, 0, 0)),
        path_part("chest", [sp((0, -0.25, 1.44)), sp((0, -0.30, 1.44))], 8, [(.19, .22), (.19, .22)], "metal", IRON, ref=(1, 0, 0)),
        # Rock lumps across the back and belly.
        box_part("spine_02", sp((0.22, 0.25, 1.30)), (0.20, 0.14, 0.18), "cloth", STONE_DARK, rot=(10, 20, 15), bevel=0.3),
        box_part("spine_02", sp((-0.20, 0.27, 1.36)), (0.18, 0.14, 0.20), "cloth", STONE_LIGHT, rot=(-12, -10, 25), bevel=0.3),
        box_part("spine_01", sp((0.10, -0.22, 1.14)), (0.20, 0.12, 0.14), "cloth", STONE_LIGHT, rot=(5, 15, -10), bevel=0.3),
        # Crystals growing from his back: the silhouette's spikes.
        path_part("chest", [sp((0.08, 0.22, 1.55)), sp((0.14, 0.42, 1.85)), sp((0.17, 0.50, 2.02))], 5,
                  [(.08, .07), (.05, .045), (0, 0)], "emissive", ICE_DEEP),
        path_part("chest", [sp((-0.12, 0.24, 1.50)), sp((-0.22, 0.46, 1.74)), sp((-0.28, 0.54, 1.86))], 5,
                  [(.07, .06), (.045, .04), (0, 0)], "emissive", ICE),
        path_part("spine_02", [sp((0.02, 0.26, 1.34)), sp((0.04, 0.50, 1.52)), sp((0.05, 0.60, 1.60))], 5,
                  [(.06, .05), (.035, .03), (0, 0)], "emissive", ICE_DEEP),
        path_part("chest", [sp((0.24, 0.20, 1.60)), sp((0.36, 0.38, 1.80)), sp((0.41, 0.44, 1.88))], 5,
                  [(.06, .05), (.035, .03), (0, 0)], "emissive", ICE),
        path_part("chest", [sp((-0.30, 0.18, 1.58)), sp((-0.42, 0.34, 1.82)), sp((-0.47, 0.40, 1.92))], 5,
                  [(.07, .06), (.04, .035), (0, 0)], "emissive", ICE_DEEP),
        path_part("spine_02", [sp((-0.18, 0.28, 1.28)), sp((-0.26, 0.50, 1.38)), sp((-0.29, 0.58, 1.42))], 5,
                  [(.05, .045), (.03, .025), (0, 0)], "emissive", ICE),
        box_part("pelvis", sp((0.18, -0.20, 0.92)), (0.16, 0.10, 0.14), "cloth", STONE_LIGHT, rot=(0, 10, 20), bevel=0.3),
        box_part("pelvis", sp((-0.16, -0.21, 0.94)), (0.15, 0.10, 0.13), "cloth", STONE, rot=(0, -12, -15), bevel=0.3),
        # Moss on the top of his back.
        box_part("chest", sp((0, 0.12, 1.69)), (0.60, 0.34, 0.05), "cloth", MOSS, bevel=0.4),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        # Boulder shoulders standing above his head, moss on top, a crystal from each.
        path_part("clavicle_l", [("clavicle_l", 0.4, (0, 0, 0.12)), ("clavicle_l", 1.0, (0.04, 0, 0.20)),
                                 ("clavicle_l", 1.0, (0.20, 0, 0.17)), ("clavicle_l", 1.0, (0.30, 0, 0.02)),
                                 ("clavicle_l", 1.0, (0.32, 0, -0.10))],
                  6, [(.24, .10), (.29, .16), (.30, .17), (.27, .13), (.20, .05)], "cloth", STONE_LIGHT, ref=(0, 1, 0)),
        box_part("clavicle_l", ("clavicle_l", 1.0, (0.10, 0.02, 0.33)), (0.30, 0.34, 0.06), "cloth", MOSS, rot=(0, -18, 0), bevel=0.4),
        path_part("clavicle_l", [("clavicle_l", 1.0, (0.12, 0.10, 0.30)), ("clavicle_l", 1.0, (0.20, 0.22, 0.56)),
                                 ("clavicle_l", 1.0, (0.24, 0.27, 0.70))], 6, [(.09, .08), (.05, .045), (0, 0)], "emissive", ICE),
        path_part("clavicle_l", [("clavicle_l", 1.0, (0.24, 0.04, 0.22)), ("clavicle_l", 1.0, (0.40, 0.10, 0.40)),
                                 ("clavicle_l", 1.0, (0.46, 0.12, 0.47))], 5, [(.06, .05), (.035, .03), (0, 0)], "emissive", ICE_DEEP),
        path_part("clavicle_l", [("clavicle_l", 1.0, (0.02, 0.16, 0.24)), ("clavicle_l", 1.0, (0.04, 0.34, 0.42)),
                                 ("clavicle_l", 1.0, (0.05, 0.40, 0.48))], 5, [(.05, .045), (.03, .025), (0, 0)], "emissive", ICE),
        # Arms: stacked stone, iron bands, the forearms swelling toward the fists.
        (ua, (ua, 0), (ua, 1), 10, [(0, .17, .16), (.5, .18, .17), (1, .15, .15)], "cloth", STONE),
        (fa, (fa, 0), (fa, 1), 10, [(0, .15, .15), (.6, .19, .18), (1, .2, .19)], "cloth", STONE_LIGHT),
        (fa, (fa, .30), (fa, .40), 8, [(0, .175, .17), (1, .18, .175)], "metal", IRON),
        (fa, (fa, .70), (fa, .78), 8, [(0, .205, .195), (1, .205, .195)], "metal", IRON),
        (fa, (fa, .55), (fa, .62), 4, [(0, .03, .2), (1, .03, .2)], "emissive", ICE),
        # Boulder fists.
        box_part("hand_l", ("hand_l", 0.45), (0.38, 0.33, 0.36), "cloth", STONE_DARK, bevel=0.35),
        box_part("fingers_l", ("fingers_l", 0.3), (0.36, 0.30, 0.20), "cloth", STONE, bevel=0.35),
        box_part("thumb_l", ("thumb_l", 0.5), (0.14, 0.14, 0.14), "cloth", STONE_DARK, bevel=0.3),
        box_part("fingers_l", ("fingers_l", 0.7, (0.09, -0.06, 0)), (0.10, 0.10, 0.10), "cloth", STONE_LIGHT, rot=(0, 20, 0), bevel=0.3),
        box_part("fingers_l", ("fingers_l", 0.7, (-0.01, -0.08, 0)), (0.10, 0.10, 0.10), "cloth", STONE_LIGHT, rot=(0, -10, 0), bevel=0.3),
        box_part("fingers_l", ("fingers_l", 0.7, (-0.10, -0.06, 0)), (0.10, 0.10, 0.10), "cloth", STONE_LIGHT, rot=(0, 10, 0), bevel=0.3),
        box_part(ua, (ua, 0.2, (-0.02, -0.08, 0.04)), (0.11, 0.11, 0.12), "cloth", STONE_DARK, rot=(25, 0, 20), bevel=0.3),
        # Rock lumps on the arms and a crystal on each forearm's back.
        box_part(ua, (ua, 0.45, (0.10, 0.05, 0.02)), (0.12, 0.14, 0.14), "cloth", STONE_LIGHT, rot=(20, 10, 30), bevel=0.3),
        box_part(fa, (fa, 0.35, (0.08, 0.10, 0.04)), (0.12, 0.12, 0.14), "cloth", STONE_DARK, rot=(-15, 25, 10), bevel=0.3),
        path_part(fa, [(fa, 0.5, (0.04, 0.14, 0.02)), (fa, 0.5, (0.10, 0.34, 0.12)), (fa, 0.5, (0.12, 0.42, 0.16))], 5,
                  [(.05, .04), (.03, .025), (0, 0)], "emissive", ICE),
        # Short stumpy legs and boulder feet.
        (th, sp((0.10, 0, 1.00)), (th, 1), 10, [(0, .20, .19), (.5, .19, .18), (1, .16, .16)], "cloth", STONE_DARK),
        box_part(ca, (ca, 0.02, (0, -0.07, 0)), (0.24, 0.12, 0.20), "cloth", STONE_LIGHT, bevel=0.35),
        box_part(th, (th, 0.4, (0.08, -0.04, 0)), (0.14, 0.14, 0.18), "cloth", STONE_LIGHT, rot=(10, 15, 25), bevel=0.3),
        box_part(ca, (ca, 0.55, (0.06, 0.06, 0)), (0.12, 0.12, 0.14), "cloth", STONE_DARK, rot=(-10, 20, 10), bevel=0.3),
        box_part(th, (th, 0.75, (-0.06, -0.08, 0)), (0.12, 0.12, 0.14), "cloth", STONE, rot=(15, -10, 20), bevel=0.3),
        box_part("foot_l", ("foot_l", 0.2, (0.10, 0.04, 0.06)), (0.12, 0.14, 0.12), "cloth", STONE_LIGHT, rot=(0, 15, 20), bevel=0.3),
        box_part("pelvis", sp((0.22, 0.06, 1.02)), (0.12, 0.18, 0.16), "cloth", STONE, rot=(0, 20, 0), bevel=0.3),
        (ca, (ca, 0), (ca, 1), 10, [(0, .16, .16), (.6, .17, .17), (1, .19, .18)], "cloth", STONE),
        box_part("foot_l", ("foot_l", 0.45), (0.28, 0.36, 0.18), "cloth", STONE_DARK, bevel=0.35),
        box_part("toe_l", ("toe_l", 0.3), (0.27, 0.14, 0.12), "cloth", STONE, bevel=0.35),
    ]


def _head():
    hb = "head"
    return [
        # A rough block head under a heavy brow, a glowing visor slit, moss on the crown.
        box_part(hb, sp((0, -0.06, 1.73)), (0.30, 0.30, 0.28), "cloth", STONE_LIGHT, taper=0.85, bevel=0.3),
        box_part(hb, sp((0, -0.18, 1.81)), (0.34, 0.11, 0.08), "cloth", STONE_DARK, bevel=0.3),
        box_part(hb, sp((0, -0.215, 1.755)), (0.26, 0.03, 0.05), "emissive", ICE),
        box_part(hb, sp((0, -0.05, 1.885)), (0.22, 0.24, 0.05), "cloth", MOSS, bevel=0.4),
        box_part("neck", sp((0, 0, 1.60)), (0.24, 0.24, 0.12), "cloth", STONE_DARK, bevel=0.3),
    ]


def _chain():
    """The grapple's chain wound round the right fist, its hook hanging from the knuckles."""
    hb = "hand_r"
    return [
        path_part(hb, [("hand_r", 0.15), ("hand_r", 0.32)], 8, [(.175, .17), (.175, .17)], "metal", IRON),
        path_part("forearm_r", [("forearm_r", 0.9), ("forearm_r", 1.0)], 8, [(.215, .205), (.215, .205)], "metal", IRON),
        path_part(hb, [("fingers_r", 0.9, (0, -0.05, -0.05)), ("fingers_r", 0.9, (0.0, -0.10, -0.20)),
                       ("fingers_r", 0.9, (-0.06, -0.16, -0.24)), ("fingers_r", 0.9, (-0.08, -0.20, -0.17))], 5,
                  [(.025, .025), (.03, .03), (.03, .03), (0, 0)], "metal", IRON),
    ]


def parts():
    return _center() + champion.mirrored(_left()) + _head() + _chain()


# --- Poses ----------------------------------------------------------------------------------

STAND_FEET = feet((0.30, 0.06), (0.30, -0.08))
HUNCH = {"spine_01": (14, 0, 0), "spine_02": (6, 0, 0), "chest": (4, 0, 0), "head": (-18, 0, 0)}


def fists_low(spread=0.62, fwd=0.20, up=0.80):
    """Both fists hanging low in front of his thighs, knuckles forward."""
    return merge(hand("l", at(spread, fwd, up, "l"), aim=at(0.1, 0.3, -1, "l")),
                 hand("r", at(spread, fwd, up, "r"), aim=at(0.1, 0.3, -1, "r")))


IDLE = merge(STAND_FEET, HUNCH, fists_low())
GUARD = merge(feet((0.32, 0.18), (0.32, -0.12)), HUNCH, {"spine_01": (18, 0, 0)},
              hand("l", at(0.30, 0.55, 1.45, "l"), aim=at(-0.2, 0.6, 0.6, "l")),
              hand("r", at(0.34, 0.35, 1.35, "r"), aim=at(-0.2, 0.6, 0.6, "r")))
GUARD_DOWN = {"pelvis": (0, 0, -0.10)}


def idle(armature):
    breathe = merge(IDLE, sym("clavicle", 0, -6, 0), {"chest": (0, 0, 0), "head": (-22, 0, 0)})
    return _clip(armature, "idle", 80, [(0, IDLE, {"pelvis": (0, 0, -0.03)}), (40, breathe, {"pelvis": (0, 0, -0.01)})], loop=True)


def idle_fidget_1(armature):
    # Rolls his great shoulders, one then the other, grinding stone.
    keys = [(0, IDLE, None), (14, merge(IDLE, {"clavicle_l": (0, -14, 0), "chest": (0, 6, 0)}), None),
            (28, merge(IDLE, {"clavicle_r": (0, 14, 0), "chest": (0, -6, 0)}), None), (42, merge(IDLE, sym("clavicle", 0, -10, 0)), {"pelvis": (0, 0, 0.02)}),
            (56, IDLE, None)]
    return _clip(armature, "idle_fidget_1", 56, keys)


def idle_fidget_2(armature):
    # Pounds his fists together twice; each blow lands and holds.
    meet = merge(STAND_FEET, HUNCH, hand("l", at(0.10, 0.62, 1.20, "l"), aim=at(-1, 0.2, 0, "l")),
                 hand("r", at(0.10, 0.62, 1.20, "r"), aim=at(-1, 0.2, 0, "r")))
    apart = merge(STAND_FEET, HUNCH, hand("l", at(0.62, 0.55, 1.30, "l"), aim=at(0.2, 0.6, 0.2, "l")),
                  hand("r", at(0.62, 0.55, 1.30, "r"), aim=at(0.2, 0.6, 0.2, "r")))
    keys = [(0, IDLE, None), (10, apart, None, "whip"), (13, meet, {"pelvis": (0, 0, -0.04)}), (18, meet, {"pelvis": (0, 0, -0.04)}),
            (24, apart, None, "whip"), (27, meet, {"pelvis": (0, 0, -0.05)}), (33, meet, {"pelvis": (0, 0, -0.05)}), (48, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 48, keys)


def idle_ready(armature):
    keys = [(0, GUARD, GUARD_DOWN), (20, merge(GUARD, {"spine_01": (21, 0, 0)}), {"pelvis": (0, 0, -0.12)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(swing):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(arm("l", 0.5, -0.5 * swing * c, -0.75, 0.2, 0.45 - 0.4 * swing * c, -0.65),
                     arm("r", 0.5, 0.5 * swing * c, -0.75, 0.2, 0.45 + 0.4 * swing * c, -0.65),
                     {"head": (-20, 0, 0), "chest": (4, 0, 0)})
    return upper


def run(armature):
    keys = gait(20, 1.0, contact=0.45, lift=0.18, bob=0.07, lean=16, twist=12, width=0.31, crouch=0.05, upper=_run_upper(1.0))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=335.0)


def run_fast(armature):
    keys = gait(16, 0.96, contact=0.40, lift=0.22, bob=0.08, lean=21, twist=14, width=0.31, crouch=0.06, upper=_run_upper(1.3))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(armature):
    # A right hook: wind back with the whole body, cock, the fist lands in 2 frames on `fire`
    # (sim: 14) and stays there, then a heavy return.
    st = feet((0.32, 0.20), (0.32, -0.16))
    wind = merge(st, HUNCH, {"pelvis": (0, 0, -22), "spine_01": (16, 0, -24), "spine_02": (6, 0, -14), "head": (-18, 0, 20)},
                 hand("r", at(0.75, -0.30, 1.25, "r"), aim=at(0.2, -0.6, 0.3, "r")), hand("l", at(0.35, 0.55, 1.35, "l"), aim=at(-0.2, 0.6, 0.6, "l")))
    cock = merge(wind, {"pelvis": (0, 0, -28), "spine_01": (16, 0, -32)}, hand("r", at(0.70, -0.50, 1.20, "r"), aim=at(0.3, -0.6, 0.2, "r")))
    hit = merge(feet((0.32, 0.34), (0.32, -0.22)), HUNCH, {"pelvis": (0, 0, 18), "spine_01": (20, 0, 26), "spine_02": (6, 0, 16), "head": (-16, 0, -20)},
                hand("r", at(0.05, 1.45, 1.10, "r"), aim=at(-0.1, 1, 0, "r")), hand("l", at(0.55, 0.0, 1.15, "l"), aim=at(0.1, -0.4, 0.4, "l")))
    keys = [(0, GUARD, GUARD_DOWN), (8, wind, {"pelvis": (0, 0.03, -0.12)}), (12, cock, {"pelvis": (0, 0.04, -0.13)}, "snap"),
            (14, hit, {"pelvis": (0, -0.08, -0.15)}), (20, merge(hit, {"spine_01": (22, 0, 28)}), {"pelvis": (0, -0.08, -0.16)}, "settle"),
            (32, GUARD, GUARD_DOWN)]
    return _clip(armature, "attack_1", 32, keys, markers={"fire": 14})


def attack_2(armature):
    # Both fists raised high, held, then hammered down in front on `fire` (sim: 14); they stay
    # in the ground a beat.
    st = feet((0.32, 0.18), (0.32, -0.12))
    up = merge(st, {"spine_01": (-4, 0, 0), "spine_02": (-10, 0, 0), "head": (-10, 0, 0)},
               hand("l", at(0.18, 0.15, 2.35, "l"), aim=at(-0.4, -0.2, 1, "l")), hand("r", at(0.18, 0.15, 2.35, "r"), aim=at(-0.4, -0.2, 1, "r")))
    cock = merge(up, {"spine_02": (-16, 0, 0)}, hand("l", at(0.16, -0.05, 2.3, "l"), aim=at(-0.4, -0.5, 0.8, "l")),
                 hand("r", at(0.16, -0.05, 2.3, "r"), aim=at(-0.4, -0.5, 0.8, "r")))
    hit = merge(feet((0.34, 0.30), (0.34, -0.16)), {"spine_01": (34, 0, 0), "spine_02": (14, 0, 0), "head": (-26, 0, 0)},
                hand("l", at(0.16, 1.20, 0.30, "l"), aim=at(-0.2, 0.5, -1, "l")), hand("r", at(0.16, 1.20, 0.30, "r"), aim=at(-0.2, 0.5, -1, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (7, up, {"pelvis": (0, 0.02, 0.03)}), (11, cock, {"pelvis": (0, 0.03, 0.02)}, "snap"),
            (14, hit, {"pelvis": (0, -0.10, -0.30)}), (21, merge(hit, {"spine_01": (36, 0, 0)}), {"pelvis": (0, -0.10, -0.31)}, "settle"),
            (32, GUARD, GUARD_DOWN)]
    return _clip(armature, "attack_2", 32, keys, markers={"fire": 14})


def q(armature):
    # Grapple: the hook swung back, then hurled down the line on `fire` (sim: 11), the arm
    # flung out after it; then he leans back and hauls the chain in hand over fist.
    st = feet((0.32, 0.22), (0.32, -0.18))
    wind = merge(st, HUNCH, {"pelvis": (0, 0, -18), "spine_01": (10, 0, -22), "head": (-14, 0, 18)},
                 hand("r", at(0.85, -0.55, 1.55, "r"), aim=at(0.3, -0.6, 0.6, "r")), hand("l", at(0.45, 0.6, 1.3, "l"), aim=at(0, 1, 0.2, "l")))
    throw = merge(feet((0.32, 0.34), (0.32, -0.22)), HUNCH, {"pelvis": (0, 0, 16), "spine_01": (20, 0, 20), "head": (-16, 0, -14)},
                  hand("r", at(0.15, 1.55, 1.25, "r"), aim=at(0, 1, -0.1, "r")), hand("l", at(0.6, -0.1, 1.1, "l"), aim=at(0.3, -0.5, 0, "l")))
    haul1 = merge(feet((0.32, 0.30), (0.32, -0.30)), {"spine_01": (-6, 0, 6), "spine_02": (-6, 0, 0), "head": (-6, 0, 0)},
                  hand("r", at(0.30, 0.35, 1.20, "r"), aim=at(0, 1, 0.2, "r")), hand("l", at(0.15, 0.95, 1.25, "l"), aim=at(-0.2, 1, 0, "l")))
    haul2 = merge(feet((0.32, 0.30), (0.32, -0.30)), {"spine_01": (-10, 0, -6), "spine_02": (-8, 0, 0), "head": (-4, 0, 0)},
                  hand("l", at(0.35, 0.30, 1.20, "l"), aim=at(0, 1, 0.2, "l")), hand("r", at(0.15, 0.95, 1.25, "r"), aim=at(-0.2, 1, 0, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (7, wind, {"pelvis": (0, 0.03, -0.10)}), (9, merge(wind, {"spine_01": (10, 0, -28)}), {"pelvis": (0, 0.04, -0.10)}, "snap"),
            (11, throw, {"pelvis": (0, -0.06, -0.13)}), (16, merge(throw, {"spine_01": (22, 0, 22)}), {"pelvis": (0, -0.06, -0.14)}, "settle"),
            (21, haul1, {"pelvis": (0, 0.08, -0.12)}), (26, haul2, {"pelvis": (0, 0.10, -0.12)}), (34, GUARD, GUARD_DOWN)]
    return _clip(armature, "q", 34, keys, markers={"fire": 11})


def w(armature):
    # Bulwark (upper body, a pulse): forearms slammed together across his face on `fire` 4,
    # held while the shield takes, then lowered.
    cross = merge(HUNCH, {"spine_01": (18, 0, 0)}, arm("l", 0.6, 0.6, -0.4, -0.75, 0.3, 0.6), arm("r", 0.6, 0.6, -0.35, -0.75, 0.3, 0.7),
                  sym("clavicle", 0, 8, 0))
    low = merge(HUNCH, arm("l", 0.5, 0.2, -0.85, 0.2, 0.4, -0.8), arm("r", 0.5, 0.2, -0.85, 0.2, 0.4, -0.8))
    keys = [(0, low, None),
            (2, merge(HUNCH, arm("l", 0.8, 0.2, -0.3, 0.3, 0.3, 0.7), arm("r", 0.8, 0.2, -0.3, 0.3, 0.3, 0.7)), None, "snap"),
            (4, cross, None), (14, merge(cross, {"spine_01": (20, 0, 0)}), None, "settle"), (24, low, None)]
    return _clip(armature, "w", 24, keys, layer="upper", markers={"fire": 4})


def e(armature):
    # Tremor: rears up with both fists high and the right knee raised, then stamps and drives
    # both fists into the ground on `fire` (sim: 8); everything stays planted a beat.
    rear = merge(feet((0.30, 0.10), (0.30, -0.02), lift_r=0.40), {"spine_02": (-12, 0, 0), "head": (-8, 0, 0)},
                 hand("l", at(0.40, 0.20, 2.25, "l"), aim=at(0, -0.2, 1, "l")), hand("r", at(0.40, 0.20, 2.25, "r"), aim=at(0, -0.2, 1, "r")))
    slam = merge(feet((0.36, 0.14), (0.36, -0.10)), {"spine_01": (40, 0, 0), "spine_02": (14, 0, 0), "head": (-30, 0, 0)},
                 hand("l", at(0.55, 0.80, 0.28, "l"), aim=at(0, 0.3, -1, "l")), hand("r", at(0.55, 0.80, 0.28, "r"), aim=at(0, 0.3, -1, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (6, rear, {"pelvis": (0, 0.02, 0.05)}, "snap"), (8, slam, {"pelvis": (0, -0.04, -0.34)}),
            (16, merge(slam, {"spine_01": (42, 0, 0)}), {"pelvis": (0, -0.04, -0.35)}, "settle"), (26, GUARD, GUARD_DOWN)]
    return _clip(armature, "e", 26, keys, markers={"fire": 8})


def r(armature):
    # Upheaval: both fists driven into the ground on `fire` (sim: 8), held, then he heaves
    # upward with his whole body as if lifting the earth that knocks them up.
    st = feet((0.36, 0.16), (0.36, -0.14))
    raise_ = merge(st, {"spine_02": (-14, 0, 0), "head": (-10, 0, 0)},
                   hand("l", at(0.30, 0.0, 2.35, "l"), aim=at(-0.2, -0.3, 1, "l")), hand("r", at(0.30, 0.0, 2.35, "r"), aim=at(-0.2, -0.3, 1, "r")))
    plant = merge(st, {"spine_01": (40, 0, 0), "spine_02": (16, 0, 0), "head": (-30, 0, 0)},
                  hand("l", at(0.45, 0.85, 0.28, "l"), aim=at(0, 0.3, -1, "l")), hand("r", at(0.45, 0.85, 0.28, "r"), aim=at(0, 0.3, -1, "r")))
    heave = merge(st, {"spine_01": (-8, 0, 0), "spine_02": (-16, 0, 0), "head": (-24, 0, 0)},
                  hand("l", at(0.70, 0.40, 2.10, "l"), aim=at(0.4, 0.2, 1, "l")), hand("r", at(0.70, 0.40, 2.10, "r"), aim=at(0.4, 0.2, 1, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (6, raise_, {"pelvis": (0, 0.02, 0.04)}, "snap"), (8, plant, {"pelvis": (0, -0.04, -0.36)}),
            (14, merge(plant, {"spine_01": (42, 0, 0)}), {"pelvis": (0, -0.04, -0.37)}, "whip"), (20, heave, {"pelvis": (0, 0, 0.04)}),
            (24, heave, {"pelvis": (0, 0, 0.04)}, "settle"), (34, GUARD, GUARD_DOWN)]
    return _clip(armature, "r", 34, keys, markers={"fire": 8})


KNEEL = merge({"ik_foot_l": clips.ik(at(0.30, 0.30, champion.ankle(), "l")), "thigh_r": (6, 0, 0), "calf_r": (100, 0, 0), "foot_r": (40, 0, 0)},
              {"spine_01": (26, 0, 0), "head": (6, 0, 0)},
              hand("l", at(0.55, 0.70, 0.28, "l"), aim=at(0, 0.3, -1, "l")), hand("r", at(0.45, 0.70, 0.28, "r"), aim=at(0, 0.3, -1, "r")))


def recall(armature):
    keys = [(0, IDLE, None), (15, KNEEL, {"pelvis": (0, 0.05, -0.44)}), (45, merge(KNEEL, sym("clavicle", 0, -6, 0)), {"pelvis": (0, 0.05, -0.42)}),
            (75, KNEEL, {"pelvis": (0, 0.05, -0.44)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    # His legs give way; he drops onto his fists, then topples forward like a felled wall.
    knees = merge({"thigh_l": (0, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (100, 0, 0), "calf_r": (100, 0, 0), "foot_l": (40, 0, 0), "foot_r": (40, 0, 0)},
                  {"spine_01": (30, 0, 0), "head": (-10, 0, 0)},
                  hand("l", at(0.55, 0.65, 0.28, "l"), aim=at(0, 0.3, -1, "l")), hand("r", at(0.55, 0.65, 0.28, "r"), aim=at(0, 0.3, -1, "r")))
    fall = merge({"thigh_l": (-14, 0, 0), "thigh_r": (-10, 0, 0), "calf_l": (24, 0, 0), "calf_r": (20, 0, 0)},
                 arm("l", 0.7, 0.6, 0.1, 0.6, 0.8, 0.0), arm("r", 0.7, 0.6, 0.1, 0.6, 0.8, 0.0), {"pelvis": (82, 0, 0), "head": (-20, 0, 0)})
    keys = [(0, IDLE, None), (8, merge(IDLE, {"spine_01": (-10, 0, 0), "head": (-30, 0, 0)}), {"pelvis": (0, 0.06, 0)}, "settle"),
            (17, knees, {"pelvis": (0, 0, -0.46)}, "snap"), (21, merge(knees, {"spine_01": (34, 0, 0)}), {"pelvis": (0, 0, -0.47)}),
            (30, fall, {"pelvis": (0, -0.70, -0.70)}, "snap"), (36, fall, {"pelvis": (0, -0.70, -0.72)})]
    return _clip(armature, "death", 36, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.05, -0.44)}, "settle"), (14, merge(IDLE, sym("clavicle", 0, -8, 0)), {"pelvis": (0, 0, -0.05)}), (26, IDLE, None)]
    return _clip(armature, "respawn", 26, keys)


def select(armature):
    # Beats his chest with both fists, then throws his arms wide and roars.
    beat_l = merge(STAND_FEET, HUNCH, hand("l", at(0.12, 0.38, 1.50, "l"), aim=at(-1, 0.1, 0, "l")), hand("r", at(0.6, 0.6, 1.3, "r"), aim=at(0, 1, 0.3, "r")))
    beat_r = merge(STAND_FEET, HUNCH, hand("r", at(0.12, 0.38, 1.50, "r"), aim=at(-1, 0.1, 0, "r")), hand("l", at(0.6, 0.6, 1.3, "l"), aim=at(0, 1, 0.3, "l")))
    wide = merge(STAND_FEET, {"spine_02": (-14, 0, 0), "head": (-28, 0, 0)}, arm("l", 1, 0.2, 0.35, 1, 0.3, 0.5), arm("r", 1, 0.2, 0.35, 1, 0.3, 0.5))
    keys = [(0, IDLE, None), (8, beat_l, None, "snap"), (11, beat_r, None, "snap"), (14, beat_l, None, "snap"), (17, beat_r, None),
            (26, wide, {"pelvis": (0, 0, 0.04)}), (40, merge(wide, {"head": (-34, 0, 0)}), {"pelvis": (0, 0, 0.04)}), (54, IDLE, None)]
    return _clip(armature, "select", 54, keys)


def emote_taunt(armature):
    # Pounds the ground in front of him with one fist, twice, staring.
    pound = merge(STAND_FEET, {"spine_01": (34, 0, 0), "head": (-30, 0, 0)}, hand("r", at(0.4, 0.75, 0.28, "r"), aim=at(0, 0.3, -1, "r")),
                  hand("l", at(0.6, 0.2, 0.85, "l"), aim=at(0.1, 0.3, -1, "l")))
    lift = merge(pound, hand("r", at(0.4, 0.55, 1.4, "r"), aim=at(0, 0.3, 1, "r")), {"spine_01": (24, 0, 0)})
    keys = [(0, IDLE, None), (10, lift, None, "whip"), (13, pound, {"pelvis": (0, 0, -0.12)}), (18, pound, {"pelvis": (0, 0, -0.12)}),
            (24, lift, None, "whip"), (27, pound, {"pelvis": (0, 0, -0.12)}), (34, pound, {"pelvis": (0, 0, -0.12)}), (46, IDLE, None)]
    return _clip(armature, "emote_taunt", 46, keys)


def emote_joke(armature):
    # A double-biceps flex; a crystal glints (the pose holds).
    flex = merge(STAND_FEET, {"spine_02": (-6, 0, 0), "head": (-10, 0, 0)}, arm("l", 1, 0.0, 0.1, 0.0, 0.1, 1), arm("r", 1, 0.0, 0.1, 0.0, 0.1, 1))
    keys = [(0, IDLE, None), (12, flex, None), (16, merge(flex, sym("clavicle", 0, -10, 0)), None, "snap"), (30, merge(flex, sym("clavicle", 0, -10, 0)), None),
            (34, flex, None), (50, IDLE, None)]
    return _clip(armature, "emote_joke", 50, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, None)]
    for i in range(1, 9):
        s = 1.0 if i % 2 else 0.5
        keys.append((4 * i, merge(IDLE, {"spine_01": (6 * s, 0, 0), "head": (-30 * s, 0, 0)}, sym("clavicle", 0, -8 * s, 0)), {"pelvis": (0, 0, -0.02 * s)}))
    keys.append((44, IDLE, None))
    return _clip(armature, "emote_laugh", 44, keys)


def emote_dance(armature):
    # A slow, ground-shaking side-to-side stomp, fists swinging.
    a = merge(feet((0.30, 0.06), (0.30, -0.08), lift_l=0.32), HUNCH, {"pelvis": (0, -8, 0)}, fists_low(0.7, 0.3, 0.9))
    b = merge(feet((0.30, 0.06), (0.30, -0.08), lift_r=0.32), HUNCH, {"pelvis": (0, 8, 0)}, fists_low(0.7, 0.3, 0.9))
    land = merge(STAND_FEET, HUNCH, fists_low(0.66, 0.25, 0.8))
    keys = [(0, a, {"pelvis": (0.05, 0, 0.0)}, "whip"), (10, land, {"pelvis": (0, 0, -0.08)}), (24, b, {"pelvis": (-0.05, 0, 0.0)}, "whip"),
            (34, land, {"pelvis": (0, 0, -0.08)})]
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def build(only=None):
    return champion.build("bastion", SHAPE, parts, OWN, 2.2, only=only)


if __name__ == "__main__":
    champion.main(build)
