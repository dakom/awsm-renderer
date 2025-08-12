use std::{borrow::Cow, collections::HashMap};

use awsm_renderer_core::pipeline::primitive::IndexFormat;
use glam::Vec3;

use crate::{
    gltf::{
        buffers::{index::extract_triangle_indices, MeshBufferIndexInfoWithOffset},
        error::{AwsmGltfError, Result},
    },
    mesh::{MeshBufferIndexInfo, MeshBufferVertexAttributeKind},
};

pub(super) fn ensure_normals<'a>(
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

pub(super) fn compute_normals(
    positions_bytes: &[u8],
    index: &MeshBufferIndexInfoWithOffset,
    index_bytes: &[u8],
) -> Result<Vec<u8>> {
    tracing::info!("no baked normals, computing from positions and indices...");

    // Validate positions buffer (must be Float32x3 format)
    if positions_bytes.len() % 12 != 0 {
        return Err(AwsmGltfError::ConstructNormals(format!(
            "Position buffer length ({}) is not a multiple of 12 (3 * f32).",
            positions_bytes.len()
        )));
    }

    // Parse vertex positions
    let vertices = positions_bytes
        .chunks_exact(12)
        .map(|chunk| {
            let values_f32 = unsafe { std::slice::from_raw_parts(chunk.as_ptr() as *const f32, 3) };
            Vec3::new(values_f32[0], values_f32[1], values_f32[2])
        })
        .collect::<Vec<Vec3>>();

    if vertices.is_empty() {
        return Ok(Vec::new());
    }

    // Get index data - we know indices are required now
    let triangle_indices = extract_triangle_indices(index, index_bytes)?;

    if triangle_indices.is_empty() {
        return Ok(Vec::new());
    }

    // Initialize normals accumulator
    let mut normals = vec![Vec3::ZERO; vertices.len()];

    // Compute face normals and accumulate to vertices
    for triangle in &triangle_indices {
        // Bounds check
        for &vertex_idx in triangle {
            if vertex_idx >= vertices.len() {
                return Err(AwsmGltfError::ConstructNormals(format!(
                    "Vertex index {} out of bounds (total vertices: {})",
                    vertex_idx,
                    vertices.len()
                )));
            }
        }

        let v0 = vertices[triangle[0]];
        let v1 = vertices[triangle[1]];
        let v2 = vertices[triangle[2]];

        // Compute face normal
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let face_normal = edge1.cross(edge2);

        // Accumulate to vertex normals
        normals[triangle[0]] += face_normal;
        normals[triangle[1]] += face_normal;
        normals[triangle[2]] += face_normal;
    }

    // Normalize vertex normals
    for normal in &mut normals {
        if *normal != Vec3::ZERO {
            *normal = normal.normalize();
        }
        // Leave as Vec3::ZERO for degenerate cases
    }

    // Convert to bytes
    let mut normals_bytes = Vec::with_capacity(normals.len() * 12);
    for normal in &normals {
        normals_bytes.extend_from_slice(&normal.x.to_le_bytes());
        normals_bytes.extend_from_slice(&normal.y.to_le_bytes());
        normals_bytes.extend_from_slice(&normal.z.to_le_bytes());
    }

    Ok(normals_bytes)
}
