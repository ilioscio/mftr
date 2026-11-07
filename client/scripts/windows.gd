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
## A window bigger than the screen (a big HUD scale on a small one) is shrunk to fit.
## Does nothing while it's being dragged.
static func place(panel: Control, name: String, settings, default: Vector2, screen: Vector2) -> void:
	if panel.has_meta("drag_from"):
		return
	var at := default
	var saved = settings.window_positions.get(name)
	if saved is Array and saved.size() == 2:
		at = Vector2(float(saved[0]) * screen.x, float(saved[1]) * screen.y)
	panel.position = at
	_fit(panel, screen)
	# Its contents may grow it after it opens (the shop's lists): fit it again then.
	if not panel.has_meta("refits"):
		panel.set_meta("refits", true)
		panel.resized.connect(_refit.bind(panel))


static func _refit(panel: Control) -> void:
	if panel.is_inside_tree() and not panel.has_meta("drag_from"):
		_fit(panel, panel.get_viewport_rect().size)


## Shrinks `panel` to fit `screen` if it's bigger, and keeps it inside.
static func _fit(panel: Control, screen: Vector2) -> void:
	var fit := minf(1.0, minf(screen.x / maxf(panel.size.x, 1.0), screen.y / maxf(panel.size.y, 1.0)))
	panel.scale = Vector2(fit, fit)
	panel.position = _inside(panel.position, panel.size * fit, screen)


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
		panel.position = _inside(panel.get_global_mouse_position() - offset, panel.size * panel.scale, screen)
		panel.accept_event()
