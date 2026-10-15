extends RefCounted
## Seasons (15 §9): everything about the map's look that changes with the season, as data. The
## light (sun, sky fill, grade, haze), the ground's palette (its shader's uniforms), the cliffs',
## the scenery's tint and substitutions (a prop id swapped for its seasonal variant when that
## exists), the snow on the props' upward facets, the life that appears and the ambient
## particles. The match, the map editor's Actual
## view and the minimap all apply the same season, so what a map maker sees is what's played.

const NAMES := ["summer", "autumn", "winter"]

const SUMMER := {
	"sun_color": Color(1.0, 0.9, 0.76),
	"sun_energy": 1.5,
	"sky_fill": Color(0.44, 0.54, 0.76),
	"sky_fill_energy": 0.6,
	"exposure": 1.08,
	"contrast": 1.12,
	"saturation": 1.02,
	"haze": Color(0.55, 0.63, 0.75),
	"ground": {
		"grass_dark": Color(0.14, 0.25, 0.12), "grass_mid": Color(0.20, 0.32, 0.14), "grass_light": Color(0.30, 0.41, 0.17),
		"grass_gloom": Color(0.10, 0.19, 0.15), "dirt": Color(0.42, 0.32, 0.22), "dirt_dark": Color(0.31, 0.23, 0.16),
		"paving": Color(0.42, 0.45, 0.47), "paving_dark": Color(0.27, 0.30, 0.33),
		"water_shallow": Color(0.24, 0.45, 0.42), "water_deep": Color(0.10, 0.27, 0.33), "foam": Color(0.62, 0.74, 0.70),
		"mud": Color(0.24, 0.21, 0.15),
		"flower_a": Color(0.40, 0.24, 0.55), "flower_b": Color(0.30, 0.42, 0.68), "flower_c": Color(0.70, 0.62, 0.22),
	},
	"cliff": {
		"rock": Color(0.31, 0.32, 0.35), "rock_dark": Color(0.17, 0.18, 0.21), "moss": Color(0.24, 0.34, 0.18),
	},
	"tint": Color(1.0, 1.0, 1.0),
	"substitute": {},
	"life": ["flowers", "butterflies", "lily_pads", "frogs", "fireflies"],
	"motes": "pollen",
}

const AUTUMN := {
	"sun_color": Color(1.0, 0.82, 0.62),
	"sun_energy": 1.4,
	"sky_fill": Color(0.5, 0.52, 0.66),
	"sky_fill_energy": 0.5,
	"exposure": 1.05,
	"contrast": 1.12,
	"saturation": 1.0,
	"haze": Color(0.62, 0.58, 0.58),
	"ground": {
		"grass_dark": Color(0.20, 0.23, 0.11), "grass_mid": Color(0.28, 0.28, 0.13), "grass_light": Color(0.42, 0.33, 0.15),
		"grass_gloom": Color(0.15, 0.16, 0.12), "dirt": Color(0.40, 0.29, 0.20), "dirt_dark": Color(0.29, 0.21, 0.15),
		"paving": Color(0.43, 0.43, 0.43), "paving_dark": Color(0.28, 0.28, 0.30),
		"water_shallow": Color(0.26, 0.40, 0.38), "water_deep": Color(0.12, 0.24, 0.28), "foam": Color(0.62, 0.68, 0.64),
		"mud": Color(0.25, 0.20, 0.14),
		"flower_a": Color(0.62, 0.22, 0.10), "flower_b": Color(0.72, 0.42, 0.10), "flower_c": Color(0.55, 0.16, 0.12),
	},
	"cliff": {
		"rock": Color(0.29, 0.28, 0.28), "rock_dark": Color(0.16, 0.15, 0.16), "moss": Color(0.32, 0.30, 0.15),
	},
	"tint": Color(1.08, 0.94, 0.78),
	"substitute": {},
	"life": ["lily_pads", "fireflies"],
	"motes": "leaves",
}

const WINTER := {
	"sun_color": Color(0.92, 0.94, 1.0),
	"sun_energy": 1.35,
	"sky_fill": Color(0.5, 0.58, 0.76),
	"sky_fill_energy": 0.6,
	"exposure": 1.0,
	"contrast": 1.08,
	"saturation": 0.95,
	"haze": Color(0.7, 0.75, 0.84),
	"ground": {
		"grass_dark": Color(0.62, 0.66, 0.72), "grass_mid": Color(0.72, 0.76, 0.82), "grass_light": Color(0.82, 0.85, 0.9),
		"grass_gloom": Color(0.48, 0.53, 0.62), "dirt": Color(0.40, 0.36, 0.33), "dirt_dark": Color(0.30, 0.27, 0.26),
		"paving": Color(0.50, 0.52, 0.56), "paving_dark": Color(0.33, 0.35, 0.40),
		"water_shallow": Color(0.62, 0.74, 0.82), "water_deep": Color(0.38, 0.52, 0.64), "foam": Color(0.9, 0.93, 0.96),
		"mud": Color(0.34, 0.33, 0.33),
		"flower_a": Color(0.75, 0.8, 0.88), "flower_b": Color(0.7, 0.76, 0.86), "flower_c": Color(0.8, 0.84, 0.9),
	},
	"cliff": {
		"rock": Color(0.28, 0.30, 0.35), "rock_dark": Color(0.15, 0.16, 0.21), "moss": Color(0.74, 0.78, 0.84),
	},
	"tint": Color(0.86, 0.92, 1.04),
	# Snow-laden pines, bare broadleaves among them, dry grass, bare shrubs in drifts.
	"substitute": {
		"pine_1": "pine_1_snow", "pine_2": "pine_2_snow", "pine_3": "tree_bare_1",
		"grass_1": "grass_winter", "bush_1": "bush_winter",
	},
	"snow": 1.0,
	"life": [],
	"motes": "snow",
}


static func get_season(name: String) -> Dictionary:
	match name:
		"autumn":
			return AUTUMN
		"winter":
			return WINTER
	return SUMMER


## The light: the sun, the sky fill, the grade and the haze.
static func apply_light(s: Dictionary, e: Environment, sun: DirectionalLight3D) -> void:
	e.ambient_light_color = s.sky_fill
	e.ambient_light_energy = s.sky_fill_energy
	e.tonemap_exposure = s.exposure
	e.adjustment_contrast = s.contrast
	e.adjustment_saturation = s.saturation
	e.fog_light_color = s.haze
	if sun != null:
		sun.light_color = s.sun_color
		sun.light_energy = s.sun_energy


## A material's uniforms from one of the season's palettes ("ground", "cliff").
static func apply_palette(s: Dictionary, part: String, m: ShaderMaterial) -> void:
	var pal: Dictionary = s.get(part, {})
	for k in pal:
		m.set_shader_parameter(k, pal[k])
