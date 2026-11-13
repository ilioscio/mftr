# SPDX-License-Identifier: AGPL-3.0-or-later
"""A steel gauntlet built on the in-game hand (05 §6, the cursor; a wardrobe item too).

**The hand** is the humanoid base's own right hand (`Hand`, from a built base's `.blend`): its
wrist, the way its fingers point, the back of the hand, the palm's breadth and depth, read from
the mesh and the bones; the fingers laid out across the knuckles in a hand's proportions. Everything is placed in the **hand's frame**: the wrist at the
origin, +Y along the hand toward the fingertips, +Z out of the back of the hand, +X across it
toward the thumb.

**The gauntlet** is pieces of plate and leather, each riding one segment of the hand: the flared
cuff and the back plate on the hand, a knuckle plate and three lames on each finger (the last
with a pointed tip), two on the thumb, and leather under them. Each piece is rigid on its
segment, as a real gauntlet's lames are on their rivets, so the hand can be posed by turning its
joints (`pose`): pointing, for the cursor.
"""

import math

import bmesh
import bpy
from mathutils import Matrix, Quaternion, Vector
from mathutils.bvhtree import BVHTree

# --- The hand -----------------------------------------------------------------------------------

class Finger:
    """A finger (or the thumb) in the hand's frame: its joints (knuckle to tip, 4 points for a
    finger, 3 for the thumb), its girth along them, and the axis its joints bend about."""

    def __init__(self, name, joints, radius, bend):
        self.name, self.joints, self.radius, self.bend = name, joints, radius, bend


class Hand:
    """The base's right hand, read from its body (`body`, under `armature`)."""

    def __init__(self, armature, body, side="r"):
        bones = armature.data.bones
        hand, fingers, thumb = bones[f"hand_{side}"], bones[f"fingers_{side}"], bones[f"thumb_{side}"]
        wrist = hand.head_local.copy()
        y = (fingers.tail_local - hand.head_local).normalized()
        # The back of the hand looks out from the body (the bundle's A pose hangs the palms
        # toward the thighs).
        out = Vector((-1.0 if side == "r" else 1.0, 0.0, 0.0))
        z = (out - y * out.dot(y)).normalized()
        x = y.cross(z)
        # +X toward the thumb: flip the frame's handedness if not.
        if (thumb.head_local - wrist).dot(x) < 0:
            x = -x
        self.frame = Matrix((x, y, z)).transposed()  # columns: the axes, in the rig's frame
        self.wrist = wrist
        to_local = self.frame.transposed()
        self.to_local = lambda p: to_local @ (Vector(p) - wrist)
        self.to_rig = lambda q: wrist + self.frame @ Vector(q)
        me = body.data
        names = {g.index: g.name for g in body.vertex_groups}
        owner = []
        for v in me.vertices:
            w = {names[g.group]: g.weight for g in v.groups}
            top = max(w.items(), key=lambda t: t[1], default=("", 0.0))[0]
            owner.append(top)
        pick = {f"hand_{side}", f"fingers_{side}", f"thumb_{side}"}
        self.verts = {i: self.to_local(v.co) for i, v in enumerate(me.vertices) if owner[i] in pick}
        # The forearm near the wrist, for the cuff.
        self.forearm = [self.to_local(v.co) for i, v in enumerate(me.vertices) if owner[i] == f"forearm_{side}"]
        self.forearm = [p for p in self.forearm if -0.12 < p.y < 0.01]
        # The hand's surface (for the back plate), in the hand's frame.
        faces = [p for p in me.polygons if all(i in self.verts for i in p.vertices)]
        ids = sorted({i for p in faces for i in p.vertices})
        at = {i: k for k, i in enumerate(ids)}
        self.surface = BVHTree.FromPolygons([self.verts[i] for i in ids], [[at[i] for i in p.vertices] for p in faces])
        self.mesh_faces = [[self.verts[i] for i in p.vertices] for p in faces]
        # The fingers. The game hand's are a dozen vertices each and merge toward the knuckles,
        # too coarse to read one by one: they're laid out from the hand's measures instead, the
        # knuckles across the palm's breadth where the fingers bone starts, each as long as the
        # fingers bone in a hand's proportions.
        ky = self.to_local(fingers.head_local).y
        reach = (fingers.tail_local - fingers.head_local).length
        palm = [p for i, p in self.verts.items() if owner[i] == f"hand_{side}" and ky - 0.035 < p.y < ky]
        xa, xb = min(p.x for p in palm), max(p.x for p in palm)
        za, zb = min(p.z for p in palm), max(p.z for p in palm)
        self.palm = (xa, xb, za, zb, ky)
        width = xb - xa
        zc = (za + zb) / 2 + (zb - za) * 0.1
        r = width / 8 * 0.98
        self.fingers = []
        for k, (name, share, splay) in enumerate(SPREAD):
            x = xa + width * (k + 0.5) / 4
            root = Vector((x, ky - 0.004 - (0.006 if name in ("little", "index") else 0.0), zc))
            d = Vector((math.sin(math.radians(splay)), math.cos(math.radians(splay)), 0.0))
            tip = root + d * reach * share
            joints = [root.lerp(tip, s) for s in PHALANGES]
            k_r = 0.86 if name == "little" else 1.0
            self.fingers.append(Finger(name, joints, [r * k_r, r * k_r * 0.9, r * k_r * 0.8], Vector((1, 0, 0))))
        # The thumb along its bone, a little past its tail (the bone stops short of the tip).
        h, t = self.to_local(thumb.head_local), self.to_local(thumb.tail_local)
        d = (t - h) * 1.1
        a = d.normalized()
        self.thumb = Finger("thumb", [h, h + d * 0.55, h + d], [r * 1.2, r * 1.05],
                            a.cross(Vector((0, 0, 1))).normalized())
        # The knuckles' line.
        self.knuckles = [f.joints[0] for f in self.fingers]

    def finger(self, name):
        return next(f for f in self.fingers + [self.thumb] if f.name == name)


# The joints along a finger, as shares of its length from the knuckle (the phalanges'
# proportions).
PHALANGES = (0.0, 0.47, 0.76, 1.0)
# Little finger to index: each one's length (a share of the fingers bone's reach) and splay
# (degrees, + toward the thumb).
SPREAD = (("little", 0.78, -5.0), ("ring", 0.95, -1.5), ("middle", 1.0, 1.0), ("index", 0.92, 3.5))


# --- Pieces ------------------------------------------------------------------------------------

class Piece:
    """A rigid piece: its vertices and faces (the hand's frame, at rest), what it's made of
    (`steel`, `leather`, `gold`), and the segment it rides: `hand`, `forearm`, or (finger name,
    segment index)."""

    def __init__(self, verts, faces, kind, seg):
        self.verts, self.faces, self.kind, self.seg = verts, faces, kind, seg


def _frame(a, up):
    """Axes for a piece along `a`: (across, along, up), `up` made square to `a`."""
    a = a.normalized()
    n = (up - a * up.dot(a)).normalized()
    b = a.cross(n)
    return b, a, n


def _shell(start, end, up, r0, r1, arc, thick, n_arc=9, ridge=0.0, flare=0.0, tip=0.0, rings=4):
    """A lame: a curved plate along `start`→`end`, an arc of `arc` degrees round the line
    centred on `up` (the back of the finger), radius `r0` to `r1`, `thick` deep, a raised
    `ridge` down its middle and its far edge flared out by `flare`; with `tip`, it runs on that
    far and closes to a point (a claw's)."""
    b, a, n = _frame(end - start, up)
    length = (end - start).length
    verts, faces = [], []
    half = math.radians(arc) / 2
    angles = [-half + 2 * half * k / (n_arc - 1) for k in range(n_arc)]
    outer, inner = [], []
    for k in range(rings):
        t = k / (rings - 1)
        r = r0 + (r1 - r0) * t + flare * max(0.0, (t - 0.7) / 0.3) ** 2
        r *= 1.0 + 0.05 * math.sin(math.pi * t)
        c = start + a * (length * t)
        o_row, i_row = [], []
        for th in angles:
            bump = ridge * max(0.0, 1.0 - abs(th) / 0.35)
            d = b * math.sin(th) + n * math.cos(th)
            o_row.append(len(verts))
            verts.append(c + d * (r + bump))
            i_row.append(len(verts))
            verts.append(c + d * (r - thick))
        outer.append(o_row)
        inner.append(i_row)
    if tip > 0.0:
        # Close to a point past the far end, a little toward the back.
        p = start + a * (length + tip) + n * r1 * 0.25
        outer.append([len(verts)] * n_arc)
        verts.append(p)
        inner.append([len(verts)] * n_arc)
        verts.append(p - n * thick)
    for rows, flip in ((outer, False), (inner, True)):
        for r in range(len(rows) - 1):
            for k in range(n_arc - 1):
                q = list(dict.fromkeys([rows[r][k], rows[r][k + 1], rows[r + 1][k + 1], rows[r + 1][k]]))
                if len(q) >= 3:
                    faces.append(q[::-1] if flip else q)
    # The edges' thickness: along each side, and round the near (and far) end.
    for r in range(len(outer) - 1):
        for k in (0, n_arc - 1):
            q = list(dict.fromkeys([outer[r][k], outer[r + 1][k], inner[r + 1][k], inner[r][k]]))
            if len(q) >= 3:
                faces.append(q if k == 0 else q[::-1])
    for r in ([0] if tip > 0.0 else [0, len(outer) - 1]):
        for k in range(n_arc - 1):
            q = [outer[r][k + 1], outer[r][k], inner[r][k], inner[r][k + 1]]
            faces.append(q if r == 0 else q[::-1])
    return verts, faces


def _tube(start, end, up, r0, r1, sides=8):
    """A closed tube (leather under the plates), its ends domed."""
    b, a, n = _frame(end - start, up)
    verts, faces, rows = [], [], []
    for t, r in ((0.0, r0), (1.0, r1)):
        c = start.lerp(end, t)
        row = []
        for k in range(sides):
            th = 2 * math.pi * k / sides
            row.append(len(verts))
            verts.append(c + (b * math.sin(th) + n * math.cos(th)) * r)
        rows.append(row)
    for k in range(sides):
        k1 = (k + 1) % sides
        faces.append([rows[0][k], rows[0][k1], rows[1][k1], rows[1][k]])
    for row, c, r, sgn in ((rows[0], start, r0, -1), (rows[1], end, r1, 1)):
        p = len(verts)
        verts.append(c + a * (sgn * r * 0.8))
        for k in range(sides):
            k1 = (k + 1) % sides
            faces.append([row[k1], row[k], p] if sgn < 0 else [row[k], row[k1], p])
    return verts, faces


def _ring(c, axis, up, rx, rz, tube, sides=18, tube_sides=6):
    """A torus round `axis` at `c` (a rim, a trim), elliptical (`rx` across, `rz` up)."""
    b, a, n = _frame(axis, up)
    verts, faces = [], []
    for k in range(sides):
        th = 2 * math.pi * k / sides
        d = b * math.sin(th) * rx + n * math.cos(th) * rz
        dn = (b * math.sin(th) * rz + n * math.cos(th) * rx).normalized()
        for j in range(tube_sides):
            ph = 2 * math.pi * j / tube_sides
            verts.append(c + d + (dn * math.cos(ph) + a * math.sin(ph)) * tube)
    for k in range(sides):
        for j in range(tube_sides):
            k1, j1 = (k + 1) % sides, (j + 1) % tube_sides
            faces.append([k * tube_sides + j, k * tube_sides + j1, k1 * tube_sides + j1, k1 * tube_sides + j])
    return verts, faces


# The cuff: how far up the forearm it runs (m) and how wide its open end flares (times the
# wrist's girth).
CUFF_LENGTH = 0.065
CUFF_FLARE = 1.5


def _cuff(hand):
    """The cuff: a bell round the wrist and the forearm, flaring to its open end, its rim rolled
    and gilt, a gilt band near the wrist."""
    pts = hand.forearm or [Vector((0.0, -0.05, 0.0))]
    near = [p for p in pts if -0.06 < p.y < -0.01] or pts
    cx = sum(p.x for p in near) / len(near)
    cz = sum(p.z for p in near) / len(near)
    rx = max(abs(p.x - cx) for p in near) * 1.12 + 0.004
    rz = max(abs(p.z - cz) for p in near) * 1.12 + 0.004
    sides, rings = 18, 6
    verts, faces, outer, inner = [], [], [], []
    for r in range(rings):
        t = r / (rings - 1)
        k = 1.0 + (CUFF_FLARE - 1.0) * t ** 1.7
        y = 0.012 - CUFF_LENGTH * t
        o, i = [], []
        for s in range(sides):
            th = 2 * math.pi * s / sides
            dn = Vector((math.sin(th), 0.0, math.cos(th)))
            p = Vector((cx + math.sin(th) * rx * k, y, cz + math.cos(th) * rz * k))
            o.append(len(verts))
            verts.append(p)
            i.append(len(verts))
            verts.append(p - dn * 0.0025)
        outer.append(o)
        inner.append(i)
    for rows, flip in ((outer, True), (inner, False)):
        for r in range(rings - 1):
            for s in range(sides):
                s1 = (s + 1) % sides
                q = [rows[r][s], rows[r][s1], rows[r + 1][s1], rows[r + 1][s]]
                faces.append(q[::-1] if flip else q)
    for r, flip in ((0, False), (rings - 1, True)):
        for s in range(sides):
            s1 = (s + 1) % sides
            q = [outer[r][s], outer[r][s1], inner[r][s1], inner[r][s]]
            faces.append(q[::-1] if flip else q)
    out = [Piece(verts, faces, "steel", "forearm")]
    axis, up = Vector((0, -1, 0)), Vector((0, 0, 1))
    rim = _ring(Vector((cx, 0.012 - CUFF_LENGTH, cz)), axis, up, rx * CUFF_FLARE, rz * CUFF_FLARE, 0.0042)
    out.append(Piece(*rim, "gold", "forearm"))
    band = _ring(Vector((cx, -0.004, cz)), axis, up, rx * 1.03, rz * 1.03, 0.0032)
    out.append(Piece(*band, "gold", "forearm"))
    return out


def _back_plate(hand):
    """The plate over the back of the hand: from the wrist to the knuckles, across the hand,
    following its surface 5 mm off, rolled down at its sides, a ridge down its middle."""
    xs = [p.x for p in hand.knuckles]
    x0, x1 = min(xs) - 0.012, max(xs) + 0.014
    y0, y1 = 0.004, min(p.y for p in hand.knuckles) - 0.004
    cols, rows = 9, 6
    grid, verts, faces = [], [], []
    for r in range(rows):
        y = y0 + (y1 - y0) * r / (rows - 1)
        row = []
        for c in range(cols):
            u = c / (cols - 1)
            x = x0 + (x1 - x0) * u
            hit = hand.surface.ray_cast(Vector((x, y, 0.15)), Vector((0, 0, -1)), 0.3)
            z = hit[0].z if hit[0] is not None else 0.0
            edge = abs(u - 0.5) * 2
            z += 0.0055 - 0.008 * max(0.0, edge - 0.6) ** 2 / 0.16 + 0.0025 * max(0.0, 1.0 - abs(u - 0.5) / 0.12)
            row.append(len(verts))
            verts.append(Vector((x, y, z)))
        grid.append(row)
    # Smooth the heights across (the low-poly hand's facets).
    for _ in range(2):
        for r in range(rows):
            zs = [verts[i].z for i in grid[r]]
            for c in range(1, cols - 1):
                verts[grid[r][c]].z = (zs[c - 1] + 2 * zs[c] + zs[c + 1]) / 4
    n0 = len(verts)
    for i in range(n0):
        verts.append(verts[i] - Vector((0, 0, 0.0025)))
    for r in range(rows - 1):
        for c in range(cols - 1):
            q = [grid[r][c], grid[r][c + 1], grid[r + 1][c + 1], grid[r + 1][c]]
            faces.append(q)
            faces.append([i + n0 for i in q[::-1]])
    rim = ([grid[0][c] for c in range(cols)] + [grid[r][cols - 1] for r in range(1, rows)]
           + [grid[rows - 1][c] for c in range(cols - 2, -1, -1)] + [grid[r][0] for r in range(rows - 2, 0, -1)])
    for k in range(len(rim)):
        a, b = rim[k], rim[(k + 1) % len(rim)]
        faces.append([b, a, a + n0, b + n0])
    return Piece(verts, faces, "steel", "hand")


def build(hand):
    """Every piece of the gauntlet on `hand`, at rest."""
    up = Vector((0, 0, 1))
    pieces = _cuff(hand)
    pieces.append(_back_plate(hand))
    # The palm's leather: the hand's own surface, behind the knuckles.
    ky = min(p.y for p in hand.knuckles) + 0.004
    lv, lf = [], []
    for f in hand.mesh_faces:
        if (sum(f, Vector()) / len(f)).y > ky:
            continue
        lf.append(list(range(len(lv), len(lv) + len(f))))
        lv.extend(p.copy() for p in f)
    pieces.append(Piece(lv, lf, "leather", "hand"))
    for f in hand.fingers + [hand.thumb]:
        thumb = f.name == "thumb"
        fup = up
        if thumb:
            # The thumb's back looks out from the hand's side and up.
            a = (f.joints[-1] - f.joints[0]).normalized()
            s = Vector((0.6, 0, 1))
            fup = (s - a * s.dot(a)).normalized()
        segs = len(f.joints) - 1
        for s in range(segs):
            p0, p1 = f.joints[s], f.joints[s + 1]
            r = f.radius[s] * 1.05 + 0.002
            d = p1 - p0
            last = s == segs - 1
            # Leather under the plates, a little past each joint.
            pieces.append(Piece(*_tube(p0 - d * 0.08, p1 + d * 0.04, fup, r * 0.92, r * 0.85), "leather", (f.name, s)))
            start = p0 - d * (0.12 if s else 0.05)
            end = p1 + (d * 0.02 if last else d * 0.10)
            sv, sf = _shell(start, end, fup, r + 0.0028, (r + 0.0018) * (0.9 if last else 1.0), 250, 0.0018,
                            ridge=0.0011, flare=0.0 if last else 0.0012, tip=r * 1.6 if last else 0.0)
            pieces.append(Piece(sv, sf, "steel", (f.name, s)))
        if not thumb:
            # A knuckle plate over the finger's root, on the hand.
            k = f.joints[0]
            a = (f.joints[1] - f.joints[0]).normalized()
            r = f.radius[0] * 1.6 + 0.004
            kv, kf = _shell(k - a * 0.011, k + a * 0.012, up, r, r * 0.95, 190, 0.002, ridge=0.002, flare=0.001)
            pieces.append(Piece(kv, kf, "steel", "hand"))
    return pieces


def tip(hand, bends):
    """Where the index finger's point is with the hand posed by `bends` (the hand's frame)."""
    f = hand.finger("index")
    r = f.radius[-1] * 1.05 + 0.002
    d = f.joints[-1] - f.joints[-2]
    a = d.normalized()
    up = Vector((0, 0, 1))
    n = (up - a * up.dot(a)).normalized()
    p = f.joints[-1] + d * 0.02 + a * (r * 1.6) + n * (r + 0.0018) * 0.9 * 0.25
    angles = bends.get("index", (0, 0, 0))
    for k in range(len(f.joints) - 2, -1, -1):
        p = _turn(p, f.joints[k], f.bend, -angles[k])
    return p


# --- Posing ------------------------------------------------------------------------------------

# The cursor's pose: each finger's joints' bend toward the palm (degrees), the thumb's too.
POINTING = {"index": (4, 2, 0), "middle": (82, 95, 55), "ring": (86, 98, 55), "little": (88, 96, 55),
            "thumb": (35, 40)}
# The thumb also swings in across the palm, about the hand's length (degrees).
THUMB_SWING = 38


def _turn(p, at, axis, deg):
    return at + Quaternion(axis, math.radians(deg)) @ (p - at)


def pose(hand, pieces, bends, swing=THUMB_SWING):
    """The pieces' vertices with the hand posed: each finger's joints bent by `bends`, the thumb
    swung in by `swing`. Returns a list of vertex lists, piece by piece."""
    out = []
    for pc in pieces:
        vs = [v.copy() for v in pc.verts]
        if isinstance(pc.seg, tuple):
            name, s = pc.seg
            f = hand.finger(name)
            angles = bends.get(name, (0,) * (len(f.joints) - 1))
            for k in range(s, -1, -1):
                deg = angles[k] if name == "thumb" else -angles[k]
                vs = [_turn(v, f.joints[k], f.bend, deg) for v in vs]
            if name == "thumb":
                vs = [_turn(v, f.joints[0], Vector((0, 1, 0)), -swing) for v in vs]
        out.append(vs)
    return out


def to_object(name, pieces, posed=None, materials=None):
    """The pieces as one mesh object `name` (the hand's frame), with the kind of each face in a
    face attribute `kind` (0 steel, 1 leather, 2 gold) and, with `materials` ({kind: material}),
    on those materials."""
    bm = bmesh.new()
    kinds = {"steel": 0, "leather": 1, "gold": 2}
    kl = bm.faces.layers.int.new("kind")
    for k, pc in enumerate(pieces):
        vs = posed[k] if posed is not None else pc.verts
        made = [bm.verts.new(v) for v in vs]
        for f in pc.faces:
            try:
                face = bm.faces.new([made[i] for i in f])
            except ValueError:
                continue
            face[kl] = kinds[pc.kind]
    me = bpy.data.meshes.new(name)
    bm.to_mesh(me)
    bm.free()
    obj = bpy.data.objects.new(name, me)
    bpy.context.scene.collection.objects.link(obj)
    if materials:
        for kname in ("steel", "leather", "gold"):
            me.materials.append(materials[kname])
        attr = me.attributes["kind"].data
        for p in me.polygons:
            p.material_index = attr[p.index].value
    return obj
