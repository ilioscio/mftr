extends Node3D
## M0 movement sandbox. Builds the scene in code, forwards input to the Rust MatchClient,
## and draws what it reports. No gameplay decisions are made here (04 §1).
##
## Server address: first command-line user arg (`-- 192.168.1.10:7777`) or 127.0.0.1:7777.

const UNITS_TO_METERS := 0.01           # 1 game unit = 1 cm
const CAMERA_PITCH_DEG := 56.0          # D13 / R01 §1
const CAMERA_VFOV_DEG := 45.0
const CAMERA_DISTANCE_U := 1900.0
const ARENA_U := 4000.0
const CHAMPION_RADIUS_U := 65.0         # gameplay radius
const OWN_COLOR := Color(0.25, 0.55, 1.0)
const ENEMY_COLOR := Color(0.95, 0.35, 0.25)

var client: MatchClient
var camera: Camera3D
var own_body: MeshInstance3D
var remote_bodies := {}                 # unit_id -> MeshInstance3D
var click_marker: MeshInstance3D
var click_marker_age := 1.0
var net_label: Label
var show_net_graph := true


func _ready() -> void:
	_build_world()
	client = MatchClient.new()
	add_child(client)
	var address := "127.0.0.1:7777"
	var args := OS.get_cmdline_user_args()
	var i := 0
	while i < args.size():
		if args[i] == "--shot" and i + 1 < args.size():
			_shot_path = args[i + 1]
			i += 1
		else:
			address = args[i]
		i += 1
	if not client.connect_to_server(address):
		push_error("MFTR: could not open socket: %s" % client.last_error())


## `--shot <file.png>`: scripted capture for automated visual checks. Waits until playing,
## issues one move order, saves a screenshot mid-move and quits.
var _shot_path := ""
var _shot_timer := 0.0
var _shot_moved := false


func _update_shot(delta: float) -> void:
	if _shot_path == "" or client.phase() != "playing":
		return
	_shot_timer += delta
	if not _shot_moved and _shot_timer > 1.0:
		var own := client.own_position()
		client.move_to(own + Vector2(900, -600))
		click_marker.position = _to_world(own + Vector2(900, -600)) * Vector3(1, 0, 1) + Vector3(0, 0.02, 0)
		click_marker.visible = true
		click_marker_age = 0.0
		_shot_moved = true
	elif _shot_moved and _shot_timer > 1.05:
		get_viewport().get_texture().get_image().save_png(_shot_path)
		print("MFTR: saved screenshot to ", _shot_path)
		get_tree().quit()


func _build_world() -> void:
	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_COLOR
	e.background_color = Color(0.08, 0.09, 0.1)
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color(0.55, 0.6, 0.65)
	e.ambient_light_energy = 0.6
	env.environment = e
	add_child(env)

	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-60, -35, 0)
	sun.light_energy = 1.1
	add_child(sun)

	var ground := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	var size_m := ARENA_U * UNITS_TO_METERS
	plane.size = Vector2(size_m, size_m)
	ground.mesh = plane
	ground.position = Vector3(size_m / 2.0, 0, size_m / 2.0)
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/ground.gdshader")
	ground.material_override = mat
	add_child(ground)

	camera = Camera3D.new()
	camera.fov = CAMERA_VFOV_DEG
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.far = 200.0
	add_child(camera)
	camera.make_current()

	own_body = _make_champion(OWN_COLOR)
	own_body.visible = false
	add_child(own_body)

	click_marker = MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.inner_radius = 0.75
	torus.outer_radius = 0.9
	click_marker.mesh = torus
	var marker_mat := StandardMaterial3D.new()
	marker_mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	marker_mat.albedo_color = Color(0.45, 0.75, 1.0)
	click_marker.material_override = marker_mat
	click_marker.visible = false
	add_child(click_marker)

	var hud := CanvasLayer.new()
	add_child(hud)
	net_label = Label.new()
	net_label.position = Vector2(16, 16)
	net_label.add_theme_font_size_override("font_size", 16)
	net_label.add_theme_color_override("font_shadow_color", Color.BLACK)
	hud.add_child(net_label)


func _make_champion(color: Color) -> MeshInstance3D:
	var body := MeshInstance3D.new()
	var capsule := CapsuleMesh.new()
	capsule.radius = CHAMPION_RADIUS_U * UNITS_TO_METERS * 0.6
	capsule.height = 1.7
	body.mesh = capsule
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	m.rim_enabled = true
	m.rim = 0.6
	body.material_override = m
	# Gameplay-radius ring on the ground: the honest hitbox (05 §1).
	var ring := MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.outer_radius = CHAMPION_RADIUS_U * UNITS_TO_METERS
	torus.inner_radius = torus.outer_radius - 0.05
	ring.mesh = torus
	ring.position = Vector3(0, -0.84, 0)
	var rm := StandardMaterial3D.new()
	rm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	rm.albedo_color = color
	ring.material_override = rm
	body.add_child(ring)
	return body


func _to_world(p: Vector2) -> Vector3:
	return Vector3(p.x * UNITS_TO_METERS, 0.85, p.y * UNITS_TO_METERS)


func _unhandled_input(event: InputEvent) -> void:
	if event.is_action_pressed("move"):
		var hit = _ground_point(get_viewport().get_mouse_position())
		if hit != null:
			client.move_to(Vector2(hit.x, hit.z) / UNITS_TO_METERS)
			click_marker.position = Vector3(hit.x, 0.02, hit.z)
			click_marker.visible = true
			click_marker_age = 0.0
	elif event.is_action_pressed("stop"):
		client.stop()
	elif event.is_action_pressed("toggle_net_graph"):
		show_net_graph = not show_net_graph


## Ray-cast the mouse onto the ground plane (y = 0). Never assume a fixed px/u (R01 §1).
func _ground_point(mouse: Vector2):
	var origin := camera.project_ray_origin(mouse)
	var dir := camera.project_ray_normal(mouse)
	if absf(dir.y) < 1e-5:
		return null
	var t := -origin.y / dir.y
	if t < 0.0:
		return null
	return origin + dir * t


var _last_phase := ""


func _process(delta: float) -> void:
	var phase := client.phase()
	if phase != _last_phase:
		print("MFTR phase: %s -> %s (unit %d)" % [_last_phase, phase, client.own_unit_id()])
		_last_phase = phase
	var playing := phase == "playing"
	own_body.visible = playing
	if playing:
		var own := _to_world(client.own_position())
		own_body.position = own
		_place_camera(own)
	_update_remotes()
	_update_click_marker(delta)
	_update_net_graph()
	_update_shot(delta)


func _place_camera(target: Vector3) -> void:
	var pitch := deg_to_rad(CAMERA_PITCH_DEG)
	var dist := CAMERA_DISTANCE_U * UNITS_TO_METERS
	var look := Vector3(target.x, 0.0, target.z)
	camera.position = look + Vector3(0, sin(pitch) * dist, cos(pitch) * dist)
	camera.look_at(look, Vector3.UP)


func _update_remotes() -> void:
	var remotes: Dictionary = client.remote_positions()
	for id in remotes.keys():
		if not remote_bodies.has(id):
			var b := _make_champion(ENEMY_COLOR)
			add_child(b)
			remote_bodies[id] = b
		remote_bodies[id].position = _to_world(remotes[id])
	for id in remote_bodies.keys():
		if not remotes.has(id):
			remote_bodies[id].queue_free()
			remote_bodies.erase(id)


## Click indicator: a ring that collapses over ~0.1 s (familiar feel, R01 §6).
func _update_click_marker(delta: float) -> void:
	if not click_marker.visible:
		return
	click_marker_age += delta
	var t := clampf(click_marker_age / 0.12, 0.0, 1.0)
	var s := lerpf(1.0, 0.25, t)
	click_marker.scale = Vector3(s, 1.0, s)
	if t >= 1.0:
		click_marker.visible = false


func _update_net_graph() -> void:
	net_label.visible = show_net_graph
	if not show_net_graph:
		return
	var phase := client.phase()
	if phase != "playing":
		net_label.text = "MFTR — %s…  %s" % [phase if phase != "" else "disconnected", client.last_error()]
		return
	var s: Dictionary = client.net_stats()
	net_label.text = "FPS %d   RTT %.0f ms   margin %.1f ms   interp %.0f ms\ncommands %d   late %d   corrections %d (last %.1f u)   on-screen correction %.1f u\nup %.1f KB   down %.1f KB      [RMB] move  [S] stop  [F1] net graph" % [
		Engine.get_frames_per_second(), s.rtt_ms, s.margin_ms, s.interp_ms,
		s.commands, s.late, s.corrections, s.last_correction, s.visible_correction,
		s.kb_up, s.kb_down,
	]
