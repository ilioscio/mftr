extends RefCounted
## What MFTR stands for today: the line under the logo, rolled from `res://splashes.txt` each time
## the client starts. The file ships inside the game's pack (the export presets include it), so a
## release reads only what it was built with; it's shown as plain text, never markup. The format is
## described at the top of the file. The last half or so of the lines shown are remembered in
## `user://splashes.cfg`, so a line doesn't come back until most of the others have.

const PATH := "res://splashes.txt"
const SEEN_PATH := "user://splashes.cfg"
const FALLBACK := "Moba For The Rest of us"
const MAX_BYTES := 256 * 1024
const MAX_LENGTH := 60


## The file's lines: {"everyday": [line], "holidays": [[from, to, dated, line]], "errors": [text]}.
## A holiday's from and to are MMDD, or YYYYMMDD when `dated`. Lines that don't parse are left out
## and described in "errors".
static func parse(text: String) -> Dictionary:
	var everyday: Array[String] = []
	var holidays := []
	var errors: Array[String] = []
	var n := 0
	for raw in text.split("\n"):
		n += 1
		var line := raw.strip_edges()
		if line == "" or line.begins_with("#"):
			continue
		var when := []
		if line.begins_with("["):
			var close := line.find("]")
			when = _parse_when(line.substr(1, close - 1)) if close > 0 else []
			if when.is_empty():
				errors.append("line %d: bad date in %s" % [n, line])
				continue
			line = line.substr(close + 1).strip_edges()
		if line == "" or line.length() > MAX_LENGTH:
			errors.append("line %d: %s" % [n, "empty" if line == "" else "longer than %d characters" % MAX_LENGTH])
			continue
		if when.is_empty():
			everyday.append(line)
		else:
			holidays.append(when + [line])
	return {"everyday": everyday, "holidays": holidays, "errors": errors}


## `MM-DD`, `YYYY-MM-DD`, or two of the same kind joined by `..`, as [from, to, dated]; [] if neither.
static func _parse_when(tag: String) -> Array:
	var ends := tag.split("..")
	if ends.size() > 2:
		return []
	var from := _parse_date(ends[0])
	var to := _parse_date(ends[-1])
	if from.is_empty() or to.is_empty() or from[1] != to[1]:
		return []
	return [from[0], to[0], from[1]]


## [MMDD or YYYYMMDD, dated] for `MM-DD` or `YYYY-MM-DD`; [] otherwise.
static func _parse_date(s: String) -> Array:
	var parts := s.strip_edges().split("-")
	if parts.size() < 2 or parts.size() > 3:
		return []
	for p in parts:
		if not p.is_valid_int():
			return []
	var month := parts[-2].to_int()
	var day := parts[-1].to_int()
	if month < 1 or month > 12 or day < 1 or day > 31:
		return []
	var md := month * 100 + day
	return [parts[0].to_int() * 10000 + md, true] if parts.size() == 3 else [md, false]


## The lines to roll from on a day (a `Time` date dict): its holidays' when it has any, else the
## everyday ones.
static func for_day(parsed: Dictionary, date: Dictionary) -> Array[String]:
	var md: int = date.month * 100 + date.day
	var ymd: int = date.year * 10000 + md
	var lines: Array[String] = []
	for h in parsed.holidays:
		var at := ymd if h[2] else md
		var inside: bool = (h[0] <= at and at <= h[1]) if h[0] <= h[1] else (at >= h[0] or at <= h[1])
		if inside:
			lines.append(h[3])
	return lines if not lines.is_empty() else parsed.everyday


## The capitals that start each word, ignoring leading punctuation: "MFTR" for a good line.
static func initials(line: String) -> String:
	var out := ""
	for word in line.split(" ", false):
		for c in word:
			if c.to_upper() != c.to_lower():
				if c == c.to_upper():
					out += c
				break
	return out


## Today's line, picked from those not shown recently, and remembered.
static func roll() -> String:
	var f := FileAccess.open(PATH, FileAccess.READ)
	if f == null:
		return FALLBACK
	var parsed := parse(f.get_buffer(MAX_BYTES).get_string_from_utf8())
	var lines := for_day(parsed, Time.get_date_dict_from_system())
	if lines.is_empty():
		return FALLBACK
	var seen := ConfigFile.new()
	seen.load(SEEN_PATH)
	var recent: Array = seen.get_value("splashes", "recent", [])
	var fresh := lines.filter(func(l): return l not in recent)
	if fresh.is_empty():
		fresh = lines.filter(func(l): return l != recent.back()) if lines.size() > 1 else lines
	var line: String = fresh.pick_random()
	recent.append(line)
	var keep := maxi(1, (parsed.everyday.size() + parsed.holidays.size()) / 2)
	seen.set_value("splashes", "recent", recent.slice(maxi(0, recent.size() - keep)))
	seen.save(SEEN_PATH)
	return line
