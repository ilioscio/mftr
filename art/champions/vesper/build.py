# SPDX-License-Identifier: AGPL-3.0-or-later
"""Vesper (A4a): the block-out generator for her model and clips.

    blender -b --python art/champions/vesper/build.py -- --out art/champions/vesper/vesper.blend

Like the shared library, this is a *starting point*: run once, then the .blend is the source of
truth and humans polish it. Re-running overwrites it. Pose values follow `mftr_blender.clips`
(arms aimed in character space; everything else in degrees about the armature axes). Her left
hand holds the bow.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

from mftr_blender import clips, mesh, rig  # noqa: E402
from mftr_blender.clips import merge, sym  # noqa: E402
from mftr_blender.library import _clip, arms, cycle, run_half  # noqa: E402

# Palette (concept.md).
SKIN = (0.84, 0.66, 0.54)
TEAL = (0.14, 0.32, 0.30)
SLATE = (0.16, 0.17, 0.20)
SLEEVE = (0.20, 0.24, 0.22)
LEATHER = (0.42, 0.27, 0.16)
BOOT = (0.30, 0.19, 0.11)
GLOVE = (0.25, 0.16, 0.10)
WOOD = (0.36, 0.21, 0.10)
SILVER = (0.74, 0.76, 0.80)
STRING = (0.82, 0.78, 0.66)
ACCENT = (0.18, 0.52, 0.95)
EYES = (0.08, 0.08, 0.10)

CAPE = [(0, 0.15, 1.50), (0, 0.19, 1.18), (0, 0.22, 0.86), (0, 0.24, 0.56)]


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", (0, 0, 0.92), (0, 0, 1.12), 12, [(0, .12, .09), (.4, .145, .10), (.8, .15, .105), (1, .14, .10)], "cloth", SLATE),
        ("pelvis", (0, 0, 1.055), (0, 0, 1.115), 12, [(0, .158, .112), (1, .158, .112)], "cloth", LEATHER),
        ("pelvis", (0, -0.11, 1.07), (0, -0.135, 1.10), 4, [(0, .03, .018), (1, .03, .018)], "metal", SILVER),
        ("spine_01", (0, 0, 1.10), (0, 0, 1.28), 12, [(0, .138, .098), (.5, .142, .10), (1, .15, .105)], "cloth", LEATHER),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .15, .105), (1, .165, .112)], "cloth", LEATHER),
        ("chest", (0, 0, 1.40), (0, 0, 1.60), 12, [(0, .165, .112), (.35, .175, .118), (.7, .163, .11), (1, .09, .075)], "cloth", LEATHER),
        # The mantle over the shoulders, part of the cloak.
        ("chest", (0, 0.015, 1.47), (0, 0.005, 1.63), 14, [(0, .20, .14), (.55, .17, .125), (1, .075, .068)], "cloth", TEAL),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.66), 8, [(0, .05, .05), (1, .045, .045)], "skin", SKIN),
        ("head", (0, -0.01, 1.62), (0, -0.01, 1.88), 12, [(0, .055, .065), (.15, .085, .095), (.45, .097, .105), (.75, .094, .10), (.92, .07, .075), (1, 0, 0)], "skin", SKIN),
        # The hood: set back so the face shows, swept to a point behind the head.
        ("head", (0, 0.055, 1.63), (0, 0.16, 2.03), 14, [(0, .118, .125), (.25, .122, .13), (.55, .105, .11), (.8, .06, .065), (1, 0, 0)], "cloth", TEAL),
        ("head", (0, -0.095, 1.73), (0, -0.122, 1.718), 4, [(0, .014, .018), (1, 0, 0)], "skin", SKIN),
        # Quiver on the back, its fletching in the team accent.
        ("spine_02", (0.07, 0.13, 1.18), (-0.05, 0.15, 1.60), 6, [(0, .048, .038), (1, .054, .042)], "cloth", LEATHER),
        ("spine_02", (-0.05, 0.15, 1.60), (-0.075, 0.16, 1.72), 6, [(0, .044, .034), (1, .008, .008)], "accent", ACCENT),
        # The cloak tail on the cape chain.
        ("extra_cape_1", CAPE[0], CAPE[1], 6, [(0, .17, .022), (1, .20, .022)], "cloth", TEAL),
        ("extra_cape_2", CAPE[1], CAPE[2], 6, [(0, .20, .022), (1, .215, .022)], "cloth", TEAL),
        ("extra_cape_3", CAPE[2], CAPE[3], 6, [(0, .215, .022), (.7, .20, .02), (1, .12, .016)], "cloth", TEAL),
    ] + [
        ("head", (x, -0.093, 1.762), (x, -0.108, 1.762), 4, [(0, .014, .007), (1, .014, .007)], "cloth", EYES)
        for x in (0.034, -0.034)
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (0.15, 0.01, 1.47), (0.205, 0.01, 1.585), 8, [(0, .078, .078), (.5, .082, .078), (1, 0, 0)], "cloth", TEAL),
        (ua, (ua, 0), (ua, 1), 8, [(0, .056, .056), (1, .046, .046)], "cloth", SLEEVE),
        (fa, (fa, 0), (fa, 1), 8, [(0, .044, .044), (1, .036, .036)], "cloth", SLEEVE),
        (fa, (fa, .35), (fa, .97), 8, [(0, .052, .052), (1, .044, .044)], "accent", ACCENT),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .038, .024), (1, .042, .021)], "cloth", GLOVE),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .042, .021), (1, .028, .015)], "cloth", GLOVE),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .017, .017), (1, .011, .011)], "cloth", GLOVE),
        (th, (0.10, 0, 1.02), (th, 1), 10, [(0, .08, .08), (.5, .072, .072), (1, .056, .056)], "cloth", SLATE),
        (ca, (ca, 0), (ca, .45), 10, [(0, .056, .056), (1, .052, .052)], "cloth", SLATE),
        (ca, (ca, .38), (ca, 1), 10, [(0, .064, .064), (.12, .06, .06), (1, .05, .054)], "cloth", BOOT),
        (ca, (ca, .36), (ca, .44), 10, [(0, .07, .07), (1, .07, .07)], "cloth", BOOT),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .052, .046), (1, .046, .032)], "cloth", BOOT),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .046, .028), (1, .03, .018)], "cloth", BOOT),
    ]


def _bow(axis, back):
    """A 1.5 m recurve weighted to the left hand. `axis` and `back` are the rest-pose directions
    that become *up* and *toward her* at full draw (measured from `attack_1`, see `bow_frame`),
    so the bow stands vertical with its belly toward her when she shoots."""
    h = next(Vector(b[2]) for b in rig.bone_table() if b[0] == "hand_l")
    t = next(Vector(b[3]) for b in rig.bone_table() if b[0] == "hand_l")
    grip = h + (t - h).normalized() * 0.05
    half = 0.74

    def at(s):
        u = abs(s) / half
        return tuple(grip + axis * s + back * (0.13 * u * u - 0.07 * u ** 6))

    def radius(s):
        return 0.03 - 0.018 * abs(s) / half

    stations = [-0.74, -0.6, -0.42, -0.22, -0.08, 0.08, 0.22, 0.42, 0.6, 0.74]
    parts = []
    for a, b in zip(stations, stations[1:]):
        mid = abs((a + b) / 2)
        slot, color = ("metal", SILVER) if mid > 0.6 else ("accent", ACCENT) if mid < 0.08 else ("cloth", WOOD)
        parts.append(("hand_l", at(a), at(b), 6, [(0, radius(a), radius(a)), (1, radius(b), radius(b))], slot, color))
    parts.append(("hand_l", at(-half), at(half), 4, [(0, .0045, .0045), (1, .0045, .0045)], "cloth", STRING))
    return parts


def parts(axis, back):
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + _bow(axis, back)


def bow_frame(arm):
    """Rest-pose directions for the bow: whatever the left hand turns into *up* and *backward*
    at the full-draw frame of `attack_1`."""
    arm.animation_data.action = bpy.data.actions["attack_1"]
    fire = next(m.frame for m in bpy.data.actions["attack_1"].pose_markers if m.name == "fire")
    bpy.context.scene.frame_set(int(fire) - 1)
    bpy.context.view_layer.update()
    pb = arm.pose.bones["hand_l"]
    delta = pb.matrix.to_quaternion() @ pb.bone.matrix_local.to_quaternion().inverted()
    inv = delta.inverted()
    arm.animation_data.action = None
    clips.reset_pose(arm)
    bpy.context.scene.frame_set(0)
    axis = (inv @ Vector((0, 0, 1))).normalized()
    back = inv @ Vector((0, 1, 0))
    back = (back - axis * back.dot(axis)).normalized()
    return axis, back


# --- Poses ----------------------------------------------------------------------------------

def cape(back=0.0, sway=0.0):
    """The cloak: `back` swings the tail behind her (degrees), `sway` to her left."""
    return {"extra_cape_1": (back * 0.5, sway * 0.5, 0), "extra_cape_2": (back * 0.3, sway * 0.3, 0),
            "extra_cape_3": (back * 0.25, sway * 0.25, 0)}


BOW_LOW = merge(arms((0.25, 0.15, -1), (0.15, 0.55, -0.8), (0.25, 0.0, -1), (0.2, 0.2, -1)))
STAND = merge(sym("thigh", -3, 0, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"pelvis": (0, 3, 0), "spine_01": (2, 0, 0)})
CROUCH = merge(sym("thigh", -14, 0, 0), sym("calf", 26, 0, 0), sym("foot", -12, 0, 0), {"spine_01": (8, 0, 0), "spine_02": (2, 0, 0)})
READY = merge(CROUCH, arms((0.15, 0.75, -0.55), (0.05, 1, -0.15), (0.3, 0.3, -0.8), (-0.2, 0.85, -0.3)), cape(4))
# Draw stance: torso turned right so the bow shoulder leads, head toward the target.
TURN = {"spine_01": (4, 0, -14), "spine_02": (2, 0, -16), "chest": (0, 0, -10), "neck": (0, 0, 18), "head": (-2, 0, 20)}
AIM_L = {"upperarm_l": clips.aim(0.02, 1, 0.04), "forearm_l": clips.aim(0.0, 1, 0.05)}
DRAW_R = {"upperarm_r": clips.aim(0.3, -0.75, 0.15, "r"), "forearm_r": clips.aim(-0.75, 0.65, 0.1, "r")}
NOCK_R = {"upperarm_r": clips.aim(0.3, 0.5, -0.7, "r"), "forearm_r": clips.aim(-0.35, 0.9, -0.1, "r")}
RELEASE_R = {"upperarm_r": clips.aim(0.35, -0.85, 0.2, "r"), "forearm_r": clips.aim(0.2, -0.6, 0.3, "r")}
DOWN = {"pelvis": (0, 0, -0.05)}


def bow_arm(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


def bow_arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, BOW_LOW, cape(0), {"head": (-2, 0, 0)})
    b = merge(STAND, BOW_LOW, cape(1.5, 2), sym("clavicle", 0, -3, 0), {"head": (-3, 0, 6), "chest": (-2, 0, 0)})
    return _clip(arm, "idle", 72, [(0, a, None), (36, b, {"pelvis": (0, 0, 0.005)})], loop=True)


def idle_fidget_1(arm):
    # Checks the string: bow up across the body, two plucks.
    base = merge(STAND, BOW_LOW, cape(0))
    up = merge(STAND, bow_arm(0.15, 0.9, -0.1, -0.25, 1, 0.05), bow_arm_r(0.2, 0.6, -0.6, -0.45, 0.9, 0.2), {"head": (12, 0, 8)}, cape(1))
    pluck = merge(up, bow_arm_r(0.2, 0.6, -0.6, -0.3, 0.7, 0.5))
    keys = [(0, base, None), (12, up, None), (24, pluck, None), (28, up, None), (32, pluck, None), (36, up, None),
            (46, merge(STAND, BOW_LOW, {"head": (4, 0, 4)}), None), (54, base, None)]
    return _clip(arm, "idle_fidget_1", 54, keys)


def idle_fidget_2(arm):
    # Scans the horizon, a hand shading her eyes.
    base = merge(STAND, BOW_LOW, cape(0))
    shade = bow_arm_r(0.25, 0.6, 0.5, -0.6, 0.4, 0.6)
    keys = [(0, base, None), (14, merge(STAND, BOW_LOW, shade, {"head": (-6, 0, 25), "spine_02": (0, 0, 6)}, cape(0, -3)), None),
            (32, merge(STAND, BOW_LOW, shade, {"head": (-6, 0, -25), "spine_02": (0, 0, -8)}, cape(0, 3)), None),
            (46, merge(STAND, BOW_LOW, {"head": (-2, 0, 0)}), None), (60, base, None)]
    return _clip(arm, "idle_fidget_2", 60, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (9, 0, 0)}, cape(5, 1)), {"pelvis": (0, 0, -0.06)})]
    return _clip(arm, "idle_ready", 40, keys, loop=True)


def _with(keys, extra):
    return [(f, merge(p, extra(f) if callable(extra) else extra), l) for f, p, l in keys]


RUN_BOW = bow_arm(0.22, -0.15, -1, 0.18, 0.45, -0.85)


def run(arm):
    keys = _with(cycle(run_half(), 20), lambda f: merge(RUN_BOW, cape(25 + (4 if f % 10 < 5 else 0))))
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(arm):
    # Haste: longer strides (×1.2 leg swing), more lean, 16 frames per cycle.
    def scale(pose):
        out = dict(pose)
        for k, v in pose.items():
            if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple):
                out[k] = (v[0] * 1.2, v[1], v[2])
        out["spine_01"] = (14, 0, pose.get("spine_01", (0, 0, 0))[2])
        return out

    half = [(round(f * 0.8), scale(p), l) for f, p, l in run_half()]
    keys = _with(cycle(half, 16), lambda f: merge(RUN_BOW, cape(35)))
    return _clip(arm, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _shot(arm, name, frames, fire, stance, aim_up, lean, draw, crouch=False):
    """A bow shot: nock, draw, full draw the frame before `fire`, a one-frame release, recoil."""
    legs = merge(CROUCH if crouch else STAND, {"thigh_l": (-12, 0, 0), "calf_l": (10, 0, 0), "thigh_r": (10, 0, 0), "calf_r": (8, 0, 0)})
    turn = merge(TURN, {"spine_01": (4 + lean, 0, -14), "spine_02": (2 + lean, 0, -16 - stance)})
    aim = bow_arm(0.02, 1, 0.04 + aim_up, 0.0, 1, 0.05 + aim_up)
    keys = [
        (0, merge(legs, bow_arm(0.2, 0.6, -0.7, 0.05, 1, -0.35), NOCK_R, {"spine_02": (2, 0, -8), "head": (-2, 0, 10)}, cape(3)), DOWN),
        (fire - 3, merge(legs, turn, aim, bow_arm_r(0.35, 0.2, 0.05, -0.5, 0.85, 0.1), cape(4)), DOWN),
        (fire - 1, merge(legs, turn, aim, draw, cape(4, -1)), DOWN),
        (fire, merge(legs, turn, bow_arm(0.02, 1, 0.08 + aim_up, 0.0, 1, 0.1 + aim_up), RELEASE_R, cape(6, 2)), DOWN),
        (fire + 4, merge(legs, turn, aim, bow_arm_r(0.35, -0.5, -0.3, 0.1, -0.2, -0.9), cape(5)), DOWN),
        (frames, merge(READY), DOWN),
    ]
    return _clip(arm, name, frames, keys, markers={"fire": fire})


def attack_1(arm):
    return _shot(arm, "attack_1", 24, 7, stance=0, aim_up=0.0, lean=0, draw=DRAW_R)


def attack_2(arm):
    # The quick variant: crouched, lower angle, drawn to the chest.
    draw = bow_arm_r(0.3, -0.6, 0.0, -0.7, 0.6, -0.15)
    return _shot(arm, "attack_2", 24, 7, stance=6, aim_up=-0.05, lean=8, draw=draw, crouch=True)


def q(arm):
    # Longshot: a deeper draw aimed higher, held a beat, a stronger recoil.
    return _shot(arm, "q", 16, 8, stance=6, aim_up=0.12, lean=-6, draw=bow_arm_r(0.32, -0.85, 0.2, -0.8, 0.6, 0.15))


def w(arm):
    # Shrapnel Charge: an overhand lob with the right hand, bow out for balance.
    bal = bow_arm(0.6, 0.4, -0.6, 0.5, 0.6, -0.5)
    keys = [
        (0, merge(STAND, bal, bow_arm_r(0.3, -0.6, -0.7, 0.2, -0.8, -0.4), {"spine_02": (-5, 0, -15)}, cape(2)), None),
        (5, merge(STAND, bal, bow_arm_r(0.3, -0.6, 0.6, 0.1, -0.7, 0.6), {"spine_01": (-10, 0, -10), "spine_02": (-4, 0, -12)}, cape(3, -2)), None),
        (8, merge(STAND, bal, bow_arm_r(0.15, 0.9, 0.45, 0.05, 1, 0.3), {"spine_01": (8, 0, 12), "spine_02": (4, 0, 8), "thigh_r": (14, 0, 0)}, cape(6, 3)), {"pelvis": (0, -0.03, -0.02)}),
        (12, merge(STAND, bal, bow_arm_r(0.1, 0.7, -0.6, 0, 0.6, -0.8), {"spine_01": (12, 0, 10), "spine_02": (4, 0, 6)}, cape(5)), {"pelvis": (0, -0.04, -0.03)}),
        (16, READY, DOWN),
    ]
    return _clip(arm, "w", 16, keys, markers={"fire": 8})


TUCK = merge(sym("thigh", -100, 0, 0), sym("calf", 120, 0, 0), sym("foot", -20, 0, 0),
             arms((0.2, 0.8, -0.3), (-0.6, 0.5, 0.4)), {"spine_01": (40, 0, 0), "spine_02": (20, 0, 0), "head": (35, 0, 0)}, cape(-20))


def e_start(arm):
    crouch = merge(sym("thigh", -35, 0, 0), sym("calf", 60, 0, 0), sym("foot", -25, 0, 0), arms((0.3, 0.6, -0.6), (-0.2, 1, 0)),
                   {"spine_01": (25, 0, 0)}, cape(5))
    dive = merge(crouch, {"pelvis": (30, 0, 0), "spine_01": (50, 0, 0)})
    return _clip(arm, "e_start", 4, [(0, crouch, {"pelvis": (0, 0, -0.15)}), (4, dive, {"pelvis": (0, 0, -0.3)})], markers={"fire": 1})


def e_travel(arm):
    # A forward roll in place (the sim carries her), tucked around the pelvis.
    keys = [(i * 3, merge(TUCK, {"pelvis": (90 * i, 0, 0)}), {"pelvis": (0, 0, -0.55)}) for i in range(5)]
    return _clip(arm, "e_travel", 12, keys, loop=True)


def e_land(arm):
    land = merge(sym("thigh", -60, 0, 0), sym("calf", 90, 0, 0), sym("foot", -30, 0, 0), arms((0.7, 0.3, -0.3), (0.6, 0.5, -0.2)),
                 {"spine_01": (30, 0, 0)}, cape(-10))
    rise = merge(sym("thigh", -30, 0, 0), sym("calf", 50, 0, 0), sym("foot", -20, 0, 0), BOW_LOW, {"spine_01": (15, 0, 0)}, cape(10))
    return _clip(arm, "e_land", 8, [(0, land, {"pelvis": (0, 0, -0.35)}), (4, rise, {"pelvis": (0, 0, -0.15)}), (8, READY, DOWN)])


def r(arm):
    # Snare Net: a coiled two-handed spinning throw, flung wide; a heavier follow-through.
    gather = merge(CROUCH, bow_arm(-0.3, 0.4, -0.8, -0.5, 0.6, -0.4), bow_arm_r(0.6, -0.3, -0.6, 0.5, 0.3, -0.6),
                   {"spine_01": (6, 0, -30), "spine_02": (2, 0, -15)}, cape(3, -4))
    coil = merge(gather, sym("thigh", -18, 0, 0), sym("calf", 30, 0, 0), {"spine_01": (8, 0, -45), "spine_02": (2, 0, -20)})
    fling = merge(arms((0.5, 0.9, 0.4), (0.6, 0.8, 0.5)), {"thigh_l": (-25, 0, 0), "calf_l": (15, 0, 0), "thigh_r": (12, 0, 0),
                                                           "spine_01": (4, 0, 25), "spine_02": (2, 0, 10)}, cape(10, 6))
    follow = merge(arms((0.7, 0.7, 0.2), (0.8, 0.6, 0.1)), {"thigh_l": (-25, 0, 0), "calf_l": (20, 0, 0), "spine_01": (12, 0, 30)}, cape(6, 4))
    keys = [(0, gather, DOWN), (5, coil, {"pelvis": (0, 0, -0.08)}), (8, fling, {"pelvis": (0, -0.05, -0.02)}),
            (12, follow, {"pelvis": (0, -0.06, -0.04)}), (19, READY, DOWN)]
    return _clip(arm, "r", 19, keys, markers={"fire": 8})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (10, 0, 0), "head": (20, 0, 0)},
              bow_arm(0.2, 0.7, -0.7, 0.1, 0.8, -0.6), bow_arm_r(0.15, 0.3, -0.9, -0.8, 0.5, 0.4), cape(-5))


def recall(arm):
    breathe = merge(KNEEL, sym("clavicle", 0, -4, 0), {"spine_01": (6, 0, 0), "head": (16, 0, 0)})
    rise = merge(arms((0.8, 0.2, 0.3), (0.9, 0.1, 0.4)), STAND, {"head": (-10, 0, 0)}, cape(8))
    keys = [(0, merge(STAND, BOW_LOW), None), (15, KNEEL, {"pelvis": (0, 0, -0.45)}), (45, breathe, {"pelvis": (0, 0, -0.44)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.45)}), (90, rise, {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    keys = [
        (0, merge(STAND, BOW_LOW), None),
        (6, merge(arms((0.6, -0.2, 0.3), (0.5, 0.1, 0.5)), {"spine_01": (-15, 0, 0), "head": (-20, 0, 0)}, cape(-10)), {"pelvis": (0, 0.04, 0)}),
        (14, merge(sym("thigh", 0, 0, 0), sym("calf", 90, 0, 0), arms((0.2, 0.4, -1), (0.15, 0.5, -1)), {"spine_01": (15, 0, 0), "head": (25, 0, 0)}, cape(5)),
         {"pelvis": (0, 0, -0.45)}),
        (24, merge(sym("thigh", -20, 0, 0), sym("calf", 70, 0, 0), arms((0.5, 0.2, -0.6), (0.4, 0.3, -0.7)), {"pelvis": (0, 60, 0), "spine_01": (5, 15, 0)}, cape(10, 10)),
         {"pelvis": (0.25, 0, -0.65)}),
        (34, merge(sym("thigh", -30, 0, 0), sym("calf", 50, 0, 0), arms((0.7, 0.1, -0.3), (0.6, 0.2, -0.3)), {"pelvis": (0, 85, 0), "head": (0, 10, 0)}, cape(20, 15)),
         {"pelvis": (0.45, 0, -0.82)}),
    ]
    return _clip(arm, "death", 34, keys)


def respawn(arm):
    low = merge(sym("thigh", -50, 0, 0), sym("calf", 90, 0, 0), sym("foot", -30, 0, 0), arms((0.2, 0.6, -0.6), (-0.3, 0.8, 0)), {"spine_01": (25, 0, 0)}, cape(-5))
    keys = [(0, low, {"pelvis": (0, 0, -0.3)}), (10, merge(STAND, bow_arm(0.9, 0.3, 0.1, 0.9, 0.4, 0.2), cape(10)), {"pelvis": (0, 0, -0.08)}),
            (16, merge(STAND, bow_arm(0.1, 0.6, 0.8, 0.05, 0.5, 1), {"head": (-8, 0, 0)}, cape(5)), None), (24, merge(STAND, BOW_LOW), None)]
    return _clip(arm, "respawn", 24, keys)


def select(arm):
    up = {"upperarm_l": clips.aim(0.0, 0.5, 0.85), "forearm_l": clips.aim(0.0, 0.45, 0.9)}
    hip_r = bow_arm_r(0.6, -0.2, -0.6, -0.6, -0.1, 0.1)
    keys = [
        (0, merge(STAND, BOW_LOW), None),
        (8, merge(STAND, up, bow_arm_r(0.3, 0.2, 0.6, -0.4, 0.4, 0.8), {"spine_01": (-8, 0, -10), "head": (-25, 0, 10)}), None),
        (16, merge(STAND, up, bow_arm_r(0.3, -0.5, 0.5, -0.7, 0.3, 0.7), {"spine_01": (-10, 0, -12), "head": (-30, 0, 12)}), None),
        (18, merge(STAND, up, bow_arm_r(0.35, -0.6, 0.6, 0.3, -0.5, 0.6), {"spine_01": (-10, 0, -12), "head": (-30, 0, 12)}, cape(4, 3)), None),
        (28, merge(STAND, up, bow_arm_r(0.3, -0.3, -0.5, 0.2, -0.1, -0.9), {"head": (-35, 0, 0)}), None),
        (36, merge(STAND, BOW_LOW, hip_r, {"head": (-4, 0, -10), "pelvis": (0, 4, 8)}), None),
        (45, merge(STAND, BOW_LOW, hip_r, {"head": (-6, 0, -12), "pelvis": (0, 5, 10), "chest": (-3, 0, 0)}), None),
    ]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, BOW_LOW)
    out = merge(base, bow_arm_r(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-5, 6, 0)})
    curl = merge(base, bow_arm_r(0.2, 1, 0.0, 0.1, 0.5, 0.8), {"head": (-5, 6, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Spins the bow on her palm, catches it, takes a bow.
    fwd = merge(STAND, bow_arm(0.1, 0.9, -0.2, 0.0, 1, 0.05))
    keys = [(0, merge(STAND, BOW_LOW), None)] + [(10 + 4 * i, merge(fwd, {"hand_l": (0, 120 * i, 0)}), None) for i in range(7)]
    keys += [(40, merge(STAND, BOW_LOW, {"spine_01": (30, 0, 0), "head": (15, 0, 0)}), None), (50, merge(STAND, BOW_LOW), None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    belly = bow_arm_r(0.1, 0.3, -0.9, -0.7, 0.6, 0.2)
    keys = [(0, merge(STAND, BOW_LOW), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, BOW_LOW, belly, {"spine_01": (8 * s, 0, 0), "head": (15 * s, 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, merge(STAND, BOW_LOW), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    a = merge(arms((0.6, 0.2, 0.8), (0.6, 0.0, 1), (0.3, 0.1, -1), (0.2, 0.3, -1)), {"thigh_l": (-20, 0, 0), "calf_l": (30, 0, 0), "pelvis": (0, -6, 0)}, cape(0, -6))
    mid = merge(STAND, arms((0.4, 0.3, -0.2), (0.3, 0.6, 0.2)), cape(2))
    b = merge(arms((0.3, 0.1, -1), (0.2, 0.3, -1), (0.6, 0.2, 0.8), (0.6, 0.0, 1)), {"thigh_r": (-20, 0, 0), "calf_r": (30, 0, 0), "pelvis": (0, 6, 0)}, cape(0, 6))
    keys = [(0, a, {"pelvis": (0.06, 0, 0)}), (12, mid, {"pelvis": (0, 0, -0.04)}), (24, b, {"pelvis": (-0.06, 0, 0)}), (36, mid, {"pelvis": (0, 0, -0.04)})]
    return _clip(arm, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e_start, e_travel, e_land, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]
SHARED = ["walk", "cast_utility", "attack_melee_alt", "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed",
          "cc_sleep", "cc_forced_move"]


def bake_shared(arm):
    """Copy the shared library's clips in (10 §7.3): same skeleton, so a direct copy."""
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
    arm = rig.build_armature()
    rig.add_chain(arm, "cape", "chest", CAPE)
    own = [f(arm).name for f in OWN]
    body = mesh.build_mannequin(arm, name="vesper", part_list=parts(*bow_frame(arm)))
    shared = bake_shared(arm)
    print(f"vesper: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(own)} own clips, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
