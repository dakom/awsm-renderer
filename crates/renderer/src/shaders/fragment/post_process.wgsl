@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;

@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    {% if material.as_post_process().gamma_correction %}
        return textureSample(input_texture, input_sampler, in.uv) * vec4(1.0, 0.0, 0.0, 1.0); 
    {% else %}
        return textureSample(input_texture, input_sampler, in.uv);
    {% endif %}
}