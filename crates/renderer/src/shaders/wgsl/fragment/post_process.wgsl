{% include "utils/color_space.wgsl" %}

@group(0) @binding(0) var input_texture: texture_2d<f32>;
@group(0) @binding(1) var input_sampler: sampler;

@group(0) @binding(2) var world_position_texture_1: texture_2d<f32>;
@group(0) @binding(3) var world_position_texture_2: texture_2d<f32>;

@group(1) @binding(0) var<uniform> settings: Settings;

struct Settings {
    ping_pong: u32 
}

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


    {% if anti_aliasing %}
        let texture_size = vec2<f32>(textureDimensions(world_position_texture_1, 0));
        let pixel_coords = vec2<i32>(in.uv * texture_size);
        let world_position_1: vec4<f32> = textureLoad(world_position_texture_1, pixel_coords, 0);
        let world_position_2: vec4<f32> = textureLoad(world_position_texture_2, pixel_coords, 0);

        let current_pos = select(
            world_position_2,
            world_position_1, // pingpong is true, so current is 0
            settings.ping_pong == 1
        );

        let prev_pos = select(
            world_position_2,
            world_position_1, // pingpong is true, prev is 0
            settings.ping_pong == 1
        );

        // TODO: Apply anti-aliasing
    {% endif %}

    return vec4<f32>(rgb, color.a);
}