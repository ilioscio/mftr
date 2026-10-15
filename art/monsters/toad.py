# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Bog Toad (16 §5): the lone monster by the river.

A huge round toad: warty olive skin over an ochre belly, a row of teal-glowing warts down its back
(`accent_glow`), a throat sac in the camp's bog teal (`accent`) that swells as it breathes,
bulging yellow eyes, a lily pad and its bloom on its head. A `creature` skeleton (16 §3): the
body, head and jaw, the throat, a three-bone tongue, folded hind legs and front legs.

Clips: idle (the throat pulses), idle_fidget_1 (the tongue snaps a fly), run (hops),
attack_1 (a ranged spit: the throat swells, the head lunges, the glob leaves on `fire`), death
(over onto its back), flinch. `fire` = 0.3 / 0.6 attacks a second = 15 frames; 50 a cycle.
"""

import math

from mftr_blender import mesh, rig
from mftr_blender.clips import merge
from mftr_blender.library import _clip

from creature import bone, socket, table

SKIN = (0.065, 0.055, 0.02)
SKIN_D = (0.03, 0.026, 0.01)
WART = (0.1, 0.085, 0.03)
BELLY = (0.2, 0.14, 0.05)
EYE = (0.45, 0.33, 0.05)
PUPIL = (0.005, 0.005, 0.005)
PAD = (0.05, 0.17, 0.04)
PETAL = (0.85, 0.4, 0.6)
MOUTH = (0.12, 0.03, 0.03)
SAC = (0.85, 0.85, 0.85)   # accent: the throat sac (the camp's teal, shaded)
GLOW = (1.0, 1.0, 1.0)     # accent_glow: the warts

CENTER = [
    bone("root", None, (0, 0, 0), (0, -0.3, 0), deform=False),
    bone("body", "root", (0, 0.32, 0.42), (0, -0.28, 0.52)),
    bone("head", "body", (0, -0.28, 0.56), (0, -0.72, 0.56)),
    bone("jaw", "body", (0, -0.3, 0.46), (0, -0.74, 0.42)),
    bone("throat", "body", (0, -0.38, 0.36), (0, -0.56, 0.3)),
    bone("tongue_1", "jaw", (0, -0.45, 0.47), (0, -0.6, 0.47)),
    bone("tongue_2", "tongue_1", (0, -0.6, 0.47), (0, -0.68, 0.47)),
    bone("tongue_3", "tongue_2", (0, -0.68, 0.47), (0, -0.74, 0.47)),
]
LEFT = [
    bone("arm_l", "body", (0.36, -0.3, 0.38), (0.48, -0.46, 0.18)),
    bone("hand_l", "arm_l", (0.48, -0.46, 0.18), (0.5, -0.62, 0.02)),
    bone("thigh_l", "body", (0.42, 0.18, 0.38), (0.66, -0.06, 0.28)),
    bone("shin_l", "thigh_l", (0.66, -0.06, 0.28), (0.6, 0.32, 0.12)),
    bone("foot_l", "shin_l", (0.6, 0.32, 0.12), (0.7, -0.02, 0.02)),
]
SOCKETS = [socket("socket_chest", "body", (0, -0.6, 0.48)), socket("socket_overhead", "root", (0, 0, 1.45)),
           socket("socket_projectile", "jaw", (0, -0.8, 0.48))]


def B(name, t, off=(0.0, 0.0, 0.0)):
    return (name, t, off)


def body():
    p = [
        # The body: a broad dome over a pale belly.
        mesh.path_part("body", [B("body", -0.1, (0, 0, -0.04)), B("body", 0.3, (0, 0, 0.02)), B("body", 0.7, (0, 0, 0.02)), B("body", 1.05, (0, 0, -0.02))], 12,
                       [(0.52, 0.34), (0.66, 0.42), (0.62, 0.4), (0.46, 0.3)], "cloth", SKIN),
        mesh.path_part("body", [B("body", 0.05, (0, 0, -0.2)), B("body", 0.95, (0, 0, -0.18))], 10, [(0.48, 0.16), (0.42, 0.14)], "cloth", BELLY),
        # The head: wide and flat, the mouth's dark line, the jaw under it.
        mesh.path_part("head", [B("head", -0.05), B("head", 0.5, (0, 0, 0.02)), B("head", 1.0, (0, 0, -0.03))], 10,
                       [(0.42, 0.2), (0.4, 0.18), (0.22, 0.1)], "cloth", SKIN),
        mesh.path_part("jaw", [B("jaw", 0.0), B("jaw", 0.6), B("jaw", 1.0)], 10, [(0.4, 0.1), (0.36, 0.09), (0.18, 0.05)], "cloth", BELLY),
        mesh.path_part("jaw", [B("jaw", 0.2, (0, 0, 0.07)), B("jaw", 0.9, (0, 0, 0.06))], 8, [(0.34, 0.03), (0.16, 0.02)], "cloth", MOUTH),
        # The throat sac, in the camp's color.
        mesh.path_part("throat", [B("throat", -0.3), B("throat", 0.5), B("throat", 1.2)], 10, [(0.12, 0.1), (0.24, 0.17), (0.08, 0.06)], "accent", SAC),
        # The tongue, folded inside the mouth until it snaps out.
        mesh.path_part("tongue_1", [B("tongue_1", 0), B("tongue_1", 1)], 6, [(0.05, 0.02), (0.045, 0.02)], "skin", PETAL),
        mesh.path_part("tongue_2", [B("tongue_2", 0), B("tongue_2", 1)], 6, [(0.045, 0.02), (0.045, 0.02)], "skin", PETAL),
        mesh.path_part("tongue_3", [B("tongue_3", 0), B("tongue_3", 1.2)], 6, [(0.05, 0.022), (0, 0)], "skin", PETAL),
        # A lily pad on its head, a bloom on the pad.
        mesh.path_part("head", [B("head", 0.2, (0.0, 0.05, 0.21)), B("head", 0.2, (0.0, 0.05, 0.24))], 12, [(0.34, 0.34), (0.33, 0.33)], "cloth", PAD),
        mesh.path_part("head", [B("head", 0.2, (0.1, 0.12, 0.24)), B("head", 0.2, (0.1, 0.12, 0.34))], 8, [(0.13, 0.13), (0.03, 0.03)], "cloth", PETAL),
        mesh.path_part("head", [B("head", 0.2, (0.1, 0.12, 0.3)), B("head", 0.2, (0.1, 0.12, 0.36))], 6, [(0.04, 0.04), (0, 0)], "cloth", EYE),
    ]
    # Warts over the back, and a row of glowing ones down the spine.
    warts = [(0.25, 0.2, 0.3), (-0.3, 0.3, 0.28), (0.35, 0.6, 0.24), (-0.38, 0.65, 0.22), (0.18, 0.85, 0.28), (-0.2, 0.05, 0.32),
             (0.42, 0.4, 0.18), (-0.45, 0.45, 0.18)]
    for i, (x, t, z) in enumerate(warts):
        p.append(mesh.rock_part("body", B("body", t, (x, 0, z)), (0.12, 0.12, 0.08), WART, seed=60 + i, rough=0.1, cuts=1, subdiv=1))
    for i, t in enumerate((0.05, 0.3, 0.55, 0.8)):
        p.append(mesh.rock_part("body", B("body", t, (0, 0, 0.4 - 0.03 * abs(i - 1.5))), (0.18, 0.18, 0.12), GLOW, seed=70 + i, rough=0.05, cuts=1,
                                subdiv=1, slot="accent_glow"))
    left = [
        # Bulging eyes on top of the head.
        mesh.rock_part("head", B("head", 0.3, (0.2, 0, 0.17)), (0.17, 0.17, 0.15), EYE, seed=80, rough=0.0, cuts=0, subdiv=2),
        mesh.rock_part("head", B("head", 0.3, (0.22, -0.07, 0.19)), (0.07, 0.04, 0.09), PUPIL, seed=81, rough=0.0, cuts=0, subdiv=1),
        mesh.rock_part("head", B("head", 0.3, (0.2, 0.01, 0.12)), (0.21, 0.21, 0.12), SKIN_D, seed=82, rough=0.05, cuts=1, subdiv=1),
        # Front legs, splayed; folded hind legs, the big feet webbed.
        mesh.path_part("arm_l", [B("arm_l", -0.1), B("arm_l", 1.0)], 7, [(0.09, 0.09), (0.06, 0.06)], "cloth", SKIN),
        mesh.path_part("hand_l", [B("hand_l", 0), B("hand_l", 1.0)], 6, [(0.06, 0.05), (0.09, 0.03)], "cloth", SKIN_D),
        mesh.path_part("thigh_l", [B("thigh_l", -0.15), B("thigh_l", 1.0)], 8, [(0.18, 0.16), (0.11, 0.1)], "cloth", SKIN),
        mesh.path_part("shin_l", [B("shin_l", 0), B("shin_l", 1.0)], 7, [(0.1, 0.09), (0.07, 0.06)], "cloth", SKIN_D),
        mesh.path_part("foot_l", [B("foot_l", 0), B("foot_l", 1.1)], 6, [(0.07, 0.03), (0.15, 0.03)], "cloth", SKIN_D),
    ]
    return p + left + [mesh.mirror(b) for b in left]


# --- Clips ----------------------------------------------------------------------------------

REST = {"head": (-4, 0, 0)}


def L(z=0.0, y=0.0, throat=0.0, tongue=0.0):
    out = {"body": (0, y, z), "throat": (0, -throat * 0.4, -throat)}
    if tongue:
        out["tongue_1"] = (0, -tongue * 0.45, 0)
        out["tongue_2"] = (0, -tongue * 0.75, 0)
        out["tongue_3"] = (0, -tongue, 0)
    return out


def idle(arm):
    # Slow breaths; the throat sac swells and empties twice a cycle.
    keys = []
    for f in range(0, 64, 8):
        p = f / 64
        sac = 0.06 * max(0.0, math.sin(4 * math.pi * p))
        keys.append((f, merge(REST, {"body": (-2 * math.sin(2 * math.pi * p), 0, 0)}), L(0.01 * math.sin(2 * math.pi * p), 0, sac)))
    return _clip(arm, "idle", 64, keys, loop=True)


def idle_fidget_1(arm):
    # Eyes a fly (the head tilts), then the tongue snaps out and back in two frames each.
    look = merge(REST, {"head": (-14, 0, 12), "body": (-4, 0, 6)})
    keys = [(0, REST, L()), (14, look, L(0.02)), (24, look, L(0.02), "snap"), (26, merge(look, {"jaw": (26, 0, 0)}), L(0.02, 0, 0, 0.75), "snap"),
            (29, merge(look, {"jaw": (8, 0, 0)}), L(0.02, 0, 0, 0.0)), (36, merge(look, {"head": (-6, 0, 10)}), L(0.01, 0, 0.08)),
            (44, merge(look, {"head": (-6, 0, 10)}), L(0.01, 0, 0.0)), (60, REST, L())]
    return _clip(arm, "idle_fidget_1", 60, keys)


def run(arm):
    # Hops: a crouch, a launch (the hind legs straighten), a flight, a landing that squashes.
    crouch = merge(REST, {"body": (6, 0, 0), "thigh_l": (0, 0, 0), "thigh_r": (0, 0, 0)})
    launch = merge(REST, {"body": (-14, 0, 0), "thigh_l": (36, 0, -10), "thigh_r": (36, 0, 10), "shin_l": (-40, 0, 0), "shin_r": (-40, 0, 0),
                          "arm_l": (40, 0, 0), "arm_r": (40, 0, 0)})
    flight = merge(REST, {"body": (-4, 0, 0), "thigh_l": (20, 0, -6), "thigh_r": (20, 0, 6), "shin_l": (-20, 0, 0), "shin_r": (-20, 0, 0),
                          "arm_l": (-30, 0, 0), "arm_r": (-30, 0, 0)})
    land = merge(REST, {"body": (8, 0, 0), "arm_l": (-10, 0, 0), "arm_r": (-10, 0, 0), "head": (2, 0, 0)})
    keys = [(0, crouch, L(-0.04)), (4, launch, L(0.12)), (9, flight, L(0.32)), (14, land, L(0.04)), (17, crouch, L(-0.06)), (20, crouch, L(-0.04))]
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 1}, stride_speed=400.0)


def attack_1(arm):
    # Draws back, the throat swelling to bursting; lunges the head forward on `fire`, mouth
    # wide (the glob leaves from `socket_projectile`); the sac empties; it settles.
    draw = merge(REST, {"body": (-12, 0, 0), "head": (-16, 0, 0)})
    spit = merge(REST, {"body": (10, 0, 0), "head": (8, 0, 0), "jaw": (34, 0, 0)})
    keys = [(0, REST, L(), "ease"), (11, draw, L(0.06, 0.08, 0.16), "snap"), (15, spit, L(-0.02, -0.12, 0.02), "hold"),
            (19, spit, L(-0.02, -0.12, 0.0), "settle"), (30, merge(REST, {"jaw": (6, 0, 0)}), L(0, -0.04)), (50, REST, L(), "ease")]
    return _clip(arm, "attack_1", 50, keys, markers={"fire": 15})


def death(arm):
    # A croak, then it rolls onto its back, legs in the air; the last frame holds.
    croak = merge(REST, {"head": (-20, 0, 0), "jaw": (24, 0, 0)})
    back = merge({"body": (0, 175, 0), "thigh_l": (-30, 0, 20), "thigh_r": (-30, 0, -20), "arm_l": (-40, 0, 0), "arm_r": (-40, 0, 0),
                  "jaw": (14, 0, 0), "head": (10, 0, 0)})
    keys = [(0, REST, L(), "ease"), (6, croak, L(0.04, 0, 0.12), "hold"), (10, croak, L(0.04, 0, 0.12), "whip"),
            (20, merge(back, {"body": (0, 90, 0)}), L(0.5, 0, 0.0)), (28, back, L(0.95, 0, 0.0), "hold")]
    return _clip(arm, "death", 28, keys)


def flinch(arm):
    back = {"body": (-6, 0, 0), "head": (-8, 0, 0)}
    return _clip(arm, "flinch", 8, [(0, {}, None), (2, back, None), (8, {}, None)], layer="additive", bones=["body", "head"])


def build(pid):
    rig.use_table(table(CENTER, LEFT, SOCKETS))
    mesh.AO_HEIGHT = 0.9
    arm = rig.build_armature(name=pid)
    made = [f(arm) for f in (idle, idle_fidget_1, run, attack_1, death, flinch)]
    obj = mesh.build_mannequin(arm, name=pid, part_list=body())
    print(f"monster {pid}: {len(arm.data.bones)} bones, {mesh.triangle_count(obj)} triangles, clips {[a.name for a in made]}")
