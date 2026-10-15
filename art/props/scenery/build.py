# SPDX-License-Identifier: AGPL-3.0-or-later
"""Scenery props: pines, rocks, grass and bushes, drawn many times over (the client batches
them and varies each instance's turn, size and tint).

    blender -b --python art/props/scenery/build.py -- --id pine_1 --out art/props/pine_1/pine_1.blend
    blender -b art/props/pine_1/pine_1.blend --python tools/blender/run.py -- export --id pine_1 --kind prop

Ids: pine_1, pine_2, pine_3 (tall, slim, broad), rock_1, rock_2, rock_3, rock_4 (a boulder group,
flat slabs, standing stones, one large leaning boulder), stones_1 (small stones), grass_1 (a
tuft), bush_1.

Winter's variants (15 §9; `seasons.gd` swaps them in): pine_1_snow, pine_2_snow (the pines under
snow, a white cap on every tier), tree_bare_1 (a leafless broadleaf, in place of the broad pine),
grass_winter (a dry straw tuft) and bush_winter (bare red stems in a drift).
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

from mathutils import Vector  # noqa: E402

from mftr_blender import prop  # noqa: E402

# Linear vertex colors (what the shader multiplies): sRGB ≈ these ** (1 / 2.2). Darker and richer
# than the first scenery, so it sits under the champions (05 §1 value hierarchy, R06 §1).
BARK = (0.12, 0.07, 0.04)
NEEDLES = [(0.04, 0.11, 0.05), (0.05, 0.13, 0.05), (0.035, 0.095, 0.045), (0.06, 0.14, 0.06)]
# The paving's and the cliffs' stone (15 §5): blue-grey, so rocks, cliffs and structures are one
# material.
ROCK = [(0.15, 0.165, 0.18), (0.13, 0.14, 0.16), (0.17, 0.18, 0.19)]
MOSS = (0.06, 0.10, 0.035)
GRASS = [(0.08, 0.17, 0.05), (0.1, 0.2, 0.06), (0.07, 0.15, 0.045)]
LEAVES = [(0.05, 0.12, 0.04), (0.07, 0.15, 0.05), (0.045, 0.1, 0.04)]
# Winter: needles darker and bluer under the snow, the snow a little cooler than the ground's
# (the prop shader's snow on upward facets matches it), straw for the dry grass, red stems.
NEEDLES_WINTER = [(0.025, 0.07, 0.055), (0.03, 0.08, 0.06), (0.022, 0.06, 0.05)]
SNOW = (0.68, 0.74, 0.84)
STRAW = [(0.33, 0.25, 0.13), (0.38, 0.29, 0.16), (0.27, 0.2, 0.11)]
STEMS = [(0.22, 0.045, 0.03), (0.26, 0.06, 0.035), (0.18, 0.05, 0.035)]
TWIGS = (0.09, 0.065, 0.05)


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


def snowy_pine(p, height, width, tiers, seed_rot=0.0):
    """A pine under snow: the summer pine's trunk and tiers in winter's darker needles, and on
    each tier a white cap over its upper part, its points short of the needles' so the green shows
    at the tips and the underside."""
    trunk_h = height * 0.22
    p.prism((0, 0), [(0.0, width * 0.1), (trunk_h + height * 0.2, width * 0.06)], 6, "cloth", BARK, var=0.1)
    for i in range(4):
        a = seed_rot + i * math.pi / 2 + 0.3
        o = Vector((math.cos(a), math.sin(a), 0))
        p.loft(o * width * 0.06 + Vector((0, 0, 0.18)), o * width * 0.2, 4, [(0, 0.05, 0.05), (1, 0.02, 0.02)], "cloth", BARK)
    span = height - trunk_h
    for k in range(tiers):
        t = k / tiers
        z = trunk_h + span * t * 0.86
        r = width * (1.0 - t * 0.78) * 0.5
        h = span * (0.42 - t * 0.12)
        points = 7 + (tiers - k) // 2
        rot = seed_rot + k * 0.45
        p.star_tier((0, 0), z, r, h, points, "cloth", p.rng.choice(NEEDLES_WINTER), var=0.12, inner=0.6, droop=0.16, rot=rot)
        # The cap rides on the tier: the same star, smaller and lifted so it sits on the
        # needles; the top tier is snow nearly to its tip.
        lift = h * 0.22 if k < tiers - 1 else h * 0.08
        p.star_tier((0, 0), z + lift, r * 0.78, h * 0.8, points, "cloth", SNOW, var=0.05, inner=0.68, droop=0.12, rot=rot)


def bare_tree(p, height, spread, seed_rot=0.0):
    """A leafless broadleaf: a trunk forking into limbs, each forking again into twigs, thinning
    as it goes (the shader lays snow along their tops)."""
    def limb(a, b, r0, r1, depth):
        p.loft(a, b, 5 if r0 > 0.05 else 4, [(0, r0, r0), (1, r1, r1)], "cloth", BARK if depth < 2 else TWIGS)
        if depth == 3:
            return
        d = b - a
        n = 3 if depth == 0 else 2
        for i in range(n):
            ang = seed_rot + depth * 1.7 + i * (2 * math.pi / n) + p.rng.uniform(-0.4, 0.4)
            out = Vector((math.cos(ang), math.sin(ang), 0))
            up = 0.9 if depth == 0 else 0.6
            nd = (d.normalized() * up + out * (1.0 - up * 0.5)).normalized()
            length = d.length * (0.62 if depth == 0 else 0.58) * p.rng.uniform(0.85, 1.15)
            start = b if depth == 0 else a.lerp(b, p.rng.uniform(0.55, 1.0))
            limb(start, start + nd * length, r1, r1 * 0.55, depth + 1)

    for i in range(4):
        a = seed_rot + i * math.pi / 2 + 0.5
        o = Vector((math.cos(a), math.sin(a), 0))
        p.loft(o * 0.1 + Vector((0, 0, 0.3)), o * spread * 0.16, 4, [(0, 0.08, 0.08), (1, 0.03, 0.03)], "cloth", BARK)
    top = Vector((p.rng.uniform(-0.2, 0.2), p.rng.uniform(-0.2, 0.2), height * 0.42))
    limb(Vector((0, 0, 0)), top, 0.2, 0.12, 0)


def rock_group(p, kind):
    """Boulders (15 §4), weathered and mossy; every foot sunk into the ground."""
    c = lambda: p.rng.choice(ROCK)  # noqa: E731
    if kind == 1:
        # A big boulder with two smaller at its foot.
        p.boulder((0, 0, 0), (1.5, 1.25, 1.15), c(), MOSS, cuts=8)
        p.boulder((0.9, 0.35, 0), (0.62, 0.55, 0.5), c(), MOSS, cuts=6, subdiv=2)
        p.boulder((-0.7, -0.55, 0), (0.48, 0.42, 0.34), c(), MOSS, cuts=6, subdiv=2)
    elif kind == 2:
        # Flat slabs, tilted and stacked.
        for i, (x, y, w, h, tx) in enumerate(((0, 0, 1.4, 0.4, 0.06), (0.5, 0.25, 0.9, 0.32, -0.2), (-0.45, 0.4, 0.7, 0.28, 0.25))):
            p.boulder((x, y, 0.05 * i), (w, w * 0.75, h), c(), MOSS, rough=0.08, cuts=7, subdiv=2, tilt=(tx, 0.1 * i))
    elif kind == 3:
        # Standing stones: a tall one and two leaning against it.
        p.boulder((0, 0, 0), (0.55, 0.5, 1.9), c(), MOSS, rough=0.12, cuts=7)
        p.boulder((0.5, 0.2, 0), (0.42, 0.38, 1.15), c(), MOSS, rough=0.12, cuts=6, subdiv=2, tilt=(0.0, 0.25))
        p.boulder((-0.42, 0.32, 0), (0.36, 0.34, 0.85), c(), MOSS, rough=0.12, cuts=6, subdiv=2, tilt=(0.2, -0.2))
    elif kind == 4:
        # One large boulder that leans on a cliff's face.
        p.boulder((0, 0, 0), (1.7, 1.4, 1.5), c(), MOSS, rough=0.18, cuts=9, tilt=(0.12, 0.0))
    else:
        # A scatter of small stones.
        for i in range(6):
            a = i * 1.1 + 0.3
            r = 0.15 + 0.45 * (i % 3) / 2
            s = 0.18 + 0.12 * ((i * 7) % 3) / 2
            p.boulder((math.cos(a) * r, math.sin(a) * r, 0), (s * 1.3, s, s * 0.7), c(), None, cuts=5, subdiv=1)


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


def winter_grass(p):
    """A dry tuft: fewer, shorter blades in straw, some bent over."""
    for i in range(11):
        a = p.rng.uniform(0, 2 * math.pi)
        r = p.rng.uniform(0.0, 0.3)
        base = Vector((math.cos(a) * r, math.sin(a) * r, 0))
        bent = p.rng.random() < 0.3
        lean = Vector((p.rng.uniform(-0.3, 0.3), p.rng.uniform(-0.3, 0.3), 0)) * (2.0 if bent else 1.0)
        h = p.rng.uniform(0.25, 0.55) * (0.6 if bent else 1.0)
        side = Vector((-math.sin(a + 1.0), math.cos(a + 1.0), 0)) * 0.045
        tip = base + lean + Vector((0, 0, h))
        col = p.rng.choice(STRAW)
        p.face((base - side, base + side, tip), "cloth", col, var=0.12, ao=0.6)
        p.face((base + side, base - side, tip), "cloth", tuple(x * 0.85 for x in col), var=0.12, ao=0.6)


def winter_bush(p):
    """A bare shrub in a drift: red stems fanning up out of a low mound of snow."""
    p.rock((0, 0, 0), (0.95, 0.85, 0.28), "cloth", SNOW, var=0.04, rough=0.2)
    for i in range(9):
        a = i * 2.4 + p.rng.uniform(-0.3, 0.3)
        o = Vector((math.cos(a), math.sin(a), 0))
        foot = o * p.rng.uniform(0.0, 0.22) + Vector((0, 0, 0.05))
        tip = foot + o * p.rng.uniform(0.25, 0.55) + Vector((0, 0, p.rng.uniform(0.7, 1.15)))
        col = p.rng.choice(STEMS)
        p.loft(foot, tip, 4, [(0, 0.03, 0.03), (1, 0.01, 0.01)], "cloth", col)
        # A side shoot from partway up.
        mid = foot.lerp(tip, p.rng.uniform(0.4, 0.65))
        side = Vector((-o.y, o.x, 0)) * p.rng.choice((-1, 1))
        p.loft(mid, mid + side * 0.25 + o * 0.1 + Vector((0, 0, 0.3)), 4, [(0, 0.018, 0.018), (1, 0.008, 0.008)], "cloth", col)


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
    "rock_4": lambda p: rock_group(p, 4),
    "stones_1": lambda p: rock_group(p, 5),
    "grass_1": grass,
    "bush_1": bush,
    "pine_1_snow": lambda p: snowy_pine(p, 6.0, 2.6, 5),
    "pine_2_snow": lambda p: snowy_pine(p, 5.2, 1.8, 5, 0.6),
    "tree_bare_1": lambda p: bare_tree(p, 5.6, 3.0, 0.4),
    "grass_winter": winter_grass,
    "bush_winter": winter_bush,
}


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    pid = argv[argv.index("--id") + 1]
    out = argv[argv.index("--out") + 1]

    def build():
        p = prop.Prop(pid, ao_height=4.0 if pid.startswith(("pine", "tree")) else 1.2, seed=sum(map(ord, pid)))
        p.skip_bottoms = True
        BUILDS[pid](p)
        p.finish()

    prop.build_and_save(build, out)
