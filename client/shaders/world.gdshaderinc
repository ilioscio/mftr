// The world's shared look (15 §7): noise for the procedural materials, the fog of war, and one
// lighting model for the ground, the cliffs and the props, so they sit in the same light.
//
// Include clouds.gdshaderinc first.

global uniform sampler2D fog_map : filter_linear;
global uniform vec2 fog_size;
global uniform float fog_on;

// An integer hash (PCG 2D) of the point's 1/16 grid cell: exact at any world coordinate. (The
// classic fract-of-a-product hash lost precision tens of meters from the origin and drew the
// noise as big flat triangles.)
uvec2 w_pcg2d(uvec2 v) {
	v = v * 1664525u + 1013904223u;
	v.x += v.y * 1664525u;
	v.y += v.x * 1664525u;
	v = v ^ (v >> 16u);
	v.x += v.y * 1664525u;
	v.y += v.x * 1664525u;
	v = v ^ (v >> 16u);
	return v;
}

float w_hash(vec2 p) {
	uvec2 u = uvec2(ivec2(floor(p * 16.0 + 0.5)) + ivec2(1 << 20));
	return float(w_pcg2d(u).x >> 8u) / 16777215.0;
}

vec2 w_hash2(vec2 p) {
	return vec2(w_hash(p), w_hash(p + 19.19));
}

float w_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(w_hash(i), w_hash(i + vec2(1.0, 0.0)), u.x),
	           mix(w_hash(i + vec2(0.0, 1.0)), w_hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float w_fbm(vec2 p) {
	// Each octave turned (~37°) so the value noise's grid never lines up into squares.
	const mat2 turn = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8));
	float v = 0.0;
	float a = 0.5;
	p = turn * p;
	for (int i = 0; i < 4; i++) {
		v += a * w_noise(p);
		p = turn * p * 2.03 + vec2(17.0, 9.0);
		a *= 0.5;
	}
	return v;
}

// Voronoi cells: (distance to the cell's edge, the cell's id hash).
vec2 w_cells(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	float d1 = 8.0;
	float d2 = 8.0;
	float id = 0.0;
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 g = vec2(float(x), float(y));
			vec2 o = 0.15 + 0.7 * w_hash2(i + g);
			float d = length(g + o - f);
			if (d < d1) {
				d2 = d1;
				d1 = d;
				id = w_hash(i + g);
			} else if (d < d2) {
				d2 = d;
			}
		}
	}
	return vec2(d2 - d1, id);
}

// Fog of war (the minimap's vision grid): what our team doesn't see is darkened and cooled.
vec3 w_fogged(vec3 c, vec3 wp) {
	if (fog_on < 0.5 || fog_size.x <= 0.0) {
		return c;
	}
	vec2 uv = wp.xz / fog_size;
	float seen = texture(fog_map, clamp(uv, vec2(0.002), vec2(0.998))).r;
	vec3 dark = mix(vec3(dot(c, vec3(0.3, 0.55, 0.15))), c, 0.45) * vec3(0.42, 0.46, 0.58);
	return mix(dark, c, seen);
}

// The shared light() body: a wrapped Lambert (the shadowed side falls off softly), cloud
// shadows on the sun only. The shade's coolness is the environment's sky fill (the season's).
#define WORLD_LIGHT 	float ndl = clamp((dot(NORMAL, LIGHT) + 0.3) / 1.3, 0.0, 1.0); 	DIFFUSE_LIGHT += LIGHT_COLOR / PI * ndl * ATTENUATION * (LIGHT_IS_DIRECTIONAL ? cloud_sun : 1.0);
