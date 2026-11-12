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
    """{bone: (head, tail)} on the source, both sides. The spine runs from the hips to
    `spine_top` (where the rig's neck sits against its shoulders, `make` sets it), not to the
    source's own neck joint: the rig's shoulders are 3 cm under its neck, the source's 12–15, and
    mapping the source's neck onto the rig's lifted the shoulders into a shrug."""
    hips = (j["hip_l"] + j["hip_r"]) / 2
    neck = j.get("spine_top", j["neck"])
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
    height = tb["head"].head_local.z / sb["head"].head_local.z
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


# The garments' edges are cut into the mesh along planes (bmesh's bisect), so they are clean
# lines rather than the faces' steps. Each plane is (a point, its normal, the box of the body it
# cuts: x half-width and z range), in the source's frame.
BAND = 0.012   # the waistband's and the band top's hem width (m)
# The band top's sides: out to just past the shoulder joint's line, where the arm leaves the
# torso, so it wraps the bust's sides and stops at the armpit on one clean edge.
BAND_SIDE = 0.005


def _cuts(j, sex):
    """The garments' planes: [(point, normal, (half-width, z0, z1))]."""
    hip = j["hip_l"]
    waist, crotch = hip.z + 0.085, hip.z - 0.075
    box = (hip.x + 0.16, hip.z - 0.2, hip.z + 0.15)
    out = [(Vector((0, 0, waist)), Vector((0, 0, 1)), box), (Vector((0, 0, waist - BAND)), Vector((0, 0, 1)), box)]
    for s in (1, -1):
        # A leg opening: from the crotch up toward the hip's side, round the thigh.
        out.append((Vector((s * 0.025, 0, crotch)), Vector((-0.95 * s, 0, 1)).normalized(), box))
    if sex == "female":
        lo, hi = j["chest"].z + 0.015, j["chest"].z + 0.135
        chest = (j["shoulder_l"].x + 0.02, lo - 0.05, hi + 0.05)
        out += [(Vector((0, 0, z)), Vector((0, 0, 1)), chest) for z in (lo, lo + BAND, hi)]
        side = j["shoulder_l"].x + BAND_SIDE
        out += [(Vector((s * side, 0, 0)), Vector((1, 0, 0)), chest) for s in (1, -1)]
    return out


def _garment(c, j, sex):
    """What a face centred at `c` (source) is: "trim", "under" or None (skin)."""
    hip = j["hip_l"]
    waist, crotch = hip.z + 0.085, hip.z - 0.075
    s = 1 if c.x >= 0 else -1
    above_leg = (c - Vector((s * 0.025, 0, crotch))).dot(Vector((-0.95 * s, 0, 1))) > 0
    if above_leg and c.z < waist and abs(c.x) < hip.x + 0.11:
        return "trim" if c.z > waist - BAND else "under"
    if sex == "female":
        lo, hi = j["chest"].z + 0.015, j["chest"].z + 0.135
        if lo < c.z < hi and abs(c.x) < j["shoulder_l"].x + BAND_SIDE:
            return "trim" if c.z < lo + BAND else "under"
    return None


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
    # The chest at twice the resolution, back on the full-detail surface (sculpting).
    densify(bm, j, sex)
    # The skull's top goes (above the eyes); the face, jaw and neck stay, to be fitted with the
    # body; `marry_neck` then drops the head and seats the source's own neck in the authored one.
    cut = j["head"].z + 0.07
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if v.co.z > cut], context="VERTS")
    _keep_largest(bm)
    from .creator import SKIN
    for v in bm.verts:
        v[col] = (*SKIN, 1.0)
    # The garments' edges cut into the mesh, then each face is skin, cloth or its trim.
    for co, no, (half, z0, z1) in _cuts(j, sex):
        faces = [f for f in bm.faces if all(abs(v.co.x) < half and z0 < v.co.z < z1 for v in f.verts)]
        geom = list({v for f in faces for v in f.verts}) + list({e for f in faces for e in f.edges}) + faces
        bmesh.ops.bisect_plane(bm, geom=geom, dist=1e-5, plane_co=co, plane_no=no)
    kind = {}
    for f in bm.faces:
        f.smooth = False
        kind[f] = _garment(f.calc_center_median(), j, sex)
        f.material_index = cloth_i if kind[f] else skin_i
    # Split where the colors change (skin, cloth, trim), so each stays crisp.
    edge = [e for e in bm.edges if len(e.link_faces) == 2 and kind[e.link_faces[0]] != kind[e.link_faces[1]]]
    bmesh.ops.split_edges(bm, edges=edge)
    for f in bm.faces:
        k = _garment(f.calc_center_median(), j, sex)
        if k:
            for v in f.verts:
                v[col] = (*(TRIM if k == "trim" else UNDER), 1.0)
    bm.to_mesh(me)
    bm.free()
    me.color_attributes.active_color = me.color_attributes["Col"]
    # The spine's top against the shoulders, as on the rig (see `_source_bones`).
    rig_gap = armature.data.bones["neck"].head_local.z - armature.data.bones["upperarm_l"].head_local.z
    scale = armature.data.bones["head"].head_local.z / j["head"].z
    j["spine_top"] = Vector((0.0, j["neck"].y, j["shoulder_l"].z + rig_gap / scale))
    src = source_rig(obj, j)
    _spine_off_clavicles(obj)
    _weld_weights(obj)
    # The fit with every bone's weight (cutting to two first steps the skin where a third
    # drops out); the runtime's two-bone budget after.
    fit(obj, src, armature, girth)
    _limit(obj, 2)
    bpy.data.objects.remove(src)
    _ground(obj, armature)
    _triangulate(obj)
    return obj


def _triangulate(obj):
    """Split every face into triangles along its flatter diagonal (Blender's "beauty"), so a
    curved quad doesn't fold the wrong way when the exporter splits it: on faceted skin a wrong
    fold reads as a crinkle."""
    import bmesh
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bmesh.ops.triangulate(bm, faces=bm.faces[:], quad_method="BEAUTY", ngon_method="BEAUTY")
    bm.to_mesh(obj.data)
    bm.free()


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


def _spine_off_clavicles(obj, inner=0.03, outer=0.09):
    """The automatic weights tie the skin by the spine to the clavicles; the rig's drop
    (`rig.use_shape`'s `drop`) swings those down and folds it. Near the middle the clavicles'
    weight goes to the chest, fading from all of it within `inner` to none past `outer`."""
    groups = {g.name: g for g in obj.vertex_groups}
    chest = groups.get("chest")
    if chest is None:
        return
    clav = {groups[n].index for n in ("clavicle_l", "clavicle_r") if n in groups}
    for v in obj.data.vertices:
        t = max(0.0, min(1.0, (abs(v.co.x) - inner) / (outer - inner)))
        keep = t * t * (3 - 2 * t)
        moved = 0.0
        for g in v.groups:
            if g.group in clav and g.weight > 0:
                moved += g.weight * (1.0 - keep)
                g.weight *= keep
        if moved > 0:
            chest.add([v.index], moved, "ADD")


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



def marry_neck(obj, n_body, joint, rise=0.012):
    """Marry the body (vertices below `n_body`) to the authored head (the rest), the source's
    own neck kept as it was modelled: the source's head goes (every vertex the head's bone weighs
    at half or more: its face, jaw and chin), and each vertex of the neck's top edge then goes
    straight up to `rise` inside the head's underside, the edge capped inside. In front of the
    head's `joint` a line straight up mustn't pass the chin (it would find the lips): a vertex
    whose line does is drawn back toward the neck's middle until its line meets the chin. The neck's last ring
    stretches into the head; nothing is added but the cap."""
    import bmesh
    from mathutils.bvhtree import BVHTree
    me = obj.data
    g = obj.vertex_groups["head"].index
    bm = bmesh.new()
    bm.from_mesh(me)
    deform = bm.verts.layers.deform.verify()
    col = bm.verts.layers.float_color.get("Col")
    bm.verts.ensure_lookup_table()
    head_v = [v for v in bm.verts if v.index >= n_body]
    tree = BVHTree.FromPolygons([v.co.copy() for v in bm.verts],
                                [[v.index for v in f.verts] for f in bm.faces if all(v.index >= n_body for v in f.verts)])
    body = {v for v in bm.verts if v.index < n_body}
    gone = [v for v in body if v[deform].get(g, 0.0) >= 0.5]
    bmesh.ops.delete(bm, geom=gone, context="VERTS")
    _keep_neck(bm, body, joint.z - 0.12)
    # The neck's top edge: the body's open edges, but the color seams' (each point has a twin).
    places = {}
    for v in bm.verts:
        k = tuple(round(c, 5) for c in v.co)
        places[k] = places.get(k, 0) + 1
    edge = {v for e in bm.edges if e.is_boundary for v in e.verts
            if v in body and places[tuple(round(c, 5) for c in v.co)] == 1}
    top = [v for v in edge if v.co.z > joint.z - 0.15]
    if top:
        axis = sum((v.co for v in top), Vector()) / len(top)

        def up(p):
            hit, _n, _i, _d = tree.ray_cast(p + Vector((0, 0, 1e-4)), Vector((0.0, 0.0, 1.0)), 0.3)
            return hit

        ring = sorted(top, key=lambda v: math.atan2(v.co.y - axis.y, v.co.x - axis.x))
        steps = (0.0, 0.15, 0.3, 0.45, 0.6, 0.75, 0.85)

        def at(v, t):
            return Vector((v.co.x + (axis.x - v.co.x) * t, v.co.y + (axis.y - v.co.y) * t, v.co.z))

        def ok(v, t):
            p = at(v, t)
            hit = up(p)
            # Not past the chin in front (the lips), nor up under an ear at the sides.
            if hit is not None and (p.y > joint.y or hit.z <= joint.z) and hit.z <= joint.z + 0.05:
                return hit
            return None

        # How far in each point must go; then each takes the most of its neighbours' along the
        # edge, so the edge is drawn in evenly (one point in, the next out, reads as teeth).
        need = []
        for v in ring:
            need.append(next((t for t in steps if ok(v, t) is not None), steps[-1]))
        n = len(ring)
        even = [max(need[(i + d) % n] for d in (-2, -1, 0, 1, 2)) for i in range(n)]
        for v, t in zip(ring, even):
            hit = ok(v, t)
            p = at(v, t)
            v.co = Vector((p.x, p.y, max(v.co.z, (hit.z + rise) if hit is not None else joint.z)))

        mid = bm.verts.new(axis + Vector((0, 0, rise)))
        mid[deform][g] = 1.0
        if col is not None:
            mid[col] = ring[0][col]
        skin = next((f.material_index for f in bm.faces if f.verts[0] in body), 0)
        for i in range(len(ring)):
            try:
                f = bm.faces.new([ring[i], ring[(i + 1) % len(ring)], mid])
            except ValueError:
                continue
            f.normal_update()
            if f.normal.z < 0:
                f.normal_flip()
            f.material_index = skin
            f.smooth = False
    del head_v
    bm.to_mesh(me)
    bm.free()


def _keep_neck(bm, body, above):
    """Drop the small loose bits the head's removal left (an ear's island): pieces wholly
    `above` a height by the head. The garments and their trims are pieces of their own (their
    seams are split), lower down, and stay."""
    import bmesh
    alive = [v for v in body if v.is_valid]
    seen, pieces = set(), []
    for v in alive:
        if v in seen:
            continue
        piece, stack = [], [v]
        seen.add(v)
        while stack:
            u = stack.pop()
            piece.append(u)
            for e in u.link_edges:
                w = e.other_vert(u)
                if w not in seen and w in body:
                    seen.add(w)
                    stack.append(w)
        pieces.append(piece)
    bmesh.ops.delete(bm, geom=[v for p in pieces if len(p) < 40 and all(v.co.z > above for v in p) for v in p],
                     context="VERTS")


def _project(bm, tree, verts, reach=0.012):
    """Each of `verts` along its own normal (either way, `reach` at most) onto the surface in
    `tree`; one the ray misses stays."""
    bm.normal_update()
    moves = []
    for v in verts:
        n = v.normal
        best = None
        for d in (n, -n):
            hit, _no, _i, dist = tree.ray_cast(v.co, d, reach)
            if hit is not None and (best is None or dist < best[1]):
                best = (hit, dist)
        if best is not None:
            moves.append((v, best[0]))
    for v, p in moves:
        v.co = p


# The chest (and bust) is re-subdivided for sculpting: the faces between the belly and the
# collarbones, in front and round the sides, the arms excluded.
def densify(bm, j, sex):
    """Subdivide the chest's faces of the reduced body `bm` once and set every new vertex on the
    full-detail body's surface (`body_<sex>_hi`): the reduction's lost detail comes back where
    sculpting the chest needs it."""
    import bmesh
    from mathutils.bvhtree import BVHTree
    with bpy.data.libraries.load(SOURCE, link=False) as (_src, dst):
        dst.meshes = [f"body_{sex}_hi"]
    hi = dst.meshes[0]
    hb = bmesh.new()
    hb.from_mesh(hi)
    tree = BVHTree.FromBMesh(hb)
    sh = j["shoulder_l"]
    z0, z1 = j["chest"].z - 0.06, sh.z - 0.02

    def chest(f):
        c = f.calc_center_median()
        return z0 < c.z < z1 and abs(c.x) < sh.x - 0.01 and c.y < 0.04

    # The whole body onto the sculpted surface first: the reduced mesh is a cage, its points
    # standing off the surface the sculpt describes.
    _project(bm, tree, bm.verts)
    faces = [f for f in bm.faces if chest(f)]
    edges = list({e for f in faces for e in f.edges})
    old = set(bm.verts)
    # Smooth subdivision (the new points follow the surface's curve), then each new point
    # pushed along its own normal onto the full-detail surface, a few millimetres at most: a
    # nearest-point snap jumps between faces and crinkles the skin.
    bmesh.ops.subdivide_edges(bm, edges=edges, cuts=1, use_grid_fill=True, smooth=1.0)
    _project(bm, tree, [v for v in bm.verts if v not in old])
    hb.free()
    bpy.data.meshes.remove(hi)


# --- A build's own mass (22 §1.4) ------------------------------------------------------------

PRESETS = os.path.normpath(os.path.join(HERE, "..", "..", "..", "art", "creator", "bodies", "presets.txt"))


def read_presets(path=PRESETS):
    """The body sliders' presets (`art/creator/bodies/presets.txt`, as `mftr_forge::body` reads
    it): {id: {helper: {"t": [x, y, z], "s": [x, y, z]}}}, mirrors expanded."""
    out, cur = {}, None
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if line.startswith("["):
                cur = out.setdefault(line[1:line.index("]")].rstrip("+"), {})
                continue
            w = line.split()
            bone, kind, v = w[0], w[1], [int(x) for x in w[2:5]]
            cur.setdefault(bone, {})[kind] = v
            if len(w) > 5 and w[5] == "mirror":
                other = bone[:-2] + "_r"
                cur.setdefault(other, {})[kind] = [-v[0], v[1], v[2]] if kind == "t" else list(v)
    return out


def bake_mass(armature, objects, mix):
    """Bake a build's own mass into `objects` (the body and its parts): `mix` is {slider: value},
    the body sliders' presets summed, carried through the helpers' weights as the forge carries
    a sculpt (each helper scales about its head in its own frame, then moves). Every shape key
    moves alike, so the morphs keep their deltas."""
    presets = read_presets()
    delta = {}
    for sid, k in mix.items():
        for bone, kinds in presets.get(sid, {}).items():
            d = delta.setdefault(bone, {"t": [0.0] * 3, "s": [0.0] * 3})
            for kind, v in kinds.items():
                for i in range(3):
                    d[kind][i] += v[i] * k
    bones = armature.data.bones
    maps = {}
    for name, d in delta.items():
        b = bones.get(name)
        if b is None:
            continue
        m = b.matrix_local
        inv = m.inverted()
        scale = Matrix.Diagonal([2.0 ** (x / 1000.0) for x in d["s"]] + [1.0])
        move = Matrix.Translation(Vector(d["t"]) / 10000.0)
        maps[name] = m @ move @ scale @ inv
    for obj in objects:
        names = {g.index: g.name for g in obj.vertex_groups}
        me = obj.data
        moved = {}
        for v in me.vertices:
            acc = Vector()
            for g in v.groups:
                t = maps.get(names[g.group])
                if t is not None and g.weight > 0:
                    acc += ((t @ v.co) - v.co) * g.weight
            if acc.length > 1e-7:
                moved[v.index] = acc
        if me.shape_keys:
            for kb in me.shape_keys.key_blocks:
                for i, d in moved.items():
                    kb.data[i].co += d
        for i, d in moved.items():
            me.vertices[i].co += d
