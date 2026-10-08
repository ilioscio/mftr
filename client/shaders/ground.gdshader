// Procedural ground: no textures, only noise (D4). Stylized grass/dirt with a lane band,
// low contrast so gameplay elements read on top of it (05 §1 value hierarchy).
shader_type spatial;
render_mode diffuse_lambert, specular_disabled;

uniform vec3 grass_a : source_color = vec3(0.20, 0.33, 0.17);
uniform vec3 grass_b : source_color = vec3(0.27, 0.40, 0.19);
uniform vec3 dirt : source_color = vec3(0.42, 0.36, 0.27);
uniform float lane_width = 9.0;
// 0: a diagonal lane (the M1 arena); 1: a horizontal lane at z = lane_z (The Bridge).
uniform float lane_mode = 0.0;
uniform float lane_z = 15.0;

varying vec3 world_pos;

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

float value_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
	           mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	for (int i = 0; i < 5; i++) {
		v += a * value_noise(p);
		p = p * 2.03 + vec2(17.0, 9.0);
		a *= 0.5;
	}
	return v;
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec2 p = world_pos.xz;
	float large = fbm(p * 0.06);
	float small = fbm(p * 0.9);
	vec3 grass = mix(grass_a, grass_b, smoothstep(0.35, 0.65, large));
	grass *= 0.9 + 0.2 * small;
	// A diagonal "lane" of dirt with a noisy edge.
	float d = lane_mode > 0.5 ? abs(p.y - lane_z) : abs(p.x - p.y) / sqrt(2.0);
	float edge = lane_width * 0.5 + (fbm(p * 0.25) - 0.5) * 3.0;
	float lane = 1.0 - smoothstep(edge - 0.6, edge + 0.6, d);
	vec3 ground = mix(grass, dirt * (0.92 + 0.16 * small), lane);
	// Subtle 1 m grid to help judge distances in the sandbox.
	vec2 g = abs(fract(p) - 0.5);
	float grid = 1.0 - smoothstep(0.47, 0.5, max(g.x, g.y));
	ALBEDO = fogged(ground * (1.0 - 0.06 * (1.0 - grid)), world_pos);
	ROUGHNESS = 1.0;
}
