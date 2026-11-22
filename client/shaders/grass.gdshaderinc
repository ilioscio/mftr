// The season's grass (15 §3, §9), shared by the ground and the cliffs' tops so the forest's floor
// and the lane's verge are one meadow. Include world.gdshaderinc first.

uniform vec3 grass_dark : source_color = vec3(0.14, 0.25, 0.12);
uniform vec3 grass_mid : source_color = vec3(0.20, 0.32, 0.14);
uniform vec3 grass_light : source_color = vec3(0.30, 0.41, 0.17);
uniform vec3 grass_gloom : source_color = vec3(0.10, 0.19, 0.15);   // under the canopy, by walls
uniform vec3 flower_a : source_color = vec3(0.40, 0.24, 0.55);
uniform vec3 flower_b : source_color = vec3(0.30, 0.42, 0.68);
uniform vec3 flower_c : source_color = vec3(0.70, 0.62, 0.22);

vec3 grass_color(WP P, float grain, float gloom) {
	float large = w_fbm(w_at(P, 0.05, vec2(0.0)));
	float patch = w_fbm(w_at(P, 0.16, vec2(3.0)));
	vec3 g = mix(grass_dark, grass_mid, smoothstep(0.3, 0.6, large));
	g = mix(g, grass_light, smoothstep(0.55, 0.75, patch) * 0.8);
	g *= 0.9 + 0.16 * w_fbm(w_at(P, 0.9, vec2(0.0))) + 0.07 * (grain - 0.5);
	// Blades: a fine soft streak, a few centimeters across and three times as long (low
	// detail: their mean).
	float blades = world_detail >= 1 ? smoothstep(0.4, 0.7, w_noise(w_at2(P, vec2(16.0, 5.3), vec2(7.0)))) : 0.5;
	g *= 0.95 + 0.08 * blades;
	g = mix(g, grass_gloom, gloom);
	// Flowers: small clusters of specks where a patch noise is high, never evenly sprinkled.
	float cluster = smoothstep(0.66, 0.74, w_fbm(w_at(P, 0.22, vec2(41.0)))) * (1.0 - gloom * 0.6);
	if (cluster <= 0.0) {
		return g;
	}
	float speck = w_specks(P.p, 9.0, 0.32, 0.3) * cluster;
	float pick = w_hash(floor(P.p * 0.4) + 3.0);
	vec3 bloom = pick < 0.4 ? flower_a : (pick < 0.75 ? flower_b : flower_c);
	return mix(g, bloom, speck * 0.85);
}
