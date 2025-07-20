@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;

@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    var color:vec4<f32> = textureSample(input_texture, input_sampler, in.uv);
    {% if material.as_post_process().gamma_correction %}
        color = vec4(pow(color.rgb, vec3<f32>(1.0 / 2.2)), color.a);
    {% endif %}

    return color;
}