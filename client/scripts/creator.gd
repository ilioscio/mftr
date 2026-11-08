extends Control
## The champion creator (20, CR3): make a champion step by step, see it in the middle, save it as
## a champion file. The champion being made lives in `MftrCreator` (Rust); this screen only shows
## it and moves its knobs. Steps not built yet (clothes, paint, moves, balance) say so.
##
## The preview turns with a drag (or Q/E); the wheel zooms. Face steps frame the head, the rest
## the whole body.

signal closed
## A champion was saved (its file's path), for the start menu's champion list.
signal saved(path: String)
## Test: play the saved champion (its file, its name) in a practice room (20 §10).
signal test(path: String, champion: String)

const DIR := "user://champions"
const SLOTS := ["skin", "cloth", "metal", "emissive", "accent", "accent_glow"]
const TEAM := Color(0.3, 0.85, 0.8)
## (id, title, what it's for, built yet)
const STEPS := [
	["build", "1  Build", "The body plan: its skeleton, base meshes, sliders and moves.", true],
	["gender", "2  Gender", "The base shape: every slider stays available after the choice.", true],
	["face", "3  Face", "Shape the head: each slider blends between two sculpted extremes.", true],
	["body", "4  Body", "Proportions scale the skeleton; shape sliders reshape the body.", true],
	["clothes", "5  Clothes", "Clothes, armor, accessories and weapons: one part per slot. They follow your sliders.", true],
	["paint", "6  Paint", "Fill each part's colors from the palette; the signature hue lights what glows. Brush and decals: CR5b.", true],
	["moves", "7  Moveset", "A move for the attack and each of Q W E R: its animation, effects and sounds, and its numbers (tuned in Balance).", true],
	["balance", "8  Balance", "Every number has a price; every champion spends the same budget. Turn power and cooldowns, then fit.", true],
	["name", "9  Name", "Name the champion and save it.", true],
]
const FACE_PREFIXES := ["head", "face", "jaw", "chin", "cheek", "brow", "nose", "mouth", "eyes", "ears"]
const PROPORTION_NAMES := {"height": "Height", "legs": "Leg length", "arms": "Arm length", "torso": "Torso length", "head": "Head size", "hands": "Hand size"}

var creator := MftrCreator.create()
var art_root := ""
var step := "build"
const BUILD_NAMES := {"humanoid_slim": "Slim humanoid", "humanoid_heavy": "Heavy humanoid"}
var file_path := ""
var dirty := false
var seed_n := 1

var _viewport: SubViewport
var _camera: Camera3D
var _pivot: Node3D
var _model_node: Node3D = null
var _model: MftrModel = null
var _animator = null
var _skeleton: Skeleton3D = null
var _head_y := 1.62   # the preview's head height (m), for framing the face
var _top_y := 1.9     # its height overall
var _yaw := -0.35
var _zoom := 1.0
var _dragging := false
var _rebuild_in := -1.0
var _step_buttons := {}
var _paint_target := "body"
var _paint_region := 0
var _panel: VBoxContainer
var _status: Label
var _title: Label


func setup(art: String) -> void:
	art_root = art
	creator.set_art(art)


func _ready() -> void:
	# The parent is a canvas layer, not a control: fill the window ourselves.
	_fit()
	get_viewport().size_changed.connect(_fit)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var bg := ColorRect.new()
	bg.color = Color(0.09, 0.1, 0.12)
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	add_child(bg)
	var root := HBoxContainer.new()
	root.set_anchors_preset(Control.PRESET_FULL_RECT)
	root.add_theme_constant_override("separation", 0)
	add_child(root)
	root.add_child(_build_steps())
	root.add_child(_build_stage())
	root.add_child(_build_side())
	_show_step("build")
	_rebuild()


func _build_steps() -> Control:
	var box := PanelContainer.new()
	box.custom_minimum_size = Vector2(230, 0)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 6)
	box.add_child(v)
	var title := Label.new()
	title.text = "Champion Creator"
	title.theme_type_variation = "TitleLabel"
	v.add_child(title)
	for s in STEPS:
		var b := Button.new()
		b.text = s[1]
		b.toggle_mode = true
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.tooltip_text = s[2]
		if not s[3]:
			b.modulate = Color(1, 1, 1, 0.55)
		b.pressed.connect(_show_step.bind(s[0]))
		v.add_child(b)
		_step_buttons[s[0]] = b
	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	v.add_child(spacer)
	var test_button := Button.new()
	test_button.text = "Test"
	test_button.theme_type_variation = "PrimaryButton"
	test_button.tooltip_text = "Save, then play it in a practice duel against a bot (Esc comes back here)."
	test_button.pressed.connect(_test)
	v.add_child(test_button)
	for row in [["Randomize", _randomize], ["Reset shape", _reset], ["New", _new], ["Load…", _load_dialog], ["Save", _save], ["Close", _close]]:
		var b := Button.new()
		b.text = row[0]
		b.pressed.connect(row[1])
		v.add_child(b)
	return box


func _build_stage() -> Control:
	var c := SubViewportContainer.new()
	c.stretch = true
	c.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	c.size_flags_vertical = Control.SIZE_EXPAND_FILL
	c.gui_input.connect(_stage_input)
	_viewport = SubViewport.new()
	_viewport.own_world_3d = true
	_viewport.msaa_3d = Viewport.MSAA_4X
	c.add_child(_viewport)
	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_COLOR
	e.background_color = Color(0.16, 0.17, 0.2)
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color(0.55, 0.58, 0.65)
	e.ambient_light_energy = 0.6
	env.environment = e
	_viewport.add_child(env)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-45, -30, 0)
	sun.light_energy = 1.1
	_viewport.add_child(sun)
	var floor := MeshInstance3D.new()
	var disc := CylinderMesh.new()
	disc.top_radius = 0.8
	disc.bottom_radius = 0.8
	disc.height = 0.02
	floor.mesh = disc
	floor.position.y = -0.01
	var fm := StandardMaterial3D.new()
	fm.albedo_color = Color(0.22, 0.24, 0.27)
	floor.material_override = fm
	_viewport.add_child(floor)
	_pivot = Node3D.new()
	_viewport.add_child(_pivot)
	_camera = Camera3D.new()
	_camera.fov = 30
	_viewport.add_child(_camera)
	_place_camera()
	return c


func _build_side() -> Control:
	var box := PanelContainer.new()
	box.custom_minimum_size = Vector2(380, 0)
	var outer := VBoxContainer.new()
	box.add_child(outer)
	_title = Label.new()
	_title.theme_type_variation = "TitleLabel"
	outer.add_child(_title)
	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	outer.add_child(scroll)
	_panel = VBoxContainer.new()
	_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_panel.add_theme_constant_override("separation", 6)
	scroll.add_child(_panel)
	_status = Label.new()
	_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_status.add_theme_color_override("font_color", Color(1.0, 0.6, 0.45))
	outer.add_child(_status)
	return box


func _fit() -> void:
	position = Vector2.ZERO
	size = get_viewport_rect().size


# --- Steps -------------------------------------------------------------------------------------

func _show_step(id: String) -> void:
	step = id
	for k in _step_buttons:
		_step_buttons[k].set_pressed_no_signal(k == id)
	for c in _panel.get_children():
		c.queue_free()
	var info: Array = STEPS.filter(func(s): return s[0] == id)[0]
	_title.text = info[1].substr(3)
	_hint(info[2])
	match id:
		"build":
			for b in creator.builds():
				_choice(BUILD_NAMES.get(String(b[0]), String(b[0]).capitalize()), creator.build() == b[0], func():
					creator.set_build(b[0])
					if not b[1]:
						creator.set_gender("")
					_changed(true))
			_hint("Both humanoids share the moves and parts; the heavy one is the brute (Rook, Bastion, Cairn). More builds (beast, serpentoid, spectre, machine) arrive as their own slices (CR10).")
		"gender":
			for g in [["masculine", "Masculine"], ["feminine", "Feminine"]]:
				_choice(g[1], creator.gender() == g[0], func():
					creator.set_gender(g[0])
					_changed(true))
			_skin_picker()
		"face":
			for id2 in creator.sliders():
				if _is_face(id2):
					_slider(_label_of(id2), creator.slider(id2), func(v): creator.set_slider(id2, v))
		"body":
			for p in creator.proportions():
				_slider(PROPORTION_NAMES.get(p, p), creator.proportion(p), func(v): creator.set_proportion(p, v))
			_panel.add_child(HSeparator.new())
			for id2 in creator.sliders():
				if not _is_face(id2):
					_slider(_label_of(id2), creator.slider(id2), func(v): creator.set_slider(id2, v))
		"clothes":
			var by_slot := {}
			for p in creator.parts():
				if not by_slot.has(p.slot):
					by_slot[p.slot] = []
				by_slot[p.slot].append(p)
			var worn: PackedStringArray = creator.outfit()
			for slot in creator.slots():
				if not by_slot.has(slot):
					continue
				var l := Label.new()
				l.text = String(slot).capitalize()
				_panel.add_child(l)
				var pick := OptionButton.new()
				pick.add_item("None")
				var options: Array = by_slot[slot]
				for i in options.size():
					pick.add_item("%s  (%d)" % [options[i].name, options[i].triangles])
					if worn.has(options[i].id):
						pick.select(i + 1)
				pick.item_selected.connect(func(i):
					creator.set_part(slot, "" if i == 0 else options[i - 1].id)
					_changed(true))
				_panel.add_child(pick)
			_hint("Paint comes next (CR5); parts wear the catalog's colors until then.")
		"paint":
			var targets: PackedStringArray = creator.paint_targets()
			if not targets.has(_paint_target):
				_paint_target = "body"
			var tpick := OptionButton.new()
			for i in targets.size():
				tpick.add_item("Body" if targets[i] == "body" else targets[i])
				if targets[i] == _paint_target:
					tpick.select(i)
			tpick.item_selected.connect(func(i):
				_paint_target = targets[i]
				_paint_region = 0
				_show_step("paint"))
			_panel.add_child(tpick)
			var regions: PackedColorArray = creator.paint_regions(_paint_target)
			var row := HBoxContainer.new()
			for i in regions.size():
				row.add_child(_swatch(regions[i], i == _paint_region, func():
					_paint_region = i
					_show_step("paint")))
			_panel.add_child(row)
			var grid := GridContainer.new()
			grid.columns = 8
			var palette: PackedColorArray = creator.palette()
			for k in palette.size():
				grid.add_child(_swatch(palette[k], false, func():
					creator.fill(_paint_target, _paint_region, k)
					_changed(true)
					_show_step("paint")))
			_panel.add_child(grid)
			var sig := Label.new()
			sig.text = "Signature hue (what glows, and later the effects)"
			_panel.add_child(sig)
			var hue := HSlider.new()
			hue.min_value = 0
			hue.max_value = 359
			hue.step = 1
			hue.value = creator.signature()
			hue.value_changed.connect(func(v):
				creator.set_signature(int(v))
				_changed(true))
			_panel.add_child(hue)
			var clear := Button.new()
			clear.text = "Clear paint"
			clear.pressed.connect(func():
				creator.clear_paint()
				_changed(true)
				_show_step("paint"))
			_panel.add_child(clear)
		"moves":
			for slot in ["attack", "q", "w", "e", "r"]:
				var l := Label.new()
				l.text = "Attack" if slot == "attack" else slot.to_upper()
				_panel.add_child(l)
				var mpick := OptionButton.new()
				var options: Array = creator.move_library(slot)
				var on: String = creator.move_on(slot)
				for i in options.size():
					mpick.add_item(options[i].label)
					if options[i].id == on:
						mpick.select(i)
				mpick.item_selected.connect(func(i):
					creator.set_move(slot, options[i].id)
					_changed(true)
					_show_step("moves"))
				_panel.add_child(mpick)
			# The passive (20 §8.1): one from the shared catalog, or none.
			var pl := Label.new()
			pl.text = "Passive"
			_panel.add_child(pl)
			var ppick := OptionButton.new()
			var passives: Array = creator.passives()
			var held: String = creator.passive()
			ppick.add_item("None")
			for i in passives.size():
				ppick.add_item(passives[i].label)
				ppick.set_item_tooltip(i + 1, passives[i].hint)
				if passives[i].id == held:
					ppick.select(i + 1)
			ppick.item_selected.connect(func(i):
				creator.set_passive("" if i == 0 else passives[i - 1].id)
				_changed(true)
				_show_step("moves"))
			_panel.add_child(ppick)
			for p in passives:
				if p.id == held:
					_hint(p.hint)
			_panel.add_child(HSeparator.new())
			var kl := Label.new()
			kl.text = "Or take a whole kit"
			_panel.add_child(kl)
			var pick := OptionButton.new()
			var kits: PackedStringArray = creator.kits()
			pick.add_item("—")
			for i in kits.size():
				pick.add_item(kits[i])
				if kits[i] == creator.kit():
					pick.select(i + 1)
			pick.item_selected.connect(func(i):
				if i == 0:
					return
				creator.set_kit(kits[i - 1])
				_changed(true)
				_show_step("moves"))
			_panel.add_child(pick)
			for line in creator.kit_summary():
				var l := Label.new()
				l.text = line
				_panel.add_child(l)
		"balance":
			var price: Dictionary = creator.price()
			var total: float = price.total
			var bar := ProgressBar.new()
			bar.max_value = price.cap
			bar.value = minf(total, price.cap)
			bar.show_percentage = false
			bar.custom_minimum_size = Vector2(0, 18)
			_panel.add_child(bar)
			var tl := Label.new()
			tl.text = "%.0f of %.0f points%s" % [total, price.budget, "  —  over!" if total > price.cap else ""]
			tl.add_theme_color_override("font_color", Color(1.0, 0.5, 0.4) if total > price.cap else Color(0.7, 0.95, 0.7))
			_panel.add_child(tl)
			for part in price.parts:
				var pl := Label.new()
				pl.text = "    %s  %.1f" % [String(part[0]).capitalize(), part[1]]
				pl.theme_type_variation = "HintLabel"
				_panel.add_child(pl)
			_panel.add_child(HSeparator.new())
			for slot in ["attack", "q", "w", "e", "r"]:
				var t: PackedFloat32Array = creator.tuning(slot)
				var head := Label.new()
				head.text = ("Attack: damage and speed" if slot == "attack" else "%s: power and cooldown" % slot.to_upper())
				_panel.add_child(head)
				_tune_slider("Power ×", t[0], 0.5, 1.6, func(v):
					var now: PackedFloat32Array = creator.tuning(slot)
					creator.tune(slot, v, now[1])
					_changed(false))
				_tune_slider("Cooldown ×" if slot != "attack" else "Time between attacks ×", t[1], 0.6, 2.0, func(v):
					var now: PackedFloat32Array = creator.tuning(slot)
					creator.tune(slot, now[0], v)
					_changed(false))
			_panel.add_child(HSeparator.new())
			var stats: PackedFloat32Array = creator.stat_tuning()
			var names := ["Health ×", "Armor ×", "Magic resist ×", "Move speed ×"]
			for i in 4:
				var lo := 0.94 if i == 3 else 0.8
				var hi := 1.06 if i == 3 else 1.2
				_tune_slider(names[i], stats[i], lo, hi, func(v):
					var now: PackedFloat32Array = creator.stat_tuning()
					now[i] = v
					creator.tune_stats(now)
					_changed(false))
			var fit := Button.new()
			fit.text = "Auto-fit to the budget"
			fit.pressed.connect(func():
				creator.auto_fit()
				_changed(false)
				_show_step("balance"))
			_panel.add_child(fit)
		"name":
			var edit := LineEdit.new()
			edit.text = creator.name()
			edit.placeholder_text = "Name (1–32 characters)"
			edit.text_changed.connect(func(t):
				creator.set_name(t)
				_changed(false))
			_panel.add_child(edit)
			var save := Button.new()
			save.text = "Save champion"
			save.theme_type_variation = "PrimaryButton"
			save.pressed.connect(_save)
			_panel.add_child(save)
		_:
			_hint("Not built yet.")
	_place_camera()
	_update_status()


func _is_face(id: String) -> bool:
	return FACE_PREFIXES.any(func(p): return id.begins_with(p))


func _label_of(id: String) -> String:
	return id.replace("_", " ").capitalize()


func _hint(text: String) -> void:
	var l := Label.new()
	l.text = text
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	l.theme_type_variation = "HintLabel"
	_panel.add_child(l)


func _choice(text: String, on: bool, act: Callable) -> void:
	var b := Button.new()
	b.text = text
	b.toggle_mode = true
	b.button_pressed = on
	b.pressed.connect(func():
		act.call()
		_show_step(step))
	_panel.add_child(b)


func _slider(text: String, value: float, set_value: Callable) -> void:
	var row := VBoxContainer.new()
	var l := Label.new()
	l.text = text
	row.add_child(l)
	var s := HSlider.new()
	s.min_value = -1.0
	s.max_value = 1.0
	s.step = 0.01
	s.value = value
	s.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	s.value_changed.connect(func(v):
		set_value.call(v)
		_changed(true))
	row.add_child(s)
	_panel.add_child(row)


func _swatch(color: Color, selected: bool, act: Callable) -> Button:
	var b := Button.new()
	b.custom_minimum_size = Vector2(36, 36)
	var sb := StyleBoxFlat.new()
	sb.bg_color = color
	sb.set_border_width_all(3 if selected else 0)
	sb.border_color = Color.WHITE
	b.add_theme_stylebox_override("normal", sb)
	b.add_theme_stylebox_override("hover", sb)
	b.add_theme_stylebox_override("pressed", sb)
	b.pressed.connect(act)
	return b


func _tune_slider(text: String, value: float, lo: float, hi: float, set_value: Callable) -> void:
	var row := HBoxContainer.new()
	var l := Label.new()
	l.text = text
	l.custom_minimum_size = Vector2(150, 0)
	row.add_child(l)
	var s := HSlider.new()
	s.min_value = lo
	s.max_value = hi
	s.step = 0.01
	s.value = value
	s.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	var v := Label.new()
	v.text = "%.2f" % value
	v.custom_minimum_size = Vector2(44, 0)
	s.value_changed.connect(func(x):
		v.text = "%.2f" % x
		set_value.call(x))
	s.drag_ended.connect(func(_changed_value): _show_step(step))
	row.add_child(s)
	row.add_child(v)
	_panel.add_child(row)


func _skin_picker() -> void:
	var l := Label.new()
	l.text = "Skin"
	_panel.add_child(l)
	var grid := GridContainer.new()
	grid.columns = 8
	var tones: PackedColorArray = creator.skin_tones()
	for i in tones.size():
		var b := Button.new()
		b.custom_minimum_size = Vector2(36, 36)
		var sb := StyleBoxFlat.new()
		sb.bg_color = tones[i]
		sb.set_border_width_all(3 if creator.skin() == i else 0)
		sb.border_color = Color.WHITE
		b.add_theme_stylebox_override("normal", sb)
		b.add_theme_stylebox_override("hover", sb)
		b.pressed.connect(func():
			creator.set_skin(i)
			_changed(true)
			_show_step(step))
		grid.add_child(b)
	_panel.add_child(grid)


# --- The preview -------------------------------------------------------------------------------

## Something changed: rebuild the preview soon (sliders move many times a second).
func _changed(shape: bool) -> void:
	dirty = true
	if shape:
		_rebuild_in = 0.06
	_update_status()


func _process(delta: float) -> void:
	if _rebuild_in >= 0.0:
		_rebuild_in -= delta
		if _rebuild_in < 0.0:
			_rebuild()
	if _animator != null and _skeleton != null:
		_animator.drive(_skeleton, {}, 0.0, delta)


func _rebuild() -> void:
	var m: MftrModel = creator.preview()
	if m == null:
		return
	if _model_node != null:
		_model_node.queue_free()
	_model = m
	_model_node = m.instantiate()
	var mesh: MeshInstance3D = _model_node.get_node("Skeleton/Mesh")
	var slots: PackedStringArray = m.surface_slots()
	for i in slots.size():
		var mat := ShaderMaterial.new()
		mat.shader = load("res://shaders/champion_model.gdshader")
		mat.set_shader_parameter("slot", SLOTS.find(slots[i]))
		mat.set_shader_parameter("team_accent", TEAM)
		mesh.set_surface_override_material(i, mat)
	_pivot.add_child(_model_node)
	_update_status()
	_skeleton = _model_node.get_node("Skeleton")
	_animator = m.new_animator()
	_pivot.rotation.y = _yaw
	var head := _skeleton.find_bone("head")
	if head >= 0:
		_head_y = _skeleton.get_bone_global_rest(head).origin.y + 0.13
		_top_y = _head_y + 0.15
	_place_camera()


func _place_camera() -> void:
	if _camera == null:
		return
	var face := step == "face" or step == "gender"
	var target := Vector3(0, _head_y, 0) if face else Vector3(0, _top_y * 0.52, 0)
	var dist := (1.5 if face else _top_y * 2.3) / _zoom
	_camera.look_at_from_position(target + Vector3(0, 0.12 if face else 0.4, dist), target)


func _stage_input(event: InputEvent) -> void:
	if event is InputEventMouseButton:
		if event.button_index == MOUSE_BUTTON_LEFT:
			_dragging = event.pressed
		elif event.button_index == MOUSE_BUTTON_WHEEL_UP and event.pressed:
			_zoom = minf(_zoom * 1.1, 3.0)
			_place_camera()
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN and event.pressed:
			_zoom = maxf(_zoom / 1.1, 0.5)
			_place_camera()
	elif event is InputEventMouseMotion and _dragging:
		_yaw += event.relative.x * 0.01
		_pivot.rotation.y = _yaw


func _unhandled_key_input(event: InputEvent) -> void:
	if not (event is InputEventKey and event.pressed):
		return
	if event.keycode == KEY_Q:
		_yaw -= 0.3
	elif event.keycode == KEY_E:
		_yaw += 0.3
	elif event.keycode == KEY_S and event.ctrl_pressed:
		_save()
	_pivot.rotation.y = _yaw


# --- Files -------------------------------------------------------------------------------------

func _update_status() -> void:
	var problems: PackedStringArray = creator.problems()
	var budget := "" if _model == null else "%d triangles (target 5,000–8,000, cap 12,000)\n" % _model.triangles()
	_status.text = budget + ("" if problems.is_empty() else "Can't save yet: " + "; ".join(problems))


func _randomize() -> void:
	seed_n += 1
	creator.randomize(int(Time.get_ticks_usec()) + seed_n)
	_changed(true)
	_show_step(step)


func _reset() -> void:
	creator.reset_shape()
	_changed(true)
	_show_step(step)


func _new() -> void:
	creator = MftrCreator.create()
	creator.set_art(art_root)
	file_path = ""
	dirty = false
	_show_step("build")
	_rebuild()


func _save() -> void:
	DirAccess.make_dir_recursive_absolute(ProjectSettings.globalize_path(DIR))
	var slug := creator.name().to_lower().strip_edges().replace(" ", "_")
	var path := file_path if file_path != "" else ProjectSettings.globalize_path("%s/%s.champion.ron" % [DIR, slug])
	var err: String = creator.save(path)
	if err != "":
		_status.text = "Not saved: " + err
		return
	file_path = path
	dirty = false
	_status.add_theme_color_override("font_color", Color(0.6, 0.95, 0.6))
	_status.text = "Saved %s" % path
	saved.emit(path)


func _test() -> void:
	_save()
	if file_path == "" or dirty:
		return
	test.emit(file_path, creator.name())
	queue_free()


func load_file(path: String) -> void:
	var err: String = creator.load(path)
	if err != "":
		_status.text = "Couldn't load: " + err
		return
	file_path = path
	dirty = false
	_show_step(step)
	_rebuild()


func _load_dialog() -> void:
	var d := FileDialog.new()
	d.file_mode = FileDialog.FILE_MODE_OPEN_FILE
	d.access = FileDialog.ACCESS_FILESYSTEM
	d.current_dir = ProjectSettings.globalize_path(DIR)
	d.filters = PackedStringArray(["*.champion.ron ; Champion files"])
	d.file_selected.connect(load_file)
	add_child(d)
	d.popup_centered(Vector2i(800, 520))


func _close() -> void:
	closed.emit()
	queue_free()
