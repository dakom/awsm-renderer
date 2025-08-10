// pub mod vertex;
//pub mod morph;
pub mod accessor;
pub mod index;
pub mod normals;
pub mod visibility;
pub mod helpers;

use super::populate::transforms::transform_gltf_node;

use awsm_renderer_core::pipeline::primitive::{FrontFace, IndexFormat};

use crate::{gltf::buffers::{helpers::transform_to_winding_order, index::{generate_fresh_indices_from_primitive, GltfMeshBufferIndexInfo}, visibility::convert_to_visibility_buffer}, mesh::{MeshBufferIndexInfo, MeshBufferInfo, MeshBufferMorphAttributes, MeshBufferMorphInfo, MeshBufferTriangleDataInfo, MeshBufferTriangleInfo, MeshBufferVertexAttributeInfo, MeshBufferVertexAttributeKind, MeshBufferVertexInfo}};

use super::error::{AwsmGltfError, Result};

#[derive(Debug)]
pub struct GltfBuffers {
    pub raw: Vec<Vec<u8>>,
    // this isn't passed to the shader at all
    // just used in the pipeline for drawing
    pub index_bytes: Vec<u8>,

    // Visibility vertex buffer (positions + triangle_id + barycentric)
    pub visibility_vertex_bytes: Vec<u8>,

    // Vertex attribute storage buffer (normals, UVs, colors, etc. per triangle)
    // these always follow the same interleaving pattern
    // although, not all primitives have all the same attributes
    // it's just that when they do, they follow the same order
    pub vertex_attribute_bytes: Vec<u8>,

    // Triangle data buffer (vertex indices + material info per triangle)
    pub triangle_data_bytes: Vec<u8>,

    // these also always follow the same interleaving pattern
    pub triangle_morph_bytes: Option<Vec<u8>>,

    // first level is mesh, second level is primitive
    pub meshes: Vec<Vec<MeshBufferInfoWithOffset>>,
}

#[derive(Clone, Debug)]
pub struct MeshBufferInfoWithOffset {
    pub vertex: MeshBufferVertexInfoWithOffset,
    pub triangles: MeshBufferTriangleInfoWithOffset,
    pub morph: Option<MeshBufferMorphInfoWithOffset>,
} 

impl From<MeshBufferInfoWithOffset> for MeshBufferInfo {
    fn from(info: MeshBufferInfoWithOffset) -> Self {
        MeshBufferInfo {
            vertex: info.vertex.into(),
            triangles: info.triangles.into(),
            morph: info.morph.map(|m| m.into()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct MeshBufferVertexInfoWithOffset {
    pub count: usize,
    pub size: usize,
    pub offset: usize,
}

impl From<MeshBufferVertexInfoWithOffset> for MeshBufferVertexInfo {
    fn from(info: MeshBufferVertexInfoWithOffset) -> Self {
        MeshBufferVertexInfo {
            count: info.count,
            size: info.size,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MeshBufferTriangleInfoWithOffset {
    pub count: usize,
    pub indices: MeshBufferIndexInfoWithOffset,
    pub vertex_attributes: Vec<MeshBufferVertexAttributeInfoWithOffset>,
    pub vertex_attributes_size: usize,
    pub triangle_data: MeshBufferTriangleDataInfoWithOffset,
}

impl From<MeshBufferTriangleInfoWithOffset> for MeshBufferTriangleInfo {
    fn from(info: MeshBufferTriangleInfoWithOffset) -> Self {
        MeshBufferTriangleInfo {
            count: info.count,
            indices: info.indices.into(),
            vertex_attributes: info.vertex_attributes.into_iter().map(|v| v.into()).collect(),
            vertex_attributes_size: info.vertex_attributes_size,
            triangle_data: info.triangle_data.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferIndexInfoWithOffset {
    pub count: usize,
    pub data_size: usize,
    pub format: IndexFormat,
    pub offset: usize,
}

impl MeshBufferIndexInfoWithOffset {
    pub fn total_size(&self) -> usize {
        self.count * self.data_size
    }
}

impl From<MeshBufferIndexInfoWithOffset> for MeshBufferIndexInfo {
    fn from(info: MeshBufferIndexInfoWithOffset) -> Self {
        MeshBufferIndexInfo {
            count: info.count,
            data_size: info.data_size,
            format: info.format,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferVertexAttributeInfoWithOffset {
    pub kind: MeshBufferVertexAttributeKind,
    pub size_per_vertex: usize,
    pub components: u32,
    pub offset: usize,
}

impl From<MeshBufferVertexAttributeInfoWithOffset> for MeshBufferVertexAttributeInfo {
    fn from(info: MeshBufferVertexAttributeInfoWithOffset) -> Self {
        MeshBufferVertexAttributeInfo {
            kind: info.kind,
            size_per_vertex: info.size_per_vertex,
            components: info.components,
        }
    }
}

#[derive(Clone, Debug)]
pub struct MeshBufferMorphInfoWithOffset {
    pub attributes: MeshBufferMorphAttributes,
    pub targets_len: usize,
    pub triangle_stride_size: usize,
    pub values_size: usize,
    pub values_offset: usize,
}

impl From<MeshBufferMorphInfoWithOffset> for MeshBufferMorphInfo {
    fn from(info: MeshBufferMorphInfoWithOffset) -> Self {
        MeshBufferMorphInfo {
            attributes: info.attributes,
            targets_len: info.targets_len,
            triangle_stride_size: info.triangle_stride_size,
            values_size: info.values_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferTriangleDataInfoWithOffset {
    pub size_per_triangle: usize,
    pub total_size: usize,
    pub offset: usize,
}

impl From<MeshBufferTriangleDataInfoWithOffset> for MeshBufferTriangleDataInfo {
    fn from(info: MeshBufferTriangleDataInfoWithOffset) -> Self {
        MeshBufferTriangleDataInfo {
            size_per_triangle: info.size_per_triangle,
            total_size: info.total_size,
        }
    }
}


impl GltfBuffers {
    pub fn new(doc: &gltf::Document, buffers: Vec<Vec<u8>>) -> Result<Self> {
        // refactor original buffers into the format we want
        // namely, pack the data in a predictable order
        // arranged by primitive
        // with indices as a separate buffer

        let mut index_bytes: Vec<u8> = Vec::new();
        let mut visibility_vertex_bytes: Vec<u8> = Vec::new();
        let mut vertex_attribute_bytes: Vec<u8> = Vec::new();
        let mut triangle_data_bytes: Vec<u8> = Vec::new();
        let mut triangle_morph_bytes: Vec<u8> = Vec::new();
        let mut meshes: Vec<Vec<MeshBufferInfoWithOffset>> = Vec::new();

        for mesh in doc.meshes() {
            let mut primitive_buffer_infos = Vec::new();

            let front_face = {
                doc.nodes()
                    .find(|node| node.mesh().is_some() && node.mesh().unwrap().index() == mesh.index())
                    .map(|node| transform_to_winding_order(&transform_gltf_node(&node).to_matrix()))
                    .unwrap_or(FrontFace::Ccw) // Default to CCW if no node found
            };
            for primitive in mesh.primitives() {
                let index:MeshBufferIndexInfoWithOffset = match GltfMeshBufferIndexInfo::maybe_new(&primitive, &buffers, &mut index_bytes)? {
                    Some(info) => info.into(),
                    None => generate_fresh_indices_from_primitive(&primitive, &mut index_bytes)?
                };


                // Step 2: Convert to visibility buffer format
                let visibility_buffer_info = convert_to_visibility_buffer(
                    &primitive,
                    front_face,
                    &buffers,
                    &index,
                    &index_bytes,
                    &mut visibility_vertex_bytes,
                    &mut vertex_attribute_bytes,
                    &mut triangle_data_bytes,
                    &mut triangle_morph_bytes,
                )?;

                primitive_buffer_infos.push(visibility_buffer_info);
            }

            meshes.push(primitive_buffer_infos);
        }

        Ok(Self {
            raw: buffers,
            index_bytes, // Always present now
            visibility_vertex_bytes,
            vertex_attribute_bytes,
            triangle_data_bytes,
            meshes,
            triangle_morph_bytes: if triangle_morph_bytes.is_empty() {
                None
            } else {
                Some(triangle_morph_bytes)
            },
        })
    }
}
