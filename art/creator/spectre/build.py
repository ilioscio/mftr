# SPDX-License-Identifier: AGPL-3.0-or-later
"""The floating spectre build's base (CR10, 20 §3): no legs; a hooded wraith whose robe trails
off into a wisp that brushes the ground, long clawed arms, eyes in the signature hue. It
hovers and glides. Its own clips are its move library (`mftr_blender.builds`).

    blender -b --python art/creator/spectre/build.py -- [--export]
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
from mftr_blender.clips import merge  # noqa: E402
from mftr_blender.library import _clip  # noqa: E402

NAME = "spectre"
S = 1.05

ROBE = (0.24, 0.24, 0.3)
ROBE_D = (0.12, 0.12, 0.16)
VOID = (0.03, 0.03, 0.04)
BONE = (0.72, 0.7, 0.62)
ACCENT = (0.18, 0.52, 0.95)
GLOW = (1.0, 1.0, 1.0)
IRON = (0.42, 0.43, 0.47)
GOLD = (0.66, 0.50, 0.20)

CENTER = [
    bone("root", None, (0, 0, 0), (0, -0.3, 0), deform=False),
    bone("pelvis", "root", (0, 0, 0.85), (0, 0, 1.05)),
    bone("spine", "pelvis", (0, 0, 1.05), (0, 0, 1.3)),
    bone("chest", "spine", (0, 0, 1.3), (0, 0, 1.55)),
    bone("neck", "chest", (0, 0, 1.55), (0, -0.02, 1.68)),
    bone("head", "neck", (0, -0.02, 1.68), (0, -0.04, 1.95)),
    bone("wisp_1", "pelvis", (0, 0.02, 0.85), (0, 0.1, 0.55)),
    bone("wisp_2", "wisp_1", (0, 0.1, 0.55), (0, 0.2, 0.28)),
    bone("wisp_3", "wisp_2", (0, 0.2, 0.28), (0, 0.34, 0.06)),
]
LEFT = [
    bone("upperarm_l", "chest", (0.2, 0, 1.5), (0.42, -0.02, 1.25)),
    bone("forearm_l", "upperarm_l", (0.42, -0.02, 1.25), (0.56, -0.12, 1.0)),
    bone("hand_l", "forearm_l", (0.56, -0.12, 1.0), (0.62, -0.2, 0.86)),
]
SOCKETS = [
    socket("socket_chest", "chest", (0, -0.15, 1.4)),
    socket("socket_overhead", "root", (0, 0, 2.2)),
    socket("socket_projectile", "hand_r", (-0.62, -0.24, 0.84)),
    socket("socket_cast", "hand_r", (-0.62, -0.24, 0.84)),
    socket("socket_weapon_tip", "hand_r", (-0.62, -0.24, 0.84)),
]
ARMS = tuple(f"{b}_{s}" for b in ("upperarm", "forearm", "hand") for s in "lr")


def B(name, t, off=(0.0, 0.0, 0.0)):
    return (name, t, tuple(x * S for x in off))


def r(*v):
    return tuple(x * S for x in v)


def body():
    p = [
        # The robe: broad at the shoulders, falling into the wisp.
        mesh.path_part("chest", [B("chest", 1.0), B("chest", 0.3), B("chest", -0.08)], 14, [r(0.24, 0.17), r(0.22, 0.16), r(0.21, 0.16)], "cloth", ROBE),
        mesh.path_part("spine", [B("spine", 1.08), B("spine", -0.08)], 14, [r(0.21, 0.16), r(0.2, 0.17)], "cloth", ROBE),
        mesh.path_part("pelvis", [B("pelvis", 1.0), B("pelvis", 0.0)], 14, [r(0.2, 0.17), r(0.22, 0.19)], "cloth", ROBE_D),
        mesh.path_part("wisp_1", [B("wisp_1", 0.0), B("wisp_1", 1.0)], 14, [r(0.22, 0.19), r(0.16, 0.14)], "cloth", ROBE),
        mesh.path_part("wisp_2", [B("wisp_2", 0.0), B("wisp_2", 1.0)], 12, [r(0.16, 0.14), r(0.09, 0.08)], "cloth", ROBE_D),
        mesh.path_part("wisp_3", [B("wisp_3", 0.0), B("wisp_3", 1.05)], 10, [r(0.09, 0.08), (0, 0)], "cloth", ROBE),
        mesh.path_part("neck", [B("neck", 0.0), B("neck", 1.0)], 10, [r(0.08, 0.08), r(0.07, 0.07)], "skin", BONE),
        # The hood, its void, the eyes in the signature hue.
        mesh.path_part("head", [B("head", -0.1, (0, 0.02, 0)), B("head", 0.45, (0, 0.03, 0)), B("head", 1.05, (0, 0.05, 0))], 12,
                       [r(0.14, 0.15), r(0.15, 0.16), (0, 0)], "cloth", ROBE_D),
        mesh.box_part("head", B("head", 0.4, (0, -0.1, 0)), r(0.17, 0.04, 0.17), "cloth", VOID, bevel=0.35),
        # The team accent (05 §1.3): a sash round the waist.
        mesh.path_part("pelvis", [B("pelvis", 0.65), B("pelvis", 0.85)], 14, [r(0.215, 0.18), r(0.21, 0.175)], "accent", ACCENT),
    ]
    left = [
        mesh.box_part("head", B("head", 0.42, (0.045, -0.125, 0.0)), r(0.035, 0.02, 0.022), "emissive", GLOW),
        mesh.path_part("upperarm_l", [B("upperarm_l", -0.1), B("upperarm_l", 1.0)], 10, [r(0.09, 0.08), r(0.07, 0.065)], "cloth", ROBE),
        mesh.path_part("forearm_l", [B("forearm_l", 0.0), B("forearm_l", 0.5), B("forearm_l", 1.0)], 10,
                       [r(0.075, 0.07), r(0.1, 0.09), r(0.04, 0.04)], "cloth", ROBE_D),
        mesh.path_part("hand_l", [B("hand_l", -0.1), B("hand_l", 0.6)], 8, [r(0.045, 0.03), r(0.04, 0.025)], "skin", BONE),
    ]
    for k in range(3):
        left.append(mesh.path_part("hand_l", [B("hand_l", 0.5, (0.02 * (k - 1), 0, 0)), B("hand_l", 1.6, (0.04 * (k - 1), -0.04, 0))], 4,
                                   [r(0.014, 0.012), (0, 0)], "skin", BONE))
    return p + left + [mesh.mirror(b) for b in left]


def sliders():
    G = creator

    def centre(r_):
        return (r_.lo + r_.hi) / 2

    return [
        ("head_size", ("head",), lambda r_: G.scale(r_, centre(r_), (1.2, 1.2, 1.2)), None),
        ("head_hood", ("head",), lambda r_: {i: Vector((0, 0, (r_.cos[i].z - r_.lo.z) / r_.size.z * 0.08 * S)) for i in r_.idx}, None),
        ("body_bulk", ("pelvis", "spine", "chest"), lambda r_: G.scale(r_, Vector((0, 0, 0)), (1.2, 1.2, 1.0)), None),
        ("body_shoulders", ("chest", "upperarm_l", "upperarm_r"), lambda r_: G.scale(r_, Vector((0, 0, 0)), (1.18, 1.0, 1.0)), None),
        ("body_arms", ARMS, lambda r_: creator._limb_scale(r_, 1.35), None),
        ("body_tail", ("wisp_1", "wisp_2", "wisp_3"), lambda r_: creator._limb_scale(r_, 1.4), None),
    ]


def catalog():
    out = []

    def add(pid, slot, name, items, hides=(), accent=False):
        out.append((pid, slot, name, list(hides), accent, items))

    add("head.crown", "head", "Crown", [
        mesh.path_part("head", [B("head", 0.85, (0, 0.03, 0)), B("head", 1.0, (0, 0.04, 0))], 12, [r(0.13, 0.13), r(0.12, 0.12)], "metal", GOLD)]
        + [mesh.path_part("head", [B("head", 0.95, (0.12 * math.cos(a), 0.03 + 0.12 * math.sin(a), 0)),
                                   B("head", 1.25, (0.12 * math.cos(a), 0.03 + 0.12 * math.sin(a), 0))], 4, [r(0.02, 0.02), (0, 0)], "metal", GOLD)
           for a in [2 * math.pi * k / 6 for k in range(6)]])
    add("face.mask", "face", "Bone mask", [mesh.box_part("head", B("head", 0.4, (0, -0.14, 0)), r(0.14, 0.03, 0.16), "skin", BONE, bevel=0.3)])
    chain = []
    for side in (1, -1):
        chain.append(mesh.path_part("chest", [B("chest", 0.9, (0.2 * side, -0.05, 0)), B("chest", 0.3, (0.05 * side, -0.18, 0)),
                                              B("spine", 0.2, (-0.15 * side, -0.17, 0))], 4, [r(0.02, 0.02), r(0.02, 0.02), r(0.02, 0.02)], "metal", IRON))
    add("shoulders.chains", "shoulders", "Chains", chain)
    add("back.lantern", "back", "Soul lantern", [
        mesh.path_part("spine", [B("spine", 0.9, (0, 0.18, 0)), B("spine", 0.9, (0, 0.24, 0.35))], 4, [r(0.015, 0.015), r(0.015, 0.015)], "metal", IRON),
        mesh.box_part("spine", B("spine", 0.9, (0, 0.24, 0.42)), r(0.09, 0.09, 0.12), "emissive", GLOW, bevel=0.3)])
    add("arms.bracers", "arms", "Bracers", [mesh.path_part(f"forearm_{s}", [B(f"forearm_{s}", 0.55), B(f"forearm_{s}", 0.9)], 8,
                                                           [r(0.08, 0.075), r(0.06, 0.055)], "accent", ACCENT) for s in "lr"], accent=True)
    return out


# --- Clips -----------------------------------------------------------------------------------

ARMS_DOWN = {"upperarm_l": (0, 0, 0), "upperarm_r": (0, 0, 0)}


def L(z=0.0, y=0.0):
    return {"pelvis": (0, y * S, z * S)}


def float_pose(phase, lean=0.0, sway=8.0):
    s = math.sin(2 * math.pi * phase)
    return ({"spine": (lean * 0.4, 0, 0), "chest": (lean * 0.4, 0, 0), "head": (-lean * 0.5, 0, 0),
             "wisp_1": (-lean - sway * s, 0, 6 * s), "wisp_2": (-lean * 0.5 - sway * math.sin(2 * math.pi * (phase - 0.15)), 0, 8 * s),
             "wisp_3": (-sway * math.sin(2 * math.pi * (phase - 0.3)), 0, 10 * s),
             "forearm_l": (10 + 4 * s, 0, 0), "forearm_r": (10 - 4 * s, 0, 0)},
            L(0.04 * s))


def glide(arm, name, frames, speed, lean):
    keys = []
    for f in range(0, frames, 3):
        pose, loc = float_pose(f / frames, lean, 14)
        keys.append((f, pose, loc))
    return _clip(arm, name, frames, keys, loop=True, markers={"foot_l": 0, "foot_r": frames // 2}, stride_speed=speed)


def clips(arm):
    made = []
    made.append(_clip(arm, "idle", 60, [(f, *float_pose(f / 60)) for f in range(0, 60, 5)], loop=True))
    made.append(_clip(arm, "idle_fidget_1", 48, [(0, {}, None), (14, {"head": (0, 0, 40), "neck": (0, 0, 20)}, None, "hold"),
                                                  (30, {"head": (0, 0, -30), "neck": (0, 0, -15)}, None), (48, {}, None)]))
    made.append(glide(arm, "walk", 40, 150.0, 8))
    made.append(glide(arm, "run", 30, 340.0, 20))
    wind = {"upperarm_r": (-30, 0, -40), "forearm_r": (-40, 0, 0), "chest": (0, 0, 15)}
    rake = {"upperarm_r": (60, 0, 20), "forearm_r": (10, 0, 0), "chest": (10, 0, -15)}
    made.append(_clip(arm, "attack_1", 36, [(0, {}, None, "ease"), (9, wind, L(0.03), "snap"), (12, rake, L(0, -0.1), "hold"),
                                            (36, {}, None, "ease")], markers={"fire": 12}))
    wind_l = {"upperarm_l": (-30, 0, 40), "forearm_l": (-40, 0, 0), "chest": (0, 0, -15)}
    rake_l = {"upperarm_l": (60, 0, -20), "forearm_l": (10, 0, 0), "chest": (10, 0, 15)}
    made.append(_clip(arm, "attack_2", 36, [(0, {}, None, "ease"), (9, wind_l, L(0.03), "snap"), (12, rake_l, L(0, -0.1), "hold"),
                                            (36, {}, None, "ease")], markers={"fire": 12}))
    raise_ = {"upperarm_l": (-80, 0, 20), "upperarm_r": (-80, 0, -20), "forearm_l": (-20, 0, 0), "forearm_r": (-20, 0, 0), "head": (-15, 0, 0)}
    thrust = {"upperarm_l": (-70, 0, 5), "upperarm_r": (-70, 0, -5), "forearm_l": (-5, 0, 0), "forearm_r": (-5, 0, 0), "chest": (10, 0, 0)}
    made.append(_clip(arm, "howl", 34, [(0, {}, None, "ease"), (9, raise_, L(0.08), "hold"), (12, thrust, L(0.0, -0.06), "hold"),
                                        (34, {}, None, "ease")], markers={"fire": 12}))
    up = {"upperarm_l": (-150, 0, 10), "upperarm_r": (-150, 0, -10), "chest": (-10, 0, 0), "head": (-10, 0, 0)}
    down = {"upperarm_l": (30, 0, 10), "upperarm_r": (30, 0, -10), "chest": (25, 0, 0), "spine": (10, 0, 0)}
    made.append(_clip(arm, "slam", 42, [(0, {}, None, "ease"), (12, up, L(0.2), "hold"), (15, up, L(0.22), "whip"),
                                        (18, down, L(-0.15), "hold"), (26, down, L(-0.15), "settle"), (42, {}, None, "ease")], markers={"fire": 18}))
    lunge = merge(rake, {"spine": (25, 0, 0), "chest": (15, 0, 0), "wisp_1": (-40, 0, 0), "wisp_2": (-30, 0, 0)})
    made.append(_clip(arm, "pounce", 40, [(0, {}, None, "ease"), (7, wind, L(0.1, 0.1), "snap"), (11, lunge, L(0, -0.4), "snap"),
                                          (14, lunge, L(0, -0.42), "hold"), (40, {}, None, "ease")], markers={"fire": 14}))
    streak = {"spine": (35, 0, 0), "chest": (20, 0, 0), "head": (-30, 0, 0), "wisp_1": (-55, 0, 0), "wisp_2": (-30, 0, 0), "wisp_3": (-20, 0, 0),
              "upperarm_l": (40, 0, 0), "upperarm_r": (40, 0, 0)}
    made.append(_clip(arm, "dash_start", 8, [(0, {}, None, "ease"), (8, streak, L(0.05), "snap")], markers={"fire": 7}))
    made.append(_clip(arm, "dash_travel", 12, [(0, streak, L(0.05)), (6, merge(streak, {"wisp_3": (-35, 0, 10)}), L(0.07)), (12, streak, L(0.05))],
                      loop=True, stride_speed=900.0))
    made.append(_clip(arm, "dash_land", 12, [(0, streak, L(0.05), "hold"), (12, {}, None, "ease")]))
    gone = {"spine": (40, 0, 0), "chest": (30, 0, 0), "head": (40, 0, 0), "upperarm_l": (40, 0, 30), "upperarm_r": (40, 0, -30),
            "wisp_1": (40, 0, 0), "wisp_2": (30, 0, 0)}
    made.append(_clip(arm, "death", 26, [(0, {}, None, "ease"), (6, {"head": (-30, 0, 0), "upperarm_l": (-60, 0, 0), "upperarm_r": (-60, 0, 0)}, L(0.1), "hold"),
                                         (26, gone, L(-0.55), "hold")]))
    back = {"chest": (-8, 0, 0), "head": (-10, 0, 0)}
    made.append(_clip(arm, "flinch", 8, [(0, {}, None), (2, back, None), (8, {}, None)], layer="additive", bones=["chest", "head"]))
    made.append(_clip(arm, "cc_stunned", 24, [(f, {"head": (20, 0, 25 * math.sin(2 * math.pi * f / 24)), "chest": (10, 0, -10 * math.sin(2 * math.pi * f / 24))},
                                               L(-0.08)) for f in range(0, 24, 4)], loop=True))
    still = {"upperarm_l": (-20, 0, -40), "upperarm_r": (-20, 0, 40), "forearm_l": (-80, 0, 0), "forearm_r": (-80, 0, 0), "head": (15, 0, 0)}
    made.append(_clip(arm, "recall", 48, [(0, {}, None, "ease"), (20, still, L(0.15), "hold"), (48, still, L(0.15), "hold")],
                      markers={"loop_in": 20, "loop_out": 48}))
    return made


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    builds.run(HERE, NAME, table(CENTER, LEFT, SOCKETS, scale=S), body, sliders, catalog, clips, 2.0 * S, argv)
