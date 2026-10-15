# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Stone Crawlers (16 §5): an elder and a young one.

Six-legged crabs under a boulder shell: the shell chipped stone, mossy on top, amber crystals grown
from its cracks (`accent_glow`, the camp's amber); two heavy claws, their inner faces banded in
amber (`accent`); small glowing eyes. A `creature` skeleton (16 §3): the shell (body), a head,
two claws of three bones (the pincer opens), three legs a side of two bones. The young one is
the elder at 0.6.

Clips: idle (legs shift, claws flex), idle_fidget_1 (claws clack), run (a skitter: alternating
tripods), attack_1 (a crushing snap on `fire`), death (legs curl, the shell drops), flinch.
`fire` = 0.3 / 0.6 attacks a second = 15 frames; 50 a cycle.
"""

import math

from mftr_blender import mesh, rig
from mftr_blender.clips import merge
from mftr_blender.library import _clip

from creature import bone, socket, table

ROCK = (0.11, 0.105, 0.1)
ROCK_D = (0.05, 0.048, 0.046)
MOSS = (0.045, 0.08, 0.028)
LEGC = (0.075, 0.07, 0.065)
BAND = (0.8, 0.8, 0.8)    # accent: claws' inner faces (amber, shaded)
GLOW = (1.0, 1.0, 1.0)    # accent_glow: crystals and eyes

S = 1.0

CENTER = [
    bone("root", None, (0, 0, 0), (0, -0.3, 0), deform=False),
    bone("body", "root", (0, 0.3, 0.48), (0, -0.3, 0.5)),
    bone("head", "body", (0, -0.3, 0.42), (0, -0.52, 0.38)),
]
LEFT = [
    bone("claw_arm_l", "body", (0.3, -0.32, 0.42), (0.56, -0.66, 0.4)),
    bone("claw_l", "claw_arm_l", (0.56, -0.66, 0.4), (0.5, -1.16, 0.32)),
    bone("pincer_l", "claw_l", (0.44, -0.82, 0.26), (0.36, -1.14, 0.22)),
    bone("leg_1_l", "body", (0.36, -0.12, 0.42), (0.72, -0.24, 0.58)),
    bone("foot_1_l", "leg_1_l", (0.72, -0.24, 0.58), (0.92, -0.34, 0.02)),
    bone("leg_2_l", "body", (0.4, 0.08, 0.42), (0.8, 0.1, 0.6)),
    bone("foot_2_l", "leg_2_l", (0.8, 0.1, 0.6), (1.02, 0.14, 0.02)),
    bone("leg_3_l", "body", (0.36, 0.28, 0.42), (0.7, 0.46, 0.56)),
    bone("foot_3_l", "leg_3_l", (0.7, 0.46, 0.56), (0.88, 0.62, 0.02)),
]
SOCKETS = [socket("socket_chest", "body", (0, -0.5, 0.45)), socket("socket_overhead", "root", (0, 0, 1.45))]


def B(name, t, off=(0.0, 0.0, 0.0)):
    return (name, t, tuple(x * S for x in off))


def body(elder):
    def r(*v):
        return tuple(x * S for x in v)

    p = [
        # The shell: a great chipped boulder, mossy on top; the underside darker.
        mesh.rock_part("body", B("body", 0.5, (0, 0, 0.18)), r(1.3, 1.25, 0.8), ROCK, moss=MOSS, seed=91 if elder else 92, cuts=6, subdiv=3),
        mesh.rock_part("body", B("body", 0.5, (0, 0, -0.08)), r(0.9, 0.9, 0.36), ROCK_D, seed=93, cuts=2, subdiv=2),
        # Amber crystals grown from the shell's cracks.
        mesh.path_part("body", [B("body", 0.45, (0.0, 0, 0.45)), B("body", 0.3, (0.05, 0, 0.95))], 5, [r(0.13, 0.12), (0, 0)], "accent_glow", GLOW),
        mesh.path_part("body", [B("body", 0.6, (0.2, 0, 0.42)), B("body", 0.75, (0.42, 0, 0.78))], 5, [r(0.09, 0.08), (0, 0)], "accent_glow", GLOW),
        mesh.path_part("body", [B("body", 0.55, (-0.22, 0, 0.42)), B("body", 0.7, (-0.36, 0, 0.82))], 5, [r(0.1, 0.09), (0, 0)], "accent_glow", GLOW),
        mesh.path_part("body", [B("body", 0.2, (-0.1, 0, 0.38)), B("body", 0.05, (-0.2, 0, 0.66))], 5, [r(0.07, 0.065), (0, 0)], "accent_glow", GLOW),
        # The head: a low stone wedge under the shell's lip, two amber eyes on stalks.
        mesh.rock_part("head", B("head", 0.45), r(0.4, 0.34, 0.24), ROCK_D, seed=94, cuts=3, subdiv=2),
    ]
    left = [
        mesh.path_part("head", [B("head", 0.3, (0.08, 0, 0.08)), B("head", 0.45, (0.11, 0, 0.18))], 4, [r(0.02, 0.02), r(0.018, 0.018)], "cloth", ROCK_D),
        mesh.rock_part("head", B("head", 0.45, (0.11, 0, 0.2)), r(0.06, 0.06, 0.06), GLOW, seed=95, rough=0.0, cuts=0, subdiv=1, slot="accent_glow"),
        # A claw: a heavy arm, a boulder of a hand, the moving pincer; amber on the inner faces.
        mesh.path_part("claw_arm_l", [B("claw_arm_l", -0.1), B("claw_arm_l", 1.0)], 6, [r(0.13, 0.13), r(0.16, 0.16)], "cloth", LEGC),
        mesh.rock_part("claw_arm_l", B("claw_arm_l", 0.55, (0, 0, 0.06)), r(0.26, 0.3, 0.2), ROCK, moss=MOSS, seed=97, cuts=3, subdiv=1),
        mesh.rock_part("claw_l", B("claw_l", 0.4), r(0.48, 0.66, 0.4), ROCK, moss=MOSS, seed=96, cuts=4, subdiv=2),
        mesh.path_part("claw_l", [B("claw_l", 0.55, (-0.08, 0, -0.05)), B("claw_l", 1.08, (-0.05, 0, -0.06))], 6, [r(0.13, 0.08), r(0.03, 0.02)], "cloth", ROCK),
        mesh.path_part("claw_l", [B("claw_l", 0.6, (-0.17, 0, -0.05)), B("claw_l", 0.98, (-0.12, 0, -0.06))], 4, [r(0.04, 0.03), r(0.025, 0.02)], "accent", BAND),
        mesh.path_part("pincer_l", [B("pincer_l", -0.1), B("pincer_l", 1.08)], 6, [r(0.1, 0.06), r(0.025, 0.02)], "cloth", ROCK_D),
        mesh.path_part("pincer_l", [B("pincer_l", 0.2, (0.05, 0, 0.03)), B("pincer_l", 0.85, (0.035, 0, 0.025))], 4, [r(0.035, 0.02), r(0.02, 0.015)], "accent", BAND),
    ]
    for i in (1, 2, 3):
        left += [
            mesh.path_part(f"leg_{i}_l", [B(f"leg_{i}_l", -0.05), B(f"leg_{i}_l", 1.05)], 6, [r(0.1, 0.1), r(0.08, 0.08)], "cloth", LEGC),
            mesh.rock_part(f"leg_{i}_l", B(f"leg_{i}_l", 1.0), r(0.13, 0.13, 0.13), ROCK, seed=100 + i, cuts=2, subdiv=1),
            mesh.path_part(f"foot_{i}_l", [B(f"foot_{i}_l", 0.0), B(f"foot_{i}_l", 0.75), B(f"foot_{i}_l", 1.0)], 6, [r(0.08, 0.08), r(0.05, 0.05), (0, 0)], "cloth", LEGC),
        ]
    return p + left + [mesh.mirror(b) for b in left]


# --- Clips ----------------------------------------------------------------------------------

def legs(lift=None, swing=None):
    """`lift` (leg n, side) -> 0..1 raises that leg; `swing` -> degrees forward (+) / back."""
    out = {}
    for i in (1, 2, 3):
        for side in ("l", "r"):
            up = (lift or {}).get((i, side), 0.0)
            sw = (swing or {}).get((i, side), 0.0)
            sgn = 1 if side == "l" else -1
            # Up: a turn about the front-back axis; forward: about the up axis.
            out[f"leg_{i}_{side}"] = (0, -sgn * 25 * up, sgn * sw)
            out[f"foot_{i}_{side}"] = (0, sgn * 15 * up, 0)
    return out


def claws(open_l=0.0, open_r=0.0, raise_l=0.0, raise_r=0.0):
    return {"claw_arm_l": (-raise_l, 0, 0), "claw_arm_r": (-raise_r, 0, 0),
            "pincer_l": (0, 0, -open_l * 30), "pincer_r": (0, 0, open_r * 30)}


REST = merge(legs(), claws(0.1, 0.1))


def L(z=0.0, y=0.0, x=0.0):
    return {"body": (x * S, y * S, z * S)}


def idle(arm, elder):
    keys = [(0, REST, L()), (16, merge(legs({(2, "l"): 0.3}), claws(0.3, 0.1, 6, 0), {"head": (0, 0, 6)}), L(0.01)),
            (32, merge(legs({(2, "r"): 0.3}), claws(0.1, 0.4, 0, 8), {"head": (0, 0, -6)}), L(0.0)),
            (48, merge(REST, {"body": (0, 0, 3)}), L(0.01))]
    return _clip(arm, "idle", 64, keys, loop=True)


def idle_fidget_1(arm, elder):
    # Raises both claws and clacks them twice, then settles with a shell-shake.
    up = merge(REST, claws(0.7, 0.7, 20, 20), {"body": (-6, 0, 0)})
    shut = merge(REST, claws(0.0, 0.0, 22, 22), {"body": (-6, 0, 0)})
    keys = [(0, REST, L()), (12, up, L(0.04)), (15, shut, L(0.04), "hold"), (18, up, L(0.04)), (21, shut, L(0.04), "hold"), (26, up, L(0.04)),
            (34, merge(REST, {"body": (0, 6, 0)}), L(0.02)), (37, merge(REST, {"body": (0, -6, 0)}), L(0.02)), (40, merge(REST, {"body": (0, 3, 0)}), L(0.01)),
            (52, REST, L())]
    return _clip(arm, "idle_fidget_1", 52, keys)


def run(arm, elder):
    # A skitter: two tripods (1 and 3 on one side with 2 on the other) step in turn.
    frames = 12
    keys = []
    for f in range(0, frames, 2):
        p = f / frames
        a = max(0.0, math.sin(2 * math.pi * p))
        b = max(0.0, -math.sin(2 * math.pi * p))
        sw = 22 * math.cos(2 * math.pi * p)
        tri_a = {(1, "l"): a, (3, "l"): a, (2, "r"): a, (1, "r"): b, (3, "r"): b, (2, "l"): b}
        swing = {(1, "l"): sw, (3, "l"): sw, (2, "r"): sw, (1, "r"): -sw, (3, "r"): -sw, (2, "l"): -sw}
        keys.append((f, merge(legs(tri_a, swing), claws(0.2, 0.2, 10, 10), {"body": (0, 4 * math.sin(2 * math.pi * p), 0)}),
                     L(0.03 * abs(math.sin(2 * math.pi * p)))))
    return _clip(arm, "run", frames, keys, loop=True, markers={"foot_l": 0, "foot_r": 6}, stride_speed=400.0)


def attack_1(arm, elder):
    # The right claw rears up wide open, then crushes down and snaps shut on `fire`; it holds.
    rear = merge(REST, claws(0.1, 1.0, 0, 40), {"body": (-8, 0, -6)})
    crush = merge(REST, claws(0.1, 0.0, 0, -18), {"body": (8, 0, 4)})
    keys = [(0, REST, L(), "ease"), (11, rear, L(0.04, 0.04), "snap"), (15, crush, L(-0.03, -0.08), "hold"), (20, crush, L(-0.03, -0.08), "settle"),
            (50, REST, L(), "ease")]
    return _clip(arm, "attack_1", 50, keys, markers={"fire": 15})


def death(arm, elder):
    # The legs fold under, the claws drop, the shell slumps to the ground; the last frame holds.
    curl = {}
    for i in (1, 2, 3):
        for side, sgn in (("l", 1), ("r", -1)):
            curl[f"leg_{i}_{side}"] = (0, sgn * 40, 0)
            curl[f"foot_{i}_{side}"] = (0, -sgn * 70, 0)
    down = merge(curl, claws(0.6, 0.5, -20, -24), {"head": (14, 0, 0), "body": (6, 0, 8)})
    keys = [(0, REST, L(), "ease"), (5, merge(REST, claws(0.9, 0.9, 30, 30)), L(0.06), "hold"), (9, merge(REST, claws(0.9, 0.9, 30, 30)), L(0.06), "whip"),
            (20, down, L(-0.4), "hold")]
    return _clip(arm, "death", 20, keys)


def flinch(arm, elder):
    back = merge({"body": (-6, 0, 0), "head": (-8, 0, 0)}, claws(0.3, 0.3, 10, 10))
    return _clip(arm, "flinch", 8, [(0, {}, None), (2, back, None), (8, {}, None)], layer="additive",
                 bones=["body", "head", "claw_arm_l", "claw_arm_r", "pincer_l", "pincer_r"])


def build(pid):
    global S
    elder = pid == "crawler_elder"
    S = 1.0 if elder else 0.6
    rig.use_table(table(CENTER, LEFT, SOCKETS, scale=S))
    mesh.AO_HEIGHT = 1.0 * S
    arm = rig.build_armature(name=pid)
    made = [f(arm, elder) for f in (idle, idle_fidget_1, run, attack_1, death, flinch)]
    obj = mesh.build_mannequin(arm, name=pid, part_list=body(elder))
    print(f"monster {pid}: {len(arm.data.bones)} bones, {mesh.triangle_count(obj)} triangles, clips {[a.name for a in made]}")
