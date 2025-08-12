use slotmap::Key;

use crate::{
    materials::{MaterialKey, Materials},
    mesh::{
        morphs::Morphs,
        morphs::{GeometryMorphKey, MaterialMorphKey},
        skins::{SkinKey, Skins},
        AwsmMeshError, MeshKey,
    },
};

pub const MESH_META_INITIAL_CAPACITY: usize = 1024;
pub const MESH_META_BYTE_SIZE: usize = 32; // 8 u32s
pub const MESH_META_BYTE_ALIGNMENT: usize = 256; // 32 bytes aligned
pub const MESH_META_MORPH_MATERIAL_BITMASK_NORMAL: u32 = 1;
pub const MESH_META_MORPH_MATERIAL_BITMASK_TANGENT: u32 = 1 << 1;

#[allow(clippy::too_many_arguments)]
pub fn mesh_meta_data(
    mesh_key: MeshKey,
    material_key: MaterialKey,
    geometry_morph_key: Option<GeometryMorphKey>,
    material_morph_key: Option<MaterialMorphKey>,
    skin_key: Option<SkinKey>,
    materials: &Materials,
    morphs: &Morphs,
    skins: &Skins,
) -> std::result::Result<[u8; 32], AwsmMeshError> {
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

    push_u32(mesh_key_u32_high);
    push_u32(mesh_key_u32_low);
    push_u32(
        materials
            .buffer_offset(material_key)
            .ok_or(AwsmMeshError::MaterialNotFound(material_key))? as u32,
    );
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
    if let Some(skin_key) = skin_key {
        push_u32(skins.joint_len(skin_key)? as u32);
    } else {
        push_u32(0);
    }

    Ok(result)
}
