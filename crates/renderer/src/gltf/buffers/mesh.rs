mod transparency;
mod visibility;

use awsm_renderer_core::pipeline::primitive::FrontFace;
use awsm_renderer_core::pipeline::vertex::VertexFormat;
use gltf::accessor::{DataType, Dimensions};
use gltf::material::AlphaMode;
use gltf::Semantic;
use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap};

use crate::buffer::helpers::{
    i16_to_i32_vec, slice_zeroes, u16_to_u32_vec, u8_to_i16_vec, u8_to_u16_vec,
};
use crate::gltf::buffers::attributes::{load_attribute_data_by_kind, pack_vertex_attributes};
use crate::gltf::buffers::index::extract_triangle_indices;
use crate::gltf::buffers::mesh::transparency::create_transparency_vertices;
use crate::gltf::buffers::mesh::visibility::create_visibility_vertices;
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

pub(super) enum GltfMeshBufferGeometryKind {
    Visibility,
    Transparency,
    Both,
}

// in theory a primitive could have both opaque and transparent materials via an extension which allows multiple materials
pub(super) fn mesh_buffer_geometry_kind(primitive: &gltf::Primitive) -> GltfMeshBufferGeometryKind {
    match primitive.material().alpha_mode() {
        AlphaMode::Opaque => GltfMeshBufferGeometryKind::Visibility,
        AlphaMode::Mask => GltfMeshBufferGeometryKind::Transparency,
        AlphaMode::Blend => GltfMeshBufferGeometryKind::Transparency,
    }
}

pub(super) fn convert_to_mesh_buffer(
    primitive: &gltf::Primitive,
    geometry_kind: GltfMeshBufferGeometryKind,
    front_face: FrontFace,
    buffers: &[Vec<u8>],
    custom_attribute_index: &MeshBufferAttributeIndexInfoWithOffset,
    custom_attribute_index_bytes: &[u8],
    visibility_geometry_vertex_bytes: &mut Vec<u8>,
    transparency_geometry_vertex_bytes: &mut Vec<u8>,
    custom_attribute_vertex_bytes: &mut Vec<u8>,
    triangle_data_bytes: &mut Vec<u8>,
    geometry_morph_bytes: &mut Vec<u8>,
    material_morph_bytes: &mut Vec<u8>,
    skin_joint_index_weight_bytes: &mut Vec<u8>,
) -> Result<MeshBufferInfoWithOffset> {
    // Step 1: Load all GLTF attributes
    let mut gltf_attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> = primitive
        .attributes()
        .filter(|(semantic, _)| {
            // Joints and Weights are NOT vertex attributes - they're skinning data
            // Handled separately by convert_skin(), never enter the attribute system
            !matches!(
                semantic,
                gltf::Semantic::Joints(_) | gltf::Semantic::Weights(_)
            )
        })
        .collect();

    // this should never be empty, but let's be safe
    let vertex_count = gltf_attributes
        .first()
        .map(|(_, accessor)| accessor.count())
        .unwrap_or(0);

    let triangle_count = custom_attribute_index.count / 3;

    // Step 2: Load attribute data by kind
    let attribute_data_by_kind = load_attribute_data_by_kind(&gltf_attributes, buffers)?;

    // Step 3: Ensure normals exist (compute if missing)
    let attribute_data_by_kind = ensure_normals(
        attribute_data_by_kind,
        custom_attribute_index,
        custom_attribute_index_bytes,
    )?;

    // Step 4: Create visibility vertices (positions + triangle_index + barycentric)
    // These are expanded such that each vertex gets its own visibility vertex (triangle_index will be repeated for all 3)
    let visability_vertex_offset = match geometry_kind {
        GltfMeshBufferGeometryKind::Visibility | GltfMeshBufferGeometryKind::Both => {
            let offset = visibility_geometry_vertex_bytes.len();
            create_visibility_vertices(
                &attribute_data_by_kind,
                custom_attribute_index,
                custom_attribute_index_bytes,
                triangle_count,
                front_face,
                visibility_geometry_vertex_bytes,
            )?;
            Some(offset)
        }

        GltfMeshBufferGeometryKind::Transparency => None,
    };

    let transparency_vertex_offset = match geometry_kind {
        GltfMeshBufferGeometryKind::Transparency | GltfMeshBufferGeometryKind::Both => {
            let offset = transparency_geometry_vertex_bytes.len();
            create_transparency_vertices(
                &attribute_data_by_kind,
                custom_attribute_index,
                custom_attribute_index_bytes,
                triangle_count,
                front_face,
                transparency_geometry_vertex_bytes,
            )?;
            Some(offset)
        }

        GltfMeshBufferGeometryKind::Visibility => None,
    };

    // Step 5: Pack vertex attributes
    // These are the original attributes per-vertex, but only non-visibility ones
    // There is no need to repack or expand these, they are used as-is
    let attribute_vertex_offset = custom_attribute_vertex_bytes.len();
    pack_vertex_attributes(
        attribute_data_by_kind
            .iter()
            .filter_map(|x| match x.0 {
                MeshBufferVertexAttributeInfo::Custom(custom) => Some((custom, x.1)),
                _ => None,
            })
            .collect(),
        custom_attribute_vertex_bytes,
    )?;

    // Step 6: Pack triangle data (vertex indices)
    let triangle_data_offset = triangle_data_bytes.len();
    let triangle_data_info = pack_triangle_data(
        custom_attribute_index,
        custom_attribute_index_bytes,
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
        custom_attribute_index,
        custom_attribute_index_bytes,
        triangle_count,
        geometry_morph_bytes,
        material_morph_bytes,
    )?;

    // Step 8: Handle skin (if any)
    let skin = convert_skin(
        primitive,
        buffers,
        custom_attribute_index,
        custom_attribute_index_bytes,
        triangle_count,
        skin_joint_index_weight_bytes,
    )?;

    // Step 7: Build final MeshBufferInfo
    Ok(MeshBufferInfoWithOffset {
        visibility_geometry_vertex: visability_vertex_offset.map(|offset| {
            MeshBufferVertexInfoWithOffset {
                offset,
                count: triangle_count * 3, // 3 vertices per triangle (i.e. exploded)
            }
        }),
        transparency_geometry_vertex: transparency_vertex_offset.map(|offset| {
            MeshBufferVertexInfoWithOffset {
                offset,
                count: vertex_count, // original vertex count
            }
        }),
        triangles: MeshBufferTriangleInfoWithOffset {
            count: triangle_count,
            vertex_attribute_indices: custom_attribute_index.clone(),
            vertex_attributes: attribute_data_by_kind
                .keys()
                .filter(|attr| attr.is_custom_attribute())
                .cloned()
                .collect(),
            vertex_attributes_offset: attribute_vertex_offset,
            vertex_attributes_size: custom_attribute_vertex_bytes.len() - attribute_vertex_offset,
            triangle_data: triangle_data_info,
        },
        geometry_morph,
        material_morph,
        skin,
    })
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

fn get_vec3_from_buffer(buffer: &[u8], vertex_index: usize, name: &str) -> Result<[f32; 3]> {
    let offset = vertex_index * 12; // 3 f32s = 12 bytes

    let vertex_count = buffer.len() / 12;
    if vertex_index >= vertex_count {
        return Err(AwsmGltfError::AttributeData(format!(
            "{} data out of bounds for vertex {}. Buffer has {} vertices ({} bytes), requested vertex {}",
            name, vertex_index, vertex_count, buffer.len(), vertex_index
        )));
    }

    if offset + 12 > buffer.len() {
        return Err(AwsmGltfError::AttributeData(format!(
            "{} data out of bounds for vertex {}. Offset {} + 12 > buffer size {}",
            name,
            vertex_index,
            offset,
            buffer.len()
        )));
    }

    let x = f32::from_le_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ]);
    let y = f32::from_le_bytes([
        buffer[offset + 4],
        buffer[offset + 5],
        buffer[offset + 6],
        buffer[offset + 7],
    ]);
    let z = f32::from_le_bytes([
        buffer[offset + 8],
        buffer[offset + 9],
        buffer[offset + 10],
        buffer[offset + 11],
    ]);

    Ok([x, y, z])
}

fn get_vec4_from_buffer(buffer: &[u8], vertex_index: usize, name: &str) -> Result<[f32; 4]> {
    let offset = vertex_index * 16; // 4 f32s = 16 bytes

    let vertex_count = buffer.len() / 16;
    if vertex_index >= vertex_count {
        return Err(AwsmGltfError::AttributeData(format!(
            "{} data out of bounds for vertex {}. Buffer has {} vertices ({} bytes), requested vertex {}",
            name, vertex_index, vertex_count, buffer.len(), vertex_index
        )));
    }

    if offset + 16 > buffer.len() {
        return Err(AwsmGltfError::AttributeData(format!(
            "{} data out of bounds for vertex {}. Offset {} + 16 > buffer size {}",
            name,
            vertex_index,
            offset,
            buffer.len()
        )));
    }

    let x = f32::from_le_bytes([
        buffer[offset],
        buffer[offset + 1],
        buffer[offset + 2],
        buffer[offset + 3],
    ]);
    let y = f32::from_le_bytes([
        buffer[offset + 4],
        buffer[offset + 5],
        buffer[offset + 6],
        buffer[offset + 7],
    ]);
    let z = f32::from_le_bytes([
        buffer[offset + 8],
        buffer[offset + 9],
        buffer[offset + 10],
        buffer[offset + 11],
    ]);
    let w = f32::from_le_bytes([
        buffer[offset + 12],
        buffer[offset + 13],
        buffer[offset + 14],
        buffer[offset + 15],
    ]);

    Ok([x, y, z, w])
}
