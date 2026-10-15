# SPDX-License-Identifier: AGPL-3.0-or-later
"""The lane turret (a map prop, 15 §5): a round stone watchtower, ~6 m to its crystal.

    blender -b --python art/props/turret/build.py -- --out art/props/turret/turret.blend
    blender -b art/props/turret/turret.blend --python tools/blender/run.py -- export --id turret --kind prop

Built in the paving's blue-grey stone so it grows out of the ground (R06 §2): a round stepped
plinth of slabs (the ground's paving rings it), braziers either side of the lane, an octagonal
buttressed stump, courses of dressed stone with iron bands and arrow slits, a corbel under a
crenellated parapet, a slate roof, and the turret's eye: a team-colored crystal floating above
iron claws. Team color only in accents: the crystal, two short banners, runes on the stump.

Collapse (15 §5): everything above the stump is on part bones the client brings down in stages
(`part_crystal`, `part_roof`, `part_top`, `part_upper`, `part_mid`), each pivoting at its foot;
the plinth and the stump (`root`) stay. Front: -Y (the lane side).
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mathutils import Vector  # noqa: E402

from mftr_blender import prop  # noqa: E402

# Linear vertex colors: the paving's stone (sRGB ≈ 0.42–0.47 blue-grey).
STONE = [(0.15, 0.16, 0.18), (0.13, 0.14, 0.16), (0.17, 0.18, 0.19), (0.12, 0.13, 0.15), (0.16, 0.165, 0.17)]
STONE_DARK = (0.085, 0.095, 0.11)
TRIM = (0.2, 0.205, 0.21)
MOSS = (0.06, 0.1, 0.035)
SLATE = (0.04, 0.05, 0.07)
IRON = (0.05, 0.055, 0.065)
BRONZE = (0.32, 0.2, 0.07)
WOOD = (0.1, 0.055, 0.03)
SLIT = (0.012, 0.012, 0.015)
EMBER = (1.0, 0.45, 0.12)
TEAM = (0.8, 0.8, 0.8)                   # the accent slots take the team color


def plinth(p):
    # Two round steps of curved slabs, the lower one wide: the ground's paving rings it.
    p.blocks_ring((0, 0), 1.55, 0.0, 1, 0.13, 18, 0.4, "cloth", STONE, var=0.12, gap=0.03, alt=MOSS, alt_every=6)
    p.prism((0, 0), [(0.0, 1.16), (0.12, 1.16)], 18, "cloth", STONE_DARK)
    p.blocks_ring((0, 0), 1.18, 0.1, 1, 0.16, 14, 0.36, "cloth", STONE, var=0.1, gap=0.03)
    p.prism((0, 0), [(0.1, 0.84), (0.25, 0.84)], 14, "cloth", STONE_DARK)
    # Braziers either side of the lane: stone posts with bronze bowls of embers.
    for x in (-1.25, 1.25):
        p.prism((x, -0.55), [(0.1, 0.13), (0.62, 0.1), (0.7, 0.14)], 6, "cloth", TRIM, var=0.08)
        p.prism((x, -0.55), [(0.7, 0.12), (0.82, 0.2), (0.86, 0.2)], 8, "metal", BRONZE, var=0.06)
        p.prism((x, -0.55), [(0.8, 0.15), (0.84, 0.15)], 8, "emissive", EMBER)


def stump(p):
    # An octagonal stump with four buttresses and a bronze-studded door on the lane side.
    p.prism((0, 0), [(0.2, 0.86), (0.85, 0.8)], 8, "cloth", STONE_DARK, var=0.06, rot=math.pi / 8)
    p.blocks_ring((0, 0), 0.88, 0.2, 2, 0.32, 10, 0.12, "cloth", STONE, var=0.12, alt=MOSS, alt_every=5)
    for a in (math.pi / 4, 3 * math.pi / 4, 5 * math.pi / 4, 7 * math.pi / 4):
        o = Vector((math.cos(a), math.sin(a), 0))
        p.box((o.x * 0.92, o.y * 0.92, 0.2), (0.3, 0.34, 0.95), "cloth", TRIM, var=0.1, rot_z=a, taper=0.55)
    p.box((0, -0.86, 0.25), (0.44, 0.1, 0.62), "cloth", WOOD, var=0.06)
    p.box((0, -0.9, 0.86), (0.56, 0.12, 0.1), "cloth", TRIM)
    for z in (0.42, 0.68):
        p.box((0, -0.915, z), (0.46, 0.03, 0.04), "metal", BRONZE)
    # Team runes: two short glowing chevrons on the stump's front.
    for x in (-0.42, 0.42):
        p.box((x, -0.84, 0.62), (0.07, 0.03, 0.22), "accent_glow", TEAM, rot_z=0.0)


def shaft(p):
    p.part("mid", (0, 0, 0.85))
    p.prism((0, 0), [(0.85, 0.6), (2.31, 0.58)], 12, "cloth", STONE_DARK, cap_top=False)
    p.blocks_ring((0, 0), 0.7, 0.85, 4, 0.365, 13, 0.13, "cloth", STONE, var=0.13, taper=0.006, alt=MOSS, alt_every=11)
    p.prism((0, 0), [(1.58, 0.715), (1.66, 0.715)], 18, "metal", IRON, var=0.05, cap_top=False)
    for a in (-math.pi / 2, math.pi / 2, 0.0, math.pi):
        c = Vector((math.cos(a), math.sin(a), 0)) * 0.69
        p.box((c.x, c.y, 1.86), (0.08, 0.1, 0.36), "cloth", SLIT, rot_z=a)
    p.part("upper", (0, 0, 2.31))
    p.prism((0, 0), [(2.31, 0.57), (3.4, 0.55)], 12, "cloth", STONE_DARK, cap_top=False)
    p.blocks_ring((0, 0), 0.68, 2.31, 3, 0.36, 13, 0.13, "cloth", STONE, var=0.13, taper=0.006, alt=MOSS, alt_every=13)
    p.prism((0, 0), [(2.98, 0.69), (3.06, 0.69)], 18, "metal", IRON, var=0.05, cap_top=False)
    for a in (-math.pi / 2 + 0.6, math.pi / 2 - 0.6):
        c = Vector((math.cos(a), math.sin(a), 0)) * 0.67
        p.box((c.x, c.y, 2.65), (0.08, 0.1, 0.3), "cloth", SLIT, rot_z=a)
    # Corbel: the wall flares out under the parapet on carved brackets.
    p.prism((0, 0), [(3.38, 0.68), (3.52, 0.82), (3.68, 0.98), (3.76, 1.0)], 14, "cloth", TRIM, var=0.1, cap_top=False)
    # Under the parapet, dark charred timber and a ring of broken masonry stubs: hidden inside the
    # parapet's wall while it stands, the torn top of the shaft once it has fallen.
    p.prism((0, 0), [(3.62, 0.96), (3.7, 0.96)], 14, "cloth", (0.025, 0.017, 0.012), var=0.15)
    for i in range(13):
        a = 2 * math.pi * (i + p.rng.uniform(-0.2, 0.2)) / 13
        c = Vector((math.cos(a), math.sin(a), 0)) * 0.94
        p.box((c.x, c.y, 3.68), (0.2, 0.08, p.rng.uniform(0.06, 0.26)), "cloth", p.rng.choice(STONE), var=0.14, rot_z=a + math.pi / 2)
    for i in range(14):
        a = 2 * math.pi * (i + 0.5) / 14
        c = Vector((math.cos(a), math.sin(a), 0)) * 0.8
        p.box((c.x, c.y, 3.24), (0.12, 0.16, 0.42), "cloth", STONE_DARK, var=0.08, rot_z=a, taper=1.4)


def top(p):
    p.part("top", (0, 0, 3.76))
    p.prism((0, 0), [(3.76, 1.0), (4.02, 1.0)], 18, "cloth", TRIM, var=0.08, cap_top=False)
    p.prism((0, 0), [(3.76, 0.9), (4.02, 0.9)], 18, "cloth", STONE_DARK, cap_top=False, inward=True)
    ring = p.ring((0, 0), 1.0, 1.0, 18, 4.02)
    inner = p.ring((0, 0), 0.9, 0.9, 18, 4.02)
    for i in range(18):
        j = (i + 1) % 18
        p.face((ring[i], ring[j], inner[j], inner[i]), "cloth", TRIM, var=0.1, out=(0, 0, 1))
    for i in range(9):
        a = 2 * math.pi * i / 9
        c = Vector((math.cos(a), math.sin(a), 0)) * 0.95
        p.box((c.x, c.y, 4.02), (0.32, 0.16, 0.34), "cloth", p.rng.choice(STONE), var=0.12, rot_z=a + math.pi / 2)
    # The parapet's floor: dark timber, seen only when the roof has come off (a hollow, not a lid).
    p.prism((0, 0), [(3.74, 0.9), (3.8, 0.9)], 12, "cloth", (0.045, 0.028, 0.018), var=0.1)
    # Two short team banners on the lane side, swallow-tailed, on bronze poles.
    for a in (-math.pi / 2 - 0.55, -math.pi / 2 + 0.55):
        out = Vector((math.cos(a), math.sin(a), 0))
        side = Vector((-out.y, out.x, 0))
        base = out * 1.04
        hi, lo = 3.7, 2.75
        w = 0.17
        pts = [base + side * w + Vector((0, 0, hi)), base - side * w + Vector((0, 0, hi)),
               base - side * w + Vector((0, 0, lo)), base + Vector((0, 0, lo + 0.18)), base + side * w + Vector((0, 0, lo))]
        p.face(pts, "accent", TEAM, out=out)
        p.face(list(reversed([q - out * 0.02 for q in pts])), "accent", (0.6, 0.6, 0.6), out=-out)
        p.box((base.x, base.y, hi), (0.42, 0.04, 0.04), "metal", BRONZE, rot_z=a + math.pi / 2)


def roof(p):
    p.part("roof", (0, 0, 3.86))
    p.prism((0, 0), [(3.8, 0.88), (3.92, 0.88)], 16, "cloth", SLATE, cap_top=False, cap_bottom=True)
    p.shingled_cone((0, 0), 3.9, 0.86, 1.3, 16, 6, "cloth", SLATE, var=0.22)
    # The eye's claws: iron, rising from the roof's tip.
    p.prism((0, 0), [(5.1, 0.07), (5.36, 0.05)], 6, "metal", IRON)
    for i in range(4):
        a = 2 * math.pi * i / 4 + math.pi / 4
        o = Vector((math.cos(a), math.sin(a), 0))
        p.loft(o * 0.08 + Vector((0, 0, 5.3)), o * 0.3 + Vector((0, 0, 5.6)), 4, [(0, 0.035, 0.035), (1, 0.025, 0.025)], "metal", BRONZE)
        p.loft(o * 0.3 + Vector((0, 0, 5.6)), o * 0.15 + Vector((0, 0, 5.98)), 4, [(0, 0.025, 0.025), (1, 0.006, 0.006)], "metal", BRONZE)


def crystal(p):
    p.part("crystal", (0, 0, 5.45))
    # A tall team-colored crystal, faceted, glowing; a few small shards orbit it.
    p.prism((0, 0), [(5.45, 0.0), (5.62, 0.16), (6.0, 0.24), (6.55, 0.0)], 6, "accent_glow", TEAM, cap_top=False, rot=0.3)
    for i in range(3):
        a = 2 * math.pi * i / 3
        c = (math.cos(a) * 0.42, math.sin(a) * 0.42)
        p.prism(c, [(5.85, 0.0), (5.92, 0.05), (6.05, 0.0)], 4, "accent_glow", TEAM, cap_top=False, rot=a)


def build():
    p = prop.Prop("turret", ao_height=4.5, seed=7)
    plinth(p)
    stump(p)
    shaft(p)
    top(p)
    roof(p)
    crystal(p)
    p.finish()


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.join(HERE, "turret.blend")
    prop.build_and_save(build, out)
