extends PanelContainer
## Esc → Settings: the camera, the minimap and the keybinds. Every change applies and saves at
## once. Rebinding: click a binding, then press the key (with Alt / Ctrl / Shift) or the mouse
## button for it; Esc cancels, Backspace clears it.

var settings
var _waiting := ""                       # the action being rebound, or ""
var _bind_buttons := {}                  # action -> Button


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
	var tabs := TabContainer.new()
	tabs.size_flags_vertical = Control.SIZE_EXPAND_FILL
	v.add_child(tabs)
	tabs.add_child(_camera_tab())
	tabs.add_child(_interface_tab())
	tabs.add_child(_graphics_tab())
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
	position = ((get_parent_control_size() - size) / 2.0).max(Vector2.ZERO)


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
	control.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	control.tooltip_text = hint
	parent.add_child(control)


func _grid(name_: String) -> GridContainer:
	var g := GridContainer.new()
	g.name = name_
	g.columns = 2
	g.add_theme_constant_override("h_separation", 16)
	g.add_theme_constant_override("v_separation", 10)
	return g


func _check(on: bool, apply: Callable) -> CheckBox:
	var c := CheckBox.new()
	c.button_pressed = on
	c.toggled.connect(func(v): apply.call(v); settings.save())
	return c


func _slider(value: float, lo: float, hi: float, apply: Callable, unit := "") -> HBoxContainer:
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
	s.value_changed.connect(func(v): apply.call(v); l.text = "%d%s" % [roundi(v), unit])
	s.drag_ended.connect(func(_c): settings.save())
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
	_row(g, "Show the minimap", _check(settings.minimap_shown, func(v): settings.minimap_shown = v))
	_row(g, "Minimap size", _slider(settings.minimap_size, 50, 160, func(v): settings.minimap_size = v, "%"))
	return g


func _graphics_tab() -> Control:
	var g := _grid("Graphics")
	_row(g, "Shadows", _check(settings.shadows, func(v): settings.shadows = v), "The sun's shadows. Off is faster on older graphics cards.")
	_row(g, "Bloom", _check(settings.bloom, func(v): settings.bloom = v), "A soft glow around bright things: crystals, fire, spells.")
	_row(g, "Clouds and motes", _check(settings.atmosphere, func(v): settings.atmosphere = v), "Cloud shadows drifting over the map, and motes floating in the light.")
	return g


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
