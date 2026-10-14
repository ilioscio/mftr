# SPDX-License-Identifier: AGPL-3.0-or-later
"""Vesper (rebuilt for 14): the generator for her model and clips.

    blender -b --python art/champions/vesper/build.py -- --out art/champions/vesper/vesper.blend

A huge recurve bow and a long one-sided cloak (14 §5): a ranger in athletic heroic proportions,
the bow taller than her shoulders with teal-glowing tips, a quiver of teal-fletched arrows rising
over her right shoulder, a forest-green cloak hanging off her left shoulder on an `extra_cape`
chain, an auburn ponytail. Her signature is teal (14 §4).

Weight (14 §3): each shot is a nock, a draw, a held full draw, a one-frame release on `fire` and
a recoil that holds; the cloak trails her runs and swings on the release. Feet planted by IK.
Her left hand holds the bow; its orientation comes from the full-draw pose (`bow_frame`), so it
stands vertical with its belly toward her when she shoots.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.normpath(os.path.join(HERE, "..", "..", "..", "tools", "blender")))

import bpy  # noqa: E402
from mathutils import Matrix, Quaternion, Vector  # noqa: E402

from mftr_blender import champion, clips, head, rig  # noqa: E402
from mftr_blender.champion import arm, feet, hand  # noqa: E402
from mftr_blender.clips import at, merge, sym  # noqa: E402
from mftr_blender.library import _clip, gait  # noqa: E402
from mftr_blender.mesh import box_part, path_part  # noqa: E402

SKIN = (0.80, 0.60, 0.48)
FOREST = (0.10, 0.24, 0.17)
FOREST_DARK = (0.06, 0.14, 0.10)
CREAM = (0.70, 0.64, 0.50)
LEATHER = (0.36, 0.22, 0.12)
LEATHER_DARK = (0.20, 0.12, 0.07)
WOOD = (0.30, 0.17, 0.08)
SILVER = (0.55, 0.58, 0.62)
STRING = (0.80, 0.76, 0.64)
TEAL = (0.20, 0.95, 0.85)
TEAL_CLOTH = (0.06, 0.42, 0.40)
HAIR = (0.46, 0.18, 0.09)
ACCENT = (0.18, 0.52, 0.95)

# Athletic heroic proportions (14 §2.2): the standard heights, slightly broader, big hands.
SHAPE = {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 1.05, "y": 1.0, "scale": 0.98, "shoulder": 0.02, "hand": 1.35}
rig.use_shape(SHAPE)
CAPE = [rig.shaped(p) for p in [(0.10, 0.14, 1.52), (0.14, 0.20, 1.18), (0.16, 0.24, 0.84), (0.17, 0.27, 0.50)]]
ARM = None  # set while building, for the bow's frame
# The bow (14 §3.3): a 1.7 m recurve on `hand_l`, rigged so it draws. `BOW_REST` (grip, axis,
# back) is its rest-pose frame, set in `_extras`; each limb is two bones (`extra_bow_1..4`) that
# bend toward her as the string comes back, and the string is two bones (`extra_string_1..2`)
# from the tips to the nocking point that turn and stretch to follow her drawing fingers.
BOW_REST = None
HALF = 0.85          # grip to tip, m
LIMB_SPLIT = 0.45    # where each limb's inner bone ends
BRACE, RECURVE = 0.20, 0.05
DRAWS = {}           # shot clip -> its `fire` frame (the string is baked onto these)


def sp(p):
    return rig.shaped(p)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        # Hips, a leather corset over a cream shirt, a narrow waist and a shaped bust.
        ("pelvis", sp((0, 0, 0.92)), sp((0, 0, 1.12)), 10, [(0, .14, .10), (.4, .175, .115), (.8, .18, .115), (1, .155, .105)], "cloth", FOREST_DARK),
        ("pelvis", sp((0, 0, 1.05)), sp((0, 0, 1.11)), 10, [(0, .168, .114), (1, .15, .106)], "accent", ACCENT),
        ("spine_01", sp((0, 0, 1.10)), sp((0, 0, 1.28)), 10, [(0, .138, .098), (.5, .122, .09), (1, .13, .094)], "cloth", LEATHER),
        ("spine_02", sp((0, 0, 1.26)), sp((0, 0, 1.42)), 10, [(0, .132, .094), (1, .155, .108)], "cloth", LEATHER),
        ("chest", sp((0, 0, 1.40)), sp((0, 0, 1.60)), 10, [(0, .155, .108), (.35, .163, .114), (.7, .155, .108), (1, .09, .075)], "cloth", CREAM),
        ("chest", sp((0, -0.03, 1.415)), sp((0, -0.036, 1.53)), 10, [(0, .115, .058), (.5, .14, .075), (1, .11, .05)], "cloth", CREAM),
        ("neck", sp((0, 0, 1.55)), sp((0, -0.01, 1.66)), 8, [(0, .055, .055), (1, .05, .05)], "skin", SKIN),
        # The cowl (her hood, down) around her neck and over her shoulders, forest green.
        ("chest", sp((0, 0.02, 1.50)), sp((0, 0.02, 1.62)), 10, [(0, .20, .15), (.6, .15, .12), (1, .10, .09)], "cloth", FOREST),
        # A teal-trimmed belt sash.
        box_part("pelvis", sp((0.10, -0.12, 0.98)), (0.10, 0.03, 0.20), "cloth", TEAL_CLOTH, rot=(0, 15, 0)),
        box_part("pelvis", sp((-0.16, -0.06, 1.02)), (0.08, 0.07, 0.10), "cloth", LEATHER_DARK, rot=(0, 0, -25), bevel=0.25),
        # The quiver across her back, its arrows rising over her right shoulder, fletched teal.
        path_part("spine_02", [sp((0.12, 0.17, 1.08)), sp((-0.10, 0.20, 1.62))], 8, [(.075, .06), (.08, .065)], "cloth", LEATHER_DARK),
        path_part("spine_02", [sp((0.10, 0.17, 1.12)), sp((0.07, 0.172, 1.16))], 8, [(.085, .07), (.085, .07)], "metal", SILVER),
        path_part("spine_02", [sp((-0.08, 0.20, 1.56)), sp((-0.13, 0.21, 1.66))], 8, [(.088, .072), (.088, .072)], "metal", SILVER),
        path_part("spine_02", [sp((-0.12, 0.20, 1.64)), sp((-0.17, 0.21, 1.80)), sp((-0.19, 0.22, 1.86))], 4,
                  [(.06, .015), (.05, .012), (0, 0)], "emissive", TEAL),
        path_part("spine_02", [sp((-0.07, 0.22, 1.64)), sp((-0.11, 0.24, 1.82)), sp((-0.12, 0.25, 1.88))], 4,
                  [(.055, .014), (.045, .012), (0, 0)], "emissive", TEAL),
        path_part("spine_02", [sp((-0.15, 0.17, 1.62)), sp((-0.21, 0.17, 1.76)), sp((-0.23, 0.17, 1.82))], 4,
                  [(.05, .013), (.04, .01), (0, 0)], "emissive", TEAL),
        # The cloak, off her left shoulder: forest green, a teal hem, on the cape chain.
        ("extra_cape_1", CAPE[0], CAPE[1], 6, [(0, .17, .024), (1, .21, .024)], "cloth", FOREST),
        ("extra_cape_2", CAPE[1], CAPE[2], 6, [(0, .21, .024), (1, .23, .024)], "cloth", FOREST),
        ("extra_cape_3", CAPE[2], CAPE[3], 6, [(0, .23, .024), (.75, .22, .022), (1, .17, .02)], "cloth", FOREST),
        ("extra_cape_3", (CAPE[3][0], CAPE[3][1] + 0.005, CAPE[3][2] + 0.06), (CAPE[3][0], CAPE[3][1] + 0.006, CAPE[3][2] - 0.01), 6,
         [(0, .20, .028), (1, .17, .026)], "cloth", TEAL_CLOTH),
        # Quiver straps across her chest, a cloak clasp, more fletching.
        box_part("chest", sp((0, -0.11, 1.40)), (0.035, 0.02, 0.44), "cloth", LEATHER_DARK, rot=(0, -38, 0)),
        box_part("chest", sp((0.06, -0.125, 1.47)), (0.04, 0.02, 0.04), "metal", SILVER, rot=(0, 45, 0)),
        box_part("chest", sp((0.13, -0.06, 1.55)), (0.05, 0.025, 0.05), "emissive", TEAL, rot=(0, 45, 0)),
        path_part("spine_02", [sp((-0.02, 0.21, 1.64)), sp((-0.04, 0.23, 1.80)), sp((-0.045, 0.235, 1.85))], 4,
                  [(.05, .012), (.04, .01), (0, 0)], "emissive", TEAL),
        path_part("spine_02", [sp((-0.18, 0.19, 1.62)), sp((-0.25, 0.20, 1.74)), sp((-0.27, 0.20, 1.79))], 4,
                  [(.045, .012), (.035, .01), (0, 0)], "emissive", TEAL),
        # A leather pauldron on the bow shoulder.
        path_part("clavicle_l", [("clavicle_l", 0.5, (0, 0, 0.06)), ("clavicle_l", 1.0, (0.03, 0, 0.09)),
                                 ("clavicle_l", 1.0, (0.12, 0, 0.04)), ("clavicle_l", 1.0, (0.15, 0, -0.06))],
                  6, [(.11, .04), (.13, .06), (.13, .06), (.11, .03)], "cloth", LEATHER, ref=(0, 1, 0)),
    ]


def _head(bm, layers, groups, mats):
    # The shared sculpted head, feminine and heroic-sized: a narrower jaw, large eyes; auburn
    # hair with side-swept bangs and a ponytail.
    head.build(bm, layers, groups, mats, head.Head(
        base=sp((0, -0.008, 1.62)), height=0.25, scale=1.1, jaw=0.48, chin=0.85, eye=1.25,
        skin=SKIN, hair=HAIR, hair_style="ponytail", eye_color=(0.12, 0.40, 0.36), lips=(0.70, 0.36, 0.36)))


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 8, [(0, .062, .062), (1, .052, .052)], "cloth", CREAM),
        (fa, (fa, 0), (fa, 1), 8, [(0, .05, .05), (1, .044, .044)], "cloth", CREAM),
        # Teal-banded leather bracers.
        (fa, (fa, .30), (fa, .98), 8, [(0, .06, .06), (1, .054, .054)], "cloth", LEATHER),
        (fa, (fa, .55), (fa, .62), 8, [(0, .064, .064), (1, .062, .062)], "cloth", TEAL_CLOTH),
        (fa, (fa, .38), (fa, .42), 8, [(0, .063, .063), (1, .063, .063)], "metal", SILVER),
        box_part(ca, (ca, 0.55, (0.0, -0.075, 0)), (0.05, 0.02, 0.04), "metal", SILVER),
        box_part(ca, (ca, 0.03, (0.0, -0.06, 0)), (0.08, 0.04, 0.09), "cloth", LEATHER, bevel=0.3),
        box_part(th, (th, 0.3, (0.07, -0.02, 0)), (0.06, 0.07, 0.10), "cloth", LEATHER_DARK, rot=(0, 0, 10), bevel=0.3),
        *champion.hand_parts("l", LEATHER_DARK, "cloth", scale=1.35),
        (th, sp((0.10, 0, 1.02)), (th, 1), 10, [(0, .10, .095), (.5, .085, .085), (1, .064, .064)], "cloth", FOREST_DARK),
        (ca, (ca, 0), (ca, .45), 10, [(0, .064, .064), (1, .058, .058)], "cloth", FOREST_DARK),
        # Tall boots with a turned cuff.
        (ca, (ca, .36), (ca, 1), 10, [(0, .078, .078), (.12, .072, .072), (1, .06, .064)], "cloth", LEATHER),
        (ca, (ca, .32), (ca, .40), 10, [(0, .086, .086), (1, .086, .086)], "cloth", LEATHER_DARK),
        box_part("foot_l", ("foot_l", 0.45), (0.10, 0.17, 0.09), "cloth", LEATHER, bevel=0.25),
        box_part("toe_l", ("toe_l", 0.35), (0.09, 0.08, 0.06), "cloth", LEATHER, bevel=0.25),
    ]


def _bow_pt(s):
    """A point on the bow's centerline, `s` meters from the grip along the limbs (rest pose)."""
    grip, axis, back = BOW_REST
    u = abs(s) / HALF
    # The limbs curve back toward her from the grip (the string side), and the recurve flicks
    # the tips forward again; the string runs behind the grip, never in front of it.
    return grip + axis * s + back * (BRACE * u * u) - back * (RECURVE * u ** 8)


def _bow():
    def radius(s):
        return 0.042 - 0.024 * abs(s) / HALF

    def bone(mid):
        if abs(mid) < 0.07:
            return "hand_l"
        return f"extra_bow_{(1 if mid > 0 else 3) + (1 if abs(mid) > LIMB_SPLIT else 0)}"

    stations = [-0.85, -0.74, -0.6, -0.45, -0.3, -0.17, -0.07, 0.07, 0.17, 0.3, 0.45, 0.6, 0.74, 0.85]
    out = []
    for a, b in zip(stations, stations[1:]):
        mid = abs((a + b) / 2)
        slot, color = ("emissive", TEAL) if mid > 0.74 else ("metal", SILVER) if mid > 0.6 else ("cloth", LEATHER_DARK) if mid < 0.07 else ("cloth", WOOD)
        out.append((bone((a + b) / 2), tuple(_bow_pt(a)), tuple(_bow_pt(b)), 6, [(0, radius(a), radius(a) * 1.3), (1, radius(b), radius(b) * 1.3)], slot, color))
    nock = tuple((_bow_pt(HALF) + _bow_pt(-HALF)) / 2)
    out.append(("extra_string_1", tuple(_bow_pt(HALF)), nock, 4, [(0, .006, .006), (1, .006, .006)], "cloth", STRING))
    out.append(("extra_string_2", tuple(_bow_pt(-HALF)), nock, 4, [(0, .006, .006), (1, .006, .006)], "cloth", STRING))
    return out


def bow_frame():
    """Rest-pose directions for the bow: whatever the left hand turns into *up* and *backward*
    at the full-draw frame of `attack_1`."""
    act = bpy.data.actions.get("attack_1")
    ARM.animation_data.action = act
    if ARM.animation_data.action_slot is None and act.slots:
        ARM.animation_data.action_slot = act.slots[0]
    fire = next(m.frame for m in act.pose_markers if m.name == "fire")
    bpy.context.scene.frame_set(int(fire) - 1)
    bpy.context.view_layer.update()
    pb = ARM.pose.bones["hand_l"]
    delta = pb.matrix.to_quaternion() @ pb.bone.matrix_local.to_quaternion().inverted()
    inv = delta.inverted()
    ARM.animation_data.action = None
    clips.reset_pose(ARM)
    bpy.context.scene.frame_set(0)
    axis = (inv @ Vector((0, 0, 1))).normalized()
    back = inv @ Vector((0, 1, 0))
    back = (back - axis * back.dot(axis)).normalized()
    return axis, back


def _bow_bones(armature):
    grip = BOW_REST[0]
    axis = BOW_REST[1]
    p = _bow_pt
    nock = (p(HALF) + p(-HALF)) / 2
    specs = [("extra_bow_1", grip + axis * 0.07, p(LIMB_SPLIT), "hand_l"), ("extra_bow_2", p(LIMB_SPLIT), p(HALF), "extra_bow_1"),
             ("extra_bow_3", grip - axis * 0.07, p(-LIMB_SPLIT), "hand_l"), ("extra_bow_4", p(-LIMB_SPLIT), p(-HALF), "extra_bow_3"),
             ("extra_string_1", p(HALF), nock, "extra_bow_2"), ("extra_string_2", p(-HALF), nock, "extra_bow_4")]
    prev = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    eb = armature.data.edit_bones
    for name, a, b, parent in specs:
        bone = eb.new(name)
        bone.head, bone.tail = a, b
        bone.use_deform = True
        bone.align_roll(BOW_REST[2])
        bone.parent = eb[parent]
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev
    for pb in armature.pose.bones:
        pb.rotation_mode = "QUATERNION"


def _bake_draws(armature):
    """The draw, baked on every frame of each shot: the nocking point follows her right fingers
    from the nock to the release, the limbs bend toward her by how far it comes back, the string
    halves turn and stretch from the (bent) tips to it; on `fire` the string snaps forward and
    shivers for a few frames."""
    grip, axis, back = BOW_REST
    side = axis.cross(back).normalized()
    p = _bow_pt
    h_top, h_bot, m_top, m_bot, tip_top, tip_bot = grip + axis * 0.07, grip - axis * 0.07, p(LIMB_SPLIT), p(-LIMB_SPLIT), p(HALF), p(-HALF)
    nock0 = (tip_top + tip_bot) / 2
    pb = armature.pose.bones
    rest = {n: pb[n].bone.matrix_local.to_quaternion() for n in ("extra_bow_1", "extra_bow_2", "extra_bow_3", "extra_bow_4", "extra_string_1", "extra_string_2")}
    ad = armature.animation_data
    sc = bpy.context.scene
    saved = ad.action
    shiver = (-0.05, 0.035, -0.02, 0.01)
    for name, fire in DRAWS.items():
        act = bpy.data.actions.get(name)
        if act is None:
            continue
        ad.action = act
        if ad.action_slot is None and act.slots:
            ad.action_slot = act.slots[0]
        full = max(fire - 4, 2)
        for f in range(0, int(act.frame_range[1]) + 1):
            sc.frame_set(f)
            hl = pb["hand_l"]
            to_rest = (hl.matrix @ hl.bone.matrix_local.inverted()).inverted()
            if f < fire:
                d = min(1.0, f / full)
                nock = nock0.lerp(to_rest @ pb["fingers_r"].head, d * d * (3 - 2 * d))
            elif f - fire < len(shiver):
                nock = nock0 + back * shiver[f - fire]
            else:
                nock = nock0.copy()
            pull = max(0.0, (nock - nock0).dot(back))
            bend = min(pull / 0.55, 1.2)
            a1, a2 = math.radians(10) * bend, math.radians(9) * bend
            q1, q12 = Quaternion(side, a1), Quaternion(side, a1 + a2)
            q3, q34 = Quaternion(side, -a1), Quaternion(side, -(a1 + a2))
            tt = h_top + q1 @ (m_top - h_top) + q12 @ (tip_top - m_top)
            tb = h_bot + q3 @ (m_bot - h_bot) + q34 @ (tip_bot - m_bot)
            turns = {"extra_bow_1": q1, "extra_bow_2": Quaternion(side, a2), "extra_bow_3": q3, "extra_bow_4": Quaternion(side, -a2)}
            scales = {}
            for s_name, tip0, tip, parent_q in (("extra_string_1", tip_top, tt, q12), ("extra_string_2", tip_bot, tb, q34)):
                want = nock - tip
                turns[s_name] = parent_q.inverted() @ (nock0 - tip0).rotation_difference(want)
                scales[s_name] = want.length / (nock0 - tip0).length
            for n, q_arm in turns.items():
                b = pb[n]
                b.rotation_quaternion = rest[n].inverted() @ q_arm @ rest[n]
                b.keyframe_insert("rotation_quaternion", frame=f, group=n)
                if n in scales:
                    b.scale = (1.0, scales[n], 1.0)
                    b.keyframe_insert("scale", frame=f, group=n)
    ad.action = saved
    clips.reset_pose(armature)
    sc.frame_set(0)


def bow_turn(want_axis, want_back):
    """`hand_l` turned so the bow's limbs lie along `want_axis` and its string faces
    `want_back` (armature space)."""
    def turn():
        _, axis, back = BOW_REST
        a = Vector(want_axis).normalized()
        b = Vector(want_back)
        b = (b - a * b.dot(a)).normalized()
        rest = Matrix((axis, back, axis.cross(back))).transposed()
        want = Matrix((a, b, a.cross(b))).transposed()
        return (want @ rest.transposed()).to_quaternion()
    return {"hand_l": {"turn": turn}}


def parts():
    return _center() + champion.mirrored(_left()) + _bow() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def cape(back=0.0, sway=0.0):
    """The cloak: `back` swings it behind her (degrees), `sway` to her left."""
    return {"extra_cape_1": (back * 0.5, sway * 0.5, 0), "extra_cape_2": (back * 0.3, sway * 0.3, 0),
            "extra_cape_3": (back * 0.25, sway * 0.25, 0)}


STAND = feet((0.13, 0.04), (0.13, -0.06))
STANCE = feet((0.18, 0.20), (0.20, -0.22))  # side-on for shooting, left foot forward
# At rest the bow is held low against her body (the user's sketch): the grip in the left hand
# before her left thigh, the bow lying across the front of her legs, the string up.
BOW_LOW = merge(hand("l", at(0.12, 0.17, 0.90, "l")), bow_turn((-1, -0.22, -0.10), (0, 0.25, 1)), arm("r", 0.25, 0.0, -1, 0.2, 0.2, -1))
READY = merge(feet((0.18, 0.12), (0.18, -0.16)), arm("l", 0.15, 0.75, -0.55, 0.05, 1, -0.15), arm("r", 0.3, 0.3, -0.8, -0.2, 0.85, -0.3),
              {"spine_01": (8, 0, 0), "head": (-4, 0, 0)}, cape(4))
DOWN = {"pelvis": (0, 0, -0.06)}
# Draw stance: the torso turned right so the bow shoulder leads, the head toward the target.
TURN = {"pelvis": (0, 0, -10), "spine_01": (4, 0, -10), "spine_02": (2, 0, -16), "chest": (0, 0, -10), "neck": (0, 0, 18), "head": (-2, 0, 18)}


def aim_l(up=0.04):
    return arm("l", 0.02, 1, up, 0.0, 1, up + 0.01)


def draw_r(depth=1.0, up=0.15):
    """The string hand at her jaw (an IK target), `depth` 1 = full draw."""
    return hand("r", at(0.06 - 0.02 * depth, 0.28 - 0.32 * depth, 1.50 + up * 0.2, "r"), aim=at(-0.3, 1, 0, "r"))


RELEASE_R = arm("r", 0.40, -0.85, 0.25, 0.25, -0.6, 0.35)


# --- Clips ----------------------------------------------------------------------------------

def idle(armature):
    a = merge(STAND, BOW_LOW, cape(0), {"head": (-2, 0, 0)})
    b = merge(STAND, BOW_LOW, cape(1.5, 2), sym("clavicle", 0, -3, 0), {"head": (-3, 0, 6), "chest": (-2, 0, 0)})
    return _clip(armature, "idle", 72, [(0, a, {"pelvis": (0, 0, -0.01)}), (36, b, {"pelvis": (0, 0, 0.0)})], loop=True)


def idle_fidget_1(armature):
    # Tests the string: the bow up across her body, two plucks, a listen.
    base = merge(STAND, BOW_LOW, cape(0))
    up = merge(STAND, arm("l", 0.15, 0.9, -0.1, -0.25, 1, 0.05), arm("r", 0.2, 0.6, -0.6, -0.45, 0.9, 0.2), {"head": (12, 0, 8)}, cape(1))
    pluck = merge(up, arm("r", 0.2, 0.6, -0.6, -0.3, 0.7, 0.5))
    keys = [(0, base, None), (12, up, None), (16, pluck, None, "snap"), (24, up, None), (28, pluck, None, "snap"), (40, merge(up, {"head": (16, 0, 14)}), None),
            (48, merge(STAND, BOW_LOW, {"head": (4, 0, 4)}), None), (56, base, None)]
    return _clip(armature, "idle_fidget_1", 56, keys)


def idle_fidget_2(armature):
    # Scans the horizon, a hand shading her eyes.
    base = merge(STAND, BOW_LOW, cape(0))
    shade = arm("r", 0.25, 0.6, 0.5, -0.6, 0.4, 0.6)
    keys = [(0, base, None), (14, merge(STAND, BOW_LOW, shade, {"head": (-6, 0, 25), "spine_02": (0, 0, 6)}, cape(0, -3)), None),
            (32, merge(STAND, BOW_LOW, shade, {"head": (-6, 0, -25), "spine_02": (0, 0, -8)}, cape(0, 3)), None),
            (46, merge(STAND, BOW_LOW, {"head": (-2, 0, 0)}), None), (60, base, None)]
    return _clip(armature, "idle_fidget_2", 60, keys)


def idle_ready(armature):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (11, 0, 0)}, cape(5, 1)), {"pelvis": (0, 0, -0.08)})]
    return _clip(armature, "idle_ready", 40, keys, loop=True)


def _run_upper(swing, cape_back):
    def upper(p):
        c = math.cos(2 * math.pi * p)
        return merge(arm("l", 0.24, -0.1 - 0.25 * swing * c, -1, 0.18, 0.45, -0.85),
                     arm("r", 0.22, 0.55 * swing * c, -0.85, 0.1, 0.6 + 0.4 * swing * c, -0.4),
                     cape(cape_back + 5 * math.sin(4 * math.pi * p), 3 * math.sin(2 * math.pi * p)), {"head": (-8, 0, 0)})
    return upper



def walk(armature):
    # A walk (heavy slows, and the moment a run comes to a stop): the run's body type, slower,
    # the weapon carried as in it, never hanging into the ground (14 §3.6).
    keys = gait(32, 0.90, contact=0.6, lift=0.10, bob=0.02, lean=4, twist=6, width=0.12, crouch=0.0, upper=_run_upper(0.4, 12))
    return _clip(armature, "walk", 32, keys, loop=True, markers={"foot_l": 0, "foot_r": 16}, stride_speed=140.0)

def run(armature):
    keys = gait(20, 0.97, contact=0.44, lift=0.24, bob=0.045, lean=11, twist=9, width=0.12, upper=_run_upper(1.0, 28))
    return _clip(armature, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(armature):
    keys = gait(16, 0.96, contact=0.40, lift=0.28, bob=0.05, lean=15, twist=11, width=0.12, upper=_run_upper(1.3, 38))
    return _clip(armature, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _shot(armature, name, frames, fire, aim_up=0.0, lean=0.0, crouch=0.0, depth=1.0):
    """A bow shot (14 §3.1): nock, draw, a held full draw, a one-frame release on `fire`, the
    recoil held, then back to ready."""
    body = merge(STANCE, TURN, {"spine_01": (4 + lean, 0, -10), "spine_02": (2 + lean * 0.5, 0, -16)})
    low = {"pelvis": (0, 0, -0.04 - crouch)}
    nock = merge(STANCE, arm("l", 0.2, 0.6, -0.7, 0.05, 1, -0.35), hand("r", at(0.10, 0.50, 1.12, "r"), aim=at(-0.4, 1, 0.3, "r")),
                 {"spine_02": (2, 0, -8), "head": (-2, 0, 10)}, cape(3))
    drawing = merge(body, aim_l(0.04 + aim_up), draw_r(0.55, aim_up), cape(4))
    full = merge(body, aim_l(0.04 + aim_up), draw_r(depth, aim_up), cape(4, -1))
    held = merge(full, {"spine_02": (2 + lean * 0.5, 0, -18)})
    release = merge(body, aim_l(0.10 + aim_up), RELEASE_R, cape(7, 3), {"head": (-4, 0, 16)})
    DRAWS[name] = fire
    keys = [(0, nock, low), (max(fire - 4, 1), drawing, low), (fire - 2, full, low), (fire - 1, held, low, "snap"),
            (fire, release, low), (fire + 4, merge(release, {"spine_02": (lean * 0.5, 0, -14)}), low, "settle"), (frames, READY, DOWN)]
    return _clip(armature, name, frames, keys, markers={"fire": fire})


def attack_1(armature):
    return _shot(armature, "attack_1", 24, 7)


def attack_2(armature):
    # The quick variant: lower, crouched, drawn to the chest.
    return _shot(armature, "attack_2", 24, 7, aim_up=-0.05, lean=6, crouch=0.08, depth=0.8)


def q(armature):
    # Longshot: a deeper draw aimed higher, leaning back into it; the recoil rocks her back.
    return _shot(armature, "q", 18, 8, aim_up=0.12, lean=-6, depth=1.1)


def w(armature):
    # Shrapnel Charge: a bomb plucked from her hip, swung back low, lobbed overhand on `fire` 8
    # (the arm follows through high), the bow kept out of the way.
    st = feet((0.16, 0.24), (0.18, -0.20))
    pluck = merge(st, BOW_LOW, hand("r", at(0.20, 0.0, 0.98, "r"), aim=at(0, 0.2, -1, "r")), {"head": (6, 0, -10)}, cape(3))
    wind = merge(st, BOW_LOW, arm("r", 0.35, -0.75, -0.35, 0.3, -0.6, 0.4), {"spine_01": (-4, 0, -18), "spine_02": (-4, 0, -10)}, cape(4, -2))
    throw = merge(feet((0.16, 0.30), (0.18, -0.20)), BOW_LOW, arm("r", 0.1, 0.75, 0.65, 0.0, 0.7, 0.75), {"spine_01": (10, 0, 12), "spine_02": (6, 0, 8)}, cape(8, 3))
    keys = [(0, READY, DOWN), (4, pluck, DOWN), (7, wind, DOWN, "snap"), (8, throw, {"pelvis": (0, -0.04, -0.06)}),
            (12, merge(throw, {"spine_01": (14, 0, 16)}), {"pelvis": (0, -0.04, -0.07)}, "settle"), (18, READY, DOWN)]
    return _clip(armature, "w", 18, keys, markers={"fire": 8})


def e_start(armature):
    # Tumble: a quick dip, then a dive into the roll.
    dip = merge(feet((0.15, 0.10), (0.15, -0.12)), BOW_LOW, {"spine_01": (24, 0, 0), "head": (-10, 0, 0)}, cape(6))
    dive = merge(BOW_LOW, {"pelvis": (40, 0, 0), "spine_01": (30, 0, 0), "thigh_l": (-30, 0, 0), "thigh_r": (10, 0, 0),
                           "calf_l": (40, 0, 0), "calf_r": (20, 0, 0)}, cape(10))
    return _clip(armature, "e_start", 4, [(0, dip, {"pelvis": (0, 0, -0.22)}, "snap"), (4, dive, {"pelvis": (0, -0.15, -0.45)})], markers={"fire": 1})


TUCK = merge(arm("l", 0.2, 0.9, -0.2, 0.0, 0.4, 0.9), arm("r", 0.2, 0.9, -0.2, 0.0, 0.4, 0.9),
             sym("thigh", -110, -6, 0), sym("calf", 130, 0, 0), sym("foot", 20, 0, 0), {"spine_01": (40, 0, 0), "spine_02": (20, 0, 0), "head": (30, 0, 0)})


def e_travel(armature):
    # A forward roll, in place: the curled body turns a full circle while the sim carries her.
    keys = [(f, merge(TUCK, {"pelvis": (90 * i + 40, 0, 0)}, cape(60)), {"pelvis": (0, 0, -0.55)}) for i, f in enumerate((0, 3, 6, 9))]
    keys.append((12, merge(TUCK, {"pelvis": (400, 0, 0)}, cape(60)), {"pelvis": (0, 0, -0.55)}))
    return _clip(armature, "e_travel", 12, keys, loop=True)


def e_land(armature):
    # Out of the roll onto one knee, bow up and ready; held a beat, then up.
    kneel = merge({"ik_foot_l": clips.ik(at(0.16, 0.30, champion.ankle(), "l")), "thigh_r": (10, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
                  arm("l", 0.15, 0.85, -0.3, 0.05, 1, 0.0), arm("r", 0.3, 0.4, -0.7, -0.3, 0.8, -0.2), {"spine_01": (14, 0, 0), "head": (-10, 0, 0)}, cape(20))
    keys = [(0, kneel, {"pelvis": (0, 0.04, -0.46)}, "settle"), (4, merge(kneel, cape(12)), {"pelvis": (0, 0.04, -0.44)}), (10, READY, DOWN)]
    return _clip(armature, "e_land", 10, keys)


def r(armature):
    # Snare Net: the net gathered in both hands and swung back, flung forward wide on `fire` 8,
    # both arms flying open and held.
    st = feet((0.18, 0.24), (0.20, -0.20))
    gather = merge(st, arm("l", 0.5, -0.55, -0.35, 0.3, -0.3, 0.3), arm("r", 0.5, -0.55, -0.35, 0.3, -0.3, 0.3),
                   {"spine_01": (-6, 0, 0), "spine_02": (-10, 0, 0), "head": (-6, 0, 0)}, cape(2))
    fling = merge(feet((0.18, 0.32), (0.20, -0.20)), arm("l", 0.75, 0.75, 0.25, 0.8, 0.6, 0.2), arm("r", 0.75, 0.75, 0.25, 0.8, 0.6, 0.2),
                  {"spine_01": (14, 0, 0), "spine_02": (8, 0, 0), "head": (-12, 0, 0)}, cape(10))
    keys = [(0, READY, DOWN), (5, gather, {"pelvis": (0, 0.04, -0.04)}), (7, merge(gather, {"spine_02": (-14, 0, 0)}), {"pelvis": (0, 0.05, -0.04)}, "snap"),
            (8, fling, {"pelvis": (0, -0.06, -0.08)}), (13, merge(fling, {"spine_01": (16, 0, 0)}), {"pelvis": (0, -0.06, -0.09)}, "settle"), (20, READY, DOWN)]
    return _clip(armature, "r", 20, keys, markers={"fire": 8})


KNEEL = merge({"ik_foot_l": clips.ik(at(0.16, 0.28, champion.ankle(), "l")), "thigh_r": (8, 0, 0), "calf_r": (105, 0, 0), "foot_r": (40, 0, 0)},
              arm("l", 0.3, 0.6, -0.6, 0.2, 0.7, 0.2), arm("r", 0.2, 0.7, -0.5, -0.4, 0.7, 0.3), {"spine_01": (12, 0, 0), "head": (14, 0, 0)}, cape(-4))


def recall(armature):
    keys = [(0, merge(STAND, BOW_LOW), None), (15, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (45, merge(KNEEL, {"head": (18, 0, 6)}), {"pelvis": (0, 0.04, -0.45)}),
            (75, KNEEL, {"pelvis": (0, 0.04, -0.46)}), (90, merge(STAND, BOW_LOW), None)]
    return _clip(armature, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(armature):
    # Struck, she twists, drops the bow arm, and falls back onto her side.
    hit = merge(feet((0.14, -0.05), (0.14, -0.20)), arm("l", 0.6, -0.1, -0.4, 0.5, 0.2, -0.6), arm("r", 0.5, -0.2, 0.2, 0.5, 0.2, 0.5),
                {"spine_01": (-14, 0, 20), "head": (-28, 0, 14)}, cape(-20))
    knees = merge({"thigh_l": (-10, 0, 0), "thigh_r": (0, 0, 0), "calf_l": (100, 0, 0), "calf_r": (110, 0, 0)},
                  arm("l", 0.6, 0.3, -0.7, 0.5, 0.3, -0.8), arm("r", 0.4, 0.3, -0.8, 0.3, 0.4, -0.8), {"spine_01": (14, 0, -10), "head": (20, 0, 0)}, cape(-8))
    down = merge({"thigh_l": (-30, 0, 0), "thigh_r": (-20, 0, 0), "calf_l": (40, 0, 0), "calf_r": (30, 0, 0)},
                 arm("l", 1, 0.0, 0.2, 1, 0.2, 0.1), arm("r", 0.6, 0.3, 0.6, 0.4, 0.4, 0.6), {"pelvis": (-70, 20, 0), "head": (20, 0, -20)}, cape(-30))
    keys = [(0, merge(STAND, BOW_LOW), None), (6, hit, {"pelvis": (0, 0.06, 0)}, "settle"), (15, knees, {"pelvis": (0, 0.02, -0.5)}, "snap"),
            (25, down, {"pelvis": (0, 0.45, -0.82)}, "snap"), (34, down, {"pelvis": (0, 0.47, -0.84)})]
    return _clip(armature, "death", 34, keys)


def respawn(armature):
    keys = [(0, KNEEL, {"pelvis": (0, 0.04, -0.46)}, "settle"), (12, merge(STAND, BOW_LOW, {"head": (-6, 0, 0)}), {"pelvis": (0, 0, -0.02)}),
            (24, merge(STAND, BOW_LOW), None)]
    return _clip(armature, "respawn", 24, keys)


def select(armature):
    # Spins the bow up into her hand, draws and holds on the sky, then lowers it with a smile.
    full = merge(STANCE, TURN, aim_l(0.5), hand("r", at(0.04, -0.02, 1.66, "r"), aim=at(-0.3, 1, 0.4, "r")), {"spine_02": (-6, 0, -16), "head": (-16, 0, 18)}, cape(5))
    keys = [(0, merge(STAND, BOW_LOW), None), (10, merge(STANCE, aim_l(0.3), arm("r", 0.3, 0.5, -0.5, -0.3, 0.9, 0.0), cape(3)), None),
            (18, full, None), (30, merge(full, {"spine_02": (-8, 0, -18)}), None), (31, merge(STANCE, TURN, aim_l(0.55), RELEASE_R, cape(8)), None),
            (40, merge(STAND, BOW_LOW, {"head": (-4, 0, -10)}), None), (48, merge(STAND, BOW_LOW), None)]
    return _clip(armature, "select", 48, keys)


def emote_taunt(armature):
    # Points two fingers at her eyes, then at you.
    eyes = merge(STAND, BOW_LOW, arm("r", 0.3, 0.6, 0.3, -0.7, 0.3, 0.6))
    you = merge(STAND, BOW_LOW, arm("r", 0.15, 1, 0.2, 0.05, 1, 0.2))
    keys = [(0, merge(STAND, BOW_LOW), None), (10, eyes, None), (16, eyes, None), (20, you, None, "snap"), (34, you, None), (44, merge(STAND, BOW_LOW), None)]
    return _clip(armature, "emote_taunt", 44, keys)


def emote_joke(armature):
    # Balances an arrow on one fingertip, wobbles, catches it.
    bal = merge(STAND, BOW_LOW, arm("r", 0.4, 0.5, -0.1, 0.0, 0.5, 0.85), {"head": (-12, 0, 0)})
    keys = [(0, merge(STAND, BOW_LOW), None), (10, bal, None)]
    for i, f in enumerate(range(14, 38, 4)):
        keys.append((f, merge(bal, {"spine_02": (0, 4 if i % 2 else -4, 0)}, arm("r", 0.4 + (0.1 if i % 2 else -0.1), 0.5, -0.1, 0.0, 0.5, 0.85)), None))
    keys += [(42, merge(STAND, BOW_LOW, arm("r", 0.3, 0.7, -0.3, 0.0, 0.8, 0.2)), None, "snap"), (50, merge(STAND, BOW_LOW), None)]
    return _clip(armature, "emote_joke", 50, keys)


def emote_laugh(armature):
    keys = [(0, merge(STAND, BOW_LOW), None)]
    for i in range(1, 9):
        s = 1.0 if i % 2 else 0.4
        keys.append((4 * i, merge(STAND, BOW_LOW, arm("r", 0.15, 0.3, -0.9, -0.7, 0.6, 0.2), {"spine_01": (8 * s, 0, 0), "head": (14 * s, 0, 0)},
                                  sym("clavicle", 0, -4 * s, 0)), None))
    keys.append((40, merge(STAND, BOW_LOW), None))
    return _clip(armature, "emote_laugh", 40, keys)


def emote_dance(armature):
    # A light, bouncing two-step, the bow twirled overhead.
    up = merge(arm("l", 0.3, 0.1, 1, 0.1, 0.1, 1), arm("r", 0.6, 0.3, -0.4, 0.3, 0.6, 0.2))
    a = merge(up, feet((0.13, 0.08), (0.16, -0.06), lift_r=0.16), {"pelvis": (0, -6, 10)}, cape(8, 8))
    b = merge(up, feet((0.16, 0.06), (0.13, -0.08), lift_l=0.16), {"pelvis": (0, 6, -10)}, cape(8, -8))
    keys = [(0, a, {"pelvis": (0.04, 0, 0.02)}, "whip"), (12, merge(up, STAND), {"pelvis": (0, 0, -0.05)}), (24, b, {"pelvis": (-0.04, 0, 0.02)}, "whip"),
            (36, merge(up, STAND), {"pelvis": (0, 0, -0.05)})]
    return _clip(armature, "emote_dance", 48, keys, loop=True)


OWN = [attack_1, idle, idle_fidget_1, idle_fidget_2, idle_ready, walk, run, run_fast, attack_2, q, w, e_start, e_travel, e_land, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]


def _extras(armature):
    global ARM, BOW_REST
    ARM = armature
    rig.add_chain(armature, "cape", "chest", CAPE)
    # A first draw to learn the bow's frame (it's rebuilt with the rest, keying the bow bones).
    attack_1(armature)
    axis, back = bow_frame()
    h = next(Vector(b[2]) for b in rig.bone_table() if b[0] == "hand_l")
    t = next(Vector(b[3]) for b in rig.bone_table() if b[0] == "hand_l")
    BOW_REST = (h + (t - h).normalized() * 0.06, axis, back)
    _bow_bones(armature)


def build(only=None):
    armature, body = champion.build("vesper", SHAPE, parts, OWN, 1.85, extras=_extras, only=only, shared=champion.SHARED + ["attack_melee_alt"],
                                    cloth={"cape": dict(stiffness=0.26, inertia=0.86)})
    _bake_draws(armature)
    return armature, body


if __name__ == "__main__":
    champion.main(build)
