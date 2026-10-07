# SPDX-License-Identifier: AGPL-3.0-or-later
"""Block-out of the shared `biped` library clips (10 §5, §7.3).

This is a *starting point*, generated once: key poses and timing that already obey the markers
and the timing contract (10 §4). After generation the `.blend` is the source of truth, and
humans polish there. Re-running `blockout` overwrites the clips it creates.

Arms are aimed with (out, fwd, up) directions in character space (`clips.aim`). Everything else
uses (rx, ry, rz) degrees about the armature axes (`clips.key_pose`): thigh rx - = leg forward,
calf rx + = knee bends, foot rx - = toes up, spine/neck/head rx + = bend forward, rz + = turn
left, ry + = lean to the character's left.
"""

from . import clips, rig
from .clips import aim_sym, merge, sym

# Arms down from the A-pose, slightly bent: the neutral that most poses build on.
RELAXED = merge(aim_sym("upperarm", 0.28, 0.02, -1), aim_sym("forearm", 0.22, 0.18, -1))


def arms(upper_l, fore_l, upper_r=None, fore_r=None):
    """Aim both arms; (out, fwd, up) per bone, the right side defaults to the left's mirror."""
    return merge(aim_sym("upperarm", *upper_l, right=upper_r), aim_sym("forearm", *fore_l, right=fore_r))


def mirror(pose):
    """Left <-> right mirror of a pose, for the second half of cycles."""
    out = {}
    for name, value in pose.items():
        if name.endswith("_l"):
            name = name[:-2] + "_r"
        elif name.endswith("_r"):
            name = name[:-2] + "_l"
        if isinstance(value, dict):
            x, y, z = value["aim"]
            out[name] = {"aim": (-x, y, z)}
        else:
            rx, ry, rz = value
            out[name] = (rx, -ry, -rz)
    return out


def _mirror_locs(locs):
    return {k: (-x, y, z) for k, (x, y, z) in (locs or {}).items()}


def _clip(arm, name, frames, keys, loop=False, layer="full", bones=None, markers=None, stride_speed=None):
    """keys: [(frame, pose, locs)]. Loops get their first key repeated on the last frame."""
    act = clips.new_action(arm, name, frames, loop=loop, layer=layer, stride_speed=stride_speed)
    bones = bones or clips.full_body()
    if loop and keys[-1][0] != frames:
        keys = keys + [(frames, keys[0][1], keys[0][2])]
    # A bone with a location on any key gets one on every key (zero by default), so an offset
    # authored on one key doesn't hold across the whole clip.
    moved = {b for _, _, locs in keys for b in (locs or {})}
    for frame, pose, locs in keys:
        clips.key_pose(arm, frame, pose, bones=bones, locs={b: (locs or {}).get(b, (0, 0, 0)) for b in moved})
    for m, f in (markers or {}).items():
        clips.set_marker(act, m, f)
    if not loop:
        clips.set_marker(act, "end", frames)
    clips.stash(arm, act)
    clips.reset_pose(arm)
    return act


def walk(arm):
    # Left contact, loading, passing, high point; the second half mirrors the first.
    half = [
        (0, merge(arms((0.25, -0.35, -1), (0.2, -0.15, -1), (0.25, 0.4, -1), (0.2, 0.8, -1)), {
            "thigh_l": (-24, 0, 0), "calf_l": (4, 0, 0), "foot_l": (-8, 0, 0),
            "thigh_r": (16, 0, 0), "calf_r": (18, 0, 0), "foot_r": (10, 0, 0),
            "pelvis": (0, 0, -5), "spine_01": (3, 0, 2), "spine_02": (2, 0, 4), "head": (-3, 0, 0)}), {"pelvis": (0, 0, -0.02)}),
        (4, merge(arms((0.25, -0.28, -1), (0.2, -0.05, -1), (0.25, 0.3, -1), (0.2, 0.65, -1)), {
            "thigh_l": (-18, 0, 0), "calf_l": (14, 0, 0), "foot_l": (2, 0, 0),
            "thigh_r": (12, 0, 0), "calf_r": (30, 0, 0), "foot_r": (16, 0, 0),
            "pelvis": (0, 0, -4), "spine_01": (3, 0, 2), "spine_02": (2, 0, 3), "head": (-3, 0, 0)}), {"pelvis": (0, 0, -0.03)}),
        (8, merge(RELAXED, {
            "thigh_l": (0, 0, 0), "calf_l": (4, 0, 0), "foot_l": (-4, 0, 0),
            "thigh_r": (-12, 0, 0), "calf_r": (40, 0, 0), "foot_r": (-6, 0, 0),
            "pelvis": (0, 2, 0), "spine_01": (3, 0, 0), "spine_02": (2, 0, 0), "head": (-3, 0, 0)}), {"pelvis": (0, 0, 0.01)}),
        (12, merge(arms((0.25, 0.25, -1), (0.2, 0.55, -1), (0.25, -0.25, -1), (0.2, -0.05, -1)), {
            "thigh_l": (10, 0, 0), "calf_l": (6, 0, 0), "foot_l": (-2, 0, 0),
            "thigh_r": (-22, 0, 0), "calf_r": (22, 0, 0), "foot_r": (-10, 0, 0),
            "pelvis": (0, 0, 3), "spine_01": (3, 0, -1), "spine_02": (2, 0, -3), "head": (-3, 0, 0)}), {"pelvis": (0, 0, 0.0)}),
    ]
    keys = half + [(f + 16, mirror(p), _mirror_locs(l)) for f, p, l in half]
    return _clip(arm, "walk", 32, keys, loop=True, markers={"foot_l": 0, "foot_r": 16}, stride_speed=140.0)


def cast_utility(arm):
    # Upper body only (it plays over locomotion): a quick palm thrust, fire on the push.
    keys = [
        (0, RELAXED, None),
        (4, merge(arms((0.3, 0.15, -1), (0.22, 0.3, -1), (0.6, -0.35, 0.1), (-0.45, 0.55, 0.6)), {
            "spine_02": (-4, 0, -14), "chest": (-2, 0, -10), "neck": (0, 0, 8), "head": (0, 0, 10)}), None),
        (7, merge(arms((0.3, -0.4, -1), (0.2, -0.2, -1), (0.12, 1, 0.12), (0.04, 1, 0.14)), {
            "hand_r": (-25, 0, 0), "spine_01": (5, 0, 4), "spine_02": (4, 0, 12), "chest": (3, 0, 8),
            "neck": (0, 0, -8), "head": (-4, 0, -8)}), None),
        (10, merge(arms((0.3, -0.35, -1), (0.2, -0.15, -1), (0.1, 1, 0.08), (0.03, 1, 0.1)), {
            "hand_r": (-20, 0, 0), "spine_01": (5, 0, 5), "spine_02": (4, 0, 13), "chest": (3, 0, 9),
            "neck": (0, 0, -8), "head": (-4, 0, -9)}), None),
        (18, RELAXED, None),
    ]
    return _clip(arm, "cast_utility", 18, keys, layer="upper", bones=rig.UPPER_BODY, markers={"fire": 7})


def attack_melee_alt(arm):
    # Ranged champions forced into melee (Close Quarters): an overhead bash with the right hand.
    stance = {"thigh_l": (-22, 0, 0), "calf_l": (24, 0, 0), "foot_l": (-2, 0, 0),
              "thigh_r": (8, 0, 0), "calf_r": (20, 0, 0), "foot_r": (-28, 0, 0)}
    ready = merge(arms((0.3, 0.1, -1), (0.22, 0.4, -1), (0.3, 0.3, -1), (0.08, 1, -0.4)), stance, {"spine_01": (4, 0, 0)})
    keys = [
        (0, ready, {"pelvis": (0, 0, -0.04)}),
        (6, merge(arms((0.3, 0.8, -0.3), (0.0, 1, 0.0), (0.3, -0.3, 1), (0.1, -0.8, -0.5)), stance, {
            "spine_01": (-8, 0, -8), "spine_02": (-6, 0, -10), "chest": (-4, 0, -6), "head": (10, 0, 12)}), {"pelvis": (0, 0.02, -0.05)}),
        (9, merge(arms((0.3, -0.5, -1), (0.2, -0.3, -1), (0.08, 1, -0.2), (0.0, 1, -0.6)), stance, {
            "spine_01": (14, 0, 6), "spine_02": (10, 0, 10), "chest": (6, 0, 6), "head": (-14, 0, -10)}), {"pelvis": (0, -0.04, -0.08)}),
        (13, merge(arms((0.3, -0.4, -1), (0.2, -0.2, -1), (0.1, 0.6, -1), (0.0, 0.8, -1)), stance, {
            "spine_01": (16, 0, 7), "spine_02": (11, 0, 11), "chest": (6, 0, 6), "head": (-16, 0, -11)}), {"pelvis": (0, -0.04, -0.09)}),
        (22, ready, {"pelvis": (0, 0, -0.04)}),
    ]
    return _clip(arm, "attack_melee_alt", 22, keys, markers={"fire": 9})


def cc_stunned(arm):
    limp = merge(arms((0.22, 0.1, -1), (0.15, 0.25, -1)), sym("thigh", -10, 0, 0), sym("calf", 18, 0, 0),
                 sym("foot", -8, 0, 0), {"spine_01": (10, 0, 0), "spine_02": (6, 0, 0)})
    sway = [((15, 12, 0), (0, -4, 0), (0.015, 0, -0.04)), ((6, 0, 10), (0, 0, -3), (0, 0, -0.035)),
            ((15, -12, 0), (0, 4, 0), (-0.015, 0, -0.04)), ((22, 0, -10), (0, 0, 3), (0, 0, -0.045))]
    keys = [(i * 10, merge(limp, {"head": h, "chest": c}), {"pelvis": p}) for i, (h, c, p) in enumerate(sway)]
    return _clip(arm, "cc_stunned", 40, keys, loop=True)


def cc_rooted(arm):
    # Additive, lower body: a struggle that layers over idle, attacks and casts.
    tug_l = {"thigh_l": (-16, 0, 0), "calf_l": (26, 0, 0), "foot_l": (-8, 0, 0), "pelvis": (0, -4, 0)}
    keys = [(0, {}, None), (6, tug_l, None), (12, {}, None), (18, mirror(tug_l), None)]
    return _clip(arm, "cc_rooted", 24, keys, loop=True, layer="additive", bones=rig.LOWER_BODY)


def cc_airborne(arm):
    # Launch, a flailing loop between loop_in and loop_out, then the landing. The sim supplies
    # the height; the clip stays in place.
    launch = merge(arms((0.7, 0.2, 0.7), (0.6, 0.2, 1)), sym("thigh", -20, 0, 0), sym("calf", 40, 0, 0),
                   {"spine_01": (-15, 0, 0), "head": (-20, 0, 0)})
    tuck_a = merge(arms((1, 0, 0.5), (0.6, 0.3, 1)),
                   {"thigh_l": (-45, 0, 0), "calf_l": (70, 0, 0), "thigh_r": (-30, 0, 0), "calf_r": (55, 0, 0),
                    "spine_01": (-10, 0, 0), "head": (-15, 0, 0)})
    tuck_b = merge(arms((0.8, -0.3, 0.8), (0.4, 0.5, 1), (1, 0.4, 0.1), (0.8, 0.6, -0.3)),
                   {"thigh_l": (-25, 0, 0), "calf_l": (45, 0, 0), "thigh_r": (-55, 0, 0), "calf_r": (75, 0, 0),
                    "spine_01": (-20, 4, 0), "head": (-25, 0, 0)})
    land_prep = merge(arms((0.9, 0.3, 0), (0.9, 0.4, 0.2)), sym("thigh", -30, 0, 0), sym("calf", 35, 0, 0), sym("foot", -5, 0, 0))
    land = merge(arms((0.4, 0.8, -0.6), (0.2, 1, -0.3)), sym("thigh", -40, 0, 0), sym("calf", 70, 0, 0),
                 sym("foot", -30, 0, 0), {"spine_01": (20, 0, 0), "spine_02": (8, 0, 0), "head": (-15, 0, 0)})
    keys = [(0, launch, None), (6, tuck_a, None), (16, tuck_b, None), (26, tuck_a, None), (30, land_prep, None),
            (34, land, {"pelvis": (0, 0, -0.18)})]
    return _clip(arm, "cc_airborne", 34, keys, markers={"loop_in": 6, "loop_out": 26})


def cc_knockback(arm):
    # Held while the sim slides the body: leaning back, reaching forward.
    base = merge(arms((0.35, 1, -0.2), (0.25, 1, 0.1)), sym("thigh", -20, 0, 0), sym("calf", 10, 0, 0),
                 sym("foot", -10, 0, 0), {"spine_01": (-15, 0, 0), "spine_02": (-10, 0, 0), "head": (15, 0, 0)})
    wobble = merge(base, arms((0.45, 1, 0.1), (0.3, 1, 0.35)), {"spine_01": (-18, 0, 0), "head": (18, 0, 0)})
    return _clip(arm, "cc_knockback", 20, [(0, base, {"pelvis": (0, 0.03, -0.03)}), (10, wobble, {"pelvis": (0, 0.04, -0.04)})], loop=True)


def cc_suppressed(arm):
    hunch = merge(arms((0.12, -0.05, -1), (-0.3, 0.6, -0.5)), sym("thigh", -15, 0, 0), sym("calf", 30, 0, 0),
                  sym("foot", -15, 0, 0), {"spine_01": (20, 0, 0), "chest": (10, 0, 0)})
    keys = []
    for i in range(10):
        s = 1 if i % 2 else -1
        keys.append((i * 3, merge(hunch, {"spine_02": (0, 0, 3 * s), "head": (10, 0, 4 * s)}), {"pelvis": (0, 0, -0.08)}))
    return _clip(arm, "cc_suppressed", 30, keys, loop=True)


def cc_sleep(arm):
    slump = merge(arms((0.12, 0.05, -1), (0.08, 0.2, -1)), sym("thigh", -6, 0, 0), sym("calf", 12, 0, 0),
                  sym("foot", -6, 0, 0), {"spine_01": (15, 0, 0), "chest": (8, 0, 0), "neck": (15, 0, 0), "head": (25, 8, 0)})
    breathe = merge(slump, {"spine_01": (12, 0, 0), "chest": (4, 0, 0)}, sym("clavicle", 0, -4, 0))
    return _clip(arm, "cc_sleep", 60, [(0, slump, {"pelvis": (0, 0, -0.02)}), (30, breathe, {"pelvis": (0, 0, -0.01)})], loop=True)


def cc_forced_move(arm):
    # Additive over `walk` (fear, charm, taunt): cower, arms up, glance back over the shoulders.
    cower = merge(arms((0.35, 0.7, 0.4), (-0.5, 0.2, 1)),
                  {"spine_01": (12, 0, 0), "spine_02": (10, 0, 0), "chest": (6, 0, 0), "head": (12, 0, 0)})
    keys = [(0, cower, None), (8, merge(cower, {"neck": (0, 0, 25)}), None), (16, cower, None),
            (24, merge(cower, {"neck": (0, 0, -25)}), None)]
    return _clip(arm, "cc_forced_move", 32, keys, loop=True, layer="additive", bones=rig.UPPER_BODY)


# Fallbacks (A3): what a champion without its own idle, run or death plays, so a pack missing
# them (or the template) still animates.
def idle(arm):
    stand = merge(arms((0.27, 0.04, -1), (0.2, 0.22, -1)), sym("thigh", -3, 0, 0), sym("calf", 6, 0, 0),
                  sym("foot", -3, 0, 0), {"spine_01": (2, 0, 0), "head": (-2, 0, 0)})
    inhale = merge(stand, arms((0.29, 0.03, -1), (0.22, 0.2, -1)), sym("clavicle", 0, -3, 0),
                   {"spine_01": (0, 0, 0), "chest": (-2, 0, 0), "head": (-3, 0, 2)})
    return _clip(arm, "idle", 60, [(0, stand, {"pelvis": (0, 0, -0.01)}), (30, inhale, {"pelvis": (0, 0, 0.0)})], loop=True)


def run(arm):
    # Champion speed (330 u/s): two 1.1 m strides in 20 frames, leaning in, arms pumping.
    half = [
        (0, merge(arms((0.18, -0.55, -1), (0.1, 0.4, -0.3), (0.18, 0.7, -0.6), (0.0, 1, 0.5)), {
            "thigh_l": (-38, 0, 0), "calf_l": (14, 0, 0), "foot_l": (-10, 0, 0),
            "thigh_r": (24, 0, 0), "calf_r": (40, 0, 0), "foot_r": (22, 0, 0),
            "pelvis": (0, 0, -7), "spine_01": (10, 0, 4), "spine_02": (4, 0, 6), "head": (-10, 0, -4)}), {"pelvis": (0, 0, -0.03)}),
        (3, merge(arms((0.18, -0.4, -1), (0.1, 0.5, -0.2), (0.18, 0.55, -0.7), (0.0, 1, 0.35)), {
            "thigh_l": (-24, 0, 0), "calf_l": (34, 0, 0), "foot_l": (4, 0, 0),
            "thigh_r": (18, 0, 0), "calf_r": (70, 0, 0), "foot_r": (20, 0, 0),
            "pelvis": (0, 0, -5), "spine_01": (11, 0, 3), "spine_02": (4, 0, 4), "head": (-11, 0, -3)}), {"pelvis": (0, 0, -0.06)}),
        (6, merge(arms((0.18, 0.0, -1), (0.1, 0.7, -0.3)), {
            "thigh_l": (-2, 0, 0), "calf_l": (12, 0, 0), "foot_l": (-2, 0, 0),
            "thigh_r": (-30, 0, 0), "calf_r": (95, 0, 0), "foot_r": (10, 0, 0),
            "pelvis": (0, 0, 0), "spine_01": (10, 0, 0), "spine_02": (4, 0, 0), "head": (-10, 0, 0)}), {"pelvis": (0, 0, 0.02)}),
    ]
    keys = half + [(f + 10, mirror(p), _mirror_locs(l)) for f, p, l in half]
    return _clip(arm, "run", 20, keys, loop=True, markers={"foot_l": 0, "foot_r": 10}, stride_speed=330.0)


def death(arm):
    # Staggers, buckles and falls backward; the last frame holds (the body lies where it fell).
    keys = [
        (0, RELAXED, None),
        (5, merge(arms((0.7, -0.2, 0.2), (0.6, 0.3, 0.5)), {"spine_01": (-14, 0, 0), "spine_02": (-8, 0, 0), "head": (-22, 0, 0)}),
         {"pelvis": (0, 0.05, 0)}),
        (13, merge(arms((0.6, 0.3, -0.4), (0.4, 0.6, -0.4)), sym("thigh", -45, 0, 0), sym("calf", 85, 0, 0), sym("foot", -30, 0, 0),
                   {"spine_01": (12, 0, 0), "spine_02": (6, 0, 0), "head": (10, 0, 0)}), {"pelvis": (0, 0.08, -0.38)}),
        (21, merge(arms((0.8, -0.4, 0.2), (0.7, -0.2, 0.4)), sym("thigh", -55, 0, 0), sym("calf", 60, 0, 0),
                   {"pelvis": (-60, 0, 0), "spine_01": (-6, 0, 0), "head": (-10, 0, 0)}), {"pelvis": (0, 0.4, -0.72)}),
        (30, merge(arms((0.9, -0.5, 0.1), (0.9, -0.3, 0.0)), sym("thigh", -12, 0, 0), sym("calf", 14, 0, 0), sym("foot", 10, 0, 0),
                   {"pelvis": (-86, 0, 0), "spine_01": (-4, 0, 0), "head": (-8, 0, 12)}), {"pelvis": (0, 0.55, -0.86)}),
    ]
    return _clip(arm, "death", 30, keys)


ALL = [walk, cast_utility, attack_melee_alt, cc_stunned, cc_rooted, cc_airborne, cc_knockback, cc_suppressed,
       cc_sleep, cc_forced_move, idle, run, death]


def blockout(arm):
    """Create every shared clip on `arm` (the reference rig). Returns the action names."""
    return [f(arm).name for f in ALL]
