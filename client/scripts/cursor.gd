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
		hotspot = Vector2(13, 3)  # the fingertip, after the gauntlet's turn
	var img := Image.new()
	img.load_svg_from_string('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">%s</svg>' % svg, SIZE_PX / 64.0)
	return [img, hotspot * (SIZE_PX / 64.0)]


## A gauntlet pointing like the classic hand cursor, from the back of the hand: the index
## finger straight up, the other three curled down beside it (each knuckle its own step), the
## thumb out to the side. Tilted a little to the left.
static func _gauntlet(plate: String, shade: String) -> String:
	var s := 'stroke="%s" stroke-width="2.4" stroke-linejoin="round"' % OUTLINE
	var g := ''
	# The cuff, gold-trimmed.
	g += '<path d="M22 52h28l1 9H21z" fill="#d4a94c" %s/>' % s
	g += '<path d="M22.5 56h27.5" stroke="#8a6a24" stroke-width="1.6"/>'
	# The thumb, out to the side.
	g += '<rect x="5" y="33" width="24" height="10" rx="5" transform="rotate(-32 27 38)" fill="%s" %s/>' % [plate, s]
	# The curled fingers: middle, ring, little, each a step lower.
	for f in [[30, 19, 8.5], [38, 22, 8.5], [46, 26, 7.5]]:
		g += '<rect x="%s" y="%s" width="%s" height="22" rx="4" fill="%s" %s/>' % [f[0], f[1], f[2], plate, s]
	# The index finger, in two plates.
	g += '<rect x="20" y="3" width="10.5" height="36" rx="5.25" fill="%s" %s/>' % [plate, s]
	g += '<path d="M21 13h8.5M21 21h8.5" stroke="%s" stroke-width="1.6"/>' % shade
	g += '<path d="M23 5v24" stroke="#ffffff" stroke-width="1.8" opacity=".55"/>'
	# The back of the hand, over the fingers' roots, the fingers' lines running into it.
	g += '<path d="M19 34h35v9q0 10-10 10H29q-10 0-10-10z" fill="%s" %s/>' % [plate, s]
	for x in [30.5, 38.25, 46.25]:
		g += '<path d="M%s 34v7" stroke="%s" stroke-width="1.6" stroke-linecap="round"/>' % [x, shade]
	return '<g transform="translate(-6 -2) rotate(-12 32 32)">%s</g>' % g


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
