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
## The whole shop (stall, keeper, plinth) is drawn this much larger than built: the reference
## game's shopkeeper stands two to three times a champion's height and its shop fills a corner of
## the screen, a call to action. The packs stay at their built size (and budgets).
const SCALE := 2.2
const PLINTH_M := 0.18                     # the plinth's top, in the stall's own (unscaled) space
const PLINTH_R_M := 1.72 * SCALE           # the plinth's foot (its step): what a click hits, what clears the props
## Where a stall stands (`spots`), as in the reference game: on the rim of the fountain's
## platform, above it and a little to the left on screen (the camera is the same for both teams),
## set into the wall where there is one. Directions are tried in this order, in degrees left of
## straight up (midway between the platform's top banner and its brazier, then between that
## brazier and the left banner), then every other one by how far it turns from the first. Along
## each, the stall starts where its plinth clears the platform's props and goes out at most
## SEARCH_U, until a spot passes `_check`. A fountain with no such spot gets no stall (the shop
## is still on G).
const SPOT_PREF := [22, 18, 26, 14, 30, 65, 60, 70, 55, 75, 10, 5, 0, 80, 85, 90, 50]
const SEARCH_U := 600.0
const STEP_U := 20.0
## A stall that meets a wall is set this far behind the wall's face (u) when it can be, in a nook
## cut out of the cliff (`alcove`, `cut_pieces`); rock at least BACK_U thick must stay around the
## nook, or the cut would show a way through that the simulation doesn't have.
const INSET_U := 70.0 * SCALE
const BACK_U := 250.0
## The deepest a stall may stand behind a wall's face (u): clearing the platform's props can push
## it past INSET_U, but never into a tunnel.
const DEEPEST_U := 350.0
## A stall keeps this far (u) from any structure, past its own plinth.
const STRUCTURE_CLEAR_U := 400.0
## The fountain platform's own props (art/props/landmarks/build.py), at its built size (600 u):
## braziers on the diagonals, banners on the axes. The plinth keeps CLEAR_U from both.
const BRAZIER_R_U := 490.0
const BANNER_R_U := 530.0
const CLEAR_U := 30.0
## The alcove: a U around the stall (rounded behind it, straight sides out to the platform), its
## radius the plinth's and a margin.
const ALCOVE_R_U := PLINTH_R_M * 100.0 + 40.0
const FADE := 0.3                          # seconds a clip crossfades into the next
const FIDGET_EVERY := Vector2(8.0, 15.0)   # seconds between fidgets
const NEAR_M := 4.0 + 2.5 * SCALE          # a champion this close is at the shop
const LOOK_M := 3.0 + 2.0 * SCALE          # and this close, the keeper turns its head to them
const U := 0.01

var art_root := ""
var shops: Array = []      # [{ key, ally, center (u), node, skel, anim, clip, t, from, from_t, fade, wait, last, held, yaw }]
var _rng := RandomNumberGenerator.new()
var _season := "summer"    # the stalls' dressing and the keepers' accessories (winter's)
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


## Each fountain's stall spot, in the fountains' order. `geo` is as `map_geometry()` gives it
## (`fountains`, `walls`, `size`, `structures`) with its `cliffs` (`outcrops`: walls that are
## boulders). A spot is `{ center (u), face (unit: where the keeper looks), notch (bool: set into
## a wall), dir (unit, from the fountain to the stall), reach (u: how far the nook runs from the
## stall back toward the fountain) }`, or `{ none: true }`.
static func spots(geo: Dictionary) -> Array:
	var ctx := _context(geo)
	var out: Array = []
	for f in geo.get("fountains", []):
		var c: Vector2 = f.center
		var k := float(f.radius) / 600.0
		var spot := {}
		if _solid(ctx, c):
			out.append({"none": true, "notch": false})  # a fountain off its own map: no stall
			continue
		for deg in _directions():
			var a := deg_to_rad(float(deg))
			var dir := Vector2(-sin(a), -cos(a))
			var d0 := _clear_of_props(dir, k)
			var tries: Array = []
			var hit := _wall_hit(ctx, c, dir, d0 + SEARCH_U + PLINTH_R_M * 100.0)
			if hit < INF and hit + INSET_U >= d0 and hit + INSET_U <= d0 + SEARCH_U:
				tries.append(hit + INSET_U)  # set into the wall, the look we want
			var d := d0
			while d <= d0 + SEARCH_U:
				tries.append(d)
				d += STEP_U
			for t in tries:
				spot = _check(ctx, c, dir, t)
				if not spot.is_empty():
					break
			if not spot.is_empty():
				break
		if spot.is_empty():
			out.append({"none": true, "notch": false})
			continue
		spot["face"] = (c - spot.center).normalized()
		out.append(spot)
	return out


## The directions to try: SPOT_PREF, then every 10° around by how far it turns from the first.
static func _directions() -> Array:
	var out: Array = SPOT_PREF.duplicate()
	var rest: Array = []
	for deg in range(-170, 190, 10):
		if not deg in out:
			rest.append(deg)
	rest.sort_custom(func(x, y): return absi(x - SPOT_PREF[0]) < absi(y - SPOT_PREF[0]))
	return out + rest


## What `spots` checks against: the walls (and their bounds), the map's size, the boulders and
## the structures.
static func _context(geo: Dictionary) -> Dictionary:
	var walls: Array = geo.get("walls", [])
	var rects: Array = walls.map(func(w): return _rect(w))
	var outcrops: Array = []
	for i in (geo.get("cliffs", {}).get("outcrops", PackedInt32Array()) as PackedInt32Array):
		if i < walls.size():
			outcrops.append(walls[i])
	var structures: Array = []
	for st in geo.get("structures", []):
		structures.append(st.pos if st is Dictionary else st)
	var size = geo.get("size", Vector2.ZERO)
	if not size is Vector2:
		size = Vector2(float(size), float(size))
	return {"walls": walls, "rects": rects, "size": size, "outcrops": outcrops, "structures": structures}


static func _rect(poly: PackedVector2Array) -> Rect2:
	var r := Rect2(poly[0], Vector2.ZERO)
	for q in poly:
		r = r.expand(q)
	return r


## Whether `p` (u) is somewhere a champion can't stand: in a wall or off the map.
static func _solid(ctx: Dictionary, p: Vector2) -> bool:
	var size: Vector2 = ctx.size
	if p.x < 0.0 or p.y < 0.0 or p.x > size.x or p.y > size.y:
		return true
	for i in ctx.walls.size():
		if (ctx.rects[i] as Rect2).grow(1.0).has_point(p) and Geometry2D.is_point_in_polygon(p, ctx.walls[i]):
			return true
	return false


## A circle of `r` around `c`, as a polygon.
static func _circle(c: Vector2, r: float, n := 24) -> PackedVector2Array:
	var out := PackedVector2Array()
	for i in n:
		var a := TAU * i / n
		out.append(c + Vector2(cos(a), sin(a)) * r)
	return out


static func _touches(poly: PackedVector2Array, others: Array) -> bool:
	var r := _rect(poly)
	for o in others:
		if _rect(o).intersects(r) and not Geometry2D.intersect_polygons(poly, o).is_empty():
			return true
	return false


## The stall `d` u from the fountain at `c` along `dir`, if it can stand there: clear of the
## structures and boulders; on open ground, or set into a wall in a nook that leaves rock BACK_U
## thick around it and opens only onto the ground in front of it. Empty if it can't.
static func _check(ctx: Dictionary, c: Vector2, dir: Vector2, d: float) -> Dictionary:
	var center := c + dir * d
	var r := PLINTH_R_M * 100.0
	for st in ctx.structures:
		if center.distance_to(st) < r + STRUCTURE_CLEAR_U:
			return {}
	var foot := _circle(center, r)
	if _touches(foot, ctx.outcrops):
		return {}
	var touching := _solid(ctx, center)
	for q in foot:
		touching = touching or _solid(ctx, q)
	if not touching:
		return {"center": center, "notch": false}
	# Set into a wall, no deeper than DEEPEST_U past its face along the way from the fountain. The
	# nook runs back toward the fountain past that face and past the whole plinth, so it opens
	# onto the ground in front however the plinth meets the wall (ahead, or at its side).
	var hit := _wall_hit(ctx, c, dir, d + r + STEP_U)
	if hit < INF and d - hit > DEEPEST_U:
		return {}
	var front := d - hit if hit < INF else -r
	var reach := clampf(maxf(r + 40.0, front + 150.0), 0.0, d - 100.0)
	if reach < r:
		return {}  # too near the fountain to open in front of the plinth
	var spot := {"center": center, "notch": true, "dir": dir, "reach": reach}
	var mouth := center - dir * reach
	if _solid(ctx, mouth):
		return {}
	var u := alcove(spot)
	if _touches(u, ctx.outcrops):
		return {}
	# Everything around the nook within BACK_U is rock, or ground the nook's mouth opens onto in
	# plain sight (a wall that slants away): never another way through.
	var around: Array = Geometry2D.offset_polygon(u, BACK_U)
	if around.is_empty():
		return {}
	var ring: PackedVector2Array = around[0]
	for i in ring.size():
		var a: Vector2 = ring[i]
		var b: Vector2 = ring[(i + 1) % ring.size()]
		var n := maxi(1, ceili(a.distance_to(b) / 40.0))
		for j in n:
			var q := a.lerp(b, float(j) / n)
			if _solid(ctx, q):
				continue
			if not _clear_view(ctx, u, q, mouth):
				return {}
	return spot


## Whether the straight line from `p` to `mouth` crosses no wall outside the nook `u`.
static func _clear_view(ctx: Dictionary, u: PackedVector2Array, p: Vector2, mouth: Vector2) -> bool:
	var n := maxi(1, ceili(p.distance_to(mouth) / STEP_U))
	for j in range(1, n):
		var q := p.lerp(mouth, float(j) / n)
		if _solid(ctx, q) and not Geometry2D.is_point_in_polygon(q, u):
			return false
	return true


## The alcove cut into the wall for `spot` (units): a U around the stall, a half circle of
## ALCOVE_R_U behind its center and straight sides running `reach` back toward the fountain.
static func alcove(spot: Dictionary) -> PackedVector2Array:
	var c: Vector2 = spot.center
	var dir: Vector2 = spot.dir
	var side := Vector2(-dir.y, dir.x)
	var mouth := -float(spot.reach)
	var out := PackedVector2Array()
	out.append(c + dir * mouth + side * ALCOVE_R_U)
	for k in 17:
		var a := PI / 2.0 - PI * k / 16.0
		out.append(c + dir * cos(a) * ALCOVE_R_U + side * sin(a) * ALCOVE_R_U)
	out.append(c + dir * mouth - side * ALCOVE_R_U)
	return out


## The cliffs' `pieces` (`map_cliff_pieces`: the walls, then the frame) with the alcoves of `spots`
## cut out. The pieces a nook touches are merged first, so no seam between them is cut on each
## side apart; the result takes the first one's place and the others are left empty, so the
## walls keep their indices (for the boulders). A piece the cut would leave with a hole (it
## can't: a nook opens onto the ground) is kept whole.
static func cut_pieces(pieces: Array, spots: Array) -> Array:
	var out: Array = pieces.duplicate()
	for s in spots:
		if not s.get("notch", false):
			continue
		var u := alcove(s)
		var ur := _rect(u)
		var touched: Array = []
		for i in out.size():
			var piece: PackedVector2Array = out[i]
			if piece.size() >= 3 and _rect(piece).intersects(ur) and not Geometry2D.intersect_polygons(piece, u).is_empty():
				touched.append(i)
		if touched.is_empty():
			continue
		var merged: PackedVector2Array = out[touched[0]]
		var parts: Array = []
		for j in range(1, touched.size()):
			var m: Array = Geometry2D.merge_polygons(merged, out[touched[j]])
			if m.size() == 1:
				merged = m[0]
			else:
				parts.append(out[touched[j]])  # not joined to the rest: cut on its own
		parts.push_front(merged)
		var cut: Array = []
		for part in parts:
			var res: Array = Geometry2D.clip_polygons(part, u)
			var sides := {}
			for r in res:
				sides[Geometry2D.is_polygon_clockwise(r)] = true
			if sides.size() > 1:
				cut.append(part)
			else:
				cut.append_array(res)
		for j in touched.size():
			out[touched[j]] = PackedVector2Array()
		if not cut.is_empty():
			out[touched[0]] = cut[0]
			out.append_array(cut.slice(1))
	return out


## How far from a fountain's center along `dir` the plinth must stand to keep CLEAR_U from every
## brazier and banner on its platform (the platform drawn `k` times its built size).
static func _clear_of_props(dir: Vector2, k: float) -> float:
	var need := PLINTH_R_M * 100.0 + CLEAR_U
	var props: Array = []
	for i in 8:
		var a := PI / 4.0 * i
		props.append(Vector2(cos(a), sin(a)) * (BRAZIER_R_U if i % 2 == 1 else BANNER_R_U) * k)
	var d := 0.0
	while d < 3000.0:
		var ok := true
		for p in props:
			if (dir * d).distance_to(p) < need:
				ok = false
				break
		if ok and d > BANNER_R_U * k:
			return d
		d += 10.0
	return d


## How far from `from` along `dir` the ground stops being walkable (a wall or the map's edge),
## up to `reach` (INF past it).
static func _wall_hit(ctx: Dictionary, from: Vector2, dir: Vector2, reach: float) -> float:
	var t := 0.0
	while t <= reach:
		if _solid(ctx, from + dir * t):
			return t
		t += STEP_U
	return INF


## The stall for `key` at `spot` (a node; its props from art/props, drawn SCALE times their size):
## the stall on its plinth, and `season`'s dressing on the plinth's top (`dress` names it).
static func stall_node(key: String, spot: Dictionary, season: String, team: Color) -> Node3D:
	var node := Node3D.new()
	node.name = "Shop_" + key
	var at: Vector2 = spot.center
	node.position = Vector3(at.x * U, 0.0, at.y * U)
	node.scale = Vector3.ONE * SCALE
	var face: Vector2 = spot.face
	node.rotation.y = atan2(face.x, face.y)  # the keeper faces the platform
	var core := MapView.prop_node("stall_" + key, team)
	if core != null:
		core.layers = MapView.LAYERS
		node.add_child(core)
	var dress := MapView.prop_node("stall_%s_%s" % [key, season if season in ["summer", "autumn", "winter"] else "summer"], team)
	if dress != null:
		dress.name = "Dress"
		dress.position.y = PLINTH_M
		dress.layers = MapView.LAYERS
		node.add_child(dress)
	return node


## Set up the shops for the map `geo` (its `fountains`, blue's first, and `shop_spots`) and
## `season`. They appear once the pair is known (`update` asks the scoreboard).
func place(geo: Dictionary, season: String, ally: Color, enemy: Color) -> void:
	_rng.randomize()
	var at: Array = geo.get("shop_spots", spots(geo))
	_placed = {"fountains": geo.get("fountains", []), "spots": at, "season": season, "ally": ally, "enemy": enemy}
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
	_season = p.season
	for i in p.fountains.size():
		var f: Dictionary = p.fountains[i]
		var key: String = keys_by_side[i] if i < keys_by_side.size() else KEYS[0]
		if i < p.spots.size() and not p.spots[i].get("none", false):
			_build_shop(key, f, p.spots[i], p.season, p.ally if f.ally else p.enemy)


func _build_shop(key: String, f: Dictionary, spot: Dictionary, season: String, team: Color) -> void:
	var at: Vector2 = spot.center
	var node := stall_node(key, spot, season, team)
	add_child(node)
	var shop := {"key": key, "ally": f.ally, "center": at, "node": node, "skel": null, "anim": null, "model": null,
		"team": team, "dress": node.get_node_or_null("Dress"), "winter_bones": [],
		"clip": "idle", "t": _rng.randf_range(0.0, 4.0), "from": "idle", "from_t": 0.0, "fade": 1.0,
		"wait": 1.0 if tour else _rng.randf_range(FIDGET_EVERY.x, FIDGET_EVERY.y), "last": "", "held": false, "yaw": 0.0}
	var path := art_root.path_join("shopkeepers/%s/export/%s.glb" % [key, key]).simplify_path()
	var pack: MftrModel = MftrModel.load(path) if FileAccess.file_exists(path) else null
	if pack != null:
		var model: Node3D = pack.instantiate()
		var keep: Vector2 = KEEP_AT[key]
		model.position = Vector3(keep.x, PLINTH_M, keep.y)
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
		var skel: Skeleton3D = shop.skel
		for i in skel.get_bone_count():
			if skel.get_bone_name(i).begins_with("extra_winter"):
				shop.winter_bones.append(i)
	shops.append(shop)


## The stall's dressing for `season` (its own pack, on the plinth's top).
func _dress(shop: Dictionary, season: String) -> void:
	if shop.dress != null:
		shop.dress.queue_free()
		shop.dress = null
	var dress := MapView.prop_node("stall_%s_%s" % [shop.key, season if season in ["summer", "autumn", "winter"] else "summer"], shop.team)
	if dress != null:
		dress.position.y = PLINTH_M
		dress.layers = MapView.LAYERS
		shop.node.add_child(dress)
		shop.dress = dress


## The season changed mid-match: each stall changes its dressing, each keeper its winter things.
func set_season(season: String) -> void:
	_season = season
	if not _pending.is_empty():
		_pending.season = season
	if not _placed.is_empty():
		_placed.season = season
	for s in shops:
		_dress(s, season)


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
	if _season != "winter":
		for i in s.winter_bones:
			(s.skel as Skeleton3D).set_bone_pose_scale(i, Vector3.ZERO)
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
