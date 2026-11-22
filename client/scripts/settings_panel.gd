extends PanelContainer
## Esc → Settings: the camera, the minimap and the keybinds. Every change applies and saves at
## once. Rebinding: click a binding, then press the key (with Alt / Ctrl / Shift) or the mouse
## button for it; Esc cancels, Backspace clears it.

const Seasons := preload("res://scripts/seasons.gd")

var settings
var _waiting := ""                       # the action being rebound, or ""
var _bind_buttons := {}                  # action -> Button
var tabs: TabContainer


func setup(s) -> void:
	settings = s
	custom_minimum_size = Vector2(640, 560)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 18)
	add_child(margin)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 10)
	margin.add_child(v)
	var title := Label.new()
	title.text = "Settings"
	title.theme_type_variation = "TitleLabel"
	v.add_child(title)
	tabs = TabContainer.new()
	tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	v.add_child(tabs)
	tabs.add_child(_camera_tab())
	tabs.add_child(_interface_tab())
	tabs.add_child(_graphics_tab())
	tabs.add_child(_audio_tab())
	tabs.add_child(_keys_tab())
	var done := Button.new()
	done.text = "Done"
	done.theme_type_variation = "PrimaryButton"
	done.pressed.connect(close)
	v.add_child(done)


func open() -> void:
	visible = true
	_refresh_bindings()
	await get_tree().process_frame
	var screen := get_parent_control_size()
	preload("res://scripts/windows.gd").place(self, "settings", settings, ((screen - size) / 2.0).max(Vector2.ZERO), screen)


func close() -> void:
	_waiting = ""
	visible = false


func get_parent_control_size() -> Vector2:
	var p := get_parent()
	return p.get_viewport().get_visible_rect().size if p != null else Vector2(1920, 1080)


func _row(parent: Control, label: String, control: Control, hint := "") -> void:
	var l := Label.new()
	l.text = label
	l.custom_minimum_size = Vector2(220, 0)
	l.tooltip_text = hint
	l.mouse_filter = Control.MOUSE_FILTER_PASS
	parent.add_child(l)
	# A checkbox stays its own size; everything else fills the row.
	control.size_flags_horizontal = Control.SIZE_SHRINK_BEGIN if control is CheckBox else Control.SIZE_EXPAND_FILL
	control.tooltip_text = hint
	parent.add_child(control)


func _grid(name_: String) -> GridContainer:
	var g := GridContainer.new()
	g.name = name_
	g.columns = 2
	g.add_theme_constant_override("h_separation", 16)
	g.add_theme_constant_override("v_separation", 10)
	return g


## A drop-down of `labels` for the values `keys`, showing `current`; `apply` gets the key.
func _choice(keys: Array, labels: Array, current: String, apply: Callable) -> OptionButton:
	var o := OptionButton.new()
	o.focus_mode = Control.FOCUS_NONE
	for l in labels:
		o.add_item(l)
	o.select(maxi(keys.find(current), 0))
	o.item_selected.connect(func(i: int):
		apply.call(keys[i])
		settings.save())
	return o


func _check(on: bool, apply: Callable) -> CheckBox:
	var c := CheckBox.new()
	c.button_pressed = on
	c.focus_mode = Control.FOCUS_NONE
	c.toggled.connect(func(v): apply.call(v); settings.save())
	return c


## A slider row; `live` false applies only when the player lets go (or clicks or uses keys),
## for a setting that would move the slider out from under the cursor.
func _slider(value: float, lo: float, hi: float, apply: Callable, unit := "", live := true) -> HBoxContainer:
	var h := HBoxContainer.new()
	var s := HSlider.new()
	s.min_value = lo
	s.max_value = hi
	s.step = 1.0
	s.value = value
	s.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	s.custom_minimum_size = Vector2(220, 0)
	var l := Label.new()
	l.custom_minimum_size = Vector2(56, 0)
	l.text = "%d%s" % [roundi(value), unit]
	var dragging := [false]
	s.drag_started.connect(func(): dragging[0] = true)
	s.value_changed.connect(func(v):
		l.text = "%d%s" % [roundi(v), unit]
		if live or not dragging[0]:
			apply.call(v)
			if not live:
				settings.save())
	s.drag_ended.connect(func(_c):
		dragging[0] = false
		apply.call(s.value)
		settings.save())
	h.add_child(s)
	h.add_child(l)
	return h


func _camera_tab() -> Control:
	var g := _grid("Camera")
	_row(g, "Camera locked on champion", _check(settings.camera_locked, func(v): settings.camera_locked = v),
		"Off: the camera scrolls freely (screen edges, arrow keys, middle-mouse drag). Toggle in game with the camera-lock key.")
	_row(g, "Scroll at screen edges", _check(settings.edge_pan, func(v): settings.edge_pan = v),
		"Bump the cursor against the screen edge to scroll the camera.")
	_row(g, "Scroll speed", _slider(settings.edge_pan_speed, 0, 100, func(v): settings.edge_pan_speed = v),
		"How fast edge and arrow-key scrolling moves the camera.")
	_row(g, "Keep cursor in the window", _check(settings.confine_cursor, func(v): settings.confine_cursor = v),
		"While playing, the cursor can't leave the game window (so edge scrolling works in windowed mode).")
	return g


func _interface_tab() -> Control:
	var g := _grid("Interface")
	var reset := Button.new()
	reset.text = "Put every window back"
	reset.pressed.connect(func(): settings.window_positions = {}; settings.save())
	_row(g, "Moved windows", reset, "The shop, the augment draft, the match breakdown and the others go back to where they start.")
	_row(g, "HUD scale", _slider(settings.hud_scale, settings.HUD_SCALE_MIN, settings.HUD_SCALE_MAX, func(v): settings.hud_scale = v, "%", false),
		"The size of the HUD, the windows and the menus. 100% is the same share of the screen at any resolution. Applies when you let go of the slider.")
	_row(g, "Show the minimap", _check(settings.minimap_shown, func(v): settings.minimap_shown = v))
	_row(g, "Minimap size", _slider(settings.minimap_size, 50, 160, func(v): settings.minimap_size = v, "%"))
	return g


func _audio_tab() -> Control:
	var g := _grid("Audio")
	_row(g, "Master volume", _slider(settings.master_volume, 0, 100, func(v): settings.master_volume = v, "%"))
	_row(g, "Effects", _slider(settings.effects_volume, 0, 100, func(v): settings.effects_volume = v, "%"),
		"Abilities, attacks, impacts and footsteps.")
	_row(g, "Interface", _slider(settings.interface_volume, 0, 100, func(v): settings.interface_volume = v, "%"),
		"The victory and defeat fanfare, and other interface sounds.")
	_row(g, "Mute in the background", _check(settings.mute_in_background, func(v): settings.mute_in_background = v),
		"Silence the game while its window isn't focused.")
	return g


## The Graphics tab's controls for what a preset sets (setting -> control), so picking a preset
## can show its values.
var _preset_controls := {}
var _preset_choice: OptionButton


func _graphics_tab() -> Control:
	var scroll := ScrollContainer.new()
	scroll.name = "Graphics"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	var g := _grid("List")
	g.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(g)
	_preset_choice = _choice(settings.PRESET_NAMES, ["Ultra", "High", "Medium", "Low", "Custom"], settings.graphics_preset, func(k):
		settings.apply_preset(k)
		_show_preset())
	_row(g, "Quality preset", _preset_choice,
		"Sets everything below that costs frame rate at once. High is the game as designed; Ultra sharpens the shadows. Medium and Low are for laptops and integrated graphics. Change any of them and the preset becomes Custom.")
	_row(g, "Window", _choice(settings.WINDOW_MODES, ["Borderless fullscreen", "Fullscreen", "Windowed"], settings.window_mode, func(k): settings.window_mode = k),
		"Borderless fills the screen without switching its mode, so alt-tab is instant. Fullscreen takes the screen over (can be a little faster). Windowed is a window you can size and move.")
	_preset_row(g, "render_scale", "Render scale", _slider(settings.render_scale, settings.RENDER_SCALE_MIN, settings.RENDER_SCALE_MAX, _preset_set.bind("render_scale"), "%"),
		"The world's resolution, as a share of the window's: the biggest lever on the frame rate. Under 100% is faster but softer; over 100% is sharper and slower. The HUD always stays at full resolution.")
	_preset_row(g, "world_detail", "World detail", _choice(settings.WORLD_DETAILS, ["High", "Medium", "Low"], settings.world_detail, _preset_set.bind("world_detail")),
		"How much fine texture the ground and cliffs draw: grain, blades of grass, cracks and pebbles. Medium looks nearly the same and is faster; Low keeps the shapes and colors and is fastest.")
	_preset_row(g, "antialias", "Anti-aliasing", _choice(settings.ANTIALIAS, ["MSAA 4×", "MSAA 2×", "FXAA", "Off"], settings.antialias, _preset_set.bind("antialias")),
		"Smooths the stair-stepped edges of champions, monsters, props and cliffs. MSAA 4× is the smoothest; 2× is cheaper; FXAA is cheapest but softens the whole picture a little.")
	_preset_row(g, "shadow_quality", "Shadows", _choice(settings.SHADOW_QUALITIES, ["Ultra", "High", "Medium", "Low", "Off"], settings.shadow_quality, _preset_set.bind("shadow_quality")),
		"The sun's shadows. Ultra is the sharpest with the softest edges; Medium and Low are blurrier and cheaper; Off is fastest.")
	_preset_row(g, "bloom", "Bloom", _check(settings.bloom, _preset_set.bind("bloom")), "A soft glow around bright things: crystals, fire, spells.")
	_preset_row(g, "atmosphere", "Clouds and motes", _check(settings.atmosphere, _preset_set.bind("atmosphere")), "Cloud shadows drifting over the map, and motes floating in the light.")
	var fps_labels: Array = settings.FPS_LIMITS.map(func(f): return "Unlimited" if f == 0 else "%d fps" % f)
	var fps_keys: Array = settings.FPS_LIMITS.map(func(f): return str(f))
	_row(g, "Frame rate limit", _choice(fps_keys, fps_labels, str(settings.fps_limit), func(k): settings.fps_limit = int(k)),
		"The most frames a second the game draws. A limit near what you get anyway keeps it steady, and keeps a laptop cooler and its battery longer.")
	_row(g, "VSync", _check(settings.vsync, func(v): settings.vsync = v),
		"Waits for the screen to refresh before showing a frame: no tearing, but a little more input delay, and the frame rate snaps down when it can't keep up.")
	_row(g, "Damage at the edges", _check(settings.hurt_edges, func(v): settings.hurt_edges = v), "The screen's edges flash red when you're hurt, redder the lower your health, and throb when you're nearly dead.")
	var seasons := OptionButton.new()
	seasons.focus_mode = Control.FOCUS_NONE
	var keys := ["calendar", "map", "summer", "autumn", "winter"]
	var names := {"calendar": "By the calendar", "map": "The map's own"}
	for k in keys:
		seasons.add_item(names.get(k, str(k).capitalize()))
	seasons.select(maxi(keys.find(settings.season), 0))
	seasons.item_selected.connect(func(i: int):
		settings.season = keys[i]
		settings.save())
	_row(g, "Season", seasons, "How the map is dressed: its ground, cliffs, light, flora and fauna, and the shops. By the calendar it's the season it is where you are; or the map maker's pick, or one you choose. Looks only, and it changes at once.")
	_row(g, "Weather", _choice(["clear", "rain", "night", "storm"], ["Clear", "Rain", "Night", "Night storm"], settings.weather, func(k):
		settings.weather = k
		settings.save()),
		"The sky over the map (being tried out): an overcast day with rain, a moonlit night lit by the braziers, or a stormy night with lightning and thunder. Looks only, and it changes at once.")
	var halves := OptionButton.new()
	halves.focus_mode = Control.FOCUS_NONE
	var sides := ["north", "south"]
	halves.add_item("Northern")
	halves.add_item("Southern")
	var half: String = settings.hemisphere if settings.hemisphere in sides else Seasons.default_hemisphere()
	halves.select(sides.find(half))
	halves.item_selected.connect(func(i: int):
		settings.hemisphere = sides[i]
		settings.save())
	_row(g, "Hemisphere", halves, "Which half of the world you're in, for the season by the calendar: winter is December to February in the north, June to August in the south.")
	return scroll


## A row for a setting a preset sets: its control is remembered for `_show_preset`.
func _preset_row(parent: Control, key: String, label: String, control: Control, hint: String) -> void:
	_preset_controls[key] = control
	_row(parent, label, control, hint)


## The player changed one of a preset's settings: it's theirs now, unless it matches a preset.
func _preset_set(value, key: String) -> void:
	settings.set(key, value)
	settings.graphics_preset = settings.matching_preset()
	_preset_choice.select(settings.PRESET_NAMES.find(settings.graphics_preset))


## After a preset is picked: its values in the controls (without applying them again).
func _show_preset() -> void:
	for key in _preset_controls:
		var c: Control = _preset_controls[key]
		var value = settings.get(key)
		if c is CheckBox:
			c.set_pressed_no_signal(value)
		elif c is OptionButton:
			var keys: Array = {"world_detail": settings.WORLD_DETAILS, "antialias": settings.ANTIALIAS,
				"shadow_quality": settings.SHADOW_QUALITIES}[key]
			c.select(keys.find(value))
		else:
			var slider: HSlider = c.get_child(0)
			slider.set_value_no_signal(value)
			c.get_child(1).text = "%d%%" % roundi(value)


func _keys_tab() -> Control:
	var scroll := ScrollContainer.new()
	scroll.name = "Keys"
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	var v := VBoxContainer.new()
	v.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(v)
	var g := _grid("List")
	v.add_child(g)
	for b in settings.BINDINGS:
		var action: String = b[0]
		var btn := Button.new()
		btn.focus_mode = Control.FOCUS_NONE
		btn.pressed.connect(func(): _start_rebind(action))
		_bind_buttons[action] = btn
		_row(g, b[1], btn)
	var hint := Label.new()
	hint.text = "Click a binding, then press the new key (Alt, Ctrl and Shift combine) or mouse button. Esc cancels, Backspace clears. If Alt+R doesn't level your ultimate, a graphics overlay (NVIDIA, AMD) has probably taken it: use Ctrl+R, click the ability's + on the bar, or rebind it."
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hint.theme_type_variation = "HintLabel"
	v.add_child(hint)
	var reset := Button.new()
	reset.text = "Reset keys to defaults"
	reset.pressed.connect(func(): settings.reset_bindings(); _refresh_bindings())
	v.add_child(reset)
	_refresh_bindings()
	return scroll


func _start_rebind(action: String) -> void:
	_waiting = action
	_refresh_bindings()


func _refresh_bindings() -> void:
	for action in _bind_buttons:
		var btn: Button = _bind_buttons[action]
		btn.text = "press a key…" if action == _waiting else settings.binding(action)
		btn.modulate = Color(1.0, 0.85, 0.4) if action == _waiting else Color.WHITE


func _input(event: InputEvent) -> void:
	if not visible or _waiting == "":
		return
	if not event.is_pressed() or event.is_echo():
		return
	if event is InputEventKey and (event as InputEventKey).keycode == KEY_ESCAPE:
		_waiting = ""
	elif event is InputEventKey and (event as InputEventKey).keycode == KEY_BACKSPACE:
		settings.set_binding(_waiting, "None")
		_waiting = ""
	else:
		var text: String = settings.text_from_event(event)
		if text == "" or text == "Mouse Left":
			return  # a lone modifier, or the click that opened the rebind
		settings.set_binding(_waiting, text)
		_waiting = ""
	get_viewport().set_input_as_handled()
	_refresh_bindings()
