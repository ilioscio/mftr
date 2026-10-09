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
		hotspot = Vector2(6, 5)  # the fingertip, after the gauntlet's turn
	var img := Image.new()
	img.load_svg_from_string('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">%s</svg>' % svg, SIZE_PX / 64.0)
	return [img, hotspot * (SIZE_PX / 64.0)]


## A gauntlet pointing with its index finger: the finger rises from the edge of the fist, the
## other three curl over beside it, the thumb wraps the front. Turned to point up and to the
## left.
static func _gauntlet(plate: String, shade: String) -> String:
	var s := 'stroke="%s" stroke-width="2.4" stroke-linejoin="round"' % OUTLINE
	var g := ''
	# The cuff, gold-trimmed.
	g += '<path d="M19 50h30l2 10H17z" fill="#d4a94c" %s/>' % s
	g += '<path d="M20 54h29" stroke="#8a6a24" stroke-width="1.6"/>'
	# The back of the hand.
	g += '<path d="M18 30q0-4 4-4h22q5 0 5 5v15q0 6-6 6H23q-5 0-5-5z" fill="%s" %s/>' % [plate, s]
	# Three curled fingers: knuckles along the top, beside the pointing one.
	for x in [30, 36.5, 43]:
		g += '<rect x="%s" y="22" width="6.5" height="11" rx="3.2" fill="%s" %s/>' % [x, plate, s]
	# The index finger, in two plates, from the hand's left edge.
	g += '<path d="M19 30V9a5.5 5.5 0 0 1 11 0v21z" fill="%s" %s/>' % [plate, s]
	g += '<path d="M20 16h9M20 23h9" stroke="%s" stroke-width="1.6"/>' % shade
	g += '<path d="M22 10v16" stroke="#ffffff" stroke-width="1.8" opacity=".55"/>'
	# The thumb, across the front.
	g += '<path d="M18 38q-6 0-6 5t7 5h11q4 0 4-4t-4-4z" fill="%s" %s/>' % [plate, s]
	return '<g transform="translate(-8 -5) rotate(-25 32 32)">%s</g>' % g


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
