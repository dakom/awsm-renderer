use awsm_renderer_core::pipeline::{primitive::IndexFormat, vertex::VertexFormat};

use crate::shaders::{ShaderCacheKeyAttribute, ShaderCacheKeyMorphs};

#[derive(Default, Debug, Clone)]
pub struct MeshBufferInfo {
    pub vertex: MeshBufferVertexInfo,
    pub index: Option<MeshBufferIndexInfo>,
    pub morph: Option<MeshBufferMorphInfo>,
}

#[derive(Default, Debug, Clone)]
pub struct MeshBufferVertexInfo {
    // number of vertices for this primitive
    pub count: usize,
    // total size in bytes of this vertex
    // same as vertex_count * sum_of_all_attribute_sizes
    // we don't need to know individual attribute sizes here
    // since that naturally follows the draw call size
    // though it is available for debugging purposes in `attributes`
    pub size: usize,

    pub attributes: Vec<MeshBufferVertexAttribute>,
}

#[derive(Debug, Clone)]
pub struct MeshBufferVertexAttribute {
    // the size of the attribute in bytes
    pub size: usize,
    // the offset of this attribute within the vertex
    pub offset: usize,
    // the format of this attribute
    pub format: VertexFormat,
    // shader key kind
    pub shader_key_kind: ShaderCacheKeyAttribute,
}

#[derive(Debug, Clone)]
pub struct MeshBufferIndexInfo {
    // number of index elements for this primitive
    pub count: usize,
    // number of bytes per index (e.g. 2 for u16, 4 for u32)
    pub data_size: usize,
    // the format of the index data
    pub format: IndexFormat,
}

impl MeshBufferIndexInfo {
    // the size in bytes of the index buffer for this primitive
    pub fn total_size(&self) -> usize {
        self.count * self.data_size
    }
}

#[derive(Default, Debug, Clone)]
pub struct MeshBufferMorphInfo {
    // contains info about the specific attribute targets
    pub shader_key: ShaderCacheKeyMorphs,
    // the number of morph targets for this primitive
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
        self.index.as_ref().map(|index| index.total_size())
    }
}
