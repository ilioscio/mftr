extends SceneTree
## Checks `splashes.txt` (every line parses, spells MFTR, appears once) and the holiday dates:
## godot --headless --path client --script res://tests/splashes_test.gd
const Splashes = preload("res://scripts/splashes.gd")
var fails := 0

func check(ok: bool, what: String) -> void:
	if not ok:
		print("FAIL  " + what)
		fails += 1

func _init() -> void:
	var parsed := Splashes.parse(FileAccess.get_file_as_string(Splashes.PATH))
	for e in parsed.errors:
		check(false, e)
	var all: Array[String] = parsed.everyday.duplicate()
	for h in parsed.holidays:
		all.append(h[3])
	var seen := {}
	for line in all:
		check(Splashes.initials(line) == "MFTR", "%s spells %s" % [line, Splashes.initials(line)])
		check(not seen.has(line), "%s is in the file twice" % line)
		seen[line] = true
	print("%d everyday lines, %d holiday lines" % [parsed.everyday.size(), parsed.holidays.size()])

	var day := func(y: int, m: int, d: int) -> Array[String]:
		return Splashes.for_day(parsed, {"year": y, "month": m, "day": d})
	check(day.call(2026, 3, 3) == parsed.everyday, "an ordinary day rolls the everyday lines")
	check("Monsters Fill The River" in day.call(2026, 10, 31), "Halloween")
	check(day.call(2026, 11, 3) == parsed.everyday, "Halloween and the Day of the Dead are over")
	check("My Fourteen Terrible Resolutions" in day.call(2027, 1, 1), "a range that wraps the new year")
	check("Mountain goats Fear The Ram" in day.call(2027, 2, 7), "a dated holiday in its year")
	check(day.call(2028, 2, 7) == parsed.everyday, "a dated holiday not in other years")

	var odd := Splashes.parse("[13-01] Bad Month\n[02-30..2027-03-01] Mixed Kinds\n[1-2-3-4] Too Many\nok line\n")
	check(odd.errors.size() == 3 and odd.everyday == ["ok line"] and odd.holidays.is_empty(), "bad dates are skipped")
	check(Splashes.initials("Mid's Fed, Team Rejoices") == "MFTR", "initials skip lowercase words and punctuation")

	print("PASS" if fails == 0 else "%d FAILED" % fails)
	quit(1 if fails > 0 else 0)
