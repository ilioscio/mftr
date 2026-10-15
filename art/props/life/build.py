# SPDX-License-Identifier: AGPL-3.0-or-later
"""Small life (15 §8): flower clumps, a lily pad and a frog, drawn many times over by the client
(`life.gd`), which places them from each map's own ground and animates the frogs' hops.

    blender -b --python art/props/life/build.py -- --id flowers_1 --out art/props/flowers_1/flowers_1.blend
    blender -b art/props/flowers_1/flowers_1.blend --python tools/blender/run.py -- export --id flowers_1 --kind prop

Ids: flowers_1 (purple and blue), flowers_2 (yellow and white), lily_1 (a pad with a bloom),
frog_1. Small and faceted, in linear vertex colors like the rest of the scenery.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mathutils import Vector  # noqa: E402

from mftr_blender import prop  # noqa: E402

STEM = (0.05, 0.12, 0.04)
LEAF = (0.045, 0.11, 0.035)
PURPLE = (0.2, 0.06, 0.32)
BLUE = (0.08, 0.16, 0.45)
YELLOW = (0.6, 0.45, 0.06)
WHITE = (0.62, 0.62, 0.58)
PAD = (0.04, 0.12, 0.05)
PAD_DARK = (0.025, 0.07, 0.03)
LILY = (0.62, 0.42, 0.5)
FROG = (0.09, 0.2, 0.05)
FROG_BELLY = (0.35, 0.33, 0.12)
EYE = (0.01, 0.01, 0.01)


def bloom(p, c, z, r, petals, color, center):
    """A flat star of petals facing up, a dot of `center` in its middle."""
    for i in range(petals):
        a0 = 2 * math.pi * i / petals
        a1 = a0 + 2 * math.pi / petals * 0.8
        am = (a0 + a1) / 2
        tip = Vector((c[0] + math.cos(am) * r, c[1] + math.sin(am) * r, z + 0.01))
        p.face((Vector((c[0], c[1], z)), Vector((c[0] + math.cos(a0) * r * 0.45, c[1] + math.sin(a0) * r * 0.45, z)), tip,
                Vector((c[0] + math.cos(a1) * r * 0.45, c[1] + math.sin(a1) * r * 0.45, z))), "cloth", color, var=0.12, out=(0, 0, 1), ao=1.0)
    p.prism(c, [(z, 0.0), (z + 0.01, r * 0.25), (z + 0.02, 0.0)], 4, "cloth", center, cap_top=False)


def flowers(p, colors):
    # A clump: leaves at the foot, five or six stems with a bloom each.
    for i in range(5):
        a = i * 1.26
        base = Vector((math.cos(a) * 0.04, math.sin(a) * 0.04, 0))
        tip = Vector((math.cos(a) * 0.14, math.sin(a) * 0.14, 0.1))
        side = Vector((-math.sin(a), math.cos(a), 0)) * 0.03
        p.face((base - side, base + side, tip), "cloth", LEAF, var=0.1, ao=0.7)
        p.face((base + side, base - side, tip), "cloth", LEAF, var=0.1, ao=0.7)
    for i in range(6):
        a = i * 1.05 + p.rng.uniform(-0.3, 0.3)
        r = p.rng.uniform(0.03, 0.13)
        c = (math.cos(a) * r, math.sin(a) * r)
        h = p.rng.uniform(0.16, 0.3)
        p.loft((c[0] * 0.5, c[1] * 0.5, 0), (c[0], c[1], h), 3, [(0, 0.006, 0.006), (1, 0.005, 0.005)], "cloth", STEM)
        bloom(p, c, h, p.rng.uniform(0.035, 0.05), 5, colors[i % len(colors)], YELLOW if colors[i % len(colors)] != YELLOW else WHITE)


def lily(p):
    # A round pad with a notch, flat on the water, and a pink bloom.
    n = 12
    ring = []
    for i in range(n + 1):
        a = 0.35 + (2 * math.pi - 0.7) * i / n
        ring.append(Vector((math.cos(a) * 0.32, math.sin(a) * 0.32, 0.005)))
    center = Vector((0, 0, 0.012))
    for i in range(n):
        p.face((center, ring[i], ring[i + 1]), "cloth", PAD if i % 3 else PAD_DARK, var=0.1, out=(0, 0, 1), ao=1.0)
    bloom(p, (0.08, -0.06), 0.03, 0.09, 7, LILY, YELLOW)


def frog(p):
    # Sitting: a squat body, folded hind legs, front legs, two bulging eyes.
    p.boulder((0, 0, 0), (0.16, 0.2, 0.1), FROG, None, rough=0.05, cuts=2, subdiv=2, sink=0.0)
    p.boulder((0, -0.02, 0.0), (0.12, 0.14, 0.05), FROG_BELLY, None, rough=0.02, cuts=0, subdiv=1, sink=0.0)
    for s in (1, -1):
        p.boulder((s * 0.08, 0.05, 0), (0.07, 0.11, 0.06), FROG, None, rough=0.05, cuts=1, subdiv=1, sink=0.0)
        p.boulder((s * 0.045, -0.085, 0), (0.035, 0.05, 0.05), FROG, None, rough=0.05, cuts=0, subdiv=1, sink=0.0)
        p.boulder((s * 0.04, -0.06, 0.075), (0.035, 0.035, 0.035), FROG, None, rough=0.0, cuts=0, subdiv=1, sink=0.0)
        p.boulder((s * 0.042, -0.072, 0.085), (0.016, 0.016, 0.016), EYE, None, rough=0.0, cuts=0, subdiv=1, sink=0.0)


BUILDS = {
    "flowers_1": lambda p: flowers(p, [PURPLE, BLUE, PURPLE]),
    "flowers_2": lambda p: flowers(p, [YELLOW, WHITE, YELLOW]),
    "lily_1": lily,
    "frog_1": frog,
}


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    pid = argv[argv.index("--id") + 1]
    out = argv[argv.index("--out") + 1]

    def build():
        p = prop.Prop(pid, ao_height=0.4, seed=sum(map(ord, pid)))
        p.skip_bottoms = True
        BUILDS[pid](p)
        p.finish()

    prop.build_and_save(build, out)
