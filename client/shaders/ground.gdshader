// Procedural ground: no textures, only noise (D4), quantized to chunky texels so it sits with
// the faceted PS1-style models (10 §2). Two-tone grass with a few flowers; on lane maps dirt
// roads down the lanes, ragged at their edges, paved with worn cobbles along their spines, and
// a shallow river where the map has one. Low contrast, so gameplay reads on top (05 §1 value
// hierarchy).
shader_type spatial;
render_mode diffuse_lambert, specular_disabled;
#include "res://shaders/clouds.gdshaderinc"

uniform vec3 grass_a : source_color = vec3(0.21, 0.35, 0.16);
uniform vec3 grass_b : source_color = vec3(0.29, 0.43, 0.19);
uniform vec3 dirt : source_color = vec3(0.45, 0.37, 0.26);
uniform vec3 dirt_dark : source_color = vec3(0.34, 0.28, 0.2);
uniform vec3 cobble : source_color = vec3(0.47, 0.45, 0.41);
uniform float lane_width = 9.0;
// 0: a diagonal lane (the M1 arena); 1: the roads below (lane maps).
uniform float lane_mode = 0.0;
// Road segments (a.xy, b.xy; meters, world x/z): every lane's polyline, cut into pieces.
uniform vec4 roads[48];
uniform int road_count = 0;
// The river's segments, and its width (meters).
uniform vec4 river[8];
uniform int river_count = 0;
uniform float river_width = 10.0;
uniform vec3 water_deep : source_color = vec3(0.16, 0.33, 0.36);
uniform vec3 water_shallow : source_color = vec3(0.3, 0.46, 0.42);
uniform vec3 mud : source_color = vec3(0.3, 0.27, 0.19);
// The size of a "texel" of grain, in meters.
uniform float texel = 0.0625;

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
	// Past the map (scenery), the nearest edge of the map decides.
	float seen = texture(fog_map, clamp(uv, vec2(0.002), vec2(0.998))).r;
	vec3 dark = mix(vec3(dot(c, vec3(0.3, 0.55, 0.15))), c, 0.45) * vec3(0.42, 0.46, 0.58);
	return mix(dark, c, seen);
}

float hash(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

vec2 hash2(vec2 p) {
	return vec2(hash(p), hash(p + 19.19));
}

float value_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x),
	           mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
	// Each octave turned (~37°) so the value noise's grid never lines up into squares.
	const mat2 turn = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8));
	float v = 0.0;
	float a = 0.5;
	p = turn * p;
	for (int i = 0; i < 5; i++) {
		v += a * value_noise(p);
		p = turn * p * 2.03 + vec2(17.0, 9.0);
		a *= 0.5;
	}
	return v;
}

// Cobbles: Voronoi cells; returns (distance to the cell's edge, the cell's id hash).
vec2 cobbles(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	float d1 = 8.0;
	float d2 = 8.0;
	float id = 0.0;
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 g = vec2(float(x), float(y));
			vec2 o = 0.15 + 0.7 * hash2(i + g);
			float d = length(g + o - f);
			if (d < d1) {
				d2 = d1;
				d1 = d;
				id = hash(i + g);
			} else if (d < d2) {
				d2 = d;
			}
		}
	}
	return vec2(d2 - d1, id);
}

float segment_distance(vec2 p, vec4 s) {
	vec2 ab = s.zw - s.xy;
	float t = clamp(dot(p - s.xy, ab) / max(dot(ab, ab), 1e-4), 0.0, 1.0);
	return length(p - s.xy - ab * t);
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	cloud_sun = cloud_shade(world_pos);
	vec2 p = world_pos.xz;
	vec2 q = floor(p / texel) * texel;     // chunky texels
	float large = fbm(p * 0.06);
	float grain = hash(q);
	float mid = fbm(q * 0.9);
	vec3 grass = mix(grass_a, grass_b, smoothstep(0.35, 0.65, large));
	grass *= 0.86 + 0.2 * mid + 0.08 * (grain - 0.5);
	// Flowers: a few bright specks in the grass.
	float flower = step(0.997, hash(q + 5.0)) * smoothstep(0.45, 0.7, large);
	// (Literal colors are linear; these are sRGB 0.72/0.68/0.42 and 0.66/0.5/0.62.)
	grass = mix(grass, mix(vec3(0.48, 0.42, 0.15), vec3(0.39, 0.21, 0.34), hash(q + 3.0)), flower * 0.8);
	// The road: a dirt band with ragged, grassy edges.
	float d = abs(p.x - p.y) / sqrt(2.0);
	if (lane_mode > 0.5) {
		d = 1e6;
		for (int i = 0; i < road_count; i++) {
			d = min(d, segment_distance(p, roads[i]));
		}
	}
	float edge = lane_width * 0.5 + (fbm(p * 0.25) - 0.5) * 3.0 + (fbm(q * 1.7) - 0.5) * 0.7;
	float lane = 1.0 - smoothstep(edge - 0.35, edge + 0.35, d);
	vec3 road = mix(mix(dirt_dark, dirt, 0.55), dirt, smoothstep(0.3, 0.7, fbm(p * 0.45 + 7.0)));
	road *= 0.9 + 0.14 * (grain - 0.5) + 0.1 * mid;
	// Wheel-worn ruts either side of the spine.
	float rut = smoothstep(0.5, 0.1, abs(d - 1.4 - (fbm(p * 0.1) - 0.5)));
	road = mix(road, dirt_dark * 0.9, rut * 0.35 * lane_mode);
	// Cobbles along the spine, worn away in patches.
	vec2 cb = cobbles(p * 2.2);
	float paved = (1.0 - smoothstep(1.2, 1.9, d + (fbm(p * 0.35 + 3.0) - 0.5) * 2.4)) * smoothstep(0.34, 0.5, fbm(p * 0.3 + 11.0) + (fbm(q * 1.4) - 0.5) * 0.25);
	vec3 stone = cobble * (0.78 + 0.34 * cb.y) * (0.92 + 0.12 * grain);
	stone = mix(road * 0.8, stone, smoothstep(0.03, 0.08, cb.x));   // packed dirt between
	road = mix(road, stone, paved * lane_mode);
	// Pebbles strewn over the dirt.
	vec2 pb = cobbles(p * 5.0 + 31.0);
	float pebble = step(0.93, pb.y) * smoothstep(0.12, 0.2, pb.x) * (1.0 - paved);
	road = mix(road, cobble * (0.72 + 0.3 * hash(floor(p * 5.0 + 31.0))), pebble * 0.7);
	vec3 ground = mix(grass, road, lane);
	if (river_count > 0) {
		// The river: shallow water with a muddy bank, deeper and darker toward its middle, its
		// edges as ragged as the roads'. Where a road crosses, it fords: the road shows through.
		float dr = 1e6;
		for (int i = 0; i < river_count; i++) {
			dr = min(dr, segment_distance(p, river[i]));
		}
		float bank = river_width * 0.5 + (fbm(p * 0.2 + 5.0) - 0.5) * 3.0 + (fbm(q * 1.5) - 0.5) * 0.6;
		float wet = 1.0 - smoothstep(bank + 0.4, bank + 1.8, dr);
		float water = 1.0 - smoothstep(bank - 0.3, bank + 0.3, dr);
		float depth = 1.0 - smoothstep(0.0, bank, dr);
		vec3 w = mix(water_shallow, water_deep, depth * (0.7 + 0.3 * fbm(p * 0.15 + 2.0)));
		// Ripples: thin bright bands across the current, in chunky texels.
		float ripple = smoothstep(0.62, 0.7, fbm(q * 0.8 + vec2(p.x + p.y) * 0.05));
		w = mix(w, water_shallow * 1.35, ripple * 0.35) * (0.94 + 0.1 * (grain - 0.5));
		ground = mix(ground, mud * (0.85 + 0.2 * mid), wet * (1.0 - lane * 0.6));
		ground = mix(ground, w, water * (1.0 - lane * 0.45));
	}
	if (lane_mode < 0.5) {
		// Subtle 1 m grid to help judge distances in the sandbox.
		vec2 g = abs(fract(p) - 0.5);
		float grid = 1.0 - smoothstep(0.47, 0.5, max(g.x, g.y));
		ground *= 1.0 - 0.06 * (1.0 - grid);
	}
	ALBEDO = fogged(ground, world_pos);
	ROUGHNESS = 1.0;
}

void light() {
	// Lambert, with cloud shadows dimming the sun.
	float ndl = clamp(dot(NORMAL, LIGHT), 0.0, 1.0);
	DIFFUSE_LIGHT += ndl * ATTENUATION * LIGHT_COLOR / PI * (LIGHT_IS_DIRECTIONAL ? cloud_sun : 1.0);
}
