# SPDX-License-Identifier: AGPL-3.0-or-later
"""Hair (22 §7 item 5, BH1): shaped hair meshes, groomed by physics on each base's own head.

A style is its **grooming pattern** (`Style`), data in `art/wardrobe/hair/<name>/groom.py`:

- **a cap**: the scalp inside the hairline, lifted off the skin by the style's volume (thin at
  the hairline, full a few centimetres in);
- **locks** (`Locks`): where they root (scalp points at a spacing, or given points), how long,
  wide and thick they are, which way they are combed, how stiff they are, and whether they are
  drawn to a **tie** (a ponytail's, a topknot's), pinned there.

Each base grows its own (`groom_all`, from `creator.build`): every lock starts as a ribbon
combed along its flow over the head, then Blender's cloth simulation lets it fall under gravity
onto the head and the shoulders, its root (and its tie) held. Its drape and its rest on the skull
are physics (D77, as M1's garments). The ribbon then gives the lock its body: a lens across,
tapering to the tip. Its faces carry the strands' coordinates (UV `14 + along`, `4 + across`),
which the champion shader draws as hairs, as on the brows.

Every place is given in the **source head's frame** (`authored_head`: metres, face toward -Y,
the Real_MALE head) and taken to the base by its fit, so one pattern grows on every head.
"""

import importlib.util
import math
import os
import random

import bmesh
import bpy
from mathutils import Vector
from mathutils.bvhtree import BVHTree
from mathutils.kdtree import KDTree

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "..", "art", "wardrobe"))
# The slots grown by grooming, each a folder of `ROOT` (22 §7 item 5).
SLOTS = ("hair", "beard")

# The default hair color (as the old catalog's).
HAIR = (0.24, 0.16, 0.11)

# --- The hairline (source frame) -----------------------------------------------------------------

# The head's vertical axis, through which the hairline's angle is measured (y).
AXIS_Y = 0.012
# The hairline's height (z) by the angle from the front (0) round to the back (π): the forehead,
# the temples, the sideburns in front of the ear, behind the ear, the nape.
HAIRLINE = [(0.00, 0.098), (0.45, 0.094), (0.75, 0.080), (1.00, 0.060), (1.20, 0.030), (1.36, 0.004),
            (1.75, 0.012), (2.20, -0.020), (2.70, -0.048), (math.pi, -0.056)]
# The ear, kept bare: an ellipsoid round it (centre, half-axes x, y, z).
EAR = (Vector((0.072, 0.022, 0.006)), (0.03, 0.026, 0.036))


def theta(p):
    return math.atan2(abs(p.x), -(p.y - AXIS_Y))


def hairline(t):
    for (t0, z0), (t1, z1) in zip(HAIRLINE, HAIRLINE[1:]):
        if t <= t1:
            f = (t - t0) / (t1 - t0)
            return z0 + (z1 - z0) * f
    return HAIRLINE[-1][1]


def above(p):
    """How far `p` (source frame) is above the hairline (m); negative below it or by the ear."""
    h = p.z - hairline(theta(p))
    c, (ax, ay, az) = EAR
    e = ((abs(p.x) - c.x) / ax) ** 2 + ((p.y - c.y) / ay) ** 2 + ((p.z - c.z) / az) ** 2
    if e < 1.0:
        h = min(h, (e - 1.0) * 0.02)
    return h


# The beard's area (source frame): the jaw, the chin and the cheeks below a line from the
# sideburns' foot to under the nose, the lip above the mouth, in front of the ears; the lips bare.
BEARD_TOP = [(0.0, -0.026), (0.018, -0.028), (0.032, -0.026), (0.045, -0.016), (0.058, -0.004), (0.068, 0.006)]
MOUTH = (Vector((0.0, -0.058, -0.045)), (0.027, 0.0105))


def beard_line(x):
    x = abs(x)
    for (x0, z0), (x1, z1) in zip(BEARD_TOP, BEARD_TOP[1:]):
        if x <= x1:
            return z0 + (z1 - z0) * (x - x0) / (x1 - x0)
    return BEARD_TOP[-1][1]


def beard(p):
    """How far `p` (source frame) is inside the beard's area (m); negative outside it."""
    h = min(beard_line(p.x) - p.z, 0.035 - p.y)
    c, (ax, az) = MOUTH
    if p.y < c.y + 0.02:
        e = (p.x / ax) ** 2 + ((p.z - c.z) / az) ** 2
        if e < 1.0:
            h = min(h, (e - 1.0) * 0.01)
    return h


# --- A pattern ---------------------------------------------------------------------------------------

class Locks:
    """A set of locks. `where(p)` picks scalp roots (source frame) at `spacing` apart, or `roots`
    gives them; `flow(p)` is the way they're combed (source frame, any length); `length` (or
    `length(p)` by the root), `width` and `thick` in metres (width and thickness taper to the tip); `stiff` is the cloth's
    bending stiffness (short hair stands, long hair falls); `tie`, a source point they're drawn
    to and held at; `profile`, `lens` (flat) or `round`; `plait`, a braid's bulges."""

    def __init__(self, where=None, roots=None, spacing=0.03, length=0.08, width=0.04, thick=0.012,
                 flow=lambda p: Vector((0, 1, -0.4)), stiff=1.0, tie=None, profile="lens", plait=False,
                 taper=0.75, seg=0.02, seed=1, lift=0.004, hug=0.03, sides=None, lengthen=True):
        self.where, self.roots, self.spacing = where, roots, spacing
        self.length, self.width, self.thick, self.flow = length, width, thick, flow
        self.stiff, self.tie, self.profile, self.plait = stiff, tie, profile, plait
        self.taper, self.seg, self.seed, self.lift = taper, seg, seed, lift
        # Within `hug` of the skin a lock is combed along it. `sides`: the faces round a lock
        # (4 for a lens, 6 for a round by default).
        self.hug = hug
        self.sides = sides or (4 if profile == "lens" else 6)
        # Whether the style's length control lengthens them (a tie's locks reach the tie whatever
        # it says; a knot keeps its coil).
        self.lengthen = lengthen and tie is None


class Style:
    """A hairstyle: its cap's volume (`cap`, m; 0 for none) and its lock sets; `tie_band`, a band
    at each tie (its color); `budget` (triangles)."""

    def __init__(self, cap=0.01, locks=(), color=HAIR, band=None, budget=2400, cap_from=0.0, cap_where=None,
                 ramp=0.025, shaved=0.0015, lengths=(0.55, 1.7), area=None, blend=0.05, drape=True):
        self.cap, self.locks, self.color, self.band, self.budget = cap, list(locks), color, band, budget
        # Where it grows: `area(p)`, how far inside it a source point is (m); the scalp's by
        # default (`above`), the beard's (`beard`). Its weights come from the skin within `blend`
        # (m) round each vertex; with `drape`, a lock falling below the skull onto the neck and
        # the back takes the body's weights there (a beard follows the jaw however long).
        self.area = area or above
        self.blend, self.drape = blend, drape
        # The cap covers the scalp from `cap_from` above the hairline (a receding line) and where
        # `cap_where(p)` says, if given; elsewhere on the scalp (a crest's sides) a `shaved` one.
        # It reaches its full depth `ramp` in from its edge.
        self.cap_from, self.cap_where, self.ramp, self.shaved = cap_from, cap_where, ramp, shaved
        # The length control's ends (22 §7 item 5, BH2): the locks it lengthens, this many times
        # as long at its short and long ends (`<slot>_length-` and `<slot>_length+`); None for
        # none.
        self.lengths = lengths


def styles():
    """Every style, hair and beards: (folder, `item.ron` fields, the `Style`), by slot and id."""
    from . import wardrobe
    out = []
    for slot in SLOTS:
        sdir = os.path.join(ROOT, slot)
        if not os.path.isdir(sdir):
            continue
        for name in sorted(os.listdir(sdir)):
            d = os.path.join(sdir, name)
            ron, groom = os.path.join(d, "item.ron"), os.path.join(d, "groom.py")
            if os.path.isfile(ron) and os.path.isfile(groom):
                spec = importlib.util.spec_from_file_location(f"groom_{slot}_{name}", groom)
                mod = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(mod)
                out.append((d, wardrobe.read_item(ron), mod.STYLE))
    return out


# --- The head ----------------------------------------------------------------------------------------

class Head:
    """The base's head on its body: the source frame's map onto it (`fit`) and back, its skin's
    vertices and faces, and a tree of the whole body for combing and pushing out."""

    def __init__(self, body, fit):
        from . import face_rig
        self.body, self.fit = body, fit
        me = body.data
        names = {g.index: g.name for g in body.vertex_groups}
        g_neck = body.vertex_groups.get("neck")

        def head_vertex(v):
            # The eyeballs and lids (on the eye bones) and the neck aren't the head's skin; a
            # trace of their weight on the forehead or the nape doesn't count.
            gs = [(names[g.group], g.weight) for g in v.groups if g.weight > 0.01]
            if sum(w for n, w in gs if n.startswith("face_eye")) > 0.3:
                return False
            if g_neck is not None and sum(w for n, w in gs if n == "neck") > 0.5:
                return False
            return sum(w for n, w in gs if n == "head" or n in face_rig.NAMES) > 0.5

        self.skin = [head_vertex(v) for v in me.vertices]
        self.pos = [v.co.copy() for v in me.vertices]
        self.normals = [v.normal.copy() for v in me.vertices]
        self.tree = BVHTree.FromPolygons(self.pos, [tuple(p.vertices) for p in me.polygons])
        self.weights = [{names[g.group]: g.weight for g in v.groups if g.weight > 0.0} for v in me.vertices]
        self.kd = KDTree(len(self.pos))
        for i, p in enumerate(self.pos):
            self.kd.insert(p, i)
        self.kd.balance()

    def src(self, p):
        """A base point in the source frame."""
        from .authored_head import JOINT
        return JOINT + (Vector(p) - self.fit.joint) / self.fit.s

    def rig(self, p):
        return self.fit(Vector(p))

    def scalp(self, margin=0.0, area=above):
        """The skin's vertices inside `area` (the scalp: above the hairline) by `margin` (source
        metres)."""
        return [i for i, ok in enumerate(self.skin) if ok and area(self.src(self.pos[i])) > margin]

    def outside(self, p, lift):
        """`p` pushed out to `lift` off the body if it's closer (or inside)."""
        loc, nrm, _, _ = self.tree.find_nearest(p)
        if loc is None:
            return p
        if (p - loc).dot(nrm) < lift:
            return loc + nrm * lift
        return p


# --- Combing and simulating -----------------------------------------------------------------------

def _roots(head, locks, scale, area=above):
    """The lock set's roots on this head: (point, normal) in the rig's frame."""
    if locks.roots is not None:
        out = []
        for q in locks.roots:
            p = head.rig(q)
            loc, nrm, _, _ = head.tree.find_nearest(p)
            out.append((loc, nrm))
        return out
    cand = [i for i in head.scalp(0.004, area) if locks.where is None or locks.where(head.src(head.pos[i]))]
    rng = random.Random(locks.seed)
    rng.shuffle(cand)
    cand.sort(key=lambda i: -head.pos[i].z)  # from the crown down: the parting's roots first
    taken = []
    spacing = locks.spacing * scale
    for i in cand:
        p = head.pos[i]
        if any((p - q).length < spacing for q, _ in taken):
            continue
        taken.append((p, head.normals[i]))
    return taken


def _comb(head, root, normal, locks, scale, k=1.0):
    """A lock's centreline combed from its root: along its flow over the head (pushed off the
    skin), toward its tie if it has one, falling under its own weight once off the skull. `k`
    lengthens it (its segments, not their number: every length has the same mesh)."""
    seg = locks.seg * scale
    lift = (locks.lift + locks.thick * 0.5) * scale
    p = head.outside(root + normal * lift, lift)
    pts = [p]
    tie = head.rig(locks.tie) if locks.tie is not None else None
    length = locks.length(head.src(root)) if callable(locks.length) else locks.length
    n = max(2, round(length * scale / seg)) if tie is None else 0
    if locks.lengthen:
        seg *= k
    k = 0
    while True:
        if tie is not None:
            if (tie - p).length <= seg * 1.2:
                pts.append(tie)
                break
            d = (tie - p).normalized()
            if len(pts) > 60:
                break
        else:
            if k >= n:
                break
            d = Vector(locks.flow(head.src(p)))
            d.normalize()
        loc, nrm, _, dist = head.tree.find_nearest(p)
        if loc is not None and dist < locks.hug:
            # Along the skin near it (combed flat over the skull), free further off.
            t = min(dist / locks.hug, 1.0) ** 2
            along = d - nrm * d.dot(nrm)
            d = along.lerp(d, t)
        d.normalize()
        p = head.outside(p + d * seg, lift)
        pts.append(p)
        k += 1
    return pts


def _ribbon(bm, pts, width, normal):
    """A ribbon along `pts` (two rails `width` apart, across the lock); returns its rows."""
    rows = []
    for i, p in enumerate(pts):
        t = (pts[min(i + 1, len(pts) - 1)] - pts[max(i - 1, 0)]).normalized()
        b = normal.cross(t)
        if b.length < 1e-6:
            b = Vector((1, 0, 0))
        b.normalize()
        rows.append((bm.verts.new(p - b * width * 0.5), bm.verts.new(p + b * width * 0.5)))
    for (a0, a1), (b0, b1) in zip(rows, rows[1:]):
        bm.faces.new((a0, a1, b1, b0))
    return rows


def _simulate(head, combed, frames=50):
    """The combed locks' ribbons dropped by cloth physics onto the body. `combed` is
    [(centreline, width, normal, stiff, held rows)]; returns each lock's settled rows as
    [(left, right)] positions."""
    bm = bmesh.new()
    held = []
    rows_of = []
    for pts, width, normal, stiff, hold in combed:
        rows = _ribbon(bm, pts, width, normal)
        rows_of.append(rows)
        for r in hold:
            held.extend(rows[r])
    bm.verts.index_update()
    rows_idx = [[(a.index, b.index) for a, b in rows] for rows in rows_of]
    pin = {v.index for v in held}
    me = bpy.data.meshes.new("locks")
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new("locks", me)
    bpy.context.scene.collection.objects.link(obj)
    g = obj.vertex_groups.new(name="pin")
    g.add(list(pin), 1.0, "REPLACE")
    # Stiffness per vertex isn't a cloth setting: the stiffest lock set's, with the others' drape
    # from their own length (a stiff short lock barely bends under its weight).
    stiff = max(s for _, _, _, s, _ in combed)
    body = head.body
    col = body.modifiers.new("Collision", "COLLISION")
    body.collision.thickness_outer = 0.003
    body.collision.cloth_friction = 8.0
    cloth = obj.modifiers.new("Cloth", "CLOTH")
    s = cloth.settings
    s.quality = 10
    s.mass = 0.05
    s.air_damping = 4.0
    s.tension_stiffness = s.compression_stiffness = 40.0
    s.shear_stiffness = 10.0
    s.bending_stiffness = stiff
    s.vertex_group_mass = "pin"
    s.pin_stiffness = 1.0
    c = cloth.collision_settings
    c.distance_min = 0.003
    c.use_self_collision = False
    cache = cloth.point_cache
    cache.frame_start, cache.frame_end = 1, frames
    scene = bpy.context.scene
    scene.frame_start, scene.frame_end = 1, frames
    for f in range(1, frames + 1):
        scene.frame_set(f)
    dg = bpy.context.evaluated_depsgraph_get()
    ev = obj.evaluated_get(dg)
    co = [v.co.copy() for v in ev.data.vertices]
    bpy.data.objects.remove(obj)
    bpy.data.meshes.remove(me)
    body.modifiers.remove(col)
    scene.frame_set(1)
    return [[(co[a], co[b]) for a, b in rows] for rows in rows_idx]


# --- Building the mesh -------------------------------------------------------------------------------

# The strands' density across (UV units per metre): the shader draws 12 to a unit.
ACROSS = 1.0 / 0.16


def _lock(bm, uv, col, rows, locks, scale, color, rng):
    """A lock's body from its settled ribbon: a lens (or a round) across, tapering to the tip;
    the tip closed on a point."""
    n = len(rows)
    sides = locks.sides
    off = rng.random() * 3.0
    rings = []
    for i, (a, b) in enumerate(rows):
        t = i / (n - 1)
        c = (a + b) * 0.5
        across = b - a
        fwd = ((rows[min(i + 1, n - 1)][0] + rows[min(i + 1, n - 1)][1]) - (rows[max(i - 1, 0)][0] + rows[max(i - 1, 0)][1]))
        if across.length < 1e-6 or fwd.length < 1e-6:
            across, fwd = Vector((1, 0, 0)), Vector((0, 0, -1))
        bdir = across.normalized()
        ndir = bdir.cross(fwd.normalized()).normalized()
        k = 1.0 - locks.taper * t ** 1.4
        w = locks.width * scale * k * 0.5
        h = locks.thick * scale * (1.0 - 0.55 * t) * 0.5
        if locks.plait:
            bulge = 1.0 if i % 2 == 0 else 0.72
            w *= bulge
            h *= bulge
        if locks.profile == "round":
            w = h = max(w, h) if not locks.plait else w
        ring = []
        for j in range(sides):
            a_ = 2 * math.pi * j / sides
            if locks.plait:
                a_ += (i % 2) * math.pi / sides
            p = c + bdir * math.cos(a_) * w + ndir * math.sin(a_) * h
            ring.append(bm.verts.new(p))
        rings.append((ring, t))
    tip = bm.verts.new((rows[-1][0] + rows[-1][1]) * 0.5)
    faces = []
    perim = 2 * math.pi * locks.width * scale * 0.4
    for (r0, t0), (r1, t1) in zip(rings, rings[1:]):
        for j in range(sides):
            jn = (j + 1) % sides
            f = bm.faces.new((r0[j], r0[jn], r1[jn], r1[j]))
            for l, (t, jj) in zip(f.loops, ((t0, j), (t0, j + 1), (t1, j + 1), (t1, j))):
                l[uv].uv = (14.0 + min(t, 0.999), 4.0 + off + perim * ACROSS * jj / sides)
            faces.append(f)
    rl, tl = rings[-1]
    for j in range(sides):
        jn = (j + 1) % sides
        f = bm.faces.new((rl[j], rl[jn], tip))
        for l, (t, jj) in zip(f.loops, ((tl, j), (tl, j + 1), (1.0, j + 0.5))):
            l[uv].uv = (14.0 + min(t, 0.999), 4.0 + off + perim * ACROSS * jj / sides)
        faces.append(f)
    for f in faces:
        for v in f.verts:
            v[col] = (*color, 1.0)
    return faces


def _cap(bm, uv, col, head, style, scale):
    """The cap: the scalp's faces, lifted by the style's volume (thin at the hairline). Its edge
    is cut along the hairline itself, not round the skin's faces: a face the hairline crosses
    keeps its part above it, the corners below moved up their edges to where it crosses."""
    me = head.body.data
    h = {i: style.area(head.src(head.pos[i])) - style.cap_from for i, ok in enumerate(head.skin) if ok}
    keep = {i for i, x in h.items() if x > 0.0}
    full = keep
    if style.cap_where is not None:
        full = {i for i in keep if style.cap_where(head.src(head.pos[i]))}
        if not style.shaved:
            keep = full
    polys = [p for p in me.polygons if all(i in h for i in p.vertices) and any(i in keep for i in p.vertices)]
    # Where each corner below the line goes: the mean of the crossings toward the face corners
    # above it that it shares a face with.
    cross = {}
    for p in polys:
        inside = [j for j in p.vertices if j in keep]
        for i in p.vertices:
            if i in keep:
                continue
            for j in inside:
                hj, hi = h[j], min(h[i], 0.0)
                f = hj / (hj - hi) if hj - hi > 1e-9 else 0.5
                pt = head.pos[j].lerp(head.pos[i], f)
                nm = head.normals[j].lerp(head.normals[i], f)
                acc = cross.setdefault(i, [Vector(), Vector(), 0])
                acc[0] += pt
                acc[1] += nm
                acc[2] += 1
    made = {}
    faces = []
    for p in polys:
        vs = []
        for i in p.vertices:
            if i not in made:
                if i in keep:
                    x = h[i]
                    ramp = min(max(x / style.ramp, 0.0), 1.0)
                    ramp = ramp * ramp * (3 - 2 * ramp)
                    depth = style.cap if i in full else style.shaved
                    at, nm = head.pos[i], head.normals[i]
                else:
                    pt, nm, n = cross[i]
                    at, nm, ramp, depth = pt / n, nm.normalized(), 0.0, 0.0
                lift = (0.0018 + depth * ramp) * scale
                made[i] = bm.verts.new(at + nm * lift)
            vs.append(made[i])
        # A face whose corners met at one crossing is gone.
        if len({v for v in vs}) < 3 or any((a.co - b.co).length < 1e-5 for a, b in zip(vs, vs[1:] + vs[:1])):
            vs = [v for k, v in enumerate(vs) if (v.co - vs[k - 1].co).length >= 1e-5]
            if len(vs) < 3:
                continue
        try:
            f = bm.faces.new(vs)
        except ValueError:
            continue
        for l in f.loops:
            q = head.src(l.vert.co)
            l[uv].uv = (14.0 + min(max(style.area(q) / 0.03, 0.0), 1.0) * 0.3, 4.0 + q.x * ACROSS)
        faces.append(f)
    for f in faces:
        for v in f.verts:
            v[col] = (*style.color, 1.0)
    bmesh.ops.remove_doubles(bm, verts=list({v for f in faces for v in f.verts}), dist=1e-5)
    return faces


def _band(bm, uv, col, head, at, color, scale, radius=0.016):
    """A tie's band: a short ring round the gathered hair."""
    c = head.rig(at)
    loc, nrm, _, _ = head.tree.find_nearest(c)
    axis = (c - loc).normalized() if (c - loc).length > 1e-5 else nrm
    u = axis.orthogonal().normalized()
    v = axis.cross(u)
    r = radius * scale
    rings = []
    for k, d in enumerate((0.006, 0.020)):
        ring = [bm.verts.new(c + axis * d * scale + (u * math.cos(a) + v * math.sin(a)) * r)
                for a in [2 * math.pi * j / 8 for j in range(8)]]
        rings.append(ring)
    faces = []
    for j in range(8):
        jn = (j + 1) % 8
        faces.append(bm.faces.new((rings[0][j], rings[0][jn], rings[1][jn], rings[1][j])))
    for f in faces:
        for l in f.loops:
            l[uv].uv = (0.5, 0.5)
            l.vert[col] = (*color, 1.0)
    return faces


# The share of the cap's faces kept.
CAP_KEEP = 0.5


def _settle(head, style, roots, scale, k=1.0):
    """The style's locks combed from `roots` (per lock set) and settled: [(rows, locks)]."""
    combed = []
    sets = []
    for locks, rs in zip(style.locks, roots):
        for root, normal in rs:
            pts = _comb(head, root, normal, locks, scale, k)
            if len(pts) < 3:
                continue
            hold = [0, 1] + ([len(pts) - 2, len(pts) - 1] if locks.tie is not None else [])
            combed.append((pts, locks.width * scale, normal, locks.stiff, hold))
            sets.append(locks)
    settled = _simulate(head, combed) if combed else []
    return list(zip(settled, sets))


def _mesh(head, style, locks, scale, name):
    """The hair's mesh: the cap, the locks' bodies (`locks`, from `_settle`), the bands."""
    bm = bmesh.new()
    uv = bm.loops.layers.uv.new("Paint")
    col = bm.verts.layers.float_color.new("Col")
    rng = random.Random(7)
    if style.cap > 0.0 or style.cap_where is not None:
        # The cap follows the skull, whose facets are finer than a cap needs: halved.
        _cap(bm, uv, col, head, style, scale)
        me = bpy.data.meshes.new("cap")
        bm.to_mesh(me)
        bm.free()
        cap = bpy.data.objects.new("cap", me)
        bpy.context.scene.collection.objects.link(cap)
        dec = cap.modifiers.new("Decimate", "DECIMATE")
        dec.ratio = CAP_KEEP
        dg = bpy.context.evaluated_depsgraph_get()
        reduced = bpy.data.meshes.new_from_object(cap.evaluated_get(dg))
        bpy.data.objects.remove(cap)
        bpy.data.meshes.remove(me)
        bm = bmesh.new()
        bm.from_mesh(reduced)
        bpy.data.meshes.remove(reduced)
        uv = bm.loops.layers.uv["Paint"]
        col = bm.verts.layers.float_color["Col"]
    for rows, ls in locks:
        _lock(bm, uv, col, rows, ls, scale, style.color, rng)
    if style.band is not None:
        for at in sorted({tuple(t.tie) for t in style.locks if t.tie is not None}):
            _band(bm, uv, col, head, Vector(at), style.band, scale)
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    return me


def grow(head, style, name, slot="hair"):
    """`style` grown on `head`: a new mesh object `name`, in the rig's frame, and its length
    control's ends ({shape key name: vertex positions}, empty for a style without one; the keys
    are `<slot>_length-` and `+`)."""
    from . import mesh
    scale = head.fit.s
    roots = [_roots(head, locks, scale, style.area) for locks in style.locks]
    me = _mesh(head, style, _settle(head, style, roots, scale), scale, name)
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    for s in mesh.MATERIAL_SLOTS:
        me.materials.append(mesh.material(s))
    cloth = mesh.MATERIAL_SLOTS.index("cloth")
    for p in me.polygons:
        p.material_index = cloth
        p.use_smooth = False
    me.color_attributes.active_color = me.color_attributes["Col"]
    me.color_attributes.render_color_index = me.color_attributes.active_color_index
    ends = {}
    if style.lengths and any(ls.lengthen for ls in style.locks):
        # The same locks, longer and shorter, settled again: the same mesh, moved.
        for key, k in ((f"{slot}_length-", style.lengths[0]), (f"{slot}_length+", style.lengths[1])):
            other = _mesh(head, style, _settle(head, style, roots, scale, k), scale, "length")
            if len(other.vertices) == len(me.vertices):
                ends[key] = [v.co.copy() for v in other.vertices]
            bpy.data.meshes.remove(other)
    return obj, ends


# --- Skinning --------------------------------------------------------------------------------------

def _weights(obj, head, blend=0.05, drape=True):
    """Each hair vertex takes the head skin's weights round where it roots (face bones included,
    averaged over 5 cm, as `authored_head.make_conform`; the neck's share given to the head), and,
    where it hangs below the skull onto the neck and shoulders, more and more of the body's under
    it. A lock that reaches the body follows the head and the body only, not the face (no face
    bone weighs near the neck, 21 §1.5). Two bones a vertex, as the body's."""
    from . import face_rig
    skin = [i for i, ok in enumerate(head.skin) if ok]
    kd_head = KDTree(len(skin))
    for j, i in enumerate(skin):
        kd_head.insert(head.pos[i], j)
    kd_head.balance()
    rest = [i for i, ok in enumerate(head.skin) if not ok and not any(n.startswith("face_eye") for n in head.weights[i])]
    kd_body = KDTree(len(rest))
    for j, i in enumerate(rest):
        kd_body.insert(head.pos[i], j)
    kd_body.balance()
    me = obj.data
    # Each lock (and the cap) is a piece of its own.
    root = list(range(len(me.vertices)))

    def find(a):
        while root[a] != a:
            root[a] = root[root[a]]
            a = root[a]
        return a

    for e in me.edges:
        ra, rb = find(e.vertices[0]), find(e.vertices[1])
        if ra != rb:
            root[rb] = ra
    top = head.fit.joint.z
    acc_of = []
    on_body = set()
    for v in me.vertices:
        p = v.co
        acc = {}
        near = kd_head.find_range(p, blend) or [kd_head.find(p)]
        total = 0.0
        for _, j, d in near:
            f = max(1.0 - d / blend, 0.05)
            total += f
            for n, w in head.weights[skin[j]].items():
                n = "head" if n == "neck" else n
                acc[n] = acc.get(n, 0.0) + w * f
        acc = {n: w / total for n, w in acc.items()}
        # Below the head's joint the hair lies on the neck and the back: the body's weights.
        t = min(max((top - p.z) / 0.12, 0.0), 1.0) if drape else 0.0
        if t > 0.0:
            on_body.add(find(v.index))
            _, j, _ = kd_body.find(p)
            body_w = head.weights[rest[j]]
            acc = {n: w * (1.0 - t) for n, w in acc.items()}
            for n, w in body_w.items():
                acc[n] = acc.get(n, 0.0) + w * t
        acc_of.append(acc)
    groups = {}
    for v, acc in zip(me.vertices, acc_of):
        if find(v.index) in on_body:
            folded = {}
            for n, w in acc.items():
                n = "head" if n in face_rig.NAMES else n
                folded[n] = folded.get(n, 0.0) + w
            acc = folded
        best = sorted(((w, n) for n, w in acc.items() if w > 1e-4), reverse=True)[:2]
        norm = sum(w for w, _ in best) or 1.0
        for w, n in best:
            if n not in groups:
                groups[n] = obj.vertex_groups.get(n) or obj.vertex_groups.new(name=n)
            groups[n].add([v.index], w / norm, "REPLACE")
    face_rig.limit_steps(obj)


def groom_all(armature, body, fit, budget_scale=1.0):
    """Every hairstyle grown on `body` and skinned to `armature`, as catalog parts. Returns
    `made` entries (as `parts.build_parts`')."""
    from . import body_rig, mesh, parts
    found = styles()
    if not found:
        return []
    head = Head(body, fit)
    made = []
    for _folder, info, style in found:
        obj, ends = grow(head, style, f"part.{info['id']}", info["slot"])
        _weights(obj, head, style.blend, style.drape)
        obj.parent = armature
        mod = obj.modifiers.new("Armature", "ARMATURE")
        mod.object = armature
        body_rig.transfer(body, obj)
        n = parts.transfer_shape_keys(body, obj)
        if ends:
            # The length control (BH2): the hair's (or the beard's) own slider pair.
            if obj.data.shape_keys is None:
                obj.shape_key_add(name="Basis", from_mix=False)
            for key, co in sorted(ends.items()):
                kb = obj.shape_key_add(name=key, from_mix=False)
                for i, q in enumerate(co):
                    kb.data[i].co = q
            n += 1
        made.append((info["id"], info["slot"], info["name"], [], bool(info["accent"]), mesh.triangle_count(obj), n,
                     {}))
    return made


def length_sliders(armature):
    """The length controls the grown parts under `armature` have (`hair_length`,
    `beard_length`)."""
    out = set()
    for o in armature.children:
        keys = o.data.shape_keys.key_blocks if o.type == "MESH" and o.data.shape_keys else []
        for slot in SLOTS:
            if o.name.startswith(f"part.{slot}.") and f"{slot}_length+" in keys:
                out.add(f"{slot}_length")
    return sorted(out)
