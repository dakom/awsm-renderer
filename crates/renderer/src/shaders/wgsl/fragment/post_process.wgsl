{% include "utils/color_space.wgsl" %}

{% if anti_aliasing %}
    {% include "fragment/post_process/anti-alias.wgsl" %}
{% else %}
    @group(0) @binding(0) var scene_texture: texture_2d<f32>;
    @group(0) @binding(1) var scene_texture_sampler: sampler;
{% endif %}

@group(1) @binding(0) var<uniform> settings: Settings;

struct Settings {
    ping_pong: u32 
}

// Input from the vertex shader
struct FragmentInput {
    @builtin(position) full_screen_quad_position: vec4<f32>,
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

struct FragmentOutput {
    @location(0) display: vec4<f32>, // Final display output
    @location(1) accumulation: vec4<f32>, // Temporal accumulation
};


@fragment
fn frag_main(in: FragmentInput) -> FragmentOutput {

    {% if anti_aliasing %}
        var accumulated_color:vec4<f32> = anti_alias(in);
    {% else %}
        var accumulated_color:vec4<f32> = textureSample(scene_texture, scene_texture_sampler, in.uv);
    {% endif %}

    var display_rgb: vec3<f32> = accumulated_color.rgb;

    {%- match tonemapping %}
        {% when Some(_) %}
            display_rgb = apply_tone_mapping(display_rgb);
        {% when _ %}
    {% endmatch %}

    {% if gamma_correction %}
        display_rgb = linear_to_srgb(display_rgb);
    {% endif %}

    var output: FragmentOutput;
    output.display = vec4<f32>(display_rgb, accumulated_color.a);
    output.accumulation = accumulated_color; 

    return output;
}