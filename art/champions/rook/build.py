# SPDX-License-Identifier: AGPL-3.0-or-later
"""Rook (A6, rebuilt for 14): the generator for his model and clips.

    blender -b --python art/champions/rook/build.py -- --out art/champions/rook/rook.blend

A block on legs (14 §5): a grizzled veteran on the `biped` bones in heroic brute proportions
(short thick legs, a barrel chest, slab pauldrons standing above a head sunk into an iron
gorget, long arms and huge gauntlets) carrying a two-handed maul with a big square stone head
banded in iron and cut with crimson runes. His signature is crimson (14 §4): the tabard, the
runes and every effect he makes.

Weight (14 §3): the maul rests on the ground when he isn't using it, both hands are on it
whenever it works (the left hand is solved onto the haft), blows have a cock, a 1–2 frame
strike and an impact hold with the head left in the ground, and his feet are planted by IK.

Like the other generators this is a starting point: run once, then the .blend is the source of
truth. Pose values follow `mftr_blender.clips` (arms aimed in character space; everything else
in degrees about the armature axes; IK targets in armature space).
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Quaternion, Vector  # noqa: E402

from mftr_blender import clips, head, mesh, rig  # noqa: E402
from mftr_blender.clips import at, grip, ik, merge, sym  # noqa: E402
from mftr_blender.library import _clip, gait  # noqa: E402
from mftr_blender.mesh import box_pair, box_part, path_pair, path_part  # noqa: E402

# Palette (concept.md): iron, oxblood and crimson.
# Neutrals are dark: the arena's sun lights the top faces the gameplay camera sees (10 §2).
SKIN = (0.70, 0.48, 0.36)
BEARD = (0.30, 0.24, 0.20)
GAMBESON = (0.36, 0.28, 0.19)
CRIMSON = (0.55, 0.06, 0.06)
CRIMSON_DARK = (0.28, 0.035, 0.04)
DARK = (0.13, 0.11, 0.10)
LEATHER = (0.31, 0.19, 0.10)
IRON = (0.30, 0.31, 0.35)
IRON_DARK = (0.15, 0.16, 0.19)
BRASS = (0.62, 0.45, 0.18)
STONE = (0.40, 0.38, 0.35)
RUNE = (1.0, 0.22, 0.16)
ACCENT = (0.18, 0.52, 0.95)

# Heroic brute proportions (14 §2.2): hips and knees lower, the neck short so the head sits in
# the gorget, broad and deep, the shoulders pushed out, long arms, huge hands.
SHAPE = {
    "heights": [(0.0, 0.0), (0.03, 0.03), (0.09, 0.09), (0.54, 0.46), (1.00, 0.89), (1.12, 1.01),
                (1.26, 1.16), (1.40, 1.31), (1.56, 1.48), (1.64, 1.53), (1.88, 1.78), (2.15, 2.05)],
    "x": 1.26, "y": 1.22, "scale": 1.1, "shoulder": 0.08, "arm": 1.06, "hand": 1.5,
}
rig.use_shape(SHAPE)
BONES = {b[0]: b for b in rig.bone_table()}


def bone(name):
    _, _, h, t, _, _ = BONES[name]
    return Vector(h), Vector(t)


def sp(p):
    """A biped-space point placed for Rook's shape."""
    return rig.shaped(p)


HAND_LEN = (bone("hand_r")[1] - bone("hand_r")[0]).length
GRIP = 0.55 * HAND_LEN          # the fist's center along hand_r, where the haft passes
HEAD_AT = GRIP + 1.30           # the maul head's center along hand_r
HEAD_SIZE = (0.64, 0.36, 0.36)  # across, and the two sides


# --- Mesh -----------------------------------------------------------------------------------

def _torso():
    return [
        # Hips and the belt.
        ("pelvis", sp((0, 0, 0.88)), sp((0, 0, 1.13)), 8, [(0, .19, .15), (.5, .23, .17), (1, .22, .165)], "cloth", DARK),
        ("pelvis", sp((0, 0, 1.03)), sp((0, 0, 1.14)), 8, [(0, .245, .185), (1, .245, .185)], "cloth", LEATHER),
        ("pelvis", sp((0, 0, 1.055)), sp((0, 0, 1.08)), 8, [(0, .25, .19), (1, .25, .19)], "accent", ACCENT),
        box_part("pelvis", sp((0, -0.19, 1.085)), (0.12, 0.05, 0.10), "metal", BRASS, bevel=0.3),
        # The tabard: long crimson panels front and back, dark hems, a team-colored band.
        box_part("pelvis", sp((0, -0.17, 0.80)), (0.36, 0.035, 0.52), "cloth", CRIMSON, rot=(-6, 0, 0), taper=1.25),
        box_part("pelvis", sp((0, 0.17, 0.80)), (0.36, 0.035, 0.52), "cloth", CRIMSON, rot=(6, 0, 0), taper=1.25),
        box_part("pelvis", sp((0, -0.20, 0.555)), (0.42, 0.04, 0.05), "cloth", CRIMSON_DARK, rot=(-6, 0, 0)),
        box_part("pelvis", sp((0, 0.20, 0.555)), (0.42, 0.04, 0.05), "cloth", CRIMSON_DARK, rot=(6, 0, 0)),
        # The gambeson belly, then the breastplate: a wide beveled block, broader at the top.
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.30)), 8, [(0, .22, .17), (1, .24, .18)], "cloth", GAMBESON),
        box_part("spine_02", sp((0, -0.01, 1.40)), (0.54, 0.40, 0.36), "metal", IRON, taper=1.18, bevel=0.32),
        box_part("chest", sp((0, -0.01, 1.53)), (0.62, 0.44, 0.16), "metal", IRON, bevel=0.32),
        box_part("spine_02", sp((0, 0.17, 1.38)), (0.46, 0.08, 0.34), "metal", IRON_DARK, bevel=0.3),
        # Belt pouches and a brass-studded strap down the back.
        box_part("pelvis", sp((0.17, -0.12, 1.02)), (0.10, 0.08, 0.12), "cloth", LEATHER, rot=(0, 0, 25), bevel=0.25),
        box_part("pelvis", sp((-0.19, -0.08, 1.02)), (0.09, 0.08, 0.11), "cloth", LEATHER, rot=(0, 0, -30), bevel=0.25),
        box_part("pelvis", sp((0.0, 0.20, 1.05)), (0.14, 0.06, 0.10), "cloth", LEATHER, bevel=0.25),
        # A leather baldric across the plate and a brass boss.
        box_part("chest", sp((0, -0.235, 1.40)), (0.09, 0.03, 0.62), "cloth", LEATHER, rot=(0, 38, 0)),
        box_part("chest", sp((0, -0.245, 1.47)), (0.09, 0.03, 0.09), "metal", BRASS, rot=(0, 45, 0)),
        # The gorget: a tall iron collar up to the jaw (the head sits in it).
        ("chest", sp((0, 0.0, 1.56)), sp((0, -0.005, 1.70)), 8, [(0, .20, .17), (.6, .16, .145), (1, .145, .135)], "metal", IRON_DARK),
        ("neck", sp((0, 0, 1.58)), sp((0, -0.01, 1.68)), 8, [(0, .09, .09), (1, .085, .085)], "skin", SKIN),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        # Slab pauldrons: curved plates arched over the shoulder, standing above the gorget, the
        # lowest crimson. The top plate rides the clavicle (it shrugs, never swings into his
        # head); the lower ones ride the upper arm.
        path_part("clavicle_l", [("clavicle_l", 0.55, (0, 0, 0.09)), ("clavicle_l", 1.0, (0.04, 0, 0.13)),
                                 ("clavicle_l", 1.0, (0.17, 0, 0.09)), ("clavicle_l", 1.0, (0.25, 0, -0.03)),
                                 ("clavicle_l", 1.0, (0.27, 0, -0.10))],
                  6, [(.20, .07), (.24, .11), (.25, .12), (.23, .09), (.20, .04)], "metal", CRIMSON, ref=(0, 1, 0)),
        path_part(ua, [(ua, 0.10, (0.12, 0, 0.02)), (ua, 0.20, (0.20, 0, -0.04)), (ua, 0.32, (0.22, 0, -0.12))],
                  6, [(.20, .06), (.21, .06), (.18, .035)], "metal", IRON, ref=(0, 1, 0)),
        box_part("clavicle_l", ("clavicle_l", 1.0, (0.08, 0, 0.24)), (0.06, 0.06, 0.06), "metal", BRASS, rot=(0, 0, 45)),
        (ua, (ua, 0), (ua, 1), 10, [(0, .12, .12), (.5, .115, .11), (1, .10, .10)], "cloth", GAMBESON),
        # Forearms: huge iron vambraces flaring to the gauntlets.
        (fa, (fa, 0), (fa, 1), 10, [(0, .095, .095), (1, .085, .085)], "cloth", LEATHER),
        (fa, (fa, .25), (fa, 1.02), 10, [(0, .10, .10), (.7, .125, .12), (1, .13, .125)], "metal", IRON),
        (fa, (fa, .62), (fa, .70), 10, [(0, .132, .127), (1, .132, .127)], "metal", BRASS),
        # Gauntlets: block fists.
        box_part("hand_l", ("hand_l", 0.5), (0.15, 0.13, 0.17), "metal", IRON_DARK, bevel=0.3),
        box_part("fingers_l", ("fingers_l", 0.4), (0.14, 0.12, 0.10), "metal", IRON_DARK, bevel=0.3),
        box_part("thumb_l", ("thumb_l", 0.5), (0.06, 0.06, 0.07), "metal", IRON_DARK),
        # Legs: short and thick; armored thighs, big knee cops, heavy boots.
        (th, sp((0.10, 0, 1.00)), (th, 1), 10, [(0, .15, .15), (.5, .14, .14), (1, .115, .115)], "cloth", DARK),
        (th, (th, .2), (th, .85), 6, [(0, .135, .15), (1, .12, .13)], "metal", IRON_DARK),
        box_part(ca, (ca, 0.02, (0, -0.06, 0)), (0.17, 0.10, 0.16), "metal", IRON, bevel=0.3),
        (ca, (ca, 0), (ca, .6), 10, [(0, .11, .11), (1, .10, .10)], "cloth", DARK),
        (ca, (ca, .45), (ca, 1), 10, [(0, .13, .13), (.2, .125, .125), (1, .115, .12)], "cloth", LEATHER),
        box_part("foot_l", ("foot_l", 0.45), (0.17, 0.24, 0.12), "cloth", LEATHER, bevel=0.25),
        box_part("toe_l", ("toe_l", 0.4), (0.16, 0.10, 0.08), "metal", IRON_DARK, bevel=0.25),
    ]


def _head(bm, layers, groups, mats):
    # The shared sculpted head, a weathered veteran: square jaw, heavy chin and brow, small hard
    # eyes, shaved, a braided beard, a scar; sized up for the heroic proportions.
    head.build(bm, layers, groups, mats, head.Head(
        base=sp((0, -0.02, 1.66)), height=0.25, scale=1.25, jaw=0.12, chin=1.25, eye=0.8,
        skin=SKIN, hair=BEARD, hair_style="bald", eye_color=(0.12, 0.1, 0.08), lips=(0.56, 0.36, 0.32),
        beard=True, scar=True, metal=IRON_DARK))


def _maul():
    """The two-handed maul, skinned to the right hand and laid along it (the fist closes around
    the haft at GRIP), so a pose aims the whole maul by aiming `hand_r` or by an IK target."""
    h, t = bone("hand_r")
    axis = (t - h).normalized()
    g = h + axis * GRIP
    c = h + axis * HEAD_AT
    across = axis.cross(Vector((0, 1, 0))).normalized()
    side = axis.cross(across).normalized()
    w, a, b = HEAD_SIZE[0] / 2, HEAD_SIZE[1] / 2, HEAD_SIZE[2] / 2

    def p(v):
        return tuple(v)

    hb = "hand_r"
    return [
        # Pommel, haft, a leather grip wrap for each hand, an iron collar under the head.
        path_part(hb, [p(g - axis * 0.30), p(g - axis * 0.22)], 8, [(.06, .06), (.05, .05)], "metal", BRASS, ref=across),
        path_part(hb, [p(g - axis * 0.24), p(c - axis * b)], 6, [(.034, .034), (.04, .04)], "cloth", LEATHER, ref=across),
        path_part(hb, [p(g - axis * 0.14), p(g + axis * 0.48)], 6, [(.042, .042), (.042, .042)], "cloth", DARK, ref=across),
        path_part(hb, [p(c - axis * (b + 0.10)), p(c - axis * b)], 8, [(.06, .06), (.075, .075)], "metal", IRON_DARK, ref=across),
        path_part(hb, [p(g + axis * 0.62), p(g + axis * 0.67)], 8, [(.05, .05), (.05, .05)], "metal", BRASS, ref=across),
        path_part(hb, [p(g + axis * 0.86), p(g + axis * 0.91)], 8, [(.05, .05), (.05, .05)], "metal", BRASS, ref=across),
        # The head: a stone block across the haft, iron bands, brass-capped faces, crimson runes.
        path_part(hb, [p(c - across * w), p(c + across * w)], 4, [(a * 1.414, b * 1.414)] * 2, "cloth", STONE, ref=axis),
        path_part(hb, [p(c - across * (w * 0.62)), p(c - across * (w * 0.48))], 4, [(a * 1.5, b * 1.5)] * 2, "metal", IRON_DARK, ref=axis),
        path_part(hb, [p(c + across * (w * 0.48)), p(c + across * (w * 0.62))], 4, [(a * 1.5, b * 1.5)] * 2, "metal", IRON_DARK, ref=axis),
        path_part(hb, [p(c - across * (w + 0.03)), p(c - across * w)], 4, [(a * 1.2, b * 1.2)] * 2, "metal", BRASS, ref=axis),
        path_part(hb, [p(c + across * w), p(c + across * (w + 0.03))], 4, [(a * 1.2, b * 1.2)] * 2, "metal", BRASS, ref=axis),
        # Runes on the four long faces, so the glow shows from any side and from above.
        path_part(hb, [p(c - across * 0.14 + axis * (a + 0.005)), p(c + across * 0.14 + axis * (a + 0.005))], 4, [(.012, .035)] * 2, "emissive", RUNE, ref=axis),
        path_part(hb, [p(c - across * 0.14 - axis * (a + 0.005)), p(c + across * 0.14 - axis * (a + 0.005))], 4, [(.012, .035)] * 2, "emissive", RUNE, ref=axis),
        path_part(hb, [p(c - across * 0.12 + side * (b + 0.005)), p(c + across * 0.12 + side * (b + 0.005))], 4, [(.035, .012)] * 2, "emissive", RUNE, ref=axis),
        path_part(hb, [p(c - across * 0.12 - side * (b + 0.005)), p(c + across * 0.12 - side * (b + 0.005))], 4, [(.035, .012)] * 2, "emissive", RUNE, ref=axis),
    ]


def parts():
    left = _left()
    tuples = [q for q in left if not callable(q)]
    out = _torso() + left + [mesh._mirror_part(q) for q in tuples]
    for q in left:
        if callable(q) and hasattr(q, "mftr_box"):
            out += box_pair(*q.mftr_box)[1:]
        elif callable(q) and hasattr(q, "mftr_path"):
            out += path_pair(*q.mftr_path)[1:]
    return out + _maul() + [_head]


# --- Pose helpers ---------------------------------------------------------------------------

ANKLE = 0.099


def feet(l=(0.20, 0.06), r=(0.22, -0.10)):
    """Both feet planted at (out, fwd) on the ground."""
    return {"ik_foot_l": ik(at(l[0], l[1], ANKLE, "l")), "ik_foot_r": ik(at(r[0], r[1], ANKLE, "r"))}


def maul(head_pt, toward, left=0.36, pole=(1.0, -0.4, -1.0)):
    """The maul's head center at `head_pt` (armature space), the haft running from it toward
    `toward` (where the hands should be); the right hand on the haft at GRIP and the left hand
    `left` meters further toward the head (negative: toward the pommel; None: off the haft)."""
    hp, tw = Vector(head_pt), Vector(toward)
    axis = (hp - tw).normalized()
    wrist = hp - axis * HEAD_AT
    out = {"ik_hand_r": ik(tuple(wrist), aim=tuple(axis))}
    if left is not None:
        out["grip_l"] = grip("hand_r", at=GRIP + left, pole=pole)
    return out


def spin(pose, deg):
    """The pose turned `deg` degrees about the vertical (spins): aims, IK targets and the
    pelvis turn with it."""
    q = Quaternion(Vector((0, 0, 1)), math.radians(deg))
    out = {}
    for k, v in pose.items():
        if isinstance(v, dict) and "aim" in v and len(v) == 1:
            out[k] = {"aim": tuple(q @ Vector(v["aim"]))}
        elif isinstance(v, dict) and "to" in v:
            out[k] = dict(v, to=tuple(q @ Vector(v["to"])), aim=None if v.get("aim") is None else tuple(q @ Vector(v["aim"])))
        else:
            out[k] = v
    px, py, pz = out.get("pelvis", (0, 0, 0))
    out["pelvis"] = (px, py, pz + deg)
    return out


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# Ground points (armature space) where the maul's head rests or lands.
REST_HEAD = at(0.32, 0.80, 0.19, "r")
SLAM_HEAD = at(0.04, 1.55, 0.19, "r")

# Leaning on the maul: its head on the ground in front of his right foot, his hands stacked on
# the haft at his belly, the weight forward onto it.
LEAN = merge(feet(), maul(REST_HEAD, at(0.05, 0.30, 1.45, "r"), left=-0.17),
             {"spine_01": (7, 0, -4), "spine_02": (4, 0, -4), "chest": (2, 0, 0), "head": (-6, 0, 6)})
# Two-handed guard: the maul diagonal across his body, head up by his left shoulder.
GUARD = merge(feet((0.27, 0.18), (0.27, -0.14)), maul(at(0.55, -0.70, 2.30, "r"), at(0.12, 0.28, 1.20, "r"), left=-0.12, pole=(1.0, 0.1, -0.8)),
              {"spine_01": (12, 0, 12), "spine_02": (4, 0, 8), "head": (-10, 0, -16)})
GUARD_DOWN = {"pelvis": (0, 0, -0.09)}
# On the shoulder (runs): the right hand forward of the shoulder, the head behind; FK aims so
# it rides the body's bob and twist.
CARRY = {"upperarm_r": clips.aim(0.3, 0.35, -0.88, "r"), "forearm_r": clips.aim(-0.3, 0.8, 0.45, "r"),
         "hand_r": clips.aim(-0.15, -0.9, 0.38, "r")}
LEFT_EASY = arm_l(0.32, 0.08, -1, 0.25, 0.3, -1)


def overhead(hands_up=2.06, back=0.05, head_back=-0.9, head_up=2.55):
    """Both hands above his head, the maul pointing up and back."""
    return maul(at(0.12, head_back, head_up, "r"), at(0.10, back, hands_up, "r"), left=0.34, pole=(1.0, 0.2, 0.3))


def slam(head_pt=SLAM_HEAD, hands=at(0.05, 0.42, 0.95, "r")):
    """The head on the ground in front, the hands low over it."""
    return maul(head_pt, hands, left=0.34, pole=(1.0, -0.2, -0.4))


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(LEAN)
    b = merge(LEAN, sym("clavicle", 0, -5, 0), {"chest": (-4, 0, 0), "head": (-9, 0, 6)})
    return _clip(arm, "idle", 90, [(0, a, {"pelvis": (0, 0, -0.02)}), (45, b, {"pelvis": (0, 0, -0.005)})], loop=True)


def idle_fidget_1(arm):
    # Rolls his neck, one way then the other, still leaning on the maul.
    keys = [(0, LEAN, None), (12, merge(LEAN, {"head": (8, 24, 0), "neck": (4, 10, 0)}), None),
            (24, merge(LEAN, {"head": (-12, 0, 0), "neck": (-6, 0, 0)}), None),
            (36, merge(LEAN, {"head": (8, -24, 0), "neck": (4, -10, 0)}), None),
            (46, merge(LEAN, sym("clavicle", 0, -10, 0), {"chest": (-6, 0, 0)}), {"pelvis": (0, 0, 0.02)}), (60, LEAN, None)]
    return _clip(arm, "idle_fidget_1", 60, keys)


def idle_fidget_2(arm):
    # Hauls the maul up onto his shoulder, stretches his back, then drops it head first back
    # onto the ground: a thud, and it stays there (14 §3.2).
    shoulder = merge(feet(), CARRY, LEFT_EASY, {"spine_02": (-4, 0, 0)})
    stretch = merge(feet(), CARRY, arm_l(0.5, -0.1, 0.85, 0.3, -0.2, 1), {"spine_02": (-14, 0, 8), "head": (-16, 0, 10)})
    drop = merge(LEAN, {"spine_01": (12, 0, -4)})
    keys = [(0, LEAN, None), (10, merge(feet(), maul(at(0.3, 0.6, 0.7, "r"), at(0.1, 0.25, 1.15, "r"), left=0.2)), {"pelvis": (0, 0, -0.06)}),
            (18, shoulder, None), (30, stretch, {"pelvis": (0, 0, 0.02)}), (40, stretch, {"pelvis": (0, 0, 0.02)}),
            (48, merge(feet(), maul(at(0.30, 0.55, 1.35, "r"), at(0.1, 0.25, 1.25, "r"), left=0.25)), None, "whip"),
            (51, drop, {"pelvis": (0, 0, -0.06)}), (58, drop, {"pelvis": (0, 0, -0.05)}), (72, LEAN, None)]
    return _clip(arm, "idle_fidget_2", 72, keys)


def idle_ready(arm):
    keys = [(0, GUARD, GUARD_DOWN), (20, merge(GUARD, {"spine_01": (15, 0, 10)}), {"pelvis": (0, 0, -0.11)})]
    return _clip(arm, "idle_ready", 40, keys, loop=True)


def _run_upper(swing):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(CARRY, arm_l(0.3, -0.45 * swing * c, -0.85, 0.2, 0.55 - 0.35 * swing * c, -0.45),
                     {"head": (-12, 0, 0), "neck": (-4, 0, 0)})
    return upper


def run(arm):
    # A heavy run (14 §3.5): long contact, a deep bob, wide feet, the maul braced on the shoulder.
    keys = gait(20, 0.95, contact=0.42, lift=0.20, bob=0.065, lean=13, twist=10, width=0.21, crouch=0.04,
                upper=_run_upper(1.0))
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=340.0)


def run_fast(arm):
    keys = gait(16, 0.91, contact=0.38, lift=0.24, bob=0.07, lean=17, twist=12, width=0.21, crouch=0.05,
                upper=_run_upper(1.3))
    return _clip(arm, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(arm):
    # The overhead slam (14 §3.1): raise, cock (the maul keeps going back as he coils), a
    # one-frame strike, contact on `fire` (sim: 12) with the head in the ground, a held impact,
    # then he hauls it back up.
    rise = merge(feet((0.22, 0.12), (0.24, -0.12)), overhead(), {"spine_02": (-10, 0, 0), "head": (-8, 0, 0)})
    cock = merge(feet((0.22, 0.12), (0.24, -0.12)), overhead(hands_up=2.0, back=-0.06, head_back=-1.05, head_up=1.55),
                 {"spine_01": (-6, 0, 0), "spine_02": (-16, 0, 0), "head": (-4, 0, 0)})
    mid = merge(feet((0.22, 0.20), (0.24, -0.14)), maul(at(0.06, 1.25, 2.0, "r"), at(0.06, 0.25, 1.75, "r"), left=0.34, pole=(1.0, 0.0, 0.0)),
                {"spine_01": (10, 0, 0), "spine_02": (6, 0, 0)})
    hit = merge(feet((0.24, 0.36), (0.26, -0.20)), slam(), {"spine_01": (24, 0, 0), "spine_02": (12, 0, 0), "head": (-14, 0, 0)})
    keys = [(0, GUARD, GUARD_DOWN), (5, rise, {"pelvis": (0, 0.02, 0.02)}), (9, cock, {"pelvis": (0, 0.03, -0.04)}, "snap"),
            (11, mid, {"pelvis": (0, -0.04, -0.12)}, "snap"), (12, hit, {"pelvis": (0, -0.08, -0.24)}),
            (18, merge(hit, {"spine_01": (27, 0, 0)}), {"pelvis": (0, -0.08, -0.26)}, "settle"),
            (24, merge(feet((0.24, 0.30), (0.26, -0.18)), slam(at(0.10, 1.25, 0.45, "r"), at(0.12, 0.35, 1.0, "r")), {"spine_01": (14, 0, 0)}),
             {"pelvis": (0, -0.04, -0.16)}),
            (28, merge(feet((0.27, 0.22), (0.27, -0.15)), maul(at(0.40, 0.10, 2.40, "r"), at(0.14, 0.30, 1.20, "r"), left=-0.10, pole=(1.0, 0.1, -0.8)),
                       {"spine_01": (12, 0, 8)}), {"pelvis": (0, 0, -0.10)}),
            (32, GUARD, GUARD_DOWN)]
    return _clip(arm, "attack_1", 32, keys, markers={"fire": 12})


def attack_2(arm):
    # The sidelong sweep: wound back to the right and low, cocked further, swept across the
    # front on `fire` (sim: 12), the follow-through held out to his left.
    st = feet((0.26, 0.18), (0.26, -0.16))
    wind = merge(st, maul(at(0.95, -0.55, 0.75, "r"), at(0.30, 0.15, 1.05, "r"), left=0.36),
                 {"pelvis": (0, 0, -18), "spine_01": (8, 0, -22), "spine_02": (2, 0, -14), "head": (-6, 0, 18)})
    cock = merge(st, maul(at(0.75, -0.95, 0.8, "r"), at(0.32, 0.0, 1.05, "r"), left=0.36),
                 {"pelvis": (0, 0, -24), "spine_01": (8, 0, -30), "spine_02": (2, 0, -18), "head": (-6, 0, 26)})
    mid = merge(st, maul(at(0.1, 1.6, 0.75, "r"), at(0.1, 0.45, 1.0, "r"), left=0.36), {"spine_01": (10, 0, 4)})
    hit = merge(st, maul(at(1.15, 1.05, 0.72, "l"), at(0.2, 0.45, 1.0, "l"), left=0.36, pole=(1.0, 0.2, -0.6)),
                {"pelvis": (0, 0, 22), "spine_01": (10, 0, 28), "spine_02": (4, 0, 16), "head": (-6, 0, -20)})
    keys = [(0, GUARD, GUARD_DOWN), (7, wind, {"pelvis": (0, 0, -0.10)}), (10, cock, {"pelvis": (0, 0, -0.11)}, "snap"),
            (11, mid, {"pelvis": (0, -0.03, -0.12)}, "snap"), (12, hit, {"pelvis": (0, -0.04, -0.13)}),
            (18, merge(hit, {"spine_01": (11, 0, 34)}), {"pelvis": (0, -0.04, -0.13)}, "settle"), (30, GUARD, GUARD_DOWN)]
    return _clip(arm, "attack_2", 30, keys, markers={"fire": 12})


# Cleave: the maul held out level in both hands, pointing ahead and to his right.
OUT = merge(feet((0.28, 0.10), (0.28, -0.10)), maul(at(0.55, 1.62, 0.85, "r"), at(0.18, 0.38, 1.02, "r"), left=0.40, pole=(1.0, 0.1, -0.6)),
            {"spine_01": (14, 0, 0), "spine_02": (4, 0, 0), "head": (-12, 0, 0)})


def q(arm):
    # Cleave (nova, `fire` 6): wound to the right, one full turn with the maul out, the hit at
    # mid-sweep; the head drags down to the ground at the end and stays a beat.
    down = {"pelvis": (0, 0, -0.14)}
    end = merge(OUT, slam(at(0.5, 1.4, 0.19, "r"), at(0.12, 0.4, 0.95, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (3, spin(OUT, -80), down, "whip"), (5, spin(OUT, 10), down, "snap"),
            (6, spin(OUT, 75), down, "snap"), (8, spin(OUT, 165), down, "snap"), (10, spin(OUT, 255), down),
            (13, spin(merge(OUT, slam(at(0.55, 1.45, 0.3, "r"), at(0.15, 0.4, 1.0, "r"))), 330), {"pelvis": (0, 0, -0.18)}, "settle"),
            (15, spin(end, 360), {"pelvis": (0, 0, -0.2)}), (20, spin(end, 360), {"pelvis": (0, 0, -0.19)}),
            (28, spin(GUARD, 360), GUARD_DOWN)]
    return _clip(arm, "q", 28, keys, markers={"fire": 6})


def w(arm):
    # Second Wind (upper body, a pulse: instant and mobile): thumps his chest with his free fist
    # and heaves a deep breath, the maul on his shoulder.
    thump = merge(CARRY, arm_l(0.2, 0.6, -0.3, -0.8, 0.4, 0.3), {"spine_02": (-4, 0, 0)})
    breath = merge(CARRY, LEFT_EASY, sym("clavicle", 0, -10, 0), {"chest": (-12, 0, 0), "head": (-16, 0, 0)})
    keys = [(0, merge(CARRY, LEFT_EASY), None), (5, merge(CARRY, arm_l(0.4, 0.3, 0.1, 0.0, 0.5, 0.8)), None, "snap"),
            (8, thump, None), (11, merge(thump, {"chest": (4, 0, 0)}), None),
            (18, breath, None), (26, merge(CARRY, LEFT_EASY), None)]
    return _clip(arm, "w", 26, keys, layer="upper", markers={"fire": 8})


def e_start(arm):
    # Lunge: a deep coil, then the launch, the maul swinging up overhead.
    coil = merge(feet((0.26, 0.12), (0.26, -0.20)), slam(at(0.6, -0.2, 0.35, "r"), at(0.2, 0.25, 0.9, "r")),
                 {"spine_01": (30, 0, -10), "head": (-20, 0, 0)})
    launch = merge(overhead(), {"spine_01": (14, 0, 0), "thigh_l": (10, 0, 0), "thigh_r": (14, 0, 0),
                                "calf_l": (8, 0, 0), "calf_r": (12, 0, 0), "foot_l": (24, 0, 0), "foot_r": (24, 0, 0)})
    return _clip(arm, "e_start", 6, [(0, coil, {"pelvis": (0, 0, -0.26)}, "snap"), (6, launch, {"pelvis": (0, 0, 0.06)})], markers={"fire": 3})


def e_travel(arm):
    # Airborne, knees tucked, the maul high behind his head.
    air = merge(overhead(hands_up=2.0, back=-0.02, head_back=-1.0, head_up=2.1),
                {"thigh_l": (-60, -6, 0), "thigh_r": (-40, 6, 0), "calf_l": (85, 0, 0), "calf_r": (70, 0, 0),
                 "foot_l": (12, 0, 0), "foot_r": (8, 0, 0), "spine_01": (8, 0, 0), "spine_02": (-8, 0, 0)})
    keys = [(0, air, {"pelvis": (0, 0, 0.10)}), (6, merge(air, {"spine_02": (-12, 0, 0)}), {"pelvis": (0, 0, 0.14)})]
    return _clip(arm, "e_travel", 12, keys, loop=True)


def e_land(arm):
    # Maul first: the head hits the ground with him, he stays down a beat (14 §3.2), then rises
    # into his guard.
    land = merge(feet((0.28, 0.30), (0.28, -0.24)), slam(), {"spine_01": (28, 0, 0), "spine_02": (12, 0, 0), "head": (-16, 0, 0)})
    keys = [(0, land, {"pelvis": (0, -0.08, -0.36)}, "settle"), (5, merge(land, {"spine_01": (30, 0, 0)}), {"pelvis": (0, -0.08, -0.34)}),
            (12, GUARD, GUARD_DOWN)]
    return _clip(arm, "e_land", 12, keys)


def r(arm):
    # Shockwave: up on his toes with the maul high, a held beat, cocked further, then a
    # ground-shaking slam on `fire` (sim: 13.5). The head stays in the ground through the hard
    # lock and a long hold while he heaves for breath, then he hauls it out.
    st = feet((0.26, 0.16), (0.26, -0.16))
    high = merge(st, overhead(hands_up=2.12, back=0.04, head_back=-0.55, head_up=2.85), {"spine_02": (-14, 0, 0), "head": (-14, 0, 0)})
    cock = merge(st, overhead(hands_up=2.08, back=-0.08, head_back=-1.15, head_up=2.0), {"spine_01": (-6, 0, 0), "spine_02": (-20, 0, 0), "head": (-8, 0, 0)})
    hit = merge(feet((0.30, 0.38), (0.30, -0.22)), slam(at(0.0, 1.75, 0.19, "r"), at(0.0, 0.55, 0.85, "r")),
                {"spine_01": (32, 0, 0), "spine_02": (14, 0, 0), "head": (-18, 0, 0)})
    keys = [(0, GUARD, GUARD_DOWN), (6, high, {"pelvis": (0, 0.03, 0.05)}), (9, merge(high, {"spine_02": (-17, 0, 0)}), {"pelvis": (0, 0.03, 0.06)}),
            (12, cock, {"pelvis": (0, 0.04, -0.02)}, "snap"),
            (13, merge(st, maul(at(0.0, 1.4, 2.0, "r"), at(0.0, 0.3, 1.7, "r"), left=0.34, pole=(1.0, 0.0, 0.0)), {"spine_01": (12, 0, 0)}),
             {"pelvis": (0, -0.04, -0.16)}, "snap"),
            (14, hit, {"pelvis": (0, -0.10, -0.34)}), (21, merge(hit, {"spine_01": (35, 0, 0)}), {"pelvis": (0, -0.10, -0.36)}),
            (28, merge(hit, {"spine_01": (30, 0, 0), "chest": (-6, 0, 0), "head": (-24, 0, 0)}), {"pelvis": (0, -0.09, -0.33)}, "settle"),
            (34, merge(feet((0.26, 0.2), (0.26, -0.18)), slam(at(0.05, 1.4, 0.55, "r"), at(0.1, 0.35, 1.05, "r")), {"spine_01": (14, 0, 0)}),
             {"pelvis": (0, -0.04, -0.17)}),
            (42, GUARD, GUARD_DOWN)]
    return _clip(arm, "r", 42, keys, markers={"fire": 14})


# Recall: kneeling, the maul upright and head down in front of him, both hands on the pommel.
KNEEL = merge({"ik_foot_l": ik(at(0.20, 0.32, ANKLE, "l")), "thigh_r": (8, 0, 0), "calf_r": (100, 0, 0), "foot_r": (40, 0, 0)},
              maul(at(0.05, 0.62, 0.19, "r"), at(0.05, 0.62, 1.2, "r"), left=-0.16, pole=(1.0, -0.3, -0.5)),
              {"spine_01": (10, 0, 0), "head": (16, 0, 0)})


def recall(arm):
    breathe = merge(KNEEL, sym("clavicle", 0, -5, 0), {"spine_01": (6, 0, 0), "head": (12, 0, 0)})
    keys = [(0, LEAN, None), (15, KNEEL, {"pelvis": (0, 0.06, -0.50)}), (45, breathe, {"pelvis": (0, 0.06, -0.49)}),
            (75, KNEEL, {"pelvis": (0, 0.06, -0.50)}), (90, LEAN, None)]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    # A blow staggers him back, he drops to his knees with the maul's head hitting the ground
    # beside him, then pitches forward onto his face.
    stagger = merge(feet((0.22, -0.05), (0.22, -0.30)), maul(at(0.55, 0.3, 1.6, "r"), at(0.2, 0.1, 1.15, "r"), left=None),
                    arm_l(0.5, 0.2, 0.0, 0.5, 0.4, 0.2), {"spine_01": (-14, 0, 0), "head": (-26, 0, 0)})
    knees = merge({"thigh_l": (0, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (100, 0, 0), "calf_r": (100, 0, 0), "foot_l": (40, 0, 0), "foot_r": (40, 0, 0)},
                  maul(at(0.7, 0.5, 0.19, "r"), at(0.3, 0.2, 0.75, "r"), left=None), arm_l(0.25, 0.4, -0.9, 0.2, 0.5, -0.8),
                  {"spine_01": (20, 0, 0), "head": (22, 0, 0)})
    fall = merge({"thigh_l": (-12, 0, 0), "thigh_r": (-8, 0, 0), "calf_l": (24, 0, 0), "calf_r": (18, 0, 0)},
                 {"upperarm_r": clips.aim(0.7, 0.6, 0.1, "r"), "forearm_r": clips.aim(0.8, 0.5, 0.0, "r"), "hand_r": clips.aim(0.9, 0.3, 0.0, "r")},
                 arm_l(0.6, 0.8, 0.1, 0.5, 0.9, 0.0), {"pelvis": (84, 0, 0), "head": (-30, 0, 10)})
    keys = [(0, LEAN, None), (6, stagger, {"pelvis": (0, 0.08, 0)}, "settle"), (15, knees, {"pelvis": (0, 0, -0.52)}, "snap"),
            (19, merge(knees, {"spine_01": (28, 0, 0)}), {"pelvis": (0, 0, -0.53)}), (27, fall, {"pelvis": (0, -0.62, -0.82)}, "snap"),
            (30, merge(fall, {"head": (-22, 0, 10)}), {"pelvis": (0, -0.62, -0.84)}), (36, fall, {"pelvis": (0, -0.62, -0.84)})]
    return _clip(arm, "death", 36, keys)


def respawn(arm):
    low = merge(feet((0.24, 0.12), (0.24, -0.16)), slam(REST_HEAD, at(0.08, 0.32, 0.8, "r")), {"spine_01": (26, 0, 0), "head": (-10, 0, 0)})
    keys = [(0, low, {"pelvis": (0, 0, -0.34)}, "settle"), (12, merge(LEAN, sym("clavicle", 0, -6, 0)), {"pelvis": (0, 0, -0.04)}),
            (26, LEAN, None)]
    return _clip(arm, "respawn", 26, keys)


def select(arm):
    # Hoists the maul overhead in both hands and roars, then brings it down head first: it
    # lands with a thud and stays.
    up = merge(feet((0.24, 0.1), (0.24, -0.12)), overhead(hands_up=2.2, back=0.05, head_back=0.2, head_up=3.0),
               {"spine_02": (-10, 0, 0), "head": (-26, 0, 0)})
    keys = [(0, LEAN, None), (12, up, {"pelvis": (0, 0, 0.03)}), (26, merge(up, {"head": (-34, 0, 0)}), {"pelvis": (0, 0, 0.03)}, "whip"),
            (31, merge(LEAN, {"spine_01": (14, 0, -4)}), {"pelvis": (0, 0, -0.08)}), (40, merge(LEAN, {"spine_01": (12, 0, -4)}), {"pelvis": (0, 0, -0.07)}),
            (52, merge(LEAN, {"head": (-8, 0, -10)}), None)]
    return _clip(arm, "select", 52, keys)


ONE_HAND = merge(feet(), maul(REST_HEAD, at(0.05, 0.30, 1.45, "r"), left=None), {"spine_01": (7, 0, -4), "head": (-8, 0, 6)})


def emote_taunt(arm):
    # Leaning on the maul, he beckons with his free hand.
    out = merge(ONE_HAND, arm_l(0.2, 1, 0.0, 0.1, 1, 0.1))
    curl = merge(ONE_HAND, arm_l(0.2, 1, 0.0, 0.1, 0.4, 0.9))
    keys = [(0, LEAN, None), (8, out, None), (13, curl, None, "snap"), (18, out, None), (23, curl, None, "snap"), (28, out, None), (40, LEAN, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Flexes his free arm, twice, and admires it.
    flex = merge(ONE_HAND, arm_l(1, 0.1, 0.05, 0.05, 0.1, 1), {"head": (4, 0, 25)})
    keys = [(0, LEAN, None), (10, flex, None), (15, merge(flex, sym("clavicle", 0, -8, 0)), None, "snap"), (20, flex, None),
            (25, merge(flex, sym("clavicle", 0, -8, 0)), None, "snap"), (38, merge(flex, {"head": (10, 0, 35)}), None), (50, LEAN, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    belly = arm_l(0.15, 0.3, -0.9, -0.7, 0.6, 0.2)
    keys = [(0, LEAN, None)]
    for i in range(1, 9):
        s = 1.0 if i % 2 else 0.5
        keys.append((4 * i, merge(ONE_HAND, belly, {"spine_01": (-8 * s, 0, 0), "head": (-22 * s, 0, 0)}, sym("clavicle", 0, -4 * s, 0)),
                     {"pelvis": (0, 0, -0.01 * s)}))
    keys.append((44, LEAN, None))
    return _clip(arm, "emote_laugh", 44, keys)


def emote_dance(arm):
    # A heavy stomping two-step with the maul held high: each stomp lands hard.
    up = overhead(hands_up=2.15, back=0.05, head_back=0.0, head_up=2.9)
    a = merge(up, {"ik_foot_l": ik(at(0.24, 0.1, 0.35, "l")), "ik_foot_r": ik(at(0.24, -0.05, ANKLE, "r")), "spine_01": (0, -6, 0)})
    land = merge(up, feet((0.26, 0.08), (0.24, -0.05)), {"spine_01": (6, 0, 0)})
    b = merge(up, {"ik_foot_l": ik(at(0.26, 0.08, ANKLE, "l")), "ik_foot_r": ik(at(0.24, -0.05, 0.35, "r")), "spine_01": (0, 6, 0)})
    keys = [(0, a, {"pelvis": (0.04, 0, 0.02)}, "whip"), (10, land, {"pelvis": (0, 0, -0.08)}), (24, b, {"pelvis": (-0.04, 0, 0.02)}, "whip"),
            (34, land, {"pelvis": (0, 0, -0.08)})]
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


def build(only=None):
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig.use_shape(SHAPE)
    mesh.AO_HEIGHT = 2.0
    arm = rig.build_armature(name="rook")
    own = [f(arm).name for f in OWN if only is None or f.__name__ in only]
    body = mesh.build_mannequin(arm, name="rook", part_list=parts())
    shared = bake_shared(arm) if only is None else []
    print(f"rook: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(own)} own clips, shared {shared}")
    return arm, body


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    build(only)
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
