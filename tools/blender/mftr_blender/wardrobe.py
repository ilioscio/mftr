# SPDX-License-Identifier: AGPL-3.0-or-later
"""The wardrobe (22 §4, BW5): clothing, armour and accessories authored once and fitted to every
humanoid base at build time.

**An item** lives in `art/wardrobe/<slot>/<name>/`:

- `item.ron`: what it is (`id`, `slot`, `name`, `layer`, `accent`, `rigid`), as the catalog lists it,
  its spring-bone chains if it hangs (`chains`, `chain_turn`, `chain_arc`, `chain_from`, `swing`:
  `chains.hang_cloth`), `hold`, a bone that what stands off the body rides whole (`_hold`), and
  `sleeves` (a torso item with sleeves is fitted to the arms too);
- `item.blend`: one mesh object, `item`, **authored around the reference body**: the CC0 male in
  its own space (`art/creator/bodies/body_source.blend`, `body_male`), standing as the bundle has
  it. Its faces use the material slots' names (`skin`, `cloth`, `metal`, `accent`…), its vertex
  colors (`Col`) are its paint regions.
- optionally `build.py`, which makes `item.blend` (a sewn garment's patterns, `sewing`).

**Fitting** (`fit_items`, from `creator.build`), per base:

1. Onto the base's sex: an item vertex is bound to the reference body's surface (its nearest
   faces, barycentric place and offset in each face's frame) and carried onto the female body's
   surface, which shares the male's topology.
2. Onto the base: the base's body remembers each vertex's place on its source body (`src_co`,
   `cc0_body.make`); the item is bound to the body there and carried to where the fit put it.
   The rig's proportions, the build's girth, the shoulders' drop all come along.
3. Pushed out: no vertex closer to the skin than its layer's offset (`LAYERS`).
4. Weighted: each vertex takes the biped weights of the body surface under it (two bones), then
   the helpers' (`body_rig.transfer`), so it follows the body sliders and the sculpting.

In the base it is a catalog part (`part.<slot>.<id>`) flagged `covers`: the forge hides the body
under it by its geometry (`mftr_forge::wardrobe`).
"""

import math
import os
import re

import bpy
from mathutils import Matrix, Vector
from mathutils.bvhtree import BVHTree

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "..", "..", "..", "art", "wardrobe"))

# How far off the skin each layer sits at least (m).
LAYERS = {"skin": 0.002, "under": 0.003, "over": 0.008, "armor": 0.012, "accessory": 0.004}


def items():
    """Every item's folder and its `item.ron` fields, sorted by id."""
    out = []
    if not os.path.isdir(ROOT):
        return out
    for slot in sorted(os.listdir(ROOT)):
        sdir = os.path.join(ROOT, slot)
        if not os.path.isdir(sdir):
            continue
        for name in sorted(os.listdir(sdir)):
            ron = os.path.join(sdir, name, "item.ron")
            if os.path.isfile(ron) and os.path.isfile(os.path.join(sdir, name, "item.blend")):
                out.append((os.path.join(sdir, name), read_item(ron)))
    return out


def read_item(path):
    """`item.ron`'s fields: strings, booleans, numbers and tuples of numbers of one flat struct."""
    text = re.sub(r"//[^\n]*", "", open(path, encoding="utf-8").read())
    out = {}
    num = r"-?[\d.]+"
    pattern = rf"(\w+)\s*:\s*(\"[^\"]*\"|true|false|\(\s*{num}(?:\s*,\s*{num})*\s*,?\s*\)|{num})"
    for key, val in re.findall(pattern, text):
        if val.startswith("("):
            out[key] = tuple(float(x) for x in re.findall(num, val))
        else:
            out[key] = val[1:-1] if val.startswith('"') else (val == "true" if val in ("true", "false") else float(val))
    for k in ("id", "slot", "name"):
        if k not in out:
            raise ValueError(f"{path}: needs `{k}`")
    out.setdefault("layer", "over")
    out.setdefault("accent", False)
    out.setdefault("rigid", False)
    return out


def load(folder):
    """The item's mesh object (a new copy in the scene)."""
    path = os.path.join(folder, "item.blend")
    with bpy.data.libraries.load(path, link=False) as (_src, dst):
        dst.objects = ["item"]
    obj = dst.objects[0]
    bpy.context.scene.collection.objects.link(obj)
    return obj


# --- Surface binding ------------------------------------------------------------------------

class Surface:
    """A triangulated surface given twice, at rest and moved (`rest`, `moved`: positions by
    vertex; `tris`: index triples): points bound to the rest one are carried to the moved one."""

    def __init__(self, rest, moved, tris):
        self.rest, self.moved, self.tris = rest, moved, tris
        self.tree = BVHTree.FromPolygons(rest, tris)

    @staticmethod
    def _frame(a, b, c):
        e1 = (b - a).normalized()
        n = (b - a).cross(c - a).normalized()
        return e1, n.cross(e1), n

    def carry(self, p, k=4):
        """Where `p` (near the rest surface) goes on the moved one: the mean, by closeness, of
        what each of its `k` nearest faces makes of it (its barycentric place there, and its
        offset in the face's frame)."""
        loc, _n, _i, d = self.tree.find_nearest(p)
        if loc is None:
            return p.copy()
        # Only faces about as near as the nearest: far off the body, a wider net reaches the
        # other side of the waist, which moves the other way, and the moves cancel.
        near = self.tree.find_nearest_range(p, d * 1.15 + 0.004)
        near.sort(key=lambda h: h[3])
        acc, tot = Vector(), 0.0
        for q, _n, fi, dist in near[:k]:
            ia, ib, ic = self.tris[fi]
            a, b, c = self.rest[ia], self.rest[ib], self.rest[ic]
            u, v, w = _bary(q, a, b, c)
            e1, e2, n = self._frame(a, b, c)
            off = p - q
            lo = (off.dot(e1), off.dot(e2), off.dot(n))
            A, B, C = self.moved[ia], self.moved[ib], self.moved[ic]
            q2 = A * u + B * v + C * w
            f1, f2, m = self._frame(A, B, C)
            wgt = 1.0 / (dist + 0.003) ** 2
            # Near the skin the offset turns with the face; far off it (a hem hanging free) only
            # moves with it: a small turn on a long lever would swing it far.
            turned = q2 + f1 * lo[0] + f2 * lo[1] + m * lo[2]
            moved = q2 + off
            t = min(1.0, max(0.0, (off.length - 0.01) / 0.03))
            acc += turned.lerp(moved, t) * wgt
            tot += wgt
        return acc / tot


def _bary(p, a, b, c):
    v0, v1, v2 = b - a, c - a, p - a
    d00, d01, d11 = v0.dot(v0), v0.dot(v1), v1.dot(v1)
    d20, d21 = v2.dot(v0), v2.dot(v1)
    den = d00 * d11 - d01 * d01
    if abs(den) < 1e-14:
        return 1.0, 0.0, 0.0
    v = (d11 * d20 - d01 * d21) / den
    w = (d00 * d21 - d01 * d20) / den
    return 1.0 - v - w, v, w


def _tris(me, keep=None):
    me.calc_loop_triangles()
    return [tuple(t.vertices) for t in me.loop_triangles if keep is None or all(keep[i] for i in t.vertices)]


def _armless(cos, upper=False):
    """Which of the source body's vertices (`cos`) aren't the arms below the shoulder: in the
    bundle's A pose, beyond the torso's widest (the shoulders' 20 cm) and below them. With
    `upper`, the shoulder caps stay (above 1.2 m, where a pauldron sits): the rest of the arm,
    the elbow on, goes."""
    top = 1.20 if upper else 1.38
    return [not (abs(c.x) > 0.19 and 0.95 < c.z < top) and not (abs(c.x) > 0.24 and c.z <= 0.95) for c in cos]


def _source_surfaces(sex, arms=True):
    """The reference body (the CC0 male) and, for a female base, the female body on the same
    topology: the surface that carries an item onto the base's sex. Without `arms`, an item
    beside the arms (a coat's side by the hands) isn't bound to them: each body's arms hang at
    their own angle and would carry it out with them."""
    from . import cc0_body
    male = cc0_body.load("male")
    tris = _tris(male.data, None if arms is True else _armless([v.co for v in male.data.vertices], arms == "upper"))
    rest = [v.co.copy() for v in male.data.vertices]
    if sex == "male":
        moved = rest
    else:
        female = cc0_body.load("female")
        moved = [v.co.copy() for v in female.data.vertices]
        bpy.data.objects.remove(female)
    bpy.data.objects.remove(male)
    return Surface(rest, moved, tris)


def _off_arms(body, arms=False):
    """Which of the base body's vertices aren't the arms an item without `arms` ignores: by the
    arms' bones; for "upper", by each vertex's place on its source body (`src_co`), as
    `_armless` draws the line on the reference body (the bones don't split the upper arm)."""
    if arms == "upper":
        return _armless([Vector(d.vector) for d in body.data.attributes["src_co"].data], upper=True)
    return [not a for a in _arm_bones(body)]


def _arm_bones(body):
    """Which of the base body's vertices the arms below the shoulder own (by their bones)."""
    names = {g.index: g.name for g in body.vertex_groups}
    out = []
    for v in body.data.vertices:
        on = sum(g.weight for g in v.groups if names[g.group].startswith(ARMS))
        out.append(on > 0.2)
    return out


ARMS = ("upperarm_", "forearm_", "hand_", "fingers_", "thumb_", "prop_")


def _base_surface(body, arms=True):
    """The base's body as it stood on its source body (`src_co`) and where the fit put it;
    without `arms`, as `_source_surfaces` (by the arms' bones, here)."""
    me = body.data
    src = me.attributes["src_co"].data
    ok = me.attributes["src_ok"].data
    keep = [d.value > 0.5 for d in ok]
    rest = [Vector(d.vector) for d in src]
    if arms is not True:
        keep = [k and a for k, a in zip(keep, _off_arms(body, arms))]
    moved = [v.co.copy() for v in me.vertices]
    return Surface(rest, moved, _tris(me, keep))


# --- Fitting ----------------------------------------------------------------------------------

def _body_keep(body, arms):
    """Which of the base body's vertices count for fitting (the head joined on later doesn't;
    nor the arms below the shoulder without `arms`)."""
    me = body.data
    keep = [d.value > 0.5 for d in me.attributes["src_ok"].data]
    if arms is not True:
        keep = [k and a for k, a in zip(keep, _off_arms(body, arms))]
    return keep


def _push_out(obj, body, offset, reach=0.05, arms=True):
    """No item vertex nearer the skin than `offset`: one inside or too close goes out along the
    skin's normal (the body's outward faces), those farther than `reach` (a hem hanging free)
    stay."""
    bme = body.data
    keep = _body_keep(body, arms)
    tris = _tris(bme, keep)
    tree = BVHTree.FromPolygons([v.co.copy() for v in bme.vertices], tris)
    moved = 0
    for v in obj.data.vertices:
        q, n, _i, d = tree.find_nearest(v.co)
        if q is None or d > reach:
            continue
        s = (v.co - q).dot(n)
        if s < offset:
            v.co = v.co + n * (offset - s)
            moved += 1
    return moved


# The most `_clear_peaks` moves any item vertex (m).
PEAK_CAP = 0.05


def _clear_peaks(obj, body, offset, rounds=8, arms=True):
    """From the body's side: a body vertex at or past the item's surface (a bust's tip, a
    shoulder's point, under the middle of one of the item's big faces, where no item vertex is
    near it) pushes that face's vertices out along the face's normal until it's `offset` under
    it. The item's lining (faces looking at the body) is left alone."""
    bme = body.data
    keep = _body_keep(body, arms)
    pts = [(v.co.copy(), v.normal.copy()) for v, k in zip(bme.vertices, keep) if k]
    me = obj.data
    # Only where the item is: a hem's rim must not be pushed by the foot below it.
    lo = min(v.co.z for v in me.vertices) + 0.03
    hi = max(v.co.z for v in me.vertices) - 0.03
    pts = [(p, bn) for p, bn in pts if lo < p.z < hi]
    total = {}
    for _ in range(rounds):
        me.calc_loop_triangles()
        tris = [tuple(t.vertices) for t in me.loop_triangles]
        pos = [v.co.copy() for v in me.vertices]
        tree = BVHTree.FromPolygons(pos, tris)
        push = {}
        for p, bn in pts:
            hit = tree.find_nearest(p, 0.04)
            if hit[0] is None:
                continue
            q, n, fi, _d = hit
            if n.dot(bn) < 0.3:
                continue
            s = (p - q).dot(n)
            # Only a point under the face (its nearest point inside it, straight above), not one
            # beside it: a hip beside a coat's free-hanging side mustn't push it out.
            if ((p - q) - n * s).length > 0.004:
                continue
            if -offset < s < 0.02:
                need = n * (s + offset)
                for vi in tris[fi]:
                    if vi not in push or need.length > push[vi].length:
                        push[vi] = need
        if not push:
            break
        for vi, d in push.items():
            # At most `CAP` in all, over every round.
            had = total.get(vi, Vector())
            room = PEAK_CAP - had.length
            if room <= 0:
                continue
            if d.length > room:
                d = d.normalized() * room
            total[vi] = had + d
            me.vertices[vi].co += d
    return len(push)


# Bones a garment on the torso, the waist or the legs never follows: a coat's side beside the
# hand must not swing out with it.
HANDS = ("forearm_", "hand_", "fingers_", "thumb_", "prop_")


def _weights(obj, body, skip, avoid=(), arms=True):
    """Each item vertex takes the biped weights of the body's surface under it (the nearest
    face's, by barycentric place), its two largest, normalized; helpers (`skip`) not. The body's
    faces owned by bones starting with `avoid` aren't candidates, nor (for "upper" `arms`) the
    arms below the shoulder caps."""
    bme = body.data
    names = {g.index: g.name for g in body.vertex_groups}
    keep = [d.value > 0.5 for d in bme.attributes["src_ok"].data]
    if arms == "upper":
        keep = [k and a for k, a in zip(keep, _off_arms(body, arms))]
    if avoid:
        for v in bme.vertices:
            top = max(((g.weight, names[g.group]) for g in v.groups), default=(0.0, ""))[1]
            if top.startswith(avoid):
                keep[v.index] = False
    tris = _tris(bme, keep)
    pos = [v.co.copy() for v in bme.vertices]
    tree = BVHTree.FromPolygons(pos, tris)
    vw = [{names[g.group]: g.weight for g in v.groups if g.weight > 0 and names[g.group] not in skip} for v in bme.vertices]
    groups = {}
    for v in obj.data.vertices:
        q, _n, fi, _d = tree.find_nearest(v.co)
        if q is None:
            continue
        ia, ib, ic = tris[fi]
        u, w1, w2 = _bary(q, pos[ia], pos[ib], pos[ic])
        acc = {}
        for idx, k in ((ia, u), (ib, w1), (ic, w2)):
            for n, x in vw[idx].items():
                acc[n] = acc.get(n, 0.0) + x * max(0.0, k)
        top = sorted(acc.items(), key=lambda t: -t[1])[:2]
        tot = sum(x for _, x in top) or 1.0
        for n, x in top:
            if n not in groups:
                groups[n] = obj.vertex_groups.get(n) or obj.vertex_groups.new(name=n)
            groups[n].add([v.index], x / tot, "REPLACE")


# How far below the body's lowest point an item may reach (m): a sole's thickness.
FLOOR = 0.008

# How far off the skin a held item's vertex rides its bone whole (m); it blends in over HOLD_BLEND.
HOLD_FROM, HOLD_BLEND = 0.04, 0.02


def _hold(obj, body, bone, start):
    """`obj`'s vertices further than `start` from `body`'s surface onto `bone` alone, blending
    from the body's weights over HOLD_BLEND."""
    from . import chains
    bme = body.data
    tree = BVHTree.FromPolygons([v.co.copy() for v in bme.vertices], [tuple(p.vertices) for p in bme.polygons])
    names = {g.index: g.name for g in obj.vertex_groups}
    weights = {}
    for v in obj.data.vertices:
        q, _n, _i, d = tree.find_nearest(v.co)
        if q is None:
            continue
        t = min(max((d - start) / HOLD_BLEND, 0.0), 1.0)
        if t <= 0.0:
            continue
        own = {names[g.group]: g.weight * (1.0 - t) for g in v.groups if g.weight > 0}
        own[bone] = own.get(bone, 0.0) + t
        weights[v.index] = own
    chains.reweigh(obj, weights, keep=4)


def _materials(obj):
    """The item's faces on the base's material slots (by their materials' names)."""
    from . import mesh
    me = obj.data
    names = [m.name.split(".")[0] if m else "cloth" for m in me.materials]
    slot_of = [mesh.MATERIAL_SLOTS.index(n) if n in mesh.MATERIAL_SLOTS else 1 for n in names]
    polys = [slot_of[p.material_index] if p.material_index < len(slot_of) else 1 for p in me.polygons]
    me.materials.clear()
    for s in mesh.MATERIAL_SLOTS:
        me.materials.append(mesh.material(s))
    for p, s in zip(me.polygons, polys):
        p.material_index = s


def _arms(info):
    """How much of the arms an item is fitted to (bound, pushed out of, weighted to): all of them
    (True: gloves, bracers, a sleeved garment), none (False: what's on the torso, the waist, the
    legs or the back hangs beside them), or "upper" (the shoulders: a cloak's pauldron rides the
    shoulder cap, but its side, beside the elbow and the hand in the reference body's A pose,
    mustn't be carried out with the arm)."""
    if "sleeve" in info["id"] or info.get("sleeves", False):
        return True
    if info["slot"] == "shoulders":
        return "upper"
    return info["slot"] not in ("torso", "back", "waist", "legs")


def _strap_handle(obj, armature, s, m, at, side, back, up, radius):
    """A strapped item (a shield on the forearm) gets its grip: a bar where the closed fist of
    hand `s` is (the prop bone's head, along the knuckles), on two posts that reach its back.
    The item is slid in its own plane till the fist is behind its middle, on the back of the
    hand's side, its back just clear of the knuckles. In the item's own frame (`m`, `at`,
    `side`, `back`, `up` place it, as `_place_grip` does)."""
    import bmesh
    from . import hands as hand_rig
    bones = armature.data.bones
    if bones.get(f"fingers_02_{s}") is None or "Col" not in obj.data.color_attributes:
        return
    fist, knuckles, dorsal = hand_rig.fist_centre(armature, s, radius)
    frame = Matrix((side, back, up)).transposed()
    to_item = lambda p: frame.inverted() @ ((m.inverted() @ (armature.matrix_world @ p)) - at)  # noqa: E731
    turn = lambda d: (frame.inverted() @ (m.to_3x3().inverted() @ (armature.matrix_world.to_3x3() @ d))).normalized()  # noqa: E731
    c, along, out = to_item(fist), turn(knuckles), turn(dorsal)
    # The shield's face: the way its body is thinnest (its plane's normal), from the fist toward
    # it. Its back is found by a ray from behind the fist, and the shield is moved along its
    # normal till its back lies just past the knuckles, whichever way its maker tilted it.
    import numpy
    from mathutils.bvhtree import BVHTree
    pts = numpy.array([tuple(v.co) for v in obj.data.vertices])
    centre = pts.mean(axis=0)
    _, vecs = numpy.linalg.eigh(numpy.cov((pts - centre).T))
    n = Vector(tuple(vecs[:, 0])).normalized()
    # On the back of the hand's side, as the clips carry it; slid in its plane so the fist is
    # behind its middle (the boss), where a centre grip's bar is.
    if out.dot(n) < 0:
        n = -n
    slide = Vector(tuple(centre)) - c
    slide -= n * slide.dot(n)
    for v in obj.data.vertices:
        v.co -= slide

    def back_at(p):
        tree = BVHTree.FromPolygons([v.co.copy() for v in obj.data.vertices], [tuple(f.vertices) for f in obj.data.polygons])
        hit = tree.ray_cast(p - n * 0.5, n)
        return None if hit[0] is None else (hit[0] - p).dot(n)

    d = back_at(c)
    if d is None:
        d = min(((v.co - c).dot(n) for v in obj.data.vertices if ((v.co - c) - n * (v.co - c).dot(n)).length < 0.12), default=0.0)
    # The fist's knuckles, and a gauntlet over them, clear of its back.
    clear = 0.055
    for v in obj.data.vertices:
        v.co += n * (clear - d)
    print(f"strap: shield slid {slide.length:.3f} m and moved {clear - d:+.3f} m off the fist")
    half = 0.06
    ends = [c + along * (half * e * 0.92) for e in (-1, 1)]
    posts = [back_at(e) for e in ends]
    bm = bmesh.new()
    bm.from_mesh(obj.data)
    col = bm.verts.layers.float_color["Col"]
    slot = list(mesh_slots()).index("metal")

    def rod(a, b, r, sides=6):
        d = (b - a).normalized()
        u = d.orthogonal().normalized()
        w = d.cross(u)
        rings = [[bm.verts.new(p + (u * math.cos(2 * math.pi * k / sides) + w * math.sin(2 * math.pi * k / sides)) * r)
                  for k in range(sides)] for p in (a, b)]
        faces = [bm.faces.new((rings[0][k], rings[0][(k + 1) % sides], rings[1][(k + 1) % sides], rings[1][k]))
                 for k in range(sides)]
        faces += [bm.faces.new(list(reversed(rings[0]))), bm.faces.new(rings[1])]
        for f in faces:
            f.material_index = slot
            f.smooth = False
            for v in f.verts:
                v[col] = (0.30, 0.31, 0.34, 1.0)
        return faces

    made = rod(c - along * half, c + along * half, radius)
    # Two posts from the bar's ends to the shield's back, along its normal.
    for e, reach in zip(ends, posts):
        made += rod(e, e + n * ((reach if reach is not None else clear) + 0.006), radius * 0.8, 4)
    bmesh.ops.recalc_face_normals(bm, faces=made)
    bm.to_mesh(obj.data)
    bm.free()


def mesh_slots():
    from . import mesh
    return mesh.MATERIAL_SLOTS


def _place_grip(obj, armature, info):
    """A held item (a weapon, a shield, a lantern: `grip` in its `item.ron`) on its hand bone, not
    fitted to the body. It is modelled in its own frame: the grip at the origin, +Z along it
    (a bow's upper limb, a blade's point), +Y its back (a bow's string side). `grip_up` and
    `grip_back` say where those point in the hand bone's own frame (+Y along the bone, as
    Blender's bones), `grip_at` how far along the bone the grip sits (a fraction of its length).
    The official champion whose moves the weapon class plays sets them, so its clips aim the
    item as they aim theirs (a bow stands upright at the full draw). Rigid, on that bone alone,
    unless it brings bones of its own: `mftr_bones` on the item object, [(name, head, tail,
    parent)] in its grip frame ("grip" the hand), added to the rig, and its vertex groups (a
    "grip" group for what rides the hand). A bow's limbs and string, named as the official
    champion's, so its clips' channels for them play on it. `grip_scale`: see `place`."""
    import json
    bone = armature.data.bones[info["grip"]]
    up = Vector(info.get("grip_up", (0.0, 1.0, 0.0))).normalized()
    back = Vector(info.get("grip_back", (0.0, 0.0, 1.0)))
    back = (back - up * back.dot(up)).normalized()
    side = back.cross(up)
    at = Vector((0.0, bone.length * info.get("grip_at", 0.5), 0.0))
    m = armature.matrix_world @ bone.matrix_local
    held = info["grip"]
    if info.get("grasp_strap"):
        _strap_handle(obj, armature, held[-1], m, at, side, back, up, info["grasp_radius"])
    elif "grasp_radius" in info:
        # Gripped in the fist (see `hands`, and the forge's `grip`): it rides that hand's prop
        # bone, which the forge keys so the hand closes on it; the point `grasp_at` along it sits
        # in the fist's middle (the prop bone's head), turned as `grip_up` and `grip_back` say in
        # the hand's frame.
        prop = armature.data.bones["prop_" + held[-1]]
        prop.use_deform = True
        at = m.inverted() @ (armature.matrix_world @ prop.head_local) - up * info.get("grasp_at", 0.0)
        held = prop.name

    # Worn rather than held (a circlet on the head): modelled round a bone `grip_scale` long, and
    # scaled to this base's (a heavy build's head is a fifth bigger).
    k = bone.length / info["grip_scale"] if info.get("grip_scale") else 1.0

    def place(c):
        c = Vector(c) * k
        return m @ (at + side * c.x + back * c.y + up * c.z)

    for v in obj.data.vertices:
        v.co = place(v.co.copy())
    own = json.loads(obj.get("mftr_bones", "[]"))
    if not own:
        obj.vertex_groups.clear()
        g = obj.vertex_groups.new(name=held)
        g.add([v.index for v in obj.data.vertices], 1.0, "REPLACE")
        return
    if "grip" in obj.vertex_groups:
        obj.vertex_groups["grip"].name = held
    world_back = (m.to_3x3() @ back).normalized()
    inv = armature.matrix_world.inverted()
    prev = bpy.context.view_layer.objects.active
    bpy.context.view_layer.objects.active = armature
    bpy.ops.object.mode_set(mode="EDIT")
    eb = armature.data.edit_bones
    for name, head, tail, parent in own:
        b = eb.new(name)
        b.head, b.tail = inv @ place(head), inv @ place(tail)
        b.use_deform = True
        b.align_roll(inv.to_3x3() @ world_back)
        b.parent = eb[held if parent == "grip" else parent]
    bpy.ops.object.mode_set(mode="OBJECT")
    bpy.context.view_layer.objects.active = prev
    for pb in armature.pose.bones:
        pb.rotation_mode = "QUATERNION"


def fit_items(armature, body, gender, room=1.0):
    """Every wardrobe item fitted to `body` (a base just built, before its mass is baked) and
    skinned to `armature`, as catalog parts. Returns `made` entries (as `parts.build_parts`'s,
    with `covers`). `room` scales the layers' distances off the skin: the heavy builds' greater
    bulk moves further under the cloth."""
    from . import body_rig, chains, mesh, parts
    found = items()
    if not found or "src_co" not in body.data.attributes:
        return []
    sex = "male" if gender == "masculine" else "female"
    surfaces = {}
    made = []
    for folder, info in found:
        obj = load(folder)
        me = obj.data
        if info.get("grip"):
            # Held, not worn: on the hand, rigid; it covers nothing.
            _place_grip(obj, armature, info)
            obj.name = f"part.{info['id']}"
            me.name = obj.name
            _materials(obj)
            if "Col" in me.color_attributes:
                me.color_attributes.active_color = me.color_attributes["Col"]
                me.color_attributes.render_color_index = me.color_attributes.active_color_index
            obj.parent = armature
            mod = obj.modifiers.new("Armature", "ARMATURE")
            mod.object = armature
            flags = {"rigid": True}
            if "grasp_radius" in info:
                flags["grasp"] = {
                    "hand": info["grip"][-1], "radius": info["grasp_radius"], "at": info.get("grasp_at", 0.0),
                    "up": tuple(info.get("grip_up", (0.0, 1.0, 0.0))), "back": tuple(info.get("grip_back", (0.0, 0.0, 1.0))),
                    "span": tuple(info.get("grasp_span", (0.0, 0.0))), "edge": bool(info.get("grasp_edge", False)),
                    "on_hand": bool(info.get("grasp_on_hand", False)), "upright": bool(info.get("grasp_upright", False)),
                    "strap": bool(info.get("grasp_strap", False)),
                }
            made.append((info["id"], info["slot"], info["name"], [], bool(info["accent"]), mesh.triangle_count(obj), 0,
                         flags))
            continue
        arms = _arms(info)
        if arms not in surfaces:
            surfaces[arms] = (_source_surfaces(sex, arms), _base_surface(body, arms))
        to_sex, to_base = surfaces[arms]
        for v in me.vertices:
            p = v.co if sex == "male" else to_sex.carry(v.co)
            v.co = to_base.carry(p)
        offset = LAYERS.get(info["layer"], 0.006) * room
        _push_out(obj, body, offset, arms=arms)
        _clear_peaks(obj, body, offset, arms=arms)
        # Nothing sinks into the floor: a boot's sole pushed out off a heavy build's foot stays
        # within FLOOR of the body's lowest point.
        ground = min(v.co.z for v in body.data.vertices)
        for v in me.vertices:
            if v.co.z < ground - FLOOR:
                v.co.z = ground - FLOOR
        obj.name = f"part.{info['id']}"
        me.name = obj.name
        _materials(obj)
        if "Col" in me.color_attributes:
            me.color_attributes.active_color = me.color_attributes["Col"]
            me.color_attributes.render_color_index = me.color_attributes.active_color_index
        _weights(obj, body, set(body_rig.NAMES),
                 HANDS if arms == "upper" or info["slot"] in ("torso", "back", "waist", "legs") else (), arms)
        # What it has on the hands closes with the fingers, joint by joint.
        from . import hands as hand_rig
        hand_rig.glove(armature, obj)
        obj.parent = armature
        mod = obj.modifiers.new("Armature", "ARMATURE")
        mod.object = armature
        # The helpers' weights too: the item follows the body sliders and the sculpting.
        body_rig.transfer(body, obj)
        # A sim's mesh is irregular: smooth its helper weights longer, under the step limit.
        body_rig.limit_steps(obj, max_step=0.28, rounds=240)
        if info.get("hold"):
            # A hard thing standing off the body rides one bone whole (22 §4: a quiver's arrows
            # mustn't follow the arm); what lies on the body keeps the body's weights.
            _hold(obj, body, info["hold"], info.get("hold_from", HOLD_FROM))
        if info.get("chains"):
            # What hangs below the hips swings on spring-bone chains (22 §4.5, BW6).
            chains.hang_cloth(obj, armature, info["id"], int(info["chains"]), info.get("chain_turn", 45.0),
                              info.get("swing", 1.0), info.get("chain_from", "pelvis"), info.get("chain_arc"))
            # The chains take up to four bones a vertex, so a helper can drop out at one vertex and
            # stay on its neighbour (a long skirt's, below the hip): smoothed again around them.
            body_rig.limit_steps(obj, max_step=0.28, rounds=240)
        parts.transfer_shape_keys(body, obj)
        made.append((info["id"], info["slot"], info["name"], [], bool(info["accent"]), mesh.triangle_count(obj), 0,
                     {"covers": True, "rigid": bool(info["rigid"]), "layer": info["layer"]}))
    return made


def record_source(obj):
    """Before the fit: each body vertex's place on its source body (`src_co`), and which have
    one (`src_ok`); the head joined on later has none."""
    me = obj.data
    a = me.attributes.new("src_co", "FLOAT_VECTOR", "POINT")
    b = me.attributes.new("src_ok", "FLOAT", "POINT")
    for i, v in enumerate(me.vertices):
        a.data[i].vector = v.co
        b.data[i].value = 1.0


def strip_source(obj):
    """The fitting's attributes, gone before export."""
    me = obj.data
    for n in ("src_co", "src_ok"):
        if n in me.attributes:
            me.attributes.remove(me.attributes[n])
