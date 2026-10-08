extends RefCounted
## The match camera (D13 framing): free by default, scrolled by bumping the cursor against the
## screen edges, the arrow keys, or dragging with the middle mouse button; locked on the champion
## with Y; centered on it while Space is held, and when the match starts or the champion
## respawns. The look point stays on the map.

const EDGE_PX := 6.0                     # how close to the window edge counts as a bump
const MARGIN_U := 300.0                  # the look point may go this far past the map's edge

var focus := Vector2.ZERO                # the look point, in game units
var _ready := false
var _drag_from := Vector2.ZERO
var _dragging := false
var _peeking := false                    # the minimap is held: the camera stays where it points


## Snap to `at` (match start, respawn).
func center(at: Vector2) -> void:
	focus = at
	_ready = true


## The minimap is clicked or dragged: look there while `held` (even when locked, like the
## reference game; a locked camera goes back to the champion on release).
func peek(at: Vector2, held: bool) -> void:
	focus = at
	_ready = true
	_peeking = held


## Moves the look point for this frame. `own` is the champion's position (or null), `size` the
## map's, `viewport` the game view, `units_per_px` how far a pixel of drag moves the camera.
func update(delta: float, settings, own, size: Vector2, viewport: Viewport, units_per_px: float, input_free: bool) -> void:
	var centering: bool = input_free and Input.is_action_pressed("camera_center")
	if own != null and (not _ready or centering or (settings.camera_locked and not _peeking)):
		center(own)
	if not settings.camera_locked and input_free and not _peeking:
		var dir := Vector2.ZERO
		if Input.is_action_pressed("pan_left"):
			dir.x -= 1.0
		if Input.is_action_pressed("pan_right"):
			dir.x += 1.0
		if Input.is_action_pressed("pan_up"):
			dir.y -= 1.0
		if Input.is_action_pressed("pan_down"):
			dir.y += 1.0
		if settings.edge_pan and settings.edge_pan_speed > 0.0 and DisplayServer.window_is_focused():
			var m := viewport.get_mouse_position()
			var r := viewport.get_visible_rect().size
			if m.x >= 0.0 and m.y >= 0.0 and m.x <= r.x and m.y <= r.y:
				if m.x <= EDGE_PX:
					dir.x -= 1.0
				elif m.x >= r.x - EDGE_PX:
					dir.x += 1.0
				if m.y <= EDGE_PX:
					dir.y -= 1.0
				elif m.y >= r.y - EDGE_PX:
					dir.y += 1.0
		if dir != Vector2.ZERO:
			focus += dir.normalized() * settings.pan_speed_u() * delta
		if _dragging:
			var m := viewport.get_mouse_position()
			focus -= (m - _drag_from) * units_per_px
			_drag_from = m
	if size != Vector2.ZERO:
		focus = focus.clamp(Vector2(-MARGIN_U, -MARGIN_U), size + Vector2(MARGIN_U, MARGIN_U))


## Middle-mouse drag (the caller forwards mouse button events).
func drag(pressed: bool, at: Vector2) -> void:
	_dragging = pressed
	_drag_from = at
