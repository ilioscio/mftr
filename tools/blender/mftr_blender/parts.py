# SPDX-License-Identifier: AGPL-3.0-or-later
"""The champion creator's clothes and accessories (CR4, 20 §7): catalog parts made for a base.

Each part is its own mesh (`part.<slot>.<id>`) on the base's skeleton, exported with the base, so
the forge (`mftr-forge`) can put any of them on. A part follows the body's sliders: its shape
keys are the body's, carried over from the nearest body vertices (a block-out of the transfer an
artist does with a surface deform). `PARTS` lists them with their slot, a name, the body bones
whose triangles they hide (the undersuit beneath a breastplate), and whether they show the team
accent; `write_catalog` writes that list next to the export (`<base>.parts.ron`).

Parts are block-outs too (D49): lofts, boxes and shells on the base's bones and head, in the
neutral palette the paint step (CR5) recolors.
"""

import math

import bmesh
import bpy
from mathutils import Vector
from mathutils.kdtree import KDTree

from . import head as headmod
from . import mesh, rig

LEATHER = (0.36, 0.22, 0.12)
LEATHER_DARK = (0.20, 0.12, 0.07)
CLOTH = (0.42, 0.40, 0.36)
CLOTH_DARK = (0.22, 0.21, 0.24)
CLOTH_RED = (0.45, 0.10, 0.08)
CLOTH_BLUE = (0.14, 0.18, 0.36)
FUR = (0.48, 0.40, 0.30)
IRON = (0.42, 0.43, 0.47)
GOLD = (0.66, 0.50, 0.20)
WOOD = (0.32, 0.20, 0.10)
HAIR = (0.24, 0.16, 0.11)
ACCENT = (0.18, 0.52, 0.95)
GLOW = (0.55, 0.85, 1.0)

TORSO_BONES = ["pelvis", "spine_01", "spine_02", "chest"]
LEG_BONES = ["thigh_l", "thigh_r", "calf_l", "calf_r"]


def P(bone, t, off=(0, 0, 0)):  # noqa: N802
    return (bone, t, off)


def loft(bone, a, b, sides, profile, slot, color):
    return (bone, a, b, sides, profile, slot, color)


def pair(part):
    """`part` and its mirror (left-side lofts)."""
    return [part, mesh._mirror_part(part)]


# --- Head shells ---------------------------------------------------------------------------------

def _head_shell(head, line, push, color, slot, top_lift=0.012, thick=True):
    """A shell over the skull from a `line(angle) -> t` up to the crown, pushed out by `push`."""
    def build(bm, layers, groups, mats):
        deform, col = layers
        g = groups["head"]
        sides = head.sides
        angles = [2 * math.pi * (i + 0.5) / sides for i in range(sides)]

        def out(p, k):
            c = head.base + Vector((0, 0.01, (p - head.base).z))
            return c + (p - c) * k + Vector((0, 0, 0.004))

        steps = [0.0, 0.2, 0.42, 0.62, 0.8, 0.93]
        rings = []
        for s in steps:
            ring = []
            for a in angles:
                t = line(a) + s * (0.985 - line(a))
                ring.append(bm.verts.new(out(head.surface(t, a), push + 0.03 * math.sin(math.pi * s))))
            rings.append(ring)
        top = bm.verts.new(head.surface(1.0, 0) + Vector((0, 0, top_lift)))
        faces = []
        for r0, r1 in zip(rings, rings[1:]):
            for i in range(sides):
                faces.append(bm.faces.new((r0[i], r0[(i + 1) % sides], r1[(i + 1) % sides], r1[i])))
        for i in range(sides):
            faces.append(bm.faces.new((rings[-1][i], rings[-1][(i + 1) % sides], top)))
        verts = [v for r in rings for v in r] + [top]
        if thick:
            inner = [bm.verts.new(out(head.surface(line(a), a), 1.0)) for a in angles]
            for i in range(sides):
                faces.append(bm.faces.new((inner[i], inner[(i + 1) % sides], rings[0][(i + 1) % sides], rings[0][i])))
            verts += inner
        for v in verts:
            v[deform][g] = 1.0
            v[col] = (*color, 1.0)
        for f in faces:
            f.material_index = mats[slot]
            f.smooth = False
    return build


def _line(front, side, back):
    def line(a):
        f = max(0.0, -math.sin(a))
        b = max(0.0, math.sin(a))
        return back * b + front * f + side * (1 - f - b)
    return line


def _hair(head, style, color):
    def build(bm, layers, groups, mats):
        h = headmod.Head(base=head.base, height=head.height / head.scale, scale=head.scale, hair=color, hair_style=style,
                         detail=head.detail)
        headmod.hair(bm, layers, groups, mats, h)
    return build


# --- The catalog ---------------------------------------------------------------------------------

def _hand_axis(side):
    """The hand bone's head, its direction, a direction across it, and its length (the current
    shape's)."""
    bones = {b[0]: b for b in rig.bone_table()}
    h, t = Vector(bones[f"hand_{side}"][2]), Vector(bones[f"hand_{side}"][3])
    axis = (t - h).normalized()
    across = axis.cross(Vector((0, 1, 0))).normalized()
    return h, axis, across, (t - h).length


def catalog(head):
    """(id, slot, name, hides, accent, parts) for every part, on `head` (the base's Head)."""
    hc = head.base + Vector((0, 0.01, head.height * 0.55))
    k = head.scale
    out = []

    def add(pid, slot, name, items, hides=(), accent=False):
        out.append((pid, slot, name, list(hides), accent, items))

    # Hair (20 §5.2: picked on the face step too).
    add("hair.short", "hair", "Short hair", [_hair(head, "short", HAIR)])
    add("hair.ponytail", "hair", "Ponytail", [_hair(head, "ponytail", HAIR)])
    add("hair.topknot", "hair", "Topknot", [
        _head_shell(head, _line(0.82, 0.55, 0.40), 1.05, HAIR, "cloth"),
        mesh.path_part("head", [tuple(hc + Vector((0, 0.03, 0.14 * k))), tuple(hc + Vector((0, 0.04, 0.24 * k)))], 6,
                       [(0.05, 0.05), (0.02, 0.02)], "cloth", HAIR)])
    add("hair.mohawk", "hair", "Crest", [
        mesh.path_part("head", [tuple(hc + Vector((0, -0.09 * k, 0.11 * k))), tuple(hc + Vector((0, 0.0, 0.16 * k))),
                                tuple(hc + Vector((0, 0.10 * k, 0.10 * k)))], 4, [(0.014, 0.05), (0.016, 0.07), (0.012, 0.04)],
                       "cloth", HAIR)])
    # Head.
    add("head.hood", "head", "Hood", [
        _head_shell(head, _line(0.80, 0.30, 0.10), 1.22, CLOTH_DARK, "cloth"),
        loft("neck", P("neck", 0.0, (0, 0.02, -0.02)), P("neck", 1.0, (0, 0.03, 0.0)), 10,
             [(0, 0.13, 0.12), (1, 0.12, 0.11)], "cloth", CLOTH_DARK)])
    add("head.helmet", "head", "Helmet", [
        _head_shell(head, _line(0.72, 0.42, 0.30), 1.18, IRON, "metal"),
        mesh.box_part("head", tuple(head.front(0.62, 0) + Vector((0, -0.03, 0))), (0.022, 0.03, 0.09 * k), "metal", IRON)])
    add("head.wide_hat", "head", "Wide hat", [
        loft("head", tuple(hc + Vector((0, 0, 0.07 * k))), tuple(hc + Vector((0, 0, 0.085 * k))), 16,
             [(0, 0.27 * k, 0.27 * k), (1, 0.27 * k, 0.27 * k)], "cloth", CLOTH_BLUE),
        loft("head", tuple(hc + Vector((0, 0, 0.08 * k))), tuple(hc + Vector((0, 0.03, 0.30 * k))), 12,
             [(0, 0.12 * k, 0.12 * k), (0.6, 0.09 * k, 0.09 * k), (1, 0, 0)], "cloth", CLOTH_BLUE)])
    add("head.circlet", "head", "Circlet", [
        loft("head", tuple(hc + Vector((0, 0, 0.035 * k))), tuple(hc + Vector((0, 0, 0.055 * k))), 16,
             [(0, 0.112 * k, 0.118 * k), (1, 0.11 * k, 0.116 * k)], "metal", GOLD),
        mesh.box_part("head", tuple(head.front(0.79, 0) + Vector((0, -0.012, 0))), (0.03, 0.012, 0.03), "emissive", GLOW,
                      rot=(0, 45, 0))])
    add("head.horns", "head", "Horns", [
        mesh.path_part("head", [tuple(hc + Vector((s * 0.07 * k, 0, 0.08 * k))), tuple(hc + Vector((s * 0.14 * k, 0.02, 0.16 * k))),
                                tuple(hc + Vector((s * 0.15 * k, 0.07, 0.24 * k)))], 6, [(0.03, 0.03), (0.022, 0.022), (0.0, 0.0)],
                       "cloth", (0.62, 0.56, 0.44)) for s in (1, -1)])
    # Face.
    add("face.mask", "face", "Mask", [_head_shell(head, lambda a: 0.12, 1.07, CLOTH_DARK, "cloth", thick=False)])
    add("face.goggles", "face", "Goggles", [
        mesh.box_part("head", tuple(head.front(0.585, s * 0.034 * k) + Vector((0, -0.012, 0))), (0.04, 0.02, 0.035), "metal", GOLD)
        for s in (1, -1)] + [
        loft("head", tuple(hc + Vector((0, 0, 0.0))), tuple(hc + Vector((0, 0, 0.015))), 16,
             [(0, 0.11 * k, 0.115 * k), (1, 0.11 * k, 0.115 * k)], "cloth", LEATHER_DARK)])
    add("face.beard", "face", "Beard", [
        mesh.path_part("head", [tuple(head.front(0.30, 0) + Vector((0, 0.004, 0))), tuple(head.front(0.13, 0) + Vector((0, -0.012, 0))),
                                tuple(head.front(0.0, 0) + Vector((0, -0.02, -0.06)))], 6,
                       [(0.075 * k, 0.03), (0.065 * k, 0.04), (0.0, 0.0)], "cloth", HAIR, ref=(1, 0, 0))])
    # Torso.
    sash = loft("pelvis", P("pelvis", 0.45), P("pelvis", 0.85), 16, [(0, 0.205, 0.155), (1, 0.205, 0.155)], "accent", ACCENT)
    add("torso.tunic", "torso", "Tunic", [
        loft("chest", P("spine_01", 0.0), P("chest", 1.0), 16,
             [(0, 0.19, 0.14), (0.4, 0.215, 0.15), (0.8, 0.23, 0.155), (1, 0.15, 0.11)], "cloth", CLOTH),
        loft("pelvis", P("pelvis", 0.9), P("pelvis", 0.0, (0, 0, -0.32)), 16,
             [(0, 0.205, 0.15), (1, 0.25, 0.19)], "cloth", CLOTH),
        sash], hides=TORSO_BONES, accent=True)
    add("torso.robe", "torso", "Robe", [
        loft("chest", P("spine_01", 0.0), P("chest", 1.0), 16,
             [(0, 0.19, 0.14), (0.4, 0.21, 0.15), (0.8, 0.22, 0.15), (1, 0.15, 0.11)], "cloth", CLOTH_BLUE),
        loft("pelvis", P("pelvis", 0.9), P("pelvis", 0.0, (0, 0, -0.86)), 16,
             [(0, 0.205, 0.15), (0.5, 0.26, 0.21), (1, 0.32, 0.27)], "cloth", CLOTH_BLUE),
        sash], hides=TORSO_BONES + LEG_BONES, accent=True)
    add("torso.breastplate", "torso", "Breastplate", [
        loft("chest", P("spine_02", 0.0), P("chest", 0.95), 16,
             [(0, 0.20, 0.15), (0.5, 0.235, 0.17), (1, 0.21, 0.15)], "metal", IRON),
        loft("spine_01", P("spine_01", 0.1), P("spine_01", 0.9), 16, [(0, 0.19, 0.14), (1, 0.2, 0.145)], "metal", IRON)],
        hides=["spine_01", "spine_02", "chest"])
    add("torso.coat", "torso", "Long coat", [
        loft("chest", P("spine_01", 0.0), P("chest", 1.0), 16,
             [(0, 0.2, 0.145), (0.5, 0.225, 0.155), (0.85, 0.235, 0.16), (1, 0.16, 0.12)], "cloth", CLOTH_DARK),
        mesh.box_part("pelvis", P("pelvis", 0.0, (0, 0.10, -0.30)), (0.40, 0.04, 0.70), "cloth", CLOTH_DARK, rot=(-8, 0, 0)),
        *mesh.box_pair("pelvis", P("pelvis", 0.0, (0.17, 0.0, -0.30)), (0.05, 0.22, 0.70), "cloth", CLOTH_DARK, rot=(0, -6, 0)),
        sash], hides=TORSO_BONES, accent=True)
    # Shoulders.
    add("shoulders.pauldrons", "shoulders", "Pauldrons", pair(
        loft("upperarm_l", P("clavicle_l", 0.8, (0, 0, -0.04)), P("clavicle_l", 1.0, (0.10, 0, 0.10)), 10,
             [(0, 0.13, 0.13), (0.4, 0.15, 0.15), (0.8, 0.12, 0.12), (1, 0, 0)], "metal", IRON)))
    add("shoulders.mantle", "shoulders", "Mantle", [
        loft("chest", P("chest", 1.0, (0, 0, 0.04)), P("chest", 0.3), 16,
             [(0, 0.12, 0.10), (0.5, 0.30, 0.20), (1, 0.34, 0.22)], "cloth", CLOTH_RED)])
    add("shoulders.fur", "shoulders", "Fur collar", [
        loft("chest", P("chest", 0.8), P("chest", 1.0, (0, 0, 0.04)), 12,
             [(0, 0.24, 0.17), (0.5, 0.21, 0.16), (1, 0.12, 0.10)], "cloth", FUR)])
    # Arms.
    add("arms.bracers", "arms", "Bracers", pair(
        loft("forearm_l", P("forearm_l", 0.35), P("forearm_l", 0.95), 8, [(0, 0.075, 0.075), (1, 0.068, 0.068)], "cloth", LEATHER))
        + pair(loft("forearm_l", P("forearm_l", 0.55), P("forearm_l", 0.65), 8, [(0, 0.08, 0.08), (1, 0.08, 0.08)], "accent", ACCENT)),
        accent=True)
    add("arms.gauntlets", "arms", "Gauntlets", pair(
        loft("forearm_l", P("forearm_l", 0.5), P("forearm_l", 1.0), 8, [(0, 0.07, 0.07), (1, 0.085, 0.085)], "metal", IRON))
        + pair(loft("hand_l", P("hand_l", 0.0), P("hand_l", 1.0), 6, [(0, 0.05, 0.035), (1, 0.055, 0.03)], "metal", IRON)))
    # Waist.
    add("waist.belt", "waist", "Belt and pouch", [
        loft("pelvis", P("pelvis", 0.55), P("pelvis", 0.85), 16, [(0, 0.2, 0.15), (1, 0.2, 0.15)], "cloth", LEATHER_DARK),
        mesh.box_part("pelvis", P("pelvis", 0.6, (0.17, -0.06, -0.03)), (0.08, 0.06, 0.09), "cloth", LEATHER),
        mesh.box_part("pelvis", P("pelvis", 0.7, (0, -0.15, 0)), (0.05, 0.02, 0.04), "metal", GOLD)])
    add("waist.skirt", "waist", "Strip skirt", [
        mesh.box_part("pelvis", P("pelvis", 0.0, (math.sin(a) * 0.19, -math.cos(a) * 0.15, -0.16)), (0.08, 0.02, 0.32), "cloth",
                      LEATHER if i % 2 else CLOTH_RED, rot=(-12 * math.cos(a), 12 * math.sin(a), -math.degrees(a)))
        for i, a in enumerate([2 * math.pi * j / 10 for j in range(10)])])
    # Legs and feet.
    add("legs.greaves", "legs", "Greaves", pair(
        loft("calf_l", P("calf_l", 0.1, (0, -0.02, 0)), P("calf_l", 0.75, (0, -0.02, 0)), 6,
             [(0, 0.08, 0.06), (1, 0.07, 0.055)], "metal", IRON)))
    add("feet.boots", "feet", "Tall boots", pair(
        loft("calf_l", P("calf_l", 0.35), P("calf_l", 1.0), 10, [(0, 0.085, 0.085), (1, 0.075, 0.08)], "cloth", LEATHER))
        + pair(loft("foot_l", P("foot_l", 0.0, (0, 0, -0.01)), P("toe_l", 1.0, (0, 0, -0.01)), 6,
                    [(0, 0.07, 0.06), (0.6, 0.065, 0.045), (1, 0.05, 0.03)], "cloth", LEATHER)),
        hides=["foot_l", "foot_r", "toe_l", "toe_r"])
    # Back.
    add("back.cape", "back", "Cape", [
        mesh.box_part("chest", P("chest", 0.8, (0, 0.17, -0.45)), (0.46, 0.03, 0.95), "cloth", CLOTH_RED, rot=(-6, 0, 0), taper=0.8)])
    add("back.quiver", "back", "Quiver", [
        mesh.box_part("chest", P("chest", 0.3, (0.05, 0.17, 0.0)), (0.10, 0.10, 0.50), "cloth", LEATHER, rot=(0, -25, 0)),
        mesh.box_part("chest", P("chest", 0.3, (0.13, 0.17, 0.26)), (0.09, 0.08, 0.10), "emissive", GLOW, rot=(0, -25, 0))])
    # Weapons (20 §7.3: a class each; moves name the classes they take, CR6).
    # Weapons are laid along the hand that holds them, like the official champions' (Rook's maul,
    # Lumen's staff): the fist closes on the grip, so the move's clips aim them as they aim theirs.
    h, axis, across, hand_len = _hand_axis("r")
    g = h + axis * (0.55 * hand_len)

    def at(d, off=Vector()):
        return tuple(g + axis * d + off)

    add("weapon.sword", "weapon", "Sword", [
        mesh.path_part("hand_r", [at(-0.12), at(0.02)], 6, [(0.03, 0.03), (0.03, 0.03)], "cloth", LEATHER_DARK, ref=tuple(across)),
        mesh.path_part("hand_r", [at(0.02, across * -0.10), at(0.02, across * 0.10)], 4, [(0.025, 0.03)] * 2, "metal", GOLD,
                       ref=tuple(axis)),
        mesh.path_part("hand_r", [at(0.04), at(0.80), at(0.92)], 4, [(0.04, 0.012), (0.035, 0.01), (0.0, 0.0)], "metal", IRON,
                       ref=tuple(across))])
    add("weapon.staff", "weapon", "Staff", [
        mesh.path_part("hand_r", [at(-0.62), at(1.0)], 6, [(0.024, 0.024), (0.028, 0.028)], "cloth", WOOD, ref=tuple(across)),
        mesh.path_part("hand_r", [at(1.0), at(1.08), at(1.16)], 6, [(0.03, 0.03), (0.07, 0.07), (0.0, 0.0)], "emissive", GLOW,
                       ref=tuple(across))])
    add("weapon.maul", "weapon", "Maul", [
        mesh.path_part("hand_r", [at(-0.28), at(1.16)], 6, [(0.036, 0.036), (0.04, 0.04)], "cloth", WOOD, ref=tuple(across)),
        mesh.path_part("hand_r", [at(1.30, across * -0.26), at(1.30, across * 0.26)], 4, [(0.22, 0.22)] * 2, "metal", IRON,
                       ref=tuple(axis))])
    add("weapon.dagger", "weapon", "Dagger", [
        mesh.path_part("hand_r", [at(0.0), at(0.38), at(0.46)], 4, [(0.034, 0.01), (0.03, 0.008), (0.0, 0.0)], "metal", IRON,
                       ref=tuple(across))])
    add("offhand.shield", "offhand", "Shield", [
        mesh.box_part("forearm_l", P("forearm_l", 0.6, (0.07, -0.02, 0)), (0.04, 0.40, 0.50), "metal", IRON, bevel=0.3),
        mesh.box_part("forearm_l", P("forearm_l", 0.6, (0.095, -0.02, 0)), (0.02, 0.16, 0.16), "accent", ACCENT, rot=(45, 0, 0))],
        accent=True)
    add("offhand.bow", "offhand", "Bow", [
        mesh.path_part("hand_l", [P("prop_l", 0.0, (0, 0, 0.75)), P("prop_l", 0.0, (0, -0.12, 0.35)), P("prop_l", 0.0, (0, 0, 0)),
                                  P("prop_l", 0.0, (0, -0.12, -0.35)), P("prop_l", 0.0, (0, 0, -0.75))], 6,
                       [(0.015, 0.015), (0.025, 0.025), (0.03, 0.03), (0.025, 0.025), (0.015, 0.015)], "cloth", WOOD)])
    add("offhand.orb", "offhand", "Orb", [
        mesh.box_part("hand_l", P("prop_l", 0.0, (0, -0.06, 0.08)), (0.10, 0.10, 0.10), "emissive", GLOW, rot=(45, 35, 0))])

    # Heavy gear (CR8, 20 §12: Rook's, Bastion's and Cairn's looks). Every base gets these too.
    face_y = hc.y - 0.145 * k
    add("head.great_helm", "head", "Great helm", [
        # A block that stands up out of the gorget: the slit at the eyes, a crest on top.
        # (the slit glows in the champion's signature hue, like Bastion's visor)
        mesh.box_part("head", tuple(hc + Vector((0, -0.015, 0.0))), (0.28 * k, 0.29 * k, 0.34 * k), "metal", IRON, bevel=0.3,
                      taper=0.92),
        mesh.box_part("head", (0, face_y - 0.026, head.front(0.62, 0).z - 0.02), (0.20 * k, 0.02, 0.036), "emissive", GLOW),
        mesh.box_part("head", tuple(hc + Vector((0, -0.015, 0.17 * k))), (0.04, 0.30 * k, 0.05), "accent", ACCENT)],
        hides=["head"], accent=True)
    add("shoulders.slabs", "shoulders", "Slab pauldrons", [
        *mesh.box_pair("upperarm_l", P("clavicle_l", 1.0, (0.05, 0, 0.06)), (0.24, 0.27, 0.11), "metal", IRON, rot=(0, -22, 0),
                       bevel=0.3),
        *mesh.box_pair("upperarm_l", P("clavicle_l", 1.0, (0.09, 0, -0.02)), (0.20, 0.24, 0.08), "metal", IRON, rot=(0, -40, 0),
                       bevel=0.3)])
    stone, moss = (0.46, 0.44, 0.41), (0.34, 0.44, 0.22)
    shoulder_rock = mesh.rock_part("upperarm_l", P("clavicle_l", 1.0, (0.04, 0, 0.07)), (0.26, 0.24, 0.19), stone, moss=moss, seed=3)
    add("shoulders.boulders", "shoulders", "Stone shoulders", [shoulder_rock, mesh.mirror(shoulder_rock)])
    fist = mesh.rock_part("hand_l", P("hand_l", 0.3), (0.22, 0.2, 0.22), stone, seed=5, cuts=5)
    add("arms.stone_fists", "arms", "Stone fists", [fist, mesh.mirror(fist)],
        hides=["hand_l", "hand_r", "fingers_l", "fingers_r", "thumb_l", "thumb_r"])
    add("waist.tassets", "waist", "Plate tassets", [
        mesh.box_part("pelvis", P("pelvis", 0.0, (math.sin(a) * 0.2, -math.cos(a) * 0.16, -0.12)), (0.17, 0.03, 0.26), "metal",
                      IRON, rot=(-10 * math.cos(a), 10 * math.sin(a), -math.degrees(a)))
        for a in (-1.2, -0.45, 0.45, 1.2)] + [
        loft("pelvis", P("pelvis", 0.55), P("pelvis", 0.85), 16, [(0, 0.205, 0.155), (1, 0.205, 0.155)], "accent", ACCENT)],
        accent=True)
    add("offhand.tower_shield", "offhand", "Tower shield", [
        mesh.box_part("forearm_l", P("forearm_l", 0.6, (0.08, -0.02, 0)), (0.05, 0.56, 0.86), "metal", IRON, bevel=0.15),
        mesh.box_part("forearm_l", P("forearm_l", 0.6, (0.11, -0.02, 0)), (0.02, 0.08, 0.70), "accent", ACCENT)],
        accent=True)
    add("weapon.great_axe", "weapon", "Great axe", [
        mesh.path_part("hand_r", [at(-0.30), at(1.22)], 6, [(0.034, 0.034), (0.036, 0.036)], "cloth", WOOD, ref=tuple(across)),
        mesh.path_part("hand_r", [at(1.06, across * 0.02), at(1.06, across * 0.18), at(1.06, across * 0.34)], 4,
                       [(0.09, 0.02), (0.16, 0.014), (0.24, 0.008)], "metal", IRON, ref=tuple(axis)),
        mesh.path_part("hand_r", [at(1.06, across * -0.02), at(1.06, across * -0.12)], 4, [(0.05, 0.03), (0.0, 0.0)], "metal", IRON,
                       ref=tuple(axis))])
    return out


SLOTS = ["hair", "head", "face", "torso", "shoulders", "arms", "waist", "legs", "feet", "back", "weapon", "offhand"]


def transfer_shape_keys(body, part):
    """Give `part` the body's shape keys: each part vertex moves as the nearest body vertices do
    (inverse-distance weights of the three nearest). Only keys that move the part are made."""
    keys = body.data.shape_keys.key_blocks
    basis = [v.co.copy() for v in keys["Basis"].data]
    tree = KDTree(len(basis))
    for i, co in enumerate(basis):
        tree.insert(co, i)
    tree.balance()
    near = []
    for v in part.data.vertices:
        hits = tree.find_n(v.co, 3)
        ws = [(i, 1.0 / max(d, 1e-4)) for _, i, d in hits]
        total = sum(w for _, w in ws)
        near.append([(i, w / total) for i, w in ws])
    moved = {}
    for kb in keys:
        if kb.name == "Basis":
            continue
        deltas = []
        any_move = False
        for nv in near:
            d = Vector()
            for i, w in nv:
                d += (kb.data[i].co - basis[i]) * w
            any_move |= d.length > 1e-5
            deltas.append(d)
        if any_move:
            moved[kb.name] = deltas
    if not moved:
        return 0
    # Keep slider pairs whole: a part that moves with `x+` also gets `x-`.
    ids = {n[:-1] for n in moved}
    part.shape_key_add(name="Basis", from_mix=False)
    for sid in sorted(ids):
        for suffix in "+-":
            name = sid + suffix
            k = part.shape_key_add(name=name, from_mix=False)
            for v, d in zip(part.data.vertices, moved.get(name, [Vector()] * len(part.data.vertices))):
                k.data[v.index].co = v.co + d
    return len(ids)


def build_parts(armature, body, head):
    """Every catalog part as its own mesh under `armature`, following `body`'s sliders."""
    made = []
    for pid, slot, name, hides, accent, items in catalog(head):
        obj = mesh.build_mannequin(armature, name=f"part.{pid}", part_list=items)
        n = transfer_shape_keys(body, obj)
        made.append((pid, slot, name, hides, accent, mesh.triangle_count(obj), n))
    return made


def write_catalog(path, base, made):
    """`<base>.parts.ron`: what the forge and the creator know of each part."""
    lines = ["// Generated by tools/blender/mftr_blender (parts). The creator's parts for this base (20 §7).",
             "(", f'    base: "{base}",', "    parts: ["]
    for pid, slot, name, hides, accent, tris, _ in made:
        hide = ", ".join(f'"{h}"' for h in hides)
        lines.append(f'        (id: "{pid}", slot: "{slot}", name: "{name}", hides: [{hide}], accent: {"true" if accent else "false"}, triangles: {tris}),')
    lines += ["    ],", ")", ""]
    with open(path, "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(lines))
