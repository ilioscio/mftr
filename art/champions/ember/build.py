# SPDX-License-Identifier: AGPL-3.0-or-later
"""Ember (A7): the block-out generator for his model and clips.

    blender -b --python art/champions/ember/build.py -- --out art/champions/ember/ember.blend

A lean pyromancer on `biped` v1: a charcoal long coat with tails that trail behind him (an
`extra_coat` chain), a tall ember-orange collar standing behind his head, glowing ember cuffs and
a rune on his chest, and fire thrown from bare hands (no staff). Like the other block-outs this is
a starting point: run once, then the .blend is the source of truth. Pose values follow
`mftr_blender.clips`. He casts from his right hand (the projectile socket).
"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", ".."))
sys.path.insert(0, os.path.join(ROOT, "tools", "blender"))

import bpy  # noqa: E402

from mftr_blender import clips, head, mesh, rig  # noqa: E402
from mftr_blender.clips import merge, sym  # noqa: E402
from mftr_blender.library import _clip, arms, cycle, run_half  # noqa: E402

# Palette (concept.md): charcoal and ember.
SKIN = (0.80, 0.60, 0.47)
HAIR = (0.15, 0.10, 0.09)
COAT = (0.17, 0.15, 0.15)
COAT_DARK = (0.11, 0.10, 0.10)
EMBER = (0.78, 0.36, 0.14)
TROUSERS = (0.20, 0.17, 0.16)
LEATHER = (0.34, 0.21, 0.13)
BOOT = (0.20, 0.13, 0.09)
GOLD = (0.82, 0.62, 0.28)
GLOW = (1.0, 0.55, 0.15)
ACCENT = (0.18, 0.52, 0.95)

COAT_TAIL = [(0, 0.12, 1.08), (0, 0.17, 0.78), (0, 0.21, 0.50)]


# --- Mesh -----------------------------------------------------------------------------------

def _center():
    return [
        ("pelvis", (0, 0, 0.90), (0, 0, 1.13), 12, [(0, .13, .10), (.4, .162, .112), (1, .158, .112)], "cloth", TROUSERS),
        ("pelvis", (0, 0, 1.06), (0, 0, 1.115), 12, [(0, .165, .118), (1, .16, .115)], "cloth", LEATHER),
        ("pelvis", (0, -0.112, 1.07), (0, -0.135, 1.10), 4, [(0, .03, .018), (1, .03, .018)], "metal", GOLD),
        # A team-colored sash tied over the belt.
        ("pelvis", (0, 0, 1.105), (0, 0, 1.15), 12, [(0, .162, .117), (1, .158, .114)], "accent", ACCENT),
        # The coat: fitted at the waist, a little broad at the shoulders, gold-trimmed front edges.
        ("spine_01", (0, 0, 1.12), (0, 0, 1.28), 12, [(0, .14, .10), (.5, .132, .095), (1, .14, .1)], "cloth", COAT),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 12, [(0, .142, .1), (1, .165, .11)], "cloth", COAT),
        ("chest", (0, 0, 1.40), (0, 0, 1.61), 12, [(0, .168, .112), (.35, .178, .116), (.7, .17, .11), (1, .095, .078)], "cloth", COAT),
        ("chest", (0.045, -0.105, 1.18), (0.05, -0.113, 1.56), 4, [(0, .012, .01), (1, .012, .01)], "metal", GOLD),
        ("chest", (-0.045, -0.105, 1.18), (-0.05, -0.113, 1.56), 4, [(0, .012, .01), (1, .012, .01)], "metal", GOLD),
        # An ember rune glowing on his chest.
        ("chest", (0, -0.11, 1.44), (0, -0.135, 1.44), 4, [(0, .03, .03), (.5, .03, .03), (1, 0, 0)], "emissive", GLOW),
        # The collar: a high mage's collar wrapping the back and sides of his neck, flaring up and
        # out behind his head, ember-orange (its front edge stays behind his chin).
        ("chest", (0, 0.035, 1.53), (0, 0.085, 1.76), 12, [(0, .115, .1), (.5, .125, .105), (1, .15, .12)], "cloth", EMBER),
        ("chest", (0, 0.04, 1.53), (0, 0.06, 1.58), 12, [(0, .13, .112), (1, .126, .108)], "cloth", COAT_DARK),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.66), 8, [(0, .05, .05), (1, .046, .046)], "skin", SKIN),
        # The coat tails on their chain.
        ("extra_coat_1", COAT_TAIL[0], COAT_TAIL[1], 6, [(0, .17, .025), (1, .19, .025)], "cloth", COAT),
        ("extra_coat_2", COAT_TAIL[1], COAT_TAIL[2], 6, [(0, .19, .025), (.7, .18, .022), (1, .12, .018)], "cloth", COAT),
        ("extra_coat_2", (0, 0.212, 0.56), (0, 0.214, 0.51), 6, [(0, .17, .026), (1, .13, .022)], "cloth", EMBER),
    ]


def _left():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (0.15, 0.01, 1.48), (0.21, 0.01, 1.6), 8, [(0, .07, .07), (.5, .074, .07), (1, 0, 0)], "cloth", COAT),
        (ua, (ua, 0), (ua, 1), 8, [(0, .054, .054), (1, .046, .046)], "cloth", COAT),
        (fa, (fa, 0), (fa, .9), 8, [(0, .045, .045), (.7, .05, .05), (1, .058, .058)], "cloth", COAT),
        (fa, (fa, .6), (fa, .8), 8, [(0, .052, .052), (1, .055, .055)], "accent", ACCENT),
        # Glowing ember cuffs at the wrists: where his fire comes from.
        (fa, (fa, .86), (fa, 1.0), 8, [(0, .044, .044), (1, .042, .042)], "emissive", GLOW),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .036, .023), (1, .04, .02)], "skin", SKIN),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .04, .02), (1, .026, .014)], "skin", SKIN),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .016, .016), (1, .011, .011)], "skin", SKIN),
        (th, (0.10, 0, 1.02), (th, 1), 10, [(0, .086, .086), (.5, .074, .074), (1, .056, .056)], "cloth", TROUSERS),
        (ca, (ca, 0), (ca, .45), 10, [(0, .056, .056), (1, .052, .052)], "cloth", TROUSERS),
        (ca, (ca, .38), (ca, 1), 10, [(0, .064, .064), (.12, .06, .06), (1, .05, .054)], "cloth", BOOT),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .052, .046), (1, .046, .032)], "cloth", BOOT),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .046, .028), (1, .03, .018)], "cloth", BOOT),
    ]


def _head(bm, layers, groups, mats):
    # The shared sculpted head: young and lean, short dark hair, amber eyes.
    head.build(bm, layers, groups, mats, head.Head(
        base=(0, -0.01, 1.64), height=0.25, scale=0.98, jaw=0.38, chin=1.0, eye=1.0,
        skin=SKIN, hair=HAIR, hair_style="short", eye_color=(0.78, 0.42, 0.1), lips=(0.62, 0.38, 0.34)))


def parts():
    left = _left()
    out = _center() + left + [mesh._mirror_part(p) for p in left]
    out = [(b, a, e, s + 2 if s >= 8 else s, mesh._smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + [_head]


# --- Poses ----------------------------------------------------------------------------------

def coat(back=0.0, sway=0.0):
    """The coat tails: `back` swings them behind him (degrees), `sway` to his left."""
    return {"extra_coat_1": (back * 0.6, sway * 0.5, 0), "extra_coat_2": (back * 0.4, sway * 0.4, 0)}


STAND = merge(sym("thigh", -3, 0, 0), sym("calf", 6, 0, 0), sym("foot", -3, 0, 0), {"spine_01": (2, 0, 0)})
CROUCH = merge(sym("thigh", -14, 0, 0), sym("calf", 26, 0, 0), sym("foot", -12, 0, 0), {"spine_01": (8, 0, 0), "spine_02": (2, 0, 0)})
HANDS_LOW = arms((0.3, 0.08, -1), (0.25, 0.25, -1))
READY = merge(CROUCH, arms((0.35, 0.5, -0.6), (0.1, 1, 0.1)), coat(4))
DOWN = {"pelvis": (0, 0, -0.05)}


def arm_r(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_r": clips.aim(out, fwd, up, "r"), "forearm_r": clips.aim(f_out, f_fwd, f_up, "r")}


def arm_l(out, fwd, up, f_out, f_fwd, f_up):
    return {"upperarm_l": clips.aim(out, fwd, up), "forearm_l": clips.aim(f_out, f_fwd, f_up)}


# --- Clips ----------------------------------------------------------------------------------

def idle(arm):
    a = merge(STAND, HANDS_LOW, coat(0), {"head": (-2, 0, 0)})
    b = merge(STAND, HANDS_LOW, coat(1.5, 2), sym("clavicle", 0, -3, 0), {"head": (-3, 0, 5), "chest": (-2, 0, 0)})
    return _clip(arm, "idle", 64, [(0, a, None), (32, b, {"pelvis": (0, 0, 0.005)})], loop=True)


def idle_fidget_1(arm):
    # Bounces a flame on his open palm.
    base = merge(STAND, HANDS_LOW, coat(0))
    palm = merge(STAND, HANDS_LOW, arm_r(0.35, 0.4, -0.7, 0.1, 0.9, 0.3), {"head": (10, 0, -8)})
    keys = [(0, base, None), (10, palm, None)]
    for i in range(4):
        keys.append((14 + 6 * i, merge(palm, arm_r(0.35, 0.4, -0.6, 0.1, 0.8, 0.5 + 0.15 * (i % 2))), None))
    keys += [(42, palm, None), (54, base, None)]
    return _clip(arm, "idle_fidget_1", 54, keys)


def idle_fidget_2(arm):
    # Snaps his fingers and blows on them like a struck match.
    base = merge(STAND, HANDS_LOW, coat(0))
    snap = merge(STAND, HANDS_LOW, arm_r(0.3, 0.5, 0.3, -0.3, 0.6, 0.7), {"head": (-4, 0, -10)})
    blow = merge(STAND, HANDS_LOW, arm_r(0.2, 0.6, 0.2, -0.6, 0.6, 0.5), {"head": (6, 0, -18)})
    keys = [(0, base, None), (12, snap, None), (16, merge(snap, {"hand_r": (20, 0, 0)}), None), (28, blow, None), (40, blow, None), (56, base, None)]
    return _clip(arm, "idle_fidget_2", 56, keys)


def idle_ready(arm):
    keys = [(0, READY, DOWN), (20, merge(READY, {"spine_01": (9, 0, 0)}, coat(6, 1)), {"pelvis": (0, 0, -0.06)})]
    return _clip(arm, "idle_ready", 40, keys, loop=True)


def run(arm):
    keys = [(f, merge(p, coat(22 + (4 if f % 10 < 5 else 0))), l) for f, p, l in cycle(run_half(), 20)]
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def run_fast(arm):
    def scale(pose):
        out = dict(pose)
        for k, v in pose.items():
            if k.split("_")[0] in ("thigh", "calf", "foot") and isinstance(v, tuple):
                out[k] = (v[0] * 1.2, v[1], v[2])
        out["spine_01"] = (14, 0, pose.get("spine_01", (0, 0, 0))[2])
        return out

    half = [(round(f * 0.8), scale(p), l) for f, p, l in run_half()]
    keys = [(f, merge(p, coat(32)), l) for f, p, l in cycle(half, 16)]
    return _clip(arm, "run_fast", 16, keys, loop=True, markers={"foot_l": 0, "foot_r": 8}, stride_speed=450.0)


def _throw(arm, name, frames, fire, wind, release, follow):
    """A one-handed fire throw from the right hand: gather by the shoulder, thrust on `fire`."""
    keys = [(0, READY, DOWN), (fire - 4, merge(CROUCH, wind, coat(3)), DOWN), (fire, merge(CROUCH, release, coat(6, -2)), {"pelvis": (0, -0.03, -0.05)}),
            (fire + 5, merge(CROUCH, follow, coat(5)), DOWN), (frames, READY, DOWN)]
    return _clip(arm, name, frames, keys, markers={"fire": fire})


def attack_1(arm):
    # Sim: windup 20% of 1/0.65 s = 9.2 frames.
    wind = merge(arm_r(0.5, -0.3, 0.2, -0.2, 0.3, 0.9), arm_l(0.3, 0.6, -0.5, 0.0, 1, 0.1), {"spine_02": (0, 0, -18)})
    release = merge(arm_r(0.05, 1, 0.1, 0.0, 1, 0.1), arm_l(0.4, -0.2, -0.8, 0.3, 0.1, -1), {"spine_02": (4, 0, 14)})
    follow = merge(arm_r(0.1, 0.9, -0.2, 0.0, 0.9, -0.3), HANDS_LOW, {"spine_02": (2, 0, 6)})
    return _throw(arm, "attack_1", 24, 9, wind, release, follow)


def attack_2(arm):
    # The quick variant: an underhand flick.
    wind = merge(arm_r(0.4, -0.5, -0.7, 0.1, -0.3, -0.9), arm_l(0.3, 0.5, -0.6, 0.0, 1, 0.0), {"spine_02": (4, 0, -12)})
    release = merge(arm_r(0.1, 0.9, -0.2, 0.0, 1, 0.3), arm_l(0.4, -0.2, -0.8, 0.3, 0.1, -1), {"spine_02": (6, 0, 10)})
    follow = merge(arm_r(0.1, 0.8, 0.2, 0.0, 0.8, 0.6), HANDS_LOW)
    return _throw(arm, "attack_2", 24, 9, wind, release, follow)


def q(arm):
    # Ember Lance: both hands draw a line of fire back, then thrust it forward (`fire` 8, sim 7.5).
    wind = merge(arms((0.4, -0.4, 0.0), (0.0, 0.3, 0.8)), {"spine_02": (-6, 0, -10)})
    release = merge(arms((0.15, 1, 0.1), (0.05, 1, 0.15)), {"spine_02": (8, 0, 6), "thigh_l": (-20, 0, 0), "calf_l": (16, 0, 0)})
    follow = merge(arms((0.25, 0.8, -0.3), (0.15, 0.8, -0.2)), {"spine_02": (6, 0, 4)})
    keys = [(0, READY, DOWN), (4, merge(CROUCH, wind, coat(4)), DOWN), (8, merge(CROUCH, release, coat(8, -2)), {"pelvis": (0, -0.04, -0.05)}),
            (13, merge(CROUCH, follow, coat(6)), DOWN), (20, READY, DOWN)]
    return _clip(arm, "q", 20, keys, markers={"fire": 8})


def w(arm):
    # Cinder Bloom: hands rise, palms down, then push toward the ground far ahead (`fire` 8).
    rise = merge(arms((0.5, 0.3, 0.6), (0.4, 0.5, 0.7)), {"spine_02": (-8, 0, 0), "head": (-10, 0, 0)})
    push = merge(arms((0.3, 0.9, -0.2), (0.2, 0.9, -0.4)), {"spine_01": (10, 0, 0), "spine_02": (6, 0, 0), "head": (8, 0, 0)})
    keys = [(0, READY, DOWN), (5, merge(CROUCH, rise, coat(2)), {"pelvis": (0, 0, -0.02)}), (8, merge(CROUCH, push, coat(6)), {"pelvis": (0, -0.03, -0.07)}),
            (14, merge(CROUCH, push, coat(5)), {"pelvis": (0, -0.03, -0.07)}), (22, READY, DOWN)]
    return _clip(arm, "w", 22, keys, markers={"fire": 8})


def e(arm):
    # Flicker: a blink, so it plays as a pulse from `fire`: he reappears crouched, the coat
    # settling, and rises into his guard.
    vanish = merge(CROUCH, arms((0.5, -0.1, -0.5), (0.4, 0.2, -0.6)), {"spine_01": (20, 0, 0)}, coat(-12))
    appear = merge(sym("thigh", -40, 0, 0), sym("calf", 70, 0, 0), sym("foot", -28, 0, 0), arms((0.7, 0.2, -0.3), (0.6, 0.5, -0.1)),
                   {"spine_01": (24, 0, 0)}, coat(18, 4))
    keys = [(0, READY, DOWN), (3, vanish, {"pelvis": (0, 0, -0.18)}), (4, appear, {"pelvis": (0, 0, -0.3)}), (12, READY, DOWN)]
    return _clip(arm, "e", 12, keys, markers={"fire": 3})


def r(arm):
    # Binding Sigil: the right hand traces a circle, then thrusts the sigil out (`fire` 9).
    trace = [arm_r(0.4, 0.6, 0.6, 0.2, 0.8, 0.6), arm_r(0.7, 0.6, 0.1, 0.6, 0.8, 0.0), arm_r(0.4, 0.6, -0.4, 0.2, 0.8, -0.5),
             arm_r(0.0, 0.7, 0.1, -0.3, 0.8, 0.0)]
    hold = arm_l(0.3, 0.6, -0.5, 0.0, 1, 0.0)
    keys = [(0, READY, DOWN)] + [(1 + 2 * i, merge(CROUCH, t, hold, coat(3)), DOWN) for i, t in enumerate(trace)]
    keys += [(9, merge(CROUCH, arm_r(0.05, 1, 0.15, 0.0, 1, 0.2), arm_l(0.15, 1, 0.1, 0.05, 1, 0.15), {"spine_02": (8, 0, 8)}, coat(10, -3)),
              {"pelvis": (0, -0.05, -0.06)}),
             (16, merge(CROUCH, arms((0.3, 0.8, -0.2), (0.2, 0.8, -0.1)), coat(7)), DOWN), (26, READY, DOWN)]
    return _clip(arm, "r", 26, keys, markers={"fire": 9})


KNEEL = merge({"thigh_r": (0, 0, 0), "calf_r": (90, 0, 0), "foot_r": (0, 0, 0), "thigh_l": (-80, 0, 0), "calf_l": (80, 0, 0),
               "foot_l": (0, 0, 0), "spine_01": (6, 0, 0), "head": (14, 0, 0)},
              arms((0.2, 0.6, -0.4), (-0.5, 0.7, 0.3)), coat(-8))


def recall(arm):
    # Kneels with his palms together, a flame between them.
    breathe = merge(KNEEL, sym("clavicle", 0, -4, 0), {"spine_01": (2, 0, 0), "head": (10, 0, 0)})
    keys = [(0, merge(STAND, HANDS_LOW), None), (15, KNEEL, {"pelvis": (0, 0, -0.45)}), (45, breathe, {"pelvis": (0, 0, -0.44)}),
            (75, KNEEL, {"pelvis": (0, 0, -0.45)}), (90, merge(STAND, HANDS_LOW, coat(6)), {"pelvis": (0, 0, -0.1)})]
    return _clip(arm, "recall", 90, keys, markers={"loop_in": 15, "loop_out": 75})


def death(arm):
    keys = [
        (0, merge(STAND, HANDS_LOW), None),
        (6, merge(arms((0.6, -0.2, 0.3), (0.5, 0.1, 0.5)), {"spine_01": (-15, 0, 0), "head": (-20, 0, 0)}, coat(-10)), {"pelvis": (0, 0.04, 0)}),
        (14, merge(sym("thigh", 0, 0, 0), sym("calf", 90, 0, 0), arms((0.2, 0.4, -1), (0.15, 0.5, -1)), {"spine_01": (15, 0, 0), "head": (25, 0, 0)}, coat(5)),
         {"pelvis": (0, 0, -0.45)}),
        (24, merge(sym("thigh", -20, 0, 0), sym("calf", 70, 0, 0), arms((0.5, 0.2, -0.6), (0.4, 0.3, -0.7)), {"pelvis": (0, -60, 0), "spine_01": (5, -15, 0)},
                   coat(10, -10)), {"pelvis": (-0.25, 0, -0.65)}),
        (34, merge(sym("thigh", -30, 0, 0), sym("calf", 50, 0, 0), arms((0.7, 0.1, -0.3), (0.6, 0.2, -0.3)), {"pelvis": (0, -85, 0), "head": (0, -10, 0)},
                   coat(20, -15)), {"pelvis": (-0.45, 0, -0.82)}),
    ]
    return _clip(arm, "death", 34, keys)


def respawn(arm):
    low = merge(sym("thigh", -50, 0, 0), sym("calf", 90, 0, 0), sym("foot", -30, 0, 0), arms((0.2, 0.6, -0.6), (-0.3, 0.8, 0)), {"spine_01": (25, 0, 0)}, coat(-5))
    keys = [(0, low, {"pelvis": (0, 0, -0.3)}), (10, merge(STAND, arms((0.9, 0.3, 0.1), (0.9, 0.4, 0.2)), coat(10)), {"pelvis": (0, 0, -0.08)}),
            (18, merge(STAND, HANDS_LOW, {"head": (-8, 0, 0)}, coat(5)), None), (24, merge(STAND, HANDS_LOW), None)]
    return _clip(arm, "respawn", 24, keys)


def select(arm):
    # Arms flung wide, palms up, as if both hands just caught fire; then a slow smirk.
    wide = merge(STAND, arms((1, 0.2, 0.2), (1, 0.3, 0.5)), {"spine_02": (-6, 0, 0), "head": (-14, 0, 0)}, coat(8, 2))
    keys = [(0, merge(STAND, HANDS_LOW), None), (10, wide, {"pelvis": (0, 0, 0.02)}), (26, merge(wide, {"head": (-18, 0, 8)}), None),
            (36, merge(STAND, HANDS_LOW, arm_r(0.35, 0.4, -0.7, 0.1, 0.9, 0.3), {"head": (4, 0, -10)}), None), (45, merge(STAND, HANDS_LOW), None)]
    return _clip(arm, "select", 45, keys)


def emote_taunt(arm):
    base = merge(STAND, HANDS_LOW)
    out = merge(base, arm_r(0.2, 1, 0.0, 0.1, 1, 0.1), {"head": (-5, 0, 0)})
    curl = merge(base, arm_r(0.2, 1, 0.0, 0.1, 0.5, 0.8), {"head": (-5, 0, 0)})
    keys = [(0, base, None), (8, out, None), (14, curl, None), (20, out, None), (26, curl, None), (40, base, None)]
    return _clip(arm, "emote_taunt", 40, keys)


def emote_joke(arm):
    # Blows out his fingertip like a candle, then shrugs.
    base = merge(STAND, HANDS_LOW)
    finger = merge(base, arm_r(0.2, 0.5, -0.2, -0.4, 0.4, 0.8), {"head": (8, 0, -6)})
    keys = [(0, base, None), (10, finger, None), (20, merge(finger, {"head": (12, 0, -10)}), None), (30, finger, None),
            (40, merge(STAND, arms((0.6, 0.3, -0.2), (0.6, 0.5, 0.4)), sym("clavicle", 0, -8, 0)), None), (50, base, None)]
    return _clip(arm, "emote_joke", 50, keys)


def emote_laugh(arm):
    belly = arm_l(0.15, 0.3, -0.9, -0.7, 0.6, 0.2)
    keys = [(0, merge(STAND, HANDS_LOW), None)]
    for i in range(1, 9):
        s = -1 if i % 2 else -0.5
        keys.append((4 * i, merge(STAND, HANDS_LOW, belly, {"spine_01": (8 * s, 0, 0), "head": (15 * s, 0, 0)}, sym("clavicle", 0, 3 * s, 0)), None))
    keys.append((40, merge(STAND, HANDS_LOW), None))
    return _clip(arm, "emote_laugh", 40, keys)


def emote_dance(arm):
    a = merge(arms((0.6, 0.2, 0.8), (0.6, 0.0, 1), (0.3, 0.1, -1), (0.2, 0.3, -1)), {"thigh_l": (-20, 0, 0), "calf_l": (30, 0, 0), "pelvis": (0, -6, 20)}, coat(0, -8))
    mid = merge(STAND, arms((0.4, 0.3, -0.2), (0.3, 0.6, 0.2)), coat(4))
    b = merge(arms((0.3, 0.1, -1), (0.2, 0.3, -1), (0.6, 0.2, 0.8), (0.6, 0.0, 1)), {"thigh_r": (-20, 0, 0), "calf_r": (30, 0, 0), "pelvis": (0, 6, -20)}, coat(0, 8))
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
    arm = rig.build_armature(name="ember")
    rig.add_chain(arm, "coat", "pelvis", COAT_TAIL)
    own = [f(arm).name for f in OWN]
    body = mesh.build_mannequin(arm, name="ember", part_list=parts())
    shared = bake_shared(arm)
    print(f"ember: {len(arm.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(own)} own clips, shared {shared}")


if __name__ == "__main__":
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    build()
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
