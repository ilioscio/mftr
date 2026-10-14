extends Node3D
## A map's world from its geometry: the ground with its roads and river, the cliffs, brush, the
## scenery (forest, rocks, tall grass, from art/props) and the fountain platforms. A match builds
## it from `map_geometry()`; the map editor's Actual view builds the same from `MftrMaps.build`,
## so what the editor shows is what will be played.
##
## Props (static models from packs, validated like the champions, 11 §4) load once per process,
## shared with the match's structures.

const Seasons = preload("res://scripts/seasons.gd")

const U := 0.01                           # game units to meters
const LAYERS := 3                         # the map's render layers (the minimap renders them)
const PROP_SLOTS := ["skin", "cloth", "metal", "emissive", "accent", "accent_glow"]
const CLIFF_M := 1.4
const BRUSH_M := 0.55
const MARGIN_U := 2600.0                  # ground past the map's edges, under the scenery

## Where the packs are (set by whoever builds the first view: the match or the editor).
static var art_root := ""
static var _models := {}                  # id -> MftrModel (or null without a pack)

## The roads as the ground shader draws them (segments in meters), for facing props along them.
var road_segments := PackedVector4Array()


static func prop_model(id: String) -> MftrModel:
	if not _models.has(id):
		var path := art_root.path_join("props/%s/export/%s.glb" % [id, id]).simplify_path()
		_models[id] = MftrModel.load(path) if FileAccess.file_exists(path) else null
	return _models[id]


static func prop_materials(model: MftrModel, team: Color, tinted := false) -> Array:
	var mats := []
	for slot in model.surface_slots():
		var m := ShaderMaterial.new()
		m.shader = load("res://shaders/prop.gdshader")
		m.set_shader_parameter("slot", PROP_SLOTS.find(slot))
		m.set_shader_parameter("team_accent", team)
		m.set_shader_parameter("instance_tint", tinted)
		mats.append(m)
	return mats


## One prop as a node (null without its pack).
static func prop_node(id: String, team: Color) -> MeshInstance3D:
	var model := prop_model(id)
	if model == null:
		return null
	var n := MeshInstance3D.new()
	n.mesh = model.static_mesh()
	var mats := prop_materials(model, team)
	for i in mats.size():
		n.set_surface_override_material(i, mats[i])
	n.set_meta("prop_mats", mats)
	return n


## Polylines (game units) as the ground shader's segments (meters), at most 48.
static func segments(polylines: Array) -> PackedVector4Array:
	var out := PackedVector4Array()
	for line in polylines:
		var pts: PackedVector2Array = line
		for i in range(1, pts.size()):
			if out.size() < 48:
				var a := pts[i - 1] * U
				var b := pts[i] * U
				out.append(Vector4(a.x, a.y, b.x, b.y))
	return out


## A ground plane with the ground shader, for a view that has none of its own.
static func make_ground() -> MeshInstance3D:
	var g := MeshInstance3D.new()
	var mat := ShaderMaterial.new()
	mat.shader = load("res://shaders/ground.gdshader")
	g.material_override = mat
	return g


## Build the world: `geo` is `{ walls, brush, size, fountains: [{ center, radius, ally }], roads,
## river, ground }` (game units; `ground` the ground control map, an Image, 15 §3); `ground` is
## sized and shaded for it. Fountains take the ally or enemy color.
func build(geo: Dictionary, ground: MeshInstance3D, ally: Color, enemy: Color) -> void:
	var size: Vector2 = geo.size
	var plane := PlaneMesh.new()
	plane.size = (size + Vector2(MARGIN_U, MARGIN_U) * 2.0) * U
	ground.mesh = plane
	ground.position = Vector3(size.x, 0, size.y) * (U / 2.0)
	ground.layers = LAYERS
	# The dressing: forest, rocks, tall grass, where the packs are present.
	var dressing := preload("res://scripts/scenery.gd").new()
	dressing.name = "Scenery"
	add_child(dressing)
	dressing.build(geo, func(id: String):
		var model := prop_model(id)
		if model == null:
			return null
		var mats := prop_materials(model, Color.WHITE, true)
		for m in mats:
			m.set_shader_parameter("sway", 0.05 if id.begins_with("grass") else (0.012 if id.begins_with("pine") else 0.0))
		return [model.static_mesh(), mats])
	for c in dressing.get_children():
		c.layers = LAYERS
	var fountains: Array = geo.get("fountains", [])
	var gm: ShaderMaterial = ground.material_override
	var season := Seasons.get_season(geo.get("season", "summer"))
	Seasons.apply_palette(season, "ground", gm)
	var control = geo.get("ground")
	if control is Image and fountains.size() == 2:
		# A lane map: the ground's materials come from its control map (15 §3).
		gm.set_shader_parameter("control", ImageTexture.create_from_image(control))
		gm.set_shader_parameter("control_on", true)
		gm.set_shader_parameter("map_size", size * U)
		gm.set_shader_parameter("lane_mode", 1.0)
	if fountains.size() == 2:
		# Facing props along the roads.
		road_segments = segments(geo.get("roads", []))
	for f in fountains:
		# The spawn platform (a prop the size of the fountain's circle), else a tinted disk.
		var platform := prop_node("fountain", ally if f.ally else enemy)
		if platform != null:
			platform.position = Vector3(f.center.x * U, 0.0, f.center.y * U)
			platform.scale = Vector3.ONE * (f.radius / 600.0)
			platform.layers = LAYERS
			add_child(platform)
			continue
		var disk := MeshInstance3D.new()
		var cyl := CylinderMesh.new()
		cyl.top_radius = f.radius * U
		cyl.bottom_radius = cyl.top_radius
		cyl.height = 0.04
		disk.mesh = cyl
		var m := StandardMaterial3D.new()
		m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		m.albedo_color = Color(ally if f.ally else enemy, 0.18)
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		disk.material_override = m
		disk.position = Vector3(f.center.x * U, 0.02, f.center.y * U)
		add_child(disk)
	var wall_mat := ShaderMaterial.new()
	wall_mat.shader = load("res://shaders/wall.gdshader")
	Seasons.apply_palette(season, "cliff", wall_mat)
	var brush_mat := ShaderMaterial.new()
	brush_mat.shader = load("res://shaders/brush.gdshader")
	var boxes := []
	for poly in geo.walls:
		var r := Rect2(poly[0], Vector2.ZERO)
		for q in poly:
			r = r.expand(q)
		boxes.append(r.grow(2.0))
	for wi in geo.walls.size():
		var poly: PackedVector2Array = geo.walls[wi]
		if prop_model("rock_1") != null and preload("res://scripts/scenery.gd").is_outcrop(poly):
			continue  # drawn as a rock cluster by the scenery
		# Seams: edges another wall piece continues past (no face there, only rock).
		var inside_other := func(p: Vector2) -> bool:
			for oi in geo.walls.size():
				if oi != wi and boxes[oi].has_point(p) and Geometry2D.is_point_in_polygon(p, geo.walls[oi]):
					return true
			return false
		var wall := MeshInstance3D.new()
		wall.mesh = cliff_mesh(poly, CLIFF_M, inside_other)
		if wall.mesh == null:
			continue
		wall.material_override = wall_mat
		wall.layers = LAYERS
		add_child(wall)
	# Tall grass props fill the brush when they're present; else a swaying block of brush.
	if prop_model("grass_1") == null:
		for poly in geo.brush:
			var b := extrude(poly, BRUSH_M, brush_mat)
			b.layers = LAYERS
			add_child(b)


## A cliff from a wall polygon (15 §4): its foot exactly on the polygon (the walkable edge the
## simulation uses), a broken face that steps back over a mossy ledge and leans in as it rises,
## and a grassy top. Faceted on purpose: the vertices are jittered, the shader shades each face
## flat. Null for a polygon too small to build.
static func cliff_mesh(poly: PackedVector2Array, height: float, is_seam_side := Callable()) -> ArrayMesh:
	var n := poly.size()
	if n < 3:
		return null
	var pts := PackedVector2Array()
	for p in poly:
		pts.append(p * U)
	# Which side of the edges is inside: probe just off the longest edge's middle.
	var longest := 0
	for i in n:
		if pts[i].distance_squared_to(pts[(i + 1) % n]) > pts[longest].distance_squared_to(pts[(longest + 1) % n]):
			longest = i
	var e0 := pts[longest]
	var e1 := pts[(longest + 1) % n]
	var probe := (e0 + e1) * 0.5 + (e1 - e0).orthogonal().normalized() * 0.02
	var turn := 1.0 if Geometry2D.is_point_in_polygon(probe, pts) else -1.0
	var inward := PackedVector2Array()
	for i in n:
		var a := pts[(i - 1 + n) % n]
		var b := pts[i]
		var c := pts[(i + 1) % n]
		var n1 := (b - a).orthogonal().normalized() * turn
		var n2 := (c - b).orthogonal().normalized() * turn
		var m := (n1 + n2)
		m = m.normalized() / maxf(0.35, m.normalized().dot(n1)) if m.length() > 1e-4 else n1
		inward.append(m)
	# Vertices on a seam (an edge with another piece of wall past it) stay put: no face there.
	var seam := PackedByteArray()
	seam.resize(n)
	if is_seam_side.is_valid():
		for i in n:
			var a2 := pts[i]
			var b2 := pts[(i + 1) % n]
			var outward := -(b2 - a2).orthogonal().normalized() * turn
			if is_seam_side.call(((a2 + b2) * 0.5 + outward * 0.2) / U):
				seam[i] = 1
				seam[(i + 1) % n] = 1
	# Rings: the foot, the lip of the ledge, the back of the ledge, the top edge.
	var rings := []
	var profile := [[0.0, 0.0], [0.1, height * 0.42], [0.22, height * 0.47], [0.32, height]]
	for k in profile.size():
		var ring := PackedVector3Array()
		for i in n:
			var j := _jitter(pts[i], k) if k > 0 else 0.0
			var d: float = 0.0 if seam[i] == 1 else profile[k][0] + j * 0.08
			var p := pts[i] + inward[i] * d
			var y: float = profile[k][1] + (j * 0.12 if k == 1 or k == 2 else 0.0)
			ring.append(Vector3(p.x, y, p.y))
		rings.append(ring)
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for k in range(1, rings.size()):
		var lo: PackedVector3Array = rings[k - 1]
		var hi: PackedVector3Array = rings[k]
		for i in n:
			var i2 := (i + 1) % n
			_quad(st, lo[i], lo[i2], hi[i2], hi[i])
	# The top: the polygon's own triangulation on the top ring's points (the same vertices, a
	# little inside), so the top never reaches past the face below it.
	var top: PackedVector3Array = rings[rings.size() - 1]
	var tris := Geometry2D.triangulate_polygon(pts)
	for t in range(0, tris.size(), 3):
		var a3 := top[tris[t]]
		var b3 := top[tris[t + 1]]
		var c3 := top[tris[t + 2]]
		a3.y = height
		b3.y = height
		c3.y = height
		# Facing up (clockwise seen from above).
		if (b3 - a3).cross(c3 - a3).y < 0.0:
			st.add_vertex(a3)
			st.add_vertex(b3)
			st.add_vertex(c3)
		else:
			st.add_vertex(a3)
			st.add_vertex(c3)
			st.add_vertex(b3)
	st.generate_normals()
	return st.commit()


## A quad of the cliff's face, front side out (Godot's front faces wind clockwise).
static func _quad(st: SurfaceTool, a: Vector3, b: Vector3, c: Vector3, d: Vector3) -> void:
	var lo := (a + b) * 0.5
	var hi := (c + d) * 0.5
	var out := Vector3(lo.x - hi.x, 0.0, lo.z - hi.z) + Vector3(0, 0.3, 0) * maxf((a - b).length(), 0.01)
	if (b - a).cross(c - a).dot(out) < 0.0:
		st.add_vertex(a)
		st.add_vertex(b)
		st.add_vertex(c)
		st.add_vertex(a)
		st.add_vertex(c)
		st.add_vertex(d)
	else:
		st.add_vertex(a)
		st.add_vertex(c)
		st.add_vertex(b)
		st.add_vertex(a)
		st.add_vertex(d)
		st.add_vertex(c)


## A small repeatable jitter in -1..1 for a cliff vertex (by its position and ring).
static func _jitter(p: Vector2, k: int) -> float:
	var h := sin(p.x * 12.9898 + p.y * 78.233 + float(k) * 37.719) * 43758.5453
	return (h - floor(h)) * 2.0 - 1.0


## CSGPolygon3D extrudes along local -Z; rotating +90° about X lays the polygon on the ground
## (x, y) -> world (x, 0, y) and extrudes upward.
static func extrude(poly: PackedVector2Array, height: float, mat: Material) -> CSGPolygon3D:
	var pts := PackedVector2Array()
	for p in poly:
		pts.append(p * U)
	var csg := CSGPolygon3D.new()
	csg.polygon = pts
	csg.mode = CSGPolygon3D.MODE_DEPTH
	csg.depth = height
	csg.rotation_degrees = Vector3(90, 0, 0)
	csg.material = mat
	return csg
