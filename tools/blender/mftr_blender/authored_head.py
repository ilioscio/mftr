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
# The lid opening round each eye centre (source metres): an ellipse, x and z half-axes. The lid
# shapes' fields (`LID_SHAPES`) measure the skin against it.
LID = (0.0105, 0.0048)
# The opening's rim (22 §2.3), relative to the eye landmark, x toward the outer corner: the
# corners, and the upper and lower lids' heights over the line between them, each lid highest
# (lowest) a little way along it, as an eye's are: the upper lid's peak toward the nose, the
# lower's toward the temple. The skin's rim is set on it and the lid shells' margins follow it.
RIM = {"inner": -0.0100, "outer": 0.0105, "inner_z": -0.0003, "outer_z": 0.0009,
       "up": 0.0050, "down": 0.0040, "up_peak": 0.42, "down_peak": 0.6}
# How far in front of the eyeball's centre the rim stays at least: in front of the lid shells.
RIM_CLEAR = 0.0120
# The eyeball (22 §2.1), its centre this far behind the eye landmark (source metres): its front
# sits just behind the lids' rim (10.6–14.5 mm from the centre).
EYEBALL_R = 0.0108
EYEBALL_BACK = 0.0085
# The lid shells (22 §2.3): spherical bands round the eyeball, the upper inside the lower so a
# closed upper lid tucks behind the lower. A lid turns about the eyeball's centre on its bone.
LID_R = {"upper": 0.0113, "lower": 0.0116}
# How far the upper lid rests below the rim at the eye's middle (radians): over the iris's top.
REST_DROP = math.radians(14)
# The default iris (brown) and pupil size (0–1), until the forge writes the recipe's.
IRIS = (0.30, 0.18, 0.09)
PUPIL = 0.5
# The closed mouth's line (source metres): the lips (22 §3.2) are the faces within `LIP_RINGS`
# edges of its seam.
MOUTH_Z = -0.0449
LIP_RINGS = 2


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

    opened = False

    def region(f):
        c = f.calc_center_median()
        if any(in_lid(c, s) for s in (1, -1)):
            # Once the lids are open, what's left round them (the crease loop's faces) is skin.
            return "skin" if opened else "lid"
        if f in lips:
            return "lips"
        return "skin"

    # The lids open along the source's own eye loop (its faces round the closed lids' seam go),
    # and the rim is set on `RIM`.
    bmesh.ops.delete(src, geom=_lid_faces(src), context="FACES")
    opened = True
    _shape_rims(src)
    _crease_loops(src)
    lips, lip_at = _lips(src)
    # The lips own their vertices (split from the skin), so their color stays crisp. The brows
    # are strips of their own (`_brow_strips`).
    painted = [f for f in src.faces if region(f) == "lips"]
    edges = {e for f in painted for e in f.edges if any(region(o) != "lips" for o in e.link_faces)}
    bmesh.ops.split_edges(src, edges=list(edges))
    src.faces.ensure_lookup_table()
    colors = {"skin": head.skin, "lips": head.lips}
    # Each lip vertex's place on the lips rides in two layers until `lip_uvs`: `lip_s` (1 +
    # across the mouth, 0 off the lips) and `lip_v` (from the lower border, -1, through the
    # line where they meet, 0, to the upper border, 1).
    ls = bm.verts.layers.float.get("lip_s") or bm.verts.layers.float.new("lip_s")
    lv = bm.verts.layers.float.get("lip_v") or bm.verts.layers.float.new("lip_v")
    out_v = {}
    for v in src.verts:
        nv = bm.verts.new(fit(v.co))
        nv[deform][g] = 1.0
        out_v[v] = nv
    for f in src.faces:
        r = region(f)
        nf = bm.faces.new([out_v[v] for v in f.verts])
        nf.material_index = mats["skin"]
        # The lips are smooth (their light rolls over them); the rest of the face is faceted.
        nf.smooth = r == "lips"
        for v in f.verts:
            out_v[v][col] = (*colors[r], 1.0)
            if r == "lips":
                s, w = lip_at(v)
                out_v[v][ls] = 1.0 + s
                out_v[v][lv] = w
    # The lids' edges a little darker (the occlusion alpha): the upper's lash line, the lower's
    # waterline.
    for side in (1, -1):
        c = Vector((side * EYE.x, EYE.y, EYE.z))
        for v in src.verts:
            if (v.co - c).length < 0.02 and any(e.is_boundary for e in v.link_edges):
                out_v[v][col] = (*head.skin, 0.4 if v.co.z > c.z + RIM["up"] * 0.2 else 0.7)
    # The sockets' walls (22 §2.3): from each opening's rim in behind the eyeball, so a look
    # past the ball at the corners finds flesh, never the inside of the head.
    for side in (1, -1):
        # On the head like the skin it hangs from: the face rig weights it with its neighbours.
        _socket_wall(src, bm, out_v, deform, col, mats, fit, Vector((side * EYE.x, EYE.y + EYEBALL_BACK, EYE.z)),
                     g, head.skin)
    _brow_strips(src, bm, deform, col, mats, fit, g, head.hair)
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


# --- Brows (22 §3.1) ---------------------------------------------------------------------------
# Each brow is a strip on the brow ridge: columns from over the nose (`BROW_X[0]`, so a unibrow
# can meet its twin) out to the temple, rows across the brow line from below it to well above it.
# That strip is an envelope: the forge (`mftr_forge::brows`) slides the columns between the
# brow's start and its tip and sets the rows by the style's thickness and arch, so every style
# and size is this one mesh. The strip lies a little off the skin and rides the face bones as
# the skin under it does.
BROW_X = (0.0005, 0.054)
BROW_COLS = 13
# Across the brow line (source metres): the rows' places in the envelope.
BROW_ROWS = (-0.008, -0.0025, 0.0035, 0.011)
BROW_LIFT = 0.0006


def brow_line(x):
    """The brow line's height at `x` (source metres from the middle): a low arc over the eye."""
    t = max(0.0, min(1.0, (x - 0.010) / 0.044))
    return EYE.z + 0.0118 + 0.0025 * math.sin(math.pi * t)


def _brow_strips(src, bm, deform, col, mats, fit, group, hair):
    """A brow strip over each eye on the source head `src` (its surface found by rays from the
    front), on the cloth slot in the hair's color. Each vertex's place in the envelope rides in
    two float layers, `brow_s` (1 + along, 0 off a brow) and `brow_t` (across, 0–1), until
    `brow_uvs` writes them into its UV."""
    from mathutils.bvhtree import BVHTree
    tree = BVHTree.FromBMesh(src)
    s_layer = bm.verts.layers.float.get("brow_s") or bm.verts.layers.float.new("brow_s")
    t_layer = bm.verts.layers.float.get("brow_t") or bm.verts.layers.float.new("brow_t")
    span = BROW_ROWS[-1] - BROW_ROWS[0]
    for side in (1, -1):
        grid = []
        for i in range(BROW_COLS):
            s = i / (BROW_COLS - 1)
            x = BROW_X[0] + (BROW_X[1] - BROW_X[0]) * s
            column = []
            for dt in BROW_ROWS:
                z = brow_line(x) + dt
                hit, n, _, _ = tree.ray_cast(Vector((side * x, EYE.y - 0.1, z)), Vector((0.0, 1.0, 0.0)))
                if hit is None:
                    hit, n, _, _ = tree.find_nearest(Vector((side * x, EYE.y, z)))
                n = n.normalized()
                if n.y > 0:
                    n = -n
                v = bm.verts.new(fit(hit + n * BROW_LIFT))
                v[deform][group] = 1.0
                v[col] = (*hair, 1.0)
                v[s_layer] = 1.0 + s
                v[t_layer] = (dt - BROW_ROWS[0]) / span
                column.append(v)
            grid.append(column)
        front = fit(Vector((side * 0.03, EYE.y - 0.2, EYE.z + 0.015)))
        for i in range(BROW_COLS - 1):
            for j in range(len(BROW_ROWS) - 1):
                f = bm.faces.new([grid[i][j], grid[i + 1][j], grid[i + 1][j + 1], grid[i][j + 1]])
                f.normal_update()
                if f.normal.dot(front - f.calc_center_median()) < 0:
                    f.normal_flip()
                f.material_index = mats["cloth"]
                f.smooth = False


BROW_UV_BASE = 10.0


def brow_uvs(body):
    """Write each brow vertex's place in its envelope into its UV, (10 + along, across), and
    drop the layers that carried it (after `creator.unwrap`)."""
    me = body.data
    if "brow_s" not in me.attributes:
        return 0
    s = [d.value for d in me.attributes["brow_s"].data]
    t = [d.value for d in me.attributes["brow_t"].data]
    uv = me.uv_layers.active
    n = 0
    for poly in me.polygons:
        for li in poly.loop_indices:
            vi = me.loops[li].vertex_index
            if s[vi] > 0.5:
                # glTF flips v (1 - v): the forge reads `across` as it is.
                uv.data[li].uv = (BROW_UV_BASE + min(0.999, s[vi] - 1.0), 1.0 - t[vi])
                n += 1
    me.attributes.remove(me.attributes["brow_s"])
    me.attributes.remove(me.attributes["brow_t"])
    return n


# --- Lips (22 §3.2) -----------------------------------------------------------------------------

def _lips(src):
    """The lips of the source head `src`: the faces within `LIP_RINGS` edges of the closed
    mouth's seam (the edge loop along the mouth line), so their border is an edge loop of the
    source's, not a box. Returns (faces, at): `at(v)` gives a lip vertex's place on them, (across
    the mouth 0–1, from the lower border -1 through the seam 0 to the upper border 1)."""
    near = [e for e in src.edges
            if all(abs(v.co.x) < 0.012 and abs(v.co.z - MOUTH_Z) < 0.004 and v.co.y < -0.04 for v in e.verts)]

    def across(e):
        d = e.verts[1].co - e.verts[0].co
        return abs(d.x) > 2 * abs(d.z)

    e0 = min((e for e in near if across(e)),
             key=lambda e: abs(e.verts[0].co.x + e.verts[1].co.x) / 2 + abs((e.verts[0].co.z + e.verts[1].co.z) / 2 - MOUTH_Z))
    seam = set(e0.verts)
    for v0 in e0.verts:
        for e in _walk(e0, v0):
            seam |= set(e.verts)
    dist = {v: 0 for v in seam}
    ring = set(seam)
    for d in range(1, LIP_RINGS + 1):
        ring = {e.other_vert(v) for v in ring for e in v.link_edges} - dist.keys()
        for v in ring:
            dist[v] = d
    faces = {f for f in src.faces if all(v in dist for v in f.verts)}
    _shape_lips(src, seam, dist, faces)
    # By place, so the copies the lips' split makes (`bmesh.ops.split_edges`) are found too.
    rings = {tuple(round(c, 6) for c in v.co): d for v, d in dist.items()}
    line = sorted((v.co.x, v.co.z) for v in seam)
    half = max(abs(v.co.x) for f in faces for v in f.verts)

    def seam_z(x):
        if x <= line[0][0]:
            return line[0][1]
        for (x0, z0), (x1, z1) in zip(line, line[1:]):
            if x <= x1:
                return z0 + (z1 - z0) * (x - x0) / max(1e-9, x1 - x0)
        return line[-1][1]

    def at(v):
        d = rings[tuple(round(c, 6) for c in v.co)]
        side = 0.0 if d == 0 else (1.0 if v.co.z > seam_z(v.co.x) else -1.0)
        return (min(0.999, max(0.0, 0.5 + v.co.x / (2 * half))), side * d / LIP_RINGS)

    return faces, at


LIP_WIDEN = 1.2


def _shape_lips(src, seam, dist, faces):
    """Set the lips on one outline: the source's rings run unevenly past the mouth's corners
    (the upper lip's about 3 mm past the seam's end, the lower's short of it). Each ring of each
    lip is drawn in across so its ends meet at the seam's end, then set at its share of the
    lip's height over the seam (fullest in the middle, tapering to the corner), on the head's own
    surface."""
    from mathutils.bvhtree import BVHTree
    tree = BVHTree.FromBMesh(src)
    line = sorted((v.co.x, v.co.z) for v in seam)
    ends = max(abs(x) for x, _ in line)
    # The mouth a little wider than the source's seam: its corners `WIDEN` further out.
    corner = ends * LIP_WIDEN

    def source_z(x):
        if x <= line[0][0]:
            return line[0][1]
        for (x0, z0), (x1, z1) in zip(line, line[1:]):
            if x <= x1:
                return z0 + (z1 - z0) * (x - x0) / max(1e-9, x1 - x0)
        return line[-1][1]

    def seam_z(x):
        # The widened seam's height at x.
        return source_z(x * ends / corner)

    lip = {v for f in faces for v in f.verts if dist[v] > 0}
    side = {v: 1 if v.co.z > source_z(v.co.x) else -1 for v in lip}
    reach, height = {}, {1: 0.0, -1: 0.0}
    for v in lip:
        k = (dist[v], side[v])
        reach[k] = max(reach.get(k, 0.0), abs(v.co.x))
        if dist[v] == LIP_RINGS and abs(v.co.x) < 0.003:
            height[side[v]] = max(height[side[v]], abs(v.co.z - source_z(v.co.x)))
    moved = {}
    for v in seam:
        x = v.co.x * LIP_WIDEN
        hit, _n, _i, _d = tree.ray_cast(Vector((x, v.co.y - 0.03, v.co.z)), Vector((0.0, 1.0, 0.0)))
        moved[v] = Vector((x, hit.y if hit is not None else v.co.y, v.co.z))
    for v in lip:
        d, s = dist[v], side[v]
        x = v.co.x * corner * (1.0 - 0.05 * d) / max(1e-9, reach[(d, s)])
        t = min(1.0, abs(x) / corner)
        z = seam_z(x) + s * height[s] * d / LIP_RINGS * (1.0 - t * t) ** (0.6 if s > 0 else 0.7)
        hit, _n, _i, _d = tree.ray_cast(Vector((x, v.co.y - 0.03, z)), Vector((0.0, 1.0, 0.0)))
        moved[v] = Vector((x, hit.y if hit is not None else v.co.y, z))
    for v, p in moved.items():
        v.co = p


LIP_UV_BASE = 12.0


def lip_uvs(body):
    """Write each lip vertex's place into its UV, (12 + across, 3 + up), and drop the layers
    that carried it (after `creator.unwrap`): the shader shades the lips from it."""
    me = body.data
    if "lip_s" not in me.attributes:
        return 0
    s = [d.value for d in me.attributes["lip_s"].data]
    w = [d.value for d in me.attributes["lip_v"].data]
    uv = me.uv_layers.active
    n = 0
    for poly in me.polygons:
        for li in poly.loop_indices:
            vi = me.loops[li].vertex_index
            if s[vi] > 0.5:
                # glTF flips v (1 - v): the shader reads 3 + up.
                uv.data[li].uv = (LIP_UV_BASE + s[vi] - 1.0, 1.0 - (3.0 + w[vi]))
                n += 1
    me.attributes.remove(me.attributes["lip_s"])
    me.attributes.remove(me.attributes["lip_v"])
    return n


# The lips' shapes (22 §3.2): each moves the lip vertices by (across in mm, forward in mm, up in
# mm) from their place on the lips (x mm from the middle, up -1–1, the mouth's half-width in
# mm); the skin within `LIP_SKIN` of the lips' border follows its nearest border vertex, fading.
LIP_SKIN = 0.007


def _lip_h(x, half):
    return max(0.0, 1.0 - (x / half) ** 2) ** 0.6


def _upper(x, up, half):
    if up <= 0:
        return (0.0, 0.0, 0.0)
    h = _lip_h(x, half)
    return (0.0, -1.6 * (0.4 * up + 0.6 * math.sin(math.pi * up)) * h, 1.1 * up * h)


def _lower(x, up, half):
    if up >= 0:
        return (0.0, 0.0, 0.0)
    a = -up
    h = _lip_h(x, half)
    return (0.0, -1.8 * (0.4 * a + 0.6 * math.sin(math.pi * a)) * h, -1.2 * a * h)


def _bow(x, up, half):
    """A cupid's bow: the upper border's two peaks rise and the dip between them deepens."""
    if up <= 0:
        return (0.0, 0.0, 0.0)
    peaks = 1.0 - _sm(abs(abs(x) - 5.5) / 4.0)
    dip = 1.0 - _sm(abs(x) / 3.0)
    return (0.0, -0.3 * dip * up, (1.0 * peaks - 0.9 * dip) * up)


LIP_SHAPES = [
    ("lips_upper", "A full upper lip or a thin one", [("~", _upper)]),
    ("lips_lower", "A full lower lip or a thin one", [("~", _lower)]),
    ("lips_bow", "A marked cupid's bow", [("+", _bow)]),
]


def lip_keys(body, fit):
    """The lip shapes (`LIP_SHAPES`) on the built `body` (before `lip_uvs`, while the lips'
    layers are there), as {slider + suffix: {vertex: delta}} in armature space."""
    me = body.data
    if "lip_s" not in me.attributes:
        return {}
    s = [d.value for d in me.attributes["lip_s"].data]
    w = [d.value for d in me.attributes["lip_v"].data]

    def source(co):
        return JOINT + (Vector(co) - fit.joint) / fit.s

    lips = [i for i in range(len(me.vertices)) if s[i] > 0.5]
    src = {i: source(me.vertices[i].co) for i in lips}
    half = max(abs(p.x) for p in src.values()) * 1000
    border = [i for i in lips if abs(abs(w[i]) - 1.0) < 1e-3]
    skin = []
    for v in me.vertices:
        if s[v.index] > 0.5:
            continue
        p = source(v.co)
        if abs(p.x) > half / 1000 + LIP_SKIN or abs(p.z - MOUTH_Z) > 0.02 or p.y > -0.03:
            continue
        b = min(border, key=lambda i: (src[i] - p).length)
        d = (src[b] - p).length
        if d < LIP_SKIN:
            skin.append((v.index, b, 1.0 - _sm(d / LIP_SKIN)))
    out = {}
    for sid, _desc, ends in LIP_SHAPES:
        for suffix, field in ends:
            moved = {}
            for i in lips:
                d = field(src[i].x * 1000, w[i], half)
                if any(d):
                    moved[i] = Vector(d) / 1000.0 * fit.s
            deltas = dict(moved)
            for i, b, k in skin:
                if b in moved:
                    deltas[i] = moved[b] * k
            out[sid + suffix] = deltas
    return out


def rim_z(x_out, upper):
    """The rim's height over the eye landmark at `x_out` (source metres, toward the outer
    corner), on the upper or the lower lid."""
    t = max(0.0, min(1.0, (x_out - RIM["inner"]) / (RIM["outer"] - RIM["inner"])))
    zc = RIM["inner_z"] + (RIM["outer_z"] - RIM["inner_z"]) * t
    peak = RIM["up_peak"] if upper else RIM["down_peak"]
    s = t ** (math.log(0.5) / math.log(peak))
    h = math.sin(math.pi * s) ** 0.8
    return zc + (RIM["up"] if upper else -RIM["down"]) * h


def _rim_points(n=120):
    """The rim, closed: (x_out, z) round it, upper lid inner to outer, then the lower back."""
    xs = [RIM["inner"] + (RIM["outer"] - RIM["inner"]) * i / n for i in range(n + 1)]
    return [(x, rim_z(x, True)) for x in xs] + [(x, rim_z(x, False)) for x in reversed(xs[1:-1])]


def _walk(e0, v0):
    """The edges of the edge loop through `e0`, walking away from its vertex `v0` until a pole."""
    out = []
    e, v = e0, e0.other_vert(v0)
    for _ in range(64):
        if len(v.link_edges) != 4:
            break
        faces = set(e.link_faces)
        nxt = [x for x in v.link_edges if x is not e and not (set(x.link_faces) & faces)]
        if len(nxt) != 1 or nxt[0] is e0:
            break
        e = nxt[0]
        out.append(e)
        v = e.other_vert(v)
    return out


def _lid_faces(src):
    """The closed lids' faces round each eye: those within one edge of the seam where the lids
    meet (the edge loop across the eye's middle), so the opening's rim is the source's first eye
    loop, whole."""
    out = []
    for side in (1, -1):
        c = Vector((side * EYE.x, EYE.y, EYE.z))
        near = [e for e in src.edges
                if all((v.co - c).length < 0.02 and v.co.y < EYE.y + 0.01 for v in e.verts)]

        def across(e):
            d = e.verts[1].co - e.verts[0].co
            return abs(d.x) > 2 * abs(d.z)

        e0 = min((e for e in near if across(e)), key=lambda e: ((e.verts[0].co + e.verts[1].co) / 2 - c).length)
        seam = set(e0.verts)
        for v0 in e0.verts:
            for e in _walk(e0, v0):
                seam |= set(e.verts)
        within = seam | {e.other_vert(v) for v in seam for e in v.link_edges}
        out += [f for f in src.faces if all(v in within for v in f.verts)]
    return out


def _shape_rims(src):
    """Each opening's rim set on `RIM`: every rim vertex moves along its direction from the
    opening's middle onto the rim, and forward if it would sit inside `RIM_CLEAR` of the
    eyeball's centre."""
    pts = _rim_points()
    mx = (RIM["inner"] + RIM["outer"]) / 2
    mz = (RIM["inner_z"] + RIM["outer_z"]) / 2
    half = ((RIM["outer"] - RIM["inner"]) / 2, (RIM["up"] + RIM["down"]) / 2)

    def angle(x, z):
        return math.atan2((z - mz) / half[1], (x - mx) / half[0])

    marks = [(angle(x, z), x, z) for x, z in pts]
    for side in (1, -1):
        c = Vector((side * EYE.x, EYE.y, EYE.z))
        ball = Vector((side * EYE.x, EYE.y + EYEBALL_BACK, EYE.z))
        rim = [v for v in src.verts if (v.co - c).length < 0.02 and any(e.is_boundary for e in v.link_edges)]
        for v in rim:
            a = angle(side * (v.co.x - c.x), v.co.z - c.z)
            _, x, z = min(marks, key=lambda m: abs(math.remainder(m[0] - a, math.tau)))
            v.co.x, v.co.z = c.x + side * x, c.z + z
            d2 = (v.co.x - ball.x) ** 2 + (v.co.z - ball.z) ** 2
            v.co.y = min(v.co.y, ball.y - math.sqrt(max(0.0, RIM_CLEAR ** 2 - d2)))


def _crease_loops(src):
    """A loop round each lid opening, between its rim and the next ring out (22 §2.3): the source
    has one ring between the lid's edge and the brow, too few for a crease, a fold or a hood."""
    for side in (1, -1):
        c = Vector((side * EYE.x, EYE.y, EYE.z))
        rim = {v for v in src.verts if (v.co - c).length < 0.02 and any(e.is_boundary for e in v.link_edges)}
        spokes = {e for v in rim for e in v.link_edges if not e.is_boundary and e.other_vert(v) not in rim}
        if spokes:
            bmesh.ops.subdivide_edges(src, edges=list(spokes), cuts=1, use_grid_fill=True)


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
    upper = which == "upper"
    sign = 1.0 if upper else -1.0
    side = 1 if c.x > 0 else -1
    cols, rows = 10, 5
    # Far enough that a closed upper lid still reaches behind the rim's top.
    reach = math.radians(80 if upper else 50)
    ends = [math.asin(max(-0.98, min(0.98, side * x / rad))) for x in (RIM["inner"] - 0.0025, RIM["outer"] + 0.0025)]
    lo, hi = min(ends), max(ends)

    def margin(az):
        # The rim's height at this azimuth (`rim_z`). The upper lid rests over the top of the
        # iris, as an eye at ease does (`REST_DROP`, by the lid's height there); the lower lid
        # just inside its rim.
        x_out = side * rad * math.sin(az)
        z = rim_z(x_out, upper)
        el = math.asin(max(-0.95, min(0.95, z / rad)))
        if upper:
            t =max(0.0, min(1.0, (x_out - RIM["inner"]) / (RIM["outer"] - RIM["inner"])))
            zc = RIM["inner_z"] + (RIM["outer_z"] - RIM["inner_z"]) * t
            return el - REST_DROP * max(0.0, z - zc) / RIM["up"]
        # The lower lid's margin as an angle below the middle (the rows go down from it).
        return -el + math.radians(1)

    grid = []
    for i in range(cols + 1):
        az = lo + (hi - lo) * i / cols
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


# --- Lid shapes (22 §2.3) ---------------------------------------------------------------------
# Each is a field over the skin round an eye opening, in the source's millimetres: what moves a
# point at (u, w), its place against the opening's ellipse (`LID`; u toward the outer corner, w
# up, 1 on the rim), returned as (out, forward-back, up). The skin slides over the eyeball as it
# moves, and the lid shells turn about the ball to keep their margin under the rim. A slider's
# `+` and `-` are two fields, or one (`~`) whose other end is its mirror.

def _sm(t):
    t = max(0.0, min(1.0, t))
    return t * t * (3 - 2 * t)


def _bump(x, at, width):
    """1 at `at`, 0 from `width` away."""
    return 1.0 - _sm(abs(x - at) / width)


def _rim_h(u):
    """The opening's half-height at u (1 in the middle, 0 at the corners)."""
    return math.sqrt(max(0.0, 1.0 - u * u))


def _fall(rho, out):
    """Outside the opening, a rim's move fading to nothing by `out`."""
    return 1.0 - _sm((rho - 1.0) / (out - 1.0))


def _opening(up, down):
    """The opening's height: its upper half-height changes by `up` mm and its lower by `down`.
    Inside the opening (the socket's wall) a point moves with its share of the rim's move."""
    def field(u, w, rho):
        a, out = (up, 2.4) if w >= 0 else (down, 2.2)
        dz = a * w if rho <= 1 else a * math.copysign(_rim_h(u), w) * _fall(rho, out)
        return (0.0, 0.0, dz)
    return field


def _inner(u, w, rho):
    """The inner corner raised."""
    d = math.hypot((u + 1.0) * LID[0] * 1000, w * LID[1] * 1000)
    return (0.0, 0.0, 1.6 * (1.0 - _sm(d / 7.0)))


def _fold(u, w, rho):
    """An epicanthic fold: the upper lid's skin comes down and across the inner corner, a soft
    ridge running from the lid down into it."""
    s = _sm((-u - 0.15) / 0.85)
    if s == 0:
        return (0.0, 0.0, 0.0)
    if w >= 0:
        f = rho if rho <= 1 else _fall(rho, 2.2)
        return (3.0 * s * f, -1.0 * s * _bump(rho, 1.35, 0.6), -1.2 * s * f)
    f = rho if rho <= 1 else _fall(rho, 1.8)
    return (1.8 * s * f, 0.0, -1.0 * s * f * (1.0 - _sm(-w / 0.5)))


def _across(u, edge):
    return 1.0 - _sm((abs(u) - edge) / 0.6)


def _crease_deep(u, w, rho):
    """A deep double crease: the loop over the lid sinks into a fold line."""
    if w <= 0:
        return (0.0, 0.0, 0.0)
    k = _bump(rho, 1.55, 0.45) * _across(u, 0.55)
    return (0.0, 1.7 * k, 0.4 * k)


def _monolid(u, w, rho):
    """A monolid: no crease; the skin above comes down full and smooth to the lash line."""
    if w <= 0:
        return (0.0, 0.0, 0.0)
    a = _across(u, 0.55)
    dy = -1.3 * _bump(rho, 1.5, 0.55) - 0.4 * _bump(rho, 1.0, 0.4)
    return (0.0, dy * a, -1.0 * _bump(rho, 1.9, 0.7) * a)


def _hood(u, w, rho):
    """Hooded: the skin under the brow hangs over the crease, more toward the outer corner."""
    if w <= 0:
        return (0.0, 0.0, 0.0)
    o = (0.4 + 0.6 * _sm((u + 0.3) / 1.1)) * _across(u, 0.9)
    return (0.0, -1.1 * _bump(rho, 1.8, 0.7) * o, -2.8 * _bump(rho, 2.0, 0.9) * o)


# (slider, description, [(suffix, field)]): the creator's Lids sliders.
LID_SHAPES = [
    ("lids_open", "Wide-open eyes or narrow ones", [("+", _opening(1.8, 0.8)), ("-", _opening(-2.6, -1.0))]),
    ("lids_inner", "The inner corners high or low", [("~", _inner)]),
    ("lids_fold", "An epicanthic fold over the inner corner", [("+", _fold)]),
    ("lids_crease", "A deep crease over the lid, or none (a monolid)", [("+", _crease_deep), ("-", _monolid)]),
    ("lids_hood", "Hooded lids: the brow's skin over the crease", [("+", _hood)]),
]


def lid_keys(armature, body, fit):
    """The lid shapes (`LID_SHAPES`) on the built `body`, as {slider + suffix: {vertex: delta}}
    in armature space. The skin round each opening moves by its field and slides over the
    eyeball; the lid shells (on the `lid_*` bones) turn about it so their margins follow the
    rim; the eyeballs stay."""
    me = body.data
    names = {g.index: g.name for g in body.vertex_groups}

    def source(co):
        return JOINT + (Vector(co) - fit.joint) / fit.s

    skin, shells = [], []
    for v in me.vertices:
        top = max(((g.weight, names[g.group]) for g in v.groups), default=(0.0, ""))[1]
        if top.startswith("eye_look_"):
            continue
        p = source(v.co)
        side = 1 if p.x > 0 else -1
        if top.startswith("lid_"):
            shells.append((v.index, p, side, top.startswith("lid_upper_")))
            continue
        dx, dz = side * (p.x - side * EYE.x), p.z - EYE.z
        if abs(dx) < 0.03 and abs(dz) < 0.03 and EYE.y - 0.03 < p.y < EYE.y + 0.015:
            skin.append((v.index, p, side, dx / LID[0], dz / LID[1]))
    out = {}
    for sid, _desc, ends in LID_SHAPES:
        for suffix, field in ends:
            deltas = {}
            for i, p, side, u, w in skin:
                rho = math.hypot(u, w)
                if rho > 3.2:
                    continue
                d_out, d_y, d_z = (x / 1000.0 for x in field(u, w, rho))
                if d_out == 0 and d_y == 0 and d_z == 0:
                    continue
                c = Vector((side * EYE.x, EYE.y + EYEBALL_BACK, EYE.z))
                moved = p + Vector((side * d_out, 0.0, d_z))
                # Sliding over the eyeball: as far from its centre as before, near the opening.
                slid = c + (moved - c) * ((p - c).length / max(1e-9, (moved - c).length))
                q = moved.lerp(slid, 1.0 - _sm((rho - 1.2) / 1.0)) + Vector((0.0, d_y, 0.0))
                deltas[i] = (q - p) * fit.s
            for i, p, side, upper in shells:
                c = Vector((side * EYE.x, EYE.y + EYEBALL_BACK, EYE.z))
                rel = p - c
                rad = rel.length
                az = math.atan2(rel.x, -rel.y)
                el = math.asin(max(-1.0, min(1.0, rel.z / rad)))
                # The rim's move over this column of the shell.
                u = max(-1.0, min(1.0, side * rad * math.sin(az) / LID[0]))
                w = _rim_h(u) if upper else -_rim_h(u)
                d_out, _d_y, d_z = (x / 1000.0 for x in field(u, w, 1.0))
                if d_out == 0 and d_z == 0:
                    continue
                az2, el2 = az + side * d_out / rad, el + d_z / rad
                q = c + Vector((math.sin(az2) * math.cos(el2), -math.cos(az2) * math.cos(el2), math.sin(el2))) * rad
                deltas[i] = (q - p) * fit.s
            out[sid + suffix] = deltas
    return out


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
