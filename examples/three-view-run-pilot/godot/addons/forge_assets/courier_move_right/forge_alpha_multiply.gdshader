shader_type canvas_item;
render_mode blend_mul;

// Fragment COLOR already includes the texture and inherited modulation.
// Transparent source pixels multiply by neutral white. Alpha 1 preserves the
// destination alpha under MUL instead of replacing it with the source alpha.
void fragment() {
    vec4 texel = COLOR;
    COLOR = vec4(mix(vec3(1.0), texel.rgb, texel.a), 1.0);
}
