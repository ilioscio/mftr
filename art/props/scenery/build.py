# SPDX-License-Identifier: AGPL-3.0-or-later
"""Scenery props: pines, rocks, grass and bushes, drawn many times over (the client batches
them and varies each instance's turn, size and tint).

    blender -b --python art/props/scenery/build.py -- --id pine_1 --out art/props/pine_1/pine_1.blend
    blender -b art/props/pine_1/pine_1.blend --python tools/blender/run.py -- export --id pine_1 --kind prop

Ids: pine_1, pine_2, pine_3 (tall, slim, broad), rock_1, rock_2, rock_3 (a boulder group, flat
slabs, standing stones), grass_1 (a tuft), bush_1.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mathutils import Vector  # noqa: E402

from mftr_blender import prop  # noqa: E402

BARK = (0.33, 0.22, 0.14)
NEEDLES = [(0.16, 0.34, 0.16), (0.2, 0.4, 0.18), (0.14, 0.3, 0.15), (0.22, 0.42, 0.2)]
ROCK = [(0.46, 0.45, 0.43), (0.41, 0.4, 0.39), (0.5, 0.48, 0.45)]
MOSS = (0.32, 0.42, 0.22)
GRASS = [(0.3, 0.5, 0.2), (0.36, 0.56, 0.22), (0.26, 0.44, 0.18)]
LEAVES = [(0.22, 0.42, 0.18), (0.27, 0.48, 0.2), (0.2, 0.36, 0.17)]


def pine(p, height, width, tiers, seed_rot=0.0):
    trunk_h = height * 0.22
    p.prism((0, 0), [(0.0, width * 0.1), (trunk_h + height * 0.2, width * 0.06)], 6, "cloth", BARK, var=0.1)
    # Roots flaring at the foot.
    for i in range(4):
        a = seed_rot + i * math.pi / 2 + 0.3
        o = Vector((math.cos(a), math.sin(a), 0))
        p.loft(o * width * 0.06 + Vector((0, 0, 0.18)), o * width * 0.2 + Vector((0, 0, 0.0)), 4, [(0, 0.05, 0.05), (1, 0.02, 0.02)], "cloth", BARK)
    span = height - trunk_h
    for k in range(tiers):
        t = k / tiers
        z = trunk_h + span * t * 0.86
        r = width * (1.0 - t * 0.78) * (0.5 if k == 0 else 0.5)
        h = span * (0.42 - t * 0.12)
        col = p.rng.choice(NEEDLES)
        p.star_tier((0, 0), z, r, h, 7 + (tiers - k) // 2, "cloth", col, var=0.14, inner=0.6, droop=0.16,
                    rot=seed_rot + k * 0.45)


def rock_group(p, kind):
    if kind == 1:
        p.rock((0, 0, 0), (1.3, 1.1, 0.95), "cloth", p.rng.choice(ROCK), var=0.1, rough=0.2)
        p.rock((0.85, 0.3, 0), (0.6, 0.5, 0.42), "cloth", p.rng.choice(ROCK), var=0.1)
        p.rock((-0.6, -0.55, 0), (0.45, 0.4, 0.3), "cloth", p.rng.choice(ROCK), var=0.1)
        p.rock((0.1, 0.05, 0.62), (0.7, 0.55, 0.2), "cloth", MOSS, var=0.12, rough=0.3)
    elif kind == 2:
        for i in range(4):
            a = i * 1.7
            c = (math.cos(a) * 0.5 * (i > 0), math.sin(a) * 0.5 * (i > 0), 0)
            p.rock(c, (1.1 - i * 0.18, 0.9 - i * 0.12, 0.32), "cloth", p.rng.choice(ROCK), var=0.12, rough=0.15)
    else:
        for i, (x, y, h) in enumerate(((0, 0, 1.8), (0.55, 0.2, 1.1), (-0.4, 0.35, 0.8))):
            p.rock((x, y, 0), (0.45, 0.4, h), "cloth", p.rng.choice(ROCK), var=0.12, rough=0.12)
        p.rock((0, 0, 1.55), (0.38, 0.32, 0.14), "cloth", MOSS, var=0.12, rough=0.3)


def grass(p):
    for i in range(14):
        a = p.rng.uniform(0, 2 * math.pi)
        r = p.rng.uniform(0.0, 0.32)
        base = Vector((math.cos(a) * r, math.sin(a) * r, 0))
        lean = Vector((p.rng.uniform(-0.18, 0.18), p.rng.uniform(-0.18, 0.18), 0))
        h = p.rng.uniform(0.35, 0.7)
        side = Vector((-math.sin(a + 1.0), math.cos(a + 1.0), 0)) * 0.05
        tip = base + lean + Vector((0, 0, h))
        col = p.rng.choice(GRASS)
        p.face((base - side, base + side, tip), "cloth", col, var=0.1, ao=0.6)
        p.face((base + side, base - side, tip), "cloth", tuple(x * 0.85 for x in col), var=0.1, ao=0.6)


def bush(p):
    for i in range(5):
        a = i * 1.3
        c = (math.cos(a) * 0.35 * (i > 0), math.sin(a) * 0.35 * (i > 0), 0)
        p.rock(c, (0.75, 0.7, 0.62 - 0.05 * i), "cloth", p.rng.choice(LEAVES), var=0.16, rough=0.28)


BUILDS = {
    "pine_1": lambda p: pine(p, 6.0, 2.6, 5),
    "pine_2": lambda p: pine(p, 5.2, 1.8, 5, 0.6),
    "pine_3": lambda p: pine(p, 4.4, 2.9, 4, 1.1),
    "rock_1": lambda p: rock_group(p, 1),
    "rock_2": lambda p: rock_group(p, 2),
    "rock_3": lambda p: rock_group(p, 3),
    "grass_1": grass,
    "bush_1": bush,
}


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    pid = argv[argv.index("--id") + 1]
    out = argv[argv.index("--out") + 1]

    def build():
        p = prop.Prop(pid, ao_height=4.0 if pid.startswith("pine") else 1.2, seed=sum(map(ord, pid)))
        p.skip_bottoms = True
        BUILDS[pid](p)
        p.finish()

    prop.build_and_save(build, out)
