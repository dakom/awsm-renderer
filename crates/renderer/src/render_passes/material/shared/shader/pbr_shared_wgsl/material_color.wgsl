
struct PbrMaterialColor {
    base: vec4<f32>,
    metallic_roughness: vec2<f32>,
    normal: vec3<f32>,
    occlusion: f32,
    emissive: vec3<f32>,
};

fn pbr_get_material_color(attribute_indices_offset: u32, attribute_data_offset: u32, triangle_index: u32, material: PbrMaterial, barycentric: vec3<f32>, vertex_attribute_stride: u32) -> PbrMaterialColor {
    // get the vertex indices for this triangle
    let base_triangle_index = attribute_indices_offset + (triangle_index * 3u);
    let triangle_indices = vec3<u32>(attribute_indices[base_triangle_index], attribute_indices[base_triangle_index + 1], attribute_indices[base_triangle_index + 2]);

    let base = _pbr_material_base_color(material, texture_uv(attribute_data_offset, triangle_indices, barycentric, material.base_color_tex_info, vertex_attribute_stride));
    let emissive = _pbr_material_emissive_color(material, texture_uv(attribute_data_offset, triangle_indices, barycentric, material.emissive_tex_info, vertex_attribute_stride));

    return PbrMaterialColor(
        base,
        vec2<f32>(material.metallic_factor, material.roughness_factor),
        vec3<f32>(0.0, 0.0, 1.0), // TODO: normal
        1.0,                     // TODO: occlusion
        emissive,
    );
}
// Base Color
fn _pbr_material_base_color(material: PbrMaterial, attribute_uv: vec2<f32>) -> vec4<f32> {
    var color = material.base_color_factor;
    if material.has_base_color_texture {
        color *= texture_load_atlas_srgb(material.base_color_tex_info, attribute_uv);
    }

    // alpha_mode: 0=opaque, 1=mask, 2=blend
    if material.alpha_mode == 0u {
        color.a = 1.0;
    }


    return color;
}

fn _pbr_material_emissive_color(material: PbrMaterial, attribute_uv: vec2<f32>) -> vec3<f32> {
    var color = material.emissive_factor;
    if material.has_emissive_texture {
        color *= texture_load_atlas_srgb(material.emissive_tex_info, attribute_uv).rgb;
    }
    return color;
}
