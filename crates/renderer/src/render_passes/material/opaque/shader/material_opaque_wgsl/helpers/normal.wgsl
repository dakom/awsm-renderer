fn get_normal(
    triangle_indices: vec3<u32>,
    barycentric: vec3<f32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    pbr_material: PbrMaterial,
) -> vec3<f32> {
    var vertex_normal = get_vertex_normal(
        attribute_data_offset,
        triangle_indices,
        barycentric,
        vertex_attribute_stride,
    );

    // TODO - normal map?

    return vertex_normal;

}

fn get_vertex_normal(attribute_data_offset: u32, triangle_indices: vec3<u32>, barycentric: vec3<f32>, vertex_attribute_stride: u32) -> vec3<f32> {
    let n0 = _get_vertex_normal(attribute_data_offset, triangle_indices.x, vertex_attribute_stride);
    let n1 = _get_vertex_normal(attribute_data_offset, triangle_indices.y, vertex_attribute_stride);
    let n2 = _get_vertex_normal(attribute_data_offset, triangle_indices.z, vertex_attribute_stride);

    return barycentric.x * n0 + barycentric.y * n1 + barycentric.z * n2;
}

fn _get_vertex_normal(attribute_data_offset: u32, vertex_index: u32, vertex_attribute_stride: u32) -> vec3<f32> {
    // First get to the right vertex, THEN to the right normal within that vertex
    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);

    // normals are always the first attribute if they exist
    let index = vertex_start;
    return vec3<f32>(attribute_data[index], attribute_data[index + 1], attribute_data[index + 2]);
}
