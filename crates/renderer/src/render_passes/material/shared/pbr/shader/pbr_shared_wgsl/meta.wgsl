
// make sure this matches MATERIAL_MESH_META_BYTE_ALIGNMENT in mesh.rs
const meta_size_in_bytes = 28u;

struct MaterialMeshMeta {
    mesh_key_high: u32,
    mesh_key_low: u32,
    morph_material_target_len: u32,
    morph_material_weights_offset: u32,
    morph_material_values_offset: u32,
    morph_material_bitmask: u32,
    material_offset: u32,
}
