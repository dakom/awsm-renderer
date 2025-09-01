@group(2) @binding(0)
var<uniform> mesh_meta: MeshMeta;

struct MeshMeta {
    mesh_key_high: u32,
    mesh_key_low: u32,
    morph_geometry_target_len: u32,
    morph_material_target_len: u32,
    morph_material_bitmask: u32,
    skin_sets_len: u32,
    transform_offset: u32,
    material_offset: u32,
}