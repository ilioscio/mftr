# SPDX-License-Identifier: AGPL-3.0-or-later
"""Wren (A14): the block-out generator for her model and clips.

    blender -b --python art/champions/wren/build.py -- --out art/champions/wren/wren.blend

A nimble scout on `biped` v1: auburn hair under a peaked russet cap with a long team-colored
feather, a tan leather jerkin over a cream shirt, an olive capelet on an `extra_cape` chain, tall
boots, a quiver of bolts at her right hip, and a hand crossbow in her right hand (laid along the
hand bone, aimed per pose). Pounce has a start / travel / land set. Like the other block-outs
this is a starting point.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

from mftr_blender import clips, head, mesh, rig  # noqa: E402
from mftr_blender.clips import merge, sym  # noqa: E402
from mftr_blender.library import _clip, arms, cycle, run_half  # noqa: E402

# Palette (concept.md): russet, tan and olive.
SKIN = (0.86, 0.66, 0.52)
HAIR = (0.5, 0.22, 0.12)
CAP = (0.58, 0.2, 0.12)
JERKIN = (0.56, 0.38, 0.2)
SHIRT = (0.82, 0.76, 0.62)
OLIVE = (0.32, 0.34, 0.16)
OLIVE_DARK = (0.22, 0.24, 0.11)
TROUSERS = (0.26, 0.24, 0.16)
BOOT = (0.28, 0.17, 0.1)
WOOD = (0.36, 0.22, 0.12)
STEEL = (0.7, 0.72, 0.76)
ACCENT = (0.18, 0.52, 0.95)

CAPE = [(0, 0.12, 1.53), (0, 0.17, 1.32), (0, 0.2, 1.12)]


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", (0, 0, 0.9), (0, 0, 1.13), 12, [(0, .12, .095), (.4, .15, .105), (1, .148, .105)], "cloth", TROUSERS),
        # The jerkin's skirt, split at the sides, and a team-colored belt.
        ("pelvis", (0, 0, 1.12), (0, 0, 0.9), 12, [(0, .152, .11), (1, .17, .124)], "cloth", JERKIN),
        ("pelvis", (0, 0, 1.08), (0, 0, 1.13), 12, [(0, .156, .113), (1, .152, .11)], "accent", ACCENT),
        ("pelvis", (0, -0.115, 1.09), (0, -0.13, 1.12), 4, [(0, .026, .016), (1, .026, .016)], "metal", STEEL),
        # The quiver at her right hip, bolts fletched in the team color.
        ("pelvis", (-0.2, 0.05, 0.84), (-0.22, 0.09, 1.12), 8, [(0, .036, .036), (1, .042, .042)], "cloth", BOOT),
        ("pelvis", (-0.22, 0.09, 1.12), (-0.225, 0.1, 1.18), 6, [(0, .03, .03), (1, .03, .03)], "cloth", WOOD),
        ("pelvis", (-0.215, 0.085, 1.16), (-0.225, 0.1, 1.24), 4, [(0, .03, .006), (1, .022, .004)], "accent", ACCENT),
        ("spine_01", (0, 0, 1.11), (0, 0, 1.28), 12, [(0, .125, .09), (.5, .112, .084), (1, .12, .088)], "cloth", JERKIN),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .122, .088), (1, .145, .1)], "cloth", JERKIN),
        ("chest", (0, 0, 1.40), (0, 0, 1.60), 12, [(0, .145, .1), (.35, .152, .106), (.7, .145, .1), (1, .085, .07)], "cloth", JERKIN),
        ("chest", (0, -0.03, 1.415), (0, -0.035, 1.53), 10, [(0, .105, .052), (.5, .124, .066), (1, .098, .046)], "cloth", JERKIN),
        # The shirt's open collar and laces.
        ("chest", (0, -0.085, 1.5), (0, -0.1, 1.6), 4, [(0, .03, .01), (1, .045, .012)], "cloth", SHIRT),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.66), 8, [(0, .046, .046), (1, .042, .042)], "skin", SKIN),
        # The capelet: a mantle at the shoulders, its back on the chain.
        ("chest", (0, 0.01, 1.47), (0, 0.005, 1.6), 14, [(0, .185, .13), (.55, .16, .118), (1, .095, .082)], "cloth", OLIVE),
        ("extra_cape_1", CAPE[0], CAPE[1], 6, [(0, .16, .02), (1, .19, .02)], "cloth", OLIVE),
        ("extra_cape_2", CAPE[1], CAPE[2], 6, [(0, .19, .02), (.7, .18, .018), (1, .1, .014)], "cloth", OLIVE_DARK),
        # The cap: a peaked russet crown over a short brim, and the long feather.
        ("head", (0, 0.0, 1.81), (0, 0.13, 1.96), 8, [(0, .114, .128), (.35, .1, .108), (.8, .036, .044), (1, .0, .0)], "cloth", CAP),
        ("head", (0, -0.008, 1.82), (0, -0.006, 1.83), 10, [(0, .142, .16), (1, .138, .155)], "cloth", CAP),
        ("head", (0.1, 0.03, 1.85), (0.17, 0.26, 2.0), 4, [(0, .012, .004), (.4, .028, .005), (1, 0, 0)], "accent", ACCENT),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 8, [(0, .052, .052), (1, .045, .045)], "cloth", SHIRT),
        (fa, (fa, 0), (fa, .5), 8, [(0, .044, .044), (1, .042, .042)], "cloth", SHIRT),
        # Leather bracers.
        (fa, (fa, .45), (fa, .96), 8, [(0, .046, .046), (1, .042, .042)], "cloth", BOOT),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .034, .021), (1, .038, .019)], "skin", SKIN),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .038, .019), (1, .025, .013)], "skin", SKIN),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .015, .015), (1, .01, .01)], "skin", SKIN),
        (th, (0.10, 0, 1.02), (th, 1), 10, [(0, .082, .082), (.5, .07, .07), (1, .054, .054)], "cloth", TROUSERS),
        (ca, (ca, 0), (ca, .4), 10, [(0, .054, .054), (1, .05, .05)], "cloth", TROUSERS),
        # Tall boots with a folded cuff.
        (ca, (ca, .3), (ca, .4), 10, [(0, .066, .066), (1, .064, .064)], "cloth", BOOT),
        (ca, (ca, .38), (ca, 1), 10, [(0, .058, .058), (.12, .055, .055), (1, .046, .05)], "cloth", BOOT),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .046, .04), (1, .04, .028)], "cloth", BOOT),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .04, .024), (1, .026, .014)], "cloth", BOOT),
    ]


def _crossbow():
    """The hand crossbow along the right hand bone: a stock forward from her fist, a steel-tipped
    prod across its front, and a loaded bolt."""
    h = Vector(next(b[2] for b in rig.bone_table() if b[0] == "hand_r"))
    t = Vector(next(b[3] for b in rig.bone_table() if b[0] == "hand_r"))
    axis = (t - h).normalized()
    grip = h + (t - h) * 0.6
    side = axis.cross(Vector((0, 1, 0))).normalized()
    c = grip + axis * 0.2
    out = [
        ("hand_r", tuple(grip - axis * 0.05), tuple(grip + axis * 0.24), 4, [(0, .022, .02), (.5, .02, .028), (1, .016, .022)], "cloth", WOOD),
        ("hand_r", tuple(grip + axis * 0.04), tuple(grip + axis * 0.3), 4, [(0, .006, .006), (1, .006, .006)], "metal", STEEL),
    ]
    for s in (1, -1):
        mid = c + side * 0.07 * s - axis * 0.01
        tip = c + side * 0.13 * s - axis * 0.05
        out.append(("hand_r", tuple(c), tuple(mid), 4, [(0, .012, .01), (1, .01, .009)], "cloth", WOOD))
        out.append(("hand_r", tuple(mid), tuple(tip), 4, [(0, .01, .009), (1, .008, .008)], "metal", STEEL))
    return out


def _head(bm, layers, groups, mats):
    # The shared sculpted head: young and quick, auburn hair in a ponytail under her cap, hazel eyes.
    head.build(bm, layers, groups, mats, head.Head(
        base=(0, -0.008, 1.64), height=0.25, scale=0.92, jaw=0.48, chin=0.84, eye=1.15,
        skin=SKIN, hair=HAIR, hair_style="ponytail", eye_color=(0.4, 0.34, 0.16), lips=(0.72, 0.42, 0.38)))


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + _crossbow() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def cape(back=0.0, sway=0.0):
    return {"extra_cape_1": (back * 0.6, sway * 0.5, 0), "extra_cape_2": (back * 0.4, sway * 0.4, 0)}


def BOW(out, fwd, up):
    """Where the crossbow points (character space)."""
    return {"hand_r": clips.aim(out, fwd, up, "r")}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


STAND = merge(sym("thigh", -4, 0, 0), sym("calf", 8, 0, 0), sym("foot", -4, 0, 0), {"spine_01": (3, 0, 0)})
CROUCH = merge(sym("thigh", -20, 0, 0), sym("calf", 36, 0, 0), sym("foot", -16, 0, 0), {"spine_01": (12, 0, 0), "spine_02": (3, 0, 0)})
LOW = merge(arms((0.3, 0.1, -1), (0.2, 0.4, -0.9)), BOW(0.1, 0.5, -0.85))
# Aimed: the right arm straight at the target, the left hand steadying the wrist.
AIM = merge(arm_r(0.05, 1, 0.08, 0.0, 1, 0.08), arm_l(0.35, 0.8, -0.1, -0.5, 0.85, 0.1), BOW(0.0, 1, 0.05), {"spine_02": (0, 0, 10), "head": (2, 0, -6)})
READY = merge(CROUCH, arms((0.35, 0.55, -0.6), (0.1, 1, 0.0)), BOW(0.1, 1, 0.3), cape(6))
DOWN = {"pelvis": (0, 0, -0.07)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, LOW, cape(0), {"head": (-2, 0, 0)})
    b = merge(STAND, LOW, cape(2, 3), sym("clavicle", 0, -3, 0), {"head": (-4, 0, 6)})
    return _clip(arm, "idle", 60, [(0, a, None), (30, b, {"pelvis": (0, 0, 0.005)})], loop=True)


def idle_fidget_1(arm):
    # Draws a bolt from her quiver and loads it.
    base = merge(STAND, LOW)
    draw = merge(STAND, LOW, arm_l(0.2, -0.1, -0.9, -0.6, 0.1, -0.6), {"head": (10, 0, -8)})
    load = merge(STAND, arm_r(0.3, 0.5, -0.6, 0.0, 0.9, 0.2), BOW(0.0, 0.9, 0.4), arm_l(0.2, 0.6, -0.5, -0.4, 0.8, 0.3), {"head": (14, 0, 0)})
    keys = [(0, base, None), (10, draw, None), (20, load, None), (26, merge(load, {"hand_l": (0, 0, 20)}), None), (32, load, None), (44, base, None)]
    return _clip(arm, "idle_fidget_1", 44, keys)


def idle_fidget_2(arm):
    # Blows the feather out of her eyes, then tips her cap back.
    base = merge(STAND, LOW)
    blow = merge(base, {"head": (-14, 0, 8)})
    tip = merge(base, arm_l(0.3, 0.3, 0.7, -0.3, 0.2, 0.9), {"head": (-6, 0, 0)})
    keys = [(0, base, None), (8, blow, None), (16, base, None), (26, tip, None), (34, tip, None), (46, base, None)]
    return _clip(arm, "idle_fidget_2", 46, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (18, merge(READY, {"spine_01": (15, 0, 0)}, cape(8, 1)), {"pelvis": (0, 0, -0.09)})]
    return _clip(arm, "idle_ready", 36, keys, loop=True)


def _no_right_arm(pose):
    return {k: v for k, v in pose.items() if not (k.endswith("_r") and k.split("_")[0] in ("upperarm", "forearm"))}


def run(arm):
    keys = [(f, merge(_no_right_arm(p), arm_r(0.3, 0.3, -0.85, 0.1, 0.9, 0.0), BOW(0.1, 0.9, 0.3), cape(26 + (4 if f % 10 < 5 else 0))), l)
            for f, p, l in cycle(run_half(), 20)]
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(arm):
    def scale(pose):
        out = dict(pose)
        for k, v in pose.items():
            if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple):
                out[k] = (v[0] * 1.25, v[1], v[2])
        out["spine_01"] = (16, 0, pose.get("spine_01", (0, 0, 0))[2])
        return _no_right_arm(out)

    half = [(round(f * 0.8), scale(p), l) for f, p, l in run_half()]
    keys = [(f, merge(p, arm_r(0.3, 0.2, -0.9, 0.1, 0.8, -0.2), BOW(0.1, 0.8, 0.0), cape(36)), l) for f, p, l in cycle(half, 16)]
    return _clip(arm, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _shot(arm, name, frames, fire, aim, kick):
    """Raise, aim, loose on `fire`, and the crossbow kicks up."""
    keys = [(0, READY, DOWN), (fire - 4, merge(CROUCH, aim, cape(4)), DOWN), (fire, merge(CROUCH, aim, cape(6)), DOWN),
            (fire + 2, merge(CROUCH, aim, kick, cape(7)), {"pelvis": (0, 0.02, -0.07)}), (fire + 8, merge(CROUCH, aim, cape(5)), DOWN), (frames, READY, DOWN)]
    return _clip(arm, name, frames, keys, markers={"fire": fire})


def attack_1(arm):
    # Sim: windup 20% of 1/0.75 s = 8.0 frames.
    return _shot(arm, "attack_1", 22, 8, AIM, merge(BOW(0.0, 0.85, 0.5), {"spine_02": (-4, 0, 10)}))


def attack_2(arm):
    # The quick variant: a one-handed snap shot from the hip.
    hip = merge(arms((0.3, 0.2, -0.9), (0.2, 0.4, -0.9)), arm_r(0.2, 0.9, -0.4, 0.1, 1, -0.1), BOW(0.05, 1, 0.0), {"spine_02": (0, 0, 16)})
    return _shot(arm, "attack_2", 22, 8, hip, BOW(0.05, 0.85, 0.45))


def q(arm):
    # Ricochet: a careful two-handed shot, leaning into it, spun off the first target (`fire` 8).
    lean = merge(AIM, arm_l(0.2, 0.9, -0.1, -0.3, 0.9, 0.1), {"spine_01": (16, 0, 0), "head": (-6, 0, -10)}, sym("thigh", -26, 0, 0), sym("calf", 40, 0, 0))
    return _shot(arm, "q", 24, 8, lean, merge(BOW(0.0, 0.8, 0.6), {"spine_02": (-6, 0, 10)}))


def w(arm):
    # Caltrops: an underhand scatter from the left hand (`fire` 6).
    wind = merge(CROUCH, LOW, arm_l(0.3, -0.5, -0.7, 0.2, -0.3, -0.9), {"spine_02": (0, 0, 14)})
    toss = merge(CROUCH, LOW, arm_l(0.2, 0.9, 0.2, 0.1, 0.9, 0.4), {"spine_02": (4, 0, -12)})
    keys = [(0, READY, DOWN), (3, wind, DOWN), (6, toss, {"pelvis": (0, -0.03, -0.06)}), (12, toss, DOWN), (20, READY, DOWN)]
    return _clip(arm, "w", 20, keys, markers={"fire": 6})


def _dash_trio(arm, slot, launch, travel, fire, land_pose):
    """`<slot>_start` (with `fire` at the launch), a looping `_travel`, `_land`."""
    start = _clip(arm, f"{slot}_start", 5, [(0, merge(CROUCH, launch, cape(5)), {"pelvis": (0, 0, -0.2)}), (5, travel, {"pelvis": (0, 0, 0.04)})],
                  markers={"fire": fire})
    trav = _clip(arm, f"{slot}_travel", 10, [(0, travel, {"pelvis": (0, 0, 0.06)}), (5, merge(travel, cape(-30, 4)), {"pelvis": (0, 0, 0.1)})], loop=True)
    end = _clip(arm, f"{slot}_land", 12, [(0, land_pose, {"pelvis": (0, -0.04, -0.28)}), (6, land_pose, {"pelvis": (0, -0.04, -0.24)}), (12, READY, DOWN)])
    return start, trav, end


def e_trio(arm):
    # Pounce: a springing leap, knees tucked, the crossbow levelled down at the target; she lands
    # on it in a crouch with a point-blank shot.
    leap = merge(sym("thigh", -80, 0, 0), sym("calf", 100, 0, 0), sym("foot", -20, 0, 0), arm_r(0.1, 0.9, -0.3, 0.0, 1, -0.5),
                 arm_l(0.7, 0.2, 0.2, 0.6, 0.3, 0.3), BOW(0.0, 0.7, -0.7), {"spine_01": (20, 0, 0), "head": (-10, 0, 0)}, cape(-30))
    land = merge(sym("thigh", -55, 0, 0), sym("calf", 90, 0, 0), sym("foot", -30, 0, 0), arm_r(0.1, 0.8, -0.5, 0.0, 0.8, -0.6),
                 arm_l(0.6, 0.0, -0.6, 0.4, 0.2, -0.8), BOW(0.0, 0.6, -0.8), {"spine_01": (26, 0, 0)}, cape(20))
    return _dash_trio(arm, "e", arms((0.4, -0.3, -0.5), (0.3, -0.2, -0.6)), leap, 2, land)


def r(arm):
    # Hail of Arrows: the crossbow raised high and loosed at the sky over the target (`fire` 8).
    lob = merge(STAND, arm_r(0.05, 0.6, 0.8, 0.0, 0.55, 0.85), arm_l(0.3, 0.6, 0.5, -0.4, 0.6, 0.7), BOW(0.0, 0.55, 0.85),
                {"spine_02": (-10, 0, 0), "head": (-18, 0, 0)}, sym("thigh", -10, 0, 0), {"thigh_r": (14, 0, 0)})
    keys = [(0, READY, DOWN), (4, lob, None), (8, lob, None), (10, merge(lob, BOW(0.0, 0.3, 1)), {"pelvis": (0, 0.02, 0)}), (16, lob, None), (26, READY, DOWN)]
    return _clip(arm, "r", 26, keys, markers={"fire": 8})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (8, 0, 0), "head": (10, 0, 0)}, LOW, cape(-6))


def recall(arm):
    # Kneels and checks the string of her crossbow.
    check = merge(KNEEL, arm_r(0.2, 0.6, -0.6, 0.0, 0.9, 0.2), BOW(0.0, 0.8, 0.5), arm_l(0.2, 0.6, -0.5, -0.4, 0.8, 0.2), {"head": (20, 0, 0)})
    keys = [(0, merge(STAND, LOW), None), (15, KNEEL, {"pelvis": (0, 0, -0.45)}), (30, check, {"pelvis": (0, 0, -0.45)}), (60, merge(check, {"hand_l": (0, 0, 20)}), {"pelvis": (0, 0, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.45)}), (90, merge(STAND, LOW, cape(6)), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    keys = [
        (0, merge(STAND, LOW), None),
        (6, merge(arms((0.6, -0.2, 0.3), (0.5, 0.1, 0.5)), {"spine_01": (-15, 0, 0), "head": (-20, 0, 0)}, cape(-10)), {"pelvis": (0, 0.04, 0)}),
        (14, merge(sym("thigh", 0, 0, 0), sym("calf", 90, 0, 0), arms((0.2, 0.4, -1), (0.15, 0.5, -1)), {"spine_01": (15, 0, 0), "head": (25, 0, 0)}, cape(5)),
         {"pelvis": (0, 0, -0.45)}),
        (24, merge(sym("thigh", -20, 0, 0), sym("calf", 70, 0, 0), arms((0.5, 0.2, -0.6), (0.4, 0.3, -0.7)), {"pelvis": (0, 60, 0), "spine_01": (5, 15, 0)},
                   cape(10, 10)), {"pelvis": (0.25, 0, -0.65)}),
        (34, merge(sym("thigh", -30, 0, 0), sym("calf", 50, 0, 0), arms((0.7, 0.1, -0.3), (0.6, 0.2, -0.3)), BOW(0.9, 0.2, -0.2), {"pelvis": (0, 85, 0), "head": (0, 10, 0)},
                   cape(20, 15)), {"pelvis": (0.45, 0, -0.82)}),
    ]
    return _clip(arm, "death", 34, keys)


def respawn(arm):
    # Drops in from a roll into a crouch and springs up.
    low = merge(sym("thigh", -60, 0, 0), sym("calf", 100, 0, 0), sym("foot", -30, 0, 0), arms((0.5, 0.3, -0.6), (0.4, 0.5, -0.5)), BOW(0.1, 0.8, 0.0),
                {"spine_01": (30, 0, 0)}, cape(10))
    keys = [(0, low, {"pelvis": (0, 0, -0.38)}), (10, merge(STAND, arms((0.6, 0.3, 0.1), (0.5, 0.4, 0.2)), BOW(0.1, 0.4, 0.9), cape(8)), {"pelvis": (0, 0, 0.04)}),
            (18, merge(STAND, LOW, {"head": (-6, 0, 0)}), None), (24, merge(STAND, LOW), None)]
    return _clip(arm, "respawn", 24, keys)


def select(arm):
    # Spins the crossbow around her finger, catches it and tips her cap.
    base = merge(STAND, LOW)
    spin = merge(STAND, arm_r(0.4, 0.4, -0.3, 0.2, 0.8, 0.3), arms((0.3, 0.1, -1), (0.2, 0.4, -0.9)))
    keys = [(0, base, None)]
    for i, a in enumerate((0, 90, 180, 270, 360)):
        keys.append((6 + 3 * i, merge(spin, BOW(math.cos(math.radians(a)) * 0.3, math.cos(math.radians(a)), math.sin(math.radians(a)))), None))
    tip = merge(STAND, merge(arm_r(0.4, 0.4, -0.3, 0.2, 0.8, 0.3), BOW(0.3, 1, 0.0)), arm_l(0.3, 0.3, 0.7, -0.3, 0.2, 0.9), {"head": (-4, 0, 8)})
    keys += [(26, tip, None), (36, tip, None), (45, base, None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, LOW)
    out = merge(base, arm_l(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-5, 0, 0)})
    curl = merge(base, arm_l(0.2, 1, 0.0, 0.1, 0.5, 0.8), {"head": (-5, 0, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Aims at something overhead, looses, and ducks as the bolt comes back down.
    base = merge(STAND, LOW)
    up = merge(STAND, arm_r(0.05, 0.2, 1, 0.0, 0.1, 1), BOW(0.0, 0.1, 1), {"head": (-24, 0, 0)})
    duck = merge(CROUCH, arms((0.3, 0.3, 0.8), (0.2, 0.2, 0.9)), BOW(0.3, 0.2, 0.9), {"spine_01": (24, 0, 0), "head": (20, 0, 0)})
    keys = [(0, base, None), (10, up, None), (18, up, None), (26, up, None), (30, duck, {"pelvis": (0, 0, -0.18)}), (40, duck, {"pelvis": (0, 0, -0.18)}),
            (50, base, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    belly = arm_l(0.15, 0.3, -0.9, -0.7, 0.6, 0.2)
    keys = [(0, merge(STAND, LOW), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, LOW, belly, {"spine_01": (8 * s, 0, 0), "head": (15 * s, 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, merge(STAND, LOW), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    # A quick heel-and-toe jig.
    a = merge(LOW, arm_l(0.8, 0.1, 0.3, 0.7, 0.2, 0.5), {"thigh_l": (-30, 0, 0), "calf_l": (50, 0, 0), "pelvis": (0, -4, 12)}, cape(0, -6))
    mid = merge(STAND, LOW, cape(4))
    b = merge(LOW, arm_l(0.8, 0.1, -0.3, 0.7, 0.2, -0.3), {"thigh_r": (-30, 0, 0), "calf_r": (50, 0, 0), "pelvis": (0, 4, -12)}, cape(0, 6))
    keys = [(0, a, {"pelvis": (0, 0, 0.04)}), (6, mid, {"pelvis": (0, 0, -0.02)}), (12, b, {"pelvis": (0, 0, 0.04)}), (18, mid, {"pelvis": (0, 0, -0.02)})]
    return _clip(arm, "emote_dance", 24, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e_trio, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]
SHARED = ["walk", "cast_utility", "attack_melee_alt", "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed",
          "cc_sleep", "cc_forced_move"]


def bake_shared(arm):
    """Copy the shared library's clips in (10 §7.3): the same skeleton, so a direct copy."""
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
    rig.use_shape("standard")
    arm = rig.build_armature(name="wren")
    rig.add_chain(arm, "cape", "chest", CAPE)
    for f in OWN:
        f(arm)
    body = mesh.build_mannequin(arm, name="wren", part_list=parts())
    shared = bake_shared(arm)
    print(f"wren: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(bpy.data.actions)} actions, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
