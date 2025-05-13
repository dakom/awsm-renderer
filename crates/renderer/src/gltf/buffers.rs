pub mod accessor;
pub mod index;
pub mod morph;
pub mod normals;
pub mod vertex;

use index::GltfMeshBufferIndexInfo;
use morph::GltfMeshBufferMorphInfo;
use vertex::GltfMeshBufferVertexInfo;

use crate::mesh::MeshBufferInfo;

use super::error::{AwsmGltfError, Result};

#[derive(Debug)]
pub struct GltfBuffers {
    pub raw: Vec<Vec<u8>>,
    // this isn't passed to the shader at all
    // just used in the pipeline for drawing
    pub index_bytes: Option<Vec<u8>>,
    // this might later be split into positions, texcoords, normals, etc
    // but for now, we just want to pack it all into one buffer
    //
    // it's pretty common to treat positions as its own buffer, but, let's see...
    //
    // the important thing is that they always follow the same interleaving pattern
    // and we track where each primitive starts
    pub vertex_bytes: Vec<u8>,

    // these also always follow the same interleaving pattern
    // and we track where each primitive starts
    pub morph_bytes: Option<Vec<u8>>,

    // first level is mesh, second level is primitive
    pub meshes: Vec<Vec<GltfMeshBufferInfo>>,
}

#[derive(Default, Debug, Clone)]
pub struct GltfMeshBufferInfo {
    pub vertex: GltfMeshBufferVertexInfo,
    pub index: Option<GltfMeshBufferIndexInfo>,
    pub morph: Option<GltfMeshBufferMorphInfo>,
}

impl From<GltfMeshBufferInfo> for MeshBufferInfo {
    fn from(info: GltfMeshBufferInfo) -> Self {
        Self {
            vertex: info.vertex.into(),
            index: info.index.map(|i| i.into()),
            morph: info.morph.map(|m| m.into()),
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
        let mut vertex_bytes: Vec<u8> = Vec::new();
        let mut morph_bytes: Vec<u8> = Vec::new();
        let mut meshes: Vec<Vec<GltfMeshBufferInfo>> = Vec::new();

        for mesh in doc.meshes() {
            let mut primitive_buffer_infos = Vec::new();

            for primitive in mesh.primitives() {
                let index =
                    GltfMeshBufferIndexInfo::maybe_new(&primitive, &buffers, &mut index_bytes)?;
                let vertex = GltfMeshBufferVertexInfo::new(
                    &primitive,
                    &buffers,
                    index.as_ref().map(|index| (index, index_bytes.as_slice())),
                    &mut vertex_bytes,
                )?;
                let morph = GltfMeshBufferMorphInfo::maybe_new(
                    &primitive,
                    &buffers,
                    vertex.count,
                    &mut morph_bytes,
                )?;

                // Done for this primitive
                primitive_buffer_infos.push(GltfMeshBufferInfo {
                    index,
                    vertex,
                    morph,
                });
            }

            meshes.push(primitive_buffer_infos);
        }

        Ok(Self {
            raw: buffers,
            vertex_bytes,
            meshes,
            index_bytes: if index_bytes.is_empty() {
                None
            } else {
                Some(index_bytes)
            },
            morph_bytes: if morph_bytes.is_empty() {
                None
            } else {
                Some(morph_bytes)
            },
        })
    }
}
