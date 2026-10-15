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


# Proportion shapes (A5): the same bones, names and hierarchy, placed for another body. A shape
# remaps heights piecewise-linearly (biped height -> shape height) and scales width and depth,
# then scales the whole thing. Minions are chibi: a big head, short legs, about 1.05 m tall.
SHAPES = {
    "standard": None,
    # Squat (15 §6, R06 §6): short legs, a wide body, the hooded head about a third of the
    # height; ~0.95 m to the hood's tip.
    "minion": {
        "heights": [(0.0, 0.0), (0.03, 0.02), (0.09, 0.045), (0.54, 0.17), (1.00, 0.32), (1.12, 0.38),
                    (1.40, 0.53), (1.56, 0.62), (1.64, 0.66), (1.88, 0.90), (2.15, 1.08)],
        "x": 0.84, "y": 0.84, "scale": 1.0,
    },
    # `biped_large` (10 §7.1): brutes. Taller (~2.05 m), much broader and deeper; the same bones.
    "large": {
        "heights": [(0.0, 0.0), (2.15, 2.15)],
        "x": 1.25, "y": 1.2, "scale": 1.08,
    },
}
_shape = None
_IDENTITY = {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 1.0, "y": 1.0, "scale": 1.0}
# The arm chain (14 §2.2): bones and sockets that `shoulder`, `arm` and `hand` move. Hand parts
# scale about the wrist.
_ARM = ("upperarm", "forearm")
_HAND = ("hand", "fingers", "thumb", "prop")
_RIGHT_HAND_SOCKETS = ("socket_projectile", "socket_weapon_tip", "socket_cast")


def use_shape(name, scale=1.0, x=1.0, **extra):
    """Place every bone (and everything built from `bone_table`) for shape `name` (a name in
    `SHAPES` or a dict in the same form), scaled by `scale`, `x` widening the body further.
    `use_shape("standard")` restores the biped.

    Heroic proportions (14 §2.2) add, as keywords or dict keys:
      shoulder  m  moves each arm outward (the clavicle's tip and everything below it)
      arm          scales the upper arm and forearm about the shoulder
      hand         scales the hand, fingers, thumb and prop about the wrist
    Parts attached to arms should use (bone, t) points so they follow."""
    global _shape
    base = SHAPES[name] if isinstance(name, str) else name
    if base is None and not extra:
        _shape = None
        return
    base = dict(_IDENTITY if base is None else base, **extra)
    _shape = dict(base, scale=base.get("scale", 1.0) * scale, x=base.get("x", 1.0) * x)


def _remap_z(z, table):
    for (a0, b0), (a1, b1) in zip(table, table[1:]):
        if z <= a1 or (a1, b1) == table[-1]:
            return b0 + (z - a0) * (b1 - b0) / (a1 - a0)
    return z


def shaped(p):
    """A point in biped space placed for the current shape."""
    if _shape is None:
        return tuple(p)
    s = _shape["scale"]
    return (p[0] * _shape["x"] * s, p[1] * _shape["y"] * s, _remap_z(p[2], _shape["heights"]) * s)


# A creature's own skeleton (16 §3), when one is in use: `use_table`.
_table = None
_table_archetype = None


def use_table(table, archetype="creature"):
    """Use `table` instead of the biped (a creature's skeleton, 16 §3): entries as the biped's,
    (name, parent, head, tail, deform, roll axis), parents first, in meters facing -Y. Paired
    bones are listed on the left (+X, names ending `_l`) and mirrored by `mirrored`. Everything
    built from `bone_table` (the armature, (bone, t) points of mesh parts) follows.
    `use_table(None)` restores the biped."""
    global _table, _table_archetype
    _table = list(table) if table is not None else None
    _table_archetype = archetype if table is not None else None


def mirrored(left):
    """`left` (bones ending `_l`, on +X) and their right-side mirrors."""
    return list(left) + [_mirror(b) for b in left]


def bone_table():
    """Every bone of the reference skeleton, parents before children (in the current shape), or
    the creature table in use."""
    if _table is not None:
        return list(_table)
    table = _CENTER + _LEFT + [_mirror(b) for b in _LEFT] + _SOCKETS
    out = [(n, par, shaped(h), shaped(t), d, ax) for n, par, h, t, d, ax in table]
    if _shape is None or not any(k in _shape for k in ("shoulder", "arm", "hand")):
        return out
    pos = {n: (Vector(h), Vector(t)) for n, _, h, t, _, _ in out}
    dx = _shape.get("shoulder", 0.0)
    arm = _shape.get("arm", 1.0)
    hand = _shape.get("hand", 1.0)

    def arm_point(p, side, kind):
        s = pos[f"clavicle_{side}"][1]
        w = pos[f"hand_{side}"][0]
        out_dir = Vector((1.0 if side == "l" else -1.0, 0.0, 0.0))
        s2 = s + out_dir * dx
        w2 = s2 + (w - s) * arm
        if kind == "arm":
            return tuple(s2 + (Vector(p) - s) * arm)
        return tuple(w2 + (Vector(p) - w) * hand)

    moved = []
    for n, par, h, t, d, ax in out:
        part, _, side = n.rpartition("_")
        if n.startswith("clavicle_"):
            t = tuple(Vector(t) + Vector((dx if side == "l" else -dx, 0, 0)))
        elif part in _ARM and side in ("l", "r"):
            h, t = arm_point(h, side, "arm"), arm_point(t, side, "arm")
        elif part in _HAND and side in ("l", "r"):
            h, t = arm_point(h, side, "hand"), arm_point(t, side, "hand")
        elif n in _RIGHT_HAND_SOCKETS:
            h, t = arm_point(h, "r", "hand"), arm_point(t, "r", "hand")
        moved.append((n, par, h, t, d, ax))
    return moved


BONE_NAMES = [b[0] for b in bone_table()]
DEFORM_BONES = [b[0] for b in bone_table() if b[4]]
LOWER_BODY = ["pelvis"] + [b for b in BONE_NAMES if b.split("_")[0] in ("thigh", "calf", "foot", "toe")]
UPPER_BODY = [b for b in DEFORM_BONES if b not in LOWER_BODY]


def add_chain(armature, chain, parent, points, axis=FORWARD):
    """Add `extra_<chain>_1..n` (10 §7.2): deforming bones through `points`, under `parent`."""
    prev_active = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    eb = armature.data.edit_bones
    up = eb[parent]
    for i, (a, b) in enumerate(zip(points, points[1:]), start=1):
        bone = eb.new(f"extra_{chain}_{i}")
        bone.head, bone.tail = a, b
        bone.use_deform = True
        bone.align_roll(axis)
        bone.parent = up
        bone.use_connect = i > 1
        up = bone
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev_active
    for pb in armature.pose.bones:
        pb.rotation_mode = "QUATERNION"


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
    obj["mftr_archetype"] = _table_archetype or ARCHETYPE
    obj["mftr_rig_version"] = VERSION
    return obj
