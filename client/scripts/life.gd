extends Node3D
## Small life (15 §8): flowers in the grass near the cliffs and the brush, lily pads on the
## river's edges, frogs on its banks that hop now and then, butterflies looping over the flowers
## and glowing spores drifting over the jungle's darker ground. A detail every few screens, never
## on a lane, a plinth or a camp's dirt.
##
## Placed from the map's ground control map (15 §3) with a fixed seed, so every map the editor
## makes gets the same treatment as an official one, and the same map always looks the same. The
## season says which life appears (`season.life`). Purely visual: nothing here is seen by the
## simulation, and none of it is drawn on the minimap (render layer 1 only).

const U := 0.01
const TEXEL_U := 25.0                     # the control map's texel (ground.rs)
const WALL_REACH_U := 1000.0              # its alpha's full scale either side of 0.5

const FROG := 3.0                         # the frog's scale (its model is life size)
const BUTTERFLY_COLORS := [Color(0.98, 0.62, 0.16), Color(0.96, 0.9, 0.55), Color(0.6, 0.78, 1.0)]

var _ctl: Image
var _size := Vector2.ZERO
var _rng := RandomNumberGenerator.new()
var _t := 0.0

var _pads: MultiMeshInstance3D
var _pad_list := []                       # [position (m), turn, phase]
var _frogs := []                          # { node, home, from, to, t, wait, turn }
var _flies: MultiMeshInstance3D
var _fly_list := []                       # { home (m), r, w, phase }


## `ctl` is the map's ground control map; `materials_for(id)` returns [mesh, materials] or null
## (as for the scenery).
func build(geo: Dictionary, ctl: Image, season: Dictionary, materials_for: Callable) -> void:
	_ctl = ctl
	_size = geo.size
	_rng.seed = 4111
	var life: Array = season.get("life", [])
	var tint: Color = season.get("tint", Color.WHITE)
	var area := _size.x * _size.y
	var patches := []
	if "flowers" in life:
		patches = _flowers(area, tint, materials_for)
	if "lily_pads" in life:
		_lily_pads(area, tint, materials_for)
	if "frogs" in life:
		_place_frogs(area, materials_for)
	if "butterflies" in life:
		_butterflies(patches)
	if "fireflies" in life:
		_spores(area)


## What the ground is at `p` (u): r dirt, g paving, b water, a signed wall distance (over 0.5 in
## the open; see `_wall_u`).
func _at(p: Vector2) -> Color:
	var x := clampi(int(p.x / TEXEL_U), 0, _ctl.get_width() - 1)
	var y := clampi(int(p.y / TEXEL_U), 0, _ctl.get_height() - 1)
	return _ctl.get_pixel(x, y)


## How far open ground is from the nearest wall (u); 0 in a wall.
func _wall_u(c: Color) -> float:
	return maxf(c.a * 2.0 - 1.0, 0.0) * WALL_REACH_U


## Plain grass: no dirt, paving or water, and not in a wall.
func _grass(c: Color) -> bool:
	return c.r < 0.04 and c.g < 0.04 and c.b < 0.02 and c.a > 0.5


func _somewhere() -> Vector2:
	return Vector2(_rng.randf_range(0.0, _size.x), _rng.randf_range(0.0, _size.y))


## Flower clumps in small patches, mostly near the foot of the cliffs (where the grass is left
## alone), a few out in the open. Returns the patches' centers (u), for the butterflies.
func _flowers(area: float, tint: Color, materials_for: Callable) -> Array:
	var batches := {"flowers_1": [], "flowers_2": []}
	var patches := []
	for i in int(area / 0.25e6):
		var p := _somewhere()
		var c := _at(p)
		if not _grass(c):
			continue
		var wall_u := _wall_u(c)
		var near_cliff := wall_u > 80.0 and wall_u < 420.0
		if not near_cliff and _rng.randf() > 0.05:
			continue
		patches.append(p)
		var id: String = "flowers_1" if _rng.randf() < 0.55 else "flowers_2"
		for k in _rng.randi_range(2, 5):
			var q := p + Vector2.from_angle(_rng.randf_range(0, TAU)) * _rng.randf_range(0.0, 110.0)
			if _grass(_at(q)):
				batches[id].append(_placed(q, 0.0, _rng.randf_range(2.6, 3.4), tint))
	for id in batches:
		_batch(id, batches[id], materials_for)
	return patches


## Lily pads in twos and threes on the river's shallow edges, away from where a lane crosses.
func _lily_pads(area: float, tint: Color, materials_for: Callable) -> void:
	var list := []
	for i in int(area / 0.06e6):
		var p := _somewhere()
		var c := _at(p)
		if c.b < 0.3 or c.b > 0.8 or c.r > 0.05 or c.g > 0.05 or _rng.randf() > 0.35:
			continue
		for k in _rng.randi_range(1, 3):
			var q := p + Vector2.from_angle(_rng.randf_range(0, TAU)) * _rng.randf_range(0.0, 120.0)
			var cq := _at(q)
			if cq.b > 0.25 and cq.r < 0.05:
				list.append(_placed(q, 0.015, _rng.randf_range(2.4, 3.4), tint))
				_pad_list.append([Vector3(q.x * U, 0.015, q.y * U), _rng.randf_range(0, TAU), _rng.randf_range(0, TAU)])
	_pads = _batch("lily_1", list, materials_for)


## A few frogs on the river's banks, each with its own patch of bank to hop about.
func _place_frogs(area: float, materials_for: Callable) -> void:
	var made = materials_for.call("frog_1")
	if made == null:
		return
	var wanted := clampi(int(area / 25e6), 2, 8)
	for i in 3000:
		if _frogs.size() >= wanted:
			break
		var p := _somewhere()
		var c := _at(p)
		if not _bank(c) or _wall_u(c) < 80.0:
			continue  # (not under a cliff, where the camera can't see it)
		if _frogs.any(func(f): return f.home.distance_to(p) < 900.0):
			continue
		var n := MeshInstance3D.new()
		n.mesh = made[0]
		for s in made[1].size():
			var m: ShaderMaterial = made[1][s]
			m.set_shader_parameter("instance_tint", false)  # one node, not a batch
			n.set_surface_override_material(s, m)
		n.layers = 1
		n.scale = Vector3.ONE * FROG
		add_child(n)
		var turn := _rng.randf_range(0, TAU)
		n.position = Vector3(p.x * U, 0.0, p.y * U)
		n.rotation.y = turn
		_frogs.append({"node": n, "home": p, "from": p, "to": p, "t": 1.0, "wait": _rng.randf_range(1.0, 6.0), "turn": turn, "landed": -9.0})


## The bank: grass or mud just out of the water.
func _bank(c: Color) -> bool:
	return c.b > 0.04 and c.b < 0.3 and c.r < 0.05 and c.g < 0.05 and c.a > 0.5


## Butterflies over some of the flower patches, in ones and twos.
func _butterflies(patches: Array) -> void:
	var wing := ShaderMaterial.new()
	wing.shader = load("res://shaders/butterfly.gdshader")
	var list := []
	for p in patches:
		if list.size() >= 16 or _rng.randf() > 0.3:
			continue
		var color: Color = BUTTERFLY_COLORS[_rng.randi() % BUTTERFLY_COLORS.size()]
		for k in _rng.randi_range(1, 2):
			_fly_list.append({"home": Vector3(p.x * U, 0.0, p.y * U), "r": Vector2(_rng.randf_range(0.5, 1.4), _rng.randf_range(0.5, 1.4)),
				"w": Vector2(_rng.randf_range(0.35, 0.6), _rng.randf_range(0.45, 0.8)), "phase": _rng.randf_range(0, TAU)})
			list.append(color)
	if list.is_empty():
		return
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = _butterfly_mesh()
	mm.instance_count = list.size()
	for i in list.size():
		var c: Color = list[i]
		mm.set_instance_custom_data(i, Color(c.r, c.g, c.b, _fly_list[i].phase))
	_flies = MultiMeshInstance3D.new()
	_flies.name = "Butterflies"
	_flies.multimesh = mm
	_flies.material_override = wing
	_flies.layers = 1
	_flies.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	add_child(_flies)
	_update_flies()


## A butterfly: two pairs of wings either side of a thin body along +z (the shader flaps them
## about it; x is the distance from the body).
static func _butterfly_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for s in [-1.0, 1.0]:
		var fore := [Vector3(0, 0, 0.03), Vector3(s * 0.15, 0, 0.09), Vector3(s * 0.13, 0, -0.01), Vector3(0, 0, -0.01)]
		var hind := [Vector3(0, 0, -0.01), Vector3(s * 0.11, 0, -0.02), Vector3(s * 0.07, 0, -0.1), Vector3(0, 0, -0.06)]
		for q in [fore, hind]:
			for idx in [0, 1, 2, 0, 2, 3]:
				st.set_color(Color(1, 1, 1, 1.0 if q == fore else 0.8))
				st.add_vertex(q[idx])
	st.set_color(Color(0.15, 0.12, 0.1, 0.0))
	for v in [Vector3(-0.012, 0, 0.05), Vector3(0.012, 0, 0.05), Vector3(0, 0, -0.07)]:
		st.add_vertex(v)
	return st.commit()


## Glowing spores drifting low over the jungle's darker ground near the cliffs.
func _spores(area: float) -> void:
	var made := 0
	for i in int(area / 2e6):
		if made >= 10:
			break
		var p := _somewhere()
		var c := _at(p)
		var wall_u := _wall_u(c)
		if c.a <= 0.5 or wall_u > 320.0 or c.r > 0.6 or c.g > 0.05 or c.b > 0.05 or _rng.randf() > 0.5:
			continue
		var e := GPUParticles3D.new()
		e.amount = 9
		e.lifetime = 5.0
		e.preprocess = 5.0
		e.randomness = 0.6
		e.layers = 1
		e.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
		e.position = Vector3(p.x * U, 0.6, p.y * U)
		e.visibility_aabb = AABB(Vector3(-5, -1, -5), Vector3(10, 4, 10))
		var pm := ParticleProcessMaterial.new()
		pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
		pm.emission_box_extents = Vector3(2.6, 0.5, 2.6)
		pm.direction = Vector3(0, 1, 0)
		pm.spread = 180.0
		pm.initial_velocity_min = 0.02
		pm.initial_velocity_max = 0.12
		pm.gravity = Vector3(0, 0.03, 0)
		pm.turbulence_enabled = true
		pm.turbulence_noise_strength = 0.35
		pm.turbulence_noise_scale = 3.0
		var fade := Gradient.new()
		fade.set_color(0, Color(1, 1, 1, 0))
		fade.set_color(1, Color(1, 1, 1, 0))
		fade.add_point(0.3, Color(1, 1, 1, 1))
		fade.add_point(0.7, Color(1, 1, 1, 1))
		var ft := GradientTexture1D.new()
		ft.gradient = fade
		pm.color_ramp = ft
		e.process_material = pm
		var quad := QuadMesh.new()
		quad.size = Vector2(0.06, 0.06)
		var m := StandardMaterial3D.new()
		m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
		m.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
		m.vertex_color_use_as_albedo = true
		m.albedo_color = Color(0.7, 1.0, 0.6, 0.8)
		m.emission_enabled = true
		m.emission = Color(0.55, 1.0, 0.5)
		m.emission_energy_multiplier = 2.2
		quad.material = m
		e.draw_pass_1 = quad
		add_child(e)
		made += 1


func _placed(p: Vector2, height: float, scale: float, tint: Color) -> Array:
	var basis := Basis(Vector3.UP, _rng.randf_range(0.0, TAU)).scaled(Vector3.ONE * scale)
	var k := _rng.randf_range(0.92, 1.08)
	return [Transform3D(basis, Vector3(p.x * U, height, p.y * U)), Color(tint.r * k, tint.g * k, tint.b * k) * 0.5]


func _batch(id: String, list: Array, materials_for: Callable) -> MultiMeshInstance3D:
	if list.is_empty():
		return null
	var made = materials_for.call(id)
	if made == null:
		return null
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_custom_data = true
	mm.mesh = made[0]
	mm.instance_count = list.size()
	for i in list.size():
		mm.set_instance_transform(i, list[i][0])
		mm.set_instance_custom_data(i, list[i][1])
	var node := MultiMeshInstance3D.new()
	node.name = id
	node.multimesh = mm
	node.layers = 1
	node.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	for i in made[1].size():
		mm.mesh.surface_set_material(i, made[1][i])
	add_child(node)
	return node


func _process(delta: float) -> void:
	_t += delta
	if _pads != null:
		# The pads turn a little on the current.
		var mm := _pads.multimesh
		for i in _pad_list.size():
			var pd: Array = _pad_list[i]
			var t := mm.get_instance_transform(i)
			var s := t.basis.get_scale().x
			var turn: float = pd[1] + sin(_t * 0.4 + pd[2]) * 0.18
			t.basis = Basis(Vector3.UP, turn).scaled(Vector3.ONE * s)
			t.origin = pd[0] + Vector3(0, sin(_t * 1.3 + pd[2]) * 0.006, 0)
			mm.set_instance_transform(i, t)
	for f in _frogs:
		_hop(f, delta)
	if _flies != null:
		_update_flies()


## A frog sits, then hops a short way along its bank: a quick arc, a squash as it lands.
func _hop(f: Dictionary, delta: float) -> void:
	var n: MeshInstance3D = f.node
	if f.t < 1.0:
		f.t = minf(f.t + delta / 0.42, 1.0)
		var p: Vector2 = f.from.lerp(f.to, f.t)
		n.position = Vector3(p.x * U, sin(f.t * PI) * 0.28, p.y * U)
		var stretch := 1.0 + 0.25 * sin(f.t * PI)
		n.scale = Vector3(FROG / sqrt(stretch), FROG * stretch, FROG / sqrt(stretch))
		return
	# Landing: a squash that settles.
	f.wait -= delta
	var squash := 1.0 - 0.22 * clampf(1.0 - (_t - f.landed) / 0.2, 0.0, 1.0)
	n.scale = Vector3(FROG / sqrt(squash), FROG * squash, FROG / sqrt(squash))
	if f.wait > 0.0:
		return
	f.wait = _rng.randf_range(3.0, 9.0)
	f.from = f.to
	for k in 6:
		var to: Vector2 = f.from + Vector2.from_angle(_rng.randf_range(0, TAU)) * _rng.randf_range(40.0, 90.0)
		if to.distance_to(f.home) < 300.0 and _bank(_at(to)):
			f.to = to
			break
	if f.to == f.from:
		return
	f.turn = atan2(f.to.x - f.from.x, f.to.y - f.from.y)  # the model faces +z
	n.rotation.y = f.turn
	f.t = 0.0
	f.landed = _t + 0.42


## Butterflies loop over their flowers, bobbing, always facing the way they're going.
func _update_flies() -> void:
	var mm := _flies.multimesh
	for i in _fly_list.size():
		var b: Dictionary = _fly_list[i]
		var t: float = _t + b.phase * 7.0
		var off := Vector3(sin(t * b.w.x + b.phase) * b.r.x, 0.0, cos(t * b.w.y) * b.r.y)
		var vel := Vector3(cos(t * b.w.x + b.phase) * b.r.x * b.w.x, 0.0, -sin(t * b.w.y) * b.r.y * b.w.y)
		var pos: Vector3 = b.home + off + Vector3(0, 0.7 + 0.25 * sin(t * 1.7) + 0.08 * sin(t * 9.0), 0)
		var basis := Basis(Vector3.UP, atan2(vel.x, vel.z)).scaled(Vector3.ONE * 2.2)
		mm.set_instance_transform(i, Transform3D(basis, pos))
