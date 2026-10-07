// Placeholder champion material (05 §4): toon-banded light, rim light, a team-accent band at
// the feet (05 §1 team color language) and object-space noise detail. No textures.
shader_type spatial;
render_mode diffuse_toon, specular_disabled;

uniform vec3 base_color : source_color = vec3(0.6, 0.6, 0.65);
uniform vec3 team_accent : source_color = vec3(0.25, 0.55, 1.0);
uniform float accent_height = -0.45;
// Impact flash (10 §4.4): 1 when a hit lands, fading within ~3 frames.
uniform float flash = 0.0;

varying vec3 obj_pos;

float hash(vec3 p) {
	p = fract(p * vec3(123.34, 456.21, 789.53));
	p += dot(p, p.yzx + 45.32);
	return fract((p.x + p.y) * p.z);
}

float noise(vec3 p) {
	vec3 i = floor(p);
	vec3 f = fract(p);
	vec3 u = f * f * (3.0 - 2.0 * f);
	float a = mix(mix(hash(i), hash(i + vec3(1, 0, 0)), u.x), mix(hash(i + vec3(0, 1, 0)), hash(i + vec3(1, 1, 0)), u.x), u.y);
	float b = mix(mix(hash(i + vec3(0, 0, 1)), hash(i + vec3(1, 0, 1)), u.x), mix(hash(i + vec3(0, 1, 1)), hash(i + vec3(1, 1, 1)), u.x), u.y);
	return mix(a, b, u.z);
}

void vertex() {
	obj_pos = VERTEX;
}

void fragment() {
	float detail = noise(obj_pos * 9.0) * 0.6 + noise(obj_pos * 23.0) * 0.4;
	vec3 c = base_color * (0.85 + 0.3 * detail);
	float band = 1.0 - smoothstep(accent_height - 0.02, accent_height + 0.02, obj_pos.y);
	c = mix(c, team_accent, band);
	ALBEDO = c;
	ROUGHNESS = 0.9;
	RIM = 0.7;
	RIM_TINT = 0.4;
	EMISSION = vec3(flash);
}
