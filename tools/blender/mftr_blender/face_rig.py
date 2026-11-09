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


def add_bones(armature, head):
    """Add the face bones to `armature` (edit mode), pointing out of the face (-Y), 2 cm long
    (the jaw toward the chin). Returns the pivots."""
    piv = pivots(head)
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
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev
    return piv


# Placeholder reach per bone (m, before head scale): where its weight fades to zero. Wide on
# purpose: the block-out head's edges are ~2.5 cm, and a narrower falloff would pinch.
REACH = {
    "face_forehead": 0.11, "face_brow_mid": 0.08, "face_brow": 0.08, "face_eye": 0.08, "face_cheekbone": 0.09,
    "face_cheek": 0.09, "face_nose_bridge": 0.08, "face_nose_tip": 0.08, "face_nostril": 0.07, "face_lip_upper": 0.07,
    "jaw": 0.14, "face_lip_lower": 0.07, "face_mouth": 0.07, "face_chin": 0.09, "face_jaw_angle": 0.09, "face_ear": 0.07,
}
# A bone never takes all of a vertex: the head keeps some, so neighbouring bones blend.
PEAK = 0.85


# Along any edge, no face bone's weight may change by more than this (mftr_pack::face::
# MAX_WEIGHT_STEP is 0.35; a margin for the exporter's quantization).
MAX_STEP = 0.32


def _final(raw):
    """A vertex's face weights as at most three bones, at most PEAK in all: the fourth-
    strongest weight is taken off the top three, so a bone fades to nothing exactly where it
    drops out of the three (a plain cut would jump there and pinch)."""
    ws = sorted(((w, i) for i, w in enumerate(raw) if w > 1e-4), reverse=True)
    cut = ws[3][0] if len(ws) > 3 else 0.0
    ws = [(w - cut, i) for w, i in ws[:3] if w > cut]
    total = sum(w for w, _ in ws)
    if total > PEAK:
        ws = [(w * PEAK / total, i) for w, i in ws]
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
    floor = head.base.z + head.height * 0.04  # the bottom of the head: seam side
    names = [n for n, _ in FACE_RIG]
    reach = [REACH.get(n, REACH.get(n[:-2], 0.08)) * k for n in names]
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
    # The falloff, letting go toward the seam (a cut would pinch the chin).
    raw = []
    for w in range(n):
        p = pos[w]
        fade = max(0.0, min(1.0, (p.z - floor) / (0.07 * k)))
        raw.append([max(0.0, 1.0 - (p - piv[nm]).length / reach[i]) * PEAK * fade for i, nm in enumerate(names)])
    seam = [pos[w].z <= floor for w in range(n)]

    def steepest(final):
        worst = 0.0
        for a in range(n):
            fa = dict((i, x) for x, i in final[a])
            for b in nbrs[a]:
                fb = dict((i, x) for x, i in final[b])
                for i in set(fa) | set(fb):
                    worst = max(worst, abs(fa.get(i, 0.0) - fb.get(i, 0.0)))
        return worst

    final = [_final(r) for r in raw]
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
        final = [_final(r) for r in raw]
    for vi, w in ids.items():
        total = 0.0
        for x, i in final[w]:
            groups[names[i]].add([vi], x, "REPLACE")
            total += x
        g_head.add([vi], 1.0 - total, "REPLACE")
    print(f"face rig: {n} head places weighted, steepest edge {steepest(final):.2f}")


def rig(armature, body, head):
    """Both: bones, then the head's weights. Returns the pivots."""
    piv = add_bones(armature, head)
    weight(armature, body, head, piv)
    return piv

