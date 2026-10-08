# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Base (a map prop; the match is won by destroying it): a crystal shrine, ~5 m tall.

    blender -b --python art/props/base/build.py -- --out art/props/base/base.blend
    blender -b art/props/base/base.blend --python tools/blender/run.py -- export --id base --kind prop

A stepped octagonal platform of dressed stone, a ring of pillars carrying a stone lintel ring,
team banners between them, and at the heart a great team-colored crystal over a stone basin,
banded in iron, with three smaller crystals leaning out around it.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mathutils import Matrix, Vector  # noqa: E402

from mftr_blender import prop  # noqa: E402

STONE = [(0.5, 0.48, 0.44), (0.44, 0.43, 0.41), (0.53, 0.5, 0.44), (0.42, 0.41, 0.4), (0.48, 0.45, 0.4)]
STONE_DARK = (0.36, 0.35, 0.34)
TRIM = (0.58, 0.55, 0.49)
MOSS = (0.36, 0.43, 0.26)
IRON = (0.28, 0.29, 0.32)
TEAM = (0.8, 0.8, 0.8)


def crystal(p, base, axis, length, radius, sides=6):
    """A bipyramid crystal from `base` along `axis`."""
    axis = Vector(axis).normalized()
    ref = Vector((1, 0, 0)) if abs(axis.x) < 0.9 else Vector((0, 1, 0))
    u = (ref - axis * ref.dot(axis)).normalized()
    v = axis.cross(u)
    base = Vector(base)
    mid = base + axis * (length * 0.32)
    ring = [mid + (u * math.cos(2 * math.pi * i / sides) + v * math.sin(2 * math.pi * i / sides)) * radius for i in range(sides)]
    tip, foot = base + axis * length, base
    center = base + axis * (length * 0.4)
    for i in range(sides):
        j = (i + 1) % sides
        p.face((ring[i], ring[j], tip), "accent_glow", TEAM, center=center)
        p.face((ring[j], ring[i], foot), "accent_glow", TEAM, center=center)


def build():
    p = prop.Prop("base", ao_height=4.0, seed=23)
    p.skip_bottoms = True
    # The stepped platform: three octagonal tiers, their risers in stone blocks.
    for k, (r, z) in enumerate(((2.25, 0.0), (1.95, 0.22), (1.62, 0.44))):
        p.prism((0, 0), [(z, r), (z + 0.22, r)], 8, "cloth", STONE_DARK, var=0.06, rot=math.pi / 8)
        p.blocks_ring((0, 0), r + 0.02, z, 1, 0.22, 16, 0.08, "cloth", STONE, var=0.12, alt=MOSS, alt_every=8)
    top = 0.66
    p.prism((0, 0), [(top - 0.01, 1.6), (top, 1.6)], 16, "cloth", TRIM, var=0.08)
    # Six pillars and a lintel ring.
    n = 6
    for i in range(n):
        a = 2 * math.pi * (i + 0.5) / n
        c = Vector((math.cos(a), math.sin(a), 0)) * 1.32
        p.box((c.x, c.y, top), (0.4, 0.4, 0.14), "cloth", TRIM, var=0.08, rot_z=a)
        p.prism((c.x, c.y), [(top + 0.14, 0.16), (top + 2.3, 0.14)], 8, "cloth", p.rng.choice(STONE), var=0.08)
        p.box((c.x, c.y, top + 2.3), (0.38, 0.38, 0.16), "cloth", TRIM, var=0.08, rot_z=a)
    lintel_z = top + 2.46
    outer = p.ring((0, 0), 1.55, 1.55, 18, lintel_z)
    inner = p.ring((0, 0), 1.1, 1.1, 18, lintel_z)
    outer_t = [v + Vector((0, 0, 0.26)) for v in outer]
    inner_t = [v + Vector((0, 0, 0.26)) for v in inner]
    for i in range(18):
        j = (i + 1) % 18
        col = p.rng.choice(STONE)
        p.face((outer[i], outer[j], outer_t[j], outer_t[i]), "cloth", col, var=0.08, center=(0, 0, lintel_z))
        p.face((inner_t[i], inner_t[j], inner[j], inner[i]), "cloth", STONE_DARK, var=0.08, out=(-inner[i].x, -inner[i].y, 0))
        p.face((outer_t[i], outer_t[j], inner_t[j], inner_t[i]), "cloth", TRIM, var=0.1, out=(0, 0, 1))
        p.face((inner[i], inner[j], outer[j], outer[i]), "cloth", STONE_DARK, out=(0, 0, -1))
    # Merlons on the lintel ring.
    for i in range(9):
        a = 2 * math.pi * i / 9
        c = Vector((math.cos(a), math.sin(a), 0)) * 1.4
        p.box((c.x, c.y, lintel_z + 0.26), (0.22, 0.3, 0.22), "cloth", TRIM, var=0.12, rot_z=a)
    # Team banners hanging from the lintel between the pillars.
    for i in range(n):
        a = 2 * math.pi * i / n
        out = Vector((math.cos(a), math.sin(a), 0))
        side = Vector((-out.y, out.x, 0))
        base = out * 1.57
        w, t, b = 0.26, lintel_z - 0.02, lintel_z - 1.25
        pts = [base + side * w + Vector((0, 0, t)), base - side * w + Vector((0, 0, t)), base - side * w + Vector((0, 0, b)),
               base + Vector((0, 0, b + 0.24)), base + side * w + Vector((0, 0, b))]
        p.face(pts, "accent", TEAM, out=out)
        p.face(list(reversed([q - out * 0.02 for q in pts])), "accent", (0.55, 0.55, 0.55), out=-out)
    # The basin and the great crystal, banded in iron, with three smaller ones leaning out.
    p.prism((0, 0), [(top, 0.75), (top + 0.2, 0.85), (top + 0.45, 0.78), (top + 0.55, 0.88)], 8, "cloth", TRIM, var=0.08)
    p.prism((0, 0), [(top + 0.5, 0.7), (top + 0.56, 0.7)], 8, "cloth", STONE_DARK)
    crystal(p, (0, 0, top + 0.35), (0, 0, 1), 3.4, 0.5)
    for z in (top + 1.2, top + 1.9):
        p.prism((0, 0), [(z, 0.5), (z + 0.1, 0.5)], 8, "metal", IRON, cap_top=False)
    for i in range(3):
        a = 2 * math.pi * i / 3 + 0.4
        o = Vector((math.cos(a), math.sin(a), 0))
        crystal(p, o * 0.55 + Vector((0, 0, top + 0.4)), o * 0.45 + Vector((0, 0, 1)), 1.3, 0.2)
    p.finish()


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.join(HERE, "base.blend")
    prop.build_and_save(build, out)
