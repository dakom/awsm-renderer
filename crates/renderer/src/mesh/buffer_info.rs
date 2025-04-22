use awsm_renderer_core::pipeline::primitive::IndexFormat;

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
    // same as vertex_count * sum_of_all_vertex_attribute_stride_sizes
    // we don't need to know individual attribute sizes here
    // since that naturally follows the draw call size
    pub size: usize,
}

#[derive(Debug, Clone)]
pub struct MeshBufferIndexInfo {
    // number of index elements for this primitive
    pub count: usize,
    // number of bytes per index (e.g. 2 for u16, 4 for u32)
    pub stride: usize,
    // the size of the whole slice of data (all indices)
    pub size: usize,
    // the format of the index data
    pub format: IndexFormat,
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
