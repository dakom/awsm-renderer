use awsm_renderer_core::pipeline::vertex::{VertexAttribute, VertexBufferLayout};

use super::{buffers::GltfMeshBufferInfo, error::Result};

pub(super) fn primitive_vertex_buffer_layout(
    buffer_info: &GltfMeshBufferInfo,
) -> Result<VertexBufferLayout> {
    let mut attributes = Vec::new();

    let mut shader_location: u32 = 0;

    let mut offset = 0u64;
    for attribute in &buffer_info.vertex.attributes {
        for _ in 0..attribute.shader_key_kind.count() {
            attributes.push(VertexAttribute {
                format: attribute.format,
                offset,
                shader_location,
            });

            offset += attribute.size as u64;
            shader_location += 1;
        }
    }

    Ok(VertexBufferLayout {
        // this is the stride across all of the attributes
        array_stride: offset as u64,
        step_mode: None, // TODO - instancing
        attributes,
    })
}
