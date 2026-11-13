# SPDX-License-Identifier: AGPL-3.0-or-later
"""The face rig (21 §1.3, D76): 24 face bones under `head`, and the head's weights to them.

A face is sculpted by moving these bones (the forge bakes the moves into their rest), so a
base's head must be skinned to them. FR1 rigs the current block-out head with **placeholder
weights**: each bone takes the vertices near its pivot with a linear falloff wide enough that
no edge of the coarse head jumps more than `mftr_pack::face::MAX_WEIGHT_STEP`. FR2 replaces the
head with the authored one (21 §1.2), weighted by cores and falloff along its loops (21 §1.4).

The bone names and parents must match `crates/mftr-pack/src/face.rs` (`FACE_RIG`).
"""

import bpy
from mathutils import Vector

# (name, parent) in rig order: parents first. Matches mftr_pack::face::FACE_RIG.
FACE_RIG = [
    ("face_forehead", "head"), ("face_brow_mid", "head"), ("face_brow_l", "head"), ("face_brow_r", "head"),
    ("face_eye_l", "head"), ("face_eye_r", "head"), ("face_cheekbone_l", "head"), ("face_cheekbone_r", "head"),
    ("face_cheek_l", "head"), ("face_cheek_r", "head"), ("face_nose_bridge", "head"), ("face_nose_tip", "face_nose_bridge"),
    ("face_nostril_l", "face_nose_tip"), ("face_nostril_r", "face_nose_tip"), ("face_lip_upper", "head"), ("jaw", "head"),
    ("face_lip_lower", "jaw"), ("face_mouth_l", "head"), ("face_mouth_r", "head"), ("face_chin", "jaw"),
    ("face_jaw_angle_l", "jaw"), ("face_jaw_angle_r", "jaw"), ("face_ear_l", "head"), ("face_ear_r", "head"),
]
NAMES = {n for n, _ in FACE_RIG}
# The eyes' life bones (22 §2.3–2.4): at each eyeball's centre, under its face bone. The client
# turns them every frame (gaze, blinks); they are never sculpted. Matches mftr_pack::face::
# LIFE_BONES.
LIFE = [("eye_look_l", "face_eye_l"), ("eye_look_r", "face_eye_r"), ("lid_upper_l", "face_eye_l"),
        ("lid_upper_r", "face_eye_r"), ("lid_lower_l", "face_eye_l"), ("lid_lower_r", "face_eye_r")]
# Into the head from the surface, so a turned feature pivots inside it (m, before head scale).
INSET = 0.012


def pivots(head):
    """Each face bone's pivot (armature space) on `head` (a `head.Head`): the left side's; the
    right mirrors it."""
    k = head.scale

    def f(t, x=0.0, inset=INSET):
        return head.front(t, x * k) + Vector((0, inset * k, 0))

    left = {
        "face_forehead": f(0.8),
        "face_brow_mid": f(0.66),
        "face_brow_l": f(0.68, 0.037),
        "face_eye_l": f(0.585, 0.034, inset=0.02),
        "face_cheekbone_l": f(0.48, 0.06),
        "face_cheek_l": f(0.34, 0.055),
        "face_nose_bridge": f(0.58),
        "face_nose_tip": f(0.45),
        "face_nostril_l": f(0.42, 0.018),
        "face_lip_upper": f(0.29),
        "jaw": head.base + Vector((0, 0.02 * k, head.height * 0.4)),
        "face_lip_lower": f(0.235),
        "face_mouth_l": f(0.262, 0.032),
        "face_chin": f(0.08),
        "face_jaw_angle_l": head.surface(0.17, -0.55) + Vector((-0.01 * k, 0, 0)),
        "face_ear_l": head.surface(0.5, 0.0) + Vector((-0.01 * k, 0.01 * k, 0)),
    }
    out = {}
    for name, _ in FACE_RIG:
        if name.endswith("_r"):
            p = left[name[:-2] + "_l"]
            out[name] = Vector((-p.x, p.y, p.z))
        else:
            out[name] = left[name]
    return out


def mirrored(left):
    """The left side's pivots with the right side's mirrored in."""
    out = {}
    for name, _ in FACE_RIG:
        if name.endswith("_r"):
            p = left[name[:-2] + "_l"]
            out[name] = Vector((-p.x, p.y, p.z))
        else:
            out[name] = left[name]
    return out


def add_bones(armature, head, piv=None):
    """Add the face bones to `armature` (edit mode), pointing out of the face (-Y), 2 cm long
    (the jaw toward the chin), at `piv` (or the block-out head's). Returns the pivots."""
    piv = piv or pivots(head)
    prev = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    eb = armature.data.edit_bones
    for name, parent in FACE_RIG:
        b = eb.new(name)
        b.head = piv[name]
        if name == "jaw":
            b.tail = piv["face_chin"]
        else:
            b.tail = piv[name] + Vector((0, -0.02, 0))
        b.roll = 0.0
        b.use_deform = True
        b.parent = eb[parent]
        b.use_connect = False
    for name, parent in LIFE:
        b = eb.new(name)
        b.head = piv[parent]
        b.tail = piv[parent] + Vector((0, -0.02, 0))
        b.roll = 0.0
        b.use_deform = True
        b.parent = eb[parent]
        b.use_connect = False
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev
    return piv


# Reach per bone (m, before head scale): where its weight fades to zero. Tight enough that each
# feature's own bone owns it (the lips, the chin and the mouth corners are 2–4 cm apart).
REACH = {
    "face_forehead": 0.075, "face_brow_mid": 0.035, "face_brow": 0.04, "face_eye": 0.035, "face_cheekbone": 0.045,
    "face_cheek": 0.045, "face_nose_bridge": 0.035, "face_nose_tip": 0.035, "face_nostril": 0.025,
    "face_lip_upper": 0.03, "jaw": 0.05, "face_lip_lower": 0.03, "face_mouth": 0.028, "face_chin": 0.045,
    "face_jaw_angle": 0.05, "face_ear": 0.04,
}
# The jaw is a frame bone: it takes the jawline (from each jaw angle to the chin) at this strength,
# so the chin, lip and jaw-angle bones, at full strength, own their own features over it.
JAW_STRENGTH = 0.6
# Bones whose weight is full within this distance of the pivot (m, before head scale), the
# falloff starting there: the eyes, so the lids move with the eyeballs (bound to them whole).
CORE = {"face_eye": 0.022}
# Shares between overlapping bones go as falloff ** SHARPEN: the nearest feature wins.
SHARPEN = 3.0
SEAM_RINGS = 2
FADE_RINGS = 4
# A bone never takes all of a vertex: the head keeps some, so neighbouring bones blend.
PEAK = 0.85


# Along any edge, no face bone's weight may change by more than this (mftr_pack::face::
# MAX_WEIGHT_STEP is 0.35; a margin for the exporter's quantization).
MAX_STEP = 0.32


def _final(raw, cap=None):
    """A vertex's face weights as at most three bones, at most PEAK in all: the fourth-
    strongest weight is taken off the top three, so a bone fades to nothing exactly where it
    drops out of the three (a plain cut would jump there and pinch)."""
    ws = sorted(((w, i) for i, w in enumerate(raw) if w > 1e-4), reverse=True)
    cut = ws[3][0] if len(ws) > 3 else 0.0
    ws = [(w - cut, i) for w, i in ws[:3] if w > cut]
    total = sum(w for w, _ in ws)
    cap = PEAK if cap is None else cap
    if total > cap:
        ws = [(w * cap / total, i) for w, i in ws]
    return ws


def weight(armature, body, head, piv):
    """Weight the head's vertices of `body` to the face bones (placeholder, see the module):
    a linear falloff from each pivot over its reach, letting go toward the neck seam, then
    smoothed across the mesh until no edge changes a bone's weight by more than MAX_STEP, then
    at most three bones a vertex with the head keeping the rest."""
    k = head.scale
    groups = {g.name: g for g in body.vertex_groups}
    for name, _ in FACE_RIG:
        if name not in groups:
            groups[name] = body.vertex_groups.new(name=name)
    g_head = groups["head"]
    names = [n for n, _ in FACE_RIG]
    reach = [REACH.get(n, REACH.get(n[:-2], 0.08)) * k for n in names]
    core = [CORE.get(n, CORE.get(n[:-2], 0.0)) * k for n in names]
    me = body.data
    head_v = [v.index for v in me.vertices
              if next((g.weight for g in v.groups if g.group == g_head.index), 0.0) >= 0.99]
    # Weld the head's split vertices (color seams) by position: one weight per place.
    weld, ids = {}, {}
    for vi in head_v:
        key = tuple(round(c, 4) for c in me.vertices[vi].co)
        ids[vi] = weld.setdefault(key, len(weld))
    n = len(weld)
    pos = [None] * n
    for vi, w in ids.items():
        pos[w] = me.vertices[vi].co.copy()
    # Neighbours: every pair in a polygon, since the export triangulates quads (a diagonal is
    # an edge too).
    nbrs = [set() for _ in range(n)]
    for poly in me.polygons:
        vs = [ids[v] for v in poly.vertices if v in ids]
        for a in vs:
            for b in vs:
                if a != b:
                    nbrs[a].add(b)
    # Rings from the neck seam (21 §1.5): ring 1 touches a vertex the neck weighs on. Face bones
    # may weigh from ring SEAM_RINGS + 1 (mftr_pack::face::SEAM_RINGS) and fade in over FADE_RINGS.
    ring = [0] * n
    front = set()
    for poly in me.polygons:
        if any(v not in ids for v in poly.vertices):
            front.update(ids[v] for v in poly.vertices if v in ids)
    r = 1
    while front:
        for a in front:
            ring[a] = r
        r += 1
        front = {b for a in front for b in nbrs[a] if ring[b] == 0}
    # The falloff (smooth), letting go toward the seam (a cut would pinch the chin); the jaw's
    # along the jawline. Then the shares: the nearest feature wins, and the strongest falloff
    # sets how much of the vertex the face takes.
    def to_segment(p, a, b):
        ab = b - a
        t = max(0.0, min(1.0, (p - a).dot(ab) / max(ab.length_squared, 1e-12)))
        return (p - (a + ab * t)).length

    jaw_i = names.index("jaw")
    line = [piv["face_jaw_angle_l"], piv["face_chin"], piv["face_jaw_angle_r"]]
    raw, cap = [], []
    for w in range(n):
        p = pos[w]
        fade = max(0.0, min(1.0, (ring[w] - SEAM_RINGS) / FADE_RINGS)) if ring[w] else 1.0
        f = []
        for i, nm in enumerate(names):
            if i == jaw_i:
                d = min(to_segment(p, line[0], line[1]), to_segment(p, line[1], line[2]))
            else:
                d = max(0.0, (p - piv[nm]).length - core[i])
            u = max(0.0, 1.0 - d / reach[i])
            f.append(u * u * (3.0 - 2.0 * u) * (JAW_STRENGTH if i == jaw_i else 1.0))
        top = max(f)
        sharp = [x ** SHARPEN for x in f]
        total = sum(sharp)
        # Inside a core the bone takes the whole vertex, letting the head have its share back
        # over the falloff.
        c = PEAK + (1.0 - PEAK) * max((f[i] ** 4 for i in range(len(names)) if core[i] > 0), default=0.0)
        cap.append(c)
        raw.append([(x / total) * top * c * fade if total > 0 else 0.0 for x in sharp])
    seam = [0 < ring[w] <= SEAM_RINGS for w in range(n)]

    def steepest(final):
        worst = 0.0
        for a in range(n):
            fa = dict((i, x) for x, i in final[a])
            for b in nbrs[a]:
                fb = dict((i, x) for x, i in final[b])
                for i in set(fa) | set(fb):
                    worst = max(worst, abs(fa.get(i, 0.0) - fb.get(i, 0.0)))
        return worst

    final = [_final(r, c) for r, c in zip(raw, cap)]
    for _ in range(40):
        if steepest(final) <= MAX_STEP:
            break
        # One pass of smoothing: each place moves halfway to its neighbours' mean.
        smoothed = []
        for a in range(n):
            if seam[a] or not nbrs[a]:
                smoothed.append(raw[a])
                continue
            mean = [sum(raw[b][i] for b in nbrs[a]) / len(nbrs[a]) for i in range(len(names))]
            smoothed.append([0.5 * raw[a][i] + 0.5 * mean[i] for i in range(len(names))])
        raw = smoothed
        final = [_final(r, c) for r, c in zip(raw, cap)]
    for vi, w in ids.items():
        total = 0.0
        for x, i in final[w]:
            groups[names[i]].add([vi], x, "REPLACE")
            total += x
        g_head.add([vi], 1.0 - total, "REPLACE")
    print(f"face rig: {n} head places weighted, steepest edge {steepest(final):.2f}")


def limit_steps(obj, max_step=MAX_STEP, rounds=60):
    """Smooth `obj`'s face-bone weights over its own mesh until no edge (diagonals included,
    split vertices welded) changes a face bone's weight by more than `max_step` (21 §1.4). Each
    vertex keeps its head-plus-face share (the head takes back what the face bones give up),
    and its other bones as they were. For parts, whose faces are far bigger than the skin's."""
    face_idx = {g.index: g.name for g in obj.vertex_groups if g.name in NAMES}
    head = obj.vertex_groups.get("head")
    if not face_idx or head is None:
        return 0.0
    me = obj.data
    weld, ids = {}, []
    for v in me.vertices:
        ids.append(weld.setdefault(tuple(round(c, 4) for c in v.co), len(weld)))
    n = len(weld)
    names = sorted(face_idx.values())
    col = {nm: i for i, nm in enumerate(names)}
    w = [[0.0] * len(names) for _ in range(n)]
    share = [0.0] * n
    for v in me.vertices:
        for x in v.groups:
            if x.group in face_idx:
                w[ids[v.index]][col[face_idx[x.group]]] = x.weight
            if x.group in face_idx or x.group == head.index:
                share[ids[v.index]] = max(share[ids[v.index]], 0.0)
        share[ids[v.index]] = sum(x.weight for x in v.groups if x.group in face_idx or x.group == head.index)
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
        nw = []
        for a in range(n):
            if not nbrs[a] or share[a] <= 0.0:
                nw.append(w[a])
                continue
            mean = [sum(w[b][k] for b in nbrs[a]) / len(nbrs[a]) for k in range(len(names))]
            row = [0.5 * w[a][k] + 0.5 * mean[k] for k in range(len(names))]
            total = sum(row)
            if total > share[a] * PEAK:
                row = [x * share[a] * PEAK / total for x in row]
            nw.append(row)
        w = nw
    for v in me.vertices:
        a = ids[v.index]
        if share[a] <= 0.0:
            continue
        others = [(obj.vertex_groups[x.group].name, x.weight) for x in v.groups
                  if x.group not in face_idx and x.group != head.index]
        faces = sorted(((w[a][k], names[k]) for k in range(len(names)) if w[a][k] > 1e-4), reverse=True)
        room = 3 - min(len(others), 2)
        cut = faces[room][0] if len(faces) > room else 0.0
        faces = [(x - cut, nm) for x, nm in faces[:room] if x > cut]
        hw = max(0.0, share[a] - sum(x for x, _ in faces))
        for x in list(v.groups):
            obj.vertex_groups[x.group].remove([v.index])
        for nm, x in others[:2] + [("head", hw)] + [(nm, x) for x, nm in faces]:
            if x > 0.0:
                obj.vertex_groups[nm].add([v.index], x, "REPLACE")
    return steepest()


def rig(armature, body, head, piv=None):
    """Both: bones, then the head's weights. Returns the pivots."""
    piv = add_bones(armature, head, piv)
    weight(armature, body, head, piv)
    return piv

