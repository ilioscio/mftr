extends Control
## The launch screen while champions are forged (D78): the official champions are built from
## their recipes on this computer, not shipped, and kept in the player's data folder. They're
## rebuilt only after an update (new code or art) or a changed recipe, which takes seconds; this
## covers the menu while that happens, so nothing half-built is clickable.
##
## Shown only when a champion actually needs forging (not when every one comes back from the
## cache in a blink), after a short delay so a fast run never flashes it, then held long enough
## to read, and faded out when the last is done. Each champion is a tile in its identity color:
## dim while waiting, a breathing gold rim while forging, lit with its portrait once done.

signal finished

const UI := preload("res://scripts/ui_theme.gd")
const SHOW_AFTER := 0.25                 # s of forging before the screen shows
const HOLD := 0.8                        # s it stays at least, once shown
const FADE := 0.35
const TILE := Vector2(76, 96)

var art_root := ""
var portraits: Node                      # portraits.gd: busts appear as each one is done
var colors := {}                         # champion -> identity color
var _names: Array = []
var _tiles := {}                         # champion -> {panel, style, initial, face, name}
var _bar: ColorRect
var _bar_back: ColorRect
var _count: Label
var _t := 0.0
var _shown_at := -1.0
var _leaving := false


## Watch these champions' forges (already started).
func watch(names: Array) -> void:
	_names = names.filter(func(n): return MftrModel.forge_state(art_root, n) != 0)
	set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	modulate.a = 0.0
	visible = false
	_build()


func _build() -> void:
	# The menu dims behind a deep ink wash, darker at the edges.
	var wash := ColorRect.new()
	wash.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	wash.color = Color(0.02, 0.025, 0.035, 0.985)
	add_child(wash)
	var vignette := TextureRect.new()
	vignette.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	vignette.stretch_mode = TextureRect.STRETCH_SCALE
	vignette.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	var g := GradientTexture2D.new()
	g.fill = GradientTexture2D.FILL_RADIAL
	g.fill_from = Vector2(0.5, 0.45)
	g.fill_to = Vector2(1.05, 1.05)
	var grad := Gradient.new()
	grad.set_color(0, Color(0.10, 0.085, 0.05, 0.55))   # a faint warm glow behind the tiles
	grad.set_color(1, Color(0, 0, 0, 0.6))
	g.gradient = grad
	vignette.texture = g
	add_child(vignette)

	var col := VBoxContainer.new()
	col.set_anchors_and_offsets_preset(Control.PRESET_CENTER)
	col.grow_horizontal = Control.GROW_DIRECTION_BOTH
	col.grow_vertical = Control.GROW_DIRECTION_BOTH
	col.alignment = BoxContainer.ALIGNMENT_CENTER
	col.add_theme_constant_override("separation", 0)
	add_child(col)

	var header := Label.new()
	header.text = "P R E P A R I N G   T H E   C H A M P I O N S"
	header.theme_type_variation = "HeaderLabel"
	header.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	col.add_child(header)
	_gap(col, 6)
	var title := Label.new()
	title.text = "Forging the roster"
	title.theme_type_variation = "TitleLabel"
	title.add_theme_font_size_override("font_size", 34)
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	col.add_child(title)
	_gap(col, 30)

	var row := HBoxContainer.new()
	row.alignment = BoxContainer.ALIGNMENT_CENTER
	row.add_theme_constant_override("separation", 12)
	col.add_child(row)
	for n in _names:
		row.add_child(_tile(n))
	_gap(col, 30)

	# The bar: a recessed slate track, a gold fill, the count beside it.
	var bar_row := HBoxContainer.new()
	bar_row.alignment = BoxContainer.ALIGNMENT_CENTER
	bar_row.add_theme_constant_override("separation", 14)
	col.add_child(bar_row)
	var track := Control.new()
	track.custom_minimum_size = Vector2(420, 4)
	track.size_flags_vertical = Control.SIZE_SHRINK_CENTER
	bar_row.add_child(track)
	_bar_back = ColorRect.new()
	_bar_back.color = UI.SLATE
	_bar_back.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	track.add_child(_bar_back)
	_bar = ColorRect.new()
	_bar.color = UI.GOLD
	_bar.position = Vector2.ZERO
	_bar.size = Vector2(0, 4)
	track.add_child(_bar)
	_count = Label.new()
	_count.theme_type_variation = "HeaderLabel"
	_count.custom_minimum_size = Vector2(56, 0)
	bar_row.add_child(_count)
	_gap(col, 18)

	var hint := Label.new()
	hint.text = "Built once on this computer after each update, then kept. Only a few seconds."
	hint.theme_type_variation = "HintLabel"
	hint.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	col.add_child(hint)


func _gap(parent: Control, h: float) -> void:
	var c := Control.new()
	c.custom_minimum_size = Vector2(0, h)
	parent.add_child(c)


func _tile(champion: String) -> Control:
	var v := VBoxContainer.new()
	v.add_theme_constant_override("separation", 6)
	var panel := PanelContainer.new()
	panel.custom_minimum_size = TILE
	var style := StyleBoxFlat.new()
	style.bg_color = UI.INK_2
	style.border_color = UI.SLATE_HI
	style.set_border_width_all(1)
	style.border_width_bottom = 3
	style.set_corner_radius_all(5)
	style.anti_aliasing = true
	panel.add_theme_stylebox_override("panel", style)
	panel.clip_contents = true
	v.add_child(panel)
	var initial := Label.new()
	initial.text = champion.substr(0, 1).to_upper()
	initial.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	initial.vertical_alignment = VERTICAL_ALIGNMENT_CENTER
	var bold := FontVariation.new()
	bold.base_font = ThemeDB.fallback_font
	bold.variation_embolden = 0.9
	initial.add_theme_font_override("font", bold)
	initial.add_theme_font_size_override("font_size", 34)
	initial.add_theme_color_override("font_color", UI.DIM.darkened(0.3))
	panel.add_child(initial)
	var face := TextureRect.new()
	face.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
	face.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_COVERED
	face.modulate.a = 0.0
	panel.add_child(face)
	var caption := Label.new()
	caption.text = champion
	caption.theme_type_variation = "HintLabel"
	caption.add_theme_font_size_override("font_size", 12)
	caption.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	v.add_child(caption)
	_tiles[champion] = {"style": style, "initial": initial, "face": face, "name": caption, "done_t": -1.0}
	return v


func _process(delta: float) -> void:
	_t += delta
	var done := 0
	var building := false
	for n in _names:
		var s: int = MftrModel.forge_state(art_root, n)
		if s == 0 or s == 3:
			done += 1
		elif s == 2:
			building = true
		_paint(n, s)
	var share := float(done) / float(maxi(_names.size(), 1))
	_bar.size.x = lerpf(_bar.size.x, _bar_back.size.x * share, minf(1.0, delta * 10.0))
	_count.text = "%d / %d" % [done, _names.size()]

	if _shown_at < 0.0:
		if done == _names.size():
			_finish()          # everything came back from the cache: never shown
		elif building and _t >= SHOW_AFTER:
			_shown_at = _t
			visible = true
			mouse_filter = Control.MOUSE_FILTER_STOP
		return
	if not _leaving:
		modulate.a = minf(1.0, modulate.a + delta / FADE)
		if done == _names.size() and _t - _shown_at >= HOLD:
			_leaving = true
	else:
		modulate.a -= delta / FADE
		if modulate.a <= 0.0:
			_finish()


func _paint(champion: String, state: int) -> void:
	var tile: Dictionary = _tiles[champion]
	var style: StyleBoxFlat = tile.style
	var ident: Color = colors.get(champion, UI.GOLD)
	match state:
		2:
			# Forging: a gold rim breathing, the initial warming toward the champion's color.
			var p := 0.5 + 0.5 * sin(_t * 4.0 + float(_names.find(champion)) * 0.7)
			style.border_color = UI.RIM.lerp(UI.GOLD, p)
			style.bg_color = UI.INK_2.lerp(ident.darkened(0.7), 0.5)
			tile.initial.add_theme_color_override("font_color", UI.DIM.lerp(ident.lightened(0.2), 0.4 + 0.3 * p))
			tile.name.add_theme_color_override("font_color", UI.TEXT)
		0, 3:
			if tile.done_t < 0.0:
				tile.done_t = _t
			# Done: the champion's color in the rim's foot, the bust fading in over the initial.
			var k := clampf((_t - tile.done_t) / 0.3, 0.0, 1.0)
			style.border_color = UI.RIM
			style.border_width_bottom = 3
			style.bg_color = UI.INK_2.lerp(ident.darkened(0.45), k)
			tile.initial.add_theme_color_override("font_color", ident.lightened(0.35))
			tile.name.add_theme_color_override("font_color", UI.CREAM)
			var face: TextureRect = tile.face
			if face.texture == null and portraits != null:
				face.texture = portraits.portrait(champion, "bust")
			if face.texture != null:
				face.modulate.a = minf(1.0, face.modulate.a + get_process_delta_time() / 0.3)
				tile.initial.modulate.a = 1.0 - face.modulate.a
			style.border_color = UI.RIM.lerp(ident.lightened(0.2), 0.5 * k)
		_:
			style.border_color = UI.SLATE_HI
			style.bg_color = UI.INK_2


func _finish() -> void:
	finished.emit()
	queue_free()
