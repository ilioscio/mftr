# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Thicket Hounds (16 §5): an alpha and two pups.

Lean wolves of the jungle: dark umber fur, bark plates down the spine, a ruff of leaves whose tips
take the camp's moss green (`accent`), green eyes glowing (`accent_glow`); the alpha bigger, with
bramble antlers. A `creature` skeleton (16 §3): four legs (the hind ones with a hock), a neck,
a jaw, ears and a three-bone tail. The pups are the alpha's skeleton at 0.68, without antlers.

Clips: idle (panting), idle_fidget_1 (a sniff and a scratch; the alpha howls), run (a gallop),
attack_1 (a crouch and a lunging bite), death, flinch. `fire` = 0.3 / 0.6 attacks a second =
15 frames; an attack lasts a cycle, 50 frames.
"""

import math

from mftr_blender import mesh, rig
from mftr_blender.clips import merge
from mftr_blender.library import _clip

from creature import bone, socket, table

FUR = (0.07, 0.055, 0.04)
FUR_L = (0.11, 0.085, 0.06)
FUR_D = (0.03, 0.022, 0.017)
BELLY = (0.17, 0.13, 0.09)
BARK = (0.04, 0.03, 0.022)
LEAF = (0.04, 0.09, 0.025)
NOSE = (0.008, 0.007, 0.007)
TOOTH = (0.4, 0.37, 0.3)
TIP = (0.75, 0.75, 0.75)   # accent: the ruff's leaf tips (the camp's green, shaded)
GLOW = (1.0, 1.0, 1.0)     # accent_glow: the eyes

S = 1.35  # the build's scale (the alpha 1.35, a pup 0.9)

CENTER = [
    bone("root", None, (0, 0, 0), (0, -0.3, 0), deform=False),
    bone("pelvis", "root", (0, 0.36, 0.72), (0, 0.16, 0.75)),
    bone("spine", "pelvis", (0, 0.16, 0.75), (0, -0.12, 0.78)),
    bone("chest", "spine", (0, -0.12, 0.78), (0, -0.38, 0.8)),
    bone("neck", "chest", (0, -0.38, 0.82), (0, -0.54, 0.97)),
    bone("head", "neck", (0, -0.54, 0.97), (0, -0.86, 0.92)),
    bone("jaw", "head", (0, -0.6, 0.92), (0, -0.84, 0.86)),
    bone("tail_1", "pelvis", (0, 0.38, 0.74), (0, 0.56, 0.72)),
    bone("tail_2", "tail_1", (0, 0.56, 0.72), (0, 0.74, 0.62)),
    bone("tail_3", "tail_2", (0, 0.74, 0.62), (0, 0.86, 0.46)),
]
LEFT = [
    bone("ear_l", "head", (0.07, -0.58, 1.04), (0.1, -0.56, 1.17)),
    bone("upper_front_l", "chest", (0.13, -0.32, 0.74), (0.15, -0.3, 0.44)),
    bone("lower_front_l", "upper_front_l", (0.15, -0.3, 0.44), (0.15, -0.33, 0.12)),
    bone("paw_front_l", "lower_front_l", (0.15, -0.33, 0.12), (0.15, -0.43, 0.03)),
    bone("thigh_l", "pelvis", (0.13, 0.32, 0.7), (0.15, 0.24, 0.42)),
    bone("shin_l", "thigh_l", (0.15, 0.24, 0.42), (0.15, 0.4, 0.2)),
    bone("hock_l", "shin_l", (0.15, 0.4, 0.2), (0.15, 0.36, 0.05)),
    bone("paw_hind_l", "hock_l", (0.15, 0.36, 0.05), (0.15, 0.26, 0.02)),
]
SOCKETS = [socket("socket_chest", "chest", (0, -0.42, 0.72)), socket("socket_overhead", "root", (0, 0, 1.4))]


def B(name, t, off=(0.0, 0.0, 0.0)):
    return (name, t, tuple(x * S for x in off))


def body(alpha):
    def r(*v):
        return tuple(x * S for x in v)

    p = [
        # The body: rigid segments, deep in the chest, tucked at the waist.
        mesh.path_part("pelvis", [B("pelvis", -0.15), B("pelvis", 1.0)], 8, [r(0.15, 0.17), r(0.17, 0.2)], "cloth", FUR),
        # A streak of glowing moss down the back between the plates: the camp's green, from above.
        mesh.path_part("chest", [B("chest", 1.0, (0, 0, 0.24)), B("chest", 0.3, (0, 0, 0.27)), B("spine", 0.6, (0, 0, 0.23)), B("pelvis", 0.2, (0, 0, 0.18))], 4,
                       [r(0.03, 0.02), r(0.04, 0.025), r(0.04, 0.025), r(0.02, 0.015)], "accent_glow", GLOW),
        mesh.path_part("spine", [B("spine", -0.05), B("spine", 0.55, (0, 0, -0.01)), B("spine", 1.05, (0, 0, -0.03))], 8,
                       [r(0.17, 0.19), r(0.18, 0.21), r(0.21, 0.26)], "cloth", FUR_D),
        mesh.path_part("chest", [B("chest", -0.05, (0, 0, -0.04)), B("chest", 0.5, (0, 0, -0.06)), B("chest", 1.05, (0, 0, -0.02))], 8,
                       [r(0.21, 0.27), r(0.22, 0.29), r(0.17, 0.24)], "cloth", FUR_L),
        mesh.path_part("chest", [B("chest", 0.2, (0, 0, -0.24)), B("chest", 0.85, (0, 0, -0.22))], 6, [r(0.12, 0.05), r(0.1, 0.05)], "cloth", BELLY),
        mesh.path_part("neck", [B("neck", -0.1), B("neck", 1.1)], 7, [r(0.14, 0.15), r(0.1, 0.11)], "cloth", FUR),
        # The head: a skull, a long snout, a black nose; the lower jaw and its teeth.
        mesh.path_part("head", [B("head", -0.05, (0, 0, 0.02)), B("head", 0.35, (0, 0, 0.02)), B("head", 0.8, (0, 0, -0.01)), B("head", 1.0, (0, 0, -0.02))], 7,
                       [r(0.11, 0.1), r(0.12, 0.1), r(0.06, 0.055), r(0.045, 0.04)], "cloth", FUR),
        mesh.path_part("head", [B("head", 0.98, (0, 0, -0.015)), B("head", 1.05, (0, 0, -0.02))], 6, [r(0.04, 0.035), r(0.02, 0.02)], "cloth", NOSE),
        mesh.path_part("jaw", [B("jaw", 0.0), B("jaw", 1.0)], 6, [r(0.06, 0.03), r(0.035, 0.02)], "cloth", FUR_L),
        mesh.path_part("jaw", [B("jaw", 0.75, (0, 0, 0.025)), B("jaw", 0.95, (0, 0, 0.03))], 4, [r(0.03, 0.01), r(0.02, 0.008)], "skin", TOOTH),
        # The tail: bushy, dark-tipped.
        mesh.path_part("tail_1", [B("tail_1", 0), B("tail_1", 1)], 6, [r(0.06, 0.06), r(0.08, 0.08)], "cloth", FUR),
        mesh.path_part("tail_2", [B("tail_2", 0), B("tail_2", 1)], 6, [r(0.08, 0.08), r(0.07, 0.07)], "cloth", FUR_L),
        mesh.path_part("tail_3", [B("tail_3", 0), B("tail_3", 1.1)], 6, [r(0.07, 0.07), (0, 0)], "cloth", FUR),
    ]
    # Bark plates down the spine.
    for i, (bn, t) in enumerate((("chest", 0.55), ("chest", 0.05), ("spine", 0.5), ("pelvis", 0.6))):
        p.append(mesh.rock_part(bn, B(bn, t, (0, 0, 0.22 - 0.02 * i)), r(0.3 - 0.03 * i, 0.24, 0.1), BARK, seed=40 + i, cuts=2, subdiv=1))
    left = [
        mesh.path_part("ear_l", [B("ear_l", -0.1), B("ear_l", 1.0)], 4, [r(0.05, 0.02), (0, 0)], "cloth", FUR),
        mesh.box_part("head", B("head", 0.32, (0.065, 0, 0.05)), r(0.06, 0.04, 0.035), "accent_glow", GLOW, rot=(0, 0, 25)),
        # The ruff: leaves swept back from the neck, their tips in the camp's green.
        mesh.path_part("neck", [B("neck", 0.3, (0.1, 0, 0.0)), B("neck", 0.3, (0.24, 0.2, 0.08))], 4, [r(0.09, 0.015), (0, 0)], "cloth", LEAF),
        mesh.path_part("neck", [B("neck", 0.6, (0.08, 0, 0.08)), B("neck", 0.6, (0.2, 0.22, 0.2))], 4, [r(0.08, 0.015), (0, 0)], "cloth", LEAF),
        mesh.path_part("neck", [B("neck", 0.1, (0.12, 0, -0.06)), B("neck", 0.1, (0.26, 0.17, -0.13))], 4, [r(0.07, 0.015), (0, 0)], "cloth", LEAF),
        mesh.path_part("neck", [B("neck", 0.85, (0.04, 0, 0.12)), B("neck", 0.85, (0.1, 0.22, 0.28))], 4, [r(0.07, 0.015), (0, 0)], "cloth", LEAF),
        mesh.path_part("neck", [B("neck", 0.3, (0.24, 0.2, 0.08)), B("neck", 0.3, (0.29, 0.29, 0.1))], 4, [r(0.045, 0.012), (0, 0)], "accent", TIP),
        mesh.path_part("neck", [B("neck", 0.6, (0.2, 0.22, 0.2)), B("neck", 0.6, (0.24, 0.31, 0.24))], 4, [r(0.045, 0.012), (0, 0)], "accent", TIP),
        mesh.path_part("neck", [B("neck", 0.85, (0.1, 0.22, 0.28)), B("neck", 0.85, (0.12, 0.31, 0.33))], 4, [r(0.04, 0.012), (0, 0)], "accent", TIP),
        # Legs: thick at the top, thin below, broad paws.
        mesh.path_part("upper_front_l", [B("upper_front_l", -0.1), B("upper_front_l", 1.0)], 6, [r(0.08, 0.1), r(0.05, 0.05)], "cloth", FUR),
        mesh.path_part("lower_front_l", [B("lower_front_l", 0), B("lower_front_l", 1.0)], 5, [r(0.042, 0.042), r(0.034, 0.034)], "cloth", FUR_L),
        mesh.box_part("paw_front_l", B("paw_front_l", 0.5, (0, 0, 0.0)), r(0.08, 0.12, 0.05), "cloth", FUR, bevel=0.3),
        mesh.path_part("thigh_l", [B("thigh_l", -0.15), B("thigh_l", 1.0)], 6, [r(0.1, 0.13), r(0.055, 0.06)], "cloth", FUR),
        mesh.path_part("shin_l", [B("shin_l", 0), B("shin_l", 1.0)], 5, [r(0.045, 0.045), r(0.035, 0.035)], "cloth", FUR_L),
        mesh.path_part("hock_l", [B("hock_l", 0), B("hock_l", 1.0)], 5, [r(0.033, 0.033), r(0.03, 0.03)], "cloth", FUR_L),
        mesh.box_part("paw_hind_l", B("paw_hind_l", 0.5, (0, 0, 0.0)), r(0.075, 0.12, 0.045), "cloth", FUR, bevel=0.3),
    ]
    if alpha:
        # Bramble antlers: two branches with twigs, a few green-glowing buds.
        left += [
            mesh.path_part("head", [B("head", 0.15, (0.06, 0, 0.08)), B("head", 0.0, (0.16, 0.08, 0.26)), B("head", -0.1, (0.22, 0.2, 0.36))], 5,
                           [r(0.035, 0.035), r(0.025, 0.025), (0, 0)], "cloth", BARK),
            mesh.path_part("head", [B("head", 0.0, (0.16, 0.08, 0.26)), B("head", 0.15, (0.26, 0.0, 0.34))], 4, [r(0.018, 0.018), (0, 0)], "cloth", BARK),
            mesh.path_part("head", [B("head", -0.05, (0.19, 0.14, 0.31)), B("head", -0.1, (0.3, 0.16, 0.26))], 4, [r(0.015, 0.015), (0, 0)], "cloth", BARK),
            mesh.box_part("head", B("head", 0.15, (0.26, 0.0, 0.34)), r(0.03, 0.03, 0.03), "accent_glow", GLOW),
        ]
    return p + left + [mesh.mirror(b) for b in left]


# --- Clips ----------------------------------------------------------------------------------

def legs(fl=0.0, fr=0.0, hl=0.0, hr=0.0, flex=None):
    """Legs swung by angle (degrees; + swings the foot back), `flex` (bone -> degrees) bending
    the lower joints."""
    pose = {"upper_front_l": (fl, 0, 0), "upper_front_r": (fr, 0, 0), "thigh_l": (hl, 0, 0), "thigh_r": (hr, 0, 0)}
    return merge(pose, {k: (v, 0, 0) for k, v in (flex or {}).items()})


STAND = merge(legs(), {"neck": (-4, 0, 0), "head": (4, 0, 0), "tail_1": (14, 0, 0), "tail_2": (10, 0, 0)})


def L(z=0.0, y=0.0):
    return {"pelvis": (0, y * S, z * S)}


def idle(arm, alpha):
    # Panting: the jaw and chest quick; the head turns now and then; the tail sways.
    keys = []
    for f in range(0, 48, 4):
        pant = 1 if (f // 4) % 2 else 0
        turn = 10 * math.sin(2 * math.pi * f / 48)
        keys.append((f, merge(STAND, {"jaw": (6 + 8 * pant, 0, 0), "chest": (-2 * pant, 0, 0), "head": (4, 0, turn), "neck": (-4, 0, turn * 0.5),
                                      "tail_1": (14, 0, 12 * math.sin(2 * math.pi * f / 24)), "tail_2": (10, 0, 10 * math.sin(2 * math.pi * f / 24 - 0.6))}),
                     L(0.006 * pant)))
    return _clip(arm, "idle", 48, keys, loop=True)


def idle_fidget_1(arm, alpha):
    if alpha:
        # The howl: sits back, neck up, jaw open; holds it.
        sit = merge(legs(-6, -6, -30, -30, {"shin_l": 40, "shin_r": 40, "hock_l": -40, "hock_r": -40}),
                    {"pelvis": (-22, 0, 0), "neck": (-40, 0, 0), "head": (-30, 0, 0), "jaw": (30, 0, 0), "tail_1": (40, 0, 0)})
        keys = [(0, STAND, L()), (14, merge(sit, {"jaw": (0, 0, 0)}), L(-0.12, 0.06)), (20, sit, L(-0.14, 0.06), "hold"),
                (44, merge(sit, {"head": (-36, 0, 0)}), L(-0.14, 0.06)), (56, merge(sit, {"jaw": (0, 0, 0), "neck": (-20, 0, 0)}), L(-0.12, 0.06)),
                (72, STAND, L())]
        return _clip(arm, "idle_fidget_1", 72, keys)
    # A pup sniffs the ground, then scratches behind its ear with a hind leg.
    sniff = merge(STAND, {"neck": (40, 0, 10), "head": (26, 0, 0), "jaw": (2, 0, 0)}, legs(-8, -8, 0, 0))
    scratch_a = merge(STAND, {"neck": (6, 0, -20), "head": (10, 0, -30)}, legs(0, 0, 0, -50, {"shin_r": 50, "hock_r": -60}))
    scratch_b = merge(scratch_a, legs(0, 0, 0, -36, {"shin_r": 40, "hock_r": -40}))
    keys = [(0, STAND, L()), (10, sniff, L(-0.04)), (16, merge(sniff, {"head": (32, 0, 6)}), L(-0.04)), (22, sniff, L(-0.04)),
            (32, scratch_a, L(-0.05)), (35, scratch_b, L(-0.05)), (38, scratch_a, L(-0.05)), (41, scratch_b, L(-0.05)), (44, scratch_a, L(-0.05)),
            (56, STAND, L())]
    return _clip(arm, "idle_fidget_1", 56, keys)


def run(arm, alpha):
    # A gallop: the hind legs drive together, the front ones reach; the back flexes and the
    # body bounds. 16 frames a stride at 400 u/s.
    frames = 16
    keys = []
    for f in range(0, frames, 2):
        p = f / frames

        def swing(off, back, fwd):
            q = (p - off) % 1.0
            # Stance (planted, sweeping back) for the first half, then a quick swing forward.
            if q < 0.5:
                return fwd + (back - fwd) * (q / 0.5), 0.0
            s = (q - 0.5) / 0.5
            return back + (fwd - back) * (0.5 - 0.5 * math.cos(math.pi * s)), math.sin(math.pi * s)

        fl, ffl = swing(0.55, 35, -45)
        fr, ffr = swing(0.65, 35, -45)
        hl, fhl = swing(0.0, 40, -30)
        hr, fhr = swing(0.1, 40, -30)
        flex = {"lower_front_l": 50 * ffl, "lower_front_r": 50 * ffr, "paw_front_l": 30 * ffl, "paw_front_r": 30 * ffr,
                "shin_l": 35 * fhl, "shin_r": 35 * fhr, "hock_l": -40 * fhl, "hock_r": -40 * fhr}
        bend = 10 * math.sin(2 * math.pi * (p - 0.2))
        pose = merge(legs(fl, fr, hl, hr, flex), {"spine": (bend, 0, 0), "chest": (-bend * 0.5, 0, 0), "neck": (-10 - bend * 0.5, 0, 0),
                                                  "head": (14, 0, 0), "jaw": (10, 0, 0), "tail_1": (-10, 0, 0), "tail_2": (6, 0, 0)})
        keys.append((f, pose, L(0.05 * math.sin(2 * math.pi * p) - 0.02)))
    return _clip(arm, "run", frames, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=400.0)


def attack_1(arm, alpha):
    # Crouch and coil, lunge with the jaws wide, snap shut on `fire`, a hold, recover.
    crouch = merge(legs(-14, -14, -16, -16, {"lower_front_l": 30, "lower_front_r": 30, "shin_l": 30, "shin_r": 30, "hock_l": -30, "hock_r": -30}),
                   {"neck": (14, 0, 0), "head": (-6, 0, 0), "jaw": (4, 0, 0), "tail_1": (-20, 0, 0)})
    lunge = merge(legs(-50, -40, 40, 36), {"spine": (-4, 0, 0), "neck": (-6, 0, 0), "head": (0, 0, 0), "jaw": (40, 0, 0), "tail_1": (-10, 0, 0)})
    bite = merge(lunge, {"jaw": (0, 0, 0), "head": (10, 0, 0), "neck": (6, 0, 0)})
    keys = [(0, STAND, L(), "ease"), (8, crouch, L(-0.12, 0.08), "snap"), (13, lunge, L(0.04, -0.18), "snap"), (15, bite, L(0.0, -0.22), "hold"),
            (20, merge(bite, {"head": (16, 0, 8)}), L(0.0, -0.2), "settle"), (32, merge(STAND, {"head": (8, 0, 0)}), L(0, -0.05), "ease"),
            (50, STAND, L(), "ease")]
    return _clip(arm, "attack_1", 50, keys, markers={"fire": 15})


def death(arm, alpha):
    # A yelp, the legs buckle, it rolls onto its side; the last frame holds.
    yelp = merge(STAND, {"neck": (-24, 0, 0), "head": (-14, 0, 0), "jaw": (26, 0, 0)}, legs(-10, -10, 10, 10))
    down = merge(legs(-20, -10, 20, 30, {"lower_front_l": 40, "shin_l": 30}), {"pelvis": (0, 80, 0), "neck": (20, 0, -10), "head": (10, 0, 0),
                                                                          "jaw": (12, 0, 0), "tail_1": (30, 0, 20)})
    keys = [(0, STAND, L(), "ease"), (5, yelp, L(0.02), "hold"), (9, yelp, L(0.02), "whip"), (22, down, L(-0.42), "hold")]
    return _clip(arm, "death", 22, keys)


def flinch(arm, alpha):
    back = {"neck": (-10, 0, 0), "head": (-8, 0, 0), "chest": (-4, 0, 0)}
    return _clip(arm, "flinch", 8, [(0, {}, None), (2, back, None), (8, {}, None)], layer="additive", bones=["neck", "head", "chest"])


def build(pid):
    global S
    alpha = pid == "hound_alpha"
    S = 1.35 if alpha else 0.9
    rig.use_table(table(CENTER, LEFT, SOCKETS, scale=S))
    mesh.AO_HEIGHT = 1.0 * S
    arm = rig.build_armature(name=pid)
    made = [f(arm, alpha) for f in (idle, idle_fidget_1, run, attack_1, death, flinch)]
    obj = mesh.build_mannequin(arm, name=pid, part_list=body(alpha))
    print(f"monster {pid}: {len(arm.data.bones)} bones, {mesh.triangle_count(obj)} triangles, clips {[a.name for a in made]}")
