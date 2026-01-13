use std::{borrow::Cow, collections::BTreeMap};

use crate::{
    gltf::{
        buffers::{index::extract_triangle_indices, MeshBufferAttributeIndexInfoWithOffset},
        error::{AwsmGltfError, Result},
    },
    mesh::{
        MeshBufferCustomVertexAttributeInfo, MeshBufferVertexAttributeInfo,
        MeshBufferVisibilityVertexAttributeInfo,
    },
};

/// Generates tangents using MikkTSpace algorithm if:
/// - The primitive has a normal map (material has normalTexture)
/// - The primitive doesn't already have tangent attributes
/// - UV coordinates exist (required for tangent calculation)
pub(super) fn ensure_tangents<'a>(
    mut attribute_data: BTreeMap<MeshBufferVertexAttributeInfo, Cow<'a, [u8]>>,
    primitive: &gltf::Primitive<'_>,
    index: &MeshBufferAttributeIndexInfoWithOffset,
    index_bytes: &[u8],
) -> Result<BTreeMap<MeshBufferVertexAttributeInfo, Cow<'a, [u8]>>> {
    // Check if tangents already exist
    let has_tangents = attribute_data.keys().any(|x| {
        matches!(
            x,
            MeshBufferVertexAttributeInfo::Visibility(
                MeshBufferVisibilityVertexAttributeInfo::Tangents { .. }
            )
        )
    });

    if has_tangents {
        return Ok(attribute_data);
    }

    // Check if this primitive needs tangents (has a normal map)
    let needs_tangents = primitive.material().normal_texture().is_some()
        || primitive
            .material()
            .clearcoat()
            .is_some_and(|cc| cc.clearcoat_normal_texture().is_some());

    if !needs_tangents {
        return Ok(attribute_data);
    }

    // Check if we have the required data for tangent generation
    let positions = attribute_data.iter().find_map(|(k, v)| match k {
        MeshBufferVertexAttributeInfo::Visibility(
            MeshBufferVisibilityVertexAttributeInfo::Positions { .. },
        ) => Some(v.as_ref()),
        _ => None,
    });

    let normals = attribute_data.iter().find_map(|(k, v)| match k {
        MeshBufferVertexAttributeInfo::Visibility(
            MeshBufferVisibilityVertexAttributeInfo::Normals { .. },
        ) => Some(v.as_ref()),
        _ => None,
    });

    // Find TEXCOORD_0 (UV set 0) - required for tangent calculation
    let texcoords = attribute_data.iter().find_map(|(k, v)| match k {
        MeshBufferVertexAttributeInfo::Custom(MeshBufferCustomVertexAttributeInfo::TexCoords {
            index: 0,
            ..
        }) => Some(v.as_ref()),
        _ => None,
    });

    let (positions, normals, texcoords) = match (positions, normals, texcoords) {
        (Some(p), Some(n), Some(t)) => (p, n, t),
        _ => {
            tracing::warn!(
                "Cannot generate tangents: missing positions, normals, or UV coordinates"
            );
            return Ok(attribute_data);
        }
    };

    // Generate tangents
    let tangents_bytes = compute_tangents(positions, normals, texcoords, index, index_bytes)?;

    attribute_data.insert(
        MeshBufferVertexAttributeInfo::Visibility(
            MeshBufferVisibilityVertexAttributeInfo::Tangents {
                data_size: 4,     // f32
                component_len: 4, // vec4 (xyz + handedness w)
            },
        ),
        Cow::Owned(tangents_bytes),
    );

    tracing::info!("Generated tangents using MikkTSpace algorithm");

    Ok(attribute_data)
}

/// Wrapper struct for mikktspace Geometry trait implementation
struct MikkTSpaceGeometry<'a> {
    positions: &'a [u8],
    normals: &'a [u8],
    texcoords: &'a [u8],
    triangles: Vec<[usize; 3]>,
    tangents: Vec<[f32; 4]>,
}

impl<'a> MikkTSpaceGeometry<'a> {
    fn new(
        positions: &'a [u8],
        normals: &'a [u8],
        texcoords: &'a [u8],
        triangles: Vec<[usize; 3]>,
        vertex_count: usize,
    ) -> Self {
        Self {
            positions,
            normals,
            texcoords,
            triangles,
            tangents: vec![[0.0, 0.0, 0.0, 1.0]; vertex_count],
        }
    }

    fn get_position(&self, vertex_index: usize) -> [f32; 3] {
        let offset = vertex_index * 12; // 3 f32s = 12 bytes
        if offset + 12 > self.positions.len() {
            return [0.0, 0.0, 0.0];
        }
        [
            f32::from_le_bytes(self.positions[offset..offset + 4].try_into().unwrap()),
            f32::from_le_bytes(self.positions[offset + 4..offset + 8].try_into().unwrap()),
            f32::from_le_bytes(self.positions[offset + 8..offset + 12].try_into().unwrap()),
        ]
    }

    fn get_normal(&self, vertex_index: usize) -> [f32; 3] {
        let offset = vertex_index * 12; // 3 f32s = 12 bytes
        if offset + 12 > self.normals.len() {
            return [0.0, 1.0, 0.0];
        }
        [
            f32::from_le_bytes(self.normals[offset..offset + 4].try_into().unwrap()),
            f32::from_le_bytes(self.normals[offset + 4..offset + 8].try_into().unwrap()),
            f32::from_le_bytes(self.normals[offset + 8..offset + 12].try_into().unwrap()),
        ]
    }

    fn get_texcoord(&self, vertex_index: usize) -> [f32; 2] {
        let offset = vertex_index * 8; // 2 f32s = 8 bytes
        if offset + 8 > self.texcoords.len() {
            return [0.0, 0.0];
        }
        [
            f32::from_le_bytes(self.texcoords[offset..offset + 4].try_into().unwrap()),
            f32::from_le_bytes(self.texcoords[offset + 4..offset + 8].try_into().unwrap()),
        ]
    }
}

impl mikktspace::Geometry for MikkTSpaceGeometry<'_> {
    fn num_faces(&self) -> usize {
        self.triangles.len()
    }

    fn num_vertices_of_face(&self, _face: usize) -> usize {
        3 // Always triangles
    }

    fn position(&self, face: usize, vert: usize) -> [f32; 3] {
        let vertex_index = self.triangles[face][vert];
        self.get_position(vertex_index)
    }

    fn normal(&self, face: usize, vert: usize) -> [f32; 3] {
        let vertex_index = self.triangles[face][vert];
        self.get_normal(vertex_index)
    }

    fn tex_coord(&self, face: usize, vert: usize) -> [f32; 2] {
        let vertex_index = self.triangles[face][vert];
        self.get_texcoord(vertex_index)
    }

    fn set_tangent_encoded(&mut self, tangent: [f32; 4], face: usize, vert: usize) {
        let vertex_index = self.triangles[face][vert];
        // MikkTSpace may call this multiple times for the same vertex from different faces.
        // We just overwrite - all calls for the same vertex should produce the same result
        // for well-formed meshes.
        self.tangents[vertex_index] = tangent;
    }
}

fn compute_tangents(
    positions: &[u8],
    normals: &[u8],
    texcoords: &[u8],
    index: &MeshBufferAttributeIndexInfoWithOffset,
    index_bytes: &[u8],
) -> Result<Vec<u8>> {
    // Validate buffer sizes
    if positions.len() % 12 != 0 {
        return Err(AwsmGltfError::GenerateTangents(
            "Position buffer length is not a multiple of 12".to_string(),
        ));
    }
    if normals.len() % 12 != 0 {
        return Err(AwsmGltfError::GenerateTangents(
            "Normal buffer length is not a multiple of 12".to_string(),
        ));
    }
    if texcoords.len() % 8 != 0 {
        return Err(AwsmGltfError::GenerateTangents(
            "TexCoord buffer length is not a multiple of 8".to_string(),
        ));
    }

    let vertex_count = positions.len() / 12;

    // Extract triangle indices
    let triangles = extract_triangle_indices(index, index_bytes)?;

    if triangles.is_empty() {
        return Ok(Vec::new());
    }

    // Create geometry wrapper and generate tangents
    let mut geometry =
        MikkTSpaceGeometry::new(positions, normals, texcoords, triangles, vertex_count);

    if !mikktspace::generate_tangents(&mut geometry) {
        return Err(AwsmGltfError::GenerateTangents(
            "MikkTSpace tangent generation failed".to_string(),
        ));
    }

    // Convert tangents to bytes
    let mut tangents_bytes = Vec::with_capacity(vertex_count * 16); // 4 f32s per tangent
    for tangent in &geometry.tangents {
        tangents_bytes.extend_from_slice(&tangent[0].to_le_bytes());
        tangents_bytes.extend_from_slice(&tangent[1].to_le_bytes());
        tangents_bytes.extend_from_slice(&tangent[2].to_le_bytes());
        tangents_bytes.extend_from_slice(&tangent[3].to_le_bytes());
    }

    Ok(tangents_bytes)
}
