use awsm_renderer_core::pipeline::primitive::FrontFace;
use awsm_renderer_core::pipeline::vertex::VertexFormat;
use gltf::accessor::{DataType, Dimensions};
use gltf::Semantic;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use crate::buffer::helpers::{
    i16_to_i32_vec, slice_zeroes, u16_to_u32_vec, u8_to_i16_vec, u8_to_u16_vec,
};
use crate::gltf::buffers::attributes::{load_attribute_data_by_kind, pack_vertex_attributes};
use crate::gltf::buffers::index::extract_triangle_indices;
use crate::gltf::buffers::morph::convert_morph_targets;
use crate::gltf::buffers::normals::{compute_normals, ensure_normals};
use crate::gltf::buffers::skin::convert_skin;
use crate::gltf::buffers::triangle::pack_triangle_data;
use crate::gltf::buffers::{
    MeshBufferAttributeIndexInfoWithOffset, MeshBufferInfoWithOffset,
    MeshBufferTriangleDataInfoWithOffset, MeshBufferTriangleInfoWithOffset,
    MeshBufferVertexInfoWithOffset,
};
use crate::gltf::error::AwsmGltfError;
use crate::mesh::{
    MeshBufferAttributeIndexInfo, MeshBufferInfo, MeshBufferTriangleDataInfo,
    MeshBufferTriangleInfo, MeshBufferVertexAttributeInfo, MeshBufferVertexInfo,
};

use super::accessor::accessor_to_bytes;
use super::Result;

pub(super) fn convert_to_visibility_buffer(
    primitive: &gltf::Primitive,
    front_face: FrontFace,
    buffers: &[Vec<u8>],
    vertex_attribute_index: &MeshBufferAttributeIndexInfoWithOffset,
    vertex_attribute_index_bytes: &[u8],
    visibility_vertex_bytes: &mut Vec<u8>,
    attribute_vertex_bytes: &mut Vec<u8>,
    triangle_data_bytes: &mut Vec<u8>,
    geometry_morph_bytes: &mut Vec<u8>,
    material_morph_bytes: &mut Vec<u8>,
    skin_joint_index_weight_bytes: &mut Vec<u8>,
) -> Result<MeshBufferInfoWithOffset> {
    // Step 1: Load all GLTF attributes
    let mut gltf_attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> =
        primitive.attributes().collect();

    // this should never be empty, but let's be safe
    let vertex_count = gltf_attributes
        .first()
        .map(|(_, accessor)| accessor.count())
        .unwrap_or(0);

    let triangle_count = vertex_attribute_index.count / 3;

    // Step 2: Load attribute data by kind
    let attribute_data_by_kind = load_attribute_data_by_kind(&gltf_attributes, buffers)?;

    // Step 3: Ensure normals exist (compute if missing)
    let attribute_data_by_kind = ensure_normals(
        attribute_data_by_kind,
        vertex_attribute_index,
        vertex_attribute_index_bytes,
    )?;

    // Step 4: Create visibility vertices (positions + triangle_index + barycentric)
    // These are expanded such that each vertex gets its own visibility vertex (triangle_index will be repeated for all 3)
    let visability_vertex_offset = visibility_vertex_bytes.len();
    create_visibility_vertices(
        &attribute_data_by_kind,
        vertex_attribute_index,
        vertex_attribute_index_bytes,
        triangle_count,
        visibility_vertex_bytes,
    )?;

    // Step 5: Pack vertex attributes
    // These are the original attributes per-vertex, excluding positions
    // There is no need to repack or expand these, they are used as-is
    let attribute_vertex_offset = attribute_vertex_bytes.len();
    let vertex_attributes =
        pack_vertex_attributes(&attribute_data_by_kind, attribute_vertex_bytes)?;

    // Step 6: Pack triangle data (vertex indices)
    let triangle_data_offset = triangle_data_bytes.len();
    let triangle_data_info = pack_triangle_data(
        vertex_attribute_index,
        vertex_attribute_index_bytes,
        triangle_count,
        triangle_data_offset,
        triangle_data_bytes,
        front_face,
        primitive.material().double_sided(),
    )?;

    // Step 7: Handle morph targets (if any)
    let (geometry_morph, material_morph) = convert_morph_targets(
        primitive,
        buffers,
        vertex_attribute_index,
        vertex_attribute_index_bytes,
        triangle_count,
        geometry_morph_bytes,
        material_morph_bytes,
    )?;

    // Step 8: Handle skin (if any)
    let skin = convert_skin(
        primitive,
        buffers,
        vertex_attribute_index,
        vertex_attribute_index_bytes,
        triangle_count,
        skin_joint_index_weight_bytes,
    )?;

    // Step 7: Build final MeshBufferInfo
    Ok(MeshBufferInfoWithOffset {
        vertex: MeshBufferVertexInfoWithOffset {
            offset: visability_vertex_offset,
            count: triangle_count * 3, // 3 vertices per triangle
        },
        triangles: MeshBufferTriangleInfoWithOffset {
            count: triangle_count,
            vertex_attribute_indices: vertex_attribute_index.clone(),
            vertex_attributes,
            vertex_attributes_offset: attribute_vertex_offset,
            vertex_attributes_size: attribute_vertex_bytes.len() - attribute_vertex_offset,
            triangle_data: triangle_data_info,
        },
        geometry_morph,
        material_morph,
        skin,
    })
}

fn create_visibility_vertices(
    attribute_data: &BTreeMap<MeshBufferVertexAttributeInfo, Cow<'_, [u8]>>,
    index: &MeshBufferAttributeIndexInfoWithOffset,
    index_bytes: &[u8],
    triangle_count: usize,
    visibility_vertex_bytes: &mut Vec<u8>,
) -> Result<()> {
    static BARYCENTRICS: [[f32; 2]; 3] = [
        [1.0, 0.0], // First vertex: (1, 0, 0) - z = 1-1-0 = 0
        [0.0, 1.0], // Second vertex: (0, 1, 0) - z = 1-0-1 = 0
        [0.0, 0.0], // Third vertex: (0, 0, 1) - z = 1-0-0 = 1
    ];
    // Get positions data
    let positions = attribute_data
        .iter()
        .find_map(|(attr_info, data)| match attr_info {
            MeshBufferVertexAttributeInfo::Positions { .. } => Some(&data[..]),
            _ => None,
        })
        .ok_or_else(|| AwsmGltfError::Positions("missing positions".to_string()))?;

    // Validate positions buffer (must be Float32x3 format)
    if positions.len() % 12 != 0 {
        return Err(AwsmGltfError::Positions(format!(
            "Position buffer length ({}) is not a multiple of 12 (3 * f32).",
            positions.len()
        )));
    }

    // Extract all triangle indices at once
    let triangle_indices = extract_triangle_indices(index, index_bytes)?;

    // Process each triangle
    for (triangle_index, triangle) in triangle_indices.iter().enumerate() {
        // Create 3 visibility vertices for this triangle
        for (vertex_in_triangle, &vertex_index) in triangle.iter().enumerate() {
            // Get position for this vertex
            let position = get_position_from_buffer(&positions, vertex_index)?;

            // Write vertex data: position (12 bytes) + triangle_index (4 bytes) + barycentric (8 bytes)

            // Position (12 bytes)
            visibility_vertex_bytes.extend_from_slice(&position[0].to_le_bytes());
            visibility_vertex_bytes.extend_from_slice(&position[1].to_le_bytes());
            visibility_vertex_bytes.extend_from_slice(&position[2].to_le_bytes());

            // Triangle index (4 bytes)
            visibility_vertex_bytes.extend_from_slice(&(triangle_index as u32).to_le_bytes());

            // Barycentric coordinates (8 bytes)
            let bary = BARYCENTRICS[vertex_in_triangle];
            visibility_vertex_bytes.extend_from_slice(&bary[0].to_le_bytes());
            visibility_vertex_bytes.extend_from_slice(&bary[1].to_le_bytes());
        }
    }

    Ok(())
}

fn get_position_from_buffer(positions: &[u8], vertex_index: usize) -> Result<[f32; 3]> {
    let offset = vertex_index * 12; // 3 f32s = 12 bytes

    let vertex_count = positions.len() / 12;
    if vertex_index >= vertex_count {
        return Err(AwsmGltfError::Positions(format!(
            "Position data out of bounds for vertex {}. Buffer has {} vertices ({} bytes), requested vertex {}", 
            vertex_index, vertex_count, positions.len(), vertex_index
        )));
    }

    if offset + 12 > positions.len() {
        return Err(AwsmGltfError::Positions(format!(
            "Position data out of bounds for vertex {}. Offset {} + 12 > buffer size {}",
            vertex_index,
            offset,
            positions.len()
        )));
    }

    // From spec:
    // "All buffer data defined in this specification (i.e., geometry attributes, geometry indices, sparse accessor data, animation inputs and outputs, inverse bind matrices)
    // MUST use little endian byte order."
    let x = f32::from_le_bytes([
        positions[offset],
        positions[offset + 1],
        positions[offset + 2],
        positions[offset + 3],
    ]);
    let y = f32::from_le_bytes([
        positions[offset + 4],
        positions[offset + 5],
        positions[offset + 6],
        positions[offset + 7],
    ]);
    let z = f32::from_le_bytes([
        positions[offset + 8],
        positions[offset + 9],
        positions[offset + 10],
        positions[offset + 11],
    ]);

    Ok([x, y, z])
}
