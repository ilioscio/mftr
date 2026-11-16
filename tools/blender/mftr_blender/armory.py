# SPDX-License-Identifier: AGPL-3.0-or-later
"""The armory (22 §4.7, BW7): a small kit for modelling held items in their grip frame (the grip
at the origin, +Z along the item, +Y its back, +X its side: `wardrobe._place_grip`). An item's
`build.py` makes a `Kit`, adds pieces and saves:

    k = armory.Kit()
    k.lathe([(-0.08, 0.02), (0.08, 0.02)], LEATHER)          # a grip
    k.slab([(0.0, 0.1), (0.03, 0.12), (0.0, 0.9)], 0.008, IRON, "metal")
    k.save(os.path.join(HERE, "item.blend"))

Faceted on purpose: flat-shaded, few sides, beveled where the light should catch an edge.
"""

import math
import os

import bmesh
import bpy
from mathutils import Matrix, Vector

from . import mesh


class Kit:
    """A held item under construction: one bmesh, each face with its (color, material slot)."""

    def __init__(self):
        bpy.ops.wm.read_homefile(use_empty=True)
        self.bm = bmesh.new()
        # The color layer first: adding a layer later reallocates the vertices.
        self.col = self.bm.verts.layers.float_color.new("Col")
        self.faces = []
        # Optional bones of its own (`wardrobe._place_grip`): [(name, head, tail, parent)].
        self.bones = []
        self.weights = {}

    def _face(self, verts, color, slot):
        f = self.bm.faces.new(verts)
        self.faces.append((f, color, slot))
        return f

    def lathe(self, profile, color, slot="cloth", sides=8, at=(0, 0, 0), axis=(0, 0, 1), twist=0.0, cap=True):
        """A surface of revolution: `profile` [(along, radius)] from bottom to top, round `axis`
        through `at`. Closed at the ends unless their radius is 0 (a point) or `cap` is False."""
        a = Vector(axis).normalized()
        o = Vector(at)
        side = a.orthogonal().normalized()
        up = a.cross(side)
        rings = []
        for z, r in profile:
            c = o + a * z
            if r <= 1e-6:
                rings.append([self.bm.verts.new(c)])
                continue
            rings.append([self.bm.verts.new(c + (side * math.cos(twist + 2 * math.pi * k / sides)
                                                 + up * math.sin(twist + 2 * math.pi * k / sides)) * r)
                          for k in range(sides)])
        for lo, hi in zip(rings, rings[1:]):
            if len(lo) == 1 and len(hi) == 1:
                continue
            if len(lo) == 1:
                for k in range(sides):
                    self._face((lo[0], hi[(k + 1) % sides], hi[k]), color, slot)
            elif len(hi) == 1:
                for k in range(sides):
                    self._face((lo[k], lo[(k + 1) % sides], hi[0]), color, slot)
            else:
                for k in range(sides):
                    self._face((lo[k], lo[(k + 1) % sides], hi[(k + 1) % sides], hi[k]), color, slot)
        if cap and len(rings[0]) > 1:
            self._face(list(reversed(rings[0])), color, slot)
        if cap and len(rings[-1]) > 1:
            self._face(rings[-1], color, slot)
        return rings

    def slab(self, outline, thickness, color, slot="metal", bevel=None, at=(0, 0, 0), turn=None):
        """A flat piece with a sharpened rim: `outline` [(x, z)] counter-clockwise seen from the
        front (-y), at y = 0 its edge; its faces `thickness` thick, inset by `bevel` (default:
        three times the thickness) so the rim slopes to the edge like a ground blade. `turn`, a
        rotation (a `Matrix`), then `at`: where it goes."""
        bevel = thickness * 3 if bevel is None else bevel
        pts = [Vector((x, 0.0, z)) for x, z in outline]
        n = len(pts)
        inner = []
        for i in range(n):
            p, a, b = pts[i], pts[i - 1], pts[(i + 1) % n]
            # Inward normals of the two edges (counter-clockwise in x-z seen from -y: inward is
            # to the left of each edge's direction).
            d1, d2 = (p - a).normalized(), (b - p).normalized()
            n1, n2 = Vector((-d1.z, 0, d1.x)), Vector((-d2.z, 0, d2.x))
            m = (n1 + n2)
            if m.length < 1e-6:
                m = n1
            m.normalize()
            k = bevel / max(m.dot(n1), 0.35)
            inner.append(p + m * k)
        rot = turn or Matrix.Identity(3)
        place = lambda v: rot @ v + Vector(at)  # noqa: E731
        edge = [self.bm.verts.new(place(p)) for p in pts]
        front = [self.bm.verts.new(place(p + Vector((0, -thickness, 0)))) for p in inner]
        back = [self.bm.verts.new(place(p + Vector((0, thickness, 0)))) for p in inner]
        self._face(front, color, slot)
        self._face(list(reversed(back)), color, slot)
        for i in range(n):
            j = (i + 1) % n
            self._face((edge[i], edge[j], front[j], front[i]), color, slot)
            self._face((edge[j], edge[i], back[i], back[j]), color, slot)
        return edge

    def blade(self, stations, color, slot="metal", ridge=0.006, at=(0, 0, 0), turn=None):
        """A double-edged blade along +Z with a diamond section: `stations` [(z, half-width)]
        from the base to the tip (a 0 width ends it in a point), `ridge` its half-thickness on
        the centre line (thinning toward the tip)."""
        rot = turn or Matrix.Identity(3)
        place = lambda v: rot @ v + Vector(at)  # noqa: E731
        rings = []
        for z, w in stations:
            t = ridge * (w / max(stations[0][1], 1e-6)) ** 0.5 if w > 0 else 0.0
            if w <= 1e-6:
                rings.append([self.bm.verts.new(place(Vector((0, 0, z))))])
            else:
                # Counter-clockwise round +z: the faces look out.
                rings.append([self.bm.verts.new(place(Vector(v))) for v in ((w, 0, z), (0, t, z), (-w, 0, z), (0, -t, z))])
        for lo, hi in zip(rings, rings[1:]):
            if len(hi) == 1:
                for k in range(4):
                    self._face((lo[k], lo[(k + 1) % 4], hi[0]), color, slot)
                continue
            for k in range(4):
                self._face((lo[k], lo[(k + 1) % 4], hi[(k + 1) % 4], hi[k]), color, slot)
        self._face(list(reversed(rings[0])), color, slot)
        return rings

    def box(self, center, size, color, slot="metal", turn=None, bevel=0.0):
        """A box `size` (x, y, z) at `center`, optionally turned (a `Matrix`) and beveled (a
        fraction of its smallest side)."""
        before = set(self.bm.faces)
        made = bmesh.ops.create_cube(self.bm, size=1.0)
        verts = made["verts"]
        rot = turn or Matrix.Identity(3)
        for v in verts:
            v.co = rot @ Vector((v.co.x * size[0], v.co.y * size[1], v.co.z * size[2])) + Vector(center)
        if bevel > 0:
            edges = list({e for v in verts for e in v.link_edges})
            bmesh.ops.bevel(self.bm, geom=verts + edges, offset=bevel * min(size), segments=1, affect="EDGES",
                            profile=0.5)
        faces = [f for f in self.bm.faces if f not in before]
        for f in faces:
            self.faces.append((f, color, slot))
        return faces

    def gem(self, center, radius, color, slot="emissive", stretch=1.0, turn=None):
        """An octahedron: a cut stone, a crystal (`stretch` lengthens it along its z)."""
        rot = turn or Matrix.Identity(3)
        c = Vector(center)
        pts = [Vector(p) for p in ((radius, 0, 0), (0, radius, 0), (-radius, 0, 0), (0, -radius, 0))]
        ring = [self.bm.verts.new(c + rot @ p) for p in pts]
        top = self.bm.verts.new(c + rot @ Vector((0, 0, radius * stretch)))
        bot = self.bm.verts.new(c + rot @ Vector((0, 0, -radius * stretch)))
        for k in range(4):
            self._face((ring[k], ring[(k + 1) % 4], top), color, slot)
            self._face((ring[(k + 1) % 4], ring[k], bot), color, slot)

    def move(self, by):
        """Shift everything made so far (to put the grip at the origin)."""
        for v in self.bm.verts:
            v.co += Vector(by)

    def weigh(self, bone, verts):
        """Give `verts` to one of the item's own bones (`bones`)."""
        for v in verts:
            self.weights[v] = {bone: 1.0}

    def save(self, path):
        """Colors, slots and normals, the item's bones if any; `item.blend` at `path`."""
        bm = self.bm
        # Every piece is a closed shell: its faces look out, whatever order they were made in.
        bmesh.ops.recalc_face_normals(bm, faces=list(bm.faces))
        slots = mesh.MATERIAL_SLOTS
        for f, c, slot in self.faces:
            if not f.is_valid:
                continue
            f.material_index = slots.index(slot)
            f.smooth = False
            for v in f.verts:
                v[self.col] = (*c, 1.0)
        bm.normal_update()
        me = bpy.data.meshes.new("item")
        for s in slots:
            me.materials.append(mesh.material(s))
        bm.verts.index_update()
        weights = [(v.index, w) for v, w in self.weights.items() if v.is_valid]
        bm.to_mesh(me)
        bm.free()
        obj = bpy.data.objects.new("item", me)
        bpy.context.scene.collection.objects.link(obj)
        me.color_attributes.active_color = me.color_attributes["Col"]
        if self.bones:
            import json
            for vi, ws in weights:
                for name, w in ws.items():
                    g = obj.vertex_groups.get(name) or obj.vertex_groups.new(name=name)
                    g.add([vi], w, "REPLACE")
            obj["mftr_bones"] = json.dumps([(n, list(a), list(b), p) for n, a, b, p in self.bones])
        tris = sum(len(p.vertices) - 2 for p in me.polygons)
        bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(path), compress=True)
        print(f"item: {tris} triangles -> {path}")
        return tris
