# SPDX-License-Identifier: AGPL-3.0-or-later
"""Hands that close (22 §4.7, BW7): the creator bases' finger joints and their weights.

The `biped` rig has mitten hands: one `fingers` bone, one `thumb` bone. The CC0 body has five
real digits, and Blender's automatic weights gave that one finger bone the index and middle
fingers only. Here the hand is read off the mesh (its five tips, the webs between them), given
the joints a grip needs, and weighted by them:

    fingers_<s>      the knuckles: all four fingers' first joints, on one line
    fingers_02_<s>   their middle joints
    fingers_03_<s>   their last joints
    thumb_<s>        the thumb from its root at the wrist (it swings across the palm)
    thumb_02_<s>     its last joint

The four fingers share each joint: three bones curl them together round a haft. Each of those
bones lies across its row of joints: its local X is the line through the four fingers' joints,
its Z the back of the hand, so turning it by a negative angle about X closes the hand, on both
sides. The thumb's bones likewise (X its hinge, Z its back).

`prop_<s>` moves to the middle of the closed fist: where a held haft's axis runs.
"""

import heapq
import math

import bpy
from mathutils import Vector

# A finger's joints, as shares of its length from the tip (index: 2.4, 2.6, 4.6 cm).
DIP, PIP = 0.25, 0.52
# How much of a finger shows beyond the web between it and its neighbor.
SHOWN = 0.74
# The thumb's last joint, as a share of what shows of it; its root, a share of the way from the
# wrist to where it leaves the palm.
THUMB_IP, THUMB_ROOT = 0.46, 0.22
# Half the width of the blend across a joint, as a share of the finger's length.
BAND = 0.035
# A nominal haft (m): `prop` sits where its axis runs in the closed fist.
HAFT = 0.018
# How much of the fingers' spread at rest stays.
SPLAY = 0.2

NEW = ("fingers_02", "fingers_03", "thumb_02")
# The hands last read, by armature name: what's worn on them is weighted the same way (`glove`).
_READ = {}


def _hand_verts(body, side):
    names = {g.index: g.name for g in body.vertex_groups}
    mine = {f"hand_{side}", f"fingers_{side}", f"thumb_{side}"}
    return [v.index for v in body.data.vertices
            if sum(g.weight for g in v.groups if names.get(g.group) in mine) >= 0.5]


def _graph(body, verts):
    inside = set(verts)
    cos = body.data.vertices
    adj = {v: [] for v in verts}
    edge = set()
    for e in body.data.edges:
        a, b = e.vertices
        if a in inside and b in inside:
            d = (cos[a].co - cos[b].co).length
            adj[a].append((b, d))
            adj[b].append((a, d))
        elif a in inside:
            edge.add(a)
        elif b in inside:
            edge.add(b)
    return adj, edge


def _dijkstra(adj, seeds):
    dist = {s: 0.0 for s in seeds}
    heap = [(0.0, s) for s in seeds]
    while heap:
        d, v = heapq.heappop(heap)
        if d > dist.get(v, 1e9):
            continue
        for w, l in adj[v]:
            nd = d + l
            if nd < dist.get(w, 1e9):
                dist[w] = nd
                heapq.heappush(heap, (nd, w))
    return dist


def _line(points):
    """The direction of the best line through `points` (their widest spread)."""
    c = sum(points, Vector()) / len(points)
    d = points[0] - points[-1]
    for _ in range(8):
        d = sum(((p - c) * (p - c).dot(d) for p in points), Vector())
        d.normalize()
    return d


def read(armature, body, side="l"):
    """The hand of `side` off the mesh: the digits' vertices, joints and frames (armature
    space). Returns a dict, or None if the mesh has no five-fingered hand there."""
    cos = body.data.vertices
    verts = _hand_verts(body, side)
    if len(verts) < 60:
        return None
    adj, ring = _graph(body, verts)
    wrist = armature.data.bones[f"hand_{side}"].head_local.copy()
    from_wrist = _dijkstra(adj, list(ring))
    # The tips: the farthest points from the wrist along the skin, well apart.
    peaks = [v for v in verts if v in from_wrist and all(from_wrist[v] >= from_wrist.get(w, 0.0) for w, _ in adj[v])]
    peaks.sort(key=lambda v: -from_wrist[v])
    tips = []
    for v in peaks:
        if all((cos[v].co - cos[t].co).length > 0.016 for t in tips):
            tips.append(v)
    if len(tips) < 5:
        return None
    tips = tips[:5]
    # The thumb: the one that points furthest from the others.
    dirs = {t: (cos[t].co - wrist).normalized() for t in tips}
    mean = sum(dirs.values(), Vector()).normalized()
    thumb = min(tips, key=lambda t: dirs[t].dot(mean))
    fingers = [t for t in tips if t != thumb]
    # Index to little finger: by the angle from the thumb.
    fingers.sort(key=lambda t: -dirs[t].dot(dirs[thumb]))
    order = fingers + [thumb]
    # Each digit: the skin nearer its tip than any other, out to the nearest web.
    from_tip = {t: _dijkstra(adj, [t]) for t in order}
    cell = {v: min(order, key=lambda t: from_tip[t].get(v, 1e9)) for v in verts}
    digits = []
    for t in order:
        web = min((from_tip[t][a] for a in verts if cell[a] == t for b, _ in adj[a] if cell[b] != t), default=0.0)
        own = [v for v in verts if cell[v] == t and from_tip[t][v] <= web]
        base = [v for v in own if from_tip[t][v] >= web * 0.82]
        root = sum((cos[v].co for v in base), Vector()) / max(len(base), 1)
        tip = cos[t].co.copy()
        axis = (root - tip).normalized()
        shown = max((cos[v].co - tip).dot(axis) for v in own)
        digits.append({"tip": tip, "axis": axis, "shown": shown, "verts": own})
    f4, th = digits[:4], digits[4]
    # A web read deep (a finger shown short) would put its knuckle out of the row.
    typical = sorted(d["shown"] for d in f4)[2] / SHOWN
    for d in f4:
        d["len"] = max(d["shown"] / SHOWN, typical * 0.94)
        d["mcp"] = d["tip"] + d["axis"] * d["len"]
        d["pip"] = d["tip"] + d["axis"] * (d["len"] * PIP)
        d["dip"] = d["tip"] + d["axis"] * (d["len"] * DIP)
    th["len"] = th["shown"]
    th["ip"] = th["tip"] + th["axis"] * (th["shown"] * THUMB_IP)
    th["mcp"] = th["tip"] + th["axis"] * th["shown"]
    th["root"] = wrist.lerp(th["mcp"], THUMB_ROOT)
    # The back of the hand: off the plane of the wrist and the knuckles, away from the way the
    # fingers curl at rest and from the thumb.
    back = (f4[0]["mcp"] - wrist).cross(f4[3]["mcp"] - wrist).normalized()
    lean = sum(((d["tip"] - d["mcp"]).dot(back) for d in f4), 0.0) + (th["tip"] - wrist).dot(back)
    if lean > 0:
        back = -back
    out = {"side": side, "wrist": wrist, "fingers": f4, "thumb": th, "back": back}
    # The rows of joints: a point and the line across the four fingers.
    for row in ("mcp", "pip", "dip"):
        pts = [d[row] for d in f4]
        out[row] = (sum(pts, Vector()) / 4, _line(pts))
    out["tips"] = sum((d["tip"] for d in f4), Vector()) / 4
    # Half a finger's thickness: how far its skin stands off its axis.
    spans = []
    for d in f4:
        for v in d["verts"]:
            p = cos[v].co - d["tip"]
            spans.append((p - d["axis"] * p.dot(d["axis"])).length)
    spans.sort()
    out["thick"] = spans[len(spans) * 3 // 4] if spans else 0.008
    return out


def _mirror(hand):
    def m(v):
        return Vector((-v.x, v.y, v.z))

    def digit(d):
        return {k: (m(v) if isinstance(v, Vector) else v) for k, v in d.items() if k != "verts"}

    out = {"side": "r", "wrist": m(hand["wrist"]), "back": m(hand["back"]), "tips": m(hand["tips"]),
           "thick": hand["thick"], "fingers": [digit(d) for d in hand["fingers"]], "thumb": digit(hand["thumb"])}
    for row in ("mcp", "pip", "dip"):
        out[row] = (m(hand[row][0]), m(hand[row][1]))
    return out


def _smooth(t):
    t = min(max(t, 0.0), 1.0)
    return t * t * (3 - 2 * t)


def _assign(hand, p):
    """Point `p` of the hand's skin on its bones: (weights by bone, the finger it belongs to or
    None)."""
    f4, th = hand["fingers"], hand["thumb"]

    def off(d):
        q = p - d["tip"]
        return (q - d["axis"] * q.dot(d["axis"])).length

    d = min(f4, key=off)
    t = (p - d["tip"]).dot(d["axis"]) / d["len"]
    t_th = (p - th["tip"]).dot(th["axis"]) / th["len"]
    # The ball of the thumb goes with the thumb, more toward where it leaves the palm.
    seg_a = th["root"]
    seg = th["mcp"] - seg_a
    u = min(max((p - seg_a).dot(seg) / seg.length_squared, 0.0), 1.0)
    near = (p - (seg_a + seg * u)).length
    ball = _smooth(1.0 - near / 0.03) * _smooth(u * 1.6) * 0.85
    if t_th <= 1.15 and off(th) < off(d) and off(th) < 0.03:
        w2 = 1.0 - _smooth((t_th - THUMB_IP + 2 * BAND) / (4 * BAND))
        # Whole on the thumb where it stands free, easing into the ball at its root.
        free = max(1.0 - _smooth((t_th - 0.8) / 0.35), ball)
        return {"thumb_02": w2 * free, "thumb": (1.0 - w2) * free, "hand": 1.0 - free}, None
    if t <= 1.0 + 3 * BAND and off(d) < hand["thick"] * 2.6:
        w3 = 1.0 - _smooth((t - DIP + BAND) / (2 * BAND))
        w2 = (1.0 - w3) * (1.0 - _smooth((t - PIP + BAND) / (2 * BAND)))
        rest = 1.0 - w3 - w2
        # Across the knuckle, wider: the skin of the back of the hand folds over it.
        w1 = rest * (1.0 - _smooth((t - 1.0 + 3 * BAND) / (6 * BAND)))
        return {"fingers_03": w3, "fingers_02": w2, "fingers": w1, "hand": rest - w1}, d
    return {"thumb": ball, "hand": 1.0 - ball}, None


def _weigh(body, hand, verts):
    """The hand's vertices onto its bones: each digit by its joints, the palm on the hand, the
    knuckles and the ball of the thumb blended."""
    s = hand["side"]
    cos = body.data.vertices
    groups = {}
    for n in ("hand", "fingers", "fingers_02", "fingers_03", "thumb", "thumb_02"):
        name = f"{n}_{s}"
        groups[n] = body.vertex_groups.get(name) or body.vertex_groups.new(name=name)
    for v in verts:
        w, _ = _assign(hand, cos[v].co)
        for g in groups.values():
            g.remove([v])
        tot = sum(w.values())
        for n, x in w.items():
            if x > 1e-3:
                groups[n].add([v], x / tot, "REPLACE")


def _gather(body, hand, verts, keep=SPLAY):
    """Bring the four fingers side by side: each turns at its knuckle toward the way the four
    point together (`keep` of its spread stays). Fingers that share
    their joints (one bone a row) close into a fist only if they lie alongside each other."""
    cos = body.data.vertices
    f4 = hand["fingers"]
    mean = sum((d["axis"] for d in f4), Vector()).normalized()
    turns = {}
    for i, d in enumerate(f4):
        a = d["axis"]
        to = mean.lerp(a, keep).normalized()
        turns[i] = a.rotation_difference(to).to_matrix()
    # And slid along itself so the middle joints make one straight row: the four share a hinge.
    c, line = hand["pip"]
    slides = {}
    for i, d in enumerate(f4):
        pip = d["mcp"] + turns[i] @ (d["pip"] - d["mcp"])
        on_row = c + line * (pip - c).dot(line)
        slides[i] = mean * (on_row - pip).dot(mean)
    for v in verts:
        p = cos[v].co
        w, d = _assign(hand, p)
        if d is None:
            continue
        i = next(k for k, f in enumerate(f4) if f is d)
        moved = d["mcp"] + turns[i] @ (p - d["mcp"]) + slides[i]
        cos[v].co = p.lerp(moved, 1.0 - w.get("hand", 0.0))


# As the forge closes a fist (`mftr-forge/src/grip.rs`): half a finger's thickness as a share of
# its first phalanx, and the knuckles' bend (rad).
THICK = 0.2
KNUCKLE = 1.22


def _centre(mcp, down, back, l1, radius):
    """Where a haft of `radius` runs in the closed fist: against the palm, in the crook of the
    fingers (`mcp` the knuckles' middle, `down` the fingers' way, `back` the hand's back, `l1`
    the first phalanx's length)."""
    r = radius + l1 * THICK
    d0 = min(r * math.tan(KNUCKLE / 2), l1 * 0.8)
    return mcp - down * d0 - back * r


def fist_centre(armature, side, radius):
    """The middle of hand `side`'s closed fist on a haft of `radius` (armature space, at rest),
    and the haft's way there (along the knuckles) and the way out of the back of the hand."""
    b = armature.data.bones
    f, f2 = b[f"fingers_{side}"], b[f"fingers_02_{side}"]
    m = f.matrix_local.to_3x3()
    x, y, z = (m @ Vector(v) for v in ((1, 0, 0), (0, 1, 0), (0, 0, 1)))
    return _centre(f.head_local, y, z, (f2.head_local - f.head_local).length, radius), x, z


def _frame(eb, head, x, z, length):
    """Edit bone `eb` at `head` with local X along `x` and Z toward `z`: Y (the bone) is what's
    left, pointing down the digit."""
    x = x.normalized()
    z = (z - x * z.dot(x)).normalized()
    y = z.cross(x)
    eb.head = head
    eb.tail = head + y * length
    eb.align_roll(z)
    return y


def rig(armature, body):
    """Give `armature` finger joints for `body`'s hands and weight the hands to them. Returns
    the left hand as read (None when the body has no five-fingered hands: nothing changes)."""
    left = read(armature, body, "l")
    if left is None:
        return None
    verts = {s: _hand_verts(body, s) for s in "lr"}
    _gather(body, left, verts["l"])
    _gather(body, _mirror(left), verts["r"])
    left = read(armature, body, "l")
    hands = {"l": left, "r": _mirror(left)}
    prev = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    eb = armature.data.edit_bones
    # Loose from their parents first: a connected bone's head drags its parent's tail, and the
    # hand bone stays where the rig has it (the clips aim weapons by it).
    for s in hands:
        for name in ("fingers", "thumb", "prop"):
            if eb.get(f"{name}_{s}") is not None:
                eb[f"{name}_{s}"].use_connect = False
    for s, h in hands.items():
        back = h["back"]
        rows = [("fingers", "mcp", "pip"), ("fingers_02", "pip", "dip"), ("fingers_03", "dip", None)]
        parent = eb[f"hand_{s}"]
        down = (h["tips"] - h["mcp"][0]).normalized()
        for name, row, nxt in rows:
            b = eb.get(f"{name}_{s}") or eb.new(f"{name}_{s}")
            head, line = h[row]
            end = h[nxt][0] if nxt else h["tips"]
            # The hinge points so that the bone runs down the finger.
            z = back - line * back.dot(line)
            if z.cross(line).dot(down) < 0:
                line = -line
            _frame(b, head, line, back, (end - head).length)
            b.use_deform = True
            b.use_connect = False
            b.parent = parent
            parent = b
        th = h["thumb"]
        # The thumb's hinge: it closes toward the palm's middle, across the fingers' roots.
        along = (th["tip"] - th["root"]).normalized()
        palm_mid = (h["wrist"] + h["mcp"][0]) / 2 - back * 0.02
        toward = palm_mid - th["ip"]
        toward = (toward - along * toward.dot(along)).normalized()
        t_back = -toward
        hinge = along.cross(t_back)
        b = eb[f"thumb_{s}"]
        b.use_connect = False
        _frame(b, th["root"], hinge, t_back, (th["ip"] - th["root"]).length)
        b2 = eb.get(f"thumb_02_{s}") or eb.new(f"thumb_02_{s}")
        along2 = (th["tip"] - th["ip"]).normalized()
        hinge2 = along2.cross(t_back - along2 * t_back.dot(along2))
        _frame(b2, th["ip"], hinge2, t_back, (th["tip"] - th["ip"]).length)
        b2.use_deform = True
        b2.parent = b
        # The fist's middle: a haft lies in the crook of the fingers, against the palm.
        prop = eb.get(f"prop_{s}")
        if prop is not None:
            fb = eb[f"fingers_{s}"]
            centre = _centre(fb.head, fb.y_axis, fb.z_axis, (eb[f"fingers_02_{s}"].head - fb.head).length, HAFT)
            shift = centre - prop.head
            kids = [c for c in prop.children]
            prop.head, prop.tail = prop.head + shift, prop.tail + shift
            for c in kids:
                c.head, c.tail = c.head + shift, c.tail + shift
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev
    for pb in armature.pose.bones:
        pb.rotation_mode = "QUATERNION"
    for s, h in hands.items():
        _weigh(body, h, verts[s])
    _READ[armature.name] = hands
    return left


def glove(armature, obj):
    """Weight what `obj` (a gauntlet, a glove, a wrap) has on the hands as the hands themselves
    are: each vertex by where it lies on the digits and their joints, not by the nearest skin (a
    finger's glove next to another finger takes that one's bones, and tears when they close)."""
    hands = _READ.get(armature.name)
    if not hands:
        return 0
    names = {g.index: g.name for g in obj.vertex_groups}
    n = 0
    for s, h in hands.items():
        mine = {f"{b}_{s}" for b in ("hand", "fingers", "fingers_02", "fingers_03", "thumb", "thumb_02")}
        groups = {b: obj.vertex_groups.get(f"{b}_{s}") or obj.vertex_groups.new(name=f"{b}_{s}")
                  for b in ("hand", "fingers", "fingers_02", "fingers_03", "thumb", "thumb_02")}
        names = {g.index: g.name for g in obj.vertex_groups}
        for v in obj.data.vertices:
            on = sum(g.weight for g in v.groups if names.get(g.group) in mine)
            if on < 0.5:
                continue
            w, _ = _assign(h, v.co)
            for g in list(v.groups):
                if names.get(g.group) in mine:
                    obj.vertex_groups[g.group].remove([v.index])
            tot = sum(w.values())
            for b, x in w.items():
                if x > 1e-3:
                    groups[b].add([v.index], on * x / tot, "REPLACE")
            n += 1
    return n
