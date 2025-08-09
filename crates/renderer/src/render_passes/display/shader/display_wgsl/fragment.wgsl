@group(0) @binding(0) var composite_texture: texture_2d<f32>;

struct FragmentInput {
    @builtin(position) full_screen_quad_position: vec4<f32>,
}

@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(in.full_screen_quad_position.xy);

    let color: vec4<f32> = textureLoad(composite_texture, coords, 0);
    
    // Return the sampled color
    return color;
}