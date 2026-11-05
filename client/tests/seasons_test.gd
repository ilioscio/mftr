extends SceneTree
## Checks the season by the calendar (15 §9, K slice 5): each month in each hemisphere, and how
## the Season setting resolves:
## godot --headless --path client --script res://tests/seasons_test.gd
const Seasons = preload("res://scripts/seasons.gd")
var fails := 0

func check(ok: bool, what: String) -> void:
	if not ok:
		print("FAIL  " + what)
		fails += 1

func _init() -> void:
	var north := ["winter", "winter", "summer", "summer", "summer", "summer", "summer", "summer", "autumn", "autumn", "autumn", "winter"]
	var south := ["summer", "summer", "autumn", "autumn", "autumn", "winter", "winter", "winter", "summer", "summer", "summer", "summer"]
	for m in 12:
		var date := {"year": 2026, "month": m + 1, "day": 15}
		check(Seasons.for_date(date, "north") == north[m], "month %d in the north is %s" % [m + 1, north[m]])
		check(Seasons.for_date(date, "south") == south[m], "month %d in the south is %s" % [m + 1, south[m]])
	var nov := {"year": 2026, "month": 11, "day": 4}
	check(Seasons.resolve("calendar", "summer", nov, "north") == "autumn", "the calendar: November is autumn in the north")
	check(Seasons.resolve("calendar", "summer", nov, "south") == "summer", "the calendar: November is summer in the south")
	check(Seasons.resolve("map", "winter", nov, "north") == "winter", "the map's own")
	check(Seasons.resolve("map", "", nov, "north") == "summer", "a map without a season is summer")
	check(Seasons.resolve("autumn", "winter", nov, "south") == "autumn", "a season picked")
	check(Seasons.default_hemisphere() in ["north", "south"], "the locale gives a hemisphere")
	print("seasons: %s" % ("ok" if fails == 0 else "%d failed" % fails))
	quit(1 if fails > 0 else 0)
