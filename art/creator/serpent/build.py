# SPDX-License-Identifier: AGPL-3.0-or-later
"""The serpentoid build's base (CR10, 20 §3): no legs, a long spine chain; the front of the
body rears up (a cobra's stance) and the rest coils behind on the ground. It slithers. Its own
clips are its move library (`mftr_blender.builds`).

    blender -b --python art/creator/serpent/build.py -- [--export]
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
from mftr_blender.library import _clip, dense  # noqa: E402

NAME = "serpent"
S = 1.3

SCALE = (0.30, 0.42, 0.26)
SCALE_D = (0.17, 0.24, 0.15)
BELLY = (0.68, 0.62, 0.42)
FANG = (0.88, 0.85, 0.78)
ACCENT = (0.18, 0.52, 0.95)
GLOW = (1.0, 1.0, 1.0)
IRON = (0.42, 0.43, 0.47)
GOLD = (0.66, 0.50, 0.20)
HORN = (0.62, 0.56, 0.44)

CENTER = [
    bone("root", None, (0, 0, 0), (0, -0.3, 0), deform=False),
    bone("pelvis", "root", (0, 0.2, 0.18), (0, -0.1, 0.2)),
    bone("spine", "pelvis", (0, -0.1, 0.2), (0, -0.3, 0.45)),
    bone("chest", "spine", (0, -0.3, 0.45), (0, -0.38, 0.85)),
    bone("neck", "chest", (0, -0.38, 0.85), (0, -0.42, 1.12)),
    bone("head", "neck", (0, -0.42, 1.12), (0, -0.72, 1.14)),
    bone("jaw", "head", (0, -0.46, 1.08), (0, -0.7, 1.04)),
    bone("tail_1", "pelvis", (0, 0.2, 0.18), (0.16, 0.45, 0.15)),
    bone("tail_2", "tail_1", (0.16, 0.45, 0.15), (0.02, 0.7, 0.12)),
    bone("tail_3", "tail_2", (0.02, 0.7, 0.12), (-0.18, 0.86, 0.1)),
    bone("tail_4", "tail_3", (-0.18, 0.86, 0.1), (-0.1, 1.06, 0.08)),
    bone("tail_5", "tail_4", (-0.1, 1.06, 0.08), (0.08, 1.2, 0.06)),
]
LEFT = [bone("hood_l", "neck", (0.04, -0.4, 0.92), (0.2, -0.36, 1.02))]
SOCKETS = [
    socket("socket_chest", "chest", (0, -0.42, 0.7)),
    socket("socket_overhead", "root", (0, 0, 1.6)),
    socket("socket_projectile", "jaw", (0, -0.74, 1.08)),
    socket("socket_cast", "jaw", (0, -0.74, 1.08)),
    socket("socket_weapon_tip", "jaw", (0, -0.74, 1.08)),
]
CHAIN = ["pelvis", "spine", "chest", "neck", "tail_1", "tail_2", "tail_3", "tail_4", "tail_5"]
TAIL = ["tail_1", "tail_2", "tail_3", "tail_4", "tail_5"]


def B(name, t, off=(0.0, 0.0, 0.0)):
    return (name, t, tuple(x * S for x in off))


def r(*v):
    return tuple(x * S for x in v)


def body():
    radii = {"pelvis": (0.15, 0.16), "spine": (0.14, 0.15), "chest": (0.13, 0.14), "neck": (0.1, 0.11),
             "tail_1": (0.14, 0.13), "tail_2": (0.12, 0.11), "tail_3": (0.1, 0.09), "tail_4": (0.07, 0.06), "tail_5": (0.04, 0.0)}
    p = []
    for bn, (a, b) in radii.items():
        p.append(mesh.path_part(bn, [B(bn, -0.06), B(bn, 0.5), B(bn, 1.06 if b else 1.0)], 12,
                                [r(a, a * 0.85), r((a + b) / 2, (a + b) / 2 * 0.85), r(b, b * 0.85)], "cloth", SCALE if bn not in TAIL[1::2] else SCALE_D))
    # The belly plates up the rearing front, the scales' dark ridge down the back.
    p.append(mesh.path_part("chest", [B("chest", 0.0, (0, -0.1, 0)), B("chest", 1.0, (0, -0.075, 0))], 8, [r(0.09, 0.03), r(0.07, 0.03)], "cloth", BELLY))
    p.append(mesh.path_part("neck", [B("neck", 0.0, (0, -0.075, 0)), B("neck", 1.0, (0, -0.055, 0))], 8, [r(0.07, 0.03), r(0.05, 0.02)], "cloth", BELLY))
    # The head: a broad wedge, the jaw, fangs, the eyes in the signature hue.
    p += [
        mesh.path_part("head", [B("head", -0.05), B("head", 0.4, (0, 0, 0.01)), B("head", 1.0, (0, 0, -0.01))], 10,
                       [r(0.1, 0.07), r(0.11, 0.07), r(0.05, 0.035)], "cloth", SCALE),
        mesh.path_part("jaw", [B("jaw", 0.0), B("jaw", 1.0)], 8, [r(0.07, 0.03), r(0.035, 0.02)], "cloth", BELLY),
        # The team accent (05 §1.3): a band round the rearing chest.
        mesh.path_part("chest", [B("chest", 0.45), B("chest", 0.6)], 12, [r(0.142, 0.125), r(0.14, 0.123)], "accent", ACCENT),
    ]
    left = [
        mesh.box_part("head", B("head", 0.3, (0.07, 0, 0.04)), r(0.04, 0.03, 0.025), "emissive", GLOW, rot=(0, 0, 20)),
        mesh.path_part("jaw", [B("jaw", 0.75, (0.025, 0, 0.01)), B("jaw", 0.8, (0.025, 0, 0.06))], 4, [r(0.012, 0.01), (0, 0)], "metal", FANG),
        # The hood: a flap of scales flaring from the neck.
        mesh.path_part("hood_l", [B("hood_l", -0.1), B("hood_l", 0.6, (0, 0.02, 0.04)), B("hood_l", 1.0)], 6,
                       [r(0.1, 0.02), r(0.09, 0.02), (0, 0)], "cloth", SCALE_D),
    ]
    # The art pass: rows of raised scales down the back, a keel of them on each flank.
    for bn in ["chest", "spine", "pelvis", "tail_1", "tail_2", "tail_3", "tail_4"]:
        a, _ = radii[bn]
        for t in (0.2, 0.55, 0.9):
            p.append(mesh.path_part(bn, [B(bn, t, (0, 0.0, 0.0)), B(bn, t, (0, a * 0.95, 0.0)), B(bn, t + 0.12, (0, a * 1.12, 0.0))], 6,
                                    [r(0.03, 0.018), r(0.022, 0.012), (0, 0)], "cloth", SCALE_D))
            left.append(mesh.path_part(bn, [B(bn, t + 0.06, (a * 0.7, a * 0.55, 0.0)), B(bn, t + 0.14, (a * 0.85, a * 0.7, 0.0))], 6,
                                       [r(0.02, 0.012), (0, 0)], "cloth", SCALE_D))
    return p + left + [mesh.mirror(b) for b in left]


def sliders():
    G = creator

    def centre(r_):
        return (r_.lo + r_.hi) / 2

    return [
        ("head_size", ("head", "jaw"), lambda r_: G.scale(r_, centre(r_), (1.2, 1.2, 1.2)), None),
        ("head_hood", ("hood_l", "hood_r"), lambda r_: {i: Vector((r_.cos[i].x * 0.5, 0, 0.03 * S)) for i in r_.idx}, None),
        ("jaw_heavy", ("jaw",), lambda r_: G.scale(r_, Vector((0, r_.hi.y, r_.hi.z)), (1.35, 1.0, 1.35)), None),
        ("body_bulk", tuple(CHAIN), lambda r_: creator._limb_scale(r_, 1.25), None),
        ("body_tail", tuple(TAIL), lambda r_: creator._limb_scale(r_, 1.4), None),
        ("body_neck", ("neck",), lambda r_: creator._limb_scale(r_, 1.35), None),
    ]


def catalog():
    out = []

    def add(pid, slot, name, items, hides=(), accent=False):
        out.append((pid, slot, name, list(hides), accent, items))

    horn = mesh.path_part("head", [B("head", 0.15, (0.05, 0, 0.05)), B("head", -0.05, (0.1, 0.06, 0.12)), B("head", -0.25, (0.12, 0.14, 0.14))], 6,
                          [r(0.025, 0.025), r(0.018, 0.018), (0, 0)], "cloth", HORN)
    add("head.horns", "head", "Horns", [horn, mesh.mirror(horn)])
    add("head.crown", "head", "Crown", [
        mesh.path_part("head", [B("head", 0.15, (0, 0, 0.065)), B("head", 0.5, (0, 0, 0.06))], 10, [r(0.09, 0.07), r(0.08, 0.06)], "metal", GOLD),
        mesh.box_part("head", B("head", 0.3, (0, 0, 0.11)), r(0.03, 0.03, 0.04), "emissive", GLOW, rot=(0, 45, 0))])
    add("shoulders.collar", "shoulders", "Collar", [
        mesh.path_part("neck", [B("neck", 0.15), B("neck", 0.35)], 12, [r(0.12, 0.11), r(0.115, 0.105)], "metal", IRON),
        mesh.path_part("neck", [B("neck", 0.22), B("neck", 0.28)], 12, [r(0.125, 0.115), r(0.125, 0.115)], "accent", ACCENT)], accent=True)
    fins = []
    for bn, t in (("chest", 0.3), ("chest", 0.8), ("spine", 0.5), ("pelvis", 0.5), ("tail_1", 0.5), ("tail_2", 0.5)):
        fins.append(mesh.path_part(bn, [B(bn, t, (0, 0.1, 0.0)), B(bn, t, (0, 0.24, 0.08))], 4, [r(0.03, 0.012), (0, 0)], "cloth", SCALE_D))
    add("back.spines", "back", "Dorsal spines", fins)
    plates = [mesh.box_part(bn, B(bn, t, (0, -0.12, 0)), r(0.16, 0.04, 0.12), "metal", IRON, bevel=0.3)
              for bn, t in (("chest", 0.25), ("chest", 0.7), ("spine", 0.5))]
    add("torso.plates", "torso", "Belly plates", plates)
    # A blade bound to the tail's tip: the serpent's weapon, swung by its whole length.
    add("weapon.tail_blade", "weapon", "Tail blade", [
        mesh.path_part("tail_5", [B("tail_5", 0.3), B("tail_5", 1.0, (0, 0, 0.05)), B("tail_5", 1.5, (0, -0.04, 0.18))], 4,
                       [r(0.055, 0.012), r(0.05, 0.01), (0, 0)], "metal", IRON),
        mesh.path_part("tail_5", [B("tail_5", 0.2), B("tail_5", 0.45)], 8, [r(0.05, 0.045), r(0.05, 0.045)], "metal", GOLD)])
    # Gold rings down the body, a temple serpent's jewelry.
    bands = [mesh.path_part(bn, [B(bn, t - 0.05), B(bn, t + 0.05)], 12, [r(rad, rad * 0.88), r(rad, rad * 0.88)], "metal", GOLD)
             for bn, t, rad in (("chest", 0.2, 0.15), ("spine", 0.5, 0.165), ("tail_1", 0.5, 0.155), ("tail_2", 0.5, 0.13), ("tail_3", 0.5, 0.11))]
    add("torso.gold_bands", "torso", "Gold bands", bands)
    jewel = mesh.box_part("hood_l", B("hood_l", 0.45, (0, -0.03, 0.01)), r(0.035, 0.02, 0.035), "emissive", GLOW, rot=(0, 45, 0))
    add("face.hood_jewels", "face", "Hood jewels", [jewel, mesh.mirror(jewel)])
    crest = [mesh.path_part("head", [B("head", t, (0, 0.0, 0.06)), B("head", t - 0.25, (0, 0.06, 0.06 + h))], 4, [r(0.05, 0.01), (0, 0)], "accent", ACCENT)
             for t, h in ((0.45, 0.1), (0.25, 0.16), (0.05, 0.12))]
    add("head.crest", "head", "Crest", crest, accent=True)
    return out


# --- Clips -----------------------------------------------------------------------------------

STAND = {"neck": (-4, 0, 0), "head": (6, 0, 0)}


def L(z=0.0, y=0.0):
    return {"pelvis": (0, y * S, z * S)}


def wave(phase, amp=14.0):
    """The chain swung side to side in a travelling wave."""
    out = {}
    for i, bn in enumerate(["chest", "spine", "pelvis"] + ["tail_1", "tail_2", "tail_3", "tail_4", "tail_5"]):
        out[bn] = (0, 0, amp * math.sin(2 * math.pi * (phase - i * 0.13)))
    out["neck"] = (-4, 0, -amp * 0.4 * math.sin(2 * math.pi * phase))
    out["head"] = (6, 0, amp * 0.3 * math.sin(2 * math.pi * phase))
    return out


def slither(arm, name, frames, speed, amp):
    keys = [(f, wave(f / frames, amp), None) for f in range(0, frames, 2)]
    return _clip(arm, name, frames, keys, loop=True, markers={"foot_l": 0, "foot_r": frames // 2}, stride_speed=speed)


def clips(arm):
    made = [_clip(arm, "idle", 60, [(f, merge(wave(f / 60, 5), {"jaw": (3 * (f % 20 < 4), 0, 0)}), None) for f in range(0, 60, 4)], loop=True)]
    made.append(_clip(arm, "idle_fidget_1", 48, [(0, STAND, None), (10, merge(STAND, {"neck": (-10, 0, 0), "jaw": (30, 0, 0)}), None, "hold"),
                                                  (24, merge(STAND, {"neck": (-10, 0, 0), "jaw": (30, 0, 0)}), None), (48, STAND, None)]))
    made.append(slither(arm, "walk", 32, 150.0, 10))
    made.append(slither(arm, "run", 20, 340.0, 18))
    reared = {"chest": (-14, 0, 0), "neck": (-20, 0, 0), "head": (10, 0, 0), "jaw": (6, 0, 0)}
    strike = {"chest": (24, 0, 0), "neck": (24, 0, 0), "head": (-6, 0, 0), "jaw": (40, 0, 0)}
    bite = merge(strike, {"jaw": (0, 0, 0)})
    made.append(_clip(arm, "attack_1", 36, [(0, STAND, None, "ease"), (8, reared, L(0, 0.05), "snap"), (11, strike, L(0, -0.12), "snap"),
                                            (13, bite, L(0, -0.14), "hold"), (36, STAND, None, "ease")], markers={"fire": 13}))
    whip_a = {"tail_3": (0, 0, -40), "tail_4": (0, 0, -40), "tail_5": (0, 0, -30), "chest": (0, 0, 20)}
    whip_b = {"tail_3": (0, 0, 50), "tail_4": (0, 0, 50), "tail_5": (0, 0, 40), "chest": (0, 0, -25)}
    made.append(_clip(arm, "attack_2", 36, [(0, STAND, None, "ease"), (9, whip_a, None, "snap"), (13, whip_b, None, "hold"),
                                            (36, STAND, None, "ease")], markers={"fire": 13}))
    hooded = merge(reared, {"hood_l": (0, -30, 0), "hood_r": (0, 30, 0)})
    spit = merge(hooded, {"neck": (10, 0, 0), "head": (-10, 0, 0), "jaw": (36, 0, 0)})
    made.append(_clip(arm, "howl", 34, [(0, STAND, None, "ease"), (8, hooded, L(0, 0.06), "snap"), (11, spit, L(0, -0.04), "hold"),
                                        (16, spit, L(0, -0.04), "settle"), (34, STAND, None, "ease")], markers={"fire": 11}))
    high = {"pelvis": (-10, 0, 0), "spine": (-20, 0, 0), "chest": (-18, 0, 0), "neck": (-10, 0, 0), "jaw": (20, 0, 0)}
    down = {"spine": (14, 0, 0), "chest": (30, 0, 0), "neck": (30, 0, 0), "head": (10, 0, 0)}
    made.append(_clip(arm, "slam", 42, [(0, STAND, None, "ease"), (12, high, L(0.12), "hold"), (15, high, L(0.14), "whip"),
                                        (18, down, L(-0.06, -0.08), "hold"), (26, down, L(-0.06, -0.08), "settle"), (42, STAND, None, "ease")],
                      markers={"fire": 18}))
    coil = merge(wave(0.25, 22), {"neck": (-24, 0, 0), "head": (14, 0, 0)})
    made.append(_clip(arm, "pounce", 40, [(0, STAND, None, "ease"), (7, coil, L(0, 0.1), "snap"), (11, strike, L(0.04, -0.3), "snap"),
                                          (14, bite, L(0, -0.32), "hold"), (40, STAND, None, "ease")], markers={"fire": 14}))
    made.append(_clip(arm, "dash_start", 8, [(0, STAND, None, "ease"), (8, coil, L(0, 0.06), "snap")], markers={"fire": 7}))
    made.append(slither(arm, "dash_travel", 12, 900.0, 22))
    made.append(_clip(arm, "dash_land", 12, [(0, coil, None, "hold"), (12, STAND, None, "ease")]))
    limp = {"chest": (50, 0, 0), "neck": (40, 0, 10), "head": (20, 0, 0), "jaw": (20, 0, 0), "spine": (20, 0, 0)}
    made.append(_clip(arm, "death", 24, [(0, STAND, None, "ease"), (6, merge(reared, {"jaw": (36, 0, 0)}), None, "hold"),
                                         (24, limp, L(-0.08), "hold")]))
    back = {"neck": (-10, 0, 0), "head": (-8, 0, 0), "chest": (-4, 0, 0)}
    made.append(_clip(arm, "flinch", 8, [(0, {}, None), (2, back, None), (8, {}, None)], layer="additive", bones=["neck", "head", "chest"]))
    made.append(_clip(arm, "cc_stunned", 24, [(f, merge(STAND, {"neck": (10, 0, 16 * math.sin(2 * math.pi * f / 24)),
                                                                 "head": (12, 0, -20 * math.sin(2 * math.pi * f / 24))}), None)
                                              for f in range(0, 24, 4)], loop=True))
    curled = merge(wave(0.0, 30), {"chest": (40, 0, 30), "neck": (30, 0, 20), "head": (10, 0, 0)})
    made.append(_clip(arm, "recall", 48, [(0, STAND, None, "ease"), (24, curled, L(-0.1), "hold"), (48, curled, L(-0.1), "hold")],
                      markers={"loop_in": 24, "loop_out": 48}))
    made += moves(arm)
    return made


# --- The move library (20 §8.2): one clip per shape, the serpent's way ------------------------
# A snake never stops moving: every move rides a wave that runs from the chest down to the tail
# tip (`body_wave`), building as it coils and dying out after it strikes. The head leads, the
# body follows a beat behind.

HOOD = {"hood_l": (0, -38, 0), "hood_r": (0, 38, 0)}


def body_wave(f, period, amp, lag=0.13):
    out = {}
    for i, bn in enumerate(["chest", "spine", "pelvis"] + TAIL):
        k = 0.45 if bn == "chest" else 1.0
        out[bn] = (0, 0, k * amp * math.sin(2 * math.pi * (f / period - i * lag)))
    return out


def envelope(f, points):
    """A piecewise-linear amplitude over the clip: [(frame, amp), …]."""
    for (f0, a0), (f1, a1) in zip(points, points[1:]):
        if f0 <= f <= f1:
            return a0 + (a1 - a0) * (f - f0) / max(1, f1 - f0)
    return points[-1][1]


def moves(arm):
    out = []

    def clip(name, frames, keys, fire, period, amps, extra=None, markers=None):
        def overlay(f):
            pose = body_wave(f, period, envelope(f, amps))
            if extra is not None:
                for k, v in extra(f).items():
                    pose[k] = tuple(x + y for x, y in zip(pose.get(k, (0, 0, 0)), v))
            return pose, {}
        m = {"fire": fire}
        m.update(markers or {})
        out.append(_clip(arm, name, frames, dense(keys, frames, overlay), markers=m))

    # Bolt (skillshots): the neck draws back into an S, the hood flares, and the head snaps
    # forward to spit; the tail lashes against the recoil.
    coil_back = merge(HOOD, {"chest": (-16, 0, 0), "neck": (-32, 0, 0), "head": (24, 0, 0)})
    spit = merge(HOOD, {"chest": (10, 0, 0), "neck": (18, 0, 0), "head": (-14, 0, 0), "jaw": (44, 0, 0)})
    clip("bolt", 32, [(0, STAND, None, "ease"), (9, coil_back, L(0, 0.08), "snap"), (11, spit, L(0, -0.06), "hold"),
                      (15, spit, L(0, -0.06), "settle"), (32, STAND, None)], 11, 14, [(0, 4), (9, 12), (13, 18), (32, 3)])
    # Beam: rears high, jaw wide, and pours it out, the head swaying a little as the stream holds.
    tall = merge(HOOD, {"chest": (-22, 0, 0), "neck": (-26, 0, 0), "head": (18, 0, 0)})
    pour = merge(HOOD, {"chest": (-12, 0, 0), "neck": (-6, 0, 0), "head": (2, 0, 0), "jaw": (48, 0, 0)})
    clip("beam", 64, [(0, STAND, None, "ease"), (11, tall, L(0.1, 0.06), "whip"), (14, pour, L(0.06, -0.02), "hold"),
                      (50, pour, L(0.06, -0.02), "settle"), (64, STAND, None)], 14, 22, [(0, 4), (12, 8), (50, 6), (64, 3)],
         extra=lambda f: {"head": (0, 0, 5 * math.sin(2 * math.pi * f / 18)) if 14 <= f <= 50 else (0, 0, 0)})
    # Nova (areas round itself): winds the whole body into a coil, then whips the tail round in
    # a full sweep, the head turned the other way.
    wound = merge(HOOD, {"chest": (10, 0, -26), "neck": (-22, 0, 22), "head": (14, 0, 0), "tail_1": (0, 0, 42), "tail_2": (0, 0, 42),
                         "tail_3": (0, 0, 36), "tail_4": (0, 0, 30), "tail_5": (0, 0, 24)})
    sweep = {"chest": (0, 0, 32), "neck": (-10, 0, -18), "head": (6, 0, 0), "jaw": (30, 0, 0), "tail_1": (0, 0, -46), "tail_2": (0, 0, -52),
             "tail_3": (0, 0, -46), "tail_4": (0, 0, -40), "tail_5": (0, 0, -34)}
    clip("nova", 42, [(0, STAND, None, "ease"), (11, wound, L(-0.05), "whip"), (18, sweep, L(0.03), "hold"), (22, sweep, L(0.03), "settle"),
                      (42, STAND, None)], 18, 12, [(0, 3), (11, 6), (18, 22), (42, 2)])
    # Guard (shields): curls into a tight coil, the head tucked down behind the spread hood,
    # hissing.
    curl = merge({"hood_l": (0, -50, 0), "hood_r": (0, 50, 0)},
                 {"chest": (30, 0, 26), "neck": (22, 0, -22), "head": (22, 0, 0), "jaw": (12, 0, 0), "tail_1": (0, 0, 48), "tail_2": (0, 0, 50),
                  "tail_3": (0, 0, 46), "tail_4": (0, 0, 40), "tail_5": (0, 0, 36)})
    clip("guard", 38, [(0, STAND, None, "ease"), (8, curl, L(-0.09), "settle"), (10, curl, L(-0.1), "hold"), (24, curl, L(-0.1), "ease"),
                       (38, STAND, None)], 10, 10, [(0, 4), (8, 2), (24, 2), (38, 3)],
         extra=lambda f: {"chest": (2.5 * math.sin(2 * math.pi * f / 6), 0, 0) if 10 <= f <= 24 else (0, 0, 0)})
    # Empower (buffs, heals): rises tall and sways like a charmed cobra, the neck against the
    # chest so the head stays on its mark; a hiss at the top.
    rise = merge(HOOD, {"chest": (-24, 0, 0), "neck": (-18, 0, 0), "head": (14, 0, 0)})
    hiss = merge(rise, {"jaw": (26, 0, 0), "head": (6, 0, 0)})
    clip("empower", 44, [(0, STAND, None, "ease"), (12, rise, L(0.12), "ease"), (16, hiss, L(0.14), "hold"), (28, hiss, L(0.14), "ease"),
                         (44, STAND, None)], 16, 26, [(0, 3), (44, 3)],
         extra=lambda f: {"chest": (0, 0, 14 * math.sin(2 * math.pi * f / 20)), "neck": (0, 0, -18 * math.sin(2 * math.pi * f / 20))})
    # Blink: coils low and dives head-first into the ground (gone at `fire`), then rises out of
    # it somewhere else, shaking the dirt off down the length of its body.
    dive = {"chest": (46, 0, 0), "neck": (42, 0, 0), "head": (22, 0, 0)}
    emerge = {"chest": (30, 0, 0), "neck": (-34, 0, 0), "head": (32, 0, 0), "jaw": (20, 0, 0)}
    clip("blink", 34, [(0, STAND, None, "ease"), (7, merge(dive, {"neck": (10, 0, 0)}), L(-0.04, 0.04), "whip"), (12, dive, L(-0.14, -0.06), "hold"),
                       (13, emerge, L(-0.16), "settle"), (34, STAND, None)], 12, 10, [(0, 4), (12, 14), (13, 4), (16, 16), (34, 2)])
    return out


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    builds.run(HERE, NAME, table(CENTER, LEFT, SOCKETS, scale=S), body, sliders, catalog, clips, 1.2 * S, argv, detail=3)
