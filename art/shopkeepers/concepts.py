# SPDX-License-Identifier: AGPL-3.0-or-later
"""Shopkeeper concepts (19 §6, K slice 1): block-outs of the four keepers and their stalls, and
the review sheets the owner approves before modeling starts.

    blender -b --python art/shopkeepers/concepts.py -- --out DIR

Throwaway geometry: lathes, tubes and boxes with flat vertex colors, no rig. What it settles is
silhouette, proportion, palette and the stall's contents. The real models come after approval,
on the `biped` bones (19 §3).

Colors are linear, like the game's vertex colors. Writes into DIR:
    lineup_silhouette.png   the four in black beside a lane minion and Rook
    lineup.png              the same in color (ally accents)
    <key>_stall.png         each keeper at their stall, 3/4 close-up
    base_gameplay.png       all four stalls at the gameplay camera, two ally and two enemy
"""

import math
import os
import sys

import bmesh
import bpy
from mathutils import Euler, Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))


def srgb(c):
    """An sRGB color (as Godot's Color) to linear."""
    f = lambda x: x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4
    return tuple(f(x) for x in c)


class Tone(tuple):
    """A color that knows its material slot (10 §2): `metal`, `emissive`, `accent` (the team
    color) or `accent_glow`. Plain tuples are `cloth`."""
    slot = "cloth"


def tone(c, slot):
    t = Tone(c)
    t.slot = slot
    return t


def met(c):
    return tone(c, "metal")


def emi(c):
    return tone(c, "emissive")


ALLY = srgb((0.3, 0.85, 0.8))      # main.gd ALLY_COLOR
ENEMY = srgb((0.95, 0.35, 0.25))   # main.gd ENEMY_COLOR
TEAM = ALLY                        # the team the next build paints in


def team(k=1.0):
    return tone(tuple(min(1.0, c * k) for c in TEAM), "accent")


def glow():
    return tone(tuple(min(1.0, c * 1.25 + 0.08) for c in TEAM), "accent_glow")


# Shared palette (linear).
VOID = (0.012, 0.012, 0.016)
CREAM = (0.72, 0.64, 0.50)
PARCH = (0.62, 0.50, 0.30)
GOLD = met((0.80, 0.52, 0.10))
GOLD_D = met((0.50, 0.30, 0.05))
BRASS = met((0.55, 0.38, 0.13))
IRON = met((0.07, 0.075, 0.085))
GLASS = met((0.55, 0.62, 0.70))
FLAME = emi((1.0, 0.70, 0.22))
OAK = (0.17, 0.085, 0.035)
OAK_D = (0.09, 0.045, 0.02)
BURLAP = (0.34, 0.24, 0.12)
STONE = (0.15, 0.16, 0.18)
STONE_D = (0.085, 0.095, 0.11)
MOSS = (0.06, 0.1, 0.035)


# --- Geometry ------------------------------------------------------------------------------

class B:
    """One object's bmesh with a float corner color, and a transform stack for sub-assemblies."""

    def __init__(self, name):
        self.name = name
        self.bm = bmesh.new()
        self.col = self.bm.loops.layers.float_color.new("Col")
        self.M = Matrix.Identity(4)
        self.stack = []
        self.cur = None   # the bone (or {bone: weight}) new vertices follow (previews.py)
        self.wts = []
        self.slots = []   # each face's material slot (build.py)

    def part(self, bone):
        """Tag the vertices that follow with a bone of the preview rigs (previews.py)."""
        self.cur = bone

    def push(self, loc=(0, 0, 0), rot=(0, 0, 0), scale=1.0):
        self.stack.append(self.M)
        r = Euler([math.radians(a) for a in rot]).to_matrix().to_4x4()
        self.M = self.M @ Matrix.Translation(Vector(loc)) @ r @ Matrix.Scale(scale, 4)

    def pop(self):
        self.M = self.stack.pop()

    def v(self, co):
        self.wts.append(self.cur)
        return self.bm.verts.new(self.M @ Vector(co))

    def face(self, verts, color):
        f = self.bm.faces.new(verts)
        self.slots.append(getattr(color, "slot", "cloth"))
        for l in f.loops:
            l[self.col] = (*color, 1.0)
        return f

    def done(self, collection):
        me = bpy.data.meshes.new(self.name)
        self.bm.to_mesh(me)
        self.bm.free()
        me.color_attributes.active_color_name = "Col"
        ob = bpy.data.objects.new(self.name, me)
        collection.objects.link(ob)
        if any(w is not None for w in self.wts):
            groups = {}
            for i, w in enumerate(self.wts):
                for bone, k in ({w: 1.0} if isinstance(w, str) else (w or {"root": 1.0})).items():
                    if bone not in groups:
                        groups[bone] = ob.vertex_groups.new(name=bone)
                    groups[bone].add([i], k, "REPLACE")
        return ob


DETAIL = 1.0


def _more(n):
    return n if n < 6 or DETAIL == 1.0 else max(n, int(round(n * DETAIL)))


def lathe(b, prof, sides, color, at=(0, 0, 0), rot=(0, 0, 0), sx=1.0, sy=1.0, shift=None, rmod=None, cap=True, phase=0.0, bone=None):
    """A surface of revolution about local Z. `prof` is [(z, r)] bottom to top; r = 0 closes an
    end to a point. `color` is a color or fn(angle, z). `shift(z)` -> (dx, dy) bends the axis.
    `bone` tags vertices for the preview rigs: fn(angle, z) or one per profile point."""
    b.push(at, rot)
    sides = _more(sides)
    saved = b.cur
    rings = []
    for k, (z, r) in enumerate(prof):
        dx, dy = shift(z) if shift else (0.0, 0.0)
        if isinstance(bone, (list, tuple)):
            b.cur = bone[k]
        if r < 1e-6:
            if callable(bone):
                b.cur = bone(0.0, z)
            rings.append([b.v((dx, dy, z))] * sides)
            continue
        ring = []
        for i in range(sides):
            a = 2 * math.pi * i / sides + phase
            if callable(bone):
                b.cur = bone(a % (2 * math.pi), z)
            f = rmod(a, z) if rmod else 1.0
            ring.append(b.v((r * f * math.cos(a) * sx + dx, r * f * math.sin(a) * sy + dy, z)))
        rings.append(ring)
    col = color if callable(color) else (lambda a, z: color)
    for k in range(len(rings) - 1):
        r0, r1 = rings[k], rings[k + 1]
        zm = (prof[k][0] + prof[k + 1][0]) / 2
        for i in range(sides):
            j = (i + 1) % sides
            am = 2 * math.pi * (i + 0.5) / sides + phase
            vs = [r0[i], r0[j], r1[j], r1[i]]
            uniq = []
            for x in vs:
                if x not in uniq:
                    uniq.append(x)
            if len(uniq) >= 3:
                b.face(uniq, col(am % (2 * math.pi), zm))
    if cap:
        if prof[0][1] > 1e-6:
            b.face(list(reversed(rings[0])), col(0.0, prof[0][0]))
        if prof[-1][1] > 1e-6:
            b.face(rings[-1], col(0.0, prof[-1][0]))
    b.cur = saved
    b.pop()


def sphere(b, c, r, color, sides=10, rings=6, s=(1, 1, 1), rot=(0, 0, 0)):
    rings = rings if rings < 4 or DETAIL == 1.0 else max(rings, int(round(rings * (1 + (DETAIL - 1) * 0.6))))
    prof = [(-math.cos(math.pi * k / rings) * r * s[2], math.sin(math.pi * k / rings) * r) for k in range(rings + 1)]
    lathe(b, prof, sides, color, at=c, rot=rot, sx=s[0], sy=s[1])


def donut(b, c, R, rt, color, sides=14, tsides=6, rot=(0, 0, 0), rmod=None):
    prof = [(math.sin(2 * math.pi * k / tsides) * rt, R + math.cos(2 * math.pi * k / tsides) * rt) for k in range(tsides + 1)]
    lathe(b, prof, sides, color, at=c, rot=rot, cap=False, rmod=rmod)


def disc(b, c, r, h, color, sides=12, rot=(0, 0, 0), rmod=None):
    """A flat cylinder centered on c, its axis along local Z."""
    lathe(b, [(-h / 2, r), (h / 2, r)], sides, color, at=c, rot=rot, rmod=rmod)


def cyl(b, a, z0, z1, r, color, sides=10, r1=None):
    lathe(b, [(z0, r), (z1, r if r1 is None else r1)], sides, color, at=(a[0], a[1], 0))


def tube(b, pts, radii, sides, color, cap=True, bones=None):
    """A loft through points, a ring of `sides` at each with its radius (parallel transport).
    `bones` tags each point's ring for the preview rigs."""
    sides = _more(sides)
    saved = b.cur
    pts = [Vector(p) for p in pts]
    n = len(pts)
    tans = []
    for i in range(n):
        t = pts[min(i + 1, n - 1)] - pts[max(i - 1, 0)]
        tans.append(t.normalized())
    ref = Vector((0, 0, 1)) if abs(tans[0].z) < 0.9 else Vector((1, 0, 0))
    nrm = (ref - tans[0] * ref.dot(tans[0])).normalized()
    rings = []
    for i in range(n):
        t = tans[i]
        nrm = (nrm - t * nrm.dot(t)).normalized()
        bi = t.cross(nrm)
        if bones:
            b.cur = bones[i]
        r = radii[i] if isinstance(radii, (list, tuple)) else radii
        if r < 1e-6:
            rings.append([b.v(pts[i])] * sides)
            continue
        rings.append([b.v(pts[i] + (nrm * math.cos(2 * math.pi * k / sides) + bi * math.sin(2 * math.pi * k / sides)) * r) for k in range(sides)])
    for k in range(n - 1):
        for i in range(sides):
            j = (i + 1) % sides
            vs = []
            for x in (rings[k][i], rings[k][j], rings[k + 1][j], rings[k + 1][i]):
                if x not in vs:
                    vs.append(x)
            if len(vs) >= 3:
                b.face(vs, color)
    if cap:
        if len(set(rings[0])) == sides:
            b.face(list(reversed(rings[0])), color)
        if len(set(rings[-1])) == sides:
            b.face(rings[-1], color)
    b.cur = saved


def box(b, c, size, color, rot=(0, 0, 0), taper=1.0):
    """A box centered on c; `taper` shrinks its top face."""
    b.push(c, rot)
    hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2
    lo = [b.v((x, y, -hz)) for x, y in ((-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy))]
    hi = [b.v((x * taper, y * taper, hz)) for x, y in ((-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy))]
    b.face(list(reversed(lo)), color)
    b.face(hi, color)
    for i in range(4):
        j = (i + 1) % 4
        b.face([lo[i], lo[j], hi[j], hi[i]], color)
    b.pop()


def quad_slab(b, corners, th, color):
    """A thin slab under four top corners (an awning stripe, a tarp, a banner)."""
    top = [b.v(c) for c in corners]
    bot = [b.v((c[0], c[1], c[2] - th)) for c in corners]
    b.face(top, color)
    b.face(list(reversed(bot)), color)
    for i in range(4):
        j = (i + 1) % 4
        b.face([bot[i], bot[j], top[j], top[i]], color)


# --- Shared pieces -------------------------------------------------------------------------

def void_face(b, c, r, eyes, eye_r=0.02, gap=0.042, left_eye_r=None):
    """The minions' face: a shadowed void and two eyes in the team's glow."""
    sphere(b, c, r, VOID, sides=12, rings=6, s=(1, 0.5, 1))
    x, y, z = c
    for sx in (-1, 1):
        if sx not in eyes:
            continue
        er = left_eye_r if (sx == 1 and left_eye_r) else eye_r
        sphere(b, (x + sx * gap, y - r * 0.45, z + r * 0.15), er, glow(), sides=8, rings=4)


def limb(b, pts, radii, color, hand=None, hand_r=0.05, bones=None):
    """An arm through shoulder, elbow and hand; `bones` = (upper, lower) for the preview rigs."""
    saved = b.cur
    tube(b, pts, radii, 8, color, bones=[bones[0], {bones[0]: 0.5, bones[1]: 0.5}, bones[1]] if bones else None)
    if hand:
        if bones:
            b.cur = bones[1].replace("fore_", "hand_")
        sphere(b, pts[-1], hand_r, hand, sides=8, rings=5)
    b.cur = saved


def plinth(b, r=1.5, h=0.18):
    """The stall's plinth: the base's paving stone, a worn step and a darkened foot (15 §2)."""
    lathe(b, [(0.0, 0.0), (0.0, r + 0.22), (0.07, r + 0.22), (0.07, r + 0.02), (0.0, 0.0)], 10,
          lambda a, z: STONE_D, phase=math.pi / 10)
    stones = [STONE, (0.13, 0.14, 0.16), (0.17, 0.18, 0.19), (0.12, 0.13, 0.15)]
    lathe(b, [(0.0, 0.0), (0.0, r), (h - 0.03, r), (h, r - 0.05), (h, 0.0)], 10,
          lambda a, z: MOSS if (int(a / (2 * math.pi) * 10) == 3 and z < h - 0.02) else stones[int(a / (2 * math.pi) * 10) % 4],
          phase=math.pi / 10)
    return h


def awning(b, x0, x1, yb, yf, zb, zf, stripes, ca, cb, valance=True):
    w = (x1 - x0) / stripes
    for i in range(stripes):
        xa, xb = x0 + i * w, x0 + (i + 1) * w
        quad_slab(b, [(xa, yf, zf), (xb, yf, zf), (xb, yb, zb), (xa, yb, zb)], 0.03, ca if i % 2 == 0 else cb)
        if valance:
            # A scalloped flap hanging off the front edge.
            box(b, ((xa + xb) / 2, yf - 0.005, zf - 0.11), (w * 0.96, 0.025, 0.16), ca if i % 2 == 0 else cb, taper=1.0)
            sphere(b, ((xa + xb) / 2, yf - 0.005, zf - 0.19), w * 0.48, ca if i % 2 == 0 else cb, sides=8, rings=4, s=(1, 0.06, 0.5))


def coin_stack(b, c, h, r=0.04):
    lathe(b, [(0.0, r), (h, r)], 10, lambda a, z: GOLD if int(z / 0.014) % 2 == 0 else GOLD_D, at=c)


def sack(b, c, s=1.0, mark=True, open_=False):
    prof = [(0.0, 0.0), (0.0, 0.17), (0.12, 0.22), (0.26, 0.19), (0.33, 0.10), (0.36, 0.06), (0.40, 0.09), (0.42, 0.0)]
    if open_:
        prof = [(0.0, 0.0), (0.0, 0.21), (0.10, 0.25), (0.20, 0.22), (0.25, 0.16), (0.27, 0.17), (0.27, 0.0)]
    prof = [(z * s, r * s) for z, r in prof]
    lathe(b, prof, 9, lambda a, z: BURLAP if z < prof[-3][0] else (0.22, 0.15, 0.07), at=c,
          rmod=lambda a, z: 1 + 0.06 * math.sin(3 * a + z * 9))
    if mark:
        disc(b, (c[0], c[1] - 0.2 * s, c[2] + 0.13 * s), 0.06 * s, 0.02, GOLD, rot=(90, 0, 0))
    if open_:
        disc(b, (c[0], c[1], c[2] + 0.27 * s), 0.17 * s, 0.03, GOLD)


def lantern(b, c, color, s=1.0):
    box(b, (c[0], c[1], c[2] + 0.09 * s), (0.12 * s, 0.12 * s, 0.03 * s), IRON)
    box(b, c, (0.09 * s, 0.09 * s, 0.13 * s), color)
    box(b, (c[0], c[1], c[2] - 0.08 * s), (0.11 * s, 0.11 * s, 0.03 * s), IRON)


# --- Goldwick: the rich merchant ------------------------------------------------------------

NAVY = (0.025, 0.03, 0.10)
NAVY_L = (0.04, 0.05, 0.15)
VEST = (0.62, 0.40, 0.07)
GLOVE_W = (0.62, 0.60, 0.55)
FUR = (0.30, 0.22, 0.15)
HAT = (0.018, 0.018, 0.022)
STACHE = (0.10, 0.05, 0.02)


def goldwick(b):
    b.part("root")
    for s in (-1, 1):
        b.part("foot_L" if s > 0 else "foot_R")
        box(b, (s * 0.12, -0.05, 0.035), (0.12, 0.22, 0.07), HAT)
        b.part("leg_L" if s > 0 else "leg_R")
        tube(b, [(s * 0.12, 0, 0.06), (s * 0.12, 0, 0.27)], 0.065, 8, (0.035, 0.03, 0.04))
    cz, rx, rz = 0.56, 0.38, 0.33

    def body_r(z):
        t = max(-1.0, min(1.0, (z - cz) / rz))
        return rx * math.sqrt(max(0.0, 1 - t * t))

    def coat(a, z):
        d = abs((a - 1.5 * math.pi + math.pi) % (2 * math.pi) - math.pi)
        if d < 0.5 and 0.3 < z < 0.84:
            return VEST
        return NAVY if int(a / (2 * math.pi) * 12) % 2 else NAVY_L
    b.part("body")
    lathe(b, [(cz - rz * math.cos(math.pi * k / 10), rx * math.sin(math.pi * k / 10)) for k in range(11)], 16, coat)
    for z in (0.40, 0.50, 0.60, 0.70):
        sphere(b, (0, -body_r(z) - 0.004, z), 0.024, GOLD, sides=6, rings=4)
    chain = []
    for x, z in ((-0.17, 0.60), (-0.09, 0.52), (0.0, 0.50), (0.09, 0.53), (0.16, 0.60)):
        chain.append((x, -math.sqrt(max(body_r(z) ** 2 - x * x, 0)) - 0.012, z))
    tube(b, chain, 0.008, 5, GOLD, bones=["body"] * 4 + ["watch"])
    box(b, (0.21, -0.29, 0.75), (0.07, 0.03, 0.06), team(), rot=(0, 0, 25))       # pocket square
    donut(b, (0, 0, 0.86), 0.17, 0.065, FUR, sides=14, tsides=6, rmod=lambda a, z: 1 + 0.05 * math.sin(7 * a))
    # Head: a black hood, the void, a monocle and a waxed moustache.
    b.part("head")
    sphere(b, (0, 0, 0.98), 0.17, HAT, sides=12, rings=6)
    void_face(b, (0, -0.115, 0.975), 0.115, eyes=(-1, 1), left_eye_r=0.028)
    b.part("monocle")
    donut(b, (0.042, -0.178, 0.993), 0.036, 0.007, GOLD, sides=12, tsides=4, rot=(90, 0, 0))
    tube(b, [(0.075, -0.17, 0.985), (0.13, -0.16, 0.90), (0.20, -0.25, 0.76)], 0.004, 4, GOLD, bones=["monocle", "head", "body"])
    b.part("head")
    for s in (-1, 1):
        tube(b, [(0, -0.185, 0.945), (s * 0.06, -0.18, 0.935), (s * 0.10, -0.165, 0.955), (s * 0.11, -0.16, 0.985)],
             [0.018, 0.016, 0.01, 0.002], 6, STACHE)
    # A tall stovepipe hat, a little rakish.
    b.part("hat")
    b.push((0, 0, 1.06), (0, 7, 0))
    lathe(b, [(0.0, 0.0), (0.0, 0.27), (0.025, 0.27), (0.025, 0.0)], 16, HAT)
    lathe(b, [(0.02, 0.150), (0.08, 0.152), (0.08, 0.0)], 16, GOLD)
    lathe(b, [(0.02, 0.148), (0.40, 0.16), (0.40, 0.0)], 16, HAT)
    b.pop()
    # Arms: the right on the cane, the left thumb in his waistcoat; rings on white gloves.
    limb(b, [(-0.30, 0, 0.78), (-0.43, -0.04, 0.61), (-0.43, -0.12, 0.48)], [0.075, 0.07, 0.065], NAVY, GLOVE_W, 0.06, bones=("up_R", "fore_R"))
    limb(b, [(0.30, 0, 0.78), (0.37, -0.16, 0.66), (0.19, -0.32, 0.68)], [0.075, 0.07, 0.065], NAVY, GLOVE_W, 0.06, bones=("up_L", "fore_L"))
    for p, bone in (((-0.40, -0.17, 0.49), "hand_R"), ((0.21, -0.37, 0.69), "hand_L")):
        b.part(bone)
        donut(b, p, 0.03, 0.009, GOLD, sides=8, tsides=4)
    b.part("hand_R")
    tube(b, [(-0.45, -0.15, 0.0), (-0.44, -0.13, 0.62)], 0.018, 6, HAT)
    disc(b, (-0.44, -0.13, 0.69), 0.075, 0.025, GOLD, sides=14, rot=(90, 0, 0))


def goldwick_stall(b):
    h = plinth(b)
    # The counter behind him, a coin carved in its front: he holds court in front of it.
    b.push((0, 0.45, 0))
    box(b, (0, -0.45, h + 0.25), (1.5, 0.45, 0.5), OAK)
    box(b, (0, -0.45, h + 0.525), (1.62, 0.54, 0.05), OAK_D)
    disc(b, (0, -0.68, h + 0.25), 0.15, 0.02, GOLD_D, sides=14, rot=(90, 0, 0))
    top = h + 0.55
    # Scales weighing a coin against a feather.
    tube(b, [(0.52, -0.45, top), (0.52, -0.45, top + 0.36)], 0.012, 6, BRASS)
    tube(b, [(0.34, -0.45, top + 0.36), (0.70, -0.45, top + 0.34)], 0.01, 5, BRASS)
    for x, z in ((0.35, top + 0.18), (0.69, top + 0.15)):
        tube(b, [(x, -0.45, z), (x, -0.45, z + 0.18)], 0.004, 4, BRASS)
        lathe(b, [(0.0, 0.0), (0.0, 0.03), (0.03, 0.085), (0.035, 0.0)], 10, BRASS, at=(x, -0.45, z - 0.03))
    disc(b, (0.35, -0.45, top + 0.17), 0.035, 0.012, GOLD)
    for x, y, hh in ((-0.58, -0.52, 0.12), (-0.48, -0.38, 0.22), (-0.66, -0.36, 0.08), (-0.38, -0.55, 0.06), (0.20, -0.36, 0.16)):
        coin_stack(b, (x, y, top), hh)
    # The ledger, open, with a quill.
    box(b, (-0.13, -0.50, top + 0.012), (0.38, 0.28, 0.025), (0.20, 0.04, 0.04), rot=(0, 0, 8))
    box(b, (-0.21, -0.50, top + 0.03), (0.17, 0.25, 0.02), CREAM, rot=(0, -4, 8))
    box(b, (-0.04, -0.49, top + 0.03), (0.17, 0.25, 0.02), CREAM, rot=(0, 4, 8))
    tube(b, [(0.0, -0.55, top + 0.04), (0.08, -0.40, top + 0.20)], [0.004, 0.02], 4, (0.6, 0.6, 0.58))
    b.pop()
    # Sacks of gold, one slumped open with coins spilled on the plinth.
    sack(b, (-1.0, -0.15, h), 1.1)
    sack(b, (-0.85, 0.35, h), 0.9)
    sack(b, (-0.98, -0.65, h), 0.8, mark=False, open_=True)
    for i, (x, y) in enumerate(((-0.70, -0.85), (-0.62, -0.75), (-0.78, -0.95), (-0.55, -0.92), (-0.66, -1.05))):
        disc(b, (x, y, h + 0.008), 0.035, 0.012, GOLD, sides=8, rot=(0, 10 * i, 0))
    # The strongbox, its lid ajar, glowing with gold.
    box(b, (1.0, -0.15, h + 0.15), (0.48, 0.34, 0.30), OAK)
    for x in (0.86, 1.14):
        box(b, (x, -0.15, h + 0.15), (0.04, 0.36, 0.32), IRON)
    box(b, (1.0, -0.15, h + 0.29), (0.42, 0.28, 0.03), emi((1.0, 0.75, 0.25)))
    box(b, (1.0, 0.03, h + 0.43), (0.50, 0.04, 0.32), OAK, rot=(-25, 0, 0))
    # A cabinet of gold behind him.
    box(b, (0, 0.62, h + 0.55), (1.5, 0.34, 1.1), OAK_D)
    for i, x in enumerate((-0.55, -0.2, 0.15, 0.5)):
        lathe(b, [(0.0, 0.05), (0.02, 0.03), (0.10, 0.025), (0.16, 0.06), (0.17, 0.06), (0.17, 0.0)], 8, GOLD, at=(x, 0.55, h + 1.10))
    # Posts and a striped awning in the team's color, a lantern on the corner.
    for x in (-0.95, 0.95):
        tube(b, [(x, 0.62, h), (x, 0.62, h + 2.05)], 0.045, 6, OAK_D)
        tube(b, [(x, -0.15, h), (x, -0.15, h + 1.80)], 0.04, 6, OAK_D)
    awning(b, -1.1, 1.1, 0.80, -0.25, h + 2.12, h + 1.84, 7, team(), CREAM)
    lantern(b, (0.95, -0.22, h + 1.50), glow())


# --- Madame Rosette: the perfumer ---------------------------------------------------------

ROSE = (0.60, 0.09, 0.26)
ROSE_D = (0.36, 0.035, 0.14)
ROSE_L = (0.85, 0.25, 0.48)
BOW = (0.90, 0.20, 0.50)
LACE = (0.80, 0.74, 0.66)
LIPS = (0.70, 0.015, 0.12)
PEARL = met((0.85, 0.83, 0.78))


def _gown_r(z, prof):
    for (z0, r0), (z1, r1) in zip(prof, prof[1:]):
        if z0 <= z <= z1 and z1 > z0:
            return r0 + (r1 - r0) * (z - z0) / (z1 - z0)
    return prof[-1][1]


def rosette(b):
    gown = [(0.0, 0.50), (0.10, 0.49), (0.25, 0.44), (0.45, 0.33), (0.62, 0.21), (0.72, 0.15)]
    skirt = lambda a, z: "skirt" if z < 0.38 else ({"skirt": 0.5, "body": 0.5} if z < 0.55 else "body")
    b.part("body")
    lathe(b, [(0.0, 0.0)] + gown, 16, lambda a, z: CREAM if z < 0.05 else (ROSE if int(a / (2 * math.pi) * 16) % 2 else ROSE_D), bone=skirt)
    for zc in (0.09, 0.29, 0.49):
        r = _gown_r(zc, gown)
        lathe(b, [(zc - 0.035, r - 0.01), (zc, r + 0.045), (zc + 0.03, r - 0.005)], 24, LACE, cap=False,
              rmod=lambda a, z: 1 + 0.035 * abs(math.sin(8 * a)), bone=skirt)
    lathe(b, [(0.70, 0.15), (0.80, 0.165), (0.88, 0.18), (0.95, 0.13), (0.98, 0.0)], 14, ROSE)
    donut(b, (0, 0, 0.965), 0.12, 0.035, LACE, sides=16, tsides=5, rmod=lambda a, z: 1 + 0.12 * abs(math.sin(6 * a)))
    for k in range(11):
        a = math.pi * 1.05 + k * math.pi * 0.9 / 10
        sphere(b, (math.cos(a) * 0.115, math.sin(a) * 0.115 - 0.01, 0.92 + 0.03 * math.sin(math.pi * k / 10)), 0.016, PEARL, sides=6, rings=3)
    # Hood, the void, lashes, painted lips and a beauty mark.
    b.part("head")
    sphere(b, (0, 0, 1.10), 0.16, ROSE, sides=12, rings=6, s=(1, 1, 1.05))
    void_face(b, (0, -0.105, 1.09), 0.11, eyes=(-1, 1))
    for s in (-1, 1):
        for k in range(3):
            x = s * (0.03 + 0.012 * k)
            tube(b, [(x, -0.157, 1.13), (x + s * 0.012 * k, -0.165, 1.165)], [0.004, 0.001], 4, VOID)
    sphere(b, (0, -0.158, 1.062), 0.034, LIPS, sides=10, rings=4, s=(1, 0.35, 0.40))
    sphere(b, (0, -0.156, 1.042), 0.03, LIPS, sides=10, rings=4, s=(1, 0.35, 0.45))
    sphere(b, (0.07, -0.145, 1.06), 0.007, VOID, sides=5, rings=3)
    # The huge bow.
    b.part("bow")
    sphere(b, (0, 0.03, 1.28), 0.048, BOW, sides=8, rings=5)
    for s in (-1, 1):
        sphere(b, (s * 0.14, 0.03, 1.32), 0.14, BOW, sides=10, rings=6, s=(1, 0.32, 0.6), rot=(0, -s * 22, 0))
        tube(b, [(s * 0.03, 0.08, 1.25), (s * 0.08, 0.14, 1.12), (s * 0.10, 0.16, 1.00)], [0.03, 0.028, 0.02], 5, BOW)
    # Puffed sleeves and long cream gloves: the right holds the parasol, the left is on her hip.
    for s in (-1, 1):
        b.part("up_L" if s > 0 else "up_R")
        sphere(b, (s * 0.20, 0, 0.90), 0.085, ROSE_L, sides=10, rings=5)
    limb(b, [(-0.22, 0, 0.86), (-0.28, -0.08, 0.74), (-0.17, -0.17, 0.80)], [0.035, 0.032, 0.03], LACE, LACE, 0.035, bones=("up_R", "fore_R"))
    limb(b, [(0.22, 0, 0.86), (0.31, 0.02, 0.74), (0.17, -0.06, 0.72)], [0.035, 0.032, 0.03], LACE, LACE, 0.035, bones=("up_L", "fore_L"))
    # The parasol over her shoulder.
    b.part("parasol")
    top = Vector((-0.36, 0.24, 1.60))
    hand = Vector((-0.17, -0.18, 0.78))
    tube(b, [hand - (top - hand) * 0.12, top + (top - hand).normalized() * 0.06], 0.012, 6, (0.75, 0.60, 0.30))
    tube(b, [hand + (top - hand) * 0.05, hand + (top - hand) * 0.12], 0.02, 6, team())
    d = (top - hand).normalized()
    rot = d.to_track_quat("Z", "Y").to_euler()
    b.push(tuple(top), tuple(math.degrees(x) for x in rot))
    lathe(b, [(0.06, 0.0), (0.02, 0.17), (-0.05, 0.31), (-0.13, 0.42), (-0.16, 0.43)], 16,
          lambda a, z: LACE if int(a / (2 * math.pi) * 8) % 2 else (0.82, 0.38, 0.55), cap=False,
          rmod=lambda a, z: 1 + (0.06 * abs(math.sin(4 * a)) if z < -0.1 else 0))
    b.pop()
    # A heart mirror on a ribbon at her waist.
    b.part("mirror")
    donut(b, (0.20, -0.22, 0.60), 0.045, 0.01, GOLD, sides=12, tsides=4, rot=(80, 0, 20))
    disc(b, (0.20, -0.22, 0.60), 0.042, 0.008, GLASS, sides=12, rot=(80, 0, 20))


def bottle(b, c, kind, color):
    x, y, z = c
    if kind == 0:
        sphere(b, (x, y, z + 0.06), 0.06, color, sides=8, rings=5)
        cyl(b, (x, y), z + 0.10, z + 0.16, 0.018, color, sides=6)
        cyl(b, (x, y), z + 0.16, z + 0.18, 0.024, GOLD, sides=6)
    elif kind == 1:
        lathe(b, [(0.0, 0.035), (0.13, 0.035), (0.15, 0.015), (0.18, 0.015)], 8, color, at=(x, y, z))
        sphere(b, (x + 0.03, y, z + 0.17), 0.022, (0.80, 0.20, 0.50), sides=6, rings=4)
    else:
        for s in (-1, 1):
            sphere(b, (x + s * 0.03, y, z + 0.09), 0.04, color, sides=8, rings=5, s=(1, 0.7, 1))
        lathe(b, [(0.0, 0.0), (0.08, 0.06)], 8, color, at=(x, y, z), cap=False, sy=0.7)
        cyl(b, (x, y), z + 0.12, z + 0.16, 0.012, GOLD, sides=6)


POTIONS = [emi(c) for c in ((0.95, 0.25, 0.55), (0.55, 0.25, 0.95), (0.80, 0.03, 0.05), (0.95, 0.55, 0.15), (0.98, 0.42, 0.30), (0.70, 0.15, 0.80))]


def rosette_stall(b):
    h = plinth(b)
    # Three tiers of white shelves crowded with bottles.
    for k, (y, z) in enumerate(((0.45, 0.55), (0.62, 0.92), (0.79, 1.29))):
        box(b, (0, y, h + z - 0.02), (1.6, 0.18, 0.04), (0.62, 0.55, 0.50))
        for i in range(9):
            x = -0.68 + i * 0.17 + (0.04 if k % 2 else 0)
            bottle(b, (x, y, h + z), (i + k) % 3, POTIONS[(i * 2 + k) % len(POTIONS)])
    for x in (-0.82, 0.82):
        box(b, (x, 0.62, h + 0.70), (0.05, 0.55, 1.40), (0.55, 0.48, 0.44))
    box(b, (0, 0.92, h + 0.70), (1.68, 0.05, 1.40), (0.40, 0.20, 0.28))
    # The heart mirror on a gold stand.
    tube(b, [(0.95, -0.35, h), (0.95, -0.35, h + 1.05)], 0.02, 6, GOLD)
    lathe(b, [(0.0, 0.14), (0.04, 0.10), (0.04, 0.0)], 8, GOLD, at=(0.95, -0.35, h))
    b.push((0.95, -0.35, h + 1.25), (0, 0, -20))
    for s in (-1, 1):
        sphere(b, (s * 0.075, 0, 0.04), 0.10, GOLD, sides=10, rings=5, s=(1, 0.25, 1))
        sphere(b, (s * 0.07, -0.01, 0.04), 0.08, GLASS, sides=10, rings=5, s=(1, 0.25, 1))
    lathe(b, [(-0.20, 0.0), (0.02, 0.15)], 4, GOLD, rot=(0, 0, 0), sy=0.25, phase=math.pi / 4, cap=False)
    b.pop()
    # The velvet fainting couch and a hat box.
    box(b, (-0.95, -0.15, h + 0.24), (0.75, 0.40, 0.18), ROSE_D)
    box(b, (-1.28, -0.15, h + 0.42), (0.14, 0.40, 0.30), ROSE_D, rot=(0, -12, 0))
    box(b, (-1.0, 0.03, h + 0.42), (0.60, 0.06, 0.24), ROSE_D, rot=(-12, 0, 0))
    for x in (-1.25, -0.65):
        for y in (-0.30, 0.0):
            cyl(b, (x, y), h, h + 0.15, 0.02, GOLD, sides=6)
    lathe(b, [(0.0, 0.17), (0.18, 0.17), (0.18, 0.0)], 12, lambda a, z: CREAM if int(a / (2 * math.pi) * 12) % 2 else ROSE_L, at=(-0.95, -0.10, h + 0.33))
    lathe(b, [(0.0, 0.18), (0.05, 0.18), (0.05, 0.0)], 12, ROSE_L, at=(-0.95, -0.10, h + 0.51))
    # A puff of perfume.
    for i, p in enumerate(((0.45, -0.45, h + 1.30), (0.52, -0.52, h + 1.38), (0.40, -0.55, h + 1.42))):
        sphere(b, p, 0.05 - 0.01 * i, (0.95, 0.60, 0.75), sides=8, rings=4)
    # The scalloped canopy in the team's color, a hanging lamp; summer's rose trellis.
    for x in (-0.95, 0.95):
        tube(b, [(x, 0.92, h), (x, 0.92, h + 2.0)], 0.035, 6, (0.62, 0.55, 0.50))
    awning(b, -1.08, 1.08, 1.0, -0.25, h + 2.05, h + 1.78, 8, team(), LACE)
    lantern(b, (-0.78, -0.30, h + 1.45), glow())


# --- Slink: the black marketeer --------------------------------------------------------------

COAT = (0.022, 0.022, 0.026)
COAT_L = (0.035, 0.034, 0.04)
LINING = (0.30, 0.48, 0.02)
BRIM = (0.05, 0.035, 0.025)
SCARF = (0.13, 0.07, 0.035)
WRAPS = (0.20, 0.15, 0.09)
VIAL = emi((0.50, 1.0, 0.05))


def _stoop(z):
    return (0.0, -0.32 * max(0.0, (z - 0.7) / 0.62) ** 2)


def _coat_bone(a, z):
    """Slink's coat: the hem swings; the front on his left opens like a door (fidget 1)."""
    base = {"hem": 1.0} if z < 0.3 else ({"hem": 0.5, "body": 0.5} if z < 0.5 else {"body": 1.0})
    da = a - 1.5 * math.pi
    wa = 1.0 if 0.05 < da < 0.9 else (max(0.0, 1 - (da - 0.9) / 0.5) if da >= 0.9 else 0.0)
    wz = 0.0 if z < 0.5 else (1.0 if z < 1.1 else (0.3 if z < 1.3 else 0.0))
    w = wa * wz
    if w <= 0:
        return base
    out = {k: v * (1 - w) for k, v in base.items()}
    out["flap"] = w
    return out


def slink(b):
    b.part("root")
    for s in (-1, 1):
        b.part("foot_L" if s > 0 else "foot_R")
        box(b, (s * 0.11, -0.07, 0.04), (0.10, 0.26, 0.08), (0.04, 0.03, 0.025))
        b.part("leg_L" if s > 0 else "leg_R")
        tube(b, [(s * 0.11, 0, 0.07), (s * 0.11, 0, 0.55)], [0.05, 0.045], 6, WRAPS)
    coat = [(0.12, 0.30), (0.40, 0.24), (0.75, 0.19), (1.05, 0.21), (1.25, 0.22), (1.33, 0.10)]

    def coat_col(a, z):
        d = abs((a - 1.5 * math.pi + math.pi) % (2 * math.pi) - math.pi)
        if d < 0.28 and z < 1.15:
            return LINING
        if z < 0.18:
            return (0.012, 0.012, 0.014)
        return COAT if int(a / (2 * math.pi) * 14) % 2 else COAT_L
    lathe(b, coat, 14, coat_col, sy=0.85, shift=_stoop,
          rmod=lambda a, z: (0.86 if (abs(a - math.pi / 2) < 0.15 and z < 0.5) else 1.0) * (1 + 0.12 * abs(math.sin(5 * a)) if z < 0.2 else 1.0),
          bone=_coat_bone)
    b.part("body")
    # A popped collar around his neck, and a lumpy sack slung on his back.
    lathe(b, [(0.0, 0.16), (0.16, 0.20), (0.22, 0.22)], 12, COAT_L, at=(0, -0.32, 1.22), rot=(10, 0, 0), cap=False,
          rmod=lambda a, z: 1.0 if abs(a - 1.5 * math.pi) > 0.7 else 0.8)
    b.part("sack")
    sphere(b, (0.20, 0.16, 1.02), 0.21, BURLAP, sides=10, rings=6, s=(1, 0.85, 1.15))
    sphere(b, (0.14, 0.10, 1.25), 0.06, (0.22, 0.15, 0.07), sides=8, rings=4)
    tube(b, [(0.14, 0.10, 1.25), (-0.12, -0.20, 1.26), (-0.22, -0.30, 1.05), (-0.16, -0.25, 0.80)], 0.02, 4, (0.22, 0.15, 0.07), bones=["sack", "body", "body", "body"])
    b.part("flap")
    for z in (0.58, 0.72, 0.86):
        dx, dy = _stoop(z)
        r = 0.19 + (0.21 - 0.19) * max(0, (z - 0.75) / 0.3)
        cyl(b, (0.05, -r * 0.85 + dy - 0.01), z, z + 0.09, 0.02, VIAL, sides=6)
    # The head under the brim: one glowing eye, a patch, a scarf over the lower void.
    b.part("head")
    sphere(b, (0, -0.40, 1.38), 0.14, BRIM, sides=12, rings=6)
    void_face(b, (0, -0.495, 1.37), 0.10, eyes=(-1,))
    box(b, (0.042, -0.547, 1.39), (0.055, 0.015, 0.045), (0.01, 0.01, 0.01))
    tube(b, [(0.065, -0.545, 1.41), (0.13, -0.46, 1.43), (0.12, -0.33, 1.44)], 0.006, 4, (0.01, 0.01, 0.01))
    donut(b, (0, -0.38, 1.31), 0.12, 0.055, SCARF, sides=12, tsides=5)
    sphere(b, (0, -0.50, 1.33), 0.085, SCARF, sides=10, rings=5, s=(1, 0.6, 0.55))
    tube(b, [(0.09, -0.46, 1.30), (0.15, -0.47, 1.12), (0.15, -0.44, 0.96)], [0.045, 0.04, 0.035], 5, SCARF, bones=["head", "scarf", "scarf"])
    b.part("brim")
    b.push((0, -0.40, 1.50), (-12, -8, 0))
    lathe(b, [(0.0, 0.0), (0.0, 0.27), (0.025, 0.27), (0.025, 0.0)], 16, BRIM, rmod=lambda a, z: 1 + 0.05 * math.sin(3 * a))
    lathe(b, [(0.02, 0.155), (0.05, 0.157), (0.05, 0.0)], 12, (0.02, 0.015, 0.01))
    lathe(b, [(0.04, 0.152), (0.16, 0.13), (0.19, 0.0)], 12, BRIM)
    b.pop()
    # Long arms: the right offers a vial, the left holds his coat open. Long fingers.
    limb(b, [(-0.22, -0.28, 1.24), (-0.36, -0.33, 0.98), (-0.20, -0.52, 0.93)], [0.075, 0.068, 0.075], COAT, (0.03, 0.025, 0.02), 0.04, bones=("up_R", "fore_R"))
    limb(b, [(0.22, -0.28, 1.24), (0.42, -0.36, 1.08), (0.33, -0.60, 1.26)], [0.075, 0.068, 0.075], COAT, (0.03, 0.025, 0.02), 0.04, bones=("up_L", "fore_L"))
    b.part("fingers_L")
    for k in range(3):
        tube(b, [(0.33 + 0.02 * (k - 1), -0.62, 1.28), (0.33 + 0.025 * (k - 1), -0.66, 1.36), (0.33 + 0.03 * (k - 1), -0.62, 1.42)], [0.013, 0.011, 0.004], 4, (0.03, 0.025, 0.02))
    b.part("hand_R")
    for k in range(3):
        tube(b, [(-0.20 + 0.02 * (k - 1), -0.55, 0.93), (-0.19 + 0.025 * (k - 1), -0.64, 0.95), (-0.18 + 0.03 * (k - 1), -0.67, 1.0)], [0.013, 0.011, 0.005], 4, (0.03, 0.025, 0.02))
    cyl(b, (-0.18, -0.65), 0.98, 1.10, 0.025, VIAL, sides=6)
    # Rolled maps on his back.
    b.part("body")
    for y, z in ((0.10, 1.12), (0.13, 1.00)):
        b.push((0, y, z), (0, 90, 0))
        lathe(b, [(-0.22, 0.055), (0.22, 0.055)], 8, PARCH)
        b.pop()


def slink_stall(b):
    h = plinth(b)
    # The handcart behind him, handles to the right, tipped on its prop leg.
    b.push((0.15, 0.55, h), (0, 0, 0))
    for y in (-0.44, 0.44):
        b.push((0.05, y, 0.40), (90, 0, 0))
        donut(b, (0, 0, 0), 0.38, 0.035, IRON, sides=14, tsides=4)
        disc(b, (0, 0, 0), 0.06, 0.12, OAK_D, sides=8)
        for k in range(6):
            a = k * math.pi / 3
            tube(b, [(0, 0, 0), (math.cos(a) * 0.36, math.sin(a) * 0.36, 0)], 0.018, 4, OAK)
        b.pop()
    box(b, (0, 0, 0.62), (1.35, 0.80, 0.08), OAK, rot=(0, 4, 0))
    for y in (-0.40, 0.40):
        box(b, (0, y, 0.74), (1.35, 0.04, 0.18), OAK_D, rot=(0, 4, 0))
    for y in (-0.30, 0.30):
        tube(b, [(0.65, y, 0.60), (1.25, y, 0.50), (1.45, y, 0.40)], 0.03, 6, OAK)
    tube(b, [(-0.6, 0.0, 0.60), (-0.62, 0.0, 0.0)], 0.03, 6, OAK)
    # Goods: crates, a rack of vials, daggers, a caged something under a cloth, the tarp.
    for (x, y, z, sx, sy, sz, r) in ((-0.40, -0.15, 0.80, 0.36, 0.32, 0.30, 6), (-0.38, 0.22, 0.80, 0.34, 0.32, 0.28, -4), (-0.40, 0.03, 1.08, 0.30, 0.28, 0.24, 12)):
        box(b, (x, y, z), (sx, sy, sz), (0.24, 0.15, 0.07), rot=(0, 0, r))
        box(b, (x, y - sy / 2 - 0.006, z), (sx * 0.95, 0.012, 0.035), OAK_D, rot=(0, 30, r))
        box(b, (x, y - sy / 2 - 0.006, z), (sx * 0.95, 0.012, 0.035), OAK_D, rot=(0, -30, r))
    box(b, (0.15, -0.25, 0.72), (0.40, 0.16, 0.06), OAK_D)
    for k in range(6):
        cyl(b, (0.0 + k * 0.06, -0.25), 0.74, 0.86, 0.022, VIAL, sides=6)
    for k in range(4):
        box(b, (0.45 + k * 0.04, -0.20, 0.72), (0.02, 0.30, 0.012), (0.35, 0.36, 0.38), rot=(0, 0, 8 * k - 12))
    for x in (0.10, 0.40):
        for y in (0.05, 0.38):
            tube(b, [(x, y, 0.68), (x, y, 1.0)], 0.01, 4, IRON)
    box(b, (0.25, 0.215, 0.99), (0.38, 0.40, 0.03), IRON)
    quad_slab(b, [(0.06, 0.0, 1.06), (0.44, 0.0, 1.06), (0.44, 0.44, 1.04), (0.06, 0.44, 1.08)], 0.02, (0.20, 0.06, 0.04))
    box(b, (0.25, 0.0, 0.88), (0.30, 0.02, 0.30), (0.20, 0.06, 0.04))
    for x in (0.22, 0.29):
        sphere(b, (x, 0.0, 0.84), 0.016, emi((1.0, 0.85, 0.20)), sides=6, rings=3)
    quad_slab(b, [(-0.70, -0.42, 1.25), (0.0, -0.42, 1.0), (0.0, 0.42, 1.0), (-0.70, 0.42, 1.25)], 0.03, (0.07, 0.08, 0.04))
    quad_slab(b, [(0.0, -0.42, 1.0), (0.08, -0.42, 0.85), (0.08, 0.42, 0.85), (0.0, 0.42, 1.0)], 0.03, (0.07, 0.08, 0.04))
    # The pole with a rag in the team's color and a hooded lantern turned low.
    tube(b, [(-0.62, 0.40, 0.60), (-0.66, 0.42, 2.05)], 0.025, 6, OAK)
    quad_slab(b, [(-0.65, 0.42, 2.0), (-0.20, 0.36, 1.92), (-0.22, 0.36, 1.68), (-0.65, 0.42, 1.74)], 0.012, team())
    tube(b, [(-0.64, 0.41, 1.55), (-0.45, 0.30, 1.50)], 0.01, 4, IRON)
    lantern(b, (-0.45, 0.30, 1.38), team(0.8), 0.9)
    b.pop()


# --- Old Folio: the archivist -----------------------------------------------------------------

ROBE = (0.075, 0.03, 0.20)
ROBE_D = (0.035, 0.012, 0.09)
BEARD = (0.72, 0.72, 0.70)
LEATHER = (0.12, 0.045, 0.02)
RUNE = emi((0.55, 0.20, 1.0))


def folio(b):
    b.part("root")
    for s in (-1, 1):
        b.part("foot_L" if s > 0 else "foot_R")
        box(b, (s * 0.07, -0.06, 0.03), (0.09, 0.17, 0.06), (0.16, 0.05, 0.08))
    stoop = lambda z: (0.0, -0.06 * (z / 0.56) ** 2)
    lathe(b, [(0.0, 0.0), (0.0, 0.24), (0.06, 0.24), (0.25, 0.21), (0.42, 0.17), (0.52, 0.13), (0.56, 0.0)], 12,
          lambda a, z: ROBE if int(a / (2 * math.pi) * 12) % 2 else ROBE_D, shift=stoop,
          bone=lambda a, z: "root" if z < 0.1 else ({"root": 0.3, "body": 0.7} if z < 0.3 else "body"))
    b.part("body")
    box(b, (0.20, -0.02, 0.22), (0.02, 0.09, 0.08), (0.20, 0.12, 0.30), rot=(0, 0, 12))
    b.part("head")
    sphere(b, (0, -0.05, 0.62), 0.13, ROBE, sides=12, rings=6)
    tube(b, [(0, 0.0, 0.70), (0, 0.03, 0.84), (0, -0.04, 0.96), (0, -0.13, 0.95)], [0.11, 0.075, 0.04, 0.008], 8, ROBE, bones=["head", "head", "hoodtip", "hoodtip"])
    tube(b, [(0.09, 0.0, 0.68), (0.15, 0.03, 0.86)], [0.012, 0.004], 4, (0.70, 0.70, 0.68))
    void_face(b, (0, -0.135, 0.61), 0.095, eyes=(-1, 1), eye_r=0.017, gap=0.035)
    b.part("specs")
    for s in (-1, 1):
        donut(b, (s * 0.035, -0.185, 0.626), 0.027, 0.005, BRASS, sides=10, tsides=4, rot=(90, 0, 0))
    tube(b, [(-0.008, -0.186, 0.63), (0.008, -0.186, 0.63)], 0.004, 4, BRASS)
    b.part("head")
    tube(b, [(0, -0.16, 0.58), (0, -0.21, 0.42), (0, -0.20, 0.24), (0, -0.18, 0.12)], [0.075, 0.07, 0.045, 0.008], 8, BEARD, bones=["head", "beard1", "beard2", "beard2"])
    # Arms: the right on his staff, the left stroking his beard. Ink-stained hands.
    limb(b, [(-0.14, -0.03, 0.48), (-0.22, -0.08, 0.38), (-0.22, -0.16, 0.44)], [0.04, 0.045, 0.055], ROBE, (0.10, 0.09, 0.12), 0.03, bones=("up_R", "fore_R"))
    limb(b, [(0.14, -0.03, 0.48), (0.19, -0.11, 0.36), (0.06, -0.22, 0.38)], [0.04, 0.045, 0.055], ROBE, (0.10, 0.09, 0.12), 0.03, bones=("up_L", "fore_L"))
    b.part("staff")
    tube(b, [(-0.23, -0.17, 0.0), (-0.22, -0.16, 1.05), (-0.31, -0.20, 1.14), (-0.40, -0.22, 1.08)], 0.018, 6, OAK)
    b.part("lantern")
    tube(b, [(-0.40, -0.22, 1.08), (-0.40, -0.22, 1.02)], 0.004, 4, IRON)
    lantern(b, (-0.40, -0.22, 0.95), emi((1.0, 0.65, 0.20)), 0.8)
    # The pack tower: tomes, scroll cases capped in the team's color, maps, a rune scroll, a candle.
    lean = lambda z: 0.10 * max(0.0, (z - 0.3) / 1.5) ** 1.5
    b.part("pack1")
    box(b, (0, 0.18, 0.27), (0.36, 0.20, 0.04), OAK)
    for s in (-1, 1):
        tube(b, [(s * 0.15, 0.17, 0.25), (s * 0.15 + lean(1.5), 0.19, 1.50)], 0.015, 5, OAK, bones=["pack1", "pack3"])
    tube(b, [(-0.14, 0.12, 0.50), (0.0, -0.02, 0.52), (0.14, 0.12, 0.50)], 0.012, 4, LEATHER, bones=["pack1", "body", "pack1"])
    z = 0.28

    def at(dz):
        return (lean(z + dz), 0.20)
    items = [
        ("tome", 0.14, (0.42, 0.28), LEATHER),
        ("cases", 0.32, None, None),
        ("maps", 0.11, None, None),
        ("tome", 0.10, (0.36, 0.26), (0.05, 0.10, 0.04)),
        ("tome", 0.09, (0.32, 0.24), (0.20, 0.03, 0.03)),
        ("rune", 0.09, None, None),
        ("tome", 0.12, (0.30, 0.22), ROBE_D),
        ("chest", 0.16, None, None),
        ("tome", 0.08, (0.26, 0.20), LEATHER),
        ("candle", 0.14, None, None),
    ]
    for kind, hh, size, col in items:
        b.part("pack1" if z < 0.7 else ("pack2" if z < 1.15 else "pack3"))
        x, y = at(hh / 2)
        c = (x, y, z + hh / 2)
        if kind == "tome":
            box(b, c, (size[0], size[1], hh), col, rot=(0, 0, (z * 37) % 14 - 7))
            box(b, (x, y - 0.01, c[2]), (size[0] * 0.94, size[1] * 0.98, hh * 0.7), PARCH, rot=(0, 0, (z * 37) % 14 - 7))
        elif kind == "cases":
            for dx in (-0.11, 0.0, 0.11):
                cyl(b, (x + dx, y), z, z + hh, 0.05, LEATHER, sides=8)
                cyl(b, (x + dx, y), z + hh, z + hh + 0.03, 0.055, team(), sides=8)
        elif kind == "maps":
            for dy in (-0.05, 0.06):
                b.push((x, y + dy, z + 0.05), (0, 90, 0))
                lathe(b, [(-0.24, 0.05), (0.24, 0.05)], 8, PARCH)
                b.pop()
        elif kind == "rune":
            b.push((x, y - 0.03, z + 0.045), (0, 90, 8))
            lathe(b, [(-0.20, 0.045), (0.20, 0.045)], 8, RUNE)
            b.pop()
        elif kind == "chest":
            box(b, c, (0.30, 0.22, hh), LEATHER)
            box(b, (x, y, c[2]), (0.31, 0.23, 0.03), BRASS)
        elif kind == "candle":
            cyl(b, (x, y), z, z + 0.03, 0.06, BRASS, sides=8)
            cyl(b, (x, y), z + 0.03, z + 0.12, 0.03, CREAM, sides=8)
            sphere(b, (x, y, z + 0.15), 0.022, FLAME, sides=6, rings=4, s=(1, 1, 1.6))
        z += hh + 0.01


def folio_stall(b):
    h = plinth(b)
    # The writing desk at his left, heaped with scrolls, one unrolled to the plinth.
    box(b, (0.80, -0.20, h + 0.28), (0.85, 0.50, 0.56), OAK)
    quad_slab(b, [(0.36, -0.48, h + 0.66), (1.24, -0.48, h + 0.66), (1.24, 0.06, h + 0.80), (0.36, 0.06, h + 0.80)], 0.04, OAK_D)
    for k in range(6):
        x = 0.52 + (k * 0.29) % 0.62
        y = -0.33 + (k * 0.17) % 0.32
        zz = h + 0.75 + 0.07 * (k % 2) + (y + 0.35) * 0.25
        b.push((x, y, zz), (0, 90, (k * 47) % 180))
        lathe(b, [(-0.14, 0.05), (0.14, 0.05)], 8, PARCH if k % 3 else CREAM)
        disc(b, (0, 0, 0.145), 0.02, 0.01, (0.30, 0.04, 0.08) if k % 2 else ROBE, sides=6)
        b.pop()
    quad_slab(b, [(0.55, -0.50, h + 0.68), (0.80, -0.50, h + 0.68), (0.82, -0.62, h + 0.02), (0.57, -0.62, h + 0.02)], 0.01, CREAM)
    # An hourglass and a lamp on the desk.
    lathe(b, [(0.0, 0.06), (0.02, 0.06), (0.02, 0.045), (0.10, 0.0), (0.18, 0.045), (0.18, 0.06), (0.20, 0.06), (0.20, 0.0)], 8,
          lambda a, z: BRASS if z < 0.02 or z > 0.18 else (0.70, 0.50, 0.20), at=(1.12, -0.05, h + 0.80))
    tube(b, [(0.50, 0.02, h + 0.80), (0.50, 0.02, h + 1.15), (0.58, -0.05, h + 1.15)], 0.012, 4, BRASS)
    sphere(b, (0.58, -0.05, h + 1.10), 0.04, FLAME, sides=6, rings=4)
    # Stacks of tomes, the globe, the Astrolabe on its pedestal.
    for sx, sy, n in ((-0.75, -0.25, 5), (-1.05, 0.25, 4), (-0.55, 0.15, 3)):
        zz = h
        for k in range(n):
            hh = 0.09 + 0.03 * ((k + n) % 3)
            box(b, (sx, sy, zz + hh / 2), (0.34 - 0.02 * k, 0.26, hh), [LEATHER, (0.05, 0.10, 0.04), (0.20, 0.03, 0.03), ROBE_D][(k + n) % 4], rot=(0, 0, (k * 23) % 20 - 10))
            zz += hh
    tube(b, [(-0.70, 0.62, h), (-0.70, 0.62, h + 0.55)], 0.02, 6, BRASS)
    lathe(b, [(0.0, 0.16), (0.04, 0.12), (0.04, 0.0)], 8, OAK_D, at=(-0.70, 0.62, h))
    sphere(b, (-0.70, 0.62, h + 0.78), 0.22, lambda a, z: (0.10, 0.20, 0.30) if (int(a * 3) + int(z * 9)) % 3 else (0.45, 0.35, 0.15), sides=14, rings=8)
    donut(b, (-0.70, 0.62, h + 0.78), 0.25, 0.012, BRASS, sides=16, tsides=4, rot=(70, 0, 30))
    lathe(b, [(0.0, 0.20), (0.08, 0.14), (0.62, 0.11), (0.70, 0.17), (0.72, 0.0)], 8, STONE, at=(0.95, 0.50, h))
    for k, r in enumerate(((90, 0, 0), (90, 0, 60), (30, 0, 0))):
        donut(b, (0.95, 0.50, h + 0.98), 0.18 - 0.03 * k, 0.012, BRASS, sides=16, tsides=4, rot=r)
    sphere(b, (0.95, 0.50, h + 0.98), 0.045, glow(), sides=8, rings=4)
    # The banner in the team's color with a quill sigil.
    tube(b, [(-1.25, 0.65, h), (-1.25, 0.65, h + 2.25)], 0.03, 6, OAK_D)
    tube(b, [(-1.25, 0.65, h + 2.15), (-0.75, 0.65, h + 2.15)], 0.02, 5, OAK_D)
    quad_slab(b, [(-1.20, 0.62, h + 2.12), (-0.80, 0.62, h + 2.12), (-0.80, 0.62, h + 1.40), (-1.20, 0.62, h + 1.30)], 0.02, team())
    tube(b, [(-1.05, 0.595, h + 1.55), (-0.93, 0.595, h + 1.95)], [0.008, 0.03], 4, RUNE)


# --- Seasonal dressing (19 §5) -----------------------------------------------------------------
# What each stall wears on top of its core, one set per season (summer, autumn, winter). Snow on
# the upward facets in winter comes from the prop material; these are the things that change.

H = 0.18          # the plinth's top
LEAF = [(0.55, 0.16, 0.02), (0.62, 0.30, 0.03), (0.40, 0.08, 0.02), (0.55, 0.42, 0.06)]
COAL = emi((1.0, 0.38, 0.06))
SMOKE = (0.30, 0.30, 0.32)
STEAM = (0.70, 0.72, 0.74)
PINE = (0.04, 0.12, 0.06)


def _leaves(b, spots, seed=0):
    """Fallen leaves: small flat tilted diamonds."""
    for k, (x, y, z) in enumerate(spots):
        a = (k * 61 + seed * 17) % 180
        b.push((x, y, z + 0.004), (6 * ((k % 3) - 1), 4 * ((k % 2) * 2 - 1), a))
        quad_slab(b, [(-0.035, 0, 0), (0, -0.02, 0.003), (0.035, 0, 0), (0, 0.02, 0.003)], 0.004, LEAF[(k + seed) % len(LEAF)])
        b.pop()


def _brazier(b, x, y, smoke=True):
    """An iron bowl on three legs, glowing coals, a wisp of smoke."""
    for k in range(3):
        a = k * 2 * math.pi / 3
        tube(b, [(x + 0.16 * math.cos(a), y + 0.16 * math.sin(a), H), (x + 0.10 * math.cos(a), y + 0.10 * math.sin(a), H + 0.30)], 0.015, 4, IRON)
    lathe(b, [(0.0, 0.0), (0.0, 0.06), (0.10, 0.17), (0.12, 0.19)], 10, IRON, at=(x, y, H + 0.28), cap=False)
    lathe(b, [(0.0, 0.17), (0.02, 0.10), (0.03, 0.0)], 8, COAL, at=(x, y, H + 0.37))
    if smoke:
        for k, (dx, dz, r) in enumerate(((0.0, 0.18, 0.07), (0.04, 0.34, 0.09), (-0.02, 0.52, 0.11))):
            sphere(b, (x + dx, y + 0.01 * k, H + 0.40 + dz), r, SMOKE, sides=6, rings=4)


def _candle(b, x, y, z, h=0.12):
    cyl(b, (x, y), z, z + h, 0.022, CREAM, sides=6)
    sphere(b, (x, y, z + h + 0.03), 0.017, FLAME, sides=6, rings=4, s=(1, 1, 1.7))


def goldwick_summer(b):
    # Bunting between the front posts, and a potted lemon tree.
    pts = [(-0.95 + 1.9 * k / 8, -0.18, H + 1.70 - 0.12 * math.sin(math.pi * k / 8)) for k in range(9)]
    tube(b, pts, 0.006, 3, (0.30, 0.25, 0.20))
    for k in range(8):
        a, c = Vector(pts[k]), Vector(pts[k + 1])
        m = (a + c) / 2
        col = [team(), CREAM, GOLD][k % 3]
        b.face([b.v(tuple(a + (c - a) * 0.1)), b.v(tuple(m - Vector((0, 0, 0.17)))), b.v(tuple(c - (c - a) * 0.1))], col)
        b.face([b.v(tuple(c - (c - a) * 0.1)), b.v(tuple(m - Vector((0, 0, 0.17)))), b.v(tuple(a + (c - a) * 0.1))], col)
    x, y = 1.05, -0.80
    lathe(b, [(0.0, 0.12), (0.22, 0.16), (0.25, 0.17)], 10, (0.45, 0.20, 0.10), at=(x, y, H))
    tube(b, [(x, y, H + 0.22), (x + 0.02, y, H + 0.60)], [0.025, 0.018], 5, OAK)
    for k, (dx, dy, dz, r) in enumerate(((0, 0, 0.78, 0.20), (0.13, 0.05, 0.70, 0.14), (-0.12, -0.04, 0.72, 0.15), (0.0, -0.08, 0.92, 0.13))):
        sphere(b, (x + dx, y + dy, H + dz), r, (0.08, 0.22, 0.05), sides=8, rings=5)
    for dx, dy, dz in ((0.10, -0.14, 0.74), (-0.14, -0.12, 0.66), (0.05, -0.12, 0.92), (0.18, 0.02, 0.62)):
        sphere(b, (x + dx, y + dy, H + dz), 0.035, (0.95, 0.80, 0.10), sides=6, rings=4, s=(1, 1, 1.3))


def goldwick_autumn(b):
    # Gourds piled by the strongbox, a harvest wreath on the front post.
    for k, (dx, dy, r, col) in enumerate(((0.0, 0.0, 0.12, (0.80, 0.32, 0.03)), (0.18, -0.06, 0.09, (0.20, 0.32, 0.06)),
                                          (-0.12, -0.10, 0.08, (0.85, 0.55, 0.08)), (0.06, -0.14, 0.07, (0.70, 0.25, 0.03)))):
        x, y = 0.95 + dx, -0.62 + dy
        sphere(b, (x, y, H + r * 0.75), r, col, sides=10, rings=5, s=(1, 1, 0.75))
        tube(b, [(x, y, H + r * 1.45), (x + 0.02, y, H + r * 1.45 + 0.05)], 0.012, 4, (0.25, 0.18, 0.06))
    b.push((-0.95, -0.21, H + 1.35), (90, 0, 0))
    donut(b, (0, 0, 0), 0.16, 0.045, (0.35, 0.22, 0.06), sides=14, tsides=5)
    for k in range(10):
        a = k * 2 * math.pi / 10
        sphere(b, (0.16 * math.cos(a), 0.16 * math.sin(a), 0.04), 0.035, LEAF[k % len(LEAF)], sides=6, rings=3)
    b.pop()
    _leaves(b, [(0.6, -1.0, H), (-0.4, -1.2, H), (0.2, -1.3, H), (-1.1, -0.6, H), (1.2, -0.2, H), (-0.7, -1.05, H)])


def goldwick_winter(b):
    # A steaming mug on the counter and a brazier to warm his hands.
    x, y, z = 0.30, -0.05, H + 0.555
    lathe(b, [(0.0, 0.0), (0.0, 0.045), (0.10, 0.05), (0.10, 0.0)], 10, (0.55, 0.15, 0.10), at=(x, y, z))
    donut(b, (x + 0.06, y, z + 0.05), 0.03, 0.009, (0.55, 0.15, 0.10), sides=8, tsides=4, rot=(90, 0, 0))
    for k, (dz, r) in enumerate(((0.16, 0.035), (0.25, 0.045), (0.36, 0.05))):
        sphere(b, (x + 0.02 * k, y, z + dz), r, STEAM, sides=6, rings=4)
    _brazier(b, 1.05, -0.78)


def rosette_summer(b):
    # The trellis of climbing roses, and an open fan on the couch.
    x = 1.30
    for zz in range(6):
        box(b, (x, 0.30, H + 0.25 + zz * 0.3), (0.03, 0.9, 0.03), (0.40, 0.30, 0.20))
    for yy in range(4):
        box(b, (x, -0.10 + yy * 0.27, H + 0.90), (0.03, 0.03, 1.8), (0.40, 0.30, 0.20))
    for k in range(14):
        yy, zz = -0.1 + (k * 0.37) % 0.85, 0.3 + (k * 0.53) % 1.5
        sphere(b, (x - 0.03, yy, H + zz), 0.07, (0.06, 0.16, 0.03), sides=6, rings=3)
        if k % 2 == 0:
            sphere(b, (x - 0.07, yy + 0.04, H + zz + 0.04), 0.045, (0.85, 0.10, 0.30), sides=6, rings=4)
    b.push((-0.80, -0.28, H + 0.345), (0, 0, 20))
    for k in range(7):
        a0, a1 = math.pi * k / 7, math.pi * (k + 1) / 7
        col = LACE if k % 2 else (0.82, 0.38, 0.55)
        b.face([b.v((0, 0, 0)), b.v((0.14 * math.cos(a0), 0.14 * math.sin(a0), 0.004)), b.v((0.14 * math.cos(a1), 0.14 * math.sin(a1), 0.004))], col)
    b.pop()


def rosette_autumn(b):
    # Dried flowers in two vases, a knitted shawl over the couch.
    for x, y in ((-1.18, -0.70), (1.18, -0.72)):
        lathe(b, [(0.0, 0.07), (0.10, 0.10), (0.22, 0.05), (0.28, 0.07)], 10, (0.55, 0.42, 0.30), at=(x, y, H))
        for k in range(7):
            a = k * 2 * math.pi / 7
            tip = (x + 0.12 * math.cos(a), y + 0.12 * math.sin(a), H + 0.55 + 0.08 * (k % 3))
            tube(b, [(x, y, H + 0.26), tip], 0.006, 3, (0.30, 0.25, 0.12))
            sphere(b, tip, 0.04, [(0.60, 0.30, 0.15), (0.70, 0.55, 0.30), (0.45, 0.20, 0.25)][k % 3], sides=6, rings=4)
    quad_slab(b, [(-1.30, -0.02, H + 0.56), (-0.70, -0.02, H + 0.56), (-0.70, -0.20, H + 0.34), (-1.30, -0.20, H + 0.34)], 0.02, (0.75, 0.55, 0.12))
    quad_slab(b, [(-1.30, 0.06, H + 0.30), (-0.70, 0.06, H + 0.30), (-0.70, 0.04, H + 0.56), (-1.30, 0.04, H + 0.56)], 0.02, (0.68, 0.48, 0.10))
    _leaves(b, [(0.5, -1.1, H), (-0.3, -1.25, H), (0.9, -1.0, H), (-0.9, -0.9, H), (0.1, -1.35, H)], seed=1)


def rosette_winter(b):
    # A candelabra by the mirror and a pine garland with bows along the canopy.
    x, y = 1.20, -0.72
    tube(b, [(x, y, H), (x, y, H + 0.85)], 0.02, 6, GOLD)
    lathe(b, [(0.0, 0.14), (0.04, 0.09), (0.04, 0.0)], 8, GOLD, at=(x, y, H))
    tube(b, [(x - 0.16, y, H + 0.95), (x - 0.16, y, H + 0.85), (x + 0.16, y, H + 0.85), (x + 0.16, y, H + 0.95)], 0.012, 4, GOLD)
    for dx in (-0.16, 0.0, 0.16):
        _candle(b, x + dx, y, H + 0.95 if dx else H + 0.85, 0.14)
    for k in range(12):
        gx = -1.0 + 2.0 * k / 11
        gz = H + 1.62 - 0.08 * math.sin(math.pi * k / 11)
        sphere(b, (gx, -0.28, gz), 0.075, PINE, sides=6, rings=4)
        if k % 3 == 1:
            for s_ in (-1, 1):
                sphere(b, (gx + s_ * 0.04, -0.34, gz - 0.02), 0.035, (0.75, 0.04, 0.10), sides=6, rings=3, s=(1, 0.5, 0.7))


def slink_summer(b):
    # A crate of fruit stacked in front as cover.
    box(b, (0.75, -0.75, H + 0.13), (0.45, 0.30, 0.26), (0.24, 0.15, 0.07), rot=(0, 0, -15))
    for k in range(8):
        sphere(b, (0.60 + (k % 4) * 0.09, -0.80 + (k // 4) * 0.12, H + 0.29), 0.05, (0.85, 0.40, 0.03) if k % 3 else (0.25, 0.45, 0.04), sides=6, rings=4)


def slink_autumn(b):
    # Leaves fallen on the tarp, a smoking brazier.
    _leaves(b, [(0.15 + x, 0.55 + y, H + z) for x, y, z in ((-0.55, -0.30, 1.21), (-0.40, 0.10, 1.17), (-0.25, -0.15, 1.13),
                                                          (-0.10, 0.25, 1.09), (-0.50, 0.30, 1.20), (-0.20, -0.32, 1.12))], seed=2)
    _leaves(b, [(0.6, -1.0, H), (-0.6, -1.1, H), (0.95, -0.4, H), (-1.1, -0.3, H)], seed=3)
    _brazier(b, 0.82, -0.78)


def slink_winter(b):
    # A pot-bellied stove with its pipe, and a stack of firewood.
    x, y = 0.85, -0.70
    lathe(b, [(0.0, 0.0), (0.0, 0.14), (0.12, 0.17), (0.30, 0.17), (0.40, 0.13), (0.44, 0.06), (0.46, 0.0)], 10, IRON, at=(x, y, H + 0.06))
    for k in range(3):
        a = k * 2 * math.pi / 3 + 0.5
        tube(b, [(x + 0.11 * math.cos(a), y + 0.11 * math.sin(a), H), (x + 0.11 * math.cos(a), y + 0.11 * math.sin(a), H + 0.08)], 0.015, 4, IRON)
    box(b, (x, y - 0.165, H + 0.26), (0.12, 0.02, 0.10), COAL)
    tube(b, [(x, y, H + 0.50), (x, y, H + 1.30), (x + 0.05, y, H + 1.40)], 0.035, 6, IRON)
    for k, (dz, r) in enumerate(((1.50, 0.07), (1.68, 0.09), (1.88, 0.11))):
        sphere(b, (x + 0.06 + 0.03 * k, y, H + dz), r, SMOKE, sides=6, rings=4)
    for k in range(6):
        b.push((0.30 + (k % 3) * 0.12, -1.0 + (k // 3) * 0.0, H + 0.05 + (k // 3) * 0.10), (0, 90, 5 * k))
        lathe(b, [(-0.18, 0.05), (0.18, 0.05)], 7, (0.30, 0.17, 0.08))
        b.pop()


def folio_summer(b):
    # Pages pegged on a line to dry.
    tube(b, [(1.30, 0.80, H), (1.30, 0.80, H + 1.75)], 0.025, 6, OAK_D)
    tube(b, [(-1.25, 0.65, H + 1.75), (1.30, 0.80, H + 1.75)], 0.005, 3, (0.4, 0.35, 0.3))
    for k in range(6):
        x = -0.9 + k * 0.38
        y = 0.65 + (x + 1.25) / 2.55 * 0.15
        box(b, (x, y, H + 1.62), (0.18, 0.01, 0.24), CREAM if k % 2 else PARCH, rot=(0, 0, 3))


def folio_autumn(b):
    # Candles burning on the desk, leaves fallen on it and the plinth.
    for x, y in ((1.10, 0.0), (0.62, 0.02), (0.95, -0.40)):
        _candle(b, x, y, H + 0.80 + (y + 0.48) / 0.54 * 0.14, 0.10)
    _leaves(b, [(0.55, -0.38, H + 0.71), (0.95, -0.15, H + 0.76), (0.75, -0.05, H + 0.78)], seed=4)
    _leaves(b, [(0.3, -1.1, H), (-0.5, -1.0, H), (1.15, -0.85, H), (-1.0, -0.4, H), (0.0, -1.3, H)], seed=5)


def folio_winter(b):
    # A lamp post with its lantern lit, a plaid blanket folded on the tomes.
    x, y = 1.25, -0.75
    tube(b, [(x, y, H), (x, y, H + 1.45), (x - 0.18, y, H + 1.52)], 0.025, 6, IRON)
    lathe(b, [(0.0, 0.10), (0.05, 0.07), (0.05, 0.0)], 8, IRON, at=(x, y, H))
    tube(b, [(x - 0.18, y, H + 1.52), (x - 0.18, y, H + 1.44)], 0.005, 3, IRON)
    lantern(b, (x - 0.18, y, H + 1.34), FLAME, 1.0)
    zz = H + 0.09 * 5 + 0.03 * 5
    for k in range(4):
        box(b, (-0.75, -0.25, zz + 0.012 + k * 0.026), (0.30, 0.24, 0.024), [(0.55, 0.08, 0.06), (0.10, 0.25, 0.12)][k % 2])


DRESSING = {
    "goldwick": {"summer": goldwick_summer, "autumn": goldwick_autumn, "winter": goldwick_winter},
    "rosette": {"summer": rosette_summer, "autumn": rosette_autumn, "winter": rosette_winter},
    "slink": {"summer": slink_summer, "autumn": slink_autumn, "winter": slink_winter},
    "folio": {"summer": folio_summer, "autumn": folio_autumn, "winter": folio_winter},
}


# Where each keeper stands on their plinth: in front, in the open, so the gameplay camera sees them
# whole; the awning and the goods stay behind and beside.
KEEP_AT = {"goldwick": (0.0, -0.55), "rosette": (0.0, -0.50), "slink": (-0.15, -0.55), "folio": (0.0, -0.50)}

KEEPERS = {
    "goldwick": (goldwick, goldwick_stall, "Goldwick"),
    "rosette": (rosette, rosette_stall, "Madame Rosette"),
    "slink": (slink, slink_stall, "Slink"),
    "folio": (folio, folio_stall, "Old Folio"),
}


# --- Scenes and renders --------------------------------------------------------------------

def _fresh_scene(name):
    old = bpy.data.scenes.get(name)
    if old:
        for ob in list(old.collection.all_objects):
            bpy.data.objects.remove(ob, do_unlink=True)
        bpy.data.scenes.remove(old)
    sc = bpy.data.scenes.new(name)
    sc.render.engine = "BLENDER_WORKBENCH"
    sc.display_settings.display_device = "sRGB"
    sc.view_settings.view_transform = "Standard"
    sc.view_settings.exposure = 0.75
    sh = sc.display.shading
    sh.light = "STUDIO"
    sh.color_type = "VERTEX"
    sh.show_shadows = True
    sh.shadow_intensity = 0.45
    sh.show_cavity = True
    sh.cavity_type = "WORLD"
    sh.show_object_outline = True
    sh.object_outline_color = (0.02, 0.02, 0.02)
    sc.display.shadow_focus = 0.2
    sc.display.light_direction = (0.35, -0.45, 0.82)
    sc.render.film_transparent = False
    w = bpy.data.worlds.new(name)
    w.color = (0.45, 0.47, 0.50)
    sc.world = w
    return sc


def _ground(sc, size, color):
    b = B("ground")
    box(b, (0, 0, -0.05), (size, size, 0.1), color)
    return b.done(sc.collection)


def _import_glb(sc, path, at, rot_z=0.0, accent=None):
    """A reference model (a lane minion, a champion) for scale; its accent faces take `accent`."""
    before = set(bpy.data.objects)
    win = bpy.context.window
    prev = win.scene
    win.scene = sc
    bpy.ops.import_scene.gltf(filepath=path)
    win.scene = prev
    new = [o for o in bpy.data.objects if o not in before]
    for o in new:
        if o.parent is None:
            o.location = at
            o.rotation_euler = (0, 0, math.radians(rot_z))
        if o.type == "MESH" and accent:
            me = o.data
            attr = me.color_attributes.active_color or (me.color_attributes[0] if me.color_attributes else None)
            if attr is None:
                continue
            slots = [s.material.name.lower() if s.material else "" for s in o.material_slots]
            for p in me.polygons:
                if p.material_index < len(slots) and "accent" in slots[p.material_index]:
                    for li in p.loop_indices:
                        c = attr.data[li].color
                        k = 1.6 if "glow" in slots[p.material_index] else 1.0
                        attr.data[li].color = (min(1, c[0] * accent[0] * k), min(1, c[1] * accent[1] * k), min(1, c[2] * accent[2] * k), 1)
    for o in new:
        if o.type == "ARMATURE" and o.animation_data:
            o.animation_data.action = None
    return new


def _camera(sc, loc, look, lens=50.0, ortho=None):
    cam = bpy.data.cameras.new("cam")
    if ortho:
        cam.type = "ORTHO"
        cam.ortho_scale = ortho
    cam.lens = lens
    ob = bpy.data.objects.new("cam", cam)
    sc.collection.objects.link(ob)
    ob.location = loc
    d = Vector(look) - Vector(loc)
    ob.rotation_euler = d.to_track_quat("-Z", "Y").to_euler()
    sc.camera = ob
    return ob


def _render(sc, path, w, h):
    sc.render.resolution_x = w
    sc.render.resolution_y = h
    sc.render.resolution_percentage = 100
    sc.render.filepath = path
    win = bpy.context.window
    prev = win.scene
    win.scene = sc
    bpy.ops.render.render(write_still=True, scene=sc.name)
    win.scene = prev
    print("wrote", path)


def build_keeper(sc, key, at=(0, 0, 0), rot_z=0.0, stall=False):
    global TEAM
    fn, stall_fn, _ = KEEPERS[key]
    b = B(key)
    fn(b)
    ob = b.done(sc.collection)
    obs = [ob]
    if stall:
        s = B(key + "_stall")
        stall_fn(s)
        DRESSING[key]["summer"](s)
        so = s.done(sc.collection)
        obs.append(so)
        ob.location = (*KEEP_AT[key], 0.18)
        ob.parent = so
        so.location = at
        so.rotation_euler = (0, 0, math.radians(rot_z))
    else:
        ob.location = at
        ob.rotation_euler = (0, 0, math.radians(rot_z))
    for o in obs:
        for p in o.data.polygons:
            p.use_smooth = False
    return obs


def lineup(out):
    global TEAM
    TEAM = ALLY
    sc = _fresh_scene("Shopkeeper lineup")
    _ground(sc, 30, (0.16, 0.17, 0.18))
    order = ["minion", "goldwick", "rosette", "slink", "folio", "rook"]
    xs = [-4.2, -2.55, -0.85, 0.85, 2.55, 4.2]
    refs = {
        "minion": os.path.join(ROOT, "art", "minions", "melee", "export", "melee.glb"),
        "rook": os.path.join(ROOT, "art", "champions", "rook", "export", "rook.glb"),
    }
    for k, x in zip(order, xs):
        if k in refs:
            _import_glb(sc, refs[k], (x, 0, 0), 0.0, accent=ALLY)
        else:
            build_keeper(sc, k, (x, 0, 0), 0.0)
    _camera(sc, (0, -14, 2.2), (0, 0, 0.95), ortho=10.2)
    _render(sc, os.path.join(out, "lineup.png"), 2000, 760)
    sh = sc.display.shading
    sh.color_type, sh.single_color, sh.light = "SINGLE", (0, 0, 0), "FLAT"
    sh.show_shadows = sh.show_cavity = sh.show_object_outline = False
    sc.world.color = (1, 1, 1)
    for o in sc.objects:
        if o.name.startswith("ground"):
            o.hide_render = True
    _render(sc, os.path.join(out, "lineup_silhouette.png"), 2000, 760)


def closeups(out):
    global TEAM
    TEAM = ALLY
    for key in KEEPERS:
        sc = _fresh_scene("Shopkeeper " + key)
        _ground(sc, 20, (0.07, 0.14, 0.035))
        build_keeper(sc, key, (0, 0, 0), 0.0, stall=True)
        _camera(sc, (-3.4, -5.0, 3.1), (0.05, 0.05, 0.85), lens=42)
        _render(sc, os.path.join(out, key + "_stall.png"), 1400, 1050)


def gameplay(out):
    """The four stalls at the gameplay camera (R01: 56° pitch, 45° vertical FOV, 18.5 m)."""
    global TEAM
    sc = _fresh_scene("Shopkeeper base")
    _ground(sc, 60, (0.07, 0.14, 0.035))
    b = B("paving")
    box(b, (0, 0, -0.03), (17, 9, 0.06), (0.12, 0.13, 0.15))
    b.done(sc.collection)
    for key, x, side in (("goldwick", -6.0, ALLY), ("rosette", -2.0, ALLY), ("slink", 2.0, ENEMY), ("folio", 6.0, ENEMY)):
        TEAM = side
        build_keeper(sc, key, (x, 0.6, 0), 0.0, stall=True)
    TEAM = ALLY
    _import_glb(sc, os.path.join(ROOT, "art", "minions", "melee", "export", "melee.glb"), (-3.6, -2.6, 0), 0.0, accent=ALLY)
    _import_glb(sc, os.path.join(ROOT, "art", "champions", "rook", "export", "rook.glb"), (-0.2, -2.4, 0), 0.0, accent=ALLY)
    pitch = math.radians(56)
    d = 18.5
    look = Vector((0, 0, 0.5))
    loc = look + Vector((0, -math.cos(pitch) * d, math.sin(pitch) * d))
    cam = _camera(sc, tuple(loc), tuple(look))
    cam.data.sensor_fit = "VERTICAL"
    cam.data.angle_y = math.radians(45)
    _render(sc, os.path.join(out, "base_gameplay.png"), 1920, 1080)


def main(out):
    os.makedirs(out, exist_ok=True)
    lineup(out)
    closeups(out)
    gameplay(out)


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.join(HERE, "review")
    main(out)
