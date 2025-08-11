use awsm_renderer_core::pipeline::primitive::FrontFace;
use awsm_renderer_core::pipeline::vertex::VertexFormat;
use gltf::accessor::{DataType, Dimensions};
use gltf::Semantic;
use std::borrow::Cow;
use std::collections::HashMap;

use crate::buffer::helpers::{
    i16_to_i32_vec, slice_zeroes, u16_to_u32_vec, u8_to_i16_vec, u8_to_u16_vec,
};
use crate::gltf::buffers::helpers::{
    extract_triangle_indices, get_attribute_components, get_position_from_buffer,
    normalize_triangle_winding, semantic_cmp, semantic_to_shader_key,
};
use crate::gltf::buffers::normals::compute_normals;
use crate::gltf::buffers::{
    MeshBufferIndexInfoWithOffset, MeshBufferInfoWithOffset, MeshBufferMorphInfoWithOffset,
    MeshBufferTriangleDataInfoWithOffset, MeshBufferTriangleInfoWithOffset,
    MeshBufferVertexAttributeInfoWithOffset, MeshBufferVertexInfoWithOffset,
};
use crate::gltf::error::AwsmGltfError;
use crate::mesh::{
    MeshBufferIndexInfo, MeshBufferInfo, MeshBufferMorphAttributes, MeshBufferMorphInfo,
    MeshBufferTriangleDataInfo, MeshBufferTriangleInfo, MeshBufferVertexAttributeInfo,
    MeshBufferVertexAttributeKind, MeshBufferVertexInfo,
};

use super::accessor::accessor_to_bytes;
use super::Result;

pub(super) fn convert_to_visibility_buffer(
    primitive: &gltf::Primitive,
    front_face: FrontFace,
    buffers: &[Vec<u8>],
    index: &MeshBufferIndexInfoWithOffset,
    index_bytes: &[u8],
    visibility_vertex_bytes: &mut Vec<u8>,
    attribute_vertex_bytes: &mut Vec<u8>,
    triangle_data_bytes: &mut Vec<u8>,
    triangle_morph_bytes: &mut Vec<u8>,
) -> Result<MeshBufferInfoWithOffset> {
    // Step 1: Load all GLTF attributes
    let mut gltf_attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> =
        primitive.attributes().collect();

    gltf_attributes.sort_by(|(a, _), (b, _)| semantic_cmp(a, b));

    // this should never be empty, but let's be safe
    let vertex_count = gltf_attributes
        .first()
        .map(|(_, accessor)| accessor.count())
        .unwrap_or(0);

    let triangle_count = index.count / 3;

    // Step 2: Load attribute data by kind
    let attribute_data_by_kind = load_attribute_data_by_kind(&gltf_attributes, buffers)?;

    // Step 3: Ensure normals exist (compute if missing)
    let attribute_data_by_kind = ensure_normals(attribute_data_by_kind, index, index_bytes)?;

    // Step 4: Create visibility vertices (positions + triangle_id + barycentric)
    // These are expanded such that each vertex gets its own visibility vertex (triangle_id will be repeated for all 3)
    let visability_vertex_offset = visibility_vertex_bytes.len();
    create_visibility_vertices(
        &attribute_data_by_kind,
        index,
        index_bytes,
        triangle_count,
        visibility_vertex_bytes,
    )?;

    // Step 5: Pack vertex attributes
    // These are the original attributes per-vertex, excluding positions
    // There is no need to repack or expand these, they are used as-is
    let attribute_vertex_offset = attribute_vertex_bytes.len();
    let vertex_attributes =
        pack_vertex_attributes(&attribute_data_by_kind, attribute_vertex_bytes)?;

    // Step 6: Pack triangle data (vertex indices + material info)
    let triangle_data_offset = triangle_data_bytes.len();
    let triangle_data_info = pack_triangle_data(
        index,
        index_bytes,
        triangle_count,
        triangle_data_offset,
        triangle_data_bytes,
        front_face,
        primitive.material().double_sided(),
    )?;

    // Step 7: Handle morph targets (if any)
    let morph_info = if primitive.morph_targets().len() > 0 {
        Some(convert_morph_targets(
            primitive,
            buffers,
            index,
            index_bytes,
            triangle_count,
            triangle_morph_bytes,
        )?)
    } else {
        None
    };

    // Step 7: Build final MeshBufferInfo
    Ok(MeshBufferInfoWithOffset {
        vertex: MeshBufferVertexInfoWithOffset {
            offset: visability_vertex_offset,
            count: triangle_count * 3,     // 3 vertices per triangle
            size: triangle_count * 3 * 24, // 24 bytes per vertex (12 pos + 4 triangle_id + 8 bary)
        },
        triangles: MeshBufferTriangleInfoWithOffset {
            count: triangle_count,
            indices: index.clone(),
            vertex_attributes,
            vertex_attributes_offset: attribute_vertex_offset,
            vertex_attributes_size: attribute_vertex_bytes.len() - attribute_vertex_offset,
            triangle_data: triangle_data_info,
        },
        morph: morph_info,
    })
}

// Helper function to load attribute data (similar to your existing code)
fn load_attribute_data_by_kind<'a>(
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

fn ensure_normals<'a>(
    mut attribute_data: HashMap<MeshBufferVertexAttributeKind, Cow<'a, [u8]>>,
    index: &MeshBufferIndexInfoWithOffset,
    index_bytes: &[u8],
) -> Result<HashMap<MeshBufferVertexAttributeKind, Cow<'a, [u8]>>> {
    if !attribute_data.contains_key(&MeshBufferVertexAttributeKind::Normals) {
        let positions = attribute_data
            .get(&MeshBufferVertexAttributeKind::Positions)
            .ok_or_else(|| AwsmGltfError::ConstructNormals("missing positions".to_string()))?;

        let normals_bytes = compute_normals(positions, index, index_bytes)?;
        attribute_data.insert(
            MeshBufferVertexAttributeKind::Normals,
            Cow::Owned(normals_bytes),
        );
    }

    Ok(attribute_data)
}

fn create_visibility_vertices(
    attribute_data: &HashMap<MeshBufferVertexAttributeKind, Cow<'_, [u8]>>,
    index: &MeshBufferIndexInfoWithOffset,
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
        .get(&MeshBufferVertexAttributeKind::Positions)
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
    for (triangle_id, triangle) in triangle_indices.iter().enumerate() {
        // Create 3 visibility vertices for this triangle
        for (vertex_in_triangle, &vertex_index) in triangle.iter().enumerate() {
            // Get position for this vertex
            let position = get_position_from_buffer(&positions, vertex_index)?;

            // Write vertex data: position (12 bytes) + triangle_id (4 bytes) + barycentric (8 bytes)

            // Position (12 bytes)
            visibility_vertex_bytes.extend_from_slice(&position[0].to_le_bytes());
            visibility_vertex_bytes.extend_from_slice(&position[1].to_le_bytes());
            visibility_vertex_bytes.extend_from_slice(&position[2].to_le_bytes());

            // Triangle ID (4 bytes)
            visibility_vertex_bytes.extend_from_slice(&(triangle_id as u32).to_le_bytes());

            // Barycentric coordinates (8 bytes)
            let bary = BARYCENTRICS[vertex_in_triangle];
            visibility_vertex_bytes.extend_from_slice(&bary[0].to_le_bytes());
            visibility_vertex_bytes.extend_from_slice(&bary[1].to_le_bytes());
        }
    }

    Ok(())
}
// Pack vertex attributes in original layout (for indexed access)
fn pack_vertex_attributes(
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

// Pack triangle data (vertex indices + material info)
fn pack_triangle_data(
    index: &MeshBufferIndexInfoWithOffset,
    index_bytes: &[u8],
    triangle_count: usize,
    offset: usize,
    triangle_data_bytes: &mut Vec<u8>,
    front_face: FrontFace,
    double_sided: bool,
) -> Result<MeshBufferTriangleDataInfoWithOffset> {
    let triangle_indices = extract_triangle_indices(index, index_bytes)?;

    for triangle in triangle_indices {
        // Normalize winding order here
        let normalized_triangle = if double_sided {
            triangle // Keep original winding for double-sided materials
        } else {
            normalize_triangle_winding(triangle, front_face)
        };
        // Pack triangle vertex indices (3 * u32 = 12 bytes)
        triangle_data_bytes.extend_from_slice(&(normalized_triangle[0] as u32).to_le_bytes());
        triangle_data_bytes.extend_from_slice(&(normalized_triangle[1] as u32).to_le_bytes());
        triangle_data_bytes.extend_from_slice(&(normalized_triangle[2] as u32).to_le_bytes());

        // Pack material_id (u32 = 4 bytes) - TODO: get actual material ID
        let material_id = 0u32; // Placeholder
        triangle_data_bytes.extend_from_slice(&material_id.to_le_bytes());
    }

    let size_per_triangle = 16; // 3 u32 indices + 1 u32 material_id
    let total_size = triangle_count * size_per_triangle;

    Ok(MeshBufferTriangleDataInfoWithOffset {
        size_per_triangle,
        offset,
        total_size,
    })
}

fn convert_morph_targets(
    primitive: &gltf::Primitive,
    buffers: &[Vec<u8>],
    index: &MeshBufferIndexInfoWithOffset,
    index_bytes: &[u8],
    triangle_count: usize,
    triangle_morph_bytes: &mut Vec<u8>,
) -> Result<MeshBufferMorphInfoWithOffset> {
    // Determine what morph attributes we have
    let attributes = MeshBufferMorphAttributes {
        position: primitive
            .morph_targets()
            .any(|morph_target| morph_target.positions().is_some()),
        normal: primitive
            .morph_targets()
            .any(|morph_target| morph_target.normals().is_some()),
        tangent: primitive
            .morph_targets()
            .any(|morph_target| morph_target.tangents().is_some()),
    };

    if !attributes.any() {
        return Err(AwsmGltfError::ConstructNormals(
            "No morph targets found".to_string(),
        ));
    }

    // Load all morph target data
    #[derive(Default)]
    struct MorphTargetBufferData<'a> {
        positions: Option<Cow<'a, [u8]>>,
        normals: Option<Cow<'a, [u8]>>,
        tangents: Option<Cow<'a, [u8]>>,
    }

    let mut morph_targets_buffer_data = Vec::new();
    for morph_target in primitive.morph_targets() {
        let mut morph_target_buffer_data = MorphTargetBufferData::default();

        if let Some(accessor) = morph_target.positions() {
            morph_target_buffer_data.positions = Some(accessor_to_bytes(&accessor, buffers)?);
        }
        if let Some(accessor) = morph_target.normals() {
            morph_target_buffer_data.normals = Some(accessor_to_bytes(&accessor, buffers)?);
        }
        if let Some(accessor) = morph_target.tangents() {
            morph_target_buffer_data.tangents = Some(accessor_to_bytes(&accessor, buffers)?);
        }

        morph_targets_buffer_data.push(morph_target_buffer_data);
    }

    let values_offset = triangle_morph_bytes.len();
    let triangle_indices = extract_triangle_indices(index, index_bytes)?;

    // Calculate triangle stride size (size of morph data per triangle across all targets)
    let mut triangle_stride_size = 0;
    let targets_len = morph_targets_buffer_data.len();

    // For each target, calculate the size per triangle
    let size_per_target_per_triangle = {
        let mut size = 0;
        if attributes.position {
            size += 36;
        } // 3 vertices * 12 bytes (vec3<f32>)
        if attributes.normal {
            size += 36;
        } // 3 vertices * 12 bytes (vec3<f32>)
        if attributes.tangent {
            size += 36;
        } // 3 vertices * 12 bytes (vec3<f32>) - morph tangents don't include w
        size
    };
    triangle_stride_size = size_per_target_per_triangle * targets_len;

    // Convert from per-vertex to per-triangle layout
    // Layout: for each triangle, for each target, for each vertex in triangle, for each attribute
    for triangle in triangle_indices {
        for morph_target_buffer_data in &morph_targets_buffer_data {
            // For each vertex in the triangle
            for &vertex_index in &triangle {
                let mut push_vertex_morph_data = |attribute_kind: MeshBufferVertexAttributeKind,
                                                  data: Option<&Cow<'_, [u8]>>|
                 -> Result<()> {
                    let stride_size = match attribute_kind {
                        MeshBufferVertexAttributeKind::Positions => 12, // vec3<f32>
                        MeshBufferVertexAttributeKind::Normals => 12,   // vec3<f32>
                        MeshBufferVertexAttributeKind::Tangents => 12, // vec3<f32> (no w component in morph targets)
                        _ => unreachable!(),
                    };

                    match data {
                        Some(data) => {
                            let data_byte_offset = vertex_index * stride_size;
                            if data_byte_offset + stride_size > data.len() {
                                return Err(AwsmGltfError::ConstructNormals(format!(
                                    "Morph data out of bounds for vertex {} in attribute {:?}",
                                    vertex_index, attribute_kind
                                )));
                            }
                            let data_bytes =
                                &data[data_byte_offset..data_byte_offset + stride_size];
                            triangle_morph_bytes.extend_from_slice(data_bytes);
                        }
                        None => {
                            // Fill with zeros if this target doesn't have this attribute
                            triangle_morph_bytes.extend_from_slice(slice_zeroes(stride_size));
                        }
                    }
                    Ok(())
                };

                // Push attributes in consistent order
                if attributes.position {
                    push_vertex_morph_data(
                        MeshBufferVertexAttributeKind::Positions,
                        morph_target_buffer_data.positions.as_ref(),
                    )?;
                }
                if attributes.normal {
                    push_vertex_morph_data(
                        MeshBufferVertexAttributeKind::Normals,
                        morph_target_buffer_data.normals.as_ref(),
                    )?;
                }
                if attributes.tangent {
                    push_vertex_morph_data(
                        MeshBufferVertexAttributeKind::Tangents,
                        morph_target_buffer_data.tangents.as_ref(),
                    )?;
                }
            }
        }
    }

    let values_size = triangle_morph_bytes.len() - values_offset;

    Ok(MeshBufferMorphInfoWithOffset {
        attributes,
        targets_len,
        triangle_stride_size,
        values_size,
        values_offset,
    })
}
