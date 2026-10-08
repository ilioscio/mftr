extends RefCounted
## Player settings (Esc → Settings), kept in `user://settings.cfg`: the camera, the minimap and
## the keybinds. Keybinds live in Godot's InputMap; this keeps the player's overrides of the
## defaults below and writes them back into the map.

const PATH := "user://settings.cfg"

## Camera: unlocked by default, scrolled by bumping the mouse against the screen edges (the
## reference game's default); Y locks it on the champion, Space centers it while held.
var camera_locked := false
var edge_pan := true
var edge_pan_speed := 50.0               # 0–100
var confine_cursor := true               # keep the cursor in the window while playing
## Minimap (bottom right): shown, and its size as a share of the default.
var minimap_shown := true
var minimap_size := 100.0                # 50–160 %

## Rebindable actions, in the order the settings list them: [action, label, default].
## A default is a key name with modifiers (`Alt+Q`) or a mouse button (`Mouse Right`).
const BINDINGS := [
	["cast_q", "Ability Q", "Q"],
	["cast_w", "Ability W", "W"],
	["cast_e", "Ability E", "E"],
	["cast_r", "Ability R (ultimate)", "R"],
	["cast_d", "Spell D (Blink)", "D"],
	["cast_f", "Spell F (Barrier)", "F"],
	["level_q", "Level up Q", "Alt+Q"],
	["level_w", "Level up W", "Alt+W"],
	["level_e", "Level up E", "Alt+E"],
	["level_r", "Level up R", "Alt+R"],
	["move", "Move / attack", "Mouse Right"],
	["attack_move", "Attack-move (then click)", "A"],
	["stop", "Stop", "S"],
	["toggle_shop", "Open / close the shop", "G"],
	["camera_lock", "Lock / unlock the camera", "Y"],
	["camera_center", "Center the camera (hold)", "Space"],
	["pan_up", "Scroll the camera up", "Up"],
	["pan_down", "Scroll the camera down", "Down"],
	["pan_left", "Scroll the camera left", "Left"],
	["pan_right", "Scroll the camera right", "Right"],
	["toggle_minimap", "Show / hide the minimap", "None"],
	["toggle_net_graph", "Network stats", "F1"],
	["toggle_proxies", "Collision proxies (debug)", "F2"],
]

var _binds := {}                         # action -> binding text (only the player's changes)


func _init() -> void:
	load_file()


func load_file() -> void:
	var cfg := ConfigFile.new()
	if cfg.load(PATH) == OK:
		camera_locked = cfg.get_value("camera", "locked", camera_locked)
		edge_pan = cfg.get_value("camera", "edge_pan", edge_pan)
		edge_pan_speed = clampf(cfg.get_value("camera", "edge_pan_speed", edge_pan_speed), 0.0, 100.0)
		confine_cursor = cfg.get_value("camera", "confine_cursor", confine_cursor)
		minimap_shown = cfg.get_value("minimap", "shown", minimap_shown)
		minimap_size = clampf(cfg.get_value("minimap", "size", minimap_size), 50.0, 160.0)
		_binds = cfg.get_value("keys", "binds", {})
	apply_bindings()


func save() -> void:
	var cfg := ConfigFile.new()
	cfg.set_value("camera", "locked", camera_locked)
	cfg.set_value("camera", "edge_pan", edge_pan)
	cfg.set_value("camera", "edge_pan_speed", edge_pan_speed)
	cfg.set_value("camera", "confine_cursor", confine_cursor)
	cfg.set_value("minimap", "shown", minimap_shown)
	cfg.set_value("minimap", "size", minimap_size)
	cfg.set_value("keys", "binds", _binds)
	cfg.save(PATH)


## Edge-pan speed in game units per second (the slider's 0–100 spans 1,000–6,000 u/s).
func pan_speed_u() -> float:
	return lerpf(1000.0, 6000.0, edge_pan_speed / 100.0)


func binding(action: String) -> String:
	if _binds.has(action):
		return _binds[action]
	for b in BINDINGS:
		if b[0] == action:
			return b[2]
	return "None"


func set_binding(action: String, text: String) -> void:
	_binds[action] = text
	apply_bindings()
	save()


func reset_bindings() -> void:
	_binds = {}
	apply_bindings()
	save()


## Writes every binding into the InputMap (actions are created if the project lacks them).
func apply_bindings() -> void:
	for b in BINDINGS:
		var action: String = b[0]
		if not InputMap.has_action(action):
			InputMap.add_action(action)
		InputMap.action_erase_events(action)
		var ev := event_from_text(binding(action))
		if ev != null:
			InputMap.action_add_event(action, ev)


## `Alt+Q`, `Ctrl+Shift+F5`, `Space`, `Mouse Right`, `None` → an input event (null for None).
static func event_from_text(text: String) -> InputEvent:
	if text == "" or text == "None":
		return null
	if text.begins_with("Mouse "):
		var mb := InputEventMouseButton.new()
		match text.substr(6):
			"Left":
				mb.button_index = MOUSE_BUTTON_LEFT
			"Right":
				mb.button_index = MOUSE_BUTTON_RIGHT
			"Middle":
				mb.button_index = MOUSE_BUTTON_MIDDLE
			"Back":
				mb.button_index = MOUSE_BUTTON_XBUTTON1
			"Forward":
				mb.button_index = MOUSE_BUTTON_XBUTTON2
			_:
				return null
		return mb
	var k := InputEventKey.new()
	var parts := text.split("+")
	for i in parts.size() - 1:
		match parts[i]:
			"Alt":
				k.alt_pressed = true
			"Ctrl":
				k.ctrl_pressed = true
			"Shift":
				k.shift_pressed = true
	var code := OS.find_keycode_from_string(parts[parts.size() - 1])
	if code == KEY_NONE:
		return null
	k.physical_keycode = code
	return k


## The text for a captured event (modifiers first), or "" for one that can't be a binding.
static func text_from_event(ev: InputEvent) -> String:
	if ev is InputEventMouseButton:
		match (ev as InputEventMouseButton).button_index:
			MOUSE_BUTTON_LEFT:
				return "Mouse Left"
			MOUSE_BUTTON_RIGHT:
				return "Mouse Right"
			MOUSE_BUTTON_MIDDLE:
				return "Mouse Middle"
			MOUSE_BUTTON_XBUTTON1:
				return "Mouse Back"
			MOUSE_BUTTON_XBUTTON2:
				return "Mouse Forward"
		return ""
	if ev is InputEventKey:
		var k := ev as InputEventKey
		var code := k.physical_keycode if k.physical_keycode != KEY_NONE else k.keycode
		if code in [KEY_ALT, KEY_CTRL, KEY_SHIFT, KEY_META, KEY_NONE]:
			return ""
		var mods := ""
		if k.ctrl_pressed:
			mods += "Ctrl+"
		if k.alt_pressed:
			mods += "Alt+"
		if k.shift_pressed:
			mods += "Shift+"
		return mods + OS.get_keycode_string(code)
	return ""
