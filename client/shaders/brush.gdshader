// Brush (05 §3): tall-grass tufts from noise, swaying slowly, translucent enough to see that
// it's an area rather than a wall. Units inside are hidden by the server, not by this shader.
shader_type spatial;
render_mode diffuse_lambert, specular_disabled, cull_back, depth_draw_always, blend_mix;

uniform vec3 grass_low : source_color = vec3(0.10, 0.28, 0.12);
uniform vec3 grass_high : source_color = vec3(0.30, 0.55, 0.22);

varying vec3 world_pos;
varying vec3 world_normal;

// Fog of war (the minimap's vision grid): ground our team doesn't see is darkened and cooled.
global uniform sampler2D fog_map : filter_linear;
global uniform vec2 fog_size;
global uniform float fog_on;

vec3 fogged(vec3 c, vec3 wp) {
	if (fog_on < 0.5 || fog_size.x <= 0.0) {
		return c;
	}
	vec2 uv = wp.xz / fog_size;
	float seen = (uv.x < 0.0 || uv.y < 0.0 || uv.x > 1.0 || uv.y > 1.0) ? 0.0 : texture(fog_map, uv).r;
	vec3 dark = mix(vec3(dot(c, vec3(0.3, 0.55, 0.15))), c, 0.45) * vec3(0.42, 0.46, 0.58);
	return mix(dark, c, seen);
}


float hash(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	float sway = sin(TIME * 1.3 + (world_pos.x + world_pos.z) * 0.7) * 0.12;
	float h = clamp(world_pos.y / 0.55, 0.0, 1.0);
	float blades;
	if (abs(world_normal.y) > 0.6) {
		// Top: clumps of tufts.
		vec2 p = world_pos.xz + vec2(sway, 0.0);
		blades = noise(p * 6.0) * 0.6 + noise(p * 1.7) * 0.4;
	} else {
		// Sides: vertical blades, swaying more toward the tips.
		float across = world_pos.x + world_pos.z + sway * h;
		blades = noise(vec2(across * 14.0, world_pos.y * 1.5)) * 0.7 + noise(vec2(across * 3.0, 0.0)) * 0.3;
		blades *= 0.6 + 0.4 * h;
	}
	vec3 c = mix(grass_low, grass_high, clamp(blades * 0.8 + h * 0.5, 0.0, 1.0));
	ALBEDO = fogged(c, world_pos);
	ALPHA = 0.72 + 0.2 * blades;
	ROUGHNESS = 1.0;
}
