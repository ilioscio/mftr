extends RefCounted
## The client's UI theme (05 §6): dark ink panels with a thin gold rim, slate buttons that warm
## to gold on hover, recessed fields. Applied at the window root, so every menu, panel and
## popup shares it; the in-game HUD draws with the same colors (hud.gd).

const INK := Color(0.035, 0.045, 0.06, 0.96)
const INK_2 := Color(0.07, 0.085, 0.11)
const SLATE := Color(0.11, 0.13, 0.16)
const SLATE_HI := Color(0.16, 0.18, 0.22)
const RIM := Color(0.42, 0.35, 0.2)
const GOLD := Color(0.78, 0.65, 0.38)
const CREAM := Color(0.95, 0.88, 0.7)
const TEXT := Color(0.86, 0.88, 0.91)
const DIM := Color(0.55, 0.6, 0.66)


static func _box(fill: Color, border := Color(0, 0, 0, 0), width := 0, radius := 4, margin := Vector4(10, 6, 10, 6)) -> StyleBoxFlat:
	var sb := StyleBoxFlat.new()
	sb.bg_color = fill
	sb.border_color = border
	sb.set_border_width_all(width)
	sb.set_corner_radius_all(radius)
	sb.content_margin_left = margin.x
	sb.content_margin_top = margin.y
	sb.content_margin_right = margin.z
	sb.content_margin_bottom = margin.w
	sb.anti_aliasing = true
	return sb


static func build() -> Theme:
	var t := Theme.new()
	t.default_font = ThemeDB.fallback_font
	t.default_font_size = 16
	var bold := FontVariation.new()
	bold.base_font = ThemeDB.fallback_font
	bold.variation_embolden = 0.7

	# Panels: ink with a gold rim and a soft shadow.
	var panel := _box(INK, RIM, 1, 7, Vector4(0, 0, 0, 0))
	panel.shadow_color = Color(0, 0, 0, 0.5)
	panel.shadow_size = 12
	t.set_stylebox("panel", "PanelContainer", panel)
	t.set_stylebox("panel", "Panel", panel)

	# Labels.
	t.set_color("font_color", "Label", TEXT)
	t.set_color("font_shadow_color", "Label", Color(0, 0, 0, 0))
	# Type variations: titles in cream bold, headers in gold, hints dim.
	t.set_type_variation("TitleLabel", "Label")
	t.set_font("font", "TitleLabel", bold)
	t.set_font_size("font_size", "TitleLabel", 26)
	t.set_color("font_color", "TitleLabel", CREAM)
	t.set_type_variation("HeaderLabel", "Label")
	t.set_font("font", "HeaderLabel", bold)
	t.set_font_size("font_size", "HeaderLabel", 14)
	t.set_color("font_color", "HeaderLabel", GOLD)
	t.set_type_variation("HintLabel", "Label")
	t.set_font_size("font_size", "HintLabel", 13)
	t.set_color("font_color", "HintLabel", DIM)

	# Buttons: slate; gold rim on hover; pressed sinks darker with a gold rim.
	for type in ["Button", "OptionButton", "MenuButton", "CheckButton", "CheckBox"]:
		var flat: bool = type in ["CheckButton", "CheckBox"]
		t.set_stylebox("normal", type, _box(Color(0, 0, 0, 0) if flat else SLATE, Color(0.24, 0.26, 0.3) if not flat else Color(0, 0, 0, 0), 0 if flat else 1))
		t.set_stylebox("hover", type, _box(Color(1, 1, 1, 0.04) if flat else SLATE_HI, GOLD, 0 if flat else 1))
		t.set_stylebox("pressed", type, _box(Color(0.05, 0.06, 0.08), GOLD, 1))
		t.set_stylebox("hover_pressed", type, _box(Color(0.05, 0.06, 0.08), CREAM, 1))
		t.set_stylebox("disabled", type, _box(Color(0.07, 0.08, 0.1, 0.6) if not flat else Color(0, 0, 0, 0), Color(0.16, 0.17, 0.2), 0 if flat else 1))
		t.set_stylebox("focus", type, _box(Color(0, 0, 0, 0), Color(GOLD, 0.6), 1))
		t.set_color("font_color", type, TEXT)
		t.set_color("font_hover_color", type, Color.WHITE)
		t.set_color("font_pressed_color", type, CREAM)
		t.set_color("font_hover_pressed_color", type, CREAM)
		t.set_color("font_focus_color", type, TEXT)
		t.set_color("font_disabled_color", type, Color(0.4, 0.42, 0.46))
	# A gold "primary" button for the main action on a screen.
	t.set_type_variation("PrimaryButton", "Button")
	t.set_stylebox("normal", "PrimaryButton", _box(Color(0.36, 0.27, 0.09), GOLD, 1, 4, Vector4(16, 8, 16, 8)))
	t.set_stylebox("hover", "PrimaryButton", _box(Color(0.48, 0.36, 0.12), CREAM, 1, 4, Vector4(16, 8, 16, 8)))
	t.set_stylebox("pressed", "PrimaryButton", _box(Color(0.26, 0.19, 0.06), GOLD, 1, 4, Vector4(16, 8, 16, 8)))
	t.set_font("font", "PrimaryButton", bold)
	t.set_color("font_color", "PrimaryButton", CREAM)
	t.set_color("font_hover_color", "PrimaryButton", Color.WHITE)

	# Text fields: recessed.
	t.set_stylebox("normal", "LineEdit", _box(Color(0.02, 0.025, 0.035), Color(0.24, 0.26, 0.3), 1, 4, Vector4(10, 7, 10, 7)))
	t.set_stylebox("focus", "LineEdit", _box(Color(0, 0, 0, 0), GOLD, 1, 4))
	t.set_stylebox("read_only", "LineEdit", _box(Color(0.05, 0.06, 0.08), Color(0.16, 0.17, 0.2), 1, 4, Vector4(10, 7, 10, 7)))
	t.set_color("font_color", "LineEdit", Color.WHITE)
	t.set_color("font_placeholder_color", "LineEdit", Color(0.42, 0.45, 0.5))
	t.set_color("caret_color", "LineEdit", CREAM)
	t.set_color("selection_color", "LineEdit", Color(GOLD, 0.35))

	# Tabs.
	t.set_stylebox("panel", "TabContainer", _box(Color(0, 0, 0, 0), Color(0, 0, 0, 0), 0, 0, Vector4(4, 10, 4, 4)))
	t.set_stylebox("tab_selected", "TabContainer", _box(INK_2, GOLD, 0, 4, Vector4(14, 6, 14, 6)))
	(t.get_stylebox("tab_selected", "TabContainer") as StyleBoxFlat).border_width_bottom = 2
	t.set_stylebox("tab_unselected", "TabContainer", _box(Color(0, 0, 0, 0), Color(0, 0, 0, 0), 0, 4, Vector4(14, 6, 14, 6)))
	t.set_stylebox("tab_hovered", "TabContainer", _box(Color(1, 1, 1, 0.04), Color(0, 0, 0, 0), 0, 4, Vector4(14, 6, 14, 6)))
	t.set_color("font_selected_color", "TabContainer", CREAM)
	t.set_color("font_unselected_color", "TabContainer", DIM)
	t.set_color("font_hovered_color", "TabContainer", TEXT)

	# Popups, menus and tooltips.
	var pop := _box(INK, RIM, 1, 5, Vector4(6, 6, 6, 6))
	t.set_stylebox("panel", "PopupMenu", pop)
	t.set_stylebox("hover", "PopupMenu", _box(SLATE_HI, Color(0, 0, 0, 0), 0, 3))
	t.set_color("font_color", "PopupMenu", TEXT)
	t.set_color("font_hover_color", "PopupMenu", Color.WHITE)
	t.set_stylebox("panel", "TooltipPanel", _box(INK, RIM, 1, 4, Vector4(10, 6, 10, 6)))
	t.set_color("font_color", "TooltipLabel", TEXT)

	# Sliders: a thin dark track, gold fill.
	t.set_stylebox("slider", "HSlider", _box(Color(0.02, 0.025, 0.035), Color(0.24, 0.26, 0.3), 1, 3, Vector4(0, 3, 0, 3)))
	t.set_stylebox("grabber_area", "HSlider", _box(Color(GOLD, 0.75), Color(0, 0, 0, 0), 0, 3, Vector4(0, 3, 0, 3)))
	t.set_stylebox("grabber_area_highlight", "HSlider", _box(GOLD, Color(0, 0, 0, 0), 0, 3, Vector4(0, 3, 0, 3)))

	# Scrollbars: slim.
	t.set_stylebox("scroll", "VScrollBar", _box(Color(0, 0, 0, 0.25), Color(0, 0, 0, 0), 0, 3, Vector4(3, 0, 3, 0)))
	t.set_stylebox("grabber", "VScrollBar", _box(Color(0.3, 0.32, 0.36), Color(0, 0, 0, 0), 0, 3, Vector4(3, 0, 3, 0)))
	t.set_stylebox("grabber_highlight", "VScrollBar", _box(GOLD, Color(0, 0, 0, 0), 0, 3, Vector4(3, 0, 3, 0)))
	t.set_stylebox("grabber_pressed", "VScrollBar", _box(CREAM, Color(0, 0, 0, 0), 0, 3, Vector4(3, 0, 3, 0)))
	var line := StyleBoxLine.new()
	line.color = Color(RIM, 0.7)
	line.thickness = 1
	t.set_stylebox("separator", "HSeparator", line)
	t.set_constant("separation", "HSeparator", 12)
	return t
