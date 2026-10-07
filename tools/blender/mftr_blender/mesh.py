# SPDX-License-Identifier: AGPL-3.0-or-later
"""The faceted template mesh ("mannequin") for the `biped` rig (10 §2).

Built from lofted prisms, one per body segment, each weighted rigidly to one bone (1 influence:
the PS1 segmented look and the crispest facets). Vertex colors carry the albedo (RGB) and a
baked height-based occlusion (A). Faces use the fixed material slots.
"""

import math

import bmesh
import bpy
from mathutils import Vector

from . import rig

# The fixed material slots (10 §2). `accent` is tinted ally/enemy at runtime.
MATERIAL_SLOTS = ("skin", "cloth", "metal", "emissive", "accent")

# Template palette: a neutral "trainee". Accent shows the default ally blue in reviews.
SKIN = (0.86, 0.64, 0.50)
TUNIC = (0.33, 0.37, 0.46)
DARK = (0.19, 0.19, 0.23)
LEATHER = (0.38, 0.25, 0.16)
HAIR = (0.24, 0.16, 0.11)
METAL = (0.72, 0.74, 0.78)
GEM = (1.00, 0.82, 0.30)
ACCENT = (0.18, 0.52, 0.95)
EYES = (0.08, 0.08, 0.10)


def _lerp(a, b, t):
    return tuple(a[i] + (b[i] - a[i]) * t for i in range(3))


def _bone_point(name, t):
    for b in rig.bone_table():
        if b[0] == name:
            return _lerp(b[2], b[3], t)
    raise KeyError(name)


# (bone, start, end, sides, profile [(t, radius across, radius front-back)], slot, color)
# Points may be coordinates or (bone, t) pairs along a bone. Left-side parts are mirrored.
def _center_parts():
    return [
        ("pelvis", (0, 0, 0.90), (0, 0, 1.13), 14, [(0, .13, .10), (.35, .165, .115), (.75, .17, .12), (1, .16, .115)], "cloth", DARK),
        ("pelvis", (0, 0, 1.055), (0, 0, 1.13), 14, [(0, .178, .128), (1, .178, .128)], "cloth", LEATHER),
        ("pelvis", (0, -0.125, 1.07), (0, -0.155, 1.11), 4, [(0, .035, .02), (1, .035, .02)], "metal", METAL),
        ("spine_01", (0, 0, 1.11), (0, 0, 1.28), 14, [(0, .155, .11), (.5, .16, .115), (1, .17, .12)], "cloth", TUNIC),
        ("spine_02", (0, 0, 1.26), (0, 0, 1.42), 14, [(0, .17, .12), (.5, .185, .126), (1, .195, .13)], "cloth", TUNIC),
        ("chest", (0, 0, 1.40), (0, 0, 1.61), 14, [(0, .195, .13), (.3, .208, .138), (.6, .205, .135), (.85, .17, .115), (1, .10, .08)], "cloth", TUNIC),
        ("chest", (0, -0.12, 1.47), (0, -0.165, 1.47), 4, [(0, .03, .03), (.5, .03, .03), (1, 0, 0)], "emissive", GEM),
        ("pelvis", (0, -0.112, 1.10), (0, -0.14, 0.70), 6, [(0, .10, .014), (.6, .11, .014), (1, .12, .014)], "accent", ACCENT),
        ("pelvis", (0, 0.112, 1.10), (0, 0.14, 0.72), 6, [(0, .10, .014), (.6, .11, .014), (1, .12, .014)], "accent", ACCENT),
        ("chest", (0, 0, 1.535), (0, 0, 1.585), 14, [(0, .15, .11), (1, .125, .095)], "metal", METAL),
        ("neck", (0, 0, 1.55), (0, -0.01, 1.67), 10, [(0, .065, .065), (1, .055, .055)], "skin", SKIN),
        ("head", (0, -0.01, 1.62), (0, -0.01, 1.91), 14, [(0, .06, .07), (.15, .095, .10), (.45, .108, .115), (.75, .104, .11), (.92, .075, .08), (1, 0, 0)], "skin", SKIN),
        ("head", (0, 0.012, 1.765), (0, 0.016, 1.935), 14, [(0, .116, .124), (.45, .108, .116), (.8, .07, .075), (1, 0, 0)], "cloth", HAIR),
        ("head", (0, 0, 1.795), (0, 0, 1.83), 14, [(0, .117, .124), (1, .113, .12)], "accent", ACCENT),
        ("head", (0, -0.105, 1.735), (0, -0.135, 1.72), 4, [(0, .016, .02), (1, 0, 0)], "skin", SKIN),
    ]


def _left_parts():
    ua, fa, th, ca = "upperarm_l", "forearm_l", "thigh_l", "calf_l"
    return [
        (ua, (0.17, 0.01, 1.46), (0.235, 0.01, 1.63), 10, [(0, .10, .10), (.35, .12, .115), (.7, .10, .10), (1, 0, 0)], "metal", METAL),
        (ua, (ua, 0), (ua, 1), 10, [(0, .072, .072), (.5, .066, .066), (1, .056, .056)], "cloth", TUNIC),
        (fa, (fa, 0), (fa, 1), 8, [(0, .055, .055), (1, .045, .045)], "skin", SKIN),
        (fa, (fa, .4), (fa, .98), 8, [(0, .062, .062), (1, .053, .053)], "accent", ACCENT),
        ("hand_l", ("hand_l", 0), ("hand_l", 1), 6, [(0, .042, .026), (1, .046, .023)], "cloth", DARK),
        ("fingers_l", ("fingers_l", 0), ("fingers_l", 1), 6, [(0, .046, .023), (1, .03, .016)], "cloth", DARK),
        (fa, (fa, .9), (fa, 1.04), 8, [(0, .058, .058), (1, .058, .058)], "cloth", DARK),
        ("thumb_l", ("thumb_l", 0), ("thumb_l", 1), 5, [(0, .019, .019), (1, .012, .012)], "cloth", DARK),
        (th, (0.10, 0, 1.02), (th, 1), 10, [(0, .095, .095), (.5, .086, .086), (1, .066, .066)], "cloth", DARK),
        (ca, (0.11, -0.045, 0.59), (0.115, -0.065, 0.46), 6, [(0, .05, .03), (.5, .062, .036), (1, .04, .025)], "metal", METAL),
        (ca, (ca, 0), (ca, .5), 10, [(0, .066, .066), (1, .06, .06)], "cloth", DARK),
        (ca, (ca, .42), (ca, 1), 10, [(0, .074, .074), (.15, .07, .07), (1, .056, .06)], "cloth", LEATHER),
        (ca, (ca, .40), (ca, .48), 10, [(0, .08, .08), (1, .08, .08)], "cloth", LEATHER),
        ("pelvis", (0.135, -0.02, 1.08), (0.19, -0.03, 0.86), 6, [(0, .055, .03), (.5, .07, .032), (1, .06, .025)], "metal", METAL),
        ("foot_l", ("foot_l", 0), ("foot_l", 1), 6, [(0, .058, .052), (1, .052, .036)], "cloth", LEATHER),
        ("toe_l", ("toe_l", 0), ("toe_l", 1), 6, [(0, .052, .032), (1, .036, .02)], "cloth", LEATHER),
    ]


def _eyes():
    # Two dark slits so the facing reads at the gameplay camera.
    return [
        ("head", (x, -0.103, 1.772), (x, -0.118, 1.772), 4, [(0, .016, .008), (1, .016, .008)], "cloth", EYES)
        for x in (0.038, -0.038)
    ]


def _mirror_part(p):
    bone, a, b, sides, prof, slot, color = p
    mb = bone[:-2] + "_r" if bone.endswith("_l") else bone

    def mpt(pt):
        if isinstance(pt[0], str):
            return (pt[0][:-2] + "_r", pt[1])
        return (-pt[0], pt[1], pt[2])

    return (mb, mpt(a), mpt(b), sides, prof, slot, color)


def parts():
    left = _left_parts()
    out = _center_parts() + _eyes() + left + [_mirror_part(p) for p in left]
    # Round parts get two more sides and smoothed in-between rings: rounder silhouettes at the
    # gameplay camera.
    return [(b, a, e, s + 2 if s >= 8 else s, _smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]


def _smooth(profile):
    """Insert a Catmull-Rom midpoint between rings of curved profiles (3+ rings, no apex)."""
    if len(profile) < 3:
        return profile
    out = []
    n = len(profile)
    for i in range(n - 1):
        out.append(profile[i])
        if profile[i + 1][1] == 0 or profile[i][1] == 0:
            continue
        p0, p1, p2, p3 = (profile[max(i - 1, 0)], profile[i], profile[i + 1], profile[min(i + 2, n - 1)])
        mid = [(-p0[k] + 9 * p1[k] + 9 * p2[k] - p3[k]) / 16 for k in range(3)]
        out.append(tuple(mid))
    out.append(profile[-1])
    return out


def _resolve(pt):
    return Vector(_bone_point(*pt) if isinstance(pt[0], str) else pt)


def _loft(bm, a, b, sides, profile, mat, color, group, layers):
    deform, col = layers
    axis = (b - a).normalized()
    # Cross-section frame: `u` across the body (character X), `v` front-back.
    ref = Vector((1, 0, 0)) if abs(axis.x) < 0.9 else Vector((0, 0, 1))
    u = (ref - axis * ref.dot(axis)).normalized()
    v = axis.cross(u)
    rings = []
    for t, ru, rv in profile:
        c = a.lerp(b, t)
        if ru == 0 and rv == 0:
            rings.append([bm.verts.new(c)])
            continue
        ring = []
        for i in range(sides):
            ang = 2 * math.pi * (i + 0.5) / sides
            ring.append(bm.verts.new(c + u * (ru * math.cos(ang)) + v * (rv * math.sin(ang))))
        rings.append(ring)
    faces = []
    for r0, r1 in zip(rings, rings[1:]):
        if len(r1) == 1:
            faces += [bm.faces.new((r0[i], r0[(i + 1) % sides], r1[0])) for i in range(sides)]
        elif len(r0) == 1:
            faces += [bm.faces.new((r1[(i + 1) % sides], r1[i], r0[0])) for i in range(sides)]
        else:
            faces += [bm.faces.new((r0[i], r0[(i + 1) % sides], r1[(i + 1) % sides], r1[i])) for i in range(sides)]
    if len(rings[0]) > 1:
        faces.append(bm.faces.new(list(reversed(rings[0]))))
    if len(rings[-1]) > 1:
        faces.append(bm.faces.new(rings[-1]))
    for f in faces:
        f.material_index = mat
        f.smooth = False
    for ring in rings:
        for vert in ring:
            vert[deform][group] = 1.0
            # Height-based occlusion in alpha: darker toward the ground (05 §2 cavity/AO).
            ao = max(0.55, min(1.0, 0.55 + 0.45 * vert.co.z / 1.9))
            vert[col] = (*color, ao)


def build_mannequin(armature, name="biped_v1_mannequin", collection=None):
    """Create the template mesh, skinned to `armature`, and return it."""
    me = bpy.data.meshes.new(name)
    obj = bpy.data.objects.new(name, me)
    (collection or bpy.context.scene.collection).objects.link(obj)

    for slot in MATERIAL_SLOTS:
        me.materials.append(material(slot))
    groups = {}
    for bname in rig.DEFORM_BONES:
        groups[bname] = obj.vertex_groups.new(name=bname).index

    bm = bmesh.new()
    layers = (bm.verts.layers.deform.verify(), bm.verts.layers.float_color.new("Col"))
    for bone, a, b, sides, prof, slot, color in parts():
        _loft(bm, _resolve(a), _resolve(b), sides, prof, MATERIAL_SLOTS.index(slot), color, groups[bone], layers)
    bm.to_mesh(me)
    bm.free()
    me.color_attributes.active_color = me.color_attributes["Col"]
    me.color_attributes.render_color_index = me.color_attributes.active_color_index

    obj.parent = armature
    mod = obj.modifiers.new("Armature", "ARMATURE")
    mod.object = armature
    return obj


def material(slot):
    """The per-slot preview material: vertex color into a principled BSDF (exports as COLOR_0)."""
    mat = bpy.data.materials.get(slot)
    if mat:
        return mat
    mat = bpy.data.materials.new(slot)
    mat.use_nodes = True
    mat.use_backface_culling = True
    nt = mat.node_tree
    bsdf = next(n for n in nt.nodes if n.type == "BSDF_PRINCIPLED")
    attr = nt.nodes.new("ShaderNodeVertexColor")
    attr.layer_name = "Col"
    nt.links.new(attr.outputs["Color"], bsdf.inputs["Base Color"])
    bsdf.inputs["Roughness"].default_value = 0.8
    if slot == "emissive":
        nt.links.new(attr.outputs["Color"], bsdf.inputs["Emission Color"])
        bsdf.inputs["Emission Strength"].default_value = 1.0
    return mat


def triangle_count(obj):
    return sum(len(p.vertices) - 2 for p in obj.data.polygons)
