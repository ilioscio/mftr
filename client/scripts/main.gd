extends Node3D
## M1 Duel Sandbox client. Builds the scene in code, forwards input to the Rust MatchClient,
## and draws what it reports. No gameplay decisions are made here (04 §1).
##
## Without a server address the client opens its start menu: pick a server (remembered ones are
## listed with their game type), how to join (play, spectate or the blind playtest) and a champion.
## User args (after `--`) skip the menu: a server address (`host:port#fingerprint` pins the
## server's key, otherwise it is trusted on first use), `--champion NAME`,
## `--spectate` to watch (N cycles champions), `--shot-lobby` for a champion-select capture,
## `--shot <file.png>` / `--shot-at <seconds>` / `--shot-shop` for scripted screenshots (`--zoom <factor>` brings the camera closer, `--look X,Y` aims it at a map point; `--shot-menu`
## captures the start menu, `--hover-slot N` shows an ability's tooltip, `--keep-points` leaves
## the starting points unspent, `--shot-charge` fights in mid, `--shot-numbers` shows sample
## damage numbers), and the blind playtest
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
const LEVEL_ACTIONS := ["level_q", "level_w", "level_e", "level_r"]

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
var world_env: Environment
var portraits                             # champion portraits from their models (portraits.gd)
var sun: DirectionalLight3D
var atmosphere                            # cloud shadows and motes (atmosphere.gd), per map
var _look_point := Vector3.ZERO           # where the camera looks, on the ground
var _graphics_applied := []
var _audio_applied := []
var _focused := true
var backdrop: CanvasLayer
var ability_tip
# The ability bar's boxes and level-up buttons this frame (overlay coordinates), for hover and
# clicks; the whole bar's panel swallows clicks so they don't move the champion.
var _bar_rect := Rect2()
var _ability_boxes: Array[Rect2] = []
var _level_buttons := {}                  # slot -> Rect2, while the slot can rank up
var _cd_total := [0.0, 0.0, 0.0, 0.0, 0.0, 0.0]  # each cooldown's length, for the sweep
var _ability_kinds := {}                  # slot -> icon kind
var _ability_kinds_at := 0
var _chips := {}                          # unit -> lagging health (the bars' damage chip)
var _hud_dt := 0.0
var _bold: FontVariation
const Hud := preload("res://scripts/hud.gd")
const AbilityTooltip := preload("res://scripts/ability_tooltip.gd")
const Windows := preload("res://scripts/windows.gd")
const GameCursor := preload("res://scripts/cursor.gd")
const AugmentIcons := preload("res://scripts/augment_icons.gd")
var _item_boxes: Array[Rect2] = []        # the HUD's inventory slots this frame
var _augment_rows := []                   # [Rect2, augment] above the HUD, for hover
var show_net_graph := false             # F1: the full network graph (the HUD shows fps and ping)
var proxies_enabled := true
var attack_move_armed := false
var own_status := {}
var floaters := []                      # damage numbers: { pos: Vector3, text, color, age }
var match_banner := ""                  # "VICTORY" / "DEFEAT" when a Base falls
var match_banner_age := 0.0
var notices := []                       # kill feed: { text, age }
# A3 (10 §6): champion models from content packs. Until champions ship their own, they all wear
# the shared template (tinted with their identity color). F3 toggles the placeholder shapes.
var champion_model: MftrModel = null     # the shared template
var champion_models := {}                # champion name -> its own MftrModel (or the template)
var minion_models := {}                  # A5: minion kind -> its MftrModel (or null)
# A5: minions that just died, playing their death where they fell: { body, t }.
var corpses := []
const MINION_LIFT := -0.45               # minion bodies sit at y 0.45 (the capsule's center)
const MINION_SLOTS := ["skin", "cloth", "metal", "emissive", "accent", "accent_glow"]
var use_models := true
var hovered_body: Node3D = null
const TURN_RATE := deg_to_rad(4500.0)    # 10 §3: a 180° turn in ~40 ms
const FLASH_TIME := 0.05                 # 10 §4.4: 2-3 frames of white on the target
const OUTLINE_WIDTH := 0.025
const MODEL_LIFT := -0.85                # bodies sit at y 0.85 (the placeholder's center)
# A4b: the VFX kit and each champion's effects by event (`<action>.<phase>`), from its pack's
# `<id>.vfx.ron`, falling back to the shared library's.
var vfx: Node3D
var _vfx_tables := {}                    # champion name -> { event: [effect] }
var _vfx_recent := {}                    # dedupe key -> time (a prediction and its confirmation)
const PARTICLE_KITS := ["flare", "burst", "ring", "dust"]
# A4c: the SFX player and each champion's sounds by event, from its pack's `<id>.sfx.ron`,
# falling back to the shared library's (as the VFX do).
var sfx: Node3D
var _sfx_tables := {}                    # champion name -> { event: [sound] }
# Player settings (Esc → Settings): camera, minimap, keybinds. The camera rig scrolls the view.
var settings = preload("res://scripts/settings.gd").new()
var cam = preload("res://scripts/camera_rig.gd").new()
var settings_panel: Control
var minimap: Control
var _was_dead := false


func _ready() -> void:
	# The UI theme goes into the engine's default theme: our panels live under CanvasLayers,
	# which a theme set on the window wouldn't reach.
	ThemeDB.get_default_theme().merge_with(preload("res://scripts/ui_theme.gd").build())
	_build_world()
	_build_backdrop()
	_load_champion_model()
	portraits = preload("res://scripts/portraits.gd").new()
	add_child(portraits)
	portraits.setup(func(c: String): return _model_for(c), champion_model, CHAMPION_COLORS)
	portraits.rendered.connect(_on_portrait)
	vfx = preload("res://scripts/vfx.gd").new()
	add_child(vfx)
	sfx = preload("res://scripts/sfx.gd").new()
	add_child(sfx)
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
		elif args[i] == "--dump-terrain" and i + 1 < args.size():
			_dump_terrain = args[i + 1]
			i += 1
		elif args[i] == "--zoom" and i + 1 < args.size():
			camera_zoom = maxf(0.05, float(args[i + 1]))
			i += 1
		elif args[i] == "--look" and i + 1 < args.size():
			var xy := args[i + 1].split(",")
			_shot_look = Vector2(float(xy[0]), float(xy[1]))
			i += 1
		elif args[i] == "--shot-shop":
			_shot_shop = true
		elif args[i] == "--dump-portraits" and i + 1 < args.size():
			_dump_portraits(args[i + 1])
			return
		elif args[i] == "--shop-pick" and i + 1 < args.size():
			_shop_pick = int(args[i + 1])
			i += 1
		elif args[i] == "--shot-settings" and i + 1 < args.size():
			_shot_settings = int(args[i + 1])
			i += 1
		elif args[i] == "--shot-tab":
			_shot_tab = true
		elif args[i] == "--tab-hover" and i + 1 < args.size():
			_shot_tab_hover = args[i + 1]
			i += 1
		elif args[i] == "--dump-augment-icons" and i + 1 < args.size():
			# Every glyph in every tier, one row per tier: a sheet for review.
			var glyphs: Array = AugmentIcons.GLYPHS.keys() + AugmentIcons.STATS
			var sheet := Image.create(glyphs.size() * 68, 3 * 68, false, Image.FORMAT_RGBA8)
			sheet.fill(Color(0.1, 0.11, 0.13))
			for row in 3:
				for col in glyphs.size():
					var img := AugmentIcons.icon({"glyph": glyphs[col], "tier": ["Silver", "Gold", "Prismatic"][row]}).get_image()
					img.decompress()
					img.clear_mipmaps()
					img.resize(64, 64, Image.INTERPOLATE_LANCZOS)
					img.convert(Image.FORMAT_RGBA8)
					sheet.blend_rect(img, Rect2i(0, 0, 64, 64), Vector2i(col * 68 + 2, row * 68 + 2))
			sheet.save_png(args[i + 1])
			print("MFTR: augment icons saved to ", args[i + 1])
			get_tree().quit()
			return
		elif args[i] == "--dump-cursors" and i + 1 < args.size():
			for state in ["default", "enemy", "ally", "attack"]:
				GameCursor._make(state)[0].save_png("%s/cursor_%s.png" % [args[i + 1], state])
			print("MFTR: cursors saved to ", args[i + 1])
			get_tree().quit()
			return
		elif args[i] == "--shot-numbers":
			_shot_numbers = true
		elif args[i] == "--shot-charge":
			_shot_charge = true
		elif args[i] == "--shot-recall":
			_shot_recall = true
		elif args[i] == "--shot-claim":
			_shot_claim = true
		elif args[i] == "--shot-goto" and i + 1 < args.size():
			var xy := args[i + 1].split(",")
			_shot_goto = Vector2(float(xy[0]), float(xy[1]))
			i += 1
		elif args[i] == "--shot-menu":
			_shot_menu = true
		elif args[i] == "--keep-points":
			_shot_keep_points = true
		elif args[i] == "--hover-slot" and i + 1 < args.size():
			_shot_hover = int(args[i + 1])
			i += 1
		elif args[i] == "--shot-lobby":
			_shot_lobby = true
		elif args[i] == "--shot-loading":
			_shot_loading = true
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


## The shared template from `art/` (A3), validated by `mftr-pack` on load (11 §4). Missing or
## refused: champions keep their placeholder shapes.
func _load_champion_model() -> void:
	var path := _art_path("library/biped/export/biped_library.glb")
	if FileAccess.file_exists(path):
		champion_model = MftrModel.load(path)
	if champion_model == null:
		use_models = false
		print("MFTR: no champion model (%s): placeholder shapes" % path)
	else:
		print("MFTR: packs from ", _art_root())


var _art_base := ""


## Where the packs are: `art/` next to the executable in a release (the app bundle's
## `Contents/Resources/art` on macOS; scripts/package-client.sh puts them there), else the repo's
## `art/` beside the project (editor and dev runs). They are plain files read by `mftr-pack`,
## never Godot resources (11 §4).
func _art_root() -> String:
	if _art_base == "":
		var exe := OS.get_executable_path().get_base_dir()
		_art_base = ProjectSettings.globalize_path("res://").path_join("../art").simplify_path()
		for dir in [exe.path_join("art"), exe.path_join("../Resources/art").simplify_path()]:
			if DirAccess.dir_exists_absolute(dir):
				_art_base = dir
				break
	return _art_base


func _art_path(rel: String) -> String:
	return _art_root().path_join(rel).simplify_path()


## A champion's own pack (`art/champions/<id>/export/<id>.glb`, A4) if it ships one and it
## validates, else the template. Loaded once per champion.
func _model_for(champion: String) -> MftrModel:
	if not champion_models.has(champion):
		var id := champion.to_lower()
		var path := _art_path("champions/%s/export/%s.glb" % [id, id])
		var own: MftrModel = MftrModel.load(path) if FileAccess.file_exists(path) else null
		champion_models[champion] = own if own != null else champion_model
	return champion_models[champion]


## A lane minion's pack (`art/minions/<kind>/export/<kind>.glb`, A5), loaded once per kind;
## null without one (the capsule stays).
func _minion_model(kind: String) -> MftrModel:
	if kind == "":
		return null
	if not minion_models.has(kind):
		var path := _art_path("minions/%s/export/%s.glb" % [kind, kind])
		minion_models[kind] = MftrModel.load(path) if FileAccess.file_exists(path) else null
	return minion_models[kind]


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
var _shot_loading := false                # `--shot-loading`: ready up, capture the loading screen
var _menu_auto_join := false             # `--menu-join`: join the first remembered server from the menu
var camera_zoom := 1.0                    # `--zoom <factor>`: closer camera for reviewing models
var _shot_look = null                     # `--look X,Y`: scripted captures look at this map point
var _shot_hover := -1                     # `--hover-slot N`: show that ability's tooltip
var _shot_settings := -1                  # `--shot-settings N`: open Settings on tab N
var _shot_tab_hover := ""                 # `--tab-hover augment|item`: show one's tooltip in it
var _shot_tab := false                    # `--shot-tab`: hold the match breakdown open
var _shot_numbers := false                # `--shot-numbers`: sample damage numbers, for review
var _shot_charge := false                 # `--shot-charge`: attack-move to mid, camera locked on us
var _shot_recall := false                 # `--shot-recall`: walk out, then recall 3 s before the shot
var _shot_claim := false                 # `--shot-claim`: take Claim in F at the start
var _shot_goto = null                     # `--shot-goto X,Y`: walk there (game units) from 2 s on
var _shot_menu := false                   # `--shot-menu`: capture the start menu
var _shot_keep_points := false            # `--keep-points`: don't spend the starting points


func _update_shot(delta: float) -> void:
	if _shot_path != "" and _shot_menu:
		_shot_timer += delta
		if _shot_timer > 1.2:
			get_viewport().get_texture().get_image().save_png(_shot_path)
			print("MFTR: saved screenshot to ", _shot_path)
			get_tree().quit()
		return
	if _shot_path != "" and _shot_loading:
		_shot_timer += delta
		if client.phase() == "lobby" and _shot_timer > 2.0 and not _lobby_ready:
			client.lobby_ready(true)
			_lobby_ready = true
		if loading_panel != null and _loading_age > 1.2:
			get_viewport().get_texture().get_image().save_png(_shot_path)
			print("MFTR: saved screenshot to ", _shot_path)
			get_tree().quit()
		return
	if _shot_path != "" and _shot_lobby and client.phase() == "lobby":
		_shot_timer += delta
		if _shot_timer > 0.6 and _shot_timer - delta <= 0.6:
			client.lobby_reroll()
		elif _shot_timer > 3.0:
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
		for slot in 0 if _shot_keep_points else 3:
			client.level_up(slot)  # ranked modes start with points to spend
		if _shot_claim:
			client.choose_spell(1)
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
	elif _shot_goto != null and _shot_moved and _shot_timer > 2.0 and fmod(_shot_timer, 2.0) < delta:
		client.move_to(_shot_goto)
	elif _shot_recall and _shot_moved and _shot_timer > _shot_at - 3.0 and _shot_timer - delta <= _shot_at - 3.0:
		client.recall()
	elif _shot_charge and _shot_moved and _shot_timer > 2.0 and _shot_timer <= _shot_at:
		# Into the fight: keep attack-moving to mid, casting at whatever's ahead.
		settings.camera_locked = true
		if fmod(_shot_timer, 1.0) < delta:
			client.attack_move(own + inward * 9000.0)  # into the enemy's side, where the fight is
			var aim := own + inward * 600.0
			client.cast(int(_shot_timer) % 3, aim)
	elif _shot_numbers and _shot_moved and _shot_timer <= _shot_at and fmod(_shot_timer, 0.18) < delta and own_body != null:
		var samples := [["physical", 64.0, false, false], ["magic", 212.0, false, false], ["true", 40.0, false, false], ["physical", 118.0, false, true], ["magic", 90.0, true, false]]
		var x: Array = samples[int(_shot_timer / 0.18) % samples.size()]
		var color: Color = {"physical": Color(1.0, 0.62, 0.24), "magic": Color(0.45, 0.68, 1.0), "true": Color(1, 1, 1)}[x[0]]
		var text := "%d" % roundi(x[1])
		if x[2]:
			color = Color(0.42, 0.95, 0.5)
			text = "+" + text
		elif x[3]:
			color = Color(1.0, 0.3, 0.28)
			text = "-" + text
		floaters.append({ "pos": own_body.position + Vector3(2.0, 0, 0), "text": text, "color": color, "age": 0.0, "size": clampf(19.0 + x[1] / 28.0, 19.0, 34.0), "drift": randf_range(-0.6, 0.6), "life": 1.0 })
	elif _shot_settings >= 0 and _shot_moved and settings_panel == null:
		_open_settings()
		settings_panel.tabs.current_tab = _shot_settings
	elif _shot_moved and _shot_timer > _shot_at:
		get_viewport().get_texture().get_image().save_png(_shot_path)
		print("MFTR: saved screenshot to ", _shot_path)
		get_tree().quit()


func _build_world() -> void:
	# Light (05 §5): a warm late-afternoon sun with soft shadows, a cool sky fill so shade reads
	# blue rather than grey, a filmic tonemap, bloom on what glows (crystals, braziers,
	# missiles), a light haze with depth, and a gentle grade.
	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_COLOR
	e.background_color = Color(0.08, 0.09, 0.1)
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color(0.56, 0.66, 0.82)
	e.ambient_light_energy = 0.62
	e.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	e.tonemap_exposure = 1.25
	e.tonemap_white = 4.0
	e.glow_enabled = true
	e.glow_intensity = 0.55
	e.glow_strength = 1.0
	e.glow_bloom = 0.0
	e.glow_hdr_threshold = 0.9
	e.glow_hdr_scale = 2.0
	e.glow_blend_mode = Environment.GLOW_BLEND_MODE_SOFTLIGHT
	for i in 7:
		e.set_glow_level(i, 1.0 if i in [1, 2, 3] else 0.0)
	# Haze only toward the top of the screen (farther from the camera), none near the action.
	e.fog_enabled = true
	e.fog_mode = Environment.FOG_MODE_DEPTH
	e.fog_light_color = Color(0.6, 0.68, 0.78)
	e.fog_light_energy = 1.0
	e.fog_density = 0.35
	e.fog_depth_begin = 24.0
	e.fog_depth_end = 60.0
	e.fog_depth_curve = 1.6
	e.fog_sky_affect = 0.0
	e.adjustment_enabled = true
	e.adjustment_contrast = 1.08
	e.adjustment_saturation = 1.18
	env.environment = e
	world_env = e
	add_child(env)

	sun = DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-52, -38, 0)
	sun.light_color = Color(1.0, 0.94, 0.84)
	sun.light_energy = 1.25
	sun.shadow_enabled = true
	sun.shadow_opacity = 0.72
	sun.shadow_blur = 1.6
	sun.shadow_bias = 0.04
	sun.shadow_normal_bias = 1.2
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS
	sun.directional_shadow_max_distance = 45.0
	sun.directional_shadow_split_1 = 0.4
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
	overlay.texture_filter = CanvasItem.TEXTURE_FILTER_LINEAR_WITH_MIPMAPS
	overlay.draw.connect(_draw_overlay)
	hud.add_child(overlay)
	ability_tip = preload("res://scripts/ability_tooltip.gd").new()
	# Tooltips over everything, the windows included.
	var tips := CanvasLayer.new()
	tips.layer = 10
	add_child(tips)
	tips.add_child(ability_tip)
	net_label = Label.new()
	net_label.position = Vector2(16, 16)
	net_label.add_theme_font_size_override("font_size", 16)
	net_label.add_theme_color_override("font_shadow_color", Color.BLACK)
	hud.add_child(net_label)


## `--dump-portraits DIR`: render every champion's portraits to DIR as PNGs, then quit (review).
func _dump_portraits(dir: String) -> void:
	var names: PackedStringArray = client.champion_names()
	portraits.prepare(Array(names))
	for n in names:
		while not portraits.is_done(n):
			portraits.portrait(n)
			await portraits.rendered
		for kind in ["bust", "round", "small", "full"]:
			var t: Texture2D = portraits.portrait(n, kind)
			if t != null:
				t.get_image().save_png("%s/%s_%s.png" % [dir, n.to_lower(), kind])
	print("MFTR: portraits saved to ", dir)
	get_tree().quit()


## The Audio settings, applied when they change (or the window gains or loses focus).
func _apply_audio() -> void:
	var muted: bool = settings.mute_in_background and not _focused
	var want := [settings.master_volume, settings.effects_volume, settings.interface_volume, muted]
	if want == _audio_applied:
		return
	_audio_applied = want
	sfx.apply_volumes(settings.master_volume, settings.effects_volume, settings.interface_volume, muted)


func _notification(what: int) -> void:
	if what == NOTIFICATION_APPLICATION_FOCUS_OUT:
		_focused = false
	elif what == NOTIFICATION_APPLICATION_FOCUS_IN:
		_focused = true


## The Graphics settings, applied when they change.
func _apply_graphics() -> void:
	var want := [settings.shadows, settings.bloom, settings.atmosphere, atmosphere != null]
	if want == _graphics_applied:
		return
	_graphics_applied = want
	sun.shadow_enabled = settings.shadows
	world_env.glow_enabled = settings.bloom
	if atmosphere != null:
		atmosphere.set_enabled(settings.atmosphere, settings.atmosphere)


## The menus' backdrop and logo, behind every screen that isn't the match itself.
func _build_backdrop() -> void:
	backdrop = CanvasLayer.new()
	backdrop.layer = -1
	add_child(backdrop)
	var bg := ColorRect.new()
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var m := ShaderMaterial.new()
	m.shader = load("res://shaders/backdrop.gdshader")
	bg.material = m
	backdrop.add_child(bg)
	var logo := VBoxContainer.new()
	logo.name = "Logo"
	logo.set_anchors_and_offsets_preset(Control.PRESET_CENTER_TOP)
	logo.mouse_filter = Control.MOUSE_FILTER_IGNORE
	logo.alignment = BoxContainer.ALIGNMENT_CENTER
	var word := Label.new()
	word.text = "MFTR"
	var f := FontVariation.new()
	f.base_font = ThemeDB.fallback_font
	f.variation_embolden = 1.1
	f.spacing_glyph = 18
	word.add_theme_font_override("font", f)
	word.add_theme_font_size_override("font_size", 92)
	word.add_theme_color_override("font_color", Color(0.97, 0.9, 0.72))
	word.add_theme_color_override("font_outline_color", Color(0.3, 0.2, 0.06))
	word.add_theme_constant_override("outline_size", 6)
	word.add_theme_color_override("font_shadow_color", Color(0, 0, 0, 0.6))
	word.add_theme_constant_override("shadow_offset_y", 5)
	word.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	logo.add_child(word)
	var tag := Label.new()
	tag.text = "M O B A   F O R   T H E   R E S T   O F   U S"
	tag.add_theme_font_size_override("font_size", 14)
	tag.add_theme_color_override("font_color", Color(0.78, 0.65, 0.38))
	tag.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	logo.add_child(tag)
	backdrop.add_child(logo)


func _update_backdrop(phase: String) -> void:
	backdrop.visible = phase != "playing" or loading_panel != null
	var logo: Control = backdrop.get_node("Logo")
	logo.visible = menu_panel != null or phase in ["", "connecting"]
	logo.position = Vector2((overlay.size.x - logo.size.x) / 2.0, overlay.size.y * 0.06)
	var m: ShaderMaterial = (backdrop.get_child(0) as ColorRect).material
	m.set_shader_parameter("aspect", overlay.size.x / maxf(overlay.size.y, 1.0))


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
	"Quill": Color(0.2, 0.5, 0.55),
	"Cairn": Color(0.45, 0.5, 0.3),
	"Marrow": Color(0.62, 0.58, 0.5),
	"Wren": Color(0.6, 0.3, 0.5),
}


func _part(parent: Node3D, mesh: Mesh, pos: Vector3, mat: Material, rot := Vector3.ZERO) -> MeshInstance3D:
	var n := MeshInstance3D.new()
	n.set_meta("placeholder", true)
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
		"Quill":
			# Tall and thin, with a long glowing quill raised over the shoulder: artillery.
			var cyl := CylinderMesh.new()
			cyl.top_radius = 0.14
			cyl.bottom_radius = 0.3
			cyl.height = 1.6
			body.mesh = cyl
			var spine := CylinderMesh.new()
			spine.top_radius = 0.0
			spine.bottom_radius = 0.06
			spine.height = 1.2
			_part(body, spine, Vector3(0.3, 0.9, 0), _unshaded(Color(0.6, 0.95, 1.0)), Vector3(0, 0, -30))
		"Cairn":
			# Stacked stones, widest at the base: the warden holds ground.
			var base := BoxMesh.new()
			base.size = Vector3(1.0, 0.7, 0.8)
			body.mesh = base
			var mid := BoxMesh.new()
			mid.size = Vector3(0.75, 0.5, 0.6)
			_part(body, mid, Vector3(0, 0.6, 0), m, Vector3(0, 20, 0))
			var top := BoxMesh.new()
			top.size = Vector3(0.45, 0.35, 0.4)
			_part(body, top, Vector3(0, 1.02, 0), m, Vector3(0, -15, 0))
		"Marrow":
			# Hunched capsule with two dark horns.
			var cap := CapsuleMesh.new()
			cap.radius = 0.38
			cap.height = 1.45
			body.mesh = cap
			var horn := CylinderMesh.new()
			horn.top_radius = 0.0
			horn.bottom_radius = 0.07
			horn.height = 0.45
			var dark := _unshaded(Color(0.2, 0.16, 0.2))
			_part(body, horn, Vector3(0.2, 0.8, 0), dark, Vector3(0, 0, -25))
			_part(body, horn, Vector3(-0.2, 0.8, 0), dark, Vector3(0, 0, 25))
		"Wren":
			# Small and light, with swept-back wings.
			var cone := CylinderMesh.new()
			cone.top_radius = 0.1
			cone.bottom_radius = 0.3
			cone.height = 1.35
			body.mesh = cone
			var wing := BoxMesh.new()
			wing.size = Vector3(0.55, 0.05, 0.28)
			_part(body, wing, Vector3(0.36, 0.35, -0.1), m, Vector3(0, 25, 20))
			_part(body, wing, Vector3(-0.36, 0.35, -0.1), m, Vector3(0, -25, -20))
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
	var outline := _outline_material(color)
	m.next_pass = outline
	body.set_meta("outline", outline)
	body.set_meta("flash_mats", [m])
	body.set_meta("placeholder_mesh", body.mesh)
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
	if champion_model != null:
		_attach_model(body, color, champion)
	_apply_model_mode(body)
	return body


func _outline_material(team: Color) -> ShaderMaterial:
	var o := ShaderMaterial.new()
	o.shader = load("res://shaders/outline.gdshader")
	o.set_shader_parameter("color", ENEMY_COLOR if team == ENEMY_COLOR else team.lerp(Color.WHITE, 0.3))
	return o


## The template model under `body`: one material per surface by slot, the team accent, the
## champion's identity color on cloth, the outline as a second pass, and its animator.
func _attach_model(body: MeshInstance3D, team: Color, champion: String, pack: MftrModel = null, lift := MODEL_LIFT) -> void:
	var source := pack if pack != null else _model_for(champion)
	var template := source == champion_model
	var model: Node3D = source.instantiate()
	model.position = Vector3(0, lift, 0)
	body.add_child(model)
	var mesh: MeshInstance3D = model.get_node("Skeleton/Mesh")
	var outline: ShaderMaterial = body.get_meta("outline")
	var mats := []
	var slots := source.surface_slots()
	for i in slots.size():
		var m := ShaderMaterial.new()
		m.shader = load("res://shaders/champion_model.gdshader")
		m.set_shader_parameter("slot", MINION_SLOTS.find(slots[i]))
		m.set_shader_parameter("team_accent", team)
		m.set_shader_parameter("identity", CHAMPION_COLORS.get(champion, Color(0.5, 0.5, 0.5)))
		m.set_shader_parameter("identity_mix", 0.6 if template else 0.0)
		m.next_pass = outline
		mesh.set_surface_override_material(i, m)
		mats.append(m)
	body.set_meta("flash_mats", body.get_meta("flash_mats", []) + mats)
	body.set_meta("rig", {
		"model": model,
		"skeleton": model.get_node("Skeleton"),
		"animator": source.new_animator(),
		"yaw": NAN,
		"last": Vector3.INF,
		"speed": 0.0,
	})


## Model or placeholder shapes (F3).
func _apply_model_mode(body: MeshInstance3D) -> void:
	var on := use_models and body.has_meta("rig")
	body.mesh = null if on else body.get_meta("placeholder_mesh", body.mesh)
	for c in body.get_children():
		if c.has_meta("placeholder"):
			c.visible = not on
	if body.has_meta("rig"):
		body.get_meta("rig").model.visible = on


func _models_shown(body: Node3D) -> bool:
	return use_models and body != null and body.has_meta("rig")


## Facing, locomotion speed, the animator and the impact flash, every frame (10 §3, §6).
## `info` is the unit's `own_status()` or `remote_units()` entry.
func _animate(body: Node3D, info: Dictionary, delta: float) -> void:
	if body == null:
		return
	var flash := maxf(0.0, float(body.get_meta("flash", 0.0)) - delta / FLASH_TIME)
	body.set_meta("flash", flash)
	for m in body.get_meta("flash_mats", []):
		m.set_shader_parameter("flash", flash)
	var dead: bool = info.get("dead", false)
	if dead != bool(body.get_meta("was_dead", false)):
		body.set_meta("was_dead", dead)
		_sfx_play(info.get("champion", ""), ["unit.death" if dead else "unit.respawn"], body.global_position)
	var dashing: bool = info.get("dashing", false)
	if dashing != bool(body.get_meta("was_dashing", false)):
		body.set_meta("was_dashing", dashing)
		var champ: String = info.get("champion", "")
		var slot := int(body.get_meta("dash_slot", -1))
		var action: String = ["q", "w", "e", "r", "d", "f"][slot] if slot >= 0 else client.dash_action(champ)
		_vfx_play(champ, action, "start" if dashing else "land", body.global_position)
	if not _models_shown(body) or delta <= 0.0:
		return
	var rig: Dictionary = body.get_meta("rig")
	# Displayed ground speed in u/s, lightly smoothed (positions arrive interpolated).
	var p := body.global_position
	if rig.last != Vector3.INF:
		var flat := Vector2(p.x - rig.last.x, p.z - rig.last.z)
		var v := flat.length() / delta / UNITS_TO_METERS
		rig.speed = lerpf(rig.speed, v, clampf(delta / 0.05, 0.0, 1.0))
	rig.last = p
	# Facing: the model's front (+Z) turns toward the sim's facing at TURN_RATE.
	if info.has("facing"):
		var target: float = PI / 2.0 - float(info.facing)
		if is_nan(rig.yaw):
			rig.yaw = target
		var diff := wrapf(target - rig.yaw, -PI, PI)
		rig.yaw += clampf(diff, -TURN_RATE * delta, TURN_RATE * delta)
		rig.model.rotation.y = rig.yaw
	info["hit"] = body.get_meta("hit", false)
	info["dash_slot"] = int(body.get_meta("dash_slot", -1))
	body.set_meta("hit", false)
	for e in rig.animator.drive(rig.skeleton, info, rig.speed, delta):
		var champ: String = info.get("champion", "")
		if e == "foot":
			_sfx_play(champ, ["unit.foot"], p)
		elif e.begins_with("fire:"):
			_fire_effects(champ, e.substr(5), p, rig.yaw)
		else:
			_sfx_play(champ, [e + ".cast", "*.cast"], p + Vector3(0, 1.2, 0))


func _flash(id: int) -> void:
	var body = own_body if id == client.own_unit_id() else remote_bodies.get(id)
	if body != null:
		body.set_meta("flash", 1.0)
		body.set_meta("hit", true)   # minions flinch (10 §5.4)


## Which of a kit's dashes a body is doing (A10): the animator picks that slot's clips, and the
## dash effects that slot's. True if `slot` is a dash or lunge.
func _note_dash(body: Node3D, champion: String, slot: int) -> bool:
	if body == null or champion == "" or slot >= SLOT_ACTIONS.size():
		return false
	if client.action_info(champion, ["q", "w", "e", "r", "d", "f"][slot]).get("shape", "") != "dash":
		return false
	body.set_meta("dash_slot", slot)
	return true


## `<action>.fire` (A6): an animation passed its `fire` marker, the moment a melee blow lands, a
## slam hits, a nova sweeps or a heal pulses. Where it plays comes from the kit: centered and
## sized for novas, at reach in front for melee blows and slams, at the chest for the rest.
var _action_info := {}


func _fire_effects(champion: String, action: String, at: Vector3, yaw: float) -> void:
	if champion == "":
		return
	var key := champion + "/" + action
	if not _action_info.has(key):
		_action_info[key] = client.action_info(champion, action)
	var info: Dictionary = _action_info[key]
	var ground := Vector3(at.x, 0.0, at.z)
	var fwd := Vector3(sin(yaw), 0.0, cos(yaw)) if not is_nan(yaw) else Vector3.ZERO
	match info.get("shape", ""):
		"nova":
			_vfx_play(champion, action, "fire", ground, Vector3.ZERO, float(info.radius))
		"melee", "line", "area":
			_vfx_play(champion, action, "fire", ground + fwd * minf(float(info.reach), 1.6) * 0.8 + Vector3(0, 0.1, 0), fwd)
		_:
			_vfx_play(champion, action, "fire", ground + Vector3(0, 1.3, 0), Vector3.UP)


## The effects for a champion's `<action>.<phase>`: its own, its own `*.<phase>`, then the
## shared library's (A4b, 11 §3).
func _effects(champion: String, action: String, phase: String) -> Array:
	var tables := [_vfx_table(champion)]
	if champion_model != null:
		tables.append(_vfx_table(""))
	for t in tables:
		for key in [action + "." + phase, "*." + phase]:
			if t.has(key):
				return t[key]
	return []


func _vfx_table(champion: String) -> Dictionary:
	if not _vfx_tables.has(champion):
		var model: MftrModel = champion_model if champion == "" else _model_for(champion)
		var table := {}
		if model != null and (champion == "" or model != champion_model):
			for e in model.vfx():
				if not table.has(e.event):
					table[e.event] = []
				table[e.event].append(e)
		_vfx_tables[champion] = table
	return _vfx_tables[champion]


## Plays a champion's particle effects for `<action>.<phase>` at `pos`, once per `dedupe` key
## within a short window (an own prediction and its confirmation are two different keys).
func _vfx_play(champion: String, action: String, phase: String, pos: Vector3, dir := Vector3.ZERO, radius := 0.0, dedupe := "") -> void:
	if champion == "" or action == "":
		return
	var now := Time.get_ticks_msec() / 1000.0
	if dedupe != "":
		if now - float(_vfx_recent.get(dedupe, -10.0)) < 0.4:
			return
		_vfx_recent[dedupe] = now
	for e in _effects(champion, action, phase):
		if e.kit in PARTICLE_KITS:
			vfx.play(e, pos, dir, radius)
	_sfx_play(champion, [action + "." + phase, "*." + phase], pos)


## Plays a champion's sound for the first of `keys` it (or else the shared library) binds, at
## `pos` (A4c, 11 §3.2).
func _sfx_play(champion: String, keys: Array, pos: Vector3) -> void:
	if champion == "":
		return
	var tables := [_sfx_table(champion)]
	if champion_model != null:
		tables.append(_sfx_table(""))
	for t in tables:
		for key in keys:
			if t.has(key):
				sfx.play(champion + "/" + key, t[key], pos)
				return


func _sfx_table(champion: String) -> Dictionary:
	if not _sfx_tables.has(champion):
		var model: MftrModel = champion_model if champion == "" else _model_for(champion)
		var table := {}
		if model != null and (champion == "" or model != champion_model):
			for s in model.sounds():
				for event in s.events:
					if not table.has(event):
						table[event] = []
					table[event].append(s)
		_sfx_tables[champion] = table
	return _sfx_tables[champion]


func _body_of(id: int):
	if id == client.own_unit_id():
		return own_body
	return remote_bodies.get(id)


## A bone's world position (sockets: where projectiles leave from), or the body's.
func _socket_world(body: Node3D, bone: String) -> Vector3:
	if _models_shown(body):
		var skel: Skeleton3D = body.get_meta("rig").skeleton
		var i := skel.find_bone(bone)
		if i >= 0:
			return skel.global_transform * skel.get_bone_global_pose(i).origin
	return body.position


## The attackable target under the cursor gets an outline (red for enemies, R04 §2).
func _update_hover() -> void:
	var target: Node3D = null
	var p = _cursor_ground() if client.phase() == "playing" else null
	if p != null:
		var id: int = client.pick_enemy(p, 30.0)
		if id >= 0 and remote_bodies.has(id):
			target = remote_bodies[id]
	if target == hovered_body:
		return
	for b in [hovered_body, target]:
		if b != null and is_instance_valid(b) and b.has_meta("outline"):
			b.get_meta("outline").set_shader_parameter("width", OUTLINE_WIDTH if b == target else 0.0)
	hovered_body = target


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
		# Esc closes what's open, innermost first: settings, the shop, then the pause menu.
		if settings_panel != null and settings_panel.visible:
			settings_panel.close()
		elif shop_panel != null and shop_panel.visible:
			_toggle_shop()
		elif menu_panel == null:
			_toggle_pause_menu()
		return
	if menu_panel != null or (pause_panel != null and pause_panel.visible):
		return
	if event is InputEventMouseButton and (event as InputEventMouseButton).button_index == MOUSE_BUTTON_MIDDLE:
		cam.drag(event.is_pressed(), (event as InputEventMouseButton).position)
		return
	if blind_panel != null and blind_panel.visible:
		return  # rating between rounds: the game ignores input
	if event is InputEventMouseButton and event.is_pressed() and _bar_rect.has_point(overlay.get_local_mouse_position()):
		# Clicks on the ability bar are the bar's: a "+" levels its ability, the rest do nothing
		# (they mustn't walk the champion under the HUD).
		if (event as InputEventMouseButton).button_index == MOUSE_BUTTON_LEFT:
			for slot in _level_buttons:
				if _level_buttons[slot].has_point(overlay.get_local_mouse_position()):
					client.level_up(slot)
		return
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
	# Items 1–6: their actives (a potion).
	for slot in 6:
		if event.is_action_pressed("item_%d" % (slot + 1), false, true):
			client.use_item(slot)
			return
	# Level-ups (default Alt + Q/W/E/R, 01 §13) before casts: both match exactly, modifiers
	# included, so Alt+Q never also casts Q.
	for slot in 4:
		if event.is_action_pressed(LEVEL_ACTIONS[slot], false, true):
			client.level_up(slot)
			return
	for slot in SLOT_ACTIONS.size():
		if event.is_action_pressed(SLOT_ACTIONS[slot], false, true):
			var aim = _cursor_ground()
			if aim != null:
				client.cast(slot, aim)
				# Our own dash is predicted before its event arrives: note its slot now.
				_note_dash(own_body, own_status.get("champion", ""), slot)
			return
	if event.is_action_pressed("camera_lock"):
		settings.camera_locked = not settings.camera_locked
		settings.save()
	elif event.is_action_pressed("toggle_minimap"):
		settings.minimap_shown = not settings.minimap_shown
		settings.save()
	elif event.is_action_pressed("attack_move"):
		attack_move_armed = true
	elif event.is_action_pressed("stop"):
		attack_move_armed = false
		client.stop()
	elif event.is_action_pressed("recall") and not client.is_spectator():
		client.recall()
	elif event.is_action_pressed("toggle_proxies"):
		proxies_enabled = not proxies_enabled
		client.set_collision_proxies(proxies_enabled)
	elif event.is_action_pressed("toggle_shop") and not client.is_spectator():
		_toggle_shop()
	elif event.is_action_pressed("spectate_next") and client.is_spectator():
		_spectate_next()
	elif event.is_action_pressed("toggle_net_graph") and not blind_enabled:
		show_net_graph = not show_net_graph
	elif event is InputEventKey and event.is_pressed() and not event.is_echo() and (event as InputEventKey).keycode == KEY_F3 and champion_model != null:
		use_models = not use_models
		for b in [own_body] + remote_bodies.values():
			if b != null and b.has_meta("placeholder_mesh"):
				_apply_model_mode(b)


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
	_update_backdrop(phase)
	if playing and not _map_built:
		_build_map()
	if playing:
		_update_blind()
	_update_draft(playing)
	own_status = client.own_status() if playing else {}
	_update_shop(delta)
	_update_lobby(delta, phase)
	_update_loading(delta)
	_update_anvil(playing)
	_update_scoreboard(delta, playing)
	_update_recap(playing)
	_update_cursor(playing)
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
		own_body.visible = playing and (not dead or _models_shown(own_body))
		var own := _to_world(client.own_position())
		own_body.position = own
		# Titan and Pebble: the model grows and shrinks with the hitbox (honest hitboxes).
		own_body.scale = Vector3.ONE * (float(own_status.get("hitbox", CHAMPION_RADIUS_U)) / CHAMPION_RADIUS_U)
		_update_camera(delta, dead)
		_show_statuses(own_body, own_status.get("stunned", false), own_status.get("rooted", false), own_status.get("shield", 0.0), own_status.get("slowed", false))
		_show_recall(own_body, own_status.get("recall", 0.0) > 0.0, ALLY_COLOR, own_status.get("champion", ""))
		_animate(own_body, own_status, delta)
	# Casts without a windup (A6): the drive never shows them, so play them on the event.
	for c in client.take_instant_casts():
		var caster = _body_of(int(c.unit))
		if caster == null or not _models_shown(caster):
			continue
		var who: String = own_status.get("champion", "") if caster == own_body else remote_info.get(int(c.unit), {}).get("champion", "")
		# Dashes and lunges have their own start / travel / land clips: note which (A10).
		if not _note_dash(caster, who, int(c.slot)):
			caster.get_meta("rig").animator.pulse(int(c.slot))
	_update_remotes(delta)
	_update_hover()
	_update_missiles()
	_update_areas()
	_update_bolts()
	_update_combat_text(delta)
	_update_fx(delta)
	_update_click_marker(delta)
	_update_net_graph()
	_update_minimap(delta, playing)
	_update_shot(delta)
	if atmosphere != null:
		atmosphere.update(delta, _look_point)
	_apply_graphics()
	_apply_audio()
	_update_ability_tip()
	_hud_dt = delta
	overlay.queue_redraw()


## The rig moves the look point (edge, keys, drag, lock, Space); the camera frames it (D13).
func _update_camera(delta: float, dead: bool) -> void:
	if _was_dead and not dead:
		cam.center(client.own_position())  # back from the dead: look at the champion
	_was_dead = dead
	var free := menu_panel == null and (pause_panel == null or not pause_panel.visible) and (settings_panel == null or not settings_panel.visible)
	if _shot_path != "":
		# Scripted captures follow the champion, or look where `--look` says.
		cam.center(client.own_position() if _shot_look == null else _shot_look)
	var units_per_px := CAMERA_DISTANCE_U / camera_zoom * 2.0 * tan(deg_to_rad(CAMERA_VFOV_DEG) / 2.0) / maxf(1.0, get_viewport().get_visible_rect().size.y)
	cam.update(delta, settings, client.own_position(), client.map_geometry().size, get_viewport(), units_per_px, free)
	_place_camera(_to_world(cam.focus))
	var confine: bool = settings.confine_cursor and free and _shot_path == ""
	Input.mouse_mode = Input.MOUSE_MODE_CONFINED if confine else Input.MOUSE_MODE_VISIBLE


func _place_camera(target: Vector3) -> void:
	var pitch := deg_to_rad(CAMERA_PITCH_DEG)
	var dist := CAMERA_DISTANCE_U * UNITS_TO_METERS / camera_zoom
	var look := Vector3(target.x, 0.0, target.z)
	_look_point = look
	camera.position = look + Vector3(0, sin(pitch) * dist, cos(pitch) * dist)
	camera.look_at(look, Vector3.UP)
	if sfx != null:
		sfx.listen(look, camera.basis)


func _show_statuses(body: Node3D, stunned: bool, rooted: bool, shield: float, slowed := false) -> void:
	body.get_node("Stun").visible = stunned
	body.get_node("Root").visible = rooted
	body.get_node("Shield").visible = shield > 0.0
	body.get_node("Slow").visible = slowed


## A recalling champion stands in a column of light in its team's color, a ring at its feet;
## its `unit.recall` sound plays as the channel starts.
func _show_recall(body: Node3D, on: bool, color: Color, champion: String) -> void:
	var beam: Node3D = body.get_node_or_null("Recall")
	if on and (beam == null or not beam.visible):
		_sfx_play(champion, ["unit.recall"], body.global_position)
	if beam == null:
		if not on:
			return
		beam = Node3D.new()
		beam.name = "Recall"
		var column := MeshInstance3D.new()
		var cyl := CylinderMesh.new()
		cyl.top_radius = 0.42
		cyl.bottom_radius = 0.55
		cyl.height = 3.2
		cyl.radial_segments = 8
		cyl.cap_top = false
		cyl.cap_bottom = false
		column.mesh = cyl
		column.position = Vector3(0, 0.8, 0)
		var mat := _unshaded(color.lerp(Color.WHITE, 0.35), 0.22)
		mat.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
		mat.cull_mode = BaseMaterial3D.CULL_DISABLED
		column.material_override = mat
		beam.add_child(column)
		var ring := MeshInstance3D.new()
		var torus := TorusMesh.new()
		torus.inner_radius = 0.6
		torus.outer_radius = 0.7
		torus.rings = 8
		torus.ring_segments = 4
		ring.mesh = torus
		ring.position = Vector3(0, -0.75, 0)
		ring.material_override = _unshaded(color.lerp(Color.WHITE, 0.5), 0.85)
		beam.add_child(ring)
		body.add_child(beam)
	beam.visible = on
	if on:
		beam.rotation.y = Time.get_ticks_msec() / 600.0


var remote_info := {}                   # unit_id -> latest dictionary (for bars and numbers)


func _update_remotes(delta: float) -> void:
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
				b = _make_minion(MINION_BLUE if u.ally else MINION_RED, u.radius, u.get("minion_kind", ""))
			elif u.kind == "monster":
				b = _make_monster(u.get("monster", ""), u.radius)
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
		if u.kind == "relic":
			_relic_pad(u.pos)
		if body.has_meta("prop"):
			p.y = 0.0
			if not body.has_meta("faced"):
				# Face down the lane, toward the enemy's side of the map.
				var dir := Vector2(1, 0)
				if _road_segments.size() > 1:
					# Several lanes: along the nearest road, away from its own fountain.
					dir = _road_direction(u.pos)
					for f in client.map_geometry().fountains:
						if f.ally == u.ally and dir.dot(u.pos - f.center) < 0.0:
							dir = -dir
				elif _lane_axis.size() == 2:
					dir = _lane_axis[1]
					var half: float = (client.map_geometry().size / 2.0 - _lane_axis[0]).dot(dir)
					if (u.pos - _lane_axis[0]).dot(dir) > half:
						dir = -dir
				body.rotation.y = atan2(dir.x, dir.y)
				body.set_meta("faced", true)
		elif u.minion:
			p.y = 0.45
		elif u.turret:
			p.y = 1.2
		elif u.kind in ["gatehouse", "base", "relic", "monster"]:
			p.y = 0.0
		body.position = p
		if u.champion != "":
			body.scale = Vector3.ONE * (float(u.gameplay_radius) / CHAMPION_RADIUS_U)
		if body.has_node("Protected"):
			body.get_node("Protected").visible = u.protected
			if body.has_node("Model") and (not body.has_meta("shown_protected") or body.get_meta("shown_protected") != u.protected):
				body.set_meta("shown_protected", u.protected)
				for m in body.get_node("Model").get_meta("prop_mats", []):
					m.set_shader_parameter("protected_glow", 1.0 if u.protected else 0.0)
		_show_windup(body, u.get("windup", -1.0), u.get("windup_dir", Vector2.ZERO))
		_show_statuses(body, u.stunned, u.rooted, u.shield, u.get("slowed", false))
		if u.champion != "":
			_show_recall(body, u.get("recalling", false), ALLY_COLOR if u.ally else ENEMY_COLOR, u.champion)
		if u.champion != "" or (u.minion and body.has_meta("rig")):
			_animate(body, u, delta)
		if u.minion:
			body.set_meta("last_health", float(u.get("health", 1.0)))
	for id in remote_bodies.keys():
		if not seen.has(id):
			var gone: Node3D = remote_bodies[id]
			remote_bodies.erase(id)
			# Structures are always visible: one that's gone fell. It leaves rubble.
			if gone.has_meta("prop"):
				_leave_rubble(gone)
			# A minion last seen at 0 health died (rather than leaving our vision): it plays its
			# death where it fell, then sinks away (A5).
			if _models_shown(gone) and float(gone.get_meta("last_health", 1.0)) <= 0.0:
				for c in gone.get_children():
					c.visible = c == gone.get_meta("rig").model
				corpses.append({ "body": gone, "t": 0.0 })
			else:
				gone.queue_free()
	for c in corpses.duplicate():
		c.t += delta
		var rig: Dictionary = c.body.get_meta("rig")
		rig.animator.drive(rig.skeleton, { "dead": true }, 0.0, delta)
		if c.t > 1.4:
			c.body.position.y -= delta * 0.6
		if c.t > 2.4:
			c.body.queue_free()
			corpses.erase(c)


## Minions: their pack's model (A5) in the team color, else a short capsule; either way the
## ground ring is the *collision* radius, so minion block is visible exactly as the simulation
## sees it (D11).
## Jungle monsters' colors (01 §7): the buff camps in their buff's color, the others in the
## jungle's own browns, greens and greys.
const MONSTER_COLORS := {
	"warden": Color(0.32, 0.5, 0.85), "brute": Color(0.82, 0.32, 0.14),
	"hound_alpha": Color(0.42, 0.38, 0.34), "hound": Color(0.5, 0.46, 0.4),
	"toad": Color(0.34, 0.52, 0.24), "raven_alpha": Color(0.32, 0.24, 0.4), "raven": Color(0.38, 0.3, 0.46),
	"crawler_elder": Color(0.5, 0.5, 0.48), "crawler": Color(0.58, 0.57, 0.53),
}
const NEUTRAL_COLOR := Color(0.95, 0.75, 0.25)


## A placeholder jungle monster (no models yet): a faceted body sized to its hitbox, a head
## facing ahead, horns on the buff camps' guardians and ears on the hounds, a gold ring at its
## feet. The ring and bar are the neutral gold, so any team reads it as "not a champion".
func _make_monster(key: String, collision_radius_u: float) -> Node3D:
	var color: Color = MONSTER_COLORS.get(key, Color(0.5, 0.45, 0.4))
	var r := collision_radius_u * UNITS_TO_METERS
	var root := Node3D.new()
	var mat := StandardMaterial3D.new()
	mat.albedo_color = color
	mat.roughness = 0.9
	var body := MeshInstance3D.new()
	var sphere := SphereMesh.new()
	sphere.radius = r
	sphere.height = r * 1.5
	sphere.radial_segments = 7
	sphere.rings = 4
	body.mesh = sphere
	body.position = Vector3(0, r * 0.75, 0)
	body.material_override = mat
	root.add_child(body)
	var head := MeshInstance3D.new()
	var hs := SphereMesh.new()
	hs.radius = r * 0.45
	hs.height = r * 0.8
	hs.radial_segments = 6
	hs.rings = 3
	head.mesh = hs
	head.position = Vector3(0, r * 1.15, r * 0.8)
	head.material_override = mat
	root.add_child(head)
	if key in ["warden", "brute", "crawler_elder", "hound_alpha", "hound"]:
		for side in [-1.0, 1.0]:
			var horn := MeshInstance3D.new()
			var cone := CylinderMesh.new()
			cone.top_radius = 0.0
			cone.bottom_radius = r * 0.12
			cone.height = r * (0.7 if key in ["warden", "brute"] else 0.4)
			cone.radial_segments = 4
			horn.mesh = cone
			horn.position = head.position + Vector3(side * r * 0.3, r * 0.4, -r * 0.05)
			horn.rotation_degrees = Vector3(-20, 0, side * -25)
			horn.material_override = _unshaded(color.lightened(0.45))
			root.add_child(horn)
	var ring := MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.outer_radius = r
	torus.inner_radius = r - 0.03
	ring.mesh = torus
	ring.position = Vector3(0, 0.02, 0)
	ring.material_override = _unshaded(NEUTRAL_COLOR)
	root.add_child(ring)
	return root


func _make_minion(color: Color, collision_radius_u: float, kind := "") -> MeshInstance3D:
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
	var pack := _minion_model(kind)
	if pack != null:
		var outline := _outline_material(color)
		body.set_meta("outline", outline)
		body.set_meta("placeholder_mesh", body.mesh)
		_attach_model(body, color, "", pack, MINION_LIFT)
		_apply_model_mode(body)
	return body


## Walls (extruded, vision-blocking) and brush (low translucent tufts) from the map the server
## announced. The same polygons drive collision, pathing and vision in the simulation.
var _map_built := false
var _lane_axis := []                      # a lane map's [blue fountain, unit direction to red's]
var _road_segments := PackedVector4Array()  # its roads (blue to red) as segments, in meters


## The ground covers the map and the scenery around it (`margin_u` past every edge).
func _size_ground(size_u: Vector2, margin_u := 0.0) -> void:
	var plane := PlaneMesh.new()
	plane.size = (size_u + Vector2(margin_u, margin_u) * 2.0) * UNITS_TO_METERS
	ground.mesh = plane
	ground.position = Vector3(size_u.x, 0, size_u.y) * (UNITS_TO_METERS / 2.0)


func _build_map() -> void:
	_map_built = true
	var geo: Dictionary = client.map_geometry()
	var size: Vector2 = geo.size
	_size_ground(size, 2600.0)
	ground.layers = MAP_LAYERS
	_build_minimap(geo)
	atmosphere = preload("res://scripts/atmosphere.gd").new()
	add_child(atmosphere)
	atmosphere.setup(size / 2.0 * UNITS_TO_METERS)
	# The dressing: forest, rocks, tall grass (art/props), where the packs are present.
	var dressing := preload("res://scripts/scenery.gd").new()
	dressing.name = "Scenery"
	add_child(dressing)
	dressing.build(geo, func(id: String):
		var model := _prop_model(id)
		if model == null:
			return null
		var mats := _prop_materials(model, Color.WHITE, true)
		for m in mats:
			m.set_shader_parameter("sway", 0.05 if id.begins_with("grass") else (0.012 if id.begins_with("pine") else 0.0))
		return [model.static_mesh(), mats])
	for c in dressing.get_children():
		c.layers = MAP_LAYERS
	if geo.fountains.size() == 2:
		# A lane map: roads down its lanes (one from fountain to fountain on The Bridge), and
		# the river if it has one.
		var a: Vector2 = geo.fountains[0].center
		var b: Vector2 = geo.fountains[1].center
		var gm: ShaderMaterial = ground.material_override
		gm.set_shader_parameter("lane_mode", 1.0)
		var roads: Array = geo.get("roads", [])
		_road_segments = _segments(roads)
		gm.set_shader_parameter("roads", _road_segments)
		gm.set_shader_parameter("road_count", _road_segments.size())
		gm.set_shader_parameter("lane_width", 13.0 if roads.size() <= 1 else 9.5)
		var river := _segments([geo.get("river", PackedVector2Array())])
		gm.set_shader_parameter("river", river)
		gm.set_shader_parameter("river_count", river.size())
		_lane_axis = [a, (b - a).normalized()]
	for f in geo.fountains:
		# The spawn platform (a prop the size of the fountain's circle), else a tinted disk.
		var platform := _prop_node("fountain", ALLY_COLOR if f.ally else ENEMY_COLOR)
		if platform != null:
			platform.position = Vector3(f.center.x * UNITS_TO_METERS, 0.0, f.center.y * UNITS_TO_METERS)
			platform.scale = Vector3.ONE * (f.radius / 600.0)
			platform.layers = MAP_LAYERS
			add_child(platform)
			continue
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
		if _prop_model("rock_1") != null and preload("res://scripts/scenery.gd").is_outcrop(poly):
			continue  # drawn as a rock cluster by the scenery
		var wall := _extrude(poly, 1.4, wall_mat)
		wall.layers = MAP_LAYERS
		add_child(wall)
	# Tall grass props fill the brush when they're present; else a swaying block of brush.
	if _prop_model("grass_1") == null:
		for poly in geo.brush:
			var b := _extrude(poly, 0.55, brush_mat)
			b.layers = MAP_LAYERS
			add_child(b)
	_render_minimap_terrain()


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


## ---- Map props (art/props: structures, trees, rocks) ------------------------------------------
## Static models from packs, validated like the champions (11 §4), drawn with the prop shader.

var _prop_models := {}                    # id -> MftrModel (or null without a pack)


func _prop_model(id: String) -> MftrModel:
	if not _prop_models.has(id):
		var path := _art_path("props/%s/export/%s.glb" % [id, id])
		_prop_models[id] = MftrModel.load(path) if FileAccess.file_exists(path) else null
	return _prop_models[id]


func _prop_materials(model: MftrModel, team: Color, tinted := false) -> Array:
	var mats := []
	for slot in model.surface_slots():
		var m := ShaderMaterial.new()
		m.shader = load("res://shaders/prop.gdshader")
		m.set_shader_parameter("slot", MINION_SLOTS.find(slot))
		m.set_shader_parameter("team_accent", team)
		m.set_shader_parameter("instance_tint", tinted)
		mats.append(m)
	return mats


## One prop as a node (null without its pack).
func _prop_node(id: String, team: Color) -> MeshInstance3D:
	var model := _prop_model(id)
	if model == null:
		return null
	var n := MeshInstance3D.new()
	n.mesh = model.static_mesh()
	var mats := _prop_materials(model, team)
	for i in mats.size():
		n.set_surface_override_material(i, mats[i])
	n.set_meta("prop_mats", mats)
	return n


## A structure from its prop (turned to face down the lane when placed); null without one.
func _structure_prop(id: String, color: Color, dome_radius: float, dome_lift: float) -> Node3D:
	var model := _prop_node(id, color)
	if model == null:
		return null
	var root := Node3D.new()
	model.name = "Model"
	root.add_child(model)
	# Protected (an earlier structure in its lane stands): a pale ward ring on the ground and a
	# cold sheen on the stone, instead of a dome hiding the model.
	var ward := MeshInstance3D.new()
	ward.name = "Protected"
	var ring := TorusMesh.new()
	ring.inner_radius = dome_radius - 0.08
	ring.outer_radius = dome_radius
	ring.rings = 48
	ward.mesh = ring
	ward.position.y = 0.04
	ward.scale = Vector3(1.0, 0.15, 1.0)
	ward.material_override = _unshaded(Color(0.75, 0.88, 1.0), 0.35)
	ward.visible = false
	root.add_child(ward)
	root.set_meta("prop", id)
	return root


var _relic_pads := {}                     # rounded position -> the pad under a relic


## A relic's pad stays where the relic floats, also while it's taken.
func _relic_pad(at: Vector2) -> void:
	var key := Vector2i(roundi(at.x), roundi(at.y))
	if _relic_pads.has(key):
		return
	var pad := _prop_node("relic_pad", Color.WHITE)
	_relic_pads[key] = pad
	if pad != null:
		pad.position = _to_world(at)
		add_child(pad)


func _leave_rubble(gone: Node3D) -> void:
	var kind: String = gone.get_meta("prop")
	var pile := _prop_node("rubble", Color.WHITE)
	if pile != null:
		pile.position = Vector3(gone.position.x, 0.0, gone.position.z)
		pile.rotation.y = gone.rotation.y
		pile.scale = Vector3.ONE * {"turret": 1.0, "gatehouse": 1.5, "base": 2.0}.get(kind, 1.0)
		add_child(pile)
	# A cloud of stone dust where it fell.
	var dust := {"kit": "dust", "ramp": PackedColorArray([Color(0.62, 0.6, 0.55), Color(0.5, 0.48, 0.45), Color(0.36, 0.35, 0.33)]),
		"count": 40, "size": 2.2, "speed": 3.0, "lifetime": 1.4}
	vfx.play(dust, Vector3(gone.position.x, 0.0, gone.position.z))


func _make_turret(color: Color, collision_radius_u: float) -> Node3D:
	var built := _structure_prop("turret", color, 1.5, 1.6)
	if built != null:
		return built
	return _make_turret_shape(color, collision_radius_u)


func _make_turret_shape(color: Color, collision_radius_u: float) -> MeshInstance3D:
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
	var built := _structure_prop("gatehouse", color, 2.0, 0.0)
	if built != null:
		return built
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
	var built := _structure_prop("base", color, 2.6, 0.0)
	if built != null:
		return built
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
			# brief smear from the caster's hand (its projectile socket, 10 §4.4) to the missile
			# ties the two together. Our own leave from our socket the same way.
			if m.side == "enemy" and remote_bodies.has(m.owner):
				_spawn_streak(_socket_world(remote_bodies[m.owner], "socket_projectile"), world + Vector3(0, 0.3, 0), ENEMY_COLOR)
			elif m.side == "own" and _models_shown(own_body):
				_spawn_streak(_socket_world(own_body, "socket_projectile"), world + Vector3(0, 0.3, 0), OWN_COLOR)
			var champ: String = m.get("champion", "")
			var action: String = m.get("action", "")
			if champ != "" and action != "":
				var owner = _body_of(m.owner)
				var src: Vector3 = _socket_world(owner, "socket_projectile") if owner != null else world
				var dir3 := Vector3(m.dir.x, 0, m.dir.y)
				_vfx_play(champ, action, "release", src, dir3, 0.0, "rel%d_%s" % [m.owner, action])
				for e in _effects(champ, action, "projectile"):
					vfx.decorate(node, e, float(m.radius) * UNITS_TO_METERS)
				node.set_meta("vfx", [champ, action])
		var node: Node3D = missile_nodes[key]
		var dir: Vector2 = m.dir
		node.position = world
		node.rotation = Vector3(0, -atan2(dir.y, dir.x), 0)
		node.get_node("Body").visible = not m.impact
		node.get_node("Impact").visible = m.impact
		for c in node.get_children():
			if c.name != "Body" and c.name != "Impact":
				c.visible = not m.impact
		if m.impact and node.has_meta("vfx") and not node.has_meta("impacted"):
			node.set_meta("impacted", true)
			var v: Array = node.get_meta("vfx")
			_vfx_play(v[0], v[1], "impact", world + Vector3(0, 0.6, 0), Vector3.ZERO, 0.0, "imp%d" % key)
			if m.get("hard_cc", false):
				_sfx_play(v[0], ["cc.hard"], world)
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
		var center := Vector3(a.center.x * UNITS_TO_METERS, 0.03, a.center.y * UNITS_TO_METERS)
		var champ: String = a.get("champion", "")
		var action: String = a.get("action", "")
		var tag := "area%d_%d_%d" % [a.get("owner", 0), roundi(a.center.x), roundi(a.center.y)]
		if not area_nodes.has(key):
			var node := _make_area(a.side, a.radius, a.get("hard_cc", false))
			add_child(node)
			area_nodes[key] = node
			var now := Time.get_ticks_msec() / 1000.0
			if champ != "" and now - float(_vfx_recent.get(tag, -10.0)) > 1.5:
				_vfx_recent[tag] = now
				var owner = _body_of(a.get("owner", -1))
				for e in _effects(champ, action, "projectile"):
					if e.kit == "lob" and owner != null:
						vfx.lob(tag.hash(), e, _socket_world(owner, "socket_projectile"), center, 0.45)
				# A thrown area's release (a nova around the caster has its cast sound).
				if owner != null and client.action_info(champ, action).get("shape", "") != "nova":
					_sfx_play(champ, [action + ".release", "*.release"], owner.global_position)
		if a.detonated and not area_nodes[key].has_meta("detonated"):
			area_nodes[key].set_meta("detonated", true)
			vfx.land(tag.hash())
			_vfx_play(champ, action, "detonate", center, Vector3.ZERO, float(a.radius) * UNITS_TO_METERS, "det" + tag)
			if a.get("hard_cc", false):
				_sfx_play(champ, ["cc.hard"], center)
		var node: MeshInstance3D = area_nodes[key]
		node.position = center
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
			var champ: String = b.get("champion", "")
			if champ != "":
				var owner = _body_of(b.owner)
				var src: Vector3 = _socket_world(owner, "socket_projectile") if owner != null else node.position
				_vfx_play(champ, "attack", "release", src, Vector3(b.dir.x, 0, b.dir.y), 0.0, "atk%d" % b.owner)
				for e in _effects(champ, "attack", "projectile"):
					vfx.decorate(node, e, 0.09, 0.0)
				node.set_meta("vfx", champ)
		var bolt: Node3D = bolt_nodes[key]
		bolt.position = Vector3(b.pos.x * UNITS_TO_METERS, 1.0, b.pos.y * UNITS_TO_METERS)
		bolt.rotation = Vector3(0, -atan2(b.dir.y, b.dir.x), 0)
	for key in bolt_nodes.keys():
		if not seen.has(key):
			var bolt: Node3D = bolt_nodes[key]
			if bolt.has_meta("vfx"):
				_vfx_play(bolt.get_meta("vfx"), "attack", "impact", bolt.position, Vector3.ZERO, 0.0, "atkhit%d" % key)
			bolt.queue_free()
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


## Confirmed damage only (03a §7): numbers float up from the unit that took it. Only what
## concerns us shows (what we deal, take and heal; a spectator, the champion they follow), so
## a teamfight isn't buried in minions' numbers. Colored by damage type like the tooltips,
## bigger for bigger hits; damage we take is red.
func _update_combat_text(delta: float) -> void:
	var me := client.own_unit_id() if not client.is_spectator() else spectate_target
	for c in client.take_combat_text():
		if not c.heal:
			_flash(c.target)
		if c.source != me and c.target != me:
			continue
		var p = _unit_world_pos(c.target)
		if p == null:
			continue
		var total: float = c.amount + c.absorbed
		if total < 0.5:
			continue
		var color: Color = {"physical": Color(1.0, 0.62, 0.24), "magic": Color(0.45, 0.68, 1.0), "true": Color(1, 1, 1)}.get(c.kind, Color(1.0, 0.62, 0.24))
		var text := "%d" % roundi(total)
		if c.heal:
			color = Color(0.42, 0.95, 0.5)
			text = "+" + text
		elif c.target == me:
			color = Color(1.0, 0.3, 0.28)
			text = "-" + text
		var size := clampf(19.0 + total / 28.0, 19.0, 34.0)
		floaters.append({ "pos": p, "text": text, "color": color, "age": 0.0, "size": size, "drift": randf_range(-0.6, 0.6), "life": 1.0 })
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
			if champion_model != null:
				sfx.play_flat(_sfx_table("").get("match.victory" if n.won else "match.defeat", []))
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
	floaters = floaters.filter(func(f): return f.age < f.get("life", 0.9))
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
	if loading_panel != null and loading_panel.modulate.a > 0.5:
		return  # the loading screen covers the match until it fades
	var font := ThemeDB.fallback_font
	if client.phase() == "playing" and client.is_spectator():
		var who: String = remote_info[spectate_target].champion if remote_info.has(spectate_target) else "the map"
		overlay.draw_string(font, Vector2(0, overlay.size.y - 40), "Spectating %s   —   N: next champion   ·   Tab: match breakdown" % who, HORIZONTAL_ALIGNMENT_CENTER, overlay.size.x, 22, Color.WHITE)
	if own_body != null and own_body.visible:
		var hp: float = own_status.get("health", 0.0)
		var mx: float = own_status.get("max_health", 1.0)
		_draw_bar(own_body.position + Vector3(0, 1.25, 0), Vector2(110, 12), hp, mx, own_status.get("shield", 0.0), Color(0.3, 0.82, 0.32), "own", own_status.get("level", 0), 100.0)
		_draw_unit_label(font, own_body.position + Vector3(0, 1.25, 0), Vector2(110, 12), "", true, own_status)
		_draw_augment_pips(font, own_body.position + Vector3(0, 1.25, 0), own_status.get("augments", []))
	for id in remote_info:
		var u: Dictionary = remote_info[id]
		if u.kind == "relic" or not remote_bodies.has(id):
			continue
		var champ: bool = u.champion != ""
		var structure: bool = u.kind in ["turret", "gatehouse", "base"]
		var color := Color(0.3, 0.7, 0.95) if u.ally else Color(0.9, 0.25, 0.2)
		var size := Vector2(110, 12) if champ else (Vector2(150, 10) if structure else Vector2(62, 6))
		var lift := 1.25 if champ else (2.6 if structure else 0.6)
		if u.get("neutral", false):
			color = NEUTRAL_COLOR
			size = Vector2(110, 9) if u.get("big", false) else Vector2(62, 6)
			lift = float(u.radius) * UNITS_TO_METERS * 1.9 + 0.3
		if remote_bodies[id].has_meta("prop"):
			lift = {"turret": 7.0, "gatehouse": 5.4, "base": 6.2}.get(u.kind, 3.0)
		_draw_bar(remote_bodies[id].position + Vector3(0, lift, 0), size, u.health, u.max_health, u.shield, color, id, u.level if champ else 0, 100.0 if champ else 0.0)
		if u.get("plates", 0) > 0:
			_draw_plates(remote_bodies[id].position + Vector3(0, lift, 0), size, u.plates)
		if champ:
			_draw_augment_pips(font, remote_bodies[id].position + Vector3(0, lift, 0), u.get("augments", []))
			_draw_unit_label(font, remote_bodies[id].position + Vector3(0, lift, 0), size, u.champion, u.ally, u)
	var k := _hud_k()
	for f in floaters:
		# Pops in large, settles, rises along a slight arc to one side, fades.
		var life: float = f.get("life", 0.9)
		var t: float = f.age / life
		var rise := 1.6 + 1.1 * (1.0 - pow(1.0 - t, 2.0))
		var s = _screen(f.pos + Vector3(f.get("drift", 0.0) * t, rise, 0))
		if s == null:
			continue
		var c: Color = f.color
		c.a = clampf((1.0 - t) * 3.0, 0.0, 1.0)
		var pop := 1.0 + 0.45 * clampf(1.0 - f.age / 0.12, 0.0, 1.0)
		var fs := roundi(f.get("size", 20.0) * pop * k)
		var bold := _bold_font()
		overlay.draw_string_outline(bold, s + Vector2(-80, 0), f.text, HORIZONTAL_ALIGNMENT_CENTER, 160, fs, maxi(4, fs / 5), Color(0, 0, 0, 0.85 * c.a))
		overlay.draw_string(bold, s + Vector2(-80, 0), f.text, HORIZONTAL_ALIGNMENT_CENTER, 160, fs, c)
	_draw_ability_bar(font)
	_draw_top_right(font)
	if _chips.size() > remote_info.size() + 32:
		for key in _chips.keys():
			if key is int and not remote_info.has(key):
				_chips.erase(key)


## Over a champion's bar: its name (enemies and allies; not our own), and an icon for each
## status it's under (stunned, rooted, slowed) to the bar's right, the icons the tooltips use.
func _draw_unit_label(font: Font, world: Vector3, size: Vector2, name: String, ally: bool, u: Dictionary) -> void:
	var s = _screen(world)
	if s == null:
		return
	var k := _hud_k()
	var w := size * k
	if name != "" and u.get("augments", []).is_empty():
		var c := Color(0.75, 0.88, 1.0) if ally else Color(1.0, 0.72, 0.66)
		Hud.text(overlay, font, s + Vector2(-w.x / 2.0, -w.y - 4.0 * k), name, roundi(13.0 * k), c, HORIZONTAL_ALIGNMENT_CENTER, w.x)
	var x: float = s.x + w.x / 2.0 + 4.0 * k
	var ic := 16.0 * k
	for status in ["stunned", "rooted", "slowed"]:
		if u.get(status, false):
			var kind: String = {"stunned": "stun", "rooted": "root", "slowed": "slow"}[status]
			overlay.draw_texture_rect(AbilityTooltip.icon(kind), Rect2(Vector2(x, s.y - w.y - (ic - w.y) / 2.0), Vector2(ic, ic)), false)
			x += ic + 2.0 * k


## Augment indicators (06 §3: no invisible power): one tier-colored diamond per held augment,
## marked with its initial, in a row above a champion's health bar.
func _draw_augment_pips(font: Font, world: Vector3, held: Array) -> void:
	if held.is_empty():
		return
	var s = _screen(world)
	if s == null:
		return
	var step := 18.0
	var x0: float = s.x - step * (held.size() - 1) / 2.0
	for i in held.size():
		var a: Dictionary = held[i]
		var c := Vector2(x0 + step * i, s.y - 20)
		var r := 8.0
		var pts := PackedVector2Array([c + Vector2(0, -r), c + Vector2(r, 0), c + Vector2(0, r), c + Vector2(-r, 0)])
		overlay.draw_colored_polygon(pts, _tier_color(a.get("tier", "")).darkened(0.35))
		pts.append(pts[0])
		overlay.draw_polyline(pts, _tier_color(a.get("tier", "")), 1.5)
		var initial: String = String(a.get("name", "?")).left(1)
		overlay.draw_string(font, c + Vector2(-6, 4), initial, HORIZONTAL_ALIGNMENT_CENTER, 12, 10, Color.WHITE)


## A health bar over a unit: a dark frame, health with a lighter top, shield in white, recent
## damage as a draining pale chip, ticks every 100 health for champions, and a level box.
func _draw_bar(world: Vector3, size: Vector2, hp: float, max_hp: float, shield: float, color: Color, id = null, level := 0, tick := 0.0) -> void:
	var s = _screen(world)
	if s == null or max_hp <= 0.0:
		return
	var k := _hud_k()
	size *= k
	var origin: Vector2 = s - Vector2(size.x / 2.0, size.y)
	var chip := _chip_of(id, hp, max_hp) if id != null else hp
	Hud.bar(overlay, Rect2(origin, size), hp, max_hp, color, shield, Color(0.95, 0.95, 0.95), tick, chip)
	if level > 0:
		var lb := Rect2(origin - Vector2(size.y + 9.0 * k, 3.0 * k), Vector2(size.y + 7.0 * k, size.y + 6.0 * k))
		overlay.draw_rect(lb, Color(0.04, 0.05, 0.07, 0.95))
		overlay.draw_rect(lb, Hud.GOLD_DIM, false, 1.0)
		overlay.draw_string(ThemeDB.fallback_font, lb.position + Vector2(0, lb.size.y - 4.0 * k), "%d" % level, HORIZONTAL_ALIGNMENT_CENTER, lb.size.x, roundi(12.0 * k), Color.WHITE)


## A plated turret's plates (01 §3): a gold pip under its bar for each one left, a dim one for
## each broken, and a notch on the bar where each next plate breaks.
func _draw_plates(world: Vector3, size: Vector2, plates: int) -> void:
	var s = _screen(world)
	if s == null:
		return
	var k := _hud_k()
	size *= k
	var origin: Vector2 = s - Vector2(size.x / 2.0, size.y)
	for i in range(1, 5):
		var x := origin.x + size.x * i / 5.0
		overlay.draw_line(Vector2(x, origin.y), Vector2(x, origin.y + size.y), Color(0.05, 0.05, 0.07, 0.9), maxf(1.0, k))
	var pip := Vector2(size.x / 5.0 - 3.0 * k, 4.0 * k)
	for i in 5:
		var r := Rect2(Vector2(origin.x + size.x * i / 5.0 + 1.5 * k, origin.y + size.y + 2.0 * k), pip)
		overlay.draw_rect(r, Hud.GOLD if i < plates else Color(0.2, 0.18, 0.12, 0.8))


func _draw_ability_bar(font: Font) -> void:
	_bar_rect = Rect2()
	_ability_boxes.clear()
	_level_buttons.clear()
	if not own_status.is_empty() and own_status.has("cooldowns"):
		_draw_bottom_hud(font)
	_draw_hud_messages(font)


## The bottom panel (05 §6): stats, the portrait with level and XP, the abilities with their
## cooldowns, ranks and level-up buttons, health, then items and gold. Centered, shifted left of
## the minimap when they'd meet; scaled with the window (k = 1 at 1080 p).
func _draw_bottom_hud(font: Font) -> void:
	var k := _hud_k()
	var bold := _bold_font()
	var ranked: bool = own_status.get("ranked", false)
	var cds: Array = own_status.cooldowns
	var mouse := overlay.get_local_mouse_position()
	var A := 66.0 * k                     # ability icon
	var S := 50.0 * k                     # spell icon
	var G := 7.0 * k                      # gap
	var I := Vector2(43.0, 43.0) * k      # item box
	var pad := 12.0 * k
	var stats_w := 156.0 * k if ranked else 0.0
	var port := 100.0 * k
	var abil_w := 4.0 * A + 3.0 * G + 14.0 * k + 2.0 * S + G
	var items_w := 3.0 * I.x + 2.0 * 5.0 * k if ranked else 0.0
	var w := pad + (stats_w + pad if ranked else 0.0) + port + pad + abil_w + (pad + items_w if ranked else 0.0) + pad
	var h := 136.0 * k
	var limit: float = minimap.position.x - 10.0 * k if minimap != null and minimap.visible else overlay.size.x
	var x0 := minf(overlay.size.x / 2.0 - w / 2.0, limit - w)
	x0 = maxf(x0, 6.0)
	var y0 := overlay.size.y - h - 6.0 * k
	_bar_rect = Rect2(x0, y0, w, h)
	Hud.panel(overlay, _bar_rect, k)
	var x := x0 + pad

	# Stats: attack damage, ability power, armor, magic resist, attack speed, haste, move speed.
	if ranked:
		var sr := Rect2(x, y0 + pad, stats_w, h - pad * 2.0)
		Hud.panel(overlay, sr, k, Hud.INK_2, Color(0, 0, 0, 0))
		var rows := [
			["ad", "%d" % roundi(own_status.attack_damage)], ["ap", "%d" % roundi(own_status.ability_power)],
			["armor", "%d" % roundi(own_status.armor)], ["mr", "%d" % roundi(own_status.magic_resist)],
			["as", "%.2f" % own_status.attack_speed], ["haste", "%d" % roundi(own_status.ability_haste)],
			["ms", "%d" % roundi(own_status.move_speed)],
		]
		var cw := stats_w / 2.0
		var rh := (sr.size.y - 8.0 * k) / 4.0
		for i in rows.size():
			var at := sr.position + Vector2(6.0 * k + (i % 2) * cw, 4.0 * k + (i / 2) * rh)
			var ic := 16.0 * k
			overlay.draw_texture_rect(Hud.stat_icon(rows[i][0]), Rect2(at + Vector2(0, (rh - ic) / 2.0), Vector2(ic, ic)), false)
			overlay.draw_string(font, at + Vector2(ic + 5.0 * k, rh / 2.0 + 5.0 * k), rows[i][1], HORIZONTAL_ALIGNMENT_LEFT, -1, roundi(14.0 * k), Hud.TEXT)
		x += stats_w + pad

	# Portrait: the champion's color and initial, the XP ring, the level badge.
	var champ: String = own_status.champion
	var pc := Vector2(x + port / 2.0, y0 + h / 2.0 - 4.0 * k)
	var pr := port / 2.0 - 6.0 * k
	var tint: Color = CHAMPION_COLORS.get(champ, Color(0.5, 0.5, 0.55))
	overlay.draw_circle(pc, pr + 5.0 * k, Color(0, 0, 0, 0.9))
	var face: Texture2D = portraits.portrait(champ, "round")
	if face != null:
		overlay.draw_texture_rect(face, Rect2(pc - Vector2(pr, pr), Vector2(pr, pr) * 2.0), false)
	else:
		overlay.draw_circle(pc, pr, tint.darkened(0.35))
		overlay.draw_circle(pc + Vector2(-pr * 0.25, -pr * 0.3), pr * 0.62, tint.lightened(0.05))
		Hud.text(overlay, bold, pc + Vector2(-pr, pr * 0.36), champ.left(1), roundi(pr * 1.05), Color(1, 1, 1, 0.92), HORIZONTAL_ALIGNMENT_CENTER, pr * 2.0)
	overlay.draw_arc(pc, pr + 2.5 * k, 0.0, TAU, 64, Hud.GOLD_DIM, 3.5 * k, true)
	if ranked:
		var xp_frac := float(own_status.xp) / maxf(float(own_status.xp_next), 1.0)
		if xp_frac > 0.0:
			overlay.draw_arc(pc, pr + 2.5 * k, -PI / 2.0, -PI / 2.0 + TAU * clampf(xp_frac, 0.0, 1.0), 64, Color(0.68, 0.5, 1.0), 3.5 * k, true)
		var lc := pc + Vector2(0, pr + 2.0 * k)
		overlay.draw_circle(lc, 13.0 * k, Color(0.05, 0.06, 0.08))
		overlay.draw_arc(lc, 13.0 * k, 0.0, TAU, 32, Hud.GOLD, 1.5 * k, true)
		Hud.text(overlay, bold, lc + Vector2(-13.0 * k, 5.5 * k), "%d" % own_status.level, roundi(15.0 * k), Color.WHITE, HORIZONTAL_ALIGNMENT_CENTER, 26.0 * k)
	x += port + pad

	# Abilities: Q W E R, then the D and F spells a little smaller.
	_refresh_ability_kinds()
	var names: Array = own_status.abilities
	var top := y0 + 22.0 * k
	for slot in 6:
		var size := A if slot < 4 else S
		var bx := x + slot * (A + G) if slot < 4 else x + 4.0 * A + 3.0 * G + 14.0 * k + (slot - 4) * (S + G)
		var box := Rect2(bx, top + (A - size), size, size)
		_ability_boxes.append(box)
		var cd: float = cds[slot]
		if cd <= 0.0:
			_cd_total[slot] = 0.0
		elif cd > _cd_total[slot]:
			_cd_total[slot] = cd
		var rank: int = own_status.ranks[slot] if slot < 4 and ranked else 1
		var kind: String = _ability_kinds.get(slot, "line")
		overlay.draw_rect(box.grow(2.0 * k), Color(0, 0, 0, 0.9))
		overlay.draw_texture_rect(Hud.ability_icon(kind, tint if slot < 4 else Color(0.36, 0.42, 0.5)), box, false, Color(1, 1, 1) if rank > 0 else Color(0.4, 0.4, 0.42))
		if rank == 0:
			overlay.draw_rect(box, Color(0, 0, 0, 0.5))
		elif cd > 0.0:
			Hud.cooldown_sweep(overlay, box, cd / maxf(_cd_total[slot], 0.01))
			var label := "%.1f" % cd if cd < 1.0 else "%d" % ceili(cd)
			Hud.text(overlay, bold, box.position + Vector2(0, size / 2.0 + 8.0 * k), label, roundi(22.0 * k if slot < 4 else 18.0 * k), Color.WHITE, HORIZONTAL_ALIGNMENT_CENTER, size)
		var hot := box.has_point(mouse)
		var rim := Hud.GOLD if cd <= 0.0 and rank > 0 else Hud.GOLD_DIM
		if hot:
			rim = Color(1.0, 0.92, 0.65)
		overlay.draw_rect(box, rim, false, 1.5 * k)
		# The key, in a little tab at the bottom-left corner.
		var key: String = settings.primary_binding(SLOT_ACTIONS[slot])
		var kf := roundi(11.0 * k)
		var kw := font.get_string_size(key, HORIZONTAL_ALIGNMENT_LEFT, -1, kf).x + 8.0 * k
		var kr := Rect2(box.position + Vector2(0, size - 15.0 * k), Vector2(kw, 15.0 * k))
		overlay.draw_rect(kr, Color(0.03, 0.04, 0.05, 0.92))
		overlay.draw_string(font, kr.position + Vector2(4.0 * k, 11.5 * k), key, HORIZONTAL_ALIGNMENT_LEFT, -1, kf, Hud.TEXT)
		if slot < 4 and ranked:
			# Rank pips under the icon; a level-up tab above it while a point can go here.
			var pips := 3 if slot == 3 else 5
			var pw := (size - (pips - 1) * 3.0 * k) / pips
			for i in pips:
				var c := Color(1.0, 0.82, 0.35) if i < rank else Color(0.2, 0.22, 0.26)
				overlay.draw_rect(Rect2(box.position.x + i * (pw + 3.0 * k), box.end.y + 4.0 * k, pw, 4.0 * k), c)
			if own_status.can_rank[slot]:
				var btn := Rect2(box.position.x + size / 2.0 - 15.0 * k, y0 - 9.0 * k, 30.0 * k, 22.0 * k)
				_level_buttons[slot] = btn
				var on := btn.has_point(mouse)
				var tri := PackedVector2Array([btn.position + Vector2(btn.size.x / 2.0, 3.0 * k), Vector2(btn.end.x - 4.0 * k, btn.end.y - 4.0 * k), Vector2(btn.position.x + 4.0 * k, btn.end.y - 4.0 * k)])
				Hud.panel(overlay, btn, k, Color(0.3, 0.22, 0.05, 0.95) if not on else Color(0.5, 0.38, 0.1, 0.98), Hud.GOLD)
				var pulse := 0.75 + 0.25 * sin(Time.get_ticks_msec() / 180.0)
				overlay.draw_colored_polygon(tri, Color(1.0, 0.86, 0.4, pulse))
		if slot == 5 and names[5] != "Barrier" and names[5] != "":
			# An augment's spell in F's place: a gold corner mark.
			overlay.draw_colored_polygon(PackedVector2Array([box.position + Vector2(size - 12.0 * k, 0), box.position + Vector2(size, 0), box.position + Vector2(size, 12.0 * k)]), Hud.GOLD)

	# Health across the abilities' width, numbers on it.
	var hp: float = own_status.health
	var mx: float = own_status.max_health
	var shield: float = own_status.shield
	var hb := Rect2(x, top + A + 16.0 * k, abil_w, 19.0 * k)
	Hud.bar(overlay, hb, hp, mx, Color(0.22, 0.72, 0.28), shield, Color(0.92, 0.94, 0.97), 100.0, _chip_of("own", hp, mx))
	var hp_text := "%d / %d" % [roundi(hp), roundi(mx)]
	if shield > 0.0:
		hp_text += "  +%d" % roundi(shield)
	var potion: float = own_status.get("potion", 0.0)
	if potion > 0.0:
		# A potion working: its icon and the seconds left, at the bar's left end.
		var pi := hb.size.y + 4.0 * k
		var pot: Dictionary = _catalog.get(27, {})
		if not pot.is_empty():
			overlay.draw_texture_rect(ItemIcons.icon(pot), Rect2(hb.position + Vector2(2.0 * k, -2.0 * k), Vector2(pi, pi)), false)
		Hud.text(overlay, font, hb.position + Vector2(pi + 6.0 * k, hb.size.y / 2.0 + 5.0 * k), "%d s" % ceili(potion), roundi(12.0 * k), Color(0.6, 1.0, 0.65))
	Hud.text(overlay, bold, hb.position + Vector2(0, hb.size.y / 2.0 + 5.0 * k), hp_text, roundi(14.0 * k), Color.WHITE, HORIZONTAL_ALIGNMENT_CENTER, hb.size.x)
	# Jungle buffs: their names and seconds left, just above the health bar.
	var bx := hb.position.x
	for buff in [["insight", "Insight", Color(0.45, 0.68, 1.0)], ["cinder", "Cinder", Color(1.0, 0.5, 0.25)]]:
		var left: float = own_status.get(buff[0], 0.0)
		if left > 0.0:
			var label := "◆ %s %d:%02d" % [buff[1], int(left) / 60, int(left) % 60]
			Hud.text(overlay, font, Vector2(bx, hb.position.y - 5.0 * k), label, roundi(12.0 * k), buff[2])
			bx += font.get_string_size(label, HORIZONTAL_ALIGNMENT_LEFT, -1, roundi(12.0 * k)).x + 12.0 * k
	var recall: float = own_status.get("recall", 0.0)
	if recall > 0.0:
		# Recalling: a channel bar above the panel, filling as home gets closer.
		var total: float = own_status.get("recall_total", 8.0)
		var rb := Rect2(x + abil_w / 2.0 - 130.0 * k, top - 46.0 * k, 260.0 * k, 16.0 * k)
		Hud.panel(overlay, rb.grow(3.0 * k), k, Color(0.04, 0.05, 0.08, 0.92), Hud.GOLD_DIM)
		overlay.draw_rect(Rect2(rb.position, Vector2(rb.size.x * clampf(1.0 - recall / total, 0.0, 1.0), rb.size.y)), Color(0.35, 0.72, 0.95))
		Hud.text(overlay, bold, rb.position + Vector2(0, rb.size.y / 2.0 + 5.0 * k), "Recall  %.1f" % recall, roundi(13.0 * k), Color.WHITE, HORIZONTAL_ALIGNMENT_CENTER, rb.size.x)
	x += abil_w + pad

	# Items in two rows of three, gold beneath; the held augments above.
	if ranked and own_status.has("items"):
		_draw_inventory(font, Vector2(x, y0 + pad), k)


## A unit's lagging health for the bar's damage chip: it holds, then drains toward the health.
func _chip_of(id, hp: float, max_hp: float) -> float:
	var c: float = _chips.get(id, hp)
	c = hp if hp >= c else maxf(hp, c - max_hp * 0.9 * _hud_dt)
	_chips[id] = c
	return c


## The HUD's scale: 1.0 at 1080 p.
func _hud_k() -> float:
	return clampf(overlay.size.y / 1080.0, 0.7, 1.35)


func _bold_font() -> Font:
	if _bold == null:
		_bold = FontVariation.new()
		_bold.base_font = ThemeDB.fallback_font
		_bold.variation_embolden = 0.7
	return _bold


## Each ability's kind (line, area, dash…) for its icon, refreshed now and then: an augment can
## replace F, and the champion can change between matches.
func _refresh_ability_kinds() -> void:
	var now := Time.get_ticks_msec()
	if now < _ability_kinds_at:
		return
	_ability_kinds_at = now + 500
	for slot in 6:
		var info: Dictionary = client.ability_info(slot)
		var kind: String = info.get("kind", "line")
		if kind == "support":
			kind = "heal" if info.has("heal") else "shield_ally"
		_ability_kinds[slot] = kind


## Messages over the world: the hint line above the bar, the match banner, the death screen, the
## blind round's timer.
func _draw_hud_messages(font: Font) -> void:
	var k := _hud_k()
	var bold := _bold_font()
	var hint := ""
	var hint_color := Color(1.0, 0.85, 0.3)
	if attack_move_armed:
		hint = "Attack-move: left-click a point"
		hint_color = ENEMY_COLOR
	elif own_status.get("points", 0) > 0:
		var keys: Array = LEVEL_ACTIONS.map(func(a): return settings.primary_binding(a))
		var joined := " / ".join(keys)
		var mod: String = keys[0].left(keys[0].rfind("+") + 1)
		if mod != "" and keys.all(func(t): return t.begins_with(mod) and t.length() == mod.length() + 1):
			joined = mod + "/".join(keys.map(func(t): return t.right(1)))  # "Alt+Q/W/E/R"
		hint = "%d ability point%s — %s or click ▲" % [own_status.points, "" if own_status.points == 1 else "s", joined]
	elif own_status.get("ranked", false) and client.can_shop():
		hint = "[%s] shop" % settings.primary_binding("toggle_shop")
	if hint != "" and _bar_rect.size.x > 0.0:
		Hud.text(overlay, font, Vector2(_bar_rect.position.x, _bar_rect.position.y - 18.0 * k), hint, roundi(15.0 * k), hint_color, HORIZONTAL_ALIGNMENT_CENTER, _bar_rect.size.x)
	if own_status.get("dead", false):
		overlay.draw_rect(Rect2(Vector2.ZERO, overlay.size), Color(0.05, 0.06, 0.08, 0.45))
		# Just above the HUD, out of the way of the fight we're watching.
		var top: float = _bar_rect.position.y if _bar_rect.size.y > 0.0 else overlay.size.y
		var r := Rect2(overlay.size.x / 2.0 - 140.0 * k, top - 128.0 * k, 280.0 * k, 78.0 * k)
		Hud.panel(overlay, r, k)
		Hud.text(overlay, font, r.position + Vector2(0, 26.0 * k), "RESPAWNING IN", roundi(14.0 * k), Hud.DIM, HORIZONTAL_ALIGNMENT_CENTER, r.size.x)
		Hud.text(overlay, bold, r.position + Vector2(0, 64.0 * k), "%d" % ceili(own_status.respawn_in), roundi(36.0 * k), Color.WHITE, HORIZONTAL_ALIGNMENT_CENTER, r.size.x)
	if match_banner != "":
		var won := match_banner == "VICTORY"
		var c := Color(0.5, 0.82, 1.0) if won else Color(1.0, 0.42, 0.36)
		var band := Rect2(0, overlay.size.y * 0.26, overlay.size.x, 132.0 * k)
		overlay.draw_rect(band, Color(0.02, 0.03, 0.05, 0.72))
		overlay.draw_rect(Rect2(band.position, Vector2(band.size.x, 2.0 * k)), c.darkened(0.2))
		overlay.draw_rect(Rect2(band.position + Vector2(0, band.size.y - 2.0 * k), Vector2(band.size.x, 2.0 * k)), c.darkened(0.2))
		Hud.text(overlay, bold, band.position + Vector2(0, 80.0 * k), match_banner, roundi(72.0 * k), c, HORIZONTAL_ALIGNMENT_CENTER, overlay.size.x)
		Hud.text(overlay, font, band.position + Vector2(0, 116.0 * k), "A new match starts shortly", roundi(18.0 * k), Hud.TEXT, HORIZONTAL_ALIGNMENT_CENTER, overlay.size.x)
	if blind_state == "playing":
		var left := maxf(blind_seconds - client.blind_elapsed(), 0.0)
		var txt := "Blind round %d / %d   %d:%02d" % [client.blind_round() + 1, client.blind_rounds(), int(left) / 60, int(left) % 60]
		overlay.draw_string(font, Vector2(16, 34), txt, HORIZONTAL_ALIGNMENT_LEFT, -1, 22, Color.WHITE)


## The top-right corner: kills and deaths, the match clock, frame rate and ping.
func _draw_top_right(font: Font) -> void:
	if client.phase() != "playing":
		return
	var k := _hud_k()
	var bold := _bold_font()
	var stats: Dictionary = client.net_stats()
	var t := client.match_seconds()
	var r := Rect2(overlay.size.x - 250.0 * k - 10.0 * k, 10.0 * k, 250.0 * k, 34.0 * k)
	Hud.panel(overlay, r, k)
	var y := r.position.y + 23.0 * k
	var kd := "%d / %d" % [stats.kills, stats.deaths]
	Hud.text(overlay, font, Vector2(r.position.x + 12.0 * k, y), "K/D", roundi(12.0 * k), Hud.DIM)
	Hud.text(overlay, bold, Vector2(r.position.x + 40.0 * k, y), kd, roundi(15.0 * k), Color.WHITE)
	Hud.text(overlay, bold, Vector2(r.position.x, y), "%d:%02d" % [int(t) / 60, int(t) % 60], roundi(16.0 * k), Color(1.0, 0.92, 0.7), HORIZONTAL_ALIGNMENT_CENTER, r.size.x)
	var perf := "%d fps  %d ms" % [Engine.get_frames_per_second(), roundi(stats.rtt_ms)]
	Hud.text(overlay, font, Vector2(r.position.x, y), perf, roundi(12.0 * k), Hud.DIM, HORIZONTAL_ALIGNMENT_RIGHT, r.size.x - 12.0 * k)
	# The kill feed under it, newest last, each line on its own dark strip.
	var fy := r.end.y + 8.0 * k
	for n in notices:
		var a := clampf(4.0 - n.age, 0.0, 1.0)
		var fs := roundi(14.0 * k)
		var tw := font.get_string_size(n.text, HORIZONTAL_ALIGNMENT_LEFT, -1, fs).x + 20.0 * k
		var strip := Rect2(overlay.size.x - 10.0 * k - tw, fy, tw, 24.0 * k)
		overlay.draw_rect(strip, Color(0.03, 0.04, 0.06, 0.7 * a))
		overlay.draw_rect(Rect2(strip.position, Vector2(3.0 * k, strip.size.y)), Color(Hud.GOLD, a))
		overlay.draw_string(font, strip.position + Vector2(10.0 * k, 17.0 * k), n.text, HORIZONTAL_ALIGNMENT_LEFT, -1, fs, Color(1, 1, 1, a))
		fy += 28.0 * k


## The hovered ability's tooltip (or `--hover-slot`'s, for scripted captures).
func _update_ability_tip() -> void:
	var slot := -1
	if not own_status.is_empty() and overlay.visible:
		var m := overlay.get_local_mouse_position()
		for i in _ability_boxes.size():
			if _ability_boxes[i].has_point(m):
				slot = i
		if slot < 0 and _shot_hover >= 0 and _shot_hover < mini(6, _ability_boxes.size()):
			slot = _shot_hover
	if not _tab_hover.is_empty() and score_panel != null and score_panel.visible:
		var c: Control = _tab_hover[2]
		if is_instance_valid(c):
			var data: Dictionary = _tab_hover[1]
			var rect := c.get_global_rect()
			if _tab_hover[0] == "item":
				ability_tip.show_custom("tab:item:%d" % data.id, func(t: RichTextLabel): _item_tooltip(t, data, 0, ""), rect, overlay.size)
			else:
				ability_tip.show_custom("tab:aug:%s" % data.name, func(t: RichTextLabel): _augment_tooltip(t, data), rect, overlay.size)
			return
	if slot < 0 and not own_status.is_empty() and overlay.visible:
		var m := overlay.get_local_mouse_position()
		for i in _item_boxes.size():
			var id: int = own_status.items[i] if i < own_status.items.size() else 0
			if (_item_boxes[i].has_point(m) or _shot_hover == 6 + i) and id != 0 and _catalog.has(id):
				var it: Dictionary = _catalog[id]
				var n: int = own_status.get("charges", [])[i] if i < own_status.get("charges", []).size() else 0
				var key := "item:%d:%d" % [id, n]
				ability_tip.show_custom(key, func(t: RichTextLabel): _item_tooltip(t, it, n, settings.primary_binding("item_%d" % (i + 1))), _item_boxes[i], overlay.size)
				return
		for row in _augment_rows:
			if row[0].has_point(m):
				var a: Dictionary = row[1]
				ability_tip.show_custom("aug:%s:%s" % [a.name, a.get("progress", "")], func(t: RichTextLabel): _augment_tooltip(t, a), row[0], overlay.size)
				return
	if slot < 0:
		ability_tip.hide()
		return
	var level_key: String = settings.primary_binding(LEVEL_ACTIONS[slot]) if slot < 4 else ""
	ability_tip.show_for(client.ability_info(slot), settings.primary_binding(SLOT_ACTIONS[slot]), level_key, _ability_boxes[slot], overlay.size)


## Item stat colors, the way the genre colors them.
const STAT_COLORS := {
	"ad": "ff9a3c", "ap": "8f9cff", "hp": "5fe08a", "armor": "e8b04f", "mr": "6fb6ff",
	"as": "ffd24a", "haste": "d6dbe2", "ms": "d6dbe2", "lifesteal": "ff6f7a", "passive": "c9a24a",
	"active": "7ee07e",
}


## An item's tooltip: icon and name, its cost, its stats colored by stat, its passive and active.
func _item_tooltip(t: RichTextLabel, it: Dictionary, charges: int, key: String) -> void:
	t.add_image(ItemIcons.icon(it), 36, 36)
	var tier_color: String = ["d6dbe2", "8fd8c8", "f2d27a"][clampi(int(it.tier), 0, 2)]
	t.append_text("  [font_size=19][b][color=#%s]%s[/color][/b][/font_size]" % [tier_color, it.name])
	t.append_text("\n[color=#ffd56b]%d gold[/color]" % it.cost)
	if it.get("consumable", false) and key != "":
		t.append_text("[color=#8a93a0]   ·   %d charge%s   ·   [%s][/color]" % [charges, "" if charges == 1 else "s", key])
	t.append_text("\n")
	for line in it.get("lines", []):
		var c: String = STAT_COLORS.get(line.kind, "d6dbe2")
		t.append_text("\n[color=#%s]%s[/color]" % [c, line.text])


## An augment's tooltip: its name in its tier's color, the tier, what it does, its progress.
func _augment_tooltip(t: RichTextLabel, a: Dictionary) -> void:
	var c: String = _tier_color(a.tier).to_html(false)
	t.add_image(AugmentIcons.icon(a), 36, 36)
	t.append_text("  [font_size=19][b][color=#%s]%s[/color][/b][/font_size]" % [c, a.name])
	t.append_text("\n[color=#8a93a0]%s augment[/color]\n\n%s" % [a.tier, a.get("text", "")])
	if a.has("progress"):
		t.append_text("\n\n[color=#7ee07e]Progress: %s[/color]" % a.progress)


## ---- The match breakdown (hold Tab) ----------------------------------------------------------
## Laid out like the reference game's: our team on the left, theirs on the right, mirrored, the
## score and turrets destroyed in the middle on top. Each row: the augments (outer edge), the D
## and F spells, the portrait with the level and, while dead, the respawn timer, minions
## killed, K/D/A, the items. Hovering an item or an augment shows its tooltip, the enemy's too.
## Movable like the other windows.

var score_panel: PanelContainer
var _score_sig := ""
var _score_refresh := 0.0
var _score_timers := {}                    # unit -> its respawn Label
var _tab_hover := []                       # [kind, data, Control] under the mouse in the breakdown
var _tab_hoverables := []                  # every hoverable in it (scripted captures pick one)


func _update_scoreboard(delta: float, playing: bool) -> void:
	var show := playing and (Input.is_action_pressed("scoreboard") or _shot_tab) and (menu_panel == null)
	if not show:
		if score_panel != null:
			score_panel.visible = false
		_tab_hover = []
		return
	if score_panel == null:
		score_panel = PanelContainer.new()
		var margin := MarginContainer.new()
		for side in ["left", "right", "top", "bottom"]:
			margin.add_theme_constant_override("margin_" + side, 12)
		score_panel.add_child(margin)
		var box := VBoxContainer.new()
		box.add_theme_constant_override("separation", 8)
		margin.add_child(box)
		score_panel.set_meta("box", box)
		overlay.get_parent().add_child(score_panel)
		Windows.make_movable(score_panel, "scoreboard", settings)
	if not score_panel.visible:
		score_panel.move_to_front()  # held open, it's on top of the other windows
	score_panel.visible = true
	_score_refresh -= delta
	if _score_refresh <= 0.0:
		_score_refresh = 0.25
		_fill_scoreboard()
	Windows.place(score_panel, "scoreboard", settings, Vector2((overlay.size.x - score_panel.size.x) / 2.0, overlay.size.y * 0.2), overlay.size)


func _fill_scoreboard() -> void:
	var rows: Array = client.scoreboard()
	var box: VBoxContainer = score_panel.get_meta("box")
	portraits.prepare(rows.map(func(r): return r.champion))
	var towers: PackedInt32Array = client.towers()
	# Rebuilt only when something but the timers changed (a tooltip under the mouse stays).
	var sig := str(rows.map(func(r): return [r.kills, r.deaths, r.assists, r.cs, r.level, r.items, r.augments.size(), r.respawn > 0.0, r.spell_f])) + str(_portraits_seen) + str(towers)
	if sig != _score_sig:
		_score_sig = sig
		_score_timers.clear()
		_tab_hover = []
		_tab_hoverables = []
		for c in box.get_children():
			c.queue_free()
		var kills := [0, 0]
		for r in rows:
			kills[0 if r.ally else 1] += int(r.kills)
		# The middle: turrets, kills against kills, turrets.
		var head := HBoxContainer.new()
		head.alignment = BoxContainer.ALIGNMENT_CENTER
		head.add_theme_constant_override("separation", 26)
		var ally_c := OWN_COLOR.lightened(0.25)
		var enemy_c := ENEMY_COLOR.lightened(0.1)
		head.add_child(_tab_label("♜ %d" % (towers[0] if towers.size() > 0 else 0), 18, ally_c))
		head.add_child(_tab_label("%d" % kills[0], 28, ally_c))
		head.add_child(_tab_label("⚔", 22, Color(0.85, 0.75, 0.5)))
		head.add_child(_tab_label("%d" % kills[1], 28, enemy_c))
		head.add_child(_tab_label("♜ %d" % (towers[1] if towers.size() > 1 else 0), 18, enemy_c))
		box.add_child(head)
		var cols := HBoxContainer.new()
		cols.add_theme_constant_override("separation", 6)
		box.add_child(cols)
		for ally in [true, false]:
			var col := VBoxContainer.new()
			col.add_theme_constant_override("separation", 4)
			for r in rows:
				if r.ally == ally:
					col.add_child(_score_row(r, not ally))
			cols.add_child(col)
	if _shot_tab_hover != "" and _tab_hover.is_empty():
		# `--tab-hover augment|item`: the last one of that kind (the enemy's side is built last).
		for hv in _tab_hoverables:
			if hv[0] == _shot_tab_hover:
				_tab_hover = hv
	for r in rows:
		var l: Label = _score_timers.get(r.unit)
		if l != null and is_instance_valid(l):
			l.text = "%d" % ceili(r.respawn) if r.respawn > 0.0 else ""


func _tab_label(text: String, size: int, color: Color) -> Label:
	var l := Label.new()
	l.text = text
	l.add_theme_font_override("font", _bold_font())
	l.add_theme_font_size_override("font_size", size)
	l.add_theme_color_override("font_color", color)
	l.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	return l


## One row. `mirrored` (the enemy's side) runs right to left: items, score, portrait, spells,
## augments on the outer edge.
func _score_row(r: Dictionary, mirrored: bool) -> Control:
	var dead: bool = r.respawn > 0.0
	var row := PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.32, 0.06, 0.06, 0.85) if dead else (Color(0.13, 0.11, 0.06, 0.9) if r.you else Color(0.05, 0.065, 0.085, 0.85))
	sb.border_color = Color(0.78, 0.65, 0.38) if r.you else Color(0.14, 0.16, 0.2)
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(4)
	sb.set_content_margin_all(5)
	row.add_theme_stylebox_override("panel", sb)
	var h := HBoxContainer.new()
	h.add_theme_constant_override("separation", 10)
	row.add_child(h)
	var parts: Array[Control] = []

	# Augments: a 2 × 2 grid.
	var augs := GridContainer.new()
	augs.columns = 2
	augs.add_theme_constant_override("h_separation", 3)
	augs.add_theme_constant_override("v_separation", 3)
	for i in 4:
		var cell := TextureRect.new()
		cell.custom_minimum_size = Vector2(24, 24)
		cell.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		if i < r.augments.size():
			var a: Dictionary = r.augments[i]
			cell.texture = AugmentIcons.icon(a)
			_tab_hoverable(cell, "augment", a)
		else:
			cell.texture = _empty_slot()
		augs.add_child(cell)
	parts.append(augs)

	# D and F.
	var spells := VBoxContainer.new()
	spells.add_theme_constant_override("separation", 2)
	var tint: Color = CHAMPION_COLORS.get(r.champion, Color(0.5, 0.5, 0.55))
	for kind in ["blink", r.spell_f]:
		var sp := TextureRect.new()
		sp.custom_minimum_size = Vector2(22, 22)
		sp.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		sp.texture = Hud.ability_icon(kind, Color(0.36, 0.42, 0.5))
		spells.add_child(sp)
	parts.append(spells)

	# The portrait: level in the corner, the respawn timer over it while dead.
	var face := Control.new()
	face.custom_minimum_size = Vector2(48, 48)
	var pic := TextureRect.new()
	pic.texture = portraits.portrait(r.champion, "round")
	pic.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	pic.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	pic.modulate = Color(0.4, 0.4, 0.45) if dead else Color.WHITE
	face.add_child(pic)
	face.tooltip_text = "%s%s" % [r.champion, "  (you)" if r.you else ("  ·  bot" if r.bot else "")]
	var level := Label.new()
	level.text = "%d" % r.level
	level.add_theme_font_override("font", _bold_font())
	level.add_theme_font_size_override("font_size", 12)
	var lsb := StyleBoxFlat.new()
	lsb.bg_color = Color(0.04, 0.05, 0.07)
	lsb.set_corner_radius_all(8)
	lsb.set_content_margin_all(2)
	level.add_theme_stylebox_override("normal", lsb)
	level.position = Vector2(30, 30)
	face.add_child(level)
	var timer := Label.new()
	timer.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	timer.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	timer.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	timer.add_theme_font_override("font", _bold_font())
	timer.add_theme_font_size_override("font_size", 20)
	timer.add_theme_color_override("font_color", Color(1.0, 0.3, 0.25))
	timer.add_theme_color_override("font_outline_color", Color.BLACK)
	timer.add_theme_constant_override("outline_size", 4)
	face.add_child(timer)
	_score_timers[r.unit] = timer
	parts.append(face)

	# Minions killed, then K/D/A.
	var cs := _tab_label("%d" % r.cs, 16, Color(1.0, 0.86, 0.5))
	cs.custom_minimum_size = Vector2(34, 0)
	cs.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	cs.tooltip_text = "Minions killed"
	cs.mouse_filter = Control.MOUSE_FILTER_PASS
	parts.append(cs)
	var kda := _tab_label("%d / %d / %d" % [r.kills, r.deaths, r.assists], 16, Color(0.92, 0.94, 0.96))
	kda.custom_minimum_size = Vector2(86, 0)
	kda.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	parts.append(kda)

	# Items.
	var items := HBoxContainer.new()
	items.add_theme_constant_override("separation", 2)
	for id in r.items:
		var cell := TextureRect.new()
		cell.custom_minimum_size = Vector2(30, 30)
		cell.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		if id != 0 and _catalog.has(id):
			cell.texture = ItemIcons.icon(_catalog[id])
			_tab_hoverable(cell, "item", _catalog[id])
		else:
			cell.texture = _empty_slot()
		items.add_child(cell)
	parts.append(items)

	if mirrored:
		parts.reverse()
	for c in parts:
		c.size_flags_vertical = Control.SIZE_SHRINK_CENTER
		h.add_child(c)
	return row


## Hovering this shows its tooltip (an item's or an augment's), from the breakdown too.
func _tab_hoverable(c: Control, kind: String, data: Dictionary) -> void:
	_tab_hoverables.append([kind, data, c])
	c.mouse_filter = Control.MOUSE_FILTER_PASS
	c.mouse_entered.connect(func(): _tab_hover = [kind, data, c])
	c.mouse_exited.connect(func():
		if not _tab_hover.is_empty() and _tab_hover[2] == c:
			_tab_hover = [])


static var _empty: ImageTexture


static func _empty_slot() -> Texture2D:
	if _empty == null:
		var img := Image.create(8, 8, false, Image.FORMAT_RGBA8)
		img.fill(Color(0.07, 0.08, 0.1))
		_empty = ImageTexture.create_from_image(img)
	return _empty


## ---- The death recap ---------------------------------------------------------------------------
## While dead: exactly what killed us. Every hit of the fight, from the server's own damage
## events (recap.rs): the total and how long it took, by damage type, by source, and by attack,
## ability or effect, with the crowd control we were under. Movable.

var recap_panel: PanelContainer
var _recap_shown := ""
const KIND_COLORS := {"physical": "ff9a3c", "magic": "6fb6ff", "true": "ffffff"}


func _update_recap(playing: bool) -> void:
	var dead: bool = playing and own_status.get("dead", false)
	var r: Dictionary = client.death_recap() if dead else {}
	if r.is_empty():
		if recap_panel != null:
			recap_panel.queue_free()
			recap_panel = null
		_recap_shown = ""
		return
	if recap_panel == null:
		var made := _panel(440)
		recap_panel = made[0]
		recap_panel.set_meta("box", made[1])
		Windows.make_movable(recap_panel, "recap", settings)
	var key := "%s|%s|%s" % [r.killer, r.total, r.seconds]
	if key != _recap_shown:
		_recap_shown = key
		_fill_recap(recap_panel.get_meta("box"), r)
	var default := Vector2(overlay.size.x - recap_panel.size.x - 20.0, (overlay.size.y - recap_panel.size.y) / 2.0)
	Windows.place(recap_panel, "recap", settings, default, overlay.size)


func _fill_recap(box: VBoxContainer, r: Dictionary) -> void:
	for c in box.get_children():
		c.queue_free()
	var title := Label.new()
	title.text = "Killed by %s" % r.killer
	title.theme_type_variation = "TitleLabel"
	box.add_child(title)
	var sum := RichTextLabel.new()
	sum.bbcode_enabled = true
	sum.fit_content = true
	sum.scroll_active = false
	sum.custom_minimum_size = Vector2(400, 0)
	sum.append_text("[b]%d[/b] damage in [b]%.1f s[/b]" % [roundi(r.total), r.seconds])
	if r.absorbed > 0.0:
		sum.append_text("[color=#8a93a0]   (%d of it on shields)[/color]" % roundi(r.absorbed))
	box.add_child(sum)
	# Physical, magic and true, as one bar.
	var bar := Control.new()
	bar.custom_minimum_size = Vector2(400, 14)
	var parts := [[r.physical, Color("ff9a3c")], [r.magic, Color("6fb6ff")], [r["true"], Color("f2f4f7")]]
	var total: float = maxf(r.total, 1.0)
	bar.draw.connect(func():
		var x := 0.0
		for p in parts:
			var w: float = bar.size.x * p[0] / total
			bar.draw_rect(Rect2(x, 0, w, bar.size.y), p[1])
			x += w)
	box.add_child(bar)
	var legend := RichTextLabel.new()
	legend.bbcode_enabled = true
	legend.fit_content = true
	legend.scroll_active = false
	legend.append_text("[color=#ff9a3c]%d physical[/color]   [color=#6fb6ff]%d magic[/color]   [color=#f2f4f7]%d true[/color]" % [roundi(r.physical), roundi(r.magic), roundi(r["true"])])
	box.add_child(legend)
	var cc := []
	for pair in [["stunned", "Stunned"], ["rooted", "Rooted"], ["slowed", "Slowed"]]:
		if r[pair[0]] > 0.05:
			cc.append("%s %.1f s" % [pair[1], r[pair[0]]])
	if not cc.is_empty():
		var ccl := Label.new()
		ccl.text = "   ·   ".join(cc)
		ccl.add_theme_color_override("font_color", Color(1.0, 0.82, 0.29))
		box.add_child(ccl)
	box.add_child(HSeparator.new())
	# The sources scroll when there are more than fit (a long fight, many enemies); the
	# summary above stays in view.
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	var list := VBoxContainer.new()
	list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	scroll.add_child(list)
	box.add_child(scroll)
	var rows := 0
	for src in r.sources:
		rows += 1 + src.lines.size()
	scroll.custom_minimum_size = Vector2(410, minf(rows * 30.0 + 8.0, overlay.size.y * 0.45))
	for src in r.sources:
		var head := HBoxContainer.new()
		head.add_theme_constant_override("separation", 8)
		var face := TextureRect.new()
		face.custom_minimum_size = Vector2(30, 30)
		face.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		face.texture = portraits.portrait(src.champion, "round") if src.champion != "" else null
		head.add_child(face)
		var name := Label.new()
		name.text = src.name + ("   (killing blow)" if src.killer else "")
		name.add_theme_font_override("font", _bold_font())
		name.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		head.add_child(name)
		var amount := Label.new()
		amount.text = "%d" % roundi(src.total)
		amount.add_theme_font_override("font", _bold_font())
		head.add_child(amount)
		list.add_child(head)
		for line in src.lines:
			var l := RichTextLabel.new()
			l.bbcode_enabled = true
			l.fit_content = true
			l.scroll_active = false

			var key: String = ("[%s] " % line.key) if line.key != "" else ""
			var c: String = KIND_COLORS.get(line.kind, "d6dbe2")
			l.append_text("      %s%s   [color=#%s]%d %s[/color]   [color=#8a93a0]%d hit%s[/color]" % [key, line.what, c, roundi(line.total), line.kind, line.hits, "" if line.hits == 1 else "s"])
			list.add_child(l)


## ---- The cursor --------------------------------------------------------------------------------

func _update_cursor(playing: bool) -> void:
	var state := "default"
	if playing and get_viewport().gui_get_hovered_control() == null:
		if attack_move_armed:
			state = "attack"
		elif hovered_body != null:
			state = "enemy"
		else:
			var p = _cursor_ground()
			if p != null:
				for id in remote_info:
					var u: Dictionary = remote_info[id]
					if u.ally and u.champion != "" and u.pos.distance_to(p) < 90.0:
						state = "ally"
						break
	GameCursor.set_state(state)


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
	net_label.text = "FPS %d   RTT %.0f ms   margin %.1f ms   interp %.0f ms\ncommands %d   late %d   corrections %d (last %.1f u)   on-screen correction %.1f u\nup %.1f KB   down %.1f KB   collision proxies %s\nenemy missiles %d   near-misses %d   ghost hits %d   phantom hits %d   K/D %d/%d\n%s" % [
		Engine.get_frames_per_second(), s.rtt_ms, s.margin_ms, s.interp_ms,
		s.commands, s.late, s.corrections, s.last_correction, s.visible_correction,
		s.kb_up, s.kb_down, "ON" if proxies_enabled else "OFF",
		s.enemy_missiles, s.near_misses, s.ghost_hits, s.phantom_hits, s.kills, s.deaths, _key_hints(),
	]


## The controls line under the net graph, from the current bindings.
func _key_hints() -> String:
	var b := func(a: String) -> String: return settings.primary_binding(a).replace("Mouse ", "M-")
	return "[%s] move / attack  [%s] attack-move  [%s %s %s %s] abilities  [%s] Blink  [%s] Barrier  [%s] stop  [%s] shop  [%s] camera lock  [Esc] settings" % [
		b.call("move"), b.call("attack_move"), b.call("cast_q"), b.call("cast_w"), b.call("cast_e"), b.call("cast_r"),
		b.call("cast_d"), b.call("cast_f"), b.call("stop"), b.call("toggle_shop"), b.call("camera_lock"),
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
## Opens with G anywhere; buying and selling only work while dead or in the own fountain (the
## sim decides, the panel just greys things out). Laid out like the reference game's: the items
## by tier on the left (click to look, right-click or double-click to buy); on the right the
## chosen item, its build path (the tree of components, the ones we own marked) and what it
## builds into, with Buy at our price (owned components are used and discounted). Click an
## inventory slot to sell it for 70%. Undo works until you leave.

const ItemIcons := preload("res://scripts/item_icons.gd")
const SHOP_NODE := 52.0                  # a build-path node, px
const SHOP_GAP := Vector2(10, 26)        # between build-path nodes

var shop_panel: PanelContainer
var shop_title: Label
var shop_anvil: Button
var shop_spell: Button
var shop_inventory: HBoxContainer
var shop_stats: Label
var shop_undo: Button
var shop_buttons := {}                  # item id -> Button
var shop_slot_buttons := []
var item_names := {}                    # item id -> name
var _shop_refresh := 0.0
var _catalog := {}                      # item id -> catalog entry (latest)
var _shop_pick := 0                     # the item shown on the right
var _shop_detail_sig := ""
var _shop_detail: VBoxContainer
var _shop_tree: Control
var _shop_tree_nodes := []              # [{ id, pos, parent, owned }]
var _shop_buy: Button


func _toggle_shop() -> void:
	if shop_panel == null:
		_build_shop()
	shop_panel.visible = not shop_panel.visible
	_shop_refresh = 0.0
	if shop_panel.visible:
		Windows.place(shop_panel, "shop", settings, Vector2(16, 64), overlay.size)


func _build_shop() -> void:
	ItemIcons.art_root = _art_root()
	shop_panel = PanelContainer.new()
	shop_panel.set_anchors_and_offsets_preset(Control.PRESET_TOP_LEFT)
	shop_panel.position = Vector2(16, 64)
	shop_panel.custom_minimum_size = Vector2(1000, 0)
	var margin := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 14)
	shop_panel.add_child(margin)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 8)
	margin.add_child(v)
	var top := HBoxContainer.new()
	v.add_child(top)
	shop_title = Label.new()
	shop_title.theme_type_variation = "TitleLabel"
	shop_title.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(shop_title)
	# Mayhem: a Stat Anvil, for late gold.
	shop_anvil = Button.new()
	shop_anvil.focus_mode = Control.FOCUS_NONE
	shop_anvil.theme_type_variation = "PrimaryButton"
	shop_anvil.pressed.connect(func(): client.buy_anvil(); _shop_refresh = 0.0)
	top.add_child(shop_anvil)
	# On a map with a jungle: Barrier or Claim in F.
	shop_spell = Button.new()
	shop_spell.focus_mode = Control.FOCUS_NONE
	shop_spell.pressed.connect(func():
		client.choose_spell(0 if own_status.get("spell_f", 0) == 1 else 1)
		_shop_refresh = 0.0)
	top.add_child(shop_spell)
	Windows.make_movable(shop_panel, "shop", settings)
	var body := HBoxContainer.new()
	body.add_theme_constant_override("separation", 14)
	v.add_child(body)

	# The items, by tier.
	var grids := VBoxContainer.new()
	grids.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	grids.add_theme_constant_override("separation", 6)
	body.add_child(grids)
	var tiers := ["CONSUMABLES  ·  use with 1–6", "COMPONENTS", "UPGRADES AND BOOTS", "LEGENDARY"]
	var catalog: Array = client.shop_catalog()
	for it in catalog:
		item_names[it.id] = it.name
		_catalog[it.id] = it
	if _shop_pick == 0 and catalog.size() > 0:
		_shop_pick = catalog[catalog.size() - 1].id
	for group in 4:
		var l := Label.new()
		l.text = tiers[group]
		l.theme_type_variation = "HeaderLabel"
		grids.add_child(l)
		var grid := GridContainer.new()
		grid.columns = 8
		grid.add_theme_constant_override("h_separation", 6)
		grid.add_theme_constant_override("v_separation", 6)
		grids.add_child(grid)
		for it in catalog:
			var consumable: bool = it.get("consumable", false)
			if (group == 0) != consumable or (group > 0 and it.tier != group - 1):
				continue
			var id: int = it.id
			var b := Button.new()
			b.custom_minimum_size = Vector2(66, 84)
			_tight(b)
			b.icon = ItemIcons.icon(it)
			b.expand_icon = true
			b.icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
			b.vertical_icon_alignment = VERTICAL_ALIGNMENT_TOP
			b.add_theme_font_size_override("font_size", 13)
			b.add_theme_color_override("font_color", Color(1.0, 0.84, 0.4))
			b.focus_mode = Control.FOCUS_NONE
			b.pressed.connect(func(): _shop_pick = id; _shop_detail_sig = ""; _shop_refresh = 0.0)
			b.gui_input.connect(func(e): _shop_item_input(e, id))
			grid.add_child(b)
			shop_buttons[id] = b

	# The chosen item: its build path and what it builds into.
	var side := PanelContainer.new()
	side.custom_minimum_size = Vector2(380, 0)
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.07, 0.085, 0.11)
	sb.set_corner_radius_all(5)
	sb.set_content_margin_all(12)
	side.add_theme_stylebox_override("panel", sb)
	body.add_child(side)
	_shop_detail = VBoxContainer.new()
	_shop_detail.add_theme_constant_override("separation", 8)
	side.add_child(_shop_detail)

	# Inventory, undo, our stats.
	var inv_label := Label.new()
	inv_label.text = "INVENTORY  ·  click to sell for 70%"
	inv_label.theme_type_variation = "HeaderLabel"
	v.add_child(inv_label)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	v.add_child(row)
	shop_inventory = HBoxContainer.new()
	shop_inventory.add_theme_constant_override("separation", 6)
	row.add_child(shop_inventory)
	for slot in 6:
		var b := Button.new()
		b.custom_minimum_size = Vector2(54, 54)
		_tight(b)
		b.expand_icon = true
		b.icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
		b.focus_mode = Control.FOCUS_NONE
		b.pressed.connect(func(): client.sell(slot); _shop_refresh = 0.0)
		shop_inventory.add_child(b)
		shop_slot_buttons.append(b)
	var gap := Control.new()
	gap.custom_minimum_size = Vector2(10, 0)
	row.add_child(gap)
	shop_undo = Button.new()
	shop_undo.text = "Undo"
	shop_undo.focus_mode = Control.FOCUS_NONE
	shop_undo.custom_minimum_size = Vector2(90, 0)
	shop_undo.pressed.connect(func(): client.undo_trade(); _shop_refresh = 0.0)
	row.add_child(shop_undo)
	shop_stats = Label.new()
	shop_stats.theme_type_variation = "HintLabel"
	shop_stats.add_theme_font_size_override("font_size", 14)
	shop_stats.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	shop_stats.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	shop_stats.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	row.add_child(shop_stats)
	shop_panel.visible = false
	overlay.get_parent().add_child(shop_panel)


## Right-click or double-click buys; a left click (the button's press) only looks.
func _shop_item_input(e: InputEvent, id: int) -> void:
	if e is InputEventMouseButton and e.is_pressed():
		var mb := e as InputEventMouseButton
		if mb.button_index == MOUSE_BUTTON_RIGHT or (mb.button_index == MOUSE_BUTTON_LEFT and mb.double_click):
			_shop_pick = id
			client.buy(id)
			_shop_detail_sig = ""
			_shop_refresh = 0.0


func _update_shop(delta: float) -> void:
	if shop_panel == null or not shop_panel.visible:
		return
	_shop_refresh -= delta
	if _shop_refresh > 0.0:
		return
	_shop_refresh = 0.15
	var open: bool = client.can_shop()
	var gold: int = own_status.get("gold", 0)
	shop_title.text = "Shop   ·   %d gold%s" % [gold, "" if open else "   (closed: return to your fountain, or shop while dead)"]
	var jungle: bool = own_status.get("jungle", false)
	shop_spell.visible = jungle
	if jungle:
		var claim: bool = own_status.get("spell_f", 0) == 1
		shop_spell.text = "F: %s   ⇄   %s" % ["Claim" if claim else "Barrier", "Barrier" if claim else "Claim"]
		shop_spell.disabled = not open
		shop_spell.tooltip_text = "Swap your F spell. Claim (for junglers) strikes a monster or minion for true damage and heals you on monsters; Barrier shields you. A swap puts it on at least a 15 s cooldown."
	var mayhem: bool = own_status.get("mayhem", false)
	shop_anvil.visible = mayhem
	if mayhem:
		var level: int = own_status.get("level", 1)
		var need: int = own_status.get("anvil_level", 9)
		var cost: float = own_status.get("anvil_cost", 750.0)
		shop_anvil.text = "⚒  Stat Anvil  ·  %d" % roundi(cost)
		shop_anvil.disabled = not open or level < need or gold < cost or not own_status.get("anvil_offer", []).is_empty()
		shop_anvil.tooltip_text = "From level %d: a Silver, Gold or Prismatic anvil offers three stats; keep one for the match.%s" % [need, "" if level >= need else "  (You're level %d.)" % level]
	for it in client.shop_catalog():
		_catalog[it.id] = it
		var b: Button = shop_buttons[it.id]
		b.text = "%d" % it.price
		b.tooltip_text = "%s  ·  %d gold\n%s" % [it.name, it.price, it.stats]
		# Never disabled: any item can be looked at; what we can't buy now is dimmed.
		b.modulate = Color(1, 1, 1) if open and it.affordable else Color(0.55, 0.55, 0.6)
		# The item on the right is framed in gold.
		if it.id == _shop_pick:
			b.add_theme_stylebox_override("normal", _shop_pick_box())
			b.add_theme_stylebox_override("disabled", _shop_pick_box())
		else:
			b.add_theme_stylebox_override("normal", _icon_box("normal"))
			b.add_theme_stylebox_override("disabled", _icon_box("disabled"))
	var inv: Array = own_status.get("items", [])
	for slot in shop_slot_buttons.size():
		var id: int = inv[slot] if slot < inv.size() else 0
		var b: Button = shop_slot_buttons[slot]
		b.icon = ItemIcons.icon(_catalog[id]) if id != 0 and _catalog.has(id) else null
		b.text = "" if id != 0 else "—"
		b.tooltip_text = "Sell %s for 70%%" % item_names.get(id, "?") if id != 0 else ""
		b.disabled = not open or id == 0
	shop_undo.disabled = not open or not own_status.get("can_undo", false)
	if not own_status.is_empty() and own_status.has("attack_damage"):
		shop_stats.text = "AD %d   AP %d   armor %d   MR %d   AS %.2f   MS %d   haste %d   HP %d" % [
			roundi(own_status.attack_damage), roundi(own_status.ability_power), roundi(own_status.armor),
			roundi(own_status.magic_resist), own_status.attack_speed, roundi(own_status.move_speed),
			roundi(own_status.ability_haste), roundi(own_status.max_health),
		]
	_update_shop_detail(open, inv)


static var _pick_box: StyleBoxFlat
static var _icon_boxes := {}


## The theme's button looks with tight margins, so an icon fills its button.
static func _icon_box(state: String) -> StyleBox:
	if not _icon_boxes.has(state):
		var base: StyleBox = ThemeDB.get_default_theme().get_stylebox(state, "Button")
		var box: StyleBox = base.duplicate()
		box.set_content_margin_all(3)
		_icon_boxes[state] = box
	return _icon_boxes[state]


static func _tight(b: Button) -> void:
	for state in ["normal", "hover", "pressed", "disabled", "hover_pressed"]:
		b.add_theme_stylebox_override(state, _icon_box(state))


static func _shop_pick_box() -> StyleBoxFlat:
	if _pick_box == null:
		_pick_box = StyleBoxFlat.new()
		_pick_box.bg_color = Color(0.2, 0.16, 0.08)
		_pick_box.border_color = Color(0.95, 0.8, 0.45)
		_pick_box.set_border_width_all(2)
		_pick_box.set_corner_radius_all(4)
		_pick_box.set_content_margin_all(3)
	return _pick_box


## The right side: the chosen item, its build path and what it builds into. Rebuilt when the
## choice, the inventory or what we can afford changes.
func _update_shop_detail(open: bool, inv: Array) -> void:
	var it: Dictionary = _catalog.get(_shop_pick, {})
	if it.is_empty():
		return
	var sig := "%d|%s|%s|%s|%d" % [_shop_pick, inv, open, it.affordable, it.price]
	if sig == _shop_detail_sig:
		return
	_shop_detail_sig = sig
	for c in _shop_detail.get_children():
		c.queue_free()

	var head := HBoxContainer.new()
	head.add_theme_constant_override("separation", 12)
	var big := TextureRect.new()
	big.texture = ItemIcons.icon(it)
	big.custom_minimum_size = Vector2(64, 64)
	big.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	head.add_child(big)
	var names := VBoxContainer.new()
	names.alignment = BoxContainer.ALIGNMENT_CENTER
	var name := Label.new()
	name.text = it.name
	name.add_theme_font_override("font", _bold_font())
	name.add_theme_font_size_override("font_size", 20)
	name.add_theme_color_override("font_color", Color(0.95, 0.88, 0.7))
	names.add_child(name)
	var cost := Label.new()
	cost.text = "%d gold" % it.cost if it.price == it.cost else "%d gold  ·  %d for you" % [it.cost, it.price]
	cost.add_theme_color_override("font_color", Color(1.0, 0.84, 0.4))
	names.add_child(cost)
	head.add_child(names)
	_shop_detail.add_child(head)
	# Its stats, colored by stat like the tooltips.
	var stats := RichTextLabel.new()
	stats.bbcode_enabled = true
	stats.fit_content = true
	stats.scroll_active = false
	stats.custom_minimum_size = Vector2(356, 0)
	var lines := []
	for line in it.get("lines", []):
		lines.append("[color=#%s]%s[/color]" % [STAT_COLORS.get(line.kind, "d6dbe2"), line.text])
	stats.append_text("\n".join(lines))
	_shop_detail.add_child(stats)

	_shop_detail.add_child(HSeparator.new())
	var path := Label.new()
	path.text = "BUILD PATH" if not it.recipe.is_empty() else ("BUILD PATH  ·  a consumable" if it.get("consumable", false) else "BUILD PATH  ·  a basic component")
	path.theme_type_variation = "HeaderLabel"
	_shop_detail.add_child(path)
	_shop_tree = Control.new()
	_shop_tree.draw.connect(_draw_shop_tree)
	_shop_detail.add_child(_shop_tree)
	_layout_shop_tree(_shop_pick, inv)

	var into := []
	for other in _catalog.values():
		if other.recipe.has(_shop_pick):
			into.append(other)
	if not into.is_empty():
		var into_label := Label.new()
		into_label.text = "BUILDS INTO"
		into_label.theme_type_variation = "HeaderLabel"
		_shop_detail.add_child(into_label)
		var row := HFlowContainer.new()
		row.add_theme_constant_override("h_separation", 6)
		row.add_theme_constant_override("v_separation", 6)
		for other in into:
			row.add_child(_shop_node_button(other, false))
		_shop_detail.add_child(row)

	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	_shop_detail.add_child(spacer)
	_shop_buy = Button.new()
	_shop_buy.text = "Buy  ·  %d gold" % it.price
	_shop_buy.theme_type_variation = "PrimaryButton"
	_shop_buy.custom_minimum_size = Vector2(0, 44)
	_shop_buy.focus_mode = Control.FOCUS_NONE
	_shop_buy.disabled = not open or not it.affordable
	var id: int = _shop_pick
	_shop_buy.pressed.connect(func(): client.buy(id); _shop_detail_sig = ""; _shop_refresh = 0.0)
	_shop_detail.add_child(_shop_buy)


## A small item button for the build path and "builds into": click to look at that item.
func _shop_node_button(it: Dictionary, owned: bool) -> Button:
	var b := Button.new()
	b.custom_minimum_size = Vector2(SHOP_NODE, SHOP_NODE)
	_tight(b)
	b.icon = ItemIcons.icon(it)
	b.expand_icon = true
	b.icon_alignment = HORIZONTAL_ALIGNMENT_CENTER
	b.focus_mode = Control.FOCUS_NONE
	b.tooltip_text = "%s  ·  %d gold%s\n%s" % [it.name, it.cost, "  (owned)" if owned else "", it.stats]
	var id: int = it.id
	b.pressed.connect(func(): _shop_pick = id; _shop_detail_sig = ""; _shop_refresh = 0.0)
	b.gui_input.connect(func(e): _shop_item_input(e, id))
	return b


## The build path as a tree: the item on top, its components under it, theirs under them. The
## components we own (each owned item used once, the way buying uses them) are marked, and
## their own components aren't needed, so they're dimmed.
func _layout_shop_tree(root: int, inv: Array) -> void:
	_shop_tree_nodes.clear()
	var have := {}
	for id in inv:
		if id != 0:
			have[id] = have.get(id, 0) + 1
	var depth := [0]
	var place := func(this: Callable, id: int, level: int, left: float, parent: int, covered: bool) -> float:
		depth[0] = maxi(depth[0], level)
		var owned := false
		if level > 0 and not covered and have.get(id, 0) > 0:
			have[id] -= 1
			owned = true
		var index := _shop_tree_nodes.size()
		_shop_tree_nodes.append({"id": id, "pos": Vector2.ZERO, "parent": parent, "owned": owned, "covered": covered})
		var kids: Array = _catalog.get(id, {}).get("recipe", [])
		var width := 0.0
		for k in kids:
			if width > 0.0:
				width += SHOP_GAP.x
			width += this.call(this, int(k), level + 1, left + width, index, covered or owned)
		width = maxf(width, SHOP_NODE)
		_shop_tree_nodes[index].pos = Vector2(left + width / 2.0 - SHOP_NODE / 2.0, level * (SHOP_NODE + SHOP_GAP.y))
		return width
	var total: float = place.call(place, root, 0, 0.0, -1, false)
	var area := 356.0
	var shift := maxf(0.0, (area - total) / 2.0)
	_shop_tree.custom_minimum_size = Vector2(area, (depth[0] + 1) * (SHOP_NODE + SHOP_GAP.y) - SHOP_GAP.y)
	for n in _shop_tree_nodes:
		n.pos.x += shift
		var it: Dictionary = _catalog.get(n.id, {})
		if it.is_empty():
			continue
		var b := _shop_node_button(it, n.owned)
		b.position = n.pos
		b.size = Vector2(SHOP_NODE, SHOP_NODE)
		if n.covered:
			b.modulate = Color(1, 1, 1, 0.35)
		_shop_tree.add_child(b)
	_shop_tree.queue_redraw()


func _draw_shop_tree() -> void:
	var half := SHOP_NODE / 2.0
	for n in _shop_tree_nodes:
		if n.parent < 0:
			continue
		var p: Dictionary = _shop_tree_nodes[n.parent]
		var a: Vector2 = p.pos + Vector2(half, SHOP_NODE)
		var c: Vector2 = n.pos + Vector2(half, 0)
		var mid := a.y + SHOP_GAP.y / 2.0
		var col := Color(0.78, 0.65, 0.38, 0.35 if n.covered else 0.85)
		_shop_tree.draw_polyline(PackedVector2Array([a, Vector2(a.x, mid), Vector2(c.x, mid), c]), col, 2.0)
	for n in _shop_tree_nodes:
		if n.owned:
			# Owned: a green frame and a check in the corner.
			var r := Rect2(n.pos, Vector2(SHOP_NODE, SHOP_NODE)).grow(2.0)
			_shop_tree.draw_rect(r, Color(0.45, 0.85, 0.5), false, 2.0)
			var tip: Vector2 = n.pos + Vector2(SHOP_NODE - 2.0, 2.0)
			_shop_tree.draw_colored_polygon(PackedVector2Array([tip + Vector2(-14, 0), tip, tip + Vector2(0, 14)]), Color(0.45, 0.85, 0.5))


func _draw_inventory(font: Font, origin: Vector2, k: float) -> void:
	if item_names.is_empty():
		ItemIcons.art_root = _art_root()
		for it in client.shop_catalog():
			item_names[it.id] = it.name
			_catalog[it.id] = it
	var inv: Array = own_status.items
	var charges: Array = own_status.get("charges", [])
	var I := Vector2(43.0, 43.0) * k
	var gap := 5.0 * k
	_item_boxes.clear()
	for slot in 6:
		var p := origin + Vector2((slot % 3) * (I.x + gap), (slot / 3) * (I.y + gap))
		var box := Rect2(p, I)
		var id: int = inv[slot]
		overlay.draw_rect(box, Color(0.075, 0.085, 0.105))
		if id != 0 and _catalog.has(id):
			# The item's icon, square, centered in the slot.
			var side := minf(box.size.x, box.size.y)
			overlay.draw_texture_rect(ItemIcons.icon(_catalog[id]), Rect2(box.get_center() - Vector2(side, side) / 2.0, Vector2(side, side)), false)
		elif id != 0:
			overlay.draw_multiline_string(font, p + Vector2(4.0 * k, 15.0 * k), item_names.get(id, "?"), HORIZONTAL_ALIGNMENT_CENTER, I.x - 8.0 * k, roundi(11.0 * k), 2, Hud.TEXT)
		overlay.draw_rect(box, Hud.GOLD if id != 0 else Color(0.25, 0.27, 0.31), false, 1.0 * k)
		_item_boxes.append(box)
		# Its key in the corner, and a stack's or a flask's charges.
		var key: String = settings.primary_binding("item_%d" % (slot + 1))
		overlay.draw_string(font, box.position + Vector2(3.0 * k, box.size.y - 3.0 * k), key, HORIZONTAL_ALIGNMENT_LEFT, -1, roundi(10.0 * k), Color(0.85, 0.87, 0.9, 0.8))
		var n: int = charges[slot] if slot < charges.size() else 0
		if id != 0 and _catalog.get(id, {}).get("consumable", false):
			Hud.text(overlay, _bold_font(), Vector2(box.position.x, box.end.y - 3.0 * k), "%d" % n, roundi(14.0 * k), Color.WHITE if n > 0 else Color(1, 0.4, 0.35), HORIZONTAL_ALIGNMENT_RIGHT, box.size.x - 3.0 * k)
	# Gold under the items.
	var gy := origin.y + 2.0 * (I.y + gap) + 2.0 * k
	var ic := 16.0 * k
	overlay.draw_texture_rect(Hud.stat_icon("gold"), Rect2(Vector2(origin.x, gy), Vector2(ic, ic)), false)
	Hud.text(overlay, _bold_font(), Vector2(origin.x + ic + 6.0 * k, gy + 13.0 * k), "%d" % own_status.gold, roundi(15.0 * k), Color(1.0, 0.84, 0.4))
	# ARAM: Mayhem: the augments held, above the panel.
	var held: Array = own_status.get("augments", [])
	_augment_rows.clear()
	var side := 30.0 * k
	for i in held.size():
		var a: Dictionary = held[i]
		var r := Rect2(Vector2(origin.x + i * (side + 4.0 * k), _bar_rect.position.y - side - 6.0 * k), Vector2(side, side))
		overlay.draw_texture_rect(AugmentIcons.icon(a), r, false)
		if a.has("progress"):
			Hud.text(overlay, font, Vector2(r.position.x, r.end.y - 2.0 * k), a.progress, roundi(10.0 * k), Color.WHITE, HORIZONTAL_ALIGNMENT_RIGHT, r.size.x - 2.0 * k)
		_augment_rows.append([r, a])


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


var lobby_panel: Control
var lobby_box: VBoxContainer
var _lobby_refresh := 0.0
var _lobby_ready := false
var _lobby_sig := ""                      # the lobby as last drawn (rebuilt when it changes)
var _lobby_timer: Label
var _last_lobby := {}                     # the lobby as last seen, for the loading screen
var _portraits_seen := 0                  # portraits rendered so far (rebuilds the lobby)


func _update_lobby(delta: float, phase: String) -> void:
	if phase != "lobby":
		if lobby_panel != null:
			lobby_panel.queue_free()
			lobby_panel = null
			_lobby_sig = ""
			# Into the match: the loading screen reveals both teams.
			if phase in ["joining", "playing"] and not _last_lobby.is_empty():
				_show_loading(_last_lobby)
		_last_lobby = {}
		return
	if lobby_panel == null:
		lobby_panel = Control.new()
		lobby_panel.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		var margin := MarginContainer.new()
		margin.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		for side in ["left", "right", "top", "bottom"]:
			margin.add_theme_constant_override("margin_" + side, 28)
		lobby_panel.add_child(margin)
		lobby_box = VBoxContainer.new()
		lobby_box.add_theme_constant_override("separation", 14)
		margin.add_child(lobby_box)
		overlay.get_parent().add_child(lobby_panel)
	_lobby_refresh -= delta
	if _lobby_refresh > 0.0:
		return
	_lobby_refresh = 0.2
	var l: Dictionary = client.lobby_state()
	if l.is_empty():
		return
	_last_lobby = l
	portraits.prepare(l.slots.map(func(x): return x.champion) + Array(l.bench))
	if _lobby_timer != null and is_instance_valid(_lobby_timer):
		_lobby_timer.text = "%d" % ceili(l.starts_in)
	# Rebuild only when something but the countdown changed, so buttons keep their hover and
	# a click isn't lost to a rebuild.
	var sig := str(l.slots) + str(l.bench) + str(_portraits_seen)
	if sig == _lobby_sig:
		return
	_lobby_sig = sig
	for c in lobby_box.get_children():
		c.queue_free()
	var me := {}
	for slot in l.slots:
		if slot.you:
			me = slot

	# Top: the mode on the left, the countdown in the middle.
	var top := HBoxContainer.new()
	var titles := VBoxContainer.new()
	titles.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	titles.size_flags_stretch_ratio = 1.0
	var title := Label.new()
	title.text = "Champion select"
	title.theme_type_variation = "TitleLabel"
	titles.add_child(title)
	var sub := Label.new()
	sub.text = "%s  ·  ALL RANDOM  ·  The Bridge" % client.game_type().to_upper()
	sub.theme_type_variation = "HeaderLabel"
	titles.add_child(sub)
	top.add_child(titles)
	var clock := VBoxContainer.new()
	clock.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	clock.alignment = BoxContainer.ALIGNMENT_CENTER
	_lobby_timer = Label.new()
	_lobby_timer.text = "%d" % ceili(l.starts_in)
	_lobby_timer.theme_type_variation = "TitleLabel"
	_lobby_timer.add_theme_font_size_override("font_size", 46)
	_lobby_timer.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	clock.add_child(_lobby_timer)
	var until := Label.new()
	until.text = "THE MATCH STARTS WHEN EVERYONE IS READY"
	until.theme_type_variation = "HintLabel"
	until.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	clock.add_child(until)
	top.add_child(clock)
	var right := Control.new()
	right.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(right)
	lobby_box.add_child(top)

	# The bench, across the top: champions anyone on the team can swap theirs for.
	var bench := HBoxContainer.new()
	bench.alignment = BoxContainer.ALIGNMENT_CENTER
	bench.add_theme_constant_override("separation", 10)
	var bench_label := Label.new()
	bench_label.text = "BENCH" if l.bench.size() > 0 else "BENCH  ·  rerolled champions land here for the team to take"
	bench_label.theme_type_variation = "HeaderLabel"
	bench.add_child(bench_label)
	for name in l.bench:
		var b := Button.new()
		b.custom_minimum_size = Vector2(64, 64)
		b.focus_mode = Control.FOCUS_NONE
		b.tooltip_text = "Swap your champion for %s" % name
		b.icon = portraits.portrait(name, "bust")
		b.expand_icon = true
		b.text = "" if b.icon != null else String(name).left(2)
		b.pressed.connect(func(): client.lobby_take(name); _lobby_refresh = 0.0)
		bench.add_child(b)
	lobby_box.add_child(bench)

	# The middle: our team, our champion large, the enemy hidden.
	var mid := HBoxContainer.new()
	mid.size_flags_vertical = Control.SIZE_EXPAND_FILL
	mid.add_theme_constant_override("separation", 24)
	lobby_box.add_child(mid)
	var ours := VBoxContainer.new()
	ours.custom_minimum_size = Vector2(320, 0)
	ours.add_theme_constant_override("separation", 8)
	var ours_head := Label.new()
	ours_head.text = "YOUR TEAM"
	ours_head.theme_type_variation = "HeaderLabel"
	ours_head.add_theme_color_override("font_color", OWN_COLOR.lightened(0.25))
	ours.add_child(ours_head)
	var enemies := 0
	for slot in l.slots:
		if slot.ally:
			ours.add_child(_lobby_card(slot, OWN_COLOR))
		else:
			enemies += 1
	mid.add_child(ours)

	var center := VBoxContainer.new()
	center.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	center.alignment = BoxContainer.ALIGNMENT_CENTER
	var splash := TextureRect.new()
	splash.texture = portraits.portrait(me.get("champion", ""), "full")
	splash.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	splash.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
	splash.size_flags_vertical = Control.SIZE_EXPAND_FILL
	splash.custom_minimum_size = Vector2(0, 240)
	# Feathered into the backdrop: no hard rectangle around the champion.
	var feather := ShaderMaterial.new()
	feather.shader = _feather_shader()
	splash.material = feather
	center.add_child(splash)
	var name_label := Label.new()
	name_label.text = String(me.get("champion", "")).to_upper()
	name_label.theme_type_variation = "TitleLabel"
	name_label.add_theme_font_size_override("font_size", 40)
	name_label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	center.add_child(name_label)
	var actions := HBoxContainer.new()
	actions.alignment = BoxContainer.ALIGNMENT_CENTER
	actions.add_theme_constant_override("separation", 14)
	var reroll := Button.new()
	reroll.text = "⟳  Reroll (%d)" % me.get("rerolls", 0)
	reroll.disabled = me.get("rerolls", 0) == 0
	reroll.focus_mode = Control.FOCUS_NONE
	reroll.custom_minimum_size = Vector2(150, 44)
	reroll.tooltip_text = "Trade your champion for a random one; yours goes to the bench."
	reroll.pressed.connect(func(): client.lobby_reroll(); _lobby_refresh = 0.0)
	actions.add_child(reroll)
	var ready := Button.new()
	_lobby_ready = me.get("ready", false)
	ready.text = "Not ready" if _lobby_ready else "Ready"
	ready.theme_type_variation = "Button" if _lobby_ready else "PrimaryButton"
	ready.custom_minimum_size = Vector2(200, 44)
	ready.focus_mode = Control.FOCUS_NONE
	ready.pressed.connect(func(): client.lobby_ready(not _lobby_ready); _lobby_refresh = 0.0)
	actions.add_child(ready)
	center.add_child(actions)
	mid.add_child(center)

	var theirs := VBoxContainer.new()
	theirs.custom_minimum_size = Vector2(320, 0)
	theirs.add_theme_constant_override("separation", 8)
	var theirs_head := Label.new()
	theirs_head.text = "ENEMY TEAM"
	theirs_head.theme_type_variation = "HeaderLabel"
	theirs_head.add_theme_color_override("font_color", ENEMY_COLOR.lightened(0.15))
	theirs_head.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	theirs.add_child(theirs_head)
	for i in enemies:
		theirs.add_child(_lobby_card({}, ENEMY_COLOR))
	mid.add_child(theirs)


static var _feather: Shader


static func _feather_shader() -> Shader:
	if _feather == null:
		_feather = Shader.new()
		_feather.code = """
shader_type canvas_item;
void fragment() {
	vec4 c = texture(TEXTURE, UV);
	vec2 d = (UV - vec2(0.5, 0.48)) * vec2(2.0, 1.85);
	c.a *= 1.0 - smoothstep(0.62, 1.0, length(d));
	COLOR = c;
}
"""
	return _feather


## A portrait arrived: champion select (and the loading screen) show it on their next refresh.
func _on_portrait(_champion: String) -> void:
	_portraits_seen += 1


## One champion-select slot: the champion's portrait, its name, who plays it, and whether
## they're ready; ours has a gold rim. An empty slot is a hidden enemy.
func _lobby_card(slot: Dictionary, team: Color) -> Control:
	var hidden := slot.is_empty()
	var you: bool = slot.get("you", false)
	var card := PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.12, 0.1, 0.06, 0.92) if you else Color(0.05, 0.065, 0.085, 0.88)
	sb.border_color = Color(0.78, 0.65, 0.38) if you else Color(0.16, 0.18, 0.22)
	sb.set_border_width_all(1)
	if hidden:
		sb.border_width_right = 4
	else:
		sb.border_width_left = 4
	sb.set_corner_radius_all(4)
	sb.set_content_margin_all(6)
	card.add_theme_stylebox_override("panel", sb)
	if not you:
		sb.border_color = team.darkened(0.35)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 12)
	card.add_child(row)
	var face := TextureRect.new()
	face.custom_minimum_size = Vector2(64, 64)
	face.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	face.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	if not hidden:
		face.texture = portraits.portrait(slot.champion, "bust")
	if face.texture == null:
		# A hidden enemy (or a portrait still rendering): a dark silhouette with a mark.
		var shade := ColorRect.new()
		shade.color = Color(0.09, 0.1, 0.13)
		shade.custom_minimum_size = Vector2(64, 64)
		var q := Label.new()
		q.text = "?" if hidden else String(slot.champion).left(1)
		q.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		q.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
		q.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
		q.add_theme_font_size_override("font_size", 26)
		q.add_theme_color_override("font_color", Color(0.4, 0.43, 0.5))
		shade.add_child(q)
		row.add_child(shade)
	else:
		row.add_child(face)
	var names := VBoxContainer.new()
	names.alignment = BoxContainer.ALIGNMENT_CENTER
	names.add_theme_constant_override("separation", -2)
	names.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var champ := Label.new()
	champ.text = "Hidden" if hidden else slot.champion
	champ.add_theme_font_override("font", _bold_font())
	champ.add_theme_font_size_override("font_size", 17)
	if you:
		champ.add_theme_color_override("font_color", Color(1.0, 0.88, 0.6))
	elif hidden:
		champ.add_theme_color_override("font_color", Color(0.45, 0.48, 0.54))
	names.add_child(champ)
	var who := Label.new()
	if hidden:
		who.text = "Picking…"
	else:
		who.text = "You" if you else ("Bot" if slot.bot else "Player %d" % slot.player)
		if you:
			who.text += "   ·   %d reroll%s" % [slot.rerolls, "" if slot.rerolls == 1 else "s"]
	who.theme_type_variation = "HintLabel"
	names.add_child(who)
	row.add_child(names)
	if not hidden and slot.ready:
		var check := Label.new()
		check.text = "READY"
		check.theme_type_variation = "HeaderLabel"
		check.add_theme_color_override("font_color", Color(0.45, 0.85, 0.5))
		row.add_child(check)
	return card


## ---- The loading screen -----------------------------------------------------------------------
## Between champion select and the match: both teams' champions as tall cards, ours on top,
## until the map is built (a few seconds at most), then it fades.

var loading_panel: Control
var _loading_age := 0.0
const LOADING_MIN_S := 3.0


func _show_loading(l: Dictionary) -> void:
	if loading_panel != null:
		loading_panel.queue_free()
	_loading_age = 0.0
	loading_panel = Control.new()
	loading_panel.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	loading_panel.mouse_filter = Control.MOUSE_FILTER_STOP
	var v := VBoxContainer.new()
	v.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	v.alignment = BoxContainer.ALIGNMENT_CENTER
	v.add_theme_constant_override("separation", 18)
	loading_panel.add_child(v)
	var title := Label.new()
	title.text = "THE BRIDGE  ·  %s" % client.game_type().to_upper()
	title.theme_type_variation = "TitleLabel"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	v.add_child(title)
	for ally in [true, false]:
		var row := HBoxContainer.new()
		row.alignment = BoxContainer.ALIGNMENT_CENTER
		row.add_theme_constant_override("separation", 14)
		for slot in l.slots:
			if slot.ally == ally:
				row.add_child(_loading_card(slot, OWN_COLOR if ally else ENEMY_COLOR))
		v.add_child(row)
	var tip := Label.new()
	tip.text = "Loading the map…"
	tip.theme_type_variation = "HintLabel"
	tip.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	v.add_child(tip)
	overlay.get_parent().add_child(loading_panel)


func _loading_card(slot: Dictionary, team: Color) -> Control:
	var card := PanelContainer.new()
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.04, 0.05, 0.07, 0.95)
	sb.border_color = Color(0.78, 0.65, 0.38) if slot.you else team.darkened(0.25)
	sb.set_border_width_all(2 if slot.you else 1)
	sb.border_width_bottom = 4
	sb.set_corner_radius_all(4)
	card.add_theme_stylebox_override("panel", sb)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 2)
	card.add_child(v)
	var face := TextureRect.new()
	face.custom_minimum_size = Vector2(150, 225)
	face.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	face.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	face.texture = portraits.portrait(slot.champion, "full")
	v.add_child(face)
	var champ := Label.new()
	champ.text = slot.champion
	champ.add_theme_font_override("font", _bold_font())
	champ.add_theme_font_size_override("font_size", 16)
	champ.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	if slot.you:
		champ.add_theme_color_override("font_color", Color(1.0, 0.88, 0.6))
	v.add_child(champ)
	var who := Label.new()
	who.text = "You" if slot.you else ("Bot" if slot.bot else "Player %d" % slot.player)
	who.theme_type_variation = "HintLabel"
	who.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	v.add_child(who)
	return card


func _update_loading(delta: float) -> void:
	if loading_panel == null:
		return
	_loading_age += delta
	var done := _map_built and _loading_age >= LOADING_MIN_S
	if done:
		loading_panel.modulate.a -= delta / 0.4
		if loading_panel.modulate.a <= 0.0:
			loading_panel.queue_free()
			loading_panel = null


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


## Spectators follow a champion (N: the next one), or look at the middle of the map.
func _update_spectator_camera() -> void:
	if not remote_info.has(spectate_target):
		spectate_target = -1
		_spectate_next()
	var target: Vector2 = client.map_geometry().size / 2.0
	if remote_info.has(spectate_target):
		target = remote_info[spectate_target].pos
	if _shot_look != null:
		target = _shot_look
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
	title.text = "Play"
	title.theme_type_variation = "TitleLabel"
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
	join.theme_type_variation = "PrimaryButton"
	join.custom_minimum_size = Vector2(96, 0)
	join.pressed.connect(func(): _menu_join(menu_address.text))
	row.add_child(join)
	v.add_child(row)

	var servers := _load_servers()
	menu_address.text = servers[0].get("address", "") if servers.size() > 0 else "127.0.0.1:7777"
	if servers.size() > 0:
		var head := Label.new()
		head.text = "RECENT SERVERS"
		head.theme_type_variation = "HeaderLabel"
		v.add_child(head)
		for e in servers.slice(0, 6):
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
	hint.theme_type_variation = "HintLabel"
	v.add_child(hint)
	if error != "":
		var err := Label.new()
		err.text = error
		err.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		err.add_theme_color_override("font_color", Color(1.0, 0.5, 0.4))
		v.add_child(err)
	v.add_child(HSeparator.new())
	var quit := Button.new()
	quit.text = "Quit"
	quit.pressed.connect(func(): get_tree().quit())
	v.add_child(quit)
	await get_tree().process_frame
	if menu_panel != null:
		_center(menu_panel)
		# Below the logo when there's room for both.
		menu_panel.position.y = clampf(overlay.size.y * 0.27, 8.0, maxf(8.0, overlay.size.y - menu_panel.size.y - 8.0))
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
		connecting_label.theme_type_variation = "HeaderLabel"
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
	title.text = "Paused"
	title.theme_type_variation = "TitleLabel"
	v.add_child(title)
	var where := Label.new()
	where.text = "%s   ·   %s" % [_server_address, client.game_type()]
	where.theme_type_variation = "HintLabel"
	v.add_child(where)
	var resume := Button.new()
	resume.text = "Resume"
	resume.theme_type_variation = "PrimaryButton"
	resume.pressed.connect(func(): pause_panel.visible = false)
	v.add_child(resume)
	var open_settings := Button.new()
	open_settings.text = "Settings"
	open_settings.pressed.connect(func(): pause_panel.visible = false; _open_settings())
	v.add_child(open_settings)
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


func _open_settings() -> void:
	if settings_panel == null:
		settings_panel = preload("res://scripts/settings_panel.gd").new()
		settings_panel.setup(settings)
		overlay.get_parent().add_child(settings_panel)
		Windows.make_movable(settings_panel, "settings", settings)
	settings_panel.open()


## ---- Minimap and fog of war ----------------------------------------------------------------
## The minimap (minimap.gd) and the fog: our team's vision grid from the extension a few times
## a second, as a texture both the minimap and the world's shaders darken unseen ground with.

const FOG_CELL_U := 50.0
var fog_texture: ImageTexture
var _fog_refresh := 0.0
var _icon_refresh := 0.0


## Map content (ground, cliffs, scenery, platforms) is also on render layer 2: the minimap's
## one top-down render sees only that, never units or structures (those are icons).
const MAP_LAYERS := 3
var _terrain_pending := false
var _dump_terrain := ""                   # --dump-terrain FILE: save the minimap's render


## Polylines (game units) as the ground shader's segments (meters), at most 48.
func _segments(polylines: Array) -> PackedVector4Array:
	var out := PackedVector4Array()
	for line in polylines:
		var pts: PackedVector2Array = line
		for i in range(1, pts.size()):
			if out.size() < 48:
				var a := pts[i - 1] * UNITS_TO_METERS
				var b := pts[i] * UNITS_TO_METERS
				out.append(Vector4(a.x, a.y, b.x, b.y))
	return out


## The direction (blue to red) of the road segment nearest `p` (game units).
func _road_direction(p: Vector2) -> Vector2:
	var best := INF
	var dir := Vector2(1, 0)
	var m := p * UNITS_TO_METERS
	for s in _road_segments:
		var a := Vector2(s.x, s.y)
		var b := Vector2(s.z, s.w)
		var d := m.distance_to(Geometry2D.get_closest_point_to_segment(m, a, b))
		if d < best and a != b:
			best = d
			dir = (b - a).normalized()
	return dir


## Paints the minimap's terrain: one orthographic render of the map from above, units left out.
## The fog waits until it's taken (it would darken the picture).
func _render_minimap_terrain() -> void:
	_terrain_pending = true
	RenderingServer.global_shader_parameter_set("fog_on", 0.0)
	if atmosphere != null:
		atmosphere.suspend(true)
	var region: Rect2 = minimap.region
	var vp := SubViewport.new()
	vp.size = Vector2i(1024, int(1024.0 * region.size.y / region.size.x))
	vp.world_3d = get_viewport().world_3d
	vp.render_target_update_mode = SubViewport.UPDATE_ONCE
	var eye := Camera3D.new()
	eye.projection = Camera3D.PROJECTION_ORTHOGONAL
	eye.keep_aspect = Camera3D.KEEP_HEIGHT
	eye.size = region.size.y * UNITS_TO_METERS
	eye.cull_mask = 2
	eye.far = 200.0
	# A flat, clear look for the map: no haze from 80 m up, no grade, no bloom.
	var plain := Environment.new()
	plain.background_mode = Environment.BG_COLOR
	plain.background_color = Color(0.08, 0.09, 0.1)
	plain.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	plain.ambient_light_color = Color(0.55, 0.6, 0.65)
	plain.ambient_light_energy = 0.6
	eye.environment = plain
	var c := region.get_center() * UNITS_TO_METERS
	eye.position = Vector3(c.x, 80.0, c.y)
	eye.rotation_degrees = Vector3(-90, 0, 0)
	vp.add_child(eye)
	add_child(vp)
	await RenderingServer.frame_post_draw
	await RenderingServer.frame_post_draw
	var top_down := vp.get_texture().get_image()
	minimap.terrain = ImageTexture.create_from_image(top_down)
	if _dump_terrain != "":
		# Map review: the render from above, as the minimap shows it.
		top_down.save_png(_dump_terrain)
		print("MFTR: terrain saved to ", _dump_terrain)
	if atmosphere != null:
		atmosphere.suspend(false)
	minimap.queue_redraw()
	vp.queue_free()
	_terrain_pending = false


func _build_minimap(geo: Dictionary) -> void:
	minimap = preload("res://scripts/minimap.gd").new()
	overlay.get_parent().add_child(minimap)
	minimap.setup(geo)
	minimap.look.connect(func(p: Vector2, held: bool): cam.peek(p, held))
	minimap.move_to.connect(func(p: Vector2):
		if not client.is_spectator():
			client.move_to(p)
			_show_click_marker(_to_world(p), Color(0.45, 0.75, 1.0)))
	RenderingServer.global_shader_parameter_set("fog_size", geo.size * UNITS_TO_METERS)


func _update_minimap(delta: float, playing: bool) -> void:
	if minimap == null:
		return
	minimap.visible = playing and settings.minimap_shown and blind_state != "rating" and loading_panel == null
	_fog_refresh -= delta
	if playing and _fog_refresh <= 0.0 and not _terrain_pending:
		_fog_refresh = 0.15
		var g: Dictionary = client.fog_grid(FOG_CELL_U)
		var img := Image.create_from_data(int(g.cols), int(g.rows), false, Image.FORMAT_L8, g.data)
		if fog_texture == null or fog_texture.get_size() != Vector2(img.get_size()):
			fog_texture = ImageTexture.create_from_image(img)
			RenderingServer.global_shader_parameter_set("fog_map", fog_texture.get_rid())
		else:
			fog_texture.update(img)
		RenderingServer.global_shader_parameter_set("fog_on", 0.0 if client.is_spectator() else 1.0)
		minimap.fog_texture = fog_texture
	if not minimap.visible:
		return
	minimap.layout(overlay.size, settings.minimap_size)
	_icon_refresh -= delta
	if _icon_refresh <= 0.0:
		_icon_refresh = 0.05
		var icons := []
		for id in remote_info:
			var u: Dictionary = remote_info[id]
			if u.get("health", 1.0) <= 0.0:
				continue
			var team := "ally" if u.get("ally", false) else ("neutral" if u.get("neutral", false) else "enemy")
			icons.append({"pos": u.pos, "kind": u.get("kind", ""), "team": team, "champion": u.get("champion", ""),
				"color": CHAMPION_COLORS.get(u.get("champion", ""), Color.GRAY),
				"face": portraits.portrait(u.get("champion", ""), "small")})
		if own_body != null and not own_status.get("dead", false):
			icons.append({"pos": client.own_position(), "kind": "champion", "team": "own", "champion": own_champion,
				"color": CHAMPION_COLORS.get(own_champion, Color.GRAY), "face": portraits.portrait(own_champion, "small")})
		# Champions above everything else.
		icons.sort_custom(func(a, b): return a.kind != "champion" and b.kind == "champion")
		minimap.icons = icons
		var corners := []
		var r := get_viewport().get_visible_rect().size
		for c in [Vector2(0, 0), Vector2(r.x, 0), Vector2(r.x, r.y), Vector2(0, r.y)]:
			var hit = _ground_point(c)
			if hit != null:
				corners.append(Vector2(hit.x, hit.z) / UNITS_TO_METERS)
		minimap.view_poly = PackedVector2Array(corners) if corners.size() == 4 else PackedVector2Array()
		minimap.refresh()


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
	var can_reroll: Array = own_status.get("can_reroll", [])
	var golden: int = own_status.get("golden", -1)
	var key := str(offer.map(func(c): return c.id)) + str(can_reroll) + str(golden)
	if draft_panel == null:
		var made := _panel(860)
		draft_panel = made[0]
		draft_box = made[1]
		Windows.make_movable(draft_panel, "augments", settings)
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
			# Each choice rerolls once; a golden reroll rolls it one tier up.
			var col := VBoxContainer.new()
			col.add_theme_constant_override("separation", 6)
			col.add_child(_augment_card(offer[i], i))
			var can: bool = i < can_reroll.size() and can_reroll[i]
			var reroll := Button.new()
			var gold := i == golden
			reroll.text = ("★  Golden reroll" if gold else "⟳  Reroll") if can else "Rerolled"
			reroll.disabled = not can
			reroll.focus_mode = Control.FOCUS_NONE
			reroll.tooltip_text = "Rerolls this choice into a %s augment." % ("higher-tier" if gold else "different")
			if gold:
				reroll.theme_type_variation = "PrimaryButton"
			var choice := i
			reroll.pressed.connect(func(): client.reroll_augment(choice))
			col.add_child(reroll)
			row.add_child(col)
		var hint := Label.new()
		hint.text = "Drag this window anywhere: your abilities' tooltips stay in reach."
		hint.theme_type_variation = "HintLabel"
		draft_box.add_child(hint)
	# Low on the screen, above the ability bar, so the fight stays visible (or where it was put).
	var default := Vector2((overlay.size.x - draft_panel.size.x) / 2.0, overlay.size.y - draft_panel.size.y - 150.0)
	Windows.place(draft_panel, "augments", settings, default, overlay.size)


## A Stat Anvil's three choices (Mayhem), in a movable window like the draft.
var anvil_panel: PanelContainer
var _anvil_shown := ""


func _update_anvil(playing: bool) -> void:
	var offer: Array = own_status.get("anvil_offer", []) if playing else []
	if offer.is_empty():
		if anvil_panel != null:
			anvil_panel.queue_free()
			anvil_panel = null
		_anvil_shown = ""
		return
	var key := str(offer)
	if anvil_panel == null:
		var made := _panel(700)
		anvil_panel = made[0]
		anvil_panel.set_meta("box", made[1])
		Windows.make_movable(anvil_panel, "anvil", settings)
	if key != _anvil_shown:
		_anvil_shown = key
		var box: VBoxContainer = anvil_panel.get_meta("box")
		for c in box.get_children():
			c.queue_free()
		var title := Label.new()
		title.text = "%s Stat Anvil: keep one" % offer[0].tier
		title.theme_type_variation = "TitleLabel"
		title.add_theme_color_override("font_color", _tier_color(offer[0].tier))
		box.add_child(title)
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 12)
		box.add_child(row)
		for i in offer.size():
			var card := Button.new()
			card.custom_minimum_size = Vector2(210, 90)
			card.focus_mode = Control.FOCUS_NONE
			card.text = String(offer[i].text)
			card.add_theme_font_size_override("font_size", 18)
			card.add_theme_color_override("font_color", _tier_color(offer[i].tier))
			var choice := i
			card.pressed.connect(func(): client.pick_anvil(choice))
			row.add_child(card)
	var default := Vector2((overlay.size.x - anvil_panel.size.x) / 2.0, overlay.size.y - anvil_panel.size.y - 150.0)
	Windows.place(anvil_panel, "anvil", settings, default, overlay.size)


## A draft choice: the augment's icon, name and description in a card that grows to fit them,
## so no description ever spills out (01 §13). Only an unusually long one gets smaller text.
func _augment_card(a: Dictionary, choice: int) -> Control:
	var card := PanelContainer.new()
	card.custom_minimum_size = Vector2(270, 170)
	card.mouse_filter = Control.MOUSE_FILTER_STOP
	card.mouse_default_cursor_shape = Control.CURSOR_POINTING_HAND
	var tint := _tier_color(a.tier)
	var look := func(hot: bool) -> StyleBoxFlat:
		var sb := StyleBoxFlat.new()
		sb.bg_color = Color(0.1, 0.12, 0.15) if not hot else Color(0.14, 0.16, 0.2)
		sb.border_color = tint.darkened(0.35) if not hot else tint
		sb.set_border_width_all(2 if hot else 1)
		sb.set_corner_radius_all(6)
		sb.set_content_margin_all(12)
		return sb
	card.add_theme_stylebox_override("panel", look.call(false))
	card.mouse_entered.connect(func(): card.add_theme_stylebox_override("panel", look.call(true)))
	card.mouse_exited.connect(func(): card.add_theme_stylebox_override("panel", look.call(false)))
	card.gui_input.connect(func(e: InputEvent):
		if e is InputEventMouseButton and e.pressed and e.button_index == MOUSE_BUTTON_LEFT:
			client.pick_augment(choice)
			card.accept_event())
	var v := VBoxContainer.new()
	v.mouse_filter = Control.MOUSE_FILTER_IGNORE
	v.add_theme_constant_override("separation", 8)
	card.add_child(v)
	var head := HBoxContainer.new()
	head.mouse_filter = Control.MOUSE_FILTER_IGNORE
	head.add_theme_constant_override("separation", 10)
	var icon := TextureRect.new()
	icon.texture = AugmentIcons.icon(a)
	icon.custom_minimum_size = Vector2(48, 48)
	icon.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	icon.mouse_filter = Control.MOUSE_FILTER_IGNORE
	head.add_child(icon)
	var name := Label.new()
	name.text = a.name
	name.add_theme_font_override("font", _bold_font())
	name.add_theme_font_size_override("font_size", 18)
	name.add_theme_color_override("font_color", tint)
	name.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	name.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	name.mouse_filter = Control.MOUSE_FILTER_IGNORE
	head.add_child(name)
	v.add_child(head)
	var text := Label.new()
	text.text = a.text
	text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	text.custom_minimum_size = Vector2(246, 0)
	var n: int = String(a.text).length()
	text.add_theme_font_size_override("font_size", 16 if n <= 150 else (14 if n <= 220 else 13))
	text.mouse_filter = Control.MOUSE_FILTER_IGNORE
	v.add_child(text)
	return card


