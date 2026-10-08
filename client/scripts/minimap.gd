extends Control
## The minimap (bottom right; Esc → Settings shows, hides and sizes it): the map from above, the
## fog of war (what our team doesn't see is darkened), champions, minions, structures and relics,
## and the camera's view on the ground. Left click or drag moves the camera there while held;
## right click orders a move.
##
## Lane maps are much wider than tall, so the view is widened to at most 2:1 with the scenery
## around the map; square maps stay square.

signal look(point: Vector2, held: bool)   # game units; held = the button is still down
signal move_to(point: Vector2)

const BASE_WIDTH := 380.0                # px at 100% for a 2:1 map (a square one gets 250)
const MARGIN := 14.0

var map_size := Vector2(4000, 4000)
var region := Rect2(0, 0, 4000, 4000)    # what the minimap shows, in game units
var geometry := {}
var terrain: Texture2D = null            # a top-down render of the map, when there is one
var fog_texture: ImageTexture = null
var icons: Array = []                    # [{ pos, kind, team: "own"/"ally"/"enemy", champion, color }]
var view_poly := PackedVector2Array()    # the camera's view on the ground, game units
var _fog: TextureRect
var _layer: Control
var _held := false


func _ready() -> void:
	mouse_filter = Control.MOUSE_FILTER_STOP
	clip_contents = true
	_fog = TextureRect.new()
	_fog.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_fog.stretch_mode = TextureRect.STRETCH_SCALE
	_fog.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR
	var mat := ShaderMaterial.new()
	var sh := Shader.new()
	sh.code = "shader_type canvas_item;\nvoid fragment() { float seen = texture(TEXTURE, UV).r; COLOR = vec4(0.02, 0.03, 0.06, (1.0 - seen) * 0.62); }"
	mat.shader = sh
	_fog.material = mat
	add_child(_fog)
	_layer = Control.new()
	_layer.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	_layer.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_layer.set_anchors_preset(Control.PRESET_FULL_RECT)
	_layer.draw.connect(_draw_icons)
	add_child(_layer)


## The map (from `map_geometry`): sets the region shown.
func setup(geo: Dictionary) -> void:
	geometry = geo
	map_size = geo.size
	region = Rect2(Vector2.ZERO, map_size)
	if map_size.x > map_size.y * 2.0:
		var h := map_size.x / 2.0
		region = Rect2(0, (map_size.y - h) / 2.0, map_size.x, h)
	queue_redraw()


## Size and place it in the bottom-right corner of `screen` at `percent` of the default.
func layout(screen: Vector2, percent: float) -> void:
	var aspect := region.size.x / region.size.y
	var w := (BASE_WIDTH if aspect > 1.2 else 250.0) * percent / 100.0
	size = Vector2(w, w / aspect)
	position = screen - size - Vector2(MARGIN, MARGIN)
	var r := _to_px_rect(Rect2(Vector2.ZERO, map_size))
	_fog.position = r.position
	_fog.size = r.size
	_fog.texture = fog_texture
	_layer.queue_redraw()


func refresh() -> void:
	_fog.texture = fog_texture
	_layer.queue_redraw()


func to_px(p: Vector2) -> Vector2:
	return (p - region.position) / region.size * size


func to_units(px: Vector2) -> Vector2:
	return region.position + px / size * region.size


func _to_px_rect(r: Rect2) -> Rect2:
	return Rect2(to_px(r.position), r.size / region.size * size)


func _draw() -> void:
	# Frame and terrain (a render of the map, or flat colors from its geometry).
	draw_rect(Rect2(Vector2.ZERO, size), Color(0.1, 0.16, 0.11))
	if terrain != null:
		draw_texture_rect(terrain, Rect2(Vector2.ZERO, size), false, Color(1.45, 1.45, 1.4))
	else:
		draw_rect(_to_px_rect(Rect2(Vector2.ZERO, map_size)), Color(0.24, 0.34, 0.2))
		if map_size.x != map_size.y:
			var lane := Rect2(0, map_size.y / 2.0 - 650.0, map_size.x, 1300.0)
			draw_rect(_to_px_rect(lane), Color(0.46, 0.4, 0.3))
		for poly in geometry.get("brush", []):
			draw_colored_polygon(_poly_px(poly), Color(0.16, 0.3, 0.14))
		for poly in geometry.get("walls", []):
			draw_colored_polygon(_poly_px(poly), Color(0.2, 0.19, 0.22))
	for f in geometry.get("fountains", []):
		var c := Color(0.3, 0.75, 0.85, 0.5) if f.ally else Color(0.9, 0.35, 0.3, 0.5)
		draw_circle(to_px(f.center), f.radius / region.size.x * size.x, c)


func _poly_px(poly: PackedVector2Array) -> PackedVector2Array:
	var out := PackedVector2Array()
	for p in poly:
		out.append(to_px(p))
	return out


func _draw_icons() -> void:
	var font := get_theme_default_font()
	var k := size.x / BASE_WIDTH
	for i in icons:
		var p := to_px(i.pos)
		var team_color: Color = {"own": Color(0.35, 0.65, 1.0), "ally": Color(0.3, 0.85, 0.8), "enemy": Color(0.95, 0.32, 0.25)}[i.team]
		match i.kind:
			"minion":
				_layer.draw_circle(p, maxf(1.6, 2.4 * k), team_color.darkened(0.15))
			"turret":
				var s := maxf(3.0, 5.0 * k)
				_layer.draw_rect(Rect2(p - Vector2(s, s), Vector2(s, s) * 2.0), Color(0.08, 0.08, 0.1))
				_layer.draw_rect(Rect2(p - Vector2(s, s) * 0.7, Vector2(s, s) * 1.4), team_color)
			"gatehouse":
				var s := maxf(4.0, 6.5 * k)
				var d := PackedVector2Array([p + Vector2(0, -s), p + Vector2(s, 0), p + Vector2(0, s), p + Vector2(-s, 0)])
				_layer.draw_colored_polygon(d, Color(0.08, 0.08, 0.1))
				_layer.draw_colored_polygon(PackedVector2Array([p + Vector2(0, -s * 0.7), p + Vector2(s * 0.7, 0), p + Vector2(0, s * 0.7), p + Vector2(-s * 0.7, 0)]), team_color)
			"base":
				var s := maxf(5.0, 8.0 * k)
				var hex := PackedVector2Array()
				for j in 6:
					hex.append(p + Vector2.from_angle(TAU * j / 6.0) * s)
				_layer.draw_colored_polygon(hex, Color(0.08, 0.08, 0.1))
				_layer.draw_circle(p, s * 0.62, team_color)
			"relic":
				_layer.draw_circle(p, maxf(2.5, 3.5 * k), Color(0.3, 0.95, 0.45))
			"champion":
				var r := maxf(6.0, (11.0 if i.team == "own" else 9.5) * k)
				_layer.draw_circle(p, r + 2.0, team_color)
				var face: Texture2D = i.get("face")
				if face != null:
					# The champion's portrait, rendered from its model (portraits.gd).
					_layer.draw_texture_rect(face, Rect2(p - Vector2(r, r), Vector2(r, r) * 2.0), false)
				else:
					_layer.draw_circle(p, r, i.color)
				if face == null and font != null:
					var letter: String = i.champion.substr(0, 1)
					var fs := int(maxf(9.0, 12.0 * k))
					_layer.draw_string(font, p + Vector2(-r, fs * 0.36), letter, HORIZONTAL_ALIGNMENT_CENTER, r * 2.0, fs, Color.WHITE)
	# The camera's view on the ground.
	if view_poly.size() == 4:
		var px := PackedVector2Array()
		for q in view_poly:
			px.append(to_px(q))
		px.append(px[0])
		_layer.draw_polyline(px, Color(1, 1, 1, 0.9), 1.5)
	# The frame matches the HUD's panels: a dark band with a thin gold rim on each side.
	# (Inside the edge: the minimap clips what's drawn past it.)
	var r := Rect2(Vector2.ZERO, size)
	_layer.draw_rect(r.grow(-2.5), Color(0.035, 0.045, 0.06), false, 5.0)
	_layer.draw_rect(r.grow(-0.75), Color(0.42, 0.35, 0.2), false, 1.5)
	_layer.draw_rect(r.grow(-5.5), Color(0.78, 0.65, 0.38, 0.75), false, 1.0)


func _gui_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_LEFT:
			_held = mb.pressed
			look.emit(_clamped(mb.position), mb.pressed)
			accept_event()
		elif mb.button_index == MOUSE_BUTTON_RIGHT and mb.pressed:
			move_to.emit(_clamped(mb.position))
			accept_event()
	elif event is InputEventMouseMotion and _held:
		look.emit(_clamped((event as InputEventMouseMotion).position), true)
		accept_event()


func _clamped(px: Vector2) -> Vector2:
	return to_units(px).clamp(Vector2.ZERO, map_size)
