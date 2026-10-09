extends RefCounted
## The game cursor (05 §6): a steel gauntlet pointing the way, as in the genre, drawn here as
## SVG (no image files). It changes with what's under it, like the reference game's: red
## plates and a blade over an enemy you can attack, teal plates over an ally, a reticle while
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
		svg = _gauntlet(plate, shade, state == "enemy")
		hotspot = Vector2(6, 5)  # the fingertip, after the gauntlet's turn
	var img := Image.new()
	img.load_svg_from_string('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">%s</svg>' % svg, SIZE_PX / 64.0)
	return [img, hotspot * (SIZE_PX / 64.0)]


## A gauntlet with its index finger out, turned to point up and to the left.
static func _gauntlet(plate: String, shade: String, blade: bool) -> String:
	var s := 'stroke="%s" stroke-width="2.4" stroke-linejoin="round"' % OUTLINE
	var g := ''
	# The cuff, gold-trimmed.
	g += '<path d="M21 49h26l3 11H18z" fill="#d4a94c" %s/>' % s
	g += '<path d="M22 53h24" stroke="#8a6a24" stroke-width="1.6"/>'
	# The fist and the thumb.
	g += '<path d="M17 31q0-5 5-5h21q7 0 7 7v11q0 6-6 6H22q-5 0-5-5z" fill="%s" %s/>' % [plate, s]
	g += '<path d="M38 33h11M38 39h11M38 45h10" stroke="%s" stroke-width="1.6"/>' % shade
	g += '<path d="M18 34q-7 0-7 6t7 6" fill="%s" %s/>' % [plate, s]
	# The index finger, in two plates.
	g += '<path d="M26 30V9a6 6 0 0 1 12 0v21z" fill="%s" %s/>' % [plate, s]
	g += '<path d="M27 17h10M27 24h10" stroke="%s" stroke-width="1.6"/>' % shade
	g += '<path d="M29 10v17" stroke="#ffffff" stroke-width="1.8" opacity=".55"/>'
	if blade:
		# A short blade across the fist: this one can be attacked.
		g += '<path d="M24 46l20-14 3 3-20 14z" fill="#f2f4f7" %s/>' % s
		g += '<path d="M23 43l6 7" stroke="#d4a94c" stroke-width="3" stroke-linecap="round"/>'
	return '<g transform="translate(-12 -3) rotate(-30 32 32)">%s</g>' % g


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
