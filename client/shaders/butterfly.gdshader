// Butterflies (15 §8): flat wings flapped in the vertex stage about the body (+z), so a
// MultiMesh of them needs only its transforms each frame. Color and phase per instance
// (INSTANCE_CUSTOM: rgb the wings, a the phase); vertex alpha 1 fore wings, 0.8 hind, 0 body.
shader_type spatial;
render_mode cull_disabled, specular_disabled;

global uniform sampler2D fog_map : filter_linear;
global uniform vec2 fog_size;
global uniform float fog_on;

varying vec3 wing;
varying float body;
varying vec3 world_pos;

void vertex() {
	float side = VERTEX.x;
	float flap = sin(TIME * 17.0 + INSTANCE_CUSTOM.a * 5.0) * 0.75 + 0.35;
	VERTEX.y = abs(side) * sin(flap);
	VERTEX.x = side * cos(flap);
	wing = INSTANCE_CUSTOM.rgb * (0.75 + 0.25 * COLOR.a);
	body = step(COLOR.a, 0.01);
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	vec3 c = mix(wing, vec3(0.08, 0.06, 0.05), body);
	if (fog_on > 0.5 && fog_size.x > 0.0) {
		float seen = texture(fog_map, clamp(world_pos.xz / fog_size, vec2(0.002), vec2(0.998))).r;
		c *= mix(0.4, 1.0, seen);
	}
	ALBEDO = c;
	ROUGHNESS = 1.0;
	EMISSION = c * 0.25;
}
