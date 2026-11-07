extends SceneTree
## Where the shops stand on every kind of map (19 §4), and the nooks cut for them: the stock maps
## at each size, and custom ones made to trip it (rocks where a stall would go, a jungle path
## just behind an ARAM court's wall). For every stall: its plinth stands clear of the cut cliffs,
## of boulders and of structures; a nook never opens onto ground the fountain can't see (the cut
## would show a way through that the simulation doesn't have); the cut cliffs still build.
##   godot --headless --path client --script res://tests/shop_spots_test.gd
const Shops = preload("res://scripts/shopkeepers.gd")
var fails := 0


func check(ok: bool, what: String) -> void:
	if not ok:
		print("FAIL  " + what)
		fails += 1


func _init() -> void:
	var maps: Array = []
	var crossroads: String = MftrMaps.crossroads()
	var bridge: String = MftrMaps.bridge()
	maps.append(["Crossroads", crossroads, true])
	maps.append(["The Bridge", bridge, true])
	for size in MftrMaps.sizes("classic"):
		maps.append(["Crossroads at %d" % size, _with_size(crossroads, size), false])
	for size in MftrMaps.sizes("aram"):
		maps.append(["The Bridge at %d" % size, _with_size(bridge, size), false])
	# Rocks right where the stalls would go (the map turns them for red).
	maps.append(["Crossroads, rocks by the fountain", crossroads + "\nrock 350,13300 180\nrock 300,13650 150\n", false])
	maps.append(["The Bridge, rocks by the fountain", bridge + "\nrock 1000,8700 160\nrock 760,9050 140\n", false])
	# A jungle path just outside the ARAM court, up and to the left of the fountain: a thin wall.
	maps.append(["The Bridge, a path behind the court", bridge + "\njungle 420 300,7400 300,9800 1000,10000\n", false])
	var placed := 0
	for m in maps:
		placed += _check_map(m[0], m[1], m[2])
	print("shop spots: %d stalls on %d maps, %s" % [placed, maps.size(), "ok" if fails == 0 else "%d failed" % fails])
	quit(1 if fails > 0 else 0)


func _with_size(text: String, size: float) -> String:
	var lines := text.split("\n")
	for i in lines.size():
		if lines[i].begins_with("size "):
			lines[i] = "size %d" % int(size)
	return "\n".join(lines)


## Checks one map's spots and cut; returns how many stalls it places.
func _check_map(name: String, text: String, stock: bool) -> int:
	var built: Dictionary = MftrMaps.build(text)
	check(built.get("ok", false), "%s builds" % name)
	if not built.get("ok", false):
		return 0
	var s := float(built.size)
	var radius := float(built.get("fountain_radius", 750.0))
	var fountains: Array = []
	for i in (built.fountains as PackedVector2Array).size():
		fountains.append({"center": built.fountains[i], "radius": radius, "ally": i == 0})
	var geo := {"walls": built.walls, "size": Vector2(s, s), "cliffs": built.cliffs, "structures": built.structures, "fountains": fountains}
	var spots: Array = Shops.spots(geo)
	check(spots.size() == fountains.size(), "%s: a spot per fountain" % name)
	var pieces: Array = MftrMaps.cliff_pieces(text)
	var cut: Array = Shops.cut_pieces(pieces, spots)
	var ctx: Dictionary = Shops._context(geo)
	var r := Shops.PLINTH_R_M * 100.0
	var placed := 0
	for i in spots.size():
		var spot: Dictionary = spots[i]
		var who := "%s, fountain %d" % [name, i]
		if spot.get("none", false):
			check(not stock, "%s: a stock map's fountain gets its stall" % who)
			print("      %s: no spot (the shop is on G)" % who)
			continue
		placed += 1
		var c: Vector2 = spot.center
		check(absf((spot.face as Vector2).length() - 1.0) < 1e-3, "%s: the keeper faces somewhere" % who)
		# The plinth stands on open ground once the nook is cut.
		var foot: Array = [c]
		for k in 24:
			foot.append(c + Vector2(cos(TAU * k / 24), sin(TAU * k / 24)) * r * 0.97)
		for q in foot:
			var inside := false
			if spot.notch:
				for p in cut:
					if (p as PackedVector2Array).size() >= 3 and Geometry2D.is_point_in_polygon(q, p):
						inside = true
			else:
				inside = Shops._solid(ctx, q)
			check(not inside, "%s: the plinth at %s is clear of the cliffs (%s)" % [who, c, q])
		# Boulders and structures.
		check(not Shops._touches(Shops._circle(c, r), ctx.outcrops), "%s: no boulder under the plinth" % who)
		for st in ctx.structures:
			check(c.distance_to(st) >= r + Shops.STRUCTURE_CLEAR_U - 1.0, "%s: clear of the structure at %s" % [who, st])
		# A nook opens only onto ground the fountain sees: every walkable point inside it has a
		# clear line to the fountain (through the nook).
		if spot.notch:
			var u: PackedVector2Array = Shops.alcove(spot)
			var box: Rect2 = Shops._rect(u)
			var f: Vector2 = fountains[i].center
			var y := box.position.y
			while y <= box.end.y:
				var x := box.position.x
				while x <= box.end.x:
					var q := Vector2(x, y)
					if Geometry2D.is_point_in_polygon(q, u) and not Shops._solid(ctx, q):
						check(Shops._clear_view(ctx, u, q, f), "%s: the nook at %s opens onto ground behind the wall (%s)" % [who, c, q])
					x += 50.0
				y += 50.0
	# The cut cliffs still build, without blowing up.
	var before: Dictionary = MftrMaps.cliffs_from_pieces(text, pieces)
	var after: Dictionary = MftrMaps.cliffs_from_pieces(text, cut)
	var n0 := 0
	var n1 := 0
	for g in before.get("groups", []):
		n0 += (g as PackedVector3Array).size()
	for g in after.get("groups", []):
		n1 += (g as PackedVector3Array).size()
		for v in (g as PackedVector3Array):
			if not (is_finite(v.x) and is_finite(v.y) and is_finite(v.z)):
				check(false, "%s: a cliff vertex isn't a number" % name)
				break
	check(n1 > 0, "%s: the cut cliffs build" % name)
	check(n1 < n0 * 1.5 + 30000, "%s: the cut cliffs stay about the same size (%d -> %d vertices)" % [name, n0, n1])
	print("      %s: %d stalls, cliffs %d -> %d vertices" % [name, placed, n0, n1])
	return placed
