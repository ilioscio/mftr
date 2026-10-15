# SPDX-License-Identifier: AGPL-3.0-or-later
"""Landmark props: the fountain's platform, a relic's pad and a fallen structure's rubble.

    blender -b --python art/props/landmarks/build.py -- --id fountain --out art/props/fountain/fountain.blend
    blender -b art/props/fountain/fountain.blend --python tools/blender/run.py -- export --id fountain --kind prop

- fountain: the team's spawn platform (6 m across the fountain's circle): flagstones in rings, a
  low rim, a basin of team-colored glowing water, four braziers and team banners on poles.
- relic_pad: a carved stone pedestal with four small standing stones; the relic floats above it.
- rubble: broken blocks and a snapped stub of wall, where a structure fell.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mathutils import Vector  # noqa: E402

from mftr_blender import prop  # noqa: E402

# Linear vertex colors: the paving's blue-grey stone (15 §5), as the structures'.
STONE = [(0.15, 0.16, 0.18), (0.13, 0.14, 0.16), (0.17, 0.18, 0.19), (0.12, 0.13, 0.15), (0.16, 0.165, 0.17)]
STONE_DARK = (0.085, 0.095, 0.11)
TRIM = (0.2, 0.205, 0.21)
IRON = (0.32, 0.2, 0.07)                 # bronze fittings
WOOD = (0.1, 0.055, 0.03)
FIRE = (1.0, 0.62, 0.22)
MOSS = (0.06, 0.1, 0.035)
SLATE = (0.04, 0.05, 0.07)
DEAD_CRYSTAL = (0.05, 0.07, 0.09)
TEAM = (0.8, 0.8, 0.8)


def flagstone_rings(p, r0, r1, rings, z):
    """Flat flagstones in concentric rings from r0 to r1 at height z (each a separate slab)."""
    for k in range(rings):
        ra = r0 + (r1 - r0) * k / rings
        rb = r0 + (r1 - r0) * (k + 1) / rings
        n = max(6, int(2 * math.pi * rb / 0.9))
        off = (math.pi / n) * (k % 2)
        for i in range(n):
            a0 = off + 2 * math.pi * i / n + 0.012
            a1 = off + 2 * math.pi * (i + 1) / n - 0.012
            pts = [Vector((math.cos(a) * r, math.sin(a) * r, z)) for a, r in ((a0, ra + 0.02), (a1, ra + 0.02), (a1, rb - 0.02), (a0, rb - 0.02))]
            p.face(pts, "cloth", p.rng.choice(STONE), var=0.1, out=(0, 0, 1), ao=0.9)


def brazier(p, c):
    x, y = c
    p.prism((x, y), [(0.0, 0.22), (0.18, 0.2), (0.18, 0.12), (1.0, 0.1), (1.0, 0.16), (1.12, 0.18)], 6, "cloth", TRIM, var=0.08)
    p.prism((x, y), [(1.12, 0.3), (1.34, 0.36), (1.36, 0.3)], 8, "metal", IRON, cap_top=False)
    p.prism((x, y), [(1.16, 0.28), (1.2, 0.28)], 8, "emissive", (0.9, 0.35, 0.1))
    for i in range(3):
        a = i * 2.1
        o = Vector((math.cos(a), math.sin(a), 0)) * 0.08
        p.prism((x + o.x, y + o.y), [(1.18, 0.12), (1.45 + 0.1 * i, 0.0)], 4, "emissive", FIRE, cap_top=False)


def fountain(p):
    p.skip_bottoms = True
    flagstone_rings(p, 1.45, 5.7, 5, 0.01)
    # The rim: low dressed blocks.
    p.blocks_ring((0, 0), 5.95, 0.0, 1, 0.14, 40, 0.25, "cloth", STONE, var=0.12, alt=MOSS, alt_every=7)
    # The basin and its team-colored glowing water.
    p.prism((0, 0), [(0.0, 1.45), (0.36, 1.4), (0.42, 1.48), (0.48, 1.44)], 16, "cloth", TRIM, var=0.08, cap_top=False)
    p.prism((0, 0), [(0.48, 1.3), (0.3, 1.25)], 16, "cloth", STONE_DARK, cap_top=False, inward=True)
    p.prism((0, 0), [(0.3, 1.26), (0.31, 1.26)], 16, "accent_glow", TEAM)
    # A spout in the middle: a stack of bowls.
    p.prism((0, 0), [(0.3, 0.18), (1.0, 0.13), (1.05, 0.5), (1.18, 0.55), (1.2, 0.45)], 8, "cloth", TRIM, var=0.06)
    p.prism((0, 0), [(1.18, 0.44), (1.19, 0.44)], 8, "accent_glow", TEAM)
    p.prism((0, 0), [(1.2, 0.08), (1.7, 0.06), (1.74, 0.24), (1.84, 0.26)], 8, "cloth", TRIM, var=0.06)
    p.prism((0, 0), [(1.83, 0.22), (1.84, 0.22)], 8, "accent_glow", TEAM)
    # Braziers on the diagonals, team banners on poles between them.
    for i in range(4):
        a = math.pi / 4 + i * math.pi / 2
        brazier(p, (math.cos(a) * 4.9, math.sin(a) * 4.9))
        b = i * math.pi / 2
        c = Vector((math.cos(b), math.sin(b), 0)) * 5.3
        p.prism((c.x, c.y), [(0.0, 0.07), (3.0, 0.05)], 6, "cloth", WOOD, var=0.08)
        p.prism((c.x, c.y), [(3.0, 0.09), (3.14, 0.0)], 6, "metal", IRON, cap_top=False)
        side = Vector((-c.y, c.x, 0)).normalized()
        pts = [c + side * 0.02 + Vector((0, 0, 2.9)), c + side * 0.62 + Vector((0, 0, 2.85)), c + side * 0.6 + Vector((0, 0, 1.75)),
               c + side * 0.32 + Vector((0, 0, 1.95)), c + side * 0.04 + Vector((0, 0, 1.75))]
        out = c.normalized()
        p.face(pts, "accent", TEAM, out=out)
        p.face(list(reversed(pts)), "accent", (0.55, 0.55, 0.55), out=-out)


def relic_pad(p):
    p.skip_bottoms = True
    p.prism((0, 0), [(0.0, 0.5), (0.1, 0.48), (0.1, 0.42), (0.22, 0.4)], 8, "cloth", TRIM, var=0.1, rot=math.pi / 8)
    p.prism((0, 0), [(0.22, 0.26), (0.23, 0.26)], 8, "emissive", (0.4, 0.95, 0.5))
    for i in range(4):
        a = math.pi / 4 + i * math.pi / 2
        c = (math.cos(a) * 0.62, math.sin(a) * 0.62, 0)
        p.rock(c, (0.18, 0.16, 0.5), "cloth", p.rng.choice(STONE), var=0.1, rough=0.12, subdiv=1)


def rubble(p):
    """What a structure leaves on its plinth (15 §5): a heap of broken dressed blocks over a
    ragged stub, roof slates and dead, dark crystal shards scattered round it."""
    p.skip_bottoms = True
    p.clamp_ground = True
    # The heap: blocks tumbled in rough layers, highest in the middle.
    for layer, (count, r_max, z) in enumerate(((22, 1.7, 0.0), (14, 1.15, 0.22), (8, 0.7, 0.45), (4, 0.35, 0.66))):
        for i in range(count):
            a = p.rng.uniform(0, 2 * math.pi)
            r = r_max * math.sqrt(p.rng.uniform(0.05, 1.0))
            c = Vector((math.cos(a), math.sin(a), 0)) * r
            size = (p.rng.uniform(0.24, 0.46), p.rng.uniform(0.18, 0.34), p.rng.uniform(0.16, 0.3))
            col = p.rng.choice(STONE) if p.rng.random() > 0.12 else MOSS
            p.box((c.x, c.y, z * (1.0 - r / (r_max + 0.4))), size, "cloth", col, var=0.14, rot_z=p.rng.uniform(0, math.pi),
                  tilt=(p.rng.uniform(-0.45, 0.45), p.rng.uniform(-0.45, 0.45)))
    # A broken bit of wall still standing at one side.
    for k in range(3):
        a0 = 2.4 + k * 0.32
        c = Vector((math.cos(a0), math.sin(a0), 0)) * 0.7
        p.box((c.x, c.y, 0.0), (0.34, 0.2, 0.9 - 0.25 * k), "cloth", p.rng.choice(STONE), var=0.1, rot_z=a0 + math.pi / 2)
    # Roof slates and dead crystal shards scattered round.
    for i in range(10):
        a = p.rng.uniform(0, 2 * math.pi)
        c = Vector((math.cos(a), math.sin(a), 0)) * p.rng.uniform(1.0, 2.1)
        p.box((c.x, c.y, 0.0), (0.32, 0.24, 0.03), "cloth", SLATE, var=0.15, rot_z=p.rng.uniform(0, math.pi),
              tilt=(p.rng.uniform(-0.2, 0.2), p.rng.uniform(-0.2, 0.2)))
    for i in range(5):
        a = p.rng.uniform(0, 2 * math.pi)
        c = Vector((math.cos(a), math.sin(a), 0)) * p.rng.uniform(0.6, 1.6)
        p.prism((c.x, c.y), [(0.0, 0.07), (0.32 + 0.1 * (i % 2), 0.0)], 5, "cloth", DEAD_CRYSTAL, cap_top=False, rot=a)


BUILDS = {"fountain": fountain, "relic_pad": relic_pad, "rubble": rubble}


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    pid = argv[argv.index("--id") + 1]
    out = argv[argv.index("--out") + 1]

    def build():
        p = prop.Prop(pid, ao_height=2.0, seed=sum(map(ord, pid)))
        BUILDS[pid](p)
        p.finish()

    prop.build_and_save(build, out)
