// The ground (15 §3, measured in R06 §1): one shader over four painted materials, grass, dirt,
// paving and water, chosen by the map's ground control map (baked per map from its own data,
// `mftr_client::ground`), so every map the editor makes is painted like an official one. Every
// boundary is broken with noise: grass tongues over the dirt, loose slabs off the paving, ragged
// banks. Chunky texels keep it with the faceted models (10 §2); low contrast keeps gameplay on
// top (05 §1). The season (15 §9) sets every color. No textures besides the baked control map.
shader_type spatial;
render_mode diffuse_lambert, specular_disabled;
#include "res://shaders/clouds.gdshaderinc"
#include "res://shaders/world.gdshaderinc"

// The ground control map: R dirt, G paving, B water, A wall distance (0..1 = 0..10 m).
uniform sampler2D control : filter_linear, repeat_disable;
uniform bool control_on = false;
uniform vec2 map_size = vec2(100.0);     // meters, what the control map covers

// The season's palette.
uniform vec3 grass_dark : source_color = vec3(0.14, 0.25, 0.12);
uniform vec3 grass_mid : source_color = vec3(0.20, 0.32, 0.14);
uniform vec3 grass_light : source_color = vec3(0.30, 0.41, 0.17);
uniform vec3 grass_gloom : source_color = vec3(0.10, 0.19, 0.15);   // under the canopy, by walls
uniform vec3 dirt : source_color = vec3(0.42, 0.32, 0.22);
uniform vec3 dirt_dark : source_color = vec3(0.31, 0.23, 0.16);
uniform vec3 paving : source_color = vec3(0.42, 0.45, 0.47);
uniform vec3 paving_dark : source_color = vec3(0.27, 0.30, 0.33);
uniform vec3 water_shallow : source_color = vec3(0.24, 0.45, 0.42);
uniform vec3 water_deep : source_color = vec3(0.10, 0.27, 0.33);
uniform vec3 foam : source_color = vec3(0.62, 0.74, 0.70);
uniform vec3 mud : source_color = vec3(0.24, 0.21, 0.15);
uniform vec3 flower_a : source_color = vec3(0.40, 0.24, 0.55);
uniform vec3 flower_b : source_color = vec3(0.30, 0.42, 0.68);
uniform vec3 flower_c : source_color = vec3(0.70, 0.62, 0.22);

// Sandboxes without a control map: 0, a diagonal lane (the M1 arena) and a 1 m grid.
uniform float lane_mode = 0.0;
uniform float lane_width = 9.0;
// The size of a "texel" of grain, in meters.
uniform float texel = 0.0625;

varying vec3 world_pos;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

vec3 grass_color(vec2 p, vec2 q, float grain, float gloom) {
	float large = w_fbm(p * 0.05);
	float patch = w_fbm(p * 0.16 + 3.0);
	vec3 g = mix(grass_dark, grass_mid, smoothstep(0.3, 0.6, large));
	g = mix(g, grass_light, smoothstep(0.55, 0.75, patch) * 0.8);
	g *= 0.9 + 0.16 * w_fbm(q * 0.9) + 0.07 * (grain - 0.5);
	// Blades: a fine streak pattern in chunky texels.
	g *= 0.95 + 0.08 * step(0.55, w_hash(floor(q * vec2(1.0, 0.35)) + 7.0));
	g = mix(g, grass_gloom, gloom);
	// Flowers: small clusters of specks where a patch noise is high, never evenly sprinkled.
	float cluster = smoothstep(0.66, 0.74, w_fbm(p * 0.22 + 41.0)) * (1.0 - gloom * 0.6);
	float speck = step(0.93, w_hash(q + 5.0)) * cluster;
	float pick = w_hash(floor(p * 0.4) + 3.0);
	vec3 bloom = pick < 0.4 ? flower_a : (pick < 0.75 ? flower_b : flower_c);
	return mix(g, bloom, speck * 0.85);
}

vec3 dirt_color(vec2 p, vec2 q, float grain) {
	vec3 d = mix(dirt_dark, dirt, smoothstep(0.3, 0.7, w_fbm(p * 0.35 + 7.0)));
	d *= 0.9 + 0.12 * (grain - 0.5) + 0.1 * w_fbm(q * 0.9);
	// Thin dark cracks (cell edges of a Voronoi), only in patches.
	vec2 cr = w_cells(p * 0.9 + 13.0);
	d = mix(d, dirt_dark * 0.8, (1.0 - smoothstep(0.01, 0.03, cr.x)) * 0.5 * smoothstep(0.55, 0.7, w_fbm(p * 0.18)));
	// A few small pebbles.
	vec2 pb = w_cells(p * 6.0 + 31.0);
	float pebble = step(0.975, pb.y) * smoothstep(0.08, 0.16, pb.x);
	d = mix(d, paving * (0.7 + 0.3 * pb.y), pebble * 0.55);
	return d;
}

// Big irregular slabs with dark joints and grass in the joints; `amount` is the paving's
// control value (the outer slabs come loose and the grass takes the joints toward the edge).
vec3 paving_color(vec2 p, vec2 q, float grain, float amount, vec3 under, out float covered) {
	vec2 c = w_cells(p * 1.3);
	float slab_id = c.y;
	vec3 slab = mix(paving_dark, paving, 0.35 + 0.65 * slab_id) * (0.9 + 0.1 * (grain - 0.5) + 0.12 * w_fbm(q * 1.3));
	// Moss and grime over some slabs.
	slab = mix(slab, grass_dark * 1.1, smoothstep(0.6, 0.8, w_fbm(p * 0.4 + 5.0)) * 0.35);
	// Worn: lighter toward each slab's middle, darker at its rim.
	slab *= 0.9 + 0.12 * smoothstep(0.02, 0.25, c.x);
	float joint = 1.0 - smoothstep(0.025, 0.07, c.x);
	// Near the edge: grass in the joints, then missing slabs.
	float edge = 1.0 - smoothstep(0.3, 1.0, amount);
	float keep = step(edge * 0.95, slab_id * 0.9 + 0.1);
	covered = keep;
	vec3 joint_col = mix(paving_dark * 0.55, grass_dark, smoothstep(0.2, 0.7, edge + w_hash(q) * 0.3));
	vec3 col = mix(slab, joint_col, joint);
	return mix(under, col, keep);
}

void fragment() {
	cloud_sun = cloud_shade(world_pos);
	vec2 p = world_pos.xz;
	vec2 q = floor(p / texel) * texel;     // chunky texels
	float grain = w_hash(q);
	vec3 ground;
	if (control_on) {
		// Sample the control map with a warped lookup, so its edges wander like a painter's.
		vec2 warp = vec2(w_fbm(p * 0.3) - 0.5, w_fbm(p * 0.3 + 9.0) - 0.5) * 1.6;
		vec4 c = texture(control, (p + warp) / map_size);
		vec4 c0 = texture(control, p / map_size);
		float wall = c0.a;                         // 0 inside walls, 1 at 10 m from one
		float gloom = (1.0 - smoothstep(0.05, 0.4, wall)) * 0.55;
		ground = grass_color(p, q, grain, gloom);
		// Dirt with grass tongues.
		float tongue = (w_fbm(p * 0.55 + 2.0) - 0.5) * 0.55 + (w_fbm(q * 2.2) - 0.5) * 0.12;
		float dirt_m = smoothstep(0.42, 0.58, c.r + tongue);
		vec3 d = dirt_color(p, q, grain);
		// A darker rim where grass meets dirt.
		float rim = smoothstep(0.3, 0.45, c.r + tongue) * (1.0 - dirt_m);
		ground = mix(ground, ground * 0.82, rim * 0.6);
		ground = mix(ground, d, dirt_m);
		// Paving: slabs, coming loose toward the edge.
		float pave = c0.g + (w_fbm(p * 0.6 + 21.0) - 0.5) * 0.25;
		if (pave > 0.12) {
			float covered;
			vec3 slabs = paving_color(p, q, grain, pave, ground, covered);
			ground = mix(ground, slabs, smoothstep(0.12, 0.3, pave));
		}
		// Water: a muddy wet bank, then shallows to deeps with ripples and foam at the edge.
		float bank_noise = (w_fbm(p * 0.25 + 5.0) - 0.5) * 0.35 + (w_fbm(q * 1.5) - 0.5) * 0.08;
		float wb = c.b + bank_noise;
		if (wb > 0.0) {
			float wet = smoothstep(0.0, 0.25, wb);
			float water = smoothstep(0.32, 0.4, wb);
			float depth = smoothstep(0.4, 1.0, wb);
			vec3 w = mix(water_shallow, water_deep, depth * (0.75 + 0.25 * w_fbm(p * 0.15 + 2.0)));
			float flow = TIME * 0.25;
			float ripple = smoothstep(0.62, 0.7, w_fbm(q * 0.6 + vec2(flow, flow * 0.6)));
			w = mix(w, water_shallow * 1.35, ripple * 0.3 * (1.0 - depth * 0.5));
			float shore = (1.0 - smoothstep(0.4, 0.5, wb)) * water;
			w = mix(w, foam, shore * (0.45 + 0.35 * sin(TIME * 1.7 + p.x * 0.8 + p.y * 0.5)));
			ground = mix(ground, mud * (0.85 + 0.2 * w_fbm(q)), wet * (1.0 - water));
			ground = mix(ground, w, water);
		}
		// The foot of every wall: a soft dark band of occlusion.
		float foot = 1.0 - smoothstep(0.0, 0.11, wall);
		ground *= 1.0 - foot * 0.45;
	} else {
		ground = grass_color(p, q, grain, 0.0);
		// The sandbox's diagonal lane.
		float d = abs(p.x - p.y) / sqrt(2.0);
		float edge = lane_width * 0.5 + (w_fbm(p * 0.25) - 0.5) * 3.0;
		float lane = 1.0 - smoothstep(edge - 0.35, edge + 0.35, d);
		ground = mix(ground, dirt_color(p, q, grain), lane * (1.0 - lane_mode));
		// Subtle 1 m grid to help judge distances.
		vec2 g = abs(fract(p) - 0.5);
		float grid = 1.0 - smoothstep(0.47, 0.5, max(g.x, g.y));
		ground *= 1.0 - 0.06 * (1.0 - grid);
	}
	ALBEDO = w_fogged(ground, world_pos);
	ROUGHNESS = 1.0;
}

void light() {
	WORLD_LIGHT
}
