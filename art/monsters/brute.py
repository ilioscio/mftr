# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Ember Brute (16 §5): the red buff's guardian.

A boar-ape of charred bark: a hunched back of smoldering thorn stumps, a low tusked head,
forearms longer than its legs, knuckles on the ground; magma glows in every crack. The
`accent_glow` slot takes the camp's ember red (the client paints it), `accent` its hot bark.

On the `biped` bones: very broad, short legs, long arms; the hunch is in the poses (the spine
pitched forward, the head low and ahead). Clips: idle, idle_fidget_1 (rears up and beats its
chest), run (knuckle gallop), attack_1 (a two-fisted pound), attack_2 (a haymaker), death,
flinch. `fire` = 0.3 / 0.5 attacks a second = 18 frames.
"""

import math

from mftr_blender import mesh, rig
from mftr_blender.clips import merge, sym
from mftr_blender.library import _clip, arms, gait

BARK = (0.026, 0.018, 0.014)
BARK_L = (0.045, 0.03, 0.022)
CHAR = (0.012, 0.01, 0.009)
ASH = (0.11, 0.1, 0.095)
TUSK = (0.42, 0.36, 0.26)
HOT = (0.8, 0.8, 0.8)      # accent: bark glowing from within (shade only)
GLOW = (1.0, 1.0, 1.0)     # accent_glow: magma

SHAPE = {
    "heights": [(0.0, 0.0), (0.03, 0.03), (0.09, 0.11), (0.54, 0.42), (1.00, 0.80), (1.12, 0.98),
                (1.26, 1.20), (1.40, 1.44), (1.56, 1.72), (1.64, 1.78), (1.88, 2.02), (2.15, 2.4)],
    "x": 1.9, "y": 1.5, "scale": 1.15, "shoulder": 0.14, "arm": 1.55, "hand": 2.2,
}
ATTACK = 60
FIRE = 18


def B(bone, t, off=(0.0, 0.0, 0.0)):
    return (bone, t, off)


def body():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    p = [
        # A barrel of bark, magma in its cracks.
        mesh.rock_part("pelvis", B("pelvis", 0.3), (0.8, 0.62, 0.5), BARK, seed=2, cuts=3, subdiv=2),
        mesh.rock_part("spine_01", B("spine_01", 0.5), (0.98, 0.74, 0.5), BARK, seed=4, cuts=3, subdiv=2),
        mesh.path_part("spine_02", [B("spine_02", -0.1, (0, -0.25, 0)), B("spine_02", 0.6, (0, -0.4, 0))], 5, [(0.18, 0.06), (0.22, 0.07)],
                       "accent_glow", GLOW),
        mesh.rock_part("spine_02", B("spine_02", 0.5), (1.2, 0.86, 0.56), BARK_L, seed=6, cuts=4, subdiv=2),
        # The great hunched back: bark slabs, smoldering thorn stumps up its ridge.
        mesh.rock_part("chest", B("chest", 0.5, (0, 0.06, 0)), (1.5, 1.02, 0.78), BARK, moss=CHAR, seed=9, cuts=5, subdiv=3),
        mesh.path_part("chest", [B("chest", 0.1, (0, -0.42, 0)), B("chest", 0.55, (0, -0.52, 0)), B("chest", 0.95, (0, -0.4, 0))], 6,
                       [(0, 0), (0.26, 0.08), (0, 0)], "accent_glow", GLOW),
        # Magma cracks across the back, where the camera looks.
        mesh.path_part("chest", [B("chest", 0.05, (0, 0.38, 0.3)), B("chest", 0.5, (0.05, 0.42, 0.42)), B("chest", 0.95, (-0.04, 0.3, 0.42))], 5,
                       [(0.06, 0.03), (0.07, 0.035), (0.04, 0.02)], "accent_glow", GLOW),
        # The head: low and ahead, a boar's snout, tusks curling up, ember eyes.
        mesh.rock_part("head", B("head", 0.35, (0, -0.06, 0)), (0.5, 0.56, 0.42), BARK_L, seed=13, cuts=3, subdiv=2),
        mesh.box_part("head", B("head", 0.2, (0, -0.32, -0.04)), (0.3, 0.3, 0.22), "cloth", BARK, rot=(-12, 0, 0), taper=0.8, bevel=0.3),
        mesh.box_part("head", B("head", 0.2, (0, -0.48, -0.02)), (0.22, 0.04, 0.14), "cloth", CHAR, bevel=0.3),
        mesh.path_part("head", [B("head", 0.1, (0, -0.4, -0.13)), B("head", 0.1, (0, -0.47, -0.13))], 6, [(0.12, 0.05), (0.1, 0.04)],
                       "accent_glow", GLOW),
    ]
    left = [
        mesh.path_part("chest", [B("chest", 0.3, (0.12, 0.4, 0.36)), B("chest", 0.55, (0.42, 0.36, 0.34)), B("chest", 0.75, (0.62, 0.2, 0.26))], 5,
                       [(0.05, 0.025), (0.06, 0.03), (0.035, 0.02)], "accent_glow", GLOW),
        mesh.box_part("head", B("head", 0.45, (0.13, -0.29, 0.02)), (0.1, 0.05, 0.05), "accent_glow", GLOW, rot=(0, 0, 18)),
        mesh.path_part("head", [B("head", 0.1, (0.13, -0.38, -0.08)), B("head", 0.2, (0.2, -0.5, 0.04)), B("head", 0.5, (0.18, -0.46, 0.2))], 5,
                       [(0.045, 0.045), (0.035, 0.035), (0, 0)], "skin", TUSK),
        # Thorn stumps up the back, each smoldering at its tip.
        mesh.path_part("chest", [B("chest", 0.6, (0.25, 0.25, 0.25)), B("chest", 0.6, (0.38, 0.45, 0.62))], 5, [(0.09, 0.09), (0.05, 0.05)],
                       "cloth", BARK_L),
        mesh.path_part("chest", [B("chest", 0.6, (0.38, 0.45, 0.62)), B("chest", 0.6, (0.42, 0.5, 0.74))], 5, [(0.05, 0.05), (0, 0)],
                       "accent_glow", GLOW),
        mesh.path_part("chest", [B("chest", 0.2, (0.42, 0.3, 0.1)), B("chest", 0.2, (0.66, 0.42, 0.4))], 5, [(0.08, 0.08), (0.045, 0.045)],
                       "cloth", BARK_L),
        mesh.path_part("chest", [B("chest", 0.2, (0.66, 0.42, 0.4)), B("chest", 0.2, (0.7, 0.46, 0.5))], 5, [(0.045, 0.045), (0, 0)],
                       "accent_glow", GLOW),
        mesh.path_part("chest", [B("chest", 0.95, (0.12, 0.15, 0.25)), B("chest", 0.95, (0.16, 0.28, 0.66))], 5, [(0.1, 0.1), (0.05, 0.05)],
                       "cloth", BARK),
        mesh.path_part("chest", [B("chest", 0.95, (0.16, 0.28, 0.66)), B("chest", 0.95, (0.17, 0.3, 0.8))], 5, [(0.05, 0.05), (0, 0)],
                       "accent_glow", GLOW),
        # Arms: a bark boulder of a shoulder, a long forearm, a fist like a stump; magma along
        # the forearm's crack.
        mesh.rock_part(ua, B(ua, 0.15, (0.04, 0, 0.1)), (0.72, 0.72, 0.62), BARK, moss=CHAR, seed=15, cuts=4, subdiv=2, rot=(0, -20, 0)),
        mesh.path_part(ua, [B(ua, 0), B(ua, 0.5), B(ua, 1)], 7, [(0.2, 0.2), (0.22, 0.21), (0.17, 0.17)], "cloth", BARK),
        mesh.path_part(fa, [B(fa, -0.05), B(fa, 0.5), B(fa, 1.0)], 7, [(0.18, 0.18), (0.23, 0.22), (0.25, 0.24)], "cloth", BARK_L),
        mesh.path_part(fa, [B(fa, 0.15, (0, -0.2, 0)), B(fa, 0.85, (0, -0.24, 0))], 5, [(0.04, 0.03), (0.035, 0.03)], "accent_glow", GLOW),
        mesh.path_part(fa, [B(fa, 0.25), B(fa, 0.4)], 7, [(0.215, 0.205), (0.225, 0.215)], "accent", HOT),
        mesh.rock_part("hand_l", B("hand_l", 0.6), (0.46, 0.44, 0.48), CHAR, seed=17, cuts=4, subdiv=2),
        mesh.box_part("fingers_l", B("fingers_l", 0.4), (0.36, 0.3, 0.26), "cloth", BARK, bevel=0.3),
        # Short, thick legs and hooves.
        mesh.path_part(th, [B(th, 0), B(th, 1)], 7, [(0.25, 0.25), (0.2, 0.2)], "cloth", BARK),
        mesh.path_part(ca, [B(ca, 0), B(ca, 1)], 6, [(0.19, 0.19), (0.16, 0.16)], "cloth", BARK_L),
        mesh.box_part("foot_l", B("foot_l", 0.25), (0.3, 0.42, 0.16), "cloth", CHAR, bevel=0.35),
        mesh.box_part("toe_l", B("toe_l", 0.3, (0, 0, 0.04)), (0.26, 0.16, 0.12), "cloth", CHAR, bevel=0.3),
    ]
    return p + left + [mesh.mirror(b) for b in left]


# --- Clips ----------------------------------------------------------------------------------

# The hunch: the spine pitched forward, the head lifted back to look ahead, knuckles down.
HUNCH = {"spine_01": (28, 0, 0), "spine_02": (16, 0, 0), "chest": (8, 0, 0), "neck": (-22, 0, 0), "head": (-26, 0, 0)}
LEGS = merge(sym("thigh", -26, 0, 4), sym("calf", 36, 0, 0), sym("foot", -10, 0, 0))
KNUCKLES = arms((0.32, 0.25, -1), (0.08, 0.18, -1))
STAND = merge(HUNCH, LEGS, KNUCKLES)
PELVIS = {"pelvis": (0, 0.05, -0.12)}


def idle(arm):
    # Heavy breaths through the crack in its chest; knuckles shifting.
    breathe = merge(STAND, {"chest": (4, 0, 0), "head": (-30, 0, 4)}, sym("clavicle", 0, -4, 0),
                    arms((0.33, 0.22, -1), (0.08, 0.14, -1)))
    keys = [(0, STAND, PELVIS), (30, breathe, {"pelvis": (0, 0.05, -0.1)}), (45, merge(STAND, {"head": (-24, 0, -6)}), PELVIS)]
    return _clip(arm, "idle", 60, keys, loop=True)


def idle_fidget_1(arm):
    # Rears up onto its legs, beats its chest twice (holds on each blow), drops back down.
    up = merge(LEGS, sym("thigh", -8, 0, 4), sym("calf", 12, 0, 0), {"spine_01": (-8, 0, 0), "spine_02": (-6, 0, 0), "head": (-14, 0, 0)},
               arms((0.6, 0.0, 0.2), (0.3, 0.7, 0.4)))
    beat_l = merge(up, arms((0.35, 0.5, -0.2), (-0.6, 0.6, 0.2), (0.6, 0.0, 0.2), (0.3, 0.7, 0.4)))
    beat_r = merge(up, arms((0.6, 0.0, 0.2), (0.3, 0.7, 0.4), (0.35, 0.5, -0.2), (-0.6, 0.6, 0.2)))
    keys = [(0, STAND, PELVIS, "ease"), (14, up, {"pelvis": (0, 0, 0.1)}, "whip"), (20, beat_l, {"pelvis": (0, 0, 0.1)}, "hold"),
            (24, up, {"pelvis": (0, 0, 0.1)}, "whip"), (30, beat_r, {"pelvis": (0, 0, 0.1)}, "hold"), (34, up, {"pelvis": (0, 0, 0.1)}, "ease"),
            (40, merge(up, {"head": (-30, 0, 0)}), {"pelvis": (0, 0, 0.12)}, "settle"), (60, STAND, PELVIS, "ease")]
    return _clip(arm, "idle_fidget_1", 60, keys)


def run(arm):
    # A knuckle gallop: the legs drive, the arms vault forward on the off-beat.
    def upper(p):
        s = math.sin(2 * math.pi * p)
        return merge(HUNCH, {"spine_01": (32 + 6 * math.sin(4 * math.pi * p), 0, 0)},
                     arms((0.32, 0.25 + 0.55 * s, -1), (0.08, 0.3 + 0.5 * s, -1), (0.32, 0.25 - 0.55 * s, -1), (0.08, 0.3 - 0.5 * s, -1)))
    keys = gait(20, 1.0, contact=0.48, lift=0.18, bob=0.08, lean=0.0, twist=10.0, crouch=0.14, upper=upper)
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=400.0)


def attack_1(arm):
    # Rears up with both fists high, cocks back, and pounds the ground with both in two frames;
    # it holds, shoulders heaving, then drops back onto its knuckles.
    rear = merge(LEGS, sym("thigh", -10, 0, 4), sym("calf", 14, 0, 0), {"spine_01": (-6, 0, 0), "spine_02": (-8, 0, 0), "neck": (-6, 0, 0), "head": (-10, 0, 0)},
                 arms((0.25, 0.2, 1), (-0.15, 0.25, 1)))
    cock = merge(rear, {"spine_01": (-14, 0, 0), "spine_02": (-12, 0, 0)}, arms((0.25, -0.1, 1), (-0.15, -0.3, 1)))
    pound = merge(LEGS, sym("thigh", -40, 0, 4), sym("calf", 52, 0, 0),
                  {"spine_01": (44, 0, 0), "spine_02": (22, 0, 0), "chest": (10, 0, 0), "neck": (-20, 0, 0), "head": (-18, 0, 0)},
                  arms((0.2, 1, -0.4), (0.0, 0.6, -1)))
    keys = [
        (0, STAND, PELVIS, "ease"),
        (11, rear, {"pelvis": (0, 0.06, 0.14)}, "ease"),
        (FIRE - 2, cock, {"pelvis": (0, 0.1, 0.16)}, "snap"),
        (FIRE, pound, {"pelvis": (0, -0.15, -0.22)}, "hold"),
        (FIRE + 4, pound, {"pelvis": (0, -0.15, -0.24)}, "ease"),
        (FIRE + 18, merge(pound, {"spine_01": (36, 0, 0), "head": (-26, 0, 0)}), {"pelvis": (0, -0.08, -0.18)}, "settle"),
        (ATTACK, STAND, PELVIS, "ease"),
    ]
    return _clip(arm, "attack_1", ATTACK, keys, markers={"fire": FIRE})


def attack_2(arm):
    # A haymaker: the right arm swung wide and back, then across in two frames, the whole body
    # twisting through.
    wind = merge(STAND, {"spine_01": (24, 0, 26), "spine_02": (12, 0, 14), "head": (-26, 0, 12)},
                 arms((0.32, 0.25, -1), (0.08, 0.18, -1), (0.9, -0.6, 0.3), (0.6, -0.4, 0.6)))
    swing = merge(STAND, {"spine_01": (30, 0, -30), "spine_02": (16, 0, -18), "head": (-24, 0, -10)},
                  arms((0.32, 0.0, -1), (0.08, 0.0, -1), (-0.6, 1, 0.1), (-0.7, 0.6, 0.1)))
    keys = [
        (0, STAND, PELVIS, "ease"),
        (FIRE - 3, wind, {"pelvis": (0, 0.06, -0.1)}, "snap"),
        (FIRE, swing, {"pelvis": (0, -0.08, -0.14)}, "hold"),
        (FIRE + 4, swing, {"pelvis": (0, -0.08, -0.14)}, "settle"),
        (ATTACK, STAND, PELVIS, "ease"),
    ]
    return _clip(arm, "attack_2", ATTACK, keys, markers={"fire": FIRE})


def death(arm):
    # A last bellow, then it keels over onto its side; the last frame holds.
    bellow = merge(STAND, {"spine_01": (6, 0, 0), "neck": (-30, 0, 0), "head": (-30, 0, 0)}, arms((0.6, 0.1, -0.6), (0.4, 0.3, -0.5)))
    side = merge(arms((0.9, 0.2, -0.3), (0.8, 0.4, -0.2), (0.4, 0.4, -1), (0.2, 0.6, -1)), sym("thigh", -40, 0, 4), sym("calf", 50, 0, 0),
                 {"pelvis": (0, -80, 0), "spine_01": (16, 0, 0), "head": (-10, 0, 10)})
    keys = [
        (0, STAND, PELVIS, "ease"),
        (8, bellow, {"pelvis": (0, 0.04, -0.05)}, "hold"),
        (14, bellow, {"pelvis": (0, 0.04, -0.06)}, "whip"),
        (30, side, {"pelvis": (0.35, 0.1, -0.5)}, "hold"),
    ]
    return _clip(arm, "death", 30, keys)


def flinch(arm):
    back = {"spine_02": (-8, 0, 0), "chest": (-6, 0, 0), "head": (-10, 0, 0)}
    return _clip(arm, "flinch", 10, [(0, {}, None), (3, back, None), (10, {}, None)], layer="additive",
                 bones=["spine_02", "chest", "head"])


def build(pid):
    rig.use_shape(SHAPE)
    mesh.AO_HEIGHT = 2.0
    arm = rig.build_armature(name="brute")
    made = [idle(arm), idle_fidget_1(arm), run(arm), attack_1(arm), attack_2(arm), death(arm), flinch(arm)]
    obj = mesh.build_mannequin(arm, name="brute", part_list=body())
    print(f"monster brute: {len(arm.data.bones)} bones, {mesh.triangle_count(obj)} triangles, clips {[a.name for a in made]}")
