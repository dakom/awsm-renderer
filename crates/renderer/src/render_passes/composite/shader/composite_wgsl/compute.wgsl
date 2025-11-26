@group(0) @binding(0) var opaque_tex: texture_2d<f32>;

{% if multisampled_geometry %}
    @group(0) @binding(1) var oit_color_tex: texture_multisampled_2d<f32>;
{% else %}
    @group(0) @binding(1) var oit_color_tex: texture_2d<f32>;
{% endif %}

@group(0) @binding(2) var composite_tex: texture_storage_2d<rgba8unorm, write>;

@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let coords = vec2<i32>(global_id.xy);
    let dimensions = textureDimensions(opaque_tex);

    // Bounds check
    if (coords.x >= i32(dimensions.x) || coords.y >= i32(dimensions.y)) {
        return;
    }

    // Read from input texture
    let opaque = textureLoad(opaque_tex, coords, 0);
    let oit_color = textureLoad(oit_color_tex, coords, 0);

    // Compose colors with alpha blending (OIT over opaque)
    let final_rgb = oit_color.rgb * oit_color.a + opaque.rgb * (1.0 - oit_color.a);
    let final_alpha = oit_color.a + opaque.a * (1.0 - oit_color.a);
    let final_color = vec4<f32>(final_rgb, final_alpha);

    // Write to output texture
    textureStore(composite_tex, coords, final_color);
}
