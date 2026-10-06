extends Node3D
## M1 sandbox client. Builds the scene in code, forwards input to the Rust MatchClient,
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
const MINION_BLUE := Color(0.35, 0.45, 0.75)
const MINION_RED := Color(0.7, 0.35, 0.35)
const ALLY_COLOR := Color(0.3, 0.85, 0.8)

var client: MatchClient
var camera: Camera3D
var own_body: MeshInstance3D
var remote_bodies := {}                 # unit_id -> MeshInstance3D
var click_marker: MeshInstance3D
var click_marker_age := 1.0
var net_label: Label
var show_net_graph := true
var proxies_enabled := true


func _ready() -> void:
	_build_world()
	client = MatchClient.new()
	add_child(client)
	var address := "127.0.0.1:7777"
	var args := OS.get_cmdline_user_args()
	var i := 0
	while i < args.size():
		if args[i] == "--shot-at" and i + 1 < args.size():
			_shot_at = float(args[i + 1])
			i += 1
		elif args[i] == "--shot" and i + 1 < args.size():
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
var _shot_at := 1.05                     # `--shot-at <seconds>` after joining


func _update_shot(delta: float) -> void:
	if _shot_path == "" or client.phase() != "playing":
		return
	_shot_timer += delta
	if not _shot_moved and _shot_timer > 1.0:
		var own := client.own_position()
		client.cast_q(own + Vector2(800, 250))
		client.move_to(own + Vector2(900, -600))
		click_marker.position = _to_world(own + Vector2(900, -600)) * Vector3(1, 0, 1) + Vector3(0, 0.02, 0)
		click_marker.visible = true
		click_marker_age = 0.0
		_shot_moved = true
	elif _shot_moved and _shot_timer > _shot_at:
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
	elif event.is_action_pressed("cast_q"):
		var aim = _ground_point(get_viewport().get_mouse_position())
		if aim != null:
			client.cast_q(Vector2(aim.x, aim.z) / UNITS_TO_METERS)
	elif event.is_action_pressed("stop"):
		client.stop()
	elif event.is_action_pressed("toggle_proxies"):
		proxies_enabled = not proxies_enabled
		client.set_collision_proxies(proxies_enabled)
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
	_update_missiles(delta)
	_update_own_status()
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
	var seen := {}
	for u in client.remote_units():
		var id: int = u.id
		seen[id] = true
		if not remote_bodies.has(id):
			var b: MeshInstance3D
			if u.turret:
				b = _make_turret(ALLY_COLOR if u.ally else ENEMY_COLOR, u.radius)
			elif u.minion:
				b = _make_minion(MINION_BLUE if u.ally else MINION_RED, u.radius)
			else:
				b = _make_champion(ALLY_COLOR if u.ally else ENEMY_COLOR)
			b.add_child(_make_windup_indicator())
			b.add_child(_make_stun_indicator())
			add_child(b)
			remote_bodies[id] = b
		var body: MeshInstance3D = remote_bodies[id]
		var p := _to_world(u.pos)
		if u.minion:
			p.y = 0.45
		elif u.turret:
			p.y = 1.2
		body.position = p
		_show_windup(body, u.get("windup", -1.0), u.get("windup_dir", Vector2.ZERO))
		body.get_node("Stun").visible = u.stunned
	for id in remote_bodies.keys():
		if not seen.has(id):
			remote_bodies[id].queue_free()
			remote_bodies.erase(id)


## Minions: short capsules whose ground ring is the *collision* radius, so minion block is
## visible exactly as the simulation sees it (D11).
func _make_minion(color: Color, collision_radius_u: float) -> MeshInstance3D:
	var body := MeshInstance3D.new()
	var capsule := CapsuleMesh.new()
	capsule.radius = collision_radius_u * UNITS_TO_METERS * 0.8
	capsule.height = 0.9
	body.mesh = capsule
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	body.material_override = m
	var ring := MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.outer_radius = collision_radius_u * UNITS_TO_METERS
	torus.inner_radius = torus.outer_radius - 0.03
	ring.mesh = torus
	ring.position = Vector3(0, -0.44, 0)
	var rm := StandardMaterial3D.new()
	rm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	rm.albedo_color = color.lightened(0.3)
	ring.material_override = rm
	body.add_child(ring)
	return body


func _make_turret(color: Color, collision_radius_u: float) -> MeshInstance3D:
	var body := MeshInstance3D.new()
	var cyl := CylinderMesh.new()
	cyl.top_radius = collision_radius_u * UNITS_TO_METERS * 0.6
	cyl.bottom_radius = collision_radius_u * UNITS_TO_METERS
	cyl.height = 2.4
	body.mesh = cyl
	var m := StandardMaterial3D.new()
	m.albedo_color = Color(0.25, 0.25, 0.3)
	m.emission_enabled = true
	m.emission = color * 0.3
	body.material_override = m
	return body


## Cast windup: a thin aim line that brightens toward the moment the missile fires. Enemy
## windups are already on the input timeline (03a §7), so they line up with their missiles.
func _make_windup_indicator() -> MeshInstance3D:
	var line := MeshInstance3D.new()
	line.name = "Windup"
	var box := BoxMesh.new()
	box.size = Vector3(6.0, 0.02, 0.08)
	line.mesh = box
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	m.albedo_color = Color(1.0, 0.45, 0.3, 0.0)
	line.material_override = m
	line.visible = false
	return line


func _show_windup(body: Node3D, progress: float, dir: Vector2) -> void:
	var line: MeshInstance3D = body.get_node("Windup")
	line.visible = progress >= 0.0
	if not line.visible:
		return
	var angle := atan2(dir.y, dir.x)
	line.global_rotation = Vector3(0, -angle, 0)
	line.global_position = Vector3(body.global_position.x, 0.05, body.global_position.z) + Vector3(dir.x, 0, dir.y) * 3.0
	(line.material_override as StandardMaterial3D).albedo_color.a = 0.15 + 0.6 * progress


func _make_stun_indicator() -> MeshInstance3D:
	var ring := MeshInstance3D.new()
	ring.name = "Stun"
	var torus := TorusMesh.new()
	torus.inner_radius = 0.25
	torus.outer_radius = 0.35
	ring.mesh = torus
	ring.position = Vector3(0, 1.3, 0)
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.albedo_color = Color(1.0, 0.9, 0.3)
	ring.material_override = m
	ring.visible = false
	return ring


## Missiles (D14): a slim bright core with a faint, always-visible sheath at the *true*
## hitbox width, so what you see is what can hit you.
var missile_nodes := {}                 # key -> Node3D


func _make_missile(side: String, radius_u: float) -> Node3D:
	var root := Node3D.new()
	var color := ENEMY_COLOR if side == "enemy" else (OWN_COLOR if side == "own" else ALLY_COLOR)
	var width := radius_u * 2.0 * UNITS_TO_METERS
	var sheath := MeshInstance3D.new()
	sheath.name = "Sheath"
	var plate := BoxMesh.new()
	plate.size = Vector3(width * 2.2, 0.02, width)
	sheath.mesh = plate
	sheath.position = Vector3(-width * 0.6, 0.06, 0)
	var sm := StandardMaterial3D.new()
	sm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	sm.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	sm.albedo_color = Color(color.r, color.g, color.b, 0.35)
	sheath.material_override = sm
	root.add_child(sheath)
	var core := MeshInstance3D.new()
	core.name = "Core"
	var capsule := CapsuleMesh.new()
	capsule.radius = width * 0.2
	capsule.height = width * 1.6
	core.mesh = capsule
	core.rotation = Vector3(0, 0, PI / 2.0)
	core.position = Vector3(0, 0.25, 0)
	var cm := StandardMaterial3D.new()
	cm.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	cm.albedo_color = color.lightened(0.5)
	core.material_override = cm
	root.add_child(core)
	var impact := MeshInstance3D.new()
	impact.name = "Impact"
	var sphere := SphereMesh.new()
	sphere.radius = width * 1.2
	sphere.height = width * 2.4
	impact.mesh = sphere
	impact.position = Vector3(0, 0.25, 0)
	var im := StandardMaterial3D.new()
	im.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	im.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	im.albedo_color = Color(1.0, 0.95, 0.7, 0.8)
	impact.material_override = im
	impact.visible = false
	root.add_child(impact)
	return root


func _update_missiles(_delta: float) -> void:
	var seen := {}
	for m in client.missiles():
		var key: int = m.key
		seen[key] = true
		if not missile_nodes.has(key):
			var node := _make_missile(m.side, m.radius)
			add_child(node)
			missile_nodes[key] = node
		var node: Node3D = missile_nodes[key]
		var dir: Vector2 = m.dir
		node.position = Vector3(m.pos.x * UNITS_TO_METERS, 0.0, m.pos.y * UNITS_TO_METERS)
		node.rotation = Vector3(0, -atan2(dir.y, dir.x), 0)
		node.get_node("Core").visible = not m.impact
		node.get_node("Sheath").visible = not m.impact
		node.get_node("Impact").visible = m.impact
		# Predicted to hit someone else first (03a §7): keep it visible, dimmed, until confirmed.
		var sheath_mat: StandardMaterial3D = node.get_node("Sheath").material_override
		sheath_mat.albedo_color.a = 0.12 if m.unconfirmed else 0.35
		node.get_node("Core").transparency = 0.6 if m.unconfirmed else 0.0
	for key in missile_nodes.keys():
		if not seen.has(key):
			missile_nodes[key].queue_free()
			missile_nodes.erase(key)


var own_stun: MeshInstance3D
var q_cooldown := 0.0


func _update_own_status() -> void:
	if own_stun == null:
		own_stun = _make_stun_indicator()
		own_body.add_child(own_stun)
	var st: Dictionary = client.own_status()
	own_stun.visible = st.get("stunned", false)
	q_cooldown = st.get("q_cooldown", 0.0)


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
	net_label.text = "FPS %d   RTT %.0f ms   margin %.1f ms   interp %.0f ms\ncommands %d   late %d   corrections %d (last %.1f u)   on-screen correction %.1f u\nup %.1f KB   down %.1f KB   collision proxies %s\nQ %s   enemy missiles %d   near-misses %d   ghost hits %d   phantom hits %d\n[RMB] move  [Q] skillshot  [S] stop  [F1] net graph  [F2] proxies" % [
		Engine.get_frames_per_second(), s.rtt_ms, s.margin_ms, s.interp_ms,
		s.commands, s.late, s.corrections, s.last_correction, s.visible_correction,
		s.kb_up, s.kb_down, "ON" if proxies_enabled else "OFF",
		"ready" if q_cooldown <= 0.0 else "%.1f s" % q_cooldown,
		s.enemy_missiles, s.near_misses, s.ghost_hits, s.phantom_hits,
	]
