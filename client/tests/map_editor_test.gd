extends SceneTree
## The map editor's tools, run headless against the real editor and MftrMaps (12):
##   godot --headless --path client --script res://tests/map_editor_test.gd
## Prints PASS/FAIL per check and exits 1 on any failure.
var fails := 0
func check(ok: bool, what: String) -> void:
	print(("PASS  " if ok else "FAIL  ") + what)
	if not ok:
		fails += 1
func _init() -> void:
	root.size = Vector2i(1600, 900)
	var ed: Control = load("res://scripts/map_editor.gd").new()
	root.add_child(ed)
	ed.set_anchors_preset(Control.PRESET_TOP_LEFT)
	ed.size = Vector2(1600, 900)
	await process_frame
	await process_frame
	await process_frame
	print("canvas size ", ed.canvas.size)
	ed._new_map()
	check(ed.spec.lane.size() == 0, "new map is empty")
	var S: float = ed.spec.size
	ed._set_tool("lane")
	ed._press(Vector2(2000, S - 2000), false, false)
	ed._press(Vector2(7000, 7300), false, false)
	ed._finish_line()
	check(ed.spec.lane.size() == 1, "a lane is drawn")
	var last: Vector2 = ed.spec.lane[0].points[1]
	check(absf(last.x - last.y) < 1.0, "the lane's end snaps onto the river line")
	ed._set_tool("camp")
	ed.camp_kind = "brute"
	ed._press(Vector2(9000, 5000), false, false)
	var cp: Vector2 = ed.spec.camps[0].pos
	check(cp == Vector2(S - 9000, S - 5000), "a camp on red's half is turned onto blue's")
	ed._set_tool("objective")
	ed._press(Vector2(4000, 3500), false, false)
	check(ed.spec.objectives.size() == 2 and ed.spec.objectives[1] == Vector2(S, S) - Vector2(4000, 3500), "an objective comes with its twin")
	ed._rebuild_now()
	check(ed.built.get("ok", false), "the map builds")
	var turret := Vector2.ZERO
	for st in ed.built.structures:
		if st.team == "blue" and int(st.tier) == 1:
			turret = st.pos
	ed._set_tool("select")
	ed._press(turret, false, false)
	check(ed.selected.get("what", "") == "structure", "a turret is picked")
	ed._drag_to(Vector2(3000, S - 3000))
	ed.dragging = false
	check(ed.spec.structures.size() == 1 and int(ed.spec.structures[0].tier) == 1, "dragging a turret records its spot")
	ed._press(ed.spec.camps[0].pos, false, false)
	ed._delete_selected(false)
	check(ed.spec.camps.size() == 0, "Delete removes the camp")
	ed._undo()
	check(ed.spec.camps.size() == 1, "Ctrl+Z brings it back")
	var text: String = ed.to_text()
	var back: Dictionary = MftrMaps.parse(text)
	check(back.ok and back.lane.size() == 1 and back.camps.size() == 1 and back.objectives.size() == 2 and back.structures.size() == 1, "the text reads back")
	var probs: PackedStringArray = MftrMaps.problems(text)
	check(probs.size() > 0, "a half-made map has problems: %s" % " | ".join(probs))
	ed._open_text(MftrMaps.crossroads(), "")
	check(MftrMaps.problems(ed.to_text()).is_empty(), "Crossroads opens without problems and writes back the same")
	# Load default map: every built-in map opens without problems.
	var opened := 0
	for m in ed.DEFAULT_MAPS:
		ed.open_map(m[0])
		if ed.spec.name == m[1] and MftrMaps.problems(ed.to_text()).is_empty():
			opened += 1
	check(opened == ed.DEFAULT_MAPS.size(), "every default map loads from the dropdown (%d of %d)" % [opened, ed.DEFAULT_MAPS.size()])
	# ARAM maps (12 §1): The Bridge opens as one, relics and rocks round-trip.
	ed._open_text(MftrMaps.bridge(), "")
	check(ed.spec.kind == "aram" and ed.spec.relics.size() == 2 and ed.spec.rocks.size() == 2 and ed.spec.grass.size() == 6, "The Bridge opens as an ARAM map with its relics, rocks and brush")
	check(MftrMaps.problems(ed.to_text()).is_empty(), "The Bridge writes back without problems")
	check(ed.built.has("base_court") and ed.built.relics.size() == 4 and float(ed.built.fountain_radius) == 600.0, "its base court, four relics and small fountains are drawn")
	check(ed.tool_buttons.camp.disabled and not ed.tool_buttons.relic.disabled, "ARAM maps have no camp tool but a relic tool")
	var B: float = ed.spec.size
	ed._set_tool("relic")
	ed._press(Vector2(B - 4000, B - 6000), false, false)
	check(ed.spec.relics.size() == 3 and ed.spec.relics[2] == Vector2(4000, 6000), "a relic on red's half is turned onto blue's")
	ed._set_tool("rock")
	ed._press(Vector2(4200, 5800), false, false)
	ed._rebuild_now()
	var back2: Dictionary = MftrMaps.parse(ed.to_text())
	check(back2.ok and back2.kind == "aram" and back2.relics.size() == 3 and back2.rocks.size() == 3, "relics and rocks read back")
	ed._new_map()
	check(ed.spec.kind == "aram", "a new map keeps the kind")
	ed._set_kind("classic")
	check(ed.spec.kind == "classic" and float(ed.spec.size) in Array(MftrMaps.sizes("classic")) and not ed.tool_buttons.camp.disabled and ed.tool_buttons.relic.disabled, "switching to Classic picks a Classic size and its tools")
	print("FAILS: %d" % fails)
	quit(1 if fails > 0 else 0)
