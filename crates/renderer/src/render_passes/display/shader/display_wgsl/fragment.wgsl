@group(0) @binding(0) var composite_texture: texture_2d<f32>;
@group(0) @binding(1) var composite_texture_sampler: sampler;

struct FragmentInput {
    @builtin(position) full_screen_quad_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    // Sample the composite texture using the provided UV coordinates
    let color: vec4<f32> = textureSample(composite_texture, composite_texture_sampler, in.uv);
    
    // Return the sampled color
    return color;

    //return vec4<f32>(1.0, 0.0, 0.0, 1.0); // Placeholder color (red)
}