extends Node3D
## A map's world from its geometry: the ground with its roads and river, the cliffs, brush, the
## scenery (forest, rocks, tall grass, from art/props) and the fountain platforms. A match builds
## it from `map_geometry()`; the map editor's Actual view builds the same from `MftrMaps.build`,
## so what the editor shows is what will be played.
##
## Props (static models from packs, validated like the champions, 11 §4) load once per process,
## shared with the match's structures.

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
## river }` (game units); `ground` is sized and shaded for it. Fountains take the ally or enemy
## color.
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
	if fountains.size() == 2:
		# A lane map: roads down its lanes (one from fountain to fountain on The Bridge), and
		# the river if it has one.
		var gm: ShaderMaterial = ground.material_override
		gm.set_shader_parameter("lane_mode", 1.0)
		var roads: Array = geo.get("roads", [])
		road_segments = segments(roads)
		gm.set_shader_parameter("roads", road_segments)
		gm.set_shader_parameter("road_count", road_segments.size())
		gm.set_shader_parameter("lane_width", 13.0 if roads.size() <= 1 else 9.5)
		var river := segments([geo.get("river", PackedVector2Array())])
		gm.set_shader_parameter("river", river)
		gm.set_shader_parameter("river_count", river.size())
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
	var brush_mat := ShaderMaterial.new()
	brush_mat.shader = load("res://shaders/brush.gdshader")
	for poly in geo.walls:
		if prop_model("rock_1") != null and preload("res://scripts/scenery.gd").is_outcrop(poly):
			continue  # drawn as a rock cluster by the scenery
		var wall := extrude(poly, CLIFF_M, wall_mat)
		wall.layers = LAYERS
		add_child(wall)
	# Tall grass props fill the brush when they're present; else a swaying block of brush.
	if prop_model("grass_1") == null:
		for poly in geo.brush:
			var b := extrude(poly, BRUSH_M, brush_mat)
			b.layers = LAYERS
			add_child(b)


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
