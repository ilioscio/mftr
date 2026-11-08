# SPDX-License-Identifier: AGPL-3.0-or-later
"""The creator's own moves' clips (20 §8.2, `art/library/biped/moves.ron`), on the shared biped
library: `move_<name>` (a dash: `move_<name>_start`, `_travel`, `_land`).

Weight first (14 §3): every move loads before it acts (a crouch, a coil, a step back), the hips
and chest lead the arms and the arms lead the hands, the blow lands on a held frame, and the
body settles out of it. Timing follows the sim (10 §4.3): a cast with a windup maps the windup
to `[0, fire]`; an instant cast (shields, buffs, blinks, lunges, dashes) plays from `fire` on, so
those put their action after `fire`.

    blender -b art/library/biped/biped_library.blend --python tools/blender/run.py -- moves

adds (or replaces) them in the library file, which is the source of truth for the rest.
"""

import math

from .clips import aim, merge, sym
from .library import _clip, arms

# A ready stance: knees soft, feet apart, the weight low.
STANCE = merge(sym("thigh", -10, 0, 0), sym("calf", 18, 0, 0), sym("foot", -8, 0, 0), {"spine_01": (6, 0, 0)})
READY = merge(STANCE, arms((0.3, 0.12, -1), (0.2, 0.45, -0.8)))
# Arms raised in front: the way through for a swing from the hips to over the head (an arm
# swung 180° in one key would go the short way, out to the side).
FRONT = arms((0.2, 1, 0.1), (0.1, 1, 0.3))
WIDE = merge({"thigh_l": (-8, 12, 0), "thigh_r": (-8, -12, 0), "calf_l": (24, 0, 0), "calf_r": (24, 0, 0), "foot_l": (-14, -10, 0),
              "foot_r": (-14, 10, 0)})


def whirlwind(arm):
    # Area round the caster, 300 ms windup (fire 9): coils right with the weapon arm wrapped
    # across the body, then unwinds into two full turns, arms flung out, and decelerates.
    coil = merge(WIDE, arms((0.1, 0.6, -0.4), (-0.6, 0.6, -0.2), (0.9, 0.0, -0.3), (0.6, 0.6, 0.0)),
                 {"spine_01": (10, 0, -24), "spine_02": (6, 0, -18), "chest": (4, 0, -12), "head": (-6, 0, 30)})
    # In the spin the weapon arm reaches out and leads, the off arm stays bent in close for
    # balance, the body leans into the turn and the knees sink; the head looks ahead of it.
    spin = merge(WIDE, {"calf_l": (34, 0, 0), "calf_r": (34, 0, 0), "thigh_l": (-16, 12, 0), "thigh_r": (-16, -12, 0)},
                 arms((0.5, -0.2, -0.8), (0.1, 0.7, -0.2), (1, 0.45, -0.3), (0.9, 0.7, -0.15)),
                 {"spine_01": (12, -6, 10), "spine_02": (6, -4, 8), "chest": (2, 0, 4), "head": (-6, 0, 20)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"),
            (9, merge(coil, {"pelvis": (0, 0, -60)}), {"pelvis": (0, 0.02, -0.1)}, "whip")]
    for i, f in enumerate(range(11, 27, 2)):
        keys.append((f, merge(spin, {"pelvis": (0, 0, -60 + 97.5 * (i + 1))}), {"pelvis": (0, 0, -0.08)}, "snap"))
    keys += [(29, merge(spin, {"pelvis": (0, 0, 720), "spine_01": (14, 0, 16)}), {"pelvis": (0, 0, -0.1)}, "settle"),
             (36, merge(READY, {"pelvis": (0, 0, 720)}), {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_whirlwind", 36, keys, markers={"fire": 9})


def shoulder_charge(arm):
    # Lunge (instant, fire 3): drops the right shoulder and drives through the target, arms
    # tucked, legs pumping, a held frame on contact, then shoves off it upright.
    drive_a = merge(arms((0.15, 0.4, -0.6), (-0.4, 0.6, 0.3), (0.5, -0.1, -0.8), (0.2, 0.7, 0.2)),
                    {"thigh_l": (-60, 0, 0), "calf_l": (60, 0, 0), "thigh_r": (20, 0, 0), "calf_r": (30, 0, 0), "foot_r": (20, 0, 0),
                     "spine_01": (26, 0, -18), "spine_02": (14, 0, -12), "chest": (6, 0, -8), "neck": (-14, 0, 10), "head": (-10, 0, 8)})
    drive_b = merge(drive_a, {"thigh_l": (20, 0, 0), "calf_l": (30, 0, 0), "foot_l": (20, 0, 0), "thigh_r": (-60, 0, 0), "calf_r": (60, 0, 0),
                              "foot_r": (0, 0, 0)})
    hit = merge(drive_a, {"spine_01": (20, 0, -24), "spine_02": (10, 0, -16), "head": (-14, 0, 14)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.04)}, "ease"), (3, merge(READY, {"spine_01": (20, 0, -12)}), {"pelvis": (0, 0, -0.1)}, "snap"),
            (6, drive_a, {"pelvis": (0, -0.08, -0.08)}, "snap"), (9, drive_b, {"pelvis": (0, -0.12, -0.06)}, "snap"),
            (12, hit, {"pelvis": (0, -0.14, -0.08)}, "hold"), (16, hit, {"pelvis": (0, -0.12, -0.07)}, "settle"),
            (30, READY, {"pelvis": (0, 0, -0.04)})]
    return _clip(arm, "move_shoulder_charge", 30, keys, markers={"fire": 3})


def hook_throw(arm):
    # Line, 400 ms windup (fire 12): whirls the chain overhead (the forearm circling, the body
    # coiling right), hurls it sidearm with a step, then hauls back on it with the whole body.
    def whirl(a):
        c, s = math.cos(a), math.sin(a)
        return merge(STANCE, arms((0.3, 0.1, -1), (0.2, 0.5, -0.6), (0.4, 0.0, 0.9), (0.6 * c, 0.6 * s, 0.5)),
                     {"spine_01": (-2, 0, -14), "spine_02": (-2, 0, -10), "head": (-6, 0, 14)})
    hurl = merge(arms((0.3, 0.3, -0.9), (0.1, 0.8, -0.3), (-0.2, 1, 0.1), (-0.3, 1, 0.0)),
                 {"thigh_l": (-30, 0, 0), "calf_l": (24, 0, 0), "thigh_r": (14, 0, 0), "calf_r": (20, 0, 0), "foot_r": (-24, 0, 0),
                  "spine_01": (12, 0, 20), "spine_02": (8, 0, 16), "chest": (4, 0, 10), "head": (-6, 0, -16)})
    haul = merge(arms((0.3, 0.5, -0.6), (0.0, 0.9, 0.2), (0.4, 0.3, -0.7), (0.1, 0.8, 0.2)),
                 {"thigh_l": (-14, 0, 0), "calf_l": (20, 0, 0), "thigh_r": (-4, 0, 0), "calf_r": (30, 0, 0),
                  "spine_01": (-14, 0, -4), "spine_02": (-8, 0, -4), "head": (6, 0, 0)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease")]
    for i, f in enumerate((3, 5, 7, 9)):
        keys.append((f, whirl(math.pi / 2 * (i + 1)), {"pelvis": (0, 0.01, -0.04)}, "snap"))
    keys += [(11, whirl(math.pi * 2.6), {"pelvis": (0, 0.03, -0.05)}, "whip"), (12, hurl, {"pelvis": (0, -0.08, -0.08)}, "hold"),
             (16, hurl, {"pelvis": (0, -0.08, -0.08)}, "ease"), (22, haul, {"pelvis": (0, 0.06, -0.08)}, "hold"),
             (26, haul, {"pelvis": (0, 0.07, -0.08)}, "settle"), (40, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_hook_throw", 40, keys, markers={"fire": 12})


def battle_cry(arm):
    # Support (instant, fire 2): plants wide, throws the chest out and roars up at the sky, both
    # fists clenched down and back; shakes with it; settles.
    roar = merge(WIDE, arms((0.6, -0.3, -0.7), (0.5, 0.2, 0.5)), sym("clavicle", 0, -8, 0),
                 {"spine_01": (-10, 0, 0), "spine_02": (-10, 0, 0), "chest": (-6, 0, 0), "neck": (-10, 0, 0), "head": (-22, 0, 0)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (2, merge(READY, {"spine_01": (14, 0, 0)}), {"pelvis": (0, 0, -0.1)}, "snap"),
            (7, roar, {"pelvis": (0, 0, -0.07)}, "hold")]
    for f in range(9, 23, 2):
        s = 1 if (f // 2) % 2 else -1
        keys.append((f, merge(roar, {"chest": (-6, 0, 2.5 * s), "head": (-22, 0, -3 * s)}), {"pelvis": (0, 0, -0.07)}, "snap"))
    keys += [(23, roar, {"pelvis": (0, 0, -0.07)}, "ease"), (36, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_battle_cry", 36, keys, markers={"fire": 2})


def vanish(arm):
    # Blink (instant, fire 2; the follow-through plays where they arrive): steps out of the dark
    # low, an arm still across the face as if behind a cloak, sweeps it away and rises.
    low = merge(sym("thigh", -50, 0, 0), sym("calf", 80, 0, 0), sym("foot", -30, 0, 0),
                arms((0.2, 0.6, -0.3), (-0.8, 0.3, 0.5), (0.4, 0.2, -0.9), (0.3, 0.6, -0.3)),
                {"spine_01": (24, 0, 0), "spine_02": (14, 0, 0), "head": (14, 0, -10)})
    sweep = merge(STANCE, arms((0.9, 0.3, -0.1), (1, 0.2, 0.1), (0.3, 0.1, -1), (0.2, 0.4, -0.8)), {"spine_01": (6, 0, 10), "head": (-4, 0, 6)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (2, low, {"pelvis": (0, 0, -0.36)}, "hold"), (8, low, {"pelvis": (0, 0, -0.34)}, "whip"),
            (14, sweep, {"pelvis": (0, 0, -0.08)}, "settle"), (30, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_vanish", 30, keys, markers={"fire": 2})


def leap(arm):
    # Dash: the start (instant, fire 4) springs up out of a crouch; the travel is the airborne
    # tuck, weapon arm cocked high; the land comes down weapon first into a deep crouch.
    crouch = merge(sym("thigh", -56, 0, 0), sym("calf", 90, 0, 0), sym("foot", -34, 0, 0), arms((0.4, -0.5, -0.7), (0.2, 0.3, -0.8)),
                   {"spine_01": (30, 0, 0), "spine_02": (10, 0, 0), "head": (-14, 0, 0)})
    spring = merge(sym("thigh", 10, 0, 0), sym("calf", 6, 0, 0), sym("foot", 30, 0, 0), arms((0.2, 0.4, 1), (0.1, 0.4, 1)),
                   {"spine_01": (-8, 0, 0), "head": (-10, 0, 0)})
    tuck = merge({"thigh_l": (-70, 0, 0), "calf_l": (100, 0, 0), "thigh_r": (-40, 0, 0), "calf_r": (90, 0, 0), "foot_l": (20, 0, 0),
                  "foot_r": (20, 0, 0)}, arms((0.6, 0.4, 0.2), (0.4, 0.8, 0.0), (0.3, -0.3, 1), (0.2, 0.6, 0.6)),
                 {"spine_01": (14, 0, 0), "head": (-12, 0, 0)})
    smash = merge(sym("thigh", -60, 0, 0), sym("calf", 96, 0, 0), sym("foot", -36, 0, 0),
                  arms((0.5, 0.4, -0.5), (0.2, 0.9, -0.3), (0.1, 1, -0.6), (0.0, 1, -0.7)),
                  {"spine_01": (34, 0, 0), "spine_02": (14, 0, 0), "head": (-18, 0, 0)})
    a = _clip(arm, "move_leap_start", 8, [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (4, crouch, {"pelvis": (0, 0, -0.3)}, "snap"),
                                         (8, spring, {"pelvis": (0, 0, 0.06)})], markers={"fire": 4})
    b = _clip(arm, "move_leap_travel", 12, [(0, tuck, {"pelvis": (0, 0, 0.1)}), (6, merge(tuck, {"spine_01": (20, 0, 0)}), {"pelvis": (0, 0, 0.12)})],
              loop=True)
    c = _clip(arm, "move_leap_land", 18, [(0, smash, {"pelvis": (0, 0, -0.32)}, "hold"), (5, smash, {"pelvis": (0, 0, -0.33)}, "settle"),
                                         (18, READY, {"pelvis": (0, 0, -0.03)})])
    return [a, b, c]


def meteor_call(arm):
    # Area at range, 600 ms windup (fire 18): rises onto the toes with both arms reaching for
    # the sky, the head thrown back, then hauls them down and drops into a deep crouch as the
    # star is called; holds it while it falls.
    reach = merge(sym("thigh", -2, 0, 0), sym("calf", 2, 0, 0), sym("foot", 30, 0, 0), arms((0.25, 0.1, 1), (0.15, 0.05, 1)),
                  {"spine_01": (-8, 0, 0), "spine_02": (-8, 0, 0), "chest": (-6, 0, 0), "head": (-30, 0, 0)})
    haul = merge(sym("thigh", -48, 0, 0), sym("calf", 80, 0, 0), sym("foot", -30, 0, 0), arms((0.3, 0.8, -0.4), (0.15, 1, -0.6)),
                 {"spine_01": (30, 0, 0), "spine_02": (16, 0, 0), "chest": (6, 0, 0), "head": (-16, 0, 0)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (6, merge(READY, {"spine_01": (12, 0, 0)}), {"pelvis": (0, 0, -0.08)}, "ease"),
            (10, merge(READY, FRONT), {"pelvis": (0, 0, -0.02)}, "ease"), (14, reach, {"pelvis": (0, 0, 0.05)}, "hold"), (16, reach, {"pelvis": (0, 0, 0.06)}, "whip"),
            (18, haul, {"pelvis": (0, 0, -0.26)}, "hold"), (28, haul, {"pelvis": (0, 0, -0.25)}, "settle"), (44, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_meteor_call", 44, keys, markers={"fire": 18})


def channel(arm):
    # Beam, 850 ms windup (fire 26): steps back into a braced stance and draws both hands to
    # the right hip, gathering; thrusts both palms out and holds the beam, the arms shuddering
    # against its recoil and the back foot sliding; lets go and recovers.
    gather = merge({"thigh_l": (-24, 0, 0), "calf_l": (30, 0, 0), "foot_l": (-8, 0, 0), "thigh_r": (14, 0, 0), "calf_r": (34, 0, 0),
                    "foot_r": (-20, 0, 0)}, arms((0.0, 0.5, -0.6), (-0.6, 0.5, 0.2), (0.4, 0.2, -0.8), (0.0, 0.6, -0.2)),
                   {"spine_01": (4, 0, -26), "spine_02": (2, 0, -18), "chest": (0, 0, -10), "head": (0, 0, 40)})
    thrust = merge({"thigh_l": (-30, 0, 0), "calf_l": (34, 0, 0), "foot_l": (-4, 0, 0), "thigh_r": (20, 0, 0), "calf_r": (24, 0, 0),
                    "foot_r": (-26, 0, 0)}, arms((0.05, 1, 0.0), (0.02, 1, 0.05)),
                   {"spine_01": (8, 0, 0), "spine_02": (4, 0, 0), "chest": (2, 0, 0), "head": (-4, 0, 0)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (14, gather, {"pelvis": (0, 0.06, -0.08)}, "ease"),
            (24, merge(gather, {"spine_01": (2, 0, -32)}), {"pelvis": (0, 0.07, -0.09)}, "whip"),
            (26, thrust, {"pelvis": (0, -0.02, -0.1)}, "hold")]
    for f in range(28, 68, 2):
        s = 1 if (f // 2) % 2 else -1
        keys.append((f, merge(thrust, {"upperarm_l": aim(0.05, 1, 0.03 * s, "l"), "upperarm_r": aim(0.05, 1, -0.03 * s, "r"),
                                       "chest": (2 - 1.5 * s, 0, 0)}), {"pelvis": (0, 0.04 * (f - 26) / 42, -0.1)}, "snap"))
    keys += [(70, thrust, {"pelvis": (0, 0.04, -0.1)}, "settle"), (80, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_channel", 80, keys, markers={"fire": 26})


def arcane_bolt(arm):
    # Line, 300 ms windup (fire 9): turns side-on and draws the right hand in to the chest, the
    # left reaching forward to aim; drives the palm out with the hips, and the hand recoils.
    draw = merge({"thigh_l": (-16, 0, 0), "calf_l": (20, 0, 0), "thigh_r": (6, 0, 0), "calf_r": (22, 0, 0), "foot_r": (-10, 0, 0)},
                 arms((0.2, 0.9, -0.2), (0.0, 1, 0.1), (0.5, -0.4, -0.6), (-0.7, 0.4, 0.3)),
                 {"spine_01": (2, 0, -22), "spine_02": (0, 0, -16), "chest": (0, 0, -8), "head": (0, 0, 30)})
    drive = merge({"thigh_l": (-24, 0, 0), "calf_l": (24, 0, 0), "thigh_r": (14, 0, 0), "calf_r": (16, 0, 0), "foot_r": (-24, 0, 0)},
                  arms((0.4, 0.1, -0.9), (0.2, 0.5, -0.7), (-0.05, 1, 0.05), (-0.05, 1, 0.08)),
                  {"hand_r": (-30, 0, 0), "spine_01": (8, 0, 14), "spine_02": (4, 0, 12), "chest": (2, 0, 6), "head": (-4, 0, -12)})
    recoil = merge(drive, arms((0.4, 0.1, -0.9), (0.2, 0.5, -0.7), (0.1, 0.9, 0.3), (0.1, 0.6, 0.6)), {"spine_01": (2, 0, 10)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (8, draw, {"pelvis": (0, 0.03, -0.05)}, "whip"), (9, drive, {"pelvis": (0, -0.05, -0.07)}, "hold"),
            (12, recoil, {"pelvis": (0, -0.03, -0.06)}, "settle"), (26, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_arcane_bolt", 26, keys, markers={"fire": 9})


def frost_nova(arm):
    # Area round the caster, 250 ms windup (fire 8): both fists come up over the head on a
    # rising breath, then are driven down into the ground as one knee drops; held there.
    up = merge(sym("thigh", -4, 0, 0), sym("calf", 6, 0, 0), sym("foot", 10, 0, 0), arms((0.2, 0.3, 1), (0.05, 0.4, 0.9)),
               {"spine_01": (-8, 0, 0), "spine_02": (-6, 0, 0), "head": (-14, 0, 0)})
    kneel = merge({"thigh_l": (-70, 0, 0), "calf_l": (80, 0, 0), "foot_l": (-10, 0, 0), "thigh_r": (10, 0, 0), "calf_r": (110, 0, 0), "foot_r": (40, 0, 0)},
                  arms((0.15, 0.5, -0.85), (0.05, 0.3, -1)), {"spine_01": (34, 0, 0), "spine_02": (16, 0, 0), "head": (-20, 0, 0)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (3, merge(READY, FRONT), {"pelvis": (0, 0, 0.0)}, "ease"),
            (6, up, {"pelvis": (0, 0, 0.04)}, "whip"), (8, kneel, {"pelvis": (0, 0, -0.42)}, "hold"),
            (16, kneel, {"pelvis": (0, 0, -0.41)}, "settle"), (34, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_frost_nova", 34, keys, markers={"fire": 8})


def benediction(arm):
    # Support (instant, fire 2): lifts the right hand open to the sky, the left over the heart,
    # the head bowed; the hand lowers slowly as if letting the light go.
    bless = merge(STANCE, arms((0.2, 0.1, -1), (-0.6, 0.5, 0.4), (0.3, 0.2, 1), (0.1, 0.2, 1)),
                  {"hand_r": (40, 0, 0), "spine_01": (-4, 0, 0), "chest": (-4, 0, 0), "head": (24, 0, 0)})
    lower = merge(bless, arms((0.2, 0.1, -1), (-0.6, 0.5, 0.4), (0.6, 0.4, 0.4), (0.4, 0.6, 0.3)), {"head": (16, 0, 0)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (2, merge(READY, {"head": (10, 0, 0)}), {"pelvis": (0, 0, -0.04)}, "settle"),
            (8, bless, {"pelvis": (0, 0, 0.0)}, "hold"), (18, bless, {"pelvis": (0, 0, 0.0)}, "ease"), (26, lower, {"pelvis": (0, 0, -0.02)}, "ease"),
            (36, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_benediction", 36, keys, markers={"fire": 2})


def smoke_roll(arm):
    # Dash with a veil: the start (instant, fire 3) dashes a bomb down at the feet; the travel is
    # a forward roll, tucked; the land comes up out of it in a crouch, ready.
    throw = merge(STANCE, arms((0.3, 0.1, -1), (0.2, 0.4, -0.8), (0.1, 0.6, -0.8), (0.0, 0.5, -0.9)), {"spine_01": (24, 0, 0), "head": (-10, 0, 0)})
    tuck = merge(sym("thigh", -100, 0, 0), sym("calf", 130, 0, 0), sym("foot", 20, 0, 0), arms((0.3, 0.6, -0.5), (-0.3, 0.6, 0.3)),
                 {"spine_01": (40, 0, 0), "spine_02": (26, 0, 0), "chest": (10, 0, 0), "neck": (20, 0, 0), "head": (20, 0, 0)})
    crouch = merge(sym("thigh", -50, 0, 0), sym("calf", 80, 0, 0), sym("foot", -30, 0, 0), arms((0.4, 0.4, -0.7), (0.2, 0.7, -0.3)),
                   {"spine_01": (24, 0, 0), "head": (-14, 0, 0)})
    a = _clip(arm, "move_smoke_roll_start", 8, [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (3, throw, {"pelvis": (0, 0, -0.1)}, "snap"),
                                               (8, merge(tuck, {"pelvis": (40, 0, 0)}), {"pelvis": (0, 0, -0.45)})], markers={"fire": 3})
    keys = [(f, merge(tuck, {"pelvis": (40 + 360.0 * f / 12, 0, 0)}), {"pelvis": (0, 0, -0.45 + 0.08 * math.sin(math.pi * f / 12))}, "snap")
            for f in range(0, 13, 3)]
    b = _clip(arm, "move_smoke_roll_travel", 12, keys, loop=True)
    c = _clip(arm, "move_smoke_roll_land", 16, [(0, merge(tuck, {"pelvis": (400, 0, 0)}), {"pelvis": (0, 0, -0.45)}, "settle"),
                                               (5, crouch, {"pelvis": (0, 0, -0.32)}, "hold"), (8, crouch, {"pelvis": (0, 0, -0.3)}, "ease"),
                                               (16, READY, {"pelvis": (0, 0, -0.03)})])
    return [a, b, c]


def riposte(arm):
    # Shield (instant, fire 2): snaps into a high guard, the weapon arm up across the face, the
    # weight back; two small parries (the guard ticks against blows), then lowers.
    guard = merge({"thigh_l": (-20, 0, 0), "calf_l": (26, 0, 0), "thigh_r": (10, 0, 0), "calf_r": (30, 0, 0), "foot_r": (-16, 0, 0)},
                  arms((0.5, 0.2, -0.8), (0.0, 0.8, 0.4), (0.2, 0.5, 0.6), (-0.6, 0.4, 0.6)),
                  {"spine_01": (-4, 0, -12), "spine_02": (-2, 0, -8), "head": (6, 0, 10)})
    parry = merge(guard, arms((0.5, 0.2, -0.8), (0.0, 0.8, 0.4), (0.4, 0.5, 0.6), (-0.4, 0.5, 0.7)), {"spine_01": (-6, 0, -16)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (2, guard, {"pelvis": (0, 0.05, -0.08)}, "hold"), (8, guard, {"pelvis": (0, 0.05, -0.08)}, "snap"),
            (10, parry, {"pelvis": (0, 0.07, -0.08)}, "settle"), (16, guard, {"pelvis": (0, 0.05, -0.08)}, "snap"),
            (18, parry, {"pelvis": (0, 0.07, -0.08)}, "settle"), (24, guard, {"pelvis": (0, 0.05, -0.08)}, "ease"), (36, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_riposte", 36, keys, markers={"fire": 2})


def knife_fan(arm):
    # Line, 350 ms windup (fire 11): crosses both hands to the left hip, coiling, then sweeps
    # them out low and wide in a fan as the body unwinds, a step forward on the throw.
    cross = merge(STANCE, arms((-0.1, 0.5, -0.8), (-0.8, 0.3, -0.3), (0.5, 0.4, -0.7), (0.9, 0.3, -0.1)),
                  {"spine_01": (12, 0, 24), "spine_02": (6, 0, 16), "head": (-4, 0, -24)})
    fan = merge({"thigh_l": (-34, 0, 0), "calf_l": (30, 0, 0), "thigh_r": (14, 0, 0), "calf_r": (20, 0, 0), "foot_r": (-24, 0, 0)},
                arms((0.8, 0.6, -0.2), (0.7, 0.7, 0.0), (0.8, 0.6, -0.2), (0.7, 0.7, 0.0)),
                {"spine_01": (16, 0, -6), "spine_02": (6, 0, -4), "head": (-12, 0, 0)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (10, cross, {"pelvis": (0, 0.03, -0.08)}, "whip"), (11, fan, {"pelvis": (0, -0.08, -0.1)}, "hold"),
            (15, fan, {"pelvis": (0, -0.08, -0.1)}, "settle"), (28, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_knife_fan", 28, keys, markers={"fire": 11})


def earthsplitter(arm):
    # Line ultimate, 550 ms windup (fire 16): heaves the weapon up and back in both hands, the
    # body arching, rising onto the toes; brings it down to the ground in front with everything,
    # a deep lunge, and holds as the split runs out.
    heave = merge(sym("thigh", -6, 0, 0), sym("calf", 10, 0, 0), sym("foot", 14, 0, 0), arms((0.25, -0.3, 1), (0.1, -0.6, 0.8)),
                  {"spine_01": (-16, 0, 0), "spine_02": (-12, 0, 0), "chest": (-8, 0, 0), "head": (-14, 0, 0)})
    split = merge({"thigh_l": (-70, 0, 0), "calf_l": (70, 0, 0), "foot_l": (-4, 0, 0), "thigh_r": (24, 0, 0), "calf_r": (30, 0, 0), "foot_r": (-30, 0, 0)},
                  arms((0.15, 0.9, -0.4), (0.05, 1, -0.5)), {"spine_01": (40, 0, 0), "spine_02": (18, 0, 0), "chest": (8, 0, 0), "head": (-26, 0, 0)})
    keys = [(0, READY, {"pelvis": (0, 0, -0.03)}, "ease"), (6, merge(READY, {"spine_01": (14, 0, 0)}), {"pelvis": (0, 0, -0.1)}, "ease"),
            (10, merge(READY, FRONT), {"pelvis": (0, 0, -0.04)}, "ease"), (14, heave, {"pelvis": (0, 0.04, 0.04)}, "hold"), (15, heave, {"pelvis": (0, 0.04, 0.05)}, "whip"),
            (16, split, {"pelvis": (0, -0.12, -0.32)}, "hold"), (26, split, {"pelvis": (0, -0.12, -0.31)}, "settle"), (44, READY, {"pelvis": (0, 0, -0.03)})]
    return _clip(arm, "move_earthsplitter", 44, keys, markers={"fire": 16})


ALL = [whirlwind, shoulder_charge, hook_throw, battle_cry, vanish, leap, meteor_call, channel,
       arcane_bolt, frost_nova, benediction, smoke_roll, riposte, knife_fan, earthsplitter]


def add(arm):
    """Create (or replace: `clips.new_action`) every move clip on the library's armature. Returns
    their names."""
    out = []
    for f in ALL:
        made = f(arm)
        out += [a.name for a in (made if isinstance(made, list) else [made])]
    return out
