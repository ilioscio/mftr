# SPDX-License-Identifier: AGPL-3.0-or-later
"""Faceted heads (10 §2): a sculpted parametric skull, the face (eyes with whites and pupils,
brows, nose, mouth, ears) and hair, shared by every model built with these tools.

The skull is a lofted surface whose cross-sections vary with height *and* angle: a V-shaped
jaw and chin, cheekbones, a flatter face, a fuller back of the skull. Hair follows the same
surface a little outside it, from a hairline that sits higher at the front than at the nape.
Everything is weighted rigidly to the `head` bone.

`Head` holds the knobs: the template uses the defaults, feminine heads narrow the jaw and the
chin and enlarge the eyes, and hair styles are `short`, `ponytail` or `bald`. A `beard` (a full
beard ending in a braid with an iron bead, and a moustache) and a `scar` across the left brow are
optional (Rook).
"""

import math

import bmesh
from mathutils import Euler, Matrix, Vector

SIDES = 16
# (t, half-width, front depth, back depth): t from under the chin (0) to the crown (1).
SKULL = [
    (0.00, .036, .040, .040),
    (0.06, .052, .072, .048),
    (0.14, .066, .090, .058),
    (0.25, .080, .098, .072),
    (0.38, .088, .102, .088),
    (0.50, .094, .100, .100),
    (0.60, .096, .098, .108),
    (0.70, .096, .097, .112),
    (0.80, .091, .088, .110),
    (0.90, .078, .070, .094),
    (0.96, .056, .048, .068),
]


class Head:
    def __init__(self, base=(0, -0.01, 1.63), height=0.25, scale=1.0, jaw=0.35, chin=1.0, eye=1.0,
                 skin=(0.86, 0.64, 0.50), hair=(0.24, 0.16, 0.11), hair_style="short",
                 eye_color=(0.16, 0.11, 0.08), lips=(0.62, 0.36, 0.32), beard=None, scar=False,
                 metal=(0.34, 0.35, 0.39)):
        self.base = Vector(base)
        self.height = height * scale
        self.scale = scale
        self.jaw = jaw        # how much the lower face narrows toward the chin (V-shape)
        self.chin = chin      # chin depth multiplier
        self.eye = eye        # eye size multiplier
        self.skin, self.hair, self.hair_style = skin, hair, hair_style
        self.eye_color, self.lips = eye_color, lips
        self.beard, self.scar, self.metal = beard, scar, metal

    def _radii(self, t):
        for (t0, w0, f0, b0), (t1, w1, f1, b1) in zip(SKULL, SKULL[1:]):
            if t <= t1:
                u = (t - t0) / (t1 - t0)
                return w0 + (w1 - w0) * u, f0 + (f1 - f0) * u, b0 + (b1 - b0) * u
        return SKULL[-1][1:]

    def surface(self, t, a):
        """The skull's surface at height `t` (0..1) and angle `a` (0 = the character's left,
        increasing toward the back), in armature space."""
        if t >= 1.0:
            return self.base + Vector((0, 0.01, self.height))
        w, f, b = self._radii(t)
        c, s = math.cos(a), math.sin(a)
        front = max(0.0, -s)
        if t < 0.3:
            w *= 1.0 - self.jaw * (1.0 - t / 0.3) * front
        if t < 0.2:
            f *= 1.0 + (self.chin - 1.0) * (1.0 - t / 0.2)
        depth = b if s > 0 else f
        return self.base + Vector((w * c * self.scale, depth * s * self.scale, t * self.height))

    def front(self, t, x):
        """A point on the face at height `t` and sideways offset `x` (m), slightly proud of it."""
        w, f, _ = self._radii(t)
        a = -math.pi / 2 + math.asin(max(-0.95, min(0.95, x / (w * self.scale))))
        p = self.surface(t, a)
        return p + Vector((0, -0.003, 0))


def _paint(bm, verts, mat, color, group, layers, faces=None):
    deform, col = layers
    for v in verts:
        v[deform][group] = 1.0
        v[col] = (*color, 1.0)
    for f in faces or {f for v in verts for f in v.link_faces}:
        f.material_index = mat
        f.smooth = False


def _box(bm, center, size, rot=(0, 0, 0)):
    m = Matrix.Translation(center) @ Euler([math.radians(r) for r in rot]).to_matrix().to_4x4() @ Matrix.Diagonal((*size, 1))
    return bmesh.ops.create_cube(bm, size=1.0, matrix=m)["verts"]


def _wedge(bm, base, tip, width, depth):
    """A three-sided pyramid from a triangle at `base` to `tip` (noses, bangs, ears)."""
    d = (tip - base).normalized()
    side = d.cross(Vector((0, 0, 1))).normalized() if abs(d.z) < 0.9 else Vector((1, 0, 0))
    up = side.cross(d).normalized()
    b0 = bm.verts.new(base + side * width)
    b1 = bm.verts.new(base - side * width)
    b2 = bm.verts.new(base + up * depth)
    t = bm.verts.new(tip)
    faces = [bm.faces.new(f) for f in ((b0, b2, t), (b2, b1, t), (b1, b0, t), (b0, b1, b2))]
    bmesh.ops.recalc_face_normals(bm, faces=faces)  # closed: point every face outward
    return [b0, b1, b2, t]


def _rings(bm, rings, close_top=True):
    """Quads between rings (each a list of SIDES verts, angle order), optionally capped."""
    n = len(rings[0])
    for r0, r1 in zip(rings, rings[1:]):
        for i in range(n):
            bm.faces.new((r0[i], r0[(i + 1) % n], r1[(i + 1) % n], r1[i]))
    if close_top:
        bm.faces.new(rings[-1])


def build(bm, layers, groups, mats, head: Head):
    """The skull, face and hair. `mats` maps slot names to material indices."""
    g = groups["head"]
    skin, cloth = mats["skin"], mats["cloth"]
    angles = [2 * math.pi * (i + 0.5) / SIDES for i in range(SIDES)]

    # Skull.
    rings = [[bm.verts.new(head.surface(t, a)) for a in angles] for t, *_ in SKULL]
    crown = bm.verts.new(head.surface(1.0, 0))
    _rings(bm, rings, close_top=False)
    for i in range(SIDES):
        bm.faces.new((rings[-1][i], rings[-1][(i + 1) % SIDES], crown))
    bm.faces.new(list(reversed(rings[0])))
    _paint(bm, [v for r in rings for v in r] + [crown], skin, head.skin, g, layers)

    # Face: eyes (whites and pupils), brows, nose, mouth.
    e = head.eye
    for x in (0.034, -0.034):
        sx = x * head.scale
        white = _box(bm, head.front(0.585, sx), Vector((0.026 * e, 0.006, 0.012 * e)) * head.scale)
        _paint(bm, white, skin, (0.93, 0.91, 0.87), g, layers)
        pupil = _box(bm, head.front(0.585, sx) + Vector((0, -0.003, 0)), Vector((0.011 * e, 0.004, 0.011 * e)) * head.scale)
        _paint(bm, pupil, cloth, head.eye_color, g, layers)
        brow = _box(bm, head.front(0.68, sx * 1.08) + Vector((0, -0.002, 0)), Vector((0.03, 0.007, 0.007)) * head.scale,
                    rot=(0, 5 if x < 0 else -5, 0))
        _paint(bm, brow, cloth, head.hair, g, layers)
    # The nose: a pyramid from the bridge and the two nostrils out to its tip.
    k = head.scale
    bridge = bm.verts.new(head.front(0.6, 0) + Vector((0, 0.004, 0)))
    nl = bm.verts.new(head.front(0.42, 0.015 * k) + Vector((0, 0.002, 0)))
    nr = bm.verts.new(head.front(0.42, -0.015 * k) + Vector((0, 0.002, 0)))
    tip = bm.verts.new(head.front(0.43, 0) + Vector((0, -0.024 * k, 0)))
    nose = [bm.faces.new(f) for f in ((bridge, nl, tip), (bridge, tip, nr), (nl, nr, tip), (bridge, nr, nl))]
    bmesh.ops.recalc_face_normals(bm, faces=nose)
    _paint(bm, [bridge, nl, nr, tip], skin, tuple(c * 0.95 for c in head.skin), g, layers)
    mouth = _box(bm, head.front(0.26, 0), Vector((0.036, 0.006, 0.0065)) * k)
    _paint(bm, mouth, skin, head.lips, g, layers)
    # Ears: flat plates against the sides of the head, level with the eyes and nose.
    for side in (1, -1):
        p = head.surface(0.5, 0 if side > 0 else math.pi) + Vector((0.004 * side, 0.012, 0))
        ear = _box(bm, p, Vector((0.012, 0.03, 0.052)) * k, rot=(0, 0, -12 * side))
        _paint(bm, ear, skin, tuple(c * 0.93 for c in head.skin), g, layers)

    if head.scar:
        # A pale scar from the left brow down across the cheek.
        p = head.front(0.6, 0.052 * k) + Vector((0, -0.003, 0))
        scar = _box(bm, p, Vector((0.006, 0.004, 0.07)) * k, rot=(0, -18, 0))
        _paint(bm, scar, skin, tuple(min(1.0, c * 1.12) for c in head.skin), g, layers)
    if head.beard:
        _beard(bm, layers, g, mats, head, angles)
    if head.hair_style == "bald":
        return

    # Hair: the skull's surface pushed out, from a hairline high at the front, low at the nape.
    def hairline(a):
        front = max(0.0, -math.sin(a))
        back = max(0.0, math.sin(a))
        return 0.47 + 0.33 * front - 0.12 * back + 0.06 * (1 - front - back)

    def out(p, push):
        c = head.base + Vector((0, 0.01, (p - head.base).z))
        d = p - c
        return c + d * push + Vector((0, 0, 0.006))

    steps = [0.0, 0.2, 0.42, 0.62, 0.8, 0.93]
    hair_rings = []
    for s in steps:
        ring = []
        for a in angles:
            t = hairline(a) + s * (0.985 - hairline(a))
            push = 1.07 + 0.05 * math.sin(math.pi * s) + (0.03 if head.hair_style == "ponytail" else 0.0)
            ring.append(bm.verts.new(out(head.surface(t, a), push)))
        hair_rings.append(ring)
    top = bm.verts.new(head.surface(1.0, 0) + Vector((0, 0, 0.014)))
    _rings(bm, hair_rings, close_top=False)
    for i in range(SIDES):
        bm.faces.new((hair_rings[-1][i], hair_rings[-1][(i + 1) % SIDES], top))
    # The hairline's underside, so the shell has thickness from below.
    inner = [bm.verts.new(out(head.surface(hairline(a), a), 1.0)) for a in angles]
    for i in range(SIDES):
        bm.faces.new((inner[i], inner[(i + 1) % SIDES], hair_rings[0][(i + 1) % SIDES], hair_rings[0][i]))
    hair_verts = [v for r in hair_rings for v in r] + [top] + inner
    _paint(bm, hair_verts, cloth, head.hair, g, layers)

    if head.hair_style == "short":
        # A few swept tufts at the hairline so short hair isn't a helmet.
        for x, drop in ((0.045, 0.0), (0.015, 0.02), (-0.018, 0.01), (-0.05, 0.0)):
            root = head.front(0.86, x * head.scale) + Vector((0, -0.008, 0.012))
            tip = head.front(0.77 - drop, (x + 0.02) * head.scale) + Vector((0, -0.016, 0))
            _paint(bm, _wedge(bm, root, tip, 0.016 * head.scale, 0.012), cloth, head.hair, g, layers)

    if head.hair_style == "ponytail":
        # Side-swept bangs across the forehead, and a ponytail from the crown's back.
        for i, x in enumerate((0.05, 0.02, -0.012, -0.045)):
            root = head.front(0.86, x * head.scale) + Vector((0, -0.006, 0.01))
            tip = head.front(0.68 - 0.03 * i, (x - 0.035) * head.scale) + Vector((0, -0.012, 0))
            _paint(bm, _wedge(bm, root, tip, 0.017 * head.scale, 0.012), cloth, head.hair, g, layers)
        tie = head.surface(0.72, math.pi / 2) + Vector((0, 0.035, 0))
        tail = [tie, tie + Vector((0, 0.06, -0.1)), tie + Vector((0, 0.085, -0.24)), tie + Vector((0, 0.08, -0.36))]
        radii = [0.034, 0.038, 0.026, 0.0]
        prev = None
        tail_verts = []
        for p, r in zip(tail, radii):
            if r == 0:
                # The tail runs downward, so its faces wind the other way round from an upward loft.
                apex = bm.verts.new(p)
                for i in range(6):
                    bm.faces.new((prev[(i + 1) % 6], prev[i], apex))
                tail_verts.append(apex)
                break
            ring = [bm.verts.new(p + Vector((r * math.cos(2 * math.pi * k / 6), 0, 0)) + Vector((0, r * 0.8 * math.sin(2 * math.pi * k / 6), 0)))
                    for k in range(6)]
            if prev is None:
                bm.faces.new(ring)
            else:
                for i in range(6):
                    bm.faces.new((prev[(i + 1) % 6], prev[i], ring[i], ring[(i + 1) % 6]))
            prev = ring
            tail_verts += ring
        _paint(bm, tail_verts, cloth, head.hair, g, layers)


def _beard(bm, layers, g, mats, head, angles):
    """A full beard over the jaw (thicker at the chin), a moustache and a braid with a bead."""
    cloth, metal = mats["cloth"], mats["metal"]
    k = head.scale

    def push(a):
        front = max(0.0, -math.sin(a))
        return 1.02 + 0.16 * front

    def out(t, a):
        p = head.surface(t, a)
        c = head.base + Vector((0, 0.01, (p - head.base).z))
        return c + (p - c) * push(a)

    rows = [0.36, 0.26, 0.15, 0.05]
    rings = [[bm.verts.new(out(t, a)) for a in angles] for t in rows]
    # Below the chin the beard gathers forward into the braid's root.
    chin = head.surface(0.0, -math.pi / 2)
    root = bm.verts.new(chin + Vector((0, -0.05 * k, -0.06 * k)))
    _rings(bm, rings, close_top=False)
    for i in range(SIDES):
        bm.faces.new((rings[-1][(i + 1) % SIDES], rings[-1][i], root))
    bm.faces.new(rings[0])
    verts = [v for r in rings for v in r] + [root]
    # Built top-down, so let bmesh point the closed shell's faces outward.
    bmesh.ops.recalc_face_normals(bm, faces=list({f for v in verts for f in v.link_faces}))
    _paint(bm, verts, cloth, head.hair, g, layers)
    # The moustache: two tapering wedges from under the nose out over the corners of the mouth.
    for x in (1, -1):
        r0 = head.front(0.36, 0.008 * x * k) + Vector((0, -0.016 * k, 0))
        tip = head.front(0.25, 0.05 * x * k) + Vector((0, -0.02 * k, 0))
        _paint(bm, _wedge(bm, r0, tip, 0.016 * k, 0.012 * k), cloth, head.hair, g, layers)
    # The braid: three tapering segments down from the chin, and an iron bead.
    top = chin + Vector((0, -0.05 * k, -0.06 * k))
    pts = [top, top + Vector((0, -0.012, -0.07)) * k, top + Vector((0, -0.006, -0.14)) * k, top + Vector((0, 0.004, -0.2)) * k]
    radii = [0.03, 0.024, 0.018, 0.0]
    prev = None
    braid = []
    for p, r in zip(pts, radii):
        if r == 0:
            apex = bm.verts.new(p)
            for i in range(6):
                bm.faces.new((prev[(i + 1) % 6], prev[i], apex))
            braid.append(apex)
            break
        ring = [bm.verts.new(p + Vector((r * k * math.cos(2 * math.pi * j / 6), r * k * 0.8 * math.sin(2 * math.pi * j / 6), 0))) for j in range(6)]
        if prev is None:
            bm.faces.new(ring)
        else:
            for i in range(6):
                bm.faces.new((prev[(i + 1) % 6], prev[i], ring[i], ring[(i + 1) % 6]))
        prev = ring
        braid += ring
    bmesh.ops.recalc_face_normals(bm, faces=list({f for v in braid for f in v.link_faces}))
    _paint(bm, braid, cloth, head.hair, g, layers)
    bead = _box(bm, pts[1] + Vector((0, 0, -0.035 * k)), Vector((0.034, 0.03, 0.026)) * k)
    _paint(bm, bead, metal, head.metal, g, layers)
