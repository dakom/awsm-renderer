@group(0) @binding(0) var opaque_tex: texture_2d<f32>;

{% if multisampled_geometry %}
    @group(0) @binding(1) var oit_rgb_tex: texture_multisampled_2d<f32>;
    @group(0) @binding(2) var oit_alpha_tex: texture_multisampled_2d<f32>;
{% else %}
    @group(0) @binding(1) var oit_rgb_tex: texture_2d<f32>;
    @group(0) @binding(2) var oit_alpha_tex: texture_2d<f32>;
{% endif %}

@group(0) @binding(3) var composite_tex: texture_storage_2d<rgba8unorm, write>;

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
    let oit_rgb = textureLoad(oit_rgb_tex, coords, 0);
    let oit_alpha = textureLoad(oit_alpha_tex, coords, 0);

    // Write to output texture
    textureStore(composite_tex, coords, opaque);
}
