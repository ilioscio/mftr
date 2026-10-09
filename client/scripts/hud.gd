extends RefCounted
## Drawing helpers for the in-game HUD (05 §6): framed panels, generated ability icons, cooldown
## sweeps, segmented health bars. Immediate-mode, on the overlay's canvas, scaled by `k` (1.0 at
## 1080 p).

const GOLD := Color(0.78, 0.65, 0.38)
const GOLD_DIM := Color(0.42, 0.35, 0.2)
const INK := Color(0.035, 0.045, 0.06, 0.94)
const INK_2 := Color(0.07, 0.085, 0.11, 0.96)
const TEXT := Color(0.86, 0.88, 0.91)
const DIM := Color(0.55, 0.6, 0.66)

# Ability glyphs (64×64, white on transparent); tinted per champion when drawn on the icon's
# backdrop. One per kind of effect, so every ability reads at a glance.
const GLYPHS := {
	# A bolt flying up-right, with a trail.
	"line": '<path d="M14 50L44 20" stroke="#fff" stroke-width="5" stroke-linecap="round" opacity=".45"/><path d="M30 34L48 16" stroke="#fff" stroke-width="7" stroke-linecap="round"/><path d="M52 12L36 16L48 28Z" fill="#fff"/><path d="M10 46l6 6M16 40l6 6" stroke="#fff" stroke-width="3" stroke-linecap="round" opacity=".5"/>',
	# A target ring with a burst inside.
	"area": '<circle cx="32" cy="34" r="20" fill="none" stroke="#fff" stroke-width="4" stroke-dasharray="7 5"/><path d="M32 22l4 8 9 1-7 6 2 9-8-5-8 5 2-9-7-6 9-1z" fill="#fff"/>',
	# Spikes radiating from the caster.
	"nova": '<circle cx="32" cy="32" r="8" fill="#fff"/><g stroke="#fff" stroke-width="5" stroke-linecap="round"><path d="M32 8v10M32 46v10M8 32h10M46 32h10M15 15l7 7M42 42l7 7M49 15l-7 7M22 42l-7 7"/></g>',
	# A curved leap ending in a strike.
	"lunge": '<path d="M10 52C14 28 28 18 44 18" fill="none" stroke="#fff" stroke-width="5" stroke-linecap="round" stroke-dasharray="2 8" opacity=".7"/><path d="M38 10l14 8-12 10z" fill="#fff"/><path d="M50 30l6 14M44 34l2 16" stroke="#fff" stroke-width="4" stroke-linecap="round"/>',
	# An arrow with speed lines.
	"dash": '<path d="M24 32h26" stroke="#fff" stroke-width="7" stroke-linecap="round"/><path d="M56 32L42 20v24z" fill="#fff"/><path d="M8 22h14M6 32h10M8 42h14" stroke="#fff" stroke-width="4" stroke-linecap="round" opacity=".6"/>',
	# Two sparkles joined by a dotted path.
	"blink": '<path d="M16 46l26-26" stroke="#fff" stroke-width="4" stroke-linecap="round" stroke-dasharray="1 8" opacity=".7"/><path d="M46 6c1.5 7 3.5 9 10 10-6.5 1.5-8.5 3.5-10 10-1.5-6.5-3.5-8.5-10-10 6.5-1 8.5-3 10-10z" fill="#fff"/><path d="M16 38c1 5 2.5 6.5 7 7.5-4.5 1-6 2.5-7 7.5-1-5-2.5-6.5-7-7.5 4.5-1 6-2.5 7-7.5z" fill="#fff" opacity=".8"/>',
	"barrier": '<path d="M32 8l20 7v14c0 13-9 22-20 27-11-5-20-14-20-27V15z" fill="none" stroke="#fff" stroke-width="5" stroke-linejoin="round"/><path d="M32 16v32" stroke="#fff" stroke-width="4" opacity=".6"/>',
	"heal": '<path d="M26 10h12v16h16v12H38v16H26V38H10V26h16z" fill="#fff"/>',
	# An eye on a stake: a ward.
	"ward": '<path d="M30 36h4l2 20h-8z" fill="#fff" opacity=".8"/><path d="M10 24c6-9 14-13 22-13s16 4 22 13c-6 9-14 13-22 13s-16-4-22-13z" fill="none" stroke="#fff" stroke-width="5"/><circle cx="32" cy="24" r="7" fill="#fff"/>',
	# A lens sweeping an arc.
	"lens": '<circle cx="26" cy="26" r="14" fill="none" stroke="#fff" stroke-width="5"/><path d="M36 36l16 16" stroke="#fff" stroke-width="7" stroke-linecap="round"/><path d="M8 50a40 40 0 0 1 10-30" fill="none" stroke="#fff" stroke-width="3" stroke-dasharray="3 5" opacity=".7"/>',
	# Three talons closing on a gem.
	"claim": '<path d="M14 10c10 6 14 16 14 28M32 6c3 10 3 20 0 32M50 10c-10 6-14 16-14 28" fill="none" stroke="#fff" stroke-width="5" stroke-linecap="round"/><path d="M32 40l8 8-8 10-8-10z" fill="#fff"/>',
	"shield_ally": '<path d="M32 8l20 7v14c0 13-9 22-20 27-11-5-20-14-20-27V15z" fill="#fff" opacity=".9"/><path d="M29 22h6v8h8v6h-8v8h-6v-8h-8v-6h8z" fill="#000" opacity=".55"/>',
}

# Small stat icons (24×24).
const STAT_ICONS := {
	"ad": '<path d="M4 20L18 6M14 4h6v6" stroke="#ff9a3c" stroke-width="2.6" stroke-linecap="round" fill="none"/><path d="M4 16l4 4" stroke="#ff9a3c" stroke-width="2.6" stroke-linecap="round"/>',
	"ap": '<path d="M12 2c.8 5.2 2.3 7.7 7.5 10-5.2 2.3-6.7 4.8-7.5 10-.8-5.2-2.3-7.7-7.5-10C9.7 9.7 11.2 7.2 12 2z" fill="#9b8cff"/>',
	"armor": '<path d="M12 2l8 3v6c0 5-3.5 9-8 11-4.5-2-8-6-8-11V5z" fill="#e0a85a"/>',
	"mr": '<path d="M12 2l8 3v6c0 5-3.5 9-8 11-4.5-2-8-6-8-11V5z" fill="#6fb6ff"/><path d="M12 7l1.5 3.5 3.5.3-2.7 2.3.9 3.6-3.2-2-3.2 2 .9-3.6-2.7-2.3 3.5-.3z" fill="#e8f2ff"/>',
	"as": '<path d="M4 20l6-6M8 20l6-6M12 20l8-8" stroke="#ffd24a" stroke-width="2.4" stroke-linecap="round"/><path d="M14 4h6v6z" fill="#ffd24a"/>',
	"haste": '<circle cx="12" cy="13" r="8" fill="none" stroke="#c9d3df" stroke-width="2.2"/><path d="M12 8v5l3 2" stroke="#c9d3df" stroke-width="2.2" stroke-linecap="round" fill="none"/>',
	"ms": '<path d="M5 16c3 0 4-8 9-8 3 0 5 2 6 5l-2 3H5z" fill="#d8dee6"/><path d="M3 20h18" stroke="#d8dee6" stroke-width="2" stroke-linecap="round"/>',
	"gold": '<circle cx="12" cy="12" r="8.5" fill="#f2c14e" stroke="#9a6b10" stroke-width="1.6"/><circle cx="12" cy="12" r="4.5" fill="none" stroke="#9a6b10" stroke-width="1.4"/>',
}

static var _cache := {}
static var _panels := {}


static func _svg(key: String, svg: String, scale: float) -> Texture2D:
	if not _cache.has(key):
		var img := Image.new()
		img.load_svg_from_string(svg, scale)
		_cache[key] = ImageTexture.create_from_image(img)
	return _cache[key]


## An ability's icon: the glyph for `kind` over a backdrop in the champion's color.
static func ability_icon(kind: String, color: Color) -> Texture2D:
	var glyph: String = GLYPHS.get(kind, GLYPHS["line"])
	# The backdrop stays dark enough for a white glyph, whatever the champion's color.
	var base := Color.from_hsv(color.h, color.s, minf(color.v, 0.6))
	var hi := base.lightened(0.2).to_html(false)
	var lo := base.darkened(0.6).to_html(false)
	var shadow := glyph.replace("#fff", "#000")
	var svg := '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64"><defs><radialGradient id="g" cx=".35" cy=".3" r=".9"><stop offset="0" stop-color="#%s"/><stop offset="1" stop-color="#%s"/></radialGradient></defs><rect width="64" height="64" fill="url(#g)"/><g transform="translate(1.5 2)" opacity=".55">%s</g><g opacity=".95">%s</g></svg>' % [hi, lo, shadow, glyph]
	return _svg("ab:%s:%s" % [kind, hi], svg, 2.0)


static func stat_icon(name: String) -> Texture2D:
	return _svg("st:" + name, '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">%s</svg>' % STAT_ICONS[name], 2.0)


## A framed panel: dark fill, a thin gold rim, rounded corners.
static func panel(ci: CanvasItem, rect: Rect2, k: float, fill := INK, rim := GOLD_DIM) -> void:
	var key := "%s|%s|%d" % [fill, rim, roundi(k * 10.0)]
	if not _panels.has(key):
		var sb := StyleBoxFlat.new()
		sb.bg_color = fill
		sb.border_color = rim
		sb.set_border_width_all(maxi(1, roundi(1.5 * k)))
		sb.set_corner_radius_all(roundi(6.0 * k))
		sb.shadow_color = Color(0, 0, 0, 0.45)
		sb.shadow_size = roundi(6.0 * k)
		sb.anti_aliasing = true
		_panels[key] = sb
	ci.draw_style_box(_panels[key], rect)


## A clockwise cooldown sweep over `rect`: `frac` of it (from 12 o'clock) still dark.
static func cooldown_sweep(ci: CanvasItem, rect: Rect2, frac: float) -> void:
	if frac <= 0.01:
		return  # a sliver: nothing to draw (and too thin to triangulate)
	var c := rect.get_center()
	var r := rect.size.length()  # past the corners; clipped to the square below
	# The dark part runs from where the sweep has reached, clockwise, back to 12 o'clock; the
	# square's corners are added where it passes them, so they stay square.
	var start := -PI / 2.0 + TAU * (1.0 - frac)
	var end := 1.5 * PI
	var angles := [start]
	for corner in [-PI / 4.0, PI / 4.0, 3.0 * PI / 4.0, 5.0 * PI / 4.0]:
		if corner > start and corner < end:
			angles.append(corner)
	angles.append(end)
	var pts := PackedVector2Array([c])
	for a in angles:
		pts.append(_clip_to_rect(c, Vector2(cos(a), sin(a)) * r, rect))
	ci.draw_colored_polygon(pts, Color(0, 0, 0, 0.62))


static func _clip_to_rect(c: Vector2, d: Vector2, rect: Rect2) -> Vector2:
	var t := 1.0
	if d.x != 0.0:
		t = minf(t, ((rect.end.x if d.x > 0.0 else rect.position.x) - c.x) / d.x)
	if d.y != 0.0:
		t = minf(t, ((rect.end.y if d.y > 0.0 else rect.position.y) - c.y) / d.y)
	return c + d * t


## A horizontal bar: a dark well, the value with a lighter top half, optional segment ticks
## every `tick` units of `max_v` (a heavier one every 10).
static func bar(ci: CanvasItem, rect: Rect2, value: float, max_v: float, color: Color, extra := 0.0, extra_color := Color(0.95, 0.95, 0.95), tick := 0.0, chip := 0.0) -> void:
	ci.draw_rect(rect.grow(1.0), Color(0, 0, 0, 0.85))
	ci.draw_rect(rect, Color(0.08, 0.09, 0.1))
	var total := maxf(max_v, value + extra)
	if total <= 0.0:
		return
	var w := rect.size.x * clampf(value / total, 0.0, 1.0)
	if chip > value:
		# Recent damage lingers as a pale strip, then drains.
		var wc := rect.size.x * clampf(chip / total, 0.0, 1.0)
		ci.draw_rect(Rect2(rect.position, Vector2(wc, rect.size.y)), Color(1.0, 0.92, 0.8, 0.85))
	ci.draw_rect(Rect2(rect.position, Vector2(w, rect.size.y)), color)
	ci.draw_rect(Rect2(rect.position, Vector2(w, rect.size.y * 0.45)), color.lightened(0.22))
	if extra > 0.0:
		var we := rect.size.x * clampf(extra / total, 0.0, 1.0)
		ci.draw_rect(Rect2(rect.position + Vector2(w, 0), Vector2(we, rect.size.y)), extra_color)
	if tick > 0.0 and total / tick < 80.0:
		var n := int(total / tick)
		for i in range(1, n + 1):
			var x := rect.position.x + rect.size.x * (i * tick) / total
			if x >= rect.end.x - 1.0:
				break
			var heavy := i % 10 == 0
			var h := rect.size.y if heavy else rect.size.y * 0.55
			ci.draw_line(Vector2(x, rect.position.y), Vector2(x, rect.position.y + h), Color(0, 0, 0, 0.75 if heavy else 0.5), 2.0 if heavy else 1.0)


## Text with a soft dark outline, for numbers over busy art.
static func text(ci: CanvasItem, font: Font, pos: Vector2, s: String, size: int, color: Color, align := HORIZONTAL_ALIGNMENT_LEFT, width := -1.0) -> void:
	ci.draw_string_outline(font, pos, s, align, width, size, maxi(2, size / 6), Color(0, 0, 0, 0.85))
	ci.draw_string(font, pos, s, align, width, size, color)
