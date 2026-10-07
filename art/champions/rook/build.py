# SPDX-License-Identifier: AGPL-3.0-or-later
"""Rook (A6): the block-out generator for his model and clips.

    blender -b --python art/champions/rook/build.py -- --out art/champions/rook/rook.blend

A grizzled veteran bruiser on the `biped` bones in the `large` shape (`biped_large`, 10 §7.1:
~2.05 m, broad and deep): shaved head, a braided beard and a scar, battered iron plate over a
quilted gambeson and an oxblood tabard, and a stone-headed maul. Like the other block-outs this
is a starting point: run once, then the .blend is the source of truth. Pose values follow
`mftr_blender.clips` (arms aimed in character space; everything else in degrees about the
armature axes). His right hand holds the maul.
"""

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

# Palette (concept.md): iron and oxblood.
SKIN = (0.76, 0.56, 0.44)
BEARD = (0.36, 0.31, 0.27)
GAMBESON = (0.40, 0.33, 0.24)
OXBLOOD = (0.36, 0.08, 0.07)
TROUSERS = (0.18, 0.15, 0.13)
LEATHER = (0.38, 0.24, 0.14)
BOOT = (0.22, 0.15, 0.10)
IRON = (0.44, 0.45, 0.48)
IRON_DARK = (0.27, 0.28, 0.31)
STONE = (0.56, 0.54, 0.49)
WOOD = (0.36, 0.22, 0.11)
ACCENT = (0.18, 0.52, 0.95)

# Radii authored for the standard biped grow with the large shape's width and depth.
WIDE, DEEP = 1.22, 1.18


def pt(p):
    return p if isinstance(p[0], str) else rig.shaped(p)


def part(bone, a, b, sides, prof, slot, color):
    return (bone, pt(a), pt(b), sides, [(t, ru * WIDE, rv * DEEP) for t, ru, rv in prof], slot, color)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        part("pelvis", (0, 0, 0.90), (0, 0, 1.13), 12, [(0, .15, .12), (.4, .18, .13), (1, .175, .13)], "cloth", TROUSERS),
        # The tabard: front and back panels from the belt to the knees, team-colored hems.
        part("pelvis", (0, -0.13, 1.08), (0, -0.16, 0.62), 4, [(0, .2, .028), (1, .21, .028)], "cloth", OXBLOOD),
        part("pelvis", (0, 0.13, 1.08), (0, 0.16, 0.62), 4, [(0, .2, .028), (1, .21, .028)], "cloth", OXBLOOD),
        part("pelvis", (0, -0.158, 0.665), (0, -0.163, 0.61), 4, [(0, .215, .034), (1, .215, .034)], "accent", ACCENT),
        part("pelvis", (0, 0.158, 0.665), (0, 0.163, 0.61), 4, [(0, .215, .034), (1, .215, .034)], "accent", ACCENT),
        part("pelvis", (0, 0, 1.06), (0, 0, 1.13), 12, [(0, .19, .14), (1, .19, .14)], "cloth", LEATHER),
        part("pelvis", (0, -0.14, 1.07), (0, -0.165, 1.12), 4, [(0, .045, .03), (1, .045, .03)], "metal", IRON),
        # Gambeson torso under a breastplate, a gorget, a team-colored sash across the plate.
        part("spine_01", (0, 0, 1.11), (0, 0, 1.28), 12, [(0, .17, .13), (1, .18, .135)], "cloth", GAMBESON),
        part("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .185, .14), (1, .2, .145)], "cloth", GAMBESON),
        part("chest", (0, 0, 1.40), (0, 0, 1.62), 12, [(0, .205, .15), (.3, .22, .155), (.65, .215, .15), (.9, .17, .12), (1, .11, .09)], "cloth", GAMBESON),
        part("chest", (0, -0.03, 1.28), (0, -0.035, 1.56), 10, [(0, .17, .13), (.5, .19, .14), (1, .16, .12)], "metal", IRON),
        part("chest", (0.15, -0.172, 1.53), (-0.15, -0.18, 1.24), 4, [(0, .032, .02), (1, .032, .02)], "accent", ACCENT),
        part("chest", (0, 0, 1.56), (0, 0, 1.64), 12, [(0, .15, .12), (1, .1, .09)], "metal", IRON_DARK),
        part("neck", (0, 0, 1.55), (0, -0.01, 1.67), 10, [(0, .085, .085), (1, .075, .075)], "skin", SKIN),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        # Layered pauldrons with a team-colored rim (reads from the gameplay camera).
        part(ua, (0.15, 0.0, 1.50), (0.30, 0.0, 1.62), 10, [(0, .12, .12), (.4, .15, .14), (.8, .12, .11), (1, 0, 0)], "metal", IRON),
        part(ua, (0.21, 0.0, 1.43), (0.29, 0.0, 1.51), 10, [(0, .13, .12), (1, .14, .13)], "metal", IRON_DARK),
        part(ua, (0.205, 0.0, 1.425), (0.225, 0.0, 1.44), 10, [(0, .138, .128), (1, .138, .128)], "accent", ACCENT),
        part(ua, (ua, 0), (ua, 1), 10, [(0, .085, .085), (.5, .082, .08), (1, .07, .07)], "cloth", GAMBESON),
        part(fa, (fa, 0), (fa, 1), 8, [(0, .07, .07), (1, .06, .06)], "cloth", LEATHER),
        part(fa, (fa, .45), (fa, 1.04), 8, [(0, .076, .076), (1, .07, .07)], "metal", IRON),
        part("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .05, .035), (1, .055, .03)], "metal", IRON_DARK),
        part("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .055, .03), (1, .04, .022)], "metal", IRON_DARK),
        part("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .022, .022), (1, .015, .015)], "metal", IRON_DARK),
        part(th, (0.10, 0, 1.02), (th, 1), 10, [(0, .11, .11), (.5, .1, .1), (1, .08, .08)], "cloth", TROUSERS),
        part(ca, (ca, -0.03), (ca, 0.12), 8, [(0, .085, .085), (1, .08, .08)], "metal", IRON),
        part(ca, (ca, 0), (ca, .5), 10, [(0, .075, .075), (1, .07, .07)], "cloth", TROUSERS),
        part(ca, (ca, .4), (ca, 1), 10, [(0, .088, .088), (.15, .083, .083), (1, .072, .076)], "cloth", BOOT),
        part("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .065, .055), (1, .06, .04)], "cloth", BOOT),
        part("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .058, .036), (1, .04, .022)], "cloth", BOOT),
    ]


def _head(bm, layers, groups, mats):
    # The shared sculpted head, a weathered veteran: square jaw, heavy chin, small hard eyes,
    # shaved, a braided beard with an iron bead, a scar over the left brow.
    head.build(bm, layers, groups, mats, head.Head(
        base=rig.shaped((0, -0.01, 1.64)), height=0.25, scale=1.08, jaw=0.18, chin=1.15, eye=0.85,
        skin=SKIN, hair=BEARD, hair_style="bald", eye_color=(0.14, 0.12, 0.1), lips=(0.56, 0.36, 0.32),
        beard=True, scar=True, metal=IRON_DARK))


def _maul():
    """The stone-headed maul, weighted to the right hand and laid along the hand bone, so every
    pose aims it exactly by aiming `hand_r` (`clips.aim`): on the shoulder, high and back in the
    windups, forward into the ground on the impacts."""
    h = Vector(next(b[2] for b in rig.bone_table() if b[0] == "hand_r"))
    t = Vector(next(b[3] for b in rig.bone_table() if b[0] == "hand_r"))
    haft = (t - h).normalized()
    across = haft.cross(Vector((0, 1, 0))).normalized()
    grip = h + (t - h) * 0.6
    c = grip + haft * 1.08

    def p(v):
        return tuple(v)

    return [
        ("hand_r", p(grip - haft * 0.32), p(grip - haft * 0.26), 6, [(0, .036, .036), (1, .036, .036)], "metal", IRON_DARK),
        ("hand_r", p(grip - haft * 0.28), p(grip + haft * 1.0), 6, [(0, .022, .022), (1, .025, .025)], "cloth", WOOD),
        ("hand_r", p(grip - haft * 0.12), p(grip + haft * 0.16), 6, [(0, .029, .029), (1, .029, .029)], "cloth", LEATHER),
        ("hand_r", p(c - across * 0.25), p(c + across * 0.25), 4,
         [(0, .12, .12), (.12, .155, .155), (.88, .155, .155), (1, .12, .12)], "cloth", STONE),
        ("hand_r", p(c - across * 0.15), p(c - across * 0.11), 4, [(0, .168, .168), (1, .168, .168)], "metal", IRON_DARK),
        ("hand_r", p(c + across * 0.11), p(c + across * 0.15), 4, [(0, .168, .168), (1, .168, .168)], "metal", IRON_DARK),
    ]


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + _maul() + [_head]


# --- Poses ----------------------------------------------------------------------------------

STAND = merge(sym("thigh", -3, -4, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"spine_01": (2, 0, 0)})
CROUCH = merge(sym("thigh", -18, -6, 0), sym("calf", 32, 0, 0), sym("foot", -14, 0, 0), {"spine_01": (10, 0, 0), "spine_02": (3, 0, 0)})
def MAUL(out, fwd, up):
    """Where the haft points (character space, from the right hand)."""
    return {"hand_r": clips.aim(out, fwd, up, "r")}


CARRY = {"upperarm_r": clips.aim(0.3, 0.35, -0.88, "r"), "forearm_r": clips.aim(-0.3, 0.8, 0.45, "r"),
         "hand_r": clips.aim(-0.15, -0.7, 0.65, "r")}
LEFT_EASY = {"upperarm_l": clips.aim(0.32, 0.08, -1), "forearm_l": clips.aim(0.25, 0.3, -1)}
GUARD = merge(arms((0.1, 0.8, -0.6), (-0.4, 0.85, 0.0), (0.25, 0.7, -0.65), (-0.2, 0.9, -0.1)), MAUL(-0.55, 0.55, 0.6))
OVERHEAD = merge(arms((0.2, -0.05, 1), (-0.1, -0.5, 0.85), (0.25, -0.1, 1), (0.1, -0.6, 0.8)), MAUL(0.0, -0.75, 0.6),
                 {"spine_02": (-12, 0, 0), "head": (-6, 0, 0)})
SLAM = merge(arms((0.1, 0.9, -0.4), (-0.1, 1, -0.5), (0.2, 0.9, -0.4), (0.0, 1, -0.5)), MAUL(0.0, 0.75, -0.65),
             {"spine_01": (18, 0, 0), "spine_02": (12, 0, 0), "head": (-10, 0, 0)})
READY = merge(CROUCH, GUARD)
DOWN = {"pelvis": (0, 0, -0.06)}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, CARRY, LEFT_EASY, {"head": (-2, 0, 0)})
    b = merge(STAND, CARRY, LEFT_EASY, sym("clavicle", 0, -4, 0), {"head": (-4, 0, 4), "chest": (-3, 0, 0)})
    return _clip(arm, "idle", 80, [(0, a, None), (40, b, {"pelvis": (0, 0, 0.01)})], loop=True)


def idle_fidget_1(arm):
    # Rolls his neck, one way then the other, and shrugs the maul higher.
    base = merge(STAND, CARRY, LEFT_EASY)
    keys = [(0, base, None), (12, merge(base, {"head": (6, 22, 0), "neck": (4, 10, 0)}), None),
            (24, merge(base, {"head": (-8, 0, 0), "neck": (-4, 0, 0)}), None),
            (36, merge(base, {"head": (6, -22, 0), "neck": (4, -10, 0)}), None),
            (46, merge(base, sym("clavicle", 0, -8, 0), {"chest": (-4, 0, 0)}), {"pelvis": (0, 0, 0.02)}), (60, base, None)]
    return _clip(arm, "idle_fidget_1", 60, keys)


def idle_fidget_2(arm):
    # A big stretch, the maul held overhead in both hands, then back to the shoulder.
    base = merge(STAND, CARRY, LEFT_EASY)
    up = merge(STAND, arms((0.25, -0.1, 1), (0.15, -0.2, 1), (0.3, -0.1, 1), (0.2, -0.3, 1)), MAUL(-1, 0.0, 0.1),
               {"spine_02": (-14, 0, 0), "head": (-15, 0, 0)})
    keys = [(0, base, None), (16, up, {"pelvis": (0, 0, 0.03)}), (30, merge(up, {"spine_02": (-18, 0, 0)}), {"pelvis": (0, 0, 0.03)}),
            (46, base, None), (64, base, None)]
    return _clip(arm, "idle_fidget_2", 64, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (13, 0, 0)}), {"pelvis": (0, 0, -0.08)})]
    return _clip(arm, "idle_ready", 40, keys, loop=True)


def _run(frames, legs_scale, lean):
    keys = []
    for f, pose, locs in cycle(run_half(), 20):
        p = dict(pose)
        if legs_scale != 1.0:
            for k, v in pose.items():
                if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple):
                    p[k] = (v[0] * legs_scale, v[1], v[2])
        # The maul stays on the shoulder; only the left arm pumps.
        p = {k: v for k, v in p.items() if not k.endswith("_r") or k.split("_")[0] not in ("upperarm", "forearm")}
        p["spine_01"] = (lean, 0, pose.get("spine_01", (0, 0, 0))[2])
        keys.append((round(f * frames / 20), merge(p, CARRY), locs))
    return keys


def run(arm):
    return _clip(arm, "run", 20, _run(20, 1.0, 12), loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=340.0)


def run_fast(arm):
    return _clip(arm, "run_fast", 16, _run(16, 1.2, 16), loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(arm):
    # The overhead slam: raise, a planted downswing, the hit on `fire` (sim: 12), a heavy recovery.
    keys = [(0, READY, DOWN), (6, merge(STAND, OVERHEAD), {"pelvis": (0, 0.02, 0.02)}), (10, merge(CROUCH, arms((0.15, 0.6, 0.7), (0.0, 0.9, 0.4))), DOWN),
            (12, merge(CROUCH, SLAM, {"thigh_l": (-28, 0, 0), "calf_l": (36, 0, 0)}), {"pelvis": (0, -0.06, -0.14)}),
            (18, merge(CROUCH, SLAM, {"thigh_l": (-28, 0, 0), "calf_l": (36, 0, 0)}), {"pelvis": (0, -0.06, -0.15)}), (30, READY, DOWN)]
    return _clip(arm, "attack_1", 30, keys, markers={"fire": 12})


def attack_2(arm):
    # The sidelong sweep: wind back to the right, sweep across to the left on `fire`.
    wind = merge(CROUCH, arms((0.5, -0.4, -0.2), (0.6, -0.3, 0.2), (0.7, -0.5, 0.0), (0.7, -0.4, 0.3)), MAUL(0.75, -0.55, 0.35),
                 {"spine_01": (6, 0, -35), "spine_02": (2, 0, -20)})
    sweep = merge(CROUCH, arms((0.6, 0.6, -0.1), (0.8, 0.5, 0.0), (-0.3, 0.9, -0.2), (-0.5, 0.8, 0.0)), MAUL(-0.85, 0.5, -0.15),
                  {"spine_01": (8, 0, 30), "spine_02": (4, 0, 20)})
    keys = [(0, READY, DOWN), (8, wind, {"pelvis": (0, 0, -0.07)}), (12, sweep, {"pelvis": (0, -0.04, -0.08)}),
            (18, merge(sweep, {"spine_01": (10, 0, 40)}), {"pelvis": (0, -0.04, -0.08)}), (30, READY, DOWN)]
    return _clip(arm, "attack_2", 30, keys, markers={"fire": 12})


def q(arm):
    # Cleave: a full spin with the maul held out, the hit at mid-sweep (`fire` 6).
    out = merge(CROUCH, arms((0.8, 0.3, -0.2), (0.9, 0.3, 0.0), (0.85, 0.2, -0.2), (0.95, 0.2, 0.0)), MAUL(0.95, 0.2, -0.2))
    keys = [(0, READY, DOWN), (3, merge(out, {"pelvis": (0, 0, -40)}), DOWN)]
    for i, f in enumerate((6, 9, 12, 15)):
        keys.append((f, merge(out, {"pelvis": (0, 0, 50 + 90 * i)}), DOWN))
    keys += [(19, merge(out, {"pelvis": (0, 0, 360)}), DOWN), (26, READY, DOWN)]
    return _clip(arm, "q", 26, keys, markers={"fire": 6})


def w(arm):
    # Second Wind: thumps his chest with the free fist and draws a deep breath (upper body, so
    # he can keep walking: the ability is `mobile`).
    thump = merge(CARRY, arm_l(0.2, 0.6, -0.3, -0.8, 0.4, 0.3), {"spine_02": (-4, 0, 0)})
    breath = merge(CARRY, LEFT_EASY, sym("clavicle", 0, -8, 0), {"chest": (-10, 0, 0), "head": (-14, 0, 0)})
    keys = [(0, merge(CARRY, LEFT_EASY), None), (5, merge(CARRY, arm_l(0.4, 0.3, 0.1, 0.0, 0.5, 0.8)), None), (8, thump, None),
            (16, breath, None), (24, merge(CARRY, LEFT_EASY), None)]
    return _clip(arm, "w", 24, keys, layer="upper", markers={"fire": 8})


def e_start(arm):
    # Lunge: a deep crouch and a launch, the maul swinging up.
    crouch = merge(sym("thigh", -45, -6, 0), sym("calf", 75, 0, 0), sym("foot", -28, 0, 0), GUARD, {"spine_01": (28, 0, 0)})
    launch = merge(sym("thigh", 10, 0, 0), sym("calf", 10, 0, 0), sym("foot", 20, 0, 0), OVERHEAD, {"spine_01": (14, 0, 0)})
    return _clip(arm, "e_start", 6, [(0, crouch, {"pelvis": (0, 0, -0.22)}), (6, launch, {"pelvis": (0, 0, 0.05)})], markers={"fire": 3})


def e_travel(arm):
    # Airborne, knees tucked, the maul high: a slow bob while the sim carries him.
    air = merge(sym("thigh", -55, -6, 0), sym("calf", 80, 0, 0), sym("foot", 10, 0, 0), OVERHEAD, {"spine_01": (16, 0, 0)})
    keys = [(0, air, {"pelvis": (0, 0, 0.1)}), (6, merge(air, {"spine_01": (22, 0, 0)}), {"pelvis": (0, 0, 0.14)})]
    return _clip(arm, "e_travel", 12, keys, loop=True)


def e_land(arm):
    # Maul first: the head hits the ground with him, then he rises into his guard.
    land = merge(sym("thigh", -60, -6, 0), sym("calf", 90, 0, 0), sym("foot", -30, 0, 0), SLAM)
    keys = [(0, land, {"pelvis": (0, -0.05, -0.38)}), (5, merge(CROUCH, SLAM), {"pelvis": (0, -0.04, -0.2)}), (10, READY, DOWN)]
    return _clip(arm, "e_land", 10, keys)


def r(arm):
    # Shockwave: both hands high, a held beat, a ground-shaking slam on `fire` (sim: 13.5); the
    # short hard lock while the shock leaves, then he hauls the maul back up.
    high = merge(STAND, OVERHEAD, {"spine_02": (-18, 0, 0), "head": (-12, 0, 0)}, sym("thigh", -6, -6, 0))
    slam = merge(sym("thigh", -50, -8, 0), sym("calf", 80, 0, 0), sym("foot", -28, 0, 0), SLAM, {"spine_01": (28, 0, 0), "spine_02": (16, 0, 0)})
    keys = [(0, READY, DOWN), (8, high, {"pelvis": (0, 0.03, 0.04)}), (11, high, {"pelvis": (0, 0.03, 0.05)}),
            (14, slam, {"pelvis": (0, -0.08, -0.32)}), (24, slam, {"pelvis": (0, -0.08, -0.33)}), (36, READY, DOWN)]
    return _clip(arm, "r", 36, keys, markers={"fire": 14})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (10, 0, 0), "head": (18, 0, 0)},
              arms((0.15, 0.7, -0.5), (-0.4, 0.8, 0.0), (0.2, 0.7, -0.6), (-0.3, 0.9, -0.1)), MAUL(-0.1, 0.45, -0.9))


def recall(arm):
    breathe = merge(KNEEL, sym("clavicle", 0, -4, 0), {"spine_01": (6, 0, 0), "head": (14, 0, 0)})
    keys = [(0, merge(STAND, CARRY, LEFT_EASY), None), (15, KNEEL, {"pelvis": (0, 0, -0.48)}), (45, breathe, {"pelvis": (0, 0, -0.47)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.48)}), (90, merge(STAND, CARRY, LEFT_EASY), {"pelvis": (0, 0, -0.08)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    # Staggers, drops to his knees, falls forward onto the maul.
    keys = [
        (0, merge(STAND, CARRY, LEFT_EASY), None),
        (6, merge(STAND, arms((0.6, 0.0, 0.2), (0.5, 0.3, 0.4)), MAUL(0.6, -0.3, 0.75), {"spine_01": (-12, 0, 0), "head": (-22, 0, 0)}),
         {"pelvis": (0, 0.05, 0)}),
        (14, merge(sym("thigh", 0, 0, 0), sym("calf", 95, 0, 0), arms((0.25, 0.5, -0.9), (0.2, 0.6, -0.8)), MAUL(0.2, 0.6, -0.75),
                   {"spine_01": (18, 0, 0), "head": (20, 0, 0)}),
         {"pelvis": (0, 0, -0.5)}),
        (24, merge(sym("thigh", -30, 0, 0), sym("calf", 100, 0, 0), arms((0.3, 0.9, 0.2), (0.3, 1, 0.1)), MAUL(0.2, 1, -0.1),
                   {"pelvis": (55, 0, 0), "spine_01": (20, 0, 0)}),
         {"pelvis": (0, -0.3, -0.7)}),
        (34, merge(sym("thigh", -10, 0, 0), sym("calf", 20, 0, 0), arms((0.6, 0.8, 0.1), (0.5, 0.9, 0.0)), MAUL(0.5, 0.85, 0.0),
                   {"pelvis": (82, 0, 0), "head": (-30, 0, 10)}),
         {"pelvis": (0, -0.55, -0.88)}),
    ]
    return _clip(arm, "death", 34, keys)


def respawn(arm):
    low = merge(sym("thigh", -55, -6, 0), sym("calf", 95, 0, 0), sym("foot", -30, 0, 0), GUARD, {"spine_01": (25, 0, 0)})
    keys = [(0, low, {"pelvis": (0, 0, -0.32)}), (10, merge(STAND, GUARD, sym("clavicle", 0, -6, 0)), {"pelvis": (0, 0, -0.06)}),
            (18, merge(STAND, CARRY, LEFT_EASY, {"head": (-8, 0, 0)}), None), (26, merge(STAND, CARRY, LEFT_EASY), None)]
    return _clip(arm, "respawn", 26, keys)


def select(arm):
    # Hoists the maul overhead one-handed and roars, then sets it back on his shoulder.
    hoist = merge(STAND, arm_r(0.15, 0.1, 1, 0.05, 0.1, 1), MAUL(0.1, 0.0, 1), arm_l(0.6, 0.3, -0.3, 0.6, 0.5, 0.2),
                  {"spine_02": (-8, 0, 0), "head": (-22, 0, 0)})
    keys = [(0, merge(STAND, CARRY, LEFT_EASY), None), (10, hoist, {"pelvis": (0, 0, 0.02)}), (24, merge(hoist, {"head": (-30, 0, 0)}), None),
            (36, merge(STAND, CARRY, LEFT_EASY), None), (45, merge(STAND, CARRY, LEFT_EASY, {"head": (-4, 0, -10)}), None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, CARRY, LEFT_EASY)
    out = merge(base, arm_l(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-6, 0, 0)})
    curl = merge(base, arm_l(0.2, 1, 0.0, 0.1, 0.4, 0.9), {"head": (-6, 0, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Flexes his free arm, twice, and admires it.
    base = merge(STAND, CARRY, LEFT_EASY)
    flex = merge(base, arm_l(1, 0.1, 0.05, 0.05, 0.1, 1), {"head": (4, 0, 25)})
    keys = [(0, base, None), (10, flex, None), (16, merge(flex, sym("clavicle", 0, -6, 0)), None), (22, flex, None),
            (28, merge(flex, sym("clavicle", 0, -6, 0)), None), (40, merge(flex, {"head": (10, 0, 35)}), None), (50, base, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    belly = arm_l(0.15, 0.3, -0.9, -0.7, 0.6, 0.2)
    base = merge(STAND, CARRY, LEFT_EASY)
    keys = [(0, base, None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, CARRY, belly, {"spine_01": (-6 * abs(s), 0, 0), "head": (-18 * abs(s), 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, base, None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    # A heavy stomping two-step, the maul held high.
    up = merge(arms((0.25, -0.1, 1), (0.15, -0.2, 1), (0.3, -0.1, 1), (0.2, -0.3, 1)), MAUL(0.1, 0.0, 1))
    a = merge(up, {"thigh_l": (-35, -4, 0), "calf_l": (50, 0, 0), "pelvis": (0, -6, 0)})
    mid = merge(STAND, up)
    b = merge(up, {"thigh_r": (-35, 4, 0), "calf_r": (50, 0, 0), "pelvis": (0, 6, 0)})
    keys = [(0, a, {"pelvis": (0.06, 0, 0.02)}), (12, mid, {"pelvis": (0, 0, -0.06)}), (24, b, {"pelvis": (-0.06, 0, 0.02)}), (36, mid, {"pelvis": (0, 0, -0.06)})]
    return _clip(arm, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e_start, e_travel, e_land, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]
SHARED = ["walk", "cast_utility", "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed", "cc_sleep", "cc_forced_move"]


def bake_shared(arm):
    """Copy the shared library's clips in (10 §7.3): the same bones, so a direct copy."""
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
    rig.use_shape("large")
    mesh.AO_HEIGHT = 2.05
    arm = rig.build_armature(name="rook")
    own = [f(arm).name for f in OWN]
    body = mesh.build_mannequin(arm, name="rook", part_list=parts())
    shared = bake_shared(arm)
    print(f"rook: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(own)} own clips, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
