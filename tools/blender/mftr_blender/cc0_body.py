# SPDX-License-Identifier: AGPL-3.0-or-later
"""The humanoid body from an artist-made mesh (22 §1, BW3): Blender Studio's realistic male and
female base meshes (CC0, `art/creator/bodies/README.md`), reduced, fitted to the biped rig and
skinned to it, in place of the lofted tubes.

How a body is made:

1. **Source** (`load`): the reduced body (`body_source.blend`, made by
   `tools/blender/extract_bodies.py`), its head cut off at the neck: the authored head
   (`authored_head`) sits on it as it did on the lofted neck.
2. **Joints** (`joints`): where the source's joints are, from the bundle's primitive bodies
   (`PRIMITIVE`). The male primitive's arms hang at another angle from the realistic body's, so
   each arm's chain is turned about the shoulder onto the mesh's own arm (a line fit through
   its vertices).
3. **Skinning on the source** (`source_rig`): a skeleton with the biped's bone names on those
   joints, and Blender's automatic weights from it.
4. **Fit** (`fit`): every vertex carried by its weights from each source bone onto the rig's
   bone of the same name: turned onto it, stretched to its length, and thickened by the
   build's girth. The weights come along unchanged: they are already the rig's.
"""

import math
import os

import bpy
from mathutils import Matrix, Vector

HERE = os.path.dirname(os.path.abspath(__file__))
SOURCE = os.path.normpath(os.path.join(HERE, "..", "..", "..", "art", "creator", "bodies", "body_source.blend"))

# The joints of the bundle's primitive bodies (their parts' origins), printed by
# `extract_bodies.py`: metres, the body's middle at x 0, facing -Y, feet at z 0. Left side only.
PRIMITIVE = {
    "male": {
        "pelvis": (0.0000, -0.0175, 0.9598), "chest": (0.0000, -0.0175, 1.1372),
        "neck": (0.0000, -0.0095, 1.4114), "head": (0.0000, -0.0413, 1.5102),
        "clavicle_l": (0.1566, 0.0073, 1.3435), "shoulder_l": (0.2150, 0.0096, 1.2957),
        "elbow_l": (0.3411, 0.0105, 1.1427), "wrist_l": (0.4689, -0.0587, 0.9561),
        "knuckles_l": (0.5363, -0.1054, 0.8749), "thumb_l": (0.4812, -0.1242, 0.9133),
        "hip_l": (0.0880, -0.0164, 0.8644), "knee_l": (0.1323, 0.0061, 0.4771),
        "ankle_l": (0.1702, 0.0524, 0.1125), "toes_l": (0.2414, -0.0712, 0.0192),
    },
    "female": {
        "pelvis": (0.0000, -0.0466, 0.9609), "chest": (0.0000, -0.0551, 1.1059),
        "neck": (0.0000, -0.0247, 1.4002), "head": (0.0000, -0.0404, 1.4524),
        "clavicle_l": (0.1171, 0.0101, 1.3076), "shoulder_l": (0.1657, 0.0099, 1.2557),
        "elbow_l": (0.2533, 0.0357, 1.0612), "wrist_l": (0.3651, -0.0162, 0.8605),
        "knuckles_l": (0.4211, -0.0633, 0.7770), "thumb_l": (0.3690, -0.0814, 0.8214),
        "hip_l": (0.0926, -0.0162, 0.8666), "knee_l": (0.1085, -0.0011, 0.4623),
        "ankle_l": (0.1222, 0.0389, 0.0896), "toes_l": (0.1627, -0.0944, 0.0144),
    },
}
ARM = ("elbow_l", "wrist_l", "knuckles_l", "thumb_l")


def load(sex):
    """A new object with the source body of `sex` ("male" or "female"), whole."""
    name = f"body_{sex}"
    with bpy.data.libraries.load(SOURCE, link=False) as (_src, dst):
        dst.objects = [name]
    obj = dst.objects[0]
    bpy.context.scene.collection.objects.link(obj)
    return obj


def _arm_axis(cos, shoulder, wrist):
    """The mesh's own arm: a line fit (the principal axis) through the vertices between the
    shoulder and about where the wrist is, out past the torso's side."""
    reach = (wrist - shoulder).length
    pts = [c for c in cos if c.x > shoulder.x + 0.02 and c.z < shoulder.z - 0.03 and c.z > shoulder.z - reach * 0.95]
    if len(pts) < 10:
        return None
    mid = sum(pts, Vector()) / len(pts)
    # Power iteration on the covariance for its main axis.
    cov = Matrix(((0.0,) * 3,) * 3)
    for p in pts:
        d = p - mid
        for i in range(3):
            for j in range(3):
                cov[i][j] += d[i] * d[j]
    axis = Vector((0.4, 0.0, -1.0)).normalized()
    for _ in range(40):
        axis = (cov @ axis).normalized()
    if axis.z > 0:
        axis = -axis
    return mid, axis


def joints(obj, sex):
    """The source body's joints (both sides), its arms turned onto the mesh's."""
    j = {k: Vector(v) for k, v in PRIMITIVE[sex].items()}
    cos = [v.co.copy() for v in obj.data.vertices]
    fit = _arm_axis(cos, j["shoulder_l"], j["wrist_l"])
    if fit is not None:
        mid, axis = fit
        # The arm's line through the shoulder's height: where the mesh's arm leaves it.
        start = mid + axis * ((j["shoulder_l"].z - mid.z) / axis.z)
        prim = (j["wrist_l"] - j["shoulder_l"]).normalized()
        turn = prim.rotation_difference(axis).to_matrix()
        pivot = j["shoulder_l"]
        for k in ARM:
            j[k] = pivot + turn @ (j[k] - pivot)
        j["arm_line"] = start
    out = {}
    for k, v in j.items():
        if k.endswith("_l"):
            out[k] = v
            out[k[:-2] + "_r"] = Vector((-v.x, v.y, v.z))
        else:
            out[k] = v
    return out


# The biped's deforming bones the body is skinned to, and where each runs on the source: from
# one joint to another (a name in `joints`), or (for the spine) a share of the way from the
# hips' middle to the neck, as the rig's own spine shares it.
SPINE = {"pelvis": (0.0, 0.214), "spine_01": (0.214, 0.464), "spine_02": (0.464, 0.714), "chest": (0.714, 1.0)}
LIMBS = {
    "clavicle": ("clavicle_base", "shoulder"), "upperarm": ("shoulder", "elbow"), "forearm": ("elbow", "wrist"),
    "hand": ("wrist", "knuckles"), "fingers": ("knuckles", "fingertips"), "thumb": ("thumb", "thumb_tip"),
    "thigh": ("hip", "knee"), "calf": ("knee", "ankle"), "foot": ("ankle", "toes"), "toe": ("toes", "toe_tip"),
}


def _source_bones(j):
    """{bone: (head, tail)} on the source, both sides."""
    hips = (j["hip_l"] + j["hip_r"]) / 2
    neck = j["neck"]
    out = {name: (hips.lerp(neck, a), hips.lerp(neck, b)) for name, (a, b) in SPINE.items()}
    out["neck"] = (neck, j["head"])
    # The head's bone, though its mesh is gone: the top of the neck turns with the head.
    out["head"] = (j["head"], j["head"] + Vector((0.0, 0.0, 0.2)))
    for side in ("l", "r"):
        s = 1 if side == "l" else -1
        q = {k[:-2]: v for k, v in j.items() if k.endswith("_" + side)}
        q["clavicle_base"] = Vector((0.02 * s, q["clavicle"].y, q["clavicle"].z))
        q["fingertips"] = q["knuckles"] + (q["knuckles"] - q["wrist"]) * 0.8
        q["thumb_tip"] = q["thumb"] + (q["thumb"] - q["wrist"]).normalized() * 0.04
        q["toe_tip"] = q["toes"] + Vector((0.0, -0.06, 0.0))
        for bone, (a, b) in LIMBS.items():
            out[f"{bone}_{side}"] = (q[a], q[b])
    return out


def source_rig(obj, j):
    """A skeleton on the source's joints with the biped's bone names, and `obj` skinned to it by
    Blender's automatic weights. Returns the skeleton."""
    bones = _source_bones(j)
    data = bpy.data.armatures.new("cc0_source")
    arm = bpy.data.objects.new("cc0_source", data)
    bpy.context.scene.collection.objects.link(arm)
    bpy.context.view_layer.objects.active = arm
    bpy.ops.object.mode_set(mode="EDIT")
    parents = {"spine_01": "pelvis", "spine_02": "spine_01", "chest": "spine_02", "neck": "chest", "head": "neck"}
    for side in ("l", "r"):
        parents.update({f"clavicle_{side}": "chest", f"upperarm_{side}": f"clavicle_{side}",
                        f"forearm_{side}": f"upperarm_{side}", f"hand_{side}": f"forearm_{side}",
                        f"fingers_{side}": f"hand_{side}", f"thumb_{side}": f"hand_{side}",
                        f"thigh_{side}": "pelvis", f"calf_{side}": f"thigh_{side}", f"foot_{side}": f"calf_{side}",
                        f"toe_{side}": f"foot_{side}"})
    for name, (h, t) in bones.items():
        eb = data.edit_bones.new(name)
        eb.head, eb.tail = h, t
    for name, p in parents.items():
        data.edit_bones[name].parent = data.edit_bones[p]
    bpy.ops.object.mode_set(mode="OBJECT")
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    obj.select_set(True)
    arm.select_set(True)
    bpy.context.view_layer.objects.active = arm
    bpy.ops.object.parent_set(type="ARMATURE_AUTO")
    obj.parent = None
    for m in list(obj.modifiers):
        obj.modifiers.remove(m)
    return arm


def fit(obj, src, target, girth=None):
    """Carry `obj` (skinned to the source skeleton `src`) onto the armature `target`, bone by
    bone: each vertex goes by its weights through every bone's map, which turns the source bone
    onto the target's of the same name, stretches it to its length and thickens it (`girth(bone)`
    gives (across, front-back) on top of the body's own scale). The torso widens by height,
    from the hips' ratio to the shoulders'."""
    sb, tb = src.data.bones, target.data.bones
    height = tb["neck"].head_local.z / sb["neck"].head_local.z
    hip_ratio = tb["thigh_l"].head_local.x / sb["thigh_l"].head_local.x
    shoulder_ratio = tb["upperarm_l"].head_local.x / sb["upperarm_l"].head_local.x
    z0, z1 = sb["pelvis"].head_local.z, sb["upperarm_l"].head_local.z

    def across(z):
        t = max(0.0, min(1.0, (z - z0) / max(1e-6, z1 - z0)))
        return hip_ratio + (shoulder_ratio - hip_ratio) * t

    maps = {}
    for b in sb:
        if b.name not in tb:
            continue
        hs, ts = b.head_local, b.tail_local
        ht, tt = tb[b.name].head_local, tb[b.name].tail_local
        ds, dt = ts - hs, tt - ht
        turn = ds.normalized().rotation_difference(dt.normalized()).to_matrix()
        axial = dt.length / max(1e-6, ds.length)
        gx, gy = girth(b.name) if girth else (1.0, 1.0)
        base = b.name.rsplit("_", 1)[0] if b.name[-2:] in ("_l", "_r") else b.name
        if base in SPINE or base in ("neck", "head"):
            # Upright: across and front-back in the world's axes.
            mid = (hs.z + ts.z) / 2
            scale = Matrix.Diagonal((across(mid) * gx, height * gy, axial))
            maps[b.name] = (hs, ht, Matrix.Identity(3), scale, ds.normalized())
        else:
            radial = max(height, axial ** 0.5) * (gx + gy) / 2 if base not in ("hand", "fingers", "thumb") else axial
            maps[b.name] = (hs, ht, turn, (radial, axial), ds.normalized())
    names = {g.index: g.name for g in obj.vertex_groups}
    for v in obj.data.vertices:
        acc, tot = Vector(), 0.0
        for g in v.groups:
            m = maps.get(names[g.group])
            if m is None or g.weight <= 0:
                continue
            hs, ht, turn, scale, axis = m
            d = v.co - hs
            if isinstance(scale, Matrix):
                q = ht + scale @ d
            else:
                radial, axial = scale
                along = axis * d.dot(axis)
                q = ht + turn @ (along * axial + (d - along) * radial)
            acc += q * g.weight
            tot += g.weight
        if tot > 0:
            v.co = acc / tot


# Underwear (22 §1.5): regions of the body itself on the cloth slot, so they add no triangles
# and never clip. Briefs on every base, a band top on the feminine ones; each a band and its
# trim, so the paint step has two regions (and recipes that painted the old undersuit's two
# still find them).
UNDER = (0.16, 0.16, 0.18)
TRIM = (0.30, 0.20, 0.12)


def _briefs(c, j):
    """Whether a face centred at `c` (source) is the briefs': from the waistband down to leg
    openings that rise from the crotch toward the hips' sides."""
    hip = j["hip_l"]
    waist = hip.z + 0.085
    crotch = hip.z - 0.075
    rise = crotch + 0.95 * max(0.0, abs(c.x) - 0.025)
    return rise < c.z < waist and abs(c.x) < hip.x + 0.11


def _band(c, j, sex):
    """Whether a face centred at `c` (source) is the band top's (feminine): round the chest from
    under the bust to above it, the arms excluded."""
    if sex != "female":
        return False
    sh = j["shoulder_l"]
    lo, hi = j["chest"].z + 0.015, j["chest"].z + 0.135
    return lo < c.z < hi and abs(c.x) < sh.x - 0.035


def make(armature, gender, build, girth=None):
    """The fitted body for `gender` on `armature` (shaped for `build`): the source body with its
    head cut away below the chin, skinned and fitted to the rig, its faces on the skin slot but
    the underwear's, colored. Returns the object (vertex groups named for the rig's bones)."""
    from . import mesh as mesh_mod
    import bmesh
    sex = "male" if gender == "masculine" else "female"
    obj = load(sex)
    j = joints(obj, sex)
    me = obj.data
    me.materials.clear()
    for slot in mesh_mod.MATERIAL_SLOTS:
        me.materials.append(mesh_mod.material(slot))
    skin_i, cloth_i = mesh_mod.MATERIAL_SLOTS.index("skin"), mesh_mod.MATERIAL_SLOTS.index("cloth")
    bm = bmesh.new()
    bm.from_mesh(me)
    col = bm.verts.layers.float_color.new("Col")
    # The head goes, cut flat a little above the neck's joint (below the jaw); `raise_neck`
    # carries the neck's top up into the authored head once they are one mesh.
    cut = j["neck"].z + 0.025

    def above(co):
        return co.z > cut

    bmesh.ops.delete(bm, geom=[v for v in bm.verts if above(v.co)], context="VERTS")
    _keep_largest(bm)
    from .creator import SKIN
    for v in bm.verts:
        v[col] = (*SKIN, 1.0)
    clothed = []
    for f in bm.faces:
        c = f.calc_center_median()
        f.smooth = False
        under = _briefs(c, j) or _band(c, j, sex)
        f.material_index = cloth_i if under else skin_i
        if under:
            clothed.append(f)
    # Each garment's edge (its trim) where its faces meet skin; the vertices it shares with the
    # skin are split off, so the colors stay crisp.
    edge = {e for f in clothed for e in f.edges if any(o.material_index == skin_i for o in e.link_faces)}
    bmesh.ops.split_edges(bm, edges=list(edge))
    trim = {v for e in edge for v in e.verts}
    for f in clothed:
        for v in f.verts:
            v[col] = (*(TRIM if v in trim else UNDER), 1.0)
    bm.to_mesh(me)
    bm.free()
    me.color_attributes.active_color = me.color_attributes["Col"]
    src = source_rig(obj, j)
    _weld_weights(obj)
    _limit(obj, 2)
    fit(obj, src, armature, girth)
    bpy.data.objects.remove(src)
    _ground(obj, armature)
    return obj


def _keep_largest(bm):
    """Only the body's main piece: bits the head's cut left loose go."""
    seen, pieces = set(), []
    for v in bm.verts:
        if v in seen:
            continue
        piece, stack = [], [v]
        seen.add(v)
        while stack:
            u = stack.pop()
            piece.append(u)
            for e in u.link_edges:
                w = e.other_vert(u)
                if w not in seen:
                    seen.add(w)
                    stack.append(w)
        pieces.append(piece)
    pieces.sort(key=len, reverse=True)
    import bmesh
    bmesh.ops.delete(bm, geom=[v for p in pieces[1:] for v in p], context="VERTS")


def _weld_weights(obj):
    """Vertices at one place (the underwear's split seams) take one set of weights, their mean:
    the automatic weights differ across a split, and the fit would open it into a crack."""
    me = obj.data
    names = {g.index: g for g in obj.vertex_groups}
    places = {}
    for v in me.vertices:
        places.setdefault(tuple(round(c, 5) for c in v.co), []).append(v)
    for twins in places.values():
        if len(twins) < 2:
            continue
        acc = {}
        for v in twins:
            for g in v.groups:
                acc[g.group] = acc.get(g.group, 0.0) + g.weight / len(twins)
        for v in twins:
            for gi, w in acc.items():
                names[gi].add([v.index], w, "REPLACE")


def _limit(obj, n):
    """At most `n` bone influences a vertex (the biped's budget, 10 §2), normalized."""
    for o in bpy.context.view_layer.objects:
        o.select_set(False)
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.vertex_group_limit_total(group_select_mode="ALL", limit=n)
    bpy.ops.object.vertex_group_normalize_all(group_select_mode="ALL", lock_active=False)


def _ground(obj, armature):
    """The soles on the floor: below each ankle, the foot is squashed so its lowest point is at
    z 0 (the fit carries the source's sole down with its taller ankle)."""
    for side in ("l", "r"):
        ankle = armature.data.bones[f"foot_{side}"].head_local.z
        s = 1 if side == "l" else -1
        feet = [v for v in obj.data.vertices if v.co.x * s > 0 and v.co.z < ankle]
        low = min((v.co.z for v in feet), default=0.0)
        if low >= 0.0 or ankle <= 0.0:
            continue
        k = ankle / (ankle - low)
        for v in feet:
            v.co.z = ankle - (ankle - v.co.z) * k



# How much the raised neck narrows toward its axis by the top ring.
TAPER = 0.18


def raise_neck(obj, n_body, cap, rise=0.012):
    """Carry the neck's top (the body's highest open loop: vertices below `n_body`) up into the
    authored head (the rest, closed underneath): two new rings over it, the top one where a line
    straight up from each vertex meets the head's underside, `rise` higher (inside the head, so
    the face's sculpting keeps the joint covered), the middle one halfway. The top ring is the
    head's, the middle one mostly the neck's: the neck turns as the head does. No higher than `cap`
    (the head's joint) and `rise`: a line past the chin would find the lips."""
    import bmesh
    from mathutils.bvhtree import BVHTree
    me = obj.data
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.verts.ensure_lookup_table()
    deform = bm.verts.layers.deform.verify()
    col = bm.verts.layers.float_color.get("Col")
    places = {}
    for v in bm.verts:
        k = tuple(round(c, 5) for c in v.co)
        places[k] = places.get(k, 0) + 1
    # The neck's top: the body's open edges with no twin (color seams are split, so open too),
    # the highest piece of them.
    edges = [e for e in bm.edges if e.is_boundary and all(v.index < n_body for v in e.verts)
             and not all(places[tuple(round(c, 5) for c in v.co)] > 1 for v in e.verts)]
    # The piece of them that holds the highest vertex, followed along its edges.
    start = max((v for e in edges for v in e.verts), key=lambda v: v.co.z)
    by_vert = {}
    for e in edges:
        for v in e.verts:
            by_vert.setdefault(v, []).append(e)
    loop, stack = {start}, [start]
    while stack:
        u = stack.pop()
        for e in by_vert.get(u, []):
            w = e.other_vert(u)
            if w not in loop:
                loop.add(w)
                stack.append(w)
    axis = sum((v.co for v in loop), Vector()) / len(loop)
    ring = sorted(loop, key=lambda v: math.atan2(v.co.y - axis.y, v.co.x - axis.x))
    head_faces = [f for f in bm.faces if all(v.index >= n_body for v in f.verts)]
    tree = BVHTree.FromPolygons([v.co.copy() for v in bm.verts], [[v.index for v in f.verts] for f in head_faces])
    groups = {g.name: g.index for g in obj.vertex_groups}
    g_head, g_neck = groups["head"], groups["neck"]
    skin = next((f.material_index for f in bm.faces if f.verts[0] in loop), 0)
    mids, tops = [], []
    for v in ring:
        # Narrowing into the head (the cut's back edge lies out on the trapezius).
        inner = Vector((axis.x + (v.co.x - axis.x) * (1 - TAPER), axis.y + (v.co.y - axis.y) * (1 - TAPER), v.co.z))
        hit, _n, _i, _d = tree.ray_cast(inner + Vector((0, 0, 0.001)), Vector((0.0, 0.0, 1.0)), 0.25)
        if hit is None:
            hit, _n, _i, _d = tree.find_nearest(inner)
        top = Vector((inner.x, inner.y, max(min(hit.z, cap), v.co.z) + rise))
        mid = v.co.lerp(top, 0.5)
        mid.x, mid.y = v.co.x + (inner.x - v.co.x) * 0.45, v.co.y + (inner.y - v.co.y) * 0.45
        for p, wh in ((mid, 0.4), (top, 1.0)):
            nv = bm.verts.new(p)
            nv[deform][g_head] = wh
            if wh < 1.0:
                nv[deform][g_neck] = 1.0 - wh
            if col is not None:
                nv[col] = v[col]
            (mids if wh < 1.0 else tops).append(nv)
    made = 0
    for a, b in ((ring, mids), (mids, tops)):
        for i in range(len(ring)):
            k = (i + 1) % len(ring)
            quad = [a[i], a[k], b[k], b[i]]
            if len({id(x) for x in quad}) < 4:
                continue
            try:
                f = bm.faces.new(quad)
            except ValueError:
                continue
            f.normal_update()
            c = f.calc_center_median()
            if f.normal.dot(Vector((c.x - axis.x, c.y - axis.y, 0.0))) < 0:
                f.normal_flip()
            f.material_index = skin
            f.smooth = False
            made += 1
    bm.to_mesh(me)
    bm.free()
    return made
