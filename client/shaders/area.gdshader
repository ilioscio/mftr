// Delayed ground AoE telegraph (05 §1 grammar): a crisp outline at the true radius from cast
// start, an inside that fills radially toward the edge exactly at detonation (so you can read
// how long you have), and a bright flash on detonation. Drawn on a flat quad of 2R × 2R.
//
// The outline is the team's color (who it threatens, 05 §1.3); the inside is the champion's
// signature color and one of its motifs (14 §4.4), so a Rockfall reads as rocks about to land and
// a Hail of Arrows as arrows: 0 plain, 1 cracks, 2 rocks, 3 frost, 4 flames, 5 bones, 6 stars,
// 7 arrows, 8 sigil, 9 shadow.
shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never;

uniform vec3 color : source_color = vec3(0.95, 0.35, 0.25);   // the inside (signature)
uniform vec3 core : source_color = vec3(1.0, 0.95, 0.85);     // the motif's bright marks
uniform vec3 edge_color : source_color = vec3(0.95, 0.35, 0.25);  // the outline (team)
uniform int motif = 0;
uniform float radius = 160.0;
uniform float progress = 0.0;     // 0 at cast, 1 at detonation
uniform float detonated = 0.0;    // 0..1 flash fade after detonation
uniform vec3 rim_color : source_color = vec3(1.0);  // hard CC: a gold outline (05 §1)
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

// Distance to the nearest cell point and that point's id (Worley).
vec3 cells(vec2 p) {
	vec2 i = floor(p);
	float best = 9.0;
	vec2 id = vec2(0.0);
	for (int y = -1; y <= 1; y++) {
		for (int x = -1; x <= 1; x++) {
			vec2 c = i + vec2(float(x), float(y));
			vec2 o = vec2(hash(c), hash(c + 17.3));
			float d = length(p - c - o);
			if (d < best) {
				best = d;
				id = c;
			}
		}
	}
	return vec3(best, id);
}

// How strongly the motif marks this point (0..1). `q` is the point in units of the radius.
float mark(vec2 q, float r, float ang) {
	float t = TIME;
	if (motif == 1) {
		// Cracks: jagged lines radiating from the center, reaching out as the cast nears.
		float a = ang * 7.0 / 6.2832;
		float k = floor(a + 0.5);
		float wob = (noise(vec2(k * 3.1, r * 6.0)) - 0.5) * 0.5;
		float line = 1.0 - smoothstep(0.02, 0.06, abs(fract(a + 0.5) - 0.5 - wob * r));
		return line * step(r, 0.25 + progress * 0.8);
	}
	if (motif == 2) {
		// Rocks: the shadows of falling stones, darkening and growing until they land.
		vec3 c = cells(q * 3.2);
		float size = 0.12 + 0.30 * progress * (0.6 + 0.8 * hash(c.yz));
		return 1.0 - smoothstep(size - 0.04, size, c.x);
	}
	if (motif == 3) {
		// Frost: crystal facets, their edges catching light.
		vec3 c = cells(q * 4.5);
		vec3 c2 = cells(q * 4.5 + 0.11);
		return smoothstep(0.02, 0.0, abs(c.x - c2.x) - 0.02) * 0.8;
	}
	if (motif == 4) {
		// Flames: noise flickering upward through the area.
		float n = noise(vec2(q.x * 5.0, q.y * 5.0 - t * 3.5)) * noise(vec2(q.x * 9.0 + 3.0, q.y * 9.0 - t * 5.0));
		return smoothstep(0.18, 0.5, n * (0.6 + 0.8 * progress));
	}
	if (motif == 5) {
		// Bones: spikes pointing in from the rim, rib arcs between them.
		float a = fract(ang * 9.0 / 6.2832);
		float spike = step(abs(a - 0.5) * 2.0, (r - 0.45) * 1.6) * step(0.45, r);
		float rib = (1.0 - smoothstep(0.015, 0.035, abs(r - 0.35 - 0.05 * sin(ang * 9.0)))) * step(r, 0.42);
		return max(spike, rib * 0.8);
	}
	if (motif == 6) {
		// Stars: twinkling points and a slow orbit ring.
		vec2 g = floor(q * 9.0);
		float s = step(0.86, hash(g)) * (0.5 + 0.5 * sin(t * 6.0 + hash(g + 3.0) * 20.0));
		float star = s * (1.0 - smoothstep(0.08, 0.22, length(fract(q * 9.0) - 0.5)));
		float orbit = 1.0 - smoothstep(0.01, 0.025, abs(r - 0.62));
		float moon = 1.0 - smoothstep(0.04, 0.07, length(q - 0.62 * vec2(cos(t * 1.5), sin(t * 1.5))));
		return max(star, max(orbit * 0.6, moon));
	}
	if (motif == 7) {
		// Arrows: their shadows, more of them as the volley nears.
		vec2 g = floor(q * 6.0);
		vec2 f = fract(q * 6.0) - 0.5;
		float on = step(1.0 - (0.25 + 0.6 * progress), hash(g));
		float a = hash(g + 7.0) * 6.2832;
		vec2 d = vec2(cos(a), sin(a));
		float along = dot(f, d);
		float across = abs(dot(f, vec2(-d.y, d.x)));
		return on * step(abs(along), 0.32) * (1.0 - smoothstep(0.02, 0.05, across));
	}
	if (motif == 8) {
		// Sigil: two runed rings turning against each other.
		float ring1 = 1.0 - smoothstep(0.01, 0.025, abs(r - 0.78));
		float ring2 = 1.0 - smoothstep(0.01, 0.025, abs(r - 0.52));
		float ticks1 = step(0.5, fract((ang + t * 0.6) * 12.0 / 6.2832)) * (1.0 - smoothstep(0.03, 0.05, abs(r - 0.70)));
		float ticks2 = step(0.6, fract((ang - t * 0.9) * 8.0 / 6.2832)) * (1.0 - smoothstep(0.03, 0.05, abs(r - 0.44)));
		return max(max(ring1, ring2), max(ticks1, ticks2));
	}
	if (motif == 9) {
		// Shadow: a swirl drawing inward.
		float s = sin(ang * 3.0 + r * 9.0 - t * 4.0);
		return smoothstep(0.55, 0.9, s) * smoothstep(0.1, 0.4, r);
	}
	return 0.0;
}

void fragment() {
	vec2 p = (UV - 0.5) * 2.0 * radius;
	float r = length(p) / radius;
	if (r > 1.0) {
		discard;
	}
	vec2 q = p / radius;
	float ang = atan(q.y, q.x);
	float px = 2.0 / radius;                       // ~2 u of edge softness
	float outline = smoothstep(1.0 - 6.0 * px, 1.0 - 3.0 * px, r) * (1.0 - smoothstep(1.0 - px, 1.0, r));
	float fill = 1.0 - smoothstep(progress - px * 2.0, progress, r);
	float front = fill * smoothstep(progress - 0.08, progress, r);   // the growing fill's edge
	float grain = hash(floor(p * 0.25) + floor(TIME * 12.0)) * 0.08;
	float m = motif == 0 ? 0.0 : mark(q, r, ang);
	vec3 c = mix(color, core, clamp(m * (0.55 + 0.45 * progress), 0.0, 1.0));
	float a = 0.10 + 0.22 * fill + grain * fill + 0.55 * front + 0.6 * m;
	if (motif == 2) {
		// Rock shadows read dark, not bright.
		c = mix(color, color * 0.25, m);
	}
	if (detonated > 0.0) {
		c = mix(c, vec3(1.0, 0.95, 0.8), 0.6);
		a = max(a, 0.85 * detonated);
	}
	vec3 rim = mix(edge_color, mix(vec3(1.0), rim_color, hard_cc), (0.35 + 0.55 * hard_cc));
	ALBEDO = mix(c, rim, outline);
	ALPHA = clamp(max(a, 0.9 * outline), 0.0, 1.0);
}
