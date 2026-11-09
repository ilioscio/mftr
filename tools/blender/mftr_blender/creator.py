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

# The builds (20 §3): per gender, the skeleton's placement (14 §2.2's heroic proportions), the
# girth of the body and of every part on it (`mesh.GIRTH`: torso, limbs, neck), the head and the
# hands. The heavy humanoid is the brute (Rook, Bastion, Cairn): low hips and short legs, the
# neck sunk between the shoulders, broad and deep, long arms, huge hands, a small head for its size.
BUILDS = {
    "humanoid_slim": {
        "masculine": {"shape": {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 1.04, "y": 1.0, "scale": 0.98, "shoulder": 0.03,
                                "hand": 1.3},
                      "girth": {"torso": (1.0, 1.0), "limb": (1.0, 1.0), "neck": (1.45, 1.4)}, "head": 1.36, "hands": 1.0},
        "feminine": {"shape": {"heights": [(0.0, 0.0), (2.15, 2.15)], "x": 0.96, "y": 0.98, "scale": 0.95, "shoulder": 0.0,
                               "hand": 1.2},
                     "girth": {"torso": (1.0, 1.0), "limb": (1.0, 1.0), "neck": (1.3, 1.25)}, "head": 1.3, "hands": 1.0},
    },
    "humanoid_heavy": {
        "masculine": {"shape": {"heights": [(0.0, 0.0), (0.03, 0.03), (0.09, 0.09), (0.54, 0.46), (1.00, 0.89), (1.12, 1.01),
                                            (1.26, 1.16), (1.40, 1.31), (1.56, 1.48), (1.64, 1.53), (1.88, 1.78), (2.15, 2.05)],
                                "x": 1.24, "y": 1.2, "scale": 1.1, "shoulder": 0.08, "arm": 1.06, "hand": 1.55},
                      "girth": {"torso": (1.3, 1.28), "limb": (1.22, 1.22), "neck": (1.55, 1.5)}, "head": 1.38, "hands": 1.2},
        "feminine": {"shape": {"heights": [(0.0, 0.0), (0.03, 0.03), (0.09, 0.09), (0.54, 0.48), (1.00, 0.91), (1.12, 1.03),
                                           (1.26, 1.18), (1.40, 1.33), (1.56, 1.50), (1.64, 1.55), (1.88, 1.80), (2.15, 2.07)],
                               "x": 1.16, "y": 1.12, "scale": 1.06, "shoulder": 0.06, "arm": 1.04, "hand": 1.4},
                     "girth": {"torso": (1.2, 1.18), "limb": (1.16, 1.16), "neck": (1.4, 1.35)}, "head": 1.32, "hands": 1.1},
    },
}

TORSO_GIRTH = ("pelvis", "spine_01", "spine_02", "chest")
LIMB_GIRTH = ("clavicle", "upperarm", "forearm", "thigh", "calf", "foot", "toe")


def _girth(table):
    """`mesh.GIRTH` for a build's girth table (None: as authored)."""
    if table is None:
        return None

    def of(bone):
        if bone in TORSO_GIRTH:
            return table["torso"]
        if bone == "neck":
            return table["neck"]
        if bone.rsplit("_", 1)[0] in LIMB_GIRTH:
            return table["limb"]
        return (1.0, 1.0)
    return of


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
        if bone == "neck":
            # The neck runs up inside the skull (the authored head sits over it, 21 §1.5): its
            # top never shows as a stump under the jaw, whatever the build.
            b = (0, 0.005, 1.76)
        out.append((bone, a, b, sides, prof, slot, color))
    return out


def base_head(gender, build="humanoid_slim"):
    """The base's head (call with the base's shape in use)."""
    # Heroic proportions (14 §2.2): about six heads tall, the jaw overhanging the neck. The
    # head's base sits a little low, so the neck runs up into the skull rather than ending in
    # a stump under it.
    masc = gender == "masculine"
    feats = ({} if masc else {"brow": 0.5, "jaw": 0.45, "chin": 0.7, "cheekbones": 1.25, "lips": 1.3, "nose": 0.75})
    return head.Head(base=rig.shaped((0, -0.012, 1.615)), height=0.255, scale=BUILDS[build][gender]["head"],
                     jaw=0.22 if masc else 0.4, chin=1.1 if masc else 0.9,
                     eye=1.0 if masc else 1.15, skin=SKIN, hair=(0.20, 0.14, 0.10), hair_style="bald",
                     detail=3, features=feats)


def _head_part(gender, build):
    def make(bm, layers, groups, mats):
        head.build(bm, layers, groups, mats, base_head(gender, build))
    return make


def use_build(gender, build="humanoid_slim"):
    """Put the skeleton's shape and the girth for `build` and `gender` in use (for everything
    built after it: the base and its parts)."""
    spec = BUILDS[build][gender]
    rig.use_shape(spec["shape"])
    mesh.GIRTH = _girth(spec["girth"])


def build_base(gender, name, build="humanoid_slim"):
    """The armature and the base mesh for `gender`, in the scene. Returns (armature, body)."""
    bpy.ops.wm.read_homefile(use_empty=True)
    bpy.context.scene.render.fps = 30
    use_build(gender, build)
    armature = rig.build_armature(name=name)
    # The authored head (21 §1.1, FR2) and the face rig on its landmarks (21 §1.3): the bones
    # first, so the eyeballs can be bound to the eye bones as the mesh is built.
    from . import authored_head, face_rig
    h = base_head(gender, build)
    mix = authored_head.MIX[(build, gender)]
    fit = authored_head.prepare(h, armature.data.bones["head"].head_local)
    piv = face_rig.add_bones(armature, h, face_rig.mirrored(authored_head.landmarks(fit)))
    head.DEFAULT_DETAIL = 3
    hands = BUILDS[build][gender]["hands"]

    def head_part(bm, layers, groups, mats):
        authored_head.build(bm, layers, groups, mats, h, mix, fit)

    try:
        parts = (champion.detailed(_body_parts(gender)) + champion.hand_parts("l", SKIN, scale=hands)
                 + champion.hand_parts("r", SKIN, scale=hands))
        body = mesh.build_mannequin(armature, name=name, part_list=parts + [head_part])
    finally:
        head.DEFAULT_DETAIL = 1
    face_rig.weight(armature, body, h, piv)
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
GAIN = {"face": 1.5, "body": 1.6}
# Each is (id, description, plus(regions) -> deltas, minus(regions) -> deltas or None for the
# mirror of plus). Ids are `region_feature` identifiers; recipes name them (`looks.shape`); the game reads them from the
# shape keys, so adding one here adds it to the creator.

def _face(r):
    """Landmarks on the face: heights from the head region's bounds, each set on the face's
    surface (the frontmost vertex near it), so a stroke at the eyes reaches the eyes and not the
    air in front of them now that the nose stands out (20 §5.2)."""
    lo, hi, s = r.lo, r.hi, r.size
    z = lambda t: lo.z + s.z * t  # noqa: E731

    def on(x, t, depth=0.0):
        pz = z(t)
        near = [r.cos[i].y for i in r.idx if abs(r.cos[i].x - x) < 0.012 and abs(r.cos[i].z - pz) < 0.012]
        return Vector((x, (min(near) if near else lo.y) + depth, pz))

    return {
        "chin": on(0.0, 0.05),
        "jaw": on(s.x * 0.33, 0.2),
        "cheek": on(s.x * 0.30, 0.47),
        "nose": on(0.0, 0.43),
        "brow": on(0.0, 0.66),
        "mouth": on(0.0, 0.27),
        "eye": on(0.034 * s.x / 0.2, 0.58),
        "ear": Vector((hi.x, (lo.y + hi.y) / 2, z(0.5))),
        "center": Vector((0.0, (lo.y + hi.y) / 2, z(0.5))),
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
         lambda r, f: {i: Vector((0, 0, -(0.55 - r.t(r.cos[i])) * r.size.z * 0.15)) for i in r.idx if r.t(r.cos[i]) < 0.55})
    face("jaw_width", "A wide, square jaw or a narrow one",
         lambda r, f: scale(r, f["center"], (1.35, 1.0, 1.0), lambda p: _smooth(abs(r.t(p) - 0.2) / 0.2) * _front(r, p)))
    face("chin_length", "A long chin or a short one",
         lambda r, f: grab(r, f["chin"], 0.09, Vector((0, 0, -0.016)), mirror=False))
    face("chin_reach", "A jutting chin or a receding one",
         lambda r, f: grab(r, f["chin"], 0.07, Vector((0, -0.022, 0)), mirror=False))
    face("cheekbones", "High, sharp cheekbones or flat ones",
         lambda r, f: grab(r, f["cheek"], 0.045, Vector((0.012, -0.008, 0.004))))
    face("cheeks_hollow", "Gaunt, hollow cheeks or full ones",
         lambda r, f: grab(r, f["cheek"] + Vector((0, 0.005, -0.05)), 0.05, Vector((-0.012, 0.006, 0))))
    face("brow_ridge", "A heavy brow ridge or a smooth forehead",
         lambda r, f: grab(r, f["brow"], 0.08, Vector((0, -0.015, -0.004)), mirror=False))
    face("nose_length", "A long nose or a short one",
         lambda r, f: grab(r, f["nose"], 0.045, Vector((0, -0.006, -0.02)), mirror=False))
    face("nose_reach", "A nose that stands far out, or a flat one",
         lambda r, f: grab(r, f["nose"], 0.04, Vector((0, -0.035, 0)), mirror=False))
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
    # The features a face reads by (20 §5.2): where the eyes sit and how they slant, the brow's
    # height and set, the lips and the mouth's corners, the forehead and crown, the jaw's corners.
    face("eyes_spacing", "Wide-set eyes or close-set ones",
         lambda r, f: grab(r, f["eye"], 0.032, Vector((0.008, 0, 0))))
    face("eyes_height", "Eyes set high in the face or low",
         lambda r, f: grab(r, f["eye"], 0.04, Vector((0, 0, 0.01))))
    face("eyes_tilt", "Eyes slanting up at the corners, or down",
         lambda r, f: _join(grab(r, f["eye"] + Vector((0.018, 0, 0)), 0.02, Vector((0, 0, 0.007))),
                            grab(r, f["eye"] - Vector((0.014, 0, 0)), 0.016, Vector((0, 0, -0.004)))))
    face("brow_height", "A brow set high or low over the eyes",
         lambda r, f: grab(r, f["brow"], 0.07, Vector((0, 0, 0.01)), mirror=False))
    face("brow_angle", "A scowling brow or a worried one",
         lambda r, f: _join(grab(r, f["brow"] + Vector((0.016, 0, -0.006)), 0.024, Vector((0, -0.004, -0.009))),
                            grab(r, f["brow"] + Vector((0.055, 0.004, -0.004)), 0.024, Vector((0, 0, 0.005)))))
    face("mouth_lips", "Full lips or thin ones",
         lambda r, f: _join(scale(r, f["mouth"], (1.03, 1.0, 1.4), lambda p: _smooth((p - f["mouth"]).length / 0.03)),
                            grab(r, f["mouth"], 0.03, Vector((0, -0.005, 0)), mirror=False)))
    face("mouth_height", "A mouth set high, near the nose, or low",
         lambda r, f: grab(r, f["mouth"], 0.035, Vector((0, 0, 0.009)), mirror=False))
    face("mouth_corners", "A smile or a frown",
         lambda r, f: grab(r, f["mouth"] + Vector((0.026, 0, 0)), 0.018, Vector((0, 0.002, 0.006))))
    face("head_forehead", "A tall, upright forehead or a sloping one",
         lambda r, f: grab(r, f["brow"] + Vector((0, 0.02, 0.07)), 0.08, Vector((0, -0.012, 0.01)), mirror=False))
    face("head_crown", "A tall crown or a flat one",
         lambda r, f: grab(r, Vector((0, (r.lo.y + r.hi.y) / 2, r.hi.z)), 0.12, Vector((0, 0.003, 0.011)), mirror=False))
    face("jaw_corners", "Heavy, dropped jaw corners or a tapered jaw",
         lambda r, f: grab(r, f["jaw"] + Vector((0.01, 0.02, -0.01)), 0.05, Vector((0.007, 0, -0.012))))
    face("nose_tip", "An upturned nose or a drooping one",
         lambda r, f: grab(r, f["nose"], 0.018, Vector((0, -0.003, 0.008)), mirror=False))

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
    """Add `Basis` and every slider's keys to `body`: `+` and `-`, or one signed `~` where the
    minus end mirrors the plus (half the morph data, which bases are capped by). Returns the
    slider ids."""
    body.shape_key_add(name="Basis", from_mix=False)
    ids = []
    for sid, _desc, groups, plus, minus in sliders():
        r = Region(body, set(groups))
        r.armature = armature.name
        f = _face(r) if groups == ("head",) else {}
        gain = GAIN["face" if groups == ("head",) else "body"]
        up = {i: v * gain for i, v in plus(r, f).items()}
        keys = (("+", up), ("-", {i: v * gain for i, v in minus(r, f).items()})) if minus is not None else (("~", up),)
        for suffix, deltas in keys:
            key = body.shape_key_add(name=sid + suffix, from_mix=False)
            for i, d in deltas.items():
                key.data[i].co = r.cos[i] + d
        ids.append(sid)
    return ids


# Where the body's islands go in its paint image (u0, v0, u1, v1): the head gets a corner of its
# own, about three times the density of the rest, since faces are what players paint most (war
# paint, tattoos); the rest of the body fills the band below. Parts use the whole square.
HEAD_RECT = (0.0, 0.0, 0.4, 0.4)
REST_RECT = (0.0, 0.42, 1.0, 1.0)


def _project(obj, faces, rect):
    """Smart-project `faces` (indices) of `obj` and fit their UVs into `rect`."""
    import bmesh as _bm
    bpy.ops.object.mode_set(mode="EDIT")
    bm = _bm.from_edit_mesh(obj.data)
    bm.faces.ensure_lookup_table()
    want = set(faces)
    for f in bm.faces:
        f.select_set(f.index in want)
    _bm.update_edit_mesh(obj.data)
    bpy.ops.uv.smart_project(angle_limit=math.radians(85), island_margin=0.02, area_weight=0.0, scale_to_bounds=True)
    bm = _bm.from_edit_mesh(obj.data)
    uv = bm.loops.layers.uv.active
    u0, v0, u1, v1 = rect
    for f in bm.faces:
        if f.index in want:
            for loop in f.loops:
                x, y = loop[uv].uv
                # Blender's v runs up; images' rows run down from the top (glTF flips v).
                loop[uv].uv = (u0 + x * (u1 - u0), 1.0 - (v0 + (1.0 - y) * (v1 - v0)))
    _bm.update_edit_mesh(obj.data)
    bpy.ops.object.mode_set(mode="OBJECT")


def unwrap(obj, head_group=None):
    """UVs for paint images (CR5b, D70): Blender's smart projection, islands packed with a margin
    so a brush stroke lands on one place of the model; with `head_group`, the faces skinned to
    it get `HEAD_RECT` and the rest `REST_RECT`."""
    if bpy.context.object and bpy.context.object.mode != "OBJECT":
        bpy.ops.object.mode_set(mode="OBJECT")
    for o in bpy.context.selected_objects:
        o.select_set(False)
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    if not obj.data.uv_layers:
        obj.data.uv_layers.new(name="Paint")
    everything = [p.index for p in obj.data.polygons]
    g = obj.vertex_groups.get(head_group) if head_group else None
    if g is None:
        _project(obj, everything, (0.0, 0.0, 1.0, 1.0))
    else:
        in_head = {v.index for v in obj.data.vertices if any(e.group == g.index and e.weight > 0.5 for e in v.groups)}
        head = [p.index for p in obj.data.polygons if all(v in in_head for v in p.vertices)]
        rest = [i for i in everything if i not in set(head)]
        _project(obj, rest, REST_RECT)
        _project(obj, head, HEAD_RECT)
    obj.select_set(False)


def build(gender, name, build="humanoid_slim"):
    """The base of `build` for `gender` with its sliders and every catalog part (CR4). Returns
    (armature, body, parts made)."""
    from . import parts
    armature, body = build_base(gender, name, build)
    ids = add_shape_keys(armature, body)
    # The catalog's head parts were shaped on the block-out head: conformed to the authored one.
    from . import authored_head
    h = base_head(gender, build)
    fit = authored_head.prepare(h, armature.data.bones["head"].head_local)
    made = parts.build_parts(armature, body, h, conform=authored_head.make_conform(h, fit, body))
    # Weapons ride the prop bones, as the official champions' do, so the moves' clips (whose
    # wrists stay human, 14 §3.3) hold them where their champions held theirs.
    from . import wrists
    wrists.rebind_weapons(armature, [o for o in bpy.data.objects if o.name.startswith(("part.weapon.", "part.offhand."))])
    mesh.GIRTH = None
    unwrap(body, head_group="head")
    for obj in [o for o in bpy.data.objects if o.name.startswith("part.")]:
        unwrap(obj)
    armature["mftr_build"] = name
    armature["mftr_sliders"] = ",".join(ids)
    keys = len(body.data.shape_keys.key_blocks) - 1
    print(f"{name}: {mesh.triangle_count(body)} triangles, {len(ids)} sliders ({keys} shape keys), {len(made)} parts")
    return armature, body, made
