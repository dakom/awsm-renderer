{% if smaa_anti_alias %}
    /*************** START msaa.wgsl ******************/
    {% include "display_wgsl/helpers/smaa.wgsl" %}
    /*************** END msaa.wgsl ******************/
{% endif %}

/*************** START tonemap.wgsl ******************/
{% include "display_wgsl/helpers/tonemap.wgsl" %}
/*************** END tonemap.wgsl ******************/

{% if bloom %}
    /*************** START bloom.wgsl ******************/
    {% include "display_wgsl/helpers/bloom.wgsl" %}
    /*************** END bloom.wgsl ******************/
{% endif %}

{% if dof %}
    /*************** START bloom.wgsl ******************/
    {% include "display_wgsl/helpers/dof.wgsl" %}
    /*************** END bloom.wgsl ******************/
{% endif %}

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

    {% if smaa_anti_alias %}
        color = apply_smaa(color, coords);
    {% endif %}

    var rgb = color.rgb;

    {% if bloom %}
        rgb = apply_bloom(rgb);
    {% endif %}

    {% if dof %}
        rgb = apply_dof(rgb);
    {% endif %}

    rgb = linear_to_srgb(rgb);

    // Apply tone mapping to compress HDR to displayable range
    {% match tonemapping %}
        {% when ToneMapping::KhronosNeutralPbr %}
            rgb = khronos_pbr_neutral_tonemap(rgb);
        {% when ToneMapping::Aces %}
            rgb = aces_tonemap(rgb);
        {% when _ %}
    {% endmatch %}

    return vec4<f32>(rgb, color.a);
}
