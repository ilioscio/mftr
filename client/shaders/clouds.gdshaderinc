// Cloud shadows (05 §5): soft patches drifting across the map that dim the sun, not the sky's
// fill, so shade stays blue and nothing turns black. Procedural (no textures), smooth at any
// zoom, and the same on everything that includes it: terrain, props, champions. A shader
// computes `cloud_sun = cloud_shade(world_pos)` in fragment() and multiplies its directional
// light by it in light(). The wind is a global, moved by atmosphere.gd; 0 strength turns it off.

global uniform vec2 cloud_offset;
global uniform float cloud_strength;
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

// 1 in sunlight, down to 1 - cloud_strength under the thickest cloud.
float cloud_shade(vec3 world) {
	if (cloud_strength <= 0.0 || cloud_receive <= 0.0) {
		return 1.0;
	}
	// Turned so no octave lines up with the map's axes.
	vec2 p = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8)) * (world.xz + cloud_offset) * 0.042;
	float n = cloud_noise(p) * 0.55 + cloud_noise(p * 2.07 + 3.7) * 0.3 + cloud_noise(p * 4.3 + 9.1) * 0.15;
	return 1.0 - smoothstep(0.48, 0.72, n) * cloud_strength;
}
