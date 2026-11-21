// Cloud shadows (05 §5): soft patches drifting across the map that dim the sun, not the sky's
// fill, so shade stays blue and nothing turns black. Procedural (no textures), smooth at any
// zoom, and the same on everything that includes it: terrain, props, champions. A shader
// computes `cloud_sun = cloud_shade(world_pos)` in fragment() and multiplies its directional
// light by it in light(). The wind is a global, moved by atmosphere.gd; 0 strength turns it off.

global uniform vec2 cloud_offset;
global uniform float cloud_strength;
// How much of the map is under cloud, about 0.2 (fair) to 1 (overcast), set per season (D80).
global uniform float cloud_cover;
// 0 for things rendered outside the map (portraits), which no cloud passes over.
uniform float cloud_receive = 1.0;

varying float cloud_sun;

float cloud_hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float cloud_noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(cloud_hash(i), cloud_hash(i + vec2(1.0, 0.0)), u.x),
			mix(cloud_hash(i + vec2(0.0, 1.0)), cloud_hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

// A cloud is about this many meters across (scales the noise below).
const float CLOUD_SIZE = 15.0;

// Three octaves of value noise, each turned so none lines up with the map's axes or another.
float cloud_fbm(vec2 p) {
	float n = 0.0;
	float a = 0.5;
	for (int i = 0; i < 3; i++) {
		n += a * cloud_noise(p);
		p = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8)) * p * 2.03 + vec2(7.3, 1.9);
		a *= 0.45;
	}
	return n / 0.82625;
}

// 1 in sunlight, down to 1 - cloud_strength under the thickest cloud (D80): puffy patches about
// CLOUD_SIZE across with soft edges a couple of meters wide, gaps of sun between them, covering
// about `cloud_cover` of the map. A slower, broader field bends their shapes, and since it drifts
// at about half the wind's speed, the clouds change shape as they go instead of sliding by as
// a fixed stencil.
float cloud_shade(vec3 world) {
	if (cloud_strength <= 0.0 || cloud_receive <= 0.0) {
		return 1.0;
	}
	mat2 turn = mat2(vec2(0.8, -0.6), vec2(0.6, 0.8));
	vec2 p = turn * (world.xz + cloud_offset) / CLOUD_SIZE;
	vec2 q = turn * (world.xz + cloud_offset * 0.55) / (CLOUD_SIZE * 2.0);
	vec2 bend = vec2(cloud_noise(q), cloud_noise(q + vec2(5.2, 1.3))) - 0.5;
	float n = cloud_fbm(p + bend * 0.55);
	float edge = 0.66 - 0.28 * cloud_cover;
	return 1.0 - smoothstep(edge - 0.07, edge + 0.1, n) * cloud_strength;
}
