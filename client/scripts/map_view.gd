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
## The season's snow on props (15 §9), set by the latest `build` for every prop material made
## after it (the scenery, the structures).
static var snow := 0.0
static var _models := {}                  # id -> MftrModel (or null without a pack)

## The roads as the ground shader draws them (segments in meters), for facing props along them.
var road_segments := PackedVector4Array()

## Local lights (15 §7): each a marker node, its color, reach (m) and strength. The ground shader
## draws the nearest LAMPS_DRAWN to what the camera looks at as warm pools on the ground (one
## big ground mesh would hit the renderer's per-object light limit with real lights).
var _lamps := []
var _ground_mat: ShaderMaterial
const LAMPS_DRAWN := 16
static var _glow_spots := {}              # prop id -> [[local position, color, "fire"|"glow"|"crystal"]]
static var _fire_mat: ParticleProcessMaterial
static var _fire_mesh: QuadMesh


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
		m.set_shader_parameter("snow", snow)
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
## river, ground, cliffs }` (game units; `ground` the ground control map, an Image, 15 §3;
## `cliffs` the cliffs' meshes and outcrops, 15 §4); `ground` is
## sized and shaded for it. Fountains take the ally or enemy color.
func build(geo: Dictionary, ground: MeshInstance3D, ally: Color, enemy: Color) -> void:
	var size: Vector2 = geo.size
	var season := Seasons.get_season(geo.get("season", "summer"))
	snow = season.get("snow", 0.0)
	var plane := PlaneMesh.new()
	plane.size = (size + Vector2(MARGIN_U, MARGIN_U) * 2.0) * U
	ground.mesh = plane
	ground.position = Vector3(size.x, 0, size.y) * (U / 2.0)
	ground.layers = LAYERS
	_ground_mat = ground.material_override
	# The dressing: forest, rocks, tall grass, where the packs are present.
	var dressing := preload("res://scripts/scenery.gd").new()
	dressing.name = "Scenery"
	add_child(dressing)
	var materials_for := func(id: String):
		var model := prop_model(id)
		if model == null:
			return null
		var mats := prop_materials(model, Color.WHITE, true)
		for m in mats:
			var sway := 0.05 if id.begins_with("grass") else (0.012 if id.begins_with("pine") or id.begins_with("tree") else 0.0)
			m.set_shader_parameter("sway", 0.08 if id.begins_with("flowers") else sway)
			if id.begins_with("grass"):
				m.set_shader_parameter("snow", 0.0)  # grass stands up out of the snow
		return [model.static_mesh(), mats]
	dressing.build(geo, materials_for)
	for c in dressing.get_children():
		c.layers = LAYERS
	var fountains: Array = geo.get("fountains", [])
	var gm: ShaderMaterial = ground.material_override
	# Small life (15 §8), from the ground control map; not on the minimap.
	var control = geo.get("ground")
	if control is Image:
		var life := preload("res://scripts/life.gd").new()
		life.name = "Life"
		add_child(life)
		life.build(geo, control, season, materials_for)
	Seasons.apply_palette(season, "ground", gm)
	if control is Image and not fountains.is_empty():
		# A lane map (Horde's have only blue's fountain): the ground's materials come from its
		# control map (15 §3).
		gm.set_shader_parameter("control", ImageTexture.create_from_image(control))
		gm.set_shader_parameter("control_on", true)
		gm.set_shader_parameter("map_size", size * U)
		gm.set_shader_parameter("lane_mode", 1.0)
	if not fountains.is_empty():
		# Facing props along the roads. Horde maps have one fountain (no red champions) but
		# still a lane, and their structures face down it like any other.
		road_segments = segments(geo.get("roads", []))
	for f in fountains:
		# The spawn platform (a prop the size of the fountain's circle), else a tinted disk.
		var platform := prop_node("fountain", ally if f.ally else enemy)
		if platform != null:
			platform.position = Vector3(f.center.x * U, 0.0, f.center.y * U)
			platform.scale = Vector3.ONE * (f.radius / 600.0)
			platform.layers = LAYERS
			add_child(platform)
			add_glows(platform, "fountain", ally if f.ally else enemy)
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
	Seasons.apply_palette(season, "ground", wall_mat)    # the tops are the ground's grass
	if control is Image:
		wall_mat.set_shader_parameter("control", gm.get_shader_parameter("control"))
		wall_mat.set_shader_parameter("control_on", gm.get_shader_parameter("control_on"))
		wall_mat.set_shader_parameter("map_size", size * U)
	var brush_mat := ShaderMaterial.new()
	brush_mat.shader = load("res://shaders/brush.gdshader")
	# The cliffs (15 §4), built whole from the wall pieces (`maps::cliffs`): one mesh per group.
	var cliffs: Dictionary = geo.get("cliffs", {})
	for tris in cliffs.get("groups", []):
		var arrays := []
		arrays.resize(Mesh.ARRAY_MAX)
		arrays[Mesh.ARRAY_VERTEX] = tris
		var mesh := ArrayMesh.new()
		mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
		var wall := MeshInstance3D.new()
		wall.mesh = mesh
		wall.material_override = wall_mat
		wall.layers = LAYERS
		add_child(wall)
	# Tall grass props fill the brush when they're present; else a swaying block of brush.
	if prop_model("grass_1") == null:
		for poly in geo.brush:
			var b := extrude(poly, BRUSH_M, brush_mat)
			b.layers = LAYERS
			add_child(b)


## Fire in a prop's braziers and the glow of its crystal (15 §7): small flames over each ember
## bowl (its `emissive` surface, where warm) and lamps on the ground under them; a lamp in the
## team's color under its highest crystal (`accent_glow`), named "Lamp_crystal" (a falling
## structure hides it). `node` is the prop's model (the spots are in its space).
func add_glows(node: Node3D, id: String, team: Color) -> void:
	var scale := node.global_transform.basis.get_scale().x if node.is_inside_tree() else node.scale.x
	for spot in glow_spots(id):
		var at: Vector3 = spot[0]
		var marker := Node3D.new()
		marker.position = at
		node.add_child(marker)
		match spot[2]:
			"fire":
				marker.name = "Lamp_fire"
				marker.add_child(_fire())
				_lamps.append([marker, Color(1.0, 0.55, 0.22), 3.6 * scale, 0.75])
			"glow":
				marker.name = "Lamp_glow"
				_lamps.append([marker, spot[1], 2.4 * scale, 0.5])
			"crystal":
				marker.name = "Lamp_crystal"
				_lamps.append([marker, team.lerp(Color.WHITE, 0.2), 4.5 * scale, 0.6])


## Where a prop glows, from its model: clusters of its warm `emissive` vertices (fire), of other
## `emissive` ones (a glow), and the highest cluster of `accent_glow` (its crystal).
static func glow_spots(id: String) -> Array:
	if _glow_spots.has(id):
		return _glow_spots[id]
	var out := []
	var model := prop_model(id)
	if model != null:
		var mesh := model.static_mesh()
		var slots := model.surface_slots()
		for si in slots.size():
			if slots[si] != "emissive" and slots[si] != "accent_glow":
				continue
			var arrays := mesh.surface_get_arrays(si)
			var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
			var colors = arrays[Mesh.ARRAY_COLOR]
			var clusters := []                # [sum, count, top, color]
			for vi in verts.size():
				var v := verts[vi]
				var hit := false
				for c in clusters:
					if Vector2(c[0].x / c[1], c[0].z / c[1]).distance_to(Vector2(v.x, v.z)) < 0.5 and absf(c[2] - v.y) < 0.8:
						c[0] += v
						c[1] += 1
						c[2] = maxf(c[2], v.y)
						hit = true
						break
				if not hit:
					clusters.append([v, 1, v.y, colors[vi] if colors is PackedColorArray and vi < colors.size() else Color.WHITE])
			if slots[si] == "emissive":
				for c in clusters:
					var col: Color = c[3]
					var at := Vector3(c[0].x / c[1], c[2], c[0].z / c[1])
					out.append([at, col, "fire" if col.r > col.g * 1.4 else "glow"])
			elif not clusters.is_empty():
				var top = clusters[0]
				for c in clusters:
					if c[2] > top[2]:
						top = c
				out.append([Vector3(top[0].x / top[1], 0.05, top[0].z / top[1]), Color.WHITE, "crystal"])
	_glow_spots[id] = out
	return out


## Small flames over an ember bowl: additive, rising and shrinking; bloom does the rest.
static func _fire() -> GPUParticles3D:
	if _fire_mat == null:
		_fire_mat = ParticleProcessMaterial.new()
		_fire_mat.emission_shape = ParticleProcessMaterial.EMISSION_SHAPE_SPHERE
		_fire_mat.emission_sphere_radius = 0.09
		_fire_mat.direction = Vector3(0, 1, 0)
		_fire_mat.spread = 12.0
		_fire_mat.initial_velocity_min = 0.35
		_fire_mat.initial_velocity_max = 0.7
		_fire_mat.gravity = Vector3(0, 0.6, 0)
		_fire_mat.scale_min = 0.7
		_fire_mat.scale_max = 1.2
		var shrink := Curve.new()
		shrink.add_point(Vector2(0, 0.6))
		shrink.add_point(Vector2(0.25, 1.0))
		shrink.add_point(Vector2(1, 0.1))
		var st := CurveTexture.new()
		st.curve = shrink
		_fire_mat.scale_curve = st
		var g := Gradient.new()
		g.colors = PackedColorArray([Color(1.0, 0.9, 0.5, 1.0), Color(1.0, 0.5, 0.12, 0.9), Color(0.7, 0.15, 0.05, 0.0)])
		g.offsets = PackedFloat32Array([0.0, 0.45, 1.0])
		var gt := GradientTexture1D.new()
		gt.gradient = g
		_fire_mat.color_ramp = gt
		_fire_mesh = QuadMesh.new()
		_fire_mesh.size = Vector2(0.2, 0.26)
		var m := StandardMaterial3D.new()
		m.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
		m.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
		m.blend_mode = BaseMaterial3D.BLEND_MODE_ADD
		m.billboard_mode = BaseMaterial3D.BILLBOARD_PARTICLES
		m.vertex_color_use_as_albedo = true
		m.albedo_color = Color(1.0, 0.8, 0.6, 1.0)
		m.albedo_texture = soft_dot()
		m.emission_enabled = true
		m.emission = Color(1.0, 0.45, 0.15)
		m.emission_energy_multiplier = 1.2
		_fire_mesh.material = m
	var p := GPUParticles3D.new()
	p.amount = 14
	p.lifetime = 0.7
	p.preprocess = 1.0
	p.randomness = 0.4
	p.local_coords = false
	p.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	p.visibility_aabb = AABB(Vector3(-1, -0.5, -1), Vector3(2, 2.5, 2))
	p.process_material = _fire_mat
	p.draw_pass_1 = _fire_mesh
	return p


## A soft round spot (generated, not a file): particles' shape.
static func soft_dot() -> GradientTexture2D:
	var g := Gradient.new()
	g.colors = PackedColorArray([Color(1, 1, 1, 1), Color(1, 1, 1, 0.6), Color(1, 1, 1, 0)])
	g.offsets = PackedFloat32Array([0.0, 0.35, 1.0])
	var t := GradientTexture2D.new()
	t.gradient = g
	t.fill = GradientTexture2D.FILL_RADIAL
	t.fill_from = Vector2(0.5, 0.5)
	t.fill_to = Vector2(0.5, 0.0)
	t.width = 32
	t.height = 32
	return t


## Each frame: the lamps nearest to what the camera looks at, to the ground shader.
func _process(_delta: float) -> void:
	if _ground_mat == null or _lamps.is_empty():
		return
	var cam := get_viewport().get_camera_3d()
	if cam == null:
		return
	var fwd := -cam.global_transform.basis.z
	var look := cam.global_position
	if fwd.y < -0.01:
		look += fwd * (-look.y / fwd.y)
	var near := []
	for l in _lamps:
		var marker: Node3D = l[0]
		if not is_instance_valid(marker) or not marker.is_visible_in_tree():
			continue
		var at := marker.global_position
		near.append([Vector2(at.x - look.x, at.z - look.z).length_squared(), at, l])
	near.sort_custom(func(a, b): return a[0] < b[0])
	var pos := PackedVector4Array()
	var col := PackedVector4Array()
	for i in LAMPS_DRAWN:
		if i < near.size():
			var at: Vector3 = near[i][1]
			var l: Array = near[i][2]
			var c: Color = l[1]
			pos.append(Vector4(at.x, at.z, l[2], l[3]))
			col.append(Vector4(c.r, c.g, c.b, float(i) * 1.7 + at.x))
		else:
			pos.append(Vector4(0, 0, 0, 0))
			col.append(Vector4(0, 0, 0, 0))
	_ground_mat.set_shader_parameter("lamps", pos)
	_ground_mat.set_shader_parameter("lamp_colors", col)
	_ground_mat.set_shader_parameter("lamp_count", mini(near.size(), LAMPS_DRAWN))


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
