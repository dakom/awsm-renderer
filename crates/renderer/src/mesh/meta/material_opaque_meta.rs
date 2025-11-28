use std::sync::LazyLock;

use awsm_renderer_core::buffers::BufferUsage;
use slotmap::Key;

use crate::{
    buffer::dynamic_storage::DynamicStorageBuffer,
    materials::{MaterialKey, Materials},
    mesh::{
        morphs::{MaterialMorphKey, Morphs},
        AwsmMeshError, MeshBufferInfo, MeshKey,
    },
};

pub const MATERIAL_MESH_META_MORPH_MATERIAL_BITMASK_NORMAL: u32 = 1;
pub const MATERIAL_MESH_META_MORPH_MATERIAL_BITMASK_TANGENT: u32 = 1 << 1;
pub const MATERIAL_MESH_META_BYTE_SIZE: usize = 64;
pub const MATERIAL_MESH_META_BYTE_ALIGNMENT: usize = MATERIAL_MESH_META_BYTE_SIZE; // storage buffer is less strict

pub static MATERIAL_BUFFER_USAGE: LazyLock<BufferUsage> =
    LazyLock::new(|| BufferUsage::new().with_copy_dst().with_storage());

// See meta.wgsl for the corresponding struct
pub struct MaterialMeshMeta<'a> {
    pub mesh_key: MeshKey,
    pub material_key: MaterialKey,
    pub material_morph_key: Option<MaterialMorphKey>,
    pub custom_attribute_indices_offset: usize,
    pub custom_attribute_data_offset: usize,
    pub geometry_data_offset: usize,
    pub transform_offset: usize,
    pub normal_matrix_offset: usize,
    pub buffer_info: &'a MeshBufferInfo,
    pub materials: &'a Materials,
    pub morphs: &'a Morphs,
}

/// Calculate the offset (in floats) to TEXCOORD_0 within the vertex attribute data.
/// This accounts for any COLOR_n attributes that come before texture coordinates.
fn calculate_uv_sets_index(buffer_info: &MeshBufferInfo) -> u32 {
    use crate::mesh::buffer_info::{
        MeshBufferCustomVertexAttributeInfo, MeshBufferVertexAttributeInfo,
    };

    let mut offset_floats = 0;
    for attr in &buffer_info.triangles.vertex_attributes {
        match attr {
            MeshBufferVertexAttributeInfo::Custom(custom) => match custom {
                MeshBufferCustomVertexAttributeInfo::Colors { .. } => {
                    // vertex_size() returns bytes, divide by 4 to get float count
                    offset_floats += attr.vertex_size() / 4;
                }
                MeshBufferCustomVertexAttributeInfo::TexCoords { .. } => {
                    // Found TexCoords, stop counting
                    break;
                }
                _ => {}
            },
            _ => {}
        }
    }
    offset_floats as u32
}

/// Calculate how many UV sets and color sets this mesh has.
/// Returns (uv_set_count, color_set_count).
fn calculate_attribute_counts(buffer_info: &MeshBufferInfo) -> (u32, u32) {
    use crate::mesh::buffer_info::{
        MeshBufferCustomVertexAttributeInfo, MeshBufferVertexAttributeInfo,
    };

    let mut uv_set_count = 0u32;
    let mut color_set_count = 0u32;

    for attr in &buffer_info.triangles.vertex_attributes {
        match attr {
            MeshBufferVertexAttributeInfo::Custom(custom) => match custom {
                MeshBufferCustomVertexAttributeInfo::TexCoords { index, .. } => {
                    uv_set_count = uv_set_count.max(*index + 1);
                }
                MeshBufferCustomVertexAttributeInfo::Colors { index, .. } => {
                    color_set_count = color_set_count.max(*index + 1);
                }
                _ => {}
            },
            _ => {}
        }
    }

    (uv_set_count, color_set_count)
}

impl<'a> MaterialMeshMeta<'a> {
    pub fn to_bytes(
        self,
    ) -> std::result::Result<[u8; MATERIAL_MESH_META_BYTE_SIZE], AwsmMeshError> {
        let Self {
            mesh_key,
            material_key,
            material_morph_key,
            buffer_info,
            custom_attribute_indices_offset,
            custom_attribute_data_offset,
            geometry_data_offset,
            transform_offset,
            normal_matrix_offset,
            materials,
            morphs,
        } = self;

        let mut result = [0u8; MATERIAL_MESH_META_BYTE_SIZE];
        let mut offset = 0;

        let mut push_u32 = |value: u32| {
            result[offset..offset + 4].copy_from_slice(&value.to_le_bytes());

            offset += 4;
        };

        fn bool_as_u32(value: bool) -> u32 {
            if value {
                1
            } else {
                0
            }
        }

        let mesh_key_u64 = mesh_key.data().as_ffi();
        let (mesh_key_u32_high, mesh_key_u32_low) = (
            (mesh_key_u64 >> 32) as u32,
            (mesh_key_u64 & 0xFFFFFFFF) as u32,
        );

        // Mesh Key (8 bytes)
        push_u32(mesh_key_u32_high);
        push_u32(mesh_key_u32_low);

        // Morph (20 bytes)
        if let Some(morph_key) = material_morph_key {
            let info = morphs.material.get_info(morph_key)?;
            push_u32(info.targets_len as u32);
            push_u32(morphs.material.weights_buffer_offset(morph_key)? as u32);
            push_u32(morphs.material.values_buffer_offset(morph_key)? as u32);
            let mut bitmask = 0;
            if info.attributes.normal {
                bitmask |= MATERIAL_MESH_META_MORPH_MATERIAL_BITMASK_NORMAL;
            }
            if info.attributes.tangent {
                bitmask |= MATERIAL_MESH_META_MORPH_MATERIAL_BITMASK_TANGENT;
            }
            push_u32(bitmask);
        } else {
            push_u32(0);
            push_u32(0);
            push_u32(0);
            push_u32(0);
        }

        // Material (4 bytes)
        push_u32(materials.buffer_offset(material_key)? as u32);

        // Transform offset (4 bytes)
        push_u32(transform_offset as u32);
        // Normal matrix offset (4 bytes)
        push_u32(normal_matrix_offset as u32);

        // Vertex attribute offsets (8 bytes)
        push_u32(custom_attribute_indices_offset as u32);
        push_u32(custom_attribute_data_offset as u32);

        // Vertex attribute stride (4 bytes)
        push_u32(buffer_info.triangles.vertex_attribute_stride() as u32);

        // UV sets index - offset in floats to TEXCOORD_0 within vertex attribute data (4 bytes)
        let uv_sets_index = calculate_uv_sets_index(buffer_info);
        push_u32(uv_sets_index);

        // UV set count and color set count (8 bytes)
        let (uv_set_count, color_set_count) = calculate_attribute_counts(buffer_info);
        push_u32(uv_set_count);
        push_u32(color_set_count);

        // Geometry data offset (4 bytes)
        push_u32(geometry_data_offset as u32);

        Ok(result)
    }
}
