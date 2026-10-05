// Procedural ground: no textures, only noise (D4). Stylized grass/dirt with a lane band,
// low contrast so gameplay elements read on top of it (05 §1 value hierarchy).
shader_type spatial;
render_mode diffuse_lambert, specular_disabled;

uniform vec3 grass_a : source_color = vec3(0.20, 0.33, 0.17);
uniform vec3 grass_b : source_color = vec3(0.27, 0.40, 0.19);
uniform vec3 dirt : source_color = vec3(0.42, 0.36, 0.27);
uniform float lane_width = 9.0;

varying vec3 world_pos;

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
	float d = abs(p.x - p.y) / sqrt(2.0);
	float edge = lane_width * 0.5 + (fbm(p * 0.25) - 0.5) * 3.0;
	float lane = 1.0 - smoothstep(edge - 0.6, edge + 0.6, d);
	vec3 ground = mix(grass, dirt * (0.92 + 0.16 * small), lane);
	// Subtle 1 m grid to help judge distances in the sandbox.
	vec2 g = abs(fract(p) - 0.5);
	float grid = 1.0 - smoothstep(0.47, 0.5, max(g.x, g.y));
	ALBEDO = ground * (1.0 - 0.06 * (1.0 - grid));
	ROUGHNESS = 1.0;
}
