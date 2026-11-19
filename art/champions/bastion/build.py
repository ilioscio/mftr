# SPDX-License-Identifier: AGPL-3.0-or-later
"""Bastion (rebuilt for 14): the generator for his model and clips.

    blender -b --python art/champions/bastion/build.py -- --out art/champions/bastion/bastion.blend

A boulder (14 §5): a stone guardian whose shoulders stand above his head, on the `biped` bones
in hulking proportions (short stumpy legs, a huge chest, long arms ending in boulder fists). Ice
crystals grow from his back and shoulders and glow in his chest and visor: his signature is ice
blue (14 §4). A chain wound round his right fist ends in the grapple's hook.

Weight (14 §3): he stands hunched with his fists hanging low; blows wind up with the whole body,
land in 1–2 frames and hold; the stomps and slams leave his fists in the ground a beat. Feet are
planted by IK.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

from mathutils import Vector  # noqa: E402

from mftr_blender import champion, clips, rig  # noqa: E402
from mftr_blender.champion import arm, feet, hand  # noqa: E402
from mftr_blender.clips import at, merge, sym  # noqa: E402
from mftr_blender.library import _clip, gait  # noqa: E402
from mftr_blender.mesh import box_part, path_part, rock_part  # noqa: E402

# Warm granite, so the ice reads against it.
STONE = (0.24, 0.22, 0.21)
STONE_LIGHT = (0.32, 0.30, 0.27)
STONE_DARK = (0.13, 0.12, 0.12)
IRON = (0.11, 0.11, 0.13)
# Ice as ice, not magic: pale, lit like the stone (not glowing), white with rime where it's thin
# or catches the frost; only his eyes glow.
ICE = (0.50, 0.72, 0.86)
ICE_DEEP = (0.26, 0.48, 0.66)
FROST = (0.82, 0.88, 0.92)
GLOW = (0.62, 0.90, 1.0)
ACCENT = (0.18, 0.52, 0.95)

# Hulking (14 §2.2): low hips and short legs, the neck below the shoulder line, very broad and
# deep, the shoulders pushed far out, long arms, huge hands.
SHAPE = {
    "heights": [(0.0, 0.0), (0.03, 0.03), (0.09, 0.09), (0.54, 0.42), (1.00, 0.84), (1.12, 0.98),
                (1.26, 1.14), (1.40, 1.32), (1.56, 1.52), (1.64, 1.47), (1.88, 1.70), (2.15, 1.98)],
    "x": 1.45, "y": 1.35, "scale": 1.12, "shoulder": 0.16, "arm": 1.22, "hand": 1.9,
}
rig.use_shape(SHAPE)


def sp(p):
    return rig.shaped(p)


_SEEDS = iter(range(100, 1000))


def lump(bone, center, size, color, rot=(0.0, 0.0, 0.0), moss=None, grow=1.2, cuts=4):
    """A chipped stone (`rock_part`) where the block-out had a box: a little larger, as the
    chipping cuts it back, each with its own seed."""
    return rock_part(bone, center, tuple(x * grow for x in size), color, moss=moss, seed=next(_SEEDS), cuts=cuts, rot=rot)


# --- Mesh -----------------------------------------------------------------------------------

def cluster(bone, base, out, length, width, colors=(None, None, None), seed=0):
    """Three faceted crystal shards growing from `base` (a point or bone point) along `out`
    (armature space), the middle one longest, fanned apart."""
    o = Vector(out).normalized()
    side = o.cross(Vector((0, 0, 1)))
    side = side.normalized() if side.length > 1e-3 else Vector((1, 0, 0))
    up = side.cross(o).normalized()
    parts = []
    for k, (spread, lf, wf) in enumerate(((-0.45, 0.62, 0.7), (0.0, 1.0, 1.0), (0.5, 0.74, 0.78))):
        d = (o + side * spread * 0.6 + up * (0.15 * ((k + seed) % 2) - 0.05)).normalized()
        L, W = length * lf * 0.85, width * wf * 1.4
        if isinstance(base[0], str):
            pts = [(base[0], base[1], tuple(Vector(base[2] if len(base) > 2 else (0, 0, 0)) + d * f * L)) for f in (0.0, 0.45, 0.85, 1.0)]
        else:
            pts = [tuple(Vector(base) + d * f * L) for f in (0.0, 0.45, 0.85, 1.0)]
        col = (ICE, FROST, ICE_DEEP)[(k + seed) % 3]
        parts.append(path_part(bone, pts, 6, [(W, W * 0.85), (W * 0.95, W * 0.8), (W * 0.55, W * 0.5), (0, 0)], "cloth", col))
    return parts


def icicles(bone, roots, colors=(FROST, ICE)):
    """Icicles hanging straight down from `roots` ((bone point or armature point), length, width)."""
    out = []
    for i, (root, length, width) in enumerate(roots):
        if isinstance(root[0], str):
            b, t, off = root
            pts = [(b, t, off), (b, t, (off[0], off[1], off[2] - length * 0.55)), (b, t, (off[0], off[1], off[2] - length))]
        else:
            pts = [root, (root[0], root[1], root[2] - length * 0.55), (root[0], root[1], root[2] - length)]
        out.append(path_part(bone, pts, 4, [(width, width), (width * 0.6, width * 0.6), (0, 0)], "cloth", colors[i % len(colors)]))
    return out


def _center():
    return [
        # Boulder hips, belly and the great chest, broader at the top.
        lump("pelvis", sp((0, 0, 0.98)), (0.50, 0.42, 0.30), STONE_DARK),
        ("pelvis", sp((0, 0, 1.00)), sp((0, 0, 1.06)), 8, [(0, .29, .25), (1, .29, .25)], "accent", ACCENT),
        lump("spine_01", sp((0, 0, 1.18)), (0.58, 0.46, 0.30), STONE),
        lump("spine_02", sp((0, 0.02, 1.40)), (0.84, 0.62, 0.40), STONE),
        # the chest's top as interlocking chipped stones over an ice core.
        rock_part("chest", sp((0, 0.06, 1.58)), (0.70, 0.60, 0.30), STONE_LIGHT, moss=FROST, seed=21, cuts=5),
        rock_part("chest", sp((0.20, -0.10, 1.55)), (0.36, 0.34, 0.26), STONE, seed=22, cuts=4),
        rock_part("chest", sp((-0.20, -0.10, 1.55)), (0.36, 0.34, 0.26), STONE, seed=23, cuts=4, flip=True),
        *cluster("chest", sp((0.0, -0.18, 1.62)), (0.0, -0.6, 0.8), 0.12, 0.035, seed=3),
        # The ice heart: a chunk of old ice set into his chest, frosted over, flush with the stone.
        rock_part("chest", sp((0, -0.33, 1.45)), (0.30, 0.12, 0.24), ICE, moss=FROST, seed=24, cuts=5),
        # Rock lumps across the back and belly.
        lump("spine_02", sp((0.22, 0.25, 1.30)), (0.20, 0.14, 0.18), STONE_DARK, rot=(10, 20, 15)),
        lump("spine_01", sp((0.18, 0.20, 1.12)), (0.16, 0.12, 0.14), STONE, rot=(-10, 10, 25)),
        lump("spine_01", sp((-0.18, 0.21, 1.14)), (0.15, 0.12, 0.14), STONE_LIGHT, rot=(8, -12, -20)),
        lump("spine_02", sp((-0.20, 0.27, 1.36)), (0.18, 0.14, 0.20), STONE_LIGHT, rot=(-12, -10, 25)),
        lump("spine_01", sp((0.10, -0.22, 1.14)), (0.20, 0.12, 0.14), STONE_LIGHT, rot=(5, 15, -10)),
        # Crystals growing from his back: the silhouette's spikes.
        path_part("chest", [sp((0.08, 0.22, 1.55)), sp((0.14, 0.42, 1.85)), sp((0.17, 0.50, 2.02))], 5,
                  [(.08, .07), (.05, .045), (0, 0)], "cloth", ICE_DEEP),
        path_part("chest", [sp((-0.12, 0.24, 1.50)), sp((-0.22, 0.46, 1.74)), sp((-0.28, 0.54, 1.86))], 5,
                  [(.07, .06), (.045, .04), (0, 0)], "cloth", ICE),
        path_part("spine_02", [sp((0.02, 0.26, 1.34)), sp((0.04, 0.50, 1.52)), sp((0.05, 0.60, 1.60))], 5,
                  [(.06, .05), (.035, .03), (0, 0)], "cloth", ICE_DEEP),
        path_part("chest", [sp((0.24, 0.20, 1.60)), sp((0.36, 0.38, 1.80)), sp((0.41, 0.44, 1.88))], 5,
                  [(.06, .05), (.035, .03), (0, 0)], "cloth", ICE),
        path_part("chest", [sp((-0.30, 0.18, 1.58)), sp((-0.42, 0.34, 1.82)), sp((-0.47, 0.40, 1.92))], 5,
                  [(.07, .06), (.04, .035), (0, 0)], "cloth", ICE_DEEP),
        path_part("spine_02", [sp((-0.18, 0.28, 1.28)), sp((-0.26, 0.50, 1.38)), sp((-0.29, 0.58, 1.42))], 5,
                  [(.05, .045), (.03, .025), (0, 0)], "cloth", ICE),
        lump("pelvis", sp((0.18, -0.20, 0.92)), (0.16, 0.10, 0.14), STONE_LIGHT, rot=(0, 10, 20)),
        lump("pelvis", sp((-0.16, -0.21, 0.94)), (0.15, 0.10, 0.13), STONE, rot=(0, -12, -15)),
        # Crystal clusters across his back.
        *cluster("chest", sp((0.0, 0.26, 1.62)), (0.0, 0.5, 0.9), 0.36, 0.06, seed=0),
        *cluster("spine_02", sp((0.20, 0.27, 1.40)), (0.4, 0.8, 0.5), 0.28, 0.05, seed=1),
        *cluster("spine_02", sp((-0.20, 0.27, 1.38)), (-0.4, 0.8, 0.5), 0.26, 0.05, seed=2),
        # Rock plates over his chest and belly.
        *[lump("spine_02", sp((x, -0.29, z)), (0.18, 0.10, 0.15), c, rot=(r, 0, x * 40))
          for x, z, r, c in ((0.16, 1.50, -8, STONE_LIGHT), (-0.16, 1.50, -8, STONE_LIGHT), (0.20, 1.33, 4, STONE), (-0.20, 1.33, 4, STONE))],
        *[lump("spine_01", sp((x, -0.24, 1.15)), (0.14, 0.05, 0.11), STONE_DARK, rot=(0, 0, x * 30)) for x in (0.11, -0.11)],
        # Moss on the top of his back.
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        # Boulder shoulders standing above his head, moss on top, a crystal from each.
        # the shoulder a heap of chipped boulders, an ice seam glowing
        # where they meet the arm.
        rock_part("clavicle_l", ("clavicle_l", 1.0, (0.12, 0.02, 0.12)), (0.60, 0.56, 0.40), STONE_LIGHT, moss=FROST, seed=11, cuts=5),
        rock_part("clavicle_l", ("clavicle_l", 1.0, (0.30, 0.02, -0.06)), (0.34, 0.40, 0.34), STONE, seed=12, cuts=4),
        rock_part("clavicle_l", ("clavicle_l", 0.4, (0.0, 0.08, 0.08)), (0.34, 0.40, 0.26), STONE_DARK, seed=13, cuts=4),
        rock_part("clavicle_l", ("clavicle_l", 1.0, (0.10, -0.20, 0.0)), (0.26, 0.20, 0.24), STONE, seed=14, cuts=3),
        *cluster("clavicle_l", ("clavicle_l", 1.0, (0.22, -0.16, -0.06)), (0.5, -0.8, 0.3), 0.11, 0.03, seed=2),
        *icicles("clavicle_l", [(("clavicle_l", 1.0, (0.06, -0.20, -0.04)), 0.14, 0.025), (("clavicle_l", 1.0, (0.18, -0.18, -0.10)), 0.10, 0.02),
                                (("clavicle_l", 1.0, (0.36, -0.06, -0.20)), 0.16, 0.028), (("clavicle_l", 1.0, (0.40, 0.10, -0.18)), 0.11, 0.022)]),
        path_part("clavicle_l", [("clavicle_l", 1.0, (0.12, 0.10, 0.30)), ("clavicle_l", 1.0, (0.20, 0.22, 0.56)),
                                 ("clavicle_l", 1.0, (0.24, 0.27, 0.70))], 6, [(.09, .08), (.05, .045), (0, 0)], "cloth", ICE),
        path_part("clavicle_l", [("clavicle_l", 1.0, (0.24, 0.04, 0.22)), ("clavicle_l", 1.0, (0.40, 0.10, 0.40)),
                                 ("clavicle_l", 1.0, (0.46, 0.12, 0.47))], 5, [(.06, .05), (.035, .03), (0, 0)], "cloth", ICE_DEEP),
        path_part("clavicle_l", [("clavicle_l", 1.0, (0.02, 0.16, 0.24)), ("clavicle_l", 1.0, (0.04, 0.34, 0.42)),
                                 ("clavicle_l", 1.0, (0.05, 0.40, 0.48))], 5, [(.05, .045), (.03, .025), (0, 0)], "cloth", ICE),
        *cluster("clavicle_l", ("clavicle_l", 1.0, (0.30, 0.0, 0.06)), (1.0, 0.1, 0.5), 0.24, 0.05, seed=1),
        # arms of chipped stones over a glowing ice core that shows
        # through the gaps, the forearms swelling toward the fists.
        (ua, (ua, 0), (ua, 1), 8, [(0, .13, .13), (.5, .14, .14), (1, .12, .12)], "cloth", ICE_DEEP),
        (fa, (fa, 0), (fa, 1), 8, [(0, .12, .12), (.5, .14, .14), (1, .15, .15)], "cloth", ICE_DEEP),
        rock_part(ua, (ua, 0.22), (0.34, 0.32, 0.32), STONE, seed=31, cuts=4),
        rock_part(ua, (ua, 0.70), (0.30, 0.30, 0.30), STONE_DARK, seed=32, cuts=4),
        rock_part(fa, (fa, 0.28), (0.32, 0.30, 0.30), STONE_LIGHT, seed=33, cuts=4),
        rock_part(fa, (fa, 0.72), (0.40, 0.38, 0.36), STONE, seed=34, cuts=5),
        (fa, (fa, .90), (fa, 1.0), 8, [(0, .205, .195), (1, .205, .195)], "metal", IRON),
        # Boulder fists.
        lump("hand_l", ("hand_l", 0.45), (0.38, 0.33, 0.36), STONE_DARK),
        lump("fingers_l", ("fingers_l", 0.3), (0.36, 0.30, 0.20), STONE),
        lump("thumb_l", ("thumb_l", 0.5), (0.14, 0.14, 0.14), STONE_DARK),
        lump("fingers_l", ("fingers_l", 0.7, (0.09, -0.06, 0)), (0.10, 0.10, 0.10), STONE_LIGHT, rot=(0, 20, 0)),
        lump("fingers_l", ("fingers_l", 0.7, (-0.01, -0.08, 0)), (0.10, 0.10, 0.10), STONE_LIGHT, rot=(0, -10, 0)),
        lump("fingers_l", ("fingers_l", 0.7, (-0.10, -0.06, 0)), (0.10, 0.10, 0.10), STONE_LIGHT, rot=(0, 10, 0)),
        # Rock lumps on the arms and a crystal on each forearm's back.
        *cluster(fa, (fa, 0.5, (0.04, 0.12, 0.02)), (0.3, 1.0, 0.4), 0.26, 0.045, seed=0),
        # Short stumpy legs and boulder feet.
        # Legs: chipped stones over the ice core, as the arms.
        (th, sp((0.10, 0, 1.00)), (th, 1), 8, [(0, .14, .14), (1, .12, .12)], "cloth", ICE_DEEP),
        rock_part(th, (th, 0.30), (0.40, 0.38, 0.34), STONE_DARK, seed=61, cuts=4),
        rock_part(th, (th, 0.78), (0.32, 0.32, 0.28), STONE, seed=62, cuts=4),
        lump(ca, (ca, 0.02, (0, -0.07, 0)), (0.24, 0.12, 0.20), STONE_LIGHT),
        lump(th, (th, 0.4, (0.08, -0.04, 0)), (0.14, 0.14, 0.18), STONE_LIGHT, rot=(10, 15, 25)),
        lump(ca, (ca, 0.55, (0.06, 0.06, 0)), (0.12, 0.12, 0.14), STONE_DARK, rot=(-10, 20, 10)),
        lump(th, (th, 0.75, (-0.06, -0.08, 0)), (0.12, 0.12, 0.14), STONE, rot=(15, -10, 20)),
        *cluster(ca, (ca, 0.05, (0.06, -0.06, 0.02)), (0.6, -0.6, 0.6), 0.16, 0.035, seed=0),
        *cluster(th, (th, 0.1, (0.12, 0.02, 0.0)), (1.0, 0.2, 0.3), 0.18, 0.04, seed=1),
        lump("foot_l", ("foot_l", 0.2, (0.10, 0.04, 0.06)), (0.12, 0.14, 0.12), STONE_LIGHT, rot=(0, 15, 20)),
        lump("pelvis", sp((0.22, 0.06, 1.02)), (0.12, 0.18, 0.16), STONE, rot=(0, 20, 0)),
        (ca, (ca, 0), (ca, 1), 8, [(0, .12, .12), (1, .14, .14)], "cloth", ICE_DEEP),
        rock_part(ca, (ca, 0.40), (0.34, 0.34, 0.32), STONE, seed=63, cuts=4),
        rock_part(ca, (ca, 0.78), (0.38, 0.38, 0.24), STONE_DARK, seed=64, cuts=4),
        lump("foot_l", ("foot_l", 0.45, (0, 0, 0.02)), (0.28, 0.36, 0.18), STONE_DARK, grow=1.0),
        lump("toe_l", ("toe_l", 0.3, (0, 0, 0.015)), (0.27, 0.14, 0.12), STONE, grow=1.0),
    ]


def _head():
    hb = "head"
    return [
        # A craggy face pushed forward from the shoulders: two brow stones angled down to the
        # middle (stern), deep sockets with small cold eyes in them, a broad nose stone, a heavy
        # jaw, and a beard of icicles hanging from it.
        rock_part(hb, sp((0, -0.140, 1.760)), (0.34, 0.30, 0.30), STONE_LIGHT, moss=FROST, seed=71, cuts=4),
        rock_part(hb, sp((0, -0.230, 1.650)), (0.36, 0.20, 0.14), STONE, seed=73, cuts=4),
        rock_part(hb, sp((0.085, -0.270, 1.845)), (0.18, 0.12, 0.08), STONE_DARK, moss=FROST, seed=74, cuts=3, rot=(-10, 0, -16)),
        rock_part(hb, sp((-0.085, -0.270, 1.845)), (0.18, 0.12, 0.08), STONE_DARK, moss=FROST, seed=74, cuts=3, rot=(-10, 0, 16), flip=True),
        rock_part(hb, sp((0, -0.305, 1.755)), (0.09, 0.10, 0.13), STONE, seed=75, cuts=3),
        box_part(hb, sp((0, -0.265, 1.795)), (0.26, 0.04, 0.07), "cloth", STONE_DARK, bevel=0.3),
        box_part(hb, sp((0.072, -0.284, 1.808)), (0.07, 0.03, 0.022), "emissive", GLOW, rot=(0, 0, -16)),
        box_part(hb, sp((-0.072, -0.284, 1.808)), (0.07, 0.03, 0.022), "emissive", GLOW, rot=(0, 0, 16)),
        *icicles(hb, [(sp((x, -0.31, 1.62)), L, w) for x, L, w in
                      ((-0.12, 0.10, 0.022), (-0.06, 0.15, 0.026), (0.0, 0.19, 0.03), (0.06, 0.14, 0.026), (0.12, 0.09, 0.02))]),
        lump("neck", sp((0, 0, 1.60)), (0.24, 0.24, 0.12), STONE_DARK),
        # A crown of crystal breaking through the back of his skull.
        *cluster(hb, sp((0.0, 0.04, 1.86)), (0.0, 0.35, 1.0), 0.22, 0.04, seed=1),
    ]


def _chain():
    """The grapple's chain wound round the right fist, its hook hanging from the knuckles."""
    hb = "hand_r"
    return [
        path_part(hb, [("hand_r", 0.15), ("hand_r", 0.32)], 8, [(.175, .17), (.175, .17)], "metal", IRON),
        path_part("forearm_r", [("forearm_r", 0.9), ("forearm_r", 1.0)], 8, [(.215, .205), (.215, .205)], "metal", IRON),
        # Chain links wound up the forearm.
        *[box_part("forearm_r", ("forearm_r", 0.62 + 0.035 * k, (0.21 * math.cos(a), 0.21 * math.sin(a), 0.0)), (0.05, 0.05, 0.03), "metal", IRON,
                   rot=(0, 0, math.degrees(a)), bevel=0.3)
          for k, a in enumerate([2 * math.pi * i / 8 for i in range(8)])],
        path_part(hb, [("fingers_r", 0.9, (0, -0.05, -0.05)), ("fingers_r", 0.9, (0.0, -0.10, -0.20)),
                       ("fingers_r", 0.9, (-0.06, -0.16, -0.24)), ("fingers_r", 0.9, (-0.08, -0.20, -0.17))], 5,
                  [(.025, .025), (.03, .03), (.03, .03), (0, 0)], "metal", IRON),
    ]


def parts():
    return _center() + champion.mirrored(_left()) + _head() + _chain()


# --- Poses ----------------------------------------------------------------------------------

STAND_FEET = feet((0.30, 0.06), (0.30, -0.08))
HUNCH = {"spine_01": (14, 0, 0), "spine_02": (6, 0, 0), "chest": (4, 0, 0), "head": (-18, 0, 0)}


def fists_low(spread=0.62, fwd=0.20, up=0.80):
    """Both fists hanging low in front of his thighs, knuckles forward."""
    return merge(hand("l", at(spread, fwd, up, "l"), aim=at(0.1, 0.3, -1, "l")),
                 hand("r", at(spread, fwd, up, "r"), aim=at(0.1, 0.3, -1, "r")))


IDLE = merge(STAND_FEET, HUNCH, fists_low())
GUARD = merge(feet((0.32, 0.18), (0.32, -0.12)), HUNCH, {"spine_01": (18, 0, 0)},
              hand("l", at(0.30, 0.55, 1.45, "l"), aim=at(-0.2, 0.6, 0.6, "l")),
              hand("r", at(0.34, 0.35, 1.35, "r"), aim=at(-0.2, 0.6, 0.6, "r")))
GUARD_DOWN = {"pelvis": (0, 0, -0.10)}


def idle(armature):
    breathe = merge(IDLE, sym("clavicle", 0, -6, 0), {"chest": (0, 0, 0), "head": (-22, 0, 0)})
    return _clip(armature, "idle", 80, [(0, IDLE, {"pelvis": (0, 0, -0.03)}), (40, breathe, {"pelvis": (0, 0, -0.01)})], loop=True)


def idle_fidget_1(armature):
    # Rolls his great shoulders, one then the other, grinding stone.
    keys = [(0, IDLE, None), (14, merge(IDLE, {"clavicle_l": (0, -14, 0), "chest": (0, 6, 0)}), None),
            (28, merge(IDLE, {"clavicle_r": (0, 14, 0), "chest": (0, -6, 0)}), None), (42, merge(IDLE, sym("clavicle", 0, -10, 0)), {"pelvis": (0, 0, 0.02)}),
            (56, IDLE, None)]
    return _clip(armature, "idle_fidget_1", 56, keys)


def idle_fidget_2(armature):
    # Pounds his fists together twice; each blow lands and holds.
    meet = merge(STAND_FEET, HUNCH, hand("l", at(0.10, 0.62, 1.20, "l"), aim=at(-1, 0.2, 0, "l")),
                 hand("r", at(0.10, 0.62, 1.20, "r"), aim=at(-1, 0.2, 0, "r")))
    apart = merge(STAND_FEET, HUNCH, hand("l", at(0.62, 0.55, 1.30, "l"), aim=at(0.2, 0.6, 0.2, "l")),
                  hand("r", at(0.62, 0.55, 1.30, "r"), aim=at(0.2, 0.6, 0.2, "r")))
    keys = [(0, IDLE, None), (10, apart, None, "whip"), (13, meet, {"pelvis": (0, 0, -0.04)}), (18, meet, {"pelvis": (0, 0, -0.04)}),
            (24, apart, None, "whip"), (27, meet, {"pelvis": (0, 0, -0.05)}), (33, meet, {"pelvis": (0, 0, -0.05)}), (48, IDLE, None)]
    return _clip(armature, "idle_fidget_2", 48, keys)


def idle_ready(armature):
    keys = [(0, GUARD, GUARD_DOWN), (20, merge(GUARD, {"spine_01": (21, 0, 0)}), {"pelvis": (0, 0, -0.12)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(swing):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(arm("l", 0.5, -0.5 * swing * c, -0.75, 0.2, 0.45 - 0.4 * swing * c, -0.65),
                     arm("r", 0.5, 0.5 * swing * c, -0.75, 0.2, 0.45 + 0.4 * swing * c, -0.65),
                     {"head": (-20, 0, 0), "chest": (4, 0, 0)})
    return upper



def walk(armature):
    # A walk (heavy slows, and the moment a run comes to a stop): the run's body type, slower,
    # the weapon carried as in it, never hanging into the ground (14 §3.6).
    keys = gait(32, 0.90, contact=0.6, lift=0.10, bob=0.02, lean=10, twist=6, width=0.31, crouch=0.04, upper=_run_upper(0.4))
    return _clip(armature, "walk", 32, keys, loop=True, markers={"foot_l": 0, "foot_r": 16}, stride_speed=140.0)

def run(armature):
    keys = gait(20, 1.0, contact=0.45, lift=0.18, bob=0.07, lean=16, twist=12, width=0.31, crouch=0.05, upper=_run_upper(1.0))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=335.0)


def run_fast(armature):
    keys = gait(16, 0.96, contact=0.40, lift=0.22, bob=0.08, lean=21, twist=14, width=0.31, crouch=0.06, upper=_run_upper(1.3))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(armature):
    # A right hook: wind back with the whole body, cock, the fist lands in 2 frames on `fire`
    # (sim: 14) and stays there, then a heavy return.
    st = feet((0.32, 0.20), (0.32, -0.16))
    wind = merge(st, HUNCH, {"pelvis": (0, 0, -22), "spine_01": (16, 0, -24), "spine_02": (6, 0, -14), "head": (-18, 0, 20)},
                 hand("r", at(0.75, -0.30, 1.25, "r"), aim=at(0.2, -0.6, 0.3, "r")), hand("l", at(0.35, 0.55, 1.35, "l"), aim=at(-0.2, 0.6, 0.6, "l")))
    cock = merge(wind, {"pelvis": (0, 0, -28), "spine_01": (16, 0, -32)}, hand("r", at(0.70, -0.50, 1.20, "r"), aim=at(0.3, -0.6, 0.2, "r")))
    hit = merge(feet((0.32, 0.34), (0.32, -0.22)), HUNCH, {"pelvis": (0, 0, 18), "spine_01": (20, 0, 26), "spine_02": (6, 0, 16), "head": (-16, 0, -20)},
                hand("r", at(0.05, 1.45, 1.10, "r"), aim=at(-0.1, 1, 0, "r")), hand("l", at(0.55, 0.0, 1.15, "l"), aim=at(0.1, -0.4, 0.4, "l")))
    keys = [(0, GUARD, GUARD_DOWN), (8, wind, {"pelvis": (0, 0.03, -0.12)}), (12, cock, {"pelvis": (0, 0.04, -0.13)}, "snap"),
            (14, hit, {"pelvis": (0, -0.08, -0.15)}), (20, merge(hit, {"spine_01": (22, 0, 28)}), {"pelvis": (0, -0.08, -0.16)}, "settle"),
            (32, GUARD, GUARD_DOWN)]
    return _clip(armature, "attack_1", 32, keys, markers={"fire": 14})


def attack_2(armature):
    # Both fists raised high, held, then hammered down in front on `fire` (sim: 14); they stay
    # in the ground a beat.
    st = feet((0.32, 0.18), (0.32, -0.12))
    up = merge(st, {"spine_01": (-4, 0, 0), "spine_02": (-10, 0, 0), "head": (-10, 0, 0)},
               hand("l", at(0.18, 0.15, 2.35, "l"), aim=at(-0.4, -0.2, 1, "l")), hand("r", at(0.18, 0.15, 2.35, "r"), aim=at(-0.4, -0.2, 1, "r")))
    cock = merge(up, {"spine_02": (-16, 0, 0)}, hand("l", at(0.16, -0.05, 2.3, "l"), aim=at(-0.4, -0.5, 0.8, "l")),
                 hand("r", at(0.16, -0.05, 2.3, "r"), aim=at(-0.4, -0.5, 0.8, "r")))
    hit = merge(feet((0.34, 0.30), (0.34, -0.16)), {"spine_01": (34, 0, 0), "spine_02": (14, 0, 0), "head": (-26, 0, 0)},
                hand("l", at(0.16, 1.20, 0.30, "l"), aim=at(-0.2, 0.5, -1, "l")), hand("r", at(0.16, 1.20, 0.30, "r"), aim=at(-0.2, 0.5, -1, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (7, up, {"pelvis": (0, 0.02, 0.03)}), (11, cock, {"pelvis": (0, 0.03, 0.02)}, "snap"),
            (14, hit, {"pelvis": (0, -0.10, -0.30)}), (21, merge(hit, {"spine_01": (36, 0, 0)}), {"pelvis": (0, -0.10, -0.31)}, "settle"),
            (32, GUARD, GUARD_DOWN)]
    return _clip(armature, "attack_2", 32, keys, markers={"fire": 14})


def q(armature):
    # Grapple: the hook swung back, then hurled down the line on `fire` (sim: 11), the arm
    # flung out after it; then he leans back and hauls the chain in hand over fist.
    st = feet((0.32, 0.22), (0.32, -0.18))
    wind = merge(st, HUNCH, {"pelvis": (0, 0, -18), "spine_01": (10, 0, -22), "head": (-14, 0, 18)},
                 hand("r", at(0.85, -0.55, 1.55, "r"), aim=at(0.3, -0.6, 0.6, "r")), hand("l", at(0.45, 0.6, 1.3, "l"), aim=at(0, 1, 0.2, "l")))
    # The throw: the arm flung all the way out, the hand open at the end of it (the grab flies
    # from it as an open stone hand); then the fists close as he hauls.
    throw = merge(feet((0.32, 0.34), (0.32, -0.22)), HUNCH, {"pelvis": (0, 0, 16), "spine_01": (20, 0, 20), "head": (-16, 0, -14)},
                  hand("r", at(0.15, 1.70, 1.30, "r"), aim=at(0, 1, 0.05, "r")), hand("l", at(0.6, -0.1, 1.1, "l"), aim=at(0.3, -0.5, 0, "l")),
                  {"fingers_r": (-35, 0, 0), "thumb_r": (-30, 0, 0)}, sym("clavicle", 0, 0, 0), {"clavicle_r": (0, 0, 14)})
    haul1 = merge(feet((0.32, 0.30), (0.32, -0.30)), {"spine_01": (-6, 0, 6), "spine_02": (-6, 0, 0), "head": (-6, 0, 0)}, sym("fingers", 75, 0, 0),
                  hand("r", at(0.30, 0.35, 1.20, "r"), aim=at(0, 1, 0.2, "r")), hand("l", at(0.15, 0.95, 1.25, "l"), aim=at(-0.2, 1, 0, "l")))
    haul2 = merge(feet((0.32, 0.30), (0.32, -0.30)), {"spine_01": (-10, 0, -6), "spine_02": (-8, 0, 0), "head": (-4, 0, 0)}, sym("fingers", 75, 0, 0),
                  hand("l", at(0.35, 0.30, 1.20, "l"), aim=at(0, 1, 0.2, "l")), hand("r", at(0.15, 0.95, 1.25, "r"), aim=at(-0.2, 1, 0, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (7, wind, {"pelvis": (0, 0.03, -0.10)}), (9, merge(wind, {"spine_01": (10, 0, -28)}), {"pelvis": (0, 0.04, -0.10)}, "snap"),
            (11, throw, {"pelvis": (0, -0.06, -0.13)}), (17, merge(throw, {"spine_01": (22, 0, 22)}), {"pelvis": (0, -0.06, -0.14)}, "settle"),
            (21, haul1, {"pelvis": (0, 0.08, -0.12)}), (26, haul2, {"pelvis": (0, 0.10, -0.12)}), (34, GUARD, GUARD_DOWN)]
    return _clip(armature, "q", 34, keys, markers={"fire": 11})


def w(armature):
    # Bulwark (upper body, a pulse): forearms slammed together across his face on `fire` 4,
    # held while the shield takes, then lowered.
    cross = merge(HUNCH, {"spine_01": (18, 0, 0)}, arm("l", 0.6, 0.6, -0.4, -0.75, 0.3, 0.6), arm("r", 0.6, 0.6, -0.35, -0.75, 0.3, 0.7),
                  sym("clavicle", 0, 8, 0))
    low = merge(HUNCH, arm("l", 0.5, 0.2, -0.85, 0.2, 0.4, -0.8), arm("r", 0.5, 0.2, -0.85, 0.2, 0.4, -0.8))
    keys = [(0, low, None),
            (2, merge(HUNCH, arm("l", 0.8, 0.2, -0.3, 0.3, 0.3, 0.7), arm("r", 0.8, 0.2, -0.3, 0.3, 0.3, 0.7)), None, "snap"),
            (4, cross, None), (14, merge(cross, {"spine_01": (20, 0, 0)}), None, "settle"), (24, low, None)]
    return _clip(armature, "w", 24, keys, layer="upper", markers={"fire": 4})


def e(armature):
    # Tremor: rears up with both fists high and the right knee raised, then stamps and drives
    # both fists into the ground on `fire` (sim: 8); everything stays planted a beat.
    rear = merge(feet((0.30, 0.10), (0.30, -0.02), lift_r=0.40), {"spine_02": (-12, 0, 0), "head": (-8, 0, 0)},
                 hand("l", at(0.40, 0.20, 2.25, "l"), aim=at(0, -0.2, 1, "l")), hand("r", at(0.40, 0.20, 2.25, "r"), aim=at(0, -0.2, 1, "r")))
    slam = merge(feet((0.36, 0.14), (0.36, -0.10)), {"spine_01": (40, 0, 0), "spine_02": (14, 0, 0), "head": (-30, 0, 0)},
                 hand("l", at(0.55, 0.80, 0.28, "l"), aim=at(0, 0.3, -1, "l")), hand("r", at(0.55, 0.80, 0.28, "r"), aim=at(0, 0.3, -1, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (6, rear, {"pelvis": (0, 0.02, 0.05)}, "snap"), (8, slam, {"pelvis": (0, -0.04, -0.34)}),
            (16, merge(slam, {"spine_01": (42, 0, 0)}), {"pelvis": (0, -0.04, -0.35)}, "settle"), (26, GUARD, GUARD_DOWN)]
    return _clip(armature, "e", 26, keys, markers={"fire": 8})


def r(armature):
    # Upheaval: both fists driven into the ground on `fire` (sim: 8), held, then he heaves
    # upward with his whole body as if lifting the earth that knocks them up.
    st = feet((0.36, 0.16), (0.36, -0.14))
    raise_ = merge(st, {"spine_02": (-14, 0, 0), "head": (-10, 0, 0)},
                   hand("l", at(0.30, 0.0, 2.35, "l"), aim=at(-0.2, -0.3, 1, "l")), hand("r", at(0.30, 0.0, 2.35, "r"), aim=at(-0.2, -0.3, 1, "r")))
    plant = merge(st, {"spine_01": (40, 0, 0), "spine_02": (16, 0, 0), "head": (-30, 0, 0)},
                  hand("l", at(0.45, 0.85, 0.28, "l"), aim=at(0, 0.3, -1, "l")), hand("r", at(0.45, 0.85, 0.28, "r"), aim=at(0, 0.3, -1, "r")))
    heave = merge(st, {"spine_01": (-8, 0, 0), "spine_02": (-16, 0, 0), "head": (-24, 0, 0)},
                  hand("l", at(0.70, 0.40, 2.10, "l"), aim=at(0.4, 0.2, 1, "l")), hand("r", at(0.70, 0.40, 2.10, "r"), aim=at(0.4, 0.2, 1, "r")))
    keys = [(0, GUARD, GUARD_DOWN), (6, raise_, {"pelvis": (0, 0.02, 0.04)}, "snap"), (8, plant, {"pelvis": (0, -0.04, -0.36)}),
            (14, merge(plant, {"spine_01": (42, 0, 0)}), {"pelvis": (0, -0.04, -0.37)}, "whip"), (20, heave, {"pelvis": (0, 0, 0.04)}),
            (24, heave, {"pelvis": (0, 0, 0.04)}, "settle"), (34, GUARD, GUARD_DOWN)]
    return _clip(armature, "r", 34, keys, markers={"fire": 8})


KNEEL = merge({"ik_foot_l": clips.ik(at(0.30, 0.30, champion.ankle(), "l")), "thigh_r": (6, 0, 0), "calf_r": (100, 0, 0), "foot_r": (40, 0, 0)},
              {"spine_01": (26, 0, 0), "head": (6, 0, 0)},
              hand("l", at(0.55, 0.70, 0.28, "l"), aim=at(0, 0.3, -1, "l")), hand("r", at(0.45, 0.70, 0.28, "r"), aim=at(0, 0.3, -1, "r")))


def recall(armature):
    keys = [(0, IDLE, None), (15, KNEEL, {"pelvis": (0, 0.05, -0.44)}), (45, merge(KNEEL, sym("clavicle", 0, -6, 0)), {"pelvis": (0, 0.05, -0.42)}),
            (75, KNEEL, {"pelvis": (0, 0.05, -0.44)}), (90, IDLE, None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    # His legs give way; he drops onto his fists, then topples forward like a felled wall.
    knees = merge({"thigh_l": (0, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (100, 0, 0), "calf_r": (100, 0, 0), "foot_l": (40, 0, 0), "foot_r": (40, 0, 0)},
                  {"spine_01": (30, 0, 0), "head": (-10, 0, 0)},
                  hand("l", at(0.55, 0.65, 0.28, "l"), aim=at(0, 0.3, -1, "l")), hand("r", at(0.55, 0.65, 0.28, "r"), aim=at(0, 0.3, -1, "r")))
    fall = merge({"thigh_l": (-14, 0, 0), "thigh_r": (-10, 0, 0), "calf_l": (24, 0, 0), "calf_r": (20, 0, 0)},
                 arm("l", 0.7, 0.6, 0.1, 0.6, 0.8, 0.0), arm("r", 0.7, 0.6, 0.1, 0.6, 0.8, 0.0), {"pelvis": (82, 0, 0), "head": (-20, 0, 0)})
    keys = [(0, IDLE, None), (8, merge(IDLE, {"spine_01": (-10, 0, 0), "head": (-30, 0, 0)}), {"pelvis": (0, 0.06, 0)}, "settle"),
            (17, knees, {"pelvis": (0, 0, -0.46)}, "snap"), (21, merge(knees, {"spine_01": (34, 0, 0)}), {"pelvis": (0, 0, -0.47)}),
            (30, fall, {"pelvis": (0, -0.70, -0.70)}, "snap"), (36, fall, {"pelvis": (0, -0.70, -0.72)})]
    return _clip(armature, "death", 36, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.05, -0.44)}, "settle"), (14, merge(IDLE, sym("clavicle", 0, -8, 0)), {"pelvis": (0, 0, -0.05)}), (26, IDLE, None)]
    return _clip(armature, "respawn", 26, keys)


def select(armature):
    # Beats his chest with both fists, then throws his arms wide and roars.
    beat_l = merge(STAND_FEET, HUNCH, hand("l", at(0.12, 0.38, 1.50, "l"), aim=at(-1, 0.1, 0, "l")), hand("r", at(0.6, 0.6, 1.3, "r"), aim=at(0, 1, 0.3, "r")))
    beat_r = merge(STAND_FEET, HUNCH, hand("r", at(0.12, 0.38, 1.50, "r"), aim=at(-1, 0.1, 0, "r")), hand("l", at(0.6, 0.6, 1.3, "l"), aim=at(0, 1, 0.3, "l")))
    wide = merge(STAND_FEET, {"spine_02": (-14, 0, 0), "head": (-28, 0, 0)}, arm("l", 1, 0.2, 0.35, 1, 0.3, 0.5), arm("r", 1, 0.2, 0.35, 1, 0.3, 0.5))
    keys = [(0, IDLE, None), (8, beat_l, None, "snap"), (11, beat_r, None, "snap"), (14, beat_l, None, "snap"), (17, beat_r, None),
            (26, wide, {"pelvis": (0, 0, 0.04)}), (40, merge(wide, {"head": (-34, 0, 0)}), {"pelvis": (0, 0, 0.04)}), (54, IDLE, None)]
    return _clip(armature, "select", 54, keys)


def emote_taunt(armature):
    # Pounds the ground in front of him with one fist, twice, staring.
    pound = merge(STAND_FEET, {"spine_01": (34, 0, 0), "head": (-30, 0, 0)}, hand("r", at(0.4, 0.75, 0.28, "r"), aim=at(0, 0.3, -1, "r")),
                  hand("l", at(0.6, 0.2, 0.85, "l"), aim=at(0.1, 0.3, -1, "l")))
    lift = merge(pound, hand("r", at(0.4, 0.55, 1.4, "r"), aim=at(0, 0.3, 1, "r")), {"spine_01": (24, 0, 0)})
    keys = [(0, IDLE, None), (10, lift, None, "whip"), (13, pound, {"pelvis": (0, 0, -0.12)}), (18, pound, {"pelvis": (0, 0, -0.12)}),
            (24, lift, None, "whip"), (27, pound, {"pelvis": (0, 0, -0.12)}), (34, pound, {"pelvis": (0, 0, -0.12)}), (46, IDLE, None)]
    return _clip(armature, "emote_taunt", 46, keys)


def emote_joke(armature):
    # A double-biceps flex; a crystal glints (the pose holds).
    flex = merge(STAND_FEET, {"spine_02": (-6, 0, 0), "head": (-10, 0, 0)}, arm("l", 1, 0.0, 0.1, 0.0, 0.1, 1), arm("r", 1, 0.0, 0.1, 0.0, 0.1, 1))
    keys = [(0, IDLE, None), (12, flex, None), (16, merge(flex, sym("clavicle", 0, -10, 0)), None, "snap"), (30, merge(flex, sym("clavicle", 0, -10, 0)), None),
            (34, flex, None), (50, IDLE, None)]
    return _clip(armature, "emote_joke", 50, keys)


def emote_laugh(armature):
    keys = [(0, IDLE, None)]
    for i in range(1, 9):
        s = 1.0 if i % 2 else 0.5
        keys.append((4 * i, merge(IDLE, {"spine_01": (6 * s, 0, 0), "head": (-30 * s, 0, 0)}, sym("clavicle", 0, -8 * s, 0)), {"pelvis": (0, 0, -0.02 * s)}))
    keys.append((44, IDLE, None))
    return _clip(armature, "emote_laugh", 44, keys)


def emote_dance(armature):
    # A slow, ground-shaking side-to-side stomp, fists swinging.
    a = merge(feet((0.30, 0.06), (0.30, -0.08), lift_l=0.32), HUNCH, {"pelvis": (0, -8, 0)}, fists_low(0.7, 0.3, 0.9))
    b = merge(feet((0.30, 0.06), (0.30, -0.08), lift_r=0.32), HUNCH, {"pelvis": (0, 8, 0)}, fists_low(0.7, 0.3, 0.9))
    land = merge(STAND_FEET, HUNCH, fists_low(0.66, 0.25, 0.8))
    keys = [(0, a, {"pelvis": (0.05, 0, 0.0)}, "whip"), (10, land, {"pelvis": (0, 0, -0.08)}), (24, b, {"pelvis": (-0.05, 0, 0.0)}, "whip"),
            (34, land, {"pelvis": (0, 0, -0.08)})]
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, walk, run, run_fast, attack_1, attack_2, q, w, e, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def build(only=None):
    return champion.build("bastion", SHAPE, parts, OWN, 2.2, only=only)


if __name__ == "__main__":
    champion.main(build)
