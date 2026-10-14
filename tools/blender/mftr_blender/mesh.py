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

from . import head, rig

# The fixed material slots (10 §2). `accent` is tinted ally/enemy at runtime.
MATERIAL_SLOTS = ("skin", "cloth", "metal", "emissive", "accent", "accent_glow")
# Baked occlusion runs from the ground to this height (the figure's height).
AO_HEIGHT = 1.9

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


def _head(bm, layers, groups, mats):
    # The sculpted head with short hair (head.py): a face, not an egg.
    head.build(bm, layers, groups, mats, head.Head(base=(0, -0.01, 1.63), height=0.255))


def _mirror_part(p):
    bone, a, b, sides, prof, slot, color = p
    mb = bone[:-2] + "_r" if bone.endswith("_l") else bone

    return (mb, _mirror_point(a), _mirror_point(b), sides, prof, slot, color)


def parts():
    left = _left_parts()
    out = _center_parts() + left + [_mirror_part(p) for p in left]
    # Round parts get two more sides and smoothed in-between rings: rounder silhouettes at the
    # gameplay camera.
    out = [(b, a, e, s + 2 if s >= 8 else s, _smooth(prof) if s >= 8 else prof, slot, col) for b, a, e, s, prof, slot, col in out]
    return out + [_head]


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
    """A coordinate, a (bone, t) point along a bone, or (bone, t, (dx, dy, dz)): that point
    moved by an offset in armature axes."""
    if isinstance(pt[0], str):
        base = Vector(_bone_point(pt[0], pt[1]))
        return base + Vector(pt[2]) if len(pt) > 2 else base
    return Vector(pt)


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
            ao = max(0.55, min(1.0, 0.55 + 0.45 * vert.co.z / AO_HEIGHT))
            vert[col] = (*color, ao)


def _loft_path(bm, centers, radii, sides, mat, color, group, layers, roll=0.0, ref=None, colors=None):
    """A loft through `centers` (a polyline), one ring per point with (ru, rv) radii; (0, 0) is an
    apex. The ring's `u` axis starts along `ref` (default: character X, or Z for paths lying
    along X), turned by `roll` degrees about the path, and is carried along it (parallel
    transport), so bent shapes (horns, plumes, curved blades) don't twist. `colors` optionally
    gives one color per ring."""
    deform, col = layers
    n = len(centers)
    tangents = []
    for i in range(n):
        a, b = centers[max(i - 1, 0)], centers[min(i + 1, n - 1)]
        tangents.append((b - a).normalized())
    if ref is None:
        ref = Vector((1, 0, 0)) if abs(tangents[0].x) < 0.9 else Vector((0, 0, 1))
    u = (Vector(ref) - tangents[0] * Vector(ref).dot(tangents[0])).normalized()
    if roll:
        from mathutils import Matrix
        u = Matrix.Rotation(math.radians(roll), 3, tangents[0]) @ u
    frames = []
    for i in range(n):
        if i:
            # Parallel transport: rotate u by the turn between consecutive tangents.
            q = tangents[i - 1].rotation_difference(tangents[i])
            u = (q @ u)
            u = (u - tangents[i] * u.dot(tangents[i])).normalized()
        frames.append((u.copy(), tangents[i].cross(u)))
    rings = []
    for i, (c, (ru, rv)) in enumerate(zip(centers, radii)):
        uu, vv = frames[i]
        if ru == 0 and rv == 0:
            rings.append([bm.verts.new(c)])
            continue
        ring = []
        for k in range(sides):
            ang = 2 * math.pi * (k + 0.5) / sides
            ring.append(bm.verts.new(c + uu * (ru * math.cos(ang)) + vv * (rv * math.sin(ang))))
        rings.append(ring)
    faces = []
    for r0, r1 in zip(rings, rings[1:]):
        if len(r1) == 1 and len(r0) == 1:
            continue
        if len(r1) == 1:
            faces += [bm.faces.new((r0[k], r0[(k + 1) % sides], r1[0])) for k in range(sides)]
        elif len(r0) == 1:
            faces += [bm.faces.new((r1[(k + 1) % sides], r1[k], r0[0])) for k in range(sides)]
        else:
            faces += [bm.faces.new((r0[k], r0[(k + 1) % sides], r1[(k + 1) % sides], r1[k])) for k in range(sides)]
    if len(rings[0]) > 1:
        faces.append(bm.faces.new(list(reversed(rings[0]))))
    if len(rings[-1]) > 1:
        faces.append(bm.faces.new(rings[-1]))
    for f in faces:
        f.material_index = mat
        f.smooth = False
    for i, ring in enumerate(rings):
        c = colors[i] if colors else color
        for vert in ring:
            vert[deform][group] = 1.0
            ao = max(0.55, min(1.0, 0.55 + 0.45 * vert.co.z / AO_HEIGHT))
            vert[col] = (*c, ao)


def path_part(bone, points, sides, radii, slot, color, roll=0.0, ref=None, colors=None):
    """A part lofted along a polyline (14 §2.3): `points` are coordinates or (bone, t) pairs,
    `radii` one (ru, rv) per point. Returns a builder for `build_mannequin`'s part list."""
    def build(bm, layers, groups, mats):
        _loft_path(bm, [_resolve(p) for p in points], radii, sides, mats[slot], color, groups[bone], layers,
                   roll, ref, colors)
    build.mftr_path = (bone, points, sides, radii, slot, color, roll, ref, colors)
    return build


def _mirror_point(p):
    if isinstance(p[0], str):
        if len(p) > 2:
            return (_swap_side(p[0]), p[1], (-p[2][0], p[2][1], p[2][2]))
        return (_swap_side(p[0]), p[1])
    return (-p[0], p[1], p[2])


def _swap_side(name):
    if name.endswith("_l"):
        return name[:-2] + "_r"
    if name.endswith("_r"):
        return name[:-2] + "_l"
    return name


def path_pair(bone, points, sides, radii, slot, color, roll=0.0, ref=None, colors=None):
    """`path_part` for the left side and its mirror for the right."""
    mref = None if ref is None else (-ref[0], ref[1], ref[2])
    return [path_part(bone, points, sides, radii, slot, color, roll, ref, colors),
            path_part(_swap_side(bone), [_mirror_point(p) for p in points], sides, radii, slot, color, -roll, mref, colors)]


def box_part(bone, center, size, slot, color, rot=(0.0, 0.0, 0.0), taper=1.0, bevel=0.0):
    """A block (14 §2.3): `size` (x, y, z) about `center` (a coordinate or (bone, t)), turned by
    `rot` degrees (XYZ, armature axes); `taper` scales the top face; `bevel` cuts the vertical
    edges (0–0.45 of the smaller side), an octagonal prism instead of a box."""
    def build(bm, layers, groups, mats):
        from mathutils import Euler
        deform, col = layers
        c = _resolve(center)
        sx, sy, sz = size[0] / 2, size[1] / 2, size[2] / 2
        b = bevel * min(sx, sy) * 2
        if b > 0:
            ring = [(sx - b, -sy), (sx, -sy + b), (sx, sy - b), (sx - b, sy), (-sx + b, sy), (-sx, sy - b), (-sx, -sy + b), (-sx + b, -sy)]
        else:
            ring = [(sx, -sy), (sx, sy), (-sx, sy), (-sx, -sy)]
        m = Euler(tuple(math.radians(a) for a in rot), "XYZ").to_matrix()
        lo = [bm.verts.new(c + m @ Vector((x, y, -sz))) for x, y in ring]
        hi = [bm.verts.new(c + m @ Vector((x * taper, y * taper, sz))) for x, y in ring]
        k = len(ring)
        faces = [bm.faces.new((lo[i], lo[(i + 1) % k], hi[(i + 1) % k], hi[i])) for i in range(k)]
        faces.append(bm.faces.new(list(reversed(lo))))
        faces.append(bm.faces.new(hi))
        for f in faces:
            f.material_index = mats[slot]
            f.smooth = False
        for v in lo + hi:
            v[deform][groups[bone]] = 1.0
            ao = max(0.55, min(1.0, 0.55 + 0.45 * v.co.z / AO_HEIGHT))
            v[col] = (*color, ao)
    build.mftr_box = (bone, center, size, slot, color, rot, taper, bevel)
    return build


def box_pair(bone, center, size, slot, color, rot=(0.0, 0.0, 0.0), taper=1.0, bevel=0.0):
    """`box_part` and its mirror (the rotation mirrored about X)."""
    return [box_part(bone, center, size, slot, color, rot, taper, bevel),
            box_part(_swap_side(bone), _mirror_point(center), size, slot, color, (rot[0], -rot[1], -rot[2]), taper, bevel)]


def build_mannequin(armature, name="biped_v1_mannequin", collection=None, part_list=None):
    """Create a lofted mesh from `part_list` (default: the template's), skinned to `armature`
    (one vertex group per deforming bone, extras included), and return it."""
    me = bpy.data.meshes.new(name)
    obj = bpy.data.objects.new(name, me)
    (collection or bpy.context.scene.collection).objects.link(obj)

    for slot in MATERIAL_SLOTS:
        me.materials.append(material(slot))
    groups = {}
    for b in armature.data.bones:
        if b.use_deform:
            groups[b.name] = obj.vertex_groups.new(name=b.name).index

    bm = bmesh.new()
    layers = (bm.verts.layers.deform.verify(), bm.verts.layers.float_color.new("Col"))
    mats = {slot: i for i, slot in enumerate(MATERIAL_SLOTS)}
    for part in (part_list if part_list is not None else parts()):
        if callable(part):
            part(bm, layers, groups, mats)  # a custom builder (heads)
            continue
        bone, a, b, sides, prof, slot, color = part
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
