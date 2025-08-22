// pub mod vertex;
//pub mod morph;
pub mod accessor;
pub mod attributes;
pub mod index;
pub mod morph;
pub mod normals;
pub mod skin;
pub mod triangle;
pub mod visibility;

use super::populate::transforms::transform_gltf_node;

use awsm_renderer_core::pipeline::primitive::{FrontFace, IndexFormat};
use gltf::Mesh;

use crate::{
    gltf::buffers::{
        index::{generate_fresh_indices_from_primitive, GltfMeshBufferIndexInfo},
        visibility::convert_to_visibility_buffer,
    },
    mesh::{
        MeshBufferGeometryMorphInfo, MeshBufferIndexInfo, MeshBufferInfo,
        MeshBufferMaterialMorphAttributes, MeshBufferMaterialMorphInfo, MeshBufferSkinInfo,
        MeshBufferTriangleDataInfo, MeshBufferTriangleInfo, MeshBufferVertexAttributeInfo,
        MeshBufferVertexAttributeKind, MeshBufferVertexInfo,
    },
};

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
    pub attribute_vertex_bytes: Vec<u8>,

    // Triangle data buffer (vertex indices + material info per triangle)
    pub triangle_data_bytes: Vec<u8>,

    // just positions
    pub geometry_morph_bytes: Vec<u8>,
    // normal, tangent (TODO: TEXCOORD_n, COLOR_n)
    pub material_morph_bytes: Vec<u8>,

    // skins
    pub skin_joint_index_weight_bytes: Vec<u8>,

    // first level is mesh, second level is primitive
    pub meshes: Vec<Vec<MeshBufferInfoWithOffset>>,
}

#[derive(Clone, Debug)]
pub struct MeshBufferInfoWithOffset {
    pub vertex: MeshBufferVertexInfoWithOffset,
    pub triangles: MeshBufferTriangleInfoWithOffset,
    pub geometry_morph: Option<MeshBufferGeometryMorphInfoWithOffset>,
    pub material_morph: Option<MeshBufferMaterialMorphInfoWithOffset>,
    pub skin: Option<MeshBufferSkinInfoWithOffset>,
}

impl From<MeshBufferInfoWithOffset> for MeshBufferInfo {
    fn from(info: MeshBufferInfoWithOffset) -> Self {
        MeshBufferInfo {
            vertex: info.vertex.into(),
            triangles: info.triangles.into(),
            geometry_morph: info.geometry_morph.map(|m| m.into()),
            material_morph: info.material_morph.map(|m| m.into()),
            skin: info.skin.map(|m| m.into()),
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
    pub vertex_attributes_offset: usize,
    pub vertex_attributes: Vec<MeshBufferVertexAttributeInfoWithOffset>,
    pub vertex_attributes_size: usize,
    pub triangle_data: MeshBufferTriangleDataInfoWithOffset,
}

impl From<MeshBufferTriangleInfoWithOffset> for MeshBufferTriangleInfo {
    fn from(info: MeshBufferTriangleInfoWithOffset) -> Self {
        MeshBufferTriangleInfo {
            count: info.count,
            indices: info.indices.into(),
            vertex_attributes: info
                .vertex_attributes
                .into_iter()
                .map(|v| v.into())
                .collect(),
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

/// Information about geometry morphs (positions only, exploded for visibility buffer)
#[derive(Debug, Clone)]
pub struct MeshBufferGeometryMorphInfoWithOffset {
    pub targets_len: usize,
    pub triangle_stride_size: usize, // Size per triangle across all targets (positions only)
    pub values_size: usize,
    pub values_offset: usize,
}

impl From<MeshBufferGeometryMorphInfoWithOffset> for MeshBufferGeometryMorphInfo {
    fn from(info: MeshBufferGeometryMorphInfoWithOffset) -> Self {
        MeshBufferGeometryMorphInfo {
            targets_len: info.targets_len,
            triangle_stride_size: info.triangle_stride_size,
            values_size: info.values_size,
        }
    }
}

/// Information about material morphs (normals + tangents, non-exploded per-vertex)
#[derive(Debug, Clone)]
pub struct MeshBufferMaterialMorphInfoWithOffset {
    pub attributes: MeshBufferMaterialMorphAttributes, // Which attributes are present
    pub targets_len: usize,
    pub vertex_stride_size: usize, // Size per original vertex across all targets
    pub values_size: usize,
    pub values_offset: usize,
}

impl From<MeshBufferMaterialMorphInfoWithOffset> for MeshBufferMaterialMorphInfo {
    fn from(info: MeshBufferMaterialMorphInfoWithOffset) -> Self {
        MeshBufferMaterialMorphInfo {
            attributes: info.attributes,
            targets_len: info.targets_len,
            vertex_stride_size: info.vertex_stride_size,
            values_size: info.values_size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MeshBufferSkinInfoWithOffset {
    pub set_count: usize,
    pub index_weights_size: usize,
    // Offsets for GLTF population (not needed after we feed the dynamic storage buffer)
    pub index_weights_offset: usize,
}

impl From<MeshBufferSkinInfoWithOffset> for MeshBufferSkinInfo {
    fn from(info: MeshBufferSkinInfoWithOffset) -> Self {
        MeshBufferSkinInfo {
            set_count: info.set_count,
            index_weights_size: info.index_weights_size,
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
        let mut attribute_vertex_bytes: Vec<u8> = Vec::new();
        let mut triangle_data_bytes: Vec<u8> = Vec::new();
        let mut geometry_morph_bytes: Vec<u8> = Vec::new();
        let mut material_morph_bytes: Vec<u8> = Vec::new();
        let mut skin_joint_index_weight_bytes: Vec<u8> = Vec::new();
        let mut meshes: Vec<Vec<MeshBufferInfoWithOffset>> = Vec::new();

        for mesh in doc.meshes() {
            let mut primitive_buffer_infos = Vec::new();

            let front_face = {
                doc.nodes()
                    .find(|node| {
                        node.mesh().is_some() && node.mesh().unwrap().index() == mesh.index()
                    })
                    .map(|node| transform_gltf_node(&node).winding_order())
                    .unwrap_or(FrontFace::Ccw) // Default to CCW if no node found
            };
            for primitive in mesh.primitives() {
                let index: MeshBufferIndexInfoWithOffset = match GltfMeshBufferIndexInfo::maybe_new(
                    &primitive,
                    &buffers,
                    &mut index_bytes,
                )? {
                    Some(info) => info.into(),
                    None => generate_fresh_indices_from_primitive(&primitive, &mut index_bytes)?,
                };

                // Step 2: Convert to visibility buffer format
                let visibility_buffer_info = convert_to_visibility_buffer(
                    &primitive,
                    front_face,
                    &buffers,
                    &index,
                    &index_bytes,
                    &mut visibility_vertex_bytes,
                    &mut attribute_vertex_bytes,
                    &mut triangle_data_bytes,
                    &mut geometry_morph_bytes,
                    &mut material_morph_bytes,
                    &mut skin_joint_index_weight_bytes,
                )?;

                primitive_buffer_infos.push(visibility_buffer_info);
            }

            meshes.push(primitive_buffer_infos);
        }

        Ok(Self {
            raw: buffers,
            index_bytes,
            visibility_vertex_bytes,
            attribute_vertex_bytes,
            triangle_data_bytes,
            meshes,
            geometry_morph_bytes,
            material_morph_bytes,
            skin_joint_index_weight_bytes,
        })
    }
}
