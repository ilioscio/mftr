// Linear skillshot telegraph (05 §1 grammar, D14): a rounded leading edge at the true hitbox
// front, a constant-width body whose faint edge sheath marks the full hitbox, a slim bright core
// with flowing noise, and a trail that fades *behind* the hitbox, never ahead of or wider than it.
// Hard CC adds chevrons in the shared accent color. Drawn on a flat quad spanning
// x ∈ [-trail, radius] (forward = +x) and y ∈ [-radius, radius], in game units.
//
// The sheath is the team's color (05 §1.3); the body and core are the champion's (14 §4.4), in
// one of its motifs: 0 streak, 1 wave, 2 shard, 3 beam, 4 bones, 5 flame, 6 bolt.
shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never;

uniform vec3 color : source_color = vec3(0.95, 0.35, 0.25);      // the body (signature)
uniform vec3 core_color : source_color = vec3(1.0, 0.97, 0.9);   // the hot core
uniform vec3 edge_color : source_color = vec3(0.95, 0.35, 0.25); // the sheath (team)
uniform vec3 accent : source_color = vec3(1.0, 0.85, 0.25);
uniform int motif = 0;
uniform float radius = 35.0;
uniform float trail = 140.0;
uniform float dim = 1.0;          // 1 = confirmed, lower = predicted interception (03a §7)
uniform float hard_cc = 0.0;

float hash(vec2 p) {
	p = fract(p * vec2(123.34, 456.21));
	p += dot(p, p + 45.32);
	return fract(p.x * p.y);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), u.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), u.x), u.y);
}

void fragment() {
	float x = mix(-trail, radius, UV.x);
	float y = mix(-radius, radius, UV.y);
	float ay = abs(y) / radius;
	// The hitbox: the circle around the missile's center, extended backwards as the swept body.
	float body = x <= 0.0 ? step(ay, 1.0) : step(length(vec2(x, y)), radius);
	if (body < 0.5) {
		discard;
	}
	float behind = clamp(-x / trail, 0.0, 1.0);          // 0 at the center, 1 at the trail's end
	float fade = 1.0 - behind;
	// Edge sheath: always visible at the true width.
	float edge = smoothstep(0.82, 0.97, x <= 0.0 ? ay : length(vec2(x, y)) / radius);
	// Slim core with flowing noise, brightest at the head.
	float n = noise(vec2(x * 0.08 + TIME * 14.0, y * 0.15));
	float core = (1.0 - smoothstep(0.18, 0.42, ay)) * (0.75 + 0.25 * n) * mix(1.0, 0.25, behind);
	// Leading edge: a bright rim on the front half of the hitbox circle.
	float lead = x > 0.0 ? smoothstep(0.7, 1.0, length(vec2(x, y)) / radius) : 0.0;
	float body_a = 0.18 * fade;
	if (motif == 1) {
		// Wave: arcs of a ground wave rolling forward, bulging at the front.
		float w = fract((x + ay * ay * radius * 0.6) / (radius * 0.55) - TIME * 6.0);
		float arc = (1.0 - smoothstep(0.0, 0.25, w)) * (1.0 - ay * 0.6) * fade;
		core = max(core * 0.6, arc);
		body_a = 0.35 * fade;
	} else if (motif == 2) {
		// Shard: chunky stone fragments tumbling through the core.
		vec2 g = floor(vec2(x - TIME * 900.0, y) / (radius * 0.35));
		float chunk = step(0.55, hash(g)) * (1.0 - smoothstep(0.3, 0.8, ay));
		core = max(core * 0.7, chunk * mix(1.0, 0.3, behind));
	} else if (motif == 3) {
		// Beam: a full-length column of light, no taper, pulsing.
		core = (1.0 - smoothstep(0.25, 0.6, ay)) * (0.85 + 0.15 * sin(TIME * 40.0 + x * 0.05));
		body_a = 0.45;
		fade = 1.0;
	} else if (motif == 4) {
		// Bones: segmented ribs across the body.
		float rib = step(0.62, fract((x - TIME * 300.0) / (radius * 0.45))) * (1.0 - smoothstep(0.55, 0.9, ay));
		core = max(core * 0.5, rib * fade);
	} else if (motif == 5) {
		// Flame: a flickering, licking core.
		float f = noise(vec2(x * 0.06 - TIME * 18.0, y * 0.2 + TIME * 4.0));
		core = (1.0 - smoothstep(0.2 + 0.4 * f, 0.7 + 0.2 * f, ay)) * (0.6 + 0.4 * f) * mix(1.0, 0.2, behind);
		body_a = 0.28 * fade;
	} else if (motif == 6) {
		// Bolt: a needle-thin core, fletching chevrons at the back.
		core = (1.0 - smoothstep(0.06, 0.16, ay)) * mix(1.0, 0.4, behind);
		float fletch = step(0.75, behind) * step(0.0, 1.0 - behind) * (1.0 - smoothstep(0.0, 0.1, abs(ay - (behind - 0.75) * 2.4)));
		core = max(core, fletch);
	}
	vec3 c = color * (0.55 + 0.45 * fade);
	float a = body_a + 0.5 * edge * (0.35 + 0.65 * fade);
	c = mix(c, edge_color, edge * 0.85);
	c = mix(c, core_color, clamp(core + lead, 0.0, 1.0));
	a = max(a, clamp(core + lead, 0.0, 1.0));
	if (hard_cc > 0.5 && x < 0.0) {
		float chevron = step(0.5, fract((-x + abs(y) * 0.9) / (radius * 0.9) - TIME * 3.0));
		float band = (1.0 - smoothstep(0.55, 0.75, ay)) * step(0.3, ay) * fade;
		c = mix(c, accent, chevron * band);
		a = max(a, 0.8 * chevron * band);
	}
	ALBEDO = c;
	ALPHA = a * dim;
}
