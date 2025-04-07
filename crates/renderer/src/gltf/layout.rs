use crate::gltf::accessors::semantic_ordering;
use awsm_renderer_core::pipeline::vertex::{VertexAttribute, VertexBufferLayout};

use super::{
    accessors::accessor_vertex_format, buffers::PrimitiveBufferInfo, error::Result,
    shaders::semantic_shader_location,
};

pub(super) fn primitive_vertex_buffer_layout(
    primitive: &gltf::Primitive<'_>,
    buffer_info: &PrimitiveBufferInfo,
) -> Result<VertexBufferLayout> {
    // not strictly necessary for the attributes array, which only needs the shader location
    // but this makes it quicker to lookup the individual array strides
    let mut gltf_attributes: Vec<(gltf::Semantic, gltf::Accessor<'_>)> =
        primitive.attributes().collect();

    gltf_attributes.sort_by(|(a, _), (b, _)| semantic_ordering(a).cmp(&semantic_ordering(b)));

    let mut attributes = Vec::with_capacity(gltf_attributes.len());

    // this is the offset within the total vertex stride
    let mut offset: u64 = 0;

    for (index, (semantic, accessor)) in gltf_attributes.into_iter().enumerate() {
        attributes.push(VertexAttribute {
            format: accessor_vertex_format(&accessor),
            offset,
            shader_location: semantic_shader_location(semantic),
        });

        // because the vertex strides are in a specific order
        // we can just add the stride of the current attribute to the offset
        offset += buffer_info.vertex_strides[index] as u64;
    }

    Ok(VertexBufferLayout {
        // this is the stride across all of the attributes
        array_stride: offset,
        step_mode: None, // TODO - instancing
        attributes,
    })
}
