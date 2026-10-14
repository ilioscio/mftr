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
	# Hit sellers (14 §4.3): a swing's arc smear, an impact star, ground cracks, a light column
	# and an expanding shock ring.
	"slash": { "count": 36, "size": 0.11, "speed": 0.6, "lifetime": 0.14 },
	"flash": { "count": 10, "size": 0.12, "speed": 5.0, "lifetime": 0.10 },
	"crack": { "count": 6, "size": 0.08, "speed": 0.0, "lifetime": 0.80 },
	"pillar": { "count": 24, "size": 0.10, "speed": 2.5, "lifetime": 0.35 },
	"shock": { "count": 0, "size": 0.12, "speed": 0.0, "lifetime": 0.30 },
}

var _mm: MultiMesh
var _pos := PackedVector3Array()
var _vel := PackedVector3Array()
var _age := PackedFloat32Array()
var _life := PackedFloat32Array()
var _size := PackedFloat32Array()
var _grow := PackedFloat32Array()
var _grav := PackedFloat32Array()
var _drag := PackedFloat32Array()
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


func _emit(p: Vector3, v: Vector3, life: float, size: float, ramp: PackedColorArray, grav := 0.0, grow := 0.0, drag := 0.9) -> void:
	if _pos.size() >= MAX_PARTICLES:
		return
	_pos.append(p)
	_vel.append(v)
	_age.append(0.0)
	_life.append(maxf(life, 0.02))
	_size.append(size)
	_grow.append(grow)
	_grav.append(grav)
	_drag.append(drag)
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
		"slash":
			_slash(spec, pos, dir, radius, n, size, speed, life, ramp)
		"flash":
			# A big white-hot core and a star of short rays.
			_emit(pos, Vector3.ZERO, life, size * 3.2, ramp, 0.0, -0.5)
			var rays := maxi(n, 4)
			var up := Vector3.UP
			for i in rays:
				var a := TAU * float(i) / float(rays) + _rng.randf_range(-0.2, 0.2)
				var d := Vector3(cos(a), 0.0, sin(a)) * 0.8 + up * sin(a) * 0.6
				_emit(pos, d.normalized() * speed, life * 1.4, size, ramp, 0.0, 0.0, 0.6)
		"crack":
			# Jagged lines radiating over the ground; they linger and step through the ramp.
			var r := radius if radius > 0.0 else 1.1 * size / KITS.crack.size
			var lines := maxi(n, 3)
			for i in lines:
				var a := TAU * float(i) / float(lines) + _rng.randf_range(-0.35, 0.35)
				var d := Vector3(cos(a), 0.0, sin(a))
				var side := Vector3(-d.z, 0.0, d.x)
				var steps := clampi(int(r / 0.09), 4, 16)
				var wob := 0.0
				for k in steps:
					var t := float(k + 1) / float(steps)
					wob = clampf(wob + _rng.randf_range(-0.07, 0.07), -0.18, 0.18)
					var p := Vector3(pos.x, 0.03, pos.z) + d * r * t + side * wob * r * t
					_emit(p, Vector3.ZERO, life * (1.0 - 0.4 * t), size * (1.15 - 0.5 * t), ramp, 0.0, 0.0, 1.0)
		"pillar":
			# A column of light rising from the ground.
			var g := Vector3(pos.x, 0.0, pos.z)
			for i in n:
				var h := _rng.randf_range(0.0, 2.2)
				var jitter := Vector3(_rng.randf_range(-0.12, 0.12), 0.0, _rng.randf_range(-0.12, 0.12))
				_emit(g + jitter + Vector3(0, h, 0), Vector3(0, speed * _rng.randf_range(0.6, 1.2), 0), life * _rng.randf_range(0.6, 1.0), size, ramp, 0.0, -0.3, 1.0)
		"shock":
			# A ring that expands from the center to `radius` (the gameplay radius) over its life.
			var r := radius if radius > 0.0 else 1.2 * size / float(KITS.shock.size)
			var count := n if n > 0 else clampi(int(r * 10.0), 16, 48)
			for i in count:
				var a := TAU * float(i) / float(count)
				var out := Vector3(cos(a), 0.0, sin(a))
				_emit(Vector3(pos.x, 0.08, pos.z) + out * r * 0.15, out * (r * 0.85 / life), life, size, ramp, 0.0, 0.0, 1.0)
		_:
			# Projectile styles and trails emit along a moving node; see `decorate`.
			pass


## An arc smear (14 §4.3): a crescent of pixels along the swing, all at once, thick in the
## middle and thin at the ends, drifting outward as it fades. `arc` (degrees) is its sweep and
## `tilt` its plane. Level swings (`tilt` under 45) sweep across the front, centered on `pos`;
## overhead swings come down from above the champion's shoulder and end on `pos`. `size` scales
## the reach. With no direction (novas) it's a full ring of `radius` around `pos`.
func _slash(spec: Dictionary, pos: Vector3, dir: Vector3, radius: float, n: int, size: float, speed: float, life: float, ramp: PackedColorArray) -> void:
	var arc := deg_to_rad(float(spec.get("arc", 140.0)))
	var tilt := deg_to_rad(float(spec.get("tilt", 0.0)))
	var m := size / float(KITS.slash.size)
	var fwd := Vector3(dir.x, 0.0, dir.z)
	var count := maxi(n, 8)
	var center: Vector3
	var r: float
	var a0: float
	var u: Vector3
	var b: Vector3
	var ring := fwd.length() < 0.01
	if ring:
		r = maxf(radius * 0.85, 0.8)
		center = Vector3(pos.x, 0.9, pos.z)
		u = Vector3.FORWARD
		b = Vector3.RIGHT
		arc = TAU
		a0 = 0.0
	else:
		fwd = fwd.normalized()
		var side := Vector3(-fwd.z, 0.0, fwd.x)
		if tilt < PI / 4.0:
			r = m
			center = Vector3(pos.x, 1.0, pos.z) - fwd * r
			u = fwd
			b = side * cos(tilt) + Vector3.UP * sin(tilt)
			a0 = -arc * 0.5
		else:
			var back := 0.9 * m
			center = Vector3(pos.x, 1.45, pos.z) - fwd * back
			var to := pos - center
			r = to.length()
			u = to.normalized()
			# Rotate from the impact direction back up over the shoulder.
			b = (Vector3.UP * sin(tilt) + side * cos(tilt))
			b = (b - u * b.dot(u)).normalized()
			a0 = 0.0
	for i in count:
		var t := float(i) / float(count - 1) if count > 1 else 0.5
		var a := a0 + arc * t
		var thick := 1.0 if ring else sin(PI * t)
		var o := (u * cos(a) + b * sin(a)).normalized()
		for layer in 2:
			var rr := r * (1.0 - 0.12 * float(layer) * thick)
			var p := center + o * rr
			p.y = maxf(p.y, 0.05)
			_emit(p, o * speed, life * (0.7 + 0.3 * thick), size * (0.6 + 0.6 * thick), ramp, 0.0, 0.0, 0.8)

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
		_vel[i] = v * _drag[i]
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
	_drag[i] = _drag[last]
	_ramp[i] = _ramp[last]
	_pos.resize(last)
	_vel.resize(last)
	_age.resize(last)
	_life.resize(last)
	_size.resize(last)
	_grow.resize(last)
	_grav.resize(last)
	_drag.resize(last)
	_ramp.resize(last)
