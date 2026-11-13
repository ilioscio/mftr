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
const UI := preload("res://scripts/ui_theme.gd")
const FaceLife := preload("res://scripts/face_life.gd")
const SLOTS := ["skin", "cloth", "metal", "emissive", "accent", "accent_glow"]
const TEAM := Color(0.3, 0.85, 0.8)
## (id, title, what it's for, built yet)
const STEPS := [
	["build", "1  Build", "The body plan: its skeleton, base meshes, sliders and moves.", true],
	["gender", "2  Gender", "The base shape: every slider stays available after the choice.", true],
	["face", "3  Face", "Shape the head: each slider moves the face's bones. Sculpt for anything they can't reach.", true],
	["body", "4  Body", "Proportions scale the skeleton; shape sliders reshape the body.", true],
	["clothes", "5  Clothes", "Clothes, armor, accessories and weapons: one part per slot. They follow your sliders.", true],
	["paint", "6  Paint", "Fill each part's colors from the palette; the signature hue lights what glows. Brush, decals and the eraser paint right on the model.", true],
	["moves", "7  Moveset", "A move for the attack and each of Q W E R: its animation, effects and sounds, and its numbers (tuned in Balance).", true],
	["balance", "8  Balance", "Every number has a price; every champion spends the same budget. Turn power and cooldowns, then fit.", true],
	["name", "9  Name", "Name the champion and save it.", true],
]
const FACE_PREFIXES := ["head", "face", "jaw", "chin", "cheek", "brow", "nose", "mouth", "lips", "eyes", "lids", "ears"]
## The Face step, feature by feature (20 §5.2): each slider as [id, name, its -1 end, its +1 end].
## A slider the base has that isn't listed lands under "More".
const FACE_GROUPS := [
	["Head", [["head_width", "Width", "narrow", "broad"], ["head_crown", "Crown", "flat", "tall"],
		["head_forehead", "Forehead", "sloping", "tall, upright"], ["face_length", "Lower face", "short", "long"]]],
	["Brow", [["brow_ridge", "Ridge", "smooth", "heavy"], ["brow_height", "Height", "low", "high"],
		["brow_angle", "Set", "worried", "scowling"]]],
	["Eyes", [["eyes_size", "Size", "small", "big"], ["eyes_spacing", "Spacing", "close-set", "wide-set"],
		["eyes_height", "Height", "low", "high"], ["eyes_tilt", "Slant", "down at the corners", "up at the corners"],
		["eyes_depth", "Depth", "prominent", "deep-set"]]],
	["Lids", [["lids_open", "Openness", "narrow", "wide"], ["lids_crease", "Crease", "none (monolid)", "deep"],
		["lids_fold", "Inner fold", "", "epicanthic"], ["lids_hood", "Hood", "", "hooded"],
		["lids_inner", "Inner corners", "low", "high"]]],
	["Nose", [["nose_length", "Length", "short", "long"], ["nose_reach", "Projection", "flat", "prominent"],
		["nose_width", "Width", "thin", "broad"], ["nose_hook", "Bridge", "scooped", "hooked"],
		["nose_tip", "Tip", "drooping", "upturned"]]],
	["Cheeks", [["cheekbones", "Cheekbones", "flat", "high, sharp"], ["cheeks_hollow", "Fullness", "full", "gaunt"]]],
	["Mouth", [["mouth_width", "Width", "small", "wide"], ["mouth_lips", "Lips", "thin", "full"],
		["mouth_height", "Height", "low", "high"], ["mouth_corners", "Corners", "frown", "smile"],
		["lips_upper", "Upper lip", "thin", "full"], ["lips_lower", "Lower lip", "thin", "full"],
		["lips_bow", "Cupid's bow", "", "marked"]]],
	["Jaw and chin", [["jaw_width", "Jaw width", "narrow", "wide, square"], ["jaw_corners", "Jaw corners", "tapered", "heavy"],
		["chin_length", "Chin length", "short", "long"], ["chin_reach", "Chin", "receding", "jutting"]]],
	["Ears", [["ears_size", "Size", "small", "big"], ["ears_point", "Points", "", "pointed"]]],
]
const PROPORTION_NAMES := {"height": "Height", "legs": "Leg length", "arms": "Arm length", "torso": "Torso length", "head": "Head size", "hands": "Hand size"}

var creator := MftrCreator.create()
var art_root := ""
var step := "build"
const BUILD_NAMES := {"humanoid_slim": "Slim humanoid", "humanoid_heavy": "Heavy humanoid", "beast": "Four-legged beast",
	"serpent": "Serpentoid", "spectre": "Floating spectre", "machine": "Rolling machine"}
const BUILD_INFO := {
	"humanoid_slim": "Lean and quick. Every humanoid part and move, a sculptable face.",
	"humanoid_heavy": "The brute: broad, low-hipped, long-armed, like Rook, Bastion and Cairn.",
	"beast": "Four legs. Any kit, animated by its own moves' shapes; its own parts.",
	"serpent": "A coiling body that slithers; its own parts and moves.",
	"spectre": "Robed and floating above the ground; its own parts and moves.",
	"machine": "Rolls on a wheel: plates, pistons, its own parts and moves.",
}
var file_path := ""
var dirty := false
var seed_n := 1

var _viewport: SubViewport
# The Paint step's tool (CR5b): "fill" a region, or paint on the model with a "brush", a
# "decal" or the "erase" brush; the brush color (a palette index), size (m) and decal.
var _paint_tool := "fill"
# Which brow the Brows panel shapes when each has its own (22 §3.1).
var _brow_side := "left"
# Whether the hair has a color of its own, apart from the brows' and the beard's (22 §7 item 5).
var _hair_own := false
# The side panel's scroll (`scroll_panel`, for captures).
var _scroll: ScrollContainer
var _brush_color := 18
var _brush_size := 0.1
var _decal := "star"
var _painting := false
# Free sculpting on the Face and Body steps (CR9): the brush ("" when off), its size and
# strength (m), symmetry.
var _sculpt_tool := ""
var _sculpt_size := 0.05
var _sculpt_strength := 0.004
var _sculpt_mirror := true
# The face editor (FR4): the region under the cursor, the drag under way ("" or its mode), the
# preview's materials (their hover and active uniforms), the stage's how-to label.
var _hover_region := -1
var _face_drag := ""
var _mats: Array[ShaderMaterial] = []
var _how: Label
const HOW := "Drag to turn   ·   Wheel to zoom   ·   Q / E"
var _camera: Camera3D
var _pivot: Node3D
var _model_node: Node3D = null
var _model: MftrModel = null
var _animator = null
## The preview's eyes (22 §2.4): they look at the camera and blink.
var _life: FaceLife = null
## A pose to hold them in for captures (`--creator-life`), or null.
var _life_hold = null
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
var _title: Label
var _step_label: Label
var _hint_label: Label
var _name_label: Label
var _sub_label: Label
var _toast: PanelContainer
var _toast_label: Label
var _meter: Label
var _meter_bar: ProgressBar


func setup(art: String) -> void:
	art_root = art
	creator.set_art(art)


func _ready() -> void:
	# The parent is a canvas layer, not a control: fill the window ourselves.
	_fit()
	get_viewport().size_changed.connect(_fit)
	mouse_filter = Control.MOUSE_FILTER_STOP
	var bg := ColorRect.new()
	bg.color = Color(0.03, 0.035, 0.05)
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


## `c` inside a margin of `m` pixels on every side.
func _pad(c: Control, m := 16) -> MarginContainer:
	var mc := MarginContainer.new()
	for side in ["left", "right", "top", "bottom"]:
		mc.add_theme_constant_override("margin_" + side, m)
	mc.add_child(c)
	return mc


func _gap(h: float) -> void:
	var g := Control.new()
	g.custom_minimum_size = Vector2(0, h)
	_panel.add_child(g)


## The left rail: who is being made, the steps (the current one marked in gold), and what to do
## with it (Test first).
func _build_steps() -> Control:
	var box := PanelContainer.new()
	box.custom_minimum_size = Vector2(264, 0)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 3)
	box.add_child(_pad(v, 18))
	var eyebrow := Label.new()
	eyebrow.text = "CHAMPION CREATOR"
	eyebrow.theme_type_variation = "HeaderLabel"
	v.add_child(eyebrow)
	_name_label = Label.new()
	_name_label.theme_type_variation = "TitleLabel"
	_name_label.clip_text = true
	_name_label.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	v.add_child(_name_label)
	_sub_label = Label.new()
	_sub_label.theme_type_variation = "HintLabel"
	# Wraps rather than widening the rail ("unsaved" comes and goes).
	_sub_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_sub_label.custom_minimum_size = Vector2(200, 0)
	v.add_child(_sub_label)
	var gap := Control.new()
	gap.custom_minimum_size = Vector2(0, 14)
	v.add_child(gap)
	for s in STEPS:
		var b := Button.new()
		b.text = "%s     %s" % [s[1].substr(0, 1), s[1].substr(3)]
		b.toggle_mode = true
		b.alignment = HORIZONTAL_ALIGNMENT_LEFT
		b.tooltip_text = s[2]
		_style_step(b)
		b.pressed.connect(_show_step.bind(s[0]))
		v.add_child(b)
		_step_buttons[s[0]] = b
	var spacer := Control.new()
	spacer.size_flags_vertical = Control.SIZE_EXPAND_FILL
	v.add_child(spacer)
	var test_button := Button.new()
	test_button.text = "Test in a practice duel"
	test_button.theme_type_variation = "PrimaryButton"
	test_button.custom_minimum_size = Vector2(0, 44)
	test_button.tooltip_text = "Save, then play it in a practice duel against a bot (leave the match to come back here)."
	test_button.pressed.connect(_test)
	v.add_child(test_button)
	var gap2 := Control.new()
	gap2.custom_minimum_size = Vector2(0, 6)
	v.add_child(gap2)
	var rows := [
		[["Save", _save, "Save the champion file (Ctrl+S)"], ["Load…", _load_dialog, "Open a champion file"]],
		[["New", _new, "Start over from a plain body"], ["Close", _close, "Back to the menu"]],
	]
	for pair in rows:
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 6)
		for item in pair:
			var b := Button.new()
			b.text = item[0]
			b.tooltip_text = item[2]
			b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
			b.pressed.connect(item[1])
			row.add_child(b)
		v.add_child(row)
	return box


func _style_step(b: Button) -> void:
	var m := Vector4(14, 9, 10, 9)
	var on := UI._box(UI.SLATE_HI, UI.GOLD, 0, 4, m)
	on.border_width_left = 3
	b.add_theme_stylebox_override("normal", UI._box(Color(0, 0, 0, 0), Color(0, 0, 0, 0), 0, 4, m))
	b.add_theme_stylebox_override("hover", UI._box(UI.SLATE, Color(0, 0, 0, 0), 0, 4, m))
	b.add_theme_stylebox_override("pressed", on)
	b.add_theme_stylebox_override("hover_pressed", on)
	b.add_theme_stylebox_override("focus", StyleBoxEmpty.new())
	b.add_theme_color_override("font_color", UI.DIM)
	b.add_theme_color_override("font_hover_color", UI.TEXT)
	b.add_theme_color_override("font_pressed_color", UI.CREAM)
	b.add_theme_color_override("font_hover_pressed_color", UI.CREAM)


## The stage: the champion on a pedestal against a vignette, lit from the front with a cool rim
## from behind. Over it: how to turn it, the shape tools, the triangle meter and messages.
func _build_stage() -> Control:
	var area := Control.new()
	area.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	area.size_flags_vertical = Control.SIZE_EXPAND_FILL
	area.clip_contents = true
	var bg := TextureRect.new()
	var g := Gradient.new()
	g.set_color(0, Color(0.22, 0.24, 0.29))
	g.set_color(1, Color(0.045, 0.05, 0.07))
	var gt := GradientTexture2D.new()
	gt.gradient = g
	gt.fill = GradientTexture2D.FILL_RADIAL
	gt.fill_from = Vector2(0.5, 0.45)
	gt.fill_to = Vector2(1.1, 1.05)
	gt.width = 512
	gt.height = 512
	bg.texture = gt
	bg.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	bg.stretch_mode = TextureRect.STRETCH_SCALE
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	area.add_child(bg)
	var c := SubViewportContainer.new()
	c.stretch = true
	c.set_anchors_preset(Control.PRESET_FULL_RECT)
	c.gui_input.connect(_stage_input)
	area.add_child(c)
	_viewport = SubViewport.new()
	_viewport.own_world_3d = true
	_viewport.transparent_bg = true
	_viewport.msaa_3d = Viewport.MSAA_4X
	c.add_child(_viewport)
	var env := WorldEnvironment.new()
	var e := Environment.new()
	e.background_mode = Environment.BG_CLEAR_COLOR
	e.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	e.ambient_light_color = Color(0.55, 0.58, 0.65)
	e.ambient_light_energy = 0.55
	env.environment = e
	_viewport.add_child(env)
	var sun := DirectionalLight3D.new()
	sun.rotation_degrees = Vector3(-45, -30, 0)
	sun.light_energy = 1.15
	sun.light_color = Color(1.0, 0.96, 0.9)
	# The stage is one figure: shadows over a few meters, so the shadow map is fine enough for
	# a chin's or a staff's (the default 100 m drew them as jagged blocks).
	sun.shadow_enabled = true
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_ORTHOGONAL
	sun.directional_shadow_max_distance = 6.0
	sun.shadow_blur = 1.5
	_viewport.add_child(sun)
	# A cool rim from behind: the silhouette stands off the backdrop.
	var rim := DirectionalLight3D.new()
	rim.rotation_degrees = Vector3(-25, 155, 0)
	rim.light_energy = 0.7
	rim.light_color = Color(0.62, 0.74, 1.0)
	_viewport.add_child(rim)
	# The pedestal: a dark disc with a gold ring.
	var floor := MeshInstance3D.new()
	var disc := CylinderMesh.new()
	disc.top_radius = 0.8
	disc.bottom_radius = 0.84
	disc.height = 0.06
	floor.mesh = disc
	floor.position.y = -0.03
	var fm := StandardMaterial3D.new()
	fm.albedo_color = Color(0.12, 0.13, 0.16)
	fm.roughness = 0.85
	floor.material_override = fm
	_viewport.add_child(floor)
	var ring := MeshInstance3D.new()
	var torus := TorusMesh.new()
	torus.inner_radius = 0.8
	torus.outer_radius = 0.825
	ring.mesh = torus
	ring.scale = Vector3(1.0, 0.4, 1.0)
	var rm := StandardMaterial3D.new()
	rm.albedo_color = UI.RIM
	rm.emission_enabled = true
	rm.emission = UI.GOLD * 0.25
	ring.material_override = rm
	_viewport.add_child(ring)
	_pivot = Node3D.new()
	_viewport.add_child(_pivot)
	_camera = Camera3D.new()
	_camera.fov = 30
	_viewport.add_child(_camera)
	_place_camera()
	area.add_child(_build_overlay())
	return area


func _build_overlay() -> Control:
	var over := VBoxContainer.new()
	over.set_anchors_preset(Control.PRESET_FULL_RECT)
	over.mouse_filter = Control.MOUSE_FILTER_IGNORE
	var top := HBoxContainer.new()
	top.mouse_filter = Control.MOUSE_FILTER_IGNORE
	top.add_theme_constant_override("separation", 6)
	var how := Label.new()
	how.text = HOW
	_how = how
	how.theme_type_variation = "HintLabel"
	how.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(how)
	var tools := [["Randomize", _randomize, "Every slider and proportion at random"], ["Reset shape", _reset, "Every slider and proportion back to 0"]]
	for item in tools:
		var b := Button.new()
		b.text = item[0]
		b.tooltip_text = item[2]
		b.pressed.connect(item[1])
		top.add_child(b)
	over.add_child(_pad_ignore(top))
	var mid := Control.new()
	mid.size_flags_vertical = Control.SIZE_EXPAND_FILL
	mid.mouse_filter = Control.MOUSE_FILTER_IGNORE
	over.add_child(mid)
	var bottom := HBoxContainer.new()
	bottom.mouse_filter = Control.MOUSE_FILTER_IGNORE
	bottom.add_theme_constant_override("separation", 12)
	# The triangle budget (20 §7.2): a slim meter, gold to the target, orange past it.
	var meter_box := PanelContainer.new()
	meter_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
	meter_box.size_flags_vertical = Control.SIZE_SHRINK_END
	meter_box.add_theme_stylebox_override("panel", UI._box(Color(0.035, 0.045, 0.06, 0.82), Color(UI.RIM, 0.6), 1, 6, Vector4(12, 8, 12, 10)))
	var meter := VBoxContainer.new()
	meter.mouse_filter = Control.MOUSE_FILTER_IGNORE
	meter.add_theme_constant_override("separation", 5)
	meter_box.add_child(meter)
	_meter = Label.new()
	_meter.theme_type_variation = "HintLabel"
	meter.add_child(_meter)
	_meter_bar = ProgressBar.new()
	_meter_bar.custom_minimum_size = Vector2(220, 6)
	_meter_bar.show_percentage = false
	_meter_bar.max_value = 14000
	_meter_bar.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_meter_bar.add_theme_stylebox_override("background", UI._box(Color(0, 0, 0, 0.45), Color(0, 0, 0, 0), 0, 3, Vector4(0, 0, 0, 0)))
	meter.add_child(_meter_bar)
	bottom.add_child(meter_box)
	var push := Control.new()
	push.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	push.mouse_filter = Control.MOUSE_FILTER_IGNORE
	bottom.add_child(push)
	_toast = PanelContainer.new()
	_toast.visible = false
	_toast.size_flags_vertical = Control.SIZE_SHRINK_END
	_toast_label = Label.new()
	_toast_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_toast_label.custom_minimum_size = Vector2(420, 0)
	_toast.add_child(_toast_label)
	bottom.add_child(_toast)
	over.add_child(_pad_ignore(bottom))
	return over


func _pad_ignore(c: Control) -> MarginContainer:
	var mc := _pad(c, 16)
	mc.mouse_filter = Control.MOUSE_FILTER_IGNORE
	return mc


## A message over the stage: `kind` "error", "ok" or "note"; "" hides it.
func _toast_show(text: String, kind := "note") -> void:
	if _toast == null:
		return
	_toast.visible = text != ""
	if text == "":
		return
	var tint: Color = {"error": Color(0.95, 0.45, 0.38), "ok": Color(0.55, 0.88, 0.55)}.get(kind, UI.GOLD)
	var sb := UI._box(Color(0.035, 0.045, 0.06, 0.94), tint, 1, 6, Vector4(14, 10, 14, 10))
	sb.border_width_left = 4
	_toast.add_theme_stylebox_override("panel", sb)
	_toast_label.add_theme_color_override("font_color", UI.TEXT if kind == "note" else tint.lightened(0.25))
	_toast_label.text = text


## The side panel: the step's name and what it's for, then its controls. A fixed width, so the
## stage doesn't jump between steps.
func _build_side() -> Control:
	var box := PanelContainer.new()
	box.custom_minimum_size = Vector2(404, 0)
	var outer := VBoxContainer.new()
	outer.add_theme_constant_override("separation", 4)
	box.add_child(_pad(outer, 18))
	_step_label = Label.new()
	_step_label.theme_type_variation = "HeaderLabel"
	outer.add_child(_step_label)
	_title = Label.new()
	_title.theme_type_variation = "TitleLabel"
	outer.add_child(_title)
	_hint_label = Label.new()
	_hint_label.theme_type_variation = "HintLabel"
	_hint_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	_hint_label.custom_minimum_size = Vector2(360, 0)
	outer.add_child(_hint_label)
	outer.add_child(HSeparator.new())
	var scroll := ScrollContainer.new()
	_scroll = scroll
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	outer.add_child(scroll)
	_panel = VBoxContainer.new()
	_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	_panel.add_theme_constant_override("separation", 8)
	# Room for the scrollbar.
	var inner := MarginContainer.new()
	inner.add_theme_constant_override("margin_right", 10)
	inner.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	inner.add_child(_panel)
	scroll.add_child(inner)
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
	var at: int = STEPS.find(info) + 1
	_step_label.text = "STEP %d OF %d" % [at, STEPS.size()]
	_title.text = info[1].substr(3)
	_hint_label.text = info[2]
	_update_header()
	match id:
		"build":
			for b in creator.builds():
				_choice(BUILD_NAMES.get(String(b[0]), String(b[0]).capitalize()), creator.build() == b[0], func():
					creator.set_build(b[0])
					_changed(true)
					_show_step("build"), BUILD_INFO.get(String(b[0]), ""))
		"gender":
			if creator.gender() == "":
				_title.text = "Body colors"
				_hint_label.text = "This body has no genders: give it its own colors."
			else:
				var pair := HBoxContainer.new()
				pair.add_theme_constant_override("separation", 8)
				_panel.add_child(pair)
				for g in [["masculine", "Masculine"], ["feminine", "Feminine"]]:
					var card := _choice(g[1], creator.gender() == g[0], func():
						creator.set_gender(g[0])
						_changed(true)
						_show_step("gender"))
					card.reparent(pair)
					card.size_flags_horizontal = Control.SIZE_EXPAND_FILL
				_gap(6)
			if creator.has_skin():
				_skin_picker()
			else:
				# No skin to tone (the beast's hide, the machine's hull): its own colors instead.
				_fill_picker("body")
				_hint("More colors, the brush and decals are in the Paint step.")
		"face":
			_sculpt_tools()
			_face_shapes()
			_eyes_panel()
			_brows_panel()
			_hair_panel()
			var have: Array = Array(creator.sliders())
			var listed := {}
			for group in FACE_GROUPS:
				var rows: Array = group[1].filter(func(r): return have.has(r[0]))
				if rows.is_empty():
					continue
				_section(group[0])
				for r in rows:
					listed[r[0]] = true
					var id2: String = r[0]
					_slider(r[1], creator.slider(id2), func(v): creator.set_slider(id2, v), r[2], r[3])
			var more: Array = have.filter(func(id2): return _is_face(id2) and not listed.has(id2))
			if not more.is_empty():
				_section("More")
				for id2 in more:
					_slider(_label_of(id2), creator.slider(id2), func(v): creator.set_slider(id2, v))
		"body":
			_sculpt_tools()
			_section("Proportions")
			for p in creator.proportions():
				_slider(PROPORTION_NAMES.get(p, p), creator.proportion(p), func(v): creator.set_proportion(p, v), "smaller", "bigger")
			_section("Shape")
			for id2 in creator.sliders():
				if not _is_face(id2) and not _is_grown(id2):
					_slider(_label_of(id2).trim_prefix("Body "), creator.slider(id2), func(v): creator.set_slider(id2, v), "less", "more")
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
				var row := HBoxContainer.new()
				row.add_theme_constant_override("separation", 10)
				var l := Label.new()
				l.text = String(slot).capitalize()
				l.custom_minimum_size = Vector2(96, 0)
				l.add_theme_color_override("font_color", UI.DIM)
				row.add_child(l)
				var pick := OptionButton.new()
				pick.size_flags_horizontal = Control.SIZE_EXPAND_FILL
				pick.fit_to_longest_item = false
				pick.clip_text = true
				pick.add_item("None")
				var options: Array = by_slot[slot]
				for i in options.size():
					pick.add_item(options[i].name)
					pick.set_item_tooltip(i + 1, "%d triangles" % options[i].triangles)
					if worn.has(options[i].id):
						pick.select(i + 1)
				pick.item_selected.connect(func(i):
					creator.set_part(slot, "" if i == 0 else options[i - 1].id)
					_changed(true))
				row.add_child(pick)
				_panel.add_child(row)
			_gap(4)
			_hint("Colors come next, in Paint: fills, the brush and decals.")
		"paint":
			var targets: PackedStringArray = creator.paint_targets()
			if not targets.has(_paint_target):
				_paint_target = "body"
			var names := {"body": "Body"}
			for p in creator.parts():
				names[p.id] = p.name
			_section("What to paint")
			var tpick := OptionButton.new()
			tpick.fit_to_longest_item = false
			for i in targets.size():
				tpick.add_item(names.get(targets[i], targets[i]))
				if targets[i] == _paint_target:
					tpick.select(i)
			tpick.item_selected.connect(func(i):
				_paint_target = targets[i]
				_paint_region = 0
				_show_step("paint"))
			_panel.add_child(tpick)
			var regions: PackedColorArray = creator.paint_regions(_paint_target)
			var row := HFlowContainer.new()
			row.add_theme_constant_override("h_separation", 6)
			for i in regions.size():
				row.add_child(_swatch(regions[i], _paint_tool == "fill" and i == _paint_region, func():
					_paint_region = i
					_paint_tool = "fill"
					_show_step("paint"), 42))
			_panel.add_child(row)
			_section("Tool")
			_tool_row([["fill", "Fill"], ["brush", "Brush"], ["decal", "Decal"], ["erase", "Erase"]], _paint_tool, func(t):
				_paint_tool = t
				_hold_rest()
				_show_step("paint"))
			if _paint_tool == "fill":
				_hint("Pick one of its colors above, then a new one below.")
			else:
				_hint("Left-drag on the model to paint; right-drag turns it.")
				_slider_raw("Brush size", _brush_size, 0.015, 0.3, func(v): _brush_size = v)
			if _paint_tool == "decal":
				var dpick := OptionButton.new()
				var decals: Array = creator.decals()
				for i in decals.size():
					dpick.add_item(decals[i].label)
					if decals[i].id == _decal:
						dpick.select(i)
				dpick.item_selected.connect(func(i): _decal = decals[i].id)
				_panel.add_child(dpick)
			_section("Palette")
			var grid := GridContainer.new()
			grid.columns = 8
			grid.add_theme_constant_override("h_separation", 6)
			grid.add_theme_constant_override("v_separation", 6)
			var palette: PackedColorArray = creator.palette()
			for k in palette.size():
				grid.add_child(_swatch(palette[k], _paint_tool != "fill" and k == _brush_color, func():
					if _paint_tool == "fill":
						creator.fill(_paint_target, _paint_region, k)
						_changed(true)
					else:
						_brush_color = k
					_show_step("paint")))
			_panel.add_child(grid)
			_section("Signature hue")
			_hint("What glows, and the color of its effects.")
			var hue := HSlider.new()
			hue.min_value = 0
			hue.max_value = 359
			hue.step = 1
			hue.value = creator.signature()
			hue.value_changed.connect(func(v):
				creator.set_signature(int(v))
				_changed(true))
			_panel.add_child(hue)
			_gap(8)
			var clear := Button.new()
			clear.text = "Clear all paint"
			clear.pressed.connect(func():
				creator.clear_paint()
				creator.clear_images()
				_changed(true)
				_show_step("paint"))
			_panel.add_child(clear)
		"moves":
			_section("Moves")
			var grid := GridContainer.new()
			grid.columns = 2
			grid.add_theme_constant_override("h_separation", 10)
			grid.add_theme_constant_override("v_separation", 6)
			_panel.add_child(grid)
			for slot in ["attack", "q", "w", "e", "r"]:
				var l := Label.new()
				l.text = "Attack" if slot == "attack" else slot.to_upper()
				l.custom_minimum_size = Vector2(64, 0)
				l.add_theme_color_override("font_color", UI.GOLD)
				grid.add_child(l)
				var mpick := OptionButton.new()
				mpick.size_flags_horizontal = Control.SIZE_EXPAND_FILL
				mpick.fit_to_longest_item = false
				mpick.clip_text = true
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
				grid.add_child(mpick)
			# The passive (20 §8.1): one from the shared catalog, or none.
			_section("Passive")
			var ppick := OptionButton.new()
			ppick.fit_to_longest_item = false
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
			# The resource (01 §9): costless, or what Q W E R spend.
			_section("Resource")
			var rpick := OptionButton.new()
			rpick.fit_to_longest_item = false
			var resources: Array = creator.resources()
			var on_res: String = creator.resource()
			rpick.add_item("None (costless)")
			for i in resources.size():
				rpick.add_item(resources[i].label)
				if resources[i].id == on_res:
					rpick.select(i + 1)
			rpick.item_selected.connect(func(i):
				creator.set_resource("" if i == 0 else resources[i - 1].id)
				_changed(true)
				_show_step("moves"))
			_panel.add_child(rpick)
			for r in resources:
				if r.id == on_res:
					_hint(r.hint)
			_section("Or take a whole kit")
			var pick := OptionButton.new()
			pick.fit_to_longest_item = false
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
			var summary := creator.kit_summary()
			for i in summary.size():
				var l := Label.new()
				l.text = summary[i]
				l.theme_type_variation = "HintLabel" if i == 0 else ""
				_panel.add_child(l)
		"balance":
			var price: Dictionary = creator.price()
			var total: float = price.total
			var over: bool = total > price.cap
			var past: bool = total > price.budget + 0.5
			var amber := Color(0.95, 0.7, 0.3)
			var head := HBoxContainer.new()
			var big := Label.new()
			big.text = "%.0f" % total
			big.theme_type_variation = "TitleLabel"
			big.add_theme_color_override("font_color", Color(1.0, 0.5, 0.4) if over else (amber if past else UI.CREAM))
			head.add_child(big)
			var of := Label.new()
			of.text = "  of %.0f points%s" % [price.budget, "  ·  over the cap: tune it down" if over else ("  ·  over budget, under the cap" if past else "")]
			of.size_flags_vertical = Control.SIZE_SHRINK_END
			of.add_theme_color_override("font_color", Color(1.0, 0.5, 0.4) if over else UI.DIM)
			head.add_child(of)
			_panel.add_child(head)
			var bar := ProgressBar.new()
			bar.max_value = price.cap
			bar.value = minf(total, price.cap)
			bar.show_percentage = false
			bar.custom_minimum_size = Vector2(0, 10)
			bar.add_theme_stylebox_override("background", UI._box(Color(0.02, 0.025, 0.035), Color(0.24, 0.26, 0.3), 1, 4, Vector4(0, 0, 0, 0)))
			bar.add_theme_stylebox_override("fill", UI._box(Color(0.86, 0.36, 0.3) if over else (amber if past else UI.GOLD), Color(0, 0, 0, 0), 0, 4, Vector4(0, 0, 0, 0)))
			_panel.add_child(bar)
			var parts := GridContainer.new()
			parts.columns = 2
			parts.add_theme_constant_override("v_separation", 2)
			for part in price.parts:
				var pl := Label.new()
				pl.text = String(part[0]).capitalize()
				pl.theme_type_variation = "HintLabel"
				pl.size_flags_horizontal = Control.SIZE_EXPAND_FILL
				parts.add_child(pl)
				var pv := Label.new()
				pv.text = "%.1f" % part[1]
				pv.theme_type_variation = "HintLabel"
				pv.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
				parts.add_child(pv)
			_panel.add_child(parts)
			var fit := Button.new()
			fit.text = "Auto-fit to the budget"
			fit.pressed.connect(func():
				creator.auto_fit()
				_changed(false)
				_show_step("balance"))
			_panel.add_child(fit)
			for slot in ["attack", "q", "w", "e", "r"]:
				var t: PackedFloat32Array = creator.tuning(slot)
				_section("Attack" if slot == "attack" else slot.to_upper())
				_tune_slider("Damage" if slot == "attack" else "Power", t[0], 0.5, 1.6, func(v):
					var now: PackedFloat32Array = creator.tuning(slot)
					creator.tune(slot, v, now[1])
					_changed(false))
				_tune_slider("Time between" if slot == "attack" else "Cooldown", t[1], 0.6, 2.0, func(v):
					var now: PackedFloat32Array = creator.tuning(slot)
					creator.tune(slot, now[0], v)
					_changed(false))
			_section("Stats")
			var stats: PackedFloat32Array = creator.stat_tuning()
			var stat_names := ["Health", "Armor", "Magic resist", "Move speed"]
			for i in 4:
				var lo := 0.94 if i == 3 else 0.8
				var hi := 1.06 if i == 3 else 1.2
				_tune_slider(stat_names[i], stats[i], lo, hi, func(v):
					var now: PackedFloat32Array = creator.stat_tuning()
					now[i] = v
					creator.tune_stats(now)
					_changed(false))
		"name":
			var edit := LineEdit.new()
			edit.text = creator.name()
			edit.placeholder_text = "Name (1–32 characters)"
			edit.add_theme_font_size_override("font_size", 22)
			edit.custom_minimum_size = Vector2(0, 48)
			edit.text_changed.connect(func(t):
				creator.set_name(t)
				_update_header()
				_changed(false))
			_panel.add_child(edit)
			_hint("It shows over the champion in game, and names its file.")
			_gap(8)
			var save := Button.new()
			save.text = "Save champion"
			save.theme_type_variation = "PrimaryButton"
			save.custom_minimum_size = Vector2(0, 44)
			save.pressed.connect(_save)
			_panel.add_child(save)
		_:
			_hint("Not built yet.")
	_place_camera()
	_update_status()


func _is_face(id: String) -> bool:
	return FACE_PREFIXES.any(func(p): return id.begins_with(p))


## The hair's and the beard's own sliders (their length), on the Face step's Hair panel.
func _is_grown(id: String) -> bool:
	return id.begins_with("hair_") or id.begins_with("beard_")


func _label_of(id: String) -> String:
	return id.replace("_", " ").capitalize()


func _hint(text: String) -> void:
	var l := Label.new()
	l.text = text
	l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	l.custom_minimum_size = Vector2(300, 0)
	l.theme_type_variation = "HintLabel"
	_panel.add_child(l)


## A card to pick: its name, and a line on what it is (`about`). Gold-edged when chosen.
func _choice(text: String, on: bool, act: Callable, about := "") -> Button:
	var b := Button.new()
	b.toggle_mode = true
	b.button_pressed = on
	b.focus_mode = Control.FOCUS_NONE
	var m := Vector4(14, 10, 12, 10)
	var chosen := UI._box(UI.SLATE_HI, UI.GOLD, 1, 6, m)
	chosen.border_width_left = 4
	b.add_theme_stylebox_override("normal", UI._box(UI.INK_2, Color(0.2, 0.22, 0.26), 1, 6, m))
	b.add_theme_stylebox_override("hover", UI._box(UI.SLATE, UI.GOLD, 1, 6, m))
	b.add_theme_stylebox_override("pressed", chosen)
	b.add_theme_stylebox_override("hover_pressed", chosen)
	if about == "":
		b.text = text
		b.custom_minimum_size = Vector2(0, 44)
	else:
		# Two lines inside the card; they let the clicks through to it.
		var v := VBoxContainer.new()
		v.set_anchors_preset(Control.PRESET_FULL_RECT)
		v.mouse_filter = Control.MOUSE_FILTER_IGNORE
		v.add_theme_constant_override("separation", 2)
		var name_l := Label.new()
		name_l.text = text
		name_l.add_theme_color_override("font_color", UI.CREAM if on else UI.TEXT)
		name_l.mouse_filter = Control.MOUSE_FILTER_IGNORE
		var about_l := Label.new()
		about_l.text = about
		about_l.theme_type_variation = "HintLabel"
		about_l.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		about_l.mouse_filter = Control.MOUSE_FILTER_IGNORE
		v.add_child(name_l)
		v.add_child(about_l)
		b.add_child(_pad_ignore_n(v, 12))
		b.custom_minimum_size = Vector2(0, 84)
	b.pressed.connect(func():
		act.call()
		_show_step(step))
	_panel.add_child(b)
	return b


func _pad_ignore_n(c: Control, m: int) -> MarginContainer:
	var mc := _pad(c, m)
	mc.mouse_filter = Control.MOUSE_FILTER_IGNORE
	mc.set_anchors_preset(Control.PRESET_FULL_RECT)
	return mc


## The eyes (22 §2.2): the iris's color (and the right eye's, for heterochromia), its size, the
## pupil's, the iris's kind and the sclera.
func _eyes_panel() -> void:
	if not creator.has_eyes():
		return
	var e: Dictionary = creator.eyes()
	var colors: PackedColorArray = creator.eye_colors()
	var names: PackedStringArray = creator.eye_color_names()
	_section("Eyes")
	var odd: bool = int(e.right) >= 0
	for which in (["iris", "right"] if odd else ["iris"]):
		if odd:
			var l := Label.new()
			l.text = "Left eye" if which == "iris" else "Right eye"
			l.add_theme_color_override("font_color", UI.DIM)
			_panel.add_child(l)
		var row := HFlowContainer.new()
		row.add_theme_constant_override("h_separation", 5)
		row.add_theme_constant_override("v_separation", 5)
		var now: int = int(e[which])
		for i in colors.size():
			var sw := _swatch(colors[i], i == now, func():
				creator.set_eye(which, i)
				_changed(true)
				_show_step("face"), 30)
			sw.tooltip_text = names[i]
			row.add_child(sw)
		_panel.add_child(row)
	var odd_box := CheckBox.new()
	odd_box.text = "Each eye its own color"
	odd_box.button_pressed = odd
	odd_box.toggled.connect(func(on):
		creator.set_eye("right", int(e.iris) if on else -1)
		_changed(true)
		_show_step("face"))
	_panel.add_child(odd_box)
	# Size 0–9 (4 is natural), pupil 0–10 (5): the sliders' middle is the natural eye.
	_slider("Iris size", (float(e.size) - 4.0) / (4.0 if int(e.size) < 4 else 5.0), func(v):
		creator.set_eye("size", int(round(4.0 + v * (4.0 if v < 0.0 else 5.0)))), "small", "large")
	_slider("Pupil", (float(e.pupil) - 5.0) / 5.0, func(v):
		creator.set_eye("pupil", int(round(5.0 + v * 5.0))), "pinpoint", "wide")
	_tool_row([["round", "Round"], ["slit", "Slit"], ["glow", "Glowing"], ["blank", "Blank"]], String(e.style), func(k):
		creator.set_eye("style", k)
		_changed(true)
		_show_step("face"))
	_tool_row([["white", "White"], ["yellowed", "Yellowed"], ["dark", "Dark"]], String(e.sclera), func(k):
		creator.set_eye("sclera", k)
		_changed(true)
		_show_step("face"))


## The Brows panel (22 §3.1): a style, a color, and the brow's sizes; each brow its own with the
## box ticked.
const BROW_SLIDERS := [["thick", "Thickness", "thin", "thick"], ["length", "Length", "short", "long"],
	["arch", "Arch", "flat", "high"], ["peak", "Peak", "toward the nose", "toward the temple"],
	["tilt", "Tilt", "tips down", "tips up"], ["gap", "Gap", "close", "apart"], ["raise", "Height", "low", "high"]]


func _brows_panel() -> void:
	if not creator.has_brows():
		return
	var b: Dictionary = creator.brows()
	_section("Brows")
	var own: bool = b.own
	if own:
		_tool_row([["left", "Left brow"], ["right", "Right brow"]], _brow_side, func(k):
			_brow_side = k
			_show_step("face"))
	var side: String = _brow_side if own else "left"
	var w: Dictionary = b[side]
	var names: PackedStringArray = creator.brow_styles()
	var styles: Array = []
	for i in names.size():
		styles.append([str(i), names[i]])
	_tool_row(styles, str(int(w.style)), func(k):
		creator.set_brow(side, "style", int(k))
		_changed(true)
		_show_step("face"))
	var row := HFlowContainer.new()
	row.add_theme_constant_override("h_separation", 5)
	row.add_theme_constant_override("v_separation", 5)
	var pal: PackedColorArray = creator.palette()
	var now: int = creator.brow_color()
	for i in range(0, mini(24, pal.size())):
		var sw := _swatch(pal[i], i == now, func():
			creator.set_brow_color(i)
			# The brows and the beard share a color (22 §7 item 5), and the hair too unless
			# it has its own.
			creator.set_hair_color("beard", i)
			if not _hair_own:
				creator.set_hair_color("hair", i)
			_changed(true)
			_show_step("face"), 26)
		row.add_child(sw)
	_panel.add_child(row)
	for s in BROW_SLIDERS:
		var key: String = s[0]
		_slider(s[1], (float(w[key]) - 5.0) / 5.0, func(v):
			creator.set_brow(side, key, int(round(5.0 + v * 5.0))), s[2], s[3])
	var own_box := CheckBox.new()
	own_box.text = "Each brow its own shape"
	own_box.button_pressed = own
	own_box.toggled.connect(func(on):
		creator.set_brows_own(on)
		_brow_side = "left"
		_changed(true)
		_show_step("face"))
	_panel.add_child(own_box)


## The Hair panel (22 §7 item 5): the hairstyle and the beard, each with its length where it has
## one, and their color, which the brows share unless the hair has its own.
func _hair_panel() -> void:
	var by_slot := {"hair": [], "beard": []}
	for p in creator.parts():
		if by_slot.has(p.slot):
			by_slot[p.slot].append(p)
	if by_slot.hair.is_empty() and by_slot.beard.is_empty():
		return
	var worn: PackedStringArray = creator.outfit()
	var have: Array = Array(creator.sliders())
	# A loaded recipe whose hair and brows differ has the hair's own color.
	var hc: int = creator.hair_color("hair")
	if hc >= 0 and creator.brow_color() >= 0 and hc != creator.brow_color():
		_hair_own = true
	for slot in ["hair", "beard"]:
		var opts: Array = by_slot[slot]
		if opts.is_empty():
			continue
		_section("Hair" if slot == "hair" else "Beard")
		var cur := ""
		var tools := [["", "None"]]
		for p in opts:
			tools.append([p.id, p.name])
			if worn.has(p.id):
				cur = p.id
		_tool_row(tools, cur, func(k):
			creator.set_part(slot, k)
			_changed(true)
			_show_step("face"))
		var len_id: String = slot + "_length"
		if cur != "" and have.has(len_id) and creator.has_length(cur):
			_slider("Length", creator.slider(len_id), func(v): creator.set_slider(len_id, v), "shorter", "longer")
	var shown := "hair" if creator.hair_color("hair") >= 0 or not by_slot.hair.is_empty() else "beard"
	var now: int = creator.hair_color(shown)
	if now < 0:
		now = creator.brow_color()
	_section("Hair color")
	var row := HFlowContainer.new()
	row.add_theme_constant_override("h_separation", 5)
	row.add_theme_constant_override("v_separation", 5)
	var pal: PackedColorArray = creator.palette()
	for i in range(0, mini(24, pal.size())):
		row.add_child(_swatch(pal[i], i == now, func():
			creator.set_hair_color("hair", i)
			if not _hair_own:
				creator.set_hair_color("beard", i)
				creator.set_brow_color(i)
			_changed(true)
			_show_step("face"), 26))
	_panel.add_child(row)
	var own_box := CheckBox.new()
	own_box.text = "Hair its own color (apart from the brows and beard)"
	own_box.button_pressed = _hair_own
	own_box.toggled.connect(func(on):
		_hair_own = on
		if not on:
			# Back to one color: the brows' and the beard's take the hair's.
			var c: int = creator.hair_color("hair")
			if c >= 0:
				creator.set_brow_color(c)
				creator.set_hair_color("beard", c)
			_changed(true)
		_show_step("face"))
	_panel.add_child(own_box)


## Scroll the side panel this far down (captures: `--creator-scroll`).
func scroll_panel(px: int) -> void:
	if _scroll != null:
		_scroll.scroll_vertical = px


## A heading over a group of sliders, with air above it.
func _section(text: String) -> void:
	if _panel.get_child_count() > 0:
		_gap(6)
	var l := Label.new()
	l.text = text.to_upper()
	l.theme_type_variation = "HeaderLabel"
	_panel.add_child(l)


## A -1..1 slider: its name and value on one line (the value says which end, and a click on it
## puts the slider back to 0), the slider under it. `low` and `high` name the two ends.
func _slider(text: String, value: float, set_value: Callable, low := "", high := "") -> void:
	var box := VBoxContainer.new()
	box.add_theme_constant_override("separation", 0)
	var top := HBoxContainer.new()
	var l := Label.new()
	l.text = text
	l.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	top.add_child(l)
	var val := Button.new()
	val.flat = true
	val.focus_mode = Control.FOCUS_NONE
	val.tooltip_text = "Back to the middle"
	val.add_theme_font_size_override("font_size", 13)
	val.add_theme_stylebox_override("normal", StyleBoxEmpty.new())
	val.add_theme_stylebox_override("hover", StyleBoxEmpty.new())
	val.add_theme_stylebox_override("pressed", StyleBoxEmpty.new())
	top.add_child(val)
	box.add_child(top)
	var s := HSlider.new()
	# A one-sided slider (no low end: a fold, pointed ears) runs from none to full.
	s.min_value = 0.0 if low == "" and high != "" else -1.0
	s.max_value = 1.0
	s.step = 0.01
	s.value = value
	s.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	if s.min_value == 0.0:
		val.tooltip_text = "Back to none"
	# Both ends are a choice: no fill from the left, the knob says where it is.
	s.add_theme_stylebox_override("grabber_area", StyleBoxEmpty.new())
	s.add_theme_stylebox_override("grabber_area_highlight", StyleBoxEmpty.new())
	if low != "" or high != "":
		s.tooltip_text = "%s  ←  →  %s" % [low if low != "" else "none", high]
	var show := func(v: float):
		var off := absf(v) > 0.005
		var side: String = (high if v > 0.0 else low) if off else ""
		val.text = ("%s  %+.2f" % [side, v]).strip_edges() if off else "—"
		val.add_theme_color_override("font_color", UI.GOLD if off else UI.DIM)
		val.add_theme_color_override("font_hover_color", UI.CREAM if off else UI.TEXT)
	show.call(value)
	s.value_changed.connect(func(v):
		set_value.call(v)
		show.call(v)
		_changed(true))
	val.pressed.connect(func(): s.value = 0.0)
	box.add_child(s)
	_panel.add_child(box)


func _swatch(color: Color, selected: bool, act: Callable, size := 34) -> Button:
	var b := Button.new()
	b.custom_minimum_size = Vector2(size, size)
	b.focus_mode = Control.FOCUS_NONE
	var normal := UI._box(color, UI.CREAM if selected else Color(0, 0, 0, 0.5), 2 if selected else 1, 5, Vector4(0, 0, 0, 0))
	var hover := UI._box(color.lightened(0.08), UI.CREAM if selected else UI.GOLD, 2, 5, Vector4(0, 0, 0, 0))
	b.add_theme_stylebox_override("normal", normal)
	b.add_theme_stylebox_override("hover", hover)
	b.add_theme_stylebox_override("pressed", normal)
	b.pressed.connect(act)
	return b


## A row of tool toggles that wraps (the panel keeps its width).
func _tool_row(tools: Array, current: String, pick: Callable) -> void:
	var row := HFlowContainer.new()
	row.add_theme_constant_override("h_separation", 4)
	row.add_theme_constant_override("v_separation", 4)
	for t in tools:
		var b := Button.new()
		b.text = t[1]
		b.toggle_mode = true
		b.focus_mode = Control.FOCUS_NONE
		b.button_pressed = current == t[0]
		b.pressed.connect(func(): pick.call(t[0]))
		row.add_child(b)
	_panel.add_child(row)


func _tune_slider(text: String, value: float, lo: float, hi: float, set_value: Callable) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
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
	s.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	var v := Label.new()
	v.custom_minimum_size = Vector2(52, 0)
	v.horizontal_alignment = HORIZONTAL_ALIGNMENT_RIGHT
	var show := func(x: float):
		v.text = "×%.2f" % x
		v.add_theme_color_override("font_color", UI.DIM if absf(x - 1.0) < 0.005 else UI.GOLD)
	show.call(value)
	s.value_changed.connect(func(x):
		show.call(x)
		set_value.call(x))
	s.drag_ended.connect(func(_changed_value): _show_step(step))
	row.add_child(s)
	row.add_child(v)
	_panel.add_child(row)


## `target`'s colors (its paint regions) and the palette: pick a region, then a color fills it.
func _fill_picker(target: String) -> void:
	var regions: PackedColorArray = creator.paint_regions(target)
	if _paint_target != target or _paint_region >= regions.size():
		_paint_target = target
		_paint_region = 0
	_section("Its colors")
	_hint("Pick one, then a new color for it.")
	var row := HFlowContainer.new()
	row.add_theme_constant_override("h_separation", 6)
	for i in regions.size():
		row.add_child(_swatch(regions[i], i == _paint_region, func():
			_paint_region = i
			_show_step(step), 42))
	_panel.add_child(row)
	_gap(4)
	var grid := GridContainer.new()
	grid.columns = 8
	grid.add_theme_constant_override("h_separation", 6)
	grid.add_theme_constant_override("v_separation", 6)
	var palette: PackedColorArray = creator.palette()
	for k in palette.size():
		grid.add_child(_swatch(palette[k], false, func():
			creator.fill(target, _paint_region, k)
			_changed(true)
			_show_step(step)))
	_panel.add_child(grid)


func _skin_picker() -> void:
	_section("Skin")
	var grid := GridContainer.new()
	grid.columns = 8
	grid.add_theme_constant_override("h_separation", 6)
	grid.add_theme_constant_override("v_separation", 6)
	var tones: PackedColorArray = creator.skin_tones()
	for i in tones.size():
		grid.add_child(_swatch(tones[i], creator.skin() == i, func():
			creator.set_skin(i)
			_changed(true)
			_show_step(step)))
	_panel.add_child(grid)
	_hint("The first eight are natural tones; the rest are for the uncanny.")


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
	if _animator != null and _skeleton != null and not _brushing():
		_animator.drive(_skeleton, {}, 0.0, delta)
	if _life != null and _skeleton != null and _camera != null and _model_node != null:
		# The eyes find the camera: the creator's champion looks at the player.
		var to_skeleton := _skeleton.global_transform.affine_inverse()
		_life.tick(_skeleton, delta, to_skeleton * _camera.global_position)


## Hold the eyes still for a capture (`--creator-life`): blink 0–1, yaw, pitch.
func hold_eyes(pose: Vector3) -> void:
	_life_hold = pose
	if _life != null:
		_life.fixed = pose


## Painting or sculpting on the model (CR5b, CR9): it holds still in its rest pose, where the
## brush aims.
func _brushing() -> bool:
	return (step == "paint" and _paint_tool != "fill") or _sculpting() or _featuring()


## The tool in use on this step: "features" (dragging the face's features or the body's masses:
## the default where the base has the bones for it), a brush, or "" (the Body step's sliders
## alone, the model in motion).
func _tool_now() -> String:
	if _sculpt_tool == "" or _sculpt_tool == "features":
		return "features" if _can_feature() else ""
	if _sculpt_tool == "sliders":
		return "" if step == "body" else "features"
	return _sculpt_tool


## Whether this step's model can be dragged: the face always, the body on a base with the
## helper bones (21 §9: the humanoids).
func _can_feature() -> bool:
	return step == "face" or (step == "body" and creator.body_rigged())


func _sculpting() -> bool:
	var t := _tool_now()
	return (step == "face" or step == "body") and t != "" and t != "features"


## Dragging the face's features (FR4, 21 §3) or the body's masses (FR6, 21 §9): what lights up
## under the cursor is what moves.
func _featuring() -> bool:
	return (step == "face" or step == "body") and _tool_now() == "features"


## The creator's method for this step's drag: `face_begin` on the Face step, `body_begin` on the
## Body step.
func _feat(method: String) -> String:
	return ("body_" if step == "body" else "face_") + method


## The Face step's tools (Features, or a brush) and the Body step's (sliders or a brush), above
## the sliders; a brush has size, strength and symmetry.
func _sculpt_tools() -> void:
	var tools := [["push", "Push"], ["pull", "Pull"], ["grab", "Grab"], ["inflate", "Inflate"], ["smooth", "Smooth"], ["flatten", "Flat"]]
	if step == "body":
		# Sliders alone leave the model in motion; a drag or a brush holds it still.
		tools.push_front(["sliders", "In motion" if _can_feature() else "Sliders"])
	if _can_feature():
		tools.push_front(["features", "Features" if step == "face" else "Drag"])
	_tool_row(tools, "sliders" if _tool_now() == "" else _tool_now(), func(t):
			_sculpt_tool = t
			_hold_rest()
			_show_step(step))
	if _tool_now() == "features":
		if step == "face":
			_hint("Point at the face: what lights up is what moves. Drag it; right-drag turns it; Shift-drag or the wheel sizes it (Alt: only across). Drag off the face to turn the model.")
		else:
			_hint("Point at the body: what lights up is what moves. Drag it; Shift-drag or the wheel sizes it (Alt: only across). Right-drag, or a drag off the body, turns the model.")
		var row := HBoxContainer.new()
		row.add_theme_constant_override("separation", 6)
		var mirror := CheckBox.new()
		mirror.text = "Symmetry"
		mirror.button_pressed = _sculpt_mirror
		mirror.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		mirror.toggled.connect(func(on): _sculpt_mirror = on)
		row.add_child(mirror)
		var undo := Button.new()
		undo.text = "Undo"
		undo.tooltip_text = "Back one drag (Ctrl+Z)"
		undo.disabled = not creator.call(_feat("can_undo"))
		undo.pressed.connect(_face_undo)
		row.add_child(undo)
		var clear := Button.new()
		clear.text = "Reset features" if step == "face" else "Reset drags"
		clear.tooltip_text = "Every feature back where the sliders put it" if step == "face" else "Every part of the body back where the sliders put it"
		clear.disabled = not creator.call(_feat("edited"))
		clear.pressed.connect(func():
			creator.call(_feat("clear"))
			_changed(true)
			_show_step(step))
		row.add_child(clear)
		_panel.add_child(row)
		_panel.add_child(HSeparator.new())
		return
	if _tool_now() == "":
		return
	_hint("Sculpt past the sliders: left-drag on the model, right-drag turns it. Parts on it follow.")
	_slider_raw("Size", _sculpt_size, 0.015, 0.25, func(v): _sculpt_size = v)
	_slider_raw("Strength", _sculpt_strength, 0.001, 0.015, func(v): _sculpt_strength = v)
	var row := HBoxContainer.new()
	var mirror := CheckBox.new()
	mirror.text = "Symmetry"
	mirror.button_pressed = _sculpt_mirror
	mirror.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	mirror.toggled.connect(func(on): _sculpt_mirror = on)
	row.add_child(mirror)
	var reset := Button.new()
	reset.text = "Undo all sculpting"
	reset.pressed.connect(func():
		creator.clear_sculpt()
		_changed(true))
	row.add_child(reset)
	_panel.add_child(row)
	_panel.add_child(HSeparator.new())


## A plain slider from `lo` to `hi`, its name to the left (brush and sculpt sizes).
func _slider_raw(label: String, value: float, lo: float, hi: float, on_change: Callable) -> void:
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 10)
	var l := Label.new()
	l.text = label
	l.custom_minimum_size = Vector2(96, 0)
	l.add_theme_color_override("font_color", UI.DIM)
	row.add_child(l)
	var s := HSlider.new()
	s.min_value = lo
	s.max_value = hi
	s.step = (hi - lo) / 100.0
	s.value = value
	s.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	s.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	s.value_changed.connect(on_change)
	row.add_child(s)
	_panel.add_child(row)


## Sculpt where the stage's `pos` looks at the model; `drag` is the mouse's move (for Grab).
func _sculpt_at(pos: Vector2, drag: Vector2) -> void:
	if _model_node == null or _camera == null:
		return
	var inv := _model_node.global_transform.affine_inverse()
	var origin := inv * _camera.project_ray_origin(pos)
	var dir := (inv.basis * _camera.project_ray_normal(pos)).normalized()
	# A pixel of drag is about this far on the model at the camera's distance.
	var per_px := 2.0 * _camera.global_position.distance_to(_model_node.global_position) * tan(deg_to_rad(_camera.fov * 0.5)) / float(_viewport.size.y)
	var pull := inv.basis * (_camera.global_basis.x * drag.x - _camera.global_basis.y * drag.y) * per_px
	if creator.sculpt(origin, dir, _tool_now(), _sculpt_size, _sculpt_strength, pull, _sculpt_mirror):
		_changed(true)


func _hold_rest() -> void:
	if _brushing() and _skeleton != null:
		_skeleton.reset_bone_poses()


## A stroke with `tool` at a point of the stage given as fractions of its size (`--creator-brush`).
## A sculpt stroke at a point of the stage given as fractions of its size, repeated `times`
## (`--creator-sculpt`, for screenshots).
func sculpt_at_fraction(tool: String, f: Vector2, times := 6) -> void:
	_sculpt_tool = tool
	_show_step(step)
	_hold_rest()
	for i in times:
		_sculpt_at(f * Vector2(_viewport.size), Vector2(0, -6) if tool == "grab" else Vector2.ZERO)


func paint_at_fraction(tool: String, f: Vector2, color := -1) -> void:
	_paint_tool = tool
	if color >= 0:
		_brush_color = color
	_show_step("paint")
	_hold_rest()
	_paint_at(f * Vector2(_viewport.size))


## Paint where the stage's `pos` looks at the model.
func _paint_at(pos: Vector2) -> void:
	if _model_node == null or _camera == null:
		return
	var inv := _model_node.global_transform.affine_inverse()
	var origin := inv * _camera.project_ray_origin(pos)
	var dir := (inv.basis * _camera.project_ray_normal(pos)).normalized()
	var color := -1 if _paint_tool == "erase" else _brush_color
	var decal := _decal if _paint_tool == "decal" else ""
	if creator.brush(origin, dir, _brush_size, color, decal) != "":
		_changed(true)
	else:
		_update_status()  # a full image says so


func _rebuild() -> void:
	var m: MftrModel = creator.preview()
	if m == null:
		# The old model stays up; the status says why this one can't be made.
		_update_status()
		return
	if _model_node != null:
		_model_node.queue_free()
	_model = m
	_model_node = m.instantiate()
	_mats.clear()
	_hover_region = -1
	var mesh: MeshInstance3D = _model_node.get_node("Skeleton/Mesh")
	var slots: PackedStringArray = m.surface_slots()
	for i in slots.size():
		var mat := ShaderMaterial.new()
		mat.shader = load("res://shaders/champion_model.gdshader")
		mat.set_shader_parameter("slot", SLOTS.find(slots[i]))
		mat.set_shader_parameter("team_accent", TEAM)
		var paint: ImageTexture = m.surface_paint(i)
		if paint != null:
			mat.set_shader_parameter("paint", paint)
			mat.set_shader_parameter("has_paint", true)
		mesh.set_surface_override_material(i, mat)
		_mats.append(mat)
	_pivot.add_child(_model_node)
	_update_status()
	_skeleton = _model_node.get_node("Skeleton")
	_animator = m.new_animator()
	_life = FaceLife.new()
	if not _life.setup(_skeleton):
		_life = null
	elif _life_hold != null:
		_life.fixed = _life_hold
	_hold_rest()
	_pivot.rotation.y = _yaw
	var head := _skeleton.find_bone("head")
	if head >= 0:
		_head_y = _skeleton.get_bone_global_rest(head).origin.y + 0.13
		_top_y = _head_y + 0.15
		# The whole figure, horns and staff too (capped: a banner pole shouldn't shrink it).
		var mesh_top: float = (_model_node.get_node("Skeleton/Mesh") as MeshInstance3D).get_aabb().end.y
		_top_y = clampf(mesh_top, _top_y, _top_y + 0.35)
	_place_camera()


func _place_camera() -> void:
	if _camera == null:
		return
	# The gender step frames the face; on a creature it's the colors step, so the whole body.
	var face := step == "face" or (step == "gender" and creator.gender() != "")
	var target := Vector3(0, _head_y - 0.04, 0) if face else Vector3(0, _top_y * 0.52, 0)
	var dist := (2.4 if face else _top_y * 2.3) / _zoom
	_camera.look_at_from_position(target + Vector3(0, 0.12 if face else 0.4, dist), target)


func _stage_input(event: InputEvent) -> void:
	if _featuring():
		_feature_input(event)
		return
	if _brushing():
		# Left paints (a decal once a click, a brush along the drag); right turns the model.
		if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_LEFT:
			_painting = event.pressed
			if event.pressed and _sculpting():
				if _tool_now() != "grab":
					_sculpt_at(event.position, Vector2.ZERO)
			elif event.pressed:
				_paint_at(event.position)
			return
		if event is InputEventMouseButton and event.button_index == MOUSE_BUTTON_RIGHT:
			_dragging = event.pressed
			return
		if event is InputEventMouseMotion and _painting and _sculpting():
			_sculpt_at(event.position - event.relative, event.relative if _tool_now() == "grab" else Vector2.ZERO)
			return
		if event is InputEventMouseMotion and _painting and _paint_tool != "decal":
			_paint_at(event.position)
			return
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


## The Face step with Features (FR4, 21 §3.1) and the Body step with Drag (FR6, 21 §9): hover
## lights a region; a drag on it moves, turns (the face only: the body's masses don't turn) or
## sizes it, posing the bones at once; the preview is forged when the drag ends. A drag off the
## model turns it; the wheel off it zooms.
func _feature_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var motion := event as InputEventMouseMotion
		if _face_drag != "":
			creator.call(_feat("drag"), motion.relative, _camera_frame(), _skeleton)
		elif _dragging:
			_yaw += motion.relative.x * 0.01
			_pivot.rotation.y = _yaw
		else:
			_set_hover(_region_at(motion.position))
		return
	if not (event is InputEventMouseButton):
		return
	var mb := event as InputEventMouseButton
	if mb.button_index == MOUSE_BUTTON_LEFT or mb.button_index == MOUSE_BUTTON_RIGHT:
		if mb.pressed:
			var region := _region_at(mb.position)
			var mode := "move" if mb.button_index == MOUSE_BUTTON_LEFT else "turn"
			if mb.shift_pressed:
				mode = "size_width" if mb.alt_pressed else "size"
			if step == "body" and mode == "turn":
				region = -1  # a right-drag on the body turns the model
			if region >= 0 and creator.call(_feat("begin"), mode, _sculpt_mirror):
				_face_drag = mode
				_set_active(region)
			else:
				_dragging = true
		else:
			if _face_drag != "":
				_face_drag = ""
				_set_active(-1)
				if creator.call(_feat("end")):
					_changed(true)
					_show_step(step)
			_dragging = false
	elif mb.pressed and (mb.button_index == MOUSE_BUTTON_WHEEL_UP or mb.button_index == MOUSE_BUTTON_WHEEL_DOWN):
		var up := mb.button_index == MOUSE_BUTTON_WHEEL_UP
		if _region_at(mb.position) >= 0 and creator.call(_feat("begin"), "size_width" if mb.alt_pressed else "size", _sculpt_mirror):
			creator.call(_feat("drag"), Vector2(0, -6.0 if up else 6.0), _camera_frame(), _skeleton)
			if creator.call(_feat("end")):
				_changed(true)
		else:
			_zoom = clampf(_zoom * (1.1 if up else 1.0 / 1.1), 0.5, 3.0)
			_place_camera()


## The region of this step (the face's, the body's) under the stage's `pos` (-1 for none), as
## the creator remembers it.
func _region_at(pos: Vector2) -> int:
	if _model_node == null or _camera == null:
		return -1
	var inv := _model_node.global_transform.affine_inverse()
	var origin := inv * _camera.project_ray_origin(pos)
	var dir := (inv.basis * _camera.project_ray_normal(pos)).normalized()
	return creator.call(_feat("hover"), origin, dir)


func _set_hover(region: int) -> void:
	if region == _hover_region:
		return
	_hover_region = region
	var mirror := creator.region_mirror(region) if region >= 0 and _sculpt_mirror else -1
	for mat in _mats:
		mat.set_shader_parameter("hover_region", region)
		mat.set_shader_parameter("mirror_region", mirror if mirror != region else -1)
	if _how != null:
		var ways := "drag to move · right-drag to turn · Shift-drag or wheel to size" if step == "face" else "drag to move · Shift-drag or wheel to size"
		_how.text = HOW if region < 0 else "%s   ·   %s" % [creator.region_name(region), ways]
		_how.add_theme_color_override("font_color", UI.DIM if region < 0 else UI.CREAM)


func _set_active(region: int) -> void:
	for mat in _mats:
		mat.set_shader_parameter("active_region", region)
		mat.set_shader_parameter("highlight", creator.signature_color().lightened(0.35) if region >= 0 else Color(1.0, 0.85, 0.55))


func _face_undo() -> void:
	if creator.call(_feat("undo")):
		_changed(true)
		_show_step(step)


## The stage camera in the preview's model space, for turning pixels into a feature's move.
func _camera_frame() -> Dictionary:
	var inv := _model_node.global_transform.affine_inverse()
	var t := _camera.global_transform
	return {
		"origin": inv * t.origin,
		"right": (inv.basis * t.basis.x).normalized(),
		"up": (inv.basis * t.basis.y).normalized(),
		"forward": (inv.basis * -t.basis.z).normalized(),
		"fov_y": deg_to_rad(_camera.fov),
		"viewport_h": float(_viewport.size.y),
	}


## Hover a feature and drag it from one point of the stage to another (fractions of its size),
## in `steps` moves (`--creator-feature`, for screenshots and checks). With `hold`, the drag
## stays open, its region glowing.
func feature_at_fraction(mode: String, from: Vector2, to: Vector2, steps := 8, hold := false, on := "face") -> void:
	_sculpt_tool = "features"
	_show_step(on)
	_hold_rest()
	var size := Vector2(_viewport.size)
	var region := _region_at(from * size)
	_set_hover(region)
	if region < 0 or not creator.call(_feat("begin"), mode, _sculpt_mirror):
		return
	_face_drag = mode
	_set_active(region)
	var step_px := (to - from) * size / float(steps)
	for i in steps:
		creator.call(_feat("drag"), step_px, _camera_frame(), _skeleton)
	if hold:
		return
	_face_drag = ""
	_set_active(-1)
	if creator.call(_feat("end")):
		_changed(true)
		_show_step(on)


func _unhandled_key_input(event: InputEvent) -> void:
	if not (event is InputEventKey and event.pressed):
		return
	if event.keycode == KEY_Q:
		_yaw -= 0.3
	elif event.keycode == KEY_E:
		_yaw += 0.3
	elif event.keycode == KEY_S and event.ctrl_pressed:
		_save()
	elif event.keycode == KEY_Z and event.ctrl_pressed and _featuring():
		_face_undo()
	_pivot.rotation.y = _yaw


# --- Files -------------------------------------------------------------------------------------

func _update_status() -> void:
	if _model != null and _meter != null:
		var tris: int = _model.triangles()
		_meter.text = "%s triangles  ·  target 5,000–10,000" % _thousands(tris)
		var fill := UI._box(UI.GOLD if tris <= 10000 else Color(0.95, 0.6, 0.25), Color(0, 0, 0, 0), 0, 3, Vector4(0, 0, 0, 0))
		_meter_bar.add_theme_stylebox_override("fill", fill)
		_meter_bar.value = tris
	var note: String = creator.note()
	var problems: PackedStringArray = creator.problems()
	if note.begins_with("Can't show"):
		_toast_show(note, "error")
	elif note != "":
		_toast_show(note)
	elif not problems.is_empty():
		_toast_show("Can't save yet: " + "; ".join(problems), "error")
	else:
		_toast_show("")
	_update_header()


## Who is being made, at the top of the rail.
func _update_header() -> void:
	if _name_label == null:
		return
	var n: String = creator.name().strip_edges()
	_name_label.text = n if n != "" else "Unnamed"
	var b: String = BUILD_NAMES.get(String(creator.build()), String(creator.build()))
	var g: String = creator.gender()
	_sub_label.text = b + ("" if g == "" else "  ·  " + g.capitalize()) + ("  ·  unsaved" if dirty else "")


func _thousands(n: int) -> String:
	var t := str(n)
	var out := ""
	while t.length() > 3:
		out = "," + t.substr(t.length() - 3) + out
		t = t.substr(0, t.length() - 3)
	return t + out


## The face shapes (21 §7 FR5): whole faces to start from, the current one marked; Random face
## mixes them. The sliders below then fine-tune, and drags go on top.
func _face_shapes() -> void:
	var shapes: Array = creator.face_shapes()
	if shapes.is_empty():
		return
	_section("Shapes")
	var grid := GridContainer.new()
	grid.columns = 3
	grid.add_theme_constant_override("h_separation", 6)
	grid.add_theme_constant_override("v_separation", 6)
	var now: String = creator.face_shape()
	for sh in shapes:
		var b := Button.new()
		b.text = sh.name
		b.tooltip_text = sh.about
		b.toggle_mode = true
		b.button_pressed = sh.id == now
		b.focus_mode = Control.FOCUS_NONE
		b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		var m := Vector4(8, 7, 8, 7)
		var on := UI._box(UI.SLATE_HI, UI.GOLD, 1, 5, m)
		b.add_theme_stylebox_override("normal", UI._box(UI.INK_2, Color(0.2, 0.22, 0.26), 1, 5, m))
		b.add_theme_stylebox_override("hover", UI._box(UI.SLATE, UI.GOLD, 1, 5, m))
		b.add_theme_stylebox_override("pressed", on)
		b.add_theme_stylebox_override("hover_pressed", on)
		b.pressed.connect(func():
			creator.set_face_shape(sh.id)
			_changed(true)
			_show_step(step))
		grid.add_child(b)
	_panel.add_child(grid)
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 6)
	var rand := Button.new()
	rand.text = "Random face"
	rand.tooltip_text = "One or two shapes mixed, a few features nudged"
	rand.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	rand.pressed.connect(func(): random_face(int(Time.get_ticks_usec())))
	row.add_child(rand)
	_panel.add_child(row)
	_hint("A shape sets the sliders below; slide or drag from there.")


## A random face (`--creator-random-face SEED` for captures).
func random_face(seed: int) -> void:
	seed_n += 1
	creator.random_face(seed + seed_n)
	_changed(true)
	_show_step(step)


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
		_toast_show("Not saved: " + err, "error")
		return
	file_path = path
	dirty = false
	_update_header()
	_toast_show("Saved to %s" % path, "ok")
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
		_toast_show("Couldn't load: " + err, "error")
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
