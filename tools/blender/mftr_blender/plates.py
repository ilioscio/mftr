# SPDX-License-Identifier: AGPL-3.0-or-later
"""Plates (22 §4): hard wardrobe items modelled round the reference body, for what is neither
cloth (`sewing`) nor a skin-tight shell (`shell`): a breastplate, a gorget, a boot's shaft,
pauldrons, knee cops.

- **Lathed** (`lathe`): rows of rays out from a vertical axis through the body (the torso's
  middle, a leg's) find the skin; each row is a ring of points stood off it, the distances
  smoothed across the grid (a plate, not the muscles under it) but never inside the skin. Its
  edges and bands are the grid's rows and columns: straight lines, not the low-poly reference
  body's facets.
- **Tubes** (`tube`): the same round a path through the body that needn't be upright (a
  boot's foot, heel to toe), capped at its ends.
- **Domed** (`dome`, `strip`): an ellipsoid's cap round an axis (a pauldron over the shoulder, a
  cop over the knee), and strips of it further down (a pauldron's lames).

Faces are collected with their colors (`faces`: {face: (color, slot)}); `finish` splits where
colors meet, assigns the material slots, turns the faces out from `inside` and gives them a
thickness; `save` joins the pieces into the item's `item` object and saves its `item.blend`.
"""

import math
import os

import bmesh
import bpy
from mathutils import Vector


def ray_centre(tree, z, guess, rays=16, rounds=3):
    """The middle of the body's section at height `z` round `guess` (x, y): the mean of where
    rays out from it meet the skin, found again from there."""
    c = Vector((guess[0], guess[1], z))
    for _ in range(rounds):
        hits = []
        for k in range(rays):
            a = 2 * math.pi * k / rays
            hit, _n, _i, _d = tree.ray_cast(c, Vector((math.sin(a), -math.cos(a), 0.0)))
            if hit is not None:
                hits.append(hit)
        if not hits:
            break
        m = sum(hits, Vector()) / len(hits)
        c = Vector((m.x, m.y, z))
    return c


def _smooth(grid, loop, rounds):
    """A grid of rays' [origin, direction, distance to the skin]: the distances smoothed toward
    their neighbours' (`loop`: the columns go all round), never inside the skin."""
    rows, n = len(grid), len(grid[0])
    skin = [[g[2] for g in row] for row in grid]
    for _ in range(rounds):
        new = []
        for r in range(rows):
            out = []
            for c in range(n):
                nb = []
                for dr, dc in ((1, 0), (-1, 0), (0, 1), (0, -1)):
                    rr, cc = r + dr, c + dc
                    if loop:
                        cc %= n
                    if 0 <= rr < rows and 0 <= cc < n:
                        nb.append(grid[rr][cc][2])
                out.append(0.5 * grid[r][c][2] + 0.5 * sum(nb) / len(nb))
            new.append(out)
        for r in range(rows):
            for c in range(n):
                grid[r][c][2] = max(new[r][c], skin[r][c])


def lathe(bm, faces, tree, rows, angles, z_at, centre, lift, color, loop=True, smooth=6):
    """A plate stood off the skin round a vertical axis: `rows` + 1 rings of points, ring r at
    height `z_at(r, angle)`, one point per angle in `angles` (degrees round from the front,
    -y, toward +x), each on a ray out from `centre(z)` (a Vector) to the skin and `lift(r, c)`
    beyond it. `color(r, c)` is the quad's (color, slot) from ring r to r + 1. `loop`: the
    angles go all round. A quad whose color is None is left out. Returns the grid of vertices."""
    grid = []
    for r in range(rows + 1):
        row = []
        for a in angles:
            z = z_at(r, a)
            d = Vector((math.sin(math.radians(a)), -math.cos(math.radians(a)), 0.0))
            o = centre(z)
            o = Vector((o.x, o.y, z))
            hit, _n, _i, _dist = tree.ray_cast(o, d)
            row.append([o, d, (hit - o).length if hit is not None else 0.1])
        grid.append(row)
    n = len(angles)
    _smooth(grid, loop, smooth)
    verts = [[bm.verts.new(o + d * (dist + lift(r, c))) for c, (o, d, dist) in enumerate(row)]
             for r, row in enumerate(grid)]
    for r in range(rows):
        for c in range(n if loop else n - 1):
            c2 = (c + 1) % n
            col = color(r, c)
            if col is None:
                continue
            f = bm.faces.new((verts[r][c], verts[r][c2], verts[r + 1][c2], verts[r + 1][c]))
            faces[f] = col
    return verts


def tube(bm, faces, tree, stations, up, sides, lift, color, smooth=4, caps=(True, True), reach=0.08):
    """A shell round a path through the body (a foot from the heel to the toes): at each of
    `stations` (points inside the body, in order), a ring of rays out across the path, `up` the
    first ray's direction (angle 0; the rest turn toward the path's left); each point `lift(i, c)`
    beyond the skin, the distances smoothed over the grid but never inside it. `color(i, c)` is
    the quad's from station i to i + 1, and a cap's (fanned to a point past each end, along the
    path, at the skin) is `color(-1, c)` or `color(len, c)`. A ray that meets no skin within
    `reach` stops there (up a foot's heel, inside the leg). A face whose color is None is left
    out (a sandal: a sole and straps). Returns the grid of vertices."""
    n = len(stations)
    rows = []
    for i, o in enumerate(stations):
        o = Vector(o)
        t = (Vector(stations[min(i + 1, n - 1)]) - Vector(stations[max(i - 1, 0)])).normalized()
        u = (Vector(up) - t * t.dot(Vector(up))).normalized()
        w = t.cross(u)
        row = []
        for c in range(sides):
            a = 2 * math.pi * c / sides
            d = u * math.cos(a) + w * math.sin(a)
            hit, _n, _i, _dist = tree.ray_cast(o, d, reach)
            row.append([o, d, (hit - o).length if hit is not None else reach])
        rows.append(row)
    _smooth(rows, True, smooth)
    verts = [[bm.verts.new(o + d * (dist + lift(i, c))) for c, (o, d, dist) in enumerate(row)]
             for i, row in enumerate(rows)]
    for i in range(n - 1):
        for c in range(sides):
            c2 = (c + 1) % sides
            col = color(i, c)
            if col is None:
                continue
            f = bm.faces.new((verts[i][c], verts[i][c2], verts[i + 1][c2], verts[i + 1][c]))
            faces[f] = col
    for end, at in ((0, -1), (n - 1, n)):
        if not caps[0 if end == 0 else 1]:
            continue
        o = Vector(stations[end])
        t = (Vector(stations[end]) - Vector(stations[end + (1 if end == 0 else -1)])).normalized()
        hit, _n, _i, _dist = tree.ray_cast(o, t)
        tip = (hit if hit is not None else o + t * 0.02) + t * lift(end, 0)
        p = bm.verts.new(tip)
        for c in range(sides):
            c2 = (c + 1) % sides
            col = color(at, c)
            if col is None:
                continue
            f = bm.faces.new((verts[end][c2], verts[end][c], p) if end == 0 else (verts[end][c], verts[end][c2], p))
            faces[f] = col
    return verts


def ribbon(bm, faces, tree, path, width, lift, color, toward=(0.0, 1.0, 0.0)):
    """A strap laid on the body: `path` [(x, z)] as seen looking `toward` it (from the front, by
    default), each point cast onto the skin along `toward` and stood `lift` off it back along the
    ray; `width` across, in the view. Its ends are square. Returns its two rails."""
    t = Vector(toward).normalized()
    up = Vector((0.0, 0.0, 1.0))
    side = t.cross(up).normalized()
    pts = [Vector(p) for p in path]
    rails = []
    for i, p in enumerate(pts):
        a, b = pts[max(i - 1, 0)], pts[min(i + 1, len(pts) - 1)]
        d = (b - a).normalized()
        n = Vector((-d.y, d.x)) * (width / 2)
        row = []
        for q in (p - n, p + n):
            start = side * q.x + up * q.y - t * 1.0
            hit, _n, _i, _d = tree.ray_cast(start, t)
            if hit is None:
                hit = start + t
            row.append(bm.verts.new(hit - t * lift))
        rails.append(row)
    for (a0, a1), (b0, b1) in zip(rails, rails[1:]):
        faces[bm.faces.new((a0, a1, b1, b0))] = color
    return rails


class Dome:
    """An ellipsoid round `centre`: `axis` its pole, `across` a second direction (the third is
    their cross product), `radii` along the three."""

    def __init__(self, centre, axis, across, radii):
        self.c = Vector(centre)
        self.a = Vector(axis).normalized()
        self.e1 = (Vector(across) - self.a * self.a.dot(Vector(across))).normalized()
        self.e2 = self.a.cross(self.e1).normalized()
        self.radii = radii

    def at(self, theta, phi, k=1.0):
        return self.c + (self.a * self.radii[0] * math.cos(theta)
                         + (self.e1 * self.radii[1] * math.cos(phi) + self.e2 * self.radii[2] * math.sin(phi))
                         * math.sin(theta)) * k


def dome(bm, faces, d, end, rows, sides, color, rim=None):
    """`d`'s cap from its pole down to `end(phi)` (its angle from the pole at `phi` round it),
    `rows` rings; colored `color`, its last ring `rim` (a brass edge) if given."""
    ring = [2 * math.pi * i / sides for i in range(sides)]
    pole = bm.verts.new(d.at(0.0, 0.0))
    first = 0.5 / rows
    grid = [[bm.verts.new(d.at(end(ph) * (first + (1 - first) * r / (rows - 1)), ph)) for ph in ring]
            for r in range(rows)]
    for i in range(sides):
        faces[bm.faces.new((pole, grid[0][i], grid[0][(i + 1) % sides]))] = color
    for r in range(rows - 1):
        for i in range(sides):
            j = (i + 1) % sides
            f = bm.faces.new((grid[r][i], grid[r][j], grid[r + 1][j], grid[r + 1][i]))
            faces[f] = rim if (rim and r == rows - 2) else color
    return grid


def strip(bm, faces, d, phis, t0, t1, rows, color, k=1.0):
    """A strip of `d`'s surface (scaled by `k`) from `t0(phi)` to `t1(phi)` across `phis`:
    a lame below a pauldron."""
    grid = [[bm.verts.new(d.at(t0(ph) + (t1(ph) - t0(ph)) * r / rows, ph, k)) for ph in phis]
            for r in range(rows + 1)]
    for r in range(rows):
        for i in range(len(phis) - 1):
            f = bm.faces.new((grid[r][i], grid[r][i + 1], grid[r + 1][i + 1], grid[r + 1][i]))
            faces[f] = color
    return grid


def finish(bm, faces, inside, thickness, name="plates"):
    """The faces as a mesh object: each turned out from `inside(centre)` (a point under it),
    split where colors meet (vertex colors are per vertex), on its slot, given `thickness`
    inward with rims. `thickness` None: the faces are closed pieces already (a kunai, a spike),
    turned out each on its own and not thickened. Frees `bm`."""
    from . import mesh
    # Points no face was made on (a sandal's left-out faces').
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
    if thickness is None:
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
    else:
        for f in faces:
            c = f.calc_center_median()
            if f.normal.dot(c - inside(c)) < 0:
                f.normal_flip()
    slots = mesh.MATERIAL_SLOTS
    for f, (_color, slot) in faces.items():
        f.material_index = slots.index(slot)
        f.smooth = False
    cut = [e for e in bm.edges if len(e.link_faces) == 2 and faces[e.link_faces[0]] != faces[e.link_faces[1]]]
    bmesh.ops.split_edges(bm, edges=cut)
    col = bm.verts.layers.float_color.new("Col")
    for f, (color, _slot) in faces.items():
        for v in f.verts:
            v[col] = (*color, 1.0)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    for s in slots:
        me.materials.append(mesh.material(s))
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    me.color_attributes.active_color = me.color_attributes["Col"]
    if thickness is None:
        return obj
    sol = obj.modifiers.new("Solidify", "SOLIDIFY")
    sol.thickness = thickness
    sol.offset = -1.0
    sol.use_rim = True
    for o in list(bpy.context.selected_objects):
        o.select_set(False)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.modifier_apply(modifier=sol.name)
    return obj


# Grain kinds for `grain`: cells per meter across (x, y) and along (z), and how deep it goes.
BARK_GRAIN = ((16.0, 16.0, 2.5), 0.45)     # streaks running up and down, like a trunk's
STONE_GRAIN = ((9.0, 9.0, 9.0), 0.4)       # blotches, the same every way


def grain(obj, kind, seed=0.0):
    """Bark's or stone's grain in the occlusion alpha, not the color (D80): a smooth noise over
    the object's points darkens it in streaks or blotches. A color is a paint region (each
    distinct one the creator can repaint), so the variation lives in the alpha, which a repaint
    keeps, and the item keeps few regions. Per point, so it flows across faces and their seams
    instead of changing face by face."""
    from mathutils import noise
    (cx, cy, cz), depth = kind
    attr = obj.data.color_attributes["Col"]
    for i, v in enumerate(obj.data.vertices):
        p = obj.matrix_world @ v.co
        q = Vector((p.x * cx + seed, p.y * cy + seed * 1.7, p.z * cz + seed * 0.3))
        n = (noise.noise(q) + 0.5 * noise.noise(q * 2.13 + Vector((5.2, 1.3, 7.9)))) / 1.5
        t = min(1.0, max(0.0, 0.5 + 0.8 * n))
        c = attr.data[i].color
        attr.data[i].color = (c[0], c[1], c[2], c[3] * (1.0 - depth * t))


def save(objs, out_path):
    """`objs` joined into one object, `item`, saved alone as `out_path` (an item's `item.blend`).
    Returns its triangles."""
    for o in list(bpy.context.selected_objects):
        o.select_set(False)
    for o in objs:
        o.select_set(True)
    bpy.context.view_layer.objects.active = objs[0]
    if len(objs) > 1:
        bpy.ops.object.join()
    obj = bpy.context.view_layer.objects.active
    obj.name = obj.data.name = "item"
    obj.data.color_attributes.active_color = obj.data.color_attributes["Col"]
    for o in list(bpy.data.objects):
        if o is not obj:
            bpy.data.objects.remove(o)
    bpy.ops.outliner.orphans_purge(do_recursive=True)
    tris = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out_path), compress=True)
    print(f"item: {tris} triangles -> {out_path}")
    return tris
