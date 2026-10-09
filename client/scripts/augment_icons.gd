extends RefCounted
## Augment icons (06 §3): a glyph for the augment's mechanic (or, for a stat augment, its main
## stat) on a backdrop in its tier's colors: Silver steel, Gold gold, Prismatic shifting
## violet and teal. Generated as SVG (no image files); the extension says which glyph
## (`augment_card`'s `glyph`).

const Hud := preload("res://scripts/hud.gd")

const W := 'fill="#fff"'
const L := 'fill="none" stroke="#fff" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"'

static var GLYPHS := {
	# Two arrows chasing each other.
	"convert": '<path d="M16 26a16 16 0 0 1 28-8" %s/><path d="M48 12l-2 12-11-4z" %s/><path d="M48 38a16 16 0 0 1-28 8" %s/><path d="M16 52l2-12 11 4z" %s/>' % [L, W, L, W],
	# Three bolts fanning out.
	"multishot": '<path d="M14 50L44 20M20 54l36-14M10 44L26 12" %s/><path d="M50 14l-12 2 8 8z" %s/>' % [L, W],
	# A shape and its fading copy.
	"echo": '<circle cx="26" cy="32" r="12" %s/><circle cx="40" cy="32" r="12" fill="none" stroke="#fff" stroke-width="4" opacity=".5"/>' % W,
	# A wide beam.
	"wide": '<path d="M8 32h48" stroke="#fff" stroke-width="16" stroke-linecap="round" opacity=".85"/><path d="M8 20v24M56 20v24" %s/>' % L,
	# A giant: a big figure.
	"titan": '<circle cx="32" cy="16" r="8" %s/><path d="M18 56l4-28h20l4 28z" %s/>' % [W, W],
	"pebble": '<circle cx="32" cy="38" r="9" %s/><path d="M20 22l-6-6M44 22l6-6M32 18v-8" %s/>' % [W, L],
	# A die.
	"unstable": '<rect x="14" y="14" width="36" height="36" rx="7" %s/><g fill="#1b140e"><circle cx="24" cy="24" r="4"/><circle cx="40" cy="40" r="4"/><circle cx="32" cy="32" r="4"/></g>' % W,
	# A falling blade.
	"execute": '<path d="M32 6l8 30-8 8-8-8z" %s/><path d="M18 46h28M32 46v12" %s/>' % [W, L],
	# A lightning strike.
	"first_strike": '<path d="M36 6L16 36h14l-4 22 22-32H34z" %s/>' % W,
	# A cracked shield, still standing.
	"last_stand": '<path d="M32 8l20 7v14c0 13-9 22-20 27-11-5-20-14-20-27V15z" %s/><path d="M32 16l-5 12 8 6-5 14" fill="none" stroke="#1b140e" stroke-width="3.5" stroke-linecap="round"/>' % W,
	# A star burst.
	"spellcrit": '<path d="M32 4l6 20 20-6-14 14 14 14-20-6-6 20-6-20-20 6 14-14L6 18l20 6z" %s/>' % W,
	# An open mouth of fire.
	"hunger": '<path d="M32 8c10 10 16 18 16 28a16 16 0 0 1-32 0c0-6 3-11 8-16 0 6 3 9 6 10-2-8 0-15 2-22z" %s/>' % W,
	# A drop.
	"vamp": '<path d="M32 8c10 14 16 22 16 30a16 16 0 0 1-32 0c0-8 6-16 16-30z" %s/>' % W,
	"thorns": '<path d="M32 10l20 7v13c0 12-9 20-20 25-11-5-20-13-20-25V17z" %s/><path d="M8 20l8 4M56 20l-8 4M10 44l8-4M54 44l-8-4M32 2v8" %s/>' % [W, L],
	# A sword with a spark.
	"spellblade": '<path d="M14 50L44 20" stroke="#fff" stroke-width="7" stroke-linecap="round"/><path d="M48 10l-8 2 10 10 2-8z" %s/><path d="M10 44l10 10" %s/><path d="M50 40c1 5 3 7 8 8-5 1-7 3-8 8-1-5-3-7-8-8 5-1 7-3 8-8z" %s/>' % [W, L, W],
	# A circular arrow.
	"reset": '<path d="M48 32a16 16 0 1 1-6-12" %s/><path d="M48 8v14H34z" %s/>' % [L, W],
	"anvil": '<path d="M10 22h34c0 6 4 10 10 10H40v8l6 10H18l6-10v-8c-8 0-14-6-14-10z" %s/>' % W,
	# A swirl.
	"chaos": '<path d="M32 32m-4 0a4 4 0 1 1 8 0a10 10 0 1 1-20 0a16 16 0 1 1 32 0a22 22 0 0 1-22 22" %s/>' % L,
	# A book.
	"fundamentals": '<path d="M10 14h18c3 0 4 2 4 4v34c0-2-1-4-4-4H10zM54 14H36c-3 0-4 2-4 4v34c0-2 1-4 4-4h18z" %s/>' % W,
	# Two blades crossing close.
	"close": '<path d="M14 14l36 36M50 14L14 50" stroke="#fff" stroke-width="6" stroke-linecap="round"/>',
	# A scope.
	"sharpshooter": '<circle cx="32" cy="32" r="16" %s/><path d="M32 6v14M32 44v14M6 32h14M44 32h14" %s/><circle cx="32" cy="32" r="4" %s/>' % [L, L, W],
	# A spell orb.
	"spell": '<circle cx="32" cy="32" r="14" %s/><path d="M32 8v6M32 50v6M8 32h6M50 32h6M15 15l4 4M45 45l4 4M49 15l-4 4M19 45l-4 4" %s/>' % [W, L],
}
const STATS := ["hp", "ad", "ap", "armor", "mr", "as", "haste", "ms"]
const STAT_ICON := {"hp": "armor", "ad": "ad", "ap": "ap", "armor": "armor", "mr": "mr", "as": "as", "haste": "haste", "ms": "ms"}

static var _cache := {}


## The icon for an augment (`{ glyph, tier }`).
static func icon(a: Dictionary) -> Texture2D:
	var glyph: String = a.get("glyph", "spell")
	var tier: String = a.get("tier", "Silver")
	var key := glyph + "|" + tier
	if _cache.has(key):
		return _cache[key]
	var body: String
	if glyph in STATS:
		# A stat augment: its stat's glyph, large and white.
		body = '<g transform="translate(8 8) scale(2)">%s</g>' % Hud.STAT_ICONS[STAT_ICON[glyph]]
		if glyph == "hp":
			body = '<path d="M32 54C14 42 8 32 8 23c0-8 6-13 12-13 5 0 9 3 12 7 3-4 7-7 12-7 6 0 12 5 12 13 0 9-6 19-24 31z" fill="#5fe08a" stroke="#1b140e" stroke-width="2.4"/>'
	else:
		var g: String = GLYPHS.get(glyph, GLYPHS["spell"])
		body = '<g transform="translate(1.5 2)" opacity=".5">%s</g><g>%s</g>' % [g.replace("#fff", "#000"), g]
	var bg: Array = {
		"Silver": ['#6f7987', '#20252c', '#c9d0da'],
		"Gold": ['#8a6420', '#2b1d08', '#f2cd6a'],
		"Prismatic": ['#7a3fb0', '#123a44', '#e6b8ff'],
	}.get(tier, ['#6f7987', '#20252c', '#c9d0da'])
	var svg := '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><defs><linearGradient id="b" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="%s"/><stop offset="1" stop-color="%s"/></linearGradient></defs><rect x="1" y="1" width="62" height="62" rx="12" fill="url(#b)" stroke="%s" stroke-width="2.5"/><g transform="translate(6 6) scale(0.8125)">%s</g></svg>' % [bg[0], bg[1], bg[2], body]
	var img := Image.new()
	img.load_svg_from_string(svg, 2.0)
	img.generate_mipmaps()
	var tex := ImageTexture.create_from_image(img)
	_cache[key] = tex
	return tex
