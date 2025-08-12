use std::{borrow::Cow, collections::HashMap};

use gltf::{accessor::DataType, Semantic};

use crate::{
    buffer::helpers::{i16_to_i32_vec, u16_to_u32_vec},
    gltf::{
        buffers::{accessor::accessor_to_bytes, MeshBufferVertexAttributeInfoWithOffset},
        error::Result,
    },
    mesh::MeshBufferVertexAttributeKind,
};

// Helper function to load attribute data (similar to your existing code)
pub(super) fn load_attribute_data_by_kind<'a>(
    gltf_attributes: &[(gltf::Semantic, gltf::Accessor<'_>)],
    buffers: &'a [Vec<u8>],
) -> Result<HashMap<MeshBufferVertexAttributeKind, Cow<'a, [u8]>>> {
    let mut attribute_data = HashMap::new();

    for (semantic, accessor) in gltf_attributes {
        let shader_key = semantic_to_shader_key(semantic);
        let bytes = accessor_to_bytes(accessor, buffers)?;

        // wgsl doesn't work with 16-bit, so we may need to convert to 32-bit
        let final_bytes = match accessor.data_type() {
            DataType::U16 => Cow::Owned(u16_to_u32_vec(&bytes)),
            DataType::I16 => Cow::Owned(i16_to_i32_vec(&bytes)),
            _ => bytes,
        };

        attribute_data.insert(shader_key, final_bytes);
    }

    Ok(attribute_data)
}

// Pack vertex attributes in original layout (for indexed access)
pub(super) fn pack_vertex_attributes(
    attribute_data: &HashMap<MeshBufferVertexAttributeKind, Cow<'_, [u8]>>,
    vertex_attribute_bytes: &mut Vec<u8>,
) -> Result<Vec<MeshBufferVertexAttributeInfoWithOffset>> {
    let mut vertex_attributes = Vec::new();
    let mut current_offset = 0;

    // Process each attribute (except positions, which are in visibility buffer)
    for (attr_kind, attr_data) in attribute_data.iter() {
        if *attr_kind == MeshBufferVertexAttributeKind::Positions {
            continue; // Skip positions
        }

        let components = get_attribute_components(attr_kind);
        let size_per_vertex = components as usize * 4; // everything is normalized to 32-bit (either u32 or f32)

        // Copy original vertex attribute data as-is
        vertex_attribute_bytes.extend_from_slice(attr_data);

        vertex_attributes.push(MeshBufferVertexAttributeInfoWithOffset {
            kind: *attr_kind,
            size_per_vertex,
            offset: current_offset,
            components,
        });

        current_offset += attr_data.len();
    }

    Ok(vertex_attributes)
}

pub(super) fn semantic_to_shader_key(semantic: &gltf::Semantic) -> MeshBufferVertexAttributeKind {
    match semantic {
        Semantic::Positions => MeshBufferVertexAttributeKind::Positions,
        Semantic::Normals => MeshBufferVertexAttributeKind::Normals,
        Semantic::Tangents => MeshBufferVertexAttributeKind::Tangents,
        Semantic::Colors(n) => MeshBufferVertexAttributeKind::Colors { count: *n },
        Semantic::TexCoords(n) => MeshBufferVertexAttributeKind::TexCoords { count: *n },
        Semantic::Joints(n) => MeshBufferVertexAttributeKind::Joints { count: *n },
        Semantic::Weights(n) => MeshBufferVertexAttributeKind::Weights { count: *n },
    }
}

pub(super) fn semantic_cmp(a: &gltf::Semantic, b: &gltf::Semantic) -> std::cmp::Ordering {
    fn level_1(semantic: &gltf::Semantic) -> usize {
        match semantic {
            gltf::Semantic::Positions => 1,
            gltf::Semantic::Normals => 2,
            gltf::Semantic::Tangents => 3,
            gltf::Semantic::Colors(_) => 4,
            gltf::Semantic::TexCoords(_) => 5,
            gltf::Semantic::Joints(_) => 6,
            gltf::Semantic::Weights(_) => 7,
        }
    }
    fn level_2(semantic: &gltf::Semantic) -> u32 {
        match semantic {
            gltf::Semantic::Positions => 0,
            gltf::Semantic::Normals => 0,
            gltf::Semantic::Tangents => 0,
            gltf::Semantic::Colors(n) => *n,
            gltf::Semantic::TexCoords(n) => *n,
            gltf::Semantic::Joints(n) => *n,
            gltf::Semantic::Weights(n) => *n,
        }
    }

    let a_level_1 = level_1(a);
    let b_level_1 = level_1(b);
    let a_level_2 = level_2(a);
    let b_level_2 = level_2(b);

    match a_level_1.cmp(&b_level_1) {
        std::cmp::Ordering::Equal => a_level_2.cmp(&b_level_2),
        other => other,
    }
}

pub(super) fn get_attribute_components(attr_kind: &MeshBufferVertexAttributeKind) -> u32 {
    match attr_kind {
        MeshBufferVertexAttributeKind::Positions => 3,
        MeshBufferVertexAttributeKind::Normals => 3,
        MeshBufferVertexAttributeKind::Tangents => 4, // vec4 (tangent + handedness)
        MeshBufferVertexAttributeKind::Colors { .. } => 4, // RGBA
        MeshBufferVertexAttributeKind::TexCoords { .. } => 2, // UV
        MeshBufferVertexAttributeKind::Joints { .. } => 4, // 4 joint indices
        MeshBufferVertexAttributeKind::Weights { .. } => 4, // 4 weights
    }
}
