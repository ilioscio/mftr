extends RefCounted
## Item icons (05 §6): `art/items/icons/<name>.svg` (the item's name in snake case), drawn on
## a backdrop in its tier's colors with a rim, so components, upgrades and legendaries read
## apart at a glance. An item without one (a mod's, say) gets an icon from its main stat. The
## art is plain SVG files next to the game, so a pack can replace any of them.

const Hud := preload("res://scripts/hud.gd")

# Per tier (0 components, 1 upgrades and boots, 2 legendary): backdrop top, bottom, rim.
const TIERS := [
	["#334050", "#121820", "#6a7686"],
	["#2f4a44", "#0f1a17", "#7fa59a"],
	["#4a2f5c", "#170f1f", "#e0b65a"],
]
# The fallback's glyph for an item's first stat (its `stats` text, e.g. "+10 AD").
const STAT_GLYPH := {"AD": "ad", "AP": "ap", "HP": "armor", "armor": "armor", "MR": "mr", "AS": "as", "MS": "ms", "haste": "haste"}

static var art_root := ""                # set by main: where `art/` is
static var _cache := {}


## The icon for a catalog entry (`{ name, tier, stats }`), cached.
static func icon(item: Dictionary) -> Texture2D:
	var name: String = item.get("name", "?")
	var tier: int = clampi(int(item.get("tier", 0)), 0, TIERS.size() - 1)
	var key := "%s|%d" % [name, tier]
	if _cache.has(key):
		return _cache[key]
	var body := _art(name)
	if body == "":
		body = _fallback(String(item.get("stats", "")))
	var t: Array = TIERS[tier]
	var svg := '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><defs><radialGradient id="b" cx=".5" cy=".38" r=".75"><stop offset="0" stop-color="%s"/><stop offset="1" stop-color="%s"/></radialGradient></defs><rect width="64" height="64" fill="url(#b)"/><g>%s</g><rect x="1" y="1" width="62" height="62" fill="none" stroke="%s" stroke-width="2"/></svg>' % [t[0], t[1], body, t[2]]
	var img := Image.new()
	img.load_svg_from_string(svg, 2.0)
	img.generate_mipmaps()
	var tex := ImageTexture.create_from_image(img)
	_cache[key] = tex
	return tex


## The drawing inside an item's SVG file, or "" without one.
static func _art(name: String) -> String:
	if art_root == "":
		return ""
	var path := art_root.path_join("items/icons/%s.svg" % name.to_lower().replace(" ", "_"))
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
