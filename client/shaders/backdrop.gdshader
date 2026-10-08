shader_type canvas_item;
// The menus' backdrop (05 §6): a deep blue-green dusk, a warm glow behind the title, slow mist,
// faint light shafts and a vignette. Procedural like the rest of the art: no texture files.

uniform vec3 top_color : source_color = vec3(0.05, 0.08, 0.1);
uniform vec3 bottom_color : source_color = vec3(0.02, 0.03, 0.04);
uniform vec3 glow_color : source_color = vec3(0.85, 0.62, 0.3);
uniform vec3 mist_color : source_color = vec3(0.3, 0.45, 0.5);
uniform vec2 glow_at = vec2(0.5, 0.18);
uniform float aspect = 1.78;

float hash(vec2 p) {
	return fract(sin(dot(p, vec2(127.1, 311.7))) * 43758.5453);
}

float noise(vec2 p) {
	vec2 i = floor(p);
	vec2 f = fract(p);
	vec2 u = f * f * (3.0 - 2.0 * f);
	return mix(mix(hash(i), hash(i + vec2(1, 0)), u.x), mix(hash(i + vec2(0, 1)), hash(i + vec2(1, 1)), u.x), u.y);
}

float fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	mat2 turn = mat2(vec2(0.8, 0.6), vec2(-0.6, 0.8));
	p = turn * p;  // no octave lines up with the screen, so the mist has no square edges
	for (int i = 0; i < 5; i++) {
		v += a * noise(p);
		p = turn * p * 2.03;
		a *= 0.5;
	}
	return v;
}

void fragment() {
	vec2 uv = UV;
	vec2 p = vec2(uv.x * aspect, uv.y);
	vec3 col = mix(top_color, bottom_color, smoothstep(0.0, 1.0, uv.y));

	// Mist: two layers drifting at different speeds.
	float t = TIME * 0.015;
	float m1 = fbm(p * 2.2 + vec2(t, t * 0.4));
	float m2 = fbm(p * 4.5 - vec2(t * 1.6, -t * 0.3) + 7.0);
	float mist = smoothstep(0.25, 1.05, m1 * 0.65 + m2 * 0.45) * (0.35 + 0.65 * uv.y);
	col = mix(col, mist_color * 0.35, mist * 0.55);

	// The warm glow behind the title.
	vec2 g = (uv - glow_at) * vec2(aspect, 1.0);
	float glow = exp(-dot(g, g) * 5.5);
	col += glow_color * glow * 0.22;

	// Light shafts falling from the top left.
	float along = uv.x * 0.55 + uv.y * 0.85;
	float across = uv.x * 0.85 - uv.y * 0.55;
	float shafts = pow(max(0.0, sin(across * 22.0 + 1.3) * sin(across * 9.0 - 0.4)), 3.0);
	col += glow_color * shafts * 0.035 * smoothstep(1.2, 0.0, along) * (0.6 + 0.4 * noise(vec2(across * 30.0, TIME * 0.1)));

	// Vignette and a touch of grain against banding.
	vec2 v = (uv - 0.5) * vec2(aspect / 1.78, 1.0);
	col *= 1.0 - smoothstep(0.35, 0.95, length(v)) * 0.75;
	col += (hash(uv * 1000.0 + fract(TIME)) - 0.5) * 0.012;
	COLOR = vec4(col, 1.0);
}
