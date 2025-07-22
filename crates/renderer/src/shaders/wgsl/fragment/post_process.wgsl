{% include "utils/color_space.wgsl" %}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;

// Input from the vertex shader
struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

{%- match tonemapping %}
    {% when Some(ToneMapping::KhronosPbrNeutral) %}
        {% include "fragment/post_process/tonemap/khronos_pbr_neutral.wgsl" %}
    {% when Some(ToneMapping::Agx) %}
        {% include "fragment/post_process/tonemap/agx.wgsl" %}
    {% when Some(ToneMapping::Filmic) %}
        {% include "fragment/post_process/tonemap/filmic.wgsl" %}
    {% when _ %}
{% endmatch %}


@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    var color:vec4<f32> = textureSample(input_texture, input_sampler, in.uv);
    var rgb: vec3<f32> = color.rgb;

    {%- match tonemapping %}
        {% when Some(_) %}
            rgb = apply_tone_mapping(rgb);
        {% when _ %}
    {% endmatch %}

    {% if gamma_correction %}
        rgb = linear_to_srgb(rgb);
    {% endif %}

    return vec4<f32>(rgb, color.a);
}