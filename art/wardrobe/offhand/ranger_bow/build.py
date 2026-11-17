# SPDX-License-Identifier: AGPL-3.0-or-later
"""A ranger's bow (22 §4.7, BW7; Vesper's): a tall recurve, its limbs curving back toward the
archer and flicking forward at the tips, flat and wide like real limbs, a leather-wrapped grip
between silver bands with a teal stone set in its face, silver caps and teal-glowing tips (her
signature), and the string.

Modelled in its grip frame (`wardrobe._place_grip`): the grip at the origin, +Z the upper limb,
+Y its back (the string's side, toward her). `item.ron` puts it in the left hand as her full draw
holds it. It has the hand-made Vesper's bow bones, by name: two on each limb and one on each half
of the string, so her shots' baked draws bend it and pull the string to her fingers.

    blender -b --python art/wardrobe/offhand/ranger_bow/build.py
"""

import json
import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

from mftr_blender import mesh  # noqa: E402

WOOD = (0.34, 0.20, 0.10)
WOOD_DARK = (0.22, 0.12, 0.06)
LEATHER = (0.16, 0.10, 0.06)
SILVER = (0.70, 0.72, 0.76)
TEAL = (0.20, 0.95, 0.85)
STRING = (0.85, 0.82, 0.72)

HALF = 0.80              # grip to tip (m): a bow taller than her shoulders
BRACE, RECURVE = 0.20, 0.09
GRIP = 0.075             # the grip's half-length
SIDES = 8
# Where each limb's bones start (off the grip) and where the inner one hands over to the outer
# (the hand-made Vesper's: 0.07 and 0.45 of her 0.85).
LIMB_ROOT, LIMB_SPLIT = 0.07, 0.42


def centre(s):
    """The bow's centre line `s` m from the grip along the limbs."""
    u = abs(s) / HALF
    return Vector((0.0, BRACE * u * u - RECURVE * u ** 6, s))


def section(s):
    """Half-width (across, x) and half-thickness (front to back, y) at `s`."""
    a = abs(s)
    if a < GRIP:
        return 0.021, 0.028
    if a < GRIP + 0.06:
        # The riser flares into the limbs.
        t = (a - GRIP) / 0.06
        return 0.021 + 0.019 * t, 0.028 - 0.008 * t
    u = (a - GRIP - 0.06) / (HALF - GRIP - 0.06)
    return 0.040 - 0.024 * u, 0.020 - 0.010 * u


def paint(s):
    a = abs(s)
    if a < GRIP:
        return LEATHER, "cloth"
    if a < GRIP + 0.02:
        return SILVER, "metal"
    if a > HALF - 0.09:
        return TEAL, "emissive"
    if HALF - 0.13 < a <= HALF - 0.09:
        return SILVER, "metal"
    return WOOD, "cloth"


def tangent(s):
    d = 0.002
    return (centre(s + d) - centre(s - d)).normalized()


def ring(bm, s, scale=1.0):
    c, t = centre(s), tangent(s)
    side = Vector((1.0, 0.0, 0.0))
    back = t.cross(side).normalized()
    w, h = section(s)
    out = []
    for k in range(SIDES):
        a = 2 * math.pi * k / SIDES
        out.append(bm.verts.new(c + side * (math.cos(a) * w * scale) + back * (math.sin(a) * h * scale)))
    return out


def bridge(bm, a, b, color, slot, faces):
    n = len(a)
    for k in range(n):
        faces.append((bm.faces.new((a[k], a[(k + 1) % n], b[(k + 1) % n], b[k])), color, slot))


def cap(bm, r, point, color, slot, faces, flip=False):
    tip = bm.verts.new(point)
    n = len(r)
    for k in range(n):
        f = (r[(k + 1) % n], r[k], tip) if not flip else (r[k], r[(k + 1) % n], tip)
        faces.append((bm.faces.new(f), color, slot))


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    bm = bmesh.new()
    # The color layer first: adding a layer reallocates the vertices (keys taken before go stale).
    col = bm.verts.layers.float_color.new("Col")
    faces = []
    # The limbs and the riser: segments between stations, each its own rings (its own color).
    stations = [-HALF, -0.76, -0.71, -0.67, -0.60, -0.50, -0.38, -0.26, -0.17, -GRIP - 0.02, -GRIP, -0.025, 0.025,
                GRIP, GRIP + 0.02, 0.17, 0.26, 0.38, 0.50, 0.60, 0.67, 0.71, 0.76, HALF]
    for a, b in zip(stations, stations[1:]):
        color, slot = paint((a + b) / 2)
        ra, rb = ring(bm, a), ring(bm, b)
        bridge(bm, ra, rb, color, slot, faces)
        if a == -HALF:
            cap(bm, ra, centre(a) + tangent(a) * -0.03, color, slot, faces, flip=True)
        if b == HALF:
            cap(bm, rb, centre(b) + tangent(b) * 0.03, color, slot, faces)
    # Silver bands round the riser's ends and the limbs' caps: a little proud.
    for s in (-GRIP - 0.01, GRIP + 0.01, -(HALF - 0.11), HALF - 0.11):
        lo, hi = ring(bm, s - 0.008, 1.18), ring(bm, s + 0.008, 1.18)
        bridge(bm, lo, hi, SILVER, "metal", faces)
    # A teal stone in the riser's face (the side away from her), and a dark arrow shelf above
    # the grip on the same side.
    face = centre(0.0) + Vector((0.0, -0.029, 0.0))
    d = [face + Vector((0, 0, 0.03)), face + Vector((0.016, 0, 0)), face + Vector((0, 0, -0.03)), face + Vector((-0.016, 0, 0))]
    top = face + Vector((0.0, -0.012, 0.0))
    vs = [bm.verts.new(p) for p in d]
    t = bm.verts.new(top)
    for k in range(4):
        faces.append((bm.faces.new((vs[k], vs[(k + 1) % 4], t)), TEAL, "emissive"))
    shelf = centre(GRIP + 0.005) + Vector((-0.02, -0.01, 0.0))
    sv = [bm.verts.new(shelf + Vector(o)) for o in ((0, -0.012, 0), (0, 0.012, 0), (0, 0.012, 0.012), (0, -0.012, 0.012))]
    faces.append((bm.faces.new(sv), WOOD_DARK, "cloth"))
    faces.append((bm.faces.new([bm.verts.new(v.co) for v in reversed(sv)]), WOOD_DARK, "cloth"))
    # The string: tip to tip on her side, at the nocks just inside the tips, in two halves that
    # meet at the nocking point (each follows its own bone when she draws).
    top, bot = centre(HALF - 0.05) + Vector((0, 0.008, 0)), centre(-(HALF - 0.05)) + Vector((0, 0.008, 0))
    nock = (top + bot) / 2
    axis = (top - bot).normalized()
    across = Vector((1, 0, 0))
    out_ = axis.cross(across).normalized()

    def string_ring(c):
        return [bm.verts.new(c + (across * math.cos(2 * math.pi * k / 4) + out_ * math.sin(2 * math.pi * k / 4)) * 0.0035)
                for k in range(4)]

    string = {}
    for half, (a, b) in (("extra_string_2", (bot, nock)), ("extra_string_1", (nock, top))):
        ra, rb = string_ring(a), string_ring(b)
        bridge(bm, ra, rb, STRING, "cloth", faces)
        for v in ra + rb:
            string[v] = {half: 1.0}
    # Weights: the grip on the hand, each limb on two bones (the hand-made Vesper's: `extra_bow_1`
    # and `_2` up, `_3` and `_4` down), blended where they meet so it bends smoothly.
    def smooth(t):
        t = min(max(t, 0.0), 1.0)
        return t * t * (3 - 2 * t)

    weights = {}
    for v in bm.verts:
        if v in string:
            weights[v] = string[v]
            continue
        a = abs(v.co.z)
        inner, outer = ("extra_bow_1", "extra_bow_2") if v.co.z > 0 else ("extra_bow_3", "extra_bow_4")
        f1 = smooth((a - LIMB_ROOT + 0.02) / 0.04)
        f2 = smooth((a - LIMB_SPLIT + 0.05) / 0.10)
        weights[v] = {"grip": 1.0 - f1, inner: f1 * (1.0 - f2), outer: f1 * f2}
    # The bones, in the grip frame ("grip" is the hand that holds it): placed on the base with the
    # bow (`wardrobe._place_grip`). Named as the hand-made Vesper's, so her shots' baked draws
    # (the limbs bending toward her, the string to her fingers) play on it (`Retarget`, by name).
    bones = [
        ("extra_bow_1", centre(LIMB_ROOT), centre(LIMB_SPLIT), "grip"),
        ("extra_bow_2", centre(LIMB_SPLIT), centre(HALF), "extra_bow_1"),
        ("extra_bow_3", centre(-LIMB_ROOT), centre(-LIMB_SPLIT), "grip"),
        ("extra_bow_4", centre(-LIMB_SPLIT), centre(-HALF), "extra_bow_3"),
        ("extra_string_1", top, nock, "extra_bow_2"),
        ("extra_string_2", bot, nock, "extra_bow_4"),
    ]
    # Colors and slots.
    slots = mesh.MATERIAL_SLOTS
    for f, c, slot in faces:
        f.material_index = slots.index(slot)
        f.smooth = False
        for v in f.verts:
            v[col] = (*c, 1.0)
    me = bpy.data.meshes.new("item")
    for s in slots:
        me.materials.append(mesh.material(s))
    bm.verts.index_update()
    order = [(v.index, weights[v]) for v in bm.verts]
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new("item", me)
    bpy.context.scene.collection.objects.link(obj)
    me.color_attributes.active_color = me.color_attributes["Col"]
    for vi, ws in order:
        for name, w in ws.items():
            if w > 1e-4:
                g = obj.vertex_groups.get(name) or obj.vertex_groups.new(name=name)
                g.add([vi], w, "REPLACE")
    obj["mftr_bones"] = json.dumps([(n, list(a), list(b), p) for n, a, b, p in bones])
    tris = sum(len(p.vertices) - 2 for p in me.polygons)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(HERE, "item.blend"), compress=True)
    print(f"item: {tris} triangles")


build()
