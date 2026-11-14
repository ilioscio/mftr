# SPDX-License-Identifier: AGPL-3.0-or-later
"""Shell items (22 §4): wardrobe items made from the reference body's own skin, for what fits
close and isn't cloth: boots, bracers, gloves. A region of the CC0 body's surface is lifted off
the skin, smoothed (toes into a boot's toe box), kept clear of the skin, reduced to a budget,
given a thickness and colored by region. The item is then fitted to every base like any other
(`wardrobe.fit_items`), so it follows the body it was made from.

An item's `build.py` describes it as a `Shell` and calls `build`:

    blender -b --python art/wardrobe/feet/ranger_boots/build.py
"""

import os

import bmesh
import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree


# How far round a shell vertex the body must lie inside it (m).
ENVELOPE = 0.025


class Shell:
    """A shell item: `keep(p)`, whether the body's face at `p` (its centre) is under it; `lift`,
    how far off the skin (m); `thickness` (m); `smooth`, smoothing passes (`smooth_where(p)`
    limits them); `shape(p, n)`, an extra move for a vertex at `p` with normal `n` (a cuff's
    flare); `paint(p)`, a face's (color, slot) by its centre; `budget`, its triangles."""

    def __init__(self, keep, lift=0.008, thickness=0.004, smooth=0, smooth_where=None, shape=None,
                 paint=None, budget=800, color=(0.36, 0.22, 0.12)):
        self.keep, self.lift, self.thickness = keep, lift, thickness
        self.smooth, self.smooth_where, self.shape = smooth, smooth_where, shape
        self.paint = paint or (lambda p: (color, "cloth"))
        self.budget = budget


def _apply(obj, mod):
    for o in list(bpy.context.selected_objects):
        o.select_set(False)
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.modifier_apply(modifier=mod.name)


def make(spec, body):
    """`spec` made on `body` (the reference body, a mesh object): a new mesh object `item`."""
    from . import mesh
    me = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in me.vertices], [tuple(p.vertices) for p in me.polygons])
    bm = bmesh.new()
    bm.from_mesh(me)
    bm.normal_update()
    normals = {v.index: v.normal.copy() for v in bm.verts}
    gone = [f for f in bm.faces if not spec.keep(f.calc_center_median())]
    bmesh.ops.delete(bm, geom=gone, context="FACES")
    bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context="VERTS")
    # Off the skin, each vertex along its own normal on the whole body.
    for v in bm.verts:
        n = normals.get(v.index, v.normal)
        v.co = v.co + n * spec.lift
    # Smoothing: the region's inner vertices toward their neighbours' mean, the edge held.
    for _ in range(spec.smooth):
        moves = {}
        for v in bm.verts:
            if v.is_boundary or (spec.smooth_where and not spec.smooth_where(v.co)):
                continue
            nb = [e.other_vert(v).co for e in v.link_edges]
            if nb:
                moves[v] = sum(nb, Vector()) / len(nb)
        for v, c in moves.items():
            v.co = v.co.lerp(c, 0.6)
    # Shaped (a cuff flaring), then an envelope: every body vertex near enough to be under it
    # stays `lift` inside it (toes don't poke through a toe box smoothed over them).
    bm.normal_update()
    from mathutils.kdtree import KDTree
    kd = KDTree(len(me.vertices))
    for v in me.vertices:
        kd.insert(v.co, v.index)
    kd.balance()
    for v in bm.verts:
        n = v.normal
        if spec.shape:
            v.co = v.co + spec.shape(v.co, n)
        push = 0.0
        for co, _i, _d in kd.find_range(v.co, ENVELOPE):
            push = max(push, (co - v.co).dot(n) + spec.lift)
        if push > 0.0:
            v.co = v.co + n * push
        q, nrm, _i, d = tree.find_nearest(v.co)
        if q is not None and (v.co - q).dot(nrm) < spec.lift:
            v.co = q + nrm * spec.lift
    out = bpy.data.meshes.new("item")
    bm.to_mesh(out)
    bm.free()
    obj = bpy.data.objects.new("item", out)
    bpy.context.scene.collection.objects.link(obj)
    # Reduced to its budget (the thickness doubles it, less the rims).
    tris = sum(len(p.vertices) - 2 for p in out.polygons)
    target = spec.budget * 0.45
    if tris > target:
        dec = obj.modifiers.new("Decimate", "DECIMATE")
        dec.ratio = target / tris
        _apply(obj, dec)
    # Colors and slots, by region; split where they meet (vertex colors are per vertex).
    slots = mesh.MATERIAL_SLOTS
    out = obj.data
    out.materials.clear()
    for s in slots:
        out.materials.append(mesh.material(s))
    bm = bmesh.new()
    bm.from_mesh(out)
    kinds = {}
    for f in bm.faces:
        kinds[f] = spec.paint(f.calc_center_median())
        f.material_index = slots.index(kinds[f][1])
        f.smooth = False
    cut = [e for e in bm.edges if len(e.link_faces) == 2 and kinds[e.link_faces[0]] != kinds[e.link_faces[1]]]
    bmesh.ops.split_edges(bm, edges=cut)
    col = bm.verts.layers.float_color.new("Col")
    for f in bm.faces:
        for v in f.verts:
            v[col] = (*kinds[f][0], 1.0)
    bm.to_mesh(out)
    bm.free()
    out.color_attributes.active_color = out.color_attributes["Col"]
    # Its thickness: inward, rims at the open edges, the inside in its shade.
    sol = obj.modifiers.new("Solidify", "SOLIDIFY")
    sol.thickness = spec.thickness
    sol.offset = -1.0
    sol.use_rim = True
    _apply(obj, sol)
    return obj


def build(spec, out_path):
    """Make `spec` on the reference body and save it as an item's `item.blend`."""
    from . import cc0_body
    bpy.ops.wm.read_homefile(use_empty=True)
    body = cc0_body.load("male")
    obj = make(spec, body)
    bpy.data.objects.remove(body)
    for o in list(bpy.data.objects):
        if o is not obj:
            bpy.data.objects.remove(o)
    bpy.ops.outliner.orphans_purge(do_recursive=True)
    tris = sum(len(p.vertices) - 2 for p in obj.data.polygons)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out_path), compress=True)
    print(f"item: {tris} triangles -> {out_path}")
    return tris
