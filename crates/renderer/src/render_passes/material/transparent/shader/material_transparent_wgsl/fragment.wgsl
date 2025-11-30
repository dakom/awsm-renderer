// Fragment input from vertex shader
struct FragmentInput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,     // Transformed world-space normal
    @location(1) world_tangent: vec4<f32>,    // Transformed world-space tangent (w = handedness)
    {% for i in 0..color_sets %}
        @location({{ in_color_set_start + i }}) color_{{ i }}: vec2<f32>,
    {% endfor %}

    {% for i in 0..uv_sets %}
        @location({{ in_uv_set_start + i }}) uv_{{ i }}: vec2<f32>,
    {% endfor %}
}

struct FragmentOutput {
    // Rgba16float
    @location(0) oit_color: vec4<f32>,
}

@fragment
fn fs_main(input: FragmentInput) -> FragmentOutput {
    var out: FragmentOutput;

    // 1. Get material from mesh_meta
    let material_offset = mesh_meta.material_offset;
    let material = pbr_get_material(material_offset);

    // 2. For now, just sample the base color texture if it exists
    var base_color = material.base_color_factor;
    if (material.has_base_color_texture) {
        let tex_color = texture_pool_sample(material.base_color_tex_info, input.uv_0);
        base_color *= tex_color;
    }

    out.oit_color = vec4<f32>(base_color.rgb, base_color.a);

    return out;
}
