# SPDX-License-Identifier: AGPL-3.0-or-later
"""Spring-bone chains (22 §4.5, BW6): the bones a hanging part swings on.

A part that hangs (long hair, a ponytail, braids, a skirt, a coat's skirts) gets chains of its
own: `extra_<part>_<chain>_<n>` bones (10 §7.2) laid along where it settled at build time, under
the bone it hangs from (the head, the pelvis). Its hanging vertices are weighted onto them, more
and more toward the tip; the rest keep the body's weights. No clip keys these bones: the client
swings them with Godot's spring bones, with the body as colliders (`client/scripts/springs.gd`).

The base carries every part's chains; the forge keeps the worn parts' only (`mftr_forge`), at
most `EXTRA_BONES_MAX` of them.

- **Hair** (`hang_locks`): a lock set marked `chain` in its pattern (`grooming.Locks`) hangs on
  chains. A tail (a ponytail, a braid, a beard's braid) is one chain along its locks' mean line,
  from its root. Loose long hair (`split`) is split into a chain on each side and one down the
  back, each from where its locks leave the skull.
- **Cloth** (`hang_cloth`): `chains` chains round the hips (`item.ron`: `chains`, `chain_turn` in
  degrees from the front, `swing`: how much of the hem the chains carry), from the hip joints
  down to the hem; each vertex blends the two chains either side of it. A cape's hang from the
  chest instead (`chain_from`), across the back (`chain_arc` degrees, not all round).
"""

import math

import bpy
from mathutils import Vector

# A chain's bones, by its length: one a `BONE` (m), two at least and three at most.
BONE = 0.09
BONES = (2, 3)
# Points a chain's path is resampled to (along its length).
SAMPLES = 24


def name(part_id, label):
    """A chain's name for the part `part_id` (`hair.long`, `l`): `hair_long_l`."""
    return f"{part_id.replace('.', '_')}_{label}"


def _resample(pts, n=SAMPLES):
    """`pts` (a polyline) at `n` points evenly along it, and its length."""
    lens = [0.0]
    for a, b in zip(pts, pts[1:]):
        lens.append(lens[-1] + (b - a).length)
    total = lens[-1]
    if total < 1e-6:
        return [pts[0].copy() for _ in range(n)], 0.0
    out = []
    j = 0
    for i in range(n):
        s = total * i / (n - 1)
        while j < len(lens) - 2 and lens[j + 1] < s:
            j += 1
        seg = lens[j + 1] - lens[j]
        f = (s - lens[j]) / seg if seg > 1e-9 else 0.0
        out.append(pts[j].lerp(pts[j + 1], min(max(f, 0.0), 1.0)))
    return out, total


def _mean_path(paths):
    """The mean of polylines (each resampled), and its length."""
    rs = [_resample(p)[0] for p in paths if len(p) >= 2]
    if not rs:
        return [], 0.0
    mean = [sum((r[i] for r in rs), Vector()) / len(rs) for i in range(SAMPLES)]
    return _resample(mean)


def _joints(path, length, most=BONES[1]):
    """A chain's joint points along `path` (resampled), by its length."""
    n = min(max(round(length / BONE), BONES[0]), most)
    return [path[round(i * (len(path) - 1) / n)].copy() for i in range(n + 1)]


def add(armature, chain, parent, points):
    """`extra_<chain>_1..n` through `points` (the rig's frame) under `parent`; their names."""
    from . import rig
    inv = armature.matrix_world.inverted()
    rig.add_chain(armature, chain, parent, [inv @ p for p in points])
    return [f"extra_{chain}_{i}" for i in range(1, len(points))]


def along(u, bones, parent):
    """Weights for a point `u` of the way along a chain of `bones` (0 at its root, 1 at its tip):
    from bone centre to bone centre, and from the `parent` into the first over its first half."""
    n = len(bones)
    k = u * n - 0.5
    if k <= -0.5:
        return {parent: 1.0}
    if k < 0.0:
        f = (k + 0.5) * 2.0
        f = f * f * (3 - 2 * f)
        return {parent: 1.0 - f, bones[0]: f}
    i = min(int(k), n - 1)
    f = k - i
    if i >= n - 1:
        return {bones[-1]: 1.0}
    return {bones[i]: 1.0 - f, bones[i + 1]: f}


def reweigh(obj, weights, keep=2):
    """Give `obj`'s vertices new weights ({index: {bone: weight}}): their `keep` largest,
    normalized, in place of what they had. Vertices not given keep theirs."""
    names = {g.index: g.name for g in obj.vertex_groups}
    for vi, acc in weights.items():
        best = sorted(((w, n) for n, w in acc.items() if w > 1e-4), reverse=True)[:keep]
        tot = sum(w for w, _ in best) or 1.0
        v = obj.data.vertices[vi]
        for g in list(v.groups):
            obj.vertex_groups[names[g.group]].remove([vi])
        for w, n in best:
            g = obj.vertex_groups.get(n) or obj.vertex_groups.new(name=n)
            g.add([vi], w / tot, "REPLACE")


def _current(obj, vi, names):
    return {names[g.group]: g.weight for g in obj.data.vertices[vi].groups if g.weight > 0}


# --- Hair ---------------------------------------------------------------------------------------

def hang_locks(obj, armature, part_id, locks, top):
    """Chains for a grown hairstyle (`grooming.grow`): `locks` is [(Locks, centreline points,
    vertex indices with their t along the lock)] per lock, `top` the height (rig frame) below
    which loose hair hangs free. Each chain hangs from the bone its locks' roots follow most (the
    head; a beard's braid, the chin's). Weights its hanging vertices onto them; returns the
    chains' bone names."""
    skull = armature.matrix_world @ armature.data.bones["head"].head_local
    groups = {}
    for ls, centre, verts in locks:
        label = getattr(ls, "chain", None)
        if not label:
            continue
        if getattr(ls, "split", False):
            # Loose hair: from where it leaves the skull; on the side it falls.
            i0 = next((i for i, p in enumerate(centre) if p.z < top), None)
            if i0 is None or i0 > len(centre) - 3:
                continue
            free = centre[i0:]
            t0 = i0 / (len(centre) - 1)
            p = free[0] - skull
            side = math.degrees(math.atan2(p.x, p.y))  # 0 down the back (+y), + to the left (+x)
            sub = "b" if abs(side) < 50.0 else ("l" if side > 0 else "r")
            groups.setdefault(f"{label}_{sub}" if label != "hang" else sub, []).append((free, t0, verts))
        else:
            groups.setdefault(label, []).append((centre, 0.0, verts))
    made = []
    weights = {}
    names = {g.index: g.name for g in obj.vertex_groups}
    for label, members in sorted(groups.items()):
        path, length = _mean_path([m[0] for m in members])
        if length < 0.06:
            continue
        rooted = {}
        for _free, t0, verts in members:
            for vi, t in verts:
                if t <= t0 + 0.08:
                    for n, w in _current(obj, vi, names).items():
                        if not n.startswith(("extra_", "body_")):
                            rooted[n] = rooted.get(n, 0.0) + w
        parent = max(rooted, key=rooted.get) if rooted else "head"
        bones = add(armature, name(part_id, label), parent, _joints(path, length))
        made += bones
        for _free, t0, verts in members:
            for vi, t in verts:
                if t <= t0:
                    continue
                u = (t - t0) / max(1.0 - t0, 1e-6)
                chain = along(u, bones, parent)
                if parent in chain and chain[parent] > 0.0:
                    # Its own weights (the head's, the face's) toward the root.
                    k = chain.pop(parent)
                    for n, w in _current(obj, vi, names).items():
                        if not n.startswith(("extra_", "body_")):
                            chain[n] = chain.get(n, 0.0) + w * k
                weights[vi] = chain
    # A face vertex may blend the head and three face bones (21 §1.4): with its chain, four.
    reweigh(obj, weights, keep=4)
    if made:
        from . import face_rig
        face_rig.limit_steps(obj)
    return made


# --- Cloth --------------------------------------------------------------------------------------

def hang_cloth(obj, armature, part_id, count, turn=45.0, swing=1.0, parent="pelvis", arc=None):
    """`count` chains for a garment fitted on the base (`wardrobe.fit_items`), hanging from
    `parent`: from the hip joints' height (the pelvis's) or the bone's own to the garment's hem,
    centred `turn` degrees round from the front, evenly all round or across `arc` degrees.
    Weights its vertices below that onto them, the chains carrying `swing` of the hem; returns
    the chains' bone names."""
    bones_ = armature.data.bones
    pel = armature.matrix_world @ bones_[parent].head_local
    top = (armature.matrix_world @ bones_["thigh_l" if parent == "pelvis" else parent].head_local).z
    me = obj.data
    cos = [v.co.copy() for v in me.vertices]
    bottom = min(c.z for c in cos)
    if top - bottom < 0.1:
        return []

    def azimuth(p):
        # 0 at the front (-y), + to the left (+x).
        return math.degrees(math.atan2(p.x - pel.x, -(p.y - pel.y)))

    def gap(a, b):
        return (a - b + 180.0) % 360.0 - 180.0

    if arc:
        spots = [turn + arc * (k / max(count - 1, 1) - 0.5) for k in range(count)]
    else:
        spots = [turn + 360.0 * k / count for k in range(count)]
    angles = [((a + 180.0) % 360.0) - 180.0 for a in spots]
    chains = []
    made = []
    bands = 8
    for k, a in enumerate(angles):
        pts = []
        for b in range(bands + 1):
            z = top - (top - bottom) * b / bands
            near = [c for c in cos if abs(c.z - z) < (top - bottom) / bands * 0.6 and abs(gap(azimuth(c), a)) < 22.0]
            if near:
                pts.append(sum(near, Vector()) / len(near))
        if len(pts) < 3:
            chains.append(None)
            continue
        path, length = _resample(pts)
        label = ("f" if abs(a) < 67.5 else "b" if abs(a) > 112.5 else "") + (
            "" if abs(a) < 22.5 or abs(a) > 157.5 else "l" if a > 0 else "r")
        # Two bones a chain: a coat, a skirt and hair together stay within the extras' cap.
        bones = add(armature, name(part_id, label), parent, _joints(path, length, most=2))
        made += bones
        chains.append(bones)
    names = {g.index: g.name for g in obj.vertex_groups}
    weights = {}
    for vi, c in enumerate(cos):
        if c.z >= top:
            continue
        u = (top - c.z) / (top - bottom)
        az = azimuth(c)
        # The two chains either side.
        order = sorted(((abs(gap(az, a)), k) for k, a in enumerate(angles) if chains[k]), key=lambda t: t[0])[:2]
        if not order:
            continue
        span = arc / max(count - 1, 1) if arc else 360.0 / count
        (d0, k0), (d1, k1) = order[0], order[-1]
        if len(order) == 2 and d0 + d1 <= span * 1.05:
            # Between the two: by how near each is.
            w0 = d1 / max(d0 + d1, 1e-6)
            pair = [(k0, w0), (k1, 1.0 - w0)]
        else:
            # Past the last chain of an arc: on it.
            pair = [(k0, 1.0)]
        own = _current(obj, vi, names)
        acc = {}
        for k, wk in pair:
            for n, w in along(u, chains[k], "__own__").items():
                if n == "__own__":
                    continue
                acc[n] = acc.get(n, 0.0) + w * wk * swing
        given = sum(acc.values())
        for n, w in own.items():
            acc[n] = acc.get(n, 0.0) + w * (1.0 - given)
        weights[vi] = acc
    reweigh(obj, weights, keep=4)
    return made
