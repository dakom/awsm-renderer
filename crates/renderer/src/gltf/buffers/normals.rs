use awsm_renderer_core::pipeline::{primitive::IndexFormat, vertex::VertexFormat};
use glam::Vec3;

use super::{index::GltfMeshBufferIndexInfo, vertex::BufferByAttributeKind};
use crate::gltf::error::{AwsmGltfError, Result};

pub(super) fn compute_normals(
    positions: &BufferByAttributeKind,
    index: Option<(&GltfMeshBufferIndexInfo, &[u8])>,
) -> Result<Vec<u8>> {
    if positions.vertex_format != VertexFormat::Float32x3 {
        return Err(AwsmGltfError::ConstructNormals(format!(
            "format should be vec3<f32> but instead is {:?}",
            positions.vertex_format
        )));
    }

    // 4 bytes * 3 floats
    let vertices = positions
        .attribute_bytes
        .chunks_exact(12)
        .map(|values_u8| {
            let values_f32 = unsafe {
                std::slice::from_raw_parts(
                    values_u8.as_ptr() as *const f32,
                    3, // xyz * 3
                )
            };
            Vec3::new(values_f32[0], values_f32[1], values_f32[2])
        })
        .collect::<Vec<Vec3>>();

    let triangle_indices = match index {
        Some((index, index_bytes)) => {
            let index_values = &index_bytes[index.offset..index.offset + index.total_size()];
            let mut triangles = Vec::new();

            for i in 0..(index.count / 3) {
                let offset = i * index.data_size;
                let mut triangle = [0; 3];
                for (j, vertex) in triangle.iter_mut().enumerate() {
                    let offset = offset + j * index.data_size;
                    let value = match index.format {
                        IndexFormat::Uint16 => u16::from_ne_bytes(
                            index_values[offset..offset + index.data_size]
                                .try_into()
                                .unwrap(),
                        ) as usize,
                        IndexFormat::Uint32 => u32::from_ne_bytes(
                            index_values[offset..offset + index.data_size]
                                .try_into()
                                .unwrap(),
                        ) as usize,
                        _ => {
                            return Err(AwsmGltfError::UnsupportedIndexFormat(index.format));
                        }
                    };
                    *vertex = value;
                }

                triangles.push(triangle);
            }

            triangles
        }
        None => {
            let mut triangles = Vec::new();
            for i in 0..(vertices.len() / 3) {
                triangles.push([i * 3, i * 3 + 1, i * 3 + 2]);
            }
            triangles
        }
    };

    let mut normals = vec![Vec3::ZERO; vertices.len()];

    for triangle_index in &triangle_indices {
        let v0 = vertices[triangle_index[0]];
        let v1 = vertices[triangle_index[1]];
        let v2 = vertices[triangle_index[2]];

        // Compute edges
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;

        // Compute face normal (unnormalized)
        let face_normal = edge1.cross(edge2);

        // Accumulate face normal into each vertex's normal
        normals[triangle_index[0]] += face_normal;
        normals[triangle_index[1]] += face_normal;
        normals[triangle_index[2]] += face_normal;
    }

    // Normalize vertex normals
    for normal in &mut normals {
        *normal = normal.normalize();
    }

    let mut normals_bytes = Vec::with_capacity(normals.len() * 12);
    for normal in &normals {
        normals_bytes.extend_from_slice(&normal.x.to_ne_bytes());
        normals_bytes.extend_from_slice(&normal.y.to_ne_bytes());
        normals_bytes.extend_from_slice(&normal.z.to_ne_bytes());
    }

    Ok(normals_bytes)
}
