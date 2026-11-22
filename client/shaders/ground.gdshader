// The ground (15 §3, measured in R06 §1): one shader over four painted materials, grass, dirt,
// paving and water, chosen by the map's ground control map (baked per map from its own data,
// `mftr_client::ground`), so every map the editor makes is painted like an official one. Every
// boundary is broken with noise: grass tongues over the dirt, loose slabs off the paving, ragged
// banks. A soft fine grain gives it texture up close (D80); low contrast keeps gameplay on
// top (05 §1). The season (15 §9) sets every color. No textures besides the baked control map
// and the world's noise, generated at load (D81).
//
// It's the costliest shader on screen (it covers most of it), so it skips what can't show:
// the land under deep water, the grass under full dirt, the dirt where there's none, the
// lamps' flicker out of their reach (05 §9).
shader_type spatial;
// Not specular_disabled: puddles (D82) glint in the lights (`light()`); SPECULAR stays 0, so no
// ambient reflection anywhere.
render_mode diffuse_lambert;
#include "res://shaders/clouds.gdshaderinc"
#include "res://shaders/world.gdshaderinc"
#include "res://shaders/grass.gdshaderinc"

// The ground control map: R dirt, G paving, B water, A signed wall distance (0.5 at a wall's
// edge, 1 at 10 m out from one, 0 at 10 m inside one).
uniform sampler2D control : filter_linear, repeat_disable;
uniform bool control_on = false;
uniform vec2 map_size = vec2(100.0);     // meters, what the control map covers

// The season's palette.
uniform vec3 dirt : source_color = vec3(0.42, 0.32, 0.22);
uniform vec3 dirt_dark : source_color = vec3(0.31, 0.23, 0.16);
uniform vec3 paving : source_color = vec3(0.42, 0.45, 0.47);
uniform vec3 paving_dark : source_color = vec3(0.27, 0.30, 0.33);
uniform vec3 water_shallow : source_color = vec3(0.24, 0.45, 0.42);
uniform vec3 water_deep : source_color = vec3(0.10, 0.27, 0.33);
uniform vec3 foam : source_color = vec3(0.62, 0.74, 0.70);
uniform vec3 mud : source_color = vec3(0.24, 0.21, 0.15);

// Local lights (15 §7, `map_view.gd`): the nearest lamps as (x, z, reach, strength) in meters,
// and (r, g, b, flicker phase); fires flicker, crystals hardly.
uniform vec4 lamps[16];
uniform vec4 lamp_colors[16];
uniform int lamp_count = 0;
// Contact shadows (D80, `map_view.gd`): units on the ground as (x, z, radius, strength) in meters.
// A soft disc of occlusion under each, deepest at its feet, so what stands reads as standing.
uniform vec4 occluders[32];
uniform int occluder_count = 0;

// Sandboxes without a control map: 0, a diagonal lane (the M1 arena) and a 1 m grid.
uniform float lane_mode = 0.0;
uniform float lane_width = 9.0;

varying vec3 world_pos;
// How much of a puddle this point is (D82), for the light's glint off it.
varying float puddle;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

// Contact shadows drawn at low world detail: the nearest few (`map_view.gd` sends them nearest
// the camera's look point first).
const int OCCLUDERS_LOW = 12;

vec3 dirt_color(WP P, float grain) {
	vec3 d = mix(dirt_dark, dirt, smoothstep(0.3, 0.7, w_fbm(w_at(P, 0.35, vec2(7.0)))));
	d *= 0.9 + 0.12 * (grain - 0.5) + 0.1 * w_fbm(w_at(P, 0.9, vec2(0.0)));
	if (world_detail < 1) {
		return d;
	}
	// Thin dark cracks (cell edges of a Voronoi), only in patches.
	float cracked = smoothstep(0.55, 0.7, w_fbm(w_at(P, 0.18, vec2(0.0))));
	if (cracked > 0.0) {
		vec2 cr = w_cells(P.p * 0.9 + 13.0);
		d = mix(d, dirt_dark * 0.8, (1.0 - smoothstep(0.01, 0.03, cr.x)) * 0.5 * cracked);
	}
	// A few small pebbles.
	vec2 pb = w_cells(P.p * 6.0 + 31.0);
	float pebble = step(0.975, pb.y) * smoothstep(0.08, 0.16, pb.x);
	d = mix(d, paving * (0.7 + 0.3 * pb.y), pebble * 0.55);
	return d;
}

// Big irregular slabs with dark joints and grass in the joints; `amount` is the paving's
// control value (the outer slabs come loose and the grass takes the joints toward the edge).
vec3 paving_color(WP P, float grain, float amount, vec3 under, out float covered) {
	vec2 c = w_cells(P.p * 1.3);
	float slab_id = c.y;
	vec3 slab = mix(paving_dark, paving, 0.35 + 0.65 * slab_id) * (0.9 + 0.1 * (grain - 0.5) + 0.12 * w_fbm(w_at(P, 1.3, vec2(0.0))));
	// Moss and grime over some slabs.
	slab = mix(slab, grass_dark * 1.1, smoothstep(0.6, 0.8, w_fbm(w_at(P, 0.4, vec2(5.0)))) * 0.35);
	// Worn: lighter toward each slab's middle, darker at its rim.
	slab *= 0.9 + 0.12 * smoothstep(0.02, 0.25, c.x);
	float joint = 1.0 - smoothstep(0.025, 0.07, c.x);
	// Near the edge: grass in the joints, then missing slabs.
	float edge = 1.0 - smoothstep(0.3, 1.0, amount);
	float keep = step(edge * 0.95, slab_id * 0.9 + 0.1);
	covered = keep;
	vec3 joint_col = mix(paving_dark * 0.55, grass_dark, smoothstep(0.2, 0.7, edge + grain * 0.3));
	vec3 col = mix(slab, joint_col, joint);
	return mix(under, col, keep);
}

void fragment() {
	cloud_sun = cloud_shade(world_pos);
	vec2 p = world_pos.xz;
	WP P = WP(p, dFdx(p), dFdy(p));
	float grain = w_grain(P, 14.0);
	vec3 ground = vec3(0.0);
	float on_water = 0.0;
	if (control_on) {
		// Sample the control map with a warped lookup, so its edges wander like a painter's.
		vec2 warp = vec2(w_fbm(w_at(P, 0.3, vec2(0.0))) - 0.5, w_fbm(w_at(P, 0.3, vec2(9.0))) - 0.5) * 1.6;
		vec4 c = texture(control, (p + warp) / map_size);
		vec4 c0 = texture(control, p / map_size);
		float wall = clamp(c0.a * 2.0 - 1.0, 0.0, 1.0);   // 0 at (and in) a wall, 1 at 10 m from one
		float gloom = (1.0 - smoothstep(0.05, 0.4, wall)) * 0.55;
		// The water's reach first: under deep water, the land isn't drawn.
		float bank_noise = (w_fbm(w_at(P, 0.25, vec2(5.0))) - 0.5) * 0.35 + (w_fbm(w_at(P, 1.5, vec2(0.0))) - 0.5) * 0.08;
		float wb = c.b + bank_noise;
		float water = smoothstep(0.32, 0.4, wb);
		on_water = water;
		if (water < 1.0) {
			// Dirt with grass tongues (and no grass drawn where the dirt covers it).
			float tongue = (w_fbm(w_at(P, 0.55, vec2(2.0))) - 0.5) * 0.55 + (w_fbm(w_at(P, 2.2, vec2(0.0))) - 0.5) * 0.12;
			float dirt_m = smoothstep(0.42, 0.58, c.r + tongue);
			if (dirt_m < 1.0) {
				ground = grass_color(P, grain, gloom);
				// A darker rim where grass meets dirt.
				float rim = smoothstep(0.3, 0.45, c.r + tongue) * (1.0 - dirt_m);
				ground = mix(ground, ground * 0.82, rim * 0.6);
			}
			if (dirt_m > 0.0) {
				ground = mix(ground, dirt_color(P, grain), dirt_m);
			}
			// Paving: slabs, coming loose toward the edge.
			float pave = c0.g + (w_fbm(w_at(P, 0.6, vec2(21.0))) - 0.5) * 0.25;
			if (pave > 0.12) {
				float covered;
				vec3 slabs = paving_color(P, grain, pave, ground, covered);
				ground = mix(ground, slabs, smoothstep(0.12, 0.3, pave));
			}
		}
		// Water: a muddy wet bank, then shallows to deeps with ripples and foam at the edge.
		if (wb > 0.0) {
			float wet = smoothstep(0.0, 0.25, wb);
			float depth = smoothstep(0.4, 1.0, wb);
			vec3 w = mix(water_shallow, water_deep, depth * (0.75 + 0.25 * w_fbm(w_at(P, 0.15, vec2(2.0)))));
			float flow = TIME * 0.25;
			float ripple = smoothstep(0.62, 0.7, w_fbm(w_at(P, 0.6, vec2(flow, flow * 0.6))));
			w = mix(w, water_shallow * 1.35, ripple * 0.3 * (1.0 - depth * 0.5));
			float shore = (1.0 - smoothstep(0.4, 0.5, wb)) * water;
			w = mix(w, foam, shore * (0.45 + 0.35 * sin(TIME * 1.7 + p.x * 0.8 + p.y * 0.5)));
			if (water < 1.0) {
				ground = mix(ground, mud * (0.85 + 0.2 * w_fbm(P)), wet * (1.0 - water));
			}
			ground = mix(ground, w, water);
		}
		// The foot of every wall: a soft dark band of occlusion.
		float foot = 1.0 - smoothstep(0.0, 0.11, wall);
		ground *= 1.0 - foot * 0.45;
	} else {
		ground = grass_color(P, grain, 0.0);
		// The sandbox's diagonal lane.
		float d = abs(p.x - p.y) / sqrt(2.0);
		float edge = lane_width * 0.5 + (w_fbm(w_at(P, 0.25, vec2(0.0))) - 0.5) * 3.0;
		float lane = (1.0 - smoothstep(edge - 0.35, edge + 0.35, d)) * (1.0 - lane_mode);
		if (lane > 0.0) {
			ground = mix(ground, dirt_color(P, grain), lane);
		}
		// Subtle 1 m grid to help judge distances.
		vec2 g = abs(fract(p) - 0.5);
		float grid = 1.0 - smoothstep(0.47, 0.5, max(g.x, g.y));
		ground *= 1.0 - 0.06 * (1.0 - grid);
	}
	// Rain (D82): the land soaked darker, and puddles where the ground dips (a broad noise),
	// mirrors of the sky that ring with the drops while it rains.
	puddle = 0.0;
	float ring = 0.0;
	if (wetness > 0.0) {
		float land = 1.0 - on_water;
		ground = mix(ground, w_soak(ground, wetness), land);
		float dip = w_fbm(w_at(P, 0.3, vec2(13.0)));
		puddle = smoothstep(0.6, 0.65, dip) * wetness * land;
		if (puddle > 0.0) {
			ground = mix(ground, ground * 0.28, puddle);
			if (rain > 0.0) {
				// One drop a cell, landing at its own time: a ring spreading and fading.
				vec2 rc = p * 2.4;
				vec2 ci = floor(rc);
				float h = w_hash(ci + 7.0);
				vec2 at = ci + 0.3 + 0.4 * vec2(h, fract(h * 13.7));
				float t = fract(TIME * 1.3 + h * 5.0);
				float d = distance(rc, at);
				ring = (1.0 - smoothstep(0.0, 0.06, abs(d - t * 0.4))) * (1.0 - t) * rain * puddle;
				ground = mix(ground, vec3(0.5), ring * 0.22);
			}
		}
	}
	float open = 1.0;
	int occluders_n = world_detail >= 1 ? occluder_count : min(occluder_count, OCCLUDERS_LOW);
	for (int i = 0; i < occluders_n; i++) {
		vec4 o = occluders[i];
		float d = distance(p, o.xy) / max(o.z, 0.05);
		open *= 1.0 - 0.42 * o.w * (1.0 - smoothstep(0.15, 1.25, d));
	}
	ground *= open;
	ALBEDO = w_fogged(ground, world_pos);
	ROUGHNESS = 1.0;
	SPECULAR = 0.0;
	// The lamps' pools: light falling off with the square of the distance's share of the reach.
	vec3 lit = vec3(0.0);
	for (int i = 0; i < lamp_count; i++) {
		vec4 l = lamps[i];
		float f = 1.0 - clamp(distance(p, l.xy) / max(l.z, 0.01), 0.0, 1.0);
		if (f > 0.0) {
			float ph = lamp_colors[i].a;
			float flicker = 0.88 + 0.12 * sin(TIME * 11.0 + ph) * sin(TIME * 6.7 + ph * 2.3);
			lit += lamp_colors[i].rgb * (f * f * l.w * flicker);
		}
	}
	EMISSION = ALBEDO * lit * 2.0;
	// A puddle mirrors the sky (as it'd look from the camera's pitch), broken where a drop rings.
	// Its rim is only the soaked dark; the water's sheen is in its middle.
	EMISSION += sky_mirror * smoothstep(0.35, 1.0, puddle) * (1.0 - ring * 0.6) * 0.6;
}

void light() {
	WORLD_LIGHT
	// A puddle catches a brazier's light as a glint. The sun or moon only faintly: from this
	// camera, the whole ground lies near the angle that mirrors it, and it's behind the clouds.
	if (puddle > 0.0) {
		vec3 h = normalize(VIEW + LIGHT);
		float nh = max(dot(NORMAL, h), 0.0);
		SPECULAR_LIGHT += pow(nh, 120.0) * 2.5 * puddle * ATTENUATION * LIGHT_COLOR * (LIGHT_IS_DIRECTIONAL ? 0.1 * cloud_sun : 1.0);
	}
}
