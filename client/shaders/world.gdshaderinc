// The world's shared look (15 §7): noise for the procedural materials, the fog of war, and one
// lighting model for the ground, the cliffs and the props, so they sit in the same light.
//
// Include clouds.gdshaderinc first.

global uniform sampler2D fog_map : filter_linear;
global uniform vec2 fog_size;
global uniform float fog_on;
// The world's noise (05 §2, D81): the smooth value noise of the hash below, baked once at load
// (`mftr_client::noise`, 2048² texels, repeating every W_NOISE_PERIOD cells) and sampled with
// mipmaps. One fetch where hashing the four corners took dozens of integer multiplies, and
// octaves finer than a pixel fade to their mean instead of shimmering.
global uniform sampler2D world_noise : filter_linear_mipmap, repeat_enable;
const float W_NOISE_PERIOD = 256.0;      // mftr_client::noise::PERIOD
// Settings → Graphics → World detail (D81): 2 high, everything; 1 medium, fBm without its finest
// octave; 0 low, two octaves and no grain, blades of grass, cracks, pebbles or crevices. What's
// left out adds its mean, so the materials' shapes and colors stay where they were.
global uniform int world_detail;

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

// A point on a material and its footprint on the screen: how far it moves one pixel right (dx)
// and one down (dy). The noise picks its mip level from them, so it filters right inside
// branches too, where the GPU can't take derivatives. Make one in fragment() with
// WP(p, dFdx(p), dFdy(p)); w_at scales and moves it with the point.
struct WP {
	vec2 p;
	vec2 dx;
	vec2 dy;
};

WP w_at(WP a, float scale, vec2 offset) {
	return WP(a.p * scale + offset, a.dx * scale, a.dy * scale);
}

// Scaled differently along each axis.
WP w_at2(WP a, vec2 scale, vec2 offset) {
	return WP(a.p * scale + offset, a.dx * scale, a.dy * scale);
}

// Turned by `m` (and scaled with it), then moved.
WP w_turn(WP a, mat2 m, vec2 offset) {
	return WP(m * a.p + offset, m * a.dx, m * a.dy);
}

// Smooth value noise, about 0..1, one cell per unit of `a.p`.
float w_noise(WP a) {
	return textureGrad(world_noise, a.p / W_NOISE_PERIOD, a.dx / W_NOISE_PERIOD, a.dy / W_NOISE_PERIOD).r;
}

// Fine grain (D80): smooth value noise at about `cells` per meter, two octaves turned against
// each other, so close up it's a soft mottle and never a grid of squares. Low detail: none.
float w_grain(WP a, float cells) {
	if (world_detail < 1) {
		return 0.5;
	}
	const mat2 turn = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8));
	return 0.6 * w_noise(w_at(a, cells, vec2(0.0))) + 0.4 * w_noise(w_turn(a, turn * cells * 2.1, vec2(31.0)));
}

// Round specks: in a grid of `cells` per meter, a dot of `size` (share of a cell) at a jittered
// spot in the cells whose hash passes `keep`. 1 inside a dot, soft at its edge.
float w_specks(vec2 p, float cells, float size, float keep) {
	vec2 g = p * cells;
	vec2 i = floor(g);
	vec2 o = 0.25 + 0.5 * w_hash2(i + 11.0);
	float d = length(g - i - o);
	return step(1.0 - keep, w_hash(i + 5.0)) * (1.0 - smoothstep(size * 0.6, size, d));
}

float w_fbm(WP a) {
	// Each octave turned (~37°) so the value noise's grid never lines up into squares.
	const mat2 turn = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8));
	int octaves = 2 + clamp(world_detail, 0, 2);
	float v = 0.0;
	float amp = 0.5;
	a = w_turn(a, turn, vec2(0.0));
	for (int i = 0; i < 4; i++) {
		v += amp * (i < octaves ? w_noise(a) : 0.5);
		a = w_turn(a, turn * 2.03, vec2(17.0, 9.0));
		amp *= 0.5;
	}
	return v;
}

// Voronoi cells: (distance to the cell's edge, the cell's id hash). One hash a neighbor: its
// two halves place the cell's point, and their mix is its id.
vec2 w_cells(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	float d1 = 8.0;
	float d2 = 8.0;
	float id = 0.0;
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 g = vec2(float(x), float(y));
			uvec2 h = w_pcg2d(uvec2(ivec2(i + g) + ivec2(1 << 20)));
			vec2 o = 0.15 + 0.7 * vec2(h >> 8u) / 16777215.0;
			float d = length(g + o - f);
			if (d < d1) {
				d2 = d1;
				d1 = d;
				id = float((h.x ^ h.y) >> 8u) / 16777215.0;
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
// shadows on the sun only, and a little of the sun bounced back off the ground into the faces
// turned from it, so a shaded cliff reads as warm stone rather than the sky's blue. The shade's
// coolness is the environment's sky fill (the season's).
#define WORLD_LIGHT 	float ndl = clamp((dot(NORMAL, LIGHT) + 0.3) / 1.3, 0.0, 1.0); 	DIFFUSE_LIGHT += LIGHT_COLOR / PI * (ndl * ATTENUATION * (LIGHT_IS_DIRECTIONAL ? cloud_sun : 1.0) + (LIGHT_IS_DIRECTIONAL ? 0.2 * (1.0 - ndl) : 0.0));
