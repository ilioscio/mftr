extends Node3D
## The shops (19 §4): beside each base's fountain a stall on a plinth of the base's paving, and
## a shopkeeper at its front who idles, fidgets and reacts. Client only: the simulation never
## knows they exist. Clicking your own keeper or stall opens the shop; clicking the enemy's gets
## a line of banter.
##
## Which two keepers a match gets comes from the match itself (its champions and their unit ids,
## the same for every player in it), so everyone sees the same pair; `--shopkeepers A[,B]` picks
## them for your base (and the enemy's) instead, for reviewing them, and `--shopkeeper-tour` has
## both play every clip in turn (the fidgets, then the reactions), a couple of seconds apart.

const MapView := preload("res://scripts/map_view.gd")

const KEYS := ["goldwick", "rosette", "slink", "folio"]
const NAMES := {"goldwick": "Goldwick", "rosette": "Madame Rosette", "slink": "Slink", "folio": "Old Folio"}
const BANTER := {
	"goldwick": ["Your coin's no good here. Well, it is. Just not from you.", "Hands off the merchandise.",
		"Enemy gold? I'd still count it."],
	"rosette": ["Not for you, darling. Not today.", "Shoo! You'll smudge the mirror.", "Charming. Now run along."],
	"slink": ["Never seen you. You never saw me.", "Wrong side of the river, friend.", "Psst... no."],
	"folio": ["Eh? Who? Come back with a letter of introduction.", "The archive is closed. Since forever.",
		"Shh. The scrolls are sleeping."],
}
## Where each keeper stands on their stall's plinth (art/shopkeepers/concepts.py KEEP_AT), in the
## stall's space: x to its left, z toward its front.
const KEEP_AT := {"goldwick": Vector2(0.0, 0.55), "rosette": Vector2(0.0, 0.50), "slink": Vector2(-0.15, 0.55), "folio": Vector2(0.0, 0.50)}
const PLINTH_M := 0.18
const PLINTH_R_M := 1.5
## The stall's spot beside its fountain, in units: `ALONG` from the fountain's center toward the
## map's middle, `ACROSS` to the side.
const ALONG := 0.90
const ACROSS := 1.46
const FADE := 0.3                          # seconds a clip crossfades into the next
const FIDGET_EVERY := Vector2(8.0, 15.0)   # seconds between fidgets
const NEAR_M := 8.0                        # a champion this close is at the shop
const LOOK_M := 6.0                        # and this close, the keeper turns its head to them
const U := 0.01

var art_root := ""
var shops: Array = []      # [{ key, ally, center (u), node, skel, anim, clip, t, from, from_t, fade, wait, last, held, yaw }]
var _rng := RandomNumberGenerator.new()
var _pending := {}         # what `place` waits on until the match's pair is known
var _placed := {}
var forced: Array = []     # `--shopkeepers`: [ours, theirs]
var tour := false          # `--shopkeeper-tour`: every clip in turn, for reviews
const TOUR := ["idle_fidget_1", "idle_fidget_2", "idle_fidget_3", "greet", "purchase", "farewell", "victory", "defeat"]
const TOUR_GAP := 2.0


## The match's pair, from its scoreboard (`[{ unit, champion, team, you }]`): [blue's, red's].
static func pair_for(rows: Array) -> Array:
	var parts: Array = []
	for r in rows:
		parts.append("%d:%s" % [int(r.unit), str(r.champion)])
	parts.sort()
	var h := hash(",".join(parts))
	h = absi(h)
	var a := h % KEYS.size()
	var b := (a + 1 + (h / KEYS.size()) % (KEYS.size() - 1)) % KEYS.size()
	return [KEYS[a], KEYS[b]]


## Set up the shops for `fountains` (`[{ center, radius, ally }]`, blue's first), the map `size`
## (units) and `season`. They appear once the pair is known (`update` asks the scoreboard).
func place(fountains: Array, size: Vector2, season: String, ally: Color, enemy: Color) -> void:
	_rng.randomize()
	_placed = {"fountains": fountains, "size": size, "season": season, "ally": ally, "enemy": enemy}
	_pending = _placed.duplicate()


## A new match on the same map: the stalls come down, and the next pair goes up once it's known.
func reset() -> void:
	for s in shops:
		s.node.queue_free()
	shops.clear()
	_pending = _placed.duplicate()
	_was_open = false


func _build(keys_by_side: Array) -> void:
	var p := _pending
	_pending = {}
	for i in p.fountains.size():
		var f: Dictionary = p.fountains[i]
		var key: String = keys_by_side[i] if i < keys_by_side.size() else KEYS[0]
		_build_shop(key, f, p.size, p.season, p.ally if f.ally else p.enemy)


func _build_shop(key: String, f: Dictionary, size: Vector2, season: String, team: Color) -> void:
	var c: Vector2 = f.center
	var r: float = f.radius
	var d := (size / 2.0 - c).normalized()
	if d == Vector2.ZERO:
		d = Vector2(1, 0)
	var across := Vector2(d.y, -d.x)
	var at := c + d * r * ALONG + across * r * ACROSS
	var node := Node3D.new()
	node.name = "Shop_" + key
	node.position = Vector3(at.x * U, 0.0, at.y * U)
	# The stall faces the camera (every base shares its view, so a keeper turned toward the
	# fountain shows its back), turned a little toward the map's middle, where its team walks.
	var face := Vector2(signf(size.x / 2.0 - at.x) * 0.45, 1.0).normalized()
	node.rotation.y = atan2(face.x, face.y)
	add_child(node)
	var core := MapView.prop_node("stall_" + key, team)
	if core != null:
		core.layers = MapView.LAYERS
		node.add_child(core)
	var dress := MapView.prop_node("stall_%s_%s" % [key, season if season in ["summer", "autumn", "winter"] else "summer"], team)
	if dress != null:
		dress.position.y = PLINTH_M
		dress.layers = MapView.LAYERS
		node.add_child(dress)
	var shop := {"key": key, "ally": f.ally, "center": at, "node": node, "skel": null, "anim": null, "model": null,
		"clip": "idle", "t": _rng.randf_range(0.0, 4.0), "from": "idle", "from_t": 0.0, "fade": 1.0,
		"wait": 1.0 if tour else _rng.randf_range(FIDGET_EVERY.x, FIDGET_EVERY.y), "last": "", "held": false, "yaw": 0.0}
	var path := art_root.path_join("shopkeepers/%s/export/%s.glb" % [key, key]).simplify_path()
	var pack: MftrModel = MftrModel.load(path) if FileAccess.file_exists(path) else null
	if pack != null:
		var model: Node3D = pack.instantiate()
		var spot: Vector2 = KEEP_AT[key]
		model.position = Vector3(spot.x, PLINTH_M, spot.y)
		node.add_child(model)
		var mesh: MeshInstance3D = model.get_node("Skeleton/Mesh")
		var outline := ShaderMaterial.new()
		outline.shader = load("res://shaders/outline.gdshader")
		outline.set_shader_parameter("color", team.lerp(Color.BLACK, 0.55))
		var slots := pack.surface_slots()
		for i in slots.size():
			var m := ShaderMaterial.new()
			m.shader = load("res://shaders/champion_model.gdshader")
			m.set_shader_parameter("slot", MapView.PROP_SLOTS.find(slots[i]))
			m.set_shader_parameter("team_accent", team)
			m.set_shader_parameter("identity_mix", 0.0)
			m.next_pass = outline
			mesh.set_surface_override_material(i, m)
		shop.model = model
		shop.skel = model.get_node("Skeleton")
		shop.anim = pack.new_animator()
		shop.pack = pack
		shop.len = shop.anim.clip_length("idle")
	shops.append(shop)


## Every frame: pick the pair once the match is known, then advance each keeper's clips. `own` is
## our champion's position (meters; null if none); `shop_open` whether the shop window is up.
func update(delta: float, scoreboard: Callable, own, shop_open: bool) -> void:
	if not _pending.is_empty():
		var keys: Array = []
		if not forced.is_empty():
			var ours: String = forced[0]
			var theirs: String = forced[1] if forced.size() > 1 else KEYS[(KEYS.find(ours) + 1) % KEYS.size()]
			for f in _pending.fountains:
				keys.append(ours if f.ally else theirs)
		else:
			var rows: Array = scoreboard.call()
			if rows.is_empty():
				return
			var pair := pair_for(rows)
			keys = pair
		_build(keys)
	for s in shops:
		_animate(s, delta, own)
	_watch_shop(own, shop_open)


var _was_open := false


## Greet when the shop opens near our keeper, farewell when it closes.
func _watch_shop(own, open: bool) -> void:
	if open == _was_open:
		return
	_was_open = open
	var s = _ours()
	if s == null or own == null or (own as Vector3).distance_to(s.node.position) > NEAR_M:
		return
	play(s, "greet" if open else "farewell")


func _ours():
	for s in shops:
		if s.ally:
			return s
	return null


## Start `clip` on keeper `s`, crossfading from what it plays now.
func play(s: Dictionary, clip: String) -> void:
	if s.anim == null or s.held:
		return
	s.from = s.clip
	s.from_t = s.t
	s.clip = clip
	s.t = 0.0
	s.fade = 0.0
	s.len = s.anim.clip_length(clip)


## Our purchase: our keeper's delight.
func purchase() -> void:
	var s = _ours()
	if s != null and (s.clip in ["idle", "greet", "farewell", "purchase"] or s.clip.begins_with("idle_fidget")):
		play(s, "purchase")


## The match is over: the side whose Base fell, defeat (held); the other, victory.
func match_ended(we_won: bool) -> void:
	for s in shops:
		play(s, "victory" if s.ally == we_won else "defeat")


func _animate(s: Dictionary, delta: float, own) -> void:
	if s.anim == null:
		return
	s.t += delta
	s.from_t += delta
	s.fade = minf(1.0, s.fade + delta / FADE)
	var length: float = s.len
	if s.clip != "idle" and s.t >= length:
		if s.clip == "defeat" and tour and s.t >= length + TOUR_GAP:
			play(s, "idle")
		elif s.clip == "defeat":
			s.t = length
			s.held = not tour
		elif s.clip != "victory" or s.t >= length + 1.0:
			play(s, "idle")
	if s.clip == "idle" and s.fade >= 1.0:
		s.wait -= delta
		if s.wait <= 0.0 and tour:
			var next: String = TOUR[(TOUR.find(s.last) + 1) % TOUR.size()]
			s.last = next
			s.wait = TOUR_GAP
			play(s, next)
		elif s.wait <= 0.0:
			var choices := ["idle_fidget_1", "idle_fidget_2", "idle_fidget_3"]
			choices.erase(s.last)
			var pick: String = choices[_rng.randi() % choices.size()]
			s.last = pick
			s.wait = _rng.randf_range(FIDGET_EVERY.x, FIDGET_EVERY.y)
			play(s, pick)
	s.anim.pose_mix(s.skel, s.from, s.from_t, s.clip, minf(s.t, length), s.fade)
	_look(s, delta, own)


## Our own keeper turns its head toward our champion when it's near (idling only).
func _look(s: Dictionary, delta: float, own) -> void:
	var want := 0.0
	if own != null and s.ally and s.clip == "idle":
		var head := (s.skel as Skeleton3D).find_bone("head")
		var local: Vector3 = (s.model as Node3D).global_transform.affine_inverse() * (own as Vector3)
		var dist := Vector2(local.x, local.z).length()
		if head >= 0 and dist < LOOK_M:
			want = clampf(atan2(local.x, local.z), -0.9, 0.9) * clampf((LOOK_M - dist) / 1.5, 0.0, 1.0)
	s.yaw = lerpf(s.yaw, want, clampf(delta * 4.0, 0.0, 1.0))
	if absf(s.yaw) < 0.005:
		return
	var skel: Skeleton3D = s.skel
	var i := skel.find_bone("head")
	var parent := skel.get_bone_parent(i)
	if i < 0 or parent < 0:
		return
	var g := skel.get_bone_global_pose(i)
	var turned := Basis(Vector3.UP, s.yaw) * g.basis
	var local_basis := skel.get_bone_global_pose(parent).basis.inverse() * turned
	skel.set_bone_pose_rotation(i, local_basis.get_rotation_quaternion())


## The shop under `p` (meters, on the ground), or null: on its plinth.
func shop_at(p: Vector3):
	for s in shops:
		if Vector2(p.x - s.node.position.x, p.z - s.node.position.z).length() < PLINTH_R_M:
			return s
	return null


func banter(s: Dictionary) -> String:
	var lines: Array = BANTER.get(s.key, [""])
	return "%s: %s" % [NAMES.get(s.key, s.key), lines[_rng.randi() % lines.size()]]
