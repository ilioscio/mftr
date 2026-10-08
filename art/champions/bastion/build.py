# SPDX-License-Identifier: AGPL-3.0-or-later
"""Bastion (A8): the block-out generator for his model and clips.

    blender -b --python art/champions/bastion/build.py -- --out art/champions/bastion/bastion.blend

A stone guardian: a golem of blocky masonry on the `biped` bones in the `large` shape (made
bulkier still), iron-banded and mossy, with no face but a glowing rune visor. A chain grapple
wraps his right fist and hangs to a hook; a stone slab shield rides his left forearm. Blocky
four-sided segments throughout: he reads as a wall. Like the other block-outs this is a starting
point: run once, then the .blend is the source of truth.
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

# Palette (concept.md): slate stone in three shades, moss, iron, a pale rune glow.
STONE = (0.40, 0.42, 0.46)
STONE_DARK = (0.28, 0.30, 0.33)
STONE_LIGHT = (0.50, 0.52, 0.55)
MOSS = (0.28, 0.40, 0.20)
IRON = (0.26, 0.27, 0.30)
RUNE = (0.55, 0.88, 1.0)
ACCENT = (0.18, 0.52, 0.95)

SCALE, BULK = 1.08, 1.12  # on top of the large shape
WIDE, DEEP = 1.22 * BULK, 1.18


def pt(p):
    return p if isinstance(p[0], str) else rig.shaped(p)


def part(bone, a, b, sides, prof, slot, color):
    return (bone, pt(a), pt(b), sides, [(t, ru * WIDE, rv * DEEP) for t, ru, rv in prof], slot, color)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        # Hips and belly: rounded boulders, the chest a great slab-sided rock.
        part("pelvis", (0, 0, 0.86), (0, 0, 1.15), 8, [(0, .18, .14), (.3, .23, .17), (.75, .245, .18), (1, .22, .16)], "cloth", STONE_DARK),
        part("spine_01", (0, 0.01, 1.11), (0, 0.0, 1.31), 8, [(0, .22, .16), (.5, .26, .19), (1, .25, .18)], "cloth", STONE),
        part("spine_02", (0, 0, 1.27), (0, 0, 1.45), 8, [(0, .27, .19), (.5, .31, .21), (1, .32, .215)], "cloth", STONE),
        part("chest", (0, 0.01, 1.42), (0, 0.02, 1.70), 8, [(0, .32, .22), (.3, .35, .24), (.7, .34, .23), (1, .26, .19)], "cloth", STONE_LIGHT),
        part("spine_02", (0, 0, 1.40), (0, 0, 1.465), 8, [(0, .325, .222), (1, .33, .225)], "metal", IRON),
        part("chest", (0.13, 0.02, 1.62), (0.16, 0.04, 1.73), 6, [(0, .13, .1), (.6, .12, .09), (1, .06, .05)], "cloth", MOSS),
        part("chest", (-0.17, 0.05, 1.6), (-0.19, 0.06, 1.68), 6, [(0, .08, .07), (1, .04, .04)], "cloth", MOSS),
        # A rune glowing on his chest, and a crack of light down from it.
        part("chest", (0, -0.2, 1.55), (0, -0.235, 1.55), 4, [(0, .055, .055), (.5, .055, .055), (1, 0, 0)], "emissive", RUNE),
        part("chest", (0.02, -0.215, 1.5), (0.04, -0.22, 1.43), 4, [(0, .012, .008), (1, .008, .006)], "emissive", RUNE),
        # The team-colored banner hanging from an iron belt.
        part("pelvis", (0, -0.17, 1.10), (0, -0.19, 0.72), 4, [(0, .15, .02), (1, .16, .02)], "accent", ACCENT),
        part("pelvis", (0, 0, 1.09), (0, 0, 1.16), 8, [(0, .245, .178), (1, .245, .178)], "metal", IRON),
        # The head: a rough rock set low between the shoulders, a heavy brow over a rune visor,
        # a crown of moss.
        part("head", (0, -0.02, 1.58), (0, -0.01, 1.86), 8, [(0, .12, .12), (.35, .145, .14), (.75, .13, .13), (1, .09, .09)], "cloth", STONE),
        part("head", (0, -0.1, 1.78), (0, -0.16, 1.78), 6, [(0, .14, .04), (1, .12, .03)], "cloth", STONE_DARK),
        part("head", (0, -0.125, 1.72), (0, -0.155, 1.72), 4, [(0, .1, .016), (1, .09, .013)], "emissive", RUNE),
        part("head", (0, 0.0, 1.84), (0, 0.01, 1.9), 6, [(0, .11, .1), (1, .05, .05)], "cloth", MOSS),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        # Boulder shoulders, moss on top, a team-colored band of paint.
        part(ua, (0.22, 0.0, 1.52), (0.42, 0.0, 1.72), 8, [(0, .13, .13), (.35, .18, .17), (.75, .16, .15), (1, 0, 0)], "cloth", STONE_LIGHT),
        part(ua, (0.33, 0.0, 1.69), (0.36, 0.0, 1.77), 6, [(0, .1, .09), (1, .05, .05)], "cloth", MOSS),
        part(ua, (0.29, 0.0, 1.5), (0.32, 0.0, 1.62), 8, [(0, .175, .165), (1, .175, .165)], "accent", ACCENT),
        part(ua, (ua, 0), (ua, 1), 6, [(0, .11, .1), (.5, .12, .11), (1, .09, .09)], "cloth", STONE_DARK),
        # Massive forearms with rune lines, iron bands, fists like boulders.
        part(fa, (fa, -0.05), (fa, 1), 8, [(0, .11, .11), (.4, .13, .13), (.8, .145, .14), (1, .13, .13)], "cloth", STONE),
        part(fa, (fa, .5), (fa, .58), 8, [(0, .15, .145), (1, .15, .145)], "metal", IRON),
        part(fa, (fa, .15), (fa, .42), 4, [(0, .02, .145), (1, .02, .145)], "emissive", RUNE),
        part("hand_l", ("hand_l", -0.2), ("hand_l", 1.5), 8, [(0, .1, .09), (.5, .12, .1), (1, .09, .08)], "cloth", STONE_DARK),
        # Legs: thick pillars on wide rock feet, moss at the knees.
        part(th, (0.11, 0, 1.0), (th, 1), 8, [(0, .13, .13), (.5, .12, .12), (1, .1, .1)], "cloth", STONE_DARK),
        part(ca, (ca, -0.05), (ca, 1), 8, [(0, .1, .1), (.4, .12, .12), (1, .11, .11)], "cloth", STONE),
        part(ca, (ca, -0.06), (ca, 0.08), 6, [(0, .12, .12), (1, .11, .11)], "cloth", MOSS),
        part("foot_l", ("foot_l", -0.3), ("foot_l", 1.1), 8, [(0, .11, .062), (.5, .12, .06), (1, .1, .048)], "cloth", STONE_DARK),
        part("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .1, .026), (1, .08, .02)], "cloth", STONE_DARK),
    ]


def _gear():
    """The grapple on the right fist and the slab shield on the left forearm."""
    # The bone table is already placed for the current shape.
    hr = Vector(next(b[2] for b in rig.bone_table() if b[0] == "hand_r"))
    ht = Vector(next(b[3] for b in rig.bone_table() if b[0] == "hand_r"))
    tip = ht + (ht - hr).normalized() * 0.06
    g = []
    # Chain wound around the right forearm, a few links hanging from the fist, the hook.
    for t in (0.25, 0.45, 0.65):
        g.append(("forearm_r", ("forearm_r", t), ("forearm_r", t + 0.07), 4, [(0, .15 * WIDE, .15), (1, .15 * WIDE, .15)], "metal", IRON))
    drop = Vector((0, 0, -0.42))
    g.append(("hand_r", tuple(tip), tuple(tip + drop), 4, [(0, .022, .022), (1, .022, .022)], "metal", IRON))
    hook = tip + drop
    g.append(("hand_r", tuple(hook), tuple(hook + Vector((0, -0.1, -0.05))), 4, [(0, .03, .03), (1, .02, .02)], "metal", IRON))
    g.append(("hand_r", tuple(hook + Vector((0, -0.1, -0.05))), tuple(hook + Vector((0, -0.12, 0.08))), 4, [(0, .02, .02), (1, 0, 0)], "metal", IRON))
    # The slab shield, on the outside of the left forearm: long along the arm, wide front to
    # back, thin; a team-colored emblem on its face.
    fa = [b for b in rig.bone_table() if b[0] == "forearm_l"][0]
    a, b = Vector(fa[2]), Vector(fa[3])
    axis = (b - a).normalized()
    out = Vector((1, 0, 0))
    out = (out - axis * out.dot(axis)).normalized()
    off = out * 0.17 * WIDE
    g.append(("forearm_l", tuple(a + off - axis * 0.12), tuple(b + off + axis * 0.08), 6, [(0, .035, .28), (.1, .045, .32), (.9, .045, .32), (1, .035, .28)], "cloth", STONE_LIGHT))
    g.append(("forearm_l", tuple(a + off * 1.25 + axis * 0.15), tuple(a + off * 1.25 + axis * 0.4), 4, [(0, .03, .12), (1, .03, .12)], "accent", ACCENT))
    return g


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    # Rounded rocks get extra sides and smoothed in-between rings, like the other models.
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 6 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + _gear()


# --- Poses ----------------------------------------------------------------------------------

STAND = merge(sym("thigh", -3, -6, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"spine_01": (4, 0, 0)})
CROUCH = merge(sym("thigh", -18, -8, 0), sym("calf", 30, 0, 0), sym("foot", -12, 0, 0), {"spine_01": (12, 0, 0), "spine_02": (4, 0, 0)})
HEAVY = arms((0.45, 0.1, -0.9), (0.3, 0.4, -0.85))
GUARD = arms((0.3, 0.6, -0.5), (-0.5, 0.8, 0.2), (0.4, 0.4, -0.7), (0.1, 0.9, -0.2))
READY = merge(CROUCH, GUARD)
DOWN = {"pelvis": (0, 0, -0.07)}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, HEAVY, {"head": (-2, 0, 0)})
    b = merge(STAND, HEAVY, sym("clavicle", 0, -3, 0), {"head": (-4, 0, 6), "chest": (-2, 0, 0)})
    return _clip(arm, "idle", 90, [(0, a, None), (45, b, {"pelvis": (0, 0, 0.015)})], loop=True)


def idle_fidget_1(arm):
    # Grinds his head side to side; dust would sift off.
    base = merge(STAND, HEAVY)
    keys = [(0, base, None), (14, merge(base, {"head": (0, 18, 0), "neck": (0, 8, 0)}), None), (30, merge(base, {"head": (0, -18, 0), "neck": (0, -8, 0)}), None),
            (44, merge(base, {"head": (-10, 0, 0)}), None), (60, base, None)]
    return _clip(arm, "idle_fidget_1", 60, keys)


def idle_fidget_2(arm):
    # Raps his fist on his shield.
    base = merge(STAND, HEAVY)
    shield_up = arm_l(0.2, 0.6, -0.3, -0.6, 0.6, 0.3)
    keys = [(0, base, None), (12, merge(STAND, shield_up, arm_r(0.35, 0.3, -0.6, -0.3, 0.8, 0.1)), None)]
    for i in range(3):
        keys += [(16 + 8 * i, merge(STAND, shield_up, arm_r(0.3, 0.4, -0.5, -0.6, 0.7, 0.0)), None),
                 (20 + 8 * i, merge(STAND, shield_up, arm_r(0.35, 0.3, -0.6, -0.3, 0.8, 0.1)), None)]
    keys += [(52, base, None)]
    return _clip(arm, "idle_fidget_2", 52, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (24, merge(READY, {"spine_01": (15, 0, 0)}), {"pelvis": (0, 0, -0.09)})]
    return _clip(arm, "idle_ready", 48, keys, loop=True)


def _run(frames, legs_scale, lean):
    keys = []
    for f, pose, locs in cycle(run_half(), 20):
        p = {k: ((v[0] * legs_scale, v[1], v[2]) if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple) else v)
             for k, v in pose.items()}
        p["spine_01"] = (lean, 0, pose.get("spine_01", (0, 0, 0))[2])
        keys.append((round(f * frames / 20), p, locs))
    return keys


def run(arm):
    return _clip(arm, "run", 20, _run(20, 0.85, 14), loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=335.0)


def run_fast(arm):
    return _clip(arm, "run_fast", 16, _run(16, 1.05, 18), loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(arm):
    # A hammer-fist: the right fist rises and comes down on `fire` (sim: 13.8).
    up = merge(CROUCH, arm_r(0.4, -0.1, 0.8, 0.0, 0.3, 1), arm_l(0.3, 0.6, -0.5, -0.5, 0.8, 0.2), {"spine_02": (-8, 0, -10)})
    hit = merge(CROUCH, arm_r(0.2, 0.9, -0.3, 0.0, 0.8, -0.6), arm_l(0.4, 0.2, -0.8, 0.2, 0.5, -0.8), {"spine_01": (18, 0, 0), "spine_02": (8, 0, 10)})
    keys = [(0, READY, DOWN), (8, up, {"pelvis": (0, 0.02, -0.03)}), (14, hit, {"pelvis": (0, -0.06, -0.12)}), (20, hit, {"pelvis": (0, -0.06, -0.12)}), (32, READY, DOWN)]
    return _clip(arm, "attack_1", 32, keys, markers={"fire": 14})


def attack_2(arm):
    # A shield bash with the left arm.
    wind = merge(CROUCH, arm_l(0.5, -0.3, -0.3, 0.4, 0.2, -0.2), arm_r(0.35, 0.4, -0.6, 0.0, 0.8, -0.2), {"spine_02": (0, 0, 22)})
    bash = merge(CROUCH, arm_l(0.1, 1, 0.0, -0.4, 0.9, 0.1), arm_r(0.45, -0.1, -0.8, 0.3, 0.2, -0.8), {"spine_02": (8, 0, -18)}, {"thigh_l": (-30, 0, 0), "calf_l": (30, 0, 0)})
    keys = [(0, READY, DOWN), (8, wind, DOWN), (14, bash, {"pelvis": (0, -0.08, -0.08)}), (20, bash, {"pelvis": (0, -0.08, -0.08)}), (32, READY, DOWN)]
    return _clip(arm, "attack_2", 32, keys, markers={"fire": 14})


def q(arm):
    # Grapple: the hook swung back, flung forward on `fire` (sim 10.5), then hauled in.
    swing = merge(CROUCH, arm_r(0.6, -0.5, 0.3, 0.5, -0.3, 0.6), arm_l(0.3, 0.6, -0.5, -0.5, 0.8, 0.2), {"spine_02": (-4, 0, -25)})
    fling = merge(CROUCH, arm_r(0.1, 1, 0.2, 0.0, 1, 0.25), arm_l(0.4, 0.2, -0.8, 0.2, 0.5, -0.8), {"spine_02": (8, 0, 18)}, {"thigh_l": (-26, 0, 0), "calf_l": (24, 0, 0)})
    haul = merge(CROUCH, arm_r(0.4, 0.1, -0.5, -0.3, 0.2, 0.4), arm_l(0.4, 0.2, -0.8, 0.2, 0.5, -0.8), {"spine_01": (-6, 0, 0), "spine_02": (-6, 0, -10)})
    keys = [(0, READY, DOWN), (6, swing, DOWN), (11, fling, {"pelvis": (0, -0.05, -0.08)}), (18, fling, {"pelvis": (0, -0.05, -0.08)}),
            (26, haul, {"pelvis": (0, 0.06, -0.06)}), (34, READY, DOWN)]
    return _clip(arm, "q", 34, keys, markers={"fire": 11})


def w(arm):
    # Bulwark: instant, so it plays as a pulse from `fire`: the shield slams up in front and he
    # sets his feet behind it.
    brace = merge(CROUCH, arm_l(0.15, 0.9, -0.1, -0.6, 0.7, 0.3), arm_r(0.35, 0.3, -0.6, -0.4, 0.8, 0.2), {"spine_01": (14, 0, 0), "head": (-6, 0, 0)},
                  sym("thigh", -24, -10, 0), sym("calf", 36, 0, 0))
    keys = [(0, READY, DOWN), (4, brace, {"pelvis": (0, 0, -0.12)}), (14, brace, {"pelvis": (0, 0, -0.13)}), (24, READY, DOWN)]
    return _clip(arm, "w", 24, keys, layer="upper", markers={"fire": 4})


def e(arm):
    # Tremor: a knee raised high and a stomp on `fire` (sim 7.5).
    lift = merge(STAND, {"thigh_r": (-70, 0, 0), "calf_r": (80, 0, 0), "foot_r": (-10, 0, 0)}, arms((0.6, 0.2, -0.3), (0.5, 0.4, 0.0)), {"spine_01": (-6, 0, 0)})
    stomp = merge(CROUCH, {"thigh_r": (-20, 0, 0), "calf_r": (30, 0, 0)}, arms((0.6, 0.3, -0.5), (0.5, 0.5, -0.3)), {"spine_01": (16, 0, 0), "head": (6, 0, 0)})
    keys = [(0, READY, DOWN), (5, lift, {"pelvis": (0, 0, 0.04)}), (8, stomp, {"pelvis": (0, 0, -0.14)}), (16, stomp, {"pelvis": (0, 0, -0.14)}), (26, READY, DOWN)]
    return _clip(arm, "e", 26, keys, markers={"fire": 8})


def r(arm):
    # Upheaval: both fists raised and driven into the ground (`fire` 8, sim 7.5); the earth
    # erupts later, on the area's own delay.
    raise_ = merge(STAND, arms((0.3, 0.1, 1), (0.1, 0.2, 1)), {"spine_02": (-12, 0, 0), "head": (-10, 0, 0)})
    slam = merge(sym("thigh", -55, -8, 0), sym("calf", 85, 0, 0), sym("foot", -28, 0, 0), arms((0.2, 0.8, -0.6), (0.1, 0.6, -0.8)),
                 {"spine_01": (30, 0, 0), "spine_02": (16, 0, 0)})
    keys = [(0, READY, DOWN), (5, raise_, {"pelvis": (0, 0.02, 0.05)}), (8, slam, {"pelvis": (0, -0.06, -0.38)}), (20, slam, {"pelvis": (0, -0.06, -0.39)}),
            (32, READY, DOWN)]
    return _clip(arm, "r", 32, keys, markers={"fire": 8})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (10, 0, 0), "head": (16, 0, 0)},
              arms((0.3, 0.6, -0.6), (-0.4, 0.8, 0.1), (0.3, 0.5, -0.8), (0.2, 0.4, -0.9)))


def recall(arm):
    still = merge(KNEEL, {"head": (24, 0, 0)})
    keys = [(0, merge(STAND, HEAVY), None), (15, KNEEL, {"pelvis": (0, 0, -0.5)}), (45, still, {"pelvis": (0, 0, -0.5)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.5)}), (90, merge(STAND, HEAVY), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    # Crumbles: sags to his knees, topples forward like a felled wall.
    keys = [
        (0, merge(STAND, HEAVY), None),
        (8, merge(STAND, arms((0.6, 0.1, -0.5), (0.5, 0.3, -0.6)), {"spine_01": (-8, 0, 0), "head": (-14, 0, 0)}), {"pelvis": (0, 0.03, -0.02)}),
        (18, merge(sym("thigh", 0, 0, 0), sym("calf", 95, 0, 0), arms((0.35, 0.4, -0.9), (0.3, 0.5, -0.8)), {"spine_01": (14, 0, 0), "head": (18, 0, 0)}),
         {"pelvis": (0, 0, -0.52)}),
        (28, merge(sym("thigh", -30, 0, 0), sym("calf", 100, 0, 0), arms((0.4, 0.9, 0.1), (0.4, 1, 0.0)), {"pelvis": (60, 0, 0), "spine_01": (16, 0, 0)}),
         {"pelvis": (0, -0.35, -0.72)}),
        (36, merge(sym("thigh", -10, 0, 0), sym("calf", 20, 0, 0), arms((0.6, 0.8, 0.1), (0.6, 0.9, 0.0)), {"pelvis": (86, 0, 0)}),
         {"pelvis": (0, -0.6, -0.9)}),
    ]
    return _clip(arm, "death", 36, keys)


def respawn(arm):
    low = merge(sym("thigh", -55, -6, 0), sym("calf", 95, 0, 0), sym("foot", -30, 0, 0), GUARD, {"spine_01": (25, 0, 0)})
    keys = [(0, low, {"pelvis": (0, 0, -0.34)}), (12, merge(STAND, arms((0.8, 0.1, 0.2), (0.7, 0.2, 0.5)), {"head": (-10, 0, 0)}), {"pelvis": (0, 0, -0.04)}),
            (20, merge(STAND, HEAVY), None), (28, merge(STAND, HEAVY), None)]
    return _clip(arm, "respawn", 28, keys)


def select(arm):
    # Raises the shield and strikes it with his fist; the rune visor flares (in the shader's time).
    shield = arm_l(0.2, 0.7, 0.1, -0.5, 0.6, 0.5)
    keys = [(0, merge(STAND, HEAVY), None), (12, merge(STAND, shield, arm_r(0.4, -0.1, 0.7, 0.0, 0.2, 1)), None),
            (18, merge(STAND, shield, arm_r(0.2, 0.7, 0.2, -0.5, 0.7, 0.3), {"spine_02": (4, 0, -10)}), None),
            (30, merge(STAND, shield, arm_r(0.2, 0.7, 0.2, -0.5, 0.7, 0.3), {"head": (-12, 0, 0)}), None), (45, merge(STAND, HEAVY), None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, HEAVY)
    out = merge(base, arm_r(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-5, 0, 0)})
    curl = merge(base, arm_r(0.2, 1, 0.0, 0.1, 0.5, 0.8), {"head": (-5, 0, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Pretends to be a statue, then nudges a nearby "wall" (his own shield).
    base = merge(STAND, HEAVY)
    statue = merge(STAND, arms((0.9, 0.0, 0.3), (0.9, 0.0, 0.4)))
    keys = [(0, base, None), (10, statue, None), (34, statue, None), (40, merge(statue, {"head": (0, 25, 0)}), None), (50, base, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    keys = [(0, merge(STAND, HEAVY), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, HEAVY, {"spine_01": (-6 * abs(s), 0, 0), "head": (-16 * abs(s), 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, merge(STAND, HEAVY), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    # A slow, ground-shaking side-to-side stomp.
    a = merge(HEAVY, {"thigh_l": (-40, -6, 0), "calf_l": (55, 0, 0), "pelvis": (0, -8, 0)})
    mid = merge(STAND, HEAVY)
    b = merge(HEAVY, {"thigh_r": (-40, 6, 0), "calf_r": (55, 0, 0), "pelvis": (0, 8, 0)})
    keys = [(0, a, {"pelvis": (0.06, 0, 0.02)}), (12, mid, {"pelvis": (0, 0, -0.08)}), (24, b, {"pelvis": (-0.06, 0, 0.02)}), (36, mid, {"pelvis": (0, 0, -0.08)})]
    return _clip(arm, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e, r,
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
    rig.use_shape("large", scale=SCALE, x=BULK)
    mesh.AO_HEIGHT = 2.2
    arm = rig.build_armature(name="bastion")
    own = [f(arm).name for f in OWN]
    body = mesh.build_mannequin(arm, name="bastion", part_list=parts())
    shared = bake_shared(arm)
    print(f"bastion: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(own)} own clips, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
