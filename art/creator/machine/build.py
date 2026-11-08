# SPDX-License-Identifier: AGPL-3.0-or-later
"""The rolling machine build's base (CR10, 20 §3): a boiler-bodied construct on one great
drive wheel, two jointed arms ending in clamps, a sensor head that glows in the signature hue.
It rolls; the wheel spins as it goes. Its own clips are its move library (`mftr_blender.builds`).

    blender -b --python art/creator/machine/build.py -- [--export]
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))
sys.path.insert(0, os.path.join(ROOT, "art", "monsters"))

from mathutils import Vector  # noqa: E402

from creature import bone, socket, table  # noqa: E402
from mftr_blender import builds, creator, mesh  # noqa: E402
from mftr_blender.library import _clip  # noqa: E402

NAME = "machine"
S = 1.1

PLATE = (0.46, 0.42, 0.36)
PLATE_D = (0.24, 0.22, 0.2)
BRASS = (0.6, 0.45, 0.2)
RUBBER = (0.08, 0.08, 0.09)
ACCENT = (0.18, 0.52, 0.95)
GLOW = (1.0, 1.0, 1.0)
IRON = (0.42, 0.43, 0.47)

CENTER = [
    bone("root", None, (0, 0, 0), (0, -0.3, 0), deform=False),
    bone("pelvis", "root", (0, 0, 0.42), (0, 0, 0.7)),
    bone("wheel", "pelvis", (0, 0, 0.42), (0, -0.2, 0.42)),
    bone("chest", "pelvis", (0, 0, 0.7), (0, 0, 1.25)),
    bone("neck", "chest", (0, 0, 1.25), (0, -0.02, 1.38)),
    bone("head", "neck", (0, -0.02, 1.38), (0, -0.06, 1.6)),
]
LEFT = [
    bone("upperarm_l", "chest", (0.3, 0, 1.15), (0.48, -0.04, 0.9)),
    bone("forearm_l", "upperarm_l", (0.48, -0.04, 0.9), (0.52, -0.3, 0.8)),
    bone("hand_l", "forearm_l", (0.52, -0.3, 0.8), (0.54, -0.45, 0.78)),
]
SOCKETS = [
    socket("socket_chest", "chest", (0, -0.24, 1.0)),
    socket("socket_overhead", "root", (0, 0, 1.9)),
    socket("socket_projectile", "hand_r", (-0.54, -0.5, 0.78)),
    socket("socket_cast", "head", (0, -0.2, 1.48)),
    socket("socket_weapon_tip", "hand_r", (-0.54, -0.5, 0.78)),
]
ARMS = tuple(f"{b}_{s}" for b in ("upperarm", "forearm", "hand") for s in "lr")
WHEEL_R = 0.42


def B(name, t, off=(0.0, 0.0, 0.0)):
    return (name, t, tuple(x * S for x in off))


def r(*v):
    return tuple(x * S for x in v)


def body():
    w = WHEEL_R
    p = [
        # The drive wheel: a tyre round an axle along X, its spokes to show it turning.
        mesh.path_part("wheel", [B("wheel", 0.0, (-0.11, 0.2, 0)), B("wheel", 0.0, (0.11, 0.2, 0))], 16, [r(w, w), r(w, w)], "cloth", RUBBER,
                       ref=(0, 0, 1)),
        mesh.path_part("wheel", [B("wheel", 0.0, (-0.125, 0.2, 0)), B("wheel", 0.0, (0.125, 0.2, 0))], 16, [r(w * 0.7, w * 0.7), r(w * 0.7, w * 0.7)],
                       "metal", BRASS, ref=(0, 0, 1)),
        # The body: a boiler on the axle, a riveted chest, a sensor head on a short neck.
        mesh.path_part("pelvis", [B("pelvis", 0.0), B("pelvis", 1.0)], 12, [r(0.2, 0.18), r(0.24, 0.22)], "metal", PLATE_D),
        mesh.path_part("chest", [B("chest", 0.0), B("chest", 0.5), B("chest", 1.0)], 12, [r(0.26, 0.24), r(0.32, 0.28), r(0.26, 0.22)], "cloth", PLATE),
        mesh.path_part("neck", [B("neck", 0.0), B("neck", 1.0)], 8, [r(0.07, 0.07), r(0.06, 0.06)], "metal", PLATE_D),
        mesh.box_part("head", B("head", 0.5), r(0.24, 0.2, 0.2), "cloth", PLATE, bevel=0.3),
        mesh.box_part("head", B("head", 0.5, (0, -0.1, 0.0)), r(0.16, 0.03, 0.05), "emissive", GLOW),
        # The team accent (05 §1.3): a band round the chest.
        mesh.path_part("chest", [B("chest", 0.62), B("chest", 0.72)], 12, [r(0.315, 0.275), r(0.31, 0.27)], "accent", ACCENT),
    ]
    for k in range(4):
        a = math.pi * k / 4
        for side in (1, -1):
            p.append(mesh.box_part("wheel", B("wheel", 0.0, (0.12 * side, 0.2, 0)), r(0.03, 2 * w * 0.66 * abs(math.cos(a)) + 0.04,
                                                                                         2 * w * 0.66 * abs(math.sin(a)) + 0.04), "metal", IRON))
    left = [
        mesh.path_part("upperarm_l", [B("upperarm_l", -0.1), B("upperarm_l", 1.0)], 8, [r(0.08, 0.08), r(0.065, 0.065)], "metal", PLATE_D),
        mesh.path_part("forearm_l", [B("forearm_l", 0.0), B("forearm_l", 1.0)], 8, [r(0.075, 0.075), r(0.09, 0.09)], "cloth", PLATE),
        mesh.box_part("upperarm_l", B("upperarm_l", 0.0), r(0.15, 0.15, 0.15), "metal", BRASS, bevel=0.3),
        mesh.path_part("hand_l", [B("hand_l", 0.0, (0.03, 0, 0)), B("hand_l", 1.3, (0.04, 0, 0))], 4, [r(0.04, 0.02), (0, 0)], "metal", IRON),
        mesh.path_part("hand_l", [B("hand_l", 0.0, (-0.03, 0, 0)), B("hand_l", 1.3, (-0.04, 0, 0))], 4, [r(0.04, 0.02), (0, 0)], "metal", IRON),
    ]
    return p + left + [mesh.mirror(b) for b in left]


def sliders():
    G = creator

    def centre(r_):
        return (r_.lo + r_.hi) / 2

    return [
        ("head_size", ("head",), lambda r_: G.scale(r_, centre(r_), (1.25, 1.25, 1.25)), None),
        ("body_bulk", ("chest", "pelvis"), lambda r_: G.scale(r_, Vector((0, 0, 0)), (1.2, 1.2, 1.0)), None),
        ("body_chest", ("chest",), lambda r_: G.scale(r_, Vector((0, 0, 0)), (1.25, 1.0, 1.0)), None),
        ("body_wheel", ("wheel",), lambda r_: G.scale(r_, Vector((0, centre(r_).y, r_.lo.z)), (1.0, 1.2, 1.2)), None),
        ("body_arms", ARMS, lambda r_: creator._limb_scale(r_, 1.35), None),
    ]


def catalog():
    out = []

    def add(pid, slot, name, items, hides=(), accent=False):
        out.append((pid, slot, name, list(hides), accent, items))

    add("head.antenna", "head", "Antenna", [
        mesh.path_part("head", [B("head", 0.8, (0.06, 0, 0.0)), B("head", 0.8, (0.06, 0.02, 0.3))], 4, [r(0.012, 0.012), r(0.01, 0.01)], "metal", IRON),
        mesh.box_part("head", B("head", 0.8, (0.06, 0.02, 0.32)), r(0.04, 0.04, 0.04), "emissive", GLOW)])
    pad = mesh.box_part("upperarm_l", B("upperarm_l", 0.1, (0.02, 0, 0.08)), r(0.22, 0.24, 0.08), "metal", IRON, rot=(0, -30, 0), bevel=0.3)
    add("shoulders.plates", "shoulders", "Shoulder plates", [pad, mesh.mirror(pad)])
    add("back.stack", "back", "Smokestack", [
        mesh.path_part("chest", [B("chest", 0.5, (0, 0.24, 0)), B("chest", 1.0, (0, 0.26, 0.35))], 10, [r(0.07, 0.07), r(0.08, 0.08)], "metal", PLATE_D),
        mesh.path_part("chest", [B("chest", 1.0, (0, 0.26, 0.33)), B("chest", 1.0, (0, 0.26, 0.38))], 10, [r(0.1, 0.1), r(0.1, 0.1)], "accent", ACCENT)],
        accent=True)
    add("torso.boiler_plate", "torso", "Boiler plate", [mesh.box_part("chest", B("chest", 0.45, (0, -0.27, 0)), r(0.4, 0.05, 0.4), "metal", BRASS,
                                                                       bevel=0.25)])
    drill = mesh.path_part("hand_r", [B("hand_r", 0.8), B("hand_r", 2.6)], 6, [r(0.07, 0.07), (0, 0)], "metal", IRON)
    add("weapon.drill", "weapon", "Drill arm", [drill])
    return out


# --- Clips -----------------------------------------------------------------------------------

def L(z=0.0, y=0.0):
    return {"pelvis": (0, y * S, z * S)}


def roll(arm, name, frames, speed, lean):
    keys = []
    for f in range(0, frames + 1, max(1, frames // 8)):
        p = f / frames
        keys.append((f, {"wheel": (-360.0 * p, 0, 0), "chest": (lean + 2 * math.sin(4 * math.pi * p), 0, 0), "head": (-lean * 0.6, 0, 0),
                         "upperarm_l": (lean, 0, 0), "upperarm_r": (lean, 0, 0)}, L(0.01 * math.sin(4 * math.pi * p))))
    return _clip(arm, name, frames, keys, loop=False, markers={"foot_l": 0, "foot_r": frames // 2}, stride_speed=speed)


def clips(arm):
    made = []
    made.append(_clip(arm, "idle", 60, [(f, {"chest": (2 * math.sin(2 * math.pi * f / 60), 0, 0), "head": (0, 0, 20 * math.sin(2 * math.pi * f / 60))},
                                         L(0.008 * math.sin(4 * math.pi * f / 60))) for f in range(0, 60, 5)], loop=True))
    made.append(_clip(arm, "idle_fidget_1", 48, [(0, {}, None), (12, {"head": (0, 0, 90)}, None, "hold"), (24, {"head": (0, 0, -90)}, None, "hold"),
                                                  (48, {}, None)]))
    # A wheel turns a whole turn per loop: a stride of 2πr.
    made.append(roll(arm, "walk", 40, 2 * math.pi * WHEEL_R * S * 30 / 40, 4))
    made.append(roll(arm, "run", 24, 2 * math.pi * WHEEL_R * S * 30 / 24, 12))
    for a in made[-2:]:
        a["mftr_loop"] = True
    wind = {"upperarm_r": (-40, 0, -20), "forearm_r": (-30, 0, 0), "chest": (0, 0, 20)}
    punch = {"upperarm_r": (-80, 0, 10), "forearm_r": (0, 0, 0), "chest": (6, 0, -20)}
    made.append(_clip(arm, "attack_1", 36, [(0, {}, None, "ease"), (9, wind, None, "snap"), (12, punch, L(0, -0.06), "hold"), (36, {}, None, "ease")],
                      markers={"fire": 12}))
    wind_l = {"upperarm_l": (-40, 0, 20), "forearm_l": (-30, 0, 0), "chest": (0, 0, -20)}
    punch_l = {"upperarm_l": (-80, 0, -10), "forearm_l": (0, 0, 0), "chest": (6, 0, 20)}
    made.append(_clip(arm, "attack_2", 36, [(0, {}, None, "ease"), (9, wind_l, None, "snap"), (12, punch_l, L(0, -0.06), "hold"),
                                            (36, {}, None, "ease")], markers={"fire": 12}))
    aim = {"upperarm_r": (-85, 0, 0), "forearm_r": (0, 0, 0), "head": (-5, 0, 0)}
    recoil = {"upperarm_r": (-100, 0, 0), "forearm_r": (-15, 0, 0), "chest": (-6, 0, 0)}
    made.append(_clip(arm, "howl", 32, [(0, {}, None, "ease"), (8, aim, None, "hold"), (11, recoil, L(0, 0.04), "hold"), (32, {}, None, "ease")],
                      markers={"fire": 11}))
    up = {"upperarm_l": (-150, 0, 0), "upperarm_r": (-150, 0, 0), "chest": (-8, 0, 0)}
    down = {"upperarm_l": (-20, 0, 0), "upperarm_r": (-20, 0, 0), "chest": (25, 0, 0)}
    made.append(_clip(arm, "slam", 40, [(0, {}, None, "ease"), (12, up, L(0.06), "hold"), (15, up, L(0.08), "whip"), (18, down, L(-0.04), "hold"),
                                        (26, down, L(-0.04), "settle"), (40, {}, None, "ease")], markers={"fire": 18}))
    lunge = {"wheel": (-180, 0, 0), "chest": (25, 0, 0), "upperarm_l": (-70, 0, 0), "upperarm_r": (-70, 0, 0)}
    made.append(_clip(arm, "pounce", 36, [(0, {}, None, "ease"), (6, {"chest": (-10, 0, 0)}, None, "snap"), (11, lunge, L(0, -0.2), "snap"),
                                          (14, lunge, L(0, -0.22), "hold"), (36, {}, None, "ease")], markers={"fire": 14}))
    made.append(_clip(arm, "dash_start", 8, [(0, {}, None, "ease"), (8, {"chest": (20, 0, 0), "wheel": (-90, 0, 0)}, None, "snap")],
                      markers={"fire": 7}))
    made.append(roll(arm, "dash_travel", 12, 900.0, 22))
    made[-1]["mftr_loop"] = True
    made.append(_clip(arm, "dash_land", 12, [(0, {"chest": (20, 0, 0)}, None, "hold"), (12, {}, None, "ease")]))
    made.append(_clip(arm, "death", 30, [(0, {}, None, "ease"), (8, {"head": (0, 0, 60), "chest": (-10, 0, 0)}, None, "hold"),
                                         (30, {"chest": (70, 0, 20), "head": (30, 0, 0), "upperarm_l": (40, 0, 30), "upperarm_r": (40, 0, -30)},
                                          L(-0.2, -0.1), "hold")]))
    made.append(_clip(arm, "flinch", 8, [(0, {}, None), (2, {"chest": (-8, 0, 0), "head": (-8, 0, 0)}, None), (8, {}, None)], layer="additive",
                      bones=["chest", "head"]))
    made.append(_clip(arm, "cc_stunned", 24, [(f, {"head": (0, 0, 360.0 * f / 24 if f < 24 else 0), "chest": (6, 0, 0)}, None)
                                              for f in range(0, 24, 3)], loop=True))
    folded = {"upperarm_l": (-10, 0, -30), "upperarm_r": (-10, 0, 30), "forearm_l": (-100, 0, 0), "forearm_r": (-100, 0, 0), "head": (20, 0, 0)}
    made.append(_clip(arm, "recall", 48, [(0, {}, None, "ease"), (20, folded, L(-0.04), "hold"), (48, folded, L(-0.04), "hold")],
                      markers={"loop_in": 20, "loop_out": 48}))
    return made


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    builds.run(HERE, NAME, table(CENTER, LEFT, SOCKETS, scale=S), body, sliders, catalog, clips, 1.8 * S, argv)
