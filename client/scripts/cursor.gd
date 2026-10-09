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
		hotspot = Vector2(18, 3)  # the fingertip, after the gauntlet's turn
	var img := Image.new()
	img.load_svg_from_string('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">%s</svg>' % svg, SIZE_PX / 64.0)
	return [img, hotspot * (SIZE_PX / 64.0)]


## A gauntlet pointing like the classic hand cursor, from the back of the hand: the index
## finger straight up, the other three curled beside it (nearly level, upright), and the thumb
## wrapped around the front of the palm from the left, its tip toward the fingers.
static func _gauntlet(plate: String, shade: String) -> String:
	var s := 'stroke="%s" stroke-width="2.4" stroke-linejoin="round"' % OUTLINE
	var g := ''
	# The cuff, gold-trimmed.
	g += '<path d="M23 53h28l1 9H22z" fill="#d4a94c" %s/>' % s
	g += '<path d="M23.5 57.5h28" stroke="#8a6a24" stroke-width="1.6"/>'
	# The curled fingers: middle, ring, little, upright, each just a little lower.
	for f in [[30, 19, 8.5], [38.5, 20, 8.5], [47, 22, 7.5]]:
		g += '<rect x="%s" y="%s" width="%s" height="24" rx="4" fill="%s" %s/>' % [f[0], f[1], f[2], plate, s]
	# The index finger, in two plates.
	g += '<rect x="19.5" y="3" width="10.5" height="38" rx="5.25" fill="%s" %s/>' % [plate, s]
	g += '<path d="M20.5 13h8.5M20.5 21h8.5" stroke="%s" stroke-width="1.6"/>' % shade
	g += '<path d="M22.5 5v24" stroke="#ffffff" stroke-width="1.8" opacity=".55"/>'
	# The back of the hand, over the fingers' roots.
	g += '<path d="M19 33h35.5v10q0 11-11 11H30q-11 0-11-11z" fill="%s" %s/>' % [plate, s]
	for x in [30.25, 38.75, 47.25]:
		g += '<path d="M%s 33v6" stroke="%s" stroke-width="1.6" stroke-linecap="round"/>' % [x, shade]
	# The thumb, wrapped around the front of the palm from the left, tip toward the fingers.
	g += '<path d="M21 34q-8 1-8 9t9 9h13q5 0 5-4.5t-5-4.5H26q-3 0-4-3z" fill="%s" %s/>' % [plate, s]
	g += '<path d="M16 41q1 5 6 6" fill="none" stroke="#ffffff" stroke-width="1.6" stroke-linecap="round" opacity=".45"/>'
	return '<g transform="translate(-4 -1) rotate(-6 32 32)">%s</g>' % g


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
