extends Control
## The map editor (12): draw blue's half of a lane map (lanes, the river, jungle paths, tall grass,
## clearings, camps, great objectives); red's half is it turned half a turn about the center. Every change is
## rebuilt with the game's own code (`MftrMaps`) and checked, so the canvas shows the walls the
## game will build and the right panel lists what still breaks the rules.
##
## Canvas: wheel zooms, middle drag (or Space + drag) pans. Line tools: click to add points,
## double-click, Enter or right-click to finish, Esc to cancel. Select: drag points, camps,
## objectives, clearings and turrets (along their lane); Delete removes; Ctrl+Z undoes; Ctrl+S
## saves.

const Seasons = preload("res://scripts/seasons.gd")

signal closed

const MAPS_DIR := "user://maps"
const CONFIG := "user://map_editor.cfg"
const TOOLS := [
	["select", "Select / move", "Drag points, camps, objectives, clearings and turrets (along their lane). Delete removes; Ctrl+Z undoes."],
	["lane", "Lane", "From inside the base out to the river line (the dashed diagonal). Click points; double-click, Enter or right-click to finish. 1–5 lanes; turrets place themselves. The top and bottom lanes pair up (each runs on into the other's turned half), so their ends snap to matching spots; a middle lane ends at the center. ARAM: one lane, from where the base should be (its first point is the back of the base) to the center."],
	["river", "River", "Along the river line. Its turned copy makes the other bank."],
	["jungle", "Jungle path", "Paths must join a lane, the river or the base."],
	["grass", "Tall grass", "Brush: hides what's inside from outside."],
	["clearing", "Clearing", "Open ground: click to place (radius from the slider)."],
	["camp", "Camp", "Click to place the chosen camp (it carves its own clearing). Each side needs 3, with both buff camps. Red's copy is turned: blue's top-side camps are red's bottom side."],
	["objective", "Great objective", "Click to place one and its twin across the center. Shift-click for a single one (1-lane maps only)."],
	["rock", "Rock", "A small wall in the open, for cover: click to place (radius from the slider)."],
	["relic", "Health relic", "ARAM: click to place a health relic (red's is its turned copy)."],
]
## The built-in maps the editor opens from "Load default map": id, name. Add a map here and to
## `_default_map_text`.
const DEFAULT_MAPS := [["crossroads", "Crossroads"], ["bridge", "The Bridge"]]
## Tools a kind of map has no use for.
const KIND_TOOLS_OFF := {"classic": ["relic"], "aram": ["camp", "objective"]}
const KINDS := [["classic", "Classic"], ["aram", "ARAM"]]
const LINE_KINDS := ["lane", "river", "jungle", "grass"]
const LINE_COLORS := {
	"lane": Color(0.86, 0.68, 0.42), "river": Color(0.35, 0.65, 0.92),
	"jungle": Color(0.62, 0.5, 0.33), "grass": Color(0.4, 0.78, 0.32),
}
const WIDTHS := {
	"lane": [1000.0, 700.0, 1400.0], "river": [900.0, 500.0, 1600.0], "jungle": [480.0, 300.0, 900.0],
	"grass": [350.0, 200.0, 700.0], "clearing": [500.0, 250.0, 1200.0], "rock": [120.0, 60.0, 400.0],
}
const CAMPS := [
	["warden", "Azure Warden (buff: Insight)", "W", Color(0.35, 0.55, 0.95)],
	["brute", "Ember Brute (buff: Cinder)", "B", Color(0.92, 0.38, 0.2)],
	["hounds", "Thicket Hounds", "H", Color(0.62, 0.58, 0.52)],
	["toad", "Bog Toad", "T", Color(0.4, 0.7, 0.3)],
	["ravens", "Ravenhawks", "R", Color(0.58, 0.45, 0.72)],
	["crawlers", "Stone Crawlers", "C", Color(0.7, 0.7, 0.66)],
]
const SIZE_NAMES := ["Small", "Medium", "Large"]
const HANDLE_PX := 9.0

var spec := {}
var built := {}
var problems := PackedStringArray()
var tool := "select"
var widths := {}
var camp_kind := "warden"
var drawing := PackedVector2Array()
var file_path := ""
var dirty := false
var history: Array = []
var selected := {}
var dragging := false
var zoom := 1.0
var pan := Vector2.ZERO
var mouse_map := Vector2.ZERO
var rebuild_in := 0.0
var panning := false
var space_down := false

var overlay_tex: Texture2D = null
var overlay_path := ""
var overlay_rect := Rect2()
var overlay_alpha := 0.5
var overlay_shown := true
var aligning := false

var canvas: Control
var tool_buttons := {}
var width_row: HBoxContainer
var width_slider: HSlider
var width_label: Label
var camp_row: OptionButton
var name_edit: LineEdit
var size_pick: OptionButton
var kind_pick: OptionButton
var season_pick: OptionButton
var view_env: Environment
var view_sun: DirectionalLight3D
var problems_label: RichTextLabel
var help_label: Label
var status_label: Label
var title_label: Label
var walls_check: CheckBox
var overlay_slider: HSlider
var overlay_check: CheckBox
var align_button: Button
var file_dialog: FileDialog
var actual_check: CheckButton
var angle_check: CheckBox

# The Actual view: the map as the game builds it, in 3D, over the canvas.
const MapView := preload("res://scripts/map_view.gd")
const ALLY := Color(0.3, 0.6, 1.0)
const ENEMY := Color(0.95, 0.35, 0.25)
var view_box: SubViewportContainer
var view_port: SubViewport
var view_cam: Camera3D
var view_root: Node3D
var view_stale := true
var view_rebuild_in := 0.0


func _ready() -> void:
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_STOP
	for k in WIDTHS:
		widths[k] = WIDTHS[k][0]
	_build_ui()
	_open_text(_default_map_text("crossroads"), "")
	_load_config()
	await get_tree().process_frame
	_fit_view()


## Open a built-in map ("bridge", "crossroads") or a map file (`--editor-map`, for captures).
func open_map(which: String) -> void:
	var text := _default_map_text(which)
	if text != "":
		_open_text(text, "")
	else:
		_open_text(FileAccess.get_file_as_string(which), which)


## A built-in map's spec text, or "" for an id that isn't one.
func _default_map_text(id: String) -> String:
	match id:
		"crossroads":
			return MftrMaps.crossroads()
		"bridge":
			return MftrMaps.bridge()
	return ""


## Switch to the Actual view (`--editor-actual`, for captures).
func show_actual_view(game_angle := false) -> void:
	angle_check.button_pressed = game_angle
	actual_check.button_pressed = true


## Open the editor on a reference image right away (`--dev-overlay FILE`).
func set_overlay_file(path: String) -> void:
	_load_overlay(path)


# ---- UI --------------------------------------------------------------------------------------

func _build_ui() -> void:
	var bg := ColorRect.new()
	bg.color = Color(0.05, 0.06, 0.08)
	bg.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(bg)
	var row := HBoxContainer.new()
	row.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	row.add_theme_constant_override("separation", 0)
	add_child(row)

	var left := _side_panel(280)
	row.add_child(left[0])
	var lv: VBoxContainer = left[1]
	title_label = Label.new()
	title_label.text = "Map editor"
	title_label.theme_type_variation = "TitleLabel"
	lv.add_child(title_label)
	var grid := GridContainer.new()
	grid.columns = 2
	grid.add_theme_constant_override("h_separation", 8)
	var nl := Label.new()
	nl.text = "Name"
	grid.add_child(nl)
	name_edit = LineEdit.new()
	name_edit.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	name_edit.text_changed.connect(func(t: String):
		spec["name"] = t
		_changed(false))
	grid.add_child(name_edit)
	var kl := Label.new()
	kl.text = "Kind"
	grid.add_child(kl)
	kind_pick = OptionButton.new()
	for k in KINDS:
		kind_pick.add_item(k[1])
	kind_pick.tooltip_text = "Classic: 1–5 lanes, a jungle, the base in the corner. ARAM: one lane whose start is the base, no jungle, health relics, a fountain that doesn't heal."
	kind_pick.item_selected.connect(func(i: int): _set_kind(KINDS[i][0]))
	grid.add_child(kind_pick)
	var sl := Label.new()
	sl.text = "Size"
	grid.add_child(sl)
	size_pick = OptionButton.new()
	_fill_sizes("classic")
	size_pick.item_selected.connect(_resize_map)
	grid.add_child(size_pick)
	var sel := Label.new()
	sel.text = "Season"
	grid.add_child(sel)
	season_pick = OptionButton.new()
	for n in Seasons.NAMES:
		season_pick.add_item(str(n).capitalize())
	season_pick.tooltip_text = "How the map is dressed by default: its ground, cliffs, light, flora and fauna. Players can choose another in their settings. Looks only: it never changes play."
	season_pick.item_selected.connect(func(i: int):
		_snapshot()
		spec["season"] = Seasons.NAMES[i]
		view_stale = true
		_changed(false))
	grid.add_child(season_pick)
	lv.add_child(grid)

	lv.add_child(_header("TOOLS"))
	var group := ButtonGroup.new()
	var tools := GridContainer.new()
	tools.columns = 2
	for t in TOOLS:
		var b := Button.new()
		b.text = t[1]
		b.toggle_mode = true
		b.button_group = group
		b.focus_mode = Control.FOCUS_NONE
		b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		b.tooltip_text = t[2]
		var id: String = t[0]
		b.pressed.connect(func(): _set_tool(id))
		tools.add_child(b)
		tool_buttons[id] = b
	lv.add_child(tools)
	camp_row = OptionButton.new()
	for c in CAMPS:
		camp_row.add_item(c[1])
	camp_row.item_selected.connect(func(i: int): camp_kind = CAMPS[i][0])
	lv.add_child(camp_row)
	width_row = HBoxContainer.new()
	width_label = Label.new()
	width_label.custom_minimum_size = Vector2(96, 0)
	width_row.add_child(width_label)
	width_slider = HSlider.new()
	width_slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	width_slider.step = 10.0
	width_slider.value_changed.connect(_width_changed)
	width_row.add_child(width_slider)
	lv.add_child(width_row)
	help_label = Label.new()
	help_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	help_label.theme_type_variation = "HintLabel"
	help_label.custom_minimum_size = Vector2(240, 0)
	lv.add_child(help_label)

	lv.add_child(_header("VIEW"))
	walls_check = CheckBox.new()
	walls_check.text = "Built walls"
	walls_check.button_pressed = true
	walls_check.toggled.connect(func(_on: bool): canvas.queue_redraw())
	lv.add_child(walls_check)
	var fit := Button.new()
	fit.text = "Fit to view"
	fit.pressed.connect(_fit_view)
	lv.add_child(fit)
	actual_check = CheckButton.new()
	actual_check.text = "Actual view"
	actual_check.tooltip_text = "The map as the game builds it: cliffs, roads, the river, trees, rocks, tall grass and structures. Zoom and pan as usual."
	actual_check.toggled.connect(_show_actual)
	lv.add_child(actual_check)
	angle_check = CheckBox.new()
	angle_check.text = "Game camera angle"
	angle_check.tooltip_text = "Look at the Actual view from the game's camera angle instead of straight down."
	angle_check.toggled.connect(func(_on: bool): _place_view_camera())
	lv.add_child(angle_check)

	lv.add_child(_header("REFERENCE IMAGE"))
	var load_ref := Button.new()
	load_ref.text = "Load image…"
	load_ref.tooltip_text = "An image to trace over (your own file; it isn't saved in the map)."
	load_ref.pressed.connect(func(): _ask_file(FileDialog.FILE_MODE_OPEN_FILE, ["*.png, *.jpg, *.jpeg, *.webp ; Images"], _load_overlay))
	lv.add_child(load_ref)
	var orow := HBoxContainer.new()
	overlay_check = CheckBox.new()
	overlay_check.text = "Show"
	overlay_check.button_pressed = true
	overlay_check.toggled.connect(func(on: bool):
		overlay_shown = on
		canvas.queue_redraw())
	orow.add_child(overlay_check)
	overlay_slider = HSlider.new()
	overlay_slider.min_value = 0.1
	overlay_slider.max_value = 1.0
	overlay_slider.step = 0.05
	overlay_slider.value = overlay_alpha
	overlay_slider.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	overlay_slider.value_changed.connect(func(v: float):
		overlay_alpha = v
		_save_config()
		canvas.queue_redraw())
	orow.add_child(overlay_slider)
	lv.add_child(orow)
	align_button = Button.new()
	align_button.text = "Align image"
	align_button.toggle_mode = true
	align_button.tooltip_text = "Drag the image's corner handles so its map lines up with the canvas square."
	align_button.toggled.connect(func(on: bool):
		aligning = on
		canvas.queue_redraw())
	lv.add_child(align_button)

	lv.add_child(_header("FILE"))
	var defaults := MenuButton.new()
	defaults.text = "Load default map ▾"
	defaults.flat = false
	defaults.tooltip_text = "Open one of the game's built-in maps to edit or start from. Save as… keeps your copy; the built-in map doesn't change."
	for i in DEFAULT_MAPS.size():
		defaults.get_popup().add_item(DEFAULT_MAPS[i][1], i)
	defaults.get_popup().id_pressed.connect(func(i: int): open_map(DEFAULT_MAPS[i][0]))
	lv.add_child(defaults)
	var files := GridContainer.new()
	files.columns = 2
	for f in [["New", _new_map], ["Open…", _open_dialog], ["Save", _save], ["Save as…", _save_as], ["Export…", _export]]:
		var b := Button.new()
		b.text = f[0]
		b.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		b.pressed.connect(f[1])
		files.add_child(b)
	lv.add_child(files)
	var close := Button.new()
	close.text = "Close editor"
	close.pressed.connect(_close)
	lv.add_child(close)

	canvas = Control.new()
	canvas.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	canvas.clip_contents = true
	canvas.focus_mode = Control.FOCUS_ALL
	canvas.mouse_filter = Control.MOUSE_FILTER_STOP
	canvas.draw.connect(_draw_canvas)
	canvas.gui_input.connect(_canvas_input)
	canvas.resized.connect(func(): canvas.queue_redraw())
	row.add_child(canvas)

	var right := _side_panel(320)
	row.add_child(right[0])
	var rv: VBoxContainer = right[1]
	rv.add_child(_header("PROBLEMS"))
	problems_label = RichTextLabel.new()
	problems_label.bbcode_enabled = true
	problems_label.fit_content = true
	problems_label.custom_minimum_size = Vector2(280, 120)
	rv.add_child(problems_label)
	status_label = Label.new()
	status_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status_label.theme_type_variation = "HintLabel"
	status_label.custom_minimum_size = Vector2(280, 0)
	rv.add_child(status_label)
	_set_tool("select")


func _side_panel(width: float) -> Array:
	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(width, 0)
	var scroll := ScrollContainer.new()
	scroll.horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	panel.add_child(scroll)
	var margin := MarginContainer.new()
	margin.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	for side in ["left", "right", "top", "bottom"]:
		margin.add_theme_constant_override("margin_" + side, 14)
	scroll.add_child(margin)
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 8)
	margin.add_child(v)
	return [panel, v]


func _header(text: String) -> Label:
	var l := Label.new()
	l.text = text
	l.theme_type_variation = "HeaderLabel"
	return l


func _set_tool(id: String) -> void:
	if drawing.size() > 0:
		_finish_line()
	tool = id
	(tool_buttons[id] as Button).button_pressed = true
	for t in TOOLS:
		if t[0] == id:
			help_label.text = t[2]
	camp_row.visible = id == "camp"
	var sized := WIDTHS.has(id)
	width_row.visible = sized
	if sized:
		var w: Array = WIDTHS[id]
		width_slider.min_value = w[1]
		width_slider.max_value = w[2]
		width_slider.set_value_no_signal(widths[id])
		width_label.text = ("Radius %d" if id in ["clearing", "rock"] else "Width %d") % int(widths[id])
	canvas.queue_redraw()


func _width_changed(v: float) -> void:
	if WIDTHS.has(tool):
		widths[tool] = v
		width_label.text = ("Radius %d" if tool in ["clearing", "rock"] else "Width %d") % int(v)
	# The selected line or clearing takes it too.
	if selected.get("what", "") == "line" and tool == "select":
		var l: Dictionary = spec[selected.kind][selected.index]
		l["width"] = v
		_changed(false)
	elif selected.get("what", "") in ["clearing", "rock"]:
		spec[selected.what + "s"][selected.index]["radius"] = v
		_changed(false)


## The size picker's choices for a kind of map.
func _fill_sizes(kind: String) -> void:
	size_pick.clear()
	var sizes: PackedFloat32Array = MftrMaps.sizes(kind)
	for i in sizes.size():
		size_pick.add_item("%s  (%d u)" % [SIZE_NAMES[mini(i, 2)], int(sizes[i])])


## Show the kind, size and season in their pickers, and only the tools the kind uses.
func _sync_kind_ui() -> void:
	var kind: String = spec.get("kind", "classic")
	season_pick.select(maxi(Seasons.NAMES.find(spec.get("season", "summer")), 0))
	for i in KINDS.size():
		if KINDS[i][0] == kind:
			kind_pick.select(i)
	_fill_sizes(kind)
	var sizes: PackedFloat32Array = MftrMaps.sizes(kind)
	for i in sizes.size():
		if is_equal_approx(sizes[i], float(spec.size)):
			size_pick.select(i)
	var off: Array = KIND_TOOLS_OFF.get(kind, [])
	for id in tool_buttons:
		(tool_buttons[id] as Button).disabled = id in off
	if tool in off:
		_set_tool("select")


## Make the map Classic or ARAM: its size moves to the kind's nearest, everything scaling.
func _set_kind(kind: String) -> void:
	if spec.get("kind", "classic") == kind:
		return
	_snapshot()
	spec["kind"] = kind
	var sizes: PackedFloat32Array = MftrMaps.sizes(kind)
	var best := 0
	for i in sizes.size():
		if absf(sizes[i] - float(spec.size)) < absf(sizes[best] - float(spec.size)):
			best = i
	_scale_spec(sizes[best])
	_sync_kind_ui()
	_changed(false)
	_fit_view()


# ---- the spec --------------------------------------------------------------------------------

func _empty_spec(size: float, kind := "classic") -> Dictionary:
	return {"name": "Untitled", "kind": kind, "size": size, "lane": [], "river": [], "jungle": [], "grass": [],
		"clearings": [], "camps": [], "objectives": PackedVector2Array(), "structures": [],
		"relics": PackedVector2Array(), "rocks": []}


func _open_text(text: String, path: String) -> void:
	var d: Dictionary = MftrMaps.parse(text)
	if not d.get("ok", false):
		status_label.text = "Couldn't open: %s" % d.get("error", "?")
		return
	spec = d
	spec.erase("ok")
	file_path = path
	dirty = false
	history.clear()
	selected = {}
	drawing = PackedVector2Array()
	name_edit.text = spec.name
	_sync_kind_ui()
	_rebuild_now()
	_fit_view()
	status_label.text = "Opened %s." % (path.get_file() if path != "" else spec.name)


func to_text() -> String:
	var pt := func(p: Vector2) -> String: return "%d,%d" % [roundi(p.x), roundi(p.y)]
	var s := "mftr-map 1\nname %s\n" % (str(spec.name).strip_edges() if str(spec.name).strip_edges() != "" else "Untitled")
	if spec.get("kind", "classic") != "classic":
		s += "kind %s\n" % spec.kind
	s += "size %d\n" % int(spec.size)
	if spec.get("season", "summer") != "summer":
		s += "season %s\n" % spec.season
	for kind in LINE_KINDS:
		for l in spec[kind]:
			var pts := PackedStringArray()
			for p in (l.points as PackedVector2Array):
				pts.append(pt.call(p))
			s += "%s %d %s\n" % [kind, roundi(l.width), " ".join(pts)]
	for c in spec.clearings:
		s += "clearing %s %d\n" % [pt.call(c.pos), roundi(c.radius)]
	for c in spec.camps:
		s += "camp %s %s\n" % [c.kind, pt.call(c.pos)]
	for p in (spec.objectives as PackedVector2Array):
		s += "objective %s\n" % pt.call(p)
	for st in spec.structures:
		s += "structure %d %d %d\n" % [int(st.lane), int(st.tier), roundi(st.distance)]
	for p in (spec.get("relics", PackedVector2Array()) as PackedVector2Array):
		s += "relic %s\n" % pt.call(p)
	for r in spec.get("rocks", []):
		s += "rock %s %d\n" % [pt.call(r.pos), roundi(r.radius)]
	return s


## Remember the spec before an edit (Ctrl+Z goes back to it).
func _snapshot() -> void:
	history.append(spec.duplicate(true))
	if history.size() > 80:
		history.pop_front()


func _undo() -> void:
	if history.is_empty():
		return
	spec = history.pop_back()
	selected = {}
	name_edit.text = spec.name
	_sync_kind_ui()
	_changed(false)


func _changed(snapshot := true) -> void:
	if snapshot:
		_snapshot()
	dirty = true
	rebuild_in = 0.15
	canvas.queue_redraw()


func _rebuild_now() -> void:
	var text := to_text()
	built = MftrMaps.build(text)
	problems = MftrMaps.problems(text)
	var t := ""
	if problems.is_empty():
		t = "[color=#7fd67f]No problems: this map can be exported and played.[/color]"
	else:
		for p in problems:
			t += "[color=#ff8a70]•[/color] %s\n" % p
	problems_label.text = t
	title_label.text = "Map editor%s" % ("  •" if dirty else "")
	view_stale = true
	if view_box != null and view_box.visible:
		view_rebuild_in = 0.4
	canvas.queue_redraw()


func _process(delta: float) -> void:
	if rebuild_in > 0.0:
		rebuild_in -= delta
		if rebuild_in <= 0.0:
			_rebuild_now()
	if view_box != null and view_box.visible:
		if view_rebuild_in > 0.0:
			view_rebuild_in -= delta
			if view_rebuild_in <= 0.0:
				_build_view()
		_place_view_camera()


func _resize_map(i: int) -> void:
	var sizes: PackedFloat32Array = MftrMaps.sizes(spec.get("kind", "classic"))
	var size: float = sizes[i]
	if is_equal_approx(float(spec.size), size):
		return
	_snapshot()
	_scale_spec(size)
	_changed(false)
	_fit_view()


## Scale everything to a map of side `size`.
func _scale_spec(size: float) -> void:
	var old: float = spec.size
	var k := size / old
	for kind in LINE_KINDS:
		for l in spec[kind]:
			var pts: PackedVector2Array = l.points
			for j in pts.size():
				pts[j] = pts[j] * k
			l["points"] = pts
	for c in spec.clearings:
		c["pos"] = (c.pos as Vector2) * k
	for c in spec.camps:
		c["pos"] = (c.pos as Vector2) * k
	var obj: PackedVector2Array = spec.objectives
	for j in obj.size():
		obj[j] = obj[j] * k
	spec["objectives"] = obj
	for st in spec.structures:
		st["distance"] = float(st.distance) * k
	var relics: PackedVector2Array = spec.get("relics", PackedVector2Array())
	for j in relics.size():
		relics[j] = relics[j] * k
	spec["relics"] = relics
	for r in spec.get("rocks", []):
		r["pos"] = (r.pos as Vector2) * k
	spec["size"] = size


# ---- view ------------------------------------------------------------------------------------

func _scale() -> float:
	# (Never degenerate, even before the canvas has its size.)
	return zoom * maxf(minf(canvas.size.x, canvas.size.y) - 40.0, 400.0) / float(spec.get("size", 14500.0))


func to_screen(p: Vector2) -> Vector2:
	return pan + p * _scale()


func to_map(s: Vector2) -> Vector2:
	return (s - pan) / _scale()


func _fit_view() -> void:
	if canvas == null or spec.is_empty():
		return
	zoom = 1.0
	var side := float(spec.size) * _scale()
	pan = (canvas.size - Vector2(side, side)) / 2.0
	canvas.queue_redraw()


# ---- drawing ---------------------------------------------------------------------------------

func _draw_canvas() -> void:
	if spec.is_empty():
		return
	var c := canvas
	var s := float(spec.size)
	var k := _scale()
	var font := get_theme_default_font()
	if view_box != null and view_box.visible:
		return
	c.draw_rect(Rect2(to_screen(Vector2.ZERO), Vector2(s, s) * k), Color(0.3, 0.38, 0.25))
	if overlay_tex != null and overlay_shown:
		c.draw_texture_rect(overlay_tex, Rect2(to_screen(overlay_rect.position), overlay_rect.size * k), false, Color(1, 1, 1, overlay_alpha))
	var faded := overlay_tex != null and overlay_shown
	if built.get("ok", false) and walls_check.button_pressed:
		var wall_color := Color(0.13, 0.13, 0.15, 0.55 if faded else 0.92)
		for poly in built.walls:
			_fill(_screen_poly(poly), wall_color)
		for poly in built.brush:
			_fill(_screen_poly(poly), Color(0.2, 0.5, 0.18, 0.7))
	# Red's half (blue's turned), shaded; the river line.
	c.draw_colored_polygon(PackedVector2Array([to_screen(Vector2.ZERO), to_screen(Vector2(s, 0)), to_screen(Vector2(s, s))]), Color(0.6, 0.1, 0.1, 0.12))
	_dashed(to_screen(Vector2.ZERO), to_screen(Vector2(s, s)), Color(0.6, 0.8, 1.0, 0.6))
	# The bases: blue's corner square (Classic), or the court at the lane's start (ARAM).
	if built.has("base_court"):
		var court: Array = built.base_court
		var cc: Vector2 = court[0]
		c.draw_arc(to_screen(cc), float(court[1]) * k, 0, TAU, 48, Color(0.4, 0.6, 1.0, 0.7), 2.0)
		c.draw_arc(to_screen(Vector2(s, s) - cc), float(court[1]) * k, 0, TAU, 48, Color(1.0, 0.45, 0.4, 0.5), 2.0)
	else:
		var b: float = built.get("base_side", s * 0.27)
		c.draw_rect(Rect2(to_screen(Vector2(0, s - b)), Vector2(b, b) * k), Color(0.4, 0.6, 1.0, 0.7), false, 2.0)
		c.draw_rect(Rect2(to_screen(Vector2(s - b, 0)), Vector2(b, b) * k), Color(1.0, 0.45, 0.4, 0.5), false, 2.0)
	# The drawn lines: their mirror faint, blue's half with its points.
	for kind in LINE_KINDS:
		var col: Color = LINE_COLORS[kind]
		for i in spec[kind].size():
			var l: Dictionary = spec[kind][i]
			var pts: PackedVector2Array = l.points
			var mirrored := PackedVector2Array()
			var mine := PackedVector2Array()
			for p in pts:
				mirrored.append(to_screen(Vector2(s, s) - p))
				mine.append(to_screen(p))
			var w := maxf(2.0, float(l.width) * k)
			var sel: bool = selected.get("what", "") == "line" and selected.kind == kind and selected.index == i
			# Lighter over a reference image, so it shows through.
			var band := 0.2 if faded else 0.42
			c.draw_polyline(mirrored, Color(col, band * 0.45), w)
			c.draw_polyline(mine, Color(col, band if not sel else band + 0.2), w)
			c.draw_polyline(mine, Color(col.lightened(0.3), 1.0), 2.0 if not sel else 3.0)
			for j in mine.size():
				var hot: bool = sel and selected.get("point", -1) == j
				c.draw_circle(mine[j], 5.0 if not hot else 7.0, Color.WHITE if hot else col.lightened(0.4))
	for i in spec.clearings.size():
		var cl: Dictionary = spec.clearings[i]
		var p: Vector2 = cl.pos
		var sel: bool = selected.get("what", "") == "clearing" and selected.index == i
		c.draw_arc(to_screen(p), float(cl.radius) * k, 0, TAU, 40, Color(0.9, 0.9, 0.7, 0.9 if sel else 0.6), 2.0)
		c.draw_arc(to_screen(Vector2(s, s) - p), float(cl.radius) * k, 0, TAU, 40, Color(0.9, 0.9, 0.7, 0.25), 1.5)
		c.draw_circle(to_screen(p), 4.0, Color(0.95, 0.95, 0.8))
	for i in spec.camps.size():
		var cp: Dictionary = spec.camps[i]
		var p: Vector2 = cp.pos
		var info: Array = _camp_info(cp.kind)
		var sel: bool = selected.get("what", "") == "camp" and selected.index == i
		var r := maxf(7.0, 170.0 * k)
		c.draw_circle(to_screen(Vector2(s, s) - p), r, Color(info[3], 0.3))
		c.draw_circle(to_screen(p), r + (3.0 if sel else 1.5), Color.WHITE if sel else Color.BLACK)
		c.draw_circle(to_screen(p), r, info[3])
		c.draw_string(font, to_screen(p) + Vector2(-r, r * 0.42), info[2], HORIZONTAL_ALIGNMENT_CENTER, r * 2.0, int(maxf(10.0, r * 1.15)), Color.WHITE)
	var rocks: Array = spec.get("rocks", [])
	for i in rocks.size():
		var rk: Dictionary = rocks[i]
		var sel: bool = selected.get("what", "") == "rock" and selected.index == i
		var r := maxf(4.0, float(rk.radius) * k)
		c.draw_circle(to_screen(Vector2(s, s) - (rk.pos as Vector2)), r, Color(0.45, 0.42, 0.4, 0.5))
		c.draw_circle(to_screen(rk.pos), r + (2.5 if sel else 1.0), Color.WHITE if sel else Color.BLACK)
		c.draw_circle(to_screen(rk.pos), r, Color(0.55, 0.52, 0.48))
	var relics: PackedVector2Array = spec.get("relics", PackedVector2Array())
	for i in relics.size():
		var sel: bool = selected.get("what", "") == "relic" and selected.index == i
		var r := maxf(6.0, 130.0 * k)
		for q in [Vector2(s, s) - relics[i], relics[i]]:
			var mine: bool = q == relics[i]
			var p := to_screen(q)
			c.draw_circle(p, r + (2.5 if sel and mine else 1.0), Color.WHITE if sel and mine else Color.BLACK)
			c.draw_circle(p, r, Color(0.35, 0.8, 0.45, 1.0 if mine else 0.45))
			c.draw_line(p - Vector2(r * 0.55, 0), p + Vector2(r * 0.55, 0), Color.WHITE, 2.0)
			c.draw_line(p - Vector2(0, r * 0.55), p + Vector2(0, r * 0.55), Color.WHITE, 2.0)
	var objs: PackedVector2Array = spec.objectives
	for i in objs.size():
		var p := to_screen(objs[i])
		var r := maxf(8.0, 220.0 * k)
		var sel: bool = selected.get("what", "") == "objective" and selected.index == i
		var d := PackedVector2Array([p + Vector2(0, -r), p + Vector2(r, 0), p + Vector2(0, r), p + Vector2(-r, 0)])
		c.draw_colored_polygon(d, Color(0.95, 0.75, 0.25))
		d.append(d[0])
		c.draw_polyline(d, Color.WHITE if sel else Color.BLACK, 2.0)
	# Structures as the game places them; blue's turrets and Gatehouses can be dragged.
	if built.get("ok", false):
		for st in built.structures:
			var p := to_screen(st.pos)
			var blue: bool = st.team == "blue"
			var col := Color(0.35, 0.6, 1.0) if blue else Color(1.0, 0.4, 0.35)
			if not blue:
				col.a = 0.55
			var sel: bool = selected.get("what", "") == "structure" and blue and selected.lane == st.spec_lane and selected.tier == st.tier
			match st.kind:
				"base":
					c.draw_circle(p, maxf(8.0, 260.0 * k), col)
				"gatehouse":
					var r := maxf(6.0, 170.0 * k)
					c.draw_rect(Rect2(p - Vector2(r, r), Vector2(r, r) * 2.0), col)
					if sel:
						c.draw_rect(Rect2(p - Vector2(r, r), Vector2(r, r) * 2.0), Color.WHITE, false, 2.0)
				_:
					var r := maxf(5.0, 110.0 * k)
					c.draw_circle(p, r + (2.5 if sel else 1.0), Color.WHITE if sel else Color.BLACK)
					c.draw_circle(p, r, col)
					if int(st.tier) <= 4:
						c.draw_string(font, p + Vector2(-r, r * 0.45), str(st.tier), HORIZONTAL_ALIGNMENT_CENTER, r * 2.0, int(maxf(9.0, r * 1.2)), Color.WHITE)
		for f in (built.fountains as PackedVector2Array):
			c.draw_arc(to_screen(f), float(built.get("fountain_radius", 750.0)) * k, 0, TAU, 40, Color(0.8, 0.9, 1.0, 0.6), 2.0)
	# The line being drawn.
	if drawing.size() > 0:
		var pts := PackedVector2Array()
		for p in drawing:
			pts.append(to_screen(p))
		pts.append(to_screen(mouse_map))
		var col: Color = LINE_COLORS.get(tool, Color.WHITE)
		c.draw_polyline(pts, Color(col, 0.4), maxf(2.0, float(widths.get(tool, 400.0)) * k))
		c.draw_polyline(pts, col.lightened(0.3), 2.0)
	if aligning and overlay_tex != null:
		for corner in [overlay_rect.position, overlay_rect.end]:
			var p := to_screen(corner)
			c.draw_rect(Rect2(p - Vector2(7, 7), Vector2(14, 14)), Color(1, 0.85, 0.2))
			c.draw_rect(Rect2(p - Vector2(7, 7), Vector2(14, 14)), Color.BLACK, false, 2.0)
	# Where the cursor is.
	var half := "blue's half" if mouse_map.y >= mouse_map.x else "red's half (blue's, turned)"
	c.draw_string(font, Vector2(10, c.size.y - 12), "%d, %d   ·   %s   ·   zoom %d%%" % [roundi(mouse_map.x), roundi(mouse_map.y), half, roundi(zoom * 100.0)], HORIZONTAL_ALIGNMENT_LEFT, -1, 13, Color(0.85, 0.88, 0.92))


func _screen_poly(poly: PackedVector2Array) -> PackedVector2Array:
	var out := PackedVector2Array()
	out.resize(poly.size())
	for i in poly.size():
		out[i] = to_screen(poly[i])
	return out


func _fill(pts: PackedVector2Array, color: Color) -> void:
	if pts.size() >= 3 and not Geometry2D.triangulate_polygon(pts).is_empty():
		canvas.draw_colored_polygon(pts, color)


func _dashed(a: Vector2, b: Vector2, color: Color) -> void:
	var len := a.distance_to(b)
	var t := 0.0
	while t < len:
		canvas.draw_line(a.lerp(b, t / len), a.lerp(b, minf(t + 10.0, len) / len), color, 1.5)
		t += 18.0


func _camp_info(kind: String) -> Array:
	for c in CAMPS:
		if c[0] == kind:
			return c
	return CAMPS[0]


# ---- input -----------------------------------------------------------------------------------

func _canvas_input(event: InputEvent) -> void:
	if event is InputEventMouseMotion:
		var mm := event as InputEventMouseMotion
		mouse_map = to_map(mm.position)
		if panning:
			pan += mm.relative
		elif dragging:
			_drag_to(mouse_map)
		canvas.queue_redraw()
	elif event is InputEventMouseButton:
		var mb := event as InputEventMouseButton
		mouse_map = to_map(mb.position)
		if mb.pressed and mb.button_index in [MOUSE_BUTTON_WHEEL_UP, MOUSE_BUTTON_WHEEL_DOWN]:
			var before := mouse_map
			zoom = clampf(zoom * (1.15 if mb.button_index == MOUSE_BUTTON_WHEEL_UP else 1.0 / 1.15), 0.5, 12.0)
			pan += mb.position - to_screen(before)
			canvas.queue_redraw()
			return
		if mb.button_index == MOUSE_BUTTON_MIDDLE or (mb.button_index == MOUSE_BUTTON_LEFT and space_down):
			panning = mb.pressed
			return
		if mb.button_index == MOUSE_BUTTON_RIGHT and mb.pressed:
			if drawing.size() > 0:
				_finish_line()
			return
		if mb.button_index == MOUSE_BUTTON_LEFT:
			canvas.grab_focus()
			if mb.pressed:
				_press(mouse_map, mb.double_click, mb.shift_pressed)
			else:
				dragging = false
	elif event is InputEventKey:
		var ke := event as InputEventKey
		if ke.keycode == KEY_SPACE:
			space_down = ke.pressed
			return
		if not ke.pressed:
			return
		match ke.keycode:
			KEY_ENTER, KEY_KP_ENTER:
				_finish_line()
			KEY_ESCAPE:
				drawing = PackedVector2Array()
				selected = {}
				canvas.queue_redraw()
			KEY_DELETE, KEY_BACKSPACE:
				_delete_selected(ke.shift_pressed)
			KEY_Z:
				if ke.ctrl_pressed:
					_undo()
			KEY_S:
				if ke.ctrl_pressed:
					_save()
		canvas.accept_event()


func _press(p: Vector2, double: bool, shift: bool) -> void:
	p = p.clamp(Vector2.ZERO, Vector2(spec.size, spec.size))
	if aligning and overlay_tex != null:
		for i in 2:
			var corner: Vector2 = overlay_rect.position if i == 0 else overlay_rect.end
			if to_screen(corner).distance_to(to_screen(p)) <= 12.0:
				selected = {"what": "overlay", "corner": i}
				dragging = true
				return
	match tool:
		"select":
			selected = _pick(p)
			dragging = not selected.is_empty()
			if dragging:
				_snapshot()
				dirty = true
			_sync_width_to_selection()
		"lane", "river", "jungle", "grass":
			if drawing.is_empty() or drawing[drawing.size() - 1].distance_to(p) > 1.0:
				drawing.append(p)
			if double:
				_finish_line()
		"clearing":
			_snapshot()
			spec.clearings.append({"pos": _blue(p), "radius": widths.clearing})
			_changed(false)
		"camp":
			_snapshot()
			spec.camps.append({"kind": camp_kind, "pos": _blue(p)})
			_changed(false)
		"objective":
			_snapshot()
			var obj: PackedVector2Array = spec.objectives
			obj.append(p)
			if not shift:
				obj.append(Vector2(spec.size, spec.size) - p)
			spec["objectives"] = obj
			_changed(false)
		"rock":
			_snapshot()
			spec.rocks.append({"pos": _blue(p), "radius": widths.rock})
			_changed(false)
		"relic":
			_snapshot()
			var relics: PackedVector2Array = spec.relics
			relics.append(_blue(p))
			spec["relics"] = relics
			_changed(false)
	canvas.queue_redraw()


## A point on blue's half (one placed on red's is turned onto blue's).
func _blue(p: Vector2) -> Vector2:
	return p if p.y >= p.x else Vector2(spec.size, spec.size) - p


func _finish_line() -> void:
	if drawing.size() >= 2 and tool in LINE_KINDS:
		_snapshot()
		var pts := drawing
		if tool == "lane":
			pts[pts.size() - 1] = _lane_end(pts[pts.size() - 1])
		spec[tool].append({"width": widths[tool], "points": pts})
		_changed(false)
	drawing = PackedVector2Array()
	canvas.queue_redraw()


## Where a lane drawn to `last` should end: on the river line when it's close; then at the map's
## center (a middle lane) or at the turned end of a lane it pairs with, when one is near.
func _lane_end(last: Vector2) -> Vector2:
	var on := Vector2((last.x + last.y) / 2.0, (last.x + last.y) / 2.0)
	if last.distance_to(on) >= 700.0:
		return last
	var s := float(spec.size)
	var center := Vector2(s, s) / 2.0
	if on.distance_to(center) < 900.0:
		return center
	for l in spec.lane:
		var pts: PackedVector2Array = l.points
		var partner := Vector2(s, s) - pts[pts.size() - 1]
		if on.distance_to(partner) < 1500.0:
			return partner
	return on


## The handle or line under the cursor (within a few pixels): structures first, then points,
## then line bodies.
func _pick(p: Vector2) -> Dictionary:
	if built.get("ok", false):
		for st in built.structures:
			if st.team == "blue" and int(st.tier) <= 4 and _near(st.pos, p, 4.0):
				return {"what": "structure", "lane": int(st.spec_lane), "tier": int(st.tier)}
	var objs: PackedVector2Array = spec.objectives
	for i in objs.size():
		if _near(objs[i], p, 4.0):
			return {"what": "objective", "index": i, "twin": _twin(i)}
	for i in spec.camps.size():
		if _near(spec.camps[i].pos, p, 4.0):
			return {"what": "camp", "index": i}
	var relics: PackedVector2Array = spec.get("relics", PackedVector2Array())
	for i in relics.size():
		if _near(relics[i], p, 4.0):
			return {"what": "relic", "index": i}
	var rocks: Array = spec.get("rocks", [])
	for i in rocks.size():
		if _near(rocks[i].pos, p, 2.0):
			return {"what": "rock", "index": i}
	for i in spec.clearings.size():
		if _near(spec.clearings[i].pos, p, 0.0):
			return {"what": "clearing", "index": i}
	for kind in LINE_KINDS:
		for i in spec[kind].size():
			var pts: PackedVector2Array = spec[kind][i].points
			for j in pts.size():
				if _near(pts[j], p, 0.0):
					return {"what": "line", "kind": kind, "index": i, "point": j}
	for kind in ["lane", "river", "jungle", "grass"]:
		for i in spec[kind].size():
			var l: Dictionary = spec[kind][i]
			var pts: PackedVector2Array = l.points
			for j in range(1, pts.size()):
				var q := Geometry2D.get_closest_point_to_segment(p, pts[j - 1], pts[j])
				if q.distance_to(p) <= float(l.width) / 2.0:
					return {"what": "line", "kind": kind, "index": i, "point": -1}
	return {}


## A handle at `q` is under the cursor at `p` (map units), within a few pixels.
func _near(q: Vector2, p: Vector2, extra: float) -> bool:
	return to_screen(q).distance_to(to_screen(p)) <= HANDLE_PX + extra


## The objective across the center from objective `i`, or -1.
func _twin(i: int) -> int:
	var objs: PackedVector2Array = spec.objectives
	var want := Vector2(spec.size, spec.size) - objs[i]
	for j in objs.size():
		if j != i and objs[j].distance_to(want) < 60.0:
			return j
	return -1


func _sync_width_to_selection() -> void:
	var what: String = selected.get("what", "")
	if what == "line":
		width_row.visible = true
		var w: Array = WIDTHS[selected.kind]
		width_slider.min_value = w[1]
		width_slider.max_value = w[2]
		width_slider.set_value_no_signal(spec[selected.kind][selected.index].width)
		width_label.text = "Width %d" % int(width_slider.value)
	elif what in ["clearing", "rock"]:
		width_row.visible = true
		var w: Array = WIDTHS[what]
		width_slider.min_value = w[1]
		width_slider.max_value = w[2]
		width_slider.set_value_no_signal(spec[what + "s"][selected.index].radius)
		width_label.text = "Radius %d" % int(width_slider.value)
	else:
		width_row.visible = WIDTHS.has(tool)


func _drag_to(p: Vector2) -> void:
	p = p.clamp(Vector2.ZERO, Vector2(spec.size, spec.size))
	match selected.get("what", ""):
		"overlay":
			if selected.corner == 0:
				overlay_rect = Rect2(p, overlay_rect.end - p)
			else:
				overlay_rect.end = p
			_save_config()
		"line":
			if selected.point >= 0:
				var l: Dictionary = spec[selected.kind][selected.index]
				var pts: PackedVector2Array = l.points
				pts[selected.point] = p
				l["points"] = pts
				_changed(false)
		"camp":
			spec.camps[selected.index]["pos"] = _blue(p)
			_changed(false)
		"clearing":
			spec.clearings[selected.index]["pos"] = _blue(p)
			_changed(false)
		"rock":
			spec.rocks[selected.index]["pos"] = _blue(p)
			_changed(false)
		"relic":
			var relics: PackedVector2Array = spec.relics
			relics[selected.index] = _blue(p)
			spec["relics"] = relics
			_changed(false)
		"objective":
			var obj: PackedVector2Array = spec.objectives
			obj[selected.index] = p
			if selected.twin >= 0:
				obj[selected.twin] = Vector2(spec.size, spec.size) - p
			spec["objectives"] = obj
			_changed(false)
		"structure":
			var lane: int = selected.lane
			if lane < spec.lane.size():
				var d := _distance_along(spec.lane[lane].points, p)
				var found := false
				for st in spec.structures:
					if int(st.lane) == lane and int(st.tier) == selected.tier:
						st["distance"] = d
						found = true
				if not found:
					spec.structures.append({"lane": lane, "tier": selected.tier, "distance": d})
				_changed(false)


## How far along a polyline the point nearest `p` lies.
func _distance_along(pts: PackedVector2Array, p: Vector2) -> float:
	var best := INF
	var at := 0.0
	var walked := 0.0
	for j in range(1, pts.size()):
		var q := Geometry2D.get_closest_point_to_segment(p, pts[j - 1], pts[j])
		var d := q.distance_to(p)
		if d < best:
			best = d
			at = walked + pts[j - 1].distance_to(q)
		walked += pts[j - 1].distance_to(pts[j])
	return at


func _delete_selected(whole: bool) -> void:
	var what: String = selected.get("what", "")
	if what == "":
		return
	_snapshot()
	match what:
		"line":
			var lines: Array = spec[selected.kind]
			var pts: PackedVector2Array = lines[selected.index].points
			if selected.point >= 0 and pts.size() > 2 and not whole:
				pts.remove_at(selected.point)
				lines[selected.index]["points"] = pts
			else:
				lines.remove_at(selected.index)
				if selected.kind == "lane":
					# Its moved structures go; later lanes' move down a number.
					var kept: Array = []
					for st in spec.structures:
						if int(st.lane) != selected.index:
							if int(st.lane) > selected.index:
								st["lane"] = int(st.lane) - 1
							kept.append(st)
					spec["structures"] = kept
		"camp":
			spec.camps.remove_at(selected.index)
		"clearing":
			spec.clearings.remove_at(selected.index)
		"rock":
			spec.rocks.remove_at(selected.index)
		"relic":
			var relics: PackedVector2Array = spec.relics
			relics.remove_at(selected.index)
			spec["relics"] = relics
		"objective":
			var obj: PackedVector2Array = spec.objectives
			var drop := [selected.index]
			if selected.twin >= 0 and not whole:
				drop.append(selected.twin)
			drop.sort()
			drop.reverse()
			for i in drop:
				obj.remove_at(i)
			spec["objectives"] = obj
		"structure":
			# Back to its default spot.
			var kept: Array = []
			for st in spec.structures:
				if not (int(st.lane) == selected.lane and int(st.tier) == selected.tier):
					kept.append(st)
			spec["structures"] = kept
	selected = {}
	_changed(false)


# ---- files -----------------------------------------------------------------------------------

func _new_map() -> void:
	var kind: String = spec.get("kind", "classic") if not spec.is_empty() else "classic"
	var sizes: PackedFloat32Array = MftrMaps.sizes(kind)
	spec = _empty_spec(sizes[mini(size_pick.selected, sizes.size() - 1)] if size_pick.selected >= 0 else sizes[1], kind)
	_sync_kind_ui()
	file_path = ""
	history.clear()
	selected = {}
	name_edit.text = spec.name
	dirty = false
	_rebuild_now()
	_fit_view()
	status_label.text = "A new ARAM map: draw its lane from where the base should be to the center, then relics, rocks and brush." if kind == "aram" else "A new map: draw lanes from the base to the river line, then the jungle."


func _maps_dir() -> String:
	DirAccess.make_dir_recursive_absolute(MAPS_DIR)
	return ProjectSettings.globalize_path(MAPS_DIR)


func _ask_file(mode: FileDialog.FileMode, filters: Array, then: Callable) -> void:
	if file_dialog != null:
		file_dialog.queue_free()
	file_dialog = FileDialog.new()
	file_dialog.file_mode = mode
	file_dialog.access = FileDialog.ACCESS_FILESYSTEM
	file_dialog.use_native_dialog = false
	file_dialog.filters = PackedStringArray(filters)
	file_dialog.current_dir = _maps_dir()
	if mode == FileDialog.FILE_MODE_SAVE_FILE:
		file_dialog.current_file = "%s.mftrmap" % str(spec.name).to_snake_case()
	file_dialog.file_selected.connect(then)
	add_child(file_dialog)
	file_dialog.popup_centered_ratio(0.6)


func _open_dialog() -> void:
	_ask_file(FileDialog.FILE_MODE_OPEN_FILE, ["*.mftrmap ; MFTR maps"], func(path: String):
		_open_text(FileAccess.get_file_as_string(path), path))


func _write(path: String) -> bool:
	var f := FileAccess.open(path, FileAccess.WRITE)
	if f == null:
		status_label.text = "Couldn't write %s." % path
		return false
	f.store_string(to_text())
	f.close()
	return true


func _save() -> void:
	if file_path == "":
		_save_as()
		return
	if _write(file_path):
		dirty = false
		_rebuild_now()
		status_label.text = "Saved %s." % file_path.get_file()


func _save_as() -> void:
	_ask_file(FileDialog.FILE_MODE_SAVE_FILE, ["*.mftrmap ; MFTR maps"], func(path: String):
		file_path = path
		_save())


func _export() -> void:
	_rebuild_now()
	if not problems.is_empty():
		status_label.text = "Fix the problems listed above before exporting (save keeps your work meanwhile)."
		return
	_ask_file(FileDialog.FILE_MODE_SAVE_FILE, ["*.mftrmap ; MFTR maps"], func(path: String):
		if _write(path):
			status_label.text = "Exported %s: it can be played with  mftr-server --scenario classic --map %s" % [path.get_file(), path.get_file()])


# ---- the Actual view -------------------------------------------------------------------------

func _show_actual(on: bool) -> void:
	if on and view_box == null:
		view_box = SubViewportContainer.new()
		view_box.stretch = true
		view_box.mouse_filter = Control.MOUSE_FILTER_IGNORE
		view_box.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
		canvas.add_child(view_box)
		view_port = SubViewport.new()
		view_port.own_world_3d = true
		view_port.msaa_3d = Viewport.MSAA_4X
		view_box.add_child(view_port)
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
		e.adjustment_enabled = true
		e.adjustment_contrast = 1.08
		e.adjustment_saturation = 1.18
		env.environment = e
		view_port.add_child(env)
		var sun := DirectionalLight3D.new()
		sun.rotation_degrees = Vector3(-52, -38, 0)
		sun.light_color = Color(1.0, 0.94, 0.84)
		sun.light_energy = 1.25
		sun.shadow_enabled = true
		sun.shadow_opacity = 0.72
		sun.directional_shadow_max_distance = 200.0
		view_port.add_child(sun)
		view_env = e
		view_sun = sun
		view_cam = Camera3D.new()
		view_cam.far = 400.0
		view_port.add_child(view_cam)
	if view_box == null:
		return
	view_box.visible = on
	if on:
		# Outside a match nothing is fogged.
		RenderingServer.global_shader_parameter_set("fog_on", 0.0)
		if view_stale:
			_build_view()
		_place_view_camera()
	canvas.queue_redraw()


## The game's own map from the current spec: ground, roads, river, cliffs, brush, scenery and
## fountains (`map_view.gd`, as in a match), with the structures and camps on it.
func _build_view() -> void:
	view_rebuild_in = 0.0
	if not built.get("ok", false):
		return
	if view_root != null:
		view_root.queue_free()
	view_root = Node3D.new()
	view_port.add_child(view_root)
	var s := float(spec.size)
	var fountains: PackedVector2Array = built.fountains
	var season: String = built.get("season", "summer")
	Seasons.apply_light(Seasons.get_season(season), view_env, view_sun)
	var geo := {
		"walls": built.walls, "brush": built.brush, "size": Vector2(s, s), "roads": built.lanes, "river": built.river, "ground": built.get("ground"),
		"season": season,
		"fountains": [{"center": fountains[0], "radius": float(built.get("fountain_radius", 750.0)), "ally": true}, {"center": fountains[1], "radius": float(built.get("fountain_radius", 750.0)), "ally": false}],
	}
	var ground := MapView.make_ground()
	view_root.add_child(ground)
	var world: Node3D = MapView.new()
	view_root.add_child(world)
	world.build(geo, ground, ALLY, ENEMY)
	var segs: PackedVector4Array = world.road_segments
	for st in built.structures:
		var blue: bool = st.team == "blue"
		var color := ALLY if blue else ENEMY
		var kind: String = st.kind
		var node: Node3D = MapView.prop_node(kind, color)
		if node == null:
			node = _stand_in(kind, color)
		var p: Vector2 = st.pos
		node.position = Vector3(p.x, 0, p.y) * MapView.U
		# Facing along its lane, away from its own base.
		var home: Vector2 = fountains[0] if blue else fountains[1]
		var dir := _road_dir(segs, p)
		if dir.dot(p - home) < 0.0:
			dir = -dir
		node.rotation.y = atan2(dir.x, dir.y)
		view_root.add_child(node)
		world.add_glows(node, kind, color)
	for rp in (built.get("relics", PackedVector2Array()) as PackedVector2Array):
		var orb := MeshInstance3D.new()
		var sphere := SphereMesh.new()
		sphere.radius = 0.45
		sphere.height = 0.9
		orb.mesh = sphere
		var om := StandardMaterial3D.new()
		om.albedo_color = Color(0.35, 0.85, 0.45)
		om.emission_enabled = true
		om.emission = Color(0.2, 0.6, 0.3)
		orb.material_override = om
		orb.position = Vector3(rp.x * MapView.U, 0.8, rp.y * MapView.U)
		view_root.add_child(orb)
	for cp in built.camps:
		var info: Array = _camp_info(cp.kind)
		var marker := MeshInstance3D.new()
		var cap := CapsuleMesh.new()
		cap.radius = 0.55 if cp.kind in ["warden", "brute"] else 0.4
		cap.height = cap.radius * 2.6
		marker.mesh = cap
		var m := StandardMaterial3D.new()
		m.albedo_color = info[3]
		marker.material_override = m
		var p: Vector2 = cp.pos
		marker.position = Vector3(p.x * MapView.U, cap.height / 2.0, p.y * MapView.U)
		view_root.add_child(marker)
	view_stale = false


## A plain shape where a structure's prop pack is missing.
func _stand_in(kind: String, color: Color) -> Node3D:
	var n := MeshInstance3D.new()
	var cyl := CylinderMesh.new()
	cyl.top_radius = {"base": 2.0, "gatehouse": 1.6}.get(kind, 0.7)
	cyl.bottom_radius = cyl.top_radius * 1.1
	cyl.height = {"base": 2.0, "gatehouse": 2.4}.get(kind, 3.2)
	n.mesh = cyl
	var m := StandardMaterial3D.new()
	m.albedo_color = color
	n.material_override = m
	n.position.y = cyl.height / 2.0
	var root := Node3D.new()
	root.add_child(n)
	return root


func _road_dir(segs: PackedVector4Array, p: Vector2) -> Vector2:
	var m := p * MapView.U
	var best := INF
	var dir := Vector2(1, 0)
	for sg in segs:
		var a := Vector2(sg.x, sg.y)
		var b := Vector2(sg.z, sg.w)
		var d := m.distance_to(Geometry2D.get_closest_point_to_segment(m, a, b))
		if d < best and a != b:
			best = d
			dir = (b - a).normalized()
	return dir


## The 3D camera frames what the canvas frames: straight down (orthographic), or from the game's
## angle (perspective) at the canvas's center.
func _place_view_camera() -> void:
	if view_cam == null:
		return
	var center := to_map(canvas.size / 2.0) * MapView.U
	var span := canvas.size.y / _scale() * MapView.U
	if angle_check.button_pressed:
		view_cam.projection = Camera3D.PROJECTION_PERSPECTIVE
		view_cam.fov = 45.0
		var pitch := deg_to_rad(56.0)
		var dist := span * 0.9
		view_cam.position = Vector3(center.x, sin(pitch) * dist, center.y + cos(pitch) * dist)
		view_cam.look_at(Vector3(center.x, 0, center.y), Vector3.UP)
	else:
		view_cam.projection = Camera3D.PROJECTION_ORTHOGONAL
		view_cam.size = span
		view_cam.position = Vector3(center.x, 120.0, center.y)
		view_cam.rotation_degrees = Vector3(-90, 0, 0)


func _close() -> void:
	_save_config()
	closed.emit()
	queue_free()


# ---- the reference image ---------------------------------------------------------------------

func _load_overlay(path: String) -> void:
	var img := Image.load_from_file(path)
	if img == null or img.is_empty():
		status_label.text = "Couldn't load the image %s." % path
		return
	var full := Vector2(img.get_width(), img.get_height())
	if maxf(full.x, full.y) > 4096.0:
		var k := 4096.0 / maxf(full.x, full.y)
		img.resize(int(full.x * k), int(full.y * k), Image.INTERPOLATE_LANCZOS)
	overlay_tex = ImageTexture.create_from_image(img)
	var same := path == overlay_path and overlay_rect.size != Vector2.ZERO
	overlay_path = path
	if not same:
		overlay_rect = _default_overlay_rect(full)
	overlay_shown = true
	overlay_check.button_pressed = true
	_save_config()
	canvas.queue_redraw()
	status_label.text = "Reference image loaded. Use Align image to line its corners up with the map."


## Where a fresh reference image goes: stretched over the map. The reference 5v5 render the
## developers trace from (12,200 × 10,240) has its playable square inside a border of scenery,
## measured once, so it starts aligned.
func _default_overlay_rect(full: Vector2) -> Rect2:
	var s := float(spec.size)
	if full == Vector2(12200, 10240):
		var play := Rect2(1525, 1281, 9943, 8357)
		var k := Vector2(s / play.size.x, s / play.size.y)
		return Rect2(-play.position * k, full * k)
	return Rect2(Vector2.ZERO, Vector2(s, s))


func _save_config() -> void:
	var cfg := ConfigFile.new()
	cfg.set_value("overlay", "path", overlay_path)
	cfg.set_value("overlay", "rect", overlay_rect)
	cfg.set_value("overlay", "alpha", overlay_alpha)
	cfg.save(CONFIG)


func _load_config() -> void:
	var cfg := ConfigFile.new()
	if cfg.load(CONFIG) != OK:
		return
	overlay_alpha = cfg.get_value("overlay", "alpha", 0.5)
	overlay_slider.set_value_no_signal(overlay_alpha)
	overlay_rect = cfg.get_value("overlay", "rect", Rect2())
	var path: String = cfg.get_value("overlay", "path", "")
	if path != "" and FileAccess.file_exists(path):
		overlay_path = path
		_load_overlay(path)
