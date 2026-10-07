## The VFX kit (05 §5, A4b): pixel-style particles and projectile decorations, driven by the
## `<id>.vfx.ron` effects of each champion's pack (validated by `mftr-pack`).
##
## Particles live in one MultiMesh and are simulated "on twos" (STEP_HZ steps a second), so they
## move in the chunky steps of the PS1 era; their squares snap to screen pixels in the shader.
## Projectile bodies, telegraphs and hitbox edges are not drawn here: they stay smooth and keep
## their gameplay sizes (05 §1). Sizes that matter (a detonation ring's radius) come from the
## gameplay data the caller passes in.
extends Node3D

const STEP_HZ := 20.0
const MAX_PARTICLES := 2048

# Kit defaults, overridden per effect by `count`, `size` (a multiplier), `speed`, `lifetime`.
const KITS := {
	"flare": { "count": 8, "size": 0.10, "speed": 2.5, "lifetime": 0.12 },
	"burst": { "count": 14, "size": 0.09, "speed": 3.5, "lifetime": 0.30 },
	"ring": { "count": 0, "size": 0.11, "speed": 1.4, "lifetime": 0.35 },
	"dust": { "count": 8, "size": 0.16, "speed": 0.8, "lifetime": 0.50 },
	"trail": { "count": 2, "size": 0.07, "speed": 0.4, "lifetime": 0.25 },
	"orb": { "count": 1, "size": 0.07, "speed": 0.3, "lifetime": 0.20 },
	"arrow": { "count": 2, "size": 0.06, "speed": 0.2, "lifetime": 0.22 },
	"net": { "count": 1, "size": 0.08, "speed": 0.3, "lifetime": 0.25 },
	"lob": { "count": 1, "size": 0.08, "speed": 0.6, "lifetime": 0.30 },
}

var _mm: MultiMesh
var _pos := PackedVector3Array()
var _vel := PackedVector3Array()
var _age := PackedFloat32Array()
var _life := PackedFloat32Array()
var _size := PackedFloat32Array()
var _grow := PackedFloat32Array()
var _grav := PackedFloat32Array()
var _ramp := []                         # PackedColorArray per particle
var _acc := 0.0
var _rng := RandomNumberGenerator.new()
# Projectiles that leave a trail: { node, spec, kit }.
var _trails := []
# Bombs in flight or waiting to go off: { node, from, to, start, flight, spec }.
var _lobs := []


func _ready() -> void:
	_rng.seed = 7
	_mm = MultiMesh.new()
	_mm.transform_format = MultiMesh.TRANSFORM_3D
	_mm.use_colors = true
	_mm.mesh = QuadMesh.new()
	_mm.instance_count = MAX_PARTICLES
	_mm.visible_instance_count = 0
	var mmi := MultiMeshInstance3D.new()
	mmi.multimesh = _mm
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/pixel_vfx.gdshader")
	mmi.material_override = m
	# One pixel block per 540 lines (2 px at 1080p).
	m.set_shader_parameter("block", maxf(1.0, roundf(get_viewport().get_visible_rect().size.y / 540.0)))
	# Particles are scattered; never cull the whole set by its stale bounds.
	mmi.custom_aabb = AABB(Vector3(-1e4, -1e4, -1e4), Vector3(2e4, 2e4, 2e4))
	add_child(mmi)


func _knob(spec: Dictionary, kit: String, key: String) -> float:
	var v = spec.get(key, -1)
	var d: float = KITS[kit][key]
	if v == null or float(v) < 0.0:
		return d
	return d * float(v) if key == "size" else float(v)


func _emit(p: Vector3, v: Vector3, life: float, size: float, ramp: PackedColorArray, grav := 0.0, grow := 0.0) -> void:
	if _pos.size() >= MAX_PARTICLES:
		return
	_pos.append(p)
	_vel.append(v)
	_age.append(0.0)
	_life.append(maxf(life, 0.02))
	_size.append(size)
	_grow.append(grow)
	_grav.append(grav)
	_ramp.append(ramp)


func _rand_dir(up_bias := 0.0) -> Vector3:
	var d := Vector3(_rng.randf_range(-1, 1), _rng.randf_range(-1, 1) + up_bias, _rng.randf_range(-1, 1))
	return d.normalized() if d.length() > 0.01 else Vector3.UP


## A particle kit at `pos`. `dir` aims flares; `radius` (m) sizes rings from gameplay data.
func play(spec: Dictionary, pos: Vector3, dir := Vector3.ZERO, radius := 0.0) -> void:
	var kit: String = spec.kit
	if not KITS.has(kit):
		return
	var ramp: PackedColorArray = spec.ramp
	var n := int(_knob(spec, kit, "count"))
	var size := _knob(spec, kit, "size")
	var speed := _knob(spec, kit, "speed")
	var life := _knob(spec, kit, "lifetime")
	match kit:
		"flare":
			_emit(pos, Vector3.ZERO, life * 0.5, size * 1.6, ramp)
			for i in n:
				var d := (_rand_dir() + dir * 1.2).normalized()
				_emit(pos, d * speed * _rng.randf_range(0.6, 1.0), life * _rng.randf_range(0.7, 1.0), size, ramp)
		"burst":
			for i in n:
				var d := _rand_dir(0.6)
				_emit(pos, d * speed * _rng.randf_range(0.5, 1.0), life * _rng.randf_range(0.6, 1.0), size * _rng.randf_range(0.8, 1.3), ramp, 9.0)
		"ring":
			var r := maxf(radius, 0.2)
			var count := n if n > 0 else clampi(int(r * 9.0), 12, 40)
			for i in count:
				var a := TAU * float(i) / float(count)
				var out := Vector3(cos(a), 0.0, sin(a))
				_emit(Vector3(pos.x, 0.08, pos.z) + out * r, out * speed + Vector3(0, 0.4, 0), life, size, ramp)
		"dust":
			for i in n:
				var d := Vector3(_rng.randf_range(-1, 1), _rng.randf_range(0.0, 0.4), _rng.randf_range(-1, 1)).normalized()
				_emit(Vector3(pos.x, 0.06, pos.z), d * speed, life * _rng.randf_range(0.7, 1.0), size, ramp, -0.4, 0.6)
		_:
			# Projectile styles and trails emit along a moving node; see `decorate`.
			pass


## A projectile's style (`arrow`, `net`, `orb`): decorations inside its gameplay-sized body,
## plus a trail of pixel sparks behind it. `radius` (m) is the projectile's hitbox radius.
func decorate(node: Node3D, spec: Dictionary, radius: float, lift := 0.35) -> void:
	var kit: String = spec.kit
	var ramp: PackedColorArray = spec.ramp
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = ramp[0]
	match kit:
		"arrow":
			# A thin bright shaft along the flight direction (+X in missile space), never wider
			# than the body.
			var shaft := MeshInstance3D.new()
			var box := BoxMesh.new()
			var w := minf(0.035, radius)
			box.size = Vector3(0.55, w, w)
			shaft.mesh = box
			shaft.material_override = mat
			shaft.position = Vector3(-0.2, lift, 0)
			node.add_child(shaft)
		"net":
			# A spinning square frame filling 90% of the body's width.
			var frame := Node3D.new()
			frame.name = "NetFrame"
			frame.position.y = lift
			var half := radius * 0.9 / sqrt(2.0)
			for k in 4:
				var bar := MeshInstance3D.new()
				var b := BoxMesh.new()
				b.size = Vector3(half * 2.0, 0.025, 0.025)
				bar.mesh = b
				bar.material_override = mat
				var a := k * PI / 2.0
				bar.position = Vector3(cos(a), 0, sin(a)) * half
				bar.rotation.y = -a + PI / 2.0
				frame.add_child(bar)
			node.add_child(frame)
		"orb", "lob":
			pass
	_trails.append({ "node": node, "spec": spec, "kit": kit, "lift": lift })


## A thrown bomb (`lob`) from `from` to `to`, landing after `flight` seconds; it then sizzles in
## place until `detonate` is called for the same `key` (the area's detonation).
func lob(key: int, spec: Dictionary, from: Vector3, to: Vector3, flight: float) -> void:
	var ramp: PackedColorArray = spec.ramp
	var bomb := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(0.18, 0.18, 0.18)
	bomb.mesh = box
	var mat := StandardMaterial3D.new()
	mat.albedo_color = ramp[0]
	bomb.material_override = mat
	bomb.position = from
	add_child(bomb)
	_lobs.append({ "key": key, "node": bomb, "from": from, "to": Vector3(to.x, 0.12, to.z), "age": 0.0, "flight": maxf(flight, 0.05), "spec": spec })


## The bomb for `key` goes off: remove it.
func land(key: int) -> void:
	for l in _lobs.duplicate():
		if l.key == key:
			l.node.queue_free()
			_lobs.erase(l)


func _process(delta: float) -> void:
	# Bombs fly smoothly (they're objects, not particles) and sizzle on twos.
	for l in _lobs:
		l.age += delta
		var t := clampf(l.age / l.flight, 0.0, 1.0)
		var p: Vector3 = l.from.lerp(l.to, t)
		p.y += sin(t * PI) * 1.4
		l.node.position = p
		l.node.rotation += Vector3(7.0, 5.0, 0.0) * delta * (1.0 - t)
	_acc += delta
	if _acc < 1.0 / STEP_HZ:
		return
	var dt := _acc
	_acc = 0.0
	_step(dt)


func _step(dt: float) -> void:
	# Trails: a spark or two behind every decorated projectile, each step.
	for tr in _trails.duplicate():
		var node = tr.node
		if not is_instance_valid(node) or not node.is_inside_tree():
			_trails.erase(tr)
			continue
		if not node.visible:
			continue
		var spec: Dictionary = tr.spec
		var kit: String = tr.kit
		var frame: Node3D = node.get_node_or_null("NetFrame")
		if frame != null:
			frame.rotation.y += dt * 9.0
		for i in int(_knob(spec, kit, "count")):
			var p: Vector3 = node.global_position + Vector3(0, tr.lift, 0) + _rand_dir() * 0.06
			_emit(p, _rand_dir() * _knob(spec, kit, "speed"), _knob(spec, kit, "lifetime"), _knob(spec, kit, "size"), spec.ramp)
	for l in _lobs:
		var spec: Dictionary = l.spec
		_emit(l.node.position + Vector3(0, 0.12, 0), _rand_dir(1.0) * 0.8, 0.2, 0.06, spec.ramp, 2.0)

	var i := 0
	while i < _pos.size():
		_age[i] += dt
		if _age[i] >= _life[i]:
			_remove(i)
			continue
		var v := _vel[i]
		v.y -= _grav[i] * dt
		_pos[i] += v * dt
		_vel[i] = v * 0.9
		i += 1
	var n := _pos.size()
	_mm.visible_instance_count = n
	for k in n:
		var f := _age[k] / _life[k]
		var ramp: PackedColorArray = _ramp[k]
		# Stepped through the ramp: whole colors, no blending between them (pixel style).
		var c: Color = ramp[mini(int(f * ramp.size()), ramp.size() - 1)]
		c.a = 1.0 if f < 0.75 else 0.6
		var s := _size[k] * (1.0 + _grow[k] * f)
		_mm.set_instance_transform(k, Transform3D(Basis().scaled(Vector3.ONE * s), _pos[k]))
		_mm.set_instance_color(k, c)


func _remove(i: int) -> void:
	var last := _pos.size() - 1
	_pos[i] = _pos[last]
	_vel[i] = _vel[last]
	_age[i] = _age[last]
	_life[i] = _life[last]
	_size[i] = _size[last]
	_grow[i] = _grow[last]
	_grav[i] = _grav[last]
	_ramp[i] = _ramp[last]
	_pos.resize(last)
	_vel.resize(last)
	_age.resize(last)
	_life.resize(last)
	_size.resize(last)
	_grow.resize(last)
	_grav.resize(last)
	_ramp.resize(last)
