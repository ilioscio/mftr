// A soft vignette over the match view (15 §7, R06 §4): the corners and edges a little darker,
// so the eye stays on the middle of the screen where the fight is.
shader_type canvas_item;

uniform float strength = 0.32;
uniform float inner = 0.35;

void fragment() {
	vec2 d = (UV - 0.5) * vec2(1.0, 0.8);
	float r = length(d) * 1.4142;
	COLOR = vec4(0.02, 0.03, 0.06, smoothstep(inner, 1.05, r) * strength);
}
