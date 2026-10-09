extends RefCounted
## Movable windows (01 §13): the augment draft, the shop, the anvil, the match breakdown, the
## death recap and the settings can be dragged by any spot that isn't a button, so they never
## hide what you need to see (your abilities' tooltips, while planning a build). Where you put
## each one is remembered in the settings, as a fraction of the screen, so it stays put across
## matches and window sizes.


## Lets `panel` be dragged; its place is saved under `name`.
static func make_movable(panel: Control, name: String, settings) -> void:
	panel.mouse_filter = Control.MOUSE_FILTER_STOP
	panel.gui_input.connect(func(e: InputEvent): _drag(panel, e, name, settings))


## Puts `panel` where it was left, else at `default` (top-left corner), inside `screen`.
## Does nothing while it's being dragged.
static func place(panel: Control, name: String, settings, default: Vector2, screen: Vector2) -> void:
	if panel.has_meta("drag_from"):
		return
	var at := default
	var saved = settings.window_positions.get(name)
	if saved is Array and saved.size() == 2:
		at = Vector2(float(saved[0]) * screen.x, float(saved[1]) * screen.y)
	panel.position = _inside(at, panel.size, screen)


static func _inside(at: Vector2, size: Vector2, screen: Vector2) -> Vector2:
	return at.clamp(Vector2.ZERO, (screen - size).max(Vector2.ZERO))


static func _drag(panel: Control, e: InputEvent, name: String, settings) -> void:
	var screen := panel.get_viewport_rect().size
	if e is InputEventMouseButton and (e as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
		if e.is_pressed():
			panel.set_meta("drag_from", panel.get_global_mouse_position() - panel.position)
			panel.mouse_default_cursor_shape = Control.CURSOR_MOVE
		elif panel.has_meta("drag_from"):
			panel.remove_meta("drag_from")
			panel.mouse_default_cursor_shape = Control.CURSOR_ARROW
			settings.window_positions[name] = [panel.position.x / maxf(screen.x, 1.0), panel.position.y / maxf(screen.y, 1.0)]
			settings.save()
		panel.accept_event()
	elif e is InputEventMouseMotion and panel.has_meta("drag_from"):
		var offset: Vector2 = panel.get_meta("drag_from")
		panel.position = _inside(panel.get_global_mouse_position() - offset, panel.size, screen)
		panel.accept_event()
