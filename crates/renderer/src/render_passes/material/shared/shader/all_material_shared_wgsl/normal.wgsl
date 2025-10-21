// Get the interpolated geometry normal (vertex normal) in world space.
// NOTE: This is the geometry normal, NOT the normal-mapped normal.
// Normal mapping is applied separately in _pbr_normal_color() which returns material_color.normal
fn get_world_normal(
    triangle_indices: vec3<u32>,
    barycentric: vec3<f32>,
    attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    pbr_material: PbrMaterial,
    normal_matrix: mat3x3<f32>,
) -> vec3<f32> {
    var vertex_normal = get_vertex_normal(
        attribute_data_offset,
        triangle_indices,
        barycentric,
        vertex_attribute_stride,
    );

    return safe_normalize(normal_matrix * vertex_normal);
}

fn get_vertex_normal(attribute_data_offset: u32, triangle_indices: vec3<u32>, barycentric: vec3<f32>, vertex_attribute_stride: u32) -> vec3<f32> {
    let n0 = _get_vertex_normal(attribute_data_offset, triangle_indices.x, vertex_attribute_stride);
    let n1 = _get_vertex_normal(attribute_data_offset, triangle_indices.y, vertex_attribute_stride);
    let n2 = _get_vertex_normal(attribute_data_offset, triangle_indices.z, vertex_attribute_stride);

    return barycentric.x * n0 + barycentric.y * n1 + barycentric.z * n2;
}

// Read normal from packed attribute buffer
// Attribute layout per vertex: [normal.xyz (3 floats), tangent.xyzw (4 floats), ...]
fn _get_vertex_normal(attribute_data_offset: u32, vertex_index: u32, vertex_attribute_stride: u32) -> vec3<f32> {
    let vertex_start = attribute_data_offset + (vertex_index * vertex_attribute_stride);
    let index = vertex_start; // normals are first 3 floats in the attribute data
    return vec3<f32>(attribute_data[index], attribute_data[index + 1], attribute_data[index + 2]);
}

fn safe_normalize(normal: vec3<f32>) -> vec3<f32> {
    let len_sq = dot(normal, normal);
    if (len_sq > 0.0) {
        return normal * inverseSqrt(len_sq);
    }
    // fallback: up vector to avoid NaNs; scene lighting expects unit normal
    return vec3<f32>(0.0, 0.0, 1.0);
}
