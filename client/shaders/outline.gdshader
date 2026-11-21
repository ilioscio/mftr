// Hover outline (10 §4.4): an inverted hull grown along the smooth normals, shown on the
// attackable target under the cursor (red for enemies). `width` 0 hides it. The smooth normal
// rides in TANGENT (`MftrModel`): NORMAL is split at creases (D80), and a hull grown along it
// would crack there.
shader_type spatial;
render_mode unshaded, cull_front, shadows_disabled;

uniform vec3 color : source_color = vec3(1.0, 0.18, 0.12);
uniform float width = 0.0;
// Off for meshes not built by `MftrModel` (the placeholders), whose tangents are tangents.
uniform bool hull_in_tangent = true;

void vertex() {
	VERTEX += (hull_in_tangent ? TANGENT : NORMAL) * width;
}

void fragment() {
	if (width <= 0.0) {
		discard;
	}
	ALBEDO = color;
}
