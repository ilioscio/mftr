# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Ravenhawks (16 §5): a matriarch and three.

Crested raptor-birds on long legs, wings folded high: violet-black feathers, a pale face mask, a
hooked beak, quill tips glowing violet (`accent_glow`), the crest and the mantle's edge in the
camp's violet (`accent`). The matriarch is bigger, with a tall crest and a mantle of quills. A
`creature` skeleton (16 §3): body, chest, neck, head, a hooked lower beak, a crest, wings of two
bones, a tail and legs with talons; `socket_projectile` at the right wing's tip (they fling
quills).

Clips: idle (head-cocks), idle_fidget_1 (a preen and a wing stretch), run (a hop-flap),
attack_1 (a wing flick that flings a quill on `fire`), death, flinch. `fire` = 0.3 / 0.65
attacks a second = 13.8 frames, so 14; 46 frames a cycle.
"""

import math

from mftr_blender import mesh, rig
from mftr_blender.clips import merge
from mftr_blender.library import _clip

from creature import bone, socket, table

FEATHER = (0.03, 0.022, 0.04)
FEATHER_L = (0.06, 0.045, 0.075)
MASK = (0.42, 0.4, 0.38)
BEAK = (0.18, 0.13, 0.05)
LEG = (0.16, 0.12, 0.04)
TALON = (0.02, 0.02, 0.02)
EDGE = (0.8, 0.8, 0.8)     # accent: the crest and the mantle's edge (the camp's violet, shaded)
GLOW = (1.0, 1.0, 1.0)     # accent_glow: quill tips, eyes

S = 1.0

CENTER = [
    bone("root", None, (0, 0, 0), (0, -0.3, 0), deform=False),
    bone("body", "root", (0, 0.16, 0.62), (0, -0.12, 0.74)),
    bone("chest", "body", (0, -0.12, 0.74), (0, -0.22, 0.9)),
    bone("neck", "chest", (0, -0.22, 0.9), (0, -0.28, 1.04)),
    bone("head", "neck", (0, -0.28, 1.04), (0, -0.48, 1.04)),
    bone("beak", "head", (0, -0.42, 1.0), (0, -0.56, 0.96)),
    bone("crest", "head", (0, -0.3, 1.1), (0, -0.2, 1.3)),
    bone("tail", "body", (0, 0.16, 0.62), (0, 0.46, 0.5)),
]
LEFT = [
    bone("wing_l", "chest", (0.12, -0.16, 0.86), (0.24, 0.06, 0.9)),
    bone("wing_tip_l", "wing_l", (0.24, 0.06, 0.9), (0.26, 0.52, 0.72)),
    bone("thigh_l", "body", (0.08, 0.02, 0.6), (0.09, -0.04, 0.42)),
    bone("shin_l", "thigh_l", (0.09, -0.04, 0.42), (0.09, 0.04, 0.06)),
    bone("foot_l", "shin_l", (0.09, 0.04, 0.06), (0.09, -0.1, 0.035)),
]
SOCKETS = [socket("socket_chest", "chest", (0, -0.26, 0.82)), socket("socket_overhead", "root", (0, 0, 1.55)),
           ("socket_projectile", "wing_tip_r", (-0.26, 0.52, 0.72), (-0.26, 0.42, 0.72), False, rig.UP)]


def B(name, t, off=(0.0, 0.0, 0.0)):
    return (name, t, tuple(x * S for x in off))


def body(matriarch):
    def r(*v):
        return tuple(x * S for x in v)

    p = [
        mesh.path_part("body", [B("body", -0.1), B("body", 0.5, (0, 0, -0.01)), B("body", 1.05)], 9, [r(0.13, 0.15), r(0.17, 0.19), r(0.16, 0.18)], "cloth", FEATHER),
        mesh.path_part("chest", [B("chest", -0.1), B("chest", 0.6), B("chest", 1.05)], 9, [r(0.16, 0.17), r(0.15, 0.16), r(0.1, 0.1)], "cloth", FEATHER_L),
        mesh.path_part("neck", [B("neck", -0.1), B("neck", 1.05)], 8, [r(0.09, 0.09), r(0.075, 0.075)], "cloth", FEATHER),
        mesh.path_part("head", [B("head", -0.05), B("head", 0.5), B("head", 0.85)], 8, [r(0.085, 0.09), r(0.08, 0.08), r(0.05, 0.05)], "cloth", FEATHER),
        # The pale mask around the eyes; the hooked beak.
        mesh.path_part("head", [B("head", 0.45, (0, 0, 0.0)), B("head", 0.75, (0, 0, -0.005))], 8, [r(0.088, 0.07), r(0.06, 0.05)], "cloth", MASK),
        mesh.path_part("beak", [B("beak", -0.2, (0, 0, 0.03)), B("beak", 0.6, (0, 0, 0.015)), B("beak", 1.0, (0, 0, -0.03))], 6,
                       [r(0.045, 0.04), r(0.03, 0.03), (0, 0)], "cloth", BEAK),
        # The crest: a fan of plumes, edged in violet.
        mesh.path_part("crest", [B("crest", -0.1), B("crest", 1.0)], 4, [r(0.03, 0.012), (0, 0)], "accent", EDGE),
        # The tail: a long fan of feathers, quill tips glowing.
        mesh.path_part("tail", [B("tail", 0.0), B("tail", 0.9)], 6, [r(0.07, 0.03), r(0.13, 0.025)], "cloth", FEATHER),
        mesh.path_part("tail", [B("tail", 0.9), B("tail", 1.05)], 6, [r(0.13, 0.025), r(0.12, 0.02)], "accent_glow", GLOW),
    ]
    left = [
        mesh.box_part("head", B("head", 0.55, (0.07, 0, 0.02)), r(0.03, 0.03, 0.025), "accent_glow", GLOW),
        mesh.path_part("crest", [B("crest", 0.0, (0.03, 0, 0)), B("crest", 0.9, (0.08, 0.06, -0.02))], 4, [r(0.025, 0.01), (0, 0)], "accent", EDGE),
        # Wings folded high: a broad shoulder of coverts, long primaries with glowing tips.
        mesh.path_part("wing_l", [B("wing_l", -0.1), B("wing_l", 0.5), B("wing_l", 1.0)], 6, [r(0.08, 0.035), r(0.16, 0.035), r(0.17, 0.03)], "cloth", FEATHER_L),
        mesh.path_part("wing_tip_l", [B("wing_tip_l", 0.0), B("wing_tip_l", 0.5), B("wing_tip_l", 0.88)], 5, [r(0.17, 0.03), r(0.15, 0.025), r(0.1, 0.02)], "cloth", FEATHER),
        mesh.path_part("wing_tip_l", [B("wing_tip_l", 0.88), B("wing_tip_l", 1.12)], 5, [r(0.1, 0.02), (0, 0)], "accent_glow", GLOW),
        # Long scaled legs, talons.
        mesh.path_part("thigh_l", [B("thigh_l", -0.1), B("thigh_l", 1.0)], 6, [r(0.06, 0.07), r(0.035, 0.035)], "cloth", FEATHER),
        mesh.path_part("shin_l", [B("shin_l", 0), B("shin_l", 1.0)], 5, [r(0.022, 0.022), r(0.02, 0.02)], "cloth", LEG),
        mesh.path_part("foot_l", [B("foot_l", 0), B("foot_l", 1.1)], 4, [r(0.03, 0.012), r(0.015, 0.01)], "cloth", LEG),
        mesh.path_part("foot_l", [B("foot_l", 0.9), B("foot_l", 1.2, (0, 0, -0.01))], 4, [r(0.012, 0.012), (0, 0)], "cloth", TALON),
    ]
    if matriarch:
        # A tall crest and a mantle of quills over the shoulders.
        p.append(mesh.path_part("crest", [B("crest", 0.6), B("crest", 1.6, (0, 0.08, 0.04))], 4, [r(0.025, 0.01), (0, 0)], "accent", EDGE))
        left += [
            mesh.path_part("chest", [B("chest", 0.9, (0.06, 0.02, 0.02)), B("chest", 0.9, (0.16, 0.2, 0.08))], 4, [r(0.03, 0.01), (0, 0)], "accent", EDGE),
            mesh.path_part("chest", [B("chest", 0.6, (0.1, 0.02, 0.0)), B("chest", 0.6, (0.22, 0.18, 0.02))], 4, [r(0.03, 0.01), (0, 0)], "accent", EDGE),
        ]
    return p + left + [mesh.mirror(b) for b in left]


# --- Clips ----------------------------------------------------------------------------------

REST = {"neck": (-6, 0, 0), "head": (6, 0, 0), "tail": (0, 0, 0)}


def wings(spread=0.0, beat=0.0):
    """`spread` 0 folded .. 1 open (degrees out and up); `beat` lifts them (+) or drives them
    down (-)."""
    # Out: a turn about the up axis swings each wing away from the body; up: a turn about the
    # front-back axis raises it (the beat adds to that); the tips unfold outward.
    lift = spread * 45 + beat * 35
    return {"wing_l": (0, -lift, -spread * 45), "wing_r": (0, lift, spread * 45),
            "wing_tip_l": (0, -spread * 10, -spread * 85), "wing_tip_r": (0, spread * 10, spread * 85)}


def L(z=0.0, y=0.0):
    return {"body": (0, y * S, z * S)}


def idle(arm, matriarch):
    # Quick head-cocks, held; small shuffles of the wings.
    keys = [(0, merge(REST, wings()), L()), (8, merge(REST, {"head": (6, 0, 28)}, wings(0.05)), L(), "hold"),
            (20, merge(REST, {"head": (6, 0, 28)}), L(), "snap"), (22, merge(REST, {"head": (-4, 0, -20)}), L(), "hold"),
            (36, merge(REST, {"head": (-4, 0, -20)}), L(), "snap"), (38, merge(REST, {"head": (12, 0, 0)}, wings(0.08)), L(-0.01), "hold"),
            (48, REST, L())]
    return _clip(arm, "idle", 56, keys, loop=True)


def idle_fidget_1(arm, matriarch):
    # Preens under the left wing, then stretches both wings wide and shakes out.
    preen = merge(REST, {"neck": (20, 0, 50), "head": (30, 0, 40)}, wings(0.25))
    stretch = merge(REST, {"neck": (-14, 0, 0), "head": (-6, 0, 0)}, wings(1.0, 0.4))
    keys = [(0, REST, L()), (10, preen, L(-0.02)), (14, merge(preen, {"head": (40, 0, 46)}), L(-0.02)), (18, preen, L(-0.02)),
            (22, merge(preen, {"head": (40, 0, 46)}), L(-0.02)), (30, stretch, L(0.03), "hold"), (40, stretch, L(0.03)),
            (43, merge(stretch, wings(0.8, -0.2)), L(0.02)), (46, merge(stretch, wings(0.9, 0.2)), L(0.03)), (56, REST, L())]
    return _clip(arm, "idle_fidget_1", 56, keys)


def run(arm, matriarch):
    # A hop-flap: two-footed bounds, the wings half open and beating.
    frames = 14
    keys = []
    for f in range(0, frames, 2):
        p = f / frames
        up = math.sin(math.pi * p)
        beat = math.cos(2 * math.pi * p)
        legs = {"thigh_l": (-30 * up, 0, 0), "thigh_r": (-30 * up, 0, 0), "shin_l": (40 * up, 0, 0), "shin_r": (40 * up, 0, 0)}
        keys.append((f, merge(REST, {"body": (14, 0, 0), "neck": (-18, 0, 0), "tail": (-10 * beat, 0, 0)}, wings(0.55, beat), legs), L(0.22 * up - 0.02)))
    return _clip(arm, "run", frames, keys, loop=True, markers={"foot_l": 0, "foot_r": 1}, stride_speed=400.0)


def attack_1(arm, matriarch):
    # Rears back, the right wing drawn up and back; flicks it forward on `fire`, a quill
    # leaving its tip; folds it again.
    draw = merge(REST, {"body": (-12, 0, 10), "neck": (-14, 0, 0)}, wings(0.3), {"wing_r": (0, 70, 60), "wing_tip_r": (0, 10, 60)})
    flick = merge(REST, {"body": (10, 0, -14), "neck": (6, 0, 0), "head": (10, 0, 0)}, wings(0.3), {"wing_r": (0, -10, -30), "wing_tip_r": (0, 0, 20)})
    keys = [(0, REST, L(), "ease"), (10, draw, L(0.02, 0.04), "snap"), (14, flick, L(0.0, -0.06), "hold"), (17, flick, L(0.0, -0.06), "settle"),
            (30, merge(REST, wings(0.1)), L()), (46, REST, L(), "ease")]
    return _clip(arm, "attack_1", 46, keys, markers={"fire": 14})


def death(arm, matriarch):
    # A shriek, wings flung open, then a fall onto its side, wings splayed; the last frame holds.
    shriek = merge(REST, {"neck": (-30, 0, 0), "head": (-20, 0, 0), "beak": (20, 0, 0)}, wings(1.0, 0.5))
    fallen = merge({"body": (0, 85, 0), "neck": (20, 0, 20), "head": (10, 0, 0), "beak": (10, 0, 0), "thigh_l": (-40, 0, 0), "thigh_r": (-50, 0, 0)},
                   wings(0.9, -0.3))
    keys = [(0, REST, L(), "ease"), (5, shriek, L(0.06), "hold"), (9, shriek, L(0.06), "whip"), (20, fallen, L(-0.4), "hold")]
    return _clip(arm, "death", 20, keys)


def flinch(arm, matriarch):
    back = merge({"neck": (-10, 0, 0), "head": (-8, 0, 0)}, wings(0.2))
    return _clip(arm, "flinch", 8, [(0, {}, None), (2, back, None), (8, {}, None)], layer="additive",
                 bones=["neck", "head", "wing_l", "wing_r", "wing_tip_l", "wing_tip_r"])


def build(pid):
    global S
    matriarch = pid == "raven_alpha"
    S = 1.2 if matriarch else 0.78
    rig.use_table(table(CENTER, LEFT, SOCKETS, scale=S))
    mesh.AO_HEIGHT = 1.0 * S
    arm = rig.build_armature(name=pid)
    made = [f(arm, matriarch) for f in (idle, idle_fidget_1, run, attack_1, death, flinch)]
    obj = mesh.build_mannequin(arm, name=pid, part_list=body(matriarch))
    print(f"monster {pid}: {len(arm.data.bones)} bones, {mesh.triangle_count(obj)} triangles, clips {[a.name for a in made]}")
