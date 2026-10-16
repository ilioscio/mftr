// Cliffs (15 §4): the walls' stepped faces in the season's dark stone (so the walls read as tall
// against the lit floor and tops, LoL's way), with strata, dark crevices and
// moss creeping down from the top and over the ledge, and a top of the ground's own grass: a
// lighter lip where it rolls over the drop, the forest's gloom deeper in (from the ground control
// map's signed wall distance, 15 §3). Each face is shaded flat (screen-space derivatives), so the
// low-poly cliff looks cut, not smoothed.
shader_type spatial;
render_mode diffuse_lambert, specular_disabled;
#include "res://shaders/clouds.gdshaderinc"
#include "res://shaders/world.gdshaderinc"
#include "res://shaders/grass.gdshaderinc"

uniform vec3 rock : source_color = vec3(0.40, 0.42, 0.44);
uniform vec3 rock_dark : source_color = vec3(0.24, 0.26, 0.29);
uniform vec3 moss : source_color = vec3(0.22, 0.32, 0.17);
uniform float height = 1.4;
// The ground control map (as the ground's): its alpha tells how far a top is from its edge.
uniform sampler2D control : filter_linear, repeat_disable;
uniform bool control_on = false;
uniform vec2 map_size = vec2(100.0);

varying vec3 world_pos;

void vertex() {
	world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

void fragment() {
	cloud_sun = cloud_shade(world_pos);
	// Flat faces: the normal from the surface's own slope.
	vec3 wn = normalize(cross(dFdx(world_pos), dFdy(world_pos)));
	if (wn.y < -0.01 || (abs(wn.y) < 0.01 && dot(wn, CAMERA_POSITION_WORLD - world_pos) < 0.0)) {
		wn = -wn;
	}
	NORMAL = normalize((VIEW_MATRIX * vec4(wn, 0.0)).xyz);
	vec3 n = abs(wn);
	vec2 p = n.y > 0.6 ? world_pos.xz : (n.x > n.z ? world_pos.zy : world_pos.xy);
	vec2 q = floor(p * 16.0) / 16.0;                // chunky texels, like the ground
	float f = w_fbm(q * 1.7);
	float strata = smoothstep(0.42, 0.58, fract(world_pos.y * 2.2 + f * 0.8));
	vec3 c = mix(rock_dark, rock, smoothstep(0.25, 0.75, f));
	c *= 0.82 + 0.22 * strata;
	// Crevices: dark cracks in cells across the face.
	vec2 cr = w_cells(p * 1.6 + 3.0);
	c *= 1.0 - (1.0 - smoothstep(0.02, 0.07, cr.x)) * 0.45;
	// Darker toward the foot (occlusion where it meets the ground).
	c *= 0.6 + 0.4 * smoothstep(0.0, 0.8, world_pos.y);
	// Moss: down from the top edge, and over any ledge (faces looking up).
	float top_moss = smoothstep(height - 0.35, height - 0.05, world_pos.y + (w_fbm(q * 2.3) - 0.5) * 0.4);
	float ledge = smoothstep(0.45, 0.8, wn.y) * step(world_pos.y, height - 0.05);
	c = mix(c, moss * (0.85 + 0.3 * w_fbm(q * 3.0)), clamp(top_moss * 0.75 + ledge * 0.85, 0.0, 1.0));
	if (wn.y > 0.85 && world_pos.y > height - 0.02) {
		// The top is the ground's grass: the forest stands on it. Meters in from the edge decide
		// the lip and the gloom.
		vec2 pt = world_pos.xz;
		vec2 qt = floor(pt * 16.0) / 16.0;
		float inside = 3.0;
		if (control_on) {
			vec2 warp = vec2(w_fbm(pt * 0.3) - 0.5, w_fbm(pt * 0.3 + 9.0) - 0.5) * 1.6;
			inside = clamp(1.0 - texture(control, (pt + warp) / map_size).a * 2.0, 0.0, 1.0) * 10.0;
		}
		float gloom = smoothstep(0.6, 3.0, inside) * 0.85;
		c = grass_color(pt, qt, w_hash(qt), gloom);
		float lip = 1.0 - smoothstep(0.35, 0.9, inside + (w_fbm(qt * 2.0) - 0.5) * 0.4);
		c *= 1.0 + lip * 0.12;
	}
	ALBEDO = w_fogged(c, world_pos);
	ROUGHNESS = 1.0;
}

void light() {
	WORLD_LIGHT
}
