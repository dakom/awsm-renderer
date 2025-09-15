use std::sync::LazyLock;

use awsm_renderer_core::buffers::BufferUsage;
use slotmap::Key;

use crate::{
    buffer::dynamic_uniform::DynamicUniformBuffer,
    materials::{MaterialKey, Materials},
    mesh::{
        morphs::{GeometryMorphKey, Morphs},
        skins::{SkinKey, Skins},
        AwsmMeshError, MeshKey,
    },
    transforms::{TransformKey, Transforms},
};

pub const GEOMETRY_MESH_META_BYTE_SIZE: usize = 40;
pub const GEOMETRY_MESH_META_BYTE_ALIGNMENT: usize = 256;

pub static GEOMETRY_BUFFER_USAGE: LazyLock<BufferUsage> =
    LazyLock::new(|| BufferUsage::new().with_copy_dst().with_uniform());

// See meta.wgsl for the corresponding struct
pub struct GeometryMeshMeta<'a> {
    pub mesh_key: MeshKey,
    pub transform_key: TransformKey,
    pub material_key: MaterialKey,
    pub geometry_morph_key: Option<GeometryMorphKey>,
    pub skin_key: Option<SkinKey>,
    pub materials: &'a Materials,
    pub transforms: &'a Transforms,
    pub morphs: &'a Morphs,
    pub skins: &'a Skins,
    pub material_meta_buffers: &'a DynamicUniformBuffer<MeshKey>,
}

impl<'a> GeometryMeshMeta<'a> {
    pub fn to_bytes(
        self,
    ) -> std::result::Result<[u8; GEOMETRY_MESH_META_BYTE_SIZE], AwsmMeshError> {
        let Self {
            mesh_key,
            transform_key,
            material_key,
            geometry_morph_key,
            skin_key,
            materials,
            transforms,
            morphs,
            skins,
            material_meta_buffers,
        } = self;

        let mut result = [0u8; GEOMETRY_MESH_META_BYTE_SIZE];
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

        // Morph (12 bytes)
        if let Some(morph_key) = geometry_morph_key {
            let info = morphs.geometry.get_info(morph_key)?;
            push_u32(info.targets_len as u32);
            push_u32(morphs.geometry.weights_buffer_offset(morph_key)? as u32);
            push_u32(morphs.geometry.values_buffer_offset(morph_key)? as u32);
        } else {
            push_u32(0);
            push_u32(0);
            push_u32(0);
        }

        // Skin (12 bytes)
        if let Some(skin_key) = skin_key {
            push_u32(skins.sets_len(skin_key)? as u32);
            push_u32(skins.joint_matrices_offset(skin_key)? as u32);
            push_u32(skins.joint_index_weights_offset(skin_key)? as u32);
        } else {
            push_u32(0);
            push_u32(0);
            push_u32(0);
        }

        // Transform (4 bytes)
        push_u32(transforms.buffer_offset(transform_key)? as u32);

        // Material Meta (4 bytes)
        push_u32(
            material_meta_buffers
                .offset(mesh_key)
                .ok_or(AwsmMeshError::MetaNotFound(mesh_key))? as u32,
        );

        Ok(result)
    }
}
