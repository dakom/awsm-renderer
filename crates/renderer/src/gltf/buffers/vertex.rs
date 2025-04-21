use std::collections::HashMap;

use crate::gltf::accessors::semantic_ordering;
use crate::mesh::{MeshAttributeSemantic, MeshBufferVertexInfo};

use super::accessor::accessor_to_bytes;
use super::Result;

#[derive(Default, Debug, Clone)]
pub struct GltfMeshBufferVertexInfo {
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

impl From<GltfMeshBufferVertexInfo> for MeshBufferVertexInfo {
    fn from(info: GltfMeshBufferVertexInfo) -> Self {
        Self {
            count: info.count,
            size: info.size,
            attribute_stride_sizes: info.attribute_stride_sizes,
        }
    }
}

impl GltfMeshBufferVertexInfo {
    pub fn new(
        primitive: &gltf::Primitive<'_>,
        buffers: &[Vec<u8>],
        vertex_bytes: &mut Vec<u8>,
    ) -> Result<Self> {
        let offset = vertex_bytes.len();

        let mut attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> =
            primitive.attributes().collect();

        attributes.sort_by(|(a, _), (b, _)| semantic_ordering(a).cmp(&semantic_ordering(b)));

        let mut attribute_stride_sizes = HashMap::new();
        let mut attributes_bytes = Vec::new();

        // this should never be empty, but let's be safe
        let vertex_count = attributes
            .first()
            .map(|(_, accessor)| accessor.count())
            .unwrap_or(0);

        // first we need to read the whole accessor. This will be zero-copy unless one of these is true:
        // 1. they're sparse and we need to replace values
        // 2. there's no view, and we need to fill it with zeroes
        //
        // otherwise, it's just a slice of the original buffer
        for (semantic, accessor) in attributes {
            let semantic: MeshAttributeSemantic = semantic.into();
            let attribute_bytes = accessor_to_bytes(&accessor, buffers)?;

            // while we're at it, we can stash the stride sizes
            let attribute_stride_size = accessor
                .view()
                .and_then(|view| view.stride())
                .unwrap_or(accessor.size());
            attribute_stride_sizes.insert(semantic.clone(), attribute_stride_size);

            attributes_bytes.push((attribute_bytes, attribute_stride_size));
        }

        // now let's predictably interleave the attributes into our final vertex buffer
        // this does extend/copy the data, but it saves us additional calls at render time
        for vertex in 0..vertex_count {
            for (attribute_bytes, attribute_stride_size) in attributes_bytes.iter() {
                let attribute_byte_offset = vertex * attribute_stride_size;
                let attribute_bytes = &attribute_bytes
                    [attribute_byte_offset..attribute_byte_offset + attribute_stride_size];

                vertex_bytes.extend_from_slice(attribute_bytes);
            }
        }

        Ok(Self {
            offset,
            count: vertex_count,
            size: vertex_bytes.len() - offset,
            attribute_stride_sizes,
        })
    }
}
