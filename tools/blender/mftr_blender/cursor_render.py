# SPDX-License-Identifier: AGPL-3.0-or-later
"""The game cursor rendered (05 §6): the steel gauntlet (`gauntlet`) on the open base's right
hand, posed pointing, lit as polished steel, outlined.

Every state is the same gauntlet in the same pose, so the fingertip, the click, is in the same
pixel in each:

| State | What |
|---|---|
| `default` | bare steel |
| `enemy` | its plates enamelled red (an enemy that can be attacked) |
| `ally` | teal (one of ours) |
| `shop` | a bag of coins in its grip (our own shop) |
| `attack` | a steel reticle with red enamel (attack-move waiting for its click) |

**The look:** the plates are polished steel under a bright sky and a key light from the upper
left, gold trim, leather between. The render is 4× the master's size, reduced, then outlined:
a dark outline all round, so it reads on any ground, and a light edge on the sides toward the
light, as the genre's cursors have.

**The click:** the fingertip's point is projected into the image (`hotspots.gd`), so the game
clicks where the finger points, to the pixel.
"""

import math
import os

import bpy
import numpy as np
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Matrix, Vector

from . import gauntlet

# The masters' size (px) and the render's supersampling.
SIZE = 128
SUPER = 4
# The outline's width and color (master px), and the light edge's.
OUTLINE = 3.2
OUTLINE_COLOR = (0.105, 0.078, 0.055)
EDGE = 1.6
# How the hand lies on screen: the pointing finger turned this far left of straight up
# (degrees), and the hand rolled this far about its length, the thumb side away (degrees).
POINT = 35.0
ROLL = -10.0
# The fingertip's place in the master (px from the top left).
TIP = (5.0, 5.0)


# --- The scene ---------------------------------------------------------------------------------

def _principled(name, color, metallic, roughness, coat=0.0):
    m = bpy.data.materials.new(name)
    m.use_nodes = True
    b = m.node_tree.nodes.get("Principled BSDF")
    b.inputs["Base Color"].default_value = (*color, 1.0)
    b.inputs["Metallic"].default_value = metallic
    b.inputs["Roughness"].default_value = roughness
    if coat and "Coat Weight" in b.inputs:
        b.inputs["Coat Weight"].default_value = coat
    return m


# Each state's plate color (steel by default, enamelled for enemy and ally).
PLATES = {
    "default": ((0.58, 0.60, 0.65), 1.0, 0.3),
    "enemy": ((0.78, 0.16, 0.10), 1.0, 0.3),
    "ally": ((0.10, 0.62, 0.56), 1.0, 0.3),
    "shop": ((0.58, 0.60, 0.65), 1.0, 0.3),
}


def _scene():
    sc = bpy.context.scene
    sc.render.engine = "CYCLES"
    sc.cycles.samples = 96
    sc.cycles.use_denoising = True
    sc.render.film_transparent = True
    sc.render.resolution_x = sc.render.resolution_y = SIZE * SUPER
    sc.render.image_settings.file_format = "PNG"
    sc.render.image_settings.color_mode = "RGBA"
    sc.view_settings.view_transform = "Standard"
    sc.view_settings.look = "None"
    # The sky: bright and cool above, dark and warm below, for the steel to mirror.
    world = bpy.data.worlds.new("sky")
    sc.world = world
    world.use_nodes = True
    nt = world.node_tree
    bg = nt.nodes["Background"]
    tex = nt.nodes.new("ShaderNodeTexCoord")
    sep = nt.nodes.new("ShaderNodeSeparateXYZ")
    ramp = nt.nodes.new("ShaderNodeValToRGB")
    nt.links.new(tex.outputs["Generated"], sep.inputs[0])
    nt.links.new(sep.outputs["Z"], ramp.inputs["Fac"])
    nt.links.new(ramp.outputs["Color"], bg.inputs["Color"])
    els = ramp.color_ramp.elements
    els[0].position, els[0].color = 0.42, (0.05, 0.035, 0.03, 1)
    els[1].position, els[1].color = 0.62, (0.62, 0.66, 0.74, 1)
    mid = els.new(0.52)
    mid.color = (0.32, 0.30, 0.30, 1)
    bg.inputs["Strength"].default_value = 1.0
    cam = bpy.data.objects.new("cam", bpy.data.cameras.new("cam"))
    sc.collection.objects.link(cam)
    sc.camera = cam
    cam.data.type = "ORTHO"
    cam.location = (0, 0, 2)
    # Lights: a key from the upper left and in front, a cool rim from behind on the right.
    for name, loc, energy, size, color in (("key", (-1.2, 1.4, 2.0), 210.0, 0.8, (1.0, 0.97, 0.9)),
                                            ("rim", (1.4, -0.6, -0.8), 160.0, 0.5, (0.75, 0.85, 1.0)),
                                            ("fill", (0.6, -1.4, 1.6), 50.0, 1.5, (1.0, 0.9, 0.8))):
        ld = bpy.data.lights.new(name, "AREA")
        ld.energy, ld.size, ld.color = energy, size, color
        lo = bpy.data.objects.new(name, ld)
        sc.collection.objects.link(lo)
        lo.location = loc
        lo.rotation_mode = "QUATERNION"
        lo.rotation_quaternion = (-Vector(loc)).to_track_quat("-Z", "Y")
    return cam


def _placement():
    """The hand's frame to the world's: the back of the hand to the camera (+Z), the fingers up
    and turned left by `POINT`, rolled by `ROLL`."""
    roll = Matrix.Rotation(math.radians(ROLL), 4, "Y")
    turn = Matrix.Rotation(math.radians(POINT), 4, "Z")
    return turn @ roll


def _frame_camera(cam, objs):
    """The camera framed so the whole model fits the master, a margin for the outline round it."""
    pts = [o.matrix_world @ v.co for o in objs for v in o.data.vertices]
    xs, ys = [p.x for p in pts], [p.y for p in pts]
    span = max(max(xs) - min(xs), max(ys) - min(ys))
    margin = OUTLINE + 2.0
    cam.data.ortho_scale = span * SIZE / (SIZE - 2 * margin)
    cam.location = ((max(xs) + min(xs)) / 2, (max(ys) + min(ys)) / 2, 2.0)


# --- The models -------------------------------------------------------------------------------

def _gauntlet(hand, pieces, state):
    color, metal, rough = PLATES[state]
    mats = {"steel": _principled(f"steel_{state}", color, metal, rough),
            "leather": _principled("leather", (0.045, 0.028, 0.018), 0.0, 0.75),
            "gold": _principled("gold", (1.0, 0.70, 0.28), 1.0, 0.28)}
    posed = gauntlet.pose(hand, pieces, gauntlet.POINTING)
    obj = gauntlet.to_object(f"gauntlet_{state}", pieces, posed, mats)
    for p in obj.data.polygons:
        p.use_smooth = True
    obj.data.set_sharp_from_angle(angle=math.radians(40))
    obj.matrix_world = _placement()
    return obj


def _coin_bag(at):
    """A bag of coins at `at`: leather, tied at the neck, gold coins on its mouth."""
    bag = _principled("bag", (0.075, 0.038, 0.016), 0.0, 0.65)
    gold = _principled("coin", (1.0, 0.74, 0.25), 1.0, 0.22)
    out = []
    bpy.ops.mesh.primitive_uv_sphere_add(segments=16, ring_count=10, radius=0.042, location=at)
    b = bpy.context.object
    b.scale = (1.0, 0.95, 0.85)
    b.data.materials.append(bag)
    out.append(b)
    bpy.ops.mesh.primitive_cone_add(vertices=12, radius1=0.022, radius2=0.012, depth=0.03,
                                    location=at + Vector((0.0, 0.042, 0.0)), rotation=(math.radians(-90), 0, 0))
    n = bpy.context.object
    n.data.materials.append(bag)
    out.append(n)
    for k, off in enumerate((Vector((0.012, -0.004, 0.036)), Vector((-0.016, 0.012, 0.03)))):
        bpy.ops.mesh.primitive_cylinder_add(vertices=16, radius=0.016, depth=0.005, location=at + off,
                                            rotation=(math.radians(18 + 12 * k), math.radians(-10), 0))
        c = bpy.context.object
        c.data.materials.append(gold)
        out.append(c)
    for o in out:
        for p in o.data.polygons:
            p.use_smooth = True
    return out


def _reticle():
    """Attack-move's reticle: a steel ring, four steel spikes pointing in, red enamel between."""
    steel = _principled("ret_steel", (0.82, 0.84, 0.88), 1.0, 0.22)
    red = _principled("ret_red", (0.78, 0.10, 0.07), 1.0, 0.3)
    out = []
    bpy.ops.mesh.primitive_torus_add(major_radius=0.07, minor_radius=0.011, major_segments=40, minor_segments=10)
    t = bpy.context.object
    t.data.materials.append(steel)
    out.append(t)
    bpy.ops.mesh.primitive_torus_add(major_radius=0.07, minor_radius=0.0065, major_segments=40, minor_segments=8,
                                     location=(0, 0, 0.007))
    t2 = bpy.context.object
    t2.data.materials.append(red)
    out.append(t2)
    for k in range(4):
        a = math.pi / 2 * k + math.pi / 4
        d = Vector((math.cos(a), math.sin(a), 0))
        bpy.ops.mesh.primitive_cone_add(vertices=8, radius1=0.016, radius2=0.0, depth=0.05,
                                        location=d * 0.058)
        c = bpy.context.object
        c.rotation_mode = "QUATERNION"
        c.rotation_quaternion = (-d).to_track_quat("Z", "Y")
        c.data.materials.append(steel)
        out.append(c)
    bpy.ops.mesh.primitive_uv_sphere_add(segments=12, ring_count=8, radius=0.008)
    dot = bpy.context.object
    dot.data.materials.append(red)
    out.append(dot)
    for o in out:
        for p in o.data.polygons:
            p.use_smooth = True
    return out


# --- Rendering and finishing --------------------------------------------------------------------

def _render(path):
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(path)
    w, h = img.size
    px = np.array(img.pixels[:], dtype=np.float32).reshape(h, w, 4)[::-1]  # top row first
    bpy.data.images.remove(img)
    return px


def _reduce(px):
    """The supersampled render to the master's size (alpha-weighted box filter)."""
    h, w, _ = px.shape
    k = SUPER
    a = px[..., 3:4]
    rgb = (px[..., :3] * a).reshape(h // k, k, w // k, k, 3).sum(axis=(1, 3))
    al = a.reshape(h // k, k, w // k, k, 1).sum(axis=(1, 3))
    out = np.zeros((h // k, w // k, 4), dtype=np.float32)
    out[..., :3] = np.where(al > 0, rgb / np.maximum(al, 1e-6), 0)
    out[..., 3:4] = al / (k * k)
    return out


def _shift(a, dy, dx):
    """`a` moved by (`dy`, `dx`) px, zeros coming in (no wrapping round)."""
    out = np.zeros_like(a)
    h, w = a.shape
    out[max(dy, 0):h + min(dy, 0), max(dx, 0):w + min(dx, 0)] = a[max(-dy, 0):h + min(-dy, 0), max(-dx, 0):w + min(-dx, 0)]
    return out


def _dilate(a, r):
    """`a`'s maximum over a disc of radius `r` (px), with a soft edge."""
    out = np.zeros_like(a)
    n = int(math.ceil(r))
    for dy in range(-n, n + 1):
        for dx in range(-n, n + 1):
            d = math.hypot(dx, dy)
            if d > r + 0.5:
                continue
            k = min(1.0, r + 0.5 - d)
            out = np.maximum(out, _shift(a, dy, dx) * k)
    return out


def _finish(px):
    """The outline round the shape and the light edge on its lit sides; returns RGBA."""
    a = px[..., 3]
    rgb = px[..., :3]
    # A touch more contrast in the steel, as the genre's cursors have.
    rgb = np.clip((rgb - 0.5) * 1.1 + 0.5, 0.0, 1.0)
    # The light edge: inside the shape, where the shape ends toward the upper left.
    shifted = _shift(a, int(round(EDGE)), int(round(EDGE)))
    edge = np.clip(a - shifted, 0.0, 1.0)[..., None]
    rgb = rgb + (1.0 - rgb) * edge * 0.55
    out_a = _dilate(a, OUTLINE)
    ol = np.array(OUTLINE_COLOR, dtype=np.float32)
    col = rgb * a[..., None] + ol * (1.0 - a[..., None])
    out = np.zeros_like(px)
    out[..., :3] = col
    out[..., 3] = np.maximum(out_a, a)
    return out


def _save(px, path):
    h, w, _ = px.shape
    img = bpy.data.images.new("out", w, h, alpha=True)
    img.pixels = px[::-1].ravel()
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)


def _hidden(objs, shown):
    for o in objs:
        o.hide_render = o not in shown


def render_all(out):
    """Every state's master into `out`, and `hotspots.gd` (each one's click point)."""
    os.makedirs(out, exist_ok=True)
    arm = next(o for o in bpy.data.objects if o.type == "ARMATURE")
    body = next(o for o in bpy.data.objects if o.type == "MESH" and not o.name.startswith("part."))
    hand = gauntlet.Hand(arm, body)
    pieces = gauntlet.build(hand)
    for o in list(bpy.data.objects):
        bpy.data.objects.remove(o)
    cam = _scene()
    tip = _placement() @ gauntlet.tip(hand, gauntlet.POINTING)
    models = {s: [_gauntlet(hand, pieces, s)] for s in ("default", "enemy", "ally", "shop")}
    # The bag hangs beside the hand, low on the left, clear of the finger.
    pts = [models["default"][0].matrix_world @ v.co for v in models["default"][0].data.vertices]
    x0, x1 = min(p.x for p in pts), max(p.x for p in pts)
    y0, y1 = min(p.y for p in pts), max(p.y for p in pts)
    models["shop"] += _coin_bag(Vector((x0 + (x1 - x0) * 0.14, y0 + (y1 - y0) * 0.22, 0.06)))
    _frame_camera(cam, models["shop"])
    models["attack"] = _reticle()
    every = [o for objs in models.values() for o in objs]
    lines = []
    tmp = os.path.join(out, "_render.png")
    for state, objs in models.items():
        _hidden(every, objs)
        if state == "attack":
            # The reticle centred, on its own scale.
            saved = (cam.location.copy(), cam.data.ortho_scale)
            cam.location = (0, 0, 2)
            cam.data.ortho_scale = 0.2
            hot = (SIZE / 2, SIZE / 2)
        else:
            u = world_to_camera_view(bpy.context.scene, cam, tip)
            hot = (u.x * SIZE, (1.0 - u.y) * SIZE)
        px = _finish(_reduce(_render(tmp)))
        _save(px, os.path.join(out, f"{state}.png"))
        lines.append(f'\t"{state}": Vector2({hot[0]:.1f}, {hot[1]:.1f}),')
        if state == "attack":
            cam.location, cam.data.ortho_scale = saved
    os.remove(tmp)
    with open(os.path.join(out, "hotspots.gd"), "w", encoding="utf-8", newline="\n") as f:
        f.write("# Generated by art/ui/cursor/build.py: where each cursor master clicks (the fingertip;\n"
                "# the reticle's centre), in px from its top left.\nextends RefCounted\n\n"
                f"const SIZE := {SIZE}\nconst AT := {{\n" + "\n".join(lines) + "\n}\n")
    print("cursor:", " ".join(line.strip() for line in lines))
