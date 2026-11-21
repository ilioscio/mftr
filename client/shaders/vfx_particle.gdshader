// VFX particles (23 §3, D79): soft, glowing particles drawn from procedural shapes, with no
// texture files. One MultiMesh draws every particle of a layer; each instance carries its own
// data in its transform (the kit writes the whole buffer each frame, vfx.gd `_upload`):
//   MODEL_MATRIX[3].xyz  position (m, world)
//   MODEL_MATRIX[0].xyz  velocity (m/s, world): streaks stretch along it on screen
//   MODEL_MATRIX[1].xyz  size (m), rotation (rad), stretch (s of travel drawn as a streak)
//   MODEL_MATRIX[2].xyz  shape, seed (0–1), soft (m of depth fade against the scene)
//   COLOR                the ramp's color at this age (linear), alpha the opacity
//   INSTANCE_CUSTOM      age (0–1), intensity (HDR: over 1 blooms), additive (1 adds light,
//                        0 covers like smoke), unused
// Output is premultiplied, so glowing and covering particles share one blend mode.
// Shapes: 0 glow, 1 spark, 2 flame, 3 smoke, 4 shard, 5 star, 6 ring (flat), 7 disc (flat).
// Never used for gameplay edges: telegraphs and hitbox sheaths stay exact (05 §1).
shader_type spatial;
// skip_vertex_transform: VERTEX is set here in view space (the depth fade reads it), not
// computed by Godot from the packed transform, which isn't one.
render_mode unshaded, cull_disabled, depth_draw_never, shadows_disabled, blend_premul_alpha, fog_disabled, skip_vertex_transform;

uniform sampler2D depth_tex : hint_depth_texture, filter_nearest;
// The sun's direction (towards the light), world space; lights smoke and shards.
uniform vec3 sun_dir = vec3(0.4, 0.8, 0.3);

varying flat float v_shape;
varying flat float v_seed;
varying flat float v_soft;
varying float v_aspect;
varying vec4 v_custom;

void vertex() {
	vec3 center = MODEL_MATRIX[3].xyz;
	vec3 vel = MODEL_MATRIX[0].xyz;
	float size = MODEL_MATRIX[1].x;
	float rot = MODEL_MATRIX[1].y;
	float stretch = MODEL_MATRIX[1].z;
	v_shape = MODEL_MATRIX[2].x;
	v_seed = MODEL_MATRIX[2].y;
	v_soft = MODEL_MATRIX[2].z;
	v_custom = INSTANCE_CUSTOM;
	v_aspect = 1.0;
	vec2 q = VERTEX.xy;    // -0.5..0.5
	if (v_shape > 5.5) {
		// Flat on the ground (rings, glows on the floor), turned by `rot`.
		float c = cos(rot), s = sin(rot);
		vec2 r = vec2(c * q.x - s * q.y, s * q.x + c * q.y) * size;
		vec4 vp = VIEW_MATRIX * vec4(center + vec3(r.x, 0.0, r.y), 1.0);
		VERTEX = vp.xyz;
		POSITION = PROJECTION_MATRIX * vp;
	} else {
		vec3 cv = (VIEW_MATRIX * vec4(center, 1.0)).xyz;
		vec2 axis = vec2(cos(rot), sin(rot));
		float len = size;
		vec2 off = vec2(0.0);
		if (stretch > 0.0) {
			// A streak along the screen-space velocity, its head at the particle and its tail
			// behind, never ahead of it.
			vec3 vv = (VIEW_MATRIX * vec4(vel, 0.0)).xyz;
			float sp = length(vv.xy);
			if (sp > 0.001) {
				axis = vv.xy / sp;
				len = size + sp * stretch;
				off = -axis * (len - size) * 0.5;
			}
		}
		v_aspect = len / size;
		vec2 perp = vec2(-axis.y, axis.x);
		if (v_shape > 1.5 && v_shape < 2.5) {
			// Flames stand tall, their base on the particle.
			q.y = q.y * 1.7 + 0.35;
		}
		vec2 o = axis * q.x * len + perp * q.y * size + off;
		VERTEX = cv + vec3(o, 0.0);
		POSITION = PROJECTION_MATRIX * vec4(VERTEX, 1.0);
	}
}

// An integer hash of a lattice point: exact at any coordinate, so the noise has no seams.
float hash(vec2 p) {
	uvec2 q = uvec2(ivec2(p)) * uvec2(1597334673u, 3812015801u);
	uint n = (q.x ^ q.y) * 1597334673u;
	return float(n >> 8u) * (1.0 / 16777216.0);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

float fbm(vec2 p) {
	float a = 0.5;
	float s = 0.0;
	for (int i = 0; i < 3; i++) {
		s += a * noise(p);
		p = p * 2.03 + vec2(1.7, 9.2);
		a *= 0.5;
	}
	return s / 0.875;
}

void fragment() {
	vec2 p = vec2(UV.x * 2.0 - 1.0, 1.0 - UV.y * 2.0);   // -1..1, +y up the screen
	float age = v_custom.x;
	float glow = v_custom.y;
	float add = v_custom.z;
	vec3 col = COLOR.rgb;
	float a = 0.0;
	vec3 c = col;
	int shape = int(v_shape + 0.5);
	float r = length(p);
	if (shape == 0) {
		// Glow: a soft halo round a hot core.
		float halo = exp(-r * r * 4.5) - exp(-4.5);
		float core = exp(-r * r * 30.0);
		a = halo;
		c = col * glow + vec3(core) * glow * 0.7 / max(halo, 0.001);
	} else if (shape == 1) {
		// Spark: a capsule along the streak with a white-hot middle, brightest at the head.
		float k = v_aspect;
		float x = p.x * k;
		float d = length(vec2(x - clamp(x, -(k - 1.0), k - 1.0), p.y));
		float body = 1.0 - smoothstep(0.35, 1.0, d);
		float head = mix(0.35, 1.0, clamp((x + k) / (2.0 * k), 0.0, 1.0));
		a = body * head;
		float core = 1.0 - smoothstep(0.0, 0.45, d);
		c = mix(col, vec3(1.0), core * 0.6) * glow;
	} else if (shape == 2) {
		// Flame: a teardrop that licks upward, eaten away by scrolling noise as it burns out;
		// white-yellow at its heart, the ramp's color toward its edge.
		vec2 f = p;
		f.x *= 1.0 + max(f.y, 0.0) * 1.1;
		float d = length(vec2(f.x * 1.15, f.y < 0.0 ? f.y * 1.5 : f.y * 1.05));
		float n = fbm(vec2(p.x * 1.8 + v_seed * 13.0, p.y * 1.4 - TIME * 3.4 + v_seed * 7.0));
		float v = (1.0 - d) + (n - 0.5) * 0.9 - age * 0.5;
		// Zero well inside the quad: where many flames pile up the light saturates, and any
		// tail reaching the quad's edge would show its rectangle.
		float edge = (1.0 - smoothstep(0.45, 0.9, abs(p.x))) * (1.0 - smoothstep(0.65, 0.92, abs(p.y)));
		a = smoothstep(0.02, 0.45, v) * edge;
		float heat = clamp(v * 1.5, 0.0, 1.0);
		vec3 hot = vec3(1.0, 0.9, 0.6);
		float h3 = heat * heat * heat;
		c = mix(col * (0.7 + 0.5 * heat), hot, h3 * 0.7) * glow;
	} else if (shape == 3) {
		// Smoke: a puff with a ragged edge, lit from the sun, thinning as it ages.
		float n = fbm(p * 1.7 + vec2(v_seed * 31.0, v_seed * 17.0 - TIME * 0.25));
		float v = (1.0 - r) + (n - 0.5) * 0.85;
		a = smoothstep(0.02, 0.55, v);
		vec3 nrm = normalize(vec3(p * 0.9, sqrt(max(1.0 - r * r, 0.0)) + 0.35));
		vec3 l = normalize((VIEW_MATRIX * vec4(sun_dir, 0.0)).xyz);
		float lit = 0.55 + 0.6 * clamp(dot(nrm, l), 0.0, 1.0) + 0.15 * (n - 0.5);
		c = col * lit * glow;
	} else if (shape == 4) {
		// Shard: a chipped crystal or stone, faceted and lit, a glint on its face.
		float k = 0.55 + 0.35 * v_seed;
		float d = abs(p.x) / k + abs(p.y) * (p.y > 0.0 ? 0.85 : 1.15);
		float w = fwidth(d) + 0.001;
		a = 1.0 - smoothstep(1.0 - w, 1.0 + w, d);
		vec3 l = normalize((VIEW_MATRIX * vec4(sun_dir, 0.0)).xyz);
		// Four facets, each its own normal.
		vec3 nrm = normalize(vec3(sign(p.x) * 0.6, sign(p.y) * 0.5, 0.65));
		float lit = 0.45 + 0.75 * clamp(dot(nrm, l), 0.0, 1.0);
		c = col * lit * glow;
	} else if (shape == 5) {
		// Star: an impact glint, four long rays and a hot core.
		float rays = exp(-abs(p.y) * 16.0) * (1.0 - abs(p.x)) + exp(-abs(p.x) * 16.0) * (1.0 - abs(p.y));
		float core = exp(-r * r * 14.0);
		a = clamp(rays + core, 0.0, 1.0);
		c = mix(col, vec3(1.0), core) * glow;
	} else if (shape == 6) {
		// Ring, flat on the ground: a shock front with a bright edge and a faint wake inside,
		// broken up a little by noise.
		float n = noise(vec2(atan(p.y, p.x) * 6.0 + v_seed * 20.0, age * 3.0));
		float w = 0.022 + 0.045 * (1.0 - age);
		float edge = exp(-pow((r - (1.0 - w)) / w, 2.0));
		float wake = (1.0 - smoothstep(0.0, 1.0 - w, r)) * smoothstep(0.45, 1.0, r) * 0.18;
		a = (edge * (0.75 + 0.5 * n) + wake) * step(r, 1.0);
		c = mix(col, vec3(1.0), edge * 0.2) * glow;
	} else {
		// Disc, flat on the ground: a pool of light (or, covering, a scorch).
		a = (1.0 - smoothstep(0.0, 1.0, r)) * (0.6 + 0.4 * exp(-r * r * 6.0));
		c = col * glow;
	}
	// Soft against the scene: fade where the particle meets the ground or a body.
	if (v_soft > 0.0) {
		float depth = texture(depth_tex, SCREEN_UV).r;
		vec4 view = INV_PROJECTION_MATRIX * vec4(SCREEN_UV * 2.0 - 1.0, depth, 1.0);
		float scene_z = view.z / view.w;
		a *= clamp((VERTEX.z - scene_z) / v_soft, 0.0, 1.0);
	}
	a = clamp(a * COLOR.a, 0.0, 1.0);
	ALBEDO = c * a;
	ALPHA = a * (1.0 - add);
}
