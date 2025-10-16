use std::borrow::Cow;

use crate::buffer::helpers::slice_zeroes;
use crate::gltf::buffers::accessor::accessor_to_bytes;
use crate::gltf::buffers::index::extract_triangle_indices;
use crate::gltf::buffers::{
    MeshBufferAttributeIndexInfoWithOffset, MeshBufferGeometryMorphInfoWithOffset,
    MeshBufferMaterialMorphInfoWithOffset,
};
use crate::gltf::error::{AwsmGltfError, Result};
use crate::mesh::{MeshBufferMaterialMorphAttributes, MeshBufferVertexAttributeInfo};

/// Converts GLTF morph targets into separate geometry and material buffers
///
/// KEY CONCEPT - Two Different Access Patterns:
///
/// GEOMETRY MORPHS (Position only):
/// - Used in visibility/geometry pass for vertex positioning
/// - Needs per-triangle-corner data (exploded) to match visibility buffer
/// - Layout: Triangle0[Corner0[T0_pos, T1_pos], Corner1[...], Corner2[...]]
///
/// MATERIAL MORPHS (Normals + Tangents):
/// - Used in material/shading pass for surface properties  
/// - Can use original per-vertex data (non-exploded) for efficiency
/// - Layout: Vertex0[T0_norm, T0_tang, T1_norm, T1_tang], Vertex1[...], etc.
/// - Material pass interpolates between original vertices anyway
pub(super) fn convert_morph_targets(
    primitive: &gltf::Primitive,
    buffers: &[Vec<u8>],
    index: &MeshBufferAttributeIndexInfoWithOffset,
    index_bytes: &[u8],
    triangle_count: usize,
    geometry_morph_bytes: &mut Vec<u8>, // Exploded position morphs only
    material_morph_bytes: &mut Vec<u8>, // Non-exploded normal/tangent morphs
) -> Result<(
    Option<MeshBufferGeometryMorphInfoWithOffset>,
    Option<MeshBufferMaterialMorphInfoWithOffset>,
)> {
    let has_any_position_morph = primitive
        .morph_targets()
        .any(|morph_target| morph_target.positions().is_some());
    let has_any_normal_morph = primitive
        .morph_targets()
        .any(|morph_target| morph_target.normals().is_some());

    let has_any_tangent_morph = primitive
        .morph_targets()
        .any(|morph_target| morph_target.tangents().is_some());

    if !has_any_position_morph && !has_any_normal_morph && !has_any_tangent_morph {
        return Ok((None, None));
    }

    // Load all morph target data from GLTF
    // This is the ORIGINAL per-vertex morph data (deltas from base mesh)
    #[derive(Default)]
    struct MorphTargetBufferData<'a> {
        positions: Option<Cow<'a, [u8]>>, // Position deltas (vec3<f32> per original vertex)
        normals: Option<Cow<'a, [u8]>>,   // Normal deltas (vec3<f32> per original vertex)
        tangents: Option<Cow<'a, [u8]>>,  // Tangent deltas (vec3<f32> per original vertex, no W)
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

    let targets_len = morph_targets_buffer_data.len();

    // Get original vertex count for material morphs
    let original_vertex_count = primitive
        .attributes()
        .next()
        .map(|(_, accessor)| accessor.count())
        .unwrap_or(0);

    // GEOMETRY MORPHS: Convert positions to exploded triangle-corner format
    let geometry_morph_info = if has_any_position_morph {
        let geometry_values_offset = geometry_morph_bytes.len();

        // Get triangles with ORIGINAL vertex indices for explosion
        let triangle_indices = extract_triangle_indices(index, index_bytes)?;

        // Size of position data for ONE triangle, ONE target (3 vertices * vec3<f32>)
        let size_per_target_per_triangle = 36; // 3 vertices * 12 bytes (vec3<f32>)

        // Size of position data for ONE triangle, ALL targets
        let triangle_stride_size = size_per_target_per_triangle * targets_len;

        // TRIANGLE EXPLOSION FOR POSITIONS
        // Convert from original indexed position morphs to exploded triangle-corner data
        //
        // INTERLEAVING PATTERN (for visibility buffer):
        // Triangle 0:
        //   Corner 0: [MorphTarget0_pos, MorphTarget1_pos, MorphTarget2_pos, ...]
        //   Corner 1: [MorphTarget0_pos, MorphTarget1_pos, MorphTarget2_pos, ...]
        //   Corner 2: [MorphTarget0_pos, MorphTarget1_pos, MorphTarget2_pos, ...]
        // Triangle 1:
        //   Corner 0: [MorphTarget0_pos, MorphTarget1_pos, MorphTarget2_pos, ...]
        //   ... etc
        for triangle in &triangle_indices {
            // For each vertex corner in this triangle (3 corners per triangle)
            for vertex_index in triangle {
                // For each morph target (interleaved per vertex corner)
                for morph_target_buffer_data in &morph_targets_buffer_data {
                    match &morph_target_buffer_data.positions {
                        Some(position_data) => {
                            // Look up the position delta using the ORIGINAL vertex index
                            let data_byte_offset = vertex_index * 12; // vec3<f32> = 12 bytes
                            if data_byte_offset + 12 > position_data.len() {
                                return Err(AwsmGltfError::ConstructNormals(format!(
                                    "Position morph data out of bounds for vertex {}",
                                    vertex_index
                                )));
                            }
                            let position_bytes =
                                &position_data[data_byte_offset..data_byte_offset + 12];
                            geometry_morph_bytes.extend_from_slice(position_bytes);
                        }
                        None => {
                            // Fill with zeros if this target doesn't have positions
                            geometry_morph_bytes.extend_from_slice(slice_zeroes(12));
                        }
                    }
                }
            }
        }

        let geometry_values_size = geometry_morph_bytes.len() - geometry_values_offset;

        Some(MeshBufferGeometryMorphInfoWithOffset {
            targets_len,
            triangle_stride_size,
            values_size: geometry_values_size,
            values_offset: geometry_values_offset,
        })
    } else {
        None
    };

    let morph_attributes = MeshBufferMaterialMorphAttributes {
        normal: has_any_normal_morph,
        tangent: has_any_tangent_morph,
    };

    // MATERIAL MORPHS: Keep normals + tangents in original per-vertex format
    let material_morph_info = if morph_attributes.normal || morph_attributes.tangent {
        let material_values_offset = material_morph_bytes.len();

        // Size of material data for ONE vertex, ONE target
        let mut size_per_target_per_vertex = 0;
        if morph_attributes.normal {
            size_per_target_per_vertex += 12; // vec3<f32>
        }
        if morph_attributes.tangent {
            size_per_target_per_vertex += 12; // vec3<f32> (no w component in morph targets)
        }

        // Size of material data for ONE vertex, ALL targets
        let vertex_stride_size = size_per_target_per_vertex * targets_len;

        // NON-EXPLODED LAYOUT FOR MATERIALS
        // Keep original per-vertex structure since material pass interpolates anyway
        //
        // INTERLEAVING PATTERN (for material pass):
        // Vertex 0: [Target0_norm, Target0_tang, Target1_norm, Target1_tang, ...]
        // Vertex 1: [Target0_norm, Target0_tang, Target1_norm, Target1_tang, ...]
        // Vertex 2: [Target0_norm, Target0_tang, Target1_norm, Target1_tang, ...]
        // ... etc (original vertex order preserved)
        for original_vertex_index in 0..original_vertex_count {
            // For each morph target (interleaved per original vertex)
            for morph_target_buffer_data in &morph_targets_buffer_data {
                // Helper to push material attribute data
                let mut push_material_morph_data =
                    |attribute_info: MeshBufferVertexAttributeInfo,
                     data: Option<&Cow<'_, [u8]>>|
                     -> Result<()> {
                        let vertex_size = attribute_info.vertex_size();
                        match data {
                            Some(data) => {
                                // Look up the morph delta using the ORIGINAL vertex index
                                let data_byte_offset = original_vertex_index * vertex_size;
                                if data_byte_offset + vertex_size > data.len() {
                                    return Err(AwsmGltfError::ConstructNormals(format!(
                                    "Material morph data out of bounds for vertex {} in attribute {:?}",
                                    original_vertex_index, attribute_info
                                )));
                                }
                                let data_bytes =
                                    &data[data_byte_offset..data_byte_offset + vertex_size];
                                material_morph_bytes.extend_from_slice(data_bytes);
                            }
                            None => {
                                // Fill with zeros if this target doesn't have this attribute
                                material_morph_bytes.extend_from_slice(slice_zeroes(vertex_size));
                            }
                        }
                        Ok(())
                    };

                // Push material attributes in consistent order FOR THIS TARGET
                if morph_attributes.normal {
                    push_material_morph_data(
                        MeshBufferVertexAttributeInfo::Normals {
                            data_size: 4,     // f32
                            component_len: 3, // vec3
                        },
                        morph_target_buffer_data.normals.as_ref(),
                    )?;
                }
                if morph_attributes.tangent {
                    push_material_morph_data(
                        MeshBufferVertexAttributeInfo::Tangents {
                            data_size: 4,     // f32
                            component_len: 3, // vec3
                        },
                        morph_target_buffer_data.tangents.as_ref(),
                    )?;
                }
            }
        }

        let material_values_size = material_morph_bytes.len() - material_values_offset;

        Some(MeshBufferMaterialMorphInfoWithOffset {
            attributes: morph_attributes,
            targets_len,
            vertex_stride_size,
            values_size: material_values_size,
            values_offset: material_values_offset,
        })
    } else {
        None
    };

    Ok((geometry_morph_info, material_morph_info))
}
