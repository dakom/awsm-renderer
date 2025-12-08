fn vertex_color(attribute_data_offset: u32, triangle_indices: vec3<u32>, barycentric: vec3<f32>, color_info: ColorInfo, vertex_attribute_stride: u32) -> vec4<f32> {
    let color0 = _vertex_color_per_vertex(attribute_data_offset, color_info.set_index, triangle_indices.x, vertex_attribute_stride);
    let color1 = _vertex_color_per_vertex(attribute_data_offset, color_info.set_index, triangle_indices.y, vertex_attribute_stride);
    let color2 = _vertex_color_per_vertex(attribute_data_offset, color_info.set_index, triangle_indices.z, vertex_attribute_stride);

    let interpolated_color = barycentric.x * color0 + barycentric.y * color1 + barycentric.z * color2;

    return interpolated_color;
}

fn _vertex_color_per_vertex(attribute_data_offset: u32, set_index: u32, vertex_index: u32, vertex_attribute_stride: u32) -> vec4<f32> {
    // First get to the right vertex, THEN to the right color set within that vertex
    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);
    // `color_sets_index` points to the beginning of COLOR_0 inside the packed stream.
    // Each additional color set contributes 4 more floats per vertex.
    let color_offset = {{ color_sets_index }}u + (set_index * 4u);
    let index = vertex_start + color_offset;
    let color = vec4<f32>(attribute_data[index], attribute_data[index + 1], attribute_data[index + 2], attribute_data[index + 3]);

    return color;
}
