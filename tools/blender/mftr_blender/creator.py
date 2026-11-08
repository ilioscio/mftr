# SPDX-License-Identifier: AGPL-3.0-or-later
"""The champion creator's catalog (CR2, 20 §5–6, D69): a base body per build and gender, and the
face and body sliders as morph targets on it.

A base is an ordinary MFTR model (the `biped` skeleton, one rigidly skinned mesh, vertex colors)
whose mesh carries **shape keys**: two per slider, `<slider>+` and `<slider>-`, its two extremes.
The game blends them by the slider's value (`mftr-forge`): +0.5 is half of `+`, -1 all of `-`.
Proportion sliders (height, limb lengths, head and hand size) aren't shape keys: the game scales
the skeleton for them.

The extremes here are **block-outs**: scripted strokes of a grab brush (move what's near a
landmark, with a smooth falloff) and region scales, the moves a sculptor makes. They are meant to
be re-sculpted by hand in Blender (D49); the slider list, names and ranges are the contract the
game and recipes rely on, and `SLIDERS` is where they're defined.

    blender -b --python art/creator/humanoid_slim/build.py -- --out art/creator/humanoid_slim/humanoid_slim.blend
"""

import math

import bmesh
import bpy
from mathutils import Vector

from . import champion, head, mesh, rig

# Neutral base colors: an undersuit, skin, a dark belt and boots. The paint step (CR5) recolors.
SUIT = (0.30, 0.30, 0.33)
SUIT_DARK = (0.16, 0.16, 0.18)
SKIN = (0.80, 0.60, 0.48)
LEATHER = (0.30, 0.20, 0.12)
ACCENT = (0.18, 0.52, 0.95)

# Body shapes per gender (14 §2.2's heroic proportions): the skeleton's placement.
SHAPES = {
    "masculine": {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 1.04, "y": 1.0, "scale": 0.98, "shoulder": 0.03, "hand": 1.3},
    "feminine": {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 0.96, "y": 0.98, "scale": 0.95, "shoulder": 0.0, "hand": 1.2},
}


def _body_parts(gender):
    """The template's lofted body (mesh.parts) as an undersuit: no armor, neutral colors, the
    team accent kept; feminine bodies narrower at the waist and wider at the hips."""
    out = []
    recolor = {mesh.TUNIC: SUIT, mesh.DARK: SUIT_DARK, mesh.LEATHER: LEATHER, mesh.ACCENT: ACCENT}
    for p in mesh.parts():
        if callable(p):
            continue  # the template's head: ours is built below
        bone, a, b, sides, prof, slot, color = p
        if slot in ("metal", "emissive") or bone.startswith(("hand_", "fingers_", "thumb_")):
            continue  # no armor; sculpted hands are added below
        color = recolor.get(color, color)
        # Heroic proportions (14 §2.2–2.3): limbs that read as solids, a fuller torso.
        limb = bone.split("_")[0] in ("upperarm", "forearm", "thigh", "calf")
        k = 1.3 if limb else (1.1 if bone in ("pelvis", "spine_01", "spine_02", "chest") else 1.0)
        prof = [(t, rr * k, d * k) for t, rr, d in prof]
        if gender == "feminine" and bone in ("spine_01", "spine_02"):
            prof = [(t, r * 0.88, d * 0.95) for t, r, d in prof]
        if gender == "feminine" and bone == "pelvis" and slot == "cloth":
            prof = [(t, r * 1.08, d * 1.05) for t, r, d in prof]
        out.append((bone, a, b, sides, prof, slot, color))
    return out


def _head_part(gender):
    def build(bm, layers, groups, mats):
        h = head.Head(base=rig.shaped((0, -0.01, 1.63)), height=0.255, scale=1.12 if gender == "masculine" else 1.08,
                      jaw=0.3 if gender == "masculine" else 0.45, chin=1.1 if gender == "masculine" else 0.9,
                      eye=1.0 if gender == "masculine" else 1.15, skin=SKIN, hair=(0.20, 0.14, 0.10), hair_style="bald")
        head.build(bm, layers, groups, mats, h)
    return build


def build_base(gender, name):
    """The armature and the base mesh for `gender`, in the scene. Returns (armature, body)."""
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    rig.use_shape(SHAPES[gender])
    armature = rig.build_armature(name=name)
    head.DEFAULT_DETAIL = 2
    try:
        parts = champion.detailed(_body_parts(gender)) + champion.hand_parts("l", SKIN) + champion.hand_parts("r", SKIN)
        body = mesh.build_mannequin(armature, name=name, part_list=parts + [_head_part(gender)])
    finally:
        head.DEFAULT_DETAIL = 1
    return armature, body


# --- Sculpting tools (block-out strokes) ------------------------------------------------------

def _smooth(u):
    u = max(0.0, min(1.0, u))
    return 1.0 - u * u * (3 - 2 * u)


class Region:
    """The rest-pose vertices of one bone group (armature space), and the group's bounds."""

    def __init__(self, body, groups):
        names = {g.index: g.name for g in body.vertex_groups}
        self.cos = [v.co.copy() for v in body.data.vertices]
        self.of = []
        for v in body.data.vertices:
            gs = [names[g.group] for g in v.groups if g.weight > 0.5]
            self.of.append(gs[0] if gs else "")
        self.idx = [i for i, g in enumerate(self.of) if g in groups]
        pts = [self.cos[i] for i in self.idx]
        self.lo = Vector((min(p.x for p in pts), min(p.y for p in pts), min(p.z for p in pts)))
        self.hi = Vector((max(p.x for p in pts), max(p.y for p in pts), max(p.z for p in pts)))

    def t(self, p):
        """Height in the region, 0 at its bottom to 1 at its top."""
        return (p.z - self.lo.z) / max(1e-6, self.hi.z - self.lo.z)

    @property
    def size(self):
        return self.hi - self.lo


def grab(region, center, radius, offset, mirror=True):
    """A grab-brush stroke: vertices within `radius` of `center` move by `offset`, fading to
    nothing at the edge; mirrored across X (symmetry on)."""
    out = {}
    for c, off in [(center, offset)] + ([(Vector((-center.x, center.y, center.z)), Vector((-offset.x, offset.y, offset.z)))]
                                         if mirror and abs(center.x) > 1e-4 else []):
        for i in region.idx:
            w = _smooth((region.cos[i] - c).length / radius)
            if w > 0:
                out[i] = out.get(i, Vector()) + off * w
    return out


def scale(region, pivot, factor, weight=lambda p: 1.0):
    """Scale the region's vertices about `pivot` by `factor` (per axis), each by `weight(p)`."""
    out = {}
    for i in region.idx:
        p = region.cos[i]
        w = weight(p)
        if w > 0:
            d = p - pivot
            out[i] = Vector((d.x * (factor[0] - 1), d.y * (factor[1] - 1), d.z * (factor[2] - 1))) * w
    return out


def _join(*parts):
    out = {}
    for p in parts:
        for i, v in p.items():
            out[i] = out.get(i, Vector()) + v
    return out


# --- The sliders ------------------------------------------------------------------------------
# How far the block-out strokes go at ±1: past natural faces and bodies on purpose (20 §5.2).
GAIN = {"face": 2.0, "body": 1.6}
# Each is (id, description, plus(regions) -> deltas, minus(regions) -> deltas or None for the
# mirror of plus). Ids are `region_feature` identifiers; recipes name them (`looks.shape`); the game reads them from the
# shape keys, so adding one here adds it to the creator.

def _face(r):
    """Landmarks on the face: the head region's bounds give its proportions."""
    lo, hi, s = r.lo, r.hi, r.size
    cx, front = 0.0, lo.y
    z = lambda t: lo.z + s.z * t  # noqa: E731
    return {
        "chin": Vector((cx, front + 0.02, z(0.05))),
        "jaw": Vector((s.x * 0.33, front + 0.05, z(0.2))),
        "cheek": Vector((s.x * 0.30, front + 0.03, z(0.47))),
        "nose": Vector((cx, front - 0.01, z(0.43))),
        "brow": Vector((cx, front + 0.01, z(0.66))),
        "mouth": Vector((cx, front + 0.005, z(0.27))),
        "eye": Vector((0.034 * s.x / 0.2, front + 0.01, z(0.58))),
        "ear": Vector((hi.x, (lo.y + hi.y) / 2, z(0.5))),
        "center": Vector((cx, (lo.y + hi.y) / 2, z(0.5))),
    }


def _front(r, p, depth=0.06):
    """1 on the face, fading to 0 toward the back of the head."""
    return _smooth((p.y - r.lo.y) / (r.size.y * 0.6))


def _sliders():
    H = ("head",)  # noqa: N806
    out = []

    def face(sid, desc, plus, minus=None):
        out.append((sid, desc, H, plus, minus))

    face("head_width", "A broad or a narrow head",
         lambda r, f: scale(r, f["center"], (1.18, 1.0, 1.0)))
    face("face_length", "A long or a short lower face",
         lambda r, f: {i: Vector((0, 0, -(0.55 - r.t(r.cos[i])) * r.size.z * 0.22)) for i in r.idx if r.t(r.cos[i]) < 0.55})
    face("jaw_width", "A wide, square jaw or a narrow one",
         lambda r, f: scale(r, f["center"], (1.35, 1.0, 1.0), lambda p: _smooth(abs(r.t(p) - 0.2) / 0.2) * _front(r, p)))
    face("chin_length", "A long chin or a short one",
         lambda r, f: grab(r, f["chin"], 0.06, Vector((0, 0, -0.025)), mirror=False))
    face("chin_reach", "A jutting chin or a receding one",
         lambda r, f: grab(r, f["chin"], 0.05, Vector((0, -0.025, 0)), mirror=False))
    face("cheekbones", "High, sharp cheekbones or flat ones",
         lambda r, f: grab(r, f["cheek"], 0.045, Vector((0.012, -0.008, 0.004))))
    face("cheeks_hollow", "Gaunt, hollow cheeks or full ones",
         lambda r, f: grab(r, f["cheek"] + Vector((0, 0.005, -0.05)), 0.05, Vector((-0.012, 0.006, 0))))
    face("brow_ridge", "A heavy brow ridge or a smooth forehead",
         lambda r, f: grab(r, f["brow"], 0.08, Vector((0, -0.015, -0.004)), mirror=False))
    face("nose_length", "A long nose or a short one",
         lambda r, f: grab(r, f["nose"], 0.035, Vector((0, -0.01, -0.03)), mirror=False))
    face("nose_reach", "A nose that stands far out, or a flat one",
         lambda r, f: grab(r, f["nose"], 0.03, Vector((0, -0.045, 0)), mirror=False))
    face("nose_width", "A broad nose or a thin one",
         lambda r, f: scale(r, f["nose"], (1.7, 1.0, 1.0), lambda p: _smooth((p - f["nose"]).length / 0.04)))
    face("nose_hook", "A hooked bridge or a scooped one",
         lambda r, f: grab(r, f["nose"] + Vector((0, 0, 0.04)), 0.025, Vector((0, -0.015, 0)), mirror=False))
    face("mouth_width", "A wide mouth or a small one",
         lambda r, f: scale(r, f["mouth"], (1.5, 1.0, 1.0), lambda p: _smooth((p - f["mouth"]).length / 0.045)))
    face("eyes_size", "Big eyes or small ones",
         lambda r, f: _join(scale(r, f["eye"], (1.35, 1.0, 1.35), lambda p: _smooth((p - f["eye"]).length / 0.03)),
                            scale(r, f["eye"] * Vector((-1, 1, 1)), (1.35, 1.0, 1.35),
                                  lambda p: _smooth((p - f["eye"] * Vector((-1, 1, 1))).length / 0.03))))
    face("ears_size", "Big ears or small ones",
         lambda r, f: _join(scale(r, f["ear"], (1.0, 1.5, 1.5), lambda p: _smooth((p - f["ear"]).length / 0.05) * (p.x > r.hi.x * 0.85)),
                            scale(r, f["ear"] * Vector((-1, 1, 1)), (1.0, 1.5, 1.5),
                                  lambda p: _smooth((p - f["ear"] * Vector((-1, 1, 1))).length / 0.05) * (p.x < -r.hi.x * 0.85))))
    face("ears_point", "Pointed ears",
         lambda r, f: _join(grab(r, f["ear"] + Vector((0, 0.01, 0.03)), 0.03, Vector((0.01, 0.02, 0.05)))),
         lambda r, f: {})

    def body(sid, desc, groups, plus, minus=None):
        out.append((sid, desc, groups, plus, minus))

    TORSO = ("pelvis", "spine_01", "spine_02", "chest")  # noqa: N806
    LIMBS = tuple(f"{b}_{s}" for b in ("upperarm", "forearm", "thigh", "calf") for s in "lr")  # noqa: N806

    def radial(r, axis_x=0.0, factor=1.15):
        return scale(r, Vector((axis_x, (r.lo.y + r.hi.y) / 2, 0)), (factor, factor, 1.0))

    body("body_bulk", "A heavy body or a lean one", TORSO + ("neck",), lambda r, f: radial(r, factor=1.18))
    body("body_belly", "A belly or a flat stomach", TORSO,
         lambda r, f: grab(r, Vector((0, r.lo.y, r.lo.z + r.size.z * 0.35)), 0.2, Vector((0, -0.07, -0.01)), mirror=False),
         lambda r, f: grab(r, Vector((0, r.lo.y, r.lo.z + r.size.z * 0.35)), 0.2, Vector((0, 0.025, 0)), mirror=False))
    body("body_chest", "A deep chest (or bust) or a flat one", TORSO,
         lambda r, f: grab(r, Vector((0.08, r.lo.y, r.lo.z + r.size.z * 0.72)), 0.11, Vector((0, -0.05, 0.0))))
    body("body_hips", "Wide hips or narrow ones", ("pelvis", "thigh_l", "thigh_r"),
         lambda r, f: scale(r, Vector((0, 0, 0)), (1.18, 1.05, 1.0), lambda p: _smooth(abs(p.z - (r.hi.z - 0.08)) / 0.25)))
    body("body_shoulders", "Broad shoulders or narrow ones", ("chest", "clavicle_l", "clavicle_r", "upperarm_l", "upperarm_r"),
         lambda r, f: scale(r, Vector((0, 0, 0)), (1.15, 1.0, 1.0), lambda p: _smooth(abs(p.z - r.hi.z) / 0.3)))
    body("body_muscle", "Thick, muscled limbs or thin ones", LIMBS,
         lambda r, f: _limb_scale(r, 1.25))
    body("neck_thickness", "A thick neck or a thin one", ("neck",), lambda r, f: radial(r, factor=1.35))
    return out


def _limb_scale(r, factor):
    """Each limb vertex pushed out from its bone's axis."""
    arm = bpy.data.objects[r.armature]
    out = {}
    for i in r.idx:
        b = arm.data.bones[r.of[i]]
        a, t = b.head_local, b.tail_local
        p = r.cos[i]
        d = t - a
        u = max(0.0, min(1.0, (p - a).dot(d) / max(1e-6, d.length_squared)))
        on = a + d * u
        out[i] = (p - on) * (factor - 1)
    return out


SLIDERS = None


def sliders():
    global SLIDERS
    if SLIDERS is None:
        SLIDERS = _sliders()
    return SLIDERS


def add_shape_keys(armature, body):
    """Add `Basis` and every slider's `+` and `-` keys to `body`. Returns the slider ids."""
    body.shape_key_add(name="Basis", from_mix=False)
    ids = []
    for sid, _desc, groups, plus, minus in sliders():
        r = Region(body, set(groups))
        r.armature = armature.name
        f = _face(r) if groups == ("head",) else {}
        gain = GAIN["face" if groups == ("head",) else "body"]
        up = {i: v * gain for i, v in plus(r, f).items()}
        down = {i: v * gain for i, v in minus(r, f).items()} if minus is not None else {i: -v for i, v in up.items()}
        for suffix, deltas in (("+", up), ("-", down)):
            key = body.shape_key_add(name=sid + suffix, from_mix=False)
            for i, d in deltas.items():
                key.data[i].co = r.cos[i] + d
        ids.append(sid)
    return ids


def build(gender, name):
    armature, body = build_base(gender, name)
    ids = add_shape_keys(armature, body)
    armature["mftr_build"] = name
    armature["mftr_sliders"] = ",".join(ids)
    print(f"{name}: {mesh.triangle_count(body)} triangles, {len(ids)} sliders ({2 * len(ids)} shape keys)")
    return armature, body
