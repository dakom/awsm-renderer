fn get_triangle_indices(attribute_indices_offset: u32, triangle_id: u32) -> vec3<u32> {
    let base_index = attribute_indices_offset + (triangle_id * 3u);
    let v0_index = attribute_indices[base_index];
    let v1_index = attribute_indices[base_index + 1];
    let v2_index = attribute_indices[base_index + 2];
    return vec3<u32>(v0_index, v1_index, v2_index);
}

fn base_color_tex_uv(attribute_data_offset: u32, triangle_indices: vec3<u32>, barycentric: vec3<f32>, tex_info: TextureInfo, vertex_attribute_stride: u32) -> vec2<f32> {
    let uv0 = get_uv(attribute_data_offset, tex_info.attribute_uv_index, triangle_indices.x, vertex_attribute_stride);
    let uv1 = get_uv(attribute_data_offset, tex_info.attribute_uv_index, triangle_indices.y, vertex_attribute_stride);
    let uv2 = get_uv(attribute_data_offset, tex_info.attribute_uv_index, triangle_indices.z, vertex_attribute_stride);

    let interpolated_uv = barycentric.x * uv0 + barycentric.y * uv1 + barycentric.z * uv2;

    return interpolated_uv;
}
