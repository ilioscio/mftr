// Cloud shadows (05 §5): soft patches drifting across the map that dim the sun, not the sky's
// fill, so shade stays blue and nothing turns black. Procedural (no textures), smooth at any
// zoom, and the same on everything that includes it: terrain, props, champions. A shader
// computes `cloud_sun = cloud_shade(world_pos)` in fragment() and multiplies its directional
// light by it in light(). The wind and the cover are globals set by atmosphere.gd; 0 strength
// turns them off.

global uniform vec2 cloud_offset;
global uniform float cloud_strength;
// How much of the map is under cloud, about 0.2 (fair) to 1 (overcast), set per season (D80).
global uniform float cloud_cover;
// Weather (D82, `weather.gd`): how soaked the world is, 0 dry to 1 after a downpour, and how
// hard it's raining now (puddles ring with drops). Here, since every world shader includes this.
global uniform float wetness;
global uniform float rain;
// The sky as a puddle mirrors it (D82): grey on a rainy day, near black at night.
global uniform vec3 sky_mirror;
// 0 for things rendered outside the map (portraits), which no cloud passes over.
uniform float cloud_receive = 1.0;

varying float cloud_sun;

// Soaked (D82): darker and a little richer, as wet soil, stone and bark are. `k` 0 dry, 1 soaked.
vec3 w_soak(vec3 c, float k) {
	vec3 d = c * (1.0 - 0.4 * k);
	float l = dot(d, vec3(0.3, 0.55, 0.15));
	return max(mix(vec3(l), d, 1.0 + 0.3 * k), vec3(0.0));
}

// An integer hash of a lattice point: exact however far the wind has carried the clouds (a
// fract(sin()) hash loses precision on many GPUs once its input grows).
uint cloud_hash(vec2 cell) {
	uvec2 u = uvec2(ivec2(cell));
	uint h = u.x * 1664525u + u.y * 1013904223u;
	h ^= h >> 16u;
	h *= 2146121005u;
	h ^= h >> 15u;
	h *= 2221713035u;
	h ^= h >> 16u;
	return h;
}

float cloud_corner(vec2 cell, vec2 d) {
	float t = max(0.5 - dot(d, d), 0.0);
	float a = float(cloud_hash(cell)) * (6.2831853 / 4294967296.0);
	t *= t;
	return t * t * dot(vec2(cos(a), sin(a)), d);
}

// Simplex noise, about 0..1 (D80). Its lattice is triangles and its gradients point every way,
// so contours of it come out round: value noise's square lattice drew straight sides and
// diamond corners, and Perlin's pinched the edges into points at its lattice.
float cloud_noise(vec2 p) {
	const float F2 = 0.36602540;
	const float G2 = 0.21132487;
	vec2 i = floor(p + (p.x + p.y) * F2);
	vec2 d0 = p - (i - (i.x + i.y) * G2);
	vec2 o = d0.x > d0.y ? vec2(1.0, 0.0) : vec2(0.0, 1.0);
	vec2 d1 = d0 - o + G2;
	vec2 d2 = d0 - 1.0 + 2.0 * G2;
	return 0.5 + 35.0 * (cloud_corner(i, d0) + cloud_corner(i + o, d1) + cloud_corner(i + 1.0, d2));
}

// A cloud is about this many meters across (scales the noise below).
const float CLOUD_SIZE = 34.0;

// 1 in sunlight, down to 1 - cloud_strength under the thickest cloud (D80): puffy patches about
// 15 m across with soft edges a few meters wide, gaps of sun between them, covering about
// `cloud_cover` of the map. The finer octaves drift at 70% of the wind's speed, so the edges
// keep reshaping as the clouds go instead of sliding by as a fixed stencil.
float cloud_shade(vec3 world) {
	if (cloud_strength <= 0.0 || cloud_receive <= 0.0) {
		return 1.0;
	}
	mat2 turn = mat2(vec2(0.8, -0.6), vec2(0.6, 0.8));
	vec2 p = turn * (world.xz + cloud_offset) / CLOUD_SIZE;
	vec2 q = turn * (world.xz + cloud_offset * 0.7) / CLOUD_SIZE;
	mat2 octave = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8));
	float n = 0.5 * cloud_noise(p);
	q = octave * q * 2.03 + vec2(7.3, 1.9);
	n += 0.18 * cloud_noise(q);
	q = octave * q * 2.03 + vec2(7.3, 1.9);
	n += 0.0648 * cloud_noise(q);
	n /= 0.7448;
	float edge = 0.66 - 0.28 * cloud_cover;
	return 1.0 - smoothstep(edge - 0.1, edge + 0.14, n) * cloud_strength;
}
