# SPDX-License-Identifier: AGPL-3.0-or-later
"""Lumen (A9): the block-out generator for her model and clips.

    blender -b --python art/champions/lumen/build.py -- --out art/champions/lumen/lumen.blend

A serene light-priestess on `biped` v1: a floor-length ivory robe with gold trim and a short
capelet, a stole in the team color over her shoulders, pale gold hair in a ponytail, and a ring
of gold light floating behind her head (a halo of glowing segments) with a gem glowing at her
throat. She heals and binds from open hands; no staff. Like the other block-outs this is a
starting point: run once, then the .blend is the source of truth.
"""

import math
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402

from mftr_blender import clips, head, mesh, rig  # noqa: E402
from mftr_blender.clips import merge, sym  # noqa: E402
from mftr_blender.library import _clip, arms, cycle, run_half  # noqa: E402

# Palette (concept.md): ivory and gold, a pale light.
SKIN = (0.88, 0.72, 0.62)
HAIR = (0.86, 0.76, 0.52)
ROBE = (0.86, 0.83, 0.74)
ROBE_SHADE = (0.72, 0.69, 0.62)
GOLD = (0.86, 0.68, 0.30)
SHOE = (0.62, 0.52, 0.38)
LIGHT = (1.0, 0.88, 0.55)
ACCENT = (0.18, 0.52, 0.95)


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    c = [
        # The robe: a fitted bodice, a narrow waist, a skirt flaring to the floor in two tiers.
        ("pelvis", (0, 0, 1.12), (0, 0, 0.62), 12, [(0, .15, .11), (.5, .2, .15), (1, .25, .19)], "cloth", ROBE),
        ("pelvis", (0, 0, 0.64), (0, 0, 0.1), 12, [(0, .25, .19), (.6, .29, .22), (1, .32, .25)], "cloth", ROBE_SHADE),
        ("pelvis", (0, 0, 0.13), (0, 0, 0.08), 12, [(0, .322, .252), (1, .325, .255)], "metal", GOLD),
        ("spine_01", (0, 0, 1.10), (0, 0, 1.28), 12, [(0, .13, .093), (.5, .112, .084), (1, .12, .088)], "cloth", ROBE),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .122, .088), (1, .145, .1)], "cloth", ROBE),
        ("chest", (0, 0, 1.40), (0, 0, 1.60), 12, [(0, .145, .1), (.35, .152, .106), (.7, .145, .1), (1, .085, .07)], "cloth", ROBE),
        ("chest", (0, -0.03, 1.415), (0, -0.035, 1.53), 10, [(0, .105, .052), (.5, .128, .068), (1, .1, .046)], "cloth", ROBE),
        ("pelvis", (0, 0, 1.08), (0, 0, 1.13), 12, [(0, .155, .113), (1, .15, .11)], "metal", GOLD),
        # The capelet over her shoulders.
        ("chest", (0, 0.01, 1.46), (0, 0.005, 1.6), 14, [(0, .2, .14), (.55, .17, .125), (1, .1, .085)], "cloth", ROBE_SHADE),
        # The stole: two team-colored bands from the shoulders down the front of the robe.
        ("chest", (0.07, -0.12, 1.58), (0.08, -0.15, 0.95), 4, [(0, .03, .012), (1, .032, .012)], "accent", ACCENT),
        ("chest", (-0.07, -0.12, 1.58), (-0.08, -0.15, 0.95), 4, [(0, .03, .012), (1, .032, .012)], "accent", ACCENT),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.66), 8, [(0, .048, .048), (1, .043, .043)], "skin", SKIN),
        # A gem glowing at her throat.
        ("chest", (0, -0.085, 1.565), (0, -0.11, 1.565), 4, [(0, .02, .02), (.5, .02, .02), (1, 0, 0)], "emissive", LIGHT),
    ]
    # The halo: twelve glowing segments in a ring floating behind her head.
    centre, r, n = (0.0, 0.11, 1.84), 0.165, 12
    pts = [(centre[0] + r * math.cos(2 * math.pi * i / n), centre[1] + 0.02 * math.sin(2 * math.pi * i / n), centre[2] + r * math.sin(2 * math.pi * i / n))
           for i in range(n)]
    for i in range(n):
        c.append(("head", pts[i], pts[(i + 1) % n], 4, [(0, .014, .014), (1, .014, .014)], "emissive", LIGHT))
    return c


def _left():
    ua, fa, ca = "upperarm_l", "forearm_l", "calf_l"
    return [
        (ua, (ua, 0), (ua, 1), 8, [(0, .05, .05), (1, .042, .042)], "cloth", ROBE),
        # Wide bell sleeves, gold at the cuff.
        (fa, (fa, 0), (fa, .92), 8, [(0, .045, .045), (.6, .06, .06), (1, .08, .08)], "cloth", ROBE),
        (fa, (fa, .86), (fa, .95), 8, [(0, .082, .082), (1, .082, .082)], "metal", GOLD),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .034, .021), (1, .038, .019)], "skin", SKIN),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .038, .019), (1, .025, .013)], "skin", SKIN),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .015, .015), (1, .01, .01)], "skin", SKIN),
        # Only the lower legs and slippers show under the robe's hem.
        (ca, (ca, .55), (ca, 1), 8, [(0, .045, .045), (1, .04, .04)], "skin", SKIN),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .045, .04), (1, .04, .028)], "cloth", SHOE),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .04, .024), (1, .026, .014)], "cloth", SHOE),
    ]


def _head(bm, layers, groups, mats):
    # The shared sculpted head, feminine and calm: pale gold hair in a ponytail, pale blue eyes.
    head.build(bm, layers, groups, mats, head.Head(
        base=(0, -0.008, 1.64), height=0.25, scale=0.92, jaw=0.5, chin=0.82, eye=1.15,
        skin=SKIN, hair=HAIR, hair_style="ponytail", eye_color=(0.42, 0.62, 0.78), lips=(0.76, 0.48, 0.46)))


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + [_head]


# --- Poses ----------------------------------------------------------------------------------

STAND = merge(sym("thigh", -3, 0, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"spine_01": (1, 0, 0)})
SOFT = merge(sym("thigh", -10, 0, 0), sym("calf", 18, 0, 0), sym("foot", -8, 0, 0), {"spine_01": (5, 0, 0)})
HANDS = arms((0.25, 0.12, -1), (0.0, 0.7, -0.6))
READY = merge(SOFT, arms((0.3, 0.45, -0.7), (0.05, 1, 0.1)))
DOWN = {"pelvis": (0, 0, -0.03)}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    # Hands folded before her, a slow breath.
    a = merge(STAND, HANDS, {"head": (-2, 0, 0)})
    b = merge(STAND, HANDS, sym("clavicle", 0, -3, 0), {"head": (-4, 0, 4), "chest": (-2, 0, 0)})
    return _clip(arm, "idle", 80, [(0, a, None), (40, b, {"pelvis": (0, 0, 0.006)})], loop=True)


def idle_fidget_1(arm):
    # Cups a mote of light in her palms and lets it rise.
    base = merge(STAND, HANDS)
    cup = merge(STAND, arms((0.2, 0.5, -0.8), (-0.3, 0.9, 0.2)), {"head": (16, 0, 0)})
    rise = merge(STAND, arms((0.2, 0.6, -0.5), (-0.1, 0.6, 0.8)), {"head": (-18, 0, 0)})
    keys = [(0, base, None), (14, cup, None), (28, cup, None), (40, rise, None), (54, merge(rise, {"head": (-24, 0, 4)}), None), (68, base, None)]
    return _clip(arm, "idle_fidget_1", 68, keys)


def idle_fidget_2(arm):
    # Tucks a strand of hair behind her ear.
    base = merge(STAND, HANDS)
    tuck = merge(STAND, HANDS, arm_r(0.3, 0.2, 0.2, -0.6, -0.2, 0.75), {"head": (4, 0, 10)})
    keys = [(0, base, None), (14, tuck, None), (24, merge(tuck, {"head": (6, 0, 14)}), None), (38, base, None)]
    return _clip(arm, "idle_fidget_2", 38, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (24, merge(READY, {"spine_01": (8, 0, 0)}), {"pelvis": (0, 0, -0.04)})]
    return _clip(arm, "idle_ready", 48, keys, loop=True)


def run(arm):
    # Light, quick steps; the arms swing less (the robe).
    keys = []
    for f, pose, locs in cycle(run_half(), 20):
        p = dict(pose)
        p["spine_01"] = (6, 0, pose.get("spine_01", (0, 0, 0))[2])
        keys.append((f, p, locs))
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(arm):
    def scale(pose):
        out = dict(pose)
        for k, v in pose.items():
            if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple):
                out[k] = (v[0] * 1.2, v[1], v[2])
        out["spine_01"] = (12, 0, pose.get("spine_01", (0, 0, 0))[2])
        return out

    half = [(round(f * 0.8), scale(p), l) for f, p, l in run_half()]
    return _clip(arm, "run_fast", 16, cycle(half, 16), loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def attack_1(arm):
    # A mote of light flicked from the right palm (`fire` 9, sim 9.2).
    gather = merge(SOFT, arm_r(0.4, 0.1, -0.2, -0.3, 0.5, 0.6), arm_l(0.3, 0.4, -0.7, 0.0, 0.9, -0.2))
    flick = merge(SOFT, arm_r(0.1, 1, 0.1, 0.0, 1, 0.15), arm_l(0.3, 0.2, -0.8, 0.1, 0.5, -0.8), {"spine_02": (4, 0, 10)})
    keys = [(0, READY, DOWN), (5, gather, DOWN), (9, flick, {"pelvis": (0, -0.02, -0.03)}), (14, merge(flick, {"spine_02": (2, 0, 6)}), DOWN), (24, READY, DOWN)]
    return _clip(arm, "attack_1", 24, keys, markers={"fire": 9})


def attack_2(arm):
    # The left-handed variant: a backhand flick.
    gather = merge(SOFT, arm_l(0.4, 0.1, -0.2, -0.3, 0.5, 0.6), arm_r(0.3, 0.4, -0.7, 0.0, 0.9, -0.2))
    flick = merge(SOFT, arm_l(0.15, 1, 0.1, 0.05, 1, 0.15), arm_r(0.3, 0.2, -0.8, 0.1, 0.5, -0.8), {"spine_02": (4, 0, -10)})
    keys = [(0, READY, DOWN), (5, gather, DOWN), (9, flick, {"pelvis": (0, -0.02, -0.03)}), (14, merge(flick, {"spine_02": (2, 0, -6)}), DOWN), (24, READY, DOWN)]
    return _clip(arm, "attack_2", 24, keys, markers={"fire": 9})


def q(arm):
    # Mending Light: instant, a pulse from `fire` 4: palms open toward the ally, then settle.
    give = merge(SOFT, arms((0.2, 0.9, -0.1), (0.1, 0.9, 0.3)), {"spine_02": (6, 0, 0), "head": (6, 0, 0)})
    keys = [(0, READY, DOWN), (4, give, {"pelvis": (0, -0.02, -0.03)}), (14, give, {"pelvis": (0, -0.02, -0.03)}), (22, READY, DOWN)]
    return _clip(arm, "q", 22, keys, layer="upper", markers={"fire": 4})


def w(arm):
    # Aegis: instant, a pulse from `fire` 4: arms crossed at the chest, then spread wide.
    cross = merge(SOFT, arms((0.1, 0.4, -0.5), (-0.8, 0.5, 0.4)))
    spread = merge(SOFT, arms((0.9, 0.3, 0.1), (0.9, 0.4, 0.3)), {"head": (-8, 0, 0)})
    keys = [(0, READY, DOWN), (2, cross, DOWN), (4, spread, None), (14, spread, None), (22, READY, DOWN)]
    return _clip(arm, "w", 22, keys, layer="upper", markers={"fire": 4})


def e(arm):
    # Lull: a slow sweep of the right arm, a wave of calm sent forward (`fire` 8, sim 7.5).
    back = merge(SOFT, arm_r(0.8, -0.3, 0.1, 0.7, 0.2, 0.3), arm_l(0.3, 0.4, -0.7, 0.0, 0.9, -0.2), {"spine_02": (0, 0, -14)})
    sweep = merge(SOFT, arm_r(0.1, 1, 0.05, -0.2, 1, 0.1), arm_l(0.3, 0.2, -0.8, 0.1, 0.5, -0.8), {"spine_02": (4, 0, 12)})
    keys = [(0, READY, DOWN), (4, back, DOWN), (8, sweep, {"pelvis": (0, -0.02, -0.03)}), (14, sweep, DOWN), (22, READY, DOWN)]
    return _clip(arm, "e", 22, keys, markers={"fire": 8})


def r(arm):
    # Binding Halo: both arms rise to draw a circle of light overhead, then sweep it down onto
    # the target area (`fire` 8, sim 7.5).
    up = merge(STAND, arms((0.35, 0.1, 1), (0.1, 0.2, 1)), {"head": (-16, 0, 0), "spine_02": (-8, 0, 0)})
    down = merge(SOFT, arms((0.3, 0.9, -0.2), (0.2, 0.9, -0.4)), {"spine_01": (10, 0, 0), "head": (8, 0, 0)})
    keys = [(0, READY, DOWN), (5, up, {"pelvis": (0, 0, 0.03)}), (8, down, {"pelvis": (0, -0.03, -0.05)}), (16, down, DOWN), (26, READY, DOWN)]
    return _clip(arm, "r", 26, keys, markers={"fire": 8})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (4, 0, 0), "head": (20, 0, 0)},
              arms((0.15, 0.6, -0.4), (-0.6, 0.6, 0.4)))


def recall(arm):
    # Kneels in prayer, hands together.
    keys = [(0, merge(STAND, HANDS), None), (15, KNEEL, {"pelvis": (0, 0, -0.44)}), (45, merge(KNEEL, {"head": (24, 0, 0)}), {"pelvis": (0, 0, -0.43)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.44)}), (90, merge(STAND, HANDS), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    # Sinks slowly to her knees and folds to one side.
    keys = [
        (0, merge(STAND, HANDS), None),
        (8, merge(STAND, arms((0.5, 0.3, -0.4), (0.2, 0.7, 0.2)), {"spine_01": (-8, 0, 0), "head": (-16, 0, 0)}), None),
        (18, merge(sym("thigh", 0, 0, 0), sym("calf", 95, 0, 0), arms((0.2, 0.4, -1), (0.15, 0.5, -1)), {"spine_01": (10, 0, 0), "head": (24, 0, 0)}),
         {"pelvis": (0, 0, -0.46)}),
        (30, merge(sym("thigh", -20, 0, 0), sym("calf", 100, 0, 0), arms((0.6, 0.1, -0.5), (0.5, 0.2, -0.6)), {"pelvis": (0, 70, 0), "spine_01": (4, 12, 0)}),
         {"pelvis": (0.3, 0, -0.7)}),
        (38, merge(sym("thigh", -30, 0, 0), sym("calf", 80, 0, 0), arms((0.8, 0.1, -0.2), (0.7, 0.2, -0.2)), {"pelvis": (0, 86, 0), "head": (0, 10, 0)}),
         {"pelvis": (0.45, 0, -0.84)}),
    ]
    return _clip(arm, "death", 38, keys)


def respawn(arm):
    low = merge(KNEEL)
    keys = [(0, low, {"pelvis": (0, 0, -0.44)}), (12, merge(STAND, arms((0.8, 0.3, 0.3), (0.8, 0.4, 0.5)), {"head": (-12, 0, 0)}), {"pelvis": (0, 0, -0.05)}),
            (20, merge(STAND, HANDS), None), (28, merge(STAND, HANDS), None)]
    return _clip(arm, "respawn", 28, keys)


def select(arm):
    # Lifts her face and opens her arms to the light.
    open_ = merge(STAND, arms((0.8, 0.4, 0.4), (0.8, 0.5, 0.6)), {"head": (-20, 0, 0), "spine_02": (-6, 0, 0)})
    keys = [(0, merge(STAND, HANDS), None), (12, open_, None), (30, merge(open_, {"head": (-24, 0, 6)}), None), (45, merge(STAND, HANDS), None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    # Wags a finger.
    base = merge(STAND, HANDS)
    up = merge(base, arm_r(0.2, 0.5, 0.3, -0.2, 0.3, 0.9))
    keys = [(0, base, None), (8, merge(up, {"hand_r": (0, 0, 20)}), None), (14, merge(up, {"hand_r": (0, 0, -20)}), None),
            (20, merge(up, {"hand_r": (0, 0, 20)}), None), (26, merge(up, {"hand_r": (0, 0, -20)}), None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Pretends her halo is heavy and props it up.
    base = merge(STAND, HANDS)
    prop = merge(STAND, arms((0.4, 0.0, 0.8), (-0.3, 0.1, 1)), {"head": (14, 10, 0), "spine_02": (6, 6, 0)})
    keys = [(0, base, None), (12, prop, {"pelvis": (0, 0, -0.04)}), (36, merge(prop, {"head": (18, 14, 0)}), {"pelvis": (0, 0, -0.05)}), (50, base, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    hide = arm_r(0.2, 0.6, 0.1, -0.6, 0.5, 0.6)
    keys = [(0, merge(STAND, HANDS), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, HANDS, hide, {"spine_01": (4 * s, 0, 0), "head": (10 * s, 0, 4)}, sym("clavicle", 0, 2 * s, 0)), None))
    keys.append((40, merge(STAND, HANDS), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    # A slow twirl, arms floating.
    keys = [(i * 12, merge(STAND, arms((0.9, 0.1, 0.2), (0.9, 0.2, 0.5)), {"pelvis": (0, 0, 90 * i)}), {"pelvis": (0, 0, 0.02 if i % 2 else 0.0)})
            for i in range(4)]
    return _clip(arm, "emote_dance", 48, keys + [(48, keys[0][1], keys[0][2])], loop=True)


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
    arm = rig.build_armature(name="lumen")
    own = [f(arm).name for f in OWN]
    body = mesh.build_mannequin(arm, name="lumen", part_list=parts())
    shared = bake_shared(arm)
    print(f"lumen: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(own)} own clips, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
