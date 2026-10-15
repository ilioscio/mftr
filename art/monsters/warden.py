# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Azure Warden (16 §5): the blue buff's guardian.

A monolith of the paving's stone on two stumps: shoulders like lintels, a small head sunk
between them, pillar forearms ending in block fists; a crack down its chest shows an azure
crystal heart, and three runestones orbit over its shoulders (`extra_rune_*`). Moss on top.
The `accent` and `accent_glow` slots take the camp's azure (the client paints them).

On the `biped` bones in its own proportions (~2.4 m to the head, ~2.9 m to the runes).
Clips: idle, idle_fidget_1, run, attack_1 (a two-fisted slam), attack_2 (a backhand), death,
flinch. `fire` is the sim's windup: 0.3 / 0.5 attacks a second = 18 frames.
"""

import math

from mftr_blender import mesh, rig
from mftr_blender.clips import merge, sym
from mftr_blender.library import _clip, arms, gait

# Linear vertex colors: the paving's blue-grey stone, darker below, moss on top.
STONE = (0.12, 0.13, 0.155)
STONE_L = (0.16, 0.17, 0.19)
STONE_D = (0.07, 0.078, 0.095)
MOSS = (0.045, 0.085, 0.03)
LICHEN = (0.11, 0.12, 0.05)
RUNE = (0.75, 0.75, 0.75)   # accent: the azure inlay (shade only)
GLOW = (1.0, 1.0, 1.0)      # accent_glow: the azure light

SHAPE = {
    # Biped heights -> the Warden's: short stumps, a tall torso, the head sunk between the
    # shoulders; very broad and deep.
    "heights": [(0.0, 0.0), (0.03, 0.03), (0.09, 0.12), (0.54, 0.50), (1.00, 0.95), (1.12, 1.15),
                (1.26, 1.42), (1.40, 1.70), (1.56, 2.02), (1.64, 2.08), (1.88, 2.36), (2.15, 2.95)],
    "x": 1.75, "y": 1.35, "scale": 1.0, "shoulder": 0.16, "arm": 1.25, "hand": 1.9,
}

RUNES = {"a": (0.78, 0.18, 2.42), "b": (-0.78, 0.18, 2.42), "c": (0.0, 0.48, 2.72)}
ATTACK = 60
FIRE = 18


def B(bone, t, off=(0.0, 0.0, 0.0)):
    return (bone, t, off)


def body():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    p = [
        # Pelvis and belly: stacked blocks, wider as they rise.
        mesh.box_part("pelvis", B("pelvis", 0.2), (0.72, 0.52, 0.42), "cloth", STONE_D, bevel=0.3),
        mesh.box_part("spine_01", B("spine_01", 0.5), (0.80, 0.58, 0.36), "cloth", STONE, taper=1.06, bevel=0.25),
        mesh.box_part("spine_02", B("spine_02", 0.5), (0.98, 0.66, 0.38), "cloth", STONE, taper=1.1, bevel=0.2),
        # The chest: an inverted trapezoid; on its back a great mossy boulder, the azure crystals
        # growing out of it (what reads from the gameplay camera, which sees tops).
        mesh.box_part("chest", B("chest", 0.45), (1.2, 0.74, 0.58), "cloth", STONE, taper=1.14, bevel=0.15),
        mesh.rock_part("chest", B("chest", 0.75, (0, 0.2, 0.08)), (1.5, 1.0, 0.78), STONE_L, moss=MOSS, seed=3, cuts=5, subdiv=3),
        mesh.rock_part("chest", B("chest", 0.3, (0, 0.34, -0.05)), (0.9, 0.6, 0.5), STONE, moss=MOSS, seed=8, cuts=4, subdiv=2),
        # The crack and the azure heart showing through it.
        mesh.path_part("chest", [B("chest", 0.0, (0, -0.33, 0)), B("chest", 0.42, (0, -0.48, 0)), B("chest", 0.88, (0, -0.35, 0))],
                       6, [(0, 0), (0.19, 0.13), (0, 0)], "accent_glow", GLOW),
        # The crystal spires on the back: one tall, others leaning out.
        mesh.path_part("chest", [B("chest", 0.9, (0, 0.28, 0.25)), B("chest", 0.9, (0, 0.48, 1.25))], 6, [(0.17, 0.15), (0, 0)], "accent_glow", GLOW),
        mesh.path_part("chest", [B("chest", 0.9, (0, 0.5, 0.15)), B("chest", 0.9, (0, 0.95, 0.75))], 5, [(0.11, 0.1), (0, 0)], "accent_glow", GLOW),
        # The head: a block sunk before the boulder, a heavy brow, two azure slits.
        mesh.box_part("head", B("head", 0.3), (0.40, 0.42, 0.36), "cloth", STONE, bevel=0.3),
        mesh.box_part("head", B("head", 0.6, (0, -0.12, 0)), (0.5, 0.22, 0.13), "cloth", STONE_D, rot=(-12, 0, 0)),
        mesh.rock_part("head", B("head", 0.85, (0, 0.03, 0)), (0.42, 0.42, 0.22), STONE_L, moss=MOSS, seed=21, cuts=3, subdiv=1),
    ]
    left = [
        # Crack plates either side of the heart.
        mesh.box_part("chest", B("chest", 0.42, (0.26, -0.34, 0)), (0.36, 0.16, 0.56), "cloth", STONE_L, rot=(0, 0, -10), bevel=0.2),
        mesh.path_part("chest", [B("chest", 0.9, (0.32, 0.3, 0.2)), B("chest", 0.9, (0.66, 0.5, 0.95))], 5, [(0.13, 0.12), (0, 0)], "accent_glow", GLOW),
        mesh.path_part("chest", [B("chest", 0.9, (0.5, 0.15, 0.2)), B("chest", 0.9, (0.78, 0.2, 0.62))], 5, [(0.08, 0.075), (0, 0)], "accent_glow", GLOW),
        mesh.box_part("head", B("head", 0.38, (0.095, -0.215, 0)), (0.1, 0.04, 0.04), "accent_glow", GLOW),
        # Arms: a boulder pauldron crusted with small crystals, the upper arm, a pillar forearm
        # banded with azure runes, a block fist.
        mesh.rock_part(ua, B(ua, 0.12, (0.08, 0, 0.16)), (0.8, 0.78, 0.6), STONE_L, moss=MOSS, seed=5, cuts=4, subdiv=2, rot=(0, -15, 0)),
        mesh.path_part(ua, [B(ua, 0.1, (0.25, -0.05, 0.4)), B(ua, 0.1, (0.42, -0.1, 0.72))], 5, [(0.07, 0.065), (0, 0)], "accent_glow", GLOW),
        mesh.path_part(ua, [B(ua, 0.1, (0.1, 0.18, 0.42)), B(ua, 0.1, (0.18, 0.36, 0.66))], 5, [(0.05, 0.05), (0, 0)], "accent_glow", GLOW),
        mesh.path_part(ua, [B(ua, 0), B(ua, 0.5), B(ua, 1)], 7, [(0.2, 0.2), (0.22, 0.22), (0.19, 0.19)], "cloth", STONE),
        mesh.path_part(fa, [B(fa, -0.05), B(fa, 0.5), B(fa, 1.0)], 6, [(0.2, 0.2), (0.25, 0.24), (0.27, 0.26)], "cloth", STONE),
        mesh.path_part(fa, [B(fa, 0.55), B(fa, 0.68)], 6, [(0.265, 0.255), (0.27, 0.26)], "accent", RUNE),
        mesh.rock_part("hand_l", B("hand_l", 0.5), (0.44, 0.46, 0.48), STONE_D, seed=11, cuts=4, subdiv=2),
        mesh.box_part("fingers_l", B("fingers_l", 0.35), (0.34, 0.32, 0.26), "cloth", STONE_D, bevel=0.2),
        # Stumps for legs, block feet.
        mesh.path_part(th, [B(th, 0), B(th, 1)], 6, [(0.24, 0.24), (0.21, 0.21)], "cloth", STONE_D),
        mesh.path_part(ca, [B(ca, 0), B(ca, 0.6), B(ca, 1)], 6, [(0.22, 0.22), (0.25, 0.25), (0.27, 0.27)], "cloth", STONE),
        mesh.box_part("foot_l", B("foot_l", 0.2, (0, 0, 0.0)), (0.40, 0.52, 0.18), "cloth", STONE_D, bevel=0.3),
        mesh.box_part("toe_l", B("toe_l", 0.3, (0, 0, 0.04)), (0.36, 0.2, 0.14), "cloth", STONE_D, bevel=0.3),
    ]
    out = p + left + [mesh.mirror(b) for b in left]
    # The runestones: a chipped slab each, an azure glyph on its outer face.
    for k, (x, y, z) in RUNES.items():
        bone = f"extra_rune_{k}_1"
        turn = math.degrees(math.atan2(x, -y))
        out.append(mesh.rock_part(bone, (x, y, z + 0.08), (0.24, 0.14, 0.36), STONE_L, seed=31 + len(out), cuts=4, subdiv=1, rot=(0, 0, turn)))
        out.append(mesh.box_part(bone, (x * 1.08, y - 0.06, z + 0.08), (0.09, 0.04, 0.18), "accent_glow", GLOW, rot=(0, 0, turn)))
    return out


# --- Clips ----------------------------------------------------------------------------------

LEGS = merge(sym("thigh", -10, 0, 3), sym("calf", 16, 0, 0), sym("foot", -6, 0, 0))
HANG = arms((0.3, 0.12, -1), (0.12, 0.3, -1))
STAND = merge(LEGS, HANG, {"spine_01": (6, 0, 0), "spine_02": (4, 0, 0), "head": (-8, 0, 0)})
PELVIS = {"pelvis": (0, 0, -0.06)}


def runes(phase, bob=0.04, spin=0.0):
    """The runestones' offsets: each bobs on its own beat; `spin` turns them about the head
    (degrees)."""
    out = {}
    for i, (k, (x, y, z)) in enumerate(RUNES.items()):
        a = math.radians(spin)
        # Orbit about the axis over the head (x, y about (0, 0.18)).
        cx, cy = 0.0, 0.18
        dx, dy = x - cx, y - cy
        nx = cx + dx * math.cos(a) - dy * math.sin(a)
        ny = cy + dx * math.sin(a) + dy * math.cos(a)
        out[f"extra_rune_{k}_1"] = (nx - x, ny - y, bob * math.sin(2 * math.pi * (phase + i / 3)))
    return out


def L(*dicts):
    return merge(*[d for d in dicts if d])


def idle(arm):
    # Slow stone breath; the runes bob on their own beats.
    inhale = merge(STAND, sym("clavicle", 0, -3, 0), {"chest": (-3, 0, 0), "head": (-10, 0, 2)})
    keys = []
    for i, f in enumerate((0, 20, 40, 60)):
        pose = inhale if i % 2 else STAND
        keys.append((f, pose, L(PELVIS if i % 2 == 0 else {"pelvis": (0, 0, -0.04)}, runes(f / 80))))
    keys.append((80, STAND, L(PELVIS, runes(1.0))))
    return _clip(arm, "idle", 80, keys, loop=True)


def idle_fidget_1(arm):
    # Rolls its right shoulder, grinding; the runes spin a full turn and settle.
    roll_a = merge(STAND, arms((0.3, 0.12, -1), (0.12, 0.3, -1), (0.55, -0.3, -0.6), (0.2, 0.3, -1)),
                   {"clavicle_r": (0, 10, 12), "chest": (0, -6, -8), "head": (-10, 0, -10)})
    roll_b = merge(STAND, arms((0.3, 0.12, -1), (0.12, 0.3, -1), (0.6, 0.35, -0.5), (0.2, 0.5, -1)),
                   {"clavicle_r": (0, -8, 16), "chest": (0, 4, -6), "head": (-6, 0, -14)})
    keys = [(0, STAND, L(PELVIS, runes(0))), (18, roll_a, L(PELVIS, runes(0.2, spin=60))),
            (34, roll_b, L({"pelvis": (0, 0, -0.08)}, runes(0.4, spin=200))),
            (50, roll_a, L(PELVIS, runes(0.6, spin=320))), (72, STAND, L(PELVIS, runes(0.9, spin=360)))]
    # Spin keys every ≤ 120°, so the runes go round, not back.
    keys = [(f, p, L(l, runes(f / 72, spin=360 * min(1.0, f / 60)))) for f, p, l in keys]
    return _clip(arm, "idle_fidget_1", 72, keys)


def run(arm):
    # A lumbering stride: heavy, short steps, arms swinging low.
    def upper(p):
        s = math.sin(2 * math.pi * p)
        return merge(arms((0.32, 0.1 + 0.35 * s, -1), (0.12, 0.35 + 0.3 * s, -1), (0.32, 0.1 - 0.35 * s, -1), (0.12, 0.35 - 0.3 * s, -1)),
                     {"head": (-10, 0, 0), "chest": (4, 0, 0)}, runes(p))
    keys = gait(22, 1.25, contact=0.5, lift=0.16, bob=0.08, lean=10.0, twist=7.0, crouch=0.06, upper=upper)
    keys = [(f, p, L(l, runes(f / 22))) for f, p, l in keys]
    return _clip(arm, "run", 22, keys, loop=True, markers={"foot_l": 0, "foot_r": 11}, stride_speed=400.0)


def attack_1(arm):
    # Both fists overhead, cocked further, then a slam into the ground in two frames on `fire`;
    # it holds there, then heaves back up.
    up = merge(LEGS, arms((0.15, 0.2, 1), (-0.25, 0.3, 1)), {"spine_01": (-8, 0, 0), "spine_02": (-10, 0, 0), "chest": (-6, 0, 0), "head": (-14, 0, 0)})
    cock = merge(up, arms((0.15, 0.05, 1), (-0.25, -0.3, 1)), {"spine_02": (-16, 0, 0), "chest": (-10, 0, 0)},
                 sym("thigh", -18, 0, 3), sym("calf", 30, 0, 0))
    slam = merge(arms((0.12, 1, -0.55), (0.0, 0.7, -1)), sym("thigh", -34, 0, 4), sym("calf", 48, 0, 0), sym("foot", -12, 0, 0),
                 {"spine_01": (22, 0, 0), "spine_02": (18, 0, 0), "chest": (10, 0, 0), "head": (6, 0, 0)})
    keys = [
        (0, STAND, L(PELVIS, runes(0)), "ease"),
        (10, up, L({"pelvis": (0, 0.02, 0.0)}, runes(0.15, bob=0.1)), "ease"),
        (FIRE - 2, cock, L({"pelvis": (0, 0.04, -0.06)}, runes(0.25, bob=0.14)), "snap"),
        (FIRE, slam, L({"pelvis": (0, -0.12, -0.22)}, runes(0.3, bob=-0.1)), "hold"),
        (FIRE + 3, slam, L({"pelvis": (0, -0.12, -0.24)}, runes(0.32, bob=-0.12)), "ease"),
        (FIRE + 16, merge(slam, {"spine_01": (16, 0, 0), "head": (0, 0, 0)}), L({"pelvis": (0, -0.1, -0.2)}, runes(0.5)), "settle"),
        (ATTACK, STAND, L(PELVIS, runes(1.0)), "ease"),
    ]
    return _clip(arm, "attack_1", ATTACK, keys, markers={"fire": FIRE})


def attack_2(arm):
    # A backhand: the right fist drawn across the body, then swept out on `fire`, the body
    # turning with it.
    wind = merge(LEGS, arms((0.3, 0.12, -1), (0.12, 0.3, -1), (-0.3, 0.6, 0.1), (-0.8, 0.4, 0.2)),
                 {"spine_01": (4, 0, 22), "spine_02": (2, 0, 14), "head": (-8, 0, 10)})
    sweep = merge(LEGS, arms((0.3, -0.1, -1), (0.12, 0.1, -1), (0.9, 0.5, 0.05), (1.0, 0.25, 0.1)),
                  {"spine_01": (8, 0, -26), "spine_02": (4, 0, -16), "head": (-6, 0, -10)})
    keys = [
        (0, STAND, L(PELVIS, runes(0)), "ease"),
        (FIRE - 3, wind, L({"pelvis": (0, 0.02, -0.08)}, runes(0.2)), "snap"),
        (FIRE, sweep, L({"pelvis": (0, -0.04, -0.1)}, runes(0.3, spin=-20)), "hold"),
        (FIRE + 4, sweep, L({"pelvis": (0, -0.04, -0.1)}, runes(0.35, spin=-25)), "settle"),
        (ATTACK, STAND, L(PELVIS, runes(1.0)), "ease"),
    ]
    return _clip(arm, "attack_2", ATTACK, keys, markers={"fire": FIRE})


def death(arm):
    # It sags to its knees, the runes dropping, then topples forward onto its face; the last
    # frame holds.
    knees = merge(arms((0.4, 0.3, -1), (0.2, 0.5, -1)), sym("thigh", -80, 0, 4), sym("calf", 100, 0, 0), sym("foot", -20, 0, 0),
                  {"spine_01": (14, 0, 0), "spine_02": (8, 0, 0), "head": (16, 0, 0)})
    face = merge(arms((0.6, 0.6, -0.2), (0.5, 0.8, 0.0)), sym("thigh", -30, 0, 4), sym("calf", 40, 0, 0),
                 {"pelvis": (70, 0, 0), "spine_01": (8, 0, 0), "head": (-10, 0, 8)})
    drop = {f"extra_rune_{k}_1": (0, 0, -1.6) for k in RUNES}
    keys = [
        (0, STAND, L(PELVIS, runes(0)), "ease"),
        (6, merge(STAND, {"spine_01": (-8, 0, 0), "head": (-20, 0, 0)}), L({"pelvis": (0, 0.06, -0.04)}, runes(0.1, bob=0.15)), "whip"),
        (16, knees, L({"pelvis": (0, -0.05, -0.55)}, {k: (x, y, -0.6) for k, (x, y, _) in runes(0.2).items()}), "hold"),
        (22, knees, L({"pelvis": (0, -0.05, -0.56)}, {k: (x, y, -0.9) for k, (x, y, _) in runes(0.2).items()}), "whip"),
        (36, face, L({"pelvis": (0, -0.75, -0.75)}, drop), "hold"),
    ]
    return _clip(arm, "death", 36, keys)


def flinch(arm):
    back = {"spine_02": (-6, 0, 0), "chest": (-5, 0, 0), "head": (-8, 0, 0)}
    return _clip(arm, "flinch", 10, [(0, {}, None), (3, back, None), (10, {}, None)], layer="additive",
                 bones=["spine_02", "chest", "head"])


def build(pid):
    rig.use_shape(SHAPE)
    mesh.AO_HEIGHT = 2.4
    arm = rig.build_armature(name="warden")
    for k, (x, y, z) in RUNES.items():
        rig.add_chain(arm, f"rune_{k}", "chest", [(x, y, z), (x, y, z + 0.16)])
    made = [idle(arm), idle_fidget_1(arm), run(arm), attack_1(arm), attack_2(arm), death(arm), flinch(arm)]
    obj = mesh.build_mannequin(arm, name="warden", part_list=body())
    print(f"monster warden: {len(arm.data.bones)} bones, {mesh.triangle_count(obj)} triangles, clips {[a.name for a in made]}")
