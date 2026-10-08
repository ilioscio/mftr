extends Node
## Champion portraits, rendered from the champions' own models (05 §6), so they follow every
## change to a model with nothing to redraw by hand. Each champion is posed in its idle, lit
## by a warm key and a cool rim against a backdrop in its identity color, and rendered once
## per session in an offscreen viewport of its own world:
##   "bust"  — head and shoulders, square (champion select, the loading screen's cards)
##   "round" — the bust in a circle with transparent corners (the HUD)
##   "small" — the round one at 64 px (the minimap)
##   "full"  — the whole champion, tall (the champion-select splash, the loading screen)
## `portrait()` returns null until a champion is rendered and queues it; `rendered` fires when
## it's ready. One champion renders at a time, a few frames each.

signal rendered(champion: String)

const BUST_PX := 256
const FULL_PX := Vector2i(384, 576)
const HUMAN_DEPTH := 0.5                # a human-sized champion's chest, front to back, meters
const ACCENT := Color(0.78, 0.65, 0.38)  # a neutral gold where team colors would go
const SLOTS := ["skin", "cloth", "metal", "emissive", "accent", "accent_glow"]

var _model_for: Callable                 # champion -> MftrModel
var _template: MftrModel                 # the shared template (wears the identity color)
var _colors := {}                        # champion -> identity color
var _cache := {}                         # champion -> { bust, round, small, full }
var _queue: Array[String] = []
var _busy := false


func setup(model_for: Callable, template: MftrModel, colors: Dictionary) -> void:
	_model_for = model_for
	_template = template
	_colors = colors


## The champion's portrait of `kind`, or null while it's being rendered (or without a model).
func portrait(champion: String, kind := "bust") -> Texture2D:
	if champion == "":
		return null
	if _cache.has(champion):
		return _cache[champion].get(kind)
	if not _queue.has(champion):
		_queue.append(champion)
	return null


## Whether a champion's portraits are done (or can't be made: no model).
func is_done(champion: String) -> bool:
	return _cache.has(champion)


## Render these in the background now (champion select's champions, say).
func prepare(champions: Array) -> void:
	for c in champions:
		portrait(c)


func _process(_delta: float) -> void:
	if not _busy and not _queue.is_empty():
		_render(_queue.pop_front())


func _render(champion: String) -> void:
	_busy = true
	var source: MftrModel = _model_for.call(champion) if _model_for.is_valid() else null
	if source == null:
		_cache[champion] = {}
		_busy = false
		rendered.emit(champion)
		return
	var vp := SubViewport.new()
	vp.size = Vector2i(BUST_PX * 2, BUST_PX * 2)
	vp.own_world_3d = true
	vp.msaa_3d = Viewport.MSAA_4X
	vp.render_target_update_mode = SubViewport.UPDATE_ALWAYS
	add_child(vp)

	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_COLOR
	e.background_color = Color(0.05, 0.06, 0.08)
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color(0.6, 0.68, 0.85)
	e.ambient_light_energy = 0.32
	e.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	e.tonemap_exposure = 1.2
	e.tonemap_white = 4.0
	env.environment = e
	vp.add_child(env)
	var key := DirectionalLight3D.new()
	key.rotation_degrees = Vector3(-30, 35, 0)
	key.light_color = Color(1.0, 0.93, 0.82)
	key.light_energy = 1.75
	vp.add_child(key)
	var rim := DirectionalLight3D.new()
	rim.rotation_degrees = Vector3(-15, 200, 0)
	rim.light_color = Color(0.6, 0.75, 1.0)
	rim.light_energy = 1.1
	vp.add_child(rim)

	var tint: Color = _colors.get(champion, Color(0.5, 0.5, 0.55))
	var backdrop := MeshInstance3D.new()
	var quad := QuadMesh.new()
	quad.size = Vector2(8, 8)
	backdrop.mesh = quad
	backdrop.position = Vector3(0, 1.0, -1.6)
	var bm := ShaderMaterial.new()
	bm.shader = _backdrop_shader()
	bm.set_shader_parameter("tint", tint)
	backdrop.material_override = bm
	vp.add_child(backdrop)

	var model: Node3D = source.instantiate()
	vp.add_child(model)
	var mesh: MeshInstance3D = model.get_node("Skeleton/Mesh")
	var slots := source.surface_slots()
	for i in slots.size():
		var m := ShaderMaterial.new()
		m.shader = load("res://shaders/champion_model.gdshader")
		m.set_shader_parameter("slot", SLOTS.find(slots[i]))
		m.set_shader_parameter("team_accent", ACCENT)
		m.set_shader_parameter("identity", tint)
		m.set_shader_parameter("identity_mix", 0.6 if source == _template else 0.0)
		m.set_shader_parameter("cloud_receive", 0.0)
		mesh.set_surface_override_material(i, m)
	# The idle pose, settled.
	var skeleton: Skeleton3D = model.get_node("Skeleton")
	var animator = source.new_animator()
	for i in 12:
		animator.drive(skeleton, {}, 0.0, 0.1)
	await get_tree().process_frame
	var head_i := skeleton.find_bone("head")
	var head := Vector3(0, 1.55, 0)
	if head_i >= 0:
		head = skeleton.global_transform * skeleton.get_bone_global_pose(head_i).origin
	var height := maxf(mesh.get_aabb().size.y, 1.0)
	# How much bigger than a human the champion's torso is (Bastion's is huge): the head's
	# distance above the lower spine, against a human's.
	var depth := _chest_depth(mesh.mesh, head.y)
	var bulk := maxf(1.0, depth / HUMAN_DEPTH)

	var cam := Camera3D.new()
	vp.add_child(cam)
	cam.make_current()
	# Bust: a three-quarter view of the head and shoulders, from a little above.
	var aim := head + Vector3(0, 0.04, 0)
	var dir := Vector3(sin(deg_to_rad(28.0)), 0.18, cos(deg_to_rad(28.0))).normalized()
	cam.fov = 26.0
	# Framed for a human-sized champion; bigger ones (Bastion) are framed wider.
	cam.position = aim + dir * (0.31 * bulk / tan(deg_to_rad(13.0)))
	cam.look_at(aim, Vector3.UP)
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw
	var bust := vp.get_texture().get_image()
	bust.resize(BUST_PX, BUST_PX, Image.INTERPOLATE_LANCZOS)

	# Full: the whole champion, tall.
	vp.size = FULL_PX
	var mid := Vector3(0, height * 0.5, 0)
	dir = Vector3(sin(deg_to_rad(22.0)), 0.1, cos(deg_to_rad(22.0))).normalized()
	cam.fov = 30.0
	cam.position = mid + dir * (height * 0.62 / tan(deg_to_rad(15.0)))
	cam.look_at(mid, Vector3.UP)
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw
	var full := vp.get_texture().get_image()
	vp.queue_free()

	var round := _circle(bust)
	var small: Image = round.duplicate()
	small.resize(64, 64, Image.INTERPOLATE_LANCZOS)
	# Mipmaps: these are drawn small (the HUD, the minimap) and stay smooth there.
	for img in [bust, round, small, full]:
		img.generate_mipmaps()
	_cache[champion] = {
		"bust": ImageTexture.create_from_image(bust),
		"round": ImageTexture.create_from_image(round),
		"small": ImageTexture.create_from_image(small),
		"full": ImageTexture.create_from_image(full),
	}
	_busy = false
	rendered.emit(champion)


## How deep the body is, front to back, around the chest (in the rest pose; the arms reach
## out sideways, so depth isn't thrown off by them the way width would be).
static func _chest_depth(mesh: Mesh, head_y: float) -> float:
	var lo := INF
	var hi := -INF
	for i in mesh.get_surface_count():
		var verts: PackedVector3Array = mesh.surface_get_arrays(i)[Mesh.ARRAY_VERTEX]
		for v in verts:
			if v.y > head_y - 0.7 and v.y < head_y:
				lo = minf(lo, v.z)
				hi = maxf(hi, v.z)
	return hi - lo if hi > lo else HUMAN_DEPTH


## The image inside a circle, with soft transparent corners.
static func _circle(src: Image) -> Image:
	var img: Image = src.duplicate()
	img.convert(Image.FORMAT_RGBA8)
	var n := img.get_width()
	var c := (n - 1) / 2.0
	for y in n:
		for x in n:
			var d := Vector2(x - c, y - c).length()
			var a := clampf(c - d, 0.0, 1.0)
			if a < 1.0:
				var p := img.get_pixel(x, y)
				p.a *= a
				img.set_pixel(x, y, p)
	return img


static var _backdrop: Shader


static func _backdrop_shader() -> Shader:
	if _backdrop == null:
		_backdrop = Shader.new()
		_backdrop.code = """
shader_type spatial;
render_mode unshaded, shadows_disabled;
uniform vec3 tint : source_color;
void fragment() {
	// A soft light behind the champion, in its color, darkening to the edges.
	float r = length((UV - vec2(0.5, 0.45)) * vec2(1.0, 1.15));
	vec3 hi = mix(tint, vec3(1.0), 0.18) * 0.8;
	ALBEDO = mix(hi, tint * 0.12, smoothstep(0.02, 0.32, r));
}
"""
	return _backdrop
