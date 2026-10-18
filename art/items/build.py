# SPDX-License-Identifier: AGPL-3.0-or-later
"""Every item's model (18 §8), built from code on the item toolkit, saved and exported:

    blender -b --python art/items/build.py -- [--only key,key] [--review DIR]

For each item (`ITEMS`, keyed by its name in snake case: "Pilgrim's Ledger" is
`pilgrims_ledger`) this writes `art/items/<key>/<key>.blend` and its `prop` pack in
`art/items/<key>/export/`. With `--review`, it also renders each item from the icon's angle
(`<key>.png`) and a contact sheet of them all (`sheet.png`) into DIR.

The look (18 §8): one silhouette per item, readable in black; families share a motif (blades
for attack damage, staffs and crystals for ability power, plates for armor, cloth and runes
for magic resist, gems for health), and a lineage's epic and legendary share a shape that
grows; color says the stat: steel and red for AD, violet for AP, ruby for health, bronze for
armor, teal for magic resist, gold for attack speed and crits, cyan for haste.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

from mftr_blender import export, prop  # noqa: E402
from mftr_blender.item import *  # noqa: E402,F401,F403
from mftr_blender.item import Item  # noqa: E402

ITEMS = {}


def model(key):
    def deco(fn):
        ITEMS[key] = fn
        return fn

    return deco


def V(x, y, z):
    return Vector((x, y, z))


# ---- Shared shapes -------------------------------------------------------------------------


def lighter(c, k=1.3):
    return tuple(min(1.0, x * k) for x in c)


def darker(c, k=0.6):
    return tuple(x * k for x in c)


def guard_arc(it, z, half, rise, r, color, slot="metal", n=6):
    """A crossguard whose quillons sweep up (`rise` > 0) or down at the ends."""
    path = [V(-half + 2 * half * i / n, 0, z + rise * (abs(-1 + 2 * i / n) ** 2)) for i in range(n + 1)]

    def sec(i, t):
        k = 1.0 - 0.45 * abs(2 * t - 1)
        return [(r * k * math.cos(2 * math.pi * j / 5), r * k * 0.8 * math.sin(2 * math.pi * j / 5)) for j in range(5)]

    it.sweep(path, sec, slot, color, 0.06)
    for x in (-half, half):
        it.orb((x, 0, z + rise), r * 1.3, slot, color, subdiv=0)


def sword(it, length=1.0, width=0.07, tip=0.0, guard=0.13, rise=0.0, grip=0.22, color=STEEL, edge=None,
          guard_color=BRONZE, grip_color=LEATHER, pommel=None, pommel_gem=None, fuller=None, curve=None,
          stations=None, thick=0.018, guard_gem=None, band=None):
    """A sword standing on its pommel, blade up: pommel, wrapped grip, crossguard, blade."""
    it.grip(V(0, 0, 0), V(0, 0, grip), 0.026, wraps=4, color=grip_color, band=band)
    guard_arc(it, grip + 0.02, guard, rise, 0.022, guard_color)
    it.bar((0, 0, grip + 0.035), (width * 1.6, 0.05, 0.05), "metal", guard_color)
    if guard_gem is not None:
        it.gem((0, -0.028, grip + 0.035), 0.026, guard_gem, n=5)
    st = stations or [(0.0, width), (0.15, width * 1.02), (0.7, width * 0.9), (0.9, width * 0.6), (1.0, tip)]
    base = grip + 0.06
    it.blade(V(0, 0, base), V(0, 0, base + length), st, thick, color=color, edge=edge or lighter(color, 1.35), curve=curve)
    if fuller is not None:
        it.blade(V(0, -0.0005, base + 0.03), V(0, -0.0005, base + length * 0.62), [(0, width * 0.22), (0.9, width * 0.2), (1, 0)],
                 thick * 1.08, color=fuller, edge=None, curve=(lambda t: curve(t * 0.62)) if curve else None)
    pc = pommel or guard_color
    it.lathe(V(0, 0, -0.06), [(0.0, 0.0), (0.015, 0.03), (0.045, 0.036), (0.065, 0.02)], 6, "metal", pc, 0.05)
    if pommel_gem is not None:
        it.gem((0, -0.03, -0.03), 0.022, pommel_gem, n=5)


def dagger(it, length=0.42, width=0.05, color=STEEL_DARK, grip_color=LEATHER, guard_color=IRON, **kw):
    sword(it, length=length, width=width, guard=0.09, grip=0.15, color=color, grip_color=grip_color,
          guard_color=guard_color, **kw)


def bottle(it, body, neck, color, glass=None, cork=WOOD, n=8, z=0.0):
    """A potion bottle: `body` [(z, r)] of liquid in glass, a neck, a lip and a cork."""
    it.lathe(V(0, 0, z), body, n, "emissive", color, 0.06)
    top = z + body[-1][0]
    g = glass or lin((0.70, 0.80, 0.84))
    it.lathe(V(0, 0, top), [(0.0, neck), (0.1, neck * 0.9), (0.12, neck * 1.3), (0.16, neck * 1.3), (0.16, neck * 0.8)], n, "skin", g, 0.04)
    it.lathe(V(0, 0, top + 0.13), [(0.0, neck * 0.82), (0.1, neck * 0.95), (0.12, neck * 0.7)], 6, "cloth", cork, 0.1)
    # A highlight streak on the glass.
    r0 = max(r for _, r in body)
    zm = z + body[len(body) // 2][0]
    it.bar((-r0 * 0.55, -r0 * 0.78, zm), (0.025, 0.01, r0 * 0.7), "emissive", lighter(color, 2.2), 0.0, rot=-15)


def boot(it, color=LEATHER, sole=LEATHER_DARK, cuff=None, height=0.42, toe=0.30, wing=None, plate=None, n=8,
         lace=None, toe_curl=0.0, rune=None, spikes=None, buckle=None):
    """A boot in profile, toe to the right: a shaft, the foot, a sole; options add a cuff, wings,
    plates, laces, a curled toe, glowing runes, spikes."""
    shaft_slot = "metal" if plate else "cloth"
    shaft = plate or color
    it.lathe(V(0, 0, 0.1), [(0.0, 0.088, 0.092), (0.12, 0.08, 0.085), (height - 0.15, 0.088, 0.092), (height - 0.1, 0.094, 0.098)],
             n, shaft_slot, shaft, 0.08)
    path = [V(-0.07, 0, 0.0), V(0.03, 0, 0.0), V(toe * 0.55, 0, 0.0), V(toe * 0.85, 0, 0.0), V(toe, 0, toe_curl * 0.5),
            V(toe + toe_curl * 0.3, 0, toe_curl)]
    hs = [0.17, 0.15, 0.1, 0.075, 0.05, 0.02]
    ws = [0.085, 0.09, 0.085, 0.075, 0.05, 0.015]

    def sec(i, t):
        w, h = ws[i], hs[i]
        return [(-w, 0.0), (-w * 0.95, h * 0.55), (-w * 0.6, h * 0.92), (0.0, h), (w * 0.6, h * 0.92), (w * 0.95, h * 0.55), (w, 0.0)]

    it.sweep([p + V(0, 0, 0.03) for p in path], sec, shaft_slot if plate else "cloth", shaft if plate else color, 0.08, normal=(0, 0, 1))
    it.lathe(V(toe * 0.4, 0, 0.0), [(0.0, toe * 0.62, 0.1), (0.035, toe * 0.64, 0.105)], 10, "cloth", sole, 0.06)
    if cuff is not None:
        it.lathe(V(0, 0, height - 0.04), [(0.0, 0.1, 0.105), (0.04, 0.115, 0.12), (0.08, 0.11, 0.115)], n, "cloth", cuff, 0.08)
    if lace is not None:
        for k in range(4):
            z = 0.12 + k * 0.065
            it.bar((0.075, -0.0, z), (0.02, 0.17, 0.016), "cloth", lace, 0.05, rot=12)
    if wing is not None:
        for k in range(3):
            dz = k * 0.06
            it.extrude([(-0.07, height - 0.22 + dz), (-0.28 + k * 0.02, height - 0.08 + dz * 1.6), (-0.24 + k * 0.03, height - 0.02 + dz * 1.4),
                        (-0.06, height - 0.12 + dz)], 0.015, "cloth", lighter(wing, 1.0 - 0.12 * k), y=-0.096, bevel=0.0)
    if rune is not None:
        for k in range(3):
            it.bar((0.0, -0.093, 0.16 + k * 0.08), (0.05, 0.008, 0.012 + 0.01 * (k % 2)), "emissive", rune, 0.0, rot=20 * (k - 1))
    if spikes is not None:
        for k in range(3):
            it.lathe(V(-0.09, 0, 0.16 + k * 0.08), [(0.0, 0.02), (0.08, 0.0)], 4, "metal", spikes, 0.05, axis=(-1, -0.4, 0.3))
    if buckle is not None:
        it.bar((0.0, -0.095, height * 0.55), (0.07, 0.012, 0.05), "metal", buckle)
        it.bar((0.0, -0.09, height * 0.55), (0.19, 0.008, 0.025), "cloth", darker(color, 0.7))


def boots_pair(it, **kw):
    boot(it, **kw)


def book(it, color, w=0.5, h=0.62, thick=0.15, clasp=GOLD, pages=PAPER, sigil=None, corners=None, spine=None):
    """A closed book standing up, its cover facing the viewer; pages show at the right edge."""
    it.bar((0, 0, h / 2), (w, thick, h), "cloth", color, 0.08)
    it.bar((0.02, 0, h / 2), (w - 0.02, thick - 0.04, h - 0.04), "skin", pages, 0.03)
    it.bar((-w / 2 - 0.005, 0, h / 2), (0.04, thick + 0.01, h + 0.01), "cloth", spine or darker(color, 0.75), 0.06)
    for z in (h * 0.15, h * 0.5, h * 0.85):
        it.bar((-w / 2 - 0.01, 0, z), (0.05, thick + 0.02, 0.03), "metal", clasp)
    it.bar((w / 2 + 0.005, -thick * 0.3, h / 2), (0.03, thick * 0.5, 0.1), "metal", clasp)
    if corners is not None:
        for x in (-1, 1):
            for z in (0, 1):
                it.bar((x * (w / 2 - 0.035), -thick / 2 - 0.004, 0.035 + z * (h - 0.07)), (0.07, 0.012, 0.07), "metal", corners, rot=45)
    if sigil is not None:
        it.gem((0, -thick / 2 - 0.012, h * 0.55), 0.085, sigil, n=6)
        it.torus((0, -thick / 2 - 0.008, h * 0.55), 0.11, 0.012, "metal", clasp, n=12, m=4)


def torso(it, color, slot="cloth", w=0.3, d=0.17, h=0.6, collar=None, neck=0.1, sleeves=True, n=10):
    """A sleeveless or short-sleeved coat seen from the front: an elliptical lathe, narrow at
    the waist, broad at the chest, shoulders and a collar."""
    prof = [(0.0, w * 0.86, d * 0.9), (0.12, w * 0.8, d * 0.85), (0.3, w * 0.92, d * 0.95), (0.46, w, d), (0.56, w * 0.88, d * 0.9),
            (0.62, w * 0.42, d * 0.55), (0.64, neck, neck * 0.8)]
    it.lathe(V(0, 0, 0), [(z * h / 0.64, rx, ry) for z, rx, ry in prof], n, slot, color, 0.1, cap_top=False)
    if sleeves:
        for s in (-1, 1):
            it.lathe(V(s * w * 0.88, 0, h * 0.8), [(0.0, 0.09, 0.08), (0.16, 0.08, 0.075)], 7, slot, darker(color, 0.85), 0.1,
                     axis=(s * 0.8, 0, -0.5))
    if collar is not None:
        it.lathe(V(0, 0, h * 0.94), [(0.0, neck * 1.25, neck * 1.05), (0.06, neck * 1.15, neck * 0.95)], n, "metal", collar, 0.05)


def ribbon(it, path, w, color, slot="cloth", thick=0.01, twist=None):
    """A flat band along `path`, `w` wide (a sash, a fluttering tail)."""

    def sec(i, t):
        ww = w(t) if callable(w) else w
        return [(-ww, -thick), (ww, -thick), (ww, thick), (-ww, thick)]

    it.sweep(path, sec, slot, color, 0.08)


def wavy(a, b, n, amp, waves, side=(1, 0, 0)):
    a, b, s = Vector(a), Vector(b), Vector(side)
    return [a + (b - a) * (i / n) + s * (amp * math.sin(waves * math.pi * i / n) * (i / n)) for i in range(n + 1)]


def bow(it, half=0.55, bend=0.18, color=WOOD, tips=GOLD, string=PAPER, grip_color=LEATHER, r=0.022, recurve=0.06, n=12):
    """A bow standing up, its back to the left: limbs sweep from the grip to curled tips."""
    pts = []
    for i in range(n + 1):
        t = -1 + 2 * i / n
        x = -bend * (1 - t * t) + recurve * (abs(t) ** 6)
        pts.append(V(x, 0, half * t + half))

    def sec(i, t):
        k = 1.0 - 0.55 * abs(2 * t - 1)
        return [(r * k * math.cos(2 * math.pi * j / 5), r * k * 1.3 * math.sin(2 * math.pi * j / 5)) for j in range(5)]

    it.sweep(pts, sec, "cloth", color, 0.08)
    it.grip(V(-bend, 0, half - 0.08), V(-bend, 0, half + 0.08), r * 1.3, wraps=3, color=grip_color)
    for p in (pts[0], pts[-1]):
        it.orb(p, r * 1.4, "metal", tips, subdiv=0)
    it.rod(pts[0], pts[-1], 0.004, 3, "skin", string, 0.0)
    return pts


def chest_plate(it, color, trim=None, slot="metal", w=0.32, h=0.56, d=0.18, pauldrons=True):
    torso(it, color, slot, w=w, d=d, h=h, collar=trim, sleeves=False)
    # The breastplate's ridge down the middle.
    it.bar((0, -d * 0.98, h * 0.55), (0.03, 0.03, h * 0.6), slot, lighter(color, 1.2))
    if pauldrons:
        for s in (-1, 1):
            it.orb((s * w * 0.95, 0, h * 0.86), 0.13, slot, lighter(color, 1.08), subdiv=1, squash=(1.1, 1.0, 0.7))
            if trim is not None:
                it.torus((s * w * 0.95, 0, h * 0.8), 0.12, 0.014, "metal", trim, n=10, m=4, axis=(0, 0, 1))
    if trim is not None:
        it.lathe(V(0, 0, 0.0), [(0.0, w * 0.9, d * 0.95), (0.05, w * 0.9, d * 0.95)], 10, "metal", trim, 0.05)


def egg(it, color, h=0.5, r=0.2, crack=None, n=9):
    prof = [(0.0, 0.0), (0.03, r * 0.6), (0.12, r * 0.95), (0.24, r), (0.36, r * 0.85), (0.45, r * 0.55), (0.5, 0.0)]
    it.lathe(V(0, 0, 0), [(z * h / 0.5, rr) for z, rr in prof], n, "skin", color, 0.1, jitter=0.03)
    if crack is not None:
        zig = [(-0.16, 0.26), (-0.1, 0.31), (-0.05, 0.25), (0.0, 0.32), (0.06, 0.26), (0.11, 0.31), (0.17, 0.25)]
        for (x0, z0), (x1, z1) in zip(zig, zig[1:]):
            it.rod(V(x0, -r * 0.93, z0 * h / 0.5), V(x1, -r * 0.93, z1 * h / 0.5), 0.008, 3, "emissive", crack, 0.0)


def lantern(it, light, frame=BRONZE, glass_slot="emissive", h=0.5, w=0.15, n=6, cap=None, handle=True, cage=True):
    """A lantern: a base, a glowing glass body in a cage, a roof and a ring to carry it by."""
    it.lathe(V(0, 0, 0), [(0.0, w * 1.15), (0.04, w * 1.2), (0.07, w)], n, "metal", frame, 0.05)
    it.lathe(V(0, 0, 0.07), [(0.0, w * 0.85), (h * 0.3, w * 0.95), (h * 0.55, w * 0.85)], n, glass_slot, light, 0.04)
    if cage:
        for i in range(n):
            a = 2 * math.pi * (i + 0.5) / n
            p0 = V(w * 0.98 * math.cos(a), w * 0.98 * math.sin(a), 0.07)
            p1 = V(w * 0.92 * math.cos(a), w * 0.92 * math.sin(a), 0.07 + h * 0.55)
            it.rod(p0, p1, 0.01, 4, "metal", frame, 0.04)
    top = 0.07 + h * 0.55
    it.lathe(V(0, 0, top), [(0.0, w * 1.25), (0.03, w * 1.25), (h * 0.22, w * 0.3), (h * 0.26, 0.02)], n, "metal", cap or frame, 0.05)
    if handle:
        it.torus((0, 0, top + h * 0.33), 0.06, 0.012, "metal", frame, n=10, m=4, axis=(0, 1, 0))


def staff(it, length, color=WOOD, r=0.024):
    """A staff standing up, thickening toward its head."""
    it.rod(V(0, 0, 0), V(0, 0, length), r, 6, "cloth", color, 0.1,
           profile=[(0.0, r * 0.8), (0.5, r), (0.9, r * 1.15), (1.0, r * 1.4)])
    it.lathe(V(0, 0, 0), [(0.0, r * 1.1), (0.04, r * 1.1)], 6, "metal", BRONZE, 0.04)


def crystal(it, base, height, r, color, sides=6, axis=(0, 0, 1), slot="emissive", tip=0.25):
    it.lathe(V(*base), [(0.0, 0.0), (height * 0.12, r), (height * (1 - tip), r * 0.92), (height, 0.0)], sides, slot, color, 0.06,
             axis=axis, rot=0.3)


# ---- Starters ------------------------------------------------------------------------------


@model("squires_blade")
def squires_blade(it):
    """A squire's short sword: bronze guard, a red pommel stone."""
    sword(it, length=0.58, width=0.06, guard=0.12, rise=0.03, grip=0.17, color=STEEL, guard_color=BRONZE,
          pommel_gem=RUBY, fuller=STEEL_DARK, band=BRONZE)
    it.pose(-45)


@model("squires_ring")
def squires_ring(it):
    """A gold signet ring with a violet stone."""
    it.torus((0, 0, 0.2), 0.2, 0.045, "metal", GOLD, n=18, m=6, axis=(0.15, 1, 0.35))
    it.bar((0, -0.06, 0.4), (0.18, 0.1, 0.07), "metal", GOLD_DARK, rot=0)
    it.gem((0, -0.1, 0.42), 0.075, AMETHYST, n=8, axis=(0, -1, 0.5))
    for x in (-0.085, 0.085):
        it.orb((x, -0.1, 0.4), 0.022, "metal", GOLD, subdiv=0)


@model("squires_buckler")
def squires_buckler(it):
    """A small round shield: planks, an iron rim and boss."""
    it.disc((0, 0, 0.3), 0.3, 0.05, "cloth", WOOD, n=14)
    for k in range(-2, 3):
        it.bar((k * 0.11, -0.027, 0.3), (0.006, 0.005, 0.5 - 0.06 * abs(k)), "cloth", WOOD_DARK, 0.0)
    it.torus((0, 0, 0.3), 0.3, 0.025, "metal", IRON, n=16, m=4, axis=(0, 1, 0))
    it.lathe(V(0, -0.025, 0.3), [(0.0, 0.1), (0.04, 0.09), (0.08, 0.03)], 8, "metal", STEEL, 0.05, axis=(0, -1, 0))
    for i in range(8):
        a = 2 * math.pi * i / 8
        it.orb((0.24 * math.cos(a), -0.03, 0.3 + 0.24 * math.sin(a)), 0.016, "metal", STEEL_DARK, subdiv=0)
    it.pose(10)


@model("tally_charm")
def tally_charm(it):
    """A bone charm scored with tally marks, a dark violet seal stone, on a cord."""
    it.extrude([(-0.12, 0.0), (0.12, 0.0), (0.16, 0.3), (0.1, 0.46), (-0.1, 0.46), (-0.16, 0.3)], 0.06, "skin", BONE, bevel=0.015)
    for k in range(4):
        it.bar((-0.08 + k * 0.035, -0.033, 0.12), (0.01, 0.006, 0.12), "skin", darker(BONE, 0.45), 0.0)
    it.bar((0.04, -0.033, 0.12), (0.16, 0.006, 0.01), "skin", darker(BONE, 0.45), 0.0, rot=30)
    it.gem((0, -0.035, 0.33), 0.06, AMETHYST, n=6)
    it.torus((0, 0, 0.5), 0.045, 0.012, "metal", GOLD_DARK, n=8, m=4)
    ribbon(it, [V(-0.03, 0, 0.53), V(-0.12, 0, 0.66), V(-0.2, 0, 0.72)], 0.008, LEATHER_DARK)
    ribbon(it, [V(0.03, 0, 0.53), V(0.12, 0, 0.66), V(0.2, 0, 0.72)], 0.008, LEATHER_DARK)


@model("pilgrims_ledger")
def pilgrims_ledger(it):
    """A traveler's ledger tied with cord, a gold coin tucked under it."""
    book(it, LEATHER, w=0.42, h=0.52, thick=0.13, clasp=BRONZE, corners=BRONZE)
    it.bar((0, -0.067, 0.26), (0.43, 0.01, 0.02), "cloth", PAPER, 0.04)
    it.bar((0, -0.067, 0.26), (0.02, 0.01, 0.53), "cloth", PAPER, 0.04)
    it.disc((0.2, -0.09, 0.06), 0.08, 0.02, "metal", GOLD, n=10)
    it.rod(V(-0.2, -0.05, 0.4), V(0.05, -0.12, 0.72), 0.008, 3, "skin", WHITE, 0.0)  # a quill
    it.extrude([(0.0, 0.62), (0.05, 0.6), (0.06, 0.74), (0.02, 0.76)], 0.01, "skin", WHITE, y=-0.11)


@model("pilgrims_astrolabe")
def pilgrims_astrolabe(it):
    """A brass astrolabe: nested rings around a star stone, a pointer, a hanging loop."""
    c = (0, 0, 0.32)
    it.disc(c, 0.28, 0.03, "metal", BRONZE, n=16)
    it.torus(c, 0.28, 0.022, "metal", GOLD, n=18, m=4)
    it.torus(c, 0.2, 0.014, "metal", GOLD_DARK, n=16, m=4, axis=(0.3, 1, 0.2))
    it.torus(c, 0.15, 0.012, "metal", GOLD, n=14, m=4, axis=(-0.4, 1, 0.3))
    it.bar((0, -0.03, 0.32), (0.5, 0.01, 0.02), "metal", GOLD, rot=30)
    it.gem((0, -0.04, 0.32), 0.05, CYAN, n=8)
    it.torus((0, 0, 0.64), 0.05, 0.014, "metal", GOLD, n=10, m=4)
    for i in range(12):
        a = 2 * math.pi * i / 12
        it.bar((0.24 * math.cos(a), -0.02, 0.32 + 0.24 * math.sin(a)), (0.012, 0.01, 0.035), "metal", GOLD_DARK, 0.0, rot=-math.degrees(a) + 90)


@model("sunward_lantern")
def sunward_lantern(it):
    """A gold lantern burning white-gold, sun rays around its crown."""
    lantern(it, lin((1.0, 0.92, 0.6)), frame=GOLD, h=0.52, w=0.15)
    for i in range(8):
        a = 2 * math.pi * i / 8
        it.lathe(V(0.17 * math.cos(a), 0.17 * math.sin(a), 0.4), [(0.0, 0.025), (0.09, 0.0)], 3, "emissive", TOPAZ, 0.0,
                 axis=(math.cos(a), math.sin(a), 0.6))


@model("bulwark_lantern")
def bulwark_lantern(it):
    """A heavy iron lantern, a teal light behind thick bars."""
    lantern(it, TEAL_GLOW, frame=IRON, h=0.5, w=0.17, n=4)
    for z in (0.15, 0.3):
        it.lathe(V(0, 0, z), [(0.0, 0.19), (0.025, 0.19)], 4, "metal", STEEL_DARK, 0.05)


@model("hunters_lantern")
def hunters_lantern(it):
    """A hooded red lantern, a spike on top: a light for the hunt."""
    lantern(it, lin((1.0, 0.25, 0.15)), frame=darker(BRONZE, 0.7), h=0.48, w=0.14, cap=CLOTH_RED)
    it.lathe(V(0, 0, 0.5), [(0.0, 0.03), (0.2, 0.0)], 4, "metal", STEEL, 0.04)


# ---- Consumables ---------------------------------------------------------------------------


@model("health_potion")
def health_potion(it):
    bottle(it, [(0.0, 0.1), (0.04, 0.16), (0.16, 0.19), (0.28, 0.16), (0.36, 0.08), (0.4, 0.05)], 0.045, RUBY)


@model("refillable_flask")
def refillable_flask(it):
    """A leather-cased hip flask, its window showing red, a steel cap on a chain."""
    it.lathe(V(0, 0, 0), [(0.0, 0.17, 0.06), (0.06, 0.2, 0.075), (0.34, 0.2, 0.075), (0.42, 0.15, 0.06), (0.46, 0.05, 0.04)], 10,
             "cloth", LEATHER, 0.08)
    it.extrude([(-0.08, 0.1), (0.08, 0.1), (0.08, 0.32), (-0.08, 0.32)], 0.01, "emissive", RUBY, y=-0.078)
    it.lathe(V(0, 0, 0.45), [(0.0, 0.05), (0.07, 0.05), (0.08, 0.035)], 8, "metal", STEEL, 0.05)
    for z in (0.06, 0.38):
        it.lathe(V(0, 0, z), [(0.0, 0.205, 0.08), (0.025, 0.205, 0.08)], 10, "metal", BRONZE, 0.05)
    it.chain(V(0.05, -0.02, 0.5), V(0.19, -0.03, 0.36), 4, 0.018)


@model("control_ward")
def control_ward(it):
    """A ward post: a stake crowned by a watching red eye."""
    it.rod(V(0, 0, 0), V(0, 0, 0.5), 0.03, 5, "cloth", WOOD_DARK, 0.1, profile=[(0.0, 0.01), (0.15, 0.03), (1.0, 0.035)])
    it.lathe(V(0, 0, 0.48), [(0.0, 0.06), (0.06, 0.12), (0.2, 0.12), (0.26, 0.05)], 6, "metal", IRON, 0.06)
    it.orb((0, -0.05, 0.62), 0.09, "skin", WHITE, subdiv=1, squash=(1.0, 0.7, 0.8))
    it.gem((0, -0.115, 0.62), 0.05, lin((1.0, 0.2, 0.35)), n=8)
    for s in (-1, 1):
        it.lathe(V(s * 0.1, 0, 0.7), [(0.0, 0.03), (0.16, 0.0)], 4, "metal", IRON, 0.04, axis=(s * 0.5, 0, 1))


# ---- Components ----------------------------------------------------------------------------


@model("long_knife")
def long_knife(it):
    """A seax: a straight back, the edge sweeping up to an angled point; iron and leather. The
    AD line's first blade."""
    it.grip(V(0, 0, 0), V(0, 0, 0.18), 0.026, wraps=4, color=LEATHER)
    it.bar((0, 0, 0.19), (0.12, 0.05, 0.035), "metal", IRON)
    out = [(-0.035, 0.2), (0.04, 0.2), (0.05, 0.5), (0.045, 0.7), (-0.035, 0.84), (-0.035, 0.2)]
    it.extrude(out[:-1], 0.022, "metal", STEEL, bevel=0.012)
    it.rod(V(0.048, -0.012, 0.22), V(0.048, -0.012, 0.66), 0.006, 3, "metal", lighter(STEEL, 1.4), 0.0)
    it.rod(V(0.046, -0.012, 0.68), V(-0.03, -0.012, 0.82), 0.006, 3, "metal", lighter(STEEL, 1.4), 0.0)
    it.lathe(V(0, 0, -0.05), [(0.0, 0.0), (0.015, 0.03), (0.05, 0.02)], 6, "metal", IRON, 0.05)
    it.pose(-45)


@model("spark_shard")
def spark_shard(it):
    """A violet crystal shard with chips of light orbiting it: the AP line's first stone."""
    crystal(it, (0, 0, 0), 0.62, 0.09, AMETHYST, sides=5, axis=(0.15, 0, 1))
    crystal(it, (0.05, 0, 0.02), 0.3, 0.05, lighter(AMETHYST, 1.1), sides=4, axis=(0.6, 0, 1))
    for i in range(5):
        a = 2 * math.pi * i / 5
        it.orb((0.2 * math.cos(a), 0.2 * math.sin(a) * 0.5, 0.25 + 0.15 * math.sin(a * 2)), 0.025, "emissive",
               lin((0.85, 0.7, 1.0)), subdiv=0)


@model("vital_crystal")
def vital_crystal(it):
    """A ruby crystal cluster in a gold cradle: health."""
    crystal(it, (0, 0, 0.04), 0.55, 0.12, RUBY)
    crystal(it, (0.12, 0, 0.04), 0.33, 0.07, GARNET, sides=5, axis=(0.5, 0.1, 1))
    crystal(it, (-0.11, 0, 0.04), 0.28, 0.06, GARNET, sides=5, axis=(-0.6, -0.1, 1))
    it.lathe(V(0, 0, -0.02), [(0.0, 0.2), (0.04, 0.22), (0.08, 0.15)], 8, "metal", GOLD, 0.06)
    for i in range(4):
        a = 2 * math.pi * i / 4 + 0.4
        it.lathe(V(0.15 * math.cos(a), 0.15 * math.sin(a), 0.05), [(0.0, 0.025), (0.12, 0.0)], 4, "metal", GOLD_DARK, 0.04,
                 axis=(-math.cos(a) * 0.3, -math.sin(a) * 0.3, 1))


@model("padded_vest")
def padded_vest(it):
    """A quilted gambeson, buttoned in bronze: armor's first layer."""
    tan = lin((0.66, 0.54, 0.36))
    torso(it, tan, "cloth", w=0.28, d=0.16, h=0.56)
    for k in range(6):
        it.lathe(V(0, 0, 0.05 + k * 0.08), [(0.0, 0.25 + 0.02 * math.sin(k), 0.15), (0.015, 0.252 + 0.02 * math.sin(k), 0.152)], 10,
                 "cloth", darker(tan, 0.7), 0.04)
    for k in range(4):
        it.orb((0, -0.163, 0.12 + k * 0.1), 0.018, "metal", BRONZE, subdiv=0)


@model("warding_cloak")
def warding_cloak(it):
    """A hooded teal cloak, its clasp a silver rune: magic resist."""
    it.lathe(V(0, 0, 0), [(0.0, 0.3, 0.2), (0.25, 0.24, 0.16), (0.48, 0.15, 0.12), (0.56, 0.1, 0.1)], 9, "cloth", CLOTH_TEAL, 0.12,
             cap_bottom=False)
    it.orb((0, 0.03, 0.62), 0.13, "cloth", darker(CLOTH_TEAL, 0.8), subdiv=1, squash=(1.0, 1.0, 1.15))
    it.orb((0, -0.04, 0.6), 0.09, "skin", BLACK, subdiv=1, squash=(0.9, 0.6, 1.0))
    it.lathe(V(0, 0, 0.0), [(0.0, 0.305, 0.205), (0.03, 0.3, 0.2)], 9, "cloth", SILVER, 0.04)
    it.gem((0, -0.13, 0.48), 0.04, CYAN, n=6)
    it.torus((0, -0.12, 0.48), 0.05, 0.01, "metal", SILVER, n=10, m=4)


@model("quick_dagger")
def quick_dagger(it):
    """A light dagger wrapped in gold: attack speed."""
    dagger(it, length=0.4, width=0.045, color=STEEL, grip_color=lin((0.8, 0.6, 0.15)), guard_color=GOLD, band=GOLD)
    it.pose(-50)


@model("focus_charm")
def focus_charm(it):
    """A brass hourglass running cyan sand: haste."""
    it.disc((0, 0, 0.02), 0.16, 0.04, "metal", BRONZE, n=6, axis=(0, 0, 1))
    it.disc((0, 0, 0.5), 0.16, 0.04, "metal", BRONZE, n=6, axis=(0, 0, 1))
    for i in range(3):
        a = 2 * math.pi * i / 3 + 0.5
        it.rod(V(0.13 * math.cos(a), 0.13 * math.sin(a), 0.03), V(0.13 * math.cos(a), 0.13 * math.sin(a), 0.5), 0.012, 4, "cloth",
               WOOD_DARK, 0.05)
    it.lathe(V(0, 0, 0.04), [(0.0, 0.1), (0.15, 0.09), (0.21, 0.02), (0.25, 0.09), (0.44, 0.1)], 8, "skin", lin((0.7, 0.85, 0.9)), 0.03,
             cap_top=False, cap_bottom=False)
    it.lathe(V(0, 0, 0.04), [(0.0, 0.085), (0.06, 0.08), (0.12, 0.0)], 8, "emissive", CYAN, 0.04)
    it.lathe(V(0, 0, 0.29), [(0.0, 0.02), (0.1, 0.075), (0.13, 0.07)], 8, "emissive", CYAN, 0.04)
    it.rod(V(0, 0, 0.12), V(0, 0, 0.26), 0.006, 3, "emissive", CYAN, 0.0)


@model("fortunes_die")
def fortunes_die(it):
    """An ivory die with gold pips, a second one tumbling behind: critical strikes."""
    def die(c, s, rot):
        it.bar(c, (s, s, s), "skin", BONE, 0.05, rot=rot)
        pip = lambda p: it.orb(p, s * 0.09, "metal", GOLD, subdiv=0)
        cx, cy, cz = c
        r = math.radians(rot)
        for dx, dz in ((0, 0), (-0.25, -0.25), (0.25, 0.25)):
            x, z = dx * s, dz * s
            pip((cx + x * math.cos(r) + z * math.sin(r), cy - s / 2, cz - x * math.sin(r) + z * math.cos(r)))
    die((0, 0, 0.22), 0.4, 18)
    die((0.3, 0.2, 0.45), 0.2, -25)


@model("heavy_pick")
def heavy_pick(it):
    """A miner's war pick: a long haft, an iron head that curves to a spike."""
    it.rod(V(0, 0, 0), V(0, 0, 0.9), 0.025, 6, "cloth", WOOD, 0.1)
    it.grip(V(0, 0, 0.0), V(0, 0, 0.22), 0.03, wraps=3, color=LEATHER_DARK)
    path = [V(-0.32, 0, 0.74), V(-0.15, 0, 0.84), V(0, 0, 0.87), V(0.15, 0, 0.84), V(0.28, 0, 0.78)]
    it.sweep(path, lambda i, t: [(0.045 * (1 - abs(2 * t - 1) * 0.8), 0.0), (0.0, 0.035 * (1 - abs(2 * t - 1) * 0.6)),
                                 (-0.05 * (1 - abs(2 * t - 1) * 0.8), 0.0), (0.0, -0.035 * (1 - abs(2 * t - 1) * 0.6))],
             "metal", IRON, 0.06)
    it.bar((0, 0, 0.86), (0.08, 0.08, 0.1), "metal", STEEL_DARK)
    it.pose(-30)


@model("charged_wand")
def charged_wand(it):
    """A silver-banded wand, a violet stone crackling at its tip."""
    it.rod(V(0, 0, 0), V(0, 0, 0.6), 0.02, 6, "cloth", WOOD_DARK, 0.1, profile=[(0.0, 0.014), (0.8, 0.022), (1.0, 0.03)])
    for z in (0.1, 0.4, 0.58):
        it.lathe(V(0, 0, z), [(0.0, 0.03), (0.03, 0.03)], 6, "metal", SILVER, 0.04)
    it.lathe(V(0, 0, 0.6), [(0.0, 0.03), (0.04, 0.06), (0.06, 0.05)], 6, "metal", SILVER, 0.05)
    crystal(it, (0, 0, 0.62), 0.2, 0.05, AMETHYST, sides=5)
    for i in range(4):
        a = 2 * math.pi * i / 4
        it.rod(V(0, 0, 0.72), V(0.11 * math.cos(a), 0.05 * math.sin(a), 0.78 + 0.04 * math.sin(a * 3)), 0.006, 3, "emissive",
               lin((0.9, 0.75, 1.0)), 0.0)
    it.pose(-35)


@model("greatsword")
def greatsword(it):
    """A two-handed greatsword: a broad blade with a long ricasso, a wide straight guard, a
    heavy pommel."""
    sword(it, length=1.15, width=0.12, guard=0.26, rise=0.0, grip=0.34, color=STEEL, guard_color=STEEL_DARK,
          pommel=STEEL_DARK, fuller=STEEL_DARK, grip_color=LEATHER_DARK, thick=0.028,
          stations=[(0.0, 0.085), (0.12, 0.085), (0.15, 0.12), (0.75, 0.11), (0.92, 0.07), (1.0, 0.0)])
    for s in (-1, 1):
        it.lathe(V(s * 0.06, 0, 0.47), [(0.0, 0.02), (0.07, 0.0)], 4, "metal", STEEL_DARK, 0.04, axis=(s, 0, 0.2))
    it.orb((0, 0, -0.06), 0.06, "metal", STEEL_DARK, subdiv=1)
    it.pose(-45)


@model("elder_staff")
def elder_staff(it):
    """A gnarled staff whose roots cradle a great violet orb: deep ability power."""
    staff(it, 1.0, WOOD_DARK, r=0.034)
    it.orb((0, 0, 1.16), 0.17, "emissive", AMETHYST, subdiv=2)
    for i in range(5):
        a = 2 * math.pi * i / 5 + 0.4
        path = [V(0.03 * math.cos(a), 0.03 * math.sin(a), 0.98), V(0.15 * math.cos(a), 0.15 * math.sin(a), 1.06),
                V(0.19 * math.cos(a), 0.19 * math.sin(a), 1.2), V(0.08 * math.cos(a), 0.08 * math.sin(a), 1.33)]
        it.sweep(path, lambda i, t: [(0.026 * (1 - t * 0.7) * math.cos(2 * math.pi * j / 4), 0.026 * (1 - t * 0.7) * math.sin(2 * math.pi * j / 4)) for j in range(4)],
                 "cloth", WOOD_DARK, 0.1, normal=(math.cos(a + 1.57), math.sin(a + 1.57), 0))
    for k in range(3):
        it.orb((0.2 * math.cos(k * 2.1), -0.1, 1.05 + 0.12 * k), 0.02, "emissive", lin((0.85, 0.7, 1.0)), subdiv=0)
    it.pose(-25)


# ---- Epics ---------------------------------------------------------------------------------


@model("titan_belt")
def titan_belt(it):
    """A wide leather belt, a heavy gold buckle with a ruby."""
    it.lathe(V(0, 0, 0), [(0.0, 0.32, 0.22), (0.14, 0.32, 0.22)], 14, "cloth", LEATHER, 0.1, cap_top=False, cap_bottom=False)
    it.lathe(V(0, 0, 0), [(0.0, 0.3, 0.2), (0.14, 0.3, 0.2)], 14, "cloth", LEATHER_DARK, 0.1, cap_top=False, cap_bottom=False)
    it.extrude([(-0.12, -0.02), (0.12, -0.02), (0.14, 0.07), (0.12, 0.16), (-0.12, 0.16), (-0.14, 0.07)], 0.04, "metal", GOLD, y=-0.23,
               bevel=0.02)
    it.gem((0, -0.26, 0.07), 0.05, RUBY, n=6)
    for i in range(10):
        a = math.pi * (0.15 + 0.7 * i / 9) + math.pi
        it.orb((0.33 * math.cos(a), 0.23 * math.sin(a), 0.07), 0.014, "metal", BRONZE, subdiv=0)
    it.pose(8)


@model("leech_fang")
def leech_fang(it):
    """A curved bone fang for a blade, its edge stained red, a crimson wrap: life steal."""
    it.grip(V(0, 0, 0), V(0, 0, 0.17), 0.026, wraps=4, color=CLOTH_RED)
    guard_arc(it, 0.19, 0.08, -0.03, 0.02, BONE, slot="skin")
    it.blade(V(0, 0, 0.21), V(0, 0, 0.72), [(0.0, 0.06), (0.4, 0.065), (0.8, 0.04), (1.0, 0.0)], 0.02, slot="skin", color=BONE,
             edge=BLOOD, curve=lambda t: 0.12 * t * t)
    it.lathe(V(0, 0, -0.05), [(0.0, 0.0), (0.02, 0.03), (0.05, 0.02)], 5, "skin", BONE, 0.05)
    it.pose(-40)


@model("arc_bow")
def arc_bow(it):
    """A recurve bow, gold-tipped, crackling: attack speed and on-hit magic."""
    pts = bow(it, half=0.5, bend=0.16, color=WOOD, tips=GOLD, recurve=0.08)
    for p in pts[2:-2:3]:
        it.rod(p, p + V(0.05, -0.02, 0.06), 0.006, 3, "emissive", TOPAZ, 0.0)
    it.pose(-30)


@model("chain_coat")
def chain_coat(it):
    """A mail shirt with short sleeves, a bronze collar and belt."""
    torso(it, STEEL_DARK, "metal", w=0.29, d=0.17, h=0.6, collar=BRONZE)
    for k in range(7):
        it.lathe(V(0, 0, 0.04 + k * 0.075), [(0.0, 0.26 + 0.025 * math.sin(k * 0.9), 0.16), (0.012, 0.262 + 0.025 * math.sin(k * 0.9), 0.162)],
                 10, "metal", STEEL, 0.06)
    it.lathe(V(0, 0, 0.02), [(0.0, 0.265, 0.165), (0.05, 0.265, 0.165)], 10, "cloth", LEATHER, 0.06)
    it.bar((0, -0.17, 0.045), (0.07, 0.02, 0.06), "metal", BRONZE)


@model("spellward_mantle")
def spellward_mantle(it):
    """A teal mantle stitched with glowing silver runes: magic resist."""
    cape(it, CLOTH_TEAL, SILVER)
    for k in range(5):
        x = -0.24 + 0.12 * k
        it.bar((x, -0.17, 0.12), (0.035, 0.01, 0.06), "emissive", CYAN, 0.0, rot=20 * (k - 2))
    it.gem((0, -0.14, 0.55), 0.045, CYAN)


@model("runed_hilt")
def runed_hilt(it):
    """A hilt with no blade of steel: runes on its guard spill a blade of light."""
    it.grip(V(0, 0, 0), V(0, 0, 0.2), 0.026, wraps=4, color=CLOTH_BLUE, band=SILVER)
    guard_arc(it, 0.22, 0.13, 0.05, 0.025, SILVER)
    for x in (-0.08, 0.0, 0.08):
        it.gem((x, -0.03, 0.225), 0.022, CYAN, n=4)
    it.blade(V(0, 0, 0.25), V(0, 0, 0.75), [(0.0, 0.05), (0.5, 0.045), (0.85, 0.03), (1.0, 0.0)], 0.012, slot="emissive",
             color=lin((0.45, 0.8, 1.0)), edge=lin((0.85, 0.95, 1.0)))
    it.lathe(V(0, 0, -0.06), [(0.0, 0.0), (0.02, 0.035), (0.06, 0.02)], 6, "metal", SILVER, 0.04)
    it.pose(-45)


@model("splitting_axe")
def splitting_axe(it):
    """A one-handed axe: a broad crescent of steel on a wooden haft."""
    it.rod(V(0, 0, 0), V(0, 0, 0.8), 0.022, 6, "cloth", WOOD, 0.1)
    it.grip(V(0, 0, 0), V(0, 0, 0.2), 0.026, wraps=3, color=LEATHER)
    it.extrude([(0.0, 0.6), (0.08, 0.6), (0.22, 0.5), (0.3, 0.62), (0.31, 0.78), (0.22, 0.88), (0.08, 0.76), (0.0, 0.76)], 0.03,
               "metal", STEEL, bevel=0.0)
    it.extrude([(0.24, 0.52), (0.31, 0.62), (0.32, 0.78), (0.24, 0.87), (0.27, 0.78), (0.27, 0.62)], 0.012, "metal", lighter(STEEL, 1.4),
               y=0.0)
    it.bar((0, 0, 0.68), (0.06, 0.06, 0.18), "metal", IRON)
    it.pose(-25)


@model("whetted_kris")
def whetted_kris(it):
    """A wavy-bladed kris, its dark steel sheened violet: lethality."""
    it.grip(V(0, 0, 0), V(0, 0, 0.15), 0.022, wraps=3, color=BLACK)
    it.bar((0, 0, 0.17), (0.12, 0.04, 0.04), "metal", lin((0.4, 0.25, 0.5)), rot=10)
    st = [(i / 24, 0.05 * (1 - (i / 24) ** 2) + 0.004) for i in range(25)]
    st[-1] = (1.0, 0.0)
    it.blade(V(0, 0, 0.19), V(0, 0, 0.66), st, 0.014, color=lin((0.3, 0.28, 0.38)), edge=lin((0.75, 0.6, 1.0)),
             curve=lambda t: 0.025 * math.sin(t * 5 * math.pi))
    it.lathe(V(0, 0, -0.05), [(0.0, 0.0), (0.02, 0.03), (0.05, 0.015)], 5, "metal", lin((0.4, 0.25, 0.5)), 0.04)
    it.pose(-45)


@model("gale_cloak")
def gale_cloak(it):
    """A gold-and-white cloak streaming in the wind, a lucky coin on its clasp: speed and crit."""
    for k, (col, dz) in enumerate(((lin((0.92, 0.86, 0.68)), 0.0), (GOLD, 0.1), (lin((0.96, 0.95, 0.9)), 0.2))):
        path = wavy((-0.1, 0, 0.55 - dz), (0.62, 0.05 * k, 0.35 - dz * 1.2), 8, 0.06, 3, side=(0, 0, 1))
        ribbon(it, path, lambda t: 0.09 * (1 - 0.6 * t), col)
    it.disc((-0.12, -0.03, 0.5), 0.07, 0.02, "metal", GOLD, n=10)
    it.gem((-0.12, -0.045, 0.5), 0.03, TOPAZ, n=5)


@model("glowing_garnet")
def glowing_garnet(it):
    """A cut garnet in gold claws, a cyan glow ringing it: health and haste."""
    it.gem((0, -0.02, 0.25), 0.18, GARNET, n=8, depth=1.2)
    for i in range(6):
        a = 2 * math.pi * i / 6 + 0.26
        it.lathe(V(0.17 * math.cos(a), 0.05, 0.25 + 0.17 * math.sin(a)), [(0.0, 0.02), (0.09, 0.0)], 4, "metal", GOLD, 0.04,
                 axis=(-math.cos(a), -1.0, -math.sin(a)))
    it.torus((0, 0.03, 0.25), 0.2, 0.02, "metal", GOLD_DARK, n=14, m=4)
    it.torus((0, 0.06, 0.25), 0.27, 0.01, "emissive", CYAN, n=18, m=3)


@model("barbed_hook")
def barbed_hook(it):
    """A barbed iron hook on a chain, red along its barbs: grievous wounds."""
    it.chain(V(0, 0, 0.95), V(0, 0, 0.62), 4, 0.04, color=STEEL_DARK)
    path = [V(0, 0, 0.6), V(0, 0, 0.35), V(-0.02, 0, 0.2), V(0.08, 0, 0.08), V(0.2, 0, 0.12), V(0.24, 0, 0.26)]

    def sec(i, t):
        r = 0.065 * (1 - 0.6 * t) + 0.012
        return [(r * math.cos(2 * math.pi * j / 6), r * math.sin(2 * math.pi * j / 6)) for j in range(6)]

    it.sweep(path, sec, "metal", STEEL_DARK, 0.08)
    for p, d in ((V(-0.02, 0, 0.44), (-1, 0, -1)), (V(-0.04, 0, 0.28), (-1, 0, -0.6)), (V(0.24, 0, 0.26), (-0.4, 0, -1))):
        it.lathe(p, [(0.0, 0.04), (0.13, 0.0)], 4, "metal", BLOOD, 0.06, axis=d)


@model("blight_orb")
def blight_orb(it):
    """A sickly green orb in a cage of black iron: magic that sours healing."""
    it.orb((0, 0, 0.28), 0.17, "emissive", lin((0.4, 0.9, 0.2)), subdiv=2)
    for i in range(4):
        a = math.pi * i / 4
        it.torus((0, 0, 0.28), 0.19, 0.013, "metal", BLACK, n=14, m=4, axis=(math.cos(a), math.sin(a), 0))
    it.lathe(V(0, 0, 0), [(0.0, 0.12), (0.04, 0.09), (0.1, 0.04)], 6, "metal", BLACK, 0.06)
    for i in range(5):
        a = 2 * math.pi * i / 5
        it.orb((0.25 * math.cos(a), -0.05, 0.28 + 0.25 * math.sin(a)), 0.02, "emissive", lin((0.6, 1.0, 0.3)), subdiv=0)


@model("rage_knot")
def rage_knot(it):
    """A crimson rope knotted round an iron ring, a spike through it: fury."""
    it.torus((0, 0, 0.3), 0.18, 0.035, "metal", IRON, n=14, m=5)
    for k in range(3):
        a = 2 * math.pi * k / 3
        it.torus((0.12 * math.cos(a), 0, 0.3 + 0.12 * math.sin(a)), 0.1, 0.03, "cloth", CLOTH_RED, n=10, m=5,
                 axis=(math.cos(a + 1.2), 0.6, math.sin(a + 1.2)))
    it.lathe(V(-0.25, 0, 0.05), [(0.0, 0.03), (0.6, 0.0)], 4, "metal", STEEL, 0.05, axis=(1, -0.2, 1))


# ---- Boots ---------------------------------------------------------------------------------


@model("boots")
def boots(it):
    boot(it, color=LEATHER, cuff=LEATHER_DARK, lace=PAPER)


@model("swift_boots")
def swift_boots(it):
    """Light boots with feathered heels: movement."""
    boot(it, color=lin((0.62, 0.52, 0.36)), cuff=CLOTH_TEAL, wing=WHITE, lace=CLOTH_TEAL)


@model("battle_boots")
def battle_boots(it):
    """Red battle boots, gold plates and spikes: attack speed."""
    boot(it, color=CLOTH_RED, cuff=GOLD, spikes=GOLD, buckle=GOLD)


@model("sage_boots")
def sage_boots(it):
    """Soft blue cloth boots, a curled toe, cyan runes: haste."""
    boot(it, color=CLOTH_BLUE, cuff=SILVER, toe_curl=0.1, rune=CYAN, toe=0.32)


@model("ironshod_boots")
def ironshod_boots(it):
    """Steel sabatons over leather: armor against attacks."""
    boot(it, color=LEATHER_DARK, plate=STEEL, cuff=BRONZE, buckle=BRONZE)


@model("stalwart_sabatons")
def stalwart_sabatons(it):
    """Teal-enameled plate boots with silver trim: magic resist and tenacity."""
    boot(it, color=LEATHER_DARK, plate=lin((0.2, 0.42, 0.44)), cuff=SILVER, buckle=SILVER)


@model("spellstriders")
def spellstriders(it):
    """Violet boots with a pointed toe and burning runes: magic penetration."""
    boot(it, color=CLOTH_PURPLE, cuff=GOLD_DARK, toe=0.36, toe_curl=0.05, rune=AMETHYST, wing=lin((0.5, 0.3, 0.8)))

# ---- Legendary: fighters -------------------------------------------------------------------


def axe_head(it, z, outline, color, edge, depth=0.035):
    it.extrude(outline, depth, "metal", color, bevel=0.0)
    # The edge: a bright inset band along the outline's outer side.
    pts = outline[1:-1]
    for (x0, z0), (x1, z1) in zip(pts, pts[1:]):
        it.rod(V(x0, -depth / 2 - 0.003, z0), V(x1, -depth / 2 - 0.003, z1), 0.008, 3, "metal", edge, 0.0)


@model("threefold_brand")
def threefold_brand(it):
    """A longsword whose guard forks in three, three runes along it in gold, red and cyan:
    attack, health, haste, and the spellblade between them."""
    sword(it, length=0.95, width=0.075, guard=0.16, rise=0.08, grip=0.24, color=lin((0.72, 0.74, 0.8)), guard_color=GOLD,
          pommel=GOLD, pommel_gem=CYAN, fuller=lin((0.3, 0.3, 0.42)))
    it.lathe(V(0, 0, 0.26), [(0.0, 0.03), (0.14, 0.0)], 4, "metal", GOLD, 0.04, axis=(0, 0, 1))
    for k, col in enumerate((TOPAZ, RUBY, CYAN)):
        it.gem((0, -0.02, 0.42 + k * 0.17), 0.026, col, n=4)
    it.pose(-45)


@model("sundering_axe")
def sundering_axe(it):
    """A black double-bitted axe, red cracks glowing in its heads: armor torn open."""
    dark = lin((0.16, 0.15, 0.17))
    it.rod(V(0, 0, 0), V(0, 0, 1.0), 0.026, 6, "cloth", WOOD_DARK, 0.1)
    it.grip(V(0, 0, 0.0), V(0, 0, 0.3), 0.03, wraps=4, color=CLOTH_RED, band=dark)
    for s in (-1, 1):
        out = [(0.0, 0.72), (s * 0.12, 0.66), (s * 0.3, 0.56), (s * 0.36, 0.76), (s * 0.3, 0.98), (s * 0.12, 0.88), (0.0, 0.84)]
        if s < 0:
            out = out[::-1]
        it.extrude(out, 0.04, "metal", dark, bevel=0.0)
        for (x0, z0), (x1, z1) in (((s * 0.1, 0.78), (s * 0.2, 0.74)), ((s * 0.2, 0.74), (s * 0.28, 0.8)), ((s * 0.2, 0.74), (s * 0.26, 0.64))):
            it.rod(V(x0, -0.024, z0), V(x1, -0.024, z1), 0.009, 3, "emissive", FIRE, 0.0)
        it.rod(V(s * 0.3, -0.022, 0.57), V(s * 0.36, -0.022, 0.76), 0.008, 3, "metal", STEEL, 0.0)
        it.rod(V(s * 0.36, -0.022, 0.76), V(s * 0.3, -0.022, 0.97), 0.008, 3, "metal", STEEL, 0.0)
    it.lathe(V(0, 0, 1.0), [(0.0, 0.03), (0.12, 0.0)], 4, "metal", dark, 0.04)
    it.pose(-30)


@model("reaping_cleaver")
def reaping_cleaver(it):
    """A great cleaver with a crescent bite out of its back and a blood-red edge."""
    it.grip(V(0, 0, 0), V(0, 0, 0.3), 0.03, wraps=4, color=LEATHER_DARK, band=STEEL_DARK)
    it.bar((0.0, 0, 0.32), (0.12, 0.06, 0.05), "metal", STEEL_DARK)
    out = [(-0.07, 0.34), (0.15, 0.34), (0.2, 0.6), (0.18, 1.02), (0.06, 1.12), (-0.02, 1.05), (-0.09, 0.93), (-0.05, 0.84),
           (-0.08, 0.72), (-0.09, 0.5)]
    it.extrude(out, 0.04, "metal", STEEL, bevel=0.012)
    for (x0, z0), (x1, z1) in zip(out[1:5], out[2:5]):
        it.rod(V(x0, -0.022, z0), V(x1, -0.022, z1), 0.012, 3, "metal", BLOOD, 0.0)
    for z in (0.42, 0.5):
        it.orb((-0.03, -0.022, z), 0.016, "metal", IRON, subdiv=0)
    it.lathe(V(0, 0, -0.06), [(0.0, 0.0), (0.02, 0.04), (0.06, 0.025)], 6, "metal", STEEL_DARK, 0.04)
    it.pose(-35)


@model("lionheart_talisman")
def lionheart_talisman(it):
    """A gold lion's head with a crimson mane, a ruby heart at its brow: courage when it's
    needed."""
    c = (0, 0, 0.32)
    for i in range(12):
        a = 2 * math.pi * i / 12
        it.lathe(V(c[0] + 0.12 * math.cos(a), 0.02, c[2] + 0.12 * math.sin(a)), [(0.0, 0.07), (0.18, 0.0)], 4, "cloth",
                 CLOTH_RED if i % 2 else darker(CLOTH_RED, 0.75), 0.06, axis=(math.cos(a), 0.15, math.sin(a)))
    it.orb(c, 0.16, "metal", GOLD, subdiv=1, squash=(1.0, 0.7, 1.05))
    it.orb((0, -0.12, 0.28), 0.07, "metal", GOLD_DARK, subdiv=1, squash=(1.0, 0.8, 0.8))
    for x in (-0.06, 0.06):
        it.bar((x, -0.13, 0.36), (0.045, 0.01, 0.018), "emissive", TOPAZ, 0.0, rot=-20 if x < 0 else 20)
        it.orb((x * 1.8, -0.02, 0.45), 0.04, "metal", GOLD, subdiv=0)
    it.gem((0, -0.13, 0.43), 0.035, RUBY, n=5)
    it.torus((0, 0, 0.62), 0.05, 0.014, "metal", GOLD, n=10, m=4)


@model("bloodletters_maul")
def bloodletters_maul(it):
    """A spiked maul, its head channeled for blood."""
    it.rod(V(0, 0, 0), V(0, 0, 0.82), 0.026, 6, "cloth", WOOD_DARK, 0.1)
    it.grip(V(0, 0, 0.0), V(0, 0, 0.28), 0.03, wraps=4, color=LEATHER_DARK, band=IRON)
    it.lathe(V(-0.2, 0, 0.84), [(0.0, 0.1), (0.04, 0.12), (0.36, 0.12), (0.4, 0.1)], 8, "metal", IRON, 0.08, axis=(1, 0, 0))
    for x in (-0.12, 0.0, 0.12):
        it.lathe(V(x, 0, 0.84), [(0.0, 0.125), (0.03, 0.125)], 8, "emissive", BLOOD, 0.04, axis=(1, 0, 0))
    for i in range(6):
        a = 2 * math.pi * i / 6
        for x in (-0.09, 0.09):
            it.lathe(V(x, 0.11 * math.cos(a), 0.84 + 0.11 * math.sin(a)), [(0.0, 0.03), (0.09, 0.0)], 4, "metal", STEEL, 0.05,
                     axis=(0, math.cos(a), math.sin(a)))
    it.pose(-30)


# ---- Legendary: assassins ------------------------------------------------------------------


@model("veilrunner_dagger")
def veilrunner_dagger(it):
    """A slim black dagger trailing a veil of violet smoke: the step you don't hear."""
    dagger(it, length=0.48, width=0.042, color=lin((0.22, 0.22, 0.27)), grip_color=BLACK, guard_color=lin((0.35, 0.25, 0.45)),
           edge=lin((0.7, 0.55, 1.0)))
    path = wavy((0.0, 0.02, -0.04), (-0.05, 0.05, -0.55), 8, 0.08, 3)
    ribbon(it, path, lambda t: 0.05 * (1 - 0.7 * t) + 0.01, lin((0.42, 0.22, 0.62)), slot="emissive")
    it.pose(-45)


@model("duskfang")
def duskfang(it):
    """A curved fang of dark steel under a gold crescent moon: the strike after the strike."""
    it.grip(V(0, 0, 0), V(0, 0, 0.17), 0.024, wraps=4, color=BLACK, band=GOLD_DARK)
    it.torus((0, 0, 0.23), 0.08, 0.016, "metal", GOLD, n=12, m=4, arc=0.6, start=math.pi * 1.2)
    it.blade(V(0, 0, 0.21), V(0, 0, 0.75), [(0.0, 0.05), (0.3, 0.07), (0.7, 0.05), (1.0, 0.0)], 0.016,
             color=lin((0.26, 0.22, 0.34)), edge=lin((0.8, 0.65, 1.0)), curve=lambda t: -0.16 * t * t)
    it.gem((0, -0.02, 0.23), 0.025, AMETHYST, n=5)
    it.pose(-40)


@model("shrikeblade")
def shrikeblade(it):
    """A blade that hooks at its tip like a shrike's beak, a fan of black feathers at its
    guard."""
    it.grip(V(0, 0, 0), V(0, 0, 0.18), 0.024, wraps=4, color=LEATHER_DARK, band=STEEL_DARK)
    for k in range(5):
        a = math.radians(-70 + 35 * k)
        tip = (0.2 * math.sin(a), 0.2 + 0.14 * math.cos(a) * 0.6 - 0.06)
        base = (0.0, 0.2)
        it.extrude([base, (tip[0] - 0.035 * math.cos(a), tip[1] + 0.035 * math.sin(a)), tip,
                    (tip[0] + 0.035 * math.cos(a), tip[1] - 0.035 * math.sin(a))], 0.012, "cloth",
                   BLACK if k % 2 else lin((0.3, 0.24, 0.36)), y=0.02 + 0.006 * k)
    it.bar((0, 0, 0.2), (0.1, 0.05, 0.04), "metal", STEEL_DARK)
    it.blade(V(0, 0, 0.22), V(0, 0, 0.76), [(0.0, 0.05), (0.5, 0.06), (0.8, 0.055), (0.93, 0.04), (1.0, 0.0)], 0.016,
             color=STEEL_DARK, edge=STEEL, curve=lambda t: 0.1 * max(0.0, t - 0.7) ** 1.2 * 3.3)
    it.pose(-40)


# ---- Legendary: marksmen -------------------------------------------------------------------


@model("starsteel_blade")
def starsteel_blade(it):
    """A long blade of pale starsteel, a gold star at its guard: the crit that lands harder."""
    sword(it, length=1.0, width=0.07, guard=0.17, rise=-0.05, grip=0.22, color=lin((0.82, 0.86, 0.95)), guard_color=GOLD,
          pommel=GOLD, fuller=lin((0.45, 0.55, 0.85)))
    c = V(0, -0.03, 0.26)
    pts = []
    for i in range(10):
        a = math.pi / 2 + 2 * math.pi * i / 10
        r = 0.09 if i % 2 == 0 else 0.04
        pts.append((c.x + r * math.cos(a), c.z + r * math.sin(a)))
    it.extrude(pts, 0.03, "emissive", TOPAZ, y=-0.04)
    for z in (0.6, 0.85, 1.05):
        it.orb((0.05, -0.02, z), 0.012, "emissive", WHITE, subdiv=0)
    it.pose(-45)


@model("fanfire_quiver")
def fanfire_quiver(it):
    """A quiver whose arrows fan out tipped in flame."""
    it.lathe(V(0, 0, 0), [(0.0, 0.09, 0.07), (0.5, 0.11, 0.08), (0.55, 0.12, 0.09)], 8, "cloth", LEATHER, 0.1)
    for z in (0.08, 0.45):
        it.lathe(V(0, 0, z), [(0.0, 0.1, 0.08), (0.03, 0.105, 0.085)], 8, "metal", GOLD, 0.04)
    for k in range(5):
        a = math.radians(-36 + 18 * k)
        d = V(math.sin(a), 0, math.cos(a))
        p0 = V(0, 0, 0.5)
        p1 = p0 + d * 0.42
        it.rod(p0, p1, 0.008, 4, "cloth", WOOD, 0.05)
        it.lathe(p1, [(0.0, 0.03), (0.07, 0.0)], 4, "emissive", FIRE, 0.04, axis=tuple(d))
        for s in (-1, 1):
            it.extrude([(p0.x + d.x * 0.08, p0.z + d.z * 0.08), (p0.x + d.x * 0.18 + s * 0.03, p0.z + d.z * 0.18),
                        (p0.x + d.x * 0.2, p0.z + d.z * 0.2)], 0.004, "cloth", GOLD if k % 2 else WHITE, y=-0.002 * k)
    it.pose(15)


@model("titanbane_bow")
def titanbane_bow(it):
    """A great bow banded in iron, a heavy bodkin arrow nocked: for felling the big ones."""
    pts = bow(it, half=0.62, bend=0.22, color=WOOD_DARK, tips=IRON, recurve=0.03, r=0.03, grip_color=LEATHER_DARK)
    for p in (pts[3], pts[-4]):
        it.lathe(p - V(0, 0, 0.02), [(0.0, 0.04), (0.04, 0.04)], 6, "metal", IRON, 0.04)
    it.rod(V(-0.3, -0.03, 0.62), V(0.38, -0.03, 0.62), 0.01, 4, "cloth", WOOD, 0.05)
    it.lathe(V(0.38, -0.03, 0.62), [(0.0, 0.035), (0.12, 0.0)], 4, "metal", STEEL, 0.04, axis=(1, 0, 0))
    it.pose(-30)


@model("crimson_fang")
def crimson_fang(it):
    """A crimson blade from a fanged guard, a ruby that drinks: life steal that overflows."""
    sword(it, length=0.9, width=0.08, guard=0.15, rise=-0.09, grip=0.22, color=lin((0.62, 0.08, 0.1)),
          edge=lin((1.0, 0.4, 0.4)), guard_color=lin((0.2, 0.18, 0.2)), pommel=lin((0.2, 0.18, 0.2)), guard_gem=RUBY,
          fuller=lin((0.3, 0.02, 0.04)))
    for s in (-1, 1):
        it.lathe(V(s * 0.08, 0, 0.27), [(0.0, 0.025), (0.12, 0.0)], 4, "skin", BONE, 0.04, axis=(s * 0.4, 0, 1))
    it.pose(-45)


@model("gale_saber")
def gale_saber(it):
    """A curved saber with ribbons of wind along it: every third stroke a gust."""
    sword(it, length=0.85, width=0.06, guard=0.1, rise=0.04, grip=0.2, color=lin((0.72, 0.8, 0.86)), guard_color=GOLD,
          pommel=GOLD, curve=lambda t: 0.15 * t * t, stations=[(0.0, 0.055), (0.6, 0.065), (0.9, 0.05), (1.0, 0.0)])
    for k in range(2):
        path = wavy((0.1 + 0.04 * k, -0.03, 0.4 + 0.2 * k), (0.25 + 0.04 * k, -0.03, 0.9 + 0.1 * k), 6, 0.04, 2)
        ribbon(it, path, lambda t: 0.018 * (1 - t) + 0.004, lin((0.6, 0.95, 1.0)), slot="emissive")
    it.pose(-45)


# ---- Legendary: mages ----------------------------------------------------------------------


@model("grand_grimoire")
def grand_grimoire(it):
    """A great violet tome, gold-cornered, a great amethyst on its cover: power multiplied."""
    book(it, CLOTH_PURPLE, w=0.55, h=0.68, thick=0.2, clasp=GOLD, corners=GOLD, sigil=AMETHYST)
    for k in range(3):
        it.bar((-0.18 + k * 0.18, -0.106, 0.1), (0.1, 0.01, 0.025), "emissive", lin((0.8, 0.6, 1.0)), 0.0)


@model("amber_reliquary")
def amber_reliquary(it):
    """A gold reliquary holding a column of amber: stillness kept for when it's needed."""
    it.lathe(V(0, 0, 0), [(0.0, 0.17), (0.05, 0.17), (0.08, 0.12), (0.1, 0.12)], 8, "metal", GOLD, 0.05)
    it.lathe(V(0, 0, 0.6), [(0.0, 0.12), (0.02, 0.12), (0.05, 0.17), (0.09, 0.14), (0.16, 0.0)], 8, "metal", GOLD, 0.05)
    crystal(it, (0, 0, 0.08), 0.52, 0.11, AMBER, sides=6, tip=0.15)
    for i in range(4):
        a = 2 * math.pi * i / 4 + math.pi / 4
        it.rod(V(0.14 * math.cos(a), 0.14 * math.sin(a), 0.1), V(0.14 * math.cos(a), 0.14 * math.sin(a), 0.6), 0.016, 4, "metal",
               GOLD_DARK, 0.04)
    it.orb((0, 0, 0.78), 0.035, "emissive", TOPAZ, subdiv=0)


@model("hollow_scepter")
def hollow_scepter(it):
    """A scepter whose head is a ring around nothing, the dark in it glowing violet: through
    any ward."""
    it.rod(V(0, 0, 0), V(0, 0, 0.7), 0.024, 6, "metal", lin((0.22, 0.2, 0.26)), 0.08)
    for z in (0.0, 0.3, 0.68):
        it.lathe(V(0, 0, z), [(0.0, 0.035), (0.04, 0.035)], 6, "metal", SILVER, 0.04)
    it.torus((0, 0, 0.88), 0.17, 0.035, "metal", lin((0.3, 0.26, 0.36)), n=14, m=5)
    it.disc((0, 0, 0.88), 0.14, 0.01, "emissive", lin((0.25, 0.05, 0.4)), n=14)
    it.torus((0, -0.01, 0.88), 0.12, 0.012, "emissive", AMETHYST, n=14, m=3)
    for i in range(3):
        a = math.pi / 2 + 2 * math.pi * i / 3
        it.lathe(V(0.2 * math.cos(a), 0, 0.88 + 0.2 * math.sin(a)), [(0.0, 0.03), (0.08, 0.0)], 4, "metal", SILVER, 0.04,
                 axis=(math.cos(a), 0, math.sin(a)))
    it.pose(-30)


@model("inferno_diadem")
def inferno_diadem(it):
    """A gold diadem whose points are flames, a fire opal at its brow: ability hits that burn."""
    it.lathe(V(0, 0, 0), [(0.0, 0.26, 0.2), (0.07, 0.26, 0.2)], 14, "metal", GOLD, 0.05, cap_top=False, cap_bottom=False)
    it.lathe(V(0, 0, 0), [(0.0, 0.24, 0.18), (0.07, 0.24, 0.18)], 14, "metal", GOLD_DARK, 0.05, cap_top=False, cap_bottom=False)
    for i in range(7):
        a = math.pi * (1.1 + 0.8 * i / 6)
        h = 0.18 + 0.1 * (1 - abs(i - 3) / 3)
        x, y = 0.26 * math.cos(a), 0.2 * math.sin(a)
        it.lathe(V(x, y, 0.06), [(0.0, 0.04), (h * 0.5, 0.03), (h, 0.0)], 4, "emissive", FIRE if i % 2 else lin((1.0, 0.75, 0.2)),
                 0.06, axis=(0.15 * math.cos(a), 0.15 * math.sin(a), 1))
    it.gem((0, -0.21, 0.04), 0.05, lin((1.0, 0.35, 0.1)), n=6)
    it.pose(8)


@model("arc_tempest")
def arc_tempest(it):
    """A rod of dark bone crowned with a storm, lightning running down it: attacks that
    crackle with magic."""
    it.rod(V(0, 0, 0), V(0, 0, 0.75), 0.026, 6, "skin", lin((0.32, 0.28, 0.34)), 0.1,
           profile=[(0.0, 0.02), (0.7, 0.026), (1.0, 0.04)])
    it.orb((0, 0, 0.86), 0.12, "emissive", lin((0.62, 0.45, 1.0)), subdiv=1, squash=(1.2, 1.0, 0.8))
    for i in range(3):
        a = 2 * math.pi * i / 3
        it.orb((0.1 * math.cos(a), 0.06 * math.sin(a), 0.92), 0.07, "skin", lin((0.35, 0.33, 0.45)), subdiv=0)
    bolt = [V(0.03, -0.03, 0.8), V(0.08, -0.03, 0.65), V(0.02, -0.03, 0.55), V(0.07, -0.03, 0.4), V(0.0, -0.03, 0.3)]
    for p0, p1 in zip(bolt, bolt[1:]):
        it.rod(p0, p1, 0.01, 3, "emissive", TOPAZ, 0.0)
    it.pose(-35)


@model("windstride_belt")
def windstride_belt(it):
    """A sash belt with a silver turbine of a buckle, wind spilling violet from it: a dash and a
    volley."""
    it.lathe(V(0, 0, 0), [(0.0, 0.3, 0.2), (0.12, 0.3, 0.2)], 14, "cloth", CLOTH_PURPLE, 0.1, cap_top=False, cap_bottom=False)
    it.lathe(V(0, 0, 0), [(0.0, 0.29, 0.19), (0.12, 0.29, 0.19)], 14, "cloth", darker(CLOTH_PURPLE), 0.1, cap_top=False, cap_bottom=False)
    it.disc((0, -0.21, 0.06), 0.11, 0.04, "metal", SILVER, n=10)
    for i in range(6):
        a = 2 * math.pi * i / 6
        it.bar((0.06 * math.cos(a), -0.235, 0.06 + 0.06 * math.sin(a)), (0.05, 0.01, 0.018), "metal", STEEL, 0.0, rot=-math.degrees(a) + 30)
    it.gem((0, -0.24, 0.06), 0.03, AMETHYST, n=6)
    for k in range(3):
        path = wavy((0.1, -0.24, 0.04 + 0.03 * k), (0.55, -0.15, -0.05 + 0.08 * k), 6, 0.03, 2, side=(0, 0, 1))
        ribbon(it, path, lambda t: 0.02 * (1 - t) + 0.004, lin((0.75, 0.6, 1.0)), slot="emissive")


@model("blightcaller_tome")
def blightcaller_tome(it):
    """A black tome bound in bone, a green skull glowing on its cover: healing denied."""
    book(it, lin((0.12, 0.13, 0.12)), w=0.48, h=0.6, thick=0.16, clasp=BONE, corners=BONE)
    it.orb((0, -0.09, 0.34), 0.09, "emissive", lin((0.45, 0.95, 0.3)), subdiv=1, squash=(1.0, 0.5, 1.0))
    for x in (-0.035, 0.035):
        it.orb((x, -0.13, 0.35), 0.025, "skin", BLACK, subdiv=0)
    it.bar((0, -0.12, 0.27), (0.07, 0.01, 0.03), "skin", BLACK, 0.0)


# ---- Legendary: tanks ----------------------------------------------------------------------


def heart_outline(cx, cz, s, n=10):
    pts = []
    for i in range(n * 2):
        t = 2 * math.pi * i / (n * 2)
        x = 16 * math.sin(t) ** 3
        z = 13 * math.cos(t) - 5 * math.cos(2 * t) - 2 * math.cos(3 * t) - math.cos(4 * t)
        pts.append((cx + x * s / 16, cz + z * s / 16))
    return pts


@model("heartstone")
def heartstone(it):
    """A great heart of ruby in a crust of stone veined with gold: a mountain of health."""
    it.extrude(heart_outline(0, 0.32, 0.3), 0.22, "skin", STONE_DARK, bevel=0.05)
    it.extrude(heart_outline(0, 0.32, 0.22), 0.26, "emissive", RUBY, bevel=0.04)
    for (x0, z0), (x1, z1) in (((-0.25, 0.4), (-0.12, 0.3)), ((0.18, 0.15), (0.26, 0.38)), ((-0.05, 0.05), (0.05, 0.12))):
        it.rod(V(x0, -0.12, z0), V(x1, -0.12, z1), 0.012, 3, "metal", GOLD, 0.0)


@model("briarhide_plate")
def briarhide_plate(it):
    """A breastplate of bark and bronze grown over with thorns: strike it and bleed."""
    chest_plate(it, lin((0.36, 0.3, 0.2)), trim=BRONZE)
    for i in range(16):
        a = math.pi * (1.1 + 0.8 * (i % 8) / 7)
        z = 0.15 + 0.25 * (i // 8) + 0.05 * (i % 2)
        it.lathe(V(0.3 * math.cos(a), 0.17 * math.sin(a), z), [(0.0, 0.025), (0.1, 0.0)], 4, "cloth", lin((0.3, 0.42, 0.18)), 0.08,
                 axis=(math.cos(a), math.sin(a) - 0.3, 0.4))


@model("furnace_plate")
def furnace_plate(it):
    """A heavy breastplate with a furnace burning in its chest: the heat goes out to everyone
    close."""
    chest_plate(it, IRON, trim=BRONZE)
    it.extrude([(-0.12, 0.18), (0.12, 0.18), (0.14, 0.34), (0.0, 0.42), (-0.14, 0.34)], 0.02, "emissive", FIRE, y=-0.175)
    for k in range(4):
        it.bar((-0.09 + k * 0.06, -0.19, 0.29), (0.018, 0.02, 0.24), "metal", IRON, 0.0)
    for x in (-0.2, 0.2):
        it.lathe(V(x, 0, 0.55), [(0.0, 0.035), (0.1, 0.03)], 6, "metal", IRON, 0.05)
        it.orb((x, 0, 0.67), 0.03, "emissive", FIRE, subdiv=0)


@model("stonewall_aegis")
def stonewall_aegis(it):
    """A tower shield of dressed stone in a bronze frame: crits blunted, the ground shaken."""
    out = [(-0.25, 0.0), (0.25, 0.0), (0.28, 0.75), (0.0, 0.85), (-0.28, 0.75)]
    it.extrude(out, 0.08, "skin", STONE, bevel=0.03)
    for k in range(3):
        it.bar((0, -0.045, 0.18 + k * 0.22), (0.5, 0.01, 0.015), "skin", STONE_DARK, 0.0)
        it.bar((-0.12 + 0.24 * (k % 2), -0.045, 0.28 + k * 0.22), (0.015, 0.01, 0.2), "skin", STONE_DARK, 0.0)
    for (x0, z0), (x1, z1) in zip(out + out[:1], out[1:] + out[:1]):
        if (x0, z0) != (x1, z1):
            it.rod(V(x0, -0.03, z0), V(x1, -0.03, z1), 0.025, 4, "metal", BRONZE, 0.04)
    it.disc((0, -0.05, 0.45), 0.09, 0.03, "metal", BRONZE, n=8)
    it.gem((0, -0.07, 0.45), 0.04, lin((0.85, 0.6, 0.3)), n=6)
    it.pose(-8)


@model("wardstone_mantle")
def wardstone_mantle(it):
    """A silver-trimmed teal mantle set with warding stones: healing and shields held fast."""
    cape(it, CLOTH_TEAL, SILVER)
    for x, z in ((-0.18, 0.42), (0.18, 0.42), (0.0, 0.2)):
        it.gem((x, -0.16, z), 0.05, TEAL_GLOW, n=6)
    for s in (-1, 1):
        it.orb((s * 0.3, 0, 0.6), 0.1, "metal", SILVER, subdiv=1, squash=(1.2, 1.0, 0.7))


@model("stoneskin_ward")
def stoneskin_ward(it):
    """A kite shield carved as a watchful stone face, teal runes in its eyes: turn to stone."""
    out = [(0.0, 0.0), (0.22, 0.3), (0.26, 0.72), (0.0, 0.8), (-0.26, 0.72), (-0.22, 0.3)]
    it.extrude(out, 0.08, "skin", STONE, bevel=0.03)
    for x in (-0.09, 0.09):
        it.extrude([(x - 0.05, 0.55), (x + 0.05, 0.55), (x, 0.6)], 0.01, "emissive", TEAL_GLOW, y=-0.045)
        it.bar((x, -0.045, 0.63), (0.1, 0.02, 0.025), "skin", STONE_DARK, 0.0, rot=-15 if x < 0 else 15)
    it.bar((0, -0.05, 0.42), (0.04, 0.03, 0.14), "skin", lighter(STONE, 1.1))
    it.bar((0, -0.045, 0.27), (0.14, 0.02, 0.02), "skin", STONE_DARK, 0.0)
    for (x0, z0), (x1, z1) in zip(out, out[1:] + out[:1]):
        it.rod(V(x0, -0.03, z0), V(x1, -0.03, z1), 0.02, 4, "metal", SILVER, 0.04)
    it.pose(-8)


def cape(it, color, trim, h=0.62, w0=0.2, w1=0.34):
    """A mantle seen from the front: a curved cloth falling from a collar, wider at the hem."""
    path = [V(0, 0, h), V(0, 0.01, h * 0.66), V(0, 0.02, h * 0.33), V(0, 0.0, 0.0)]

    def sec(i, t):
        w = w0 + (w1 - w0) * t
        n = 8
        return [(w * math.cos(math.pi * k / n), -0.16 * math.sin(math.pi * k / n) * (1 - 0.3 * t)) for k in range(n + 1)] + \
               [(w * math.cos(math.pi * k / n) * 0.95, -0.16 * math.sin(math.pi * k / n) * (1 - 0.3 * t) + 0.02) for k in range(n, -1, -1)]

    it.double_sided = True
    it.sweep(path, sec, "cloth", color, 0.1, normal=(0, -1, 0))
    it.double_sided = False
    it.lathe(V(0, 0, h - 0.04), [(0.0, w0 * 0.75, 0.12), (0.06, w0 * 0.8, 0.13)], 10, "metal", trim, 0.05)
    it.lathe(V(0, 0, 0.0), [(0.0, w1 * 1.0, 0.17), (0.04, w1 * 1.0, 0.17)], 10, "cloth", trim, 0.05, cap_top=False, cap_bottom=False)


# ---- Legendary: supports -------------------------------------------------------------------


@model("kinship_locket")
def kinship_locket(it):
    """An oval gold locket swung open on its hinge, a teal light inside shared with whoever is
    near."""
    it.lathe(V(0, 0, 0.3), [(0.0, 0.17, 0.21), (0.05, 0.17, 0.21)], 14, "metal", GOLD, 0.05, axis=(0, -1, 0))
    it.lathe(V(0, -0.052, 0.3), [(0.0, 0.13, 0.17), (0.01, 0.13, 0.17)], 14, "emissive", TEAL_GLOW, 0.03, axis=(0, -1, 0))
    it.orb((0, -0.07, 0.3), 0.06, "emissive", lin((0.8, 1.0, 0.95)), subdiv=1)
    # The lid, swung open toward the viewer on the right.
    it.lathe(V(0.17, -0.05, 0.3), [(0.0, 0.17, 0.21), (0.035, 0.15, 0.19)], 14, "metal", GOLD_DARK, 0.05, axis=(1, -0.35, 0))
    it.gem((0.2, -0.04, 0.3), 0.04, RUBY, n=5, axis=(1, -0.35, 0))
    it.lathe(V(0.17, -0.03, 0.18), [(0.0, 0.02), (0.24, 0.02)], 6, "metal", GOLD, 0.04)
    it.torus((0, 0, 0.55), 0.04, 0.012, "metal", GOLD, n=8, m=4)
    it.chain(V(0, 0, 0.6), V(-0.12, 0, 0.82), 3, 0.025, color=GOLD)
    it.chain(V(0, 0, 0.6), V(0.12, 0, 0.82), 3, 0.025, color=GOLD)


@model("purifying_chalice")
def purifying_chalice(it):
    """A silver chalice brimming with light: cleansing for an ally."""
    it.lathe(V(0, 0, 0), [(0.0, 0.16), (0.03, 0.16), (0.06, 0.05), (0.26, 0.035), (0.3, 0.06), (0.36, 0.18), (0.52, 0.22),
                          (0.55, 0.21)], 10, "metal", SILVER, 0.06, cap_top=False)
    it.lathe(V(0, 0, 0.51), [(0.0, 0.2), (0.015, 0.2)], 10, "emissive", lin((0.75, 0.95, 1.0)), 0.03)
    it.lathe(V(0, 0, 0.36), [(0.0, 0.19), (0.04, 0.2)], 10, "metal", GOLD, 0.04)
    for i in range(4):
        a = 2 * math.pi * i / 4 + 0.6
        it.gem((0.2 * math.cos(a), 0.2 * math.sin(a) - 0.0, 0.44), 0.03, SAPPHIRE, n=5, axis=(math.cos(a), math.sin(a), 0))
    for i in range(5):
        it.orb((-0.1 + 0.05 * i, -0.02, 0.6 + 0.06 * (i % 2)), 0.018, "emissive", WHITE, subdiv=0)


# ---- Hatchlings ----------------------------------------------------------------------------


def chick_head(it, c, color, beak, eye=BLACK, r=0.09):
    it.orb(c, r, "skin", color, subdiv=1)
    it.lathe(V(c.x + 0.0, c.y - r * 0.8, c.z - 0.01), [(0.0, 0.03), (0.06, 0.0)], 4, "skin", beak, 0.04, axis=(0, -1, -0.2))
    for s in (-1, 1):
        it.orb((c.x + s * r * 0.45, c.y - r * 0.75, c.z + r * 0.25), 0.018, "skin", eye, subdiv=0)


@model("windling")
def windling(it):
    """A downy chick bursting from a pale blue egg, wings spread to the wind, a feather crest
    blown back."""
    shell = lin((0.72, 0.85, 0.95))
    rings = it.lathe(V(0, 0, 0), [(0.0, 0.0), (0.04, 0.13), (0.12, 0.2), (0.22, 0.21)], 9, "skin", shell, 0.08, cap_top=False)
    top = rings[-1]
    for i in range(len(top)):
        p, q = top[i], top[(i + 1) % len(top)]
        tip = (p + q) / 2 + V(0, 0, 0.07 if i % 2 else 0.03)
        it.face([p, q, tip], "skin", shell, 0.06, center=V(0, 0, 0.1))
    body = lin((0.88, 0.94, 1.0))
    it.orb((0, 0, 0.3), 0.15, "skin", body, subdiv=1, squash=(1.0, 0.95, 1.05))
    it.orb((0, -0.03, 0.5), 0.11, "skin", body, subdiv=1)
    it.lathe(V(0, -0.13, 0.49), [(0.0, 0.035), (0.08, 0.0)], 4, "skin", GOLD, 0.04, axis=(0, -1, -0.15))
    for sx in (-1, 1):
        it.orb((sx * 0.045, -0.115, 0.53), 0.022, "skin", BLACK, subdiv=0)
        it.orb((sx * 0.04, -0.13, 0.54), 0.007, "emissive", WHITE, subdiv=0)
        it.extrude([(sx * 0.12, 0.34), (sx * 0.34, 0.5), (sx * 0.3, 0.42), (sx * 0.36, 0.4), (sx * 0.28, 0.34), (sx * 0.32, 0.3),
                    (sx * 0.14, 0.26)], 0.02, "cloth", lin((0.62, 0.86, 1.0)), y=-0.02)
    for k in range(3):
        it.extrude([(0.0, 0.58 + 0.01 * k), (0.15 + 0.03 * k, 0.66 + 0.04 * k), (0.05, 0.6 + 0.01 * k)], 0.012, "cloth",
                   lin((0.45, 0.8, 1.0)), y=0.01 * k)


@model("barkling")
def barkling(it):
    """A mossy egg plated in bark, a sprout and two bright eyes peering from a crack."""
    egg(it, lin((0.4, 0.3, 0.2)), h=0.5, r=0.21)
    for i in range(7):
        a = 2 * math.pi * i / 7
        it.bar((0.2 * math.cos(a), 0.2 * math.sin(a), 0.18 + 0.05 * (i % 3)), (0.06, 0.03, 0.14), "cloth",
               WOOD_DARK if i % 2 else WOOD, 0.08, rot=10 * (i - 3))
    it.orb((0, 0, 0.48), 0.12, "cloth", lin((0.25, 0.45, 0.18)), subdiv=1, squash=(1.2, 1.0, 0.45))
    it.rod(V(0, 0, 0.5), V(0.03, 0, 0.66), 0.012, 4, "cloth", lin((0.3, 0.55, 0.2)), 0.05)
    for s in (-1, 1):
        it.extrude([(0.03, 0.64), (0.03 + s * 0.09, 0.7), (0.03 + s * 0.04, 0.72)], 0.008, "cloth", lin((0.4, 0.75, 0.25)), y=0.0)
        it.orb((s * 0.06, -0.2, 0.32), 0.022, "emissive", lin((0.9, 1.0, 0.4)), subdiv=0)


@model("flintling")
def flintling(it):
    """A black flint egg split by glowing cracks, sparks rising off it: a spark that bites."""
    egg(it, lin((0.16, 0.15, 0.16)), h=0.48, r=0.2, crack=FIRE)
    for (x0, z0), (x1, z1) in (((-0.1, 0.15), (-0.02, 0.24)), ((0.06, 0.1), (0.12, 0.2)), ((0.0, 0.32), (0.08, 0.4))):
        it.rod(V(x0, -0.19, z0), V(x1, -0.19, z1), 0.009, 3, "emissive", FIRE, 0.0)
    for s in (-1, 1):
        it.orb((s * 0.06, -0.19, 0.3), 0.02, "emissive", lin((1.0, 0.8, 0.3)), subdiv=0)
    for i in range(6):
        it.orb((-0.12 + 0.05 * i, -0.05, 0.55 + 0.08 * ((i * 7) % 3)), 0.014, "emissive", lin((1.0, 0.7, 0.2)), subdiv=0)

#@@ITEMS@@


# ---- Build ---------------------------------------------------------------------------------


def build(key):
    prop.new_scene()
    it = Item(key)
    ITEMS[key](it)
    obj = it.finish()
    return obj, it.tris


def review(key, path, size=256):
    """Render the item from the icon's angle (18 §8: three-quarter, a little above, from the
    front right) on a dark ground, vertex colors, flat studio light."""
    scene = bpy.context.scene
    objs = [o for o in scene.objects if o.type == "MESH"]
    lo = Vector((min(min((o.matrix_world @ v.co)[i] for v in o.data.vertices) for o in objs) for i in range(3)))
    hi = Vector((max(max((o.matrix_world @ v.co)[i] for v in o.data.vertices) for o in objs) for i in range(3)))
    center = (lo + hi) / 2
    reach = (hi - lo).length / 2
    # Godot's icon camera direction (sin 18°, sin 14°, cos 18°) in Blender's axes.
    d = Vector((math.sin(math.radians(18)), -math.cos(math.radians(18)), math.sin(math.radians(14)))).normalized()
    cam_data = bpy.data.cameras.new("cam")
    cam_data.lens_unit = "FOV"
    cam_data.angle = math.radians(24)
    cam = bpy.data.objects.new("cam", cam_data)
    scene.collection.objects.link(cam)
    cam.location = center + d * (reach * 1.06 / math.tan(math.radians(12)))
    cam.rotation_euler = (center - cam.location).to_track_quat("-Z", "Y").to_euler()
    scene.camera = cam
    scene.render.engine = "BLENDER_WORKBENCH"
    shading = scene.display.shading
    shading.light = "STUDIO"
    shading.color_type = "VERTEX"
    shading.show_cavity = False
    scene.display.shading.background_type = "VIEWPORT"
    scene.display.shading.background_color = (0.06, 0.07, 0.09)
    scene.render.film_transparent = True
    scene.render.resolution_x = size
    scene.render.resolution_y = size
    scene.render.filepath = path
    scene.render.image_settings.file_format = "PNG"
    bpy.ops.render.render(write_still=True)
    bpy.data.objects.remove(cam)


def sheet(paths, out, cols=10, size=256):
    """The reviews in a grid, one image."""
    import numpy as np

    rows = (len(paths) + cols - 1) // cols
    big = np.zeros((rows * size, cols * size, 4), dtype=np.float32)
    big[..., :3] = (0.05, 0.06, 0.08)
    big[..., 3] = 1.0
    for i, p in enumerate(paths):
        img = bpy.data.images.load(p)
        px = np.array(img.pixels[:], dtype=np.float32).reshape(img.size[1], img.size[0], 4)
        r, c = i // cols, i % cols
        # Blender's pixel rows run bottom to top.
        y0 = (rows - 1 - r) * size
        cell = big[y0:y0 + size, c * size:(c + 1) * size]
        a = px[:size, :size, 3:4]
        cell[..., :3] = cell[..., :3] * (1 - a) + px[:size, :size, :3] * a
        bpy.data.images.remove(img)
    img = bpy.data.images.new("sheet", cols * size, rows * size, alpha=True)
    img.pixels = big.ravel()
    img.filepath_raw = out
    img.file_format = "PNG"
    img.save()


def main():
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    opts = {argv[i][2:]: argv[i + 1] for i in range(0, len(argv) - 1, 2)}
    keys = opts["only"].split(",") if "only" in opts else list(ITEMS)
    rev = opts.get("review")
    if rev:
        os.makedirs(rev, exist_ok=True)
    shots = []
    for key in keys:
        obj, tris = build(key)
        out = os.path.join(HERE, key)
        os.makedirs(out, exist_ok=True)
        if rev:
            shots.append(os.path.join(rev, f"{key}.png"))
            review(key, shots[-1])
        bpy.ops.wm.save_as_mainfile(filepath=os.path.join(out, f"{key}.blend"), compress=True)
        export.export(os.path.join(out, "export"), key, "prop")
        print(f"ITEM {key}: {tris} triangles")
    if rev and shots:
        sheet(shots, os.path.join(rev, "sheet.png"))


main()
