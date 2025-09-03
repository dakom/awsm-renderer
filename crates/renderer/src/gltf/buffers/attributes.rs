use std::{borrow::Cow, collections::{BTreeMap, HashMap}};

use gltf::{accessor::DataType, Semantic};

use crate::{
    buffer::helpers::{i16_to_i32_vec, u16_to_u32_vec},
    gltf::{
        buffers::{accessor::accessor_to_bytes},
        error::Result,
    },
    mesh::MeshBufferVertexAttributeInfo,
};

// Helper function to load attribute data (similar to your existing code)
pub(super) fn load_attribute_data_by_kind<'a>(
    gltf_attributes: &[(gltf::Semantic, gltf::Accessor<'_>)],
    buffers: &'a [Vec<u8>],
) -> Result<BTreeMap<MeshBufferVertexAttributeInfo, Cow<'a, [u8]>>> {
    let mut attribute_data = BTreeMap::new();

    for (semantic, accessor) in gltf_attributes {
        let mut attribute_kind = convert_attribute_kind(semantic, accessor);
        let bytes = accessor_to_bytes(accessor, buffers)?;

        // wgsl doesn't work with 16-bit, so we may need to convert to 32-bit
        let final_bytes = match accessor.data_type() {
            DataType::U16 => {
                attribute_kind.force_data_size(4); // Update data size to 4 bytes (u32)
                Cow::Owned(u16_to_u32_vec(&bytes))
            },
            DataType::I16 => {
                attribute_kind.force_data_size(4); // Update data size to 4 bytes (i32)
                Cow::Owned(i16_to_i32_vec(&bytes))
            },
            _ => bytes,
        };

        attribute_data.insert(attribute_kind, final_bytes);
    }

    Ok(attribute_data)
}

// Pack vertex attributes in interleaved layout (for indexed access)
pub(super) fn pack_vertex_attributes(
    attribute_data: &BTreeMap<MeshBufferVertexAttributeInfo, Cow<'_, [u8]>>,
    vertex_attribute_bytes: &mut Vec<u8>,
) -> Result<Vec<MeshBufferVertexAttributeInfo>> {
    let mut offsets:HashMap<MeshBufferVertexAttributeInfo, usize> = attribute_data.keys().map(|k| (*k, 0)).collect();

    // Process each attribute (except positions, which are in visibility buffer)
    loop {
        let mut done = false;

        for (attr_info, attr_data) in attribute_data.iter() {
            if matches!(attr_info, MeshBufferVertexAttributeInfo::Positions{..}) {
                continue; // Skip positions
            }

            let offset = offsets.get_mut(attr_info).unwrap();
            let slice = &attr_data[*offset..*offset + attr_info.vertex_size()];
            *offset += attr_info.vertex_size();

            if *offset == attr_data.len() {
                done = true;
            }

            // Copy original vertex attribute data as-is
            vertex_attribute_bytes.extend_from_slice(slice);
        }

        if done {
            break;
        }
    }

    Ok(
        attribute_data
            .keys()
            .filter(|k| !matches!(k, MeshBufferVertexAttributeInfo::Positions{..}))
            .cloned()
            .collect()
    )
}

pub(super) fn convert_attribute_kind(semantic: &gltf::Semantic, accessor: &gltf::Accessor<'_>) -> MeshBufferVertexAttributeInfo {
    match semantic {
        Semantic::Positions => MeshBufferVertexAttributeInfo::Positions {
            data_size: accessor.data_type().size(),
            component_len: accessor.dimensions().multiplicity() as usize,
        },
        Semantic::Normals => MeshBufferVertexAttributeInfo::Normals {
            data_size: accessor.data_type().size(),
            component_len: accessor.dimensions().multiplicity() as usize,
        },
        Semantic::Tangents => MeshBufferVertexAttributeInfo::Tangents {
            data_size: accessor.data_type().size(),
            component_len: accessor.dimensions().multiplicity() as usize,
        },
        Semantic::Colors(count) => MeshBufferVertexAttributeInfo::Colors {
            data_size: accessor.data_type().size(),
            component_len: accessor.dimensions().multiplicity() as usize,
            count: *count,
        },
        Semantic::TexCoords(count) => MeshBufferVertexAttributeInfo::TexCoords {
            data_size: accessor.data_type().size(),
            component_len: accessor.dimensions().multiplicity() as usize,
            count: *count,
        },
        Semantic::Joints(count) => MeshBufferVertexAttributeInfo::Joints {
            data_size: accessor.data_type().size(),
            component_len: accessor.dimensions().multiplicity() as usize,
            count: *count,
        },
        Semantic::Weights(count) => MeshBufferVertexAttributeInfo::Weights {
            data_size: accessor.data_type().size(),
            component_len: accessor.dimensions().multiplicity() as usize,
            count: *count, 
        }
    }
}