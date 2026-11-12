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

void vertex() {
	region = int(round(UV2.x * 256.0)) - 1;
}

void fragment() {
	cloud_sun = cloud_shade((INV_VIEW_MATRIX * vec4(VERTEX, 1.0)).xyz);
	// Faceted: one normal per triangle, from the derivatives of the view-space position.
	NORMAL = normalize(cross(dFdy(VERTEX), dFdx(VERTEX)));
	vec3 c = COLOR.rgb;
	if (has_paint && slot <= 2) {
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

void light() {
	// Three bands: lit, half-lit, shadowed. Faces a few degrees apart land in different bands.
	float ndl = clamp(dot(NORMAL, LIGHT), 0.0, 1.0);
	float band = ndl > 0.7 ? 1.0 : (ndl > 0.35 ? 0.68 : 0.38);
	if (slot == 0) {
		// Skin: the same range, continuous. A sculpted face's facets are small and a few
		// degrees apart, and hard bands scattered them into a mosaic; this keeps the facets
		// and lets the light follow the brow, cheekbones and jaw.
		band = 0.38 + 0.62 * smoothstep(0.08, 0.85, ndl);
	}
	DIFFUSE_LIGHT += band * ATTENUATION * LIGHT_COLOR / PI * (LIGHT_IS_DIRECTIONAL ? cloud_sun : 1.0);
}
