# SPDX-License-Identifier: AGPL-3.0-or-later
"""A ranger's quiver (22 §4; Vesper's): a leather quiver slung diagonally across the back, from
the left hip to above the right shoulder, with steel bands and a sheaf of arrows whose teal
fletching glows over the shoulder, on a strap that runs over the right shoulder, across the
chest and under the left arm. Modelled here around the reference body (a hard object: there is
nothing to simulate); the strap is laid on the body's own surface.

    blender -b --python art/wardrobe/back/ranger_quiver/build.py
"""

import math
import os
import random
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "..", "tools", "blender")))

import bmesh  # noqa: E402
import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402
from mathutils.bvhtree import BVHTree  # noqa: E402

from mftr_blender import cc0_body, mesh  # noqa: E402

LEATHER = (0.36, 0.22, 0.12)
STRAP = (0.20, 0.12, 0.07)
STEEL = (0.55, 0.58, 0.62)
WOOD = (0.30, 0.17, 0.08)
TEAL = (0.20, 0.95, 0.85)

# The quiver's line, in the reference body's frame (her left is +x, her back +y): from the
# bottom at the left hip to the mouth above the right shoulder blade.
BOTTOM = Vector((0.13, 0.0, 0.92))
TOP = Vector((-0.13, 0.0, 1.50))
# Its radius at the bottom and the mouth, and how far its middle stands off the back.
R0, R1 = 0.042, 0.052
STANDOFF = 0.068
SIDES = 10
# The strap: its width, and how far off the skin (over a tunic).
STRAP_W, STRAP_OFF = 0.034, 0.024


def back_y(tree, x, z):
    """The back's surface (its y) at (x, z): a ray from behind."""
    hit, _n, _i, _d = tree.ray_cast(Vector((x, 1.0, z)), Vector((0, -1, 0)))
    return hit.y if hit is not None else 0.12


def frame(axis):
    """Two directions across `axis`: toward the back (+y-ish), and the third."""
    across = axis.cross(Vector((0, 1, 0))).normalized()
    out = across.cross(axis).normalized()
    if out.y < 0:
        out = -out
    return out, across


def ring(bm, c, axis, r, n=SIDES, twist=0.0):
    out, across = frame(axis)
    return [bm.verts.new(c + (out * math.cos(twist + 2 * math.pi * k / n) + across * math.sin(twist + 2 * math.pi * k / n)) * r)
            for k in range(n)]


def bridge(bm, a, b, color, slot, faces):
    n = len(a)
    for k in range(n):
        f = bm.faces.new((a[k], a[(k + 1) % n], b[(k + 1) % n], b[k]))
        faces.append((f, color, slot))


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    bme = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in bme.vertices], [tuple(p.vertices) for p in bme.polygons])
    bm = bmesh.new()
    faces = []
    # The quiver's centre line, standing off the back.
    b = BOTTOM.copy()
    t = TOP.copy()
    b.y = back_y(tree, b.x, b.z) + STANDOFF
    t.y = back_y(tree, t.x, min(t.z, 1.42)) + STANDOFF * 1.1
    axis = (t - b).normalized()
    length = (t - b).length
    # The tube: closed at the bottom, open at the mouth (a short inner wall), its rings tapering.
    rings = []
    steps = 6
    for i in range(steps + 1):
        s = i / steps
        rings.append(ring(bm, b + axis * length * s, axis, R0 + (R1 - R0) * s))
    for a_, b_ in zip(rings, rings[1:]):
        bridge(bm, a_, b_, LEATHER, "cloth", faces)
    tip = bm.verts.new(b - axis * 0.012)
    for k in range(SIDES):
        f = bm.faces.new((rings[0][(k + 1) % SIDES], rings[0][k], tip))
        faces.append((f, LEATHER, "cloth"))
    inner = ring(bm, t - axis * 0.03, axis, R1 * 0.85)
    bridge(bm, inner, rings[-1], LEATHER, "cloth", faces)
    # Steel bands: a little proud of the leather.
    for s in (0.12, 0.55, 0.94):
        c = b + axis * length * s
        r = (R0 + (R1 - R0) * s) * 1.12
        lo, hi = ring(bm, c - axis * 0.012, axis, r), ring(bm, c + axis * 0.012, axis, r)
        bridge(bm, lo, hi, STEEL, "metal", faces)
        bridge(bm, hi, ring(bm, c + axis * 0.012, axis, r * 0.9), STEEL, "metal", faces)
        bridge(bm, ring(bm, c - axis * 0.012, axis, r * 0.9), lo, STEEL, "metal", faces)
    # The arrows: shafts out of the mouth, fanning a little, each fletched in two crossed vanes.
    rng = random.Random(4)
    out, across = frame(axis)
    for k in range(7):
        a = 2 * math.pi * k / 7
        off = (out * math.cos(a) + across * math.sin(a)) * R1 * 0.5
        lean = (out * math.cos(a) + across * math.sin(a)) * 0.08
        d = (axis + lean).normalized()
        base = t - axis * 0.05 + off
        rise = 0.17 + rng.random() * 0.05
        end = base + d * rise
        sa = ring(bm, base, d, 0.005, 3)
        sb = ring(bm, end, d, 0.005, 3)
        bridge(bm, sa, sb, WOOD, "cloth", faces)
        cap = bm.verts.new(end + d * 0.004)
        for j in range(3):
            faces.append((bm.faces.new((sb[j], sb[(j + 1) % 3], cap)), WOOD, "cloth"))
        # Two crossed vanes, back from the nock.
        o2, a2 = frame(d)
        for side in (o2, a2):
            p0, p1 = end - d * 0.075, end - d * 0.01
            w0, w1 = side * 0.022, side * 0.012
            vs = [bm.verts.new(p0 - w0), bm.verts.new(p1 - w1), bm.verts.new(p1 + w1), bm.verts.new(p0 + w0)]
            faces.append((bm.faces.new(vs), TEAL, "emissive"))
            faces.append((bm.faces.new(list(reversed([bm.verts.new(v.co) for v in vs]))), TEAL, "emissive"))
    # The strap: round the torso in the plane of the quiver's line and the body's depth, laid on
    # the skin from inside (rays out from the torso's middle).
    centre = Vector((0.0, 0.0, (BOTTOM.z + TOP.z) * 0.5))
    centre.y = (back_y(tree, 0.0, centre.z) - 0.11)
    u = axis
    v = Vector((0, 1, 0))
    v = (v - u * v.dot(u)).normalized()
    normal = u.cross(v).normalized()
    pts = []
    for k in range(48):
        th = 2 * math.pi * k / 48
        d = (u * math.cos(th) + v * math.sin(th)).normalized()
        hit, nrm, _i, _d = tree.ray_cast(centre, d, 1.0)
        if hit is None:
            continue
        # Outward: away from the torso's middle.
        if nrm.dot(d) < 0:
            nrm = -nrm
        pts.append((hit + nrm * STRAP_OFF, nrm))
    lo, hi = [], []
    for p, _n in pts:
        lo.append(bm.verts.new(p - normal * STRAP_W * 0.5))
        hi.append(bm.verts.new(p + normal * STRAP_W * 0.5))
    m = len(lo)
    for k in range(m):
        k2 = (k + 1) % m
        f = bm.faces.new((lo[k], lo[k2], hi[k2], hi[k]))
        faces.append((f, STRAP, "cloth"))
    # Colors and slots.
    col = bm.verts.layers.float_color.new("Col")
    slots = mesh.MATERIAL_SLOTS
    for f, c, slot in faces:
        f.material_index = slots.index(slot)
        f.smooth = False
    # Each piece's own color: split where colors meet.
    for f, c, _slot in faces:
        for vv in f.verts:
            vv[col] = (*c, 1.0)
    bmesh.ops.recalc_face_normals(bm, faces=[f for f, _c, _s in faces if f.is_valid and _s != "emissive"])
    me = bpy.data.meshes.new("item")
    bm.to_mesh(me)
    bm.free()
    for s in slots:
        mat = bpy.data.materials.get(s) or bpy.data.materials.new(s)
        me.materials.append(mat)
    obj = bpy.data.objects.new("item", me)
    bpy.context.scene.collection.objects.link(obj)
    me.color_attributes.active_color = me.color_attributes["Col"]
    bpy.data.objects.remove(body)
    bpy.ops.outliner.orphans_purge(do_recursive=True)
    tris = sum(len(p.vertices) - 2 for p in me.polygons)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.join(HERE, "item.blend"), compress=True)
    print(f"item: {tris} triangles")


build()
