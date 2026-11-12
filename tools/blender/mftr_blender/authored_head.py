# SPDX-License-Identifier: AGPL-3.0-or-later
"""The authored head (21 §1.1, FR2): the champion creator's humanoid head, from an artist-made
mesh instead of `head.py`'s procedural skull.

The source is Blender Studio's **Human Base Meshes** "Generic Topology" head (CC0, see
`art/creator/heads/README.md`): 318 quads with proper eye, mouth and nose loops and artist-made
variants as shape keys, subdivided once here to ~1,270 vertices (21 §1.2's budget). Each base
mixes its variant (Real_MALE or Real_FEMALE, with Ork on the heavy builds, the owner's choice
2026-11-09), fits it to the rig's head frame (`head.Head`: base, height, scale), opens the lids
onto low eyeballs, and paints brows and lips as crisp areas (their faces' vertices split, so the
color doesn't smear across the skin).

All variants share one topology, so the landmarks (eyes, nose, lips, chin…) are found once on the
mesh and the face bones' pivots (`face_rig`) sit on them.
"""

import math
import os

import bmesh
import bpy
from mathutils import Vector

HERE = os.path.dirname(os.path.abspath(__file__))
SOURCE = os.path.normpath(os.path.join(HERE, "..", "..", "..", "art", "creator", "heads", "head_source.blend"))

# Each base's variant mix (shape key: value).
MIX = {
    ("humanoid_slim", "masculine"): {"Real_MALE": 1.0},
    ("humanoid_slim", "feminine"): {"Real_FEMALE": 1.0},
    ("humanoid_heavy", "masculine"): {"Real_MALE": 1.0, "Ork": 0.45},
    ("humanoid_heavy", "feminine"): {"Real_FEMALE": 1.0, "Ork": 0.25},
}

# Landmarks in the source's own frame (metres, face toward -Y, Real_MALE; the same vertices
# move with the variant, so they are found on each mix by these positions' nearest vertices).
CHIN_Z, CROWN_Z = -0.0892, 0.1359
EYE = Vector((0.030, -0.0431, 0.020))
# The lid opening round each eye centre (source metres): an ellipse, x and z half-axes.
LID = (0.0105, 0.0048)
# The eyeball (22 §2.1), its centre this far behind the eye landmark (source metres): its front
# sits just behind the lids' rim (10.6–14.5 mm from the centre).
EYEBALL_R = 0.0108
EYEBALL_BACK = 0.0085
# The lid shells (22 §2.3): spherical bands round the eyeball, the upper inside the lower so a
# closed upper lid tucks behind the lower. The opening they close (measured on the rim): x and z
# half-axes. A lid turns about the eyeball's centre on its bone.
LID_R = {"upper": 0.0113, "lower": 0.0116}
OPENING = (0.0108, 0.0064)
# How far the upper lid rests below the rim at the eye's middle (radians): over the iris's top.
REST_DROP = math.radians(14)
# The default iris (brown) and pupil size (0–1), until the forge writes the recipe's.
IRIS = (0.30, 0.18, 0.09)
PUPIL = 0.5
# Brows: a band over each eye; lips: round the mouth line.
BROW = {"z": (EYE.z + 0.010, EYE.z + 0.021), "x": (0.011, 0.050)}
LIPS = {"z": (-0.0525, -0.0375), "x": 0.024}
MOUTH_Z = -0.0449


def _source_mesh(mix):
    """The source head with `mix` applied and one level of subdivision, as a new mesh."""
    name = "head_source"
    with bpy.data.libraries.load(SOURCE, link=False) as (src, dst):
        dst.objects = [name]
    obj = dst.objects[0]
    bpy.context.scene.collection.objects.link(obj)
    for kb in obj.data.shape_keys.key_blocks:
        kb.value = mix.get(kb.name, 0.0)
    for m in obj.modifiers:
        if m.type == "SUBSURF":
            m.levels = m.render_levels = 1
    dg = bpy.context.evaluated_depsgraph_get()
    me = bpy.data.meshes.new_from_object(obj.evaluated_get(dg))
    data = obj.data
    bpy.data.objects.remove(obj)
    if data.users == 0:
        bpy.data.meshes.remove(data)
    return me


# Where the skull sits on the neck (source metres): behind the jaw at the earlobes' height. The
# rig's head joint (the `head` bone's head) goes here, so the neck always runs up into the skull.
JOINT = Vector((0.0, 0.025, -0.07))


class Fit:
    """Source frame → the rig: scaled to `head`'s height (chin to crown), with the source's
    `JOINT` on the rig's head joint `joint`."""

    def __init__(self, head, joint):
        self.head = head
        self.joint = Vector(joint)
        self.s = head.height * 0.97 / (CROWN_Z - CHIN_Z)

    def __call__(self, p):
        return self.joint + (Vector(p) - JOINT) * self.s


def prepare(head, joint):
    """The fit to the rig (before the mesh is built: the face bones' pivots come from it)."""
    return Fit(head, joint)


def build(bm, layers, groups, mats, head, mix, fit):
    """Add the authored head to `bm` (a mannequin part, as `head.build`): the skin, the brows
    and lips painted, the lids opened onto eyeballs (bound to the eye bones when the armature
    has them)."""
    deform, col = layers
    g = groups["head"]
    me = _source_mesh(mix)
    src = bmesh.new()
    src.from_mesh(me)
    bpy.data.meshes.remove(me)
    src.faces.ensure_lookup_table()

    def in_lid(p, side):
        dx = (p.x - side * EYE.x) / LID[0]
        dz = (p.z - EYE.z) / LID[1]
        return dx * dx + dz * dz < 1.0 and p.y < EYE.y + 0.01

    def region(f):
        c = f.calc_center_median()
        if any(in_lid(c, s) for s in (1, -1)):
            return "lid"
        if BROW["z"][0] < c.z < BROW["z"][1] and BROW["x"][0] < abs(c.x) < BROW["x"][1] and c.y < EYE.y + 0.01:
            return "brow"
        if LIPS["z"][0] < c.z < LIPS["z"][1] and abs(c.x) < LIPS["x"] and c.y < -0.045:
            return "lips"
        return "skin"

    # The lids open: their faces go.
    lids = [f for f in src.faces if region(f) == "lid"]
    bmesh.ops.delete(src, geom=lids, context="FACES")
    # Brows and lips own their vertices (split from the skin), so their color stays crisp.
    painted = [f for f in src.faces if region(f) in ("brow", "lips")]
    edges = {e for f in painted for e in f.edges if any(region(o) not in ("brow", "lips") for o in e.link_faces)}
    bmesh.ops.split_edges(src, edges=list(edges))
    src.faces.ensure_lookup_table()
    colors = {"skin": head.skin, "brow": head.hair, "lips": head.lips}
    out_v = {}
    for v in src.verts:
        nv = bm.verts.new(fit(v.co))
        nv[deform][g] = 1.0
        out_v[v] = nv
    for f in src.faces:
        r = region(f)
        nf = bm.faces.new([out_v[v] for v in f.verts])
        nf.material_index = mats["cloth"] if r == "brow" else mats["skin"]
        nf.smooth = False
        for v in f.verts:
            out_v[v][col] = (*colors[r], 1.0)
    # The mouth line a little darker (the lips' meeting).
    for v in src.verts:
        if abs(v.co.z - MOUTH_Z) < 0.0018 and abs(v.co.x) < LIPS["x"] * 0.9 and v.co.y < -0.05:
            out_v[v][col] = (*(c * 0.5 for c in head.lips), 1.0)
    # The sockets' walls (22 §2.3): from each opening's rim in behind the eyeball, so a look
    # past the ball at the corners finds flesh, never the inside of the head.
    for side in (1, -1):
        # On the head like the skin it hangs from: the face rig weights it with its neighbours.
        _socket_wall(src, bm, out_v, deform, col, mats, fit, Vector((side * EYE.x, EYE.y + EYEBALL_BACK, EYE.z)),
                     g, head.skin)
    src.free()
    # The eyes (22 §2): an eyeball behind each opening, turning on its look bone, and an upper
    # and a lower lid that close over it on theirs. The iris is drawn by the shader from the
    # ball's own coordinates (`eye_uvs`, after the unwrap).
    for side in (1, -1):
        sfx = "l" if side > 0 else "r"
        eye_g = groups.get(f"face_eye_{sfx}", g)
        c = Vector((side * EYE.x, EYE.y + EYEBALL_BACK, EYE.z))
        _eyeball(bm, deform, col, mats, fit, c, groups.get(f"eye_look_{sfx}", eye_g))
        for which in ("upper", "lower"):
            _lid(bm, deform, col, mats, fit, c, groups.get(f"lid_{which}_{sfx}", eye_g), which, head.skin)
    return fit


def _socket_wall(src, bm, out_v, deform, col, mats, fit, c, group, skin):
    """A ring of faces from the lid opening's rim round source centre `c` (its boundary edges in
    `src`, the source head after the lids were opened) in to a smaller ring behind the eyeball's
    front, facing out of the opening: the socket's flesh, darker toward the back."""
    rim = [v for v in src.verts if (v.co - c).length < 0.02 and any(e.is_boundary for e in v.link_edges)]
    if len(rim) < 3:
        return
    rim.sort(key=lambda v: math.atan2(v.co.z - c.z, v.co.x - c.x))
    inner = []
    for v in rim:
        d = v.co - c
        # In toward the eye's axis and back, well inside the eyeball, which hides it.
        q = c + Vector((d.x * 0.62, d.y * 0.62 + 0.004, d.z * 0.62))
        nv = bm.verts.new(fit(q))
        nv[deform][group] = 1.0
        nv[col] = (skin[0] * 0.85, skin[1] * 0.62, skin[2] * 0.6, 0.35)
        inner.append(nv)
    front = fit(c + Vector((0, -0.03, 0)))
    n = len(rim)
    for i in range(n):
        j = (i + 1) % n
        quad = [out_v[rim[i]], out_v[rim[j]], inner[j], inner[i]]
        if len({id(v) for v in quad}) < 4:
            continue
        f = bm.faces.new(quad)
        f.normal_update()
        if f.normal.dot(front - f.calc_center_median()) < 0:
            f.normal_flip()
        f.material_index = mats["skin"]
        f.smooth = False

def _eyeball(bm, deform, col, mats, fit, c, group):
    """An eyeball at source centre `c`, its pole looking out (-Y), skinned to `group`."""
    r = EYEBALL_R * fit.s
    centre = fit(c)
    ball = bmesh.ops.create_uvsphere(bm, u_segments=14, v_segments=10, radius=r)["verts"]
    # Only the very back of the ball goes: the opening's corners look past the lids onto its
    # sides, which must be there (shaded as the sclera's shadowed edge).
    back = [v for v in ball if v.co.z < -r * 0.8]  # local +z becomes the front (-Y)
    bmesh.ops.delete(bm, geom=back, context="VERTS")
    ball = [v for v in ball if v.is_valid]
    for v in ball:
        local = v.co.copy()
        v.co = centre + Vector((local.x, -local.z, local.y))  # the sphere's pole toward -Y
        v[deform][group] = 1.0
        v[col] = (*IRIS, PUPIL)
    for f in {f for v in ball for f in v.link_faces}:
        f.material_index = mats["skin"]
        f.smooth = True


def _lid(bm, deform, col, mats, fit, c, group, which, skin):
    """A lid shell round the eyeball at source centre `c`: a band of the sphere of radius
    `LID_R[which]` from the opening's rim (its margin, darkened: the lash line) away from it,
    across the eye's width. At rest it hides behind the skin round the opening; turned about
    the centre it closes over the eyeball."""
    rad = LID_R[which]
    sign = 1.0 if which == "upper" else -1.0
    cols, rows = 10, 5
    # Far enough that a closed upper lid still reaches behind the rim's top.
    reach = math.radians(80 if which == "upper" else 50)
    edge = math.asin(min(0.98, (OPENING[0] + 0.0025) / rad))

    def margin(az):
        # The rim's height over the eye at this azimuth: the opening's ellipse. The upper lid
        # rests over the top of the iris, as an eye at ease does (`REST_DROP`); the lower lid
        # just inside its rim.
        x = rad * math.sin(az)
        z = OPENING[1] * math.sqrt(max(0.0, 1.0 - (x / OPENING[0]) ** 2))
        el = math.asin(min(0.95, z / rad))
        return el - (REST_DROP * (z / OPENING[1]) if which == "upper" else math.radians(-1))

    grid = []
    for i in range(cols + 1):
        az = -edge + 2 * edge * i / cols
        m = margin(az)
        row = []
        for j in range(rows + 1):
            el = sign * (m + reach * j / rows)
            p = Vector((math.sin(az) * math.cos(el), -math.cos(az) * math.cos(el), math.sin(el))) * rad
            v = bm.verts.new(fit(c + p))
            v[deform][group] = 1.0
            # The margin darker (the lash line) through the occlusion alpha, so the skin tone
            # and the paint step treat the lid as skin.
            v[col] = (*skin, (0.0 if which == "upper" else 0.45) if j == 0 else 1.0)
            row.append(v)
        grid.append(row)
    for i in range(cols):
        for j in range(rows):
            quad = [grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1]]
            if which == "lower":
                quad.reverse()
            f = bm.faces.new(quad)
            f.material_index = mats["skin"]
            f.smooth = False


# Where the eyes' data rides (22 §2.1), read by `champion_model.gdshader` and rewritten by the
# forge (`mftr_forge::eyes`): an eye vertex's UV is (2 + 2·style + u, 2·code + v), (u, v) its
# place on the ball seen from the front (0–1), `style` the iris's kind and `code` its size and
# the sclera; its color is the iris, its alpha the pupil's size.
EYE_UV_BASE = 2.0
DEFAULT_CODE = 4


def eye_uvs(armature, body):
    """Write each eyeball vertex's place on its ball into its UV (after `creator.unwrap`, which
    projects the whole body): see `EYE_UV_BASE`."""
    me = body.data
    uv = me.uv_layers.active
    names = {g.index: g.name for g in body.vertex_groups}
    centre = {}
    for sfx in ("l", "r"):
        b = armature.data.bones.get(f"eye_look_{sfx}")
        if b is not None:
            centre[f"eye_look_{sfx}"] = b.head_local
    if not centre:
        return 0
    place = {}
    for v in me.vertices:
        for gr in v.groups:
            n = names[gr.group]
            if n in centre and gr.weight > 0.5:
                d = (v.co - centre[n]).normalized()
                u = min(0.999, max(0.0, d.x * 0.5 + 0.5))
                w = min(0.999, max(0.0, d.z * 0.5 + 0.5))
                # glTF flips v (1 - v): the shader reads 2·code + w.
                place[v.index] = (EYE_UV_BASE + u, 1.0 - (2 * DEFAULT_CODE + w))
    for poly in me.polygons:
        for li in poly.loop_indices:
            vi = me.loops[li].vertex_index
            if vi in place:
                uv.data[li].uv = place[vi]
    return len(place)


def _crown_back(fit):
    """The crown and the back of the skull, fitted (warp anchors besides the face bones)."""
    return {"crown": fit(Vector((0.0, 0.030, CROWN_Z))), "back": fit(Vector((0.0, 0.135, 0.020))),
            "under": fit(Vector((0.0, 0.040, -0.075)))}


def make_conform(old_head, fit, body):
    """A function conforming a catalog part (an object) made for the block-out head `old_head`
    to the authored head (21 §1.1, FR2):

    1. Its head-bound vertices move by a thin-plate spline through matching landmarks on the old
       head and the new (the face bones' pivots, the crown, the back of the skull), so a beard
       lands on the new chin and goggles on the new eyes.
    2. Any that end up inside the new head are pushed out to 3 mm off its skin.
    3. They take the head's weights (face bones included) from the nearest skin vertex, so a
       part follows the face's sculpt as the skin does.
    """
    import numpy as np
    from mathutils.bvhtree import BVHTree
    from . import face_rig

    old = face_rig.mirrored({k: v for k, v in face_rig.pivots(old_head).items() if not k.endswith("_r")})
    new = face_rig.mirrored(landmarks(fit))
    k = old_head.scale
    old.update({"crown": old_head.base + Vector((0, 0.01 * k, old_head.height)),
                "back": old_head.surface(0.55, 1.5708), "under": old_head.base + Vector((0, 0.03 * k, 0.04 * old_head.height))})
    new.update(_crown_back(fit))
    names = sorted(old)
    A = np.array([tuple(old[n]) for n in names])
    Bv = np.array([tuple(new[n]) for n in names])
    n = len(names)
    # Thin-plate spline in 3D (kernel r): K w + P a = B, Pᵀ w = 0.
    K = np.linalg.norm(A[:, None, :] - A[None, :, :], axis=2)
    P = np.hstack([np.ones((n, 1)), A])
    M = np.zeros((n + 4, n + 4))
    M[:n, :n] = K
    M[:n, n:] = P
    M[n:, :n] = P.T
    rhs = np.zeros((n + 4, 3))
    rhs[:n] = Bv
    sol = np.linalg.solve(M + np.eye(n + 4) * 1e-9, rhs)
    w, aff = sol[:n], sol[n:]

    def warp(p):
        q = np.array(tuple(p))
        r = np.linalg.norm(A - q, axis=1)
        out = aff[0] + q @ aff[1:] + r @ w
        return Vector(tuple(out))

    me = body.data
    g_head = body.vertex_groups["head"].index
    head_v = [v.index for v in me.vertices if any(g.group == g_head and g.weight > 0.0 for g in v.groups)
              or any(body.vertex_groups[g.group].name in face_rig.NAMES for g in v.groups if g.weight > 0.0)]
    head_v = [i for i in head_v if not any(body.vertex_groups[g.group].name.startswith("face_eye") for g in me.vertices[i].groups)]
    tree = BVHTree.FromPolygons([me.vertices[i].co.copy() for i in range(len(me.vertices))],
                                [tuple(p.vertices) for p in me.polygons
                                 if all(any(g.group == g_head or body.vertex_groups[g.group].name in face_rig.NAMES
                                            for g in me.vertices[v].groups) for v in p.vertices)])
    from mathutils.kdtree import KDTree
    kd = KDTree(len(head_v))
    for j, i in enumerate(head_v):
        kd.insert(me.vertices[i].co, j)
    kd.balance()
    groups_of = {}
    for i in head_v:
        groups_of[i] = [(body.vertex_groups[g.group].name, g.weight) for g in me.vertices[i].groups if g.weight > 0.0]

    eye_z = new["face_eye_l"].z

    def conform(obj):
        g = obj.vertex_groups.get("head")
        if g is None:
            return 0
        verts = [v for v in obj.data.vertices if next((x.weight for x in v.groups if x.group == g.index), 0.0) >= 0.5]
        warped = {v.index: warp(v.co) for v in verts}
        # Headwear rests on the skull: one that floats (its nearest point above the eyes more
        # than a centimetre off the skin) settles down until it touches.
        if obj.name.startswith("part.head.") and verts:
            gaps = []
            for p in warped.values():
                if p.z > eye_z:
                    loc, normal, _, dist = tree.find_nearest(p)
                    if loc is not None and (p - loc).dot(normal) > 0:
                        gaps.append(dist)
            if gaps and min(gaps) > 0.01:
                drop = Vector((0, 0, -(min(gaps) - 0.004)))
                warped = {i: p + drop for i, p in warped.items()}
        moved = 0
        for v in verts:
            p = warped[v.index]
            loc, normal, _, dist = tree.find_nearest(p)
            # Off the skin by at least 6 mm.
            if loc is not None and (p - loc).dot(normal) < 0.006:
                p = loc + normal * 0.006
            v.co = p
        # A part's faces are bigger than the skin's: where one's middle or an edge's midpoint
        # dips into the skin (the skull's curve between its corners), its corners move out.
        on_head = {v.index for v in verts}
        polys = [p for p in obj.data.polygons if all(i in on_head for i in p.vertices)]
        for _ in range(6):
            push = {}
            for poly in polys:
                cs = [obj.data.vertices[i].co for i in poly.vertices]
                samples = [sum(cs, Vector()) / len(cs)] + [(cs[i] + cs[(i + 1) % len(cs)]) / 2 for i in range(len(cs))]
                for s in samples:
                    loc, normal, _, _ = tree.find_nearest(s)
                    if loc is None:
                        continue
                    deficit = 0.004 - (s - loc).dot(normal)
                    if deficit > 0:
                        for i in poly.vertices:
                            prev = push.get(i)
                            step = normal * deficit
                            if prev is None or step.length > prev.length:
                                push[i] = step
            if not push:
                break
            for i, step in push.items():
                obj.data.vertices[i].co += step
        # Weights: wherever the head holds a part (all of it or part of it), the skin's weights
        # round there (face bones included) take the head's share, averaged over 5 cm so a
        # part's big faces don't jump from bone to bone (21 §1.4's rule holds on parts too).
        names = {gg.index: gg.name for gg in obj.vertex_groups}
        for v in obj.data.vertices:
            hw = next((x.weight for x in v.groups if x.group == g.index), 0.0)
            if hw <= 0.0:
                continue
            p = v.co
            near = kd.find_range(p, 0.05) or [kd.find(p)]
            acc, total = {}, 0.0
            for _, j, d in near:
                f = max(1.0 - d / 0.05, 0.05)
                total += f
                for name, wgt in groups_of[head_v[j]]:
                    acc[name] = acc.get(name, 0.0) + wgt * f
            mix = {n: w / total * hw for n, w in acc.items()}
            for x in v.groups:
                n = names[x.group]
                if n != "head" and n not in face_rig.NAMES:
                    mix[n] = mix.get(n, 0.0) + x.weight
            top = sorted(((w, n) for n, w in mix.items() if w > 1e-4), reverse=True)[:4]
            norm = sum(w for w, _ in top) or 1.0
            for x in list(v.groups):
                obj.vertex_groups[x.group].remove([v.index])
            for w, n in top:
                tg = obj.vertex_groups.get(n) or obj.vertex_groups.new(name=n)
                names[tg.index] = n
                tg.add([v.index], w / norm, "REPLACE")
        # And no edge of the part jumps (the part's own faces are long).
        face_rig.limit_steps(obj)
        return len(verts)

    return conform


def landmarks(fit):
    """Where the face bones go (21 §1.3), fitted: the left side's (the right mirrors it)."""
    def at(x, y, z):
        return fit(Vector((x, y, z)))

    return {
        "face_forehead": at(0.0, -0.048, 0.082),
        "face_brow_mid": at(0.0, -0.055, 0.034),
        "face_brow_l": at(0.031, -0.050, 0.036),
        "face_eye_l": at(EYE.x, EYE.y + 0.0085, EYE.z),
        "face_cheekbone_l": at(0.047, -0.040, 0.000),
        "face_cheek_l": at(0.042, -0.040, -0.030),
        "face_nose_bridge": at(0.0, -0.058, 0.020),
        "face_nose_tip": at(0.0, -0.070, -0.012),
        "face_nostril_l": at(0.014, -0.062, -0.018),
        "face_lip_upper": at(0.0, -0.058, -0.038),
        "jaw": at(0.0, 0.020, -0.008),
        "face_lip_lower": at(0.0, -0.057, -0.052),
        "face_mouth_l": at(0.024, -0.052, -0.045),
        "face_chin": at(0.0, -0.052, -0.074),
        "face_jaw_angle_l": at(0.060, 0.005, -0.060),
        "face_ear_l": at(0.072, 0.022, 0.006),
    }
