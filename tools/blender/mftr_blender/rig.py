# SPDX-License-Identifier: AGPL-3.0-or-later
"""The `biped` v1 reference skeleton (10 §7.2).

The character faces -Y (it looks at you in Front view), which the glTF exporter turns into +Z,
Godot's MODEL_FRONT. Left is +X. Z is up. 1 Blender unit = 1 m = 100 game units. A-pose.

Rolls: every bone's local Z axis is aligned to "forward" (-Y), or to "up" for bones that lie
along Y (root, feet, props), so mirrored bones get Blender's symmetric rolls and X-mirror
posing works.
"""

import bpy
from mathutils import Vector

ARCHETYPE = "biped"
VERSION = 1

FORWARD = Vector((0.0, -1.0, 0.0))
UP = Vector((0.0, 0.0, 1.0))

# (name, parent, head, tail, deform, roll-axis). Left side only for paired bones; `_mirror`
# adds the right side. Heights are for a ~1.9 m standard biped (10 §2).
_CENTER = [
    ("root", None, (0, 0, 0), (0, -0.30, 0), False, UP),
    ("pelvis", "root", (0, 0, 1.00), (0, 0, 1.12), True, FORWARD),
    ("spine_01", "pelvis", (0, 0, 1.12), (0, 0, 1.26), True, FORWARD),
    ("spine_02", "spine_01", (0, 0, 1.26), (0, 0, 1.40), True, FORWARD),
    ("chest", "spine_02", (0, 0, 1.40), (0, 0, 1.56), True, FORWARD),
    ("neck", "chest", (0, 0, 1.56), (0, -0.01, 1.64), True, FORWARD),
    ("head", "neck", (0, -0.01, 1.64), (0, -0.01, 1.88), True, FORWARD),
]

_LEFT = [
    ("clavicle_l", "chest", (0.03, -0.01, 1.52), (0.17, 0.01, 1.53), True, FORWARD),
    ("upperarm_l", "clavicle_l", (0.17, 0.01, 1.53), (0.38, 0.02, 1.32), True, FORWARD),
    ("forearm_l", "upperarm_l", (0.38, 0.02, 1.32), (0.56, -0.01, 1.13), True, FORWARD),
    ("hand_l", "forearm_l", (0.56, -0.01, 1.13), (0.63, -0.02, 1.05), True, FORWARD),
    ("fingers_l", "hand_l", (0.63, -0.02, 1.05), (0.68, -0.03, 0.99), True, FORWARD),
    ("thumb_l", "hand_l", (0.58, -0.05, 1.10), (0.60, -0.09, 1.06), True, FORWARD),
    ("prop_l", "hand_l", (0.61, -0.03, 1.07), (0.61, -0.23, 1.07), False, UP),
    ("thigh_l", "pelvis", (0.10, 0, 1.00), (0.11, -0.01, 0.54), True, FORWARD),
    ("calf_l", "thigh_l", (0.11, -0.01, 0.54), (0.12, 0.02, 0.09), True, FORWARD),
    ("foot_l", "calf_l", (0.12, 0.02, 0.09), (0.12, -0.10, 0.03), True, UP),
    ("toe_l", "foot_l", (0.12, -0.10, 0.03), (0.12, -0.17, 0.03), True, UP),
]

# Non-deforming attachment points (10 §7.2): where projectiles spawn, trails attach, casts
# originate, impacts land and the health bar sits.
_SOCKETS = [
    ("socket_projectile", "prop_r", (-0.61, -0.25, 1.07), (-0.61, -0.35, 1.07), False, UP),
    ("socket_weapon_tip", "prop_r", (-0.61, -0.45, 1.07), (-0.61, -0.55, 1.07), False, UP),
    ("socket_cast", "hand_r", (-0.64, -0.06, 1.06), (-0.64, -0.16, 1.06), False, UP),
    ("socket_chest", "chest", (0, -0.16, 1.45), (0, -0.26, 1.45), False, UP),
    ("socket_overhead", "root", (0, 0, 2.15), (0, -0.10, 2.15), False, UP),
]

REQUIRED_SOCKETS = [s[0] for s in _SOCKETS]


def _mirror(entry):
    name, parent, head, tail, deform, axis = entry
    swap = lambda n: n[:-2] + "_r" if n and n.endswith("_l") else n  # noqa: E731
    return (swap(name), swap(parent), (-head[0], head[1], head[2]), (-tail[0], tail[1], tail[2]), deform, axis)


def bone_table():
    """Every bone of the reference skeleton, parents before children."""
    return _CENTER + _LEFT + [_mirror(b) for b in _LEFT] + _SOCKETS


BONE_NAMES = [b[0] for b in bone_table()]
DEFORM_BONES = [b[0] for b in bone_table() if b[4]]
LOWER_BODY = ["pelvis"] + [b for b in BONE_NAMES if b.split("_")[0] in ("thigh", "calf", "foot", "toe")]
UPPER_BODY = [b for b in DEFORM_BONES if b not in LOWER_BODY]


def build_armature(name="biped_v1", collection=None):
    """Create the reference armature object in A-pose and return it."""
    data = bpy.data.armatures.new(name)
    obj = bpy.data.objects.new(name, data)
    (collection or bpy.context.scene.collection).objects.link(obj)
    data.display_type = "STICK"
    obj.show_in_front = True

    prev_active = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    for bname, parent, head, tail, deform, axis in bone_table():
        eb = data.edit_bones.new(bname)
        eb.head = head
        eb.tail = tail
        eb.use_deform = deform
        eb.align_roll(axis)
        if parent:
            eb.parent = data.edit_bones[parent]
            # Chains are connected where the child starts exactly at the parent's tail.
            eb.use_connect = (Vector(eb.head) - data.edit_bones[parent].tail).length < 1e-6
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev_active

    for pb in obj.pose.bones:
        pb.rotation_mode = "QUATERNION"
    obj["mftr_archetype"] = ARCHETYPE
    obj["mftr_rig_version"] = VERSION
    return obj
