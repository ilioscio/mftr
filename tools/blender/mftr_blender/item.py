# SPDX-License-Identifier: AGPL-3.0-or-later
"""Item models (18 §8): small static props (`prop` packs) the client renders item icons from.

Same look as the champions and the map's props: faceted, vertex-colored, a material slot per
surface (`metal` steel and bronze, `cloth` leather, wood and cloth, `skin` bone and stone,
`emissive` what glows in its own color). An item is modeled facing the front view (Blender's
-Y), upright, the way it should read in its icon; `Item.pose` then tilts the whole of it in
the view plane (a sword lying corner to corner). 300–1,500 triangles: it's only ever an icon.

    it = Item("long_knife")
    it.blade(...); it.grip(...); it.pose(-45)
    it.finish()                       # then export as a `prop`

Coordinates: X right, Z up, Y away from the viewer. Sizes are about a meter for a sword;
the icon frames the whole model, so only proportions matter.
"""

import math

import bmesh
from mathutils import Matrix, Vector

from .prop import Prop


def lin(c):
    """A color picked in sRGB (0–1) to the linear vertex color the shader expects."""
    return tuple(x / 12.92 if x <= 0.04045 else ((x + 0.055) / 1.055) ** 2.4 for x in c)


# The palette (linear), darker than it looks: the icon's key light is strong.
STEEL = lin((0.62, 0.65, 0.70))
STEEL_DARK = lin((0.36, 0.38, 0.43))
IRON = lin((0.30, 0.31, 0.34))
BRONZE = lin((0.70, 0.48, 0.24))
GOLD = lin((0.86, 0.66, 0.24))
GOLD_DARK = lin((0.58, 0.40, 0.12))
SILVER = lin((0.74, 0.76, 0.80))
LEATHER = lin((0.40, 0.24, 0.14))
LEATHER_DARK = lin((0.24, 0.14, 0.08))
WOOD = lin((0.46, 0.30, 0.17))
WOOD_DARK = lin((0.28, 0.17, 0.09))
CLOTH_RED = lin((0.58, 0.12, 0.12))
CLOTH_BLUE = lin((0.18, 0.30, 0.58))
CLOTH_TEAL = lin((0.14, 0.44, 0.44))
CLOTH_PURPLE = lin((0.36, 0.18, 0.50))
CLOTH_GREEN = lin((0.20, 0.40, 0.18))
PAPER = lin((0.86, 0.80, 0.64))
BONE = lin((0.84, 0.80, 0.68))
STONE = lin((0.50, 0.50, 0.52))
STONE_DARK = lin((0.30, 0.30, 0.33))
RUBY = lin((0.92, 0.14, 0.20))
GARNET = lin((0.78, 0.10, 0.18))
SAPPHIRE = lin((0.22, 0.42, 1.0))
AMETHYST = lin((0.66, 0.32, 1.0))
EMERALD = lin((0.20, 0.86, 0.46))
TOPAZ = lin((1.0, 0.72, 0.20))
AMBER = lin((1.0, 0.58, 0.12))
CYAN = lin((0.30, 0.90, 1.0))
TEAL_GLOW = lin((0.30, 1.0, 0.82))
FIRE = lin((1.0, 0.48, 0.10))
BLOOD = lin((0.70, 0.04, 0.08))
WHITE = lin((0.96, 0.95, 0.92))
BLACK = lin((0.08, 0.08, 0.10))


class Item(Prop):
    def __init__(self, name, seed=1):
        super().__init__(name, ao_height=1.0, seed=seed)
        self.skip_bottoms = False
        self._tilt = 0.0

    def face(self, pts, slot, color, var=0.0, ao=None, out=None, center=None):
        # No ground to darken toward: items are lit whole.
        f = super().face(pts, slot, color, var, ao=1.0 if ao is None else ao, out=out, center=center)
        if getattr(self, "double_sided", False):
            # Thin cloth: the other side too, so it never shows its inside.
            back = [v.co.copy() for v in reversed(f.verts)]
            super().face(back, slot, tuple(x * 0.7 for x in color), 0.0, ao=1.0)
        return f

    # --- core: sweeps -----------------------------------------------------------------------

    def sweep(self, path, section, slot, color, var=0.06, normal=(0, 1, 0), closed=False, caps=True):
        """A tube along `path` (points); `section(i, t)` gives its cross-section at point `i`
        (`t` 0–1 along the path) as 2D points (x across the path in the view plane, y toward
        `normal`, the view axis by default). Each ring a face loop; caps at the ends."""
        path = [Vector(p) for p in path]
        n = len(path)
        nrm = Vector(normal).normalized()
        rings = []
        for i, p in enumerate(path):
            if closed:
                d = path[(i + 1) % n] - path[i - 1]
            else:
                d = path[min(i + 1, n - 1)] - path[max(i - 1, 0)]
            d = d.normalized()
            side = d.cross(nrm)
            if side.length < 1e-6:
                side = Vector((1, 0, 0))
            side.normalize()
            up = side.cross(d).normalized()
            t = i / (n - 1) if n > 1 else 0.0
            rings.append([p + side * a + up * b for a, b in section(i, t)])
        pairs = list(zip(rings, rings[1:])) + ([(rings[-1], rings[0])] if closed else [])
        for r0, r1 in pairs:
            m = len(r0)
            for k in range(m):
                j = (k + 1) % m
                q = [r0[k], r0[j], r1[j], r1[k]]
                q = [v for idx, v in enumerate(q) if (v - q[idx - 1]).length > 1e-7]
                if len(q) < 3:
                    continue
                mid = (sum(r0, Vector()) + sum(r1, Vector())) / (2 * m)
                self.face(q, slot, color, var, center=mid)
        if caps and not closed:
            ax0 = (path[1] - path[0]).normalized()
            ax1 = (path[-1] - path[-2]).normalized()
            if _area(rings[0]) > 1e-8:
                self.face(rings[0], slot, color, var, out=-ax0)
            if _area(rings[-1]) > 1e-8:
                self.face(rings[-1], slot, color, var, out=ax1)
        return rings

    def lathe(self, base, profile, n, slot, color, var=0.06, axis=(0, 0, 1), rot=0.0, cap_top=True, cap_bottom=True,
              jitter=0.0):
        """A body of revolution: rings of radius r at heights z (`profile` [(z, r)] or
        [(z, rx, ry)]) around `axis` through `base`, `n` sides."""
        base = Vector(base)
        ax = Vector(axis).normalized()
        ref = Vector((1, 0, 0)) if abs(ax.x) < 0.9 else Vector((0, 1, 0))
        u = (ref - ax * ref.dot(ax)).normalized()
        v = ax.cross(u)
        rings = []
        for p in profile:
            z, rx = p[0], p[1]
            ry = p[2] if len(p) > 2 else rx
            ring = []
            for i in range(n):
                a = rot + 2 * math.pi * i / n
                j = 1.0 + (self.rng.uniform(-jitter, jitter) if jitter else 0.0)
                ring.append(base + ax * z + u * (rx * j * math.cos(a)) + v * (ry * j * math.sin(a)))
            rings.append(ring)
        for i, (r0, r1) in enumerate(zip(rings, rings[1:])):
            mid = base + ax * ((profile[i][0] + profile[i + 1][0]) / 2)
            for k in range(n):
                j = (k + 1) % n
                q = [r0[k], r0[j], r1[j], r1[k]]
                q = [w for idx, w in enumerate(q) if (w - q[idx - 1]).length > 1e-7]
                if len(q) >= 3:
                    self.face(q, slot, color, var, center=mid)
        if cap_top and profile[-1][1] > 1e-5:
            self.face(rings[-1], slot, color, var, out=ax)
        if cap_bottom and profile[0][1] > 1e-5:
            self.face(rings[0], slot, color, var, out=-ax)
        return rings

    def extrude(self, outline, depth, slot, color, var=0.06, y=0.0, bevel=0.0, side_color=None):
        """A flat shape: `outline` [(x, z)] in the view plane, `depth` thick around `y`; with
        `bevel`, its front and back faces are inset by that much (a chamfered edge)."""
        pts = [Vector((x, y, z)) for x, z in outline]
        c = sum(pts, Vector()) / len(pts)
        h = depth / 2
        if bevel > 0:
            inner = [c + (p - c) * max(0.0, 1.0 - bevel / max((p - c).length, 1e-6)) for p in pts]
        else:
            inner = pts
        front = [p + Vector((0, -h, 0)) for p in inner]
        back = [p + Vector((0, h, 0)) for p in inner]
        mf = [p + Vector((0, -h * (0.35 if bevel else 1.0), 0)) for p in pts]
        mb = [p + Vector((0, h * (0.35 if bevel else 1.0), 0)) for p in pts]
        sc = side_color or tuple(x * 0.8 for x in color)
        self._ngon(front, slot, color, var, out=(0, -1, 0))
        self._ngon(back, slot, color, var, out=(0, 1, 0))
        n = len(pts)
        for i in range(n):
            j = (i + 1) % n
            self.face((mf[i], mf[j], mb[j], mb[i]), slot, sc, var, center=c)
            if bevel > 0:
                self.face((front[i], front[j], mf[j], mf[i]), slot, color, var, center=c + Vector((0, h, 0)))
                self.face((back[i], back[j], mb[j], mb[i]), slot, color, var, center=c - Vector((0, h, 0)))

    def _ngon(self, pts, slot, color, var, out):
        """A face for a possibly concave outline: triangulated."""
        tmp = bmesh.new()
        vs = [tmp.verts.new(p) for p in pts]
        f = tmp.faces.new(vs)
        res = bmesh.ops.triangulate(tmp, faces=[f])
        for tri in res["faces"]:
            self.face([v.co.copy() for v in tri.verts], slot, color, var, out=out)
        tmp.free()

    # --- parts ---------------------------------------------------------------------------

    def blade(self, a, b, widths, thick, slot="metal", color=STEEL, edge=None, normal=(0, 1, 0), var=0.05, curve=None):
        """A blade from `a` (hilt) to `b` (tip): a diamond cross-section, width `widths(t)` (a
        function or [(t, w)] stations), its edges lighter (`edge`). `curve(t)` bends the
        spine sideways (in the view plane)."""
        a, b = Vector(a), Vector(b)
        stations = widths if isinstance(widths, list) else [(i / 8, widths(i / 8)) for i in range(9)]
        d = b - a
        side = d.normalized().cross(Vector(normal)).normalized()
        path = []
        for t, _ in stations:
            p = a + d * t
            if curve:
                p += side * curve(t)
            path.append(p)

        def sec(i, t):
            w = stations[i][1]
            k = thick * (w / max(w for _, w in stations))
            return [(w, 0.0), (0.0, k), (-w, 0.0), (0.0, -k)]

        rings = self.sweep(path, sec, slot, color, var, normal=normal, caps=False)
        # A brighter edge: the outermost quads of each segment.
        if edge is not None:
            for r0, r1 in zip(rings, rings[1:]):
                for k in (0, 2):
                    q = [r0[k], r1[k], r1[k] + (r1[(k + 1) % 4] - r1[k]) * 0.18, r0[k] + (r0[(k + 1) % 4] - r0[k]) * 0.18]
                    if (q[0] - q[1]).length > 1e-6:
                        self.face(q, slot, edge, 0.0, out=(r0[k] - sum(r0, Vector()) / 4))
        # The base, shut.
        self.face(rings[0], slot, tuple(x * 0.7 for x in color), 0.0, out=-d)
        return rings

    def rod(self, a, b, r, sides=6, slot="cloth", color=WOOD, var=0.08, profile=None, rot=0.0):
        """A straight round rod from `a` to `b` (a grip, a shaft, a staff); `profile` [(t, r)]
        overrides the radius along it."""
        a, b = Vector(a), Vector(b)
        prof = profile or [(0.0, r), (1.0, r)]
        return self.lathe(a, [((b - a).length * t, rr) for t, rr in prof], sides, slot, color, var, axis=b - a, rot=rot)

    def grip(self, a, b, r, wraps=4, color=LEATHER, band=None, sides=6):
        """A wrapped grip: rings that bulge and pinch, in two shades."""
        a, b = Vector(a), Vector(b)
        prof = []
        for i in range(wraps * 2 + 1):
            t = i / (wraps * 2)
            prof.append((t, r * (1.08 if i % 2 else 0.94)))
        L = (b - a).length
        self.lathe(a, [(L * t, rr) for t, rr in prof], sides, "cloth", color, 0.12, axis=b - a)
        if band is not None:
            self.lathe(a, [(-0.01 * L, r * 1.25), (0.06 * L, r * 1.25)], sides, "metal", band, 0.05, axis=b - a)

    def bar(self, center, size, slot, color, var=0.06, rot=0.0):
        """A box centered on `center`, `size` (x, y, z), turned `rot` degrees in the view
        plane."""
        c = Vector(center)
        hx, hy, hz = size[0] / 2, size[1] / 2, size[2] / 2
        m = Matrix.Translation(c) @ Matrix.Rotation(math.radians(rot), 4, "Y")
        corners = [m @ Vector((sx * hx, sy * hy, sz * hz)) for sx, sy, sz in (
            (-1, -1, -1), (1, -1, -1), (1, 1, -1), (-1, 1, -1), (-1, -1, 1), (1, -1, 1), (1, 1, 1), (-1, 1, 1))]
        for f in ((0, 1, 2, 3), (4, 5, 6, 7), (0, 1, 5, 4), (1, 2, 6, 5), (2, 3, 7, 6), (3, 0, 4, 7)):
            self.face([corners[i] for i in f], slot, color, var, center=c)

    def gem(self, center, r, color, n=6, depth=1.0, slot="emissive", axis=(0, -1, 0), table=0.45, var=0.04):
        """A cut gem facing `axis`: a flat table, a crown and a pointed pavilion behind."""
        c = Vector(center)
        ax = Vector(axis).normalized()
        ref = Vector((0, 0, 1)) if abs(ax.z) < 0.9 else Vector((1, 0, 0))
        u = (ref - ax * ref.dot(ax)).normalized()
        v = ax.cross(u)
        ring = [c + u * (r * math.cos(2 * math.pi * i / n + 0.3)) + v * (r * math.sin(2 * math.pi * i / n + 0.3)) for i in range(n)]
        top = [c + ax * (0.35 * r * depth) + (p - c) * table for p in ring]
        back = c - ax * (0.9 * r * depth)
        lit = tuple(min(1.0, x * 1.25) for x in color)
        self.face(top, slot, lit, var, out=ax)
        for i in range(n):
            j = (i + 1) % n
            self.face((ring[i], ring[j], top[j], top[i]), slot, color, var, center=c - ax * r)
            self.face((ring[j], ring[i], back), slot, tuple(x * 0.6 for x in color), var, center=c + ax * r)

    def orb(self, center, r, slot, color, subdiv=1, var=0.08, squash=(1, 1, 1)):
        """A faceted sphere."""
        tmp = bmesh.new()
        bmesh.ops.create_icosphere(tmp, subdivisions=subdiv, radius=1.0)
        c = Vector(center)
        for f in tmp.faces:
            pts = [c + Vector((v.co.x * r * squash[0], v.co.y * r * squash[1], v.co.z * r * squash[2])) for v in f.verts]
            # Lit from above: upper facets a little brighter.
            k = 1.0 + 0.12 * f.normal.z
            self.face(pts, slot, tuple(min(1.0, x * k) for x in color), var, center=c)
        tmp.free()

    def torus(self, center, R, r, slot, color, n=12, m=5, axis=(0, 1, 0), var=0.06, arc=1.0, start=0.0):
        """A ring of radius `R`, tube `r`, around `axis` (facing the viewer by default)."""
        c = Vector(center)
        ax = Vector(axis).normalized()
        ref = Vector((0, 0, 1)) if abs(ax.z) < 0.9 else Vector((1, 0, 0))
        u = (ref - ax * ref.dot(ax)).normalized()
        v = ax.cross(u)
        closed = arc >= 1.0
        count = n if closed else n + 1
        path = [c + u * (R * math.cos(start + 2 * math.pi * arc * i / n)) + v * (R * math.sin(start + 2 * math.pi * arc * i / n))
                for i in range(count)]

        def sec(i, t):
            return [(r * math.cos(2 * math.pi * k / m), r * math.sin(2 * math.pi * k / m)) for k in range(m)]

        return self.sweep(path, sec, slot, color, var, normal=ax, closed=closed)

    def disc(self, center, r, thick, slot, color, n=12, axis=(0, -1, 0), var=0.05, rim=None):
        """A coin-like disc facing `axis`; `rim` colors its edge."""
        c = Vector(center)
        ax = Vector(axis).normalized()
        self.lathe(c - ax * thick / 2, [(0.0, r), (thick, r)], n, "metal" if rim else slot, rim or color, var, axis=ax,
                   cap_top=False, cap_bottom=False)
        self.lathe(c - ax * thick / 2, [(0.0, r), (thick, r)], n, slot, color, var, axis=ax)

    def chain(self, a, b, links, r, color=STEEL_DARK):
        """Small links from `a` to `b`, alternating their faces."""
        a, b = Vector(a), Vector(b)
        d = b - a
        for i in range(links):
            p = a + d * ((i + 0.5) / links)
            ax = Vector((0, 1, 0)) if i % 2 == 0 else d.normalized().cross(Vector((0, 1, 0)))
            self.torus(p, r, r * 0.3, "metal", color, n=6, m=3, axis=ax)

    def pose(self, deg):
        """Tilt the whole item `deg` degrees in the view plane (counter-clockwise as seen)."""
        self._tilt = deg

    def finish(self):
        if self._tilt:
            # Seen from -Y, a counter-clockwise turn is about +Y.
            rot = Matrix.Rotation(math.radians(self._tilt), 4, "Y")
            bmesh.ops.transform(self.bm, matrix=rot, verts=self.bm.verts)
        # Rest the item on the origin: its lowest point at z = 0, centered in x and y.
        if self.bm.verts:
            lo = Vector((min(v.co.x for v in self.bm.verts), min(v.co.y for v in self.bm.verts), min(v.co.z for v in self.bm.verts)))
            hi = Vector((max(v.co.x for v in self.bm.verts), max(v.co.y for v in self.bm.verts), max(v.co.z for v in self.bm.verts)))
            shift = Vector((-(lo.x + hi.x) / 2, -(lo.y + hi.y) / 2, -lo.z))
            bmesh.ops.translate(self.bm, vec=shift, verts=self.bm.verts)
        return super().finish()


def _area(ring):
    if len(ring) < 3:
        return 0.0
    n = Vector()
    for i, p in enumerate(ring):
        n += p.cross(ring[(i + 1) % len(ring)])
    return n.length / 2
