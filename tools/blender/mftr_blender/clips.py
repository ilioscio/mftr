# SPDX-License-Identifier: AGPL-3.0-or-later
"""Clip names, markers and authoring helpers (10 §4.5, §5).

The authoritative checks live in the Rust validator (`mftr-tools pack validate`, crate
`mftr-pack`); this module mirrors its names so the add-on can help while authoring.

Conventions stored on each action (custom properties, exported to the `.anims.ron` sidecar):
  mftr_loop          bool  the clip loops (its last frame equals its first)
  mftr_layer         str   "full", "upper" (spine_01 and up, over locomotion) or "additive"
  mftr_stride_speed  float locomotion only: ground speed the cycle was authored for, in u/s
"""

import math

import bpy
from mathutils import Euler, Quaternion, Vector

from . import rig

FPS = 30

# 10 §5: every champion ships these (abilities add their own, derived from the kit).
REQUIRED_CHAMPION = [
    "idle", "idle_fidget_1", "idle_fidget_2", "idle_ready", "run", "run_fast", "walk",
    "attack_1", "attack_2",
    "cast_utility", "recall", "death", "respawn", "select",
    "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed", "cc_sleep", "cc_forced_move",
    "emote_taunt", "emote_joke", "emote_laugh", "emote_dance",
]
# 10 §7.3: the shared library (retargeted onto each champion at export).
SHARED_LIBRARY = [
    "walk", "cast_utility", "attack_melee_alt",
    "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed", "cc_sleep", "cc_forced_move",
]

MARKERS = ["fire", "end", "loop_in", "loop_out", "foot_l", "foot_r"]  # plus hit_<n>, fx_<name>, sfx_<name>
# Fallbacks the library also carries (A3): played by champions that lack their own.
LIBRARY_FALLBACKS = ["idle", "run", "death"]
LAYERS = ("full", "upper", "additive")


def new_action(armature, name, frames, loop=False, layer="full", stride_speed=None):
    """Create (or reset) the action `name`, make it the armature's active action and return it."""
    old = bpy.data.actions.get(name)
    if old:
        bpy.data.actions.remove(old)
    act = bpy.data.actions.new(name)
    act.use_fake_user = True
    act.use_frame_range = True
    act.frame_range = (0, frames)
    act["mftr_loop"] = bool(loop)
    act["mftr_layer"] = layer
    if stride_speed is not None:
        act["mftr_stride_speed"] = float(stride_speed)
    armature.animation_data_create()
    armature.animation_data.action = act
    return act


def set_marker(action, name, frame):
    """Add or move the pose marker `name` (one per name)."""
    for m in action.pose_markers:
        if m.name == name:
            m.frame = frame
            return m
    m = action.pose_markers.new(name)
    m.frame = frame
    return m


def stash(armature, action):
    """Put `action` on its own muted NLA track (how the exporter finds every clip)."""
    ad = armature.animation_data_create()
    for tr in list(ad.nla_tracks):
        if tr.name == action.name:
            ad.nla_tracks.remove(tr)
    tr = ad.nla_tracks.new()
    tr.name = action.name
    strip = tr.strips.new(action.name, 0, action)
    # Actions appended from another file (the shared library) keep their slot; make sure the
    # strip uses one (Blender 4.4+ slotted actions).
    if getattr(strip, "action_slot", None) is None and getattr(action, "slots", None):
        strip.action_slot = action.slots[0]
    tr.mute = True
    ad.action = None


def _rest_quat(pbone):
    return pbone.bone.matrix_local.to_quaternion()


def key_pose(armature, frame, pose, bones=None, locs=None, prev=None):
    """Key a pose at `frame` (see `_key_pose`). A pose may hold `grip_l` / `grip_r` (`grip`):
    that arm is solved so its hand holds a point on another bone (two-handed weapons, 14 §3.3).
    `prev` (bone -> quaternion) keeps each bone's quaternion on the same hemisphere as its last
    key, so interpolation takes the short way (and spins keep turning)."""
    iks = {k: pose[k] for k in IK_KEYS if k in pose}
    grips = {k: pose[k] for k in ("grip_l", "grip_r") if k in pose}
    pose = {k: v for k, v in pose.items() if k not in grips and k not in iks}
    if iks:
        _key_pose(armature, frame, pose, bones, locs, insert=False)
        bpy.context.view_layer.update()
        for key, spec in iks.items():
            pose = merge(pose, _solve_ik(armature, key, spec))
    if grips:
        _key_pose(armature, frame, pose, bones, locs, insert=False)
        bpy.context.view_layer.update()
        for key, spec in grips.items():
            pose = merge(pose, _solve_grip(armature, key[-1], spec))
    _key_pose(armature, frame, pose, bones, locs, prev=prev)


# IK targets a pose may hold (14 §3): a hand or a foot placed at a point in armature space
# (meters; X = the character's left, -Y = forward, Z = up), so weapons rest on the ground and
# feet stay planted while the body moves.
IK_KEYS = ("ik_hand_l", "ik_hand_r", "ik_foot_l", "ik_foot_r")
_CHAINS = {"hand": ("upperarm", "forearm", "hand"), "foot": ("thigh", "calf", "foot")}


def at(out, fwd, up, side="l"):
    """A point in armature space from character terms: `out` from the midline on `side`."""
    return ((out if side == "l" else -out), -fwd, up)


def ik(to, aim=None, pole=None):
    """An IK target for `key_pose` (`ik_hand_r=ik(...)`): the end bone's head at `to` (armature
    space, `at` builds one), the end bone aimed along `aim` (armature space; None keeps the
    pose's own), the middle joint bent toward `pole` (out, fwd, up for that side; the default
    bends elbows out and back, knees forward)."""
    return {"to": to, "aim": aim, "pole": pole}


def _two_bone(armature, side, chain, end_at, pole):
    """Aims for a chain's upper and lower bones so the lower bone's tail lands on `end_at`."""
    pb = armature.pose.bones
    upper, lower = pb[f"{chain[0]}_{side}"], pb[f"{chain[1]}_{side}"]
    s = upper.head.copy()
    l1, l2 = upper.bone.length, lower.bone.length
    d_vec = Vector(end_at) - s
    d = max(min(d_vec.length, (l1 + l2) * 0.999), abs(l1 - l2) + 1e-4)
    dirn = d_vec.normalized()
    out, fwd, upv = pole
    p = Vector(((out if side == "l" else -out), -fwd, upv))
    p = p - dirn * p.dot(dirn)
    p = p.normalized() if p.length > 1e-6 else Vector((0, 0, -1))
    a = (l1 * l1 - l2 * l2 + d * d) / (2 * d)
    h = math.sqrt(max(l1 * l1 - a * a, 0.0))
    mid = s + dirn * a + p * h
    end = s + dirn * d
    return {f"{chain[0]}_{side}": {"aim": tuple(mid - s)}, f"{chain[1]}_{side}": {"aim": tuple(end - mid)}}, end


def _solve_ik(armature, key, spec):
    _, kind, side = key.split("_")
    chain = _CHAINS[kind]
    pole = spec.get("pole") or ((0.0, 1.0, 0.1) if kind == "foot" else (1.0, -0.5, -0.6))
    out, _ = _two_bone(armature, side, chain, spec["to"], pole)
    if spec.get("aim") is not None:
        out[f"{chain[2]}_{side}"] = {"aim": tuple(spec["aim"])}
    elif kind == "foot":
        # A planted foot stays level, pointing where the character faces.
        out[f"{chain[2]}_{side}"] = {"aim": (0.0, -1.0, -0.42)}
    return out


def grip(on="hand_r", at=0.3, pole=(1.0, -0.4, -1.0), reach=0.075, curl=70.0):
    """A grip for `key_pose`: put this side's hand on bone `on`'s axis, `at` meters from its
    head (negative: behind it), the elbow bent toward `pole` (out, fwd, up in character space,
    for this side). `reach` is the wrist-to-palm distance; `curl` bends the fingers around."""
    return {"on": on, "at": at, "pole": pole, "reach": reach, "curl": curl}


def _solve_grip(armature, side, spec):
    """The upper arm, forearm, hand and fingers of `side` aimed so the palm closes on the grip
    point (analytic two-bone IK on the posed armature)."""
    pb = armature.pose.bones
    on = pb[spec["on"]]
    axis = (on.tail - on.head).normalized()
    target = on.head + axis * spec["at"]
    s = pb[f"upperarm_{side}"].head.copy()
    # The wrist sits `reach` from the haft, on the shoulder's side of it.
    to_s = s - target
    across = (to_s - axis * to_s.dot(axis))
    across = across.normalized() if across.length > 1e-6 else Vector((0, 0, 1))
    wrist = target + across * spec["reach"]
    out_pose, wrist = _two_bone(armature, side, _CHAINS["hand"], wrist, spec["pole"])
    out_pose[f"hand_{side}"] = {"aim": tuple(target - wrist)}
    if spec.get("curl"):
        out_pose[f"fingers_{side}"] = (spec["curl"], 0, 0)
        out_pose[f"thumb_{side}"] = (spec["curl"] * 0.5, 0, 0)
    return out_pose


# How a key leaves for the next one (14 §3.4): (interpolation, easing).
EASES = {
    "ease": ("BEZIER", "AUTO"),      # the default: eases out of this key and into the next
    "snap": ("LINEAR", "AUTO"),      # constant speed: strikes, whips, impacts
    "whip": ("CUBIC", "EASE_IN"),    # slow out of this key, accelerating into the next: swings
    "settle": ("CUBIC", "EASE_OUT"), # fast out of this key, settling into the next: recoveries
    "hold": ("CONSTANT", "AUTO"),    # frozen until the next key (rare: a frame-perfect hold)
}


def set_ease(armature, action, frame, ease):
    """Set how every channel's key at `frame` leaves (`EASES`)."""
    from bpy_extras import anim_utils
    interp, easing = EASES[ease]
    slot = armature.animation_data.action_slot
    bag = anim_utils.action_get_channelbag_for_slot(action, slot)
    if bag is None:
        return
    for fc in bag.fcurves:
        for kp in fc.keyframe_points:
            if abs(kp.co[0] - frame) < 1e-3:
                kp.interpolation = interp
                kp.easing = easing


def _key_pose(armature, frame, pose, bones=None, locs=None, insert=True, prev=None):
    """Key a pose at `frame`.

    `pose` maps bone name -> one of:
      (rx, ry, rz)  degrees about the **armature axes** (X = character's left, Y = backward,
                    Z = up), applied relative to the parent: +rx tips up-pointing bones forward
                    and swings hanging bones backward; +rz turns left.
      aim(...)      point the bone along a direction given in character space, absolute (not
                    relative to the parent); the intuitive form for arms and legs.
    `sym(...)`/`aim_sym(...)` build left/right pairs with the mirror rule.
    `bones` lists every bone to key (unlisted ones are keyed at rest, so clips never inherit
    poses); `locs` maps bone -> (x, y, z) offset in armature axes, in meters.
    """
    bones = bones if bones is not None else list(pose)
    locs = locs or {}
    # Armature-space rotation each bone has received relative to its rest (parents first).
    delta = {}
    # Every bone, parents first (the armature's own order: extras are added after the base rig).
    for name in [b.name for b in armature.data.bones]:
        pb = armature.pose.bones[name]
        parent = pb.parent.name if pb.parent else None
        d_parent = delta.get(parent, Quaternion())
        rest = _rest_quat(pb)
        value = pose.get(name, (0, 0, 0)) if name in bones else None
        if value is None:
            q_arm = Quaternion()
        elif isinstance(value, dict):
            rest_dir = (rest @ Vector((0, 1, 0))).normalized()
            target = d_parent.inverted() @ Vector(value["aim"]).normalized()
            q_arm = rest_dir.rotation_difference(target)
        else:
            rx, ry, rz = value
            q_arm = Euler((math.radians(rx), math.radians(ry), math.radians(rz)), "XYZ").to_quaternion()
        delta[name] = d_parent @ q_arm
        if value is None:
            continue
        q = rest.inverted() @ q_arm @ rest
        if prev is not None:
            if name in prev and prev[name].dot(q) < 0:
                q.negate()
            prev[name] = q.copy()
        pb.rotation_quaternion = q
        if insert:
            pb.keyframe_insert("rotation_quaternion", frame=frame, group=name)
        if name in locs:
            pb.location = rest.inverted() @ Vector(locs[name])
            if insert:
                pb.keyframe_insert("location", frame=frame, group=name)


def sym(base, rx=0.0, ry=0.0, rz=0.0, right=None):
    """`{base_l: (rx, ry, rz), base_r: mirrored}`; `right` overrides the right side's values."""
    return {f"{base}_l": (rx, ry, rz), f"{base}_r": right if right is not None else (rx, -ry, -rz)}


def aim(out, fwd, up, side="l"):
    """A direction in character space: `out` away from the body's midline on `side`,
    `fwd` toward where the character faces, `up` up."""
    return {"aim": ((out if side == "l" else -out), -fwd, up)}


def aim_sym(base, out, fwd, up, right=None):
    """Both sides of `base` aimed the same way (mirrored); `right` = (out, fwd, up) overrides."""
    r = right if right is not None else (out, fwd, up)
    return {f"{base}_l": aim(out, fwd, up, "l"), f"{base}_r": aim(*r, side="r")}


def merge(*poses):
    out = {}
    for p in poses:
        out.update(p)
    return out


def reset_pose(armature):
    for pb in armature.pose.bones:
        pb.rotation_quaternion = Quaternion()
        pb.location = Vector()
        pb.scale = Vector((1, 1, 1))


def full_body(armature=None):
    """Every deforming bone: the base rig's, plus `armature`'s extras (capes, hair) if given."""
    if armature is None:
        return list(rig.DEFORM_BONES)
    return [b.name for b in armature.data.bones if b.use_deform]
