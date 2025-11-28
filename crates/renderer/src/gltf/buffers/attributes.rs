use std::{borrow::Cow, collections::BTreeMap};

use gltf::{accessor::DataType, Semantic};

use crate::{
    buffer::helpers::{i16_to_i32_vec, u16_to_u32_vec},
    gltf::{
        buffers::accessor::accessor_to_bytes,
        error::{AwsmGltfError, Result},
    },
    mesh::{MeshBufferCustomVertexAttributeInfo, MeshBufferVertexAttributeInfo},
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
    attribute_data: Vec<(&MeshBufferCustomVertexAttributeInfo, &Cow<'_, [u8]>)>,
    vertex_attribute_bytes: &mut Vec<u8>,
) -> Result<()> {
    if attribute_data.is_empty() {
        return Ok(());
    }

    // Determine vertex count (ensure all attributes have matching lengths).
    // This keeps silently truncated buffers from slipping through and causing
    // unpredictable out-of-bounds reads in the compute shaders later on.
    let vertex_count = attribute_data
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
        for (attr_info, attr_data) in attribute_data.iter() {
            let stride = attr_info.vertex_size();
            let start = vertex_index * stride;
            let end = start + stride;
            // Copy one vertex worth of data per attribute, preserving the
            // attribute ordering enforced by the BTree map above.
            vertex_attribute_bytes.extend_from_slice(&attr_data[start..end]);
        }
    }

    Ok(())
}

pub(super) fn convert_attribute_kind(
    semantic: &gltf::Semantic,
    accessor: &gltf::Accessor<'_>,
) -> MeshBufferVertexAttributeInfo {
    use crate::mesh::{
        MeshBufferCustomVertexAttributeInfo, MeshBufferVisibilityVertexAttributeInfo,
    };

    match semantic {
        Semantic::Positions => MeshBufferVertexAttributeInfo::Visibility(
            MeshBufferVisibilityVertexAttributeInfo::Positions {
                data_size: accessor.data_type().size(),
                component_len: accessor.dimensions().multiplicity() as usize,
            },
        ),
        Semantic::Normals => MeshBufferVertexAttributeInfo::Visibility(
            MeshBufferVisibilityVertexAttributeInfo::Normals {
                data_size: accessor.data_type().size(),
                component_len: accessor.dimensions().multiplicity() as usize,
            },
        ),
        Semantic::Tangents => MeshBufferVertexAttributeInfo::Visibility(
            MeshBufferVisibilityVertexAttributeInfo::Tangents {
                data_size: accessor.data_type().size(),
                component_len: accessor.dimensions().multiplicity() as usize,
            },
        ),
        Semantic::Colors(index) => {
            MeshBufferVertexAttributeInfo::Custom(MeshBufferCustomVertexAttributeInfo::Colors {
                data_size: accessor.data_type().size(),
                component_len: accessor.dimensions().multiplicity() as usize,
                index: *index,
            })
        }
        Semantic::TexCoords(index) => {
            MeshBufferVertexAttributeInfo::Custom(MeshBufferCustomVertexAttributeInfo::TexCoords {
                data_size: accessor.data_type().size(),
                component_len: accessor.dimensions().multiplicity() as usize,
                index: *index,
            })
        }
        Semantic::Joints(index) => {
            MeshBufferVertexAttributeInfo::Custom(MeshBufferCustomVertexAttributeInfo::Joints {
                data_size: accessor.data_type().size(),
                component_len: accessor.dimensions().multiplicity() as usize,
                index: *index,
            })
        }
        Semantic::Weights(index) => {
            MeshBufferVertexAttributeInfo::Custom(MeshBufferCustomVertexAttributeInfo::Weights {
                data_size: accessor.data_type().size(),
                component_len: accessor.dimensions().multiplicity() as usize,
                index: *index,
            })
        }
    }
}
