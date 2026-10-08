// Champion models (10 §2, "faceted PS1"): flat shading from screen-space derivatives, vertex
// colors as albedo (alpha = baked occlusion), a three-band toon ramp so neighbouring facets
// step visibly, a soft rim, the team accent on the `accent` slot (05 §1.3), and the impact
// flash (10 §4.4). One material per surface; `slot` says which. No textures.
shader_type spatial;
render_mode specular_disabled;
#include "res://shaders/clouds.gdshaderinc"

// 0 skin, 1 cloth, 2 metal, 3 emissive, 4 accent, 5 accent_glow (the team color, glowing:
// minions' eyes and orbs) (`MftrModel.surface_slots`).
uniform int slot = 1;
uniform vec3 team_accent : source_color = vec3(0.25, 0.55, 1.0);
// Until a champion ships its own model, the shared template wears its identity color.
uniform vec3 identity : source_color = vec3(1.0);
uniform float identity_mix = 0.0;
// 1 on the frame a hit lands, fading to 0 within ~3 frames.
uniform float flash = 0.0;
// Vertex colors are authored for a neutral studio light; the arena's sun and ambient are
// brighter, so albedo is scaled down to keep colors in range.
uniform float exposure = 0.72;

void fragment() {
	cloud_sun = cloud_shade((INV_VIEW_MATRIX * vec4(VERTEX, 1.0)).xyz);
	// Faceted: one normal per triangle, from the derivatives of the view-space position.
	NORMAL = normalize(cross(dFdy(VERTEX), dFdx(VERTEX)));
	vec3 c = COLOR.rgb;
	if (slot == 4) {
		c = team_accent * (0.75 + 0.25 * dot(COLOR.rgb, vec3(0.3333)));
	} else if (slot == 5) {
		c = mix(team_accent, vec3(1.0), 0.35);
	} else if (slot == 1) {
		c = mix(c, identity * (0.55 + dot(COLOR.rgb, vec3(0.3333))), identity_mix);
	}
	ALBEDO = c * mix(0.6, 1.0, COLOR.a) * exposure;
	ROUGHNESS = 1.0;
	RIM = 0.25;
	RIM_TINT = 0.5;
	EMISSION = (slot == 3 ? c * 0.9 : (slot == 5 ? c * 1.4 : vec3(0.0))) + vec3(flash);
}

void light() {
	// Three bands: lit, half-lit, shadowed. Faces a few degrees apart land in different bands.
	float ndl = clamp(dot(NORMAL, LIGHT), 0.0, 1.0);
	float band = ndl > 0.7 ? 1.0 : (ndl > 0.35 ? 0.68 : 0.38);
	DIFFUSE_LIGHT += band * ATTENUATION * LIGHT_COLOR / PI * (LIGHT_IS_DIRECTIONAL ? cloud_sun : 1.0);
}
