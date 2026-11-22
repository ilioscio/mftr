extends RefCounted
## Player settings (Esc → Settings), kept in `user://settings.cfg`: the camera, the minimap and
## the keybinds. Keybinds live in Godot's InputMap; this keeps the player's overrides of the
## defaults below and writes them back into the map.

const PATH := "user://settings.cfg"
const HUD_SCALE_MIN := 75.0
const HUD_SCALE_MAX := 150.0
const WINDOW_MODES := ["borderless", "fullscreen", "windowed"]
const ANTIALIAS := ["msaa4", "msaa2", "fxaa", "off"]
const RENDER_SCALE_MIN := 50.0
const RENDER_SCALE_MAX := 150.0
const SHADOW_QUALITIES := ["ultra", "high", "medium", "low", "off"]
const WORLD_DETAILS := ["high", "medium", "low"]
const FPS_LIMITS := [0, 30, 60, 90, 120, 144, 165, 240]   # 0: unlimited

## Graphics presets (05 §6, D81): each sets every setting that costs frame time. Picking one
## writes these values; changing any of them afterwards makes the preset "custom". High is the
## look as designed and what a discrete graphics card gets; Medium and Low are for integrated
## graphics (05 §9's 60 fps target), Low giving up the most. Ultra adds sharper, softer-edged
## shadows. Measured costs are in 05 §9.
const PRESETS := {
	"low": {"render_scale": 75.0, "antialias": "fxaa", "shadow_quality": "off", "world_detail": "low", "bloom": false, "atmosphere": false},
	"medium": {"render_scale": 90.0, "antialias": "fxaa", "shadow_quality": "low", "world_detail": "medium", "bloom": true, "atmosphere": true},
	"high": {"render_scale": 100.0, "antialias": "msaa4", "shadow_quality": "high", "world_detail": "high", "bloom": true, "atmosphere": true},
	"ultra": {"render_scale": 100.0, "antialias": "msaa4", "shadow_quality": "ultra", "world_detail": "high", "bloom": true, "atmosphere": true},
}
const PRESET_NAMES := ["ultra", "high", "medium", "low", "custom"]

## Camera: unlocked by default, scrolled by bumping the mouse against the screen edges (the
## reference game's default); Y locks it on the champion, Space centers it while held.
var camera_locked := false
var edge_pan := true
var edge_pan_speed := 50.0               # 0–100
var confine_cursor := true               # keep the cursor in the window while playing
## Minimap (bottom right): shown, and its size as a share of the default.
var minimap_shown := true
var minimap_size := 100.0                # 50–160 %
## The HUD's size (75–150 %): everything drawn over the view (the HUD, panels, menus, tooltips),
## on top of the window's own size, so 100 % looks the same at 1080p and 4K (`main.gd`
## `_apply_hud_scale`). The world isn't scaled: it always renders at the window's full resolution.
var hud_scale := 100.0
## Display (Settings → Graphics). The window: "borderless" (fullscreen without a mode switch),
## "fullscreen" (exclusive) or "windowed".
var window_mode := "borderless"
## Smoothing the edges of models: "msaa4", "msaa2", "fxaa" (cheap, softer) or "off".
var antialias := "msaa4"
## The world's render resolution as a share of the window's (50–150 %): under 100 is faster and
## upscaled, over 100 renders more pixels and scales them down (sharper, slower). The HUD
## is always drawn at the window's own resolution.
var render_scale := 100.0
## The preset the values below came from, or "custom". "" until the player (or the first-run
## guess, `detect_preset`) has picked one.
var graphics_preset := ""
## The sun's shadows: "ultra", "high" (the design's), "medium", "low" or "off" (`main.gd`
## `_apply_shadows`: the shadow map's size and its edges' softness).
var shadow_quality := "high"
## How much fine detail the ground and cliffs draw (the `world_detail` shader global, 0–2, in
## `world.gdshaderinc`): "high" is all of it; "medium" drops the noise's finest octave; "low"
## keeps the shapes and colors and drops the grain, blades of grass, cracks, pebbles and
## crevices (and draws the nearest 12 contact shadows).
var world_detail := "high"
var fps_limit := 0                       # frames per second at most, 0 unlimited
var vsync := false                       # wait for the screen's refresh (no tearing, more latency)
var bloom := true                        # glow around bright things
var atmosphere := true                   # cloud shadows and floating motes
var hurt_edges := true                   # the screen's edges flash red when hurt, and pulse when low
## The season the map is dressed in (15 §9): "calendar" (the default: the season it is where you
## are, from the date and `hemisphere`), "map" for the map's own, else "summer", "autumn" or
## "winter". Looks only; it changes at once, mid-match too.
var season := "calendar"
## The sky over the map (D82, an evaluation): "clear", "rain", "night" or "storm" (weather.gd).
## Looks only, on this screen; a match doesn't choose it yet.
var weather := "clear"
## "north" or "south": which half of the world's seasons "calendar" follows. The default comes
## from the system's locale (`Seasons.default_hemisphere`).
var hemisphere := ""
var master_volume := 80.0                # 0–100 %
var effects_volume := 100.0              # abilities, attacks, impacts: under Master
var interface_volume := 100.0            # the fanfare and other interface sounds: under Master
var mute_in_background := true           # silence while the window isn't focused
var window_positions := {}               # window name -> [x, y] as fractions of the screen

## Rebindable actions, in the order the settings list them: [action, label, default].
## A default is a key or mouse button with modifiers (`Alt+Q`, `Shift+Mouse Right`); an action
## may have several, separated by " / ". Level-ups take Ctrl too: Alt+R is also the NVIDIA and
## AMD overlays' hotkey, which some players' drivers swallow before the game sees it.
const BINDINGS := [
	["cast_q", "Ability Q", "Q"],
	["cast_w", "Ability W", "W"],
	["cast_e", "Ability E", "E"],
	["cast_r", "Ability R (ultimate)", "R"],
	["cast_d", "Spell D (Blink)", "D"],
	["cast_f", "Spell F (Barrier)", "F"],
	["level_q", "Level up Q", "Alt+Q / Ctrl+Q"],
	["level_w", "Level up W", "Alt+W / Ctrl+W"],
	["level_e", "Level up E", "Alt+E / Ctrl+E"],
	["level_r", "Level up R", "Alt+R / Ctrl+R"],
	["move", "Move / attack", "Mouse Right"],
	["attack_move", "Attack-move (then click)", "A"],
	["attack_move_click", "Attack-move at the cursor", "Shift+Mouse Right"],
	["stop", "Stop", "S"],
	["recall", "Recall (8 s home)", "B"],
	["trinket", "Trinket: ward at the cursor / sweep", "T"],
	["toggle_shop", "Open / close the shop", "G"],
	["camera_lock", "Lock / unlock the camera", "Y"],
	["camera_center", "Center the camera (hold)", "Space"],
	["pan_up", "Scroll the camera up", "Up"],
	["pan_down", "Scroll the camera down", "Down"],
	["pan_left", "Scroll the camera left", "Left"],
	["pan_right", "Scroll the camera right", "Right"],
	["toggle_minimap", "Show / hide the minimap", "None"],
	["scoreboard", "Match breakdown (hold)", "Tab"],
	["item_1", "Use item 1", "1"],
	["item_2", "Use item 2", "2"],
	["item_3", "Use item 3", "3"],
	["item_4", "Use item 4", "4"],
	["item_5", "Use item 5", "5"],
	["item_6", "Use item 6", "6"],
	["toggle_net_graph", "Network stats", "F1"],
	["toggle_proxies", "Collision proxies (debug)", "F2"],
]

var _binds := {}                         # action -> binding text (only the player's changes)


func _init() -> void:
	load_file()


func load_file(path := PATH) -> void:
	var cfg := ConfigFile.new()
	if cfg.load(path) == OK:
		camera_locked = cfg.get_value("camera", "locked", camera_locked)
		edge_pan = cfg.get_value("camera", "edge_pan", edge_pan)
		edge_pan_speed = clampf(cfg.get_value("camera", "edge_pan_speed", edge_pan_speed), 0.0, 100.0)
		confine_cursor = cfg.get_value("camera", "confine_cursor", confine_cursor)
		minimap_shown = cfg.get_value("minimap", "shown", minimap_shown)
		minimap_size = clampf(cfg.get_value("minimap", "size", minimap_size), 50.0, 160.0)
		hud_scale = clampf(cfg.get_value("interface", "hud_scale", hud_scale), HUD_SCALE_MIN, HUD_SCALE_MAX)
		window_mode = one_of(cfg.get_value("graphics", "window_mode", window_mode), WINDOW_MODES, window_mode)
		antialias = one_of(cfg.get_value("graphics", "antialias", antialias), ANTIALIAS, antialias)
		render_scale = clampf(cfg.get_value("graphics", "render_scale", render_scale), RENDER_SCALE_MIN, RENDER_SCALE_MAX)
		# Before the presets (0.22.1), shadows were on or off.
		var legacy := "high" if cfg.get_value("graphics", "shadows", true) else "off"
		shadow_quality = one_of(cfg.get_value("graphics", "shadow_quality", legacy), SHADOW_QUALITIES, shadow_quality)
		world_detail = one_of(cfg.get_value("graphics", "world_detail", world_detail), WORLD_DETAILS, world_detail)
		var fps = cfg.get_value("graphics", "fps_limit", fps_limit)
		fps_limit = int(fps) if int(fps) in FPS_LIMITS else 0
		vsync = cfg.get_value("graphics", "vsync", vsync)
		graphics_preset = str(cfg.get_value("graphics", "preset", ""))
		bloom = cfg.get_value("graphics", "bloom", bloom)
		atmosphere = cfg.get_value("graphics", "atmosphere", atmosphere)
		hurt_edges = cfg.get_value("graphics", "hurt_edges", hurt_edges)
		season = str(cfg.get_value("graphics", "season", season))
		weather = one_of(cfg.get_value("graphics", "weather", weather), ["clear", "rain", "night", "storm"], "clear")
		# Before the calendar (0.18), "" was the map's own and the default: now the calendar is.
		if season == "":
			season = "calendar"
		hemisphere = str(cfg.get_value("graphics", "hemisphere", hemisphere))
		master_volume = clampf(cfg.get_value("audio", "master", master_volume), 0.0, 100.0)
		effects_volume = clampf(cfg.get_value("audio", "effects", effects_volume), 0.0, 100.0)
		interface_volume = clampf(cfg.get_value("audio", "interface", interface_volume), 0.0, 100.0)
		mute_in_background = cfg.get_value("audio", "mute_in_background", mute_in_background)
		window_positions = cfg.get_value("windows", "positions", {})
		_binds = cfg.get_value("keys", "binds", {})
	if graphics_preset not in PRESET_NAMES:
		# Never picked (a first run, or settings from before the presets): untouched graphics
		# (still High's, the old defaults) get the guess for this machine, anything else is
		# the player's own.
		graphics_preset = matching_preset()
		if graphics_preset == "high":
			apply_preset(detect_preset())
	apply_bindings()


## Sets every value the preset covers (not for "custom", which keeps them).
func apply_preset(preset: String) -> void:
	if not PRESETS.has(preset):
		graphics_preset = "custom"
		return
	for key in PRESETS[preset]:
		set(key, PRESETS[preset][key])
	graphics_preset = preset


## The preset the current values are, or "custom" (after the player changes one of them).
func matching_preset() -> String:
	for preset in PRESETS:
		var same := true
		for key in PRESETS[preset]:
			if get(key) != PRESETS[preset][key]:
				same = false
				break
		if same:
			return preset
	return "custom"


## A first guess for this machine (05 §9): integrated graphics get Medium, which keeps the
## shadows (a Radeon 840M runs it at about 57 fps on a 2256×1504 screen); no graphics card at all
## (a software renderer) gets Low; everything else, High. Low, which gives up the shadows, is
## the player's own choice. The player can always choose another.
static func detect_preset() -> String:
	var kind := RenderingServer.get_video_adapter_type()
	if kind == RenderingDevice.DEVICE_TYPE_CPU:
		return "low"
	return "medium" if kind == RenderingDevice.DEVICE_TYPE_INTEGRATED_GPU else "high"


## `v` if it's one of `allowed`, else `fallback` (a saved choice from another version).
static func one_of(v, allowed: Array, fallback: String) -> String:
	return str(v) if str(v) in allowed else fallback


func save() -> void:
	var cfg := ConfigFile.new()
	cfg.set_value("camera", "locked", camera_locked)
	cfg.set_value("camera", "edge_pan", edge_pan)
	cfg.set_value("camera", "edge_pan_speed", edge_pan_speed)
	cfg.set_value("camera", "confine_cursor", confine_cursor)
	cfg.set_value("minimap", "shown", minimap_shown)
	cfg.set_value("minimap", "size", minimap_size)
	cfg.set_value("interface", "hud_scale", hud_scale)
	cfg.set_value("graphics", "window_mode", window_mode)
	cfg.set_value("graphics", "antialias", antialias)
	cfg.set_value("graphics", "render_scale", render_scale)
	cfg.set_value("graphics", "preset", graphics_preset)
	cfg.set_value("graphics", "shadow_quality", shadow_quality)
	cfg.set_value("graphics", "world_detail", world_detail)
	cfg.set_value("graphics", "fps_limit", fps_limit)
	cfg.set_value("graphics", "vsync", vsync)
	cfg.set_value("graphics", "bloom", bloom)
	cfg.set_value("graphics", "atmosphere", atmosphere)
	cfg.set_value("graphics", "hurt_edges", hurt_edges)
	cfg.set_value("graphics", "season", season)
	cfg.set_value("graphics", "weather", weather)
	cfg.set_value("graphics", "hemisphere", hemisphere)
	cfg.set_value("audio", "master", master_volume)
	cfg.set_value("audio", "effects", effects_volume)
	cfg.set_value("audio", "interface", interface_volume)
	cfg.set_value("audio", "mute_in_background", mute_in_background)
	cfg.set_value("windows", "positions", window_positions)
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


## The first of an action's bindings (for short hints).
func primary_binding(action: String) -> String:
	return binding(action).split(" / ")[0]


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
		for text in binding(action).split(" / "):
			var ev := event_from_text(text)
			if ev != null:
				InputMap.action_add_event(action, ev)


## `Alt+Q`, `Ctrl+Shift+F5`, `Space`, `Shift+Mouse Right`, `None` → an input event (null for None).
static func event_from_text(text: String) -> InputEvent:
	if text == "" or text == "None":
		return null
	var parts := text.split("+")
	var last := parts[parts.size() - 1]
	var ev: InputEventWithModifiers
	if last.begins_with("Mouse "):
		var mb := InputEventMouseButton.new()
		match last.substr(6):
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
		ev = mb
	else:
		var code := OS.find_keycode_from_string(last)
		if code == KEY_NONE:
			return null
		var k := InputEventKey.new()
		k.physical_keycode = code
		ev = k
	for i in parts.size() - 1:
		match parts[i]:
			"Alt":
				ev.alt_pressed = true
			"Ctrl":
				ev.ctrl_pressed = true
			"Shift":
				ev.shift_pressed = true
	return ev


## The text for a captured event (modifiers first), or "" for one that can't be a binding.
static func text_from_event(ev: InputEvent) -> String:
	var name := ""
	if ev is InputEventMouseButton:
		match (ev as InputEventMouseButton).button_index:
			MOUSE_BUTTON_LEFT:
				name = "Mouse Left"
			MOUSE_BUTTON_RIGHT:
				name = "Mouse Right"
			MOUSE_BUTTON_MIDDLE:
				name = "Mouse Middle"
			MOUSE_BUTTON_XBUTTON1:
				name = "Mouse Back"
			MOUSE_BUTTON_XBUTTON2:
				name = "Mouse Forward"
			_:
				return ""
	elif ev is InputEventKey:
		var k := ev as InputEventKey
		var code := k.physical_keycode if k.physical_keycode != KEY_NONE else k.keycode
		if code in [KEY_ALT, KEY_CTRL, KEY_SHIFT, KEY_META, KEY_NONE]:
			return ""
		name = OS.get_keycode_string(code)
	else:
		return ""
	var m := ev as InputEventWithModifiers
	var mods := ""
	if m.ctrl_pressed:
		mods += "Ctrl+"
	if m.alt_pressed:
		mods += "Alt+"
	if m.shift_pressed:
		mods += "Shift+"
	return mods + name
