extends SceneTree
## Checks the graphics presets (05 §6, D81): picking one sets its values, changing one makes it
## Custom, and settings from before the presets carry over (the shadows' old on/off; untouched
## graphics get the guess for this machine):
## godot --headless --path client --script res://tests/graphics_settings_test.gd
const Settings = preload("res://scripts/settings.gd")
var fails := 0

func check(ok: bool, what: String) -> void:
	if not ok:
		print("FAIL  " + what)
		fails += 1

## Settings loaded from a config file of `values` ({section: {key: value}}), not the player's.
func loaded(values: Dictionary) -> RefCounted:
	var path := OS.get_temp_dir().path_join("mftr_graphics_settings_test.cfg")
	var cfg := ConfigFile.new()
	for section in values:
		for key in values[section]:
			cfg.set_value(section, key, values[section][key])
	cfg.save(path)
	var s := Settings.new()
	s.load_file(path)
	DirAccess.remove_absolute(path)
	return s

func _init() -> void:
	var s := Settings.new()
	for preset in s.PRESETS:
		check(preset in s.PRESET_NAMES, "%s is offered" % preset)
		s.apply_preset(preset)
		check(s.graphics_preset == preset, "picking %s selects it" % preset)
		check(s.matching_preset() == preset, "%s's values are %s's alone" % [preset, preset])
		var p: Dictionary = s.PRESETS[preset]
		check(p.shadow_quality in s.SHADOW_QUALITIES and p.world_detail in s.WORLD_DETAILS and p.antialias in s.ANTIALIAS,
			"%s's choices are ones the settings offer" % preset)
		check(p.render_scale >= s.RENDER_SCALE_MIN and p.render_scale <= s.RENDER_SCALE_MAX, "%s's render scale is in range" % preset)
	s.apply_preset("medium")
	s.world_detail = "high"
	check(s.matching_preset() == "custom", "changing a preset's setting makes it custom")
	s.apply_preset("custom")
	check(s.graphics_preset == "custom" and s.world_detail == "high", "choosing Custom keeps the values")
	# Costs go down the presets.
	var order := ["ultra", "high", "medium", "low"]
	for i in order.size() - 1:
		var a: Dictionary = s.PRESETS[order[i]]
		var b: Dictionary = s.PRESETS[order[i + 1]]
		check(b.render_scale <= a.render_scale, "%s renders no more pixels than %s" % [order[i + 1], order[i]])
		check(s.SHADOW_QUALITIES.find(b.shadow_quality) >= s.SHADOW_QUALITIES.find(a.shadow_quality), "%s's shadows cost no more than %s's" % [order[i + 1], order[i]])

	# From before the presets (every graphics key written, as 0.22 saved them; Settings.new()
	# has already read the player's own file, which mustn't show through).
	var old := {"window_mode": "borderless", "antialias": "msaa4", "render_scale": 100.0, "shadows": true, "bloom": true,
		"atmosphere": true, "hurt_edges": true, "season": "calendar", "hemisphere": ""}
	# Shadows off, the rest the old defaults: the player's own, kept as Custom.
	var off := old.duplicate()
	off.shadows = false
	var old_off := loaded({"graphics": off})
	check(old_off.shadow_quality == "off", "the old shadows off is Shadows: Off")
	check(old_off.graphics_preset == "custom", "changed graphics from before the presets are Custom")
	# Untouched graphics from before the presets: the guess for this machine.
	var old_default := loaded({"graphics": old, "camera": {"locked": true}})
	check(old_default.graphics_preset == Settings.detect_preset(), "untouched graphics get this machine's guess")
	check(old_default.camera_locked, "the rest of the settings load as before")
	# A chosen preset stays chosen; a frame limit not on the list is unlimited.
	var chosen := loaded({"graphics": {"preset": "low", "render_scale": 75.0, "antialias": "fxaa", "shadow_quality": "off",
		"world_detail": "low", "bloom": false, "atmosphere": false, "fps_limit": 77}})
	check(chosen.graphics_preset == "low" and chosen.world_detail == "low", "a saved preset loads")
	check(chosen.fps_limit == 0, "an unknown frame limit is unlimited")
	check(Settings.detect_preset() in ["high", "medium", "low"], "the guess is a preset")
	print("graphics settings: %s" % ("ok" if fails == 0 else "%d failed" % fails))
	quit(1 if fails > 0 else 0)
