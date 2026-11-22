extends Node3D
## Weather (D82, an evaluation): what the sky does over the season's map. Clear is the season as
## it is; rain is an overcast day, the ground soaked and puddled, rain falling around the camera
## and its hiss; night is moonlight, the braziers and crystals carrying the light, and a faint
## light around your own champion so the fight stays readable; a storm is night and rain with
## lightning and thunder. Everything is made at runtime: no textures or sounds ship.
##
## `dress` lays the weather over a season's light (`Seasons.apply_light` takes the result), and
## `apply` sets the rest: the haze, the shadows, the moon, the shader globals (`wetness`, `rain`),
## the rain, the lamps' lights and the lightning. `update` runs each frame.

const NAMES := ["clear", "rain", "night", "storm"]

## Over the season's light: the keys `Seasons.apply_light` reads (sun, sky fill, grade, haze,
## clouds), and the weather's own (`fx`).
const WEATHERS := {
	"clear": {},
	"rain": {
		"sun_color": Color(0.80, 0.85, 0.92), "sun_energy": 0.55,
		"sky_fill": Color(0.46, 0.52, 0.61), "sky_fill_energy": 0.78,
		"exposure": 1.06, "contrast": 1.02, "saturation": 0.84,
		"haze": Color(0.44, 0.48, 0.54), "clouds": 0.95, "motes": "",
		"fx": {"cloud_strength": 0.3, "shadow_opacity": 0.5, "fog": [14.0, 46.0, 0.55], "wet": 1.0,
			"rain": 1.0, "rain_color": Color(0.8, 0.84, 0.92, 0.16), "mirror": Color(0.055, 0.062, 0.078),
			"background": Color(0.07, 0.08, 0.09)},
	},
	"night": {
		"sun_color": Color(0.52, 0.64, 1.0), "sun_energy": 0.68,
		"sky_fill": Color(0.11, 0.16, 0.32), "sky_fill_energy": 0.78,
		"exposure": 1.3, "contrast": 1.06, "saturation": 0.9,
		"haze": Color(0.07, 0.09, 0.16), "clouds": 0.4,
		"fx": {"moon": true, "shadow_opacity": 0.8, "fog": [20.0, 56.0, 0.5], "lamps": [1.3, 1.35],
			"carry": true, "background": Color(0.02, 0.025, 0.04)},
	},
	"storm": {
		"sun_color": Color(0.50, 0.60, 0.92), "sun_energy": 0.42,
		"sky_fill": Color(0.10, 0.13, 0.24), "sky_fill_energy": 0.72,
		"exposure": 1.35, "contrast": 1.04, "saturation": 0.82,
		"haze": Color(0.06, 0.07, 0.11), "clouds": 1.0, "motes": "",
		"fx": {"moon": true, "cloud_strength": 0.35, "shadow_opacity": 0.55, "fog": [16.0, 48.0, 0.55],
			"wet": 1.0, "rain": 1.0, "rain_color": Color(0.62, 0.68, 0.8, 0.13), "mirror": Color(0.012, 0.015, 0.026),
			"lamps": [1.3, 1.35],
			"carry": true, "lightning": true, "background": Color(0.015, 0.02, 0.03)},
	},
}

const SUN_ROTATION := Vector3(-52, -38, 0)   # main.gd's sun
const MOON_ROTATION := Vector3(-62, 28, 0)   # higher, from the other side: shadows fall another way
const LAMP_LIGHTS := 6                       # real lights at the nearest lamps after dark (Mobile: 8 a mesh)

var current := "clear"
var strike_every := 0.0                      # `--strikes S`: lightning every S seconds (captures), else at random
var _fx := {}
var _env: Environment
var _sun: DirectionalLight3D
var _ambient := 0.6                          # the fill's energy as dressed, which lightning lifts
var _background := Color()
var _rain: GPUParticles3D
var _splashes: GPUParticles3D
var _rain_sound: AudioStreamPlayer
var _lamp_lights: Array[OmniLight3D] = []
var _carry: OmniLight3D
var _flash: DirectionalLight3D
var _next_strike := 0.0
var _strike_t := -1.0
var _thunder_in := -1.0
var _thunder_gain := 0.0
var _thunder: Array[AudioStreamWAV] = []
var _thunder_player: AudioStreamPlayer
var _rng := RandomNumberGenerator.new()
var _suspended := []                         # the sun as it was, while the minimap renders in daylight


func setup(env: Environment, sun: DirectionalLight3D) -> void:
	_env = env
	_sun = sun
	_rng.randomize()


## The season under the weather: the season's dictionary with the weather's light laid over it.
func dress(season: Dictionary) -> Dictionary:
	var w: Dictionary = WEATHERS.get(current, {})
	var out := season.duplicate()
	for k in w:
		if k != "fx":
			out[k] = w[k]
	return out


## After `Seasons.apply_light` took the dressed season: everything else the weather sets.
func apply(map_view: Node) -> void:
	_fx = WEATHERS.get(current, {}).get("fx", {})
	_sun.rotation_degrees = MOON_ROTATION if _fx.get("moon", false) else SUN_ROTATION
	_sun.shadow_opacity = _fx.get("shadow_opacity", 0.9)
	var fog: Array = _fx.get("fog", [24.0, 60.0, 0.35])
	_env.fog_depth_begin = fog[0]
	_env.fog_depth_end = fog[1]
	_env.fog_density = fog[2]
	_background = _fx.get("background", Color(0.08, 0.09, 0.1))
	_env.background_color = _background
	_ambient = _env.ambient_light_energy
	RenderingServer.global_shader_parameter_set("wetness", _fx.get("wet", 0.0))
	RenderingServer.global_shader_parameter_set("rain", _fx.get("rain", 0.0))
	var mirror: Color = _fx.get("mirror", Color(0, 0, 0))
	RenderingServer.global_shader_parameter_set("sky_mirror", Vector3(mirror.r, mirror.g, mirror.b))
	var lamps: Array = _fx.get("lamps", [1.0, 1.0])
	if map_view != null:
		map_view.lamp_boost = lamps[0]
		map_view.lamp_reach = lamps[1]
	_set_rain(_fx.get("rain", 0.0) > 0.0)
	_set_lamp_lights(_fx.has("lamps"))
	if not _fx.get("carry", false) and _carry != null:
		_carry.queue_free()
		_carry = null
	_set_lightning(_fx.get("lightning", false))


## Daylight while something renders the map for itself (the minimap's terrain), then back: a
## map read at a glance stays the same in any weather.
func suspend(off: bool) -> void:
	if off and _suspended.is_empty():
		_suspended = [_sun.rotation_degrees, _sun.light_color, _sun.light_energy]
		_sun.rotation_degrees = SUN_ROTATION
		_sun.light_color = Color(1.0, 0.9, 0.76)
		_sun.light_energy = 1.5
		RenderingServer.global_shader_parameter_set("wetness", 0.0)
		for l in _lamp_lights:
			l.visible = false
		if _carry != null:
			_carry.visible = false
	elif not off and not _suspended.is_empty():
		_sun.rotation_degrees = _suspended[0]
		_sun.light_color = _suspended[1]
		_sun.light_energy = _suspended[2]
		_suspended = []
		RenderingServer.global_shader_parameter_set("wetness", _fx.get("wet", 0.0))
		if _carry != null:
			_carry.visible = true


## How much of the sun the thickest cloud takes in this weather (atmosphere.gd).
func cloud_strength(default: float) -> float:
	return _fx_of(current).get("cloud_strength", default)


static func _fx_of(name: String) -> Dictionary:
	return WEATHERS.get(name, {}).get("fx", {})


## Each frame: the rain over what the camera looks at, the lamps' lights at the nearest lamps,
## the light around your champion, and the lightning.
func update(delta: float, look: Vector3, map_view: Node, own: Node3D) -> void:
	if not _suspended.is_empty():
		return
	if _rain != null:
		_rain.global_position = look + Vector3(0, 10.0, 0)
		_splashes.global_position = Vector3(look.x, 0.03, look.z)
	if not _lamp_lights.is_empty():
		var near: Array = map_view.nearest_lamps if map_view != null else []
		for i in _lamp_lights.size():
			var l := _lamp_lights[i]
			l.visible = i < near.size()
			if l.visible:
				var lamp: Array = near[i]
				l.global_position = lamp[0] + Vector3(0, 0.6, 0)
				l.light_color = lamp[1]
				l.omni_range = lamp[2] * 1.6
				l.light_energy = lamp[3] * 1.4
	if _fx.get("carry", false) and own != null and is_instance_valid(own):
		if _carry == null or not is_instance_valid(_carry):
			_carry = OmniLight3D.new()
			_carry.light_color = Color(1.0, 0.9, 0.76)
			_carry.light_energy = 3.0
			_carry.omni_range = 6.5
			_carry.omni_attenuation = 1.4
			_carry.shadow_enabled = false
			add_child(_carry)
		_carry.global_position = own.global_position + Vector3(0, 2.4, 0)
	if _flash != null:
		_update_lightning(delta)


# ---- Rain --------------------------------------------------------------------------------------

func _set_rain(on: bool) -> void:
	if not on:
		for n in [_rain, _splashes, _rain_sound]:
			if n != null:
				n.queue_free()
		_rain = null
		_splashes = null
		_rain_sound = null
		return
	if _rain == null:
		_rain = _make_rain()
		add_child(_rain)
		_splashes = _make_splashes()
		add_child(_splashes)
		_rain_sound = AudioStreamPlayer.new()
		_rain_sound.stream = _rain_loop()
		_rain_sound.volume_db = -9.0
		_rain_sound.bus = "Effects"
		add_child(_rain_sound)
		_rain_sound.play()
	var col: Color = _fx.get("rain_color", Color(0.78, 0.82, 0.9, 0.3))
	(_rain.draw_pass_1.surface_get_material(0) as StandardMaterial3D).albedo_color = col
	(_splashes.draw_pass_1.surface_get_material(0) as StandardMaterial3D).albedo_color = Color(col.r, col.g, col.b, col.a * 1.5)


## Streaks falling from 10 m over the view, slanted by the wind, each lined up with its fall.
static func _make_rain() -> GPUParticles3D:
	var p := GPUParticles3D.new()
	p.amount = 1800
	p.lifetime = 0.62
	p.preprocess = 1.0
	p.local_coords = false
	p.visibility_aabb = AABB(Vector3(-24, -14, -20), Vector3(48, 18, 40))
	var pm := ParticleProcessMaterial.new()
	pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	pm.emission_box_extents = Vector3(20, 0.5, 16)
	pm.direction = Vector3(0.14, -1.0, 0.06)
	pm.spread = 2.0
	pm.initial_velocity_min = 16.0
	pm.initial_velocity_max = 19.0
	pm.gravity = Vector3.ZERO
	pm.particle_flag_align_y = true
	pm.scale_min = 0.8
	pm.scale_max = 1.2
	p.process_material = pm
	var quad := QuadMesh.new()
	quad.size = Vector2(0.014, 0.7)
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.billboard_mode = BaseMaterial3D.BILLBOARD_FIXED_Y
	m.billboard_keep_scale = true
	m.albedo_texture = _streak()
	m.disable_receive_shadows = true
	quad.material = m
	p.draw_pass_1 = quad
	return p


## Rings where drops land: flat on the ground, spreading and fading in a quarter second.
static func _make_splashes() -> GPUParticles3D:
	var p := GPUParticles3D.new()
	p.amount = 520
	p.lifetime = 0.28
	p.local_coords = false
	p.visibility_aabb = AABB(Vector3(-24, -1, -20), Vector3(48, 2, 40))
	var pm := ParticleProcessMaterial.new()
	pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	pm.emission_box_extents = Vector3(20, 0.0, 16)
	pm.gravity = Vector3.ZERO
	pm.initial_velocity_min = 0.0
	pm.initial_velocity_max = 0.0
	var grow := Curve.new()
	grow.add_point(Vector2(0, 0.25))
	grow.add_point(Vector2(1, 1.0))
	var gt := CurveTexture.new()
	gt.curve = grow
	pm.scale_curve = gt
	var fade := Gradient.new()
	fade.set_color(0, Color(1, 1, 1, 1))
	fade.set_color(1, Color(1, 1, 1, 0))
	var ft := GradientTexture1D.new()
	ft.gradient = fade
	pm.color_ramp = ft
	p.process_material = pm
	var plane := PlaneMesh.new()
	plane.size = Vector2(0.2, 0.2)
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.vertex_color_use_as_albedo = true
	m.albedo_texture = _ring()
	m.disable_receive_shadows = true
	plane.material = m
	p.draw_pass_1 = plane
	return p


## A streak's texture: clear at the top, densest low, gone at the very end.
static func _streak() -> GradientTexture2D:
	var g := Gradient.new()
	g.set_color(0, Color(1, 1, 1, 0))
	g.set_color(1, Color(1, 1, 1, 0))
	g.add_point(0.7, Color(1, 1, 1, 1))
	var t := GradientTexture2D.new()
	t.gradient = g
	t.fill_from = Vector2(0.5, 0.0)
	t.fill_to = Vector2(0.5, 1.0)
	t.width = 4
	t.height = 32
	return t


## A thin soft ring, for a splash.
static func _ring() -> GradientTexture2D:
	var g := Gradient.new()
	g.offsets = PackedFloat32Array([0.0, 0.55, 0.8, 1.0])
	g.colors = PackedColorArray([Color(1, 1, 1, 0), Color(1, 1, 1, 0), Color(1, 1, 1, 1), Color(1, 1, 1, 0)])
	var t := GradientTexture2D.new()
	t.gradient = g
	t.fill = GradientTexture2D.FILL_RADIAL
	t.fill_from = Vector2(0.5, 0.5)
	t.fill_to = Vector2(1.0, 0.5)
	t.width = 32
	t.height = 32
	return t


# ---- Night: lights at the lamps -------------------------------------------------------------

func _set_lamp_lights(on: bool) -> void:
	if not on:
		for l in _lamp_lights:
			l.queue_free()
		_lamp_lights.clear()
		return
	while _lamp_lights.size() < LAMP_LIGHTS:
		var l := OmniLight3D.new()
		l.shadow_enabled = false
		l.omni_attenuation = 1.6
		l.visible = false
		add_child(l)
		_lamp_lights.append(l)


# ---- Lightning and thunder ----------------------------------------------------------------------

func _set_lightning(on: bool) -> void:
	if not on:
		if _flash != null:
			_flash.queue_free()
			_thunder_player.queue_free()
		_flash = null
		_thunder_player = null
		return
	if _flash == null:
		_flash = DirectionalLight3D.new()
		_flash.light_color = Color(0.78, 0.85, 1.0)
		_flash.light_energy = 0.0
		_flash.shadow_enabled = false
		_flash.visible = false
		add_child(_flash)
		_thunder_player = AudioStreamPlayer.new()
		_thunder_player.bus = "Effects"
		add_child(_thunder_player)
		if _thunder.is_empty():
			for variant in [11, 23, 37]:
				_thunder.append(_thunder_clap(variant))
		_next_strike = _rng.randf_range(2.0, 5.0)


## A strike's light over time (seconds since it began): a first stroke, a return stroke, and a
## longer flicker as the channel restrikes, the way lightning seen through rain flickers.
static func _strike_light(t: float) -> float:
	var e := 0.0
	e += exp(-pow((t - 0.03) / 0.03, 2.0)) * 1.0
	e += exp(-pow((t - 0.16) / 0.025, 2.0)) * 0.7
	e += exp(-pow((t - 0.34) / 0.07, 2.0)) * 0.9 * (0.8 + 0.2 * sin(t * 90.0))
	return e


func _update_lightning(delta: float) -> void:
	_next_strike -= delta
	if _next_strike <= 0.0:
		_strike_t = 0.0
		_next_strike = strike_every if strike_every > 0.0 else _rng.randf_range(6.0, 16.0)
		# From somewhere in the sky: a high light from a random side.
		_flash.rotation_degrees = Vector3(_rng.randf_range(-75.0, -55.0), _rng.randf_range(-180.0, 180.0), 0)
		# Thunder after the light, later the farther the strike (sound's 340 m/s), and softer.
		var far := _rng.randf()
		_thunder_in = lerpf(0.3, 3.0, far)
		_thunder_gain = lerpf(0.0, -12.0, far)
	if _strike_t >= 0.0:
		_strike_t += delta
		var e := _strike_light(_strike_t)
		_flash.visible = e > 0.01
		_flash.light_energy = e * 3.2
		_env.ambient_light_energy = _ambient * (1.0 + e * 1.6)
		_env.background_color = _background.lerp(Color(0.5, 0.56, 0.7), e * 0.5)
		if _strike_t > 0.7:
			_strike_t = -1.0
			_flash.visible = false
			_env.ambient_light_energy = _ambient
			_env.background_color = _background
	if _thunder_in >= 0.0:
		_thunder_in -= delta
		if _thunder_in < 0.0:
			_thunder_player.stream = _thunder[_rng.randi() % _thunder.size()]
			_thunder_player.volume_db = _thunder_gain
			_thunder_player.pitch_scale = _rng.randf_range(0.85, 1.1)
			_thunder_player.play()


# ---- Sounds, made here --------------------------------------------------------------------------

const RATE := 22050


## Rain's hiss, three seconds that loop without a seam: soft noise with a low body and drops
## ticking here and there.
static func _rain_loop() -> AudioStreamWAV:
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	var n := RATE * 3
	var fade := RATE / 3
	var s := PackedFloat32Array()
	s.resize(n + fade)
	var hiss := 0.0
	var body := 0.0
	var drop := 0.0
	var drop_f := 0.0
	var phase := 0.0
	for i in n + fade:
		var w := rng.randf_range(-1.0, 1.0)
		hiss += (w - hiss) * 0.45
		body += (w - body) * 0.05
		if rng.randf() < 0.0025:
			drop = rng.randf_range(0.2, 0.6)
			drop_f = rng.randf_range(1800.0, 4200.0)
		phase += TAU * drop_f / RATE
		drop *= 0.992
		s[i] = (hiss - body) * 0.42 + body * 1.1 + sin(phase) * drop * 0.25
	# The loop's end fades into its beginning.
	for i in fade:
		var k := float(i) / fade
		s[i] = s[i] * k + s[n + i] * (1.0 - k)
	s.resize(n)
	return _wav(s, true)


## One thunderclap, six seconds: a crack for a near strike, then the rumble rolling in and dying
## away in uneven swells.
static func _thunder_clap(variant: int) -> AudioStreamWAV:
	var rng := RandomNumberGenerator.new()
	rng.seed = variant
	var n := RATE * 6
	var s := PackedFloat32Array()
	s.resize(n)
	var lo := 0.0
	var lo2 := 0.0
	var crack_lp := 0.0
	var swell := 0.0
	var swell_target := 1.0
	var crack := rng.randf_range(0.3, 1.0)
	for i in n:
		var t := float(i) / RATE
		var w := rng.randf_range(-1.0, 1.0)
		lo += (w - lo) * 0.02
		lo2 += (lo - lo2) * 0.05
		crack_lp += (w - crack_lp) * 0.3
		if i % 2205 == 0:
			swell_target = rng.randf_range(0.35, 1.0)
		swell += (swell_target - swell) * 0.0004
		var rumble := lo2 * 9.0 * (1.0 - exp(-t * 5.0)) * exp(-t * 0.6) * swell
		var snap := (w - crack_lp) * crack * exp(-t * 22.0) * 0.6
		s[i] = rumble + snap
	var peak := 0.0001
	for v in s:
		peak = maxf(peak, absf(v))
	for i in n:
		s[i] = s[i] / peak * 0.9
	return _wav(s, false)


static func _wav(s: PackedFloat32Array, loop: bool) -> AudioStreamWAV:
	var data := PackedByteArray()
	data.resize(s.size() * 2)
	for i in s.size():
		data.encode_s16(i * 2, int(clampf(s[i], -1.0, 1.0) * 32767.0))
	var w := AudioStreamWAV.new()
	w.format = AudioStreamWAV.FORMAT_16_BITS
	w.mix_rate = RATE
	w.stereo = false
	w.data = data
	if loop:
		w.loop_mode = AudioStreamWAV.LOOP_FORWARD
		w.loop_end = s.size()
	return w
