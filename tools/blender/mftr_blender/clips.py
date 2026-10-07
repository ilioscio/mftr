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


def key_pose(armature, frame, pose, bones=None, locs=None):
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
        pb.rotation_quaternion = rest.inverted() @ q_arm @ rest
        pb.keyframe_insert("rotation_quaternion", frame=frame, group=name)
        if name in locs:
            pb.location = rest.inverted() @ Vector(locs[name])
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
