use awsm_renderer_core::pipeline::vertex::{VertexAttribute, VertexBufferLayout};

use crate::mesh::MeshBufferInfo;

use super::{
    accessors::accessor_vertex_format,
    error::{AwsmGltfError, Result},
    shaders::semantic_shader_location,
};

pub(super) fn primitive_vertex_buffer_layout(
    primitive: &gltf::Primitive<'_>,
    buffer_info: &MeshBufferInfo,
) -> Result<VertexBufferLayout> {
    // not strictly necessary for the attributes array, which only needs the shader location
    // but this makes it quicker to lookup the individual array strides
    let mut attributes = Vec::with_capacity(primitive.attributes().len());

    // this is the offset within the total vertex stride
    let mut stride_offset = 0;

    for (semantic, accessor) in primitive.attributes() {
        attributes.push(VertexAttribute {
            format: accessor_vertex_format(&accessor),
            offset: stride_offset as u64,
            shader_location: semantic_shader_location(semantic.clone()),
        });

        // because the vertex strides are in a specific order
        // we can just add the stride of the current attribute to the offset
        stride_offset += buffer_info
            .vertex
            .attribute_stride_sizes
            .get(&semantic.clone().into())
            .ok_or(AwsmGltfError::MissingPositionAttribute(semantic))?;
    }

    Ok(VertexBufferLayout {
        // this is the stride across all of the attributes
        array_stride: stride_offset as u64,
        step_mode: None, // TODO - instancing
        attributes,
    })
}
