// Map props (structures, trees, rocks; `art/props`): the champions' look (10 §2, "faceted PS1")
// so the map and its characters belong together: flat facets from screen-space derivatives,
// vertex colors as albedo (alpha = baked occlusion), the three-band toon ramp, the team color on
// `accent` and `accent_glow`. A fine world-space grain stands in for texture (05 §3), and the
// fog of war darkens what our team doesn't see.
shader_type spatial;
render_mode specular_disabled;
#include "res://shaders/clouds.gdshaderinc"

// 0 skin, 1 cloth, 2 metal, 3 emissive, 4 accent, 5 accent_glow (`MftrModel.surface_slots`).
uniform int slot = 1;
uniform vec3 team_accent : source_color = vec3(0.25, 0.55, 1.0);
uniform float exposure = 0.8;
// Scenery drawn in batches gets a per-instance tint (MultiMesh color): a little variety.
uniform bool instance_tint = false;
// Grass and trees move a little in the wind (meters of sway per meter of height).
uniform float sway = 0.0;
// 1 while a structure is protected (an earlier one in its lane stands): a faint cold sheen.
uniform float protected_glow = 0.0;

global uniform sampler2D fog_map : filter_linear;
global uniform vec2 fog_size;
global uniform float fog_on;

varying vec3 world_pos;
varying vec3 tint;

float hash(vec3 p) {
	p = fract(p * vec3(0.1031, 0.1030, 0.0973));
	p += dot(p, p.yxz + 33.33);
	return fract((p.x + p.y) * p.z);
}

vec3 fogged(vec3 c, vec3 wp) {
	if (fog_on < 0.5 || fog_size.x <= 0.0) {
		return c;
	}
	vec2 uv = wp.xz / fog_size;
	// Past the map (scenery), the nearest edge of the map decides.
	float seen = texture(fog_map, clamp(uv, vec2(0.002), vec2(0.998))).r;
	vec3 dark = mix(vec3(dot(c, vec3(0.3, 0.55, 0.15))), c, 0.45) * vec3(0.42, 0.46, 0.58);
	return mix(dark, c, seen);
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	if (sway > 0.0) {
		float h = max(VERTEX.y, 0.0);
		VERTEX.x += sin(TIME * 1.3 + world_pos.x * 0.6 + world_pos.z * 0.4) * sway * h;
		VERTEX.z += cos(TIME * 1.1 + world_pos.x * 0.5) * sway * 0.6 * h;
	}
	tint = instance_tint ? INSTANCE_CUSTOM.rgb * 2.0 : vec3(1.0);
}

void fragment() {
	cloud_sun = cloud_shade(world_pos);
	NORMAL = normalize(cross(dFdy(VERTEX), dFdx(VERTEX)));
	vec3 c = COLOR.rgb * tint;
	if (slot == 4) {
		c = team_accent * (0.5 + 0.5 * dot(COLOR.rgb, vec3(0.3333)));
	} else if (slot == 5) {
		c = mix(team_accent, vec3(1.0), 0.2);
	}
	if (slot < 3) {
		// Grain: chunky world-space speckle (≈ 6 cm cells), like a low-resolution texture.
		float g = hash(floor(world_pos * 16.0));
		c *= 0.9 + 0.16 * g;
	}
	vec3 lit = c * mix(0.58, 1.0, COLOR.a) * exposure;
	ALBEDO = fogged(lit, world_pos) + vec3(0.04, 0.06, 0.1) * protected_glow;
	ROUGHNESS = 1.0;
	RIM = 0.2;
	RIM_TINT = 0.5;
	vec3 glow = slot == 3 ? c * 0.9 : (slot == 5 ? c * 1.25 : vec3(0.0));
	EMISSION = fogged(glow, world_pos);
}

void light() {
	float ndl = clamp(dot(NORMAL, LIGHT), 0.0, 1.0);
	float band = ndl > 0.7 ? 1.0 : (ndl > 0.35 ? 0.68 : 0.38);
	DIFFUSE_LIGHT += band * ATTENUATION * LIGHT_COLOR / PI * (LIGHT_IS_DIRECTIONAL ? cloud_sun : 1.0);
}
