# SPDX-License-Identifier: AGPL-3.0-or-later
"""Quill (A11): the block-out generator for her model and clips.

    blender -b --python art/champions/quill/build.py -- --out art/champions/quill/quill.blend

A stargazer scholar on `biped` v1: a long teal coat with brass buttons and coat tails (an
`extra_coat` chain), tall boots, brass goggles pushed up on her forehead, short dark hair, and a
tall staff topped with a brass armillary sphere around a glowing star (laid along her right hand
bone and aimed per pose). Long-range artillery: Arc Shot, Static Field, a Recoil hop, and the
map-length Starfall Lance. Like the other block-outs this is a starting point.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402
from mathutils import Vector  # noqa: E402

from mftr_blender import clips, head, mesh, rig  # noqa: E402
from mftr_blender.clips import merge, sym  # noqa: E402
from mftr_blender.library import _clip, arms, cycle, run_half  # noqa: E402

# Palette (concept.md): deep teal and brass, starlight.
SKIN = (0.82, 0.62, 0.5)
HAIR = (0.12, 0.12, 0.18)
COAT = (0.12, 0.34, 0.38)
COAT_DARK = (0.08, 0.22, 0.25)
SHIRT = (0.82, 0.78, 0.68)
TROUSERS = (0.2, 0.18, 0.2)
BOOT = (0.24, 0.16, 0.1)
BRASS = (0.8, 0.62, 0.3)
WOOD = (0.3, 0.2, 0.12)
STAR = (0.82, 0.97, 1.0)
ACCENT = (0.18, 0.52, 0.95)

COAT_TAIL = [(0, 0.11, 1.06), (0, 0.16, 0.78), (0, 0.2, 0.52)]


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", (0, 0, 0.92), (0, 0, 1.12), 12, [(0, .13, .095), (.4, .168, .11), (.8, .172, .11), (1, .15, .10)], "cloth", TROUSERS),
        ("pelvis", (0, 0, 1.05), (0, 0, 1.11), 12, [(0, .168, .112), (1, .16, .108)], "accent", ACCENT),
        # The coat: fitted, buttoned in brass, a pale shirt at the collar.
        ("spine_01", (0, 0, 1.10), (0, 0, 1.28), 12, [(0, .132, .094), (.5, .116, .086), (1, .124, .09)], "cloth", COAT),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .126, .09), (1, .15, .104)], "cloth", COAT),
        ("chest", (0, 0, 1.40), (0, 0, 1.60), 12, [(0, .15, .104), (.35, .158, .11), (.7, .15, .104), (1, .085, .072)], "cloth", COAT),
        ("chest", (0, -0.03, 1.415), (0, -0.036, 1.53), 10, [(0, .11, .055), (.5, .135, .072), (1, .105, .048)], "cloth", COAT),
        ("chest", (0, -0.06, 1.55), (0, -0.07, 1.62), 8, [(0, .06, .04), (1, .05, .035)], "cloth", SHIRT),
        ("chest", (0.06, -0.125, 1.5), (0.06, -0.14, 1.5), 4, [(0, .016, .016), (1, .014, .014)], "metal", BRASS),
        ("chest", (0.06, -0.12, 1.38), (0.06, -0.135, 1.38), 4, [(0, .016, .016), (1, .014, .014)], "metal", BRASS),
        ("spine_01", (0.06, -0.105, 1.24), (0.06, -0.12, 1.24), 4, [(0, .016, .016), (1, .014, .014)], "metal", BRASS),
        # Team-colored lapels.
        ("chest", (0.07, -0.115, 1.58), (0.03, -0.13, 1.44), 4, [(0, .03, .01), (1, .02, .01)], "accent", ACCENT),
        ("chest", (-0.07, -0.115, 1.58), (-0.03, -0.13, 1.44), 4, [(0, .03, .01), (1, .02, .01)], "accent", ACCENT),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.66), 8, [(0, .05, .05), (1, .045, .045)], "skin", SKIN),
        ("extra_coat_1", COAT_TAIL[0], COAT_TAIL[1], 6, [(0, .16, .024), (1, .18, .024)], "cloth", COAT),
        ("extra_coat_2", COAT_TAIL[1], COAT_TAIL[2], 6, [(0, .18, .024), (.7, .17, .021), (1, .11, .017)], "cloth", COAT_DARK),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (0.15, 0.01, 1.47), (0.205, 0.01, 1.585), 8, [(0, .07, .07), (.5, .074, .07), (1, 0, 0)], "cloth", COAT_DARK),
        (ua, (ua, 0), (ua, 1), 8, [(0, .054, .054), (1, .046, .046)], "cloth", COAT),
        (fa, (fa, 0), (fa, .92), 8, [(0, .045, .045), (1, .052, .052)], "cloth", COAT),
        (fa, (fa, .8), (fa, .94), 8, [(0, .056, .056), (1, .056, .056)], "metal", BRASS),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .036, .023), (1, .04, .02)], "skin", SKIN),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .04, .02), (1, .026, .014)], "skin", SKIN),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .016, .016), (1, .011, .011)], "skin", SKIN),
        (th, (0.10, 0, 1.02), (th, 1), 10, [(0, .086, .085), (.5, .074, .074), (1, .056, .056)], "cloth", TROUSERS),
        (ca, (ca, -0.1), (ca, 1), 10, [(0, .066, .066), (.15, .062, .062), (1, .052, .056)], "cloth", BOOT),
        (ca, (ca, -0.12), (ca, -0.02), 10, [(0, .072, .072), (1, .07, .07)], "cloth", BOOT),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .052, .046), (1, .046, .032)], "cloth", BOOT),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .046, .028), (1, .03, .018)], "cloth", BOOT),
    ]


def _staff():
    """The armillary staff along the right hand bone: a wooden shaft, brass fittings, three brass
    rings crossing around a glowing star at the top."""
    h = Vector(next(b[2] for b in rig.bone_table() if b[0] == "hand_r"))
    t = Vector(next(b[3] for b in rig.bone_table() if b[0] == "hand_r"))
    axis = (t - h).normalized()
    grip = h + (t - h) * 0.6
    top = grip + axis * 1.05
    out = [
        ("hand_r", tuple(grip - axis * 0.55), tuple(grip + axis * 0.95), 6, [(0, .014, .014), (1, .018, .018)], "cloth", WOOD),
        ("hand_r", tuple(grip - axis * 0.57), tuple(grip - axis * 0.5), 6, [(0, .022, .022), (1, .022, .022)], "metal", BRASS),
        ("hand_r", tuple(grip + axis * 0.92), tuple(grip + axis * 0.98), 6, [(0, .028, .028), (1, .022, .022)], "metal", BRASS),
        ("hand_r", tuple(top - axis * 0.05), tuple(top + axis * 0.05), 8, [(0, 0, 0), (.5, .045, .045), (1, 0, 0)], "emissive", STAR),
    ]
    # Three rings in different planes around the star.
    side = axis.cross(Vector((0, 1, 0)))
    if side.length < 1e-3:
        side = Vector((1, 0, 0))
    side.normalize()
    fwd = axis.cross(side).normalized()
    for u, v in ((side, fwd), (axis, side), (axis, fwd)):
        n = 10
        pts = [top + (u * math.cos(2 * math.pi * i / n) + v * math.sin(2 * math.pi * i / n)) * 0.11 for i in range(n)]
        for i in range(n):
            out.append(("hand_r", tuple(pts[i]), tuple(pts[(i + 1) % n]), 4, [(0, .008, .008), (1, .008, .008)], "metal", BRASS))
    return out


def _head(bm, layers, groups, mats):
    # The shared sculpted head, feminine and bright-eyed: a short dark bob with bangs, and brass
    # goggles pushed up on her forehead.
    head.build(bm, layers, groups, mats, head.Head(
        base=(0, -0.008, 1.64), height=0.25, scale=0.93, jaw=0.46, chin=0.86, eye=1.15,
        skin=SKIN, hair=HAIR, hair_style="short", eye_color=(0.22, 0.42, 0.44), lips=(0.7, 0.4, 0.38)))
    import bmesh
    deform, col = layers
    hd = head.Head(base=(0, -0.008, 1.64), height=0.25, scale=0.93)
    for x in (0.035, -0.035):
        c = hd.front(0.84, x * 0.93) + Vector((0, -0.02, 0.01))
        for r, d, color in ((0.022, 0.018, BRASS), (0.015, 0.024, STAR)):
            verts = []
            ring = [bm.verts.new(c + Vector((r * math.cos(2 * math.pi * k / 8), 0, r * math.sin(2 * math.pi * k / 8)))) for k in range(8)]
            front = [bm.verts.new(v.co + Vector((0, -d, 0))) for v in ring]
            faces = [bm.faces.new((ring[k], ring[(k + 1) % 8], front[(k + 1) % 8], front[k])) for k in range(8)]
            faces.append(bm.faces.new(list(reversed(ring))))
            faces.append(bm.faces.new(front))
            bmesh.ops.recalc_face_normals(bm, faces=faces)
            verts = ring + front
            for v in verts:
                v[deform][groups["head"]] = 1.0
                v[col] = (*color, 1.0)
            for f in faces:
                f.material_index = mats["metal"] if color == BRASS else mats["emissive"]
                f.smooth = False
    # The goggles' strap around the head.
    strap = []
    for k in range(16):
        a = 2 * math.pi * (k + 0.5) / 16
        p = hd.surface(0.84, a)
        c0 = hd.base + Vector((0, 0.01, (p - hd.base).z))
        strap.append(bm.verts.new(c0 + (p - c0) * 1.12))
    strap2 = [bm.verts.new(v.co + Vector((0, 0, 0.022))) for v in strap]
    faces = [bm.faces.new((strap[k], strap[(k + 1) % 16], strap2[(k + 1) % 16], strap2[k])) for k in range(16)]
    for v in strap + strap2:
        v[deform][groups["head"]] = 1.0
        v[col] = (*BOOT, 1.0)
    for f in faces:
        f.material_index = mats["cloth"]
        f.smooth = False


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + _staff() + [_head]


# --- Poses ----------------------------------------------------------------------------------

def coat(back=0.0, sway=0.0):
    return {"extra_coat_1": (back * 0.6, sway * 0.5, 0), "extra_coat_2": (back * 0.4, sway * 0.4, 0)}


def STAFF(out, fwd, up):
    """Where the staff points from her right hand (character space)."""
    return {"hand_r": clips.aim(out, fwd, up, "r")}


STAND = merge(sym("thigh", -3, 0, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"spine_01": (2, 0, 0)})
CROUCH = merge(sym("thigh", -14, 0, 0), sym("calf", 26, 0, 0), sym("foot", -12, 0, 0), {"spine_01": (8, 0, 0), "spine_02": (2, 0, 0)})
PLANTED = merge(arms((0.3, 0.1, -1), (0.25, 0.3, -1), (0.4, 0.15, -0.9), (0.2, 0.5, -0.1)), STAFF(0.05, 0.05, 1))
READY = merge(CROUCH, arms((0.3, 0.5, -0.6), (0.1, 1, 0.1), (0.4, 0.4, -0.7), (0.1, 0.8, 0.3)), STAFF(0.1, 0.4, 0.9), coat(4))
DOWN = {"pelvis": (0, 0, -0.05)}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, PLANTED, coat(0), {"head": (-2, 0, 0)})
    b = merge(STAND, PLANTED, coat(1.5, 2), sym("clavicle", 0, -3, 0), {"head": (-6, 0, 6)})
    return _clip(arm, "idle", 70, [(0, a, None), (35, b, {"pelvis": (0, 0, 0.005)})], loop=True)


def idle_fidget_1(arm):
    # Pulls her goggles down, peers at the sky, pushes them back up.
    base = merge(STAND, PLANTED)
    look = merge(STAND, PLANTED, arm_l(0.3, 0.4, 0.4, -0.4, 0.2, 0.9), {"head": (-26, 0, 0)})
    keys = [(0, base, None), (12, merge(STAND, PLANTED, arm_l(0.3, 0.4, 0.4, -0.4, 0.2, 0.9)), None), (22, look, None), (40, merge(look, {"head": (-30, 0, 18)}), None),
            (50, merge(STAND, PLANTED, arm_l(0.3, 0.4, 0.4, -0.4, 0.2, 0.9)), None), (62, base, None)]
    return _clip(arm, "idle_fidget_1", 62, keys)


def idle_fidget_2(arm):
    # Taps the staff's rings to set them turning.
    base = merge(STAND, PLANTED)
    tap = merge(STAND, PLANTED, arm_l(0.2, 0.5, 0.3, -0.6, 0.4, 0.7), {"head": (-12, 0, -10)})
    keys = [(0, base, None), (12, tap, None), (16, merge(tap, arm_l(0.2, 0.5, 0.35, -0.5, 0.5, 0.7)), None), (20, tap, None), (36, base, None)]
    return _clip(arm, "idle_fidget_2", 36, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (9, 0, 0)}, coat(6, 1)), {"pelvis": (0, 0, -0.06)})]
    return _clip(arm, "idle_ready", 40, keys, loop=True)


def run(arm):
    keys = []
    for f, p, l in cycle(run_half(), 20):
        p = {k: v for k, v in p.items() if not (k.endswith("_r") and k.split("_")[0] in ("upperarm", "forearm"))}
        keys.append((f, merge(p, arm_r(0.3, 0.2, -0.9, 0.1, 0.8, 0.0), STAFF(0.05, 0.5, 0.85), coat(24 + (4 if f % 10 < 5 else 0))), l))
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(arm):
    def scale(pose):
        out = {k: ((v[0] * 1.2, v[1], v[2]) if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple) else v) for k, v in pose.items()}
        out["spine_01"] = (14, 0, pose.get("spine_01", (0, 0, 0))[2])
        return {k: v for k, v in out.items() if not (k.endswith("_r") and k.split("_")[0] in ("upperarm", "forearm"))}

    half = [(round(f * 0.8), scale(p), l) for f, p, l in run_half()]
    keys = [(f, merge(p, arm_r(0.3, 0.2, -0.9, 0.1, 0.8, 0.0), STAFF(0.05, 0.6, 0.8), coat(34)), l) for f, p, l in cycle(half, 16)]
    return _clip(arm, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _point(arm, name, frames, fire, lift, lean):
    """She levels the staff at the target and the star fires on `fire`."""
    aim = merge(CROUCH, arm_r(0.2, 0.8, 0.2 + lift, 0.0, 1, 0.2 + lift), arm_l(0.2, 0.8, 0.0, -0.3, 0.9, 0.1), STAFF(0.0, 1, 0.2 + lift))
    keys = [(0, READY, DOWN), (fire - 4, merge(aim, {"spine_02": (-4 - lean, 0, -8)}, coat(3)), DOWN),
            (fire, merge(aim, {"spine_02": (4 + lean, 0, 6)}, coat(8, -2)), {"pelvis": (0, 0.03, -0.05)}),
            (fire + 6, merge(aim, {"spine_02": (-6, 0, 0)}, coat(5)), {"pelvis": (0, 0.05, -0.04)}), (frames, READY, DOWN)]
    return _clip(arm, name, frames, keys, markers={"fire": fire})


def attack_1(arm):
    return _point(arm, "attack_1", 26, 11, 0.0, 0)


def attack_2(arm):
    return _point(arm, "attack_2", 26, 11, -0.15, 4)


def q(arm):
    # Arc Shot: the staff swept up and over, lobbing the star (`fire` 8).
    up = merge(CROUCH, arm_r(0.3, 0.1, 0.9, 0.1, 0.0, 1), arm_l(0.3, 0.4, -0.5, 0.0, 0.8, 0.2), STAFF(0.0, -0.3, 1), {"spine_02": (-10, 0, 0)})
    over = merge(CROUCH, arm_r(0.2, 0.8, 0.5, 0.0, 0.9, 0.5), arm_l(0.3, 0.4, -0.5, 0.0, 0.8, 0.2), STAFF(0.0, 0.8, 0.6), {"spine_02": (6, 0, 0)})
    keys = [(0, READY, DOWN), (4, up, DOWN), (8, over, {"pelvis": (0, -0.03, -0.05)}), (14, over, DOWN), (22, READY, DOWN)]
    return _clip(arm, "q", 22, keys, markers={"fire": 8})


def w(arm):
    # Static Field: the staff's foot struck on the ground, the field sent out ahead (`fire` 8).
    lift = merge(CROUCH, arm_r(0.3, 0.3, 0.3, 0.1, 0.5, 0.6), arm_l(0.3, 0.5, -0.3, 0.0, 0.9, 0.3), STAFF(0.0, 0.2, 1), {"spine_02": (-6, 0, 0)})
    strike = merge(CROUCH, arm_r(0.3, 0.6, -0.6, 0.1, 0.9, -0.2), arm_l(0.2, 0.9, 0.1, 0.0, 1, 0.2), STAFF(0.1, -0.2, 1), {"spine_01": (14, 0, 0)})
    keys = [(0, READY, DOWN), (5, lift, {"pelvis": (0, 0, 0.02)}), (8, strike, {"pelvis": (0, 0, -0.1)}), (14, strike, {"pelvis": (0, 0, -0.1)}), (22, READY, DOWN)]
    return _clip(arm, "w", 22, keys, markers={"fire": 8})


def e_trio(arm):
    # Recoil: a hop backward, coat flaring, landing in a crouch.
    crouch = merge(sym("thigh", -35, 0, 0), sym("calf", 60, 0, 0), sym("foot", -25, 0, 0), arms((0.4, 0.5, -0.4), (0.2, 0.8, 0.0)), STAFF(0.2, 0.5, 0.8), coat(4))
    air = merge(sym("thigh", -50, 0, 0), sym("calf", 70, 0, 0), arms((0.7, 0.3, 0.1), (0.6, 0.4, 0.3)), STAFF(0.3, 0.6, 0.7), {"spine_01": (-12, 0, 0)}, coat(-20, 4))
    land = merge(sym("thigh", -45, 0, 0), sym("calf", 80, 0, 0), sym("foot", -28, 0, 0), arms((0.6, 0.4, -0.2), (0.4, 0.7, 0.0)), STAFF(0.1, 0.4, 0.9), coat(12))
    start = _clip(arm, "e_start", 4, [(0, crouch, {"pelvis": (0, 0, -0.18)}), (4, air, {"pelvis": (0, 0, 0.05)})], markers={"fire": 2})
    trav = _clip(arm, "e_travel", 10, [(0, air, {"pelvis": (0, 0, 0.06)}), (5, merge(air, coat(-26, 6)), {"pelvis": (0, 0, 0.1)})], loop=True)
    end = _clip(arm, "e_land", 10, [(0, land, {"pelvis": (0, 0, -0.32)}), (5, merge(CROUCH, READY), {"pelvis": (0, 0, -0.12)}), (10, READY, DOWN)])
    return start, trav, end


def r(arm):
    # Starfall Lance: she raises the staff to the sky for a long beat as the star gathers, then
    # brings it down level and the lance flies (`fire` 18, sim 18).
    sky = merge(STAND, arm_r(0.15, 0.1, 1, 0.05, 0.1, 1), arm_l(0.3, 0.2, 0.9, 0.1, 0.3, 1), STAFF(0.0, 0.0, 1), {"head": (-24, 0, 0), "spine_02": (-10, 0, 0)}, coat(2))
    level = merge(CROUCH, arm_r(0.1, 1, 0.05, 0.0, 1, 0.05), arm_l(0.1, 1, 0.0, -0.2, 0.95, 0.05), STAFF(0.0, 1, 0.0), {"spine_01": (10, 0, 0)}, coat(12, -3))
    keys = [(0, READY, DOWN), (8, sky, {"pelvis": (0, 0, 0.02)}), (15, merge(sky, {"head": (-28, 0, 0)}), {"pelvis": (0, 0, 0.03)}),
            (18, level, {"pelvis": (0, 0.06, -0.08)}), (26, level, {"pelvis": (0, 0.06, -0.08)}), (36, READY, DOWN)]
    return _clip(arm, "r", 36, keys, markers={"fire": 18})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (8, 0, 0), "head": (-20, 0, 0)},
              arms((0.2, 0.6, -0.5), (0.0, 0.9, 0.2), (0.35, 0.3, -0.8), (0.1, 0.6, 0.1)), STAFF(0.0, 0.1, 1), coat(-8))


def recall(arm):
    # Kneels and studies the sky through her staff's rings.
    keys = [(0, merge(STAND, PLANTED), None), (15, KNEEL, {"pelvis": (0, 0, -0.45)}), (45, merge(KNEEL, {"head": (-26, 0, 6)}), {"pelvis": (0, 0, -0.44)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.45)}), (90, merge(STAND, PLANTED), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    keys = [
        (0, merge(STAND, PLANTED), None),
        (6, merge(arms((0.6, -0.2, 0.3), (0.5, 0.1, 0.5)), STAFF(0.6, -0.2, 0.7), {"spine_01": (-15, 0, 0), "head": (-20, 0, 0)}, coat(-10)), {"pelvis": (0, 0.04, 0)}),
        (14, merge(sym("thigh", 0, 0, 0), sym("calf", 90, 0, 0), arms((0.2, 0.4, -1), (0.15, 0.5, -1)), STAFF(0.4, 0.4, -0.8), {"spine_01": (15, 0, 0), "head": (25, 0, 0)}, coat(5)),
         {"pelvis": (0, 0, -0.45)}),
        (24, merge(sym("thigh", -20, 0, 0), sym("calf", 70, 0, 0), arms((0.5, 0.2, -0.6), (0.4, 0.3, -0.7)), STAFF(0.8, 0.2, -0.4), {"pelvis": (0, 60, 0), "spine_01": (5, 15, 0)},
                   coat(10, 10)), {"pelvis": (0.25, 0, -0.65)}),
        (34, merge(sym("thigh", -30, 0, 0), sym("calf", 50, 0, 0), arms((0.7, 0.1, -0.3), (0.6, 0.2, -0.3)), STAFF(1, 0.1, 0.0), {"pelvis": (0, 85, 0), "head": (0, 10, 0)},
                   coat(20, 15)), {"pelvis": (0.45, 0, -0.82)}),
    ]
    return _clip(arm, "death", 34, keys)


def respawn(arm):
    low = merge(KNEEL)
    keys = [(0, low, {"pelvis": (0, 0, -0.45)}), (12, merge(STAND, PLANTED, {"head": (-10, 0, 0)}, coat(8)), {"pelvis": (0, 0, -0.05)}), (24, merge(STAND, PLANTED), None)]
    return _clip(arm, "respawn", 24, keys)


def select(arm):
    # Raises the staff to the sky and lets the rings spin.
    sky = merge(STAND, arm_r(0.15, 0.1, 1, 0.05, 0.1, 1), STAFF(0.0, 0.0, 1), arm_l(0.6, 0.3, -0.3, 0.6, 0.5, 0.2), {"head": (-24, 0, 0)}, coat(4))
    keys = [(0, merge(STAND, PLANTED), None), (12, sky, None), (34, merge(sky, {"head": (-28, 0, 8)}), None), (45, merge(STAND, PLANTED), None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, PLANTED)
    out = merge(base, arm_l(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-5, 0, 0)})
    curl = merge(base, arm_l(0.2, 1, 0.0, 0.1, 0.5, 0.8), {"head": (-5, 0, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Counts the stars on her fingers, loses count, starts again.
    base = merge(STAND, PLANTED)
    keys = [(0, base, None)]
    for i in range(5):
        keys.append((6 + 6 * i, merge(STAND, PLANTED, arm_l(0.4, 0.3, 0.6 + 0.08 * (i % 2), 0.0, 0.2, 1), {"head": (-24, 0, -8 + 4 * i)}), None))
    keys += [(38, merge(base, {"head": (6, 0, 0)}, sym("clavicle", 0, -6, 0)), None), (50, base, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    belly = arm_l(0.15, 0.3, -0.9, -0.7, 0.6, 0.2)
    keys = [(0, merge(STAND, PLANTED), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, PLANTED, belly, {"spine_01": (8 * s, 0, 0), "head": (15 * s, 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, merge(STAND, PLANTED), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    # Spins with the staff held out like a dance partner.
    keys = [(i * 12, merge(STAND, arm_r(0.8, 0.3, 0.0, 0.8, 0.3, 0.2), STAFF(0.6, 0.0, 0.8), arm_l(0.8, 0.1, 0.3, 0.8, 0.2, 0.5),
                         {"pelvis": (0, 0, 90 * i)}, coat(10, 6)), {"pelvis": (0, 0, 0.02 if i % 2 else 0.0)}) for i in range(4)]
    return _clip(arm, "emote_dance", 48, keys + [(48, keys[0][1], keys[0][2])], loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e_trio, r,
       recall, death, respawn, select, emote_taunt, emote_joke, emote_laugh, emote_dance]
SHARED = ["walk", "cast_utility", "attack_melee_alt", "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed",
          "cc_sleep", "cc_forced_move"]


def bake_shared(arm):
    """Copy the shared library's clips in (10 §7.3): the same skeleton, so a direct copy."""
    path = os.path.join(ROOT, "art", "library", "biped", "biped_library.blend")
    with bpy.data.libraries.load(path, link=False) as (src, dst):
        dst.actions = [n for n in src.actions if n in SHARED]
    for act in dst.actions:
        act.use_fake_user = True
        clips.stash(arm, act)
    return [a.name for a in dst.actions]


def build():
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig.use_shape("standard")
    arm = rig.build_armature(name="quill")
    rig.add_chain(arm, "coat", "pelvis", COAT_TAIL)
    for f in OWN:
        f(arm)
    body = mesh.build_mannequin(arm, name="quill", part_list=parts())
    shared = bake_shared(arm)
    print(f"quill: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(bpy.data.actions)} actions, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
