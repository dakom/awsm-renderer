
// make sure this matches MATERIAL_MESH_META_BYTE_ALIGNMENT in material_opaque_meta.rs
const META_SIZE_IN_BYTES = 64u;

struct MeshMeta {
    mesh_key_high: u32,
    mesh_key_low: u32,
    morph_material_target_len: u32,
    morph_material_weights_offset: u32,
    morph_material_values_offset: u32,
    morph_material_bitmask: u32,
    material_offset: u32,
    transform_offset: u32,
    normal_matrix_offset: u32,
    vertex_attribute_indices_offset: u32,
    vertex_attribute_data_offset: u32,
    vertex_attribute_stride: u32,
    uv_sets_index: u32,
    uv_set_count: u32,
    color_set_count: u32,
    visibility_data_offset: u32,
}
