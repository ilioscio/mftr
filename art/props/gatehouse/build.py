# SPDX-License-Identifier: AGPL-3.0-or-later
"""The Gatehouse (a map prop): a fortified gate across the lane, ~5 m tall.

    blender -b --python art/props/gatehouse/build.py -- --out art/props/gatehouse/gatehouse.blend
    blender -b art/props/gatehouse/gatehouse.blend --python tools/blender/run.py -- export --id gatehouse --kind prop

Two square towers of dressed stone with team-colored pyramid roofs, joined by an arched wall with
an iron portcullis, a walkway with merlons on top, and the gate's heart over the arch: a
team-colored crystal in an iron ring. Front: -Y (the lane side); it stands across the lane (X).
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mathutils import Vector  # noqa: E402

from mftr_blender import prop  # noqa: E402

STONE = [(0.5, 0.48, 0.44), (0.44, 0.43, 0.41), (0.53, 0.5, 0.44), (0.42, 0.41, 0.4), (0.48, 0.45, 0.4)]
STONE_DARK = (0.36, 0.35, 0.34)
TRIM = (0.56, 0.53, 0.47)
MOSS = (0.36, 0.43, 0.26)
IRON = (0.28, 0.29, 0.32)
DARK = (0.08, 0.07, 0.07)
TEAM = (0.8, 0.8, 0.8)


def block_wall(p, x0, x1, y0, y1, z0, courses, ch, n_per, var=0.12, alt_every=10):
    """A straight wall of stone blocks (front and back faces), x0..x1 by y0..y1."""
    w = (x1 - x0) / n_per
    for k in range(courses):
        z = z0 + k * ch
        off = (w / 2) * (k % 2)
        x = x0 - off
        while x < x1 - 1e-4:
            a, b = max(x, x0), min(x + w, x1)
            col = p.rng.choice(STONE) if p.rng.randrange(alt_every) else MOSS
            p.box(((a + b) / 2, (y0 + y1) / 2, z), (b - a - 0.03, y1 - y0, ch - 0.03), "cloth", col, var=var)
            x += w


def tower(p, cx, size, height):
    s = size / 2
    p.box((cx, 0, 0), (size + 0.24, size + 0.24, 0.3), "cloth", STONE_DARK, var=0.1)   # footing
    p.box((cx, 0, 0.3), (size - 0.08, size - 0.08, height - 0.3), "cloth", STONE_DARK)  # core
    # Courses of blocks on the four faces.
    courses, ch = 8, (height - 0.3) / 8
    for k in range(courses):
        z = 0.3 + k * ch
        for face in range(4):
            a = face * math.pi / 2
            nrm = Vector((math.cos(a), math.sin(a), 0))
            side = Vector((-nrm.y, nrm.x, 0))
            n = 3
            off = (k % 2) * 0.5
            for i in range(n + (1 if off else 0)):
                t0 = max(-s, -s + (i - off) * size / n)
                t1 = min(s, -s + (i + 1 - off) * size / n)
                if t1 - t0 < 0.05:
                    continue
                mid = (t0 + t1) / 2
                c = Vector((cx, 0, 0)) + nrm * (s - 0.02) + side * mid
                col = p.rng.choice(STONE) if p.rng.randrange(9) else MOSS
                dims = (0.1, t1 - t0 - 0.03, ch - 0.03) if face % 2 == 0 else (t1 - t0 - 0.03, 0.1, ch - 0.03)
                p.box((c.x, c.y, z), dims, "cloth", col, var=0.12)
    # Corbel, parapet and merlons.
    top = height
    p.box((cx, 0, top), (size + 0.3, size + 0.3, 0.16), "cloth", TRIM, var=0.08, taper=1.0)
    for dx, dy in ((-1, -1), (1, -1), (1, 1), (-1, 1), (0, -1), (1, 0), (0, 1), (-1, 0)):
        c = (cx + dx * (s + 0.06), dy * (s + 0.06))
        p.box((c[0], c[1], top + 0.16), (0.24, 0.24, 0.32), "cloth", TRIM, var=0.12)
    # A team-colored pyramid roof.
    base = [Vector((cx + x, y, top + 0.18)) for x, y in ((-s, -s), (s, -s), (s, s), (-s, s))]
    tip = Vector((cx, 0, top + 1.3))
    mid = Vector((cx, 0, top + 0.5))
    for i in range(4):
        a, b = base[i], base[(i + 1) % 4]
        # Each roof side in three shingle bands.
        for t0, t1 in ((0.0, 0.34), (0.34, 0.68), (0.68, 1.0)):
            a0, b0 = a.lerp(tip, t0), b.lerp(tip, t0)
            a1, b1 = a.lerp(tip, t1), b.lerp(tip, t1)
            if t1 >= 1.0:
                p.face((a0, b0, tip), "accent", TEAM, var=0.2, center=mid)
            else:
                p.face((a0, b0, b1, a1), "accent", TEAM, var=0.2, center=mid)
    p.prism((cx, 0), [(top + 1.25, 0.03), (top + 1.6, 0.02)], 4, "metal", IRON)
    # Arrow slits on the lane face.
    for z in (1.6, 2.6):
        p.box((cx, -s - 0.04, z), (0.1, 0.06, 0.45), "cloth", DARK)


def build():
    p = prop.Prop("gatehouse", ao_height=4.0, seed=11)
    p.skip_bottoms = True
    tw, h = 0.9, 3.7
    for cx in (-1.05, 1.05):
        tower(p, cx, tw, h)
    # The arch wall between the towers: piers, an arch of voussoirs, a portcullis.
    span0, span1 = -0.6, 0.6
    block_wall(p, span0, span1, -0.22, 0.22, 2.0, 4, 0.33, 3)
    for side in (-1, 1):
        p.box((side * 0.66, 0, 0), (0.18, 0.5, 2.0), "cloth", TRIM, var=0.08)
    ring = 9
    for i in range(ring):
        a0 = math.pi * i / ring
        a1 = math.pi * (i + 1) / ring
        r0, r1 = 0.6, 0.78
        pts0 = [Vector((math.cos(a) * r, -0.26, 1.45 + math.sin(a) * r)) for a, r in ((a0, r0), (a1, r0), (a1, r1), (a0, r1))]
        pts1 = [v + Vector((0, 0.52, 0)) for v in pts0]
        col = p.rng.choice(STONE)
        p.face(pts0, "cloth", TRIM, var=0.1, out=(0, -1, 0))
        p.face(pts1, "cloth", TRIM, var=0.1, out=(0, 1, 0))
        p.face((pts0[2], pts0[3], pts1[3], pts1[2]), "cloth", col, var=0.1, out=(math.cos((a0 + a1) / 2), 0, math.sin((a0 + a1) / 2)))
        p.face((pts0[0], pts0[1], pts1[1], pts1[0]), "cloth", STONE_DARK, var=0.1, out=(-math.cos((a0 + a1) / 2), 0, -math.sin((a0 + a1) / 2)))
    # The dark passage and the portcullis.
    p.box((0, 0.05, 0), (1.1, 0.04, 2.0), "cloth", DARK)
    for i in range(6):
        x = -0.5 + i * 0.2
        p.box((x, -0.08, 0.25), (0.04, 0.04, 1.75), "metal", IRON)
    for z in (0.5, 1.0, 1.5):
        p.box((0, -0.1, z), (1.1, 0.04, 0.04), "metal", IRON)
    for i in range(6):
        p.prism((-0.5 + i * 0.2, -0.08), [(0.25, 0.025), (0.0, 0.0)], 4, "metal", IRON, cap_top=False)
    # The walkway on top with merlons, and the gate's heart: a team crystal in an iron ring.
    top = 2.0 + 4 * 0.33
    p.box((0, 0, top), (1.4, 0.6, 0.12), "cloth", TRIM, var=0.08)
    for x in (-0.45, 0.0, 0.45):
        for y in (-0.24, 0.24):
            p.box((x, y, top + 0.12), (0.22, 0.12, 0.3), "cloth", TRIM, var=0.12)
    p.prism((0, -0.34), [(1.9, 0.0), (2.18, 0.2), (2.56, 0.0)], 6, "accent_glow", TEAM, cap_top=False)
    for i in range(10):
        a0, a1 = 2 * math.pi * i / 10, 2 * math.pi * (i + 1) / 10
        r = 0.32
        c0 = Vector((math.cos(a0) * r, -0.36, 2.23 + math.sin(a0) * r))
        c1 = Vector((math.cos(a1) * r, -0.36, 2.23 + math.sin(a1) * r))
        p.loft(c0, c1, 4, [(0, 0.03, 0.03), (1, 0.03, 0.03)], "metal", IRON)
    # Team banners down the towers' lane faces.
    for cx in (-1.05, 1.05):
        base = Vector((cx, -0.6, 0))
        w, top_z, bot = 0.2, 3.5, 2.1
        pts = [base + Vector((w, 0, top_z)), base + Vector((-w, 0, top_z)), base + Vector((-w, 0, bot)),
               base + Vector((0, 0, bot + 0.2)), base + Vector((w, 0, bot))]
        p.face(pts, "accent", TEAM, out=(0, -1, 0))
        p.box((cx, -0.6, top_z), (0.5, 0.05, 0.05), "metal", IRON)
    p.finish()


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.join(HERE, "gatehouse.blend")
    prop.build_and_save(build, out)
