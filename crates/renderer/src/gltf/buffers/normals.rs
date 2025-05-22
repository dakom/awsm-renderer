use awsm_renderer_core::pipeline::{primitive::IndexFormat, vertex::VertexFormat};
use glam::Vec3;

use super::{index::GltfMeshBufferIndexInfo, vertex::BufferByAttributeKind};
use crate::gltf::error::{AwsmGltfError, Result};

pub(super) fn compute_normals(
    positions: &BufferByAttributeKind,
    index_info_opt: Option<(&GltfMeshBufferIndexInfo, &[u8])>,
) -> Result<Vec<u8>> {
    tracing::info!("no baked normals, computing out of thin air...");
    if positions.vertex_format != VertexFormat::Float32x3 {
        return Err(AwsmGltfError::ConstructNormals(format!(
            "Position attribute format should be Float32x3 (Vec3<f32>) but instead is {:?}",
            positions.vertex_format
        )));
    }

    // Parse vertex positions
    // Each vertex is 3 f32 values, so 3 * 4 = 12 bytes.
    if positions.attribute_bytes.len() % 12 != 0 {
        return Err(AwsmGltfError::ConstructNormals(format!(
            "Position attribute_bytes length ({}) is not a multiple of 12 (3 * f32).",
            positions.attribute_bytes.len()
        )));
    }

    let vertices = positions
        .attribute_bytes
        .chunks_exact(12)
        .map(|values_u8| {
            // Safety: We've checked that attribute_bytes.len() is a multiple of 12,
            // and chunks_exact(12) ensures values_u8 is always 12 bytes.
            // The VertexFormat check ensures these bytes represent three f32s.
            // The pointer is valid for 12 bytes, and we're reading 3 f32s (12 bytes).
            // Alignment of f32 on most platforms is 4 bytes, and slices from Vec<u8>
            // might not guarantee this if not careful, but from_raw_parts relies on caller.
            // Assuming the source buffer (e.g., GLTF accessor) is correctly aligned for f32.
            let values_f32 = unsafe {
                std::slice::from_raw_parts(
                    values_u8.as_ptr() as *const f32,
                    3, // x, y, z
                )
            };
            Vec3::new(values_f32[0], values_f32[1], values_f32[2])
        })
        .collect::<Vec<Vec3>>();

    if vertices.is_empty() {
        // If there are no vertices, there can be no normals or triangles.
        return Ok(Vec::new());
    }

    let triangle_indices: Vec<[usize; 3]> = match index_info_opt {
        Some((index_spec, index_bytes_buffer)) => {
            // Ensure the index buffer slice is valid based on offset and total_size
            if index_spec.offset > index_bytes_buffer.len()
                || index_spec.offset.saturating_add(index_spec.total_size())
                    > index_bytes_buffer.len()
            {
                return Err(AwsmGltfError::ConstructNormals(format!(
                    "Index buffer access out of bounds. Buffer len: {}, offset: {}, total_size: {}",
                    index_bytes_buffer.len(),
                    index_spec.offset,
                    index_spec.total_size()
                )));
            }

            let index_values_slice =
                &index_bytes_buffer[index_spec.offset..index_spec.offset + index_spec.total_size()];

            if index_spec.count % 3 != 0 {
                return Err(AwsmGltfError::ConstructNormals(format!(
                    "Index count ({}) is not a multiple of 3, cannot form triangles.",
                    index_spec.count
                )));
            }
            if index_spec.total_size() != index_spec.count * index_spec.data_size {
                return Err(AwsmGltfError::ConstructNormals(format!(
                    "Index total_size ({}) does not match count ({}) * data_size ({}).",
                    index_spec.total_size(),
                    index_spec.count,
                    index_spec.data_size
                )));
            }
            if index_spec.count == 0 {
                // No indices means no triangles
                return Ok(Vec::new());
            }

            let num_triangles = index_spec.count / 3;
            let mut triangles = Vec::with_capacity(num_triangles);

            for i in 0..num_triangles {
                let mut current_triangle_vertex_indices = [0usize; 3];
                for (j, current_vertex_index) in
                    current_triangle_vertex_indices.iter_mut().enumerate()
                {
                    // For each of the 3 vertices in the current triangle
                    // Calculate the byte offset for the current vertex's index within index_values_slice
                    let index_element_offset = (i * 3 + j) * index_spec.data_size;

                    if index_element_offset + index_spec.data_size > index_values_slice.len() {
                        return Err(AwsmGltfError::ConstructNormals(format!(
                            "Attempting to read index data out of bounds of index_values_slice. Triangle {}, Vertex {}, Offset {}, Slice len {}",
                            i, j, index_element_offset, index_values_slice.len()
                        )));
                    }

                    let individual_index_bytes = &index_values_slice
                        [index_element_offset..index_element_offset + index_spec.data_size];

                    let vertex_idx = match index_spec.format {
                        IndexFormat::Uint16 => {
                            if index_spec.data_size != 2 {
                                return Err(AwsmGltfError::ConstructNormals(
                                    "IndexFormat::Uint16 expects data_size of 2.".to_string(),
                                ));
                            }
                            u16::from_ne_bytes(individual_index_bytes.try_into().map_err(|e| {
                                AwsmGltfError::ConstructNormals(format!(
                                    "Failed to convert index bytes to u16 for triangle {}, vertex {}: {:?}. Slice len: {}. Expected 2.",
                                    i, j, e, individual_index_bytes.len()
                                ))
                            })?) as usize
                        }
                        IndexFormat::Uint32 => {
                            if index_spec.data_size != 4 {
                                return Err(AwsmGltfError::ConstructNormals(
                                    "IndexFormat::Uint32 expects data_size of 4.".to_string(),
                                ));
                            }
                            u32::from_ne_bytes(individual_index_bytes.try_into().map_err(|e| {
                                AwsmGltfError::ConstructNormals(format!(
                                    "Failed to convert index bytes to u32 for triangle {}, vertex {}: {:?}. Slice len: {}. Expected 4.",
                                    i, j, e, individual_index_bytes.len()
                                ))
                            })?) as usize
                        }
                        _ => return Err(AwsmGltfError::UnsupportedIndexFormat(index_spec.format)),
                    };

                    if vertex_idx >= vertices.len() {
                        return Err(AwsmGltfError::ConstructNormals(format!(
                            "Vertex index {} out of bounds (total vertices: {}). Triangle {}, vertex {}.",
                            vertex_idx, vertices.len(), i, j
                        )));
                    }

                    *current_vertex_index = vertex_idx;
                }
                triangles.push(current_triangle_vertex_indices);
            }
            triangles
        }
        None => {
            // Non-indexed mesh
            if vertices.len() % 3 != 0 {
                // For non-indexed meshes, if we have vertices but not a multiple of 3,
                // we can't form triangles, so no normals.
                // If vertices.is_empty() was true, we'd have returned already.
                return Err(AwsmGltfError::ConstructNormals(format!(
                    "For non-indexed meshes, vertex count ({}) must be a multiple of 3.",
                    vertices.len()
                )));
            }
            // If vertices.len() is 0 (and a multiple of 3), this will correctly produce an empty Vec.
            (0..(vertices.len() / 3))
                .map(|i| [i * 3, i * 3 + 1, i * 3 + 2])
                .collect()
        }
    };

    // If no triangles could be formed (either from indices or non-indexed setup),
    // then there are no normals to compute.
    if triangle_indices.is_empty() {
        // This covers cases like:
        // - Indexed mesh with index_spec.count == 0.
        // - Non-indexed mesh with vertices.len() == 0.
        // - (Error cases like index_spec.count < 3 would have returned Err earlier).
        return Ok(Vec::new());
    }

    let mut normals = vec![Vec3::ZERO; vertices.len()];

    for triangle_vertex_indices_array in &triangle_indices {
        // These indices have already been bounds-checked during triangle construction
        let v0 = vertices[triangle_vertex_indices_array[0]];
        let v1 = vertices[triangle_vertex_indices_array[1]];
        let v2 = vertices[triangle_vertex_indices_array[2]];

        // Compute edges
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;

        // Compute face normal (unnormalized)
        // The direction of this normal depends on the winding order (v0,v1,v2)
        let face_normal = edge1.cross(edge2);

        // Accumulate face normal into each vertex's normal
        // This weighting by face normal magnitude is standard
        normals[triangle_vertex_indices_array[0]] += face_normal;
        normals[triangle_vertex_indices_array[1]] += face_normal;
        normals[triangle_vertex_indices_array[2]] += face_normal;
    }

    // Normalize vertex normals
    for normal in &mut normals {
        // Handle cases where the accumulated normal might be zero (e.g., degenerate triangles or perfectly opposing face normals)
        if *normal != Vec3::ZERO {
            *normal = normal.normalize();
        } else {
            // Default normal for degenerate cases, e.g., up vector or (0,0,0) if preferred.
            // Or leave as Vec3::ZERO if that's acceptable for the renderer.
            // Using (0,1,0) as a fallback, but this might not be ideal for all situations.
            // Consider what your renderer expects for zero-length normals.
            // *normal = Vec3::Y; // Example: default to Y-up
        }
    }

    // Convert Vec<Vec3> normals to Vec<u8>
    let mut normals_bytes = Vec::with_capacity(normals.len() * 12); // 3 floats * 4 bytes/float
    for normal in &normals {
        normals_bytes.extend_from_slice(&normal.x.to_ne_bytes());
        normals_bytes.extend_from_slice(&normal.y.to_ne_bytes());
        normals_bytes.extend_from_slice(&normal.z.to_ne_bytes());
    }

    Ok(normals_bytes)
}
