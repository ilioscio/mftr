# SPDX-License-Identifier: AGPL-3.0-or-later
"""Ember (rebuilt for 14): the generator for his model and clips.

    blender -b --python art/champions/ember/build.py -- --out art/champions/ember/ember.blend

A flame (14 §5): a cocky pyromancer, top-heavy and slim. Burning hair sweeps up and back from his
head, framed by a tall flared collar of soot-black petals lined with ember red; bell sleeves
flare wide at the wrists; fire burns in both palms; a long charcoal coat trails on an
`extra_coat` chain. His signature is flame orange (14 §4).

Weight (14 §3): a light, springing run; every cast has a coil, a cock and a one-frame thrust on
`fire` with a recoil that holds (the lance pushes him back). Feet planted by IK.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

from mftr_blender import champion, clips, head, rig  # noqa: E402
from mftr_blender.champion import arm, feet, hand  # noqa: E402
from mftr_blender.clips import at, merge, sym  # noqa: E402
from mftr_blender.library import _clip, gait  # noqa: E402
from mftr_blender.mesh import box_part, path_part  # noqa: E402

SKIN = (0.76, 0.54, 0.40)
CHARCOAL = (0.11, 0.10, 0.10)
SOOT = (0.05, 0.045, 0.045)
EMBER_RED = (0.55, 0.10, 0.04)
GOLD = (0.60, 0.40, 0.12)
FLAME = (1.0, 0.50, 0.10)
FLAME_HOT = (1.0, 0.82, 0.30)
ACCENT = (0.18, 0.52, 0.95)

SHAPE = {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 1.0, "y": 0.96, "scale": 0.97, "shoulder": 0.04, "arm": 1.04, "hand": 1.3}
rig.use_shape(SHAPE)
COAT = [rig.shaped(p) for p in [(0, 0.12, 1.06), (0, 0.16, 0.80), (0, 0.19, 0.52), (0, 0.21, 0.26)]]


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", sp((0, 0, 0.92)), sp((0, 0, 1.12)), 10, [(0, .125, .095), (.5, .14, .10), (1, .135, .098)], "cloth", SOOT),
        ("pelvis", sp((0, 0, 1.04)), sp((0, 0, 1.10)), 10, [(0, .145, .105), (1, .14, .10)], "accent", ACCENT),
        box_part("pelvis", sp((0, -0.115, 1.07)), (0.07, 0.03, 0.06), "metal", GOLD, bevel=0.3),
        # A slim waist under a broad chest: the coat's body, gold-buttoned, an ember-red lining.
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 10, [(0, .118, .088), (1, .13, .094)], "cloth", CHARCOAL),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 10, [(0, .135, .098), (1, .17, .115)], "cloth", CHARCOAL),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.60)), 10, [(0, .175, .118), (.5, .185, .122), (1, .10, .085)], "cloth", CHARCOAL),
        box_part("chest", sp((0, -0.12, 1.40)), (0.05, 0.03, 0.30), "cloth", EMBER_RED),
        # Lapels opening over the ember-red lining.
        box_part("chest", sp((0.07, -0.12, 1.48)), (0.06, 0.025, 0.20), "cloth", SOOT, rot=(0, -20, 0), bevel=0.3),
        box_part("chest", sp((-0.07, -0.12, 1.48)), (0.06, 0.025, 0.20), "cloth", SOOT, rot=(0, 20, 0), bevel=0.3),
        box_part("chest", sp((0, -0.135, 1.48)), (0.03, 0.02, 0.03), "metal", GOLD),
        box_part("chest", sp((0, -0.135, 1.38)), (0.03, 0.02, 0.03), "metal", GOLD),
        box_part("chest", sp((0, -0.10, 1.585)), (0.07, 0.03, 0.06), "metal", GOLD, rot=(0, 45, 0), bevel=0.3),
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .05, .05), (1, .046, .046)], "skin", SKIN),
        # The flared collar: soot-black petals rising behind his head, lined ember red.
        path_part("chest", [sp((0.0, 0.08, 1.56)), sp((0.0, 0.16, 1.80)), sp((0.0, 0.22, 2.02))], 4, [(.16, .03), (.20, .03), (0, 0)], "cloth", SOOT, ref=(1, 0, 0)),
        path_part("chest", [sp((0.0, 0.07, 1.57)), sp((0.0, 0.145, 1.79)), sp((0.0, 0.20, 1.98))], 4, [(.13, .02), (.16, .02), (0, 0)], "cloth", EMBER_RED, ref=(1, 0, 0)),
        path_part("chest", [sp((0.10, 0.04, 1.56)), sp((0.22, 0.10, 1.76)), sp((0.30, 0.15, 1.92))], 4, [(.10, .025), (.12, .025), (0, 0)], "cloth", SOOT, ref=(0, 1, 0)),
        path_part("chest", [sp((-0.10, 0.04, 1.56)), sp((-0.22, 0.10, 1.76)), sp((-0.30, 0.15, 1.92))], 4, [(.10, .025), (.12, .025), (0, 0)], "cloth", SOOT, ref=(0, 1, 0)),
        path_part("chest", [sp((0.09, 0.035, 1.565)), sp((0.20, 0.09, 1.75)), sp((0.27, 0.135, 1.88))], 4, [(.08, .018), (.095, .018), (0, 0)], "cloth", EMBER_RED, ref=(0, 1, 0)),
        path_part("chest", [sp((-0.09, 0.035, 1.565)), sp((-0.20, 0.09, 1.75)), sp((-0.27, 0.135, 1.88))], 4, [(.08, .018), (.095, .018), (0, 0)], "cloth", EMBER_RED, ref=(0, 1, 0)),
        # Shoulder caps with gold trim.
        path_part("clavicle_l", [("clavicle_l", 0.55, (0, 0, 0.05)), ("clavicle_l", 1.0, (0.03, 0, 0.08)), ("clavicle_l", 1.0, (0.12, 0, 0.03)),
                                 ("clavicle_l", 1.0, (0.14, 0, -0.05))], 6, [(.10, .04), (.12, .055), (.12, .055), (.10, .03)], "cloth", CHARCOAL, ref=(0, 1, 0)),
        path_part("clavicle_r", [("clavicle_r", 0.55, (0, 0, 0.05)), ("clavicle_r", 1.0, (-0.03, 0, 0.08)), ("clavicle_r", 1.0, (-0.12, 0, 0.03)),
                                 ("clavicle_r", 1.0, (-0.14, 0, -0.05))], 6, [(.10, .04), (.12, .055), (.12, .055), (.10, .03)], "cloth", CHARCOAL, ref=(0, 1, 0)),
        # Coat tails on the chain, ember-red lined, a gold hem.
        ("extra_coat_1", COAT[0], COAT[1], 6, [(0, .16, .026), (1, .19, .026)], "cloth", CHARCOAL),
        ("extra_coat_2", COAT[1], COAT[2], 6, [(0, .19, .026), (1, .21, .026)], "cloth", CHARCOAL),
        ("extra_coat_3", COAT[2], COAT[3], 6, [(0, .21, .026), (.8, .20, .024), (1, .14, .02)], "cloth", EMBER_RED),
        ("extra_coat_3", (COAT[3][0], COAT[3][1] + 0.005, COAT[3][2] + 0.04), (COAT[3][0], COAT[3][1] + 0.006, COAT[3][2] - 0.005), 6,
         [(0, .17, .03), (1, .14, .028)], "metal", GOLD),
    ]


def _head(bm, layers, groups, mats):
    head.build(bm, layers, groups, mats, head.Head(
        base=sp((0, -0.008, 1.63)), height=0.25, scale=1.1, jaw=0.4, chin=1.05, eye=1.1,
        skin=SKIN, hair=EMBER_RED, hair_style="short", eye_color=(0.9, 0.5, 0.1), lips=(0.60, 0.34, 0.30)))


def _hair():
    # Burning hair: flames sweeping up and back from his crown, hottest at the roots.
    hb = "head"
    flames = [((0.0, -0.06, 1.84), (0.0, 0.06, 2.10), (0.0, 0.16, 2.24), .07),
              ((0.07, -0.02, 1.83), (0.12, 0.10, 2.04), (0.15, 0.20, 2.14), .055),
              ((-0.07, -0.02, 1.83), (-0.12, 0.10, 2.04), (-0.15, 0.20, 2.14), .055),
              ((0.04, 0.06, 1.82), (0.06, 0.18, 1.98), (0.07, 0.30, 2.04), .05),
              ((-0.04, 0.06, 1.82), (-0.06, 0.18, 1.98), (-0.07, 0.30, 2.04), .05),
              ((0.10, 0.02, 1.80), (0.17, 0.12, 1.92), (0.21, 0.24, 1.96), .04),
              ((-0.10, 0.02, 1.80), (-0.17, 0.12, 1.92), (-0.21, 0.24, 1.96), .04),
              ((0.0, 0.08, 1.80), (0.0, 0.22, 1.90), (0.0, 0.36, 1.92), .05)]
    out = []
    for a, b, c, r in flames:
        out.append(path_part(hb, [sp(a), sp(b), sp(c)], 5, [(r, r), (r * 0.6, r * 0.6), (0, 0)], "emissive", FLAME,
                             colors=[FLAME_HOT, FLAME, FLAME]))
    return out


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 10, [(0, .06, .06), (1, .052, .052)], "cloth", CHARCOAL),
        # Bell sleeves, flaring wide at the wrist, lined ember red, gold-hemmed.
        (fa, (fa, 0), (fa, 1.0), 10, [(0, .058, .058), (.5, .08, .076), (1, .12, .105)], "cloth", CHARCOAL),
        (fa, (fa, 0.96), (fa, 1.02), 10, [(0, .124, .109), (1, .124, .109)], "cloth", EMBER_RED),
        (fa, (fa, 0.75), (fa, 1.0), 10, [(0, .066, .064), (1, .10, .09)], "cloth", EMBER_RED),
        box_part("hand_l", ("hand_l", 0.45), (0.08, 0.06, 0.09), "skin", SKIN, bevel=0.3),
        box_part("fingers_l", ("fingers_l", 0.4), (0.075, 0.05, 0.06), "skin", SKIN, bevel=0.3),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .02, .02), (1, .015, .015)], "skin", SKIN),
        # Fire in the palm.
        path_part("hand_l", [("hand_l", 0.6, (0, -0.06, 0)), ("hand_l", 0.6, (0, -0.10, 0.08)), ("hand_l", 0.6, (0, -0.12, 0.18))], 5,
                  [(.055, .055), (.035, .035), (0, 0)], "emissive", FLAME, colors=[FLAME_HOT, FLAME, FLAME]),
        (th, sp((0.10, 0, 1.02)), (th, 1), 10, [(0, .088, .085), (.5, .074, .074), (1, .058, .058)], "cloth", SOOT),
        (ca, (ca, 0), (ca, .5), 10, [(0, .058, .058), (1, .052, .052)], "cloth", SOOT),
        (ca, (ca, .42), (ca, 1), 10, [(0, .07, .07), (.15, .066, .066), (1, .056, .058)], "cloth", CHARCOAL),
        box_part("foot_l", ("foot_l", 0.45), (0.09, 0.16, 0.08), "cloth", CHARCOAL, bevel=0.25),
        (ca, (ca, .40), (ca, .48), 10, [(0, .078, .078), (1, .078, .078)], "cloth", EMBER_RED),
        box_part(ca, (ca, 0.02, (0, -0.05, 0)), (0.08, 0.04, 0.09), "metal", GOLD, bevel=0.3),
        path_part("toe_l", [("toe_l", 0.0), ("toe_l", 0.9), ("toe_l", 1.5, (0, 0, 0.04))], 4, [(.045, .03), (.03, .02), (0, 0)], "cloth", CHARCOAL),
    ]


def parts():
    return _center() + champion.mirrored(_left()) + _hair() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def coat(back=0.0, sway=0.0):
    return {"extra_coat_1": (back * 0.45, sway * 0.5, 0), "extra_coat_2": (back * 0.3, sway * 0.3, 0),
            "extra_coat_3": (back * 0.25, sway * 0.25, 0)}


STAND = feet((0.11, 0.06), (0.15, -0.08))
# Cocky: weight on his right leg, the left hand on his hip, the right palm up with its flame.
IDLE = merge(STAND, {"pelvis": (0, -4, 0), "spine_01": (0, 3, 0), "head": (-4, 0, -6)}, arm("l", 0.5, -0.1, -0.6, -0.4, 0.4, 0.3),
             arm("r", 0.35, 0.3, -0.85, 0.1, 0.9, 0.25), {"hand_r": clips.aim(0.0, 0.6, 0.1, "r")}, coat(2))
READY = merge(feet((0.16, 0.16), (0.16, -0.14)), {"spine_01": (8, 0, 0), "head": (-6, 0, 0)}, arm("l", 0.4, 0.5, -0.5, 0.1, 0.9, 0.0),
              arm("r", 0.4, 0.5, -0.5, 0.1, 0.9, 0.0), coat(4))
DOWN = {"pelvis": (0, 0, -0.05)}


def idle(armature):
    b = merge(IDLE, sym("clavicle", 0, -3, 0), {"head": (-6, 0, -2)}, coat(3, 2))
    return _clip(armature, "idle", 64, [(0, IDLE, {"pelvis": (0.01, 0, -0.01)}), (32, b, {"pelvis": (0.01, 0, 0.0)})], loop=True)


def idle_fidget_1(armature):
    # Tosses his palm's flame up and catches it.
    up = merge(IDLE, arm("r", 0.35, 0.5, -0.2, 0.1, 0.5, 0.85), {"head": (-18, 0, -4)})
    keys = [(0, IDLE, None), (8, merge(IDLE, arm("r", 0.35, 0.3, -0.9, 0.1, 0.8, 0.0)), None), (11, up, None, "snap"), (22, merge(up, {"head": (-24, 0, -4)}), None),
            (26, merge(IDLE, arm("r", 0.35, 0.3, -0.9, 0.1, 0.8, 0.0)), None, "snap"), (36, IDLE, None)]
    return _clip(armature, "idle_fidget_1", 36, keys)


def idle_fidget_2(armature):
    # Pats out a spark on his sleeve, irritated, then smooths his coat.
    pat = merge(IDLE, arm("r", 0.3, 0.6, -0.3, -0.7, 0.4, 0.2), arm("l", 0.5, 0.4, -0.4, 0.3, 0.6, 0.1), {"head": (10, 0, 14)})
    keys = [(0, IDLE, None), (10, pat, None), (13, merge(pat, arm("r", 0.3, 0.6, -0.2, -0.7, 0.4, 0.35)), None, "snap"), (16, pat, None, "snap"),
            (19, merge(pat, arm("r", 0.3, 0.6, -0.2, -0.7, 0.4, 0.35)), None, "snap"), (30, merge(IDLE, {"spine_02": (-4, 0, 0)}), None), (44, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 44, keys)


def idle_ready(armature):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (10, 0, 0)}, coat(6, 2)), {"pelvis": (0, 0, -0.07)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(swing, trail):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(arm("l", 0.35, -0.45 * swing * c, -0.75, 0.2, 0.55 - 0.3 * swing * c, -0.25),
                     arm("r", 0.35, 0.45 * swing * c, -0.75, 0.2, 0.55 + 0.3 * swing * c, -0.25),
                     coat(trail + 6 * math.sin(4 * math.pi * p), 4 * math.sin(2 * math.pi * p)), {"head": (-8, 0, 0)})
    return upper


def run(armature):
    keys = gait(20, 0.79, contact=0.36, lift=0.26, bob=0.05, lean=10, twist=10, width=0.11, upper=_run_upper(1.0, 32))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    keys = gait(16, 0.84, contact=0.35, lift=0.3, bob=0.055, lean=15, twist=12, width=0.11, upper=_run_upper(1.3, 44))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _bolt(armature, name, side, fire=9, frames=24):
    """A fire bolt flung from one palm: coil it back by the ear, cock, a one-frame thrust on
    `fire`, the arm held out, then back to ready."""
    other = "l" if side == "r" else "r"
    turn = 1 if side == "r" else -1
    st = feet((0.16, 0.20), (0.16, -0.18))
    coil = merge(st, {"spine_01": (2, 0, -20 * turn), "spine_02": (-2, 0, -12 * turn), "head": (-6, 0, 18 * turn)},
                 hand(side, at(0.25, -0.12, 1.62, side), aim=at(0.0, 0.5, 0.6, side)), arm(other, 0.4, 0.6, -0.3, 0.2, 0.9, 0.1), coat(4))
    cock = merge(coil, {"spine_01": (0, 0, -26 * turn)}, hand(side, at(0.28, -0.22, 1.60, side), aim=at(0.1, 0.3, 0.8, side)))
    thrust = merge(feet((0.16, 0.30), (0.16, -0.18)), {"spine_01": (10, 0, 16 * turn), "spine_02": (4, 0, 10 * turn), "head": (-8, 0, -12 * turn)},
                   arm(side, 0.12, 1, 0.08, 0.05, 1, 0.12), arm(other, 0.5, -0.4, -0.5, 0.3, -0.2, -0.4), coat(8, -3 * turn))
    keys = [(0, READY, DOWN), (fire - 4, coil, DOWN), (fire - 1, cock, DOWN, "snap"), (fire, thrust, {"pelvis": (0, -0.04, -0.06)}),
            (fire + 5, merge(thrust, {"spine_01": (12, 0, 18 * turn)}), {"pelvis": (0, -0.04, -0.07)}, "settle"), (frames, READY, DOWN)]
    return _clip(armature, name, frames, keys, markers={"fire": fire})


def attack_1(armature):
    return _bolt(armature, "attack_1", "r")


def attack_2(armature):
    return _bolt(armature, "attack_2", "l")


def q(armature):
    # Ember Lance: both palms drawn back to his right hip, the fire gathered, cocked, then thrust
    # forward together on `fire` 8; the recoil rocks him back a step and holds.
    st = feet((0.16, 0.24), (0.18, -0.20))
    gather = merge(st, {"pelvis": (0, 0, -16), "spine_01": (6, 0, -26), "spine_02": (0, 0, -12), "head": (-6, 0, 24)},
                   hand("r", at(0.32, -0.10, 1.05, "r"), aim=at(0.2, 0.4, 0.3, "r")), hand("l", at(-0.05, -0.02, 1.08, "r"), aim=at(-0.2, 0.4, 0.3, "r")), coat(4, -3))
    cock = merge(gather, {"spine_01": (6, 0, -32)}, hand("r", at(0.34, -0.20, 1.04, "r"), aim=at(0.2, 0.2, 0.3, "r")),
                 hand("l", at(-0.02, -0.12, 1.06, "r"), aim=at(-0.2, 0.2, 0.3, "r")))
    thrust = merge(feet((0.16, 0.34), (0.18, -0.20)), {"spine_01": (14, 0, 0), "spine_02": (6, 0, 0), "head": (-10, 0, 0)},
                   arm("l", 0.08, 1, 0.05, 0.02, 1, 0.08), arm("r", 0.08, 1, 0.05, 0.02, 1, 0.08), coat(10))
    recoil = merge(feet((0.16, 0.24), (0.18, -0.26)), {"spine_01": (-6, 0, 0), "spine_02": (-6, 0, 0), "head": (-6, 0, 0)},
                   arm("l", 0.15, 0.9, 0.25, 0.05, 0.8, 0.5), arm("r", 0.15, 0.9, 0.25, 0.05, 0.8, 0.5), coat(-4))
    keys = [(0, READY, DOWN), (5, gather, {"pelvis": (0, 0.02, -0.08)}), (7, cock, {"pelvis": (0, 0.03, -0.09)}, "snap"),
            (8, thrust, {"pelvis": (0, -0.05, -0.08)}, "snap"), (10, recoil, {"pelvis": (0, 0.06, -0.06)}),
            (15, merge(recoil, {"spine_01": (-4, 0, 0)}), {"pelvis": (0, 0.06, -0.06)}, "settle"), (20, READY, DOWN)]
    return _clip(armature, "q", 20, keys, markers={"fire": 8})


def w(armature):
    # Cinder Bloom: the right arm raised high, cocked back, then swept down to point at the
    # ground where it will erupt on `fire` 8, held.
    st = feet((0.16, 0.22), (0.16, -0.18))
    high = merge(st, {"spine_02": (-10, 0, -8), "head": (-12, 0, 0)}, arm("r", 0.25, 0.1, 1, 0.15, 0.0, 1), arm("l", 0.5, 0.2, -0.5, 0.3, 0.6, 0.0), coat(3))
    cock = merge(high, arm("r", 0.25, -0.25, 0.95, 0.15, -0.4, 0.9), {"spine_02": (-14, 0, -8)})
    point = merge(feet((0.16, 0.30), (0.16, -0.18)), {"spine_01": (16, 0, 6), "spine_02": (6, 0, 0), "head": (-16, 0, 0)},
                  arm("r", 0.15, 0.85, -0.45, 0.08, 0.8, -0.55), arm("l", 0.6, -0.3, -0.4, 0.4, -0.1, -0.3), coat(7))
    keys = [(0, READY, DOWN), (5, high, {"pelvis": (0, 0, 0.0)}), (7, cock, {"pelvis": (0, 0.02, 0.01)}, "snap"), (8, point, {"pelvis": (0, -0.04, -0.08)}),
            (14, merge(point, {"spine_01": (18, 0, 6)}), {"pelvis": (0, -0.04, -0.09)}, "settle"), (22, READY, DOWN)]
    return _clip(armature, "w", 22, keys, markers={"fire": 8})


def e(armature):
    # Flicker: a snap of the fingers, the arms crossed tight as he vanishes on `fire` 3, flung
    # open as he reappears.
    cross = merge(feet((0.12, 0.06), (0.12, -0.06)), {"spine_01": (14, 0, 0), "head": (-10, 0, 0)}, arm("l", 0.1, 0.7, 0.1, -0.8, 0.3, 0.4),
                  arm("r", 0.1, 0.7, 0.1, -0.8, 0.3, 0.4), coat(2))
    open_ = merge(feet((0.18, 0.12), (0.18, -0.12)), {"spine_01": (-4, 0, 0), "head": (-10, 0, 0)}, arm("l", 1, 0.2, 0.1, 1, 0.3, 0.2),
                  arm("r", 1, 0.2, 0.1, 1, 0.3, 0.2), coat(14))
    keys = [(0, READY, DOWN), (2, cross, {"pelvis": (0, 0, -0.12)}, "snap"), (3, cross, {"pelvis": (0, 0, -0.12)}, "snap"), (5, open_, {"pelvis": (0, 0, -0.03)}),
            (8, merge(open_, coat(8)), {"pelvis": (0, 0, -0.04)}, "settle"), (12, READY, DOWN)]
    return _clip(armature, "e", 12, keys, markers={"fire": 3})


def r(armature):
    # Binding Sigil: both hands trace the sigil's circle in the air, then hurl it forward on
    # `fire` 9; the throw is held while it flies.
    st = feet((0.16, 0.20), (0.18, -0.18))
    keys = [(0, READY, DOWN)]
    for i, f in enumerate((2, 4, 6)):
        a = math.radians(90 * i + 45)
        keys.append((f, merge(st, {"spine_01": (2, 0, 0), "head": (-8, 0, 0)},
                              hand("r", at(0.18 + 0.18 * math.cos(a), 0.40, 1.40 + 0.18 * math.sin(a), "r"), aim=at(0, 1, 0.2, "r")),
                              hand("l", at(0.18 + 0.18 * math.cos(a), 0.40, 1.40 - 0.18 * math.sin(a), "l"), aim=at(0, 1, 0.2, "l")), coat(4)), DOWN))
    pull = merge(st, {"spine_01": (-6, 0, 0), "spine_02": (-8, 0, 0), "head": (-6, 0, 0)}, hand("r", at(0.22, 0.05, 1.45, "r"), aim=at(0, 1, 0.4, "r")),
                 hand("l", at(0.22, 0.05, 1.45, "l"), aim=at(0, 1, 0.4, "l")), coat(2))
    hurl = merge(feet((0.16, 0.32), (0.18, -0.20)), {"spine_01": (16, 0, 0), "spine_02": (8, 0, 0), "head": (-12, 0, 0)},
                 arm("l", 0.25, 1, 0.1, 0.15, 1, 0.12), arm("r", 0.25, 1, 0.1, 0.15, 1, 0.12), coat(12))
    keys += [(8, pull, {"pelvis": (0, 0.03, -0.04)}, "snap"), (9, hurl, {"pelvis": (0, -0.05, -0.08)}),
             (16, merge(hurl, {"spine_01": (18, 0, 0)}), {"pelvis": (0, -0.05, -0.09)}, "settle"), (26, READY, DOWN)]
    return _clip(armature, "r", 26, keys, markers={"fire": 9})


SIT = merge({"ik_foot_l": clips.ik(at(0.16, 0.30, champion.ankle(), "l")), "thigh_r": (8, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
            hand("l", at(0.04, 0.32, 1.02, "l"), aim=at(-0.6, 0.4, 0.6, "l")), hand("r", at(0.04, 0.32, 1.02, "r"), aim=at(-0.6, 0.4, 0.6, "r")),
            {"spine_01": (8, 0, 0), "head": (14, 0, 0)}, coat(-10))


def recall(armature):
    keys = [(0, IDLE, None), (15, SIT, {"pelvis": (0, 0.04, -0.46)}), (45, merge(SIT, {"head": (18, 0, 0)}), {"pelvis": (0, 0.04, -0.45)}),
            (75, SIT, {"pelvis": (0, 0.04, -0.46)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    hit = merge(feet((0.12, -0.04), (0.14, -0.22)), arm("l", 0.6, -0.2, 0.0, 0.5, 0.1, 0.2), arm("r", 0.6, -0.2, 0.0, 0.5, 0.1, 0.2),
                {"spine_01": (-14, 0, 0), "head": (-30, 0, 0)}, coat(-20))
    knees = merge({"thigh_l": (-6, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (110, 0, 0), "calf_r": (110, 0, 0)}, arm("l", 0.3, 0.3, -0.9, 0.2, 0.4, -0.8),
                  arm("r", 0.3, 0.3, -0.9, 0.2, 0.4, -0.8), {"spine_01": (24, 0, 0), "head": (20, 0, 0)}, coat(-6))
    down = merge({"thigh_l": (-20, 0, 0), "thigh_r": (-12, 0, 0), "calf_l": (40, 0, 0), "calf_r": (24, 0, 0)}, arm("l", 0.9, 0.2, 0.2, 0.8, 0.4, 0.1),
                 arm("r", 0.5, 0.6, 0.4, 0.3, 0.8, 0.3), {"pelvis": (-76, 16, 0), "head": (18, 0, -14)}, coat(-30))
    keys = [(0, IDLE, None), (6, hit, {"pelvis": (0, 0.06, -0.02)}, "settle"), (15, knees, {"pelvis": (0, 0.02, -0.5)}, "snap"),
            (25, down, {"pelvis": (0, 0.45, -0.82)}, "snap"), (34, down, {"pelvis": (0, 0.47, -0.84)})]
    return _clip(armature, "death", 34, keys)


def respawn(armature):
    keys = [(0, SIT, {"pelvis": (0, 0.04, -0.46)}, "settle"), (12, merge(IDLE, {"head": (-10, 0, 0)}), {"pelvis": (0, 0, -0.02)}), (24, IDLE, None)]
    return _clip(armature, "respawn", 24, keys)


def select(armature):
    # Snaps his fingers by his face: both palms flare; he spreads his arms, grinning.
    snap = merge(STAND, arm("r", 0.3, 0.6, 0.2, -0.5, 0.4, 0.75), arm("l", 0.5, -0.1, -0.6, -0.4, 0.4, 0.3), {"head": (-8, 0, -12)}, coat(2))
    wide = merge(STAND, arm("l", 1, 0.3, 0.3, 1, 0.4, 0.4), arm("r", 1, 0.3, 0.3, 1, 0.4, 0.4), {"spine_02": (-8, 0, 0), "head": (-18, 0, 0)}, coat(10))
    keys = [(0, IDLE, None), (10, snap, None), (13, merge(snap, arm("r", 0.3, 0.6, 0.25, -0.4, 0.5, 0.75)), None, "snap"), (24, wide, {"pelvis": (0, 0, 0.02)}),
            (40, merge(wide, {"head": (-22, 0, 6)}), {"pelvis": (0, 0, 0.02)}), (52, IDLE, None)]
    return _clip(armature, "select", 52, keys)


def emote_taunt(armature):
    # Flicks a spark off his fingertip at you, then blows on the finger.
    aim_ = merge(IDLE, arm("r", 0.15, 1, 0.2, 0.05, 1, 0.2))
    blow = merge(IDLE, arm("r", 0.2, 0.6, 0.0, -0.4, 0.4, 0.8), {"head": (6, 0, 0)})
    keys = [(0, IDLE, None), (8, aim_, None), (10, merge(aim_, arm("r", 0.15, 1, 0.3, 0.05, 1, 0.4)), None, "snap"), (22, blow, None), (32, blow, None), (42, IDLE, None)]
    return _clip(armature, "emote_taunt", 42, keys)


def emote_joke(armature):
    # Warms his hands at his own palm-fire, rubbing them.
    rub = merge(STAND, hand("l", at(0.06, 0.32, 1.25, "l"), aim=at(-0.6, 0.6, 0.3, "l")), hand("r", at(0.06, 0.32, 1.25, "r"), aim=at(-0.6, 0.6, 0.3, "r")),
                {"spine_01": (8, 0, 0), "head": (6, 0, 0)}, coat(2))
    keys = [(0, IDLE, None), (10, rub, None)]
    for i, f in enumerate(range(14, 38, 4)):
        keys.append((f, merge(rub, hand("l", at(0.06, 0.32 + (0.04 if i % 2 else -0.04), 1.25, "l"), aim=at(-0.6, 0.6, 0.3, "l"))), None))
    keys.append((48, IDLE, None))
    return _clip(armature, "emote_joke", 48, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, None)]
    for i in range(1, 9):
        s = 1.0 if i % 2 else 0.5
        keys.append((4 * i, merge(IDLE, {"spine_01": (-8 * s, 0, 0), "spine_02": (-6 * s, 0, 0), "head": (-22 * s, 0, 0)}, sym("clavicle", 0, -5 * s, 0)), None))
    keys.append((40, IDLE, None))
    return _clip(armature, "emote_laugh", 40, keys)


def emote_dance(armature):
    # A swaggering hip-sway, palms burning held high.
    up = merge(arm("l", 0.5, 0.2, 0.75, 0.3, 0.2, 0.9), arm("r", 0.5, 0.2, 0.75, 0.3, 0.2, 0.9))
    a = merge(up, feet((0.13, 0.06), (0.15, -0.06)), {"pelvis": (0, -10, 14), "spine_01": (0, 8, 0)}, coat(4, 8))
    b = merge(up, feet((0.15, 0.06), (0.13, -0.06)), {"pelvis": (0, 10, -14), "spine_01": (0, -8, 0)}, coat(4, -8))
    keys = [(0, a, {"pelvis": (0.04, 0, -0.03)}), (12, merge(up, STAND), {"pelvis": (0, 0, 0.0)}), (24, b, {"pelvis": (-0.04, 0, -0.03)}),
            (36, merge(up, STAND), {"pelvis": (0, 0, 0.0)})]
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    rig.add_chain(armature, "coat", "pelvis", COAT)


def build(only=None):
    return champion.build("ember", SHAPE, parts, OWN, 1.85, extras=_extras, only=only, shared=champion.SHARED + ["attack_melee_alt"])


if __name__ == "__main__":
    champion.main(build)
