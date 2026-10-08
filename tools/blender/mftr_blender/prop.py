# SPDX-License-Identifier: AGPL-3.0-or-later
"""Map props (structures, trees, rocks): static models on a one-bone `prop` rig (11 §3, kind
`prop`). Same look as the champions: faceted, vertex-colored, a material slot per surface
(`accent` takes the team color, `accent_glow` glows in it, `emissive` glows in its own color).

Every face gets its own vertices, so each can carry its own shade: stone blocks, needles and rock
facets vary a little in color, which is where the detail comes from (no textures, 05 §3). The
alpha channel carries baked occlusion, darker toward the ground.

    p = Prop("turret", ao_height=5.0)
    p.blocks_ring(...); p.loft(...); p.rock(...)
    p.finish()                        # then run.py export --id turret --kind prop
"""

import math
import random

import bmesh
import bpy
from mathutils import Matrix, Vector

from . import mesh

ARCHETYPE = "prop"


def new_scene():
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30


def _armature(name):
    data = bpy.data.armatures.new(name)
    obj = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(obj)
    prev = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.mode_set(mode="EDIT")
    b = data.edit_bones.new("root")
    b.head = (0, 0, 0)
    b.tail = (0, 0, 0.5)
    b.use_deform = True
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev
    obj["mftr_archetype"] = ARCHETYPE
    obj["mftr_rig_version"] = 1
    return obj


class Prop:
    def __init__(self, name, ao_height=3.0, seed=1):
        self.name = name
        self.ao_height = ao_height
        self.rng = random.Random(seed)
        self.arm = _armature(name)
        self.bm = bmesh.new()
        self.deform = self.bm.verts.layers.deform.verify()
        self.col = self.bm.verts.layers.float_color.new("Col")
        self.tris = 0

    # --- faces ----------------------------------------------------------------------------

    def face(self, pts, slot, color, var=0.0, ao=None, out=None, center=None):
        """One flat face with its own vertices and a shade of `color` (±`var`). It faces `out`
        (a direction), or away from `center` (a point); else its winding stands."""
        pts = [Vector(p) for p in pts]
        if getattr(self, "clamp_ground", False):
            for q in pts:
                q.z = max(q.z, 0.0)  # tumbled pieces rest on the ground, not in it
        if out is None and center is not None:
            out = sum(pts, Vector()) / len(pts) - Vector(center)
        if out is not None:
            n = Vector()
            for i, p in enumerate(pts):
                q = pts[(i + 1) % len(pts)]
                n += Vector(((p.y - q.y) * (p.z + q.z), (p.z - q.z) * (p.x + q.x), (p.x - q.x) * (p.y + q.y)))
            if n.dot(Vector(out)) < 0:
                pts.reverse()
        k = 1.0 + (self.rng.uniform(-var, var) if var else 0.0)
        c = tuple(max(0.0, min(1.0, x * k)) for x in color)
        vs = []
        for p in pts:
            v = self.bm.verts.new(p)
            v[self.deform][0] = 1.0
            a = ao if ao is not None else max(0.5, min(1.0, 0.5 + 0.5 * p[2] / self.ao_height))
            v[self.col] = (*c, a)
            vs.append(v)
        f = self.bm.faces.new(vs)
        f.material_index = mesh.MATERIAL_SLOTS.index(slot)
        f.smooth = False
        self.tris += len(pts) - 2
        return f

    def quad_strip(self, ring0, ring1, slot, color, var=0.0, closed=True, inward=False):
        """Quads between two rings, facing away from the axis through their centers."""
        n = len(ring0)
        c0 = sum(ring0, Vector()) / n
        c1 = sum(ring1, Vector()) / n
        for i in range(n if closed else n - 1):
            j = (i + 1) % n
            q = [ring0[i], ring0[j], ring1[j], ring1[i]]
            q = [v for k, v in enumerate(q) if (v - q[k - 1]).length > 1e-7]  # cones: drop the repeats
            if len(q) < 3:
                continue
            mid = sum(q, Vector()) / 4
            # The nearest point on the axis c0–c1.
            d = c1 - c0
            t = 0.0 if d.length < 1e-6 else max(0.0, min(1.0, (mid - c0).dot(d) / d.length_squared))
            axis_pt = c0 + d * t
            if inward:
                self.face(q, slot, color, var, out=axis_pt - sum(q, Vector()) / len(q))
            else:
                self.face(q, slot, color, var, center=axis_pt)

    def cap(self, ring, slot, color, var=0.0, up=True):
        self.face(list(ring), slot, color, var, out=(0, 0, 1 if up else -1))

    # --- shapes ---------------------------------------------------------------------------

    def ring(self, c, rx, ry, n, z, rot=0.0, jitter=0.0):
        out = []
        for i in range(n):
            a = rot + 2 * math.pi * i / n
            j = 1.0 + (self.rng.uniform(-jitter, jitter) if jitter else 0.0)
            out.append(Vector((c[0] + rx * j * math.cos(a), c[1] + ry * j * math.sin(a), z)))
        return out

    def prism(self, c, profile, n, slot, color, var=0.0, rot=0.0, cap_top=True, cap_bottom=False, jitter=0.0, inward=False):
        """A vertical prism through `profile` [(z, r)] or [(z, rx, ry)], `n` sides."""
        rings = []
        for p in profile:
            z, rx = p[0], p[1]
            ry = p[2] if len(p) > 2 else rx
            rings.append(self.ring(c, rx, ry, n, z, rot, jitter))
        for r0, r1 in zip(rings, rings[1:]):
            self.quad_strip(r0, r1, slot, color, var, inward=inward)
        if cap_top:
            self.cap(rings[-1], slot, color, var)
        if cap_bottom:
            self.cap(rings[0], slot, color, var, up=False)
        return rings

    def box(self, center, size, slot, color, var=0.0, rot_z=0.0, taper=1.0, tilt=(0.0, 0.0)):
        """A box standing on `center` (its bottom face's center); `taper` scales the top."""
        hx, hy, h = size[0] / 2, size[1] / 2, size[2]
        m = Matrix.Translation(Vector(center)) @ Matrix.Rotation(rot_z, 4, "Z") @ Matrix.Rotation(tilt[0], 4, "X") @ Matrix.Rotation(tilt[1], 4, "Y")
        bot = [m @ Vector((x, y, 0)) for x, y in ((-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy))]
        top = [m @ Vector((x * taper, y * taper, h)) for x, y in ((-hx, -hy), (hx, -hy), (hx, hy), (-hx, hy))]
        mid = m @ Vector((0, 0, h / 2))
        for i in range(4):
            j = (i + 1) % 4
            self.face((bot[i], bot[j], top[j], top[i]), slot, color, var, center=mid)
        self.face(top, slot, color, var, center=mid)
        if not getattr(self, "skip_bottoms", False):
            self.face(bot, slot, color, var, center=mid)

    def blocks_ring(self, c, r, z0, courses, course_h, n, depth, slot, color, var=0.12, gap=0.025, taper=0.0,
                    alt=None, alt_every=0):
        """Courses of stone blocks around a round wall of radius `r` (shrinking by `taper` per
        course), every other course offset half a block. `alt` recolors one block in
        `alt_every` (moss, darker stones)."""
        for k in range(courses):
            rr = r - taper * k
            z = z0 + k * course_h
            off = (math.pi / n) * (k % 2)
            for i in range(n):
                a0 = off + 2 * math.pi * i / n + gap / rr
                a1 = off + 2 * math.pi * (i + 1) / n - gap / rr
                ro, ri = rr, rr - depth
                hz = course_h - gap
                pts = lambda rad, a, zz: Vector((c[0] + rad * math.cos(a), c[1] + rad * math.sin(a), zz))
                col = self.rng.choice(color) if isinstance(color, list) else color
                if alt is not None and alt_every and self.rng.randrange(alt_every) == 0:
                    col = alt
                # The outer face bulges a little at its middle (a dressed stone).
                am = (a0 + a1) / 2
                bulge = ro + depth * 0.08
                o0, o1, om = pts(ro, a0, z), pts(ro, a1, z), pts(bulge, am, z + hz * 0.5)
                o2, o3 = pts(ro, a1, z + hz), pts(ro, a0, z + hz)
                k_ = 1.0 + self.rng.uniform(-var, var)
                cc = tuple(min(1.0, x * k_) for x in col)
                axis = Vector((c[0], c[1], z + hz * 0.5))
                for tri in ((o0, o1, om), (o1, o2, om), (o2, o3, om), (o3, o0, om)):
                    self.face(tri, slot, cc, center=axis)
                # Top of the block.
                self.face((o3, o2, pts(ri, a1, z + hz), pts(ri, a0, z + hz)), slot, tuple(min(1.0, x * 1.08) for x in cc), out=(0, 0, 1))

    def rock(self, center, size, slot, color, var=0.1, rough=0.22, subdiv=1, flat_bottom=True):
        """A faceted boulder: an icosphere pushed about, squashed to `size`."""
        tmp = bmesh.new()
        bmesh.ops.create_icosphere(tmp, subdivisions=subdiv, radius=1.0)
        for v in tmp.verts:
            v.co *= 1.0 + self.rng.uniform(-rough, rough)
            v.co = Vector((v.co.x * size[0] / 2, v.co.y * size[1] / 2, v.co.z * size[2] / 2 + size[2] * 0.42))
            if flat_bottom and v.co.z < 0:
                v.co.z = 0.0
            v.co += Vector(center)
        mid = Vector(center) + Vector((0, 0, size[2] * 0.42))
        for f in tmp.faces:
            pts = [v.co.copy() for v in f.verts]
            if max(p.z for p in pts) - Vector(center).z < 1e-4:
                continue  # flattened into the ground: never seen
            # Lighter tops, darker undersides: a weathered stone.
            out = sum(pts, Vector()) / len(pts) - mid
            k = 1.0 + 0.14 * out.normalized().z
            self.face(pts, slot, tuple(min(1.0, x * k) for x in color), var, out=out)
        tmp.free()

    def star_tier(self, c, z, r, h, points, slot, color, var=0.1, inner=0.62, droop=0.12, rot=0.0):
        """A jagged cone (a pine's tier): a star-shaped skirt rising to a point."""
        ring = []
        for i in range(points * 2):
            a = rot + math.pi * i / points
            rr = r if i % 2 == 0 else r * inner
            dz = -droop * r if i % 2 == 0 else 0.0
            ring.append(Vector((c[0] + rr * math.cos(a), c[1] + rr * math.sin(a), z + dz + self.rng.uniform(-0.03, 0.03))))
        tip = Vector((c[0] + self.rng.uniform(-0.05, 0.05) * r, c[1] + self.rng.uniform(-0.05, 0.05) * r, z + h))
        n = len(ring)
        axis = Vector((c[0], c[1], z + h * 0.3))
        for i in range(n):
            self.face((ring[i], ring[(i + 1) % n], tip), slot, color, var, center=axis)
        # The underside, darker (shadowed needles), facing down.
        center = Vector((c[0], c[1], z + h * 0.15))
        dark = tuple(x * 0.55 for x in color)
        for i in range(n):
            self.face((ring[(i + 1) % n], ring[i], center), slot, dark, var * 0.5, out=(0, 0, -1))

    def shingled_cone(self, c, z0, r, h, n, courses, slot, color, var=0.1, overhang=0.08, teeth=True):
        """A conical roof of overlapping shingle courses, each with a toothed lower edge."""
        for k in range(courses):
            t0, t1 = k / courses, (k + 1) / courses
            r0, r1 = r * (1 - t0) + overhang * (1 - t0), r * (1 - t1)
            za, zb = z0 + h * t0, z0 + h * t1
            m = n if k < courses - 1 else max(6, n // 2)
            count = m * 2 if teeth else m
            lo = []
            for i in range(count):
                a = 2 * math.pi * i / count + (math.pi / m) * (k % 2)
                rr = r0 * (1.0 if (i % 2 == 0 or not teeth) else 0.93)
                dz = -0.05 if (teeth and i % 2 == 0) else 0.0
                lo.append(Vector((c[0] + rr * math.cos(a), c[1] + rr * math.sin(a), za + dz)))
            if k == courses - 1:
                tip = Vector((c[0], c[1], z0 + h))
                for i in range(len(lo)):
                    self.face((lo[i], lo[(i + 1) % len(lo)], tip), slot, color, var, center=Vector((c[0], c[1], za)))
                break
            hi = []
            for v in lo:
                a = math.atan2(v.y - c[1], v.x - c[0])
                hi.append(Vector((c[0] + r1 * math.cos(a), c[1] + r1 * math.sin(a), zb + 0.04)))
            self.quad_strip(lo, hi, slot, color, var)
            # The lower lip of the course, in shadow.
            under = [Vector((v.x - (v.x - c[0]) * 0.12, v.y - (v.y - c[1]) * 0.12, za + 0.02)) for v in lo]
            for i in range(len(lo)):
                j = (i + 1) % len(lo)
                self.face((lo[i], lo[j], under[j], under[i]), slot, tuple(x * 0.6 for x in color), out=(0, 0, -1))

    def loft(self, a, b, sides, profile, slot, color, var=0.0):
        """`mesh._loft`'s cross-sections from `a` to `b`, faces of their own."""
        a, b = Vector(a), Vector(b)
        axis = (b - a).normalized()
        ref = Vector((1, 0, 0)) if abs(axis.x) < 0.9 else Vector((0, 0, 1))
        u = (ref - axis * ref.dot(axis)).normalized()
        v = axis.cross(u)
        rings = []
        for t, ru, rv in profile:
            cc = a.lerp(b, t)
            rings.append([cc + u * (ru * math.cos(2 * math.pi * (i + 0.5) / sides)) + v * (rv * math.sin(2 * math.pi * (i + 0.5) / sides))
                          for i in range(sides)])
        for r0, r1 in zip(rings, rings[1:]):
            self.quad_strip(r0, r1, slot, color, var)
        self.face(rings[-1], slot, color, var, out=axis)
        self.face(rings[0], slot, color, var, out=-axis)

    # --- output ---------------------------------------------------------------------------

    def finish(self):
        me = bpy.data.meshes.new(self.name)
        obj = bpy.data.objects.new(self.name, me)
        bpy.context.scene.collection.objects.link(obj)
        for slot in mesh.MATERIAL_SLOTS:
            me.materials.append(mesh.material(slot))
        obj.vertex_groups.new(name="root")
        self.bm.to_mesh(me)
        self.bm.free()
        me.color_attributes.active_color = me.color_attributes["Col"]
        me.color_attributes.render_color_index = me.color_attributes.active_color_index
        obj.parent = self.arm
        mod = obj.modifiers.new("Armature", "ARMATURE")
        mod.object = self.arm
        # Unused material slots would export as empty primitives: drop them.
        used = {p.material_index for p in me.polygons}
        for i in reversed(range(len(me.materials))):
            if i not in used:
                me.materials.pop(index=i)
        print(f"{self.name}: {mesh.triangle_count(obj)} triangles")
        return obj


def build_and_save(build, out):
    """Run `build()` in a fresh scene and save the .blend to `out`."""
    import os
    new_scene()
    build()
    os.makedirs(os.path.dirname(os.path.abspath(out)), exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=os.path.abspath(out), compress=True)
