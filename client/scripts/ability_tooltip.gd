extends PanelContainer
## The ability bar's hover tooltip (01 §13): what an ability does, its numbers at the current rank
## with their scaling, every rank's values, and what the next rank changes, colored the way
## MOBA players read them (physical orange, magic blue, AP green, hard CC gold, slows light
## blue). Everything comes from the sim's own data (`MftrClient.ability_info`), so a tooltip
## can't drift from the game.

const WIDTH := 400.0

# Text colors, as hex for BBCode.
const C_PHYSICAL := "ff9a3c"
const C_MAGIC := "6fb6ff"
const C_TRUE := "ffffff"
const C_AD := "ffb066"
const C_AP := "7ee07e"
const C_HEAL := "5fe08a"
const C_SHIELD := "d8e4f0"
const C_CC := "ffd24a"
const C_SLOW := "8fd8ff"
const C_DIM := "8a93a0"
const C_TEXT := "d6dbe2"
const C_NUM := "ffffff"
const C_UP := "8fe36f"

# Status icons, 24×24 SVG, drawn inline at the text's height.
const ICONS := {
	# A dizzy star.
	"stun": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M12 2l2.6 6.3 6.8.5-5.2 4.4 1.6 6.6L12 16.3 6.2 19.8l1.6-6.6L2.6 8.8l6.8-.5z" fill="#ffd24a" stroke="#7a5a00" stroke-width="1.2" stroke-linejoin="round"/></svg>',
	# Chains around a boot: two linked rings.
	"root": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><ellipse cx="8.5" cy="12" rx="6" ry="4" fill="none" stroke="#c9b27a" stroke-width="2.6"/><ellipse cx="15.5" cy="12" rx="6" ry="4" fill="none" stroke="#e8d49a" stroke-width="2.6"/><path d="M4 21h16" stroke="#7aa35a" stroke-width="2.4" stroke-linecap="round"/></svg>',
	# An upward arrow over the ground.
	"knockup": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M12 2l7 8h-4.2v7H9.2v-7H5z" fill="#ffd24a" stroke="#7a5a00" stroke-width="1.2" stroke-linejoin="round"/><path d="M3 21h18" stroke="#c9a46a" stroke-width="2.4" stroke-linecap="round"/></svg>',
	# A hook pulling inward.
	"pull": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M20 4v9a6 6 0 0 1-12 0v-2" fill="none" stroke="#ffd24a" stroke-width="2.8" stroke-linecap="round"/><path d="M4 12l4-4 4 4" fill="none" stroke="#ffd24a" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round"/></svg>',
	# Down-pointing chevrons.
	"slow": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M5 5l7 6 7-6M5 12l7 6 7-6" fill="none" stroke="#8fd8ff" stroke-width="2.8" stroke-linecap="round" stroke-linejoin="round"/></svg>',
	"shield": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M12 2l8 3v6c0 5-3.5 9-8 11-4.5-2-8-6-8-11V5z" fill="#c6d4e2" stroke="#55606c" stroke-width="1.4" stroke-linejoin="round"/><path d="M12 5v14" stroke="#ffffff" stroke-width="1.6" opacity=".7"/></svg>',
	"heal": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M9 3h6v6h6v6h-6v6H9v-6H3V9h6z" fill="#5fe08a" stroke="#1f6b39" stroke-width="1.4" stroke-linejoin="round"/></svg>',
	# Crossed blades.
	"physical": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M4 4l11 11M20 4L9 15" stroke="#ff9a3c" stroke-width="2.6" stroke-linecap="round"/><path d="M13 17l4 4M11 17l-4 4" stroke="#b86a22" stroke-width="2.6" stroke-linecap="round"/></svg>',
	# A four-point spark.
	"magic": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M12 1.5c.8 5.2 2.3 7.7 7.5 10.5-5.2 2.8-6.7 5.3-7.5 10.5-.8-5.2-2.3-7.7-7.5-10.5C9.7 9.2 11.2 6.7 12 1.5z" fill="#6fb6ff" stroke="#24508a" stroke-width="1.2" stroke-linejoin="round"/></svg>',
	"true": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><path d="M12 2l9 10-9 10-9-10z" fill="#ffffff" stroke="#80868f" stroke-width="1.4" stroke-linejoin="round"/></svg>',
	"cooldown": '<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><circle cx="12" cy="13" r="8.5" fill="none" stroke="#b9c3cf" stroke-width="2.2"/><path d="M12 8v5l3.5 2" fill="none" stroke="#b9c3cf" stroke-width="2.2" stroke-linecap="round"/><path d="M9 2h6" stroke="#b9c3cf" stroke-width="2.2" stroke-linecap="round"/></svg>',
}

static var _textures := {}

var _text: RichTextLabel
var _shown_key := ""


func _init() -> void:
	mouse_filter = Control.MOUSE_FILTER_IGNORE
	visible = false
	custom_minimum_size = Vector2(WIDTH, 0)
	var sb := StyleBoxFlat.new()
	sb.bg_color = Color(0.035, 0.05, 0.07, 0.96)
	sb.border_color = Color(0.62, 0.52, 0.3)
	sb.set_border_width_all(1)
	sb.set_corner_radius_all(3)
	sb.set_content_margin_all(12)
	sb.shadow_color = Color(0, 0, 0, 0.5)
	sb.shadow_size = 6
	add_theme_stylebox_override("panel", sb)
	_text = RichTextLabel.new()
	_text.bbcode_enabled = true
	_text.fit_content = true
	_text.scroll_active = false
	_text.mouse_filter = Control.MOUSE_FILTER_IGNORE
	_text.custom_minimum_size = Vector2(WIDTH - 24, 0)
	_text.add_theme_font_size_override("normal_font_size", 14)
	_text.add_theme_font_size_override("bold_font_size", 14)
	_text.add_theme_color_override("default_color", Color.html(C_TEXT))
	add_child(_text)


static func icon(name: String) -> Texture2D:
	if not _textures.has(name):
		var img := Image.new()
		img.load_svg_from_string(ICONS[name], 2.0)
		_textures[name] = ImageTexture.create_from_image(img)
	return _textures[name]


## Shows the tooltip for `info` (from `ability_info`) above `anchor` (overlay coordinates),
## clamped to `bounds`. `key` is the cast binding; `level_key` the level-up binding.
func show_for(info: Dictionary, key: String, level_key: String, anchor: Rect2, bounds: Vector2) -> void:
	if info.is_empty():
		hide()
		return
	var shown := "%s|%s|%s" % [info, key, level_key]
	if shown != _shown_key or not visible:
		_shown_key = shown
		_build(info, key, level_key)
		reset_size()
	visible = true
	var sz := get_combined_minimum_size()
	var pos := Vector2(anchor.position.x + anchor.size.x / 2.0 - sz.x / 2.0, anchor.position.y - sz.y - 14.0)
	pos.x = clampf(pos.x, 8.0, bounds.x - sz.x - 8.0)
	pos.y = maxf(pos.y, 8.0)
	position = pos


func _build(info: Dictionary, key: String, level_key: String) -> void:
	var t := _text
	t.clear()
	var rank: int = info.rank
	var max_rank: int = info.max_rank
	var ranked: bool = info.ranked
	var now := clampi(rank, 1, max_rank) - 1  # index into the per-rank lists

	# Title line: name, binding, rank.
	t.append_text("[font_size=19][b][color=#f2e2b0]%s[/color][/b][/font_size]" % info.name)
	t.append_text("  [color=#%s][%s][/color]" % [C_DIM, key])
	if ranked:
		t.append_text("\n[color=#%s]Rank %d / %d[/color]" % [C_DIM, rank, max_rank] if rank > 0 else "\n[color=#%s]Not learned yet[/color]" % C_DIM)
	t.append_text("\n")

	# Facts: cooldown, range, size, speed, cast time.
	var cds: PackedFloat32Array = info.cooldowns
	t.add_image(icon("cooldown"), 17, 17)
	t.append_text(" " + _per_rank(cds, now, "s", 1) if ranked else " %s s" % _num(cds[0], 1))
	if info.get("haste", 0.0) > 0.0:
		t.append_text(" [color=#%s](%d ability haste)[/color]" % [C_DIM, roundi(info.haste)])
	var facts := []
	if info.get("range", 0.0) > 0.0:
		facts.append("Range %d" % roundi(info.range))
	if info.has("radius") and info.kind in ["area", "nova"]:
		facts.append("Radius %d" % roundi(info.radius))
	elif info.has("radius"):
		facts.append("Width %d" % roundi(info.radius * 2.0))
	if info.get("speed", 0.0) > 0.0:
		facts.append("Speed %d" % roundi(info.speed))
	if info.get("windup", 0.0) > 0.0:
		facts.append("Cast %s s" % _num(info.windup, 2))
	if not facts.is_empty():
		t.append_text("\n[color=#%s]%s[/color]" % [C_DIM, "   ·   ".join(facts)])

	# What it does.
	t.append_text("\n\n")
	_describe(info, now)

	# Every rank, and what the next one changes.
	if ranked and max_rank > 1:
		var rows := []
		if info.has("damage"):
			rows.append(["Damage", _per_rank(info.damage.base, now, "", 0, _damage_color(info.damage.kind))])
		if info.has("heal"):
			rows.append(["Heal", _per_rank(info.heal.base, now, "", 0, C_HEAL)])
		if info.has("shield") and info.kind == "support":
			rows.append(["Shield", _per_rank(info.shield.base, now, "", 0, C_SHIELD)])
		rows.append(["Cooldown", _per_rank(cds, now, "s", 1)])
		t.append_text("\n")
		for r in rows:
			t.append_text("\n[color=#%s]%s:[/color] %s" % [C_DIM, r[0], r[1]])
		if rank < max_rank:
			var changes := []
			var nxt := rank  # the next rank's index (rank is 1-based; unlearned learns rank 1)
			var cur := rank - 1
			if rank == 0:
				changes.append("learn it")
			else:
				for pair in [["damage", "Damage"], ["heal", "Heal"], ["shield", "Shield"]]:
					if info.has(pair[0]) and (pair[0] != "shield" or info.kind == "support"):
						var base: PackedFloat32Array = info[pair[0]].base
						if not is_equal_approx(base[cur], base[nxt]):
							changes.append("%s %s → [color=#%s]%s[/color]" % [pair[1], _num(base[cur], 0), C_UP, _num(base[nxt], 0)])
				if not is_equal_approx(cds[cur], cds[nxt]):
					changes.append("Cooldown %s → [color=#%s]%s s[/color]" % [_num(cds[cur], 1), C_UP, _num(cds[nxt], 1)])
			if not changes.is_empty():
				t.append_text("\n\n[color=#%s]Next rank[/color] [color=#%s](%s)[/color][color=#%s]:[/color] %s" % [C_UP, C_DIM, level_key, C_UP, ",  ".join(changes)])
		else:
			t.append_text("\n\n[color=#%s]Max rank[/color]" % C_DIM)


## The sentence(s) saying what the ability does, numbers at the current rank.
func _describe(info: Dictionary, now: int) -> void:
	var t := _text
	match info.kind:
		"line":
			t.append_text("Fires a missile that stops at the first enemy it hits")
			if info.has("damage"):
				t.append_text(", dealing ")
				_damage(info.damage, now)
			_cc(info, "it", info.has("damage"))
			t.append_text(".")
		"area", "nova":
			var where := "around you" if info.kind == "nova" else "at the target point"
			t.append_text("Marks a circle %s that erupts after [color=#%s]%s s[/color], dealing " % [where, C_NUM, _num(info.delay, 2)])
			_damage(info.damage, now)
			t.append_text(" to every enemy inside")
			_cc(info, "them", true)
			t.append_text(".")
		"lunge":
			t.append_text("Dashes onto the enemy nearest the cursor and strikes it for ")
			_damage(info.damage, now)
			_cc(info, "it", true)
			t.append_text(". Needs an enemy in range.")
		"dash":
			t.append_text("Dashes toward the cursor, through units, sliding along walls.")
		"blink":
			t.append_text("Teleports toward the cursor, landing short of walls.")
		"barrier":
			t.append_text("Shields you for ")
			t.add_image(icon("shield"), 17, 17)
			t.append_text(" [color=#%s]%s[/color] damage for %s s." % [C_SHIELD, _num(info.shield.total, 0), _num(info.shield.seconds, 1)])
		"support":
			var who := "yourself" if info.range <= 0.0 else "the ally nearest the cursor (or yourself)"
			var parts := 0
			if info.has("heal"):
				var h: Dictionary = info.heal
				t.append_text("Heals %s for " % who)
				t.add_image(icon("heal"), 17, 17)
				t.append_text(" [color=#%s]%s[/color]" % [C_HEAL, _num(h.base[now], 0)])
				if h.ap > 0.0:
					t.append_text(" [color=#%s](+%d%% AP)[/color]" % [C_AP, roundi(h.ap * 100.0)])
					t.append_text(" [color=#%s](%s now)[/color]" % [C_DIM, _num(h.total, 0)])
				if h.get("missing_pct", 0.0) > 0.0:
					t.append_text(" plus [color=#%s]%d%% of missing health[/color]" % [C_HEAL, roundi(h.missing_pct)])
				parts += 1
			if info.has("shield"):
				var s: Dictionary = info.shield
				t.append_text(" and shields them for " if parts > 0 else "Shields %s for " % who)
				t.add_image(icon("shield"), 17, 17)
				t.append_text(" [color=#%s]%s[/color]" % [C_SHIELD, _num(s.base[now], 0)])
				if s.ap > 0.0:
					t.append_text(" [color=#%s](+%d%% AP)[/color]" % [C_AP, roundi(s.ap * 100.0)])
					t.append_text(" [color=#%s](%s now)[/color]" % [C_DIM, _num(s.total, 0)])
				t.append_text(" for %s s" % _num(s.seconds, 1))
			t.append_text(".")


## "80 (+60% AP) magic damage", with an icon, in the damage type's color.
func _damage(d: Dictionary, now: int) -> void:
	var t := _text
	var c := _damage_color(d.kind)
	t.add_image(icon(d.kind), 17, 17)
	t.append_text(" [color=#%s]%s[/color]" % [c, _num(d.base[now], 0)])
	if d.ad > 0.0:
		t.append_text(" [color=#%s](+%d%% AD)[/color]" % [C_AD, roundi(d.ad * 100.0)])
	if d.ap > 0.0:
		t.append_text(" [color=#%s](+%d%% AP)[/color]" % [C_AP, roundi(d.ap * 100.0)])
	t.append_text(" [color=#%s]%s damage[/color]" % [c, d.kind])
	if d.ad > 0.0 or d.ap > 0.0:
		t.append_text(" [color=#%s](%s now)[/color]" % [C_DIM, _num(d.total, 0)])


## ", stunning it for 1.2 s" (with the status icon), or nothing.
func _cc(info: Dictionary, obj: String, after_damage: bool) -> void:
	if not info.has("cc"):
		return
	var cc: Dictionary = info.cc
	var t := _text
	t.append_text(" and " if after_damage else ", ")
	t.add_image(icon(cc.kind), 17, 17)
	var secs := _num(cc.seconds, 2)
	match cc.kind:
		"stun":
			t.append_text(" [color=#%s]stunning[/color] %s for %s s" % [C_CC, obj, secs])
		"root":
			t.append_text(" [color=#%s]rooting[/color] %s for %s s" % [C_CC, obj, secs])
		"knockup":
			t.append_text(" [color=#%s]knocking %s up[/color] for %s s" % [C_CC, obj, secs])
		"pull":
			t.append_text(" [color=#%s]pulling %s[/color] to you" % [C_CC, obj])
		"slow":
			t.append_text(" [color=#%s]slowing[/color] %s by [color=#%s]%d%%[/color] for %s s" % [C_SLOW, obj, C_SLOW, cc.pct, secs])


## "80 / 120 / [b]160[/b] / 200 / 240 s", the current rank bright, the rest dim.
func _per_rank(values: PackedFloat32Array, now: int, unit: String, decimals: int, color: String = C_NUM) -> String:
	var parts := []
	for i in values.size():
		var v := _num(values[i], decimals)
		parts.append("[b][color=#%s]%s[/color][/b]" % [color, v] if i == now else "[color=#%s]%s[/color]" % [C_DIM, v])
	var out := "[color=#%s] / [/color]" % C_DIM
	return out.join(parts) + ((" " + unit) if unit != "" else "")


static func _damage_color(kind: String) -> String:
	match kind:
		"physical":
			return C_PHYSICAL
		"true":
			return C_TRUE
	return C_MAGIC


## A number with up to `decimals` places, trailing zeros dropped.
static func _num(v: float, decimals: int) -> String:
	if decimals == 0:
		return str(roundi(v))
	if is_equal_approx(v, roundf(v)):
		return str(roundi(v))
	return String.num(v, decimals)
