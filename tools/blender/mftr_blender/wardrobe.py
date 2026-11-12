# SPDX-License-Identifier: AGPL-3.0-or-later
"""The wardrobe (22 §4, BW5): clothing, armour and accessories authored once and fitted to every
humanoid base at build time.

**An item** lives in `art/wardrobe/<slot>/<name>/`:

- `item.ron`: what it is (`id`, `slot`, `name`, `layer`, `accent`, `rigid`), as the catalog lists it;
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

import os
import re

import bpy
from mathutils import Vector
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
    """`item.ron`'s fields: strings, booleans and numbers of one flat struct."""
    text = re.sub(r"//[^\n]*", "", open(path, encoding="utf-8").read())
    out = {}
    for key, val in re.findall(r"(\w+)\s*:\s*(\"[^\"]*\"|true|false|-?[\d.]+)", text):
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

    def carry(self, p, k=6):
        """Where `p` (near the rest surface) goes on the moved one: the mean, by closeness, of
        what each of its `k` nearest faces makes of it (its barycentric place there, and its
        offset in the face's frame)."""
        loc, _n, _i, d = self.tree.find_nearest(p)
        if loc is None:
            return p.copy()
        near = self.tree.find_nearest_range(p, d * 1.6 + 0.01)
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
            acc += (q2 + f1 * lo[0] + f2 * lo[1] + m * lo[2]) * wgt
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


def _source_surfaces(sex):
    """The reference body (the CC0 male) and, for a female base, the female body on the same
    topology: the surface that carries an item onto the base's sex."""
    from . import cc0_body
    male = cc0_body.load("male")
    tris = _tris(male.data)
    rest = [v.co.copy() for v in male.data.vertices]
    if sex == "male":
        moved = rest
    else:
        female = cc0_body.load("female")
        moved = [v.co.copy() for v in female.data.vertices]
        bpy.data.objects.remove(female)
    bpy.data.objects.remove(male)
    return Surface(rest, moved, tris)


def _base_surface(body):
    """The base's body as it stood on its source body (`src_co`) and where the fit put it."""
    me = body.data
    src = me.attributes["src_co"].data
    ok = me.attributes["src_ok"].data
    keep = [d.value > 0.5 for d in ok]
    rest = [Vector(d.vector) for d in src]
    moved = [v.co.copy() for v in me.vertices]
    return Surface(rest, moved, _tris(me, keep))


# --- Fitting ----------------------------------------------------------------------------------

def _push_out(obj, body, offset, reach=0.05):
    """No item vertex nearer the skin than `offset`: one inside or too close goes out along the
    skin's normal (the body's outward faces), those farther than `reach` (a hem hanging free)
    stay."""
    bme = body.data
    keep = [d.value > 0.5 for d in bme.attributes["src_ok"].data]
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


def _weights(obj, body, skip):
    """Each item vertex takes the biped weights of the body's surface under it (the nearest
    face's, by barycentric place), its two largest, normalized; helpers (`skip`) not."""
    bme = body.data
    names = {g.index: g.name for g in body.vertex_groups}
    keep = [d.value > 0.5 for d in bme.attributes["src_ok"].data]
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


def fit_items(armature, body, gender):
    """Every wardrobe item fitted to `body` (a base just built, before its mass is baked) and
    skinned to `armature`, as catalog parts. Returns `made` entries (as `parts.build_parts`'s,
    with `covers`)."""
    from . import body_rig, mesh, parts
    found = items()
    if not found or "src_co" not in body.data.attributes:
        return []
    sex = "male" if gender == "masculine" else "female"
    to_sex = _source_surfaces(sex)
    to_base = _base_surface(body)
    made = []
    for folder, info in found:
        obj = load(folder)
        me = obj.data
        for v in me.vertices:
            p = v.co if sex == "male" else to_sex.carry(v.co)
            v.co = to_base.carry(p)
        _push_out(obj, body, LAYERS.get(info["layer"], 0.006))
        obj.name = f"part.{info['id']}"
        me.name = obj.name
        _materials(obj)
        if "Col" in me.color_attributes:
            me.color_attributes.active_color = me.color_attributes["Col"]
            me.color_attributes.render_color_index = me.color_attributes.active_color_index
        _weights(obj, body, set(body_rig.NAMES))
        obj.parent = armature
        mod = obj.modifiers.new("Armature", "ARMATURE")
        mod.object = armature
        # The helpers' weights too: the item follows the body sliders and the sculpting.
        body_rig.transfer(body, obj)
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
