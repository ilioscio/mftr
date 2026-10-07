// Hover outline (10 §4.4): an inverted hull grown along the smooth normals, shown on the
// attackable target under the cursor (red for enemies). `width` 0 hides it.
shader_type spatial;
render_mode unshaded, cull_front, shadows_disabled;

uniform vec3 color : source_color = vec3(1.0, 0.18, 0.12);
uniform float width = 0.0;

void vertex() {
	VERTEX += NORMAL * width;
}

void fragment() {
	if (width <= 0.0) {
		discard;
	}
	ALBEDO = color;
}
