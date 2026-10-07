extends Node3D
## M1 Duel Sandbox client. Builds the scene in code, forwards input to the Rust MatchClient,
## and draws what it reports. No gameplay decisions are made here (04 §1).
##
## Without a server address the client opens its start menu: pick a server (remembered ones are
## listed with their game type), how to join (play, spectate or the blind playtest) and a champion.
## User args (after `--`) skip the menu: a server address (`host:port#fingerprint` pins the
## server's key, otherwise it is trusted on first use), `--champion NAME`,
## `--spectate` to watch (Tab cycles champions), `--shot-lobby` for a champion-select capture,
## `--shot <file.png>` / `--shot-at <seconds>` / `--shot-shop` for scripted screenshots, and the blind playtest
## options `--blind [seed]`, `--blind-rounds N`, `--blind-seconds S`, `--blind-auto`. `--menu-join`
## (scripted checks) opens the menu and joins the first remembered server through it.

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
var ground: MeshInstance3D
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
var match_banner := ""                  # "VICTORY" / "DEFEAT" when a Base falls
var match_banner_age := 0.0
var notices := []                       # kill feed: { text, age }


func _ready() -> void:
	_build_world()
	client = MatchClient.new()
	add_child(client)
	var address := ""
	_load_menu_choices()  # command-line flags below override them
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
		elif args[i] == "--shot-shop":
			_shot_shop = true
		elif args[i] == "--shot-lobby":
			_shot_lobby = true
		elif args[i] == "--menu-join":
			_menu_auto_join = true
		elif args[i] == "--spectate":
			_menu_mode = 1
			client.set_spectate(true)
		elif args[i] == "--blind-auto":
			blind_auto = true
		elif args[i] == "--champion" and i + 1 < args.size():
			if not client.set_champion(args[i + 1]):
				push_error("MFTR: unknown champion %s" % args[i + 1])
			_menu_champion = args[i + 1]
			i += 1
		else:
			address = args[i]
		i += 1
	if blind_enabled:
		_menu_mode = 2
	if address == "" or _back_to_menu:
		_back_to_menu = false
		_show_menu("")
	else:
		_connect(address)


## Join `address` (as set up: spectating, champion, blind playtest).
func _connect(address: String) -> void:
	_server_address = address
	_remembered = false
	# Reconnect: a session to this server from moments ago gets its champion back.
	var saved := _load_session()
	var recent: bool = Time.get_unix_time_from_system() - float(saved.get("at", 0.0)) < 55.0
	client.set_resume_token(saved.get("token", "") if saved.get("address", "") == address and recent else "")
	if not client.connect_to_server(address):
		_show_menu("Could not connect to %s: %s" % [address, client.last_error()])


## `--shot <file.png>`: scripted capture for automated visual checks. Waits until playing,
## casts an area and a skillshot, issues one move order, saves a screenshot and quits.
var _shot_path := ""
var _shot_timer := 0.0
var _shot_moved := false
var _shot_at := 1.05                     # `--shot-at <seconds>` after joining
var _shot_shop := false                  # `--shot-shop`: buy from the fountain, show the shop
var _shot_lobby := false                 # `--shot-lobby`: reroll in champion select, capture it
var _menu_auto_join := false             # `--menu-join`: join the first remembered server from the menu


func _update_shot(delta: float) -> void:
	if _shot_path != "" and _shot_lobby and client.phase() == "lobby":
		_shot_timer += delta
		if _shot_timer > 0.6 and _shot_timer - delta <= 0.6:
			client.lobby_reroll()
		elif _shot_timer > 1.6:
			get_viewport().get_texture().get_image().save_png(_shot_path)
			print("MFTR: saved screenshot to ", _shot_path)
			get_tree().quit()
		return
	if _shot_path == "" or client.phase() != "playing":
		return
	_shot_timer += delta
	# Everything aims toward the arena's center, whichever side we spawned on.
	var own := client.own_position()
	var size: Vector2 = client.map_geometry().size
	var inward := (size / 2.0 - own).normalized()
	var side := Vector2(-inward.y, inward.x)
	if not _shot_moved and _shot_timer > 1.0:
		for slot in 3:
			client.level_up(slot)  # ranked modes start with points to spend
		if _shot_shop:
			for item in [7, 1, 3]:  # Boots, Long Knife, Vital Crystal from the fountain
				client.buy(item)
			_toggle_shop()
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

	ground = MeshInstance3D.new()
	_size_ground(Vector2(ARENA_U, ARENA_U))
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
## Placeholder silhouettes: each champion gets an identity color and a shape that reads at
## gameplay zoom (05 §2: silhouette first). Real models come later.
const CHAMPION_COLORS := {
	"Ember": Color(0.62, 0.36, 0.22),
	"Vesper": Color(0.34, 0.42, 0.36),
	"Bastion": Color(0.45, 0.5, 0.58),
	"Rook": Color(0.55, 0.3, 0.27),
	"Lumen": Color(0.86, 0.82, 0.62),
	"Shade": Color(0.32, 0.24, 0.42),
}


func _part(parent: Node3D, mesh: Mesh, pos: Vector3, mat: Material, rot := Vector3.ZERO) -> MeshInstance3D:
	var n := MeshInstance3D.new()
	n.mesh = mesh
	n.position = pos
	n.rotation_degrees = rot
	n.material_override = mat
	parent.add_child(n)
	return n


func _make_champion(color: Color, champion: String) -> Node3D:
	var body := MeshInstance3D.new()
	# Identity color per champion; the team accent is a band at the feet plus the ring.
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/champion.gdshader")
	m.set_shader_parameter("base_color", CHAMPION_COLORS.get(champion, Color(0.5, 0.5, 0.5)))
	m.set_shader_parameter("team_accent", color)
	match champion:
		"Vesper":
			var cyl := CylinderMesh.new()
			cyl.top_radius = 0.22
			cyl.bottom_radius = 0.3
			cyl.height = 1.5
			body.mesh = cyl
			var cone := CylinderMesh.new()
			cone.top_radius = 0.0
			cone.bottom_radius = 0.28
			cone.height = 0.45
			_part(body, cone, Vector3(0, 0.95, 0), m)
		"Bastion":
			# Broad and blocky, with shoulder slabs: the tank reads as a wall.
			var box := BoxMesh.new()
			box.size = Vector3(0.95, 1.45, 0.7)
			body.mesh = box
			var slab := BoxMesh.new()
			slab.size = Vector3(0.4, 0.22, 0.8)
			_part(body, slab, Vector3(-0.62, 0.62, 0), m)
			_part(body, slab, Vector3(0.62, 0.62, 0), m)
		"Rook":
			# Stocky capsule with a big hammer head over the shoulder.
			var cap := CapsuleMesh.new()
			cap.radius = 0.46
			cap.height = 1.55
			body.mesh = cap
			var haft := CylinderMesh.new()
			haft.top_radius = 0.04
			haft.bottom_radius = 0.04
			haft.height = 1.3
			_part(body, haft, Vector3(0.5, 0.35, 0), _unshaded(Color(0.35, 0.25, 0.18)), Vector3(0, 0, -25))
			var head := BoxMesh.new()
			head.size = Vector3(0.42, 0.28, 0.28)
			_part(body, head, Vector3(0.78, 0.92, 0), m, Vector3(0, 0, -25))
		"Lumen":
			# Slender, with a glowing halo.
			var cyl := CylinderMesh.new()
			cyl.top_radius = 0.16
			cyl.bottom_radius = 0.34
			cyl.height = 1.55
			body.mesh = cyl
			var halo := TorusMesh.new()
			halo.inner_radius = 0.22
			halo.outer_radius = 0.28
			_part(body, halo, Vector3(0, 1.0, 0), _unshaded(Color(1.0, 0.95, 0.65)))
		"Shade":
			# A sharp three-sided blade of a body with two forward blades.
			var prism := CylinderMesh.new()
			prism.top_radius = 0.05
			prism.bottom_radius = 0.38
			prism.height = 1.65
			prism.radial_segments = 3
			body.mesh = prism
			var blade := BoxMesh.new()
			blade.size = Vector3(0.06, 0.06, 0.6)
			var steel := _unshaded(Color(0.8, 0.82, 0.9))
			_part(body, blade, Vector3(0.32, 0.0, 0.25), steel, Vector3(0, -15, 0))
			_part(body, blade, Vector3(-0.32, 0.0, 0.25), steel, Vector3(0, 15, 0))
		_:
			var capsule := CapsuleMesh.new()
			capsule.radius = CHAMPION_RADIUS_U * UNITS_TO_METERS * 0.6
			capsule.height = 1.7
			body.mesh = capsule
			var sphere := SphereMesh.new()
			sphere.radius = 0.14
			sphere.height = 0.28
			_part(body, sphere, Vector3(0.45, 0.55, 0), _unshaded(Color(1.0, 0.7, 0.3)))
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
	body.add_child(_make_slow_indicator())
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
	if event is InputEventKey and event.is_pressed() and not event.is_echo() and (event as InputEventKey).keycode == KEY_ESCAPE:
		if menu_panel == null:
			_toggle_pause_menu()
		return
	if menu_panel != null or (pause_panel != null and pause_panel.visible):
		return
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
			# Ctrl + Q/W/E/R spends an ability point (01 §13).
			if slot < 4 and event is InputEventKey and (event as InputEventKey).ctrl_pressed:
				client.level_up(slot)
				return
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
	elif event.is_action_pressed("toggle_shop") and not client.is_spectator():
		_toggle_shop()
	elif event.is_action_pressed("spectate_next") and client.is_spectator():
		_spectate_next()
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
		print("MFTR phase: %s -> %s (unit %d, server key %s)" % [_last_phase, phase, client.own_unit_id(), client.server_fingerprint()])
		_last_phase = phase
	var playing := phase == "playing"
	if phase == "lobby" and _last_phase_seen in ["joining", "playing"]:
		_reset_match_view()  # the match ended; champion select again
	_last_phase_seen = phase
	if phase in ["lobby", "joining", "playing"]:
		_remember_server()
	_update_connecting(phase)
	if playing and not _map_built:
		_build_map()
	if playing:
		_update_blind()
	_update_draft(playing)
	own_status = client.own_status() if playing else {}
	_update_shop(delta)
	_update_lobby(delta, phase)
	_save_session(delta, playing)
	var spectating: bool = playing and client.is_spectator()
	if spectating:
		_update_spectator_camera()
	if playing and own_body == null and not spectating:
		own_champion = own_status.get("champion", "")
		own_body = _make_champion(OWN_COLOR, own_champion)
		add_child(own_body)
	if own_body != null:
		var dead: bool = own_status.get("dead", false)
		own_body.visible = playing and not dead
		var own := _to_world(client.own_position())
		own_body.position = own
		# Titan and Pebble: the model grows and shrinks with the hitbox (honest hitboxes).
		own_body.scale = Vector3.ONE * (float(own_status.get("hitbox", CHAMPION_RADIUS_U)) / CHAMPION_RADIUS_U)
		_place_camera(own)
		_show_statuses(own_body, own_status.get("stunned", false), own_status.get("rooted", false), own_status.get("shield", 0.0), own_status.get("slowed", false))
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


func _show_statuses(body: Node3D, stunned: bool, rooted: bool, shield: float, slowed := false) -> void:
	body.get_node("Stun").visible = stunned
	body.get_node("Root").visible = rooted
	body.get_node("Shield").visible = shield > 0.0
	body.get_node("Slow").visible = slowed


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
			var team_color := ALLY_COLOR if u.ally else ENEMY_COLOR
			if u.kind == "gatehouse":
				b = _make_gatehouse(team_color)
			elif u.kind == "base":
				b = _make_base(team_color)
			elif u.kind == "relic":
				b = _make_relic()
			elif u.turret:
				b = _make_turret(team_color, u.radius)
			elif u.minion:
				b = _make_minion(MINION_BLUE if u.ally else MINION_RED, u.radius)
			else:
				b = _make_champion(ALLY_COLOR if u.ally else ENEMY_COLOR, u.champion)
			b.add_child(_make_windup_indicator())
			if not b.has_node("Stun"):
				b.add_child(_make_stun_indicator())
				b.add_child(_make_root_indicator())
				b.add_child(_make_shield_bubble())
			if not b.has_node("Slow"):
				b.add_child(_make_slow_indicator())
			add_child(b)
			remote_bodies[id] = b
		var body: Node3D = remote_bodies[id]
		var p := _to_world(u.pos)
		if u.minion:
			p.y = 0.45
		elif u.turret:
			p.y = 1.2
		elif u.kind in ["gatehouse", "base", "relic"]:
			p.y = 0.0
		body.position = p
		if u.champion != "":
			body.scale = Vector3.ONE * (float(u.gameplay_radius) / CHAMPION_RADIUS_U)
		if body.has_node("Protected"):
			body.get_node("Protected").visible = u.protected
		_show_windup(body, u.get("windup", -1.0), u.get("windup_dir", Vector2.ZERO))
		_show_statuses(body, u.stunned, u.rooted, u.shield, u.get("slowed", false))
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


func _size_ground(size_u: Vector2) -> void:
	var plane := PlaneMesh.new()
	plane.size = size_u * UNITS_TO_METERS
	ground.mesh = plane
	ground.position = Vector3(size_u.x, 0, size_u.y) * (UNITS_TO_METERS / 2.0)


func _build_map() -> void:
	_map_built = true
	var geo: Dictionary = client.map_geometry()
	var size: Vector2 = geo.size
	_size_ground(size)
	if size.x != size.y:
		# A lane map: the dirt lane runs along its middle.
		var gm: ShaderMaterial = ground.material_override
		gm.set_shader_parameter("lane_mode", 1.0)
		gm.set_shader_parameter("lane_z", size.y / 2.0 * UNITS_TO_METERS)
		gm.set_shader_parameter("lane_width", 13.0)
	for f in geo.fountains:
		var disk := MeshInstance3D.new()
		var cyl := CylinderMesh.new()
		cyl.top_radius = f.radius * UNITS_TO_METERS
		cyl.bottom_radius = cyl.top_radius
		cyl.height = 0.04
		disk.mesh = cyl
		disk.material_override = _unshaded(ALLY_COLOR if f.ally else ENEMY_COLOR, 0.18)
		disk.position = Vector3(f.center.x * UNITS_TO_METERS, 0.02, f.center.y * UNITS_TO_METERS)
		add_child(disk)
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
	body.material_override = _structure_material(color)
	var crown := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = 0.28
	sphere.height = 0.56
	crown.mesh = sphere
	crown.position = Vector3(0, 1.4, 0)
	crown.material_override = _unshaded(color.lightened(0.3))
	body.add_child(crown)
	body.add_child(_make_protected_dome(1.1, 2.0))
	return body


## Structures share the wall rock with a team-colored accent (05 §1 team color language).
func _structure_material(color: Color) -> ShaderMaterial:
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/champion.gdshader")
	m.set_shader_parameter("base_color", Color(0.36, 0.35, 0.4))
	m.set_shader_parameter("team_accent", color)
	m.set_shader_parameter("accent_height", -0.7)
	return m


## A faint dome over a structure that can't be hurt yet (an earlier one in its lane stands).
func _make_protected_dome(radius_m: float, lift: float) -> MeshInstance3D:
	var dome := MeshInstance3D.new()
	dome.name = "Protected"
	var sphere := SphereMesh.new()
	sphere.radius = radius_m
	sphere.height = radius_m * 2.0
	dome.mesh = sphere
	dome.position = Vector3(0, lift - 1.2, 0)
	dome.material_override = _unshaded(Color(0.85, 0.9, 1.0), 0.12)
	dome.visible = false
	return dome


func _make_gatehouse(color: Color) -> Node3D:
	var root := Node3D.new()
	for side in [-1.0, 1.0]:
		var pillar := MeshInstance3D.new()
		var box := BoxMesh.new()
		box.size = Vector3(0.8, 2.2, 0.8)
		pillar.mesh = box
		pillar.position = Vector3(0, 1.1, side * 0.9)
		pillar.material_override = _structure_material(color)
		root.add_child(pillar)
	var lintel := MeshInstance3D.new()
	var top := BoxMesh.new()
	top.size = Vector3(0.9, 0.5, 2.6)
	lintel.mesh = top
	lintel.position = Vector3(0, 2.4, 0)
	lintel.material_override = _structure_material(color)
	root.add_child(lintel)
	var core := MeshInstance3D.new()
	var orb := SphereMesh.new()
	orb.radius = 0.35
	orb.height = 0.7
	core.mesh = orb
	core.position = Vector3(0, 1.3, 0)
	core.material_override = _unshaded(color.lightened(0.4))
	root.add_child(core)
	var dome := _make_protected_dome(1.6, 2.4)
	root.add_child(dome)
	return root


func _make_base(color: Color) -> Node3D:
	var root := Node3D.new()
	var plinth := MeshInstance3D.new()
	var cyl := CylinderMesh.new()
	cyl.top_radius = 1.6
	cyl.bottom_radius = 2.0
	cyl.height = 0.6
	plinth.mesh = cyl
	plinth.position = Vector3(0, 0.3, 0)
	plinth.material_override = _structure_material(color)
	root.add_child(plinth)
	var crystal := MeshInstance3D.new()
	var prism := CylinderMesh.new()
	prism.top_radius = 0.0
	prism.bottom_radius = 0.9
	prism.height = 3.2
	prism.radial_segments = 6
	crystal.mesh = prism
	crystal.position = Vector3(0, 2.2, 0)
	crystal.material_override = _unshaded(color.lightened(0.2), 0.9)
	root.add_child(crystal)
	root.add_child(_make_protected_dome(2.4, 2.4))
	return root


func _make_relic() -> Node3D:
	var orb := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = 0.3
	sphere.height = 0.6
	orb.mesh = sphere
	orb.material_override = _unshaded(Color(0.45, 1.0, 0.55), 0.85)
	var glow := MeshInstance3D.new()
	var ring := TorusMesh.new()
	ring.inner_radius = 0.4
	ring.outer_radius = 0.5
	glow.mesh = ring
	glow.position = Vector3(0, -0.55, 0)
	glow.material_override = _unshaded(Color(0.45, 1.0, 0.55), 0.6)
	orb.add_child(glow)
	var holder := Node3D.new()
	orb.position = Vector3(0, 0.6, 0)
	holder.add_child(orb)
	return holder


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
## Slowed: a cold blue ring of chevrons at the feet (not hard CC, so not gold).
func _make_slow_indicator() -> MeshInstance3D:
	var ring := MeshInstance3D.new()
	ring.name = "Slow"
	var torus := TorusMesh.new()
	torus.inner_radius = 0.5
	torus.outer_radius = 0.56
	torus.rings = 6
	torus.ring_segments = 6
	ring.mesh = torus
	ring.position = Vector3(0, -0.78, 0)
	ring.material_override = _unshaded(Color(0.45, 0.75, 1.0), 0.8)
	ring.visible = false
	return ring


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


func _make_area(side: String, radius_u: float, hard_cc: bool) -> Node3D:
	var node := MeshInstance3D.new()
	var plane := PlaneMesh.new()
	var d := radius_u * 2.0 * UNITS_TO_METERS
	plane.size = Vector2(d, d)
	node.mesh = plane
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/area.gdshader")
	m.set_shader_parameter("color", _side_color(side))
	m.set_shader_parameter("radius", radius_u)
	m.set_shader_parameter("rim_color", HARD_CC_COLOR)
	m.set_shader_parameter("hard_cc", 1.0 if hard_cc else 0.0)
	node.material_override = m
	return node


func _update_areas() -> void:
	var seen := {}
	for a in client.areas():
		var key: int = a.key
		seen[key] = true
		if not area_nodes.has(key):
			var node := _make_area(a.side, a.radius, a.get("hard_cc", false))
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
		var text := "%d" % roundi(total)
		if c.heal:
			color = Color(0.4, 1.0, 0.45)
			text = "+" + text
		floaters.append({ "pos": p, "text": text, "color": color, "age": 0.0 })
	for n in client.take_notices():
		var text := ""
		if n.kind == "died":
			if not remote_info.has(n.unit) and n.unit != client.own_unit_id():
				continue
			var victim := _name_of(n.unit)
			if remote_info.has(n.unit) and remote_info[n.unit].minion:
				continue
			text = "%s killed %s" % [_name_of(n.killer), victim]
		elif n.kind == "reward":
			if n.gold >= 1.0 and own_body != null:
				floaters.append({ "pos": own_body.position + Vector3(0.6, 0, 0), "text": "+%dg" % roundi(n.gold), "color": Color(1.0, 0.85, 0.3), "age": 0.0 })
			continue
		elif n.kind == "match_ended":
			match_banner = "VICTORY" if n.won else "DEFEAT"
			match_banner_age = 0.0
			continue
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
	match_banner_age += delta
	if match_banner != "" and match_banner_age > 10.0:
		match_banner = ""
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
	if client.phase() == "playing" and client.is_spectator():
		var who: String = remote_info[spectate_target].champion if remote_info.has(spectate_target) else "the map"
		overlay.draw_string(font, Vector2(0, overlay.size.y - 40), "Spectating %s   —   Tab: next champion" % who, HORIZONTAL_ALIGNMENT_CENTER, overlay.size.x, 22, Color.WHITE)
	if own_body != null and own_body.visible:
		var hp: float = own_status.get("health", 0.0)
		var mx: float = own_status.get("max_health", 1.0)
		_draw_bar(own_body.position + Vector3(0, 1.25, 0), Vector2(104, 11), hp, mx, own_status.get("shield", 0.0), Color(0.3, 0.85, 0.35))
	for id in remote_info:
		var u: Dictionary = remote_info[id]
		if u.kind == "relic" or not remote_bodies.has(id):
			continue
		var champ: bool = u.champion != ""
		var structure: bool = u.kind in ["turret", "gatehouse", "base"]
		var color := Color(0.3, 0.7, 0.95) if u.ally else Color(0.9, 0.25, 0.2)
		var size := Vector2(104, 11) if champ else (Vector2(150, 10) if structure else Vector2(62, 6))
		var lift := 1.25 if champ else (2.6 if structure else 0.6)
		_draw_bar(remote_bodies[id].position + Vector3(0, lift, 0), size, u.health, u.max_health, u.shield, color)
		if champ and u.level > 0:
			var sp = _screen(remote_bodies[id].position + Vector3(0, lift, 0))
			if sp != null:
				overlay.draw_string(font, sp + Vector2(-size.x / 2.0 - 26, 0), "%d" % u.level, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, Color.WHITE)
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
	if own_status.get("ranked", false):
		header = "Lv %d  %s   %d / %d     %d gold" % [own_status.level, own_status.champion, roundi(hp), roundi(mx), own_status.gold]
		var xp_frac := float(own_status.xp) / maxf(float(own_status.xp_next), 1.0)
		overlay.draw_rect(Rect2(x0 - 10, y0 - 38, (slot_w * 6.0 + 20) * xp_frac, 4), Color(0.6, 0.45, 1.0))
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
		if slot < 4 and own_status.get("ranked", false):
			var rank: int = own_status.ranks[slot]
			var max_pips := 3 if slot == 3 else 5
			for i in max_pips:
				var c := Color(1.0, 0.8, 0.3) if i < rank else Color(0.3, 0.3, 0.35)
				overlay.draw_rect(Rect2(x + 6 + i * 12, y0 + 50, 9, 3), c)
			if rank == 0:
				overlay.draw_rect(box, Color(0, 0, 0, 0.55))
			if own_status.can_rank[slot]:
				overlay.draw_string(font, Vector2(x + slot_w - 30, y0 + 20), "+", HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Color(1.0, 0.85, 0.3))
	if own_status.get("ranked", false) and own_status.has("items"):
		_draw_inventory(font, Vector2(x0 + slot_w * 6.0 + 24, y0 - 34))
	if match_banner != "":
		var c := Color(0.45, 0.8, 1.0) if match_banner == "VICTORY" else Color(1.0, 0.4, 0.35)
		overlay.draw_string(font, Vector2(0, overlay.size.y * 0.3), match_banner, HORIZONTAL_ALIGNMENT_CENTER, overlay.size.x, 72, c)
		overlay.draw_string(font, Vector2(0, overlay.size.y * 0.3 + 50), "A new match starts shortly", HORIZONTAL_ALIGNMENT_CENTER, overlay.size.x, 22, Color.WHITE)
	if own_status.get("dead", false):
		var msg := "Respawning in %.1f s" % own_status.respawn_in
		overlay.draw_rect(Rect2(Vector2.ZERO, overlay.size), Color(0.1, 0.1, 0.1, 0.35))
		overlay.draw_string(font, Vector2(0, overlay.size.y * 0.4), msg, HORIZONTAL_ALIGNMENT_CENTER, overlay.size.x, 36, Color.WHITE)
	if attack_move_armed:
		overlay.draw_string(font, Vector2(x0, y0 + 80), "Attack-move: left-click a point", HORIZONTAL_ALIGNMENT_LEFT, -1, 16, ENEMY_COLOR)
	elif own_status.get("points", 0) > 0:
		overlay.draw_string(font, Vector2(x0, y0 + 80), "%d ability point(s): Ctrl + Q/W/E/R" % own_status.points, HORIZONTAL_ALIGNMENT_LEFT, -1, 16, Color(1.0, 0.85, 0.3))
	elif own_status.get("ranked", false) and client.can_shop():
		overlay.draw_string(font, Vector2(x0, y0 + 80), "[P] shop", HORIZONTAL_ALIGNMENT_LEFT, -1, 16, Color(1.0, 0.85, 0.3))
	if blind_state == "playing":
		var left := maxf(blind_seconds - client.blind_elapsed(), 0.0)
		var txt := "Blind round %d / %d   %d:%02d" % [client.blind_round() + 1, client.blind_rounds(), int(left) / 60, int(left) % 60]
		overlay.draw_string(font, Vector2(16, 34), txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Color.WHITE)


func _update_net_graph() -> void:
	var phase := client.phase()
	# The start menu and the connecting panel say it all.
	net_label.visible = show_net_graph and phase not in ["", "connecting"]
	if not net_label.visible:
		return
	if phase != "playing":
		net_label.text = "MFTR — %s…  %s" % [phase if phase != "" else "disconnected", client.last_error()]
		return
	var s: Dictionary = client.net_stats()
	net_label.text = "FPS %d   RTT %.0f ms   margin %.1f ms   interp %.0f ms\ncommands %d   late %d   corrections %d (last %.1f u)   on-screen correction %.1f u\nup %.1f KB   down %.1f KB   collision proxies %s\nenemy missiles %d   near-misses %d   ghost hits %d   phantom hits %d   K/D %d/%d\n[RMB] move / attack  [A+LMB] attack-move  [Q W E R] abilities  [D] Blink  [F] Barrier  [S] stop  [P] shop  [F1] net graph  [F2] proxies" % [
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


## ---- Shop (01 §11) ------------------------------------------------------------------------------
## Opens with P anywhere; buying and selling only work while dead or in the own fountain (the
## sim decides, the panel just greys things out). Click an item to buy it (owned components are
## used and discounted), click an inventory slot to sell it for 70%. Undo works until you leave.

var shop_panel: PanelContainer
var shop_title: Label
var shop_grid: GridContainer
var shop_inventory: HBoxContainer
var shop_stats: Label
var shop_undo: Button
var shop_buttons := {}                  # item id -> Button
var shop_slot_buttons := []
var item_names := {}                    # item id -> name
var _shop_refresh := 0.0


func _toggle_shop() -> void:
	if shop_panel == null:
		_build_shop()
	shop_panel.visible = not shop_panel.visible
	_shop_refresh = 0.0


func _build_shop() -> void:
	shop_panel = PanelContainer.new()
	shop_panel.set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	shop_panel.position = Vector2(16, 160)
	shop_panel.custom_minimum_size = Vector2(800, 0)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 14)
	shop_panel.add_child(margin)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 6)
	margin.add_child(v)
	shop_title = Label.new()
	shop_title.add_theme_font_size_override("font_size", 22)
	v.add_child(shop_title)
	var tiers := ["Components", "Upgrades and boots", "Legendary"]
	var catalog: Array = client.shop_catalog()
	for it in catalog:
		item_names[it.id] = it.name
	for tier in 3:
		var l := Label.new()
		l.text = tiers[tier]
		l.add_theme_color_override("font_color", Color(1.0, 0.85, 0.4))
		v.add_child(l)
		shop_grid = GridContainer.new()
		shop_grid.columns = 5
		shop_grid.add_theme_constant_override("h_separation", 6)
		shop_grid.add_theme_constant_override("v_separation", 6)
		v.add_child(shop_grid)
		for it in catalog:
			if it.tier != tier:
				continue
			var b := Button.new()
			b.custom_minimum_size = Vector2(150, 40)
			b.add_theme_font_size_override("font_size", 12)
			b.clip_text = true
			b.focus_mode = Control.FOCUS_NONE
			b.pressed.connect(func(): client.buy(it.id); _shop_refresh = 0.0)
			shop_grid.add_child(b)
			shop_buttons[it.id] = b
	var inv_label := Label.new()
	inv_label.text = "Inventory (click to sell for 70%)"
	inv_label.add_theme_color_override("font_color", Color(1.0, 0.85, 0.4))
	v.add_child(inv_label)
	shop_inventory = HBoxContainer.new()
	shop_inventory.add_theme_constant_override("separation", 6)
	v.add_child(shop_inventory)
	for slot in 6:
		var b := Button.new()
		b.custom_minimum_size = Vector2(124, 30)
		b.add_theme_font_size_override("font_size", 12)
		b.clip_text = true
		b.focus_mode = Control.FOCUS_NONE
		b.pressed.connect(func(): client.sell(slot); _shop_refresh = 0.0)
		shop_inventory.add_child(b)
		shop_slot_buttons.append(b)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 16)
	v.add_child(row)
	shop_undo = Button.new()
	shop_undo.text = "Undo"
	shop_undo.focus_mode = Control.FOCUS_NONE
	shop_undo.custom_minimum_size = Vector2(90, 30)
	shop_undo.pressed.connect(func(): client.undo_trade(); _shop_refresh = 0.0)
	row.add_child(shop_undo)
	shop_stats = Label.new()
	shop_stats.add_theme_color_override("font_color", Color(0.8, 0.85, 0.9))
	row.add_child(shop_stats)
	shop_panel.visible = false
	overlay.get_parent().add_child(shop_panel)


func _update_shop(delta: float) -> void:
	if shop_panel == null or not shop_panel.visible:
		return
	_shop_refresh -= delta
	if _shop_refresh > 0.0:
		return
	_shop_refresh = 0.15
	var open: bool = client.can_shop()
	var gold: int = own_status.get("gold", 0)
	shop_title.text = "Shop — %d gold%s" % [gold, "" if open else "   (closed: return to your fountain, or shop while dead)"]
	for it in client.shop_catalog():
		var b: Button = shop_buttons[it.id]
		var price: int = it.price
		b.text = "%s  %d\n%s" % [it.name, price, it.stats]
		b.tooltip_text = "%s (%d total)\n%s" % [it.name, it.cost, it.stats]
		if it.recipe.size() > 0:
			var parts := []
			for r in it.recipe:
				parts.append(item_names.get(r, "?"))
			b.tooltip_text += "\nBuilds from: " + ", ".join(parts)
		b.disabled = not open or not it.affordable
		b.modulate = Color(0.75, 1.0, 0.75) if it.owned else Color.WHITE
	var inv: Array = own_status.get("items", [])
	for slot in shop_slot_buttons.size():
		var id: int = inv[slot] if slot < inv.size() else 0
		var b: Button = shop_slot_buttons[slot]
		b.text = item_names.get(id, "—") if id != 0 else "—"
		b.disabled = not open or id == 0
	shop_undo.disabled = not open or not own_status.get("can_undo", false)
	if not own_status.is_empty() and own_status.has("attack_damage"):
		shop_stats.text = "AD %d   AP %d   armor %d   MR %d   AS %.2f   MS %d   haste %d   HP %d" % [
			roundi(own_status.attack_damage), roundi(own_status.ability_power), roundi(own_status.armor),
			roundi(own_status.magic_resist), own_status.attack_speed, roundi(own_status.move_speed),
			roundi(own_status.ability_haste), roundi(own_status.max_health),
		]


func _draw_inventory(font: Font, origin: Vector2) -> void:
	if item_names.is_empty():
		for it in client.shop_catalog():
			item_names[it.id] = it.name
	var inv: Array = own_status.items
	overlay.draw_rect(Rect2(origin - Vector2(8, 0), Vector2(3 * 92 + 12, 120)), Color(0, 0, 0, 0.55))
	for slot in 6:
		var p := origin + Vector2((slot % 3) * 92, 24 + (slot / 3) * 46)
		var box := Rect2(p, Vector2(86, 40))
		var id: int = inv[slot]
		overlay.draw_rect(box, Color(0.18, 0.2, 0.24) if id != 0 else Color(0.1, 0.1, 0.12))
		overlay.draw_rect(box, Color(0.85, 0.7, 0.35) if id != 0 else Color(0.3, 0.3, 0.35), false, 1.5)
		if id != 0:
			overlay.draw_string(font, p + Vector2(4, 24), item_names.get(id, "?"), HORIZONTAL_ALIGNMENT_LEFT, 80, 12, Color(0.9, 0.9, 0.95))
	# ARAM: Mayhem: the augments held, above the inventory.
	var held: Array = own_status.get("augments", [])
	for i in held.size():
		var a: Dictionary = held[i]
		var at := origin + Vector2(0, -10 - 18 * (held.size() - 1 - i))
		overlay.draw_string(font, at, "◆ " + a.name, HORIZONTAL_ALIGNMENT_LEFT, -1, 14, _tier_color(a.tier))


## ---- Session, champion select and spectating (M2 slice 5) --------------------------------------

var _server_address := ""
var _session_save := 0.0


func _session_path() -> String:
	return "user://session.cfg"


func _load_session() -> Dictionary:
	var cfg := ConfigFile.new()
	if cfg.load(_session_path()) != OK:
		return {}
	return {"address": cfg.get_value("session", "address", ""), "token": cfg.get_value("session", "token", ""), "at": cfg.get_value("session", "at", 0.0)}


## While playing, remember the token (and when) so a restarted client can take its champion back.
func _save_session(delta: float, playing: bool) -> void:
	_session_save -= delta
	if not playing or client.is_spectator() or _session_save > 0.0:
		return
	_session_save = 2.0
	var token: String = client.session_token()
	if token == "":
		return
	var cfg := ConfigFile.new()
	cfg.set_value("session", "address", _server_address)
	cfg.set_value("session", "token", token)
	cfg.set_value("session", "at", Time.get_unix_time_from_system())
	cfg.save(_session_path())


var lobby_panel: PanelContainer
var lobby_box: VBoxContainer
var _lobby_refresh := 0.0
var _lobby_ready := false


func _update_lobby(delta: float, phase: String) -> void:
	if phase != "lobby":
		if lobby_panel != null:
			lobby_panel.queue_free()
			lobby_panel = null
		return
	if lobby_panel == null:
		lobby_panel = PanelContainer.new()
		lobby_panel.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
		lobby_panel.custom_minimum_size = Vector2(760, 0)
		var margin := MarginContainer.new()
		for side in ["left", "right", "top", "bottom"]:
			margin.add_theme_constant_override("margin_" + side, 18)
		lobby_panel.add_child(margin)
		lobby_box = VBoxContainer.new()
		lobby_box.add_theme_constant_override("separation", 10)
		margin.add_child(lobby_box)
		overlay.get_parent().add_child(lobby_panel)
	_lobby_refresh -= delta
	if _lobby_refresh > 0.0:
		return
	_lobby_refresh = 0.2
	lobby_panel.position = (overlay.size - lobby_panel.size) / 2.0
	var l: Dictionary = client.lobby_state()
	if l.is_empty():
		return
	for c in lobby_box.get_children():
		c.queue_free()
	var title := Label.new()
	title.text = "Champion select — ARAM all random   (starts in %d s)" % ceili(l.starts_in)
	title.add_theme_font_size_override("font_size", 22)
	lobby_box.add_child(title)
	var cols := HBoxContainer.new()
	cols.add_theme_constant_override("separation", 40)
	lobby_box.add_child(cols)
	var me := {}
	for team in ["blue", "red"]:
		var col := VBoxContainer.new()
		col.custom_minimum_size = Vector2(330, 0)
		var head := Label.new()
		head.text = "Blue team" if team == "blue" else "Red team"
		head.add_theme_color_override("font_color", OWN_COLOR if team == "blue" else ENEMY_COLOR)
		col.add_child(head)
		for s in l.slots:
			if s.team != team:
				continue
			if s.you:
				me = s
			var row := Label.new()
			var who := "You" if s.you else ("Bot" if s.bot else "Player %d" % s.player)
			row.text = "%s  %s  —  %s%s" % ["✔" if s.ready else "  ", s.champion, who, ("  (%d rerolls)" % s.rerolls) if s.you else ""]
			if s.you:
				row.add_theme_color_override("font_color", Color(1.0, 0.85, 0.4))
			col.add_child(row)
		cols.add_child(col)
	var actions := HBoxContainer.new()
	actions.add_theme_constant_override("separation", 12)
	lobby_box.add_child(actions)
	var reroll := Button.new()
	reroll.text = "Reroll (%d)" % me.get("rerolls", 0)
	reroll.disabled = me.get("rerolls", 0) == 0
	reroll.focus_mode = Control.FOCUS_NONE
	reroll.pressed.connect(func(): client.lobby_reroll(); _lobby_refresh = 0.0)
	actions.add_child(reroll)
	var ready := Button.new()
	_lobby_ready = me.get("ready", false)
	ready.text = "Not ready" if _lobby_ready else "Ready"
	ready.focus_mode = Control.FOCUS_NONE
	ready.pressed.connect(func(): client.lobby_ready(not _lobby_ready); _lobby_refresh = 0.0)
	actions.add_child(ready)
	if l.bench.size() > 0:
		var bench := HBoxContainer.new()
		bench.add_theme_constant_override("separation", 8)
		var label := Label.new()
		label.text = "Team bench:"
		bench.add_child(label)
		for name in l.bench:
			var b := Button.new()
			b.text = "Take %s" % name
			b.focus_mode = Control.FOCUS_NONE
			b.pressed.connect(func(): client.lobby_take(name); _lobby_refresh = 0.0)
			bench.add_child(b)
		lobby_box.add_child(bench)


var spectate_target := -1


func _spectate_next() -> void:
	var ids := []
	for id in remote_info:
		if remote_info[id].champion != "":
			ids.append(id)
	ids.sort()
	if ids.is_empty():
		return
	var i := ids.find(spectate_target)
	spectate_target = ids[(i + 1) % ids.size()]


## Spectators follow a champion (Tab: the next one), or look at the middle of the map.
func _update_spectator_camera() -> void:
	if not remote_info.has(spectate_target):
		spectate_target = -1
		_spectate_next()
	var target: Vector2 = client.map_geometry().size / 2.0
	if remote_info.has(spectate_target):
		target = remote_info[spectate_target].pos
	_place_camera(_to_world(target))


## ---- Start menu, pause menu, remembered servers --------------------------------------------------
## The start menu picks a server and how to join it. Servers that were joined are remembered in
## `user://servers.cfg` with their game type, most recent first, so playing again is one click.
## Esc opens a small menu to leave the server (back to the start menu) at any time.

const SERVERS_PATH := "user://servers.cfg"
const MAX_SERVERS := 10
const JOIN_MODES := ["Play", "Spectate", "Blind playtest"]

static var _back_to_menu := false       # leaving a server reloads the scene into the menu
var menu_panel: PanelContainer
var menu_address: LineEdit
var _menu_mode := 0                     # index into JOIN_MODES
var _menu_champion := ""                # "" = the server picks
var _remembered := false                # this connection's server is saved in the list
var _last_phase_seen := ""
var pause_panel: PanelContainer
var connecting_panel: PanelContainer
var connecting_label: Label


func _load_servers() -> Array:
	var cfg := ConfigFile.new()
	if cfg.load(SERVERS_PATH) != OK:
		return []
	return cfg.get_value("servers", "list", [])


func _load_menu_choices() -> void:
	var cfg := ConfigFile.new()
	if cfg.load(SERVERS_PATH) == OK:
		_menu_mode = cfg.get_value("menu", "mode", 0)
		_menu_champion = cfg.get_value("menu", "champion", "")


## The list, and with `choices` the menu's join mode and champion too.
func _save_servers(list: Array, choices := false) -> void:
	var cfg := ConfigFile.new()
	cfg.load(SERVERS_PATH)
	cfg.set_value("servers", "list", list.slice(0, MAX_SERVERS))
	if choices:
		cfg.set_value("menu", "mode", _menu_mode)
		cfg.set_value("menu", "champion", _menu_champion)
	cfg.save(SERVERS_PATH)


## Once a server answers (champion select or a welcome), put it first in the list.
func _remember_server() -> void:
	if _remembered:
		return
	var game: String = client.game_type()
	if game == "":
		return
	_remembered = true
	var list := _load_servers().filter(func(e): return e.get("address", "") != _server_address)
	list.push_front({"address": _server_address, "game": game, "at": Time.get_unix_time_from_system()})
	_save_servers(list)


func _forget_server(address: String) -> void:
	_save_servers(_load_servers().filter(func(e): return e.get("address", "") != address))
	_show_menu("")


func _ago(at: float) -> String:
	var s := Time.get_unix_time_from_system() - at
	if s < 120.0:
		return "just now"
	if s < 7200.0:
		return "%d min ago" % int(s / 60.0)
	if s < 172800.0:
		return "%d h ago" % int(s / 3600.0)
	return "%d days ago" % int(s / 86400.0)


func _panel(width: float) -> Array:
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(width, 0)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 20)
	panel.add_child(margin)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 10)
	margin.add_child(v)
	overlay.get_parent().add_child(panel)
	return [panel, v]


func _center(panel: Control) -> void:
	panel.position = ((overlay.size - panel.size) / 2.0).max(Vector2.ZERO)


func _show_menu(error: String) -> void:
	print("MFTR: start menu")
	if menu_panel != null:
		menu_panel.queue_free()
	var made := _panel(620)
	menu_panel = made[0]
	var v: VBoxContainer = made[1]
	var title := Label.new()
	title.text = "MFTR"
	title.add_theme_font_size_override("font_size", 32)
	v.add_child(title)

	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 8)
	menu_address = LineEdit.new()
	menu_address.placeholder_text = "host:port   (host:port#fingerprint pins the server's key)"
	menu_address.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	menu_address.text_submitted.connect(func(_t): _menu_join(menu_address.text))
	row.add_child(menu_address)
	var join := Button.new()
	join.text = "Join"
	join.custom_minimum_size = Vector2(90, 0)
	join.pressed.connect(func(): _menu_join(menu_address.text))
	row.add_child(join)
	v.add_child(row)

	var servers := _load_servers()
	menu_address.text = servers[0].get("address", "") if servers.size() > 0 else "127.0.0.1:7777"
	if servers.size() > 0:
		var head := Label.new()
		head.text = "Recent servers"
		head.add_theme_color_override("font_color", Color(0.7, 0.75, 0.8))
		v.add_child(head)
		for e in servers:
			var address: String = e.get("address", "")
			var line := HBoxContainer.new()
			line.add_theme_constant_override("separation", 6)
			var b := Button.new()
			b.text = "%s   ·   %s   ·   %s" % [address, e.get("game", "?"), _ago(float(e.get("at", 0.0)))]
			b.alignment = HORIZONTAL_ALIGNMENT_LEFT
			b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			b.tooltip_text = "Join %s" % address
			b.pressed.connect(func(): _menu_join(address))
			line.add_child(b)
			var x := Button.new()
			x.text = "✕"
			x.tooltip_text = "Forget this server"
			x.pressed.connect(func(): _forget_server(address))
			line.add_child(x)
			v.add_child(line)

	var opts := GridContainer.new()
	opts.columns = 2
	opts.add_theme_constant_override("h_separation", 12)
	var mode_label := Label.new()
	mode_label.text = "Join as"
	opts.add_child(mode_label)
	var mode := OptionButton.new()
	for m in JOIN_MODES:
		mode.add_item(m)
	mode.selected = clampi(_menu_mode, 0, JOIN_MODES.size() - 1)
	mode.item_selected.connect(func(i): _menu_mode = i)
	opts.add_child(mode)
	var champ_label := Label.new()
	champ_label.text = "Champion"
	opts.add_child(champ_label)
	var champ := OptionButton.new()
	champ.add_item("Server picks")
	var names: PackedStringArray = client.champion_names()
	for n in names:
		champ.add_item(n)
	champ.selected = 0
	for n in names.size():
		if names[n].to_lower() == _menu_champion.to_lower():
			champ.selected = n + 1
	champ.item_selected.connect(func(i): _menu_champion = "" if i == 0 else names[i - 1])
	champ.tooltip_text = "Duel and sandbox servers. ARAM deals random champions in champion select."
	opts.add_child(champ)
	v.add_child(opts)

	var hint := Label.new()
	hint.text = "ARAM servers deal random champions in champion select. Blind playtest: rounds under hidden network conditions, rated after each (send the results file to the developers)."
	hint.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	hint.add_theme_font_size_override("font_size", 13)
	hint.add_theme_color_override("font_color", Color(0.65, 0.68, 0.72))
	v.add_child(hint)
	if error != "":
		var err := Label.new()
		err.text = error
		err.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		err.add_theme_color_override("font_color", Color(1.0, 0.5, 0.4))
		v.add_child(err)
	var quit := Button.new()
	quit.text = "Quit"
	quit.pressed.connect(func(): get_tree().quit())
	v.add_child(quit)
	await get_tree().process_frame
	if menu_panel != null:
		_center(menu_panel)
		menu_address.grab_focus()
		if _menu_auto_join and servers.size() > 0:
			_menu_auto_join = false
			await get_tree().create_timer(1.0).timeout
			_menu_join(servers[0].get("address", ""))


func _menu_join(address: String) -> void:
	address = address.strip_edges()
	if address == "":
		return
	client.set_spectate(_menu_mode == 1)
	blind_enabled = _menu_mode == 2
	client.set_champion(_menu_champion)
	_save_servers(_load_servers(), true)
	menu_panel.queue_free()
	menu_panel = null
	_connect(address)


## While the server hasn't answered: where we're connecting, why it fails, and a way back.
func _update_connecting(phase: String) -> void:
	if phase != "connecting":
		if connecting_panel != null:
			connecting_panel.queue_free()
			connecting_panel = null
		return
	if connecting_panel == null:
		var made := _panel(520)
		connecting_panel = made[0]
		var v: VBoxContainer = made[1]
		connecting_label = Label.new()
		connecting_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		v.add_child(connecting_label)
		var cancel := Button.new()
		cancel.text = "Back to the server list"
		cancel.pressed.connect(_leave)
		v.add_child(cancel)
	var err: String = client.last_error()
	connecting_label.text = "Connecting to %s…%s" % [_server_address, ("\n\n" + err) if err != "" else ""]
	_center(connecting_panel)


func _toggle_pause_menu() -> void:
	if pause_panel != null:
		pause_panel.visible = not pause_panel.visible
		_center(pause_panel)
		return
	var made := _panel(320)
	pause_panel = made[0]
	var v: VBoxContainer = made[1]
	var title := Label.new()
	title.text = _server_address
	v.add_child(title)
	var resume := Button.new()
	resume.text = "Resume"
	resume.pressed.connect(func(): pause_panel.visible = false)
	v.add_child(resume)
	var leave := Button.new()
	leave.text = "Leave server"
	leave.pressed.connect(_leave)
	v.add_child(leave)
	var quit := Button.new()
	quit.text = "Quit"
	quit.pressed.connect(func(): get_tree().quit())
	v.add_child(quit)
	await get_tree().process_frame
	_center(pause_panel)


## Disconnect and start over in the menu (a fresh scene: nothing of the match is left).
func _leave() -> void:
	client.disconnect_from_server()
	_back_to_menu = true
	get_tree().reload_current_scene()


## The match ended and the server holds champion select again: drop what belonged to it.
func _reset_match_view() -> void:
	if own_body != null:
		own_body.queue_free()
		own_body = null
	own_champion = ""
	if shop_panel != null:
		shop_panel.visible = false
	match_banner = ""
	floaters.clear()
	notices.clear()
	_lobby_ready = false


## ---- Augment draft (ARAM: Mayhem, M3) -------------------------------------------------------------
## At its draft levels the champion is offered three augments of one tier: click one to keep it
## (predicted, like shopping), or reroll the offer once. Play goes on while the cards are up.

var draft_panel: PanelContainer
var draft_box: VBoxContainer
var _draft_shown := ""                  # the offer on screen (rebuilt when it changes)


func _tier_color(tier: String) -> Color:
	match tier:
		"Gold":
			return Color(1.0, 0.8, 0.3)
		"Prismatic":
			return Color(0.85, 0.55, 1.0)
	return Color(0.78, 0.82, 0.88)


func _update_draft(playing: bool) -> void:
	var offer: Array = own_status.get("offer", []) if playing else []
	if offer.is_empty():
		if draft_panel != null:
			draft_panel.queue_free()
			draft_panel = null
		_draft_shown = ""
		return
	var can_reroll: bool = own_status.get("can_reroll", false)
	var key := str(offer.map(func(c): return c.id)) + str(can_reroll)
	if draft_panel == null:
		var made := _panel(760)
		draft_panel = made[0]
		draft_box = made[1]
	if key != _draft_shown:
		_draft_shown = key
		for c in draft_box.get_children():
			c.queue_free()
		var title := Label.new()
		title.text = "Choose a %s augment" % offer[0].tier
		title.add_theme_font_size_override("font_size", 20)
		title.add_theme_color_override("font_color", _tier_color(offer[0].tier))
		draft_box.add_child(title)
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 12)
		draft_box.add_child(row)
		for i in offer.size():
			row.add_child(_augment_card(offer[i], i))
		var reroll := Button.new()
		reroll.text = "Reroll" if can_reroll else "Rerolled"
		reroll.disabled = not can_reroll
		reroll.focus_mode = Control.FOCUS_NONE
		reroll.pressed.connect(func(): client.reroll_augments())
		draft_box.add_child(reroll)
	# Low on the screen, above the ability bar, so the fight stays visible.
	draft_panel.position = Vector2((overlay.size.x - draft_panel.size.x) / 2.0, overlay.size.y - draft_panel.size.y - 150.0)


func _augment_card(a: Dictionary, choice: int) -> Button:
	var card := Button.new()
	card.custom_minimum_size = Vector2(230, 130)
	card.focus_mode = Control.FOCUS_NONE
	card.pressed.connect(func(): client.pick_augment(choice))
	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_FULL_RECT)
	margin.mouse_filter = Control.MOUSE_FILTER_IGNORE
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 10)
	card.add_child(margin)
	var v := VBoxContainer.new()
	v.mouse_filter = Control.MOUSE_FILTER_IGNORE
	margin.add_child(v)
	var name := Label.new()
	name.text = a.name
	name.add_theme_font_size_override("font_size", 18)
	name.add_theme_color_override("font_color", _tier_color(a.tier))
	name.mouse_filter = Control.MOUSE_FILTER_IGNORE
	v.add_child(name)
	var text := Label.new()
	text.text = a.text
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text.custom_minimum_size = Vector2(205, 0)
	text.mouse_filter = Control.MOUSE_FILTER_IGNORE
	v.add_child(text)
	return card
