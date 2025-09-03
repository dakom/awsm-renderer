fn get_triangle_indices(triangle_id: u32) -> vec3<u32> {
    let base_index = triangle_id * 3u;
    let v0_index = attribute_indices[base_index];
    let v1_index = attribute_indices[base_index + 1];
    let v2_index = attribute_indices[base_index + 2];
    return vec3<u32>(v0_index, v1_index, v2_index);
}

fn base_color_tex_uv(triangle_indices: vec3<u32>, barycentric: vec3<f32>, tex_info: TextureInfo) -> vec2<f32> {
    let uv0 = get_uv(triangle_indices.x);
    let uv1 = get_uv(triangle_indices.y);
    let uv2 = get_uv(triangle_indices.z);

    let interpolated_uv = barycentric.x * uv0 + barycentric.y * uv1 + barycentric.z * uv2;

    return interpolated_uv;
}

fn get_uv(index: u32) -> vec2<f32> {
    // TODO - needs to be dynamic based on attribute presence
    let index_0 = 3u + (index * 5u);
    return vec2<f32>(attribute_data[index_0], attribute_data[index_0 + 1]);
}