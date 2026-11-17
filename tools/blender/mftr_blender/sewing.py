# SPDX-License-Identifier: AGPL-3.0-or-later
"""Sewn garments (22 §5 M1, D77): a garment is its sewing pattern, flat panels and the seams that
join them, simulated onto the reference body with Blender's cloth (its sewing springs pull each
seam shut while the cloth collides with the body and falls under gravity). Its folds and drape
are physics, not shapes made by code. The result is welded at its seams, trimmed at its edges,
reduced to its budget and given thickness, and saved as a wardrobe item (`wardrobe`).

A pattern (`Garment`) is data:

- **panels**: each a closed outline of named points in metres, drawn flat in the plane the panel
  hangs in before the sim (front panels a little in front of the body, back panels behind it);
- **seams**: pairs of outline stretches (from a named point to another, along the outline),
  sewn together, the first's start to the second's start;
- the hem trim's width, the material of the body and of the trim, colors, the triangle budget.

Run from an item's `build.py` (`build`).
"""

import math
import os

import bmesh
import bpy
from mathutils import Vector
from mathutils.geometry import delaunay_2d_cdt


class Panel:
    """A flat panel: `outline`, [(name or None, x, z)] in order round it (counter-clockwise seen
    from the front), standing in the plane y = `y`, facing -y (`front`) or +y. `color`: its own
    cloth color (a paint region of its own), else the garment's."""

    def __init__(self, name, outline, y, front=True, turn=None, color=None):
        self.name, self.outline, self.y, self.front = name, outline, y, front
        self.color = color
        # Optional: (pivot (x, z), angle in degrees about the y axis), so a sleeve hangs along
        # the arm rather than in the body's plane.
        self.turn = turn


class Garment:
    def __init__(self, panels, seams, spacing=0.035, trim=0.03, budget=900, thickness=0.004,
                 cloth=(0.46, 0.42, 0.34), trim_color=(0.30, 0.20, 0.12), trim_slot="accent", frames=120,
                 lining=0.07, mirror="-x", over=()):
        self.panels, self.seams = panels, seams
        # Items it is worn over (their folders, as `art/wardrobe/<slot>/<name>`): once
        # simulated, it is laid over them (`_lay_over`): a skirt's tails over the skirt's flare,
        # which the forge's layering (`wardrobe::MAX_PUSH`) wouldn't clear. Not colliders in the
        # sim: the tails caught on the skirt there and bunched at the waist.
        self.over = over
        self.spacing, self.trim, self.budget, self.thickness = spacing, trim, budget, thickness
        self.cloth, self.trim_color, self.trim_slot, self.frames = cloth, trim_color, trim_slot, frames
        self.lining = lining
        # A symmetric garment keeps one simulated half, mirrored (`-x` or `+x`; None to keep the
        # sim as it is): the sim drapes the two sides a little differently.
        self.mirror = mirror


def _resample(points, step):
    """A closed outline resampled at about `step`, keeping every named point exactly; returns
    [(name or None, Vector2)]."""
    out = []
    n = len(points)
    for i in range(n):
        name, x0, z0 = points[i]
        _n1, x1, z1 = points[(i + 1) % n]
        a, b = Vector((x0, z0)), Vector((x1, z1))
        k = max(1, round((b - a).length / step))
        out.append((name, a))
        for j in range(1, k):
            out.append((None, a.lerp(b, j / k)))
    return out


def _panel_mesh(bm, panel, step):
    """Mesh `panel` into `bm` (a constrained Delaunay of its outline and an inner grid). Returns
    its outline's vertices in order and {name: index into them}."""
    ring = _resample(panel.outline, step)
    pts = [p for _, p in ring]
    xs, zs = [p.x for p in pts], [p.y for p in pts]
    inner = []
    x = min(xs) + step
    row = 0
    while x < max(xs):
        z = min(zs) + step * (0.5 if row % 2 else 1.0)
        while z < max(zs):
            q = Vector((x, z))
            if _inside(q, pts) and min((q - p).length for p in pts) > step * 0.6:
                inner.append(q)
            z += step
        x += step * 0.866
        row += 1
    verts = pts + inner
    edges = [(i, (i + 1) % len(pts)) for i in range(len(pts))]
    vo, _eo, fo, _o1, _o2, _o3 = delaunay_2d_cdt(verts, edges, [], 2, 1e-6)
    made = []
    if panel.turn:
        (px, pz), deg = panel.turn
        a = math.radians(deg)
        ca, sa = math.cos(a), math.sin(a)
    for p in vo:
        x, z = p.x, p.y
        if panel.turn:
            dx, dz = x - px, z - pz
            x, z = px + dx * ca - dz * sa, pz + dx * sa + dz * ca
        made.append(bm.verts.new((x, panel.y, z)))
    index = {}
    for i, p in enumerate(vo):
        index[(round(p.x, 6), round(p.y, 6))] = made[i]
    outline = [index[(round(p.x, 6), round(p.y, 6))] for p in pts]
    for f in fo:
        vs = [made[i] for i in f]
        if not panel.front:
            vs.reverse()
        try:
            bm.faces.new(vs)
        except ValueError:
            pass
    names = {name: i for i, (name, _) in enumerate(ring) if name}
    return outline, names


def _inside(q, poly):
    c = False
    n = len(poly)
    for i in range(n):
        a, b = poly[i], poly[(i + 1) % n]
        if (a.y > q.y) != (b.y > q.y):
            x = a.x + (q.y - a.y) * (b.x - a.x) / (b.y - a.y)
            if q.x < x:
                c = not c
    return c


def _stretch(outline, names, a, b):
    """The outline's vertices from point `a` to point `b`, the shorter way round it."""
    i, j = names[a], names[b]
    n = len(outline)
    step = 1 if (j - i) % n <= n // 2 else -1
    out = [outline[i]]
    while i != j:
        i = (i + step) % n
        out.append(outline[i])
    return out


def mirrored(outline):
    """An outline mirrored across x = 0, still counter-clockwise."""
    return [(name, -x, z) for name, x, z in reversed(outline)]


def _pairs(sa, sb):
    """Seam stretches `sa` and `sb` matched by their share of length along each."""
    def params(s):
        d = [0.0]
        for p, q in zip(s, s[1:]):
            d.append(d[-1] + (q.co - p.co).length)
        return [x / (d[-1] or 1.0) for x in d]
    pa, pb = params(sa), params(sb)
    out = []
    for v, t in zip(sa, pa):
        j = min(range(len(sb)), key=lambda k: abs(pb[k] - t))
        out.append((v, sb[j]))
    for v, t in zip(sb, pb):
        i = min(range(len(sa)), key=lambda k: abs(pa[k] - t))
        out.append((sa[i], v))
    return list({(id(a), id(b)): (a, b) for a, b in out}.values())


def simulate(garment, body):
    """The garment simulated onto `body` (a mesh object, the collider): returns a new mesh
    object, welded at its seams, its sewing edges gone."""
    bm = bmesh.new()
    outlines = {}
    # Each face remembers its panel (`finish` colors it by the panel's own color).
    which = bm.faces.layers.int.new("panel")
    for k, p in enumerate(garment.panels):
        before = set(bm.faces)
        outlines[p.name] = _panel_mesh(bm, p, garment.spacing)
        for f in bm.faces:
            if f not in before:
                f[which] = k
    sewn = []
    for (pa, a0, a1), (pb, b0, b1) in garment.seams:
        sa = _stretch(*outlines[pa], a0, a1)
        sb = _stretch(*outlines[pb], b0, b1)
        for u, v in _pairs(sa, sb):
            if u is not v and not bm.edges.get((u, v)):
                sewn.append(bm.edges.new((u, v)))
    bm.verts.index_update()
    stitches = [(e.verts[0].index, e.verts[1].index) for e in sewn]
    me = bpy.data.meshes.new("garment")
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new("garment", me)
    bpy.context.scene.collection.objects.link(obj)
    # The body collides.
    col = body.modifiers.new("Collision", "COLLISION")
    body.collision.thickness_outer = 0.004
    body.collision.cloth_friction = 5.0
    cloth = obj.modifiers.new("Cloth", "CLOTH")
    s = cloth.settings
    s.quality = 8
    s.mass = 0.25
    s.air_damping = 2.0
    s.tension_stiffness = s.compression_stiffness = 12.0
    s.shear_stiffness = 6.0
    s.bending_stiffness = 0.4
    s.use_sewing_springs = True
    s.sewing_force_max = 6.0
    s.shrink_min = 0.0
    c = cloth.collision_settings
    c.distance_min = 0.004
    c.use_self_collision = False
    cache = cloth.point_cache
    cache.frame_start, cache.frame_end = 1, garment.frames
    scene = bpy.context.scene
    scene.frame_start, scene.frame_end = 1, garment.frames
    # The seams close first, then the cloth falls.
    s.effector_weights.gravity = 0.0
    s.effector_weights.keyframe_insert("gravity", frame=1)
    s.effector_weights.keyframe_insert("gravity", frame=garment.frames * 3 // 5)
    s.effector_weights.gravity = 1.0
    s.effector_weights.keyframe_insert("gravity", frame=garment.frames * 3 // 5 + 15)
    for f in range(1, garment.frames + 1):
        scene.frame_set(f)
    dg = bpy.context.evaluated_depsgraph_get()
    out = bpy.data.meshes.new_from_object(obj.evaluated_get(dg))
    bpy.data.objects.remove(obj)
    body.modifiers.remove(col)
    res = bpy.data.objects.new("item", out)
    bpy.context.scene.collection.objects.link(res)
    # Weld the seams: every stitch's ends merged at their mean, wherever the sim left them (a
    # seam held open by the body still closes); then what the sewing left.
    bm = bmesh.new()
    bm.from_mesh(out)
    bm.verts.ensure_lookup_table()
    root = list(range(len(bm.verts)))

    def find(a):
        while root[a] != a:
            root[a] = root[root[a]]
            a = root[a]
        return a

    for a, b in stitches:
        ra, rb = find(a), find(b)
        if ra != rb:
            root[rb] = ra
    groups = {}
    for i in range(len(bm.verts)):
        groups.setdefault(find(i), []).append(i)
    target = {}
    seam = set()
    for r, members in groups.items():
        if len(members) < 2:
            continue
        mid = sum((bm.verts[i].co for i in members), Vector()) / len(members)
        for i in members:
            bm.verts[i].co = mid
            seam.add(bm.verts[i])
            if i != r:
                target[bm.verts[i]] = bm.verts[r]
    # What the sewing left: near the seams only. A fold elsewhere that lies on itself (a cape
    # bunched on the back) stays two layers, not merged into a tear.
    near = {w for v in seam for e in v.link_edges for w in e.verts} | seam
    bmesh.ops.weld_verts(bm, targetmap=target)
    bmesh.ops.remove_doubles(bm, verts=[v for v in near if v.is_valid], dist=garment.spacing * 0.2)
    _fill_holes(bm)
    if garment.mirror:
        _mirror(bm, garment.mirror)
        _fill_holes(bm)
    loose = [e for e in bm.edges if not e.link_faces]
    bmesh.ops.delete(bm, geom=loose, context="EDGES")
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)
    bm.to_mesh(out)
    bm.free()
    return res


def _fill_holes(bm, longest=0.15):
    """Fill the small holes where seams meet (a crotch, a mirror line's slit): open loops shorter
    than `longest` round. A garment's openings (a neckline, a hem) are far longer."""
    left = {e for e in bm.edges if e.is_boundary}
    small = []
    while left:
        e = left.pop()
        loop, stack = {e}, [e]
        while stack:
            x = stack.pop()
            for v in x.verts:
                for y in v.link_edges:
                    if y in left:
                        left.discard(y)
                        loop.add(y)
                        stack.append(y)
        if sum(x.calc_length() for x in loop) < longest:
            small.extend(loop)
    if small:
        bmesh.ops.holes_fill(bm, edges=small, sides=0)


def _mirror(bm, keep):
    """Keep the `keep` half (`-x` or `+x`) of the garment and mirror it onto the other, welded
    down the middle."""
    from mathutils import Matrix
    sign = -1.0 if keep == "-x" else 1.0
    geom = bm.verts[:] + bm.edges[:] + bm.faces[:]
    bmesh.ops.bisect_plane(bm, geom=geom, dist=1e-6, plane_co=(0.0, 0.0, 0.0), plane_no=(-sign, 0.0, 0.0),
                           clear_outer=True)
    for v in bm.verts:
        if abs(v.co.x) < 1e-5:
            v.co.x = 0.0
    half = bm.verts[:] + bm.edges[:] + bm.faces[:]
    bmesh.ops.mirror(bm, geom=half, matrix=Matrix.Identity(4), merge_dist=1e-4, axis="X")
    bmesh.ops.recalc_face_normals(bm, faces=bm.faces)


def finish(obj, garment):
    """Trim the garment's edges, reduce it to its budget, give it thickness and its colors and
    materials (`cloth` and the trim's slot)."""
    from . import mesh
    # Reduce to the budget only if the sim's mesh is over it (its spacing sets the facets).
    tris = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    if tris > garment.budget * 0.8:
        dec = obj.modifiers.new("Decimate", "DECIMATE")
        dec.ratio = garment.budget * 0.8 / tris
        _apply(obj, dec)
    # Trims: a band along the open edges (the hems, the neckline, the armholes).
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    trim = _cut_trims(bm, garment.trim)
    # The real openings, before the trims are split off (their line isn't one).
    openings = {_edge_key(e) for e in bm.edges if e.is_boundary}
    slots = mesh.MATERIAL_SLOTS
    for f in bm.faces:
        f.material_index = slots.index(garment.trim_slot) if f in trim else slots.index("cloth")
        f.smooth = False
    seam = [e for e in bm.edges if len(e.link_faces) == 2 and (e.link_faces[0] in trim) != (e.link_faces[1] in trim)]
    bmesh.ops.split_edges(bm, edges=seam)
    col = bm.verts.layers.float_color.new("Col")
    for f in bm.faces:
        c = garment.trim_color if f in trim else garment.cloth
        for v in f.verts:
            v[col] = (*c, 1.0)
    # The slots first: a face's material index past the mesh's materials is reset to 0 (every
    # garment came out on `skin` until this was in this order).
    me = obj.data
    me.materials.clear()
    for s in slots:
        me.materials.append(mesh.material(s))
    bm.to_mesh(me)
    bm.free()
    _line(obj, garment.lining, garment.thickness, openings)
    _panel_colors(obj, garment)
    me = obj.data
    me.color_attributes.active_color = me.color_attributes["Col"]
    for p in me.polygons:
        p.use_smooth = False
    return sum(len(p.vertices) - 2 for p in me.polygons)


def _cut_trims(bm, width):
    """The faces of a trim `width` wide along `bm`'s open edges, cut out along its line: each
    vertex's distance to the nearest open edge, and the faces split where it is `width` (between
    two vertices on either side of it, where it crosses), so the trim's edge runs smooth beside
    the hem rather than as a sawtooth of whole faces."""
    from mathutils.kdtree import KDTree
    bmesh.ops.triangulate(bm, faces=bm.faces[:])
    samples = []
    for e in bm.edges:
        if e.is_boundary:
            a, b = e.verts[0].co, e.verts[1].co
            n = max(1, int((b - a).length / 0.003))
            samples += [a.lerp(b, k / n) for k in range(n + 1)]
    if not samples:
        return set()
    kd = KDTree(len(samples))
    for i, c in enumerate(samples):
        kd.insert(c, i)
    kd.balance()
    dist = {v: kd.find(v.co)[2] for v in bm.verts}
    for v, d in dist.items():
        if abs(d - width) < 1e-5:
            dist[v] = width + 1e-5
    cut = set()
    for e in list(bm.edges):
        a, b = e.verts
        da, db = dist[a], dist[b]
        if (da - width) * (db - width) < 0:
            _e, v = bmesh.utils.edge_split(e, a, (width - da) / (db - da))
            dist[v] = width
            cut.add(v)
    for f in list(bm.faces):
        on = [v for v in f.verts if v in cut]
        if len(on) == 2 and not any(e.other_vert(on[0]) is on[1] for e in on[0].link_edges):
            bmesh.ops.connect_verts(bm, verts=on)
    return {f for f in bm.faces if sum(dist[v] for v in f.verts) / len(f.verts) < width}


def _panel_colors(obj, garment):
    """The panels with colors of their own (`Panel.color`) in them, outside the trims: split from
    their neighbours where they meet (vertex colors are per vertex), each vertex keeping its
    occlusion. After the lining, so a seam between two colors isn't lined as an opening."""
    if not any(p.color for p in garment.panels):
        return
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    which = bm.faces.layers.int.get("panel")
    col = bm.verts.layers.float_color.get("Col")
    if which is None or col is None:
        bm.free()
        return

    def own(f):
        k = f[which]
        return garment.panels[k].color if 0 <= k < len(garment.panels) else None

    trim_c = tuple(round(c, 4) for c in garment.trim_color)
    cut = [e for e in bm.edges if len(e.link_faces) == 2 and own(e.link_faces[0]) != own(e.link_faces[1])]
    bmesh.ops.split_edges(bm, edges=cut)
    for f in bm.faces:
        c = own(f)
        if c is None:
            continue
        for v in f.verts:
            r, g, b_, a = v[col]
            if tuple(round(x, 4) for x in (r, g, b_)) != trim_c:
                v[col] = (*c, a)
    bm.to_mesh(obj.data)
    bm.free()


# A lining vertex's occlusion alpha: the inside of a garment is in its shade.
LINING = 0.55


def _edge_key(e):
    """An edge by where its ends are (the same after the mesh is split or rebuilt)."""
    return frozenset(tuple(round(c, 5) for c in v.co) for v in e.verts)


def _line(obj, depth, thickness, openings=None):
    """Thickness where it shows: an inner layer for the faces within `depth` of the open edges
    (a look into a neckline or a hem sees cloth, not the void), set `thickness` in and facing in,
    joined to the outer layer at the edge by a rim. Deeper in, the body fills the garment.
    `openings` (`_edge_key`s): which open edges are the garment's own; a trim split off along
    its line, for its color, isn't one."""
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    bm.normal_update()
    edge = [e for e in bm.edges if e.is_boundary and (openings is None or _edge_key(e) in openings)]
    from mathutils.kdtree import KDTree
    pts = [v.co.copy() for e in edge for v in e.verts]
    kd = KDTree(len(pts))
    for i, c in enumerate(pts):
        kd.insert(c, i)
    kd.balance()
    near = [f for f in bm.faces if kd.find(f.calc_center_median())[2] < depth]
    col = bm.verts.layers.float_color.get("Col")
    which = bm.faces.layers.int.get("panel")
    inner = {}
    for f in near:
        for v in f.verts:
            if v not in inner:
                nv = bm.verts.new(v.co - v.normal * thickness)
                if col is not None:
                    # The inside darker (the occlusion alpha), which also marks it as the lining
                    # (`mftr_forge::wardrobe::LINING`).
                    c = v[col]
                    nv[col] = (c[0], c[1], c[2], LINING)
                inner[v] = nv
    for f in near:
        try:
            nf = bm.faces.new([inner[v] for v in reversed(f.verts)])
            nf.material_index = f.material_index
            nf.smooth = False
            if which is not None:
                nf[which] = f[which]
        except ValueError:
            pass
    for e in edge:
        a, b = e.verts
        if a in inner and b in inner:
            f0 = e.link_faces[0]
            # The rim faces outward, along the face's edge away from it.
            try:
                rf = bm.faces.new([b, a, inner[a], inner[b]])
                rf.material_index = f0.material_index
                rf.smooth = False
                if which is not None:
                    rf[which] = f0[which]
            except ValueError:
                pass
    bm.normal_update()
    bm.to_mesh(obj.data)
    bm.free()


def _apply(obj, mod):
    for o in list(bpy.context.selected_objects):
        o.select_set(False)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.modifier_apply(modifier=mod.name)


# How far a garment laid over another stands clear of it (m).
OVER_GAP = 0.008


def _lay_over(obj, under, gap=OVER_GAP):
    """`obj`'s vertices out of `under` (another item, as the reference body wears it): each
    along its level line out from the body's upright middle, to `gap` past `under`'s outside
    there, where it isn't already. Rays from far out find that outside."""
    from mathutils.bvhtree import BVHTree
    me = under.data
    tree = BVHTree.FromPolygons([under.matrix_world @ v.co for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    moved = 0
    for v in obj.data.vertices:
        d = Vector((v.co.x, v.co.y - 0.02, 0.0))
        if d.length < 1e-6:
            continue
        d.normalize()
        axis = Vector((0.0, 0.02, v.co.z))
        hit, _n, _i, _dist = tree.ray_cast(axis + d * 1.0, -d)
        if hit is None:
            continue
        r_out = (hit - axis).length + gap
        if (v.co - axis).length < r_out:
            v.co = axis + d * r_out
            moved += 1
    return moved


def build(garment, out_path):
    """Simulate `garment` on the reference body and save it as an item's `item.blend`."""
    from . import cc0_body
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 24
    body = cc0_body.load("male")
    obj = simulate(garment, body)
    if garment.over:
        from . import wardrobe
        for d in garment.over:
            under = wardrobe.load(os.path.abspath(d))
            _lay_over(obj, under)
            bpy.data.objects.remove(under)
    tris = finish(obj, garment)
    bpy.data.objects.remove(body)
    for o in list(bpy.data.objects):
        if o is not obj:
            bpy.data.objects.remove(o)
    # Named `item` (what it was sewn over had the name while it was loaded).
    obj.name = "item"
    bpy.ops.outliner.orphans_purge(do_recursive=True)
    bpy.ops.wm.save_as_mainfile(filepath=out_path, compress=True)
    print(f"item: {tris} triangles -> {out_path}")
    return tris
