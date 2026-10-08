extends Node3D
## Atmosphere over the map (05 §5): cloud shadows drifting across the terrain and pollen motes
## floating in the light around the camera. Generated at runtime: no texture files.
##
## The cloud shadows live in the map's shaders (clouds.gdshaderinc): they dim the sun on
## terrain, props and champions alike; this node only moves the wind and sets their strength.

const WIND := Vector2(0.55, 0.22)        # meters per second
const CLOUD_STRENGTH := 0.42             # how much of the sun the thickest cloud takes

var _drift := Vector2.ZERO
var _clouds := true
var _motes: GPUParticles3D


func setup(_map_center_m: Vector2) -> void:
	RenderingServer.global_shader_parameter_set("cloud_strength", CLOUD_STRENGTH)
	_motes = GPUParticles3D.new()
	_motes.amount = 90
	_motes.lifetime = 9.0
	_motes.preprocess = 9.0
	_motes.local_coords = false
	_motes.visibility_aabb = AABB(Vector3(-30, -5, -30), Vector3(60, 15, 60))
	var pm := ParticleProcessMaterial.new()
	pm.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_BOX
	pm.emission_box_extents = Vector3(22, 1.5, 16)
	pm.direction = Vector3(WIND.x, 0.25, WIND.y)
	pm.spread = 70.0
	pm.initial_velocity_min = 0.15
	pm.initial_velocity_max = 0.5
	pm.gravity = Vector3(0, 0.02, 0)
	pm.turbulence_enabled = true
	pm.turbulence_noise_strength = 0.6
	pm.turbulence_noise_scale = 6.0
	pm.scale_min = 0.6
	pm.scale_max = 1.3
	var fade := Gradient.new()
	fade.set_color(0, Color(1, 1, 1, 0))
	fade.set_color(1, Color(1, 1, 1, 0))
	fade.add_point(0.2, Color(1, 1, 1, 1))
	fade.add_point(0.8, Color(1, 1, 1, 1))
	var ft := GradientTexture1D.new()
	ft.gradient = fade
	pm.color_ramp = ft
	_motes.process_material = pm
	var quad := QuadMesh.new()
	quad.size = Vector2(0.07, 0.07)
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
	m.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
	m.vertex_color_use_as_albedo = true
	m.albedo_color = Color(1.0, 0.92, 0.7, 0.55)
	m.emission_enabled = true
	m.emission = Color(1.0, 0.85, 0.55)
	m.emission_energy_multiplier = 0.6
	quad.material = m
	_motes.draw_pass_1 = quad
	add_child(_motes)


## Each frame: drift the clouds, keep the motes around what the camera looks at (meters).
func update(delta: float, look: Vector3) -> void:
	_drift += WIND * delta
	RenderingServer.global_shader_parameter_set("cloud_offset", -_drift)
	if _motes != null:
		_motes.global_position = look + Vector3(0, 1.8, 0)


func set_enabled(clouds: bool, motes: bool) -> void:
	_clouds = clouds
	RenderingServer.global_shader_parameter_set("cloud_strength", CLOUD_STRENGTH if clouds else 0.0)
	if _motes != null:
		_motes.visible = motes
		_motes.emitting = motes


## Off while something renders without them (the minimap's terrain), then back.
func suspend(off: bool) -> void:
	RenderingServer.global_shader_parameter_set("cloud_strength", 0.0 if off or not _clouds else CLOUD_STRENGTH)
