extends Node3D
## M1 Duel Sandbox client. Builds the scene in code, forwards input to the Rust MatchClient,
## and draws what it reports. No gameplay decisions are made here (04 §1).
##
## User args (after `--`): a server address (default 127.0.0.1:7777), `--champion ember|vesper`,
## `--shot <file.png>` / `--shot-at <seconds>` for scripted screenshots, and the blind playtest
## options `--blind [seed]`, `--blind-rounds N`, `--blind-seconds S`, `--blind-auto`.

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
const HARD_CC_COLOR := Color(1.0, 0.85, 0.25)
const SLOT_KEYS := ["Q", "W", "E", "R", "D", "F"]
const SLOT_ACTIONS := ["cast_q", "cast_w", "cast_e", "cast_r", "cast_d", "cast_f"]

var client: MatchClient
var camera: Camera3D
var own_body: Node3D
var own_champion := ""
var remote_bodies := {}                 # unit_id -> Node3D
var click_marker: MeshInstance3D
var click_marker_age := 1.0
var net_label: Label
var overlay: Control
var show_net_graph := true
var proxies_enabled := true
var attack_move_armed := false
var own_status := {}
var floaters := []                      # damage numbers: { pos: Vector3, text, color, age }
var notices := []                       # kill feed: { text, age }


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
		elif args[i] == "--blind":
			blind_enabled = true
			if i + 1 < args.size() and args[i + 1].is_valid_int():
				blind_seed = int(args[i + 1])
				i += 1
		elif args[i] == "--blind-rounds" and i + 1 < args.size():
			blind_rounds = int(args[i + 1])
			i += 1
		elif args[i] == "--blind-seconds" and i + 1 < args.size():
			blind_seconds = float(args[i + 1])
			i += 1
		elif args[i] == "--blind-auto":
			blind_auto = true
		elif args[i] == "--champion" and i + 1 < args.size():
			if not client.set_champion(args[i + 1]):
				push_error("MFTR: unknown champion %s" % args[i + 1])
			i += 1
		else:
			address = args[i]
		i += 1
	if not client.connect_to_server(address):
		push_error("MFTR: could not open socket: %s" % client.last_error())


## `--shot <file.png>`: scripted capture for automated visual checks. Waits until playing,
## casts an area and a skillshot, issues one move order, saves a screenshot and quits.
var _shot_path := ""
var _shot_timer := 0.0
var _shot_moved := false
var _shot_at := 1.05                     # `--shot-at <seconds>` after joining


func _update_shot(delta: float) -> void:
	if _shot_path == "" or client.phase() != "playing":
		return
	_shot_timer += delta
	# Everything aims toward the arena's center, whichever side we spawned on.
	var own := client.own_position()
	var inward := (Vector2(2000, 2000) - own).normalized()
	var side := Vector2(-inward.y, inward.x)
	if not _shot_moved and _shot_timer > 1.0:
		client.cast(1, own + inward * 700.0 + side * 200.0)
		_shot_moved = true
	elif _shot_moved and _shot_timer > 1.4 and _shot_timer - delta <= 1.4:
		client.cast(0, own + inward * 800.0 - side * 250.0)
		client.move_to(own + inward * 600.0 - side * 500.0)
		_show_click_marker(_to_world(own + inward * 600.0 - side * 500.0), OWN_COLOR)
	elif _shot_moved and _shot_timer > 1.7 and _shot_timer - delta <= 1.7:
		client.cast(4, own + side * 400.0)
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

	click_marker = MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.inner_radius = 0.75
	torus.outer_radius = 0.9
	click_marker.mesh = torus
	click_marker.material_override = _unshaded(Color(0.45, 0.75, 1.0))
	click_marker.visible = false
	add_child(click_marker)

	var hud := CanvasLayer.new()
	add_child(hud)
	overlay = Control.new()
	overlay.set_anchors_preset(Control.PRESET_FULL_RECT)
	overlay.mouse_filter = Control.MOUSE_FILTER_IGNORE
	overlay.draw.connect(_draw_overlay)
	hud.add_child(overlay)
	net_label = Label.new()
	net_label.position = Vector2(16, 16)
	net_label.add_theme_font_size_override("font_size", 16)
	net_label.add_theme_color_override("font_shadow_color", Color.BLACK)
	hud.add_child(net_label)


func _unshaded(color: Color, alpha := 1.0) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	m.albedo_color = Color(color.r, color.g, color.b, alpha)
	if alpha < 1.0:
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	return m


## Placeholder champions with distinct silhouettes (05 §1): Ember is a capsule with a floating
## orb, Vesper a slimmer column with a pointed hood. The ground ring is the gameplay radius,
## the honest hitbox.
func _make_champion(color: Color, champion: String) -> Node3D:
	var body := MeshInstance3D.new()
	# Identity color per champion; the team accent is a band at the feet plus the ring.
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/champion.gdshader")
	m.set_shader_parameter("base_color", Color(0.34, 0.42, 0.36) if champion == "Vesper" else Color(0.62, 0.36, 0.22))
	m.set_shader_parameter("team_accent", color)
	if champion == "Vesper":
		var cyl := CylinderMesh.new()
		cyl.top_radius = 0.22
		cyl.bottom_radius = 0.3
		cyl.height = 1.5
		body.mesh = cyl
		var hood := MeshInstance3D.new()
		var cone := CylinderMesh.new()
		cone.top_radius = 0.0
		cone.bottom_radius = 0.28
		cone.height = 0.45
		hood.mesh = cone
		hood.position = Vector3(0, 0.95, 0)
		hood.material_override = m
		body.add_child(hood)
	else:
		var capsule := CapsuleMesh.new()
		capsule.radius = CHAMPION_RADIUS_U * UNITS_TO_METERS * 0.6
		capsule.height = 1.7
		body.mesh = capsule
		var orb := MeshInstance3D.new()
		var sphere := SphereMesh.new()
		sphere.radius = 0.14
		sphere.height = 0.28
		orb.mesh = sphere
		orb.position = Vector3(0.45, 0.55, 0)
		orb.material_override = _unshaded(Color(1.0, 0.7, 0.3))
		body.add_child(orb)
	body.material_override = m
	var ring := MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.outer_radius = CHAMPION_RADIUS_U * UNITS_TO_METERS
	torus.inner_radius = torus.outer_radius - 0.05
	ring.mesh = torus
	ring.position = Vector3(0, -0.84, 0)
	ring.material_override = _unshaded(color)
	body.add_child(ring)
	body.add_child(_make_shield_bubble())
	body.add_child(_make_stun_indicator())
	body.add_child(_make_root_indicator())
	return body


func _make_shield_bubble() -> MeshInstance3D:
	var bubble := MeshInstance3D.new()
	bubble.name = "Shield"
	var sphere := SphereMesh.new()
	sphere.radius = 0.8
	sphere.height = 1.9
	bubble.mesh = sphere
	bubble.material_override = _unshaded(Color(0.95, 0.95, 1.0), 0.22)
	bubble.visible = false
	return bubble


func _to_world(p: Vector2) -> Vector3:
	return Vector3(p.x * UNITS_TO_METERS, 0.85, p.y * UNITS_TO_METERS)


func _show_click_marker(at: Vector3, color: Color) -> void:
	click_marker.position = Vector3(at.x, 0.02, at.z)
	(click_marker.material_override as StandardMaterial3D).albedo_color = color
	click_marker.visible = true
	click_marker_age = 0.0


func _cursor_ground():
	var hit = _ground_point(get_viewport().get_mouse_position())
	if hit == null:
		return null
	return Vector2(hit.x, hit.z) / UNITS_TO_METERS


func _unhandled_input(event: InputEvent) -> void:
	if blind_panel != null and blind_panel.visible:
		return  # rating between rounds: the game ignores input
	if event.is_action_pressed("move"):
		attack_move_armed = false
		var p = _cursor_ground()
		if p == null:
			return
		var target: int = client.pick_enemy(p, 30.0)
		if target >= 0:
			client.attack_unit(target)
			_show_click_marker(_to_world(p), ENEMY_COLOR)
		else:
			client.move_to(p)
			_show_click_marker(_to_world(p), Color(0.45, 0.75, 1.0))
		return
	if event.is_action_pressed("select") and attack_move_armed:
		attack_move_armed = false
		var p = _cursor_ground()
		if p != null:
			client.attack_move(p)
			_show_click_marker(_to_world(p), ENEMY_COLOR)
		return
	for slot in SLOT_ACTIONS.size():
		if event.is_action_pressed(SLOT_ACTIONS[slot]):
			var aim = _cursor_ground()
			if aim != null:
				client.cast(slot, aim)
			return
	if event.is_action_pressed("attack_move"):
		attack_move_armed = true
	elif event.is_action_pressed("stop"):
		attack_move_armed = false
		client.stop()
	elif event.is_action_pressed("toggle_proxies"):
		proxies_enabled = not proxies_enabled
		client.set_collision_proxies(proxies_enabled)
	elif event.is_action_pressed("toggle_net_graph") and not blind_enabled:
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
	if playing and not _map_built:
		_build_map()
	if playing:
		_update_blind()
	own_status = client.own_status() if playing else {}
	if playing and own_body == null:
		own_champion = own_status.get("champion", "")
		own_body = _make_champion(OWN_COLOR, own_champion)
		add_child(own_body)
	if own_body != null:
		var dead: bool = own_status.get("dead", false)
		own_body.visible = playing and not dead
		var own := _to_world(client.own_position())
		own_body.position = own
		_place_camera(own)
		_show_statuses(own_body, own_status.get("stunned", false), own_status.get("rooted", false), own_status.get("shield", 0.0))
	_update_remotes()
	_update_missiles()
	_update_areas()
	_update_bolts()
	_update_combat_text(delta)
	_update_fx(delta)
	_update_click_marker(delta)
	_update_net_graph()
	_update_shot(delta)
	overlay.queue_redraw()


func _place_camera(target: Vector3) -> void:
	var pitch := deg_to_rad(CAMERA_PITCH_DEG)
	var dist := CAMERA_DISTANCE_U * UNITS_TO_METERS
	var look := Vector3(target.x, 0.0, target.z)
	camera.position = look + Vector3(0, sin(pitch) * dist, cos(pitch) * dist)
	camera.look_at(look, Vector3.UP)


func _show_statuses(body: Node3D, stunned: bool, rooted: bool, shield: float) -> void:
	body.get_node("Stun").visible = stunned
	body.get_node("Root").visible = rooted
	body.get_node("Shield").visible = shield > 0.0


var remote_info := {}                   # unit_id -> latest dictionary (for bars and numbers)


func _update_remotes() -> void:
	var seen := {}
	remote_info.clear()
	for u in client.remote_units():
		var id: int = u.id
		seen[id] = true
		remote_info[id] = u
		if not remote_bodies.has(id):
			var b: Node3D
			if u.turret:
				b = _make_turret(ALLY_COLOR if u.ally else ENEMY_COLOR, u.radius)
			elif u.minion:
				b = _make_minion(MINION_BLUE if u.ally else MINION_RED, u.radius)
			else:
				b = _make_champion(ALLY_COLOR if u.ally else ENEMY_COLOR, u.champion)
			b.add_child(_make_windup_indicator())
			if not b.has_node("Stun"):
				b.add_child(_make_stun_indicator())
				b.add_child(_make_root_indicator())
				b.add_child(_make_shield_bubble())
			add_child(b)
			remote_bodies[id] = b
		var body: Node3D = remote_bodies[id]
		var p := _to_world(u.pos)
		if u.minion:
			p.y = 0.45
		elif u.turret:
			p.y = 1.2
		body.position = p
		_show_windup(body, u.get("windup", -1.0), u.get("windup_dir", Vector2.ZERO))
		_show_statuses(body, u.stunned, u.rooted, u.shield)
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
	ring.material_override = _unshaded(color.lightened(0.3))
	body.add_child(ring)
	return body


## Walls (extruded, vision-blocking) and brush (low translucent tufts) from the map the server
## announced. The same polygons drive collision, pathing and vision in the simulation.
var _map_built := false


func _build_map() -> void:
	_map_built = true
	var geo: Dictionary = client.map_geometry()
	var wall_mat := ShaderMaterial.new()
	wall_mat.shader = load("res://shaders/wall.gdshader")
	var brush_mat := ShaderMaterial.new()
	brush_mat.shader = load("res://shaders/brush.gdshader")
	for poly in geo.walls:
		add_child(_extrude(poly, 1.4, wall_mat))
	for poly in geo.brush:
		add_child(_extrude(poly, 0.55, brush_mat))


## CSGPolygon3D extrudes along local -Z; rotating +90° about X lays the polygon on the ground
## (x, y) -> world (x, 0, y) and extrudes upward.
func _extrude(poly: PackedVector2Array, height: float, mat: Material) -> CSGPolygon3D:
	var pts := PackedVector2Array()
	for p in poly:
		pts.append(p * UNITS_TO_METERS)
	var csg := CSGPolygon3D.new()
	csg.polygon = pts
	csg.mode = CSGPolygon3D.MODE_DEPTH
	csg.depth = height
	csg.rotation_degrees = Vector3(90, 0, 0)
	csg.material = mat
	return csg


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


## Cast windup: a thin aim line that brightens toward the moment the cast fires. Enemy
## windups are already on the input timeline (03a §7), so they line up with their missiles.
func _make_windup_indicator() -> MeshInstance3D:
	var line := MeshInstance3D.new()
	line.name = "Windup"
	var box := BoxMesh.new()
	box.size = Vector3(6.0, 0.02, 0.08)
	line.mesh = box
	line.material_override = _unshaded(Color(1.0, 0.45, 0.3), 0.0)
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
	ring.material_override = _unshaded(HARD_CC_COLOR)
	ring.visible = false
	return ring


## Root: a tight ring at the feet in the hard-CC accent color.
func _make_root_indicator() -> MeshInstance3D:
	var ring := MeshInstance3D.new()
	ring.name = "Root"
	var torus := TorusMesh.new()
	torus.inner_radius = 0.42
	torus.outer_radius = 0.52
	ring.mesh = torus
	ring.position = Vector3(0, -0.8, 0)
	ring.material_override = _unshaded(HARD_CC_COLOR)
	ring.visible = false
	return ring


func _side_color(side: String) -> Color:
	if side == "enemy":
		return ENEMY_COLOR
	return OWN_COLOR if side == "own" else ALLY_COLOR


## Missiles (D14): a slim bright core with a faint, always-visible sheath at the *true*
## hitbox width, so what you see is what can hit you. Hard CC adds a ring accent (05 §1).
var missile_nodes := {}                 # key -> Node3D


func _make_missile(side: String, radius_u: float, hard_cc: bool) -> Node3D:
	var root := Node3D.new()
	var color := _side_color(side)
	var trail_u := clampf(radius_u * 4.0, 100.0, 260.0)
	var body := MeshInstance3D.new()
	body.name = "Body"
	var plane := PlaneMesh.new()
	plane.size = Vector2((radius_u + trail_u) * UNITS_TO_METERS, radius_u * 2.0 * UNITS_TO_METERS)
	plane.center_offset = Vector3((radius_u - trail_u) * 0.5 * UNITS_TO_METERS, 0, 0)
	body.mesh = plane
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/missile.gdshader")
	m.set_shader_parameter("color", color)
	m.set_shader_parameter("accent", HARD_CC_COLOR)
	m.set_shader_parameter("radius", radius_u)
	m.set_shader_parameter("trail", trail_u)
	m.set_shader_parameter("hard_cc", 1.0 if hard_cc else 0.0)
	body.material_override = m
	body.position = Vector3(0, 0.08, 0)
	root.add_child(body)
	var impact := MeshInstance3D.new()
	impact.name = "Impact"
	var sphere := SphereMesh.new()
	sphere.radius = radius_u * 2.4 * UNITS_TO_METERS
	sphere.height = radius_u * 4.8 * UNITS_TO_METERS
	impact.mesh = sphere
	impact.position = Vector3(0, 0.25, 0)
	impact.material_override = _unshaded(Color(1.0, 0.95, 0.7), 0.8)
	impact.visible = false
	root.add_child(impact)
	return root


func _update_missiles() -> void:
	var seen := {}
	for m in client.missiles():
		var key: int = m.key
		seen[key] = true
		var world := Vector3(m.pos.x * UNITS_TO_METERS, 0.0, m.pos.y * UNITS_TO_METERS)
		if not missile_nodes.has(key):
			var node := _make_missile(m.side, m.radius, m.hard_cc)
			add_child(node)
			missile_nodes[key] = node
			# Spawn streak (03a §7): enemy missiles are on T_input, their caster on T_interp; a
			# brief smear from the caster's drawn hand to the missile ties the two together.
			if m.side == "enemy" and remote_bodies.has(m.owner):
				_spawn_streak(remote_bodies[m.owner].position, world + Vector3(0, 0.3, 0), ENEMY_COLOR)
		var node: Node3D = missile_nodes[key]
		var dir: Vector2 = m.dir
		node.position = world
		node.rotation = Vector3(0, -atan2(dir.y, dir.x), 0)
		node.get_node("Body").visible = not m.impact
		node.get_node("Impact").visible = m.impact
		# Predicted to hit someone else first (03a §7): keep it visible, dimmed, until confirmed.
		var mat: ShaderMaterial = node.get_node("Body").material_override
		mat.set_shader_parameter("dim", 0.35 if m.unconfirmed else 1.0)
	for key in missile_nodes.keys():
		if not seen.has(key):
			missile_nodes[key].queue_free()
			missile_nodes.erase(key)


## Delayed ground AoE telegraph (05 §1): the outline appears at cast and the inside fills
## toward detonation, so you can read how long you have. Drawn at the true radius.
var area_nodes := {}                    # key -> Node3D


func _make_area(side: String, radius_u: float) -> Node3D:
	var node := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	var d := radius_u * 2.0 * UNITS_TO_METERS
	plane.size = Vector2(d, d)
	node.mesh = plane
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/area.gdshader")
	m.set_shader_parameter("color", _side_color(side))
	m.set_shader_parameter("radius", radius_u)
	node.material_override = m
	return node


func _update_areas() -> void:
	var seen := {}
	for a in client.areas():
		var key: int = a.key
		seen[key] = true
		if not area_nodes.has(key):
			var node := _make_area(a.side, a.radius)
			add_child(node)
			area_nodes[key] = node
		var node: MeshInstance3D = area_nodes[key]
		node.position = Vector3(a.center.x * UNITS_TO_METERS, 0.03, a.center.y * UNITS_TO_METERS)
		var mat: ShaderMaterial = node.material_override
		mat.set_shader_parameter("progress", a.progress)
		mat.set_shader_parameter("detonated", 1.0 if a.detonated else 0.0)
	for key in area_nodes.keys():
		if not seen.has(key):
			area_nodes[key].queue_free()
			area_nodes.erase(key)


var bolt_nodes := {}                    # key -> MeshInstance3D


func _update_bolts() -> void:
	var seen := {}
	for b in client.bolts():
		var key: int = b.key
		seen[key] = true
		if not bolt_nodes.has(key):
			var node := MeshInstance3D.new()
			var sphere := SphereMesh.new()
			sphere.radius = 0.09
			sphere.height = 0.18
			node.mesh = sphere
			node.material_override = _unshaded(_side_color(b.side).lightened(0.4))
			add_child(node)
			bolt_nodes[key] = node
		bolt_nodes[key].position = Vector3(b.pos.x * UNITS_TO_METERS, 1.0, b.pos.y * UNITS_TO_METERS)
	for key in bolt_nodes.keys():
		if not seen.has(key):
			bolt_nodes[key].queue_free()
			bolt_nodes.erase(key)


func _unit_world_pos(id: int):
	if id == client.own_unit_id():
		return own_body.position if own_body != null else null
	if remote_bodies.has(id):
		return remote_bodies[id].position
	return null


func _name_of(id: int) -> String:
	if id == client.own_unit_id():
		return "You"
	if remote_info.has(id):
		var c: String = remote_info[id].champion
		if c != "":
			return c
		return "Minion" if remote_info[id].minion else "Turret"
	return "Unit %d" % id


## Confirmed damage only (03a §7): numbers float up from the unit that took it.
func _update_combat_text(delta: float) -> void:
	for c in client.take_combat_text():
		var p = _unit_world_pos(c.target)
		if p == null:
			continue
		var total: float = c.amount + c.absorbed
		var color := Color(0.75, 0.55, 1.0) if c.kind == "magic" else Color(1.0, 0.65, 0.3)
		if c.target == client.own_unit_id():
			color = Color(1.0, 0.3, 0.3)
		floaters.append({ "pos": p, "text": "%d" % roundi(total), "color": color, "age": 0.0 })
	for n in client.take_notices():
		var text := ""
		if n.kind == "died":
			if not remote_info.has(n.unit) and n.unit != client.own_unit_id():
				continue
			var victim := _name_of(n.unit)
			if remote_info.has(n.unit) and remote_info[n.unit].minion:
				continue
			text = "%s killed %s" % [_name_of(n.killer), victim]
		elif n.kind == "blinked":
			_blink_marks(_to_world(n.from), _to_world(n.to))
			continue
		elif n.unit == client.own_unit_id():
			text = "You respawned"
		if text != "":
			notices.append({ "text": text, "age": 0.0 })
	for f in floaters:
		f.age += delta
	floaters = floaters.filter(func(f): return f.age < 0.9)
	for n in notices:
		n.age += delta
	notices = notices.filter(func(n): return n.age < 4.0)


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


func _screen(p: Vector3):
	if camera.is_position_behind(p):
		return null
	return camera.unproject_position(p)


## Health bars, damage numbers, the ability bar and the kill feed (2D, over the 3D view).
## Bar sizes follow the familiarity targets of R01 §6.
func _draw_overlay() -> void:
	var font := ThemeDB.fallback_font
	if own_body != null and own_body.visible:
		var hp: float = own_status.get("health", 0.0)
		var mx: float = own_status.get("max_health", 1.0)
		_draw_bar(own_body.position + Vector3(0, 1.25, 0), Vector2(104, 11), hp, mx, own_status.get("shield", 0.0), Color(0.3, 0.85, 0.35))
	for id in remote_info:
		var u: Dictionary = remote_info[id]
		if u.turret or not remote_bodies.has(id):
			continue
		var champ: bool = u.champion != ""
		var color := Color(0.3, 0.7, 0.95) if u.ally else Color(0.9, 0.25, 0.2)
		var size := Vector2(104, 11) if champ else Vector2(62, 6)
		var lift := 1.25 if champ else 0.6
		_draw_bar(remote_bodies[id].position + Vector3(0, lift, 0), size, u.health, u.max_health, u.shield, color)
	for f in floaters:
		var s = _screen(f.pos + Vector3(0, 1.5 + f.age * 0.8, 0))
		if s != null:
			var c: Color = f.color
			c.a = clampf(1.5 - f.age * 1.5, 0.0, 1.0)
			overlay.draw_string(font, s + Vector2(-40, 0), f.text, HORIZONTAL_ALIGNMENT_CENTER, 80, 20, c)
	_draw_ability_bar(font)
	var y := 120.0
	for n in notices:
		overlay.draw_string(font, Vector2(overlay.size.x - 420, y), n.text, HORIZONTAL_ALIGNMENT_RIGHT, 400, 18, Color(1, 1, 1, clampf(4.0 - n.age, 0.0, 1.0)))
		y += 24.0


func _draw_bar(world: Vector3, size: Vector2, hp: float, max_hp: float, shield: float, color: Color) -> void:
	var s = _screen(world)
	if s == null or max_hp <= 0.0:
		return
	var origin: Vector2 = s - Vector2(size.x / 2.0, size.y)
	var total := maxf(max_hp, hp + shield)
	overlay.draw_rect(Rect2(origin - Vector2(1, 1), size + Vector2(2, 2)), Color(0, 0, 0, 0.8))
	var w_hp := size.x * clampf(hp / total, 0.0, 1.0)
	overlay.draw_rect(Rect2(origin, Vector2(w_hp, size.y)), color)
	if shield > 0.0:
		var w_sh := size.x * clampf(shield / total, 0.0, 1.0)
		overlay.draw_rect(Rect2(origin + Vector2(w_hp, 0), Vector2(w_sh, size.y)), Color(0.95, 0.95, 0.95))


func _draw_ability_bar(font: Font) -> void:
	if own_status.is_empty() or not own_status.has("cooldowns"):
		return
	var names: Array = own_status.abilities
	var cds: Array = own_status.cooldowns
	var slot_w := 120.0
	var x0 := overlay.size.x / 2.0 - slot_w * 3.0
	var y0 := overlay.size.y - 92.0
	overlay.draw_rect(Rect2(x0 - 10, y0 - 34, slot_w * 6.0 + 20, 120), Color(0, 0, 0, 0.55))
	var hp: float = own_status.health
	var mx: float = own_status.max_health
	var header := "%s   %d / %d" % [own_status.champion, roundi(hp), roundi(mx)]
	if own_status.shield > 0.0:
		header += "  (+%d shield)" % roundi(own_status.shield)
	overlay.draw_string(font, Vector2(x0, y0 - 10), header, HORIZONTAL_ALIGNMENT_LEFT, -1, 18, Color.WHITE)
	for slot in 6:
		var x := x0 + slot * slot_w
		var cd: float = cds[slot]
		var ready := cd <= 0.0
		var box := Rect2(x, y0, slot_w - 8, 54)
		overlay.draw_rect(box, Color(0.18, 0.2, 0.24) if ready else Color(0.1, 0.1, 0.12))
		overlay.draw_rect(box, Color(0.45, 0.75, 1.0) if ready else Color(0.3, 0.3, 0.35), false, 2.0)
		overlay.draw_string(font, Vector2(x + 6, y0 + 20), SLOT_KEYS[slot], HORIZONTAL_ALIGNMENT_LEFT, -1, 18, Color.WHITE)
		var label := "" if ready else ("%.1f" % cd if cd < 10.0 else "%d" % ceili(cd))
		overlay.draw_string(font, Vector2(x + 30, y0 + 20), label, HORIZONTAL_ALIGNMENT_LEFT, -1, 18, Color(1, 0.85, 0.4))
		overlay.draw_string(font, Vector2(x + 6, y0 + 44), names[slot], HORIZONTAL_ALIGNMENT_LEFT, slot_w - 14, 13, Color(0.8, 0.85, 0.9))
	if own_status.get("dead", false):
		var msg := "Respawning in %.1f s" % own_status.respawn_in
		overlay.draw_rect(Rect2(Vector2.ZERO, overlay.size), Color(0.1, 0.1, 0.1, 0.35))
		overlay.draw_string(font, Vector2(0, overlay.size.y * 0.4), msg, HORIZONTAL_ALIGNMENT_CENTER, overlay.size.x, 36, Color.WHITE)
	if attack_move_armed:
		overlay.draw_string(font, Vector2(x0, y0 + 80), "Attack-move: left-click a point", HORIZONTAL_ALIGNMENT_LEFT, -1, 16, ENEMY_COLOR)
	if blind_state == "playing":
		var left := maxf(blind_seconds - client.blind_elapsed(), 0.0)
		var txt := "Blind round %d / %d   %d:%02d" % [client.blind_round() + 1, client.blind_rounds(), int(left) / 60, int(left) % 60]
		overlay.draw_string(font, Vector2(16, 34), txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Color.WHITE)


func _update_net_graph() -> void:
	net_label.visible = show_net_graph
	if not show_net_graph:
		return
	var phase := client.phase()
	if phase != "playing":
		net_label.text = "MFTR — %s…  %s" % [phase if phase != "" else "disconnected", client.last_error()]
		return
	var s: Dictionary = client.net_stats()
	net_label.text = "FPS %d   RTT %.0f ms   margin %.1f ms   interp %.0f ms\ncommands %d   late %d   corrections %d (last %.1f u)   on-screen correction %.1f u\nup %.1f KB   down %.1f KB   collision proxies %s\nenemy missiles %d   near-misses %d   ghost hits %d   phantom hits %d   K/D %d/%d\n[RMB] move / attack  [A+LMB] attack-move  [Q W E R] abilities  [D] Blink  [F] Barrier  [S] stop  [F1] net graph  [F2] proxies" % [
		Engine.get_frames_per_second(), s.rtt_ms, s.margin_ms, s.interp_ms,
		s.commands, s.late, s.corrections, s.last_correction, s.visible_correction,
		s.kb_up, s.kb_down, "ON" if proxies_enabled else "OFF",
		s.enemy_missiles, s.near_misses, s.ghost_hits, s.phantom_hits, s.kills, s.deaths,
	]


## ---- Blind playtest (03 §14) ----------------------------------------------------------------
## Rounds under hidden network conditions and A/B switches, rated by the tester. The net graph
## stays hidden; the condition is only written to the results file (`mftr-tools blind-report`).

var blind_enabled := false
var blind_seed := -1
var blind_rounds := 10
var blind_seconds := 60.0
var blind_auto := false
var blind_state := ""                   # "intro", "playing", "rating", "done"
var blind_panel: PanelContainer
var blind_title: Label
var blind_body: VBoxContainer
var blind_fair := -1
var blind_resp := 0
var blind_notes: LineEdit


func _blind_path() -> String:
	return OS.get_user_data_dir().path_join("blind_results.tsv")


func _update_blind() -> void:
	if not blind_enabled:
		return
	if blind_state == "":
		show_net_graph = false
		if blind_seed < 0:
			blind_seed = int(Time.get_unix_time_from_system()) % 1000000
		client.blind_begin(blind_seed, blind_rounds)
		_build_blind_panel()
		_blind_show("intro")
	elif blind_state == "playing" and client.blind_elapsed() >= blind_seconds:
		_blind_show("rating")
	if blind_auto and blind_state == "intro":
		_blind_start()
	elif blind_auto and blind_state == "rating":
		blind_fair = randi() % 2
		blind_resp = 1 + randi() % 5
		blind_notes.text = "auto"
		_blind_submit()


func _build_blind_panel() -> void:
	blind_panel = PanelContainer.new()
	blind_panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	blind_panel.custom_minimum_size = Vector2(560, 0)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 20)
	blind_panel.add_child(margin)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 12)
	margin.add_child(v)
	blind_title = Label.new()
	blind_title.add_theme_font_size_override("font_size", 24)
	v.add_child(blind_title)
	blind_body = VBoxContainer.new()
	blind_body.add_theme_constant_override("separation", 10)
	v.add_child(blind_body)
	overlay.get_parent().add_child(blind_panel)


func _clear_blind_body() -> void:
	for c in blind_body.get_children():
		c.queue_free()


func _label(text: String) -> Label:
	var l := Label.new()
	l.text = text
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	l.custom_minimum_size = Vector2(520, 0)
	return l


func _choice_row(options: Array, on_pick: Callable) -> HBoxContainer:
	var row := HBoxContainer.new()
	var group := ButtonGroup.new()
	for i in options.size():
		var b := Button.new()
		b.text = options[i]
		b.toggle_mode = true
		b.button_group = group
		b.custom_minimum_size = Vector2(80, 36)
		b.pressed.connect(on_pick.bind(i))
		row.add_child(b)
	return row


func _blind_show(state: String) -> void:
	blind_state = state
	var round: int = client.blind_round()
	var total: int = client.blind_rounds()
	_clear_blind_body()
	blind_panel.visible = state != "playing"
	if state == "intro":
		blind_title.text = "Blind playtest — %d rounds of %d s" % [total, int(blind_seconds)]
		blind_body.add_child(_label("Each round plays under a hidden network condition. Play normally: fight, and dodge every skillshot you can. After each round, say whether dodging felt fair and how responsive your champion felt. There are no right answers."))
		var start := Button.new()
		start.text = "Start round 1"
		start.pressed.connect(_blind_start)
		blind_body.add_child(start)
	elif state == "rating":
		blind_fair = -1
		blind_resp = 0
		blind_title.text = "Round %d of %d" % [round + 1, total]
		blind_body.add_child(_label("When you dodged (or failed to dodge), did the outcome feel fair?"))
		blind_body.add_child(_choice_row(["Fair", "Unfair"], func(i): blind_fair = 1 - i))
		blind_body.add_child(_label("How responsive did your champion feel? (1 = sluggish, 5 = instant)"))
		blind_body.add_child(_choice_row(["1", "2", "3", "4", "5"], func(i): blind_resp = i + 1))
		blind_notes = LineEdit.new()
		blind_notes.placeholder_text = "Anything you noticed (optional)"
		blind_body.add_child(blind_notes)
		var submit := Button.new()
		submit.text = "Submit"
		submit.pressed.connect(_blind_submit)
		blind_body.add_child(submit)
	elif state == "done":
		blind_title.text = "Thank you!"
		blind_body.add_child(_label("All rounds rated. Results were appended to:\n%s\nSend that file to the developers (it records the hidden conditions)." % _blind_path()))
		print("MFTR: blind playtest finished, results in ", _blind_path())


func _blind_start() -> void:
	client.blind_start_round()
	_blind_show("playing")


func _blind_submit() -> void:
	if blind_fair < 0 or blind_resp == 0:
		return  # both answers are required
	if not client.blind_rate(blind_fair == 1, blind_resp, blind_notes.text, _blind_path()):
		push_error("MFTR: %s" % client.last_error())
	if client.blind_round() < 0:
		_blind_show("done")
		if blind_auto:
			get_tree().quit()
	else:
		_blind_start()


## ---- Short-lived effects ----------------------------------------------------------------------
## Spawn streaks and Blink marks. Blink leaves a golden mark at its origin for ~1.5 s, so "where
## did they blink from" stays readable (R01 §5), and a brief burst where it lands.

var fx := []                            # { node: Node3D, age, life, grow }


func _add_fx(node: Node3D, life: float, grow: float) -> void:
	add_child(node)
	fx.append({ "node": node, "age": 0.0, "life": life, "grow": grow })


func _spawn_streak(from: Vector3, to: Vector3, color: Color) -> void:
	var v := to - from
	var length := Vector2(v.x, v.z).length()
	if length < 0.05:
		return
	var streak := MeshInstance3D.new()
	var box := BoxMesh.new()
	box.size = Vector3(length, 0.04, 0.12)
	streak.mesh = box
	streak.material_override = _unshaded(color.lightened(0.4), 0.7)
	streak.position = (from + to) * 0.5
	streak.rotation = Vector3(0, -atan2(v.z, v.x), 0)
	_add_fx(streak, 0.1, 0.0)


func _blink_marks(from: Vector3, to: Vector3) -> void:
	var origin := MeshInstance3D.new()
	var ring := TorusMesh.new()
	ring.inner_radius = 0.35
	ring.outer_radius = 0.5
	origin.mesh = ring
	origin.material_override = _unshaded(Color(1.0, 0.85, 0.35), 0.9)
	origin.position = Vector3(from.x, 0.05, from.z)
	_add_fx(origin, 1.5, 0.0)
	var burst := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = 0.4
	sphere.height = 0.8
	burst.mesh = sphere
	burst.material_override = _unshaded(Color(1.0, 0.95, 0.75), 0.8)
	burst.position = Vector3(to.x, 0.6, to.z)
	_add_fx(burst, 0.3, 2.5)


func _update_fx(delta: float) -> void:
	for f in fx:
		f.age += delta
		var t: float = f.age / f.life
		var node: MeshInstance3D = f.node
		var mat: StandardMaterial3D = node.material_override
		mat.albedo_color.a = clampf(1.0 - t, 0.0, 1.0) * 0.9
		if f.grow > 0.0:
			var s: float = 1.0 + f.grow * t
			node.scale = Vector3(s, s, s)
		if f.age >= f.life:
			node.queue_free()
	fx = fx.filter(func(f): return f.age < f.life)
