extends RefCounted
## Item icons (05 §6, 18 §8): each item's model (`art/items/<key>/export/<key>.glb`, a prop pack)
## rendered by the portrait renderer (`portraits.gd`, an "item:<key>" render) and laid on a
## backdrop in its tier's colors with a rim, so the tiers read apart at a glance. Until a model
## is rendered, or for an item without one (a mod's, say), the drawing in
## `art/items/icons/<key>.svg` stands in, else its main stat's glyph. The texture an icon
## returns is updated in place when its model's render arrives, so everything showing it
## follows. A pack can replace any model or drawing.

const Hud := preload("res://scripts/hud.gd")

## Per tier (`items::Tier`: starter, component, epic, legendary, boots, consumable, support,
## jungle): backdrop top, bottom, rim.
const TIERS := [
	["#4c3a26", "#1a120a", "#c58d4c"],
	["#334050", "#121820", "#6a7686"],
	["#2f4a44", "#0f1a17", "#7fa59a"],
	["#4a2f5c", "#170f1f", "#e0b65a"],
	["#2c4a30", "#0e1a10", "#8cc27a"],
	["#3a3440", "#141018", "#a491ac"],
	["#4c3a26", "#1a120a", "#c58d4c"],
	["#2c4a30", "#0e1a10", "#8cc27a"],
]
## A tier's color for item names (tooltips).
const NAME_COLORS := ["e0b07a", "d6dbe2", "8fd8c8", "f2d27a", "a8e08f", "d6c6dc", "e0b07a", "a8e08f"]
## The fallback's glyph for an item's first stat (its `stats` text, e.g. "+10 AD").
const STAT_GLYPH := {"AD": "ad", "AP": "ap", "HP": "armor", "armor": "armor", "MR": "mr", "AS": "as", "MS": "ms", "haste": "haste"}
const PX := 128

static var art_root := ""                # set by main: where `art/` is
## The portrait renderer (main sets it): `portrait("item:<key>", "icon")` is the model's render,
## or null while it's queued.
static var renderer = null
static var _cache := {}                  # "name|tier" -> ImageTexture
static var _waiting := {}                # "item:<key>" -> [[ImageTexture, tier]]


## The file key of an item's name ("Pilgrim's Ledger" -> "pilgrims_ledger").
static func key_of(name: String) -> String:
	return name.to_lower().replace("'", "").replace(" ", "_")


static func name_color(tier: int) -> String:
	return NAME_COLORS[clampi(tier, 0, NAME_COLORS.size() - 1)]


## The icon for a catalog entry (`{ name, tier, stats }`), cached.
static func icon(item: Dictionary) -> Texture2D:
	var name: String = item.get("name", "?")
	var tier: int = clampi(int(item.get("tier", 1)), 0, TIERS.size() - 1)
	var key := "%s|%d" % [name, tier]
	if _cache.has(key):
		return _cache[key]
	var body := _art(name)
	if body == "":
		body = _fallback(String(item.get("stats", "")))
	var t: Array = TIERS[tier]
	var svg := '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">%s<g>%s</g>%s</svg>' % [_backdrop_svg(t), body, _rim_svg(t)]
	var img := Image.new()
	img.load_svg_from_string(svg, PX / 64.0)
	img.generate_mipmaps()
	var tex := ImageTexture.create_from_image(img)
	_cache[key] = tex
	# The model's render, when it's ready.
	if renderer != null:
		var model_key := "item:" + key_of(name)
		var shot: Texture2D = renderer.portrait(model_key, "icon")
		if shot != null:
			_compose(tex, shot, tier)
		else:
			if not _waiting.has(model_key):
				_waiting[model_key] = []
			_waiting[model_key].append([tex, tier])
	return tex


## A render finished (`portraits.rendered`): the icons waiting for it take the model.
static func on_rendered(model_key: String) -> void:
	if not _waiting.has(model_key) or renderer == null:
		return
	var shot: Texture2D = renderer.portrait(model_key, "icon")
	if shot != null:
		for w in _waiting[model_key]:
			_compose(w[0], shot, w[1])
	_waiting.erase(model_key)


## The tier's backdrop, the model's render over it, the rim on top: into `tex`, in place.
static func _compose(tex: ImageTexture, shot: Texture2D, tier: int) -> void:
	var t: Array = TIERS[tier]
	var base := _svg_image('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">%s</svg>' % _backdrop_svg(t))
	var model: Image = shot.get_image()
	model.clear_mipmaps()
	model.convert(Image.FORMAT_RGBA8)
	if model.get_width() != PX:
		model.resize(PX, PX, Image.INTERPOLATE_LANCZOS)
	base.blend_rect(model, Rect2i(0, 0, PX, PX), Vector2i.ZERO)
	var rim := _svg_image('<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">%s</svg>' % _rim_svg(t))
	base.blend_rect(rim, Rect2i(0, 0, PX, PX), Vector2i.ZERO)
	base.generate_mipmaps()
	tex.update(base)


static func _svg_image(svg: String) -> Image:
	var img := Image.new()
	img.load_svg_from_string(svg, PX / 64.0)
	img.convert(Image.FORMAT_RGBA8)
	return img


static func _backdrop_svg(t: Array) -> String:
	return '<defs><radialGradient id="b" cx=".5" cy=".38" r=".75"><stop offset="0" stop-color="%s"/><stop offset="1" stop-color="%s"/></radialGradient></defs><rect width="64" height="64" fill="url(#b)"/>' % [t[0], t[1]]


static func _rim_svg(t: Array) -> String:
	return '<rect x="1" y="1" width="62" height="62" fill="none" stroke="%s" stroke-width="2"/>' % t[2]


## The drawing inside an item's SVG file, or "" without one.
static func _art(name: String) -> String:
	if art_root == "":
		return ""
	var path := art_root.path_join("items/icons/%s.svg" % key_of(name))
	if not FileAccess.file_exists(path):
		return ""
	var s := FileAccess.get_file_as_string(path)
	var open := s.find("<svg")
	var start := s.find(">", open) + 1
	var end := s.rfind("</svg>")
	return s.substr(start, end - start) if open >= 0 and end > start else ""


## A stand-in: the item's main stat's glyph, large.
static func _fallback(stats: String) -> String:
	var glyph := "gold"
	for word in stats.replace(",", " ").split(" "):
		if STAT_GLYPH.has(word):
			glyph = STAT_GLYPH[word]
			break
	return '<g transform="translate(10 10) scale(1.85)">%s</g>' % Hud.STAT_ICONS[glyph]
