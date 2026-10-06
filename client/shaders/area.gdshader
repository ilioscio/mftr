// Delayed ground AoE telegraph (05 §1 grammar): a crisp outline at the true radius from cast
// start, an inside that fills radially toward the edge exactly at detonation (so you can read
// how long you have), and a bright flash on detonation. Drawn on a flat quad of 2R × 2R.
shader_type spatial;
render_mode unshaded, blend_mix, cull_disabled, depth_draw_never;

uniform vec3 color : source_color = vec3(0.95, 0.35, 0.25);
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

void fragment() {
	vec2 p = (UV - 0.5) * 2.0 * radius;
	float r = length(p) / radius;
	if (r > 1.0) {
		discard;
	}
	float px = 2.0 / radius;                       // ~2 u of edge softness
	float outline = smoothstep(1.0 - 6.0 * px, 1.0 - 3.0 * px, r) * (1.0 - smoothstep(1.0 - px, 1.0, r));
	float fill = 1.0 - smoothstep(progress - px * 2.0, progress, r);
	float front = fill * smoothstep(progress - 0.08, progress, r);   // the growing fill's edge
	float grain = hash(floor(p * 0.25) + floor(TIME * 12.0)) * 0.08;
	vec3 c = color;
	float a = 0.10 + 0.22 * fill + grain * fill + 0.55 * front + 0.9 * outline;
	if (detonated > 0.0) {
		c = mix(c, vec3(1.0, 0.95, 0.8), 0.6);
		a = max(a, 0.85 * detonated);
	}
	ALBEDO = mix(c, mix(vec3(1.0), rim_color, hard_cc), (0.35 + 0.55 * hard_cc) * outline);
	ALPHA = clamp(a, 0.0, 1.0);
}
