# SPDX-License-Identifier: AGPL-3.0-or-later
"""Shade (A10): the block-out generator for his model and clips.

    blender -b --python art/champions/shade/build.py -- --out art/champions/shade/shade.blend

A lithe shadow assassin on `biped` v1: fitted violet-black leathers, a dark mask over the lower
face, a curved dagger in each hand, and a long scarf in the team color on an `extra_scarf` chain
that streams behind him. Three dashing abilities (Shadow Step, Veil Step, Execution), each with
its own start / travel / land clips. Like the other block-outs this is a starting point.
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

# Palette (concept.md): violet-black, steel.
SKIN = (0.78, 0.6, 0.5)
HAIR = (0.1, 0.08, 0.1)
LEATHER = (0.17, 0.13, 0.2)
LEATHER_DARK = (0.1, 0.08, 0.12)
VIOLET = (0.32, 0.22, 0.42)
STEEL = (0.74, 0.76, 0.82)
WRAP = (0.24, 0.2, 0.26)
ACCENT = (0.18, 0.52, 0.95)

SCARF = [(0, 0.09, 1.56), (0, 0.17, 1.34), (0, 0.24, 1.1), (0, 0.29, 0.86)]


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", (0, 0, 0.9), (0, 0, 1.13), 12, [(0, .12, .095), (.4, .15, .105), (1, .148, .105)], "cloth", LEATHER_DARK),
        ("pelvis", (0, 0, 1.06), (0, 0, 1.11), 12, [(0, .152, .108), (1, .148, .106)], "cloth", WRAP),
        # Lean torso in fitted leather with a violet panel crossing the chest.
        ("spine_01", (0, 0, 1.11), (0, 0, 1.28), 12, [(0, .13, .092), (.5, .118, .086), (1, .126, .09)], "cloth", LEATHER),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .128, .09), (1, .155, .104)], "cloth", LEATHER),
        ("chest", (0, 0, 1.40), (0, 0, 1.61), 12, [(0, .158, .106), (.35, .168, .11), (.7, .16, .106), (1, .09, .074)], "cloth", LEATHER),
        ("chest", (0.12, -0.105, 1.55), (-0.1, -0.11, 1.2), 4, [(0, .045, .012), (1, .045, .012)], "cloth", VIOLET),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.66), 8, [(0, .048, .048), (1, .043, .043)], "skin", SKIN),
        # The scarf: wound at the neck, tails in the team color.
        ("chest", (0, 0.0, 1.55), (0, 0.0, 1.63), 12, [(0, .095, .085), (1, .085, .08)], "accent", ACCENT),
        ("extra_scarf_1", SCARF[0], SCARF[1], 4, [(0, .06, .014), (1, .065, .014)], "accent", ACCENT),
        ("extra_scarf_2", SCARF[1], SCARF[2], 4, [(0, .065, .014), (1, .06, .013)], "accent", ACCENT),
        ("extra_scarf_3", SCARF[2], SCARF[3], 4, [(0, .06, .013), (.7, .055, .012), (1, .03, .01)], "accent", ACCENT),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (0.15, 0.01, 1.47), (0.2, 0.01, 1.57), 8, [(0, .062, .062), (.5, .066, .062), (1, 0, 0)], "cloth", LEATHER_DARK),
        (ua, (ua, 0), (ua, 1), 8, [(0, .05, .05), (1, .043, .043)], "cloth", LEATHER),
        (fa, (fa, 0), (fa, 1), 8, [(0, .042, .042), (1, .036, .036)], "cloth", WRAP),
        (fa, (fa, .5), (fa, .95), 8, [(0, .046, .046), (1, .042, .042)], "cloth", VIOLET),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .036, .022), (1, .04, .02)], "cloth", LEATHER_DARK),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .04, .02), (1, .026, .014)], "cloth", LEATHER_DARK),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .015, .015), (1, .01, .01)], "cloth", LEATHER_DARK),
        (th, (0.10, 0, 1.02), (th, 1), 10, [(0, .084, .084), (.5, .07, .07), (1, .054, .054)], "cloth", LEATHER_DARK),
        (ca, (ca, 0), (ca, .45), 10, [(0, .054, .054), (1, .05, .05)], "cloth", LEATHER_DARK),
        (ca, (ca, .3), (ca, 1), 10, [(0, .058, .058), (.12, .055, .055), (1, .046, .05)], "cloth", WRAP),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .048, .042), (1, .042, .03)], "cloth", LEATHER_DARK),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .042, .026), (1, .028, .016)], "cloth", LEATHER_DARK),
    ]


def _daggers():
    """A curved dagger in each hand, laid along the hand bone so each pose aims it (hand aim)."""
    out = []
    for side in ("l", "r"):
        h = Vector(next(b[2] for b in rig.bone_table() if b[0] == f"hand_{side}"))
        t = Vector(next(b[3] for b in rig.bone_table() if b[0] == f"hand_{side}"))
        axis = (t - h).normalized()
        grip = h + (t - h) * 0.6
        bend = axis.cross(Vector((0, 1, 0))).normalized() * 0.03
        out += [
            (f"hand_{side}", tuple(grip - axis * 0.05), tuple(grip + axis * 0.06), 6, [(0, .016, .016), (1, .016, .016)], "cloth", WRAP),
            (f"hand_{side}", tuple(grip + axis * 0.06), tuple(grip + axis * 0.07), 4, [(0, .04, .012), (1, .04, .012)], "metal", STEEL),
            (f"hand_{side}", tuple(grip + axis * 0.07), tuple(grip + axis * 0.2 + bend), 4, [(0, .024, .006), (1, .02, .005)], "metal", STEEL),
            (f"hand_{side}", tuple(grip + axis * 0.2 + bend), tuple(grip + axis * 0.32 + bend * 2.5), 4, [(0, .02, .005), (1, 0, 0)], "metal", STEEL),
        ]
    return out


def _head(bm, layers, groups, mats):
    # The shared sculpted head: sharp and lean, short dark hair, a mask over the lower face.
    head.build(bm, layers, groups, mats, head.Head(
        base=(0, -0.01, 1.64), height=0.25, scale=0.96, jaw=0.45, chin=0.95, eye=0.95,
        skin=SKIN, hair=HAIR, hair_style="short", eye_color=(0.36, 0.22, 0.46), lips=(0.6, 0.38, 0.36)))
    h = head.Head(base=(0, -0.01, 1.64), height=0.25, scale=0.96, jaw=0.45, chin=0.95)
    deform, col = layers
    import bmesh
    # The mask: a dark band wrapping the lower face, from under the nose to the chin.
    rings = []
    for t in (-0.06, 0.0, 0.18, 0.4):
        ring = []
        for i in range(14):
            import math
            a = 2 * math.pi * (i + 0.5) / 14
            p = h.surface(t, a)
            c = h.base + Vector((0, 0.01, (p - h.base).z))
            front = max(0.0, -math.sin(a))
            ring.append(bm.verts.new(c + (p - c) * (1.06 + 0.1 * front) + Vector((0, 0, -0.012 if t < 0 else 0))))
        rings.append(ring)
    faces = []
    for r0, r1 in zip(rings, rings[1:]):
        for i in range(14):
            faces.append(bm.faces.new((r0[i], r0[(i + 1) % 14], r1[(i + 1) % 14], r1[i])))
    faces.append(bm.faces.new(list(reversed(rings[0]))))
    faces.append(bm.faces.new(rings[-1]))
    bmesh.ops.recalc_face_normals(bm, faces=faces)
    for r in rings:
        for v in r:
            v[deform][groups["head"]] = 1.0
            v[col] = (*LEATHER_DARK, 1.0)
    for f in faces:
        f.material_index = mats["cloth"]
        f.smooth = False


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + _daggers() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def scarf(back=0.0, sway=0.0):
    return {"extra_scarf_1": (back * 0.5, sway * 0.5, 0), "extra_scarf_2": (back * 0.35, sway * 0.35, 0),
            "extra_scarf_3": (back * 0.3, sway * 0.3, 0)}


def DAGGERS(l_out, l_fwd, l_up, r_out=None, r_fwd=None, r_up=None):
    """Where the blades point (character space)."""
    r = (l_out, l_fwd, l_up) if r_out is None else (r_out, r_fwd, r_up)
    return {"hand_l": clips.aim(l_out, l_fwd, l_up), "hand_r": clips.aim(*r, side="r")}


STAND = merge(sym("thigh", -4, 0, 0), sym("calf", 8, 0, 0), sym("foot", -4, 0, 0), {"spine_01": (4, 0, 0)})
CROUCH = merge(sym("thigh", -22, 0, 0), sym("calf", 40, 0, 0), sym("foot", -18, 0, 0), {"spine_01": (16, 0, 0), "spine_02": (4, 0, 0)})
LOW = merge(arms((0.3, 0.1, -1), (0.2, 0.35, -0.95)), DAGGERS(0.1, 0.2, -1))
READY = merge(CROUCH, arms((0.35, 0.55, -0.6), (0.1, 1, 0.0)), DAGGERS(0.2, 0.8, 0.5), scarf(6))
DOWN = {"pelvis": (0, 0, -0.08)}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, LOW, scarf(0), {"head": (-2, 0, 0)})
    b = merge(STAND, LOW, scarf(2, 3), sym("clavicle", 0, -3, 0), {"head": (-4, 0, 6)})
    return _clip(arm, "idle", 60, [(0, a, None), (30, b, {"pelvis": (0, 0, 0.005)})], loop=True)


def idle_fidget_1(arm):
    # Spins a dagger around his fingers.
    base = merge(STAND, LOW)
    keys = [(0, base, None)] + [(8 + 5 * i, merge(STAND, arm_r(0.3, 0.4, -0.6, 0.0, 0.9, 0.2), DAGGERS(0.1, 0.2, -1, *(
        (0.0, 1, 0), (0.0, 0, 1), (0.0, -1, 0), (0.0, 0, -1))[i % 4]), {"head": (12, 0, -10)}), None) for i in range(8)]
    keys.append((56, base, None))
    return _clip(arm, "idle_fidget_1", 56, keys)


def idle_fidget_2(arm):
    # Glances over each shoulder.
    base = merge(STAND, LOW)
    keys = [(0, base, None), (12, merge(base, {"head": (0, 0, 40), "spine_02": (0, 0, 12)}), None), (24, merge(base, {"head": (0, 0, 40), "spine_02": (0, 0, 12)}), None),
            (34, merge(base, {"head": (0, 0, -40), "spine_02": (0, 0, -12)}), None), (46, merge(base, {"head": (0, 0, -40), "spine_02": (0, 0, -12)}), None), (56, base, None)]
    return _clip(arm, "idle_fidget_2", 56, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (16, merge(READY, {"spine_01": (20, 0, 0)}, scarf(8, 2)), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "idle_ready", 32, keys, loop=True)


def run(arm):
    keys = [(f, merge(p, DAGGERS(0.1, -0.4, -0.8), scarf(30 + (5 if f % 10 < 5 else 0))), l) for f, p, l in cycle(run_half(), 20)]
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=345.0)


def run_fast(arm):
    def scale(pose):
        out = dict(pose)
        for k, v in pose.items():
            if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple):
                out[k] = (v[0] * 1.2, v[1], v[2])
        out["spine_01"] = (18, 0, pose.get("spine_01", (0, 0, 0))[2])
        return out

    half = [(round(f * 0.8), scale(p), l) for f, p, l in run_half()]
    keys = [(f, merge(p, DAGGERS(0.1, -0.5, -0.7), scarf(42)), l) for f, p, l in cycle(half, 16)]
    return _clip(arm, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=460.0)


def _slash(arm, name, right):
    # A quick cross-body slash with one dagger (`fire` 10, sim 10.4).
    a_wind = arm_r(0.7, -0.2, 0.3, 0.6, 0.2, 0.5) if right else arm_l(0.7, -0.2, 0.3, 0.6, 0.2, 0.5)
    a_hit = arm_r(-0.2, 1, -0.1, -0.5, 0.8, -0.2) if right else arm_l(-0.2, 1, -0.1, -0.5, 0.8, -0.2)
    blade_wind = DAGGERS(0.2, 0.8, 0.5, 0.6, 0.2, 0.7) if right else DAGGERS(0.6, 0.2, 0.7, 0.2, 0.8, 0.5)
    blade_hit = DAGGERS(0.2, 0.8, 0.5, -0.7, 0.6, -0.3) if right else DAGGERS(-0.7, 0.6, -0.3, 0.2, 0.8, 0.5)
    turn = 1 if right else -1
    keys = [(0, READY, DOWN), (6, merge(CROUCH, a_wind, blade_wind, {"spine_02": (0, 0, -18 * turn)}, scarf(6)), DOWN),
            (10, merge(CROUCH, a_hit, blade_hit, {"spine_02": (6, 0, 20 * turn)}, scarf(10, 4 * turn)), {"pelvis": (0, -0.04, -0.08)}),
            (15, merge(CROUCH, a_hit, blade_hit, {"spine_02": (4, 0, 24 * turn)}, scarf(8)), DOWN), (24, READY, DOWN)]
    return _clip(arm, name, 24, keys, markers={"fire": 10})


def attack_1(arm):
    return _slash(arm, "attack_1", True)


def attack_2(arm):
    return _slash(arm, "attack_2", False)


LUNGE = merge(sym("thigh", -30, 0, 0), sym("calf", 30, 0, 0), arms((0.2, 1, 0.0), (0.1, 1, 0.1)), DAGGERS(0.0, 1, 0.0),
              {"spine_01": (35, 0, 0), "head": (-20, 0, 0)}, scarf(40))
TUCK = merge(sym("thigh", -90, 0, 0), sym("calf", 110, 0, 0), arms((0.3, 0.6, -0.4), (0.2, 0.9, 0.0)), DAGGERS(0.3, 0.5, 0.6),
             {"spine_01": (30, 0, 0)}, scarf(-15))


def _dash_trio(arm, slot, launch, travel, land, fire, land_pose):
    """`<slot>_start` (with `fire` at the launch), a looping `_travel`, `_land`."""
    start = _clip(arm, f"{slot}_start", 5, [(0, merge(CROUCH, launch, scarf(5)), {"pelvis": (0, 0, -0.18)}), (5, travel, {"pelvis": (0, 0, -0.1)})],
                  markers={"fire": fire})
    trav = _clip(arm, f"{slot}_travel", 10, [(0, travel, {"pelvis": (0, 0, -0.1)}), (5, merge(travel, scarf(46, 4)), {"pelvis": (0, 0, -0.12)})], loop=True)
    end = _clip(arm, f"{slot}_land", 12, [(0, land_pose, {"pelvis": (0, -0.04, -0.2)}), (6, land_pose, {"pelvis": (0, -0.04, -0.18)}), (12, READY, DOWN)])
    return start, trav, end


def q_trio(arm):
    # Shadow Step: a low lunge, both blades leading; strikes on arrival.
    strike = merge(CROUCH, arms((0.1, 1, -0.2), (0.0, 1, -0.3)), DAGGERS(0.0, 1, -0.4), {"spine_01": (28, 0, 0)}, scarf(20))
    return _dash_trio(arm, "q", arms((0.4, -0.3, -0.5), (0.3, 0.0, -0.6)), LUNGE, None, 2, strike)


def w(arm):
    # Fan of Blades: a crouched spin, arms flung wide (`fire` 4, sim 4.5).
    wide = merge(CROUCH, arms((1, 0.1, 0.0), (1, 0.2, 0.1)), DAGGERS(1, 0.3, 0.0))
    keys = [(0, READY, DOWN), (2, merge(wide, {"pelvis": (0, 0, -60)}), DOWN), (4, merge(wide, {"pelvis": (0, 0, 60)}), {"pelvis": (0, 0, -0.1)}),
            (7, merge(wide, {"pelvis": (0, 0, 180)}), DOWN), (10, merge(wide, {"pelvis": (0, 0, 300)}), DOWN), (13, merge(wide, {"pelvis": (0, 0, 360)}), DOWN),
            (20, READY, DOWN)]
    return _clip(arm, "w", 20, keys, markers={"fire": 4})


def e_trio(arm):
    # Veil Step: a low, smoky glide sideways and through.
    glide = merge(sym("thigh", -40, 0, 0), sym("calf", 70, 0, 0), arms((0.6, -0.4, -0.3), (0.5, -0.5, -0.2)), DAGGERS(0.2, -0.6, -0.6),
                  {"spine_01": (40, 0, 0), "head": (-26, 0, 0)}, scarf(50))
    return _dash_trio(arm, "e", arms((0.3, 0.3, -0.7), (0.2, 0.5, -0.6)), glide, None, 2, merge(CROUCH, LOW, scarf(10)))


def r_trio(arm):
    # Execution: a leap with both blades raised, driven down on landing.
    high = merge(TUCK, arms((0.2, 0.1, 1), (0.1, -0.2, 1)), DAGGERS(0.0, -0.4, 0.9), scarf(-20))
    stab = merge(sym("thigh", -55, 0, 0), sym("calf", 85, 0, 0), arms((0.1, 0.8, -0.5), (0.0, 0.6, -0.8)), DAGGERS(0.0, 0.3, -1),
                 {"spine_01": (32, 0, 0)}, scarf(25))
    return _dash_trio(arm, "r", arms((0.3, -0.2, 0.6), (0.2, -0.3, 0.8)), high, None, 2, stab)


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (10, 0, 0), "head": (16, 0, 0)}, LOW, scarf(-6))


def recall(arm):
    keys = [(0, merge(STAND, LOW), None), (15, KNEEL, {"pelvis": (0, 0, -0.45)}), (45, merge(KNEEL, {"head": (22, 0, 0)}), {"pelvis": (0, 0, -0.44)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.45)}), (90, merge(STAND, LOW), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    keys = [
        (0, merge(STAND, LOW), None),
        (5, merge(arms((0.6, -0.2, 0.3), (0.5, 0.1, 0.5)), {"spine_01": (-15, 0, 0), "head": (-20, 0, 0)}, scarf(-10)), {"pelvis": (0, 0.04, 0)}),
        (12, merge(sym("thigh", 0, 0, 0), sym("calf", 90, 0, 0), arms((0.2, 0.4, -1), (0.15, 0.5, -1)), {"spine_01": (15, 0, 0), "head": (25, 0, 0)}, scarf(5)),
         {"pelvis": (0, 0, -0.45)}),
        (22, merge(sym("thigh", -20, 0, 0), sym("calf", 70, 0, 0), arms((0.5, 0.2, -0.6), (0.4, 0.3, -0.7)), {"pelvis": (0, 60, 0), "spine_01": (5, 15, 0)}, scarf(10, 10)),
         {"pelvis": (0.25, 0, -0.65)}),
        (30, merge(sym("thigh", -30, 0, 0), sym("calf", 50, 0, 0), arms((0.7, 0.1, -0.3), (0.6, 0.2, -0.3)), {"pelvis": (0, 85, 0), "head": (0, 10, 0)}, scarf(20, 15)),
         {"pelvis": (0.45, 0, -0.82)}),
    ]
    return _clip(arm, "death", 30, keys)


def respawn(arm):
    keys = [(0, merge(CROUCH, LOW), {"pelvis": (0, 0, -0.2)}), (10, merge(STAND, arms((0.9, 0.3, 0.1), (0.9, 0.4, 0.2)), DAGGERS(0.9, 0.3, 0.3), scarf(10)), None),
            (20, merge(STAND, LOW), None)]
    return _clip(arm, "respawn", 20, keys)


def select(arm):
    # Steps out of a crouch, twirls both daggers and crosses them before his masked face.
    cross = merge(STAND, arms((0.1, 0.6, 0.3), (-0.6, 0.6, 0.6)), DAGGERS(0.5, 0.2, 0.8, 0.5, 0.2, 0.8), {"head": (6, 0, 0)}, scarf(8, 3))
    keys = [(0, merge(CROUCH, LOW), {"pelvis": (0, 0, -0.18)}), (12, merge(STAND, LOW, scarf(6)), None), (20, cross, None), (38, cross, None),
            (45, merge(STAND, LOW), None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, LOW)
    out = merge(base, arm_l(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-5, 0, 0)})
    curl = merge(base, arm_l(0.2, 1, 0.0, 0.1, 0.5, 0.8), {"head": (-5, 0, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Juggles both daggers.
    keys = [(0, merge(STAND, LOW), None)]
    for i in range(6):
        up_l = i % 2 == 0
        keys.append((6 + 6 * i, merge(STAND, arms((0.3, 0.5, 0.2 if up_l else -0.4), (0.0, 0.4, 0.9 if up_l else 0.3),
                                                  (0.3, 0.5, -0.4 if up_l else 0.2), (0.0, 0.4, 0.3 if up_l else 0.9)), DAGGERS(0.0, 0.2, 1), {"head": (-14, 0, 0)}), None))
    keys.append((50, merge(STAND, LOW), None))
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    keys = [(0, merge(STAND, LOW), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, LOW, {"spine_01": (8 * s, 0, 0), "head": (14 * s, 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, merge(STAND, LOW), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    a = merge(CROUCH, arms((0.8, 0.2, 0.3), (0.8, 0.3, 0.5)), DAGGERS(1, 0.2, 0.3), {"pelvis": (0, 0, 25)}, scarf(10, -10))
    b = merge(CROUCH, arms((0.8, 0.2, 0.3), (0.8, 0.3, 0.5)), DAGGERS(1, 0.2, 0.3), {"pelvis": (0, 0, -25)}, scarf(10, 10))
    keys = [(0, a, DOWN), (12, merge(STAND, LOW), None), (24, b, DOWN), (36, merge(STAND, LOW), None)]
    return _clip(arm, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q_trio, w, e_trio, r_trio,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]
SHARED = ["walk", "cast_utility", "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed", "cc_sleep", "cc_forced_move"]


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
    arm = rig.build_armature(name="shade")
    rig.add_chain(arm, "scarf", "chest", SCARF)
    for f in OWN:
        f(arm)
    body = mesh.build_mannequin(arm, name="shade", part_list=parts())
    shared = bake_shared(arm)
    print(f"shade: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(bpy.data.actions)} actions, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
