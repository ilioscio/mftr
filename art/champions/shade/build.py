# SPDX-License-Identifier: AGPL-3.0-or-later
"""Shade (rebuilt for 14): the generator for his model and clips.

    blender -b --python art/champions/shade/build.py -- --out art/champions/shade/shade.blend

A blade (14 §5): a lean assassin, always hunched forward, under a tall pointed hood with violet
eyes burning out of a mask, a long scarf streaming behind him on an `extra_scarf` chain, and two
long curved knives held reversed along his forearms. His signature is violet (14 §4).

Weight (14 §3): a sneaking run, low and leaning, blades trailing; slashes coil, cock and cut in
a frame, the cut held; each of his three dashes ends in a strike or a skid that holds. Feet
planted by IK.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import champion, clips, rig  # noqa: E402
from mftr_blender.champion import arm, feet, spin  # noqa: E402
from mftr_blender.clips import at, merge, sym  # noqa: E402
from mftr_blender.library import _clip, gait  # noqa: E402
from mftr_blender.mesh import box_part, path_part  # noqa: E402

BLACK = (0.06, 0.055, 0.09)
INDIGO = (0.12, 0.09, 0.20)
VIOLET_CLOTH = (0.26, 0.10, 0.38)
WRAP = (0.24, 0.21, 0.28)
STEEL = (0.42, 0.44, 0.50)
STEEL_DARK = (0.20, 0.21, 0.25)
LEATHER = (0.18, 0.12, 0.10)
VIOLET = (0.78, 0.36, 1.0)
ACCENT = (0.18, 0.52, 0.95)

# Lean (14 §2.2): narrow, long arms, big hands; slightly shorter than standard.
SHAPE = {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 0.98, "y": 0.95, "scale": 0.95, "shoulder": 0.03, "arm": 1.08, "hand": 1.35}
rig.use_shape(SHAPE)
SCARF = [rig.shaped(p) for p in [(0, 0.10, 1.56), (0, 0.20, 1.32), (0, 0.27, 1.06), (0, 0.32, 0.78)]]


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", sp((0, 0, 0.92)), sp((0, 0, 1.12)), 12, [(0, .13, .10), (.5, .155, .11), (1, .145, .105)], "cloth", BLACK),
        ("pelvis", sp((0, 0, 1.04)), sp((0, 0, 1.10)), 8, [(0, .155, .112), (1, .15, .108)], "accent", ACCENT),
        # Long cloth flaps front and back, violet, split at the hips.
        box_part("pelvis", sp((0, -0.11, 0.80)), (0.20, 0.025, 0.42), "cloth", VIOLET_CLOTH, rot=(-8, 0, 0), taper=1.3),
        box_part("pelvis", sp((0, 0.11, 0.78)), (0.22, 0.025, 0.46), "cloth", INDIGO, rot=(10, 0, 0), taper=1.3),
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 12, [(0, .12, .09), (1, .125, .092)], "cloth", WRAP),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 12, [(0, .13, .095), (1, .15, .105)], "cloth", INDIGO),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.60)), 12, [(0, .155, .108), (.6, .16, .11), (1, .09, .075)], "cloth", INDIGO),
        box_part("pelvis", sp((0.13, -0.08, 1.01)), (0.07, 0.06, 0.09), "cloth", LEATHER, rot=(0, 0, 25), bevel=0.25),
        box_part("pelvis", sp((-0.14, -0.06, 1.01)), (0.07, 0.06, 0.09), "cloth", LEATHER, rot=(0, 0, -25), bevel=0.25),
        box_part("pelvis", sp((0.0, 0.13, 1.03)), (0.10, 0.05, 0.08), "cloth", LEATHER, bevel=0.25),
        # Two spare blades sheathed crossed on his back, hilts over the shoulders.
        box_part("spine_02", sp((0, 0.14, 1.36)), (0.05, 0.03, 0.50), "cloth", LEATHER, rot=(0, 30, 0), bevel=0.3),
        box_part("spine_02", sp((0, 0.15, 1.36)), (0.05, 0.03, 0.50), "cloth", LEATHER, rot=(0, -30, 0), bevel=0.3),
        path_part("spine_02", [sp((0.13, 0.15, 1.58)), sp((0.17, 0.16, 1.68))], 6, [(.02, .02), (.02, .02)], "metal", STEEL_DARK),
        path_part("spine_02", [sp((-0.13, 0.16, 1.58)), sp((-0.17, 0.17, 1.68))], 6, [(.02, .02), (.02, .02)], "metal", STEEL_DARK),
        # Crossed straps with throwing knives.
        box_part("chest", sp((0, -0.115, 1.38)), (0.05, 0.025, 0.42), "cloth", LEATHER, rot=(0, 34, 0)),
        box_part("chest", sp((0, -0.118, 1.38)), (0.05, 0.025, 0.42), "cloth", LEATHER, rot=(0, -34, 0)),
        box_part("chest", sp((0.07, -0.13, 1.45)), (0.022, 0.012, 0.10), "metal", STEEL, rot=(0, 34, 0)),
        box_part("chest", sp((-0.07, -0.13, 1.45)), (0.022, 0.012, 0.10), "metal", STEEL, rot=(0, -34, 0)),
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .055, .055), (1, .05, .05)], "cloth", BLACK),
        # The scarf: wound round his neck, its tail on the chain.
        ("chest", sp((0, 0.0, 1.53)), sp((0, -0.005, 1.62)), 8, [(0, .12, .11), (1, .10, .095)], "cloth", VIOLET_CLOTH),
        ("extra_scarf_1", SCARF[0], SCARF[1], 6, [(0, .075, .016), (1, .08, .016)], "cloth", VIOLET_CLOTH),
        ("extra_scarf_2", SCARF[1], SCARF[2], 6, [(0, .08, .016), (1, .08, .015)], "cloth", VIOLET_CLOTH),
        ("extra_scarf_3", SCARF[2], SCARF[3], 6, [(0, .08, .015), (.8, .075, .014), (1, .03, .012)], "cloth", VIOLET_CLOTH),
        ("extra_scarf_3", (SCARF[3][0], SCARF[3][1] + 0.004, SCARF[3][2] + 0.05), (SCARF[3][0], SCARF[3][1] + 0.005, SCARF[3][2] - 0.02), 4,
         [(0, .065, .02), (1, .02, .018)], "emissive", VIOLET),
        # A spiked steel guard on his right shoulder only.
        path_part("clavicle_r", [("clavicle_r", 0.5, (0, 0, 0.06)), ("clavicle_r", 1.0, (-0.03, 0, 0.09)),
                                 ("clavicle_r", 1.0, (-0.12, 0, 0.04)), ("clavicle_r", 1.0, (-0.14, 0, -0.05))],
                  6, [(.10, .035), (.12, .05), (.12, .05), (.10, .03)], "metal", STEEL_DARK, ref=(0, 1, 0)),
        path_part("clavicle_r", [("clavicle_r", 1.0, (-0.06, 0, 0.12)), ("clavicle_r", 1.0, (-0.10, 0.04, 0.26)), ("clavicle_r", 1.0, (-0.11, 0.05, 0.31))],
                  4, [(.04, .03), (.02, .015), (0, 0)], "metal", STEEL),
    ]


def _head():
    hb = "head"
    hood_tip = sp((0, 0.24, 2.02))
    return [
        # A small masked head inside a tall pointed hood that sweeps back into a point.
        box_part(hb, sp((0, -0.02, 1.72)), (0.17, 0.19, 0.21), "cloth", BLACK, taper=0.85, bevel=0.3),
        box_part(hb, sp((-0.04, -0.115, 1.745)), (0.05, 0.02, 0.018), "emissive", VIOLET, rot=(0, 0, -8)),
        box_part(hb, sp((0.04, -0.115, 1.745)), (0.05, 0.02, 0.018), "emissive", VIOLET, rot=(0, 0, 8)),
        path_part(hb, [sp((0, 0.0, 1.60)), sp((0, 0.0, 1.66)), sp((0, 0.0, 1.72)), sp((0, 0.02, 1.79)), sp((0, 0.05, 1.86)),
                       sp((0, 0.10, 1.92)), sp((0, 0.16, 1.97)), hood_tip], 10,
                  [(.13, .14), (.145, .155), (.15, .16), (.145, .155), (.12, .13), (.085, .095), (.045, .05), (0, 0)], "cloth", INDIGO, ref=(1, 0, 0)),
        # The hood's face opening, a dark rim.
        box_part(hb, sp((0, -0.13, 1.77)), (0.24, 0.04, 0.05), "cloth", BLACK, rot=(-20, 0, 0)),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 12, [(0, .06, .06), (1, .05, .05)], "cloth", INDIGO),
        (fa, (fa, 0), (fa, 1), 12, [(0, .05, .05), (1, .045, .045)], "cloth", WRAP),
        (fa, (fa, .35), (fa, .95), 12, [(0, .058, .058), (1, .052, .052)], "cloth", LEATHER),
        box_part("hand_l", ("hand_l", 0.45), (0.08, 0.07, 0.09), "cloth", BLACK, bevel=0.3),
        box_part("fingers_l", ("fingers_l", 0.4), (0.075, 0.06, 0.06), "cloth", BLACK, bevel=0.3),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .02, .02), (1, .015, .015)], "cloth", BLACK),
        (th, sp((0.10, 0, 1.02)), (th, 1), 12, [(0, .092, .09), (.5, .08, .08), (1, .06, .06)], "cloth", BLACK),
        (ca, (ca, 0), (ca, 1), 12, [(0, .062, .062), (.5, .056, .056), (1, .05, .05)], "cloth", WRAP),
        (ca, (ca, .55), (ca, .95), 12, [(0, .066, .066), (1, .058, .058)], "cloth", INDIGO),
        box_part("foot_l", ("foot_l", 0.45), (0.09, 0.16, 0.08), "cloth", BLACK, bevel=0.25),
        box_part(ca, (ca, 0.03, (0, -0.06, 0)), (0.09, 0.05, 0.10), "metal", STEEL_DARK, bevel=0.3),
        box_part(ca, (ca, 0.5, (0, -0.05, 0)), (0.07, 0.03, 0.22), "metal", STEEL_DARK, bevel=0.3),
        box_part(fa, (fa, 0.6, (0.03, -0.03, 0.02)), (0.05, 0.08, 0.20), "metal", STEEL_DARK, rot=(0, 30, 0), bevel=0.3),
        box_part(th, (th, 0.35, (0.06, -0.06, 0)), (0.07, 0.07, 0.10), "cloth", LEATHER, bevel=0.3),
        (th, (th, .52), (th, .60), 12, [(0, .088, .088), (1, .086, .086)], "cloth", LEATHER),
        box_part(th, (th, 0.56, (0.07, -0.05, 0.0)), (0.02, 0.015, 0.11), "metal", STEEL, rot=(0, 0, 0)),
        box_part(th, (th, 0.56, (0.085, -0.01, 0.0)), (0.02, 0.015, 0.11), "metal", STEEL, rot=(0, 0, 0)),
        box_part(th, (th, 0.56, (0.08, 0.03, 0.0)), (0.02, 0.015, 0.11), "metal", STEEL, rot=(0, 0, 0)),
        (fa, (fa, .90), (fa, 1.0), 12, [(0, .056, .056), (1, .056, .056)], "cloth", WRAP),
        (ua, (ua, .1), (ua, .2), 12, [(0, .066, .066), (1, .064, .064)], "cloth", WRAP),
        path_part("toe_l", [("toe_l", 0.0), ("toe_l", 0.8), ("toe_l", 1.4, (0, 0, 0.03))], 4, [(.045, .03), (.03, .02), (0, 0)], "cloth", BLACK),
    ]


def _knife(side):
    """A long curved knife held reversed: the blade leaves the bottom of the fist and runs back
    along the forearm, curving out, its edge glowing violet."""
    hb = f"hand_{side}"
    h = Vector(next(b[2] for b in rig.bone_table() if b[0] == hb))
    t = Vector(next(b[3] for b in rig.bone_table() if b[0] == hb))
    axis = (t - h).normalized()
    out = Vector((1 if side == "l" else -1, 0, 0))
    out = (out - axis * out.dot(axis)).normalized()
    g = h + axis * 0.05

    def p(v):
        return tuple(v)

    spine = [g + axis * 0.02 - out * 0.02, g - axis * 0.12 + out * 0.04, g - axis * 0.32 + out * 0.09, g - axis * 0.50 + out * 0.10,
             g - axis * 0.60 + out * 0.06]
    return [
        path_part(hb, [p(g + axis * 0.05), p(g - axis * 0.02)], 6, [(.03, .03), (.03, .03)], "cloth", LEATHER),
        path_part(hb, [p(g - axis * 0.02), p(g - axis * 0.04)], 6, [(.05, .02), (.05, .02)], "metal", STEEL_DARK, ref=tuple(out)),
        path_part(hb, [p(v) for v in spine], 6, [(.012, .05), (.012, .055), (.01, .045), (.008, .03), (0, 0)], "metal", STEEL, ref=tuple(out)),
        path_part(hb, [p(v + out * 0.045) for v in spine[1:]], 4, [(.006, .012), (.006, .012), (.005, .01), (0, 0)], "emissive", VIOLET, ref=tuple(out)),
    ]


def parts():
    return _center() + champion.mirrored(_left()) + _head() + _knife("l") + _knife("r")


# --- Poses ----------------------------------------------------------------------------------

def scarf(back=0.0, sway=0.0):
    return {"extra_scarf_1": (back * 0.5, sway * 0.5, 0), "extra_scarf_2": (back * 0.35, sway * 0.35, 0),
            "extra_scarf_3": (back * 0.3, sway * 0.3, 0)}


HUNCH = {"spine_01": (16, 0, 0), "spine_02": (8, 0, 0), "chest": (4, 0, 0), "head": (-20, 0, 0)}
STAND = feet((0.15, 0.10), (0.15, -0.10))
BLADES_LOW = merge(arm("l", 0.35, 0.25, -0.9, 0.15, 0.7, -0.6), arm("r", 0.35, 0.25, -0.9, 0.15, 0.7, -0.6))
IDLE = merge(STAND, HUNCH, BLADES_LOW, scarf(4))
READY = merge(feet((0.20, 0.18), (0.20, -0.18)), HUNCH, {"spine_01": (24, 0, 0)},
              arm("l", 0.35, 0.65, -0.35, -0.2, 0.85, 0.3), arm("r", 0.45, 0.3, -0.55, 0.2, 0.7, -0.2), scarf(8))
DOWN = {"pelvis": (0, 0, -0.14)}


def idle(armature):
    b = merge(IDLE, sym("clavicle", 0, -3, 0), {"head": (-24, 0, 8)}, scarf(6, 3))
    return _clip(armature, "idle", 64, [(0, IDLE, {"pelvis": (0, 0, -0.05)}), (32, b, {"pelvis": (0, 0, -0.04)})], loop=True)


def idle_fidget_1(armature):
    # Flips the right knife: a toss, a spin, a catch.
    up = merge(IDLE, arm("r", 0.4, 0.6, -0.2, 0.0, 0.5, 0.85), {"head": (-28, 0, -10)})
    keys = [(0, IDLE, None), (8, merge(up, arm("r", 0.4, 0.6, -0.4, 0.2, 0.7, 0.3)), None), (11, up, None, "snap"), (20, up, None),
            (23, merge(up, arm("r", 0.4, 0.6, -0.4, 0.2, 0.7, 0.3)), None, "snap"), (34, IDLE, None)]
    return _clip(armature, "idle_fidget_1", 34, keys)


def idle_fidget_2(armature):
    # Looks over his shoulder, still, then the other.
    keys = [(0, IDLE, None), (8, merge(IDLE, {"head": (-16, 0, 60), "spine_02": (8, 0, 12)}, scarf(4, -4)), None, "snap"),
            (24, merge(IDLE, {"head": (-16, 0, 62), "spine_02": (8, 0, 12)}), None), (30, merge(IDLE, {"head": (-16, 0, -60), "spine_02": (8, 0, -12)}, scarf(4, 4)), None, "snap"),
            (44, merge(IDLE, {"head": (-16, 0, -60)}), None), (54, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 54, keys)


def idle_ready(armature):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (27, 0, 0)}, scarf(10, 2)), {"pelvis": (0, 0, -0.16)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(trail):
    # The sneak (14 §3.5): arms swept back, blades trailing, the scarf streaming.
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(arm("l", 0.35, -0.75 - 0.1 * c, -0.45, 0.30, -0.55, 0.1), arm("r", 0.35, -0.75 + 0.1 * c, -0.45, 0.30, -0.55, 0.1),
                     {"head": (-34, 0, 0), "neck": (-8, 0, 0)}, scarf(trail + 8 * math.sin(4 * math.pi * p), 4 * math.sin(2 * math.pi * p)))
    return upper


def run(armature):
    keys = gait(20, 1.0, contact=0.42, lift=0.22, bob=0.04, lean=28, twist=7, width=0.15, crouch=0.14, upper=_run_upper(45))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=345.0)


def run_fast(armature):
    keys = gait(16, 0.98, contact=0.38, lift=0.26, bob=0.045, lean=34, twist=8, width=0.15, crouch=0.16, upper=_run_upper(55))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=460.0)


def _slash(armature, name, side):
    """A reverse-grip slash (14 §3.1): coil the cutting arm across the body, cock, cut through in
    one frame on `fire` (sim: 10), the cut held out, then back to ready."""
    other = "r" if side == "l" else "l"
    turn = 1 if side == "r" else -1
    st = feet((0.20, 0.24), (0.20, -0.18))
    coil = merge(st, HUNCH, {"spine_01": (20, 0, 26 * turn), "spine_02": (8, 0, 14 * turn), "head": (-22, 0, -16 * turn)},
                 arm(side, -0.1, 0.6, 0.1, -0.6, 0.2, 0.5), arm(other, 0.4, 0.2, -0.7, 0.2, 0.6, -0.3), scarf(6, 4 * turn))
    cock = merge(coil, {"spine_01": (20, 0, 34 * turn)}, arm(side, -0.2, 0.4, 0.2, -0.7, 0.0, 0.5))
    cut = merge(feet((0.20, 0.36), (0.20, -0.20)), HUNCH, {"spine_01": (24, 0, -26 * turn), "spine_02": (10, 0, -16 * turn), "head": (-22, 0, 10 * turn)},
                arm(side, 0.8, 0.55, -0.1, 0.85, 0.2, -0.1), arm(other, 0.3, -0.4, -0.6, 0.2, -0.2, -0.5), scarf(12, -6 * turn))
    keys = [(0, READY, DOWN), (6, coil, {"pelvis": (0, 0.03, -0.15)}), (9, cock, {"pelvis": (0, 0.04, -0.16)}, "snap"),
            (10, cut, {"pelvis": (0, -0.08, -0.20)}), (15, merge(cut, {"spine_01": (26, 0, -30 * turn)}), {"pelvis": (0, -0.08, -0.21)}, "settle"),
            (24, READY, DOWN)]
    return _clip(armature, name, 24, keys, markers={"fire": 10})


def attack_1(armature):
    return _slash(armature, "attack_1", "r")


def attack_2(armature):
    return _slash(armature, "attack_2", "l")


# Fan of Blades: crouched low, arms flung out with the knives, spinning.
FAN = merge(feet((0.24, 0.10), (0.24, -0.10)), {"spine_01": (30, 0, 0), "head": (-28, 0, 0)},
            arm("l", 1, 0.2, -0.1, 1, 0.3, 0.0), arm("r", 1, 0.2, -0.1, 1, 0.3, 0.0), scarf(30, 0))


def w(armature):
    # A crouch-spin: knives flung out, one full turn, the nova on `fire` 4, a held crouch.
    tuck = merge(feet((0.22, 0.08), (0.22, -0.08)), HUNCH, {"spine_01": (34, 0, 0)}, arm("l", 0.2, 0.8, -0.4, -0.6, 0.5, 0.2),
                 arm("r", 0.2, 0.8, -0.4, -0.6, 0.5, 0.2), scarf(6))
    low = {"pelvis": (0, 0, -0.32)}
    keys = [(0, READY, DOWN), (2, spin(tuck, -40), low, "snap"), (3, spin(FAN, 30), low, "snap"), (4, spin(FAN, 120), low, "snap"),
            (6, spin(FAN, 220), low, "snap"), (8, spin(FAN, 320), low), (10, spin(FAN, 360), low, "settle"), (14, spin(FAN, 360), {"pelvis": (0, 0, -0.30)}),
            (20, spin(READY, 360), DOWN)]
    return _clip(armature, "w", 20, keys, markers={"fire": 4})


STREAK = merge(arm("l", 0.35, -0.85, -0.3, 0.25, -0.7, 0.2), arm("r", 0.35, -0.85, -0.3, 0.25, -0.7, 0.2),
               sym("thigh", -30, 0, 0), sym("calf", 70, 0, 0), {"thigh_r": (20, 0, 0), "calf_r": (40, 0, 0)},
               {"spine_01": (40, 0, 0), "spine_02": (10, 0, 0), "head": (-36, 0, 0)}, scarf(70))
COIL = merge(feet((0.20, 0.12), (0.20, -0.20)), HUNCH, {"spine_01": (36, 0, 0)}, arm("l", 0.3, -0.5, -0.6, 0.2, -0.3, -0.5),
             arm("r", 0.3, -0.5, -0.6, 0.2, -0.3, -0.5), scarf(14))


def _trio(armature, slot, land_keys, travel=STREAK, land_frames=12):
    start = _clip(armature, f"{slot}_start", 5, [(0, COIL, {"pelvis": (0, 0.04, -0.30)}, "snap"), (2, merge(travel, scarf(40)), {"pelvis": (0, -0.1, -0.36)}),
                                                 (5, travel, {"pelvis": (0, -0.1, -0.36)})], markers={"fire": 2})
    loop = _clip(armature, f"{slot}_travel", 10, [(0, travel, {"pelvis": (0, -0.1, -0.36)}),
                                                  (5, merge(travel, scarf(80, 6)), {"pelvis": (0, -0.1, -0.33)})], loop=True)
    land = _clip(armature, f"{slot}_land", land_frames, land_keys)
    return start, loop, land


def q_start(armature):
    # Shadow Step: a streak onto the target, a cross-slash on arrival held a beat.
    cross = merge(feet((0.22, 0.30), (0.22, -0.20)), HUNCH, {"spine_01": (30, 0, 0)}, arm("l", 0.7, 0.7, -0.3, 0.9, 0.2, -0.3),
                  arm("r", 0.7, 0.7, -0.3, 0.9, 0.2, -0.3), scarf(20))
    out = _trio(armature, "q", [(0, merge(STREAK, {"spine_01": (30, 0, 0)}), {"pelvis": (0, -0.05, -0.30)}, "snap"), (2, cross, {"pelvis": (0, -0.05, -0.28)}),
                                (7, merge(cross, scarf(12)), {"pelvis": (0, -0.05, -0.27)}, "settle"), (12, READY, DOWN)])
    return out[0]


def e_start(armature):
    # Veil Step: a low glide out, landing in a long skid that holds.
    glide = merge(arm("l", 0.6, 0.2, -0.4, 0.5, 0.3, -0.3), arm("r", 0.6, 0.2, -0.4, 0.5, 0.3, -0.3), sym("thigh", -40, 0, 0), sym("calf", 80, 0, 0),
                  {"spine_01": (30, 0, 0), "head": (-30, 0, 0)}, scarf(60))
    skid = merge(feet((0.30, 0.25), (0.25, -0.30)), {"spine_01": (24, 0, 0), "head": (-26, 0, 0)}, arm("l", 0.9, 0.3, -0.3, 0.8, 0.4, -0.2),
                 arm("r", 0.6, -0.2, -0.7, 0.4, 0.1, -0.7), scarf(30, 10))
    out = _trio(armature, "e", [(0, skid, {"pelvis": (0, 0, -0.42)}, "settle"), (6, merge(skid, scarf(16, 6)), {"pelvis": (0, 0, -0.40)}), (12, READY, DOWN)],
                travel=glide)
    return out[0]


def r_start(armature):
    # Execution: a leap with both knives raised, plunged down on arrival and held there long.
    high = merge(arm("l", 0.3, 0.2, 0.9, 0.2, 0.1, 1), arm("r", 0.3, 0.2, 0.9, 0.2, 0.1, 1), sym("thigh", -60, 0, 0), sym("calf", 90, 0, 0),
                 {"spine_01": (10, 0, 0), "spine_02": (-10, 0, 0), "head": (-16, 0, 0)}, scarf(70))
    stab = merge(feet((0.24, 0.24), (0.24, -0.22)), {"spine_01": (44, 0, 0), "spine_02": (12, 0, 0), "head": (-34, 0, 0)},
                 arm("l", 0.2, 0.8, -0.6, 0.0, 0.5, -0.9), arm("r", 0.2, 0.8, -0.6, 0.0, 0.5, -0.9), scarf(20))
    out = _trio(armature, "r", [(0, merge(high, {"spine_01": (24, 0, 0)}), {"pelvis": (0, 0, -0.20)}, "snap"), (2, stab, {"pelvis": (0, -0.04, -0.46)}),
                                (9, merge(stab, {"spine_01": (46, 0, 0)}), {"pelvis": (0, -0.04, -0.47)}, "settle"), (16, READY, DOWN)],
                travel=high, land_frames=16)
    return out[0]


KNEEL = merge({"ik_foot_l": clips.ik(at(0.16, 0.26, champion.ankle(), "l")), "thigh_r": (8, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
              BLADES_LOW, {"spine_01": (24, 0, 0), "head": (10, 0, 0)}, scarf(-6))


def recall(armature):
    keys = [(0, IDLE, None), (15, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (45, merge(KNEEL, {"head": (14, 0, 0)}), {"pelvis": (0, 0.04, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    hit = merge(feet((0.15, -0.02), (0.15, -0.22)), arm("l", 0.6, -0.2, -0.3, 0.5, 0.1, -0.5), arm("r", 0.6, -0.2, -0.3, 0.5, 0.1, -0.5),
                {"spine_01": (-16, 0, 0), "head": (-30, 0, 0)}, scarf(-30))
    knees = merge({"thigh_l": (-6, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (110, 0, 0), "calf_r": (110, 0, 0)}, BLADES_LOW,
                  {"spine_01": (30, 0, 0), "head": (16, 0, 0)}, scarf(-10))
    down = merge({"thigh_l": (-14, 0, 0), "thigh_r": (-10, 0, 0), "calf_l": (30, 0, 0), "calf_r": (20, 0, 0)},
                 arm("l", 0.8, 0.5, 0.0, 0.7, 0.6, 0.0), arm("r", 0.6, 0.7, 0.1, 0.5, 0.8, 0.0), {"pelvis": (84, 0, 0), "head": (-20, 0, 12)}, scarf(20))
    keys = [(0, IDLE, None), (6, hit, {"pelvis": (0, 0.06, -0.02)}, "settle"), (14, knees, {"pelvis": (0, 0, -0.5)}, "snap"),
            (24, down, {"pelvis": (0, -0.62, -0.84)}, "snap"), (34, down, {"pelvis": (0, -0.63, -0.85)})]
    return _clip(armature, "death", 34, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "settle"), (12, merge(IDLE, {"head": (-28, 0, 0)}), {"pelvis": (0, 0, -0.06)}), (24, IDLE, None)]
    return _clip(armature, "respawn", 24, keys)


def select(armature):
    # Crosses the knives in front of his mask, eyes burning over them, then flicks them apart.
    cross = merge(feet((0.18, 0.12), (0.18, -0.16)), HUNCH, arm("l", 0.1, 0.8, 0.2, -0.6, 0.4, 0.7), arm("r", 0.1, 0.8, 0.2, -0.6, 0.4, 0.7), scarf(8))
    apart = merge(feet((0.18, 0.12), (0.18, -0.16)), HUNCH, arm("l", 1, 0.3, -0.2, 0.9, 0.4, -0.2), arm("r", 1, 0.3, -0.2, 0.9, 0.4, -0.2), scarf(14))
    keys = [(0, IDLE, None), (12, cross, {"pelvis": (0, 0, -0.08)}), (28, merge(cross, {"head": (-14, 0, 0)}), {"pelvis": (0, 0, -0.08)}),
            (30, apart, {"pelvis": (0, 0, -0.12)}, "snap"), (40, merge(apart, scarf(8, 4)), {"pelvis": (0, 0, -0.12)}), (52, IDLE, None)]
    return _clip(armature, "select", 52, keys)


def emote_taunt(armature):
    # Draws a thumb across his throat.
    t = merge(IDLE, arm("r", 0.1, 0.7, 0.2, -0.9, 0.2, 0.3), {"head": (-12, 0, 0)})
    t2 = merge(IDLE, arm("r", 0.5, 0.6, 0.2, 0.6, 0.6, 0.3), {"head": (-12, 0, 0)})
    keys = [(0, IDLE, None), (10, t, None), (14, t2, None, "snap"), (26, t2, None), (36, IDLE, None)]
    return _clip(armature, "emote_taunt", 36, keys)


def emote_joke(armature):
    # Pretends to vanish: crouches behind his raised scarf, peeks out.
    hide = merge(feet((0.18, 0.06), (0.18, -0.06)), {"spine_01": (40, 0, 0), "head": (20, 0, 0)}, arm("l", 0.2, 0.7, 0.3, -0.5, 0.3, 0.8),
                 arm("r", 0.2, 0.7, 0.3, -0.5, 0.3, 0.8), scarf(-20))
    keys = [(0, IDLE, None), (10, hide, {"pelvis": (0, 0, -0.4)}, "snap"), (26, hide, {"pelvis": (0, 0, -0.4)}),
            (32, merge(hide, {"head": (-10, 0, 30)}), {"pelvis": (0, 0, -0.38)}, "snap"), (44, IDLE, None)]
    return _clip(armature, "emote_joke", 44, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, None)]
    for i in range(1, 7):
        s = 1.0 if i % 2 else 0.4
        keys.append((4 * i, merge(IDLE, {"head": (-26 - 6 * s, 0, 0)}, sym("clavicle", 0, -5 * s, 0)), None))
    keys.append((34, IDLE, None))
    return _clip(armature, "emote_laugh", 34, keys)


def emote_dance(armature):
    # A low, sliding shuffle, knives flicked out on the beat.
    a = merge(feet((0.22, 0.08), (0.14, -0.08)), HUNCH, {"pelvis": (0, -6, 12)}, arm("l", 1, 0.2, -0.2, 0.9, 0.3, -0.1), BLADES_LOW, scarf(10, 8))
    b = merge(feet((0.14, 0.08), (0.22, -0.08)), HUNCH, {"pelvis": (0, 6, -12)}, arm("r", 1, 0.2, -0.2, 0.9, 0.3, -0.1), scarf(10, -8),
              arm("l", 0.35, 0.25, -0.9, 0.15, 0.7, -0.6))
    keys = [(0, a, {"pelvis": (0.05, 0, -0.12)}, "snap"), (12, a, {"pelvis": (0.05, 0, -0.10)}), (24, b, {"pelvis": (-0.05, 0, -0.12)}, "snap"),
            (36, b, {"pelvis": (-0.05, 0, -0.10)})]
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, w, q_start, e_start, r_start,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    rig.add_chain(armature, "scarf", "chest", SCARF)


def build(only=None):
    return champion.build("shade", SHAPE, parts, OWN, 1.8, extras=_extras, only=only)


if __name__ == "__main__":
    champion.main(build)
