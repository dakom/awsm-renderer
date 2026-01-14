/*************** START tonemap.wgsl ******************/
{% include "display_wgsl/helpers/tonemap.wgsl" %}
/*************** END tonemap.wgsl ******************/

/*************** START color_space.wgsl ******************/
{% include "shared_wgsl/color_space.wgsl" %}
/*************** END color_space.wgsl ******************/

struct FragmentInput {
    @builtin(position) full_screen_quad_position: vec4<f32>,
}

@fragment
fn frag_main(in: FragmentInput) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(in.full_screen_quad_position.xy);

    var color: vec4<f32> = textureLoad(composite_texture, coords, 0);

    // Apply tone mapping to compress HDR to displayable range
    {% match tonemapping %}
        {% when ToneMapping::KhronosNeutralPbr %}
            let rgb = khronos_pbr_neutral_tonemap(color.rgb);
        {% when ToneMapping::Aces %}
            let rgb = aces_tonemap(color.rgb);
        {% when _ %}
            let rgb = color.rgb;
    {% endmatch %}

    return vec4<f32>(linear_to_srgb(rgb), color.a);
}
