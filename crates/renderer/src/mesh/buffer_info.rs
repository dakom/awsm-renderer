use std::collections::HashMap;

#[derive(Default, Debug, Clone)]
pub struct MeshBufferInfo {
    pub vertex: MeshBufferVertexInfo,
    pub index: Option<MeshBufferIndexInfo>,
    pub morph: Option<MeshBufferMorphInfo>,
}

#[derive(Default, Debug, Clone)]
pub struct MeshBufferVertexInfo {
    // offset in vertex_bytes where this primitive starts
    pub offset: usize,
    // number of vertices for this primitive
    pub count: usize,
    // total size in bytes of this vertex
    // same as vertex_count * sum_of_all_vertex_attribute_stride_sizes
    pub size: usize,
    // size of each individual vertex attribute stride
    pub attribute_stride_sizes: HashMap<MeshAttributeSemantic, usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MeshAttributeSemantic {
    Position,
    Normal,
    Tangent,
}

#[cfg(feature = "gltf")]
impl From<gltf::mesh::Semantic> for MeshAttributeSemantic {
    fn from(semantic: gltf::mesh::Semantic) -> Self {
        match semantic {
            gltf::mesh::Semantic::Positions => MeshAttributeSemantic::Position,
            gltf::mesh::Semantic::Normals => MeshAttributeSemantic::Normal,
            gltf::mesh::Semantic::Tangents => MeshAttributeSemantic::Tangent,
            _ => panic!("Unsupported mesh attribute semantic {:?}", semantic),
        }
    }
}

#[cfg(feature = "gltf")]
impl From<MeshAttributeSemantic> for gltf::mesh::Semantic {
    fn from(semantic: MeshAttributeSemantic) -> Self {
        match semantic {
            MeshAttributeSemantic::Position => gltf::mesh::Semantic::Positions,
            MeshAttributeSemantic::Normal => gltf::mesh::Semantic::Normals,
            MeshAttributeSemantic::Tangent => gltf::mesh::Semantic::Tangents,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct MeshBufferIndexInfo {
    // offset in index_bytes where this primitive starts
    pub offset: usize,
    // number of index elements for this primitive
    pub count: usize,
    // number of bytes per index (e.g. 2 for u16, 4 for u32)
    pub stride: usize,
}

#[derive(Default, Debug, Clone)]
pub struct MeshBufferMorphInfo {
    // number of morph targets
    pub targets_len: usize,
    // the stride of all morph targets across the vertice, without padding
    pub vertex_stride_size: usize,
    // the size of the whole slice of data (all vertices and targets)
    pub values_size: usize,
}

impl MeshBufferInfo {
    pub fn draw_count(&self) -> usize {
        // if we have indices, we use that count
        // otherwise, we use the vertex count
        self.index
            .as_ref()
            .map(|index| index.count)
            .unwrap_or(self.vertex.count)
    }

    // the size in bytes of the index buffer for this primitive, if it exists
    pub fn index_len(&self) -> Option<usize> {
        self.index.as_ref().map(|index| index.count * index.stride)
    }
}
