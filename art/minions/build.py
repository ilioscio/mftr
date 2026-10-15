# SPDX-License-Identifier: AGPL-3.0-or-later
"""Lane minions (A5): the block-out generator for the four minion packs.

    blender -b --python art/minions/build.py -- --kind melee --out art/minions/melee/melee.blend

Tiny hooded figures, distinct from champions at a glance: chibi proportions on the `biped`
skeleton (`rig.use_shape("minion")`: the same bones, a big head on short legs, ~1.05 m), a
team-colored robe and pointed hood (`accent`), and no face, only a shadowed void with two eyes
glowing in the team color (`accent_glow`). Squat (15 §6): short legs, a wide body, ~0.95 m to the
hood's tip. Four kinds, as the sim has them:

    melee   sword and buckler           caster  a staff with a glowing orb
    siege   a gunner pushing a wheeled  super   a 1.6x armored brute with a maul
            bronze cannon (extra bones)

Clips: idle, run, attack_1 (`fire` on the sim's windup), death and an additive flinch. Like the
other block-outs this is a starting point: run once, then polish the .blend.
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

from mftr_blender import clips, mesh, rig  # noqa: E402
from mftr_blender.clips import grip, ik, merge, sym  # noqa: E402
from mftr_blender.library import _clip, arms, cycle, run_half  # noqa: E402

# Per kind: overall scale, extra width, attack timing (frames at 30 fps; `fire` = the sim's
# windup: windup_fraction / attack_speed, lane.rs minion_attack).
KINDS = {
    "melee": dict(scale=1.0, wide=1.0, attack=24, fire=7),
    "caster": dict(scale=1.0, wide=1.0, attack=30, fire=13),
    "siege": dict(scale=0.95, wide=1.0, attack=30, fire=9),
    "super": dict(scale=1.6, wide=1.3, attack=34, fire=11),
}

# Palette. Robe and hood are the `accent` slot: the client paints them in the team color, and the
# vertex color only sets the shade (folds, hem, the hood's rim).
ROBE = (0.80, 0.80, 0.80)
HOOD = (0.95, 0.95, 0.95)
HEM = (0.45, 0.45, 0.45)
RIM = (0.30, 0.30, 0.30)
MANTLE = (0.62, 0.62, 0.62)
VOID = (0.015, 0.015, 0.02)
GLOW = (1.0, 1.0, 1.0)
GLOVE = (0.17, 0.13, 0.11)
PANTS = (0.14, 0.13, 0.15)
BOOT = (0.24, 0.16, 0.10)
LEATHER = (0.42, 0.27, 0.15)
METAL = (0.72, 0.74, 0.78)
IRON = (0.34, 0.35, 0.39)
WOOD = (0.42, 0.26, 0.12)
BRONZE = (0.74, 0.52, 0.24)
HORN = (0.80, 0.74, 0.60)

S = 1.0   # the current kind's scale and width (set by `build`)
W = 1.0


# The parts below are authored for the first, taller minion; the squat one (15 §6) stands on the
# same bones remapped (`rig.SHAPES["minion"]`), so authored heights go through the same remap
# and widths widen with the body.
_OLD_Z = [0.0, 0.02, 0.05, 0.22, 0.40, 0.46, 0.60, 0.68, 0.71, 0.98, 1.20]
_NEW_Z = [0.0, 0.02, 0.045, 0.17, 0.32, 0.38, 0.53, 0.62, 0.66, 0.90, 1.08]
_WIDEN = (0.84 / 0.72, 0.84 / 0.75)


def squat_z(z):
    for i in range(len(_OLD_Z) - 1):
        if z <= _OLD_Z[i + 1] or i == len(_OLD_Z) - 2:
            a, b = _OLD_Z[i], _OLD_Z[i + 1]
            t = (z - a) / (b - a)
            return _NEW_Z[i] + t * (_NEW_Z[i + 1] - _NEW_Z[i])
    return z


# The siege cart is a vehicle on the ground, not part of the body: it isn't squashed.
RAW = False


def pt(p):
    """A point authored for a 1.0-scale minion, placed for the current kind (bone refs pass)."""
    if isinstance(p[0], str):
        return p
    if RAW:
        return (p[0] * S * W, p[1] * S, p[2] * S)
    return (p[0] * _WIDEN[0] * S * W, p[1] * _WIDEN[1] * S, squat_z(p[2]) * S)


def part(bone, a, b, sides, prof, slot, color):
    # Round parts get half again as many sides (D60's larger budget): rounder robes and hoods.
    sides = sides + sides // 2 if sides >= 8 else sides
    wx, wy = (1.0, 1.0) if RAW else _WIDEN
    return (bone, pt(a), pt(b), sides, [(t, ru * wx * S * W, rv * wy * S) for t, ru, rv in prof], slot, color)


# --- Mesh -----------------------------------------------------------------------------------

def body(hood_tip=1.10, hood_back=0.10):
    c = [
        # The robe: a skirt over the hips flaring to a darker hem, the torso, a mantle.
        part("pelvis", (0, 0, 0.47), (0, 0, 0.13), 12, [(0, .10, .085), (.45, .125, .105), (1, .15, .13)], "accent", ROBE),
        part("pelvis", (0, 0, 0.14), (0, 0, 0.11), 12, [(0, .152, .132), (1, .155, .135)], "accent", HEM),
        part("spine_01", (0, 0, 0.44), (0, 0, 0.54), 12, [(0, .098, .082), (1, .10, .084)], "accent", ROBE),
        part("chest", (0, 0, 0.53), (0, 0, 0.70), 12, [(0, .10, .084), (.5, .11, .088), (.85, .09, .075), (1, .06, .05)], "accent", ROBE),
        part("chest", (0, 0.005, 0.62), (0, 0.005, 0.71), 12, [(0, .135, .105), (.6, .12, .095), (1, .075, .065)], "accent", MANTLE),
        part("pelvis", (0, 0, 0.445), (0, 0, 0.485), 12, [(0, .108, .09), (1, .108, .09)], "cloth", LEATHER),
        part("pelvis", (0, -0.088, 0.465), (0, -0.104, 0.465), 4, [(0, .022, .018), (1, .02, .016)], "metal", METAL),
        # The hood: big and pointed, its tip falling back.
        part("head", (0, 0.0, 0.665), (0, hood_back, hood_tip), 12,
             [(0, .10, .095), (.12, .15, .14), (.33, .162, .152), (.55, .135, .128), (.75, .075, .072), (.9, .035, .035), (1, 0, 0)],
             "accent", HOOD),
        # No face: a darker rim, a void, and two eyes glowing in the team color.
        part("head", (0, -0.09, 0.805), (0, -0.128, 0.805), 12, [(0, .108, .10), (1, .104, .096)], "accent", RIM),
        part("head", (0, -0.095, 0.80), (0, -0.135, 0.80), 10, [(0, .088, .078), (1, .082, .072)], "cloth", VOID),
    ]
    for x in (0.032, -0.032):
        c.append(part("head", (x, -0.128, 0.815), (x, -0.146, 0.815), 4, [(0, .017, .012), (1, .015, .011)], "accent_glow", GLOW))
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    left = [
        part(ua, (ua, 0), (ua, 1), 8, [(0, .05, .05), (1, .048, .048)], "accent", ROBE),
        part(fa, (fa, 0), (fa, .9), 8, [(0, .05, .05), (.6, .058, .058), (1, .07, .07)], "accent", ROBE),
        part(fa, (fa, .82), (fa, .93), 8, [(0, .072, .072), (1, .072, .072)], "accent", HEM),
        part("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .034, .028), (1, .036, .026)], "cloth", GLOVE),
        part("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .034, .024), (1, .022, .016)], "cloth", GLOVE),
        part(th, (th, 0), (th, 1), 8, [(0, .055, .055), (1, .045, .045)], "cloth", PANTS),
        part(ca, (ca, 0), (ca, 1), 8, [(0, .045, .045), (.5, .05, .05), (1, .042, .042)], "cloth", BOOT),
        part("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .045, .04), (1, .042, .03)], "cloth", BOOT),
        part("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .04, .028), (1, .03, .018)], "cloth", BOOT),
    ]
    return c + left + [mesh._mirror_part(p) for p in left]


def melee_gear():
    # A short sword in the right hand (along the prop axis; skinned to the hand, props don't
    # deform), a round buckler on the left forearm.
    return [
        part("hand_r", (-0.439, 0.03, 0.435), (-0.439, -0.03, 0.435), 6, [(0, .014, .014), (1, .014, .014)], "cloth", LEATHER),
        part("hand_r", (-0.40, -0.035, 0.435), (-0.478, -0.035, 0.435), 4, [(0, .014, .012), (1, .014, .012)], "metal", IRON),
        part("hand_r", (-0.439, -0.04, 0.435), (-0.439, -0.34, 0.435), 4, [(0, .022, .007), (.85, .02, .006), (1, 0, 0)], "metal", METAL),
        part("forearm_l", (0.35, -0.05, 0.53), (0.35, -0.08, 0.53), 10, [(0, .092, .092), (1, .088, .088)], "cloth", WOOD),
        part("forearm_l", (0.35, -0.052, 0.53), (0.35, -0.077, 0.53), 10, [(0, .1, .1), (1, .1, .1)], "metal", IRON),
        part("forearm_l", (0.35, -0.08, 0.53), (0.35, -0.10, 0.53), 6, [(0, .03, .03), (1, 0, 0)], "metal", METAL),
    ]


def caster_gear():
    # A tall staff held upright, topped by an orb glowing in the team color in a bronze cup.
    return [
        part("hand_r", (-0.439, -0.02, 0.10), (-0.439, -0.02, 0.86), 6, [(0, .012, .012), (1, .016, .016)], "cloth", WOOD),
        part("hand_r", (-0.439, -0.02, 0.84), (-0.439, -0.02, 0.89), 8, [(0, .02, .02), (1, .04, .04)], "metal", BRONZE),
        part("hand_r", (-0.439, -0.02, 0.87), (-0.439, -0.02, 0.99), 8, [(0, 0, 0), (.5, .05, .05), (1, 0, 0)], "accent_glow", GLOW),
    ]


# The siege cart: a carriage bone, a recoiling barrel, two wheels (extra bones, 10 §7.2).
CART = [(0, -0.15, 0.25), (0, -0.55, 0.25)]
BARREL = [(0, -0.22, 0.38), (0, -0.78, 0.42)]
WHEEL_L = [(0.17, -0.36, 0.17), (0.24, -0.36, 0.17)]
WHEEL_R = [(-0.17, -0.36, 0.17), (-0.24, -0.36, 0.17)]


def siege_gear():
    global RAW
    RAW = True
    g = [
        part("extra_cart_1", (0, -0.13, 0.25), (0, -0.57, 0.25), 4, [(0, .21, .07), (1, .21, .07)], "cloth", WOOD),
        part("extra_cart_1", (0.20, -0.36, 0.17), (-0.20, -0.36, 0.17), 6, [(0, .018, .018), (1, .018, .018)], "metal", IRON),
        part("extra_cart_1", (0, -0.20, 0.31), (0, -0.52, 0.33), 4, [(0, .12, .08), (1, .12, .08)], "cloth", WOOD),
        part("extra_cart_1", (0.12, -0.50, 0.30), (0.12, -0.50, 0.80), 6, [(0, .01, .01), (1, .008, .008)], "cloth", WOOD),
        part("extra_cart_1", (0.12, -0.50, 0.78), (0.12, -0.36, 0.72), 4, [(0, .008, .055), (1, 0, 0)], "accent", ROBE),
        part("extra_barrel_1", (0, -0.22, 0.38), (0, -0.78, 0.42), 10,
             [(0, .075, .075), (.1, .088, .088), (.18, .07, .07), (.85, .058, .058), (.9, .07, .07), (1, .07, .07)], "metal", BRONZE),
        part("extra_barrel_1", (0, -0.15, 0.375), (0, -0.23, 0.38), 8, [(0, 0, 0), (.5, .04, .04), (1, .05, .05)], "metal", BRONZE),
        part("extra_barrel_1", (0, -0.775, 0.42), (0, -0.79, 0.42), 8, [(0, .042, .042), (1, .042, .042)], "cloth", VOID),
    ]
    for x in (0.11, -0.11):
        g.append(part("extra_cart_1", (x, -0.15, 0.27), (x * 1.1, -0.02, 0.40), 6, [(0, .015, .015), (1, .014, .014)], "cloth", WOOD))
    for bone, sgn in (("extra_wheel_l_1", 1), ("extra_wheel_r_1", -1)):
        g += [
            part(bone, (0.165 * sgn, -0.36, 0.17), (0.225 * sgn, -0.36, 0.17), 8, [(0, .15, .15), (1, .15, .15)], "cloth", WOOD),
            part(bone, (0.175 * sgn, -0.36, 0.17), (0.215 * sgn, -0.36, 0.17), 8, [(0, .168, .168), (1, .168, .168)], "metal", IRON),
            part(bone, (0.225 * sgn, -0.36, 0.17), (0.245 * sgn, -0.36, 0.17), 6, [(0, .04, .04), (1, .03, .03)], "metal", IRON),
        ]
    RAW = False
    return g


# The super minion's maul, in meters at its built size (`hand_r` runs ~0.14 m): the fist closes
# around the haft at GRIP along `hand_r`, the head's center HEAD_AT along it.
MAUL_GRIP = 0.08
MAUL_HEAD_AT = MAUL_GRIP + 0.80


def _hand_r():
    b = next(b for b in rig.bone_table() if b[0] == "hand_r")
    return Vector(b[2]), Vector(b[3])


def maul():
    """The great maul, skinned to the right hand and laid along it (the haft runs through the
    fist), so a pose aims the whole maul by aiming `hand_r` or by an IK target (`maul_at`)."""
    h, t = _hand_r()
    axis = (t - h).normalized()
    g = h + axis * MAUL_GRIP
    c = h + axis * MAUL_HEAD_AT
    across = axis.cross(Vector((0, 1, 0))).normalized()

    def p(v):
        return tuple(v)

    hb = "hand_r"
    return [
        # Pommel, haft, a leather wrap under the fist, an iron collar under the head.
        mesh.path_part(hb, [p(g - axis * 0.16), p(g - axis * 0.12)], 6, [(.035, .035), (.035, .035)], "metal", IRON, ref=across),
        mesh.path_part(hb, [p(g - axis * 0.13), p(c)], 6, [(.026, .026), (.03, .03)], "cloth", WOOD, ref=across),
        mesh.path_part(hb, [p(g - axis * 0.08), p(g + axis * 0.10)], 6, [(.033, .033), (.033, .033)], "cloth", LEATHER, ref=across),
        mesh.path_part(hb, [p(c - axis * 0.22), p(c - axis * 0.15)], 6, [(.04, .04), (.05, .05)], "metal", IRON, ref=across),
        # The head: an iron block across the haft, steel-faced at both ends.
        mesh.path_part(hb, [p(c - across * 0.17), p(c + across * 0.17)], 4, [(.19, .19), (.19, .19)], "metal", IRON, ref=axis),
        mesh.path_part(hb, [p(c - across * 0.21), p(c - across * 0.17)], 4, [(.15, .15), (.15, .15)], "metal", METAL, ref=axis),
        mesh.path_part(hb, [p(c + across * 0.17), p(c + across * 0.21)], 4, [(.15, .15), (.15, .15)], "metal", METAL, ref=axis),
    ]


def maul_at(head, toward, left=None):
    """The maul's head center at `head` (armature space: X left, -Y forward, Z up), the haft
    running from it toward `toward`, the right fist on it at MAUL_GRIP; with `left`, the left hand
    on the haft that many meters past the right toward the pommel (both hands on it)."""
    hp, tw = Vector(head), Vector(toward)
    axis = (hp - tw).normalized()
    out = {"ik_hand_r": ik(tuple(hp - axis * MAUL_HEAD_AT), aim=tuple(axis))}
    if left is not None:
        out["grip_l"] = grip("hand_r", at=MAUL_GRIP - left, pole=(1.0, 0.2, -1.0))
    return out


# Where the maul is: on the right shoulder, head behind it (idle, run); raised high behind the
# head in both hands (windup); slammed into the ground in front (`fire`).
MAUL_SHOULDER = maul_at((-0.40, 0.32, 1.30), (-0.30, -0.42, 0.72))
MAUL_RAISED = maul_at((-0.10, 0.45, 1.95), (-0.12, -0.12, 1.40), left=0.16)
MAUL_SLAM = maul_at((-0.12, -1.00, 0.19), (-0.12, -0.30, 0.78), left=0.16)
# In-betweens that keep the swing in the plane the minion faces, not out to its side: lifted
# up off the shoulder, upright overhead on the way down, carried upright back to the shoulder.
MAUL_LIFT = maul_at((-0.30, 0.18, 1.85), (-0.26, -0.22, 1.05), left=0.16)
MAUL_OVER = maul_at((-0.12, -0.45, 1.95), (-0.12, -0.22, 1.15), left=0.16)
MAUL_CARRY = maul_at((-0.30, -0.30, 1.60), (-0.28, -0.25, 0.75))


def super_gear():
    # Armor over the robe (chest plate, pauldrons, a horned crown around the hood, gauntlets) and
    # a great maul in the right hand.
    g = [
        part("chest", (0, -0.03, 0.55), (0, -0.035, 0.69), 8, [(0, .10, .07), (.5, .115, .08), (1, .09, .06)], "metal", IRON),
        part("head", (0, 0.005, 0.69), (0, 0.01, 0.75), 12, [(0, .14, .135), (1, .135, .13)], "metal", IRON),
    ] + maul()
    left = [
        part("upperarm_l", (0.09, 0.0, 0.69), (0.21, 0.0, 0.72), 8, [(0, .055, .055), (.45, .085, .08), (1, 0, 0)], "metal", IRON),
        part("forearm_l", ("forearm_l", .55), ("forearm_l", .95), 8, [(0, .062, .062), (1, .07, .07)], "metal", IRON),
        part("head", (0.11, 0.02, 0.86), (0.24, 0.06, 1.02), 6, [(0, .035, .035), (.6, .022, .022), (1, 0, 0)], "cloth", HORN),
    ]
    return g + left + [mesh._mirror_part(p) for p in left]


# --- Clips ----------------------------------------------------------------------------------

STAND = merge(sym("thigh", -3, 0, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"spine_01": (2, 0, 0)})


def L(locs):
    """Pelvis offsets authored for a 1.9 m biped, scaled to this (squat) minion."""
    return {k: (x * S * 0.5, y * S * 0.5, z * S * 0.45) for k, (x, y, z) in (locs or {}).items()} if locs else None


def hold(kind):
    """What the arms do when nothing else does: sword ready, staff planted, hands on the cart's
    handles, maul over the shoulder."""
    return {
        "melee": arms((0.35, 0.35, -1), (0.1, 1, -0.3), (0.6, 0.15, -0.8), (0.8, 0.35, -0.5)),
        "caster": arms((0.3, 0.05, -1), (0.2, 0.25, -1), (0.65, 0.15, -0.75), (0.85, 0.15, -0.5)),
        "siege": arms((0.15, 0.7, -1), (0.0, 1, -0.6)),
        "super": merge(arms((0.35, 0.0, -1), (0.25, 0.3, -1)), MAUL_SHOULDER),
    }[kind]


def wheels(turn):
    return {"extra_wheel_l_1": (turn, 0, 0), "extra_wheel_r_1": (turn, 0, 0)}


def idle(arm, kind):
    a = merge(STAND, hold(kind), {"head": (-2, 0, 0)})
    b = merge(STAND, hold(kind), sym("clavicle", 0, -4, 0), {"head": (-5, 0, 4), "chest": (-3, 0, 0)})
    return _clip(arm, "idle", 48, [(0, a, L({"pelvis": (0, 0, -0.01)})), (24, b, L({"pelvis": (0, 0, 0.01)}))], loop=True)


def run(arm, kind):
    # Short legs, quick steps: the champion run's poses, the arms replaced by the kind's hold
    # (a minion never pumps a sword arm). Lane speed is 325 u/s.
    keys = []
    for f, pose, locs in cycle(run_half(), 20):
        keep = {k: v for k, v in pose.items() if not k.startswith(("upperarm", "forearm"))}
        extra = hold(kind) if kind != "melee" else merge(hold(kind), arms((0.3, 0.3 if f < 10 else -0.2, -1), (0.2, 0.7, -0.5)))
        if kind == "siege":
            extra = merge(extra, wheels(-120 * ((f // 7) if f < 20 else 3)))
        keys.append((f, merge(keep, extra), L(locs)))
    if kind == "siege":
        # A full wheel turn per loop, keyed every 120 degrees so it interpolates the right way.
        keys = [(f, merge(p, wheels(-120 * i)), l) for i, (f, p, l) in zip(range(4), [(0, keys[0][1], keys[0][2]),
                (7, keys[1][1], keys[1][2]), (13, keys[3][1], keys[3][2]), (20, keys[0][1], keys[0][2])])]
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=325.0)


def attack_1(arm, kind, frames, fire):
    st = STAND
    if kind == "melee":
        # Wind up overhead, chop down on `fire`, recover.
        up = merge(st, hold("melee"), arms((0.3, 0.1, -1), (0.2, 0.6, -0.6), (0.3, 0.2, 1), (0.2, -0.3, 1)), {"spine_02": (-8, 0, 10)})
        hit = merge(st, arms((0.3, 0.4, -1), (0.2, 0.8, -0.4), (0.15, 1, -0.2), (0.0, 1, -0.6)), {"spine_02": (14, 0, -10), "head": (-6, 0, 0)},
                    sym("thigh", -10, 0, 0))
        keys = [(0, merge(st, hold("melee")), None), (fire - 3, up, None), (fire, hit, L({"pelvis": (0, -0.03, -0.03)})),
                (fire + 6, merge(hit, {"spine_02": (8, 0, -6)}), None), (frames, merge(st, hold("melee")), None)]
    elif kind == "caster":
        # The staff stays planted; the free hand gathers the spell back by the shoulder and
        # throws it forward on `fire`.
        staff = hold("caster")
        up = merge(st, staff, arms((0.45, -0.3, 0.3), (0.1, 0.4, 1), (0.65, 0.15, -0.75), (0.85, 0.15, -0.5)),
                   {"spine_02": (-6, 0, 14), "head": (-6, 0, 4)})
        thrust = merge(st, staff, arms((0.1, 1, 0.15), (0.05, 1, 0.25), (0.65, 0.15, -0.75), (0.85, 0.15, -0.5)),
                       {"spine_02": (10, 0, -12), "head": (2, 0, -4)})
        keys = [(0, merge(st, hold("caster")), None), (fire - 5, up, None), (fire, thrust, L({"pelvis": (0, -0.03, 0)})),
                (fire + 8, merge(thrust, {"spine_02": (6, 0, -2)}), None), (frames, merge(st, hold("caster")), None)]
    elif kind == "siege":
        # Brace, light the fuse, the barrel kicks back on `fire`.
        brace = merge(st, hold("siege"), sym("thigh", -18, 0, 0), sym("calf", 30, 0, 0), {"spine_01": (14, 0, 0), "head": (6, 0, 0)})
        keys = [(0, merge(st, hold("siege")), None), (fire - 4, brace, L({"pelvis": (0, 0, -0.05)})),
                (fire, merge(brace, {"head": (-12, 0, 0)}), L({"pelvis": (0, 0.02, -0.05)})),
                (fire + 3, merge(brace, {"head": (-14, 0, 0), "spine_01": (4, 0, 0)}), L({"pelvis": (0, 0.05, -0.04)})),
                (frames, merge(st, hold("siege")), None)]
        barrel = {fire - 1: 0.0, fire: 0.0, fire + 2: 0.12, fire + 10: 0.0}
        keys = [(f, p, merge(l or {}, {"extra_barrel_1": (0, barrel.get(f, 0.0) * S, 0)})) for f, p, l in keys]
    else:
        # The super minion: the maul raised high behind the head in both hands, a ground-shaking
        # slam on `fire`.
        up = merge(st, MAUL_RAISED, {"spine_02": (-14, 0, 8), "head": (-8, 0, 0)}, sym("thigh", -6, 0, 0))
        slam = merge(st, MAUL_SLAM, {"spine_01": (16, 0, 0), "spine_02": (14, 0, -8)}, sym("thigh", -24, 0, 0), sym("calf", 30, 0, 0))
        over = merge(st, MAUL_OVER, {"spine_01": (6, 0, 0), "spine_02": (2, 0, 0)})
        keys = [(0, merge(st, hold("super")), None), (fire - 8, merge(st, MAUL_LIFT, {"spine_02": (-6, 0, 4)}), None),
                (fire - 5, up, L({"pelvis": (0, 0.02, 0.02)})), (fire - 2, over, L({"pelvis": (0, -0.01, -0.03)})),
                (fire, slam, L({"pelvis": (0, -0.04, -0.10)})), (fire + 10, merge(slam, {"spine_01": (10, 0, 0)}), L({"pelvis": (0, -0.03, -0.07)})),
                (fire + 17, merge(st, hold("super"), MAUL_CARRY), None), (frames, merge(st, hold("super")), None)]
    return _clip(arm, "attack_1", frames, keys, markers={"fire": fire})


def death(arm, kind):
    # A stagger and a crumple backward; the last frame holds. The siege cart tips over.
    keys = [
        (0, merge(STAND, hold(kind)), None),
        (5, merge(arms((0.7, -0.2, 0.2), (0.6, 0.3, 0.5)), {"spine_01": (-14, 0, 0), "head": (-22, 0, 0)}), L({"pelvis": (0, 0.05, 0)})),
        (12, merge(arms((0.6, 0.3, -0.4), (0.4, 0.6, -0.4)), sym("thigh", -45, 0, 0), sym("calf", 85, 0, 0), {"spine_01": (12, 0, 0), "head": (10, 0, 0)}),
         L({"pelvis": (0, 0.08, -0.38)})),
        (24, merge(arms((0.9, -0.5, 0.1), (0.9, -0.3, 0.0)), sym("thigh", -12, 0, 0), sym("calf", 14, 0, 0),
                   {"pelvis": (-86, 0, 0), "head": (-8, 0, 12)}), L({"pelvis": (0, 0.55, -0.86)})),
    ]
    if kind == "siege":
        keys = [(f, merge(p, {"extra_cart_1": (0, -28 * min(1.0, f / 12), 0)}), l) for f, p, l in keys]
    if kind == "super":
        # The maul flies up as it staggers, drops, and lies on the ground beside it.
        hand = {5: (0.2, -0.5, 0.85), 12: (0.5, 0.4, -0.75), 24: (0.15, 1.0, -0.05)}
        keys = [(f, merge(p, {"hand_r": clips.aim(*hand[f], side="r")}) if f in hand else p, l) for f, p, l in keys]
    return _clip(arm, "death", 24, keys)


def flinch(arm, kind):
    # Additive: a quick recoil from a hit (minions flinch, champions don't, 10 §5.4).
    back = {"spine_01": (-10, 0, 0), "spine_02": (-6, 0, 0), "head": (-12, 0, 0)}
    return _clip(arm, "flinch", 8, [(0, {}, None), (2, back, None), (8, {}, None)], layer="additive",
                 bones=["spine_01", "spine_02", "head"])


# --- Build ----------------------------------------------------------------------------------

def build(kind):
    global S, W, RAW
    cfg = KINDS[kind]
    S, W = cfg["scale"], cfg["wide"]
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig.use_shape("minion", scale=S, x=W)
    mesh.AO_HEIGHT = 1.1 * S
    arm = rig.build_armature(name=f"minion_{kind}")
    parts = body(hood_tip=1.16 if kind == "caster" else 1.10, hood_back=0.14 if kind == "caster" else 0.10)
    if kind == "siege":
        RAW = True
        rig.add_chain(arm, "cart", "root", [pt(p) for p in CART])
        rig.add_chain(arm, "barrel", "extra_cart_1", [pt(p) for p in BARREL])
        rig.add_chain(arm, "wheel_l", "extra_cart_1", [pt(p) for p in WHEEL_L])
        rig.add_chain(arm, "wheel_r", "extra_cart_1", [pt(p) for p in WHEEL_R])
        RAW = False
        parts += siege_gear()
    else:
        parts += {"melee": melee_gear, "caster": caster_gear, "super": super_gear}[kind]()
    made = [idle(arm, kind), run(arm, kind), attack_1(arm, kind, cfg["attack"], cfg["fire"]), death(arm, kind), flinch(arm, kind)]
    body_obj = mesh.build_mannequin(arm, name=kind, part_list=parts)
    print(f"minion {kind}: {len(arm.data.bones)} bones, {mesh.triangle_count(body_obj)} triangles, clips {[a.name for a in made]}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    kind = argv[argv.index("--kind") + 1]
    build(kind)
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
