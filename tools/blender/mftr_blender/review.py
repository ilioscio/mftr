# SPDX-License-Identifier: AGPL-3.0-or-later
"""Review renders (10 §8.5): per clip, a contact sheet (PNG) and a looping GIF, from
(a) the gameplay camera of R01 (56° pitch, 45° vertical FOV, ~18.5 m away) at its true
1080p pixel size, shown 2x enlarged with nearest filtering, and (b) a 3/4 close-up.

Workbench render, vertex colors, flat shading: what the faceted style will look like, minus the
game's toon ramp. Nobody should have to open Blender to review timing.
"""

import math
import os
import tempfile

import bpy
import numpy as np
from bpy_extras.object_utils import world_to_camera_view
from mathutils import Euler, Vector

from . import clips, export, gif

GAMEPLAY_PITCH = 56.0
GAMEPLAY_VFOV = 45.0
GAMEPLAY_DISTANCE = 18.5
# The character faces the camera's lower right in the gameplay view (+45° turns it left).
GAMEPLAY_YAW = 45.0
CLOSE_SIZE = 256
MAX_SHEET_TILES = 12
BACKGROUND = (0.11, 0.13, 0.12)
GROUND = (0.22, 0.26, 0.21)


def _camera(name, location, look_at, vfov_deg=None, lens=None):
    data = bpy.data.cameras.new(name)
    if vfov_deg:
        data.sensor_fit = "VERTICAL"
        data.angle_y = math.radians(vfov_deg)
    if lens:
        data.lens = lens
    cam = bpy.data.objects.new(name, data)
    bpy.context.scene.collection.objects.link(cam)
    cam.location = location
    cam.rotation_euler = (Vector(look_at) - Vector(location)).to_track_quat("-Z", "Y").to_euler()
    # New objects have no world matrix until the depsgraph runs; projections need it.
    bpy.context.view_layer.update()
    return cam


def _ground():
    me = bpy.data.meshes.new("review_ground")
    s = 3.0
    me.from_pydata([(-s, -s, 0), (s, -s, 0), (s, s, 0), (-s, s, 0)], [], [(0, 1, 2, 3)])
    col = me.color_attributes.new("Col", "FLOAT_COLOR", "POINT")
    for d in col.data:
        d.color = (*GROUND, 1.0)
    obj = bpy.data.objects.new("review_ground", me)
    bpy.context.scene.collection.objects.link(obj)
    return obj


def _setup_scene():
    sc = bpy.context.scene
    sc.render.engine = "BLENDER_WORKBENCH"
    sh = sc.display.shading
    sh.light = "STUDIO"
    sh.color_type = "VERTEX"
    sh.show_shadows = True
    sh.show_cavity = False
    sh.show_specular_highlight = False
    sc.render.film_transparent = False
    if sc.world is None:
        sc.world = bpy.data.worlds.new("review_world")
    sc.world.color = BACKGROUND
    sc.render.image_settings.file_format = "PNG"
    sc.render.image_settings.color_mode = "RGB"
    sc.render.use_stamp = False
    sc.render.resolution_percentage = 100


def _border_for(cam, points):
    sc = bpy.context.scene
    ndc = [world_to_camera_view(sc, cam, Vector(p)) for p in points]
    xs, ys = [v.x for v in ndc], [v.y for v in ndc]
    return max(0.0, min(xs)), min(1.0, max(xs)), max(0.0, min(ys)), min(1.0, max(ys))


def _render(path):
    bpy.context.scene.render.filepath = path
    bpy.ops.render.render(write_still=True)
    img = bpy.data.images.load(path)
    w, h = img.size
    px = np.empty(w * h * 4, dtype=np.float32)
    img.pixels.foreach_get(px)
    bpy.data.images.remove(img)
    rgb = px.reshape(h, w, 4)[::-1, :, :3]
    # PNG pixels come back as stored (display-referred sRGB): no transform needed.
    return (np.clip(rgb, 0, 1) * 255 + 0.5).astype(np.uint8)


def _fit(img, size):
    """Nearest-neighbour scale into a size x size tile (letterboxed)."""
    h, w = img.shape[:2]
    s = size / max(h, w)
    ys = (np.arange(int(h * s)) / s).astype(int).clip(0, h - 1)
    xs = (np.arange(int(w * s)) / s).astype(int).clip(0, w - 1)
    scaled = img[ys][:, xs]
    tile = np.empty((size, size, 3), dtype=np.uint8)
    tile[:] = (np.array(BACKGROUND) * 255).astype(np.uint8)
    oy, ox = (size - scaled.shape[0]) // 2, (size - scaled.shape[1]) // 2
    tile[oy:oy + scaled.shape[0], ox:ox + scaled.shape[1]] = scaled
    return tile


def _save_png(arr, path):
    h, w = arr.shape[:2]
    img = bpy.data.images.new("review_out", w, h, alpha=False)
    rgba = np.concatenate([arr[::-1].astype(np.float32) / 255, np.ones((h, w, 1), np.float32)], axis=2)
    img.pixels.foreach_set(rgba.ravel())
    img.filepath_raw = path
    img.file_format = "PNG"
    img.save()
    bpy.data.images.remove(img)


# 3x5 pixel glyphs for frame labels (no font rendering inside Blender's Python).
_GLYPHS = {
    "0": "111101101101111", "1": "010110010010111", "2": "111001111100111", "3": "111001111001111",
    "4": "101101111001001", "5": "111100111001111", "6": "111100111101111", "7": "111001001001001",
    "8": "111101111101111", "9": "111101111001111", "f": "011100110100100", " ": "000000000000000",
}


def _text(img, x, y, text, color=(230, 230, 230), scale=2):
    for ch in text:
        g = _GLYPHS.get(ch, _GLYPHS[" "])
        for i, bit in enumerate(g):
            if bit == "1":
                r, c = divmod(i, 3)
                img[y + r * scale:y + (r + 1) * scale, x + c * scale:x + (c + 1) * scale] = color
        x += 4 * scale


def _marker_band(tile_w, frames, markers, frame):
    """A band under each sheet tile: the frame number, a grey timeline, a white playhead, red at
    `fire`, blue at loop markers, green at foot contacts."""
    band = np.full((18, tile_w, 3), 40, np.uint8)
    _text(band, 4, 1, f"f{frame}")
    tl = band[12:18]
    colors = {"fire": (230, 60, 60), "loop_in": (70, 140, 240), "loop_out": (70, 140, 240),
              "foot_l": (80, 200, 90), "foot_r": (80, 200, 90), "end": (200, 200, 200)}
    for name, f in markers:
        x = int(round(f / max(frames, 1) * (tile_w - 1)))
        tl[:, max(0, x - 1):x + 2] = colors.get(name, (200, 160, 60))
    x = int(round(frame / max(frames, 1) * (tile_w - 1)))
    tl[1:5, max(0, x - 1):x + 2] = (255, 255, 255)
    return band


def render_clips(out_dir, names=None, tile=CLOSE_SIZE):
    """Render every stashed clip (or `names`) of the scene's MFTR armature into `out_dir`."""
    arm = export.find_armature()
    acts = [a for a in export.stashed_actions(arm) if names is None or a.name in names]
    sc = bpy.context.scene
    _setup_scene()
    ground = _ground()
    target = Vector((0, 0, 0.95))
    pitch = math.radians(GAMEPLAY_PITCH)
    game_cam = _camera("review_game", target + Vector((0, -math.cos(pitch), math.sin(pitch))) * GAMEPLAY_DISTANCE,
                       target, vfov_deg=GAMEPLAY_VFOV)
    az, el, dist = math.radians(35), math.radians(15), 4.4
    close_cam = _camera("review_close", target + Vector((math.cos(el) * math.sin(az), -math.cos(el) * math.cos(az), math.sin(el))) * dist,
                        target + Vector((0, 0, -0.08)), lens=60)
    old_rot = arm.rotation_euler.copy()
    os.makedirs(out_dir, exist_ok=True)
    tmp = tempfile.mkdtemp(prefix="mftr_review_")
    written = []
    try:
        for act in acts:
            arm.animation_data.action = act
            start, end = (int(round(v)) for v in act.frame_range)
            frames = end - start
            markers = sorted(((m.name, int(m.frame - start)) for m in act.pose_markers), key=lambda m: m[1])
            loop = bool(act.get("mftr_loop"))
            # A loop's last frame repeats its first: play [start, end).
            play = list(range(start, end if loop else end + 1))
            game, close = {}, {}
            # Gameplay view: true 1080p pixel size, cropped around the character.
            sc.camera = game_cam
            sc.render.resolution_x, sc.render.resolution_y = 1920, 1080
            arm.rotation_euler = Euler((0, 0, math.radians(GAMEPLAY_YAW)))
            box = [(x, y, z) for x in (-1.1, 1.1) for y in (-1.1, 1.1) for z in (0, 2.3)]
            sc.render.border_min_x, sc.render.border_max_x, sc.render.border_min_y, sc.render.border_max_y = _border_for(game_cam, box)
            sc.render.use_border = sc.render.use_crop_to_border = True
            for f in play:
                sc.frame_set(f)
                game[f] = _render(os.path.join(tmp, "g.png"))
            # Close-up 3/4 view.
            sc.camera = close_cam
            sc.render.resolution_x = sc.render.resolution_y = tile
            sc.render.use_border = sc.render.use_crop_to_border = False
            arm.rotation_euler = old_rot
            for f in play:
                sc.frame_set(f)
                close[f] = _render(os.path.join(tmp, "c.png"))

            def pair(f):
                return np.concatenate([_fit(game[f], tile), close[f]], axis=1)

            gif.write(os.path.join(out_dir, f"{act.name}.gif"), [pair(f) for f in play], fps=clips.FPS)
            # Contact sheet: marker frames first, then evenly spaced frames.
            pick = sorted({start + f for _, f in markers if start + f in game} |
                          {play[int(i)] for i in np.linspace(0, len(play) - 1, min(MAX_SHEET_TILES, len(play)))})
            pick = pick[:MAX_SHEET_TILES + len(markers)]
            cols = min(6, len(pick))
            rows = []
            tiles = [np.concatenate([pair(f), _marker_band(tile * 2, frames, markers, f - start)], axis=0) for f in pick]
            while len(tiles) % cols:
                tiles.append(np.zeros_like(tiles[0]))
            for r in range(0, len(tiles), cols):
                rows.append(np.concatenate(tiles[r:r + cols], axis=1))
            _save_png(np.concatenate(rows, axis=0), os.path.join(out_dir, f"{act.name}.png"))
            written.append(act.name)
    finally:
        arm.rotation_euler = old_rot
        arm.animation_data.action = None
        clips.reset_pose(arm)
        sc.render.use_border = sc.render.use_crop_to_border = False
        for cam in (game_cam, close_cam):
            data = cam.data
            bpy.data.objects.remove(cam)
            bpy.data.cameras.remove(data)
        me = ground.data
        bpy.data.objects.remove(ground)
        bpy.data.meshes.remove(me)
    return written
