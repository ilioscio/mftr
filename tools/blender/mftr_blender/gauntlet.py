# SPDX-License-Identifier: AGPL-3.0-or-later
"""A steel gauntlet built on the in-game hand (05 §6, the cursor; a wardrobe item too).

**The hand** is the humanoid base's own right hand (`Hand`, from a built base's `.blend`): its
wrist, the way its fingers point, the back of the hand, the palm's breadth and depth, read from
the mesh and the bones; the fingers laid out across the knuckles in a hand's proportions.
Everything is placed in the **hand's frame**, a right-handed one: the wrist at the origin, +Y
along the hand toward the fingertips, +Z out of the back of the hand, +X across it (toward the
little finger on a right hand, the thumb on a left). The back of the hand is found from the
thumb, not assumed: on a right hand it is the fingers' way crossed with the thumb's.

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
        # The thumb's side, square to the hand; the back of the hand from it (a right hand seen
        # from its back, fingers up, has its thumb on the left).
        t = thumb.head_local - wrist
        t = (t - y * t.dot(y)).normalized()
        z = y.cross(t) if side == "r" else t.cross(y)
        x = y.cross(z)
        self.side = side
        # Which way +X is from the thumb: -1 on a right hand (the thumb at -X), +1 on a left.
        self.thumb_x = 1.0 if t.dot(x) > 0 else -1.0
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
        tx = self.thumb_x
        self.fingers = []
        for k, (name, share, splay) in enumerate(SPREAD):
            # Little finger to index, toward the thumb.
            u = (k + 0.5) / 4
            x = (xa + width * u) if tx > 0 else (xb - width * u)
            root = Vector((x, ky - 0.004 - (0.006 if name in ("little", "index") else 0.0), zc))
            d = Vector((tx * math.sin(math.radians(splay)), math.cos(math.radians(splay)), 0.0))
            tip = root + d * reach * share
            joints = [root.lerp(tip, s) for s in PHALANGES]
            k_r = 0.86 if name == "little" else 1.0
            self.fingers.append(Finger(name, joints, [r * k_r, r * k_r * 0.9, r * k_r * 0.8], _curl_axis(d)))
        # The thumb along its bone, past its tail (the bone stops short of the tip).
        h, tt = self.to_local(thumb.head_local), self.to_local(thumb.tail_local)
        d = (tt - h) * THUMB_REACH
        self.thumb = Finger("thumb", [h, h + d * 0.55, h + d], [r * 1.2, r * 1.05], _curl_axis(d))
        # The knuckles' line.
        self.knuckles = [f.joints[0] for f in self.fingers]

    def finger(self, name):
        return next(f for f in self.fingers + [self.thumb] if f.name == name)


def _curl_axis(d):
    """The axis a finger along `d` curls about, toward the palm (-Z), turning by a positive
    angle."""
    return d.normalized().cross(Vector((0, 0, -1))).normalized()


# The joints along a finger, as shares of its length from the knuckle (the phalanges'
# proportions).
PHALANGES = (0.0, 0.47, 0.76, 1.0)
# Little finger to index: each one's length (a share of the fingers bone's reach) and splay
# (degrees, + toward the thumb).
# The thumb's length, times its bone's (the bone stops at the second joint's middle).
THUMB_REACH = 1.3
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


def _shell(start, end, up, r0, r1, arc, thick, n_arc=9, ridge=0.0, flare=0.0, tip=0.0, rings=4, solid=True):
    """A lame: a curved plate along `start`→`end`, an arc of `arc` degrees round the line
    centred on `up` (the back of the finger), radius `r0` to `r1`, `thick` deep, a raised
    `ridge` down its middle and its far edge flared out by `flare`; with `tip`, it runs on that
    far and closes to a point (a claw's). Not `solid`, only its outer face (the game's: its
    underside never shows)."""
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
    for rows, flip in ((outer, False), (inner, True)) if solid else ((outer, False),):
        for r in range(len(rows) - 1):
            for k in range(n_arc - 1):
                q = list(dict.fromkeys([rows[r][k], rows[r][k + 1], rows[r + 1][k + 1], rows[r + 1][k]]))
                if len(q) >= 3:
                    faces.append(q[::-1] if flip else q)
    if not solid:
        return verts, faces
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


def _tube(start, end, up, r0, r1, sides=8, caps=True):
    """A tube (leather under the plates), its ends domed (with `caps`)."""
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
    for row, c, r, sgn in ((rows[0], start, r0, -1), (rows[1], end, r1, 1)) if caps else ():
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


# Levels of detail: the cursor's render, close up, and the game's, on a champion (05 §5's
# budgets): each plate's arc and its rings, the leather's sides, the cuff's sides and rings,
# the trims' sides and their tubes', the back plate's grid, and whether the plates have their
# underside and edges (the game's don't: they never show).
LOD = {
    "cursor": {"arc": 9, "rings": 4, "tube": 8, "cuff": (18, 6), "ring": (18, 6), "plate": (9, 6), "solid": True},
    "game": {"arc": 5, "rings": 3, "tube": 5, "cuff": (12, 3), "ring": (10, 3), "plate": (5, 4), "solid": False},
}


def _cuff(hand, lod):
    """The cuff: a bell round the wrist and the forearm, flaring to its open end, its rim rolled
    and gilt, a gilt band near the wrist."""
    pts = hand.forearm or [Vector((0.0, -0.05, 0.0))]
    near = [p for p in pts if -0.06 < p.y < -0.01] or pts
    cx = sum(p.x for p in near) / len(near)
    cz = sum(p.z for p in near) / len(near)
    rx = max(abs(p.x - cx) for p in near) * 1.12 + 0.004
    rz = max(abs(p.z - cz) for p in near) * 1.12 + 0.004
    sides, rings = lod["cuff"]
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
    rim = _ring(Vector((cx, 0.012 - CUFF_LENGTH, cz)), axis, up, rx * CUFF_FLARE, rz * CUFF_FLARE, 0.0042, *lod["ring"])
    out.append(Piece(*rim, "gold", "forearm"))
    band = _ring(Vector((cx, -0.004, cz)), axis, up, rx * 1.03, rz * 1.03, 0.0032, *lod["ring"])
    out.append(Piece(*band, "gold", "forearm"))
    return out


def _back_plate(hand, lod):
    """The plate over the back of the hand: from the wrist to the knuckles, across the hand,
    following its surface 5 mm off, rolled down at its sides, a ridge down its middle."""
    xs = [p.x for p in hand.knuckles]
    x0, x1 = min(xs) - 0.012, max(xs) + 0.014
    y0, y1 = 0.004, min(p.y for p in hand.knuckles) - 0.004
    cols, rows = lod["plate"]
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
    if not lod["solid"]:
        for r in range(rows - 1):
            for c in range(cols - 1):
                faces.append([grid[r][c], grid[r][c + 1], grid[r + 1][c + 1], grid[r + 1][c]])
        return Piece(verts, faces, "steel", "hand")
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


def build(hand, detail="cursor"):
    """Every piece of the gauntlet on `hand`, at rest, at a level of detail (`LOD`)."""
    lod = LOD[detail]
    up = Vector((0, 0, 1))
    pieces = _cuff(hand, lod)
    pieces.append(_back_plate(hand, lod))
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
            s = Vector((0.6 * hand.thumb_x, 0, 1))
            fup = (s - a * s.dot(a)).normalized()
        segs = len(f.joints) - 1
        for s in range(segs):
            p0, p1 = f.joints[s], f.joints[s + 1]
            r = f.radius[s] * 1.05 + 0.002
            d = p1 - p0
            last = s == segs - 1
            # Leather under the plates, a little past each joint.
            tube = _tube(p0 - d * 0.08, p1 + d * 0.04, fup, r * 0.92, r * 0.85, lod["tube"], caps=lod["solid"] or last)
            pieces.append(Piece(*tube, "leather", (f.name, s)))
            start = p0 - d * (0.12 if s else 0.05)
            end = p1 + (d * 0.02 if last else d * 0.10)
            sv, sf = _shell(start, end, fup, r + 0.0028, (r + 0.0018) * (0.9 if last else 1.0), 250, 0.0018,
                            n_arc=lod["arc"], ridge=0.0011, flare=0.0 if last else 0.0012, tip=r * 1.6 if last else 0.0,
                            rings=lod["rings"], solid=lod["solid"])
            pieces.append(Piece(sv, sf, "steel", (f.name, s)))
        # A knuckle plate over the finger's root, on the hand; the thumb's rides the thumb and
        # closes its lame's open root.
        k = f.joints[0]
        a = (f.joints[1] - f.joints[0]).normalized()
        r = f.radius[0] * (1.25 if thumb else 1.6) + 0.004
        kv, kf = _shell(k - a * 0.011, k + a * 0.012, fup, r, r * 0.95, 300 if thumb else 190, 0.002, n_arc=lod["arc"],
                        ridge=0.002, flare=0.001, rings=lod["rings"], solid=lod["solid"])
        pieces.append(Piece(kv, kf, "steel", (f.name, 0) if thumb else "hand"))
    return pieces


def tip(hand, bends, thumb=None):
    """Where the index finger's point is with the hand posed by `bends` (the hand's frame)."""
    f = hand.finger("index")
    r = f.radius[-1] * 1.05 + 0.002
    d = f.joints[-1] - f.joints[-2]
    a = d.normalized()
    up = Vector((0, 0, 1))
    n = (up - a * up.dot(a)).normalized()
    p = f.joints[-1] + d * 0.02 + a * (r * 1.6) + n * (r + 0.0018) * 0.9 * 0.25
    return _posed(hand, "index", len(f.joints) - 2, [p], bends, thumb or THUMB)[0]


# --- Posing ------------------------------------------------------------------------------------

# The cursor's pose, a right hand pointing (as the owner showed it): each finger's joints' curl
# toward the palm (degrees): the index straight, the others closed in a fist, so only their
# knuckles show from the back; the thumb's two joints.
POINTING = {"index": (2, 0, 0), "middle": (92, 100, 70), "ring": (95, 100, 70), "little": (98, 100, 70),
            "thumb": (6, 10)}
# The thumb is tucked under the palm before it curls: swung down under it (about the hand's
# length) and in toward the fingers (about the back of the hand's axis), degrees.
THUMB = {"under": 18, "across": 36}


def _turn(p, at, axis, deg):
    return at + Quaternion(axis, math.radians(deg)) @ (p - at)


def _posed(hand, name, s, vs, bends, thumb):
    """`vs` on segment `s` of finger `name`, posed: its joints curled (the far ones first), and
    the thumb tucked under the palm."""
    f = hand.finger(name)
    angles = bends.get(name, (0,) * (len(f.joints) - 1))
    for k in range(s, -1, -1):
        vs = [_turn(v, f.joints[k], f.bend, angles[k]) for v in vs]
    if name == "thumb":
        root = f.joints[0]
        # Under the palm: the thumb's side of the hand turns down (toward -Z).
        under = Vector((0, hand.thumb_x, 0))
        vs = [_turn(v, root, under, thumb["under"]) for v in vs]
        # In toward the fingers: about the back of the hand's axis, the thumb's side toward the
        # middle of the palm.
        across = Vector((0, 0, hand.thumb_x))
        vs = [_turn(v, root, across, thumb["across"]) for v in vs]
    return vs


def pose(hand, pieces, bends, thumb=None):
    """The pieces' vertices with the hand posed by `bends` (and the thumb tucked by `thumb`, as
    `THUMB`). Returns a list of vertex lists, piece by piece."""
    thumb = thumb or THUMB
    out = []
    for pc in pieces:
        vs = [v.copy() for v in pc.verts]
        if isinstance(pc.seg, tuple):
            vs = _posed(hand, pc.seg[0], pc.seg[1], vs, bends, thumb)
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


# --- The game's part --------------------------------------------------------------------------

# The part's colors (its paint regions): steel, gilt trim, leather.
COLORS = {"steel": (0.62, 0.64, 0.68), "gold": (0.80, 0.60, 0.24), "leather": (0.17, 0.10, 0.06)}


# How far the game's glove stands off the skin (m).
GLOVE = 0.0025


def game_part(armature, body, pid="arms.gauntlets"):
    """The gauntlets as the base's catalog part `pid`, a pair, fitted to each hand by
    construction:

    - **the glove** is the hand's own surface lifted `GLOVE` off the skin, with the hand's own
      weights, so it follows every finger as the hand does: steel on the back and the fingers,
      leather on the palm;
    - **the cuff** (gilt at its rim and the wrist) and **the back plate** ride on it, built as
      the cursor's are (the game's detail): the cuff on the forearm, half on the hand at the
      wrist, the plate on the hand.

    Steel and gilt are on the `metal` slot, leather on `cloth`. Returns its `made` entry (as
    `parts.build_parts`')."""
    from . import body_rig, mesh, parts
    bm = bmesh.new()
    deform = bm.verts.layers.deform.verify()
    col = bm.verts.layers.float_color.new("Col")
    groups = {}

    def group(name):
        if name not in groups:
            groups[name] = len(groups)
        return groups[name]

    slots = mesh.MATERIAL_SLOTS
    helpers = set(body_rig.NAMES)
    me = body.data
    names = {g.index: g.name for g in body.vertex_groups}
    for side in ("r", "l"):
        hand = Hand(armature, body, side)
        back = hand.frame.col[2].normalized()  # the back of the hand, in the rig's frame
        # The glove.
        made = {}
        for i in hand.verts:
            v = me.vertices[i]
            nv = bm.verts.new(v.co + v.normal * GLOVE)
            for g in v.groups:
                n = names[g.group]
                if g.weight > 0.0 and n not in helpers:
                    nv[deform][group(n)] = g.weight
            made[i] = nv
        for p in me.polygons:
            if not all(i in made for i in p.vertices):
                continue
            try:
                face = bm.faces.new([made[i] for i in p.vertices])
            except ValueError:
                continue
            palm = p.normal.dot(back) < -0.35
            kind = "leather" if palm else "steel"
            face.material_index = slots.index("cloth" if palm else "metal")
            face.smooth = False
            for v in face.verts:
                v[col] = (*COLORS[kind], 1.0)
        # The cuff and the back plate.
        lod = LOD["game"]
        for pc in _cuff(hand, lod) + [_back_plate(hand, lod)]:
            vs = []
            for v in pc.verts:
                nv = bm.verts.new(hand.to_rig(v))
                if pc.seg == "forearm":
                    t = min(max((v.y + 0.02) / 0.03, 0.0), 1.0)
                    w = [(f"forearm_{side}", 1.0 - 0.5 * t), (f"hand_{side}", 0.5 * t)]
                else:
                    w = [(f"hand_{side}", 1.0)]
                for n, x in w:
                    if x > 0.0:
                        nv[deform][group(n)] = x
                nv[col] = (*COLORS[pc.kind], 1.0)
                vs.append(nv)
            faces = []
            for f in pc.faces:
                try:
                    faces.append(bm.faces.new([vs[i] for i in f]))
                except ValueError:
                    continue
            for face in faces:
                face.material_index = slots.index("metal")
                face.smooth = False
            if pc.seg == "forearm":
                # Closed solids: outward.
                bmesh.ops.recalc_face_normals(bm, faces=faces)
            else:
                # The plate, one-sided: facing out of the back of the hand.
                for face in faces:
                    face.normal_update()
                    if face.normal.dot(back) < 0:
                        face.normal_flip()
    out = bpy.data.meshes.new(f"part.{pid}")
    bm.to_mesh(out)
    bm.free()
    obj = bpy.data.objects.new(f"part.{pid}", out)
    bpy.context.scene.collection.objects.link(obj)
    for name in sorted(groups, key=groups.get):
        obj.vertex_groups.new(name=name)
    for sl in slots:
        out.materials.append(mesh.material(sl))
    out.color_attributes.active_color = out.color_attributes["Col"]
    out.color_attributes.render_color_index = out.color_attributes.active_color_index
    obj.parent = armature
    mod = obj.modifiers.new("Armature", "ARMATURE")
    mod.object = armature
    body_rig.transfer(body, obj)
    n = parts.transfer_shape_keys(body, obj)
    return (pid, "arms", "Gauntlets", [], False, mesh.triangle_count(obj), n,
            {"covers": True, "rigid": True, "layer": "armor"})
