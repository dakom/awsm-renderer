fn srgb_to_linear(color: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.04045);
    let low = color / 12.92;
    let high = pow((color + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, color <= cutoff);
}

fn linear_to_srgb(color: vec3<f32>) -> vec3<f32> {
    let cutoff = vec3<f32>(0.0031308);
    let low = color * 12.92;
    let high = 1.055 * pow(color, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, color <= cutoff);
}

// Smart texture sampling that handles sRGB->linear conversion
// Use this as a drop-in replacement for textureSample when you need linear color space
fn srgb_texture_load_uv(
    texture: texture_2d<f32>,
    coords: vec2<f32>,
    mip_level: u32,
) -> vec4<f32> {
    let raw_color = texture_load_uv(texture, coords, mip_level);
    return vec4<f32>(srgb_to_linear(raw_color.rgb), raw_color.a);
}

fn srgb_texture_load_2d_array_uv(
    texture: texture_2d_array<f32>,
    coords: vec2<f32>,
    array_index: i32,
    mip_level: u32,
) -> vec4<f32> {
    let raw_color = texture_load_2d_array_uv(texture, coords, array_index, mip_level);
    
    return vec4<f32>(srgb_to_linear(raw_color.rgb), raw_color.a);
}