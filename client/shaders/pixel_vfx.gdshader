// Pixel-style VFX particles (10 §2, D48): camera-facing squares whose corners snap to a grid of
// `block` screen pixels, so sparks and dust read as crisp chunky pixels at any distance. Colors
// come from the instance (the kit steps them through a short ramp). Never used for gameplay
// edges: telegraphs and hitbox sheaths stay smooth (05 §1).
shader_type spatial;
render_mode unshaded, cull_disabled, depth_draw_never, shadows_disabled, blend_mix;

uniform float block = 2.0;

void vertex() {
	// Billboard around the instance's center; the instance's scale is the particle size.
	vec4 center = MODELVIEW_MATRIX * vec4(0.0, 0.0, 0.0, 1.0);
	float size = length(MODEL_MATRIX[0].xyz);
	vec4 clip = PROJECTION_MATRIX * vec4(center.xyz + vec3(VERTEX.xy * size, 0.0), 1.0);
	vec2 px = (clip.xy / clip.w * 0.5 + 0.5) * VIEWPORT_SIZE;
	px = floor(px / block + 0.5) * block;
	clip.xy = (px / VIEWPORT_SIZE * 2.0 - 1.0) * clip.w;
	POSITION = clip;
}

void fragment() {
	ALBEDO = COLOR.rgb;
	ALPHA = COLOR.a;
}
