use std::{borrow::Cow, collections::BTreeMap};

use gltf::{accessor::DataType, Semantic};

use crate::{
    buffer::helpers::{i16_to_i32_vec, u16_to_u32_vec},
    gltf::{
        buffers::accessor::accessor_to_bytes,
        error::{AwsmGltfError, Result},
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
            }
            DataType::I16 => {
                attribute_kind.force_data_size(4); // Update data size to 4 bytes (i32)
                Cow::Owned(i16_to_i32_vec(&bytes))
            }
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
    let mut per_vertex_attributes: Vec<(&MeshBufferVertexAttributeInfo, &Cow<'_, [u8]>)> =
        attribute_data.iter().collect();

    if per_vertex_attributes.is_empty() {
        return Ok(Vec::new());
    }

    // Determine vertex count (ensure all attributes have matching lengths).
    // This keeps silently truncated buffers from slipping through and causing
    // unpredictable out-of-bounds reads in the compute shaders later on.
    let vertex_count = per_vertex_attributes
        .iter()
        .map(|(attr_info, attr_data)| {
            let stride = attr_info.vertex_size();
            debug_assert!(stride > 0);
            (attr_data.len() / stride, attr_data.len() % stride)
        })
        .try_fold(None, |acc, (count, remainder)| {
            if remainder != 0 {
                return Err(());
            }
            match acc {
                None => Ok(Some(count)),
                Some(prev) if prev == count => Ok(Some(prev)),
                Some(_) => Err(()),
            }
        })
        .map_err(|_| {
            AwsmGltfError::AttributeData(
                "vertex attribute buffers do not share a common vertex count".to_string(),
            )
        })?
        .unwrap_or(0);

    for vertex_index in 0..vertex_count {
        for (attr_info, attr_data) in per_vertex_attributes.iter() {
            let stride = attr_info.vertex_size();
            let start = vertex_index * stride;
            let end = start + stride;
            // Copy one vertex worth of data per attribute, preserving the
            // attribute ordering enforced by the BTree map above.
            vertex_attribute_bytes.extend_from_slice(&attr_data[start..end]);
        }
    }

    Ok(per_vertex_attributes
        .into_iter()
        .map(|(attr_info, _)| (*attr_info))
        .collect())
}

pub(super) fn convert_attribute_kind(
    semantic: &gltf::Semantic,
    accessor: &gltf::Accessor<'_>,
) -> MeshBufferVertexAttributeInfo {
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
        },
    }
}
