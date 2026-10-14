# SPDX-License-Identifier: AGPL-3.0-or-later
"""Shared helpers for champion generators (14): planted feet, spins, arm aims, mirrored parts,
the shared library bake and the command line every `art/champions/<id>/build.py` uses.
"""

import math
import os
import sys

import bpy
from mathutils import Quaternion, Vector

from . import clips, head, mesh, rig
from .clips import at, ik

SHARED = ["walk", "cast_utility", "cc_stunned", "cc_rooted", "cc_airborne", "cc_knockback", "cc_suppressed", "cc_sleep",
          "cc_forced_move"]
ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", ".."))


def ankle():
    """The ankle's height in the current shape (where foot IK targets sit)."""
    return next(b for b in rig.bone_table() if b[0] == "foot_l")[2][2]


def feet(l=(0.14, 0.04), r=(0.14, -0.06), lift_l=0.0, lift_r=0.0):
    """Both feet planted at (out, fwd) on the ground (or lifted)."""
    z = ankle()
    return {"ik_foot_l": ik(at(l[0], l[1], z + lift_l, "l")), "ik_foot_r": ik(at(r[0], r[1], z + lift_r, "r"))}


def hand(side, point, aim=None, pole=None):
    """An IK target for one hand: its wrist at `point` (armature space)."""
    return {f"ik_hand_{side}": ik(point, aim=aim, pole=pole)}


def arm(side, out, fwd, up, f_out, f_fwd, f_up):
    """Aim one arm's upper arm and forearm (character space)."""
    return {f"upperarm_{side}": clips.aim(out, fwd, up, side), f"forearm_{side}": clips.aim(f_out, f_fwd, f_up, side)}


def spin(pose, deg):
    """The pose turned `deg` degrees about the vertical: aims, IK targets and the pelvis turn."""
    q = Quaternion(Vector((0, 0, 1)), math.radians(deg))
    out = {}
    for k, v in pose.items():
        if isinstance(v, dict) and "aim" in v and len(v) == 1:
            out[k] = {"aim": tuple(q @ Vector(v["aim"]))}
        elif isinstance(v, dict) and "to" in v:
            out[k] = dict(v, to=tuple(q @ Vector(v["to"])), aim=None if v.get("aim") is None else tuple(q @ Vector(v["aim"])))
        else:
            out[k] = v
    px, py, pz = out.get("pelvis", (0, 0, 0))
    out["pelvis"] = (px, py, pz + deg)
    return out


def mirrored(left):
    """A left-side part list plus its mirror: lofted tuples, `box_part`s and `path_part`s."""
    out = list(left)
    for p in left:
        if callable(p) and hasattr(p, "mftr_box"):
            out += mesh.box_pair(*p.mftr_box)[1:]
        elif callable(p) and hasattr(p, "mftr_path"):
            out += mesh.path_pair(*p.mftr_path)[1:]
        elif not callable(p):
            out.append(mesh._mirror_part(p))
    return out


def bake_shared(armature, names=None):
    """Copy the shared library's clips in (10 §7.3): the same bones, so a direct copy."""
    names = names or SHARED
    path = os.path.join(ROOT, "art", "library", "biped", "biped_library.blend")
    with bpy.data.libraries.load(path, link=False) as (src, dst):
        dst.actions = [n for n in src.actions if n in names]
    for act in dst.actions:
        act.use_fake_user = True
        clips.stash(armature, act)
    return [a.name for a in dst.actions]


def detailed(parts):
    """The detail pass (D59, 14 §2.3): round lofts (8+ sides) get four more sides and smoothed
    in-between rings, so round masses read round; blocky parts and hand-made paths are left
    as authored."""
    out = []
    for p in parts:
        if not callable(p) and p[3] >= 8:
            bone, a, b, sides, prof, slot, color = p
            out.append((bone, a, b, min(sides + 4, 16), mesh._smooth(prof) if len(prof) >= 3 else prof, slot, color))
        else:
            out.append(p)
    return out


def hand_parts(side, color, slot="skin", scale=1.0, knuckles=None):
    """A sculpted hand (14 §2.2): a palm on `hand_<side>`, four tapered fingers on
    `fingers_<side>` (they curl together), a thumb; `knuckles` colors a guard across them."""
    bones = {b[0]: b for b in rig.bone_table()}
    hh, ht = Vector(bones[f"hand_{side}"][2]), Vector(bones[f"hand_{side}"][3])
    fh, ft = Vector(bones[f"fingers_{side}"][2]), Vector(bones[f"fingers_{side}"][3])
    d = (ft - fh).normalized()
    across = Vector((0, 1, 0))
    across = (across - d * across.dot(d)).normalized()
    curl = d.cross(across).normalized() * (1 if side == "l" else -1)
    s = scale
    out = [mesh.path_part(f"hand_{side}", [tuple(hh - (ht - hh) * 0.05), tuple(hh + (ht - hh) * 0.5), tuple(ht)], 6,
                          [(.036 * s, .02 * s), (.044 * s, .024 * s), (.042 * s, .02 * s)], slot, color, ref=tuple(across))]
    length = (ft - fh).length * 1.05
    for k, (off, lf) in enumerate(((-1.5, 0.82), (-0.5, 1.0), (0.5, 0.96), (1.5, 0.80))):
        base = fh + across * (off * 0.0215 * s)
        mid = base + d * (length * lf * 0.5) - curl * 0.004 * s
        tip = base + d * (length * lf) - curl * 0.012 * s
        out.append(mesh.path_part(f"fingers_{side}", [tuple(base), tuple(mid), tuple(tip)], 5,
                                  [(.0115 * s, .011 * s), (.0105 * s, .010 * s), (.008 * s, .007 * s)], slot, color, ref=tuple(across)))
    out.append(mesh.path_part(f"thumb_{side}", [(f"thumb_{side}", -0.1), (f"thumb_{side}", 0.5), (f"thumb_{side}", 1.15)], 5,
                              [(.016 * s, .015 * s), (.014 * s, .013 * s), (.010 * s, .009 * s)], slot, color))
    if knuckles:
        out.append(mesh.path_part(f"fingers_{side}", [tuple(fh - across * 0.05 * s), tuple(fh + across * 0.05 * s)], 4,
                                  [(.016 * s, .02 * s)] * 2, "metal", knuckles, ref=tuple(d)))
    return out


def build(name, shape, parts, own, height, extras=None, only=None, shared=None, detail=2):
    """Build the champion: armature in `shape`, `extras(armature)` (chains), clips, mesh (with
    the detail pass and fine heads at `detail` 2), the shared clips. Returns (armature, body)."""
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig.use_shape(shape)
    mesh.AO_HEIGHT = height
    armature = rig.build_armature(name=name)
    if extras:
        extras(armature)
    made = [f(armature).name for f in own if only is None or f.__name__ in only]
    head.DEFAULT_DETAIL = detail
    try:
        part_list = parts()
        body = mesh.build_mannequin(armature, name=name, part_list=detailed(part_list) if detail >= 2 else part_list)
    finally:
        head.DEFAULT_DETAIL = 1
    baked = bake_shared(armature, shared) if only is None else []
    print(f"{name}: {len(armature.data.bones)} bones, {mesh.triangle_count(body)} triangles, {len(made)} own clips, shared {baked}")
    return armature, body


def main(build_fn):
    """The generator's command line: `-- --out FILE.blend [--only clip,clip]`."""
    argv = sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else []
    only = argv[argv.index("--only") + 1].split(",") if "--only" in argv else None
    build_fn(only)
    if "--out" in argv:
        out = os.path.abspath(argv[argv.index("--out") + 1])
        os.makedirs(os.path.dirname(out), exist_ok=True)
        bpy.ops.wm.save_as_mainfile(filepath=out, compress=True)
        print(f"saved {out}")
