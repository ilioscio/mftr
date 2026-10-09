extends RefCounted
## The game cursor (05 §6): a steel gauntlet pointing the way, as in the genre, drawn here as
## SVG (no image files). It changes with what's under it, like the reference game's: red
## plates over an enemy you can attack, teal plates over an ally, a reticle while
## an attack-move waits for its click. Menus and panels get the plain gauntlet.

const SIZE_PX := 40
const OUTLINE := "#1b140e"

static var _cache := {}
static var _state := ""


## Show the cursor for `state`: "default", "enemy", "ally" or "attack" (a no-op if unchanged).
static func set_state(state: String) -> void:
	if state == _state:
		return
	_state = state
	if not _cache.has(state):
		_cache[state] = _make(state)
	var made: Array = _cache[state]
	Input.set_custom_mouse_cursor(made[0], Input.CURSOR_ARROW, made[1])


## [image, hotspot] for a state.
static func _make(state: String) -> Array:
	var svg: String
	var hotspot: Vector2
	if state == "attack":
		svg = _reticle()
		hotspot = Vector2(32, 32)
	else:
		var plate: String = {"enemy": "#d65a4a", "ally": "#4fc2b4"}.get(state, "#c9d0da")
		var shade: String = {"enemy": "#8f2f25", "ally": "#2b7f75"}.get(state, "#7d8794")
		svg = _gauntlet(plate, shade)
		hotspot = Vector2(5, 5)  # the fingertip, after the gauntlet's turn
	var img := Image.new()
	img.load_svg_from_string('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">%s</svg>' % svg, SIZE_PX / 64.0)
	return [img, hotspot * (SIZE_PX / 64.0)]


## A gauntlet pointing with its index finger, seen from the back of the hand: the finger
## rises from the hand's left edge, the other three are folded into the fist (their plates as
## ridges across it), the thumb is out of sight. Turned to point up and to the left.
static func _gauntlet(plate: String, shade: String) -> String:
	var s := 'stroke="%s" stroke-width="2.4" stroke-linejoin="round"' % OUTLINE
	var g := ''
	# The cuff, gold-trimmed.
	g += '<path d="M20 48h29l2 11H18z" fill="#d4a94c" %s/>' % s
	g += '<path d="M21 52.5h28" stroke="#8a6a24" stroke-width="1.6"/>'
	# The fist: its top steps down from the finger to the little finger's knuckle.
	g += '<path d="M18 30q0-5 5-5h6q3.5-4 7-1q3.5-3 7 0q3.5-2.5 6.5 1q3.5 1 3.5 5.5v15q0 6-6 6H23q-5 0-5-5z" fill="%s" %s/>' % [plate, s]
	# The folded fingers' plates, as ridges across the back of the fist.
	for y in [33, 39, 45]:
		g += '<path d="M31 %dq9-2 18 0" fill="none" stroke="%s" stroke-width="1.8" stroke-linecap="round"/>' % [y, shade]
	g += '<path d="M30 28v21" stroke="%s" stroke-width="1.6"/>' % shade
	# The index finger, in two plates, up the hand's left edge.
	g += '<path d="M18 32V9a5.5 5.5 0 0 1 11 0v23z" fill="%s" %s/>' % [plate, s]
	g += '<path d="M19 16h9M19 23h9" stroke="%s" stroke-width="1.6"/>' % shade
	g += '<path d="M21 10v18" stroke="#ffffff" stroke-width="1.8" opacity=".55"/>'
	return '<g transform="translate(-10 -6) rotate(-20 32 32)">%s</g>' % g


## Attack-move: a red reticle, aimed at its center.
static func _reticle() -> String:
	var c := "#ff5a4a"
	var r := ''
	r += '<circle cx="32" cy="32" r="13" fill="none" stroke="%s" stroke-width="5"/>' % OUTLINE
	r += '<circle cx="32" cy="32" r="13" fill="none" stroke="%s" stroke-width="2.6"/>' % c
	for d in ['M32 11v9', 'M32 44v9', 'M11 32h9', 'M44 32h9']:
		r += '<path d="%s" stroke="%s" stroke-width="5" stroke-linecap="round"/>' % [d, OUTLINE]
		r += '<path d="%s" stroke="%s" stroke-width="2.6" stroke-linecap="round"/>' % [d, c]
	r += '<circle cx="32" cy="32" r="2.6" fill="%s" stroke="%s" stroke-width="1.4"/>' % [c, OUTLINE]
	return r
