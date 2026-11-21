## The VFX kit (05 §5, 23, A4b): particles, ribbons and projectile decorations, driven by the
## `<id>.vfx.ron` effects of each champion's pack (validated by `mftr-pack`).
##
## Particles (23 §3) are emitted here in each kit's pattern, simulated every frame by
## `MftrParticles` (Rust, `crates/mftr-gdext/src/particles.rs`) and drawn by
## `vfx_particle.gdshader` in two MultiMeshes, the covering layer (smoke, dust, debris) under the glowing one (sparks, flames,
## flashes). Each is a procedural shape, soft against the scene, its color walking the effect's
## ramp smoothly, its light over 1 at birth so bloom catches what's hot. Ribbons (23 §4) are
## strips rebuilt each frame for what points can't draw: lightning, lasers, swing smears, cracks
## and columns of light (`vfx_ribbon.gdshader`).
##
## Projectile bodies, telegraphs and hitbox edges are not drawn here: they stay exact and keep
## their gameplay sizes (05 §1). Sizes that matter (a detonation ring's radius) come from the
## gameplay data the caller passes in.
extends Node3D

const MAX_RIBBONS := 64

# Particle shapes (vfx_particle.gdshader).
const GLOW := 0
const SPARK := 1
const FLAME := 2
const SMOKE := 3
const SHARD := 4
const STAR := 5
const RING := 6
const DISC := 7
# Per shape: additive (1 adds light, 0 covers), heat (light at birth; over 1 blooms), soft (m of
# depth fade), stretch (s of travel drawn as a streak).
const SHAPES := {
	GLOW: { "add": 1.0, "heat": 1.8, "soft": 0.4, "stretch": 0.0 },
	SPARK: { "add": 1.0, "heat": 2.4, "soft": 0.0, "stretch": 0.045 },
	FLAME: { "add": 0.5, "heat": 1.35, "soft": 0.3, "stretch": 0.0 },
	SMOKE: { "add": 0.0, "heat": 1.0, "soft": 0.6, "stretch": 0.0 },
	SHARD: { "add": 0.0, "heat": 1.05, "soft": 0.0, "stretch": 0.0 },
	STAR: { "add": 1.0, "heat": 2.6, "soft": 0.5, "stretch": 0.0 },
	RING: { "add": 0.9, "heat": 1.6, "soft": 0.0, "stretch": 0.0 },
	DISC: { "add": 1.0, "heat": 1.0, "soft": 0.0, "stretch": 0.0 },
}
# Particle flags.
const BOUNCE := 1                        # hits the ground and bounces
const PUFF := 2                          # raises a puff of dust where it first lands
const SPLAT := 4                         # dies where it lands
const FLICKER := 8                       # twinkles

# Ribbon styles (vfx_ribbon.gdshader).
const LIGHTNING := 0
const LASER := 1
const SMEAR := 2
const CRACK := 3
const COLUMN := 4

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
	# The shaped projectiles' trails (14 §4.4): the body's shape is the missile shader's; these
	# are what it sheds (a `beam` is also a laser along its length, a `flame` licks of fire).
	"wave": { "count": 3, "size": 0.09, "speed": 0.8, "lifetime": 0.30 },
	"shard": { "count": 2, "size": 0.08, "speed": 0.5, "lifetime": 0.25 },
	"beam": { "count": 3, "size": 0.08, "speed": 0.6, "lifetime": 0.30 },
	"bones": { "count": 2, "size": 0.08, "speed": 0.6, "lifetime": 0.30 },
	"flame": { "count": 3, "size": 0.09, "speed": 0.9, "lifetime": 0.28 },
	"bolt": { "count": 2, "size": 0.06, "speed": 0.3, "lifetime": 0.22 },
	"grab": { "count": 1, "size": 0.07, "speed": 0.3, "lifetime": 0.20 },
	# Hit sellers (14 §4.3): a swing's arc smear, an impact star, ground cracks, a light column
	# and an expanding shock ring.
	"slash": { "count": 36, "size": 0.11, "speed": 0.6, "lifetime": 0.14 },
	"flash": { "count": 10, "size": 0.12, "speed": 5.0, "lifetime": 0.10 },
	"crack": { "count": 6, "size": 0.08, "speed": 0.0, "lifetime": 0.80 },
	"pillar": { "count": 24, "size": 0.10, "speed": 2.5, "lifetime": 0.35 },
	"shock": { "count": 0, "size": 0.12, "speed": 0.0, "lifetime": 0.30 },
	# Things that fall: stones dropping onto an area, and a volley of arrows that keeps falling
	# across it for `lifetime` seconds (14 §4.3).
	"rocks": { "count": 14, "size": 0.26, "speed": 9.0, "lifetime": 0.45 },
	"rain": { "count": 6, "size": 0.07, "speed": 16.0, "lifetime": 2.0 },
	# Moods (20 §8.3, the creator's moves): power drawn in on a rising helix, sparks drifting up
	# off something hot, crystals that shatter and fall, smoke that swells and hangs, glimmers
	# that hover.
	"vortex": { "count": 28, "size": 0.08, "speed": 2.4, "lifetime": 0.5 },
	"embers": { "count": 16, "size": 0.06, "speed": 1.2, "lifetime": 1.0 },
	"shards": { "count": 12, "size": 0.12, "speed": 5.0, "lifetime": 0.6 },
	"smoke": { "count": 10, "size": 0.30, "speed": 0.8, "lifetime": 1.2 },
	"motes": { "count": 14, "size": 0.07, "speed": 0.4, "lifetime": 1.2 },
	# Elements (23 §5): a fire that burns across the area for `lifetime` seconds (flames every
	# tenth of a second, `count` at a time), and `count` bolts of lightning striking it.
	"fire": { "count": 7, "size": 0.55, "speed": 1.6, "lifetime": 1.0 },
	"lightning": { "count": 1, "size": 1.0, "speed": 0.0, "lifetime": 0.35 },
}

var sun: DirectionalLight3D              # lights smoke and debris; found on the parent if unset

var _mm_cover: MultiMesh                 # smoke, dust, debris: drawn first
var _mm_glow: MultiMesh                  # light: drawn over them
var _mats: Array[ShaderMaterial] = []
var _im: ImmediateMesh
var _ribbon_mat: ShaderMaterial

var _sim := MftrParticles.new()      # every live particle
var _rng := RandomNumberGenerator.new()
var _t := 0.0
# Ribbons: { style, lines: [{pts, w} camera-facing | {a, b} flat], age, life, ramp, heat, ... }.
var _ribbons := []
# Projectiles that leave a trail: { node, spec, kit, lift, owed }.
var _trails := []
# Bombs in flight or waiting to go off: { node, from, to, start, flight, spec }.
var _lobs := []
# Rains and fires still going: { kit, pos, radius, left, spec, owed }.
var _spells := []
# Lightning still to strike: { at, pos, radius, spec }.
var _strikes := []


func _ready() -> void:
	_rng.seed = 7
	var shader: Shader = load("res://shaders/vfx_particle.gdshader")
	_mm_cover = _layer(shader, 0)
	_mm_glow = _layer(shader, 1)
	_sim.set_dust(PackedColorArray(DUST))
	_im = ImmediateMesh.new()
	var ri := MeshInstance3D.new()
	ri.mesh = _im
	ri.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	ri.custom_aabb = AABB(Vector3(-1e4, -1e4, -1e4), Vector3(2e4, 2e4, 2e4))
	_ribbon_mat = ShaderMaterial.new()
	_ribbon_mat.shader = load("res://shaders/vfx_ribbon.gdshader")
	_ribbon_mat.render_priority = 2
	add_child(ri)


func _layer(shader: Shader, priority: int) -> MultiMesh:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.use_colors = true
	mm.use_custom_data = true
	mm.mesh = QuadMesh.new()
	mm.instance_count = MftrParticles.capacity()
	mm.visible_instance_count = 0
	var mmi := MultiMeshInstance3D.new()
	mmi.multimesh = mm
	mmi.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	var m := ShaderMaterial.new()
	m.shader = shader
	m.render_priority = priority
	mmi.material_override = m
	_mats.append(m)
	# Particles are scattered; never cull the whole set by its stale bounds.
	mmi.custom_aabb = AABB(Vector3(-1e4, -1e4, -1e4), Vector3(2e4, 2e4, 2e4))
	add_child(mmi)
	return mm


func _knob(spec: Dictionary, kit: String, key: String) -> float:
	var v = spec.get(key, -1)
	var d: float = KITS[kit][key]
	if v == null or float(v) < 0.0:
		return d
	return d * float(v) if key == "size" else float(v)


## A particle. `o` overrides: grav (m/s² down), grow (size added by the end, eased), drag (speed
## kept per twentieth of a second), rot, spin (rad/s), add, heat, soft, stretch, flags.
func _emit(p: Vector3, v: Vector3, life: float, size: float, ramp: PackedColorArray, shape := GLOW, o := {}) -> void:
	var d: Dictionary = SHAPES[shape]
	# Flames stand upright; everything else lands at any turn.
	var rot: float = o.get("rot", 0.0 if shape == FLAME else _rng.randf() * TAU)
	_sim.emit(p, v, ramp, shape, PackedFloat32Array([life, size, o.get("grow", 0.0), o.get("grav", 0.0), o.get("drag", 0.9),
		rot, o.get("spin", 0.0), o.get("flags", 0), o.get("add", d.add), o.get("heat", d.heat), o.get("soft", d.soft),
		o.get("stretch", d.stretch)]))


func _rand_dir(up_bias := 0.0) -> Vector3:
	var d := Vector3(_rng.randf_range(-1, 1), _rng.randf_range(-1, 1) + up_bias, _rng.randf_range(-1, 1))
	return d.normalized() if d.length() > 0.01 else Vector3.UP


## Whether a ramp starts hot (bright enough to read as light: sparks, fire, magic) rather than
## matter (stone chips, soot): a burst of a dark ramp throws debris, not sparks.
func _hot(ramp: PackedColorArray) -> bool:
	var c: Color = ramp[0]
	return maxf(c.r, maxf(c.g, c.b)) > 0.75 and c.get_luminance() > 0.5


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
	var m := size / float(KITS[kit].size)       # the effect's size multiplier
	var ground := Vector3(pos.x, 0.0, pos.z)
	match kit:
		"flare":
			# A glow at the socket and a spray of sparks along the shot.
			_emit(pos, Vector3.ZERO, life * 1.4, size * 5.0, ramp, GLOW, {"grow": -0.4})
			for i in n:
				var d := (_rand_dir() + dir * 1.2).normalized()
				_emit(pos, d * speed * _rng.randf_range(0.6, 1.2), life * _rng.randf_range(1.2, 2.0), size * 0.55, ramp, SPARK, {"drag": 0.8})
		"burst":
			# Sparks that fly out, fall and skip off the ground; a dark ramp throws chips instead.
			var hot := _hot(ramp)
			if hot:
				_emit(pos, Vector3.ZERO, life * 0.6, size * 6.0, ramp, GLOW, {"grow": 0.3})
			for i in n:
				var d := _rand_dir(0.6)
				var v := d * speed * _rng.randf_range(0.5, 1.15)
				if hot:
					_emit(pos, v, life * _rng.randf_range(0.8, 1.5), size * _rng.randf_range(0.45, 0.7), ramp, SPARK, {"grav": 9.0, "drag": 0.94, "flags": BOUNCE})
				else:
					_emit(pos, v * 0.8, life * _rng.randf_range(1.2, 1.8), size * _rng.randf_range(0.9, 1.6), ramp, SHARD,
						{"grav": 14.0, "drag": 0.97, "spin": _rng.randf_range(-12, 12), "flags": BOUNCE})
		"ring":
			# A ring of light on the area's edge (its gameplay radius), drifting out a touch, and
			# glimmers rising off it.
			var r := maxf(radius, 0.2)
			_emit(Vector3(pos.x, 0.06, pos.z), Vector3.ZERO, life * 1.3, r * 2.0, ramp, RING, {"grow": 0.06 * speed})
			var count := n if n > 0 else clampi(int(r * 6.0), 8, 28)
			for i in count:
				var a := TAU * (float(i) + _rng.randf()) / float(count)
				var out := Vector3(cos(a), 0.0, sin(a))
				_emit(Vector3(pos.x, 0.1, pos.z) + out * r, out * speed * 0.4 + Vector3(0, _rng.randf_range(0.5, 1.4), 0),
					life * _rng.randf_range(1.2, 2.2), size * 1.6, ramp, GLOW, {"drag": 0.9, "grow": -0.5})
		"dust":
			# Slow, swelling puffs that hug the ground.
			for i in n:
				var d := Vector3(_rng.randf_range(-1, 1), _rng.randf_range(0.0, 0.4), _rng.randf_range(-1, 1)).normalized()
				_emit(Vector3(pos.x, 0.18, pos.z) + d * 0.15, d * speed, life * _rng.randf_range(1.2, 1.8), size * 2.6, ramp, SMOKE,
					{"grav": -0.3, "grow": 1.1, "drag": 0.88})
		"slash":
			_slash(spec, pos, dir, radius, size, speed, life, ramp)
		"flash":
			# An impact glint: a star, a hot core, short rays, and the light thrown on the ground.
			_emit(pos, Vector3.ZERO, life * 1.6, size * 7.0, ramp, STAR, {"grow": 0.5})
			_emit(pos, Vector3.ZERO, life * 1.3, size * 7.0, ramp, GLOW, {"grow": -0.3})
			if pos.y < 2.0:
				_emit(Vector3(pos.x, 0.04, pos.z), Vector3.ZERO, life * 2.5, size * 14.0, ramp, DISC, {"rot": 0.0})
			var rays := maxi(n, 4)
			for i in rays:
				var a := TAU * float(i) / float(rays) + _rng.randf_range(-0.2, 0.2)
				var d := Vector3(cos(a), 0.0, sin(a)) * 0.8 + Vector3.UP * sin(a) * 0.6
				_emit(pos, d.normalized() * speed, life * 1.6, size * 0.5, ramp, SPARK, {"drag": 0.6, "stretch": 0.03})
		"crack":
			_crack(pos, radius if radius > 0.0 else 1.1 * m, maxi(n, 3), m, life, ramp)
		"pillar":
			# A column of light rising from the ground, sparks streaming up inside it.
			var lift := 2.4
			_ribbon(COLUMN, [{"pts": PackedVector3Array([ground, ground + Vector3(0, lift * 0.5, 0), ground + Vector3(0, lift, 0)]),
				"w": PackedFloat32Array([0.45 * m, 0.4 * m, 0.3 * m])}], life * 1.6, ramp, 2.2)
			for i in n:
				var h := _rng.randf_range(0.0, 1.6)
				var jitter := Vector3(_rng.randf_range(-0.18, 0.18), 0.0, _rng.randf_range(-0.18, 0.18)) * m
				_emit(ground + jitter + Vector3(0, h, 0), Vector3(0, speed * _rng.randf_range(0.8, 1.6), 0), life * _rng.randf_range(0.8, 1.4),
					size * 0.5, ramp, SPARK, {"drag": 1.0})
			_emit(Vector3(pos.x, 0.04, pos.z), Vector3.ZERO, life * 1.8, 1.4 * m, ramp, DISC)
		"rocks":
			# Stones dropping from above onto points across the area, hard, bouncing as they land
			# and kicking up dust.
			var r := radius if radius > 0.0 else 1.2
			for i in n:
				var a := _rng.randf() * TAU
				var d := sqrt(_rng.randf()) * r * 0.9
				var spot := Vector3(pos.x + cos(a) * d, 0.0, pos.z + sin(a) * d)
				var h := _rng.randf_range(1.6, 3.2)
				var fall := h / speed
				_emit(spot + Vector3(0, h, 0), Vector3(0, -speed, 0), fall + 0.5, size * _rng.randf_range(0.8, 1.4), ramp, SHARD,
					{"drag": 1.0, "grav": 12.0, "spin": _rng.randf_range(-6, 6), "flags": BOUNCE | PUFF})
		"rain":
			# The volley keeps falling for `lifetime` seconds (`_spells`).
			_spells.append({"kit": "rain", "pos": ground, "radius": radius if radius > 0.0 else 1.5, "left": life, "spec": spec, "owed": 0.0})
		"vortex":
			# Power drawn in: streaks wound on a rising helix, tightening as they climb.
			var r := radius if radius > 0.0 else 0.9
			var turns := 2.0
			for i in n:
				var t := float(i) / float(maxi(n - 1, 1))
				var a := TAU * turns * t + _rng.randf_range(-0.15, 0.15)
				var out := Vector3(cos(a), 0.0, sin(a))
				var tangent := Vector3(-sin(a), 0.0, cos(a))
				var rr := r * (1.0 - 0.55 * t)
				var p := Vector3(pos.x, 0.1 + 1.6 * t, pos.z) + out * rr
				_emit(p, tangent * speed - out * speed * 0.35 + Vector3(0, 0.6, 0), life * (0.6 + 0.6 * t), size * (0.9 - 0.4 * t), ramp, SPARK,
					{"drag": 1.0, "stretch": 0.09})
			_emit(pos + Vector3(0, 0.9, 0), Vector3.ZERO, life * 1.2, r * 1.6, ramp, GLOW, {"grow": -0.7, "heat": 1.3})
		"embers":
			# Sparks lifting off and drifting, twinkling as they cool through the ramp.
			for i in n:
				var a := _rng.randf() * TAU
				var d := sqrt(_rng.randf()) * 0.6
				var p := Vector3(pos.x + cos(a) * d, _rng.randf_range(0.2, 1.1), pos.z + sin(a) * d)
				var v := Vector3(_rng.randf_range(-0.4, 0.4), speed * _rng.randf_range(0.5, 1.0), _rng.randf_range(-0.4, 0.4))
				_emit(p, v, life * _rng.randf_range(0.6, 1.0), size * _rng.randf_range(1.2, 2.0), ramp, GLOW,
					{"grav": -0.6, "grow": -0.5, "drag": 0.96, "flags": FLICKER, "heat": 2.2})
		"shards":
			# Crystals bursting out and up, then falling hard and skittering.
			for i in n:
				var d := _rand_dir(1.0)
				_emit(pos, d * speed * _rng.randf_range(0.6, 1.0), life * _rng.randf_range(1.0, 1.5), size * _rng.randf_range(1.0, 1.8), ramp, SHARD,
					{"grav": 14.0, "drag": 0.97, "spin": _rng.randf_range(-10, 10), "flags": BOUNCE, "heat": 1.25})
			_emit(pos, Vector3.ZERO, life * 0.35, size * 6.0, ramp, GLOW)
		"smoke":
			# Puffs that swell and rise slowly, hiding what is in them.
			var r := radius if radius > 0.0 else 0.8
			for i in n:
				var a := _rng.randf() * TAU
				var d := sqrt(_rng.randf()) * r * 0.7
				var p := Vector3(pos.x + cos(a) * d, _rng.randf_range(0.3, 1.2), pos.z + sin(a) * d)
				var v := Vector3(cos(a) * speed * 0.3, speed * 0.4, sin(a) * speed * 0.3)
				_emit(p, v, life * _rng.randf_range(0.8, 1.1), size * _rng.randf_range(1.8, 2.4), ramp, SMOKE, {"grav": -0.2, "grow": 1.2, "drag": 0.9})
		"motes":
			# Glimmers hovering about `pos`, drifting slowly, twinkling as they shrink.
			for i in n:
				var p := pos + Vector3(_rng.randf_range(-0.6, 0.6), _rng.randf_range(0.3, 1.6), _rng.randf_range(-0.6, 0.6))
				_emit(p, _rand_dir(0.3) * speed, life * _rng.randf_range(0.6, 1.0), size * 2.2, ramp, GLOW,
					{"grav": -0.1, "grow": -0.6, "drag": 0.97, "flags": FLICKER})
		"shock":
			# A shock front racing out from the center to `radius` (the gameplay radius) over its
			# life, easing as it goes, a flash of light where it started.
			var r := radius if radius > 0.0 else 1.2 * m
			_emit(Vector3(pos.x, 0.07, pos.z), Vector3.ZERO, life, r * 0.3, ramp, RING, {"grow": 2.0 / 0.3 - 1.0, "rot": _rng.randf() * TAU})
			_emit(Vector3(pos.x, 0.05, pos.z), Vector3.ZERO, life * 0.8, r * 1.2, ramp, DISC, {"grow": 0.4, "heat": 1.2})
		"fire":
			# Burns across the area for `lifetime` seconds (`_spells`), starting with a roar.
			var r := radius if radius > 0.0 else 0.8 * m
			_spells.append({"kit": "fire", "pos": ground, "radius": r, "left": life, "total": life, "spec": spec, "owed": float(n) * 2.5})
			_emit(Vector3(pos.x, 0.05, pos.z), Vector3.ZERO, life + 0.3, r * 2.4, ramp, DISC, {"heat": 1.2})
		"lightning":
			# `count` bolts striking across the area, the first at once, the rest over the next
			# half of `lifetime`.
			var r := radius if radius > 0.0 else 0.0
			for i in maxi(n, 1):
				var at := _t + (0.0 if i == 0 else _rng.randf_range(0.05, 0.5) * life)
				_strikes.append({"at": at, "pos": ground, "radius": r, "spec": spec, "m": m, "life": life})
		_:
			# Projectile styles and trails emit along a moving node; see `decorate`.
			pass


## An arc smear (14 §4.3): a band along the swing, revealed from its start to its end in a
## blink and eaten away from the tail as it fades, sparks thrown off its edge. `arc` (degrees) is
## its sweep and `tilt` its plane. Level swings (`tilt` under 45) sweep across the front,
## centered on `pos`; overhead swings come down from above the champion's shoulder and end on
## `pos`. `size` scales the reach. With no direction (novas) it's a full ring of `radius`.
func _slash(spec: Dictionary, pos: Vector3, dir: Vector3, radius: float, size: float, speed: float, life: float, ramp: PackedColorArray) -> void:
	var arc := deg_to_rad(float(spec.get("arc", 140.0)))
	var tilt := deg_to_rad(float(spec.get("tilt", 0.0)))
	var m := size / float(KITS.slash.size)
	var fwd := Vector3(dir.x, 0.0, dir.z)
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
	# An overhead blow's arc runs from the impact back over the shoulder; the swing comes the
	# other way.
	_ribbons.append({"style": SMEAR, "arc": {"center": center, "u": u, "b": b, "r": r, "a0": a0, "arc": arc, "ring": ring,
		"width": 0.34 * r, "rev": not ring and tilt >= PI / 4.0}, "lines": [], "age": 0.0, "life": maxf(life * 1.7, 0.16), "ramp": ramp, "heat": 2.0})
	# Sparks flung off the blade's edge.
	for i in 6:
		var t := _rng.randf()
		var a := a0 + arc * t
		var o := (u * cos(a) + b * sin(a)).normalized()
		var p := center + o * r
		p.y = maxf(p.y, 0.1)
		_emit(p, o * (speed + 2.0) * _rng.randf_range(0.6, 1.2), life * 2.0, size * 0.4, ramp, SPARK, {"drag": 0.8})


## The smear's band for the part of the arc showing at `f` (0–1 of its life).
func _smear_lines(s: Dictionary, f: float) -> Array:
	var head := minf(1.0, f / 0.3)
	var tail := clampf((f - 0.25) / 0.75, 0.0, 1.0) * 0.9
	if s.ring:
		head = 1.0
		tail = 0.0
	var steps := 24 if not s.ring else 40
	var a := PackedVector3Array()
	var b := PackedVector3Array()
	for i in steps + 1:
		var t := lerpf(tail, head, float(i) / float(steps))
		if s.rev:
			t = 1.0 - t
		var ang: float = s.a0 + s.arc * t
		var o: Vector3 = (s.u * cos(ang) + s.b * sin(ang)).normalized()
		# Thick in the middle of the swing, thin at its ends.
		var thick := 1.0 if s.ring else sin(PI * t)
		var outer: Vector3 = s.center + o * s.r
		var inner: Vector3 = s.center + o * (s.r - s.width * (0.25 + 0.75 * thick))
		outer.y = maxf(outer.y, 0.05)
		inner.y = maxf(inner.y, 0.05)
		a.append(inner)
		b.append(outer)
	return [{"a": a, "b": b}]


## Jagged cracks radiating over the ground from `pos` to `r`, spreading out in a blink and
## lingering; a bright ramp glows in them.
func _crack(pos: Vector3, r: float, lines: int, m: float, life: float, ramp: PackedColorArray) -> void:
	var out := []
	for i in lines:
		var a := TAU * float(i) / float(lines) + _rng.randf_range(-0.35, 0.35)
		var d := Vector3(cos(a), 0.0, sin(a))
		var side := Vector3(-d.z, 0.0, d.x)
		var steps := clampi(int(r / 0.12), 4, 14)
		var wob := 0.0
		var pts := PackedVector3Array([Vector3(pos.x, 0.03, pos.z)])
		var w := PackedFloat32Array([0.1 * m])
		for k in steps:
			var t := float(k + 1) / float(steps)
			wob = clampf(wob + _rng.randf_range(-0.09, 0.09), -0.2, 0.2)
			pts.append(Vector3(pos.x, 0.03, pos.z) + d * r * t + side * wob * r * t)
			w.append(0.1 * m * (1.0 - 0.75 * t))
		out.append(_flatten(pts, w))
	_ribbons.append({"style": CRACK, "lines": out, "age": 0.0, "life": life, "ramp": ramp, "heat": 1.6, "spread": 0.15})


## A ground-flat ribbon: the spine `pts` widened by `w` either side, on the ground's plane.
func _flatten(pts: PackedVector3Array, w: PackedFloat32Array) -> Dictionary:
	var a := PackedVector3Array()
	var b := PackedVector3Array()
	for k in pts.size():
		var t: Vector3 = pts[mini(k + 1, pts.size() - 1)] - pts[maxi(k - 1, 0)]
		var side := Vector3(-t.z, 0.0, t.x).normalized()
		a.append(pts[k] + side * w[k])
		b.append(pts[k] - side * w[k])
	return {"a": a, "b": b}


func _ribbon(style: int, lines: Array, life: float, ramp: PackedColorArray, heat := 1.8) -> Dictionary:
	var rb := {"style": style, "lines": lines, "age": 0.0, "life": maxf(life, 0.05), "ramp": ramp, "heat": heat}
	if _ribbons.size() < MAX_RIBBONS:
		_ribbons.append(rb)
	return rb


## A bolt of lightning from the sky onto the ground near `ground`: a forked, flickering ribbon,
## re-drawn every few hundredths of a second, and the strike's flash, scorch and sparks.
func _strike(s: Dictionary) -> void:
	var spec: Dictionary = s.spec
	var ramp: PackedColorArray = spec.ramp
	var m: float = s.m
	var a := _rng.randf() * TAU
	var d := sqrt(_rng.randf()) * float(s.radius) * 0.85
	var hit: Vector3 = s.pos + Vector3(cos(a) * d, 0.02, sin(a) * d)
	var top := hit + Vector3(_rng.randf_range(-1.5, 1.5), 7.0, _rng.randf_range(-1.5, 1.5))
	var life: float = clampf(float(s.life) * 0.7, 0.15, 0.6)
	_ribbons.append({"style": LIGHTNING, "bolt": {"from": top, "to": hit, "w": 0.09 * m}, "lines": [], "age": 0.0, "life": life,
		"ramp": ramp, "heat": 2.6, "regen": 0.0, "flick": 1.0})
	_emit(hit + Vector3(0, 0.3, 0), Vector3.ZERO, 0.18, 2.2 * m, ramp, STAR, {"grow": 0.6, "heat": 3.0})
	_emit(hit + Vector3(0, 0.4, 0), Vector3.ZERO, 0.2, 1.6 * m, ramp, GLOW, {"grow": -0.3})
	_emit(Vector3(hit.x, 0.04, hit.z), Vector3.ZERO, 0.45, 2.4 * m, ramp, DISC, {"heat": 1.4})
	_emit(Vector3(hit.x, 0.07, hit.z), Vector3.ZERO, 0.35, 0.4 * m, ramp, RING, {"grow": 5.0, "heat": 1.4})
	# The scorch it leaves.
	_emit(Vector3(hit.x, 0.035, hit.z), Vector3.ZERO, 1.6, 1.3 * m, PackedColorArray([Color(0.08, 0.07, 0.07, 0.75), Color(0.1, 0.09, 0.09, 0.6)]), DISC,
		{"add": 0.0, "heat": 1.0})
	for i in 12:
		_emit(hit + Vector3(0, 0.1, 0), _rand_dir(0.8) * _rng.randf_range(2.5, 6.0), _rng.randf_range(0.25, 0.5), 0.05 * m, ramp, SPARK,
			{"grav": 9.0, "drag": 0.92, "flags": BOUNCE})
	var soot := PackedColorArray([Color(0.30, 0.30, 0.34, 0.7), Color(0.18, 0.18, 0.2, 0.0)])
	for i in 3:
		_emit(hit + Vector3(_rng.randf_range(-0.2, 0.2), 0.3, _rng.randf_range(-0.2, 0.2)), Vector3(0, 0.5, 0), 0.9, 0.5 * m, soot, SMOKE, {"grow": 1.2, "grav": -0.2})


## A jagged path from `a` to `b`: midpoint displacement, `depth` times halved.
func _jag(a: Vector3, b: Vector3, rough: float, depth: int) -> PackedVector3Array:
	var pts := PackedVector3Array([a, b])
	var amp := a.distance_to(b) * rough
	for level in depth:
		var next := PackedVector3Array()
		for k in pts.size() - 1:
			var p: Vector3 = pts[k]
			var q: Vector3 = pts[k + 1]
			next.append(p)
			var mid := (p + q) * 0.5
			var t := (q - p).normalized()
			var off := _rand_dir()
			off = (off - t * off.dot(t)).normalized() * _rng.randf_range(-amp, amp)
			next.append(mid + off)
		next.append(pts[pts.size() - 1])
		pts = next
		amp *= 0.5
	return pts


func _bolt_lines(bolt: Dictionary) -> Array:
	var main := _jag(bolt.from, bolt.to, 0.16, 5)
	var w := PackedFloat32Array()
	for k in main.size():
		w.append(float(bolt.w) * (0.55 + 0.45 * float(k) / float(main.size() - 1)))
	var lines := [{"pts": main, "w": w}]
	# Forks off the upper two thirds, thinner, reaching out and down.
	for f in 3:
		var k := _rng.randi_range(4, int(main.size() * 0.66))
		var from: Vector3 = main[k]
		var to := from + Vector3(_rng.randf_range(-1.6, 1.6), -_rng.randf_range(0.8, 2.2), _rng.randf_range(-1.6, 1.6))
		var fork := _jag(from, to, 0.22, 3)
		var fw := PackedFloat32Array()
		for j in fork.size():
			fw.append(float(bolt.w) * 0.5 * (1.0 - float(j) / float(fork.size())))
		lines.append({"pts": fork, "w": fw})
	return lines


## The grapple hand's size (m) for a projectile of hitbox `radius` (m).
func grab_size(radius: float) -> float:
	return minf(radius * 1.6, 0.42)


## A chipped block: a box of `size` at `at`, turned by `rot` (radians), under `parent`.
func _lump(parent: Node3D, size: Vector3, at: Vector3, rot: Vector3, mat: Material) -> MeshInstance3D:
	var m := MeshInstance3D.new()
	var b := BoxMesh.new()
	b.size = size
	m.mesh = b
	m.material_override = mat
	m.position = at
	m.rotation = rot
	parent.add_child(m)
	return m


## The grapple's stretched arm from the caster's wrist to the flying hand: a core in the
## ramp's middle color with chipped links of the hand's stone strung along it, every `GAP` m.
## Made once (`grab_arm`), restrung each frame (`string_arm`) between two world points.
const ARM_GAP := 0.19

func grab_arm(spec: Dictionary, s: float) -> Node3D:
	var ramp: PackedColorArray = spec.ramp
	var arm := Node3D.new()
	arm.name = "Arm"
	arm.top_level = true
	var core_mat := StandardMaterial3D.new()
	core_mat.albedo_color = ramp[1] if ramp.size() > 1 else ramp[0]
	core_mat.roughness = 0.35
	var core := MeshInstance3D.new()
	core.name = "Core"
	var cm := CylinderMesh.new()
	cm.top_radius = s * 0.11
	cm.bottom_radius = s * 0.11
	cm.height = 1.0
	cm.radial_segments = 6
	cm.rings = 1
	core.mesh = cm
	core.material_override = core_mat
	arm.add_child(core)
	var stone := StandardMaterial3D.new()
	stone.albedo_color = ramp[ramp.size() - 1] if ramp.size() > 2 else ramp[0]
	stone.roughness = 0.95
	arm.set_meta("stone", stone)
	arm.set_meta("s", s)
	return arm


func string_arm(arm: Node3D, a: Vector3, b: Vector3, t: float) -> void:
	var span := b - a
	var length := span.length()
	var s: float = arm.get_meta("s")
	var core: MeshInstance3D = arm.get_node("Core")
	core.visible = length > 0.02
	if not core.visible:
		return
	var dir := span / length
	# A basis whose Y runs along the span (the cylinder's axis).
	var side := dir.cross(Vector3.UP)
	if side.length() < 0.01:
		side = Vector3.RIGHT
	side = side.normalized()
	var along := Basis(side, dir, side.cross(dir))
	core.global_transform = Transform3D(along.scaled_local(Vector3(1, length, 1)), (a + b) * 0.5)
	var n := int(length / ARM_GAP)
	var links: Array = arm.get_meta("links", [])
	while links.size() < n:
		var k := links.size()
		# Each link its own chipped stone, with a smaller one wedged beside it: sizes, shades and
		# turns from its place in the arm, so no two read alike.
		var h := float((k * 37) % 11) / 11.0
		var g := float((k * 53 + 5) % 7) / 7.0
		var base: Material = arm.get_meta("stone")
		var mat: StandardMaterial3D = base.duplicate()
		mat.albedo_color = base.albedo_color.darkened(0.25 * g) if k % 2 == 0 else base.albedo_color.lightened(0.12 * g)
		var m := _lump(arm, Vector3(s * (0.36 + 0.18 * h), s * (0.30 + 0.14 * (1.0 - h)), s * (0.34 + 0.14 * g)),
			Vector3.ZERO, Vector3.ZERO, mat)
		var chip := _lump(m, Vector3(s * 0.22, s * 0.18, s * 0.20), Vector3(s * (0.16 - 0.3 * g), s * 0.10, s * (0.12 - 0.24 * h)),
			Vector3(0.5 + h, 0.7 * g, 0.4), mat)
		chip.name = "Chip"
		m.set_meta("turn", Vector3(0.9 * h - 0.45, float(k) * 1.7, 0.5 - 1.0 * g))
		links.append(m)
	arm.set_meta("links", links)
	for k in links.size():
		var l: MeshInstance3D = links[k]
		l.visible = k < n
		if not l.visible:
			continue
		# Spaced evenly from the wrist, a slight sway along the arm as it flies.
		var f := (float(k) + 0.5) / float(n)
		var sway := side * sin(f * PI) * sin(t * 9.0 + f * 6.0) * s * 0.10
		l.global_transform = Transform3D(along * Basis.from_euler(l.get_meta("turn")), a + span * f + sway)


## A projectile's style: decorations inside its gameplay-sized body (a shaft, a spinning frame,
## the grapple's hand, a laser along a `beam`), plus what it sheds behind it each frame (glow,
## sparks, licks of flame). `radius` (m) is the projectile's hitbox radius: nothing drawn here
## is wider than it or ahead of it (05 §1.1).
func decorate(node: Node3D, spec: Dictionary, radius: float, lift := 0.35) -> void:
	var kit: String = spec.kit
	var ramp: PackedColorArray = spec.ramp
	var mat := StandardMaterial3D.new()
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.albedo_color = ramp[0]
	match kit:
		"grab":
			# The caster's own hand flying out on the end of the stretched arm (14 §4.4: a grapple
			# reads as an arm): chipped blocks for a palm, fingers reaching forward and a thumb, lit
			# like the world, the arm's core showing at the wrist. The caller hides the caster's real
			# hand and strings the arm back to the wrist (`grab_arm`).
			var hand := Node3D.new()
			hand.name = "Grab"
			hand.position = Vector3(0, lift, 0)
			var s := grab_size(radius)
			var stone := StandardMaterial3D.new()
			stone.albedo_color = ramp[ramp.size() - 1] if ramp.size() > 2 else ramp[0]
			stone.roughness = 0.95
			var core := StandardMaterial3D.new()
			core.albedo_color = ramp[1] if ramp.size() > 1 else ramp[0]
			core.roughness = 0.35
			_lump(hand, Vector3(s * 0.62, s * 0.34, s * 0.66), Vector3.ZERO, Vector3(0.10, 0.0, 0.06), stone)
			_lump(hand, Vector3(s * 0.36, s * 0.26, s * 0.40), Vector3(-s * 0.06, s * 0.16, 0.0), Vector3(0.0, 0.5, 0.2), stone)
			for k in 4:
				var z := (float(k) - 1.5) * s * 0.17
				var spread := (float(k) - 1.5) * 0.22
				var root := Vector3(s * 0.40, 0.0, z)
				_lump(hand, Vector3(s * 0.26, s * 0.17, s * 0.15), root, Vector3(0.05 * k, -spread, -0.15), stone)
				# The tip curls down a little, reaching to close.
				var tip := root + Vector3(cos(spread) * s * 0.24, -s * 0.05, -sin(spread) * s * 0.24)
				_lump(hand, Vector3(s * 0.22, s * 0.14, s * 0.13), tip, Vector3(-0.04 * k, -spread, -0.55), stone)
			_lump(hand, Vector3(s * 0.30, s * 0.17, s * 0.17), Vector3(s * 0.16, -s * 0.02, s * 0.42), Vector3(0.2, -0.9, 0.0), stone)
			# The arm's core where the wrist was.
			var wrist := MeshInstance3D.new()
			wrist.name = "Wrist"
			var wm := CylinderMesh.new()
			wm.top_radius = s * 0.16
			wm.bottom_radius = s * 0.16
			wm.height = s * 0.4
			wm.radial_segments = 6
			wm.rings = 1
			wrist.mesh = wm
			wrist.material_override = core
			wrist.position = Vector3(-s * 0.40, 0.0, 0.0)
			wrist.rotation = Vector3(0, 0, PI * 0.5)
			hand.add_child(wrist)
			node.add_child(hand)
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
		"beam":
			# A laser down the beam's length, from where it was first seen to its head, inside the
			# body's width.
			_ribbons.append({"style": LASER, "node": node, "lift": lift, "w": minf(radius * 0.5, 0.24), "age": 0.0, "life": 1.0,
				"ramp": ramp, "heat": 2.4, "lines": []})
		"orb", "lob":
			pass
	_trails.append({ "node": node, "spec": spec, "kit": kit, "lift": lift, "radius": radius, "owed": 0.0 })


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



## The busiest moment since the last call: live particles, ribbons and the most a frame's
## simulation and upload took (ms). The effects gallery prints it.
func peak() -> Dictionary:
	var p := _peak
	_peak = {"particles": 0, "ribbons": 0, "ms": 0.0}
	return p


var _peak := {"particles": 0, "ribbons": 0, "ms": 0.0}


func _process(delta: float) -> void:
	var t0 := Time.get_ticks_usec()
	_t += delta
	if sun == null and get_parent() != null:
		var s = get_parent().get("sun")
		if s is DirectionalLight3D:
			sun = s
	if sun != null:
		for m in _mats:
			m.set_shader_parameter("sun_dir", sun.global_basis.z)
	# Bombs fly smoothly (they're objects, not particles).
	for l in _lobs:
		l.age += delta
		var t := clampf(l.age / l.flight, 0.0, 1.0)
		var p: Vector3 = l.from.lerp(l.to, t)
		p.y += sin(t * PI) * 1.4
		l.node.position = p
		l.node.rotation += Vector3(7.0, 5.0, 0.0) * delta * (1.0 - t)
	var dt := minf(delta, 0.05)
	_step(dt)
	_upload()
	_draw_ribbons(dt)
	_peak.particles = maxi(_peak.particles, _sim.count())
	_peak.ribbons = maxi(_peak.ribbons, _ribbons.size())
	_peak.ms = maxf(_peak.ms, float(Time.get_ticks_usec() - t0) / 1000.0)


## A dash's `travel` effect: `spec` streams off `node` each frame while `node`'s `was_dashing`
## meta stays set (the caller sets it while the unit dashes).
func follow(node: Node3D, spec: Dictionary, lift := 0.9) -> void:
	_trails.append({"node": node, "spec": spec, "kit": spec.kit, "lift": lift, "dash": true, "radius": 0.5, "owed": 0.0})


const DUST := [Color(0.62, 0.58, 0.50, 0.55), Color(0.50, 0.47, 0.42, 0.3), Color(0.42, 0.40, 0.37, 0.0)]


func _step(dt: float) -> void:
	_step_trails(dt)
	for l in _lobs:
		var spec: Dictionary = l.spec
		if _rng.randf() < dt * 30.0:
			_emit(l.node.position + Vector3(0, 0.12, 0), _rand_dir(1.0) * 1.2, 0.25, 0.04, spec.ramp, SPARK, {"grav": 3.0})
	for sp in _spells.duplicate():
		sp.left -= dt
		if sp.left <= 0.0:
			_spells.erase(sp)
			continue
		if sp.kit == "rain":
			_rain(sp, dt)
		else:
			_fire(sp, dt)
	for s in _strikes.duplicate():
		if s.at <= _t:
			_strikes.erase(s)
			_strike(s)
	_sim.step(dt)


## Trails: what every decorated projectile (and dashing champion) sheds, each frame.
func _step_trails(dt: float) -> void:
	for tr in _trails.duplicate():
		var node = tr.node
		if not is_instance_valid(node) or not node.is_inside_tree() or (tr.has("dash") and not node.get_meta("was_dashing", false)):
			_trails.erase(tr)
			continue
		if not node.visible:
			continue
		var spec: Dictionary = tr.spec
		var kit: String = tr.kit
		var ramp: PackedColorArray = spec.ramp
		var frame: Node3D = node.get_node_or_null("NetFrame")
		if frame != null:
			frame.rotation.y += dt * 9.0
		var head: Vector3 = node.global_position + Vector3(0, tr.lift, 0)
		var back: Vector3 = -node.global_basis.x.normalized() if not tr.has("dash") else Vector3.ZERO
		var r: float = tr.radius
		var size := _knob(spec, kit, "size")
		var speed := _knob(spec, kit, "speed")
		var life := _knob(spec, kit, "lifetime")
		# A soft glow riding the head, never wider than the body: its short life smears it into
		# a comet's tail behind.
		if not tr.has("dash") and kit not in ["arrow", "grab", "lob"]:
			_emit(head, Vector3.ZERO, 0.08, minf(r * 1.5, 0.55), ramp, GLOW, {"heat": 1.5, "soft": 0.0})
		tr.owed += _knob(spec, kit, "count") * 20.0 * dt
		while tr.owed >= 1.0:
			tr.owed -= 1.0
			if tr.has("dash"):
				var p := head + _rand_dir() * 0.35
				if kit in ["dust", "smoke"]:
					_emit(Vector3(p.x, 0.2, p.z), _rand_dir(0.3) * speed, life * 2.0, 0.4, ramp, SMOKE, {"grow": 1.0, "drag": 0.85})
				else:
					_emit(p, _rand_dir() * speed, life * 1.6, size * 0.6, ramp, SPARK, {"drag": 0.85})
					_emit(p, Vector3.ZERO, life, size * 3.0, ramp, GLOW, {"grow": -0.5})
				continue
			var jitter := _rand_dir() * r * 0.35
			match kit:
				"flame":
					# Licks of fire peeling off the lance, rising as they fall behind.
					_emit(head + jitter + back * r * 0.3, back * speed + Vector3(0, 0.7, 0), life * _rng.randf_range(1.6, 2.4),
						minf(size * 6.5, r * 2.0) * _rng.randf_range(0.8, 1.15), ramp, FLAME, {"grav": -1.2, "grow": -0.45, "drag": 0.9})
				"shard", "bones":
					_emit(head + jitter, _rand_dir() * speed + back * 0.5, life * 1.5, size * 1.2, ramp, SHARD,
						{"grav": 6.0, "spin": _rng.randf_range(-10, 10), "heat": 1.2})
				"wave":
					_emit(Vector3(head.x, 0.2, head.z) + jitter, _rand_dir(0.6) * speed, life * 2.0, size * 3.0, PackedColorArray(DUST), SMOKE,
						{"grow": 1.0, "drag": 0.85})
					_emit(head + jitter, _rand_dir(0.5) * speed * 2.0, life * 1.4, size * 0.5, ramp, SPARK, {"grav": 6.0})
				_:
					_emit(head + jitter, _rand_dir() * speed + back * speed, life * 1.4, size * 0.5, ramp, SPARK, {"drag": 0.85})


## A volley falling across the area: each arrow a streak that dies where it lands, kicking up a
## little dust.
func _rain(sp: Dictionary, dt: float) -> void:
	var spec: Dictionary = sp.spec
	var speed := _knob(spec, "rain", "speed")
	var size := _knob(spec, "rain", "size")
	sp.owed += _knob(spec, "rain", "count") * 20.0 * dt
	while sp.owed >= 1.0:
		sp.owed -= 1.0
		var a := _rng.randf() * TAU
		var d := sqrt(_rng.randf()) * float(sp.radius)
		var spot: Vector3 = sp.pos + Vector3(cos(a) * d, 0.0, sin(a) * d)
		var h := _rng.randf_range(2.0, 3.5)
		var slant := Vector3(0.25, 0.0, 0.15) * speed * 0.3
		var t := h / speed
		_emit(spot + Vector3(0, h, 0) - slant * t, Vector3(slant.x, -speed, slant.z), t + 0.1, size * 0.8, spec.ramp, SPARK,
			{"drag": 1.0, "stretch": 0.03, "flags": SPLAT | (PUFF if _rng.randf() < 0.3 else 0), "heat": 1.6})


## A fire burning across the area: flames rising off the ground and shrinking as they climb,
## embers flying from them, smoke over them; fiercest at first, dying down at the end.
func _fire(sp: Dictionary, dt: float) -> void:
	var spec: Dictionary = sp.spec
	var ramp: PackedColorArray = spec.ramp
	var size := _knob(spec, "fire", "size")
	var speed := _knob(spec, "fire", "speed")
	var r: float = sp.radius
	var left := float(sp.left) / maxf(float(sp.total), 0.01)
	var strength := clampf(left / 0.3, 0.25, 1.0)
	sp.owed += _knob(spec, "fire", "count") * 10.0 * dt * strength * maxf(1.0, r / 0.8)
	var soot := PackedColorArray([Color(0.42, 0.39, 0.37, 0.0), Color(0.36, 0.33, 0.31, 0.45), Color(0.45, 0.43, 0.41, 0.0)])
	while sp.owed >= 1.0:
		sp.owed -= 1.0
		var a := _rng.randf() * TAU
		var d := sqrt(_rng.randf()) * r * 0.85
		var p: Vector3 = sp.pos + Vector3(cos(a) * d, 0.12, sin(a) * d)
		# Taller in the middle of the fire.
		var mid := 1.0 - 0.5 * d / maxf(r, 0.01)
		_emit(p, Vector3(_rng.randf_range(-0.2, 0.2), speed * _rng.randf_range(0.7, 1.3) * mid, _rng.randf_range(-0.2, 0.2)),
			_rng.randf_range(0.45, 0.8), size * _rng.randf_range(0.7, 1.3) * (0.6 + 0.4 * strength), ramp, FLAME,
			{"grav": -1.5, "grow": -0.45, "drag": 0.93, "rot": 0.0})
		if _rng.randf() < 0.3:
			_emit(p + Vector3(0, 0.3, 0), Vector3(_rng.randf_range(-0.8, 0.8), speed * _rng.randf_range(1.2, 2.2), _rng.randf_range(-0.8, 0.8)),
				_rng.randf_range(0.6, 1.1), 0.07, ramp, GLOW, {"grav": -0.5, "drag": 0.95, "flags": FLICKER, "heat": 2.4, "grow": -0.6})
		if _rng.randf() < 0.18:
			_emit(p + Vector3(0, 1.4 + 0.6 * mid, 0), Vector3(0, 1.0, 0), _rng.randf_range(0.9, 1.4), size * 1.8, soot, SMOKE,
				{"grow": 1.3, "grav": -0.3, "drag": 0.92})


## The ramp's color at `f` (0–1 of a life), blended between its colors.
func _ramp_at(ramp: PackedColorArray, f: float) -> Color:
	var n := ramp.size()
	if n == 1:
		return ramp[0]
	var x := clampf(f, 0.0, 1.0) * float(n - 1)
	var k := mini(int(x), n - 2)
	return ramp[k].lerp(ramp[k + 1], x - float(k))


## Hands both layers' buffers, packed by the simulation, to their MultiMeshes.
func _upload() -> void:
	var n: PackedInt32Array = _sim.pack(_t)
	if n[0] > 0 or _mm_cover.visible_instance_count > 0:
		_mm_cover.buffer = _sim.cover_buffer()
		_mm_cover.visible_instance_count = n[0]
	if n[1] > 0 or _mm_glow.visible_instance_count > 0:
		_mm_glow.buffer = _sim.glow_buffer()
		_mm_glow.visible_instance_count = n[1]


## Rebuilds the ribbons' strips for this frame.
func _draw_ribbons(dt: float) -> void:
	_im.clear_surfaces()
	var draw := []
	for rb in _ribbons.duplicate():
		var lines: Array = rb.lines
		var life_a := 1.0
		var f := 0.0
		if rb.has("node"):
			var node = rb.node
			if not rb.get("dying", false) and is_instance_valid(node) and node.is_inside_tree() and node.visible:
				var head: Vector3 = node.global_position + Vector3(0, rb.lift, 0)
				if not rb.has("start"):
					rb.start = head
				rb.head = head
			else:
				rb.dying = true
				rb.age += dt
				if rb.age >= 0.15 or not rb.has("head"):
					_ribbons.erase(rb)
					continue
				life_a = 1.0 - rb.age / 0.15
			if not rb.has("head"):
				continue
			var tail: Vector3 = rb.start
			var span: Vector3 = rb.head - tail
			if span.length() > 7.0:
				tail = rb.head - span.normalized() * 7.0
				span = rb.head - tail
			if span.length() < 0.05:
				continue
			var pts := PackedVector3Array()
			var w := PackedFloat32Array()
			for k in 9:
				var t := float(k) / 8.0
				pts.append(tail + span * t)
				w.append(float(rb.w) * (0.25 + 0.75 * sqrt(t)))
			lines = [{"pts": pts, "w": w}]
			f = 0.3
		else:
			rb.age += dt
			if rb.age >= rb.life:
				_ribbons.erase(rb)
				continue
			f = rb.age / rb.life
			match int(rb.style):
				LIGHTNING:
					rb.regen -= dt
					if rb.regen <= 0.0:
						rb.regen = 0.045
						rb.lines = _bolt_lines(rb.bolt)
						rb.flick = _rng.randf_range(0.45, 1.0)
					lines = rb.lines
					life_a = (1.0 if rb.age < 0.05 else rb.flick) * pow(1.0 - f, 0.7)
					f *= 0.4
				SMEAR:
					lines = _smear_lines(rb.arc, f)
					life_a = 1.0 - f
				CRACK:
					var g := minf(1.0, f / float(rb.spread))
					var cut := []
					for l in lines:
						var n := maxi(2, int(ceil(g * l.a.size())))
						cut.append({"a": l.a.slice(0, n), "b": l.b.slice(0, n)})
					lines = cut
					life_a = 1.0 - smoothstep(0.6, 1.0, f)
				COLUMN:
					var g := minf(1.0, f / 0.12)
					var risen := []
					for l in lines:
						var pts := PackedVector3Array()
						for p in l.pts:
							pts.append(Vector3(p.x, p.y * g, p.z))
						risen.append({"pts": pts, "w": l.w})
					lines = risen
					life_a = pow(1.0 - f, 1.3)
		if lines.is_empty():
			continue
		var ramp: PackedColorArray = rb.ramp
		var c := (_ramp_at(ramp, f) if rb.style != LASER else (ramp[1] if ramp.size() > 1 else ramp[0])).srgb_to_linear()
		c.a = clampf(life_a, 0.0, 1.0)
		var glow: float = 1.0 + (float(rb.heat) - 1.0) * (1.0 - f)
		draw.append([rb.style, lines, c, glow])
	if draw.is_empty():
		return
	_im.surface_begin(Mesh.PRIMITIVE_TRIANGLES, _ribbon_mat)
	for d in draw:
		for l in d[1]:
			if l.has("pts"):
				_strip(l.pts, l.w, d[0], d[2], d[3])
			else:
				_flat_strip(l.a, l.b, d[0], d[2], d[3])
	_im.surface_end()


func _vtx(p: Vector3, n: Vector3, along: float, across: float, style: int, w: float, glow: float, c: Color) -> void:
	_im.surface_set_normal(n)
	_im.surface_set_color(c)
	_im.surface_set_uv(Vector2(along, float(style) * 2.0 + across))
	_im.surface_set_uv2(Vector2(w, glow))
	_im.surface_add_vertex(p)


## A camera-facing strip along `pts`, `w` wide either side (the shader turns it to the camera).
func _strip(pts: PackedVector3Array, w: PackedFloat32Array, style: int, c: Color, glow: float) -> void:
	var n := pts.size()
	if n < 2:
		return
	var along := PackedFloat32Array([0.0])
	for k in range(1, n):
		along.append(along[k - 1] + pts[k].distance_to(pts[k - 1]))
	var tan := PackedVector3Array()
	for k in n:
		var t: Vector3 = pts[mini(k + 1, n - 1)] - pts[maxi(k - 1, 0)]
		tan.append(t.normalized() if t.length() > 0.0001 else Vector3.UP)
	for k in n - 1:
		var j := k + 1
		_vtx(pts[k], tan[k], along[k], 0.0, style, w[k], glow, c)
		_vtx(pts[k], tan[k], along[k], 1.0, style, w[k], glow, c)
		_vtx(pts[j], tan[j], along[j], 0.0, style, w[j], glow, c)
		_vtx(pts[k], tan[k], along[k], 1.0, style, w[k], glow, c)
		_vtx(pts[j], tan[j], along[j], 1.0, style, w[j], glow, c)
		_vtx(pts[j], tan[j], along[j], 0.0, style, w[j], glow, c)


## A strip with both edges given (on the ground, or in a swing's plane): `a` across 0, `b` 1.
func _flat_strip(a: PackedVector3Array, b: PackedVector3Array, style: int, c: Color, glow: float) -> void:
	var n := mini(a.size(), b.size())
	if n < 2:
		return
	var along := PackedFloat32Array([0.0])
	for k in range(1, n):
		along.append(along[k - 1] + a[k].distance_to(a[k - 1]))
	for k in n - 1:
		var j := k + 1
		_vtx(a[k], Vector3.UP, along[k], 0.0, style, -1.0, glow, c)
		_vtx(b[k], Vector3.UP, along[k], 1.0, style, -1.0, glow, c)
		_vtx(a[j], Vector3.UP, along[j], 0.0, style, -1.0, glow, c)
		_vtx(b[k], Vector3.UP, along[k], 1.0, style, -1.0, glow, c)
		_vtx(b[j], Vector3.UP, along[j], 1.0, style, -1.0, glow, c)
		_vtx(a[j], Vector3.UP, along[j], 0.0, style, -1.0, glow, c)
