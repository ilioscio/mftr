// Champion models (10 §2, "faceted PS1"): flat shading from screen-space derivatives, vertex
// colors as albedo (alpha = baked occlusion), a three-band toon ramp so neighbouring facets
// step visibly, a soft rim, the team accent on the `accent` slot (05 §1.3), and the impact
// flash (10 §4.4). One material per surface; `slot` says which. No textures, but for the
// champion creator's paint images (D70, CR5b): palette colors over the vertex colors, nearest.
shader_type spatial;
render_mode specular_disabled;
#include "res://shaders/clouds.gdshaderinc"

// 0 skin, 1 cloth, 2 metal, 3 emissive, 4 accent, 5 accent_glow (the team color, glowing:
// minions' eyes and orbs) (`MftrModel.surface_slots`).
uniform int slot = 1;
uniform vec3 team_accent : source_color = vec3(0.25, 0.55, 1.0);
// How white-hot `accent_glow` burns: champions' and minions' glows have a pale core; monsters'
// glow in their camp's color (16 §2).
uniform float glow_white = 0.35;
// Until a champion ships its own model, the shared template wears its identity color.
uniform vec3 identity : source_color = vec3(1.0);
uniform float identity_mix = 0.0;
// A forged champion's brush strokes and decals: RGBA, alpha 0 where unpainted. Palette values
// are stored as they are in vertex colors, so no sRGB hint.
uniform sampler2D paint : filter_nearest, repeat_disable;
uniform bool has_paint = false;
// 1 on the frame a hit lands, fading to 0 within ~3 frames.
uniform float flash = 0.0;
// Vertex colors are authored for a neutral studio light; the arena's sun and ambient are
// brighter, so albedo is scaled down to keep colors in range.
uniform float exposure = 0.72;
// The champion creator's face editor (21 §3.2): each preview vertex carries its face region in
// UV2.x (k / 256, 0 = none). The region under the cursor brightens; the one being dragged
// glows in `highlight`. Changing these is a uniform write, never a buffer. -1: off (the game).
uniform int hover_region = -1;
uniform int active_region = -1;
// The other side's region, which follows with symmetry on: lit a little less.
uniform int mirror_region = -1;
uniform vec3 highlight : source_color = vec3(1.0, 0.85, 0.55);
// Flat: a triangle takes one vertex's region whole (interpolating would light every region
// whose code lies between two neighbours').
varying flat int region;
// An eyeball's vertex (22 §2.1): its UV is (2 + 2·style + u, 2·code + v), (u, v) its place on
// the ball seen from the front, `style` 0 round, 1 slit, 2 glowing, 3 blank; `code` the iris's
// size (code % 10: 80% to 125%) and the sclera (code / 10: white, yellowed, dark). Its color is
// the iris, its alpha the pupil's size. -1: not an eye.
varying flat int eye_style;
varying flat int eye_code;
// A brow strip's vertex (22 §3.1): its UV is (10 + along the brow, 2 + across it), each 0-1.
varying flat int brow;
// A lip's vertex (22 §3.2): (12 + across the mouth, 3 + up), up -1 (the lower lip's border)
// through 0 (where they meet) to 1 (the upper's). Shaded smooth.
varying flat int lip;
// A hair vertex (22 §7 item 5, `mftr_blender.grooming`): (14 + along the lock, 0 at the root to 1
// at the tip, 4 + across it, in strands' units: 12 strands to one).
varying flat int hair;

void vertex() {
	region = int(round(UV2.x * 256.0)) - 1;
	bool eye = UV.x >= 1.99 && UV.x < 9.99;
	eye_style = eye ? int(floor((UV.x - 2.0) * 0.5)) : -1;
	eye_code = eye ? int(floor(UV.y * 0.5)) : 0;
	brow = UV.x >= 9.99 && UV.x < 11.99 ? 1 : 0;
	lip = UV.x >= 11.99 && UV.x < 13.99 ? 1 : 0;
	hair = UV.x >= 13.99 && UV.x < 15.99 ? 1 : 0;
}

float eye_hash(float a) {
	return fract(sin(a * 12.9898) * 43758.5453);
}

// The eye's color at a point of the ball: sclera, limbal ring, iris with radial streaks, pupil
// and a catchlight. `e` is the place on the ball seen from the front, -1 to 1.
vec3 eye_color(vec2 e, vec3 iris, float pupil, int style, int code, out float glow) {
	float d = length(e);
	float size = 0.8 + 0.05 * float(code % 10);
	float R = 0.41 * size;
	int sc = code / 10;
	vec3 sclera = sc == 1 ? vec3(0.86, 0.80, 0.60) : (sc == 2 ? vec3(0.05, 0.05, 0.06) : vec3(0.93, 0.91, 0.87));
	// The sclera darkens toward the corners, where the lids shade it.
	sclera *= mix(1.0, 0.72, smoothstep(0.55, 0.95, d));
	float a = atan(e.y, e.x);
	float streak = 0.5 + 0.5 * sin(a * 23.0 + 2.0 * sin(a * 7.0)) * (0.6 + 0.4 * eye_hash(floor(a * 9.0)));
	vec3 ir = iris * mix(0.62, 1.18, streak);
	// Lighter round the pupil, a dark limbal ring at the edge.
	ir *= mix(1.25, 1.0, smoothstep(0.0, 0.55, d / R));
	ir *= mix(1.0, 0.35, smoothstep(0.78, 1.0, d / R));
	float P = R * mix(0.22, 0.62, pupil);
	float in_pupil = style == 1
		? 1.0 - smoothstep(0.85, 1.0, length(vec2(e.x * 3.2, e.y)) / (R * 0.95))
		: 1.0 - smoothstep(0.9, 1.0, d / P);
	float in_iris = 1.0 - smoothstep(0.94, 1.0, d / R);
	glow = 0.0;
	vec3 c;
	if (style == 3) {
		// Blank: the whole eye one color, no pupil.
		c = mix(iris * 0.8, iris * 1.05, 1.0 - d);
	} else {
		c = mix(sclera, ir, in_iris);
		c = mix(c, vec3(0.015), in_pupil * in_iris);
		if (style == 2) {
			glow = in_iris * (1.0 - in_pupil * 0.6);
		}
	}
	// The catchlight: a small window of the sky, up and to one side.
	float spot = 1.0 - smoothstep(0.05, 0.085, length(e - vec2(-0.16, 0.2) * size));
	c = mix(c, vec3(1.0), spot * 0.85);
	glow += spot * 0.6;
	return c;
}

void fragment() {
	cloud_sun = cloud_shade((INV_VIEW_MATRIX * vec4(VERTEX, 1.0)).xyz);
	// Faceted: one normal per triangle, from the derivatives of the view-space position.
	if (eye_style < 0 && lip == 0) {
		NORMAL = normalize(cross(dFdy(VERTEX), dFdx(VERTEX)));
	}
	if (eye_style >= 0) {
		vec2 e = vec2(UV.x - 2.0 - 2.0 * float(eye_style), UV.y - 2.0 * float(eye_code)) * 2.0 - 1.0;
		float glow;
		vec3 ec = eye_color(e, COLOR.rgb, COLOR.a, eye_style, eye_code, glow);
		ALBEDO = ec * exposure;
		ROUGHNESS = 0.4;
		RIM = 0.0;
		EMISSION = ec * glow * (eye_style == 2 ? 1.6 : 0.35) + vec3(flash);
		if (region >= 0 && region == active_region) {
			EMISSION += highlight * 0.32;
		} else if (region >= 0 && (region == hover_region || region == mirror_region)) {
			EMISSION += highlight * 0.13;
		}
	} else {
	vec3 c = COLOR.rgb;
	if (brow == 1) {
		// Hairs: fine strokes slanting up from the brow's start and lying flatter toward its tip,
		// darker at the roots along its lower edge, the ends lighter.
		float u = UV.x - 10.0;
		float v = clamp(UV.y - 2.0, 0.0, 1.0);
		float lean = mix(1.15, 0.3, u);
		float along = u * 30.0 * cos(lean) + v * 3.2 * sin(lean) * (1.0 - 0.5 * u);
		float stroke = 0.5 + 0.5 * sin((along + eye_hash(floor(along)) * 0.6) * 6.2832);
		c *= mix(0.78, 1.18, stroke) * mix(0.82, 1.0, smoothstep(0.0, 0.45, v)) * mix(1.0, 1.12, smoothstep(0.6, 1.0, u));
	} else if (lip == 1) {
		// Darker where the lips meet, fullest in each lip's middle, a soft sheen on the lower.
		float up = clamp(UV.y - 3.0, -1.0, 1.0);
		float across = abs(UV.x - 12.5) * 2.0;
		float a = abs(up);
		c *= mix(0.55, 1.0, smoothstep(0.0, 0.35, a)) * mix(1.0, 0.9, smoothstep(0.75, 1.0, a));
		c *= mix(1.0, 0.85, smoothstep(0.6, 1.0, across));
		if (up < 0.0) {
			c *= 1.0 + 0.14 * (1.0 - smoothstep(0.0, 0.5, abs(a - 0.5))) * (1.0 - smoothstep(0.0, 0.6, across));
		}
	} else if (hair == 1) {
		// Strands along the lock, wavering a little, each a shade of its own; darker at the
		// roots, lighter toward the tips.
		float along = clamp(UV.x - 14.0, 0.0, 1.0);
		float across = UV.y - 4.0;
		float s = across * 12.0 + 0.3 * sin(along * 7.0 + across * 5.0);
		float stroke = smoothstep(0.15, 0.85, 0.5 + 0.5 * sin(s * 6.2832));
		float shade = eye_hash(floor(s + 0.5));
		c *= mix(0.86, 1.08, stroke) * mix(0.84, 1.1, shade);
		c *= mix(0.78, 1.0, smoothstep(0.0, 0.3, along)) * mix(1.0, 1.12, smoothstep(0.6, 1.0, along));
	} else if (has_paint && slot <= 2) {
		vec4 p = texture(paint, UV);
		c = mix(c, p.rgb, p.a);
	}
	if (slot == 4) {
		c = team_accent * (0.75 + 0.25 * dot(COLOR.rgb, vec3(0.3333)));
	} else if (slot == 5) {
		c = mix(team_accent, vec3(1.0), glow_white);
	} else if (slot == 1) {
		c = mix(c, identity * (0.55 + dot(COLOR.rgb, vec3(0.3333))), identity_mix);
	}
	ALBEDO = c * mix(0.6, 1.0, COLOR.a) * exposure;
	ROUGHNESS = 1.0;
	RIM = 0.25;
	RIM_TINT = 0.5;
	EMISSION = (slot == 3 ? c * 0.9 : (slot == 5 ? c * 1.4 : vec3(0.0))) + vec3(flash);
	if (region >= 0 && region == active_region) {
		EMISSION += highlight * 0.32;
	} else if (region >= 0 && region == mirror_region && active_region >= 0) {
		EMISSION += highlight * 0.2;
	} else if (region >= 0 && (region == hover_region || region == mirror_region)) {
		EMISSION += highlight * 0.13;
		RIM = 0.7;
	}
	}
}

void light() {
	// Three bands: lit, half-lit, shadowed. Faces a few degrees apart land in different bands.
	float ndl = clamp(dot(NORMAL, LIGHT), 0.0, 1.0);
	float band = ndl > 0.7 ? 1.0 : (ndl > 0.35 ? 0.68 : 0.38);
	if (eye_style >= 0) {
		// An eye: soft and bright, never banded (a band across an iris reads as a lid).
		band = 0.7 + 0.3 * ndl;
	} else if (slot == 0) {
		// Skin: the same range, continuous. A sculpted face's facets are small and a few
		// degrees apart, and hard bands scattered them into a mosaic; this keeps the facets
		// and lets the light follow the brow, cheekbones and jaw.
		band = 0.38 + 0.62 * smoothstep(0.08, 0.85, ndl);
	}
	DIFFUSE_LIGHT += band * ATTENUATION * LIGHT_COLOR / PI * (LIGHT_IS_DIRECTIONAL ? cloud_sun : 1.0);
}
