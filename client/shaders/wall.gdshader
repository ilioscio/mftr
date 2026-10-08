// Rock walls (05 §3): world-space noise on the sides with dark crevices and strata, a lighter,
// mossy top. Low contrast so gameplay reads on top (05 §1 value hierarchy). No textures.
shader_type spatial;
render_mode diffuse_lambert, specular_disabled;

uniform vec3 rock : source_color = vec3(0.47, 0.45, 0.42);
uniform vec3 rock_dark : source_color = vec3(0.3, 0.29, 0.29);
uniform vec3 moss : source_color = vec3(0.30, 0.38, 0.24);
// The cliff top's grass, matching the ground's.
uniform vec3 top_a : source_color = vec3(0.2, 0.33, 0.15);
uniform vec3 top_b : source_color = vec3(0.27, 0.41, 0.18);

varying vec3 world_pos;

// Fog of war (the minimap's vision grid): ground our team doesn't see is darkened and cooled.
global uniform sampler2D fog_map : filter_linear;
global uniform vec2 fog_size;
global uniform float fog_on;

vec3 fogged(vec3 c, vec3 wp) {
	if (fog_on < 0.5 || fog_size.x <= 0.0) {
		return c;
	}
	vec2 uv = wp.xz / fog_size;
	// Past the map (scenery), the nearest edge of the map decides.
	float seen = texture(fog_map, clamp(uv, vec2(0.002), vec2(0.998))).r;
	vec3 dark = mix(vec3(dot(c, vec3(0.3, 0.55, 0.15))), c, 0.45) * vec3(0.42, 0.46, 0.58);
	return mix(dark, c, seen);
}

varying vec3 world_normal;

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

float fbm(vec2 p) {
	float v = 0.0;
	float a = 0.5;
	for (int i = 0; i < 4; i++) {
		v += a * noise(p);
		p = p * 2.1 + vec2(5.0, 11.0);
		a *= 0.5;
	}
	return v;
}

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_normal = normalize((MODEL_MATRIX * vec4(NORMAL, 0.0)).xyz);
}

void fragment() {
	vec3 n = abs(world_normal);
	// Triplanar-lite: project on the dominant axis.
	vec2 p = n.y > 0.6 ? world_pos.xz : (n.x > n.z ? world_pos.zy : world_pos.xy);
	vec2 q = floor(p * 16.0) / 16.0;                // chunky texels, like the ground
	float f = fbm(q * 1.7);
	float strata = smoothstep(0.42, 0.58, fract(world_pos.y * 1.6 + f * 0.8));
	vec3 c = mix(rock_dark, rock, smoothstep(0.25, 0.75, f));
	c *= 0.84 + 0.2 * strata;
	// Moss creeping down from the top edge.
	c = mix(c, moss, smoothstep(1.1, 1.38, world_pos.y + (fbm(q * 2.3) - 0.5) * 0.5) * 0.8);
	if (n.y > 0.6 || world_pos.y > 1.37) {
		// The cliff top (1.4 m) is grassland, like the ground: the forest stands on it.
		float g = fbm(q * 0.9 + 3.0);
		c = mix(top_a, top_b, smoothstep(0.35, 0.65, g)) * (0.9 + 0.12 * fract(sin(dot(q, vec2(12.9898, 78.233))) * 43758.5453));
	}
	ALBEDO = fogged(c, world_pos);
	ROUGHNESS = 1.0;
}
