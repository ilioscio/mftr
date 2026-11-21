// VFX ribbons (23 §4, D79): strips built each frame by the kit (vfx.gd `_draw_ribbons`) for
// what particles can't draw well: lightning, lasers, a swing's smear, cracks in the ground,
// columns of light. Each vertex carries:
//   VERTEX   a point on the ribbon's spine (camera-facing) or its final position (flat)
//   NORMAL   the spine's direction there (camera-facing ribbons widen across it)
//   UV       x: metres along the spine; y: style * 2 + across (0 one edge, 1 the other)
//   UV2      x: half width in m (camera-facing) or -1 (flat); y: intensity (over 1 blooms)
//   COLOR    the ramp's color (linear), alpha the life left (also how far noise has eaten it away)
// Styles: 0 lightning, 1 laser, 2 slash, 3 crack, 4 pillar. Output is premultiplied.
shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, shadows_disabled, blend_premul_alpha, fog_disabled, skip_vertex_transform;

varying flat float v_style;
varying float v_glow;

void vertex() {
	v_glow = UV2.y;
	v_style = floor(UV.y * 0.5 + 0.001);
	vec3 c = (VIEW_MATRIX * vec4(VERTEX, 1.0)).xyz;
	if (UV2.x > 0.0) {
		vec3 t = (VIEW_MATRIX * vec4(NORMAL, 0.0)).xyz;
		vec3 side = cross(t, normalize(c));
		float l = length(side);
		side = l > 0.0001 ? side / l : vec3(1.0, 0.0, 0.0);
		float across = UV.y - v_style * 2.0;
		c += side * UV2.x * (across * 2.0 - 1.0);
	}
	VERTEX = c;
	POSITION = PROJECTION_MATRIX * vec4(c, 1.0);
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

void fragment() {
	int style = int(v_style + 0.5);
	float across = UV.y - v_style * 2.0;
	float x = abs(across * 2.0 - 1.0);      // 0 on the spine, 1 at the edges
	float along = UV.x;
	float life = COLOR.a;
	vec3 col = COLOR.rgb;
	float a = 0.0;
	vec3 c = col;
	float add = 1.0;
	if (style == 0) {
		// Lightning: a white-hot thread inside a glow of the ramp's color.
		float core = exp(-pow(x / 0.16, 2.0));
		float halo = exp(-pow(x / 0.55, 2.0)) * 0.7;
		a = clamp(core + halo, 0.0, 1.0) * life;
		c = mix(col, vec3(1.0), core) * v_glow;
	} else if (style == 1) {
		// Laser: a steady white core, a colored glow pulsing down its length.
		float n = noise(vec2(along * 3.0 - TIME * 22.0, across * 2.0));
		float core = exp(-pow(x / 0.22, 2.0));
		float halo = exp(-pow(x / 0.6, 2.0)) * (0.55 + 0.45 * n);
		a = clamp(core + halo, 0.0, 1.0) * life;
		c = mix(col, vec3(1.0), core * 0.85) * v_glow;
	} else if (style == 2) {
		// Slash: a smear brightest at the blade's edge (across = 1), streaked along the swing,
		// eaten away from the inside as it fades.
		float n = noise(vec2(along * 5.0, across * 9.0));
		float edge = pow(across, 2.2);
		float v = edge + (n - 0.5) * 0.35 - (1.0 - life) * 0.9;
		a = smoothstep(0.0, 0.25, v) * smoothstep(1.0, 0.92, across) * life;
		c = mix(col, vec3(1.0), smoothstep(0.75, 0.97, across) * 0.8) * v_glow;
	} else if (style == 3) {
		// Crack: a dark, ragged split in the ground; a bright ramp burns in its core (lava,
		// light, a crimson wound), a dark one is just the split.
		float n = noise(vec2(along * 9.0, across * 3.0));
		float v = (1.0 - x) + (n - 0.5) * 0.7;
		float split = smoothstep(0.2, 0.55, v) * life;
		float lum = dot(col, vec3(0.3, 0.5, 0.2));
		float core = smoothstep(0.55, 0.95, v) * life * smoothstep(0.12, 0.3, lum);
		// Premultiplied by hand: the split covers, the core adds light.
		float cover = split * (1.0 - core) * 0.85;
		c = (col * 0.12 * cover + col * v_glow * core) / max(cover + core, 0.0001);
		a = cover + core;
		add = core / max(cover + core, 0.0001);
	} else {
		// Pillar: a soft column, rising streaks inside it, fading toward its top.
		float n = noise(vec2(across * 6.0, along * 2.0 - TIME * 6.0));
		float body = exp(-pow(x / 0.5, 2.0));
		float top = 1.0 - smoothstep(0.0, 1.0, along / 2.4);
		a = clamp(body * (0.55 + 0.6 * n) * top, 0.0, 1.0) * life;
		c = mix(col, vec3(1.0), body * body * 0.5) * v_glow;
	}
	a = clamp(a, 0.0, 1.0);
	ALBEDO = c * a;
	ALPHA = a * (1.0 - add);
}
