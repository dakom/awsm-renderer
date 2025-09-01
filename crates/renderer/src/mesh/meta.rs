use slotmap::Key;

use crate::{
    materials::{MaterialKey, Materials},
    mesh::{
        morphs::{GeometryMorphKey, MaterialMorphKey, Morphs},
        skins::{SkinKey, Skins},
        AwsmMeshError, MeshKey,
    }, transforms::{TransformKey, Transforms},
};

pub const MESH_META_INITIAL_CAPACITY: usize = 1024;
pub const MESH_META_BYTE_SIZE: usize = 32; // 8 u32s
pub const MESH_META_BYTE_ALIGNMENT: usize = 256; // 32 bytes aligned
pub const MESH_META_MORPH_MATERIAL_BITMASK_NORMAL: u32 = 1;
pub const MESH_META_MORPH_MATERIAL_BITMASK_TANGENT: u32 = 1 << 1;

// See meta.wgsl for the corresponding struct
pub struct MeshMeta <'a> {
    pub mesh_key: MeshKey,
    pub transform_key: TransformKey,
    pub material_key: MaterialKey,
    pub geometry_morph_key: Option<GeometryMorphKey>,
    pub material_morph_key: Option<MaterialMorphKey>,
    pub skin_key: Option<SkinKey>,
    pub materials: &'a Materials,
    pub transforms: &'a Transforms,
    pub morphs: &'a Morphs,
    pub skins: &'a Skins,
}

impl <'a> MeshMeta<'a> {
    pub fn to_bytes(self) -> std::result::Result<[u8; 32], AwsmMeshError> {
        let Self { 
            mesh_key,
            transform_key,
            material_key, 
            geometry_morph_key, 
            material_morph_key, 
            skin_key, 
            materials,
            transforms,
            morphs, 
            skins 
        } = self;

        let mut result = [0u8; 32];
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
        } else {
            push_u32(0);
        }
        if let Some(morph_key) = material_morph_key {
            let info = morphs.material.get_info(morph_key)?;
            let mut bitmask = 0;
            if info.attributes.normal {
                bitmask |= MESH_META_MORPH_MATERIAL_BITMASK_NORMAL;
            }
            if info.attributes.tangent {
                bitmask |= MESH_META_MORPH_MATERIAL_BITMASK_TANGENT;
            }
            push_u32(info.targets_len as u32);
            push_u32(bitmask);
        } else {
            push_u32(0);
            push_u32(0);
        }

        // Skin (4 bytes)
        if let Some(skin_key) = skin_key {
            push_u32(skins.sets_len(skin_key)? as u32);
        } else {
            push_u32(0);
        }

        // Transform (4 bytes)
        push_u32(
            transforms
                .buffer_offset(transform_key)? as u32
        );


        // Material (4 bytes)
        push_u32(
            materials
                .buffer_offset(material_key)? as u32
        );

        Ok(result)
    }
}
