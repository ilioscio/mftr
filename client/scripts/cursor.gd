extends RefCounted
## The game cursor (05 §6): a steel gauntlet pointing the way, as in the genre. It is rendered
## in Blender from the in-game hand (`art/ui/cursor/build.py`): `res://cursor/<state>.png`,
## 128 px masters scaled to the screen here. It changes with what's under it: red over an enemy
## you can attack, teal over an ally, a steel reticle while an attack-move waits for its click,
## a bag of coins beside it over your own shop (19 §4). Menus and panels get the plain gauntlet.
## Every gauntlet clicks at its fingertip, the pixel the render put it on (`hotspots.gd`).

const Hotspots := preload("res://cursor/hotspots.gd")
const STATES := ["default", "enemy", "ally", "attack", "shop"]

## The cursor's size at 1080 lines and 100 % HUD scale (px); it grows with the HUD's scale.
const SIZE_PX := 48

static var _cache := {}
static var _state := ""
static var _scale := 1.0


## Show the cursor for `state`: "default", "enemy", "ally", "attack" or "shop" (a no-op if
## unchanged).
static func set_state(state: String) -> void:
	if state == _state:
		return
	_state = state
	if not _cache.has(state):
		_cache[state] = _make(state)
	var made: Array = _cache[state]
	Input.set_custom_mouse_cursor(made[0], Input.CURSOR_ARROW, made[1])


## The HUD's scale (the window's content scale, 05 §6): the cursor grows with it.
static func set_scale(k: float) -> void:
	if absf(k - _scale) < 1e-3:
		return
	_scale = k
	_cache.clear()
	var now := _state
	_state = ""
	if now != "":
		set_state(now)


## [image, hotspot] for a state, at the current scale.
static func _make(state: String) -> Array:
	var tex: Texture2D = load("res://cursor/%s.png" % state)
	var img := tex.get_image()
	img.decompress()
	img.convert(Image.FORMAT_RGBA8)
	# Godot draws custom cursors up to 256 px.
	var px := clampi(roundi(SIZE_PX * _scale), 24, 256)
	img.resize(px, px, Image.INTERPOLATE_LANCZOS)
	var at: Vector2 = Hotspots.AT[state]
	return [img, (at * (px / float(Hotspots.SIZE))).floor()]
