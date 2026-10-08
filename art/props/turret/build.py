# SPDX-License-Identifier: AGPL-3.0-or-later
"""The lane turret (a map prop): a round stone watchtower, ~5.3 m tall.

    blender -b --python art/props/turret/build.py -- --out art/props/turret/turret.blend
    blender -b art/props/turret/turret.blend --python tools/blender/run.py -- export --id turret --kind prop

Courses of dressed stone with a few mossy blocks, iron bands, arrow slits and a studded door, a
flared corbel under a crenellated parapet, team-colored banners, and the turret's eye on top: a
team-colored crystal held in iron claws. It stands slim at the foot (champions come within
0.95 m of its center) and broad above their heads; a team-colored roof and a spire carry the eye to ~6.6 m. Front: -Y (the lane side).
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
STONE_DARK = (0.38, 0.37, 0.36)
TRIM = (0.56, 0.53, 0.47)
MOSS = (0.36, 0.43, 0.26)
SLATE = (0.28, 0.31, 0.38)
IRON = (0.28, 0.29, 0.32)
WOOD = (0.36, 0.24, 0.14)
SLIT = (0.06, 0.05, 0.05)
TEAM = (0.8, 0.8, 0.8)                   # the accent slot takes the team color


def build():
    p = prop.Prop("turret", ao_height=4.5, seed=7)
    # Footing: an octagonal plinth in two steps.
    p.prism((0, 0), [(0.0, 0.92), (0.16, 0.9), (0.16, 0.84), (0.34, 0.8)], 8, "cloth", STONE_DARK, var=0.08, rot=math.pi / 8)
    # The tower: courses of dressed stone over a core that hides the joints.
    p.prism((0, 0), [(0.3, 0.6), (3.5, 0.55)], 10, "cloth", STONE_DARK, cap_top=False)
    p.blocks_ring((0, 0), 0.72, 0.34, 9, 0.35, 11, 0.13, "cloth", STONE, var=0.13, taper=0.008, alt=MOSS, alt_every=9)
    # Iron bands.
    for z in (1.05, 2.82):
        r = 0.74 - 0.008 * (z - 0.34) / 0.35
        p.prism((0, 0), [(z, r), (z + 0.08, r)], 16, "metal", IRON, var=0.05, cap_top=False)
    # Arrow slits, front, back and sides.
    for a in (-math.pi / 2, 0, math.pi / 2, math.pi):
        for z in (1.6, 2.4):
            c = Vector((math.cos(a), math.sin(a), 0)) * 0.7
            p.box((c.x, c.y, z), (0.08, 0.1, 0.42), "cloth", SLIT, rot_z=a)
    # A studded door at the foot, lane side.
    p.box((0, -0.68, 0.34), (0.42, 0.1, 0.78), "cloth", WOOD, var=0.06)
    for z in (0.5, 0.82):
        p.box((0, -0.735, z), (0.44, 0.03, 0.05), "metal", IRON)
    p.box((0, -0.72, 1.12), (0.52, 0.12, 0.08), "cloth", TRIM)
    # Corbel: the wall flares out under the parapet.
    p.prism((0, 0), [(3.48, 0.68), (3.62, 0.82), (3.78, 0.98), (3.86, 1.0)], 12, "cloth", TRIM, var=0.1)
    for i in range(12):
        a = 2 * math.pi * (i + 0.5) / 12
        c = Vector((math.cos(a), math.sin(a), 0)) * 0.8
        p.box((c.x, c.y, 3.36), (0.12, 0.16, 0.42), "cloth", STONE_DARK, var=0.08, rot_z=a, taper=1.4)
    # The parapet: a low wall with merlons.
    p.prism((0, 0), [(3.86, 1.0), (4.12, 1.0)], 16, "cloth", TRIM, var=0.08, cap_top=False)
    p.prism((0, 0), [(3.86, 0.9), (4.12, 0.9)], 16, "cloth", STONE_DARK, cap_top=False, inward=True)
    ring = p.ring((0, 0), 1.0, 1.0, 16, 4.12)
    inner = p.ring((0, 0), 0.9, 0.9, 16, 4.12)
    for i in range(16):
        j = (i + 1) % 16
        p.face((ring[i], ring[j], inner[j], inner[i]), "cloth", TRIM, var=0.1, out=(0, 0, 1))
    for i in range(8):
        a = 2 * math.pi * i / 8
        c = Vector((math.cos(a), math.sin(a), 0)) * 0.95
        p.box((c.x, c.y, 4.12), (0.34, 0.16, 0.36), "cloth", TRIM, var=0.12, rot_z=a + math.pi / 2)
    p.prism((0, 0), [(3.84, 0.9), (3.9, 0.9)], 12, "cloth", STONE_DARK)   # the floor
    # Team banners from the parapet, lane side, swallow-tailed.
    for a in (-math.pi / 2 - 0.62, -math.pi / 2 + 0.62):
        out = Vector((math.cos(a), math.sin(a), 0))
        side = Vector((-out.y, out.x, 0))
        base = out * 1.04
        top, bot = 3.82, 2.25
        w = 0.21
        pts = [base + side * w + Vector((0, 0, top)), base - side * w + Vector((0, 0, top)),
               base - side * w + Vector((0, 0, bot)), base + Vector((0, 0, bot + 0.22)), base + side * w + Vector((0, 0, bot))]
        p.face(pts, "accent", TEAM, out=out)
        p.face(list(reversed([q - out * 0.02 for q in pts])), "accent", (0.6, 0.6, 0.6), out=-out)
        p.box((base.x, base.y, top), (0.5, 0.05, 0.05), "metal", IRON, rot_z=a + math.pi / 2)
    # A roof of team-colored shingles inside the parapet, and the eye on a spire above:
    # a team-colored crystal in iron claws (what the gameplay camera sees first).
    p.prism((0, 0), [(3.9, 0.88), (4.02, 0.88)], 16, "cloth", SLATE, cap_top=False)
    p.shingled_cone((0, 0), 4.0, 0.86, 1.55, 14, 5, "accent", TEAM, var=0.25)
    p.prism((0, 0), [(5.42, 0.06), (5.72, 0.05)], 6, "metal", IRON)
    p.prism((0, 0), [(5.62, 0.16), (5.7, 0.12)], 8, "metal", IRON)
    for i in range(4):
        a = 2 * math.pi * i / 4 + math.pi / 4
        o = Vector((math.cos(a), math.sin(a), 0))
        p.loft(o * 0.12 + Vector((0, 0, 5.66)), o * 0.3 + Vector((0, 0, 6.0)), 4, [(0, 0.035, 0.035), (1, 0.025, 0.025)], "metal", IRON)
        p.loft(o * 0.3 + Vector((0, 0, 6.0)), o * 0.16 + Vector((0, 0, 6.42)), 4, [(0, 0.025, 0.025), (1, 0.006, 0.006)], "metal", IRON)
    p.prism((0, 0), [(5.72, 0.0), (6.05, 0.26), (6.6, 0.0)], 6, "accent_glow", TEAM, cap_top=False)
    p.finish()


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    out = argv[argv.index("--out") + 1] if "--out" in argv else os.path.join(HERE, "turret.blend")
    prop.build_and_save(build, out)
