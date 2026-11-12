# SPDX-License-Identifier: AGPL-3.0-or-later
"""The body's helper bones (21 §9, FR6): 24 leaves under the biped bones that carry the body's
mass, and the body's and parts' weights to them.

A body is sculpted the way a face is: by moving and scaling these bones, which the forge bakes
into their rest (`mftr_forge::body`). Clips never key them, and each is a leaf, so a scale never
cascades. Each helper is aligned with its parent (limbs: along the limb; the torso: x across,
y up, z forward) and rolled as the rig rolls its bones (`rig`), so a right helper's frame is the
left one's mirror and a delta mirrors by flipping x (`BoneDelta::mirrored`).

The weights follow the face's rule (21 §1.4): a smooth falloff from each helper's core,
overlapping helpers sharing a vertex by falloff³ (the nearest mass wins), smoothed until no edge
changes a helper's weight by more than `MAX_STEP`; the biped bone keeps the rest, and with it
what bends at the joints. A helper never weighs on a joint's crease: limb helpers fade to nothing
before each end of their bone, and torso helpers keep to the torso.

The bone names and parents must match `crates/mftr-pack/src/body.rs` (`BODY_RIG`).
"""

import math

import bpy
from mathutils import Vector
from mathutils.kdtree import KDTree

from . import rig

# (name, parent) in rig order. Matches mftr_pack::body::BODY_RIG.
BODY_RIG = [
    ("body_neck", "neck"),
    ("body_trap_l", "clavicle_l"), ("body_trap_r", "clavicle_r"),
    ("body_delt_l", "upperarm_l"), ("body_delt_r", "upperarm_r"),
    ("body_pec_l", "chest"), ("body_pec_r", "chest"),
    ("body_lat_l", "spine_02"), ("body_lat_r", "spine_02"),
    ("body_belly", "spine_01"),
    ("body_waist_l", "spine_01"), ("body_waist_r", "spine_01"),
    ("body_hip_l", "pelvis"), ("body_hip_r", "pelvis"),
    ("body_glute_l", "pelvis"), ("body_glute_r", "pelvis"),
    ("body_bicep_l", "upperarm_l"), ("body_bicep_r", "upperarm_r"),
    ("body_forearm_l", "forearm_l"), ("body_forearm_r", "forearm_r"),
    ("body_thigh_l", "thigh_l"), ("body_thigh_r", "thigh_r"),
    ("body_calf_l", "calf_l"), ("body_calf_r", "calf_r"),
]
NAMES = {n for n, _ in BODY_RIG}
TORSO = ("pelvis", "spine_01", "spine_02", "chest")

# Limb helpers: a window along the parent bone (t from its head, 0, to its tail, 1): full over
# [core0, core1], fading to nothing at end0 and end1, inside the bone, so the joints (elbow,
# knee, hip and shoulder creases) keep to the biped bones. `out` (the delt) keeps to the side
# away from the body. (parent kind, end0, core0, core1, end1, bones it may weigh on)
LIMB = {
    "body_delt": ("upperarm", -0.25, -0.02, 0.14, 0.42, ("upperarm", "chest", "clavicle")),
    "body_bicep": ("upperarm", 0.10, 0.34, 0.66, 0.92, ("upperarm",)),
    "body_forearm": ("forearm", 0.06, 0.22, 0.52, 0.86, ("forearm",)),
    "body_thigh": ("thigh", 0.04, 0.28, 0.66, 0.94, ("thigh",)),
    "body_calf": ("calf", 0.06, 0.26, 0.58, 0.90, ("calf",)),
    "body_neck": ("neck", -0.30, 0.25, 2.5, 3.5, ("neck",)),
}

# Torso helpers: a patch about a point on the torso's surface at a height (t along `bone`) and
# an angle around it (0 the front, 90 the character's left side, 180 the back), reaching
# (across, depth, height) metres; and its pivot (what it scales about): `axis` on the spine at
# that height, or `under` the patch on the body's middle plane (x kept, y 0).
# (bone, t, angle, reach, pivot, bones it may weigh on)
PATCH = {
    "body_pec": ("chest", 0.40, 30, (0.16, 0.16, 0.17), "under", ("chest", "spine_02")),
    "body_lat": ("spine_02", 0.70, 138, (0.20, 0.17, 0.19), "axis", ("spine_02", "chest", "spine_01")),
    "body_belly": ("spine_01", 0.60, 0, (0.20, 0.16, 0.17), "axis", ("spine_01", "spine_02", "pelvis")),
    "body_waist": ("spine_01", 0.55, 90, (0.16, 0.18, 0.13), "axis", ("spine_01", "spine_02", "pelvis")),
    "body_hip": ("pelvis", 0.25, 90, (0.16, 0.18, 0.14), "axis", ("pelvis", "thigh")),
    # Below the hip joints on the CC0 body (22 §1): the buttocks' mass sits low.
    "body_glute": ("pelvis", -0.35, 158, (0.17, 0.18, 0.17), "under", ("pelvis", "thigh")),
    # The trapezius: the slope from the neck to the shoulder, on top of the chest.
    "body_trap": ("clavicle", 0.45, None, (0.10, 0.12, 0.08), "under", ("chest", "neck", "clavicle")),
}



def _allowed():
    out = {}
    for name, _ in BODY_RIG:
        kind, side = _kind(name), _side(name)
        bones = LIMB[kind][5] if kind in LIMB else PATCH[kind][5]
        out[name] = {_sided(b, side) for b in bones}
    return out


# Per helper, the biped bones whose vertices it may weigh on (the body's and the parts').
ALLOWED = None

# A helper never takes all of a vertex: its bone keeps some, and with it the joint's bend.
PEAK = 0.9
SHARPEN = 3.0
# Along any edge, no helper's weight changes by more than this (mftr_pack's limit is 0.35; a
# margin for the exporter's quantization).
MAX_STEP = 0.32
# Parts take the body's helper weights from its surface within this distance (m), fading out to
# `PART_REACH`: a cape's hem far below the back takes none.
PART_NEAR = 0.05
PART_REACH = 0.14


def _kind(name):
    return name[:-2] if name.endswith(("_l", "_r")) else name


def _side(name):
    return name[-1] if name.endswith(("_l", "_r")) else ""


def _sided(bone, side):
    return f"{bone}_{side}" if side and bone not in TORSO + ("neck",) else bone


def _dominant(body):
    """Each body vertex's own bone: the group with the most weight."""
    names = {g.index: g.name for g in body.vertex_groups}
    out = []
    for v in body.data.vertices:
        best = max(v.groups, key=lambda g: g.weight, default=None)
        out.append(names[best.group] if best is not None else "")
    return out


def _surface(cos, own, bones, z, direction):
    """The torso's surface at height `z` toward `direction` (horizontal): the vertex of `bones`
    near that height furthest along it."""
    best, far = None, -1e9
    for c, b in zip(cos, own):
        if b in bones and abs(c.z - z) < 0.03:
            d = c.x * direction.x + c.y * direction.y
            if d > far:
                best, far = c, d
    return best


def _towards(angle, side):
    a = math.radians(angle) * (-1 if side == "r" else 1)
    return Vector((math.sin(a), -math.cos(a), 0.0))


def landmarks(armature, body):
    """Per helper: (pivot, centre of its patch or None) in armature space."""
    bones = armature.data.bones
    cos = [v.co.copy() for v in body.data.vertices]
    own = _dominant(body)
    out = {}
    for name, parent in BODY_RIG:
        kind, side = _kind(name), _side(name)
        pb = bones[parent]
        a, b = pb.head_local, pb.tail_local
        if kind in LIMB:
            _, _, c0, c1, _, _ = LIMB[kind]
            out[name] = (a.lerp(b, (c0 + c1) / 2 if kind != "body_neck" else 0.5), None)
            continue
        bone, t, angle, _reach, pivot, allowed = PATCH[kind]
        src = bones[_sided(bone, side)]
        at = src.head_local.lerp(src.tail_local, t)
        if angle is None:
            # The trapezius: the top of the shoulder line, above the clavicle.
            top = None
            for c, o in zip(cos, own):
                if o in ("chest", "neck") and abs(c.x - at.x) < 0.02 and abs(c.y - at.y) < 0.05:
                    if top is None or c.z > top.z:
                        top = c
            centre = Vector((at.x, at.y, top.z if top else at.z + 0.04))
            out[name] = (Vector((at.x, 0.0, centre.z - 0.05)), centre)
            continue
        group = [_sided(x, side) for x in allowed]
        centre = _surface(cos, own, set(group), at.z, _towards(angle, side)) or at
        piv = Vector((0.0, 0.0, centre.z)) if pivot == "axis" else Vector((centre.x, 0.0, centre.z))
        out[name] = (piv, centre)
    return out


def add_bones(armature, marks):
    """Add the helpers to `armature` at their pivots, each aligned with its parent (limbs) or
    upright (the torso, the trapezius), rolled as the rig rolls its bones."""
    prev = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    eb = armature.data.edit_bones
    for name, parent in BODY_RIG:
        p = eb[parent]
        b = eb.new(name)
        b.head = marks[name][0]
        along = (p.tail - p.head).normalized() if _kind(name) in LIMB else Vector((0, 0, 1))
        b.tail = b.head + along * 0.05
        b.align_roll(rig.FORWARD)
        b.use_deform = True
        b.parent = p
        b.use_connect = False
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev


def _falloff(u):
    u = max(0.0, min(1.0, u))
    return u * u * (3.0 - 2.0 * u)


def _torso_scale(armature):
    """The build's torso against the slim masculine one's: its shoulder joints' spread."""
    return max(0.8, min(1.35, armature.data.bones["upperarm_l"].head_local.x / 0.203))


def _raw(name, p, own, armature, marks):
    """Helper `name`'s falloff at `p` (a vertex of bone `own`), 0 off its bones; and whether
    the vertex is held at 0 (off its bones or past a joint)."""
    kind, side = _kind(name), _side(name)
    bones = armature.data.bones
    if kind in LIMB:
        parent_kind, e0, c0, c1, e1, allowed = LIMB[kind]
        if own not in {_sided(x, side) for x in allowed}:
            return 0.0, True
        pb = bones[_sided(parent_kind, side)]
        a, b = pb.head_local, pb.tail_local
        d = b - a
        t = (p - a).dot(d) / max(d.length_squared, 1e-9)
        if t <= e0 or t >= e1:
            return 0.0, True
        f = _falloff((t - e0) / (c0 - e0)) if t < c0 else (_falloff((e1 - t) / (e1 - c1)) if t > c1 else 1.0)
        if kind == "body_delt":
            # The cap of the shoulder: away from the body and up, never the armpit.
            on = a + d * max(0.0, min(1.0, t))
            radial = p - on
            if radial.length > 1e-6:
                out = Vector((1.0 if side == "l" else -1.0, 0.0, 0.9)).normalized()
                f *= _falloff((radial.normalized().dot(out) + 0.35) / 0.75)
            if own.startswith(("chest", "clavicle")):
                # The chest's own shoulder: only right at the joint, outside.
                f *= _falloff((abs(p.x) - abs(a.x) + 0.04) / 0.06)
        return f, False
    _bone, _t, _angle, reach, _pivot, allowed = PATCH[kind]
    if own not in {_sided(x, side) for x in allowed}:
        return 0.0, True
    # The patches were drawn for the slim masculine torso: they grow with the build's.
    k = _torso_scale(armature)
    reach = (reach[0] * k, reach[1] * k, reach[2] * k)
    centre = marks[name][1]
    q = p - centre
    if kind == "body_trap":
        # Only the top of the shoulders, from the neck out.
        if p.z < centre.z - reach[2] * 1.2:
            return 0.0, True
    d = math.sqrt((q.x / reach[0]) ** 2 + (q.y / reach[1]) ** 2 + (q.z / reach[2]) ** 2)
    return _falloff(1.0 - d), False


def _final(raw, cap=3):
    """At most `cap` helpers, the next one's weight taken off the top ones (no jump where a
    helper drops out), at most PEAK in all."""
    ws = sorted(((w, i) for i, w in enumerate(raw) if w > 1e-4), reverse=True)
    cut = ws[cap][0] if len(ws) > cap else 0.0
    ws = [(w - cut, i) for w, i in ws[:cap] if w > cut]
    total = sum(w for w, _ in ws)
    if total > PEAK:
        ws = [(w * PEAK / total, i) for w, i in ws]
    return ws


ALLOWED = _allowed()

SKIP = ("head", "jaw", "hand_", "fingers_", "thumb_", "foot_", "toe_", "face_", "prop_")


def weight(armature, body, marks):
    """Weight `body` to the helpers (see the module): falloffs, shares, smoothing to the step
    limit, at most three helpers a vertex, the vertex's bone keeping the rest."""
    names = [n for n, _ in BODY_RIG]
    groups = {g.name: g for g in body.vertex_groups}
    for n in names:
        if n not in groups:
            groups[n] = body.vertex_groups.new(name=n)
    me = body.data
    own = _dominant(body)
    # The head, hands and feet take no helper weight, but stay in the graph at nothing, so the
    # step limit holds across their edges too (the neck's top meets the head, 22 §1).
    verts = [v.index for v in me.vertices if own[v.index]]
    weld, ids = {}, {}
    for vi in verts:
        key = (own[vi],) + tuple(round(c, 4) for c in me.vertices[vi].co)
        ids[vi] = weld.setdefault(key, len(weld))
    n = len(weld)
    pos, bone = [None] * n, [""] * n
    for vi, w in ids.items():
        pos[w], bone[w] = me.vertices[vi].co.copy(), own[vi]
    nbrs = [set() for _ in range(n)]
    for poly in me.polygons:
        vs = [ids[v] for v in poly.vertices if v in ids]
        for a in vs:
            nbrs[a].update(b for b in vs if b != a)
    raw, held = [], []
    for w in range(n):
        f, h = [], []
        for nm in names:
            x, z = (0.0, 0.0) if bone[w].startswith(SKIP) else _raw(nm, pos[w], bone[w], armature, marks)
            f.append(x)
            h.append(z)
        top = max(f)
        sharp = [x ** SHARPEN for x in f]
        total = sum(sharp)
        raw.append([(x / total) * top * PEAK if total > 0 else 0.0 for x in sharp])
        held.append(h)

    def steepest(final):
        worst = 0.0
        for a in range(n):
            fa = {i: x for x, i in final[a]}
            for b in nbrs[a]:
                fb = {i: x for x, i in final[b]}
                for i in set(fa) | set(fb):
                    worst = max(worst, abs(fa.get(i, 0.0) - fb.get(i, 0.0)))
        return worst

    # The step limit: each helper's weight lowered (never raised) to the most it can be with
    # no edge rising more than the limit from a vertex held at nothing: peaks stay where the
    # mesh has room for them, and the creases stay clean. (Smoothing instead would flatten
    # every peak on the torso's 16 sides.)
    live = [i for i in range(len(names)) if any(r[i] > 0 for r in raw)]
    # Four influences a vertex in all (glTF's): the helpers get what the body's own bones leave
    # (one bone a vertex on the lofted body, two on the CC0 one, 22 §1.3).
    helper_set = set(names)
    caps = [3] * n
    for vi, w in ids.items():
        k = sum(1 for g in me.vertices[vi].groups if g.weight > 0 and body.vertex_groups[g.group].name not in helper_set)
        caps[w] = min(caps[w], max(1, 4 - k))
    final = [_final(r, caps[w]) for w, r in enumerate(raw)]
    rounds = 0
    step = MAX_STEP * 0.95
    for rounds in range(1, 12):
        if steepest(final) <= MAX_STEP:
            break
        changed = True
        while changed:
            changed = False
            for a in range(n):
                ra = raw[a]
                for b in nbrs[a]:
                    rb = raw[b]
                    for i in live:
                        if ra[i] > rb[i] + step:
                            ra[i] = rb[i] + step
                            changed = True
        final = [_final(r, caps[w]) for w, r in enumerate(raw)]
        step *= 0.9
    for vi, w in ids.items():
        if own[vi].startswith(SKIP):
            continue
        total = 0.0
        for x, i in final[w]:
            groups[names[i]].add([vi], x, "REPLACE")
            total += x
        # The vertex's own bones share what's left, as they shared the whole.
        mine = [(body.vertex_groups[g.group].name, g.weight) for g in me.vertices[vi].groups
                if g.weight > 0 and body.vertex_groups[g.group].name not in helper_set]
        whole = sum(x for _, x in mine) or 1.0
        for name, x in mine or [(own[vi], 1.0)]:
            groups[name].add([vi], x / whole * (1.0 - total), "REPLACE")
    peaks = {}
    for f in final:
        for x, i in f:
            peaks[names[i]] = max(peaks.get(names[i], 0.0), x)
    print(f"body rig: {n} body places weighted in {rounds} passes, steepest edge {steepest(final):.2f}; "
          + ", ".join(f"{k} {v:.2f}" for k, v in sorted(peaks.items()) if k.endswith(("_l", "neck", "belly"))))


def transfer(body, part):
    """Give `part` the body's helper weights from the body's surface under it (21 §9.2): each
    part vertex takes the inverse-distance mean of its four nearest body vertices' helper
    weights, full within PART_NEAR and fading out by PART_REACH, its own bones keeping the
    rest. Vertices on the head (hair, hats) and props take none."""
    me = body.data
    bnames = {g.index: g.name for g in body.vertex_groups}
    pts = []
    helpers = []
    for v in me.vertices:
        hs = {bnames[g.group]: g.weight for g in v.groups if bnames[g.group] in NAMES and g.weight > 0}
        pts.append(v.co.copy())
        helpers.append(hs)
    tree = KDTree(len(pts))
    for i, c in enumerate(pts):
        tree.insert(c, i)
    tree.balance()
    pnames = {g.index: g.name for g in part.vertex_groups}
    groups = {g.name: g for g in part.vertex_groups}
    moved = 0
    for v in part.data.vertices:
        mine = [(pnames[g.group], g.weight) for g in v.groups if g.weight > 0]
        if not mine or any(n.startswith(SKIP) for n, _ in mine):
            continue
        own = max(mine, key=lambda m: m[1])[0]
        hits = tree.find_n(v.co, 4)
        if not hits or hits[0][2] > PART_REACH:
            continue
        fade = 1.0 - _falloff((hits[0][2] - PART_NEAR) / (PART_REACH - PART_NEAR)) if hits[0][2] > PART_NEAR else 1.0
        acc, total = {}, 0.0
        for _co, i, d in hits:
            k = 1.0 / max(d, 1e-4)
            total += k
            for n, x in helpers[i].items():
                # Only the helpers the body lets weigh on this bone: a coat's hem by the calves
                # mustn't follow the calf.
                if own in ALLOWED[n]:
                    acc[n] = acc.get(n, 0.0) + x * k
        ws = sorted(((x / total * fade, n) for n, x in acc.items()), reverse=True)
        room = 4 - len(mine)
        cut = ws[room][0] if len(ws) > room else 0.0
        ws = [(x - cut, n) for x, n in ws[:room] if x - cut > 1e-3]
        if not ws:
            continue
        given = sum(x for x, _ in ws)
        own_total = sum(x for _, x in mine)
        for n, x in mine:
            groups[n].add([v.index], x * (1.0 - given) / own_total, "REPLACE")
        for x, n in ws:
            if n not in groups:
                groups[n] = part.vertex_groups.new(name=n)
            groups[n].add([v.index], x, "REPLACE")
        moved += 1
    if moved:
        limit_steps(part)
    return moved


def limit_steps(obj, max_step=MAX_STEP, rounds=60):
    """Smooth `obj`'s helper weights over its own mesh (split vertices welded, diagonals
    included) until no edge changes one by more than `max_step`, each vertex's own bones taking
    back what the helpers give up. For parts, whose faces are far bigger than the skin's."""
    idx = {g.index: g.name for g in obj.vertex_groups if g.name in NAMES}
    if not idx:
        return 0.0
    me = obj.data
    weld, ids = {}, []
    for v in me.vertices:
        ids.append(weld.setdefault(tuple(round(c, 4) for c in v.co), len(weld)))
    n = len(weld)
    names = sorted(idx.values())
    col = {nm: i for i, nm in enumerate(names)}
    w = [[0.0] * len(names) for _ in range(n)]
    for v in me.vertices:
        for x in v.groups:
            if x.group in idx:
                w[ids[v.index]][col[idx[x.group]]] = x.weight
    nbrs = [set() for _ in range(n)]
    for poly in me.polygons:
        vs = [ids[i] for i in poly.vertices]
        for a in vs:
            nbrs[a].update(b for b in vs if b != a)

    def steepest():
        return max((abs(w[a][k] - w[b][k]) for a in range(n) for b in nbrs[a] for k in range(len(names))), default=0.0)

    for _ in range(rounds):
        if steepest() <= max_step:
            break
        w = [[0.5 * w[a][k] + 0.5 * sum(w[b][k] for b in nbrs[a]) / len(nbrs[a]) for k in range(len(names))]
             if nbrs[a] else w[a] for a in range(n)]
    for v in me.vertices:
        a = ids[v.index]
        others = [(obj.vertex_groups[x.group].name, x.weight) for x in v.groups if x.group not in idx and x.weight > 0]
        if not others:
            continue
        hs = sorted(((w[a][k], names[k]) for k in range(len(names)) if w[a][k] > 1e-3), reverse=True)
        room = 4 - len(others)
        hs = hs[:room]
        given = min(PEAK, sum(x for x, _ in hs))
        scale = given / max(sum(x for x, _ in hs), 1e-9)
        own_total = sum(x for _, x in others)
        for x in list(v.groups):
            obj.vertex_groups[x.group].remove([v.index])
        for nm, x in others:
            obj.vertex_groups[nm].add([v.index], x * (1.0 - given) / own_total, "REPLACE")
        for x, nm in hs:
            obj.vertex_groups[nm].add([v.index], x * scale, "REPLACE")
    return steepest()


def rig_body(armature, body):
    """Both: the helpers at the body's landmarks, then the body's weights. Returns the marks."""
    marks = landmarks(armature, body)
    add_bones(armature, marks)
    weight(armature, body, marks)
    return marks
