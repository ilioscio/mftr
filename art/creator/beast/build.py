# SPDX-License-Identifier: AGPL-3.0-or-later
"""The four-legged beast build's base (CR10, 20 §3): a champion-sized beast on the `creature`
skeleton of the jungle's hounds (16 §3), with its sliders as shape keys, its catalog parts and
its own clips (a beast's move library: the moves' numbers come from any kit, their animation
from these clips, by the ability's shape).

    blender -b --python art/creator/beast/build.py -- [--export]

Lands in `beast.blend` next to this file; `--export` also writes `export/beast.glb`, its
`.anims.ron` (kind `base`, with the clips) and `beast.parts.ron`. No genders (20 §4 allows a
subtle one later). Block-outs (D49): run once, then polish the .blend.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))
sys.path.insert(0, os.path.join(ROOT, "art", "monsters"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

from creature import bone, socket, table  # noqa: E402
from mftr_blender import creator, export, mesh, parts, rig  # noqa: E402
from mftr_blender.clips import merge  # noqa: E402
from mftr_blender.library import _clip, dense  # noqa: E402

NAME = "beast"
S = 1.5  # a champion's size: the back at about 1.15 m, the head at 1.45 m

# Neutral colors the paint step recolors: fur, its darker back, a pale belly and muzzle.
FUR = (0.42, 0.34, 0.26)
FUR_D = (0.26, 0.2, 0.15)
BELLY = (0.62, 0.55, 0.45)
NOSE = (0.06, 0.05, 0.05)
TOOTH = (0.85, 0.82, 0.74)
ACCENT = (0.18, 0.52, 0.95)
GLOW = (1.0, 1.0, 1.0)

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
# What a champion's effects need (`mftr_pack::rules`): the mouth casts and fires.
SOCKETS = [
    socket("socket_chest", "chest", (0, -0.42, 0.72)),
    socket("socket_overhead", "root", (0, 0, 1.4)),
    socket("socket_projectile", "jaw", (0, -0.86, 0.9)),
    socket("socket_cast", "jaw", (0, -0.86, 0.9)),
    socket("socket_weapon_tip", "jaw", (0, -0.86, 0.9)),
]


def B(name, t, off=(0.0, 0.0, 0.0)):
    return (name, t, tuple(x * S for x in off))


def r(*v):
    return tuple(x * S for x in v)


def body():
    p = [
        mesh.path_part("pelvis", [B("pelvis", -0.15), B("pelvis", 1.0)], 18, [r(0.15, 0.17), r(0.17, 0.2)], "cloth", FUR),
        mesh.path_part("spine", [B("spine", -0.05), B("spine", 0.55, (0, 0, -0.01)), B("spine", 1.05, (0, 0, -0.03))], 10,
                       [r(0.17, 0.19), r(0.18, 0.21), r(0.21, 0.26)], "cloth", FUR_D),
        mesh.path_part("chest", [B("chest", -0.05, (0, 0, -0.04)), B("chest", 0.5, (0, 0, -0.06)), B("chest", 1.05, (0, 0, -0.02))], 10,
                       [r(0.21, 0.27), r(0.22, 0.29), r(0.17, 0.24)], "cloth", FUR),
        mesh.path_part("chest", [B("chest", 0.2, (0, 0, -0.24)), B("chest", 0.85, (0, 0, -0.22))], 15, [r(0.12, 0.05), r(0.1, 0.05)], "cloth", BELLY),
        mesh.path_part("neck", [B("neck", -0.1), B("neck", 1.1)], 16, [r(0.14, 0.15), r(0.1, 0.11)], "cloth", FUR),
        mesh.path_part("head", [B("head", -0.05, (0, 0, 0.02)), B("head", 0.35, (0, 0, 0.02)), B("head", 0.8, (0, 0, -0.01)), B("head", 1.0, (0, 0, -0.02))], 8,
                       [r(0.11, 0.1), r(0.12, 0.1), r(0.06, 0.055), r(0.045, 0.04)], "cloth", FUR),
        mesh.path_part("head", [B("head", 0.55, (0, 0, -0.04)), B("head", 0.95, (0, 0, -0.03))], 14, [r(0.06, 0.04), r(0.04, 0.03)], "cloth", BELLY),
        mesh.path_part("head", [B("head", 0.98, (0, 0, -0.015)), B("head", 1.05, (0, 0, -0.02))], 15, [r(0.04, 0.035), r(0.02, 0.02)], "metal", NOSE),
        mesh.path_part("jaw", [B("jaw", 0.0), B("jaw", 1.0)], 15, [r(0.06, 0.03), r(0.035, 0.02)], "cloth", BELLY),
        mesh.path_part("jaw", [B("jaw", 0.75, (0, 0, 0.025)), B("jaw", 0.95, (0, 0, 0.03))], 4, [r(0.03, 0.01), r(0.02, 0.008)], "metal", TOOTH),
        mesh.path_part("tail_1", [B("tail_1", 0), B("tail_1", 1)], 15, [r(0.06, 0.06), r(0.08, 0.08)], "cloth", FUR),
        mesh.path_part("tail_2", [B("tail_2", 0), B("tail_2", 1)], 15, [r(0.08, 0.08), r(0.07, 0.07)], "cloth", FUR_D),
        mesh.path_part("tail_3", [B("tail_3", 0), B("tail_3", 1.1)], 15, [r(0.07, 0.07), (0, 0)], "cloth", FUR),
        # The team accent (05 §1.3): a band round the chest, under the barding's line.
        mesh.path_part("chest", [B("chest", 0.62, (0, 0, -0.03)), B("chest", 0.78, (0, 0, -0.03))], 18, [r(0.225, 0.295), r(0.22, 0.29)], "accent", ACCENT),
    ]
    left = [
        mesh.path_part("ear_l", [B("ear_l", -0.1), B("ear_l", 1.0)], 4, [r(0.05, 0.02), (0, 0)], "cloth", FUR_D),
        # Eyes in the signature hue (they glow, 14 §4).
        mesh.box_part("head", B("head", 0.32, (0.065, 0, 0.05)), r(0.05, 0.035, 0.03), "emissive", GLOW, rot=(0, 0, 25)),
        mesh.path_part("upper_front_l", [B("upper_front_l", -0.1), B("upper_front_l", 1.0)], 15, [r(0.08, 0.1), r(0.05, 0.05)], "cloth", FUR),
        mesh.path_part("lower_front_l", [B("lower_front_l", 0), B("lower_front_l", 1.0)], 13, [r(0.042, 0.042), r(0.034, 0.034)], "cloth", FUR_D),
        mesh.box_part("paw_front_l", B("paw_front_l", 0.5), r(0.08, 0.12, 0.05), "cloth", FUR_D, bevel=0.3),
        mesh.path_part("paw_front_l", [B("paw_front_l", 0.9, (0, 0, 0.01)), B("paw_front_l", 1.25, (0, 0, -0.005))], 4, [r(0.03, 0.012), (0, 0)], "metal", NOSE),
        mesh.path_part("thigh_l", [B("thigh_l", -0.15), B("thigh_l", 1.0)], 15, [r(0.1, 0.13), r(0.055, 0.06)], "cloth", FUR),
        mesh.path_part("shin_l", [B("shin_l", 0), B("shin_l", 1.0)], 13, [r(0.045, 0.045), r(0.035, 0.035)], "cloth", FUR_D),
        mesh.path_part("hock_l", [B("hock_l", 0), B("hock_l", 1.0)], 13, [r(0.033, 0.033), r(0.03, 0.03)], "cloth", FUR_D),
        mesh.box_part("paw_hind_l", B("paw_hind_l", 0.5), r(0.075, 0.12, 0.045), "cloth", FUR_D, bevel=0.3),
        mesh.path_part("paw_hind_l", [B("paw_hind_l", 0.9, (0, 0, 0.01)), B("paw_hind_l", 1.25, (0, 0, -0.005))], 4, [r(0.03, 0.012), (0, 0)], "metal", NOSE),
        mesh.path_part("jaw", [B("jaw", 0.7, (0.03, 0, 0.03)), B("jaw", 0.72, (0.03, 0, 0.07))], 3, [r(0.012, 0.009), (0, 0)], "metal", TOOTH),
    ]
    # The art pass: a mane of fur swept back from the neck, tufts down the flanks and over the
    # haunches, claws on every paw.
    for k, (t, out, up, back) in enumerate(((0.15, 0.13, -0.08, 0.18), (0.35, 0.15, 0.0, 0.22), (0.55, 0.13, 0.08, 0.24),
                                            (0.75, 0.1, 0.14, 0.24), (0.95, 0.05, 0.18, 0.2))):
        left.append(mesh.path_part("neck", [B("neck", t, (out * 0.5, 0.0, up * 0.6)), B("neck", t, (out + 0.04, back * 0.6, up + 0.03)),
                                            B("neck", t, (out + 0.07, back, up + 0.05))], 6, [r(0.06, 0.03), r(0.05, 0.025), (0, 0)], "cloth",
                                   FUR_D if k % 2 else FUR))
    for bn, t, z in (("chest", 0.3, 0.05), ("chest", 0.8, 0.0), ("spine", 0.3, 0.06), ("spine", 0.8, 0.04), ("pelvis", 0.4, 0.06),
                     ("thigh_l", 0.3, 0.0)):
        x = 0.2 if bn not in ("pelvis", "thigh_l") else 0.17
        left.append(mesh.path_part(bn, [B(bn, t, (x, 0, z)), B(bn, t, (x * 1.12, 0.08, z + 0.01)), B(bn, t, (x * 1.18, 0.15, z + 0.02))], 6,
                                   [r(0.05, 0.022), r(0.035, 0.016), (0, 0)], "cloth", FUR_D))
    for paw in ("paw_front_l", "paw_hind_l"):
        for k in (-1, 0, 1):
            left.append(mesh.path_part(paw, [B(paw, 0.95, (0.025 * k, 0, 0.0)), B(paw, 1.35, (0.03 * k, 0, -0.012))], 6,
                                       [r(0.014, 0.012), (0, 0)], "metal", NOSE))
    for k in range(4):
        p.append(mesh.path_part("chest", [B("chest", 0.9 - 0.25 * k, (0, 0.0, 0.2)), B("chest", 0.9 - 0.25 * k, (0, 0.12, 0.28))], 6,
                                [r(0.05, 0.02), (0, 0)], "cloth", FUR_D))
    return p + left + [mesh.mirror(b) for b in left]


# --- Sliders (20 §5–6), on the beast's own landmarks -------------------------------------------

def sliders():
    G = creator  # the block-out strokes
    HEAD = ("head",)  # noqa: N806
    out = []

    def add(sid, desc, groups, plus, minus=None):
        out.append((sid, desc, groups, plus, minus))

    def centre(r_):
        return (r_.lo + r_.hi) / 2

    add("head_size", "A big head or a small one", HEAD, lambda r_: G.scale(r_, centre(r_), (1.18, 1.18, 1.18)))
    add("head_snout", "A long snout or a short one", HEAD,
        lambda r_: {i: Vector((0, -(1 - (r_.cos[i].y - r_.lo.y) / r_.size.y) * 0.06 * S, 0)) for i in r_.idx if (r_.cos[i].y - r_.lo.y) / r_.size.y < 0.6})
    add("head_brow", "A heavy brow or a smooth one", HEAD,
        lambda r_: G.grab(r_, Vector((0, r_.lo.y + r_.size.y * 0.3, r_.hi.z)), 0.08 * S, Vector((0, -0.01, 0.025)) * S, mirror=False))
    add("jaw_heavy", "A heavy jaw or a light one", ("jaw",), lambda r_: G.scale(r_, Vector((0, r_.hi.y, r_.hi.z)), (1.35, 1.0, 1.35)))
    add("ears_size", "Big ears or small ones", ("ear_l", "ear_r"),
        lambda r_: {i: (r_.cos[i] - Vector((math.copysign(0.07 * S, r_.cos[i].x), -0.58 * S, 1.04 * S))) * 0.5 for i in r_.idx})
    add("body_bulk", "A heavy body or a lean one", ("pelvis", "spine", "chest", "neck"),
        lambda r_: G.scale(r_, Vector((0, 0, (r_.lo.z + r_.hi.z) / 2)), (1.18, 1.0, 1.15)))
    add("body_chest", "A deep chest or a shallow one", ("chest",),
        lambda r_: G.grab(r_, Vector((0, (r_.lo.y + r_.hi.y) / 2, r_.lo.z)), 0.2 * S, Vector((0, 0, -0.06)) * S, mirror=False))
    add("body_legs", "Thick legs or thin ones",
        tuple(f"{b}_{s}" for b in ("upper_front", "lower_front", "thigh", "shin", "hock") for s in "lr"), lambda r_: creator._limb_scale(r_, 1.3))
    add("body_tail", "A bushy tail or a thin one", ("tail_1", "tail_2", "tail_3"), lambda r_: creator._limb_scale(r_, 1.5))
    return out


def add_shape_keys(armature, body_obj):
    body_obj.shape_key_add(name="Basis", from_mix=False)
    ids = []
    for sid, _desc, groups, plus, minus in sliders():
        reg = creator.Region(body_obj, set(groups))
        reg.armature = armature.name
        gain = creator.GAIN["face" if sid.startswith(("head", "jaw", "ears")) else "body"]
        up = {i: v * gain for i, v in plus(reg).items()}
        down = {i: v * gain for i, v in minus(reg).items()} if minus else {i: -v for i, v in up.items()}
        for suffix, deltas in (("+", up), ("-", down)):
            key = body_obj.shape_key_add(name=sid + suffix, from_mix=False)
            for i, d in deltas.items():
                key.data[i].co = reg.cos[i] + d
        ids.append(sid)
    return ids


# --- Parts (20 §7): a beast's gear ----------------------------------------------------------

IRON = (0.42, 0.43, 0.47)
LEATHER = (0.36, 0.22, 0.12)
CLOTH_RED = (0.45, 0.10, 0.08)
HORN = (0.62, 0.56, 0.44)


def catalog():
    out = []

    def add(pid, slot, name, items, hides=(), accent=False):
        out.append((pid, slot, name, list(hides), accent, items))

    horn = mesh.path_part("head", [B("head", 0.2, (0.06, 0, 0.07)), B("head", 0.0, (0.13, 0.05, 0.16)), B("head", -0.15, (0.16, 0.16, 0.22))], 6,
                          [r(0.035, 0.035), r(0.024, 0.024), (0, 0)], "cloth", HORN)
    add("head.horns", "head", "Horns", [horn, mesh.mirror(horn)])
    collar = [mesh.path_part("neck", [B("neck", 0.25), B("neck", 0.5)], 12, [r(0.155, 0.165), r(0.145, 0.155)], "cloth", LEATHER),
              mesh.path_part("neck", [B("neck", 0.33), B("neck", 0.42)], 12, [r(0.16, 0.17), r(0.155, 0.165)], "accent", ACCENT)]
    for k in range(6):
        a = 2 * math.pi * (k + 0.5) / 6
        collar.append(mesh.path_part("neck", [B("neck", 0.37, (math.cos(a) * 0.15, 0, math.sin(a) * 0.16)),
                                              B("neck", 0.37, (math.cos(a) * 0.24, 0, math.sin(a) * 0.25))], 4, [r(0.025, 0.025), (0, 0)], "metal", IRON))
    add("shoulders.collar", "shoulders", "Spiked collar", collar, accent=True)
    plates = []
    for i, (bn, t) in enumerate((("chest", 0.55), ("chest", 0.1), ("spine", 0.55), ("pelvis", 0.6))):
        plates.append(mesh.box_part(bn, B(bn, t, (0, 0, 0.21 - 0.015 * i)), r(0.36 - 0.03 * i, 0.22, 0.05), "metal", IRON, bevel=0.3))
    add("torso.barding", "torso", "Barding", plates)
    add("back.saddle", "back", "Saddle", [
        mesh.box_part("spine", B("spine", 0.4, (0, 0, 0.21)), r(0.42, 0.34, 0.04), "accent", ACCENT),
        mesh.box_part("spine", B("spine", 0.4, (0, 0, 0.25)), r(0.24, 0.28, 0.07), "cloth", LEATHER, bevel=0.3),
        mesh.box_part("spine", B("spine", 0.15, (0, 0, 0.31)), r(0.14, 0.05, 0.08), "cloth", LEATHER)], accent=True)
    add("back.cape", "back", "Cape", [mesh.box_part("chest", B("chest", 0.3, (0, 0.1, 0.24)), r(0.5, 0.6, 0.03), "cloth", CLOTH_RED, rot=(-8, 0, 0))])
    beam = mesh.path_part("head", [B("head", 0.2, (0.06, 0, 0.08)), B("head", 0.05, (0.15, 0.06, 0.24)), B("head", -0.05, (0.2, 0.14, 0.42))], 6,
                          [r(0.03, 0.03), r(0.022, 0.022), (0, 0)], "cloth", HORN)
    tine = mesh.path_part("head", [B("head", 0.05, (0.15, 0.06, 0.24)), B("head", 0.22, (0.25, -0.02, 0.34))], 4, [r(0.016, 0.016), (0, 0)], "cloth", HORN)
    tine2 = mesh.path_part("head", [B("head", 0.0, (0.18, 0.1, 0.34)), B("head", 0.1, (0.3, 0.06, 0.46))], 4, [r(0.014, 0.014), (0, 0)], "cloth", HORN)
    add("head.antlers", "head", "Antlers", [beam, tine, tine2, mesh.mirror(beam), mesh.mirror(tine), mesh.mirror(tine2)])
    add("face.muzzle", "face", "Iron muzzle", [
        mesh.path_part("head", [B("head", 0.6, (0, 0, -0.005)), B("head", 0.65, (0, 0, -0.005))], 8, [r(0.075, 0.068), r(0.075, 0.068)], "metal", IRON),
        mesh.path_part("head", [B("head", 0.88, (0, 0, -0.012)), B("head", 0.92, (0, 0, -0.012))], 8, [r(0.055, 0.05), r(0.055, 0.05)], "metal", IRON),
        mesh.path_part("head", [B("head", 0.6, (0, 0, 0.065)), B("head", 0.9, (0, 0, 0.042))], 4, [r(0.014, 0.01), r(0.014, 0.01)], "metal", IRON)])
    wrap = mesh.path_part("lower_front_l", [B("lower_front_l", 0.3), B("lower_front_l", 0.8)], 8, [r(0.06, 0.06), r(0.052, 0.052)], "cloth", LEATHER)
    wrap_h = mesh.path_part("shin_l", [B("shin_l", 0.35), B("shin_l", 0.8)], 8, [r(0.06, 0.06), r(0.052, 0.052)], "cloth", LEATHER)
    add("legs.wraps", "legs", "Leg wraps", [wrap, mesh.mirror(wrap), wrap_h, mesh.mirror(wrap_h)])
    add("back.banner", "back", "War banner", [
        mesh.path_part("chest", [B("chest", 0.1, (0, 0.02, 0.2)), B("chest", 0.1, (0, 0.06, 0.95))], 6, [r(0.016, 0.016), r(0.014, 0.014)], "cloth", LEATHER),
        mesh.box_part("chest", B("chest", 0.1, (0, 0.17, 0.74)), r(0.02, 0.22, 0.3), "accent", ACCENT),
        mesh.box_part("chest", B("chest", 0.1, (0, 0.06, 0.96)), r(0.04, 0.04, 0.04), "metal", IRON, rot=(45, 0, 45))], accent=True)
    tusk = mesh.path_part("jaw", [B("jaw", 0.3, (0.05, 0, 0)), B("jaw", 0.7, (0.09, -0.04, 0.06)), B("jaw", 1.0, (0.08, -0.1, 0.15))], 4,
                          [r(0.025, 0.01), r(0.02, 0.008), (0, 0)], "metal", IRON)
    add("weapon.tusk_blades", "weapon", "Tusk blades", [tusk, mesh.mirror(tusk)])
    return out


def build_parts(armature, body_obj):
    made = []
    for pid, slot, name, hides, accent, items in catalog():
        obj = mesh.build_mannequin(armature, name=f"part.{pid}", part_list=items)
        n = parts.transfer_shape_keys(body_obj, obj)
        made.append((pid, slot, name, hides, accent, mesh.triangle_count(obj), n))
    return made


# --- Clips: the beast's move library (20 §8.2) ----------------------------------------------

def legs(fl=0.0, fr=0.0, hl=0.0, hr=0.0, flex=None):
    pose = {"upper_front_l": (fl, 0, 0), "upper_front_r": (fr, 0, 0), "thigh_l": (hl, 0, 0), "thigh_r": (hr, 0, 0)}
    return merge(pose, {k: (v, 0, 0) for k, v in (flex or {}).items()})


STAND = merge(legs(), {"neck": (-4, 0, 0), "head": (4, 0, 0), "tail_1": (14, 0, 0), "tail_2": (10, 0, 0)})
CROUCH = merge(legs(-14, -14, -16, -16, {"lower_front_l": 30, "lower_front_r": 30, "shin_l": 30, "shin_r": 30, "hock_l": -30, "hock_r": -30}),
               {"neck": (14, 0, 0), "head": (-6, 0, 0), "tail_1": (-20, 0, 0)})
LEAP = merge(legs(-60, -55, 50, 45, {"lower_front_l": -10, "lower_front_r": -10}), {"spine": (-6, 0, 0), "neck": (-10, 0, 0), "jaw": (30, 0, 0),
                                                                                  "tail_1": (-20, 0, 0)})


def L(z=0.0, y=0.0):
    return {"pelvis": (0, y * S, z * S)}


def gallop(arm, name, frames, stride_speed, reach=1.0):
    keys = []
    for f in range(0, frames, 2):
        p = f / frames

        def swing(off, back, fwd):
            q = (p - off) % 1.0
            if q < 0.5:
                return fwd + (back - fwd) * (q / 0.5), 0.0
            s = (q - 0.5) / 0.5
            return back + (fwd - back) * (0.5 - 0.5 * math.cos(math.pi * s)), math.sin(math.pi * s)

        fl, ffl = swing(0.55, 35 * reach, -45 * reach)
        fr, ffr = swing(0.65, 35 * reach, -45 * reach)
        hl, fhl = swing(0.0, 40 * reach, -30 * reach)
        hr, fhr = swing(0.1, 40 * reach, -30 * reach)
        flex = {"lower_front_l": 50 * ffl, "lower_front_r": 50 * ffr, "paw_front_l": 30 * ffl, "paw_front_r": 30 * ffr,
                "shin_l": 35 * fhl, "shin_r": 35 * fhr, "hock_l": -40 * fhl, "hock_r": -40 * fhr}
        bend = 10 * reach * math.sin(2 * math.pi * (p - 0.2))
        pose = merge(legs(fl, fr, hl, hr, flex), {"spine": (bend, 0, 0), "chest": (-bend * 0.5, 0, 0), "neck": (-10 - bend * 0.5, 0, 0),
                                                  "head": (14, 0, 0), "jaw": (6, 0, 0), "tail_1": (-10, 0, 0), "tail_2": (6, 0, 0)})
        keys.append((f, pose, L(0.05 * reach * math.sin(2 * math.pi * p) - 0.02)))
    return _clip(arm, name, frames, keys, loop=True, markers={"foot_l": 0, "foot_r": frames // 2}, stride_speed=stride_speed)


def trot(arm):
    # Diagonal pairs together: a walk for slow moves.
    frames = 24
    keys = []
    for f in range(0, frames, 3):
        s = math.sin(2 * math.pi * f / frames)
        lift_a, lift_b = max(0.0, s), max(0.0, -s)
        pose = merge(legs(-25 * s, 25 * s, 22 * s, -22 * s, {"lower_front_l": 40 * lift_b, "lower_front_r": 40 * lift_a,
                                                          "shin_l": 30 * lift_a, "shin_r": 30 * lift_b}),
                     {"neck": (-6, 0, 0), "head": (6, 0, 4 * s), "tail_1": (10, 0, 10 * s)})
        keys.append((f, pose, L(0.015 * abs(s))))
    return _clip(arm, "walk", frames, keys, loop=True, markers={"foot_l": 0, "foot_r": frames // 2}, stride_speed=150.0)


def clips(arm):
    made = []
    keys = []
    for f in range(0, 48, 4):
        pant = 1 if (f // 4) % 2 else 0
        turn = 8 * math.sin(2 * math.pi * f / 48)
        keys.append((f, merge(STAND, {"jaw": (4 + 6 * pant, 0, 0), "chest": (-2 * pant, 0, 0), "head": (4, 0, turn), "neck": (-4, 0, turn * 0.5),
                                      "tail_1": (14, 0, 12 * math.sin(2 * math.pi * f / 24))}), L(0.006 * pant)))
    made.append(_clip(arm, "idle", 48, keys, loop=True))
    sniff = merge(STAND, {"neck": (40, 0, 10), "head": (26, 0, 0)}, legs(-8, -8, 0, 0))
    made.append(_clip(arm, "idle_fidget_1", 48, [(0, STAND, L()), (12, sniff, L(-0.04)), (20, merge(sniff, {"head": (32, 0, 8)}), L(-0.04)),
                                                  (30, sniff, L(-0.04)), (48, STAND, L())]))
    made.append(trot(arm))
    made.append(gallop(arm, "run", 16, 340.0))
    # Attacks: a lunging bite, and a raking swipe of a forepaw.
    lunge = merge(legs(-50, -40, 40, 36), {"spine": (-4, 0, 0), "neck": (-6, 0, 0), "jaw": (40, 0, 0), "tail_1": (-10, 0, 0)})
    bite = merge(lunge, {"jaw": (0, 0, 0), "head": (10, 0, 0), "neck": (6, 0, 0)})
    made.append(_clip(arm, "attack_1", 40, [(0, STAND, L(), "ease"), (7, CROUCH, L(-0.1, 0.06), "snap"), (11, lunge, L(0.03, -0.16), "snap"),
                                            (13, bite, L(0.0, -0.2), "hold"), (18, bite, L(0.0, -0.18), "settle"), (40, STAND, L(), "ease")],
                      markers={"fire": 13}))
    rear = merge(legs(-70, 10, 0, 0, {"lower_front_l": 60}), {"chest": (-10, 0, 0), "neck": (-10, 0, 10), "head": (0, 0, 10)})
    rake = merge(legs(20, 10, 0, 0, {"lower_front_l": -10}), {"chest": (4, 0, 0), "neck": (8, 0, -10), "head": (6, 0, -10)})
    made.append(_clip(arm, "attack_2", 40, [(0, STAND, L(), "ease"), (8, rear, L(0.04), "snap"), (12, rake, L(-0.02, -0.08), "hold"),
                                            (18, rake, L(-0.02, -0.08), "settle"), (40, STAND, L(), "ease")], markers={"fire": 12}))
    # Casts. The howl (projectiles, beams, buffs): sits back, the head thrown up, the jaw wide.
    sit = merge(legs(-6, -6, -30, -30, {"shin_l": 40, "shin_r": 40, "hock_l": -40, "hock_r": -40}),
                {"pelvis": (-22, 0, 0), "neck": (-40, 0, 0), "head": (-30, 0, 0), "jaw": (34, 0, 0), "tail_1": (40, 0, 0)})
    snap_fwd = merge(STAND, {"neck": (6, 0, 0), "head": (-4, 0, 0), "jaw": (40, 0, 0)})
    made.append(_clip(arm, "howl", 36, [(0, STAND, L(), "ease"), (8, merge(sit, {"jaw": (0, 0, 0)}), L(-0.1, 0.05), "snap"),
                                        (11, snap_fwd, L(0.0, -0.04), "hold"), (16, snap_fwd, L(0.0, -0.04), "settle"), (36, STAND, L(), "ease")],
                      markers={"fire": 11}))
    # The slam (areas): rears up on the hind legs and comes down on both forepaws.
    up = merge(legs(-80, -80, -10, -10, {"lower_front_l": 70, "lower_front_r": 70}), {"pelvis": (-35, 0, 0), "neck": (-14, 0, 0), "jaw": (20, 0, 0)})
    down = merge(legs(-20, -20, 10, 10, {"lower_front_l": -10, "lower_front_r": -10}), {"chest": (10, 0, 0), "neck": (16, 0, 0), "head": (10, 0, 0)})
    made.append(_clip(arm, "slam", 44, [(0, STAND, L(), "ease"), (12, up, L(0.18, 0.1), "hold"), (16, up, L(0.2, 0.1), "whip"),
                                        (19, down, L(-0.08, -0.06), "hold"), (26, down, L(-0.08, -0.06), "settle"), (44, STAND, L(), "ease")],
                      markers={"fire": 19}))
    # The pounce (lunges): a coiled crouch, a leap onto the target, the bite on landing.
    made.append(_clip(arm, "pounce", 40, [(0, STAND, L(), "ease"), (6, CROUCH, L(-0.12, 0.08), "snap"), (10, LEAP, L(0.25, -0.2), "snap"),
                                          (14, bite, L(0.0, -0.3), "hold"), (20, bite, L(0.0, -0.25), "settle"), (40, STAND, L(), "ease")],
                      markers={"fire": 14}))
    # The dash set (dashes): a crouch, a bounding leap held while travelling, a landing.
    made.append(_clip(arm, "dash_start", 8, [(0, STAND, L(), "ease"), (8, CROUCH, L(-0.1, 0.06), "snap")], markers={"fire": 7}))
    made.append(gallop(arm, "dash_travel", 12, 900.0, reach=1.3))
    made.append(_clip(arm, "dash_land", 14, [(0, CROUCH, L(-0.12), "hold"), (14, STAND, L(), "ease")]))
    made.append(_clip(arm, "death", 22, [(0, STAND, L(), "ease"), (5, merge(STAND, {"neck": (-24, 0, 0), "jaw": (26, 0, 0)}), L(0.02), "hold"),
                                         (22, merge(legs(-20, -10, 20, 30, {"lower_front_l": 40, "shin_l": 30}),
                                                    {"pelvis": (0, 80, 0), "neck": (20, 0, -10), "tail_1": (30, 0, 20)}), L(-0.42), "hold")]))
    back = {"neck": (-10, 0, 0), "head": (-8, 0, 0), "chest": (-4, 0, 0)}
    made.append(_clip(arm, "flinch", 8, [(0, {}, None), (2, back, None), (8, {}, None)], layer="additive", bones=["neck", "head", "chest"]))
    dazed = [(f, merge(STAND, {"neck": (10, 0, 14 * math.sin(2 * math.pi * f / 24)), "head": (14, 0, -18 * math.sin(2 * math.pi * f / 24))},
                       legs(-6, 6, 0, 0)), L(-0.04)) for f in range(0, 24, 4)]
    made.append(_clip(arm, "cc_stunned", 24, dazed, loop=True))
    lie = merge(legs(-70, -70, -40, -40, {"lower_front_l": 70, "lower_front_r": 70, "shin_l": 60, "shin_r": 60}),
                {"neck": (10, 0, 0), "head": (6, 0, 0), "tail_1": (40, 0, 30)})
    made.append(_clip(arm, "recall", 48, [(0, STAND, L(), "ease"), (16, sit, L(-0.14, 0.06), "ease"), (32, lie, L(-0.4), "hold"),
                                          (48, lie, L(-0.4), "hold")], markers={"loop_in": 32, "loop_out": 48}))
    made += moves(arm)
    return made


# --- The move library (20 §8.2): one clip per shape, the beast's way ---------------------------
# A beast is all weight: it loads its legs before it acts, the head leads and the body follows,
# and it never quite stops breathing (`breath`: the ribs heave, the tail swings, the ears flick).


def breath(f, rate=12, heave=2.0, tail=10.0):
    s = math.sin(2 * math.pi * f / rate)
    return ({"chest": (-heave * s, 0, 0), "tail_1": (0, 0, tail * math.sin(2 * math.pi * f / (rate * 2))),
             "tail_2": (0, 0, tail * 0.8 * math.sin(2 * math.pi * (f / (rate * 2) - 0.15)))}, {"pelvis": (0, 0, 0.004 * S * s)})


def moves(arm):
    out = []

    def clip(name, frames, keys, fire, overlay=None):
        out.append(_clip(arm, name, frames, dense(keys, frames, overlay or (lambda f: breath(f))), markers={"fire": fire}))

    ears_back = {"ear_l": (40, 0, 20), "ear_r": (40, 0, -20)}
    ears_up = {"ear_l": (-14, 0, 0), "ear_r": (-14, 0, 0)}
    # Bolt (skillshots): draws its head down and back between braced forelegs, then thrusts it
    # forward with the jaw thrown open, the hind legs pushing.
    gather = merge(legs(-20, -20, -8, -8, {"lower_front_l": 20, "lower_front_r": 20}), ears_back,
                   {"neck": (24, 0, 0), "head": (-14, 0, 0), "chest": (6, 0, 0), "tail_1": (-14, 0, 0)})
    spit = merge(legs(-30, -30, 24, 24), ears_back, {"neck": (-4, 0, 0), "head": (-10, 0, 0), "jaw": (44, 0, 0), "chest": (-6, 0, 0),
                                                     "tail_1": (-26, 0, 0), "tail_2": (-10, 0, 0)})
    clip("bolt", 34, [(0, STAND, L(), "ease"), (9, gather, L(-0.06, 0.06), "whip"), (11, spit, L(0.0, -0.08), "hold"), (16, spit, L(0.0, -0.07), "settle"),
                      (34, STAND, L())], 11)
    # Beam (breath): plants all four legs wide, lowers its head and pours it out of an open jaw,
    # the ribs heaving and the head shaking with the strain.
    plant = merge(legs(-18, -18, 12, 12, {"lower_front_l": 16, "lower_front_r": 16, "shin_l": 16, "shin_r": 16}), ears_back,
                  {"neck": (10, 0, 0), "head": (-16, 0, 0), "jaw": (50, 0, 0), "tail_1": (-20, 0, 0)})
    clip("beam", 64, [(0, STAND, L(), "ease"), (10, merge(plant, {"jaw": (10, 0, 0), "neck": (-14, 0, 0)}), L(-0.04, 0.05), "whip"),
                      (14, plant, L(-0.08, -0.02), "hold"), (52, plant, L(-0.08, -0.02), "settle"), (64, STAND, L())], 14,
         lambda f: (merge(breath(f, rate=6, heave=4.0)[0], {"head": (0, 0, 3 * math.sin(2 * math.pi * f / 5)) if 14 <= f <= 52 else (0, 0, 0)}),
                    breath(f, rate=6)[1]))
    # Nova (areas round itself): sinks low, then rears up on its hind legs with a roar that
    # shakes its whole body.
    rear = merge(legs(-70, -66, -14, -14, {"lower_front_l": 64, "lower_front_r": 60}), ears_back,
                 {"pelvis": (-30, 0, 0), "neck": (-30, 0, 0), "head": (-24, 0, 0), "jaw": (46, 0, 0), "tail_1": (30, 0, 0)})
    clip("nova", 44, [(0, STAND, L(), "ease"), (10, CROUCH, L(-0.14, 0.06), "whip"), (16, rear, L(0.16, 0.12), "hold"), (24, rear, L(0.16, 0.12), "ease"),
                      (44, STAND, L())], 16,
         lambda f: (merge(breath(f)[0], {"chest": (0, 0, 4 * math.sin(2 * math.pi * f / 3)) if 16 <= f <= 24 else (0, 0, 0)}), breath(f)[1]))
    # Guard (shields): hunkers down, head low and tucked, ears flat, hackles up (the back
    # arched), the tail clamped.
    hunker = merge(legs(-30, -30, -26, -26, {"lower_front_l": 50, "lower_front_r": 50, "shin_l": 46, "shin_r": 46, "hock_l": -40, "hock_r": -40}),
                   ears_back, {"spine": (-12, 0, 0), "chest": (6, 0, 0), "neck": (30, 0, 0), "head": (10, 0, 0), "jaw": (14, 0, 0),
                               "tail_1": (40, 0, 0), "tail_2": (30, 0, 0)})
    clip("guard", 38, [(0, STAND, L(), "ease"), (8, hunker, L(-0.18), "settle"), (10, hunker, L(-0.2), "hold"), (24, hunker, L(-0.2), "ease"),
                       (38, STAND, L())], 10, lambda f: breath(f, rate=6, heave=3.0, tail=2.0))
    # Empower (buffs, heals): sits back on its haunches and howls at the sky, ears up.
    sit = merge(legs(-6, -6, -30, -30, {"shin_l": 40, "shin_r": 40, "hock_l": -40, "hock_r": -40}), ears_up,
                {"pelvis": (-22, 0, 0), "neck": (-44, 0, 0), "head": (-34, 0, 0), "tail_1": (40, 0, 0)})
    howl_ = merge(sit, {"jaw": (36, 0, 0), "head": (-40, 0, 0)})
    clip("empower", 46, [(0, STAND, L(), "ease"), (12, sit, L(-0.1, 0.06), "ease"), (16, howl_, L(-0.08, 0.06), "hold"), (30, howl_, L(-0.08, 0.06), "ease"),
                         (46, STAND, L())], 16)
    # Blink: crouches and springs straight up out of sight (gone at `fire`); drops in somewhere
    # else, landing in a crouch, and shakes itself off.
    spring = merge(LEAP, ears_back, {"spine": (-14, 0, 0)})
    land = merge(CROUCH, ears_back)
    clip("blink", 36, [(0, STAND, L(), "ease"), (8, CROUCH, L(-0.14, 0.06), "whip"), (12, spring, L(0.5), "hold"), (13, spring, L(0.4), "whip"),
                       (16, land, L(-0.14), "settle"), (36, STAND, L())], 12,
         lambda f: (merge(breath(f)[0], {"spine": (0, 0, 6 * math.sin(2 * math.pi * f / 4)), "neck": (0, 0, -8 * math.sin(2 * math.pi * f / 4))}
                          if 22 <= f <= 30 else breath(f)[0]), breath(f)[1]))
    return out


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig.use_table(table(CENTER, LEFT, SOCKETS, scale=S))
    mesh.AO_HEIGHT = 1.0 * S
    mesh.PATH_DETAIL = 3  # the art pass: rounder lofts (mesh.PATH_DETAIL)
    arm = rig.build_armature(name=NAME)
    made_clips = clips(arm)
    body_obj = mesh.build_mannequin(arm, name=NAME, part_list=body())
    ids = add_shape_keys(arm, body_obj)
    made = build_parts(arm, body_obj)
    creator.unwrap(body_obj, head_group="head")
    for obj in [o for o in bpy.data.objects if o.name.startswith("part.")]:
        creator.unwrap(obj)
    arm["mftr_build"] = NAME
    arm["mftr_sliders"] = ",".join(ids)
    print(f"{NAME}: {mesh.triangle_count(body_obj)} triangles, {len(ids)} sliders, {len(made)} parts, clips {[a.name for a in made_clips]}")
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(HERE, f"{NAME}.blend"), compress=True)
    if "--export" in argv:
        print(export.export(os.path.join(HERE, "export"), NAME, "base"))
        parts.write_catalog(os.path.join(HERE, "export", f"{NAME}.parts.ron"), NAME, made)
    rig.use_table(None)
    mesh.PATH_DETAIL = 1


if __name__ == "__main__":
    main()
