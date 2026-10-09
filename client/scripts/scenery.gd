extends Node3D
## The map's dressing (art/props): forest on and beyond the walls, rocks at their edges, tall
## grass filling the brush and tufts along the lane, so the playable space sits in a world
## rather than on a slab. Purely visual: it never blocks, hides or reveals anything the
## simulation doesn't (walls, brush and vision come from the map's polygons).
##
## Everything is batched (one MultiMeshInstance3D per prop), placed from a fixed seed, and each
## instance gets its own turn, size and a slight tint. On the side nearest the camera, only low
## props stand close to the lane, so trees never hide the fight.

const U := 0.01                           # game units to meters
const OUTSIDE_U := 1700.0                 # how far the forest reaches past the map's edges
## The cliffs' height (the client extrudes walls this tall); scenery on them stands on top, and
## the land past the map is raised to it too.
const CLIFF_M := 1.4

var _walls: Array = []
var _size := Vector2.ZERO
# A lane map's axis (from blue's fountain to red's), and the unit vector across it pointing
# toward the camera (screen down, +y).
var _lane_mid := Vector2.ZERO
var _across := Vector2.ZERO

var _probe_camera_side := false
var _batches := {}                        # prop id -> [Transform3D, Color]
var _rng := RandomNumberGenerator.new()


## `geo` is `map_geometry()`; `materials_for(id)` returns [mesh, materials] for a prop, or null.
func build(geo: Dictionary, materials_for: Callable) -> void:
	_rng.seed = 1971
	var size: Vector2 = geo.size
	var walls: Array = geo.walls
	_walls = walls
	_size = size
	var brush: Array = geo.brush
	var fountains: Array = geo.get("fountains", [])
	var lane_map := fountains.size() == 2
	# Several lanes (Crossroads): whether a spot is on the camera's side of the ground it
	# borders is probed per spot, not taken from one lane's axis.
	_probe_camera_side = lane_map and geo.get("roads", []).size() > 1
	if lane_map:
		var a: Vector2 = fountains[0].center
		var b: Vector2 = fountains[1].center
		_lane_mid = (a + b) / 2.0
		var dir := (b - a).normalized()
		_across = Vector2(-dir.y, dir.x)
		if _across.y < 0.0:
			_across = -_across
	else:
		_lane_mid = size / 2.0
		_across = Vector2(0, 1)

	# Forest: a jittered grid over everything that isn't walkable ground, out past the map.
	var step := 260.0
	var margin := OUTSIDE_U if lane_map else 900.0
	var y := -margin
	while y < size.y + margin:
		var x := -margin
		while x < size.x + margin:
			var p := Vector2(x + _rng.randf_range(-110, 110), y + _rng.randf_range(-110, 110))
			var edge := _edge_distance(p, walls, size)
			if edge > 0.0:
				_forest_at(p, edge, _near_camera(p, edge), lane_map)
			x += step
		y += step
	# Small walls (outcrops in the lane) are drawn as rock clusters (`is_outcrop`).
	for poly in walls:
		if is_outcrop(poly):
			var r := _bounds(poly)
			var c := r.get_center()
			var rad := maxf(r.size.x, r.size.y) / 2.0
			_add("rock_1", c, rad / 105.0)
			for k in 3:
				var a := _rng.randf_range(0.0, TAU)
				_add(["rock_2", "bush_1", "rock_1"][k], c + Vector2.from_angle(a) * rad * 0.9, rad / 190.0)
	# Rocks along the walls' inner edges, half sunk into the cliff.
	for poly in walls:
		if is_outcrop(poly):
			continue
		var n: int = poly.size()
		for i in n:
			var a: Vector2 = poly[i]
			var b: Vector2 = poly[(i + 1) % n]
			var len := a.distance_to(b)
			var t := _rng.randf_range(0.0, 400.0)
			while t < len:
				var q := a.lerp(b, t / len)
				if q.x > 0 and q.x < size.x and q.y > 0 and q.y < size.y:
					var id: String = ["rock_1", "rock_2", "bush_1", "bush_1", "rock_1", "rock_3"][_rng.randi() % 6]
					_add(id, q, _rng.randf_range(0.9, 1.6))
				t += _rng.randf_range(350.0, 800.0)
	# Tall grass filling the brush.
	for poly in brush:
		var r := _bounds(poly)
		var gy := r.position.y
		while gy < r.end.y:
			var gx := r.position.x
			while gx < r.end.x:
				var p := Vector2(gx + _rng.randf_range(-25, 25), gy + _rng.randf_range(-25, 25))
				if Geometry2D.is_point_in_polygon(p, poly):
					_add("grass_1", p, _rng.randf_range(1.3, 1.9))
				gx += 55.0
			gy += 55.0
	# Tufts scattered over the walkable grass, sparse in the middle of the lane.
	for i in int(size.x * size.y / 90000.0):
		var p := Vector2(_rng.randf_range(0, size.x), _rng.randf_range(0, size.y))
		if _edge_distance(p, walls, size) > 0.0:
			continue
		var from_mid := clampf(absf((p - _lane_mid).dot(_across)) / 1500.0, 0.0, 1.0) if lane_map else 1.0
		if _rng.randf() < from_mid * 0.9:
			_add("grass_1", p, _rng.randf_range(0.7, 1.2))
	_flush(materials_for)


## Trees where there's room, low props near the lane on the camera's side.
func _forest_at(p: Vector2, edge: float, near_camera: bool, lane_map: bool) -> void:
	if near_camera and edge < 520.0:
		if _rng.randf() < 0.55:
			_add(["bush_1", "bush_1", "rock_1", "rock_2"][_rng.randi() % 4], p, _rng.randf_range(0.9, 1.5))
		return
	if edge < 120.0:
		if _rng.randf() < 0.5:
			_add("bush_1", p, _rng.randf_range(0.8, 1.3))
		return
	if _rng.randf() < 0.9:
		var id: String = ["pine_1", "pine_2", "pine_3"][_rng.randi() % 3]
		_add(id, p, _rng.randf_range(0.85, 1.35))
	elif _rng.randf() < 0.5:
		_add(["rock_1", "rock_3"][_rng.randi() % 2], p, _rng.randf_range(1.0, 1.8))


## Whether `p` (unwalkable, `edge` from walkable ground) stands between the camera and the
## ground it borders: that ground lies up-screen of it (the camera looks up the screen). The
## probe reaches 1.5 × `edge` up, so ground beyond a diagonal edge (√2 × `edge`) counts.
func _near_camera(p: Vector2, edge: float) -> bool:
	if not _probe_camera_side:
		return (p - _lane_mid).dot(_across) > 0.0
	if edge >= 520.0:
		return false
	return _edge_distance(p + Vector2(0, -(edge * 1.5 + 80.0)), _walls, _size) == 0.0


## A wall small enough to be a boulder in the field rather than a cliff (the client draws it
## with rock props when they're present).
static func is_outcrop(poly: PackedVector2Array) -> bool:
	var r := Rect2(poly[0], Vector2.ZERO)
	for q in poly:
		r = r.expand(q)
	return r.size.x < 500.0 and r.size.y < 500.0


## How far `p` is inside unwalkable ground (in a wall, or off the map), or 0 if it's walkable.
func _edge_distance(p: Vector2, walls: Array, size: Vector2) -> float:
	var outside := p.x < 0 or p.y < 0 or p.x > size.x or p.y > size.y
	var inside_wall := false
	for poly in walls:
		if Geometry2D.is_point_in_polygon(p, poly):
			inside_wall = true
			break
	if not outside and not inside_wall:
		return 0.0
	# The distance to the nearest wall edge or map edge that borders walkable ground.
	var best := INF
	for poly in walls:
		var n: int = poly.size()
		for i in n:
			var c := Geometry2D.get_closest_point_to_segment(p, poly[i], poly[(i + 1) % n])
			if c.x > 1.0 and c.y > 1.0 and c.x < size.x - 1.0 and c.y < size.y - 1.0:
				best = minf(best, p.distance_to(c))
	if best == INF and outside:
		best = _out_distance(p, size)  # no walls (open maps): the distance past the map's edge
	return best if best < INF else 9999.0


func _out_distance(p: Vector2, size: Vector2) -> float:
	var dx := maxf(maxf(-p.x, p.x - size.x), 0.0)
	var dy := maxf(maxf(-p.y, p.y - size.y), 0.0)
	return Vector2(dx, dy).length()


## The ground's height under `p`: cliff tops (inside a wall, not an outcrop) stand CLIFF_M high.
## A lane map's walls reach well past its edges, so the land beyond is cliff too.
func _height_at(p: Vector2) -> float:
	for poly in _walls:
		if not is_outcrop(poly) and Geometry2D.is_point_in_polygon(p, poly):
			return CLIFF_M
	return 0.0


func _bounds(poly: PackedVector2Array) -> Rect2:
	var r := Rect2(poly[0], Vector2.ZERO)
	for q in poly:
		r = r.expand(q)
	return r


func _add(id: String, p: Vector2, scale: float) -> void:
	if not _batches.has(id):
		_batches[id] = []
	var basis := Basis(Vector3.UP, _rng.randf_range(0.0, TAU)).scaled(Vector3.ONE * scale)
	var t := Transform3D(basis, Vector3(p.x * U, _height_at(p), p.y * U))
	var k := _rng.randf_range(0.88, 1.1) * (0.82 if id.begins_with("pine") else 1.0)
	var tint := Color(k * _rng.randf_range(0.96, 1.04), k, k * _rng.randf_range(0.94, 1.04)) * 0.5
	_batches[id].append([t, tint])


func _flush(materials_for: Callable) -> void:
	for id in _batches:
		var made = materials_for.call(id)
		if made == null:
			continue
		var list: Array = _batches[id]
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_custom_data = true
		mm.mesh = made[0]
		mm.instance_count = list.size()
		for i in list.size():
			mm.set_instance_transform(i, list[i][0])
			mm.set_instance_custom_data(i, list[i][1])
		var node := MultiMeshInstance3D.new()
		node.name = id
		node.multimesh = mm
		var mats: Array = made[1]
		for i in mats.size():
			mm.mesh.surface_set_material(i, mats[i])
		add_child(node)
	_batches.clear()
