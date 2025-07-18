@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;

@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    return textureSample(input_texture, input_sampler, in.uv);
}