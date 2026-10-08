# SPDX-License-Identifier: AGPL-3.0-or-later
"""Marrow (A13): the block-out generator for her model and clips.

    blender -b --python art/champions/marrow/build.py -- --out art/champions/marrow/marrow.blend

A bone-witch on `biped` v1: pale, with long black hair under a crown of bone spikes, ash-plum robes
with a ragged skirt, a corset of bone ribs over the bodice, skull-capped pauldrons, a tattered
shawl on an `extra_cloak` chain, and grave-green light at her wrists (she casts from bare hands,
no staff). A team-colored sash and hem. Like the other block-outs this is a starting point.
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

# Palette (concept.md): ash and plum, bone, a grave-green light.
SKIN = (0.82, 0.76, 0.74)
HAIR = (0.08, 0.07, 0.09)
ROBE = (0.2, 0.09, 0.19)
ROBE_DARK = (0.11, 0.05, 0.11)
SOCKET = (0.08, 0.06, 0.06)
WRAP = (0.3, 0.26, 0.24)
BONE = (0.86, 0.82, 0.7)
BONE_DARK = (0.66, 0.62, 0.52)
GLOW = (0.55, 1.0, 0.5)
ACCENT = (0.18, 0.52, 0.95)

SHAWL = [(0, 0.12, 1.52), (0, 0.17, 1.22), (0, 0.2, 0.92)]


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    c = [
        # The robe: a fitted bodice, then a ragged skirt to the shins.
        ("pelvis", (0, 0, 1.12), (0, 0, 0.6), 12, [(0, .15, .11), (.5, .2, .15), (1, .24, .18)], "cloth", ROBE),
        ("pelvis", (0, 0, 0.62), (0, 0, 0.56), 12, [(0, .242, .182), (1, .245, .185)], "accent", ACCENT),
        ("pelvis", (0, 0, 1.08), (0, 0, 1.14), 12, [(0, .156, .114), (1, .152, .111)], "accent", ACCENT),
        ("spine_01", (0, 0, 1.10), (0, 0, 1.28), 12, [(0, .13, .093), (.5, .112, .084), (1, .12, .088)], "cloth", ROBE_DARK),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .122, .088), (1, .145, .1)], "cloth", ROBE_DARK),
        ("chest", (0, 0, 1.40), (0, 0, 1.60), 12, [(0, .145, .1), (.35, .152, .106), (.7, .145, .1), (1, .085, .07)], "cloth", ROBE),
        ("chest", (0, -0.03, 1.415), (0, -0.035, 1.53), 10, [(0, .105, .052), (.5, .124, .066), (1, .098, .046)], "cloth", ROBE),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.66), 8, [(0, .046, .046), (1, .042, .042)], "skin", SKIN),
        # A bone choker with a green bead.
        ("neck", (0, 0, 1.585), (0, -0.002, 1.61), 8, [(0, .05, .05), (1, .05, .05)], "cloth", BONE),
        ("neck", (0, -0.05, 1.59), (0, -0.07, 1.59), 4, [(0, .014, .014), (.5, .014, .014), (1, 0, 0)], "emissive", GLOW),
        # The shawl on its chain, ragged at the end.
        ("extra_cloak_1", SHAWL[0], SHAWL[1], 6, [(0, .17, .02), (1, .2, .02)], "cloth", ROBE_DARK),
        ("extra_cloak_2", SHAWL[1], SHAWL[2], 6, [(0, .2, .02), (.8, .19, .018), (1, .12, .014)], "cloth", ROBE_DARK),
    ]
    # Ragged strips hanging from the skirt's hem, of uneven length.
    for i, (a, drop) in enumerate(((-0.45, 0.2), (0.1, 0.14), (0.8, 0.22), (1.6, 0.16), (2.3, 0.2), (3.0, 0.12), (3.8, 0.18), (4.5, 0.15), (5.4, 0.21))):
        x, y = 0.235 * math.cos(a), 0.175 * math.sin(a)
        c.append(("pelvis", (x, y, 0.58), (x * 1.06, y * 1.06, 0.58 - drop), 4, [(0, .035, .006), (1, .02, .004)], "cloth", ROBE if i % 2 else ROBE_DARK))
    # The bone corset: a sternum and three pairs of ribs curving round the bodice.
    c.append(("chest", (0, -0.112, 1.52), (0, -0.112, 1.33), 4, [(0, .012, .008), (1, .01, .007)], "cloth", BONE))
    for s in (1, -1):
        for z, w in ((1.5, 0.11), (1.44, 0.115), (1.38, 0.11)):
            c.append(("chest" if z > 1.41 else "spine_02", (0.008 * s, -0.112, z), (w * s, -0.06, z - 0.05), 4, [(0, .009, .007), (1, .007, .005)], "cloth", BONE))
    return c


def _left():
    ua, fa, ca = "upperarm_l", "forearm_l", "calf_l"
    return [
        # A skull-capped pauldron: a bone dome on the shoulder.
        (ua, (0.13, 0.0, 1.5), (0.22, 0.0, 1.58), 8, [(0, .07, .068), (.55, .075, .07), (1, 0, 0)], "cloth", BONE),
        # Its eye sockets, looking forward.
        (ua, (0.152, -0.058, 1.556), (0.152, -0.068, 1.556), 4, [(0, .013, .011), (1, .012, .01)], "cloth", SOCKET),
        (ua, (0.198, -0.058, 1.556), (0.198, -0.068, 1.556), 4, [(0, .013, .011), (1, .012, .01)], "cloth", SOCKET),
        (ua, (ua, 0), (ua, 1), 8, [(0, .05, .05), (1, .042, .042)], "cloth", ROBE),
        # Tight sleeves, ragged at the wrist, over grave-green light.
        (fa, (fa, 0), (fa, .84), 8, [(0, .044, .044), (.7, .05, .05), (1, .056, .056)], "cloth", ROBE_DARK),
        (fa, (fa, .82), (fa, .96), 8, [(0, .04, .04), (1, .038, .038)], "emissive", GLOW),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .034, .021), (1, .038, .019)], "skin", SKIN),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .038, .019), (1, .025, .012)], "skin", SKIN),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .015, .015), (1, .01, .01)], "skin", SKIN),
        # Wrapped shins and feet under the skirt.
        (ca, (ca, .35), (ca, 1), 8, [(0, .052, .052), (1, .042, .042)], "cloth", WRAP),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .046, .04), (1, .04, .028)], "cloth", WRAP),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .04, .024), (1, .026, .014)], "cloth", WRAP),
    ]


def _prism(bm, a, b, r0):
    """A small 5-sided spike from `a` to a point at `b` (the crown)."""
    import bmesh
    a, b = Vector(a), Vector(b)
    axis = (b - a).normalized()
    ref = Vector((1, 0, 0)) if abs(axis.x) < 0.9 else Vector((0, 0, 1))
    u = (ref - axis * ref.dot(axis)).normalized()
    v = axis.cross(u)
    ring = [bm.verts.new(a + (u * math.cos(2 * math.pi * k / 5) + v * math.sin(2 * math.pi * k / 5)) * r0) for k in range(5)]
    tip = bm.verts.new(b)
    faces = [bm.faces.new((ring[k], ring[(k + 1) % 5], tip)) for k in range(5)]
    faces.append(bm.faces.new(list(reversed(ring))))
    bmesh.ops.recalc_face_normals(bm, faces=faces)
    return ring + [tip], faces


def _head(bm, layers, groups, mats):
    # The shared sculpted head, feminine and gaunt: long black hair, grave-green eyes, and a crown
    # of bone spikes, tallest at the front.
    hd = head.Head(base=(0, -0.008, 1.64), height=0.25, scale=0.92, jaw=0.52, chin=0.84, eye=1.1,
                   skin=SKIN, hair=HAIR, hair_style="ponytail", eye_color=(0.4, 0.85, 0.4), lips=(0.5, 0.32, 0.4))
    head.build(bm, layers, groups, mats, hd)
    deform, col = layers
    n = 9
    for i in range(n):
        a = -math.pi / 2 + (i - (n - 1) / 2) * (math.pi / (n - 1)) * 1.4
        root = hd.surface(0.8, a)
        out = (root - (hd.base + Vector((0, 0.01, 0.2)))).normalized()
        tall = 0.13 - 0.018 * abs(i - (n - 1) / 2)
        tip = root + out * 0.03 + Vector((0, 0, tall))
        vs, fs = _prism(bm, root, tip, 0.016)
        for v in vs:
            v[deform][groups["head"]] = 1.0
            v[col] = (*(BONE if i % 2 == 0 else BONE_DARK), 1.0)
        for f in fs:
            f.material_index = mats["cloth"]
            f.smooth = False
    # The band the spikes stand on.
    ring = [bm.verts.new(hd.surface(0.78, -math.pi / 2 + (k / 11 - 0.5) * math.pi * 1.5) + Vector((0, 0, 0))) for k in range(12)]
    top = [bm.verts.new(v.co + Vector((0, 0, 0.022))) for v in ring]
    faces = [bm.faces.new((ring[k], ring[k + 1], top[k + 1], top[k])) for k in range(11)]
    import bmesh
    bmesh.ops.recalc_face_normals(bm, faces=faces)
    for v in ring + top:
        v.co = hd.base + (v.co - hd.base) * Vector((1.06, 1.06, 1.0)) + Vector((0, 0, 0))
        v[deform][groups["head"]] = 1.0
        v[col] = (*BONE_DARK, 1.0)
    for f in faces:
        f.material_index = mats["cloth"]
        f.smooth = False


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + [_head]


# --- Poses ----------------------------------------------------------------------------------

def shawl(back=0.0, sway=0.0):
    return {"extra_cloak_1": (back * 0.6, sway * 0.5, 0), "extra_cloak_2": (back * 0.4, sway * 0.4, 0)}


STAND = merge(sym("thigh", -3, 0, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"spine_01": (4, 0, 0), "head": (4, 0, 0)})
CROUCH = merge(sym("thigh", -16, 0, 0), sym("calf", 28, 0, 0), sym("foot", -12, 0, 0), {"spine_01": (10, 0, 0), "spine_02": (3, 0, 0)})
# Hands hang a little forward, fingers curled like claws.
CLAWS = {"fingers_l": (30, 0, 0), "fingers_r": (30, 0, 0)}
HANDS = merge(arms((0.3, 0.18, -1), (0.15, 0.45, -1)), CLAWS)
READY = merge(CROUCH, arms((0.45, 0.45, -0.6), (0.2, 1, 0.0)), CLAWS, shawl(4))
DOWN = {"pelvis": (0, 0, -0.05)}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, HANDS, shawl(0), {"head": (2, 0, 0)})
    b = merge(STAND, HANDS, shawl(1.5, 2), sym("clavicle", 0, -3, 0), {"head": (0, 0, 6), "chest": (-2, 0, 0)})
    return _clip(arm, "idle", 72, [(0, a, None), (36, b, {"pelvis": (0, 0, 0.005)})], loop=True)


def idle_fidget_1(arm):
    # Rolls a knucklebone across her knuckles and watches it.
    base = merge(STAND, HANDS)
    look = merge(STAND, HANDS, arm_r(0.3, 0.4, -0.7, 0.0, 0.9, 0.3), {"head": (14, 0, -10)})
    keys = [(0, base, None), (10, look, None)]
    for i in range(4):
        keys.append((14 + 6 * i, merge(look, {"hand_r": (0, 0, 20 if i % 2 else -20)}), None))
    keys += [(40, look, None), (52, base, None)]
    return _clip(arm, "idle_fidget_1", 52, keys)


def idle_fidget_2(arm):
    # Cracks her neck to one side, then the other.
    base = merge(STAND, HANDS)
    keys = [(0, base, None), (10, merge(base, {"head": (2, -22, 0)}), None), (14, merge(base, {"head": (2, -26, 0)}), None),
            (24, merge(base, {"head": (2, 22, 0)}), None), (28, merge(base, {"head": (2, 26, 0)}), None), (40, base, None)]
    return _clip(arm, "idle_fidget_2", 40, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (13, 0, 0)}, shawl(6, 1)), {"pelvis": (0, 0, -0.07)})]
    return _clip(arm, "idle_ready", 40, keys, loop=True)


def run(arm):
    keys = [(f, merge(p, CLAWS, shawl(22 + (4 if f % 10 < 5 else 0))), l) for f, p, l in cycle(run_half(), 20)]
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(arm):
    def scale(pose):
        out = dict(pose)
        for k, v in pose.items():
            if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple):
                out[k] = (v[0] * 1.2, v[1], v[2])
        out["spine_01"] = (16, 0, pose.get("spine_01", (0, 0, 0))[2])
        return out

    half = [(round(f * 0.8), scale(p), l) for f, p, l in run_half()]
    keys = [(f, merge(p, CLAWS, shawl(32)), l) for f, p, l in cycle(half, 16)]
    return _clip(arm, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _flick(arm, name, frames, fire, wind, release, follow):
    keys = [(0, READY, DOWN), (fire - 4, merge(CROUCH, wind, CLAWS, shawl(3)), DOWN), (fire, merge(CROUCH, release, shawl(6, -2)), {"pelvis": (0, -0.03, -0.05)}),
            (fire + 5, merge(CROUCH, follow, CLAWS, shawl(5)), DOWN), (frames, READY, DOWN)]
    return _clip(arm, name, frames, keys, markers={"fire": fire})


def attack_1(arm):
    # A clawed backhand flinging a green bolt (`fire` 10, sim 10.2).
    wind = merge(arm_r(0.2, 0.4, 0.3, -0.6, 0.4, 0.4), arm_l(0.4, 0.4, -0.6, 0.1, 0.9, 0.0), {"spine_02": (0, 0, 16)})
    release = merge(arm_r(0.5, 0.8, 0.1, 0.4, 0.9, 0.0), arm_l(0.4, -0.1, -0.8, 0.3, 0.2, -1), {"spine_02": (4, 0, -14)})
    follow = merge(arm_r(0.7, 0.5, -0.1, 0.6, 0.6, -0.2), HANDS, {"spine_02": (2, 0, -8)})
    return _flick(arm, "attack_1", 26, 10, wind, release, follow)


def attack_2(arm):
    # An open palm thrust.
    wind = merge(arm_r(0.4, -0.4, 0.1, -0.1, 0.4, 0.9), arm_l(0.4, 0.4, -0.6, 0.1, 0.9, 0.0), {"spine_02": (0, 0, -16)})
    release = merge(arm_r(0.05, 1, 0.1, 0.0, 1, 0.15), arm_l(0.4, -0.2, -0.8, 0.3, 0.1, -1), {"spine_02": (4, 0, 12)})
    follow = merge(arm_r(0.1, 0.9, -0.2, 0.0, 0.9, -0.3), HANDS, {"spine_02": (2, 0, 6)})
    return _flick(arm, "attack_2", 26, 10, wind, release, follow)


def q(arm):
    # Siphon: claws flung wide, then dragged in to her chest, drawing the life around her
    # (`fire` 6, sim 6).
    wide = merge(CROUCH, arms((1, 0.4, 0.2), (0.9, 0.6, 0.2)), CLAWS, {"spine_02": (-8, 0, 0), "head": (-10, 0, 0)}, shawl(8))
    pull = merge(CROUCH, arms((0.4, 0.5, -0.4), (-0.6, 0.5, 0.4)), CLAWS, {"spine_01": (14, 0, 0), "head": (8, 0, 0)}, shawl(2))
    keys = [(0, READY, DOWN), (3, wide, {"pelvis": (0, 0, -0.02)}), (6, pull, {"pelvis": (0, 0, -0.09)}), (12, pull, {"pelvis": (0, 0, -0.08)}),
            (20, READY, DOWN)]
    return _clip(arm, "q", 20, keys, markers={"fire": 6})


def w(arm):
    # Grasping Bones: she drops to a knee and drives a clawed hand into the ground; bones burst
    # up along the line (`fire` 9).
    raise_ = merge(CROUCH, arm_r(0.3, 0.2, 0.8, 0.1, 0.4, 0.9), arm_l(0.4, 0.4, -0.5, 0.1, 0.9, 0.0), CLAWS, {"spine_02": (-6, 0, 0)})
    slam = merge(sym("thigh", -40, 0, 0), sym("calf", 70, 0, 0), sym("foot", -24, 0, 0), arm_r(0.1, 0.8, -0.6, 0.0, 0.7, -0.8),
                 arm_l(0.5, 0.2, -0.6, 0.3, 0.4, -0.7), CLAWS, {"spine_01": (26, 0, 0), "spine_02": (8, 0, 0)}, shawl(-12))
    keys = [(0, READY, DOWN), (5, raise_, {"pelvis": (0, 0, 0.0)}), (9, slam, {"pelvis": (0, -0.04, -0.28)}), (16, slam, {"pelvis": (0, -0.04, -0.28)}),
            (26, READY, DOWN)]
    return _clip(arm, "w", 26, keys, markers={"fire": 9})


def e(arm):
    # Grave Pact: instant, a pulse from `fire` 4 on the upper body: a hand clutched to her chest,
    # her head thrown back as the green light floods in.
    pact = merge(arm_l(0.3, 0.5, -0.4, -0.7, 0.5, 0.3), {"spine_02": (-6, 0, 0), "head": (-22, 0, 0)}, CLAWS)
    keys = [(0, READY, DOWN), (4, merge(READY, pact), DOWN), (14, merge(READY, pact), DOWN), (22, READY, DOWN)]
    return _clip(arm, "e", 22, keys, layer="upper", markers={"fire": 4})


def r(arm):
    # Ossuary: both arms raised, then thrust down toward the far ground where the bone-pit opens
    # (`fire` 8).
    raise_ = merge(STAND, arms((0.5, 0.3, 0.8), (0.3, 0.4, 0.9)), CLAWS, {"spine_02": (-10, 0, 0), "head": (-16, 0, 0)}, shawl(6))
    thrust = merge(CROUCH, arms((0.3, 0.9, -0.3), (0.2, 1, -0.3)), {"spine_01": (14, 0, 0), "spine_02": (8, 0, 0)}, shawl(-4))
    keys = [(0, READY, DOWN), (5, raise_, {"pelvis": (0, 0, 0.03)}), (8, thrust, {"pelvis": (0, -0.05, -0.08)}), (16, thrust, {"pelvis": (0, -0.05, -0.08)}),
            (28, READY, DOWN)]
    return _clip(arm, "r", 28, keys, markers={"fire": 8})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (10, 0, 0), "head": (20, 0, 0)},
              arms((0.3, 0.4, -0.8), (0.2, 0.6, -0.7)), CLAWS, shawl(-8))


def recall(arm):
    # Kneels and lays both palms flat on the ground, drawing herself home through it.
    breathe = merge(KNEEL, sym("clavicle", 0, -4, 0), {"head": (26, 0, 0)})
    keys = [(0, merge(STAND, HANDS), None), (15, KNEEL, {"pelvis": (0, 0, -0.45)}), (45, breathe, {"pelvis": (0, 0, -0.44)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.45)}), (90, merge(STAND, HANDS, shawl(6)), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    keys = [
        (0, merge(STAND, HANDS), None),
        (6, merge(arms((0.6, -0.2, 0.3), (0.5, 0.1, 0.5)), {"spine_01": (-15, 0, 0), "head": (-24, 0, 0)}, shawl(-10)), {"pelvis": (0, 0.04, 0)}),
        (14, merge(sym("thigh", 0, 0, 0), sym("calf", 90, 0, 0), arms((0.2, 0.4, -1), (0.15, 0.5, -1)), {"spine_01": (15, 0, 0), "head": (25, 0, 0)}, shawl(5)),
         {"pelvis": (0, 0, -0.45)}),
        (24, merge(sym("thigh", -20, 0, 0), sym("calf", 70, 0, 0), arms((0.5, 0.2, -0.6), (0.4, 0.3, -0.7)), {"pelvis": (0, -60, 0), "spine_01": (5, -15, 0)},
                   shawl(10, -10)), {"pelvis": (-0.25, 0, -0.65)}),
        (34, merge(sym("thigh", -30, 0, 0), sym("calf", 50, 0, 0), arms((0.7, 0.1, -0.3), (0.6, 0.2, -0.3)), {"pelvis": (0, -85, 0), "head": (0, -10, 0)},
                   shawl(20, -15)), {"pelvis": (-0.45, 0, -0.82)}),
    ]
    return _clip(arm, "death", 34, keys)


def respawn(arm):
    # Claws her way up from a crouch.
    low = merge(sym("thigh", -50, 0, 0), sym("calf", 90, 0, 0), sym("foot", -30, 0, 0), arms((0.3, 0.6, -0.7), (0.2, 0.7, -0.7)), CLAWS,
                {"spine_01": (30, 0, 0)}, shawl(-5))
    keys = [(0, low, {"pelvis": (0, 0, -0.32)}), (12, merge(STAND, arms((0.6, 0.4, 0.2), (0.6, 0.5, 0.3)), CLAWS, {"head": (-14, 0, 0)}, shawl(8)), {"pelvis": (0, 0, -0.06)}),
            (24, merge(STAND, HANDS), None)]
    return _clip(arm, "respawn", 24, keys)


def select(arm):
    # Rises from a hunch, one claw lifted to her face, a slow crooked smile.
    hunch = merge(CROUCH, HANDS, {"spine_01": (20, 0, 0), "head": (20, 0, 0)})
    claw = merge(STAND, HANDS, arm_r(0.2, 0.5, 0.2, -0.3, 0.6, 0.7), {"head": (-8, 0, 10)})
    keys = [(0, merge(STAND, HANDS), None), (8, hunch, {"pelvis": (0, 0, -0.06)}), (22, claw, None), (36, merge(claw, {"head": (-10, 0, 16)}), None),
            (45, merge(STAND, HANDS), None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, HANDS)
    out = merge(base, arm_r(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-5, 0, 0)})
    curl = merge(base, arm_r(0.2, 1, 0.0, 0.1, 0.5, 0.8), {"head": (-5, 0, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Takes a rib from her corset, snaps it like a wishbone, and puts the halves back.
    base = merge(STAND, HANDS)
    hold = merge(STAND, arms((0.2, 0.5, -0.3), (-0.4, 0.6, 0.4)), {"head": (14, 0, 0)})
    snap = merge(STAND, arms((0.35, 0.5, -0.3), (0.3, 0.6, 0.4)), {"head": (10, 0, 0)})
    keys = [(0, base, None), (12, hold, None), (20, hold, None), (24, snap, None), (32, snap, None), (40, hold, None), (50, base, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    belly = arm_l(0.15, 0.3, -0.9, -0.7, 0.6, 0.2)
    keys = [(0, merge(STAND, HANDS), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, HANDS, belly, {"spine_01": (-8 * abs(s), 0, 0), "head": (-18 * abs(s), 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, merge(STAND, HANDS), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    # A swaying, puppet-jointed shuffle.
    a = merge(arms((0.7, 0.2, 0.3), (0.7, 0.2, -0.6), (0.4, 0.2, -0.8), (0.4, 0.3, 0.6)), CLAWS, {"thigh_l": (-24, 0, 0), "calf_l": (36, 0, 0), "pelvis": (0, -8, 16),
                                                                                             "head": (0, -14, 0)}, shawl(0, -8))
    mid = merge(STAND, HANDS, shawl(4))
    b = merge(arms((0.4, 0.2, -0.8), (0.4, 0.3, 0.6), (0.7, 0.2, 0.3), (0.7, 0.2, -0.6)), CLAWS, {"thigh_r": (-24, 0, 0), "calf_r": (36, 0, 0), "pelvis": (0, 8, -16),
                                                                                             "head": (0, 14, 0)}, shawl(0, 8))
    keys = [(0, a, {"pelvis": (0.06, 0, 0)}), (12, mid, {"pelvis": (0, 0, -0.04)}), (24, b, {"pelvis": (-0.06, 0, 0)}), (36, mid, {"pelvis": (0, 0, -0.04)})]
    return _clip(arm, "emote_dance", 48, keys, loop=True)


OWN = [idle, idle_fidget_1, idle_fidget_2, idle_ready, run, run_fast, attack_1, attack_2, q, w, e, r,
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
    arm = rig.build_armature(name="marrow")
    rig.add_chain(arm, "cloak", "chest", SHAWL)
    own = [f(arm).name for f in OWN]
    body = mesh.build_mannequin(arm, name="marrow", part_list=parts())
    shared = bake_shared(arm)
    print(f"marrow: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(own)} own clips, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
